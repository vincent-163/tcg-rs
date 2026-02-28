//! TCG IR → LLVM IR translator.

use std::ffi::CString;
use std::ptr;

use super::ffi::*;
use tcg_core::temp::TempKind;
use tcg_core::{Cond, Context, Opcode, TempIdx, Type, OPCODE_DEFS};

const E: *const i8 = c"".as_ptr();

/// Per-TB translation state.
pub struct TbTranslator {
    ctx_ref: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    func: LLVMValueRef,
    // LLVM types
    i1: LLVMTypeRef,
    i8t: LLVMTypeRef,
    i16t: LLVMTypeRef,
    i32t: LLVMTypeRef,
    i64t: LLVMTypeRef,
    i128t: LLVMTypeRef,
    ptr: LLVMTypeRef,
    // Function params
    env: LLVMValueRef,       // ptr %env
    guest_base: LLVMValueRef, // i64 %guest_base
    // Per-temp LLVM alloca slots (indexed by TempIdx)
    temps: Vec<LLVMValueRef>,
    // Per-label basic blocks
    labels: Vec<LLVMBasicBlockRef>,
    // Epilogue block (for exit_tb)
    exit_bb: LLVMBasicBlockRef,
    exit_val: LLVMValueRef, // alloca for return value
    // TB index for encoding exits
    tb_idx: u32,
    // Intrinsic IDs cached
    ctlz_i32: LLVMValueRef,
    ctlz_i64: LLVMValueRef,
    cttz_i32: LLVMValueRef,
    cttz_i64: LLVMValueRef,
    ctpop_i32: LLVMValueRef,
    ctpop_i64: LLVMValueRef,
    bswap_i16: LLVMValueRef,
    bswap_i32: LLVMValueRef,
    bswap_i64: LLVMValueRef,
}

fn get_intrinsic(m: LLVMModuleRef, name: &str, tys: &[LLVMTypeRef]) -> LLVMValueRef {
    unsafe {
        let id = LLVMLookupIntrinsicID(name.as_ptr() as *const i8, name.len());
        LLVMGetIntrinsicDeclaration(m, id, tys.as_ptr(), tys.len())
    }
}

impl TbTranslator {
    pub fn new(
        llvm_ctx: LLVMContextRef,
        ir: &Context,
        func_name: &str,
    ) -> Self {
        unsafe {
            let cname = CString::new(func_name).unwrap();
            let module = LLVMModuleCreateWithNameInContext(cname.as_ptr(), llvm_ctx);

            let i1 = LLVMInt1TypeInContext(llvm_ctx);
            let i8t = LLVMInt8TypeInContext(llvm_ctx);
            let i16t = LLVMInt16TypeInContext(llvm_ctx);
            let i32t = LLVMInt32TypeInContext(llvm_ctx);
            let i64t = LLVMInt64TypeInContext(llvm_ctx);
            let i128t = LLVMInt128TypeInContext(llvm_ctx);
            let ptr = LLVMPointerTypeInContext(llvm_ctx, 0);

            // fn(ptr %env, i64 %guest_base) -> i64
            let mut params = [ptr, i64t];
            let fty = LLVMFunctionType(i64t, params.as_mut_ptr(), 2, 0);
            let func = LLVMAddFunction(module, cname.as_ptr(), fty);

            let builder = LLVMCreateBuilderInContext(llvm_ctx);

            // Entry block with allocas
            let entry_bb = LLVMAppendBasicBlockInContext(llvm_ctx, func, c"entry".as_ptr());
            LLVMPositionBuilderAtEnd(builder, entry_bb);

            let env = LLVMGetParam(func, 0);
            let guest_base = LLVMGetParam(func, 1);

            // Alloca for return value
            let exit_val = LLVMBuildAlloca(builder, i64t, c"exit_val".as_ptr());

            // Allocas for TCG temps
            let nb_temps = ir.nb_temps() as usize;
            let mut temps = Vec::with_capacity(nb_temps);
            for i in 0..nb_temps {
                let tidx = TempIdx(i as u32);
                let temp = ir.temp(tidx);
                let ty = match temp.ty {
                    Type::I32 => i32t,
                    Type::I64 => i64t,
                    _ => i64t,
                };
                let alloca = LLVMBuildAlloca(builder, ty, E);
                temps.push(alloca);
            }

            // Pre-create basic blocks for labels
            let num_labels = ir.labels().len();
            let mut labels = Vec::with_capacity(num_labels);
            for i in 0..num_labels {
                let name = CString::new(format!("L{i}")).unwrap();
                let bb = LLVMAppendBasicBlockInContext(llvm_ctx, func, name.as_ptr());
                labels.push(bb);
            }

            // Exit block
            let exit_bb = LLVMAppendBasicBlockInContext(llvm_ctx, func, c"exit".as_ptr());

            // Cache intrinsics
            let ctlz_i32 = get_intrinsic(module, "llvm.ctlz.i32", &[i32t]);
            let ctlz_i64 = get_intrinsic(module, "llvm.ctlz.i64", &[i64t]);
            let cttz_i32 = get_intrinsic(module, "llvm.cttz.i32", &[i32t]);
            let cttz_i64 = get_intrinsic(module, "llvm.cttz.i64", &[i64t]);
            let ctpop_i32 = get_intrinsic(module, "llvm.ctpop.i32", &[i32t]);
            let ctpop_i64 = get_intrinsic(module, "llvm.ctpop.i64", &[i64t]);
            let bswap_i16 = get_intrinsic(module, "llvm.bswap.i16", &[i16t]);
            let bswap_i32 = get_intrinsic(module, "llvm.bswap.i32", &[i32t]);
            let bswap_i64 = get_intrinsic(module, "llvm.bswap.i64", &[i64t]);

            Self {
                ctx_ref: llvm_ctx, module, builder, func,
                i1, i8t, i16t, i32t, i64t, i128t, ptr,
                env, guest_base, temps, labels,
                exit_bb, exit_val,
                tb_idx: ir.tb_idx,
                ctlz_i32, ctlz_i64, cttz_i32, cttz_i64,
                ctpop_i32, ctpop_i64,
                bswap_i16, bswap_i32, bswap_i64,
            }
        }
    }

