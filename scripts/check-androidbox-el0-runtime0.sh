#!/usr/bin/env bash
set -euo pipefail

# Offline AndroidBox EL0Runtime-0 gate.
#
# The existing ABI-44 local-APK gate remains the admission/install oracle. This
# gate consumes its durable generation-1 image, boots a separately built ABI-45
# kernel without an APK source, and proves that two compatible-Activity entries
# execute and render from the App process through one-shot read-only image
# claims. It intentionally proves only the bounded Resources-1 profile.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-el0-runtime0.sh --apk /absolute/path.apk [--base-artifact /absolute/path]' \
    '' \
    'By default the ABI-44 local-APK gate first creates and verifies a durable' \
    'generation-1 package image. --base-artifact may reuse the artifact_dir of' \
    'an already successful run when its summary APK digest matches --apk.'
}

APK_PATH=""
BASE_ARTIFACT=""
while (($#)); do
  case "$1" in
    --apk)
      if [[ -n "$APK_PATH" || $# -lt 2 ]]; then
        usage >&2
        exit 2
      fi
      APK_PATH="$2"
      shift 2
      ;;
    --base-artifact)
      if [[ -n "$BASE_ARTIFACT" || $# -lt 2 ]]; then
        usage >&2
        exit 2
      fi
      BASE_ARTIFACT="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$APK_PATH" ]] || {
  usage >&2
  exit 2
}
case "$APK_PATH" in
  /*) ;;
  *)
    echo "--apk must be absolute: $APK_PATH" >&2
    exit 2
    ;;
esac
[[ -f "$APK_PATH" ]] || {
  echo "--apk must name one regular file: $APK_PATH" >&2
  exit 2
}
if [[ -n "$BASE_ARTIFACT" ]]; then
  case "$BASE_ARTIFACT" in
    /*) ;;
    *)
      echo "--base-artifact must be absolute: $BASE_ARTIFACT" >&2
      exit 2
      ;;
  esac
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed chmod; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the EL0Runtime-0 gate." >&2
    exit 1
  }
done

BASE_GATE="$SCRIPT_DIR/check-androidbox-local-apk.sh"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in "$BASE_GATE" "$BUILD_KERNEL" "$QMP_HELPER"; do
  [[ -x "$helper" ]] || {
    echo "Required helper is not executable: $helper" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-90}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

mkdir -p "$WORKSPACE_ROOT/target/androidbox-el0-runtime0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-el0-runtime0/check.XXXXXX"
)"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

BASE_GATE_LOG="$ARTIFACT_DIR/abi44-prerequisite.log"
BUILD_LOG="$ARTIFACT_DIR/build.log"
SERIAL_LOG="$ARTIFACT_DIR/recovery.serial.log"
NORMALIZED_LOG="$ARTIFACT_DIR/recovery.serial.normalized.log"
QEMU_LOG="$ARTIFACT_DIR/recovery.qemu.log"
QMP_SOCKET="$ARTIFACT_DIR/recovery.qmp.sock"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
ACTIVITY_ONE="$ARTIFACT_DIR/activity-sequence1.ppm"
ACTIVITY_TWO="$ARTIFACT_DIR/activity-sequence2.ppm"
OVERVIEW="$ARTIFACT_DIR/compatible-overview.ppm"
EVIDENCE="$ARTIFACT_DIR/evidence.txt"
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-el0-runtime0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
QEMU_PID=""

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill -TERM "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

normalize_log() {
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
}

show_failure() {
  if [[ -f "$NORMALIZED_LOG" ]]; then
    tail -n 280 "$NORMALIZED_LOG" >&2
  elif [[ -f "$SERIAL_LOG" ]]; then
    tr -d '\r' <"$SERIAL_LOG" | tail -n 280 >&2
  fi
  if [[ -f "$QEMU_LOG" ]]; then
    tail -n 120 "$QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "EL0Runtime-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_panic_or_fatal() {
  normalize_log
  if grep -Eqi \
    'fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)' \
    "$NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fatal, or userspace failure marker."
  fi
}

wait_for_pattern() {
  local pattern="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
    if grep -Eq "$pattern" "$NORMALIZED_LOG"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

log_count() {
  local pattern="$1"
  normalize_log
  grep -Ec "$pattern" "$NORMALIZED_LOG" || true
}

wait_for_count() {
  local pattern="$1"
  local minimum="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
    local observed
    observed="$(grep -Ec "$pattern" "$NORMALIZED_LOG" || true)"
    if ((observed >= minimum)); then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

qmp() {
  if ! "$QMP_HELPER" "$QMP_SOCKET" "$@"; then
    fail_gate "QMP action failed: $*"
  fi
}

commit_count() {
  local producer_pid="$1"
  log_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${producer_pid} "
}

wait_for_new_commit() {
  local producer_pid="$1"
  local previous="$2"
  local description="$3"
  wait_for_count \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${producer_pid} " \
    "$((previous + 1))" \
    "$description"
}

mobile_screenshot_matches() {
  local kind="$1"
  local screenshot="$2"
  python3 - "$kind" "$screenshot" <<'PY'
from pathlib import Path
import sys

kind = sys.argv[1]
path = Path(sys.argv[2])
try:
    parts = path.read_bytes().split(b"\n", 3)
except OSError:
    raise SystemExit(1)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)

def pixel(x: int, y: int) -> tuple[int, int, int]:
    offset = (y * 720 + x) * 3
    return tuple(pixels[offset:offset + 3])

expected = {
    "drawer": {
        (360, 300): (13, 22, 41),
        (104, 630): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    },
    "activity": {
        (108, 268): (16, 118, 111),
        (180, 267): (24, 34, 56),
        (360, 300): (24, 34, 56),
        (100, 500): (32, 45, 73),
        (719, 1599): (0, 0, 0),
    },
    "overview": {
        (360, 300): (13, 22, 41),
        (360, 580): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    },
}.get(kind)
fallback_icon_colors = {
    (16, 118, 111),
    (11, 87, 208),
    (142, 36, 170),
    (0, 108, 76),
    (179, 38, 30),
    (122, 79, 1),
    (64, 81, 181),
}
def matches(point, color):
    actual = pixel(*point)
    if kind == "drawer" and point == (104, 630):
        return actual in fallback_icon_colors
    return actual == color
if expected is None or any(not matches(point, color) for point, color in expected.items()):
    raise SystemExit(1)
minimum_colors = 64 if kind == "activity" else 24
if len({pixels[index:index + 3] for index in range(0, len(pixels), 3)}) < minimum_colors:
    raise SystemExit(1)
PY
}

wait_for_screenshot() {
  local kind="$1"
  local screenshot="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  qmp move 700 1500
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
    qmp screenshot "$screenshot"
    if mobile_screenshot_matches "$kind" "$screenshot"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

summary_value() {
  local summary="$1"
  local name="$2"
  sed -n "s/^${name}=//p" "$summary"
}

APK_SHA256="$(shasum -a 256 "$APK_PATH" | awk '{print $1}')"
[[ "$APK_SHA256" =~ ^[0-9a-f]{64}$ ]] || {
  echo "Could not derive the APK SHA-256." >&2
  exit 1
}

if [[ -z "$BASE_ARTIFACT" ]]; then
  if ! "$BASE_GATE" --apk "$APK_PATH" >"$BASE_GATE_LOG" 2>&1; then
    tail -n 260 "$BASE_GATE_LOG" >&2
    echo "The prerequisite ABI-44 local-APK gate failed." >&2
    exit 1
  fi
  BASE_ARTIFACT="$(
    sed -n \
      's/^ANDROIDBOX_LOCAL_APK_QEMU_OK artifact_dir=\([^ ]*\) .*/\1/p' \
      "$BASE_GATE_LOG"
  )"
  [[ "$(grep -c '^ANDROIDBOX_LOCAL_APK_QEMU_OK ' "$BASE_GATE_LOG" || true)" == "1" \
    && -n "$BASE_ARTIFACT" ]] || {
    echo "Could not identify one successful ABI-44 prerequisite artifact." >&2
    exit 1
  }
else
  printf '%s\n' "reused_base_artifact=$BASE_ARTIFACT" >"$BASE_GATE_LOG"
fi

BASE_SUMMARY="$BASE_ARTIFACT/summary.txt"
BASE_DISK="$BASE_ARTIFACT/packages.raw"
[[ -f "$BASE_SUMMARY" && -f "$BASE_DISK" ]] || {
  echo "ABI-44 prerequisite lacks summary.txt or packages.raw: $BASE_ARTIFACT" >&2
  exit 1
}
[[ "$(summary_value "$BASE_SUMMARY" apk_sha256)" == "$APK_SHA256" ]] || {
  echo "Reused ABI-44 artifact does not match the explicit APK digest." >&2
  exit 1
}
[[ "$(summary_value "$BASE_SUMMARY" install_generation)" == "1" \
  && "$(summary_value "$BASE_SUMMARY" recovery_disk_unchanged)" == "1" \
  && "$(summary_value "$BASE_SUMMARY" framework_subset)" == "Resources-1" ]] || {
  echo "ABI-44 prerequisite summary is not the required durable Resources-1 state." >&2
  exit 1
}

APK_BYTES="$(summary_value "$BASE_SUMMARY" apk_bytes)"
PACKAGE="$(summary_value "$BASE_SUMMARY" package)"
ACTIVITY="$(summary_value "$BASE_SUMMARY" launcher_activity)"
VERSION_CODE="$(summary_value "$BASE_SUMMARY" version_code)"
[[ "$APK_BYTES" =~ ^[1-9][0-9]*$ \
  && -n "$PACKAGE" \
  && -n "$ACTIVITY" \
  && "$VERSION_CODE" =~ ^[1-9][0-9]*$ ]] || {
  echo "ABI-44 prerequisite metadata is incomplete." >&2
  exit 1
}

cp "$BASE_DISK" "$DISK_IMAGE"
DISK_BEFORE_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0 \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 260 "$BUILD_LOG" >&2
  echo "ABI-45 EL0Runtime-0 kernel/userspace build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || {
  echo "ABI-45 build did not produce the expected kernel image." >&2
  exit 1
}

