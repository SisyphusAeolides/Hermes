#!/usr/bin/env sh
# Typecheck Hermes formal models (Idris2, Agda, Austral).
# Default: run every available compiler, skip missing ones with a notice.
# --strict: fail if any of idris2, agda, or austral is missing.

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
  checked=$((checked + 1))
  printf 'ok: idris2 HermesAuthority\n'
fi

if need agda; then
  (cd "$ROOT/formal/agda" && agda HermesWire.agda)
  checked=$((checked + 1))
  printf 'ok: agda HermesWire\n'
fi

if need austral; then
  (
    cd "$ROOT/formal/austral"
    austral compile --target-type=tc HermesResources.aui,HermesResources.aum
    austral compile --target-type=tc HermesRings.aui,HermesRings.aum
    austral compile --target-type=tc HermesFailClosed.aui,HermesFailClosed.aum
    austral compile --target-type=tc HermesWpr.aui,HermesWpr.aum
    austral compile --target-type=tc HermesBootstrap.aui,HermesBootstrap.aum
    austral compile --target-type=tc HermesFirmware.aui,HermesFirmware.aum
  )
  checked=$((checked + 1))
  printf 'ok: austral (6 modules)\n'
else
  n=$(find "$ROOT/formal/austral" -name '*.aui' 2>/dev/null | wc -l | tr -d ' ')
  lines=$(find "$ROOT/formal/austral" -type f \( -name '*.aui' -o -name '*.aum' \) -print0 2>/dev/null | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
  printf 'note: austral not installed; %s modules / %s lines under formal/austral/\n' "$n" "$lines"
  printf '      (stock tokei ignores .aui/.aum — run scripts/loc.sh)\n'
  printf '      install from https://github.com/austral/austral then re-run with --strict\n'
fi

if [ "$checked" -eq 0 ]; then
  printf 'error: no formal toolchains available\n' >&2
  exit 1
fi

printf 'formal gate finished (%s toolchain(s))\n' "$checked"
