//! AArch64 frontend — A64 user-mode instruction translation.

pub mod cpu;
#[allow(dead_code)]
mod insn_decode;
#[allow(function_casts_as_integer)]
mod trans;

use crate::{DisasContextBase, DisasJumpType, TranslatorOps};
use cpu::{
    xreg_offset, NUM_XREGS,
    PC_OFFSET, SP_OFFSET,
    CC_OP_OFFSET, CC_A_OFFSET, CC_B_OFFSET, CC_RESULT_OFFSET,
};
use tcg_core::tb::{EXCP_UNDEF, TB_EXIT_IDX0};
use tcg_core::{Context, TempIdx, Type};

// ---------------------------------------------------------------
// Lazy NZCV state (compile-time tracking within a TB)
// ---------------------------------------------------------------

/// Tracks the last flag-setting operation within the current TB.
///
/// When this is `Some(...)`, we know the cc_op statically and can
/// generate inline condition evaluation instead of a helper call.
/// When `None`, the cc_op may have come from a previous TB (or been
/// invalidated by a label/branch target), and we must use the
/// runtime globals.
#[derive(Clone, Copy)]
pub enum LazyNzcvKind {
    /// Flags from ADD with sf (64-bit if true, 32-bit if false).
    Add { sf: bool },
    /// Flags from SUB with sf.
    Sub { sf: bool },
    /// Flags from logical op with sf. C=0, V=0.
    Logic { sf: bool },
}

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
    /// IR globals for lazy NZCV state (carried across TBs).
    /// When cc_op == CC_OP_EAGER, cc_a holds the packed NZCV value.
    pub cc_op: TempIdx,
    pub cc_a: TempIdx,
    pub cc_b: TempIdx,
    pub cc_result: TempIdx,
    /// Compile-time lazy NZCV tracking within this TB.
    /// None means cc_op is unknown (e.g., at TB start or after a label).
    pub lazy_nzcv: Option<LazyNzcvKind>,
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
            cc_op: TempIdx(0),
            cc_a: TempIdx(0),
            cc_b: TempIdx(0),
            cc_result: TempIdx(0),
            lazy_nzcv: None,
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

        // Register lazy NZCV globals.
        ctx.cc_op = ir.new_global(
            Type::I64,
            ctx.env,
            CC_OP_OFFSET,
            "cc_op",
        );
        ctx.cc_a = ir.new_global(
            Type::I64,
            ctx.env,
            CC_A_OFFSET,
            "cc_a",
        );
        ctx.cc_b = ir.new_global(
            Type::I64,
            ctx.env,
            CC_B_OFFSET,
            "cc_b",
        );
        ctx.cc_result = ir.new_global(
            Type::I64,
            ctx.env,
            CC_RESULT_OFFSET,
            "cc_result",
        );
    }

    fn tb_start(
        _ctx: &mut Aarch64DisasContext,
        _ir: &mut Context,
    ) {
        // lazy_nzcv starts as None — at TB entry, cc_op is unknown
        // (it was set by the previous TB).
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
