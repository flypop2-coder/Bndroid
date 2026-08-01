#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp mkdir rm sleep cat; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M49 service-dependency runtime." >&2
    exit 1
  fi
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-20}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
DISCOVER_HASHES="${BNDROID_SERVICE_DEPENDENCY_DISCOVER_HASHES:-0}"
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
if [[ "$DISCOVER_HASHES" != "0" && "$DISCOVER_HASHES" != "1" ]]; then
  echo "BNDROID_SERVICE_DEPENDENCY_DISCOVER_HASHES must be 0 or 1." >&2
  exit 2
fi

# Freeze all five values after the first controlled discovery run. Default
# verification is deliberately unavailable while any placeholder remains.
EXPECTED_PRE_SHA256="9ce8c28d089417834583104bc03a8dbbf626d2a5b3ae68042e37136bcb9b1992"
EXPECTED_GAP_SHA256="767f08eca43b7cef18684805236fb8dfdd0a6f01f8b22fb777c067eb3e516c14"
EXPECTED_SURFACE_RECOVERED_SHA256="5e32917de2e20e532ea21bce31b2c42aefe62de0c682c3f295a55cf4bb714f65"
EXPECTED_DEGRADED_SHA256="32bd74c87d2cdc6ec0a2712890d3a040ec1d3609139639006cebfa7540e32fff"
EXPECTED_RECOVERED_SHA256="a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d"

if [[ "$DISCOVER_HASHES" == "0" ]]; then
  for expected in \
    "$EXPECTED_PRE_SHA256" \
    "$EXPECTED_GAP_SHA256" \
    "$EXPECTED_SURFACE_RECOVERED_SHA256" \
    "$EXPECTED_DEGRADED_SHA256" \
    "$EXPECTED_RECOVERED_SHA256"; do
    if [[ ! "$expected" =~ ^[0-9a-f]{64}$ ]]; then
      echo "M49 screenshot SHA-256 constants are not frozen; run once with BNDROID_SERVICE_DEPENDENCY_DISCOVER_HASHES=1 and replace all five placeholders." >&2
      exit 2
    fi
  done
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="${BNDROID_SERVICE_DEPENDENCY_TARGET_DIR:-$WORKSPACE_ROOT/target/service-dependency-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=service-dependency-runtime \
  BNDROID_KERNEL_FEATURES=service-dependency-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M49 service-dependency kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-service-dependency-m49.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
PRE_PPM="$TMP_DIR/service-dependency-pre.ppm"
GAP_PPM="$TMP_DIR/service-dependency-gap.ppm"
SURFACE_RECOVERED_PPM="$TMP_DIR/service-dependency-surface-recovered.ppm"
DEGRADED_PPM="$TMP_DIR/service-dependency-degraded.ppm"
RECOVERED_PPM="$TMP_DIR/service-dependency-recovered.ppm"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m49"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

ARMED_PREFIX="SERVICE_DEPENDENCY_ARMED"
SURFACE_GAP_PREFIX="SERVICE_DEPENDENCY_SURFACE_GAP"
SURFACE_RECOVERED_PREFIX="SERVICE_DEPENDENCY_SURFACE_RECOVERED"
SURFACE_HEALTHY_PREFIX="SERVICE_DEPENDENCY_SURFACE_HEALTHY"
WATCHDOG_PREFIX="SERVICE_DEPENDENCY_WATCHDOG"
DEGRADED_PREFIX="SERVICE_DEPENDENCY_DEGRADED"
INPUT_REBOUND_PREFIX="SERVICE_DEPENDENCY_INPUT_REBOUND"
INPUT_HEALTHY_PREFIX="SERVICE_DEPENDENCY_INPUT_HEALTHY"
STABLE_PREFIX="SERVICE_DEPENDENCY_STABLE"
SUCCESS_PREFIX="SERVICE_DEPENDENCY_OK"
BOOT_MARKER="BOOT_OK: M49 dependency-aware SurfaceServer and InputServer supervision verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"

