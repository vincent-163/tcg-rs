//! Differential testing: compare tcg-rs AArch64 instruction
//! simulation against QEMU (qemu-aarch64 user-mode).
//!
//! For each test case we:
//! 1. Run the instruction through tcg-rs full pipeline
//! 2. Generate AArch64 assembly, cross-compile, run under
//!    qemu-aarch64, and parse the register dump
//! 3. Compare the specified output registers (and optionally NZCV)

use std::io::Write;
use std::process::Command;

use tcg_backend::code_buffer::CodeBuffer;
use tcg_backend::translate::translate_and_execute;
use tcg_backend::HostCodeGen;
use tcg_backend::X86_64CodeGen;
use tcg_core::opcode::Opcode;
use tcg_core::types::MemOp;
use tcg_core::Context;
use tcg_frontend::aarch64::cpu::Aarch64Cpu;
use tcg_frontend::aarch64::{Aarch64DisasContext, Aarch64Translator};
use tcg_frontend::translator_loop;

// ── AArch64 instruction encoders ─────────────────────────

// Data processing - immediate - Add/Sub
fn a64_add_imm(sf: u32, rd: u32, rn: u32, imm12: u32) -> u32 {
    (sf << 31) | (0b00100010 << 23) | (imm12 << 10) | (rn << 5) | rd
}
fn a64_sub_imm(sf: u32, rd: u32, rn: u32, imm12: u32) -> u32 {
    (sf << 31) | (0b10100010 << 23) | (imm12 << 10) | (rn << 5) | rd
}
fn a64_adds_imm(sf: u32, rd: u32, rn: u32, imm12: u32) -> u32 {
    (sf << 31) | (0b01100010 << 23) | (imm12 << 10) | (rn << 5) | rd
}
fn a64_subs_imm(sf: u32, rd: u32, rn: u32, imm12: u32) -> u32 {
    (sf << 31) | (0b11100010 << 23) | (imm12 << 10) | (rn << 5) | rd
}

// Data processing - register - Add/Sub (shifted)
fn a64_add_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0001011000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_sub_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b1001011000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_adds_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0101011000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_subs_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b1101011000 << 21) | (rm << 16) | (rn << 5) | rd
}

// Logical (shifted register)
fn a64_and_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0001010000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_orr_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0101010000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_eor_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b1001010000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_ands_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b1101010000 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_orn_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0101010001 << 21) | (rm << 16) | (rn << 5) | rd
}
fn a64_bic_r(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31) | (0b0001010001 << 21) | (rm << 16) | (rn << 5) | rd
}

// Shift variable
fn a64_lslv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b001000 << 10)
        | (rn << 5)
        | rd
}
fn a64_lsrv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b001001 << 10)
        | (rn << 5)
        | rd
}
fn a64_asrv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b001010 << 10)
        | (rn << 5)
        | rd
}
fn a64_rorv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b001011 << 10)
        | (rn << 5)
        | rd
}

// Multiply
fn a64_madd(sf: u32, rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    (sf << 31) | (0b0011011000 << 21) | (rm << 16) | (ra << 10) | (rn << 5) | rd
}
fn a64_msub(sf: u32, rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    (sf << 31)
        | (0b0011011000 << 21)
        | (rm << 16)
        | (1 << 15)
        | (ra << 10)
        | (rn << 5)
        | rd
}

// Division
fn a64_udiv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b000010 << 10)
        | (rn << 5)
        | rd
}
fn a64_sdiv(sf: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    (sf << 31)
        | (0b0011010110 << 21)
        | (rm << 16)
        | (0b000011 << 10)
        | (rn << 5)
        | rd
}

// Unsigned high multiply
fn a64_umulh(rd: u32, rn: u32, rm: u32) -> u32 {
    0x9bc0_0000 | (rm << 16) | (31 << 10) | (rn << 5) | rd
}

fn a64_ubfm(sf: u32, rd: u32, rn: u32, immr: u32, imms: u32) -> u32 {
    let n = sf;
    (sf << 31)
        | (0b10 << 29)
        | (0b100110 << 23)
        | (n << 22)
        | (immr << 16)
        | (imms << 10)
        | (rn << 5)
        | rd
}

fn a64_sbfm(sf: u32, rd: u32, rn: u32, immr: u32, imms: u32) -> u32 {
    let n = sf;
    (sf << 31)
        | (0b00 << 29)
        | (0b100110 << 23)
        | (n << 22)
        | (immr << 16)
        | (imms << 10)
        | (rn << 5)
        | rd
}

fn a64_bfm(sf: u32, rd: u32, rn: u32, immr: u32, imms: u32) -> u32 {
    let n = sf;
    (sf << 31)
        | (0b01 << 29)
        | (0b100110 << 23)
        | (n << 22)
        | (immr << 16)
        | (imms << 10)
        | (rn << 5)
        | rd
}

// Move wide
fn a64_movz(sf: u32, rd: u32, imm16: u32, hw: u32) -> u32 {
    (sf << 31) | (0b10100101 << 23) | (hw << 21) | (imm16 << 5) | rd
}
fn a64_movn(sf: u32, rd: u32, imm16: u32, hw: u32) -> u32 {
    (sf << 31) | (0b00100101 << 23) | (hw << 21) | (imm16 << 5) | rd
}
fn a64_movk(sf: u32, rd: u32, imm16: u32, hw: u32) -> u32 {
    (sf << 31) | (0b11100101 << 23) | (hw << 21) | (imm16 << 5) | rd
}

// Conditional select
fn a64_csel(sf: u32, rd: u32, rn: u32, rm: u32, cond: u32) -> u32 {
    (sf << 31)
        | (0b0011010100 << 21)
        | (rm << 16)
        | (cond << 12)
        | (rn << 5)
        | rd
}
fn a64_csinc(sf: u32, rd: u32, rn: u32, rm: u32, cond: u32) -> u32 {
    (sf << 31)
        | (0b0011010100 << 21)
        | (rm << 16)
        | (cond << 12)
        | (0b01 << 10)
        | (rn << 5)
        | rd
}

// CLZ
fn a64_clz(sf: u32, rd: u32, rn: u32) -> u32 {
    (sf << 31) | (0b1011010110 << 21) | (0b000100 << 10) | (rn << 5) | rd
}

// REV (byte reverse)
fn a64_rev(sf: u32, rd: u32, rn: u32) -> u32 {
    if sf == 1 {
        // REV X: opc=11
        (1 << 31) | (0b1011010110 << 21) | (0b000011 << 10) | (rn << 5) | rd
    } else {
        // REV W: opc=10
        (0b1011010110 << 21) | (0b000010 << 10) | (rn << 5) | rd
    }
}
fn a64_rev16(sf: u32, rd: u32, rn: u32) -> u32 {
    (sf << 31) | (0b1011010110 << 21) | (0b000001 << 10) | (rn << 5) | rd
}
fn a64_rev32(rd: u32, rn: u32) -> u32 {
    (1 << 31) | (0b1011010110 << 21) | (0b000010 << 10) | (rn << 5) | rd
}

// ── Difftest infrastructure ──────────────────────────────

/// A single ALU difftest case.
struct AluTest {
    name: &'static str,
    /// AArch64 assembly for the QEMU side.
    asm: String,
    /// Machine code for tcg-rs.
    insn: u32,
    /// (xreg_index, value) pairs to initialize before the test.
    init: Vec<(usize, u64)>,
    /// Register index to check after execution.
    check_reg: usize,
    /// Also compare NZCV flags.
    check_nzcv: bool,
}

/// A branch difftest case.
struct BranchTest {
    name: &'static str,
    /// Assembly for the branch instruction.
    asm: String,
    /// Machine code for tcg-rs (branch offset = +16 bytes).
    insn: u32,
    /// Initial register values.
    init: Vec<(usize, u64)>,
}

/// AArch64 register names for assembly generation.
const XREG_NAME: [&str; 32] = [
    "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11",
    "x12", "x13", "x14", "x15", "x16", "x17", "x18", "x19", "x20", "x21",
    "x22", "x23", "x24", "x25", "x26", "x27", "x28", "x29", "x30", "sp",
];

