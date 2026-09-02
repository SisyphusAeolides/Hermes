#!/usr/bin/env bash
# Produce the Hermes release qualification contract.
#
# This is deliberately a gate, not a best-effort smoke test.  A green cargo
# build, a simulation, or a loadable kernel module is not evidence that a GPU
# can run the complete Hermes stack.  The report is therefore written on both
# success and failure, and the script exits non-zero unless every software,
# completeness, and physical-hardware gate is present.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT=${HERMES_QUALIFICATION_DIR:-$ROOT/target/hermes-qualification}
LOG_DIR=$OUT/log
REPORT=${HERMES_RELEASE_MANIFEST:-$OUT/release-manifest.txt}
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/hermes-qualification.XXXXXX")
trap 'rm -rf -- "$TMP_DIR"' EXIT

# ArachOS calls this script from its own checkout. Every gate must execute
# against Hermes' workspace, otherwise Cargo could qualify whichever
# repository happened to be the caller's current directory.
cd "$ROOT"

mkdir -p "$LOG_DIR"

overall=pass
declare -A result

record_fail() {
    overall=blocked
}

run_gate() {
    local name=$1
    shift
    local log=$LOG_DIR/$name.log
    if "$@" >"$log" 2>&1; then
        result[$name]=pass
    else
        result[$name]=fail
        record_fail
    fi
}

run_gate cargo_fmt cargo fmt --all -- --check
run_gate cargo_clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run_gate cargo_tests cargo test --workspace --all-targets --locked
run_gate formal_strict bash "$ROOT/scripts/check-formal.sh" --strict
run_gate dropin_catalog cargo run --frozen --locked --release -p hermes-ctl -- dropin-parity
run_gate source_license bash "$ROOT/scripts/audit-open-source.sh"
run_gate source_clean bash -c '
    set -euo pipefail
    test -z "$(git -C "$1" status --porcelain --untracked-files=all)"
' bash "$ROOT"
run_gate integration_smoke cargo run --frozen --locked --release -p hermes-ctl -- stack-smoke

