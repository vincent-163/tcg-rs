//! AArch64 instruction translation — TCG IR generation.
//!
//! Translates decoded A64 instructions into TCG IR opcodes.
//! Follows the same gen_xxx helper pattern as the RISC-V frontend.

use super::cpu::{
    FPCR_OFFSET, FPSR_OFFSET, NZCV_OFFSET,
    TPIDR_EL0_OFFSET,
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
    let elem = if r == 0 {
        welem
    } else {
        (welem >> r) | (welem << (size - r))
    } & ((1u64 << size) - 1);
    let mut imm = elem;
    let mut sz = size;
    while sz < 64 { imm |= imm << sz; sz <<= 1; }
    if !sf { imm &= 0xffff_ffff; }
    Some(imm)
}

// ── Helpers ──────────────────────────────────────────────

impl Aarch64DisasContext {
    fn read_xreg(
        &self, ir: &mut Context, reg: i64,
    ) -> TempIdx {
        if reg == 31 {
            ir.new_const(Type::I64, 0)
        } else {
            self.xregs[reg as usize]
        }
    }

    fn write_xreg(
        &self, ir: &mut Context, reg: i64, val: TempIdx,
    ) {
        if reg != 31 {
            ir.gen_mov(
                Type::I64, self.xregs[reg as usize], val,
            );
        }
    }

    fn read_xreg_sp(
        &self, ir: &mut Context, reg: i64,
    ) -> TempIdx {
        if reg == 31 { self.sp }
        else { self.xregs[reg as usize] }
    }

    fn write_xreg_sp(
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
    fn compute_addr_imm(
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

}

// ── Decode trait implementation ──────────────────────────

impl Decode<Context> for Aarch64DisasContext {
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
        // imm field from decoder is the full signed offset
        let target =
            (self.base.pc_next as i64 + a.imm) as u64;
        let c = ir.new_const(Type::I64, target);
        self.write_xreg(ir, a.rd, c);
        true
    }

    fn trans_ADRP(
        &mut self, ir: &mut Context, a: &ArgsPcrel,
    ) -> bool {
        let base = self.base.pc_next & !0xfff;
        let offset = a.imm << 12;
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
            let d = ir.new_temp(ty);
            ir.gen_sextract(ty, d, src, immr, len);
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // SBFIZ
            let len = imms + 1;
            let pos = bits - immr;
            let d = ir.new_temp(ty);
            ir.gen_sextract(ty, d, src, 0, len);
            let sh = ir.new_const(ty, pos as u64);
            let r = ir.new_temp(ty);
            ir.gen_shl(ty, r, d, sh);
            // Arithmetic shift right to sign-extend
            ir.gen_sar(ty, r, r, sh);
            // Actually: shift left by pos
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
            let d = ir.new_temp(ty);
            ir.gen_deposit(ty, d, dst, src, immr, len);
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // BFI
            let len = imms + 1;
            let bits = if sf { 64u32 } else { 32u32 };
            let pos = bits - immr;
            if a.rd == 31 { return true; }
            let dst = self.read_xreg(ir, a.rd);
            let dst = Self::trunc32(ir, dst, sf);
            let d = ir.new_temp(ty);
            ir.gen_deposit(ty, d, dst, src, pos, len);
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
            let d = ir.new_temp(ty);
            ir.gen_extract(ty, d, src, immr, len);
            self.write_xreg_sz(ir, a.rd, d, sf);
        } else {
            // UBFIZ / LSL
            let len = imms + 1;
            let pos = bits - immr;
            let d = ir.new_temp(ty);
            let zero_c = ir.new_const(ty, 0);
            ir.gen_deposit(
                ty, d, zero_c, src, pos, len,
            );
            self.write_xreg_sz(ir, a.rd, d, sf);
        }
        true
    }

    // -- Branches --

    fn trans_B(
        &mut self, ir: &mut Context, a: &ArgsBranch,
    ) -> bool {
        let target =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
        self.gen_direct_branch(ir, target, 0);
        self.base.is_jmp = DisasJumpType::NoReturn;
        true
    }

    fn trans_BL(
        &mut self, ir: &mut Context, a: &ArgsBranch,
    ) -> bool {
        let target =
            (self.base.pc_next as i64 + a.imm * 4) as u64;
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
        let d = ir.new_temp(ty);
        ir.gen_divu(ty, d, n, safe);
        ir.gen_movcond(ty, d, m, zero, zero, d, Cond::Eq);
        self.write_xreg_sz(ir, a.rd, d, sf);
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
        let d = ir.new_temp(ty);
        ir.gen_divs(ty, d, n, safe);
        ir.gen_movcond(ty, d, m, zero, zero, d, Cond::Eq);
        self.write_xreg_sz(ir, a.rd, d, sf);
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
        let op0 = ((insn >> 19) & 0x3) + 2;
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
        false
    }

    fn trans_MSR(
        &mut self, ir: &mut Context, a: &ArgsSys,
    ) -> bool {
        let insn = self.opcode;
        let op0 = ((insn >> 19) & 0x3) + 2;
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
}
