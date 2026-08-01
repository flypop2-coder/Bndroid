#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep kill cp tail; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the userspace-driven clickable UI." >&2
    exit 1
  fi
done

BNDROID_USERSPACE_FEATURES=ui-stale-present-evidence \
  BNDROID_KERNEL_FEATURES=surface-trace-evidence,ui-stale-present-evidence \
  "$SCRIPT_DIR/build-kernel.sh" >/dev/null
"$SCRIPT_DIR/build-storage-image.sh" >/dev/null

PROFILE="${BNDROID_PROFILE:-debug}"
KERNEL_IMAGE="$WORKSPACE_ROOT/target/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-15}"
BASELINE_SHA256="0adbceee84974eaee5af0d0105020417cbce2c71a7cc46e0f56885239c9b45ac"
AFTER_SHA256="62b15db5e54d3bbca7bd23e74a60e7fc66d2566e2094be7718e7c8f1a0ebc5dd"
SCENE_DIGEST="0xf38fcac0ca9b43a5"
SCANOUT_DIGEST="0x9ea54989d1a75b09"
BOOT_MARKER="BOOT_OK: M32 transferable graphics buffers, M25 durable data records, and M20 multi-session services verified"
PROCESS_IMAGE_MARKER="PROCESS_IMAGE_OK abi=23 init=1 spawn_sequence=2/3/4/2/4/5/6/7 manager_reused=1 client_image_reused=1 distinct_catalog=1"
UI_RUNTIME_MARKER="UI_RUNTIME_OK protocol=3 server_pid=0x0000000100000006 launcher_pid=0x0000000100000007 app_pid=0x0000000100000008 distinct=1 ui_pairs=2 surface_owner=server launcher_surface=0 app_surface=0 server_handles=4 launcher_handles=1 app_handles=2 dual_clients=1 focus_routed=1 ready_before_present=1 single_outstanding=1 first_present_ack=1 graphics_buffer=1 resident=1"
GRAPHICS_BUFFER_MARKER="GRAPHICS_BUFFER_OK abi=23 format=XRGB8888 width=208 height=368 logical_bytes=306176 backing_bytes=307200 slots=2 created=1 write_calls=1 write_bytes=4 presents=0 handles=2 producer_pid=0x0000000100000008 consumer_pid=0x0000000100000006 slot=0 allocation_generation=1 producer_rights=0x0000000f server_rights=0x00000009 owners_valid=1 rights_valid=1 generation_qualified=1 transferred=1 writable=1 copy_present=1"

