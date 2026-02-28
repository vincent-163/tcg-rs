use std::sync::atomic::Ordering;

use crate::{
    ExecEnv, GuestCpu, PerCpuState, SharedState, TranslateGuard, MIN_CODE_BUF_REMAINING,
};
use tcg_backend::translate::translate;
use tcg_backend::HostCodeGen;
use tcg_core::tb::{decode_tb_exit, EXIT_TARGET_NONE, TB_EXIT_NOCHAIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    Exit(usize),
    BufferFull,
}

pub unsafe fn cpu_exec_loop<B, C>(env: &mut ExecEnv<B>, cpu: &mut C) -> ExitReason
where B: HostCodeGen, C: GuestCpu,
{
    cpu_exec_loop_mt(&env.shared, &mut env.per_cpu, cpu)
}

pub unsafe fn cpu_exec_loop_mt<B, C>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, cpu: &mut C,
) -> ExitReason
where B: HostCodeGen, C: GuestCpu,
{
    let mut next_tb_hint: Option<usize> = None;
    let tb_limit: u64 = std::env::var("TCG_TB_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let tb_trace = tb_limit > 0;
    let mut tb_count: u64 = 0;
    let mut last_pcs: [u64; 8] = [0; 8];

    loop {
        per_cpu.stats.loop_iters += 1;
        if tb_trace {
            let pc = cpu.get_pc();
            last_pcs[(tb_count as usize) & 7] = pc;
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

        let tb_idx = match next_tb_hint.take() {
            Some(idx) => { per_cpu.stats.hint_used += 1; idx }
            None => {
                let pc = cpu.get_pc();
                let flags = cpu.get_flags();
                match tb_find(shared, per_cpu, cpu, pc, flags) {
                    Some(idx) => idx,
                    None => return ExitReason::BufferFull,
                }
            }
        };

        if shared.profiling {
            shared.tb_profile(tb_idx).exec_count.fetch_add(1, Ordering::Relaxed);
        }

        let raw_exit = cpu_tb_exec(shared, cpu, tb_idx);
        let (last_tb, exit_code) = decode_tb_exit(raw_exit);
        let src_tb = last_tb.unwrap_or(tb_idx);

        match exit_code {
            v @ 0..=1 => {
                per_cpu.stats.chain_exit[v] += 1;
                let pc = cpu.get_pc();
                let flags = cpu.get_flags();
                let dst = match tb_find(shared, per_cpu, cpu, pc, flags) {
                    Some(idx) => idx,
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

                let stb = shared.tb_store.get(src_tb);
                let cached = stb.exit_target.load(Ordering::Relaxed);
                if cached != EXIT_TARGET_NONE {
                    let tb = shared.tb_store.get(cached);
                    if !tb.invalid.load(Ordering::Acquire) && tb.pc == pc && tb.flags == flags {
                        if shared.profiling {
                            shared.tb_profile(cached).indirect_count.fetch_add(1, Ordering::Relaxed);
                        }
                        next_tb_hint = Some(cached);
                        continue;
                    }
                }

                let dst = match tb_find(shared, per_cpu, cpu, pc, flags) {
                    Some(idx) => idx,
                    None => return ExitReason::BufferFull,
                };
                if shared.profiling {
                    shared.tb_profile(dst).indirect_count.fetch_add(1, Ordering::Relaxed);
                }
                stb.exit_target.store(dst, Ordering::Relaxed);
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
) -> Option<usize>
where B: HostCodeGen, C: GuestCpu,
{
    if let Some(idx) = per_cpu.jump_cache.lookup(pc) {
        let tb = shared.tb_store.get(idx);
        if !tb.invalid.load(Ordering::Acquire) && tb.pc == pc && tb.flags == flags {
            per_cpu.stats.jc_hit += 1;
            return Some(idx);
        }
    }
    if let Some(idx) = shared.tb_store.lookup(pc, flags) {
        per_cpu.jump_cache.insert(pc, idx);
        per_cpu.stats.ht_hit += 1;
        return Some(idx);
    }
    per_cpu.stats.translate += 1;
    tb_gen_code(shared, per_cpu, cpu, pc, flags)
}

fn tb_gen_code<B, C>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, cpu: &mut C, pc: u64, flags: u32,
) -> Option<usize>
where B: HostCodeGen, C: GuestCpu,
{
    if shared.code_buf().remaining() < MIN_CODE_BUF_REMAINING { return None; }

    let mut guard = shared.translate_lock.lock().unwrap();

    if let Some(idx) = shared.tb_store.lookup(pc, flags) {
        per_cpu.jump_cache.insert(pc, idx);
        return Some(idx);
    }

    let tb_idx = unsafe { shared.tb_store.alloc(pc, flags, 0) };
    if shared.profiling { unsafe { shared.alloc_profile(); } }

    guard.ir_ctx.reset();
    guard.ir_ctx.tb_idx = tb_idx as u32;
    let guest_size = cpu.gen_code(&mut guard.ir_ctx, pc, tcg_core::tb::TranslationBlock::max_insns(0));
    unsafe { shared.tb_store.get_mut(tb_idx).size = guest_size; }

    shared.backend.clear_goto_tb_offsets();
    let code_buf_mut = unsafe { shared.code_buf_mut() };

    // Check AOT table first
    let aot_addr = shared.aot_table.as_ref().and_then(|t| t.lookup(pc));

    let host_offset = if let Some(func_addr) = aot_addr {
        // Emit trampoline to AOT function
        // AOT functions have tb_idx=0 in their exit encoding, so we need to
        // re-encode with the correct tb_idx after the call.
        let tb_start = code_buf_mut.offset();
        code_buf_mut.emit_bytes(&[0x48, 0x89, 0xef]); // mov rdi, rbp
        code_buf_mut.emit_bytes(&[0x4c, 0x89, 0xf6]); // mov rsi, r14
        code_buf_mut.emit_bytes(&[0x48, 0xb8]);        // movabs rax,
        code_buf_mut.emit_u64(func_addr);
        code_buf_mut.emit_bytes(&[0xff, 0xd0]);        // call rax
        // Re-encode: if upper 32 bits != 0 (TB exit), replace tb_idx
        // test eax's upper half: check if rax >> 32 != 0
        code_buf_mut.emit_bytes(&[0x48, 0x89, 0xc1]); // mov rcx, rax
        code_buf_mut.emit_bytes(&[0x48, 0xc1, 0xe9, 0x20]); // shr rcx, 32
        code_buf_mut.emit_bytes(&[0x48, 0x85, 0xc9]); // test rcx, rcx
        code_buf_mut.emit_bytes(&[0x74, 0x0e]);        // jz +14 (skip re-encode)
        // Re-encode: rax = ((tb_idx+1) << 32) | (rax & 0x3)
        code_buf_mut.emit_bytes(&[0x48, 0x83, 0xe0, 0x03]); // and rax, 3
        code_buf_mut.emit_bytes(&[0x48, 0xb9]);              // movabs rcx,
        code_buf_mut.emit_u64(((tb_idx as u64) + 1) << 32);
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
            let use_llvm = llvm_jit.is_some() && match std::env::var("TCG_LLVM_MAX_PC") {
                Ok(s) => pc <= u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(u64::MAX),
                Err(_) => true,
            };
            if use_llvm {
                tcg_backend::translate::translate_llvm(
                    ir_ctx, llvm_jit.as_mut().unwrap(), code_buf_mut, shared.backend.epilogue_offset(),
                )
            } else {
                translate(ir_ctx, &shared.backend, code_buf_mut)
            }
        }
        #[cfg(not(feature = "llvm"))]
        { translate(&mut guard.ir_ctx, &shared.backend, code_buf_mut) }
    };

    let host_size = shared.code_buf().offset() - host_offset;
    unsafe {
        let tb = shared.tb_store.get_mut(tb_idx);
        tb.host_offset = host_offset;
        tb.host_size = host_size;
    }

    if aot_addr.is_none() {
        let offsets = shared.backend.goto_tb_offsets();
        unsafe {
            let tb = shared.tb_store.get_mut(tb_idx);
            for (i, &(jmp, reset)) in offsets.iter().enumerate().take(2) {
                tb.set_jmp_insn_offset(i, jmp as u32);
                tb.set_jmp_reset_offset(i, reset as u32);
            }
        }
    }

    shared.tb_store.insert(tb_idx);
    per_cpu.jump_cache.insert(pc, tb_idx);
    Some(tb_idx)
}

unsafe fn cpu_tb_exec<B, C>(shared: &SharedState<B>, cpu: &mut C, tb_idx: usize) -> usize
where B: HostCodeGen, C: GuestCpu,
{
    let tb = shared.tb_store.get(tb_idx);
    let tb_ptr = shared.code_buf().ptr_at(tb.host_offset);
    let env_ptr = cpu.env_ptr();
    let prologue_fn: unsafe extern "C" fn(*mut u8, *const u8) -> usize =
        core::mem::transmute(shared.code_buf().base_ptr());
    prologue_fn(env_ptr, tb_ptr)
}

fn tb_add_jump<B: HostCodeGen>(
    shared: &SharedState<B>, per_cpu: &mut PerCpuState, src: usize, slot: usize, dst: usize,
) {
    let src_tb = shared.tb_store.get(src);
    let jmp_off = match src_tb.jmp_insn_offset[slot] {
        Some(off) => off as usize,
        None => return,
    };
    if shared.tb_store.get(dst).invalid.load(Ordering::Acquire) { return; }

    let mut src_jmp = src_tb.jmp.lock().unwrap();
    if src_jmp.jmp_dest[slot] == Some(dst) {
        per_cpu.stats.chain_already += 1;
        return;
    }

    let abs_dst = shared.tb_store.get(dst).host_offset;
    shared.backend.patch_jump(shared.code_buf(), jmp_off, abs_dst);
    src_jmp.jmp_dest[slot] = Some(dst);
    drop(src_jmp);

    let dst_tb = shared.tb_store.get(dst);
    let mut dst_jmp = dst_tb.jmp.lock().unwrap();
    dst_jmp.jmp_list.push((src, slot));

    // Profile: track chain source count
    if shared.profiling {
        shared.tb_profile(dst).chain_source_count.fetch_add(1, Ordering::Relaxed);
    }

    per_cpu.stats.chain_patched += 1;
}
