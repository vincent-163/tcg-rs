//! TCG AOT compiler: translates guest TBs via LLVM, emits
//! object file.
//!
//! Two modes:
//!   Profile-guided: tcg-aot <profile.bin> <elf> [-o out.o]
//!   Static:         tcg-aot <elf> [-o out.o]
//!
//! Guest architecture is detected automatically from the ELF
//! e_machine field (EM_RISCV=0xf3, EM_AARCH64=0xb7).

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::ffi::CString;
use std::path::Path;
use std::process;
use std::ptr;

use tcg_backend::llvm::ffi::*;
use tcg_backend::llvm::translate::TbTranslator;
use tcg_backend::optimize::optimize;
use tcg_backend::HostCodeGen;
use tcg_core::context::Context;
use tcg_core::tb::TranslationBlock;
use tcg_core::temp::TempKind;
use tcg_core::{Opcode, TempIdx, OPCODE_DEFS};
use tcg_exec::profile::ProfileData;
use tcg_frontend::aarch64::cpu::{
    LOAD_BIAS_OFFSET as AA64_LB_OFFSET,
    NUM_XREGS,
    PC_OFFSET as AA64_PC_OFFSET,
};
use tcg_frontend::aarch64::{
    Aarch64DisasContext, Aarch64Translator,
};
use tcg_frontend::riscv::cpu::{
    LOAD_BIAS_OFFSET as RV64_LB_OFFSET,
    NUM_GPRS,
    PC_OFFSET as RV64_PC_OFFSET,
};
use tcg_frontend::riscv::ext::RiscvCfg;
use tcg_frontend::riscv::{
    RiscvDisasContext, RiscvTranslator,
};
use tcg_frontend::translator_loop;

const USAGE: &str = "\
usage: tcg-aot <elf> [-o output.o]            (static)
       tcg-aot <profile.bin> <elf> [-o output.o] (profile)

Static mode discovers all reachable TBs via recursive
descent from the ELF entry point and compiles them.

Profile mode reads hot TBs from a profiling run.

Guest architecture is auto-detected from the ELF e_machine
field (supports riscv64 and aarch64).

Options:
  -o <file>   Output object file (default: aot.o)";

// ELF e_machine constants (little-endian, offset 18)
const EM_AARCH64: u16 = 183;
const EM_RISCV: u16 = 243;

// ── Architecture ─────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arch {
    Riscv64,
    Aarch64,
}

/// Detect guest architecture from ELF e_machine field.
fn detect_arch(elf_data: &[u8]) -> Arch {
    if elf_data.len() < 20 {
        eprintln!("[aot] ELF too short to read e_machine");
        process::exit(1);
    }
    let e_machine = u16::from_le_bytes(
        elf_data[18..20].try_into().unwrap(),
    );
    match e_machine {
        EM_RISCV => Arch::Riscv64,
        EM_AARCH64 => Arch::Aarch64,
        other => {
            eprintln!(
                "[aot] unsupported e_machine {other:#x}; \
                 only riscv64 (0xf3) and aarch64 \
                 (0xb7) are supported"
            );
            process::exit(1);
        }
    }
}

impl Arch {
    fn pc_temp(&self) -> TempIdx {
        match self {
            Arch::Riscv64 => TempIdx(1 + NUM_GPRS as u32),
            Arch::Aarch64 => TempIdx(1 + NUM_XREGS as u32),
        }
    }

    fn pc_offset(&self) -> i64 {
        match self {
            Arch::Riscv64 => RV64_PC_OFFSET,
            Arch::Aarch64 => AA64_PC_OFFSET,
        }
    }

    fn load_bias_offset(&self) -> i64 {
        match self {
            Arch::Riscv64 => RV64_LB_OFFSET,
            Arch::Aarch64 => AA64_LB_OFFSET,
        }
    }

    fn init_context(&self, ir: &mut Context) {
        let backend = tcg_backend::X86_64CodeGen::new();
        backend.init_context(ir);
    }

