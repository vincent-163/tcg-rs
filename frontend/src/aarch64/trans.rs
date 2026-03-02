//! AArch64 instruction translation — TCG IR generation.
//!
//! Translates decoded A64 instructions into TCG IR opcodes.
//! Follows the same gen_xxx helper pattern as the RISC-V frontend.
//!
// Function pointers are intentionally cast to u64 (not usize) because
// gen_call() stores helper addresses as u64 IR constants.
#![allow(clippy::fn_to_numeric_cast)]

use super::cpu::{
    helper_lazy_nzcv_eval_cond, helper_lazy_nzcv_to_packed, vreg_hi_offset,
    vreg_lo_offset, CC_OP_ADD32, CC_OP_ADD64, CC_OP_EAGER, CC_OP_LOGIC32,
    CC_OP_LOGIC64, CC_OP_SUB32, CC_OP_SUB64, FPCR_OFFSET, FPSR_OFFSET,
    TPIDR_EL0_OFFSET,
};
use super::insn_decode::*;
use super::{Aarch64DisasContext, LazyNzcvKind};
use crate::DisasJumpType;
use tcg_core::context::Context;
use tcg_core::tb::{EXCP_ECALL, TB_EXIT_IDX0, TB_EXIT_IDX1, TB_EXIT_NOCHAIN};
use tcg_core::types::{Cond, MemOp, Type};
use tcg_core::TempIdx;

/// Binary IR operation: `fn(ir, ty, dst, lhs, rhs) -> dst`.
#[allow(dead_code)]
type BinOp = fn(&mut Context, Type, TempIdx, TempIdx, TempIdx) -> TempIdx;

// ── Bitmask immediate decoding ───────────────────────────

fn decode_bitmask_imm(sf: bool, n: u32, immr: u32, imms: u32) -> Option<u64> {
    let len = if n != 0 {
        6
    } else {
        let combined = !imms & 0x3f;
        if combined == 0 {
            return None;
        }
        31 - combined.leading_zeros()
    };
    if len == 0 {
        return None;
    }
    let size = 1u32 << len;
    let mask = size - 1;
    let s = imms & mask;
    let r = immr & mask;
    if s == mask {
        return None;
    }
    let welem = (1u64 << (s + 1)) - 1;
    let elem_mask = if size >= 64 {
        u64::MAX
    } else {
        (1u64 << size) - 1
    };
    let elem = if r == 0 {
        welem
    } else {
        ((welem >> r) | (welem << (size - r))) & elem_mask
    };
    let mut imm = elem;
    let mut sz = size;
    while sz < 64 {
        imm |= imm << sz;
        sz <<= 1;
    }
    if !sf {
        imm &= 0xffff_ffff;
    }
    Some(imm)
}

// ── Helpers ──────────────────────────────────────────────

impl Aarch64DisasContext {
    pub(crate) fn read_xreg(&self, ir: &mut Context, reg: i64) -> TempIdx {
        if reg == 31 {
            ir.new_const(Type::I64, 0)
        } else {
            self.xregs[reg as usize]
        }
    }

    pub(crate) fn write_xreg(&self, ir: &mut Context, reg: i64, val: TempIdx) {
        if reg != 31 {
            ir.gen_mov(Type::I64, self.xregs[reg as usize], val);
        }
    }

    pub(crate) fn read_xreg_sp(&self, _ir: &mut Context, reg: i64) -> TempIdx {
        if reg == 31 {
            self.sp
        } else {
            self.xregs[reg as usize]
        }
    }

    pub(crate) fn write_xreg_sp(
        &self,
        ir: &mut Context,
        reg: i64,
        val: TempIdx,
    ) {
        if reg == 31 {
            ir.gen_mov(Type::I64, self.sp, val);
        } else {
            ir.gen_mov(Type::I64, self.xregs[reg as usize], val);
        }
    }

    /// Write with optional 32-bit zero-extension.
    fn write_xreg_sz(
        &self,
        ir: &mut Context,
        reg: i64,
        val: TempIdx,
        sf: bool,
    ) {
        if reg == 31 {
            return;
        }
        if sf {
            ir.gen_mov(Type::I64, self.xregs[reg as usize], val);
        } else {
            let ext = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(ext, val);
            ir.gen_mov(Type::I64, self.xregs[reg as usize], ext);
        }
    }

    fn write_xreg_sp_sz(
        &self,
        ir: &mut Context,
        reg: i64,
        val: TempIdx,
        sf: bool,
    ) {
        let dst = if reg == 31 {
            self.sp
        } else {
            self.xregs[reg as usize]
        };
        if sf {
            ir.gen_mov(Type::I64, dst, val);
        } else {
            let ext = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(ext, val);
            ir.gen_mov(Type::I64, dst, ext);
        }
    }

    fn sf_type(sf: bool) -> Type {
        if sf {
            Type::I64
        } else {
            Type::I32
        }
    }

    fn trunc32(ir: &mut Context, val: TempIdx, sf: bool) -> TempIdx {
        if sf {
            val
        } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, val);
            t
        }
    }

    fn apply_shift(
        ir: &mut Context,
        ty: Type,
        val: TempIdx,
        shift_type: i64,
        amount: i64,
    ) -> TempIdx {
        if amount == 0 {
            return val;
        }
        let sh = ir.new_const(ty, amount as u64);
        let d = ir.new_temp(ty);
        match shift_type {
            0 => ir.gen_shl(ty, d, val, sh),
            1 => ir.gen_shr(ty, d, val, sh),
            2 => ir.gen_sar(ty, d, val, sh),
            3 => ir.gen_rotr(ty, d, val, sh),
            _ => unreachable!(),
        };
        d
    }

    // -- Lazy NZCV: set flags from add/sub --
    fn gen_nzcv_add_sub(
        &mut self,
        ir: &mut Context,
        a: TempIdx,
        b: TempIdx,
        result: TempIdx,
        sf: bool,
        is_sub: bool,
    ) {
        // Write cc_op, cc_a, cc_b, cc_result to globals.
        let op_val = if is_sub {
            if sf {
                CC_OP_SUB64
            } else {
                CC_OP_SUB32
            }
        } else {
            if sf {
                CC_OP_ADD64
            } else {
                CC_OP_ADD32
            }
        };
        let op_c = ir.new_const(Type::I64, op_val);
        ir.gen_mov(Type::I64, self.cc_op, op_c);
        // Widen a, b, result to I64 for storage in cc_a/cc_b/cc_result.
        let a64 = if sf {
            a
        } else {
            let t = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(t, a);
            t
        };
        let b64 = if sf {
            b
        } else {
            let t = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(t, b);
            t
        };
        let r64 = if sf {
            result
        } else {
            let t = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(t, result);
            t
        };
        ir.gen_mov(Type::I64, self.cc_a, a64);
        ir.gen_mov(Type::I64, self.cc_b, b64);
        ir.gen_mov(Type::I64, self.cc_result, r64);
        // Set compile-time lazy state.
        self.lazy_nzcv = Some(if is_sub {
            LazyNzcvKind::Sub { sf }
        } else {
            LazyNzcvKind::Add { sf }
        });
    }

    // -- Lazy NZCV: set flags from logic (C=0, V=0) --
    fn gen_nzcv_logic(&mut self, ir: &mut Context, result: TempIdx, sf: bool) {
        let op_val = if sf { CC_OP_LOGIC64 } else { CC_OP_LOGIC32 };
        let op_c = ir.new_const(Type::I64, op_val);
        ir.gen_mov(Type::I64, self.cc_op, op_c);
        let r64 = if sf {
            result
        } else {
            let t = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(t, result);
            t
        };
        ir.gen_mov(Type::I64, self.cc_result, r64);
        // cc_a and cc_b are don't-care for logic ops.
        self.lazy_nzcv = Some(LazyNzcvKind::Logic { sf });
    }

    // -- Materialize packed NZCV from lazy state --
    // Call this before any operation that needs the packed nzcv
    // global (MRS NZCV, ADC, SBC, etc.)
    fn materialize_nzcv(&mut self, ir: &mut Context) {
        if self.lazy_nzcv.is_some() {
            // We know cc_op at compile time; call the helper
            // to compute packed NZCV and store it.
            let packed = ir.new_temp(Type::I64);
            ir.gen_call(
                packed,
                helper_lazy_nzcv_to_packed as u64,
                &[self.cc_op, self.cc_a, self.cc_b, self.cc_result],
            );
            ir.gen_mov(Type::I64, self.nzcv, packed);
            // Mark cc_op as EAGER and store packed in cc_a.
            let eager = ir.new_const(Type::I64, CC_OP_EAGER);
            ir.gen_mov(Type::I64, self.cc_op, eager);
            ir.gen_mov(Type::I64, self.cc_a, packed);
            self.lazy_nzcv = None;
        } else {
            // cc_op unknown at compile time — might be lazy from
            // previous TB. We need to check at runtime.
            // Generate: if cc_op != EAGER { nzcv = helper(...); cc_op = EAGER; cc_a = nzcv; }
            let eager_c = ir.new_const(Type::I64, CC_OP_EAGER);
            let skip = ir.new_label();
            ir.gen_brcond(Type::I64, self.cc_op, eager_c, Cond::Eq, skip);
            let packed = ir.new_temp(Type::I64);
            ir.gen_call(
                packed,
                helper_lazy_nzcv_to_packed as u64,
                &[self.cc_op, self.cc_a, self.cc_b, self.cc_result],
            );
            ir.gen_mov(Type::I64, self.nzcv, packed);
            ir.gen_mov(Type::I64, self.cc_op, eager_c);
            ir.gen_mov(Type::I64, self.cc_a, packed);
            ir.gen_set_label(skip);
        }
    }

    // -- Set NZCV to a specific packed value (EAGER) --
    // For FCMP, MSR NZCV, CCMP fallthrough, etc.
    fn set_nzcv_eager(&mut self, ir: &mut Context, val: TempIdx) {
        ir.gen_mov(Type::I64, self.nzcv, val);
        let eager = ir.new_const(Type::I64, CC_OP_EAGER);
        ir.gen_mov(Type::I64, self.cc_op, eager);
        // Store packed nzcv in cc_a so the runtime helper
        // can access it when cc_op == EAGER.
        ir.gen_mov(Type::I64, self.cc_a, val);
        self.lazy_nzcv = None;
    }

    // -- Invalidate compile-time lazy tracking --
    // Called at labels (branch targets) where cc_op is unknown.
    fn invalidate_lazy_nzcv(&mut self) {
        self.lazy_nzcv = None;
    }

    // -- Condition evaluation (lazy-aware) --
    fn eval_cond(&mut self, ir: &mut Context, cond: i64) -> TempIdx {
        if cond == 0xe || cond == 0xf {
            // AL/NV — always true
            let one = ir.new_const(Type::I64, 1);
            return one;
        }

        if std::env::var_os("TCG_A64_NO_INLINE_COND").is_none() {
            if let Some(lazy) = self.lazy_nzcv {
                return match lazy {
                    LazyNzcvKind::Sub { sf } => {
                        let packed = self.pack_lazy_sub_nzcv(ir, sf);
                        self.eval_cond_from_packed(ir, cond, packed)
                    }
                    _ => self.eval_cond_inline(ir, cond, lazy),
                };
            }
        }

        // cc_op unknown — call runtime helper.
        let cond_c = ir.new_const(Type::I64, cond as u64);
        let result = ir.new_temp(Type::I64);
        ir.gen_call(
            result,
            helper_lazy_nzcv_eval_cond as u64,
            &[self.cc_op, self.cc_a, self.cc_b, self.cc_result, cond_c],
        );
        result
    }

    fn extract_nzcv_bit_from(
        &self,
        ir: &mut Context,
        nzcv: TempIdx,
        bit: u64,
    ) -> TempIdx {
        let sh = ir.new_const(Type::I64, bit);
        let one = ir.new_const(Type::I64, 1);
        let t = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, t, nzcv, sh);
        ir.gen_and(Type::I64, t, t, one);
        t
    }

    fn eval_cond_from_packed(
        &self,
        ir: &mut Context,
        cond: i64,
        nzcv: TempIdx,
    ) -> TempIdx {
        let n = self.extract_nzcv_bit_from(ir, nzcv, 31);
        let z = self.extract_nzcv_bit_from(ir, nzcv, 30);
        let c = self.extract_nzcv_bit_from(ir, nzcv, 29);
        let v = self.extract_nzcv_bit_from(ir, nzcv, 28);
        let base_cond = (cond >> 1) as u32;
        let result = match base_cond {
            0 => z, // EQ/NE
            1 => c, // CS/CC
            2 => n, // MI/PL
            3 => v, // VS/VC
            4 => {
                let t = ir.new_temp(Type::I64);
                ir.gen_andc(Type::I64, t, c, z); // HI/LS
                t
            }
            5 => {
                let r = ir.new_temp(Type::I64);
                ir.gen_setcond(Type::I64, r, n, v, Cond::Eq); // GE/LT
                r
            }
            6 => {
                let nv = ir.new_temp(Type::I64);
                ir.gen_setcond(Type::I64, nv, n, v, Cond::Eq);
                let t = ir.new_temp(Type::I64);
                ir.gen_andc(Type::I64, t, nv, z); // GT/LE
                t
            }
            7 => ir.new_const(Type::I64, 1), // AL
            _ => unreachable!(),
        };

        if (cond & 1) != 0 && cond != 0xf {
            let inv = ir.new_temp(Type::I64);
            let one = ir.new_const(Type::I64, 1);
            ir.gen_xor(Type::I64, inv, result, one);
            inv
        } else {
            result
        }
    }

    fn pack_lazy_sub_nzcv(&self, ir: &mut Context, sf: bool) -> TempIdx {
        let ty = Self::sf_type(sf);
        let bits = if sf { 63u64 } else { 31u64 };
        let a = if sf {
            self.cc_a
        } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, self.cc_a);
            t
        };
        let b = if sf {
            self.cc_b
        } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, self.cc_b);
            t
        };
        let res = if sf {
            self.cc_result
        } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, self.cc_result);
            t
        };
        let zero = ir.new_const(ty, 0);
        let sh = ir.new_const(ty, bits);

        let n_tmp = ir.new_temp(ty);
        ir.gen_shr(ty, n_tmp, res, sh);
        let n_bit = ir.new_temp(Type::I64);
        if sf {
            ir.gen_mov(Type::I64, n_bit, n_tmp);
        } else {
            ir.gen_ext_u32_i64(n_bit, n_tmp);
        }

        let z_bit = ir.new_temp(Type::I64);
        ir.gen_setcond(ty, z_bit, res, zero, Cond::Eq);

        let c_bit = ir.new_temp(Type::I64);
        ir.gen_setcond(ty, c_bit, a, b, Cond::Geu);

        let xor_ab = ir.new_temp(ty);
        ir.gen_xor(ty, xor_ab, a, b);
        let xor_ar = ir.new_temp(ty);
        ir.gen_xor(ty, xor_ar, a, res);
        let v_tmp = ir.new_temp(ty);
        ir.gen_and(ty, v_tmp, xor_ab, xor_ar);
        let v_sh = ir.new_temp(ty);
        ir.gen_shr(ty, v_sh, v_tmp, sh);
        let v_bit = ir.new_temp(Type::I64);
        if sf {
            ir.gen_mov(Type::I64, v_bit, v_sh);
        } else {
            ir.gen_ext_u32_i64(v_bit, v_sh);
        }

        let c31 = ir.new_const(Type::I64, 31);
        let c30 = ir.new_const(Type::I64, 30);
        let c29 = ir.new_const(Type::I64, 29);
        let c28 = ir.new_const(Type::I64, 28);
        let n_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, n_s, n_bit, c31);
        let z_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, z_s, z_bit, c30);
        let c_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, c_s, c_bit, c29);
        let v_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, v_s, v_bit, c28);
        let nzcv = ir.new_temp(Type::I64);
        ir.gen_or(Type::I64, nzcv, n_s, z_s);
        let tmp = ir.new_temp(Type::I64);
        ir.gen_or(Type::I64, tmp, c_s, v_s);
        ir.gen_or(Type::I64, nzcv, nzcv, tmp);
        nzcv
    }

    /// Generate inline condition evaluation from known lazy state.
    fn eval_cond_inline(
        &self,
        ir: &mut Context,
        cond: i64,
        lazy: LazyNzcvKind,
    ) -> TempIdx {
        let base_cond = (cond >> 1) as u32;
        let invert = (cond & 1) != 0;

        // Read operands from globals (always live, unlike local temps).
        let result = match lazy {
            LazyNzcvKind::Add { sf } | LazyNzcvKind::Sub { sf } => {
                let is_sub = matches!(lazy, LazyNzcvKind::Sub { .. });
                let ty = Self::sf_type(sf);
                // Read from cc_a, cc_b, cc_result globals.
                let a = if sf {
                    self.cc_a
                } else {
                    let t = ir.new_temp(Type::I32);
                    ir.gen_extrl_i64_i32(t, self.cc_a);
                    t
                };
                let b = if sf {
                    self.cc_b
                } else {
                    let t = ir.new_temp(Type::I32);
                    ir.gen_extrl_i64_i32(t, self.cc_b);
                    t
                };
                let res = if sf {
                    self.cc_result
                } else {
                    let t = ir.new_temp(Type::I32);
                    ir.gen_extrl_i64_i32(t, self.cc_result);
                    t
                };
                let zero = ir.new_const(ty, 0);
                match base_cond {
                    0 => {
                        // EQ: Z==1 ⟹ result == 0
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Eq);
                        r
                    }
                    1 => {
                        // CS: C==1
                        let r = ir.new_temp(Type::I64);
                        if is_sub {
                            ir.gen_setcond(ty, r, a, b, Cond::Geu);
                        } else {
                            ir.gen_setcond(ty, r, res, a, Cond::Ltu);
                        }
                        r
                    }
                    2 => {
                        // MI: N==1 ⟹ result is negative
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Lt);
                        r
                    }
                    3 => {
                        // VS: V==1 (overflow)
                        self.eval_overflow(ir, a, b, res, sf, is_sub)
                    }
                    4 => {
                        // HI: C==1 && Z==0
                        let r = ir.new_temp(Type::I64);
                        if is_sub {
                            ir.gen_setcond(ty, r, a, b, Cond::Gtu);
                        } else {
                            let c = ir.new_temp(Type::I64);
                            ir.gen_setcond(ty, c, res, a, Cond::Ltu);
                            let nz = ir.new_temp(Type::I64);
                            ir.gen_setcond(ty, nz, res, zero, Cond::Ne);
                            ir.gen_and(Type::I64, r, c, nz);
                        }
                        r
                    }
                    5 => {
                        // GE: N==V ⟹ signed >=
                        let r = ir.new_temp(Type::I64);
                        if is_sub {
                            ir.gen_setcond(ty, r, a, b, Cond::Ge);
                        } else {
                            let v =
                                self.eval_overflow(ir, a, b, res, sf, false);
                            let n = ir.new_temp(Type::I64);
                            ir.gen_setcond(ty, n, res, zero, Cond::Lt);
                            ir.gen_setcond(Type::I64, r, n, v, Cond::Eq);
                        }
                        r
                    }
                    6 => {
                        // GT: N==V && Z==0 ⟹ signed >
                        let r = ir.new_temp(Type::I64);
                        if is_sub {
                            ir.gen_setcond(ty, r, a, b, Cond::Gt);
                        } else {
                            let ge = ir.new_temp(Type::I64);
                            let v =
                                self.eval_overflow(ir, a, b, res, sf, false);
                            let n = ir.new_temp(Type::I64);
                            ir.gen_setcond(ty, n, res, zero, Cond::Lt);
                            ir.gen_setcond(Type::I64, ge, n, v, Cond::Eq);
                            let nz = ir.new_temp(Type::I64);
                            ir.gen_setcond(ty, nz, res, zero, Cond::Ne);
                            ir.gen_and(Type::I64, r, ge, nz);
                        }
                        r
                    }
                    7 => ir.new_const(Type::I64, 1),
                    _ => unreachable!(),
                }
            }
            LazyNzcvKind::Logic { sf } => {
                let ty = Self::sf_type(sf);
                // Read result from cc_result global.
                let res = if sf {
                    self.cc_result
                } else {
                    let t = ir.new_temp(Type::I32);
                    ir.gen_extrl_i64_i32(t, self.cc_result);
                    t
                };
                let zero = ir.new_const(ty, 0);
                match base_cond {
                    0 => {
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Eq);
                        r
                    }
                    1 => ir.new_const(Type::I64, 0),
                    2 => {
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Lt);
                        r
                    }
                    3 => ir.new_const(Type::I64, 0),
                    4 => ir.new_const(Type::I64, 0),
                    5 => {
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Ge);
                        r
                    }
                    6 => {
                        let r = ir.new_temp(Type::I64);
                        ir.gen_setcond(ty, r, res, zero, Cond::Gt);
                        r
                    }
                    7 => ir.new_const(Type::I64, 1),
                    _ => unreachable!(),
                }
            }
        };

        if invert {
            let inv = ir.new_temp(Type::I64);
            let zero = ir.new_const(Type::I64, 0);
            ir.gen_setcond(Type::I64, inv, result, zero, Cond::Eq);
            inv
        } else {
            result
        }
    }

    /// Compute overflow flag (V) from operands.
    fn eval_overflow(
        &self,
        ir: &mut Context,
        a: TempIdx,
        b: TempIdx,
        result: TempIdx,
        sf: bool,
        is_sub: bool,
    ) -> TempIdx {
        let ty = Self::sf_type(sf);
        let bits = if sf { 63u64 } else { 31u64 };
        let sh = ir.new_const(ty, bits);

        let xor_ab = ir.new_temp(ty);
        ir.gen_xor(ty, xor_ab, a, b);
        let xor_ar = ir.new_temp(ty);
        ir.gen_xor(ty, xor_ar, a, result);
        let v_tmp = ir.new_temp(ty);
        if is_sub {
            ir.gen_and(ty, v_tmp, xor_ab, xor_ar);
        } else {
            let not_xor = ir.new_temp(ty);
            ir.gen_not(ty, not_xor, xor_ab);
            ir.gen_and(ty, v_tmp, not_xor, xor_ar);
        }
        let v_sh = ir.new_temp(ty);
        ir.gen_shr(ty, v_sh, v_tmp, sh);
        let v_bit = ir.new_temp(Type::I64);
        if sf {
            ir.gen_mov(Type::I64, v_bit, v_sh);
        } else {
            ir.gen_ext_u32_i64(v_bit, v_sh);
        }
        v_bit
    }

    // -- Branch helpers --
    fn gen_direct_branch(&mut self, ir: &mut Context, target: u64, slot: u32) {
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(slot);
        let exit = if slot == 0 {
            TB_EXIT_IDX0
        } else {
            TB_EXIT_IDX1
        };
        ir.gen_exit_tb(exit);
    }

    fn gen_indirect_branch(&mut self, ir: &mut Context, addr: TempIdx) {
        ir.gen_mov(Type::I64, self.pc, addr);
        ir.gen_exit_tb(TB_EXIT_NOCHAIN);
    }

    // -- Load/store address helpers --
    pub(crate) fn compute_addr_imm(
        &self,
        ir: &mut Context,
        rn: i64,
        offset: i64,
    ) -> TempIdx {
        let base = self.read_xreg_sp(ir, rn);
        if offset == 0 {
            return base;
        }
        let c = ir.new_const(Type::I64, offset as u64);
        let addr = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr, base, c);
        addr
    }

    fn compute_addr_reg(
        &self,
        ir: &mut Context,
        rn: i64,
        rm: i64,
        option: i64,
        shift_amount: i64,
    ) -> TempIdx {
        let base = self.read_xreg_sp(ir, rn);
        let idx = self.read_xreg(ir, rm);
        let ext = match option {
            0b010 => {
                // UXTW
                let t = ir.new_temp(Type::I64);
                let mask = ir.new_const(Type::I64, 0xffff_ffff);
                ir.gen_and(Type::I64, t, idx, mask);
                t
            }
            0b110 => {
                // SXTW
                let t32 = ir.new_temp(Type::I32);
                ir.gen_extrl_i64_i32(t32, idx);
                let t = ir.new_temp(Type::I64);
                ir.gen_ext_i32_i64(t, t32);
                t
            }
            _ => idx, // LSL/UXTX/SXTX
        };
        let shifted = if shift_amount != 0 {
            let sh = ir.new_const(Type::I64, shift_amount as u64);
            let t = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, t, ext, sh);
            t
        } else {
            ext
        };
        let addr = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr, base, shifted);
        addr
    }

    // -- Extend register helper for ADD/SUB extended --
    fn extend_reg(
        ir: &mut Context,
        val: TempIdx,
        option: i64,
        shift: i64,
    ) -> TempIdx {
        // Extract based on option[1:0] size
        let extracted = match option & 0x3 {
            0 => {
                // xTB - byte
                let t = ir.new_temp(Type::I64);
                let m = ir.new_const(Type::I64, 0xff);
                ir.gen_and(Type::I64, t, val, m);
                t
            }
            1 => {
                // xTH - halfword
                let t = ir.new_temp(Type::I64);
                let m = ir.new_const(Type::I64, 0xffff);
                ir.gen_and(Type::I64, t, val, m);
                t
            }
            2 => {
                // xTW - word
                let t = ir.new_temp(Type::I64);
                let m = ir.new_const(Type::I64, 0xffff_ffff);
                ir.gen_and(Type::I64, t, val, m);
                t
            }
            3 => val, // xTX - doubleword
            _ => unreachable!(),
        };
        // Sign-extend if option[2] == 1
        let sign_ext = if option >= 4 {
            match option & 0x3 {
                0 => {
                    let t = ir.new_temp(Type::I64);
                    ir.gen_sextract(Type::I64, t, val, 0, 8);
                    t
                }
                1 => {
                    let t = ir.new_temp(Type::I64);
                    ir.gen_sextract(Type::I64, t, val, 0, 16);
                    t
                }
                2 => {
                    let t32 = ir.new_temp(Type::I32);
                    ir.gen_extrl_i64_i32(t32, val);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_ext_i32_i64(t, t32);
                    t
                }
                3 => val,
                _ => unreachable!(),
            }
        } else {
            extracted
        };
        if shift != 0 {
            let sh = ir.new_const(Type::I64, shift as u64);
            let t = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, t, sign_ext, sh);
            t
        } else {
            sign_ext
        }
    }

    /// Deposit `len` bits from `src` into `dst` at bit position
    /// `ofs`. Equivalent to `gen_deposit` but uses shift/mask
    /// operations that the backend supports.
    fn deposit(
        ir: &mut Context,
        ty: Type,
        dst: TempIdx,
        src: TempIdx,
        ofs: u32,
        len: u32,
        sf: bool,
    ) -> TempIdx {
        if ofs == 0 && (len == 8 || len == 16) {
            // Simple case: use gen_deposit
            let d = ir.new_temp(ty);
            ir.gen_deposit(ty, d, dst, src, 0, len);
            d
        } else {
            // Build (dst & ~(mask << ofs)) | ((src & mask) << ofs)
            // Use minimal temps to avoid register pressure
            let bits = if sf { 64u32 } else { 32u32 };
            let mask_val = if len >= bits {
                if sf {
                    u64::MAX
                } else {
                    0xffff_ffff
                }
            } else {
                (1u64 << len) - 1
            };
            let shifted_mask = mask_val << ofs;
            let inv_mask = if sf {
                !shifted_mask
            } else {
                (!shifted_mask) & 0xffff_ffff
            };

            // t1 = dst & ~(mask << ofs)
            let im = ir.new_const(ty, inv_mask);
            let t1 = ir.new_temp(ty);
            ir.gen_and(ty, t1, dst, im);

            // t2 = (src & mask) << ofs
            // Reuse const slots
            let m = ir.new_const(ty, mask_val);
            let t2 = ir.new_temp(ty);
            ir.gen_and(ty, t2, src, m);
            if ofs > 0 {
                let sh = ir.new_const(ty, ofs as u64);
                ir.gen_shl(ty, t2, t2, sh);
            }

            ir.gen_or(ty, t1, t1, t2);
            t1
        }
    }

    // -- VREG helpers (env-relative load/store) --

    fn read_vreg_lo(&self, ir: &mut Context, reg: usize) -> TempIdx {
        let d = ir.new_temp(Type::I64);
        ir.gen_ld(Type::I64, d, self.env, vreg_lo_offset(reg));
        d
    }

    fn read_vreg_hi(&self, ir: &mut Context, reg: usize) -> TempIdx {
        let d = ir.new_temp(Type::I64);
        ir.gen_ld(Type::I64, d, self.env, vreg_hi_offset(reg));
        d
    }

    fn write_vreg_lo(&self, ir: &mut Context, reg: usize, val: TempIdx) {
        ir.gen_st(Type::I64, val, self.env, vreg_lo_offset(reg));
    }

    fn write_vreg_hi(&self, ir: &mut Context, reg: usize, val: TempIdx) {
        ir.gen_st(Type::I64, val, self.env, vreg_hi_offset(reg));
    }

    /// Write full 128-bit vreg (lo, hi).
    #[allow(dead_code)]
    fn write_vreg128(
        &self,
        ir: &mut Context,
        reg: usize,
        lo: TempIdx,
        hi: TempIdx,
    ) {
        self.write_vreg_lo(ir, reg, lo);
        self.write_vreg_hi(ir, reg, hi);
    }

    /// Zero the high half of a vreg.
    fn clear_vreg_hi(&self, ir: &mut Context, reg: usize) {
        let z = ir.new_const(Type::I64, 0);
        self.write_vreg_hi(ir, reg, z);
    }
}

// ── NEON / FP manual decoder ────────────────────────────

impl Aarch64DisasContext {
    /// Try to decode and translate a NEON/FP instruction.
    /// Returns true if handled.
    pub(crate) fn try_neon(&mut self, ir: &mut Context, insn: u32) -> bool {
        // Dispatch by top-level encoding groups
        let op0 = (insn >> 25) & 0xf;
        match op0 {
            // Load/Store SIMD & FP
            0b0100 | 0b0110 | 0b1100 | 0b1110 => self.try_fp_ldst(ir, insn),
            // Data processing — SIMD & FP
            0b0111 | 0b1111 => self.try_fp_data(ir, insn),
            _ => false,
        }
    }
}

// ── FP/SIMD load/store ──────────────────────────────────

impl Aarch64DisasContext {
    fn try_fp_ldst(&mut self, ir: &mut Context, insn: u32) -> bool {
        let op3 = (insn >> 10) & 0x3;
        let top6 = (insn >> 24) & 0x3f;

        // LDR/STR (unsigned immediate) — SIMD & FP
        // xx 111 101 opc imm12 rn rt
        if top6 == 0b111101 {
            return self.fp_ldst_uimm(ir, insn);
        }

        // LDR (literal) — SIMD & FP
        // opc 011 100 imm19 rt
        if top6 == 0b011100 {
            return self.fp_ldr_literal(ir, insn);
        }

        // LDUR/STUR — SIMD & FP
        // xx 111 100 x0 imm9 00 rn rt
        if top6 == 0b111100 && (insn >> 21) & 1 == 0 && op3 == 0 {
            return self.fp_ldst_unscaled(ir, insn);
        }

        // LDR/STR pre/post-index — SIMD & FP
        if top6 == 0b111100 && (insn >> 21) & 1 == 0 && (op3 == 1 || op3 == 3) {
            return self.fp_ldst_prepost(ir, insn);
        }

        // LDR/STR register offset — SIMD & FP
        if top6 == 0b111100 && (insn >> 21) & 1 == 1 && op3 == 2 {
            return self.fp_ldst_reg(ir, insn);
        }

        // LDP/STP — SIMD & FP
        // opc 101 1xx imm7 rt2 rn rt  (opc=00→S, 01→D, 10→Q)
        if (insn >> 26) & 0xf == 0b1011 {
            return self.fp_ldst_pair(ir, insn);
        }

        // LD1/ST1 multiple structures — 0 Q 001100 0 L 000000 opcode size Rn Rt
        if top6 == 0b001100 && (insn >> 21) & 1 == 0 {
            return self.fp_ldst_multiple(ir, insn);
        }

        // LD1/ST1 single structure — 0 Q 001101 0 L R opcode S size Rn Rt
        if top6 == 0b001101 {
            return self.fp_ldst_single(ir, insn);
        }

        false
    }
}

// ── FP/SIMD load/store implementations ──────────────────

impl Aarch64DisasContext {
    /// Decode size/opc fields for FP/SIMD load/store.
    /// Returns (log2_bytes, is_128bit, is_load).
    fn fp_ldst_size(size: u32, opc: u32) -> Option<(u32, bool, bool)> {
        // size[31:30], opc[23:22]
        match (size, opc) {
            (0b00, 0b00) => Some((0, false, false)), // STR B
            (0b00, 0b01) => Some((0, false, true)),  // LDR B
            (0b00, 0b10) => Some((4, true, false)),  // STR Q
            (0b00, 0b11) => Some((4, true, true)),   // LDR Q
            (0b01, 0b00) => Some((1, false, false)), // STR H
            (0b01, 0b01) => Some((1, false, true)),  // LDR H
            (0b10, 0b00) => Some((2, false, false)), // STR S
            (0b10, 0b01) => Some((2, false, true)),  // LDR S
            (0b11, 0b00) => Some((3, false, false)), // STR D
            (0b11, 0b01) => Some((3, false, true)),  // LDR D
            _ => None,
        }
    }