# Every host-side submission boundary must use the shared equation engine.  A
# source-level coverage check catches accidental regressions where one layer
# silently falls back to an unrelated scheduler.
chaos_log=$LOG_DIR/chaos-coverage.log
chaos_ok=1
for boundary in \
    'crates/hermes-core/src/ring.rs:ChaosScheduler' \
    'crates/hermes-gsp/src/stage.rs:ChaosScheduler' \
    'crates/hermes-gsp/src/mailbox.rs:ChaosScheduler' \
    'crates/hermes-cuda/src/lib.rs:ChaosScheduler' \
    'crates/hermes-drm/src/pageflip.rs:ChaosScheduler' \
    'crates/hermes-mesa/src/lib.rs:ChaosScheduler' \
    'crates/hermes-nvml/src/lib.rs:ChaosScheduler' \
    'crates/hermes-ctl/src/mps_main.rs:ChaosScheduler'; do
    file=${boundary%%:*}
    symbol=${boundary#*:}
    if ! rg -q "$symbol" "$ROOT/$file"; then
        printf 'missing %s in %s\n' "$symbol" "$file" >>"$chaos_log"
        chaos_ok=0
    fi
done
if ((chaos_ok)); then
    printf '%s\n' 'shared chaos scheduler coverage: PASS' >"$chaos_log"
    result[chaos_coverage]=pass
else
    result[chaos_coverage]=fail
    record_fail
fi

# Build both the loadable modules and their host-only C tests.  The clean is
# scoped to this kmod tree so a failed qualification cannot accumulate stale
# module objects, while the Rust target directory remains reusable.
run_gate kmod_build bash -c '
    set -euo pipefail
    make -C "$1/linux/kmod" clean
    make -C "$1/linux/kmod" CC=gcc all
    make -C "$1/linux/kmod" host-test
' bash "$ROOT"

# These markers identify code paths that are still compatibility shells rather
# than production implementations.  Safety checks for a missing prerequisite
# are valid; an explicit placeholder or unimplemented surface is not release
# evidence.  Keep this audit in source trees only so generated build logs do
# not make the result nondeterministic.
runtime_log=$LOG_DIR/runtime-completeness.log
if ! command -v rg >/dev/null 2>&1; then
    printf '%s\n' 'ripgrep is required for the source completeness audit' >"$runtime_log"
    result[runtime_completeness]=fail
    record_fail
elif rg -n --glob '!target/**' --glob '!.git/**' \
        'Full DRM subsystem registration is future work|Full MPS server is not implemented|HermesStub|GL stubs' \
        "$ROOT/crates" "$ROOT/linux" >"$runtime_log" 2>&1; then
    result[runtime_completeness]=fail
    record_fail
else
    result[runtime_completeness]=pass
fi

# A physical qualification report is supplied by the operator after running
# the hardware matrix.  Simulation output and a missing report are rejected.
hardware_log=$LOG_DIR/hardware-evidence.log
hardware_report=${HERMES_HARDWARE_EVIDENCE:-}
required_hardware_gates=(
    nvidia_online amd_online intel_online firmware_measurement gsp_boot
    drm_kms cuda nvml mesa mps uvm peermem fault_recovery soak
)
if [[ -z $hardware_report || ! -r $hardware_report ]]; then
    printf '%s\n' 'no HERMES_HARDWARE_EVIDENCE report was supplied' >"$hardware_log"
    result[hardware]=fail
    record_fail
else
    {
        printf 'report=%s\n' "$hardware_report"
        printf 'attestation=%s\n' "$(sed -n 's/^attestation=//p' "$hardware_report" | head -n 1)"
        printf 'simulation=%s\n' "$(sed -n 's/^simulation=//p' "$hardware_report" | head -n 1)"
    } >"$hardware_log"
    hardware_ok=1
    [[ $(sed -n 's/^schema=//p' "$hardware_report" | head -n 1) == hermes-hardware-v1 ]] || hardware_ok=0
    [[ $(sed -n 's/^attestation=//p' "$hardware_report" | head -n 1) == physical-gpu ]] || hardware_ok=0
    [[ $(sed -n 's/^simulation=//p' "$hardware_report" | head -n 1) == 0 ]] || hardware_ok=0
    for gate in "${required_hardware_gates[@]}"; do
        value=$(sed -n "s/^$gate=//p" "$hardware_report" | head -n 1)
        printf '%s=%s\n' "$gate" "$value" >>"$hardware_log"
        [[ $value == pass ]] || hardware_ok=0
    done
    if ((hardware_ok)); then
        result[hardware]=pass
    else
        result[hardware]=fail
        record_fail
    fi
fi

source_revision=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || printf unknown)
report_tmp=$TMP_DIR/release-manifest.txt
{
    printf 'schema=hermes-release-v1\n'
    printf 'status=%s\n' "$overall"
    printf 'source_revision=%s\n' "$source_revision"
    for gate in cargo_fmt cargo_clippy cargo_tests formal_strict dropin_catalog \
                source_license source_clean integration_smoke chaos_coverage kmod_build \
                runtime_completeness hardware; do
        printf '%s=%s\n' "$gate" "${result[$gate]:-fail}"
    done
    if [[ -n $hardware_report ]]; then
        printf 'hardware_evidence=%s\n' "$hardware_report"
    else
        printf 'hardware_evidence=missing\n'
    fi
} >"$report_tmp"
mkdir -p "$(dirname "$REPORT")"
mv -f -- "$report_tmp" "$REPORT"

printf 'Hermes qualification: %s\n' "$overall"
printf 'Hermes release manifest: %s\n' "$REPORT"
for gate in cargo_fmt cargo_clippy cargo_tests formal_strict dropin_catalog \
            source_license source_clean integration_smoke chaos_coverage kmod_build \
            runtime_completeness hardware; do
    printf '  %-24s %s\n' "$gate" "${result[$gate]:-fail}"
done

[[ $overall == pass ]]
