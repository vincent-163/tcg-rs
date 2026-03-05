//! AArch64 CPU state for user-mode emulation.

/// Number of general-purpose registers (X0-X30).
/// X31 is not a GPR — SP is a separate register.
pub const NUM_XREGS: usize = 31;

/// Number of SIMD/FP registers (V0-V31), stored as lo/hi u64 pairs.
pub const NUM_VREGS: usize = 32;

/// Lazy NZCV condition code operation types.
///
/// When `cc_op == CC_OP_EAGER`, the packed NZCV is stored in `cc_a`.
/// Otherwise, flags must be computed from `cc_a`, `cc_b`, `cc_result`.
pub const CC_OP_EAGER: u64 = 0;
pub const CC_OP_ADD32: u64 = 1;
pub const CC_OP_ADD64: u64 = 2;
pub const CC_OP_SUB32: u64 = 3;
pub const CC_OP_SUB64: u64 = 4;
pub const CC_OP_LOGIC32: u64 = 5;
pub const CC_OP_LOGIC64: u64 = 6;

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
    /// Lazy NZCV: operation type (CC_OP_*).
    pub cc_op: u64,
    /// Lazy NZCV: first operand.
    /// When cc_op == CC_OP_EAGER, this holds the packed NZCV value.
    pub cc_a: u64,
    /// Lazy NZCV: second operand.
    pub cc_b: u64,
    /// Lazy NZCV: result of the operation.
    pub cc_result: u64,
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

/// Byte offset of the `cc_op` field.
pub const CC_OP_OFFSET: i64 = LOAD_BIAS_OFFSET + 8; // 280

/// Byte offset of the `cc_a` field.
pub const CC_A_OFFSET: i64 = CC_OP_OFFSET + 8; // 288

/// Byte offset of the `cc_b` field.
pub const CC_B_OFFSET: i64 = CC_A_OFFSET + 8; // 296

/// Byte offset of the `cc_result` field.
pub const CC_RESULT_OFFSET: i64 = CC_B_OFFSET + 8; // 304

/// Byte offset of the `fpcr` field.
pub const FPCR_OFFSET: i64 = CC_RESULT_OFFSET + 8; // 312

/// Byte offset of the `fpsr` field.
pub const FPSR_OFFSET: i64 = FPCR_OFFSET + 8; // 320

/// Byte offset of the `tpidr_el0` field.
pub const TPIDR_EL0_OFFSET: i64 = FPSR_OFFSET + 8; // 328

/// Byte offset of `vregs[i]` low half: 336 + i*16.
pub const VREGS_OFFSET: i64 = TPIDR_EL0_OFFSET + 8; // 336

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
            cc_op: CC_OP_EAGER,
            cc_a: 0,
            cc_b: 0,
            cc_result: 0,
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

// -- Lazy NZCV helper functions --

/// Compute packed NZCV from lazy cc_op state.
/// Called at runtime when the packed value is needed.
pub extern "C" fn helper_lazy_nzcv_to_packed(
    cc_op: u64, cc_a: u64, cc_b: u64, cc_result: u64,
) -> u64 {
    match cc_op {
        CC_OP_EAGER => {
            // Packed NZCV already in cc_a for eager mode.
            cc_a
        }
        CC_OP_ADD32 => compute_nzcv_add(cc_a, cc_b, cc_result, false),
        CC_OP_ADD64 => compute_nzcv_add(cc_a, cc_b, cc_result, true),
        CC_OP_SUB32 => compute_nzcv_sub(cc_a, cc_b, cc_result, false),
        CC_OP_SUB64 => compute_nzcv_sub(cc_a, cc_b, cc_result, true),
        CC_OP_LOGIC32 => compute_nzcv_logic(cc_result, false),
        CC_OP_LOGIC64 => compute_nzcv_logic(cc_result, true),
        _ => 0,
    }
}