    fn fp_do_load(
        &mut self,
        ir: &mut Context,
        reg: usize,
        addr: TempIdx,
        log2: u32,
        is_128: bool,
    ) {
        if is_128 {
            let lo = ir.new_temp(Type::I64);
            ir.gen_qemu_ld(Type::I64, lo, addr, MemOp::uq().bits() as u32);
            self.write_vreg_lo(ir, reg, lo);
            let c8 = ir.new_const(Type::I64, 8);
            let addr_hi = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, addr_hi, addr, c8);
            let hi = ir.new_temp(Type::I64);
            ir.gen_qemu_ld(Type::I64, hi, addr_hi, MemOp::uq().bits() as u32);
            self.write_vreg_hi(ir, reg, hi);
        } else {
            let memop = match log2 {
                2 => MemOp::ul(),
                3 => MemOp::uq(),
                _ => return,
            };
            let val = ir.new_temp(Type::I64);
            ir.gen_qemu_ld(Type::I64, val, addr, memop.bits() as u32);
            self.write_vreg_lo(ir, reg, val);
            self.clear_vreg_hi(ir, reg);
        }
    }

    fn fp_do_store(
        &mut self,
        ir: &mut Context,
        reg: usize,
        addr: TempIdx,
        log2: u32,
        is_128: bool,
    ) {
        if is_128 {
            let lo = self.read_vreg_lo(ir, reg);
            ir.gen_qemu_st(Type::I64, lo, addr, MemOp::uq().bits() as u32);
            let c8 = ir.new_const(Type::I64, 8);
            let addr_hi = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, addr_hi, addr, c8);
            let hi = self.read_vreg_hi(ir, reg);
            ir.gen_qemu_st(Type::I64, hi, addr_hi, MemOp::uq().bits() as u32);
        } else {
            let memop = match log2 {
                2 => MemOp::ul(),
                3 => MemOp::uq(),
                _ => return,
            };
            let val = self.read_vreg_lo(ir, reg);
            ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        }
    }

    /// LDR/STR (unsigned immediate) — SIMD & FP
    fn fp_ldst_uimm(&mut self, ir: &mut Context, insn: u32) -> bool {
        let size = (insn >> 30) & 0x3;
        let opc = (insn >> 22) & 0x3;
        let imm12 = ((insn >> 10) & 0xfff) as i64;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let (log2, is_128, is_load) = match Self::fp_ldst_size(size, opc) {
            Some(v) => v,
            None => return false,
        };
        let offset = imm12 << log2;
        let addr = self.compute_addr_imm(ir, rn, offset);
        if is_load {
            self.fp_do_load(ir, rt, addr, log2, is_128);
        } else {
            self.fp_do_store(ir, rt, addr, log2, is_128);
        }
        true
    }

    /// LDR (literal) — SIMD & FP
    fn fp_ldr_literal(&mut self, ir: &mut Context, insn: u32) -> bool {
        let opc = (insn >> 30) & 0x3;
        let imm19 = ((insn >> 5) & 0x7ffff) as i64;
        let rt = (insn & 0x1f) as usize;
        let offset = (imm19 << 45) >> 43; // sign-extend * 4
        let pc = self.base.pc_next as i64;
        let addr_val = (pc + offset) as u64;
        let addr = ir.new_const(Type::I64, addr_val);
        let (log2, is_128) = match opc {
            0b00 => (2u32, false),
            0b01 => (3, false),
            0b10 => (4, true),
            _ => return false,
        };
        self.fp_do_load(ir, rt, addr, log2, is_128);
        true
    }

    /// LDUR/STUR — SIMD & FP (unscaled)
    fn fp_ldst_unscaled(&mut self, ir: &mut Context, insn: u32) -> bool {
        let size = (insn >> 30) & 0x3;
        let opc = (insn >> 22) & 0x3;
        let imm9 = (((insn >> 12) & 0x1ff) as i32 as i64) << 55 >> 55;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let (log2, is_128, is_load) = match Self::fp_ldst_size(size, opc) {
            Some(v) => v,
            None => return false,
        };
        let addr = self.compute_addr_imm(ir, rn, imm9);
        if is_load {
            self.fp_do_load(ir, rt, addr, log2, is_128);
        } else {
            self.fp_do_store(ir, rt, addr, log2, is_128);
        }
        true
    }

    /// LDR/STR pre/post-index — SIMD & FP
    fn fp_ldst_prepost(&mut self, ir: &mut Context, insn: u32) -> bool {
        let size = (insn >> 30) & 0x3;
        let opc = (insn >> 22) & 0x3;
        let imm9 = (((insn >> 12) & 0x1ff) as i32 as i64) << 55 >> 55;
        let is_pre = (insn >> 11) & 1 != 0;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let (log2, is_128, is_load) = match Self::fp_ldst_size(size, opc) {
            Some(v) => v,
            None => return false,
        };
        let base = self.read_xreg_sp(ir, rn);
        let offset_c = ir.new_const(Type::I64, imm9 as u64);
        let new_base = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, new_base, base, offset_c);
        let addr = if is_pre { new_base } else { base };
        if is_load {
            self.fp_do_load(ir, rt, addr, log2, is_128);
        } else {
            self.fp_do_store(ir, rt, addr, log2, is_128);
        }
        self.write_xreg_sp(ir, rn, new_base);
        true
    }

    /// LDR/STR register offset — SIMD & FP
    fn fp_ldst_reg(&mut self, ir: &mut Context, insn: u32) -> bool {
        let size = (insn >> 30) & 0x3;
        let opc = (insn >> 22) & 0x3;
        let rm = ((insn >> 16) & 0x1f) as i64;
        let option = ((insn >> 13) & 0x7) as i64;
        let s = (insn >> 12) & 1;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let (log2, is_128, is_load) = match Self::fp_ldst_size(size, opc) {
            Some(v) => v,
            None => return false,
        };
        let shift = if s != 0 { log2 as i64 } else { 0 };
        let addr = self.compute_addr_reg(ir, rn, rm, option, shift);
        if is_load {
            self.fp_do_load(ir, rt, addr, log2, is_128);
        } else {
            self.fp_do_store(ir, rt, addr, log2, is_128);
        }
        true
    }

    /// LDP/STP — SIMD & FP
    fn fp_ldst_pair(&mut self, ir: &mut Context, insn: u32) -> bool {
        let opc = (insn >> 30) & 0x3;
        let is_load = (insn >> 22) & 1 != 0;
        let imm7 = (((insn >> 15) & 0x7f) as i32 as i64) << 57 >> 57;
        let rt2 = ((insn >> 10) & 0x1f) as usize;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let wback = ((insn >> 23) & 1) != 0;
        let is_pre = wback && ((insn >> 24) & 1) != 0;
        let is_post = wback && ((insn >> 24) & 1) == 0;
        let (log2, is_128) = match opc {
            0b00 => (2u32, false),
            0b01 => (3, false),
            0b10 => (4, true),
            _ => return false,
        };
        let scale = 2 + opc;
        let offset = imm7 << scale;
        let base = self.read_xreg_sp(ir, rn);
        let addr = if is_pre || !wback {
            if offset != 0 {
                let c = ir.new_const(Type::I64, offset as u64);
                let t = ir.new_temp(Type::I64);
                ir.gen_add(Type::I64, t, base, c);
                t
            } else {
                base
            }
        } else {
            base
        };
        let elem_bytes = 1u64 << log2;
        if is_load {
            self.fp_do_load(ir, rt, addr, log2, is_128);
            let c = ir.new_const(Type::I64, elem_bytes);
            let addr2 = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, addr2, addr, c);
            self.fp_do_load(ir, rt2, addr2, log2, is_128);
        } else {
            self.fp_do_store(ir, rt, addr, log2, is_128);
            let c = ir.new_const(Type::I64, elem_bytes);
            let addr2 = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, addr2, addr, c);
            self.fp_do_store(ir, rt2, addr2, log2, is_128);
        }
        if wback {
            let new_base = if is_post {
                let c = ir.new_const(Type::I64, offset as u64);
                let t = ir.new_temp(Type::I64);
                ir.gen_add(Type::I64, t, base, c);
                t
            } else {
                addr
            };
            self.write_xreg_sp(ir, rn, new_base);
        }
        true
    }

    /// LD1/ST1 multiple structures (single register, no offset).
    fn fp_ldst_multiple(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let is_load = (insn >> 22) & 1 != 0;
        let opcode = (insn >> 12) & 0xf;
        let size = (insn >> 10) & 0x3;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;

        let bytes_per_reg: u64 = if q != 0 { 16 } else { 8 };
        let addr = self.read_xreg_sp(ir, rn);

        // LD1/ST1 — contiguous (non-interleaved)
        let ld1_nregs = match opcode {
            0b0111 => Some(1),
            0b1010 => Some(2),
            0b0110 => Some(3),
            0b0010 => Some(4),
            _ => None,
        };

        if let Some(nregs) = ld1_nregs {
            for i in 0..nregs {
                let reg = (rt + i) & 31;
                let off = (i as u64) * bytes_per_reg;
                let cur_addr = if off != 0 {
                    let c = ir.new_const(Type::I64, off);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_add(Type::I64, t, addr, c);
                    t
                } else {
                    addr
                };
                if is_load {
                    let lo = ir.new_temp(Type::I64);
                    ir.gen_qemu_ld(
                        Type::I64,
                        lo,
                        cur_addr,
                        MemOp::uq().bits() as u32,
                    );
                    self.write_vreg_lo(ir, reg, lo);
                    if q != 0 {
                        let c8 = ir.new_const(Type::I64, 8);
                        let hi_addr = ir.new_temp(Type::I64);
                        ir.gen_add(Type::I64, hi_addr, cur_addr, c8);
                        let hi = ir.new_temp(Type::I64);
                        ir.gen_qemu_ld(
                            Type::I64,
                            hi,
                            hi_addr,
                            MemOp::uq().bits() as u32,
                        );
                        self.write_vreg_hi(ir, reg, hi);
                    } else {
                        self.clear_vreg_hi(ir, reg);
                    }
                } else {
                    let lo = self.read_vreg_lo(ir, reg);
                    ir.gen_qemu_st(
                        Type::I64,
                        lo,
                        cur_addr,
                        MemOp::uq().bits() as u32,
                    );
                    if q != 0 {
                        let c8 = ir.new_const(Type::I64, 8);
                        let hi_addr = ir.new_temp(Type::I64);
                        ir.gen_add(Type::I64, hi_addr, cur_addr, c8);
                        let hi = self.read_vreg_hi(ir, reg);
                        ir.gen_qemu_st(
                            Type::I64,
                            hi,
                            hi_addr,
                            MemOp::uq().bits() as u32,
                        );
                    }
                }
            }

            // Post-index writeback
            let post_index = (insn >> 23) & 1 != 0;
            if post_index && rn != 31 {
                let rm = ((insn >> 16) & 0x1f) as i64;
                let total_bytes = (nregs as u64) * bytes_per_reg;
                let inc = if rm == 31 {
                    ir.new_const(Type::I64, total_bytes)
                } else {
                    self.read_xreg(ir, rm)
                };
                let new_addr = ir.new_temp(Type::I64);
                ir.gen_add(Type::I64, new_addr, addr, inc);
                self.write_xreg(ir, rn, new_addr);
            }
            return true;
        }

        // LD2/ST2, LD3/ST3, LD4/ST4 — interleaved structure loads/stores
        // opcode: 0b1000=LD2/ST2, 0b0100=LD3/ST3, 0b0000=LD4/ST4
        let nregs: usize = match opcode {
            0b1000 => 2,
            0b0100 => 3,
            0b0000 => 4,
            _ => return false,
        };
        // Element size in bytes
        let elem_bytes: u64 = 1 << size;
        let nelems: u64 = bytes_per_reg / elem_bytes;
        let memop = match elem_bytes {
            1 => MemOp::ub(),
            2 => MemOp::uw(),
            4 => MemOp::ul(),
            8 => MemOp::uq(),
            _ => return false,
        };

        if is_load {
            // Load: read nregs*nelems interleaved elements, de-interleave into nregs vector regs
            // For each element index j in [0..nelems], load nregs consecutive elements at
            // addr + j*nregs*elem_bytes, and scatter them into reg[0..nregs][j]
            // We do this via a call helper — but to keep IR simple, use a temp array approach:
            // Actually, emit nregs*nelems individual element loads and insert into registers.
            let total_elems = nelems * nregs as u64;
            // Pre-clear all dest regs
            for i in 0..nregs {
                let reg = (rt + i) & 31;
                let zero = ir.new_const(Type::I64, 0);
                self.write_vreg_lo(ir, reg, zero);
                if q != 0 {
                    self.write_vreg_hi(ir, reg, zero);
                } else {
                    self.clear_vreg_hi(ir, reg);
                }
            }
            for flat_idx in 0..total_elems {
                let byte_off = flat_idx * elem_bytes;
                let cur_addr = if byte_off != 0 {
                    let c = ir.new_const(Type::I64, byte_off);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_add(Type::I64, t, addr, c);
                    t
                } else {
                    addr
                };
                let val = ir.new_temp(Type::I64);
                ir.gen_qemu_ld(Type::I64, val, cur_addr, memop.bits() as u32);
                // De-interleave: element flat_idx goes to reg[flat_idx % nregs], element [flat_idx / nregs]
                let dest_reg = (rt + (flat_idx as usize % nregs)) & 31;
                let elem_idx = flat_idx / nregs as u64;
                let bit_off = (elem_idx * elem_bytes * 8) % 64;
                let is_hi = (elem_idx * elem_bytes * 8) >= 64;
                // Read current half, insert element, write back
                let half = if is_hi {
                    self.read_vreg_hi(ir, dest_reg)
                } else {
                    self.read_vreg_lo(ir, dest_reg)
                };
                let elem_mask = if elem_bytes == 8 {
                    !0u64
                } else {
                    (1u64 << (elem_bytes * 8)) - 1
                };
                let cmask = ir.new_const(Type::I64, !(elem_mask << bit_off));
                let cleared = ir.new_temp(Type::I64);
                ir.gen_and(Type::I64, cleared, half, cmask);
                let inserted = if bit_off > 0 {
                    let sh = ir.new_const(Type::I64, bit_off);
                    let shifted = ir.new_temp(Type::I64);
                    ir.gen_shl(Type::I64, shifted, val, sh);
                    let result = ir.new_temp(Type::I64);
                    ir.gen_or(Type::I64, result, cleared, shifted);
                    result
                } else {
                    let result = ir.new_temp(Type::I64);
                    ir.gen_or(Type::I64, result, cleared, val);
                    result
                };
                if is_hi {
                    self.write_vreg_hi(ir, dest_reg, inserted);
                } else {
                    self.write_vreg_lo(ir, dest_reg, inserted);
                }
            }
        } else {
            // Store: read nregs vector regs, interleave elements, store to memory
            let total_elems = nelems * nregs as u64;
            for flat_idx in 0..total_elems {
                let src_reg = (rt + (flat_idx as usize % nregs)) & 31;
                let elem_idx = flat_idx / nregs as u64;
                let bit_off = (elem_idx * elem_bytes * 8) % 64;
                let is_hi = (elem_idx * elem_bytes * 8) >= 64;
                let half = if is_hi {
                    self.read_vreg_hi(ir, src_reg)
                } else {
                    self.read_vreg_lo(ir, src_reg)
                };
                let val = if bit_off > 0 {
                    let sh = ir.new_const(Type::I64, bit_off);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_shr(Type::I64, t, half, sh);
                    t
                } else {
                    half
                };
                let byte_off = flat_idx * elem_bytes;
                let cur_addr = if byte_off != 0 {
                    let c = ir.new_const(Type::I64, byte_off);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_add(Type::I64, t, addr, c);
                    t
                } else {
                    addr
                };
                ir.gen_qemu_st(Type::I64, val, cur_addr, memop.bits() as u32);
            }
        }

        // Post-index writeback
        let post_index = (insn >> 23) & 1 != 0;
        if post_index && rn != 31 {
            let rm = ((insn >> 16) & 0x1f) as i64;
            let total_bytes = nregs as u64 * bytes_per_reg;
            let inc = if rm == 31 {
                ir.new_const(Type::I64, total_bytes)
            } else {
                self.read_xreg(ir, rm)
            };
            let new_addr = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, new_addr, addr, inc);
            self.write_xreg(ir, rn, new_addr);
        }

        true
    }

    /// LD1/ST1 single structure: load/store one element from/to a vector register.
    /// Encoding: 0 Q 001101 0 L R opcode S size Rn Rt
    fn fp_ldst_single(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let is_load = (insn >> 22) & 1 != 0;
        let opcode = (insn >> 13) & 7;
        let s = (insn >> 12) & 1;
        let size = (insn >> 10) & 3;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;
        let addr = self.read_xreg_sp(ir, rn);

        // LD1R: opcode=110, S=0, L=1 — load one element and replicate to all lanes
        if opcode == 0b110 && s == 0 && is_load {
            let (elem_bits, memop) = match size {
                0b00 => (8, MemOp::ub()),
                0b01 => (16, MemOp::uw()),
                0b10 => (32, MemOp::ul()),
                0b11 => (64, MemOp::uq()),
                _ => unreachable!(),
            };
            let val = ir.new_temp(Type::I64);
            ir.gen_qemu_ld(Type::I64, val, addr, memop.bits() as u32);
            // Replicate element across 64-bit lane
            let rep = match elem_bits {
                8 => {
                    let mask = ir.new_const(Type::I64, 0xff);
                    let masked = ir.new_temp(Type::I64);
                    ir.gen_and(Type::I64, masked, val, mask);
                    let m = ir.new_const(Type::I64, 0x0101_0101_0101_0101u64);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_mul(Type::I64, t, masked, m);
                    t
                }
                16 => {
                    let mask = ir.new_const(Type::I64, 0xffff);
                    let masked = ir.new_temp(Type::I64);
                    ir.gen_and(Type::I64, masked, val, mask);
                    let m = ir.new_const(Type::I64, 0x0001_0001_0001_0001u64);
                    let t = ir.new_temp(Type::I64);
                    ir.gen_mul(Type::I64, t, masked, m);
                    t
                }
                32 => {
                    let t = ir.new_temp(Type::I64);
                    let c32 = ir.new_const(Type::I64, 32);
                    let hi = ir.new_temp(Type::I64);
                    ir.gen_shl(Type::I64, hi, val, c32);
                    ir.gen_or(Type::I64, t, val, hi);
                    t
                }
                64 => val,
                _ => unreachable!(),
            };
            self.write_vreg_lo(ir, rt, rep);
            if q != 0 {
                self.write_vreg_hi(ir, rt, rep);
            } else {
                self.clear_vreg_hi(ir, rt);
            }
            return true;
        }

        // Determine element size and index
        let (elem_bits, idx) = match opcode {
            0b000 => (8, (q << 3) | (s << 2) | size), // B
            0b010 => (16, (q << 2) | (s << 1) | (size >> 1)), // H
            0b100 if size == 0 => (32, (q << 1) | s), // S
            0b100 if size == 1 && s == 0 => (64, q),  // D
            _ => return false,
        };

        let idx = idx as usize;
        let byte_off = (idx * elem_bits) / 8;
        let is_hi = byte_off >= 8;
        let bit_off = (byte_off % 8) * 8;

        if is_load {
            let val = ir.new_temp(Type::I64);
            let memop = match elem_bits {
                8 => MemOp::ub(),
                16 => MemOp::uw(),
                32 => MemOp::ul(),
                64 => MemOp::uq(),
                _ => unreachable!(),
            };
            ir.gen_qemu_ld(Type::I64, val, addr, memop.bits() as u32);
            let half = if is_hi {
                self.read_vreg_hi(ir, rt)
            } else {
                self.read_vreg_lo(ir, rt)
            };
            let elem_mask = if elem_bits == 64 {
                !0u64
            } else {
                (1u64 << elem_bits) - 1
            };
            let cmask = ir.new_const(Type::I64, !(elem_mask << bit_off));
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, half, cmask);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shl(Type::I64, val, val, sh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, val);
            if is_hi {
                self.write_vreg_hi(ir, rt, result);
            } else {
                self.write_vreg_lo(ir, rt, result);
            }
        } else {
            // Store
            let half = if is_hi {
                self.read_vreg_hi(ir, rt)
            } else {
                self.read_vreg_lo(ir, rt)
            };
            let val = ir.new_temp(Type::I64);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shr(Type::I64, val, half, sh);
            } else {
                ir.gen_mov(Type::I64, val, half);
            }
            let memop = match elem_bits {
                8 => MemOp::ub(),
                16 => MemOp::uw(),
                32 => MemOp::ul(),
                64 => MemOp::uq(),
                _ => unreachable!(),
            };
            ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        }
        true
    }

    /// FP/SIMD data processing — handles DUP, UMOV, and other
    /// NEON instructions needed by glibc.
    fn try_fp_data(&mut self, ir: &mut Context, insn: u32) -> bool {
        // DUP (general) — 0 Q 00 1110 000 imm5 0 0001 1 Rn Rd
        // Encodes as: 0x0e000c00 mask 0xbfe0fc00
        // DUP (element) — 0 Q 00 1110 000 imm5 0 0000 1 Rn Rd
        if insn & 0xbfe0_fc00 == 0x0e00_0400 {
            return self.neon_dup_element(ir, insn);
        }
        if insn & 0xbfe0_fc00 == 0x0e00_0c00 {
            return self.neon_dup_general(ir, insn);
        }
        // UMOV / MOV (to general) — 0 Q 00 1110 000 imm5 0 0111 1 Rn Rd
        // Encodes as: 0x0e003c00 mask 0xbfe0fc00
        if insn & 0xbfe0_fc00 == 0x0e00_3c00 {
            return self.neon_umov(ir, insn);
        }
        // INS (general) — 0100 1110 000 imm5 0 0011 1 Rn Rd
        if insn & 0xffe0_fc00 == 0x4e00_1c00 {
            return self.neon_ins_general(ir, insn);
        }
        // INS (element) — 0110 1110 000 imm5 0 imm4 1 Rn Rd
        if insn & 0xffe0_8400 == 0x6e00_0400 {
            return self.neon_ins_element(ir, insn);
        }
        // TBL/TBX — 0 Q 00 1110 000 Rm 0 len op 00 Rn Rd
        if insn & 0xbfe0_0c00 == 0x0e00_0000 {
            return self.neon_tbl(ir, insn);
        }
        // MOVI/MVNI — 0 Q op 0 1111 00 abc cmode 01 defgh Rd
        if insn & 0x9ff8_0400 == 0x0f00_0400 {
            return self.neon_movi(ir, insn);
        }
        // FMOV Xd, Dn — 1001 1110 0110 0110 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x9e66_0000 {
            return self.neon_fmov_to_gpr(ir, insn);
        }
        // FMOV Dn, #imm — 0001 1110 011 1 imm8 100 00000 Rd
        if insn & 0xffe0_1fe0 == 0x1e60_1000 {
            return self.neon_fmov_imm(ir, insn);
        }
        // FMOV Sn, #imm — 0001 1110 001 1 imm8 100 00000 Rd
        if insn & 0xffe0_1fe0 == 0x1e20_1000 {
            return self.neon_fmov_imm(ir, insn);
        }
        // FMADD/FMSUB: 0001 1111 0T o1 Rm o0 Ra Rn Rd
        if insn >> 24 == 0x1f {
            let is_double = (insn >> 22) & 1 != 0;
            let o1 = (insn >> 21) & 1;
            let rm = ((insn >> 16) & 0x1f) as usize;
            let o0 = (insn >> 15) & 1;
            let ra = ((insn >> 10) & 0x1f) as usize;
            let rn = ((insn >> 5) & 0x1f) as usize;
            let rd = (insn & 0x1f) as usize;
            let n = self.read_vreg_lo(ir, rn);
            let m = self.read_vreg_lo(ir, rm);
            let a = self.read_vreg_lo(ir, ra);
            let d = ir.new_temp(Type::I64);
            // o1=0,o0=0: FMADD; o1=0,o0=1: FMSUB; o1=1,o0=0: FNMADD; o1=1,o0=1: FNMSUB
            let helper = match (o1, o0, is_double) {
                (0, 0, true) => helper_fmadd64 as u64,
                (0, 1, true) => helper_fmsub64 as u64,
                (1, 0, true) => helper_fnmadd64 as u64,
                (1, 1, true) => helper_fnmsub64 as u64,
                (0, 0, false) => helper_fmadd32 as u64,
                (0, 1, false) => helper_fmsub32 as u64,
                (1, 0, false) => helper_fnmadd32 as u64,
                (1, 1, false) => helper_fnmsub32 as u64,
                _ => return false,
            };
            ir.gen_call(d, helper, &[n, m, a]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // FP data-processing (scalar): 0001 111x ...
        if insn >> 24 == 0x1e || insn >> 24 == 0x9e {
            if let Some(r) = self.try_fp_scalar(ir, insn) {
                return r;
            }
        }
        // EXT: 0 Q 10 1110 000 Rm 0 imm4 0 Rn Rd
        if insn & 0xbfe0_8400 == 0x2e00_0000 {
            return self.try_neon_ext(ir, insn);
        }
        // AdvSIMD vector × indexed element: 0 Q U 0 1111 size L M Rm opcode H 0 Rn Rd
        if insn & 0x9f00_0400 == 0x0f00_0000 {
            if let Some(r) = self.neon_indexed_element(ir, insn) {
                return r;
            }
        }
        // Scalar AdvSIMD 2-reg misc: 01 U 11110 size 10000 opcode 10 Rn Rd
        // CMGE d,d,#0 (U=1 size=11 opcode=01000) / CMGT d,d,#0 (U=0 size=11 opcode=01000)
        // CMLE d,d,#0 (U=1 size=11 opcode=01001) / CMEQ d,d,#0 (U=0 size=11 opcode=01001)
        // CMLT d,d,#0 (U=0 size=11 opcode=01010)
        // SCVTF scalar (s,s) / SCVTF scalar (d,d): opcode=11101, size=00→single, size=11→double
        if insn & 0xdf3e_0c00 == 0x5e20_0800 {
            let u = (insn >> 29) & 1;
            let size = (insn >> 22) & 3;
            let opcode = (insn >> 12) & 0x1f;
            let rn = ((insn >> 5) & 0x1f) as usize;
            let rd = (insn & 0x1f) as usize;
            let src = self.read_vreg_lo(ir, rn);
            let zero = ir.new_const(Type::I64, 0);
            let d = ir.new_temp(Type::I64);
            match (u, opcode) {
                (1, 0b01000) => {
                    // CMGE #0
                    ir.gen_call(d, helper_cmge_scalar as u64, &[src]);
                }
                (0, 0b01000) => {
                    // CMGT #0
                    ir.gen_call(d, helper_cmgt_scalar as u64, &[src]);
                }
                (1, 0b01001) => {
                    // CMLE #0
                    ir.gen_call(d, helper_cmle_scalar as u64, &[src]);
                }
                (0, 0b01001) => {
                    // CMEQ #0
                    let _ = d;
                    let d2 = ir.new_temp(Type::I64);
                    ir.gen_setcond(Type::I64, d2, src, zero, Cond::Eq);
                    ir.gen_neg(Type::I64, d2, d2);
                    self.write_vreg_lo(ir, rd, d2);
                    self.clear_vreg_hi(ir, rd);
                    return true;
                }
                (1, 0b11101) => {
                    // UCVTF scalar: int-in-reg → float
                    let helper = if size == 0b11 {
                        helper_ucvtf_d_x as u64
                    } else {
                        helper_ucvtf_s_s as u64
                    };
                    ir.gen_call(d, helper, &[src]);
                }
                (0, 0b01100) => {
                    // FCMGT #0 scalar: (src > 0.0) ? -1 : 0
                    ir.gen_call(d, helper_fcmgt_zero_scalar as u64, &[src]);
                }
                (0, 0b11101) => {
                    // SCVTF scalar: int-in-reg → float
                    let helper = if size == 0b11 {
                        helper_scvtf_d_d as u64
                    } else {
                        helper_scvtf_s_s as u64
                    };
                    ir.gen_call(d, helper, &[src]);
                }
                (0, 0b11011) => {
                    // FCVTZS scalar: float → int (truncate toward zero)
                    let helper = if size == 0b11 {
                        helper_fcvtzs_x_d as u64
                    } else {
                        helper_fcvtzs_w_s as u64
                    };
                    ir.gen_call(d, helper, &[src]);
                }
                (1, 0b11011) => {
                    // FCVTZU scalar: float → uint (truncate toward zero)
                    let helper = if size == 0b11 {
                        helper_fcvtzu_x_d as u64
                    } else {
                        helper_fcvtzu_w_s as u64
                    };
                    ir.gen_call(d, helper, &[src]);
                }
                _ => return false,
            };
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // AdvSIMD scalar three same: 01 U 11110 size 1 Rm opcode 1 Rn Rd
        if insn & 0xdf20_0400 == 0x5e20_0400 {
            return self.neon_scalar_3same(ir, insn);
        }
        // AdvSIMD scalar shift by immediate: 01 U 111110 immh immb opcode 1 Rn Rd
        if insn & 0xdf80_0400 == 0x5f00_0400 {
            return self.neon_scalar_shift_imm(ir, insn);
        }
        // FADDP scalar: 01 1 11110 sz 1 10000 01101 10 Rn Rd (mask 0xff3e0c00 == 0x7e300800)
        if insn & 0xff3e_0c00 == 0x7e30_0800 {
            let sz = (insn >> 22) & 1; // 0=f32, 1=f64
            let rn = ((insn >> 5) & 0x1f) as usize;
            let rd = (insn & 0x1f) as usize;
            let n_lo = self.read_vreg_lo(ir, rn);
            let n_hi = self.read_vreg_hi(ir, rn);
            let d = ir.new_temp(Type::I64);
            if sz == 1 {
                // faddp d,v.2d: add the two f64 lanes
                ir.gen_call(d, helper_fadd64 as u64, &[n_lo, n_hi]);
            } else {
                // faddp s,v.2s: add the two f32 lanes
                ir.gen_call(d, helper_fadd32 as u64, &[n_lo, n_hi]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // ADDP scalar: 01 0 11110 size 11000 10111 10 Rn Rd (mask 0xff3ffc00 == 0x5e31b800)
        if insn & 0xff3f_fc00 == 0x5e31_b800 {
            let size = (insn >> 22) & 3;
            let rn = ((insn >> 5) & 0x1f) as usize;
            let rd = (insn & 0x1f) as usize;
            if size == 0b11 {
                // addp d, v.2d: add the two i64 lanes
                let n_lo = self.read_vreg_lo(ir, rn);
                let n_hi = self.read_vreg_hi(ir, rn);
                let d = ir.new_temp(Type::I64);
                ir.gen_add(Type::I64, d, n_lo, n_hi);
                self.write_vreg_lo(ir, rd, d);
                self.clear_vreg_hi(ir, rd);
                return true;
            }
            return false;
        }
        // Dispatch 3-same / 2-reg-misc / shift-imm by top bits
        self.try_neon_3same_misc(ir, insn)
    }

    /// DUP (element): replicate a vector element into all lanes.
    fn neon_dup_element(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let imm5 = (insn >> 16) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // Element size from lowest set bit of imm5
        if imm5 & 1 != 0 {
            // 8-bit: index = imm5[4:1]
            let idx = (imm5 >> 1) as usize;
            let half = if idx < 8 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let bit_off = (idx % 8) * 8;
            let sh = ir.new_const(Type::I64, bit_off as u64);
            let elem = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, elem, half, sh);
            let mask = ir.new_const(Type::I64, 0xff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let mul = ir.new_const(Type::I64, 0x0101_0101_0101_0101u64);
            let lo = ir.new_temp(Type::I64);
            ir.gen_mul(Type::I64, lo, elem, mul);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 2 != 0 {
            // 16-bit: index = imm5[4:2]
            let idx = (imm5 >> 2) as usize;
            let half = if idx < 4 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let bit_off = (idx % 4) * 16;
            let sh = ir.new_const(Type::I64, bit_off as u64);
            let elem = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, elem, half, sh);
            let mask = ir.new_const(Type::I64, 0xffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let mul = ir.new_const(Type::I64, 0x0001_0001_0001_0001u64);
            let lo = ir.new_temp(Type::I64);
            ir.gen_mul(Type::I64, lo, elem, mul);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 4 != 0 {
            // 32-bit: index = imm5[4:3]
            let idx = (imm5 >> 3) as usize;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let bit_off = (idx % 2) * 32;
            let sh = ir.new_const(Type::I64, bit_off as u64);
            let elem = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, elem, half, sh);
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let sh32 = ir.new_const(Type::I64, 32);
            let hi32 = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi32, elem, sh32);
            let lo = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, lo, elem, hi32);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 8 != 0 {
            // 64-bit: index = imm5[4]
            let idx = (imm5 >> 4) as usize;
            let half = if idx == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            self.write_vreg_lo(ir, rd, half);
            if q != 0 {
                self.write_vreg_hi(ir, rd, half);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else {
            return false;
        }
        true
    }

    /// DUP (general): replicate a GPR scalar into all vector lanes.
    fn neon_dup_general(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let imm5 = (insn >> 16) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rd = (insn & 0x1f) as usize;
        let src = self.read_xreg(ir, rn);

        // Determine element size from imm5 lowest set bit
        if imm5 & 1 != 0 {
            // 8-bit: replicate byte across 64-bit word
            let byte = ir.new_temp(Type::I64);
            let mask = ir.new_const(Type::I64, 0xff);
            ir.gen_and(Type::I64, byte, src, mask);
            // byte * 0x0101010101010101
            let mul = ir.new_const(Type::I64, 0x0101_0101_0101_0101u64);
            let lo = ir.new_temp(Type::I64);
            ir.gen_mul(Type::I64, lo, byte, mul);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 2 != 0 {
            // 16-bit
            let hw = ir.new_temp(Type::I64);
            let mask = ir.new_const(Type::I64, 0xffff);
            ir.gen_and(Type::I64, hw, src, mask);
            let mul = ir.new_const(Type::I64, 0x0001_0001_0001_0001u64);
            let lo = ir.new_temp(Type::I64);
            ir.gen_mul(Type::I64, lo, hw, mul);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 4 != 0 {
            // 32-bit
            let w = ir.new_temp(Type::I64);
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, w, src, mask);
            let sh = ir.new_const(Type::I64, 32);
            let hi32 = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi32, w, sh);
            let lo = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, lo, w, hi32);
            self.write_vreg_lo(ir, rd, lo);
            if q != 0 {
                self.write_vreg_hi(ir, rd, lo);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else if imm5 & 8 != 0 {
            // 64-bit
            self.write_vreg_lo(ir, rd, src);
            if q != 0 {
                self.write_vreg_hi(ir, rd, src);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
        } else {
            return false;
        }
        true
    }

    /// UMOV: extract a vector element to a GPR.
    fn neon_umov(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let imm5 = (insn >> 16) & 0x1f;
        let _rn = (insn & 0x1f) as usize; // Vn — source SIMD reg
                                          // Note: field at bits[9:5] is Rn (SIMD), bits[4:0] is Rd (GPR)
        let rn_simd = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as i64;

        // Determine element size and index from imm5
        if imm5 & 1 != 0 {
            // 8-bit element
            let idx = (imm5 >> 1) as u64;
            let half = if idx < 8 {
                self.read_vreg_lo(ir, rn_simd)
            } else {
                self.read_vreg_hi(ir, rn_simd)
            };
            let shift_amt = (idx % 8) * 8;
            let val = if shift_amt != 0 {
                let sh = ir.new_const(Type::I64, shift_amt);
                let t = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, t, half, sh);
                t
            } else {
                half
            };
            let mask = ir.new_const(Type::I64, 0xff);
            let result = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, result, val, mask);
            self.write_xreg(ir, rd, result);
        } else if imm5 & 2 != 0 {
            // 16-bit
            let idx = (imm5 >> 2) as u64;
            let half = if idx < 4 {
                self.read_vreg_lo(ir, rn_simd)
            } else {
                self.read_vreg_hi(ir, rn_simd)
            };
            let shift_amt = (idx % 4) * 16;
            let val = if shift_amt != 0 {
                let sh = ir.new_const(Type::I64, shift_amt);
                let t = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, t, half, sh);
                t
            } else {
                half
            };
            let mask = ir.new_const(Type::I64, 0xffff);
            let result = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, result, val, mask);
            self.write_xreg(ir, rd, result);
        } else if imm5 & 4 != 0 {
            // 32-bit
            let idx = (imm5 >> 3) as u64;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, rn_simd)
            } else {
                self.read_vreg_hi(ir, rn_simd)
            };
            let shift_amt = (idx % 2) * 32;
            let val = if shift_amt != 0 {
                let sh = ir.new_const(Type::I64, shift_amt);
                let t = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, t, half, sh);
                t
            } else {
                half
            };
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            let result = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, result, val, mask);
            self.write_xreg(ir, rd, result);
        } else if imm5 & 8 != 0 && q != 0 {
            // 64-bit (MOV Xd, Vn.d[idx])
            let idx = (imm5 >> 4) as u64;
            let val = if idx == 0 {
                self.read_vreg_lo(ir, rn_simd)
            } else {
                self.read_vreg_hi(ir, rn_simd)
            };
            self.write_xreg(ir, rd, val);
        } else {
            return false;
        }
        true
    }

    /// MOVI/MVNI: modified immediate to vector.
    fn neon_movi(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let op = (insn >> 29) & 1;
        let cmode = (insn >> 12) & 0xf;
        let abc = (insn >> 16) & 0x7;
        let defgh = (insn >> 5) & 0x1f;
        let rd = (insn & 0x1f) as usize;
        let imm8 = ((abc << 5) | defgh) as u64;

        // Build 64-bit element value based on cmode
        let elem64 = match cmode >> 1 {
            // 32-bit shifted: cmode=000x,001x,010x,011x
            0b000 => {
                let v = imm8;
                v | (v << 32)
            }
            0b001 => {
                let v = imm8 << 8;
                v | (v << 32)
            }
            0b010 => {
                let v = imm8 << 16;
                v | (v << 32)
            }
            0b011 => {
                let v = imm8 << 24;
                v | (v << 32)
            }
            // 16-bit shifted: cmode=100x,101x
            0b100 => {
                let v = imm8;
                v | (v << 16) | (v << 32) | (v << 48)
            }
            0b101 => {
                let v = imm8 << 8;
                v | (v << 16) | (v << 32) | (v << 48)
            }
            // 32-bit shifting ones: cmode=110x
            0b110 => {
                let v = if cmode & 1 == 0 {
                    (imm8 << 8) | 0xff
                } else {
                    (imm8 << 16) | 0xffff
                };
                v | (v << 32)
            }
            // cmode=1110: 8-bit or 64-bit
            0b111 => {
                if cmode & 1 == 0 && op == 0 {
                    // 8-bit: replicate byte
                    let v = imm8 & 0xff;
                    v * 0x0101_0101_0101_0101u64
                } else if cmode & 1 == 0 && op == 1 {
                    // MOVI 64-bit: each bit of imm8 → byte of 0x00 or 0xff
                    let mut v = 0u64;
                    for i in 0..8 {
                        if imm8 & (1 << i) != 0 {
                            v |= 0xffu64 << (i * 8);
                        }
                    }
                    v
                } else if cmode & 1 == 1 && op == 0 {
                    // FMOV 32-bit: VFPExpandImm → replicate 32-bit float to lanes
                    let a = (imm8 >> 7) & 1;
                    let b = (imm8 >> 6) & 1;
                    let not_b = 1 - b;
                    let exp8 =
                        (not_b << 7) | ((b * 0x1f) << 2) | ((imm8 >> 4) & 3);
                    let frac23 = (imm8 & 0xf) << 19;
                    let val32 = (a << 31) | (exp8 << 23) | frac23;
                    val32 | (val32 << 32)
                } else {
                    // FMOV 64-bit: op=1 cmode=1111 — VFPExpandImm → 64-bit float
                    let a = (imm8 >> 7) & 1;
                    let b = (imm8 >> 6) & 1;
                    let not_b = 1 - b;
                    let exp11 =
                        (not_b << 10) | ((b * 0xff) << 2) | ((imm8 >> 4) & 3);
                    let frac52 = (imm8 & 0xf) << 48;
                    (a << 63) | (exp11 << 52) | frac52
                }
            }
            _ => return false,
        };

        let val = if op == 1 && (cmode >> 1) < 0b111 {
            !elem64 // MVNI
        } else {
            elem64
        };

        let c = ir.new_const(Type::I64, val);
        self.write_vreg_lo(ir, rd, c);
        if q != 0 {
            self.write_vreg_hi(ir, rd, c);
        } else {
            self.clear_vreg_hi(ir, rd);
        }
        true
    }

    /// INS (general): insert a GPR value into a vector element.
    fn neon_ins_general(&mut self, ir: &mut Context, insn: u32) -> bool {
        let imm5 = (insn >> 16) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as i64; // GPR source
        let rd = (insn & 0x1f) as usize; // SIMD dest
        let src = self.read_xreg(ir, rn);

        if imm5 & 1 != 0 {
            // 8-bit: index = imm5[4:1]
            let idx = (imm5 >> 1) as usize;
            let is_hi = idx >= 8;
            let bit_off = (idx % 8) * 8;
            let half = if is_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let mask_val = !(0xffu64 << bit_off);
            let mask = ir.new_const(Type::I64, mask_val);
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, half, mask);
            let byte = ir.new_temp(Type::I64);
            let bmask = ir.new_const(Type::I64, 0xff);
            ir.gen_and(Type::I64, byte, src, bmask);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shl(Type::I64, byte, byte, sh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, byte);
            if is_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 2 != 0 {
            // 16-bit: index = imm5[4:2]
            let idx = (imm5 >> 2) as usize;
            let is_hi = idx >= 4;
            let bit_off = (idx % 4) * 16;
            let half = if is_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let mask_val = !(0xffffu64 << bit_off);
            let mask = ir.new_const(Type::I64, mask_val);
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, half, mask);
            let hw = ir.new_temp(Type::I64);
            let hmask = ir.new_const(Type::I64, 0xffff);
            ir.gen_and(Type::I64, hw, src, hmask);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shl(Type::I64, hw, hw, sh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, hw);
            if is_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 4 != 0 {
            // 32-bit: index = imm5[4:3]
            let idx = (imm5 >> 3) as usize;
            let is_hi = idx >= 2;
            let bit_off = (idx % 2) * 32;
            let half = if is_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let mask_val = !(0xffff_ffffu64 << bit_off);
            let mask = ir.new_const(Type::I64, mask_val);
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, half, mask);
            let w = ir.new_temp(Type::I64);
            let wmask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, w, src, wmask);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shl(Type::I64, w, w, sh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, w);
            if is_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 8 != 0 {
            // 64-bit: index = imm5[4]
            let idx = (imm5 >> 4) as usize;
            if idx == 0 {
                self.write_vreg_lo(ir, rd, src);
            } else {
                self.write_vreg_hi(ir, rd, src);
            }
        } else {
            return false;
        }
        true
    }

    /// TBL/TBX: byte-level table lookup.
    fn neon_tbl(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let len = (insn >> 13) & 3; // 0→1reg, 1→2reg, 2→3reg, 3→4reg
        let op = (insn >> 12) & 1; // 0→TBL, 1→TBX
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        if op != 0 {
            return false;
        } // TBX not yet implemented

        match len {
            0 => {
                // 1-reg TBL
                let t_lo = self.read_vreg_lo(ir, rn);
                let t_hi = self.read_vreg_hi(ir, rn);
                let idx_lo = self.read_vreg_lo(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_tbl1 as u64, &[t_lo, t_hi, idx_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let idx_hi = self.read_vreg_hi(ir, rm);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(
                        d_hi,
                        helper_tbl1 as u64,
                        &[t_lo, t_hi, idx_hi],
                    );
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            1 => {
                // 2-reg TBL: table = Vn, V(n+1)
                let rn2 = (rn + 1) & 31;
                let t0_lo = self.read_vreg_lo(ir, rn);
                let t0_hi = self.read_vreg_hi(ir, rn);
                let t1_lo = self.read_vreg_lo(ir, rn2);
                let t1_hi = self.read_vreg_hi(ir, rn2);
                let idx_lo = self.read_vreg_lo(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(
                    d_lo,
                    helper_tbl2 as u64,
                    &[t0_lo, t0_hi, t1_lo, t1_hi, idx_lo],
                );
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let idx_hi = self.read_vreg_hi(ir, rm);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(
                        d_hi,
                        helper_tbl2 as u64,
                        &[t0_lo, t0_hi, t1_lo, t1_hi, idx_hi],
                    );
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            _ => false,
        }
    }

    /// INS (element): copy element from one vector to another.
    /// Encoding: 0110 1110 000 imm5 0 imm4 1 Rn Rd
    fn neon_ins_element(&mut self, ir: &mut Context, insn: u32) -> bool {
        let imm5 = (insn >> 16) & 0x1f;
        let imm4 = (insn >> 11) & 0xf;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // Element size from lowest set bit of imm5
        if imm5 & 1 != 0 {
            // 8-bit: dst_idx = imm5[4:1], src_idx = imm4[3:0]
            let dst_idx = (imm5 >> 1) as usize;
            let src_idx = imm4 as usize;
            let src_half = if src_idx < 8 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let src_off = (src_idx % 8) * 8;
            let elem = ir.new_temp(Type::I64);
            let sh = ir.new_const(Type::I64, src_off as u64);
            ir.gen_shr(Type::I64, elem, src_half, sh);
            let mask = ir.new_const(Type::I64, 0xff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let dst_hi = dst_idx >= 8;
            let dst_off = (dst_idx % 8) * 8;
            let dst_half = if dst_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let cmask = ir.new_const(Type::I64, !(0xffu64 << dst_off));
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, dst_half, cmask);
            if dst_off > 0 {
                let dsh = ir.new_const(Type::I64, dst_off as u64);
                ir.gen_shl(Type::I64, elem, elem, dsh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, elem);
            if dst_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 2 != 0 {
            // 16-bit: dst_idx = imm5[4:2], src_idx = imm4[3:1]
            let dst_idx = (imm5 >> 2) as usize;
            let src_idx = (imm4 >> 1) as usize;
            let src_half = if src_idx < 4 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let src_off = (src_idx % 4) * 16;
            let elem = ir.new_temp(Type::I64);
            let sh = ir.new_const(Type::I64, src_off as u64);
            ir.gen_shr(Type::I64, elem, src_half, sh);
            let mask = ir.new_const(Type::I64, 0xffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let dst_hi = dst_idx >= 4;
            let dst_off = (dst_idx % 4) * 16;
            let dst_half = if dst_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let cmask = ir.new_const(Type::I64, !(0xffffu64 << dst_off));
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, dst_half, cmask);
            if dst_off > 0 {
                let dsh = ir.new_const(Type::I64, dst_off as u64);
                ir.gen_shl(Type::I64, elem, elem, dsh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, elem);
            if dst_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 4 != 0 {
            // 32-bit: dst_idx = imm5[4:3], src_idx = imm4[3:2]
            let dst_idx = (imm5 >> 3) as usize;
            let src_idx = (imm4 >> 2) as usize;
            let src_half = if src_idx < 2 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let src_off = (src_idx % 2) * 32;
            let elem = ir.new_temp(Type::I64);
            let sh = ir.new_const(Type::I64, src_off as u64);
            ir.gen_shr(Type::I64, elem, src_half, sh);
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let dst_hi = dst_idx >= 2;
            let dst_off = (dst_idx % 2) * 32;
            let dst_half = if dst_hi {
                self.read_vreg_hi(ir, rd)
            } else {
                self.read_vreg_lo(ir, rd)
            };
            let cmask = ir.new_const(Type::I64, !(0xffff_ffffu64 << dst_off));
            let cleared = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, cleared, dst_half, cmask);
            if dst_off > 0 {
                let dsh = ir.new_const(Type::I64, dst_off as u64);
                ir.gen_shl(Type::I64, elem, elem, dsh);
            }
            let result = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, result, cleared, elem);
            if dst_hi {
                self.write_vreg_hi(ir, rd, result);
            } else {
                self.write_vreg_lo(ir, rd, result);
            }
        } else if imm5 & 8 != 0 {
            // 64-bit: dst_idx = imm5[4], src_idx = imm4[3]
            let dst_idx = (imm5 >> 4) as usize;
            let src_idx = (imm4 >> 3) as usize;
            let val = if src_idx == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            if dst_idx == 0 {
                self.write_vreg_lo(ir, rd, val);
            } else {
                self.write_vreg_hi(ir, rd, val);
            }
        } else {
            return false;
        }
        true
    }

    /// FMOV Xd, Dn — move D register low half to GPR.
    fn neon_fmov_to_gpr(&mut self, ir: &mut Context, insn: u32) -> bool {
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as i64;
        let val = self.read_vreg_lo(ir, rn);
        self.write_xreg(ir, rd, val);
        true
    }

    fn neon_fmov_imm(&mut self, ir: &mut Context, insn: u32) -> bool {
        let rd = (insn & 0x1f) as usize;
        let imm8 = ((insn >> 13) & 0xff) as u64;
        let is_double = (insn >> 22) & 3 == 1;
        let bits = if is_double {
            // aBbbbbbb bbcdefgh 0{48}
            vfp_expand_imm64(imm8)
        } else {
            // aBbbbbbb bcdefgh0 0{24} — store as 64-bit (zero-extended)
            vfp_expand_imm32(imm8) as u64
        };
        let c = ir.new_const(Type::I64, bits);
        self.write_vreg_lo(ir, rd, c);
        let zero = ir.new_const(Type::I64, 0);
        self.write_vreg_hi(ir, rd, zero);
        true
    }

    fn try_fp_scalar(&mut self, ir: &mut Context, insn: u32) -> Option<bool> {
        let rd = (insn & 0x1f) as usize;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let ftype = (insn >> 22) & 3;
        let is_double = ftype == 1;

        // FCMP/FCMPE: x001 1110 xx1 xxxxx 00 1000 xxxxx 00000/10000
        if insn & 0x7f20_fc07 == 0x1e20_2000 {
            let a = self.read_vreg_lo(ir, rn);
            let b = if (insn >> 3) & 1 != 0 {
                ir.new_const(Type::I64, 0) // compare with zero
            } else {
                let rm = ((insn >> 16) & 0x1f) as usize;
                self.read_vreg_lo(ir, rm)
            };
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcmp64 as u64, &[a, b]);
            self.set_nzcv_eager(ir, d);
            return Some(true);
        }

        // FP two-source: x001 1110 xx1 Rm opcode 10 Rn Rd
        if insn & 0x5f20_0c00 == 0x1e20_0800 {
            let rm = ((insn >> 16) & 0x1f) as usize;
            let opcode = (insn >> 12) & 0xf;
            let a = self.read_vreg_lo(ir, rn);
            let b = self.read_vreg_lo(ir, rm);
            let helper = match (opcode, is_double) {
                (0, true) => helper_fmul64 as u64,
                (1, true) => helper_fdiv64 as u64,
                (2, true) => helper_fadd64 as u64,
                (3, true) => helper_fsub64 as u64,
                (4, true) => helper_fmax64 as u64,
                (5, true) => helper_fmin64 as u64,
                (6, true) => helper_fmaxnm64 as u64,
                (7, true) => helper_fminnm64 as u64,
                (8, true) => helper_fnmul64 as u64, // FNMUL
                (0, false) => helper_fmul32 as u64,
                (1, false) => helper_fdiv32 as u64,
                (2, false) => helper_fadd32 as u64,
                (3, false) => helper_fsub32 as u64,
                (4, false) => helper_fmax32 as u64,
                (5, false) => helper_fmin32 as u64,
                (6, false) => helper_fmaxnm32 as u64,
                (7, false) => helper_fminnm32 as u64,
                (8, false) => helper_fnmul32 as u64, // FNMUL
                _ => return None,
            };
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper, &[a, b]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }

        // SCVTF/UCVTF GPR→FP: 0 sf 0 11110 ftype 1 00010 000000 Rn Rd
        // ftype=01 (double), ftype=00 (single); sf=1 → Xn, sf=0 → Wn
        // opcode bit[16]=0→SCVTF, bit[16]=1→UCVTF; rmode=00
        // Mask: top byte + ftype + opcode field approach
        // SCVTF Dd, Xn: mask 0xffff_fc00 == 0x9e62_0000
        if insn & 0xffff_fc00 == 0x9e62_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_scvtf_d_x as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // SCVTF Dd, Wn: mask 0xffff_fc00 == 0x1e62_0000
        if insn & 0xffff_fc00 == 0x1e62_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_scvtf_d_w as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // SCVTF Sd, Xn: mask 0xffff_fc00 == 0x9e22_0000
        if insn & 0xffff_fc00 == 0x9e22_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_scvtf_s_x as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // SCVTF Sd, Wn: mask 0xffff_fc00 == 0x1e22_0000
        if insn & 0xffff_fc00 == 0x1e22_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_scvtf_s_w as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // UCVTF Dd, Wn: mask 0xffff_fc00 == 0x1e63_0000
        if insn & 0xffff_fc00 == 0x1e63_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_ucvtf_d_w as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // UCVTF Dd, Xn: mask 0xffff_fc00 == 0x9e63_0000
        if insn & 0xffff_fc00 == 0x9e63_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_ucvtf_d_x as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // UCVTF Sd, Wn: mask 0xffff_fc00 == 0x1e23_0000
        if insn & 0xffff_fc00 == 0x1e23_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_ucvtf_s_w as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // UCVTF Sd, Xn: mask 0xffff_fc00 == 0x9e23_0000
        if insn & 0xffff_fc00 == 0x9e23_0000 {
            let src = self.read_xreg(ir, rn as i64);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_ucvtf_s_x as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // SCVTF/UCVTF fixed-point (scalar):
        //   scvtf d,w,#fbits : 0x1e42_0000 (masked by 0xff7f_0000)
        //   scvtf d,x,#fbits : 0x9e42_0000
        //   scvtf s,w,#fbits : 0x1e02_0000
        //   scvtf s,x,#fbits : 0x9e02_0000
        //   ucvtf d,w,#fbits : 0x1e43_0000
        //   ucvtf d,x,#fbits : 0x9e43_0000
        //   ucvtf s,w,#fbits : 0x1e03_0000
        //   ucvtf s,x,#fbits : 0x9e03_0000
        //
        // scale = bits[15:10], fbits = 64 - scale.
        // Exercised by SPEC2006 gobmk: scvtf d13, w1, #1 (0x1e42fc2d).
        let int_to_fp_fix = insn & 0xff7f_0000;
        if int_to_fp_fix == 0x1e42_0000
            || int_to_fp_fix == 0x9e42_0000
            || int_to_fp_fix == 0x1e02_0000
            || int_to_fp_fix == 0x9e02_0000
            || int_to_fp_fix == 0x1e43_0000
            || int_to_fp_fix == 0x9e43_0000
            || int_to_fp_fix == 0x1e03_0000
            || int_to_fp_fix == 0x9e03_0000
        {
            let scale = ((insn >> 10) & 0x3f) as u64;
            if scale == 0 || scale > 64 {
                return Some(false);
            }
            let fbits = 64 - scale;
            let src = self.read_xreg(ir, rn as i64);
            let fb = ir.new_const(Type::I64, fbits);
            let d = ir.new_temp(Type::I64);
            let helper = match int_to_fp_fix {
                0x1e42_0000 => helper_scvtf_d_w_fixed as u64,
                0x9e42_0000 => helper_scvtf_d_x_fixed as u64,
                0x1e02_0000 => helper_scvtf_s_w_fixed as u64,
                0x9e02_0000 => helper_scvtf_s_x_fixed as u64,
                0x1e43_0000 => helper_ucvtf_d_w_fixed as u64,
                0x9e43_0000 => helper_ucvtf_d_x_fixed as u64,
                0x1e03_0000 => helper_ucvtf_s_w_fixed as u64,
                0x9e03_0000 => helper_ucvtf_s_x_fixed as u64,
                _ => unreachable!(),
            };
            ir.gen_call(d, helper, &[src, fb]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FCVTZS/FCVTZU fixed-point (scalar):
        //   fcvtzs w,s,#fbits : 0x1e18_0000 (masked by 0xff3f_0000)
        //   fcvtzu w,s,#fbits : 0x1e19_0000
        //   fcvtzs x,d,#fbits : 0x9e18_0000
        //   fcvtzu x,d,#fbits : 0x9e19_0000
        //
        // scale = bits[15:10], fbits = 64 - scale.
        // This is exercised by SPEC2006 gobmk (fcvtzs w?, s?, #12).
        let fp_fix = insn & 0xff3f_0000;
        if fp_fix == 0x1e18_0000
            || fp_fix == 0x1e19_0000
            || fp_fix == 0x9e18_0000
            || fp_fix == 0x9e19_0000
        {
            let scale = ((insn >> 10) & 0x3f) as u64;
            if scale == 0 || scale > 64 {
                return Some(false);
            }
            let fbits = 64 - scale;
            let src = self.read_vreg_lo(ir, rn);
            let fb = ir.new_const(Type::I64, fbits);
            let d = ir.new_temp(Type::I64);
            let helper = match fp_fix {
                0x1e18_0000 => helper_fcvtzs_w_s_fixed as u64,
                0x1e19_0000 => helper_fcvtzu_w_s_fixed as u64,
                0x9e18_0000 => helper_fcvtzs_x_d_fixed as u64,
                0x9e19_0000 => helper_fcvtzu_x_d_fixed as u64,
                _ => unreachable!(),
            };
            ir.gen_call(d, helper, &[src, fb]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZx double-precision: ftype=01 (bits[23:22]=01)
        // FCVTZU Xd, Dn: mask 0xffff_fc00 == 0x9e79_0000
        if insn & 0xffff_fc00 == 0x9e79_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzu_x_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZU Wd, Dn: mask 0xffff_fc00 == 0x1e79_0000
        if insn & 0xffff_fc00 == 0x1e79_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzu_w_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZS Wd, Dn: mask 0xffff_fc00 == 0x1e78_0000
        if insn & 0xffff_fc00 == 0x1e78_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzs_w_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZS Xd, Dn: mask 0xffff_fc00 == 0x9e78_0000
        if insn & 0xffff_fc00 == 0x9e78_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzs_x_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZx single-precision: ftype=00 (bits[23:22]=00)
        // FCVTZU Xd, Sn: mask 0xffff_fc00 == 0x9e39_0000
        if insn & 0xffff_fc00 == 0x9e39_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzu_x_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZU Wd, Sn: mask 0xffff_fc00 == 0x1e39_0000
        if insn & 0xffff_fc00 == 0x1e39_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzu_w_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZS Wd, Sn: mask 0xffff_fc00 == 0x1e38_0000
        if insn & 0xffff_fc00 == 0x1e38_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzs_w_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTZS Xd, Sn: mask 0xffff_fc00 == 0x9e38_0000
        if insn & 0xffff_fc00 == 0x9e38_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtzs_x_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTAS Xd, Dn: mask 0xffff_fc00 == 0x9e64_0000
        if insn & 0xffff_fc00 == 0x9e64_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtas_x_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTAS Wd, Dn: mask 0xffff_fc00 == 0x1e64_0000
        if insn & 0xffff_fc00 == 0x1e64_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtas_w_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTAS Xd, Sn: mask 0xffff_fc00 == 0x9e24_0000
        if insn & 0xffff_fc00 == 0x9e24_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtas_x_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTAS Wd, Sn: mask 0xffff_fc00 == 0x1e24_0000
        if insn & 0xffff_fc00 == 0x1e24_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtas_w_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTPS/FCVTPU: round toward +inf FP → int
        // FCVTPS Wd, Dn: 0001 1110 0110 1000 0000 00 Rn Rd (0x1e680000)
        if insn & 0xffff_fc00 == 0x1e68_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtps_w_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTPS Xd, Dn: 1001 1110 0110 1000 0000 00 Rn Rd (0x9e680000)
        if insn & 0xffff_fc00 == 0x9e68_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtps_x_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTPS Wd, Sn: ftype=00 sf=0 (0x1e280000)
        if insn & 0xffff_fc00 == 0x1e28_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtps_w_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTPS Xd, Sn: ftype=00 sf=1 (0x9e280000)
        if insn & 0xffff_fc00 == 0x9e28_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtps_x_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FNMUL Dd,Dn,Dm / FNMUL Sd,Sn,Sm: x001 1110 xx1 Rm 1000 10 Rn Rd
        // mask: 0x5f20_fc00 == 0x1e20_8800
        if insn & 0x5f20_fc00 == 0x1e20_8800 {
            let rm = ((insn >> 16) & 0x1f) as usize;
            let a = self.read_vreg_lo(ir, rn);
            let b = self.read_vreg_lo(ir, rm);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_fnmul64 as u64, &[a, b]);
            } else {
                ir.gen_call(d, helper_fnmul32 as u64, &[a, b]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FMAXNM/FMINNM/FMAX/FMIN scalar: x001 1110 xx1 Rm opcode 10 Rn Rd
        // These share the FP two-source mask but with additional opcodes
        // FMAXNM: opcode=0110, FMINNM: opcode=0111, FMAX: opcode=0100, FMIN: opcode=0101
        if insn & 0x5f20_0c00 == 0x1e20_0800 {
            let rm = ((insn >> 16) & 0x1f) as usize;
            let opcode = (insn >> 12) & 0xf;
            let a = self.read_vreg_lo(ir, rn);
            let b = self.read_vreg_lo(ir, rm);
            let helper = match (opcode, is_double) {
                (0, true) => helper_fmul64 as u64,
                (1, true) => helper_fdiv64 as u64,
                (2, true) => helper_fadd64 as u64,
                (3, true) => helper_fsub64 as u64,
                (0, false) => helper_fmul32 as u64,
                (1, false) => helper_fdiv32 as u64,
                (2, false) => helper_fadd32 as u64,
                (3, false) => helper_fsub32 as u64,
                (4, true) => helper_fmax64 as u64,
                (5, true) => helper_fmin64 as u64,
                (6, true) => helper_fmaxnm64 as u64,
                (7, true) => helper_fminnm64 as u64,
                (4, false) => helper_fmax32 as u64,
                (5, false) => helper_fmin32 as u64,
                (6, false) => helper_fmaxnm32 as u64,
                (7, false) => helper_fminnm32 as u64,
                _ => return None,
            };
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper, &[a, b]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // double: 0x1e664000, single: 0x1e264000
        if insn & 0xffbf_fc00 == 0x1e26_4000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_frinta_d as u64, &[src]);
            } else {
                ir.gen_call(d, helper_frinta_s as u64, &[src]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FRINTM Dd/Sd: round toward minus infinity
        // double: 0x1e654000, single: 0x1e254000
        if insn & 0xffbf_fc00 == 0x1e25_4000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_frintm_d as u64, &[src]);
            } else {
                ir.gen_call(d, helper_frintm_s as u64, &[src]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FRINTP Dd/Sd: round toward plus infinity
        // double: 0x1e64c000, single: 0x1e24c000
        if insn & 0xffbf_fc00 == 0x1e24_c000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_frintp_d as u64, &[src]);
            } else {
                ir.gen_call(d, helper_frintp_s as u64, &[src]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FRINTN Dd/Sd: round to nearest even
        // double: 0x1e644000, single: 0x1e244000
        if insn & 0xffbf_fc00 == 0x1e24_4000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_frintn_d as u64, &[src]);
            } else {
                ir.gen_call(d, helper_frintn_s as u64, &[src]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FRINTZ Dd/Sd: round toward zero
        // double: 0x1e65c000, single: 0x1e25c000
        if insn & 0xffbf_fc00 == 0x1e25_c000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_call(d, helper_frintz_d as u64, &[src]);
            } else {
                ir.gen_call(d, helper_frintz_s as u64, &[src]);
            }
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FMOV Xd, Dn: 1001 1110 0110 0110 0000 00 Rn Rd
        // (already handled above, but catch here too)
        if insn & 0xffff_fc00 == 0x9e66_0000 {
            let val = self.read_vreg_lo(ir, rn);
            self.write_xreg(ir, rd as i64, val);
            return Some(true);
        }
        // FMOV Dn, Xn: 1001 1110 0110 0111 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x9e67_0000 {
            let val = self.read_xreg(ir, rn as i64);
            self.write_vreg_lo(ir, rd, val);
            let zero = ir.new_const(Type::I64, 0);
            self.write_vreg_hi(ir, rd, zero);
            return Some(true);
        }
        // FMOV V.d[1], Xn: 1001 1110 1010 1111 0000 00 Rn Rd (0x9eaf0000)
        if insn & 0xffff_fc00 == 0x9eaf_0000 {
            let val = self.read_xreg(ir, rn as i64);
            self.write_vreg_hi(ir, rd, val);
            return Some(true);
        }
        // FMOV Xd, V.d[1]: 1001 1110 1010 1110 0000 00 Rn Rd (0x9eae0000)
        if insn & 0xffff_fc00 == 0x9eae_0000 {
            let val = self.read_vreg_hi(ir, rn);
            self.write_xreg(ir, rd as i64, val);
            return Some(true);
        }
        // FCVTMS/FCVTMU: round toward -inf
        // FCVTMS Wd, Dn: 0001 1110 0111 0000 0000 00 Rn Rd (ftype=01, sf=0)
        if insn & 0xffff_fc00 == 0x1e70_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtms_w_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTMS Xd, Dn: 1001 1110 0111 0000 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x9e70_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtms_x_d as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTMS Wd, Sn: ftype=00 sf=0: 0001 1110 0011 0000 0000 00 Rn Rd (0x1e300000)
        if insn & 0xffff_fc00 == 0x1e30_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtms_w_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCVTMS Xd, Sn: ftype=00 sf=1: 1001 1110 0011 0000 0000 00 Rn Rd (0x9e300000)
        if insn & 0xffff_fc00 == 0x9e30_0000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvtms_x_s as u64, &[src]);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }
        // FCCMP/FCCMPE: x001 1110 xx1 Rm cond 01 Rn nzcv
        // mask: 0x5f200c00 == 0x1e200400
        if insn & 0x5f20_0c00 == 0x1e20_0400 {
            let rm = ((insn >> 16) & 0x1f) as usize;
            let cond = ((insn >> 12) & 0xf) as i64;
            let nzcv_imm = (insn & 0xf) as u64;
            let a = self.read_vreg_lo(ir, rn);
            let b = self.read_vreg_lo(ir, rm);
            let cond_val = self.eval_cond(ir, cond);
            let zero_const = ir.new_const(Type::I64, 0);
            let cmp_result = ir.new_temp(Type::I64);
            let fp_helper = if is_double {
                helper_fcmp64 as u64
            } else {
                helper_fcmp32 as u64
            };
            ir.gen_call(cmp_result, fp_helper, &[a, b]);
            let nzcv_alt = ir.new_const(Type::I64, nzcv_imm << 28);
            let result = ir.new_temp(Type::I64);
            ir.gen_movcond(
                Type::I64,
                result,
                cond_val,
                zero_const,
                cmp_result,
                nzcv_alt,
                Cond::Ne,
            );
            self.set_nzcv_eager(ir, result);
            return Some(true);
        }
        // FMOV Sn, Wn: 0001 1110 0010 0111 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x1e27_0000 {
            let val = self.read_xreg(ir, rn as i64);
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            let masked = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, masked, val, mask);
            self.write_vreg_lo(ir, rd, masked);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FMOV Wn, Sn: 0001 1110 0010 0110 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x1e26_0000 {
            let val = self.read_vreg_lo(ir, rn);
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            let d = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, d, val, mask);
            self.write_xreg(ir, rd as i64, d);
            return Some(true);
        }

        // FMOV Sd,Sn / FMOV Dd,Dn: 000 1111 0 T 10 0000 0100 00 Rn Rd
        if insn & 0xffbf_fc00 == 0x1e20_4000 {
            let src = self.read_vreg_lo(ir, rn);
            self.write_vreg_lo(ir, rd, src);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }

        // FABS Dd, Dn: 0001 1110 0110 0000 1100 00 Rn Rd
        if insn & 0xffbf_fc00 == 0x1e20_c000 {
            let src = self.read_vreg_lo(ir, rn);
            let mask = ir.new_const(Type::I64, 0x7fff_ffff_ffff_ffff);
            let d = ir.new_temp(Type::I64);
            if is_double {
                ir.gen_and(Type::I64, d, src, mask);
            } else {
                let mask32 = ir.new_const(Type::I64, 0x7fff_ffff);
                ir.gen_and(Type::I64, d, src, mask32);
            }
            self.write_vreg_lo(ir, rd, d);
            let zero = ir.new_const(Type::I64, 0);
            self.write_vreg_hi(ir, rd, zero);
            return Some(true);
        }

        // FNEG Dd,Dn / FNEG Sd,Sn: 000 1111 0 T 10 0001 0100 00 Rn Rd
        if insn & 0xffbf_fc00 == 0x1e21_4000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            if is_double {
                let sign = ir.new_const(Type::I64, 1u64 << 63);
                ir.gen_xor(Type::I64, d, src, sign);
            } else {
                let sign = ir.new_const(Type::I64, 1u64 << 31);
                ir.gen_xor(Type::I64, d, src, sign);
            }
            self.write_vreg_lo(ir, rd, d);
            let zero = ir.new_const(Type::I64, 0);
            self.write_vreg_hi(ir, rd, zero);
            return Some(true);
        }

        // FCSEL Dd,Dn,Dm,cond: x001 1110 T10 Rm cond 11 Rn Rd
        if insn & 0x5f20_0c00 == 0x1e20_0c00 {
            let rm = ((insn >> 16) & 0x1f) as usize;
            let cond = (insn >> 12) & 0xf;
            let a = self.read_vreg_lo(ir, rn);
            let b = self.read_vreg_lo(ir, rm);
            let cond_val = self.eval_cond(ir, cond as i64);
            let zero = ir.new_const(Type::I64, 0);
            let d = ir.new_temp(Type::I64);
            ir.gen_movcond(Type::I64, d, cond_val, zero, a, b, Cond::Ne);
            self.write_vreg_lo(ir, rd, d);
            let zero2 = ir.new_const(Type::I64, 0);
            self.write_vreg_hi(ir, rd, zero2);
            return Some(true);
        }

        // FCVT Dd, Sn: 0001 1110 0010 0010 1100 00 Rn Rd (single→double)
        if insn & 0xffff_fc00 == 0x1e22_c000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvt_s_to_d as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }
        // FCVT Sn, Dd: 0001 1110 0110 0010 0100 00 Rn Rd (double→single)
        if insn & 0xffff_fc00 == 0x1e62_4000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_fcvt_d_to_s as u64, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }

        // FSQRT Dd,Dn / FSQRT Sd,Sn: 000 1111 0 T 10 0001 1100 00 Rn Rd
        if insn & 0xffbf_fc00 == 0x1e21_c000 {
            let src = self.read_vreg_lo(ir, rn);
            let d = ir.new_temp(Type::I64);
            let helper = if is_double {
                helper_fsqrt64 as u64
            } else {
                helper_fsqrt32 as u64
            };
            ir.gen_call(d, helper, &[src]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return Some(true);
        }

        None
    }
    fn try_neon_3same_misc(&mut self, ir: &mut Context, insn: u32) -> bool {
        // AdvSIMD three same: 0 Q U 01110 size 1 Rm opcode 1 Rn Rd
        if insn & 0x9f20_0400 == 0x0e20_0400 {
            let size = (insn >> 22) & 0x3;
            let opcode = (insn >> 11) & 0x1f;
            // FP 3-same: opcode>=0b11000, sz = bit[22] (0=f32, 1=f64)
            // Integer 3-same uses opcodes in 0b00000..0b10111 range
            if opcode >= 0b11000 {
                let sz = size & 1; // bit[22]: 0=f32, 1=f64
                return self.neon_fp_3same(ir, insn, sz);
            }
            return self.neon_3same(ir, insn);
        }
        // AdvSIMD two-reg misc: 0 Q U 01110 size 10000 opcode 10 Rn Rd
        if insn & 0x9f3e_0c00 == 0x0e20_0800 {
            return self.neon_2reg_misc(ir, insn);
        }
        // AdvSIMD shift by immediate: 0 Q U 011110 immh immb opcode 1 Rn Rd
        if insn & 0x9f80_0400 == 0x0f00_0400 {
            return self.neon_shift_imm(ir, insn);
        }
        // AdvSIMD across lanes: 0 Q U 01110 size 11000 opcode 10 Rn Rd
        if insn & 0x9f3e_0c00 == 0x0e30_0800 {
            return self.neon_across_lanes(ir, insn);
        }
        // AdvSIMD permute (UZP1/UZP2/ZIP1/ZIP2/TRN1/TRN2):
        // 0 Q 00 1110 size 0 Rm 0 opcode 10 Rn Rd
        if insn & 0xbf20_8c00 == 0x0e00_0800 {
            return self.neon_permute(ir, insn);
        }
        // AdvSIMD three different: 0 Q U 01110 size 1 Rm opcode 00 Rn Rd
        if insn & 0x9f20_0c00 == 0x0e20_0000 {
            return self.neon_3diff(ir, insn);
        }
        false
    }

    /// Scalar AdvSIMD shift-by-immediate: 01 U 111110 immh immb opcode 1 Rn Rd
    fn neon_scalar_shift_imm(
        &mut self,
        ir: &mut Context,
        insn: u32,
    ) -> bool {
        let u = (insn >> 29) & 1;
        let immh = (insn >> 19) & 0xf;
        let immb = (insn >> 16) & 0x7;
        let opcode = (insn >> 11) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // SHL Dd, Dn, #imm : U=0 opcode=01010 and 64-bit element class (immh=1xxx)
        if u == 0 && opcode == 0b01010 && (immh & 0b1000) != 0 {
            let immhb = (immh << 3) | immb;
            if immhb < 64 {
                return false;
            }
            let shift = (immhb - 64) as u64;
            let src = self.read_vreg_lo(ir, rn);
            let sh = ir.new_const(Type::I64, shift);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_shl64 as u64, &[src, sh]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        // USHR Dd, Dn, #imm : U=1 opcode=00000 and 64-bit element class (immh=1xxx)
        if u == 1 && opcode == 0b00000 && (immh & 0b1000) != 0 {
            let immhb = (immh << 3) | immb;
            if !(64..128).contains(&immhb) {
                return false;
            }
            let shift = (128 - immhb) as u64;
            let src = self.read_vreg_lo(ir, rn);
            let sh = ir.new_const(Type::I64, shift);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_ushr64 as u64, &[src, sh]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        false
    }

    /// AdvSIMD three different (widening): 0 Q U 01110 size 1 Rm opcode 00 Rn Rd
    fn neon_3diff(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 3;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let opcode = (insn >> 12) & 0xf;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        match (u, size, opcode) {
            // SMULL/SMULL2 .2d, .2s/.4s: U=0 size=10 opcode=1100
            (0, 0b10, 0b1100) => {
                // Q=0: use low halves of Rn,Rm; Q=1: use high halves
                let n = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let m = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_smull32_lo as u64, &[n, m]);
                ir.gen_call(d_hi, helper_smull32_hi as u64, &[n, m]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // SADDW/SADDW2: U=0 size=01 opcode=0001 — .4S += sign-extend(.4H)
            (0, 0b01, 0b0001) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let n_lo = self.read_vreg_lo(ir, rn);
                let n_hi = self.read_vreg_hi(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_saddw16_lo as u64, &[n_lo, src]);
                ir.gen_call(d_hi, helper_saddw16_hi as u64, &[n_hi, src]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // UADDW/UADDW2: U=1 size=01 opcode=0001 — .4S += zero-extend(.4H)
            (1, 0b01, 0b0001) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let n_lo = self.read_vreg_lo(ir, rn);
                let n_hi = self.read_vreg_hi(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uaddw16_lo as u64, &[n_lo, src]);
                ir.gen_call(d_hi, helper_uaddw16_hi as u64, &[n_hi, src]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // SSUBW/SSUBW2: U=0 size=01 opcode=0011 — .4S -= sign-extend(.4H)
            (0, 0b01, 0b0011) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let n_lo = self.read_vreg_lo(ir, rn);
                let n_hi = self.read_vreg_hi(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_ssubw16_lo as u64, &[n_lo, src]);
                ir.gen_call(d_hi, helper_ssubw16_hi as u64, &[n_hi, src]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // SADDW/SADDW2 .2D += sign-extend(.2S): size=10
            (0, 0b10, 0b0001) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let n_lo = self.read_vreg_lo(ir, rn);
                let n_hi = self.read_vreg_hi(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_saddw32_lo as u64, &[n_lo, src]);
                ir.gen_call(d_hi, helper_saddw32_hi as u64, &[n_hi, src]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // USUBL/USUBL2: U=1 size=01 opcode=0010 — Vd.4S = zext(Vn.4H) - zext(Vm.4H)
            (1, 0b01, 0b0010) => {
                let n = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let m = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_usubl16_lo as u64, &[n, m]);
                ir.gen_call(d_hi, helper_usubl16_hi as u64, &[n, m]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // SMLAL/SMLAL2 .2D, .2S, .2S: U=0 size=10 opcode=1000
            (0, 0b10, 0b1000) => {
                let n = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let m = if q == 0 {
                    self.read_vreg_lo(ir, rm)
                } else {
                    self.read_vreg_hi(ir, rm)
                };
                let acc_lo = self.read_vreg_lo(ir, rd);
                let acc_hi = self.read_vreg_hi(ir, rd);
                let d_lo = ir.new_temp(Type::I64);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_smlal32_lo as u64, &[acc_lo, n, m]);
                ir.gen_call(d_hi, helper_smlal32_hi as u64, &[acc_hi, n, m]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            _ => false,
        }
    }

    fn neon_indexed_element(
        &mut self,
        ir: &mut Context,
        insn: u32,
    ) -> Option<bool> {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 3;
        let l = (insn >> 21) & 1;
        let m = (insn >> 20) & 1;
        let rm = ((insn >> 16) & 0xf) as usize;
        let opcode = (insn >> 12) & 0xf;
        let h = (insn >> 11) & 1;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // MUL .4S/.2S by element: U=0 size=10 opcode=1000
        if u == 0 && size == 0b10 && opcode == 0b1000 {
            // 32-bit element: index = H:L, Rm = M:Rm
            let idx = ((h << 1) | l) as usize;
            let vrm = (m << 4) as usize | rm;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, vrm)
            } else {
                self.read_vreg_hi(ir, vrm)
            };
            let bit_off = (idx % 2) * 32;
            let elem = ir.new_temp(Type::I64);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shr(Type::I64, elem, half, sh);
            } else {
                ir.gen_mov(Type::I64, elem, half);
            }
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            // Broadcast scalar into both 32-bit lanes
            let sh32 = ir.new_const(Type::I64, 32);
            let scalar = ir.new_temp(Type::I64);
            let hi_part = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi_part, elem, sh32);
            ir.gen_or(Type::I64, scalar, elem, hi_part);

            let n_lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_mul32_elem as u64, &[n_lo, scalar]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_mul32_elem as u64, &[n_hi, scalar]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return Some(true);
        }
        // MLA .4S/.2S by element: U=1 size=10 opcode=0000
        if u == 1 && size == 0b10 && opcode == 0b0000 {
            let idx = ((h << 1) | l) as usize;
            let vrm = (m << 4) as usize | rm;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, vrm)
            } else {
                self.read_vreg_hi(ir, vrm)
            };
            let bit_off = (idx % 2) * 32;
            let elem = ir.new_temp(Type::I64);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shr(Type::I64, elem, half, sh);
            } else {
                ir.gen_mov(Type::I64, elem, half);
            }
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            let sh32 = ir.new_const(Type::I64, 32);
            let scalar = ir.new_temp(Type::I64);
            let hi_part = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi_part, elem, sh32);
            ir.gen_or(Type::I64, scalar, elem, hi_part);

            let d_lo = self.read_vreg_lo(ir, rd);
            let n_lo = self.read_vreg_lo(ir, rn);
            let r_lo = ir.new_temp(Type::I64);
            ir.gen_call(r_lo, helper_mla32_elem as u64, &[d_lo, n_lo, scalar]);
            self.write_vreg_lo(ir, rd, r_lo);
            if q != 0 {
                let d_hi = self.read_vreg_hi(ir, rd);
                let n_hi = self.read_vreg_hi(ir, rn);
                let r_hi = ir.new_temp(Type::I64);
                ir.gen_call(
                    r_hi,
                    helper_mla32_elem as u64,
                    &[d_hi, n_hi, scalar],
                );
                self.write_vreg_hi(ir, rd, r_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return Some(true);
        }
        // FMLA .4S/.2S by element: U=0 size=10 opcode=0001
        if u == 0 && size == 0b10 && opcode == 0b0001 {
            // 32-bit float element: index = H:L, Rm = M:Rm (4-bit)
            let idx = ((h << 1) | l) as usize;
            let vrm = (m << 4) as usize | rm;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, vrm)
            } else {
                self.read_vreg_hi(ir, vrm)
            };
            let bit_off = (idx % 2) * 32;
            let elem = ir.new_temp(Type::I64);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shr(Type::I64, elem, half, sh);
            } else {
                ir.gen_mov(Type::I64, elem, half);
            }
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            // Broadcast scalar into both 32-bit lanes
            let sh32 = ir.new_const(Type::I64, 32);
            let scalar = ir.new_temp(Type::I64);
            let hi_part = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi_part, elem, sh32);
            ir.gen_or(Type::I64, scalar, elem, hi_part);

            let d_lo = self.read_vreg_lo(ir, rd);
            let n_lo = self.read_vreg_lo(ir, rn);
            let r_lo = ir.new_temp(Type::I64);
            ir.gen_call(r_lo, helper_vfmla32 as u64, &[d_lo, n_lo, scalar]);
            self.write_vreg_lo(ir, rd, r_lo);
            if q != 0 {
                let d_hi = self.read_vreg_hi(ir, rd);
                let n_hi = self.read_vreg_hi(ir, rn);
                let r_hi = ir.new_temp(Type::I64);
                ir.gen_call(r_hi, helper_vfmla32 as u64, &[d_hi, n_hi, scalar]);
                self.write_vreg_hi(ir, rd, r_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return Some(true);
        }
        // FMUL .4S/.2S by element: U=0 size=10 opcode=1001
        if u == 0 && size == 0b10 && opcode == 0b1001 {
            let idx = ((h << 1) | l) as usize;
            let vrm = (m << 4) as usize | rm;
            let half = if idx < 2 {
                self.read_vreg_lo(ir, vrm)
            } else {
                self.read_vreg_hi(ir, vrm)
            };
            let bit_off = (idx % 2) * 32;
            let elem = ir.new_temp(Type::I64);
            if bit_off > 0 {
                let sh = ir.new_const(Type::I64, bit_off as u64);
                ir.gen_shr(Type::I64, elem, half, sh);
            } else {
                ir.gen_mov(Type::I64, elem, half);
            }
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, elem, elem, mask);
            // Broadcast scalar into both 32-bit lanes.
            let sh32 = ir.new_const(Type::I64, 32);
            let scalar = ir.new_temp(Type::I64);
            let hi_part = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi_part, elem, sh32);
            ir.gen_or(Type::I64, scalar, elem, hi_part);

            let n_lo = self.read_vreg_lo(ir, rn);
            let r_lo = ir.new_temp(Type::I64);
            ir.gen_call(r_lo, helper_vfmul32 as u64, &[n_lo, scalar]);
            self.write_vreg_lo(ir, rd, r_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let r_hi = ir.new_temp(Type::I64);
                ir.gen_call(r_hi, helper_vfmul32 as u64, &[n_hi, scalar]);
                self.write_vreg_hi(ir, rd, r_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return Some(true);
        }
        None
    }

    /// AdvSIMD permute: UZP1/UZP2/ZIP1/ZIP2/TRN1/TRN2
    fn neon_permute(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let size = (insn >> 22) & 3;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let opcode = (insn >> 12) & 7;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // UZP1: opcode=001
        if opcode == 1 && size == 0 {
            // UZP1 .16B: gather even-indexed bytes within each source register
            // Q=0 (.8B): d_lo = uzp1(n_lo, m_lo)
            // Q=1 (.16B): d_lo = uzp1(n_lo, n_hi), d_hi = uzp1(m_lo, m_hi)
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_8 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_8 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp1_8 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        if opcode == 1 && size == 1 {
            // UZP1 .8H/.4H: gather even halfwords within each source
            // Q=0 (.4H): d_lo = uzp1(n_lo, m_lo)
            // Q=1 (.8H): d_lo = uzp1(n_lo, n_hi), d_hi = uzp1(m_lo, m_hi)
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_16 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_16 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp1_16 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        // ZIP1/ZIP2: opcode=011/111, size=10 (32-bit)
        // .2S (Q=0): result_lo = [Vn[0], Vm[0]]
        // .4S (Q=1): result_lo = [Vn[0], Vm[0]], result_hi = [Vn[1], Vm[1]]
        if opcode == 3 && size == 2 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            // lo half: Vn[0] in low 32, Vm[0] in high 32
            let mask32 = ir.new_const(Type::I64, 0xffff_ffff);
            let n0 = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, n0, n_lo, mask32);
            let m0 = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, m0, m_lo, mask32);
            let c32 = ir.new_const(Type::I64, 32);
            let m0_hi = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, m0_hi, m0, c32);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, d_lo, n0, m0_hi);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                // hi half: Vn[1] in low 32, Vm[1] in high 32
                let c32b = ir.new_const(Type::I64, 32);
                let n1 = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, n1, n_lo, c32b);
                let m1 = ir.new_temp(Type::I64);
                let c32c = ir.new_const(Type::I64, 32);
                ir.gen_shr(Type::I64, m1, m_lo, c32c);
                let m1_hi = ir.new_temp(Type::I64);
                let c32d = ir.new_const(Type::I64, 32);
                ir.gen_shl(Type::I64, m1_hi, m1, c32d);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_or(Type::I64, d_hi, n1, m1_hi);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }
        // ZIP2 .2S/.4S: opcode=111, size=10
        if opcode == 7 && size == 2 {
            if q == 0 {
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                let c32 = ir.new_const(Type::I64, 32);
                let n1 = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, n1, n_lo, c32);
                let m1 = ir.new_temp(Type::I64);
                let c32b = ir.new_const(Type::I64, 32);
                ir.gen_shr(Type::I64, m1, m_lo, c32b);
                let c32c = ir.new_const(Type::I64, 32);
                let m1_hi = ir.new_temp(Type::I64);
                ir.gen_shl(Type::I64, m1_hi, m1, c32c);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_or(Type::I64, d_lo, n1, m1_hi);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let mask32 = ir.new_const(Type::I64, 0xffff_ffff);
                let n2 = ir.new_temp(Type::I64);
                ir.gen_and(Type::I64, n2, n_hi, mask32);
                let m2 = ir.new_temp(Type::I64);
                ir.gen_and(Type::I64, m2, m_hi, mask32);
                let c32 = ir.new_const(Type::I64, 32);
                let m2_hi = ir.new_temp(Type::I64);
                ir.gen_shl(Type::I64, m2_hi, m2, c32);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_or(Type::I64, d_lo, n2, m2_hi);
                self.write_vreg_lo(ir, rd, d_lo);
                let c32b = ir.new_const(Type::I64, 32);
                let n3 = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, n3, n_hi, c32b);
                let m3 = ir.new_temp(Type::I64);
                let c32c = ir.new_const(Type::I64, 32);
                ir.gen_shr(Type::I64, m3, m_hi, c32c);
                let c32d = ir.new_const(Type::I64, 32);
                let m3_hi = ir.new_temp(Type::I64);
                ir.gen_shl(Type::I64, m3_hi, m3, c32d);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_or(Type::I64, d_hi, n3, m3_hi);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        // UZP2 .8B/.16B: opcode=101, size=00
        // Q=0: d_lo = uzp2(n_lo, m_lo)
        // Q=1: d_lo = uzp2(n_lo, n_hi), d_hi = uzp2(m_lo, m_hi)
        if opcode == 5 && size == 0 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_8 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_8 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp2_8 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        // UZP2 .4H/.8H: opcode=101, size=01
        if opcode == 5 && size == 1 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_16 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_16 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp2_16 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        // UZP2 .4S/.2S: opcode=101, size=10 — take odd-indexed 32-bit elements
        if opcode == 5 && size == 2 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_32 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp2_32 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp2_32 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        // ZIP1 .8B/.16B: opcode=011, size=00
        // Q=0: d_lo = zip1(n_lo, m_lo) — interleave low 4 bytes of Vn and Vm
        // Q=1: d_lo = zip1(n_lo, m_lo), d_hi = zip1(n_hi, m_hi) — pairs from matching halves
        if opcode == 3 && size == 0 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_zip1_8 as u64, &[n_lo, m_lo]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_zip1_8 as u64, &[n_hi, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }
        // ZIP2 .8B/.16B: opcode=111, size=00
        if opcode == 7 && size == 0 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_zip2_8 as u64, &[n_lo, m_lo]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_zip2_8 as u64, &[n_hi, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }
        // ZIP1 .4H/.8H: opcode=011, size=01
        if opcode == 3 && size == 1 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_zip1_16 as u64, &[n_lo, m_lo]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_zip1_16 as u64, &[n_hi, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }
        // ZIP2 .4H/.8H: opcode=111, size=01
        if opcode == 7 && size == 1 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_zip2_16 as u64, &[n_lo, m_lo]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_zip2_16 as u64, &[n_hi, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }
        // UZP1 .4S/.2S: opcode=001, size=10 — take even-indexed 32-bit elements
        // .2S (Q=0): Rd = [Rn[0], Rm[0]]
        // .4S (Q=1): Rd = [Rn[0], Rn[2], Rm[0], Rm[2]]
        if opcode == 1 && size == 2 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            if q == 0 {
                // .2S: take low 32 of each source
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_32 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.clear_vreg_hi(ir, rd);
            } else {
                // .4S: lo = [Rn[0], Rn[2]], hi = [Rm[0], Rm[2]]
                let n_hi = self.read_vreg_hi(ir, rn);
                let m_hi = self.read_vreg_hi(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_uzp1_32 as u64, &[n_lo, n_hi]);
                self.write_vreg_lo(ir, rd, d_lo);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_uzp1_32 as u64, &[m_lo, m_hi]);
                self.write_vreg_hi(ir, rd, d_hi);
            }
            return true;
        }
        false
    }
}

