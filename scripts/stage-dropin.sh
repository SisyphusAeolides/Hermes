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
  "$PREFIX/share/hermes" \
  "$PREFIX/share/glvnd/egl_vendor.d"

# Binaries (classic NVIDIA names + Hermes control)
for b in hermes-ctl nvidia-smi nvidia-settings hermes-settings nvidia-modprobe; do
  if [ -f "$TARGET/$b" ]; then
    install -m 0755 "$TARGET/$b" "$PREFIX/bin/$b"
    printf '  bin/%s\n' "$b"
  else
    printf '  WARN missing bin/%s (build may have failed)\n' "$b" >&2
  fi
done

# Find cdylib outputs
for f in "$TARGET"/libhermes_mesa.so "$TARGET"/libhermes_mesa.so.* \
         "$TARGET"/libnvidia_ml.so "$TARGET"/libnvidia_ml.so.* \
         "$TARGET"/libhermes_cuda.so "$TARGET"/libhermes_cuda.so.*; do
  [ -e "$f" ] || continue
  base=$(basename "$f")
  install -m 0755 "$f" "$PREFIX/lib/$base"
  printf '  lib/%s\n' "$base"
done

# Soname-friendly links for loader drop-in
pick_one() {
  # shellcheck disable=SC2046
  ls -1 "$1" 2>/dev/null | head -1 || true
}

if [ -f "$PREFIX/lib/libhermes_mesa.so" ]; then
  ln -sfn libhermes_mesa.so "$PREFIX/lib/libGLX_nvidia.so.0"
  ln -sfn libhermes_mesa.so "$PREFIX/lib/libEGL_nvidia.so.0"
  printf '  lib/libGLX_nvidia.so.0 -> libhermes_mesa.so\n'
  printf '  lib/libEGL_nvidia.so.0 -> libhermes_mesa.so\n'
fi
ml=$(pick_one "$PREFIX/lib"/libnvidia_ml.so*)
if [ -n "$ml" ]; then
  base=$(basename "$ml")
  ln -sfn "$base" "$PREFIX/lib/libnvidia-ml.so.1"
  ln -sfn "$base" "$PREFIX/lib/libnvidia-ml.so"
  printf '  lib/libnvidia-ml.so.1 -> %s\n' "$base"
fi
cu=$(pick_one "$PREFIX/lib"/libhermes_cuda.so*)
if [ -n "$cu" ]; then
  base=$(basename "$cu")
  ln -sfn "$base" "$PREFIX/lib/libcuda.so.1"
  ln -sfn "$base" "$PREFIX/lib/libcuda.so"
  printf '  lib/libcuda.so.1 -> %s\n' "$base"
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

# EGL vendor JSON (GLVND) — points at Hermes Mesa soname when present
if [ -f "$PREFIX/lib/libEGL_nvidia.so.0" ] || [ -L "$PREFIX/lib/libEGL_nvidia.so.0" ]; then
  cat >"$PREFIX/share/glvnd/egl_vendor.d/10_nvidia.json" <<EOF
{
    "file_format_version" : "1.0.0",
    "ICD" : {
        "library_path" : "$PREFIX/lib/libEGL_nvidia.so.0"
    }
}
EOF
  printf '  EGL vendor 10_nvidia.json\n'
fi

# Catalog snapshot from hermes-ctl when available
if [ -x "$PREFIX/bin/hermes-ctl" ]; then
  "$PREFIX/bin/hermes-ctl" dropin-catalog >"$PREFIX/share/hermes/DROPIN_CATALOG.txt" 2>&1 || true
fi

# Manifest
cat >"$PREFIX/share/hermes/DROPIN_MANIFEST.txt" <<EOF
Hermes drop-in stage
prefix=$PREFIX
bins=hermes-ctl,nvidia-smi,nvidia-settings,nvidia-modprobe
libs=libnvidia-ml.so.1,libcuda.so.1,libGLX_nvidia.so.0,libEGL_nvidia.so.0
modules=linux/kmod (nvidia, nvidia-modeset, nvidia-uvm, nvidia-drm, nvidia-peermem)
note=GSP Online is never implied by staging binaries
firmware=stage separately via scripts/stage-gsp-rm.sh or stage-linux-firmware-gsp.sh
device=/dev/nvidia* via kmod + nvidia-modprobe --status / -c
modprobe=$PREFIX/bin/nvidia-modprobe --status
EOF

printf 'done. Load kmods from linux/kmod after make. Export:\n'
printf '  export PATH=%s/bin:\$PATH\n' "$PREFIX"
printf '  export LD_LIBRARY_PATH=%s/lib:\$LD_LIBRARY_PATH\n' "$PREFIX"
printf '  export VK_ICD_FILENAMES=%s\n' "$ICD_JSON"
printf '  nvidia-modprobe --status\n'
