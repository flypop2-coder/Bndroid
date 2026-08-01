#!/usr/bin/env bash
set -euo pipefail

# Strict offline ABI-48 AndroidApp restart/rebind gate.
#
# This is deliberately a bounded compatibility proof. It builds the pinned
# InteractiveActivity-1 APK with the Mac's installed Android SDK, performs one
# sourced install boot, then one source-free recovery boot. The recovery boot
# admits exactly one controlled AndroidApp EL0 data abort after a completed
# Open, proves same-slot generation+1 replacement and same-VMO grant reissue,
# binds success to five real App layered presents, completes the visible
# Open/Click/Close lifecycle, then proves one ordinary Open(4)/Close(5)
# relaunch through the still-live replacement worker.
#
# Cleanup owns only the exact QEMU PID captured from `$!`. Each QEMU invocation
# contains exactly one literal network-disable argument.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if (($#)); then
  printf '%s\n' 'Usage: check-androidbox-restart0.sh' >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed head sort; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the Restart-0 gate." >&2
    exit 1
  }
done

FIXTURE_DIR="$WORKSPACE_ROOT/fixtures/androidbox-interactive-demo"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
FIXTURE_APK="$FIXTURE_DIR/androidbox-interactive-demo.apk"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="$SCRIPT_DIR/build-storage-image.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in "$FIXTURE_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$QMP_HELPER"; do
  [[ -x "$helper" ]] || {
    echo "Required helper is not executable: $helper" >&2
    exit 1
  }
done

SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
AAPT2="${BNDROID_AAPT2:-"$SDK_ROOT/build-tools/36.1.0/aapt2"}"
APKSIGNER="${BNDROID_APKSIGNER:-"$SDK_ROOT/build-tools/36.1.0/apksigner"}"
for sdk_tool in "$AAPT2" "$APKSIGNER"; do
  [[ "$sdk_tool" == /* && -x "$sdk_tool" ]] || {
    echo "Required offline Android SDK tool is unavailable: $sdk_tool" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-120}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

EXPECTED_APK_BYTES=12566
EXPECTED_APK_SHA256=0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee
EXPECTED_SIGNER_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
PACKAGE=org.bndroid.interactive
ACTIVITY='Lorg/bndroid/interactive/MainActivity;'
VERSION_CODE=1

mkdir -p "$WORKSPACE_ROOT/target/androidbox-restart0"
ARTIFACT_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/androidbox-restart0/check.XXXXXX")"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

SOURCE_APK="$ARTIFACT_DIR/interactive.apk"
FIRST_APK="$ARTIFACT_DIR/interactive.first.apk"
FIXTURE_LOG_ONE="$ARTIFACT_DIR/fixture-build-one.log"
FIXTURE_LOG_TWO="$ARTIFACT_DIR/fixture-build-two.log"
BADGING_LOG="$ARTIFACT_DIR/badging.txt"
SIGNER_LOG="$ARTIFACT_DIR/apksigner.txt"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_LOG="$ARTIFACT_DIR/storage-build.log"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-restart0-build"
TARGET_RELEASE="$TARGET_ROOT/aarch64-unknown-none/release"
KERNEL_IMAGE="$TARGET_RELEASE/bndroid-kernel.img"
ACTIVITY_BEFORE="$ARTIFACT_DIR/activity.before.ppm"
ACTIVITY_AFTER="$ARTIFACT_DIR/activity.after.ppm"
ACTIVITY_RELAUNCH="$ARTIFACT_DIR/activity.relaunch.ppm"
DRAWER="$ARTIFACT_DIR/drawer.ppm"
OVERVIEW="$ARTIFACT_DIR/overview.ppm"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
PROTOCOL_EVIDENCE="$ARTIFACT_DIR/protocol-evidence.txt"
SUMMARY="$ARTIFACT_DIR/summary.txt"

QEMU_PID=""
BOOT_MODE=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""
BOOT_QMP_SOCKET=""

cleanup() {
  local pid="$QEMU_PID"
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    kill -TERM "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

normalize_log() {
  tr -d '\r' <"$BOOT_SERIAL_LOG" >"$BOOT_NORMALIZED_LOG"
}

show_failure() {
  if [[ -f "$BOOT_NORMALIZED_LOG" ]]; then
    tail -n 360 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 360 >&2
  fi
  [[ -f "$BOOT_QEMU_LOG" ]] && tail -n 120 "$BOOT_QEMU_LOG" >&2 || true
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Restart-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_bad_output() {
  normalize_log
  local common='fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_CHILD_DIAG|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROID_APP_RESTART_TIMEOUT|ANDROIDBOX_(PROCESS0|RESTART0)_PROFILE_FAIL'
  if grep -Eqi "$common" "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fatal, child failure, or profile failure."
  fi
  if [[ "$BOOT_MODE" == install ]] \
    && grep -Eq 'MOBILE_UI_USER_FAULT|ANDROID_APP_RESTART_ARMED|ANDROID_APP_FAULT_REAP_OK|ANDROID_APP_REBIND_TRANSFER_OK|ANDROID_APP_RESTART_OK' \
      "$BOOT_NORMALIZED_LOG"; then
    fail_gate "The install boot unexpectedly entered the restart path."
  fi
}

wait_for_pattern() {
  local pattern="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_bad_output
    if grep -Eq "$pattern" "$BOOT_NORMALIZED_LOG"; then
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
  grep -Ec "$pattern" "$BOOT_NORMALIZED_LOG" || true
}

wait_for_count() {
  local pattern="$1"
  local minimum="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_bad_output
    local observed
    observed="$(grep -Ec "$pattern" "$BOOT_NORMALIZED_LOG" || true)"
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
  "$QMP_HELPER" "$BOOT_QMP_SOCKET" "$@" \
    || fail_gate "QMP action failed: $*"
}

ppm_matches() {
  local kind="$1"
  local screenshot="$2"
  python3 - "$kind" "$screenshot" <<'PY'
from pathlib import Path
import sys

kind, filename = sys.argv[1:3]
parts = Path(filename).read_bytes().split(b"\n", 3)
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
if expected is None:
    raise SystemExit(1)
fallback_icon_colors = {
    (16, 118, 111), (11, 87, 208), (142, 36, 170), (0, 108, 76),
    (179, 38, 30), (122, 79, 1), (64, 81, 181),
}
def matches(point, color):
    actual = pixel(*point)
    if kind == "drawer" and point == (104, 630):
        return actual in fallback_icon_colors
    return actual == color
if any(not matches(point, color) for point, color in expected.items()):
    raise SystemExit(1)
minimum_colors = 96 if kind == "activity" else 24
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
    reject_bad_output
    qmp screenshot "$screenshot"
    if ppm_matches "$kind" "$screenshot"; then
      return
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

start_qemu() {
  local mode="$1"
  local source_apk="$2"
  local -a source_args=(-name "Bndroid Restart-0 Gate $mode")
  if [[ -n "$source_apk" ]]; then
    source_args+=(-fw_cfg "name=opt/bndroid/apk,file=$source_apk")
  fi

  BOOT_MODE="$mode"
  BOOT_SERIAL_LOG="$ARTIFACT_DIR/$mode.serial.log"
  BOOT_NORMALIZED_LOG="$ARTIFACT_DIR/$mode.serial.normalized.log"
  BOOT_QEMU_LOG="$ARTIFACT_DIR/$mode.qemu.log"
  BOOT_QMP_SOCKET="$ARTIFACT_DIR/$mode.qmp.sock"
  : >"$BOOT_SERIAL_LOG"
  : >"$BOOT_NORMALIZED_LOG"
  : >"$BOOT_QEMU_LOG"

  qemu-system-aarch64 \
    -machine virt,gic-version=2,secure=off,virtualization=off \
    -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
    -nic none \
    -rtc base=2026-07-30T14:30:00,clock=vm \
    -serial "file:$BOOT_SERIAL_LOG" \
    -qmp "unix:$BOOT_QMP_SOCKET,server=on,wait=off" \
    -no-reboot -kernel "$KERNEL_IMAGE" -device ramfb \
    -global virtio-mmio.force-legacy=false \
    -drive "if=none,file=$DISK_IMAGE,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
    -device virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off \
    -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
    -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
    "${source_args[@]}" \
    >"$BOOT_QEMU_LOG" 2>&1 &
  QEMU_PID=$!
}

stop_qemu() {
  local pid="$QEMU_PID"
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    kill -TERM "$pid"
    wait "$pid" 2>/dev/null || true
  fi
  QEMU_PID=""
  normalize_log
  reject_bad_output
}

# Build the real APK twice with only local SDK tools and pin its identity.
"$FIXTURE_BUILD" >"$FIXTURE_LOG_ONE" 2>&1 \
  || { tail -n 160 "$FIXTURE_LOG_ONE" >&2; exit 1; }
cp "$FIXTURE_APK" "$FIRST_APK"
"$FIXTURE_BUILD" >"$FIXTURE_LOG_TWO" 2>&1 \
  || { tail -n 160 "$FIXTURE_LOG_TWO" >&2; exit 1; }
cmp "$FIRST_APK" "$FIXTURE_APK" \
  || { echo "Two fixture builds produced different APK bytes." >&2; exit 1; }
cp "$FIXTURE_APK" "$SOURCE_APK"

APK_BYTES="$(wc -c <"$SOURCE_APK" | tr -d '[:space:]')"
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_BYTES" == "$EXPECTED_APK_BYTES" && "$APK_SHA256" == "$EXPECTED_APK_SHA256" ]] \
  || { echo "Pinned APK length or digest changed." >&2; exit 1; }
"$AAPT2" dump badging "$SOURCE_APK" >"$BADGING_LOG"
"$APKSIGNER" verify --verbose --print-certs "$SOURCE_APK" >"$SIGNER_LOG"
grep -Fqx "package: name='$PACKAGE' versionCode='1' versionName='1.0' platformBuildVersionName='16' platformBuildVersionCode='36' compileSdkVersion='36' compileSdkVersionCodename='16'" "$BADGING_LOG" \
  || { echo "aapt2 package identity mismatch." >&2; exit 1; }
grep -Fqx "Signer #1 certificate SHA-256 digest: $EXPECTED_SIGNER_SHA256" "$SIGNER_LOG" \
  || { echo "APK signer mismatch." >&2; exit 1; }
grep -Fqx 'Verified using v2 scheme (APK Signature Scheme v2): true' "$SIGNER_LOG" \
  || { echo "APK v2 signature was not verified." >&2; exit 1; }

RESTART0_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0,androidbox-process0,androidbox-restart0
if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$RESTART0_FEATURES" \
  BNDROID_USERSPACE_FEATURES="$RESTART0_FEATURES" \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 300 "$BUILD_LOG" >&2
  echo "ABI-48 Restart-0 build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || { echo "Kernel image missing." >&2; exit 1; }

if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$BUILD_STORAGE" --with-package-store >"$STORAGE_LOG" 2>&1; then
  tail -n 180 "$STORAGE_LOG" >&2
  exit 1
fi
[[ "$(wc -c <"$INITIAL_IMAGE" | tr -d '[:space:]')" == 16777216 ]] \
  || { echo "Package disk is not exactly 16 MiB." >&2; exit 1; }
cp "$INITIAL_IMAGE" "$DISK_IMAGE"
INITIAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"

# Boot 1: explicit source installs generation 1, but no Activity launch means
# no controlled fault or replacement is permitted.
start_qemu install "$SOURCE_APK"
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=1 source_admitted=1 .* operation=install .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} .*" \
  "the sourced package install"
wait_for_pattern '^ANDROIDBOX_RESTART0_PROFILE_OK .* abi=48 ' "the ABI-48 profile marker"
stop_qemu
INSTALL_LOG="$BOOT_NORMALIZED_LOG"
INSTALLED_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$INSTALLED_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The install boot did not change the package disk."

# Boot 2: no APK source. Recover the same disk, unlock, open All apps, launch
# the installed row, exercise the post-replacement Button and Close, and
# launch the same compatible identity once more through Overview.
RECOVERY_BEFORE_DISK_SHA256="$INSTALLED_DISK_SHA256"
start_qemu recovery ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* mutation_performed=0 .*" \
  "source-free package recovery"
wait_for_pattern '^MOBILE_UI_PREVIEW_OK .*abi=48 width=720 height=1600 ' \
  "the 720x1600 preview"
wait_for_pattern '^ANDROID_APP_PROCESS_OK .* abi=48 ' "the initial process topology"

normalize_log
SURFACE_PID="$(sed -n 's/^SURFACE_ACQUIRE_OK owner=kernel-fallback pid=\([1-9][0-9]*\) .*/\1/p' "$BOOT_NORMALIZED_LOG")"
LAUNCHER_PID="$(sed -n 's/^UI_ROUTE_FOCUS_OK .* receiver_image=launcher receiver_pid=\([1-9][0-9]*\) .* active_client=launcher .*/\1/p' "$BOOT_NORMALIZED_LOG" | head -n 1)"
APP_PID="$(sed -n 's/^ANDROID_APP_PROCESS_OK .* app_pid=\([1-9][0-9]*\) .*/\1/p' "$BOOT_NORMALIZED_LOG")"
OLD_PID="$(sed -n 's/^ANDROID_APP_PROCESS_OK .* android_app_pid=\([1-9][0-9]*\) .*/\1/p' "$BOOT_NORMALIZED_LOG")"
for pid in "$SURFACE_PID" "$LAUNCHER_PID" "$APP_PID" "$OLD_PID"; do
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || fail_gate "Could not derive the initial process identities."
done
[[ "$(printf '%s\n' "$SURFACE_PID" "$LAUNCHER_PID" "$APP_PID" "$OLD_PID" | sort -u | wc -l | tr -d '[:space:]')" == 4 ]] \
  || fail_gate "Initial process identities are not distinct."

qmp drag 360 1390 360 620
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=none ' \
  "the unlocked Home state"
qmp tap 700 900
qmp drag 360 1280 360 520
wait_for_screenshot drawer "$DRAWER" "the installed All apps row"
qmp tap 106 674

wait_for_pattern '^ANDROID_APP_RESTART_OK .* abi=48 .* errors=0$' \
  "the completed AndroidApp replacement"
normalize_log
NEW_PID="$(sed -n 's/^ANDROID_APP_RESTART_OK .* new_pid=\([1-9][0-9]*\) .*/\1/p' "$BOOT_NORMALIZED_LOG")"
[[ "$NEW_PID" =~ ^[1-9][0-9]*$ && "$NEW_PID" != "$OLD_PID" ]] \
  || fail_gate "Could not derive a distinct replacement AndroidApp PID."

wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  5 \
  "five post-replacement Activity transition frames"
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the recovered Activity raster"

APP_COMMITS_BEFORE="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
qmp tap 360 728
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${NEW_PID} receiver_image=app receiver_pid=${APP_PID} kind=updated request_id=2 " \
  "the replacement worker's Updated response"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE + 2))" \
  "the pressed and callback frames"
wait_for_screenshot activity "$ACTIVITY_AFTER" "the updated Activity raster"

qmp tap 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible ' \
  "Home with compatible recent identity"
wait_for_pattern \
  "^ANDROID_APP_RPC_OK .* abi=48 protocol=BNDAPC01 app_pid=${APP_PID} android_app_pid=${NEW_PID} .* close=1 closed=1 .* errors=0 .* queues_empty=1$" \
  "the complete replacement RPC lifecycle"

# Re-enter the retained compatible identity through Overview. This is an
# ordinary post-recovery launch: it must use the replacement worker, publish a
# fresh ordinary one-shot APK grant, continue request IDs at 4/5, and render a
# second five-frame Activity transition without re-entering Restart-0.
qmp touch-down 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=0 ' \
  "the Overview gesture capture"
qmp touch-move 360 1330
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=240 ' \
  "the Overview finger-follow state"
qmp touch-up
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ' \
  "the compatible Overview"
wait_for_screenshot overview "$OVERVIEW" "the compatible Overview raster"

APP_COMMITS_BEFORE_RELAUNCH="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
NEW_WORKER_CLAIMS_BEFORE_RELAUNCH="$(
  log_count "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${NEW_PID} "
)"
qmp tap 360 920
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} android_app_owner=${NEW_PID} compatible_session_id=1 request_sequence=2 generation=1 .* authority_granted=android-app-read-only-vmo .* apk_bytes_exposed_to_android_app=1 storage_authority_granted=0$" \
  "the ordinary post-recovery durable relaunch"