count_surface_commits() {
  grep -Ec '^(USER_SURFACE_COMMIT_OK|USER_SURFACE_BUFFER_COMMIT_OK) ' "$1" || true
}

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-ui.XXXXXX")"
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
  if grep -Eqi 'fatal exception:|kernel panic:|boot error:|FRAMEBUFFER_FAIL:|INPUT_FAIL:|input (probe|install|IRQ validation|event|stats|event validation) failed|pointer (event|mapping|stats|tracker|info) failed|pointer event validation failed|compositor (update|interaction) failed|UI (scene commit|input stats|interaction) failed' "$NORMALIZED_LOG"; then
    show_failure
    echo "Kernel emitted a fatal UI, display, input, or boot failure." >&2
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
    && grep -Eq '^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=[1-9][0-9]* session=[1-9][0-9]* handle=[1-9][0-9]* rights=0x00000103 unique=1 duplicate=0 transferable=0 input_capacity=64$' "$NORMALIZED_LOG" \
    && grep -Eq '^SURFACE_HANDOFF_OK from=kernel-fallback to=userspace-bound caller=el0 pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=1 scene_digest=0x6ef9c2b7d15fde25 scanout_digest=0x6ef9c2b7d15fde25$' "$NORMALIZED_LOG" \
    && grep -Fqx "$PROCESS_IMAGE_MARKER" "$NORMALIZED_LOG" \
    && grep -Fqx "$UI_RUNTIME_MARKER" "$NORMALIZED_LOG" \
    && grep -Fqx "$GRAPHICS_BUFFER_MARKER" "$NORMALIZED_LOG" \
    && grep -Fqx "$BOOT_MARKER" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the UI and tablet became ready." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if [[ "$(grep -c '^SURFACE_ACQUIRE_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^SURFACE_HANDOFF_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^POINTER_READY ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -Fxc "$PROCESS_IMAGE_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -Fxc "$UI_RUNTIME_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^GRAPHICS_BUFFER_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -Fxc "$GRAPHICS_BUFFER_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "Timed out waiting for the unique userspace surface handoff and tablet readiness." >&2
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
command({"execute": "screendump", "arguments": {"filename": baseline, "format": "ppm"}})
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "abs", "data": {"axis": "x", "value": 1234}},
    {"type": "abs", "data": {"axis": "y", "value": 23456}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": True, "button": "touch"}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": False, "button": "touch"}},
]}})
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
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if [[ "$(grep -c '^POINTER_EVENT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^COMPOSITOR_UPDATE_OK ' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "The M27 cursor sequence did not drain before the UI tap." >&2
  exit 1
fi

python3 - "$QMP_SOCKET" <<'PY'
import json
import socket
import sys
import time

connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(300):
    try:
        connection.connect(sys.argv[1])
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available for UI input")
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
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "abs", "data": {"axis": "x", "value": 16486}},
    {"type": "abs", "data": {"axis": "y", "value": 23456}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": True, "button": "touch"}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": False, "button": "touch"}},
]}})
connection.close()
PY

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq '^USER_INPUT_READ_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* sequence=6 x=160 y=342 pressed=0 pending=0 enqueued=6 dequeued=6 high_water=[1-9][0-9]* coalesced=0$' "$NORMALIZED_LOG" \
    && grep -Eq "^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=3 frame_id=4 commit=4 mode=full buffer_generation=226 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=${SCENE_DIGEST} scanout_digest=${SCANOUT_DIGEST} cursor_preserved=1 dma_barrier=1$" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before publishing UI evidence." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)" != "6" ]] \
  || [[ "$(count_surface_commits "$NORMALIZED_LOG")" != "4" ]] \
  || [[ "$(grep -c '^USER_SURFACE_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "1" ]] \
  || [[ "$(grep -c '^USER_SURFACE_BUFFER_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "3" ]]; then
  show_failure
  echo "The userspace SurfaceServer did not read exactly six samples and commit frames 1..4." >&2
  exit 1
fi

for sample in \
  '1 12 342 0' \
  '2 12 342 1' \
  '3 12 342 0' \
  '4 160 342 0' \
  '5 160 342 1' \
  '6 160 342 0'; do
  read -r sequence x y pressed <<<"$sample"
  if ! grep -Eq "^USER_INPUT_READ_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* sequence=${sequence} x=${x} y=${y} pressed=${pressed} pending=0 enqueued=${sequence} dequeued=${sequence} high_water=[1-9][0-9]* coalesced=0$" "$NORMALIZED_LOG"; then
    show_failure
    echo "Input sample ${sequence} was not normalized and drained exactly once." >&2
    exit 1
  fi
done

if ! grep -Eq '^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=1 frame_id=2 commit=2 mode=full buffer_generation=76 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=0x902b99890e7da7a5 scanout_digest=0x3b4118521589bf09 cursor_preserved=1 dma_barrier=1$' "$NORMALIZED_LOG" \
  || ! grep -Eq '^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=2 frame_id=3 commit=3 mode=full buffer_generation=151 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=0xab2e8e642712aca5 scanout_digest=0x56440d2d2e1ec409 cursor_preserved=1 dma_barrier=1$' "$NORMALIZED_LOG" \
  || ! grep -Eq "^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=3 frame_id=4 commit=4 mode=full buffer_generation=226 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=${SCENE_DIGEST} scanout_digest=${SCANOUT_DIGEST} cursor_preserved=1 dma_barrier=1$" "$NORMALIZED_LOG"; then
  show_failure
  echo "The Settings transition was not the exact three-frame graphics-buffer transaction." >&2
  exit 1
fi

pointer_line="$(grep '^POINTER_EVENT_OK ' "$NORMALIZED_LOG")"
pointer_regex='^POINTER_EVENT_OK .* max_buffered=([1-8]) .* irq_entries=([1-9][0-9]*) queue_irqs=([1-9][0-9]*) .* dropped=0 invalid=0 descriptor_reuse=1$'
if [[ ! "$pointer_line" =~ $pointer_regex ]]; then
  show_failure
  echo "The first seven-event tablet proof was malformed." >&2
  exit 1
fi
max_buffered="${BASH_REMATCH[1]}"
irq_entries="${BASH_REMATCH[2]}"
queue_irqs="${BASH_REMATCH[3]}"

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
    raise SystemExit("settings framebuffer digest changed")
changed = []
for index in range(320 * 480):
    offset = index * 3
    if before[offset:offset + 3] != after[offset:offset + 3]:
        changed.append((index % 320, index // 320))
if len(changed) != 53760:
    raise SystemExit(f"expected 53760 changed pixels, found {len(changed)}")
if any(not (56 <= x < 264 and 64 <= y < 432) for x, y in changed):
    raise SystemExit("UI pixels escaped the declared scene damage")
xs = [point[0] for point in changed]
ys = [point[1] for point in changed]
bbox = (min(xs), min(ys), max(xs), max(ys))
if bbox != (56, 64, 263, 379):
    raise SystemExit(f"settings diff bounding box changed: {bbox}")
colors = {tuple(after[offset:offset + 3]) for offset in range(0, len(after), 3)}
if len(colors) != 13:
    raise SystemExit(f"expected 13 final colors, found {len(colors)}")
PY

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if ! kill -0 "$QEMU_PID" 2>/dev/null; then
  show_failure
  echo "QEMU exited after the settings screenshot." >&2
  exit 1
fi

send_tap() {
  local raw_y="$1"
  python3 - "$QMP_SOCKET" "$raw_y" <<'PY'
import json
import socket
import sys
import time

socket_path, raw_y = sys.argv[1], int(sys.argv[2])
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(300):
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available for navigation input")
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
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "abs", "data": {"axis": "x", "value": 16486}},
    {"type": "abs", "data": {"axis": "y", "value": raw_y}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": True, "button": "touch"}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": False, "button": "touch"}},
]}})
connection.close()
PY
}

