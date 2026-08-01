#!/usr/bin/env bash
set -euo pipefail

# Strict offline ABI-50 AndroidApp MultiActionActivity-3 gate.
#
# This remains a bounded compatibility proof. It builds the pinned multi-action
# APK with the Mac's installed SDK, performs one sourced install boot, then
# one source-free recovery boot. The recovery boot proves the five-node
# publisher scene, one controlled worker replacement, two independently
# addressed callback branches, and an ordinary source-free relaunch through
# the same replacement worker.
#
# Cleanup owns only the exact QEMU PID captured from `$!`. Each QEMU invocation
# contains exactly one literal network-disable argument.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if (($#)); then
  printf '%s\n' 'Usage: check-androidbox-multiaction3.sh' >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed head sort; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the MultiActionActivity-3 gate." >&2
    exit 1
  }
done

FIXTURE_DIR="$WORKSPACE_ROOT/fixtures/androidbox-multiaction-demo"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
FIXTURE_APK="$FIXTURE_DIR/androidbox-multiaction-demo.apk"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
if [[ "${BNDROID_MULTIACTION_GATE_MULTIPACKAGE_STORAGE:-0}" == 1 ]]; then
  BUILD_STORAGE="$SCRIPT_DIR/build-multipackage-storage-image.sh"
  STORAGE_BUILD_ARG=""
else
  BUILD_STORAGE="$SCRIPT_DIR/build-storage-image.sh"
  STORAGE_BUILD_ARG=--with-package-store
fi
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

ABI="${BNDROID_MULTIACTION_GATE_ABI:-50}"
PROTOCOL="${BNDROID_MULTIACTION_GATE_PROTOCOL:-BNDAPC03}"
PROTOCOL_VERSION="${BNDROID_MULTIACTION_GATE_PROTOCOL_VERSION:-3}"
EXPECTED_APK_BYTES="${BNDROID_MULTIACTION_GATE_EXPECTED_APK_BYTES:-12573}"
EXPECTED_APK_SHA256="${BNDROID_MULTIACTION_GATE_EXPECTED_APK_SHA256:-aecf0749e2c063f44945f6154b479714fa8ebead8da26b0ebef005d2737036c6}"
TERMINAL="${BNDROID_MULTIACTION_GATE_TERMINAL:-ANDROIDBOX_MULTIACTION3_QEMU_OK}"
EXPECTED_SIGNER_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
PACKAGE=org.bndroid.multiaction
ACTIVITY='Lorg/bndroid/multiaction/MainActivity;'
VERSION_CODE=1
APPROVE_ID=2130771968
REJECT_ID=2130771969
STATUS_ID=2130771970
TITLE_ID=2130771971
SCENE_NODES=5
SCENE_TEXT_BYTES=44
SCENE_TEXT_VIEWS=2
CALLBACK_BUTTONS=2
CRASH_REQUEST_ID=7
APPROVE_REQUEST_ID=7
REJECT_REQUEST_ID=8
CLOSE_REQUEST_ID=9
RELAUNCH_OPEN_REQUEST_ID=10
RELAUNCH_DESCRIBE_REQUEST_IDS=11/12/13/14/15
RELAUNCH_CLOSE_REQUEST_ID=16
FIRST_RPC_MESSAGES=25
RELAUNCH_RPC_MESSAGES=18
REPLACEMENT_RPC_MESSAGES=43
FIRST_ROUND_APP_COMMITS=9
TOTAL_APP_COMMITS=14
if [[ "$ABI" == 58 && "$PROTOCOL" == BNDAPC04 && "$PROTOCOL_VERSION" == 4 ]]; then
  APP_DEFINED_CALL_COUNT=1
elif [[ "$ABI" == 50 && "$PROTOCOL" == BNDAPC03 && "$PROTOCOL_VERSION" == 3 ]]; then
  APP_DEFINED_CALL_COUNT=0
else
  echo "Expected ABI/protocol 50/BNDAPC03/v3 or 58/BNDAPC04/v4." >&2
  exit 2
fi
UPDATED_ARG0=$(((APP_DEFINED_CALL_COUNT << 32) | STATUS_ID))

ARTIFACT_ROOT="${BNDROID_MULTIACTION_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-multiaction3"}"
mkdir -p "$ARTIFACT_ROOT"
ARTIFACT_DIR="$(mktemp -d "$ARTIFACT_ROOT/check.XXXXXX")"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

SOURCE_APK="$ARTIFACT_DIR/multiaction.apk"
FIRST_APK="$ARTIFACT_DIR/multiaction.first.apk"
FIXTURE_LOG_ONE="$ARTIFACT_DIR/fixture-build-one.log"
FIXTURE_LOG_TWO="$ARTIFACT_DIR/fixture-build-two.log"
BADGING_LOG="$ARTIFACT_DIR/badging.txt"
SIGNER_LOG="$ARTIFACT_DIR/apksigner.txt"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_LOG="$ARTIFACT_DIR/storage-build.log"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
TARGET_ROOT="${BNDROID_MULTIACTION_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-multiaction3-build"}"
TARGET_RELEASE="$TARGET_ROOT/aarch64-unknown-none/release"
KERNEL_IMAGE="$TARGET_RELEASE/bndroid-kernel.img"
ACTIVITY_BEFORE="$ARTIFACT_DIR/activity.before.ppm"
ACTIVITY_APPROVED="$ARTIFACT_DIR/activity.approved.ppm"
ACTIVITY_REJECTED="$ARTIFACT_DIR/activity.rejected.ppm"
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
    tail -n 420 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 420 >&2
  fi
  [[ -f "$BOOT_QEMU_LOG" ]] && tail -n 120 "$BOOT_QEMU_LOG" >&2 || true
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "MultiActionActivity-3 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_bad_output() {
  normalize_log
  local common='fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_CHILD_DIAG|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROID_APP_RESTART_TIMEOUT|ANDROIDBOX_(PROCESS0|RESTART0|SCENE_RPC2|MULTIACTION3)_PROFILE_FAIL'
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

if kind == "drawer":
    expected = {
        (360, 300): (13, 22, 41),
        (104, 630): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    }
    minimum_colors = 24
elif kind == "overview":
    expected = {
        (360, 300): (13, 22, 41),
        (360, 580): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    }
    minimum_colors = 24