wait_for_count \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${NEW_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} " \
  "$((NEW_WORKER_CLAIMS_BEFORE_RELAUNCH + 1))" \
  "the replacement worker's fresh ordinary APK claim"
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=app sender_pid=${APP_PID} receiver_image=android-app receiver_pid=${NEW_PID} kind=open request_id=4 " \
  "post-recovery Open request 4"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE_RELAUNCH + 5))" \
  "five ordinary relaunch Activity transition frames"
wait_for_screenshot activity "$ACTIVITY_RELAUNCH" "the ordinarily relaunched Activity raster"
cmp "$ACTIVITY_BEFORE" "$ACTIVITY_RELAUNCH" \
  || fail_gate "The ordinary post-recovery relaunch changed the initial Activity raster."

LAUNCHER_HOME_COUNT_BEFORE="$(
  log_count '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 '
)"
qmp tap 360 1570
wait_for_count \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ' \
  "$((LAUNCHER_HOME_COUNT_BEFORE + 1))" \
  "Home after the ordinary relaunch"
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${NEW_PID} receiver_image=app receiver_pid=${APP_PID} kind=closed request_id=5 " \
  "post-recovery Closed response 5"
wait_for_pattern \
  '^ANDROID_APP_POST_RESTART_RELAUNCH_OK .* abi=48 .* current_last_request_id=5 .* errors=0 queues_empty=1$' \
  "the complete ordinary post-recovery lifecycle"
