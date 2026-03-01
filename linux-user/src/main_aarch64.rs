use std::env;
use std::process;
use std::sync::atomic::Ordering;

use tcg_backend::X86_64CodeGen;
use tcg_core::context::Context;
use tcg_core::tb::{EXCP_ECALL, EXCP_UNDEF};
use tcg_core::TempIdx;
use tcg_exec::exec_loop::{cpu_exec_loop, ExitReason};
use tcg_exec::profile::{
    ProfileData, ProfileEntry, DEFAULT_HOT_THRESHOLD,
};
use tcg_exec::{AotTable, ExecEnv, GuestCpu};
use tcg_frontend::aarch64::cpu::{
    Aarch64Cpu, NUM_XREGS,
};
use tcg_frontend::aarch64::{
    Aarch64DisasContext, Aarch64Translator,
};
use tcg_frontend::{
    translator_loop, DisasJumpType, TranslatorOps,
};
use tcg_linux_user::elf::EM_AARCH64;
use tcg_linux_user::guest_space::GuestSpace;
use tcg_linux_user::loader::{load_elf, ElfInfo};
use tcg_linux_user::syscall::SyscallResult;
use tcg_linux_user::syscall_aarch64::handle_syscall_aarch64;

/// Wrapper: Aarch64Cpu for GuestCpu trait.
struct LinuxCpu {
    cpu: Aarch64Cpu,
    single_step: bool,
}

impl GuestCpu for LinuxCpu {
    fn get_pc(&self) -> u64 {
        self.cpu.pc
    }

    fn get_flags(&self) -> u32 {
        0
    }

    fn gen_code(
        &mut self,
        ir: &mut Context,
        pc: u64,
        max_insns: u32,
    ) -> u32 {
        let max_insns = if self.single_step { 1 } else { max_insns };
        let base = self.cpu.guest_base as *const u8;
        if ir.nb_globals() == 0 {
            let mut d =
                Aarch64DisasContext::new(pc, base);
            d.base.max_insns = max_insns;
            translator_loop::<Aarch64Translator>(
                &mut d, ir,
            );
            d.base.num_insns * 4
        } else {
            let mut d =
                Aarch64DisasContext::new(pc, base);
            d.base.max_insns = max_insns;
            d.env = TempIdx(0);
            for i in 0..NUM_XREGS {
                d.xregs[i] = TempIdx(1 + i as u32);
            }
            d.pc = TempIdx(1 + NUM_XREGS as u32);
            d.sp = TempIdx(2 + NUM_XREGS as u32);
            d.nzcv = TempIdx(3 + NUM_XREGS as u32);
            Aarch64Translator::tb_start(&mut d, ir);
            loop {
                Aarch64Translator::insn_start(
                    &mut d, ir,
                );
                Aarch64Translator::translate_insn(
                    &mut d, ir,
                );
                if d.base.is_jmp != DisasJumpType::Next
                {
                    break;
                }
                if d.base.num_insns >= d.base.max_insns
                {
                    d.base.is_jmp =
                        DisasJumpType::TooMany;
                    break;
                }
            }
            Aarch64Translator::tb_stop(&mut d, ir);
            d.base.num_insns * 4
        }
    }

    fn env_ptr(&mut self) -> *mut u8 {
        &mut self.cpu as *mut Aarch64Cpu as *mut u8
    }
}

/// Mini AArch64 interpreter for IFUNC resolvers.
/// These are simple functions that check HWCAP and return a function pointer.
/// We simulate with x0=0 (HWCAP=0), all memory reads return 0.
fn resolve_ifunc_static(
    space: &tcg_linux_user::guest_space::GuestSpace,
    entry: u64,
) -> u64 {
    let mut regs = [0u64; 32]; // x0-x31
    let mut pc = entry;
    let mut nzcv: u32 = 0; // N=3, Z=2, C=1, V=0

    for _ in 0..100 {
        let insn = unsafe {
            let p = space.g2h(pc) as *const u32;
            p.read_unaligned()
        };

        if insn == 0xd65f03c0 {
            // RET — return x0
            return regs[0];
        }

        let result = interp_one(insn, &mut regs, &mut nzcv, &mut pc, space);
        if !result {
            // Unknown instruction — fall back to resolver address itself
            eprintln!(
                "[ifunc] unknown insn {:#010x} at pc={:#x}, using resolver addr",
                insn, pc,
            );
            return entry;
        }
        pc += 4;
    }
    eprintln!("[ifunc] resolver at {:#x} exceeded step limit", entry);
    entry
}

