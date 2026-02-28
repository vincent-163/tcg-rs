use std::env;
use std::process;

use tcg_backend::X86_64CodeGen;
use tcg_core::context::Context;
use tcg_core::tb::{EXCP_ECALL, EXCP_UNDEF};
use tcg_core::TempIdx;
use tcg_exec::exec_loop::{cpu_exec_loop, ExitReason};
use tcg_exec::{ExecEnv, GuestCpu};
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
    let mut lcpu = LinuxCpu {
        cpu: Aarch64Cpu::new(),
    };
    lcpu.cpu.pc = info.entry;
    lcpu.cpu.sp = info.sp;
    lcpu.cpu.guest_base = space.guest_base() as u64;

    // mmap_next starts after brk
    let mut mmap_next =
        tcg_linux_user::guest_space::page_align_up(
            info.brk,
        ) + 0x1000_0000;

    // Install SIGSEGV handler to dump guest state
    unsafe {
        unsafe extern "C" fn sigsegv_handler(
            _sig: i32,
            _info: *mut libc::siginfo_t,
            ctx: *mut libc::c_void,
        ) {
            let uc = ctx as *const libc::ucontext_t;
            let mctx = &(*uc).uc_mcontext;
            let rbp = mctx.gregs[libc::REG_RBP as usize]
                as *const u64;
            eprintln!("=== SIGSEGV in JIT code ===");
            eprintln!(
                "RIP={:#x} RBP={:#x}",
                mctx.gregs[libc::REG_RIP as usize],
                mctx.gregs[libc::REG_RBP as usize],
            );
            for i in 0..8usize {
                let v = *rbp.add(i);
                eprint!("x{}={:#018x} ", i, v);
            }
            eprintln!();
            for i in 19..21usize {
                let v = *rbp.add(i);
                eprint!("x{}={:#018x} ", i, v);
            }
            for i in 29..31usize {
                let v = *rbp.add(i);
                eprint!("x{}={:#018x} ", i, v);
            }
            eprintln!();
            let pc = *rbp.add(31); // PC at offset 248/8=31
            let sp = *rbp.add(32); // SP at offset 256/8=32
            let nzcv = *rbp.add(34); // NZCV at offset 272/8=34
            eprintln!(
                "pc={:#018x} sp={:#018x} nzcv={:#018x}",
                pc, sp, nzcv,
            );
            std::process::exit(139);
        }
        let mut sa: libc::sigaction =
            std::mem::zeroed();
        sa.sa_sigaction =
            sigsegv_handler as usize;
        sa.sa_flags =
            libc::SA_SIGINFO | libc::SA_NODEFER;
        libc::sigaction(
            libc::SIGSEGV, &sa, std::ptr::null_mut(),
        );
    }

    // Run
    let show_stats = env::var("TCG_STATS").is_ok();

    let show_trace = env::var("TCG_TRACE").is_ok();
    let mut codegen = X86_64CodeGen::new();
    codegen.guest_base_offset =
        tcg_frontend::aarch64::cpu::GUEST_BASE_OFFSET as i32;
    let env = ExecEnv::new(codegen);
    #[cfg(feature = "llvm")]
    if std::env::var("TCG_LLVM").is_ok() {
        eprintln!("[tcg] LLVM JIT backend enabled");
        env.enable_llvm();
    }
    let mut env = env;
    let mut icount: u64 = 0;
    loop {
        if show_trace {
            eprintln!(
                "[trace] pc={:#x} sp={:#x} i={}",
                lcpu.cpu.pc, lcpu.cpu.sp, icount,
            );
            icount += 1;
            // Debug: dump stack around sp+64..sp+79 when near crash
            if lcpu.cpu.pc == 0x4016c4 || lcpu.cpu.pc == 0x4008c8 || lcpu.cpu.pc == 0x400764 || lcpu.cpu.pc == 0x40078c {
                let sp = lcpu.cpu.sp;
                let gb = lcpu.cpu.guest_base;
                for off in [64u64, 72, 80] {
                    let addr = (gb + sp + off) as *const u64;
                    let val = unsafe { *addr };
                    eprintln!("  [sp+{}] = {:#018x}", off, val);
                }
            }
        }
        let reason =
            unsafe { cpu_exec_loop(&mut env, &mut lcpu) };
        match reason {
            ExitReason::Exit(v)
                if v == EXCP_ECALL as usize =>
            {
                // SVC (syscall)
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
                eprintln!("unexpected exit {v}");
                process::exit(1);
            }
            ExitReason::BufferFull => {
                if show_stats {
                    eprint!("{}", env.per_cpu.stats);
                }
                eprintln!("code buffer full");
                process::exit(1);
            }
        }
    }
}