elif kind == "activity":
    # Scene content is publisher-derived, so the gate validates the canonical
    # full frame and trusted rounded display rather than pinning publisher
    # glyph pixels here. The before/after proof below binds the exact mutation.
    expected = {
        (0, 0): (0, 0, 0),
        (719, 0): (0, 0, 0),
        (0, 1599): (0, 0, 0),
        (719, 1599): (0, 0, 0),
    }
    minimum_colors = 96
else:
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
  local -a source_args=(-name "Bndroid MultiActionActivity-3 Gate $mode")
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

# Build the real multi-action APK twice with local SDK tools and pin its identity.
BNDROID_MULTIACTION_OUTPUT_APK="$SOURCE_APK" \
  "$FIXTURE_BUILD" >"$FIXTURE_LOG_ONE" 2>&1 \
  || { tail -n 180 "$FIXTURE_LOG_ONE" >&2; exit 1; }
cp "$SOURCE_APK" "$FIRST_APK"
BNDROID_MULTIACTION_OUTPUT_APK="$SOURCE_APK" \
  "$FIXTURE_BUILD" >"$FIXTURE_LOG_TWO" 2>&1 \
  || { tail -n 180 "$FIXTURE_LOG_TWO" >&2; exit 1; }
cmp "$FIRST_APK" "$SOURCE_APK" \
  || { echo "Two fixture builds produced different APK bytes." >&2; exit 1; }

APK_BYTES="$(wc -c <"$SOURCE_APK" | tr -d '[:space:]')"
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_BYTES" == "$EXPECTED_APK_BYTES" && "$APK_SHA256" == "$EXPECTED_APK_SHA256" ]] \
  || { echo "Pinned multi-action APK length or digest changed." >&2; exit 1; }
"$AAPT2" dump badging "$SOURCE_APK" >"$BADGING_LOG"
"$APKSIGNER" verify --verbose --print-certs "$SOURCE_APK" >"$SIGNER_LOG"
grep -Fqx "package: name='$PACKAGE' versionCode='1' versionName='1.0' platformBuildVersionName='16' platformBuildVersionCode='36' compileSdkVersion='36' compileSdkVersionCodename='16'" "$BADGING_LOG" \
  || { echo "aapt2 package identity mismatch." >&2; exit 1; }
grep -Fqx "launchable-activity: name='org.bndroid.multiaction.MainActivity'  label='' icon=''" "$BADGING_LOG" \
  || { echo "aapt2 launcher component mismatch." >&2; exit 1; }
grep -Fqx "Signer #1 certificate SHA-256 digest: $EXPECTED_SIGNER_SHA256" "$SIGNER_LOG" \
  || { echo "APK signer mismatch." >&2; exit 1; }
grep -Fqx 'Verified using v2 scheme (APK Signature Scheme v2): true' "$SIGNER_LOG" \
  || { echo "APK v2 signature was not verified." >&2; exit 1; }

MULTIACTION3_FEATURES="${BNDROID_MULTIACTION_GATE_FEATURES:-mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0,androidbox-process0,androidbox-restart0,androidbox-scene-rpc2,androidbox-multiaction3}"
if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$MULTIACTION3_FEATURES" \
  BNDROID_USERSPACE_FEATURES="$MULTIACTION3_FEATURES" \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 320 "$BUILD_LOG" >&2
  echo "ABI-$ABI MultiActionActivity-3 build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || { echo "Kernel image missing." >&2; exit 1; }

if [[ -n "$STORAGE_BUILD_ARG" ]]; then
  BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
    "$BUILD_STORAGE" "$STORAGE_BUILD_ARG" >"$STORAGE_LOG" 2>&1 \
    || { tail -n 180 "$STORAGE_LOG" >&2; exit 1; }
elif ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$BUILD_STORAGE" >"$STORAGE_LOG" 2>&1; then
  tail -n 180 "$STORAGE_LOG" >&2
  exit 1
fi
[[ "$(wc -c <"$INITIAL_IMAGE" | tr -d '[:space:]')" == 16777216 ]] \
  || { echo "Package disk is not exactly 16 MiB." >&2; exit 1; }
cp "$INITIAL_IMAGE" "$DISK_IMAGE"
INITIAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"

# Boot 1: explicit source installs generation 1 without launching the Activity.
start_qemu install "$SOURCE_APK"
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=1 source_admitted=1 .* operation=install .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} .*" \
  "the sourced package install"
wait_for_pattern \
  "^ANDROIDBOX_MULTIACTION3_PROFILE_OK format=1 abi=${ABI} parent_profile=androidbox-scene-rpc2 protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} scene_max_nodes=8 node_text_max_bytes=96 scene_transport=pull describe_order=preorder describe_max_outstanding=1 node_descriptor_bytes=16 " \
  "the ABI-$ABI MultiActionActivity-3 profile marker"
if [[ "$APP_DEFINED_CALL_COUNT" == 1 ]]; then
  wait_for_pattern \
    "^ANDROIDBOX_DEX_METHODS8_PROFILE_OK .* abi=${ABI} .* protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} .* app_defined_invoke=invoke-static .* helper_owner=same-activity helper_access=private-static helper_proto=I-to-I .* network=disabled .* real_phone_claim=0$" \
    "the ABI-$ABI APK-defined method profile marker"
fi
stop_qemu
INSTALL_LOG="$BOOT_NORMALIZED_LOG"
INSTALLED_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$INSTALLED_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The install boot did not change the package disk."

# Boot 2: recover without an APK source, launch the profile row, survive the
# one bounded worker fault, exercise both status branches, and relaunch.
RECOVERY_BEFORE_DISK_SHA256="$INSTALLED_DISK_SHA256"
start_qemu recovery ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* mutation_performed=0 .*" \
  "source-free package recovery"
wait_for_pattern "^MOBILE_UI_PREVIEW_OK .*abi=${ABI} width=720 height=1600 " \
  "the ABI-$ABI 720x1600 preview"
wait_for_pattern "^ANDROID_APP_PROCESS_OK .* abi=${ABI} " "the initial process topology"
wait_for_pattern \
  "^ANDROIDBOX_MULTIACTION3_PROFILE_OK format=1 abi=${ABI} parent_profile=androidbox-scene-rpc2 protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} " \
  "the recovery MultiActionActivity-3 profile marker"
if [[ "$APP_DEFINED_CALL_COUNT" == 1 ]]; then
  wait_for_pattern \
    "^ANDROIDBOX_DEX_METHODS8_PROFILE_OK .* abi=${ABI} .* protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} .* app_defined_invoke=invoke-static .* helper_owner=same-activity helper_access=private-static helper_proto=I-to-I .* network=disabled .* real_phone_claim=0$" \
    "the recovered APK-defined method profile marker"
