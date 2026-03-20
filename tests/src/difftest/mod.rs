//! Differential testing framework tests

#[cfg(test)]
mod tests {
    use tcg_difftest::mixed_executor::{
        create_alternating_executor, MixedExecutor, SwitchStrategy,
    };
    use tcg_difftest::qemu_bridge::{
        create_stub_bridge, QemuBridgeConfig, QemuExecResult,
    };
    use tcg_difftest::qemu_cpu_state::{
        Aarch64CpuState, QemuCpuState, RiscvCpuState,
    };
    use tcg_difftest::sync::{
        compare_results, DifftestState, ExecutionStats, StateComparison,
        StateSynchronizer,
    };

    #[test]
    fn test_qemu_cpu_state_creation() {
        let aarch64 = Aarch64CpuState::new();
        assert_eq!(aarch64.xregs.len(), 31);
        assert_eq!(aarch64.vregs.len(), 32);

        let riscv = RiscvCpuState::new();
        assert_eq!(riscv.xregs.len(), 32);
        assert_eq!(riscv.fregs.len(), 32);
    }

    #[test]
    fn test_difftest_state_creation() {
        let state = DifftestState::new("aarch64");
        assert_eq!(state.tb_count, 0);
        assert_eq!(state.mismatch_count, 0);
    }

    #[test]
    fn test_state_synchronizer() {
        let mut sync = StateSynchronizer::new().with_interval(3);

        // First 2 calls should return false (tb_since_sync: 1, 2)
        assert!(!sync.should_sync(0x1000));
        assert!(!sync.should_sync(0x1004));
        // 3rd call should return true (tb_since_sync reaches 3, then reset to 0)
        assert!(sync.should_sync(0x1008));
        // After reset, should start counting again
        assert!(!sync.should_sync(0x100c));
    }

    #[test]
    fn test_execution_stats() {
        let mut stats = ExecutionStats::new();

        let comparison = StateComparison::new(0x4000);
        stats.record_tb(&comparison);

        assert_eq!(stats.total_tbs, 1);
        assert_eq!(stats.matched_tbs, 1);
        assert_eq!(stats.match_rate(), 100.0);
    }

    #[test]
    fn test_mixed_executor() {
        let executor =
            MixedExecutor::new("aarch64", SwitchStrategy::tcg_only());
        assert_eq!(executor.get_strategy(), &SwitchStrategy::TcgOnly);
    }

    #[test]
    fn test_alternating_executor() {
        let executor = create_alternating_executor("aarch64", 3, 2);

        if let SwitchStrategy::Alternating {
            tcg_count,
            qemu_count,
        } = executor.get_strategy()
        {
            assert_eq!(*tcg_count, 3);
            assert_eq!(*qemu_count, 2);
        } else {
            panic!("Expected Alternating strategy");
        }
    }
}
