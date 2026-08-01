#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M43 text-input runtime." >&2
    exit 1
  fi
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-20}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
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
if [[ "$KEEP_TEMP" != "0" && "$KEEP_TEMP" != "1" ]]; then
  echo "BNDROID_KEEP_TEMP must be 0 or 1." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="${BNDROID_TEXT_INPUT_TARGET_DIR:-$WORKSPACE_ROOT/target/text-input-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=text-input-runtime \
  BNDROID_KERNEL_FEATURES=text-input-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M43 text-input kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-text-input.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
FINAL_PPM="$TMP_DIR/final-text-field.ppm"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

PHASE1_MARKER="WINDOW_INPUT_PHASE1_READY"
PHASE2_MARKER="WINDOW_INPUT_PHASE2_READY"
PERSISTENT_INPUT_MARKER="PERSISTENT_WINDOW_INPUT_READY"
TEXT_POINTER_FOCUS_MARKER="TEXT_POINTER_FOCUS_READY"
TEXT_INPUT_READY_MARKER="TEXT_INPUT_READY"
RENDER_PREFIX="TEXT_FIELD_RENDER_OK"
FOCUS_LOST_MARKER="TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0"
# Keep this full-line marker in one place: the runtime and checker must advance
# together whenever the sealed M43 evidence ledger changes.
SUCCESS_MARKER="TEXT_INPUT_OK abi=23 protocol=1 wires=BTI1/BTE1 wire=64 session=1 window=app2 focus=none4-app5-launcher6 messages=19/0 commands=7/1/5/1 events=12/1/2/2/1/5/1 revisions=0-1-2-3-4-5 editor=utf8-8 preedit=1 committed=a keys=12/6/6 active=10/5/5 unfocused=2 fifo=12/12/0/32 raw=24 reports=12 transitions=12 renders=5 field=8/16/96/16 pixels=1536 glyph=64 colors=111118/facc15/f4f4f5 digests=90a60d86afe0de65/6e9b51d9b30b78a5/474faa238cb15725 frames=9-10-11-12-13 output=13 schedule=0-1-0-1-0-1-0-1-0-1-0-1-0 write_generations=7/6 damage=126464/123136/3328 pointer=20/12/8 capture=app-launcher-app-app-launcher clock=14/14/13/13/1 graphics=13/13/13/13 validates=995072/3980288 mappings=2/150 protects=26 per_slot=7/6/7/6/7/6 waits=8/2/6 topology=resident final_state=ready final_app_resident=1"
BOOT_MARKER="BOOT_OK: M43 focus-scoped hardware keyboard and bounded UTF-8 text-editor slice verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK) '

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M43 diagnostic artifacts retained at $TMP_DIR" >&2
  else
    rm -rf "$TMP_DIR"
  fi
}

show_failure() {
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  cat "$NORMALIZED_LOG"
  cat "$QEMU_LOG" >&2
}

reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|text[- ]input runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M43 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M43 published an M33-M42 runtime's success evidence." >&2
    exit 1
  fi
  local unexpected_boot
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" || true)"
  if [[ -n "$unexpected_boot" ]]; then
    show_failure
    echo "M43 published an unexpected BOOT_OK marker." >&2
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

wait_for_prefix_count() {
  local prefix="$1"
  local expected="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    local count
    count="$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)"
    if [[ "$count" == "$expected" ]]; then
      return
    fi
    if ((count > expected)); then
      show_failure
      echo "$description was published too many times." >&2
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
    echo "$description changed while advancing the M43 transcript." >&2
    exit 1
  fi
}