fi

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
wait_for_screenshot drawer "$DRAWER" "the installed profile row"
qmp tap 106 674

wait_for_pattern "^ANDROID_APP_RESTART_OK .* abi=${ABI} .* crash_request_id=${CRASH_REQUEST_ID} .* errors=0$" \
  "the completed MultiActionActivity-3 worker replacement"
normalize_log
NEW_PID="$(sed -n 's/^ANDROID_APP_RESTART_OK .* new_pid=\([1-9][0-9]*\) .*/\1/p' "$BOOT_NORMALIZED_LOG")"
[[ "$NEW_PID" =~ ^[1-9][0-9]*$ && "$NEW_PID" != "$OLD_PID" ]] \
  || fail_gate "Could not derive a distinct replacement AndroidApp PID."

wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  5 \
  "five post-replacement scene transition frames"
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the initial pending five-node scene raster"

APP_COMMITS_BEFORE="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
qmp tap 360 688
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${NEW_PID} receiver_image=app receiver_pid=${APP_PID} kind=updated request_id=${APPROVE_REQUEST_ID} arg0=${UPDATED_ARG0} " \
  "the approved status TextView response"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE + 2))" \
  "the pressed and approved-callback frames"
wait_for_screenshot activity "$ACTIVITY_APPROVED" "the approved scene raster"

APP_COMMITS_AFTER_APPROVE="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
qmp tap 360 792
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${NEW_PID} receiver_image=app receiver_pid=${APP_PID} kind=updated request_id=${REJECT_REQUEST_ID} arg0=${UPDATED_ARG0} " \
  "the rejected status TextView response"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_AFTER_APPROVE + 2))" \
  "the pressed and rejected-callback frames"
wait_for_screenshot activity "$ACTIVITY_REJECTED" "the rejected scene raster"

qmp tap 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible ' \
  "Home with the compatible recent identity"
wait_for_pattern \
  "^ANDROID_APP_MULTIACTION_RPC_OK .* abi=${ABI} protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} app_pid=${APP_PID} android_app_pid=${NEW_PID} .* node_count=${SCENE_NODES} .* title_id=${TITLE_ID} status_id=${STATUS_ID} approve_button_id=${APPROVE_ID} reject_button_id=${REJECT_ID} .* clicked_button_ids=${APPROVE_ID}/${REJECT_ID} .* update_view_ids=${STATUS_ID}/${STATUS_ID} .* final_revision=2 .* errors=0 .* queues_empty=1$" \
  "the complete two-action lifecycle"
