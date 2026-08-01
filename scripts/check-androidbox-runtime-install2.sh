#!/usr/bin/env bash
set -euo pipefail

# Strict offline ABI-53 local sideload/install/update acceptance gate.
#
# The gate builds two real SDK APKs with one repository test signer, boots a
# virgin package store with v1 as immutable fw_cfg input, proves that the first
# Settings tap is confirmation-only, installs on the second tap, launches the
# package in the isolated AndroidApp process, recovers it without a source,
# updates v1 -> v2 with the same two-step contract, and finally recovers v2
# source-free. Every VM has networking disabled and cleanup owns only `$!`.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if (($#)); then
  printf '%s\n' 'Usage: check-androidbox-runtime-install2.sh' >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed head; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the runtime-install gate." >&2
    exit 1
  }
done

FIXTURE_DIR="${BNDROID_INSTALL_GATE_FIXTURE_DIR:-"$WORKSPACE_ROOT/fixtures/androidbox-envelope-demo"}"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="${BNDROID_INSTALL_GATE_STORAGE_BUILDER:-"$SCRIPT_DIR/build-storage-image.sh"}"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in "$FIXTURE_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$QMP_HELPER"; do
  [[ -x "$helper" ]] || {
    echo "Required helper is not executable: $helper" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-900}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}
SETTINGS_ENTER_FRAMES="${BNDROID_INSTALL_GATE_SETTINGS_ENTER_FRAMES:-5}"
[[ "$SETTINGS_ENTER_FRAMES" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_INSTALL_GATE_SETTINGS_ENTER_FRAMES must be a positive integer." >&2
  exit 2
}
MULTIPACKAGE_STORAGE="${BNDROID_INSTALL_GATE_MULTIPACKAGE_STORAGE:-0}"
[[ "$MULTIPACKAGE_STORAGE" == 0 || "$MULTIPACKAGE_STORAGE" == 1 ]] || {
  echo "BNDROID_INSTALL_GATE_MULTIPACKAGE_STORAGE must be 0 or 1." >&2
  exit 2
}

ABI="${BNDROID_INSTALL_GATE_ABI:-53}"
PACKAGE="${BNDROID_INSTALL_GATE_PACKAGE:-org.bndroid.envelope}"
ACTIVITY="${BNDROID_INSTALL_GATE_ACTIVITY:-Lorg/bndroid/envelope/MainActivity;}"
SIGNER_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
FEATURES="${BNDROID_INSTALL_GATE_FEATURES:-androidbox-runtime-install2}"
EXTRA_PROFILE_PATTERN="${BNDROID_INSTALL_GATE_EXTRA_PROFILE_PATTERN:-}"
GATE_OK="${BNDROID_INSTALL_GATE_OK:-ANDROIDBOX_RUNTIME_INSTALL2_QEMU_OK}"
ARTIFACT_ROOT="${BNDROID_INSTALL_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-runtime-install2"}"

mkdir -p "$ARTIFACT_ROOT"
ARTIFACT_DIR="$(
  mktemp -d "$ARTIFACT_ROOT/check.XXXXXX"
)"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

BASE_APK="$ARTIFACT_DIR/envelope-v1.apk"
UPDATE_APK="$ARTIFACT_DIR/envelope-v2.apk"
BASE_BUILD_LOG="$ARTIFACT_DIR/envelope-v1-build.log"
UPDATE_BUILD_LOG="$ARTIFACT_DIR/envelope-v2-build.log"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_LOG="$ARTIFACT_DIR/storage-build.log"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
TARGET_ROOT="${BNDROID_INSTALL_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-runtime-install2-build"}"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
SUMMARY="$ARTIFACT_DIR/summary.txt"

INSTALL_READY="$ARTIFACT_DIR/install-ready.ppm"
INSTALL_CONFIRM="$ARTIFACT_DIR/install-confirm.ppm"
INSTALL_DONE="$ARTIFACT_DIR/install-done.ppm"
INSTALL_DRAWER="$ARTIFACT_DIR/install-drawer.ppm"
INSTALL_ACTIVITY="$ARTIFACT_DIR/install-activity.ppm"
RECOVERY_DRAWER="$ARTIFACT_DIR/recovery-drawer.ppm"
RECOVERY_ACTIVITY="$ARTIFACT_DIR/recovery-activity.ppm"
UPDATE_READY="$ARTIFACT_DIR/update-ready.ppm"
UPDATE_CONFIRM="$ARTIFACT_DIR/update-confirm.ppm"
UPDATE_DONE="$ARTIFACT_DIR/update-done.ppm"
UPDATE_DRAWER="$ARTIFACT_DIR/update-drawer.ppm"
UPDATE_ACTIVITY="$ARTIFACT_DIR/update-activity.ppm"

