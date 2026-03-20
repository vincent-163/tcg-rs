//! State synchronization between tcg-rs and QEMU
//!
//! This module provides utilities for synchronizing CPU state between tcg-rs and QEMU
//! and comparing execution results.

use crate::qemu_bridge::{MemWrite, QemuExecResult};
use crate::qemu_cpu_state::{Aarch64CpuState, QemuCpuState, RiscvCpuState};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

/// Represents the state of the differential testing system
#[derive(Clone, Debug)]
pub struct DifftestState {
    pub tcg_state: QemuCpuState,
    pub qemu_state: QemuCpuState,
    pub current_pc: u64,
    pub tb_count: u64,
    pub mismatch_count: u64,
    pub last_sync_pc: u64,
}

impl DifftestState {
    pub fn new(arch: &str) -> Self {
        let state = match arch {
            "aarch64" => QemuCpuState::Aarch64(Aarch64CpuState::new()),
            "riscv64" => QemuCpuState::Riscv64(RiscvCpuState::new()),
            _ => panic!("Unsupported architecture: {}", arch),
        };

        Self {
            tcg_state: state.clone(),
            qemu_state: state,
            current_pc: 0,
            tb_count: 0,
            mismatch_count: 0,
            last_sync_pc: 0,
        }
    }

    pub fn with_initial_state(mut self, state: QemuCpuState) -> Self {
        self.tcg_state = state.clone();
        self.qemu_state = state;
        self
    }

    pub fn with_pc(mut self, pc: u64) -> Self {
        self.current_pc = pc;
        self.last_sync_pc = pc;
        self
    }

    pub fn increment_tb_count(&mut self) {
        self.tb_count += 1;
    }

    pub fn record_mismatch(&mut self) {
        self.mismatch_count += 1;
    }

    pub fn update_pc(&mut self, pc: u64) {
        self.current_pc = pc;
    }

    pub fn sync_from_tcg(&mut self, state: &QemuCpuState) {
        self.tcg_state = state.clone();
    }

    pub fn sync_from_qemu(&mut self, state: &QemuCpuState) {
        self.qemu_state = state.clone();
    }
}

/// Synchronizes state between tcg-rs and QEMU
pub struct StateSynchronizer {
    sync_interval: usize,
    tb_since_sync: usize,
    force_sync_pcs: Vec<u64>,
    skip_sync_pcs: Vec<u64>,
    sync_history: Vec<SyncRecord>,
}

#[derive(Clone, Debug)]
struct SyncRecord {
    pc: u64,
    timestamp: Instant,
    reason: SyncReason,
}

#[derive(Clone, Debug, PartialEq)]
enum SyncReason {
    Interval,
    Forced,
    Mismatch,
    Manual,
}

impl StateSynchronizer {
    pub fn new() -> Self {
        Self {
            sync_interval: 1,
            tb_since_sync: 0,
            force_sync_pcs: Vec::new(),
            skip_sync_pcs: Vec::new(),
            sync_history: Vec::new(),
        }
    }

    pub fn with_interval(mut self, interval: usize) -> Self {
        self.sync_interval = interval;
        self
    }

    pub fn add_force_sync_pc(&mut self, pc: u64) {
        self.force_sync_pcs.push(pc);
    }

    pub fn add_skip_sync_pc(&mut self, pc: u64) {
        self.skip_sync_pcs.push(pc);
    }

    pub fn should_sync(&mut self, pc: u64) -> bool {
        if self.skip_sync_pcs.contains(&pc) {
            return false;
        }

        if self.force_sync_pcs.contains(&pc) {
            self.tb_since_sync = 0;
            self.sync_history.push(SyncRecord {
                pc,
                timestamp: Instant::now(),
                reason: SyncReason::Forced,
            });
            return true;
        }

        self.tb_since_sync += 1;

        if self.tb_since_sync >= self.sync_interval {
            self.tb_since_sync = 0;
            self.sync_history.push(SyncRecord {
                pc,
                timestamp: Instant::now(),
                reason: SyncReason::Interval,
            });
            true
        } else {
            false
        }
    }