fn interp_one(
    insn: u32,
    regs: &mut [u64; 32],
    nzcv: &mut u32,
    pc: &mut u64,
    space: &tcg_linux_user::guest_space::GuestSpace,
) -> bool {
    let rd = (insn & 0x1f) as usize;
    let rn = ((insn >> 5) & 0x1f) as usize;

    // ADRP Xd, #imm
    if insn & 0x9f00_0000 == 0x9000_0000 {
        let immlo = ((insn >> 29) & 0x3) as i64;
        let immhi = (((insn >> 5) & 0x7ffff) as i32) << 13 >> 13;
        let imm = ((immhi as i64) << 2) | immlo;
        let base = *pc & !0xfff;
        regs[rd] = (base as i64 + (imm << 12)) as u64;
        return true;
    }

    // ADD Xd, Xn, #imm12 (sf=1)
    if insn & 0xff00_0000 == 0x9100_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u64;
        let sh = (insn >> 22) & 1;
        let val = if sh != 0 { imm12 << 12 } else { imm12 };
        regs[rd] = regs[rn].wrapping_add(val);
        return true;
    }

    // LDR Xt, [Xn, #imm12] (64-bit, unsigned offset)
    if insn & 0xffc0_0000 == 0xf940_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u64;
        let addr = regs[rn].wrapping_add(imm12 * 8);
        regs[rd] = if addr < 0x1_0000_0000 {
            unsafe { (space.g2h(addr) as *const u64).read_unaligned() }
        } else {
            0
        };
        return true;
    }

    // LDRB Wt, [Xn, #imm12] (unsigned offset)
    if insn & 0xffc0_0000 == 0x3940_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u64;
        let addr = regs[rn].wrapping_add(imm12);
        regs[rd] = if addr < 0x1_0000_0000 {
            unsafe { *(space.g2h(addr)) as u64 }
        } else {
            0
        };
        return true;
    }

    // LDR Wt, [Xn, #imm12] (32-bit, unsigned offset)
    if insn & 0xffc0_0000 == 0xb940_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u64;
        let addr = regs[rn].wrapping_add(imm12 * 4);
        regs[rd] = if addr < 0x1_0000_0000 {
            unsafe { (space.g2h(addr) as *const u32).read_unaligned() as u64 }
        } else {
            0
        };
        return true;
    }

    // LSR Xd, Xn, #imm (UBFM alias: sf=1, N=1, immr=shift, imms=63)
    if insn & 0xffc0_0000 == 0xd340_0000 {
        let immr = ((insn >> 16) & 0x3f) as u32;
        regs[rd] = regs[rn] >> immr;
        return true;
    }

    // UBFX Xd, Xn, #lsb, #width (UBFM: sf=1, N=1)
    if insn & 0xffc0_0000 == 0xd340_0000 {
        // Already handled by LSR above
        return true;
    }

    // UBFM general (sf=1, N=1): 1101 0011 01 immr imms Rn Rd
    if insn & 0xff80_0000 == 0xd340_0000 {
        let immr = ((insn >> 16) & 0x3f) as u32;
        let imms = ((insn >> 10) & 0x3f) as u32;
        let width = imms + 1;
        let mask = (1u64 << width) - 1;
        regs[rd] = (regs[rn] >> immr) & mask;
        return true;
    }

    // MOV Xd, #imm16 (MOVZ: sf=1, hw=0)
    if insn & 0xffe0_0000 == 0xd280_0000 {
        let imm16 = ((insn >> 5) & 0xffff) as u64;
        let hw = ((insn >> 21) & 3) as u64;
        regs[rd] = imm16 << (hw * 16);
        return true;
    }

    // MOVZ Wd, #imm16 (sf=0)
    if insn & 0xffe0_0000 == 0x5280_0000 {
        let imm16 = ((insn >> 5) & 0xffff) as u64;
        let hw = ((insn >> 21) & 3) as u64;
        regs[rd] = imm16 << (hw * 16);
        return true;
    }

    // TST Xn, #imm (ANDS XZR, Xn, #imm): sf=1 opc=11 100100 N immr imms Rn Rd
    if insn & 0xff80_0000 == 0xf200_0000 {
        let val = regs[rn];
        // Decode bitmask immediate
        if let Some(mask) = decode_bitmask_64(insn) {
            let result = val & mask;
            *nzcv = if result == 0 { 0b0100 } else { 0 };
            if result >> 63 != 0 { *nzcv |= 0b1000; }
        }
        return true;
    }

    // CMP Xn, #imm12 (SUBS XZR, Xn, #imm12)
    if insn & 0xff00_0000 == 0xf100_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u64;
        let sh = (insn >> 22) & 1;
        let val = if sh != 0 { imm12 << 12 } else { imm12 };
        let a = regs[rn];
        let result = a.wrapping_sub(val);
        *nzcv = 0;
        if result == 0 { *nzcv |= 0b0100; } // Z
        if result >> 63 != 0 { *nzcv |= 0b1000; } // N
        if a >= val { *nzcv |= 0b0010; } // C
        return true;
    }

    // CMP Wn, #imm12 (SUBS WZR, Wn, #imm12, sf=0)
    if insn & 0xff00_0000 == 0x7100_0000 {
        let imm12 = ((insn >> 10) & 0xfff) as u32;
        let a = regs[rn] as u32;
        let result = a.wrapping_sub(imm12);
        *nzcv = 0;
        if result == 0 { *nzcv |= 0b0100; }
        if result >> 31 != 0 { *nzcv |= 0b1000; }
        if a >= imm12 { *nzcv |= 0b0010; }
        return true;
    }

    // CSEL Xd, Xn, Xm, cond
    if insn & 0xffe0_0c00 == 0x9a80_0000 {
        let rm = ((insn >> 16) & 0x1f) as usize;
        let cond = ((insn >> 12) & 0xf) as u32;
        regs[rd] = if cond_holds(*nzcv, cond) { regs[rn] } else { regs[rm] };
        return true;
    }

    // B.cond #imm19
    if insn & 0xff00_0010 == 0x5400_0000 {
        let cond = (insn & 0xf) as u32;
        if cond_holds(*nzcv, cond) {
            let imm19 = ((insn >> 5) & 0x7ffff) as i32;
            let offset = ((imm19 << 13) >> 13) as i64 * 4;
            *pc = (*pc as i64 + offset) as u64;
            *pc -= 4; // will be incremented by caller
        }
        return true;
    }

    // B #imm26
    if insn & 0xfc00_0000 == 0x1400_0000 {
        let imm26 = (insn & 0x03ff_ffff) as i32;
        let offset = ((imm26 << 6) >> 6) as i64 * 4;
        *pc = (*pc as i64 + offset) as u64;
        *pc -= 4;
        return true;
    }

    // CBZ Xn, #imm19 (sf=1)
    if insn & 0xff00_0000 == 0xb400_0000 {
        let rt = (insn & 0x1f) as usize;
        if regs[rt] == 0 {
            let imm19 = ((insn >> 5) & 0x7ffff) as i32;
            let offset = ((imm19 << 13) >> 13) as i64 * 4;
            *pc = (*pc as i64 + offset) as u64;
            *pc -= 4;
        }
        return true;
    }

    // CBZ Wn, #imm19 (sf=0)
    if insn & 0xff00_0000 == 0x3400_0000 {
        let rt = (insn & 0x1f) as usize;
        if regs[rt] as u32 == 0 {
            let imm19 = ((insn >> 5) & 0x7ffff) as i32;
            let offset = ((imm19 << 13) >> 13) as i64 * 4;
            *pc = (*pc as i64 + offset) as u64;
            *pc -= 4;
        }
        return true;
    }

    // CCMP Xn, #imm5, #nzcv, cond
    if insn & 0xffe0_0c10 == 0xfa40_0800 {
        let imm5 = ((insn >> 16) & 0x1f) as u64;
        let cond = ((insn >> 12) & 0xf) as u32;
        let alt_nzcv = (insn & 0xf) as u32;
        if cond_holds(*nzcv, cond) {
            let a = regs[rn];
            let result = a.wrapping_sub(imm5);
            *nzcv = 0;
            if result == 0 { *nzcv |= 0b0100; }
            if result >> 63 != 0 { *nzcv |= 0b1000; }
            if a >= imm5 { *nzcv |= 0b0010; }
        } else {
            *nzcv = alt_nzcv;
        }
        return true;
    }

    // NOP
    if insn == 0xd503201f {
        return true;
    }

    false
}

