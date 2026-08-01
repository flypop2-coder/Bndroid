#!/usr/bin/env bash
set -euo pipefail

# Strict offline ABI-52 AndroidApp runtime-uninstall gate.
#
# A real SDK-built APK is installed on boot 1. Boot 2 has no APK source and
# removes that exact package through Settings -> Apps with two distinct taps.
# Boot 3 proves the durable tombstone survives source-free recovery. Cleanup
# owns only the exact QEMU PID captured from `$!`; every VM has networking
# disabled explicitly.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

if (($#)); then
  printf '%s\n' 'Usage: check-androidbox-runtime-uninstall1.sh' >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed head sort; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the runtime-uninstall gate." >&2
    exit 1
  }
done

FIXTURE_DIR="$WORKSPACE_ROOT/fixtures/androidbox-envelope-demo"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
FIXTURE_APK="$FIXTURE_DIR/androidbox-envelope-demo.apk"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="$SCRIPT_DIR/build-storage-image.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in "$FIXTURE_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$QMP_HELPER"; do
  [[ -x "$helper" ]] || {
    echo "Required helper is not executable: $helper" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-120}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

ABI=52
EXPECTED_APK_BYTES=12646
EXPECTED_APK_SHA256=a65584441a524698bcae4810e558bc6b947eb81275fa5f0f5df914304647e3c5
EXPECTED_SIGNER_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
PACKAGE=org.bndroid.envelope
ACTIVITY='Lorg/bndroid/envelope/MainActivity;'
VERSION_CODE=1
FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0,androidbox-process0,androidbox-restart0,androidbox-scene-rpc2,androidbox-multiaction3,androidbox-apk-envelope4,androidbox-runtime-uninstall1

mkdir -p "$WORKSPACE_ROOT/target/androidbox-runtime-uninstall1"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-runtime-uninstall1/check.XXXXXX"
)"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

SOURCE_APK="$ARTIFACT_DIR/envelope.apk"
FIXTURE_LOG="$ARTIFACT_DIR/fixture-build.log"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_LOG="$ARTIFACT_DIR/storage-build.log"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-runtime-uninstall1-build"
TARGET_RELEASE="$TARGET_ROOT/aarch64-unknown-none/release"
KERNEL_IMAGE="$TARGET_RELEASE/bndroid-kernel.img"
SETTINGS_INSTALLED="$ARTIFACT_DIR/settings-apps-installed.ppm"
SETTINGS_CONFIRM="$ARTIFACT_DIR/settings-apps-confirm.ppm"
SETTINGS_REMOVED="$ARTIFACT_DIR/settings-apps-removed.ppm"
SETTINGS_REBOOT="$ARTIFACT_DIR/settings-apps-removed-reboot.ppm"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
SUMMARY="$ARTIFACT_DIR/summary.txt"

QEMU_PID=""
BOOT_MODE=""
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
  echo "Runtime-uninstall evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_bad_output() {
  normalize_log
  local bad='fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_CHILD_DIAG|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROIDBOX_RUNTIME_UNINSTALL1_PROFILE_FAIL|ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_FAIL'
  if grep -Eqi "$bad" "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fatal, child failure, or uninstall failure."
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

canonical_ppm() {
  local kind="$1"
  local screenshot="$2"
  python3 - "$kind" "$screenshot" <<'PY'
from pathlib import Path
import sys

kind, filename = sys.argv[1:]
path = Path(filename)
parts = path.read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)

def pixel(x: int, y: int) -> bytes:
    offset = (y * 720 + x) * 3
    return pixels[offset:offset + 3]

if any(pixel(x, y) != b"\0\0\0" for x, y in (
    (0, 0), (719, 0), (0, 1599), (719, 1599),
)):
    raise SystemExit(1)
if len({pixels[index:index + 3] for index in range(0, len(pixels), 3)}) < 80:
    raise SystemExit(1)
if kind == "apps":
    if pixel(360, 200) != bytes((30, 27, 79)) \
            or pixel(360, 220) != bytes((29, 26, 78)):
        raise SystemExit(1)
elif kind == "confirm":
    if pixel(360, 200) != bytes((16, 21, 36)) \
            or pixel(360, 500) != bytes((32, 45, 73)):
        raise SystemExit(1)
else:
    raise SystemExit(1)
PY
}

