#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp mkdir; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M48 ServiceSupervisor runtime." >&2
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

TARGET_ROOT="${BNDROID_SERVICE_SUPERVISOR_TARGET_DIR:-$WORKSPACE_ROOT/target/service-supervisor-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=service-supervisor-runtime \
  BNDROID_KERNEL_FEATURES=service-supervisor-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M48 ServiceSupervisor kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-service-supervisor-m48.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
PRE_PPM="$TMP_DIR/service-supervisor-pre.ppm"
GAP_PPM="$TMP_DIR/service-supervisor-gap.ppm"
RECOVERED_PPM="$TMP_DIR/service-supervisor-recovered.ppm"
DEGRADED_PPM="$TMP_DIR/service-supervisor-degraded.ppm"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m48"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

ARMED_PREFIX="SERVICE_SUPERVISOR_ARMED"
FIRST_FAULT_PREFIX="SERVICE_SUPERVISOR_FIRST_FAULT_OK"
RESYNC_READY_PREFIX="SERVICE_SUPERVISOR_RESYNC_READY"
HEALTH_PREFIX="SERVICE_SUPERVISOR_HEALTH_OK"
WATCHDOG_PREFIX="SERVICE_SUPERVISOR_WATCHDOG_OK"
QUARANTINE_PREFIX="SERVICE_SUPERVISOR_QUARANTINE_OK"
SUCCESS_PREFIX="SERVICE_SUPERVISOR_OK"
BOOT_MARKER="BOOT_OK: M48 generic ServiceSupervisor watchdog, runtime quarantine, and degraded UI verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"

STALE_INPUT_RUNTIME_REGEX='^(INPUT_SERVER_KERNEL_OK|INPUT_SERVER_OK|INPUT_SERVER_SURFACE_RESTART_ARMED|INPUT_SERVER_ROUTE_GAP_READY|INPUT_SERVER_ROUTE_EPOCH_OK|INPUT_SERVER_ROUTE_GAP_OK|INPUT_SERVER_CAPTURE_CANCEL_OK|INPUT_SERVER_SURFACE_REBIND_OK|INPUT_SERVER_SURFACE_RESTART_OK|INPUT_SERVER_RESTART_PERMISSION_OK|INPUT_SERVER_RESTART_ARMED|INPUT_SERVER_RESTART_GAP_READY|INPUT_SERVER_RESTART_BACKOFF_OK|INPUT_SERVER_REACQUIRE_OK|INPUT_SERVER_RESYNC_OK|INPUT_SERVER_RESTART_ROUTE_OK|INPUT_SERVER_RESTART_OK)( |$)'
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK|TEXT_INPUT_OK|SOFT_KEYBOARD_OK)( |$)'

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M48 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU appends directly to the serial file. Do not classify a marker until
  # its terminating newline is present.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|service[- ]supervisor runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAIL|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M48 emitted a panic, failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0([[:space:]]|$)|route=surface-direct|surface_fallback=[1-9][0-9]*|legacy_(key_reads|path)=[1-9][0-9]*|^(SURFACE_INPUT|SURFACE_KEY)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M48 fell back to a retired SurfaceServer-owned or legacy input path." >&2
    exit 1
  fi
  if grep -Eq "$STALE_INPUT_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M48 leaked M45, M46, or M47 InputServer runtime evidence." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M48 published an unrelated leaf runtime's success evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M48 published a stale or unrelated BOOT_OK marker." >&2
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
    sleep 0.01
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
    sleep 0.01
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
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
    # The first fault is triggered by one ABS-only report, without a touch edge.
    absolute()
elif action == "touch-down":
    # The replacement must route the coordinate retained before the fault.
    touch(True)
elif action == "touch-up":
    # No intervening ABS report is allowed before this retained-coordinate edge.
    touch(False)
elif action == "screendump":
    if not argument:
        raise SystemExit("screendump requires an output path")
    command({"execute": "stop"})
    command({"execute": "screendump", "arguments": {
        "filename": argument, "format": "ppm",
    }})
    command({"execute": "cont"})
else:
    raise SystemExit(f"unsupported QMP action: {action}")
connection.close()
PY
}

