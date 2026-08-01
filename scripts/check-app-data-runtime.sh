#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

APP_DATA_ASYNC_RECOVERY_MODE="${BNDROID_APP_DATA_ASYNC_RECOVERY_MODE:-0}"
if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" != "0" && "$APP_DATA_ASYNC_RECOVERY_MODE" != "1" ]]; then
  echo "BNDROID_APP_DATA_ASYNC_RECOVERY_MODE must be 0 or 1." >&2
  exit 2
fi
MILESTONE="M54"
if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
  MILESTONE="M59"
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep tail kill cp mkdir rm sleep cat; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the $MILESTONE AppData runtime." >&2
    exit 1
  fi
done

M53_CHECKER="$SCRIPT_DIR/check-post-recovery-lifecycle-focus.sh"
if [[ ! -f "$M53_CHECKER" ]]; then
  echo "The immutable M53 prefix checker is missing: $M53_CHECKER" >&2
  exit 1
fi
M59_CHECKER="$SCRIPT_DIR/check-app-data-async-recovery-runtime.sh"
if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" && ! -f "$M59_CHECKER" ]]; then
  echo "The M59 AppData evidence parser is missing: $M59_CHECKER" >&2
  exit 1
fi

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-40}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
PROFILE="${BNDROID_PROFILE:-debug}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be an integer from 1 through 600." >&2
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

EXPECTED_M50_SHA256="798d5cbce5830b315444967974ad8abcbf77f19fea87063a554cfc986e149974"
EXPECTED_M51_SHA256="cb84032b533910a88c74a767148702894336b9bf8409663fbd2f9aa49f8ec729"
EXPECTED_M52_SHA256="97807add9d09682d39e75ace000bc1ccb7548c6b5e97854bde1f3166976e6dfb"
EXPECTED_M54_SHA256="$EXPECTED_M52_SHA256"
BOOT_MARKER="BOOT_OK: M54 capability-scoped crash-safe AppData runtime verified"
if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
  BOOT_MARKER="BOOT_OK: M59 cooperative kernel-monitor AppData recovery verified"
fi
AUTHORITY_MARKER="APPDATA_AUTHORITY_OK principal=1 root_success=1/6 non_app_denied=4 identities=init/launcher/surface-v1/surface-v2 attenuated_write_denied=1 transfer_escalation_denied=1 validation_rejected=12 system_write_rejected=1 data_write_rejected=1"
M53_BOOT_MARKER="BOOT_OK: M53 post-recovery lifecycle focus convergence verified"

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

if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
  TARGET_ROOT="${BNDROID_APP_DATA_ASYNC_RECOVERY_TARGET_DIR:-$WORKSPACE_ROOT/target/app-data-async-recovery-runtime}"
  KERNEL_FEATURES="app-data-async-recovery-runtime"
  USERSPACE_FEATURES="app-data-async-recovery-runtime"
  IMAGE_BASENAME="bndroid-storage-m59.raw"
  TMP_PREFIX="bndroid-appdata-m59"
  OUTPUT_DIR="$WORKSPACE_ROOT/target/m59"
else
  TARGET_ROOT="${BNDROID_APP_DATA_RUNTIME_TARGET_DIR:-$WORKSPACE_ROOT/target/app-data-runtime}"
  KERNEL_FEATURES="app-data-runtime"
  USERSPACE_FEATURES="app-data-runtime"
  IMAGE_BASENAME="bndroid-storage-m54.raw"
  TMP_PREFIX="bndroid-appdata-m54"
  OUTPUT_DIR="$WORKSPACE_ROOT/target/m54"
fi
mkdir -p "$TARGET_ROOT" "$WORKSPACE_ROOT/target"
if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
  # Exercise the production parser's negative fixtures before trusting it
  # with guest evidence. This is local CPU work and cannot start QEMU.
  bash "$M59_CHECKER" --parser-self-test >/dev/null
fi
CANONICAL_IMAGE="$TARGET_ROOT/$IMAGE_BASENAME"
python3 "$SCRIPT_DIR/build_storage_image.py" build "$CANONICAL_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$CANONICAL_IMAGE" --appdata
if ! validate_storage_image_geometry "$CANONICAL_IMAGE"; then
  echo "$MILESTONE storage image does not have the required 8 MiB geometry." >&2
  exit 1
fi

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_USERSPACE_FEATURES="$USERSPACE_FEATURES" \
  BNDROID_KERNEL_FEATURES="$KERNEL_FEATURES" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "$MILESTONE AppData kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/$TMP_PREFIX.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
RECOVERY_IMAGE="$TMP_DIR/recovery.raw"
AFTER_BOOT1_IMAGE="$TMP_DIR/after-boot1.raw"
AFTER_BOOT2_IMAGE="$TMP_DIR/after-boot2.raw"
cp "$CANONICAL_IMAGE" "$RUNTIME_IMAGE"

