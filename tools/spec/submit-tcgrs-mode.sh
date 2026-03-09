#!/usr/bin/env bash
set -euo pipefail

TCG_RS=${TCG_RS:-tcg-aarch64}
TCG_SPEC_MODE=${TCG_SPEC_MODE:-jit}
TCG_SPEC_ARTIFACT_DIR=${TCG_SPEC_ARTIFACT_DIR:-}

if [[ "${1:-}" == "--" ]]; then
    shift
fi

if [[ $# -lt 1 ]]; then
    echo "submit-tcgrs-mode.sh: missing benchmark command" >&2
    exit 2
fi

exe=$1
shift

bench_base=$(basename "$exe")
bench_name=${bench_base%%_base*}

perl_case=
for arg in "$@"; do
    if [[ "$arg" == *.pl ]]; then
        perl_case=$arg
        break
    fi
done

if [[ "$TCG_SPEC_MODE" == "jit" ]]; then
    exec "$TCG_RS" "$exe" "$@"
fi

if [[ -z "$TCG_SPEC_ARTIFACT_DIR" ]]; then
    echo "submit-tcgrs-mode.sh: TCG_SPEC_ARTIFACT_DIR is required for mode '$TCG_SPEC_MODE'" >&2
    exit 2
fi

artifact_dir=$(readlink -f "$TCG_SPEC_ARTIFACT_DIR")
profile_dir="$artifact_dir/profiles"
aot_dir="$artifact_dir/aot"
meta_dir="$artifact_dir/meta"
mkdir -p "$profile_dir" "$aot_dir" "$meta_dir"
readlink -f "$exe" > "$meta_dir/$bench_name.exe"

case "$TCG_SPEC_MODE" in
    profile)
        export TCG_PROFILE=1
        export TCG_PROFILE_MODE=${TCG_PROFILE_MODE:-all}
        export TCG_PROFILE_OUT="$profile_dir/$bench_name.profile.bin"
        exec "$TCG_RS" "$exe" "$@"
        ;;
    aot)
        aot_so="$aot_dir/$bench_name.aot.so"
        if [[ ! -f "$aot_so" ]]; then
            echo "submit-tcgrs-mode.sh: missing AOT library $aot_so" >&2
            exit 2
        fi
        export TCG_AOT="$aot_so"
        exec "$TCG_RS" "$exe" "$@"
        ;;
    *)
        echo "submit-tcgrs-mode.sh: unknown TCG_SPEC_MODE '$TCG_SPEC_MODE'" >&2
        exit 2
        ;;
esac
