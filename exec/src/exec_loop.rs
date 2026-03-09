use std::sync::atomic::Ordering;

use crate::{
    ExecEnv, GuestCpu, PerCpuState, SharedState, MIN_CODE_BUF_REMAINING,
};
#[cfg(feature = "llvm")]
use crate::TranslateGuard;
use tcg_backend::translate::translate;
use tcg_backend::HostCodeGen;
use tcg_core::tb::{decode_tb_exit, TranslationBlock, TB_EXIT_NOCHAIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    Exit(usize),
    BufferFull,
}

/// # Safety
/// Caller must ensure `env` contains valid code buffer and CPU state.
pub unsafe fn cpu_exec_loop<B, C>(env: &mut ExecEnv<B>, cpu: &mut C) -> ExitReason
where B: HostCodeGen, C: GuestCpu,
{
    cpu_exec_loop_mt(&env.shared, &mut env.per_cpu, cpu)
}

/// # Safety
/// Caller must ensure `shared` contains valid code buffer and CPU state,
/// and that `cpu` is properly initialised.
pub unsafe fn cpu_exec_loop_mt<B, C>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, cpu: &mut C,
) -> ExitReason
where B: HostCodeGen, C: GuestCpu,
{
    let mut next_tb_hint: Option<*mut TranslationBlock> = None;
    let tb_limit: u64 = std::env::var("TCG_TB_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let tb_trace = tb_limit > 0;
    let tb_dump = std::env::var("TCG_TB_DUMP").is_ok();
    let mut tb_count: u64 = 0;
    let mut tb_dump_idx: u64 = 0;
    let mut last_pcs: [u64; 8] = [0; 8];

    loop {
        per_cpu.stats.loop_iters += 1;
        let iter_pc = cpu.get_pc();
        if tb_dump {
            eprintln!("[tb] {} {:#x}", tb_dump_idx, iter_pc);
            tb_dump_idx += 1;
        }
        if tb_trace {
            last_pcs[(tb_count as usize) & 7] = iter_pc;
            tb_count += 1;
            if tb_count > tb_limit {
                eprintln!("[exec] TB limit reached after {} TBs", tb_count);
                for i in 0..8 {
                    let idx = ((tb_count as usize).wrapping_sub(8).wrapping_add(i)) & 7;
                    eprintln!("  pc[{}] = {:#x}", i, last_pcs[idx]);
                }
                return ExitReason::Exit(0xff);
            }
        }

        let tb_ptr = match next_tb_hint.take() {
            Some(ptr) => { per_cpu.stats.hint_used += 1; ptr }
            None => {
                let flags = cpu.get_flags();
                match tb_find(shared, per_cpu, cpu, iter_pc, flags) {
                    Some(ptr) => ptr,
                    None => return ExitReason::BufferFull,
                }
            }
        };

        let raw_exit = cpu_tb_exec(shared, cpu, tb_ptr);
        let (last_tb, exit_code) = decode_tb_exit(raw_exit);
        let src_tb = last_tb.unwrap_or(tb_ptr);

        match exit_code {
            v @ 0..=1 => {
                per_cpu.stats.chain_exit[v] += 1;
                let pc = cpu.get_pc();
                let flags = cpu.get_flags();
                let dst = match tb_find(shared, per_cpu, cpu, pc, flags) {
                    Some(ptr) => ptr,
                    None => return ExitReason::BufferFull,
                };
                if tb_limit == 0 {
                    tb_add_jump(shared, per_cpu, src_tb, v, dst);
                }
                next_tb_hint = Some(dst);
            }
            v if v == TB_EXIT_NOCHAIN as usize => {
                per_cpu.stats.nochain_exit += 1;
                let pc = cpu.get_pc();
                let flags = cpu.get_flags();

                let stb = &*src_tb;
                let cached_raw = stb.exit_target.load(Ordering::Relaxed);
                if cached_raw != 0 {
                    let cached = cached_raw as *mut TranslationBlock;
                    let cached_tb = &*cached;
                    if !cached_tb.invalid.load(Ordering::Acquire)
                        && cached_tb.pc == pc
                        && cached_tb.flags == flags
                        && cached_tb.host_size > 0
                    {
                        per_cpu.stats.exit_target_hit += 1;
                        // Mark as indirect target
                        cached_tb.indirect_target.store(
                            true,
                            Ordering::Relaxed,
                        );
                        next_tb_hint = Some(cached);
                        continue;
                    }
                    per_cpu.stats.exit_target_miss += 1;
                }

                let dst = match tb_find(shared, per_cpu, cpu, pc, flags) {
                    Some(ptr) => ptr,
                    None => return ExitReason::BufferFull,
                };
                // Mark as indirect target
                (&*dst).indirect_target.store(
                    true,
                    Ordering::Relaxed,
                );
                stb.exit_target.store(dst as usize, Ordering::Relaxed);
                next_tb_hint = Some(dst);
            }
            _ => {
                per_cpu.stats.real_exit += 1;
                return ExitReason::Exit(exit_code);
            }
        }
    }
}

fn tb_find<B, C>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, cpu: &mut C, pc: u64, flags: u32,
) -> Option<*mut TranslationBlock>
where B: HostCodeGen, C: GuestCpu,
{
    if let Some(ptr) = per_cpu.jump_cache.lookup(pc) {
        let tb = unsafe { &*ptr };
        if !tb.invalid.load(Ordering::Acquire)
            && tb.pc == pc
            && tb.flags == flags
            && tb.host_size > 0
        {
            per_cpu.stats.jc_hit += 1;
            return Some(ptr);
        }
    }
    if let Some(ptr) = shared.tb_store.lookup(pc, flags) {
        per_cpu.jump_cache.insert(pc, ptr);
        per_cpu.stats.ht_hit += 1;
        return Some(ptr);
    }
    per_cpu.stats.translate += 1;
    tb_gen_code(shared, per_cpu, cpu, pc, flags)
}