// ── VFP immediate expansion ─────────────────────────────

fn vfp_expand_imm64(imm8: u64) -> u64 {
    let a = (imm8 >> 7) & 1;
    let b = (imm8 >> 6) & 1;
    let b_rep = if b != 0 { 0xffu64 } else { 0 };
    let payload = imm8 & 0x3f;
    (a << 63) | ((b ^ 1) << 62) | ((b_rep & 0xff) << 54) | (payload << 48)
}

fn vfp_expand_imm32(imm8: u64) -> u32 {
    let a = ((imm8 >> 7) & 1) as u32;
    let b = ((imm8 >> 6) & 1) as u32;
    let b_rep = if b != 0 { 0x1fu32 } else { 0 };
    let payload = (imm8 & 0x3f) as u32;
    (a << 31) | ((b ^ 1) << 30) | (b_rep << 25) | (payload << 19)
}

// ── Division helper functions ────────────────────────────

unsafe extern "C" fn helper_udiv64(n: u64, m: u64) -> u64 {
    if m == 0 {
        0
    } else {
        n / m
    }
}

unsafe extern "C" fn helper_udiv32(n: u64, m: u64) -> u64 {
    let n = n as u32;
    let m = m as u32;
    if m == 0 {
        0
    } else {
        (n / m) as u64
    }
}

