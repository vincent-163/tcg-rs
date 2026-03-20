//! Mixed executor for differential testing
//!
//! This module provides an executor that can switch between tcg-rs and QEMU execution
//! based on various strategies (PC-based, alternating, range-based, etc.).

use crate::qemu_bridge::{QemuBridge, QemuError, QemuExecResult};
use crate::qemu_cpu_state::QemuCpuState;
use crate::sync::{
    compare_results, DifftestState, ExecutionStats, StateComparison,
    StateSynchronizer,
};
use std::collections::HashSet;
use std::time::Duration;

/// Strategy for switching between tcg-rs and QEMU execution
#[derive(Clone, Debug, PartialEq)]
pub enum SwitchStrategy {
    /// Execute on tcg-rs for N TBs, then switch to QEMU for M TBs
    Alternating { tcg_count: usize, qemu_count: usize },

    /// Execute in QEMU for specific PC ranges, otherwise tcg-rs
    PcBased { qemu_pcs: HashSet<u64> },

    /// Execute in QEMU for specific PC ranges, otherwise tcg-rs
    RangeBased { qemu_ranges: Vec<(u64, u64)> },

    /// Always execute in tcg-rs
    TcgOnly,

    /// Always execute in QEMU
    QemuOnly,
}

impl SwitchStrategy {
    pub fn alternating(tcg_count: usize, qemu_count: usize) -> Self {
        Self::Alternating {
            tcg_count,
            qemu_count,
        }
    }

    pub fn pc_based(qemu_pcs: Vec<u64>) -> Self {
        Self::PcBased {
            qemu_pcs: qemu_pcs.into_iter().collect(),
        }
    }

    pub fn range_based(qemu_ranges: Vec<(u64, u64)>) -> Self {
        Self::RangeBased { qemu_ranges }
    }

    pub fn tcg_only() -> Self {
        Self::TcgOnly
    }

    pub fn qemu_only() -> Self {
        Self::QemuOnly
    }

    pub fn should_use_qemu(&self, pc: u64, tb_counter: usize) -> bool {
        match self {
            Self::Alternating {
                tcg_count,
                qemu_count,
            } => {
                let cycle = tb_counter % (tcg_count + qemu_count);
                cycle >= *tcg_count
            }
            Self::PcBased { qemu_pcs } => qemu_pcs.contains(&pc),
            Self::RangeBased { qemu_ranges } => qemu_ranges
                .iter()
                .any(|(start, end)| pc >= *start && pc < *end),
            Self::TcgOnly => false,
            Self::QemuOnly => true,
        }
    }
}

impl Default for SwitchStrategy {
    fn default() -> Self {
        Self::tcg_only()
    }
}

/// Execution mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExecutionMode {
    Tcg,
    Qemu,
}

/// Mixed executor that can switch between tcg-rs and QEMU
pub struct MixedExecutor {
    strategy: SwitchStrategy,
    state: DifftestState,
    synchronizer: StateSynchronizer,
    stats: ExecutionStats,
    current_mode: ExecutionMode,
    tb_counter: usize,
    bridge: Option<QemuBridge>,
    max_mismatches: usize,
    stop_on_mismatch: bool,
}

impl MixedExecutor {
    pub fn new(arch: &str, strategy: SwitchStrategy) -> Self {
        Self {
            strategy,
            state: DifftestState::new(arch),
            synchronizer: StateSynchronizer::new(),
            stats: ExecutionStats::new(),
            current_mode: ExecutionMode::Tcg,
            tb_counter: 0,
            bridge: None,
            max_mismatches: 10,
            stop_on_mismatch: true,
        }
    }

    pub fn with_bridge(mut self, bridge: QemuBridge) -> Self {
        self.bridge = Some(bridge);
        self
    }

    pub fn with_initial_pc(mut self, pc: u64) -> Self {
        self.state = self.state.with_pc(pc);
        self
    }

    pub fn with_initial_state(mut self, state: QemuCpuState) -> Self {
        self.state = self.state.with_initial_state(state);
        self
    }

    pub fn with_max_mismatches(mut self, max: usize) -> Self {
        self.max_mismatches = max;
        self
    }

    pub fn with_stop_on_mismatch(mut self, stop: bool) -> Self {
        self.stop_on_mismatch = stop;
        self
    }

    pub fn connect_bridge(&mut self) -> Result<(), QemuError> {
        if let Some(ref mut bridge) = self.bridge {
            bridge.connect()?;
        }
        Ok(())
    }

    pub fn disconnect_bridge(&mut self) {
        if let Some(ref mut bridge) = self.bridge {
            bridge.disconnect();
        }
    }

    pub fn step(
        &mut self,
        tcg_execute_fn: impl FnOnce(&QemuCpuState, u64) -> QemuExecResult,
    ) -> Result<Option<StateComparison>, DifftestError> {
        let pc = self.state.current_pc;
        let use_qemu = self.strategy.should_use_qemu(pc, self.tb_counter);

        let tcg_result = tcg_execute_fn(&self.state.tcg_state, pc);

        let comparison = if use_qemu {
            if let Some(ref mut bridge) = self.bridge {
                match bridge.execute_tb(pc, &self.state.tcg_state) {
                    Ok(qemu_result) => {
                        let comp = compare_results(&tcg_result, &qemu_result);
                        self.state.sync_from_qemu(&qemu_result.cpu_state);
                        Some(comp)
                    }
                    Err(e) => return Err(DifftestError::QemuError(e)),
                }
            } else {
                None
            }
        } else {
            self.state.sync_from_tcg(&tcg_result.cpu_state);
            None
        };

        if let Some(ref comp) = comparison {
            self.stats.record_tb(comp);
            if !comp.is_match() {
                self.state.record_mismatch();
                if self.stop_on_mismatch
                    && self.state.mismatch_count >= self.max_mismatches as u64
                {
                    return Err(DifftestError::MaxMismatchesReached);
                }
            }
        } else {
            let default_comp = StateComparison::new(pc);
            self.stats.record_tb(&default_comp);
        }

        self.state.increment_tb_count();
        self.tb_counter += 1;
        self.state.update_pc(tcg_result.next_pc);

        Ok(comparison)
    }