fn tb_gen_code<B, C>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, cpu: &mut C, pc: u64, flags: u32,
) -> Option<*mut TranslationBlock>
where B: HostCodeGen, C: GuestCpu,
{
    if shared.code_buf().remaining() < MIN_CODE_BUF_REMAINING { return None; }

#[cfg(feature = "llvm")]
fn llvm_helper_brcond_fallback(ir: &tcg_core::Context) -> bool {
    let ops = ir.ops();
    if !ops.iter().any(|op| op.opc == tcg_core::Opcode::GotoTb) {
        return false;
    }
    ops.windows(2).any(|w| w[0].opc == tcg_core::Opcode::Call && w[1].opc == tcg_core::Opcode::BrCond)
}

    let mut guard = shared.translate_lock.lock().unwrap();

    if let Some(ptr) = shared.tb_store.lookup(pc, flags) {
        per_cpu.jump_cache.insert(pc, ptr);
        return Some(ptr);
    }

    let tb_ptr = shared.tb_store.alloc(pc, flags, 0);

    guard.ir_ctx.reset();
    guard.ir_ctx.tb_ptr = tb_ptr as usize;
    let guest_size = cpu.gen_code(&mut guard.ir_ctx, pc, tcg_core::tb::TranslationBlock::max_insns(0));
    unsafe { shared.tb_store.get_mut(tb_ptr).size = guest_size; }

    shared.backend.clear_goto_tb_offsets();
    let code_buf_mut = unsafe { shared.code_buf_mut() };

    // Check AOT table first
    let aot_addr = shared.aot_table.as_ref().and_then(|t| t.lookup(pc));

    let host_offset = if let Some(func_addr) = aot_addr {
        // Emit trampoline to AOT function.
        // AOT functions use tb_ptr=0 in their exit encoding, so we re-encode
        // with the correct tb_ptr after the call.
        let tb_start = code_buf_mut.offset();
        code_buf_mut.emit_bytes(&[0x48, 0x89, 0xef]); // mov rdi, rbp
        code_buf_mut.emit_bytes(&[0x4c, 0x89, 0xf6]); // mov rsi, r14
        code_buf_mut.emit_bytes(&[0x48, 0xb8]);        // movabs rax,
        code_buf_mut.emit_u64(func_addr);
        code_buf_mut.emit_bytes(&[0xff, 0xd0]);        // call rax
        // AOT returns bare slot (0/1/2) because tb_ptr=0 at compile time.
        // Real exits (ECALL=3, EBREAK=4, UNDEF=5...) are > TB_EXIT_NOCHAIN(2).
        // Re-encode slot exits (rax <= 2): rax = tb_ptr | (rax & 7).
        // Pass real exits (rax >= 3) through unchanged.
        code_buf_mut.emit_bytes(&[0x48, 0x83, 0xf8, 0x02]); // cmp rax, 2
        code_buf_mut.emit_bytes(&[0x77, 0x11]);              // ja +17 (skip re-encode)
        // Re-encode: rax = (tb_ptr as usize) | (rax & 7)
        code_buf_mut.emit_bytes(&[0x48, 0x83, 0xe0, 0x07]); // and rax, 7
        code_buf_mut.emit_bytes(&[0x48, 0xb9]);              // movabs rcx,
        code_buf_mut.emit_u64(tb_ptr as u64);
        code_buf_mut.emit_bytes(&[0x48, 0x09, 0xc8]);        // or rax, rcx
        // jmp epilogue
        code_buf_mut.emit_u8(0xe9);                    // jmp rel32
        let jmp_site = code_buf_mut.offset();
        let epi = shared.backend.epilogue_offset();
        let rel = (epi as i64) - ((jmp_site + 4) as i64);
        code_buf_mut.emit_u32(rel as u32);
        tb_start
    } else {
        #[cfg(feature = "llvm")]
        {
            let TranslateGuard { ref mut ir_ctx, ref mut llvm_jit } = *guard;
            let llvm_max_pc = match std::env::var("TCG_LLVM_MAX_PC") {
                Ok(s) => pc <= u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(u64::MAX),
                Err(_) => true,
            };
            let llvm_helper_brcond = std::env::var("TCG_LLVM_HELPER_BRCOND_FALLBACK").is_ok()
                && llvm_helper_brcond_fallback(ir_ctx);
            let use_llvm = llvm_jit.is_some() && (llvm_max_pc || llvm_helper_brcond);
            if use_llvm {
                let prof_addr = if shared.profiling {
                    unsafe { std::ptr::addr_of!((*tb_ptr).exec_count) as u64 }
                } else {
                    0
                };
                tcg_backend::translate::translate_llvm(
                    ir_ctx, llvm_jit.as_mut().unwrap(), code_buf_mut, shared.backend.epilogue_offset(),
                    if shared.profiling { Some(prof_addr) } else { None },
                )
            } else {
                let prof_addr = if shared.profiling {
                    unsafe { std::ptr::addr_of!((*tb_ptr).exec_count) as u64 }
                } else {
                    0
                };
                translate(ir_ctx, &shared.backend, code_buf_mut, if shared.profiling { Some(prof_addr) } else { None })
            }
        }
        #[cfg(not(feature = "llvm"))]
        {
            let prof_addr = if shared.profiling {
                unsafe { std::ptr::addr_of!((*tb_ptr).exec_count) as u64 }
            } else {
                0
            };
            translate(&mut guard.ir_ctx, &shared.backend, code_buf_mut, if shared.profiling { Some(prof_addr) } else { None })
        }
    };

    let host_size = shared.code_buf().offset() - host_offset;
    unsafe {
        let tb = shared.tb_store.get_mut(tb_ptr);
        tb.host_offset = host_offset;
        tb.host_size = host_size;
    }

    if aot_addr.is_none() {
        let offsets = shared.backend.goto_tb_offsets();
        unsafe {
            let tb = shared.tb_store.get_mut(tb_ptr);
            for &(slot, jmp, reset) in offsets.iter() {
                let slot = slot as usize;
                if slot < 2 {
                    tb.set_jmp_insn_offset(slot, jmp as u32);
                    tb.set_jmp_reset_offset(slot, reset as u32);
                }
            }
        }
    }

    // SAFETY: tb_ptr was just returned by tb_store.alloc() above and has not been freed.
    unsafe { shared.tb_store.insert(tb_ptr); }
    per_cpu.jump_cache.insert(pc, tb_ptr);
    Some(tb_ptr)
}

