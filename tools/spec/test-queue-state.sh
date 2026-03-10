#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

mkdir -p \
  "$TMPDIR/spec/benchspec/CPU2006/458.sjeng/run/run_base_ref_aarch64.Ofast.tcgrs.testtag.jit.0001" \
  "$TMPDIR/artifacts"

touch "$TMPDIR/spec/benchspec/CPU2006/458.sjeng/run/run_base_ref_aarch64.Ofast.tcgrs.testtag.jit.0001/compare.cmd"

cat > "$TMPDIR/fake-status.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
benchmark      jit      profile  aot      so
400.perlbench  ok       ok       run      yes
403.gcc        run      ok       -        yes
458.sjeng      compare  ok       -        yes
OUT
SH
chmod +x "$TMPDIR/fake-status.sh"

cat > "$TMPDIR/fake-rerun.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
echo "$1" >> "$QUEUE_TEST_TMPDIR/compare.calls"
SH
chmod +x "$TMPDIR/fake-rerun.sh"

cat > "$TMPDIR/fake-runspec.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "$QUEUE_TEST_TMPDIR/validate.calls"
SH
chmod +x "$TMPDIR/fake-runspec.sh"

cat > "$TMPDIR/ps.txt" <<'OUT'
CMD
/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64/bin/specinvoke -d /tmp/spec/benchspec/CPU2006/458.sjeng/run/run_base_ref_aarch64.Ofast.tcgrs.testtag.jit.0001 -e speccmds.err -o speccmds.stdout -f speccmds.cmd -C
/data/Sync/all/projects/2026-02-11-cc-work/2026-03-tcg-rs/worktrees/fix/.cargo-target/release/tcg-aarch64 ../run_base_ref_aarch64.Ofast.tcgrs.testtag.jit.0001/sjeng_base.aarch64.Ofast.tcgrs.testtag.jit ref.txt
OUT
sed -i "s#/tmp/spec#$TMPDIR/spec#g" "$TMPDIR/ps.txt"

export QUEUE_SPECINT_LIB_ONLY=1
export QUEUE_TEST_TMPDIR="$TMPDIR"
export SPECINT_STATUS="$TMPDIR/fake-status.sh"
export RERUN_COMPARE="$TMPDIR/fake-rerun.sh"
export RUN_RUNSPEC="$TMPDIR/fake-runspec.sh"
export SPEC_ROOT="$TMPDIR/spec"
export TAG=testtag
export ARTIFACT_DIR="$TMPDIR/artifacts"
export PS_CMD_FILE="$TMPDIR/ps.txt"
source "$ROOT/tools/spec/queue-specint.sh"

[[ "$(state_for 400.perlbench 2)" == "ok" ]]
[[ "$(state_for 403.gcc 2)" == "run" ]]
[[ "$(state_for 458.sjeng 2)" == "compare" ]]

live_for_bench 458.sjeng
advance_bench jit 458.sjeng "$TMPDIR/fake.cfg" 2

[[ -s "$TMPDIR/compare.calls" ]]
[[ ! -e "$TMPDIR/validate.calls" ]]

echo "queue-specint compare handling ok"
