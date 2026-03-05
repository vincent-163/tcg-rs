use std::env;
use std::process;
use std::sync::atomic::Ordering;

use tcg_backend::X86_64CodeGen;
use tcg_core::context::Context;
use tcg_core::tb::{EXCP_EBREAK, EXCP_ECALL, EXCP_UNDEF};
use tcg_core::TempIdx;
use tcg_exec::exec_loop::{cpu_exec_loop, ExitReason};
use tcg_exec::{ExecEnv, GuestCpu};
use tcg_exec::profile::{ProfileData, ProfileEntry, DEFAULT_HOT_THRESHOLD};
use tcg_frontend::riscv::cpu::{RiscvCpu, NUM_GPRS};
use tcg_frontend::riscv::ext::RiscvCfg;
use tcg_frontend::riscv::{RiscvDisasContext, RiscvTranslator};
use tcg_frontend::{translator_loop, DisasJumpType, TranslatorOps};
use tcg_linux_user::elf::EM_RISCV;
use tcg_linux_user::guest_space::GuestSpace;
use tcg_linux_user::loader::{load_elf, ElfInfo};
use tcg_linux_user::syscall::{handle_syscall, SyscallResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProfileSaveMode {
    Hot,
    All,
}

impl ProfileSaveMode {
    fn from_env() -> Self {
        match env::var("TCG_PROFILE_MODE")
            .unwrap_or_else(|_| "hot".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "hot" => Self::Hot,
            "all" => Self::All,
            other => {
                eprintln!("[tcg] warning: unknown TCG_PROFILE_MODE={other:?}, using \"hot\"");
                Self::Hot
            }
        }
    }

    fn min_exec_count(self) -> u64 {
        match self {
            Self::Hot => DEFAULT_HOT_THRESHOLD,
            Self::All => 1,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::All => "all",
        }
    }
}

/// Wrapper: RiscvCpu + guest_base for GuestCpu trait.
struct LinuxCpu {
    cpu: RiscvCpu,
    cfg: RiscvCfg,
}

impl GuestCpu for LinuxCpu {
    fn get_pc(&self) -> u64 {
        self.cpu.pc
    }

    fn get_flags(&self) -> u32 {
        0
    }

    fn gen_code(&mut self, ir: &mut Context, pc: u64, max_insns: u32) -> u32 {
        let base = self.cpu.guest_base as *const u8;
        if ir.nb_globals() == 0 {
            let mut d = RiscvDisasContext::new(pc, base, self.cfg);
            d.base.max_insns = max_insns;
            translator_loop::<RiscvTranslator>(&mut d, ir);
            d.base.num_insns * 4
        } else {
            let mut d = RiscvDisasContext::new(pc, base, self.cfg);
            d.base.max_insns = max_insns;
            d.env = TempIdx(0);
            for i in 0..NUM_GPRS {
                d.gpr[i] = TempIdx(1 + i as u32);
            }
            d.pc = TempIdx(1 + NUM_GPRS as u32);
            d.load_res = TempIdx(1 + NUM_GPRS as u32 + 1);
            d.load_val = TempIdx(1 + NUM_GPRS as u32 + 2);
            RiscvTranslator::tb_start(&mut d, ir);
            loop {
                RiscvTranslator::insn_start(&mut d, ir);
                RiscvTranslator::translate_insn(&mut d, ir);
                if d.base.is_jmp != DisasJumpType::Next {
                    break;
                }
                if d.base.num_insns >= d.base.max_insns {
                    d.base.is_jmp = DisasJumpType::TooMany;
                    break;
                }
            }
            RiscvTranslator::tb_stop(&mut d, ir);
            d.base.num_insns * 4
        }
    }

    fn env_ptr(&mut self) -> *mut u8 {
        &mut self.cpu as *mut RiscvCpu as *mut u8
    }
}

fn save_profile<B: tcg_backend::HostCodeGen>(
    env: &ExecEnv<B>,
    load_vaddr: u64,
    mode: ProfileSaveMode,
) {
    let out = std::env::var("TCG_PROFILE_OUT").unwrap_or_else(|_| "profile.bin".into());
    let path = std::path::Path::new(&out);
    let shared = &env.shared;
    let min_exec_count = mode.min_exec_count();

    // Load existing profile entries (accumulate entry addresses only, not counts)
    let mut accumulated: std::collections::HashMap<u64, ProfileEntry> =
        ProfileData::load(path).map(|existing| {
            eprintln!("[tcg] accumulating with existing profile ({} entries)",
                existing.entries.len());
            existing.entries.into_iter()
                .map(|e| (e.file_offset, e))
                .collect()
        }).unwrap_or_default();

    for tb_ptr in shared.tb_store.iter_all() {
        let tb = unsafe { &*tb_ptr };
        let exec = tb.exec_count.load(Ordering::Relaxed);
        let file_offset = tb.pc - load_vaddr;
        let indirect = tb.indirect_target.load(Ordering::Relaxed);

        // Add to profile if this TB met the mode threshold in this run.
        // Don't sum counts across runs - each run must independently satisfy it.
        if exec >= min_exec_count {
            let entry = accumulated.entry(file_offset).or_insert(ProfileEntry {
                file_offset,
                exec_count: exec,  // Use this run's count, don't sum
                indirect_target: indirect,
            });
            // Keep the max exec count seen in any single run
            if exec > entry.exec_count {
                entry.exec_count = exec;
            }
            // Accumulate indirect_target flag (OR operation)
            if indirect {
                entry.indirect_target = true;
            }
        }
    }

    let entries: Vec<ProfileEntry> = accumulated.into_values().collect();

    let data = ProfileData { threshold: min_exec_count as u32, entries };
    if let Err(e) = data.save(path) {
        eprintln!("[tcg] failed to save profile: {e}");
    } else {
        eprintln!(
            "[tcg] profile saved to {out} (mode={}, min_exec_count={}, {} TBs)",
            mode.as_str(),
            min_exec_count,
            data.entries.len()
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: tcg-riscv64 <elf> [args...]");
        process::exit(1);
    }

    let elf_path =
        std::fs::canonicalize(&args[1]).expect("failed to resolve elf path");
    let elf_path = elf_path.to_str().unwrap();
    let guest_argv: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    // Load ELF
    let mut space = GuestSpace::new().expect("failed to create guest space");
    let info: ElfInfo = load_elf(
        std::path::Path::new(elf_path),
        &mut space,
        &guest_argv,
        &[],
        EM_RISCV,
    )
    .expect("failed to load ELF");

    // Set up CPU
    let mut lcpu = LinuxCpu {
        cpu: RiscvCpu::new(),
        cfg: RiscvCfg::default(),
    };
    lcpu.cpu.pc = info.entry;
    lcpu.cpu.gpr[2] = info.sp; // SP = x2
    lcpu.cpu.guest_base = space.guest_base() as u64;
    lcpu.cpu.load_bias = info.load_vaddr; // for AOT dispatch: file_offset = pc - load_bias

    // mmap_next starts after brk
    let mut mmap_next =
        tcg_linux_user::guest_space::page_align_up(info.brk) + 0x1000_0000; // 256 MB gap

    // Run
    let show_stats = env::var("TCG_STATS").is_ok();
    let profiling = env::var("TCG_PROFILE").is_ok();
    let profile_mode = ProfileSaveMode::from_env();

    // Load AOT if specified
    let aot = env::var("TCG_AOT").ok().and_then(|p| {
        let t = tcg_exec::AotTable::load(std::path::Path::new(&p), info.load_vaddr);
        if t.is_some() { eprintln!("[tcg] AOT loaded from {p}"); }
        else { eprintln!("[tcg] warning: failed to load AOT from {p}"); }
        t
    });

    let env = ExecEnv::new_with_opts(X86_64CodeGen::new(), profiling, aot);
    #[cfg(feature = "llvm")]
    if std::env::var("TCG_LLVM").is_ok() {
        eprintln!("[tcg] LLVM JIT backend enabled");
        env.enable_llvm();
    }
    if profiling {
        eprintln!(
            "[tcg] profiling enabled (mode={}, min_exec_count={})",
            profile_mode.as_str(),
            profile_mode.min_exec_count()
        );
    }
    let mut env = env;
    loop {
        let reason = unsafe { cpu_exec_loop(&mut env, &mut lcpu) };
        match reason {
            ExitReason::Exit(v) if v == EXCP_ECALL as usize => {
                // ECALL
                match handle_syscall(
                    &mut space,
                    &mut lcpu.cpu.gpr,
                    &mut mmap_next,
                    elf_path,
                ) {
                    SyscallResult::Continue(ret) => {
                        lcpu.cpu.gpr[10] = ret;
                        lcpu.cpu.pc += 4; // skip past ECALL
                    }
                    SyscallResult::Exit(code) => {
                        if show_stats { eprint!("{}", env.per_cpu.stats); }
                        if profiling { save_profile(&env, info.load_vaddr, profile_mode); }
                        process::exit(code);
                    }
                }
            }
            ExitReason::Exit(v) if v == EXCP_EBREAK as usize => {
                if show_stats { eprint!("{}", env.per_cpu.stats); }
                if profiling { save_profile(&env, info.load_vaddr, profile_mode); }
                eprintln!("ebreak at pc={:#x}", lcpu.cpu.pc);
                process::exit(1);
            }
            ExitReason::Exit(v) if v == EXCP_UNDEF as usize => {
                if show_stats { eprint!("{}", env.per_cpu.stats); }
                if profiling { save_profile(&env, info.load_vaddr, profile_mode); }
                eprintln!("illegal instruction at pc={:#x}", lcpu.cpu.pc);
                process::exit(1);
            }
            ExitReason::Exit(v) => {
                if show_stats { eprint!("{}", env.per_cpu.stats); }
                if profiling { save_profile(&env, info.load_vaddr, profile_mode); }
                eprintln!("unexpected exit {v}");
                process::exit(1);
            }
            ExitReason::BufferFull => {
                if show_stats { eprint!("{}", env.per_cpu.stats); }
                if profiling { save_profile(&env, info.load_vaddr, profile_mode); }
                eprintln!("code buffer full");
                process::exit(1);
            }
        }
    }
}