/// Generate assembly source for an ALU difftest.
/// Uses x1, x2 as source regs, x0 as dest.
/// x3 is reserved for the save-area pointer.
fn gen_alu_asm(test: &AluTest) -> String {
    let mut asm = String::from(
        ".global _start\n_start:\n    adrp x3, save_area\n\
         \x20   add x3, x3, :lo12:save_area\n",
    );
    // Load initial register values
    for &(reg, val) in &test.init {
        assert!(reg != 3, "x3 reserved for save area");
        asm.push_str(&format!(
            "    mov {}, #0x{:x}\n",
            XREG_NAME[reg],
            val & 0xFFFF
        ));
        if (val >> 16) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #16\n",
                XREG_NAME[reg],
                (val >> 16) & 0xFFFF
            ));
        }
        if (val >> 32) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #32\n",
                XREG_NAME[reg],
                (val >> 32) & 0xFFFF
            ));
        }
        if (val >> 48) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #48\n",
                XREG_NAME[reg],
                (val >> 48) & 0xFFFF
            ));
        }
    }
    // Test instruction
    asm.push_str(&format!("    {}\n", test.asm));
    // Save NZCV to x4 if needed
    if test.check_nzcv {
        asm.push_str("    mrs x4, nzcv\n");
    }
    // Save registers x0-x30 to save area
    for i in 0..31 {
        asm.push_str(&format!("    str {}, [x3, #{}]\n", XREG_NAME[i], i * 8));
    }
    // write(1, save_area, 248)
    asm.push_str(
        "    mov x8, #64\n\
         \x20   mov x0, #1\n\
         \x20   mov x1, x3\n\
         \x20   mov x2, #248\n\
         \x20   svc #0\n\
         \x20   mov x8, #93\n\
         \x20   mov x0, #0\n\
         \x20   svc #0\n\
         .bss\n\
         .align 3\n\
         save_area: .space 248\n",
    );
    asm
}
/// Generate assembly for a branch difftest.
/// Sets x1, x2 as source regs, branches, records
/// taken=1 / not-taken=0 in x0.
fn gen_branch_asm(test: &BranchTest) -> String {
    let mut asm = String::from(
        ".global _start\n_start:\n    adrp x3, save_area\n\
         \x20   add x3, x3, :lo12:save_area\n",
    );
    for &(reg, val) in &test.init {
        assert!(reg != 3, "x3 reserved for save area");
        asm.push_str(&format!(
            "    mov {}, #0x{:x}\n",
            XREG_NAME[reg],
            val & 0xFFFF
        ));
        if (val >> 16) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #16\n",
                XREG_NAME[reg],
                (val >> 16) & 0xFFFF
            ));
        }
        if (val >> 32) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #32\n",
                XREG_NAME[reg],
                (val >> 32) & 0xFFFF
            ));
        }
        if (val >> 48) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #48\n",
                XREG_NAME[reg],
                (val >> 48) & 0xFFFF
            ));
        }
    }
    asm.push_str(&format!(
        "    {}\n\
         \x20   mov x0, #0\n\
         \x20   b 2f\n\
         1:  mov x0, #1\n\
         2:\n",
        test.asm
    ));
    for i in 0..31 {
        asm.push_str(&format!("    str {}, [x3, #{}]\n", XREG_NAME[i], i * 8));
    }
    asm.push_str(
        "    mov x8, #64\n\
         \x20   mov x0, #1\n\
         \x20   mov x1, x3\n\
         \x20   mov x2, #248\n\
         \x20   svc #0\n\
         \x20   mov x8, #93\n\
         \x20   mov x0, #0\n\
         \x20   svc #0\n\
         .bss\n\
         .align 3\n\
         save_area: .space 248\n",
    );
    asm
}

/// Cross-compile assembly and run under qemu-aarch64.
/// Returns the 31-element register array (x0-x30).
fn run_qemu(asm_src: &str) -> [u64; 31] {
    let dir = std::env::temp_dir();
    let id = std::process::id();
    let tid: u64 = unsafe { std::mem::transmute(std::thread::current().id()) };
    let tag = format!("a64_difftest_{id}_{tid}");
    let s_path = dir.join(format!("{tag}.S"));
    let elf_path = dir.join(format!("{tag}.elf"));

    {
        let mut f = std::fs::File::create(&s_path).unwrap();
        f.write_all(asm_src.as_bytes()).unwrap();
    }

    let cc = Command::new("aarch64-none-linux-gnu-gcc")
        .args([
            "-nostdlib",
            "-static",
            "-o",
            elf_path.to_str().unwrap(),
            s_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run aarch64-none-linux-gnu-gcc");
    assert!(
        cc.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&cc.stderr)
    );

    let qemu = Command::new("qemu-aarch64")
        .arg(elf_path.to_str().unwrap())
        .output()
        .expect("failed to run qemu-aarch64");
    assert!(
        qemu.status.success(),
        "qemu-aarch64 exited with {:?}\nstderr: {}",
        qemu.status.code(),
        String::from_utf8_lossy(&qemu.stderr)
    );
    assert_eq!(
        qemu.stdout.len(),
        248,
        "expected 248 bytes (31 regs), got {}",
        qemu.stdout.len()
    );

    let mut regs = [0u64; 31];
    for i in 0..31 {
        let off = i * 8;
        regs[i] =
            u64::from_le_bytes(qemu.stdout[off..off + 8].try_into().unwrap());
    }

    let _ = std::fs::remove_file(&s_path);
    let _ = std::fs::remove_file(&elf_path);

    regs
}

/// Run instruction(s) through tcg-rs and return the CPU state.
fn run_tcgrs(init: &[(usize, u64)], insns: &[u32]) -> Aarch64Cpu {
    let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    let mut mem = vec![0u8; 4096];
    mem[..code.len()].copy_from_slice(&code);
    let guest_base = mem.as_ptr();

    let mut backend = X86_64CodeGen::new();
    backend.guest_base_offset =
        tcg_frontend::aarch64::cpu::GUEST_BASE_OFFSET as i32;
    let mut buf = CodeBuffer::new(4096).unwrap();
    backend.emit_prologue(&mut buf);
    backend.emit_epilogue(&mut buf);

    let mut ctx = Context::new();
    backend.init_context(&mut ctx);

    let mut disas = Aarch64DisasContext::new(0, guest_base);
    disas.base.max_insns = insns.len() as u32;
    translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);

    let mut cpu = Aarch64Cpu::new();
    for &(reg, val) in init {
        if reg < 31 {
            cpu.xregs[reg] = val;
        }
    }

    unsafe {
        translate_and_execute(
            &mut ctx,
            &backend,
            &mut buf,
            &mut cpu as *mut Aarch64Cpu as *mut u8,
        );
    }
    // Materialize NZCV from lazy state if needed.
    use tcg_frontend::aarch64::cpu::{
        helper_lazy_nzcv_to_packed, CC_OP_EAGER,
    };
    if cpu.cc_op != CC_OP_EAGER {
        cpu.nzcv = helper_lazy_nzcv_to_packed(
            cpu.cc_op, cpu.cc_a, cpu.cc_b, cpu.cc_result,
        );
        cpu.cc_op = CC_OP_EAGER;
    }
    cpu
}

fn run_tcgrs_with_state(
    x_init: &[(usize, u64)],
    v_init: &[(usize, u64, u64)],
    insns: &[u32],
) -> Aarch64Cpu {
    let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    let mut mem = vec![0u8; 4096];
    mem[..code.len()].copy_from_slice(&code);
    let guest_base = mem.as_ptr();

    let mut backend = X86_64CodeGen::new();
    let mut buf = CodeBuffer::new(4096).unwrap();
    backend.emit_prologue(&mut buf);
    backend.emit_epilogue(&mut buf);

    let mut ctx = Context::new();
    backend.init_context(&mut ctx);

    let mut disas = Aarch64DisasContext::new(0, guest_base);
    disas.base.max_insns = insns.len() as u32;
    translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);

    let mut cpu = Aarch64Cpu::new();
    cpu.guest_base = guest_base as u64;
    for &(reg, val) in x_init {
        if reg < 31 {
            cpu.xregs[reg] = val;
        }
    }
    for &(reg, lo, hi) in v_init {
        if reg < 32 {
            cpu.vregs[reg * 2] = lo;
            cpu.vregs[reg * 2 + 1] = hi;
        }
    }

    unsafe {
        translate_and_execute(
            &mut ctx,
            &backend,
            &mut buf,
            &mut cpu as *mut Aarch64Cpu as *mut u8,
        );
    }
    use tcg_frontend::aarch64::cpu::{
        helper_lazy_nzcv_to_packed, CC_OP_EAGER,
    };
    if cpu.cc_op != CC_OP_EAGER {
        cpu.nzcv = helper_lazy_nzcv_to_packed(
            cpu.cc_op, cpu.cc_a, cpu.cc_b, cpu.cc_result,
        );
        cpu.cc_op = CC_OP_EAGER;
    }
    cpu
}

/// Run an ALU difftest: compare tcg-rs vs QEMU.
fn difftest_alu(test: &AluTest) {
    let asm = gen_alu_asm(test);
    let qemu_regs = run_qemu(&asm);
    let cpu = run_tcgrs(&test.init, &[test.insn]);
    let r = test.check_reg;
    let tcgrs_val = if r < 31 { cpu.xregs[r] } else { 0 };
    assert_eq!(
        tcgrs_val, qemu_regs[r],
        "DIFFTEST FAIL [{}]: x{} tcg-rs={:#x} qemu={:#x}",
        test.name, r, tcgrs_val, qemu_regs[r]
    );
    if test.check_nzcv {
        // QEMU stores NZCV in x4 via mrs
        let qemu_nzcv = qemu_regs[4];
        assert_eq!(
            cpu.nzcv, qemu_nzcv,
            "DIFFTEST FAIL [{}]: nzcv tcg-rs={:#x} qemu={:#x}",
            test.name, cpu.nzcv, qemu_nzcv
        );
    }
}

/// Run a branch difftest: compare taken/not-taken.
fn difftest_branch(test: &BranchTest) {
    let asm = gen_branch_asm(test);
    let qemu_regs = run_qemu(&asm);
    let qemu_taken = qemu_regs[0]; // x0

    let cpu = run_tcgrs(&test.init, &[test.insn]);
    // If taken → PC = 0 + 16 = 16; if not taken → PC = 4.
    let tcgrs_taken: u64 = if cpu.pc == 16 { 1 } else { 0 };

    assert_eq!(
        tcgrs_taken, qemu_taken,
        "DIFFTEST FAIL [{}]: tcg-rs_taken={} (pc={:#x}) \
         qemu_taken={}",
        test.name, tcgrs_taken, cpu.pc, qemu_taken
    );
}