wait_for_screenshot() {
  local kind="$1"
  local screenshot="$2"
  local different_from="$3"
  local description="$4"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  qmp move 700 1500
  while ((SECONDS < deadline)); do
    reject_bad_output
    qmp screenshot "$screenshot"
    if canonical_ppm "$kind" "$screenshot" \
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
  local -a source_args=(-name "Bndroid Runtime Uninstall-1 Gate $mode")
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

open_settings_apps() {
  local screenshot="$1"
  local description="$2"
  local previous="$3"
  qmp drag 360 1390 360 620
  wait_for_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=none ' \
    "the unlocked Home state"
  qmp tap 615 1435
  wait_for_pattern \
    '^UI_ROUTE_FOCUS_OK .* receiver_image=app .* active_client=app app=settings ' \
    "Settings focus"
  local app_commits
  app_commits="$(log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} ")"
  qmp tap 360 1350
  wait_for_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
    "$((app_commits + 3))" \
    "the Apps page frames"
  wait_for_screenshot apps "$screenshot" "$previous" "$description"
}

# Produce the exact real Android package using only the already-installed SDK.
"$FIXTURE_BUILD" >"$FIXTURE_LOG" 2>&1 \
  || { tail -n 180 "$FIXTURE_LOG" >&2; exit 1; }
cp "$FIXTURE_APK" "$SOURCE_APK"
APK_BYTES="$(wc -c <"$SOURCE_APK" | tr -d '[:space:]')"
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_BYTES" == "$EXPECTED_APK_BYTES" && "$APK_SHA256" == "$EXPECTED_APK_SHA256" ]] \
  || { echo "Pinned runtime-uninstall APK length or digest changed." >&2; exit 1; }

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$FEATURES" \
  BNDROID_USERSPACE_FEATURES="$FEATURES" \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 320 "$BUILD_LOG" >&2
  echo "ABI-52 Runtime Uninstall-1 build failed." >&2
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

# Boot 1: source-backed install, no UI mutation.
start_qemu install "$SOURCE_APK"
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=1 source_admitted=1 .* operation=install .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} .*" \
  "the sourced package install"
wait_for_pattern \
  "^ANDROIDBOX_RUNTIME_UNINSTALL1_PROFILE_OK format=1 abi=${ABI} parent_profile=androidbox-apk-envelope4 caller=system-app syscall=63 wire=BNDURQ01/BNDURT01 wire_bytes=256 confirmation=two-step catalog=live durable=tombstone async_monitor=1 irq_masked_io=0 generation_bound=1 version_bound=1 package_bound=1 apk_digest_bound=1 signer_digest_bound=1 data_disposition=no-managed-package-data logical_apk_revocation=1 apk_blob_erased=0 storage_handle_granted=0 block_handle_granted=0 arbitrary_path=0 launcher_authority=0 android_app_authority=0 network=disabled general_android_compatibility=0$" \
  "the ABI-52 runtime-uninstall profile marker"
wait_for_pattern "^MOBILE_UI_PREVIEW_OK .*abi=${ABI} width=720 height=1600 " \
  "the ABI-52 720x1600 preview"
stop_qemu
INSTALL_LOG="$BOOT_NORMALIZED_LOG"
INSTALLED_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$INSTALLED_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] \
  || fail_gate "The install boot did not change the package disk."

# Boot 2: source-free recovery, Settings double confirmation, durable removal.
start_qemu uninstall ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* mutation_performed=0 .*" \
  "source-free installed-package recovery"
wait_for_pattern "^ANDROID_APP_PROCESS_OK .* abi=${ABI} " "the process topology"
normalize_log
APP_PID="$(
  sed -n 's/^ANDROID_APP_PROCESS_OK .* app_pid=\([1-9][0-9]*\) .*/\1/p' \
    "$BOOT_NORMALIZED_LOG" | head -n 1
)"
ANDROID_APP_PID="$(
  sed -n 's/^ANDROID_APP_PROCESS_OK .* android_app_pid=\([1-9][0-9]*\) .*/\1/p' \
    "$BOOT_NORMALIZED_LOG" | head -n 1
)"
[[ "$APP_PID" =~ ^[1-9][0-9]*$ && "$ANDROID_APP_PID" =~ ^[1-9][0-9]*$ ]] \
  || fail_gate "Could not derive App and AndroidApp process identities."
[[ "$APP_PID" != "$ANDROID_APP_PID" ]] \
  || fail_gate "App and AndroidApp identities unexpectedly match."

open_settings_apps "$SETTINGS_INSTALLED" "the installed Apps page raster" ""

APP_COMMITS_BEFORE_CONFIRM="$(
  log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} "
)"
qmp tap 360 1460
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE_CONFIRM + 2))" \
  "the uninstall confirmation frames"
wait_for_screenshot \
  confirm "$SETTINGS_CONFIRM" "$SETTINGS_INSTALLED" "the first-tap confirmation raster"