qmp_action() {
  local action="$1"
  local argument="${2:-}"
  python3 - "$QMP_SOCKET" "$action" "$argument" <<'PY'
import json
import socket
import sys
import time

socket_path, action, argument = sys.argv[1:]

# These are the first raw ABS values that map to the requested 320x480
# scanout pixel. This rejects an off-by-one in either the checker or runtime.
OVERLAP = (13970, 12587)       # shell (136, 184), inside App
M41_OUTSIDE = (7396, 6568)    # shell (72, 96), Launcher-only
PHONE_OUTSIDE = (2055, 1369)  # shell (20, 20), outside logical phone

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
if "QMP" not in json.loads(stream.readline()):
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

def absolute(position):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [
            {"type": "abs", "data": {"axis": "x", "value": position[0]}},
            {"type": "abs", "data": {"axis": "y", "value": position[1]}},
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

def key(qcode, down):
    command({
        "execute": "input-send-event",
        "arguments": {"events": [{
            "type": "key",
            "data": {
                "down": down,
                "key": {"type": "qcode", "data": qcode},
            },
        }]},
    })
    time.sleep(0.05)

def click(position):
    absolute(position)
    touch(True)
    touch(False)

command({"execute": "qmp_capabilities"})
if action == "phase1":
    absolute(OVERLAP)
    touch(True)
    absolute(M41_OUTSIDE)
    touch(False)
elif action == "phase2":
    click(OVERLAP)
elif action == "phase3":
    absolute(PHONE_OUTSIDE)
    touch(True)
    touch(False)
    absolute(OVERLAP)
    touch(True)
    absolute(PHONE_OUTSIDE)
    touch(False)
elif action == "focus-app":
    click(OVERLAP)
elif action == "focus-launcher":
    click(M41_OUTSIDE)
elif action == "key":
    if argument not in {"a", "ret", "backspace"}:
        raise SystemExit(f"unsupported qcode: {argument}")
    key(argument, True)
    key(argument, False)
elif action == "screendump":
    if not argument:
        raise SystemExit("screendump requires an output path")
    command({
        "execute": "screendump",
        "arguments": {"filename": argument, "format": "ppm"},
    })
else:
    raise SystemExit(f"unsupported QMP action: {action}")
connection.close()
PY
}

validate_render_transcript() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import sys

expected = [
    "TEXT_FIELD_RENDER_OK step=1 action=preedit committed=0 preedit=a revision=1 content_generation=3 frame=9 digest=0x90a60d86afe0de65\n",
    "TEXT_FIELD_RENDER_OK step=2 action=commit committed=1 preedit=none revision=2 content_generation=4 frame=10 digest=0x6e9b51d9b30b78a5\n",
    "TEXT_FIELD_RENDER_OK step=3 action=delete committed=0 preedit=none revision=3 content_generation=5 frame=11 digest=0x474faa238cb15725\n",
    "TEXT_FIELD_RENDER_OK step=4 action=preedit committed=0 preedit=a revision=4 content_generation=6 frame=12 digest=0x90a60d86afe0de65\n",
    "TEXT_FIELD_RENDER_OK step=5 action=commit committed=1 preedit=none revision=5 content_generation=7 frame=13 digest=0x6e9b51d9b30b78a5\n",
]
observed = [
    line
    for line in open(sys.argv[1], encoding="utf-8")
    if line.startswith("TEXT_FIELD_RENDER_OK ")
]
if observed != expected:
    raise SystemExit(f"unexpected exact render transcript: {observed!r}")
PY
}