unsafe fn cpu_tb_exec<B, C>(shared: &SharedState<B>, cpu: &mut C, tb_ptr: *mut TranslationBlock) -> usize
where B: HostCodeGen, C: GuestCpu,
{
    let tb = &*tb_ptr;
    let tb_code_ptr = shared.code_buf().ptr_at(tb.host_offset);
    let env_ptr = cpu.env_ptr();
    let prologue_fn: unsafe extern "C" fn(*mut u8, *const u8) -> usize =
        core::mem::transmute(shared.code_buf().base_ptr());
    prologue_fn(env_ptr, tb_code_ptr)
}

fn tb_add_jump<B: HostCodeGen>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState,
    src: *mut TranslationBlock, slot: usize, dst: *mut TranslationBlock,
) {
    let src_tb = unsafe { &*src };
    let jmp_off = match src_tb.jmp_insn_offset[slot] {
        Some(off) => off as usize,
        None => return,
    };
    if unsafe { &*dst }.invalid.load(Ordering::Acquire) { return; }

    let mut src_jmp = src_tb.jmp.lock().unwrap();
    if src_jmp.jmp_dest[slot] == Some(dst) {
        per_cpu.stats.chain_already += 1;
        return;
    }

    let abs_dst = unsafe { &*dst }.host_offset;
    shared.backend.patch_jump(shared.code_buf(), jmp_off, abs_dst);
    src_jmp.jmp_dest[slot] = Some(dst);
    drop(src_jmp);

    let dst_tb = unsafe { &*dst };
    let mut dst_jmp = dst_tb.jmp.lock().unwrap();
    dst_jmp.jmp_list.push((src, slot));

    per_cpu.stats.chain_patched += 1;
}
