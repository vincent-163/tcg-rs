#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

cat > "$TMPDIR/fake-status.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
benchmark      jit      profile  aot      so
400.perlbench  ok       ok       run      yes
403.gcc        run      ok       -        yes
OUT
SH
chmod +x "$TMPDIR/fake-status.sh"

export QUEUE_SPECINT_LIB_ONLY=1
export SPECINT_STATUS="$TMPDIR/fake-status.sh"
export TAG=testtag
export ARTIFACT_DIR="$TMPDIR/artifacts"
source "$ROOT/tools/spec/queue-specint.sh"

[[ "$(state_for 400.perlbench 2)" == "ok" ]]
[[ "$(state_for 403.gcc 2)" == "run" ]]

echo "queue-specint state_for ok"