: >"$SERIAL_LOG"
: >"$NORMALIZED_LOG"
: >"$QEMU_LOG"
qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
  -nic none \
  -rtc base=2026-07-30T10:41:00,clock=vm \
  -serial "file:$SERIAL_LOG" \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -no-reboot -kernel "$KERNEL_IMAGE" -device ramfb \
  -global virtio-mmio.force-legacy=false \
  -drive "if=none,file=$DISK_IMAGE,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
  -device virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  -name "Bndroid AndroidBox EL0Runtime-0 Gate" \
  >"$QEMU_LOG" 2>&1 &
QEMU_PID=$!

wait_for_pattern \
  '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=45 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 .* ui_client_control_version=8 ui_server_event_version=7 .* network=disabled .* real_phone_claim=0$' \
  "the ABI-45/BUC8/BUE7 phone preview"
wait_for_pattern \
  '^ANDROIDBOX_EL0_RUNTIME0_PROFILE_OK format=1 abi=45 .* relaunch_syscall=60 .* package_image_claim_syscall=61 .* ui_activity_raster_owner=app-generation launcher_activity_raster=0 .* network=disabled .* real_phone_claim=0$' \
  "the ABI-45 EL0Runtime-0 profile"
wait_for_pattern \
  '^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=[1-9][0-9]* ' \
  "the initial Launcher frame"