    fn llvm_ty(&self, ty: Type) -> LLVMTypeRef {
        match ty {
            Type::I32 => self.i32t,
            Type::I64 => self.i64t,
            _ => self.i64t,
        }
    }

    fn ci(&self, ty: LLVMTypeRef, val: u64) -> LLVMValueRef {
        unsafe { LLVMConstInt(ty, val, 0) }
    }

    /// Load a TCG temp's current value.
    fn load_temp(&self, ir: &Context, tidx: TempIdx) -> LLVMValueRef {
        let temp = ir.temp(tidx);
        let b = self.builder;
        unsafe {
            match temp.kind {
                TempKind::Const => {
                    LLVMConstInt(self.llvm_ty(temp.ty), temp.val, 0)
                }
                TempKind::Global => {
                    // Load from [env + offset]
                    let off = self.ci(self.i64t, temp.mem_offset as u64);
                    let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                    LLVMBuildLoad2(b, self.llvm_ty(temp.ty), p, E)
                }
                TempKind::Fixed => {
                    // Fixed temps (env pointer) - return env as i64
                    LLVMBuildPtrToInt(b, self.env, self.i64t, E)
                }
                _ => {
                    // Ebb/Tb local - load from alloca
                    LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E)
                }
            }
        }
    }

    /// Store a value to a TCG temp.
    fn store_temp(&self, ir: &Context, tidx: TempIdx, val: LLVMValueRef) {
        let temp = ir.temp(tidx);
        let b = self.builder;
        unsafe {
            match temp.kind {
                TempKind::Global => {
                    let off = self.ci(self.i64t, temp.mem_offset as u64);
                    let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                    LLVMBuildStore(b, val, p);
                }
                TempKind::Const | TempKind::Fixed => {}
                _ => {
                    LLVMBuildStore(b, val, self.temps[tidx.0 as usize]);
                }
            }
        }
    }

    /// Sync all globals back to memory (at BB boundaries).
    fn sync_globals(&self, ir: &Context) {
        // Globals are always loaded/stored directly from env, so no-op.
    }

    fn cond_to_pred(cond: Cond) -> LLVMIntPredicate {
        match cond {
            Cond::Eq | Cond::TstEq => LLVMIntPredicate::LLVMIntEQ,
            Cond::Ne | Cond::TstNe => LLVMIntPredicate::LLVMIntNE,
            Cond::Lt => LLVMIntPredicate::LLVMIntSLT,
            Cond::Ge => LLVMIntPredicate::LLVMIntSGE,
            Cond::Le => LLVMIntPredicate::LLVMIntSLE,
            Cond::Gt => LLVMIntPredicate::LLVMIntSGT,
            Cond::Ltu => LLVMIntPredicate::LLVMIntULT,
            Cond::Geu => LLVMIntPredicate::LLVMIntUGE,
            Cond::Leu => LLVMIntPredicate::LLVMIntULE,
            Cond::Gtu => LLVMIntPredicate::LLVMIntUGT,
            _ => LLVMIntPredicate::LLVMIntEQ,
        }
    }

    fn do_exit(&self, exit_code: u64) {
        let encoded = tcg_core::tb::encode_tb_exit(self.tb_idx, exit_code);
        unsafe {
            LLVMBuildStore(self.builder, self.ci(self.i64t, encoded), self.exit_val);
            LLVMBuildBr(self.builder, self.exit_bb);
        }
    }

    /// Guest memory pointer: inttoptr(guest_base + addr)
    fn guest_ptr(&self, addr: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let a64 = LLVMBuildZExt(self.builder, addr, self.i64t, E);
            let sum = LLVMBuildAdd(self.builder, self.guest_base, a64, E);
            LLVMBuildIntToPtr(self.builder, sum, self.ptr, E)
        }
    }

    /// Main translation: walk IR ops and emit LLVM IR.
    /// Returns the LLVM module (caller takes ownership).
    pub fn translate(mut self, ir: &Context) -> LLVMModuleRef {
        let b = self.builder;
        let ops = ir.ops();

        // Jump from entry to first code block
        let code_bb = unsafe {
            let bb = LLVMAppendBasicBlockInContext(self.ctx_ref, self.func, c"code".as_ptr());
            LLVMBuildBr(b, bb);
            LLVMPositionBuilderAtEnd(b, bb);
            bb
        };

        // Load initial values for globals from env
        for i in 0..ir.nb_globals() as usize {
            let tidx = TempIdx(i as u32);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Global {
                let val = self.load_temp(ir, tidx);
                // Store into local alloca for faster access within TB
                unsafe { LLVMBuildStore(b, val, self.temps[i]); }
            }
        }

        let mut terminated = false;

        for op in ops {
            if terminated {
                // After a terminator, we need a new BB target (SetLabel will provide one)
                if op.opc != Opcode::SetLabel {
                    continue;
                }
            }

            let def = &OPCODE_DEFS[op.opc as usize];
            let nb_o = def.nb_oargs as usize;
            let nb_i = def.nb_iargs as usize;

            // Helper closures for common patterns
            macro_rules! oarg { ($n:expr) => { op.args[$n] } }
            macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
            macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }

            // Load input temp values (for non-global access, use local allocas)
            macro_rules! ival {
                ($n:expr) => {{
                    let tidx = iarg!($n);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Const {
                        self.ci(self.llvm_ty(temp.ty), temp.val)
                    } else if temp.kind == TempKind::Fixed {
                        unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) }
                    } else if temp.kind == TempKind::Global {
                        // Read from local alloca (cached)
                        unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) }
                    } else {
                        unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) }
                    }
                }}
            }

            macro_rules! store_out {
                ($n:expr, $val:expr) => {{
                    let tidx = oarg!($n);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        // Write to local alloca
                        unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
                    } else if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                        unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
                    }
                }}
            }

            // Flush globals to env memory before BB boundaries
            macro_rules! flush_globals {
                () => {
                    for i in 0..ir.nb_globals() as usize {
                        let tidx = TempIdx(i as u32);
                        let temp = ir.temp(tidx);
                        if temp.kind == TempKind::Global {
                            unsafe {
                                let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[i], E);
                                let off = self.ci(self.i64t, temp.mem_offset as u64);
                                let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                                LLVMBuildStore(b, v, p);
                            }
                        }
                    }
                }
            }

            unsafe { match op.opc {
                Opcode::Nop | Opcode::InsnStart | Opcode::Discard => {}

                Opcode::Mov => {
                    let v = ival!(0);
                    store_out!(0, v);
                }

                // -- Arithmetic --
                Opcode::Add => { let v = LLVMBuildAdd(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Sub => { let v = LLVMBuildSub(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Mul => { let v = LLVMBuildMul(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Neg => { let v = LLVMBuildNeg(b, ival!(0), E); store_out!(0, v); }
                Opcode::DivS => { let v = LLVMBuildSDiv(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::DivU => { let v = LLVMBuildUDiv(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::RemS => { let v = LLVMBuildSRem(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::RemU => { let v = LLVMBuildURem(b, ival!(0), ival!(1), E); store_out!(0, v); }

                // -- Logic --
                Opcode::And => { let v = LLVMBuildAnd(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Or  => { let v = LLVMBuildOr(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Xor => { let v = LLVMBuildXor(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Not => { let v = LLVMBuildNot(b, ival!(0), E); store_out!(0, v); }

                Opcode::AndC => {
                    let nb = LLVMBuildNot(b, ival!(1), E);
                    let v = LLVMBuildAnd(b, ival!(0), nb, E);
                    store_out!(0, v);
                }
                Opcode::OrC => {
                    let nb = LLVMBuildNot(b, ival!(1), E);
                    let v = LLVMBuildOr(b, ival!(0), nb, E);
                    store_out!(0, v);
                }
                Opcode::Eqv => {
                    let x = LLVMBuildXor(b, ival!(0), ival!(1), E);
                    let v = LLVMBuildNot(b, x, E);
                    store_out!(0, v);
                }
                Opcode::Nand => {
                    let a = LLVMBuildAnd(b, ival!(0), ival!(1), E);
                    let v = LLVMBuildNot(b, a, E);
                    store_out!(0, v);
                }
                Opcode::Nor => {
                    let o = LLVMBuildOr(b, ival!(0), ival!(1), E);
                    let v = LLVMBuildNot(b, o, E);
                    store_out!(0, v);
                }

                // -- Shifts --
                Opcode::Shl => { let v = LLVMBuildShl(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Shr => { let v = LLVMBuildLShr(b, ival!(0), ival!(1), E); store_out!(0, v); }
                Opcode::Sar => { let v = LLVMBuildAShr(b, ival!(0), ival!(1), E); store_out!(0, v); }

                _ => { self.translate_op_part2(ir, op, &mut terminated); }
            }}
        }

        // If last op didn't terminate, flush and return 0
        if !terminated {
            for i in 0..ir.nb_globals() as usize {
                let tidx = TempIdx(i as u32);
                let temp = ir.temp(tidx);
                if temp.kind == TempKind::Global {
                    unsafe {
                        let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[i], E);
                        let off = self.ci(self.i64t, temp.mem_offset as u64);
                        let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                        LLVMBuildStore(b, v, p);
                    }
                }
            }
            unsafe {
                let zero = self.ci(self.i64t, 0);
                LLVMBuildStore(b, zero, self.exit_val);
                LLVMBuildBr(b, self.exit_bb);
            }
        }

        // Build exit block
        unsafe {
            LLVMPositionBuilderAtEnd(b, self.exit_bb);
            let rv = LLVMBuildLoad2(b, self.i64t, self.exit_val, E);
            LLVMBuildRet(b, rv);
            LLVMDisposeBuilder(b);
        }

        self.module
    }

    /// Part 2 of opcode translation: rotates, type conversions, memory, control flow.
    fn translate_op_part2(
        &self,
        ir: &Context,
        op: &tcg_core::Op,
        terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;

        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }

        macro_rules! ival {
            ($n:expr) => {{
                let tidx = iarg!($n);
                let temp = ir.temp(tidx);
                if temp.kind == TempKind::Const {
                    self.ci(self.llvm_ty(temp.ty), temp.val)
                } else if temp.kind == TempKind::Fixed {
                    unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) }
                } else {
                    unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) }
                }
            }}
        }

        macro_rules! store_out {
            ($n:expr, $val:expr) => {{
                let tidx = oarg!($n);
                let temp = ir.temp(tidx);
                if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                    unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
                }
            }}
        }

        macro_rules! flush_globals {
            () => {
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        unsafe {
                            let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[i], E);
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            LLVMBuildStore(b, v, p);
                        }
                    }
                }
            }
        }

        unsafe { match op.opc {
            // -- Rotates --
            Opcode::RotL => {
                let a = ival!(0); let sh = ival!(1);
                let bits = self.ci(self.llvm_ty(op.op_type), op.op_type.size_bits() as u64);
                let rsh = LLVMBuildSub(b, bits, sh, E);
                let l = LLVMBuildShl(b, a, sh, E);
                let r = LLVMBuildLShr(b, a, rsh, E);
                let v = LLVMBuildOr(b, l, r, E);
                store_out!(0, v);
            }
            Opcode::RotR => {
                let a = ival!(0); let sh = ival!(1);
                let bits = self.ci(self.llvm_ty(op.op_type), op.op_type.size_bits() as u64);
                let lsh = LLVMBuildSub(b, bits, sh, E);
                let r = LLVMBuildLShr(b, a, sh, E);
                let l = LLVMBuildShl(b, a, lsh, E);
                let v = LLVMBuildOr(b, l, r, E);
                store_out!(0, v);
            }

            // -- Type conversions --
            Opcode::ExtI32I64 => {
                let v = LLVMBuildSExt(b, ival!(0), self.i64t, E);
                store_out!(0, v);
            }
            Opcode::ExtUI32I64 => {
                let v = LLVMBuildZExt(b, ival!(0), self.i64t, E);
                store_out!(0, v);
            }
            Opcode::ExtrlI64I32 => {
                let v = LLVMBuildTrunc(b, ival!(0), self.i32t, E);
                store_out!(0, v);
            }
            Opcode::ExtrhI64I32 => {
                let sh = LLVMBuildLShr(b, ival!(0), self.ci(self.i64t, 32), E);
                let v = LLVMBuildTrunc(b, sh, self.i32t, E);
                store_out!(0, v);
            }

            _ => { self.translate_op_part3(ir, op, terminated); }
        }}
    }

    /// Part 3: comparisons, memory, setcond
    fn translate_op_part3(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const { self.ci(self.llvm_ty(temp.ty), temp.val) }
            else if temp.kind == TempKind::Fixed { unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) } }
            else { unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) } }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
            }
        }}}

        unsafe { match op.opc {
            Opcode::SetCond => {
                let cond = Cond::from_u8(carg!(0) as u8);
                let (a, b_val) = (ival!(0), ival!(1));
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, a, b_val, E);
                    let zero = self.ci(self.llvm_ty(op.op_type), 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a, b_val, E)
                };
                let v = LLVMBuildZExt(b, cmp, self.llvm_ty(op.op_type), E);
                store_out!(0, v);
            }
            Opcode::NegSetCond => {
                let cond = Cond::from_u8(carg!(0) as u8);
                let (a, b_val) = (ival!(0), ival!(1));
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, a, b_val, E);
                    let zero = self.ci(self.llvm_ty(op.op_type), 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a, b_val, E)
                };
                let ext = LLVMBuildZExt(b, cmp, self.llvm_ty(op.op_type), E);
                let v = LLVMBuildNeg(b, ext, E);
                store_out!(0, v);
            }
            Opcode::MovCond => {
                let cond = Cond::from_u8(carg!(0) as u8);
                let (c1, c2) = (ival!(0), ival!(1));
                let (v1, v2) = (ival!(2), ival!(3));
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, c1, c2, E);
                    let zero = self.ci(self.llvm_ty(op.op_type), 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), c1, c2, E)
                };
                let v = LLVMBuildSelect(b, cmp, v1, v2, E);
                store_out!(0, v);
            }

            // -- Host memory load/store (CPUState fields) --
            Opcode::Ld | Opcode::Ld8U | Opcode::Ld8S | Opcode::Ld16U
            | Opcode::Ld16S | Opcode::Ld32U | Opcode::Ld32S => {
                let base = ival!(0);
                let offset = carg!(0) as i64;
                let base_ptr = LLVMBuildIntToPtr(b, base, self.ptr, E);
                let off = self.ci(self.i64t, offset as u64);
                let p = LLVMBuildGEP2(b, self.i8t, base_ptr, [off].as_ptr(), 1, E);
                let (load_ty, sext) = match op.opc {
                    Opcode::Ld8U => (self.i8t, false),
                    Opcode::Ld8S => (self.i8t, true),
                    Opcode::Ld16U => (self.i16t, false),
                    Opcode::Ld16S => (self.i16t, true),
                    Opcode::Ld32U => (self.i32t, false),
                    Opcode::Ld32S => (self.i32t, true),
                    _ => (self.llvm_ty(op.op_type), false),
                };
                let raw = LLVMBuildLoad2(b, load_ty, p, E);
                let dst_ty = self.llvm_ty(ir.temp(oarg!(0)).ty);
                let v = if load_ty == dst_ty { raw }
                    else if sext { LLVMBuildSExt(b, raw, dst_ty, E) }
                    else { LLVMBuildZExt(b, raw, dst_ty, E) };
                store_out!(0, v);
            }
            Opcode::St | Opcode::St8 | Opcode::St16 | Opcode::St32 => {
                let val = ival!(0);
                let base = ival!(1);
                let offset = carg!(0) as i64;
                let base_ptr = LLVMBuildIntToPtr(b, base, self.ptr, E);
                let off = self.ci(self.i64t, offset as u64);
                let p = LLVMBuildGEP2(b, self.i8t, base_ptr, [off].as_ptr(), 1, E);
                let store_val = match op.opc {
                    Opcode::St8 => LLVMBuildTrunc(b, val, self.i8t, E),
                    Opcode::St16 => LLVMBuildTrunc(b, val, self.i16t, E),
                    Opcode::St32 => LLVMBuildTrunc(b, val, self.i32t, E),
                    _ => val,
                };
                LLVMBuildStore(b, store_val, p);
            }

            _ => { self.translate_op_part4(ir, op, terminated); }
        }}
    }

    /// Part 4: guest memory, control flow, calls, misc
    fn translate_op_part4(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const { self.ci(self.llvm_ty(temp.ty), temp.val) }
            else if temp.kind == TempKind::Fixed { unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) } }
            else { unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) } }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
            }
        }}}
        macro_rules! flush_globals {
            () => {
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        unsafe {
                            let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[i], E);
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            LLVMBuildStore(b, v, p);
                        }
                    }
                }
            }
        }

        unsafe { match op.opc {
            // -- Guest memory --
            Opcode::QemuLd => {
                let addr = ival!(0);
                let memop = carg!(0) as u16;
                let size = memop & 0x3;
                let sign = memop & 4 != 0;
                let p = self.guest_ptr(addr);
                let (load_ty, sext) = match (size, sign) {
                    (0, false) => (self.i8t, false),
                    (0, true) => (self.i8t, true),
                    (1, false) => (self.i16t, false),
                    (1, true) => (self.i16t, true),
                    (2, false) => (self.i32t, false),
                    (2, true) => (self.i32t, true),
                    _ => (self.i64t, false),
                };
                let raw = LLVMBuildLoad2(b, load_ty, p, E);
                let dst_ty = self.llvm_ty(ir.temp(oarg!(0)).ty);
                let v = if load_ty == dst_ty { raw }
                    else if sext { LLVMBuildSExt(b, raw, dst_ty, E) }
                    else { LLVMBuildZExt(b, raw, dst_ty, E) };
                store_out!(0, v);
            }
            Opcode::QemuSt => {
                let val = ival!(0);
                let addr = ival!(1);
                let memop = carg!(0) as u16;
                let size = memop & 0x3;
                let p = self.guest_ptr(addr);
                let store_val = match size {
                    0 => LLVMBuildTrunc(b, val, self.i8t, E),
                    1 => LLVMBuildTrunc(b, val, self.i16t, E),
                    2 => LLVMBuildTrunc(b, val, self.i32t, E),
                    _ => val,
                };
                LLVMBuildStore(b, store_val, p);
            }

            _ => { self.translate_op_part5(ir, op, terminated); }
        }}
    }

    /// Part 5: control flow, calls, bit ops, misc
    fn translate_op_part5(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const { self.ci(self.llvm_ty(temp.ty), temp.val) }
            else if temp.kind == TempKind::Fixed { unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) } }
            else { unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) } }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
            }
        }}}
        macro_rules! flush_globals {
            () => {
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        unsafe {
                            let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[i], E);
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            LLVMBuildStore(b, v, p);
                        }
                    }
                }
            }
        }

        unsafe { match op.opc {
            // -- Control flow --
            Opcode::Br => {
                let label_id = carg!(0);
                flush_globals!();
                LLVMBuildBr(b, self.labels[label_id as usize]);
                *terminated = true;
            }
            Opcode::BrCond => {
                let (a, bv) = (ival!(0), ival!(1));
                let cond = Cond::from_u8(carg!(0) as u8);
                let label_id = carg!(1);
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, a, bv, E);
                    let zero = self.ci(self.llvm_ty(op.op_type), 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a, bv, E)
                };
                flush_globals!();
                let fall_bb = LLVMAppendBasicBlockInContext(
                    self.ctx_ref, self.func, c"fall".as_ptr(),
                );
                LLVMBuildCondBr(b, cmp, self.labels[label_id as usize], fall_bb);
                LLVMPositionBuilderAtEnd(b, fall_bb);
                // Reload globals after branch
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        let off = self.ci(self.i64t, temp.mem_offset as u64);
                        let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                        let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), p, E);
                        LLVMBuildStore(b, v, self.temps[i]);
                    }
                }
            }
            Opcode::SetLabel => {
                let label_id = carg!(0);
                let target_bb = self.labels[label_id as usize];
                // If previous block wasn't terminated, branch to this label
                if !*terminated {
                    flush_globals!();
                    LLVMBuildBr(b, target_bb);
                }
                LLVMPositionBuilderAtEnd(b, target_bb);
                *terminated = false;
                // Reload globals from env
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        let off = self.ci(self.i64t, temp.mem_offset as u64);
                        let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                        let v = LLVMBuildLoad2(b, self.llvm_ty(temp.ty), p, E);
                        LLVMBuildStore(b, v, self.temps[i]);
                    }
                }
            }
            Opcode::ExitTb => {
                let val = carg!(0) as u64;
                flush_globals!();
                self.do_exit(val);
                *terminated = true;
            }
            Opcode::GotoTb => {
                // No chaining in LLVM backend; use NOCHAIN exit
                flush_globals!();
                self.do_exit(tcg_core::tb::TB_EXIT_NOCHAIN);
                *terminated = true;
            }
            Opcode::GotoPtr => {
                flush_globals!();
                self.do_exit(tcg_core::tb::TB_EXIT_NOCHAIN);
                *terminated = true;
            }

            _ => { self.translate_op_part6(ir, op, terminated); }
        }}
    }

    /// Part 6: calls, bit ops, multiply/divide widening, misc
    fn translate_op_part6(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const {
                self.ci(self.llvm_ty(temp.ty), temp.val)
            } else if temp.kind == TempKind::Fixed {
                unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) }
            } else {
                unsafe { LLVMBuildLoad2(
                    b, self.llvm_ty(temp.ty),
                    self.temps[tidx.0 as usize], E,
                ) }
            }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const
                && temp.kind != TempKind::Fixed
            {
                unsafe {
                    LLVMBuildStore(
                        b, $val,
                        self.temps[tidx.0 as usize],
                    );
                }
            }
        }}}

        unsafe { match op.opc {
            // -- Call --
            Opcode::Call => {
                // Flush globals before call
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        let v = LLVMBuildLoad2(
                            b, self.llvm_ty(temp.ty),
                            self.temps[i], E,
                        );
                        let off = self.ci(
                            self.i64t,
                            temp.mem_offset as u64,
                        );
                        let p = LLVMBuildGEP2(
                            b, self.i8t, self.env,
                            [off].as_ptr(), 1, E,
                        );
                        LLVMBuildStore(b, v, p);
                    }
                }
                // func_addr = cargs[1] << 32 | cargs[0]
                let lo = carg!(0) as u64;
                let hi = carg!(1) as u64;
                let func_addr = (hi << 32) | lo;
                // Build args: first input is always env ptr
                let nb_call_iargs = nb_i;
                let mut args = Vec::with_capacity(nb_call_iargs);
                for i in 0..nb_call_iargs {
                    args.push(ival!(i));
                }
                // Create function type
                let mut param_tys: Vec<LLVMTypeRef> =
                    args.iter().map(|_| self.i64t).collect();
                let fty = LLVMFunctionType(
                    self.i64t,
                    param_tys.as_mut_ptr(),
                    param_tys.len() as u32,
                    0,
                );
                let fptr = LLVMConstInt(
                    self.i64t, func_addr, 0,
                );
                let fptr = LLVMBuildIntToPtr(
                    b, fptr, self.ptr, E,
                );
                let ret = LLVMBuildCall2(
                    b, fty, fptr,
                    args.as_ptr(), args.len() as u32, E,
                );
                store_out!(0, ret);
                // Reload globals after call
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        let off = self.ci(
                            self.i64t,
                            temp.mem_offset as u64,
                        );
                        let p = LLVMBuildGEP2(
                            b, self.i8t, self.env,
                            [off].as_ptr(), 1, E,
                        );
                        let v = LLVMBuildLoad2(
                            b, self.llvm_ty(temp.ty), p, E,
                        );
                        LLVMBuildStore(b, v, self.temps[i]);
                    }
                }
            }

            // -- Bit counting --
            Opcode::Clz => {
                let a = ival!(0);
                let is64 = op.op_type == Type::I64;
                let intr = if is64 {
                    self.ctlz_i64
                } else {
                    self.ctlz_i32
                };
                let false_val = self.ci(self.i1, 0);
                let args = [a, false_val];
                let fty = LLVMFunctionType(
                    self.llvm_ty(op.op_type),
                    [self.llvm_ty(op.op_type), self.i1]
                        .as_mut_ptr(),
                    2, 0,
                );
                let v = LLVMBuildCall2(
                    b, fty, intr,
                    args.as_ptr(), 2, E,
                );
                store_out!(0, v);
            }
            Opcode::Ctz => {
                let a = ival!(0);
                let is64 = op.op_type == Type::I64;
                let intr = if is64 {
                    self.cttz_i64
                } else {
                    self.cttz_i32
                };
                let false_val = self.ci(self.i1, 0);
                let args = [a, false_val];
                let fty = LLVMFunctionType(
                    self.llvm_ty(op.op_type),
                    [self.llvm_ty(op.op_type), self.i1]
                        .as_mut_ptr(),
                    2, 0,
                );
                let v = LLVMBuildCall2(
                    b, fty, intr,
                    args.as_ptr(), 2, E,
                );
                store_out!(0, v);
            }
            Opcode::CtPop => {
                let a = ival!(0);
                let is64 = op.op_type == Type::I64;
                let intr = if is64 {
                    self.ctpop_i64
                } else {
                    self.ctpop_i32
                };
                let fty = LLVMFunctionType(
                    self.llvm_ty(op.op_type),
                    [self.llvm_ty(op.op_type)].as_mut_ptr(),
                    1, 0,
                );
                let v = LLVMBuildCall2(
                    b, fty, intr,
                    [a].as_ptr(), 1, E,
                );
                store_out!(0, v);
            }

            _ => {
                self.translate_op_part7(ir, op, terminated);
            }
        }}
    }

    /// Part 7: bswap, extract/deposit, widening mul/div, carry ops, misc
    fn translate_op_part7(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! carg { ($n:expr) => { op.args[nb_o + nb_i + $n].0 } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const { self.ci(self.llvm_ty(temp.ty), temp.val) }
            else if temp.kind == TempKind::Fixed { unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) } }
            else { unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) } }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
            }
        }}}

        unsafe { match op.opc {
            // -- Byte swap --
            Opcode::Bswap16 => {
                let a = ival!(0);
                let tr = LLVMBuildTrunc(b, a, self.i16t, E);
                let fty = LLVMFunctionType(self.i16t, [self.i16t].as_mut_ptr(), 1, 0);
                let sw = LLVMBuildCall2(b, fty, self.bswap_i16, [tr].as_ptr(), 1, E);
                let v = LLVMBuildZExt(b, sw, self.llvm_ty(op.op_type), E);
                store_out!(0, v);
            }
            Opcode::Bswap32 => {
                let a = ival!(0);
                let tr = LLVMBuildTrunc(b, a, self.i32t, E);
                let fty = LLVMFunctionType(self.i32t, [self.i32t].as_mut_ptr(), 1, 0);
                let sw = LLVMBuildCall2(b, fty, self.bswap_i32, [tr].as_ptr(), 1, E);
                let v = LLVMBuildZExt(b, sw, self.llvm_ty(op.op_type), E);
                store_out!(0, v);
            }
            Opcode::Bswap64 => {
                let a = ival!(0);
                let fty = LLVMFunctionType(self.i64t, [self.i64t].as_mut_ptr(), 1, 0);
                let v = LLVMBuildCall2(b, fty, self.bswap_i64, [a].as_ptr(), 1, E);
                store_out!(0, v);
            }

            // -- Bit extract/deposit --
            Opcode::Extract => {
                let a = ival!(0);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let ty = self.llvm_ty(op.op_type);
                let sh = LLVMBuildLShr(b, a, self.ci(ty, ofs), E);
                let mask = self.ci(ty, (1u64 << len).wrapping_sub(1));
                let v = LLVMBuildAnd(b, sh, mask, E);
                store_out!(0, v);
            }
            Opcode::SExtract => {
                let a = ival!(0);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let shl_amt = bits - len - ofs;
                let sar_amt = bits - len;
                let sh = LLVMBuildShl(b, a, self.ci(ty, shl_amt), E);
                let v = LLVMBuildAShr(b, sh, self.ci(ty, sar_amt), E);
                store_out!(0, v);
            }
            Opcode::Deposit => {
                let a = ival!(0);
                let val = ival!(1);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let ty = self.llvm_ty(op.op_type);
                let mask = self.ci(ty, ((1u64 << len).wrapping_sub(1)) << ofs);
                let nmask = LLVMBuildNot(b, mask, E);
                let cleared = LLVMBuildAnd(b, a, nmask, E);
                let shifted = LLVMBuildShl(b, val, self.ci(ty, ofs), E);
                let masked = LLVMBuildAnd(b, shifted, mask, E);
                let v = LLVMBuildOr(b, cleared, masked, E);
                store_out!(0, v);
            }
            Opcode::Extract2 => {
                let ah = ival!(0);
                let al = ival!(1);
                let shr = carg!(0) as u64;
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let lo = LLVMBuildLShr(b, al, self.ci(ty, shr), E);
                let hi = LLVMBuildShl(b, ah, self.ci(ty, bits - shr), E);
                let v = LLVMBuildOr(b, hi, lo, E);
                store_out!(0, v);
            }

            _ => { self.translate_op_part8(ir, op, terminated); }
        }}
    }

    /// Part 8: widening mul/div, carry arithmetic, misc
    fn translate_op_part8(
        &self, ir: &Context, op: &tcg_core::Op, terminated: &mut bool,
    ) {
        let b = self.builder;
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let nb_i = def.nb_iargs as usize;
        macro_rules! oarg { ($n:expr) => { op.args[$n] } }
        macro_rules! iarg { ($n:expr) => { op.args[nb_o + $n] } }
        macro_rules! ival { ($n:expr) => {{
            let tidx = iarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind == TempKind::Const { self.ci(self.llvm_ty(temp.ty), temp.val) }
            else if temp.kind == TempKind::Fixed { unsafe { LLVMBuildPtrToInt(b, self.env, self.i64t, E) } }
            else { unsafe { LLVMBuildLoad2(b, self.llvm_ty(temp.ty), self.temps[tidx.0 as usize], E) } }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                unsafe { LLVMBuildStore(b, $val, self.temps[tidx.0 as usize]); }
            }
        }}}

        unsafe { match op.opc {
            // Widening multiply: two outputs (lo, hi)
            Opcode::MulU2 => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let a = LLVMBuildZExt(b, ival!(0), wide_ty, E);
                let bv = LLVMBuildZExt(b, ival!(1), wide_ty, E);
                let wide = LLVMBuildMul(b, a, bv, E);
                let lo = LLVMBuildTrunc(b, wide, ty, E);
                let sh = LLVMBuildLShr(b, wide, self.ci(wide_ty, bits), E);
                let hi = LLVMBuildTrunc(b, sh, ty, E);
                store_out!(0, lo);
                store_out!(1, hi);
            }
            Opcode::MulS2 => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let a = LLVMBuildSExt(b, ival!(0), wide_ty, E);
                let bv = LLVMBuildSExt(b, ival!(1), wide_ty, E);
                let wide = LLVMBuildMul(b, a, bv, E);
                let lo = LLVMBuildTrunc(b, wide, ty, E);
                let sh = LLVMBuildAShr(b, wide, self.ci(wide_ty, bits), E);
                let hi = LLVMBuildTrunc(b, sh, ty, E);
                store_out!(0, lo);
                store_out!(1, hi);
            }
            Opcode::MulUH => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let a = LLVMBuildZExt(b, ival!(0), wide_ty, E);
                let bv = LLVMBuildZExt(b, ival!(1), wide_ty, E);
                let wide = LLVMBuildMul(b, a, bv, E);
                let sh = LLVMBuildLShr(b, wide, self.ci(wide_ty, bits), E);
                let v = LLVMBuildTrunc(b, sh, ty, E);
                store_out!(0, v);
            }
            Opcode::MulSH => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let a = LLVMBuildSExt(b, ival!(0), wide_ty, E);
                let bv = LLVMBuildSExt(b, ival!(1), wide_ty, E);
                let wide = LLVMBuildMul(b, a, bv, E);
                let sh = LLVMBuildAShr(b, wide, self.ci(wide_ty, bits), E);
                let v = LLVMBuildTrunc(b, sh, ty, E);
                store_out!(0, v);
            }

            // Double-width division: (in_lo, in_hi, divisor) -> (quot, rem)
            Opcode::DivU2 => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let lo = LLVMBuildZExt(b, ival!(0), wide_ty, E);
                let hi = LLVMBuildZExt(b, ival!(1), wide_ty, E);
                let hi_sh = LLVMBuildShl(b, hi, self.ci(wide_ty, bits), E);
                let dividend = LLVMBuildOr(b, hi_sh, lo, E);
                let divisor = LLVMBuildZExt(b, ival!(2), wide_ty, E);
                let quot_w = LLVMBuildUDiv(b, dividend, divisor, E);
                let rem_w = LLVMBuildURem(b, dividend, divisor, E);
                let quot = LLVMBuildTrunc(b, quot_w, ty, E);
                let rem = LLVMBuildTrunc(b, rem_w, ty, E);
                store_out!(0, quot);
                store_out!(1, rem);
            }
            Opcode::DivS2 => {
                let ty = self.llvm_ty(op.op_type);
                let bits = op.op_type.size_bits() as u64;
                let wide_ty = if bits == 32 { self.i64t } else { self.i128t };
                let lo = LLVMBuildZExt(b, ival!(0), wide_ty, E);
                let hi = LLVMBuildSExt(b, ival!(1), wide_ty, E);
                let hi_sh = LLVMBuildShl(b, hi, self.ci(wide_ty, bits), E);
                let dividend = LLVMBuildOr(b, hi_sh, lo, E);
                let divisor = LLVMBuildSExt(b, ival!(2), wide_ty, E);
                let quot_w = LLVMBuildSDiv(b, dividend, divisor, E);
                let rem_w = LLVMBuildSRem(b, dividend, divisor, E);
                let quot = LLVMBuildTrunc(b, quot_w, ty, E);
                let rem = LLVMBuildTrunc(b, rem_w, ty, E);
                store_out!(0, quot);
                store_out!(1, rem);
            }

            // Memory barrier - fence (no-op for single-threaded guest)
            Opcode::Mb => {}

            // Carry arithmetic - stub as simple add/sub (sufficient for most guests)
            Opcode::AddCO | Opcode::AddC1O => {
                let v = LLVMBuildAdd(b, ival!(0), ival!(1), E);
                store_out!(0, v);
            }
            Opcode::SubBO | Opcode::SubB1O => {
                let v = LLVMBuildSub(b, ival!(0), ival!(1), E);
                store_out!(0, v);
            }
            Opcode::AddCI | Opcode::AddCIO | Opcode::SubBI | Opcode::SubBIO => {
                // These need carry input - stub as basic op for now
                let v = LLVMBuildAdd(b, ival!(0), ival!(1), E);
                store_out!(0, v);
            }

            // Plugin callbacks - no-op
            Opcode::PluginCb | Opcode::PluginMemCb => {}

            // Vector ops - not yet supported, panic
            _ if op.opc as u8 >= Opcode::MovVec as u8 => {
                panic!("LLVM backend: vector op {:?} not supported", op.opc);
            }

            _ => {
                panic!("LLVM backend: unhandled opcode {:?}", op.opc);
            }
        }}
    }
}