send_captured_drag_to_home() {
  python3 - "$QMP_SOCKET" <<'PY'
import json
import socket
import sys
import time

connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(300):
    try:
        connection.connect(sys.argv[1])
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available for captured drag input")
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
# The Settings App owns focus and the pointer is at pixel (160, 342). Capture
# starts inside the App, then crosses into the Home target while still down.
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": True, "button": "touch"}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "abs", "data": {"axis": "x", "value": 16486}},
    {"type": "abs", "data": {"axis": "y", "value": 31502}},
]}})
time.sleep(0.05)
command({"execute": "input-send-event", "arguments": {"events": [
    {"type": "btn", "data": {"down": False, "button": "touch"}},
]}})
connection.close()
PY
}

wait_transition() {
  local generation="$1"
  local target="$2"
  local from="$3"
  local to="$4"
  local reports=$((9 + 3 * (generation - 1)))
  local final_frame expected_scene commit_regex
  case "$generation" in
    2|4|6)
      case "$generation" in
        2) final_frame=5 ;;
        4) final_frame=9 ;;
        6) final_frame=13 ;;
      esac
      expected_scene=0x6ef9c2b7d15fde25
      commit_regex="^USER_SURFACE_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=${final_frame} commit=${final_frame} mode=(full|damage) rects=[0-9]+ local_damage=[0-9]+/[0-9]+/[0-9]+/[0-9]+ global_damage=[0-9]+/[0-9]+/[0-9]+/[0-9]+ raster_writes=[1-9][0-9]* composition=[0-9]+/[0-9]+/[0-9]+/[0-9]+ restored=[1-9][0-9]* blended=[0-9]+ scene_digest=${expected_scene} scanout_digest=0x[0-9a-f]{16} input_enqueued=${reports} input_dequeued=${reports} input_pending=0 input_coalesced=0 cursor_preserved=1 dma_barrier=1$"
      ;;
    3)
      final_frame=8
      expected_scene=0x699b8a5a5e0de0a5
      commit_regex="^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=6 frame_id=8 commit=8 mode=full buffer_generation=451 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=${expected_scene} scanout_digest=0x[0-9a-f]{16} cursor_preserved=1 dma_barrier=1$"
      ;;
    5)
      final_frame=12
      expected_scene=0xfa6b2b128c82caa5
      commit_regex="^USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* producer_pid=[1-9][0-9]* client_frame_id=9 frame_id=12 commit=12 mode=full buffer_generation=676 format=xrgb8888 width=208 height=368 global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 scene_digest=${expected_scene} scanout_digest=0x[0-9a-f]{16} cursor_preserved=1 dma_barrier=1$"
      ;;
    *) echo "Unsupported userspace transition generation: $generation" >&2; exit 2 ;;
  esac
  local input_regex="^USER_INPUT_READ_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* sequence=${reports} x=[0-9]+ y=[0-9]+ pressed=0 pending=0 enqueued=${reports} dequeued=${reports} high_water=[1-9][0-9]* coalesced=0$"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    if grep -Eq "$input_regex" "$NORMALIZED_LOG" \
      && grep -Eq "$commit_regex" "$NORMALIZED_LOG"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      show_failure
      echo "QEMU exited before userspace transition ${generation} (${from} -> ${to}, target ${target})." >&2
      exit 1
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for userspace transition ${generation} (${from} -> ${to}, target ${target})." >&2
  exit 1
}