    /// Translate one TB starting at `guest_pc`, return
    /// the next sequential PC (fall-through address).
    fn translate_tb(
        &self,
        ir: &mut Context,
        guest_pc: u64,
        base: *const u8,
        max_insns: u32,
    ) -> u64 {
        match self {
            Arch::Riscv64 => {
                let cfg = RiscvCfg::default();
                let mut d = RiscvDisasContext::new(
                    guest_pc, base, cfg,
                );
                d.base.max_insns = max_insns;
                setup_riscv_temps(&mut d, ir);
                translator_loop::<RiscvTranslator>(
                    &mut d, ir,
                );
                d.base.pc_next
            }
            Arch::Aarch64 => {
                let mut d = Aarch64DisasContext::new(
                    guest_pc, base,
                );
                d.base.max_insns = max_insns;
                setup_aarch64_temps(&mut d, ir);
                translator_loop::<Aarch64Translator>(
                    &mut d, ir,
                );
                d.base.pc_next
            }
        }
    }
}

// ── Mode detection ───────────────────────────────────────

enum Mode {
    Static {
        elf_path: String,
        output: String,
    },
    Profile {
        profile_path: String,
        elf_path: String,
        output: String,
    },
}

fn parse_mode(args: &[String]) -> Mode {
    if args.len() < 2 {
        eprintln!("{USAGE}");
        process::exit(1);
    }

    // Collect -o flag
    let output = args
        .iter()
        .position(|a| a == "-o")
        .map(|i| {
            args.get(i + 1)
                .unwrap_or_else(|| {
                    eprintln!("-o requires an argument");
                    process::exit(1);
                })
                .clone()
        })
        .unwrap_or_else(|| "aot.o".into());

    // Collect positional args (skip -o and its value)
    let mut positional = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-o" {
            i += 2;
            continue;
        }
        if args[i] == "-h" || args[i] == "--help" {
            eprintln!("{USAGE}");
            process::exit(0);
        }
        positional.push(args[i].clone());
        i += 1;
    }

    match positional.len() {
        1 => Mode::Static {
            elf_path: positional[0].clone(),
            output,
        },
        2 => Mode::Profile {
            profile_path: positional[0].clone(),
            elf_path: positional[1].clone(),
            output,
        },
        _ => {
            eprintln!("{USAGE}");
            process::exit(1);
        }
    }
}

// ── Main ─────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = parse_mode(&args);

    match mode {
        Mode::Static { elf_path, output } => {
            let elf_data = std::fs::read(&elf_path)
                .expect("failed to read ELF");
            let arch = detect_arch(&elf_data);
            eprintln!("[aot] detected arch: {arch:?}");
            run_static(arch, &elf_path, &output);
        }
        Mode::Profile {
            profile_path,
            elf_path,
            output,
        } => {
            run_profile(
                &profile_path,
                &elf_path,
                &output,
            );
        }
    }
}
// ── Static mode ──────────────────────────────────────────

fn run_static(arch: Arch, elf_path: &str, output: &str) {
    let elf_data =
        std::fs::read(elf_path).expect("failed to read ELF");
    let (load_vaddr, load_file_offset) =
        parse_elf_load(&elf_data);
    let entry = parse_elf_entry(&elf_data);

    eprintln!(
        "[aot] static mode: entry={entry:#x} \
         load_vaddr={load_vaddr:#x} \
         file_offset={load_file_offset:#x}"
    );

    let offsets = discover_tbs_static(
        arch,
        &elf_data,
        load_vaddr,
        load_file_offset,
        entry,
    );
    eprintln!(
        "[aot] discovered {} TBs via static analysis",
        offsets.len()
    );

    // In static mode all TBs are exported (no frequency
    // data to distinguish hot/cold).
    let all_entries: Vec<(u64, bool)> =
        offsets.iter().map(|&o| (o, true)).collect();

    compile_aot(
        arch,
        &elf_data,
        load_vaddr,
        load_file_offset,
        &all_entries,
        output,
    );
}