unsafe extern "C" fn helper_sdiv64(n: u64, m: u64) -> u64 {
    let n = n as i64;
    let m = m as i64;
    if m == 0 {
        0
    } else {
        (n / m) as u64
    }
}

unsafe extern "C" fn helper_sdiv32(n: u64, m: u64) -> u64 {
    let n = n as u32 as i32;
    let m = m as u32 as i32;
    if m == 0 {
        0
    } else {
        (n / m) as i64 as u64
    }
}

unsafe extern "C" fn helper_rbit64(a: u64) -> u64 {
    a.reverse_bits()
}
unsafe extern "C" fn helper_rbit32(a: u64) -> u64 {
    (a as u32).reverse_bits() as u64
}
unsafe extern "C" fn helper_rev16_64(a: u64) -> u64 {
    ((a & 0x00ff_00ff_00ff_00ff) << 8) | ((a & 0xff00_ff00_ff00_ff00) >> 8)
}
unsafe extern "C" fn helper_rev16_32(a: u64) -> u64 {
    let x = a as u32;
    let y = ((x & 0x00ff_00ff) << 8) | ((x & 0xff00_ff00) >> 8);
    y as u64
}
unsafe extern "C" fn helper_rev32_64(a: u64) -> u64 {
    let lo = (a as u32).swap_bytes() as u64;
    let hi = ((a >> 32) as u32).swap_bytes() as u64;
    lo | (hi << 32)
}

// ── NEON helper functions (called via gen_call) ─────────

/// Byte-wise compare equal: each byte → 0xFF if equal, 0x00 otherwise.
unsafe extern "C" fn helper_cmeq8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        if ab == bb {
            r |= 0xffu64 << (i * 8);
        }
    }
    r
}

/// Halfword-wise compare equal: each 16-bit lane -> 0xFFFF if equal.
unsafe extern "C" fn helper_cmeq16(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let ab = (a >> (i * 16)) & 0xffff;
        let bb = (b >> (i * 16)) & 0xffff;
        if ab == bb {
            r |= 0xffffu64 << (i * 16);
        }
    }
    r
}

/// CMTST .16B/.8B: each byte → 0xFF if (a & b) != 0, else 0x00.
unsafe extern "C" fn helper_cmtst8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        if ab & bb != 0 {
            r |= 0xffu64 << (i * 8);
        }
    }
    r
}

/// CMTST .8H/.4H: each 16-bit lane -> 0xFFFF if (a & b) != 0.
unsafe extern "C" fn helper_cmtst16(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let ab = (a >> (i * 16)) & 0xffff;
        let bb = (b >> (i * 16)) & 0xffff;
        if (ab & bb) != 0 {
            r |= 0xffffu64 << (i * 16);
        }
    }
    r
}

/// Byte-wise unsigned compare higher or same.
unsafe extern "C" fn helper_cmhs8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        if ab >= bb {
            r |= 0xffu64 << (i * 8);
        }
    }
    r
}

/// Byte-wise BIT: Vd = (Vd & ~Vm) | (Vn & Vm).
unsafe extern "C" fn helper_bit(vd: u64, vn: u64, vm: u64) -> u64 {
    (vd & !vm) | (vn & vm)
}

/// Byte-wise unsigned max pairwise (8B→8B, adjacent pairs).
unsafe extern "C" fn helper_umaxp8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let a0 = (a >> (i * 16)) & 0xff;
        let a1 = (a >> (i * 16 + 8)) & 0xff;
        r |= a0.max(a1) << (i * 8);
    }
    for i in 0..4 {
        let b0 = (b >> (i * 16)) & 0xff;
        let b1 = (b >> (i * 16 + 8)) & 0xff;
        r |= b0.max(b1) << ((4 + i) * 8);
    }
    r
}

/// Byte-wise unsigned min pairwise.
unsafe extern "C" fn helper_uminp8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let a0 = (a >> (i * 16)) & 0xff;
        let a1 = (a >> (i * 16 + 8)) & 0xff;
        r |= a0.min(a1) << (i * 8);
    }
    for i in 0..4 {
        let b0 = (b >> (i * 16)) & 0xff;
        let b1 = (b >> (i * 16 + 8)) & 0xff;
        r |= b0.min(b1) << ((4 + i) * 8);
    }
    r
}

/// Narrowing shift right (16→8 bit elements, 4 elements per u64).
unsafe extern "C" fn helper_shrn8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let elem = (a >> (i * 16)) & 0xffff;
        let shifted = (elem >> shift) & 0xff;
        r |= shifted << (i * 8);
    }
    r
}

/// Byte-wise EXT: concatenate and extract.
unsafe extern "C" fn helper_ext8(a: u64, b: u64, pos: u64) -> u64 {
    // Concatenate b:a (128-bit), extract 8 bytes starting at byte `pos`
    // For the lo half: pos is 0..7 from the 16-byte concatenation
    let mut r = 0u64;
    for i in 0..8 {
        let idx = pos + i;
        let byte = if idx < 8 {
            (a >> (idx * 8)) & 0xff
        } else {
            (b >> ((idx - 8) * 8)) & 0xff
        };
        r |= byte << (i * 8);
    }
    r
}

/// Byte-wise population count.
unsafe extern "C" fn helper_cnt8(a: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let byte = ((a >> (i * 8)) & 0xff) as u8;
        r |= (byte.count_ones() as u64) << (i * 8);
    }
    r
}

/// Byte-wise add pairwise.
unsafe extern "C" fn helper_addp8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let a0 = (a >> (i * 16)) & 0xff;
        let a1 = (a >> (i * 16 + 8)) & 0xff;
        r |= ((a0 + a1) & 0xff) << (i * 8);
    }
    for i in 0..4 {
        let b0 = (b >> (i * 16)) & 0xff;
        let b1 = (b >> (i * 16 + 8)) & 0xff;
        r |= ((b0 + b1) & 0xff) << ((4 + i) * 8);
    }
    r
}

/// ADDP .4S/.2S: pairwise add 32-bit elements.
unsafe extern "C" fn helper_addp32(a: u64, b: u64) -> u64 {
    let a0 = (a as u32).wrapping_add((a >> 32) as u32) as u64;
    let b0 = (b as u32).wrapping_add((b >> 32) as u32) as u64;
    a0 | (b0 << 32)
}

/// CMTST .4S/.2S: if (a & b) != 0 then 0xFFFFFFFF else 0 per 32-bit element.
unsafe extern "C" fn helper_cmtst32(a: u64, b: u64) -> u64 {
    let lo_a = a as u32;
    let lo_b = b as u32;
    let hi_a = (a >> 32) as u32;
    let hi_b = (b >> 32) as u32;
    let r_lo = if lo_a & lo_b != 0 { !0u32 } else { 0 };
    let r_hi = if hi_a & hi_b != 0 { !0u32 } else { 0 };
    r_lo as u64 | ((r_hi as u64) << 32)
}

/// Byte-wise add.
unsafe extern "C" fn helper_add8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        r |= ((ab + bb) & 0xff) << (i * 8);
    }
    r
}

/// Byte-wise subtract.
unsafe extern "C" fn helper_sub8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        r |= ((ab.wrapping_sub(bb)) & 0xff) << (i * 8);
    }
    r
}

unsafe extern "C" fn helper_add32(a: u64, b: u64) -> u64 {
    let lo = (a as u32).wrapping_add(b as u32) as u64;
    let hi = ((a >> 32) as u32).wrapping_add((b >> 32) as u32) as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_sub32(a: u64, b: u64) -> u64 {
    let lo = (a as u32).wrapping_sub(b as u32) as u64;
    let hi = ((a >> 32) as u32).wrapping_sub((b >> 32) as u32) as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_mul32(a: u64, b: u64) -> u64 {
    let lo = (a as u32).wrapping_mul(b as u32) as u64;
    let hi = ((a >> 32) as u32).wrapping_mul((b >> 32) as u32) as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_abs32(a: u64) -> u64 {
    let lo = (a as i32).wrapping_abs() as u32 as u64;
    let hi = ((a >> 32) as i32).wrapping_abs() as u32 as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_smax32(a: u64, b: u64) -> u64 {
    let lo = std::cmp::max(a as i32, b as i32) as u32 as u64;
    let hi = std::cmp::max((a >> 32) as i32, (b >> 32) as i32) as u32 as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_smin32(a: u64, b: u64) -> u64 {
    let lo = std::cmp::min(a as i32, b as i32) as u32 as u64;
    let hi = std::cmp::min((a >> 32) as i32, (b >> 32) as i32) as u32 as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_umax32(a: u64, b: u64) -> u64 {
    let lo = std::cmp::max(a as u32, b as u32) as u64;
    let hi = std::cmp::max((a >> 32) as u32, (b >> 32) as u32) as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_umin32(a: u64, b: u64) -> u64 {
    let lo = std::cmp::min(a as u32, b as u32) as u64;
    let hi = std::cmp::min((a >> 32) as u32, (b >> 32) as u32) as u64;
    lo | (hi << 32)
}
/// Reduce 2x32-bit to single signed max
unsafe extern "C" fn helper_smaxv32_reduce(a: u64) -> u64 {
    std::cmp::max(a as i32, (a >> 32) as i32) as u32 as u64
}
/// Reduce 2x32-bit to single signed min
unsafe extern "C" fn helper_sminv32_reduce(a: u64) -> u64 {
    std::cmp::min(a as i32, (a >> 32) as i32) as u32 as u64
}
/// Reduce 2x32-bit to single unsigned max
unsafe extern "C" fn helper_umaxv32_reduce(a: u64) -> u64 {
    std::cmp::max(a as u32, (a >> 32) as u32) as u64
}
/// Reduce 2x32-bit to single unsigned min
unsafe extern "C" fn helper_uminv32_reduce(a: u64) -> u64 {
    std::cmp::min(a as u32, (a >> 32) as u32) as u64
}
/// SMAXV 16-bit: max across 4x16-bit elements in two 64-bit halves
unsafe extern "C" fn helper_smaxv16_pair(lo: u64, hi: u64) -> u64 {
    let mut m = i16::MIN;
    for i in 0..4 {
        let e = ((lo >> (i * 16)) & 0xffff) as i16;
        if e > m {
            m = e;
        }
    }
    for i in 0..4 {
        let e = ((hi >> (i * 16)) & 0xffff) as i16;
        if e > m {
            m = e;
        }
    }
    m as u16 as u64
}
/// SMINV 16-bit: min across 4x16-bit elements in two 64-bit halves
unsafe extern "C" fn helper_sminv16_pair(lo: u64, hi: u64) -> u64 {
    let mut m = i16::MAX;
    for i in 0..4 {
        let e = ((lo >> (i * 16)) & 0xffff) as i16;
        if e < m {
            m = e;
        }
    }
    for i in 0..4 {
        let e = ((hi >> (i * 16)) & 0xffff) as i16;
        if e < m {
            m = e;
        }
    }
    m as u16 as u64
}
unsafe extern "C" fn helper_cmeq32(a: u64, b: u64) -> u64 {
    let lo: u32 = if a as u32 == b as u32 { !0 } else { 0 };
    let hi: u32 = if (a >> 32) as u32 == (b >> 32) as u32 {
        !0
    } else {
        0
    };
    lo as u64 | ((hi as u64) << 32)
}
/// CMHS 32-bit: unsigned higher or same
unsafe extern "C" fn helper_cmhs32(a: u64, b: u64) -> u64 {
    let lo: u32 = if a as u32 >= b as u32 { !0 } else { 0 };
    let hi: u32 = if (a >> 32) as u32 >= (b >> 32) as u32 {
        !0
    } else {
        0
    };
    lo as u64 | ((hi as u64) << 32)
}
/// CMGT 32-bit: signed greater than
unsafe extern "C" fn helper_cmgt32(a: u64, b: u64) -> u64 {
    let lo: u32 = if (a as i32) > (b as i32) { !0 } else { 0 };
    let hi: u32 = if ((a >> 32) as i32) > ((b >> 32) as i32) {
        !0
    } else {
        0
    };
    lo as u64 | ((hi as u64) << 32)
}
/// CMGT 64-bit: signed greater than
unsafe extern "C" fn helper_cmgt64(a: u64, b: u64) -> u64 {
    if (a as i64) > (b as i64) {
        !0
    } else {
        0
    }
}
/// CMGE 32-bit: signed greater than or equal
unsafe extern "C" fn helper_cmge32(a: u64, b: u64) -> u64 {
    let lo: u32 = if (a as i32) >= (b as i32) { !0 } else { 0 };
    let hi: u32 = if ((a >> 32) as i32) >= ((b >> 32) as i32) {
        !0
    } else {
        0
    };
    lo as u64 | ((hi as u64) << 32)
}
unsafe extern "C" fn helper_ushr32(a: u64, shift: u64) -> u64 {
    let lo = ((a as u32) >> shift) as u64;
    let hi = (((a >> 32) as u32) >> shift) as u64;
    lo | (hi << 32)
}
unsafe extern "C" fn helper_ushr64(a: u64, shift: u64) -> u64 {
    if shift >= 64 {
        0
    } else {
        a >> shift
    }
}
/// SSHR 32-bit: signed shift right each 32-bit element
unsafe extern "C" fn helper_sshr32(a: u64, shift: u64) -> u64 {
    let lo = ((a as i32) >> shift) as u32 as u64;
    let hi = (((a >> 32) as i32) >> shift) as u32 as u64;
    lo | (hi << 32)
}
/// USHR 8-bit: logical shift right 8x8-bit elements
unsafe extern "C" fn helper_ushr8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let elem = (a >> (i * 8)) & 0xff;
        r |= (elem >> shift) << (i * 8);
    }
    r
}
/// SSHR 8-bit: arithmetic shift right 8x8-bit elements
unsafe extern "C" fn helper_sshr8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let elem = ((a >> (i * 8)) & 0xff) as i8;
        let shifted = ((elem as i32) >> shift) as u64;
        r |= (shifted & 0xff) << (i * 8);
    }
    r
}
/// UZP1 16-bit: take even-indexed 16-bit elements from two sources
unsafe extern "C" fn helper_uzp1_16(a: u64, b: u64) -> u64 {
    // a has elements [a0, a1, a2, a3], b has [b0, b1, b2, b3]
    // result: [a0, a2, b0, b2]
    let a0 = a & 0xffff;
    let a2 = (a >> 32) & 0xffff;
    let b0 = b & 0xffff;
    let b2 = (b >> 32) & 0xffff;
    a0 | (a2 << 16) | (b0 << 32) | (b2 << 48)
}
unsafe extern "C" fn helper_cmeq32_zero(a: u64) -> u64 {
    let lo: u32 = if a as u32 == 0 { !0 } else { 0 };
    let hi: u32 = if (a >> 32) as u32 == 0 { !0 } else { 0 };
    lo as u64 | ((hi as u64) << 32)
}
unsafe extern "C" fn helper_cmeq16_zero(a: u64) -> u64 {
    helper_cmeq16(a, 0)
}
/// CMLT #0 32-bit: each 32-bit element → -1 if negative, else 0
unsafe extern "C" fn helper_cmlt32_zero(a: u64) -> u64 {
    let lo: u32 = if (a as i32) < 0 { !0 } else { 0 };
    let hi: u32 = if ((a >> 32) as i32) < 0 { !0 } else { 0 };
    lo as u64 | ((hi as u64) << 32)
}
/// UZP2 32-bit: take odd-indexed 32-bit elements (high halves)
unsafe extern "C" fn helper_uzp2_32(a: u64, b: u64) -> u64 {
    let a1 = a >> 32;
    let b1 = b >> 32;
    a1 | (b1 << 32)
}
unsafe extern "C" fn helper_uzp1_32(a: u64, b: u64) -> u64 {
    let a0 = a & 0xffff_ffff;
    let b0 = b & 0xffff_ffff;
    a0 | (b0 << 32)
}
/// UZP1 8-bit: take even-indexed bytes from two 64-bit sources
unsafe extern "C" fn helper_uzp1_8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        r |= ((a >> (i * 16)) & 0xff) << (i * 8);
    }
    for i in 0..4 {
        r |= ((b >> (i * 16)) & 0xff) << ((i + 4) * 8);
    }
    r
}
/// UZP2 .8B/.16B: take odd-indexed bytes
unsafe extern "C" fn helper_uzp2_8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        r |= ((a >> (i * 16 + 8)) & 0xff) << (i * 8);
    }
    for i in 0..4 {
        r |= ((b >> (i * 16 + 8)) & 0xff) << ((i + 4) * 8);
    }
    r
}
/// ZIP1 .8B/.16B: interleave low bytes of a and b
unsafe extern "C" fn helper_zip1_8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        r |= ((a >> (i * 8)) & 0xff) << (i * 16);
        r |= ((b >> (i * 8)) & 0xff) << (i * 16 + 8);
    }
    r
}
/// ZIP2 .8B/.16B: interleave high bytes of a and b
unsafe extern "C" fn helper_zip2_8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        r |= ((a >> (i * 8 + 32)) & 0xff) << (i * 16);
        r |= ((b >> (i * 8 + 32)) & 0xff) << (i * 16 + 8);
    }
    r
}
/// UZP2 .4H/.8H: take odd-indexed 16-bit elements
unsafe extern "C" fn helper_uzp2_16(a: u64, b: u64) -> u64 {
    let a1 = (a >> 16) & 0xffff;
    let a3 = (a >> 48) & 0xffff;
    let b1 = (b >> 16) & 0xffff;
    let b3 = (b >> 48) & 0xffff;
    a1 | (a3 << 16) | (b1 << 32) | (b3 << 48)
}
/// ZIP1 .4H/.8H: interleave low halfwords
unsafe extern "C" fn helper_zip1_16(a: u64, b: u64) -> u64 {
    let a0 = a & 0xffff;
    let a1 = (a >> 16) & 0xffff;
    let b0 = b & 0xffff;
    let b1 = (b >> 16) & 0xffff;
    a0 | (b0 << 16) | (a1 << 32) | (b1 << 48)
}
/// ZIP2 .4H/.8H: interleave high halfwords
unsafe extern "C" fn helper_zip2_16(a: u64, b: u64) -> u64 {
    let a2 = (a >> 32) & 0xffff;
    let a3 = (a >> 48) & 0xffff;
    let b2 = (b >> 32) & 0xffff;
    let b3 = (b >> 48) & 0xffff;
    a2 | (b2 << 16) | (a3 << 32) | (b3 << 48)
}
/// FCVTL: convert 2x f32 (low 64 bits) to 2x f64
#[allow(dead_code, improper_ctypes_definitions)]
unsafe extern "C" fn helper_fcvtl_lo(a: u64) -> (u64, u64) {
    let f0 = f32::from_bits(a as u32) as f64;
    let f1 = f32::from_bits((a >> 32) as u32) as f64;
    (f0.to_bits(), f1.to_bits())
}
/// FCVTL2: convert 2x f32 (high 64 bits) to 2x f64
unsafe extern "C" fn helper_fcvtl2_lo(a: u64) -> u64 {
    (f32::from_bits(a as u32) as f64).to_bits()
}
unsafe extern "C" fn helper_fcvtl2_hi(a: u64) -> u64 {
    (f32::from_bits((a >> 32) as u32) as f64).to_bits()
}
/// USHLL 8→16: zero-extend 4 bytes to 4x16-bit, then shift left
unsafe extern "C" fn helper_ushll8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let byte = ((a >> (i * 8)) & 0xff) as u16;
        let wide = byte << shift;
        r |= (wide as u64) << (i * 16);
    }
    r
}
/// SSHLL 8→16: sign-extend 4 bytes from low 32 bits to 4x16-bit, then shift left.
unsafe extern "C" fn helper_sshll8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let byte = ((a >> (i * 8)) & 0xff) as u8 as i8;
        let wide = ((byte as i16) << shift) as u16;
        r |= (wide as u64) << (i * 16);
    }
    r
}
/// SSHLL 16→32: sign-extend 2 halfwords from low 32 bits to 2x32-bit, then shift left.
unsafe extern "C" fn helper_sshll16(a: u64, shift: u64) -> u64 {
    let e0 = (a & 0xffff) as u16 as i16;
    let e1 = ((a >> 16) & 0xffff) as u16 as i16;
    let w0 = ((e0 as i32) << shift) as u32 as u64;
    let w1 = ((e1 as i32) << shift) as u32 as u64;
    w0 | (w1 << 32)
}
/// XTN 64→32: narrow 2x64-bit to 2x32-bit (take low 32 bits of each element)
unsafe extern "C" fn helper_xtn32(lo: u64, hi: u64) -> u64 {
    let e0 = lo as u32 as u64;
    let e1 = hi as u32 as u64;
    e0 | (e1 << 32)
}
/// XTN 32→16: narrow 2x32-bit (packed in 64-bit) to 2x16-bit (packed in low 32-bit)
unsafe extern "C" fn helper_xtn16(a: u64) -> u64 {
    let e0 = (a as u32 & 0xffff) as u64;
    let e1 = (((a >> 32) as u32) & 0xffff) as u64;
    e0 | (e1 << 16)
}
/// REV64 for 16-bit elements: reverse 4x16-bit halfwords within a 64-bit value
unsafe extern "C" fn helper_rev64_16(a: u64) -> u64 {
    let h0 = a & 0xffff;
    let h1 = (a >> 16) & 0xffff;
    let h2 = (a >> 32) & 0xffff;
    let h3 = (a >> 48) & 0xffff;
    (h3) | (h2 << 16) | (h1 << 32) | (h0 << 48)
}
/// TBL: byte-level table lookup. table is up to 4 x 128-bit regs (passed as 64-bit halves).
/// For 2-reg TBL: table has 32 bytes (t0_lo, t0_hi, t1_lo, t1_hi), indices in idx.
unsafe extern "C" fn helper_tbl2(
    t0_lo: u64,
    t0_hi: u64,
    t1_lo: u64,
    t1_hi: u64,
    idx: u64,
) -> u64 {
    let table: [u8; 32] = {
        let mut t = [0u8; 32];
        let vals = [t0_lo, t0_hi, t1_lo, t1_hi];
        for (i, &v) in vals.iter().enumerate() {
            for j in 0..8 {
                t[i * 8 + j] = (v >> (j * 8)) as u8;
            }
        }
        t
    };
    let mut r = 0u64;
    for i in 0..8 {
        let index = ((idx >> (i * 8)) & 0xff) as usize;
        let byte = if index < 32 { table[index] } else { 0 };
        r |= (byte as u64) << (i * 8);
    }
    r
}
/// TBL 1-reg: table has 16 bytes
/// SMULL 32→64: signed multiply two 32-bit elements to produce two 64-bit results
#[allow(dead_code, improper_ctypes_definitions)]
unsafe extern "C" fn helper_smull32(a: u64, b: u64) -> (u64, u64) {
    let a0 = a as i32 as i64;
    let b0 = b as i32 as i64;
    let a1 = (a >> 32) as i32 as i64;
    let b1 = (b >> 32) as i32 as i64;
    ((a0 * b0) as u64, (a1 * b1) as u64)
}
unsafe extern "C" fn helper_smull32_lo(a: u64, b: u64) -> u64 {
    let a0 = a as i32 as i64;
    let b0 = b as i32 as i64;
    (a0 * b0) as u64
}
unsafe extern "C" fn helper_smull32_hi(a: u64, b: u64) -> u64 {
    let a0 = (a >> 32) as i32 as i64;
    let b0 = (b >> 32) as i32 as i64;
    (a0 * b0) as u64
}
unsafe extern "C" fn helper_tbl1(t_lo: u64, t_hi: u64, idx: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let index = ((idx >> (i * 8)) & 0xff) as usize;
        let byte = if index < 8 {
            (t_lo >> (index * 8)) as u8
        } else if index < 16 {
            (t_hi >> ((index - 8) * 8)) as u8
        } else {
            0
        };
        r |= (byte as u64) << (i * 8);
    }
    r
}

