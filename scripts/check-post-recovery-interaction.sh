#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp mkdir rm sleep cat; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M50 post-recovery interaction runtime." >&2
    exit 1
  fi
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-20}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
DISCOVER_HASHES="${BNDROID_POST_RECOVERY_DISCOVER_HASHES:-0}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
fi
if ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must not exceed 600." >&2
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
  echo "BNDROID_POST_RECOVERY_DISCOVER_HASHES must be 0 or 1." >&2
  exit 2
fi

# Frozen from the controlled offline, single-core, `-nic none` M50 discovery
# run. Default verification remains fail-closed if any value is removed.
EXPECTED_ARMED_SHA256="a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d"
EXPECTED_REJECTED_SHA256="a251ce92f2d9e4e1fd0e76f3c267a16837520fef3f99268cae72bb4681b61d0d"
EXPECTED_PRESENTED_SHA256="798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974"
ZERO_SHA256="0000000000000000000000000000000000000000000000000000000000000000"

if [[ "$DISCOVER_HASHES" == "0" ]]; then
  for expected in \
    "$EXPECTED_ARMED_SHA256" \
    "$EXPECTED_REJECTED_SHA256" \
    "$EXPECTED_PRESENTED_SHA256"; do
    if [[ ! "$expected" =~ ^[0-9a-f]{64}$ || "$expected" == "$ZERO_SHA256" ]]; then
      echo "M50 screenshot SHA-256 constants are not frozen; run once with BNDROID_POST_RECOVERY_DISCOVER_HASHES=1 and replace all three placeholders." >&2
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

TARGET_ROOT="${BNDROID_POST_RECOVERY_TARGET_DIR:-$WORKSPACE_ROOT/target/post-recovery-interaction-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=post-recovery-interaction-runtime \
  BNDROID_KERNEL_FEATURES=post-recovery-interaction-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M50 post-recovery interaction kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-post-recovery-m50.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
ARMED_PPM="$TMP_DIR/post-recovery-armed.ppm"
REJECTED_PPM="$TMP_DIR/post-recovery-rejected.ppm"
PRESENTED_PPM="$TMP_DIR/post-recovery-presented.ppm"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m50"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

M49_ARMED="SERVICE_DEPENDENCY_ARMED"
M49_SURFACE_GAP="SERVICE_DEPENDENCY_SURFACE_GAP"
M49_SURFACE_RECOVERED="SERVICE_DEPENDENCY_SURFACE_RECOVERED"
M49_SURFACE_HEALTHY="SERVICE_DEPENDENCY_SURFACE_HEALTHY"
M49_WATCHDOG="SERVICE_DEPENDENCY_WATCHDOG"
M49_DEGRADED="SERVICE_DEPENDENCY_DEGRADED"
M49_INPUT_REBOUND="SERVICE_DEPENDENCY_INPUT_REBOUND"
M49_INPUT_HEALTHY="SERVICE_DEPENDENCY_INPUT_HEALTHY"
POST_ARMED="POST_RECOVERY_ARMED"
POST_REJECTED="POST_RECOVERY_REJECTED"
POST_ROUTED="POST_RECOVERY_ROUTED"
POST_PRESENTED="POST_RECOVERY_PRESENTED"
POST_HEALTHY="POST_RECOVERY_HEALTHY"
POST_SUCCESS="POST_RECOVERY_INTERACTION_OK"
BOOT_MARKER="BOOT_OK: M50 post-recovery resident input-to-frame interaction verified"
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
    echo "M50 diagnostic artifacts retained at $TMP_DIR" >&2
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
  # QEMU appends serial data directly; defer classification until the current
  # evidence line has its terminating newline.
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return 1
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|post-recovery interaction runtime (failed|timed out)|service[- ]dependency runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAIL|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M50 emitted a panic, failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0([[:space:]]|$)|route=surface-direct|surface_fallback=[1-9][0-9]*|legacy_(key_reads|path)=[1-9][0-9]*|^(SURFACE_INPUT|SURFACE_KEY)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M50 fell back to a retired SurfaceServer-owned or legacy input path." >&2
    exit 1
  fi
  if grep -Eq "$STALE_INPUT_RUNTIME_REGEX|$STALE_SUPERVISOR_REGEX|$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M50 leaked stale or unrelated leaf-runtime evidence." >&2
    exit 1
  fi
  if grep -Eq '^(SERVICE_DEPENDENCY_STABLE|SERVICE_DEPENDENCY_OK)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M50 incorrectly published an M49 terminal success marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M50 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
  local unknown_post
  unknown_post="$(grep '^POST_RECOVERY_' "$NORMALIZED_LOG" | grep -Ev '^(POST_RECOVERY_ARMED|POST_RECOVERY_REJECTED|POST_RECOVERY_ROUTED|POST_RECOVERY_PRESENTED|POST_RECOVERY_HEALTHY|POST_RECOVERY_INTERACTION_OK)( |$)' || true)"
  if [[ -n "$unknown_post" ]]; then
    show_failure
    echo "M50 published an unknown POST_RECOVERY_ marker." >&2
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