stop_qemu
RECOVERY_LOG="$BOOT_NORMALIZED_LOG"
RECOVERY_AFTER_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$RECOVERY_AFTER_DISK_SHA256" == "$RECOVERY_BEFORE_DISK_SHA256" ]] \
  || fail_gate "Source-free recovery or interaction changed the package disk."

# Pixel proof: trusted chrome is byte-identical and the callback materially
# changes only Activity content.
python3 - "$ACTIVITY_BEFORE" "$ACTIVITY_AFTER" "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

before_path, after_path, output_path = map(Path, sys.argv[1:4])

def read_ppm(path: Path) -> tuple[bytes, bytes]:
    payload = path.read_bytes()
    parts = payload.split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{path.name}: noncanonical PPM")
    if len(parts[3]) != 720 * 1600 * 3:
        raise SystemExit(f"{path.name}: wrong pixel length")
    return payload, parts[3]

before_payload, before = read_ppm(before_path)
after_payload, after = read_ppm(after_path)
stride = 720 * 3
if before[:64 * stride] != after[:64 * stride]:
    raise SystemExit("trusted top chrome changed")
if before[1512 * stride:] != after[1512 * stride:]:
    raise SystemExit("trusted bottom chrome changed")

def changed(bounds: tuple[int, int, int, int]) -> int:
    left, top, right, bottom = bounds
    count = 0
    for y in range(top, bottom):
        for x in range(left, right):
            offset = (y * 720 + x) * 3
            count += before[offset:offset + 3] != after[offset:offset + 3]
    return count

content = changed((0, 64, 720, 1512))
label = changed((68, 448, 652, 624))
if content < 3500 or label < 3000 or before_payload == after_payload:
    raise SystemExit("callback did not materially change the expected content")
output_path.write_text(
    "\n".join((
        f"content_changed_pixels={content}",
        f"label_changed_pixels={label}",
        "top_chrome_byte_identical=1",
        "bottom_chrome_byte_identical=1",
        f"before_sha256={sha256(before_payload).hexdigest()}",
        f"after_sha256={sha256(after_payload).hexdigest()}",
    )) + "\n",
    encoding="utf-8",
)
PY

