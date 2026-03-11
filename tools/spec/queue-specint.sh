#!/usr/bin/env bash
set -euo pipefail

WORKTREE=${WORKTREE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}
SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}
TAG=${TAG:-20260310a}
ARTIFACT_DIR=${ARTIFACT_DIR:-$WORKTREE/.spec-artifacts/specint-$TAG}
JIT_CFG=${JIT_CFG:-$SPEC_ROOT/config/aarch64.Ofast.tcgrs.$TAG.jit.cfg}
PROFILE_CFG=${PROFILE_CFG:-$WORKTREE/spec-logs/aarch64.Ofast.tcgrs.$TAG.profile.cfg}
AOT_CFG=${AOT_CFG:-$WORKTREE/spec-logs/aarch64.Ofast.tcgrs.$TAG.aot.cfg}
BUILD_AOT=${BUILD_AOT:-$WORKTREE/tools/spec/build-spec-aot-artifacts.sh}
LOG_DIR=${LOG_DIR:-$WORKTREE/spec-logs}
SPECINT_STATUS=${SPECINT_STATUS:-$WORKTREE/tools/spec/specint-status.sh}
RERUN_COMPARE=${RERUN_COMPARE:-$WORKTREE/tools/spec/rerun-compare.sh}
RUN_RUNSPEC=${RUN_RUNSPEC:-$WORKTREE/tools/spec/run-runspec-tcgrs.sh}
MAX_LIVE=${MAX_LIVE:-1}
SLEEP_SECS=${SLEEP_SECS:-20}
WAIT_SLEEP_SECS=${WAIT_SLEEP_SECS:-60}

mkdir -p "$LOG_DIR"

jit_benches=(
  400.perlbench 401.bzip2 403.gcc 429.mcf 445.gobmk 456.hmmer 458.sjeng
  462.libquantum 464.h264ref 471.omnetpp 473.astar 483.xalancbmk 999.specrand
)
profile_benches=("${jit_benches[@]}")
aot_benches=("${jit_benches[@]}")

ps_cmd() {
  if [[ -n ${PS_CMD_FILE:-} ]]; then
    cat "$PS_CMD_FILE"
  else
    ps -eo cmd
  fi
}

artifact_name() {
  case "$1" in
    403.gcc) echo gcc ;;
    483.xalancbmk) echo Xalan ;;
    *.*) echo "${1#*.}" ;;
    *) echo "$1" ;;
  esac
}

aot_so_for() {
  local bench=$1
  echo "$ARTIFACT_DIR/aot/$(artifact_name "$bench").aot.so"
}

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
  local snapshot
  snapshot=$(ps_cmd)
  grep -Ec "runspec .*tcgrs\.$TAG\.(jit|profile|aot)\.cfg" <<<"$snapshot" || true
}

latest_run_dir() {
  local bench=$1
  local mode=$2
  find "$SPEC_ROOT/benchspec/CPU2006/$bench/run" -maxdepth 1 -type d \
    -name "run_base_ref_aarch64.Ofast.tcgrs.$TAG.$mode.*" 2>/dev/null |
    sort | tail -n 1
}

live_for_bench() {
  local bench=$1
  local mode
  local dir
  local run_name
  local snapshot

  snapshot=$(ps_cmd)
  if grep -Eq "runspec .*tcgrs\.$TAG\.(jit|profile|aot)\.cfg.* ${bench}( |$)" <<<"$snapshot"; then
    return 0
  fi

  for mode in jit profile aot; do
    dir=$(latest_run_dir "$bench" "$mode")
    if [[ -z "$dir" ]]; then
      continue
    fi
    run_name=$(basename "$dir")
    if grep -Fq -- "$dir" <<<"$snapshot" || grep -Fq -- "$run_name" <<<"$snapshot"; then
      return 0
    fi
  done

  return 1
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
  local col
  local log="$LOG_DIR/queue-${bench//./-}-$mode-$(date +%Y%m%d-%H%M%S).log"

  case "$mode" in
    jit) col=2 ;;
    profile) col=3 ;;
    aot) col=4 ;;
    *) echo "queue-specint.sh: unknown mode $mode" >&2; return 2 ;;
  esac

  echo "[$(date '+%F %T')] start $mode $bench -> $log"
  "$RUN_RUNSPEC" \
    --noreportable --action validate --size ref --iterations 1 \
    --config="$cfg" "$bench" >"$log" 2>&1 || true
  tail -n 30 "$log" || true
  maybe_rerun_compare "$bench" "$mode" >>"$log" 2>&1 || true
  echo "[$(date '+%F %T')] done $mode $bench state=$(state_for "$bench" "$col")"
}

build_aot_for_bench() {
  local bench=$1
  local so
  local log

  so=$(aot_so_for "$bench")
  if [[ -f "$so" ]]; then
    return 1
  fi
  if ! is_satisfied "$(state_for "$bench" 3)"; then
    return 1
  fi
  if live_for_bench "$bench"; then
    return 1
  fi

  log="$LOG_DIR/build-aot-${bench//./-}-$(date +%Y%m%d-%H%M%S).log"
  echo "[$(date '+%F %T')] build aot $bench -> $log"
  "$BUILD_AOT" "$ARTIFACT_DIR" "$bench" >"$log" 2>&1 || true
  tail -n 30 "$log" || true
  echo "[$(date '+%F %T')] done build aot $bench so=$([[ -f "$so" ]] && echo yes || echo no)"
  return 0
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

advance_bench() {
  local mode=$1
  local bench=$2
  local cfg=$3
  local col=$4
  local state

  state=$(state_for "$bench" "$col")
  if is_satisfied "$state"; then
    return 1
  fi

  if [[ "$state" == compare ]]; then
    maybe_rerun_compare "$bench" "$mode"
    return 0
  fi

  if live_for_bench "$bench"; then
    return 1
  fi

  wait_for_slot
  if live_for_bench "$bench"; then
    return 1
  fi

  run_validate "$mode" "$bench" "$cfg"
  return 0
}

advance_aot_bench() {
  local bench=$1
  local cfg=$2
  local so

  if is_satisfied "$(state_for "$bench" 4)"; then
    return 1
  fi

  so=$(aot_so_for "$bench")
  if [[ ! -f "$so" ]]; then
    if build_aot_for_bench "$bench"; then
      return 0
    fi
    return 1
  fi

  advance_bench aot "$bench" "$cfg" 4
}

main() {
  echo "[$(date '+%F %T')] queue driver started"

  while ! all_done jit_benches 2; do
    local bench
    for bench in "${jit_benches[@]}"; do
      if advance_bench jit "$bench" "$JIT_CFG" 2; then
        break
      fi
    done
    sleep "$SLEEP_SECS"
  done

  echo "[$(date '+%F %T')] jit phase satisfied"

  while ! all_done profile_benches 3; do
    local bench
    for bench in "${profile_benches[@]}"; do
      if advance_bench profile "$bench" "$PROFILE_CFG" 3; then
        break
      fi
    done
    sleep "$SLEEP_SECS"
  done

  echo "[$(date '+%F %T')] profile phase satisfied"

  while ! all_done aot_benches 4; do
    local bench
    for bench in "${aot_benches[@]}"; do
      if advance_aot_bench "$bench" "$AOT_CFG"; then
        break
      fi
    done
    sleep "$SLEEP_SECS"
  done

  echo "[$(date '+%F %T')] queue driver finished"
}

if [[ ${QUEUE_SPECINT_LIB_ONLY:-0} == 1 ]]; then
  return 0 2>/dev/null || exit 0
fi

main "$@"
