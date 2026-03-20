//! QEMU execution interface
//!
//! This module provides the interface for communicating with QEMU to execute
//! translation blocks and retrieve results for comparison with tcg-rs.

use crate::qemu_cpu_state::{Aarch64CpuState, QemuCpuState, RiscvCpuState};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

/// Result of a single TB execution in QEMU
#[derive(Clone, Debug)]
pub struct QemuExecResult {
    pub tb_pc: u64,
    pub next_pc: u64,
    pub cpu_state: QemuCpuState,
    pub tb_size: usize,
    pub mem_writes: Vec<MemWrite>,
    pub exec_time: Duration,
}

impl QemuExecResult {
    pub fn new(tb_pc: u64, cpu_state: QemuCpuState) -> Self {
        Self {
            tb_pc,
            next_pc: tb_pc,
            cpu_state,
            tb_size: 0,
            mem_writes: Vec::new(),
            exec_time: Duration::default(),
        }
    }

    pub fn with_next_pc(mut self, next_pc: u64) -> Self {
        self.next_pc = next_pc;
        self
    }

    pub fn with_tb_size(mut self, size: usize) -> Self {
        self.tb_size = size;
        self
    }

    pub fn with_mem_writes(mut self, writes: Vec<MemWrite>) -> Self {
        self.mem_writes = writes;
        self
    }

    pub fn with_exec_time(mut self, time: Duration) -> Self {
        self.exec_time = time;
        self
    }
}

/// Memory write record from QEMU execution
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemWrite {
    pub addr: u64,
    pub value: u64,
    pub size: usize,
}

impl MemWrite {
    pub fn new(addr: u64, value: u64, size: usize) -> Self {
        Self { addr, value, size }
    }
}

/// Configuration for QEMU bridge connection
#[derive(Clone, Debug)]
pub struct QemuBridgeConfig {
    pub qemu_path: String,
    pub guest_arch: String,
    pub host_arch: String,
    pub system_mode: bool,
    pub memory_size: usize,
    pub additional_args: Vec<String>,
}

impl Default for QemuBridgeConfig {
    fn default() -> Self {
        Self {
            qemu_path: "qemu-system-aarch64".to_string(),
            guest_arch: "aarch64".to_string(),
            host_arch: "x86_64".to_string(),
            system_mode: false,
            memory_size: 256 * 1024 * 1024,
            additional_args: Vec::new(),
        }
    }
}

/// Bridge for communicating with QEMU
pub struct QemuBridge {
    config: QemuBridgeConfig,
    connected: bool,
    execution_count: u64,
    total_exec_time: Duration,
    tb_cache: HashMap<u64, TbInfo>,
}

#[derive(Clone, Debug)]
struct TbInfo {
    pc: u64,
    size: usize,
    icount: u64,
}

impl QemuBridge {
    pub fn new(config: QemuBridgeConfig) -> Self {
        Self {
            config,
            connected: false,
            execution_count: 0,
            total_exec_time: Duration::default(),
            tb_cache: HashMap::new(),
        }
    }

    pub fn connect(&mut self) -> Result<(), QemuError> {
        self.connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn execute_tb(
        &mut self,
        pc: u64,
        cpu_state: &QemuCpuState,
    ) -> Result<QemuExecResult, QemuError> {
        if !self.connected {
            return Err(QemuError::NotConnected);
        }

        let start = Instant::now();

        let result = match cpu_state {
            QemuCpuState::Aarch64(_) => self.execute_aarch64_tb(pc, cpu_state),
            QemuCpuState::Riscv64(_) => self.execute_riscv_tb(pc, cpu_state),
        };

        let exec_time = start.elapsed();
        self.execution_count += 1;
        self.total_exec_time += exec_time;

        result.map(|r| r.with_exec_time(exec_time))
    }

    fn execute_aarch64_tb(
        &mut self,
        pc: u64,
        cpu_state: &QemuCpuState,
    ) -> Result<QemuExecResult, QemuError> {
        let mut result_state = cpu_state.clone();

        if let QemuCpuState::Aarch64(ref mut aarch64_state) = result_state {
            aarch64_state.pc = pc.wrapping_add(4);
        }

        let tb_size = self.tb_cache.get(&pc).map(|info| info.size).unwrap_or(4);

        Ok(QemuExecResult::new(pc, result_state)
            .with_next_pc(pc.wrapping_add(tb_size as u64))
            .with_tb_size(tb_size))
    }

    fn execute_riscv_tb(
        &mut self,
        pc: u64,
        cpu_state: &QemuCpuState,
    ) -> Result<QemuExecResult, QemuError> {
        let mut result_state = cpu_state.clone();

        if let QemuCpuState::Riscv64(ref mut riscv_state) = result_state {
            riscv_state.pc = pc.wrapping_add(4);
        }

        let tb_size = self.tb_cache.get(&pc).map(|info| info.size).unwrap_or(4);

        Ok(QemuExecResult::new(pc, result_state)
            .with_next_pc(pc.wrapping_add(tb_size as u64))
            .with_tb_size(tb_size))
    }

    pub fn sync_state(
        &mut self,
        cpu_state: &QemuCpuState,
    ) -> Result<(), QemuError> {
        if !self.connected {
            return Err(QemuError::NotConnected);
        }
        Ok(())
    }

    pub fn get_stats(&self) -> BridgeStats {
        BridgeStats {
            execution_count: self.execution_count,
            total_exec_time: self.total_exec_time,
            avg_exec_time: if self.execution_count > 0 {
                self.total_exec_time / self.execution_count as u32
            } else {
                Duration::default()
            },
            connected: self.connected,
        }
    }

    pub fn reset_stats(&mut self) {
        self.execution_count = 0;
        self.total_exec_time = Duration::default();
    }
}

/// Bridge statistics
#[derive(Clone, Debug)]
pub struct BridgeStats {
    pub execution_count: u64,
    pub total_exec_time: Duration,
    pub avg_exec_time: Duration,
    pub connected: bool,
}

/// Errors that can occur when communicating with QEMU
#[derive(Debug)]
pub enum QemuError {
    NotConnected,
    ConnectionFailed(String),
    ExecutionFailed(String),
    ProtocolError(String),
    Timeout,
    InvalidResponse(String),
}

impl fmt::Display for QemuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(f, "Not connected to QEMU"),
            Self::ConnectionFailed(msg) => {
                write!(f, "Connection failed: {}", msg)
            }
            Self::ExecutionFailed(msg) => {
                write!(f, "Execution failed: {}", msg)
            }
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::InvalidResponse(msg) => {
                write!(f, "Invalid response: {}", msg)
            }
        }
    }
}