unsafe extern "C" fn helper_cmge_scalar(a: u64) -> u64 {
    if (a as i64) >= 0 {
        !0u64
    } else {
        0
    }
}
unsafe extern "C" fn helper_cmgt_scalar(a: u64) -> u64 {
    if (a as i64) > 0 {
        !0u64
    } else {
        0
    }
}
unsafe extern "C" fn helper_cmle_scalar(a: u64) -> u64 {
    if (a as i64) <= 0 {
        !0u64
    } else {
        0
    }
}
unsafe extern "C" fn helper_fcmgt_zero_scalar(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f > 0.0 {
        !0u64
    } else {
        0
    }
}
/// MUL vector × scalar element (32-bit): multiply two 32-bit elements by a single 32-bit scalar
unsafe extern "C" fn helper_mul32_elem(a: u64, scalar: u64) -> u64 {
    let s = scalar as u32;
    let lo = (a as u32).wrapping_mul(s) as u64;
    let hi = ((a >> 32) as u32).wrapping_mul(s) as u64;
    lo | (hi << 32)
}
/// MLA by element 32-bit: Vd += Vn * scalar
unsafe extern "C" fn helper_mla32_elem(vd: u64, vn: u64, scalar: u64) -> u64 {
    let s = scalar as u32;
    let lo = (vd as u32).wrapping_add((vn as u32).wrapping_mul(s)) as u64;
    let hi = ((vd >> 32) as u32)
        .wrapping_add(((vn >> 32) as u32).wrapping_mul(s)) as u64;
    lo | (hi << 32)
}
/// MLA 32-bit: Vd += Vn * Vm (element-wise)
unsafe extern "C" fn helper_mla32(vd: u64, vn: u64, vm: u64) -> u64 {
    let lo =
        (vd as u32).wrapping_add((vn as u32).wrapping_mul(vm as u32)) as u64;
    let hi = ((vd >> 32) as u32)
        .wrapping_add(((vn >> 32) as u32).wrapping_mul((vm >> 32) as u32))
        as u64;
    lo | (hi << 32)
}
/// MLS 32-bit: Vd = Vd - Vn * Vm per 32-bit element
unsafe extern "C" fn helper_mls32(vd: u64, vn: u64, vm: u64) -> u64 {
    let lo =
        (vd as u32).wrapping_sub((vn as u32).wrapping_mul(vm as u32)) as u64;
    let hi = ((vd >> 32) as u32)
        .wrapping_sub(((vn >> 32) as u32).wrapping_mul((vm >> 32) as u32))
        as u64;
    lo | (hi << 32)
}
/// SCVTF scalar d,d: signed 64-bit int → double
unsafe extern "C" fn helper_scvtf_d_d(a: u64) -> u64 {
    ((a as i64) as f64).to_bits()
}
/// FCVT single→double
unsafe extern "C" fn helper_fcvt_s_to_d(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    (f as f64).to_bits()
}
/// FCVT double→single
unsafe extern "C" fn helper_fcvt_d_to_s(a: u64) -> u64 {
    let f = f64::from_bits(a);
    (f as f32).to_bits() as u64
}
/// USHLL 16→32: zero-extend 2 halfwords to 2x32-bit, then shift left
unsafe extern "C" fn helper_ushll16(a: u64, shift: u64) -> u64 {
    let e0 = (a & 0xffff) as u32;
    let e1 = ((a >> 16) & 0xffff) as u32;
    let w0 = (e0 << shift) as u64;
    let w1 = (e1 << shift) as u64;
    w0 | (w1 << 32)
}

/// BIF: Vd = (Vd & Vm) | (Vn & ~Vm)
unsafe extern "C" fn helper_bif(vd: u64, vn: u64, vm: u64) -> u64 {
    (vd & vm) | (vn & !vm)
}

/// BSL: Vd = (Vn & Vd) | (Vm & ~Vd)
unsafe extern "C" fn helper_bsl(vd: u64, vn: u64, vm: u64) -> u64 {
    (vn & vd) | (vm & !vd)
}

/// ADDV: add across vector (byte elements).
unsafe extern "C" fn helper_addv8(a: u64) -> u64 {
    let mut sum = 0u64;
    for i in 0..8 {
        sum += (a >> (i * 8)) & 0xff;
    }
    sum & 0xff
}

/// ADDV .4S: add all four 32-bit elements from lo and hi halves.
unsafe extern "C" fn helper_addv32(lo: u64, hi: u64) -> u64 {
    let s0 = (lo as u32).wrapping_add((lo >> 32) as u32);
    let s1 = (hi as u32).wrapping_add((hi >> 32) as u32);
    s0.wrapping_add(s1) as u64
}

/// Byte-wise shift left.
unsafe extern "C" fn helper_shl8(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let byte = (a >> (i * 8)) & 0xff;
        r |= ((byte << shift) & 0xff) << (i * 8);
    }
    r
}
/// SHL 32-bit: shift left 2x32-bit elements packed in 64 bits
unsafe extern "C" fn helper_shl32(a: u64, shift: u64) -> u64 {
    let lo = ((a as u32) << shift) as u64;
    let hi = (((a >> 32) as u32) << shift) as u64;
    lo | (hi << 32)
}
/// SHL 16-bit: shift left 4x16-bit elements packed in 64 bits
unsafe extern "C" fn helper_shl16(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let elem = (a >> (i * 16)) & 0xffff;
        r |= ((elem << shift) & 0xffff) << (i * 16);
    }
    r
}
/// USHR 16-bit: logical shift right 4x16-bit elements
unsafe extern "C" fn helper_ushr16(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let elem = (a >> (i * 16)) & 0xffff;
        r |= (elem >> shift) << (i * 16);
    }
    r
}
/// SSHR 16-bit: arithmetic shift right 4x16-bit elements
unsafe extern "C" fn helper_sshr16(a: u64, shift: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let elem = ((a >> (i * 16)) & 0xffff) as i16;
        let shifted = ((elem as i32) >> shift) as u64;
        r |= (shifted & 0xffff) << (i * 16);
    }
    r
}
/// Vector FP: fadd 2x f32 packed in 64 bits
unsafe extern "C" fn helper_vfadd32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (a0 + b0).to_bits() as u64 | (((a1 + b1).to_bits() as u64) << 32)
}
/// Vector FP: fsub 2x f32 packed in 64 bits
unsafe extern "C" fn helper_vfsub32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (a0 - b0).to_bits() as u64 | (((a1 - b1).to_bits() as u64) << 32)
}
/// Vector FP: fmul 2x f32 packed in 64 bits
unsafe extern "C" fn helper_vfmul32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (a0 * b0).to_bits() as u64 | (((a1 * b1).to_bits() as u64) << 32)
}
/// Vector FP: fdiv 2x f32 packed in 64 bits
unsafe extern "C" fn helper_vfdiv32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (a0 / b0).to_bits() as u64 | (((a1 / b1).to_bits() as u64) << 32)
}
/// Vector FP: fabd 2x f32 packed in 64 bits (absolute difference)
unsafe extern "C" fn helper_vfabd32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (a0 - b0).abs().to_bits() as u64
        | (((a1 - b1).abs().to_bits() as u64) << 32)
}
/// Vector FP: fmax 2x f32
unsafe extern "C" fn helper_vfmax32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    a0.max(b0).to_bits() as u64 | ((a1.max(b1).to_bits() as u64) << 32)
}
/// Vector FP: fmin 2x f32
unsafe extern "C" fn helper_vfmin32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    a0.min(b0).to_bits() as u64 | ((a1.min(b1).to_bits() as u64) << 32)
}
/// Vector FP: fcmeq (==) 2x f32: each lane: all-ones if equal, else 0
unsafe extern "C" fn helper_vfcmeq32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    let r0: u32 = if a0 == b0 { !0 } else { 0 };
    let r1: u32 = if a1 == b1 { !0 } else { 0 };
    r0 as u64 | ((r1 as u64) << 32)
}
/// Vector FP: fcmgt (>) 2x f32
unsafe extern "C" fn helper_vfcmgt32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    let r0: u32 = if a0 > b0 { !0 } else { 0 };
    let r1: u32 = if a1 > b1 { !0 } else { 0 };
    r0 as u64 | ((r1 as u64) << 32)
}
/// Vector FP: fcmge (>=) 2x f32
unsafe extern "C" fn helper_vfcmge32(a: u64, b: u64) -> u64 {
    let a0 = f32::from_bits(a as u32);
    let b0 = f32::from_bits(b as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    let r0: u32 = if a0 >= b0 { !0 } else { 0 };
    let r1: u32 = if a1 >= b1 { !0 } else { 0 };
    r0 as u64 | ((r1 as u64) << 32)
}
/// scvtf vector: convert 2x i32 to 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vscvtf32(a: u64) -> u64 {
    let e0 = (a as i32) as f32;
    let e1 = ((a >> 32) as i32) as f32;
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// scvtf vector: convert 2x i64 to 2x f64 (one lane per 64-bit half)
unsafe extern "C" fn helper_vscvtf64(a: u64) -> u64 {
    (a as i64 as f64).to_bits()
}
/// ucvtf vector: convert 2x u64 to 2x f64 (one lane per 64-bit half)
unsafe extern "C" fn helper_vucvtf64(a: u64) -> u64 {
    (a as f64).to_bits()
}
/// fcvtzs vector: convert f64 → i64 (one lane per 64-bit half)
#[allow(dead_code)]
unsafe extern "C" fn helper_vfcvtzs64(a: u64) -> u64 {
    (f64::from_bits(a) as i64) as u64
}
/// faddp vector 32-bit: pairwise add adjacent f32 lanes from two halves
/// Input: n = [n0, n1] (two f32 in u64), m = [m0, m1] (two f32 in u64)
/// Output: [n0+n1, m0+m1]
#[allow(dead_code)]
unsafe extern "C" fn helper_faddp32(n: u64, m: u64) -> u64 {
    let n0 = f32::from_bits(n as u32);
    let n1 = f32::from_bits((n >> 32) as u32);
    let m0 = f32::from_bits(m as u32);
    let m1 = f32::from_bits((m >> 32) as u32);
    let r0 = (n0 + n1).to_bits() as u64;
    let r1 = (m0 + m1).to_bits() as u64;
    r0 | (r1 << 32)
}
/// shl vector: shift left each 64-bit lane by shift amount
unsafe extern "C" fn helper_shl64(a: u64, shift: u64) -> u64 {
    if shift >= 64 {
        0
    } else {
        a << shift
    }
}
/// ucvtf vector: convert 2x u32 to 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vucvtf32(a: u64) -> u64 {
    let e0 = (a as u32) as f32;
    let e1 = ((a >> 32) as u32) as f32;
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// fcvtzs vector: convert 2x f32 to 2x i32 (truncate toward zero)
unsafe extern "C" fn helper_vfcvtzs32(a: u64) -> u64 {
    let e0 = f32::from_bits(a as u32) as i32 as u32;
    let e1 = f32::from_bits((a >> 32) as u32) as i32 as u32;
    e0 as u64 | ((e1 as u64) << 32)
}
/// fcvtzs vector fixed-point: multiply by 2^fbits then truncate to i32
unsafe extern "C" fn helper_vfcvtzs32_fixedpt(a: u64, fbits: u64) -> u64 {
    let scale = (1u64 << fbits) as f32;
    let e0 = (f32::from_bits(a as u32) * scale) as i32 as u32;
    let e1 = (f32::from_bits((a >> 32) as u32) * scale) as i32 as u32;
    e0 as u64 | ((e1 as u64) << 32)
}
/// SMLAL .2D: signed multiply-accumulate long — acc += sext(n) * sext(m), low lanes
unsafe extern "C" fn helper_smlal32_lo(acc: u64, n: u64, m: u64) -> u64 {
    let n0 = n as u32 as i32 as i64;
    let m0 = m as u32 as i32 as i64;
    (acc as i64).wrapping_add(n0.wrapping_mul(m0)) as u64
}
unsafe extern "C" fn helper_smlal32_hi(acc: u64, n: u64, m: u64) -> u64 {
    let n1 = (n >> 32) as u32 as i32 as i64;
    let m1 = (m >> 32) as u32 as i32 as i64;
    (acc as i64).wrapping_add(n1.wrapping_mul(m1)) as u64
}
/// fabs vector: abs 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vfabs32(a: u64) -> u64 {
    let e0 = f32::from_bits(a as u32).abs();
    let e1 = f32::from_bits((a >> 32) as u32).abs();
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// fneg vector: negate 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vfneg32(a: u64) -> u64 {
    let e0 = -f32::from_bits(a as u32);
    let e1 = -f32::from_bits((a >> 32) as u32);
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// fsqrt vector: sqrt 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vfsqrt32(a: u64) -> u64 {
    let e0 = f32::from_bits(a as u32).sqrt();
    let e1 = f32::from_bits((a >> 32) as u32).sqrt();
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// frecpe vector: reciprocal estimate 2x f32
#[allow(dead_code)]
unsafe extern "C" fn helper_vfrecpe32(a: u64) -> u64 {
    let e0 = 1.0f32 / f32::from_bits(a as u32);
    let e1 = 1.0f32 / f32::from_bits((a >> 32) as u32);
    e0.to_bits() as u64 | ((e1.to_bits() as u64) << 32)
}
/// fmla vector: fused multiply-add 2x f32: d = a + b*c
unsafe extern "C" fn helper_vfmla32(d: u64, a: u64, b: u64) -> u64 {
    let d0 = f32::from_bits(d as u32);
    let d1 = f32::from_bits((d >> 32) as u32);
    let a0 = f32::from_bits(a as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b0 = f32::from_bits(b as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (d0 + a0 * b0).to_bits() as u64 | (((d1 + a1 * b1).to_bits() as u64) << 32)
}
/// fmls vector: fused multiply-subtract 2x f32: d = d - a*b
unsafe extern "C" fn helper_vfmls32(d: u64, a: u64, b: u64) -> u64 {
    let d0 = f32::from_bits(d as u32);
    let d1 = f32::from_bits((d >> 32) as u32);
    let a0 = f32::from_bits(a as u32);
    let a1 = f32::from_bits((a >> 32) as u32);
    let b0 = f32::from_bits(b as u32);
    let b1 = f32::from_bits((b >> 32) as u32);
    (d0 - a0 * b0).to_bits() as u64 | (((d1 - a1 * b1).to_bits() as u64) << 32)
}

/// XTN: narrow 16→8 (take low byte of each 16-bit element, 4 elements).
unsafe extern "C" fn helper_xtn8(a: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..4 {
        let elem = (a >> (i * 16)) & 0xff;
        r |= elem << (i * 8);
    }
    r
}

// ── FP helper functions ─────────────────────────────────

unsafe extern "C" fn helper_fcmp32(a: u64, b: u64) -> u64 {
    let fa = f32::from_bits(a as u32);
    let fb = f32::from_bits(b as u32);
    let nzcv = if fa.is_nan() || fb.is_nan() {
        0b0011u64
    } else if fa == fb {
        0b0110
    } else if fa < fb {
        0b1000
    } else {
        0b0010
    };
    nzcv << 28
}
unsafe extern "C" fn helper_fcvtms_w_d(a: u64) -> u64 {
    let f = f64::from_bits(a).floor();
    if f.is_nan() {
        0
    } else if f >= i32::MAX as f64 {
        i32::MAX as u64
    } else if f <= i32::MIN as f64 {
        i32::MIN as u32 as u64
    } else {
        f as i32 as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtms_x_d(a: u64) -> u64 {
    let f = f64::from_bits(a).floor();
    if f.is_nan() {
        0
    } else if f >= i64::MAX as f64 {
        i64::MAX as u64
    } else if f <= i64::MIN as f64 {
        i64::MIN as u64
    } else {
        f as i64 as u64
    }
}
unsafe extern "C" fn helper_fcvtms_w_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32).floor();
    if f.is_nan() {
        0
    } else if f >= i32::MAX as f32 {
        i32::MAX as u64
    } else if f <= i32::MIN as f32 {
        i32::MIN as u32 as u64
    } else {
        f as i32 as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtms_x_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32).floor();
    if f.is_nan() {
        0
    } else if f >= i64::MAX as f32 {
        i64::MAX as u64
    } else if f <= i64::MIN as f32 {
        i64::MIN as u64
    } else {
        f as i64 as u64
    }
}
unsafe extern "C" fn helper_neg8(a: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let b = ((a >> (i * 8)) & 0xff) as u8;
        r |= (b.wrapping_neg() as u64) << (i * 8);
    }
    r
}
unsafe extern "C" fn helper_neg16(a: u64) -> u64 {
    let lo = (a as u16).wrapping_neg() as u64;
    let hi = ((a >> 16) as u16).wrapping_neg() as u64;
    let hi2 = ((a >> 32) as u16).wrapping_neg() as u64;
    let hi3 = ((a >> 48) as u16).wrapping_neg() as u64;
    lo | (hi << 16) | (hi2 << 32) | (hi3 << 48)
}
unsafe extern "C" fn helper_neg32(a: u64) -> u64 {
    let lo = (a as u32).wrapping_neg() as u64;
    let hi = ((a >> 32) as u32).wrapping_neg() as u64;
    lo | (hi << 32)
}
// REV64 .2S: swap the two 32-bit words within a 64-bit lane
unsafe extern "C" fn helper_rev64_2s(a: u64) -> u64 {
    let lo = a as u32;
    let hi = (a >> 32) as u32;
    (lo as u64) << 32 | (hi as u64)
}
// REV64 .4H: reverse 4 halfwords within a 64-bit lane
#[allow(dead_code)]
unsafe extern "C" fn helper_rev64_4h(a: u64) -> u64 {
    let h0 = (a) as u16 as u64;
    let h1 = (a >> 16) as u16 as u64;
    let h2 = (a >> 32) as u16 as u64;
    let h3 = (a >> 48) as u16 as u64;
    h3 | (h2 << 16) | (h1 << 32) | (h0 << 48)
}
// USHL .2S: unsigned shift each 32-bit lane by signed amount in corresponding lane of Vm
unsafe extern "C" fn helper_ushl32(vn: u64, vm: u64) -> u64 {
    let shift0 = (vm as i8) as i32;
    let shift1 = ((vm >> 32) as i8) as i32;
    let n0 = vn as u32;
    let n1 = (vn >> 32) as u32;
    let r0 = if shift0 >= 32 || shift0 <= -32 {
        0u32
    } else if shift0 >= 0 {
        n0.wrapping_shl(shift0 as u32)
    } else {
        n0.wrapping_shr((-shift0) as u32)
    };
    let r1 = if shift1 >= 32 || shift1 <= -32 {
        0u32
    } else if shift1 >= 0 {
        n1.wrapping_shl(shift1 as u32)
    } else {
        n1.wrapping_shr((-shift1) as u32)
    };
    (r0 as u64) | ((r1 as u64) << 32)
}
// SSHL .2S: signed shift each 32-bit lane by signed amount in corresponding lane of Vm
unsafe extern "C" fn helper_sshl32(vn: u64, vm: u64) -> u64 {
    let shift0 = (vm as i8) as i32;
    let shift1 = ((vm >> 32) as i8) as i32;
    let n0 = vn as u32 as i32;
    let n1 = (vn >> 32) as u32 as i32;
    let r0 = if shift0 >= 32 {
        0i32
    } else if shift0 <= -32 {
        n0 >> 31
    } else if shift0 >= 0 {
        n0.wrapping_shl(shift0 as u32)
    } else {
        n0 >> (-shift0)
    };
    let r1 = if shift1 >= 32 {
        0i32
    } else if shift1 <= -32 {
        n1 >> 31
    } else if shift1 >= 0 {
        n1.wrapping_shl(shift1 as u32)
    } else {
        n1 >> (-shift1)
    };
    (r0 as u32 as u64) | ((r1 as u32 as u64) << 32)
}
// USHL .1D/.2D: unsigned shift each 64-bit lane by signed amount in corresponding lane of Vm
unsafe extern "C" fn helper_ushl64(vn: u64, vm: u64) -> u64 {
    let shift = vm as i64;
    if shift >= 64 || shift <= -64 {
        0
    } else if shift >= 0 {
        vn.wrapping_shl(shift as u32)
    } else {
        vn.wrapping_shr((-shift) as u32)
    }
}
// SSHL .1D/.2D: signed shift each 64-bit lane by signed amount in corresponding lane of Vm
unsafe extern "C" fn helper_sshl64(vn: u64, vm: u64) -> u64 {
    let shift = vm as i64;
    let n = vn as i64;
    let r = if shift >= 64 {
        0i64
    } else if shift <= -64 {
        n >> 63
    } else if shift >= 0 {
        n.wrapping_shl(shift as u32)
    } else {
        n >> (-shift)
    };
    r as u64
}
// SADDW .4S += sext(.4H): add each of 4 signed 16-bit lanes from src into 4 32-bit lanes of acc
unsafe extern "C" fn helper_saddw16_lo(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32) as i32;
    let a1 = ((acc >> 32) as u32) as i32;
    let s0 = (src as i16) as i32;
    let s1 = ((src >> 16) as i16) as i32;
    let r0 = a0.wrapping_add(s0) as u32 as u64;
    let r1 = a1.wrapping_add(s1) as u32 as u64;
    r0 | (r1 << 32)
}
unsafe extern "C" fn helper_saddw16_hi(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32) as i32;
    let a1 = ((acc >> 32) as u32) as i32;
    let s0 = ((src >> 32) as i16) as i32;
    let s1 = ((src >> 48) as i16) as i32;
    let r0 = a0.wrapping_add(s0) as u32 as u64;
    let r1 = a1.wrapping_add(s1) as u32 as u64;
    r0 | (r1 << 32)
}
unsafe extern "C" fn helper_uaddw16_lo(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32).wrapping_add((src as u16) as u32) as u64;
    let a1 =
        ((acc >> 32) as u32).wrapping_add(((src >> 16) as u16) as u32) as u64;
    a0 | (a1 << 32)
}
unsafe extern "C" fn helper_uaddw16_hi(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32).wrapping_add(((src >> 32) as u16) as u32) as u64;
    let a1 =
        ((acc >> 32) as u32).wrapping_add(((src >> 48) as u16) as u32) as u64;
    a0 | (a1 << 32)
}
unsafe extern "C" fn helper_ssubw16_lo(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32) as i32;
    let a1 = ((acc >> 32) as u32) as i32;
    let s0 = (src as i16) as i32;
    let s1 = ((src >> 16) as i16) as i32;
    let r0 = a0.wrapping_sub(s0) as u32 as u64;
    let r1 = a1.wrapping_sub(s1) as u32 as u64;
    r0 | (r1 << 32)
}
unsafe extern "C" fn helper_ssubw16_hi(acc: u64, src: u64) -> u64 {
    let a0 = (acc as u32) as i32;
    let a1 = ((acc >> 32) as u32) as i32;
    let s0 = ((src >> 32) as i16) as i32;
    let s1 = ((src >> 48) as i16) as i32;
    let r0 = a0.wrapping_sub(s0) as u32 as u64;
    let r1 = a1.wrapping_sub(s1) as u32 as u64;
    r0 | (r1 << 32)
}
// SADDW .2D += sext(.2S)
unsafe extern "C" fn helper_saddw32_lo(acc: u64, src: u64) -> u64 {
    acc.wrapping_add((src as u32 as i32 as i64) as u64)
}
unsafe extern "C" fn helper_saddw32_hi(acc: u64, src: u64) -> u64 {
    acc.wrapping_add(((src >> 32) as u32 as i32 as i64) as u64)
}
// CMHI .8B: unsigned greater-than per byte, result = 0xFF or 0x00
unsafe extern "C" fn helper_cmhi8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let va = ((a >> (i * 8)) & 0xff) as u8;
        let vb = ((b >> (i * 8)) & 0xff) as u8;
        if va > vb {
            r |= 0xffu64 << (i * 8);
        }
    }
    r
}
// CMGT .8B: signed greater-than per byte
unsafe extern "C" fn helper_cmgt8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let va = ((a >> (i * 8)) & 0xff) as u8 as i8;
        let vb = ((b >> (i * 8)) & 0xff) as u8 as i8;
        if va > vb {
            r |= 0xffu64 << (i * 8);
        }
    }
    r
}
// SSRA .2S: signed shift right and accumulate per 32-bit lane
unsafe extern "C" fn helper_ssra32(acc: u64, src: u64, shift: u64) -> u64 {
    let sh = shift as u32;
    let s0 = (src as u32 as i32) >> sh;
    let s1 = ((src >> 32) as u32 as i32) >> sh;
    let a0 = (acc as u32).wrapping_add(s0 as u32);
    let a1 = ((acc >> 32) as u32).wrapping_add(s1 as u32);
    (a0 as u64) | ((a1 as u64) << 32)
}
// USUBL .4S: Vd.4S = zext(Vn.4H) - zext(Vm.4H), low two lanes
unsafe extern "C" fn helper_usubl16_lo(n: u64, m: u64) -> u64 {
    let n0 = (n as u16) as u32;
    let n1 = ((n >> 16) as u16) as u32;
    let m0 = (m as u16) as u32;
    let m1 = ((m >> 16) as u16) as u32;
    let r0 = n0.wrapping_sub(m0) as u64;
    let r1 = n1.wrapping_sub(m1) as u64;
    r0 | (r1 << 32)
}
unsafe extern "C" fn helper_usubl16_hi(n: u64, m: u64) -> u64 {
    let n0 = ((n >> 32) as u16) as u32;
    let n1 = ((n >> 48) as u16) as u32;
    let m0 = ((m >> 32) as u16) as u32;
    let m1 = ((m >> 48) as u16) as u32;
    let r0 = n0.wrapping_sub(m0) as u64;
    let r1 = n1.wrapping_sub(m1) as u64;
    r0 | (r1 << 32)
}
unsafe extern "C" fn helper_fadd64(a: u64, b: u64) -> u64 {
    (f64::from_bits(a) + f64::from_bits(b)).to_bits()
}
unsafe extern "C" fn helper_fsub64(a: u64, b: u64) -> u64 {
    (f64::from_bits(a) - f64::from_bits(b)).to_bits()
}
unsafe extern "C" fn helper_fmul64(a: u64, b: u64) -> u64 {
    (f64::from_bits(a) * f64::from_bits(b)).to_bits()
}
unsafe extern "C" fn helper_fnmul64(a: u64, b: u64) -> u64 {
    (-(f64::from_bits(a) * f64::from_bits(b))).to_bits()
}
unsafe extern "C" fn helper_fnmul32(a: u64, b: u64) -> u64 {
    (-(f32::from_bits(a as u32) * f32::from_bits(b as u32))).to_bits() as u64
}
unsafe extern "C" fn helper_fmax64(a: u64, b: u64) -> u64 {
    let fa = f64::from_bits(a);
    let fb = f64::from_bits(b);
    // IEEE 754: FMAX returns the larger, treating NaN from sNaN specially
    if fa.is_nan() {
        fb.to_bits()
    } else if fb.is_nan() {
        fa.to_bits()
    } else {
        fa.max(fb).to_bits()
    }
}
unsafe extern "C" fn helper_fmin64(a: u64, b: u64) -> u64 {
    let fa = f64::from_bits(a);
    let fb = f64::from_bits(b);
    if fa.is_nan() {
        fb.to_bits()
    } else if fb.is_nan() {
        fa.to_bits()
    } else {
        fa.min(fb).to_bits()
    }
}
unsafe extern "C" fn helper_fmaxnm64(a: u64, b: u64) -> u64 {
    let fa = f64::from_bits(a);
    let fb = f64::from_bits(b);
    if fa.is_nan() && fb.is_nan() {
        a
    } else if fa.is_nan() {
        fb.to_bits()
    } else if fb.is_nan() {
        fa.to_bits()
    } else {
        fa.max(fb).to_bits()
    }
}
unsafe extern "C" fn helper_fminnm64(a: u64, b: u64) -> u64 {
    let fa = f64::from_bits(a);
    let fb = f64::from_bits(b);
    if fa.is_nan() && fb.is_nan() {
        a
    } else if fa.is_nan() {
        fb.to_bits()
    } else if fb.is_nan() {
        fa.to_bits()
    } else {
        fa.min(fb).to_bits()
    }
}
unsafe extern "C" fn helper_fmax32(a: u64, b: u64) -> u64 {
    let fa = f32::from_bits(a as u32);
    let fb = f32::from_bits(b as u32);
    if fa.is_nan() {
        fb.to_bits() as u64
    } else if fb.is_nan() {
        fa.to_bits() as u64
    } else {
        fa.max(fb).to_bits() as u64
    }
}
unsafe extern "C" fn helper_fmin32(a: u64, b: u64) -> u64 {
    let fa = f32::from_bits(a as u32);
    let fb = f32::from_bits(b as u32);
    if fa.is_nan() {
        fb.to_bits() as u64
    } else if fb.is_nan() {
        fa.to_bits() as u64
    } else {
        fa.min(fb).to_bits() as u64
    }
}
unsafe extern "C" fn helper_fmaxnm32(a: u64, b: u64) -> u64 {
    let fa = f32::from_bits(a as u32);
    let fb = f32::from_bits(b as u32);
    if fa.is_nan() && fb.is_nan() {
        a
    } else if fa.is_nan() {
        fb.to_bits() as u64
    } else if fb.is_nan() {
        fa.to_bits() as u64
    } else {
        fa.max(fb).to_bits() as u64
    }
}
unsafe extern "C" fn helper_fminnm32(a: u64, b: u64) -> u64 {
    let fa = f32::from_bits(a as u32);
    let fb = f32::from_bits(b as u32);
    if fa.is_nan() && fb.is_nan() {
        a
    } else if fa.is_nan() {
        fb.to_bits() as u64
    } else if fb.is_nan() {
        fa.to_bits() as u64
    } else {
        fa.min(fb).to_bits() as u64
    }
}
unsafe extern "C" fn helper_fcvtps_w_d(a: u64) -> u64 {
    let f = f64::from_bits(a).ceil();
    if f.is_nan() {
        0
    } else if f >= i32::MAX as f64 {
        i32::MAX as u64
    } else if f <= i32::MIN as f64 {
        i32::MIN as u32 as u64
    } else {
        f as i32 as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtps_x_d(a: u64) -> u64 {
    let f = f64::from_bits(a).ceil();
    if f.is_nan() {
        0
    } else if f >= i64::MAX as f64 {
        i64::MAX as u64
    } else if f <= i64::MIN as f64 {
        i64::MIN as u64
    } else {
        f as i64 as u64
    }
}
unsafe extern "C" fn helper_fcvtps_w_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32).ceil();
    if f.is_nan() {
        0
    } else if f >= i32::MAX as f32 {
        i32::MAX as u64
    } else if f <= i32::MIN as f32 {
        i32::MIN as u32 as u64
    } else {
        f as i32 as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtps_x_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32).ceil();
    if f.is_nan() {
        0
    } else if f >= i64::MAX as f32 {
        i64::MAX as u64
    } else if f <= i64::MIN as f32 {
        i64::MIN as u64
    } else {
        f as i64 as u64
    }
}
unsafe extern "C" fn helper_fabd64(a: u64, b: u64) -> u64 {
    let r = f64::from_bits(a) - f64::from_bits(b);
    r.abs().to_bits()
}
unsafe extern "C" fn helper_fabd32(a: u64, b: u64) -> u64 {
    let r = f32::from_bits(a as u32) - f32::from_bits(b as u32);
    r.abs().to_bits() as u64
}
unsafe extern "C" fn helper_fdiv64(a: u64, b: u64) -> u64 {
    (f64::from_bits(a) / f64::from_bits(b)).to_bits()
}
unsafe extern "C" fn helper_fsqrt64(a: u64) -> u64 {
    f64::from_bits(a).sqrt().to_bits()
}
unsafe extern "C" fn helper_fsqrt32(a: u64) -> u64 {
    f32::from_bits(a as u32).sqrt().to_bits() as u64
}
unsafe extern "C" fn helper_fmadd64(a: u64, b: u64, c: u64) -> u64 {
    (f64::from_bits(a) * f64::from_bits(b) + f64::from_bits(c)).to_bits()
}
unsafe extern "C" fn helper_fmsub64(a: u64, b: u64, c: u64) -> u64 {
    (-(f64::from_bits(a) * f64::from_bits(b)) + f64::from_bits(c)).to_bits()
}
unsafe extern "C" fn helper_fmadd32(a: u64, b: u64, c: u64) -> u64 {
    (f32::from_bits(a as u32) * f32::from_bits(b as u32)
        + f32::from_bits(c as u32))
    .to_bits() as u64
}
unsafe extern "C" fn helper_fmsub32(a: u64, b: u64, c: u64) -> u64 {
    (-(f32::from_bits(a as u32) * f32::from_bits(b as u32))
        + f32::from_bits(c as u32))
    .to_bits() as u64
}
unsafe extern "C" fn helper_fnmadd64(a: u64, b: u64, c: u64) -> u64 {
    (-(f64::from_bits(a) * f64::from_bits(b)) - f64::from_bits(c)).to_bits()
}
unsafe extern "C" fn helper_fnmsub64(a: u64, b: u64, c: u64) -> u64 {
    (f64::from_bits(a) * f64::from_bits(b) - f64::from_bits(c)).to_bits()
}
unsafe extern "C" fn helper_fnmadd32(a: u64, b: u64, c: u64) -> u64 {
    (-(f32::from_bits(a as u32) * f32::from_bits(b as u32))
        - f32::from_bits(c as u32))
    .to_bits() as u64
}
unsafe extern "C" fn helper_fnmsub32(a: u64, b: u64, c: u64) -> u64 {
    (f32::from_bits(a as u32) * f32::from_bits(b as u32)
        - f32::from_bits(c as u32))
    .to_bits() as u64
}
unsafe extern "C" fn helper_fadd32(a: u64, b: u64) -> u64 {
    (f32::from_bits(a as u32) + f32::from_bits(b as u32)).to_bits() as u64
}
unsafe extern "C" fn helper_fsub32(a: u64, b: u64) -> u64 {
    (f32::from_bits(a as u32) - f32::from_bits(b as u32)).to_bits() as u64
}
unsafe extern "C" fn helper_fmul32(a: u64, b: u64) -> u64 {
    (f32::from_bits(a as u32) * f32::from_bits(b as u32)).to_bits() as u64
}
unsafe extern "C" fn helper_fdiv32(a: u64, b: u64) -> u64 {
    (f32::from_bits(a as u32) / f32::from_bits(b as u32)).to_bits() as u64
}
unsafe extern "C" fn helper_scvtf_d_x(a: u64) -> u64 {
    ((a as i64) as f64).to_bits()
}
unsafe extern "C" fn helper_scvtf_d_w(a: u64) -> u64 {
    ((a as u32 as i32) as f64).to_bits()
}
unsafe extern "C" fn helper_ucvtf_d_w(a: u64) -> u64 {
    ((a as u32) as f64).to_bits()
}
unsafe extern "C" fn helper_ucvtf_d_x(a: u64) -> u64 {
    (a as f64).to_bits()
}
// Single-precision (Sd) conversions from/to GPR
unsafe extern "C" fn helper_scvtf_s_w(a: u64) -> u64 {
    ((a as u32 as i32) as f32).to_bits() as u64
}
unsafe extern "C" fn helper_scvtf_s_x(a: u64) -> u64 {
    ((a as i64) as f32).to_bits() as u64
}
unsafe extern "C" fn helper_ucvtf_s_w(a: u64) -> u64 {
    ((a as u32) as f32).to_bits() as u64
}
unsafe extern "C" fn helper_ucvtf_s_x(a: u64) -> u64 {
    (a as f32).to_bits() as u64
}
unsafe extern "C" fn helper_scvtf_d_w_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    (((a as u32 as i32) as f64) / scale).to_bits()
}
unsafe extern "C" fn helper_scvtf_d_x_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    ((a as i64) as f64 / scale).to_bits()
}
unsafe extern "C" fn helper_ucvtf_d_w_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    ((a as u32) as f64 / scale).to_bits()
}
unsafe extern "C" fn helper_ucvtf_d_x_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    (a as f64 / scale).to_bits()
}
unsafe extern "C" fn helper_scvtf_s_w_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    (((a as u32 as i32) as f32) / scale).to_bits() as u64
}
unsafe extern "C" fn helper_scvtf_s_x_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    ((a as i64) as f32 / scale).to_bits() as u64
}
unsafe extern "C" fn helper_ucvtf_s_w_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    ((a as u32) as f32 / scale).to_bits() as u64
}
unsafe extern "C" fn helper_ucvtf_s_x_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    (a as f32 / scale).to_bits() as u64
}
// Single-precision: integer-in-Sn register to float-in-Sd
unsafe extern "C" fn helper_scvtf_s_s(a: u64) -> u64 {
    ((a as u32 as i32) as f32).to_bits() as u64
}
unsafe extern "C" fn helper_ucvtf_s_s(a: u64) -> u64 {
    ((a as u32) as f32).to_bits() as u64
}
// FCVTZx single-precision float to integer
unsafe extern "C" fn helper_fcvtzu_w_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() || f <= 0.0 {
        0
    } else if f >= u32::MAX as f32 {
        u32::MAX as u64
    } else {
        f as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtzu_x_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() || f <= 0.0 {
        0
    } else if f >= u64::MAX as f32 {
        u64::MAX
    } else {
        f as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_w_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() {
        0
    } else if f >= i32::MAX as f32 {
        i32::MAX as u64
    } else if f <= i32::MIN as f32 {
        i32::MIN as u32 as u64
    } else {
        f as i32 as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_x_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() {
        0
    } else if f >= i64::MAX as f32 {
        i64::MAX as u64
    } else if f <= i64::MIN as f32 {
        i64::MIN as u64
    } else {
        f as i64 as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_w_s_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    let f = f32::from_bits(a as u32) * scale;
    if f.is_nan() {
        0
    } else {
        (f as i32) as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtzu_w_s_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f32).powi(fbits as i32);
    let f = f32::from_bits(a as u32) * scale;
    if f.is_nan() || f <= 0.0 {
        0
    } else {
        (f as u32) as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_x_d_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    let f = f64::from_bits(a) * scale;
    if f.is_nan() {
        0
    } else {
        (f as i64) as u64
    }
}
unsafe extern "C" fn helper_fcvtzu_x_d_fixed(a: u64, fbits: u64) -> u64 {
    let scale = (2.0f64).powi(fbits as i32);
    let f = f64::from_bits(a) * scale;
    if f.is_nan() || f <= 0.0 {
        0
    } else {
        f as u64
    }
}
unsafe extern "C" fn helper_fcvtas_x_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() {
        0
    } else {
        f.round() as i64 as u64
    }
}
unsafe extern "C" fn helper_fcvtas_w_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    if f.is_nan() {
        0
    } else {
        f.round() as i32 as u64
    }
}
unsafe extern "C" fn helper_fcvtzu_x_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() || f < 0.0 {
        0
    } else {
        f as u64
    }
}
unsafe extern "C" fn helper_fcvtzu_w_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() || f < 0.0 {
        0
    } else {
        (f as u32) as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_w_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() {
        0
    } else {
        (f as i32) as u32 as u64
    }
}
unsafe extern "C" fn helper_fcvtzs_x_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() {
        0
    } else {
        (f as i64) as u64
    }
}
unsafe extern "C" fn helper_fcvtas_x_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() {
        0
    } else {
        f.round() as i64 as u64
    }
}
unsafe extern "C" fn helper_fcvtas_w_d(a: u64) -> u64 {
    let f = f64::from_bits(a);
    if f.is_nan() {
        0
    } else {
        f.round() as i32 as u64
    }
}
unsafe extern "C" fn helper_frinta_d(a: u64) -> u64 {
    f64::from_bits(a).round().to_bits()
}
unsafe extern "C" fn helper_frinta_s(a: u64) -> u64 {
    (f32::from_bits(a as u32).round()).to_bits() as u64
}
unsafe extern "C" fn helper_frintm_d(a: u64) -> u64 {
    f64::from_bits(a).floor().to_bits()
}
unsafe extern "C" fn helper_frintm_s(a: u64) -> u64 {
    (f32::from_bits(a as u32).floor()).to_bits() as u64
}
unsafe extern "C" fn helper_frintp_d(a: u64) -> u64 {
    f64::from_bits(a).ceil().to_bits()
}
unsafe extern "C" fn helper_frintp_s(a: u64) -> u64 {
    (f32::from_bits(a as u32).ceil()).to_bits() as u64
}
unsafe extern "C" fn helper_frintn_d(a: u64) -> u64 {
    // Round to nearest even (banker's rounding)
    let f = f64::from_bits(a);
    let rounded = f.round();
    // Check ties (exactly 0.5): use banker's rounding
    if (f - rounded).abs() == 0.5 {
        let even = if (rounded as i64) % 2 == 0 {
            rounded
        } else {
            rounded - rounded.signum()
        };
        even.to_bits()
    } else {
        rounded.to_bits()
    }
}
unsafe extern "C" fn helper_frintn_s(a: u64) -> u64 {
    let f = f32::from_bits(a as u32);
    let rounded = f.round();
    let result = if (f - rounded).abs() == 0.5 {
        if (rounded as i32) % 2 == 0 {
            rounded
        } else {
            rounded - rounded.signum()
        }
    } else {
        rounded
    };
    result.to_bits() as u64
}
unsafe extern "C" fn helper_frintz_d(a: u64) -> u64 {
    f64::from_bits(a).trunc().to_bits()
}
unsafe extern "C" fn helper_frintz_s(a: u64) -> u64 {
    (f32::from_bits(a as u32).trunc()).to_bits() as u64
}
/// Returns NZCV bits (packed in bits 31:28) for fcmp/fcmpe.
unsafe extern "C" fn helper_fcmp64(a: u64, b: u64) -> u64 {
    let fa = f64::from_bits(a);
    let fb = f64::from_bits(b);
    let nzcv = if fa.is_nan() || fb.is_nan() {
        0b0011u64 // C=1, V=1 (unordered)
    } else if fa == fb {
        0b0110 // Z=1, C=1
    } else if fa < fb {
        0b1000 // N=1
    } else {
        0b0010 // C=1
    };
    nzcv << 28
}