QEMU_PID=""
SERIAL_LOG=""
NORMALIZED_LOG=""
QEMU_LOG=""
QMP_SOCKET=""
M50_PPM=""
M51_PPM=""
M52_PPM=""
M54_PPM=""
QMP_INPUT_SEND_EVENTS=0

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    echo "$MILESTONE diagnostic artifacts retained at $TMP_DIR" >&2
  else
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -n "$SERIAL_LOG" && -f "$SERIAL_LOG" ]]; then
    tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
    cat "$NORMALIZED_LOG"
  fi
  if [[ -n "$QEMU_LOG" && -f "$QEMU_LOG" ]]; then
    cat "$QEMU_LOG" >&2
  fi
}

reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|panic:|panicked at|boot error:|BOOT_FAIL|STORAGE_FAIL|USER_FAIL:|USER_FAIL |EL0_FAIL|THREAD_EXITED:|runtime (failed|timed out)|^[A-Z0-9_]+_(DIAG|TIMEOUT|FAIL|FAILED)(:| |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "$MILESTONE emitted a panic, failure, diagnostic, timeout, or failed marker." >&2
    exit 1
  fi
  if grep -Fqx "$M53_BOOT_MARKER" "$NORMALIZED_LOG"; then
    show_failure
    echo "$MILESTONE leaked the retired M53 BOOT_OK marker." >&2
    exit 1
  fi
  if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]] \
    && grep -Eq '^STORAGE_SERVER_RECOVERY_OK( |$)|^STORAGE_SERVER_REPEATED_RECOVERY_OK( |$)|^STORAGE_SERVER_ASYNC_RECOVERY_OK( |$)|^STORAGE_SERVER_FAULT_POLICY_OK( |$)|^STORAGE_IRQ_RACE_OK( |$)|^STORAGE_IRQ_COOPERATIVE_RECOVERY_OK( |$)|^BOOT_OK: M54 |^BOOT_OK: M60( |$)' "$NORMALIZED_LOG"; then
    show_failure
    echo "M59 observed an M54, M56--M60, or timeout-profile marker." >&2
    exit 1
  fi
  if grep '^BOOT_OK:' "$NORMALIZED_LOG" | grep -Fvx "$BOOT_MARKER" >/dev/null; then
    show_failure
    echo "$MILESTONE published a stale or unrelated BOOT_OK marker." >&2
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
      show_failure
      echo "QEMU exited before $description." >&2
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
      show_failure
      echo "QEMU exited before $description." >&2
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
    m49-pointer-down) QMP_INPUT_SEND_EVENTS=$((QMP_INPUT_SEND_EVENTS + 2)) ;;
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
timeout = float(timeout_text)
coordinates = {
    "invalid-down": (1644, 2190, 16, 32),
    "app-down": (13970, 12587, 136, 184),
    "launcher-down": (8218, 6568, 80, 96),
}
for raw_x, raw_y, expected_x, expected_y in coordinates.values():
    for raw, extent, expected in ((raw_x, 320, expected_x), (raw_y, 480, expected_y)):
        mapped = raw * (extent - 1) // 32767
        previous = (raw - 1) * (extent - 1) // 32767
        if mapped != expected or previous == expected:
            raise SystemExit("QMP absolute-coordinate mapping changed")

connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
connection.settimeout(timeout)
deadline = time.monotonic() + timeout
while time.monotonic() < deadline:
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        time.sleep(0.01)
else:
    raise SystemExit("QMP socket did not become available")

stream = connection.makefile("rwb", buffering=0)
greeting = stream.readline()
if not greeting or "QMP" not in json.loads(greeting):
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

