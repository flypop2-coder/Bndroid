#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp mkdir rm sleep cat; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M53 post-recovery lifecycle focus runtime." >&2
    exit 1
  fi
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-20}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
DISCOVER_HASHES="${BNDROID_POST_RECOVERY_LIFECYCLE_FOCUS_DISCOVER_HASHES:-0}"
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
  echo "BNDROID_POST_RECOVERY_LIFECYCLE_FOCUS_DISCOVER_HASHES must be 0 or 1." >&2
  exit 2
fi

# The complete M50--M52 visual prefix remains immutable during discovery.
# Only the final M53 hash may start as the all-zero controlled placeholder.
EXPECTED_M50_SHA256="798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974"
EXPECTED_M51_SHA256="cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729"
EXPECTED_M52_SHA256="97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb"
EXPECTED_M53_SHA256="97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb"
ZERO_SHA256="0000000000000000000000000000000000000000000000000000000000000000"
for expected in "$EXPECTED_M50_SHA256" "$EXPECTED_M51_SHA256" "$EXPECTED_M52_SHA256" "$EXPECTED_M53_SHA256"; do
  if [[ ! "$expected" =~ ^[0-9a-f]{64}$ ]]; then
    echo "M53 screenshot SHA-256 constants must be canonical lowercase hexadecimal." >&2
    exit 2
  fi
done
if [[ "$EXPECTED_M50_SHA256" == "$ZERO_SHA256" \
  || "$EXPECTED_M51_SHA256" == "$ZERO_SHA256" \
  || "$EXPECTED_M52_SHA256" == "$ZERO_SHA256" ]]; then
  echo "M53 discovery cannot weaken the frozen M50, M51, or M52 screenshot hashes." >&2
  exit 2
fi
if [[ "$DISCOVER_HASHES" == "0" && "$EXPECTED_M53_SHA256" == "$ZERO_SHA256" ]]; then
  echo "M53 final screenshot SHA-256 is not frozen; run once with BNDROID_POST_RECOVERY_LIFECYCLE_FOCUS_DISCOVER_HASHES=1, then replace only EXPECTED_M53_SHA256." >&2
  exit 2
fi
if [[ "$DISCOVER_HASHES" == "0" && "$EXPECTED_M53_SHA256" != "$EXPECTED_M52_SHA256" ]]; then
  echo "M53 final screenshot SHA-256 must equal the frozen M52 framebuffer SHA-256." >&2
  exit 2
fi

STORAGE_IMAGE="${BNDROID_STORAGE_IMAGE:-$WORKSPACE_ROOT/target/bndroid-storage-m25.raw}"
BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
if ! validate_storage_image_geometry "$STORAGE_IMAGE"; then
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
fi
build_storage_qemu_args "$STORAGE_IMAGE" modern

TARGET_ROOT="${BNDROID_POST_RECOVERY_LIFECYCLE_FOCUS_TARGET_DIR:-$WORKSPACE_ROOT/target/post-recovery-lifecycle-focus-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=post-recovery-lifecycle-focus-runtime \
  BNDROID_KERNEL_FEATURES=post-recovery-lifecycle-focus-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M53 post-recovery lifecycle focus kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-post-recovery-m53.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