STALE_INPUT_RUNTIME_REGEX='^(INPUT_SERVER_KERNEL_OK|INPUT_SERVER_OK|INPUT_SERVER_SURFACE_RESTART_ARMED|INPUT_SERVER_ROUTE_GAP_READY|INPUT_SERVER_ROUTE_EPOCH_OK|INPUT_SERVER_ROUTE_GAP_OK|INPUT_SERVER_CAPTURE_CANCEL_OK|INPUT_SERVER_SURFACE_REBIND_OK|INPUT_SERVER_SURFACE_RESTART_OK|INPUT_SERVER_RESTART_PERMISSION_OK|INPUT_SERVER_RESTART_ARMED|INPUT_SERVER_RESTART_GAP_READY|INPUT_SERVER_RESTART_BACKOFF_OK|INPUT_SERVER_REACQUIRE_OK|INPUT_SERVER_RESYNC_OK|INPUT_SERVER_RESTART_ROUTE_OK|INPUT_SERVER_RESTART_OK)( |$)'
STALE_SUPERVISOR_REGEX='^(SERVICE_SUPERVISOR_ARMED|SERVICE_SUPERVISOR_FIRST_FAULT_OK|SERVICE_SUPERVISOR_RESYNC_READY|SERVICE_SUPERVISOR_HEALTH_OK|SERVICE_SUPERVISOR_WATCHDOG_OK|SERVICE_SUPERVISOR_QUARANTINE_OK|SERVICE_SUPERVISOR_OK)( |$)'
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK|TEXT_INPUT_OK|SOFT_KEYBOARD_OK)( |$)'

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M49 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU appends directly to the serial file. Do not classify an incomplete
  # evidence line until its terminating newline has arrived.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return 1
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|service[- ]dependency runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAIL|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M49 emitted a panic, failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0([[:space:]]|$)|route=surface-direct|surface_fallback=[1-9][0-9]*|legacy_(key_reads|path)=[1-9][0-9]*|^(SURFACE_INPUT|SURFACE_KEY)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M49 fell back to a retired SurfaceServer-owned or legacy input path." >&2
    exit 1
  fi
  if grep -Eq "$STALE_INPUT_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M49 leaked M45, M46, or M47 InputServer leaf evidence." >&2
    exit 1
  fi
  if grep -Eq "$STALE_SUPERVISOR_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M49 leaked M48 ServiceSupervisor leaf evidence." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M49 published an unrelated leaf runtime's success evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M49 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
  return 0
}

