#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify compositor interaction." >&2
    exit 1
  fi
done

BNDROID_KERNEL_FEATURES=surface-trace-evidence "$SCRIPT_DIR/build-kernel.sh" >/dev/null
"$SCRIPT_DIR/build-storage-image.sh" >/dev/null

PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-15}"
BASELINE_SHA256="0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac"
AFTER_SHA256="f24699248bcdfe5069fa13a40ef7990ba7a647cfe0ac818c04f671047e748f82"
BOOT_DIGEST="0x6ef9c2b7d15fde25"
PRESSED_DIGEST="0x61880733dfa7cfb1"
FINAL_DIGEST="0x09f8448817d7a8e1"

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-compositor.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
BASELINE_PPM="$TMP_DIR/baseline.ppm"
AFTER_PPM="$TMP_DIR/after.ppm"
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

show_failure() {
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  cat "$NORMALIZED_LOG"
  cat "$QEMU_LOG" >&2
}

reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|FRAMEBUFFER_FAIL:|INPUT_FAIL:|input (probe|install|IRQ validation|event|stats|event validation) failed|pointer (event|mapping|stats|tracker|info) failed|pointer event validation failed|compositor (update|interaction) failed' "$NORMALIZED_LOG"; then
    show_failure
    echo "Kernel emitted a fatal display, pointer, or boot failure." >&2
    exit 1
  fi
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
  reject_failure
  if grep -Fxq 'POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1' "$NORMALIZED_LOG" \
    && grep -Eq '^SURFACE_HANDOFF_OK from=kernel-fallback to=userspace-bound caller=el0 pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25$' "$NORMALIZED_LOG" \
    && grep -Eq '^USER_SURFACE_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 commit=1 mode=full .* scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25 .* input_pending=0 .* dma_barrier=1$' "$NORMALIZED_LOG" \
    && grep -q '^BOOT_OK:' "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the compositor and tablet became ready." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if [[ "$(grep -c '^SURFACE_HANDOFF_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^USER_SURFACE_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^POINTER_READY ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "Timed out waiting for exact userspace handoff/tablet readiness." >&2
  exit 1
fi

python3 - "$QMP_SOCKET" "$BASELINE_PPM" <<'PY'
import json
import socket
import sys
import time

socket_path, baseline = sys.argv[1:]
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(300):
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
command({"execute": "screendump", "arguments": {"filename": baseline, "format": "ppm"}})
command({
    "execute": "input-send-event",
    "arguments": {"events": [
        {"type": "abs", "data": {"axis": "x", "value": 1234}},
        {"type": "abs", "data": {"axis": "y", "value": 23456}},
    ]},
})
time.sleep(0.05)
command({
    "execute": "input-send-event",
    "arguments": {"events": [{"type": "btn", "data": {"down": True, "button": "touch"}}]},
})
time.sleep(0.05)
command({
    "execute": "input-send-event",
    "arguments": {"events": [{"type": "btn", "data": {"down": False, "button": "touch"}}]},
})
connection.close()
PY

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -q '^POINTER_EVENT_OK ' "$NORMALIZED_LOG" \
    && grep -q '^COMPOSITOR_UPDATE_OK ' "$NORMALIZED_LOG" \
    && grep -Eq '^USER_INPUT_READ_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* sequence=3 x=12 y=342 pressed=0 pending=0 enqueued=3 dequeued=3 high_water=[1-9][0-9]* coalesced=0$' "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before publishing touch/compositor evidence." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -c '^POINTER_EVENT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^COMPOSITOR_UPDATE_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)" != "3" ]] \
  || [[ "$(grep -c '^USER_SURFACE_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^SURFACE_DEGRADED ' "$NORMALIZED_LOG" || true)" != "0" ]]; then
  show_failure
  echo "The cursor route did not deliver exactly three userspace samples without a scene commit." >&2
  exit 1
fi

pointer_line="$(grep '^POINTER_EVENT_OK ' "$NORMALIZED_LOG")"
pointer_regex='^POINTER_EVENT_OK device=tablet sequence=abs_x/abs_y/syn/touch_down/syn/touch_up/syn events=7 raw_x=1234 raw_y=23456 pixel_x=12 pixel_y=342 touch_down=1 touch_up=1 syn=3 samples=3 completions=7 delivered=7 recycled=7 buffered=0 max_buffered=([1-7]) avail_idx=15 used_idx=7 irq_entries=([1-9][0-9]*) queue_irqs=([1-9][0-9]*) config_irqs=0 spurious=0 dropped=0 invalid=0 descriptor_reuse=1$'
if [[ ! "$pointer_line" =~ $pointer_regex ]]; then
  show_failure
  echo "POINTER_EVENT_OK did not prove the exact lossless seven-event contract." >&2
  exit 1
fi
max_buffered="${BASH_REMATCH[1]}"
irq_entries="${BASH_REMATCH[2]}"
queue_irqs="${BASH_REMATCH[3]}"

compositor_line="$(grep '^COMPOSITOR_UPDATE_OK ' "$NORMALIZED_LOG")"
compositor_regex="^COMPOSITOR_UPDATE_OK layers=2 updates=3 dirty=12/342/12/22 restored=264 blended=122 boot_digest=${BOOT_DIGEST} pressed_digest=${PRESSED_DIGEST} final_digest=${FINAL_DIGEST} digest_changed=1 press_feedback=1 cursor_visible=1 cursor_pressed=0 alpha=1 dirty_rect=1 input=touch$"
if [[ ! "$compositor_line" =~ $compositor_regex ]]; then
  show_failure
  echo "COMPOSITOR_UPDATE_OK did not prove the exact three-redraw dirty rectangle." >&2
  exit 1
fi

python3 - "$QMP_SOCKET" "$AFTER_PPM" <<'PY'
import json
import socket
import sys
import time

socket_path, after = sys.argv[1:]
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(300):
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available for final screenshot")
stream = connection.makefile("rwb", buffering=0)
if "QMP" not in json.loads(stream.readline()):
    raise SystemExit("invalid QMP greeting")

def command(payload):
    stream.write(json.dumps(payload).encode("ascii") + b"\n")
    while True:
        response = json.loads(stream.readline())
        if "error" in response:
            raise SystemExit(f"QMP error: {response['error']}")
        if "return" in response:
            return response["return"]

command({"execute": "qmp_capabilities"})
command({"execute": "screendump", "arguments": {"filename": after, "format": "ppm"}})
connection.close()
PY

python3 - "$BASELINE_PPM" "$AFTER_PPM" "$BASELINE_SHA256" "$AFTER_SHA256" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

before_path, after_path, before_sha, after_sha = sys.argv[1:]

def pixels(path):
    parts = Path(path).read_bytes().split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"320 480", b"255"]:
        raise SystemExit(f"{path} is not an exact 320x480 binary PPM")
    if len(parts[3]) != 320 * 480 * 3:
        raise SystemExit(f"{path} has a truncated or oversized pixel payload")
    return parts[3]

before = pixels(before_path)
after = pixels(after_path)
if sha256(before).hexdigest() != before_sha:
    raise SystemExit("baseline framebuffer digest changed")
if sha256(after).hexdigest() != after_sha:
    raise SystemExit("interactive framebuffer digest changed")

changed = []
for index in range(320 * 480):
    offset = index * 3
    if before[offset:offset + 3] != after[offset:offset + 3]:
        changed.append((index % 320, index // 320))
if len(changed) != 122:
    raise SystemExit(f"expected 122 changed pixels, found {len(changed)}")
xs = [point[0] for point in changed]
ys = [point[1] for point in changed]
bbox = (min(xs), min(ys), max(xs), max(ys))
if bbox != (12, 342, 21, 363):
    raise SystemExit(f"interactive diff bounding box changed: {bbox}")
if any(not (12 <= x < 24 and 342 <= y < 364) for x, y in changed):
    raise SystemExit("interactive pixels escaped the declared dirty rectangle")
colors = {tuple(after[offset:offset + 3]) for offset in range(0, len(after), 3)}
if len(colors) != 11:
    raise SystemExit(f"expected 11 final colors, found {len(colors)}")
PY

mkdir -p "$WORKSPACE_ROOT/target"
cp "$BASELINE_PPM" "$WORKSPACE_ROOT/target/bndroid-m27-before.ppm"
cp "$AFTER_PPM" "$WORKSPACE_ROOT/target/bndroid-m27-after.ppm"
printf '%s\n' "COMPOSITOR_INTERACTION_OK qemu=1 transport=ramfb input=virtio-tablet surface_owner=userspace input_sequence=1..3 queue_drained=1 scene_commits=0 qmp_commands=3 events=7 samples=3 redraws=3 dirty=12/342/12/22 restored=264 blended=122 diff_pixels=122 diff_bbox=12/342/21/363 baseline_sha256=$BASELINE_SHA256 after_sha256=$AFTER_SHA256 colors=11 boot_digest=$BOOT_DIGEST pressed_digest=$PRESSED_DIGEST final_digest=$FINAL_DIGEST max_buffered=$max_buffered irq_entries=$irq_entries queue_irqs=$queue_irqs completions=7 delivered=7 recycled=7 avail_idx=15 used_idx=7 dropped=0 invalid=0 alpha=1 press_feedback=1 full_frame_capture=1"
