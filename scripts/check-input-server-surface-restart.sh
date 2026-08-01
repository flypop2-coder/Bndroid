#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M46 InputServer surface-restart runtime." >&2
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

TARGET_ROOT="${BNDROID_INPUT_SERVER_SURFACE_RESTART_TARGET_DIR:-$WORKSPACE_ROOT/target/input-server-surface-restart-runtime}"
CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES=input-server-surface-restart-runtime \
  BNDROID_KERNEL_FEATURES=input-server-surface-restart-runtime,surface-trace-evidence \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M46 InputServer surface-restart kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bndroid-input-server-surface-restart.XXXXXX")"
SERIAL_LOG="$TMP_DIR/serial.log"
NORMALIZED_LOG="$TMP_DIR/serial.normalized.log"
QEMU_LOG="$TMP_DIR/qemu.log"
QMP_SOCKET="$TMP_DIR/qmp.sock"
PRE_PPM="$TMP_DIR/input-server-surface-restart-pre.ppm"
FROZEN_PPM="$TMP_DIR/input-server-surface-restart-frozen.ppm"
POST_PPM="$TMP_DIR/input-server-surface-restart-post.ppm"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"

ARMED_PREFIX="INPUT_SERVER_SURFACE_RESTART_ARMED"
GAP_READY_PREFIX="INPUT_SERVER_ROUTE_GAP_READY"
EPOCH_OK_PREFIX="INPUT_SERVER_ROUTE_EPOCH_OK"
GAP_OK_PREFIX="INPUT_SERVER_ROUTE_GAP_OK"
CANCEL_OK_PREFIX="INPUT_SERVER_CAPTURE_CANCEL_OK"
REBIND_OK_PREFIX="INPUT_SERVER_SURFACE_REBIND_OK"
SUCCESS_PREFIX="INPUT_SERVER_SURFACE_RESTART_OK"
BOOT_MARKER="BOOT_OK: M46 InputServer SurfaceServer restart rebind, route-epoch gap recovery, and capture cancellation verified"
KEYBOARD_READY="INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1"
TABLET_READY="POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1"
OTHER_RUNTIME_REGEX='^(PROCESS_TERMINATE_OK|APP_LIFECYCLE_OK|APP_CRASH_RECOVERY_OK|MAPPED_GRAPHICS_OK|GRAPHICS_OWNER_DEATH_OK|GRAPHICS_SURFACE_RESTART_OK|GRAPHICS_PRODUCER_ORPHAN_OK|GRAPHICS_FRAME_CLOCK_OK|GRAPHICS_SWAPCHAIN_OK|WINDOW_COMPOSITOR_OK|WINDOW_SESSION_OK) '

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "M46 diagnostic artifacts retained at $TMP_DIR" >&2
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
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|input[- ]server surface[- ]restart runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M46 emitted a panic, userspace failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Eq 'input_method=in_surface|input_server=0|route=surface-direct|surface_fallback=1' "$NORMALIZED_LOG"; then
    show_failure
    echo "M46 fell back to a retired SurfaceServer-owned input path." >&2
    exit 1
  fi
  if grep -Eq "$OTHER_RUNTIME_REGEX" "$NORMALIZED_LOG"; then
    show_failure
    echo "M46 published an unrelated leaf runtime's success evidence." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "M46 published a stale or unrelated BOOT_OK marker." >&2
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