// ── NEON 3-same, 2-reg-misc, shift-imm ─────────────────

impl Aarch64DisasContext {
    /// Helper: apply a per-u64-half operation on vector registers.
    fn neon_binop_halves(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
        f: fn(&mut Context, TempIdx, TempIdx) -> TempIdx,
    ) {
        let an = self.read_vreg_lo(ir, rn);
        let am = self.read_vreg_lo(ir, rm);
        let lo = f(ir, an, am);
        self.write_vreg_lo(ir, rd, lo);
        if q != 0 {
            let bn = self.read_vreg_hi(ir, rn);
            let bm = self.read_vreg_hi(ir, rm);
            let hi = f(ir, bn, bm);
            self.write_vreg_hi(ir, rd, hi);
        } else {
            self.clear_vreg_hi(ir, rd);
        }
    }

    /// Helper: call a 2-arg helper on each u64 half.
    fn neon_call2_halves(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
        helper: unsafe extern "C" fn(u64, u64) -> u64,
    ) {
        let an = self.read_vreg_lo(ir, rn);
        let am = self.read_vreg_lo(ir, rm);
        let dst_lo = ir.new_temp(Type::I64);
        ir.gen_call(dst_lo, helper as u64, &[an, am]);
        self.write_vreg_lo(ir, rd, dst_lo);
        if q != 0 {
            let bn = self.read_vreg_hi(ir, rn);
            let bm = self.read_vreg_hi(ir, rm);
            let dst_hi = ir.new_temp(Type::I64);
            ir.gen_call(dst_hi, helper as u64, &[bn, bm]);
            self.write_vreg_hi(ir, rd, dst_hi);
        } else {
            self.clear_vreg_hi(ir, rd);
        }
    }

    fn neon_call1_halves(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        helper: unsafe extern "C" fn(u64) -> u64,
    ) {
        let an = self.read_vreg_lo(ir, rn);
        let dst_lo = ir.new_temp(Type::I64);
        ir.gen_call(dst_lo, helper as u64, &[an]);
        self.write_vreg_lo(ir, rd, dst_lo);
        if q != 0 {
            let bn = self.read_vreg_hi(ir, rn);
            let dst_hi = ir.new_temp(Type::I64);
            ir.gen_call(dst_hi, helper as u64, &[bn]);
            self.write_vreg_hi(ir, rd, dst_hi);
        } else {
            self.clear_vreg_hi(ir, rd);
        }
    }

