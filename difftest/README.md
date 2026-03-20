# Differential Testing Framework for tcg-rs

This crate provides a framework for comparing execution between tcg-rs and QEMU by running TBs (Translation Blocks) on both systems and comparing results.

## Overview

The difftest framework enables:

- **Data structures aligned with QEMU's internal structures** for accurate comparison
- **PC-based switching** between tcg-rs and QEMU execution
- **State synchronization** between the two systems
- **Integration with the existing tcg-rs execution loop**

## Architecture

### Core Components

1. **qemu_cpu_state.rs** - CPU state structures aligned with QEMU
   - `Aarch64CpuState` - AArch64 CPU state matching QEMU's CPUARMState
   - `RiscvCpuState` - RISC-V CPU state matching QEMU's CPURISCVState
   - `QemuCpuState` - Generic enum holding either architecture's state

2. **qemu_bridge.rs** - QEMU execution interface
   - `QemuBridge` - Bridge for communicating with QEMU
   - `StubQemuBridge` - Stub implementation for testing
   - `QemuExecResult` - Result of TB execution in QEMU

3. **sync.rs** - State synchronization
   - `DifftestState` - Tracks state of both tcg-rs and QEMU
   - `StateSynchronizer` - Manages synchronization points
   - `ExecutionStats` - Collects execution statistics

4. **mixed_executor.rs** - Mixed execution engine
   - `MixedExecutor` - Switches between tcg-rs and QEMU
   - `SwitchStrategy` - Defines when to use each engine
   - Helper functions: `create_alternating_executor`, `create_pc_based_executor`, `create_range_based_executor`

## Usage

### Basic Usage

```rust
use tcg_difftest::mixed_executor::MixedExecutor;
use tcg_difftest::qemu_cpu_state::{Aarch64CpuState, QemuCpuState};

// Create executor with TCG-only strategy
let mut executor = MixedExecutor::new("aarch64", SwitchStrategy::tcg_only())
    .with_initial_pc(0x4000)
    .with_initial_state(QemuCpuState::Aarch64(Aarch64CpuState::new()));

// Run execution
let stats = executor.run(
    |state, pc| {
        // Your TCG execution function
        tcg_execute(state, pc)
    },
    Some(1000), // Run max 1000 TBs
).expect("Execution failed");

println!("Match rate: {}%", stats.match_rate());
```

### Alternating Execution

```rust
use tcg_difftest::mixed_executor::create_alternating_executor;

// Execute 5 TBs in TCG, then 5 TBs in QEMU
let mut executor = create_alternating_executor("aarch64", 5, 5)
    .with_initial_pc(0x4000);
```

### PC-Based Execution

```rust
use tcg_difftest::mixed_executor::create_pc_based_executor;

// Execute specific PCs in QEMU
let qemu_pcs = vec![0x1000, 0x2000, 0x3000];
let mut executor = create_pc_based_executor("aarch64", qemu_pcs)
    .with_initial_pc(0x4000);
```

### Range-Based Execution

```rust
use tcg_difftest::mixed_executor::create_range_based_executor;

// Execute specific ranges in QEMU
let ranges = vec![(0x1000, 0x2000), (0x5000, 0x6000)];
let mut executor = create_range_based_executor("aarch64", ranges)
    .with_initial_pc(0x4000);
```

## Testing

Run the difftest tests:

```bash
cargo test -p tcg-difftest
cargo test -p tcg-tests difftest
```

### Environment Variables

- `SKIP_COREMARK_TESTS` - Set to skip Coremark integration tests

## State Comparison

The framework automatically compares:

- **GPRs** - General-purpose registers
- **FPRs** - Floating-point registers
- **PC** - Program counter
- **Control registers** - Architecture-specific control state
- **Memory writes** - Memory modifications during TB execution

## Switch Strategies

### Alternating
Execute N TBs in tcg-rs, then M TBs in QEMU.

### PC-Based
Use QEMU for specific program counter values.

### Range-Based
Use QEMU for execution within specific address ranges.

### TCG-Only
Always execute in tcg-rs (useful for baseline testing).

### QEMU-Only
Always execute in QEMU (for validation).

## Integration with tcg-rs

The framework integrates with tcg-rs by:

1. Accepting a closure/function for TCG execution
2. Comparing results with QEMU execution
3. Tracking statistics and mismatches
4. Providing detailed diff output

## Future Enhancements

- [ ] GDB remote protocol support for QEMU communication
- [ ] Automatic TB discovery and synchronization
- [ ] Parallel execution for performance comparison
- [ ] Detailed profiling data collection
- [ ] Support for more architectures (x86_64, arm32)

## License

See the main project LICENSE file.