    pub fn run<F>(
        &mut self,
        mut tcg_execute_fn: F,
        max_steps: Option<usize>,
    ) -> Result<ExecutionStats, DifftestError>
    where
        F: FnMut(&QemuCpuState, u64) -> QemuExecResult,
    {
        self.connect_bridge()?;

        let mut steps = 0;
        loop {
            if let Some(max) = max_steps {
                if steps >= max {
                    break;
                }
            }

            match self.step(|state, pc| tcg_execute_fn(state, pc)) {
                Ok(_) => {
                    steps += 1;
                }
                Err(DifftestError::MaxMismatchesReached) => {
                    break;
                }
                Err(e) => {
                    self.disconnect_bridge();
                    return Err(e);
                }
            }
        }

        self.disconnect_bridge();
        Ok(self.stats.clone())
    }

    pub fn get_state(&self) -> &DifftestState {
        &self.state
    }

    pub fn get_stats(&self) -> &ExecutionStats {
        &self.stats
    }

    pub fn get_strategy(&self) -> &SwitchStrategy {
        &self.strategy
    }

    pub fn set_strategy(&mut self, strategy: SwitchStrategy) {
        self.strategy = strategy;
    }
}

impl Default for MixedExecutor {
    fn default() -> Self {
        Self::new("aarch64", SwitchStrategy::default())
    }
}

/// Errors that can occur during differential testing
#[derive(Debug)]
pub enum DifftestError {
    QemuError(QemuError),
    MaxMismatchesReached,
    InvalidState(String),
    ExecutionError(String),
}

impl std::fmt::Display for DifftestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QemuError(e) => write!(f, "QEMU error: {}", e),
            Self::MaxMismatchesReached => {
                write!(f, "Maximum number of mismatches reached")
            }
            Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Self::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for DifftestError {}

impl From<QemuError> for DifftestError {
    fn from(e: QemuError) -> Self {
        Self::QemuError(e)
    }
}

/// Create an alternating executor
pub fn create_alternating_executor(
    arch: &str,
    tcg_count: usize,
    qemu_count: usize,
) -> MixedExecutor {
    MixedExecutor::new(arch, SwitchStrategy::alternating(tcg_count, qemu_count))
}

/// Create a PC-based executor
pub fn create_pc_based_executor(
    arch: &str,
    qemu_pcs: Vec<u64>,
) -> MixedExecutor {
    MixedExecutor::new(arch, SwitchStrategy::pc_based(qemu_pcs))
}

/// Create a range-based executor
pub fn create_range_based_executor(
    arch: &str,
    qemu_ranges: Vec<(u64, u64)>,
) -> MixedExecutor {
    MixedExecutor::new(arch, SwitchStrategy::range_based(qemu_ranges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qemu_cpu_state::Aarch64CpuState;

    fn dummy_tcg_execute(state: &QemuCpuState, pc: u64) -> QemuExecResult {
        let mut result_state = state.clone();
        if let QemuCpuState::Aarch64(ref mut aarch64) = result_state {
            aarch64.pc = pc + 4;
        }
        QemuExecResult::new(pc, result_state).with_next_pc(pc + 4)
    }

    #[test]
    fn test_switch_strategy_alternating() {
        let strategy = SwitchStrategy::alternating(2, 1);

        assert!(!strategy.should_use_qemu(0x1000, 0));
        assert!(!strategy.should_use_qemu(0x1000, 1));
        assert!(strategy.should_use_qemu(0x1000, 2));
        assert!(!strategy.should_use_qemu(0x1000, 3));
    }

    #[test]
    fn test_switch_strategy_pc_based() {
        let strategy = SwitchStrategy::pc_based(vec![0x2000, 0x3000]);

        assert!(!strategy.should_use_qemu(0x1000, 0));
        assert!(strategy.should_use_qemu(0x2000, 0));
        assert!(!strategy.should_use_qemu(0x2500, 0));
        assert!(strategy.should_use_qemu(0x3000, 0));
    }

    #[test]
    fn test_switch_strategy_range_based() {
        let strategy = SwitchStrategy::range_based(vec![(0x2000, 0x3000)]);

        assert!(!strategy.should_use_qemu(0x1000, 0));
        assert!(strategy.should_use_qemu(0x2000, 0));
        assert!(strategy.should_use_qemu(0x2500, 0));
        assert!(!strategy.should_use_qemu(0x3000, 0));
    }

    #[test]
    fn test_mixed_executor_creation() {
        let executor =
            MixedExecutor::new("aarch64", SwitchStrategy::tcg_only());

        assert_eq!(executor.get_state().current_pc, 0);
        assert_eq!(executor.get_strategy(), &SwitchStrategy::TcgOnly);
    }

    #[test]
    fn test_mixed_executor_with_initial_state() {
        let state = QemuCpuState::Aarch64(Aarch64CpuState::new());
        let executor =
            MixedExecutor::new("aarch64", SwitchStrategy::tcg_only())
                .with_initial_pc(0x4000)
                .with_initial_state(state);

        assert_eq!(executor.get_state().current_pc, 0x4000);
    }
}
