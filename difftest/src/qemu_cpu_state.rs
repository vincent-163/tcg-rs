//! QEMU-aligned CPU state structures
//!
//! This module defines CPU state structures that match QEMU's internal representation
//! to enable accurate comparison between tcg-rs and QEMU.

use std::fmt;

/// Architecture-specific CPU state trait
trait ArchCpuState: fmt::Debug + Clone + PartialEq {
    type RegType: Copy + Default + fmt::Debug + PartialEq;

    fn arch_name() -> &'static str;
    fn num_gprs() -> usize;
    fn num_fprs() -> usize;
    fn pc_reg_name() -> &'static str;
    fn sp_reg_name() -> &'static str;
    fn gpr_name(idx: usize) -> Option<&'static str>;
    fn fpr_name(idx: usize) -> Option<&'static str>;
}

/// Aarch64 CPU state (aligned with QEMU's CPUARMState)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Aarch64CpuState {
    pub xregs: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub pstate: u64,
    pub vregs: [u128; 32],
    pub fpcr: u32,
    pub fpsr: u32,
    pub tpidr_el0: u64,
    pub tpidrro_el0: u64,
}

impl Aarch64CpuState {
    pub const NUM_GPRS: usize = 31;
    pub const NUM_FPRS: usize = 32;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_x(&self, idx: usize) -> Option<u64> {
        self.xregs.get(idx).copied()
    }

    pub fn set_x(&mut self, idx: usize, value: u64) {
        if idx < self.xregs.len() {
            self.xregs[idx] = value;
        }
    }

    pub fn get_v(&self, idx: usize) -> Option<u128> {
        self.vregs.get(idx).copied()
    }

    pub fn set_v(&mut self, idx: usize, value: u128) {
        if idx < self.vregs.len() {
            self.vregs[idx] = value;
        }
    }

    pub fn diff(&self, other: &Self) -> Vec<String> {
        let mut diffs = Vec::new();

        for (i, (a, b)) in self.xregs.iter().zip(other.xregs.iter()).enumerate()
        {
            if a != b {
                diffs.push(format!("X{}: {:016x} vs {:016x}", i, a, b));
            }
        }

        if self.sp != other.sp {
            diffs.push(format!("SP: {:016x} vs {:016x}", self.sp, other.sp));
        }

        if self.pc != other.pc {
            diffs.push(format!("PC: {:016x} vs {:016x}", self.pc, other.pc));
        }

        if self.pstate != other.pstate {
            diffs.push(format!(
                "PSTATE: {:016x} vs {:016x}",
                self.pstate, other.pstate
            ));
        }

        for (i, (a, b)) in self.vregs.iter().zip(other.vregs.iter()).enumerate()
        {
            if a != b {
                diffs.push(format!("V{}: {:032x} vs {:032x}", i, a, b));
            }
        }

        if self.fpcr != other.fpcr {
            diffs
                .push(format!("FPCR: {:08x} vs {:08x}", self.fpcr, other.fpcr));
        }

        if self.fpsr != other.fpsr {
            diffs
                .push(format!("FPSR: {:08x} vs {:08x}", self.fpsr, other.fpsr));
        }

        diffs
    }
}

impl ArchCpuState for Aarch64CpuState {
    type RegType = u64;

    fn arch_name() -> &'static str {
        "aarch64"
    }

    fn num_gprs() -> usize {
        Self::NUM_GPRS
    }

    fn num_fprs() -> usize {
        Self::NUM_FPRS
    }

    fn pc_reg_name() -> &'static str {
        "pc"
    }

    fn sp_reg_name() -> &'static str {
        "sp"
    }

    fn gpr_name(idx: usize) -> Option<&'static str> {
        match idx {
            0..=30 => Some(match idx {
                0 => "x0",
                1 => "x1",
                2 => "x2",
                3 => "x3",
                4 => "x4",
                5 => "x5",
                6 => "x6",
                7 => "x7",
                8 => "x8",
                9 => "x9",
                10 => "x10",
                11 => "x11",
                12 => "x12",
                13 => "x13",
                14 => "x14",
                15 => "x15",
                16 => "x16",
                17 => "x17",
                18 => "x18",
                19 => "x19",
                20 => "x20",
                21 => "x21",
                22 => "x22",
                23 => "x23",
                24 => "x24",
                25 => "x25",
                26 => "x26",
                27 => "x27",
                28 => "x28",
                29 => "x29",
                30 => "x30",
                _ => unreachable!(),
            }),
            _ => None,
        }
    }

    fn fpr_name(idx: usize) -> Option<&'static str> {
        if idx < 32 {
            Some(match idx {
                0 => "v0",
                1 => "v1",
                2 => "v2",
                3 => "v3",
                4 => "v4",
                5 => "v5",
                6 => "v6",
                7 => "v7",
                8 => "v8",
                9 => "v9",
                10 => "v10",
                11 => "v11",
                12 => "v12",
                13 => "v13",
                14 => "v14",
                15 => "v15",
                16 => "v16",
                17 => "v17",
                18 => "v18",
                19 => "v19",
                20 => "v20",
                21 => "v21",
                22 => "v22",
                23 => "v23",
                24 => "v24",
                25 => "v25",
                26 => "v26",
                27 => "v27",
                28 => "v28",
                29 => "v29",
                30 => "v30",
                31 => "v31",
                _ => unreachable!(),
            })
        } else {
            None
        }
    }
}