M50_PPM="$TMP_DIR/m50-presented.ppm"
M51_PPM="$TMP_DIR/m51-presented.ppm"
M52_PPM="$TMP_DIR/m52-presented.ppm"
M53_PPM="$TMP_DIR/m53-lifecycle-focus.ppm"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m53"
QEMU_PID=""
QMP_INPUT_SEND_EVENTS=0
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
FOCUS_ARMED="POST_RECOVERY_FOCUS_ARMED"
LAUNCHER_CAPTURED="POST_RECOVERY_LAUNCHER_CAPTURED"
LAUNCHER_PRESENTED="POST_RECOVERY_LAUNCHER_PRESENTED"
FOCUS_SUCCESS="POST_RECOVERY_FOCUS_OK"
ROUNDTRIP_ARMED="POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED"
APP_CAPTURED="POST_RECOVERY_APP_CAPTURED"
APP_PRESENTED="POST_RECOVERY_APP_PRESENTED"
ROUNDTRIP_SUCCESS="POST_RECOVERY_FOCUS_ROUNDTRIP_OK"
LIFECYCLE_SESSION_READY="POST_RECOVERY_LIFECYCLE_SESSION_READY"
LIFECYCLE_LAUNCHER_SYNCED="POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED"
LIFECYCLE_APP_SYNCED="POST_RECOVERY_LIFECYCLE_APP_SYNCED"
LIFECYCLE_SUCCESS="POST_RECOVERY_LIFECYCLE_FOCUS_OK"
BOOT_MARKER="BOOT_OK: M53 post-recovery lifecycle focus convergence verified"
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
    echo "M53 diagnostic artifacts retained at $TMP_DIR" >&2
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
  if [[ -s "$NORMALIZED_LOG" && -n "$(tail -c 1 "$NORMALIZED_LOG")" ]]; then
    return 1
  fi
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|post-recovery (interaction|focus|focus roundtrip|lifecycle[- ]focus) runtime (failed|timed out)|service[- ]dependency runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAIL|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M53 emitted a panic, failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0([[:space:]]|$)|route=surface-direct|surface_fallback=[1-9][0-9]*|legacy_(key_reads|path)=[1-9][0-9]*|^(SURFACE_INPUT|SURFACE_KEY)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M53 fell back to a retired SurfaceServer-owned or legacy input path." >&2
    exit 1
  fi
  if grep -Eq "$STALE_INPUT_RUNTIME_REGEX|$STALE_SUPERVISOR_REGEX|$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M53 leaked stale or unrelated leaf-runtime evidence." >&2
    exit 1
  fi
  if grep -Eq '^(SERVICE_DEPENDENCY_STABLE|SERVICE_DEPENDENCY_OK)( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M53 incorrectly published an M49 terminal success marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M53 published a stale or unrelated BOOT_OK marker." >&2
    exit 1
  fi
  local unknown_post
  unknown_post="$(grep '^POST_RECOVERY_' "$NORMALIZED_LOG" | grep -Ev '^(POST_RECOVERY_ARMED|POST_RECOVERY_REJECTED|POST_RECOVERY_ROUTED|POST_RECOVERY_PRESENTED|POST_RECOVERY_HEALTHY|POST_RECOVERY_INTERACTION_OK|POST_RECOVERY_FOCUS_ARMED|POST_RECOVERY_LAUNCHER_CAPTURED|POST_RECOVERY_LAUNCHER_PRESENTED|POST_RECOVERY_FOCUS_OK|POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED|POST_RECOVERY_APP_CAPTURED|POST_RECOVERY_APP_PRESENTED|POST_RECOVERY_FOCUS_ROUNDTRIP_OK|POST_RECOVERY_LIFECYCLE_SESSION_READY|POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED|POST_RECOVERY_LIFECYCLE_APP_SYNCED|POST_RECOVERY_LIFECYCLE_FOCUS_OK)( |$)' || true)"
  if [[ -n "$unknown_post" ]]; then
    show_failure
    echo "M53 published an unknown POST_RECOVERY_ marker." >&2
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
  case "$action" in
    m49-pointer-down)
      QMP_INPUT_SEND_EVENTS=$((QMP_INPUT_SEND_EVENTS + 2))
      ;;
    m49-pointer-up | invalid-down | app-down | launcher-down | touch-up)
      QMP_INPUT_SEND_EVENTS=$((QMP_INPUT_SEND_EVENTS + 1))
      ;;
    screendump | quit) ;;
    *)
      echo "unsupported local QMP action accounting: $action" >&2
      exit 2
      ;;
  esac
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
    "app-down": (13970, 12587, 136, 184),
    "launcher-down": (8218, 6568, 80, 96),
}
for raw_x, raw_y, expected_x, expected_y in coordinates.values():
    for raw, extent, expected in (
        (raw_x, 320, expected_x),
        (raw_y, 480, expected_y),
    ):
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

def touch_down(name):
    raw_x, raw_y, _, _ = coordinates[name]
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

command({"execute": "qmp_capabilities"})
if action == "m49-pointer-down":
    absolute("app-down")
    touch(True)
elif action == "m49-pointer-up":
    touch(False)
elif action in coordinates:
    touch_down(action)
