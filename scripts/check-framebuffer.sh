#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp cp tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the framebuffer." >&2
    exit 1
  fi
done

"$SCRIPT_DIR/build-kernel.sh" >/dev/null
"$SCRIPT_DIR/build-storage-image.sh" >/dev/null

PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-10}"
EXPECTED_KERNEL_DIGEST="0x6ef9c2b7d15fde25"
EXPECTED_PIXEL_SHA256="0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac"

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-framebuffer.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
SCREENSHOT="$TMP_DIR/screen.ppm"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 \
  -smp 1 \
  -m 128M \
  -display none \
  -monitor none \
  -nic none \
  -serial "file:$SERIAL_LOG" \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -no-reboot \
  -kernel "$KERNEL_IMAGE" \
  -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  >"$QEMU_LOG" 2>&1 &
QEMU_PID=$!

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  if grep -Eq 'fatal exception:|kernel panic:|boot error:|FRAMEBUFFER_FAIL:' "$NORMALIZED_LOG"; then
    cat "$NORMALIZED_LOG"
    cat "$QEMU_LOG" >&2
    echo "Kernel failed before publishing ramfb." >&2
    exit 1
  fi
  if grep -Eq "^FRAMEBUFFER_OK transport=ramfb fw_cfg=mmio fw_cfg_base=0x[0-9a-f]+ fw_cfg_bytes=24 features=0x3 dma=1 dma_ops=3 directory_files=[1-9][0-9]* selector=0x[0-9a-f]{4} width=320 height=480 stride=1280 format=XRGB8888 bytes=614400 address=0x[0-9a-f]+ page_aligned=1 digest=${EXPECTED_KERNEL_DIGEST} sample_status=0x002e66f5 sample_cyan=0x0006b6d4 sample_purple=0x008b5cf6 sample_green=0x0022c55e configured=1 screenshot_verified=0 input=0 compositor=1$" "$NORMALIZED_LOG" \
    && grep -Eq "^COMPOSITOR_READY layers=2 scene=opaque cursor=alpha scene_bytes=614400 scanout_bytes=614400 scene_address=0x[0-9a-f]+ scanout_address=0x[0-9a-f]+ page_aligned=1 scene_digest=${EXPECTED_KERNEL_DIGEST} scanout_digest=${EXPECTED_KERNEL_DIGEST} cursor_hidden=1 dirty_rect=1 dynamic_redraw=1 input_bound=0$" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    cat "$NORMALIZED_LOG"
    cat "$QEMU_LOG" >&2
    echo "QEMU exited before ramfb was configured." >&2
    exit 1
  fi
  sleep 0.05
done

if ! grep -q '^FRAMEBUFFER_OK ' "$NORMALIZED_LOG"; then
  cat "$NORMALIZED_LOG"
  cat "$QEMU_LOG" >&2
  echo "Timed out waiting for exact FRAMEBUFFER_OK evidence." >&2
  exit 1
fi
if [[ "$(grep -c '^FRAMEBUFFER_OK ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  cat "$NORMALIZED_LOG"
  echo "Expected exactly one FRAMEBUFFER_OK marker." >&2
  exit 1
fi

python3 - "$QMP_SOCKET" "$SCREENSHOT" <<'PY'
import json
import socket
import sys
import time

socket_path, screenshot = sys.argv[1:]
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(200):
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available")

stream = connection.makefile("rwb", buffering=0)
greeting = json.loads(stream.readline())
if "QMP" not in greeting:
    raise SystemExit("invalid QMP greeting")

def command(payload):
    stream.write(json.dumps(payload).encode("ascii") + b"\n")
    while True:
        line = stream.readline()
        if not line:
            raise SystemExit("QMP connection closed")
        response = json.loads(line)
        if "error" in response:
            raise SystemExit(f"QMP error: {response['error']}")
        if "return" in response:
            return response["return"]

command({"execute": "qmp_capabilities"})
command({
    "execute": "screendump",
    "arguments": {"filename": screenshot, "format": "ppm"},
})
connection.close()
PY

for _ in {1..100}; do
  [[ -s "$SCREENSHOT" ]] && break
  sleep 0.02
done
if [[ ! -s "$SCREENSHOT" ]]; then
  echo "QEMU did not produce a framebuffer screenshot." >&2
  exit 1
fi

python3 - "$SCREENSHOT" "$EXPECTED_PIXEL_SHA256" <<'PY'
from collections import Counter
from hashlib import sha256
from pathlib import Path
import sys

path = Path(sys.argv[1])
expected_sha = sys.argv[2]
payload = path.read_bytes()
header = payload.split(b"\n", 3)
if len(header) != 4 or header[:3] != [b"P6", b"320 480", b"255"]:
    raise SystemExit("QEMU screenshot is not exact 320x480 binary PPM")
pixels = header[3]
if len(pixels) != 320 * 480 * 3:
    raise SystemExit("QEMU screenshot pixel payload is truncated or oversized")
actual_sha = sha256(pixels).hexdigest()
if actual_sha != expected_sha:
    raise SystemExit(f"framebuffer pixel digest mismatch: {actual_sha}")

expected_counts = {
    (16, 22, 45): 48384,
    (30, 41, 59): 28544,
    (6, 182, 212): 12672,
    (139, 92, 246): 12672,
    (34, 197, 94): 12672,
    (29, 78, 216): 9984,
    (248, 250, 252): 9856,
    (31, 41, 55): 9856,
    (46, 102, 245): 8960,
}
counts = Counter(tuple(pixels[offset:offset + 3]) for offset in range(0, len(pixels), 3))
if counts != expected_counts:
    raise SystemExit(f"framebuffer color histogram mismatch: {counts}")

samples = {
    (0, 0): (46, 102, 245),
    (0, 100): (16, 22, 45),
    (50, 100): (248, 250, 252),
    (60, 100): (29, 78, 216),
    (100, 160): (6, 182, 212),
    (100, 230): (139, 92, 246),
    (100, 320): (34, 197, 94),
    (160, 455): (31, 41, 55),
    (160, 460): (248, 250, 252),
}
for (x, y), expected in samples.items():
    offset = (y * 320 + x) * 3
    actual = tuple(pixels[offset:offset + 3])
    if actual != expected:
        raise SystemExit(f"pixel {(x, y)} mismatch: {actual}")
PY

mkdir -p "$WORKSPACE_ROOT/target"
cp "$SCREENSHOT" "$WORKSPACE_ROOT/target/bndroid-m26-screen.ppm"
printf '%s\n' "FRAMEBUFFER_SCREENSHOT_OK qemu=1 width=320 height=480 stride=1280 format=RGB888 pixels=153600 colors=9 pixel_sha256=$EXPECTED_PIXEL_SHA256 kernel_digest=$EXPECTED_KERNEL_DIGEST samples=9 full_frame=1 headless_capture=1 input_devices=2 compositor_ready=1 interaction=0"