def touch_down(name):
    raw_x, raw_y, _, _ = coordinates[name]
    command({"execute": "input-send-event", "arguments": {"events": [
        {"type": "abs", "data": {"axis": "x", "value": raw_x}},
        {"type": "abs", "data": {"axis": "y", "value": raw_y}},
        {"type": "btn", "data": {"down": True, "button": "touch"}},
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

# Execute the exact M53 transcript validator as the M54 prefix validator. Only
# its four ABI expectations and its terminal marker differ. This makes M54
# inherit the complete M49--M53 topology instead of maintaining a looser copy.
validate_m53_prefix_evidence() {
  local log_file="$1"
  python3 - "$M53_CHECKER" "$log_file" "$BOOT_MARKER" <<'PY'
import sys
from pathlib import Path

checker = Path(sys.argv[1]).read_text(encoding="utf-8")
log_path = sys.argv[2]
new_boot = sys.argv[3]
function = checker.index("validate_final_evidence() {")
start = checker.index("<<'PY'\n", function) + len("<<'PY'\n")
end = checker.index("\nPY\n}", start)
validator = checker[start:end]
if validator.count('"abi": "23"') != 7:
    raise SystemExit("the frozen M53 validator no longer has seven ABI-23 leaves")
validator = validator.replace('"abi": "23"', '"abi": "24"')
old_boot = "BOOT_OK: M53 post-recovery lifecycle focus convergence verified"
if validator.count(old_boot) != 1:
    raise SystemExit("the frozen M53 validator terminal marker changed")
validator = validator.replace(old_boot, new_boot)
sys.argv = ["appdata-prefix-validator", log_path]
exec(compile(validator, "appdata-prefix-validator", "exec"), {"__name__": "__main__"})
PY
}

validate_screenshots() {
  python3 - \
    "$M50_PPM" "$M51_PPM" "$M52_PPM" "$M54_PPM" \
    "$EXPECTED_M50_SHA256" "$EXPECTED_M51_SHA256" \
    "$EXPECTED_M52_SHA256" "$EXPECTED_M54_SHA256" <<'PY'
import hashlib
import sys

paths = sys.argv[1:5]
expected = sys.argv[5:9]

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
        raise SystemExit(f"{path}: incorrect pixel payload")
    with open(path, "rb") as source:
        digest = hashlib.sha256(source.read()).hexdigest()
    return pixels, digest

def exact_damage(before, after, x0, y0, width, height, label):
    changed = set()
    for pixel in range(320 * 480):
        offset = pixel * 3
        if before[offset:offset + 3] != after[offset:offset + 3]:
            changed.add((pixel % 320, pixel // 320))
    wanted = {(x, y) for y in range(y0, y0 + height) for x in range(x0, x0 + width)}
    if changed != wanted:
        raise SystemExit(f"{label}: changed={len(changed)}, expected={len(wanted)}")

frames = [read_ppm(path) for path in paths]
digests = [digest for _, digest in frames]
if digests != expected:
    raise SystemExit(f"M50/M51/M52/M54 hashes changed: {digests!r}, expected {expected!r}")
exact_damage(frames[0][0], frames[1][0], 64, 80, 40, 32, "M50-to-M51")
exact_damage(frames[1][0], frames[2][0], 160, 216, 40, 32, "M51-to-M52")
exact_damage(frames[2][0], frames[3][0], 0, 0, 0, 0, "M52-to-M54")
print(*digests)
PY
}

validate_storage_and_appdata_evidence() {
  local log_file="$1"
  local boot_number="$2"
  local phase="$3"
  local version="$4"
  local boot_generation="$5"
  local committed_generation="$6"
  local formatted="$7"
  local checkpoint_slot="$8"
  local bank="$9"
  local valid_snapshots="${10}"
  local rejected_snapshots="${11}"
  local boot_entries="${12}"
  local boot_live_bytes="${13}"
  local submissions="${14}"
  local mutations="${15}"
  local reads="${16}"
  local lists="${17}"
  local expected_terminal="${18}"
  local disk_writes="${19}"
  local disk_flushes="${20}"
  local data_initial=$((boot_number - 1))
  local data_committed=$boot_number
  local data_initial_slot=$((data_initial % 2))
  local data_committed_slot=$((data_committed % 2))
  local data_valid=2
  local data_rejected=0
  if [[ "$boot_number" == "1" ]]; then
    data_valid=1
    data_rejected=1
  fi
  local data_marker="DATA_PERSIST_OK format=1 format_epoch_bound=1 partition_lba=64-127 slots=2 initial_generation=$data_initial committed_generation=$data_committed initial_slot=$data_initial_slot committed_slot=$data_committed_slot valid_slots=$data_valid rejected_slots=$data_rejected reads=5 writes=1 flushes=1 write_completion=1 flush_completion=1 readback_verified=1 old_slot_preserved=1 system_write_rejected=1 out_of_data_rejected=1 rejected_request_unchanged=1 raw_sector_write=1 filesystem_write=0 crash_consistency=0 qemu_reboot_proof=0"

  python3 - "$log_file" "$phase" "$version" "$boot_generation" \
    "$committed_generation" "$formatted" "$checkpoint_slot" "$bank" \
    "$valid_snapshots" "$rejected_snapshots" "$boot_entries" \
    "$boot_live_bytes" "$submissions" "$mutations" "$reads" "$lists" \
    "$expected_terminal" "$disk_writes" "$disk_flushes" \
    "$AUTHORITY_MARKER" "$BOOT_MARKER" "$data_marker" \
    "$APP_DATA_ASYNC_RECOVERY_MODE" <<'PY'
import re
import sys

(log_path, phase, version, boot_generation, committed_generation, formatted,
 checkpoint_slot, bank, valid_snapshots, rejected_snapshots, boot_entries,
 boot_live_bytes, submissions, mutations, reads, lists, expected_terminal,
 disk_writes, disk_flushes, authority, boot, data_marker, async_mode) = sys.argv[1:]
with open(log_path, encoding="utf-8") as source:
    lines = [line.rstrip("\n") for line in source]

def fields(prefix, ordered):
    found = [line for line in lines if line.split(" ", 1)[0] == prefix]
    if len(found) != 1:
        raise SystemExit(f"expected exactly one {prefix}, observed {found!r}")
    tokens = found[0].split(" ")
    if "" in tokens:
        raise SystemExit(f"{prefix} contains non-canonical spacing")
    values = {}
    keys = []
    for token in tokens[1:]:
        if token.count("=") != 1:
            raise SystemExit(f"{prefix} has malformed token {token!r}")
        key, value = token.split("=", 1)
        if not key or not value or key in values:
            raise SystemExit(f"{prefix} has invalid field {token!r}")
        values[key] = value
        keys.append(key)
    if keys != ordered:
        raise SystemExit(f"{prefix} schema {keys!r}, expected {ordered!r}")
    return found[0], values

required_once = (
    "FDT_VIRTIO_OK", "VIRTIO_IRQ_OK", "VIRTIO_BLK_OK", "BLOCK_IRQ_OK",
    "BLOCK_LAYER_OK", "GPT_OK", "DATA_GPT_OK", "FAT16_OK", "VFS_OK",
    "DATA_PERSIST_OK", "APPDATA_GPT_OK", "APPDATA_MOUNT_OK", "STORAGE_LIMITS",
    "APPDATA_AUTHORITY_OK", "APPDATA_RUNTIME_OK",
)
if async_mode == "1":
    required_once += ("APPDATA_ASYNC_RECOVERY_OK",)
for prefix in required_once:
    if sum(line.startswith(prefix + " ") for line in lines) != 1:
        raise SystemExit(f"{prefix} is absent or duplicated")
if lines.count(authority) != 1 or lines.count(boot) != 1:
    raise SystemExit("M54 authority or boot marker is not exact and unique")
if lines.count(data_marker) != 1:
    raise SystemExit(f"BNDROID_DATA persistence transition changed: {data_marker!r}")
if any(line in {
    "BOOT_OK: M53 post-recovery lifecycle focus convergence verified",
    "BOOT_OK: M54 capability-scoped crash-safe AppData runtime verified",
} for line in lines if line != boot):
    raise SystemExit("retired AppData boot marker leaked")
if [line for line in lines if line.startswith("BOOT_OK:")] != [boot]:
    raise SystemExit("the selected AppData profile does not own the sole BOOT_OK marker")

gpt_line = (
    "APPDATA_GPT_OK partition_index=2 partition_lba=128-2047 "
    "partition_sectors=1920 name=BNDROID_APPDATA type=private epoch_bound=1 "
    "data_overlap=0 system_overlap=0 initial_zero=" + formatted
)
if lines.count(gpt_line) != 1:
    raise SystemExit(f"AppData GPT evidence changed: expected {gpt_line!r}")

_, mount = fields("APPDATA_MOUNT_OK", [
    "format", "formatted", "generation", "checkpoint_slot", "bank",
    "valid_snapshots", "rejected_snapshots", "entries", "live_bytes",
    "reads", "writes", "flushes", "full_readback", "fail_closed",
])
mount_expected = {
    "format": "1", "formatted": formatted, "generation": boot_generation,
    "checkpoint_slot": checkpoint_slot, "bank": bank,
    "valid_snapshots": valid_snapshots, "rejected_snapshots": rejected_snapshots,
    "entries": boot_entries, "live_bytes": boot_live_bytes,
    "full_readback": "1", "fail_closed": "1",
}
for key, expected in mount_expected.items():
    if mount[key] != expected:
        raise SystemExit(f"APPDATA_MOUNT_OK {key}={mount[key]!r}, expected {expected!r}")
for key in ("reads", "writes", "flushes"):
    if not re.fullmatch(r"[0-9]+", mount[key]):
        raise SystemExit(f"APPDATA_MOUNT_OK {key} is not canonical decimal")
if int(mount["reads"]) == 0:
    raise SystemExit("AppData mount did not read the durable volume")
if formatted == "0" and (mount["writes"] != "0" or mount["flushes"] != "0"):
    raise SystemExit("a non-formatting mount mutated AppData")
if formatted == "1" and (int(mount["writes"]) == 0 or int(mount["flushes"]) == 0):
    raise SystemExit("virgin AppData format did not publish durable writes and flushes")

runtime_keys = [
    "abi", "phase", "version", "boot_generation", "committed_generation",
    "entries", "files", "directories", "submissions", "completions",
    "retrievals", "mutations", "reads", "lists", "conflicts",
    "expected_terminal", "disk_reads", "disk_writes", "disk_flushes",
    "old_or_new", "full_readback", "resident", "errors",
]
if async_mode == "1":
    runtime_keys.append("async_recovery")
runtime_line, runtime = fields("APPDATA_RUNTIME_OK", runtime_keys)
runtime_expected = {
    "abi": "24", "phase": phase, "version": version,
    "boot_generation": boot_generation, "committed_generation": committed_generation,
    "entries": "2", "files": "1", "directories": "1",
    "submissions": submissions, "completions": submissions,
    "retrievals": submissions, "mutations": mutations, "reads": reads,
    "lists": lists, "conflicts": "1", "expected_terminal": expected_terminal,
    "disk_writes": disk_writes, "disk_flushes": disk_flushes,
    "old_or_new": "1", "full_readback": "1", "resident": "1", "errors": "0",
}
if async_mode == "1":
    runtime_expected["async_recovery"] = "1"
for key, expected in runtime_expected.items():
    if runtime[key] != expected:
        raise SystemExit(f"APPDATA_RUNTIME_OK {key}={runtime[key]!r}, expected {expected!r}")
if not re.fullmatch(r"[1-9][0-9]*", runtime["disk_reads"]):
    raise SystemExit("AppData runtime did not publish positive durable read evidence")

limits = (
    "STORAGE_LIMITS writes=1 partitions=3 filesystems=2 vfs=1 persistence=2 "
    "flush=1 readback=1 el0_storage=1 catalog_files=2 catalog_bytes=68 "
    "runtime_disk_io=1 mapped=0 shared_memory=0 filesystem_write=1 "
    "crash_consistency=1 core_powercut_points=292 appdata_runtime=kernel-monitor "
    "general_runtime=0"
)
if lines.count(limits) != 1:
    raise SystemExit("M54 storage limits are absent, duplicated, or overstated")

post = next(i for i, line in enumerate(lines) if line.startswith("POST_RECOVERY_LIFECYCLE_FOCUS_OK "))
authority_pos = lines.index(authority)
runtime_pos = lines.index(runtime_line)
boot_pos = lines.index(boot)
if async_mode == "1":
    recovery_pos = next(
        i for i, line in enumerate(lines) if line.startswith("APPDATA_ASYNC_RECOVERY_OK ")
    )
    ordered = post < authority_pos < recovery_pos < runtime_pos < boot_pos
else:
    ordered = post < authority_pos < runtime_pos < boot_pos
if not ordered:
    raise SystemExit("M53 prefix, AppData authority/recovery/runtime, and boot are out of order")
PY
}

validate_async_recovery_evidence() {
  local log_file="$1"
  if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" != "1" ]]; then
    return
  fi
  if ! bash "$M59_CHECKER" --validate-log "$log_file"; then
    show_failure
    echo "M59 AppData evidence did not match its exact cooperative recovery schema." >&2
    exit 1
  fi
}

validate_disk_boundaries() {
  local image="$1"
  local data_generation="$2"
  python3 - "$CANONICAL_IMAGE" "$image" "$SCRIPT_DIR" "$data_generation" "$MILESTONE" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[3])
import build_storage_image as fixture

base = Path(sys.argv[1]).read_bytes()
runtime = Path(sys.argv[2]).read_bytes()
generation = int(sys.argv[4])
milestone = sys.argv[5]
if len(base) != fixture.IMAGE_BYTES or len(runtime) != fixture.IMAGE_BYTES:
    raise SystemExit(f"{milestone} persistent image geometry changed")

changed_lbas = []
for lba in range(fixture.SECTOR_COUNT):
    start = lba * fixture.SECTOR_BYTES
    end = start + fixture.SECTOR_BYTES
    if base[start:end] != runtime[start:end]:
        changed_lbas.append(lba)
        if not (
            fixture.DATA_PARTITION_FIRST_LBA <= lba <= fixture.DATA_PARTITION_LAST_LBA
            or fixture.APPDATA_PARTITION_FIRST_LBA <= lba <= fixture.APPDATA_PARTITION_LAST_LBA
        ):
            raise SystemExit(f"persistent boot wrote forbidden LBA {lba}")

for first, last, label in (
    (0, 0, "protective MBR"),
    (1, 33, "primary GPT"),
    (34, 63, "pre-data reserved space"),
    (fixture.SYSTEM_PARTITION_FIRST_LBA, fixture.SYSTEM_PARTITION_LAST_LBA, "system/FAT"),
    (fixture.BACKUP_ENTRIES_LBA, fixture.BACKUP_HEADER_LBA, "backup GPT"),
):
    start = first * fixture.SECTOR_BYTES
    end = (last + 1) * fixture.SECTOR_BYTES
    if base[start:end] != runtime[start:end]:
        raise SystemExit(f"{label} changed")

def sector(image, lba):
    start = lba * fixture.SECTOR_BYTES
    return image[start:start + fixture.SECTOR_BYTES]

if sector(runtime, fixture.DATA_PARTITION_FIRST_LBA) != fixture.build_data_superblock():
    raise SystemExit("the immutable BNDROID_DATA superblock changed")
current_slot = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[generation % 2]
previous_generation = generation - 1
previous_slot = fixture.DATA_PARTITION_FIRST_LBA + fixture.DATA_SLOT_RELATIVE_LBAS[previous_generation % 2]
if sector(runtime, current_slot) != fixture.build_data_record(
    generation, fixture.build_boot_state(generation)
):
    raise SystemExit("BNDROID_DATA current committed record changed")
if sector(runtime, previous_slot) != fixture.build_data_record(
    previous_generation, fixture.build_boot_state(previous_generation)
):
    raise SystemExit("BNDROID_DATA did not preserve the previous committed record")
data_end = (fixture.DATA_PARTITION_LAST_LBA + 1) * fixture.SECTOR_BYTES
unused = (fixture.DATA_PARTITION_FIRST_LBA + 3) * fixture.SECTOR_BYTES
if any(runtime[unused:data_end]):
    raise SystemExit("an unused BNDROID_DATA sector changed")
if not any(
    fixture.APPDATA_PARTITION_FIRST_LBA <= lba <= fixture.APPDATA_PARTITION_LAST_LBA
    for lba in changed_lbas
):
    raise SystemExit("AppData runtime did not durably change its partition")
print(
    f"{milestone}_DISK_BOUNDARY_OK data_generation={generation} "
    f"changed_lbas={len(changed_lbas)} allowed=data64-127/appdata128-2047 "
    "mbr=unchanged gpt=unchanged system=unchanged forbidden_writes=0"
)
PY
}

run_boot() {
  local label="$1"
  local image="$2"
  local data_boot_number="$3"
  local phase="$4"
  local version="$5"
  local boot_generation="$6"
  local committed_generation="$7"
  local formatted="$8"
  local checkpoint_slot="$9"
  local bank="${10}"
  local valid_snapshots="${11}"
  local rejected_snapshots="${12}"
  local boot_entries="${13}"
  local boot_live_bytes="${14}"
  local submissions="${15}"
  local mutations="${16}"
  local reads="${17}"
  local lists="${18}"
  local expected_terminal="${19}"
  local disk_writes="${20}"
  local disk_flushes="${21}"
  local boot_dir="$TMP_DIR/$label"
  mkdir -p "$boot_dir"
  SERIAL_LOG="$boot_dir/serial.log"
  NORMALIZED_LOG="$boot_dir/serial.normalized.log"
  QEMU_LOG="$boot_dir/qemu.log"
  QMP_SOCKET="$boot_dir/qmp.sock"
  M50_PPM="$boot_dir/m50-presented.ppm"
  M51_PPM="$boot_dir/m51-presented.ppm"
  M52_PPM="$boot_dir/m52-presented.ppm"
  M54_PPM="$boot_dir/m54-appdata.ppm"
  QMP_INPUT_SEND_EVENTS=0
  : >"$SERIAL_LOG"
  : >"$NORMALIZED_LOG"
  : >"$QEMU_LOG"

  build_storage_qemu_args "$image" modern writable-persistent
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

  wait_for_prefix_once "$M49_ARMED" "$label M49 dependency-armed marker"
  qmp_action m49-pointer-down
  wait_for_prefix_once "$M49_SURFACE_GAP" "$label M49 SurfaceServer gap"
  qmp_action m49-pointer-up
  wait_for_prefix_once "$M49_SURFACE_RECOVERED" "$label M49 SurfaceServer recovery"
  wait_for_prefix_once "$M49_SURFACE_HEALTHY" "$label M49 SurfaceServer health"
  wait_for_prefix_once "$M49_WATCHDOG" "$label M49 InputServer watchdog"
  wait_for_prefix_once "$M49_DEGRADED" "$label M49 degraded marker"
  wait_for_prefix_once "$M49_INPUT_REBOUND" "$label M49 InputServer rebound"
  wait_for_prefix_once "$M49_INPUT_HEALTHY" "$label M49 InputServer health"

  wait_for_prefix_once "$POST_ARMED" "$label M50 armed marker"
  qmp_action invalid-down
  qmp_action touch-up
  wait_for_prefix_once "$POST_REJECTED" "$label M50 rejected contact"
  qmp_action app-down
  qmp_action touch-up
  wait_for_prefix_once "$POST_ROUTED" "$label M50 routed contact"
  wait_for_prefix_once "$POST_PRESENTED" "$label M50 presented frame"
  wait_for_prefix_once "$POST_HEALTHY" "$label M50 health"
  wait_for_prefix_once "$POST_SUCCESS" "$label M50 terminal"

  wait_for_prefix_once "$FOCUS_ARMED" "$label M51 armed marker"
  qmp_action screendump "$M50_PPM"
  qmp_action launcher-down
  wait_for_prefix_once "$LAUNCHER_CAPTURED" "$label M51 launcher capture"
  qmp_action touch-up
  wait_for_prefix_once "$LAUNCHER_PRESENTED" "$label M51 launcher frame"
  qmp_action screendump "$M51_PPM"
  wait_for_prefix_once "$FOCUS_SUCCESS" "$label M51 terminal"

  wait_for_prefix_once "$ROUNDTRIP_ARMED" "$label M52 armed marker"
  qmp_action app-down
  wait_for_prefix_once "$APP_CAPTURED" "$label M52 app capture"
  qmp_action touch-up
  wait_for_prefix_once "$APP_PRESENTED" "$label M52 app frame"
  qmp_action screendump "$M52_PPM"
  wait_for_prefix_once "$ROUNDTRIP_SUCCESS" "$label M52 terminal"
  if [[ "$QMP_INPUT_SEND_EVENTS" != "11" ]]; then
    show_failure
    echo "$label did not use exactly eleven M49--M52 input-send-event commands." >&2
    exit 1
  fi

  wait_for_prefix_once "$LIFECYCLE_SESSION_READY" "$label M53 session-ready marker"
  wait_for_prefix_once "$LIFECYCLE_LAUNCHER_SYNCED" "$label M53 launcher-sync marker"
  wait_for_prefix_once "$LIFECYCLE_APP_SYNCED" "$label M53 app-sync marker"
  wait_for_prefix_once "$LIFECYCLE_SUCCESS" "$label M53 lifecycle terminal"
  wait_for_exact_once "$AUTHORITY_MARKER" "$label $MILESTONE authority marker"
  if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
    wait_for_prefix_once "APPDATA_ASYNC_RECOVERY_OK" "$label M59 asynchronous recovery marker"
  fi
  wait_for_prefix_once "APPDATA_RUNTIME_OK" "$label $MILESTONE AppData runtime marker"
  wait_for_exact_once "$BOOT_MARKER" "$label $MILESTONE boot marker"
  qmp_action screendump "$M54_PPM"
  if [[ "$QMP_INPUT_SEND_EVENTS" != "11" ]]; then
    show_failure
    echo "$label injected input after the immutable M52 prefix." >&2
    exit 1
  fi

  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  validate_m53_prefix_evidence "$NORMALIZED_LOG"
  validate_storage_and_appdata_evidence \
    "$NORMALIZED_LOG" "$data_boot_number" "$phase" "$version" \
    "$boot_generation" "$committed_generation" "$formatted" \
    "$checkpoint_slot" "$bank" "$valid_snapshots" "$rejected_snapshots" \
    "$boot_entries" "$boot_live_bytes" "$submissions" "$mutations" \
    "$reads" "$lists" "$expected_terminal" "$disk_writes" "$disk_flushes"
  validate_async_recovery_evidence "$NORMALIZED_LOG"
  local screenshot_hashes
  screenshot_hashes="$(validate_screenshots)"

  qmp_action quit
  local wait_pid="$QEMU_PID"
  local timeout_flag="$boot_dir/qemu-quit.timeout"
  (
    sleep "$BOOT_TIMEOUT_SECONDS"
    if kill -0 "$wait_pid" 2>/dev/null; then
      : >"$timeout_flag"
      kill "$wait_pid" 2>/dev/null || true
    fi
  ) &
  local watchdog_pid=$!
  set +e
  wait "$wait_pid"
  local qemu_status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true
  set -e
  QEMU_PID=""
  if [[ -e "$timeout_flag" || "$qemu_status" != "0" ]]; then
    show_failure
    echo "$label QEMU did not exit cleanly after QMP quit (status $qemu_status)." >&2
    exit 1
  fi

  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
  reject_failure
  validate_m53_prefix_evidence "$NORMALIZED_LOG"
  validate_storage_and_appdata_evidence \
    "$NORMALIZED_LOG" "$data_boot_number" "$phase" "$version" \
    "$boot_generation" "$committed_generation" "$formatted" \
    "$checkpoint_slot" "$bank" "$valid_snapshots" "$rejected_snapshots" \
    "$boot_entries" "$boot_live_bytes" "$submissions" "$mutations" \
    "$reads" "$lists" "$expected_terminal" "$disk_writes" "$disk_flushes"
  validate_async_recovery_evidence "$NORMALIZED_LOG"
  validate_disk_boundaries "$image" "$data_boot_number"
  echo "APPDATA_BOOT_QMP_OK boot=$label phase=$phase version=$version boot_generation=$boot_generation committed_generation=$committed_generation qmp_inputs=11 screenshots=$screenshot_hashes"
}

if [[ "$APP_DATA_ASYNC_RECOVERY_MODE" == "1" ]]; then
  # M59 is a fresh-boot child of M54. The one lost read completion adds one
  # terminal service error and one exact read-only resubmission; durable
  # mutation, screenshot, authority, and disk-boundary ledgers stay unchanged.
  run_boot boot1 "$RUNTIME_IMAGE" 1 created 1 0 2 1 0 0 1 1 0 0 11 2 1 3 4 580 4

  mkdir -p "$OUTPUT_DIR"
  cp "$TMP_DIR/boot1/serial.normalized.log" "$OUTPUT_DIR/app-data-async-recovery-runtime.log"
  cp "$TMP_DIR/boot1/m50-presented.ppm" "$OUTPUT_DIR/m50-presented.ppm"
  cp "$TMP_DIR/boot1/m51-presented.ppm" "$OUTPUT_DIR/m51-presented.ppm"
  cp "$TMP_DIR/boot1/m52-presented.ppm" "$OUTPUT_DIR/m52-presented.ppm"
  cp "$TMP_DIR/boot1/m54-appdata.ppm" "$OUTPUT_DIR/m59-appdata.ppm"

  echo "APPDATA_ASYNC_RECOVERY_QMP_OK abi=24 boots=1 phase=created-v1 generations=0-2 persistent_image=1 authority=unique prefix=m49-m53 qmp_inputs=11 screenshots=frozen disk_scope=data64-127/appdata128-2047 recoveries=1 read_only_retries=1 blind_mutation_retries=0 powercut_claim=0 boot='M59 cooperative kernel-monitor AppData recovery verified'"
  exit 0
fi

# Boot 1 formats the virgin AppData volume and commits directory generation 1
# followed by the v1 file snapshot at generation 2.
run_boot boot1 "$RUNTIME_IMAGE" 1 created 1 0 2 1 0 0 1 1 0 0 10 2 1 3 3 580 4
cp "$RUNTIME_IMAGE" "$AFTER_BOOT1_IMAGE"

# Boot 2 mounts generation 2 and atomically upgrades v1 to v2/generation 3.
run_boot boot2 "$RUNTIME_IMAGE" 2 upgraded 2 2 3 0 0 0 2 0 2 24 12 1 2 4 4 290 2
cp "$RUNTIME_IMAGE" "$AFTER_BOOT2_IMAGE"
cp "$RUNTIME_IMAGE" "$RECOVERY_IMAGE"

# Corrupt only the newest checkpoint in the copied boot-2 image. This is a
# deterministic corrupt-checkpoint recovery proof, not a simulated power cut.
python3 - "$RECOVERY_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[2])
import build_storage_image as fixture

path = Path(sys.argv[1])
image = bytearray(path.read_bytes())
newest_checkpoint_lba = fixture.APPDATA_PARTITION_FIRST_LBA + 2
offset = newest_checkpoint_lba * fixture.SECTOR_BYTES + 32
image[offset] ^= 1
path.write_bytes(image)
PY

# Boot 3 proves the v2 state is idempotent and performs no AppData write.
run_boot boot3 "$RUNTIME_IMAGE" 3 stable 2 3 3 0 1 1 2 0 2 24 11 0 2 4 4 0 0

python3 - "$AFTER_BOOT2_IMAGE" "$RUNTIME_IMAGE" "$SCRIPT_DIR" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[3])
import build_storage_image as fixture

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
start = fixture.APPDATA_PARTITION_FIRST_LBA * fixture.SECTOR_BYTES
end = (fixture.APPDATA_PARTITION_LAST_LBA + 1) * fixture.SECTOR_BYTES
if before[start:end] != after[start:end]:
    raise SystemExit("stable boot changed AppData despite mutations=0")
print("APPDATA_STABLE_DISK_OK generation=3 appdata_writes=0 partition_unchanged=1")
PY

# The recovery copy rejects corrupt checkpoint 1, mounts intact generation 2
# (v1), and then performs the same atomic v1-to-v2 upgrade. This demonstrates
# old-complete fallback with no mixed snapshot.
run_boot recovery "$RECOVERY_IMAGE" 3 upgraded 2 2 3 0 0 0 1 1 2 24 12 1 2 4 4 290 2

mkdir -p "$OUTPUT_DIR"
cp "$TMP_DIR/boot1/m50-presented.ppm" "$OUTPUT_DIR/m50-presented.ppm"
cp "$TMP_DIR/boot1/m51-presented.ppm" "$OUTPUT_DIR/m51-presented.ppm"
cp "$TMP_DIR/boot1/m52-presented.ppm" "$OUTPUT_DIR/m52-presented.ppm"
cp "$TMP_DIR/boot1/m54-appdata.ppm" "$OUTPUT_DIR/m54-appdata.ppm"

echo "APPDATA_RUNTIME_QMP_OK abi=24 boots=3 phases=created-v1/upgraded-v2/stable-v2 generations=0-2-3 persistent_image=1 authority=unique prefix=m49-m53 qmp_inputs=11/11/11 screenshots=frozen disk_scope=data64-127/appdata128-2047 recovery=corrupt-newest-checkpoint/fallback-gen2/reupgrade-gen3 old_or_new=1 mixed_snapshot=0 powercut_claim=0 boot='M54 capability-scoped crash-safe AppData runtime verified'"
