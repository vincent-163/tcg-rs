#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
clang --target=aarch64-linux-gnu -c -fPIC -nostdlib -o vdso.o vdso.S
aarch64-linux-gnu-ld \
  -shared \
  -soname=linux-vdso.so.1 \
  -z max-page-size=4096 \
  --hash-style=both \
  --version-script=vdso.ver \
  -o linux-vdso.so.1 \
  vdso.o
rm -f vdso.o
