# SPEC2006int AOT Benchmark Results

**Date:** 2026-03-07
**Script:** `run-spec2006int-aot.sh`
**Mode:** Parallel execution with LLVM JIT backend (`TCG_LLVM=1`)
**Results Directory:** `spec2006int-results-20260307-113717/`

## Summary

**5 out of 13 benchmarks PASSED** through the complete 3-phase AOT pipeline (profile → compile → run).

## Successful Benchmarks (PASS)

| Benchmark | Profile Time | AOT Compile | AOT Run | Total Time |
|-----------|--------------|-------------|---------|------------|
| 401.bzip2 | 61s | 1s | 45s | 107s |
| 999.specrand | 82s | 16s | 22s | 120s |
| 458.sjeng | 60s | 27s | 38s | 125s |
| 445.gobmk | 69s | 98s | 45s | 212s |
| 471.omnetpp | 122s | 6s | 90s | 218s |

## Failed Benchmarks

### Profile Phase Failures

| Benchmark | Profile Time | Failure Reason |
|-----------|--------------|----------------|
| 473.astar | 41s | SIGSEGV crash |
| 429.mcf | 44s | File I/O error ("read error, exit") |
| 456.hmmer | 52s | Non-zero exit code |
| 464.h264ref | 58s | Config parsing error ("Expected numerical value") |
| 462.libquantum | 66s | glibc malloc assertion failure |
| 400.perlbench | 94s | Working directory issue ("Can't open perl script") |
| 403.gcc | 131s | glibc malloc assertion failure |

### Other Errors

| Benchmark | Error |
|-----------|-------|
| 483.xalancbmk | Executable not found (wrong filename) |

## Key Findings

1. **LLVM JIT Backend Stability**: Using `TCG_LLVM=1` significantly improved stability compared to the x86-64 backend (5 passing vs 1 without LLVM).

2. **Parallel Execution**: All 13 benchmarks ran in parallel successfully, utilizing available CPU cores efficiently.

3. **Common Failure Patterns**:
   - **Memory corruption**: libquantum and gcc both hit glibc malloc assertions
   - **File I/O issues**: mcf, perlbench, h264ref had file/config reading problems
   - **Crashes**: astar hit SIGSEGV during execution

4. **AOT Compilation Performance**:
   - Fastest AOT compile: bzip2 (1s)
   - Slowest AOT compile: gobmk (98s)
   - Average AOT compile time: ~30s

5. **Total Execution Time**:
   - Fastest benchmark: bzip2 (107s)
   - Slowest benchmark: omnetpp (218s)

## Script Usage

```bash
# Run all benchmarks in parallel (default)
./run-spec2006int-aot.sh parallel

# Run all benchmarks serially
./run-spec2006int-aot.sh serial
```

## Output Structure

```
spec2006int-results-<timestamp>/
├── logs/           # Individual benchmark logs
├── profiles/       # Profile data (.profile.bin files)
├── aot/           # AOT compiled objects (.aot.o, .aot.so)
└── summary.csv    # Results summary
```

## Next Steps

To improve the pass rate, the following issues need to be addressed in tcg-rs:

1. Fix SIGSEGV crashes (astar)
2. Fix file I/O handling (mcf, perlbench, h264ref)
3. Fix memory corruption issues (libquantum, gcc)
4. Fix xalancbmk executable path
