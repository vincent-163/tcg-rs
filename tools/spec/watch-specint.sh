#!/usr/bin/env bash
set -euo pipefail

WORKTREE=${WORKTREE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}
TAG=${1:-${TAG:-20260310a}}
ARTIFACT_DIR=${2:-${ARTIFACT_DIR:-$WORKTREE/.spec-artifacts/specint-$TAG}}
SPECINT_STATUS=${SPECINT_STATUS:-$WORKTREE/tools/spec/specint-status.sh}
SLEEP_SECS=${SLEEP_SECS:-30}

is_satisfied() {
  local state=$1
  [[ "$state" == ok || "$state" == ok+run || "$state" == ok+cmp ]]
}

all_satisfied() {
  local status=$1
  while read -r bench jit profile aot so_flag; do
    [[ "$bench" == benchmark || -z "$bench" ]] && continue
    if ! is_satisfied "$jit" || ! is_satisfied "$profile" || ! is_satisfied "$aot"; then
      return 1
    fi
  done <<<"$status"
  return 0
}

live_snapshot() {
  ps -eo pid,cmd | rg "runspec .*tcgrs\.$TAG\.(jit|profile|aot)\.cfg|specinvoke .*run_base_ref_aarch64\.Ofast\.tcgrs\.$TAG\.|tcg-aarch64 .*tcgrs\.$TAG" || true
}

last_status=
last_live=

while true; do
  now=$(date '+%F %T')
  status=$("$SPECINT_STATUS" "$TAG" "$ARTIFACT_DIR")
  live=$(live_snapshot)

  if [[ "$status" != "$last_status" ]]; then
    echo "[$now] status changed"
    echo "$status"
    echo
    last_status=$status
  fi

  if [[ "$live" != "$last_live" ]]; then
    echo "[$now] live processes changed"
    if [[ -n "$live" ]]; then
      echo "$live"
    else
      echo "(none)"
    fi
    echo
    last_live=$live
  fi

  if all_satisfied "$status"; then
    echo "[$now] all jit/profile/aot states satisfied for tag $TAG"
    exit 0
  fi

  sleep "$SLEEP_SECS"
done