/// Discover all TBs in executable sections using linear
/// sweep (like objdump -d) combined with recursive descent
/// from branch targets.  Linear sweep ensures we find code
/// reachable only via indirect jumps or jump tables.
fn discover_tbs_static(
    arch: Arch,
    elf_data: &[u8],
    load_vaddr: u64,
    load_file_offset: usize,
    entry: u64,
) -> Vec<u64> {
    let base = unsafe {
        elf_data.as_ptr().offset(
            load_file_offset as isize
                - load_vaddr as isize,
        )
    };
    let pc_temp = arch.pc_temp();
    let exec_segs = find_exec_segments(elf_data);

    let mut visited: HashSet<u64> = HashSet::new();
    let mut worklist: VecDeque<u64> = VecDeque::new();
    let mut result: Vec<u64> = Vec::new();

    // Seed: entry point
    worklist.push_back(entry);

    // Seed: linear sweep — start of every executable
    // segment produces TBs at natural boundaries
    for &(seg_lo, _seg_hi) in &exec_segs {
        worklist.push_back(seg_lo);
    }

    while let Some(guest_pc) = worklist.pop_front() {
        // Bounds check: must be in an exec segment
        let in_exec = exec_segs.iter().any(|&(lo, hi)| {
            guest_pc >= lo && guest_pc < hi
        });
        if !in_exec {
            continue;
        }
        let file_offset = guest_pc - load_vaddr;
        if !visited.insert(file_offset) {
            continue;
        }

        result.push(file_offset);

        // Translate TB to collect branch targets
        let mut ir = Context::new();
        arch.init_context(&mut ir);
        ir.tb_ptr = 0;
        let max_insns = TranslationBlock::max_insns(0);
        let pc_next =
            arch.translate_tb(&mut ir, guest_pc, base, max_insns);

        // Collect goto_tb targets (direct branches)
        let mut targets: HashSet<u64> = HashSet::new();
        collect_goto_targets(&ir, pc_temp, &mut targets);
        for va in targets {
            worklist.push_back(va);
        }

        // Fall-through: next PC after this TB
        worklist.push_back(pc_next);
    }

    result
}

/// Find all executable PT_LOAD segments and return their
/// (vaddr_lo, vaddr_hi) ranges.
fn find_exec_segments(data: &[u8]) -> Vec<(u64, u64)> {
    let e_phoff = u64::from_le_bytes(
        data[32..40].try_into().unwrap(),
    ) as usize;
    let e_phentsize = u16::from_le_bytes(
        data[54..56].try_into().unwrap(),
    ) as usize;
    let e_phnum = u16::from_le_bytes(
        data[56..58].try_into().unwrap(),
    ) as usize;

    let mut segs = Vec::new();
    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        let p_type = u32::from_le_bytes(
            data[off..off + 4].try_into().unwrap(),
        );
        let p_flags = u32::from_le_bytes(
            data[off + 4..off + 8].try_into().unwrap(),
        );
        if p_type == 1 && (p_flags & 1) != 0 {
            let p_vaddr = u64::from_le_bytes(
                data[off + 16..off + 24]
                    .try_into()
                    .unwrap(),
            );
            let p_memsz = u64::from_le_bytes(
                data[off + 32..off + 40]
                    .try_into()
                    .unwrap(),
            );
            segs.push((p_vaddr, p_vaddr + p_memsz));
        }
    }
    if segs.is_empty() {
        eprintln!(
            "[aot] no executable PT_LOAD segment found"
        );
        process::exit(1);
    }
    segs
}

/// Extract the ELF entry point.
fn parse_elf_entry(data: &[u8]) -> u64 {
    u64::from_le_bytes(
        data[24..32].try_into().unwrap(),
    )
}

// ── Profile mode ─────────────────────────────────────────

