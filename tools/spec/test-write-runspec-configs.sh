#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../.." && pwd)
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/worktree/tools/spec" "$tmpdir/spec/config" "$tmpdir/out"
cp "$repo_root/tools/spec/submit-tcgrs-mode.sh" "$tmpdir/worktree/tools/spec/submit-tcgrs-mode.sh"
chmod +x "$tmpdir/worktree/tools/spec/submit-tcgrs-mode.sh"

cat > "$tmpdir/spec/config/aarch64.Ofast.tcgrs.cfg" <<'EOF'
ext           = aarch64.Ofast.cogbt
submit = OLD submit line
other = value
EOF

cat > "$tmpdir/worktree/tcg-aarch64" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$tmpdir/worktree/tcg-aarch64"

WORKTREE="$tmpdir/worktree" \
SPEC_ROOT="$tmpdir/spec" \
BASE_CFG="$tmpdir/spec/config/aarch64.Ofast.tcgrs.cfg" \
OUT_DIR="$tmpdir/out" \
ARTIFACT_DIR="$tmpdir/artifacts" \
TCG_RS="$tmpdir/worktree/tcg-aarch64" \
"$repo_root/tools/spec/write-runspec-configs.sh" testtag > "$tmpdir/generated.txt"

for mode in jit profile aot; do
    cfg="$tmpdir/out/aarch64.Ofast.tcgrs.testtag.$mode.cfg"
    [[ -f "$cfg" ]]
    rg -n "^ext           = aarch64\.Ofast\.tcgrs\.testtag\.$mode$" "$cfg" >/dev/null
    rg -n '^submit = .* \$command$' "$cfg" >/dev/null
done

rg -n 'TCG_SPEC_MODE=profile' "$tmpdir/out/aarch64.Ofast.tcgrs.testtag.profile.cfg" >/dev/null
rg -n 'TCG_SPEC_MODE=aot' "$tmpdir/out/aarch64.Ofast.tcgrs.testtag.aot.cfg" >/dev/null
! rg -n 'TCG_SPEC_MODE=' "$tmpdir/out/aarch64.Ofast.tcgrs.testtag.jit.cfg" >/dev/null

echo 'ok'
