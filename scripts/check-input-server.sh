#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M45 InputServer runtime." >&2
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

TARGET_ROOT="${BNDROID_INPUT_SERVER_TARGET_DIR:-$WORKSPACE_ROOT/target/input-server-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=input-server-runtime \
  BNDROID_KERNEL_FEATURES=input-server-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M45 InputServer kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-input-server.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
VISIBLE_PPM="$TMP_DIR/input-server-visible.ppm"
HIDDEN_PPM="$TMP_DIR/input-server-hidden.ppm"
FINAL_PPM="$TMP_DIR/input-server-final.ppm"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

PHASE1_MARKER="WINDOW_INPUT_PHASE1_READY"
PHASE2_MARKER="WINDOW_INPUT_PHASE2_READY"
PERSISTENT_INPUT_MARKER="PERSISTENT_WINDOW_INPUT_READY"
TEXT_POINTER_FOCUS_MARKER="TEXT_POINTER_FOCUS_READY"
TEXT_INPUT_READY_MARKER="TEXT_INPUT_READY"
TEXT_FOCUS_LOST_MARKER="TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0"
SOFT_POINTER_MARKER="SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6"
SOFT_READY_MARKER="SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible"
SOFT_HIDDEN_MARKER="SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent"
TEXT_RENDER_PREFIX="TEXT_FIELD_RENDER_OK"
SOFT_RENDER_PREFIX="SOFT_TEXT_RENDER_OK"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK) '

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M45 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU writes directly to the serial file. Do not classify an unterminated
  # tail while a long evidence line is still being emitted.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|input[- ]server runtime (failed|timed out)|soft[- ]keyboard runtime (failed|timed out)|text[- ]input runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M45 emitted a panic, userspace/InputServer/Surface failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0|route=surface-direct' "$NORMALIZED_LOG"; then
    show_failure
    echo "M45 fell back to the retired in-Surface input path." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M45 published an unrelated leaf runtime's success evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Ev '^BOOT_OK: M45( |$)' >/dev/null; then
    show_failure
    echo "M45 published a stale or unrelated BOOT_OK marker." >&2
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
    echo "$description changed while advancing the M45 transcript." >&2
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
positions = {
    "overlap": ((13970, 12587), (136, 184)),
    "launcher": ((7396, 6568), (72, 96)),
    "phone-outside": ((2055, 1369), (20, 20)),
    "soft-a": ((9040, 25722), (88, 376)),
    "soft-backspace": ((15614, 25722), (152, 376)),
    "soft-enter": ((23009, 25722), (224, 376)),
}

for raw_position, expected_position in positions.values():
    for raw, extent, expected in zip(raw_position, (320, 480), expected_position):
        mapped = raw * (extent - 1) // 32767
        previous = (raw - 1) * (extent - 1) // 32767
        if mapped != expected or previous == expected:
            raise SystemExit(f"raw ABS inverse changed for pixel {expected}: {raw}")

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
    x, y = positions[name][0]
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "abs", "data": {"axis": "x", "value": x}},
        {"type": "abs", "data": {"axis": "y", "value": y}},
    ]}})
    time.sleep(0.05)

def touch(down):
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "btn", "data": {"down": down, "button": "touch"}},
    ]}})
    time.sleep(0.05)

def key(qcode, down):
    command({"execute": "input-send-event", "arguments": {"events": [{
        "type": "key",
        "data": {"down": down, "key": {"type": "qcode", "data": qcode}},
    }]}})
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
    click("phone-outside")
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
    command({"execute": "screendump", "arguments": {
        "filename": argument, "format": "ppm",
    }})
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
    raise SystemExit(f"unexpected exact M45 hardware render transcript: {observed_text!r}")
if observed_soft != expected_soft:
    raise SystemExit(f"unexpected exact M45 soft render transcript: {observed_soft!r}")
PY
}