fn run_profile(
    profile_path: &str,
    elf_path: &str,
    output: &str,
) {
    let profile =
        ProfileData::load(Path::new(profile_path))
            .expect("failed to load profile");
    let elf_data =
        std::fs::read(elf_path).expect("failed to read ELF");
    let arch = detect_arch(&elf_data);
    let (load_vaddr, load_file_offset) =
        parse_elf_load(&elf_data);
    let min_exec_count = u64::from(profile.threshold.max(1));

    eprintln!(
        "[aot] profile mode ({arch:?}): {} entries, \
         min_exec_count={min_exec_count}, \
         load vaddr={load_vaddr:#x} \
         file_offset={load_file_offset:#x}",
        profile.entries.len()
    );

    let selected_entries: Vec<_> = profile
        .entries
        .iter()
        .copied()
        .filter(|e| e.exec_count > min_exec_count)
        .collect();
    eprintln!(
        "[aot] keeping {} entries with exec_count > {} \
         (dropped {})",
        selected_entries.len(),
        min_exec_count,
        profile.entries.len() - selected_entries.len()
    );
    if selected_entries.is_empty() {
        eprintln!(
            "[aot] no profile entries satisfy exec_count > {}",
            min_exec_count
        );
        process::exit(1);
    }

    // Determine which TBs to export vs keep internal
    let export_set: HashSet<u64> = profile
        .entries
        .iter()
        .filter(|e| e.exec_count > min_exec_count)
        .filter(|e| ProfileData::should_export(e))
        .map(|e| e.file_offset)
        .collect();

    eprintln!(
        "[aot] {} exported, {} internal",
        export_set.len(),
        selected_entries.len() - export_set.len()
    );

    let all_entries: Vec<(u64, bool)> = selected_entries
        .iter()
        .map(|e| {
            (
                e.file_offset,
                export_set.contains(&e.file_offset),
            )
        })
        .collect();

    compile_aot(
        arch,
        &elf_data,
        load_vaddr,
        load_file_offset,
        &all_entries,
        output,
    );
}

// ── Shared LLVM compilation ──────────────────────────────

