#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$SCRIPT_DIR/build_storage_image.py"
MODE="build"
VARIANT="m25"
MODE_SET=0
VARIANT_SET=0

usage() {
  cat <<'EOF'
Usage: build-storage-image.sh [--verify] [--with-appdata|--with-package-store]

Builds a deterministic Bndroid GPT/FAT16 plus persistent-data fixture.
--with-appdata selects the existing 8 MiB M54 AppData image.
--with-package-store selects the opt-in 16 MiB APK-install0 image, which also
includes AppData and an empty BNDROID_PACKAGES partition.

Set BNDROID_STORAGE_IMAGE to select the image path. With --verify, the selected
image is compared byte-for-byte with a newly generated reference and checked
against fixed hashes, GPT CRC32/bounds, non-overlapping partitions, zero
initialization, FAT16 layout, and file fingerprints.
EOF
}

while (($#)); do
  case "$1" in
    --verify)
      if ((MODE_SET)); then
        usage >&2
        exit 2
      fi
      MODE="verify"
      MODE_SET=1
      ;;
    --with-appdata)
      if ((VARIANT_SET)); then
        usage >&2
        exit 2
      fi
      VARIANT="m54"
      VARIANT_SET=1
      ;;
    --with-package-store)
      if ((VARIANT_SET)); then
        usage >&2
        exit 2
      fi
      VARIANT="apk-install0"
      VARIANT_SET=1
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
  shift
done

GENERATOR_FLAG=""
case "$VARIANT" in
  m25)
    DEFAULT_IMAGE_PATH="$WORKSPACE_ROOT/target/bndroid-storage-m25.raw"
    ;;
  m54)
    DEFAULT_IMAGE_PATH="$WORKSPACE_ROOT/target/bndroid-storage-m54.raw"
    GENERATOR_FLAG="--with-appdata"
    ;;
  apk-install0)
    DEFAULT_IMAGE_PATH="$WORKSPACE_ROOT/target/bndroid-storage-apk-install0.raw"
    GENERATOR_FLAG="--with-package-store"
    ;;
esac
IMAGE_PATH="${BNDROID_STORAGE_IMAGE:-$DEFAULT_IMAGE_PATH}"

for tool in python3 cmp mktemp cp mv mkdir dirname rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required to build the Bndroid storage image." >&2
    exit 1
  fi
done
if [[ ! -f "$GENERATOR" ]]; then
  echo "Bndroid storage image generator is missing: $GENERATOR" >&2
  exit 1
fi
if [[ "$IMAGE_PATH" == *$'\n'* ]]; then
  echo "Storage image paths containing newlines are unsupported." >&2
  exit 2
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-storage-$VARIANT.XXXXXX")"
REFERENCE_IMAGE="$TMP_DIR/reference.raw"
OUTPUT_TMP=""

cleanup() {
  if [[ -n "$OUTPUT_TMP" ]]; then
    rm -f "$OUTPUT_TMP"
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run_generator() {
  local command="$1"
  local image="$2"
  if [[ -n "$GENERATOR_FLAG" ]]; then
    python3 "$GENERATOR" "$command" "$image" "$GENERATOR_FLAG"
  else
    python3 "$GENERATOR" "$command" "$image"
  fi
}

run_generator build "$REFERENCE_IMAGE"

if [[ "$MODE" == "build" ]]; then
  IMAGE_DIR="$(dirname -- "$IMAGE_PATH")"
  mkdir -p "$IMAGE_DIR"
  OUTPUT_TMP="$(mktemp "$IMAGE_DIR/.bndroid-storage-$VARIANT.XXXXXX")"
  cp "$REFERENCE_IMAGE" "$OUTPUT_TMP"
  mv -f "$OUTPUT_TMP" "$IMAGE_PATH"
  OUTPUT_TMP=""
elif [[ ! -f "$IMAGE_PATH" ]]; then
  echo "Storage image does not exist: $IMAGE_PATH" >&2
  exit 1
fi

if ! cmp -s "$REFERENCE_IMAGE" "$IMAGE_PATH"; then
  echo "Storage image differs byte-for-byte from the deterministic $VARIANT fixture: $IMAGE_PATH" >&2
  exit 1
fi

run_generator verify "$IMAGE_PATH"
run_generator marker "$IMAGE_PATH"
