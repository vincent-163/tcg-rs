# AArch64 Guest Translation Comparison: tcg-rs vs QEMU TCG

## Overview

This document compares how tcg-rs and QEMU translate AArch64 guest instructions to TCG IR.

**File Sizes:**
- tcg-rs: `frontend/src/aarch64/trans.rs` - 10,484 lines
- QEMU: `target/arm/tcg/translate-a64.c` - 10,429 lines

## Key Architectural Differences

### 1. **Condition Code (NZCV) Handling** (Major Difference)

#### tcg-rs: Lazy NZCV with Runtime Computation
Uses a "lazy" approach storing operation type and operands, computing flags only when needed:

```rust
// In cpu.rs - Lazy NZCV state
pub struct Aarch64Cpu {
    pub cc_op: u64,      // Operation type (CC_OP_ADD32/64, CC_OP_SUB32/64, etc.)
    pub cc_a: u64,       // First operand (or packed NZCV if CC_OP_EAGER)
    pub cc_b: u64,       // Second operand
    pub cc_result: u64,  // Operation result
}

// Compile-time tracking within TB
pub enum LazyNzcvKind {
    Add { sf: bool },
    Sub { sf: bool },
    Logic { sf: bool },
}
```

**Translation (trans.rs lines 194-251):**
```rust
fn gen_nzcv_add_sub(&mut self, ir: &mut Context, a: TempIdx, b: TempIdx, 
                    result: TempIdx, sf: bool, is_sub: bool) {
    let op_val = if is_sub { CC_OP_SUB64 } else { CC_OP_ADD64 };
    let op_c = ir.new_const(Type::I64, op_val);
    ir.gen_mov(Type::I64, self.cc_op, op_c);
    ir.gen_mov(Type::I64, self.cc_a, a64);
    ir.gen_mov(Type::I64, self.cc_b, b64);
    ir.gen_mov(Type::I64, self.cc_result, r64);
    self.lazy_nzcv = Some(LazyNzcvKind::Add { sf });
}
```

**Runtime helpers (cpu.rs lines 145-175):**
```rust
#[no_mangle]
pub extern "C" fn helper_lazy_nzcv_to_packed(...) -> u64 {
    match cc_op {
        CC_OP_EAGER => cc_a,  // Already packed
        CC_OP_ADD32 => compute_nzcv_add(cc_a, cc_b, cc_result, false),
        CC_OP_ADD64 => compute_nzcv_add(cc_a, cc_b, cc_result, true),
        CC_OP_SUB32 => compute_nzcv_sub(cc_a, cc_b, cc_result, false),
        // ... etc
    }
}
```

**Advantages:**
- Avoids computing flags when not needed
- Reduces register pressure in host code
- Smaller code size for flag-setting operations

**Disadvantages:**
- Requires helper call when flags are needed
- More complex for ADC/SBC (must materialize flags)
- Potential cache pressure from global state

#### QEMU: Eager Flag Computation with Separate Globals
Uses separate globals for each flag, computing them eagerly:

```c
// In translate-a64.c
tcg_gen_add2_i64(result, flag, t0, tmp, t1, tmp);
tcg_gen_extrl_i64_i32(cpu_NF, result);
tcg_gen_mov_i32(cpu_ZF, cpu_NF);
tcg_gen_extr_i64_i32(cpu_CF, flag);
tcg_gen_extrh_i64_i32(cpu_VF, flag);
```

**Advantages:**
- Flags ready immediately for branches
- Simpler ADC/SBC implementation
- Direct mapping to host flag registers when possible

**Disadvantages:**
- More IR instructions per flag-setting op
- Higher register pressure
- May compute unused flags

**Performance Comparison:**
- **tcg-rs better for:** Code with flag-setting ops but no flag reads (common in arithmetic-heavy code)
- **QEMU better for:** Code with frequent flag-dependent branches (common in conditional execution)

### 2. **Instruction Decoding Architecture**

