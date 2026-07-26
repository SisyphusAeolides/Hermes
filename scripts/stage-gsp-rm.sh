#!/usr/bin/env sh
# Stage one OpenRM gsp_*.bin into target/hermes-gsp after allow-list check.
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source_image=${1:?usage: stage-gsp-rm.sh SOURCE_IMAGE TARGET_DIRECTORY}
target_directory=${2:?usage: stage-gsp-rm.sh SOURCE_IMAGE TARGET_DIRECTORY}

case "$target_directory" in
  "$project_root"/target/hermes-gsp/*) ;;
  target/hermes-gsp/*) target_directory="$project_root/$target_directory" ;;
  *)
    printf '%s\n' 'target directory must be below target/hermes-gsp/' >&2
    exit 64
    ;;
esac

if [ ! -f "$source_image" ]; then
  printf '%s\n' "missing: $source_image" >&2
  exit 66
fi

set -- $(sha256sum "$source_image")
image_sha256=$1
image_bytes=$(wc -c < "$source_image" | tr -d '[:space:]')

# Allow-list mirrors crates/hermes-gsp/src/firmware.rs pins.
case "$image_sha256:$image_bytes" in
  c8fc1a92c90b034bbbe4d56ca94b0dc95afb52d3409a7880186ae03c7dde17f3:29352832)
    family=tu10x; version=610.43.02 ;;
  00da3fd9b41db8afd661c9dcec2a32a31d3c14b93e6d7112d4fb3f46876525ce:84277400)
    family=ga10x; version=610.43.02 ;;
  73065619db9ec921d19fc4e519dd04d91a9199b525eaca9b257b89fb8c5ec52c:29352832)
    family=tu10x; version=610.43.03 ;;
  572373620a37418f24dc16b5031c39338778c3257e48e8408de9a57291b24f3a:84277400)
    family=ga10x; version=610.43.03 ;;
  *)
    printf '%s\n' 'firmware not in Hermes GSP-RM allow-list' >&2
    printf '%s\n' "sha256=$image_sha256 bytes=$image_bytes" >&2
    exit 65
    ;;
esac

mkdir -p "$target_directory"
install -m 0644 "$source_image" "$target_directory/gsp_${family}.bin"
{
  printf '%s\n' "family=$family"
  printf '%s\n' "version=$version"
  printf '%s\n' "bytes=$image_bytes"
  printf '%s\n' "sha256=$image_sha256"
  printf '%s\n' "source=$source_image"
} > "$target_directory/manifest.env"
printf '%s\n' "$target_directory/manifest.env"