normalize_log
LAUNCHER_PID="$(
  awk '/^USER_SURFACE_BUFFER_COMMIT_OK / {
    for (field = 1; field <= NF; field++) {
      if ($field ~ /^producer_pid=/) {
        sub(/^producer_pid=/, "", $field)
        print $field
        exit
      }
    }
  }' "$NORMALIZED_LOG"
)"
[[ "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ ]] || {
  fail_gate "Could not derive the Launcher producer PID."
}

# Unlock, settle All apps, and request the first generation-bound entry.
qmp drag 360 1390 360 620
wait_for_pattern \
  "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${LAUNCHER_PID} .* action=unlock app=none request_id=1 " \
  "the Launcher unlock request"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' \
  "the unlocked Home state"
qmp tap 700 900
qmp drag 360 1280 360 520
wait_for_screenshot drawer "$ARTIFACT_DIR/drawer.ppm" "the installed All apps row"

qmp tap 106 674
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} app_owner=[1-9][0-9]* compatible_session_id=1 request_sequence=1 generation=1 .* apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=1 storage_authority_granted=0$" \
  "syscall 60 sequence 1 with compatible session 1"
normalize_log
APP_PID="$(
  sed -n \
    's/^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK .* app_owner=\([1-9][0-9]*\) compatible_session_id=1 request_sequence=1 .*/\1/p' \
    "$NORMALIZED_LOG"
)"
[[ "$APP_PID" =~ ^[1-9][0-9]*$ && "$APP_PID" != "$LAUNCHER_PID" ]] || {
  fail_gate "Could not derive one App-only image-claim owner distinct from Launcher."
}
wait_for_pattern \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${APP_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} .* rights=READ .* write=0 execute=0 .* storage_authority_granted=0$" \
  "App-only syscall 61 claim 1"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "compatible foreground 1"