# Parse the serial transcript as one closed event graph.
python3 - \
  "$INSTALL_LOG" "$RECOVERY_LOG" "$PROTOCOL_EVIDENCE" \
  "$APP_PID" "$OLD_PID" "$NEW_PID" "$APK_BYTES" <<'PY'
from pathlib import Path
import re
import sys

install_path, recovery_path, output_path = map(Path, sys.argv[1:4])
app_pid, old_pid, new_pid, apk_bytes = sys.argv[4:8]
install = install_path.read_text(encoding="utf-8").splitlines()
lines = recovery_path.read_text(encoding="utf-8").splitlines()

def fields(line: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for item in line.split()[1:]:
        if "=" not in item:
            raise SystemExit(f"noncanonical marker field: {item!r}")
        key, value = item.split("=", 1)
        if key in result:
            raise SystemExit(f"duplicate marker field: {key}")
        result[key] = value
    return result

def exact(prefix: str) -> tuple[int, dict[str, str]]:
    found = [(index, fields(line)) for index, line in enumerate(lines) if line.startswith(prefix)]
    if len(found) != 1:
        raise SystemExit(f"expected exactly one {prefix!r}, saw {len(found)}")
    return found[0]

for forbidden in (
    "MOBILE_UI_USER_FAULT ",
    "ANDROID_APP_RESTART_ARMED ",
    "ANDROID_APP_FAULT_REAP_OK ",
    "ANDROID_APP_REBIND_TRANSFER_OK ",
    "ANDROID_APP_RESTART_OK ",
):
    if any(line.startswith(forbidden) for line in install):
        raise SystemExit(f"install boot contained {forbidden.strip()}")

process_i, process = exact("ANDROID_APP_PROCESS_OK ")
preview_i, preview = exact("MOBILE_UI_PREVIEW_OK ")
process_profile_i, process_profile = exact("ANDROIDBOX_PROCESS0_PROFILE_OK ")
restart_profile_i, restart_profile = exact("ANDROIDBOX_RESTART0_PROFILE_OK ")
armed_i, armed = exact("ANDROID_APP_RESTART_ARMED ")
faults = [(i, line) for i, line in enumerate(lines) if line.startswith("MOBILE_UI_USER_FAULT ")]
if len(faults) != 1:
    raise SystemExit(f"expected one controlled user fault, saw {len(faults)}")
fault_i, fault_line = faults[0]
reap_i, reap = exact("ANDROID_APP_FAULT_REAP_OK ")
rebind_i, rebind = exact("ANDROID_APP_REBIND_TRANSFER_OK ")
ready_i, ready = exact("ANDROID_APP_REPLACEMENT_READY_OK ")
grant_i, grant = exact("ANDROID_PACKAGE_IMAGE_GRANT_REBIND_OK ")
rebound_i, rebound = exact("ANDROID_APP_REBOUND_ACK_OK ")
reopen_i, reopen_marker = exact("ANDROID_APP_REOPEN_OK ")
restart_i, restart = exact("ANDROID_APP_RESTART_OK ")
rpc_i, rpc = exact("ANDROID_APP_RPC_OK ")
post_relaunch_i, post_relaunch = exact("ANDROID_APP_POST_RESTART_RELAUNCH_OK ")

install_restart_profiles = [
    fields(line) for line in install if line.startswith("ANDROIDBOX_RESTART0_PROFILE_OK ")
]
if len(install_restart_profiles) != 1 or install_restart_profiles[0].get("abi") != "48":
    raise SystemExit("install boot lacks one ABI-48 Restart-0 profile")

if not (process_i < armed_i < fault_i < reap_i < rebind_i < ready_i < grant_i < rebound_i < reopen_i < restart_i < rpc_i < post_relaunch_i):
    raise SystemExit("restart markers are out of canonical order")

if process.get("app_pid") != app_pid or process.get("android_app_pid") != old_pid:
    raise SystemExit("initial process marker identity mismatch")
for key, value in {
    "format": "1", "abi": "48", "app_image": "7", "android_app_image": "10",
    "elf_distinct": "1", "pid_distinct": "1", "asid_distinct": "1",
    "root_distinct": "1", "process_capacity": "9", "dynamic_capacity": "8",
    "live_processes": "9", "android_app_live": "1", "worker_handles": "1",
    "worker_channel_count": "1", "worker_unexpected_handle_count": "0",
    "private_endpoint_count": "2", "private_pair_count": "1",
    "app_endpoint_count": "1", "unexpected_private_owner_count": "0",
    "endpoints_unique": "1", "worker_rights_valid": "1",
    "app_rights_valid": "1", "android_app_duplicate": "0",
    "android_app_transfer": "0", "queues_empty": "1", "objects_valid": "1",
    "isolation_valid": "1", "valid": "1", "surface_handles": "0",
    "graphics_handles": "0", "input_handles": "0", "storage_handles": "0",
}.items():
    if process.get(key) != value:
        raise SystemExit(f"initial process topology mismatch: {key}")

for actual, expected in (
    (preview, {
        "abi": "48", "width": "720", "height": "1600", "buffers": "3",
        "live_processes": "9", "network": "disabled", "real_phone_claim": "0",
    }),
    (process_profile, {
        "format": "1", "abi": "48", "parent_profile": "androidbox-interactive0",
        "execution_host": "independent-android-app-el0",
        "standalone_android_process": "1", "dedicated_android_process": "1",
        "trusted_ui_host": "app-el0", "android_app_surface_handles": "0",
        "android_app_graphics_handles": "0", "android_app_input_handles": "0",
        "android_app_storage_handles": "0", "app_apk_digest_reverified": "0",
        "android_app_apk_digest_reverified": "1",
        "android_app_apk_v2_signature_reverified": "1",
        "android_app_signer_reverified": "1", "art": "0", "dalvik": "0",
        "binder": "0", "bionic": "0", "jni": "0", "native_lib": "0",
        "general_apk_claim": "0", "general_android_compatibility_claim": "0",
        "network": "disabled", "emulator_only": "1", "real_phone_claim": "0",
    }),
    (restart_profile, {
        "format": "1", "abi": "48", "parent_profile": "androidbox-process0",
        "restart_owner": "init", "restart_budget": "1",
        "controlled_worker_fault": "el0-data-abort",
        "rebind_transport": "retained-authenticated-bootstrap",
        "same_slot_required": "1", "generation_step_required": "1",
        "replacement_ready_required": "1", "completed_open_replayed": "1",
        "ambiguous_request_replay": "0",
        "recovery_scope": "one-android-app-worker",
        "general_process_supervisor": "0", "general_crash_recovery_claim": "0",
        "general_android_compatibility_claim": "0", "network": "disabled",
        "emulator_only": "1", "real_phone_claim": "0",
    }),
):
    for key, value in expected.items():
        if actual.get(key) != value:
            raise SystemExit(f"profile mismatch: {key}")

old = int(old_pid)
new = int(new_pid)
if old & 0xffff_ffff != new & 0xffff_ffff:
    raise SystemExit("replacement did not reuse the same process slot")
if (new >> 32) != (old >> 32) + 1:
    raise SystemExit("replacement generation did not advance exactly once")

fault_match = re.fullmatch(
    rf"MOBILE_UI_USER_FAULT pid={old_pid} reason=2 "
    r"esr=0x0000000092000047 far=0x00000002001ee000 "
    r"elr=(0x[0-9a-f]{16}) sp=(0x[0-9a-f]{16})",
    fault_line,
)
if fault_match is None:
    raise SystemExit("controlled data-abort tuple is not exact")
elr = int(fault_match.group(1), 16)
sp = int(fault_match.group(2), 16)
if not (0x0000000200000000 <= elr < 0x0000000200200000):
    raise SystemExit("controlled fault ELR is outside AndroidApp code")
if not (0x00000002001ef000 <= sp < 0x00000002001ff000):
    raise SystemExit("controlled fault SP is outside the mapped user stack")

expected_maps = (
    (armed, {"format": "1", "abi": "48", "app_pid": app_pid, "old_pid": old_pid,
             "session_id": "1", "package_generation": "1", "request_id": "2",
             "fault_point": "after-open-before-ui-commit", "restart_budget": "1"}),
    (reap, {"format": "1", "abi": "48", "epoch": "1", "old_pid": old_pid,
            "exit_code": "0", "termination_reason": "faulted",
            "fault_reason": "2", "esr": "0x0000000092000047",
            "far": "0x00000002001ee000", "elr": fault_match.group(1),
            "sp": fault_match.group(2), "witness_valid": "1",
            "escrow_retained": "1"}),
    (rebind, {"format": "1", "abi": "48", "epoch": "1", "app_pid": app_pid,
              "old_pid": old_pid, "new_pid": new_pid, "exit_code": "0",
              "termination_reason": "2", "same_slot": "1", "generation_step": "1",
              "endpoint_transfer": "1"}),
    (ready, {"format": "1", "abi": "48", "epoch": "1", "app_pid": app_pid,
             "old_pid": old_pid, "new_pid": new_pid, "request_reset": "1",
             "ready_request_id": "0"}),
    (grant, {"format": "1", "abi": "48", "epoch": "1", "old_owner": old_pid,
             "new_owner": new_pid, "compatible_session_id": "1",
             "package_generation": "1", "request_id": "1",
             "trigger": "authenticated-replacement-open", "immutable_vmo": "1",
             "same_vmo": "1", "same_slot": "1", "generation_step": "1",
             "reissue": "1/1", "apk_bytes_exposed_to_init": "0",
             "apk_bytes_exposed_to_app": "0"}),
    (rebound, {"format": "1", "abi": "48", "epoch": "1", "app_pid": app_pid,
               "old_pid": old_pid, "new_pid": new_pid, "authenticated": "1"}),
    (reopen_marker, {"format": "1", "abi": "48", "epoch": "1",
                     "app_pid": app_pid, "new_pid": new_pid, "session_id": "1",
                     "package_generation": "1", "request_id": "1", "ready": "1",
                     "open": "1", "opened": "1", "label_chunks": "2",
                     "label_bytes": "26", "button_chunks": "1",
                     "button_bytes": "11", "phase": "active",
                     "completed_open_replayed": "1",
                     "ambiguous_request_replay": "0"}),
)
for actual, expected in expected_maps:
    for key, value in expected.items():
        if actual.get(key) != value:
            raise SystemExit(f"restart field mismatch: {key}={actual.get(key)!r}, expected {value!r}")

for key, value in {
    "format": "1", "abi": "48", "epoch": "1", "app_pid": app_pid,
    "old_pid": old_pid, "new_pid": new_pid, "session_id": "1",
    "package_generation": "1", "crash_request_id": "2", "exit_code": "0",
    "termination_reason": "faulted", "fault_reap": "1",
    "peer_close_observed": "1", "endpoint_transfer": "1",
    "replacement_ready": "1", "rebound_ack": "1", "same_slot": "1",
    "generation_step": "1", "created_delta": "1", "exited_delta": "1",
    "reaped_delta": "1", "terminated_faulted_delta": "1",
    "live_processes": "9", "android_app_live": "1", "worker_handles": "1",
    "worker_channels": "1", "queues_empty": "1", "restart_budget": "1/1",
    "image_escrow": "1", "image_reissue": "1", "replacement_image_claim": "1",
    "image_held": "0", "completed_open_replayed": "1",
    "ambiguous_request_replay": "0", "app_graphics_present": "1",
    "app_enter_frames": "5", "app_layered_commits": "5", "errors": "0",
}.items():
    if restart.get(key) != value:
        raise SystemExit(f"restart summary mismatch: {key}")

claims = [(i, fields(line)) for i, line in enumerate(lines)
          if line.startswith("ANDROID_PACKAGE_IMAGE_CLAIM_OK ")]
if len(claims) != 3:
    raise SystemExit(f"expected exactly three package image claims, saw {len(claims)}")
if [claim["owner"] for _, claim in claims] != [old_pid, new_pid, new_pid]:
    raise SystemExit("package claims do not belong exactly to old, replacement, replacement")
for _, claim in claims:
    for key, value in {
        "compatible_session_id": "1", "generation": "1",
        "apk_length": apk_bytes, "image": "android-app",
        "apk_bytes_exposed_to_app": "0", "apk_bytes_exposed_to_android_app": "1",
        "rights": "READ", "duplicate": "0", "transfer": "0", "map": "0",
        "write": "0", "execute": "0", "wait": "0",
        "storage_authority_granted": "0",
    }.items():
        if claim.get(key) != value:
            raise SystemExit(f"package claim mismatch: {key}")
if not (
    claims[0][0] < armed_i
    # Rebound travels on Init's supervisor channel while the replacement
    # claim follows Open on the private worker channel. Their reads may be
    # scheduled in either order; both remain causally after grant reissue and
    # before the completed reopen.
    and grant_i < claims[1][0] < reopen_i
    and rpc_i < claims[2][0] < post_relaunch_i
):
    raise SystemExit("package claim/reissue causal bounds are not exact")

for key, value in {
    "format": "1", "abi": "48", "protocol": "BNDAPC01",
    "app_pid": app_pid, "android_app_pid": new_pid,
    "ready_request_id": "0", "open_request_id": "1",
    "click_request_id": "2", "close_request_id": "3",
    "request_order": "0/1/2/3", "app_sender_authenticated": "1",
    "worker_sender_authenticated": "1", "ready": "1", "open": "1",
    "opened": "1", "label_chunks": "2", "label_chunk_bytes": "24/2",
    "label_bytes": "26", "button_chunks": "1", "button_chunk_bytes": "11",
    "button_bytes": "11", "click": "1", "updated": "1",
    "update_chunks": "1", "update_chunk_bytes": "24", "update_bytes": "24",
    "close": "1", "closed": "1", "final_revision": "1", "errors": "0",
    "max_outstanding": "1", "queue_capacity": "8", "queues_empty": "1",
}.items():
    if rpc.get(key) != value:
        raise SystemExit(f"final RPC mismatch: {key}")

for key, value in {
    "format": "1", "abi": "48", "epoch": "1", "app_pid": app_pid,
    "android_app_pid": new_pid, "replacement_pid": new_pid,
    "open_request_id": "4", "close_request_id": "5",
    "current_last_request_id": "5", "completed_rounds": "2",
    "post_complete_messages": "7", "phase": "idle",
    "worker_still_replacement": "1", "errors": "0", "queues_empty": "1",
}.items():
    if post_relaunch.get(key) != value:
        raise SystemExit(f"post-restart relaunch mismatch: {key}")

app_commit_indices = [
    i for i, line in enumerate(lines)
    if line.startswith("USER_SURFACE_LAYERED_COMMIT_OK ")
    and f" content_producer_pid={app_pid} " in line
]
if len(app_commit_indices) != 12:
    raise SystemExit(f"expected exactly twelve App commits, saw {len(app_commit_indices)}")
if not (app_commit_indices[4] < restart_i < app_commit_indices[5]):
    raise SystemExit("restart success was not bound to exactly five visible App commits")
if not (rpc_i < app_commit_indices[7] and app_commit_indices[11] < post_relaunch_i):
    raise SystemExit("ordinary relaunch commits are outside its canonical lifecycle")

post_rpc = [
    (i, fields(line)) for i, line in enumerate(lines[rpc_i + 1:], rpc_i + 1)
    if line.startswith("ANDROID_APP_RPC_READ_OK ")
]
expected_post_rpc = (
    ("app", app_pid, "android-app", new_pid, "open", "4"),
    ("android-app", new_pid, "app", app_pid, "opened", "4"),
    ("android-app", new_pid, "app", app_pid, "label-chunk", "4"),
    ("android-app", new_pid, "app", app_pid, "label-chunk", "4"),
    ("android-app", new_pid, "app", app_pid, "button-chunk", "4"),
    ("app", app_pid, "android-app", new_pid, "close", "5"),
    ("android-app", new_pid, "app", app_pid, "closed", "5"),
)
if len(post_rpc) != len(expected_post_rpc):
    raise SystemExit(f"expected seven post-complete RPC messages, saw {len(post_rpc)}")
for (index, actual), expected in zip(post_rpc, expected_post_rpc, strict=True):
    sender_image, sender_pid, receiver_image, receiver_pid, kind, request_id = expected
    for key, value in {
        "sender_image": sender_image, "sender_pid": sender_pid,
        "receiver_image": receiver_image, "receiver_pid": receiver_pid,
        "kind": kind, "request_id": request_id, "first_round": "0",
        "errors": "0",
    }.items():
        if actual.get(key) != value:
            raise SystemExit(f"post-complete RPC mismatch at line {index + 1}: {key}")
if not (
    post_rpc[0][0] < claims[2][0] < post_rpc[1][0]
    and post_rpc[-1][0] < post_relaunch_i
):
    raise SystemExit("ordinary grant/RPC/post-relaunch marker order is not exact")

if any(old_pid in line for line in lines[fault_i + 1:] if line.startswith("ANDROID_APP_RPC_READ_OK ")):
    raise SystemExit("old worker identity appeared in RPC after its fault")

output_path.write_text(
    "\n".join((
        f"app_pid={app_pid}",
        f"old_android_app_pid={old_pid}",
        f"new_android_app_pid={new_pid}",
        "same_slot=1",
        "generation_step=1",
        "controlled_faults=1",
        "fault_reaps=1",
        "image_claims=3",
        "same_vmo_reissue=1",
        "app_commits=12",
        "rpc_request_order=0/1/2/3",
        "post_restart_request_order=4/5",
        "ordinary_relaunch=1",
        "rpc_errors=0",
    )) + "\n",
    encoding="utf-8",
)
PY

BEFORE_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_BEFORE" | awk '{print $1}')"
AFTER_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_AFTER" | awk '{print $1}')"
RELAUNCH_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_RELAUNCH" | awk '{print $1}')"
CONTENT_CHANGED="$(sed -n 's/^content_changed_pixels=//p' "$RASTER_EVIDENCE")"
LABEL_CHANGED="$(sed -n 's/^label_changed_pixels=//p' "$RASTER_EVIDENCE")"

printf '%s\n' \
  "ANDROIDBOX_RESTART0_QEMU_OK artifact_dir=$ARTIFACT_DIR abi=48 scope=bounded-interactiveactivity1-worker-restart qemu_boots=2 qemu_nic_none_per_boot=1 install_source=qemu-fw_cfg recovery_source=none apk_bytes=$APK_BYTES apk_sha256=$APK_SHA256 signer_cert_sha256=$EXPECTED_SIGNER_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE app_pid=$APP_PID old_android_app_pid=$OLD_PID new_android_app_pid=$NEW_PID same_slot=1 generation_step=1 controlled_faults=1 fault_witness_bound=1 fault_reaps=1 endpoint_rebinds=1 image_claims=3 image_escrows=1 image_reissues=1 same_vmo=1 reopened=1 post_restart_relaunch=1 ambiguous_request_replay=0 app_layered_commits=12 restart_bound_app_commits=5 rpc_request_order=0/1/2/3 post_restart_request_order=4/5 rpc_click=1 rpc_close=2 rpc_errors=0 content_changed_pixels=$CONTENT_CHANGED label_changed_pixels=$LABEL_CHANGED before_ppm_sha256=$BEFORE_PPM_SHA256 after_ppm_sha256=$AFTER_PPM_SHA256 relaunch_ppm_sha256=$RELAUNCH_PPM_SHA256 relaunch_ppm_identical=1 recovery_disk_unchanged=1 recovery_disk_sha256=$RECOVERY_AFTER_DISK_SHA256 network=disabled unexpected_faults=0 restart_timeout=armed panic_fatal_free=1 art=0 dalvik=0 binder=0 general_android_compatibility_claim=0 real_phone_claim=0" \
  | tee "$SUMMARY"

trap - EXIT
