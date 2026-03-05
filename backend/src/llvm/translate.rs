//! TCG IR → LLVM IR translator.

use std::collections::HashMap;
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
    // TB pointer for encoding exits
    tb_ptr: usize,
    // AOT direct linking: peer TB functions in the same module
    aot_peers: HashMap<u64, LLVMValueRef>, // target_pc → declared function
    pc_temp: Option<TempIdx>,               // which temp is the PC register
    last_pc_const: Option<u64>,             // last constant written to PC
    peer_fty: LLVMTypeRef,                  // fn(ptr, i64) -> i64
    // AOT dispatch super-function for indirect jumps
    aot_dispatch: Option<LLVMValueRef>,     // @aot_dispatch declaration
    aot_dispatch_cache: Option<LLVMValueRef>, // per-TB cache for aot_dispatch
    // AOT helper functions: addr → (declared function, function type)
    aot_helpers: HashMap<u64, (LLVMValueRef, LLVMTypeRef)>,
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
    // TBAA metadata for alias analysis
    tbaa_kind: u32,           // metadata kind ID for "tbaa"
    tbaa_cpu: LLVMValueRef,   // TBAA access tag for CPUState
    tbaa_guest: LLVMValueRef, // TBAA access tag for guest mem
    // Profiling counter address (optional)
    profile_counter: Option<u64>,
}

fn get_intrinsic(m: LLVMModuleRef, name: &str, tys: &[LLVMTypeRef]) -> LLVMValueRef {
    unsafe {
        let id = LLVMLookupIntrinsicID(name.as_ptr() as *const i8, name.len());
        LLVMGetIntrinsicDeclaration(m, id, tys.as_ptr(), tys.len())
    }
}

/// Build TBAA metadata tree with two disjoint access types.
/// Returns (tbaa_kind_id, cpu_access_tag, guest_access_tag).
fn build_tbaa(ctx: LLVMContextRef) -> (u32, LLVMValueRef, LLVMValueRef) {
    unsafe {
        let kind = LLVMGetMDKindIDInContext(
            ctx, c"tbaa".as_ptr(), 4,
        );
        // Root node: !{!"tcg-rs tbaa"}
        let root_str = LLVMMDStringInContext2(
            ctx, c"tcg-rs tbaa".as_ptr(), 11,
        );
        let root = LLVMMDNodeInContext2(
            ctx, [root_str].as_ptr(), 1,
        );
        // Type descriptors under root (disjoint siblings)
        // CPUState: !{!"cpustate", root, i64 0}
        let cpu_str = LLVMMDStringInContext2(
            ctx, c"cpustate".as_ptr(), 8,
        );
        let zero = LLVMValueAsMetadata(
            LLVMConstInt(LLVMInt64TypeInContext(ctx), 0, 0),
        );
        let cpu_ty = LLVMMDNodeInContext2(
            ctx, [cpu_str, root, zero].as_ptr(), 3,
        );
        // Guest mem: !{!"guest", root, i64 0}
        let guest_str = LLVMMDStringInContext2(
            ctx, c"guest".as_ptr(), 5,
        );
        let guest_ty = LLVMMDNodeInContext2(
            ctx, [guest_str, root, zero].as_ptr(), 3,
        );
        // Access tags: !{type, type, i64 0}
        let cpu_tag = LLVMMDNodeInContext2(
            ctx, [cpu_ty, cpu_ty, zero].as_ptr(), 3,
        );
        let guest_tag = LLVMMDNodeInContext2(
            ctx, [guest_ty, guest_ty, zero].as_ptr(), 3,
        );
        let cpu_val = LLVMMetadataAsValue(ctx, cpu_tag);
        let guest_val = LLVMMetadataAsValue(ctx, guest_tag);
        (kind, cpu_val, guest_val)
    }
}

