#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

usage() {
  cat <<'EOF'
Usage: ./scripts/run-mobile-ui.sh [--tour] [--headless] [--skip-build]

Launch the interactive 720x1600 phone demo using a temporary disk overlay.
  --tour        Walk through Home, Calculator, Settings and AndroidBox;
                retain screenshots and leave the window open at Home.
  --headless    Run the tour without a window, then exit (for smoke checks).
  --skip-build  Reuse the previously built demo kernel and storage fixture.
  --help        Show this help without building or starting QEMU.

Drag upward on Lock to unlock; click dock icons; swipe up on Home for All apps.
Click the bottom bar to return Home. Close the window or press Ctrl-C to stop.
Disk changes and appearance settings are discarded when the demo exits.
No Android SDK is required: the bounded AndroidBox APK is included locally.
EOF
}

DEMO_ARGS=()
HEADLESS=0
while (($#)); do
  case "$1" in
    --tour) DEMO_ARGS+=(--tour) ;;
    --headless) HEADLESS=1; DEMO_ARGS+=(--headless --tour) ;;
    --skip-build) export BNDROID_SKIP_BUILD=1 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

for tool in qemu-system-aarch64 python3 mktemp mkdir; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot launch the local mobile UI preview." >&2
    exit 1
  fi
done

# Keep this DEX-enabled kernel separate from the pure-mobile acceptance gate.
TARGET_ROOT="${BNDROID_MOBILE_TARGET_DIR:-$WORKSPACE_ROOT/target/mobile-ui-demo-build}"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
STORAGE_IMAGE="$TARGET_ROOT/bndroid-storage-m25.raw"
if [[ "$HEADLESS" == 1 ]]; then
  QEMU_DISPLAY=none
elif [[ -n "${BNDROID_QEMU_DISPLAY:-}" ]]; then
  QEMU_DISPLAY="$BNDROID_QEMU_DISPLAY"
elif [[ "$(uname -s)" == Darwin ]]; then
  QEMU_DISPLAY=cocoa,show-cursor=on,zoom-to-fit=on
else
  QEMU_DISPLAY=gtk,show-cursor=on
fi
mkdir -p "$WORKSPACE_ROOT/target/mobile-ui"
ARTIFACT_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/mobile-ui/demo.XXXXXX")"
SERIAL_LOG="$ARTIFACT_DIR/serial.log"
QMP_SOCKET="${BNDROID_QMP_SOCKET:-$ARTIFACT_DIR/qmp.sock}"

# Never unlink another running demo's control socket.
python3 "$SCRIPT_DIR/mobile_ui_demo.py" --validate-socket "$QMP_SOCKET"

if [[ "${BNDROID_SKIP_BUILD:-0}" != "1" ]]; then
  echo "Building the offline mobile + AndroidBox demo..."
  CARGO_TARGET_DIR="$TARGET_ROOT" \
    BNDROID_PROFILE=release \
    BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    "$SCRIPT_DIR/build-kernel.sh" >"$ARTIFACT_DIR/build.log" 2>&1 || {
      tail -n 60 "$ARTIFACT_DIR/build.log" >&2
      echo "Build failed. Full log: $ARTIFACT_DIR/build.log" >&2
      exit 1
    }
  BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" \
    "$SCRIPT_DIR/build-storage-image.sh" >"$ARTIFACT_DIR/storage-build.log" 2>&1 || {
      cat "$ARTIFACT_DIR/storage-build.log" >&2
      exit 1
    }
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
: >"$SERIAL_LOG"

echo "Launching Bndroid with a temporary disk. Evidence: $ARTIFACT_DIR"

# Bash 3.2 treats an empty array as unset under `set -u`. Append the mandatory
# arguments before expansion so the ordinary no-option launch works on macOS.
DEMO_ARGS+=(--artifacts "$ARTIFACT_DIR" --socket "$QMP_SOCKET")
exec python3 "$SCRIPT_DIR/mobile_ui_demo.py" "${DEMO_ARGS[@]}" -- \
  qemu-system-aarch64 \
  -name "Bndroid Mobile UI Preview" \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 256M \
  -display "$QEMU_DISPLAY" \
  -monitor none \
  -nic none \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -rtc base=localtime,clock=host \
  -serial "file:$SERIAL_LOG" \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on
