pub mod ffi;
pub mod translate;

use std::ffi::CString;
use std::ptr;

use ffi::*;

/// Persistent LLVM JIT state, reused across TB compilations.
pub struct LlvmJit {
    jit: LLVMOrcLLJITRef,
    jd: LLVMOrcJITDylibRef,
    tb_counter: u64,
}

unsafe impl Send for LlvmJit {}
unsafe impl Sync for LlvmJit {}

impl LlvmJit {
    pub fn new() -> Self {
        unsafe {
            LLVMInitializeX86Target();
            LLVMInitializeX86TargetInfo();
            LLVMInitializeX86TargetMC();
            LLVMInitializeX86AsmPrinter();
            LLVMInitializeX86AsmParser();

            let builder = LLVMOrcCreateLLJITBuilder();
            let mut jit: LLVMOrcLLJITRef = ptr::null_mut();
            let err = LLVMOrcCreateLLJIT(&mut jit, builder);
            if !err.is_null() {
                let msg = LLVMGetErrorMessage(err);
                let s = std::ffi::CStr::from_ptr(msg)
                    .to_string_lossy().into_owned();
                LLVMDisposeErrorMessage(msg);
                panic!("LLVMOrcCreateLLJIT failed: {s}");
            }
            let jd = LLVMOrcLLJITGetMainJITDylib(jit);

            Self { jit, jd, tb_counter: 0 }
        }
    }

    /// Compile a module and look up the function.
    /// Takes ownership of the module and its context.
    pub fn compile(&mut self, module: LLVMModuleRef, func_name: &str) -> u64 {
        unsafe {
            let ctx = LLVMGetModuleContext(module);
            let tsc = LLVMOrcCreateNewThreadSafeContextFromLLVMContext(ctx);
            let tsm = LLVMOrcCreateNewThreadSafeModule(module, tsc);
            let err = LLVMOrcLLJITAddLLVMIRModule(self.jit, self.jd, tsm);
            if !err.is_null() {
                let msg = LLVMGetErrorMessage(err);
                let s = std::ffi::CStr::from_ptr(msg)
                    .to_string_lossy().into_owned();
                LLVMDisposeErrorMessage(msg);
                panic!("LLVMOrcLLJITAddLLVMIRModule failed: {s}");
            }

            let cname = CString::new(func_name).unwrap();
            let mut addr: LLVMOrcExecutorAddress = 0;
            let err = LLVMOrcLLJITLookup(self.jit, &mut addr, cname.as_ptr());
            if !err.is_null() {
                let msg = LLVMGetErrorMessage(err);
                let s = std::ffi::CStr::from_ptr(msg)
                    .to_string_lossy().into_owned();
                LLVMDisposeErrorMessage(msg);
                panic!("LLVMOrcLLJITLookup({func_name}) failed: {s}");
            }
            addr
        }
    }

    pub fn next_tb_name(&mut self) -> String {
        let n = self.tb_counter;
        self.tb_counter += 1;
        format!("tb_{n}")
    }

    /// Create a fresh LLVM context for building a module.
    pub fn context(&self) -> LLVMContextRef {
        unsafe { LLVMContextCreate() }
    }
}

impl Drop for LlvmJit {
    fn drop(&mut self) {
        unsafe {
            LLVMOrcDisposeLLJIT(self.jit);
        }
    }
}