if [[ "$APP_DEFINED_CALL_COUNT" == 1 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_DEX_METHODS8_RPC_OK .* abi=${ABI} protocol=${PROTOCOL} protocol_version=${PROTOCOL_VERSION} app_pid=${APP_PID} android_app_pid=${NEW_PID} .* clicked_button_ids=${APPROVE_ID}/${REJECT_ID} update_view_ids=${STATUS_ID}/${STATUS_ID} app_defined_calls=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the APK-defined method provenance"
fi

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
  "the ordinary source-free durable relaunch"
wait_for_count \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${NEW_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} " \
  "$((NEW_WORKER_CLAIMS_BEFORE_RELAUNCH + 1))" \
  "the replacement worker's fresh ordinary APK claim"
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=app sender_pid=${APP_PID} receiver_image=android-app receiver_pid=${NEW_PID} kind=open request_id=${RELAUNCH_OPEN_REQUEST_ID} " \
  "post-complete Scene Open request ${RELAUNCH_OPEN_REQUEST_ID}"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE_RELAUNCH + 5))" \
  "five ordinary scene-relaunch transition frames"
wait_for_screenshot activity "$ACTIVITY_RELAUNCH" "the ordinarily relaunched five-node scene"
cmp "$ACTIVITY_BEFORE" "$ACTIVITY_RELAUNCH" \
  || fail_gate "The ordinary source-free relaunch changed the initial scene raster."

LAUNCHER_HOME_COUNT_BEFORE="$(
  log_count '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 '
)"
qmp tap 360 1570
wait_for_count \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ' \
  "$((LAUNCHER_HOME_COUNT_BEFORE + 1))" \
  "Home after the ordinary scene relaunch"
wait_for_pattern \
  "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${NEW_PID} receiver_image=app receiver_pid=${APP_PID} kind=closed request_id=${RELAUNCH_CLOSE_REQUEST_ID} " \
  "post-complete Scene Closed response ${RELAUNCH_CLOSE_REQUEST_ID}"
wait_for_pattern \
  "^ANDROID_APP_MULTIACTION_POST_RESTART_RELAUNCH_OK .* abi=${ABI} .* open_request_id=${RELAUNCH_OPEN_REQUEST_ID} describe_request_ids=${RELAUNCH_DESCRIBE_REQUEST_IDS} close_request_id=${RELAUNCH_CLOSE_REQUEST_ID} current_last_request_id=${RELAUNCH_CLOSE_REQUEST_ID} completed_rounds=2 post_complete_messages=${RELAUNCH_RPC_MESSAGES} .* errors=0 queues_empty=1$" \
  "the complete ordinary post-restart scene lifecycle"
stop_qemu
RECOVERY_LOG="$BOOT_NORMALIZED_LOG"
RECOVERY_AFTER_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$RECOVERY_AFTER_DISK_SHA256" == "$RECOVERY_BEFORE_DISK_SHA256" ]] \
  || fail_gate "Source-free recovery or scene interaction changed the package disk."

# Pixel proof: each independently addressed callback changes publisher pixels
# only inside the status TextView. Internal RPC revisions remain observable in
# state and logs but deliberately produce no Activity pixels; title, both
# Buttons, app bar, chrome and every other pixel remain byte-identical.
python3 - \
  "$ACTIVITY_BEFORE" "$ACTIVITY_APPROVED" "$ACTIVITY_REJECTED" \
  "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

before_path, approved_path, rejected_path, output_path = map(Path, sys.argv[1:5])

def read_ppm(path: Path) -> tuple[bytes, bytes]:
    payload = path.read_bytes()
    parts = payload.split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{path.name}: noncanonical PPM")
    if len(parts[3]) != 720 * 1600 * 3:
        raise SystemExit(f"{path.name}: wrong pixel length")
    return payload, parts[3]

before_payload, before = read_ppm(before_path)
approved_payload, approved = read_ppm(approved_path)
rejected_payload, rejected = read_ppm(rejected_path)

# Multi-action fixture: four direct root leaves, pitch=52 design pixels.
# Status is leaf ordinal 1: DRect(34, 268, 292, 48), at fixed 2x scale.
status_bounds = (68, 536, 652, 632)

def bounded_transition(
    name: str, old: bytes, new: bytes
) -> tuple[int, int]:
    stride = 720 * 3
    if old[:64 * stride] != new[:64 * stride]:
        raise SystemExit(f"{name}: trusted top chrome changed")
    if old[1512 * stride:] != new[1512 * stride:]:
        raise SystemExit(f"{name}: trusted bottom chrome changed")
    changed_total = 0
    changed_status = 0
    for y in range(1600):
        for x in range(720):
            offset = (y * 720 + x) * 3
            if old[offset:offset + 3] == new[offset:offset + 3]:
                continue
            changed_total += 1
            if (
                status_bounds[0] <= x < status_bounds[2]
                and status_bounds[1] <= y < status_bounds[3]
            ):
                changed_status += 1
    if changed_status < 100 or changed_total != changed_status:
        raise SystemExit(
            f"{name}: status-only publisher proof failed: "
            f"total={changed_total}, status={changed_status}"
        )
    return changed_total, changed_status

approved_counts = bounded_transition("approve", before, approved)
rejected_counts = bounded_transition("reject", approved, rejected)
output_path.write_text(
    "\n".join((
        f"approve_changed_pixels={approved_counts[0]}",
        f"approve_status_changed_pixels={approved_counts[1]}",
        "approve_scene_revision_changed_pixels=0",
        f"reject_changed_pixels={rejected_counts[0]}",
        f"reject_status_changed_pixels={rejected_counts[1]}",
        "reject_scene_revision_changed_pixels=0",
        "scene_revision_pixels_hidden=1",
        "approve_outside_status_publisher_changed_pixels=0",
        "reject_outside_status_publisher_changed_pixels=0",
        "title_byte_identical=1",
        "approve_button_byte_identical=1",
        "reject_button_byte_identical=1",
        "top_chrome_byte_identical=1",
        "bottom_chrome_byte_identical=1",
        f"before_sha256={sha256(before_payload).hexdigest()}",
        f"approved_sha256={sha256(approved_payload).hexdigest()}",
        f"rejected_sha256={sha256(rejected_payload).hexdigest()}",
    )) + "\n",
    encoding="utf-8",
)
PY

# Parse the serial transcript as one closed authenticated event graph.
python3 - \
  "$INSTALL_LOG" "$RECOVERY_LOG" "$PROTOCOL_EVIDENCE" \
  "$APP_PID" "$OLD_PID" "$NEW_PID" "$APK_BYTES" \
  "$ABI" "$PROTOCOL" "$PROTOCOL_VERSION" \
  "$TITLE_ID" "$STATUS_ID" "$APPROVE_ID" "$REJECT_ID" \
  "$FIRST_RPC_MESSAGES" "$RELAUNCH_RPC_MESSAGES" \
  "$REPLACEMENT_RPC_MESSAGES" "$APP_DEFINED_CALL_COUNT" <<'PY'
from pathlib import Path
import re
import sys

install_path, recovery_path, output_path = map(Path, sys.argv[1:4])
(
    app_pid, old_pid, new_pid, apk_bytes, abi, protocol, protocol_version,
    title_id, status_id, approve_id, reject_id, first_rpc_messages,
    relaunch_rpc_messages, replacement_rpc_messages, app_defined_call_count,
) = sys.argv[4:19]
install = install_path.read_text(encoding="utf-8").splitlines()
lines = recovery_path.read_text(encoding="utf-8").splitlines()
if any(line.startswith("ANDROIDBOX_SCENE_RPC2_PROFILE_OK ") for line in lines):
    raise SystemExit("recovery boot emitted the superseded ABI-49 Scene-RPC-2 profile")

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

def exact_from(
    source: list[str], prefix: str, scope: str
) -> tuple[int, dict[str, str]]:
    found = [
        (index, fields(line))
        for index, line in enumerate(source)
        if line.startswith(prefix)
    ]
    if len(found) != 1:
        raise SystemExit(
            f"{scope}: expected exactly one {prefix!r}, saw {len(found)}"
        )
    return found[0]

def exact(prefix: str) -> tuple[int, dict[str, str]]:
    return exact_from(lines, prefix, "recovery boot")

def require(actual: dict[str, str], expected: dict[str, str], context: str) -> None:
    for key, value in expected.items():
        if actual.get(key) != value:
            raise SystemExit(
                f"{context}: {key}={actual.get(key)!r}, expected {value!r}"
            )

def require_exact(
    actual: dict[str, str], expected: dict[str, str], context: str
) -> None:
    if actual == expected:
        return
    missing = sorted(set(expected) - set(actual))
    unexpected = sorted(set(actual) - set(expected))
    mismatched = sorted(
        key
        for key in set(actual) & set(expected)
        if actual[key] != expected[key]
    )
    raise SystemExit(
        f"{context}: noncanonical field set/value; "
        f"missing={missing}, unexpected={unexpected}, mismatched={mismatched}"
    )

for forbidden in (
    "MOBILE_UI_USER_FAULT ",
    "ANDROID_APP_RESTART_ARMED ",
    "ANDROID_APP_FAULT_REAP_OK ",
    "ANDROID_APP_REBIND_TRANSFER_OK ",
    "ANDROID_APP_RESTART_OK ",
    "ANDROID_APP_MULTIACTION_REOPEN_OK ",
    "ANDROID_APP_MULTIACTION_RPC_OK ",
    "ANDROID_APP_DEX_METHODS8_RPC_OK ",
    "ANDROID_APP_MULTIACTION_POST_RESTART_RELAUNCH_OK ",
    "ANDROIDBOX_SCENE_RPC2_PROFILE_OK ",
):
    if any(line.startswith(forbidden) for line in install):
        raise SystemExit(f"install boot contained {forbidden.strip()}")

install_profiles = [
    fields(line) for line in install
    if line.startswith("ANDROIDBOX_MULTIACTION3_PROFILE_OK ")
]
if len(install_profiles) != 1:
    raise SystemExit("install boot lacks exactly one MultiActionActivity-3 profile")

install_process_i, install_process = exact_from(
    install, "ANDROID_APP_PROCESS_OK ", "install boot"
)
install_profile_i = next(
    index
    for index, line in enumerate(install)
    if line.startswith("ANDROIDBOX_MULTIACTION3_PROFILE_OK ")
)
install_app_pid = install_process.get("app_pid", "")
install_worker_pid = install_process.get("android_app_pid", "")
if not (
    install_app_pid.isdecimal()
    and int(install_app_pid) > 0
    and install_worker_pid.isdecimal()
    and int(install_worker_pid) > 0
    and install_app_pid != install_worker_pid
):
    raise SystemExit("install boot has invalid App/AndroidApp process identities")
install_rpc_reads = [
    (index, fields(line))
    for index, line in enumerate(install)
    if line.startswith("ANDROID_APP_RPC_READ_OK ")
]
if len(install_rpc_reads) != 1:
    raise SystemExit(
        "install boot must contain exactly one worker Ready RPC, "
        f"saw {len(install_rpc_reads)} RPC reads"
    )
install_ready_i, install_ready = install_rpc_reads[0]
require_exact(install_ready, {
    "sender_image": "android-app", "sender_pid": install_worker_pid,
    "receiver_image": "app", "receiver_pid": install_app_pid,
    "kind": "ready", "request_id": "0", "arg0": abi, "arg1": "0",
    "payload_bytes": "0", "chunk_offset": "0", "chunk_total": abi,
    "first_round": "1", "reads": "1", "errors": "0",
    "phase": "idle", "complete": "0",
}, "install boot Ready RPC")
if not install_ready_i < install_process_i < install_profile_i:
    raise SystemExit("install Ready/process/profile markers are out of canonical order")

process_i, process = exact("ANDROID_APP_PROCESS_OK ")
preview_i, preview = exact("MOBILE_UI_PREVIEW_OK ")
process_profile_i, process_profile = exact("ANDROIDBOX_PROCESS0_PROFILE_OK ")
scene_profile_i, scene_profile = exact("ANDROIDBOX_MULTIACTION3_PROFILE_OK ")
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
reopen_i, reopen_marker = exact("ANDROID_APP_MULTIACTION_REOPEN_OK ")
restart_i, restart = exact("ANDROID_APP_RESTART_OK ")
rpc_i, rpc = exact("ANDROID_APP_MULTIACTION_RPC_OK ")
if app_defined_call_count == "1":
    dex_methods_rpc_i, dex_methods_rpc = exact("ANDROID_APP_DEX_METHODS8_RPC_OK ")
else:
    dex_methods_rpc_i, dex_methods_rpc = None, None
post_relaunch_i, post_relaunch = exact(
    "ANDROID_APP_MULTIACTION_POST_RESTART_RELAUNCH_OK "
)

rpc_order_valid = (
    rpc_i < post_relaunch_i
    if dex_methods_rpc_i is None
    else rpc_i < dex_methods_rpc_i < post_relaunch_i
)
if not (
    process_i < armed_i < fault_i < reap_i < rebind_i < ready_i < grant_i
    < rebound_i < reopen_i < restart_i < rpc_i
    and rpc_order_valid
):
    raise SystemExit("MultiActionActivity-3 restart markers are out of canonical order")

require(install_profiles[0], {
    "format": "1", "abi": abi, "parent_profile": "androidbox-scene-rpc2",
    "protocol": protocol, "protocol_version": protocol_version,
    "scene_max_nodes": "8", "node_text_max_bytes": "96",
    "scene_transport": "pull", "describe_order": "preorder",
    "describe_max_outstanding": "1", "node_descriptor_bytes": "16",
    "supported_views": "LinearLayout+TextView+Button",
    "supported_orientation": "vertical", "callback_buttons": "1-4",
    "fixture_callback_buttons": "2",
    "callback_dispatch": "view-getId+bounded-if-eq-if-ne",
    "callback_cfg": "forward-acyclic",
    "update_target": "existing-TextView",
    "publisher_tree_renderer": "trusted-app-el0",
    "art": "0", "activitythread": "0", "binder": "0", "bionic": "0",
    "jni": "0", "native_lib": "0", "general_apk_claim": "0",
    "general_android_compatibility_claim": "0", "network": "disabled",
    "emulator_only": "1", "real_phone_claim": "0",
}, "install Scene profile")
require(scene_profile, install_profiles[0], "recovery Scene profile")
require(preview, {
    "abi": abi, "width": "720", "height": "1600", "buffers": "3",
    "live_processes": "9", "network": "disabled", "real_phone_claim": "0",
}, "preview")
require(process_profile, {
    "format": "1", "abi": abi, "parent_profile": "androidbox-interactive0",
    "rpc_protocol": protocol, "rpc_transport": "private-kernel-stamped-channel",
    "rpc_max_outstanding": "1", "execution_host": "independent-android-app-el0",
    "standalone_android_process": "1", "dedicated_android_process": "1",
    "trusted_ui_host": "app-el0", "android_app_surface_handles": "0",
    "android_app_graphics_handles": "0", "android_app_input_handles": "0",
    "android_app_storage_handles": "0", "android_app_apk_digest_reverified": "1",
    "android_app_apk_v2_signature_reverified": "1",
    "android_app_signer_reverified": "1", "art": "0", "binder": "0",
    "bionic": "0", "jni": "0", "native_lib": "0",
    "general_android_compatibility_claim": "0", "network": "disabled",
    "emulator_only": "1", "real_phone_claim": "0",
}, "process profile")
require(restart_profile, {
    "format": "1", "abi": abi, "parent_profile": "androidbox-process0",
    "restart_owner": "init", "restart_budget": "1",
    "controlled_worker_fault": "el0-data-abort",
    "rebind_transport": "retained-authenticated-bootstrap",
    "same_slot_required": "1", "generation_step_required": "1",
    "replacement_ready_required": "1", "completed_open_replayed": "1",
    "ambiguous_request_replay": "0", "recovery_scope": "one-android-app-worker",
    "general_process_supervisor": "0", "general_crash_recovery_claim": "0",
    "general_android_compatibility_claim": "0", "network": "disabled",
    "emulator_only": "1", "real_phone_claim": "0",
}, "restart profile")

require(process, {
    "format": "1", "abi": abi, "app_pid": app_pid,
    "android_app_pid": old_pid, "app_image": "7", "android_app_image": "10",
    "elf_distinct": "1", "pid_distinct": "1", "asid_distinct": "1",
    "root_distinct": "1", "live_processes": "9", "android_app_live": "1",
    "worker_handles": "1", "worker_channel_count": "1",
    "worker_unexpected_handle_count": "0", "endpoints_unique": "1",
    "worker_rights_valid": "1", "app_rights_valid": "1",
    "queues_empty": "1", "objects_valid": "1", "isolation_valid": "1",
    "valid": "1", "surface_handles": "0", "graphics_handles": "0",
    "input_handles": "0", "storage_handles": "0",
}, "initial process")

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
    raise SystemExit("controlled MultiActionActivity-3 data-abort tuple is not exact")
if not (0x0000000200000000 <= int(fault_match.group(1), 16) < 0x0000000200200000):
    raise SystemExit("controlled fault ELR is outside AndroidApp code")
if not (0x00000002001ef000 <= int(fault_match.group(2), 16) < 0x00000002001ff000):
    raise SystemExit("controlled fault SP is outside the mapped user stack")

require(armed, {
    "format": "1", "abi": abi, "app_pid": app_pid, "old_pid": old_pid,
    "session_id": "1", "package_generation": "1", "request_id": "7",
    "fault_point": "after-open-before-ui-commit", "restart_budget": "1",
}, "restart armed")
require(reap, {
    "format": "1", "abi": abi, "epoch": "1", "old_pid": old_pid,
    "exit_code": "0", "termination_reason": "faulted", "fault_reason": "2",
    "esr": "0x0000000092000047", "far": "0x00000002001ee000",
    "elr": fault_match.group(1), "sp": fault_match.group(2),
    "witness_valid": "1", "escrow_retained": "1",
}, "fault reap")
require(rebind, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "old_pid": old_pid, "new_pid": new_pid, "exit_code": "0",
    "termination_reason": "2", "same_slot": "1", "generation_step": "1",
    "endpoint_transfer": "1",
}, "rebind")
require(ready, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "old_pid": old_pid, "new_pid": new_pid, "request_reset": "1",
    "ready_request_id": "0",
}, "replacement ready")
require(grant, {
    "format": "1", "abi": abi, "epoch": "1", "old_owner": old_pid,
    "new_owner": new_pid, "compatible_session_id": "1",
    "package_generation": "1", "request_id": "1",
    "trigger": "authenticated-replacement-open", "immutable_vmo": "1",
    "same_vmo": "1", "same_slot": "1", "generation_step": "1",
    "reissue": "1/1", "apk_bytes_exposed_to_init": "0",
    "apk_bytes_exposed_to_app": "0",
}, "replacement grant")
require(rebound, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "old_pid": old_pid, "new_pid": new_pid, "authenticated": "1",
}, "rebound")
require(reopen_marker, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "new_pid": new_pid, "session_id": "1", "package_generation": "1",
    "open_request_id": "1", "node_count": "5", "describe_requests": "5",
    "node_responses": "5", "node_text_chunks": "4",
    "scene_text_bytes": "44", "text_views": "2", "callback_buttons": "2",
    "first_button_id": approve_id, "second_button_id": reject_id,
    "first_button_bytes": "7", "second_button_bytes": "6",
    "last_request_id": "6", "phase": "active",
    "completed_open_replayed": "1", "ambiguous_request_replay": "0",
}, "scene reopen")
require(restart, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "old_pid": old_pid, "new_pid": new_pid, "session_id": "1",
    "package_generation": "1", "crash_request_id": "7", "exit_code": "0",
    "termination_reason": "faulted", "fault_reap": "1",
    "peer_close_observed": "1", "endpoint_transfer": "1",
    "replacement_ready": "1", "rebound_ack": "1", "same_slot": "1",
    "generation_step": "1", "restart_budget": "1/1", "image_escrow": "1",
    "image_reissue": "1", "replacement_image_claim": "1",
    "completed_open_replayed": "1", "ambiguous_request_replay": "0",
    "app_graphics_present": "1", "app_enter_frames": "5",
    "app_layered_commits": "5", "errors": "0",
}, "restart summary")
require(rpc, {
    "format": "1", "abi": abi, "protocol": protocol,
    "protocol_version": protocol_version,
    "app_pid": app_pid, "android_app_pid": new_pid,
    "ready_request_id": "0", "open_request_id": "1",
    "describe_request_ids": "2/3/4/5/6", "click_request_ids": "7/8",
    "close_request_id": "9", "request_order": "0/1/2/3/4/5/6/7/8/9",
    "app_sender_authenticated": "1", "worker_sender_authenticated": "1",
    "ready": "1", "open": "1", "scene_opened": "1", "node_count": "5",
    "describe_requests": "5", "node_responses": "5",
    "node_text_chunks": "4", "scene_text_bytes": "44", "text_views": "2",
    "callback_buttons": "2", "title_id": title_id, "status_id": status_id,
    "approve_button_id": approve_id, "reject_button_id": reject_id,
    "title_bytes": "14", "approve_button_bytes": "7",
    "reject_button_bytes": "6", "click": "2",
    "clicked_button_ids": f"{approve_id}/{reject_id}",
    "updated": "2", "update_chunks": "2",
    "update_view_ids": f"{status_id}/{status_id}",
    "update_chunk_bytes": "18/18", "update_bytes": "36",
    "final_revision": "2", "close": "1", "closed": "1",
    "requests": "9", "responses": "10", "chunks": "6",
    "first_round_messages": first_rpc_messages,
    "errors": "0", "max_outstanding": "1",
    "queue_capacity": "8", "queues_empty": "1",
}, "MultiActionActivity-3 summary")
if dex_methods_rpc is not None:
    require_exact(dex_methods_rpc, {
        "format": "1", "abi": abi, "protocol": protocol,
        "protocol_version": protocol_version,
        "app_pid": app_pid, "android_app_pid": new_pid,
        "clicked_button_ids": f"{approve_id}/{reject_id}",
        "update_view_ids": f"{status_id}/{status_id}",
        "app_defined_calls": "1/1", "final_revision": "2",
        "errors": "0", "queues_empty": "1",
    }, "DexMethods-8 summary")