/// Evaluate an AArch64 condition code from lazy NZCV state.
/// Returns 1 if condition is true, 0 if false.
pub extern "C" fn helper_lazy_nzcv_eval_cond(
    cc_op: u64, cc_a: u64, cc_b: u64, cc_result: u64,
    cond: u64,
) -> u64 {
    let nzcv = match cc_op {
        CC_OP_EAGER => cc_a, // Packed NZCV stored in cc_a
        _ => helper_lazy_nzcv_to_packed(cc_op, cc_a, cc_b, cc_result),
    };
    eval_cond_from_packed(nzcv, cond as u32)
}

fn compute_nzcv_add(a: u64, b: u64, result: u64, sf: bool) -> u64 {
    if sf {
        let n = (result >> 63) & 1;
        let z = if result == 0 { 1u64 } else { 0 };
        let c = if result < a { 1u64 } else { 0 };
        let xor_ab = a ^ b;
        let xor_ar = a ^ result;
        let v = ((!xor_ab) & xor_ar) >> 63;
        (n << 31) | (z << 30) | (c << 29) | (v << 28)
    } else {
        let a32 = a as u32;
        let b32 = b as u32;
        let r32 = result as u32;
        let n = ((r32 >> 31) & 1) as u64;
        let z = if r32 == 0 { 1u64 } else { 0 };
        let c = if r32 < a32 { 1u64 } else { 0 };
        let xor_ab = a32 ^ b32;
        let xor_ar = a32 ^ r32;
        let v = (((!xor_ab) & xor_ar) >> 31) as u64;
        (n << 31) | (z << 30) | (c << 29) | (v << 28)
    }
}

fn compute_nzcv_sub(a: u64, b: u64, result: u64, sf: bool) -> u64 {
    if sf {
        let n = (result >> 63) & 1;
        let z = if result == 0 { 1u64 } else { 0 };
        let c = if a >= b { 1u64 } else { 0 };
        let xor_ab = a ^ b;
        let xor_ar = a ^ result;
        let v = (xor_ab & xor_ar) >> 63;
        (n << 31) | (z << 30) | (c << 29) | (v << 28)
    } else {
        let a32 = a as u32;
        let b32 = b as u32;
        let r32 = result as u32;
        let n = ((r32 >> 31) & 1) as u64;
        let z = if r32 == 0 { 1u64 } else { 0 };
        let c = if a32 >= b32 { 1u64 } else { 0 };
        let xor_ab = a32 ^ b32;
        let xor_ar = a32 ^ r32;
        let v = ((xor_ab & xor_ar) >> 31) as u64;
        (n << 31) | (z << 30) | (c << 29) | (v << 28)
    }
}

fn compute_nzcv_logic(result: u64, sf: bool) -> u64 {
    if sf {
        let n = (result >> 63) & 1;
        let z = if result == 0 { 1u64 } else { 0 };
        (n << 31) | (z << 30)
    } else {
        let r32 = result as u32;
        let n = ((r32 >> 31) & 1) as u64;
        let z = if r32 == 0 { 1u64 } else { 0 };
        (n << 31) | (z << 30)
    }
}

fn eval_cond_from_packed(nzcv: u64, cond: u32) -> u64 {
    let n = (nzcv >> 31) & 1;
    let z = (nzcv >> 30) & 1;
    let c = (nzcv >> 29) & 1;
    let v = (nzcv >> 28) & 1;
    let base_cond = cond >> 1;
    let result = match base_cond {
        0 => z,           // EQ/NE
        1 => c,           // CS/CC
        2 => n,           // MI/PL
        3 => v,           // VS/VC
        4 => c & (z ^ 1), // HI/LS
        5 => (n ^ v) ^ 1, // GE/LT (N==V)
        6 => ((n ^ v) ^ 1) & (z ^ 1), // GT/LE
        7 => 1,           // AL
        _ => 0,
    };
    if (cond & 1) != 0 && cond != 0xf {
        result ^ 1
    } else {
        result
    }
}
