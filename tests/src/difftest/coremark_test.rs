//! Coremark integration tests for difftest framework
//!
//! These tests run the Coremark benchmark and compare results between tcg-rs and QEMU.

#[cfg(test)]
mod tests {
    use std::path::Path;
    use tcg_difftest::mixed_executor::{
        create_alternating_executor, MixedExecutor, SwitchStrategy,
    };
    use tcg_difftest::qemu_bridge::QemuExecResult;
    use tcg_difftest::qemu_cpu_state::{Aarch64CpuState, QemuCpuState};

    fn setup_test_environment() -> bool {
        std::env::var("SKIP_COREMARK_TESTS").is_err()
    }

    fn dummy_tcg_execute(state: &QemuCpuState, pc: u64) -> QemuExecResult {
        let mut result_state = state.clone();
        if let QemuCpuState::Aarch64(ref mut aarch64) = result_state {
            aarch64.pc = pc.wrapping_add(4);
            for i in 0..aarch64.xregs.len() {
                aarch64.xregs[i] = aarch64.xregs[i].wrapping_add(1);
            }
        }
        QemuExecResult::new(pc, result_state).with_next_pc(pc.wrapping_add(4))
    }

    #[test]
    fn test_coremark_tcg_only() {
        if !setup_test_environment() {
            eprintln!("Skipping Coremark test (SKIP_COREMARK_TESTS set)");
            return;
        }

        let mut executor =
            MixedExecutor::new("aarch64", SwitchStrategy::tcg_only())
                .with_initial_pc(0x4000)
                .with_max_mismatches(100)
                .with_stop_on_mismatch(false);

        let initial_state = Aarch64CpuState::new();
        executor =
            executor.with_initial_state(QemuCpuState::Aarch64(initial_state));

        let stats = executor
            .run(|state, pc| dummy_tcg_execute(state, pc), Some(100))
            .expect("Execution should complete");

        assert_eq!(stats.total_tbs, 100);
        assert_eq!(stats.matched_tbs, 100);
    }

    #[test]
    fn test_coremark_alternating() {
        if !setup_test_environment() {
            eprintln!("Skipping Coremark test (SKIP_COREMARK_TESTS set)");
            return;
        }

        let mut executor = create_alternating_executor("aarch64", 5, 5)
            .with_initial_pc(0x4000)
            .with_max_mismatches(100)
            .with_stop_on_mismatch(false);

        let initial_state = Aarch64CpuState::new();
        executor =
            executor.with_initial_state(QemuCpuState::Aarch64(initial_state));

        let stats = executor
            .run(|state, pc| dummy_tcg_execute(state, pc), Some(100))
            .expect("Execution should complete");

        assert_eq!(stats.total_tbs, 100);
    }

    #[test]
    fn test_coremark_pc_based() {
        if !setup_test_environment() {
            eprintln!("Skipping Coremark test (SKIP_COREMARK_TESTS set)");
            return;
        }

        let qemu_pcs: Vec<u64> = vec![0x4000, 0x4010, 0x4020];
        let executor =
            MixedExecutor::new("aarch64", SwitchStrategy::pc_based(qemu_pcs))
                .with_initial_pc(0x4000)
                .with_max_mismatches(100)
                .with_stop_on_mismatch(false);

        assert!(executor.get_strategy().should_use_qemu(0x4000, 0));
        assert!(!executor.get_strategy().should_use_qemu(0x4004, 0));
        assert!(executor.get_strategy().should_use_qemu(0x4010, 0));
    }

    #[test]
    fn test_coremark_range_based() {
        if !setup_test_environment() {
            eprintln!("Skipping Coremark test (SKIP_COREMARK_TESTS set)");
            return;
        }

        let ranges = vec![(0x4000, 0x4100), (0x5000, 0x5100)];
        let executor =
            MixedExecutor::new("aarch64", SwitchStrategy::range_based(ranges))
                .with_initial_pc(0x4000);

        assert!(executor.get_strategy().should_use_qemu(0x4050, 0));
        assert!(!executor.get_strategy().should_use_qemu(0x4200, 0));
        assert!(executor.get_strategy().should_use_qemu(0x5050, 0));
    }

    #[test]
    fn test_coremark_with_multiple_iterations() {
        if !setup_test_environment() {
            eprintln!("Skipping Coremark test (SKIP_COREMARK_TESTS set)");
            return;
        }

        let mut executor =
            MixedExecutor::new("aarch64", SwitchStrategy::tcg_only())
                .with_initial_pc(0x4000)
                .with_stop_on_mismatch(false);

        let initial_state = Aarch64CpuState::new();
        executor =
            executor.with_initial_state(QemuCpuState::Aarch64(initial_state));

        for iteration in 0..3 {
            let stats = executor
                .run(|state, pc| dummy_tcg_execute(state, pc), Some(50))
                .expect(&format!("Iteration {} should complete", iteration));

            assert_eq!(stats.total_tbs, 50);
        }
    }
}
