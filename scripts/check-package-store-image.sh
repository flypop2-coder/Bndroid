#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$SCRIPT_DIR/build_storage_image.py"
WRAPPER="$SCRIPT_DIR/build-storage-image.sh"

for tool in python3 cmp mktemp cp grep rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for the package-store image host checks." >&2
    exit 1
  fi
done

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-package-store-image.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

build_variant() {
  local variant="$1"
  local output="$2"
  case "$variant" in
    default)
      python3 "$GENERATOR" build "$output"
      ;;
    appdata)
      python3 "$GENERATOR" build "$output" --with-appdata
      ;;
    package-store)
      python3 "$GENERATOR" build "$output" --with-package-store
      ;;
    *)
      echo "Unknown storage fixture variant: $variant" >&2
      return 2
      ;;
  esac
}

verify_variant() {
  local variant="$1"
  local image="$2"
  case "$variant" in
    default)
      python3 "$GENERATOR" verify "$image"
      ;;
    appdata)
      python3 "$GENERATOR" verify "$image" --with-appdata
      ;;
    package-store)
      python3 "$GENERATOR" verify "$image" --with-package-store
      ;;
  esac
}

for variant in default appdata package-store; do
  for ordinal in 1 2 3; do
    image="$TMP_DIR/$variant-$ordinal.raw"
    build_variant "$variant" "$image"
    verify_variant "$variant" "$image"
  done
  cmp "$TMP_DIR/$variant-1.raw" "$TMP_DIR/$variant-2.raw"
  cmp "$TMP_DIR/$variant-1.raw" "$TMP_DIR/$variant-3.raw"
done

python3 - \
  "$TMP_DIR/default-1.raw" \
  "$TMP_DIR/appdata-1.raw" \
  "$TMP_DIR/package-store-1.raw" <<'PY'
import hashlib
import sys
from pathlib import Path

expected = {
    "default": (
        Path(sys.argv[1]),
        8_388_608,
        "6cdca2781345e712a2a0d94d4b1327ed7f971c0a971cfd7d5c5f78b8b6d2e838",
    ),
    "appdata": (
        Path(sys.argv[2]),
        8_388_608,
        "577a9c422b0edfb05f18f02120c0632d9f64c632664465106fb9876b08ffc1cf",
    ),
    "package-store": (
        Path(sys.argv[3]),
        16_777_216,
        "b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801",
    ),
}
for variant, (path, size, digest) in expected.items():
    image = path.read_bytes()
    if len(image) != size:
        raise SystemExit(f"{variant} image has {len(image)} bytes; expected {size}")
    actual = hashlib.sha256(image).hexdigest()
    if actual != digest:
        raise SystemExit(f"{variant} SHA-256 {actual} != {digest}")
PY

WRAPPER_IMAGE="$TMP_DIR/wrapper-package-store.raw"
BNDROID_STORAGE_IMAGE="$WRAPPER_IMAGE" \
  "$WRAPPER" --with-package-store >"$TMP_DIR/wrapper.marker"
BNDROID_STORAGE_IMAGE="$WRAPPER_IMAGE" \
  "$WRAPPER" --verify --with-package-store >/dev/null
cmp "$TMP_DIR/package-store-1.raw" "$WRAPPER_IMAGE"

if ! grep -Fq \
  'DEFAULT_IMAGE_PATH="$WORKSPACE_ROOT/target/bndroid-storage-apk-install0.raw"' \
  "$WRAPPER"; then
  echo "Package-store wrapper does not select its independent target filename." >&2
  exit 1
fi

MARKER="$(python3 "$GENERATOR" marker \
  "$TMP_DIR/package-store-1.raw" --with-package-store)"
for field in \
  "STORAGE_IMAGE_OK bytes=16777216 sectors=32768 sector_bytes=512 variant=apk-install0" \
  "sha256=b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801" \
  "gpt_last_usable=32734" \
  "gpt_backup_entries=32735-32766" \
  "gpt_backup_header=32767" \
  "gpt_primary_header_crc32=0xcf2787ad" \
  "gpt_backup_header_crc32=0x36d1d613" \
  "gpt_partition_entry_crc32=0x13648066" \
  "appdata_partition_lba=128-2047" \
  "package_type_guid=b8f4d2a3-7c3e-4b91-a6d5-0f2e9c781355" \
  "package_type_guid_raw=a3d2f4b83e7c914ba6d50f2e9c781355" \
  "package_partition_guid=d25cf174-a879-4e5a-9364-67b2d8905801" \
  "package_partition_name=BNDROID_PACKAGES" \
  "package_partition_lba=16384-16895" \
  "package_partition_sectors=512" \
  "package_initial_zero=1"; do
  if [[ "$MARKER" != *"$field"* ]]; then
    echo "Package-store marker is missing fixed field: $field" >&2
    exit 1
  fi
done

python3 - "$SCRIPT_DIR" "$TMP_DIR" <<'PY'
import struct
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[1])
import build_storage_image as fixture

tmp = Path(sys.argv[2])
canonical = fixture.build_image(include_package_store=True)
negative_cases = 0


def reject(label, image, fragment):
    global negative_cases
    try:
        fixture.validate_layout(bytes(image), include_package_store=True)
    except fixture.FixtureError as error:
        if fragment not in str(error):
            raise SystemExit(
                f"{label} failed for the wrong reason: {error!s}; "
                f"expected fragment {fragment!r}"
            )
        negative_cases += 1
        return
    raise SystemExit(f"{label} unexpectedly passed package-store validation")