coordinates = {
    "invalid-down": (1644, 2190, 16, 32),
    "valid-down": (13970, 12587, 136, 184),
}
for raw_x, raw_y, expected_x, expected_y in coordinates.values():
    for raw, extent, expected in (
        (raw_x, 320, expected_x),
        (raw_y, 480, expected_y),
    ):
        mapped = raw * (extent - 1) // 32767
        previous = (raw - 1) * (extent - 1) // 32767
        if mapped != expected or previous == expected:
            raise SystemExit(
                f"raw ABS inverse changed for pixel {expected}: {raw}"
            )

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

def touch_down(name):
    raw_x, raw_y, _, _ = coordinates[name]
    # One QMP input transaction is one physical down sample: both ABS reports
    # and BTN_TOUCH down must remain in this exact combined command.
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "abs", "data": {"axis": "x", "value": raw_x}},
        {"type": "abs", "data": {"axis": "y", "value": raw_y}},
        {"type": "btn", "data": {"down": True, "button": "touch"}},
    ]}})
    time.sleep(0.05)

def absolute(name):
    raw_x, raw_y, _, _ = coordinates[name]
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

def touch_up():
    # Retain the most recent absolute position. A release-only transaction
    # prevents an ABS-only report from advancing the physical sequence.
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "btn", "data": {"down": False, "button": "touch"}},
    ]}})
    time.sleep(0.05)

command({"execute": "qmp_capabilities"})
if action == "m49-pointer-down":
    # Preserve M49's original two-transaction trigger exactly: the ABS sample
    # advances the physical floor before BTN_TOUCH starts the captured contact.
    absolute("valid-down")
    touch(True)
elif action == "m49-pointer-up":
    touch(False)
elif action in coordinates:
    touch_down(action)
elif action == "touch-up":
    touch_up()
elif action == "screendump":
    if not argument:
        raise SystemExit("screendump requires an output path")
    command({"execute": "stop"})
    command({"execute": "screendump", "arguments": {
        "filename": argument,
        "format": "ppm",
    }})
    command({"execute": "cont"})
elif action == "quit":
    command({"execute": "quit"})
else:
    raise SystemExit(f"unsupported QMP action: {action}")
connection.close()
PY
}

