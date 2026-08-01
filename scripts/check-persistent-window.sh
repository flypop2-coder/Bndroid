#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M42 persistent window runtime." >&2
    exit 1
  fi
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-20}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "BNDROID_PROFILE must be 'debug' or 'release'." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="${BNDROID_PERSISTENT_WINDOW_TARGET_DIR:-$WORKSPACE_ROOT/target/persistent-window-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=persistent-window-runtime \
  BNDROID_KERNEL_FEATURES=persistent-window-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M42 persistent-window kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-persistent-window.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

PHASE1_MARKER="WINDOW_INPUT_PHASE1_READY"
PHASE2_MARKER="WINDOW_INPUT_PHASE2_READY"
PERSISTENT_INPUT_MARKER="PERSISTENT_WINDOW_INPUT_READY"
SUCCESS_MARKER="WINDOW_SESSION_OK abi=23 protocol=1 policy=userspace persistent=1 capacity=2 live=2 z=launcher-app commands=13/3/6/3/1 events=21/3/6/3/8/1 input=8 raw=14 dropped=6 capture=app-launcher-app signed=4 focus_routes=1-2-3 final_focus=none/4 generation=app1-app2 damage=118784/115456/3328 exposure=53760 hidden=1 retained=1 output=full-buffer frames=8 schedule=0-1-0-1-0-1-0-1 generations=1-1/2-2/3-3/4-4 clock=software-timer release=post-copy opportunities=9 edges=9 acquired=8 presented=8 pending=1 lifecycle=10/0/2/2 supervisor=4/0/2/2 processes=9/1/1/8 handles=31 endpoints=26 buffers=2 map=2/2 queue=8/8 acquire=8/8 releases=8 validates=612352/2449408 mappings=2/150 protects=16 producer=2/rw consumer=0 pool=2/0/0 peaks=1/1/1 per_slot=4/4/4/4/4/4 outside=reject-until-release captured_outside=signed-local waits=8/2/6 topology=resident final_state=ready final_app_resident=1"
BOOT_MARKER="BOOT_OK: M42 persistent userspace window session, boundary routing, and generation-safe recreation verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK) '

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
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|persistent[- ]window runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M42 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M42 published an M33-M41 runtime's success evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    show_failure
    echo "M42 published an unexpected BOOT_OK marker." >&2
    exit 1
  fi
}

wait_for_exact_marker() {
  local marker="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    local count
    count="$(grep -Fxc "$marker" "$NORMALIZED_LOG" || true)"
    if [[ "$count" == "1" ]]; then
      return
    fi
    if [[ "$count" != "0" ]]; then
      show_failure
      echo "$description was not published exactly once." >&2
      exit 1
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      set +e
      wait "$QEMU_PID"
      local status=$?
      set -e
      QEMU_PID=""
      show_failure
      echo "QEMU exited before $description (status $status)." >&2
      exit 1
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
}

