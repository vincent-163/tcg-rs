#!/usr/bin/env bash
set -euo pipefail

WORKTREE=${WORKTREE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}
SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}
TAG=${TAG:-20260310a}
ARTIFACT_DIR=${ARTIFACT_DIR:-$WORKTREE/.spec-artifacts/specint-$TAG}
JIT_CFG=${JIT_CFG:-$SPEC_ROOT/config/aarch64.Ofast.tcgrs.$TAG.jit.cfg}
AOT_CFG=${AOT_CFG:-$WORKTREE/spec-logs/aarch64.Ofast.tcgrs.$TAG.aot.cfg}
LOG_DIR=${LOG_DIR:-$WORKTREE/spec-logs}
SPECINT_STATUS=${SPECINT_STATUS:-$WORKTREE/tools/spec/specint-status.sh}
RERUN_COMPARE=${RERUN_COMPARE:-$WORKTREE/tools/spec/rerun-compare.sh}
RUN_RUNSPEC=${RUN_RUNSPEC:-$WORKTREE/tools/spec/run-runspec-tcgrs.sh}
MAX_LIVE=${MAX_LIVE:-4}
SLEEP_SECS=${SLEEP_SECS:-20}
WAIT_SLEEP_SECS=${WAIT_SLEEP_SECS:-60}

mkdir -p "$LOG_DIR"

jit_benches=(
  400.perlbench 401.bzip2 403.gcc 429.mcf 445.gobmk 456.hmmer 458.sjeng
  462.libquantum 464.h264ref 471.omnetpp 473.astar 483.xalancbmk 999.specrand
)
aot_benches=("${jit_benches[@]}")

state_for() {
  local bench=$1
  local col=$2
  "$SPECINT_STATUS" "$TAG" "$ARTIFACT_DIR" |
    awk -v bench="$bench" -v col="$col" '$1 == bench { print $col }'
}

is_satisfied() {
  local state=$1
  [[ "$state" == ok || "$state" == ok+run || "$state" == ok+cmp ]]
}

live_total() {
  ps -eo cmd | rg -c "runspec .*tcgrs\.$TAG\.(jit|aot)\.cfg" || true
}

live_for_bench() {
  local bench=$1
  ps -eo cmd | rg -q "runspec .*tcgrs\.$TAG\.(jit|aot)\.cfg.* ${bench}"
}

latest_run_dir() {
  local bench=$1
  local mode=$2
  find "$SPEC_ROOT/benchspec/CPU2006/$bench/run" -maxdepth 1 -type d \
    -name "run_base_ref_aarch64.Ofast.tcgrs.$TAG.$mode.*" 2>/dev/null | sort | tail -n 1
}

maybe_rerun_compare() {
  local bench=$1
  local mode=$2
  local dir
  dir=$(latest_run_dir "$bench" "$mode")
  if [[ -n "$dir" && -f "$dir/compare.cmd" && ! -f "$dir/compare.out" && ! -f "$dir/compare.stdout" && ! -f "$dir/compare.rerun.stdout" ]]; then
    echo "[$(date '+%F %T')] rerun compare for $bench ($mode) in $dir"
    "$RERUN_COMPARE" "$dir"
  fi
}

run_validate() {
  local mode=$1
  local bench=$2
  local cfg=$3
  local col=2
  local log="$LOG_DIR/queue-${bench//./-}-$mode-$(date +%Y%m%d-%H%M%S).log"
  if [[ "$mode" == aot ]]; then
    col=4
  fi
  echo "[$(date '+%F %T')] start $mode $bench -> $log"
  "$RUN_RUNSPEC" \
    --noreportable --action validate --size ref --iterations 1 --config="$cfg" "$bench" \
    >"$log" 2>&1 || true
  tail -n 30 "$log" || true
  maybe_rerun_compare "$bench" "$mode" >>"$log" 2>&1 || true
  echo "[$(date '+%F %T')] done $mode $bench state=$(state_for "$bench" "$col")"
}

wait_for_slot() {
  while true; do
    local count
    count=$(live_total)
    if [[ ${count:-0} -lt $MAX_LIVE ]]; then
      return
    fi
    sleep "$WAIT_SLEEP_SECS"
  done
}

all_done() {
  local benches_name=$1
  local col=$2
  local bench
  local -n benches=$benches_name
  for bench in "${benches[@]}"; do
    if ! is_satisfied "$(state_for "$bench" "$col")"; then
      return 1
    fi
  done
  return 0
}

main() {
  echo "[$(date '+%F %T')] queue driver started"

  while ! all_done jit_benches 2; do
    local bench
    local state
    for bench in "${jit_benches[@]}"; do
      state=$(state_for "$bench" 2)
      if is_satisfied "$state"; then
        continue
      fi
      if live_for_bench "$bench"; then
        continue
      fi
      wait_for_slot
      if live_for_bench "$bench"; then
        continue
      fi
      run_validate jit "$bench" "$JIT_CFG"
      break
    done
    sleep "$SLEEP_SECS"
  done

  echo "[$(date '+%F %T')] jit phase satisfied"

  while ! all_done aot_benches 4; do
    local bench
    local state
    for bench in "${aot_benches[@]}"; do
      state=$(state_for "$bench" 4)
      if is_satisfied "$state"; then
        continue
      fi
      if live_for_bench "$bench"; then
        continue
      fi
      wait_for_slot
      if live_for_bench "$bench"; then
        continue
      fi
      run_validate aot "$bench" "$AOT_CFG"
      break
    done
    sleep "$SLEEP_SECS"
  done

  echo "[$(date '+%F %T')] queue driver finished"
}

if [[ ${QUEUE_SPECINT_LIB_ONLY:-0} == 1 ]]; then
  return 0 2>/dev/null || exit 0
fi

main "$@"
