use crate::code_buffer::CodeBuffer;
use crate::liveness::liveness_analysis;
use crate::optimize::optimize;
use crate::regalloc::regalloc_and_codegen;
use crate::HostCodeGen;
use tcg_core::Context;

/// Full translation pipeline: optimize → liveness → regalloc+codegen.
/// Returns the offset where TB code starts in the buffer.
pub fn translate(
    ctx: &mut Context,
    backend: &impl HostCodeGen,
    buf: &mut CodeBuffer,
) -> usize {
    optimize(ctx);
    liveness_analysis(ctx);

    let tb_start = buf.offset();
    regalloc_and_codegen(ctx, backend, buf);
    tb_start
}

/// LLVM JIT translation: TCG IR → LLVM IR → native, then emit a
/// trampoline in the code buffer that calls the JIT'd function.
///
/// The trampoline bridges the x86_64 prologue calling convention
/// (env in RBP, guest_base in R14) to the LLVM function signature
/// `fn(env: *mut u8, guest_base: u64) -> u64`.
///
/// Returns the offset where the trampoline starts in the buffer.
#[cfg(feature = "llvm")]
pub fn translate_llvm(
    ctx: &mut Context,
    jit: &mut crate::llvm::LlvmJit,
    buf: &mut CodeBuffer,
    epilogue_offset: usize,
) -> usize {
    optimize(ctx);

    let func_name = jit.next_tb_name();
    let translator = crate::llvm::translate::TbTranslator::new(
        jit.context(), ctx, &func_name,
    );
    let module = translator.translate(ctx);

    // Verify and compile
    unsafe {
        let mut err_msg: *mut i8 = std::ptr::null_mut();
        let rc = crate::llvm::ffi::LLVMVerifyModule(
            module, 2, // ReturnStatus
            &mut err_msg,
        );
        if rc != 0 {
            if !err_msg.is_null() {
                let s = std::ffi::CStr::from_ptr(err_msg)
                    .to_string_lossy().into_owned();
                crate::llvm::ffi::LLVMDisposeMessage(err_msg);
                eprintln!("LLVM verify warning for {func_name}: {s}");
            }
        }
    }

    let jit_addr = jit.compile(module, &func_name);

    // Emit x86_64 trampoline in code buffer:
    //   mov rdi, rbp        ; arg0 = env (48 89 ef)
    //   mov rsi, r14        ; arg1 = guest_base (4c 89 f6)
    //   movabs rax, <addr>  ; (48 b8 <8 bytes>)
    //   call rax             ; (ff d0)
    //   jmp <epilogue>       ; (e9 <rel32>)
    let tb_start = buf.offset();

    // mov rdi, rbp
    buf.emit_bytes(&[0x48, 0x89, 0xef]);
    // mov rsi, r14
    buf.emit_bytes(&[0x4c, 0x89, 0xf6]);
    // movabs rax, jit_addr
    buf.emit_bytes(&[0x48, 0xb8]);
    buf.emit_u64(jit_addr);
    // call rax
    buf.emit_bytes(&[0xff, 0xd0]);
    // jmp rel32 to epilogue (tb_ret_offset)
    buf.emit_u8(0xe9);
    let jmp_site = buf.offset();
    let rel = (epilogue_offset as i64) - ((jmp_site + 4) as i64);
    buf.emit_u32(rel as u32);

    tb_start
}

/// Translate and execute a TB.
///
/// # Safety
/// `env` must point to a valid CPUState-like struct that
/// matches the globals registered in `ctx`.
pub unsafe fn translate_and_execute(
    ctx: &mut Context,
    backend: &impl HostCodeGen,
    buf: &mut CodeBuffer,
    env: *mut u8,
) -> usize {
    // Buffer is RWX, no permission switch needed.
    let tb_start = translate(ctx, backend, buf);

    // Prologue signature:
    //   fn(env: *mut u8, tb_ptr: *const u8) -> usize
    // RDI = env, RSI = TB code pointer, returns RAX
    let prologue_fn: unsafe extern "C" fn(*mut u8, *const u8) -> usize =
        core::mem::transmute(buf.base_ptr());
    let tb_ptr = buf.ptr_at(tb_start);
    let raw = prologue_fn(env, tb_ptr);
    // Decode: strip the encoded TB index, return only the
    // exit code (slot number or exception code).
    let (_, exit_code) = tcg_core::tb::decode_tb_exit(raw);
    exit_code
}
