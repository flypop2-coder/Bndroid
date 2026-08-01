#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
GENERATOR="$SCRIPT_DIR/build_multipackage_storage_image.py"
EXPECTED_SHA256="c82bffe86b4fea83e5483723dc659b43f11bb9f320c6b2c7442d00e5026cdadc"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-multipackage-image.XXXXXX")"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

for ordinal in 1 2 3; do
  python3 "$GENERATOR" build "$TMP_DIR/image-$ordinal.raw"
  python3 "$GENERATOR" verify "$TMP_DIR/image-$ordinal.raw"
done
cmp "$TMP_DIR/image-1.raw" "$TMP_DIR/image-2.raw"
cmp "$TMP_DIR/image-1.raw" "$TMP_DIR/image-3.raw"

actual_sha256="$(shasum -a 256 "$TMP_DIR/image-1.raw" | awk '{print $1}')"
if [[ "$actual_sha256" != "$EXPECTED_SHA256" ]]; then
  echo "Unexpected multi-package storage SHA-256: $actual_sha256" >&2
  exit 1
fi

python3 - "$SCRIPT_DIR" "$TMP_DIR" <<'PY'
import struct
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[1])
import build_multipackage_storage_image as multi

tmp = Path(sys.argv[2])
canonical = bytearray((tmp / "image-1.raw").read_bytes())
negative_cases = 0


def reject(label, image, fragment):
    global negative_cases
    try:
        multi.validate_image(bytes(image))
    except multi.MultiPackageImageError as error:
        if fragment not in str(error):
            raise SystemExit(
                f"{label} failed for {error!s}; expected fragment {fragment!r}"
            )
        negative_cases += 1
        return
    raise SystemExit(f"{label} unexpectedly passed")


nonzero_second_slot = bytearray(canonical)
offset = (
    multi.base.PACKAGE_STORE_PARTITION_FIRST_LBA + 512
) * multi.base.SECTOR_BYTES
nonzero_second_slot[offset] = 1
reject(
    "nonzero second slot",
    nonzero_second_slot,
    "multi-package partition is not virgin zero storage",
)

bad_header_crc = bytearray(canonical)
bad_header_crc[multi.base.PRIMARY_HEADER_LBA * multi.base.SECTOR_BYTES + 16] ^= 1
reject("bad primary header CRC", bad_header_crc, "GPT header CRC32")

bad_entries_crc = bytearray(canonical)
bad_entries_crc[multi.base.PRIMARY_ENTRIES_LBA * multi.base.SECTOR_BYTES + 7] ^= 1
reject("bad primary entries CRC", bad_entries_crc, "GPT entry array CRC32")

legacy_extent = bytearray(canonical)
entry_offset = (
    multi.base.PRIMARY_ENTRIES_LBA * multi.base.SECTOR_BYTES
    + 3 * multi.base.GPT_ENTRY_BYTES
    + 40
)
struct.pack_into(
    "<Q", legacy_extent, entry_offset, multi.base.PACKAGE_STORE_PARTITION_LAST_LBA
)
reject("legacy extent", legacy_extent, "GPT entry array CRC32")

if negative_cases != 4:
    raise SystemExit(f"negative case count {negative_cases} != 4")
PY

marker="$(python3 "$GENERATOR" marker "$TMP_DIR/image-1.raw")"
for field in \
  "MULTIPACKAGE_STORAGE_IMAGE_OK" \
  "sha256=$EXPECTED_SHA256" \
  "package_partition_lba=16384-17407" \
  "package_partition_sectors=1024" \
  "package_capacity=2" \
  "legacy_first_volume=1" \
  "package_initial_zero=1"; do
  if [[ "$marker" != *"$field"* ]]; then
    echo "Multi-package image marker is missing: $field" >&2
    exit 1
  fi
done

echo "MULTIPACKAGE_STORAGE_IMAGE_HOST_OK deterministic_builds=3 negative_cases=4 sha256=$EXPECTED_SHA256"
