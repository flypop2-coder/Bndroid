#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 mkdir rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot launch the local mobile UI preview." >&2
    exit 1
  fi
done

TARGET_ROOT="${BNDROID_MOBILE_TARGET_DIR:-$WORKSPACE_ROOT/target/mobile-ui-build}"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
STORAGE_IMAGE="$TARGET_ROOT/bndroid-storage-m25.raw"
QEMU_DISPLAY="${BNDROID_QEMU_DISPLAY:-cocoa,show-cursor=on,zoom-to-fit=on}"
SERIAL_LOG="$WORKSPACE_ROOT/target/mobile-ui/demo-serial.log"
QMP_SOCKET="${BNDROID_QMP_SOCKET:-}"
QMP_ARGS=()

if [[ -n "$QMP_SOCKET" ]]; then
  case "$QMP_SOCKET" in
    "$WORKSPACE_ROOT"/target/mobile-ui/*.sock) ;;
    *)
      echo "BNDROID_QMP_SOCKET must be a .sock path under target/mobile-ui." >&2
      exit 2
      ;;
  esac
  rm -f "$QMP_SOCKET"
  QMP_ARGS=(-qmp "unix:$QMP_SOCKET,server=on,wait=off")
fi

if [[ "${BNDROID_SKIP_BUILD:-0}" != "1" ]]; then
  CARGO_TARGET_DIR="$TARGET_ROOT" \
    BNDROID_PROFILE=release \
    BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    "$SCRIPT_DIR/build-kernel.sh" >/dev/null
  BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" \
    "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
fi

if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "Mobile preview kernel is missing: $KERNEL_IMAGE" >&2
  exit 1
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi

build_storage_qemu_args "$STORAGE_IMAGE" modern writable-temporary
mkdir -p "$WORKSPACE_ROOT/target/mobile-ui"
: >"$SERIAL_LOG"

echo "Launching the local Bndroid mobile UI preview."
echo "Close the QEMU window to stop it. Serial log: $SERIAL_LOG"

exec qemu-system-aarch64 \
  -name "Bndroid Mobile UI Preview" \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 256M \
  -display "$QEMU_DISPLAY" \
  -monitor none \
  -nic none \
  "${QMP_ARGS[@]}" \
  -rtc base=localtime,clock=host \
  -serial "file:$SERIAL_LOG" \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on
