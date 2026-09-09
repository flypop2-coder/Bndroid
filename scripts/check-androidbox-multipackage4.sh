#!/usr/bin/env bash
set -euo pipefail

# Strict offline ABI-55 two-package coexistence gate.
#
# Boot 1 installs the real SDK Envelope APK without launching it. Boot 2 adds
# the distinct Manifest Catalog APK to the same disk, proves that both launcher
# entries coexist, switches between both Settings package-detail selectors,
# launches Catalog first so the fixed ABI-55 restart transcript is exercised
# against its matching fixture, then launches Envelope. Boot 3 has no fw_cfg
# APK source and repeats both launches while proving zero disk writes.
# Every QEMU instance is network-disabled and cleanup owns only its direct `$!`.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if (($#)); then
  echo "Usage: check-androidbox-multipackage4.sh" >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed head; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the ABI-55 multipackage gate." >&2
    exit 1
  }
done

ENVELOPE_BUILD="$WORKSPACE_ROOT/fixtures/androidbox-envelope-demo/build.sh"
CATALOG_BUILD="$WORKSPACE_ROOT/fixtures/androidbox-manifest-catalog-demo/build.sh"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="$SCRIPT_DIR/build-multipackage-storage-image.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in \
  "$ENVELOPE_BUILD" "$CATALOG_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$QMP_HELPER"; do
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

ABI="${BNDROID_MULTIPACKAGE_GATE_ABI:-55}"
FEATURES="${BNDROID_MULTIPACKAGE_GATE_FEATURES:-androidbox-multipackage4}"
case "$ABI/$FEATURES" in
  55/androidbox-multipackage4)
    TERMINAL=ANDROIDBOX_MULTIPACKAGE4_QEMU_OK
    ICON_PIXELS=0
    ;;
  56/androidbox-icon-resources5)
    TERMINAL=ANDROIDBOX_ICON_RESOURCES5_QEMU_OK
    ICON_PIXELS=1
    ;;
  57/androidbox-density-icons7)
    TERMINAL=ANDROIDBOX_DENSITY_ICONS7_QEMU_OK
    ICON_PIXELS=1
    ;;
  58/androidbox-dex-methods8)
    TERMINAL=ANDROIDBOX_DEX_METHODS8_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0=6425804802
    ;;
  59/androidbox-dex-instance9)
    TERMINAL=ANDROIDBOX_DEX_INSTANCE9_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0=1105937432578
    ;;
  60/androidbox-activity-fields10)
    TERMINAL=ANDROIDBOX_ACTIVITY_FIELDS10_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0=282580914143234
    ;;
  61/androidbox-activity-state11)
    TERMINAL=ANDROIDBOX_ACTIVITY_STATE11_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=72621649928781826
    UPDATE_ARG0_SECOND=144679243966709762
    ;;
  62/androidbox-string-text12)
    TERMINAL=ANDROIDBOX_STRING_TEXT12_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=108649343141150722
    UPDATE_ARG0_SECOND=180706937179078658
    ;;
  63/androidbox-string-builder13)
    TERMINAL=ANDROIDBOX_STRING_BUILDER13_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    ;;
  64/androidbox-layout-row14)
    TERMINAL=ANDROIDBOX_LAYOUT_ROW14_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=210
    FIRST_CLICK_Y=740
    SECOND_CLICK_X=500
    SECOND_CLICK_Y=740
    ;;
  65/androidbox-layout-weight15)
    TERMINAL=ANDROIDBOX_LAYOUT_WEIGHT15_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=210
    FIRST_CLICK_Y=740
    SECOND_CLICK_X=500
    SECOND_CLICK_Y=740
    ;;
  66/androidbox-layout-spacing16)
    TERMINAL=ANDROIDBOX_LAYOUT_SPACING16_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=218
    FIRST_CLICK_Y=740
    SECOND_CLICK_X=502
    SECOND_CLICK_Y=740
    ;;
  67/androidbox-layout-directional17)
    TERMINAL=ANDROIDBOX_LAYOUT_DIRECTIONAL17_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=219
    FIRST_CLICK_Y=734
    SECOND_CLICK_X=509
    SECOND_CLICK_Y=740
    ;;
  68/androidbox-layout-size18)
    TERMINAL=ANDROIDBOX_LAYOUT_SIZE18_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=219
    FIRST_CLICK_Y=682
    SECOND_CLICK_X=509
    SECOND_CLICK_Y=682
    ;;
  69/androidbox-layout-mixed19)
    TERMINAL=ANDROIDBOX_LAYOUT_MIXED19_QEMU_OK
    ICON_PIXELS=1
    UPDATE_ARG0_FIRST=126663741650632706
    UPDATE_ARG0_SECOND=198721335688560642
    FIRST_CLICK_REQUEST_ID=8
    SECOND_CLICK_REQUEST_ID=9
    FIRST_CLICK_X=216
    FIRST_CLICK_Y=682
    SECOND_CLICK_X=506
    SECOND_CLICK_Y=682
    ;;
  *)
    echo "Expected ABI/feature pair 55/multipackage4 through 69/layout-mixed19." >&2
    exit 2
    ;;
esac
UPDATE_ARG0=${UPDATE_ARG0:-2130837506}
UPDATE_ARG0_FIRST=${UPDATE_ARG0_FIRST:-"$UPDATE_ARG0"}
UPDATE_ARG0_SECOND=${UPDATE_ARG0_SECOND:-"$UPDATE_ARG0"}
FIRST_CLICK_REQUEST_ID=${FIRST_CLICK_REQUEST_ID:-7}
SECOND_CLICK_REQUEST_ID=${SECOND_CLICK_REQUEST_ID:-8}
FIRST_CLICK_X=${FIRST_CLICK_X:-360}
FIRST_CLICK_Y=${FIRST_CLICK_Y:-688}
SECOND_CLICK_X=${SECOND_CLICK_X:-360}
SECOND_CLICK_Y=${SECOND_CLICK_Y:-792}
ENVELOPE_PACKAGE=org.bndroid.envelope
ENVELOPE_ACTIVITY='Lorg/bndroid/envelope/MainActivity;'
CATALOG_PACKAGE=org.bndroid.catalog
CATALOG_ACTIVITY='Lorg/bndroid/catalog/MainActivity;'
ARTIFACT_ROOT="${BNDROID_MULTIPACKAGE_GATE_ARTIFACT_ROOT:-"$WORKSPACE_ROOT/target/androidbox-multipackage4"}"
TARGET_ROOT="${BNDROID_MULTIPACKAGE_GATE_TARGET_ROOT:-"$WORKSPACE_ROOT/target/androidbox-multipackage4-build"}"

mkdir -p "$ARTIFACT_ROOT"
ARTIFACT_DIR="$(mktemp -d "$ARTIFACT_ROOT/coexist.XXXXXX")"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

