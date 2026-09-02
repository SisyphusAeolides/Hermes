#!/usr/bin/env sh
# Stage redistributable GSP-RM images from a linux-firmware / driver install.
# Never commits blobs — only copies into target/ and writes manifest.env.
set -eu

usage() {
  printf '%s\n' "usage: $0 FIRMWARE_ROOT TARGET_DIR [VERSION]"
  printf '%s\n' "  FIRMWARE_ROOT  e.g. /lib/firmware"
  printf '%s\n' "  TARGET_DIR     must be under target/hermes-gsp/"
  printf '%s\n' "  VERSION        default: 610.57.04"
}

firmware_root=${1:?$(usage)}
target_directory=${2:?$(usage)}
version=${3:-610.57.04}

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
case "$target_directory" in
  "$project_root"/target/hermes-gsp/*) ;;
  target/hermes-gsp/*) target_directory="$project_root/$target_directory" ;;
  *)
    printf '%s\n' 'target directory must be below target/hermes-gsp/' >&2
    exit 64
    ;;
esac

if ! command -v sha256sum >/dev/null 2>&1; then
  printf '%s\n' 'sha256sum required' >&2
  exit 69
fi

mkdir -p "$target_directory"
staged=0

for family in tu10x ga10x; do
  src="$firmware_root/nvidia/$version/gsp_${family}.bin"
  if [ ! -f "$src" ]; then
    printf '%s\n' "skip missing: $src" >&2
    continue
  fi
  set -- $(sha256sum "$src")
  image_sha256=$1
  image_bytes=$(wc -c < "$src" | tr -d '[:space:]')
  dest="$target_directory/gsp_${family}.bin"
  install -m 0644 "$src" "$dest"
  {
    printf '%s\n' "family=$family"
    printf '%s\n' "version=$version"
    printf '%s\n' "bytes=$image_bytes"
    printf '%s\n' "sha256=$image_sha256"
    printf '%s\n' "source=$src"
    printf '%s\n' "layout=openrm-versioned"
  } > "$target_directory/gsp_${family}.manifest.env"
  printf '%s\n' "staged $dest ($image_bytes bytes)"
  staged=$((staged + 1))
done

if [ "$staged" -eq 0 ]; then
  printf '%s\n' "no GSP images found under $firmware_root/nvidia/$version/" >&2
  exit 66
fi

printf '%s\n' "staged=$staged dir=$target_directory"