    pub fn record_sync(&mut self, pc: u64, reason: SyncReason) {
        self.sync_history.push(SyncRecord {
            pc,
            timestamp: Instant::now(),
            reason,
        });
    }

    pub fn get_sync_history(&self) -> &Vec<SyncRecord> {
        &self.sync_history
    }

    pub fn reset(&mut self) {
        self.tb_since_sync = 0;
    }

    pub fn clear_history(&mut self) {
        self.sync_history.clear();
    }
}

impl Default for StateSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison result between tcg-rs and QEMU execution
#[derive(Clone, Debug)]
pub struct StateComparison {
    pub pc: u64,
    pub matches: bool,
    pub diffs: Vec<String>,
    pub tcg_mem_writes: Vec<MemWrite>,
    pub qemu_mem_writes: Vec<MemWrite>,
    pub exec_time_tcg: Duration,
    pub exec_time_qemu: Duration,
}

impl StateComparison {
    pub fn new(pc: u64) -> Self {
        Self {
            pc,
            matches: true,
            diffs: Vec::new(),
            tcg_mem_writes: Vec::new(),
            qemu_mem_writes: Vec::new(),
            exec_time_tcg: Duration::default(),
            exec_time_qemu: Duration::default(),
        }
    }

    pub fn with_diffs(mut self, diffs: Vec<String>) -> Self {
        self.matches = diffs.is_empty();
        self.diffs = diffs;
        self
    }

    pub fn with_mem_writes(
        mut self,
        tcg_writes: Vec<MemWrite>,
        qemu_writes: Vec<MemWrite>,
    ) -> Self {
        self.tcg_mem_writes = tcg_writes;
        self.qemu_mem_writes = qemu_writes;
        self
    }

    pub fn with_exec_times(
        mut self,
        tcg_time: Duration,
        qemu_time: Duration,
    ) -> Self {
        self.exec_time_tcg = tcg_time;
        self.exec_time_qemu = qemu_time;
        self
    }

    pub fn is_match(&self) -> bool {
        self.matches && self.tcg_mem_writes == self.qemu_mem_writes
    }
}

/// Statistics for differential testing execution
#[derive(Clone, Debug, Default)]
pub struct ExecutionStats {
    pub total_tbs: u64,
    pub matched_tbs: u64,
    pub mismatched_tbs: u64,
    pub total_exec_time_tcg: Duration,
    pub total_exec_time_qemu: Duration,
    pub sync_count: u64,
    pub mismatch_details: Vec<StateComparison>,
}

impl ExecutionStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_tb(&mut self, comparison: &StateComparison) {
        self.total_tbs += 1;
        self.total_exec_time_tcg += comparison.exec_time_tcg;
        self.total_exec_time_qemu += comparison.exec_time_qemu;

        if comparison.is_match() {
            self.matched_tbs += 1;
        } else {
            self.mismatched_tbs += 1;
            self.mismatch_details.push(comparison.clone());
        }
    }

    pub fn record_sync(&mut self) {
        self.sync_count += 1;
    }

    pub fn match_rate(&self) -> f64 {
        if self.total_tbs == 0 {
            0.0
        } else {
            (self.matched_tbs as f64 / self.total_tbs as f64) * 100.0
        }
    }

    pub fn avg_exec_time_tcg(&self) -> Duration {
        if self.total_tbs == 0 {
            Duration::default()
        } else {
            self.total_exec_time_tcg / self.total_tbs as u32
        }
    }

    pub fn avg_exec_time_qemu(&self) -> Duration {
        if self.total_tbs == 0 {
            Duration::default()
        } else {
            self.total_exec_time_qemu / self.total_tbs as u32
        }
    }

    pub fn speedup(&self) -> f64 {
        let tcg_avg = self.avg_exec_time_tcg().as_nanos() as f64;
        let qemu_avg = self.avg_exec_time_qemu().as_nanos() as f64;

        if tcg_avg == 0.0 || qemu_avg == 0.0 {
            1.0
        } else {
            qemu_avg / tcg_avg
        }
    }
}