ENVELOPE_APK="$ARTIFACT_DIR/envelope-v1.apk"
CATALOG_APK="$ARTIFACT_DIR/catalog-v1.apk"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
SUMMARY="$ARTIFACT_DIR/summary.txt"
COPY_EVIDENCE="$ARTIFACT_DIR/copy-evidence.json"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
DEX_METHOD_RASTER_EVIDENCE="$ARTIFACT_DIR/dex-method-raster-evidence.txt"

QEMU_PID=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""
BOOT_QMP_SOCKET=""
QEMU_STARTS=0

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
  if [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 460 >&2
  fi
  [[ -f "$BOOT_QEMU_LOG" ]] && tail -n 120 "$BOOT_QEMU_LOG" >&2 || true
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Multipackage evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_bad_output() {
  normalize_log
  local bad='fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_CHILD_DIAG|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROID_APP_RESTART_TIMEOUT|ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_FAIL'
  if grep -Eqi "$bad" "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fatal, child failure, or package failure."
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
  normalize_log
  grep -Ec "$1" "$BOOT_NORMALIZED_LOG" || true
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
  python3 - "$1" "${2:-any}" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
parts = path.read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)
if len({pixels[index:index + 3] for index in range(0, len(pixels), 3)}) < 80:
    raise SystemExit(1)
if sys.argv[2] == "drawer":
    # The default appearance's settled handle is at y=160. A held reveal or
    # the last outstanding Home frame must never satisfy this visual fence.
    for x in (300, 360, 420):
        offset = (160 * 720 + x) * 3
        if pixels[offset:offset + 3] != bytes.fromhex("a6b2c8"):
            raise SystemExit(1)
elif sys.argv[2] != "any":
    raise SystemExit("unknown screenshot predicate")
PY
}

take_screenshot() {
  local screenshot="$1"
  local different_from="${2:-}"
  local expected_view="${3:-any}"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  qmp move 700 1500
  while ((SECONDS < deadline)); do
    reject_bad_output
    qmp screenshot "$screenshot"
    if canonical_ppm "$screenshot" "$expected_view" \
      && { [[ -z "$different_from" ]] || ! cmp -s "$different_from" "$screenshot"; }; then
      return
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for canonical screenshot $screenshot."
}

start_qemu() {
  local mode="$1"
  local source_apk="$2"
  local -a source_args=(-name "Bndroid ABI-$ABI Multipackage Gate $mode")
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
  wait_for_pattern \
    "^MOBILE_UI_PREVIEW_OK .*abi=${ABI} width=720 height=1600 " \
    "the ABI-55 720x1600 mobile preview"
  wait_for_pattern \
    "^ANDROID_APP_PROCESS_OK .*abi=${ABI} " \
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
  local before
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 615 1435
  wait_for_pattern \
    '^UI_ROUTE_FOCUS_OK .* receiver_image=app .* active_client=app app=settings ' \
    "Settings focus"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 1))" \
    "the Settings frame"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 360 1350
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the Apps page"
}

install_candidate() {
  local package="$1"
  local installed_count="$2"
  local trigger_x="$3"
  local trigger_y="$4"
  local before
  local durable_before
  local ready="$ARTIFACT_DIR/$5-ready.ppm"
  local confirm="$ARTIFACT_DIR/$5-confirm.ppm"
  local done="$ARTIFACT_DIR/$5-done.ppm"
  take_screenshot "$ready"
  durable_before="$(log_count '^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK ')"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap "$trigger_x" "$trigger_y"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the install confirmation frames"
  take_screenshot "$confirm" "$ready"
  [[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK ')" == "$durable_before" ]] \
    || fail_gate "The first confirmation tap mutated package storage."
  qmp tap 515 1070
  wait_for_pattern \
    "^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK owner=${APP_PID} request_sequence=1 .* action=install previous_generation=0 installed_generation=1 version_code=1 package=${package} reads=[1-9][0-9]* writes=[1-9][0-9]* flushes=[1-9][0-9]* source=immutable-fw-cfg network=disabled$" \
    "the durable install for $package"
  wait_for_pattern \
    "^ANDROID_PACKAGE_DIRECTORY_READ_OK image=7 bytes=1344 revision=[1-9][0-9]* installed_count=${installed_count} authority_granted=0 apk_bytes_exposed=0$" \
    "the ${installed_count}-entry live directory"
  take_screenshot "$done" "$confirm"
}

verify_two_package_settings_selection() {
  local before
  local durable_before
  local catalog_selected="$ARTIFACT_DIR/settings-catalog-selected.ppm"
  local envelope_selected="$ARTIFACT_DIR/settings-envelope-selected.ppm"
  durable_before="$(log_count '^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK ')"

  # Back dismisses only the completed install result and intentionally remains
  # on Apps. ABI 55 can then expose the two zero-authority detail selectors.
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 56 120
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the two-package Settings summary"
  take_screenshot "$catalog_selected"

  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 198 336
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 2))" \
    "the first-package Settings selection"
  take_screenshot "$envelope_selected" "$catalog_selected"

  [[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_INSTALL_DURABLE_OK ')" == "$durable_before" ]] \
    || fail_gate "Settings package selection unexpectedly submitted a durable transaction."
}

return_home_and_open_drawer() {
  local expected_recent="$1"
  local before
  local home_states_before
  home_states_before="$(log_count "^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=${expected_recent} nav_pressed=0 nav_reveal_px=0 ")"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} ")"
  qmp tap 360 1570
  wait_for_count \
    "^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=${expected_recent} nav_pressed=0 nav_reveal_px=0 " \
    "$((home_states_before + 1))" \
    "the fresh Launcher Home state"
  wait_for_pattern \
    '^UI_ROUTE_FOCUS_OK .* receiver_image=launcher .* active_client=launcher app=none ' \
    "Launcher focus"
  wait_for_pattern \
    '^ANDROID_PACKAGE_DIRECTORY_READ_OK image=6 bytes=1344 revision=[1-9][0-9]* installed_count=2 authority_granted=0 apk_bytes_exposed=0$' \
    "the two-entry Launcher directory"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
    "$((before + 1))" \
    "the fresh Launcher Home frame"
  open_drawer_from_home
}

open_drawer_from_home() {
  local before
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} ")"
  qmp drag 360 1280 360 520
  # One held reveal plus one settled release. The blank down location has no
  # pressed frame; waiting for four commits would count unrelated minute ticks.
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
    "$((before + 2))" \
    "the two-app drawer"
  # Count alone can include an outstanding unlock/Home frame. Require the
  # actual settled geometry before capturing icons or tapping an installed app.
  take_screenshot "$ARTIFACT_DIR/drawer-ready.ppm" "" drawer
}

