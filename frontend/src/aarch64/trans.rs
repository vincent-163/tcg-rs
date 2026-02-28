//! AArch64 instruction translation — TCG IR generation.
//!
//! Translates decoded A64 instructions into TCG IR opcodes.
//! Follows the same gen_xxx helper pattern as the RISC-V frontend.

use super::cpu::{
    vreg_hi_offset, vreg_lo_offset, FPCR_OFFSET,
    FPSR_OFFSET, NZCV_OFFSET, TPIDR_EL0_OFFSET,
};
use super::insn_decode::*;
use super::Aarch64DisasContext;
use crate::DisasJumpType;
use tcg_core::context::Context;
use tcg_core::tb::{
    EXCP_ECALL, TB_EXIT_IDX0, TB_EXIT_IDX1,
    TB_EXIT_NOCHAIN,
};
use tcg_core::types::{Cond, MemOp, Type};
use tcg_core::TempIdx;

/// Binary IR operation: `fn(ir, ty, dst, lhs, rhs) -> dst`.
type BinOp =
    fn(&mut Context, Type, TempIdx, TempIdx, TempIdx)
        -> TempIdx;

// ── Bitmask immediate decoding ───────────────────────────

fn decode_bitmask_imm(
    sf: bool, n: u32, immr: u32, imms: u32,
) -> Option<u64> {
    let len = if n != 0 {
        6
    } else {
        let combined = (!imms & 0x3f) as u32;
        if combined == 0 { return None; }
        31 - combined.leading_zeros()
    };
    if len == 0 { return None; }
    let size = 1u32 << len;
    let mask = size - 1;
    let s = imms & mask;
    let r = immr & mask;
    if s == mask { return None; }
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
    while sz < 64 { imm |= imm << sz; sz <<= 1; }
    if !sf { imm &= 0xffff_ffff; }
    Some(imm)
}

// ── Helpers ──────────────────────────────────────────────

impl Aarch64DisasContext {
    pub(crate) fn read_xreg(
        &self, ir: &mut Context, reg: i64,
    ) -> TempIdx {
        if reg == 31 {
            ir.new_const(Type::I64, 0)
        } else {
            self.xregs[reg as usize]
        }
    }

    pub(crate) fn write_xreg(
        &self, ir: &mut Context, reg: i64, val: TempIdx,
    ) {
        if reg != 31 {
            ir.gen_mov(
                Type::I64, self.xregs[reg as usize], val,
            );
        }
    }

    pub(crate) fn read_xreg_sp(
        &self, ir: &mut Context, reg: i64,
    ) -> TempIdx {
        if reg == 31 { self.sp }
        else { self.xregs[reg as usize] }
    }

    pub(crate) fn write_xreg_sp(
        &self, ir: &mut Context, reg: i64, val: TempIdx,
    ) {
        if reg == 31 {
            ir.gen_mov(Type::I64, self.sp, val);
        } else {
            ir.gen_mov(
                Type::I64, self.xregs[reg as usize], val,
            );
        }
    }

    /// Write with optional 32-bit zero-extension.
    fn write_xreg_sz(
        &self, ir: &mut Context, reg: i64,
        val: TempIdx, sf: bool,
    ) {
        if reg == 31 { return; }
        if sf {
            ir.gen_mov(
                Type::I64, self.xregs[reg as usize], val,
            );
        } else {
            let ext = ir.new_temp(Type::I64);
            ir.gen_ext_u32_i64(ext, val);
            ir.gen_mov(
                Type::I64, self.xregs[reg as usize], ext,
            );
        }
    }

    fn write_xreg_sp_sz(
        &self, ir: &mut Context, reg: i64,
        val: TempIdx, sf: bool,
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
        if sf { Type::I64 } else { Type::I32 }
    }

    fn trunc32(
        ir: &mut Context, val: TempIdx, sf: bool,
    ) -> TempIdx {
        if sf { val } else {
            let t = ir.new_temp(Type::I32);
            ir.gen_extrl_i64_i32(t, val);
            t
        }
    }

    fn apply_shift(
        ir: &mut Context, ty: Type, val: TempIdx,
        shift_type: i64, amount: i64,
    ) -> TempIdx {
        if amount == 0 { return val; }
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

    // -- NZCV for add/sub --
    fn gen_nzcv_add_sub(
        &self, ir: &mut Context,
        a: TempIdx, b: TempIdx, result: TempIdx,
        sf: bool, is_sub: bool,
    ) {
        let ty = Self::sf_type(sf);
        let bits = if sf { 63u64 } else { 31u64 };
        let zero = ir.new_const(ty, 0);
        let sh = ir.new_const(ty, bits);

        // N
        let n_tmp = ir.new_temp(ty);
        ir.gen_shr(ty, n_tmp, result, sh);
        let n_bit = ir.new_temp(Type::I64);
        if sf { ir.gen_mov(Type::I64, n_bit, n_tmp); }
        else { ir.gen_ext_u32_i64(n_bit, n_tmp); }

        // Z
        let z_bit = ir.new_temp(Type::I64);
        ir.gen_setcond(ty, z_bit, result, zero, Cond::Eq);

        // C
        let c_bit = ir.new_temp(Type::I64);
        if is_sub {
            ir.gen_setcond(ty, c_bit, a, b, Cond::Geu);
        } else {
            ir.gen_setcond(
                ty, c_bit, result, a, Cond::Ltu,
            );
        }

        // V
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
        if sf { ir.gen_mov(Type::I64, v_bit, v_sh); }
        else { ir.gen_ext_u32_i64(v_bit, v_sh); }

        // Pack (N<<31)|(Z<<30)|(C<<29)|(V<<28)
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
        ir.gen_mov(Type::I64, self.nzcv, nzcv);
    }

    // -- NZCV for logical (C=0, V=0) --
    fn gen_nzcv_logic(
        &self, ir: &mut Context,
        result: TempIdx, sf: bool,
    ) {
        let ty = Self::sf_type(sf);
        let bits = if sf { 63u64 } else { 31u64 };
        let zero = ir.new_const(ty, 0);
        let sh = ir.new_const(ty, bits);

        let n_tmp = ir.new_temp(ty);
        ir.gen_shr(ty, n_tmp, result, sh);
        let n_bit = ir.new_temp(Type::I64);
        if sf { ir.gen_mov(Type::I64, n_bit, n_tmp); }
        else { ir.gen_ext_u32_i64(n_bit, n_tmp); }

        let z_bit = ir.new_temp(Type::I64);
        ir.gen_setcond(ty, z_bit, result, zero, Cond::Eq);

        let c31 = ir.new_const(Type::I64, 31);
        let c30 = ir.new_const(Type::I64, 30);
        let n_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, n_s, n_bit, c31);
        let z_s = ir.new_temp(Type::I64);
        ir.gen_shl(Type::I64, z_s, z_bit, c30);
        let nzcv = ir.new_temp(Type::I64);
        ir.gen_or(Type::I64, nzcv, n_s, z_s);
        ir.gen_mov(Type::I64, self.nzcv, nzcv);
    }

