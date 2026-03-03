# AArch64 Guest Benchmarks

This document shows how to run AArch64 guest workloads with `tcg-rs`, including CoreMark and SPEC CPU2006 assets under the `cogbt` project.

Validated with the repository state on March 2, 2026.

## 1. Build `tcg-aarch64`

From the `tcg-rs` repo root:

```bash
cargo build --release -p tcg-linux-user --bin tcg-aarch64
export TCG_A64="$PWD/target/release/tcg-aarch64"
```

## 2. CoreMark smoke test

A prebuilt static AArch64 CoreMark binary is already in this repo:

```bash
$TCG_A64 ./coremark-aarch64
```

Expected output includes:

- `Correct operation validated.`
- `CoreMark 1.0 : ...`

### 2.1 Profile all executed TBs, then AOT CoreMark

`TCG_PROFILE_MODE=all` records every executed TB (`exec_count >= 1`) into the profile.
`tcg-aot` will reuse the saved profile threshold automatically.

```bash
# 1) collect profile from all executed TBs
TCG_PROFILE=1 TCG_PROFILE_MODE=all TCG_PROFILE_OUT=/tmp/coremark-aa64-all.prof \
  $TCG_A64 ./coremark-aarch64

# 2) build AOT object from that profile
./target/release/tcg-aot /tmp/coremark-aa64-all.prof ./coremark-aarch64 \
  -o /tmp/coremark-aa64-all.o

# 3) link shared object
cc -shared -o /tmp/coremark-aa64-all.so /tmp/coremark-aa64-all.o

# 4) run with AOT enabled
TCG_AOT=/tmp/coremark-aa64-all.so TCG_STATS=1 \
  $TCG_A64 ./coremark-aarch64
```

## 3. SPEC2006 from `cogbt`

The `cogbt` project contains a SPEC2006 tree and AArch64 benchmark binaries:

- SPEC root:
  `/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64`
- submit wrapper:
  `$SPEC_ROOT/bin/submit-tcgrs.sh`
- config template:
  `$SPEC_ROOT/config/aarch64.Ofast.tcgrs.cfg`

Set:

```bash
export SPEC_ROOT=/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64
```

### 3.1 Quick direct run (no `runspec`)

```bash
$TCG_A64 "$SPEC_ROOT/cache/specrand_base.aarch64.Ofast.cogbt" 324342 24239
```

### 3.2 `runspec` integration (recommended)

1. Copy config and point `submit` to your built `tcg-aarch64`.

```bash
cp "$SPEC_ROOT/config/aarch64.Ofast.tcgrs.cfg" \
   "$SPEC_ROOT/config/aarch64.Ofast.tcgrs.local.cfg"
```

Edit this line in `aarch64.Ofast.tcgrs.local.cfg`:

```text
submit = TCG_RS=<ABSOLUTE_PATH_TO_TCG_A64> $[SPEC]/bin/submit-tcgrs.sh -- $command
```

2. Run a known-good smoke benchmark:

```bash
cd "$SPEC_ROOT"
source shrc
bin/runspec --config=aarch64.Ofast.tcgrs.local --size=test --iterations=1 999.specrand
```

`999.specrand` is the easiest correctness baseline for this flow.

### 3.3 Run a specific benchmark command manually

You can inspect per-benchmark command lines in `speccmds.cmd` and run them directly with `tcg-aarch64`.

Example (`429.mcf`, test workload):

```bash
RUN_DIR="$SPEC_ROOT/benchspec/CPU2006/429.mcf/run/run_base_test_aarch64.Ofast.cogbt.0001"
cd "$RUN_DIR"
$TCG_A64 ./mcf_base.aarch64.Ofast.cogbt inp.in > inp.out 2> inp.err
```

## 4. Current AArch64 benchmark status

- CoreMark executes successfully.
- SPEC2006 `999.specrand` test workload passes via `runspec`.
- Some larger SPEC2006 integer benchmarks still need additional frontend/runtime correctness work.