elif action == "touch-up":
    touch(False)
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
    "$M50_PPM" \
    "$M51_PPM" \
    "$M52_PPM" \
    "$M53_PPM" \
    "$DISCOVER_HASHES" \
    "$EXPECTED_M50_SHA256" \
    "$EXPECTED_M51_SHA256" \
    "$EXPECTED_M52_SHA256" \
    "$EXPECTED_M53_SHA256" <<'PY'
import hashlib
import re
import sys

m50_path, m51_path, m52_path, m53_path = sys.argv[1:5]
discover = sys.argv[5] == "1"
expected_m50, expected_m51, expected_m52, expected_m53 = sys.argv[6:10]

def read_ppm(path):
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
    with open(path, "rb") as source:
        digest = hashlib.sha256(source.read()).hexdigest()
    return pixels, digest

def exact_damage(before, after, x0, y0, width, height, label):
    changed = set()
    for pixel in range(320 * 480):
        offset = pixel * 3
        if before[offset:offset + 3] != after[offset:offset + 3]:
            changed.add((pixel % 320, pixel // 320))
    expected = {
        (x, y)
        for y in range(y0, y0 + height)
        for x in range(x0, x0 + width)
    }
    if changed != expected:
        escaped = sorted(changed - expected)[:8]
        missing = sorted(expected - changed)[:8]
        raise SystemExit(
            f"{label}: changed={len(changed)}, expected={len(expected)}, "
            f"escaped={escaped!r}, missing={missing!r}"
        )

m50, m50_hash = read_ppm(m50_path)
m51, m51_hash = read_ppm(m51_path)
m52, m52_hash = read_ppm(m52_path)
m53, m53_hash = read_ppm(m53_path)
if m50_hash != expected_m50:
    raise SystemExit(f"M50 prefix SHA-256 changed: {m50_hash}, expected {expected_m50}")
if m51_hash != expected_m51:
    raise SystemExit(f"M51 prefix SHA-256 changed: {m51_hash}, expected {expected_m51}")
if m52_hash != expected_m52:
    raise SystemExit(f"M52 prefix SHA-256 changed: {m52_hash}, expected {expected_m52}")
if len({m50_hash, m51_hash, m52_hash}) != 3:
    raise SystemExit("M50, M51, and M52 screenshots are not three distinct frames")

exact_damage(m50, m51, 64, 80, 40, 32, "M50-to-M51 focus damage")
exact_damage(m51, m52, 160, 216, 40, 32, "M51-to-M52 roundtrip damage")
exact_damage(m52, m53, 0, 0, 0, 0, "M52-to-M53 lifecycle-focus damage")
if m53 != m52 or m53_hash != m52_hash:
    raise SystemExit(
        "M53 lifecycle-focus convergence changed the frozen M52 framebuffer: "
        f"{m52_hash} != {m53_hash}"
    )

if not discover:
    if not re.fullmatch(r"[0-9a-f]{64}", expected_m53) or expected_m53 == "0" * 64:
        raise SystemExit("M53 final expected SHA-256 is not frozen")
    if m53_hash != expected_m53:
        raise SystemExit(f"M53 final SHA-256 changed: {m53_hash}, expected {expected_m53}")
print(m50_hash, m51_hash, m52_hash, m53_hash)
PY
}

validate_final_evidence() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import re
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    lines = [line.rstrip("\n") for line in source]

schemas = {
    "SERVICE_DEPENDENCY_ARMED": (
        "services", "dependency", "surface_pid", "input_pid",
        "surface_generation", "input_generation", "session", "epoch",
        "floor", "budget",
    ),
    "SERVICE_DEPENDENCY_SURFACE_GAP": (
        "fault", "surface_generation", "attempt", "input_alive",
        "route_epoch", "floor",
    ),
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED": (
        "surface_generation", "surface_session", "input_generation",
        "input_session", "route_epoch", "floor", "frame",
    ),
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY": (
        "probes", "healthy", "surface_generation", "timeout_ns",
    ),
    "SERVICE_DEPENDENCY_WATCHDOG": (
        "service", "generation", "probes", "reads", "healthy",
        "timeout_ns", "impact",
    ),
    "SERVICE_DEPENDENCY_DEGRADED": (
        "surface_alive", "input_alive", "phase", "frame",
        "write_generation", "floor",
    ),
    "SERVICE_DEPENDENCY_INPUT_REBOUND": (
        "input_generation", "input_session", "route_epoch", "floor",
        "bic", "bie",
    ),
    "SERVICE_DEPENDENCY_INPUT_HEALTHY": (
        "probes", "reads", "healthy", "input_generation",
    ),
    "POST_RECOVERY_ARMED": (
        "abi", "surface_session", "input_session", "route_epoch", "floor", "frame",
    ),
    "POST_RECOVERY_REJECTED": ("physical", "target", "routed", "frame"),
    "POST_RECOVERY_ROUTED": ("physical", "target", "events", "capture"),
    "POST_RECOVERY_PRESENTED": (
        "command", "app_frame", "scene", "output_frame", "write_generation",
    ),
    "POST_RECOVERY_HEALTHY": ("surface", "input"),
    "POST_RECOVERY_INTERACTION_OK": (
        "abi", "surface_session", "input_session", "route_epoch", "rejected",
        "physical", "target", "events", "capture", "app_present",
        "output_frame", "health", "resident", "errors",
    ),
    "POST_RECOVERY_FOCUS_ARMED": (
        "abi", "surface_session", "input_session", "route_epoch",
        "physical_floor", "focus", "output_frame",
    ),
    "POST_RECOVERY_LAUNCHER_CAPTURED": (
        "physical", "target", "events", "focus", "capture",
    ),
    "POST_RECOVERY_LAUNCHER_PRESENTED": (
        "physical", "command", "launcher_frame", "scene", "output_frame",
        "write_generation", "focus", "capture",
    ),
    "POST_RECOVERY_FOCUS_OK": (
        "abi", "surface_session", "input_session", "route_epoch", "physical",
        "target", "events", "capture", "launcher_present", "output_frame",
        "focus", "health", "resident", "errors",
    ),
    "POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED": (
        "abi", "surface_session", "input_session", "route_epoch",
        "physical_floor", "focus", "output_frame",
    ),
    "POST_RECOVERY_APP_CAPTURED": (
        "physical", "target", "events", "focus", "capture",
    ),
    "POST_RECOVERY_APP_PRESENTED": (
        "physical", "command", "app_frame", "scene", "output_frame",
        "write_generation", "focus", "capture",
    ),
    "POST_RECOVERY_FOCUS_ROUNDTRIP_OK": (
        "abi", "surface_session", "input_session", "route_epoch", "physical",
        "target", "events", "capture", "app_present", "output_frame",
        "focus", "health", "resident", "errors",
    ),
    "POST_RECOVERY_LIFECYCLE_SESSION_READY": (
        "surface_session", "clients", "ready_events", "focus",
        "lifecycle_events", "ack", "channels",
    ),
    "POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED": (
        "physical", "clients", "focus", "lifecycle_events", "ack",
        "channels", "compositor", "input",
    ),
    "POST_RECOVERY_LIFECYCLE_APP_SYNCED": (
        "physical", "clients", "focus", "lifecycle_events", "ack",
        "channels", "compositor", "input",
    ),
    "POST_RECOVERY_LIFECYCLE_FOCUS_OK": (
        "abi", "surface_session", "input_session", "route_epoch", "physical",
        "clients", "ready_events", "focus_events", "acks", "lifecycle",
        "compositor", "input", "channels", "m52_boundary", "output_frame",
        "health", "resident", "errors",
    ),
}