validate_screenshot() {
  local path="$1"
  local mode="$2"
  local cursor_x="${3:-0}"
  local cursor_y="${4:-0}"
  python3 - "$path" "$mode" "$cursor_x" "$cursor_y" <<'PY'
import hashlib
import sys

path, mode = sys.argv[1:3]
cursor_x, cursor_y = map(int, sys.argv[3:5])
with open(path, "rb") as source:
    if source.readline() != b"P6\n":
        raise SystemExit("M45 screenshot is not a binary PPM")
    dimensions = source.readline()
    while dimensions.startswith(b"#"):
        dimensions = source.readline()
    if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
        raise SystemExit("M45 screenshot is not exact 320x480 8-bit RGB")
    pixels = source.read()
if len(pixels) != 320 * 480 * 3:
    raise SystemExit(f"M45 screenshot payload has {len(pixels)} bytes")

def pixel(x, y):
    offset = (y * 320 + x) * 3
    return pixels[offset:offset + 3]

key_colors = {bytes.fromhex(value) for value in ("3b82f6", "ef4444", "22c55e")}
if mode == "hidden":
    for y in range(336, 432):
        for x in range(56, 264):
            if pixel(x, y) in key_colors:
                raise SystemExit(f"hidden InputServer overlay retained a key at ({x},{y})")
else:
    overlay = (56, 336, 208, 96)
    background = bytes.fromhex("181b24")
    rectangles = (
        ((64, 344, 48, 64), bytes.fromhex("3b82f6")),
        ((120, 344, 64, 64), bytes.fromhex("ef4444")),
        ((192, 344, 64, 64), bytes.fromhex("22c55e")),
    )
    def contains(rect, x, y):
        rx, ry, width, height = rect
        return rx <= x < rx + width and ry <= y < ry + height
    def arrow_class(x, y):
        if y < 14 and x <= y // 2:
            return x == 0 or x == y // 2 or y == 13
        if 10 <= y < 20 and 4 <= x < 8:
            return x == 4 or x == 7 or y == 10 or y == 19
        return None
    def blend(base, foreground, alpha):
        inverse = 255 - alpha
        return bytes((front * alpha + back * inverse + 127) // 255
                     for back, front in zip(base, foreground))
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
    counts = {background: 0, **{color: 0 for _, color in rectangles}}
    cursor_pixels = 0
    ox, oy, width, height = overlay
    for y in range(oy, oy + height):
        for x in range(ox, ox + width):
            base = background
            for rect, color in rectangles:
                if contains(rect, x, y):
                    base = color
                    break
            expected = compose_cursor(base, x, y)
            if pixel(x, y) != expected:
                raise SystemExit(f"InputServer overlay mismatch at ({x},{y})")
            counts[base] += 1
            cursor_pixels += expected != base
    expected_counts = {
        background: 8704,
        bytes.fromhex("3b82f6"): 3072,
        bytes.fromhex("ef4444"): 4096,
        bytes.fromhex("22c55e"): 4096,
    }
    if counts != expected_counts:
        raise SystemExit(f"unexpected InputServer overlay color ledger: {counts!r}")
    expected_cursor_pixels = 122 if (cursor_x, cursor_y) == (224, 376) else 0
    if cursor_pixels != expected_cursor_pixels:
        raise SystemExit(f"unexpected overlay cursor ledger: {cursor_pixels}")

# The final text state must be two committed 'a' glyphs regardless of whether
# the trusted overlay is currently visible.
field_x, field_y, field_width, field_height = 112, 160, 96, 16
field_background = bytes.fromhex("111118")
committed = bytes.fromhex("f4f4f5")
glyph_a = [0b00000, 0b01110, 0b10001, 0b00001,
           0b01111, 0b10001, 0b01111, 0b00000]
glyph = set()
for character in range(2):
    origin_x = character * 12
    for row, bits in enumerate(glyph_a):
        for column in range(5):
            if bits & (1 << (4 - column)):
                for dy in range(2):
                    for dx in range(2):
                        glyph.add((origin_x + column * 2 + dx, row * 2 + dy))
for local_y in range(field_height):
    for local_x in range(field_width):
        expected = committed if (local_x, local_y) in glyph else field_background
        if pixel(field_x + local_x, field_y + local_y) != expected:
            raise SystemExit(f"M45 text field mismatch at ({local_x},{local_y})")

with open(path, "rb") as source:
    print(hashlib.sha256(source.read()).hexdigest())
PY
}