/// RISC-V CPU state (aligned with QEMU's CPURISCVState)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RiscvCpuState {
    pub xregs: [u64; 32],
    pub pc: u64,
    pub fregs: [u64; 32],
    pub fcsr: u32,
    pub r#priv: u8,
    pub mstatus: u64,
    pub mie: u64,
    pub mip: u64,
    pub mtvec: u64,
    pub mcause: u64,
    pub mepc: u64,
    pub satp: u64,
}

impl RiscvCpuState {
    pub const NUM_GPRS: usize = 32;
    pub const NUM_FPRS: usize = 32;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_x(&self, idx: usize) -> Option<u64> {
        self.xregs.get(idx).copied()
    }

    pub fn set_x(&mut self, idx: usize, value: u64) {
        if idx < self.xregs.len() {
            self.xregs[idx] = value;
        }
    }

    pub fn get_f(&self, idx: usize) -> Option<u64> {
        self.fregs.get(idx).copied()
    }

    pub fn set_f(&mut self, idx: usize, value: u64) {
        if idx < self.fregs.len() {
            self.fregs[idx] = value;
        }
    }

    pub fn diff(&self, other: &Self) -> Vec<String> {
        let mut diffs = Vec::new();

        for (i, (a, b)) in self.xregs.iter().zip(other.xregs.iter()).enumerate()
        {
            if a != b {
                let name = Self::gpr_name(i).unwrap_or("?");
                diffs.push(format!("{}: {:016x} vs {:016x}", name, a, b));
            }
        }

        if self.pc != other.pc {
            diffs.push(format!("PC: {:016x} vs {:016x}", self.pc, other.pc));
        }

        for (i, (a, b)) in self.fregs.iter().zip(other.fregs.iter()).enumerate()
        {
            if a != b {
                let name = Self::fpr_name(i).unwrap_or("?");
                diffs.push(format!("{}: {:016x} vs {:016x}", name, a, b));
            }
        }

        if self.fcsr != other.fcsr {
            diffs
                .push(format!("FCSR: {:08x} vs {:08x}", self.fcsr, other.fcsr));
        }

        if self.r#priv != other.r#priv {
            diffs.push(format!("PRIV: {} vs {}", self.r#priv, other.r#priv));
        }

        if self.mstatus != other.mstatus {
            diffs.push(format!(
                "MSTATUS: {:016x} vs {:016x}",
                self.mstatus, other.mstatus
            ));
        }

        diffs
    }
}

impl ArchCpuState for RiscvCpuState {
    type RegType = u64;

    fn arch_name() -> &'static str {
        "riscv64"
    }

    fn num_gprs() -> usize {
        Self::NUM_GPRS
    }

    fn num_fprs() -> usize {
        Self::NUM_FPRS
    }

    fn pc_reg_name() -> &'static str {
        "pc"
    }

    fn sp_reg_name() -> &'static str {
        "x2"
    }

    fn gpr_name(idx: usize) -> Option<&'static str> {
        match idx {
            0..=31 => Some(match idx {
                0 => "zero",
                1 => "ra",
                2 => "sp",
                3 => "gp",
                4 => "tp",
                5 => "t0",
                6 => "t1",
                7 => "t2",
                8 => "s0",
                9 => "s1",
                10 => "a0",
                11 => "a1",
                12 => "a2",
                13 => "a3",
                14 => "a4",
                15 => "a5",
                16 => "a6",
                17 => "a7",
                18 => "s2",
                19 => "s3",
                20 => "s4",
                21 => "s5",
                22 => "s6",
                23 => "s7",
                24 => "s8",
                25 => "s9",
                26 => "s10",
                27 => "s11",
                28 => "t3",
                29 => "t4",
                30 => "t5",
                31 => "t6",
                _ => unreachable!(),
            }),
            _ => None,
        }
    }

    fn fpr_name(idx: usize) -> Option<&'static str> {
        if idx < 32 {
            Some(match idx {
                0 => "ft0",
                1 => "ft1",
                2 => "ft2",
                3 => "ft3",
                4 => "ft4",
                5 => "ft5",
                6 => "ft6",
                7 => "ft7",
                8 => "fs0",
                9 => "fs1",
                10 => "fa0",
                11 => "fa1",
                12 => "fa2",
                13 => "fa3",
                14 => "fa4",
                15 => "fa5",
                16 => "fa6",
                17 => "fa7",
                18 => "fs2",
                19 => "fs3",
                20 => "fs4",
                21 => "fs5",
                22 => "fs6",
                23 => "fs7",
                24 => "fs8",
                25 => "fs9",
                26 => "fs10",
                27 => "fs11",
                28 => "ft8",
                29 => "ft9",
                30 => "ft10",
                31 => "ft11",
                _ => unreachable!(),
            })
        } else {
            None
        }
    }
}