expected = {
    "SERVICE_DEPENDENCY_ARMED": {
        "services": "2", "dependency": "input-soft-surface",
        "surface_generation": "1", "input_generation": "1", "session": "1",
        "epoch": "1", "floor": "0", "budget": "1",
    },
    "SERVICE_DEPENDENCY_SURFACE_GAP": {
        "fault": "process-exit", "surface_generation": "1", "attempt": "1",
        "input_alive": "1", "route_epoch": "2", "floor": "2",
    },
    "SERVICE_DEPENDENCY_SURFACE_RECOVERED": {
        "surface_generation": "2", "surface_session": "2",
        "input_generation": "1", "input_session": "1", "route_epoch": "2",
        "floor": "3", "frame": "1",
    },
    "SERVICE_DEPENDENCY_SURFACE_HEALTHY": {
        "probes": "1", "healthy": "1", "surface_generation": "2",
        "timeout_ns": "100000000",
    },
    "SERVICE_DEPENDENCY_WATCHDOG": {
        "service": "input", "generation": "1", "probes": "1", "reads": "1",
        "healthy": "0", "timeout_ns": "100000000",
        "impact": "input-hard/surface-soft",
    },
    "SERVICE_DEPENDENCY_DEGRADED": {
        "surface_alive": "1", "input_alive": "0", "phase": "route-lost",
        "frame": "2", "write_generation": "2", "floor": "3",
    },
    "SERVICE_DEPENDENCY_INPUT_REBOUND": {
        "input_generation": "2", "input_session": "2", "route_epoch": "3",
        "floor": "3", "bic": "6", "bie": "7",
    },
    "SERVICE_DEPENDENCY_INPUT_HEALTHY": {
        "probes": "1", "reads": "1", "healthy": "1", "input_generation": "2",
    },
    "POST_RECOVERY_ARMED": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "floor": "3", "frame": "3",
    },
    "POST_RECOVERY_REJECTED": {
        "physical": "4/5", "target": "none", "routed": "0", "frame": "3",
    },
    "POST_RECOVERY_ROUTED": {
        "physical": "6/7", "target": "app", "events": "2", "capture": "1",
    },
    "POST_RECOVERY_PRESENTED": {
        "command": "5", "app_frame": "3", "scene": "10",
        "output_frame": "4", "write_generation": "4",
    },
    "POST_RECOVERY_HEALTHY": {"surface": "2/2", "input": "2/2"},
    "POST_RECOVERY_INTERACTION_OK": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "rejected": "4..5", "physical": "6..7",
        "target": "app", "events": "2", "capture": "1", "app_present": "1",
        "output_frame": "4", "health": "2/2", "resident": "1", "errors": "0",
    },
    "POST_RECOVERY_FOCUS_ARMED": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "physical_floor": "7", "focus": "app/3",
        "output_frame": "4",
    },
    "POST_RECOVERY_LAUNCHER_CAPTURED": {
        "physical": "8", "target": "launcher", "events": "1",
        "focus": "launcher/4", "capture": "1",
    },
    "POST_RECOVERY_LAUNCHER_PRESENTED": {
        "physical": "8/9", "command": "6", "launcher_frame": "4",
        "scene": "11", "output_frame": "5", "write_generation": "5",
        "focus": "launcher/4", "capture": "0",
    },
    "POST_RECOVERY_FOCUS_OK": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "physical": "8..9", "target": "launcher",
        "events": "2", "capture": "0", "launcher_present": "1",
        "output_frame": "5", "focus": "launcher/4", "health": "2/2",
        "resident": "1", "errors": "0",
    },
    "POST_RECOVERY_FOCUS_ROUNDTRIP_ARMED": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "physical_floor": "9", "focus": "launcher/4",
        "output_frame": "5",
    },
    "POST_RECOVERY_APP_CAPTURED": {
        "physical": "10", "target": "app", "events": "1",
        "focus": "app/5", "capture": "1",
    },
    "POST_RECOVERY_APP_PRESENTED": {
        "physical": "10/11", "command": "6", "app_frame": "4",
        "scene": "12", "output_frame": "6", "write_generation": "6",
        "focus": "app/5", "capture": "0",
    },
    "POST_RECOVERY_FOCUS_ROUNDTRIP_OK": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "physical": "10..11", "target": "app",
        "events": "2", "capture": "0", "app_present": "1",
        "output_frame": "6", "focus": "app/5", "health": "2/2",
        "resident": "1", "errors": "0",
    },
    "POST_RECOVERY_LIFECYCLE_SESSION_READY": {
        "surface_session": "2", "clients": "launcher/app", "ready_events": "2",
        "focus": "app/1", "lifecycle_events": "2", "ack": "rfck/2",
        "channels": "6/6",
    },
    "POST_RECOVERY_LIFECYCLE_LAUNCHER_SYNCED": {
        "physical": "8", "clients": "launcher/app", "focus": "launcher/2",
        "lifecycle_events": "2", "ack": "lfck/2", "channels": "10/10",
        "compositor": "launcher/4", "input": "launcher/3",
    },
    "POST_RECOVERY_LIFECYCLE_APP_SYNCED": {
        "physical": "10", "clients": "launcher/app", "focus": "app/3",
        "lifecycle_events": "2", "ack": "afck/2", "channels": "14/14",
        "compositor": "app/5", "input": "app/4",
    },
    "POST_RECOVERY_LIFECYCLE_FOCUS_OK": {
        "abi": "23", "surface_session": "2", "input_session": "2",
        "route_epoch": "3", "physical": "1..11", "clients": "2",
        "ready_events": "2", "focus_events": "6",
        "acks": "rfck2/lfck2/afck2", "lifecycle": "app/1-launcher/2-app/3",
        "compositor": "app/5", "input": "app/4", "channels": "14/14",
        "m52_boundary": "1/1", "output_frame": "6", "health": "2/2",
        "resident": "1", "errors": "0",
    },
}