APP_COMMITS="$(commit_count "$APP_PID")"
if ((APP_COMMITS == 0)); then
  wait_for_new_commit "$APP_PID" 0 "the first App-produced Activity frame"
fi
wait_for_screenshot activity "$ACTIVITY_ONE" "the first App-produced Activity raster"

# System Home hands rendering back to Launcher while retaining only identity.
LAUNCHER_BEFORE="$(commit_count "$LAUNCHER_PID")"
qmp tap 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "Home retaining the compatible identity"
wait_for_new_commit "$LAUNCHER_PID" "$LAUNCHER_BEFORE" "a Launcher-produced Home frame"

# SurfaceServer owns the navigation capture; stable Overview is Launcher-owned.
LAUNCHER_BEFORE="$(commit_count "$LAUNCHER_PID")"
qmp touch-down 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "Overview gesture capture"
qmp touch-move 360 1330
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=240 .* compatible_session=1 package_generation=1 ' \
  "Overview finger-follow state"
qmp touch-up
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "stable compatible Overview"
wait_for_new_commit "$LAUNCHER_PID" "$LAUNCHER_BEFORE" "a Launcher-produced Overview frame"
wait_for_screenshot overview "$OVERVIEW" "the compatible identity-only Overview"

# Activating recent must repeat both syscalls and produce the raster from App.
APP_BEFORE="$(commit_count "$APP_PID")"
qmp tap 360 920
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} app_owner=${APP_PID} compatible_session_id=1 request_sequence=2 generation=1 .* apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=1 storage_authority_granted=0$" \
  "syscall 60 sequence 2 with compatible session 1"
wait_for_count \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${APP_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} .* rights=READ .* write=0 execute=0 .* storage_authority_granted=0$" \
  2 \
  "App-only syscall 61 claim 2"
wait_for_new_commit "$APP_PID" "$APP_BEFORE" "the second App-produced Activity frame"
wait_for_screenshot activity "$ACTIVITY_TWO" "the second App-produced Activity raster"

# App header Back asks only to relinquish focus. SurfaceServer performs the
# internal compatible finish, clears recent, and publishes Home from Launcher.
LAUNCHER_BEFORE="$(commit_count "$LAUNCHER_PID")"
qmp tap 56 120
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 .* recent_kind=none compatible_session=0 package_generation=0 ' \
  "Back clearing the compatible recent"
wait_for_pattern \
  '^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=finish-compatible status=accepted request_id=6 .* compatible_session=1 package_generation=1 reservation_origin=none$' \
  "the server-internal compatible finish completion"
wait_for_new_commit "$LAUNCHER_PID" "$LAUNCHER_BEFORE" "a Launcher-produced Home frame after Back"

kill -TERM "$QEMU_PID"
wait "$QEMU_PID" 2>/dev/null || true
QEMU_PID=""
reject_panic_or_fatal

DISK_AFTER_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$DISK_AFTER_SHA256" == "$DISK_BEFORE_SHA256" ]] || {
  fail_gate "EL0Runtime-0 launch/navigation changed the complete package disk."
}
cmp "$ACTIVITY_ONE" "$ACTIVITY_TWO" || {
  fail_gate "The two App-produced Activity screenshots are not byte-identical."
}
ACTIVITY_SHA256="$(shasum -a 256 "$ACTIVITY_ONE" | awk '{print $1}')"