fn cond_holds(nzcv: u32, cond: u32) -> bool {
    let n = (nzcv >> 3) & 1 != 0;
    let z = (nzcv >> 2) & 1 != 0;
    let c = (nzcv >> 1) & 1 != 0;
    let v = nzcv & 1 != 0;
    let base = match cond >> 1 {
        0 => z,           // EQ/NE
        1 => c,           // CS/CC (HS/LO)
        2 => n,           // MI/PL
        3 => v,           // VS/VC
        4 => c && !z,     // HI/LS
        5 => n == v,      // GE/LT
        6 => n == v && !z, // GT/LE
        7 => true,        // AL
        _ => unreachable!(),
    };
    if cond & 1 != 0 && cond != 0xf { !base } else { base }
}

fn decode_bitmask_64(insn: u32) -> Option<u64> {
    let n = (insn >> 22) & 1;
    let immr = ((insn >> 16) & 0x3f) as u32;
    let imms = ((insn >> 10) & 0x3f) as u32;
    let len = 63 - (((!((imms as u64) | (!n as u64) << 6)) << 57) >> 57).leading_zeros();
    if len > 6 { return None; }
    let size = 1u32 << len;
    let levels = size - 1;
    let s = imms & levels;
    let r = immr & levels;
    let welem = (1u64 << (s + 1)) - 1;
    let elem = if r == 0 { welem } else { (welem >> r) | (welem << (size - r)) };
    let mask = (0..64).step_by(size as usize).fold(0u64, |acc, sh| {
        acc | ((elem & ((1u64 << size) - 1)) << sh)
    });
    Some(mask)
}

