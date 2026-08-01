#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$SCRIPT_DIR/build_multipackage_storage_image.py"
DEFAULT_IMAGE_PATH="$WORKSPACE_ROOT/target/bndroid-storage-multipackage4.raw"
IMAGE_PATH="${BNDROID_STORAGE_IMAGE:-$DEFAULT_IMAGE_PATH}"

MODE=build
case "${1:-}" in
  "")
    ;;
  --verify)
    MODE=verify
    ;;
  *)
    echo "Usage: build-multipackage-storage-image.sh [--verify]" >&2
    exit 2
    ;;
esac

mkdir -p "$(dirname -- "$IMAGE_PATH")"
if [[ "$MODE" == build ]]; then
  python3 "$GENERATOR" build "$IMAGE_PATH"
else
  python3 "$GENERATOR" verify "$IMAGE_PATH"
fi
python3 "$GENERATOR" marker "$IMAGE_PATH"