/// Compile all TBs in `all_entries` via LLVM and emit an
/// object file.  Each entry is (file_offset, exported).
fn compile_aot(
    arch: Arch,
    elf_data: &[u8],
    load_vaddr: u64,
    load_file_offset: usize,
    all_entries: &[(u64, bool)],
    output: &str,
) {
    // Initialize LLVM
    unsafe {
        LLVMInitializeX86Target();
        LLVMInitializeX86TargetInfo();
        LLVMInitializeX86TargetMC();
        LLVMInitializeX86AsmPrinter();
        LLVMInitializeX86AsmParser();
    }

    let llvm_ctx = unsafe { LLVMContextCreate() };
    let module = unsafe {
        LLVMModuleCreateWithNameInContext(
            c"tcg_aot".as_ptr(),
            llvm_ctx,
        )
    };

    let (tm, triple_str) = create_target_machine();
    unsafe {
        let triple_c =
            CString::new(triple_str.as_str()).unwrap();
        LLVMSetTarget(module, triple_c.as_ptr());
        let td = LLVMCreateTargetDataLayout(tm);
        let dl = LLVMCopyStringRepOfTargetData(td);
        LLVMSetDataLayout(module, dl);
        LLVMDisposeMessage(dl);
        LLVMDisposeTargetData(td);

        let i32t = LLVMInt32TypeInContext(llvm_ctx);
        let pic_val =
            LLVMValueAsMetadata(LLVMConstInt(i32t, 2, 0));
        LLVMAddModuleFlag(
            module,
            7,
            c"PIC Level".as_ptr(),
            9,
            pic_val,
        );
    }

    let pc_temp = arch.pc_temp();

    let base = unsafe {
        elf_data.as_ptr().offset(
            load_file_offset as isize
                - load_vaddr as isize,
        )
    };

    // Pre-scan: find TBs that contain helper calls (Opcode::Call).
    // Helper function addresses are absolute host pointers from
    // the AOT compilation process; they are not relocatable and
    // would SIGSEGV at runtime under ASLR.  We must exclude these
    // from the peer-lookup map so that other TBs that would
    // goto_tb into them fall back to aot_dispatch instead of
    // emitting an unresolved musttail-call reference.
    let mut skipped_offsets: HashSet<u64> = HashSet::new();
    for &(offset, _) in all_entries {
        let mut ir = Context::new();
        arch.init_context(&mut ir);
        ir.tb_ptr = 0;
        let guest_pc = offset + load_vaddr;
        let max_insns = TranslationBlock::max_insns(0);
        arch.translate_tb(&mut ir, guest_pc, base, max_insns);
        optimize(&mut ir);
        if ir.ops().iter().any(|op| op.opc == Opcode::Call) {
            skipped_offsets.insert(offset);
        }
    }

    // Build guest VA → file_offset map for peer lookup,
    // excluding any TBs that will be skipped.
    let all_va_to_offset: HashMap<u64, u64> = all_entries
        .iter()
        .filter(|&&(off, _)| !skipped_offsets.contains(&off))
        .map(|&(off, _)| (off + load_vaddr, off))
        .collect();

    // Translate all TBs with LLVM
    let mut translated = 0u32;
    let skipped_call = skipped_offsets.len() as u32;

    for &(offset, exported) in all_entries {
        let func_name = format!("tb_{offset:x}");

        // Skip TBs with non-relocatable helper calls.
        if skipped_offsets.contains(&offset) {
            continue;
        }

        let mut ir = Context::new();
        arch.init_context(&mut ir);
        ir.tb_ptr = 0;

        let guest_pc = offset + load_vaddr;
        let max_insns = TranslationBlock::max_insns(0);
        arch.translate_tb(&mut ir, guest_pc, base, max_insns);

        optimize(&mut ir);

        let tb_translator =
            TbTranslator::new_with_peers(
                llvm_ctx,
                &ir,
                &func_name,
                &all_va_to_offset,
                pc_temp,
            );
        let tb_module = tb_translator.translate(&ir);

        unsafe {
            let triple_c =
                CString::new(triple_str.as_str())
                    .unwrap();
            LLVMSetTarget(tb_module, triple_c.as_ptr());
            let td = LLVMCreateTargetDataLayout(tm);
            let dl = LLVMCopyStringRepOfTargetData(td);
            LLVMSetDataLayout(tb_module, dl);
            LLVMDisposeMessage(dl);
            LLVMDisposeTargetData(td);
        }

        unsafe {
            let cfunc_name =
                CString::new(func_name.as_str()).unwrap();
            let func = LLVMGetNamedFunction(
                tb_module,
                cfunc_name.as_ptr(),
            );
            if !func.is_null() {
                LLVMSetLinkage(func, 0);
                if exported {
                    LLVMSetVisibility(func, 0);
                }
            }
            let err =
                LLVMLinkModules2(module, tb_module);
            if err != 0 {
                eprintln!(
                    "[aot] warning: failed to link \
                     {func_name}"
                );
                continue;
            }
        }

        translated += 1;
    }

    eprintln!(
        "[aot] translated {translated} TBs \
         (skipped {skipped_call} with helper calls)",
    );

    // Post-link: hide non-exported TBs
    unsafe {
        for &(offset, exported) in all_entries {
            if !exported {
                let name = CString::new(
                    format!("tb_{offset:x}"),
                )
                .unwrap();
                let func = LLVMGetNamedFunction(
                    module,
                    name.as_ptr(),
                );
                if !func.is_null() {
                    LLVMSetVisibility(func, 1);
                }
            }
        }
    }

    // Emit tb_index (exported TBs only)
    let exported_offsets: Vec<u64> = all_entries
        .iter()
        .filter(|&&(_, exp)| exp)
        .map(|&(offset, _)| offset)
        .collect();
    emit_tb_index_pcs(module, llvm_ctx, &exported_offsets);

    // Emit aot_dispatch over all TBs
    let all_offset_list: Vec<u64> =
        all_entries.iter().map(|&(o, _)| o).collect();
    emit_aot_dispatch(
        arch,
        module,
        llvm_ctx,
        &all_offset_list,
    );

    // Verify module
    unsafe {
        let mut err_msg: *mut i8 = ptr::null_mut();
        let rc =
            LLVMVerifyModule(module, 2, &mut err_msg);
        if rc != 0 && !err_msg.is_null() {
            let s = std::ffi::CStr::from_ptr(err_msg)
                .to_string_lossy();
            eprintln!("[aot] verify warning: {s}");
            LLVMDisposeMessage(err_msg);
        }
    }

    // Dump pre-optimization IR
    let ll_path = format!("{output}.ll");
    unsafe {
        let c_path =
            CString::new(ll_path.as_str()).unwrap();
        let mut err: *mut i8 = ptr::null_mut();
        LLVMPrintModuleToFile(
            module,
            c_path.as_ptr(),
            &mut err,
        );
        if !err.is_null() {
            LLVMDisposeMessage(err);
        }
    }
    eprintln!("[aot] IR dumped to {ll_path}");

    // Run O3 optimization
    eprintln!("[aot] running O3 optimization...");
    unsafe {
        let opts = LLVMCreatePassBuilderOptions();
        let passes = c"default<O3>";
        let err =
            LLVMRunPasses(module, passes.as_ptr(), tm, opts);
        if !err.is_null() {
            let msg = LLVMGetErrorMessage(err);
            let s = std::ffi::CStr::from_ptr(msg)
                .to_string_lossy();
            eprintln!("[aot] pass error: {s}");
            LLVMDisposeErrorMessage(msg);
        }
        LLVMDisposePassBuilderOptions(opts);
    }

    // Emit object file
    eprintln!("[aot] emitting {output}...");
    unsafe {
        let out_path =
            CString::new(output).unwrap().into_raw();
        let mut err: *mut i8 = ptr::null_mut();
        let rc = LLVMTargetMachineEmitToFile(
            tm, module, out_path, 1, &mut err,
        );
        let _ = CString::from_raw(out_path);
        if rc != 0 {
            if !err.is_null() {
                let s = std::ffi::CStr::from_ptr(err)
                    .to_string_lossy();
                eprintln!("[aot] emit error: {s}");
                LLVMDisposeMessage(err);
            }
            process::exit(1);
        }
    }

    unsafe {
        LLVMDisposeTargetMachine(tm);
        LLVMDisposeModule(module);
        LLVMContextDispose(llvm_ctx);
    }

    eprintln!("[aot] done: {output}");
    eprintln!(
        "[aot] link with: cc -shared -o aot.so {output}"
    );
}