require(post_relaunch, {
    "format": "1", "abi": abi, "epoch": "1", "app_pid": app_pid,
    "android_app_pid": new_pid, "replacement_pid": new_pid,
    "open_request_id": "10", "close_request_id": "16",
    "describe_request_ids": "11/12/13/14/15",
    "current_last_request_id": "16", "completed_rounds": "2",
    "post_complete_messages": relaunch_rpc_messages,
    "reads": replacement_rpc_messages, "phase": "idle",
    "worker_still_replacement": "1", "errors": "0", "queues_empty": "1",
}, "post-restart relaunch")

claims = [(i, fields(line)) for i, line in enumerate(lines)
          if line.startswith("ANDROID_PACKAGE_IMAGE_CLAIM_OK ")]
if len(claims) != 3:
    raise SystemExit(f"expected exactly three package image claims, saw {len(claims)}")
if [claim["owner"] for _, claim in claims] != [old_pid, new_pid, new_pid]:
    raise SystemExit("package claims do not belong to old/replacement/replacement")
for _, claim in claims:
    require(claim, {
        "compatible_session_id": "1", "generation": "1",
        "apk_length": apk_bytes, "image": "android-app",
        "apk_bytes_exposed_to_app": "0", "apk_bytes_exposed_to_android_app": "1",
        "rights": "READ", "duplicate": "0", "transfer": "0", "map": "0",
        "write": "0", "execute": "0", "wait": "0",
        "storage_authority_granted": "0",
    }, "package image claim")
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