wait_for_prefix_once() {
  local prefix="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    if ! reject_failure; then
      if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        show_failure
        echo "QEMU exited while publishing an incomplete line before $description." >&2
        exit 1
      fi
      sleep 0.01
      continue
    fi
    local count
    count="$(grep -Ec "^${prefix} [^[:space:]]+( [^[:space:]]+)*\$" "$NORMALIZED_LOG" || true)"
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
    if ! reject_failure; then
      if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        show_failure
        echo "QEMU exited while publishing an incomplete line before $description." >&2
        exit 1
      fi
      sleep 0.01
      continue
    fi
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
  python3 - "$QMP_SOCKET" "$action" "$argument" "$BOOT_TIMEOUT_SECONDS" <<'PY'
import json
import socket
import sys
import time

socket_path, action, argument, timeout_text = sys.argv[1:]
timeout_seconds = float(timeout_text)
if not 0 < timeout_seconds <= 600:
    raise SystemExit(f"invalid QMP timeout: {timeout_text!r}")
raw_x, raw_y = 13970, 12587
expected_x, expected_y = 136, 184
for raw, extent, expected in ((raw_x, 320, expected_x), (raw_y, 480, expected_y)):
    mapped = raw * (extent - 1) // 32767
    previous = (raw - 1) * (extent - 1) // 32767
    if mapped != expected or previous == expected:
        raise SystemExit(f"raw ABS inverse changed for pixel {expected}: {raw}")

connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
connection.settimeout(timeout_seconds)
deadline = time.monotonic() + timeout_seconds
while time.monotonic() < deadline:
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available")

stream = connection.makefile("rwb", buffering=0)
try:
    greeting = stream.readline()
except TimeoutError:
    raise SystemExit("timed out waiting for QMP greeting") from None
if "QMP" not in json.loads(greeting):
    raise SystemExit("invalid QMP greeting")

def command(payload):
    stream.write(json.dumps(payload).encode("ascii") + b"\n")
    while True:
        try:
            line = stream.readline()
        except TimeoutError:
            raise SystemExit("timed out waiting for QMP response") from None
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
if action == "pointer-down":
    absolute()
    touch(True)
elif action == "pointer-up":
    # The release uses the exact retained (136,184) coordinate. No second ABS
    # report may advance the kernel physical-sequence floor.
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
  python3 - \
    "$PRE_PPM" \
    "$GAP_PPM" \
    "$SURFACE_RECOVERED_PPM" \
    "$DEGRADED_PPM" \
    "$RECOVERED_PPM" \
    "$DISCOVER_HASHES" \
    "$EXPECTED_PRE_SHA256" \
    "$EXPECTED_GAP_SHA256" \
    "$EXPECTED_SURFACE_RECOVERED_SHA256" \
    "$EXPECTED_DEGRADED_SHA256" \
    "$EXPECTED_RECOVERED_SHA256" <<'PY'
import hashlib
import re
import sys

WIDTH, HEIGHT = 320, 480
PHONE_SURFACE = (56, 64, 208, 368)
BADGE = (112, 152, 32, 24)
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
GREEN = bytes.fromhex("22c55e")
PHASE_COLORS = {ORANGE, TEAL, RED, GREEN}

paths = sys.argv[1:6]
discover = sys.argv[6] == "1"
expected_hashes = sys.argv[7:12]
phase_names = ("pre", "gap", "surface-recovered", "degraded", "recovered")
phase_color = {
    "pre": None,
    "gap": ORANGE,
    "surface-recovered": TEAL,
    "degraded": RED,
    "recovered": GREEN,
}

def pixel(pixels, x, y):
    offset = (y * WIDTH + x) * 3
    return pixels[offset:offset + 3]

def inside(rect, x, y):
    rx, ry, width, height = rect
    return rx <= x < rx + width and ry <= y < ry + height

bx, by, bw, bh = BADGE
badge_pixels = {
    (x, y)
    for y in range(by, by + bh)
    for x in range(bx, bx + bw)
}
if len(badge_pixels) != 32 * 24:
    raise SystemExit("M49 badge geometry is not exactly 32x24")
if not inside(PHONE_SURFACE, bx, by) or not inside(
    PHONE_SURFACE, bx + bw - 1, by + bh - 1
):
    raise SystemExit("fixed M49 badge escaped the phone/App surface")

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
        raise SystemExit(
            f"{path}: missing core UI colors {[color.hex() for color in missing]}"
        )
    expected = phase_color[phase]
    if expected is None:
        leaked = PHASE_COLORS & colors
        if leaked:
            raise SystemExit(
                f"{path}: pre phase leaked recovery colors "
                f"{[color.hex() for color in leaked]}"
            )
    else:
        leaked = (PHASE_COLORS - {expected}) & colors
        if expected not in colors or leaked:
            raise SystemExit(
                f"{path}: {phase} phase is not exclusive {expected.hex()} badge evidence"
            )
        actual_positions = {
            (x, y)
            for y in range(HEIGHT)
            for x in range(WIDTH)
            if pixel(pixels, x, y) == expected
        }
        if actual_positions != badge_pixels:
            raise SystemExit(
                f"{path}: {phase} {expected.hex()} area is not the exact 32x24 badge; "
                f"observed {len(actual_positions)} pixels"
            )
    minimum_colors = len(CORE_COLORS) + (0 if expected is None else 1)
    if len(colors) < minimum_colors:
        raise SystemExit(f"{path}: only {len(colors)} colors")
    return pixels

pixels_by_phase = {
    phase: load(path, phase)
    for phase, path in zip(phase_names, paths, strict=True)
}
degraded = pixels_by_phase["degraded"]
recovered = pixels_by_phase["recovered"]
for x, y in badge_pixels:
    if pixel(degraded, x, y) != RED:
        raise SystemExit("M49 degraded badge is not exact 0x00dc2626 red")
    if pixel(recovered, x, y) != GREEN:
        raise SystemExit("M49 recovered frame did not fully overwrite the degraded badge")

def changes(before, after):
    return {
        (x, y)
        for y in range(HEIGHT)
        for x in range(WIDTH)
        if pixel(before, x, y) != pixel(after, x, y)
    }

for previous, current in zip(phase_names, phase_names[1:]):
    changed = changes(pixels_by_phase[previous], pixels_by_phase[current])
    if not (len(badge_pixels) <= len(changed) <= 8192):
        raise SystemExit(
            f"M49 {previous}/{current} diff ledger is out of bounds: {len(changed)}"
        )
    if not badge_pixels.issubset(changed):
        raise SystemExit(f"M49 {previous}/{current} did not replace the full badge")
    outside = [(x, y) for x, y in changed if not inside(PHONE_SURFACE, x, y)]
    if outside:
        raise SystemExit(
            f"M49 {previous}/{current} changed outside Surface: {outside[0]}"
        )

hashes = []
for path in paths:
    with open(path, "rb") as source:
        hashes.append(hashlib.sha256(source.read()).hexdigest())
if len(set(hashes)) != 5:
    raise SystemExit(f"M49 screenshots are not five distinct phases: {hashes!r}")
if not discover:
    for phase, observed, expected in zip(
        phase_names, hashes, expected_hashes, strict=True
    ):
        if not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise SystemExit(
                f"M49 {phase} expected SHA-256 is not frozen: {expected!r}"
            )
        if observed != expected:
            raise SystemExit(
                f"M49 {phase} screenshot SHA-256 changed: {observed}, expected {expected}"
            )
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
    "SERVICE_DEPENDENCY_ARMED",
    "SERVICE_DEPENDENCY_SURFACE_GAP",
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED",
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY",
    "SERVICE_DEPENDENCY_WATCHDOG",
    "SERVICE_DEPENDENCY_DEGRADED",
    "SERVICE_DEPENDENCY_INPUT_REBOUND",
    "SERVICE_DEPENDENCY_INPUT_HEALTHY",
    "SERVICE_DEPENDENCY_STABLE",
    "SERVICE_DEPENDENCY_OK",
)

