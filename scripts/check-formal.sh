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
  )
  checked=$((checked + 1))
  printf 'ok: austral HermesResources HermesRings HermesFailClosed\n'
else
  printf 'note: austral not installed; linear resource models present under formal/austral/\n'
  printf '      install from https://github.com/austral/austral then re-run with --strict\n'
fi

if [ "$checked" -eq 0 ]; then
  printf 'error: no formal toolchains available\n' >&2
  exit 1
fi

printf 'formal gate finished (%s toolchain(s))\n' "$checked"
