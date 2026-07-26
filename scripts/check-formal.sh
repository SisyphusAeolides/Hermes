#!/usr/bin/env sh
# Typecheck Hermes formal models (Idris2, Agda, Fortran).
# Default: run every available compiler, skip missing ones with a notice.
# --strict: fail if any of idris2, agda, or gfortran is missing.

set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
STRICT=0
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT=1 ;;
    -h|--help)
      printf '%s\n' "usage: $0 [--strict]"
      exit 0
      ;;
  esac
done

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    if [ "$STRICT" -eq 1 ]; then
      printf 'error: required toolchain missing: %s\n' "$1" >&2
      exit 1
    fi
    printf 'skip: %s not installed\n' "$1"
    return 1
  fi
  return 0
}

checked=0

if need idris2; then
  (cd "$ROOT/formal/idris2" && idris2 --check HermesAuthority.idr)
  if [ -f "$ROOT/formal/idris2/NvkmGsp.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check NvkmGsp.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Cccl.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check Cccl.idr)
  fi
  if [ -f "$ROOT/formal/idris2/DrmKms.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check DrmKms.idr)
  fi
  if [ -f "$ROOT/formal/idris2/CudaStream.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check CudaStream.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Mailbox.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check Mailbox.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Stage.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check Stage.idr)
  fi
  if [ -f "$ROOT/formal/idris2/HostGate.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check HostGate.idr)
  fi
  if [ -f "$ROOT/formal/idris2/DropIn.idr" ]; then
    (cd "$ROOT/formal/idris2" && idris2 --check DropIn.idr)
  fi
  checked=$((checked + 1))
  printf 'ok: idris2 formal models\n'
fi

if need agda; then
  (cd "$ROOT/formal/agda" && agda HermesWire.agda)
  if [ -f "$ROOT/formal/agda/NvkmGsp.agda" ]; then
    (cd "$ROOT/formal/agda" && agda NvkmGsp.agda)
  fi
  if [ -f "$ROOT/formal/agda/Cccl.agda" ]; then
    (cd "$ROOT/formal/agda" && agda Cccl.agda)
  fi
  if [ -f "$ROOT/formal/agda/DrmKms.agda" ]; then
    (cd "$ROOT/formal/agda" && agda DrmKms.agda)
  fi
  if [ -f "$ROOT/formal/agda/CudaStream.agda" ]; then
    (cd "$ROOT/formal/agda" && agda CudaStream.agda)
  fi
  if [ -f "$ROOT/formal/agda/Mailbox.agda" ]; then
    (cd "$ROOT/formal/agda" && agda Mailbox.agda)
  fi
  if [ -f "$ROOT/formal/agda/Stage.agda" ]; then
    (cd "$ROOT/formal/agda" && agda Stage.agda)
  fi
  if [ -f "$ROOT/formal/agda/HostGate.agda" ]; then
    (cd "$ROOT/formal/agda" && agda HostGate.agda)
  fi
  if [ -f "$ROOT/formal/agda/DropIn.agda" ]; then
    (cd "$ROOT/formal/agda" && agda DropIn.agda)
  fi
  checked=$((checked + 1))
  printf 'ok: agda formal models\n'
fi

if need gfortran; then
  make -C "$ROOT/formal/fortran" check
  checked=$((checked + 1))
  printf 'ok: fortran formal modules\n'
else
  n=$(find "$ROOT/formal/fortran" -name '*.f90' 2>/dev/null | wc -l | tr -d ' ')
  lines=$(find "$ROOT/formal/fortran" -type f -name '*.f90' -print0 2>/dev/null | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
  printf 'note: gfortran not installed; %s modules / %s lines under formal/fortran/\n' "$n" "$lines"
  printf '      install gcc-gfortran (Fedora) or gfortran, then re-run with --strict\n'
fi

if [ "$checked" -eq 0 ]; then
  printf 'error: no formal toolchains available\n' >&2
  exit 1
fi

printf 'formal gate finished (%s toolchain(s))\n' "$checked"