def fields(prefix, expected_keys):
    def first_token(line):
        parts = line.split(maxsplit=1)
        return parts[0] if parts else ""

    found = [line for line in lines if first_token(line) == prefix]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} marker, observed {found!r}")
    if not re.fullmatch(r"[^\t ]+(?: [^\t ]+)+", found[0]):
        raise SystemExit(f"{prefix}: marker does not use strict single-space fields")
    result = {}
    ordered_keys = []
    for token in found[0].split(" ")[1:]:
        if "=" not in token:
            raise SystemExit(f"{prefix}: non-field token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in result:
            raise SystemExit(f"{prefix}: invalid or duplicate field {token!r}")
        result[key] = value
        ordered_keys.append(key)
    if ordered_keys != list(expected_keys):
        raise SystemExit(
            f"{prefix}: ordered schema {ordered_keys!r}, expected {list(expected_keys)!r}"
        )
    return found[0], result

def require(marker, observed, expected):
    for key, value in expected.items():
        if observed[key] != value:
            raise SystemExit(
                f"{marker}: {key}={observed[key]!r}, expected {value!r}"
            )

armed_line, armed = fields("SERVICE_DEPENDENCY_ARMED", (
    "services", "dependency", "surface_pid", "input_pid",
    "surface_generation", "input_generation", "session", "epoch",
    "floor", "budget",
))
require("armed", armed, {
    "services": "2", "dependency": "input-soft-surface",
    "surface_generation": "1", "input_generation": "1", "session": "1",
    "epoch": "1", "floor": "0", "budget": "1",
})

surface_gap_line, surface_gap = fields("SERVICE_DEPENDENCY_SURFACE_GAP", (
    "fault", "surface_generation", "attempt", "input_alive", "route_epoch", "floor",
))
require("surface-gap", surface_gap, {
    "fault": "process-exit", "surface_generation": "1", "attempt": "1",
    "input_alive": "1", "route_epoch": "2", "floor": "2",
})

surface_recovered_line, surface_recovered = fields(
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED", (
        "surface_generation", "surface_session", "input_generation",
        "input_session", "route_epoch", "floor", "frame",
    )
)
require("surface-recovered", surface_recovered, {
    "surface_generation": "2", "surface_session": "2",
    "input_generation": "1", "input_session": "1", "route_epoch": "2",
    "floor": "3", "frame": "1",
})

surface_healthy_line, surface_healthy = fields(
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY", (
        "probes", "healthy", "surface_generation", "timeout_ns",
    )
)
require("surface-healthy", surface_healthy, {
    "probes": "1", "healthy": "1", "surface_generation": "2",
    "timeout_ns": "100000000",
})

watchdog_line, watchdog = fields("SERVICE_DEPENDENCY_WATCHDOG", (
    "service", "generation", "probes", "reads", "healthy", "timeout_ns", "impact",
))
require("watchdog", watchdog, {
    "service": "input", "generation": "1", "probes": "1", "reads": "1",
    "healthy": "0", "timeout_ns": "100000000",
    "impact": "input-hard/surface-soft",
})

degraded_line, degraded = fields("SERVICE_DEPENDENCY_DEGRADED", (
    "surface_alive", "input_alive", "phase", "frame", "write_generation", "floor",
))
require("degraded", degraded, {
    "surface_alive": "1", "input_alive": "0", "phase": "route-lost",
    "frame": "2", "write_generation": "2", "floor": "3",
})