fn save_profile<B: tcg_backend::HostCodeGen>(
    env: &ExecEnv<B>,
    load_vaddr: u64,
) {
    let out = std::env::var("TCG_PROFILE_OUT")
        .unwrap_or_else(|_| "profile.bin".into());
    let path = std::path::Path::new(&out);
    let shared = &env.shared;
    let tb_count = shared.tb_store.len();
    let profiles =
        unsafe { &*shared.tb_profiles.get() };

    // Load existing profile entries and accumulate
    let mut accumulated: std::collections::HashMap<
        u64,
        ProfileEntry,
    > = ProfileData::load(path)
        .map(|existing| {
            eprintln!(
                "[tcg] accumulating with existing \
                 profile ({} entries)",
                existing.entries.len()
            );
            existing
                .entries
                .into_iter()
                .map(|e| (e.file_offset, e))
                .collect()
        })
        .unwrap_or_default();

    for i in 0..tb_count {
        let tb = shared.tb_store.get(i);
        let prof = &profiles[i];
        let exec =
            prof.exec_count.load(Ordering::Relaxed);
        let file_offset = tb.pc - load_vaddr;

        if exec >= DEFAULT_HOT_THRESHOLD {
            let entry = accumulated
                .entry(file_offset)
                .or_insert(ProfileEntry {
                    file_offset,
                    exec_count: exec,
                });
            if exec > entry.exec_count {
                entry.exec_count = exec;
            }
        }
    }

    let entries: Vec<ProfileEntry> =
        accumulated.into_values().collect();
    let data = ProfileData {
        threshold: DEFAULT_HOT_THRESHOLD as u32,
        entries,
    };
    if let Err(e) = data.save(path) {
        eprintln!("[tcg] failed to save profile: {e}");
    } else {
        eprintln!(
            "[tcg] profile saved to {out} \
             ({} hot TBs)",
            data.entries.len()
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: tcg-aarch64 <elf> [args...]");
        process::exit(1);
    }

    let elf_path = std::fs::canonicalize(&args[1])
        .expect("failed to resolve elf path");
    let elf_path = elf_path.to_str().unwrap();
    let guest_argv: Vec<&str> =
        args[1..].iter().map(|s| s.as_str()).collect();

    // Load ELF
    let mut space = GuestSpace::new()
        .expect("failed to create guest space");
    let info: ElfInfo = load_elf(
        std::path::Path::new(elf_path),
        &mut space,
        &guest_argv,
        &[],
        EM_AARCH64,
    )
    .expect("failed to load ELF");

    // Set up CPU
    let single_step = env::var("TCG_SINGLE").is_ok();
    let mut lcpu = LinuxCpu {
        cpu: Aarch64Cpu::new(),
        single_step,
    };
    lcpu.cpu.pc = info.entry;
    lcpu.cpu.sp = info.sp;
    lcpu.cpu.guest_base = space.guest_base() as u64;
    // Store load bias for AOT dispatch (file_offset = pc - load_bias)
    lcpu.cpu.load_bias = info.load_vaddr;
    eprintln!("[tcg] guest_base={:#x}", lcpu.cpu.guest_base);

    // mmap_next starts after brk
    let mut mmap_next =
        tcg_linux_user::guest_space::page_align_up(
            info.brk,
        ) + 0x1000_0000;

    // Install SIGSEGV handler to dump guest state
    unsafe {
        unsafe extern "C" fn sigsegv_handler(
            _sig: i32,
            info: *mut libc::siginfo_t,
            ctx: *mut libc::c_void,
        ) {
            let uc = ctx as *const libc::ucontext_t;
            let mctx = &(*uc).uc_mcontext;
            let rip = mctx.gregs[libc::REG_RIP as usize] as u64;
            let rbp_val = mctx.gregs[libc::REG_RBP as usize] as u64;
            let rsp_val = mctx.gregs[libc::REG_RSP as usize] as u64;
            let fault_addr = (*info).si_addr() as u64;
            eprintln!("=== SIGSEGV ===");
            eprintln!(
                "RIP={:#x} RBP={:#x} RSP={:#x} fault_addr={:#x}",
                rip, rbp_val, rsp_val, fault_addr,
            );
            // Only dump guest state if RBP looks like a valid pointer
            if rbp_val > 0x1000 && rbp_val < 0x7fff_ffff_ffff {
                let rbp = rbp_val as *const u64;
                for i in 0..31usize {
                    let v = *rbp.add(i);
                    eprint!("x{}={:#018x} ", i, v);
                    if (i + 1) % 4 == 0 { eprintln!(); }
                }
                eprintln!();
                let pc = *rbp.add(31);
                let sp = *rbp.add(32);
                let gb = *rbp.add(33);
                let nzcv = *rbp.add(34);
                eprintln!(
                    "pc={:#018x} sp={:#018x} gb={:#018x} nzcv={:#018x}",
                    pc, sp, gb, nzcv,
                );
            } else {
                eprintln!("RBP invalid, cannot dump guest state");
            }
            libc::_exit(139);
        }
        let mut sa: libc::sigaction =
            std::mem::zeroed();
        sa.sa_sigaction =
            sigsegv_handler as unsafe extern "C" fn(i32, *mut libc::siginfo_t, *mut libc::c_void) as usize;
        sa.sa_flags = libc::SA_SIGINFO;
        libc::sigaction(
            libc::SIGSEGV, &sa, std::ptr::null_mut(),
        );
    }

    // Run
    let show_stats = env::var("TCG_STATS").is_ok();
    let profiling = env::var("TCG_PROFILE").is_ok();
    let show_trace = env::var("TCG_TRACE").is_ok();

    // Load AOT if specified
    let aot = env::var("TCG_AOT").ok().and_then(|p| {
        let t = AotTable::load(
            std::path::Path::new(&p),
            info.load_vaddr,
        );
        if t.is_some() {
            eprintln!("[tcg] AOT loaded from {p}");
        } else {
            eprintln!(
                "[tcg] warning: failed to load AOT \
                 from {p}"
            );
        }
        t
    });

    let mut codegen = X86_64CodeGen::new();
    codegen.guest_base_offset =
        tcg_frontend::aarch64::cpu::GUEST_BASE_OFFSET as i32;
    let env =
        ExecEnv::new_with_opts(codegen, profiling, aot);
    #[cfg(feature = "llvm")]
    if std::env::var("TCG_LLVM").is_ok() {
        eprintln!("[tcg] LLVM JIT backend enabled");
        env.enable_llvm();
    }
    if profiling {
        eprintln!("[tcg] profiling enabled");
    }
    let mut env = env;

    // Resolve IRELATIVE relocations statically.
    // Each relocation has a resolver function address. We simulate
    // the resolver by interpreting its code directly: the resolvers
    // check AT_HWCAP (which we set to 0) and return a function pointer.
    // We use a mini-interpreter to handle the simple resolver patterns.
    for &(got_offset, resolver_addr) in &info.irelatives {
        let result = resolve_ifunc_static(&space, resolver_addr);
        unsafe { space.write_u64(got_offset, result); }
        if show_trace {
            eprintln!(
                "[ifunc] GOT[{:#x}] = {:#x} (resolver {:#x})",
                got_offset, result, resolver_addr,
            );
        }
    }

    let mut icount: u64 = 0;
    loop {
        if show_trace {
            eprintln!(
                "[trace] pc={:#x} sp={:#x} i={}",
                lcpu.cpu.pc, lcpu.cpu.sp, icount,
            );
            icount += 1;
        }
        let reason =
            unsafe { cpu_exec_loop(&mut env, &mut lcpu) };
        match reason {
            ExitReason::Exit(v)
                if v == EXCP_ECALL as usize =>
            {
                // SVC (syscall)
                if show_trace {
                    eprintln!(
                        "[syscall] nr={} pc={:#x} x0={:#x} x1={:#x} x2={:#x}",
                        lcpu.cpu.xregs[8], lcpu.cpu.pc,
                        lcpu.cpu.xregs[0], lcpu.cpu.xregs[1], lcpu.cpu.xregs[2],
                    );
                }
                match handle_syscall_aarch64(
                    &mut space,
                    &mut lcpu.cpu.xregs,
                    &mut lcpu.cpu.sp,
                    &mut mmap_next,
                    elf_path,
                ) {
                    SyscallResult::Continue(ret) => {
                        lcpu.cpu.xregs[0] = ret;
                        lcpu.cpu.pc += 4;
                    }
                    SyscallResult::Exit(code) => {
                        if show_stats {
                            eprint!(
                                "{}",
                                env.per_cpu.stats
                            );
                        }
                        if profiling {
                            save_profile(
                                &env,
                                info.load_vaddr,
                            );
                        }
                        process::exit(code);
                    }
                }
            }
            ExitReason::Exit(v)
                if v == EXCP_UNDEF as usize =>
            {
                if show_stats {
                    eprint!("{}", env.per_cpu.stats);
                }
                if profiling {
                    save_profile(&env, info.load_vaddr);
                }
                eprintln!(
                    "illegal instruction at pc={:#x}",
                    lcpu.cpu.pc
                );
                process::exit(1);
            }
            ExitReason::Exit(v) => {
                if show_stats {
                    eprint!("{}", env.per_cpu.stats);
                }
                if profiling {
                    save_profile(&env, info.load_vaddr);
                }
                eprintln!("unexpected exit {v}");
                process::exit(1);
            }
            ExitReason::BufferFull => {
                if show_stats {
                    eprint!("{}", env.per_cpu.stats);
                }
                if profiling {
                    save_profile(&env, info.load_vaddr);
                }
                eprintln!("code buffer full");
                process::exit(1);
            }
        }
    }
}