// ── Helpers ──────────────────────────────────────────────

fn parse_elf_load(data: &[u8]) -> (u64, usize) {
    let e_phoff = u64::from_le_bytes(
        data[32..40].try_into().unwrap(),
    ) as usize;
    let e_phentsize = u16::from_le_bytes(
        data[54..56].try_into().unwrap(),
    ) as usize;
    let e_phnum = u16::from_le_bytes(
        data[56..58].try_into().unwrap(),
    ) as usize;

    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        let p_type = u32::from_le_bytes(
            data[off..off + 4].try_into().unwrap(),
        );
        let p_flags = u32::from_le_bytes(
            data[off + 4..off + 8].try_into().unwrap(),
        );
        if p_type == 1 && (p_flags & 1) != 0 {
            let p_offset = u64::from_le_bytes(
                data[off + 8..off + 16]
                    .try_into()
                    .unwrap(),
            );
            let p_vaddr = u64::from_le_bytes(
                data[off + 16..off + 24]
                    .try_into()
                    .unwrap(),
            );
            return (p_vaddr, p_offset as usize);
        }
    }
    eprintln!(
        "[aot] no executable PT_LOAD segment found"
    );
    process::exit(1);
}

fn create_target_machine()
    -> (LLVMTargetMachineRef, String)
{
    unsafe {
        let triple = LLVMGetDefaultTargetTriple();
        let triple_str =
            std::ffi::CStr::from_ptr(triple)
                .to_string_lossy()
                .into_owned();
        let mut target: LLVMTargetRef = ptr::null_mut();
        let mut err: *mut i8 = ptr::null_mut();
        if LLVMGetTargetFromTriple(
            triple,
            &mut target,
            &mut err,
        ) != 0
        {
            if !err.is_null() {
                let s = std::ffi::CStr::from_ptr(err)
                    .to_string_lossy();
                eprintln!("[aot] target error: {s}");
                LLVMDisposeMessage(err);
            }
            process::exit(1);
        }
        let host_cpu_ptr = LLVMGetHostCPUName();
        let host_features_ptr = LLVMGetHostCPUFeatures();
        let host_cpu = if host_cpu_ptr.is_null() {
            "generic".to_string()
        } else {
            std::ffi::CStr::from_ptr(host_cpu_ptr)
                .to_string_lossy()
                .into_owned()
        };
        let host_features = if host_features_ptr.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(host_features_ptr)
                .to_string_lossy()
                .into_owned()
        };
        if !host_cpu_ptr.is_null() {
            LLVMDisposeMessage(host_cpu_ptr);
        }
        if !host_features_ptr.is_null() {
            LLVMDisposeMessage(host_features_ptr);
        }

        let cpu_str = env::var("TCG_AOT_CPU")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or(host_cpu);
        let features_str = env::var("TCG_AOT_FEATURES")
            .ok()
            .unwrap_or(host_features);
        let cpu = CString::new(cpu_str.as_str())
            .expect("target cpu contains interior NUL");
        let features = CString::new(features_str.as_str())
            .expect("target features contains interior NUL");
        eprintln!(
            "[aot] target machine: triple={triple_str}, cpu={}, features={}",
            cpu_str,
            if features_str.is_empty() {
                "<none>"
            } else {
                features_str.as_str()
            }
        );
        let tm = LLVMCreateTargetMachine(
            target,
            triple,
            cpu.as_ptr(),
            features.as_ptr(),
            2,
            2,
            3,
        );
        LLVMDisposeMessage(triple);
        (tm, triple_str)
    }
}