validate_text_field() {
  python3 - "$FINAL_PPM" <<'PY'
import sys

path = sys.argv[1]
with open(path, "rb") as source:
    if source.readline() != b"P6\n":
        raise SystemExit("M43 screenshot is not a binary PPM")
    dimensions = source.readline()
    while dimensions.startswith(b"#"):
        dimensions = source.readline()
    if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
        raise SystemExit("M43 screenshot is not exact 320x480 8-bit RGB")
    pixels = source.read()
if len(pixels) != 320 * 480 * 3:
    raise SystemExit(f"M43 screenshot payload has {len(pixels)} bytes")

FIELD_X, FIELD_Y, FIELD_WIDTH, FIELD_HEIGHT = 112, 160, 96, 16
BACKGROUND = bytes((0x11, 0x11, 0x18))
COMMITTED = bytes((0xF4, 0xF4, 0xF5))
GLYPH_A = [0b00000, 0b01110, 0b10001, 0b00001,
           0b01111, 0b10001, 0b01111, 0b00000]

expected_glyph = set()
for row, bits in enumerate(GLYPH_A):
    for column in range(5):
        if bits & (1 << (4 - column)):
            for dy in range(2):
                for dx in range(2):
                    expected_glyph.add((column * 2 + dx, row * 2 + dy))
if len(expected_glyph) != 64:
    raise SystemExit("checker glyph does not contain exactly 64 pixels")

committed_count = 0
for local_y in range(FIELD_HEIGHT):
    for local_x in range(FIELD_WIDTH):
        offset = ((FIELD_Y + local_y) * 320 + FIELD_X + local_x) * 3
        observed = pixels[offset:offset + 3]
        expected = COMMITTED if (local_x, local_y) in expected_glyph else BACKGROUND
        if observed != expected:
            raise SystemExit(
                "text-field mismatch at "
                f"local ({local_x},{local_y}) / scanout "
                f"({FIELD_X + local_x},{FIELD_Y + local_y}): "
                f"observed={observed.hex()} expected={expected.hex()}"
            )
        committed_count += observed == COMMITTED
if committed_count != 64:
    raise SystemExit(f"expected 64 committed glyph pixels, found {committed_count}")
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
qmp_action phase1

wait_for_exact_marker "$PHASE2_MARKER" "the phase-2 input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
qmp_action phase2

wait_for_exact_marker "$PERSISTENT_INPUT_MARKER" "the persistent-session input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
assert_marker_once "$PHASE2_MARKER" "The phase-2 marker"
qmp_action phase3

wait_for_exact_marker "$TEXT_POINTER_FOCUS_MARKER" "the App-focus pointer-ready marker"
qmp_action focus-app
wait_for_exact_marker "$TEXT_INPUT_READY_MARKER" "the active text-input session marker"

qmp_action key a
wait_for_prefix_count "$RENDER_PREFIX" 1 "the A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$RENDER_PREFIX" 2 "the Enter-commit render marker"
qmp_action key backspace
wait_for_prefix_count "$RENDER_PREFIX" 3 "the Backspace-delete render marker"
qmp_action key a
wait_for_prefix_count "$RENDER_PREFIX" 4 "the second A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$RENDER_PREFIX" 5 "the final Enter-commit render marker"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
validate_render_transcript
qmp_action screendump "$FINAL_PPM"
validate_text_field
cp "$FINAL_PPM" "$WORKSPACE_ROOT/target/bndroid-m43-text-field.ppm"

# The full-screen Launcher would cover the App after this click, so the exact
# App-local text-field screenshot is deliberately captured immediately above.
qmp_action focus-launcher
wait_for_exact_marker "$FOCUS_LOST_MARKER" "the text-input focus-loss marker"
qmp_action key a

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if grep -Fxq "$SUCCESS_MARKER" "$NORMALIZED_LOG" \
    && grep -Fxq "$BOOT_MARKER" "$NORMALIZED_LOG"; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    show_failure
    echo "QEMU exited before final M43 evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -Fxc "$PHASE1_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PHASE2_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PERSISTENT_INPUT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TEXT_POINTER_FOCUS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TEXT_INPUT_READY_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Ec "^${RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" \
  || "$(grep -Fxc "$FOCUS_LOST_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$SUCCESS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M43 did not publish each device, phase, render, final, and boot marker exactly once." >&2
  exit 1
fi
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M43 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M43 did not enter and normalize execution at EL1." >&2
  exit 1
fi

echo "TEXT_INPUT_QMP_OK phase1=1 phase2=1 persistent=1 app_focus=136/184 keys=a-ret-backspace-a-ret key_pairs=6 transitions=12 active=10/5/5 renders=5 launcher_focus=72/96 unfocused=2/drop field=112/160/96/16 committed_pixels=64 background_pixels=1472 screenshot=target/bndroid-m43-text-field.ppm markers=1/1/1/1/1/5/1/1/1"