QEMU_PID=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""
BOOT_QMP_SOCKET=""
QEMU_STARTS=0
QEMU_NETWORK_DISABLED_STARTS=0

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
  echo "Runtime-install evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_bad_output() {
  normalize_log
  local bad='fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_CHILD_DIAG|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROID_APP_RESTART_TIMEOUT|ANDROIDBOX_RUNTIME_INSTALL2_PROFILE_FAIL|ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_FAIL'
  if grep -Eqi "$bad" "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fatal, child failure, or install failure."
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
    if (($(grep -Ec "$pattern" "$BOOT_NORMALIZED_LOG" || true) >= minimum)); then
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

canonical_ppm() {
  local screenshot="$1"
  python3 - "$screenshot" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
parts = path.read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)
if any(pixels[(y * 720 + x) * 3:(y * 720 + x) * 3 + 3] != b"\0\0\0"
       for x, y in ((0, 0), (719, 0), (0, 1599), (719, 1599))):
    raise SystemExit(1)
if len({pixels[index:index + 3] for index in range(0, len(pixels), 3)}) < 80:
    raise SystemExit(1)
PY
}

take_screenshot() {
  local screenshot="$1"
  local different_from="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  qmp move 700 1500
  while ((SECONDS < deadline)); do
    reject_bad_output
    qmp screenshot "$screenshot"
    if canonical_ppm "$screenshot" \
      && { [[ -z "$different_from" ]] || ! cmp -s "$different_from" "$screenshot"; }; then
      return
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

start_qemu() {
  local mode="$1"
  local source_apk="$2"
  local -a source_args=(-name "Bndroid Runtime Install-2 Gate $mode")
  if [[ -n "$source_apk" ]]; then
    source_args+=(-fw_cfg "name=opt/bndroid/apk,file=$source_apk")
  fi

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
    -rtc base=2026-07-30T19:30:00,clock=vm \
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
  QEMU_STARTS=$((QEMU_STARTS + 1))
  QEMU_NETWORK_DISABLED_STARTS=$((QEMU_NETWORK_DISABLED_STARTS + 1))
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

boot_ready() {
  wait_for_pattern "^MOBILE_UI_PREVIEW_OK .*abi=${ABI} width=720 height=1600 " \
    "the ABI-53 720x1600 preview"
  wait_for_pattern "^ANDROID_APP_PROCESS_OK .*abi=${ABI} " \
    "the isolated AndroidApp topology"
  normalize_log
  APP_PID="$(
    sed -n 's/^ANDROID_APP_PROCESS_OK .* app_pid=\([1-9][0-9]*\) .*/\1/p' \
      "$BOOT_NORMALIZED_LOG" | head -n 1
  )"
  LAUNCHER_PID="$(
    sed -n 's/^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=\([1-9][0-9]*\) .* frame_id=1 .*/\1/p' \
      "$BOOT_NORMALIZED_LOG" | head -n 1
  )"
  [[ "$APP_PID" =~ ^[1-9][0-9]*$ && "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ ]] \
    || fail_gate "Could not derive App and Launcher process identities."
}

unlock_home() {
  qmp drag 360 1390 360 620
  wait_for_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=none ' \
    "the unlocked Home state"
}

open_settings_apps() {
  local screenshot="$1"
  local different_from="$2"
  local description="$3"
  local before
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 615 1435
  wait_for_pattern \
    '^UI_ROUTE_FOCUS_OK .* receiver_image=app .* active_client=app app=settings ' \
    "Settings focus"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + SETTINGS_ENTER_FRAMES))" \
    "the settled Settings enter transition"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 360 1350
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the Apps page frames"
  take_screenshot "$screenshot" "$different_from" "$description"
}

