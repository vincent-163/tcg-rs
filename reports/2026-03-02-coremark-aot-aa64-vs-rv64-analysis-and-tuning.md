# CoreMark AOT LLVM IR Comparison (AArch64 vs RISC-V) and Tuning Attempts

Date: 2026-03-02

## Scope

Goal:
- Compare optimized AOT LLVM IR for CoreMark between AArch64 and RISC-V.
- Identify likely improvement areas for AArch64 (reported ~15% slower in AOT mode).
- Implement improvements and measure impact.

## Inputs and Caveats

- Compared existing optimized IR artifacts:
  - `coremark-aarch64.o.O3.ll`
  - `coremark-riscv64.o.O3.ll`
- Measured performance in this task worktree with:
  - `coremark-aarch64` (AOT path)
  - Profile-guided AOT built from a fresh profile.
- Caveat: AArch64 and RISC-V guest CoreMark binaries in repo are not built identically
  (compiler/version/flags differ), so cross-arch absolute speed comparison is noisy.

## IR Comparison Summary (AArch64 vs RISC-V)

From `coremark-*.o.O3.ll` static analysis:

- IR size:
  - AArch64: 41,466 lines
  - RISC-V: 38,756 lines
- `tb_index` exported entries:
  - AArch64: 440
  - RISC-V: 59
- Opcode mix skew (AArch64 significantly higher):
  - `select`: 993 vs 29
  - `and`: 1977 vs 795
  - `xor`: 1080 vs 82
  - `lshr`: 1258 vs 187
- Flag/NZCV shape:
  - AArch64 TBs touching NZCV slot (offset 280): 56.4%
  - Direct NZCV-literal stores in AArch64 IR: 67
  - RISC-V has no equivalent NZCV machinery.

Interpretation:
- Major extra AArch64 IR cost is flag-condition machinery and related bit-manipulation,
  not AOT dispatch complexity (dispatch switch case counts were similar scale).

## Implemented Improvements

File changed:
- `frontend/src/aarch64/trans.rs`

### Change 1: On-demand NZCV bit extraction in `eval_cond`

Before:
- `eval_cond` always extracted all N/Z/C/V bits from packed `nzcv`, then used a subset.

After:
- Extract only the specific flag bits needed by each `base_cond` case.

### Change 2: Fold condition inversion into each case

Before:
- For odd condition codes, performed a final extra inversion compare.

After:
- Applied inversion directly in each case (`Cond::Eq`/`Cond::Ne` selection),
  removing the extra generic inversion step.

## Measurement Method

Baseline pipeline:
1. Profile:
   - `TCG_PROFILE=1 TCG_PROFILE_OUT=/tmp/aa64_baseline.prof ./target/release/tcg-aarch64 ./coremark-aarch64`
2. Build AOT:
   - `./target/release/tcg-aot /tmp/aa64_baseline.prof ./coremark-aarch64 -o /tmp/aa64_baseline.o`
   - `cc -shared -o /tmp/aa64_baseline.so /tmp/aa64_baseline.o`
3. Run AOT 3 times:
   - `TCG_AOT=/tmp/aa64_baseline.so TCG_STATS=1 ./target/release/tcg-aarch64 ./coremark-aarch64`

Post-change:
- Reused the same profile (`/tmp/aa64_baseline.prof`) for apples-to-apples AOT compilation.
- Ran 3 times for each tuning iteration.

## Results

Iterations/sec (higher is better):

- Baseline:
  - 4699.616198
  - 4734.101310
  - 4687.866240
  - Avg: **4707.194583**

- Opt1 (on-demand bit extraction only):
  - 4707.728521
  - 4651.523374
  - 4645.760743
  - Avg: **4668.337546**
  - Delta vs baseline: **-0.83%**

- Opt2 (on-demand extraction + folded inversion):
  - 4665.267087
  - 4692.265582
  - 4653.327129
  - Avg: **4670.286599**
  - Delta vs baseline: **-0.78%**

Observed exec stats were very close across runs (no significant lookup/exit-mode shift).

## IR Effect of Changes

Comparing pre-opt LLVM dumps from AOT tool (`/tmp/aa64_baseline.o.ll` vs `/tmp/aa64_opt2.o.ll`):

- Lines: 352,924 -> 347,528
- `icmp`: 1309 -> 1170
- `lshr`: 1632 -> 917
- `and`: 1625 -> 910
- `store`: 101,537 -> 99,968

So IR was simplified as intended, but it did not translate into CoreMark AOT speedup in this test.

## Conclusion

- The tested `eval_cond` micro-optimizations reduced IR complexity but did not improve
  end-to-end AArch64 AOT CoreMark throughput; measured average regressed slightly (~0.8%).
- The 15% gap is likely dominated by other hot paths (e.g. flag production patterns,
  TB shape/coverage, instruction translation strategy, or guest-binary differences).

## Next Recommended Experiments

1. Normalize guest build parity first (same CoreMark source/config/compiler level across
   AArch64 and RISC-V) before attributing gap purely to translator behavior.
2. Introduce true lazy-flags state (`cc_op/cc_a/cc_b/cc_result`) and defer packed NZCV
   materialization to explicit NZCV consumers.
3. Profile-guided TB export policy (currently always exports) and hot-TB shaping.
4. Correlate top TB runtime weight with IR complexity (e.g. TB-level sampling in AOT mode).

## Update (2026-03-02): Keep Original Coverage, Filter `exec_count > 10`

Per follow-up direction:
- Kept profile-mode coverage unchanged (no additional AOT coverage expansion logic).
- Compiled only profile TB entries where `exec_count > 10`.

Code change:
- `tools/aot/src/main.rs`
  - Added profile-entry filter: `e.exec_count > 10`.
  - Removed extra-target (`goto_tb`) coverage expansion from this path.

Build/runtime observations:
- AOT build selected `430`/`439` profile entries (dropped 9).
- AOT translated TBs: `252` (skipped `178` with helper calls).
- `aot_dispatch` cases: `430` (vs previous baseline `577`).
- Runtime loaded functions: `252` (baseline: `256`).

CoreMark AOT (3 runs, higher is better):
- Baseline (current `origin/main` profile-mode behavior):
  - 692.869740
  - 703.369781
  - 700.101833
  - Avg: **698.780451**
- `exec_count > 10` filtering:
  - 712.989370
  - 715.493691
  - 719.848177
  - Avg: **716.110413**
  - Delta vs baseline: **+2.48%**

Interpretation:
- Trimming cold profile entries improved end-to-end CoreMark throughput despite
  nearly identical translation/exit stats, likely by reducing code size / i-cache
  pressure in AOT-compiled TBs and dispatch.
