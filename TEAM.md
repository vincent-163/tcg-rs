# AArch64 Guest Implementation - Team Coordination

## Team Lead Session ID
Check `list_team_members` for the lead's session ID.

## Project Path
`/data/Sync/all/projects/2026-02-11-cc-work/2026-03-tcg-rs`

## Key Files
- `frontend/src/aarch64/mod.rs` — AArch64 context, NEON try_neon() fallback, helper fns
- `frontend/src/aarch64/trans.rs` — 130 scalar instruction translations (decode-driven)
- `frontend/src/aarch64/insn.decode` — Decode patterns for scalar instructions
- `frontend/src/aarch64/cpu.rs` — CPU state, register offsets
- `linux-user/src/main_aarch64.rs` — AArch64 runner entry point
- `linux-user/src/syscall_aarch64.rs` — Syscall emulation
- `core/src/ir_builder.rs` — IR builder (gen_add, gen_sub, gen_ld, gen_st, etc.)
- `core/src/opcode.rs` — 158 IR opcodes
- `backend/src/x86_64/codegen.rs` — x86-64 backend codegen

## Tools
- Cross-compiler: `aarch64-none-linux-gnu-gcc` (static builds)
- Reference emulator: `qemu-aarch64`
- TCG runner: `./target/release/tcg-aarch64`
- Disassembler: `aarch64-none-linux-gnu-objdump -d`

## Build
Cannot `cargo build` (read-only ~/.cargo). Pre-built binaries in `target/release/`.
To rebuild after code changes: `CARGO_HOME=/tmp/cargo-home cargo build --release 2>&1`

## Architecture
- NEON/FP instructions are NOT in insn.decode. They're handled by `try_neon()` in mod.rs.
- `try_neon()` matches raw opcode patterns and emits IR directly.
- 128-bit vectors stored as two I64 globals: `vregs[i]` (lo) and `vregs[32+i]` (hi).
- Helper functions (neon_cmeq8, neon_cmhs8, etc.) called via `call_helper()` for complex ops.
- Unhandled instructions trigger `EXCP_UNDEF` (exit with "illegal instruction at pc=...").

## Difftest Pattern
1. Write a small C or asm test that exercises the instruction
2. Cross-compile: `aarch64-none-linux-gnu-gcc -static -o /tmp/test /tmp/test.c`
3. Run on qemu-aarch64 to get reference output
4. Run on tcg-aarch64 and compare
5. If mismatch, debug the translation in try_neon() or trans.rs

## Current Status
- 130 scalar instructions: FULLY IMPLEMENTED (data processing, branches, loads/stores, system, etc.)
- NEON/SIMD: PARTIAL (~25 patterns in try_neon())
- Bare-metal hello world: WORKS
- printf hello world: CRASHES (libc uses NEON internally)
- coremark: CRASHES at ld1/cmeq NEON instructions

## Task Tracking

### Phase 1: Instruction Categories (Difftests + Bug Fixes)
| Category | Agent | Status |
|----------|-------|--------|
| NEON Load/Store (ld1/ld2/ld4/st1/st2/st4, LDR/STR Q/D/S) | - | pending |
| NEON Arithmetic (add/sub/mul, cmeq/cmgt/cmhs, abs/neg) | - | pending |
| NEON Logical & Bitwise (and/orr/eor/bic/bit/bif/bsl, dup/mov/ins/ext) | - | pending |
| NEON Shift & Narrow (shl/sshr/ushr, shrn/rshrn, sshll/ushll) | - | pending |
| FP Scalar (fadd/fsub/fmul/fdiv, fcmp, fcvt, fmov) | - | pending |

### Phase 2: Program Debugging
| Program | Agent | Status |
|---------|-------|--------|
| hello_printf (libc) | - | pending |
| coremark | - | pending |

### Phase 3: SPEC2006 Integer Benchmarks
| Benchmark | Agent | Status |
|-----------|-------|--------|
| 999.specrand | - | pending |
| 401.bzip2 | - | pending |
| 429.mcf | - | pending |
| 462.libquantum | - | pending |
| 458.sjeng | - | pending |
| 456.hmmer | - | pending |
| 464.h264ref | - | pending |
| 473.astar | - | pending |
| 471.omnetpp | - | pending |
| 445.gobmk | - | pending |
| 403.gcc | - | pending |
| 400.perlbench | - | pending |
| 483.xalancbmk | - | pending |
