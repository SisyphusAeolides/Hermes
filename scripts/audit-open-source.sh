#!/usr/bin/env bash
# Verify the source/firmware boundary before packaging Hermes.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"

[[ -f LICENSE ]] || { echo "missing MIT LICENSE" >&2; exit 1; }
grep -q "MIT License" LICENSE || { echo "LICENSE is not the MIT text" >&2; exit 1; }
grep -q '^license = "MIT"' Cargo.toml || {
    echo "workspace Cargo.toml must declare MIT" >&2
    exit 1
}

while IFS= read -r -d '' manifest; do
    grep -q 'license.workspace = true' "$manifest" || {
        echo "package missing workspace license: $manifest" >&2
        exit 1
    }
done < <(find crates -mindepth 2 -maxdepth 2 -name Cargo.toml -print0 | sort -z)

# A source checkout must not carry prebuilt proprietary driver or firmware
# artifacts. Build output is ignored and therefore not part of this audit.
if git ls-files --cached --others --exclude-standard -z | tr '\0' '\n' | rg -n -i \
    '\.(bin|elf|ko|o|so|a|run|exe|dll|dylib|deb|rpm)$|(^|/)\.git/lfs/' ; then
    echo "binary driver/firmware artifact found in source tree" >&2
    exit 1
fi

if git grep -n '^version https://git-lfs.github.com/spec/v1' -- ':!target' 2>/dev/null; then
    echo "Git LFS pointer found in source tree" >&2
    exit 1
fi

echo "Hermes open-source audit: PASS (MIT source; firmware remains separately staged)"