app_commit_indices = [
    i for i, line in enumerate(lines)
    if line.startswith("USER_SURFACE_LAYERED_COMMIT_OK ")
    and f" content_producer_pid={app_pid} " in line
]
if len(app_commit_indices) != 14:
    raise SystemExit(f"expected exactly fourteen App commits, saw {len(app_commit_indices)}")
if not (app_commit_indices[4] < restart_i < app_commit_indices[5]):
    raise SystemExit("restart success was not bound to exactly five scene commits")
if not (
    app_commit_indices[8] < rpc_i < app_commit_indices[9]
    and app_commit_indices[13] < post_relaunch_i
):
    raise SystemExit("ordinary scene relaunch commits are outside its lifecycle")

rpc_reads = [
    (i, fields(line)) for i, line in enumerate(lines)
    if line.startswith("ANDROID_APP_RPC_READ_OK ")
    and (
        f"sender_pid={new_pid} " in line
        or f"receiver_pid={new_pid} " in line
    )
]

def expected_rpc(
    sender_image: str,
    kind: str,
    request_id: int,
    arg0: int,
    arg1: int,
    payload_bytes: int,
    first_round: bool,
    reads: int,
    phase: str,
    complete: bool,
) -> dict[str, str]:
    if sender_image == "app":
        sender_pid = app_pid
        receiver_image = "android-app"
        receiver_pid = new_pid
    elif sender_image == "android-app":
        sender_pid = new_pid
        receiver_image = "app"
        receiver_pid = app_pid
    else:
        raise AssertionError(f"unknown sender image: {sender_image}")
    return {
        "sender_image": sender_image,
        "sender_pid": sender_pid,
        "receiver_image": receiver_image,
        "receiver_pid": receiver_pid,
        "kind": kind,
        "request_id": str(request_id),
        "arg0": str(arg0),
        "arg1": str(arg1),
        "payload_bytes": str(payload_bytes),
        # The trace deliberately prints these projections for every message,
        # not only text chunks.
        "chunk_offset": str(arg0 >> 32),
        "chunk_total": str(arg0 & 0xffff_ffff),
        "first_round": str(int(first_round)),
        "reads": str(reads),
        "errors": "0",
        "phase": phase,
        "complete": str(int(complete)),
    }