def first_token(line):
    parts = line.split(maxsplit=1)
    return parts[0] if parts else ""

parsed = {}
marker_lines = {}
for prefix, ordered_schema in schemas.items():
    found = [line for line in lines if first_token(line) == prefix]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} marker, observed {found!r}")
    line = found[0]
    if not re.fullmatch(r"[^\t ]+(?: [^\t ]+)+", line):
        raise SystemExit(f"{prefix}: marker does not use strict single-space fields")
    values = {}
    ordered_keys = []
    for token in line.split(" ")[1:]:
        if "=" not in token:
            raise SystemExit(f"{prefix}: non-field token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in values:
            raise SystemExit(f"{prefix}: invalid or duplicate field {token!r}")
        values[key] = value
        ordered_keys.append(key)
    if ordered_keys != list(ordered_schema):
        raise SystemExit(
            f"{prefix}: ordered schema {ordered_keys!r}, expected {list(ordered_schema)!r}"
        )
    for key, value in expected[prefix].items():
        if values[key] != value:
            raise SystemExit(f"{prefix}: {key}={values[key]!r}, expected {value!r}")
    parsed[prefix] = values
    marker_lines[prefix] = line

def generation_qualified_pid(prefix, key, generation):
    raw = parsed[prefix][key]
    if not re.fullmatch(r"0x[0-9a-f]{16}", raw):
        raise SystemExit(f"{key} is not canonical hexadecimal: {raw!r}")
    value = int(raw, 0)
    if value <= 0 or value > (1 << 64) - 1 or value & 0xFFFFFFFF == 0:
        raise SystemExit(f"{key} is not a non-zero generation-qualified PID: {raw!r}")
    if value >> 32 != generation:
        raise SystemExit(f"{key} generation {value >> 32} does not match {generation}")
    return value

surface_pid = generation_qualified_pid("SERVICE_DEPENDENCY_ARMED", "surface_pid", 1)
input_pid = generation_qualified_pid("SERVICE_DEPENDENCY_ARMED", "input_pid", 1)
if surface_pid == input_pid:
    raise SystemExit("SurfaceServer and InputServer published the same PID")

focus_pattern = re.compile(
    r"UI_ROUTE_FOCUS_OK sender_image=surface-server sender_pid=([1-9][0-9]*) "
    r"receiver_image=(launcher|app) receiver_pid=([1-9][0-9]*) session=2 "
    r"active_client=(launcher|app) app=(none|phone) focus_generation=([1-3])"
)
session_focus_lines = [
    line
    for line in lines
    if line.startswith("UI_ROUTE_FOCUS_OK ")
    and re.search(r" session=2(?: |$)", line)
]
if len(session_focus_lines) != 6:
    raise SystemExit(
        f"expected six session-2 lifecycle focus routes, observed {session_focus_lines!r}"
    )
expected_focus = {
    (1, "launcher"): ("app", "phone"),
    (1, "app"): ("app", "phone"),
    (2, "launcher"): ("launcher", "none"),
    (2, "app"): ("launcher", "none"),
    (3, "launcher"): ("app", "phone"),
    (3, "app"): ("app", "phone"),
}
observed_focus = {}
focus_positions = {1: [], 2: [], 3: []}
sender_pids = set()
receiver_pids = {}
for line in session_focus_lines:
    match = focus_pattern.fullmatch(line)
    if match is None:
        raise SystemExit(f"malformed session-2 lifecycle focus route: {line!r}")
    sender_raw, receiver, receiver_raw, active, app, generation_raw = match.groups()
    generation = int(generation_raw)
    key = (generation, receiver)
    if key in observed_focus:
        raise SystemExit(f"duplicate session-2 lifecycle focus route: {line!r}")
    observed_focus[key] = (active, app)
    focus_positions[generation].append(lines.index(line))
    sender_pids.add(int(sender_raw))
    receiver_pid = int(receiver_raw)
    if receiver in receiver_pids and receiver_pids[receiver] != receiver_pid:
        raise SystemExit(f"{receiver} focus routes changed receiver PID")
    receiver_pids[receiver] = receiver_pid
if observed_focus != expected_focus:
    raise SystemExit(
        f"session-2 lifecycle focus transcript {observed_focus!r}, "
        f"expected {expected_focus!r}"
    )
if not (
    max(focus_positions[1]) < min(focus_positions[2])
    and max(focus_positions[2]) < min(focus_positions[3])
):
    raise SystemExit(f"session-2 lifecycle focus generations are out of order: {focus_positions!r}")
if len(sender_pids) != 1 or set(receiver_pids) != {"launcher", "app"}:
    raise SystemExit("session-2 lifecycle focus routes did not retain exact owners")
replacement_surface_pid = next(iter(sender_pids))
if (
    replacement_surface_pid >> 32 != 2
    or (replacement_surface_pid & 0xFFFFFFFF) != (surface_pid & 0xFFFFFFFF)
    or any(pid >> 32 != 1 or pid & 0xFFFFFFFF == 0 for pid in receiver_pids.values())
    or len({replacement_surface_pid, *receiver_pids.values()}) != 3
):
    raise SystemExit("session-2 lifecycle focus routes used invalid process generations")

boot = "BOOT_OK: M53 post-recovery lifecycle focus convergence verified"
boot_lines = [line for line in lines if line.startswith("BOOT_OK:")]
if boot_lines != [boot]:
    raise SystemExit(f"M53 must own the only boot marker, observed {boot_lines!r}")
ordered_lines = [marker_lines[prefix] for prefix in schemas] + [boot]
positions = [lines.index(line) for line in ordered_lines]
if positions != sorted(positions) or len(set(positions)) != len(positions):
    raise SystemExit(f"M49/M50/M51/M52/M53 proof markers are out of order: {positions!r}")

m49_prefixes = {prefix for prefix in schemas if prefix.startswith("SERVICE_DEPENDENCY_")}
post_prefixes = {prefix for prefix in schemas if prefix.startswith("POST_RECOVERY_")}
for line in lines:
    prefix = first_token(line)
    if line.startswith("SERVICE_DEPENDENCY_") and prefix not in m49_prefixes:
        raise SystemExit(f"unexpected M49 service-dependency marker: {line!r}")
    if line.startswith("POST_RECOVERY_") and prefix not in post_prefixes:
        raise SystemExit(f"unexpected post-recovery marker: {line!r}")

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
        or re.search(r"\b(?:surface_fallback|legacy_key_reads|legacy_path)=[1-9][0-9]*\b", line)
    ):
        raise SystemExit("retired SurfaceServer input ownership leaked into M53")
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

