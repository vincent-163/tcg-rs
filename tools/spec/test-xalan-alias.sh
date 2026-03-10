#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../.." && pwd)
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

artifact_dir="$tmpdir/artifacts"
mkdir -p "$artifact_dir/meta" "$artifact_dir/profiles" "$artifact_dir/aot" "$tmpdir/bin"

printf '/bin/true\n' > "$artifact_dir/meta/Xalan.exe"
printf 'profile\n' > "$artifact_dir/profiles/Xalan.profile.bin"

cat > "$tmpdir/bin/fake-tcg-aot" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=
while [[ $# -gt 0 ]]; do
    if [[ "$1" == '-o' ]]; then
        out=$2
        shift 2
        continue
    fi
    shift
done
if [[ -z "$out" ]]; then
    echo 'missing -o' >&2
    exit 2
fi
cc -c -x c -o "$out" - <<'SRC'
int xalan_alias_test(void) { return 0; }
SRC
EOF
chmod +x "$tmpdir/bin/fake-tcg-aot"

TCG_AOT_BIN="$tmpdir/bin/fake-tcg-aot" \
    "$repo_root/tools/spec/build-spec-aot-artifacts.sh" "$artifact_dir" xalancbmk >/dev/null

if [[ ! -f "$artifact_dir/aot/Xalan.aot.so" ]]; then
    echo 'expected Xalan.aot.so to be built from xalancbmk alias' >&2
    exit 1
fi

SPEC_ROOT="$tmpdir/spec" mkdir -p "$tmpdir/spec"
mkdir -p "$tmpdir/spec/benchspec/CPU2006/483.xalancbmk/run/run_base_ref_aarch64.Ofast.tcgrs.20260310a.aot.0000"
touch "$tmpdir/spec/benchspec/CPU2006/483.xalancbmk/run/run_base_ref_aarch64.Ofast.tcgrs.20260310a.aot.0000/compare.rerun.stdout"

status_out=$(SPEC_ROOT="$tmpdir/spec" "$repo_root/tools/spec/specint-status.sh" 20260310a "$artifact_dir")
echo "$status_out" | rg '^483\.xalancbmk\s+\S+\s+\S+\s+ok\s+yes$' >/dev/null

echo 'ok'
