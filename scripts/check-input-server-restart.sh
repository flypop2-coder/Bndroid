#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M47 InputServer restart runtime." >&2
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

TARGET_ROOT="${BNDROID_INPUT_SERVER_RESTART_TARGET_DIR:-$WORKSPACE_ROOT/target/input-server-restart-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=input-server-restart-runtime \
  BNDROID_KERNEL_FEATURES=input-server-restart-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M47 InputServer restart kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-input-server-restart.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
PRE_PPM="$TMP_DIR/input-server-restart-pre.ppm"
GAP_PPM="$TMP_DIR/input-server-restart-gap.ppm"
POST_PPM="$TMP_DIR/input-server-restart-post.ppm"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

ARMED_PREFIX="INPUT_SERVER_RESTART_ARMED"
PERMISSION_PREFIX="INPUT_SERVER_RESTART_PERMISSION_OK"
GAP_READY_PREFIX="INPUT_SERVER_RESTART_GAP_READY"
BACKOFF_PREFIX="INPUT_SERVER_RESTART_BACKOFF_OK"
REACQUIRE_PREFIX="INPUT_SERVER_REACQUIRE_OK"
RESYNC_PREFIX="INPUT_SERVER_RESYNC_OK"
ROUTE_PREFIX="INPUT_SERVER_RESTART_ROUTE_OK"
SUCCESS_PREFIX="INPUT_SERVER_RESTART_OK"
BOOT_MARKER="BOOT_OK: M47 bounded InputServer restart, backoff, Surface resync, and permission denial verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
M46_MARKER_REGEX='^(INPUT_SERVER_SURFACE_RESTART_ARMED|INPUT_SERVER_ROUTE_GAP_READY|INPUT_SERVER_ROUTE_EPOCH_OK|INPUT_SERVER_ROUTE_GAP_OK|INPUT_SERVER_CAPTURE_CANCEL_OK|INPUT_SERVER_SURFACE_REBIND_OK|INPUT_SERVER_SURFACE_RESTART_OK)( |$)'
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK|TEXT_INPUT_OK|SOFT_KEYBOARD_OK|INPUT_SERVER_KERNEL_OK|INPUT_SERVER_OK) '

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M47 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU writes directly to the serial file. Avoid classifying an incomplete
  # evidence line while the guest is still emitting it.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|input[- ]server restart runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M47 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0([[:space:]]|$)|route=surface-direct|surface_fallback=[1-9][0-9]*|legacy_(key_reads|path)=[1-9][0-9]*' "$NORMALIZED_LOG"; then
    show_failure
    echo "M47 fell back to a retired SurfaceServer-owned or legacy input path." >&2
    exit 1
  fi
  if grep -Eq "$M46_MARKER_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M47 leaked an M46 SurfaceServer-restart marker." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M47 published an unrelated leaf runtime's success evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M47 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
}

wait_for_prefix_once() {
  local prefix="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    reject_failure
    local count
    count="$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)"
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

wait_for_exact_once() {
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

assert_prefix_once() {
  local prefix="$1"
  local description="$2"
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  if [[ "$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "$description changed while advancing the M47 transcript." >&2
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
raw_x, raw_y = 13970, 12587
expected_x, expected_y = 136, 184
for raw, extent, expected in ((raw_x, 320, expected_x), (raw_y, 480, expected_y)):
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

def absolute():
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "abs", "data": {"axis": "x", "value": raw_x}},
        {"type": "abs", "data": {"axis": "y", "value": raw_y}},
    ]}})
    time.sleep(0.05)

def touch(down):
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "btn", "data": {"down": down, "button": "touch"}},
    ]}})
    time.sleep(0.05)

command({"execute": "qmp_capabilities"})
if action == "move":
    # The restart trigger is one ABS-only report. No touch edge is allowed.
    absolute()
elif action == "touch-down":
    # QEMU must retain the exact coordinate established before the restart.
    touch(True)
elif action == "touch-up":
    # No intervening ABS event is emitted before the retained-coordinate release.
    touch(False)
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