# Replay the immutable M49 dependency and M50 post-recovery prefixes.
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
qmp_action invalid-down
qmp_action touch-up
wait_for_prefix_once "$POST_REJECTED" "the M50 rejected-contact marker"
qmp_action app-down
qmp_action touch-up
wait_for_prefix_once "$POST_ROUTED" "the M50 routed-contact marker"
wait_for_prefix_once "$POST_PRESENTED" "the M50 presented-frame marker"
wait_for_prefix_once "$POST_HEALTHY" "the M50 second-round health marker"
wait_for_prefix_once "$POST_SUCCESS" "the M50 interaction marker"

# M51 arms on the frozen M50 frame. Hold each down until the guest publishes
# its authenticated capture before releasing the next physical sequence.
wait_for_prefix_once "$FOCUS_ARMED" "the M51 focus-armed marker"
qmp_action screendump "$M50_PPM"
qmp_action launcher-down
wait_for_prefix_once "$LAUNCHER_CAPTURED" "the M51 Launcher-captured marker"
qmp_action touch-up
wait_for_prefix_once "$LAUNCHER_PRESENTED" "the M51 Launcher-presented marker"
qmp_action screendump "$M51_PPM"
wait_for_prefix_once "$FOCUS_SUCCESS" "the M51 focus marker"

