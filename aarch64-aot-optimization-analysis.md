# AArch64 AOT LLVM IR Optimization Analysis

**File analyzed**: `coremark-aarch64.o.O3.ll`
**Date**: 2026-03-05
**LLVM optimization level**: O3
**Total functions**: 405
**Total lines**: 34,551

## Executive Summary

The AOT-compiled LLVM IR from AArch64 guest code shows several optimization opportunities despite O3 optimization. The main issues stem from the TCG IR → LLVM IR translation strategy, which uses i64 allocas for all temps and relies on LLVM's mem2reg pass. While this simplifies translation, it introduces significant type conversion overhead and misses architecture-specific optimization opportunities.

**Key findings**:
- **1,995 type conversion operations** (878 masks, 520 truncations, 496 zero-extends, 101 sign-extends)
- **69 sign-extend-then-mask sequences** that lose sign information
- **287 add operation chains** that could use LEA-style addressing
- **1,184 inttoptr conversions** for guest memory access (expected but could be optimized)
- **Excessive GEP operations**: offset 248 computed 405 times, offset 240 computed 313 times

## Detailed Optimization Opportunities

### 1. Sign-Extend Then Mask Pattern (HIGH PRIORITY)

**Occurrences**: 69
**Impact**: Medium - wastes sign extension computation

**Pattern**:
```llvm
%21 = sext i16 %20 to i64
%22 = and i64 %21, 4294967295  ; Masks to 32 bits, losing sign info
```

**Root cause**: TCG IR represents AArch64 32-bit operations on 64-bit registers. The translator:
1. Sign-extends a 16-bit value to i64 (preserving sign)
2. Immediately masks to 32 bits (discarding upper 32 bits including sign)

**Fix**: In `backend/src/llvm/translate.rs`, detect when a sign-extended value is only used in 32-bit contexts and emit `sext i16 -> i32` followed by `zext i32 -> i64` instead. Or better, track value ranges and emit the correct extension type.

**Example fix**:
```rust
// In TbTranslator::translate_op for INDEX_op_ext16s_i64
// Check if result is only used with 32-bit mask
if self.is_only_used_as_i32(result_temp) {
    let i32_val = self.builder.build_sext(val, i32_type, "sext32");
    let i64_val = self.builder.build_zext(i32_val, i64_type, "zext64");
    self.store_temp(result_temp, i64_val);
} else {
    // Current behavior
    let i64_val = self.builder.build_sext(val, i64_type, "sext64");
    self.store_temp(result_temp, i64_val);
}
```

### 2. Excessive 32-bit Masking (HIGH PRIORITY)

**Occurrences**: 878 `and i64 %x, 4294967295`
**Impact**: High - dominates type conversion overhead

**Root cause**: AArch64 is a 64-bit architecture, but many operations work on 32-bit values (W registers). The current translation:
1. Loads all temps as i64
2. Performs operations
3. Masks back to 32 bits
4. Stores as i64

**Why LLVM can't optimize it away**: LLVM doesn't know that the upper 32 bits are semantically irrelevant for AArch64 W-register operations.

**Fix options**:

**Option A** (Recommended): Type-aware temp allocation
```rust
// In backend/src/llvm/translate.rs
enum TempType {
    I32,  // AArch64 W registers
    I64,  // AArch64 X registers
}

impl TbTranslator {
    fn get_temp_type(&self, temp: TempIdx) -> TempType {
        // Analyze TCG IR to determine if temp is always used as 32-bit
        // This requires a pre-pass over the TB
    }

    fn alloca_temp(&mut self, temp: TempIdx) -> LLVMValueRef {
        match self.get_temp_type(temp) {
            TempType::I32 => self.builder.build_alloca(i32_type, "temp"),
            TempType::I64 => self.builder.build_alloca(i64_type, "temp"),
        }
    }
}
```

**Option B**: Emit i32 operations directly when possible
```rust
// For INDEX_op_add_i32 on AArch64
let lhs = self.load_temp_as_i32(args[1]);  // Truncate if needed
let rhs = self.load_temp_as_i32(args[2]);
let result = self.builder.build_add(lhs, rhs, "add");
self.store_temp_as_i32(args[0], result);  // Zero-extend to i64
```

**Option C**: Add LLVM metadata to hint that upper bits are undefined
```llvm
%result = and i64 %val, 4294967295, !range !{i64 0, i64 4294967296}
```