/// Run an arbitrary instruction sequence through both backends and compare.
fn difftest_sequence(
    name: &str,
    init: &[(usize, u64)],
    insns: &[u32],
    asm_body: &str,
    check_regs: &[usize],
    check_nzcv: bool,
) {
    let mut asm = String::from(
        ".global _start\n_start:\n    adrp x3, save_area\n\
         \x20   add x3, x3, :lo12:save_area\n",
    );
    for &(reg, val) in init {
        assert!(reg != 3, "x3 reserved for save area");
        asm.push_str(&format!(
            "    mov {}, #0x{:x}\n",
            XREG_NAME[reg],
            val & 0xFFFF
        ));
        if (val >> 16) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #16\n",
                XREG_NAME[reg],
                (val >> 16) & 0xFFFF
            ));
        }
        if (val >> 32) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #32\n",
                XREG_NAME[reg],
                (val >> 32) & 0xFFFF
            ));
        }
        if (val >> 48) & 0xFFFF != 0 {
            asm.push_str(&format!(
                "    movk {}, #0x{:x}, lsl #48\n",
                XREG_NAME[reg],
                (val >> 48) & 0xFFFF
            ));
        }
    }
    asm.push_str(asm_body);
    if !asm_body.ends_with('\n') {
        asm.push('\n');
    }
    if check_nzcv {
        asm.push_str("    mrs x4, nzcv\n");
    }
    for i in 0..31 {
        asm.push_str(&format!("    str {}, [x3, #{}]\n", XREG_NAME[i], i * 8));
    }
    asm.push_str(
        "    mov x8, #64\n\
         \x20   mov x0, #1\n\
         \x20   mov x1, x3\n\
         \x20   mov x2, #248\n\
         \x20   svc #0\n\
         \x20   mov x8, #93\n\
         \x20   mov x0, #0\n\
         \x20   svc #0\n\
         .bss\n\
         .align 3\n\
         save_area: .space 248\n",
    );

    let qemu_regs = run_qemu(&asm);
    let cpu = run_tcgrs(init, insns);
    for &r in check_regs {
        let tcg_v = if r < 31 { cpu.xregs[r] } else { 0 };
        assert_eq!(
            tcg_v, qemu_regs[r],
            "DIFFTEST FAIL [{name}]: x{r} tcg-rs={:#x} qemu={:#x}",
            tcg_v, qemu_regs[r]
        );
    }
    if check_nzcv {
        let qemu_nzcv = qemu_regs[4];
        assert_eq!(
            cpu.nzcv, qemu_nzcv,
            "DIFFTEST FAIL [{name}]: nzcv tcg-rs={:#x} qemu={:#x}",
            cpu.nzcv, qemu_nzcv
        );
    }
}

// ── Edge-case values ─────────────────────────────────────

const V0: u64 = 0;
const V1: u64 = 1;
const VMAX: u64 = 0x7FFF_FFFF_FFFF_FFFF; // i64::MAX
const VMIN: u64 = 0x8000_0000_0000_0000; // i64::MIN
const VNEG1: u64 = 0xFFFF_FFFF_FFFF_FFFF; // -1
const V32MAX: u64 = 0x7FFF_FFFF; // i32::MAX
const V32MIN: u64 = 0xFFFF_FFFF_8000_0000; // i32::MIN sext
const V32FF: u64 = 0xFFFF_FFFF; // u32::MAX
const VPATTERN: u64 = 0xDEAD_BEEF_CAFE_BABE;
// ── Helpers ──────────────────────────────────────────────

/// Build an R-type ALU test (64-bit) with two source values.
fn rtype64(
    name: &'static str,
    mnemonic: &str,
    insn: u32,
    v1: u64,
    v2: u64,
) -> AluTest {
    AluTest {
        name,
        asm: format!("{} x0, x1, x2", mnemonic),
        insn,
        init: vec![(1, v1), (2, v2)],
        check_reg: 0,
        check_nzcv: false,
    }
}

/// Build an R-type ALU test (32-bit) with two source values.
fn rtype32(
    name: &'static str,
    mnemonic: &str,
    insn: u32,
    v1: u64,
    v2: u64,
) -> AluTest {
    AluTest {
        name,
        asm: format!("{} w0, w1, w2", mnemonic),
        insn,
        init: vec![(1, v1), (2, v2)],
        check_reg: 0,
        check_nzcv: false,
    }
}

/// Build a flag-setting R-type test (64-bit).
fn rtype64_s(
    name: &'static str,
    mnemonic: &str,
    insn: u32,
    v1: u64,
    v2: u64,
) -> AluTest {
    AluTest {
        name,
        asm: format!("{} x0, x1, x2", mnemonic),
        insn,
        init: vec![(1, v1), (2, v2)],
        check_reg: 0,
        check_nzcv: true,
    }
}

/// Build an immediate ALU test (64-bit).
fn itype64(name: &'static str, asm: &str, insn: u32, v1: u64) -> AluTest {
    AluTest {
        name,
        asm: asm.to_string(),
        insn,
        init: vec![(1, v1)],
        check_reg: 0,
        check_nzcv: false,
    }
}

// ── R-type ALU difftests (64-bit) ────────────────────────