# Replay the immutable M52 App contact. Its down/up remain physical 10/11 and
# its screenshot follows committed frame 6, but M53 suppresses M52's BOOT_OK.
wait_for_prefix_once "$ROUNDTRIP_ARMED" "the M52 focus-roundtrip armed marker"
qmp_action app-down
wait_for_prefix_once "$APP_CAPTURED" "the M52 App-captured marker"
qmp_action touch-up
wait_for_prefix_once "$APP_PRESENTED" "the M52 App-presented marker"
qmp_action screendump "$M52_PPM"
wait_for_prefix_once "$ROUNDTRIP_SUCCESS" "the M52 roundtrip marker"
if [[ "$QMP_INPUT_SEND_EVENTS" != "11" ]]; then
  show_failure
  echo "M49--M52 did not use the exact eleven QMP input-send-event commands." >&2
  exit 1
fi

# M53 is autonomous: it authenticates the session-2 Ready/focus/ACK history
# without injecting another input report or committing another output frame.
wait_for_prefix_once "$LIFECYCLE_SESSION_READY" "the M53 session-ready marker"
wait_for_prefix_once "$LIFECYCLE_LAUNCHER_SYNCED" "the M53 Launcher-focus marker"
wait_for_prefix_once "$LIFECYCLE_APP_SYNCED" "the M53 App-focus marker"
wait_for_prefix_once "$LIFECYCLE_SUCCESS" "the final M53 lifecycle-focus marker"
wait_for_exact_once "$BOOT_MARKER" "the final M53 boot marker"
qmp_action screendump "$M53_PPM"
if [[ "$QMP_INPUT_SEND_EVENTS" != "11" ]]; then
  show_failure
  echo "M53 injected an unexpected QMP input-send-event command." >&2
  exit 1