# Prove press-to-release capture before ordinary navigation. The gesture starts
# in the focused Settings App, crosses the Home target, and releases there. All
# three client-local samples must still be delivered to App, with no focus or
# frame transition; the following paced Home tap proves Launcher remains live.
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
focus_routes_before="$(grep -c '^UI_ROUTE_FOCUS_OK ' "$NORMALIZED_LOG" || true)"
launcher_routes_before="$(grep -c '^UI_ROUTE_INPUT_OK .* receiver_image=launcher ' "$NORMALIZED_LOG" || true)"
send_captured_drag_to_home

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq '^USER_INPUT_READ_OK owner=userspace pid=[1-9][0-9]* session=[1-9][0-9]* sequence=9 x=160 y=460 pressed=0 pending=0 enqueued=9 dequeued=9 high_water=[1-9][0-9]* coalesced=0$' "$NORMALIZED_LOG" \
    && grep -Eq '^UI_ROUTE_INPUT_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* sequence=3 x=160 y=460 pressed=0$' "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before publishing userspace gesture-capture evidence." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
for routed in \
  '1 160 342 1' \
  '2 160 460 1' \
  '3 160 460 0'; do
  read -r sequence x y pressed <<<"$routed"
  if ! grep -Eq "^UI_ROUTE_INPUT_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* sequence=${sequence} x=${x} y=${y} pressed=${pressed}$" "$NORMALIZED_LOG"; then
    show_failure
    echo "Captured App input sample ${sequence} was not routed exactly once." >&2
    exit 1
  fi
done
if [[ "$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)" != "9" ]] \
  || [[ "$(count_surface_commits "$NORMALIZED_LOG")" != "4" ]] \
  || [[ "$(grep -c '^UI_ROUTE_FOCUS_OK ' "$NORMALIZED_LOG" || true)" != "$focus_routes_before" ]] \
  || [[ "$(grep -c '^UI_ROUTE_INPUT_OK .* receiver_image=launcher ' "$NORMALIZED_LOG" || true)" != "$launcher_routes_before" ]]; then
  show_failure
  echo "The captured drag crossed focus, committed a frame, or leaked a sample to Launcher." >&2
  exit 1
fi

# Exercise every advertised target and three independent Home returns. Each
# seven-event tap is drained before the next one so queue capacity is not part
# of the navigation proof.
send_tap 31502
wait_transition 2 home settings home
send_tap 11527
wait_transition 3 phone home phone
send_tap 31502
wait_transition 4 home phone home
send_tap 17547
wait_transition 5 messages home messages
send_tap 31502
wait_transition 6 home messages home

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if ! kill -0 "$QEMU_PID" 2>/dev/null; then
  show_failure
  echo "QEMU exited after the full UI navigation proof." >&2
  exit 1
