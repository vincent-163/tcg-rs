# User Question
Try using tcg-rs profile entries for cogbt aot compilation and vice versa. Note second value of cogbt path file indicates whether the tb is exported.

## Setup
- Guest binary: `/data/Sync/all/projects/2026-02-11-cc-work/2026-03-tcg-rs/coremark-aarch64`
- Runtime args: `0x0 0x0 0x66 5000 7 1 2000`
- cogbt runner: latest aarch64 build (`cogbt-2` HEAD `10205c85b2`)
- tcg-rs runner: main (`ee1c0db`), profile-guided AOT only

## Export Bit Semantics (validated)
From symbol linkage correlation:
- `cogbt` path value `0` -> exported/global TB symbol (`T`)
- `cogbt` path value `1` -> internal/local TB symbol (`t`)

So `type=0` means exported.

## Conversion Rules Used
### A) tcg-rs profile -> cogbt path
- tcg-rs `profile.bin` entries are `(file_offset, exec_count)` with no export bit.
- Converted each entry to `guest_pc = load_vaddr + file_offset`.
- Since no export info exists in tcg-rs profile, tested:
  1) `type=0` for all entries (all exported)
  2) `type=1` for all entries (all internal, sensitivity check)

### B) cogbt path -> tcg-rs profile
- Converted each `cogbt.path.full` line `addr type` to:
  - `file_offset = addr - load_vaddr`
  - `exec_count = 2` if `type==0` (exported), else `1`
- Produced two profile variants:
  1) threshold=1 (include all entries)
  2) threshold=2 (include exported-only entries)

## Input Sizes
- tcg-rs profile entries: `426`
- cogbt path entries: `1619` (`type=0`: `313`, `type=1`: `1306`)

## Results
### Baselines (from same session)
- cogbt native full pipeline AOT: `6858.710562 it/s`
- tcg-rs native profile-AOT: `4812.319538 it/s`

### A) tcg-rs profile -> cogbt
1) all exported (`type=0`)
- AOT compile: `real 142.00s`
- AOT run: `5767.012687 it/s`
- CoreMark status: `Errors detected`

2) all internal (`type=1`)
- AOT compile: `real 85.27s`
- AOT run: `273.194186 it/s`
- CoreMark status: `Correct operation validated`

Interpretation: export bit materially affects whether hot TBs are actually used in fast path.

### B) cogbt path -> tcg-rs
1) threshold=1 (all entries)
- tcg-aot kept: `1619`
- translated: `1561` (skipped `58`)
- dispatch cases: `1619`
- AOT compile: `real 76.92s`
- AOT run: `4366.812227 it/s`
- CoreMark status: `Errors detected`

2) threshold=2 (exported-only)
- tcg-aot kept: `313`
- translated: `306` (skipped `7`)
- dispatch cases: `313`
- AOT compile: `real 3.43s`
- AOT run: `1250.000000 it/s`
- CoreMark status: `Errors detected`

## Takeaways
1. The cogbt export bit is not cosmetic; using `type=1` for all TBs collapses performance.
2. tcg-rs profile -> cogbt (all exported) retains much of cogbt speed (5767 vs 6859), but correctness failed in this arg set.
3. cogbt path -> tcg-rs all-entries runs faster than exported-only, but both were incorrect here.
4. Exported-only reverse feed drastically reduces tcg-rs compile time (76.92s -> 3.43s) but also heavily drops throughput.

## Note on Validity Banner
All 5000-iteration runs complete <10s and show CoreMark minimum-runtime warning; values are comparative throughput indicators for this experiment.