validate_final_evidence() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    lines = [line.rstrip("\n") for line in source]

def one(prefix):
    found = [line for line in lines if line.startswith(prefix)]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} line, observed {found!r}")
    return found[0]

def fields(line):
    result = {}
    for token in line.split()[1:]:
        if "=" in token:
            key, value = token.split("=", 1)
            result[key] = value
    return result

catalog = fields(one("USER_IMAGE_CATALOG_OK "))
if catalog.get("count") != "8" or catalog.get("distinct") != "1":
    raise SystemExit(f"InputServer is not a distinct eighth userspace image: {catalog!r}")
if (int(catalog.get("input_server_bytes", "0"), 0) <= 0
        or int(catalog.get("input_server_digest", "0"), 0) <= 0):
    raise SystemExit("embedded InputServer ELF is empty or has no catalog digest")

kernel_line = one("INPUT_SERVER_KERNEL_OK ")
kernel = fields(kernel_line)
required_kernel = {
    "abi": "23", "capacity": "64", "pending": "0",
    "enqueued": "59", "dequeued": "59", "coalesced": "0",
    "sequence_next": "60", "process_capacity": "9", "dynamic_capacity": "8",
    "scheduler_contexts": "12", "route": "broker-only", "surface_fifo": "0",
}
for key, value in required_kernel.items():
    if kernel.get(key) != value:
        raise SystemExit(f"kernel InputServer field {key}={kernel.get(key)!r}, expected {value!r}")
for key in ("pid", "session", "high_water"):
    if int(kernel.get(key, "0"), 0) <= 0:
        raise SystemExit(f"kernel InputServer field {key} is not positive: {kernel!r}")
if int(kernel["high_water"], 0) > 64:
    raise SystemExit(f"kernel InputServer high-water exceeds capacity: {kernel!r}")
if int(kernel["dequeued"], 0) > int(kernel["enqueued"], 0):
    raise SystemExit(f"InputServer dequeued more events than the broker enqueued: {kernel!r}")

text = fields(one("TEXT_INPUT_OK "))
required_text = {
    "abi": "23", "protocol": "1", "prefix": "m45",
    "wires": "BTI1/BTE1+BIC1/BIE1", "session": "1", "window": "app2",
    "surface_fifo": "0/0", "input_method": "in_input_server",
    "input_server": "1", "process_delta": "1", "final_state": "ready",
    "final_app_resident": "1", "waits": "9/2/7",
}
for key, value in required_text.items():
    if text.get(key) != value:
        raise SystemExit(f"M45 text prefix field {key}={text.get(key)!r}, expected {value!r}")
text_broker = text.get("broker", "").split("/")
if text_broker[:3] != ["32", "32", "0"] or len(text_broker) != 4:
    raise SystemExit(f"unexpected hardware-prefix broker ledger: {text_broker!r}")
if not (1 <= int(text_broker[3], 0) <= 64) or text.get("next") != "33":
    raise SystemExit(f"unexpected hardware-prefix broker high-water/sequence: {text!r}")

soft = fields(one("SOFT_KEYBOARD_OK "))
required_soft = {
    "abi": "23", "protocol": "1", "prefix": "m45",
    "input_method": "in_input_server", "input_server": "1",
    "process_delta": "1", "surface_fifo": "0/0", "windows": "2",
    "overlay": "system/nonfocusable", "source": "tablet",
    "visibility": "show-hide-show", "hidden_text": "0", "state": "aa",
    "waits": "9/2/7", "final_state": "ready", "final_app_resident": "1",
}
for key, value in required_soft.items():
    if soft.get(key) != value:
        raise SystemExit(f"M45 soft-input field {key}={soft.get(key)!r}, expected {value!r}")