scene_nodes = (
    (0, 0),
    (1, 14),
    (2, 17),
    (3, 7),
    (4, 6),
)

def expected_scene_round(
    *,
    open_request_id: int,
    describe_request_ids: tuple[int, ...],
    close_request_id: int,
    first_read: int,
    first_round: bool,
    already_complete: bool,
    include_ready: bool,
    include_callbacks: bool,
) -> list[dict[str, str]]:
    expected: list[dict[str, str]] = []
    read = first_read

    def append(
        sender_image: str,
        kind: str,
        request_id: int,
        arg0: int,
        arg1: int,
        payload_bytes: int,
        phase: str,
        complete: bool = already_complete,
    ) -> None:
        nonlocal read
        expected.append(expected_rpc(
            sender_image,
            kind,
            request_id,
            arg0,
            arg1,
            payload_bytes,
            first_round,
            read,
            phase,
            complete,
        ))
        read += 1

    if include_ready:
        append("android-app", "ready", 0, int(abi), 0, 0, "idle")
    append("app", "open", open_request_id, 1, 1, 0, "await-opened")
    append(
        "android-app",
        "scene-opened",
        open_request_id,
        len(scene_nodes),
        0,
        0,
        "await-node-request",
    )
    if len(describe_request_ids) != len(scene_nodes):
        raise AssertionError("one DescribeNode request is required per scene node")
    for (index, text_total), request_id in zip(
        scene_nodes, describe_request_ids, strict=True
    ):
        append("app", "describe-node", request_id, 1, index, 0, "await-node")
        append(
            "android-app",
            "node",
            request_id,
            index,
            0,
            16,
            "node-chunks" if text_total else "await-node-request",
        )
        if text_total:
            append(
                "android-app",
                "node-text-chunk",
                request_id,
                text_total,
                index,
                text_total,
                "active" if index == scene_nodes[-1][0] else "await-node-request",
            )
    if include_callbacks:
        append(
            "app",
            "click",
            7,
            1,
            int(approve_id) << 32,
            0,
            "await-updated",
        )
        append(
            "android-app",
            "updated",
            7,
            (int(app_defined_call_count) << 32) | int(status_id),
            (1 << 32) | 18,
            0,
            "update-chunks",
        )
        append("android-app", "update-text-chunk", 7, 18, 0, 18, "active")
        append(
            "app",
            "click",
            8,
            1,
            (int(reject_id) << 32) | 1,
            0,
            "await-updated",
        )
        append(
            "android-app",
            "updated",
            8,
            (int(app_defined_call_count) << 32) | int(status_id),
            (2 << 32) | 18,
            0,
            "update-chunks",
        )
        append("android-app", "update-text-chunk", 8, 18, 0, 18, "active")
    append("app", "close", close_request_id, 1, 1, 0, "await-closed")
    append(
        "android-app",
        "closed",
        close_request_id,
        0,
        0,
        0,
        "idle",
        True,
    )
    return expected