fn setup_riscv_temps(
    d: &mut RiscvDisasContext,
    _ir: &Context,
) {
    d.env = TempIdx(0);
    for i in 0..NUM_GPRS {
        d.gpr[i] = TempIdx(1 + i as u32);
    }
    d.pc = TempIdx(1 + NUM_GPRS as u32);
    d.load_res = TempIdx(1 + NUM_GPRS as u32 + 1);
    d.load_val = TempIdx(1 + NUM_GPRS as u32 + 2);
}

fn setup_aarch64_temps(
    d: &mut Aarch64DisasContext,
    _ir: &Context,
) {
    d.env = TempIdx(0);
    for i in 0..NUM_XREGS {
        d.xregs[i] = TempIdx(1 + i as u32);
    }
    d.pc = TempIdx(1 + NUM_XREGS as u32);
    d.sp = TempIdx(1 + NUM_XREGS as u32 + 1);
    d.nzcv = TempIdx(1 + NUM_XREGS as u32 + 2);
}

/// Scan TCG IR for goto_tb targets: find
/// Mov(pc_temp, const) before GotoTb.
fn collect_goto_targets(
    ir: &Context,
    pc_temp: TempIdx,
    targets: &mut HashSet<u64>,
) {
    let ops = ir.ops();
    let mut last_pc_const: Option<u64> = None;
    for op in ops {
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        match op.opc {
            Opcode::Mov => {
                let dst = op.args[0];
                let src = op.args[nb_o];
                if dst == pc_temp {
                    let temp = ir.temp(src);
                    if temp.kind == TempKind::Const {
                        last_pc_const = Some(temp.val);
                    } else {
                        last_pc_const = None;
                    }
                }
            }
            Opcode::GotoTb => {
                if let Some(pc) = last_pc_const {
                    targets.insert(pc);
                }
            }
            _ => {}
        }
    }
}

fn emit_tb_index_pcs(
    module: LLVMModuleRef,
    ctx: LLVMContextRef,
    pcs: &[u64],
) {
    unsafe {
        let i64t = LLVMInt64TypeInContext(ctx);
        let mut vals: Vec<LLVMValueRef> = pcs
            .iter()
            .map(|&pc| LLVMConstInt(i64t, pc, 0))
            .collect();
        // Sentinel: u64::MAX (avoids conflict with
        // TB at file offset 0)
        vals.push(LLVMConstInt(i64t, u64::MAX, 0));

        let arr_ty =
            LLVMArrayType2(i64t, vals.len() as u64);
        let arr_val = LLVMConstArray2(
            i64t,
            vals.as_ptr(),
            vals.len() as u64,
        );

        let name = c"tb_index";
        let global =
            LLVMAddGlobal(module, arr_ty, name.as_ptr());
        LLVMSetInitializer(global, arr_val);
        LLVMSetGlobalConstant(global, 1);
        LLVMSetLinkage(global, 0);
    }
}