/// Generic CPU state enum that can hold either architecture's state
#[derive(Clone, Debug, PartialEq)]
pub enum QemuCpuState {
    Aarch64(Aarch64CpuState),
    Riscv64(RiscvCpuState),
}

impl QemuCpuState {
    pub fn as_aarch64(&self) -> Option<&Aarch64CpuState> {
        match self {
            Self::Aarch64(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_aarch64_mut(&mut self) -> Option<&mut Aarch64CpuState> {
        match self {
            Self::Aarch64(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_riscv64(&self) -> Option<&RiscvCpuState> {
        match self {
            Self::Riscv64(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_riscv64_mut(&mut self) -> Option<&mut RiscvCpuState> {
        match self {
            Self::Riscv64(state) => Some(state),
            _ => None,
        }
    }

    pub fn diff(&self, other: &Self) -> Vec<String> {
        match (self, other) {
            (Self::Aarch64(a), Self::Aarch64(b)) => a.diff(b),
            (Self::Riscv64(a), Self::Riscv64(b)) => a.diff(b),
            _ => vec!["Architecture mismatch".to_string()],
        }
    }

    pub fn arch_name(&self) -> &'static str {
        match self {
            Self::Aarch64(_) => "aarch64",
            Self::Riscv64(_) => "riscv64",
        }
    }
}

impl From<Aarch64CpuState> for QemuCpuState {
    fn from(state: Aarch64CpuState) -> Self {
        Self::Aarch64(state)
    }
}

impl From<RiscvCpuState> for QemuCpuState {
    fn from(state: RiscvCpuState) -> Self {
        Self::Riscv64(state)
    }
}

/// Trait for architecture-specific CPU state
pub trait CpuState: Clone + fmt::Debug + PartialEq {
    fn num_gprs() -> usize;
    fn num_fprs() -> usize;
    fn diff(&self, other: &Self) -> Vec<String>;
    fn arch_name(&self) -> &'static str;
}

impl CpuState for Aarch64CpuState {
    fn num_gprs() -> usize {
        Self::NUM_GPRS
    }

    fn num_fprs() -> usize {
        Self::NUM_FPRS
    }

    fn diff(&self, other: &Self) -> Vec<String> {
        self.diff(other)
    }

    fn arch_name(&self) -> &'static str {
        "aarch64"
    }
}

impl CpuState for RiscvCpuState {
    fn num_gprs() -> usize {
        Self::NUM_GPRS
    }

    fn num_fprs() -> usize {
        Self::NUM_FPRS
    }

    fn diff(&self, other: &Self) -> Vec<String> {
        self.diff(other)
    }

    fn arch_name(&self) -> &'static str {
        "riscv64"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aarch64_cpu_state() {
        let mut state = Aarch64CpuState::new();
        state.set_x(0, 0x1234);
        state.set_x(1, 0x5678);
        state.pc = 0x4000;
        state.sp = 0x8000;

        assert_eq!(state.get_x(0), Some(0x1234));
        assert_eq!(state.get_x(1), Some(0x5678));
        assert_eq!(state.pc, 0x4000);
        assert_eq!(state.sp, 0x8000);
    }

    #[test]
    fn test_aarch64_diff() {
        let mut state1 = Aarch64CpuState::new();
        let mut state2 = Aarch64CpuState::new();

        state1.set_x(0, 0x1234);
        state2.set_x(0, 0x5678);
        state1.pc = 0x4000;
        state2.pc = 0x4004;

        let diffs = state1.diff(&state2);
        assert!(!diffs.is_empty());
        assert!(diffs.iter().any(|d| d.contains("X0")));
        assert!(diffs.iter().any(|d| d.contains("PC")));
    }

    #[test]
    fn test_riscv_cpu_state() {
        let mut state = RiscvCpuState::new();
        state.set_x(10, 0x1234);
        state.set_x(11, 0x5678);
        state.pc = 0x4000;

        assert_eq!(state.get_x(10), Some(0x1234));
        assert_eq!(state.get_x(11), Some(0x5678));
        assert_eq!(state.pc, 0x4000);
    }

    #[test]
    fn test_qemu_cpu_state_enum() {
        let aarch64_state = Aarch64CpuState::new();
        let riscv_state = RiscvCpuState::new();

        let qemu_aarch64: QemuCpuState = aarch64_state.into();
        let qemu_riscv: QemuCpuState = riscv_state.into();

        assert!(qemu_aarch64.as_aarch64().is_some());
        assert!(qemu_aarch64.as_riscv64().is_none());
        assert!(qemu_riscv.as_riscv64().is_some());
        assert!(qemu_riscv.as_aarch64().is_none());
    }
}