validate_screenshots() {
  python3 - "$PRE_PPM" "$GAP_PPM" "$RECOVERED_PPM" "$DEGRADED_PPM" <<'PY'
import hashlib
from collections import Counter
import sys

WIDTH, HEIGHT = 320, 480
PHONE_SURFACE = (56, 64, 208, 368)
BADGE = (112, 152, 32, 24)
CURSOR_MOVE = (136, 184, 12, 22)
CORE_COLORS = {
    bytes.fromhex(value)
    for value in (
        "10162d", "162436", "f8fafc", "1f2937", "2e66f5",
        "29a36a", "385a7c", "844ec7",
    )
}
ORANGE = bytes.fromhex("f97300")
TEAL = bytes.fromhex("14b8a6")
RED = bytes.fromhex("dc2626")
PHASE_COLORS = {ORANGE, TEAL, RED}
EXPECTED_SHA256 = {
    "pre": "9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992",
    "gap": "c85fbd7ef65f5ae3b47b696b9fa25ef104bd2e692d7b20e3852e169d46985324",
    "recovered": "5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65",
    "degraded": "32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff",
}

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
        raise SystemExit(f"{path}: missing core UI colors {[color.hex() for color in missing]}")
    expected = {
        "pre": None,
        "gap": ORANGE,
        "recovered": TEAL,
        "degraded": RED,
    }[phase]
    if expected is None:
        leaked = PHASE_COLORS & colors
        if leaked:
            raise SystemExit(f"{path}: pre phase leaked badge colors {[color.hex() for color in leaked]}")
    else:
        leaked = (PHASE_COLORS - {expected}) & colors
        if expected not in colors or leaked:
            raise SystemExit(
                f"{path}: {phase} phase is not exclusive {expected.hex()} badge evidence"
            )
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

bx, by, bw, bh = BADGE
if not inside(PHONE_SURFACE, bx, by) or not inside(
    PHONE_SURFACE, bx + bw - 1, by + bh - 1
):
    raise SystemExit("fixed M48 badge escaped the phone/App surface")
cx, cy, cw, ch = CURSOR_MOVE
if not inside(PHONE_SURFACE, cx, cy) or not inside(
    PHONE_SURFACE, cx + cw - 1, cy + ch - 1
):
    raise SystemExit("fixed M48 cursor bounds escaped the phone/App surface")

phase_names = ("pre", "gap", "recovered", "degraded")
pixels_by_phase = {
    phase: load(path, phase)
    for phase, path in zip(phase_names, sys.argv[1:], strict=True)
}
for phase, expected in (("gap", ORANGE), ("recovered", TEAL), ("degraded", RED)):
    pixels = pixels_by_phase[phase]
    for y in range(by, by + bh):
        for x in range(bx, bx + bw):
            if pixel(pixels, x, y) != expected:
                raise SystemExit(
                    f"M48 {phase} badge is not an exact 32x24 {expected.hex()} commit"
                )

hashes = []
for phase, path in zip(phase_names, sys.argv[1:], strict=True):
    with open(path, "rb") as source:
        digest = hashlib.sha256(source.read()).hexdigest()
    hashes.append(digest)
    expected = EXPECTED_SHA256.get(phase)
    if expected is not None and digest != expected:
        raise SystemExit(
            f"M48 {phase} screenshot SHA-256 changed: {digest}, expected {expected}"
        )
if len(set(hashes)) != 4:
    raise SystemExit(f"M48 screenshots are not four distinct phases: {hashes!r}")

pre = pixels_by_phase["pre"]
gap = pixels_by_phase["gap"]
recovered = pixels_by_phase["recovered"]
degraded = pixels_by_phase["degraded"]

def changes(before, after):
    return {
        (x, y)
        for y in range(HEIGHT)
        for x in range(WIDTH)
        if pixel(before, x, y) != pixel(after, x, y)
    }

pre_gap = changes(pre, gap)
gap_recovered = changes(gap, recovered)
recovered_degraded = changes(recovered, degraded)
badge_pixels = {
    (x, y)
    for y in range(by, by + bh)
    for x in range(bx, bx + bw)
}
for phase, changed in (
    ("pre/gap", pre_gap),
    ("gap/recovered", gap_recovered),
    ("recovered/degraded", recovered_degraded),
):
    outside_surface = [(x, y) for x, y in changed if not inside(PHONE_SURFACE, x, y)]
    if outside_surface:
        raise SystemExit(f"{phase} changed outside Surface: {outside_surface[0]}")

if len(pre_gap) != 768 + 122 or not badge_pixels.issubset(pre_gap):
    raise SystemExit(f"pre/gap diff is not exact badge plus cursor damage: {len(pre_gap)}")
if gap_recovered != badge_pixels:
    raise SystemExit(f"gap/recovered diff is not the exact 768-pixel badge: {len(gap_recovered)}")
if recovered_degraded != badge_pixels:
    raise SystemExit(
        f"recovered/degraded diff is not the exact 768-pixel badge: {len(recovered_degraded)}"
    )

cursor_changes = pre_gap - badge_pixels
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

PREFIXES = (
    "SERVICE_SUPERVISOR_ARMED",
    "SERVICE_SUPERVISOR_FIRST_FAULT_OK",
    "SERVICE_SUPERVISOR_RESYNC_READY",
    "SERVICE_SUPERVISOR_HEALTH_OK",
    "SERVICE_SUPERVISOR_WATCHDOG_OK",
    "SERVICE_SUPERVISOR_QUARANTINE_OK",
    "SERVICE_SUPERVISOR_OK",
)

def fields(prefix, required_keys):
    found = [line for line in lines if line.startswith(prefix + " ")]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} marker, observed {found!r}")
    result = {}
    for token in found[0].split()[1:]:
        if "=" not in token:
            raise SystemExit(f"{prefix}: non-field token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in result:
            raise SystemExit(f"{prefix}: invalid or duplicate field {token!r}")
        result[key] = value
    required_keys = set(required_keys)
    if set(result) != required_keys:
        missing = sorted(required_keys - set(result))
        extra = sorted(set(result) - required_keys)
        raise SystemExit(f"{prefix}: schema mismatch missing={missing!r} extra={extra!r}")
    return found[0], result

def require(marker, observed, expected):
    for key, value in expected.items():
        if observed[key] != value:
            raise SystemExit(
                f"{marker}: {key}={observed[key]!r}, expected {value!r}"
            )

armed_line, armed = fields("SERVICE_SUPERVISOR_ARMED", {
    "wire", "input_pid", "generation", "session", "epoch",
    "surface_session", "budget",
})
require("armed", armed, {
    "wire": "BSH1", "session": "1", "epoch": "1",
    "surface_session": "1", "budget": "1",
})

fault_line, fault = fields("SERVICE_SUPERVISOR_FIRST_FAULT_OK", {
    "class", "attempt", "budget", "gap", "broker",
})
require("first-fault", fault, {
    "class": "process-exit", "attempt": "1", "budget": "1",
    "gap": "1", "broker": "unbound",
})

resync_line, resync = fields("SERVICE_SUPERVISOR_RESYNC_READY", {
    "sessions", "epochs", "routes", "floor", "bic", "bie",
})
require("resync", resync, {
    "sessions": "1/2", "epochs": "1/2", "routes": "2",
    "floor": "1", "bic": "6", "bie": "7",
})

health_line, health = fields("SERVICE_SUPERVISOR_HEALTH_OK", {
    "probes", "healthy", "input_pid", "generation", "timeout_ns",
})
require("health", health, {
    "probes": "1", "healthy": "1", "timeout_ns": "200000000",
})

watchdog_line, watchdog = fields("SERVICE_SUPERVISOR_WATCHDOG_OK", {
    "probes", "healthy", "requested_ns", "timeout", "class",
})
require("watchdog", watchdog, {
    "probes": "2", "healthy": "1", "requested_ns": "200000000",
    "timeout": "1", "class": "health-timeout",
})

quarantine_line, quarantine = fields("SERVICE_SUPERVISOR_QUARANTINE_OK", {
    "attempt", "budget", "reason", "degraded", "frame", "generation",
})
require("quarantine", quarantine, {
    "attempt": "2", "budget": "1", "reason": "restart-budget-exhausted",
    "degraded": "1", "frame": "9", "generation": "6",
})

runtime_line, runtime = fields("SERVICE_SUPERVISOR_OK", {
    "abi", "protocol", "wire", "faults", "probes", "watchdog",
    "restart", "budget", "quarantine", "degraded", "broker", "releases",
    "unbound_drops", "surface_reacquires", "surface_fallback", "processes",
    "handles", "endpoints", "pairs", "waits", "errors",
})
require("runtime", runtime, {
    "abi": "23", "protocol": "1", "wire": "BSH1",
    "faults": "process-exit/health-timeout", "probes": "2/1",
    "watchdog": "fixed-200ms", "restart": "1", "budget": "1/1",
    "quarantine": "runtime", "degraded": "1", "broker": "unbound",
    "releases": "2", "unbound_drops": "0", "surface_reacquires": "0",
    "surface_fallback": "0", "processes": "11/3/3/8", "handles": "31",
    "endpoints": "26", "pairs": "13", "waits": "8/2/6", "errors": "0",
})

def unsigned(marker, key, value):
    if not re.fullmatch(r"(?:0[xX][0-9a-fA-F]+|[0-9]+)", value):
        raise SystemExit(f"{marker}: {key} is not an unsigned integer: {value!r}")
    return int(value, 0)

old_pid = unsigned("armed", "input_pid", armed["input_pid"])
old_generation = unsigned("armed", "generation", armed["generation"])
new_pid = unsigned("health", "input_pid", health["input_pid"])
new_generation = unsigned("health", "generation", health["generation"])
if any(value <= 0 for value in (old_pid, old_generation, new_pid, new_generation)):
    raise SystemExit("ServiceSupervisor identities must be non-zero")
if old_pid == new_pid:
    raise SystemExit("ServiceSupervisor replacement PID did not advance")
if old_pid >> 32 != old_generation or new_pid >> 32 != new_generation:
    raise SystemExit("ServiceSupervisor marker generation does not match its PID")
if (old_pid & 0xFFFFFFFF) != (new_pid & 0xFFFFFFFF):
    raise SystemExit("InputServer restart did not reuse one process slot")
if new_generation != old_generation + 1:
    raise SystemExit("InputServer generation did not advance exactly once")

boot = "BOOT_OK: M48 generic ServiceSupervisor watchdog, runtime quarantine, and degraded UI verified"
if lines.count(boot) != 1:
    raise SystemExit("M48 boot marker is absent or duplicated")
ordered = [
    armed_line, fault_line, resync_line, health_line, watchdog_line,
    quarantine_line, runtime_line, boot,
]
positions = [lines.index(line) for line in ordered]
if positions != sorted(positions) or len(set(positions)) != len(positions):
    raise SystemExit(f"M48 proof markers are out of order: {positions!r}")

allowed_prefixes = set(PREFIXES)
for line in lines:
    if line.startswith("SERVICE_SUPERVISOR_"):
        prefix = line.split(maxsplit=1)[0]
        if prefix not in allowed_prefixes:
            raise SystemExit(f"unexpected ServiceSupervisor leaf marker: {line!r}")

for marker in (
    "INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1",
    "POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1",
):
    if lines.count(marker) != 1:
        raise SystemExit(f"device marker {marker!r} is absent or duplicated")

for line in lines:
    if (
        "input_method=in_surface" in line
        or "route=surface-direct" in line
        or re.search(
            r"\b(?:surface_fallback|legacy_key_reads|legacy_path)=[1-9][0-9]*\b",
            line,
        )
    ):
        raise SystemExit("retired SurfaceServer input ownership leaked into M48")
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

# Capture each state before advancing the authenticated health transcript. The
# recovered screenshot is taken under an explicit QMP stop/dump/continue gate;
# userspace also observes both control paths during its bounded 2 s cadence.
wait_for_prefix_once "$ARMED_PREFIX" "the M48 ServiceSupervisor armed marker"
qmp_action screendump "$PRE_PPM"
qmp_action move

wait_for_prefix_once "$FIRST_FAULT_PREFIX" "the M48 process-exit fault marker"
qmp_action screendump "$GAP_PPM"

wait_for_prefix_once "$RESYNC_READY_PREFIX" "the M48 resync-ready marker"
qmp_action touch-down
qmp_action touch-up

wait_for_prefix_once "$HEALTH_PREFIX" "the M48 healthy-probe marker"
qmp_action screendump "$RECOVERED_PPM"

wait_for_prefix_once "$WATCHDOG_PREFIX" "the M48 watchdog-timeout marker"
wait_for_prefix_once "$QUARANTINE_PREFIX" "the M48 runtime-quarantine marker"
wait_for_prefix_once "$SUCCESS_PREFIX" "the final M48 ServiceSupervisor marker"
wait_for_exact_once "$BOOT_MARKER" "the final M48 boot marker"
qmp_action screendump "$DEGRADED_PPM"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
for prefix in \
  "$ARMED_PREFIX" "$FIRST_FAULT_PREFIX" "$RESYNC_READY_PREFIX" \
  "$HEALTH_PREFIX" "$WATCHDOG_PREFIX" "$QUARANTINE_PREFIX" "$SUCCESS_PREFIX"; do
  if [[ "$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M48 proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M48 did not publish each boot and input-device marker exactly once." >&2
  exit 1
fi

read -r PRE_SHA256 GAP_SHA256 RECOVERED_SHA256 DEGRADED_SHA256 < <(validate_screenshots)
validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M48 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M48 did not enter and normalize execution at EL1." >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
cp "$PRE_PPM" "$OUTPUT_DIR/service-supervisor-pre.ppm"
cp "$GAP_PPM" "$OUTPUT_DIR/service-supervisor-gap.ppm"
cp "$RECOVERED_PPM" "$OUTPUT_DIR/service-supervisor-recovered.ppm"
cp "$DEGRADED_PPM" "$OUTPUT_DIR/service-supervisor-degraded.ppm"

echo "SERVICE_SUPERVISOR_QMP_OK armed=1 first_fault=process-exit resync=1 health=1 watchdog=health-timeout quarantine=runtime degraded=1 broker=unbound pointer=136/184/move-down-up screenshots=pre-gap-recovered-degraded pre_sha256=$PRE_SHA256 gap_sha256=$GAP_SHA256 recovered_sha256=$RECOVERED_SHA256 degraded_sha256=$DEGRADED_SHA256 markers=armed/fault/resync/health/watchdog/quarantine/runtime/boot"
