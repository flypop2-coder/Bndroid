#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M44 soft-keyboard runtime." >&2
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

TARGET_ROOT="${BNDROID_SOFT_KEYBOARD_TARGET_DIR:-$WORKSPACE_ROOT/target/soft-keyboard-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=soft-keyboard-runtime \
  BNDROID_KERNEL_FEATURES=soft-keyboard-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M44 soft-keyboard kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-soft-keyboard.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
VISIBLE_PPM="$TMP_DIR/soft-keyboard-visible.ppm"
HIDDEN_PPM="$TMP_DIR/soft-keyboard-hidden.ppm"
FINAL_PPM="$TMP_DIR/soft-keyboard-final.ppm"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

PHASE1_MARKER="WINDOW_INPUT_PHASE1_READY"
PHASE2_MARKER="WINDOW_INPUT_PHASE2_READY"
PERSISTENT_INPUT_MARKER="PERSISTENT_WINDOW_INPUT_READY"
TEXT_POINTER_FOCUS_MARKER="TEXT_POINTER_FOCUS_READY"
TEXT_INPUT_READY_MARKER="TEXT_INPUT_READY"
TEXT_RENDER_PREFIX="TEXT_FIELD_RENDER_OK"
TEXT_FOCUS_LOST_MARKER="TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0"
SOFT_POINTER_MARKER="SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6"
SOFT_READY_MARKER="SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible"
SOFT_RENDER_PREFIX="SOFT_TEXT_RENDER_OK"
SOFT_HIDDEN_MARKER="SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent"
# Keep these full-line markers in one place: the runtime and checker must
# advance together whenever either sealed evidence ledger changes.
TEXT_SUCCESS_MARKER="TEXT_INPUT_OK abi=23 protocol=1 wires=BTI1/BTE1 wire=64 session=1 window=app2 focus=none4-app5-launcher6 messages=19/0 commands=7/1/5/1 events=12/1/2/2/1/5/1 revisions=0-1-2-3-4-5 editor=utf8-8 preedit=1 committed=a keys=12/6/6 active=10/5/5 unfocused=2 fifo=12/12/0/32 raw=24 reports=12 transitions=12 renders=5 field=8/16/96/16 pixels=1536 glyph=64 colors=111118/facc15/f4f4f5 digests=90a60d86afe0de65/6e9b51d9b30b78a5/474faa238cb15725 frames=9-10-11-12-13 output=13 schedule=0-1-0-1-0-1-0-1-0-1-0-1-0 write_generations=7/6 damage=126464/123136/3328 pointer=20/12/8 capture=app-launcher-app-app-launcher clock=14/14/13/13/1 graphics=13/13/13/13 validates=995072/3980288 mappings=2/150 protects=26 per_slot=7/6/7/6/7/6 waits=8/2/6 topology=resident final_state=ready final_app_resident=1"
SUCCESS_MARKER="SOFT_KEYBOARD_OK abi=23 protocol=1 prefix=m43 input_method=in_surface input_server=0 process_delta=0 windows=2 overlay=system/nonfocusable source=tablet hardware_delta=0 fifo_delta=0 contacts=9 soft=5/5 preedit=2 commit=2 delete=1 focus=launcher6-app7-launcher8-app9 visibility=show-hide-show hidden_text=0 sessions=2-3 messages=21/0 commands=8/2/5/1 events=13/2/2/2/1/5/1 state=aa editor=utf8-8 renders=5 outputs=8 frames=14-21 overlay_bounds=0/272/208/96 scanout=56/336/208/96 pixels=19968 pointer=47/27/10/17 graphics=21/21/21/21 validates=1607424/6429696 clock=22/22/21/21/1 write_generations=11/10 per_slot=11/10/11/10/11/10 waits=8/2/6 topology=resident final_state=ready final_app_resident=1"
BOOT_MARKER="BOOT_OK: M44 touch soft keyboard and focus-preserving text input verified"
M43_BOOT_MARKER="BOOT_OK: M43 focus-scoped hardware keyboard and bounded UTF-8 text-editor slice verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK) '

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M44 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU appends serial bytes directly to the file.  A polling turn can land
  # halfway through a long success line; never classify that incomplete tail
  # as an unexpected full marker.  The next turn observes the terminating
  # newline and validates the complete record.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|soft[- ]keyboard runtime (failed|timed out)|text[- ]input runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M44 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M44 published an M33-M42 leaf runtime's success evidence." >&2
    exit 1
  fi
  local unexpected_text unexpected_soft unexpected_boot
  unexpected_text="$(grep '^TEXT_INPUT_OK ' "$NORMALIZED_LOG" | grep -Fvx "$TEXT_SUCCESS_MARKER" || true)"
  unexpected_soft="$(grep '^SOFT_KEYBOARD_OK ' "$NORMALIZED_LOG" | grep -Fvx "$SUCCESS_MARKER" || true)"
  unexpected_boot="$(grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" || true)"
  if [[ -n "$unexpected_text" || -n "$unexpected_soft" || -n "$unexpected_boot" ]]; then
    show_failure
    echo "M44 published an unexpected leaf success or BOOT_OK marker." >&2
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
    echo "$description changed while advancing the M44 transcript." >&2
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