assert_prefix_once() {
  local prefix="$1"
  local description="$2"
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  if [[ "$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "$description changed while advancing the M46 transcript." >&2
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
if action == "pointer-down":
    absolute()
    touch(True)
elif action == "pointer-up":
    # No intervening absolute event is emitted: release is at the exact
    # retained (136,184) coordinate from pointer-down.
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
  python3 - "$PRE_PPM" "$FROZEN_PPM" "$POST_PPM" <<'PY'
import hashlib
import sys

WIDTH, HEIGHT = 320, 480
SURFACE = (56, 64, 208, 368)
CORE_COLORS = {
    bytes.fromhex(value)
    for value in ("10162d", "162436", "f8fafc", "1f2937", "2e66f5")
}
PHASE_COLORS = {
    "pre": {bytes.fromhex(value) for value in ("29a36a", "385a7c", "844ec7")},
    "frozen": {bytes.fromhex("f97300")},
    "post": {bytes.fromhex("14b8a6")},
}
RECOVERY_COLORS = PHASE_COLORS["frozen"] | PHASE_COLORS["post"]

def load(path, phase, minimum_colors):
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
    missing = (CORE_COLORS | PHASE_COLORS[phase]) - colors
    if missing:
        raise SystemExit(
            f"{path}: missing {phase} UI colors {[c.hex() for c in missing]}"
        )
    unexpected_recovery = (RECOVERY_COLORS - PHASE_COLORS[phase]) & colors
    if unexpected_recovery:
        raise SystemExit(
            f"{path}: leaked recovery colors {[c.hex() for c in unexpected_recovery]}"
        )
    if len(colors) < minimum_colors:
        raise SystemExit(
            f"{path}: only {len(colors)} colors; expected at least {minimum_colors}"
        )
    return pixels

def pixel(pixels, x, y):
    offset = (y * WIDTH + x) * 3
    return pixels[offset:offset + 3]

def inside(rect, x, y):
    rx, ry, width, height = rect
    return rx <= x < rx + width and ry <= y < ry + height

pre = load(sys.argv[1], "pre", 8)
frozen = load(sys.argv[2], "frozen", 9)
post = load(sys.argv[3], "post", 9)
hashes = []
for path in sys.argv[1:]:
    with open(path, "rb") as source:
        hashes.append(hashlib.sha256(source.read()).hexdigest())
if len(set(hashes)) != 3:
    raise SystemExit(f"M46 screenshots are not three distinct phases: {hashes!r}")

pre_frozen = []
frozen_post = []
for y in range(HEIGHT):
    for x in range(WIDTH):
        if pixel(pre, x, y) != pixel(frozen, x, y):
            pre_frozen.append((x, y))
        if pixel(frozen, x, y) != pixel(post, x, y):
            frozen_post.append((x, y))

if not (1 <= len(pre_frozen) <= 8192):
    raise SystemExit(f"unexpected pre/frozen diff ledger: {len(pre_frozen)}")
if not (1 <= len(frozen_post) <= 8192):
    raise SystemExit(f"unexpected frozen/post diff ledger: {len(frozen_post)}")
for phase, changes in (("pre/frozen", pre_frozen), ("frozen/post", frozen_post)):
    outside = [(x, y) for x, y in changes if not inside(SURFACE, x, y)]
    if outside:
        raise SystemExit(f"{phase} changed outside the preserved app surface: {outside[0]}")

print(*hashes)
PY
}

validate_final_evidence() {
  python3 - "$NORMALIZED_LOG" <<'PY'
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    lines = [line.rstrip("\n") for line in source]

def one(prefix):
    found = [line for line in lines if line.startswith(prefix + " ")]
    if len(found) != 1:
        raise SystemExit(f"expected one {prefix!r} marker, observed {found!r}")
    return found[0]

def fields(prefix, expected_keys):
    line = one(prefix)
    result = {}
    for token in line.split()[1:]:
        if "=" not in token:
            raise SystemExit(f"{prefix}: non-field token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in result:
            raise SystemExit(f"{prefix}: invalid or duplicate field {token!r}")
        result[key] = value
    if set(result) != set(expected_keys):
        raise SystemExit(
            f"{prefix}: fields {sorted(result)!r}, expected {sorted(expected_keys)!r}"
        )
    return result

def require(marker, observed, expected):
    for key, value in expected.items():
        if observed.get(key) != value:
            raise SystemExit(
                f"{marker}: {key}={observed.get(key)!r}, expected {value!r}"
            )

armed = fields("INPUT_SERVER_SURFACE_RESTART_ARMED", {
    "route_epoch", "surface_session", "input_server", "surface", "capture",
})
require("armed", armed, {
    "route_epoch": "1", "surface_session": "1",
    "input_server": "resident", "surface": "ready", "capture": "none",
})

gap_ready = fields("INPUT_SERVER_ROUTE_GAP_READY", {
    "old_epoch", "new_epoch", "surface_session", "capture",
    "expected", "observed", "release",
})
require("gap-ready", gap_ready, {
    "old_epoch": "1", "new_epoch": "2", "surface_session": "1",
    "capture": "app", "release": "awaiting",
})
expected = int(gap_ready["expected"], 0)
observed = int(gap_ready["observed"], 0)
if expected <= 0 or observed != expected + 1:
    raise SystemExit(f"gap-ready is not one exact sequence gap: {gap_ready!r}")

epoch = fields("INPUT_SERVER_ROUTE_EPOCH_OK", {
    "old", "new", "stale_rejected", "reset",
})
require("route-epoch", epoch, {
    "old": "1", "new": "2", "stale_rejected": "1", "reset": "1",
})

gap = fields("INPUT_SERVER_ROUTE_GAP_OK", {
    "epoch", "expected", "observed", "rejected", "recovery", "resumed", "errors",
})
require("route-gap", gap, {
    "epoch": "2", "expected": gap_ready["expected"],
    "observed": gap_ready["observed"], "rejected": "1",
    "recovery": "snapshot", "resumed": "1", "errors": "0",
})

cancel = fields("INPUT_SERVER_CAPTURE_CANCEL_OK", {
    "old_epoch", "new_epoch", "target", "cancel", "client_contact", "final",
    "stale_release", "routed", "text_delta",
})
require("capture-cancel", cancel, {
    "old_epoch": "1", "new_epoch": "2", "target": "app",
    "cancel": "1", "client_contact": "1/1/1", "final": "none",
    "stale_release": "ignored", "routed": "0", "text_delta": "0",
})

rebind = fields("INPUT_SERVER_SURFACE_REBIND_OK", {
    "old_pid", "new_pid", "sessions", "epochs", "snapshot", "routes", "focus", "capture",
})
require("surface-rebind", rebind, {
    "sessions": "1/2", "epochs": "1/2", "snapshot": "1",
    "routes": "2", "focus": "app", "capture": "none",
})
old_pid = int(rebind["old_pid"], 0)
new_pid = int(rebind["new_pid"], 0)
if old_pid <= 0 or new_pid <= 0 or old_pid == new_pid:
    raise SystemExit(f"SurfaceServer identity did not advance: {rebind!r}")
if (old_pid & 0xFFFFFFFF) != (new_pid & 0xFFFFFFFF):
    raise SystemExit(f"SurfaceServer restart did not reuse one process slot: {rebind!r}")
if (new_pid >> 32) != (old_pid >> 32) + 1:
    raise SystemExit(f"SurfaceServer generation did not advance exactly once: {rebind!r}")

runtime = fields("INPUT_SERVER_SURFACE_RESTART_OK", {
    "abi", "protocol", "input_server_pid", "input_server_session",
    "surface_pids", "surface_sessions", "route_epochs", "rebind", "gaps",
    "capture_cancel", "stale_release", "broker", "surface_fifo",
    "legacy_key_reads", "input_method", "surface_fallback", "errors",
    "topology", "final_state", "final_app_resident",
})
require("runtime", runtime, {
    "abi": "23", "protocol": "1", "input_server_session": "1",
    "surface_pids": f"{rebind['old_pid']}/{rebind['new_pid']}",
    "surface_sessions": "1/2", "route_epochs": "1/2", "rebind": "1",
    "gaps": "1/1/1", "capture_cancel": "1", "stale_release": "ignored",
    "surface_fifo": "0/0", "legacy_key_reads": "0",
    "input_method": "in_input_server", "surface_fallback": "0", "errors": "0",
    "topology": "resident", "final_state": "ready", "final_app_resident": "1",
})
if int(runtime["input_server_pid"], 0) <= 0:
    raise SystemExit(f"InputServer is not resident: {runtime!r}")
broker = runtime["broker"].split("/")
if len(broker) != 4 or broker[:3] != ["3", "3", "0"]:
    raise SystemExit(f"unexpected post-restart broker ledger: {runtime!r}")
if not (1 <= int(broker[3], 0) <= 64):
    raise SystemExit(f"post-restart broker high-water is invalid: {runtime!r}")

if lines.count("BOOT_OK: M46 InputServer SurfaceServer restart rebind, route-epoch gap recovery, and capture cancellation verified") != 1:
    raise SystemExit("M46 boot marker is absent or duplicated")
for marker in (
    "INPUT_READY device=keyboard queue=8 buffers=8 key_code_a=30 qmp_injection_supported=1",
    "POINTER_READY device=tablet queue=8 buffers=8 abs_x=0/32767 abs_y=0/32767 btn_touch=330 qmp_injection_supported=1",
):
    if lines.count(marker) != 1:
        raise SystemExit(f"device marker {marker!r} is absent or duplicated")
if any(
    bad in line
    for line in lines
    for bad in ("input_method=in_surface", "input_server=0", "route=surface-direct", "surface_fallback=1")
):
    raise SystemExit("retired SurfaceServer input ownership leaked into M46")
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

# The guest arms the one-shot SurfaceServer failure only after the complete
# resident topology is ready. The down event establishes the old-epoch App
# capture; the guest then restarts SurfaceServer and exposes the intentional
# one-sequence gap before the checker sends the retained-coordinate release.
wait_for_prefix_once "$ARMED_PREFIX" "the M46 restart-armed marker"
qmp_action screendump "$PRE_PPM"
qmp_action pointer-down

wait_for_prefix_once "$GAP_READY_PREFIX" "the M46 route-gap-ready marker"
assert_prefix_once "$ARMED_PREFIX" "The restart-armed marker"
qmp_action screendump "$FROZEN_PPM"
qmp_action pointer-up

wait_for_prefix_once "$EPOCH_OK_PREFIX" "the route-epoch proof marker"
wait_for_prefix_once "$GAP_OK_PREFIX" "the sequence-gap recovery proof marker"
wait_for_prefix_once "$CANCEL_OK_PREFIX" "the capture-cancellation proof marker"
wait_for_prefix_once "$REBIND_OK_PREFIX" "the SurfaceServer rebind proof marker"
wait_for_prefix_once "$SUCCESS_PREFIX" "the final M46 runtime marker"

deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
while ((SECONDS < deadline)); do
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" == "1" ]]; then
    break
  fi
  if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    set +e
    wait "$QEMU_PID"
    status=$?
    set -e
    QEMU_PID=""
    show_failure
    echo "QEMU exited before final M46 boot evidence (status $status)." >&2
    exit 1
  fi
  sleep 0.05
done

tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
reject_failure
for prefix in \
  "$ARMED_PREFIX" "$GAP_READY_PREFIX" "$EPOCH_OK_PREFIX" "$GAP_OK_PREFIX" \
  "$CANCEL_OK_PREFIX" "$REBIND_OK_PREFIX" "$SUCCESS_PREFIX"; do
  if [[ "$(grep -Ec "^${prefix} .+\$" "$NORMALIZED_LOG" || true)" != "1" ]]; then
    show_failure
    echo "M46 proof marker $prefix is absent or duplicated." >&2
    exit 1
  fi
done
if [[ "$(grep -Fxc "$BOOT_MARKER" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$KEYBOARD_READY" "$NORMALIZED_LOG" || true)" != "1" \
  || "$(grep -Fxc "$TABLET_READY" "$NORMALIZED_LOG" || true)" != "1" ]]; then
  show_failure
  echo "M46 did not publish each boot and input-device marker exactly once." >&2
  exit 1
fi

qmp_action screendump "$POST_PPM"
read -r PRE_SHA256 FROZEN_SHA256 POST_SHA256 < <(validate_screenshots)
validate_final_evidence
if ! validate_storage_success_evidence "$NORMALIZED_LOG"; then
  show_failure
  echo "M46 did not preserve the exact M25 storage evidence." >&2
  exit 1
fi
if ! grep -Fq 'entered AArch64 EL1' "$NORMALIZED_LOG" \
  || ! grep -Fq 'running EL1' "$NORMALIZED_LOG"; then
  show_failure
  echo "M46 did not enter and normalize execution at EL1." >&2
  exit 1
fi

cp "$PRE_PPM" "$WORKSPACE_ROOT/target/bndroid-m46-input-server-surface-restart-pre.ppm"
cp "$FROZEN_PPM" "$WORKSPACE_ROOT/target/bndroid-m46-input-server-surface-restart-frozen.ppm"
cp "$POST_PPM" "$WORKSPACE_ROOT/target/bndroid-m46-input-server-surface-restart.ppm"

echo "INPUT_SERVER_SURFACE_RESTART_QMP_OK armed=1 gap_ready=1 route_epoch=1 route_gap=1 capture_cancel=1 client_contact=1/1/1 surface_rebind=1 broker=kernel/userspace surface_fallback=0 pointer=136/184/down-restart-up screenshots=pre-frozen-post pre_sha256=$PRE_SHA256 frozen_sha256=$FROZEN_SHA256 post_sha256=$POST_SHA256 markers=epoch/gap/cancel/rebind/runtime/boot"
