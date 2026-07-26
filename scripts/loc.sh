#!/usr/bin/env sh
# Line counts including Austral (.aui/.aum), which stock tokei ignores.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

printf '=== tokei (built-in languages only) ===\n'
if command -v tokei >/dev/null 2>&1; then
  tokei crates linux/kmod formal scripts include \
    --exclude '*.mod.c' \
    --exclude 'target'
else
  printf 'tokei not installed\n'
fi

printf '\n=== Austral formal (.aui / .aum) — invisible to stock tokei ===\n'
if [ -d formal/austral ]; then
  find formal/austral -type f \( -name '*.aui' -o -name '*.aum' \) | sort | while read -r f; do
    printf '%6s  %s\n' "$(wc -l < "$f" | tr -d ' ')" "$f"
  done
  printf '%6s  TOTAL Austral\n' "$(find formal/austral -type f \( -name '*.aui' -o -name '*.aum' \) -print0 | xargs -0 cat | wc -l | tr -d ' ')"
else
  printf 'formal/austral missing\n'
  exit 1
fi

printf '\n=== Implementation sources (rs/c/h/idr/agda/aui/aum/sh) ===\n'
find crates linux/kmod formal scripts include \
  -type f \( \
    -name '*.rs' -o -name '*.c' -o -name '*.h' -o \
    -name '*.idr' -o -name '*.agda' -o -name '*.aui' -o -name '*.aum' -o -name '*.sh' \
  \) ! -name '*.mod.c' -print0 \
  | xargs -0 wc -l | tail -1