# First raw ABS values mapping to each requested 320x480 scanout pixel.
POSITIONS = {
    "overlap": ((13970, 12587), (136, 184)),
    "launcher": ((7396, 6568), (72, 96)),
    "phone-outside": ((2055, 1369), (20, 20)),
    "soft-a": ((9040, 25722), (88, 376)),
    "soft-backspace": ((15614, 25722), (152, 376)),
    "soft-enter": ((23009, 25722), (224, 376)),
}

def mapped(raw, extent):
    return raw * (extent - 1) // 32767

for raw_position, expected_position in POSITIONS.values():
    for raw, extent, expected in zip(raw_position, (320, 480), expected_position):
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

def absolute(name):
    position = POSITIONS[name][0]
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

def click(name):
    absolute(name)
    touch(True)
    touch(False)

command({"execute": "qmp_capabilities"})
if action == "phase1":
    absolute("overlap")
    touch(True)
    absolute("launcher")
    touch(False)
elif action == "phase2":
    click("overlap")
elif action == "phase3":
    absolute("phone-outside")
    touch(True)
    touch(False)
    absolute("overlap")
    touch(True)
    absolute("phone-outside")
    touch(False)
elif action == "focus-app":
    click("overlap")
elif action == "focus-launcher":
    click("launcher")
elif action in {"soft-a", "soft-backspace", "soft-enter"}:
    click(action)
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

validate_render_transcripts() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import sys

expected_text = [
    "TEXT_FIELD_RENDER_OK step=1 action=preedit committed=0 preedit=a revision=1 content_generation=3 frame=9 digest=0x90a60d86afe0de65\n",
    "TEXT_FIELD_RENDER_OK step=2 action=commit committed=1 preedit=none revision=2 content_generation=4 frame=10 digest=0x6e9b51d9b30b78a5\n",
    "TEXT_FIELD_RENDER_OK step=3 action=delete committed=0 preedit=none revision=3 content_generation=5 frame=11 digest=0x474faa238cb15725\n",
    "TEXT_FIELD_RENDER_OK step=4 action=preedit committed=0 preedit=a revision=4 content_generation=6 frame=12 digest=0x90a60d86afe0de65\n",
    "TEXT_FIELD_RENDER_OK step=5 action=commit committed=1 preedit=none revision=5 content_generation=7 frame=13 digest=0x6e9b51d9b30b78a5\n",
]
expected_soft = [
    "SOFT_TEXT_RENDER_OK step=1 action=preedit committed=a preedit=a revision=1 content_generation=8 frame=15\n",
    "SOFT_TEXT_RENDER_OK step=2 action=commit committed=aa preedit=none revision=2 content_generation=9 frame=16\n",
    "SOFT_TEXT_RENDER_OK step=3 action=delete committed=a preedit=none revision=3 content_generation=10 frame=17\n",
    "SOFT_TEXT_RENDER_OK step=4 action=preedit committed=a preedit=a revision=4 content_generation=11 frame=18\n",
    "SOFT_TEXT_RENDER_OK step=5 action=commit committed=aa preedit=none revision=5 content_generation=12 frame=19\n",
]
with open(sys.argv[1], encoding="utf-8") as source:
    lines = list(source)
