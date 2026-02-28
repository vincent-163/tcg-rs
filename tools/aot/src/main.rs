//! TCG AOT compiler: reads a profile, translates hot TBs via LLVM, emits object file.

use std::collections::HashSet;
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
use tcg_core::{Opcode, TempIdx, OPCODE_DEFS};
use tcg_core::temp::TempKind;
use tcg_exec::profile::ProfileData;
use tcg_frontend::riscv::cpu::{NUM_GPRS, PC_OFFSET};
use tcg_frontend::riscv::ext::RiscvCfg;
use tcg_frontend::riscv::{RiscvDisasContext, RiscvTranslator};
use tcg_frontend::translator_loop;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: tcg-aot <profile.bin> <elf> [-o output.o]");
        process::exit(1);
    }

    let profile_path = &args[1];
    let elf_path = &args[2];
    let output = args.iter().position(|a| a == "-o")
        .map(|i| args[i + 1].as_str())
        .unwrap_or("aot.o");

    let profile = ProfileData::load(Path::new(profile_path))
        .expect("failed to load profile");
    let elf_data = std::fs::read(elf_path).expect("failed to read ELF");

    // Parse ELF to find load offset (vaddr → file offset mapping for guest code access)
    let (load_vaddr, load_file_offset) = parse_elf_load(&elf_data);

    eprintln!("[aot] {} hot entries, ELF load vaddr={load_vaddr:#x} file_offset={load_file_offset:#x}",
        profile.entries.len());

    // Determine which TBs to export vs keep internal
    let export_set: HashSet<u64> = profile.entries.iter()
        .filter(|e| ProfileData::should_export(e))
        .map(|e| e.file_offset)
        .collect();

    eprintln!("[aot] {} exported, {} internal",
        export_set.len(), profile.entries.len() - export_set.len());

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
        LLVMModuleCreateWithNameInContext(c"tcg_aot".as_ptr(), llvm_ctx)
    };

    // Set up target
    let (tm, triple_str) = create_target_machine();
    unsafe {
        let triple_c = CString::new(triple_str.as_str()).unwrap();
        LLVMSetTarget(module, triple_c.as_ptr());
        let td = LLVMCreateTargetDataLayout(tm);
        let dl = LLVMCopyStringRepOfTargetData(td);
        LLVMSetDataLayout(module, dl);
        LLVMDisposeMessage(dl);
        LLVMDisposeTargetData(td);

        // Set PIC Level=2 so LLVM generates PIC-compatible jump tables
        let i32t = LLVMInt32TypeInContext(llvm_ctx);
        let pic_val = LLVMValueAsMetadata(LLVMConstInt(i32t, 2, 0));
        LLVMAddModuleFlag(module, 7, // Max behavior
            c"PIC Level".as_ptr(), 9, pic_val);
    }

    // Collect all AOT'd file offsets for musttail direct linking.
    // Profile entries now contain true file offsets (not guest VAs).
    let mut all_offsets: HashSet<u64> = profile.entries.iter()
        .map(|e| e.file_offset)
        .collect();
    let pc_temp = TempIdx(1 + NUM_GPRS as u32);

    // Pass 1: translate hot TBs to TCG IR and collect goto_tb target PCs.
    // The disassembler needs guest VAs, so we add load_vaddr to file offsets.
    // goto_tb targets in IR are guest VAs, which we convert back to file offsets.
    let cfg = RiscvCfg::default();
    let base = unsafe { elf_data.as_ptr().offset(load_file_offset as isize - load_vaddr as isize) };
    let mut extra_offsets: HashSet<u64> = HashSet::new();

    for entry in &profile.entries {
        let guest_pc = entry.file_offset + load_vaddr;
        let mut ir = Context::new();
        init_riscv_context(&mut ir);
        ir.tb_idx = 0;
        let mut d = RiscvDisasContext::new(guest_pc, base, cfg);
        d.base.max_insns = TranslationBlock::max_insns(0);
        setup_disas_temps(&mut d, &ir);
        translator_loop::<RiscvTranslator>(&mut d, &mut ir);
        optimize(&mut ir);
        // Collect goto_tb targets (guest VAs) and convert to file offsets
        let mut va_targets: HashSet<u64> = HashSet::new();
        collect_goto_targets(&ir, pc_temp, &mut va_targets);
        for va in va_targets {
            extra_offsets.insert(va - load_vaddr);
        }
    }

    // Add reachable targets that aren't already in the hot set
    let new_targets: Vec<u64> = extra_offsets.difference(&all_offsets).copied().collect();
    eprintln!("[aot] {} goto_tb targets outside hot set, adding to AOT", new_targets.len());
    all_offsets.extend(&extra_offsets);

    // Build extra entries for newly discovered targets
    let mut all_entries: Vec<(u64, bool)> = profile.entries.iter()
        .map(|e| (e.file_offset, export_set.contains(&e.file_offset)))
        .collect();
    for &offset in &new_targets {
        all_entries.push((offset, false)); // extra targets are internal
    }

    // Pass 2: translate all TBs with full peer set
    let mut translated = 0u32;

    for &(offset, exported) in &all_entries {
        let func_name = format!("tb_{offset:x}");

        // Translate guest → TCG IR
        let mut ir = Context::new();
        init_riscv_context(&mut ir);
        ir.tb_idx = 0;

        let guest_pc = offset + load_vaddr;
        let mut d = RiscvDisasContext::new(guest_pc, base, cfg);
        d.base.max_insns = TranslationBlock::max_insns(0);
        setup_disas_temps(&mut d, &ir);
        translator_loop::<RiscvTranslator>(&mut d, &mut ir);

        optimize(&mut ir);

        // Translate TCG IR → LLVM IR (into a fresh per-TB module, then link)
        let tb_translator = TbTranslator::new_with_peers(llvm_ctx, &ir, &func_name, &all_offsets, pc_temp);
        let tb_module = tb_translator.translate(&ir);

        // Copy data layout/target to per-TB module to avoid link warnings
        unsafe {
            let triple_c = CString::new(triple_str.as_str()).unwrap();
            LLVMSetTarget(tb_module, triple_c.as_ptr());
            let td = LLVMCreateTargetDataLayout(tm);
            let dl = LLVMCopyStringRepOfTargetData(td);
            LLVMSetDataLayout(tb_module, dl);
            LLVMDisposeMessage(dl);
            LLVMDisposeTargetData(td);
        }

        // Set linkage before linking into main module
        // All AOT TBs need ExternalLinkage so LLVMLinkModules2 can resolve
        // peer declarations. We'll set non-exported ones to internal AFTER
        // all modules are linked.
        unsafe {
            let cfunc_name = CString::new(func_name.as_str()).unwrap();
            let func = LLVMGetNamedFunction(tb_module, cfunc_name.as_ptr());
            if !func.is_null() {
                LLVMSetLinkage(func, 0); // ExternalLinkage
                if exported {
                    LLVMSetVisibility(func, 0); // Default
                }
            }
            let err = LLVMLinkModules2(module, tb_module);
            if err != 0 {
                eprintln!("[aot] warning: failed to link {func_name}");
                continue;
            }
        }

        translated += 1;
    }

    eprintln!("[aot] translated {translated} TBs");

    // Post-link: set non-exported TBs to hidden visibility (callable within
    // the .so via musttail but not visible to dlsym)
    unsafe {
        for &(offset, exported) in &all_entries {
            if !exported {
                let name = CString::new(format!("tb_{offset:x}")).unwrap();
                let func = LLVMGetNamedFunction(module, name.as_ptr());
                if !func.is_null() {
                    LLVMSetVisibility(func, 1); // HiddenVisibility
                }
            }
        }
    }

    // Emit tb_index: only exported TBs (ones the exec loop can call via dlsym)
    let exported_offsets: Vec<u64> = all_entries.iter()
        .filter(|&&(_, exp)| exp)
        .map(|&(offset, _)| offset)
        .collect();
    emit_tb_index_pcs(module, llvm_ctx, &exported_offsets);

    // Emit aot_dispatch super-function: switch(PC - load_vaddr) over all AOT'd TBs
    let all_offset_list: Vec<u64> = all_entries.iter().map(|&(offset, _)| offset).collect();
    emit_aot_dispatch(module, llvm_ctx, &all_offset_list, load_vaddr);

    // Verify module
    unsafe {
        let mut err_msg: *mut i8 = ptr::null_mut();
        let rc = LLVMVerifyModule(module, 2, &mut err_msg);
        if rc != 0 && !err_msg.is_null() {
            let s = std::ffi::CStr::from_ptr(err_msg).to_string_lossy();
            eprintln!("[aot] verify warning: {s}");
            LLVMDisposeMessage(err_msg);
        }
    }

    // Dump pre-optimization IR
    let ll_path = format!("{output}.ll");
    unsafe {
        let c_path = CString::new(ll_path.as_str()).unwrap();
        let mut err: *mut i8 = ptr::null_mut();
        LLVMPrintModuleToFile(module, c_path.as_ptr(), &mut err);
        if !err.is_null() { LLVMDisposeMessage(err); }
    }
    eprintln!("[aot] IR dumped to {ll_path}");

    // Run O2 optimization
    eprintln!("[aot] running O2 optimization...");
    unsafe {
        let opts = LLVMCreatePassBuilderOptions();
        let passes = c"default<O2>";
        let err = LLVMRunPasses(module, passes.as_ptr(), tm, opts);
        if !err.is_null() {
            let msg = LLVMGetErrorMessage(err);
            let s = std::ffi::CStr::from_ptr(msg).to_string_lossy();
            eprintln!("[aot] pass error: {s}");
            LLVMDisposeErrorMessage(msg);
        }
        LLVMDisposePassBuilderOptions(opts);
    }

    // Emit object file
    eprintln!("[aot] emitting {output}...");
    unsafe {
        let mut out_path = CString::new(output).unwrap().into_raw();
        let mut err: *mut i8 = ptr::null_mut();
        let rc = LLVMTargetMachineEmitToFile(tm, module, out_path, 1, &mut err); // 1 = ObjectFile
        let _ = CString::from_raw(out_path);
        if rc != 0 {
            if !err.is_null() {
                let s = std::ffi::CStr::from_ptr(err).to_string_lossy();
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
    eprintln!("[aot] link with: cc -shared -o aot.so {output}");
}

fn parse_elf_load(data: &[u8]) -> (u64, usize) {
    // Minimal ELF64 parsing: find first PT_LOAD with PF_X
    let e_phoff = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
    let e_phentsize = u16::from_le_bytes(data[54..56].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(data[56..58].try_into().unwrap()) as usize;

    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        let p_type = u32::from_le_bytes(data[off..off+4].try_into().unwrap());
        let p_flags = u32::from_le_bytes(data[off+4..off+8].try_into().unwrap());
        if p_type == 1 && (p_flags & 1) != 0 { // PT_LOAD + PF_X
            let p_offset = u64::from_le_bytes(data[off+8..off+16].try_into().unwrap());
            let p_vaddr = u64::from_le_bytes(data[off+16..off+24].try_into().unwrap());
            return (p_vaddr, p_offset as usize);
        }
    }
    eprintln!("[aot] no executable PT_LOAD segment found");
    process::exit(1);
}

fn create_target_machine() -> (LLVMTargetMachineRef, String) {
    unsafe {
        let triple = LLVMGetDefaultTargetTriple();
        let triple_str = std::ffi::CStr::from_ptr(triple).to_string_lossy().into_owned();
        let mut target: LLVMTargetRef = ptr::null_mut();
        let mut err: *mut i8 = ptr::null_mut();
        if LLVMGetTargetFromTriple(triple, &mut target, &mut err) != 0 {
            if !err.is_null() {
                let s = std::ffi::CStr::from_ptr(err).to_string_lossy();
                eprintln!("[aot] target error: {s}");
                LLVMDisposeMessage(err);
            }
            process::exit(1);
        }
        let cpu = c"generic";
        let features = c"";
        // level=2 (Aggressive), reloc=2 (PIC), code_model=3 (Small)
        let tm = LLVMCreateTargetMachine(
            target, triple, cpu.as_ptr(), features.as_ptr(), 2, 2, 3,
        );
        LLVMDisposeMessage(triple);
        (tm, triple_str)
    }
}

fn init_riscv_context(ir: &mut Context) {
    // Set up globals matching what X86_64CodeGen::init_context does
    let backend = tcg_backend::X86_64CodeGen::new();
    backend.init_context(ir);
}

fn setup_disas_temps(d: &mut RiscvDisasContext, _ir: &Context) {
    d.env = TempIdx(0);
    for i in 0..NUM_GPRS {
        d.gpr[i] = TempIdx(1 + i as u32);
    }
    d.pc = TempIdx(1 + NUM_GPRS as u32);
    d.load_res = TempIdx(1 + NUM_GPRS as u32 + 1);
    d.load_val = TempIdx(1 + NUM_GPRS as u32 + 2);
}

/// Scan TCG IR for goto_tb targets: find Mov(pc_temp, const) before GotoTb.
fn collect_goto_targets(ir: &Context, pc_temp: TempIdx, targets: &mut HashSet<u64>) {
    let ops = ir.ops();
    let mut last_pc_const: Option<u64> = None;
    for op in ops {
        let def = &OPCODE_DEFS[op.opc as usize];
        let nb_o = def.nb_oargs as usize;
        let _nb_i = def.nb_iargs as usize;
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

fn emit_tb_index_pcs(module: LLVMModuleRef, ctx: LLVMContextRef, pcs: &[u64]) {
    unsafe {
        let i64t = LLVMInt64TypeInContext(ctx);
        let mut vals: Vec<LLVMValueRef> = pcs.iter()
            .map(|&pc| LLVMConstInt(i64t, pc, 0))
            .collect();
        vals.push(LLVMConstInt(i64t, 0, 0)); // sentinel

        let arr_ty = LLVMArrayType2(i64t, vals.len() as u64);
        let arr_val = LLVMConstArray2(i64t, vals.as_ptr(), vals.len() as u64);

        let name = c"tb_index";
        let global = LLVMAddGlobal(module, arr_ty, name.as_ptr());
        LLVMSetInitializer(global, arr_val);
        LLVMSetGlobalConstant(global, 1);
        LLVMSetLinkage(global, 0); // ExternalLinkage
    }
}

/// Emit `aot_dispatch(env, guest_base) -> i64`: loads PC from env, subtracts load_vaddr,
/// switches over all AOT'd file offsets, musttail-calls the matching TB function.
/// Default case returns TB_EXIT_NOCHAIN to the exec loop.
fn emit_aot_dispatch(module: LLVMModuleRef, ctx: LLVMContextRef, all_offsets: &[u64], load_vaddr: u64) {
    const E: *const i8 = c"".as_ptr();
    unsafe {
        let i8t = LLVMInt8TypeInContext(ctx);
        let i64t = LLVMInt64TypeInContext(ctx);
        let ptr = LLVMPointerTypeInContext(ctx, 0);

        let mut params = [ptr, i64t];
        let fty = LLVMFunctionType(i64t, params.as_mut_ptr(), 2, 0);

        // Get or create the function (may already be declared by linked TB modules)
        let mut func = LLVMGetNamedFunction(module, c"aot_dispatch".as_ptr());
        if func.is_null() {
            func = LLVMAddFunction(module, c"aot_dispatch".as_ptr(), fty);
        }
        LLVMSetLinkage(func, 0); // ExternalLinkage
        LLVMSetVisibility(func, 1); // Hidden — only called within .so

        let builder = LLVMCreateBuilderInContext(ctx);
        let entry = LLVMAppendBasicBlockInContext(ctx, func, c"entry".as_ptr());
        LLVMPositionBuilderAtEnd(builder, entry);

        let env = LLVMGetParam(func, 0);
        let guest_base = LLVMGetParam(func, 1);

        // Load PC from env + PC_OFFSET
        let pc_off = LLVMConstInt(i64t, PC_OFFSET as u64, 0);
        let pc_ptr = LLVMBuildGEP2(builder, i8t, env, [pc_off].as_ptr(), 1, E);
        let pc_val = LLVMBuildLoad2(builder, i64t, pc_ptr, c"pc".as_ptr());

        // Compute file_offset = pc - load_vaddr
        let load_vaddr_val = LLVMConstInt(i64t, load_vaddr, 0);
        let file_offset_val = LLVMBuildSub(builder, pc_val, load_vaddr_val, c"file_offset".as_ptr());

        // Default (miss) block: return TB_EXIT_NOCHAIN
        let miss_bb = LLVMAppendBasicBlockInContext(ctx, func, c"miss".as_ptr());
        LLVMPositionBuilderAtEnd(builder, miss_bb);
        let nochain = LLVMConstInt(i64t, tcg_core::tb::TB_EXIT_NOCHAIN, 0);
        LLVMBuildRet(builder, nochain);

        // Build switch in entry block
        LLVMPositionBuilderAtEnd(builder, entry);
        let sw = LLVMBuildSwitch(builder, file_offset_val, miss_bb, all_offsets.len() as u32);

        // Add a case + musttail call block for each AOT'd file offset
        for &offset in all_offsets {
            let name = CString::new(format!("tb_{offset:x}")).unwrap();
            let tb_func = LLVMGetNamedFunction(module, name.as_ptr());
            if tb_func.is_null() { continue; }

            let bb_name = CString::new(format!("offset_{offset:x}")).unwrap();
            let bb = LLVMAppendBasicBlockInContext(ctx, func, bb_name.as_ptr());
            LLVMAddCase(sw, LLVMConstInt(i64t, offset, 0), bb);

            LLVMPositionBuilderAtEnd(builder, bb);
            let mut args = [env, guest_base];
            let call = LLVMBuildCall2(builder, fty, tb_func, args.as_mut_ptr(), 2, E);
            LLVMSetTailCallKind(call, 2); // MustTail
            LLVMBuildRet(builder, call);
        }

        LLVMDisposeBuilder(builder);
    }
    eprintln!("[aot] emitted aot_dispatch with {} cases", all_offsets.len());
}