#[test]
fn a64_difftest_add() {
    let cases: Vec<(u64, u64)> = vec![
        (V0, V0),
        (V1, VNEG1),
        (VMAX, V1),
        (VMIN, VNEG1),
        (VPATTERN, V32FF),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("add", "add", a64_add_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_sub() {
    let cases: Vec<(u64, u64)> = vec![
        (V0, V0),
        (V0, V1),
        (VMIN, V1),
        (VMAX, VNEG1),
        (VPATTERN, VPATTERN),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("sub", "sub", a64_sub_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_adc_carry_in_64() {
    // cmp x7, x8; adc x0, x1, x2
    let seq = [0xeb08_00ff_u32, 0x9a02_0020_u32];
    for (name, x7, x8, expected) in [
        ("adc64_c1", 5_u64, 4_u64, 4_u64),
        ("adc64_c0", 4_u64, 5_u64, 3_u64),
    ] {
        let cpu = run_tcgrs(&[(1, 1), (2, 2), (7, x7), (8, x8)], &seq);
        assert_eq!(
            cpu.xregs[0], expected,
            "unexpected adc64 result for {name}"
        );
        difftest_sequence(
            name,
            &[(1, 1), (2, 2), (7, x7), (8, x8)],
            &seq,
            "    cmp x7, x8\n    adc x0, x1, x2\n",
            &[0],
            false,
        );
    }
}

#[test]
fn a64_difftest_adc_carry_in_32() {
    // cmp w7, w8; adc w0, w1, w2
    let seq = [0x6b08_00ff_u32, 0x1a02_0020_u32];
    for (name, w7, w8, expected) in [
        ("adc32_c1", 5_u64, 4_u64, 4_u64),
        ("adc32_c0", 4_u64, 5_u64, 3_u64),
    ] {
        let cpu = run_tcgrs(&[(1, 1), (2, 2), (7, w7), (8, w8)], &seq);
        assert_eq!(
            cpu.xregs[0], expected,
            "unexpected adc32 result for {name}"
        );
        difftest_sequence(
            name,
            &[(1, 1), (2, 2), (7, w7), (8, w8)],
            &seq,
            "    cmp w7, w8\n    adc w0, w1, w2\n",
            &[0],
            false,
        );
    }
}

#[test]
fn a64_difftest_sbc_carry_in_64() {
    // cmp x7, x8; sbc x0, x1, x2
    let seq = [0xeb08_00ff_u32, 0xda02_0020_u32];
    for (name, x7, x8, expected) in [
        ("sbc64_c1", 5_u64, 4_u64, 0_u64),
        ("sbc64_c0", 4_u64, 5_u64, u64::MAX),
    ] {
        let cpu = run_tcgrs(&[(1, 0), (2, 0), (7, x7), (8, x8)], &seq);
        assert_eq!(
            cpu.xregs[0], expected,
            "unexpected sbc64 result for {name}"
        );
        difftest_sequence(
            name,
            &[(1, 0), (2, 0), (7, x7), (8, x8)],
            &seq,
            "    cmp x7, x8\n    sbc x0, x1, x2\n",
            &[0],
            false,
        );
    }
}

#[test]
fn a64_difftest_sbc_carry_in_32() {
    // cmp w7, w8; sbc w0, w1, w2
    let seq = [0x6b08_00ff_u32, 0x5a02_0020_u32];
    for (name, w7, w8, expected) in [
        ("sbc32_c1", 5_u64, 4_u64, 0_u64),
        ("sbc32_c0", 4_u64, 5_u64, 0xffff_ffff_u64),
    ] {
        let cpu = run_tcgrs(&[(1, 0), (2, 0), (7, w7), (8, w8)], &seq);
        assert_eq!(
            cpu.xregs[0], expected,
            "unexpected sbc32 result for {name}"
        );
        difftest_sequence(
            name,
            &[(1, 0), (2, 0), (7, w7), (8, w8)],
            &seq,
            "    cmp w7, w8\n    sbc w0, w1, w2\n",
            &[0],
            false,
        );
    }
}

#[test]
fn a64_difftest_sub_shifted_reg_patterns() {
    // Patterns seen in glibc strcmp.
    let neg_lsl3 = 0xcb01_0fe9u32; // neg x9, x1, lsl #3
    for v in [0, 1, 6, 0x1234_5678_9abc_def0, VNEG1] {
        difftest_sequence(
            "neg_x_lsl3",
            &[(1, v)],
            &[neg_lsl3],
            "    neg x9, x1, lsl #3\n",
            &[9],
            false,
        );
    }

    let sub_lsr56 = 0xcb47_e040u32; // sub x0, x2, x7, lsr #56
    for (x2, x7) in [
        (0x1122_3344_5566_7788, 0x8877_6655_4433_2211),
        (VNEG1, VMAX),
        (0, VNEG1),
        (VPATTERN, 0x0102_0304_0506_0708),
    ] {
        difftest_sequence(
            "sub_x_lsr56",
            &[(2, x2), (7, x7)],
            &[sub_lsr56],
            "    sub x0, x2, x7, lsr #56\n",
            &[0],
            false,
        );
    }
}

#[test]
fn a64_difftest_neg_lsl3_then_lsrv_mask64() {
    // strcmp uses:
    //   neg x9, x1, lsl #3
    //   lsr x6, x8, x9
    // Shift amount must use low 6 bits of x9 (64-bit variable shift).
    let seq = [0xcb01_0fe9u32, 0x9ac9_2506u32];
    for low in 0u64..8 {
        let x1 = 0x1234_5678_9abc_de00u64 | low;
        difftest_sequence(
            "neg_lsl3_then_lsrv_mask64",
            &[(1, x1), (8, 0x0101_0101_0101_0101)],
            &seq,
            "    neg x9, x1, lsl #3\n    lsr x6, x8, x9\n",
            &[6, 9],
            false,
        );
    }
}

#[test]
fn a64_difftest_and() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, VNEG1),
        (VNEG1, V0),
        (VPATTERN, V32FF),
        (0xFF00, 0x0FF0),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("and", "and", a64_and_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_orr() {
    let cases: Vec<(u64, u64)> =
        vec![(V0, V0), (VPATTERN, V0), (0xF0F0, 0x0F0F), (VMIN, VMAX)];
    for (a, b) in cases {
        difftest_alu(&rtype64("orr", "orr", a64_orr_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_eor() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, VNEG1),
        (VNEG1, V0),
        (VPATTERN, VNEG1),
        (V32MAX, V32FF),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("eor", "eor", a64_eor_r(1, 0, 1, 2), a, b));
    }
}

#[test]
#[ignore] // BUG: backend missing OrC opcode constraints in regalloc
fn a64_difftest_orn() {
    let cases: Vec<(u64, u64)> = vec![(V0, V0), (V0, VNEG1), (VPATTERN, V32FF)];
    for (a, b) in cases {
        difftest_alu(&rtype64("orn", "orn", a64_orn_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_bic() {
    let cases: Vec<(u64, u64)> =
        vec![(VNEG1, V0), (VNEG1, VNEG1), (VPATTERN, V32FF)];
    for (a, b) in cases {
        difftest_alu(&rtype64("bic", "bic", a64_bic_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_bics() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, V0),
        (VNEG1, VNEG1),
        (VPATTERN, V32FF),
        (0x0101_0101_0101_0101, 0x7f7f_7f7f_7f7f_7f7f),
    ];
    for (a, b) in cases {
        difftest_alu(&AluTest {
            name: "bics",
            asm: "bics x0, x1, x2".to_string(),
            insn: 0xea22_0020, // bics x0, x1, x2
            init: vec![(1, a), (2, b)],
            check_reg: 0,
            check_nzcv: true,
        });
    }
}
// ── Shift variable difftests ─────────────────────────────

#[test]
fn a64_difftest_lslv() {
    let cases: Vec<(u64, u64)> =
        vec![
            (V1, 0),
            (V1, 63),
            (VNEG1, 32),
            (VPATTERN, 4),
            (V32MAX, 1),
            (VPATTERN, 64),
            (VPATTERN, 65),
            (VPATTERN, 0xffff_ffff_ffff_ffff),
        ];
    for (a, b) in cases {
        difftest_alu(&rtype64("lslv", "lsl", a64_lslv(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_lsrv() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, 0),
        (VNEG1, 1),
        (VNEG1, 63),
        (VPATTERN, 16),
        (VMIN, 32),
        (VPATTERN, 64),
        (VPATTERN, 65),
        (VPATTERN, 0xffff_ffff_ffff_ffff),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("lsrv", "lsr", a64_lsrv(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_asrv() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, 0),
        (VNEG1, 1),
        (VNEG1, 63),
        (VMIN, 32),
        (VMAX, 32),
        (VPATTERN, 8),
        (VPATTERN, 64),
        (VPATTERN, 65),
        (VPATTERN, 0xffff_ffff_ffff_ffff),
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("asrv", "asr", a64_asrv(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_rorv() {
    let cases: Vec<(u64, u64)> =
        vec![
            (VNEG1, 0),
            (V1, 1),
            (VPATTERN, 16),
            (VMIN, 63),
            (VPATTERN, 64),
            (VPATTERN, 65),
            (VPATTERN, 0xffff_ffff_ffff_ffff),
        ];
    for (a, b) in cases {
        difftest_alu(&rtype64("rorv", "ror", a64_rorv(1, 0, 1, 2), a, b));
    }
}

// ── Multiply / Divide difftests ──────────────────────────

#[test]
fn a64_difftest_madd() {
    // MADD x0, x1, x2, xzr  =>  MUL x0, x1, x2
    let cases: Vec<(u64, u64)> = vec![
        (V0, V0),
        (V1, VNEG1),
        (VMAX, 2),
        (0x1234, 0x5678),
        (VMIN, 2),
    ];
    for (a, b) in cases {
        difftest_alu(&AluTest {
            name: "madd",
            asm: "mul x0, x1, x2".to_string(),
            insn: a64_madd(1, 0, 1, 2, 31), // ra=xzr
            init: vec![(1, a), (2, b)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_msub() {
    // MSUB x0, x1, x2, x5  =>  x0 = x5 - x1*x2
    let cases: Vec<(u64, u64, u64)> = vec![
        (2, 3, 10),   // 10 - 6 = 4
        (V1, V1, V0), // 0 - 1 = -1
        (0, 100, 42), // 42 - 0 = 42
    ];
    for (a, b, c) in cases {
        difftest_alu(&AluTest {
            name: "msub",
            asm: "msub x0, x1, x2, x5".to_string(),
            insn: a64_msub(1, 0, 1, 2, 5),
            init: vec![(1, a), (2, b), (5, c)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_udiv() {
    let cases: Vec<(u64, u64)> = vec![
        (100, 10),
        (VNEG1, 2),
        (V0, V1),
        (VMAX, VMAX),
        (42, 0), // div by zero → 0
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("udiv", "udiv", a64_udiv(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_umulh() {
    let cases: Vec<(u64, u64)> = vec![
        (0, 0),
        (1, 1),
        (VNEG1, VNEG1),
        (VMAX, VMAX),
        (0x1234_5678_9abc_def0, 0xfedc_ba98_7654_3210),
    ];
    for (a, b) in cases {
        difftest_alu(&AluTest {
            name: "umulh",
            asm: "umulh x0, x1, x2".to_string(),
            insn: a64_umulh(0, 1, 2),
            init: vec![(1, a), (2, b)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_logical_imm_masks() {
    let and_cases: Vec<u64> = vec![0, 1, 7, 8, 0x1234_5678_9abc_def0, VNEG1];
    for v in and_cases {
        difftest_alu(&AluTest {
            name: "and_imm_fff8",
            asm: "and x25, x25, #0xfffffffffffffff8".to_string(),
            insn: 0x927d_f339, // and x25, x25, #0xfffffffffffffff8
            init: vec![(25, v)],
            check_reg: 25,
            check_nzcv: false,
        });
    }
    difftest_alu(&AluTest {
        name: "mov_log_imm_fc000000",
        asm: "mov x0, #0xfffffffffc000000".to_string(),
        insn: 0xb266_97e0,
        init: vec![],
        check_reg: 0,
        check_nzcv: false,
    });

    // Masks used by glibc strcmp hot path.
    for v in [0, 1, VNEG1, VPATTERN, 0x0102_0304_0506_0708] {
        difftest_alu(&AluTest {
            name: "orr_imm_7f7f",
            asm: "orr x6, x2, #0x7f7f7f7f7f7f7f7f".to_string(),
            insn: 0xb200_d846,
            init: vec![(2, v)],
            check_reg: 6,
            check_nzcv: false,
        });
    }
    difftest_alu(&AluTest {
        name: "mov_imm_0101",
        asm: "mov x8, #0x0101010101010101".to_string(),
        insn: 0xb200_c3e8,
        init: vec![],
        check_reg: 8,
        check_nzcv: false,
    });
}

#[test]
fn a64_difftest_bitfield_ubfm() {
    let vals = [V0, VNEG1, VPATTERN, VMAX, VMIN];
    let imm_pairs = [(0, 7), (8, 15), (16, 31), (4, 3), (60, 3)];
    for &v in &vals {
        for &(immr, imms) in &imm_pairs {
            difftest_alu(&AluTest {
                name: "ubfm",
                asm: format!("ubfm x0, x1, #{immr}, #{imms}"),
                insn: a64_ubfm(1, 0, 1, immr, imms),
                init: vec![(1, v)],
                check_reg: 0,
                check_nzcv: false,
            });
        }
    }
}

#[test]
fn a64_difftest_bitfield_sbfm() {
    let vals = [V0, VNEG1, VPATTERN, VMAX, VMIN];
    let imm_pairs = [(0, 7), (8, 15), (16, 31), (4, 3), (60, 3)];
    for &v in &vals {
        for &(immr, imms) in &imm_pairs {
            difftest_alu(&AluTest {
                name: "sbfm",
                asm: format!("sbfm x0, x1, #{immr}, #{imms}"),
                insn: a64_sbfm(1, 0, 1, immr, imms),
                init: vec![(1, v)],
                check_reg: 0,
                check_nzcv: false,
            });
        }
    }
}

#[test]
fn a64_difftest_bitfield_bfm() {
    let src_vals = [V0, VNEG1, VPATTERN, VMAX, VMIN];
    let dst_vals = [0u64, 0x0123_4567_89ab_cdef, VNEG1];
    let imm_pairs = [(0, 7), (8, 15), (16, 31), (4, 3), (60, 3)];
    for &dst in &dst_vals {
        for &src in &src_vals {
            for &(immr, imms) in &imm_pairs {
                difftest_alu(&AluTest {
                    name: "bfm",
                    asm: format!("bfm x0, x1, #{immr}, #{imms}"),
                    insn: a64_bfm(1, 0, 1, immr, imms),
                    init: vec![(0, dst), (1, src)],
                    check_reg: 0,
                    check_nzcv: false,
                });
            }
        }
    }
}

#[test]
fn a64_difftest_ccmp_nzcv_seq() {
    // Sequence seen in glibc __calloc:
    //   cmp  x23, x26
    //   ccmp x25, x2, #2, eq
    // Compare NZCV against QEMU.
    let seq = [a64_subs_r(1, 31, 23, 26), 0xfa42_0322];
    let cases: Vec<(u64, u64, u64, u64)> = vec![
        (0, 0, 5, 7),
        (1, 2, 0x100, 0x80),
        (2, 2, 0x80, 0x100),
        (VNEG1, VNEG1, VMAX, VMIN),
        (0x1000, 0x800, 0x40, 0x40),
    ];
    for (x23, x26, x25, x2) in cases {
        difftest_sequence(
            "ccmp_nzcv_seq",
            &[(23, x23), (26, x26), (25, x25), (2, x2)],
            &seq,
            "    cmp x23, x26\n    ccmp x25, x2, #2, eq\n",
            &[],
            true,
        );
    }
}

#[test]
fn a64_difftest_ccmp_i_false_path_followed_by_cset() {
    // Pattern from SPEC 403.gcc:
    //   cmp  x19, #0
    //   ccmp w0, #0, #4, eq
    //   cset w5, eq
    // When cmp is not equal, CCMP must force NZCV from immediate #4 (Z=1),
    // and cset w5, eq must produce 1.
    let seq = [a64_subs_imm(1, 31, 19, 0), 0x7a40_0804, a64_csinc(0, 5, 31, 31, 1)];
    let cases: Vec<(u64, u64)> = vec![(1, 1), (1, 0), (0, 1), (0, 0)];
    for (x19, x0) in cases {
        difftest_sequence(
            "ccmp_i_false_path_followed_by_cset",
            &[(19, x19), (0, x0)],
            &seq,
            "    cmp x19, #0\n    ccmp w0, #0, #4, eq\n    cset w5, eq\n",
            &[5],
            true,
        );
    }
}

#[test]
fn a64_difftest_ccmp_after_bics_eq_gate() {
    // Pattern used by glibc strcmp hot loop:
    //   bics x4, x4, x6
    //   ccmp x2, x3, #0, eq
    //   cset w0, eq
    //
    // ccmp must consume Z from the prior logic-flags producer.
    let seq = [0xea26_0084u32, 0xfa47_0040u32, a64_csinc(0, 0, 31, 31, 1)];
    let cases: Vec<(u64, u64, u64, u64)> = vec![
        (5, 5, 0x10, 0x10), // bics Z=1, x2==x3 => 1
        (5, 7, 0x10, 0x10), // bics Z=1, x2!=x3 => 0
        (5, 5, 0x11, 0x10), // bics Z=0 => ccmp false path => 0
        (9, 9, 0x0, 0xffff_ffff_ffff_ffff), // bics Z=1 path
        (9, 8, 0x0, 0xffff_ffff_ffff_ffff),
    ];
    for (x2, x7, x4, x6) in cases {
        difftest_sequence(
            "ccmp_after_bics_eq_gate",
            &[(2, x2), (7, x7), (4, x4), (6, x6)],
            &seq,
            "    bics x4, x4, x6\n    ccmp x2, x7, #0, eq\n    cset w0, eq\n",
            &[0],
            true,
        );
    }
}

#[test]
#[ignore] // BUG: helper_sdiv64 panics on i64::MIN / -1 (Rust overflow)
fn a64_difftest_sdiv() {
    let cases: Vec<(u64, u64)> = vec![
        (100, 10),
        (VNEG1, 2),    // -1 / 2 = 0
        (VMIN, VNEG1), // MIN / -1 = MIN (overflow)
        (42, 0),       // div by zero → 0
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64("sdiv", "sdiv", a64_sdiv(1, 0, 1, 2), a, b));
    }
}

// ── 32-bit ALU difftests ─────────────────────────────────

#[test]
fn a64_difftest_add_w() {
    let cases: Vec<(u64, u64)> =
        vec![(V32MAX, V1), (V0, V0), (VNEG1, V1), (V32MIN, VNEG1)];
    for (a, b) in cases {
        difftest_alu(&rtype32("add_w", "add", a64_add_r(0, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_sub_w() {
    let cases: Vec<(u64, u64)> = vec![(V0, V1), (V32MIN, V1), (V1, V1)];
    for (a, b) in cases {
        difftest_alu(&rtype32("sub_w", "sub", a64_sub_r(0, 0, 1, 2), a, b));
    }
}
// ── Immediate ALU difftests ──────────────────────────────

#[test]
fn a64_difftest_add_imm() {
    let cases: Vec<(u64, u32)> =
        vec![(V0, 0), (V0, 4095), (VNEG1, 1), (VMAX, 1), (VPATTERN, 100)];
    for (a, imm) in cases {
        difftest_alu(&itype64(
            "add_imm",
            &format!("add x0, x1, #{imm}"),
            a64_add_imm(1, 0, 1, imm),
            a,
        ));
    }
}

#[test]
fn a64_difftest_sub_imm() {
    let cases: Vec<(u64, u32)> =
        vec![(V0, 0), (VNEG1, 1), (VMAX, 4095), (VPATTERN, 1)];
    for (a, imm) in cases {
        difftest_alu(&itype64(
            "sub_imm",
            &format!("sub x0, x1, #{imm}"),
            a64_sub_imm(1, 0, 1, imm),
            a,
        ));
    }
}

// ── Flag-setting difftests ───────────────────────────────

#[test]
fn a64_difftest_adds() {
    let cases: Vec<(u64, u64)> = vec![
        (V0, V0),      // Z=1
        (VMAX, V1),    // overflow → N=1, V=1
        (VNEG1, V1),   // C=1, Z=1
        (VMIN, VNEG1), // C=1, V=1
        (V1, V1),      // no flags
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64_s("adds", "adds", a64_adds_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_subs() {
    let cases: Vec<(u64, u64)> = vec![
        (V0, V0),      // Z=1, C=1
        (V0, V1),      // N=1
        (VMIN, V1),    // V=1
        (VMAX, VNEG1), // N=1
        (V1, V1),      // Z=1, C=1
    ];
    for (a, b) in cases {
        difftest_alu(&rtype64_s("subs", "subs", a64_subs_r(1, 0, 1, 2), a, b));
    }
}

#[test]
fn a64_difftest_adds_imm() {
    let cases: Vec<(u64, u32)> = vec![(V0, 0), (VNEG1, 1), (VMAX, 1)];
    for (a, imm) in cases {
        difftest_alu(&AluTest {
            name: "adds_imm",
            asm: format!("adds x0, x1, #{imm}"),
            insn: a64_adds_imm(1, 0, 1, imm),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: true,
        });
    }
}

#[test]
fn a64_difftest_subs_imm() {
    let cases: Vec<(u64, u32)> = vec![(V0, 0), (V0, 1), (V1, 1), (VMIN, 1)];
    for (a, imm) in cases {
        difftest_alu(&AluTest {
            name: "subs_imm",
            asm: format!("subs x0, x1, #{imm}"),
            insn: a64_subs_imm(1, 0, 1, imm),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: true,
        });
    }
}

#[test]
fn a64_difftest_add_sub_ext_w_semantics() {
    // 32-bit extended-register ops must truncate to W operands and
    // zero-extend results, while SUBS still updates NZCV from W math.
    let seq = [
        0x0b22_4020u32, // add  w0,  w1,  w2,  uxtw
        0x4b25_c08cu32, // sub  w12, w4,  w5,  sxtw
        0x2b28_40e6u32, // adds w6,  w7,  w8,  uxtw
        0x6b2b_c149u32, // subs w9,  w10, w11, sxtw
    ];
    difftest_sequence(
        "add_sub_ext_w_semantics",
        &[
            (1, 0xffff_ffff_0000_0010),
            (2, 0xffff_ffff_ffff_fff0),
            (4, 5),
            (5, 0xffff_ffff_8000_0000),
            (7, 0xffff_ffff_7fff_ffff),
            (8, 1),
            (10, 1),
            (11, 0xffff_ffff_8000_0000),
        ],
        &seq,
        "    add w0, w1, w2, uxtw\n\
         \x20   sub w12, w4, w5, sxtw\n\
         \x20   adds w6, w7, w8, uxtw\n\
         \x20   subs w9, w10, w11, sxtw\n",
        &[0, 6, 9, 12],
        true,
    );
}

#[test]
fn a64_difftest_ands() {
    let cases: Vec<(u64, u64)> = vec![
        (VNEG1, V0),    // Z=1
        (VNEG1, VNEG1), // N=1
        (VMIN, VMAX),   // Z=1
        (VPATTERN, VNEG1),
    ];
    for (a, b) in cases {
        difftest_alu(&AluTest {
            name: "ands",
            asm: "ands x0, x1, x2".to_string(),
            insn: a64_ands_r(1, 0, 1, 2),
            init: vec![(1, a), (2, b)],
            check_reg: 0,
            check_nzcv: true,
        });
    }
}

// ── Move wide difftests ──────────────────────────────────

#[test]
fn a64_difftest_movz() {
    let cases: Vec<(u32, u32)> = vec![
        (0x1234, 0),
        (0xFFFF, 0),
        (0xABCD, 1),
        (0x5678, 2),
        (0x9ABC, 3),
    ];
    for (imm16, hw) in cases {
        let expected = (imm16 as u64) << (hw * 16);
        difftest_alu(&AluTest {
            name: "movz",
            asm: format!("movz x0, #0x{imm16:x}, lsl #{}", hw * 16),
            insn: a64_movz(1, 0, imm16, hw),
            init: vec![],
            check_reg: 0,
            check_nzcv: false,
        });
        // Also verify against expected value directly
        let cpu = run_tcgrs(&[], &[a64_movz(1, 0, imm16, hw)]);
        assert_eq!(
            cpu.xregs[0], expected,
            "movz hw={hw} imm={imm16:#x}: got {:#x}",
            cpu.xregs[0]
        );
    }
}

#[test]
fn a64_difftest_movn() {
    let cases: Vec<(u32, u32)> = vec![
        (0, 0),      // ~0 = 0xFFFF_FFFF_FFFF_FFFF
        (0xFFFF, 0), // ~0xFFFF = 0xFFFF_FFFF_FFFF_0000
        (0x1234, 1),
    ];
    for (imm16, hw) in cases {
        difftest_alu(&AluTest {
            name: "movn",
            asm: format!("movn x0, #0x{imm16:x}, lsl #{}", hw * 16),
            insn: a64_movn(1, 0, imm16, hw),
            init: vec![],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_mov_w_zero_ext() {
    let cases: Vec<(u32, u32, u64)> = vec![
        (0x1234, 0, 0x0000_0000_0000_1234),
        (0xabcd, 1, 0x0000_0000_abcd_0000),
    ];
    for (imm16, hw, expected) in cases {
        difftest_sequence(
            "movz_w_zero_ext",
            &[(0, 0xffff_ffff_ffff_ffff)],
            &[a64_movz(0, 0, imm16, hw)],
            &format!("    movz w0, #0x{imm16:x}, lsl #{}\n", hw * 16),
            &[0],
            false,
        );
        let cpu = run_tcgrs(
            &[(0, 0xffff_ffff_ffff_ffff)],
            &[a64_movz(0, 0, imm16, hw)],
        );
        assert_eq!(
            cpu.xregs[0], expected,
            "movz w0 hw={hw} imm={imm16:#x} got={:#x}",
            cpu.xregs[0]
        );
    }

    difftest_sequence(
        "movk_w_zero_ext",
        &[(0, 0xffff_ffff_ffff_ffff)],
        &[a64_movk(0, 0, 0xabcd, 1)],
        "    movk w0, #0xabcd, lsl #16\n",
        &[0],
        false,
    );
    let cpu =
        run_tcgrs(&[(0, 0xffff_ffff_ffff_ffff)], &[a64_movk(0, 0, 0xabcd, 1)]);
    assert_eq!(cpu.xregs[0], 0x0000_0000_abcd_ffff);

    difftest_sequence(
        "movn_w_zero_ext",
        &[(0, 0x0123_4567_89ab_cdef)],
        &[a64_movn(0, 0, 0x0, 0)],
        "    movn w0, #0x0\n",
        &[0],
        false,
    );
    let cpu =
        run_tcgrs(&[(0, 0x0123_4567_89ab_cdef)], &[a64_movn(0, 0, 0x0, 0)]);
    assert_eq!(cpu.xregs[0], 0x0000_0000_ffff_ffff);
}

// ── CLZ / REV difftests ──────────────────────────────────

#[test]
fn a64_difftest_clz() {
    let cases: Vec<u64> =
        vec![V0, V1, VMAX, VMIN, VNEG1, 0x0000_0001_0000_0000];
    for a in cases {
        difftest_alu(&AluTest {
            name: "clz",
            asm: "clz x0, x1".to_string(),
            insn: a64_clz(1, 0, 1),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_rev() {
    let cases: Vec<u64> = vec![V0, VNEG1, 0x0102030405060708, VPATTERN];
    for a in cases {
        difftest_alu(&AluTest {
            name: "rev",
            asm: "rev x0, x1".to_string(),
            insn: a64_rev(1, 0, 1),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_rev16() {
    let cases: Vec<u64> = vec![V0, VNEG1, 0x1122334455667788, VPATTERN];
    for a in cases {
        difftest_alu(&AluTest {
            name: "rev16",
            asm: "rev16 x0, x1".to_string(),
            insn: a64_rev16(1, 0, 1),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}

#[test]
fn a64_difftest_rev32() {
    let cases: Vec<u64> = vec![V0, VNEG1, 0x1122334455667788, VPATTERN];
    for a in cases {
        difftest_alu(&AluTest {
            name: "rev32",
            asm: "rev32 x0, x1".to_string(),
            insn: a64_rev32(0, 1),
            init: vec![(1, a)],
            check_reg: 0,
            check_nzcv: false,
        });
    }
}
// ── Conditional select difftests ──────────────────────────

#[test]
fn a64_difftest_csel() {
    // CSEL depends on NZCV. We set flags via SUBS first,
    // then run CSEL. Two-instruction sequence.
    // SUBS xzr, x1, x2 (compare) then CSEL x0, x5, x6, EQ
    let subs_cmp = a64_subs_r(1, 31, 1, 2); // CMP x1, x2
    let csel_eq = a64_csel(1, 0, 5, 6, 0); // CSEL x0, x5, x6, EQ

    // Equal case: x1==x2 → select x5
    let cpu = run_tcgrs(
        &[(1, 42), (2, 42), (5, 100), (6, 200)],
        &[subs_cmp, csel_eq],
    );
    assert_eq!(cpu.xregs[0], 100, "csel eq: expected x5");

    // Not-equal case: x1!=x2 → select x6
    let cpu = run_tcgrs(
        &[(1, 42), (2, 99), (5, 100), (6, 200)],
        &[subs_cmp, csel_eq],
    );
    assert_eq!(cpu.xregs[0], 200, "csel ne: expected x6");
}

#[test]
fn a64_difftest_csinc() {
    let subs_cmp = a64_subs_r(1, 31, 1, 2);
    // CSINC x0, x5, x6, EQ → eq: x5, ne: x6+1
    let csinc_eq = a64_csinc(1, 0, 5, 6, 0);

    let cpu = run_tcgrs(
        &[(1, 10), (2, 10), (5, 100), (6, 200)],
        &[subs_cmp, csinc_eq],
    );
    assert_eq!(cpu.xregs[0], 100, "csinc eq: expected x5");

    let cpu = run_tcgrs(
        &[(1, 10), (2, 20), (5, 100), (6, 200)],
        &[subs_cmp, csinc_eq],
    );
    assert_eq!(cpu.xregs[0], 201, "csinc ne: expected x6+1");
}

// ── Branch difftests ─────────────────────────────────────

// B.cond encoding: 01010100 imm19 0 cond
fn a64_bcond(cond: u32, imm19: i32) -> u32 {
    let imm = ((imm19 >> 2) as u32) & 0x7FFFF;
    (0b01010100 << 24) | (imm << 5) | cond
}

// CBZ encoding: sf 0110100 imm19 rt
fn a64_cbz(sf: u32, rt: u32, imm19: i32) -> u32 {
    let imm = ((imm19 >> 2) as u32) & 0x7FFFF;
    (sf << 31) | (0b0110100 << 24) | (imm << 5) | rt
}

// CBNZ encoding: sf 0110101 imm19 rt
fn a64_cbnz(sf: u32, rt: u32, imm19: i32) -> u32 {
    let imm = ((imm19 >> 2) as u32) & 0x7FFFF;
    (sf << 31) | (0b0110101 << 24) | (imm << 5) | rt
}

// TBZ encoding: b5 0110110 b40 imm14 rt
fn a64_tbz(_sf: u32, rt: u32, bit: u32, imm14: i32) -> u32 {
    let imm = ((imm14 >> 2) as u32) & 0x3FFF;
    let b5 = (bit >> 5) & 1;
    let b40 = bit & 0x1F;
    (b5 << 31) | (0b0110110 << 24) | (b40 << 19) | (imm << 5) | rt
}

// TBNZ encoding: b5 0110111 b40 imm14 rt
fn a64_tbnz(_sf: u32, rt: u32, bit: u32, imm14: i32) -> u32 {
    let imm = ((imm14 >> 2) as u32) & 0x3FFF;
    let b5 = (bit >> 5) & 1;
    let b40 = bit & 0x1F;
    (b5 << 31) | (0b0110111 << 24) | (b40 << 19) | (imm << 5) | rt
}

#[test]
fn a64_difftest_cbz() {
    // CBZ x1, +16: if x1==0 → taken (pc=16), else pc=4
    let cases: Vec<(u64, bool)> =
        vec![(0, true), (1, false), (VNEG1, false), (VMIN, false)];
    for (val, expect_taken) in cases {
        let cpu = run_tcgrs(&[(1, val)], &[a64_cbz(1, 1, 16)]);
        let taken = cpu.pc == 16;
        assert_eq!(taken, expect_taken, "cbz x1={val:#x}: pc={:#x}", cpu.pc);
    }
}

#[test]
fn a64_difftest_cbnz() {
    let cases: Vec<(u64, bool)> =
        vec![(0, false), (1, true), (VNEG1, true), (VMIN, true)];
    for (val, expect_taken) in cases {
        let cpu = run_tcgrs(&[(1, val)], &[a64_cbnz(1, 1, 16)]);
        let taken = cpu.pc == 16;
        assert_eq!(taken, expect_taken, "cbnz x1={val:#x}: pc={:#x}", cpu.pc);
    }
}

#[test]
fn a64_difftest_tbz() {
    // TBZ x1, #0, +16: if bit 0 is 0 → taken
    let cases: Vec<(u64, u32, bool)> = vec![
        (0, 0, true),
        (1, 0, false),
        (2, 0, true),
        (0x8000_0000_0000_0000, 63, false),
        (0, 63, true),
    ];
    for (val, bit, expect_taken) in cases {
        let cpu = run_tcgrs(&[(1, val)], &[a64_tbz(1, 1, bit, 16)]);
        let taken = cpu.pc == 16;
        assert_eq!(
            taken, expect_taken,
            "tbz x1={val:#x} bit={bit}: pc={:#x}",
            cpu.pc
        );
    }
}

#[test]
fn a64_difftest_tbnz() {
    let cases: Vec<(u64, u32, bool)> = vec![
        (0, 0, false),
        (1, 0, true),
        (2, 1, true),
        (0x8000_0000_0000_0000, 63, true),
        (0, 63, false),
    ];
    for (val, bit, expect_taken) in cases {
        let cpu = run_tcgrs(&[(1, val)], &[a64_tbnz(1, 1, bit, 16)]);
        let taken = cpu.pc == 16;
        assert_eq!(
            taken, expect_taken,
            "tbnz x1={val:#x} bit={bit}: pc={:#x}",
            cpu.pc
        );
    }
}

#[test]
fn a64_difftest_bcond() {
    // Set flags via SUBS, then B.cond
    // B.EQ +16 (cond=0)
    let subs_cmp = a64_subs_r(1, 31, 1, 2);
    let beq = a64_bcond(0, 12); // +12 from bcond (pc=4+12=16)

    // Equal → taken
    let cpu = run_tcgrs(&[(1, 42), (2, 42)], &[subs_cmp, beq]);
    assert_eq!(cpu.pc, 16, "b.eq taken: pc={:#x}", cpu.pc);

    // Not equal → not taken
    let cpu = run_tcgrs(&[(1, 42), (2, 99)], &[subs_cmp, beq]);
    assert_eq!(cpu.pc, 8, "b.eq not taken: pc={:#x}", cpu.pc);

    // B.NE +12 (cond=1)
    let bne = a64_bcond(1, 12);
    let cpu = run_tcgrs(&[(1, 42), (2, 99)], &[subs_cmp, bne]);
    assert_eq!(cpu.pc, 16, "b.ne taken: pc={:#x}", cpu.pc);

    // B.LT +12 (cond=0b1011)
    let blt = a64_bcond(0b1011, 12);
    // -1 < 0 → taken (signed)
    let cpu = run_tcgrs(&[(1, VNEG1), (2, V0)], &[subs_cmp, blt]);
    assert_eq!(cpu.pc, 16, "b.lt taken: pc={:#x}", cpu.pc);

    // 1 < 0 → not taken
    let cpu = run_tcgrs(&[(1, V1), (2, V0)], &[subs_cmp, blt]);
    assert_eq!(cpu.pc, 8, "b.lt not taken: pc={:#x}", cpu.pc);
}

#[test]
fn a64_difftest_w_add_sub_then_cmp_x_zeroext() {
    // Branchless equivalent of the 403.gcc probe-loop pattern:
    //   add w0, w1, w2
    //   cmp x5, x0
    //   csinc w6, wzr, wzr, hi   ; x6=1 when HI is false
    //   sub w0, w0, w5
    //   mov x1, x0
    //
    // W writes must clear upper 32 bits before subsequent X users.
    let insns = [
        a64_add_r(0, 0, 1, 2),      // add w0, w1, w2
        a64_subs_r(1, 31, 5, 0),    // cmp x5, x0
        a64_csinc(0, 6, 31, 31, 8), // csinc w6, wzr, wzr, hi
        a64_sub_r(0, 0, 0, 5),      // sub w0, w0, w5
        a64_orr_r(1, 1, 0, 31),     // mov x1, x0
    ];

    difftest_sequence(
        "w_add_sub_then_cmp_x_zeroext",
        &[
            (0, 0xdead_beef_0000_0000),
            (1, 0x0000_0000_003f_ff80),
            (2, 0x0000_0000_0000_0200),
            (5, 0x0000_0000_003f_ff81),
        ],
        &insns,
        "    add w0, w1, w2\n\
         \x20   cmp x5, x0\n\
         \x20   csinc w6, wzr, wzr, hi\n\
         \x20   sub w0, w0, w5\n\
         \x20   mov x1, x0\n",
        &[0, 1, 6],
        false,
    );
}

#[test]
fn a64_difftest_bhi_fallthrough_next_tb() {
    // Same branch shape as the 403.gcc probe loop:
    // add w0,w1,w2; cmp x5,x0; b.hi skip; sub w0,w0,w5; mov x1,x0; skip:
    let insns = [
        a64_add_r(0, 0, 1, 2),
        a64_subs_r(1, 31, 5, 0),
        a64_bcond(0b1000, 8), // b.hi -> skip sub+mov
        a64_sub_r(0, 0, 0, 5),
        a64_orr_r(1, 1, 0, 31),
    ];
    let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    let mut mem = vec![0u8; 4096];
    mem[..code.len()].copy_from_slice(&code);
    let guest_base = mem.as_ptr();

    let mut cpu = Aarch64Cpu::new();
    cpu.xregs[0] = 0xdead_beef_0000_0000;
    cpu.xregs[1] = 0x0000_0000_003f_ff80;
    cpu.xregs[2] = 0x0000_0000_0000_0200;
    cpu.xregs[5] = 0x0000_0000_003f_ff81;

    // TB1: starts at pc=0, ends at B.cond exit.
    {
        let mut backend = X86_64CodeGen::new();
        let mut buf = CodeBuffer::new(4096).unwrap();
        backend.emit_prologue(&mut buf);
        backend.emit_epilogue(&mut buf);
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(0, guest_base);
        disas.base.max_insns = insns.len() as u32;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);
        unsafe {
            translate_and_execute(
                &mut ctx,
                &backend,
                &mut buf,
                &mut cpu as *mut Aarch64Cpu as *mut u8,
            );
        }
    }

    // HI should be false (x5 < x0 is false), so fallthrough to pc=12.
    assert_eq!(cpu.pc, 12, "expected B.HI fallthrough");

    // TB2: execute sub+mov at fallthrough.
    {
        let mut backend = X86_64CodeGen::new();
        let mut buf = CodeBuffer::new(4096).unwrap();
        backend.emit_prologue(&mut buf);
        backend.emit_epilogue(&mut buf);
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(12, guest_base);
        disas.base.max_insns = 4;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);
        unsafe {
            translate_and_execute(
                &mut ctx,
                &backend,
                &mut buf,
                &mut cpu as *mut Aarch64Cpu as *mut u8,
            );
        }
    }

    assert_eq!(cpu.xregs[0], 0x1ff, "sub w0,w0,w5 result");
    assert_eq!(cpu.xregs[1], 0x1ff, "mov x1,x0 result");
}

#[test]
fn a64_difftest_bhi_equal_not_taken() {
    // cmp x5, x0 with equal operands sets C=1,Z=1.
    // HI (C==1 && Z==0) must be false.
    let insns = [
        a64_subs_r(1, 31, 5, 0), // cmp x5, x0
        a64_bcond(0b1000, 8),    // b.hi skip mov
        a64_orr_r(1, 1, 0, 31),  // mov x1, x0
    ];
    let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    let mut mem = vec![0u8; 4096];
    mem[..code.len()].copy_from_slice(&code);
    let guest_base = mem.as_ptr();

    let mut cpu = Aarch64Cpu::new();
    cpu.xregs[0] = 0x942000;
    cpu.xregs[5] = 0x942000;
    cpu.xregs[1] = 0;

    // TB1: cmp + b.hi
    {
        let mut backend = X86_64CodeGen::new();
        let mut buf = CodeBuffer::new(4096).unwrap();
        backend.emit_prologue(&mut buf);
        backend.emit_epilogue(&mut buf);
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(0, guest_base);
        disas.base.max_insns = insns.len() as u32;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);
        unsafe {
            translate_and_execute(
                &mut ctx,
                &backend,
                &mut buf,
                &mut cpu as *mut Aarch64Cpu as *mut u8,
            );
        }
    }
    assert_eq!(cpu.pc, 8, "expected B.HI fallthrough on equal compare");

    // TB2: mov x1, x0
    {
        let mut backend = X86_64CodeGen::new();
        let mut buf = CodeBuffer::new(4096).unwrap();
        backend.emit_prologue(&mut buf);
        backend.emit_epilogue(&mut buf);
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(8, guest_base);
        disas.base.max_insns = 2;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);
        unsafe {
            translate_and_execute(
                &mut ctx,
                &backend,
                &mut buf,
                &mut cpu as *mut Aarch64Cpu as *mut u8,
            );
        }
    }
    assert_eq!(cpu.xregs[1], 0x942000);
}

#[test]
fn a64_difftest_fcvtzs_w_s_fixedpoint_scale12() {
    // fcvtzs w0, s1, #12
    let insn = 0x1e18_d020u32;
    let input = 1.5f32.to_bits() as u64;
    let cpu = run_tcgrs_with_state(&[], &[(1, input, 0)], &[insn]);
    assert_eq!(cpu.xregs[0], 6144);
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_scvtf_v2s_decode() {
    // scvtf v2.2s, v2.2s
    let insn = 0x0e21_d842u32;
    let src_lane0 = 4i32 as u32 as u64;
    let src_lane1 = (-8i32) as u32 as u64;
    let src = src_lane0 | (src_lane1 << 32);
    let cpu = run_tcgrs_with_state(&[], &[(2, src, 0)], &[insn]);

    let out = cpu.vregs[2 * 2];
    let want_lane0 = 4.0f32.to_bits() as u64;
    let want_lane1 = (-8.0f32).to_bits() as u64;
    let want = want_lane0 | (want_lane1 << 32);
    assert_eq!(out, want);
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_ucvtf_v2d() {
    // ucvtf v1.2d, v0.2d
    let insn = 0x6e61_d801u32;
    let src_lo = 3u64;
    let src_hi = 5u64;
    let cpu = run_tcgrs_with_state(&[], &[(0, src_lo, src_hi)], &[insn]);

    assert_eq!(cpu.vregs[1 * 2], 3.0f64.to_bits());
    assert_eq!(cpu.vregs[1 * 2 + 1], 5.0f64.to_bits());
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_scvtf_d_w_fixedpoint_scale1() {
    // scvtf d13, w1, #1
    let insn = 0x1e42_fc2du32;
    let cpu = run_tcgrs_with_state(&[(1, 3)], &[], &[insn]);

    assert_eq!(cpu.vregs[13 * 2], 1.5f64.to_bits());
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_scvtf_d_d_uses_64bit_source() {
    // scvtf d2, d1
    let insn = 0x5e61_d822u32;
    let src = 0x0000_0001_0000_0001u64;
    let cpu = run_tcgrs_with_state(&[], &[(1, src, 0)], &[insn]);

    assert_eq!(cpu.vregs[2 * 2], (src as i64 as f64).to_bits());
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_ucvtf_d_d_uses_64bit_source() {
    // ucvtf d3, d4
    let insn = 0x7e61_d883u32;
    let src = 0x0000_0002_0000_0003u64;
    let cpu = run_tcgrs_with_state(&[], &[(4, src, 0)], &[insn]);

    assert_eq!(cpu.vregs[3 * 2], (src as f64).to_bits());
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_fmul_v2s_by_element() {
    // fmul v2.2s, v2.2s, v0.s[0]
    let insn = 0x0f80_9042u32;
    let n0 = 2.0f32.to_bits() as u64;
    let n1 = (-3.0f32).to_bits() as u64;
    let n = n0 | (n1 << 32);
    let s0 = 0.5f32.to_bits() as u64;
    let m = s0;

    let cpu = run_tcgrs_with_state(&[], &[(2, n, 0), (0, m, 0)], &[insn]);

    let out = cpu.vregs[2 * 2];
    let want0 = 1.0f32.to_bits() as u64;
    let want1 = (-1.5f32).to_bits() as u64;
    let want = want0 | (want1 << 32);
    assert_eq!(out, want);
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_shl_d_imm3() {
    // shl d0, d0, #3
    let insn = 0x5f43_5400u32;
    let cpu = run_tcgrs_with_state(&[], &[(0, 5, 0)], &[insn]);

    assert_eq!(cpu.vregs[0], 40);
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_frintp_d() {
    // frintp d0, d0
    let insn = 0x1e64_c000u32;
    let in_bits = 1.25f64.to_bits();
    let cpu = run_tcgrs_with_state(&[], &[(0, in_bits, 0)], &[insn]);

    assert_eq!(cpu.vregs[0], 2.0f64.to_bits());
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_ushr_d_imm60() {
    // ushr d0, d0, #60
    let insn = 0x7f44_0400u32;
    let cpu = run_tcgrs_with_state(
        &[],
        &[(0, 0xf000_0000_0000_0000, 0)],
        &[insn],
    );

    assert_eq!(cpu.vregs[0], 0xf);
    assert_eq!(cpu.pc, 4);
}

#[test]
fn a64_difftest_cmgt_v2d() {
    // cmgt v3.2d, v3.2d, v0.2d
    // Encoding observed in perlbench pack.pl failure path at pc=0x4716e0.
    let insn = 0x4ee0_3463u32;
    let cpu = run_tcgrs_with_state(
        &[],
        &[(3, 5, (-1i64) as u64), (0, 3, 2)],
        &[insn],
    );

    assert_eq!(cpu.vregs[3 * 2], !0u64);
    assert_eq!(cpu.vregs[3 * 2 + 1], 0);
    assert_eq!(cpu.pc, 4);
}

// ── Load semantics difftests ─────────────────────────────

fn translated_qemu_ld_memops(insns: &[u32]) -> Vec<u32> {
    let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    let mut mem = vec![0u8; 4096];
    mem[..code.len()].copy_from_slice(&code);
    let guest_base = mem.as_ptr();

    let backend = X86_64CodeGen::new();
    let mut ctx = Context::new();
    backend.init_context(&mut ctx);

    let mut disas = Aarch64DisasContext::new(0, guest_base);
    disas.base.max_insns = insns.len() as u32;
    translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);

    let mut memops = Vec::new();
    for op in ctx.ops() {
        if op.opc == Opcode::QemuLd {
            memops.push(op.cargs()[0].0);
        }
    }
    memops
}

#[test]
fn a64_difftest_ldr_w_literal_zero_ext() {
    // LDR W literal must use unsigned 32-bit load semantics.
    let memops = translated_qemu_ld_memops(&[0x1800_0040]);
    assert_eq!(
        memops,
        vec![MemOp::ul().bits() as u32],
        "expected LDR W literal to use MemOp::ul"
    );
}

#[test]
fn a64_difftest_ldp_w_zero_ext() {
    // LDP W, W must use unsigned 32-bit loads for both lanes.
    let memops = translated_qemu_ld_memops(&[0x2940_0440]);
    assert_eq!(
        memops,
        vec![MemOp::ul().bits() as u32, MemOp::ul().bits() as u32],
        "expected LDP W/W to use MemOp::ul on both loads"
    );
}

#[test]
fn a64_difftest_ldp_w_pre_post_zero_ext() {
    let pre_memops = translated_qemu_ld_memops(&[0x29c1_0440]);
    assert_eq!(
        pre_memops,
        vec![MemOp::ul().bits() as u32, MemOp::ul().bits() as u32],
        "expected LDP pre-index W/W to use MemOp::ul"
    );

    let post_memops = translated_qemu_ld_memops(&[0x28c1_0440]);
    assert_eq!(
        post_memops,
        vec![MemOp::ul().bits() as u32, MemOp::ul().bits() as u32],
        "expected LDP post-index W/W to use MemOp::ul"
    );
}

#[test]
fn a64_difftest_ldrs_w_vs_x_masking() {
    fn has_mask_after_ld(insn: u32) -> bool {
        let code = insn.to_le_bytes();
        let mut mem = vec![0u8; 4096];
        mem[..4].copy_from_slice(&code);
        let guest_base = mem.as_ptr();

        let backend = X86_64CodeGen::new();
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(0, guest_base);
        disas.base.max_insns = 1;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);

        let mut seen_ld = false;
        for op in ctx.ops() {
            if op.opc == Opcode::QemuLd {
                seen_ld = true;
                continue;
            }
            if seen_ld && op.opc == Opcode::And {
                return true;
            }
        }
        false
    }

    // W destinations must insert a post-load mask to zero upper 32 bits.
    assert!(has_mask_after_ld(0x39c0_00a0)); // ldrsb w0, [x5]
    assert!(has_mask_after_ld(0x79c0_04a2)); // ldrsh w2, [x5, #2]
    assert!(has_mask_after_ld(0x38ff_c8a6)); // ldrsb w6, [x5, wzr, sxtw]
    assert!(has_mask_after_ld(0x78ea_d8a8)); // ldrsh w8, [x5, w10, sxtw #1]

    // X destinations must not mask.
    assert!(!has_mask_after_ld(0x3980_00a3)); // ldrsb x3, [x5]
    assert!(!has_mask_after_ld(0x7980_04a4)); // ldrsh x4, [x5, #2]
    assert!(!has_mask_after_ld(0x38bf_c8a7)); // ldrsb x7, [x5, wzr, sxtw]
    assert!(!has_mask_after_ld(0x78aa_d8a9)); // ldrsh x9, [x5, w10, sxtw #1]
}

#[test]
fn a64_difftest_ldr_reg_scaled_offset() {
    fn shl_consts(insns: &[u32]) -> Vec<u64> {
        let code: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
        let mut mem = vec![0u8; 4096];
        mem[..code.len()].copy_from_slice(&code);
        let guest_base = mem.as_ptr();

        let backend = X86_64CodeGen::new();
        let mut ctx = Context::new();
        backend.init_context(&mut ctx);
        let mut disas = Aarch64DisasContext::new(0, guest_base);
        disas.base.max_insns = insns.len() as u32;
        translator_loop::<Aarch64Translator>(&mut disas, &mut ctx);

        let mut out = Vec::new();
        for op in ctx.ops() {
            if op.opc != Opcode::Shl {
                continue;
            }
            let iargs = op.iargs();
            if iargs.len() != 2 {
                continue;
            }
            let sh = ctx.temp(iargs[1]);
            if sh.is_const() {
                out.push(sh.val);
            }
        }
        out
    }

    // ldr w0, [x1, x2, lsl #2]
    let shls_w = shl_consts(&[0xb862_7820u32]);
    assert!(
        shls_w.contains(&2),
        "expected scaled LDR W reg-offset to emit shl #2, got {shls_w:?}"
    );
    // ldrh w3, [x4, x5, lsl #1]
    let shls_h = shl_consts(&[0x7865_7883u32]);
    assert!(
        shls_h.contains(&1),
        "expected scaled LDRH reg-offset to emit shl #1, got {shls_h:?}"
    );
    // ldr x6, [x7, x8, lsl #3]
    let shls_x = shl_consts(&[0xf868_78e6u32]);
    assert!(
        shls_x.contains(&3),
        "expected scaled LDR X reg-offset to emit shl #3, got {shls_x:?}"
    );
}