first_confirmation_tap() {
  local x="$1"
  local y="$2"
  local screenshot="$3"
  local different_from="$4"
  local before
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap "$x" "$y"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the first-tap confirmation frames"
  take_screenshot "$screenshot" "$different_from" "the confirmation raster"
  [[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK ')" == 0 ]] \
    || fail_gate "The first tap performed a durable package mutation."
}

confirm_install() {
  local action="$1"
  local previous_generation="$2"
  local installed_generation="$3"
  local version_code="$4"
  local screenshot="$5"
  local different_from="$6"
  qmp tap 515 1070
  wait_for_pattern \
    "^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK owner=${APP_PID} request_sequence=1 .* action=${action} previous_generation=${previous_generation} installed_generation=${installed_generation} version_code=${version_code} package=${PACKAGE} reads=[1-9][0-9]* writes=[1-9][0-9]* flushes=[1-9][0-9]* source=immutable-fw-cfg network=disabled$" \
    "the durable $action transaction"
  wait_for_pattern \
    "^ANDROID_PACKAGE_RUNTIME_INSTALL_COLLECT_OK owner=${APP_PID} request_sequence=1 .* previous_generation=${previous_generation} installed_generation=${installed_generation} version_code=${version_code} bytes=256 reads=[1-9][0-9]* writes=[1-9][0-9]* flushes=[1-9][0-9]* authority_granted=0 apk_bytes_exposed=0 package_store_handle=0 block_handle=0 arbitrary_path=0$" \
    "the exact App-owned terminal result"
  if [[ "$MULTIPACKAGE_STORAGE" == 1 ]]; then
    wait_for_pattern \
      '^ANDROID_PACKAGE_DIRECTORY_READ_OK image=7 bytes=1344 revision=[1-9][0-9]* installed_count=1 authority_granted=0 apk_bytes_exposed=0$' \
      "the live installed package directory"
  else
    wait_for_pattern \
      "^ANDROID_PACKAGE_SNAPSHOT_READ_OK image=7 bytes=640 installed=1 generation=${installed_generation} version_code=${version_code} package_writes=0 authority_granted=0 apk_bytes_exposed=0$" \
      "the live installed catalog"
  fi
  take_screenshot "$screenshot" "$different_from" "the installed terminal raster"
}

launch_from_home() {
  local generation="$1"
  local drawer="$2"
  local activity="$3"
  local from_settings="$4"
  local before
  if [[ "$from_settings" == 1 ]]; then
    qmp tap 360 1570
    wait_for_pattern \
      '^UI_ROUTE_FOCUS_OK .* receiver_image=launcher .* active_client=launcher app=none ' \
      "Launcher focus after Settings"
    if [[ "$MULTIPACKAGE_STORAGE" == 1 ]]; then
      wait_for_pattern \
        '^ANDROID_PACKAGE_DIRECTORY_READ_OK image=6 bytes=1344 revision=[1-9][0-9]* installed_count=1 authority_granted=0 apk_bytes_exposed=0$' \
        "the refreshed Launcher package directory"
    else
      wait_for_pattern \
        "^ANDROID_PACKAGE_SNAPSHOT_READ_OK image=6 bytes=640 installed=1 generation=${generation} " \
        "the refreshed Launcher catalog"
    fi
  fi
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} ")"
  qmp drag 360 1280 360 520
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
    "$((before + 4))" \
    "the installed All apps drawer"
  take_screenshot "$drawer" "" "the installed All apps row"
  qmp tap 106 674
  wait_for_pattern \
    "^ANDROID_PACKAGE_DURABLE_LAUNCH_OK owner=${LAUNCHER_PID} request_sequence=1 generation=${generation} .* package=${PACKAGE} activity=${ACTIVITY} " \
    "the durable generation-$generation launch"
  wait_for_pattern \
    "^ANDROID_APP_RESTART_OK .*abi=${ABI} .* package_generation=${generation} .* app_enter_frames=5 app_layered_commits=5 errors=0$" \
    "the isolated AndroidApp recovery and Activity surface"
  take_screenshot "$activity" "$drawer" "the real APK Activity raster"
}

# Build reproducible v1 and v2 APKs only with the installed Android SDK.
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$BASE_APK" \
  "$FIXTURE_BUILD" >"$BASE_BUILD_LOG" 2>&1 \
  || { tail -n 180 "$BASE_BUILD_LOG" >&2; exit 1; }
BNDROID_ENVELOPE_VERSION_CODE=2 \
BNDROID_ENVELOPE_VERSION_NAME=2.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$UPDATE_APK" \
  "$FIXTURE_BUILD" >"$UPDATE_BUILD_LOG" 2>&1 \
  || { tail -n 180 "$UPDATE_BUILD_LOG" >&2; exit 1; }

BASE_BYTES="$(wc -c <"$BASE_APK" | tr -d '[:space:]')"
UPDATE_BYTES="$(wc -c <"$UPDATE_APK" | tr -d '[:space:]')"
BASE_SHA256="$(shasum -a 256 "$BASE_APK" | awk '{print $1}')"
UPDATE_SHA256="$(shasum -a 256 "$UPDATE_APK" | awk '{print $1}')"
[[ "$BASE_BYTES" =~ ^[1-9][0-9]*$ && "$UPDATE_BYTES" =~ ^[1-9][0-9]*$ \
  && "$BASE_SHA256" =~ ^[0-9a-f]{64}$ && "$UPDATE_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$BASE_SHA256" != "$UPDATE_SHA256" ]] \
  || { echo "The two SDK APK artifacts are not distinct canonical inputs." >&2; exit 1; }

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 320 "$BUILD_LOG" >&2
  echo "ABI-$ABI Runtime Install-2 child build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || { echo "Kernel image missing." >&2; exit 1; }

if [[ "$MULTIPACKAGE_STORAGE" == 1 ]]; then
  STORAGE_BUILD_COMMAND=("$BUILD_STORAGE")
else
  STORAGE_BUILD_COMMAND=("$BUILD_STORAGE" --with-package-store)
fi
if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "${STORAGE_BUILD_COMMAND[@]}" >"$STORAGE_LOG" 2>&1; then
  tail -n 180 "$STORAGE_LOG" >&2
  exit 1
fi
cp "$INITIAL_IMAGE" "$DISK_IMAGE"
INITIAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"

# Boot 1: v1 candidate, confirmation-only first tap, install, live launch.
start_qemu install "$BASE_APK"
wait_for_pattern \
  "^ANDROID_PACKAGE_INSTALL_CANDIDATE_OK abi=${ABI} .* action=install expected_generation=0 expected_version_code=0 candidate_version_code=1 apk_bytes=${BASE_BYTES} package=${PACKAGE} activity=${ACTIVITY} .* boot_writes=0 boot_flushes=0 first_tap_mutation=0 .* network=disabled$" \
  "the v1 install candidate"
wait_for_pattern \
  "^ANDROIDBOX_RUNTIME_INSTALL2_PROFILE_OK .*abi=${ABI} .*confirmation=two-step .*network=disabled general_android_compatibility=0$" \
  "the ABI-$ABI runtime-install profile"
if [[ -n "$EXTRA_PROFILE_PATTERN" ]]; then
  wait_for_pattern "$EXTRA_PROFILE_PATTERN" "the child compatibility profile"
fi
boot_ready
[[ "$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')" == "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "Candidate admission changed the virgin package disk."
unlock_home
open_settings_apps "$INSTALL_READY" "" "the install-ready Apps page"
first_confirmation_tap 360 600 "$INSTALL_CONFIRM" "$INSTALL_READY"
[[ "$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')" == "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The first install tap changed the package disk."
confirm_install install 0 1 1 "$INSTALL_DONE" "$INSTALL_CONFIRM"
INSTALL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$INSTALL_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The confirmed install did not change the package disk."
launch_from_home 1 "$INSTALL_DRAWER" "$INSTALL_ACTIVITY" 1
stop_qemu

# Boot 2: source-free v1 recovery and actual Activity launch.
start_qemu recovery-v1 ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* generation=1 .* version_code=1 package=${PACKAGE} activity=${ACTIVITY} .* mutation_performed=0 .* writes=0 flushes=0 source_free_zero_writes=1 .*" \
  "source-free v1 recovery"
boot_ready
unlock_home
launch_from_home 1 "$RECOVERY_DRAWER" "$RECOVERY_ACTIVITY" 0
stop_qemu
[[ "$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')" == "$INSTALL_DISK_SHA256" ]] \
  || fail_gate "Source-free v1 recovery changed the package disk."

# Boot 3: v2 update candidate, confirmation-only first tap, update, launch.
start_qemu update "$UPDATE_APK"
wait_for_pattern \
  "^ANDROID_PACKAGE_INSTALL_CANDIDATE_OK abi=${ABI} .* action=update expected_generation=1 expected_version_code=1 candidate_version_code=2 apk_bytes=${UPDATE_BYTES} package=${PACKAGE} activity=${ACTIVITY} .* boot_writes=0 boot_flushes=0 first_tap_mutation=0 .* network=disabled$" \
  "the same-signer v2 update candidate"
boot_ready
[[ "$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')" == "$INSTALL_DISK_SHA256" ]] \
  || fail_gate "Update candidate admission changed the package disk."
unlock_home
open_settings_apps "$UPDATE_READY" "$INSTALL_READY" "the update-ready Apps page"
first_confirmation_tap 572 380 "$UPDATE_CONFIRM" "$UPDATE_READY"
[[ "$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')" == "$INSTALL_DISK_SHA256" ]] \
  || fail_gate "The first update tap changed the package disk."
confirm_install update 1 2 2 "$UPDATE_DONE" "$UPDATE_CONFIRM"
UPDATE_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$UPDATE_DISK_SHA256" != "$INSTALL_DISK_SHA256" ]] \
  || fail_gate "The confirmed update did not change the package disk."
launch_from_home 2 "$UPDATE_DRAWER" "$UPDATE_ACTIVITY" 1
stop_qemu

# Boot 4: source-free v2 recovery must be read-only.
start_qemu recovery-v2 ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* generation=2 .* version_code=2 package=${PACKAGE} activity=${ACTIVITY} .* mutation_performed=0 .* writes=0 flushes=0 source_free_zero_writes=1 .*" \
  "source-free v2 recovery"
boot_ready
stop_qemu
FINAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$FINAL_DISK_SHA256" == "$UPDATE_DISK_SHA256" ]] \
  || fail_gate "Source-free v2 recovery changed the package disk."
[[ "$QEMU_STARTS" == 4 && "$QEMU_NETWORK_DISABLED_STARTS" == 4 ]] \
  || fail_gate "The gate did not start exactly four network-disabled owned VMs."

python3 - \
  "$INSTALL_READY" "$INSTALL_CONFIRM" "$INSTALL_DONE" "$INSTALL_ACTIVITY" \
  "$RECOVERY_ACTIVITY" "$UPDATE_READY" "$UPDATE_CONFIRM" "$UPDATE_DONE" \
  "$UPDATE_ACTIVITY" "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

*input_names, output_name = sys.argv[1:]
paths = [Path(name) for name in input_names]
frames = [path.read_bytes() for path in paths]
for left, right, description in (
    (0, 1, "install-ready/confirm"),
    (1, 2, "install-confirm/done"),
    (5, 6, "update-ready/confirm"),
    (6, 7, "update-confirm/done"),
    (0, 5, "install-ready/update-ready"),
):
    if frames[left] == frames[right]:
        raise SystemExit(f"{description} frames are unexpectedly pixel-identical")
lines = ["width=720", "height=1600", "format=P6"]
for path, frame in zip(paths, frames):
    lines.append(f"{path.stem}_sha256={sha256(frame).hexdigest()}")
Path(output_name).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf '%s\n' \
  "$GATE_OK" \
  "abi=$ABI" \
  "package=$PACKAGE" \
  "activity=$ACTIVITY" \
  "signer_sha256=$SIGNER_SHA256" \
  "base_version=1" \
  "base_apk_bytes=$BASE_BYTES" \
  "base_apk_sha256=$BASE_SHA256" \
  "update_version=2" \
  "update_apk_bytes=$UPDATE_BYTES" \
  "update_apk_sha256=$UPDATE_SHA256" \
  'confirmation=two-step' \
  'first_tap_mutation=0' \
  'install_generation=1' \
  'update_generation=2' \
  'source_free_recovery=1' \
  'source_free_writes=0' \
  'same_boot_launcher_refresh=1' \
  'isolated_android_app_launch=1' \
  'qemu_starts=4' \
  'qemu_network=disabled' \
  'storage_handle_granted=0' \
  'block_handle_granted=0' \
  'arbitrary_path=0' \
  "initial_disk_sha256=$INITIAL_DISK_SHA256" \
  "installed_disk_sha256=$INSTALL_DISK_SHA256" \
  "updated_disk_sha256=$UPDATE_DISK_SHA256" \
  "final_disk_sha256=$FINAL_DISK_SHA256" \
  "raster_evidence=$RASTER_EVIDENCE" \
  >"$SUMMARY"

printf '%s\n' \
  "$GATE_OK" \
  "Evidence: $ARTIFACT_DIR" \
  "Summary: $SUMMARY"