python3 - \
  "$NORMALIZED_LOG" \
  "$EVIDENCE" \
  "$LAUNCHER_PID" \
  "$APP_PID" \
  "$APK_BYTES" \
  "$PACKAGE" \
  "$ACTIVITY" \
  "$VERSION_CODE" \
  "$ACTIVITY_SHA256" \
  "$DISK_BEFORE_SHA256" <<'PY'
from pathlib import Path
import re
import sys

log_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
launcher_pid, app_pid, apk_bytes = sys.argv[3:6]
package, activity, version_code = sys.argv[6:9]
activity_sha256, disk_sha256 = sys.argv[9:11]
lines = log_path.read_text(encoding="utf-8").splitlines()

def fields(line: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for item in line.split()[1:]:
        if "=" not in item:
            raise SystemExit(f"non-canonical marker field: {item!r}")
        name, value = item.split("=", 1)
        if name in result:
            raise SystemExit(f"duplicate marker field: {name}")
        result[name] = value
    return result

def exact(prefix: str) -> list[tuple[int, dict[str, str]]]:
    return [
        (index, fields(line))
        for index, line in enumerate(lines)
        if line.startswith(prefix)
    ]

preview = exact("MOBILE_UI_PREVIEW_OK ")
if len(preview) != 1:
    raise SystemExit("expected one mobile preview marker")
for name, expected in {
    "abi": "45",
    "width": "720",
    "height": "1600",
    "ui_client_control_version": "8",
    "ui_server_event_version": "7",
    "network": "disabled",
}.items():
    if preview[0][1].get(name) != expected:
        raise SystemExit(f"preview {name} mismatch")

profile = exact("ANDROIDBOX_EL0_RUNTIME0_PROFILE_OK ")
if len(profile) != 1:
    raise SystemExit("expected one EL0Runtime-0 profile marker")
for name, expected in {
    "abi": "45",
    "parent_profile": "androidbox-apk-install0",
    "relaunch_syscall": "60",
    "relaunch_session_arg": "x2-nonzero",
    "package_image_claim_syscall": "61",
    "package_image_claim_wire": "BNDACM01",
    "package_image_claim_owner": "app-generation",
    "package_image_grant": "one-shot",
    "package_image_handle_rights": "READ",
    "apk_bytes_exposed_to_launcher": "0",
    "ui_activity_raster_owner": "app-generation",
    "launcher_activity_raster": "0",
    "network": "disabled",
}.items():
    if profile[0][1].get(name) != expected:
        raise SystemExit(f"EL0Runtime-0 profile {name} mismatch")

collects = exact("ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ")
claims = exact("ANDROID_PACKAGE_IMAGE_CLAIM_OK ")
if len(collects) != 2 or len(claims) != 2:
    raise SystemExit("expected exactly two syscall-60 collections and two syscall-61 claims")
for sequence, ((collect_index, collect), (claim_index, claim)) in enumerate(
    zip(collects, claims), 1
):
    expected_collect = {
        "owner": launcher_pid,
        "app_owner": app_pid,
        "compatible_session_id": "1",
        "request_sequence": str(sequence),
        "generation": "1",
        "bytes": "640",
        "writes": "0",
        "flushes": "0",
        "authority_granted": "app-read-only-vmo",
        "apk_bytes_exposed_to_launcher": "0",
        "apk_bytes_exposed_to_app": "1",
        "storage_authority_granted": "0",
    }
    expected_claim = {
        "owner": app_pid,
        "compatible_session_id": "1",
        "generation": "1",
        "apk_length": apk_bytes,
        "rights": "READ",
        "duplicate": "0",
        "transfer": "0",
        "map": "0",
        "write": "0",
        "execute": "0",
        "wait": "0",
        "storage_authority_granted": "0",
    }
    if any(collect.get(name) != value for name, value in expected_collect.items()):
        raise SystemExit(f"syscall-60 collection {sequence} fields mismatch")
    if any(claim.get(name) != value for name, value in expected_claim.items()):
        raise SystemExit(f"syscall-61 claim {sequence} fields mismatch")
    if collect_index >= claim_index:
        raise SystemExit(f"syscall-61 claim {sequence} did not follow syscall 60")

if launcher_pid == app_pid:
    raise SystemExit("Launcher and App producer identities are equal")
commits = exact("USER_SURFACE_BUFFER_COMMIT_OK ")
app_commits = [(index, value) for index, value in commits if value.get("producer_pid") == app_pid]
launcher_commits = [
    (index, value) for index, value in commits if value.get("producer_pid") == launcher_pid
]
if len(app_commits) < 2 or not launcher_commits:
    raise SystemExit("missing App or Launcher frame producer evidence")
for claim_index, _claim in claims:
    if not any(index > claim_index for index, _value in app_commits):
        raise SystemExit("a syscall-61 claim has no subsequent App-produced frame")

states = exact("UI_SYSTEM_UI_CHANGED_OK ")
def state_index(mode: str, recent: str, after: int = -1) -> int:
    for index, value in states:
        if (
            index > after
            and value.get("receiver_image") == "launcher"
            and value.get("mode") == mode
            and value.get("recent_app") == recent
            and value.get("nav_pressed") == "0"
            and value.get("nav_reveal_px") == "0"
        ):
            return index
    raise SystemExit(f"missing stable {mode}/{recent} Launcher state")

foreground_one = state_index("foreground", "android-compatible")
home_recent = state_index("home", "android-compatible", foreground_one)
overview = state_index("overview", "android-compatible", home_recent)
foreground_two = state_index("foreground", "android-compatible", overview)
home_empty = state_index("home", "none", foreground_two)
for boundary in (home_recent, overview, home_empty):
    if not any(index > boundary for index, _value in launcher_commits):
        raise SystemExit("Home/Overview boundary lacks a later Launcher-produced frame")
if not (
    collects[0][0] < foreground_one < claims[0][0]
    and foreground_one < home_recent < overview
    and overview < collects[1][0] < foreground_two < claims[1][0] < home_empty
):
    raise SystemExit("EL0 compatible-Activity lifecycle evidence is out of order")

finishes = [
    (index, value)
    for index, value in exact("UI_SYSTEM_UI_REQUEST_COMPLETED_OK ")
    if value.get("receiver_image") == "launcher"
    and value.get("action") == "finish-compatible"
    and value.get("status") == "accepted"
]
if len(finishes) != 1 or finishes[0][0] <= home_empty:
    raise SystemExit("Back did not finish exactly one compatible recent after empty Home publish")

if any(
    re.search(
        r"fatal exception:|kernel panic:|panicked at|panic!|boot error:|"
        r"USER_FAIL:|EL0_FAIL|THREAD_EXITED:",
        line,
        re.IGNORECASE,
    )
    for line in lines
):
    raise SystemExit("failure marker survived final log validation")

output_path.write_text(
    "\n".join(
        (
            "abi=45",
            "ui_client_control_version=8",
            "ui_server_event_version=7",
            "relaunch_syscall=60",
            "relaunch_count=2",
            "compatible_session_id=1",
            "image_claim_syscall=61",
            "image_claim_count=2",
            "image_claim_owner=app-only",
            f"launcher_pid={launcher_pid}",
            f"app_pid={app_pid}",
            "activity_frame_producer=app",
            "home_overview_frame_producer=launcher",
            "activity_raster_identical=1",
            f"activity_ppm_sha256={activity_sha256}",
            "back_cleared_recent=1",
            "package_disk_unchanged=1",
            f"package_disk_sha256={disk_sha256}",
            f"package={package}",
            f"activity={activity}",
            f"version_code={version_code}",
            "network=disabled",
            "panic_free=1",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

printf '%s\n' \
  "ANDROIDBOX_EL0_RUNTIME0_QEMU_OK artifact_dir=$ARTIFACT_DIR abi=45 ui_client_control_version=8 ui_server_event_version=7 relaunch_syscall=60 relaunch_count=2 compatible_session_id=1 image_claim_syscall=61 image_claim_count=2 image_claim_owner=app-only launcher_pid=$LAUNCHER_PID app_pid=$APP_PID activity_frame_producer=app home_overview_frame_producer=launcher activity_raster_identical=1 activity_ppm_sha256=$ACTIVITY_SHA256 back_cleared_recent=1 package_disk_unchanged=1 package_disk_sha256=$DISK_AFTER_SHA256 apk_sha256=$APK_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE network=disabled panic_free=1 general_apk_claim=0 android_compatibility_claim=0"