soft_broker = soft.get("broker", "").split("/")
if soft_broker[:3] != ["59", "59", "0"] or len(soft_broker) != 5:
    raise SystemExit(f"unexpected final soft-input broker ledger: {soft_broker!r}")
if not (1 <= int(soft_broker[3], 0) <= 64) or soft_broker[4] != "0" or soft.get("next") != "60":
    raise SystemExit(f"unexpected final soft-input broker high-water/coalescing/sequence: {soft!r}")

runtime_line = one("INPUT_SERVER_OK ")
runtime = fields(runtime_line)
required_runtime = {
    "abi": "23", "protocol": "1", "wires": "BIC1/BIE1", "wire": "64",
    "owner": "unique", "capacity": "64", "surface_fifo": "0/0",
    "events": "59/59/0", "coalesced": "0", "sequence": "1-59", "next": "60",
    "physical": "pointer47+key12",
    "legacy_key_reads": "0", "routes": "generation-qualified",
    "focus": "server", "capture": "server", "ime": "server",
    "processes": "10/1/1/9", "process_capacity": "9/8",
    "input_server": "1/3", "handles": "36", "endpoints": "30",
    "pairs": "15/2", "waits": "9/2/7", "topology": "resident",
    "final_state": "ready", "final_app_resident": "1",
}
for key, value in required_runtime.items():
    if runtime.get(key) != value:
        raise SystemExit(f"M45 runtime field {key}={runtime.get(key)!r}, expected {value!r}")
if not (1 <= int(runtime.get("high_water", "0"), 0) <= 64):
    raise SystemExit(f"unexpected InputServer high-water mark: {runtime!r}")
if runtime.get("pid") != kernel.get("pid") or runtime.get("session") != kernel.get("session"):
    raise SystemExit(f"kernel/userspace InputServer ownership diverged: {kernel!r} / {runtime!r}")
if runtime.get("high_water") != kernel.get("high_water") or soft_broker[3] != kernel.get("high_water"):
    raise SystemExit("final broker high-water ledgers disagree")

boot = one("BOOT_OK: ")
if boot != "BOOT_OK: M45 dedicated InputServer routing, capture, and input-method ownership verified":
    raise SystemExit(f"unexpected M45 boot marker: {boot!r}")

exact = {
    "WINDOW_INPUT_PHASE1_READY": 1,
    "WINDOW_INPUT_PHASE2_READY": 1,
    "PERSISTENT_WINDOW_INPUT_READY": 1,
    "TEXT_POINTER_FOCUS_READY": 1,
    "TEXT_INPUT_READY": 1,
    "TEXT_INPUT_FOCUS_LOST focus=launcher/6 session=1 dropped=0": 1,
    "SOFT_KEYBOARD_POINTER_READY prefix=m43 focus=launcher/6": 1,
    "SOFT_KEYBOARD_READY session=2 focus=app/7 overlay=visible": 1,
    "SOFT_KEYBOARD_HIDDEN session=2 focus=launcher/8 overlay=absent": 1,
    "INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1": 1,
    "POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1": 1,
}
for marker, expected in exact.items():
    observed = lines.count(marker)
    if observed != expected:
        raise SystemExit(f"marker {marker!r}: observed {observed}, expected {expected}")
if sum(line.startswith("TEXT_FIELD_RENDER_OK ") for line in lines) != 5:
    raise SystemExit("M45 did not publish exactly five hardware text renders")
if sum(line.startswith("SOFT_TEXT_RENDER_OK ") for line in lines) != 5:
    raise SystemExit("M45 did not publish exactly five soft text renders")
if any("input_method=in_surface" in line or "input_server=0" in line for line in lines):
    raise SystemExit("retired in-Surface input ownership leaked into M45")
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

