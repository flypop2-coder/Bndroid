#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep kill; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M41 multi-window runtime." >&2
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

TARGET_ROOT="${BNDROID_MULTI_WINDOW_TARGET_DIR:-$WORKSPACE_ROOT/target/multi-window-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=multi-window-runtime \
  BNDROID_KERNEL_FEATURES=multi-window-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M41 multi-window kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-multi-window.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

PHASE1_MARKER="WINDOW_INPUT_PHASE1_READY"
PHASE2_MARKER="WINDOW_INPUT_PHASE2_READY"
SUCCESS_MARKER="WINDOW_COMPOSITOR_OK abi=23 protocol=1 policy=userspace capacity=2 live=2 z=launcher-app commands=9/2/5/2 events=14/2/5/2 input=5 capture=app-then-launcher signed=2 focus=1-2 damage=100864/97536/3328 hidden=1 retained=1 raises=35840 output=full-buffer frames=6 schedule=0-1-0-1-0-1 generations=1-1/2-2/3-3 clock=software-timer release=post-copy opportunities=7 edges=7 acquired=6 presented=6 pending=1 lifecycle=10/0/2/2 supervisor=4/0/2/2 processes=9/1/1/8 handles=31 endpoints=26 buffers=2 map=2/2 queue=6/6 acquire=6/6 releases=6 validates=459264/1837056 mappings=2/150 protects=12 producer=2/rw consumer=0 shared_pairs=0 physical_alias=0 pool=2/0/0 peaks=1/1/1 per_slot=3/3/3/3/3/3 waits=8/2/6 topology=resident final_state=ready final_app_resident=1"
BOOT_MARKER="BOOT_OK: M41 bounded userspace two-window z-order, occlusion, damage, and input capture verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK) '

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
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|multi[- ]window runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M41 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M41 published another M33-M40 runtime's success evidence." >&2
    exit 1
  fi
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    show_failure
    echo "M41 published an unexpected BOOT_OK marker." >&2
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

send_pointer_phase() {
  local phase="$1"
  python3 - "$QMP_SOCKET" "$phase" <<'PY'
import json
import socket
import sys
import time

socket_path, phase = sys.argv[1], int(sys.argv[2])

# map_absolute_to_pixel(raw, 0..32767, extent) is
# floor(raw * (extent - 1) / 32767). These are the first raw values mapping
# to the requested shell pixels, so an off-by-one cannot silently pass.
OVERLAP = (13970, 12587)  # shell (136, 184), phone-local (80, 120)
OUTSIDE = (7396, 6568)    # shell (72, 96), phone-local (16, 32)

def mapped(raw, extent):
    return raw * (extent - 1) // 32767

for raw, extent, expected in [
    (OVERLAP[0], 320, 136),
    (OVERLAP[1], 480, 184),
    (OUTSIDE[0], 320, 72),
    (OUTSIDE[1], 480, 96),
]:
    if mapped(raw, extent) != expected or mapped(raw - 1, extent) == expected:
        raise SystemExit(
            f"raw ABS inverse is not the first value for pixel {expected}: {raw}"
        )

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

def absolute(raw_x, raw_y):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [
            {"type": "abs", "data": {"axis": "x", "value": raw_x}},
            {"type": "abs", "data": {"axis": "y", "value": raw_y}},
        ]},
    })
    time.sleep(0.05)

def touch(down):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [
            {"type": "btn", "data": {"down": down, "button": "touch"}},
        ]},
    })
    time.sleep(0.05)

command({"execute": "qmp_capabilities"})
absolute(*OVERLAP)
touch(True)
if phase == 1:
    # Capture starts in App, crosses its bounds while held, and must remain App.
    absolute(*OUTSIDE)
elif phase != 2:
    raise SystemExit(f"unsupported pointer phase: {phase}")
touch(False)
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
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if [[ "$(grep -Fxc "$PHASE1_MARKER" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "The phase-1 marker changed while waiting for phase 2." >&2
  exit 1
fi
send_pointer_phase 2

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
    echo "QEMU exited before final M41 evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -Fxc "$PHASE1_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PHASE2_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$SUCCESS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M41 did not publish each device, phase, final, and boot marker exactly once." >&2
  exit 1
fi
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M41 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M41 did not enter and normalize execution at EL1." >&2
  exit 1
fi

echo "MULTI_WINDOW_OK phase1_raw=13970/12587->136/184 drag_raw=7396/6568->72/96 phase2_raw=13970/12587->136/184 markers=1/1/1/1"
