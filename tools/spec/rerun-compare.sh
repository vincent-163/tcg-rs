#!/usr/bin/env bash
set -euo pipefail

SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}

if [[ $# -ne 1 ]]; then
    echo "usage: rerun-compare.sh <run-dir>" >&2
    exit 2
fi

run_dir=$1
if [[ ! -d "$run_dir" ]]; then
    echo "rerun-compare.sh: missing run dir '$run_dir'" >&2
    exit 2
fi

run_dir=$(readlink -f "$run_dir")
if [[ ! -f "$run_dir/compare.cmd" ]]; then
    echo "rerun-compare.sh: missing $run_dir/compare.cmd" >&2
    exit 2
fi
if [[ ! -f "$SPEC_ROOT/shrc" || ! -x "$SPEC_ROOT/bin/specinvoke" ]]; then
    echo "rerun-compare.sh: invalid SPEC_ROOT '$SPEC_ROOT'" >&2
    exit 2
fi

cd "$SPEC_ROOT"
# shellcheck disable=SC1091
source ./shrc >/dev/null
"$SPEC_ROOT/bin/specinvoke" -E -d "$run_dir" -c 1 \
    -e compare.rerun.err -o compare.rerun.stdout -f compare.cmd
