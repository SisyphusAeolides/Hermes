#!/usr/bin/env sh
# Stage Hermes drop-in artifacts into a prefix (default: ./staging/dropin).
# Does NOT claim GPU Online. Firmware blobs are never copied here.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
PREFIX="${1:-$ROOT/staging/dropin}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}/release"

printf 'Hermes drop-in stage → %s\n' "$PREFIX"

cargo build --release \
  -p hermes-ctl \
  -p hermes-settings \
  -p hermes-nvml \
  -p hermes-cuda \
  -p hermes-mesa \
  --manifest-path "$ROOT/Cargo.toml"

mkdir -p \
  "$PREFIX/bin" \
  "$PREFIX/lib" \
  "$PREFIX/share/vulkan/icd.d" \
  "$PREFIX/etc/vulkan/icd.d" \
  "$PREFIX/share/hermes"

# Binaries
for b in hermes-ctl nvidia-smi nvidia-settings hermes-settings; do
  if [ -f "$TARGET/$b" ]; then
    install -m 0755 "$TARGET/$b" "$PREFIX/bin/$b"
    printf '  bin/%s\n' "$b"
  fi
done

# Libraries (names may vary by rustc target naming)
copy_lib() {
  src="$1"
  dst="$2"
  if [ -f "$src" ]; then
    install -m 0755 "$src" "$PREFIX/lib/$dst"
    printf '  lib/%s\n' "$dst"
  fi
}

# Find cdylib outputs
for f in "$TARGET"/libhermes_mesa.so "$TARGET"/libhermes_mesa.so.* \
         "$TARGET"/libnvidia_ml.so "$TARGET"/libnvidia_ml.so.* \
         "$TARGET"/libhermes_cuda.so "$TARGET"/libhermes_cuda.so.*; do
  [ -e "$f" ] || continue
  base=$(basename "$f")
  install -m 0755 "$f" "$PREFIX/lib/$base"
  printf '  lib/%s\n' "$base"
done

# Soname-friendly links when present
if [ -f "$PREFIX/lib/libhermes_mesa.so" ]; then
  ln -sfn libhermes_mesa.so "$PREFIX/lib/libGLX_nvidia.so.0" 2>/dev/null || true
fi
if ls "$PREFIX/lib"/libnvidia_ml.so* >/dev/null 2>&1; then
  ln -sfn "$(ls "$PREFIX/lib"/libnvidia_ml.so* | head -1 | xargs basename)" \
    "$PREFIX/lib/libnvidia-ml.so.1" 2>/dev/null || true
fi
if ls "$PREFIX/lib"/libhermes_cuda.so* >/dev/null 2>&1; then
  ln -sfn "$(ls "$PREFIX/lib"/libhermes_cuda.so* | head -1 | xargs basename)" \
    "$PREFIX/lib/libcuda.so.1" 2>/dev/null || true
fi

# Vulkan ICD JSON
ICD_JSON="$PREFIX/share/vulkan/icd.d/hermes_icd.json"
cat >"$ICD_JSON" <<EOF
{
    "file_format_version": "1.0.0",
    "ICD": {
        "library_path": "$PREFIX/lib/libhermes_mesa.so",
        "api_version": "1.3.0"
    }
}
EOF
cp "$ICD_JSON" "$PREFIX/etc/vulkan/icd.d/hermes_icd.json"
# Optional NVIDIA-named ICD pointing at Hermes (operator chooses)
cp "$ICD_JSON" "$PREFIX/share/vulkan/icd.d/nvidia_icd.json"
printf '  ICD hermes_icd.json + nvidia_icd.json\n'

# Manifest
cat >"$PREFIX/share/hermes/DROPIN_MANIFEST.txt" <<EOF
Hermes drop-in stage
prefix=$PREFIX
modules=linux/kmod (nvidia, nvidia-modeset, nvidia-uvm, nvidia-drm, nvidia-peermem)
note=GSP Online is never implied by staging binaries
firmware=stage separately via scripts/stage-gsp-rm.sh or stage-linux-firmware-gsp.sh
device=/dev/nvidia-drm (from nvidia-drm.ko misc device)
EOF

printf 'done. Load kmods from linux/kmod after make. Export:\n'
printf '  export PATH=%s/bin:\$PATH\n' "$PREFIX"
printf '  export LD_LIBRARY_PATH=%s/lib:\$LD_LIBRARY_PATH\n' "$PREFIX"
printf '  export VK_ICD_FILENAMES=%s\n' "$ICD_JSON"