launch_drawer_app() {
  local request_sequence="$1"
  local tap_x="$2"
  local package="$3"
  local activity="$4"
  local screenshot="$5"
  local expect_restart="$6"
  local before
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap "$tap_x" 674
  wait_for_pattern \
    "^ANDROID_PACKAGE_DURABLE_LAUNCH_OK owner=${LAUNCHER_PID} request_sequence=${request_sequence} generation=1 .* package=${package} activity=${activity} " \
    "the durable launch for $package"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((before + 5))" \
    "the Activity frames for $package"
  if [[ "$expect_restart" == 1 ]]; then
    wait_for_pattern \
      "^ANDROID_APP_RESTART_OK .*abi=${ABI} .*package_generation=1 .*app_enter_frames=5 app_layered_commits=5 errors=0$" \
      "the authenticated AndroidApp restart"
  fi
  take_screenshot "$screenshot"
  if [[ ( "$ABI" == 58 || "$ABI" == 59 || "$ABI" == 60 || "$ABI" == 61 || "$ABI" == 62 || "$ABI" == 63 || "$ABI" == 64 || "$ABI" == 65 || "$ABI" == 66 || "$ABI" == 67 || "$ABI" == 68 || "$ABI" == 69 ) && "$expect_restart" == 1 ]]; then
    local new_worker_pid
    local approved="$ARTIFACT_DIR/$(basename "${screenshot%.ppm}")-approved.ppm"
    local rejected="$ARTIFACT_DIR/$(basename "${screenshot%.ppm}")-rejected.ppm"
    new_worker_pid="$(
      sed -n 's/^ANDROID_APP_RESTART_OK .* new_pid=\([1-9][0-9]*\) .*/\1/p' \
        "$BOOT_NORMALIZED_LOG" | tail -n 1
    )"
    [[ "$new_worker_pid" =~ ^[1-9][0-9]*$ ]] \
      || fail_gate "Could not derive the replacement AndroidApp PID."
    before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
    qmp tap "$FIRST_CLICK_X" "$FIRST_CLICK_Y"
    wait_for_pattern \
      "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${new_worker_pid} receiver_image=app receiver_pid=${APP_PID} kind=updated request_id=${FIRST_CLICK_REQUEST_ID} arg0=${UPDATE_ARG0_FIRST} " \
      "the APK-defined approve result"
    wait_for_count \
      "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
      "$((before + 2))" \
      "the approved callback frames"
    take_screenshot "$approved" "$screenshot"

    before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
    qmp tap "$SECOND_CLICK_X" "$SECOND_CLICK_Y"
    wait_for_pattern \
      "^ANDROID_APP_RPC_READ_OK sender_image=android-app sender_pid=${new_worker_pid} receiver_image=app receiver_pid=${APP_PID} kind=updated request_id=${SECOND_CLICK_REQUEST_ID} arg0=${UPDATE_ARG0_SECOND} " \
      "the APK-defined reject result"
    wait_for_count \
      "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
      "$((before + 2))" \
      "the rejected callback frames"
    take_screenshot "$rejected" "$approved"
  fi
}

capture_compatible_overview() {
  local screenshot="$1"
  local home_states_before
  local overview_states_before
  local before

  home_states_before="$(log_count '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ')"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} ")"
  qmp tap 360 1570
  wait_for_count \
    '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ' \
    "$((home_states_before + 1))" \
    "Home retaining the compatible Catalog identity"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
    "$((before + 1))" \
    "the Home frame before compatible Overview"

  overview_states_before="$(log_count '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ')"
  before="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} ")"
  qmp touch-down 360 1570
  qmp touch-move 360 1330
  qmp touch-up
  wait_for_count \
    '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 ' \
    "$((overview_states_before + 1))" \
    "the compatible Catalog Overview state"
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
    "$((before + 1))" \
    "the compatible Catalog Overview frame"
  take_screenshot "$screenshot"
}

# Build two distinct real SDK APKs offline with the repository test signer.
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$ENVELOPE_APK" \
  "$ENVELOPE_BUILD" >"$ARTIFACT_DIR/envelope-build.log" 2>&1 \
  || { tail -n 180 "$ARTIFACT_DIR/envelope-build.log" >&2; exit 1; }
BNDROID_ENVELOPE_VERSION_CODE=1 \
BNDROID_ENVELOPE_VERSION_NAME=1.0 \
BNDROID_ENVELOPE_OUTPUT_APK="$CATALOG_APK" \
  "$CATALOG_BUILD" >"$ARTIFACT_DIR/catalog-build.log" 2>&1 \
  || { tail -n 180 "$ARTIFACT_DIR/catalog-build.log" >&2; exit 1; }

ENVELOPE_BYTES="$(wc -c <"$ENVELOPE_APK" | tr -d '[:space:]')"
CATALOG_BYTES="$(wc -c <"$CATALOG_APK" | tr -d '[:space:]')"
ENVELOPE_SHA256="$(shasum -a 256 "$ENVELOPE_APK" | awk '{print $1}')"
CATALOG_SHA256="$(shasum -a 256 "$CATALOG_APK" | awk '{print $1}')"
[[ "$ENVELOPE_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$CATALOG_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$ENVELOPE_SHA256" != "$CATALOG_SHA256" ]] \
  || { echo "The two APK inputs are not distinct canonical artifacts." >&2; exit 1; }

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  "$BUILD_KERNEL" >"$ARTIFACT_DIR/build.log" 2>&1; then
  tail -n 320 "$ARTIFACT_DIR/build.log" >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || { echo "Kernel image missing." >&2; exit 1; }

if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$BUILD_STORAGE" >"$ARTIFACT_DIR/storage-build.log" 2>&1; then
  tail -n 180 "$ARTIFACT_DIR/storage-build.log" >&2
  exit 1
fi
cp "$INITIAL_IMAGE" "$DISK_IMAGE"
INITIAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"

# Boot 1: install Envelope into volume 0, but do not start the fixed restart
# transcript because this compile profile intentionally matches Catalog.
start_qemu install-envelope "$ENVELOPE_APK"
wait_for_pattern \
  "^ANDROID_PACKAGE_INSTALL_CANDIDATE_OK abi=${ABI} .*action=install expected_generation=0 .*apk_bytes=${ENVELOPE_BYTES} package=${ENVELOPE_PACKAGE} activity=${ENVELOPE_ACTIVITY} .*boot_writes=0 boot_flushes=0 first_tap_mutation=0 .*network=disabled$" \
  "the Envelope install candidate"
boot_ready
unlock_home
open_settings_apps
install_candidate "$ENVELOPE_PACKAGE" 1 360 600 envelope
stop_qemu
ONE_PACKAGE_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$ONE_PACKAGE_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The first package did not change the package disk."

# Boot 2: Catalog must target the empty second volume instead of replacing
# Envelope. Launch Catalog first, then prove Envelope remains independently
# launchable from the other drawer cell.
start_qemu install-catalog "$CATALOG_APK"
wait_for_pattern \
  "^ANDROID_PACKAGE_INSTALL_CANDIDATE_OK abi=${ABI} .*action=install expected_generation=0 .*apk_bytes=${CATALOG_BYTES} package=${CATALOG_PACKAGE} activity=${CATALOG_ACTIVITY} .*boot_writes=0 boot_flushes=0 first_tap_mutation=0 .*network=disabled$" \
  "the distinct Catalog install candidate"