input_rebound_line, input_rebound = fields("SERVICE_DEPENDENCY_INPUT_REBOUND", (
    "input_generation", "input_session", "route_epoch", "floor", "bic", "bie",
))
require("input-rebound", input_rebound, {
    "input_generation": "2", "input_session": "2", "route_epoch": "3",
    "floor": "3", "bic": "6", "bie": "7",
})

input_healthy_line, input_healthy = fields("SERVICE_DEPENDENCY_INPUT_HEALTHY", (
    "probes", "reads", "healthy", "input_generation",
))
require("input-healthy", input_healthy, {
    "probes": "1", "reads": "1", "healthy": "1", "input_generation": "2",
})

stable_line, stable = fields("SERVICE_DEPENDENCY_STABLE", (
    "cadence_ns", "restarts", "impacts", "created", "exited", "reaped", "live",
))
require("stable", stable, {
    "cadence_ns": "2000000000", "restarts": "1/1",
    "impacts": "unaffected/unaffected", "created": "12", "exited": "3",
    "reaped": "3", "live": "9",
})

runtime_line, runtime = fields("SERVICE_DEPENDENCY_OK", (
    "abi", "protocol", "services", "dependency", "health_messages", "probes",
    "probe_reads", "healthy", "watchdog", "bir", "bip", "bic", "bie",
    "surface", "input", "sessions", "epochs", "floor", "frames", "outputs",
    "restarts", "impacts", "created", "exited", "reaped", "live", "reasons",
    "handles", "endpoints", "pairs", "waits", "topology", "final_state",
))
require("runtime", runtime, {
    "abi": "23", "protocol": "1", "services": "2",
    "dependency": "input-soft-surface", "health_messages": "5", "probes": "3",
    "probe_reads": "3", "healthy": "2", "watchdog": "1/1", "bir": "8+3",
    "bip": "4", "bic": "6", "bie": "7", "surface": "1/2",
    "input": "1/2", "sessions": "1/2", "epochs": "1/2/3",
    "floor": "0/2/3", "frames": "1/2/3", "outputs": "3",
    "restarts": "1/1", "impacts": "unaffected/unaffected", "created": "12",
    "exited": "3", "reaped": "3", "live": "9",
    "reasons": "exited1/killed2", "handles": "36", "endpoints": "30",
    "pairs": "15", "waits": "9/2/7", "topology": "resident",
    "final_state": "ready",
})

def generation_qualified_pid(marker, key, generation):
    raw = marker[key]
    if not re.fullmatch(r"0x[0-9a-f]{16}", raw):
        raise SystemExit(f"{key} is not canonical hexadecimal: {raw!r}")
    value = int(raw, 0)
    if value <= 0 or value > (1 << 64) - 1 or value & 0xFFFFFFFF == 0:
        raise SystemExit(f"{key} is not a non-zero generation-qualified PID: {raw!r}")
    if value >> 32 != generation:
        raise SystemExit(
            f"{key} generation {value >> 32} does not match marker generation {generation}"
        )
    return value

surface_pid = generation_qualified_pid(armed, "surface_pid", 1)
input_pid = generation_qualified_pid(armed, "input_pid", 1)
if surface_pid == input_pid:
    raise SystemExit("SurfaceServer and InputServer published the same PID")

boot = "BOOT_OK: M49 dependency-aware SurfaceServer and InputServer supervision verified"
if lines.count(boot) != 1:
    raise SystemExit("M49 boot marker is absent or duplicated")
ordered = [
    armed_line,
    surface_gap_line,
    surface_recovered_line,
    surface_healthy_line,
    watchdog_line,
    degraded_line,
    input_rebound_line,
    input_healthy_line,
    stable_line,
    runtime_line,
    boot,
]
positions = [lines.index(line) for line in ordered]
if positions != sorted(positions) or len(set(positions)) != len(positions):
    raise SystemExit(f"M49 proof markers are out of order: {positions!r}")

allowed_prefixes = set(PREFIXES)
for line in lines:
    if line.startswith("SERVICE_DEPENDENCY_"):
        prefix = line.split(maxsplit=1)[0]
        if prefix not in allowed_prefixes:
            raise SystemExit(f"unexpected M49 service-dependency marker: {line!r}")

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
        raise SystemExit("retired SurfaceServer input ownership leaked into M49")
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

# M49 deliberately uses the same retained absolute coordinate for one down/up
# pair. The two autonomous watchdog phases require no additional QMP input.
wait_for_prefix_once "$ARMED_PREFIX" "the M49 dependency-armed marker"
qmp_action screendump "$PRE_PPM"
qmp_action pointer-down