def reseal_header(image, lba):
    offset = lba * fixture.SECTOR_BYTES
    struct.pack_into("<I", image, offset + 16, 0)
    checksum = fixture.crc32(
        bytes(image[offset : offset + fixture.GPT_HEADER_SIZE])
    )
    struct.pack_into("<I", image, offset + 16, checksum)


def mutate_entries(image, mutate):
    primary_slice = fixture.image_sector_slice(
        image, fixture.PRIMARY_ENTRIES_LBA, fixture.GPT_ENTRY_SECTORS
    )
    entries = bytearray(image[primary_slice])
    mutate(entries)
    entry_crc = fixture.crc32(entries)
    image[primary_slice] = entries
    backup_slice = fixture.image_sector_slice(
        image,
        fixture.PACKAGE_STORE_BACKUP_ENTRIES_LBA,
        fixture.GPT_ENTRY_SECTORS,
    )
    image[backup_slice] = entries
    for header_lba in (
        fixture.PRIMARY_HEADER_LBA,
        fixture.PACKAGE_STORE_BACKUP_HEADER_LBA,
    ):
        struct.pack_into(
            "<I",
            image,
            header_lba * fixture.SECTOR_BYTES + 88,
            entry_crc,
        )
        reseal_header(image, header_lba)


package_nonzero = bytearray(canonical)
package_nonzero[
    fixture.PACKAGE_STORE_PARTITION_FIRST_LBA * fixture.SECTOR_BYTES
] = 1
reject("nonzero package partition", package_nonzero, "package-store partition is not empty")
(tmp / "corrupt-package.raw").write_bytes(package_nonzero)

appdata_nonzero = bytearray(canonical)
appdata_nonzero[fixture.APPDATA_PARTITION_FIRST_LBA * fixture.SECTOR_BYTES] = 1
reject("nonzero appdata partition", appdata_nonzero, "appdata partition is not empty")

primary_crc = bytearray(canonical)
primary_crc[fixture.PRIMARY_HEADER_LBA * fixture.SECTOR_BYTES + 24] ^= 1
reject("primary GPT CRC", primary_crc, "header CRC32")
(tmp / "corrupt-primary-gpt.raw").write_bytes(primary_crc)

backup_crc = bytearray(canonical)
backup_crc[
    fixture.PACKAGE_STORE_BACKUP_HEADER_LBA * fixture.SECTOR_BYTES + 24
] ^= 1
reject("backup GPT CRC", backup_crc, "header CRC32")

overlap = bytearray(canonical)
mutate_entries(
    overlap,
    lambda entries: struct.pack_into(
        "<QQ",
        entries,
        3 * fixture.GPT_ENTRY_BYTES + 32,
        16_000,
        16_511,
    ),
)
reject("overlapping package partition", overlap, "overlap")

outside_bounds = bytearray(canonical)
mutate_entries(
    outside_bounds,
    lambda entries: struct.pack_into(
        "<QQ",
        entries,
        3 * fixture.GPT_ENTRY_BYTES + 32,
        fixture.PACKAGE_STORE_PARTITION_FIRST_LBA,
        fixture.PACKAGE_STORE_LAST_USABLE_LBA + 1,
    ),
)
reject("out-of-bounds package partition", outside_bounds, "outside usable disk bounds")

unused_entry = bytearray(canonical)
mutate_entries(
    unused_entry,
    lambda entries: entries.__setitem__(4 * fixture.GPT_ENTRY_BYTES, 1),
)
reject("nonzero unused GPT entry", unused_entry, "unused GPT partition entries")

if fixture.PACKAGE_STORE_TYPE_GUID_RAW != bytes.fromhex(
    "a3 d2 f4 b8 3e 7c 91 4b a6 d5 0f 2e 9c 78 13 55"
):
    raise SystemExit("package-store type GUID does not have the fixed on-disk bytes")
if negative_cases != 7:
    raise SystemExit(f"ran {negative_cases} negative cases; expected 7")
PY

if python3 "$GENERATOR" verify \
  "$TMP_DIR/corrupt-package.raw" --with-package-store >/dev/null 2>&1 \
  || python3 "$GENERATOR" verify \
  "$TMP_DIR/corrupt-primary-gpt.raw" --with-package-store >/dev/null 2>&1 \
  || python3 "$GENERATOR" verify \
  "$TMP_DIR/package-store-1.raw" >/dev/null 2>&1 \
  || python3 "$GENERATOR" verify \
  "$TMP_DIR/default-1.raw" --with-package-store >/dev/null 2>&1; then
  echo "A corrupt or wrong-variant image passed package-store verify mode." >&2
  exit 1
fi

echo \
  "PACKAGE_STORE_IMAGE_HOST_OK deterministic_builds=3 variants=3 old_hashes_unchanged=2 package_bytes=16777216 package_sectors=32768 package_lba=16384-16895 package_sectors_count=512 package_initial_zero=1 gpt_crc_checked=1 gpt_bounds_checked=1 partition_overlap_checked=1 negative_cases=7 sha256=b2ae6008e4a386911608d603b261751a9942db4c95870cdeaa655854e2e5e801"