boot_ready
wait_for_pattern \
  '^ANDROID_PACKAGE_DIRECTORY_READ_OK image=6 bytes=1344 revision=1 installed_count=1 authority_granted=0 apk_bytes_exposed=0$' \
  "the pre-install one-entry directory"
unlock_home
open_settings_apps
install_candidate "$CATALOG_PACKAGE" 2 572 380 catalog
TWO_PACKAGE_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$TWO_PACKAGE_DISK_SHA256" != "$ONE_PACKAGE_DISK_SHA256" ]] \
  || fail_gate "The second package did not change the package disk."
verify_two_package_settings_selection
return_home_and_open_drawer settings
take_screenshot "$ARTIFACT_DIR/two-app-drawer.ppm"
launch_drawer_app 1 276 "$CATALOG_PACKAGE" "$CATALOG_ACTIVITY" \
  "$ARTIFACT_DIR/catalog-activity.ppm" 1
capture_compatible_overview "$ARTIFACT_DIR/catalog-overview.ppm"
return_home_and_open_drawer android-compatible
if [[ "$ABI" == 58 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_DEX_METHODS8_RPC_OK .*abi=58 protocol=BNDAPC04 protocol_version=4 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the complete APK-defined Catalog transcript"
elif [[ "$ABI" == 59 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_DEX_INSTANCE9_RPC_OK .*abi=59 protocol=BNDAPC05 protocol_version=5 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the complete APK-defined instance Catalog transcript"
elif [[ "$ABI" == 60 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_ACTIVITY_FIELDS10_RPC_OK .*abi=60 protocol=BNDAPC06 protocol_version=6 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 field_reads=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the complete Activity-field Catalog transcript"
elif [[ "$ABI" == 61 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_ACTIVITY_STATE11_RPC_OK .*abi=61 protocol=BNDAPC07 protocol_version=7 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 field_reads=2/2 int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the complete persistent Activity-state Catalog transcript"
elif [[ "$ABI" == 62 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_STRING_TEXT12_RPC_OK .*abi=62 protocol=BNDAPC08 protocol_version=8 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the complete direct DEX-string Catalog transcript"
elif [[ "$ABI" == 63 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_STRING_BUILDER13_RPC_OK .*abi=63 protocol=BNDAPC09 protocol_version=9 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the complete dynamic StringBuilder Catalog transcript"
elif [[ "$ABI" == 64 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_ROW14_RPC_OK .*abi=64 protocol=BNDAPC10 protocol_version=10 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete nested horizontal-layout Catalog transcript"
elif [[ "$ABI" == 65 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_WEIGHT15_RPC_OK .*abi=65 protocol=BNDAPC11 protocol_version=11 descriptor_version=2 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_geometry=apk-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete weighted horizontal-layout Catalog transcript"
elif [[ "$ABI" == 66 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_SPACING16_RPC_OK .*abi=66 protocol=BNDAPC12 protocol_version=12 descriptor_version=3 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_padding_dp=4 button_margins_dp=2/2 row_geometry=apk-padding-margin-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete APK-spaced horizontal-layout Catalog transcript"
elif [[ "$ABI" == 67 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_DIRECTIONAL17_RPC_OK .*abi=67 protocol=BNDAPC13 protocol_version=13 descriptor_version=4 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-directional-padding-margin-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete APK-directional horizontal-layout Catalog transcript"
elif [[ "$ABI" == 68 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_SIZE18_RPC_OK .*abi=68 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=exact-240dp/match/zero-dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-exact-size-directional-spacing-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete exact-dp horizontal-layout Catalog transcript"
elif [[ "$ABI" == 69 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_MIXED19_RPC_OK .*abi=69 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=exact-240dp/match/exact-132dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=0/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-mixed-fixed-weighted-remaining-space .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the complete mixed fixed/weighted horizontal-layout Catalog transcript"
fi
launch_drawer_app 2 106 "$ENVELOPE_PACKAGE" "$ENVELOPE_ACTIVITY" \
  "$ARTIFACT_DIR/envelope-activity.ppm" 0
stop_qemu

# Boot 3: no source. Both persisted entries must reappear and both Activity
# launches must succeed without changing a single byte of the disk image.
start_qemu recovery-two ""
wait_for_pattern \
  '^APK_PACKAGE_STORE_OK .*source_present=0 source_admitted=0 .*operation=recovery .*mutation_performed=0 .*writes=0 flushes=0 source_free_zero_writes=1 ' \
  "source-free multipackage recovery"
boot_ready
wait_for_pattern \
  '^ANDROID_PACKAGE_DIRECTORY_READ_OK image=6 bytes=1344 revision=1 installed_count=2 authority_granted=0 apk_bytes_exposed=0$' \
  "the recovered two-entry directory"
unlock_home
open_drawer_from_home
take_screenshot "$ARTIFACT_DIR/recovery-two-app-drawer.ppm"
launch_drawer_app 1 276 "$CATALOG_PACKAGE" "$CATALOG_ACTIVITY" \
  "$ARTIFACT_DIR/recovery-catalog-activity.ppm" 1
capture_compatible_overview "$ARTIFACT_DIR/recovery-catalog-overview.ppm"
return_home_and_open_drawer android-compatible
if [[ "$ABI" == 58 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_DEX_METHODS8_RPC_OK .*abi=58 protocol=BNDAPC04 protocol_version=4 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered APK-defined Catalog transcript"
elif [[ "$ABI" == 59 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_DEX_INSTANCE9_RPC_OK .*abi=59 protocol=BNDAPC05 protocol_version=5 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered APK-defined instance Catalog transcript"
elif [[ "$ABI" == 60 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_ACTIVITY_FIELDS10_RPC_OK .*abi=60 protocol=BNDAPC06 protocol_version=6 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 field_reads=1/1 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered Activity-field Catalog transcript"
elif [[ "$ABI" == 61 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_ACTIVITY_STATE11_RPC_OK .*abi=61 protocol=BNDAPC07 protocol_version=7 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=1/1 instance_calls=1/1 field_reads=2/2 int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered persistent Activity-state Catalog transcript"
elif [[ "$ABI" == 62 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_STRING_TEXT12_RPC_OK .*abi=62 protocol=BNDAPC08 protocol_version=8 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered direct DEX-string Catalog transcript"
elif [[ "$ABI" == 63 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_STRING_BUILDER13_RPC_OK .*abi=63 protocol=BNDAPC09 protocol_version=9 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 errors=0 queues_empty=1$" \
    "the recovered dynamic StringBuilder Catalog transcript"
elif [[ "$ABI" == 64 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_ROW14_RPC_OK .*abi=64 protocol=BNDAPC10 protocol_version=10 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered nested horizontal-layout Catalog transcript"
elif [[ "$ABI" == 65 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_WEIGHT15_RPC_OK .*abi=65 protocol=BNDAPC11 protocol_version=11 descriptor_version=2 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_geometry=apk-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered weighted horizontal-layout Catalog transcript"
elif [[ "$ABI" == 66 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_SPACING16_RPC_OK .*abi=66 protocol=BNDAPC12 protocol_version=12 descriptor_version=3 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_padding_dp=4 button_margins_dp=2/2 row_geometry=apk-padding-margin-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered APK-spaced horizontal-layout Catalog transcript"
elif [[ "$ABI" == 67 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_DIRECTIONAL17_RPC_OK .*abi=67 protocol=BNDAPC13 protocol_version=13 descriptor_version=4 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=0dp/0dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-directional-padding-margin-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered APK-directional horizontal-layout Catalog transcript"
elif [[ "$ABI" == 68 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_SIZE18_RPC_OK .*abi=68 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=exact-240dp/match/zero-dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-exact-size-directional-spacing-weight-proportional .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered exact-dp horizontal-layout Catalog transcript"
elif [[ "$ABI" == 69 ]]; then
  wait_for_pattern \
    "^ANDROID_APP_LAYOUT_MIXED19_RPC_OK .*abi=69 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 row_parent_index=3 row_child_indices=4/5 layout_widths=exact-240dp/match/exact-132dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=0/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\\+6/5/2/1 row_geometry=apk-mixed-fixed-weighted-remaining-space .*clicked_button_ids=2130837504/2130837505 update_view_ids=2130837506/2130837506 app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 final_revision=2 requests=10 responses=11 chunks=6 first_round_messages=27 errors=0 queues_empty=1$" \
    "the recovered mixed fixed/weighted horizontal-layout Catalog transcript"
fi
launch_drawer_app 2 106 "$ENVELOPE_PACKAGE" "$ENVELOPE_ACTIVITY" \
  "$ARTIFACT_DIR/recovery-envelope-activity.ppm" 0
stop_qemu

FINAL_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$FINAL_DISK_SHA256" == "$TWO_PACKAGE_DISK_SHA256" ]] \
  || fail_gate "Source-free two-package recovery changed the package disk."
[[ "$QEMU_STARTS" == 3 ]] \
  || fail_gate "The gate did not start exactly three owned network-disabled VMs."

python3 - \
  "$ARTIFACT_DIR/two-app-drawer.ppm" \
  "$ARTIFACT_DIR/catalog-activity.ppm" \
  "$ARTIFACT_DIR/envelope-activity.ppm" \
  "$ARTIFACT_DIR/settings-catalog-selected.ppm" \
  "$ARTIFACT_DIR/settings-envelope-selected.ppm" \
  "$ARTIFACT_DIR/recovery-two-app-drawer.ppm" \
  "$ARTIFACT_DIR/recovery-catalog-activity.ppm" \
  "$ARTIFACT_DIR/recovery-envelope-activity.ppm" \
  "$ARTIFACT_DIR/catalog-overview.ppm" \
  "$ARTIFACT_DIR/recovery-catalog-overview.ppm" \
  "$ABI" \
  "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

*inputs, abi, output = sys.argv[1:]
paths = [Path(name) for name in inputs]
frames = [path.read_bytes() for path in paths]
if frames[1] == frames[2] or frames[6] == frames[7]:
    raise SystemExit("the two package Activity rasters are pixel-identical")
if frames[3] == frames[4]:
    raise SystemExit("the two Settings package selections are pixel-identical")

header = b"P6\n720 1600\n255\n"

def pixels(frame: bytes) -> bytes:
    if not frame.startswith(header):
        raise SystemExit("unexpected PPM header")
    payload = frame[len(header):]
    if len(payload) != 720 * 1600 * 3:
        raise SystemExit("unexpected PPM pixel payload")
    return payload

def region(frame: bytes, x0: int, y0: int, x1: int, y1: int) -> bytes:
    payload = pixels(frame)
    return b"".join(
        payload[(y * 720 + x0) * 3:(y * 720 + x1) * 3]
        for y in range(y0, y1)
    )

# The publisher scene begins below row 440. Requiring the package-owned header
# itself to differ catches a cross-process selection bug where Catalog content
# ran underneath Envelope's title, package name, and icon.
if region(frames[1], 0, 64, 720, 400) == region(frames[2], 0, 64, 720, 400):
    raise SystemExit("the live Activity headers retained one package identity")
if region(frames[6], 0, 64, 720, 400) == region(frames[7], 0, 64, 720, 400):
    raise SystemExit("the recovered Activity headers retained one package identity")

if abi in ("56", "57", "58", "59", "60", "61", "62", "63", "64", "65", "66", "67", "68", "69"):
    def rgb(frame: bytes, x: int, y: int) -> bytes:
        payload = pixels(frame)
        start = (y * 720 + x) * 3
        return payload[start:start + 3]

    drawer = frames[0]
    recovered = frames[5]
    expected = ((104, bytes.fromhex("0b57d0")), (276, bytes.fromhex("8e24aa")))
    for x, color in expected:
        if rgb(drawer, x, 634) != color or rgb(recovered, x, 634) != color:
            raise SystemExit("APK launcher icon pixels did not reach both drawer rasters")
    activity_icons = (
        (frames[1], bytes.fromhex("8e24aa")),
        (frames[2], bytes.fromhex("0b57d0")),
        (frames[6], bytes.fromhex("8e24aa")),
        (frames[7], bytes.fromhex("0b57d0")),
    )
    for frame, color in activity_icons:
        if rgb(frame, 640, 84) != color:
            raise SystemExit("the Activity header icon did not match its launched package")
    for frame in (frames[8], frames[9]):
        if rgb(frame, 360, 580) != bytes.fromhex("8e24aa"):
            raise SystemExit("the compatible Overview card did not retain the Catalog APK icon")
lines = [
    "width=720",
    "height=1600",
    "format=P6",
    "distinct_activity_rasters=1",
    "distinct_activity_headers=1",
]
lines.append(f"abi={abi}")
lines.append(f"apk_launcher_icon_pixels={1 if abi in ('56', '57', '58', '59', '60', '61', '62', '63', '64', '65', '66', '67', '68', '69') else 0}")
lines.append(f"apk_activity_header_icon_pixels={1 if abi in ('56', '57', '58', '59', '60', '61', '62', '63', '64', '65', '66', '67', '68', '69') else 0}")
lines.append(f"apk_overview_recent_icon_pixels={1 if abi in ('56', '57', '58', '59', '60', '61', '62', '63', '64', '65', '66', '67', '68', '69') else 0}")
for path, frame in zip(paths, frames):
    lines.append(f"{path.stem}_sha256={sha256(frame).hexdigest()}")
Path(output).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

OVERVIEW_ICON_PIXELS="$(
  sed -n 's/^apk_overview_recent_icon_pixels=//p' "$RASTER_EVIDENCE"
)"
[[ "$OVERVIEW_ICON_PIXELS" == "$ICON_PIXELS" ]] \
  || fail_gate "The compatible Overview APK-icon evidence did not match ABI expectations."

DYNAMIC_STRING_TEXTS=false/false
if [[ "$ABI" == 58 || "$ABI" == 59 || "$ABI" == 60 || "$ABI" == 61 || "$ABI" == 62 || "$ABI" == 63 || "$ABI" == 64 || "$ABI" == 65 || "$ABI" == 66 || "$ABI" == 67 || "$ABI" == 68 || "$ABI" == 69 ]]; then
  python3 - \
    "$ARTIFACT_DIR/catalog-activity.ppm" \
    "$ARTIFACT_DIR/catalog-activity-approved.ppm" \
    "$ARTIFACT_DIR/catalog-activity-rejected.ppm" \
    "$ARTIFACT_DIR/recovery-catalog-activity.ppm" \
    "$ARTIFACT_DIR/recovery-catalog-activity-approved.ppm" \
    "$ARTIFACT_DIR/recovery-catalog-activity-rejected.ppm" \
    "$DEX_METHOD_RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

*inputs, output = sys.argv[1:]
paths = [Path(value) for value in inputs]
frames = [path.read_bytes() for path in paths]
header = b"P6\n720 1600\n255\n"

def pixels(frame: bytes) -> bytes:
    if not frame.startswith(header) or len(frame) != len(header) + 720 * 1600 * 3:
        raise SystemExit("noncanonical APK-method callback raster")
    return frame[len(header):]

def transition(before: bytes, after: bytes, name: str) -> int:
    old = pixels(before)
    new = pixels(after)
    stride = 720 * 3
    if old[:64 * stride] != new[:64 * stride] or old[1512 * stride:] != new[1512 * stride:]:
        raise SystemExit(f"{name}: trusted chrome changed")
    changed = 0
    inside = 0
    for y in range(1600):
        for x in range(720):
            start = (y * 720 + x) * 3
            if old[start:start + 3] == new[start:start + 3]:
                continue
            changed += 1
            if 68 <= x < 652 and 536 <= y < 632:
                inside += 1
    if changed < 100 or changed != inside:
        raise SystemExit(f"{name}: change escaped the status TextView: {changed}/{inside}")
    return changed

counts = (
    transition(frames[0], frames[1], "live-approve"),
    transition(frames[1], frames[2], "live-reject"),
    transition(frames[3], frames[4], "recovery-approve"),
    transition(frames[4], frames[5], "recovery-reject"),
)
lines = [
    "status_only_updates=4",
    "trusted_chrome_unchanged=1",
    f"live_approve_changed_pixels={counts[0]}",
    f"live_reject_changed_pixels={counts[1]}",
    f"recovery_approve_changed_pixels={counts[2]}",
    f"recovery_reject_changed_pixels={counts[3]}",
]
for path, frame in zip(paths, frames):
    lines.append(f"{path.stem}_sha256={sha256(frame).hexdigest()}")
Path(output).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
  if [[ "$ABI" == 58 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_DEX_METHODS8_RPC_OK .*abi=58 protocol=BNDAPC04 protocol_version=4 .*app_defined_calls=1/1 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_DEX_METHODS8_PROFILE_OK .*abi=58 .*protocol=BNDAPC04 protocol_version=4 .*app_defined_invoke=invoke-static .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=0/0
    ACTIVITY_INT_STATES=0/0
    DIRECT_STRING_TEXTS=false/false
  elif [[ "$ABI" == 59 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_DEX_INSTANCE9_RPC_OK .*abi=59 protocol=BNDAPC05 protocol_version=5 .*app_defined_calls=1/1 instance_calls=1/1 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_DEX_INSTANCE9_PROFILE_OK .*abi=59 .*protocol=BNDAPC05 protocol_version=5 .*app_defined_invoke=invoke-direct .*helper_framework_call=View.getId .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=1/1
    ACTIVITY_FIELD_READS=0/0
    ACTIVITY_INT_STATES=0/0
    DIRECT_STRING_TEXTS=false/false
  elif [[ "$ABI" == 60 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_ACTIVITY_FIELDS10_RPC_OK .*abi=60 protocol=BNDAPC06 protocol_version=6 .*app_defined_calls=1/1 instance_calls=1/1 field_reads=1/1 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_ACTIVITY_FIELDS10_PROFILE_OK .*abi=60 .*protocol=BNDAPC06 protocol_version=6 .*field_write=onCreate-iput-object field_read=onClick-iget-object .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=1/1
    ACTIVITY_FIELD_READS=1/1
    ACTIVITY_INT_STATES=0/0
    DIRECT_STRING_TEXTS=false/false
  elif [[ "$ABI" == 61 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_ACTIVITY_STATE11_RPC_OK .*abi=61 protocol=BNDAPC07 protocol_version=7 .*app_defined_calls=1/1 instance_calls=1/1 field_reads=2/2 int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_ACTIVITY_STATE11_PROFILE_OK .*abi=61 .*protocol=BNDAPC07 protocol_version=7 .*state_read=onClick-iget state_transition=add-int-lit8-plus1 state_write=onClick-iput .*transaction=scene\+fields\+revision-atomic .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=1/1
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=false/false
  elif [[ "$ABI" == 62 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_STRING_TEXT12_RPC_OK .*abi=62 protocol=BNDAPC08 protocol_version=8 .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_STRING_TEXT12_PROFILE_OK .*abi=62 .*protocol=BNDAPC08 protocol_version=8 .*framework_call=TextView.setText-CharSequence .*string_source=classes.dex-const-string .*state_transition=const1\+add-int-2addr .*transaction=scene\+fields\+revision-atomic .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
  elif [[ "$ABI" == 63 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_STRING_BUILDER13_RPC_OK .*abi=63 protocol=BNDAPC09 protocol_version=9 .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_STRING_BUILDER13_PROFILE_OK .*abi=63 .*protocol=BNDAPC09 protocol_version=9 .*allocation=new-instance-StringBuilder .*append=StringBuilder-append-int .*materialize=StringBuilder-toString .*update_provenance=arg0-field-byte-bit6-dynamic\+bit7-direct\+int-state .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 64 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_ROW14_RPC_OK .*abi=64 protocol=BNDAPC10 protocol_version=10 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_ROW14_PROFILE_OK .*abi=64 .*protocol=BNDAPC10 protocol_version=10 .*nested_orientation=vertical\+horizontal .*horizontal_row_parent=3 horizontal_row_children=4/5 .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 65 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_WEIGHT15_RPC_OK .*abi=65 protocol=BNDAPC11 protocol_version=11 descriptor_version=2 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*layout_widths=0dp/0dp layout_weights=1/1 row_geometry=apk-weight-proportional .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_WEIGHT15_PROFILE_OK .*abi=65 .*protocol=BNDAPC11 protocol_version=11 descriptor_version=2 .*layout_width=zero-dp layout_weight=bounded-integer-1-8 fixture_weights=1/1 row_geometry=apk-weight-proportional .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 66 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_SPACING16_RPC_OK .*abi=66 protocol=BNDAPC12 protocol_version=12 descriptor_version=3 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*layout_widths=0dp/0dp layout_weights=1/1 row_padding_dp=4 button_margins_dp=2/2 row_geometry=apk-padding-margin-weight-proportional .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_SPACING16_PROFILE_OK .*abi=66 .*protocol=BNDAPC12 protocol_version=12 descriptor_version=3 .*uniform_spacing_unit=dp spacing_range=0-16 row_padding_dp=4 button_margins_dp=2/2 .*physical_button_geometry=80/652/276/176\+364/652/276/176 .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 67 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_DIRECTIONAL17_RPC_OK .*abi=67 protocol=BNDAPC13 protocol_version=13 descriptor_version=4 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*layout_widths=0dp/0dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\+6/5/2/1 row_geometry=apk-directional-padding-margin-weight-proportional .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_DIRECTIONAL17_PROFILE_OK .*abi=67 .*protocol=BNDAPC13 protocol_version=13 descriptor_version=4 descriptor_bytes=24 .*directional_spacing_unit=dp spacing_range=0-16 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\+6/5/2/1 .*physical_button_geometry=84/650/270/168\+374/658/270/164 .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 68 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_SIZE18_RPC_OK .*abi=68 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*layout_widths=exact-240dp/match/zero-dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=1/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\+6/5/2/1 row_geometry=apk-exact-size-directional-spacing-weight-proportional .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_SIZE18_PROFILE_OK .*abi=68 .*protocol=BNDAPC14 protocol_version=14 descriptor_version=5 descriptor_bytes=24 .*exact_size_unit=dp exact_size_range=1-255 fixture_exact_sizes_dp=title-width-240\+row-height-120\+button-heights-64/56 .*physical_button_geometry=84/618/270/128\+374/626/270/112 .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  elif [[ "$ABI" == 69 ]]; then
    APP_DEFINED_METHOD_TRANSCRIPTS="$(
      { grep -hE \
          '^ANDROID_APP_LAYOUT_MIXED19_RPC_OK .*abi=69 protocol=BNDAPC14 protocol_version=14 descriptor_version=5 node_descriptor_bytes=24 .*node_count=6 layout_nodes=2 horizontal_rows=1 .*layout_widths=exact-240dp/match/exact-132dp/zero-dp layout_heights=wrap/wrap/exact-120dp/exact-64dp/exact-56dp layout_weights=0/1 row_padding_ltrb_dp=6/4/2/8 button_margins_ltrb_dp=2/1/4/3\+6/5/2/1 row_geometry=apk-mixed-fixed-weighted-remaining-space .*app_defined_calls=0/0 instance_calls=0/0 field_reads=2/2 direct_strings=true/true dynamic_strings=true/true int_state=1/2 .*errors=0 queues_empty=1$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    DEX_METHODS_PROFILE_RECORDS="$(
      { grep -hE \
          '^ANDROIDBOX_LAYOUT_MIXED19_PROFILE_OK .*abi=69 .*protocol=BNDAPC14 protocol_version=14 descriptor_version=5 descriptor_bytes=24 .*fixture_exact_sizes_dp=title-width-240\+row-height-120\+fixed-button-width-132\+button-heights-64/56 .*fixture_width_modes=fixed-132dp/weighted-remaining fixture_weights=0/1 .*physical_button_geometry=84/618/264/128\+368/626/276/112 .*remaining_space_distribution=parent-minus-fixed-minus-margins .*screen=720x1600 aspect=20:9 .*network=disabled .*real_phone_claim=0$' \
          "$ARTIFACT_DIR"/*.serial.normalized.log || true; } \
        | wc -l | tr -d '[:space:]'
    )"
    APP_DEFINED_INSTANCE_CALLS=0/0
    ACTIVITY_FIELD_READS=2/2
    ACTIVITY_INT_STATES=1/2
    DIRECT_STRING_TEXTS=true/true
    DYNAMIC_STRING_TEXTS=true/true
  fi
  [[ "$APP_DEFINED_METHOD_TRANSCRIPTS" == 2 ]] \
    || fail_gate "ABI $ABI did not prove exactly two app-defined method RPC transcripts."
  [[ "$DEX_METHODS_PROFILE_RECORDS" == 3 ]] \
    || fail_gate "ABI $ABI did not publish its bounded DEX method profile on all three boots."
else
  APP_DEFINED_METHOD_TRANSCRIPTS=0
  DEX_METHODS_PROFILE_RECORDS=0
  APP_DEFINED_INSTANCE_CALLS=0/0
  ACTIVITY_FIELD_READS=0/0
  ACTIVITY_INT_STATES=0/0
  DIRECT_STRING_TEXTS=false/false
  : >"$DEX_METHOD_RASTER_EVIDENCE"
fi

if [[ "$ABI" == 57 || "$ABI" == 58 || "$ABI" == 59 || "$ABI" == 60 || "$ABI" == 61 || "$ABI" == 62 || "$ABI" == 63 || "$ABI" == 64 || "$ABI" == 65 || "$ABI" == 66 || "$ABI" == 67 || "$ABI" == 68 || "$ABI" == 69 ]]; then
  DENSITY_ICON_READS="$(
    grep -hEc \
      '^ANDROID_PACKAGE_ICON_READ_OK .*present=1 .*source_width=48 source_height=48 source_density_dpi=160 color_quantized=0 ' \
      "$ARTIFACT_DIR"/*.serial.normalized.log \
      | awk '{ total += $1 } END { print total + 0 }'
  )"
  [[ "$DENSITY_ICON_READS" -ge 8 ]] \
    || fail_gate "ABI $ABI did not publish enough package-bound mdpi normalization records."
else
  DENSITY_ICON_READS=0
fi

case "$ABI" in
  69)
    RPC_PROTOCOL=BNDAPC14
    RPC_PROTOCOL_VERSION=14
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  68)
    RPC_PROTOCOL=BNDAPC14
    RPC_PROTOCOL_VERSION=14
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  67)
    RPC_PROTOCOL=BNDAPC13
    RPC_PROTOCOL_VERSION=13
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  66)
    RPC_PROTOCOL=BNDAPC12
    RPC_PROTOCOL_VERSION=12
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  65)
    RPC_PROTOCOL=BNDAPC11
    RPC_PROTOCOL_VERSION=11
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  64)
    RPC_PROTOCOL=BNDAPC10
    RPC_PROTOCOL_VERSION=10
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  63)
    RPC_PROTOCOL=BNDAPC09
    RPC_PROTOCOL_VERSION=9
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=java-lang-StringBuilder
    DEX_METHOD_ACCESS=framework-subset
    DEX_METHOD_PROTO=String-plus-I-to-String
    ;;
  62)
    RPC_PROTOCOL=BNDAPC08
    RPC_PROTOCOL_VERSION=8
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=same-activity
    DEX_METHOD_ACCESS=private
    DEX_METHOD_PROTO=CharSequence-direct
    ;;
  61)
    RPC_PROTOCOL=BNDAPC07
    RPC_PROTOCOL_VERSION=7
    APP_DEFINED_CALLS=1/1
    DEX_METHOD_OWNER=same-activity
    DEX_METHOD_ACCESS=private-instance
    DEX_METHOD_PROTO=View-to-I
    ;;
  60)
    RPC_PROTOCOL=BNDAPC06
    RPC_PROTOCOL_VERSION=6
    APP_DEFINED_CALLS=1/1
    DEX_METHOD_OWNER=same-activity
    DEX_METHOD_ACCESS=private-instance
    DEX_METHOD_PROTO=View-to-I
    ;;
  59)
    RPC_PROTOCOL=BNDAPC05
    RPC_PROTOCOL_VERSION=5
    APP_DEFINED_CALLS=1/1
    DEX_METHOD_OWNER=same-activity
    DEX_METHOD_ACCESS=private-instance
    DEX_METHOD_PROTO=View-to-I
    ACTIVITY_FIELD_READS=0/0
    ;;
  58)
    RPC_PROTOCOL=BNDAPC04
    RPC_PROTOCOL_VERSION=4
    APP_DEFINED_CALLS=1/1
    DEX_METHOD_OWNER=same-activity
    DEX_METHOD_ACCESS=private-static
    DEX_METHOD_PROTO=I-to-I
    ;;
  *)
    RPC_PROTOCOL=BNDAPC03
    RPC_PROTOCOL_VERSION=3
    APP_DEFINED_CALLS=0/0
    DEX_METHOD_OWNER=none
    DEX_METHOD_ACCESS=none
    DEX_METHOD_PROTO=none
    ;;
esac

if [[ "$ABI" == 69 ]]; then
  python3 "$SCRIPT_DIR/verify-mobile-layer-copy.py" \
    --callback-pixels 80512 --callback-pixels 77632 \
    "$BOOT_NORMALIZED_LOG" >"$COPY_EVIDENCE"
fi

printf '%s\n' \
  "$TERMINAL" \
  "abi=$ABI" \
  'capacity=2' \
  "package0=$ENVELOPE_PACKAGE" \
  "activity0=$ENVELOPE_ACTIVITY" \
  "apk0_bytes=$ENVELOPE_BYTES" \
  "apk0_sha256=$ENVELOPE_SHA256" \
  "package1=$CATALOG_PACKAGE" \
  "activity1=$CATALOG_ACTIVITY" \
  "apk1_bytes=$CATALOG_BYTES" \
  "apk1_sha256=$CATALOG_SHA256" \
  'two_distinct_packages=1' \
  'two_launcher_entries=1' \
  "apk_launcher_icon_pixels=$ICON_PIXELS" \
  "apk_activity_header_icon_pixels=$ICON_PIXELS" \
  "apk_overview_recent_icon_pixels=$OVERVIEW_ICON_PIXELS" \
  'overview_icon_source=verified-package-catalog' \
  'overview_icon_binding=session+generation+apk-digest' \
  'overview_icon_activity_pixels=0' \
  'overview_icon_thumbnail_pixels=0' \
  "density_icon_reads=$DENSITY_ICON_READS" \
  "selected_source_density_dpi=$([[ "$ABI" == 57 || "$ABI" == 58 || "$ABI" == 59 || "$ABI" == 60 || "$ABI" == 61 || "$ABI" == 62 || "$ABI" == 63 || "$ABI" == 64 || "$ABI" == 65 || "$ABI" == 66 || "$ABI" == 67 || "$ABI" == 68 || "$ABI" == 69 ]] && printf 160 || printf 0)" \
  "selected_source_dimensions=$([[ "$ABI" == 57 || "$ABI" == 58 || "$ABI" == 59 || "$ABI" == 60 || "$ABI" == 61 || "$ABI" == 62 || "$ABI" == 63 || "$ABI" == 64 || "$ABI" == 65 || "$ABI" == 66 || "$ABI" == 67 || "$ABI" == 68 || "$ABI" == 69 ]] && printf 48x48 || printf 16x16)" \
  "icon_read_authority=launcher+settings-only" \
  "rpc_protocol=$RPC_PROTOCOL" \
  "rpc_protocol_version=$RPC_PROTOCOL_VERSION" \
  "app_defined_method_transcripts=$APP_DEFINED_METHOD_TRANSCRIPTS" \
  "app_defined_calls_per_transcript=$APP_DEFINED_CALLS" \
  "app_defined_instance_calls_per_transcript=$APP_DEFINED_INSTANCE_CALLS" \
  "activity_field_reads_per_transcript=$ACTIVITY_FIELD_READS" \
  "activity_int_state_values_per_transcript=$ACTIVITY_INT_STATES" \
  "direct_string_texts_per_transcript=$DIRECT_STRING_TEXTS" \
  "dynamic_string_texts_per_transcript=$DYNAMIC_STRING_TEXTS" \
  "dex_methods_profile_records=$DEX_METHODS_PROFILE_RECORDS" \
  "dex_method_owner=$DEX_METHOD_OWNER" \
  "dex_method_access=$DEX_METHOD_ACCESS" \
  "dex_method_proto=$DEX_METHOD_PROTO" \
  'settings_two_package_selector=1' \
  'settings_selection_storage_mutation=0' \
  'two_activity_launches_same_boot=1' \
  'distinct_activity_headers=1' \
  'source_free_recovery=1' \
  'source_free_writes=0' \
  'qemu_starts=3' \
  'qemu_network=disabled' \
  'aosp_downloaded=0' \
  'storage_handle_granted=0' \
  'block_handle_granted=0' \
  "initial_disk_sha256=$INITIAL_DISK_SHA256" \
  "one_package_disk_sha256=$ONE_PACKAGE_DISK_SHA256" \
  "two_package_disk_sha256=$TWO_PACKAGE_DISK_SHA256" \
  "final_disk_sha256=$FINAL_DISK_SHA256" \
  "raster_evidence=$RASTER_EVIDENCE" \
  "dex_method_raster_evidence=$DEX_METHOD_RASTER_EVIDENCE" \
  >"$SUMMARY"

printf '%s\n' \
  "$TERMINAL" \
  "Evidence: $ARTIFACT_DIR" \
  "Summary: $SUMMARY"
