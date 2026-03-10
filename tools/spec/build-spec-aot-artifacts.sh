#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: build-spec-aot-artifacts.sh <artifact-dir> [bench-name]" >&2
    exit 2
fi

artifact_dir=$(readlink -f "$1")
bench_filter=${2:-}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../.." && pwd)
meta_dir="$artifact_dir/meta"
profile_dir="$artifact_dir/profiles"
aot_dir="$artifact_dir/aot"
mkdir -p "$aot_dir"

tcg_aot_bin=${TCG_AOT_BIN:-}
if [[ -z "$tcg_aot_bin" ]]; then
    candidates=(
        "$repo_root/target/release/tcg-aot"
    )
    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        candidates+=("$(readlink -f "$CARGO_TARGET_DIR")/release/tcg-aot")
    fi
    candidates+=(
        "$repo_root/.cargo-target-llvm/release/tcg-aot"
        "$repo_root/.cargo-target-llvm-unclamped/release/tcg-aot"
        "$repo_root/.cargo-target/release/tcg-aot"
    )
    for candidate in "${candidates[@]}"; do
        if [[ -x "$candidate" ]]; then
            tcg_aot_bin=$candidate
            break
        fi
    done
fi

if [[ -z "$tcg_aot_bin" ]]; then
    echo "build-spec-aot-artifacts.sh: could not find tcg-aot; set TCG_AOT_BIN or build it first" >&2
    exit 2
fi

if [[ ! -d "$meta_dir" ]]; then
    echo "build-spec-aot-artifacts.sh: missing $meta_dir" >&2
    exit 2
fi

resolve_bench_name() {
    case "$1" in
        403.gcc|gcc) echo gcc ;;
        483.xalancbmk|xalancbmk|Xalan) echo Xalan ;;
        *.*) echo "${1#*.}" ;;
        *) echo "$1" ;;
    esac
}

build_one() {
    local bench_name=$1
    local exe_file="$meta_dir/$bench_name.exe"
    local profile_bin="$profile_dir/$bench_name.profile.bin"
    local exe
    local aot_o="$aot_dir/$bench_name.aot.o"
    local aot_so="$aot_dir/$bench_name.aot.so"

    if [[ ! -f "$exe_file" ]]; then
        echo "[aot] missing exe manifest for $bench_name" >&2
        return 1
    fi
    if [[ ! -s "$profile_bin" ]]; then
        echo "[aot] missing or empty profile for $bench_name" >&2
        return 1
    fi

    exe=$(<"$exe_file")
    echo "[aot] compiling $bench_name"
    "$tcg_aot_bin" "$profile_bin" "$exe" -o "$aot_o"
    cc -shared -o "$aot_so" "$aot_o"
}

if [[ -n "$bench_filter" ]]; then
    build_one "$(resolve_bench_name "$bench_filter")"
    exit 0
fi

status=0
while IFS= read -r exe_path; do
    bench_name=${exe_path##*/}
    bench_name=${bench_name%.exe}
    if ! build_one "$bench_name"; then
        status=1
    fi
done < <(find "$meta_dir" -maxdepth 1 -type f -name '*.exe' | sort)

exit $status