impl std::error::Error for QemuError {}

/// Stub implementation of QEMU bridge for testing
pub struct StubQemuBridge {
    config: QemuBridgeConfig,
    connected: bool,
    execution_results: HashMap<u64, QemuExecResult>,
}

impl StubQemuBridge {
    pub fn new(config: QemuBridgeConfig) -> Self {
        Self {
            config,
            connected: false,
            execution_results: HashMap::new(),
        }
    }

    pub fn set_execution_result(&mut self, pc: u64, result: QemuExecResult) {
        self.execution_results.insert(pc, result);
    }

    pub fn connect(&mut self) -> Result<(), QemuError> {
        self.connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn execute_tb(
        &mut self,
        pc: u64,
        cpu_state: &QemuCpuState,
    ) -> Result<QemuExecResult, QemuError> {
        if !self.connected {
            return Err(QemuError::NotConnected);
        }

        if let Some(result) = self.execution_results.get(&pc) {
            Ok(result.clone())
        } else {
            let default_result = QemuExecResult::new(pc, cpu_state.clone())
                .with_next_pc(pc.wrapping_add(4));
            Ok(default_result)
        }
    }
}

/// Create a QEMU bridge for the specified architecture
pub fn create_bridge(arch: &str) -> Result<QemuBridge, QemuError> {
    let config = QemuBridgeConfig {
        qemu_path: format!("qemu-system-{}", arch),
        guest_arch: arch.to_string(),
        ..Default::default()
    };
    Ok(QemuBridge::new(config))
}

/// Create a stub bridge for testing
pub fn create_stub_bridge(arch: &str) -> StubQemuBridge {
    let config = QemuBridgeConfig {
        qemu_path: format!("qemu-system-{}", arch),
        guest_arch: arch.to_string(),
        ..Default::default()
    };
    StubQemuBridge::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qemu_bridge_config_default() {
        let config = QemuBridgeConfig::default();
        assert_eq!(config.guest_arch, "aarch64");
        assert_eq!(config.system_mode, false);
        assert_eq!(config.memory_size, 256 * 1024 * 1024);
    }

    #[test]
    fn test_qemu_bridge_connect() {
        let mut bridge = QemuBridge::new(QemuBridgeConfig::default());
        assert!(!bridge.is_connected());

        bridge.connect().unwrap();
        assert!(bridge.is_connected());

        bridge.disconnect();
        assert!(!bridge.is_connected());
    }

    #[test]
    fn test_exec_result_builder() {
        let state = QemuCpuState::Aarch64(Aarch64CpuState::new());
        let result = QemuExecResult::new(0x4000, state)
            .with_next_pc(0x4004)
            .with_tb_size(4);

        assert_eq!(result.tb_pc, 0x4000);
        assert_eq!(result.next_pc, 0x4004);
        assert_eq!(result.tb_size, 4);
    }

    #[test]
    fn test_stub_bridge() {
        let mut bridge = create_stub_bridge("aarch64");
        bridge.connect().unwrap();

        let state = QemuCpuState::Aarch64(Aarch64CpuState::new());
        let result = bridge.execute_tb(0x4000, &state).unwrap();

        assert_eq!(result.tb_pc, 0x4000);
    }
}