observed_text = [line for line in lines if line.startswith("TEXT_FIELD_RENDER_OK ")]
observed_soft = [line for line in lines if line.startswith("SOFT_TEXT_RENDER_OK ")]
if observed_text != expected_text:
    raise SystemExit(f"unexpected exact M43 render transcript: {observed_text!r}")
if observed_soft != expected_soft:
    raise SystemExit(f"unexpected exact M44 render transcript: {observed_soft!r}")
PY
}

validate_visible_screenshot() {
  local path="$1"
  local cursor_x="$2"
  local cursor_y="$3"
  python3 - "$path" "$cursor_x" "$cursor_y" <<'PY'
import hashlib
import sys

path = sys.argv[1]
cursor_x, cursor_y = map(int, sys.argv[2:])
with open(path, "rb") as source:
    if source.readline() != b"P6\n":
        raise SystemExit("M44 screenshot is not a binary PPM")
    dimensions = source.readline()
    while dimensions.startswith(b"#"):
        dimensions = source.readline()
    if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
        raise SystemExit("M44 screenshot is not exact 320x480 8-bit RGB")
    pixels = source.read()
if len(pixels) != 320 * 480 * 3:
    raise SystemExit(f"M44 screenshot payload has {len(pixels)} bytes")

def pixel(x, y):
    offset = (y * 320 + x) * 3
    return pixels[offset:offset + 3]

OVERLAY = (56, 336, 208, 96)
BACKGROUND = bytes.fromhex("181b24")
A_COLOR = bytes.fromhex("3b82f6")
BACKSPACE_COLOR = bytes.fromhex("ef4444")
ENTER_COLOR = bytes.fromhex("22c55e")
A_RECT = (64, 344, 48, 64)
BACKSPACE_RECT = (120, 344, 64, 64)
ENTER_RECT = (192, 344, 64, 64)

def contains(rect, x, y):
    rx, ry, width, height = rect
    return rx <= x < rx + width and ry <= y < ry + height

def arrow_class(x, y):
    if y < 14 and x <= y // 2:
        return x == 0 or x == y // 2 or y == 13
    if 10 <= y < 20 and 4 <= x < 8:
        return x == 4 or x == 7 or y == 10 or y == 19
    return None

def blend(background, foreground, alpha):
    inverse = 255 - alpha
    return bytes(
        (front * alpha + back * inverse + 127) // 255
        for back, front in zip(background, foreground)
    )

def compose_cursor(base, x, y):
    local_x, local_y = x - cursor_x, y - cursor_y
    if not (0 <= local_x < 12 and 0 <= local_y < 22):
        return base
    foreground = arrow_class(local_x, local_y)
    if foreground is not None:
        return bytes.fromhex("111827") if foreground else bytes.fromhex("f8fafc")
    if local_x >= 2 and local_y >= 2 and arrow_class(local_x - 2, local_y - 2) is not None:
        return blend(base, bytes.fromhex("000000"), 96)
    return base

base_counts = {BACKGROUND: 0, A_COLOR: 0, BACKSPACE_COLOR: 0, ENTER_COLOR: 0}
cursor_pixels = 0
ox, oy, width, height = OVERLAY
for y in range(oy, oy + height):
    for x in range(ox, ox + width):
        base = BACKGROUND
        if contains(A_RECT, x, y):
            base = A_COLOR
        elif contains(BACKSPACE_RECT, x, y):
            base = BACKSPACE_COLOR
        elif contains(ENTER_RECT, x, y):
            base = ENTER_COLOR
        expected = compose_cursor(base, x, y)
        observed = pixel(x, y)
        if observed != expected:
            raise SystemExit(
                f"soft-keyboard mismatch at ({x},{y}): "
                f"observed={observed.hex()} expected={expected.hex()}"
            )
        base_counts[base] += 1
        cursor_pixels += expected != base
expected_base_counts = {
    BACKGROUND: 8704,
    A_COLOR: 3072,
    BACKSPACE_COLOR: 4096,
    ENTER_COLOR: 4096,
}
if base_counts != expected_base_counts:
    raise SystemExit(f"unexpected soft-keyboard base-color ledger: {base_counts!r}")
expected_cursor_pixels = 122 if (cursor_x, cursor_y) == (224, 376) else 0
if cursor_pixels != expected_cursor_pixels:
    raise SystemExit(
        f"expected {expected_cursor_pixels} cursor-composited overlay pixels, "
        f"found {cursor_pixels}"
    )

FIELD_X, FIELD_Y, FIELD_WIDTH, FIELD_HEIGHT = 112, 160, 96, 16
FIELD_BACKGROUND = bytes.fromhex("111118")
COMMITTED = bytes.fromhex("f4f4f5")
GLYPH_A = [0b00000, 0b01110, 0b10001, 0b00001,
           0b01111, 0b10001, 0b01111, 0b00000]
expected_glyph = set()
for character in range(2):
    origin_x = character * 12
    for row, bits in enumerate(GLYPH_A):
        for column in range(5):
            if bits & (1 << (4 - column)):
                for dy in range(2):
                    for dx in range(2):
                        expected_glyph.add((origin_x + column * 2 + dx, row * 2 + dy))
if len(expected_glyph) != 128:
    raise SystemExit("checker glyph ledger does not contain exactly 128 pixels")

committed_count = 0
for local_y in range(FIELD_HEIGHT):
    for local_x in range(FIELD_WIDTH):
        observed = pixel(FIELD_X + local_x, FIELD_Y + local_y)
        expected = COMMITTED if (local_x, local_y) in expected_glyph else FIELD_BACKGROUND
        if observed != expected:
            raise SystemExit(
                "text-field mismatch at "
                f"local ({local_x},{local_y}) / scanout "
                f"({FIELD_X + local_x},{FIELD_Y + local_y}): "
                f"observed={observed.hex()} expected={expected.hex()}"
            )
        committed_count += observed == COMMITTED
if committed_count != 128:
    raise SystemExit(f"expected 128 committed glyph pixels, found {committed_count}")

with open(path, "rb") as source:
    print(hashlib.sha256(source.read()).hexdigest())
PY
}

