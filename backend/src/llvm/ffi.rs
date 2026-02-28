//! Minimal LLVM C API FFI bindings for JIT compilation.

#![allow(non_camel_case_types, dead_code)]

use std::ffi::c_char;

// Opaque types
pub enum LLVMOpaqueContext {}
pub enum LLVMOpaqueModule {}
pub enum LLVMOpaqueBuilder {}
pub enum LLVMOpaqueType {}
pub enum LLVMOpaqueValue {}
pub enum LLVMOpaqueBasicBlock {}
pub enum LLVMOpaquePassManager {}
pub enum LLVMOpaqueTargetMachine {}

// OrcV2 opaque types
pub enum LLVMOrcOpaqueThreadSafeContext {}
pub enum LLVMOrcOpaqueThreadSafeModule {}
pub enum LLVMOrcOpaqueLLJIT {}
pub enum LLVMOrcOpaqueLLJITBuilder {}
pub enum LLVMOrcOpaqueJITDylib {}
pub enum LLVMOrcOpaqueExecutionSession {}

pub type LLVMContextRef = *mut LLVMOpaqueContext;
pub type LLVMModuleRef = *mut LLVMOpaqueModule;
pub type LLVMBuilderRef = *mut LLVMOpaqueBuilder;
pub type LLVMTypeRef = *mut LLVMOpaqueType;
pub type LLVMValueRef = *mut LLVMOpaqueValue;
pub type LLVMBasicBlockRef = *mut LLVMOpaqueBasicBlock;
pub type LLVMPassManagerRef = *mut LLVMOpaquePassManager;

pub type LLVMOrcThreadSafeContextRef = *mut LLVMOrcOpaqueThreadSafeContext;
pub type LLVMOrcThreadSafeModuleRef = *mut LLVMOrcOpaqueThreadSafeModule;
pub type LLVMOrcLLJITRef = *mut LLVMOrcOpaqueLLJIT;
pub type LLVMOrcLLJITBuilderRef = *mut LLVMOrcOpaqueLLJITBuilder;
pub type LLVMOrcJITDylibRef = *mut LLVMOrcOpaqueJITDylib;
pub type LLVMOrcExecutionSessionRef = *mut LLVMOrcOpaqueExecutionSession;
pub type LLVMOrcExecutorAddress = u64;

// Error type
pub enum LLVMOpaqueError {}
pub type LLVMErrorRef = *mut LLVMOpaqueError;

// Enums
#[repr(C)]
#[derive(Clone, Copy)]
pub enum LLVMIntPredicate {
    LLVMIntEQ = 32,
    LLVMIntNE = 33,
    LLVMIntUGT = 34,
    LLVMIntUGE = 35,
    LLVMIntULT = 36,
    LLVMIntULE = 37,
    LLVMIntSGT = 38,
    LLVMIntSGE = 39,
    LLVMIntSLT = 40,
    LLVMIntSLE = 41,
}

#[repr(C)]
pub enum LLVMCallConv {
    LLVMCCallConv = 0,
}

