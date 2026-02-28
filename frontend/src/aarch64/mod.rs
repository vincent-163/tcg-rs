//! AArch64 frontend — A64 user-mode instruction translation.

pub mod cpu;
#[allow(dead_code)]
mod insn_decode;
#[allow(function_casts_as_integer)]
mod trans;

use crate::{DisasContextBase, DisasJumpType, TranslatorOps};
use cpu::{
    xreg_offset, NZCV_OFFSET, NUM_XREGS,
    PC_OFFSET, SP_OFFSET,
};
use tcg_core::tb::{EXCP_UNDEF, TB_EXIT_IDX0};
use tcg_core::{Context, TempIdx, Type};

// ---------------------------------------------------------------
// Disassembly context
// ---------------------------------------------------------------

/// AArch64 disassembly context (extends `DisasContextBase`).
pub struct Aarch64DisasContext {
    /// Generic base fields (pc, is_jmp, counters).
    pub base: DisasContextBase,
    /// IR temp for the env pointer (fixed to host RBP).
    pub env: TempIdx,
    /// IR temps for guest GPRs X0-X30 (globals).
    pub xregs: [TempIdx; NUM_XREGS],
    /// IR temp for the guest PC (global).
    pub pc: TempIdx,
    /// IR temp for the stack pointer (global).
    pub sp: TempIdx,
    /// IR temp for NZCV condition flags (global).
    pub nzcv: TempIdx,
    /// Raw instruction word being decoded.
    pub opcode: u32,
    /// Pointer to guest code bytes for fetching.
    pub guest_base: *const u8,
}

impl Aarch64DisasContext {
    /// Create a new context for translating a TB starting
    /// at `pc`.
    pub fn new(pc: u64, guest_base: *const u8) -> Self {
        Self {
            base: DisasContextBase {
                pc_first: pc,
                pc_next: pc,
                is_jmp: DisasJumpType::Next,
                num_insns: 0,
                max_insns: 512,
            },
            env: TempIdx(0),
            xregs: [TempIdx(0); NUM_XREGS],
            pc: TempIdx(0),
            sp: TempIdx(0),
            nzcv: TempIdx(0),
            opcode: 0,
            guest_base,
        }
    }

    /// Fetch a 32-bit instruction at the current PC.
    ///
    /// # Safety
    /// `guest_base + pc_next` must be a valid, readable
    /// 4-byte aligned host address.
    unsafe fn fetch_insn(&self) -> u32 {
        let ptr =
            self.guest_base.add(self.base.pc_next as usize)
                as *const u32;
        ptr.read_unaligned()
    }
}

// ---------------------------------------------------------------
// TranslatorOps implementation
// ---------------------------------------------------------------

/// Marker type for the AArch64 translator.
pub struct Aarch64Translator;

impl TranslatorOps for Aarch64Translator {
    type DisasContext = Aarch64DisasContext;

    fn init_disas_context(
        ctx: &mut Aarch64DisasContext,
        ir: &mut Context,
    ) {
        // Register the env pointer (fixed to host RBP = reg 5).
        ctx.env = ir.new_fixed(Type::I64, 5, "env");

        // Register guest GPRs as globals at known offsets.
        for i in 0..NUM_XREGS {
            ctx.xregs[i] = ir.new_global(
                Type::I64,
                ctx.env,
                xreg_offset(i),
                "xreg",
            );
        }

        // Register guest PC as a global.
        ctx.pc =
            ir.new_global(Type::I64, ctx.env, PC_OFFSET, "pc");

        // Register SP as a global.
        ctx.sp =
            ir.new_global(Type::I64, ctx.env, SP_OFFSET, "sp");

        // Register NZCV as a global.
        ctx.nzcv = ir.new_global(
            Type::I64,
            ctx.env,
            NZCV_OFFSET,
            "nzcv",
        );
    }

    fn tb_start(
        _ctx: &mut Aarch64DisasContext,
        _ir: &mut Context,
    ) {
        // Nothing special for user-mode.
    }

    fn insn_start(
        ctx: &mut Aarch64DisasContext,
        ir: &mut Context,
    ) {
        ir.gen_insn_start(ctx.base.pc_next);
        ctx.base.num_insns += 1;
    }

    fn translate_insn(
        ctx: &mut Aarch64DisasContext,
        ir: &mut Context,
    ) {
        // AArch64 instructions are always 32-bit.
        let insn = unsafe { ctx.fetch_insn() };
        ctx.opcode = insn;

        let decoded = insn_decode::decode(ctx, ir, insn);

        if !decoded && !ctx.try_neon(ir, insn) {
            let pc_val = ctx.base.pc_next;
            let pc_const = ir.new_const(Type::I64, pc_val);
            ir.gen_mov(Type::I64, ctx.pc, pc_const);
            ir.gen_exit_tb(EXCP_UNDEF);
            ctx.base.is_jmp = DisasJumpType::NoReturn;
        }

        ctx.base.pc_next += 4;
    }

    fn tb_stop(
        ctx: &mut Aarch64DisasContext,
        ir: &mut Context,
    ) {
        match ctx.base.is_jmp {
            DisasJumpType::NoReturn => {
                // TB already terminated by the instruction.
            }
            DisasJumpType::Next
            | DisasJumpType::TooMany => {
                let pc_val = ctx.base.pc_next;
                let pc_const =
                    ir.new_const(Type::I64, pc_val);
                ir.gen_mov(Type::I64, ctx.pc, pc_const);
                ir.gen_goto_tb(0);
                ir.gen_exit_tb(TB_EXIT_IDX0);
            }
        }
    }

    fn base(
        ctx: &Aarch64DisasContext,
    ) -> &DisasContextBase {
        &ctx.base
    }

    fn base_mut(
        ctx: &mut Aarch64DisasContext,
    ) -> &mut DisasContextBase {
        &mut ctx.base
    }
}