fi

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ! reject_failure; then
  show_failure
  echo "M53 serial log ended with an incomplete evidence line." >&2
  exit 1
fi
validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M53 did not preserve the exact unique M25 storage evidence." >&2
  exit 1
fi
if [[ "$(grep -Fc 'entered AArch64 EL1' "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fc 'running EL1' "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M53 did not publish unique entered/running EL1 evidence." >&2
  exit 1
fi

read -r M50_SHA256 M51_SHA256 M52_SHA256 M53_SHA256 < <(validate_screenshots)

mkdir -p "$OUTPUT_DIR"
cp "$M50_PPM" "$OUTPUT_DIR/m50-presented.ppm"
cp "$M51_PPM" "$OUTPUT_DIR/m51-presented.ppm"
cp "$M52_PPM" "$OUTPUT_DIR/m52-presented.ppm"
cp "$M53_PPM" "$OUTPUT_DIR/m53-lifecycle-focus.ppm"

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
  echo "QEMU did not exit within ${BOOT_TIMEOUT_SECONDS}s after the M53 QMP quit." >&2
  exit 1
fi
if [[ "$QEMU_STATUS" != "0" ]]; then
  show_failure
  echo "QEMU did not exit cleanly after the M53 QMP quit (status $QEMU_STATUS)." >&2
  exit 1
fi

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
if ! reject_failure; then
  show_failure
  echo "M53 serial log ended with an incomplete evidence line after QMP quit." >&2
  exit 1
fi
validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M53 post-quit storage evidence changed or duplicated." >&2
  exit 1
fi

if [[ "$DISCOVER_HASHES" == "1" ]]; then
  echo "POST_RECOVERY_LIFECYCLE_FOCUS_DISCOVER_HASHES m50_sha256=$M50_SHA256 m51_sha256=$M51_SHA256 m52_sha256=$M52_SHA256 m53_sha256=$M53_SHA256 diffs=1280/1280/0 damage=64/80/40/32+160/216/40/32+none"
else
  echo "POST_RECOVERY_LIFECYCLE_FOCUS_QMP_OK physical=1..11 m53_input=0 lifecycle=app1-launcher2-app3 clients=2 ready=2 focus_events=6 acks=rfck2/lfck2/afck2 channels=14/14 m52_boundary=1/1 compositor=app5 input=app4 output=6-6 diffs=1280/1280/0 damage=64/80/40/32+160/216/40/32+none m50_sha256=$M50_SHA256 m51_sha256=$M51_SHA256 m52_sha256=$M52_SHA256 m53_sha256=$M53_SHA256 markers=m49-prefix/m50-prefix/m51-prefix/m52-prefix/session-ready/launcher-synced/app-synced/lifecycle-focus/boot"
fi