first_expected = expected_scene_round(
    open_request_id=1,
    describe_request_ids=(2, 3, 4, 5, 6),
    close_request_id=9,
    first_read=1,
    first_round=True,
    already_complete=False,
    include_ready=True,
    include_callbacks=True,
)
relaunch_expected = expected_scene_round(
    open_request_id=10,
    describe_request_ids=(11, 12, 13, 14, 15),
    close_request_id=16,
    first_read=26,
    first_round=False,
    already_complete=True,
    include_ready=False,
    include_callbacks=False,
)
expected = first_expected + relaunch_expected
if len(rpc_reads) != len(expected):
    raise SystemExit(
        f"expected {replacement_rpc_messages} replacement RPC reads, "
        f"saw {len(rpc_reads)}"
    )
for ordinal, ((index, actual), wanted) in enumerate(zip(rpc_reads, expected, strict=True)):
    require_exact(actual, wanted, f"replacement RPC {ordinal + 1} at line {index + 1}")

first_reads = rpc_reads[:len(first_expected)]
if not (
    reopen_i < first_reads[17][0] < first_reads[20][0] < rpc_i
):
    raise SystemExit(
        "the two clicks are not ordered after scene reopen and before summary"
    )
if not (rpc_i < rpc_reads[len(first_expected)][0] < post_relaunch_i):
    raise SystemExit("ordinary relaunch RPC sequence is not after first completion")

if any(
    old_pid in line
    for line in lines[fault_i + 1:]
    if line.startswith("ANDROID_APP_RPC_READ_OK ")
):
    raise SystemExit("old worker identity appeared in RPC after its fault")

output_path.write_text(
    "\n".join((
        f"app_pid={app_pid}",
        f"old_android_app_pid={old_pid}",
        f"new_android_app_pid={new_pid}",
        f"protocol={protocol}",
        "scene_nodes=5",
        "scene_text_bytes=44",
        "text_views=2",
        "callback_buttons=2",
        f"title_id={title_id}",
        f"status_id={status_id}",
        f"approve_button_id={approve_id}",
        f"reject_button_id={reject_id}",
        "approve_update=Decision:_approved",
        "reject_update=Decision:_rejected",
        "update_views=status/status",
        "install_rpc_messages=1",
        f"first_rpc_messages={first_rpc_messages}",
        f"relaunch_rpc_messages={relaunch_rpc_messages}",
        f"replacement_rpc_messages={replacement_rpc_messages}",
        "replacement_rpc_fields_closed=1",
        "same_slot=1",
        "generation_step=1",
        "controlled_faults=1",
        "image_claims=3",
        "same_vmo_reissue=1",
        "app_commits=14",
        "rpc_errors=0",
        f"app_defined_calls={app_defined_call_count}/{app_defined_call_count}",
    )) + "\n",
    encoding="utf-8",
)
PY

BEFORE_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_BEFORE" | awk '{print $1}')"
APPROVED_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_APPROVED" | awk '{print $1}')"
REJECTED_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_REJECTED" | awk '{print $1}')"
RELAUNCH_PPM_SHA256="$(shasum -a 256 "$ACTIVITY_RELAUNCH" | awk '{print $1}')"
APPROVE_STATUS_CHANGED="$(
  sed -n 's/^approve_status_changed_pixels=//p' "$RASTER_EVIDENCE"
)"
REJECT_STATUS_CHANGED="$(
  sed -n 's/^reject_status_changed_pixels=//p' "$RASTER_EVIDENCE"
)"

printf '%s\n' \
  "$TERMINAL artifact_dir=$ARTIFACT_DIR abi=$ABI scope=bounded-multiactionactivity3-scene-rpc qemu_boots=2 qemu_nic_none_per_boot=1 install_source=qemu-fw_cfg recovery_source=none apk_bytes=$APK_BYTES apk_sha256=$APK_SHA256 signer_cert_sha256=$EXPECTED_SIGNER_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE app_pid=$APP_PID old_android_app_pid=$OLD_PID new_android_app_pid=$NEW_PID protocol=$PROTOCOL protocol_version=$PROTOCOL_VERSION scene_nodes=$SCENE_NODES scene_text_bytes=$SCENE_TEXT_BYTES text_views=$SCENE_TEXT_VIEWS callback_buttons=$CALLBACK_BUTTONS title_id=$TITLE_ID status_id=$STATUS_ID approve_button_id=$APPROVE_ID reject_button_id=$REJECT_ID approve_update=Decision:_approved reject_update=Decision:_rejected update_views=status/status app_defined_calls=$APP_DEFINED_CALL_COUNT/$APP_DEFINED_CALL_COUNT app_defined_method=$([[ "$APP_DEFINED_CALL_COUNT" == 1 ]] && printf private-static-I-to-I || printf none) approve_status_changed_pixels=$APPROVE_STATUS_CHANGED reject_status_changed_pixels=$REJECT_STATUS_CHANGED approve_outside_status_publisher_changed_pixels=0 reject_outside_status_publisher_changed_pixels=0 same_slot=1 generation_step=1 controlled_faults=1 fault_witness_bound=1 fault_reaps=1 endpoint_rebinds=1 image_claims=3 image_escrows=1 image_reissues=1 same_vmo=1 reopened=1 post_restart_relaunch=1 ambiguous_request_replay=0 app_layered_commits=$TOTAL_APP_COMMITS restart_bound_app_commits=5 install_rpc_messages=1 first_rpc_messages=$FIRST_RPC_MESSAGES relaunch_rpc_messages=$RELAUNCH_RPC_MESSAGES replacement_rpc_messages=$REPLACEMENT_RPC_MESSAGES replacement_rpc_fields_closed=1 rpc_request_order=0/1/2/3/4/5/6/7/8/9 post_restart_request_order=10/11/12/13/14/15/16 rpc_click=2 rpc_close=2 rpc_errors=0 before_ppm_sha256=$BEFORE_PPM_SHA256 approved_ppm_sha256=$APPROVED_PPM_SHA256 rejected_ppm_sha256=$REJECTED_PPM_SHA256 relaunch_ppm_sha256=$RELAUNCH_PPM_SHA256 relaunch_ppm_identical=1 recovery_disk_unchanged=1 recovery_disk_sha256=$RECOVERY_AFTER_DISK_SHA256 network=disabled unexpected_faults=0 restart_timeout=armed panic_fatal_free=1 art=0 dalvik=0 binder=0 general_android_compatibility_claim=0 real_phone_claim=0" \
  | tee "$SUMMARY"

trap - EXIT