[[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_OK ')" == 0 ]] \
  || fail_gate "The first uninstall tap performed a durable mutation."
[[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_UNINSTALL_COLLECT_OK ')" == 0 ]] \
  || fail_gate "The first uninstall tap collected a terminal result."

APP_COMMITS_BEFORE_REMOVE="$(
  log_count "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} "
)"
qmp tap 515 1070
wait_for_pattern \
  "^ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_OK owner=${APP_PID} request_sequence=1 operation_id=[1-9][0-9]* last_generation=1 removal_generation=2 package=${PACKAGE} reads=[1-9][0-9]* writes=[1-9][0-9]* flushes=[1-9][0-9]* logical_apk_reachable=0 apk_blob_erased=0 data_disposition=no-managed-package-data$" \
  "the exact durable runtime uninstall"
wait_for_pattern \
  "^ANDROID_PACKAGE_RUNTIME_UNINSTALL_COLLECT_OK owner=${APP_PID} request_sequence=1 operation_id=[1-9][0-9]* last_generation=1 removal_generation=2 bytes=256 reads=[1-9][0-9]* writes=[1-9][0-9]* flushes=[1-9][0-9]* authority_granted=0 package_store_handle=0 block_handle=0 arbitrary_path=0$" \
  "the exact App-owned terminal result"
wait_for_pattern \
  '^ANDROID_PACKAGE_SNAPSHOT_READ_OK image=7 bytes=640 installed=0 generation=0 version_code=0 package_writes=0 authority_granted=0 apk_bytes_exposed=0$' \
  "the empty live package catalog"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE_REMOVE + 3))" \
  "the terminal removed Apps page frame"
wait_for_screenshot \
  apps "$SETTINGS_REMOVED" "$SETTINGS_CONFIRM" "the immediate removed Apps page raster"
[[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_UNINSTALL_DURABLE_OK ')" == 1 ]] \
  || fail_gate "Runtime uninstall did not execute exactly once."
[[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_UNINSTALL_COLLECT_OK ')" == 1 ]] \
  || fail_gate "Runtime uninstall did not collect exactly once."
[[ "$(log_count "^ANDROID_PACKAGE_RUNTIME_UNINSTALL_(DURABLE|COLLECT)_OK owner=${ANDROID_APP_PID} ")" == 0 ]] \
  || fail_gate "The AndroidApp worker unexpectedly owned uninstall authority."
stop_qemu
UNINSTALL_LOG="$BOOT_NORMALIZED_LOG"
REMOVED_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$REMOVED_DISK_SHA256" != "$INSTALLED_DISK_SHA256" ]] \
  || fail_gate "The runtime uninstall did not change the package disk."

# Boot 3: no source and no request; recover the same durable tombstone.
start_qemu removed-reboot ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_REMOVED_OK format=3 formatted_this_boot=0 source_present=0 source_admitted=0 uninstall_request_present=0 uninstall_request_used=0 operation=removed-recovery previous_generation=2 previous_version_code=${VERSION_CODE} installed=0 removed=1 removal_generation=2 last_generation=1 last_version_code=${VERSION_CODE} last_apk_bytes=${APK_BYTES} last_blob_slot=[01] package=${PACKAGE} data_disposition=no-managed-package-data tombstone_policy=identical-dual-registry logical_apk_reachable=0 apk_blob_erased=0 mutation_performed=0 reads=[1-9][0-9]* writes=0 flushes=0 source_free_zero_writes=1 uninstall_replay_zero_writes=0 old_kernel_downgrade_safe=0 network=disabled el0_package_write=0 general_android_compatibility=0$" \
  "source-free durable tombstone recovery"
wait_for_pattern "^MOBILE_UI_PREVIEW_OK .*abi=${ABI} width=720 height=1600 " \
  "the rebooted ABI-52 720x1600 preview"
wait_for_pattern "^ANDROID_APP_PROCESS_OK .* abi=${ABI} " "the rebooted process topology"
normalize_log
APP_PID="$(
  sed -n 's/^ANDROID_APP_PROCESS_OK .* app_pid=\([1-9][0-9]*\) .*/\1/p' \
    "$BOOT_NORMALIZED_LOG" | head -n 1
)"
[[ "$APP_PID" =~ ^[1-9][0-9]*$ ]] \
  || fail_gate "Could not derive the rebooted App process identity."
open_settings_apps \
  "$SETTINGS_REBOOT" "the reboot-persistent removed Apps page raster" ""
[[ "$(log_count '^ANDROID_PACKAGE_RUNTIME_UNINSTALL_(DURABLE|COLLECT)_OK ')" == 0 ]] \
  || fail_gate "The reboot unexpectedly replayed a runtime uninstall syscall."
stop_qemu
REBOOT_LOG="$BOOT_NORMALIZED_LOG"
REBOOT_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$REBOOT_DISK_SHA256" == "$REMOVED_DISK_SHA256" ]] \
  || fail_gate "Source-free removed recovery changed the package disk."
[[ "$QEMU_STARTS" == 3 ]] || fail_gate "The gate did not start exactly three owned VMs."

python3 - \
  "$SETTINGS_INSTALLED" "$SETTINGS_CONFIRM" "$SETTINGS_REMOVED" \
  "$SETTINGS_REBOOT" "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

installed_path, confirm_path, removed_path, reboot_path, evidence_path = map(
    Path, sys.argv[1:]
)

def read(path: Path) -> tuple[bytes, bytes]:
    payload = path.read_bytes()
    parts = payload.split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{path.name}: noncanonical PPM")
    if len(parts[3]) != 720 * 1600 * 3:
        raise SystemExit(f"{path.name}: wrong pixel length")
    return payload, parts[3]

frames = [read(path) for path in (
    installed_path, confirm_path, removed_path, reboot_path
)]
if frames[0][0] == frames[1][0]:
    raise SystemExit("confirmation is pixel-identical to the installed page")
if frames[1][0] == frames[2][0]:
    raise SystemExit("terminal removal is pixel-identical to confirmation")
if frames[0][0] == frames[2][0]:
    raise SystemExit("terminal removal is pixel-identical to installed state")

if frames[0][0] == frames[3][0]:
    raise SystemExit("reboot-persistent empty catalog looks installed")
if frames[2][0] == frames[3][0]:
    raise SystemExit("session removal result leaked across reboot")

def changed_pixels(left: bytes, right: bytes) -> int:
    return sum(
        left[index:index + 3] != right[index:index + 3]
        for index in range(0, len(left), 3)
    )

evidence_path.write_text(
    "\n".join((
        "width=720",
        "height=1600",
        "format=P6",
        f"installed_sha256={sha256(frames[0][0]).hexdigest()}",
        f"confirm_sha256={sha256(frames[1][0]).hexdigest()}",
        f"removed_sha256={sha256(frames[2][0]).hexdigest()}",
        f"reboot_removed_sha256={sha256(frames[3][0]).hexdigest()}",
        f"installed_to_confirm_changed_pixels={changed_pixels(frames[0][1], frames[1][1])}",
        f"confirm_to_removed_changed_pixels={changed_pixels(frames[1][1], frames[2][1])}",
        f"installed_to_reboot_empty_changed_pixels={changed_pixels(frames[0][1], frames[3][1])}",
        f"removed_session_to_reboot_empty_changed_pixels={changed_pixels(frames[2][1], frames[3][1])}",
    )) + "\n",
    encoding="utf-8",
)
PY

for log in "$INSTALL_LOG" "$UNINSTALL_LOG" "$REBOOT_LOG"; do
  grep -Eq '^MOBILE_UI_PREVIEW_OK .*width=720 height=1600 ' "$log" \
    || fail_gate "A boot lacked the full-size mobile preview marker."
done
[[ "$QEMU_NETWORK_DISABLED_STARTS" == "$QEMU_STARTS" ]] \
  || fail_gate "A gate-owned VM did not receive explicit network disablement."

printf '%s\n' \
  'ANDROIDBOX_RUNTIME_UNINSTALL1_QEMU_OK' \
  "abi=$ABI" \
  "apk_package=$PACKAGE" \
  "apk_activity=$ACTIVITY" \
  "apk_bytes=$APK_BYTES" \
  "apk_sha256=$APK_SHA256" \
  "signer_sha256=$EXPECTED_SIGNER_SHA256" \
  "qemu_starts=$QEMU_STARTS" \
  'qemu_network=disabled' \
  'caller=system-app' \
  "caller_pid=$APP_PID" \
  'wire=BNDURQ01/BNDURT01' \
  'wire_bytes=256' \
  'confirmation=two-step' \
  'last_generation=1' \
  'removal_generation=2' \
  'data_disposition=no-managed-package-data' \
  'logical_apk_reachable=0' \
  'apk_blob_erased=0' \
  'storage_handle_granted=0' \
  'block_handle_granted=0' \
  'arbitrary_path=0' \
  "installed_disk_sha256=$INSTALLED_DISK_SHA256" \
  "removed_disk_sha256=$REMOVED_DISK_SHA256" \
  "reboot_disk_sha256=$REBOOT_DISK_SHA256" \
  "raster_evidence=$RASTER_EVIDENCE" \
  >"$SUMMARY"

printf '%s\n' \
  'ANDROIDBOX_RUNTIME_UNINSTALL1_QEMU_OK' \
  "Evidence: $ARTIFACT_DIR" \
  "Summary: $SUMMARY"