    // -- Condition evaluation --
    fn eval_cond(
        &self, ir: &mut Context, cond: i64,
    ) -> TempIdx {
        let nzcv = self.nzcv;
        let c31 = ir.new_const(Type::I64, 31);
        let c30 = ir.new_const(Type::I64, 30);
        let c29 = ir.new_const(Type::I64, 29);
        let c28 = ir.new_const(Type::I64, 28);
        let one64 = ir.new_const(Type::I64, 1);
        let zero = ir.new_const(Type::I64, 0);

        let n = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, n, nzcv, c31);
        ir.gen_and(Type::I64, n, n, one64);
        let z = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, z, nzcv, c30);
        ir.gen_and(Type::I64, z, z, one64);
        let c = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, c, nzcv, c29);
        ir.gen_and(Type::I64, c, c, one64);
        let v = ir.new_temp(Type::I64);
        ir.gen_shr(Type::I64, v, nzcv, c28);
        ir.gen_and(Type::I64, v, v, one64);

        let base_cond = (cond >> 1) as u32;
        let result = ir.new_temp(Type::I64);
        match base_cond {
            0 => { // EQ/NE: Z==1
                ir.gen_setcond(
                    Type::I64, result, z, zero, Cond::Ne,
                );
            }
            1 => { // CS/CC: C==1
                ir.gen_setcond(
                    Type::I64, result, c, zero, Cond::Ne,
                );
            }
            2 => { // MI/PL: N==1
                ir.gen_setcond(
                    Type::I64, result, n, zero, Cond::Ne,
                );
            }
            3 => { // VS/VC: V==1
                ir.gen_setcond(
                    Type::I64, result, v, zero, Cond::Ne,
                );
            }
            4 => { // HI/LS: C==1 && Z==0
                let t = ir.new_temp(Type::I64);
                ir.gen_andc(Type::I64, t, c, z);
                ir.gen_setcond(
                    Type::I64, result, t, zero, Cond::Ne,
                );
            }
            5 => { // GE/LT: N==V
                ir.gen_setcond(
                    Type::I64, result, n, v, Cond::Eq,
                );
            }
            6 => { // GT/LE: N==V && Z==0
                let nv = ir.new_temp(Type::I64);
                ir.gen_setcond(
                    Type::I64, nv, n, v, Cond::Eq,
                );
                let t = ir.new_temp(Type::I64);
                ir.gen_andc(Type::I64, t, nv, z);
                ir.gen_setcond(
                    Type::I64, result, t, zero, Cond::Ne,
                );
            }
            7 => { // AL
                let one = ir.new_const(Type::I64, 1);
                ir.gen_mov(Type::I64, result, one);
            }
            _ => unreachable!(),
        }
        // Invert if low bit set (and not AL/NV).
        if (cond & 1) != 0 && cond != 0xf {
            let inv = ir.new_temp(Type::I64);
            ir.gen_setcond(
                Type::I64, inv, result, zero, Cond::Eq,
            );
            inv
        } else {
            result
        }
    }

    // -- Branch helpers --
    fn gen_direct_branch(
        &mut self, ir: &mut Context,
        target: u64, slot: u32,
    ) {
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

    fn gen_indirect_branch(
        &mut self, ir: &mut Context, addr: TempIdx,
    ) {
        ir.gen_mov(Type::I64, self.pc, addr);
        ir.gen_exit_tb(TB_EXIT_NOCHAIN);
    }

    // -- Load/store address helpers --
    pub(crate) fn compute_addr_imm(
        &self, ir: &mut Context, rn: i64, offset: i64,
    ) -> TempIdx {
        let base = self.read_xreg_sp(ir, rn);
        if offset == 0 { return base; }
        let c = ir.new_const(Type::I64, offset as u64);
        let addr = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr, base, c);
        addr
    }

    fn compute_addr_reg(
        &self, ir: &mut Context,
        rn: i64, rm: i64,
        option: i64, shift_amount: i64,
    ) -> TempIdx {
        let base = self.read_xreg_sp(ir, rn);
        let idx = self.read_xreg(ir, rm);
        let ext = match option {
            0b010 => { // UXTW
                let t = ir.new_temp(Type::I64);
                let mask =
                    ir.new_const(Type::I64, 0xffff_ffff);
                ir.gen_and(Type::I64, t, idx, mask);
                t
            }
            0b110 => { // SXTW
                let t32 = ir.new_temp(Type::I32);
                ir.gen_extrl_i64_i32(t32, idx);
                let t = ir.new_temp(Type::I64);
                ir.gen_ext_i32_i64(t, t32);
                t
            }
            _ => idx, // LSL/UXTX/SXTX
        };
        let shifted = if shift_amount != 0 {
            let sh =
                ir.new_const(Type::I64, shift_amount as u64);
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
        ir: &mut Context, val: TempIdx,
        option: i64, shift: i64,
    ) -> TempIdx {
        // Extract based on option[1:0] size
        let extracted = match option & 0x3 {
            0 => { // xTB - byte
                let t = ir.new_temp(Type::I64);
                let m = ir.new_const(Type::I64, 0xff);
                ir.gen_and(Type::I64, t, val, m);
                t
            }
            1 => { // xTH - halfword
                let t = ir.new_temp(Type::I64);
                let m = ir.new_const(Type::I64, 0xffff);
                ir.gen_and(Type::I64, t, val, m);
                t
            }
            2 => { // xTW - word
                let t = ir.new_temp(Type::I64);
                let m =
                    ir.new_const(Type::I64, 0xffff_ffff);
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
                    ir.gen_sextract(
                        Type::I64, t, val, 0, 16,
                    );
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
        ir: &mut Context, ty: Type,
        dst: TempIdx, src: TempIdx,
        ofs: u32, len: u32, sf: bool,
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
                if sf { u64::MAX } else { 0xffff_ffff }
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

    fn read_vreg_lo(
        &self, ir: &mut Context, reg: usize,
    ) -> TempIdx {
        let d = ir.new_temp(Type::I64);
        ir.gen_ld(
            Type::I64, d, self.env, vreg_lo_offset(reg),
        );
        d
    }

    fn read_vreg_hi(
        &self, ir: &mut Context, reg: usize,
    ) -> TempIdx {
        let d = ir.new_temp(Type::I64);
        ir.gen_ld(
            Type::I64, d, self.env, vreg_hi_offset(reg),
        );
        d
    }

    fn write_vreg_lo(
        &self, ir: &mut Context, reg: usize,
        val: TempIdx,
    ) {
        ir.gen_st(
            Type::I64, val, self.env, vreg_lo_offset(reg),
        );
    }

    fn write_vreg_hi(
        &self, ir: &mut Context, reg: usize,
        val: TempIdx,
    ) {
        ir.gen_st(
            Type::I64, val, self.env, vreg_hi_offset(reg),
        );
    }

    /// Write full 128-bit vreg (lo, hi).
    fn write_vreg128(
        &self, ir: &mut Context, reg: usize,
        lo: TempIdx, hi: TempIdx,
    ) {
        self.write_vreg_lo(ir, reg, lo);
        self.write_vreg_hi(ir, reg, hi);
    }

    /// Zero the high half of a vreg.
    fn clear_vreg_hi(
        &self, ir: &mut Context, reg: usize,
    ) {
        let z = ir.new_const(Type::I64, 0);
        self.write_vreg_hi(ir, reg, z);
    }
}

// ── NEON / FP manual decoder ────────────────────────────

impl Aarch64DisasContext {
    /// Try to decode and translate a NEON/FP instruction.
    /// Returns true if handled.
    pub(crate) fn try_neon(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        // Dispatch by top-level encoding groups
        let op0 = (insn >> 25) & 0xf;
        match op0 {
            // Load/Store SIMD & FP
            0b0100 | 0b0110 | 0b1100 | 0b1110 => {
                self.try_fp_ldst(ir, insn)
            }
            // Data processing — SIMD & FP
            0b0111 | 0b1111 => {
                self.try_fp_data(ir, insn)
            }
            _ => false,
        }
    }
}

// ── FP/SIMD load/store ──────────────────────────────────

impl Aarch64DisasContext {
    fn try_fp_ldst(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
        if top6 == 0b111100
            && (insn >> 21) & 1 == 0
            && op3 == 0
        {
            return self.fp_ldst_unscaled(ir, insn);
        }

        // LDR/STR pre/post-index — SIMD & FP
        if top6 == 0b111100
            && (insn >> 21) & 1 == 0
            && (op3 == 1 || op3 == 3)
        {
            return self.fp_ldst_prepost(ir, insn);
        }

        // LDR/STR register offset — SIMD & FP
        if top6 == 0b111100
            && (insn >> 21) & 1 == 1
            && op3 == 2
        {
            return self.fp_ldst_reg(ir, insn);
        }

        // LDP/STP — SIMD & FP
        // xx 101 1xx opc imm7 rt2 rn rt
        if (insn >> 26) & 0x3f == 0b101011 {
            return self.fp_ldst_pair(ir, insn);
        }

        // LD1/ST1 multiple structures — 0 Q 001100 0 L 000000 opcode size Rn Rt
        if top6 == 0b001100 && (insn >> 21) & 1 == 0 {
            return self.fp_ldst_multiple(ir, insn);
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
        &mut self, ir: &mut Context, reg: usize,
        addr: TempIdx, log2: u32, is_128: bool,
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
        &mut self, ir: &mut Context, reg: usize,
        addr: TempIdx, log2: u32, is_128: bool,
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
    fn fp_ldst_uimm(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldr_literal(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldst_unscaled(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldst_prepost(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldst_reg(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldst_pair(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
    fn fp_ldst_multiple(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        let q = (insn >> 30) & 1;
        let is_load = (insn >> 22) & 1 != 0;
        let opcode = (insn >> 12) & 0xf;
        let size = (insn >> 10) & 0x3;
        let rn = ((insn >> 5) & 0x1f) as i64;
        let rt = (insn & 0x1f) as usize;

        // Only handle single-register LD1/ST1 (opcode=0b0111)
        let nregs = match opcode {
            0b0111 => 1, // LD1/ST1 {Vt.T}, [Xn]
            0b1010 => 2, // LD1/ST1 {Vt.T, Vt2.T}, [Xn]
            0b0110 => 3, // LD1/ST1 {Vt.T, Vt2.T, Vt3.T}, [Xn]
            0b0010 => 4, // LD1/ST1 4 regs
            _ => return false,
        };

        let bytes_per_reg: u64 = if q != 0 { 16 } else { 8 };
        let addr = self.read_xreg_sp(ir, rn);

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
                ir.gen_qemu_ld(Type::I64, lo, cur_addr, MemOp::uq().bits() as u32);
                self.write_vreg_lo(ir, reg, lo);
                if q != 0 {
                    let c8 = ir.new_const(Type::I64, 8);
                    let hi_addr = ir.new_temp(Type::I64);
                    ir.gen_add(Type::I64, hi_addr, cur_addr, c8);
                    let hi = ir.new_temp(Type::I64);
                    ir.gen_qemu_ld(Type::I64, hi, hi_addr, MemOp::uq().bits() as u32);
                    self.write_vreg_hi(ir, reg, hi);
                } else {
                    self.clear_vreg_hi(ir, reg);
                }
            } else {
                let lo = self.read_vreg_lo(ir, reg);
                ir.gen_qemu_st(Type::I64, lo, cur_addr, MemOp::uq().bits() as u32);
                if q != 0 {
                    let c8 = ir.new_const(Type::I64, 8);
                    let hi_addr = ir.new_temp(Type::I64);
                    ir.gen_add(Type::I64, hi_addr, cur_addr, c8);
                    let hi = self.read_vreg_hi(ir, reg);
                    ir.gen_qemu_st(Type::I64, hi, hi_addr, MemOp::uq().bits() as u32);
                }
            }
        }
        true
    }

    /// FP/SIMD data processing — handles DUP, UMOV, and other
    /// NEON instructions needed by glibc.
    fn try_fp_data(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        // DUP (general) — 0 Q 00 1110 000 imm5 0 0001 1 Rn Rd
        // Encodes as: 0x0e000c00 mask 0xbfe0fc00
        if insn & 0xbfe0_fc00 == 0x0e00_0c00 {
            return self.neon_dup_general(ir, insn);
        }
        // UMOV / MOV (to general) — 0 Q 00 1110 000 imm5 0 0111 1 Rn Rd
        // Encodes as: 0x0e003c00 mask 0xbfe0fc00
        if insn & 0xbfe0_fc00 == 0x0e00_3c00 {
            return self.neon_umov(ir, insn);
        }
        // MOVI/MVNI — 0 Q op 0 1111 00 abc cmode 01 defgh Rd
        if insn & 0x9ff8_0400 == 0x0f00_0400 {
            return self.neon_movi(ir, insn);
        }
        // FMOV Xd, Dn — 1001 1110 0110 0110 0000 00 Rn Rd
        if insn & 0xffff_fc00 == 0x9e66_0000 {
            return self.neon_fmov_to_gpr(ir, insn);
        }
        // Dispatch 3-same / 2-reg-misc / shift-imm by top bits
        self.try_neon_3same_misc(ir, insn)
    }

    /// DUP (general): replicate a GPR scalar into all vector lanes.
    fn neon_dup_general(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
            if q != 0 { self.write_vreg_hi(ir, rd, lo); }
            else { self.clear_vreg_hi(ir, rd); }
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
            if q != 0 { self.write_vreg_hi(ir, rd, lo); }
            else { self.clear_vreg_hi(ir, rd); }
        } else if imm5 & 8 != 0 {
            // 64-bit
            self.write_vreg_lo(ir, rd, src);
            if q != 0 { self.write_vreg_hi(ir, rd, src); }
            else { self.clear_vreg_hi(ir, rd); }
        } else {
            return false;
        }
        true
    }

    /// UMOV: extract a vector element to a GPR.
    fn neon_umov(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        let q = (insn >> 30) & 1;
        let imm5 = (insn >> 16) & 0x1f;
        let rn = (insn & 0x1f) as usize; // Vn — source SIMD reg
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
            } else { half };
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
            } else { half };
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
            } else { half };
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
    fn neon_movi(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
            0b000 => { let v = imm8; v | (v << 32) }
            0b001 => { let v = imm8 << 8; v | (v << 32) }
            0b010 => { let v = imm8 << 16; v | (v << 32) }
            0b011 => { let v = imm8 << 24; v | (v << 32) }
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
                } else {
                    return false; // FMOV — not yet
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

    /// FMOV Xd, Dn — move D register low half to GPR.
    fn neon_fmov_to_gpr(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        let rn = ((insn >> 5) & 0x1f) as usize;
        let rd = (insn & 0x1f) as i64;
        let val = self.read_vreg_lo(ir, rn);
        self.write_xreg(ir, rd, val);
        true
    }

    /// Dispatch NEON 3-same, 2-reg-misc, and shift-immediate.
    fn neon_3same(&mut self, _ir: &mut Context, _insn: u32) -> bool { false }
    fn neon_2reg_misc(&mut self, _ir: &mut Context, _insn: u32) -> bool { false }
    fn neon_shift_imm(&mut self, _ir: &mut Context, _insn: u32) -> bool { false }
    fn neon_across_lanes(&mut self, _ir: &mut Context, _insn: u32) -> bool { false }

    fn try_neon_3same_misc(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
        // AdvSIMD three same: 0 Q U 01110 size 1 Rm opcode 1 Rn Rd
        if insn & 0x9f20_0400 == 0x0e20_0400 {
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
        // AdvSIMD pairwise: 0 Q U 01110 size 1 Rm opcode 1 Rn Rd
        // (same encoding as 3-same, handled there)
        false
    }
}

// ── NEON helper functions (called via gen_call) ─────────

/// Byte-wise compare equal: each byte → 0xFF if equal, 0x00 otherwise.
unsafe extern "C" fn helper_cmeq8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        if ab == bb { r |= 0xffu64 << (i * 8); }
    }
    r
}

/// Byte-wise unsigned compare higher or same.
unsafe extern "C" fn helper_cmhs8(a: u64, b: u64) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        let ab = (a >> (i * 8)) & 0xff;
        let bb = (b >> (i * 8)) & 0xff;
        if ab >= bb { r |= 0xffu64 << (i * 8); }
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

// ── NEON 3-same, 2-reg-misc, shift-imm ─────────────────

impl Aarch64DisasContext {
    /// Helper: apply a per-u64-half operation on vector registers.
    fn neon_binop_halves(
        &mut self, ir: &mut Context, q: u32,
        rd: usize, rn: usize, rm: usize,
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
        &mut self, ir: &mut Context, q: u32,
        rd: usize, rn: usize, rm: usize,
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

    /// AdvSIMD three same: 0 Q U 01110 size 1 Rm opcode 1 Rn Rd
    fn neon_3same(
        &mut self, ir: &mut Context, insn: u32,
    ) -> bool {
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
                (0, 0b00) => { // AND
                    self.neon_binop_halves(ir, q, rd, rn, rm,
                        |ir, a, b| { let d = ir.new_temp(Type::I64); ir.gen_and(Type::I64, d, a, b); d });
                    true
                }
                (0, 0b01) => { // BIC
                    self.neon_binop_halves(ir, q, rd, rn, rm,
                        |ir, a, b| { let nb = ir.new_temp(Type::I64); ir.gen_not(Type::I64, nb, b);
                            let d = ir.new_temp(Type::I64); ir.gen_and(Type::I64, d, a, nb); d });
                    true
                }
                (0, 0b10) => { // ORR
                    self.neon_binop_halves(ir, q, rd, rn, rm,
                        |ir, a, b| { let d = ir.new_temp(Type::I64); ir.gen_or(Type::I64, d, a, b); d });
                    true
                }
                (0, 0b11) => { // ORN
                    self.neon_binop_halves(ir, q, rd, rn, rm,
                        |ir, a, b| { let nb = ir.new_temp(Type::I64); ir.gen_not(Type::I64, nb, b);
                            let d = ir.new_temp(Type::I64); ir.gen_or(Type::I64, d, a, nb); d });
                    true
                }
                (1, 0b00) => { // EOR
                    self.neon_binop_halves(ir, q, rd, rn, rm,
                        |ir, a, b| { let d = ir.new_temp(Type::I64); ir.gen_xor(Type::I64, d, a, b); d });
                    true
                }
                (1, 0b01) => { self.neon_bsl(ir, q, rd, rn, rm); true } // BSL
                (1, 0b10) => { self.neon_bit(ir, q, rd, rn, rm); true } // BIT
                (1, 0b11) => { self.neon_bif(ir, q, rd, rn, rm); true } // BIF
                _ => false,
            };
        }

        // Byte-level ops (size=00)
        if size == 0b00 {
            return match (u, opcode) {
                (0, 0b10000) => { self.neon_call2_halves(ir, q, rd, rn, rm, helper_add8); true }
                (1, 0b10000) => { self.neon_call2_halves(ir, q, rd, rn, rm, helper_sub8); true }
                (1, 0b10001) => { self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmeq8); true }
                (1, 0b00111) => { self.neon_call2_halves(ir, q, rd, rn, rm, helper_cmhs8); true }
                (1, 0b10100) => { self.neon_pairwise(ir, q, rd, rn, rm, helper_umaxp8); true }
                (1, 0b10101) => { self.neon_pairwise(ir, q, rd, rn, rm, helper_uminp8); true }
                (0, 0b10111) => { self.neon_pairwise(ir, q, rd, rn, rm, helper_addp8); true }
                _ => false,
            };
        }
        false
    }

    // __CONTINUE_HERE__
    // -- Add/Sub immediate --

    fn trans_ADD_i(
        &mut self, ir: &mut Context, a: &ArgsRriSh,
    ) -> bool {
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

    fn trans_SUB_i(
        &mut self, ir: &mut Context, a: &ArgsRriSh,
    ) -> bool {
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

    fn trans_ADDS_i(
        &mut self, ir: &mut Context, a: &ArgsRriSh,
    ) -> bool {
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

    fn trans_SUBS_i(
        &mut self, ir: &mut Context, a: &ArgsRriSh,
    ) -> bool {
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

    fn trans_AND_i(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf, a.nbit as u32, a.immr as u32, a.imms as u32,
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

    fn trans_ORR_i(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf, a.nbit as u32, a.immr as u32, a.imms as u32,
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

    fn trans_EOR_i(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf, a.nbit as u32, a.immr as u32, a.imms as u32,
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

    fn trans_ANDS_i(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let imm = match decode_bitmask_imm(
            sf, a.nbit as u32, a.immr as u32, a.imms as u32,
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

    fn trans_MOVZ(
        &mut self, ir: &mut Context, a: &ArgsRi16,
    ) -> bool {
        let val = (a.imm as u64) << (a.hw * 16);
        let c = ir.new_const(Type::I64, val);
        self.write_xreg(ir, a.rd, c);
        true
    }

    fn trans_MOVK(
        &mut self, ir: &mut Context, a: &ArgsRi16,
    ) -> bool {
        if a.rd == 31 { return true; }
        let shift = a.hw * 16;
        let mask = !(0xffffu64 << shift);
        let bits = (a.imm as u64) << shift;
        let old = self.xregs[a.rd as usize];
        let m = ir.new_const(Type::I64, mask);
        let t = ir.new_temp(Type::I64);
        ir.gen_and(Type::I64, t, old, m);
        let b = ir.new_const(Type::I64, bits);
        let d = ir.new_temp(Type::I64);
        ir.gen_or(Type::I64, d, t, b);
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_MOVN(
        &mut self, ir: &mut Context, a: &ArgsRi16,
    ) -> bool {
        let sf = a.sf != 0;
        let val = !((a.imm as u64) << (a.hw * 16));
        let val = if !sf { val & 0xffff_ffff } else { val };
        let c = ir.new_const(Type::I64, val);
        self.write_xreg(ir, a.rd, c);
        true
    }

    // -- PC-relative addressing --

    fn trans_ADR(
        &mut self, ir: &mut Context, a: &ArgsPcrel,
    ) -> bool {
        // ADR immediate: immhi = bits[23:5], immlo = bits[30:29]
        let insn = self.opcode;
        let immlo = ((insn >> 29) & 0x3) as i64;
        let immhi_raw = ((insn >> 5) & 0x7ffff) as i32;
        // Sign-extend from 19 bits
        let immhi = ((immhi_raw << 13) >> 13) as i64;
        let imm = (immhi << 2) | immlo;
        let target =
            (self.base.pc_next as i64 + imm) as u64;
        let c = ir.new_const(Type::I64, target);
        self.write_xreg(ir, a.rd, c);
        true
    }

    fn trans_ADRP(
        &mut self, ir: &mut Context, a: &ArgsPcrel,
    ) -> bool {
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

    fn trans_SBFM(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
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
                    let sh1 =
                        ir.new_const(ty, shl_amt as u64);
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

    fn trans_BFM(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let immr = a.immr as u32;
        let imms = a.imms as u32;

        if imms >= immr {
            let len = imms - immr + 1;
            if a.rd == 31 { return true; }
            let dst = self.read_xreg(ir, a.rd);
            let dst = Self::trunc32(ir, dst, sf);
            let d = Self::deposit(
                ir, ty, dst, src, immr, len, sf,
            );
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // BFI
            let len = imms + 1;
            let bits = if sf { 64u32 } else { 32u32 };
            let pos = bits - immr;
            if a.rd == 31 { return true; }
            let dst = self.read_xreg(ir, a.rd);
            let dst = Self::trunc32(ir, dst, sf);
            let d = Self::deposit(
                ir, ty, dst, src, pos, len, sf,
            );
            self.write_xreg_sz(ir, a.rd, d, sf);
        }
        true
    }

    fn trans_UBFM(
        &mut self, ir: &mut Context, a: &ArgsLogicImm,
    ) -> bool {
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
                    if sf { u64::MAX } else { 0xffff_ffff }
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
                    if sf { u64::MAX } else { 0xffff_ffff }
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

    fn trans_B(
        &mut self, ir: &mut Context, _a: &ArgsBranch,
    ) -> bool {
        // imm26 = bits[25:0], sign-extended, *4
        let insn = self.opcode;
        let imm26 = (insn & 0x03ff_ffff) as i32;
        let imm = ((imm26 << 6) >> 6) as i64; // sign-extend
        let target =
            (self.base.pc_next as i64 + imm * 4) as u64;
        self.gen_direct_branch(ir, target, 0);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BL(
        &mut self, ir: &mut Context, _a: &ArgsBranch,
    ) -> bool {
        let insn = self.opcode;
        let imm26 = (insn & 0x03ff_ffff) as i32;
        let imm = ((imm26 << 6) >> 6) as i64;
        let target =
            (self.base.pc_next as i64 + imm * 4) as u64;
        let link = self.base.pc_next + 4;
        let c = ir.new_const(Type::I64, link);
        self.write_xreg(ir, 30, c);
        self.gen_direct_branch(ir, target, 0);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BR(
        &mut self, ir: &mut Context, a: &ArgsBr,
    ) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BLR(
        &mut self, ir: &mut Context, a: &ArgsBr,
    ) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        let link = self.base.pc_next + 4;
        let c = ir.new_const(Type::I64, link);
        self.write_xreg(ir, 30, c);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_RET(
        &mut self, ir: &mut Context, a: &ArgsBr,
    ) -> bool {
        let addr = self.read_xreg(ir, a.rn);
        self.gen_indirect_branch(ir, addr);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_B_cond(
        &mut self, ir: &mut Context, a: &ArgsBcond,
    ) -> bool {
        let target =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
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
        ir.gen_brcond(
            Type::I64, cond_val, zero, Cond::Ne, taken,
        );

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

    fn trans_CBZ(
        &mut self, ir: &mut Context, a: &ArgsCb,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let target =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
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

    fn trans_CBNZ(
        &mut self, ir: &mut Context, a: &ArgsCb,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let target =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
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

    fn trans_TBZ(
        &mut self, ir: &mut Context, a: &ArgsTb,
    ) -> bool {
        // TBZ: bit number is encoded in sf:imm5 from insn
        // The decoder gives us sf and rn; we extract bit
        // from the raw opcode.
        let insn = self.opcode;
        let b5 = (insn >> 31) & 1;
        let b40 = (insn >> 19) & 0x1f;
        let bit = (b5 << 5) | b40;
        let imm14 = ((insn >> 5) & 0x3fff) as i32;
        let offset = ((imm14 << 18) >> 18) as i64 * 4;
        let target =
            (self.base.pc_next as i64 + offset) as u64;
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

    fn trans_TBNZ(
        &mut self, ir: &mut Context, a: &ArgsTb,
    ) -> bool {
        let insn = self.opcode;
        let b5 = (insn >> 31) & 1;
        let b40 = (insn >> 19) & 0x1f;
        let bit = (b5 << 5) | b40;
        let imm14 = ((insn >> 5) & 0x3fff) as i32;
        let offset = ((imm14 << 18) >> 18) as i64 * 4;
        let target =
            (self.base.pc_next as i64 + offset) as u64;
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

    fn trans_ADD_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUB_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ADDS_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src1, b);
        self.gen_nzcv_add_sub(ir, src1, b, d, sf, false);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_SUBS_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src1, b);
        self.gen_nzcv_add_sub(ir, src1, b, d, sf, true);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Add/Sub extended register --

    fn trans_ADD_ext(
        &mut self, ir: &mut Context, a: &ArgsExtReg,
    ) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src2 = self.read_xreg(ir, a.rm);
        let ext = Self::extend_reg(
            ir, src2, a.option, a.imm,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, src1, ext);
        self.write_xreg_sp(ir, a.rd, d);
        true
    }

    fn trans_SUB_ext(
        &mut self, ir: &mut Context, a: &ArgsExtReg,
    ) -> bool {
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src2 = self.read_xreg(ir, a.rm);
        let ext = Self::extend_reg(
            ir, src2, a.option, a.imm,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, src1, ext);
        self.write_xreg_sp(ir, a.rd, d);
        true
    }

    fn trans_ADDS_ext(
        &mut self, ir: &mut Context, a: &ArgsExtReg,
    ) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src2 = self.read_xreg(ir, a.rm);
        let ext = Self::extend_reg(
            ir, src2, a.option, a.imm,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, d, src1, ext);
        self.gen_nzcv_add_sub(
            ir, src1, ext, d, true, false,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_SUBS_ext(
        &mut self, ir: &mut Context, a: &ArgsExtReg,
    ) -> bool {
        let sf = a.sf != 0;
        let src1 = self.read_xreg_sp(ir, a.rn);
        let src2 = self.read_xreg(ir, a.rm);
        let ext = Self::extend_reg(
            ir, src2, a.option, a.imm,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_sub(Type::I64, d, src1, ext);
        self.gen_nzcv_add_sub(
            ir, src1, ext, d, true, true,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Logical shifted register --

    fn trans_AND_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_BIC_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_andc(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ORR_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_or(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ORN_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_orc(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_EOR_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_xor(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_EON_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_eqv(ty, d, src1, b);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_ANDS_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_and(ty, d, src1, b);
        self.gen_nzcv_logic(ir, d, sf);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    fn trans_BICS_r(
        &mut self, ir: &mut Context, a: &ArgsShiftReg,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let src1 = self.read_xreg(ir, a.rn);
        let src1 = Self::trunc32(ir, src1, sf);
        let src2 = self.read_xreg(ir, a.rm);
        let src2 = Self::trunc32(ir, src2, sf);
        let imm6 = (self.opcode >> 10) & 0x3f;
        let b = Self::apply_shift(
            ir, ty, src2, a.shift, imm6 as i64,
        );
        let d = ir.new_temp(ty);
        ir.gen_andc(ty, d, src1, b);
        self.gen_nzcv_logic(ir, d, sf);
        self.write_xreg_sz(ir, a.rd, d, sf);
        true
    }

    // -- Multiply --

    fn trans_MADD(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_MSUB(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_SMADDL(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_UMADDL(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_UMSUBL(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_UMULH(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
        // Xd = (Xn * Xm) >> 64 (unsigned)
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let lo = ir.new_temp(Type::I64);
        let hi = ir.new_temp(Type::I64);
        ir.gen_mulu2(Type::I64, lo, hi, n, m);
        self.write_xreg(ir, a.rd, hi);
        true
    }

    fn trans_SMULH(
        &mut self, ir: &mut Context, a: &ArgsRrrrS,
    ) -> bool {
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

    fn trans_UDIV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let zero = ir.new_const(ty, 0);
        let one = ir.new_const(ty, 1);
        let safe = ir.new_temp(ty);
        ir.gen_movcond(ty, safe, m, zero, one, m, Cond::Eq);
        let quot = ir.new_temp(ty);
        let rem = ir.new_temp(ty);
        ir.gen_divu2(ty, quot, rem, n, zero, safe);
        ir.gen_movcond(
            ty, quot, m, zero, zero, quot, Cond::Eq,
        );
        self.write_xreg_sz(ir, a.rd, quot, sf);
        true
    }

    fn trans_SDIV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let n = self.read_xreg(ir, a.rn);
        let n = Self::trunc32(ir, n, sf);
        let m = self.read_xreg(ir, a.rm);
        let m = Self::trunc32(ir, m, sf);
        let zero = ir.new_const(ty, 0);
        let one = ir.new_const(ty, 1);
        let safe = ir.new_temp(ty);
        ir.gen_movcond(ty, safe, m, zero, one, m, Cond::Eq);
        // Sign-extend dividend into high half
        let bits = if sf { 63u64 } else { 31u64 };
        let sh = ir.new_const(ty, bits);
        let ah = ir.new_temp(ty);
        ir.gen_sar(ty, ah, n, sh);
        let quot = ir.new_temp(ty);
        let rem = ir.new_temp(ty);
        ir.gen_divs2(ty, quot, rem, n, ah, safe);
        ir.gen_movcond(
            ty, quot, m, zero, zero, quot, Cond::Eq,
        );
        self.write_xreg_sz(ir, a.rd, quot, sf);
        true
    }

    // -- Variable shifts --

    fn trans_LSLV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
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

    fn trans_LSRV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
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

    fn trans_ASRV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
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

    fn trans_RORV(
        &mut self, ir: &mut Context, a: &ArgsRrrS,
    ) -> bool {
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

    fn trans_CLZ(
        &mut self, ir: &mut Context, a: &ArgsRrS,
    ) -> bool {
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

    fn trans_RBIT(
        &mut self, _ir: &mut Context, _a: &ArgsRrS,
    ) -> bool {
        false // Complex; rarely needed for CoreMark
    }

    fn trans_REV(
        &mut self, ir: &mut Context, a: &ArgsRrS,
    ) -> bool {
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

    fn trans_REV16(
        &mut self, _ir: &mut Context, _a: &ArgsRrS,
    ) -> bool {
        false
    }

    fn trans_REV32(
        &mut self, _ir: &mut Context, _a: &ArgsRrS,
    ) -> bool {
        false
    }

    // -- Conditional select --

    fn trans_CSEL(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(
            Type::I64, d, cond_val, zero, n, m, Cond::Ne,
        );
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSINC(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let one = ir.new_const(Type::I64, 1);
        let m_inc = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, m_inc, m, one);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(
            Type::I64, d, cond_val, zero,
            n, m_inc, Cond::Ne,
        );
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSINV(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let m_inv = ir.new_temp(Type::I64);
        ir.gen_not(Type::I64, m_inv, m);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(
            Type::I64, d, cond_val, zero,
            n, m_inv, Cond::Ne,
        );
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    fn trans_CSNEG(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let cond_val = self.eval_cond(ir, a.cond);
        let n = self.read_xreg(ir, a.rn);
        let m = self.read_xreg(ir, a.rm);
        let m_neg = ir.new_temp(Type::I64);
        ir.gen_neg(Type::I64, m_neg, m);
        let zero = ir.new_const(Type::I64, 0);
        let d = ir.new_temp(Type::I64);
        ir.gen_movcond(
            Type::I64, d, cond_val, zero,
            n, m_neg, Cond::Ne,
        );
        if sf {
            self.write_xreg(ir, a.rd, d);
        } else {
            self.write_xreg_sz(ir, a.rd, d, false);
        }
        true
    }

    // -- Loads: unsigned immediate offset --

    fn trans_LDR_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRB_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::ub().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRH_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::uw().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSB_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sb().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSH_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sw().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSW_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let offset = a.imm << 2;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sl().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Stores: unsigned immediate offset --

    fn trans_STR_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        true
    }

    fn trans_STRB_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::ub().bits() as u32,
        );
        true
    }

    fn trans_STRH_i(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let offset = a.imm << 1;
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::uw().bits() as u32,
        );
        true
    }

    // -- Loads: register offset --

    fn trans_LDR_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 {
            if sf { 3 } else { 2 }
        } else { 0 };
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, shift,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRB_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, 0,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::ub().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRH_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 1 } else { 0 };
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, shift,
        );
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::uw().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Stores: register offset --

    fn trans_STR_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 {
            if sf { 3 } else { 2 }
        } else { 0 };
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, shift,
        );
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        true
    }

    fn trans_STRB_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, 0,
        );
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::ub().bits() as u32,
        );
        true
    }

    fn trans_STRH_r(
        &mut self, ir: &mut Context, a: &ArgsLdstReg,
    ) -> bool {
        let s = (self.opcode >> 12) & 1;
        let shift = if s != 0 { 1 } else { 0 };
        let addr = self.compute_addr_reg(
            ir, a.rn, a.rm, a.option, shift,
        );
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::uw().bits() as u32,
        );
        true
    }

    // -- PC-relative literal loads --

    fn trans_LDR_lit(
        &mut self, ir: &mut Context, a: &ArgsLdstLit,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::sl() };
        let addr_val =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
        let addr = ir.new_const(Type::I64, addr_val);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDRSW_lit(
        &mut self, ir: &mut Context, a: &ArgsLdstLit,
    ) -> bool {
        let addr_val =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
        let addr = ir.new_const(Type::I64, addr_val);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sl().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Pre/post-index loads/stores --

    fn trans_LDR_pre(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        // Writeback
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_STR_pre(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_LDR_post(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, base, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        // Writeback: base + offset
        let new_base =
            self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    fn trans_STR_post(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let base = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, base, memop.bits() as u32,
        );
        let new_base =
            self.compute_addr_imm(ir, a.rn, a.imm);
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
        self.write_xreg(ir, a.rd, d);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }
    fn trans_LDRSH_pre(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, addr, MemOp::sw().bits() as u32);
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
        self.write_xreg(ir, a.rd, d);
        let wb = self.compute_addr_imm(ir, a.rn, a.imm);
        self.write_xreg_sp(ir, a.rn, wb);
        true
    }
    fn trans_LDRSH_post(&mut self, ir: &mut Context, a: &ArgsLdstImm) -> bool {
        let base = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(Type::I64, d, base, MemOp::sw().bits() as u32);
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

    fn trans_LDP(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::sl() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d1, addr, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d2, addr2, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        true
    }

    fn trans_STP(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, v1, addr, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(
            Type::I64, v2, addr2, memop.bits() as u32,
        );
        true
    }

    fn trans_LDP_pre(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::sl() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d1, addr, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d2, addr2, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_STP_pre(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let addr = self.compute_addr_imm(ir, a.rn, offset);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, v1, addr, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, addr, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(
            Type::I64, v2, addr2, memop.bits() as u32,
        );
        self.write_xreg_sp(ir, a.rn, addr);
        true
    }

    fn trans_LDP_post(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::sl() };
        let size = if sf { 8i64 } else { 4 };
        let base = self.read_xreg_sp(ir, a.rn);
        let d1 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d1, base, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, base, off2);
        let d2 = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d2, addr2, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d1);
        self.write_xreg(ir, a.ra, d2);
        let new_base =
            self.compute_addr_imm(ir, a.rn, offset);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    fn trans_STP_post(
        &mut self, ir: &mut Context, a: &ArgsLdstPair,
    ) -> bool {
        let sf = a.sf != 0;
        let scale = if sf { 3i64 } else { 2 };
        let offset = a.imm << scale;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let size = if sf { 8i64 } else { 4 };
        let base = self.read_xreg_sp(ir, a.rn);
        let v1 = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, v1, base, memop.bits() as u32,
        );
        let off2 = ir.new_const(Type::I64, size as u64);
        let addr2 = ir.new_temp(Type::I64);
        ir.gen_add(Type::I64, addr2, base, off2);
        let v2 = self.read_xreg(ir, a.ra);
        ir.gen_qemu_st(
            Type::I64, v2, addr2, memop.bits() as u32,
        );
        let new_base =
            self.compute_addr_imm(ir, a.rn, offset);
        self.write_xreg_sp(ir, a.rn, new_base);
        true
    }

    // -- System --

    fn trans_NOP(
        &mut self, _ir: &mut Context, _a: &ArgsEmpty,
    ) -> bool {
        true
    }

    fn trans_SVC(
        &mut self, ir: &mut Context, _a: &ArgsSys,
    ) -> bool {
        let pc = ir.new_const(Type::I64, self.base.pc_next);
        ir.gen_mov(Type::I64, self.pc, pc);
        ir.gen_exit_tb(EXCP_ECALL);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_MRS(
        &mut self, ir: &mut Context, a: &ArgsSys,
    ) -> bool {
        // Decode system register from raw opcode.
        let insn = self.opcode;
        let op0 = ((insn >> 19) & 0x1) + 2;
        let op1 = (insn >> 16) & 0x7;
        let crn = (insn >> 12) & 0xf;
        let crm = (insn >> 8) & 0xf;
        let op2 = (insn >> 5) & 0x7;

        // TPIDR_EL0
        if op0 == 3 && op1 == 3 && crn == 13
            && crm == 0 && op2 == 2
        {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(
                Type::I64, v, self.env, TPIDR_EL0_OFFSET,
            );
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // NZCV
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 2 && op2 == 0
        {
            let v = self.nzcv;
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // FPCR
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 4 && op2 == 0
        {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(
                Type::I64, v, self.env, FPCR_OFFSET,
            );
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // FPSR
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 4 && op2 == 1
        {
            let v = ir.new_temp(Type::I64);
            ir.gen_ld(
                Type::I64, v, self.env, FPSR_OFFSET,
            );
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // DCZID_EL0: op0=3, op1=3, CRn=0, CRm=0, op2=7
        // Return DZP=1 (bit4) to disable DC ZVA usage.
        if op0 == 3 && op1 == 3 && crn == 0
            && crm == 0 && op2 == 7
        {
            let v = ir.new_const(Type::I64, 0x10);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        // CTR_EL0: op0=3, op1=3, CRn=0, CRm=0, op2=1
        // Return a reasonable cache geometry.
        if op0 == 3 && op1 == 3 && crn == 0
            && crm == 0 && op2 == 1
        {
            // IminLine=4 (16 words = 64 bytes), DminLine=4,
            // L1Ip=3 (PIPT), bits: 0x80038003
            let v = ir.new_const(Type::I64, 0x80038003);
            self.write_xreg(ir, a.rd, v);
            return true;
        }
        false
    }

    fn trans_MSR(
        &mut self, ir: &mut Context, a: &ArgsSys,
    ) -> bool {
        let insn = self.opcode;
        let op0 = ((insn >> 19) & 0x1) + 2;
        let op1 = (insn >> 16) & 0x7;
        let crn = (insn >> 12) & 0xf;
        let crm = (insn >> 8) & 0xf;
        let op2 = (insn >> 5) & 0x7;
        let val = self.read_xreg(ir, a.rd);

        // TPIDR_EL0
        if op0 == 3 && op1 == 3 && crn == 13
            && crm == 0 && op2 == 2
        {
            ir.gen_st(
                Type::I64, val, self.env,
                TPIDR_EL0_OFFSET,
            );
            return true;
        }
        // NZCV
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 2 && op2 == 0
        {
            ir.gen_mov(Type::I64, self.nzcv, val);
            return true;
        }
        // FPCR
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 4 && op2 == 0
        {
            ir.gen_st(
                Type::I64, val, self.env, FPCR_OFFSET,
            );
            return true;
        }
        // FPSR
        if op0 == 3 && op1 == 3 && crn == 4
            && crm == 4 && op2 == 1
        {
            ir.gen_st(
                Type::I64, val, self.env, FPSR_OFFSET,
            );
            return true;
        }
        false
    }

    // -- Unscaled loads/stores (LDUR/STUR) --

    fn trans_LDUR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STUR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        true
    }

    fn trans_LDURB(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::ub().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STURB(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::ub().bits() as u32,
        );
        true
    }
    fn trans_LDURH(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::uw().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STURH(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, MemOp::uw().bits() as u32,
        );
        true
    }

    fn trans_LDURSW(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sl().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDURSH(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sw().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDURSB(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let addr = self.compute_addr_imm(ir, a.rn, a.imm);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, MemOp::sb().bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    // -- Conditional compare (CCMP/CCMN) --

    fn trans_CCMP(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(
            Type::I64, cond_val, zero, Cond::Ne, taken,
        );
        // Condition false: set NZCV to immediate
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        ir.gen_mov(Type::I64, self.nzcv, imm_c);
        ir.gen_br(done);
        // Condition true: do CMP (SUBS discarding result)
        ir.gen_set_label(taken);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = self.read_xreg(ir, a.rm);
        let b = Self::trunc32(ir, b, sf);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, true);
        ir.gen_set_label(done);
        true
    }

    fn trans_CCMN(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(
            Type::I64, cond_val, zero, Cond::Ne, taken,
        );
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        ir.gen_mov(Type::I64, self.nzcv, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = self.read_xreg(ir, a.rm);
        let b = Self::trunc32(ir, b, sf);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, false);
        ir.gen_set_label(done);
        true
    }

    fn trans_CCMP_i(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let imm5 = a.rm as u64; // rm field holds imm5
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(
            Type::I64, cond_val, zero, Cond::Ne, taken,
        );
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        ir.gen_mov(Type::I64, self.nzcv, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm5);
        let d = ir.new_temp(ty);
        ir.gen_sub(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, true);
        ir.gen_set_label(done);
        true
    }

    fn trans_CCMN_i(
        &mut self, ir: &mut Context, a: &ArgsCsel,
    ) -> bool {
        let sf = a.sf != 0;
        let ty = Self::sf_type(sf);
        let nzcv_imm = (self.opcode & 0xf) as u64;
        let imm5 = a.rm as u64;
        let cond_val = self.eval_cond(ir, a.cond);
        let zero = ir.new_const(Type::I64, 0);
        let taken = ir.new_label();
        let done = ir.new_label();
        ir.gen_brcond(
            Type::I64, cond_val, zero, Cond::Ne, taken,
        );
        let imm_c = ir.new_const(Type::I64, nzcv_imm << 28);
        ir.gen_mov(Type::I64, self.nzcv, imm_c);
        ir.gen_br(done);
        ir.gen_set_label(taken);
        let src = self.read_xreg(ir, a.rn);
        let src = Self::trunc32(ir, src, sf);
        let b = ir.new_const(ty, imm5);
        let d = ir.new_temp(ty);
        ir.gen_add(ty, d, src, b);
        self.gen_nzcv_add_sub(ir, src, b, d, sf, false);
        ir.gen_set_label(done);
        true
    }

    // -- Extract (EXTR) --

    fn trans_EXTR(
        &mut self, ir: &mut Context, a: &ArgsRrrSf,
    ) -> bool {
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

    fn trans_LDAR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDAXR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        // Simplified: treat as regular load (no exclusives)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_LDXR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        // Simplified: treat as regular load (no exclusives
        // in single-threaded mode)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let d = ir.new_temp(Type::I64);
        ir.gen_qemu_ld(
            Type::I64, d, addr, memop.bits() as u32,
        );
        self.write_xreg(ir, a.rd, d);
        true
    }

    fn trans_STLR(
        &mut self, ir: &mut Context, a: &ArgsLdstImm,
    ) -> bool {
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        true
    }

    fn trans_STXR(
        &mut self, ir: &mut Context, a: &ArgsStx,
    ) -> bool {
        // Simplified: always succeeds (single-threaded)
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        // Write 0 (success) to status register Rs
        let zero = ir.new_const(Type::I64, 0);
        self.write_xreg(ir, a.rs, zero);
        true
    }

    fn trans_STLXR(
        &mut self, ir: &mut Context, a: &ArgsStx,
    ) -> bool {
        // Same as STXR — store-release exclusive, always
        // succeeds in single-threaded mode
        let sf = a.sf != 0;
        let memop = if sf { MemOp::uq() } else { MemOp::ul() };
        let addr = self.read_xreg_sp(ir, a.rn);
        let val = self.read_xreg(ir, a.rd);
        ir.gen_qemu_st(
            Type::I64, val, addr, memop.bits() as u32,
        );
        // Write 0 (success) to status register Rs
        let zero = ir.new_const(Type::I64, 0);
        self.write_xreg(ir, a.rs, zero);
        true
    }

    // -- Barriers --

    fn trans_DMB(
        &mut self, _ir: &mut Context, _a: &ArgsSys,
    ) -> bool {
        // Single-threaded: barriers are NOPs
        true
    }
}