validate_screenshots() {
  python3 - \
    "$ARMED_PPM" \
    "$REJECTED_PPM" \
    "$PRESENTED_PPM" \
    "$DISCOVER_HASHES" \
    "$EXPECTED_ARMED_SHA256" \
    "$EXPECTED_REJECTED_SHA256" \
    "$EXPECTED_PRESENTED_SHA256" <<'PY'
import hashlib
import re
import sys

paths = sys.argv[1:4]
discover = sys.argv[4] == "1"
expected_hashes = sys.argv[5:8]
payloads = []
hashes = []
for path in paths:
    with open(path, "rb") as source:
        if source.readline() != b"P6\n":
            raise SystemExit(f"{path}: not a binary PPM")
        dimensions = source.readline()
        while dimensions.startswith(b"#"):
            dimensions = source.readline()
        if dimensions.strip() != b"320 480" or source.readline().strip() != b"255":
            raise SystemExit(f"{path}: expected exact 320x480 8-bit RGB")
        pixels = source.read()
    if len(pixels) != 320 * 480 * 3:
        raise SystemExit(f"{path}: payload has {len(pixels)} bytes")
    if len({pixels[index:index + 3] for index in range(0, len(pixels), 3)}) < 8:
        raise SystemExit(f"{path}: implausibly flat phone frame")
    payloads.append(pixels)
    with open(path, "rb") as source:
        hashes.append(hashlib.sha256(source.read()).hexdigest())

if payloads[0] != payloads[1] or hashes[0] != hashes[1]:
    raise SystemExit(
        "M50 rejected outside-phone contact changed the displayed frame: "
        f"{hashes[0]} != {hashes[1]}"
    )
if payloads[2] == payloads[0] or hashes[2] == hashes[0]:
    raise SystemExit("M50 accepted App contact did not produce a new displayed frame")
if not discover:
    for phase, observed, expected in zip(
        ("armed", "rejected", "presented"), hashes, expected_hashes, strict=True
    ):
        if not re.fullmatch(r"[0-9a-f]{64}", expected) or expected == "0" * 64:
            raise SystemExit(f"M50 {phase} expected SHA-256 is not frozen")
        if observed != expected:
            raise SystemExit(
                f"M50 {phase} screenshot SHA-256 changed: {observed}, expected {expected}"
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

M49_PREFIXES = (
    "SERVICE_DEPENDENCY_ARMED",
    "SERVICE_DEPENDENCY_SURFACE_GAP",
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED",
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY",
    "SERVICE_DEPENDENCY_WATCHDOG",
    "SERVICE_DEPENDENCY_DEGRADED",
    "SERVICE_DEPENDENCY_INPUT_REBOUND",
    "SERVICE_DEPENDENCY_INPUT_HEALTHY",
)
POST_PREFIXES = (
    "POST_RECOVERY_ARMED",
    "POST_RECOVERY_REJECTED",
    "POST_RECOVERY_ROUTED",
    "POST_RECOVERY_PRESENTED",
    "POST_RECOVERY_HEALTHY",
    "POST_RECOVERY_INTERACTION_OK",
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
require("M49 armed", armed, {
    "services": "2", "dependency": "input-soft-surface",
    "surface_generation": "1", "input_generation": "1", "session": "1",
    "epoch": "1", "floor": "0", "budget": "1",
})

surface_gap_line, surface_gap = fields("SERVICE_DEPENDENCY_SURFACE_GAP", (
    "fault", "surface_generation", "attempt", "input_alive", "route_epoch", "floor",
))
require("M49 surface gap", surface_gap, {
    "fault": "process-exit", "surface_generation": "1", "attempt": "1",
    "input_alive": "1", "route_epoch": "2", "floor": "2",
})

surface_recovered_line, surface_recovered = fields(
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED", (
        "surface_generation", "surface_session", "input_generation",
        "input_session", "route_epoch", "floor", "frame",
    )
)
require("M49 surface recovered", surface_recovered, {
    "surface_generation": "2", "surface_session": "2",
    "input_generation": "1", "input_session": "1", "route_epoch": "2",
    "floor": "3", "frame": "1",
})

surface_healthy_line, surface_healthy = fields(
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY", (
        "probes", "healthy", "surface_generation", "timeout_ns",
    )
)
require("M49 surface healthy", surface_healthy, {
    "probes": "1", "healthy": "1", "surface_generation": "2",
    "timeout_ns": "100000000",
})

watchdog_line, watchdog = fields("SERVICE_DEPENDENCY_WATCHDOG", (
    "service", "generation", "probes", "reads", "healthy", "timeout_ns", "impact",
))
require("M49 watchdog", watchdog, {
    "service": "input", "generation": "1", "probes": "1", "reads": "1",
    "healthy": "0", "timeout_ns": "100000000",
    "impact": "input-hard/surface-soft",
})

degraded_line, degraded = fields("SERVICE_DEPENDENCY_DEGRADED", (
    "surface_alive", "input_alive", "phase", "frame", "write_generation", "floor",
))
require("M49 degraded", degraded, {
    "surface_alive": "1", "input_alive": "0", "phase": "route-lost",
    "frame": "2", "write_generation": "2", "floor": "3",
})

input_rebound_line, input_rebound = fields("SERVICE_DEPENDENCY_INPUT_REBOUND", (
    "input_generation", "input_session", "route_epoch", "floor", "bic", "bie",
))
require("M49 input rebound", input_rebound, {
    "input_generation": "2", "input_session": "2", "route_epoch": "3",
    "floor": "3", "bic": "6", "bie": "7",
})

input_healthy_line, input_healthy = fields("SERVICE_DEPENDENCY_INPUT_HEALTHY", (
    "probes", "reads", "healthy", "input_generation",
))
require("M49 input healthy", input_healthy, {
    "probes": "1", "reads": "1", "healthy": "1", "input_generation": "2",
})

post_armed_line, post_armed = fields("POST_RECOVERY_ARMED", (
    "abi", "surface_session", "input_session", "route_epoch", "floor", "frame",
))
require("M50 armed", post_armed, {
    "abi": "23", "surface_session": "2", "input_session": "2",
    "route_epoch": "3", "floor": "3", "frame": "3",
})

rejected_line, rejected = fields("POST_RECOVERY_REJECTED", (
    "physical", "target", "routed", "frame",
))
require("M50 rejected", rejected, {
    "physical": "4/5", "target": "none", "routed": "0", "frame": "3",
})

routed_line, routed = fields("POST_RECOVERY_ROUTED", (
    "physical", "target", "events", "capture",
))
require("M50 routed", routed, {
    "physical": "6/7", "target": "app", "events": "2", "capture": "1",
})

presented_line, presented = fields("POST_RECOVERY_PRESENTED", (
    "command", "app_frame", "scene", "output_frame", "write_generation",
))
require("M50 presented", presented, {
    "command": "5", "app_frame": "3", "scene": "10",
    "output_frame": "4", "write_generation": "4",
})

healthy_line, healthy = fields("POST_RECOVERY_HEALTHY", ("surface", "input"))
require("M50 healthy", healthy, {"surface": "2/2", "input": "2/2"})

success_line, success = fields("POST_RECOVERY_INTERACTION_OK", (
    "abi", "surface_session", "input_session", "route_epoch", "rejected",
    "physical", "target", "events", "capture", "app_present", "output_frame",
    "health", "resident", "errors",
))
require("M50 success", success, {
    "abi": "23", "surface_session": "2", "input_session": "2",
    "route_epoch": "3", "rejected": "4..5", "physical": "6..7",
    "target": "app", "events": "2", "capture": "1", "app_present": "1",
    "output_frame": "4", "health": "2/2", "resident": "1", "errors": "0",
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

boot = "BOOT_OK: M50 post-recovery resident input-to-frame interaction verified"
if lines.count(boot) != 1:
    raise SystemExit("M50 boot marker is absent or duplicated")
ordered = [
    armed_line, surface_gap_line, surface_recovered_line, surface_healthy_line,
    watchdog_line, degraded_line, input_rebound_line, input_healthy_line,
    post_armed_line, rejected_line, routed_line, presented_line, healthy_line,
    success_line, boot,
]
positions = [lines.index(line) for line in ordered]
if positions != sorted(positions) or len(set(positions)) != len(positions):
    raise SystemExit(f"M49/M50 proof markers are out of order: {positions!r}")

for line in lines:
    if line.startswith("SERVICE_DEPENDENCY_"):
        prefix = line.split(maxsplit=1)[0]
        if prefix not in M49_PREFIXES:
            raise SystemExit(f"unexpected M49 service-dependency marker: {line!r}")
    if line.startswith("POST_RECOVERY_"):
        prefix = line.split(maxsplit=1)[0]
        if prefix not in POST_PREFIXES:
            raise SystemExit(f"unexpected M50 post-recovery marker: {line!r}")

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
        raise SystemExit("retired SurfaceServer input ownership leaked into M50")
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

# Preserve the exact M49 prefix through the replacement InputServer health
# boundary. M50 intentionally owns the terminal success and boot evidence.
wait_for_prefix_once "$M49_ARMED" "the M49 dependency-armed marker"
qmp_action m49-pointer-down
wait_for_prefix_once "$M49_SURFACE_GAP" "the M49 SurfaceServer route-gap marker"
qmp_action m49-pointer-up
wait_for_prefix_once "$M49_SURFACE_RECOVERED" "the M49 SurfaceServer recovered marker"
wait_for_prefix_once "$M49_SURFACE_HEALTHY" "the M49 SurfaceServer health marker"
wait_for_prefix_once "$M49_WATCHDOG" "the M49 InputServer watchdog marker"
wait_for_prefix_once "$M49_DEGRADED" "the M49 degraded marker"
wait_for_prefix_once "$M49_INPUT_REBOUND" "the M49 InputServer rebound marker"
wait_for_prefix_once "$M49_INPUT_HEALTHY" "the M49 InputServer healthy marker"

wait_for_prefix_once "$POST_ARMED" "the M50 post-recovery armed marker"
qmp_action screendump "$ARMED_PPM"

# Outside-phone down/up must be rejected without a frame commit. The down is
# one ABS-X/ABS-Y/BTN transaction; the release is BTN-only.
qmp_action invalid-down
qmp_action touch-up
wait_for_prefix_once "$POST_REJECTED" "the M50 rejected-contact marker"
qmp_action screendump "$REJECTED_PPM"

# The recovered App contact uses the same sequencing discipline and must route
# and present exactly one new frame.
qmp_action valid-down
qmp_action touch-up
wait_for_prefix_once "$POST_ROUTED" "the M50 routed-contact marker"
wait_for_prefix_once "$POST_PRESENTED" "the M50 presented-frame marker"
qmp_action screendump "$PRESENTED_PPM"
wait_for_prefix_once "$POST_HEALTHY" "the M50 second-round health marker"
wait_for_prefix_once "$POST_SUCCESS" "the final M50 interaction marker"
wait_for_exact_once "$BOOT_MARKER" "the final M50 boot marker"

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ! reject_failure; then
  show_failure
  echo "M50 serial log ended with an incomplete evidence line." >&2
  exit 1
fi

for prefix in \
  "$M49_ARMED" \
  "$M49_SURFACE_GAP" \
  "$M49_SURFACE_RECOVERED" \
  "$M49_SURFACE_HEALTHY" \
  "$M49_WATCHDOG" \
  "$M49_DEGRADED" \
  "$M49_INPUT_REBOUND" \
  "$M49_INPUT_HEALTHY" \
  "$POST_ARMED" \
  "$POST_REJECTED" \
  "$POST_ROUTED" \
  "$POST_PRESENTED" \
  "$POST_HEALTHY" \
  "$POST_SUCCESS"; do
  if [[ "$(grep -Ec "^${prefix} [^[:space:]]+( [^[:space:]]+)*\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M50 proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M50 did not publish each boot and input-device marker exactly once." >&2
  exit 1
fi

validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M50 did not preserve the exact unique M25 storage evidence." >&2
  exit 1
fi
if [[ "$(grep -Fc 'entered AArch64 EL1' "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fc 'running EL1' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M50 did not publish unique entered/running EL1 evidence." >&2
  exit 1
fi

read -r ARMED_SHA256 REJECTED_SHA256 PRESENTED_SHA256 < <(validate_screenshots)

mkdir -p "$OUTPUT_DIR"
cp "$ARMED_PPM" "$OUTPUT_DIR/post-recovery-armed.ppm"
cp "$REJECTED_PPM" "$OUTPUT_DIR/post-recovery-rejected.ppm"
cp "$PRESENTED_PPM" "$OUTPUT_DIR/post-recovery-presented.ppm"

# End the local VM through QMP so a successful run also proves control-channel
# liveness. The EXIT trap retains a kill fallback for all failure paths.
qmp_action quit
QEMU_WAIT_PID="$QEMU_PID"
QUIT_TIMEOUT_FLAG="$TMP_DIR/qemu-quit.timeout"
(
  sleep "$BOOT_TIMEOUT_SECONDS"
  if kill -0 "$QEMU_WAIT_PID" 2>/dev/null; then
    : >"$QUIT_TIMEOUT_FLAG"
    kill "$QEMU_WAIT_PID" 2>/dev/null || true
  fi
) &
QUIT_WATCHDOG_PID=$!
set +e
wait "$QEMU_WAIT_PID"
QEMU_STATUS=$?
kill "$QUIT_WATCHDOG_PID" 2>/dev/null || true
wait "$QUIT_WATCHDOG_PID" 2>/dev/null || true
set -e
QEMU_PID=""
if [[ -e "$QUIT_TIMEOUT_FLAG" ]]; then
  show_failure
  echo "QEMU did not exit within ${BOOT_TIMEOUT_SECONDS}s after the M50 QMP quit." >&2
  exit 1
fi
if [[ "$QEMU_STATUS" != "0" ]]; then
  show_failure
  echo "QEMU did not exit cleanly after the M50 QMP quit (status $QEMU_STATUS)." >&2
  exit 1
fi

# Close the interval between the pre-quit validation and process exit. A late
# failure, duplicate proof marker, or truncated serial record must not be
# hidden by an otherwise successful QMP quit.
tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ! reject_failure; then
  show_failure
  echo "M50 serial log ended with an incomplete evidence line after QMP quit." >&2
  exit 1
fi
validate_final_evidence
for prefix in \
  "$M49_ARMED" \
  "$M49_SURFACE_GAP" \
  "$M49_SURFACE_RECOVERED" \
  "$M49_SURFACE_HEALTHY" \
  "$M49_WATCHDOG" \
  "$M49_DEGRADED" \
  "$M49_INPUT_REBOUND" \
  "$M49_INPUT_HEALTHY" \
  "$POST_ARMED" \
  "$POST_REJECTED" \
  "$POST_ROUTED" \
  "$POST_PRESENTED" \
  "$POST_HEALTHY" \
  "$POST_SUCCESS"; do
  if [[ "$(grep -Ec "^${prefix} [^[:space:]]+( [^[:space:]]+)*\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M50 post-quit proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M50 post-quit boot or input-device evidence is absent or duplicated." >&2
  exit 1
fi

if [[ "$DISCOVER_HASHES" == "1" ]]; then
  echo "POST_RECOVERY_DISCOVER_HASHES armed_sha256=$ARMED_SHA256 rejected_sha256=$REJECTED_SHA256 presented_sha256=$PRESENTED_SHA256"
else
  echo "POST_RECOVERY_QMP_OK invalid=16/32/down-up rejected=1 valid=136/184/down-up routed=app presented=1 health=2/2 armed_sha256=$ARMED_SHA256 rejected_sha256=$REJECTED_SHA256 presented_sha256=$PRESENTED_SHA256 markers=m49-armed/surface-gap/surface-recovered/surface-healthy/watchdog/degraded/input-rebound/input-healthy/post-armed/rejected/routed/presented/healthy/interaction/boot"
fi
