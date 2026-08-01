#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-runtime-m25.raw}"
QEMU_DISPLAY="${BNDROID_QEMU_DISPLAY:-default}"
RESET_STORAGE="${BNDROID_STORAGE_RESET:-0}"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "qemu-system-aarch64 not found. Install QEMU first, then rerun this script."
  exit 1
fi

"$SCRIPT_DIR/build-kernel.sh"
if [[ "$RESET_STORAGE" != "0" && "$RESET_STORAGE" != "1" ]]; then
  echo "BNDROID_STORAGE_RESET must be 0 or 1." >&2
  exit 2
fi
if [[ ! -f "$STORAGE_IMAGE" || "$RESET_STORAGE" == "1" ]]; then
  BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern writable-persistent

exec qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 128M \
  -display "$QEMU_DISPLAY" \
  -monitor none \
  -nic none \
  -serial stdio \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on