validate_hidden_screenshot() {
  local path="$1"
  python3 - "$path" <<'PY'
import sys

with open(sys.argv[1], "rb") as source:
    if source.readline() != b"P6\n":
        raise SystemExit("M44 hidden screenshot is not a binary PPM")
    dimensions = source.readline()
    while dimensions.startswith(b"#"):
        dimensions = source.readline()
    if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
        raise SystemExit("M44 hidden screenshot is not exact 320x480 8-bit RGB")
    pixels = source.read()
if len(pixels) != 320 * 480 * 3:
    raise SystemExit(f"M44 hidden screenshot payload has {len(pixels)} bytes")

key_colors = {bytes.fromhex(value) for value in ("3b82f6", "ef4444", "22c55e")}
for y in range(336, 432):
    for x in range(56, 264):
        offset = (y * 320 + x) * 3
        observed = pixels[offset:offset + 3]
        if observed in key_colors:
            raise SystemExit(
                f"hidden overlay retained key color {observed.hex()} at ({x},{y})"
            )
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

# Replay the complete M41/M42/M43 prefix before beginning M44 input.
wait_for_exact_marker "$PHASE1_MARKER" "the phase-1 input-ready marker"
qmp_action phase1

wait_for_exact_marker "$PHASE2_MARKER" "the phase-2 input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
qmp_action phase2

wait_for_exact_marker "$PERSISTENT_INPUT_MARKER" "the persistent-session input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
assert_marker_once "$PHASE2_MARKER" "The phase-2 marker"
qmp_action phase3

wait_for_exact_marker "$TEXT_POINTER_FOCUS_MARKER" "the M43 App-focus pointer-ready marker"
qmp_action focus-app
wait_for_exact_marker "$TEXT_INPUT_READY_MARKER" "the active M43 text-input session marker"

qmp_action key a
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 1 "the M43 A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 2 "the M43 Enter-commit render marker"
qmp_action key backspace
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 3 "the M43 Backspace-delete render marker"
qmp_action key a
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 4 "the second M43 A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 5 "the final M43 Enter-commit render marker"

qmp_action focus-launcher
wait_for_exact_marker "$TEXT_FOCUS_LOST_MARKER" "the M43 text-input focus-loss marker"
qmp_action key a
wait_for_exact_marker "$TEXT_SUCCESS_MARKER" "the exact M43 prefix success marker"
if grep -Fxq "$M43_BOOT_MARKER" "$NORMALIZED_LOG"; then
  show_failure
  echo "M44 incorrectly published the standalone M43 BOOT_OK marker." >&2
  exit 1
fi

wait_for_exact_marker "$SOFT_POINTER_MARKER" "the M44 pointer-ready marker"
qmp_action focus-app
wait_for_exact_marker "$SOFT_READY_MARKER" "the visible session-2 soft keyboard marker"

qmp_action soft-a
wait_for_prefix_count "$SOFT_RENDER_PREFIX" 1 "the soft A-preedit render marker"
qmp_action soft-enter
wait_for_prefix_count "$SOFT_RENDER_PREFIX" 2 "the soft Enter-commit render marker"
qmp_action soft-backspace
wait_for_prefix_count "$SOFT_RENDER_PREFIX" 3 "the soft Backspace-delete render marker"
qmp_action soft-a
wait_for_prefix_count "$SOFT_RENDER_PREFIX" 4 "the second soft A-preedit render marker"
qmp_action soft-enter
wait_for_prefix_count "$SOFT_RENDER_PREFIX" 5 "the final soft Enter-commit render marker"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
validate_render_transcripts
qmp_action screendump "$VISIBLE_PPM"
VISIBLE_SHA256="$(validate_visible_screenshot "$VISIBLE_PPM" 224 376)"

qmp_action focus-launcher
wait_for_exact_marker "$SOFT_HIDDEN_MARKER" "the hidden session-2 soft keyboard marker"
qmp_action screendump "$HIDDEN_PPM"
validate_hidden_screenshot "$HIDDEN_PPM"

# The old A-key coordinate is now ordinary Launcher content. Its contact must
# neither reach the hidden input method nor produce a sixth text render.
qmp_action soft-a
sleep 0.20
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -Ec "^${SOFT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" \
  || "$(grep -Ec "^${TEXT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" ]]; then
  show_failure
  echo "A contact in the hidden overlay bounds produced text." >&2
  exit 1
fi

# Refocusing App starts session 3 and restores the trusted overlay. The final
# screenshot is the sealed artifact rather than the earlier session-2 image.
qmp_action focus-app
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
    echo "QEMU exited before final M44 evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
qmp_action screendump "$FINAL_PPM"
FINAL_SHA256="$(validate_visible_screenshot "$FINAL_PPM" 136 184)"
validate_render_transcripts

if [[ "$(grep -Fxc "$PHASE1_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PHASE2_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$PERSISTENT_INPUT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TEXT_POINTER_FOCUS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TEXT_INPUT_READY_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Ec "^${TEXT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" \
  || "$(grep -Fxc "$TEXT_FOCUS_LOST_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TEXT_SUCCESS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$M43_BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "0" \
  || "$(grep -Fxc "$SOFT_POINTER_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$SOFT_READY_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Ec "^${SOFT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" \
  || "$(grep -Fxc "$SOFT_HIDDEN_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$SUCCESS_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M44 did not publish each device, prefix, render, visibility, final, and boot marker exactly once." >&2
  exit 1
fi
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M44 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M44 did not enter and normalize execution at EL1." >&2
  exit 1
fi

cp "$VISIBLE_PPM" "$WORKSPACE_ROOT/target/bndroid-m44-soft-keyboard-visible.ppm"
cp "$HIDDEN_PPM" "$WORKSPACE_ROOT/target/bndroid-m44-soft-keyboard-hidden.ppm"
cp "$FINAL_PPM" "$WORKSPACE_ROOT/target/bndroid-m44-soft-keyboard.ppm"

echo "SOFT_KEYBOARD_QMP_OK phase1=1 phase2=1 persistent=1 m43=success/no-boot hardware_keys=a-ret-backspace-a-ret hardware_pairs=6 soft_focus=136/184 soft_keys=a-enter-backspace-a-enter soft_contacts=5 launcher_focus=72/96 hidden_key=88/376 hidden_text=0 visibility=show-hide-show overlay=56/336/208/96 key_rects=64/344/48/64-120/344/64/64-192/344/64/64 colors=181b24/3b82f6/ef4444/22c55e field=112/160/96/16 committed_pixels=128 screenshots=visible-hidden-final screenshot_sha256=$FINAL_SHA256 markers=1/1/1/1/1/5/1/1/1/1/5/1/1/1"