### 3. Add Operation Chains (MEDIUM PRIORITY)

**Occurrences**: 287
**Impact**: Medium - missed addressing mode optimization

**Pattern**:
```llvm
%19 = add i64 %4, %1        ; guest_base + offset1
%20 = add i64 %19, %8       ; + offset2
%21 = inttoptr i64 %20 to ptr
%22 = load i64, ptr %21
```

**Root cause**: Guest memory address calculation is broken into multiple adds. x86-64 has complex addressing modes (base + index*scale + disp) that could fold these.

**Why LLVM doesn't optimize**: The inttoptr barrier prevents LLVM from seeing this as a GEP that could use addressing modes.

**Fix**: Emit GEP instead of add+inttoptr
```rust
// In translate_qemu_ld/st
// Instead of:
//   %addr = add i64 %guest_base, %offset
//   %ptr = inttoptr i64 %addr to ptr
// Emit:
//   %base_ptr = inttoptr i64 %guest_base to ptr
//   %ptr = getelementptr i8, ptr %base_ptr, i64 %offset

let base_ptr = self.builder.build_inttoptr(guest_base, ptr_type, "base");
let ptr = self.builder.build_gep(base_ptr, &[offset], "gep");
```

This allows LLVM to:
1. Fold the GEP into x86-64 addressing modes
2. Perform better alias analysis
3. Potentially eliminate redundant address calculations

### 4. Repeated GEP Calculations (MEDIUM PRIORITY)

**Occurrences**:
- `%0+248` (PC field): 405 times
- `%0+240` (flags field): 313 times
- `%0+8` (X1 register): 276 times
- `%0+288` (NZCV flags): 274 times

**Impact**: Medium - redundant address calculations

**Pattern**:
```llvm
; In every TB function:
%5 = getelementptr i8, ptr %0, i64 248  ; PC field
store i64 4199592, ptr %5, align 8
```

**Root cause**: Each TB function independently calculates CPUState field offsets. LLVM's CSE should handle this within a function, but can't optimize across function boundaries.

**Fix**: Pre-calculate common field pointers in prologue
```rust
// In backend/src/llvm/translate.rs
impl TbTranslator {
    fn emit_prologue(&mut self) {
        // Calculate common CPUState field pointers once
        self.pc_ptr = self.builder.build_gep(env_ptr, &[248], "pc_ptr");
        self.flags_ptr = self.builder.build_gep(env_ptr, &[240], "flags_ptr");
        // ... etc

        // Store in struct for use throughout translation
    }
}
```

**Alternative**: Use a struct type for CPUState instead of i8 pointer
```llvm
%CPUState = type { [31 x i64], i64, i64, ... }  ; Proper struct layout
%env = bitcast ptr %0 to ptr %CPUState
%pc_ptr = getelementptr %CPUState, ptr %env, i32 0, i32 31  ; PC field
```

This gives LLVM better type information and enables more optimizations.

### 5. IntToPtr Barrier (LOW PRIORITY - BY DESIGN)

**Occurrences**: 1,184
**Impact**: Low - necessary for guest memory access, but limits optimization

**Pattern**:
```llvm
%14 = add i64 %4, %13
%15 = inttoptr i64 %14 to ptr
%16 = load i64, ptr %15, align 4, !tbaa !4
```

**Root cause**: Guest memory is accessed via integer addresses that are converted to host pointers. This is fundamental to the design.

**Why it matters**: `inttoptr` is an optimization barrier - LLVM can't reason about pointer provenance or perform alias analysis across it.

**Current mitigation**: TBAA metadata (`!tbaa !4` for guest memory, `!tbaa !1` for CPUState) helps LLVM understand that guest memory and CPUState don't alias.

**Potential improvement**: Use LLVM's address space feature
```llvm
; Define address space 1 as guest memory
%ptr = inttoptr i64 %addr to ptr addrspace(1)
%val = load i64, ptr addrspace(1) %ptr
```

This gives LLVM explicit information that guest memory is separate from host memory, enabling better optimization. Requires LLVM backend support for address space 1.

### 6. Truncate-Extend Roundtrips (LOW PRIORITY)

**Occurrences**: 520 truncations, 496 zero-extends
**Impact**: Low - many are necessary for 32-bit operations