#### tcg-rs: Manual Bit-Masking with Large Match
Uses explicit bit masks in a 10,000+ line decode function:

```rust
// Lines 8177-8193: ADD immediate
fn trans_ADD_i(&mut self, ir: &mut Context, a: &ArgsRriSh) -> bool {
    let sf = a.sf != 0;
    let ty = Self::sf_type(sf);
    let imm = if a.shift == 1 {
        (a.imm as u64) << 12
    } else {
        a.imm as u64
    };
    let src = self.read_xreg_sp(ir, a.rn);
    let src = Self::trunc32(ir, src, sf);
    let c = ir.new_const(ty, imm);
    let d = ir.new_temp(ty);
    ir.gen_add(ty, d, src, c);
    self.write_xreg_sp_sz(ir, a.rd, d, sf);
    true
}

// Lines 2835-2842: Floating-point conversion with manual masking
if insn & 0xffff_fc00 == 0x1e42_0000 { ... }
if insn & 0x9e42_0000 == 0x9e42_0000 { ... }
```

#### QEMU: Decoder-Generated Function Dispatch
Uses QEMU's `.decode` file to generate dispatch tables:

```c
// From a64.decode (generated decode-a64.c.inc)
// TRANS() macro generates trans_* functions

TRANS(ADD_i, gen_rri, a, 1, 1, tcg_gen_add_i64)
TRANS(SUB_i, gen_rri, a, 1, 1, tcg_gen_sub_i64)
TRANS(ADDS_i, gen_rri, a, 0, 1, a->sf ? gen_add64_CC : gen_add32_CC)
TRANS(SUBS_i, gen_rri, a, 0, 1, a->sf ? gen_sub64_CC : gen_sub32_CC)

// Lines 4543-4558: Generic immediate handler
static bool gen_rri(DisasContext *s, arg_rri_sf *a,
                    bool rd_sp, bool rn_sp, ArithTwoOp *fn) {
    TCGv_i64 tcg_rn = rn_sp ? cpu_reg_sp(s, a->rn) : cpu_reg(s, a->rn);
    TCGv_i64 tcg_rd = rd_sp ? cpu_reg_sp(s, a->rd) : cpu_reg(s, a->rd);
    TCGv_i64 tcg_imm = tcg_constant_i64(a->imm);
    fn(tcg_rd, tcg_rn, tcg_imm);
    if (!a->sf) {
        tcg_gen_ext32u_i64(tcg_rd, tcg_rd);
    }
    return true;
}
```

**Comparison:**
- **tcg-rs:** More explicit control, easier to trace, but more verbose
- **QEMU:** More maintainable, uses code generation, but harder to debug

### 3. **Helper Function Usage** (Significant Difference)

#### tcg-rs: Extensive Helper Usage (637 calls)

Uses helper functions extensively, especially for:
- Floating-point operations (almost all FP ops use helpers)
- SIMD/NEON operations
- Condition code evaluation

```rust
// Lines 1709-1716: FP fused multiply-add
gen_helper_call!(ir, d, helper_fmadd64, [n, m, a]);
gen_helper_call!(ir, d, helper_fmsub64, [n, m, a]);
gen_helper_call!(ir, d, helper_fnmadd64, [n, m, a]);
gen_helper_call!(ir, d, helper_fnmsub64, [n, m, a]);

// Lines 3038-3054: FP arithmetic - all through helpers
gen_helper_call!(ir, d, helper_fmul64, [a, b]);
gen_helper_call!(ir, d, helper_fdiv64, [a, b]);
gen_helper_call!(ir, d, helper_fadd64, [a, b]);
```

#### QEMU: Inline TCG for Simple Operations, Helpers for Complex

Uses inline TCG for simple operations, helpers only for complex ones:

