#!/usr/bin/env sh
# Line counts including Fortran formal sources.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

printf '=== tokei (built-in languages only) ===\n'
if command -v tokei >/dev/null 2>&1; then
  tokei crates linux/kmod formal scripts include \
    --exclude '*.mod.c' \
    --exclude 'target' \
    --exclude 'formal/fortran/build'
else
  printf 'tokei not installed\n'
fi

printf '\n=== Fortran formal (.f90) ===\n'
if [ -d formal/fortran ]; then
  find formal/fortran -type f -name '*.f90' | sort | while read -r f; do
    printf '%6s  %s\n' "$(wc -l < "$f" | tr -d ' ')" "$f"
  done
  printf '%6s  TOTAL Fortran formal\n' "$(find formal/fortran -type f -name '*.f90' -print0 | xargs -0 cat | wc -l | tr -d ' ')"
else
  printf 'formal/fortran missing\n'
  exit 1
fi

printf '\n=== Implementation sources (rs/c/h/idr/agda/f90/sh) ===\n'
find crates linux/kmod formal scripts include \
  -type f \( \
    -name '*.rs' -o -name '*.c' -o -name '*.h' -o \
    -name '*.idr' -o -name '*.agda' -o -name '*.f90' -o -name '*.sh' \
  \) ! -name '*.mod.c' ! -path '*/formal/fortran/build/*' -print0 \
  | xargs -0 wc -l | tail -1