    /// AdvSIMD scalar three same: 01 U 11110 size 1 Rm opcode 1 Rn Rd
    /// Scalar 64-bit integer operations on D registers.
    fn neon_scalar_3same(&mut self, ir: &mut Context, insn: u32) -> bool {
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 3;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let opcode = (insn >> 11) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        let n = self.read_vreg_lo(ir, rn);
        let m = self.read_vreg_lo(ir, rm);
        let d = ir.new_temp(Type::I64);

        // FP scalar 3-same: size=10 (single) or size=11 (double, FP or int)
        if opcode >= 0b11000 {
            // FP ops: sz = size & 1 (0=f32, 1=f64)
            let sz = size & 1;
            let helper: u64 = match (u, opcode) {
                (0, 0b11010) => {
                    if sz == 1 {
                        helper_fadd64 as u64
                    } else {
                        helper_fadd32 as u64
                    }
                } // FADD
                (0, 0b11101) => {
                    if sz == 1 {
                        helper_fsub64 as u64
                    } else {
                        helper_fsub32 as u64
                    }
                } // FSUB
                (0, 0b11011) | (1, 0b11011) => {
                    if sz == 1 {
                        helper_fmul64 as u64
                    } else {
                        helper_fmul32 as u64
                    }
                } // FMUL
                (1, 0b11111) => {
                    if sz == 1 {
                        helper_fdiv64 as u64
                    } else {
                        helper_fdiv32 as u64
                    }
                } // FDIV
                (1, 0b11010) => {
                    if sz == 1 {
                        helper_fabd64 as u64
                    } else {
                        helper_fabd32 as u64
                    }
                } // FABD (abs diff)
                _ => return false,
            };
            ir.gen_call(d, helper, &[n, m]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        // Integer scalar 3-same: only size=11 (64-bit)
        if size != 0b11 {
            return false;
        }

        match (u, opcode) {
            (0, 0b10000) => {
                // ADD d,d,d
                ir.gen_add(Type::I64, d, n, m);
            }
            (1, 0b10000) => {
                // SUB d,d,d
                ir.gen_sub(Type::I64, d, n, m);
            }
            (1, 0b10001) => {
                // CMEQ d,d,d
                ir.gen_setcond(Type::I64, d, n, m, Cond::Eq);
                ir.gen_neg(Type::I64, d, d);
            }
            (0, 0b00110) => {
                // CMGT d,d,d (signed >)
                ir.gen_setcond(Type::I64, d, n, m, Cond::Gt);
                ir.gen_neg(Type::I64, d, d);
            }
            (1, 0b00110) => {
                // CMHI d,d,d (unsigned >)
                ir.gen_setcond(Type::I64, d, n, m, Cond::Gtu);
                ir.gen_neg(Type::I64, d, d);
            }
            (0, 0b00111) => {
                // CMGE d,d,d (signed >=)
                ir.gen_setcond(Type::I64, d, n, m, Cond::Ge);
                ir.gen_neg(Type::I64, d, d);
            }
            (1, 0b00111) => {
                // CMHS d,d,d (unsigned >=)
                ir.gen_setcond(Type::I64, d, n, m, Cond::Geu);
                ir.gen_neg(Type::I64, d, d);
            }
            _ => return false,
        }

        self.write_vreg_lo(ir, rd, d);
        self.clear_vreg_hi(ir, rd);
        true
    }

    /// AdvSIMD FP three same: bit[23]=1 means FP op, sz=bit[22] (0=f32, 1=f64)
    /// Opcodes: fadd=11010, fsub=11101, fmul=11011, fdiv=11111,
    ///          fmax=11110, fmin=11000, fmaxnm=11100, fminnm=11001,
    ///          fcmeq=11100(U=0), fcmge=11100(U=1), fcmgt=11101(U=1),
    ///          fmla=11001(U=0), fmls=11001(U=1), frsqrts=11111(U=0)
    fn neon_fp_3same(&mut self, ir: &mut Context, insn: u32, sz: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let opcode = (insn >> 11) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        if sz == 0 {
            // f32 — use vf*32 helpers that process 2x f32 per 64-bit half
            let helper: Option<u64> = match (u, opcode) {
                (0, 0b11010) => Some(helper_vfadd32 as u64), // FADD
                (0, 0b11101) => Some(helper_vfsub32 as u64), // FSUB
                (0, 0b11011) | (1, 0b11011) => Some(helper_vfmul32 as u64), // FMUL/FMULX
                (1, 0b11111) => Some(helper_vfdiv32 as u64), // FDIV
                (0, 0b11110) | (1, 0b11110) => Some(helper_vfmax32 as u64), // FMAX/FMAXNM
                (0, 0b11000) | (1, 0b11000) => Some(helper_vfmin32 as u64), // FMIN/FMINNM
                (0, 0b11100) => Some(helper_vfcmeq32 as u64), // FCMEQ
                (1, 0b11100) => Some(helper_vfcmge32 as u64), // FCMGE
                (1, 0b11101) => Some(helper_vfcmgt32 as u64), // FCMGT
                _ => None,
            };
            if let Some(h) = helper {
                self.neon_call2_halves(ir, q, rd, rn, rm, unsafe {
                    std::mem::transmute::<
                        u64,
                        unsafe extern "C" fn(u64, u64) -> u64,
                    >(h)
                });
                return true;
            }
            // FMLA: d = d + n*m
            if (u, opcode) == (0, 0b11001) {
                let d_lo = self.read_vreg_lo(ir, rd);
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                let r_lo = ir.new_temp(Type::I64);
                ir.gen_call(r_lo, helper_vfmla32 as u64, &[d_lo, n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, r_lo);
                if q != 0 {
                    let d_hi = self.read_vreg_hi(ir, rd);
                    let n_hi = self.read_vreg_hi(ir, rn);
                    let m_hi = self.read_vreg_hi(ir, rm);
                    let r_hi = ir.new_temp(Type::I64);
                    ir.gen_call(
                        r_hi,
                        helper_vfmla32 as u64,
                        &[d_hi, n_hi, m_hi],
                    );
                    self.write_vreg_hi(ir, rd, r_hi);
                }
                return true;
            }
            // FMLS: d = d - n*m
            if (u, opcode) == (1, 0b11001) {
                let d_lo = self.read_vreg_lo(ir, rd);
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                let r_lo = ir.new_temp(Type::I64);
                ir.gen_call(r_lo, helper_vfmls32 as u64, &[d_lo, n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, r_lo);
                if q != 0 {
                    let d_hi = self.read_vreg_hi(ir, rd);
                    let n_hi = self.read_vreg_hi(ir, rn);
                    let m_hi = self.read_vreg_hi(ir, rm);
                    let r_hi = ir.new_temp(Type::I64);
                    ir.gen_call(
                        r_hi,
                        helper_vfmls32 as u64,
                        &[d_hi, n_hi, m_hi],
                    );
                    self.write_vreg_hi(ir, rd, r_hi);
                }
                return true;
            }
            // FADDP .2S/.4S: U=1, size[1]=0, opcode=11010 — pairwise add adjacent f32 lanes
            // (size[1]=1 would be FABD, not FADDP)
            if (u, opcode) == (1, 0b11010) && (insn >> 23) & 1 == 0 {
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                if q != 0 {
                    // .4S: d_lo = faddp(n_lo, n_hi), d_hi = faddp(m_lo, m_hi)
                    let n_hi = self.read_vreg_hi(ir, rn);
                    let m_hi = self.read_vreg_hi(ir, rm);
                    let d_lo = ir.new_temp(Type::I64);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_faddp32 as u64, &[n_lo, n_hi]);
                    ir.gen_call(d_hi, helper_faddp32 as u64, &[m_lo, m_hi]);
                    self.write_vreg_lo(ir, rd, d_lo);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    // .2S: d_lo = faddp(n_lo, m_lo)
                    let d_lo = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_faddp32 as u64, &[n_lo, m_lo]);
                    self.write_vreg_lo(ir, rd, d_lo);
                    self.clear_vreg_hi(ir, rd);
                }
                return true;
            }
            // FABD .2S/.4S: U=1, size[1]=1, opcode=11010 — absolute difference
            if (u, opcode) == (1, 0b11010) && (insn >> 23) & 1 == 1 {
                self.neon_call2_halves(ir, q, rd, rn, rm, helper_vfabd32);
                return true;
            }
        } else {
            // f64 — sz=1, only .2D (Q=1) or .1D (Q=0)
            // Process each 64-bit lane as a scalar f64 op
            let helper64: Option<u64> = match (u, opcode) {
                (0, 0b11010) => Some(helper_fadd64 as u64), // FADD
                (0, 0b11101) => Some(helper_fsub64 as u64), // FSUB
                (0, 0b11011) | (1, 0b11011) => Some(helper_fmul64 as u64), // FMUL/FMULX
                (1, 0b11111) => Some(helper_fdiv64 as u64), // FDIV
                _ => None,
            };
            if let Some(h) = helper64 {
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, h, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let n_hi = self.read_vreg_hi(ir, rn);
                    let m_hi = self.read_vreg_hi(ir, rm);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, h, &[n_hi, m_hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                return true;
            }
            // FABD .2D: U=1, size[1]=1, opcode=11010 — absolute difference f64
            if (u, opcode) == (1, 0b11010) && (insn >> 23) & 1 == 1 {
                let n_lo = self.read_vreg_lo(ir, rn);
                let m_lo = self.read_vreg_lo(ir, rm);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_fabd64 as u64, &[n_lo, m_lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let n_hi = self.read_vreg_hi(ir, rn);
                    let m_hi = self.read_vreg_hi(ir, rm);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_fabd64 as u64, &[n_hi, m_hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                return true;
            }
        }
        false
    }

    /// AdvSIMD three same: 0 Q U 01110 size 1 Rm opcode 1 Rn Rd
    fn neon_3same(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 0x3;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let opcode = (insn >> 11) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // Bitwise ops (size encodes sub-op, not element size)
        if opcode == 0b00011 {
            return match (u, size) {
                (0, 0b00) => {
                    // AND
                    self.neon_binop_halves(ir, q, rd, rn, rm, |ir, a, b| {
                        let d = ir.new_temp(Type::I64);
                        ir.gen_and(Type::I64, d, a, b);
                        d
                    });
                    true
                }
                (0, 0b01) => {
                    // BIC
                    self.neon_binop_halves(ir, q, rd, rn, rm, |ir, a, b| {
                        let nb = ir.new_temp(Type::I64);
                        ir.gen_not(Type::I64, nb, b);
                        let d = ir.new_temp(Type::I64);
                        ir.gen_and(Type::I64, d, a, nb);
                        d
                    });
                    true
                }
                (0, 0b10) => {
                    // ORR
                    self.neon_binop_halves(ir, q, rd, rn, rm, |ir, a, b| {
                        let d = ir.new_temp(Type::I64);
                        ir.gen_or(Type::I64, d, a, b);
                        d
                    });
                    true
                }
                (0, 0b11) => {
                    // ORN
                    self.neon_binop_halves(ir, q, rd, rn, rm, |ir, a, b| {
                        let nb = ir.new_temp(Type::I64);
                        ir.gen_not(Type::I64, nb, b);
                        let d = ir.new_temp(Type::I64);
                        ir.gen_or(Type::I64, d, a, nb);
                        d
                    });
                    true
                }
                (1, 0b00) => {
                    // EOR
                    self.neon_binop_halves(ir, q, rd, rn, rm, |ir, a, b| {
                        let d = ir.new_temp(Type::I64);
                        ir.gen_xor(Type::I64, d, a, b);
                        d
                    });
                    true
                }
                (1, 0b01) => {
                    self.neon_bsl(ir, q, rd, rn, rm);
                    true
                } // BSL
                (1, 0b10) => {
                    self.neon_bit(ir, q, rd, rn, rm);
                    true
                } // BIT
                (1, 0b11) => {
                    self.neon_bif(ir, q, rd, rn, rm);
                    true
                } // BIF
                _ => false,
            };
        }

        // Byte-level ops (size=00)
        if size == 0b00 {
            return match (u, opcode) {
                (0, 0b10000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_add8);
                    true
                }
                (1, 0b10000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_sub8);
                    true
                }
                (1, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmeq8);
                    true
                }
                (0, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmtst8);
                    true
                } // CMTST
                (1, 0b00111) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmhs8);
                    true
                }
                (1, 0b00110) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmhi8);
                    true
                } // CMHI .8B/.16B
                (0, 0b00110) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmgt8);
                    true
                } // CMGT .8B/.16B
                (1, 0b10100) => {
                    self.neon_pairwise(ir, q, rd, rn, rm, helper_umaxp8);
                    true
                }
                (1, 0b10101) => {
                    self.neon_pairwise(ir, q, rd, rn, rm, helper_uminp8);
                    true
                }
                (0, 0b10111) => {
                    self.neon_pairwise(ir, q, rd, rn, rm, helper_addp8);
                    true
                }
                _ => false,
            };
        }
        // 16-bit element ops (size=01)
        if size == 0b01 {
            return match (u, opcode) {
                (1, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmeq16);
                    true
                } // CMEQ .8H/.4H
                (0, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmtst16);
                    true
                } // CMTST .8H/.4H
                _ => false,
            };
        }
        // 32-bit element ops (size=10)
        if size == 0b10 {
            return match (u, opcode) {
                (0, 0b10000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_add32);
                    true
                }
                (1, 0b10000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_sub32);
                    true
                }
                (0, 0b10011) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_mul32);
                    true
                }
                (0, 0b01100) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_smax32);
                    true
                } // SMAX
                (0, 0b01101) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_smin32);
                    true
                } // SMIN
                (1, 0b01100) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_umax32);
                    true
                } // UMAX
                (1, 0b01101) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_umin32);
                    true
                } // UMIN
                (1, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmeq32);
                    true
                } // CMEQ
                (1, 0b00111) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmhs32);
                    true
                } // CMHS
                (0, 0b00110) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmgt32);
                    true
                } // CMGT
                (0, 0b00111) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmge32);
                    true
                } // CMGE
                (0, 0b10010) => {
                    // MLA .4S/.2S
                    let d_lo = self.read_vreg_lo(ir, rd);
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let r_lo = ir.new_temp(Type::I64);
                    ir.gen_call(r_lo, helper_mla32 as u64, &[d_lo, n_lo, m_lo]);
                    self.write_vreg_lo(ir, rd, r_lo);
                    if q != 0 {
                        let d_hi = self.read_vreg_hi(ir, rd);
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let r_hi = ir.new_temp(Type::I64);
                        ir.gen_call(
                            r_hi,
                            helper_mla32 as u64,
                            &[d_hi, n_hi, m_hi],
                        );
                        self.write_vreg_hi(ir, rd, r_hi);
                    }
                    true
                }
                (1, 0b10010) => {
                    // MLS .4S/.2S
                    let d_lo = self.read_vreg_lo(ir, rd);
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let r_lo = ir.new_temp(Type::I64);
                    ir.gen_call(r_lo, helper_mls32 as u64, &[d_lo, n_lo, m_lo]);
                    self.write_vreg_lo(ir, rd, r_lo);
                    if q != 0 {
                        let d_hi = self.read_vreg_hi(ir, rd);
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let r_hi = ir.new_temp(Type::I64);
                        ir.gen_call(
                            r_hi,
                            helper_mls32 as u64,
                            &[d_hi, n_hi, m_hi],
                        );
                        self.write_vreg_hi(ir, rd, r_hi);
                    }
                    true
                }
                (0, 0b10111) => {
                    self.neon_pairwise(ir, q, rd, rn, rm, helper_addp32);
                    true
                } // ADDP .4S/.2S
                (0, 0b10001) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmtst32);
                    true
                } // CMTST .4S/.2S
                (1, 0b01000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_ushl32);
                    true
                } // USHL .2S/.4S
                (0, 0b01000) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_sshl32);
                    true
                } // SSHL .2S/.4S
                _ => false,
            };
        }
        // 64-bit element ops (size=11, .2D / .1D)
        if size == 0b11 {
            return match (u, opcode) {
                (0, 0b10000) | (1, 0b10000) => {
                    // ADD/SUB .2D
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let d_lo = ir.new_temp(Type::I64);
                    if u == 0 {
                        ir.gen_add(Type::I64, d_lo, n_lo, m_lo);
                    } else {
                        ir.gen_sub(Type::I64, d_lo, n_lo, m_lo);
                    }
                    self.write_vreg_lo(ir, rd, d_lo);
                    if q != 0 {
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let d_hi = ir.new_temp(Type::I64);
                        if u == 0 {
                            ir.gen_add(Type::I64, d_hi, n_hi, m_hi);
                        } else {
                            ir.gen_sub(Type::I64, d_hi, n_hi, m_hi);
                        }
                        self.write_vreg_hi(ir, rd, d_hi);
                    } else {
                        self.clear_vreg_hi(ir, rd);
                    }
                    true
                }
                (1, 0b10001) => {
                    // CMEQ .2D
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let d_lo = ir.new_temp(Type::I64);
                    ir.gen_setcond(Type::I64, d_lo, n_lo, m_lo, Cond::Eq);
                    ir.gen_neg(Type::I64, d_lo, d_lo);
                    self.write_vreg_lo(ir, rd, d_lo);
                    if q != 0 {
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let d_hi = ir.new_temp(Type::I64);
                        ir.gen_setcond(Type::I64, d_hi, n_hi, m_hi, Cond::Eq);
                        ir.gen_neg(Type::I64, d_hi, d_hi);
                        self.write_vreg_hi(ir, rd, d_hi);
                    } else {
                        self.clear_vreg_hi(ir, rd);
                    }
                    true
                }
                (0, 0b00110) => {
                    self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmgt64);
                    true
                } // CMGT .1D/.2D
                (0, 0b10001) => {
                    // CMTST .2D: if (a & b) != 0 then -1 else 0 per lane
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let t_lo = ir.new_temp(Type::I64);
                    ir.gen_and(Type::I64, t_lo, n_lo, m_lo);
                    let zero = ir.new_const(Type::I64, 0);
                    let all_ones = ir.new_const(Type::I64, !0u64);
                    let zero2 = ir.new_const(Type::I64, 0);
                    let d_lo = ir.new_temp(Type::I64);
                    ir.gen_movcond(
                        Type::I64,
                        d_lo,
                        t_lo,
                        zero,
                        all_ones,
                        zero2,
                        Cond::Ne,
                    );
                    self.write_vreg_lo(ir, rd, d_lo);
                    if q != 0 {
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let t_hi = ir.new_temp(Type::I64);
                        ir.gen_and(Type::I64, t_hi, n_hi, m_hi);
                        let zero3 = ir.new_const(Type::I64, 0);
                        let all_ones2 = ir.new_const(Type::I64, !0u64);
                        let zero4 = ir.new_const(Type::I64, 0);
                        let d_hi = ir.new_temp(Type::I64);
                        ir.gen_movcond(
                            Type::I64,
                            d_hi,
                            t_hi,
                            zero3,
                            all_ones2,
                            zero4,
                            Cond::Ne,
                        );
                        self.write_vreg_hi(ir, rd, d_hi);
                    } else {
                        self.clear_vreg_hi(ir, rd);
                    }
                    true
                }
                (1, 0b01000) => {
                    // USHL .1D/.2D
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let d_lo = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_ushl64 as u64, &[n_lo, m_lo]);
                    self.write_vreg_lo(ir, rd, d_lo);
                    if q != 0 {
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let d_hi = ir.new_temp(Type::I64);
                        ir.gen_call(
                            d_hi,
                            helper_ushl64 as u64,
                            &[n_hi, m_hi],
                        );
                        self.write_vreg_hi(ir, rd, d_hi);
                    } else {
                        self.clear_vreg_hi(ir, rd);
                    }
                    true
                }
                (0, 0b01000) => {
                    // SSHL .1D/.2D
                    let n_lo = self.read_vreg_lo(ir, rn);
                    let m_lo = self.read_vreg_lo(ir, rm);
                    let d_lo = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_sshl64 as u64, &[n_lo, m_lo]);
                    self.write_vreg_lo(ir, rd, d_lo);
                    if q != 0 {
                        let n_hi = self.read_vreg_hi(ir, rn);
                        let m_hi = self.read_vreg_hi(ir, rm);
                        let d_hi = ir.new_temp(Type::I64);
                        ir.gen_call(
                            d_hi,
                            helper_sshl64 as u64,
                            &[n_hi, m_hi],
                        );
                        self.write_vreg_hi(ir, rd, d_hi);
                    } else {
                        self.clear_vreg_hi(ir, rd);
                    }
                    true
                }
                _ => false,
            };
        }
        false
    }

    // BIT/BIF/BSL helpers and pairwise, 2-reg-misc, shift-imm, across, EXT

    fn neon_bit(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
    ) {
        let vd = self.read_vreg_lo(ir, rd);
        let vn = self.read_vreg_lo(ir, rn);
        let vm = self.read_vreg_lo(ir, rm);
        let r = ir.new_temp(Type::I64);
        ir.gen_call(r, helper_bit as u64, &[vd, vn, vm]);
        self.write_vreg_lo(ir, rd, r);
        if q != 0 {
            let vd = self.read_vreg_hi(ir, rd);
            let vn = self.read_vreg_hi(ir, rn);
            let vm = self.read_vreg_hi(ir, rm);
            let r = ir.new_temp(Type::I64);
            ir.gen_call(r, helper_bit as u64, &[vd, vn, vm]);
            self.write_vreg_hi(ir, rd, r);
        }
    }

    fn neon_bif(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
    ) {
        let vd = self.read_vreg_lo(ir, rd);
        let vn = self.read_vreg_lo(ir, rn);
        let vm = self.read_vreg_lo(ir, rm);
        let r = ir.new_temp(Type::I64);
        ir.gen_call(r, helper_bif as u64, &[vd, vn, vm]);
        self.write_vreg_lo(ir, rd, r);
        if q != 0 {
            let vd = self.read_vreg_hi(ir, rd);
            let vn = self.read_vreg_hi(ir, rn);
            let vm = self.read_vreg_hi(ir, rm);
            let r = ir.new_temp(Type::I64);
            ir.gen_call(r, helper_bif as u64, &[vd, vn, vm]);
            self.write_vreg_hi(ir, rd, r);
        }
    }

    fn neon_bsl(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
    ) {
        let vd = self.read_vreg_lo(ir, rd);
        let vn = self.read_vreg_lo(ir, rn);
        let vm = self.read_vreg_lo(ir, rm);
        let r = ir.new_temp(Type::I64);
        ir.gen_call(r, helper_bsl as u64, &[vd, vn, vm]);
        self.write_vreg_lo(ir, rd, r);
        if q != 0 {
            let vd = self.read_vreg_hi(ir, rd);
            let vn = self.read_vreg_hi(ir, rn);
            let vm = self.read_vreg_hi(ir, rm);
            let r = ir.new_temp(Type::I64);
            ir.gen_call(r, helper_bsl as u64, &[vd, vn, vm]);
            self.write_vreg_hi(ir, rd, r);
        }
    }

    fn neon_pairwise(
        &mut self,
        ir: &mut Context,
        q: u32,
        rd: usize,
        rn: usize,
        rm: usize,
        helper: unsafe extern "C" fn(u64, u64) -> u64,
    ) {
        if q != 0 {
            let n_lo = self.read_vreg_lo(ir, rn);
            let n_hi = self.read_vreg_hi(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper as u64, &[n_lo, n_hi]);
            let m_lo = self.read_vreg_lo(ir, rm);
            let m_hi = self.read_vreg_hi(ir, rm);
            let d_hi = ir.new_temp(Type::I64);
            ir.gen_call(d_hi, helper as u64, &[m_lo, m_hi]);
            self.write_vreg_lo(ir, rd, d_lo);
            self.write_vreg_hi(ir, rd, d_hi);
        } else {
            let n_lo = self.read_vreg_lo(ir, rn);
            let m_lo = self.read_vreg_lo(ir, rm);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper as u64, &[n_lo, m_lo]);
            self.write_vreg_lo(ir, rd, d_lo);
            self.clear_vreg_hi(ir, rd);
        }
    }

    /// AdvSIMD two-reg misc: 0 Q U 01110 size 10000 opcode 10 Rn Rd
    fn neon_2reg_misc(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 0x3;
        let opcode = (insn >> 12) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        match (u, size, opcode) {
            // CMEQ #0 .8B/.16B: U=0 size=00 opcode=01001
            (0, 0b00, 0b01001) => {
                let zero = ir.new_const(Type::I64, 0);
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_cmeq8 as u64, &[lo, zero]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_cmeq8 as u64, &[hi, zero]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // CNT .8B/.16B: U=0 size=00 opcode=00101
            (0, 0b00, 0b00101) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_cnt8 as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_cnt8 as u64, &[hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // REV64 .2S/.4S: U=0 size=10 opcode=00000 — swap two 32-bit words in each 64-bit lane
            (0, 0b10, 0b00000) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_rev64_2s as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_rev64_2s as u64, &[hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // REV64 .8B/.16B: U=0 size=00 opcode=00000
            (0, 0b00, 0b00000) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_bswap64(Type::I64, d_lo, lo, 0);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_bswap64(Type::I64, d_hi, hi, 0);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // XTN .8B: U=0 size=00 opcode=10010 (narrow)
            (0, 0b00, 0b10010) => {
                // Narrow 8x16→8x8: take low byte of each 16-bit element
                let lo = self.read_vreg_lo(ir, rn);
                let d = ir.new_temp(Type::I64);
                ir.gen_call(d, helper_xtn8 as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d);
                self.clear_vreg_hi(ir, rd);
                true
            }
            // XTN .4H: U=0 size=01 opcode=10010 (narrow 32→16)
            // XTN2 .8H: Q=1 version appends to high half
            (0, 0b01, 0b10010) => {
                // Narrow 4x32-bit to 4x16-bit (XTN) or append to high (XTN2)
                if q == 0 {
                    // XTN: process lo half (2x32→2x16) and hi half (2x32→2x16), pack into lo 32 bits
                    let src_lo = self.read_vreg_lo(ir, rn);
                    let src_hi = self.read_vreg_hi(ir, rn);
                    let d_lo = ir.new_temp(Type::I64);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_xtn16 as u64, &[src_lo]);
                    ir.gen_call(d_hi, helper_xtn16 as u64, &[src_hi]);
                    // Pack: low 16 bits from each half into a single 64-bit value (4x16)
                    let sh = ir.new_const(Type::I64, 32);
                    let d_hi_shifted = ir.new_temp(Type::I64);
                    ir.gen_shl(Type::I64, d_hi_shifted, d_hi, sh);
                    let d = ir.new_temp(Type::I64);
                    ir.gen_or(Type::I64, d, d_lo, d_hi_shifted);
                    self.write_vreg_lo(ir, rd, d);
                    self.clear_vreg_hi(ir, rd);
                } else {
                    // XTN2: write narrowed result to upper half of Vd
                    let src_lo = self.read_vreg_lo(ir, rn);
                    let src_hi = self.read_vreg_hi(ir, rn);
                    let d_lo = ir.new_temp(Type::I64);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_lo, helper_xtn16 as u64, &[src_lo]);
                    ir.gen_call(d_hi, helper_xtn16 as u64, &[src_hi]);
                    let sh = ir.new_const(Type::I64, 32);
                    let d_hi_shifted = ir.new_temp(Type::I64);
                    ir.gen_shl(Type::I64, d_hi_shifted, d_hi, sh);
                    let d = ir.new_temp(Type::I64);
                    ir.gen_or(Type::I64, d, d_lo, d_hi_shifted);
                    self.write_vreg_hi(ir, rd, d);
                }
                true
            }
            // ABS .4S/.2S: U=0 size=10 opcode=01011
            (0, 0b10, 0b01011) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_abs32);
                true
            }
            // CMEQ #0 .4S/.2S: U=0 size=10 opcode=01001
            (0, 0b10, 0b01001) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_cmeq32_zero);
                true
            }
            // CMEQ #0 .8H/.4H: U=0 size=01 opcode=01001
            (0, 0b01, 0b01001) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_cmeq16_zero);
                true
            }
            // XTN .2S (64→32): U=0 size=10 opcode=10010
            (0, 0b10, 0b10010) => {
                let lo = self.read_vreg_lo(ir, rn);
                let hi = self.read_vreg_hi(ir, rn);
                let d = ir.new_temp(Type::I64);
                ir.gen_call(d, helper_xtn32 as u64, &[lo, hi]);
                self.write_vreg_lo(ir, rd, d);
                self.clear_vreg_hi(ir, rd);
                true
            }
            // REV64 .4H/.8H: U=0 size=01 opcode=00000
            (0, 0b01, 0b00000) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_rev64_16);
                true
            }
            // CMLT #0 .4S/.2S: U=0 size=10 opcode=01010
            (0, 0b10, 0b01010) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_cmlt32_zero);
                true
            }
            // SHLL/SHLL2 .4S, .4H/.8H, #16: U=1 size=01 opcode=10011
            (1, 0b01, 0b10011) => {
                // Q=0 (shll): widen low 4×16-bit → 4×32-bit shifted left 16
                // Q=1 (shll2): widen high 4×16-bit → 4×32-bit shifted left 16
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let sh = ir.new_const(Type::I64, 16);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_ushll16 as u64, &[src, sh]);
                // High pair: shift src right by 32 to get elements 2-3
                let c32 = ir.new_const(Type::I64, 32);
                let src_hi = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, src_hi, src, c32);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushll16 as u64, &[src_hi, sh]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // SHLL/SHLL2 .8H, .8B/.16B, #8: U=1 size=00 opcode=10011
            (1, 0b00, 0b10011) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let sh = ir.new_const(Type::I64, 8);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_ushll8 as u64, &[src, sh]);
                let c32 = ir.new_const(Type::I64, 32);
                let src_hi = ir.new_temp(Type::I64);
                ir.gen_shr(Type::I64, src_hi, src, c32);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushll8 as u64, &[src_hi, sh]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // CMEQ #0 .2D: U=0 size=11 opcode=01001
            (0, 0b11, 0b01001) => {
                // Each 64-bit lane == 0 → all-ones, else all-zeros
                let lo = self.read_vreg_lo(ir, rn);
                let zero = ir.new_const(Type::I64, 0);
                let all_ones = ir.new_const(Type::I64, !0u64);
                let zero2 = ir.new_const(Type::I64, 0);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_movcond(
                    Type::I64,
                    d_lo,
                    lo,
                    zero,
                    all_ones,
                    zero2,
                    Cond::Eq,
                );
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let zero3 = ir.new_const(Type::I64, 0);
                    let all_ones2 = ir.new_const(Type::I64, !0u64);
                    let zero4 = ir.new_const(Type::I64, 0);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_movcond(
                        Type::I64,
                        d_hi,
                        hi,
                        zero3,
                        all_ones2,
                        zero4,
                        Cond::Eq,
                    );
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // FCVTL .2D, .2S: U=0 size=01 opcode=10111 (Q=0: low 2x f32 → 2x f64)
            // FCVTL2 .2D, .4S: Q=1 reads the high half
            (0, 0b01, 0b10111) => {
                let src = if q == 0 {
                    self.read_vreg_lo(ir, rn)
                } else {
                    self.read_vreg_hi(ir, rn)
                };
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_fcvtl2_lo as u64, &[src]);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_fcvtl2_hi as u64, &[src]);
                self.write_vreg_lo(ir, rd, d_lo);
                self.write_vreg_hi(ir, rd, d_hi);
                true
            }
            // NEG .2S/.4S: U=1 size=10 opcode=01011
            (1, 0b10, 0b01011) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_neg32);
                true
            }
            // MVN/NOT .8B/.16B: U=1 size=00 opcode=00101
            (1, 0b00, 0b00101) => {
                self.neon_binop_halves(ir, q, rd, rn, rn, |ir, a, _b| {
                    let d = ir.new_temp(Type::I64);
                    ir.gen_not(Type::I64, d, a);
                    d
                });
                true
            }
            // NEG .8B/.16B: U=1 size=00 opcode=01011
            (1, 0b00, 0b01011) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_neg8);
                true
            }
            // NEG .4H/.8H: U=1 size=01 opcode=01011
            (1, 0b01, 0b01011) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_neg16);
                true
            }
            // NEG .2D: U=1 size=11 opcode=01011
            (1, 0b11, 0b01011) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_neg(Type::I64, d_lo, lo);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_neg(Type::I64, d_hi, hi);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // FCVTZS .2S/.4S: U=0 size=10 opcode=11011 — vector float-to-signed-int (round toward zero)
            (0, 0b10, 0b11011) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_vfcvtzs32);
                true
            }
            // SCVTF .2S/.4S: U=0 size=00 opcode=11101 — vector signed-int-to-float (32-bit)
            (0, 0b00, 0b11101) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_vscvtf32);
                true
            }
            // UCVTF .2S/.4S: U=1 size=00 opcode=11101 — vector unsigned-int-to-float (32-bit)
            (1, 0b00, 0b11101) => {
                self.neon_call1_halves(ir, q, rd, rn, helper_vucvtf32);
                true
            }
            // SCVTF .2D/.4D: U=0 size=01 opcode=11101 — vector signed-int-to-float (64-bit)
            // Each 64-bit lane holds one i64 → f64
            (0, 0b01, 0b11101) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_vscvtf64 as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_vscvtf64 as u64, &[hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // UCVTF .2D/.4D: U=1 size=01 opcode=11101 — vector unsigned-int-to-float (64-bit)
            // Each 64-bit lane holds one u64 → f64
            (1, 0b01, 0b11101) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_vucvtf64 as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_vucvtf64 as u64, &[hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            // FCVTZS .2D: U=0 size=11 opcode=11011 — vector f64-to-i64 (round toward zero)
            (0, 0b11, 0b11011) => {
                let lo = self.read_vreg_lo(ir, rn);
                let d_lo = ir.new_temp(Type::I64);
                ir.gen_call(d_lo, helper_vfcvtzs64 as u64, &[lo]);
                self.write_vreg_lo(ir, rd, d_lo);
                if q != 0 {
                    let hi = self.read_vreg_hi(ir, rn);
                    let d_hi = ir.new_temp(Type::I64);
                    ir.gen_call(d_hi, helper_vfcvtzs64 as u64, &[hi]);
                    self.write_vreg_hi(ir, rd, d_hi);
                } else {
                    self.clear_vreg_hi(ir, rd);
                }
                true
            }
            _ => false,
        }
    }
    fn neon_shift_imm(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let immh = (insn >> 19) & 0xf;
        let immb = (insn >> 16) & 0x7;
        let opcode = (insn >> 11) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;
        let immhb = (immh << 3) | immb;

        // SHRN: U=0 opcode=10000, narrow shift right (16→8)
        // Narrows 8x16-bit (128-bit src) → 8x8-bit (64-bit dst)
        if u == 0 && opcode == 0b10000 && (1..2).contains(&immh) {
            let shift = 16 - immhb;
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let sh = ir.new_const(Type::I64, shift as u64);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_shrn8 as u64, &[lo, sh]);
            let d_hi = ir.new_temp(Type::I64);
            ir.gen_call(d_hi, helper_shrn8 as u64, &[hi, sh]);
            // Combine: lo gives bytes 0-3, hi gives bytes 4-7
            let c32 = ir.new_const(Type::I64, 32);
            let hi_shifted = ir.new_temp(Type::I64);
            ir.gen_shl(Type::I64, hi_shifted, d_hi, c32);
            let d = ir.new_temp(Type::I64);
            ir.gen_or(Type::I64, d, d_lo, hi_shifted);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        // SHL: U=0 opcode=01010 (8-bit)
        if u == 0 && opcode == 0b01010 && (1..2).contains(&immh) {
            let shift = immhb - 8;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_shl8 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_shl8 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SHL: U=0 opcode=01010 (16-bit): immh=001x
        if u == 0 && opcode == 0b01010 && (2..4).contains(&immh) {
            let shift = immhb - 16;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_shl16 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_shl16 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SHL: U=0 opcode=01010 (32-bit): immh=01xx
        if u == 0 && opcode == 0b01010 && (4..8).contains(&immh) {
            let shift = immhb - 32;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_shl32 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_shl32 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SHL: U=0 opcode=01010 (64-bit): immh=1xxx → shift = immhb - 64
        if u == 0 && opcode == 0b01010 && immh >= 8 {
            let shift = immhb - 64;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_shl64 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_shl64 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // USHR 32-bit: U=1 opcode=00000, immh=01xx → shift = 2*32 - immhb = 64 - immhb
        if u == 1 && opcode == 0b00000 && (4..8).contains(&immh) {
            let shift = 64 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_ushr32 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushr32 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // USHR 64-bit: U=1 opcode=00000, immh=1xxx → shift = 128 - immhb
        if u == 1 && opcode == 0b00000 && immh >= 8 {
            let shift = 128 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_ushr64 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushr64 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // USHR 16-bit: U=1 opcode=00000, immh=001x → shift = 32 - immhb
        if u == 1 && opcode == 0b00000 && (2..4).contains(&immh) {
            let shift = 32 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_ushr16 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushr16 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // USHR 8-bit: U=1 opcode=00000, immh=0001 → shift = 16 - immhb
        if u == 1 && opcode == 0b00000 && (1..2).contains(&immh) {
            let shift = 16 - immhb;
            // reuse helper_sshr8 with logical mask
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_ushr8 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_ushr8 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SSHR 32-bit: U=0 opcode=00000, immh=01xx → shift = 64 - immhb
        if u == 0 && opcode == 0b00000 && (4..8).contains(&immh) {
            let shift = 64 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_sshr32 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_sshr32 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SSRA 32-bit: U=0 opcode=00010, immh=01xx → shift = 64 - immhb
        if u == 0 && opcode == 0b00010 && (4..8).contains(&immh) {
            let shift = 64 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let d_lo = self.read_vreg_lo(ir, rd);
            let n_lo = self.read_vreg_lo(ir, rn);
            let r_lo = ir.new_temp(Type::I64);
            ir.gen_call(r_lo, helper_ssra32 as u64, &[d_lo, n_lo, sh]);
            self.write_vreg_lo(ir, rd, r_lo);
            if q != 0 {
                let d_hi = self.read_vreg_hi(ir, rd);
                let n_hi = self.read_vreg_hi(ir, rn);
                let r_hi = ir.new_temp(Type::I64);
                ir.gen_call(r_hi, helper_ssra32 as u64, &[d_hi, n_hi, sh]);
                self.write_vreg_hi(ir, rd, r_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SSHR 16-bit: U=0 opcode=00000, immh=001x → shift = 32 - immhb
        if u == 0 && opcode == 0b00000 && (2..4).contains(&immh) {
            let shift = 32 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_sshr16 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_sshr16 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // SSHR 8-bit: U=0 opcode=00000, immh=0001 → shift = 16 - immhb
        if u == 0 && opcode == 0b00000 && (1..2).contains(&immh) {
            let shift = 16 - immhb;
            let sh = ir.new_const(Type::I64, shift as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_sshr8 as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_sshr8 as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        // USHLL/UXTL: U=1 opcode=10100, immh=0001 → 8→16 bit
        if u == 1 && opcode == 0b10100 && (1..2).contains(&immh) {
            let shift = immhb - 8;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let d_lo = ir.new_temp(Type::I64);
            let shift_c = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_lo, helper_ushll8 as u64, &[src, shift_c]);
            self.write_vreg_lo(ir, rd, d_lo);
            let c32 = ir.new_const(Type::I64, 32);
            let src_hi = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, src_hi, src, c32);
            let d_hi = ir.new_temp(Type::I64);
            let shift_c2 = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_hi, helper_ushll8 as u64, &[src_hi, shift_c2]);
            self.write_vreg_hi(ir, rd, d_hi);
            return true;
        }

        // USHLL/UXTL: U=1 opcode=10100, immh=001x → 16→32 bit
        if u == 1 && opcode == 0b10100 && (2..4).contains(&immh) {
            let shift = immhb - 16;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let d_lo = ir.new_temp(Type::I64);
            let shift_c = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_lo, helper_ushll16 as u64, &[src, shift_c]);
            self.write_vreg_lo(ir, rd, d_lo);
            let c32 = ir.new_const(Type::I64, 32);
            let src_hi = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, src_hi, src, c32);
            let d_hi = ir.new_temp(Type::I64);
            let shift_c2 = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_hi, helper_ushll16 as u64, &[src_hi, shift_c2]);
            self.write_vreg_hi(ir, rd, d_hi);
            return true;
        }

        // USHLL/UXTL: U=1 opcode=10100, widen unsigned (e.g. 32→64)
        // immh=01xx → 32→64 bit, shift = immhb - 32
        if u == 1 && opcode == 0b10100 && (4..8).contains(&immh) {
            let shift = immhb - 32;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            // Low 32 bits → element 0 of result (lo half)
            let mask32 = ir.new_const(Type::I64, 0xffff_ffff);
            let e0 = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, e0, src, mask32);
            if shift > 0 {
                let sh = ir.new_const(Type::I64, shift as u64);
                ir.gen_shl(Type::I64, e0, e0, sh);
            }
            self.write_vreg_lo(ir, rd, e0);
            // High 32 bits → element 1 of result (hi half)
            let c32 = ir.new_const(Type::I64, 32);
            let e1 = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, e1, src, c32);
            if shift > 0 {
                let sh = ir.new_const(Type::I64, shift as u64);
                ir.gen_shl(Type::I64, e1, e1, sh);
            }
            self.write_vreg_hi(ir, rd, e1);
            return true;
        }

        // SSHLL/SXTL: U=0 opcode=10100, widen signed
        // immh=0001 → 8→16 bit, shift = immhb - 8
        if u == 0 && opcode == 0b10100 && (1..2).contains(&immh) {
            let shift = immhb - 8;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let d = ir.new_temp(Type::I64);
            let shift_c = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d, helper_sshll8 as u64, &[src, shift_c]);
            self.write_vreg_lo(ir, rd, d);
            let d_hi = ir.new_temp(Type::I64);
            let c32 = ir.new_const(Type::I64, 32);
            let src_hi = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, src_hi, src, c32);
            let shift_c2 = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_hi, helper_sshll8 as u64, &[src_hi, shift_c2]);
            self.write_vreg_hi(ir, rd, d_hi);
            return true;
        }

        // SSHLL/SXTL: U=0 opcode=10100, immh=001x → 16→32 bit
        if u == 0 && opcode == 0b10100 && (2..4).contains(&immh) {
            let shift = immhb - 16;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            let d_lo = ir.new_temp(Type::I64);
            let shift_c = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_lo, helper_sshll16 as u64, &[src, shift_c]);
            self.write_vreg_lo(ir, rd, d_lo);
            let c32 = ir.new_const(Type::I64, 32);
            let src_hi = ir.new_temp(Type::I64);
            ir.gen_shr(Type::I64, src_hi, src, c32);
            let d_hi = ir.new_temp(Type::I64);
            let shift_c2 = ir.new_const(Type::I64, shift as u64);
            ir.gen_call(d_hi, helper_sshll16 as u64, &[src_hi, shift_c2]);
            self.write_vreg_hi(ir, rd, d_hi);
            return true;
        }

        // SSHLL/SXTL: U=0 opcode=10100, immh=01xx → 32→64 bit
        if u == 0 && opcode == 0b10100 && (4..8).contains(&immh) {
            let shift = immhb - 32;
            let src = if q == 0 {
                self.read_vreg_lo(ir, rn)
            } else {
                self.read_vreg_hi(ir, rn)
            };
            // Low 32 bits → sign-extend to 64, shift left
            let mask32 = ir.new_const(Type::I64, 0xffff_ffff);
            let e0 = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, e0, src, mask32);
            // Sign-extend: shl 32, then sar 32
            let c32 = ir.new_const(Type::I64, 32);
            ir.gen_shl(Type::I64, e0, e0, c32);
            ir.gen_sar(Type::I64, e0, e0, c32);
            if shift > 0 {
                let sh = ir.new_const(Type::I64, shift as u64);
                ir.gen_shl(Type::I64, e0, e0, sh);
            }
            self.write_vreg_lo(ir, rd, e0);
            // High 32 bits → sign-extend to 64, shift left
            let e1 = ir.new_temp(Type::I64);
            let c32b = ir.new_const(Type::I64, 32);
            ir.gen_sar(Type::I64, e1, src, c32b);
            if shift > 0 {
                let sh = ir.new_const(Type::I64, shift as u64);
                ir.gen_shl(Type::I64, e1, e1, sh);
            }
            self.write_vreg_hi(ir, rd, e1);
            return true;
        }

        // FCVTZS (vector, fixed-point) 32-bit: U=0 opcode=11111, immh=01xx
        if u == 0 && opcode == 0b11111 && (4..8).contains(&immh) {
            let fbits = 64 - immhb; // number of fractional bits
            let sh = ir.new_const(Type::I64, fbits as u64);
            let lo = self.read_vreg_lo(ir, rn);
            let d_lo = ir.new_temp(Type::I64);
            ir.gen_call(d_lo, helper_vfcvtzs32_fixedpt as u64, &[lo, sh]);
            self.write_vreg_lo(ir, rd, d_lo);
            if q != 0 {
                let hi = self.read_vreg_hi(ir, rn);
                let d_hi = ir.new_temp(Type::I64);
                ir.gen_call(d_hi, helper_vfcvtzs32_fixedpt as u64, &[hi, sh]);
                self.write_vreg_hi(ir, rd, d_hi);
            } else {
                self.clear_vreg_hi(ir, rd);
            }
            return true;
        }

        false
    }

    /// AdvSIMD across lanes: 0 Q U 01110 size 11000 opcode 10 Rn Rd
    fn neon_across_lanes(&mut self, ir: &mut Context, insn: u32) -> bool {
        let q = (insn >> 30) & 1;
        let u = (insn >> 29) & 1;
        let size = (insn >> 22) & 0x3;
        let opcode = (insn >> 12) & 0x1f;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        // ADDV .16B: U=0 size=00 opcode=11011
        if u == 0 && size == 0b00 && opcode == 0b11011 && q != 0 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let sum_lo = ir.new_temp(Type::I64);
            ir.gen_call(sum_lo, helper_addv8 as u64, &[lo]);
            let sum_hi = ir.new_temp(Type::I64);
            ir.gen_call(sum_hi, helper_addv8 as u64, &[hi]);
            let total = ir.new_temp(Type::I64);
            ir.gen_add(Type::I64, total, sum_lo, sum_hi);
            let mask = ir.new_const(Type::I64, 0xff);
            let result = ir.new_temp(Type::I64);
            ir.gen_and(Type::I64, result, total, mask);
            self.write_vreg_lo(ir, rd, result);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        // SMAXV .4S: U=0 size=10 opcode=01010, Q must be 1
        if u == 0 && size == 0b10 && opcode == 0b01010 && q != 0 {
            // max of 4x32-bit elements across lo and hi halves
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_smax32 as u64, &[lo, hi]);
            // now reduce: max of the 2 elements in t
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_smaxv32_reduce as u64, &[t]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // SMINV .4S: U=0 size=10 opcode=11010, Q must be 1
        if u == 0 && size == 0b10 && opcode == 0b11010 && q != 0 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_smin32 as u64, &[lo, hi]);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_sminv32_reduce as u64, &[t]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // UMAXV .4S: U=1 size=10 opcode=01010
        if u == 1 && size == 0b10 && opcode == 0b01010 && q != 0 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_umax32 as u64, &[lo, hi]);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_umaxv32_reduce as u64, &[t]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // UMINV .4S: U=1 size=10 opcode=11010
        if u == 1 && size == 0b10 && opcode == 0b11010 && q != 0 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_umin32 as u64, &[lo, hi]);
            let d = ir.new_temp(Type::I64);
            ir.gen_call(d, helper_uminv32_reduce as u64, &[t]);
            self.write_vreg_lo(ir, rd, d);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // SMAXV .8H / SMINV .8H: size=01
        if u == 0 && size == 0b01 && opcode == 0b01010 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = if q != 0 {
                self.read_vreg_hi(ir, rn)
            } else {
                ir.new_const(Type::I64, i16::MAX as u64)
            };
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_smaxv16_pair as u64, &[lo, hi]);
            self.write_vreg_lo(ir, rd, t);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        if u == 0 && size == 0b01 && opcode == 0b11010 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = if q != 0 {
                self.read_vreg_hi(ir, rn)
            } else {
                ir.new_const(Type::I64, i16::MIN as u64)
            };
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_sminv16_pair as u64, &[lo, hi]);
            self.write_vreg_lo(ir, rd, t);
            self.clear_vreg_hi(ir, rd);
            return true;
        }
        // ADDV .4S: U=0 size=10 opcode=11011, Q=1 — sum all 4x 32-bit elements
        if u == 0 && size == 0b10 && opcode == 0b11011 && q != 0 {
            let lo = self.read_vreg_lo(ir, rn);
            let hi = self.read_vreg_hi(ir, rn);
            let t = ir.new_temp(Type::I64);
            ir.gen_call(t, helper_addv32 as u64, &[lo, hi]);
            self.write_vreg_lo(ir, rd, t);
            self.clear_vreg_hi(ir, rd);
            return true;
        }

        // ADDV .8B/.16B: U=0 size=00 opcode=11011 (already at top)
        false
    }
}

// ── EXT instruction ─────────────────────────────────────

impl Aarch64DisasContext {
    /// EXT: 0 Q 10 1110 000 Rm 0 imm4 0 Rn Rd
    pub(crate) fn try_neon_ext(&mut self, ir: &mut Context, insn: u32) -> bool {
        if insn & 0xbfe0_8400 != 0x2e00_0000 {
            return false;
        }
        let q = (insn >> 30) & 1;
        let rm = ((insn >> 16) & 0x1f) as usize;
        let imm4 = ((insn >> 11) & 0xf) as u64;
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as usize;

        let n_lo = self.read_vreg_lo(ir, rn);
        let n_hi = if q != 0 {
            self.read_vreg_hi(ir, rn)
        } else {
            ir.new_const(Type::I64, 0)
        };
        let m_lo = self.read_vreg_lo(ir, rm);
        let m_hi = if q != 0 {
            self.read_vreg_hi(ir, rm)
        } else {
            ir.new_const(Type::I64, 0)
        };

        // EXT concatenates Vm:Vn and extracts starting at byte imm4
        let pos = ir.new_const(Type::I64, imm4);
        let d_lo = ir.new_temp(Type::I64);
        if imm4 < 8 {
            ir.gen_call(d_lo, helper_ext8 as u64, &[n_lo, n_hi, pos]);
        } else {
            let adj = ir.new_const(Type::I64, imm4 - 8);
            ir.gen_call(d_lo, helper_ext8 as u64, &[n_hi, m_lo, adj]);
        }
        self.write_vreg_lo(ir, rd, d_lo);

        if q != 0 {
            let d_hi = ir.new_temp(Type::I64);
            if imm4 < 8 {
                let adj = ir.new_const(Type::I64, imm4);
                ir.gen_call(d_hi, helper_ext8 as u64, &[n_hi, m_lo, adj]);
            } else {
                let adj = ir.new_const(Type::I64, imm4 - 8);
                ir.gen_call(d_hi, helper_ext8 as u64, &[m_lo, m_hi, adj]);
            }
            self.write_vreg_hi(ir, rd, d_hi);
        } else {
            self.clear_vreg_hi(ir, rd);
        }
        true
    }
}

impl Decode<Context> for Aarch64DisasContext {
    // -- Add/Sub immediate --

    fn trans_ADD_i(&mut self, ir: &mut Context, a: &ArgsRriSh) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = if a.shift == 1 {
            (a.imm as u64) << 12
        } else {
            a.imm as u64
        };
        let src = self.read_xreg_sp(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, c);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUB_i(&mut self, ir: &mut Context, a: &ArgsRriSh) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = if a.shift == 1 {
            (a.imm as u64) << 12
        } else {
            a.imm as u64
        };
        let src = self.read_xreg_sp(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, c);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ADDS_i(&mut self, ir: &mut Context, a: &ArgsRriSh) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = if a.shift == 1 {
            (a.imm as u64) << 12
        } else {
            a.imm as u64
        };
        let src = self.read_xreg_sp(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, false);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUBS_i(&mut self, ir: &mut Context, a: &ArgsRriSh) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = if a.shift == 1 {
            (a.imm as u64) << 12
        } else {
            a.imm as u64
        };
        let src = self.read_xreg_sp(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, true);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Logical immediate --