```c
// Floating-point is still mostly helpers in QEMU
// But integer SIMD is often inline:

// Vector add - inline TCG
tcg_gen_add_i64(dest, src1, src2);

// Simple data movement - inline
tcg_gen_mov_i64(dest, src);

// Complex FP ops - helpers
gen_helper_vfp_addd(dest, src1, src2, fpst);
```

**Performance Impact:**
- **tcg-rs helpers:** Function call overhead for every FP op
- **QEMU inline:** More opportunities for host optimization
- **Recommendation:** Consider inlining simple operations to reduce call overhead

### 4. **Immediate Value Decoding**

#### tcg-rs: Manual Bitmask Decoding

```rust
// Lines 32-73: Manual bitmask immediate decoding
fn decode_bitmask_imm(sf: bool, n: u32, immr: u32, imms: u32) -> Option<u64> {
    let len = if n != 0 {
        6
    } else {
        let combined = !imms & 0x3f;
        if combined == 0 {
            return None;
        }
        31 - combined.leading_zeros()
    };
    // ... complex bit manipulation
}
```

#### QEMU: Shared Logic Imm Decode Function

```c
// Lines 4702-4728: Uses shared logic_imm_decode_wmask()
static bool gen_rri_log(DisasContext *s, arg_rri_log *a, bool set_cc,
                        void (*fn)(TCGv_i64, TCGv_i64, int64_t)) {
    uint64_t imm;
    if (!logic_imm_decode_wmask(&imm, extract32(a->dbm, 12, 1),
                                extract32(a->dbm, 0, 6),
                                extract32(a->dbm, 6, 6))) {
        return false;
    }
    // ...
}
```

**Note:** Both likely compute the same result; QEMU may share code with other targets.

### 5. **Memory Operations**

#### tcg-rs: Direct IR Generation
```rust
// Example pattern from trans.rs
let addr = ir.new_temp(Type::I64);
ir.gen_add(Type::I64, addr, base, offset);
ir.gen_qemu_ld(ty, data, addr, mem_idx);
```

#### QEMU: Helper-Based with Alignment Checks
```c
// From translate-a64.c - uses arm_ldst helpers
static void do_gpr_ld(DisasContext *s, int dest, int base, ...)
{
    TCGv_i64 clean_addr = gen_mte_check1(...);
    tcg_gen_qemu_ld_i64(dest, clean_addr, idx, mop);
}
```

**QEMU Advantage:** MTE (Memory Tagging Extension) support integrated

### 6. **SIMD/NEON Translation** (Major Difference)

#### tcg-rs: Almost Entirely Helper-Based
Lines 1698-2500 show extensive use of helpers for SIMD:

```rust
// Lines 1700-1760: SIMD comparisons all use helpers
gen_helper_call!(ir, d, helper_cmeq_scalar_zero, [src]);
gen_helper_call!(ir, d, helper_cmge_scalar_zero, [src]);
gen_helper_call!(ir, d, helper_cmgt_scalar_zero, [src]);

// Lines 2469-2495: Table lookup through helpers
gen_helper_call!(ir, d_lo, helper_tbl1, [t_lo, t_hi, idx_lo]);
gen_helper_call!(ir, d_lo, helper_tbl2, [t0_lo, t0_hi, t1_lo, t1_hi, idx_lo]);
```

#### QEMU: Mix of Inline TCG and Helpers

QEMU uses inline TCG for many SIMD ops:

```c
// Vector add - inline
tcg_gen_add_i64(vd, vn, vm);

// Vector shift by immediate - inline with encoding
tcg_gen_shli_i64(vd, vn, shift);

// Complex ops - helpers
gen_helper_neon_qadd_s8(vd, vn, vm, fpst);
```

**Performance Impact:**
- **tcg-rs:** Every SIMD op is a function call (high overhead)
- **QEMU:** Simple SIMD ops inline (lower overhead, better optimization)
- **Recommendation:** Implement inline SIMD for simple operations (add, sub, and, or, shifts)

## Specific Optimization Opportunities for tcg-rs

