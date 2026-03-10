#!/usr/bin/env bash
set -euo pipefail

SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}
TAG=${1:-20260310a}
ARTIFACT_DIR=${2:-$PWD/.spec-artifacts/specint-$TAG}

benches=(
  400.perlbench 401.bzip2 403.gcc 429.mcf 445.gobmk 456.hmmer 458.sjeng
  462.libquantum 464.h264ref 471.omnetpp 473.astar 483.xalancbmk 999.specrand
)

artifact_name() {
  case "$1" in
    403.gcc) echo gcc ;;
    483.xalancbmk) echo xalancbmk ;;
    *) echo "${1#*.}" ;;
  esac
}

run_dir_for() {
  local bench=$1
  local mode=$2
  find "$SPEC_ROOT/benchspec/CPU2006/$bench/run" -maxdepth 1 -type d \
    -name "run_base_ref_aarch64.Ofast.tcgrs.$TAG.$mode.*" | sort | tail -n 1
}

run_state() {
  local dir=$1
  if [[ -z "$dir" ]]; then
    echo '-'
    return
  fi
  if [[ -f "$dir/compare.out" || -f "$dir/compare.stdout" || -f "$dir/compare.rerun.stdout" ]]; then
    echo ok
    return
  fi
  if [[ -f "$dir/compare.cmd" ]]; then
    echo compare
    return
  fi
  if [[ -f "$dir/speccmds.cmd" ]]; then
    echo run
    return
  fi
  echo dir
}

printf '%-14s %-8s %-8s %-8s %-3s\n' benchmark jit profile aot so
for bench in "${benches[@]}"; do
  jit_dir=$(run_dir_for "$bench" jit || true)
  prof_dir=$(run_dir_for "$bench" profile || true)
  aot_dir=$(run_dir_for "$bench" aot || true)
  so_base=$(artifact_name "$bench")
  so_flag=no
  [[ -f "$ARTIFACT_DIR/aot/$so_base.aot.so" ]] && so_flag=yes
  printf '%-14s %-8s %-8s %-8s %-3s\n' \
    "$bench" \
    "$(run_state "${jit_dir:-}")" \
    "$(run_state "${prof_dir:-}")" \
    "$(run_state "${aot_dir:-}")" \
    "$so_flag"
done