fi
if [[ "$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)" != "24" ]] \
  || [[ "$(count_surface_commits "$NORMALIZED_LOG")" != "13" ]] \
  || [[ "$(grep -c '^USER_SURFACE_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "4" ]] \
  || [[ "$(grep -c '^USER_SURFACE_BUFFER_COMMIT_OK ' "$NORMALIZED_LOG" || true)" != "9" ]]; then
  show_failure
  echo "Expected exactly 24 normalized samples and 13 ordered userspace frame commits." >&2
  exit 1
fi

python3 - "$NORMALIZED_LOG" <<'PY'
import re
import sys

lines = open(sys.argv[1], encoding="utf-8").read().splitlines()
inputs = [line for line in lines if line.startswith("USER_INPUT_READ_OK ")]
legacy_commits = [line for line in lines if line.startswith("USER_SURFACE_COMMIT_OK ")]
buffer_commits = [line for line in lines if line.startswith("USER_SURFACE_BUFFER_COMMIT_OK ")]
commits = [
    line for line in lines
    if line.startswith(("USER_SURFACE_COMMIT_OK ", "USER_SURFACE_BUFFER_COMMIT_OK "))
]
acquires = [line for line in lines if line.startswith("SURFACE_ACQUIRE_OK ")]
handoffs = [line for line in lines if line.startswith("SURFACE_HANDOFF_OK ")]
if len(acquires) != 1 or len(handoffs) != 1:
    raise SystemExit("surface acquire/handoff was not unique")
if [int(re.search(r" sequence=(\d+) ", line).group(1)) for line in inputs] != list(range(1, 25)):
    raise SystemExit("userspace input sequence was not exactly 1..24")
if [int(re.search(r" frame_id=(\d+) ", line).group(1)) for line in commits] != list(range(1, 14)):
    raise SystemExit("userspace frame sequence was not exactly 1..13")
if [line.startswith("USER_SURFACE_BUFFER_COMMIT_OK ") for line in commits] != [
    False, True, True, True, False, True, True, True, False, True, True, True, False
]:
    raise SystemExit("Launcher/App commit types did not follow the exact 4/9 protocol split")
buffer_pattern = re.compile(
    r"USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=(\d+) session=(\d+) "
    r"producer_pid=(\d+) client_frame_id=(\d+) frame_id=(\d+) commit=(\d+) "
    r"mode=full buffer_generation=(\d+) format=xrgb8888 width=208 height=368 "
    r"global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 "
    r"scene_digest=0x[0-9a-f]{16} scanout_digest=0x[0-9a-f]{16} "
    r"cursor_preserved=1 dma_barrier=1"
)
buffer_matches = [buffer_pattern.fullmatch(line) for line in buffer_commits]
if any(match is None for match in buffer_matches):
    raise SystemExit("graphics-buffer commit evidence was malformed or weakened")
if [int(match.group(4)) for match in buffer_matches] != list(range(1, 10)):
    raise SystemExit("App client-local frame sequence was not exactly 1..9")
if [int(match.group(7)) for match in buffer_matches] != [76 + 75 * index for index in range(9)]:
    raise SystemExit("App buffer generations were not exactly 76..676 in 75-write steps")
if any(match.group(3) == match.group(1) for match in buffer_matches):
    raise SystemExit("graphics-buffer producer unexpectedly matched SurfaceServer")
if len({match.group(3) for match in buffer_matches}) != 1:
    raise SystemExit("graphics-buffer commits crossed producer identity")
identities = {
    (re.search(r" pid=(\d+) ", line).group(1), re.search(r" session=(\d+) ", line).group(1))
    for line in inputs + commits + acquires + handoffs
}
if len(identities) != 1:
    raise SystemExit(f"surface evidence crossed process/session identity: {identities}")
if any(not 1 <= int(re.search(r" high_water=(\d+) ", line).group(1)) <= 64 for line in inputs):
    raise SystemExit("surface input FIFO high-water escaped its fixed 64-slot bound")
if any(not line.endswith(" coalesced=0") for line in inputs):
    raise SystemExit("unexpected input coalescing occurred in the paced navigation proof")
if any(" input_coalesced=0 " not in f" {line} " for line in legacy_commits):
    raise SystemExit("unexpected input coalescing preceded a paced frame commit")
if any(" input_pending=0 " not in f" {line} " for line in legacy_commits):
    raise SystemExit("a frame committed before its input batch drained")
PY

# Exercise one directed App->Home generation-stale Present after the exact
# 13-frame baseline.  The evidence-only userspace feature holds App frame 10
# after SurfaceServer has read generation 8; only that read marker authorizes
# the Home tap.  Focus generation 9 must then cancel the held frame through the
# normal protocol path before generation 10 retries the same local frame.
cancel_count_before="$(grep -c '^UI_ROUTE_PRESENT_CANCELLED_OK ' "$NORMALIZED_LOG" || true)"
if [[ "$cancel_count_before" != "1" ]]; then
  show_failure
  echo "The directed stale-Present proof did not start after the unique bootstrap cancellation." >&2
  exit 1
fi

held_frame=10
held_focus_generation=8
cancel_focus_generation=9
retry_focus_generation=10
final_focus_generation=11
stress_attempts=1

send_tap 11527
held_ready=0
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=launcher receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=app app=phone focus_generation=${held_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=app app=phone focus_generation=${held_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_PRESENT_READ_OK sender_image=app sender_pid=[1-9][0-9]* receiver_image=surface-server receiver_pid=[1-9][0-9]* frame_id=${held_frame} focus_generation=${held_focus_generation} mode=full$" "$NORMALIZED_LOG"; then
    held_ready=1
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the directed App Present was held." >&2
    exit 1
  fi
  sleep 0.02
done
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ((held_ready != 1)) \
  || [[ "$(grep -Ec "^UI_ROUTE_PRESENT_READ_OK sender_image=app sender_pid=[1-9][0-9]* receiver_image=surface-server receiver_pid=[1-9][0-9]* frame_id=${held_frame} focus_generation=${held_focus_generation} mode=full$" "$NORMALIZED_LOG" || true)" != "1" ]] \
  || grep -Eq "^UI_ROUTE_(PRESENTED|PRESENT_CANCELLED)_OK .* receiver_image=app .* frame_id=${held_frame}( |$)" "$NORMALIZED_LOG"; then
  show_failure
  echo "App frame 10 was not uniquely held at focus generation 8 before Home input." >&2
  exit 1
fi

send_tap 31502
cancel_ready=0
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=launcher receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=launcher app=none focus_generation=${cancel_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=launcher app=none focus_generation=${cancel_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_PRESENT_CANCELLED_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=${held_frame} focus_generation=${cancel_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq '^UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=launcher receiver_pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=5 commit=14$' "$NORMALIZED_LOG"; then
    cancel_ready=1
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the held App Present was cancelled." >&2
    exit 1
  fi
  sleep 0.02
done
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ((cancel_ready != 1)); then
  show_failure
  echo "Timed out waiting for the directed generation-stale App cancellation." >&2
  exit 1
fi

cancel_line="$(grep '^UI_ROUTE_PRESENT_CANCELLED_OK ' "$NORMALIZED_LOG" | tail -n 1 || true)"
cancel_regex='^UI_ROUTE_PRESENT_CANCELLED_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=([1-9][0-9]*) focus_generation=([1-9][0-9]*)$'
if [[ "$cancel_line" =~ $cancel_regex ]]; then
  cancel_frame_id="${BASH_REMATCH[1]}"
  cancel_focus_generation="${BASH_REMATCH[2]}"
else
  show_failure
  echo "Generation-stale App cancellation marker was malformed or misrouted." >&2
  exit 1
fi
if [[ "$cancel_frame_id" != "$held_frame" || "$cancel_focus_generation" != "9" ]]; then
  show_failure
  echo "Directed cancellation did not preserve frame 10 at Home focus generation 9." >&2
  exit 1
fi

retry_tail_frame=$((cancel_frame_id + 2))
send_tap 11527
retry_ready=0
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=app app=phone focus_generation=${retry_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_PRESENT_READ_OK sender_image=app sender_pid=[1-9][0-9]* receiver_image=surface-server receiver_pid=[1-9][0-9]* frame_id=${cancel_frame_id} focus_generation=${retry_focus_generation} mode=full$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=${retry_tail_frame} commit=17$" "$NORMALIZED_LOG"; then
    retry_ready=1
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited during the same-frame retry." >&2
    exit 1
  fi
  sleep 0.02
done
if ((retry_ready != 1)); then
  show_failure
  echo "Timed out waiting for the generation-10 same-frame retry." >&2
  exit 1
fi

final_launcher_frame=6
send_tap 31502
final_ready=0
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=launcher receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=launcher app=none focus_generation=${final_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=app receiver_pid=[1-9][0-9]* session=[1-9][0-9]* active_client=launcher app=none focus_generation=${final_focus_generation}$" "$NORMALIZED_LOG" \
    && grep -Eq "^UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9][0-9]* receiver_image=launcher receiver_pid=[1-9][0-9]* session=[1-9][0-9]* frame_id=${final_launcher_frame} commit=18$" "$NORMALIZED_LOG"; then
    final_ready=1
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    show_failure
    echo "QEMU exited before the final Home frame." >&2
    exit 1
  fi
  sleep 0.02
done
if ((final_ready != 1)); then
  show_failure
  echo "Timed out waiting for the final generation-11 Home frame." >&2
  exit 1
fi

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
final_input_count="$(grep -c '^USER_INPUT_READ_OK ' "$NORMALIZED_LOG" || true)"
final_commit_count="$(count_surface_commits "$NORMALIZED_LOG")"
cancel_count_after="$(grep -c '^UI_ROUTE_PRESENT_CANCELLED_OK ' "$NORMALIZED_LOG" || true)"
if ((final_input_count != 36 \
  || cancel_count_after - cancel_count_before != 1 \
  || final_commit_count != 18)); then
  show_failure
  echo "Directed focus cancellation/retry did not end at 36 inputs, one cancellation, and 18 commits." >&2
  exit 1
fi

python3 - "$NORMALIZED_LOG" "$cancel_frame_id" "$cancel_focus_generation" <<'PY'
import re
import sys

path, raw_frame, raw_generation = sys.argv[1:]
frame = int(raw_frame)
generation = int(raw_generation)
lines = open(path, encoding="utf-8").read().splitlines()
if (frame, generation) != (10, 9):
    raise SystemExit(f"directed cancellation identity changed: {(frame, generation)}")

def index(pattern, start=0, unique=False):
    compiled = re.compile(pattern)
    matches = [
        position
        for position in range(start, len(lines))
        if compiled.fullmatch(lines[position])
    ]
    if matches and (not unique or len(matches) == 1):
        return matches[0]
    if len(matches) > 1:
        raise SystemExit(f"duplicate ordered UI route evidence: {pattern}")
    raise SystemExit(f"missing ordered UI route evidence: {pattern}")

cancel = index(
    rf"UI_ROUTE_PRESENT_CANCELLED_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* frame_id={frame} "
    rf"focus_generation={generation}",
    unique=True,
)
old_read = index(
    rf"UI_ROUTE_PRESENT_READ_OK sender_image=app sender_pid=[1-9]\d* "
    rf"receiver_image=surface-server receiver_pid=[1-9]\d* frame_id={frame} "
    rf"focus_generation={generation - 1} mode=full",
    unique=True,
)
app_home_focus = index(
    rf"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* active_client=launcher "
    rf"app=none focus_generation={generation}",
    unique=True,
)
launcher_home_focus = index(
    rf"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=launcher receiver_pid=[1-9]\d* session=[1-9]\d* active_client=launcher "
    rf"app=none focus_generation={generation}",
    unique=True,
)
launcher_home_presented = index(
    r"UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9]\d* "
    r"receiver_image=launcher receiver_pid=[1-9]\d* session=[1-9]\d* frame_id=5 commit=14",
    unique=True,
)
if not old_read < app_home_focus < cancel or launcher_home_focus >= launcher_home_presented:
    raise SystemExit("held Present, Home focus delivery, and cancellation were not causally ordered")
retry_read = index(
    rf"UI_ROUTE_PRESENT_READ_OK sender_image=app sender_pid=[1-9]\d* "
    rf"receiver_image=surface-server receiver_pid=[1-9]\d* frame_id={frame} "
    rf"focus_generation={generation + 1} mode=full",
    cancel + 1,
    unique=True,
)
presented = index(
    rf"UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* frame_id={frame} commit=15",
    retry_read + 1,
    unique=True,
)
if not cancel < retry_read < presented:
    raise SystemExit("cancelled local frame was not retried and presented in order")

app_retry_focus = index(
    rf"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* active_client=app "
    rf"app=phone focus_generation={generation + 1}",
    cancel + 1,
    unique=True,
)
retry_tail = index(
    r"UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9]\d* "
    r"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* frame_id=12 commit=17",
    presented + 1,
    unique=True,
)
final_launcher_focus = index(
    rf"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=launcher receiver_pid=[1-9]\d* session=[1-9]\d* active_client=launcher "
    rf"app=none focus_generation={generation + 2}",
    retry_tail + 1,
    unique=True,
)
index(
    rf"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=[1-9]\d* "
    rf"receiver_image=app receiver_pid=[1-9]\d* session=[1-9]\d* active_client=launcher "
    rf"app=none focus_generation={generation + 2}",
    retry_tail + 1,
    unique=True,
)
final_presented = index(
    r"UI_ROUTE_PRESENTED_OK sender_image=surface-server sender_pid=[1-9]\d* "
    r"receiver_image=launcher receiver_pid=[1-9]\d* session=[1-9]\d* frame_id=6 commit=18",
    retry_tail + 1,
    unique=True,
)
if not cancel < app_retry_focus < retry_read < presented < retry_tail:
    raise SystemExit("generation-10 retry focus and three App presents were not ordered")
if final_launcher_focus >= final_presented:
    raise SystemExit("final Launcher focus was not delivered before commit 18")

commits = [
    line for line in lines
    if line.startswith(("USER_SURFACE_COMMIT_OK ", "USER_SURFACE_BUFFER_COMMIT_OK "))
]
ids = [int(re.search(r" frame_id=(\d+) ", line).group(1)) for line in commits]
if ids != list(range(1, 19)):
    raise SystemExit(f"global frame sequence has a gap after cancellation: {ids}")
buffer_pattern = re.compile(
    r"USER_SURFACE_BUFFER_COMMIT_OK owner=userspace pid=(\d+) session=(\d+) "
    r"producer_pid=(\d+) client_frame_id=(\d+) frame_id=(\d+) commit=(\d+) "
    r"mode=full buffer_generation=(\d+) format=xrgb8888 width=208 height=368 "
    r"global_damage=56/64/208/368 raster_writes=76544 composition=56/64/208/368 "
    r"scene_digest=0x[0-9a-f]{16} scanout_digest=0x[0-9a-f]{16} "
    r"cursor_preserved=1 dma_barrier=1"
)
buffer_matches = [
    buffer_pattern.fullmatch(line)
    for line in commits
    if line.startswith("USER_SURFACE_BUFFER_COMMIT_OK ")
]
if any(match is None for match in buffer_matches):
    raise SystemExit("graphics-buffer commit evidence became malformed during focus stress")
client_frames = [int(match.group(4)) for match in buffer_matches]
if client_frames != list(range(1, 13)):
    raise SystemExit(f"App client-local frame sequence has a gap after cancellation: {client_frames}")
buffer_generations = [int(match.group(7)) for match in buffer_matches]
expected_generations = [76 + 75 * index for index in range(9)] + [826, 901, 976]
if buffer_generations != expected_generations:
    raise SystemExit(f"cancelled generation 751 was not skipped exactly: {buffer_generations}")
inputs = [line for line in lines if line.startswith("USER_INPUT_READ_OK ")]
sequences = [int(re.search(r" sequence=(\d+) ", line).group(1)) for line in inputs]
if sequences != list(range(1, 37)):
    raise SystemExit(f"global input sequence has a gap during focus stress: {sequences}")
directed_samples = []
for line in inputs[-12:]:
    match = re.search(r" sequence=(\d+) x=(\d+) y=(\d+) pressed=([01]) ", line)
    if match is None:
        raise SystemExit(f"directed input evidence was malformed: {line}")
    directed_samples.append(tuple(map(int, match.groups())))
expected_samples = []
for start, y in [(25, 168), (28, 460), (31, 168), (34, 460)]:
    expected_samples.extend((start + offset, 160, y, pressed) for offset, pressed in enumerate((0, 1, 0)))
if directed_samples != expected_samples:
    raise SystemExit(f"directed Phone/Home samples changed: {directed_samples}")
PY

for forbidden in UI_EVENT_OK SURFACE_COMMIT_OK UI_INTERACTION_OK UI_TRANSITION_OK SURFACE_DEGRADED; do
  if grep -q "^${forbidden} " "$NORMALIZED_LOG"; then
    show_failure
    echo "Forbidden kernel-fallback/degraded marker appeared after userspace handoff: ${forbidden}." >&2
    exit 1
  fi
done

mkdir -p "$WORKSPACE_ROOT/target"
cp "$BASELINE_PPM" "$WORKSPACE_ROOT/target/bndroid-m29-before.ppm"
cp "$AFTER_PPM" "$WORKSPACE_ROOT/target/bndroid-m29-after.ppm"
stress_qmp_commands=$((6 * stress_attempts + 6))
stress_events=$((14 * stress_attempts + 14))
stress_transitions=$((2 * stress_attempts + 2))
total_events=$((56 + stress_events))
total_transitions=$((6 + stress_transitions))
home_returns=$((4 + stress_attempts))
printf '%s\n' "USERSPACE_UI_OK qemu=1 transport=ramfb input=virtio-tablet abi=23 ui_server=userspace launcher=userspace app=userspace ui_clients=2 ui_channels=2 present_protocol=3 present_wire=64 launcher_present=rect-wire app_present=graphics-buffer graphics_buffer=1 buffer_transfer=1 buffer_writable=1 buffer_copy_present=1 focus_routed=1 generation_qualified=1 present_cancelled=1 same_frame_retry=1 surface_owner=userspace unique_capability=1 surface_duplicate=0 surface_transferable=0 bounded_input_queue=64 baseline_normalized_samples=24 baseline_input_sequence=1..24 baseline_frame_sequence=1..13 baseline_commits=13 baseline_launcher_commits=4 baseline_buffer_commits=9 normalized_samples=$final_input_count input_sequence=1..$final_input_count commits=$final_commit_count initial_handoff=1 settings_frames=3 settings_modes=full/full/full settings_buffer_generations=76/151/226 settings_final_frame=4 target=settings view=settings damage_union=56/64/208/368 diff_pixels=53760 diff_bbox=56/64/263/379 baseline_sha256=$BASELINE_SHA256 after_sha256=$AFTER_SHA256 colors=13 scene_digest=$SCENE_DIGEST scanout_digest=$SCANOUT_DIGEST max_buffered=$max_buffered first_irq_entries=$irq_entries first_queue_irqs=$queue_irqs first_completions=7 first_delivered=7 first_recycled=7 dropped=0 invalid=0 gesture=tap+drag gesture_capture=press-to-release capture_receiver=app capture_samples=3 capture_app_sequence=1..3 capture_qmp_commands=3 capture_events=7 cancel_frame=$cancel_frame_id cancel_generation=$cancel_focus_generation retry_generation=$retry_focus_generation final_focus_generation=$final_focus_generation stress_attempts=$stress_attempts stress_qmp_commands=$stress_qmp_commands stress_events=$stress_events hit_test=userspace cursor_preserved=1 full_frame_capture=1 navigation_qmp_commands=15 navigation_events=35 total_events=$total_events targets=3 home_returns=$home_returns transitions=$total_transitions final_view=home degraded=0"