# Replay the full M41/M42/M43/M44 interaction ledger through the dedicated
# broker. This exercises captured out-of-bounds pointer motion, persistent
# window replacement, focus-scoped hardware input, and trusted overlay input.
wait_for_exact_marker "$PHASE1_MARKER" "the phase-1 input-ready marker"
qmp_action phase1
wait_for_exact_marker "$PHASE2_MARKER" "the phase-2 input-ready marker"
assert_marker_once "$PHASE1_MARKER" "The phase-1 marker"
qmp_action phase2
wait_for_exact_marker "$PERSISTENT_INPUT_MARKER" "the persistent-session input-ready marker"
qmp_action phase3

wait_for_exact_marker "$TEXT_POINTER_FOCUS_MARKER" "the App-focus pointer-ready marker"
qmp_action focus-app
wait_for_exact_marker "$TEXT_INPUT_READY_MARKER" "the active text-input session marker"
qmp_action key a
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 1 "the A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 2 "the Enter-commit render marker"
qmp_action key backspace
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 3 "the Backspace-delete render marker"
qmp_action key a
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 4 "the second A-preedit render marker"
qmp_action key ret
wait_for_prefix_count "$TEXT_RENDER_PREFIX" 5 "the final Enter-commit render marker"
qmp_action focus-launcher
wait_for_exact_marker "$TEXT_FOCUS_LOST_MARKER" "the text-input focus-loss marker"
qmp_action key a
wait_for_prefix_count "TEXT_INPUT_OK" 1 "the M43 semantic prefix success marker"

wait_for_exact_marker "$SOFT_POINTER_MARKER" "the soft-keyboard pointer-ready marker"
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
VISIBLE_SHA256="$(validate_screenshot "$VISIBLE_PPM" visible 224 376)"

qmp_action focus-launcher
wait_for_exact_marker "$SOFT_HIDDEN_MARKER" "the hidden session-2 soft keyboard marker"
qmp_action screendump "$HIDDEN_PPM"
validate_screenshot "$HIDDEN_PPM" hidden >/dev/null

# The old overlay coordinate is now ordinary Launcher content and must not be
# routed into the hidden trusted input method.
qmp_action soft-a
sleep 0.20
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
if [[ "$(grep -Ec "^${SOFT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" \
  || "$(grep -Ec "^${TEXT_RENDER_PREFIX} .+\$" "$NORMALIZED_LOG" || true)" != "5" ]]; then
  show_failure
  echo "A contact in the hidden InputServer overlay bounds produced text." >&2
  exit 1
fi

qmp_action focus-app
deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if [[ "$(grep -c '^INPUT_SERVER_OK ' "$NORMALIZED_LOG" || true)" == "1" \
    && "$(grep -c '^BOOT_OK: M45 ' "$NORMALIZED_LOG" || true)" == "1" ]]; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    show_failure
    echo "QEMU exited before final M45 evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
qmp_action screendump "$FINAL_PPM"
FINAL_SHA256="$(validate_screenshot "$FINAL_PPM" visible 136 184)"
validate_render_transcripts
validate_final_evidence

if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M45 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M45 did not enter and normalize execution at EL1." >&2
  exit 1
fi

cp "$VISIBLE_PPM" "$WORKSPACE_ROOT/target/bndroid-m45-input-server-visible.ppm"
cp "$HIDDEN_PPM" "$WORKSPACE_ROOT/target/bndroid-m45-input-server-hidden.ppm"
cp "$FINAL_PPM" "$WORKSPACE_ROOT/target/bndroid-m45-input-server.ppm"

echo "INPUT_SERVER_QMP_OK phase1=1 phase2=1 persistent=1 broker=kernel/userspace surface_fallback=0 hardware_keys=a-ret-backspace-a-ret hardware_pairs=6 soft_keys=a-enter-backspace-a-enter soft_contacts=5 hidden_text=0 visibility=show-hide-show screenshots=visible-hidden-final visible_sha256=$VISIBLE_SHA256 final_sha256=$FINAL_SHA256 markers=kernel/text/runtime/boot"
