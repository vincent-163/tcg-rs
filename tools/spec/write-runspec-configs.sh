#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'EOF'
usage: write-runspec-configs.sh <tag>

Environment:
  WORKTREE      tcg-rs worktree root (default: repo root)
  SPEC_ROOT     SPEC CPU2006 root
  BASE_CFG      base tcg-rs SPEC config template
  OUT_DIR       output directory for generated configs
  ARTIFACT_DIR  profile/AOT artifact directory
  TCG_RS        tcg-aarch64 binary path

Writes:
  <OUT_DIR>/aarch64.Ofast.tcgrs.<tag>.jit.cfg
  <OUT_DIR>/aarch64.Ofast.tcgrs.<tag>.profile.cfg
  <OUT_DIR>/aarch64.Ofast.tcgrs.<tag>.aot.cfg
EOF
    exit 2
}

if [[ $# -ne 1 ]]; then
    usage
fi

TAG=$1
WORKTREE=${WORKTREE:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)}
SPEC_ROOT=${SPEC_ROOT:-/data/Sync/all/projects/2026-02-11-cc-work/2025-08-13-cogbt/cpu2006v11_run_x64_Ofast_ld64}
BASE_CFG=${BASE_CFG:-$SPEC_ROOT/config/aarch64.Ofast.tcgrs.cfg}
OUT_DIR=${OUT_DIR:-$WORKTREE/spec-logs}
ARTIFACT_DIR=${ARTIFACT_DIR:-$WORKTREE/.spec-artifacts/specint-$TAG}

find_tcg_rs() {
    local candidates=()

    if [[ -n ${TCG_RS:-} ]]; then
        candidates+=("$TCG_RS")
    fi

    candidates+=(
        "$WORKTREE/.cargo-target-llvm/release/tcg-aarch64"
        "$WORKTREE/target/release/tcg-aarch64"
    )

    if [[ -n ${CARGO_TARGET_DIR:-} ]]; then
        candidates+=("$(readlink -f "$CARGO_TARGET_DIR")/release/tcg-aarch64")
    fi

    local candidate
    for candidate in "${candidates[@]}"; do
        if [[ -x "$candidate" ]]; then
            readlink -f "$candidate"
            return 0
        fi
    done

    echo "write-runspec-configs.sh: could not find tcg-aarch64; set TCG_RS or build it first" >&2
    exit 2
}

if [[ ! -f "$BASE_CFG" ]]; then
    echo "write-runspec-configs.sh: missing base config $BASE_CFG" >&2
    exit 2
fi

mkdir -p "$OUT_DIR" "$ARTIFACT_DIR"

tcg_rs=$(find_tcg_rs)
submit_wrapper=$(readlink -f "$WORKTREE/tools/spec/submit-tcgrs-mode.sh")

write_cfg() {
    local mode=$1
    local cfg="$OUT_DIR/aarch64.Ofast.tcgrs.$TAG.$mode.cfg"
    local submit_line

    case "$mode" in
        jit)
            submit_line="submit = TCG_RS=$tcg_rs $submit_wrapper -- "'$command'
            ;;
        profile)
            submit_line="submit = TCG_RS=$tcg_rs TCG_SPEC_MODE=profile TCG_SPEC_ARTIFACT_DIR=$ARTIFACT_DIR $submit_wrapper -- "'$command'
            ;;
        aot)
            submit_line="submit = TCG_RS=$tcg_rs TCG_SPEC_MODE=aot TCG_SPEC_ARTIFACT_DIR=$ARTIFACT_DIR $submit_wrapper -- "'$command'
            ;;
        *)
            echo "write-runspec-configs.sh: unknown mode $mode" >&2
            exit 2
            ;;
    esac

    awk \
        -v ext="aarch64.Ofast.tcgrs.$TAG.$mode" \
        -v submit_line="$submit_line" \
        'BEGIN { replaced_submit = 0 }
         /^ext[[:space:]]*=/ { print "ext           = " ext; next }
         /^submit = / { print submit_line; replaced_submit = 1; next }
         { print }
         END { if (!replaced_submit) print submit_line }' \
        "$BASE_CFG" > "$cfg"

    echo "$cfg"
}

write_cfg jit
write_cfg profile
write_cfg aot