impl TbTranslator {
    pub fn new(
        llvm_ctx: LLVMContextRef,
        ir: &Context,
        func_name: &str,
        profile_counter: Option<u64>,
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

            // Emit profiling counter increment if requested
            if let Some(counter_addr) = profile_counter {
                // Create constant for counter address
                let counter_val = LLVMConstInt(i64t, counter_addr, 0);
                // Cast to pointer
                let counter_ptr = LLVMBuildIntToPtr(builder, counter_val, ptr, c"prof_ctr".as_ptr());
                // Load current value
                let cur_val = LLVMBuildLoad2(builder, i64t, counter_ptr, c"cur_count".as_ptr());
                // Increment
                let one = LLVMConstInt(i64t, 1, 0);
                let new_val = LLVMBuildAdd(builder, cur_val, one, c"new_count".as_ptr());
                // Store back
                LLVMBuildStore(builder, new_val, counter_ptr);
            }

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

            // TBAA metadata for CPUState vs guest memory
            let (tbaa_kind, tbaa_cpu, tbaa_guest) =
                build_tbaa(llvm_ctx);

            Self {
                ctx_ref: llvm_ctx, module, builder, func,
                i1, i8t, i16t, i32t, i64t, i128t, ptr,
                env, guest_base, temps, labels,
                exit_bb, exit_val,
                tb_ptr: ir.tb_ptr,
                aot_peers: HashMap::new(),
                pc_temp: None,
                last_pc_const: None,
                peer_fty: fty,
                aot_dispatch: None,
                aot_dispatch_cache: None,
                aot_helpers: HashMap::new(),
                ctlz_i32, ctlz_i64, cttz_i32, cttz_i64,
                ctpop_i32, ctpop_i64,
                bswap_i16, bswap_i32, bswap_i64,
                tbaa_kind, tbaa_cpu, tbaa_guest,
                profile_counter,
            }
        }
    }

    /// Create a translator with AOT peer functions for musttail direct linking.
    /// `peer_va_to_offset` maps guest VA → file offset for all AOT'd TBs.
    /// Functions are named by file offset (`tb_{offset:x}`) but looked up at
    /// GotoTb time by guest VA (the constant written to the PC temp).
    /// `pc_temp` identifies the PC register temp.
    pub fn new_with_peers(
        llvm_ctx: LLVMContextRef,
        ir: &Context,
        func_name: &str,
        peer_va_to_offset: &std::collections::HashMap<u64, u64>,
        pc_temp: TempIdx,
    ) -> Self {
        let mut s = Self::new(llvm_ctx, ir, func_name, None);
        s.pc_temp = Some(pc_temp);

        // Declare all peer TB functions in this module.
        // Key aot_peers by guest VA so GotoTb lookup (which uses last_pc_const,
        // a guest VA) can find the matching function (named by file offset).
        for (&guest_va, &file_offset) in peer_va_to_offset {
            let name = format!("tb_{file_offset:x}");
            let cname = CString::new(name).unwrap();
            unsafe {
                // Check if already defined (our own function)
                let existing = LLVMGetNamedFunction(s.module, cname.as_ptr());
                if !existing.is_null() {
                    s.aot_peers.insert(guest_va, existing);
                } else {
                    let f = LLVMAddFunction(s.module, cname.as_ptr(), s.peer_fty);
                    s.aot_peers.insert(guest_va, f);
                }
            }
        }
        // Declare aot_dispatch super-function for indirect jump resolution
        // Signature: i64 @aot_dispatch(ptr %env, i64 %guest_base, ptr %cache)
        unsafe {
            let ptr = s.ptr;
            let i64t = s.i64t;
            let mut params = [ptr, i64t, ptr];
            let dispatch_fty = LLVMFunctionType(
                i64t,
                params.as_mut_ptr(),
                3,
                0,
            );
            let f = LLVMAddFunction(s.module, c"aot_dispatch".as_ptr(), dispatch_fty);
            s.aot_dispatch = Some(f);

            // Allocate per-TB cache in BSS (initialized to null)
            let cache_name = CString::new(
                format!("{}_cache", func_name),
            )
            .unwrap();
            let cache_global = LLVMAddGlobal(
                s.module,
                ptr,
                cache_name.as_ptr(),
            );
            LLVMSetInitializer(cache_global, LLVMConstNull(ptr));
            LLVMSetLinkage(cache_global, 8); // Internal linkage
            s.aot_dispatch_cache = Some(cache_global);
        }
        s
    }

    /// Create a translator with AOT peer functions and helper functions.
    /// This version also declares external helper functions that will be
    /// resolved from the tcg-rs executable at runtime.
    pub fn new_with_peers_and_helpers(
        llvm_ctx: LLVMContextRef,
        ir: &Context,
        func_name: &str,
        peer_va_to_offset: &std::collections::HashMap<u64, u64>,
        pc_temp: TempIdx,
        helper_info: &std::collections::HashMap<u64, (String, usize)>,
    ) -> Self {
        let mut s = Self::new_with_peers(
            llvm_ctx, ir, func_name, peer_va_to_offset, pc_temp
        );

        // Declare all helper functions as external using their actual names
        unsafe {
            for (&addr, (name, num_params)) in helper_info {
                let helper_name_c = CString::new(name.as_str()).unwrap();

                // Check if already declared
                let existing = LLVMGetNamedFunction(s.module, helper_name_c.as_ptr());
                if !existing.is_null() {
                    // Get the function type
                    let fty = LLVMTypeOf(existing);
                    s.aot_helpers.insert(addr, (existing, fty));
                } else {
                    // Create function type with actual parameter count
                    let mut param_tys = vec![s.i64t; *num_params];
                    let fty = LLVMFunctionType(
                        s.i64t,
                        param_tys.as_mut_ptr(),
                        *num_params as u32,
                        0,
                    );

                    let func = LLVMAddFunction(
                        s.module,
                        helper_name_c.as_ptr(),
                        fty,
                    );
                    // Mark as external linkage
                    LLVMSetLinkage(func, 0); // LLVMExternalLinkage
                    s.aot_helpers.insert(addr, (func, fty));
                }
            }
        }

        s
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

    /// Ensure two values have the same type for comparison.
    /// If types differ, zero-extend the narrower one to match the wider.
    fn match_cmp_types(
        &self,
        b: LLVMBuilderRef,
        a: LLVMValueRef,
        b_val: LLVMValueRef,
    ) -> (LLVMValueRef, LLVMValueRef) {
        unsafe {
            let a_ty = LLVMTypeOf(a);
            let b_ty = LLVMTypeOf(b_val);
            if a_ty == b_ty {
                (a, b_val)
            } else {
                // Extend narrower operand to match wider one
                // For simplicity, always extend to i64 if types differ
                let target_ty = self.i64t;
                let a_ext = if a_ty != target_ty {
                    LLVMBuildZExt(b, a, target_ty, E)
                } else {
                    a
                };
                let b_ext = if b_ty != target_ty {
                    LLVMBuildZExt(b, b_val, target_ty, E)
                } else {
                    b_val
                };
                (a_ext, b_ext)
            }
        }
    }

    /// Tag a load or store with TBAA metadata.
    fn set_tbaa(&self, inst: LLVMValueRef, tag: LLVMValueRef) {
        unsafe { LLVMSetMetadata(inst, self.tbaa_kind, tag); }
    }

    fn align_for_type(&self, ty: Type) -> u32 {
        match ty {
            Type::I32 => 4,
            _ => 8,
        }
    }

    fn build_load_typed(
        &self,
        b: LLVMBuilderRef,
        ty: Type,
        ptr: LLVMValueRef,
    ) -> LLVMValueRef {
        unsafe {
            let ld = LLVMBuildLoad2(
                b,
                self.llvm_ty(ty),
                ptr,
                E,
            );
            LLVMSetAlignment(ld, self.align_for_type(ty));
            ld
        }
    }

    fn build_store_typed(
        &self,
        b: LLVMBuilderRef,
        ty: Type,
        val: LLVMValueRef,
        ptr: LLVMValueRef,
    ) -> LLVMValueRef {
        unsafe {
            let st = LLVMBuildStore(b, val, ptr);
            LLVMSetAlignment(st, self.align_for_type(ty));
            st
        }
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
                    let ld = self.build_load_typed(
                        b,
                        temp.ty,
                        p,
                    );
                    self.set_tbaa(ld, self.tbaa_cpu);
                    ld
                }
                TempKind::Fixed => {
                    // Fixed temps (env pointer) - return env as i64
                    LLVMBuildPtrToInt(b, self.env, self.i64t, E)
                }
                _ => {
                    // Ebb/Tb local - load from alloca
                    self.build_load_typed(
                        b,
                        temp.ty,
                        self.temps[tidx.0 as usize],
                    )
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
                    let st = self.build_store_typed(
                        b,
                        temp.ty,
                        val,
                        p,
                    );
                    self.set_tbaa(st, self.tbaa_cpu);
                }
                TempKind::Const | TempKind::Fixed => {}
                _ => {
                    self.build_store_typed(
                        b,
                        temp.ty,
                        val,
                        self.temps[tidx.0 as usize],
                    );
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
        let encoded = tcg_core::tb::encode_tb_exit(self.tb_ptr, exit_code);
        unsafe {
            LLVMBuildStore(self.builder, self.ci(self.i64t, encoded as u64), self.exit_val);
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
                self.build_store_typed(
                    b,
                    temp.ty,
                    val,
                    self.temps[i],
                );
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
                        self.build_load_typed(
                            b,
                            temp.ty,
                            self.temps[tidx.0 as usize],
                        )
                    } else {
                        self.build_load_typed(
                            b,
                            temp.ty,
                            self.temps[tidx.0 as usize],
                        )
                    }
                }}
            }

            macro_rules! store_out {
                ($n:expr, $val:expr) => {{
                    let tidx = oarg!($n);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        // Write to local alloca
                        self.build_store_typed(
                            b,
                            temp.ty,
                            $val,
                            self.temps[tidx.0 as usize],
                        );
                    } else if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                        self.build_store_typed(
                            b,
                            temp.ty,
                            $val,
                            self.temps[tidx.0 as usize],
                        );
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
                                let v = self.build_load_typed(
                                    b,
                                    temp.ty,
                                    self.temps[i],
                                );
                                let off = self.ci(self.i64t, temp.mem_offset as u64);
                                let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                                self.build_store_typed(
                                    b,
                                    temp.ty,
                                    v,
                                    p,
                                );
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
                    // Track constant writes to PC for AOT direct linking
                    if let Some(pc_t) = self.pc_temp {
                        if oarg!(0) == pc_t {
                            let src = iarg!(0);
                            let temp = ir.temp(src);
                            if temp.kind == TempKind::Const {
                                self.last_pc_const = Some(temp.val);
                            } else {
                                self.last_pc_const = None;
                            }
                        }
                    }
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
                Opcode::And => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let v = LLVMBuildAnd(b, a, bv, E);
                    store_out!(0, v);
                }
                Opcode::Or  => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let v = LLVMBuildOr(b, a, bv, E);
                    store_out!(0, v);
                }
                Opcode::Xor => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let v = LLVMBuildXor(b, a, bv, E);
                    store_out!(0, v);
                }
                Opcode::Not => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let v = LLVMBuildNot(b, a, E);
                    store_out!(0, v);
                }

                Opcode::AndC => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let nb = LLVMBuildNot(b, bv, E);
                    let v = LLVMBuildAnd(b, a, nb, E);
                    store_out!(0, v);
                }
                Opcode::OrC => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let nb = LLVMBuildNot(b, bv, E);
                    let v = LLVMBuildOr(b, a, nb, E);
                    store_out!(0, v);
                }
                Opcode::Eqv => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let x = LLVMBuildXor(b, a, bv, E);
                    let v = LLVMBuildNot(b, x, E);
                    store_out!(0, v);
                }
                Opcode::Nand => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let anded = LLVMBuildAnd(b, a, bv, E);
                    let v = LLVMBuildNot(b, anded, E);
                    store_out!(0, v);
                }
                Opcode::Nor => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut bv = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_b_ty = ir.temp(iarg!(1)).ty;
                    if src_b_ty.size_bits() < dst_ty.size_bits() {
                        bv = LLVMBuildZExt(b, bv, ty, E);
                    } else if src_b_ty.size_bits() > dst_ty.size_bits() {
                        bv = LLVMBuildTrunc(b, bv, ty, E);
                    }
                    let ored = LLVMBuildOr(b, a, bv, E);
                    let v = LLVMBuildNot(b, ored, E);
                    store_out!(0, v);
                }

                // -- Shifts --
                Opcode::Shl => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut sh = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_sh_ty = ir.temp(iarg!(1)).ty;
                    if src_sh_ty.size_bits() < dst_ty.size_bits() {
                        sh = LLVMBuildZExt(b, sh, ty, E);
                    } else if src_sh_ty.size_bits() > dst_ty.size_bits() {
                        sh = LLVMBuildTrunc(b, sh, ty, E);
                    }
                    let v = LLVMBuildShl(b, a, sh, E);
                    store_out!(0, v);
                }
                Opcode::Shr => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut sh = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_sh_ty = ir.temp(iarg!(1)).ty;
                    if src_sh_ty.size_bits() < dst_ty.size_bits() {
                        sh = LLVMBuildZExt(b, sh, ty, E);
                    } else if src_sh_ty.size_bits() > dst_ty.size_bits() {
                        sh = LLVMBuildTrunc(b, sh, ty, E);
                    }
                    let v = LLVMBuildLShr(b, a, sh, E);
                    store_out!(0, v);
                }
                Opcode::Sar => {
                    let dst_ty = ir.temp(oarg!(0)).ty;
                    let ty = self.llvm_ty(dst_ty);
                    let mut a = ival!(0);
                    let mut sh = ival!(1);
                    let src_a_ty = ir.temp(iarg!(0)).ty;
                    if src_a_ty.size_bits() < dst_ty.size_bits() {
                        a = LLVMBuildZExt(b, a, ty, E);
                    } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                        a = LLVMBuildTrunc(b, a, ty, E);
                    }
                    let src_sh_ty = ir.temp(iarg!(1)).ty;
                    if src_sh_ty.size_bits() < dst_ty.size_bits() {
                        sh = LLVMBuildZExt(b, sh, ty, E);
                    } else if src_sh_ty.size_bits() > dst_ty.size_bits() {
                        sh = LLVMBuildTrunc(b, sh, ty, E);
                    }
                    let v = LLVMBuildAShr(b, a, sh, E);
                    store_out!(0, v);
                }

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
                        let v = self.build_load_typed(
                            b,
                            temp.ty,
                            self.temps[i],
                        );
                        let off = self.ci(self.i64t, temp.mem_offset as u64);
                        let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                        self.build_store_typed(
                            b,
                            temp.ty,
                            v,
                            p,
                        );
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
                    self.build_load_typed(
                        b,
                        temp.ty,
                        self.temps[tidx.0 as usize],
                    )
                }
            }}
        }

        macro_rules! store_out {
            ($n:expr, $val:expr) => {{
                let tidx = oarg!($n);
                let temp = ir.temp(tidx);
                if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                    self.build_store_typed(
                        b,
                        temp.ty,
                        $val,
                        self.temps[tidx.0 as usize],
                    );
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
                            let v = self.build_load_typed(
                                b,
                                temp.ty,
                                self.temps[i],
                            );
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            let st = self.build_store_typed(
                                b,
                                temp.ty,
                                v,
                                p,
                            );
                            self.set_tbaa(st, self.tbaa_cpu);
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
            else { self.build_load_typed(b, temp.ty, self.temps[tidx.0 as usize]) }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                self.build_store_typed(b, temp.ty, $val, self.temps[tidx.0 as usize]);
            }
        }}}

        unsafe { match op.opc {
            Opcode::SetCond => {
                let cond = Cond::from_u8(carg!(0) as u8);
                let (a, b_val) = (ival!(0), ival!(1));
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, a, b_val, E);
                    let zero = LLVMConstInt(LLVMTypeOf(anded), 0, 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    let (a_ext, b_ext) = self.match_cmp_types(b, a, b_val);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a_ext, b_ext, E)
                };
                let v = LLVMBuildZExt(b, cmp, self.llvm_ty(op.op_type), E);
                store_out!(0, v);
            }
            Opcode::NegSetCond => {
                let cond = Cond::from_u8(carg!(0) as u8);
                let (a, b_val) = (ival!(0), ival!(1));
                let cmp = if cond.is_tst() {
                    let anded = LLVMBuildAnd(b, a, b_val, E);
                    let zero = LLVMConstInt(LLVMTypeOf(anded), 0, 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    let (a_ext, b_ext) = self.match_cmp_types(b, a, b_val);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a_ext, b_ext, E)
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
                    let zero = LLVMConstInt(LLVMTypeOf(anded), 0, 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    let (c1_ext, c2_ext) = self.match_cmp_types(b, c1, c2);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), c1_ext, c2_ext, E)
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
                self.set_tbaa(raw, self.tbaa_cpu);
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
                let st = LLVMBuildStore(b, store_val, p);
                self.set_tbaa(st, self.tbaa_cpu);
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
            else { self.build_load_typed(b, temp.ty, self.temps[tidx.0 as usize]) }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                self.build_store_typed(b, temp.ty, $val, self.temps[tidx.0 as usize]);
            }
        }}}
        macro_rules! flush_globals {
            () => {
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        unsafe {
                            let v = self.build_load_typed(
                                b,
                                temp.ty,
                                self.temps[i],
                            );
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            let st = self.build_store_typed(
                                b,
                                temp.ty,
                                v,
                                p,
                            );
                            self.set_tbaa(st, self.tbaa_cpu);
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
                self.set_tbaa(raw, self.tbaa_guest);
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
                let st = LLVMBuildStore(b, store_val, p);
                self.set_tbaa(st, self.tbaa_guest);
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
            else { self.build_load_typed(b, temp.ty, self.temps[tidx.0 as usize]) }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                self.build_store_typed(b, temp.ty, $val, self.temps[tidx.0 as usize]);
            }
        }}}
        macro_rules! flush_globals {
            () => {
                for i in 0..ir.nb_globals() as usize {
                    let tidx = TempIdx(i as u32);
                    let temp = ir.temp(tidx);
                    if temp.kind == TempKind::Global {
                        unsafe {
                            let v = self.build_load_typed(
                                b,
                                temp.ty,
                                self.temps[i],
                            );
                            let off = self.ci(self.i64t, temp.mem_offset as u64);
                            let p = LLVMBuildGEP2(b, self.i8t, self.env, [off].as_ptr(), 1, E);
                            let st = self.build_store_typed(
                                b,
                                temp.ty,
                                v,
                                p,
                            );
                            self.set_tbaa(st, self.tbaa_cpu);
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
                    let zero = LLVMConstInt(LLVMTypeOf(anded), 0, 0);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), anded, zero, E)
                } else {
                    let (a_ext, bv_ext) = self.match_cmp_types(b, a, bv);
                    LLVMBuildICmp(b, Self::cond_to_pred(cond), a_ext, bv_ext, E)
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
                        let v = self.build_load_typed(
                            b,
                            temp.ty,
                            p,
                        );
                        self.build_store_typed(
                            b,
                            temp.ty,
                            v,
                            self.temps[i],
                        );
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
                        let v = self.build_load_typed(
                            b,
                            temp.ty,
                            p,
                        );
                        self.build_store_typed(
                            b,
                            temp.ty,
                            v,
                            self.temps[i],
                        );
                    }
                }
            }
            Opcode::ExitTb => {
                let val = carg!(0) as u64;
                flush_globals!();
                // For NOCHAIN exits (jalr / indirect jumps), try aot_dispatch
                // to stay within the AOT .so instead of returning to exec loop.
                if val == tcg_core::tb::TB_EXIT_NOCHAIN {
                    if let Some(dispatch_fn) = self.aot_dispatch {
                        let cache = self.aot_dispatch_cache.unwrap();
                        let ptr = self.ptr;
                        let i64t = self.i64t;
                        let mut params = [ptr, i64t, ptr];
                        let dispatch_fty = LLVMFunctionType(
                            i64t,
                            params.as_mut_ptr(),
                            3,
                            0,
                        );
                        let mut args = [self.env, self.guest_base, cache];
                        let call = LLVMBuildCall2(
                            b, dispatch_fty, dispatch_fn,
                            args.as_mut_ptr(), 3, E,
                        );
                        LLVMSetTailCallKind(call, 2); // MustTail
                        LLVMBuildRet(b, call);
                    } else {
                        self.do_exit(val);
                    }
                } else {
                    self.do_exit(val);
                }
                *terminated = true;
            }
            Opcode::GotoTb => {
                flush_globals!();
                // AOT direct linking: musttail call to peer if target PC is known
                // and exists in the AOT file. If target doesn't exist in AOT,
                // return to exec loop instead of calling aot_dispatch.
                let peer = self.last_pc_const
                    .and_then(|pc| self.aot_peers.get(&pc).copied());
                if let Some(peer_fn) = peer {
                    let mut args = [self.env, self.guest_base];
                    let call = LLVMBuildCall2(
                        b, self.peer_fty, peer_fn,
                        args.as_mut_ptr(), 2, E,
                    );
                    LLVMSetTailCallKind(call, 2); // MustTail
                    LLVMBuildRet(b, call);
                } else {
                    // Target not in AOT file; return to exec loop
                    self.do_exit(tcg_core::tb::TB_EXIT_NOCHAIN);
                }
                *terminated = true;
            }
            Opcode::GotoPtr => {
                flush_globals!();
                if let Some(dispatch_fn) = self.aot_dispatch {
                    // musttail call aot_dispatch — it will switch on PC
                    let cache = self.aot_dispatch_cache.unwrap();
                    let ptr = self.ptr;
                    let i64t = self.i64t;
                    let mut params = [ptr, i64t, ptr];
                    let dispatch_fty = LLVMFunctionType(
                        i64t,
                        params.as_mut_ptr(),
                        3,
                        0,
                    );
                    let mut args = [self.env, self.guest_base, cache];
                    let call = LLVMBuildCall2(
                        b, dispatch_fty, dispatch_fn,
                        args.as_mut_ptr(), 3, E,
                    );
                    LLVMSetTailCallKind(call, 2); // MustTail
                    LLVMBuildRet(b, call);
                } else {
                    self.do_exit(tcg_core::tb::TB_EXIT_NOCHAIN);
                }
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
                self.build_load_typed(
                    b,
                    temp.ty,
                    self.temps[tidx.0 as usize],
                )
            }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const
                && temp.kind != TempKind::Fixed
            {
                self.build_store_typed(
                    b,
                    temp.ty,
                    $val,
                    self.temps[tidx.0 as usize],
                );
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
                        let v = self.build_load_typed(
                            b,
                            temp.ty,
                            self.temps[i],
                        );
                        let off = self.ci(
                            self.i64t,
                            temp.mem_offset as u64,
                        );
                        let p = LLVMBuildGEP2(
                            b, self.i8t, self.env,
                            [off].as_ptr(), 1, E,
                        );
                        self.build_store_typed(
                            b,
                            temp.ty,
                            v,
                            p,
                        );
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

                // Check if this is an AOT helper call
                let ret = if let Some(&(helper_func, helper_fty)) = self.aot_helpers.get(&func_addr) {
                    // Get the actual parameter count from the function type
                    let param_count = LLVMCountParamTypes(helper_fty) as usize;
                    // Only pass the required number of arguments
                    let call_args = &args[..param_count];
                    LLVMBuildCall2(
                        b,
                        helper_fty,
                        helper_func,
                        call_args.as_ptr(),
                        call_args.len() as u32,
                        E,
                    )
                } else {
                    // Fall back to inttoptr for non-AOT mode
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
                    LLVMBuildCall2(
                        b, fty, fptr,
                        args.as_ptr(), args.len() as u32, E,
                    )
                };

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
                        let v = self.build_load_typed(
                            b,
                            temp.ty,
                            p,
                        );
                        self.build_store_typed(
                            b,
                            temp.ty,
                            v,
                            self.temps[i],
                        );
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
            else { self.build_load_typed(b, temp.ty, self.temps[tidx.0 as usize]) }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                self.build_store_typed(b, temp.ty, $val, self.temps[tidx.0 as usize]);
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
                let mut a = ival!(0);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let dst_ty = ir.temp(oarg!(0)).ty;
                let ty = self.llvm_ty(dst_ty);
                let src_ty = ir.temp(iarg!(0)).ty;
                if src_ty.size_bits() < dst_ty.size_bits() {
                    a = LLVMBuildZExt(b, a, ty, E);
                } else if src_ty.size_bits() > dst_ty.size_bits() {
                    a = LLVMBuildTrunc(b, a, ty, E);
                }
                let sh = LLVMBuildLShr(b, a, self.ci(ty, ofs), E);
                let mask = self.ci(ty, (1u64 << len).wrapping_sub(1));
                let v = LLVMBuildAnd(b, sh, mask, E);
                store_out!(0, v);
            }
            Opcode::SExtract => {
                let mut a = ival!(0);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let dst_ty = ir.temp(oarg!(0)).ty;
                let ty = self.llvm_ty(dst_ty);
                let src_ty = ir.temp(iarg!(0)).ty;
                if src_ty.size_bits() < dst_ty.size_bits() {
                    a = LLVMBuildZExt(b, a, ty, E);
                } else if src_ty.size_bits() > dst_ty.size_bits() {
                    a = LLVMBuildTrunc(b, a, ty, E);
                }
                let bits = dst_ty.size_bits() as u64;
                let shl_amt = bits - len - ofs;
                let sar_amt = bits - len;
                let sh = LLVMBuildShl(b, a, self.ci(ty, shl_amt), E);
                let v = LLVMBuildAShr(b, sh, self.ci(ty, sar_amt), E);
                store_out!(0, v);
            }
            Opcode::Deposit => {
                let mut a = ival!(0);
                let mut val = ival!(1);
                let ofs = carg!(0) as u64;
                let len = carg!(1) as u64;
                let dst_ty = ir.temp(oarg!(0)).ty;
                let ty = self.llvm_ty(dst_ty);
                let src_a_ty = ir.temp(iarg!(0)).ty;
                if src_a_ty.size_bits() < dst_ty.size_bits() {
                    a = LLVMBuildZExt(b, a, ty, E);
                } else if src_a_ty.size_bits() > dst_ty.size_bits() {
                    a = LLVMBuildTrunc(b, a, ty, E);
                }
                let src_v_ty = ir.temp(iarg!(1)).ty;
                if src_v_ty.size_bits() < dst_ty.size_bits() {
                    val = LLVMBuildZExt(b, val, ty, E);
                } else if src_v_ty.size_bits() > dst_ty.size_bits() {
                    val = LLVMBuildTrunc(b, val, ty, E);
                }
                let mask = self.ci(ty, ((1u64 << len).wrapping_sub(1)) << ofs);
                let nmask = LLVMBuildNot(b, mask, E);
                let cleared = LLVMBuildAnd(b, a, nmask, E);
                let shifted = LLVMBuildShl(b, val, self.ci(ty, ofs), E);
                let masked = LLVMBuildAnd(b, shifted, mask, E);
                let v = LLVMBuildOr(b, cleared, masked, E);
                store_out!(0, v);
            }
            Opcode::Extract2 => {
                let mut ah = ival!(0);
                let mut al = ival!(1);
                let shr = carg!(0) as u64;
                let dst_ty = ir.temp(oarg!(0)).ty;
                let ty = self.llvm_ty(dst_ty);
                let src_h_ty = ir.temp(iarg!(0)).ty;
                if src_h_ty.size_bits() < dst_ty.size_bits() {
                    ah = LLVMBuildZExt(b, ah, ty, E);
                } else if src_h_ty.size_bits() > dst_ty.size_bits() {
                    ah = LLVMBuildTrunc(b, ah, ty, E);
                }
                let src_l_ty = ir.temp(iarg!(1)).ty;
                if src_l_ty.size_bits() < dst_ty.size_bits() {
                    al = LLVMBuildZExt(b, al, ty, E);
                } else if src_l_ty.size_bits() > dst_ty.size_bits() {
                    al = LLVMBuildTrunc(b, al, ty, E);
                }
                let bits = dst_ty.size_bits() as u64;
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
            else { self.build_load_typed(b, temp.ty, self.temps[tidx.0 as usize]) }
        }}}
        macro_rules! store_out { ($n:expr, $val:expr) => {{
            let tidx = oarg!($n);
            let temp = ir.temp(tidx);
            if temp.kind != TempKind::Const && temp.kind != TempKind::Fixed {
                self.build_store_typed(b, temp.ty, $val, self.temps[tidx.0 as usize]);
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