validate_screenshots() {
  python3 - "$PRE_PPM" "$GAP_PPM" "$POST_PPM" <<'PY'
import hashlib
from collections import Counter
import sys

WIDTH, HEIGHT = 320, 480
PHONE_SURFACE = (56, 64, 208, 368)
# SurfaceServer remains resident in M47. Its fixed App window is (48,80,112,160)
# in Surface coordinates, and the recovery badge is (8,8,32,24) in that App.
DAMAGE = (112, 152, 32, 24)
# The ABS-only fault trigger also makes the software cursor visible at the
# retained App coordinate. Keep that layer change separate from the recovery
# badge instead of treating the cursor as arbitrary Surface damage.
CURSOR_MOVE = (136, 184, 12, 22)
CORE_COLORS = {
    bytes.fromhex(value)
    for value in (
        "10162d", "162436", "f8fafc", "1f2937", "2e66f5",
        "29a36a", "385a7c", "844ec7",
    )
}
ORANGE = bytes.fromhex("f97300")
CYAN = bytes.fromhex("14b8a6")
RECOVERY_COLORS = {ORANGE, CYAN}

def load(path, phase):
    with open(path, "rb") as source:
        if source.readline() != b"P6\n":
            raise SystemExit(f"{path}: not a binary PPM")
        dimensions = source.readline()
        while dimensions.startswith(b"#"):
            dimensions = source.readline()
        if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
            raise SystemExit(f"{path}: expected exact 320x480 8-bit RGB")
        pixels = source.read()
    if len(pixels) != WIDTH * HEIGHT * 3:
        raise SystemExit(f"{path}: payload has {len(pixels)} bytes")
    colors = {pixels[index:index + 3] for index in range(0, len(pixels), 3)}
    missing = CORE_COLORS - colors
    if missing:
        raise SystemExit(f"{path}: missing core UI colors {[c.hex() for c in missing]}")
    if phase == "pre":
        leaked = RECOVERY_COLORS & colors
        if leaked:
            raise SystemExit(f"{path}: pre phase leaked recovery colors {[c.hex() for c in leaked]}")
    elif phase == "gap":
        if ORANGE not in colors or CYAN in colors:
            raise SystemExit(f"{path}: gap phase is not orange-only recovery evidence")
    elif phase == "post":
        if CYAN not in colors or ORANGE in colors:
            raise SystemExit(f"{path}: post phase is not cyan-only recovery evidence")
    else:
        raise SystemExit(f"unknown screenshot phase {phase!r}")
    minimum_colors = len(CORE_COLORS) + (0 if phase == "pre" else 1)
    if len(colors) < minimum_colors:
        raise SystemExit(f"{path}: only {len(colors)} colors")
    return pixels

def pixel(pixels, x, y):
    offset = (y * WIDTH + x) * 3
    return pixels[offset:offset + 3]

def inside(rect, x, y):
    rx, ry, width, height = rect
    return rx <= x < rx + width and ry <= y < ry + height

dx, dy, dw, dh = DAMAGE
if not inside(PHONE_SURFACE, dx, dy) or not inside(PHONE_SURFACE, dx + dw - 1, dy + dh - 1):
    raise SystemExit("fixed M47 damage rectangle escaped the phone/App surface")

pre = load(sys.argv[1], "pre")
gap = load(sys.argv[2], "gap")
post = load(sys.argv[3], "post")
for y in range(dy, dy + dh):
    for x in range(dx, dx + dw):
        if pixel(gap, x, y) != ORANGE or pixel(post, x, y) != CYAN:
            raise SystemExit("M47 recovery badge is not an exact 32x24 orange/teal commit")
hashes = []
for path in sys.argv[1:]:
    with open(path, "rb") as source:
        hashes.append(hashlib.sha256(source.read()).hexdigest())
if len(set(hashes)) != 3:
    raise SystemExit(f"M47 screenshots are not three distinct phases: {hashes!r}")

pre_gap = []
gap_post = []
for y in range(HEIGHT):
    for x in range(WIDTH):
        if pixel(pre, x, y) != pixel(gap, x, y):
            pre_gap.append((x, y))
        if pixel(gap, x, y) != pixel(post, x, y):
            gap_post.append((x, y))

for phase, changes in (("pre/gap", pre_gap), ("gap/post", gap_post)):
    if not (1 <= len(changes) <= 8192):
        raise SystemExit(f"unexpected {phase} diff ledger: {len(changes)}")
    outside_surface = [(x, y) for x, y in changes if not inside(PHONE_SURFACE, x, y)]
    if outside_surface:
        raise SystemExit(f"{phase} changed outside Surface: {outside_surface[0]}")
damage_pixels = {
    (x, y)
    for y in range(dy, dy + dh)
    for x in range(dx, dx + dw)
}
if set(gap_post) != damage_pixels:
    raise SystemExit(f"gap/post diff is not the exact fixed App damage: {len(gap_post)}")
if not damage_pixels.issubset(pre_gap):
    raise SystemExit("pre/gap diff omitted part of the fixed App damage")

cursor_changes = set(pre_gap) - damage_pixels
if len(cursor_changes) != 122 or any(
    not inside(CURSOR_MOVE, x, y) for x, y in cursor_changes
):
    raise SystemExit(f"pre/gap cursor ledger is not exact: {len(cursor_changes)}")
cursor_x = [x for x, _ in cursor_changes]
cursor_y = [y for _, y in cursor_changes]
if (min(cursor_x), min(cursor_y), max(cursor_x), max(cursor_y)) != (136, 184, 145, 205):
    raise SystemExit("pre/gap cursor footprint moved outside its retained coordinate")
if Counter(pixel(pre, x, y) for x, y in cursor_changes) != Counter({
    bytes.fromhex("844ec7"): 90,
    bytes.fromhex("385a7c"): 32,
}) or Counter(pixel(gap, x, y) for x, y in cursor_changes) != Counter({
    bytes.fromhex("111827"): 50,
    bytes.fromhex("f8fafc"): 36,
    bytes.fromhex("52317c"): 20,
    bytes.fromhex("23384d"): 16,
}):
    raise SystemExit("pre/gap cursor colors do not prove the exact software-cursor layer")

print(*hashes)
PY
}

