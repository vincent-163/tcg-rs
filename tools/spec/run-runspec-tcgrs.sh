#!/usr/bin/env bash
set -euo pipefail

SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}

if [[ $# -lt 1 ]]; then
    echo "usage: run-runspec-tcgrs.sh <runspec args...>" >&2
    echo "example: run-runspec-tcgrs.sh --config=/tmp/aarch64.Ofast.tcgrs.20260310a.jit.cfg --size=ref --iterations=1 445.gobmk" >&2
    exit 2
fi

if [[ ! -f "$SPEC_ROOT/shrc" || ! -x "$SPEC_ROOT/bin/runspec" ]]; then
    echo "run-runspec-tcgrs.sh: invalid SPEC_ROOT '$SPEC_ROOT'" >&2
    exit 2
fi

cd "$SPEC_ROOT"
# shellcheck disable=SC1091
source ./shrc >/dev/null
exec "$SPEC_ROOT/bin/runspec" "$@"