wait_for_prefix_once "$SURFACE_GAP_PREFIX" "the M49 SurfaceServer route-gap marker"
qmp_action screendump "$GAP_PPM"
qmp_action pointer-up

wait_for_prefix_once "$SURFACE_RECOVERED_PREFIX" "the M49 SurfaceServer recovered marker"
qmp_action screendump "$SURFACE_RECOVERED_PPM"
wait_for_prefix_once "$SURFACE_HEALTHY_PREFIX" "the M49 SurfaceServer health marker"

wait_for_prefix_once "$WATCHDOG_PREFIX" "the M49 InputServer watchdog marker"
wait_for_prefix_once "$DEGRADED_PREFIX" "the M49 degraded marker"
qmp_action screendump "$DEGRADED_PPM"

wait_for_prefix_once "$INPUT_REBOUND_PREFIX" "the M49 InputServer rebound marker"
wait_for_prefix_once "$INPUT_HEALTHY_PREFIX" "the M49 InputServer healthy marker"
wait_for_prefix_once "$STABLE_PREFIX" "the M49 stable-cadence marker"
wait_for_prefix_once "$SUCCESS_PREFIX" "the final M49 service-dependency marker"
wait_for_exact_once "$BOOT_MARKER" "the final M49 boot marker"
qmp_action screendump "$RECOVERED_PPM"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ! reject_failure; then
  show_failure
  echo "M49 serial log ended with an incomplete evidence line." >&2
  exit 1
fi
for prefix in \
  "$ARMED_PREFIX" \
  "$SURFACE_GAP_PREFIX" \
  "$SURFACE_RECOVERED_PREFIX" \
  "$SURFACE_HEALTHY_PREFIX" \
  "$WATCHDOG_PREFIX" \
  "$DEGRADED_PREFIX" \
  "$INPUT_REBOUND_PREFIX" \
  "$INPUT_HEALTHY_PREFIX" \
  "$STABLE_PREFIX" \
  "$SUCCESS_PREFIX"; do
  if [[ "$(grep -Ec "^${prefix} [^[:space:]]+( [^[:space:]]+)*\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M49 proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M49 did not publish each boot and input-device marker exactly once." >&2
  exit 1
fi

validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M49 did not preserve the exact unique M25 storage evidence." >&2
  exit 1
fi
if [[ "$(grep -Fc 'entered AArch64 EL1' "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fc 'running EL1' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M49 did not publish unique entered/running EL1 evidence." >&2
  exit 1
fi

read -r \
  PRE_SHA256 \
  GAP_SHA256 \
  SURFACE_RECOVERED_SHA256 \
  DEGRADED_SHA256 \
  RECOVERED_SHA256 < <(validate_screenshots)

mkdir -p "$OUTPUT_DIR"
cp "$PRE_PPM" "$OUTPUT_DIR/service-dependency-pre.ppm"
cp "$GAP_PPM" "$OUTPUT_DIR/service-dependency-gap.ppm"
cp "$SURFACE_RECOVERED_PPM" "$OUTPUT_DIR/service-dependency-surface-recovered.ppm"
cp "$DEGRADED_PPM" "$OUTPUT_DIR/service-dependency-degraded.ppm"
cp "$RECOVERED_PPM" "$OUTPUT_DIR/service-dependency-recovered.ppm"

if [[ "$DISCOVER_HASHES" == "1" ]]; then
  echo "SERVICE_DEPENDENCY_DISCOVER_HASHES pre_sha256=$PRE_SHA256 gap_sha256=$GAP_SHA256 surface_recovered_sha256=$SURFACE_RECOVERED_SHA256 degraded_sha256=$DEGRADED_SHA256 recovered_sha256=$RECOVERED_SHA256"
else
  echo "SERVICE_DEPENDENCY_QMP_OK services=2 dependency=input-soft-surface surface_restart=1 input_watchdog=1 degraded=1 recovered=1 pointer=136/184/down-gap-up screenshots=pre-gap-surface-recovered-degraded-recovered pre_sha256=$PRE_SHA256 gap_sha256=$GAP_SHA256 surface_recovered_sha256=$SURFACE_RECOVERED_SHA256 degraded_sha256=$DEGRADED_SHA256 recovered_sha256=$RECOVERED_SHA256 markers=armed/surface-gap/surface-recovered/surface-healthy/watchdog/degraded/input-rebound/input-healthy/stable/runtime/boot"
fi