    fn trans_AND_i(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf,
            a.nbit as u32,
            a.immr as u32,
            a.imms as u32,
        ) {
            Some(v) => v,
            None => return false,
        };
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src, c);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ORR_i(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf,
            a.nbit as u32,
            a.immr as u32,
            a.imms as u32,
        ) {
            Some(v) => v,
            None => return false,
        };
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_or(ty, d, src, c);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_EOR_i(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf,
            a.nbit as u32,
            a.immr as u32,
            a.imms as u32,
        ) {
            Some(v) => v,
            None => return false,
        };
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_xor(ty, d, src, c);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ANDS_i(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf,
            a.nbit as u32,
            a.immr as u32,
            a.imms as u32,
        ) {
            Some(v) => v,
            None => return false,
        };
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let c = ir.new_const(ty, imm);
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src, c);
        self.gen_nzcv_logic(ir, d, sf);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Move wide immediate --

    fn trans_MOVZ(&mut self, ir: &mut Context, a: &ArgsRi16) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let val = (a.imm as u64) << (a.hw * 16);
        let c = ir.new_const(ty, val);
        self.write_xreg_sz(ir, a.rd, c, sf);
        true
    }

    fn trans_MOVK(&mut self, ir: &mut Context, a: &ArgsRi16) -> bool {
        let sf = a.sf != 0;
        if a.rd == 31 {
            return true;
        }
        let ty = Self::sf_type(sf);
        let shift = a.hw * 16;
        let mask = if sf {
            !(0xffffu64 << shift)
        } else {
            (!(0xffffu64 << shift)) & 0xffff_ffff
        };
        let bits = (a.imm as u64) << shift;
        let old = if sf {
            self.xregs[a.rd as usize]
        } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, self.xregs[a.rd as usize]);
            t
        };
        let m = ir.new_const(ty, mask);
        let t = ir.new_temp(ty);
        ir.gen_and(ty, t, old, m);
        let b = ir.new_const(ty, bits);
        let d = ir.new_temp(ty);
        ir.gen_or(ty, d, t, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_MOVN(&mut self, ir: &mut Context, a: &ArgsRi16) -> bool {
        let sf = a.sf != 0;
        let val = !((a.imm as u64) << (a.hw * 16));
        let val = if !sf { val & 0xffff_ffff } else { val };
        let c = ir.new_const(Type::I64, val);
        self.write_xreg(ir, a.rd, c);
        true
    }

    // -- PC-relative addressing --

    fn trans_ADR(&mut self, ir: &mut Context, a: &ArgsPcrel) -> bool {
        // ADR immediate: immhi = bits[23:5], immlo = bits[30:29]
        let insn = self.opcode;
        let immlo = ((insn >> 29) & 0x3) as i64;
        let immhi_raw = ((insn >> 5) & 0x7ffff) as i32;
        // Sign-extend from 19 bits
        let immhi = ((immhi_raw << 13) >> 13) as i64;
        let imm = (immhi << 2) | immlo;
        let target = (self.base.pc_next as i64 + imm) as u64;
        let c = ir.new_const(Type::I64, target);
        self.write_xreg(ir, a.rd, c);
        true
    }

    fn trans_ADRP(&mut self, ir: &mut Context, a: &ArgsPcrel) -> bool {
        // ADRP immediate: immhi = bits[23:5], immlo = bits[30:29]
        let insn = self.opcode;
        let immlo = ((insn >> 29) & 0x3) as i64;
        let immhi_raw = ((insn >> 5) & 0x7ffff) as i32;
        // Sign-extend from 19 bits
        let immhi = ((immhi_raw << 13) >> 13) as i64;
        let imm = (immhi << 2) | immlo;
        let base = self.base.pc_next & !0xfff;
        let offset = imm << 12;
        let target = (base as i64 + offset) as u64;
        let c = ir.new_const(Type::I64, target);
        self.write_xreg(ir, a.rd, c);
        true
    }

    // -- Bitfield --

    fn trans_SBFM(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let immr = a.immr as u32;
        let imms = a.imms as u32;
        let bits = if sf { 64u32 } else { 32u32 };

        if imms >= immr {
            // SBFX / ASR / SXTB/SXTH/SXTW
            let len = imms - immr + 1;
            // Signed extract: shift right by immr, then
            // sign-extend from len bits.
            // = (src << (bits - immr - len)) >>a (bits - len)
            // Or equivalently: (src >> immr) sign-ext from len
            if immr == 0 && (len == 8 || len == 16 || len == 32) {
                let d = ir.new_temp(ty);
                ir.gen_sextract(ty, d, src, 0, len);
                self.write_xreg_sz(ir, a.rd, d, sf);
            } else {
                // General case: shift left to put sign bit at
                // top, then arithmetic shift right
                let shl_amt = bits - immr - len;
                let sar_amt = bits - len;
                let d = ir.new_temp(ty);
                if shl_amt > 0 {
                    let sh1 = ir.new_const(ty, shl_amt as u64);
                    ir.gen_shl(ty, d, src, sh1);
                } else {
                    ir.gen_mov(ty, d, src);
                }
                let sh2 = ir.new_const(ty, sar_amt as u64);
                let r = ir.new_temp(ty);
                ir.gen_sar(ty, r, d, sh2);
                self.write_xreg_sz(ir, a.rd, r, sf);
            }
        } else {
            // SBFIZ
            let len = imms + 1;
            let pos = bits - immr;
            // Sign-extend low `len` bits, then shift left by pos
            let d = if len == 8 || len == 16 || len == 32 {
                let t = ir.new_temp(ty);
                ir.gen_sextract(ty, t, src, 0, len);
                t
            } else {
                // General: shl then sar to sign-extend
                let shl_amt = bits - len;
                let t1 = ir.new_temp(ty);
                let sh = ir.new_const(ty, shl_amt as u64);
                ir.gen_shl(ty, t1, src, sh);
                let t2 = ir.new_temp(ty);
                ir.gen_sar(ty, t2, t1, sh);
                t2
            };
            let sh = ir.new_const(ty, pos as u64);
            let d2 = ir.new_temp(ty);
            ir.gen_shl(ty, d2, d, sh);
            self.write_xreg_sz(ir, a.rd, d2, sf);
        }
        true
    }

    fn trans_BFM(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let immr = a.immr as u32;
        let imms = a.imms as u32;

        if imms >= immr {
            // BFXIL: extract len bits from src at immr, insert at bit 0 of dst
            let len = imms - immr + 1;
            if a.rd == 31 {
                return true;
            }
            let dst = self.read_xreg(ir, a.rd);
            let dst = Self::trunc32(ir, dst, sf);
            // Shift src right by immr to get the extracted bits at position 0
            let extracted = if immr > 0 {
                let sh = ir.new_const(ty, immr as u64);
                let t = ir.new_temp(ty);
                ir.gen_shr(ty, t, src, sh);
                t
            } else {
                src
            };
            let d = Self::deposit(ir, ty, dst, extracted, 0, len, sf);
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // BFI
            let len = imms + 1;
            let bits = if sf { 64u32 } else { 32u32 };
            let pos = bits - immr;
            if a.rd == 31 {
                return true;
            }
            let dst = self.read_xreg(ir, a.rd);
            let dst = Self::trunc32(ir, dst, sf);
            let d = Self::deposit(ir, ty, dst, src, pos, len, sf);
            self.write_xreg_sz(ir, a.rd, d, sf);
        }
        true
    }

    fn trans_UBFM(&mut self, ir: &mut Context, a: &ArgsLogicImm) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let immr = a.immr as u32;
        let imms = a.imms as u32;
        let bits = if sf { 64u32 } else { 32u32 };

        if imms >= immr {
            // UBFX / LSR / UXTB/UXTH
            let len = imms - immr + 1;
            // Extract = (src >> immr) & mask
            let d = if immr == 0 {
                let mask_val = if len >= bits {
                    if sf {
                        u64::MAX
                    } else {
                        0xffff_ffff
                    }
                } else {
                    (1u64 << len) - 1
                };
                let m = ir.new_const(ty, mask_val);
                let t = ir.new_temp(ty);
                ir.gen_and(ty, t, src, m);
                t
            } else {
                let sh = ir.new_const(ty, immr as u64);
                let shifted = ir.new_temp(ty);
                ir.gen_shr(ty, shifted, src, sh);
                let mask_val = if len >= bits {
                    if sf {
                        u64::MAX
                    } else {
                        0xffff_ffff
                    }
                } else {
                    (1u64 << len) - 1
                };
                let m = ir.new_const(ty, mask_val);
                let t = ir.new_temp(ty);
                ir.gen_and(ty, t, shifted, m);
                t
            };
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // UBFIZ / LSL: insert low bits of src
            // at position pos in zero
            let len = imms + 1;
            let pos = bits - immr;
            // Mask the low `len` bits, shift left by `pos`
            let mask_val = if len >= 64 {
                u64::MAX
            } else {
                (1u64 << len) - 1
            };
            let m = ir.new_const(ty, mask_val);
            let masked = ir.new_temp(ty);
            ir.gen_and(ty, masked, src, m);
            let sh = ir.new_const(ty, pos as u64);
            let d = ir.new_temp(ty);
            ir.gen_shl(ty, d, masked, sh);
            self.write_xreg_sz(ir, a.rd, d, sf);
        }
        true
    }

    // -- Branches --

    fn trans_B(&mut self, ir: &mut Context, _a: &ArgsBranch) -> bool {
        // imm26 = bits[25:0], sign-extended, *4
        let insn = self.opcode;
        let imm26 = (insn & 0x03ff_ffff) as i32;
        let imm = ((imm26 << 6) >> 6) as i64; // sign-extend
        let target = (self.base.pc_next as i64 + imm * 4) as u64;
        self.gen_direct_branch(ir, target, 0);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BL(&mut self, ir: &mut Context, _a: &ArgsBranch) -> bool {
        let insn = self.opcode;
        let imm26 = (insn & 0x03ff_ffff) as i32;
        let imm = ((imm26 << 6) >> 6) as i64;
        let target = (self.base.pc_next as i64 + imm * 4) as u64;
        let link = self.base.pc_next + 4;
        let c = ir.new_const(Type::I64, link);
        self.write_xreg(ir, 30, c);
        self.gen_direct_branch(ir, target, 0);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BR(&mut self, ir: &mut Context, a: &ArgsBr) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BLR(&mut self, ir: &mut Context, a: &ArgsBr) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        let link = self.base.pc_next + 4;
        let c = ir.new_const(Type::I64, link);
        self.write_xreg(ir, 30, c);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_RET(&mut self, ir: &mut Context, a: &ArgsBr) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_B_cond(&mut self, ir: &mut Context, a: &ArgsBcond) -> bool {
        let target = (self.base.pc_next as i64 + a.imm * 4) as u64;
        let next_pc = self.base.pc_next + 4;

        if a.cond == 0xe {
            // AL — always taken
            self.gen_direct_branch(ir, target, 0);
            self.base.is_jmp = DisasJumpType::NoReturn;
            return true;
        }

        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        ir.gen_brcond(Type::I64, cond_val, zero, Cond::Ne, taken);

        let c = ir.new_const(Type::I64, next_pc);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(0);
        ir.gen_exit_tb(TB_EXIT_IDX0);

        ir.gen_set_label(taken);
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(1);
        ir.gen_exit_tb(TB_EXIT_IDX1);

        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_CBZ(&mut self, ir: &mut Context, a: &ArgsCb) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let target = (self.base.pc_next as i64 + a.imm * 4) as u64;
        let next_pc = self.base.pc_next + 4;

        let val = self.read_xreg(ir, a.rn);
        let val = Self::trunc32(ir, val, sf);
        let zero = ir.new_const(ty, 0);
        let taken = ir.new_label();
        ir.gen_brcond(ty, val, zero, Cond::Eq, taken);

        let c = ir.new_const(Type::I64, next_pc);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(0);
        ir.gen_exit_tb(TB_EXIT_IDX0);

        ir.gen_set_label(taken);
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(1);
        ir.gen_exit_tb(TB_EXIT_IDX1);

        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_CBNZ(&mut self, ir: &mut Context, a: &ArgsCb) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let target = (self.base.pc_next as i64 + a.imm * 4) as u64;
        let next_pc = self.base.pc_next + 4;

        let val = self.read_xreg(ir, a.rn);
        let val = Self::trunc32(ir, val, sf);
        let zero = ir.new_const(ty, 0);
        let taken = ir.new_label();
        ir.gen_brcond(ty, val, zero, Cond::Ne, taken);

        let c = ir.new_const(Type::I64, next_pc);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(0);
        ir.gen_exit_tb(TB_EXIT_IDX0);

        ir.gen_set_label(taken);
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(1);
        ir.gen_exit_tb(TB_EXIT_IDX1);

        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_TBZ(&mut self, ir: &mut Context, a: &ArgsTb) -> bool {
        // TBZ: bit number is encoded in sf:imm5 from insn
        // The decoder gives us sf and rn; we extract bit
        // from the raw opcode.
        let insn = self.opcode;
        let b5 = (insn >> 31) & 1;
        let b40 = (insn >> 19) & 0x1f;
        let bit = (b5 << 5) | b40;
        let imm14 = ((insn >> 5) & 0x3fff) as i32;
        let offset = ((imm14 << 18) >> 18) as i64 * 4;
        let target = (self.base.pc_next as i64 + offset) as u64;
        let next_pc = self.base.pc_next + 4;

        let val = self.read_xreg(ir, a.rn);
        let mask = ir.new_const(Type::I64, 1u64 << bit);
        let t = ir.new_temp(Type::I64);
        ir.gen_and(Type::I64, t, val, mask);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        ir.gen_brcond(Type::I64, t, zero, Cond::Eq, taken);

        let c = ir.new_const(Type::I64, next_pc);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(0);
        ir.gen_exit_tb(TB_EXIT_IDX0);

        ir.gen_set_label(taken);
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(1);
        ir.gen_exit_tb(TB_EXIT_IDX1);

        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_TBNZ(&mut self, ir: &mut Context, a: &ArgsTb) -> bool {
        let insn = self.opcode;
        let b5 = (insn >> 31) & 1;
        let b40 = (insn >> 19) & 0x1f;
        let bit = (b5 << 5) | b40;
        let imm14 = ((insn >> 5) & 0x3fff) as i32;
        let offset = ((imm14 << 18) >> 18) as i64 * 4;
        let target = (self.base.pc_next as i64 + offset) as u64;
        let next_pc = self.base.pc_next + 4;

        let val = self.read_xreg(ir, a.rn);
        let mask = ir.new_const(Type::I64, 1u64 << bit);
        let t = ir.new_temp(Type::I64);
        ir.gen_and(Type::I64, t, val, mask);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        ir.gen_brcond(Type::I64, t, zero, Cond::Ne, taken);

        let c = ir.new_const(Type::I64, next_pc);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(0);
        ir.gen_exit_tb(TB_EXIT_IDX0);

        ir.gen_set_label(taken);
        let c = ir.new_const(Type::I64, target);
        ir.gen_mov(Type::I64, self.pc, c);
        ir.gen_goto_tb(1);
        ir.gen_exit_tb(TB_EXIT_IDX1);

        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    // -- Add/Sub shifted register --

    fn trans_ADD_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUB_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ADDS_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src1, b);
        self.gen_nzcv_add_sub(ir, src1, b, d, sf, false);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUBS_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src1, b);
        self.gen_nzcv_add_sub(ir, src1, b, d, sf, true);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Add/Sub extended register --

    fn trans_ADD_ext(&mut self, ir: &mut Context, a: &ArgsExtReg) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let ext = Self::extend_reg(ir, src2, a.option, a.imm);
        let ext = Self::trunc32(ir, ext, sf);
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, src1, ext);
        let d = Self::trunc32(ir, d, sf);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUB_ext(&mut self, ir: &mut Context, a: &ArgsExtReg) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let ext = Self::extend_reg(ir, src2, a.option, a.imm);
        let ext = Self::trunc32(ir, ext, sf);
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, src1, ext);
        let d = Self::trunc32(ir, d, sf);
        self.write_xreg_sp_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ADDS_ext(&mut self, ir: &mut Context, a: &ArgsExtReg) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let ext = Self::extend_reg(ir, src2, a.option, a.imm);
        let ext = Self::trunc32(ir, ext, sf);
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, src1, ext);
        let d = Self::trunc32(ir, d, sf);
        self.gen_nzcv_add_sub(ir, src1, ext, d, sf, false);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUBS_ext(&mut self, ir: &mut Context, a: &ArgsExtReg) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let ext = Self::extend_reg(ir, src2, a.option, a.imm);
        let ext = Self::trunc32(ir, ext, sf);
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, src1, ext);
        let d = Self::trunc32(ir, d, sf);
        self.gen_nzcv_add_sub(ir, src1, ext, d, sf, true);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Logical shifted register --

    fn trans_AND_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_BIC_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_andc(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ORR_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_or(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ORN_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let nb = ir.new_temp(ty);
        ir.gen_not(ty, nb, b);
        let d = ir.new_temp(ty);
        ir.gen_or(ty, d, src1, nb);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_EOR_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_xor(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_EON_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_eqv(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ANDS_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src1, b);
        self.gen_nzcv_logic(ir, d, sf);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_BICS_r(&mut self, ir: &mut Context, a: &ArgsShiftReg) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(ir, ty, src2, a.shift, imm6 as i64);
        let d = ir.new_temp(ty);
        ir.gen_andc(ty, d, src1, b);
        self.gen_nzcv_logic(ir, d, sf);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Multiply --

    fn trans_MADD(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let acc = self.read_xreg(ir, a.ra);
        let acc = Self::trunc32(ir, acc, sf);
        let prod = ir.new_temp(ty);
        ir.gen_mul(ty, prod, n, m);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, acc, prod);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_MSUB(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let acc = self.read_xreg(ir, a.ra);
        let acc = Self::trunc32(ir, acc, sf);
        let prod = ir.new_temp(ty);
        ir.gen_mul(ty, prod, n, m);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, acc, prod);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SMADDL(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = sext(Wn) * sext(Wm) + Xa
        let n = self.read_xreg(ir, a.rn);
        let n32 = ir.new_temp(Type::I32);
        ir.gen_extrl_i64_i32(n32, n);
        let ns = ir.new_temp(Type::I64);
        ir.gen_ext_i32_i64(ns, n32);
        let m = self.read_xreg(ir, a.rm);
        let m32 = ir.new_temp(Type::I32);
        ir.gen_extrl_i64_i32(m32, m);
        let ms = ir.new_temp(Type::I64);
        ir.gen_ext_i32_i64(ms, m32);
        let prod = ir.new_temp(Type::I64);
        ir.gen_mul(Type::I64, prod, ns, ms);
        let acc = self.read_xreg(ir, a.ra);
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, acc, prod);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_SMSUBL(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = Xa - sext(Wn) * sext(Wm)
        let n = self.read_xreg(ir, a.rn);
        let n32 = ir.new_temp(Type::I32);
        ir.gen_extrl_i64_i32(n32, n);
        let ns = ir.new_temp(Type::I64);
        ir.gen_ext_i32_i64(ns, n32);
        let m = self.read_xreg(ir, a.rm);
        let m32 = ir.new_temp(Type::I32);
        ir.gen_extrl_i64_i32(m32, m);
        let ms = ir.new_temp(Type::I64);
        ir.gen_ext_i32_i64(ms, m32);
        let prod = ir.new_temp(Type::I64);
        ir.gen_mul(Type::I64, prod, ns, ms);
        let acc = self.read_xreg(ir, a.ra);
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, acc, prod);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_UMADDL(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = zext(Wn) * zext(Wm) + Xa
        let n = self.read_xreg(ir, a.rn);
        let nz = ir.new_temp(Type::I64);
        let mask = ir.new_const(Type::I64, 0xffff_ffff);
        ir.gen_and(Type::I64, nz, n, mask);
        let m = self.read_xreg(ir, a.rm);
        let mz = ir.new_temp(Type::I64);
        ir.gen_and(Type::I64, mz, m, mask);
        let prod = ir.new_temp(Type::I64);
        ir.gen_mul(Type::I64, prod, nz, mz);
        let acc = self.read_xreg(ir, a.ra);
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, acc, prod);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_UMSUBL(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = Xa - zext(Wn) * zext(Wm)
        let n = self.read_xreg(ir, a.rn);
        let nz = ir.new_temp(Type::I64);
        let mask = ir.new_const(Type::I64, 0xffff_ffff);
        ir.gen_and(Type::I64, nz, n, mask);
        let m = self.read_xreg(ir, a.rm);
        let mz = ir.new_temp(Type::I64);
        ir.gen_and(Type::I64, mz, m, mask);
        let prod = ir.new_temp(Type::I64);
        ir.gen_mul(Type::I64, prod, nz, mz);
        let acc = self.read_xreg(ir, a.ra);
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, acc, prod);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_UMULH(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = (Xn * Xm) >> 64 (unsigned)
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let lo = ir.new_temp(Type::I64);
        let hi = ir.new_temp(Type::I64);
        ir.gen_mulu2(Type::I64, lo, hi, n, m);
        self.write_xreg(ir, a.rd, hi);
        true
    }

    fn trans_SMULH(&mut self, ir: &mut Context, a: &ArgsRrrrS) -> bool {
        // Xd = (Xn * Xm) >> 64 (signed)
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let lo = ir.new_temp(Type::I64);
        let hi = ir.new_temp(Type::I64);
        ir.gen_muls2(Type::I64, lo, hi, n, m);
        self.write_xreg(ir, a.rd, hi);
        true
    }

    // -- Divide --

    fn trans_ADC(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        // Materialize NZCV so we can read packed C flag.
        self.materialize_nzcv(ir);
        let c29 = ir.new_const(Type::I64, 29);
        let one = ir.new_const(Type::I64, 1);
        let c = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, c, self.nzcv, c29);
        ir.gen_and(Type::I64, c, c, one);
        let d = ir.new_temp(Type::I64);
        ir.gen_add(ty, d, n, m);
        ir.gen_add(ty, d, d, c);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SBC(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        // Materialize NZCV so we can read packed C flag.
        self.materialize_nzcv(ir);
        // Extract C flag (bit 29 of nzcv)
        let c29 = ir.new_const(Type::I64, 29);
        let one = ir.new_const(Type::I64, 1);
        let c = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, c, self.nzcv, c29);
        ir.gen_and(Type::I64, c, c, one);
        // SBC: Rd = Rn - Rm - (1 - C) = Rn - Rm - 1 + C
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(ty, d, n, m);
        ir.gen_sub(ty, d, d, one);
        ir.gen_add(ty, d, d, c);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_UDIV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let d = ir.new_temp(Type::I64);
        if sf {
            ir.gen_call(d, helper_udiv64 as u64, &[n, m]);
        } else {
            ir.gen_call(d, helper_udiv32 as u64, &[n, m]);
        }
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SDIV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let d = ir.new_temp(Type::I64);
        if sf {
            ir.gen_call(d, helper_sdiv64 as u64, &[n, m]);
        } else {
            ir.gen_call(d, helper_sdiv32 as u64, &[n, m]);
        }
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Variable shifts --

    fn trans_LSLV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let d = ir.new_temp(ty);
        ir.gen_shl(ty, d, n, m);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_LSRV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let d = ir.new_temp(ty);
        ir.gen_shr(ty, d, n, m);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ASRV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let d = ir.new_temp(ty);
        ir.gen_sar(ty, d, n, m);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_RORV(&mut self, ir: &mut Context, a: &ArgsRrrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let d = ir.new_temp(ty);
        ir.gen_rotr(ty, d, n, m);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Bit manipulation --

    fn trans_CLZ(&mut self, ir: &mut Context, a: &ArgsRrS) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let bits = if sf { 64u64 } else { 32u64 };
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let fb = ir.new_const(ty, bits);
        let d = ir.new_temp(ty);
        ir.gen_clz(ty, d, src, fb);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_RBIT(&mut self, ir: &mut Context, a: &ArgsRrS) -> bool {
        let sf = a.sf != 0;
        let src = self.read_xreg(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        if sf {
            ir.gen_call(d, helper_rbit64 as u64, &[src]);
        } else {
            ir.gen_call(d, helper_rbit32 as u64, &[src]);
        }
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_REV(&mut self, ir: &mut Context, a: &ArgsRrS) -> bool {
        let sf = a.sf != 0;
        let src = self.read_xreg(ir, a.rn);
        if sf {
            let d = ir.new_temp(Type::I64);
            ir.gen_bswap64(Type::I64, d, src, 0);
            self.write_xreg(ir, a.rd, d);
        } else {
            let s32 = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(s32, src);
            let d32 = ir.new_temp(Type::I32);
            ir.gen_bswap32(Type::I32, d32, s32, 0);
            let d = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(d, d32);
            self.write_xreg(ir, a.rd, d);
        }
        true
    }

    fn trans_REV16(&mut self, ir: &mut Context, a: &ArgsRrS) -> bool {
        let sf = a.sf != 0;
        let src = self.read_xreg(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        if sf {
            ir.gen_call(d, helper_rev16_64 as u64, &[src]);
            self.write_xreg(ir, a.rd, d);
        } else {
            ir.gen_call(d, helper_rev16_32 as u64, &[src]);
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_REV32(&mut self, ir: &mut Context, a: &ArgsRrS) -> bool {
        let sf = a.sf != 0;
        let src = self.read_xreg(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        if sf {
            ir.gen_call(d, helper_rev32_64 as u64, &[src]);
            self.write_xreg(ir, a.rd, d);
        } else {
            let s32 = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(s32, src);
            let d32 = ir.new_temp(Type::I32);
            ir.gen_bswap32(Type::I32, d32, s32, 0);
            ir.gen_ext_u32_i64(d, d32);
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    // -- Conditional select --

    fn trans_CSEL(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(Type::I64, d, cond_val, zero, n, m, Cond::Ne);
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSINC(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let one = ir.new_const(Type::I64, 1);
        let m_inc = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, m_inc, m, one);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(Type::I64, d, cond_val, zero, n, m_inc, Cond::Ne);
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSINV(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let m_inv = ir.new_temp(Type::I64);
        ir.gen_not(Type::I64, m_inv, m);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(Type::I64, d, cond_val, zero, n, m_inv, Cond::Ne);
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSNEG(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let m_neg = ir.new_temp(Type::I64);
        ir.gen_neg(Type::I64, m_neg, m);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(Type::I64, d, cond_val, zero, n, m_neg, Cond::Ne);
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    // -- Loads: unsigned immediate offset --

    fn trans_LDR_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRB_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::ub().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRH_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::uw().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSB_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sb().bits() as u32);
        if a.sf != 0 {
            // W destination form: sign-extend to 32, then clear upper 32 bits.
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSH_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sw().bits() as u32);
        if a.sf != 0 {
            // W destination form: sign-extend to 32, then clear upper 32 bits.
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSW_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let offset = a.imm << 2;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Stores: unsigned immediate offset --

    fn trans_STR_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        true
    }

    fn trans_STRB_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::ub().bits() as u32);
        true
    }

    fn trans_STRH_i(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::uw().bits() as u32);
        true
    }

    // -- Loads: register offset --

    fn trans_LDR_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 {
            if sf {
                3
            } else {
                2
            }
        } else {
            0
        };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRB_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::ub().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRH_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 1 } else { 0 };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::uw().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSH_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 1 } else { 0 };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sw().bits() as u32);
        if a.sf != 0 {
            // W destination form: sign-extend to 32, then clear upper 32 bits.
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSW_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 2 } else { 0 };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSB_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, 0);
        let d = ir.new_temp(Type::I64);
        // sf field (bit22 inverted): sf=0 in decode means 64-bit target (sign-extend to X),
        // sf=1 means 32-bit target (sign-extend to W, zero-extend to X)
        // But we use MemOp::sb() which sign-extends to 64-bit, then mask if needed
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sb().bits() as u32);
        if a.sf != 0 {
            // 32-bit target: mask to 32 bits
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Stores: register offset --

    fn trans_STR_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 {
            if sf {
                3
            } else {
                2
            }
        } else {
            0
        };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        true
    }

    fn trans_STRB_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, 0);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::ub().bits() as u32);
        true
    }

    fn trans_STRH_r(&mut self, ir: &mut Context, a: &ArgsLdstReg) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 1 } else { 0 };
        let addr = self.compute_addr_reg(ir, a.rn, a.rm, a.option, shift);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::uw().bits() as u32);
        true
    }

    // -- PC-relative literal loads --

    fn trans_LDR_lit(&mut self, ir: &mut Context, a: &ArgsLdstLit) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr_val = (self.base.pc_next as i64 + a.imm * 4) as u64;
        let addr = ir.new_const(Type::I64, addr_val);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSW_lit(&mut self, ir: &mut Context, a: &ArgsLdstLit) -> bool {
        let addr_val = (self.base.pc_next as i64 + a.imm * 4) as u64;
        let addr = ir.new_const(Type::I64, addr_val);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Pre/post-index loads/stores --

    fn trans_LDR_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        // Writeback
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_STR_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_LDR_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        // Writeback: base + offset
        let new_base = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    fn trans_STR_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let base = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, base, memop.bits() as u32);
        let new_base = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    fn trans_LDRB_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::ub().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_STRB_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::ub().bits() as u32);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRH_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::uw().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_STRH_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::uw().bits() as u32);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRSB_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sb().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRSH_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sw().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRB_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::ub().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_STRB_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, base, MemOp::ub().bits() as u32);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_LDRH_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::uw().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_STRH_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, base, MemOp::uw().bits() as u32);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_LDRSB_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::sb().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_LDRSH_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::sw().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_LDRSW_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRSW_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }

    // -- Load/Store pair --

    fn trans_LDP(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d1, addr, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d2, addr2, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        true
    }

    fn trans_STP(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, v1, addr, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(Type::I64, v2, addr2, memop.bits() as u32);
        true
    }

    fn trans_LDP_pre(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d1, addr, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d2, addr2, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_STP_pre(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, v1, addr, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(Type::I64, v2, addr2, memop.bits() as u32);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_LDP_post(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let base = self.read_xreg_sp(ir, a.rn);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d1, base, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, base, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d2, addr2, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        let new_base = self.compute_addr_imm(ir, a.rn, offset);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    fn trans_STP_post(&mut self, ir: &mut Context, a: &ArgsLdstPair) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let base = self.read_xreg_sp(ir, a.rn);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, v1, base, memop.bits() as u32);
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, base, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(Type::I64, v2, addr2, memop.bits() as u32);
        let new_base = self.compute_addr_imm(ir, a.rn, offset);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    // -- System --

    fn trans_NOP(&mut self, _ir: &mut Context, _a: &ArgsEmpty) -> bool {
        true
    }

    fn trans_SVC(&mut self, ir: &mut Context, _a: &ArgsSys) -> bool {
        let pc = ir.new_const(Type::I64, self.base.pc_next);
        ir.gen_mov(Type::I64, self.pc, pc);
        ir.gen_exit_tb(EXCP_ECALL);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_MRS(&mut self, ir: &mut Context, a: &ArgsSys) -> bool {
        // Decode system register from raw opcode.
        let insn = self.opcode;
        let op0 = ((insn >> 19) & 0x1) + 2;
        let op1 = (insn >> 16) & 0x7;
        let crn = (insn >> 12) & 0xf;
        let crm = (insn >> 8) & 0xf;
        let op2 = (insn >> 5) & 0x7;

        // TPIDR_EL0
        if op0 == 3 && op1 == 3 && crn == 13 && crm == 0 && op2 == 2 {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(Type::I64, v, self.env, TPIDR_EL0_OFFSET);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // NZCV
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 2 && op2 == 0 {
            // Materialize NZCV before reading it.
            self.materialize_nzcv(ir);
            let v = self.nzcv;
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // FPCR
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 4 && op2 == 0 {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(Type::I64, v, self.env, FPCR_OFFSET);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // FPSR
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 4 && op2 == 1 {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(Type::I64, v, self.env, FPSR_OFFSET);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // DCZID_EL0: op0=3, op1=3, CRn=0, CRm=0, op2=7
        // Return DZP=1 (bit4) to disable DC ZVA usage.
        if op0 == 3 && op1 == 3 && crn == 0 && crm == 0 && op2 == 7 {
            let v = ir.new_const(Type::I64, 0x10);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // CTR_EL0: op0=3, op1=3, CRn=0, CRm=0, op2=1
        // Return a reasonable cache geometry.
        if op0 == 3 && op1 == 3 && crn == 0 && crm == 0 && op2 == 1 {
            // IminLine=4 (16 words = 64 bytes), DminLine=4,
            // L1Ip=3 (PIPT), bits: 0x80038003
            let v = ir.new_const(Type::I64, 0x80038003);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        false
    }

    fn trans_MSR(&mut self, ir: &mut Context, a: &ArgsSys) -> bool {
        let insn = self.opcode;
        let op0 = ((insn >> 19) & 0x1) + 2;
        let op1 = (insn >> 16) & 0x7;
        let crn = (insn >> 12) & 0xf;
        let crm = (insn >> 8) & 0xf;
        let op2 = (insn >> 5) & 0x7;
        let val = self.read_xreg(ir, a.rd);

        // TPIDR_EL0
        if op0 == 3 && op1 == 3 && crn == 13 && crm == 0 && op2 == 2 {
            ir.gen_st(Type::I64, val, self.env, TPIDR_EL0_OFFSET);
            return true;
        }
        // NZCV
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 2 && op2 == 0 {
            self.set_nzcv_eager(ir, val);
            return true;
        }
        // FPCR
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 4 && op2 == 0 {
            ir.gen_st(Type::I64, val, self.env, FPCR_OFFSET);
            return true;
        }
        // FPSR
        if op0 == 3 && op1 == 3 && crn == 4 && crm == 4 && op2 == 1 {
            ir.gen_st(Type::I64, val, self.env, FPSR_OFFSET);
            return true;
        }
        false
    }

    // -- Unscaled loads/stores (LDUR/STUR) --

    fn trans_LDUR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STUR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        true
    }

    fn trans_LDURB(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::ub().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STURB(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::ub().bits() as u32);
        true
    }
    fn trans_LDURH(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::uw().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STURH(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, MemOp::uw().bits() as u32);
        true
    }

    fn trans_LDURSW(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sl().bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDURSH(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sw().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDURSB(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sb().bits() as u32);
        if a.sf != 0 {
            let mask = ir.new_const(Type::I64, 0xffff_ffff);
            ir.gen_and(Type::I64, d, d, mask);
        }
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Conditional compare (CCMP/CCMN) --

    fn trans_CCMP(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(Type::I64, cond_val, zero, Cond::Ne, taken);
        // Condition false: set NZCV to immediate (eager)
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        self.set_nzcv_eager(ir, imm_c);
        ir.gen_br(done);
        // Condition true: do CMP (SUBS discarding result)
        ir.gen_set_label(taken);
        self.invalidate_lazy_nzcv();
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = self.read_xreg(ir, a.rm);
        let b = Self::trunc32(ir, b, sf);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, true);
        ir.gen_set_label(done);
        // After join: cc_op unknown at compile time
        self.invalidate_lazy_nzcv();
        true
    }

    fn trans_CCMN(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(Type::I64, cond_val, zero, Cond::Ne, taken);
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        self.set_nzcv_eager(ir, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        self.invalidate_lazy_nzcv();
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = self.read_xreg(ir, a.rm);
        let b = Self::trunc32(ir, b, sf);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, false);
        ir.gen_set_label(done);
        self.invalidate_lazy_nzcv();
        true
    }

    fn trans_CCMP_i(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let imm5 = a.rm as u64; // rm field holds imm5
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(Type::I64, cond_val, zero, Cond::Ne, taken);
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        self.set_nzcv_eager(ir, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        self.invalidate_lazy_nzcv();
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm5);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, true);
        ir.gen_set_label(done);
        // After join, runtime may come from either:
        // - false path: eager NZCV immediate
        // - true path: lazy SUB flags
        // Compile-time lazy state is therefore unknown.
        self.invalidate_lazy_nzcv();
        true
    }

    fn trans_CCMN_i(&mut self, ir: &mut Context, a: &ArgsCsel) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let imm5 = a.rm as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(Type::I64, cond_val, zero, Cond::Ne, taken);
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        self.set_nzcv_eager(ir, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        self.invalidate_lazy_nzcv();
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm5);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, false);
        ir.gen_set_label(done);
        self.invalidate_lazy_nzcv();
        true
    }

    // -- Extract (EXTR) --

    fn trans_EXTR(&mut self, ir: &mut Context, a: &ArgsRrrSf) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imms = ((self.opcode >> 10) & 0x3f) as u64;
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        if a.rn == a.rm {
            // ROR alias
            let sh = ir.new_const(ty, imms);
            let d = ir.new_temp(ty);
            ir.gen_rotr(ty, d, src1, sh);
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else if imms == 0 {
            self.write_xreg_sz(ir, a.rd, src1, sf);
        } else {
            let bits = if sf { 64u64 } else { 32 };
            let sh_lo = ir.new_const(ty, imms);
            let sh_hi = ir.new_const(ty, bits - imms);
            let lo = ir.new_temp(ty);
            ir.gen_shr(ty, lo, src2, sh_lo);
            let hi = ir.new_temp(ty);
            ir.gen_shl(ty, hi, src1, sh_hi);
            let d = ir.new_temp(ty);
            ir.gen_or(ty, d, hi, lo);
            self.write_xreg_sz(ir, a.rd, d, sf);
        }
        true
    }

    // -- Load-Acquire / Store-Release --

    fn trans_LDAR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let memop = match a.opc_lo {
            29 => MemOp::ub(), // LDARB
            30 => MemOp::uw(), // LDARH
            _ => {
                if a.sf != 0 {
                    MemOp::uq()
                } else {
                    MemOp::ul()
                }
            }
        };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDAXR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        // Simplified: treat as regular load (no exclusives)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDXR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        // Simplified: treat as regular load (no exclusives
        // in single-threaded mode)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, memop.bits() as u32);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STLR(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        true
    }

    fn trans_STXR(&mut self, ir: &mut Context, a: &ArgsStx) -> bool {
        // Simplified: always succeeds (single-threaded)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        // Write 0 (success) to status register Rs
        let zero = ir.new_const(Type::I64, 0);
        self.write_xreg(ir, a.rs, zero);
        true
    }

    fn trans_STLXR(&mut self, ir: &mut Context, a: &ArgsStx) -> bool {
        // Same as STXR — store-release exclusive, always
        // succeeds in single-threaded mode
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(Type::I64, val, addr, memop.bits() as u32);
        // Write 0 (success) to status register Rs
        let zero = ir.new_const(Type::I64, 0);
        self.write_xreg(ir, a.rs, zero);
        true
    }

    // -- Barriers --

    fn trans_DMB(&mut self, _ir: &mut Context, _a: &ArgsSys) -> bool {
        // Single-threaded: barriers are NOPs
        true
    }
}