### 1. **Inline Simple SIMD Operations** (High Impact)

**Current:**
```rust
gen_helper_call!(ir, d, helper_vadd_8b, [a, b]);
```

**Optimized:**
```rust
// Generate inline TCG IR for simple ops
let d_lo = ir.new_temp(Type::I64);
let d_hi = ir.new_temp(Type::I64);
ir.gen_add(Type::I64, d_lo, a_lo, b_lo);  // Lower 64 bits
ir.gen_add(Type::I64, d_hi, a_hi, b_hi);  // Upper 64 bits
```

**Expected gain:** 20-50% for SIMD-heavy code

### 2. **Batch Flag Computations** (Medium Impact)

Current lazy NZCV requires a helper call. Consider batching multiple flag-dependent operations:

```rust
// Instead of calling helper for each branch:
// materialize_nzcv() -> helper_lazy_nzcv_to_packed()

// Consider keeping flags in registers for TB duration
// and only spill to globals at TB exit
```

### 3. **Reduce Helper Calls for FP** (Medium Impact)

Many FP operations could be inlined:

```rust
// Current - all through helpers
gen_helper_call!(ir, d, helper_fadd64, [a, b]);

// Potential - use host FP instructions directly
// (requires host backend support for FP ops)
```

### 4. **Optimize Condition Evaluation** (Low-Medium Impact)

Current approach for conditional branches:

```rust
fn eval_cond(&mut self, ir: &mut Context, cond: i64) -> TempIdx {
    self.materialize_nzcv(ir);  // Helper call
    let cond_val = ir.new_temp(Type::I64);
    gen_helper_call!(ir, cond_val, helper_lazy_nzcv_eval_cond, 
                     [self.cc_op, self.cc_a, self.cc_b, self.cc_result, cond_c]);
    cond_val
}
```

**Optimization:** For known cc_op at compile time, generate inline IR:

```rust
if let Some(lazy_kind) = self.lazy_nzcv {
    // Generate inline condition check based on lazy_kind
    match lazy_kind {
        LazyNzcvKind::Add { sf } => {
            // Generate: (result == 0) for EQ condition
            // Generate: (result >> 63) for MI condition
            // etc.
        }
        // ...
    }
} else {
    // Fall back to helper
}
```

### 5. **Use tcg_constant for Immediates** (Low Impact)

QEMU uses `tcg_constant_i64()` which may allow better host code generation:

```c
// QEMU
tcg_imm = tcg_constant_i64(a->imm);
fn(tcg_rd, tcg_rn, tcg_imm);

// tcg-rs
tcg_imm = ir.new_const(ty, imm);
ir.gen_add(ty, d, src, tcg_imm);
```

Both likely equivalent; verify LLVM backend handles constants optimally.

## Summary of Recommendations

| Priority | Optimization | Est. Impact | Effort |
|----------|-------------|-------------|--------|
| High | Inline simple SIMD ops (add, sub, and, or, xor) | 20-50% SIMD perf | Medium |
| High | Reduce FP helper calls | 10-30% FP perf | High |
| Medium | Inline condition evaluation for known cc_op | 5-15% branch perf | Medium |
| Medium | Optimize ADC/SBC with lazy flags | 5-10% crypto perf | Low |
| Low | Review constant encoding | 2-5% code density | Low |

## Key Files Reference

**tcg-rs:**
- `frontend/src/aarch64/trans.rs` - Main translation (10,484 lines)
- `frontend/src/aarch64/cpu.rs` - CPU state & lazy NZCV helpers
- `frontend/src/aarch64/mod.rs` - Translator framework

**QEMU:**
- `target/arm/tcg/translate-a64.c` - Main translation (10,429 lines)
- `target/arm/tcg/translate-a64.h` - Translator header
- `target/arm/tcg/a64.decode` - Instruction decode spec
- `target/arm/tcg/decode-a64.c.inc` - Generated decoder
