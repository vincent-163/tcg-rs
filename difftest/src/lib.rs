//! Differential Testing Framework for tcg-rs
//!
//! This crate provides a framework for comparing execution between tcg-rs and QEMU
//! by running TBs (Translation Blocks) on both systems and comparing results.
//!
//! Key features:
//! - Data structures aligned with QEMU's internal structures
//! - PC-based switching between tcg-rs and QEMU execution
//! - State synchronization between the two systems
//! - Integration with the existing tcg-rs execution loop

pub mod mixed_executor;
pub mod qemu_bridge;
pub mod qemu_cpu_state;
pub mod sync;

pub use mixed_executor::{
    create_alternating_executor, create_pc_based_executor,
    create_range_based_executor, DifftestError, ExecutionMode, MixedExecutor,
    SwitchStrategy,
};
pub use qemu_bridge::{
    create_bridge, create_stub_bridge, BridgeStats, MemWrite, QemuBridge,
    QemuBridgeConfig, QemuError, QemuExecResult, StubQemuBridge,
};
pub use qemu_cpu_state::{
    Aarch64CpuState, CpuState, QemuCpuState, RiscvCpuState,
};
pub use sync::{
    compare_mem_writes, compare_results, DifftestState, ExecutionStats,
    StateComparison, StateSynchronizer,
};