**Pattern**:
```llvm
%24 = trunc i64 %8 to i32
%25 = sub i32 %23, %24
%28 = zext i32 %25 to i64
```

**Root cause**: All temps are i64, but AArch64 32-bit operations require i32 operands.

**Why LLVM can't eliminate**: These are semantically necessary - 32-bit operations have different overflow behavior than 64-bit.

**Fix**: Same as issue #2 - use type-aware temp allocation to keep 32-bit values in i32 allocas.

### 7. Musttail Call Overhead (LOW PRIORITY - BY DESIGN)

**Occurrences**: 1,400 musttail calls
**Impact**: Low - necessary for TB dispatch, already optimized

**Pattern**:
```llvm
%36 = musttail call i64 @tb_14a8(ptr nonnull %0, i64 %1)
ret i64 %36
```

**Root cause**: TB-to-TB dispatch uses musttail calls to avoid stack growth.

**Why it's optimal**: `musttail` guarantees tail call optimization, making this a direct jump. This is the correct design.

**No fix needed**: This is already the best approach for AOT dispatch.

## Quantitative Impact Estimation

Based on the analysis, here's the estimated impact of each optimization:

| Issue | Occurrences | Est. Instructions Saved | Priority |
|-------|-------------|------------------------|----------|
| 32-bit masking | 878 | 878 AND instructions | HIGH |
| Sign-extend-then-mask | 69 | 69 MOVSX + 69 AND | HIGH |
| Add chains | 287 | ~200 ADD (via LEA folding) | MEDIUM |
| Repeated GEPs | ~1,500 | ~1,000 LEA (via CSE) | MEDIUM |
| Trunc/extend | 1,016 | ~300 (many necessary) | LOW |

**Total estimated savings**: ~2,500 instructions eliminated or folded into addressing modes.

For a CoreMark run with ~10M TB executions, this could translate to:
- **25 billion fewer instructions** executed
- **~5-10% performance improvement** (rough estimate)

## Recommended Implementation Plan

### Phase 1: Type-Aware Translation (Highest ROI)
1. Add a pre-pass to analyze TCG IR and determine temp types (i32 vs i64)
2. Modify `TbTranslator` to allocate i32 allocas for 32-bit temps
3. Update operation translation to emit i32 ops when both operands are i32
4. Add proper zero-extension when storing i32 results to i64 globals

**Expected impact**: Eliminates ~800 mask operations, ~500 truncations

### Phase 2: Address Calculation Optimization
1. Change guest memory access to use GEP instead of add+inttoptr
2. Pre-calculate common CPUState field pointers in prologue
3. Consider using LLVM struct types for CPUState

**Expected impact**: Eliminates ~200 add operations, ~1,000 redundant GEPs

### Phase 3: Advanced Optimizations
1. Implement address space support for guest memory
2. Add range metadata for 32-bit values
3. Investigate custom LLVM passes for TCG-specific patterns

**Expected impact**: Enables further LLVM optimizations, ~5% additional gain

## Comparison with QEMU TCG

QEMU's native TCG backend avoids many of these issues by:
1. **Direct register allocation**: TCG temps map directly to host registers, no i64 alloca overhead
2. **Type-aware codegen**: Knows when to emit 32-bit vs 64-bit x86 instructions
3. **Integrated address calculation**: Can fold guest_base + offset into x86 addressing modes
4. **No LLVM IR overhead**: Direct machine code emission

However, LLVM AOT has advantages:
1. **Advanced optimizations**: Loop unrolling, vectorization, interprocedural optimization
2. **Better register allocation**: LLVM's allocator is more sophisticated
3. **Cross-TB optimization**: Can optimize across TB boundaries (not yet implemented)

The goal is to get LLVM AOT performance close to native TCG while retaining LLVM's optimization advantages.

## Conclusion

The main optimization opportunity is **type-aware translation** - keeping 32-bit values in i32 instead of i64. This single change would eliminate ~1,300 type conversion operations (65% of total overhead).

The current translation strategy (i64 allocas + mem2reg) is simple and correct, but leaves significant performance on the table. A more sophisticated approach that tracks value types and emits appropriately-sized operations would yield substantial gains.

The good news: these are all frontend (translation) issues, not LLVM backend issues. LLVM O3 is doing a reasonable job given the IR it receives. Better IR → better code.
