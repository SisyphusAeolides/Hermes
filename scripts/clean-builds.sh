#!/usr/bin/env bash
# Remove only Hermes-generated build output; operator-staged firmware is kept.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DRY_RUN=0
KEEP_TARGET=0

usage() {
    printf '%s\n' "Usage: $0 [--dry-run] [--keep-target]"
}

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        --keep-target) KEEP_TARGET=1 ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'unknown option: %s\n' "$arg" >&2; usage >&2; exit 2 ;;
    esac
done

remove_path() {
    local path=$1
    if [[ ! -e $path && ! -L $path ]]; then
        return
    fi
    if ((DRY_RUN)); then
        printf 'would remove %s\n' "$path"
    else
        rm -rf -- "$path"
        printf 'removed %s\n' "$path"
    fi
}

remove_files() {
    local root=$1
    shift
    [[ -d $root ]] || return
    while IFS= read -r -d '' path; do
        if ((DRY_RUN)); then
            printf 'would remove %s\n' "$path"
        else
            rm -f -- "$path"
            printf 'removed %s\n' "$path"
        fi
    done < <(find "$root" -maxdepth 1 -type f \( "$@" \) -print0)
}

if ((KEEP_TARGET == 0)); then
    remove_path "$ROOT/target"
fi
remove_path "$ROOT/formal/fortran/build"
remove_path "$ROOT/formal/idris2/build"
remove_files "$ROOT/formal/agda" -name '*.agdai'

# `make clean` is scoped to this repository's kernel-module directory.  The
# explicit patterns below also remove host-test binaries that kbuild does not
# know about; no path outside the checkout is touched.
if [[ -f $ROOT/linux/kmod/Makefile ]]; then
    if ((DRY_RUN)); then
        printf 'would run make -C %s clean\n' "$ROOT/linux/kmod"
    else
        make -C "$ROOT/linux/kmod" clean >/dev/null
    fi
fi
remove_files "$ROOT/linux/kmod" \
    -name '*.o' -o -name '*.ko' -o -name '*.mod' -o -name '*.mod.c' \
    -o -name '*.cmd' -o -name 'Module.symvers' -o -name 'modules.order'
remove_files "$ROOT/linux/kmod/tests" -name 'test_*_host'

if ((DRY_RUN)); then
    printf '%s\n' 'Hermes generated-build cleanup: dry run complete'
else
    printf '%s\n' 'Hermes generated-build cleanup: complete'
fi
