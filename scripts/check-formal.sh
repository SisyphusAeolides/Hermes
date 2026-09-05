#!/usr/bin/env sh
# Typecheck Hermes formal models (Idris2, Agda, Fortran).
# Idris2 is built from https://github.com/idris-lang/Idris2 and is normally
# available as ~/.local/bin/idris2 or another explicit IDRIS2 path.
# Default: run every available compiler, skip missing ones with a notice.
# --strict: fail if any of idris2, agda, or gfortran is missing.

set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
idris2_cmd=${IDRIS2:-idris2}
agda_cmd=${AGDA:-agda}
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

if need "$idris2_cmd"; then
  (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check HermesAuthority.idr)
  if [ -f "$ROOT/formal/idris2/NvkmGsp.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check NvkmGsp.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Cccl.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check Cccl.idr)
  fi
  if [ -f "$ROOT/formal/idris2/DrmKms.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check DrmKms.idr)
  fi
  if [ -f "$ROOT/formal/idris2/CudaStream.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check CudaStream.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Mailbox.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check Mailbox.idr)
  fi
  if [ -f "$ROOT/formal/idris2/Stage.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check Stage.idr)
  fi
  if [ -f "$ROOT/formal/idris2/HostGate.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check HostGate.idr)
  fi
  if [ -f "$ROOT/formal/idris2/DropIn.idr" ]; then
    (cd "$ROOT/formal/idris2" && "$idris2_cmd" --check DropIn.idr)
  fi
  checked=$((checked + 1))
  printf 'ok: idris2 formal models\n'
fi

if need "$agda_cmd"; then
  agda_scratch=$(mktemp -d "${TMPDIR:-/tmp}/hermes-formal.XXXXXXXX")
  trap 'find "$agda_scratch" -depth -delete 2>/dev/null || :' EXIT HUP INT TERM
  mkdir -p "$agda_scratch/src" "$agda_scratch/data" "$agda_scratch/config"
  cp "$ROOT"/formal/agda/*.agda "$agda_scratch/src/"

  # The Arch package stores Agda's primitive library under /usr/share.  Keep
  # interface caches in this private directory so a normal user can run the
  # check without modifying the system installation.
  Agda_datadir="$agda_scratch/data" "$agda_cmd" --setup >/dev/null 2>&1

  run_agda() {
    (
      cd "$agda_scratch/src"
      Agda_datadir="$agda_scratch/data" \
      XDG_DATA_HOME="$agda_scratch/data" \
      XDG_CONFIG_HOME="$agda_scratch/config" \
        "$agda_cmd" "$@"
    )
  }

  run_agda HermesWire.agda
  if [ -f "$ROOT/formal/agda/NvkmGsp.agda" ]; then
    run_agda NvkmGsp.agda
  fi
  if [ -f "$ROOT/formal/agda/Cccl.agda" ]; then
    run_agda Cccl.agda
  fi
  if [ -f "$ROOT/formal/agda/DrmKms.agda" ]; then
    run_agda DrmKms.agda
  fi
  if [ -f "$ROOT/formal/agda/CudaStream.agda" ]; then
    run_agda CudaStream.agda
  fi
  if [ -f "$ROOT/formal/agda/Mailbox.agda" ]; then
    run_agda Mailbox.agda
  fi
  if [ -f "$ROOT/formal/agda/Stage.agda" ]; then
    run_agda Stage.agda
  fi
  if [ -f "$ROOT/formal/agda/HostGate.agda" ]; then
    run_agda HostGate.agda
  fi
  if [ -f "$ROOT/formal/agda/DropIn.agda" ]; then
    run_agda DropIn.agda
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
  printf '      install Agda and gfortran from Arch, then build Idris2 from the upstream repository and re-run with --strict\n'
fi

if [ "$checked" -eq 0 ]; then
  printf 'error: no formal toolchains available\n' >&2
  exit 1
fi

printf 'formal gate finished (%s toolchain(s))\n' "$checked"