validate_final_evidence() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import re
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    lines = [line.rstrip("\n") for line in source]

def one(prefix):
    found = [line for line in lines if line.startswith(prefix + " ")]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} marker, observed {found!r}")
    return found[0]

def fields(prefix, required_keys):
    line = one(prefix)
    result = {}
    for token in line.split()[1:]:
        if "=" not in token:
            raise SystemExit(f"{prefix}: non-field token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in result:
            raise SystemExit(f"{prefix}: invalid or duplicate field {token!r}")
        result[key] = value
    missing = set(required_keys) - set(result)
    if missing:
        raise SystemExit(f"{prefix}: missing required fields {sorted(missing)!r}")
    return line, result

def require(marker, observed, expected):
    for key, value in expected.items():
        if observed.get(key) != value:
            raise SystemExit(
                f"{marker}: {key}={observed.get(key)!r}, expected {value!r}"
            )

armed_line, armed = fields("INPUT_SERVER_RESTART_ARMED", {
    "wire", "input_pid", "session", "surface_session", "epoch",
    "state", "focus", "capture", "budget",
})
require("armed", armed, {
    "wire": "BIP1", "session": "1", "surface_session": "1", "epoch": "1",
    "state": "active", "focus": "launcher", "capture": "none", "budget": "1",
})

permission_line, permission = fields("INPUT_SERVER_RESTART_PERMISSION_OK", {
    "caller", "syscall", "status", "audits", "handles_delta", "sessions_delta",
})
require("permission", permission, {
    "caller": "surface", "syscall": "input_acquire", "status": "permission_denied",
    "audits": "1", "handles_delta": "0", "sessions_delta": "0",
})

gap_line, gap = fields("INPUT_SERVER_RESTART_GAP_READY", {
    "floor", "broker", "surface", "pending",
})
require("gap-ready", gap, {
    "floor": "1", "broker": "unbound", "surface": "alive", "pending": "0",
})

backoff_line, backoff = fields("INPUT_SERVER_RESTART_BACKOFF_OK", {
    "attempt", "requested_ns", "timeout", "early_spawn", "budget",
})
require("backoff", backoff, {
    "attempt": "1", "requested_ns": "30000000", "timeout": "1",
    "early_spawn": "0", "budget": "1/1",
})

reacquire_line, reacquire = fields("INPUT_SERVER_REACQUIRE_OK", {
    "wire", "old_pid", "new_pid", "sessions", "surface_session", "epochs",
    "acquired", "errors",
})
require("reacquire", reacquire, {
    "wire": "BIR1", "sessions": "1/2", "surface_session": "1",
    "epochs": "1/2", "acquired": "1", "errors": "0",
})

resync_line, resync = fields("INPUT_SERVER_RESYNC_OK", {
    "snapshot", "routes", "focus", "capture", "text", "floor", "bic", "bie",
})
require("resync", resync, {
    "snapshot": "1", "routes": "2", "focus": "launcher", "capture": "none",
    "text": "none", "floor": "1", "bic": "6", "bie": "7",
})

route_line, route = fields("INPUT_SERVER_RESTART_ROUTE_OK", {
    "sequence", "target", "focus", "capture", "final_capture", "text_delta",
})
require("route", route, {
    "sequence": "2-3", "target": "app", "focus": "launcher/app",
    "capture": "down/up", "final_capture": "none", "text_delta": "0",
})

runtime_line, runtime = fields("INPUT_SERVER_RESTART_OK", {
    "abi", "protocol", "wires", "sessions", "surface_session", "epochs",
    "restart", "backoff", "budget", "quarantine", "broker", "releases",
    "unbound_drops", "surface_reacquires", "surface_fallback", "processes",
    "handles", "endpoints", "pairs", "waits", "errors",
})
require("runtime", runtime, {
    "abi": "23", "protocol": "1", "wires": "BIR1/BIP1/BIC1/BIE1",
    "sessions": "1/2", "surface_session": "1", "epochs": "1/2",
    "restart": "1", "backoff": "fixed-30ms", "budget": "1/1",
    "quarantine": "host-verified", "broker": "3/3/0/1", "releases": "1",
    "unbound_drops": "0", "surface_reacquires": "0", "surface_fallback": "0",
    "processes": "11/2/2/9", "handles": "36", "endpoints": "30",
    "pairs": "15", "waits": "9/2/7", "errors": "0",
})

old_pid = int(armed["input_pid"], 0)
reacquire_old_pid = int(reacquire["old_pid"], 0)
new_pid = int(reacquire["new_pid"], 0)
if any(pid <= 0 for pid in (old_pid, reacquire_old_pid, new_pid)) or old_pid == new_pid:
    raise SystemExit(f"InputServer identity did not advance: {old_pid}/{new_pid}")
if reacquire_old_pid != old_pid:
    raise SystemExit("reacquire old_pid does not match the armed InputServer")
if (old_pid & 0xFFFFFFFF) != (new_pid & 0xFFFFFFFF):
    raise SystemExit("InputServer restart did not reuse one process slot")
if (new_pid >> 32) != (old_pid >> 32) + 1:
    raise SystemExit("InputServer generation did not advance exactly once")

boot = "BOOT_OK: M47 bounded InputServer restart, backoff, Surface resync, and permission denial verified"
if lines.count(boot) != 1:
    raise SystemExit("M47 boot marker is absent or duplicated")
ordered = [
    permission_line, armed_line, gap_line, backoff_line, reacquire_line,
    resync_line, route_line, runtime_line, boot,
]
positions = [lines.index(line) for line in ordered]
if positions != sorted(positions) or len(set(positions)) != len(positions):
    raise SystemExit(f"M47 proof markers are out of order: {positions!r}")

for marker in (
    "INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1",
    "POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1",
):
    if lines.count(marker) != 1:
        raise SystemExit(f"device marker {marker!r} is absent or duplicated")

m46_prefixes = (
    "INPUT_SERVER_SURFACE_RESTART_ARMED", "INPUT_SERVER_ROUTE_GAP_READY",
    "INPUT_SERVER_ROUTE_EPOCH_OK", "INPUT_SERVER_ROUTE_GAP_OK",
    "INPUT_SERVER_CAPTURE_CANCEL_OK", "INPUT_SERVER_SURFACE_REBIND_OK",
    "INPUT_SERVER_SURFACE_RESTART_OK",
)
if any(any(line == prefix or line.startswith(prefix + " ") for prefix in m46_prefixes) for line in lines):
    raise SystemExit("M46 proof marker leaked into M47")
if any(
    "input_method=in_surface" in line
    or "route=surface-direct" in line
    or re.search(r"\b(?:surface_fallback|legacy_key_reads|legacy_path)=[1-9][0-9]*\b", line)
    for line in lines
):
    raise SystemExit("retired SurfaceServer input ownership leaked into M47")
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

# M47 keeps SurfaceServer and its App window resident. The first QMP report is
# an ABS-only move that crosses the one-shot InputServer fault boundary. Only
# after the replacement has reacquired BIR1 and completed BIP1/BIC1 resync do
# we emit one retained-coordinate down/up pair through the new BIE1 route.
wait_for_prefix_once "$ARMED_PREFIX" "the M47 restart-armed marker"
qmp_action screendump "$PRE_PPM"
qmp_action move

wait_for_prefix_once "$PERMISSION_PREFIX" "the M47 permission-denial proof marker"
wait_for_prefix_once "$GAP_READY_PREFIX" "the M47 unbound-gap-ready marker"
wait_for_prefix_once "$BACKOFF_PREFIX" "the M47 fixed-backoff proof marker"
wait_for_prefix_once "$REACQUIRE_PREFIX" "the M47 InputServer reacquire proof marker"
wait_for_prefix_once "$RESYNC_PREFIX" "the M47 Surface resync proof marker"
assert_prefix_once "$ARMED_PREFIX" "The restart-armed marker"
assert_prefix_once "$PERMISSION_PREFIX" "The permission-denial marker"
assert_prefix_once "$GAP_READY_PREFIX" "The gap-ready marker"
assert_prefix_once "$BACKOFF_PREFIX" "The fixed-backoff marker"
assert_prefix_once "$REACQUIRE_PREFIX" "The reacquire marker"
qmp_action screendump "$GAP_PPM"

qmp_action touch-down
qmp_action touch-up
wait_for_prefix_once "$ROUTE_PREFIX" "the M47 post-resync route proof marker"
wait_for_prefix_once "$SUCCESS_PREFIX" "the final M47 runtime marker"
wait_for_exact_once "$BOOT_MARKER" "the final M47 boot marker"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
for prefix in \
  "$ARMED_PREFIX" "$PERMISSION_PREFIX" "$GAP_READY_PREFIX" "$BACKOFF_PREFIX" \
  "$REACQUIRE_PREFIX" "$RESYNC_PREFIX" "$ROUTE_PREFIX" "$SUCCESS_PREFIX"; do
  if [[ "$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M47 proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M47 did not publish each boot and input-device marker exactly once." >&2
  exit 1
fi

qmp_action screendump "$POST_PPM"
read -r PRE_SHA256 GAP_SHA256 POST_SHA256 < <(validate_screenshots)
validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M47 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M47 did not enter and normalize execution at EL1." >&2
  exit 1
fi

cp "$PRE_PPM" "$WORKSPACE_ROOT/target/bndroid-m47-input-server-restart-pre.ppm"
cp "$GAP_PPM" "$WORKSPACE_ROOT/target/bndroid-m47-input-server-restart-gap.ppm"
cp "$POST_PPM" "$WORKSPACE_ROOT/target/bndroid-m47-input-server-restart.ppm"

echo "INPUT_SERVER_RESTART_QMP_OK armed=1 permission=1 gap_ready=1 backoff=1 reacquire=1 resync=1 route=1 broker=kernel/userspace surface_session=stable pointer=136/184/move-down-up screenshots=pre-gap-post pre_sha256=$PRE_SHA256 gap_sha256=$GAP_SHA256 post_sha256=$POST_SHA256 markers=permission/gap/backoff/reacquire/resync/route/runtime/boot"