/// Compare tcg-rs execution result with QEMU result
pub fn compare_results(
    tcg_result: &QemuExecResult,
    qemu_result: &QemuExecResult,
) -> StateComparison {
    let diffs = tcg_result.cpu_state.diff(&qemu_result.cpu_state);

    StateComparison::new(tcg_result.tb_pc)
        .with_diffs(diffs)
        .with_mem_writes(
            tcg_result.mem_writes.clone(),
            qemu_result.mem_writes.clone(),
        )
        .with_exec_times(tcg_result.exec_time, qemu_result.exec_time)
}

/// Format a comparison result for display
pub fn format_comparison(comparison: &StateComparison) -> String {
    let mut output = format!("PC: 0x{:016x}\n", comparison.pc);
    output.push_str(&format!("Match: {}\n", comparison.is_match()));

    if !comparison.diffs.is_empty() {
        output.push_str("Differences:\n");
        for diff in &comparison.diffs {
            output.push_str(&format!("  - {}\n", diff));
        }
    }

    output
        .push_str(&format!("TCG exec time: {:?}\n", comparison.exec_time_tcg));
    output.push_str(&format!(
        "QEMU exec time: {:?}\n",
        comparison.exec_time_qemu
    ));

    output
}

/// Check if memory writes are equivalent
pub fn compare_mem_writes(
    tcg_writes: &[MemWrite],
    qemu_writes: &[MemWrite],
) -> Vec<String> {
    let mut diffs = Vec::new();

    if tcg_writes.len() != qemu_writes.len() {
        diffs.push(format!(
            "Write count mismatch: {} vs {}",
            tcg_writes.len(),
            qemu_writes.len()
        ));
    }

    let min_len = tcg_writes.len().min(qemu_writes.len());
    for i in 0..min_len {
        if tcg_writes[i] != qemu_writes[i] {
            diffs.push(format!(
                "Write {}: TCG({:?}) vs QEMU({:?})",
                i, tcg_writes[i], qemu_writes[i]
            ));
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difftest_state() {
        let state = DifftestState::new("aarch64");
        assert_eq!(state.tb_count, 0);
        assert_eq!(state.mismatch_count, 0);
        assert_eq!(state.current_pc, 0);
    }

    #[test]
    fn test_state_synchronizer() {
        let mut sync = StateSynchronizer::new().with_interval(5);

        // First 4 calls should return false (tb_since_sync: 1,2,3,4)
        assert!(!sync.should_sync(0x1000));
        assert!(!sync.should_sync(0x1004));
        assert!(!sync.should_sync(0x1008));
        assert!(!sync.should_sync(0x100c));
        // 5th call should return true (tb_since_sync reaches 5, then reset to 0)
        assert!(sync.should_sync(0x1010));
        // After reset, should start counting again
        assert!(!sync.should_sync(0x1014));

        // Force sync should always return true
        sync.add_force_sync_pc(0x2000);
        assert!(sync.should_sync(0x2000));
    }

    #[test]
    fn test_execution_stats() {
        let mut stats = ExecutionStats::new();

        let comparison = StateComparison::new(0x4000);
        stats.record_tb(&comparison);

        assert_eq!(stats.total_tbs, 1);
        assert_eq!(stats.matched_tbs, 1);
        assert_eq!(stats.mismatched_tbs, 0);
    }

    #[test]
    fn test_compare_mem_writes() {
        let writes1 = vec![
            MemWrite::new(0x1000, 0x1234, 4),
            MemWrite::new(0x1004, 0x5678, 4),
        ];

        let writes2 = vec![
            MemWrite::new(0x1000, 0x1234, 4),
            MemWrite::new(0x1004, 0x5678, 4),
        ];

        let diffs = compare_mem_writes(&writes1, &writes2);
        assert!(diffs.is_empty());
    }
}