assert_marker_once() {
  local marker="$1"
  local description="$2"
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  if [[ "$(grep -Fxc "$marker" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "$description changed while advancing the input transcript." >&2
    exit 1
  fi
}

send_pointer_phase() {
  local phase="$1"
  python3 - "$QMP_SOCKET" "$phase" "$SERIAL_LOG" "$BOOT_TIMEOUT_SECONDS" <<'PY'
import json
from pathlib import Path
import re
import socket
import sys
import time

socket_path = sys.argv[1]
phase = int(sys.argv[2])
serial_path = Path(sys.argv[3])
sample_timeout = max(1.0, min(float(sys.argv[4]), 10.0))

# map_absolute_to_pixel(raw, 0..32767, extent) is
# floor(raw * (extent - 1) / 32767). Every coordinate below is the first raw
# value mapping to its requested shell pixel, so an off-by-one cannot pass.
OVERLAP = (13970, 12587)  # shell (136, 184), phone-local (80, 120)
M41_OUTSIDE = (7396, 6568)  # shell (72, 96), phone-local (16, 32)
PHONE_OUTSIDE = (2055, 1369)  # shell (20, 20), phone-local (-36, -44)

def mapped(raw, extent):
    return raw * (extent - 1) // 32767

for raw, extent, expected in [
    (OVERLAP[0], 320, 136),
    (OVERLAP[1], 480, 184),
    (M41_OUTSIDE[0], 320, 72),
    (M41_OUTSIDE[1], 480, 96),
    (PHONE_OUTSIDE[0], 320, 20),
    (PHONE_OUTSIDE[1], 480, 20),
]:
    if mapped(raw, extent) != expected or mapped(raw - 1, extent) == expected:
        raise SystemExit(
            f"raw ABS inverse is not the first value for pixel {expected}: {raw}"
        )

# The App begins at phone-local (48, 80), so the captured physical endpoint
# must retain the signed App-local coordinate (-84, -124).
if (20 - 56 - 48, 20 - 64 - 80) != (-84, -124):
    raise SystemExit("captured outside endpoint no longer maps to (-84, -124)")

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

def wait_sample(sequence, x, y, pressed):
    deadline = time.monotonic() + sample_timeout
    sequence_token = f" sequence={sequence} "
    expected = re.compile(
        rf"^USER_INPUT_READ_OK .* sequence={sequence} x={x} y={y} "
        rf"pressed={pressed} "
    )
    while time.monotonic() < deadline:
        data = serial_path.read_bytes().replace(b"\r", b"")
        # Ignore a final unterminated fragment while QEMU is appending the
        # serial line; otherwise a normal concurrent write can look malformed.
        lines = data.split(b"\n")
        if data and not data.endswith(b"\n"):
            lines = lines[:-1]
        decoded = [line.decode("utf-8", errors="replace") for line in lines]
        matches = [line for line in decoded if sequence_token in line]
        if len(matches) == 1:
            if expected.search(matches[0]):
                return
            raise SystemExit(
                f"guest input sample {sequence} changed: {matches[0]}"
            )
        if len(matches) > 1:
            raise SystemExit(
                f"guest input sample {sequence} was published more than once"
            )
        time.sleep(0.01)
    raise SystemExit(
        f"timed out waiting for guest input sample {sequence} "
        f"at {x}/{y} pressed={pressed}"
    )

def absolute(raw_x, raw_y, sequence, x, y, pressed):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [
            {"type": "abs", "data": {"axis": "x", "value": raw_x}},
            {"type": "abs", "data": {"axis": "y", "value": raw_y}},
        ]},
    })
    wait_sample(sequence, x, y, pressed)

def touch(down, sequence, x, y):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [
            {"type": "btn", "data": {"down": down, "button": "touch"}},
        ]},
    })
    wait_sample(sequence, x, y, int(down))

command({"execute": "qmp_capabilities"})
if phase == 1:
    # M41 phase 1: App capture crosses its bounds and remains routed to App.
    absolute(*OVERLAP, 1, 136, 184, 0)
    touch(True, 2, 136, 184)
    absolute(*M41_OUTSIDE, 3, 72, 96, 1)
    touch(False, 4, 72, 96)
elif phase == 2:
    # M41 phase 2: the exposed Launcher receives the complete contact.
    absolute(*OVERLAP, 5, 136, 184, 0)
    touch(True, 6, 136, 184)
    touch(False, 7, 136, 184)
elif phase == 3:
    # An outside-origin contact is rejected through release (three samples).
    absolute(*PHONE_OUTSIDE, 8, 20, 20, 0)
    touch(True, 9, 20, 20)
    touch(False, 10, 20, 20)
    # App capture then survives a drag beyond the logical phone (four samples).
    absolute(*OVERLAP, 11, 136, 184, 0)
    touch(True, 12, 136, 184)
    absolute(*PHONE_OUTSIDE, 13, 20, 20, 1)
    touch(False, 14, 20, 20)
else:
    raise SystemExit(f"unsupported pointer phase: {phase}")
connection.close()
PY
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu "$QEMU_CPU" \
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

wait_for_exact_marker "$PHASE1_MARKER" "the phase-1 input-ready marker"
send_pointer_phase 1

wait_for_exact_marker "$PHASE2_MARKER" "the phase-2 input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
send_pointer_phase 2

wait_for_exact_marker "$PERSISTENT_INPUT_MARKER" "the persistent-session input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
assert_marker_once "$PHASE2_MARKER" "The phase-2 marker"
send_pointer_phase 3

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Fxq "$BOOT_MARKER" "$NORMALIZED_LOG" \
    && grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    show_failure
    echo "QEMU exited before final M42 evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -Fxc "$PHASE1_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PHASE2_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PERSISTENT_INPUT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$SUCCESS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M42 did not publish each device, phase, final, and boot marker exactly once." >&2
  exit 1
fi
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M42 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M42 did not enter and normalize execution at EL1." >&2
  exit 1
fi

echo "PERSISTENT_WINDOW_OK phase1_raw=13970/12587->136/184 drag_raw=7396/6568->72/96 phase2_raw=13970/12587->136/184 phase3_outside_raw=2055/1369->20/20 captured_local=-84/-124 markers=1/1/1/1/1"
