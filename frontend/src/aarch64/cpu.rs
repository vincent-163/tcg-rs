//! AArch64 CPU state for user-mode emulation.

/// Number of general-purpose registers (X0-X30).
/// X31 is not a GPR — SP is a separate register.
pub const NUM_XREGS: usize = 31;

/// Number of SIMD/FP registers (V0-V31), stored as lo/hi u64 pairs.
pub const NUM_VREGS: usize = 32;

/// AArch64 CPU architectural state (user-mode).
///
/// Layout must be `#[repr(C)]` so that TCG global temps can
/// reference fields at fixed offsets from the env pointer.
#[repr(C)]
pub struct Aarch64Cpu {
    /// General-purpose registers X0-X30 (X30 = LR).
    pub xregs: [u64; NUM_XREGS],
    /// Program counter.
    pub pc: u64,
    /// Stack pointer (separate from GPRs in AArch64).
    pub sp: u64,
    /// Guest memory base pointer (host address).
    pub guest_base: u64,
    /// Load bias: guest VA of the executable PT_LOAD segment.
    /// Stored in env so AOT dispatch can compute
    /// file_offset = pc - load_bias without changing the AOT
    /// function signature fn(ptr, i64) -> i64.
    pub load_bias: u64,
    /// Condition flags (N, Z, C, V).
    pub nzcv: u64,
    /// Floating-point control register.
    pub fpcr: u64,
    /// Floating-point status register.
    pub fpsr: u64,
    /// Thread pointer (user-mode TLS, TPIDR_EL0).
    pub tpidr_el0: u64,
    /// SIMD/FP registers V0-V31 as (lo, hi) u64 pairs.
    pub vregs: [u64; NUM_VREGS * 2],
}

// Field offsets (bytes) from the start of Aarch64Cpu.
// Used by `Context::new_global()` to bind IR temps.

/// Byte offset of `xregs[i]`: `i * 8`.
pub const fn xreg_offset(i: usize) -> i64 {
    (i * 8) as i64
}

/// Byte offset of the `pc` field.
pub const PC_OFFSET: i64 = (NUM_XREGS * 8) as i64; // 248

/// Byte offset of the `sp` field.
pub const SP_OFFSET: i64 = PC_OFFSET + 8; // 256

/// Byte offset of the `guest_base` field.
pub const GUEST_BASE_OFFSET: i64 = SP_OFFSET + 8; // 264

/// Byte offset of the `load_bias` field.
pub const LOAD_BIAS_OFFSET: i64 = GUEST_BASE_OFFSET + 8; // 272

/// Byte offset of the `nzcv` field.
pub const NZCV_OFFSET: i64 = LOAD_BIAS_OFFSET + 8; // 280

/// Byte offset of the `fpcr` field.
pub const FPCR_OFFSET: i64 = NZCV_OFFSET + 8; // 288

/// Byte offset of the `fpsr` field.
pub const FPSR_OFFSET: i64 = FPCR_OFFSET + 8; // 296

/// Byte offset of the `tpidr_el0` field.
pub const TPIDR_EL0_OFFSET: i64 = FPSR_OFFSET + 8; // 304

/// Byte offset of `vregs[i]` low half: 312 + i*16.
pub const VREGS_OFFSET: i64 = TPIDR_EL0_OFFSET + 8; // 312

/// Byte offset of vreg i low half.
pub const fn vreg_lo_offset(i: usize) -> i64 {
    VREGS_OFFSET + (i as i64) * 16
}

/// Byte offset of vreg i high half.
pub const fn vreg_hi_offset(i: usize) -> i64 {
    VREGS_OFFSET + (i as i64) * 16 + 8
}

impl Aarch64Cpu {
    pub fn new() -> Self {
        Self {
            xregs: [0u64; NUM_XREGS],
            pc: 0,
            sp: 0,
            guest_base: 0,
            load_bias: 0,
            nzcv: 0,
            fpcr: 0,
            fpsr: 0,
            tpidr_el0: 0,
            vregs: [0u64; NUM_VREGS * 2],
        }
    }
}

impl Default for Aarch64Cpu {
    fn default() -> Self {
        Self::new()
    }
}