fn emit_aot_dispatch(
    arch: Arch,
    module: LLVMModuleRef,
    ctx: LLVMContextRef,
    all_offsets: &[u64],
) {
    const E: *const i8 = c"".as_ptr();
    unsafe {
        let i8t = LLVMInt8TypeInContext(ctx);
        let i64t = LLVMInt64TypeInContext(ctx);
        let ptr = LLVMPointerTypeInContext(ctx, 0);

        let mut params = [ptr, i64t];
        let fty = LLVMFunctionType(
            i64t,
            params.as_mut_ptr(),
            2,
            0,
        );

        let mut func = LLVMGetNamedFunction(
            module,
            c"aot_dispatch".as_ptr(),
        );
        if func.is_null() {
            func = LLVMAddFunction(
                module,
                c"aot_dispatch".as_ptr(),
                fty,
            );
        }
        LLVMSetLinkage(func, 0);
        LLVMSetVisibility(func, 1);

        let builder = LLVMCreateBuilderInContext(ctx);
        let entry = LLVMAppendBasicBlockInContext(
            ctx,
            func,
            c"entry".as_ptr(),
        );
        LLVMPositionBuilderAtEnd(builder, entry);

        let env = LLVMGetParam(func, 0);
        let guest_base = LLVMGetParam(func, 1);

        let pc_off = LLVMConstInt(
            i64t,
            arch.pc_offset() as u64,
            0,
        );
        let pc_ptr = LLVMBuildGEP2(
            builder,
            i8t,
            env,
            [pc_off].as_ptr(),
            1,
            E,
        );
        let pc_val = LLVMBuildLoad2(
            builder,
            i64t,
            pc_ptr,
            c"pc".as_ptr(),
        );

        let lb_off = LLVMConstInt(
            i64t,
            arch.load_bias_offset() as u64,
            0,
        );
        let lb_ptr = LLVMBuildGEP2(
            builder,
            i8t,
            env,
            [lb_off].as_ptr(),
            1,
            E,
        );
        let load_bias_val = LLVMBuildLoad2(
            builder,
            i64t,
            lb_ptr,
            c"load_bias".as_ptr(),
        );

        let file_offset_val = LLVMBuildSub(
            builder,
            pc_val,
            load_bias_val,
            c"file_offset".as_ptr(),
        );

        let miss_bb = LLVMAppendBasicBlockInContext(
            ctx,
            func,
            c"miss".as_ptr(),
        );
        LLVMPositionBuilderAtEnd(builder, miss_bb);
        let nochain = LLVMConstInt(
            i64t,
            tcg_core::tb::TB_EXIT_NOCHAIN,
            0,
        );
        LLVMBuildRet(builder, nochain);

        LLVMPositionBuilderAtEnd(builder, entry);
        let sw = LLVMBuildSwitch(
            builder,
            file_offset_val,
            miss_bb,
            all_offsets.len() as u32,
        );

        for &offset in all_offsets {
            let name = CString::new(
                format!("tb_{offset:x}"),
            )
            .unwrap();
            let tb_func = LLVMGetNamedFunction(
                module,
                name.as_ptr(),
            );
            if tb_func.is_null() {
                continue;
            }

            let bb_name = CString::new(
                format!("offset_{offset:x}"),
            )
            .unwrap();
            let bb = LLVMAppendBasicBlockInContext(
                ctx,
                func,
                bb_name.as_ptr(),
            );
            LLVMAddCase(
                sw,
                LLVMConstInt(i64t, offset, 0),
                bb,
            );

            LLVMPositionBuilderAtEnd(builder, bb);
            let mut args = [env, guest_base];
            let call = LLVMBuildCall2(
                builder,
                fty,
                tb_func,
                args.as_mut_ptr(),
                2,
                E,
            );
            LLVMSetTailCallKind(call, 2);
            LLVMBuildRet(builder, call);
        }

        LLVMDisposeBuilder(builder);
    }
    eprintln!(
        "[aot] emitted aot_dispatch with {} cases",
        all_offsets.len()
    );
}