#[link(name = "LLVM-21")]
extern "C" {
    // Target initialization (x86_64 host)
    pub fn LLVMInitializeX86Target();
    pub fn LLVMInitializeX86TargetInfo();
    pub fn LLVMInitializeX86TargetMC();
    pub fn LLVMInitializeX86AsmPrinter();
    pub fn LLVMInitializeX86AsmParser();

    // Context
    pub fn LLVMContextCreate() -> LLVMContextRef;
    pub fn LLVMContextDispose(c: LLVMContextRef);

    // Module
    pub fn LLVMModuleCreateWithNameInContext(
        name: *const c_char,
        c: LLVMContextRef,
    ) -> LLVMModuleRef;
    pub fn LLVMDisposeModule(m: LLVMModuleRef);
    pub fn LLVMDumpModule(m: LLVMModuleRef);

    // Builder
    pub fn LLVMCreateBuilderInContext(c: LLVMContextRef) -> LLVMBuilderRef;
    pub fn LLVMPositionBuilderAtEnd(b: LLVMBuilderRef, bb: LLVMBasicBlockRef);
    pub fn LLVMDisposeBuilder(b: LLVMBuilderRef);

    // Types
    pub fn LLVMInt1TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMInt8TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMInt16TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMInt32TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMInt64TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMInt128TypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMPointerTypeInContext(c: LLVMContextRef, addr_space: u32) -> LLVMTypeRef;
    pub fn LLVMVoidTypeInContext(c: LLVMContextRef) -> LLVMTypeRef;
    pub fn LLVMFunctionType(
        ret: LLVMTypeRef,
        params: *const LLVMTypeRef,
        param_count: u32,
        is_var_arg: i32,
    ) -> LLVMTypeRef;

    // Values
    pub fn LLVMConstInt(ty: LLVMTypeRef, n: u64, sign_extend: i32) -> LLVMValueRef;
    pub fn LLVMGetParam(func: LLVMValueRef, idx: u32) -> LLVMValueRef;
    pub fn LLVMSetValueName2(val: LLVMValueRef, name: *const c_char, len: usize);

    // Functions
    pub fn LLVMAddFunction(
        m: LLVMModuleRef,
        name: *const c_char,
        ty: LLVMTypeRef,
    ) -> LLVMValueRef;
    pub fn LLVMAppendBasicBlockInContext(
        c: LLVMContextRef,
        func: LLVMValueRef,
        name: *const c_char,
    ) -> LLVMBasicBlockRef;

    // Instructions - arithmetic
    pub fn LLVMBuildAdd(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildSub(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildMul(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildSDiv(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildUDiv(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildSRem(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildURem(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildNeg(
        b: LLVMBuilderRef, v: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;

    // Instructions - logic
    pub fn LLVMBuildAnd(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildOr(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildXor(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildNot(
        b: LLVMBuilderRef, v: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildShl(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildLShr(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildAShr(
        b: LLVMBuilderRef, lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;

    // Instructions - comparison
    pub fn LLVMBuildICmp(
        b: LLVMBuilderRef, op: LLVMIntPredicate,
        lhs: LLVMValueRef, rhs: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildSelect(
        b: LLVMBuilderRef, cond: LLVMValueRef,
        then_val: LLVMValueRef, else_val: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;

    // Instructions - memory
    pub fn LLVMBuildLoad2(
        b: LLVMBuilderRef, ty: LLVMTypeRef, ptr: LLVMValueRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildStore(
        b: LLVMBuilderRef, val: LLVMValueRef, ptr: LLVMValueRef,
    ) -> LLVMValueRef;
    pub fn LLVMBuildGEP2(
        b: LLVMBuilderRef, ty: LLVMTypeRef, ptr: LLVMValueRef,
        indices: *const LLVMValueRef, num_indices: u32, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildAlloca(
        b: LLVMBuilderRef, ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;

    // Instructions - casts
    pub fn LLVMBuildTrunc(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildZExt(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildSExt(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildIntToPtr(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildPtrToInt(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildBitCast(
        b: LLVMBuilderRef, val: LLVMValueRef, dest_ty: LLVMTypeRef, name: *const c_char,
    ) -> LLVMValueRef;

    // Instructions - control flow
    pub fn LLVMBuildBr(b: LLVMBuilderRef, dest: LLVMBasicBlockRef) -> LLVMValueRef;
    pub fn LLVMBuildCondBr(
        b: LLVMBuilderRef, cond: LLVMValueRef,
        then_bb: LLVMBasicBlockRef, else_bb: LLVMBasicBlockRef,
    ) -> LLVMValueRef;
    pub fn LLVMBuildRet(b: LLVMBuilderRef, v: LLVMValueRef) -> LLVMValueRef;
    pub fn LLVMBuildRetVoid(b: LLVMBuilderRef) -> LLVMValueRef;
    pub fn LLVMBuildCall2(
        b: LLVMBuilderRef, ty: LLVMTypeRef, func: LLVMValueRef,
        args: *const LLVMValueRef, num_args: u32, name: *const c_char,
    ) -> LLVMValueRef;
    pub fn LLVMBuildUnreachable(b: LLVMBuilderRef) -> LLVMValueRef;

    // Intrinsics
    pub fn LLVMGetIntrinsicDeclaration(
        m: LLVMModuleRef, id: u32,
        param_types: *const LLVMTypeRef, param_count: usize,
    ) -> LLVMValueRef;
    pub fn LLVMLookupIntrinsicID(
        name: *const c_char, name_len: usize,
    ) -> u32;

    // OrcV2 JIT
    pub fn LLVMOrcCreateLLJITBuilder() -> LLVMOrcLLJITBuilderRef;
    pub fn LLVMOrcDisposeLLJITBuilder(builder: LLVMOrcLLJITBuilderRef);
    pub fn LLVMOrcCreateLLJIT(
        result: *mut LLVMOrcLLJITRef,
        builder: LLVMOrcLLJITBuilderRef,
    ) -> LLVMErrorRef;
    pub fn LLVMOrcDisposeLLJIT(jit: LLVMOrcLLJITRef) -> LLVMErrorRef;
    pub fn LLVMOrcLLJITGetMainJITDylib(jit: LLVMOrcLLJITRef) -> LLVMOrcJITDylibRef;
    pub fn LLVMOrcLLJITAddLLVMIRModule(
        jit: LLVMOrcLLJITRef,
        jd: LLVMOrcJITDylibRef,
        tsm: LLVMOrcThreadSafeModuleRef,
    ) -> LLVMErrorRef;
    pub fn LLVMOrcLLJITLookup(
        jit: LLVMOrcLLJITRef,
        result: *mut LLVMOrcExecutorAddress,
        name: *const c_char,
    ) -> LLVMErrorRef;
    pub fn LLVMOrcCreateNewThreadSafeContext() -> LLVMOrcThreadSafeContextRef;
    pub fn LLVMOrcThreadSafeContextGetContext(
        tsc: LLVMOrcThreadSafeContextRef,
    ) -> LLVMContextRef;
    pub fn LLVMOrcCreateNewThreadSafeContextFromLLVMContext(
        ctx: LLVMContextRef,
    ) -> LLVMOrcThreadSafeContextRef;
    pub fn LLVMOrcDisposeThreadSafeContext(tsc: LLVMOrcThreadSafeContextRef);
    pub fn LLVMOrcCreateNewThreadSafeModule(
        m: LLVMModuleRef,
        tsc: LLVMOrcThreadSafeContextRef,
    ) -> LLVMOrcThreadSafeModuleRef;
    pub fn LLVMOrcDisposeThreadSafeModule(tsm: LLVMOrcThreadSafeModuleRef);

    pub fn LLVMGetModuleContext(m: LLVMModuleRef) -> LLVMContextRef;
    pub fn LLVMOrcLLJITGetExecutionSession(
        jit: LLVMOrcLLJITRef,
    ) -> LLVMOrcExecutionSessionRef;
    pub fn LLVMOrcLLJITGetDataLayoutStr(jit: LLVMOrcLLJITRef) -> *const c_char;

    // Error handling
    pub fn LLVMGetErrorMessage(err: LLVMErrorRef) -> *mut c_char;
    pub fn LLVMDisposeErrorMessage(msg: *mut c_char);

    // Module verification
    pub fn LLVMVerifyModule(
        m: LLVMModuleRef,
        action: u32, // 0=AbortProcess, 1=PrintMessage, 2=ReturnStatus
        out_msg: *mut *mut c_char,
    ) -> i32;
    pub fn LLVMDisposeMessage(msg: *mut c_char);

    // Fence
    pub fn LLVMBuildFence(b: LLVMBuilderRef, ordering: u32, single_thread: i32, name: *const c_char) -> LLVMValueRef;

    // Overflow arithmetic intrinsics
    pub fn LLVMBuildExtractValue(b: LLVMBuilderRef, agg: LLVMValueRef, idx: u32, name: *const c_char) -> LLVMValueRef;

    // Data layout
    pub fn LLVMSetDataLayout(m: LLVMModuleRef, layout: *const c_char);

    // Pass manager (new pass manager via C API)
    pub fn LLVMCreatePassManager() -> LLVMPassManagerRef;
    pub fn LLVMRunPassManager(pm: LLVMPassManagerRef, m: LLVMModuleRef) -> i32;
    pub fn LLVMDisposePassManager(pm: LLVMPassManagerRef);
}
