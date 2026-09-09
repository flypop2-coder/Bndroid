#!/usr/bin/env bash
set -euo pipefail

# Strict offline AndroidBox Process-0 end-to-end gate.
#
# The gate admits only the checked-in InteractiveActivity-1 fixture and the
# bounded ABI-47 compatibility worker. It builds the fixture twice, proves the
# APK and the App/AndroidApp ELF identities are distinct and deterministic,
# installs generation 1 into one deterministic package disk, then performs a
# source-free recovery boot. The recovery boot proves the authenticated
# App<->AndroidApp RPC transcript, exact capability graph, unchanged
# SurfaceServer-owned chrome, and byte-identical recovered package disk.
#
# Passing this gate is deliberately not evidence of ART/Dalvik, Binder/Bionic,
# JNI, Android Framework completeness, arbitrary Android applications, general
# APK compatibility, hardware support, or a real-phone operating system.
#
# Process ownership is narrow: cleanup sends TERM only to the exact QEMU PID
# stored from `$!` by this script. It never searches for, attaches to, or
# terminates a pre-existing emulator or any other process.
#
# ABI-47 serial marker contract
# =============================
# Some producer-side marker plumbing may land after this gate. The gate
# intentionally fails closed until all of these exact marker fields exist.
#
# ANDROIDBOX_PROCESS0_PROFILE_OK:
#   format abi parent_profile snapshot_syscall snapshot_readers
#   package_image_claim_syscall package_image_claim_owner package_image_grant
#   package_image_binding package_image_transport package_image_handle_rights
#   apk_bytes_exposed_to_launcher apk_bytes_exposed_to_app
#   apk_bytes_exposed_to_android_app execution_host standalone_android_process
#   dedicated_android_process trusted_ui_host android_app_stable_handles
#   android_app_channel_rights
#   android_app_surface_handles android_app_graphics_handles
#   android_app_input_handles android_app_storage_handles
#   graphics_identities graphics_handles chrome_owner content_owner
#   content_viewport art dalvik binder general_apk_claim
#   general_android_compatibility_claim crash_recovery_claim network
#   emulator_only real_phone_claim
#
# ANDROID_APP_PROCESS_OK:
#   format abi app_image android_app_image app_pid android_app_pid app_asid
#   android_app_asid app_digest android_app_digest elf_distinct pid_distinct
#   asid_distinct android_app_live worker_handles worker_channel_count
#   worker_unexpected_handle_count private_endpoint_count private_pair_count
#   app_endpoint_count unexpected_private_owner_count endpoints_unique
#   android_app_rights worker_rights_valid app_rights app_rights_valid
#   android_app_duplicate android_app_transfer queues_empty objects_valid
#   isolation_valid valid
#   surface_handles graphics_handles input_handles storage_handles
#
# ANDROID_APP_RPC_OK:
#   format abi protocol app_pid android_app_pid ready_request_id
#   open_request_id click_request_id close_request_id request_order
#   app_sender_authenticated worker_sender_authenticated ready open opened
#   label_chunks label_chunk_bytes label_bytes button_chunks
#   button_chunk_bytes button_bytes click updated update_chunks
#   update_chunk_bytes update_bytes close closed final_revision errors
#   max_outstanding queue_capacity queues_empty

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-process0.sh' \
    '' \
    'Builds the fixed interactive fixture and runs two strictly offline' \
    'ABI-47 QEMU boots: sourced install, then source-free recovery with one' \
    'bounded App/AndroidApp Open, Click, and Close RPC lifecycle.'
}

if (($#)); then
  case "$1" in
    --help|-h)
      if (($# == 1)); then
        usage
        exit 0
      fi
      ;;
  esac
  usage >&2
  exit 2
fi

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the Process-0 gate." >&2
    exit 1
  }
done

FIXTURE_DIR="$WORKSPACE_ROOT/fixtures/androidbox-interactive-demo"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
FIXTURE_APK="$FIXTURE_DIR/androidbox-interactive-demo.apk"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="$SCRIPT_DIR/build-storage-image.sh"
VERIFY_ELF="$SCRIPT_DIR/verify-init-elf.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in \
  "$FIXTURE_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$VERIFY_ELF" "$QMP_HELPER"; do
  [[ -x "$helper" ]] || {
    echo "Required helper is not executable: $helper" >&2
    exit 1
  }
done

SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
AAPT2="${BNDROID_AAPT2:-"$SDK_ROOT/build-tools/36.1.0/aapt2"}"
APKSIGNER="${BNDROID_APKSIGNER:-"$SDK_ROOT/build-tools/36.1.0/apksigner"}"
for sdk_tool in "$AAPT2" "$APKSIGNER"; do
  case "$sdk_tool" in
    /*) ;;
    *)
      echo "Android SDK tool paths must be absolute: $sdk_tool" >&2
      exit 2
      ;;
  esac
  [[ -x "$sdk_tool" ]] || {
    echo "Required offline Android SDK tool is not executable: $sdk_tool" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-90}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

EXPECTED_APK_SHA256=0b6c7617a6d0491d3ac81ea65eb04f651ad4afe1472eab987a9c36e4280c81ee
EXPECTED_APK_BYTES=12566
EXPECTED_SIGNER_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
PACKAGE=org.bndroid.interactive
ACTIVITY='Lorg/bndroid/interactive/MainActivity;'
ACTIVITY_BADGING=org.bndroid.interactive.MainActivity
VERSION_CODE=1
APP_TITLE='Interactive Android app'
INITIAL_LABEL='Ready for a real APK click'
CLICKED_LABEL='Button callback executed'
BUTTON_LABEL='Update text'
LABEL_VIEW_ID=2130771969
BUTTON_VIEW_ID=2130771968

mkdir -p "$WORKSPACE_ROOT/target/androidbox-process0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-process0/check.XXXXXX"
)"
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR

FIXTURE_BUILD_ONE_LOG="$ARTIFACT_DIR/fixture-build-one.log"
FIXTURE_BUILD_TWO_LOG="$ARTIFACT_DIR/fixture-build-two.log"
FIXTURE_ONE="$ARTIFACT_DIR/fixture-build-one.apk"
SOURCE_APK="$ARTIFACT_DIR/interactive.apk"
BADGING_LOG="$ARTIFACT_DIR/badging.txt"
SIGNER_LOG="$ARTIFACT_DIR/apksigner.txt"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_BUILD_LOG="$ARTIFACT_DIR/storage-build.log"
INITIAL_IMAGE="$ARTIFACT_DIR/packages.initial.raw"
DISK_IMAGE="$ARTIFACT_DIR/packages.raw"
INSTALLED_IMAGE="$ARTIFACT_DIR/packages.installed.raw"
INSTALL_DIFF="$ARTIFACT_DIR/install.disk-diff.txt"
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-process0-build"
TARGET_RELEASE="$TARGET_ROOT/aarch64-unknown-none/release"
KERNEL_IMAGE="$TARGET_RELEASE/bndroid-kernel.img"
APP_ELF="$TARGET_RELEASE/bndroid-app"
ANDROID_APP_ELF="$TARGET_RELEASE/bndroid-android-app"

ACTIVITY_BEFORE="$ARTIFACT_DIR/activity.before.ppm"
ACTIVITY_AFTER="$ARTIFACT_DIR/activity.after.ppm"
DRAWER="$ARTIFACT_DIR/drawer.ppm"
TOP_BEFORE="$ARTIFACT_DIR/activity.top-before.ppm"
TOP_AFTER="$ARTIFACT_DIR/activity.top-after.ppm"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
PROTOCOL_EVIDENCE="$ARTIFACT_DIR/protocol-evidence.txt"
SUMMARY="$ARTIFACT_DIR/summary.txt"

QEMU_PID=""
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
  if [[ -n "$BOOT_NORMALIZED_LOG" && -f "$BOOT_NORMALIZED_LOG" ]]; then
    tail -n 360 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -n "$BOOT_SERIAL_LOG" && -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 360 >&2
  fi
  if [[ -n "$BOOT_QEMU_LOG" && -f "$BOOT_QEMU_LOG" ]]; then
    tail -n 160 "$BOOT_QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Process-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_panic_fault_or_fatal() {
  normalize_log
  if grep -Eqi \
    'fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|DATA_ABORT|INSTRUCTION_ABORT|PAGE_FAULT|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)|ANDROID_APP_(PROCESS|RPC)_FAIL|ANDROIDBOX_PROCESS0_PROFILE_FAIL' \
    "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fault, fatal, or Process-0 failure marker."
  fi
}

wait_for_pattern() {
  local pattern="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_panic_fault_or_fatal
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
    reject_panic_fault_or_fatal
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
  if ! "$QMP_HELPER" "$BOOT_QMP_SOCKET" "$@"; then
    fail_gate "QMP action failed: $*"
  fi
}

layered_commit_count() {
  local producer_pid="$1"
  log_count \
    "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${producer_pid} "
}

app_route_count() {
  local app_pid="$1"
  log_count \
    "^UI_ROUTE_INPUT_OK .* receiver_image=app receiver_pid=${app_pid} "
}

claim_count() {
  local android_app_pid="$1"
  log_count \
    "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${android_app_pid} "
}

ppm_matches() {
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
    reject_panic_fault_or_fatal
    qmp screenshot "$screenshot"
    if ppm_matches "$kind" "$screenshot"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

# One launch site is reused for both boots. Every invocation therefore carries
# one literal network-disable argument. Only the install call adds fw_cfg.
start_qemu() {
  local name="$1"
  local source_apk="$2"
  local -a source_args=(-name "Bndroid Process-0 Gate $name")
  if [[ -n "$source_apk" ]]; then
    source_args+=(-fw_cfg "name=opt/bndroid/apk,file=$source_apk")
  fi

  BOOT_SERIAL_LOG="$ARTIFACT_DIR/$name.serial.log"
  BOOT_NORMALIZED_LOG="$ARTIFACT_DIR/$name.serial.normalized.log"
  BOOT_QEMU_LOG="$ARTIFACT_DIR/$name.qemu.log"
  BOOT_QMP_SOCKET="$ARTIFACT_DIR/$name.qmp.sock"
  : >"$BOOT_SERIAL_LOG"
  : >"$BOOT_NORMALIZED_LOG"
  : >"$BOOT_QEMU_LOG"

  qemu-system-aarch64 \
    -machine virt,gic-version=2,secure=off,virtualization=off \
    -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
    -nic none \
    -rtc base=2026-07-30T10:41:00,clock=vm \
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
  reject_panic_fault_or_fatal
}

require_once_ere() {
  local log="$1"
  local pattern="$2"
  local description="$3"
  if [[ "$(grep -Ec "$pattern" "$log" || true)" != "1" ]]; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Missing or duplicated $description."
  fi
}

# The fixture builder uses only installed SDK/JDK tools. Two whole builds prove
# deterministic fixture bytes before the APK is admitted to the first boot.
if ! "$FIXTURE_BUILD" >"$FIXTURE_BUILD_ONE_LOG" 2>&1; then
  tail -n 220 "$FIXTURE_BUILD_ONE_LOG" >&2
  echo "First deterministic InteractiveActivity-1 fixture build failed." >&2
  exit 1
fi
cp "$FIXTURE_APK" "$FIXTURE_ONE"
if ! "$FIXTURE_BUILD" >"$FIXTURE_BUILD_TWO_LOG" 2>&1; then
  tail -n 220 "$FIXTURE_BUILD_TWO_LOG" >&2
  echo "Second deterministic InteractiveActivity-1 fixture build failed." >&2
  exit 1
fi
cmp "$FIXTURE_ONE" "$FIXTURE_APK" || {
  echo "Two complete fixture builds produced different APK bytes." >&2
  exit 1
}
cp "$FIXTURE_APK" "$SOURCE_APK"

APK_BYTES="$(wc -c <"$SOURCE_APK" | tr -d '[:space:]')"
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_BYTES" == "$EXPECTED_APK_BYTES" \
  && "$APK_SHA256" == "$EXPECTED_APK_SHA256" ]] || {
  echo "Interactive fixture bytes or SHA-256 do not match the pinned source." >&2
  exit 1
}

"$AAPT2" dump badging "$SOURCE_APK" >"$BADGING_LOG"
"$APKSIGNER" verify --verbose --print-certs "$SOURCE_APK" >"$SIGNER_LOG"
for exact in \
  "package: name='$PACKAGE' versionCode='$VERSION_CODE' versionName='1.0' platformBuildVersionName='16' platformBuildVersionCode='36' compileSdkVersion='36' compileSdkVersionCodename='16'" \
  "application-label:'$APP_TITLE'" \
  "launchable-activity: name='$ACTIVITY_BADGING'  label='' icon=''"; do
  [[ "$(grep -Fxc "$exact" "$BADGING_LOG" || true)" == "1" ]] || {
    echo "aapt2 did not report the exact pinned interactive fixture identity." >&2
    exit 1
  }
done
for exact in \
  'Verified using v1 scheme (JAR signing): false' \
  'Verified using v2 scheme (APK Signature Scheme v2): true' \
  'Verified using v3 scheme (APK Signature Scheme v3): false' \
  'Verified using v3.1 scheme (APK Signature Scheme v3.1): false' \
  'Verified using v4 scheme (APK Signature Scheme v4): false' \
  'Number of signers: 1' \
  "Signer #1 certificate SHA-256 digest: $EXPECTED_SIGNER_SHA256"; do
  [[ "$(grep -Fxc "$exact" "$SIGNER_LOG" || true)" == "1" ]] || {
    echo "apksigner did not report the exact v2-only fixture signer." >&2
    exit 1
  }
done

PROCESS0_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0,androidbox-process0
if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES="$PROCESS0_FEATURES" \
  BNDROID_USERSPACE_FEATURES="$PROCESS0_FEATURES" \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 300 "$BUILD_LOG" >&2
  echo "ABI-47 Process-0 kernel/userspace build failed." >&2
  exit 1
fi
for image in "$KERNEL_IMAGE" "$APP_ELF" "$ANDROID_APP_ELF"; do
  [[ -f "$image" ]] || {
    echo "ABI-47 build did not produce the expected image: $image" >&2
    exit 1
  }
done
"$VERIFY_ELF" "$APP_ELF"
"$VERIFY_ELF" "$ANDROID_APP_ELF"
if cmp -s "$APP_ELF" "$ANDROID_APP_ELF"; then
  echo "App and AndroidApp ELFs must be byte-distinct." >&2
  exit 1
fi
APP_ELF_BYTES="$(wc -c <"$APP_ELF" | tr -d '[:space:]')"
ANDROID_APP_ELF_BYTES="$(wc -c <"$ANDROID_APP_ELF" | tr -d '[:space:]')"
APP_ELF_SHA256="$(shasum -a 256 "$APP_ELF" | awk '{print $1}')"
ANDROID_APP_ELF_SHA256="$(shasum -a 256 "$ANDROID_APP_ELF" | awk '{print $1}')"
[[ "$APP_ELF_BYTES" =~ ^[1-9][0-9]*$ \
  && "$ANDROID_APP_ELF_BYTES" =~ ^[1-9][0-9]*$ \
  && "$APP_ELF_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$ANDROID_APP_ELF_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$APP_ELF_SHA256" != "$ANDROID_APP_ELF_SHA256" ]] || {
  echo "Could not retain distinct canonical App/AndroidApp ELF evidence." >&2
  exit 1
}

if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$BUILD_STORAGE" --with-package-store >"$STORAGE_BUILD_LOG" 2>&1; then
  tail -n 180 "$STORAGE_BUILD_LOG" >&2
  echo "The deterministic 16 MiB package-store image build failed." >&2
  exit 1
fi
[[ "$(wc -c <"$INITIAL_IMAGE" | tr -d '[:space:]')" == "16777216" ]] || {
  echo "Package-store image is not exactly 16 MiB." >&2
  exit 1
}
cp "$INITIAL_IMAGE" "$DISK_IMAGE"
INITIAL_DISK_SHA256="$(shasum -a 256 "$INITIAL_IMAGE" | awk '{print $1}')"

# Boot 1: install generation 1 from the only admitted host source.
start_qemu install "$SOURCE_APK"
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=1 source_admitted=1 .* operation=install .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled " \
  "the sourced generation-1 install"
wait_for_pattern \
  '^ANDROIDBOX_PROCESS0_PROFILE_OK .* abi=47 .* general_android_compatibility_claim=0 .* network=disabled .* real_phone_claim=0$' \
  "the bounded ABI-47 Process-0 profile"
stop_qemu
INSTALL_LOG="$BOOT_NORMALIZED_LOG"

require_once_ere \
  "$INSTALL_LOG" \
  "^APK_SOURCE_OK .* present=1 bytes=${APK_BYTES} .* sha256=${APK_SHA256} .* host_directory_scan=0 network=disabled install_mutation=0$" \
  "sourced fw_cfg APK marker"
require_once_ere \
  "$INSTALL_LOG" \
  "^APK_PACKAGE_STORE_OK .* operation=install previous_generation=0 previous_version_code=0 installed=1 removed=0 generation=1 slot=0 apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} profile=Resources-1 .* mutation_performed=1 reads=1032 writes=130 flushes=3 .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled .*" \
  "durable generation-1 install marker"
require_once_ere \
  "$INSTALL_LOG" \
  '^USER_IMAGE_CATALOG_OK count=8 distinct=1 .* app_bytes=[1-9][0-9]* android_app_bytes=[1-9][0-9]* .* app_digest=0x[0-9a-f]+ android_app_digest=0x[0-9a-f]+$' \
  "eight-image ABI-47 catalog"
require_once_ere \
  "$INSTALL_LOG" \
  '^ANDROID_APP_PROCESS_OK .* abi=47 .* android_app_live=1 worker_handles=1 worker_channel_count=1 worker_unexpected_handle_count=0 private_endpoint_count=2 private_pair_count=1 app_endpoint_count=1 unexpected_private_owner_count=0 .* queues_empty=1 objects_valid=1 isolation_valid=1 valid=1 surface_handles=0 graphics_handles=0 input_handles=0 storage_handles=0$' \
  "install steady narrowed AndroidApp process"

INSTALLED_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$INSTALLED_DISK_SHA256" != "$INITIAL_DISK_SHA256" ]] || {
  fail_gate "Generation-1 install did not mutate the empty package store."
}
cp "$DISK_IMAGE" "$INSTALLED_IMAGE"
python3 - "$INITIAL_IMAGE" "$INSTALLED_IMAGE" "$INSTALL_DIFF" <<'PY'
from pathlib import Path
import sys

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
output = Path(sys.argv[3])
if len(before) != 16 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit("install diff requires two exact 16 MiB images")
first = 16384 * 512
last = (16895 + 1) * 512
changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
if not changed:
    raise SystemExit("install changed no disk bytes")
outside = [index for index in changed if not first <= index < last]
if outside:
    raise SystemExit(f"install changed byte {outside[0]} outside BNDROID_PACKAGES")
output.write_text(
    "\n".join(
        (
            f"changed_bytes={len(changed)}",
            f"first_changed_offset={changed[0]}",
            f"last_changed_offset={changed[-1]}",
            "allowed_first_lba=16384",
            "allowed_last_lba=16895",
            "outside_changed_bytes=0",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

# Boot 2 has no fw_cfg APK source. Recover the durable package, open it through
# the independent worker, dispatch one Button callback, and leave the Activity
# so request 3 closes the retained worker session.
RECOVERY_BEFORE_DISK_SHA256="$INSTALLED_DISK_SHA256"
start_qemu recovery ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* writes=0 flushes=0 source_free_zero_writes=1 .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled " \
  "source-free durable package recovery"
wait_for_pattern \
  '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=47 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 buffers=3 .* live_processes=9 .* network=disabled .* real_phone_claim=0$' \
  "the recovery ABI-47 preview"
wait_for_pattern \
  '^ANDROIDBOX_INTERACTIVE0_PROFILE_OK .* abi=47 .* chrome_owner=surface-server .* graphics_identities=3 graphics_handles=6 .* graphics_producer_roles=surface-server\+launcher\+app .* network=disabled .* real_phone_claim=0$' \
  "the unchanged three-identity graphics profile"
wait_for_pattern \
  '^ANDROIDBOX_PROCESS0_PROFILE_OK .* abi=47 .* android_app_stable_handles=1 .* graphics_identities=3 graphics_handles=6 .* chrome_owner=surface-server .* art=0 .* general_android_compatibility_claim=0 .* real_phone_claim=0$' \
  "the complete bounded Process-0 profile"
wait_for_pattern \
  '^ANDROID_APP_PROCESS_OK .* abi=47 .* android_app_live=1 worker_handles=1 worker_channel_count=1 .* queues_empty=1 objects_valid=1 isolation_valid=1 valid=1 surface_handles=0 graphics_handles=0 input_handles=0 storage_handles=0$' \
  "the independent narrowed AndroidApp process"
wait_for_pattern \
  '^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=[1-9][0-9]* chrome_producer_pid=[1-9][0-9]* .* viewport=0/64/720/1448 chrome_regions=0-64/1512-1600 .* atomic_sources=2 system_chrome_owner=surface-server$' \
  "the initial layered Launcher frame"

normalize_log
SURFACE_PID="$(
  awk '/^USER_SURFACE_LAYERED_COMMIT_OK / {
    for (field = 1; field <= NF; field++) {
      if ($field ~ /^pid=/) {
        sub(/^pid=/, "", $field)
        print $field
        exit
      }
    }
  }' "$BOOT_NORMALIZED_LOG"
)"
LAUNCHER_PID="$(
  awk '/^USER_SURFACE_LAYERED_COMMIT_OK / {
    for (field = 1; field <= NF; field++) {
      if ($field ~ /^content_producer_pid=/) {
        sub(/^content_producer_pid=/, "", $field)
        print $field
        exit
      }
    }
  }' "$BOOT_NORMALIZED_LOG"
)"
[[ "$SURFACE_PID" =~ ^[1-9][0-9]*$ \
  && "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ \
  && "$LAUNCHER_PID" != "$SURFACE_PID" ]] || {
  fail_gate "Could not derive distinct Launcher and SurfaceServer owners."
}

# Unlock and launch the pinned installed row.
qmp drag 360 1390 360 620
wait_for_pattern \
  "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=${SURFACE_PID} action=unlock app=none request_id=1 " \
  "the authenticated unlock"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' \
  "the unlocked Home state"
qmp tap 700 900
qmp drag 360 1280 360 520
wait_for_screenshot drawer "$DRAWER" "the installed All apps row"
qmp tap 106 674
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} android_app_owner=[1-9][0-9]* compatible_session_id=1 request_sequence=1 generation=1 bytes=640 reads=389 writes=0 flushes=0 authority_granted=android-app-read-only-vmo apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=0 apk_bytes_exposed_to_android_app=1 storage_authority_granted=0$" \
  "the AndroidApp-bound durable relaunch collection"
normalize_log
ANDROID_APP_PID="$(
  sed -n \
    "s/^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} android_app_owner=\\([1-9][0-9]*\\) compatible_session_id=1 request_sequence=1 .*/\\1/p" \
    "$BOOT_NORMALIZED_LOG"
)"
APP_PID="$(
  sed -n \
    's/^ANDROID_APP_PROCESS_OK .* app_pid=\([1-9][0-9]*\) .*/\1/p' \
    "$BOOT_NORMALIZED_LOG"
)"
[[ "$APP_PID" =~ ^[1-9][0-9]*$ \
  && "$ANDROID_APP_PID" =~ ^[1-9][0-9]*$ \
  && "$APP_PID" != "$ANDROID_APP_PID" \
  && "$APP_PID" != "$LAUNCHER_PID" \
  && "$APP_PID" != "$SURFACE_PID" \
  && "$ANDROID_APP_PID" != "$LAUNCHER_PID" \
  && "$ANDROID_APP_PID" != "$SURFACE_PID" ]] || {
  fail_gate "Could not derive four distinct runtime process owners."
}
wait_for_pattern \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${ANDROID_APP_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} .* rights=READ duplicate=0 transfer=0 map=0 write=0 execute=0 wait=0 storage_authority_granted=0$" \
  "the one-shot AndroidApp image claim"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=app .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "the compatible Activity foreground"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  5 \
  "five App-hosted enter-transition commits"
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the pre-click Activity raster"

# A trusted top-chrome transaction cannot enter App or AndroidApp and cannot
# change a settled Activity pixel.
qmp move 360 32
qmp screenshot "$TOP_BEFORE"
ppm_matches activity "$TOP_BEFORE" || {
  fail_gate "Top-chrome baseline is not the settled Activity."
}
TOP_APP_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
TOP_APP_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
TOP_CLAIMS_BEFORE="$(claim_count "$ANDROID_APP_PID")"
qmp tap 360 32
qmp screenshot "$TOP_AFTER"
[[ "$(app_route_count "$APP_PID")" == "$TOP_APP_ROUTES_BEFORE" \
  && "$(layered_commit_count "$APP_PID")" == "$TOP_APP_COMMITS_BEFORE" \
  && "$(claim_count "$ANDROID_APP_PID")" == "$TOP_CLAIMS_BEFORE" ]] || {
  fail_gate "Trusted top chrome reached App/AndroidApp or mutated the Activity."
}
cmp "$TOP_BEFORE" "$TOP_AFTER" || {
  fail_gate "Trusted top-chrome tap changed the displayed Activity raster."
}

# Request 2 is the one admitted Button transaction. App remains the sole
# content raster producer; AndroidApp receives no input or graphics handle.
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the restored pre-click raster"
BUTTON_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
BUTTON_COORD_ROUTES_BEFORE="$(
  log_count "^UI_ROUTE_INPUT_OK .* receiver_image=app receiver_pid=${APP_PID} .* x=360 y=728 "
)"
BUTTON_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
BUTTON_CLAIMS_BEFORE="$(claim_count "$ANDROID_APP_PID")"
qmp tap 360 728
wait_for_count \
  "^UI_ROUTE_INPUT_OK .* receiver_image=app receiver_pid=${APP_PID} .* x=360 y=728 " \
  "$((BUTTON_COORD_ROUTES_BEFORE + 3))" \
  "all three Button pointer samples"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((BUTTON_COMMITS_BEFORE + 2))" \
  "the Button pressed and authenticated Updated frames"
[[ "$(app_route_count "$APP_PID")" == "$((BUTTON_ROUTES_BEFORE + 3))" \
  && "$(layered_commit_count "$APP_PID")" == "$((BUTTON_COMMITS_BEFORE + 2))" \
  && "$(claim_count "$ANDROID_APP_PID")" == "$BUTTON_CLAIMS_BEFORE" ]] || {
  fail_gate "Button dispatch did not preserve the exact 3-route/2-frame shape."
}
wait_for_screenshot activity "$ACTIVITY_AFTER" "the post-Updated Activity raster"

# Leaving the Activity triggers request 3 Close/Closed. Home is owned by
# SurfaceServer/Launcher; no navigation sample is delivered to AndroidApp.
HOME_APP_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
HOME_APP_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
HOME_CLAIMS_BEFORE="$(claim_count "$ANDROID_APP_PID")"
LAUNCHER_COMMITS_BEFORE="$(layered_commit_count "$LAUNCHER_PID")"
qmp tap 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 ' \
  "Home retaining only compatible identity"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
  "$((LAUNCHER_COMMITS_BEFORE + 1))" \
  "the Launcher Home frame"
wait_for_pattern \
  "^ANDROID_APP_RPC_OK .* abi=47 protocol=BNDAPC01 app_pid=${APP_PID} android_app_pid=${ANDROID_APP_PID} ready_request_id=0 open_request_id=1 click_request_id=2 close_request_id=3 request_order=0/1/2/3 .* close=1 closed=1 .* errors=0 .* queues_empty=1$" \
  "the complete authenticated Process-0 RPC transcript"
[[ "$(app_route_count "$APP_PID")" == "$HOME_APP_ROUTES_BEFORE" \
  && "$(layered_commit_count "$APP_PID")" == "$HOME_APP_COMMITS_BEFORE" \
  && "$(claim_count "$ANDROID_APP_PID")" == "$HOME_CLAIMS_BEFORE" ]] || {
  fail_gate "System Home routed to App/AndroidApp or consumed another image claim."
}

stop_qemu
RECOVERY_LOG="$BOOT_NORMALIZED_LOG"
RECOVERY_AFTER_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$RECOVERY_AFTER_DISK_SHA256" == "$RECOVERY_BEFORE_DISK_SHA256" ]] || {
  fail_gate "Source-free recovery or the RPC lifecycle changed the package disk."
}

require_once_ere \
  "$RECOVERY_LOG" \
  '^APK_SOURCE_OK .* present=0 bytes=0 .* sha256=0000000000000000000000000000000000000000000000000000000000000000 .* host_directory_scan=0 network=disabled install_mutation=0$' \
  "absent recovery APK-source marker"
require_once_ere \
  "$RECOVERY_LOG" \
  '^BLOCK_LAYER_OK .* package_reads=389 package_writes=0 package_flushes=0 .* timeouts=0 .*' \
  "source-free recovery block-layer marker"
require_once_ere \
  "$RECOVERY_LOG" \
  "^APK_PACKAGE_STORE_OK .* operation=recovery previous_generation=1 previous_version_code=${VERSION_CODE} installed=1 removed=0 generation=1 slot=0 apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} profile=Resources-1 .* mutation_performed=0 reads=389 writes=0 flushes=0 source_free_zero_writes=1 .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled .*" \
  "source-free durable recovery marker"
require_once_ere \
  "$RECOVERY_LOG" \
  "^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=${APP_TITLE// /[[:space:]]} text=${INITIAL_LABEL// /[[:space:]]} constructor=1 .* constructor_instructions=2 on_create=1 .* on_create_instructions=8 .* framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$" \
  "recovered bounded Activity preflight"

# Prove the authenticated Updated result changed only client content. Both
# SurfaceServer-owned chrome regions remain byte-identical.
python3 - "$ACTIVITY_BEFORE" "$ACTIVITY_AFTER" "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

before_path, after_path, output_path = map(Path, sys.argv[1:4])

def read_ppm(path: Path) -> tuple[bytes, bytes]:
    payload = path.read_bytes()
    parts = payload.split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{path.name} is not an exact 720x1600 PPM")
    pixels = parts[3]
    if len(pixels) != 720 * 1600 * 3:
        raise SystemExit(f"{path.name} has a non-canonical pixel payload")
    return payload, pixels

before_payload, before = read_ppm(before_path)
after_payload, after = read_ppm(after_path)

def rows(pixels: bytes, first: int, last: int) -> bytes:
    stride = 720 * 3
    return pixels[first * stride:last * stride]

if rows(before, 0, 64) != rows(after, 0, 64):
    raise SystemExit("Updated changed trusted top chrome")
if rows(before, 1512, 1600) != rows(after, 1512, 1600):
    raise SystemExit("Updated changed trusted bottom chrome")

def changed_in(bounds: tuple[int, int, int, int]) -> int:
    left, top, right, bottom = bounds
    changed = 0
    for y in range(top, bottom):
        for x in range(left, right):
            offset = (y * 720 + x) * 3
            changed += before[offset:offset + 3] != after[offset:offset + 3]
    return changed

content_changed = changed_in((0, 64, 720, 1512))
label_changed = changed_in((68, 448, 652, 624))
if content_changed < 3000 or label_changed < 3000:
    raise SystemExit("authenticated Updated did not materially change expected content")
if content_changed != label_changed:
    raise SystemExit("authenticated Updated changed pixels outside the publisher TextView")
if before_payload == after_payload:
    raise SystemExit("pre-click and post-click screenshots are identical")

output_path.write_text(
    "\n".join(
        (
            "width=720",
            "height=1600",
            "content_viewport=0/64/720/1448",
            "top_chrome_rows=0-64",
            "bottom_chrome_rows=1512-1600",
            "top_chrome_byte_identical=1",
            "bottom_chrome_byte_identical=1",
            f"content_changed_pixels={content_changed}",
            f"text_label_changed_pixels={label_changed}",
            "revision_changed_pixels=0",
            "revision_pixels_hidden=1",
            f"initial_ppm_sha256={sha256(before_payload).hexdigest()}",
            f"updated_ppm_sha256={sha256(after_payload).hexdigest()}",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

# Parse canonical key=value markers once. This is the strict ABI-47 proof:
# independent images/PIDs/ASIDs; one narrowed private channel; an exact
# authenticated RPC sequence; unchanged three-identity graphics ownership.
python3 - \
  "$RECOVERY_LOG" \
  "$PROTOCOL_EVIDENCE" \
  "$SURFACE_PID" \
  "$LAUNCHER_PID" \
  "$APP_PID" \
  "$ANDROID_APP_PID" \
  "$APK_BYTES" <<'PY'
from pathlib import Path
import re
import sys

log_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
surface_pid, launcher_pid, app_pid, android_app_pid, apk_bytes = sys.argv[3:8]
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

catalogs = exact("USER_IMAGE_CATALOG_OK ")
previews = exact("MOBILE_UI_PREVIEW_OK ")
interactive_profiles = exact("ANDROIDBOX_INTERACTIVE0_PROFILE_OK ")
process_profiles = exact("ANDROIDBOX_PROCESS0_PROFILE_OK ")
processes = exact("ANDROID_APP_PROCESS_OK ")
rpcs = exact("ANDROID_APP_RPC_OK ")
if any(len(values) != 1 for values in (
    catalogs,
    previews,
    interactive_profiles,
    process_profiles,
    processes,
    rpcs,
)):
    raise SystemExit("expected one canonical ABI-47 catalog/profile/process/RPC marker")

catalog = catalogs[0][1]
for name, expected in {"count": "8", "distinct": "1"}.items():
    if catalog.get(name) != expected:
        raise SystemExit(f"catalog field mismatch: {name}")
for name in ("app_bytes", "android_app_bytes"):
    if not re.fullmatch(r"[1-9][0-9]*", catalog.get(name, "")):
        raise SystemExit(f"catalog lacks positive {name}")
for name in ("app_digest", "android_app_digest"):
    if not re.fullmatch(r"0x[0-9a-f]+", catalog.get(name, "")):
        raise SystemExit(f"catalog lacks canonical {name}")
if catalog["app_digest"] == catalog["android_app_digest"]:
    raise SystemExit("App and AndroidApp catalog digests are identical")

preview = previews[0][1]
for name, expected in {
    "abi": "47",
    "width": "720",
    "height": "1600",
    "buffers": "3",
    "live_processes": "9",
    "buffer_present_version": "5",
    "network": "disabled",
    "real_phone_claim": "0",
}.items():
    if preview.get(name) != expected:
        raise SystemExit(f"preview field mismatch: {name}")

interactive = interactive_profiles[0][1]
for name, expected in {
    "abi": "47",
    "sources": "2",
    "content_owner": "launcher-or-app",
    "chrome_owner": "surface-server",
    "content_viewport": "0/64/720/1448",
    "chrome_regions": "0-64/1512-1600",
    "graphics_handle_rights": "verified",
    "graphics_identities": "3",
    "graphics_handles": "6",
    "graphics_identity_handles": "2/2/2",
    "graphics_producer_handles": "1/1/1",
    "graphics_server_handles": "3",
    "graphics_producer_roles": "surface-server+launcher+app",
    "graphics_role_counts": "1/1/1",
    "system_chrome_input_capture": "surface-server",
    "network": "disabled",
    "real_phone_claim": "0",
}.items():
    if interactive.get(name) != expected:
        raise SystemExit(f"Interactive-0 inheritance mismatch: {name}")

profile = process_profiles[0][1]
for name, expected in {
    "format": "1",
    "abi": "47",
    "parent_profile": "androidbox-interactive0",
    "snapshot_syscall": "59",
    "snapshot_readers": "launcher+app+android-app",
    "package_image_claim_syscall": "61",
    "package_image_claim_owner": "android-app-generation",
    "package_image_grant": "one-shot",
    "package_image_transport": "immutable-private-vmo",
    "package_image_handle_rights": "READ",
    "apk_bytes_exposed_to_launcher": "0",
    "apk_bytes_exposed_to_app": "0",
    "apk_bytes_exposed_to_android_app": "1",
    "execution_host": "independent-android-app-el0",
    "standalone_android_process": "1",
    "dedicated_android_process": "1",
    "trusted_ui_host": "app-el0",
    "android_app_stable_handles": "1",
    "android_app_channel_rights": "READ+WRITE+WAIT",
    "android_app_surface_handles": "0",
    "android_app_graphics_handles": "0",
    "android_app_input_handles": "0",
    "android_app_storage_handles": "0",
    "app_apk_digest_reverified": "0",
    "android_app_apk_digest_reverified": "1",
    "android_app_apk_v2_signature_reverified": "1",
    "android_app_signer_reverified": "1",
    "ui_activity_raster_owner": "trusted-app-generation",
    "graphics_identities": "3",
    "graphics_handles": "6",
    "chrome_owner": "surface-server",
    "content_owner": "launcher-or-app",
    "content_viewport": "0/64/720/1448",
    "art": "0",
    "dalvik": "0",
    "binder": "0",
    "general_apk_claim": "0",
    "general_android_compatibility_claim": "0",
    "crash_recovery_claim": "0",
    "network": "disabled",
    "emulator_only": "1",
    "real_phone_claim": "0",
}.items():
    if profile.get(name) != expected:
        raise SystemExit(f"Process-0 profile field mismatch: {name}")

process = processes[0][1]
for name, expected in {
    "format": "1",
    "abi": "47",
    "app_image": "7",
    "android_app_image": "10",
    "app_pid": app_pid,
    "android_app_pid": android_app_pid,
    "app_digest": catalog["app_digest"],
    "android_app_digest": catalog["android_app_digest"],
    "elf_distinct": "1",
    "pid_distinct": "1",
    "asid_distinct": "1",
    "root_distinct": "1",
    "process_capacity": "9",
    "dynamic_capacity": "8",
    "live_processes": "9",
    "android_app_live": "1",
    "worker_handles": "1",
    "worker_channel_count": "1",
    "worker_unexpected_handle_count": "0",
    "private_endpoint_count": "2",
    "private_pair_count": "1",
    "app_endpoint_count": "1",
    "unexpected_private_owner_count": "0",
    "endpoints_unique": "1",
    "android_app_rights": "0x00000103",
    "worker_rights_valid": "1",
    "app_rights": "channel-default",
    "app_rights_valid": "1",
    "android_app_duplicate": "0",
    "android_app_transfer": "0",
    "queues_empty": "1",
    "objects_valid": "1",
    "isolation_valid": "1",
    "valid": "1",
    "surface_handles": "0",
    "graphics_handles": "0",
    "input_handles": "0",
    "storage_handles": "0",
}.items():
    if process.get(name) != expected:
        raise SystemExit(f"AndroidApp process field mismatch: {name}")
for name in ("app_asid", "android_app_asid"):
    if not re.fullmatch(r"[1-9][0-9]*", process.get(name, "")):
        raise SystemExit(f"invalid {name}")
if process["app_asid"] == process["android_app_asid"]:
    raise SystemExit("App and AndroidApp share one ASID")
for name in ("app_root", "android_app_root"):
    if not re.fullmatch(r"0x[0-9a-f]{16}", process.get(name, "")):
        raise SystemExit(f"invalid {name}")
if process["app_root"] == process["android_app_root"]:
    raise SystemExit("App and AndroidApp share one translation root")

rpc = rpcs[0][1]
for name, expected in {
    "format": "1",
    "abi": "47",
    "protocol": "BNDAPC01",
    "app_pid": app_pid,
    "android_app_pid": android_app_pid,
    "ready_request_id": "0",
    "open_request_id": "1",
    "click_request_id": "2",
    "close_request_id": "3",
    "request_order": "0/1/2/3",
    "app_sender_authenticated": "1",
    "worker_sender_authenticated": "1",
    "ready": "1",
    "open": "1",
    "opened": "1",
    "label_chunks": "2",
    "label_chunk_bytes": "24/2",
    "label_bytes": "26",
    "button_chunks": "1",
    "button_chunk_bytes": "11",
    "button_bytes": "11",
    "click": "1",
    "updated": "1",
    "update_chunks": "1",
    "update_chunk_bytes": "24",
    "update_bytes": "24",
    "close": "1",
    "closed": "1",
    "final_revision": "1",
    "errors": "0",
    "max_outstanding": "1",
    "queue_capacity": "8",
    "queues_empty": "1",
}.items():
    if rpc.get(name) != expected:
        raise SystemExit(f"AndroidApp RPC field mismatch: {name}")

collects = exact("ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ")
claims = exact("ANDROID_PACKAGE_IMAGE_CLAIM_OK ")
if len(collects) != 1 or len(claims) != 1:
    raise SystemExit("expected exactly one durable collection and image claim")
collect_index, collect = collects[0]
claim_index, claim = claims[0]
for name, expected in {
    "owner": launcher_pid,
    "android_app_owner": android_app_pid,
    "compatible_session_id": "1",
    "request_sequence": "1",
    "generation": "1",
    "bytes": "640",
    "reads": "389",
    "writes": "0",
    "flushes": "0",
    "authority_granted": "android-app-read-only-vmo",
    "apk_bytes_exposed_to_launcher": "0",
    "apk_bytes_exposed_to_app": "0",
    "apk_bytes_exposed_to_android_app": "1",
    "storage_authority_granted": "0",
}.items():
    if collect.get(name) != expected:
        raise SystemExit(f"durable collection mismatch: {name}")
for name, expected in {
    "owner": android_app_pid,
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
}.items():
    if claim.get(name) != expected:
        raise SystemExit(f"image claim mismatch: {name}")
if not collect_index < claim_index < rpcs[0][0]:
    raise SystemExit("collection, claim, and completed RPC marker are out of order")

commits = exact("USER_SURFACE_LAYERED_COMMIT_OK ")
if not commits:
    raise SystemExit("recovery emitted no layered commit")
app_commits = []
launcher_commits = []
for index, value in commits:
    expected = {
        "owner": "surface-server",
        "pid": surface_pid,
        "chrome_producer_pid": surface_pid,
        "mode": "content-plus-system-chrome",
        "format": "xrgb8888",
        "surface": "720x1600",
        "viewport": "0/64/720/1448",
        "chrome_regions": "0-64/1512-1600",
        "atomic_sources": "2",
        "system_chrome_owner": "surface-server",
    }
    if any(value.get(name) != item for name, item in expected.items()):
        raise SystemExit("layered commit ownership or geometry mismatch")
    producer = value.get("content_producer_pid")
    if producer == app_pid:
        app_commits.append((index, value))
    elif producer == launcher_pid:
        launcher_commits.append((index, value))
    else:
        raise SystemExit("layered content came from an unauthorised producer")
    if producer == android_app_pid:
        raise SystemExit("AndroidApp became a graphics producer")
if len(app_commits) < 7 or not launcher_commits:
    raise SystemExit("missing App or Launcher layered producer evidence")

routes = exact("UI_ROUTE_INPUT_OK ")
if any(
    value.get("receiver_pid") == android_app_pid
    or value.get("receiver_image") == "android-app"
    for _index, value in routes
):
    raise SystemExit("an input sample reached AndroidApp")
app_routes = [
    (index, value)
    for index, value in routes
    if value.get("receiver_image") == "app"
    and value.get("receiver_pid") == app_pid
]
if any(
    int(value.get("y", "-1"), 10) < 64
    or int(value.get("y", "-1"), 10) >= 1512
    for _index, value in app_routes
):
    raise SystemExit("trusted chrome routed an input sample to App")
button_routes = [
    (index, value)
    for index, value in app_routes
    if value.get("x") == "360" and value.get("y") == "728"
]
if len(button_routes) != 3:
    raise SystemExit("Button did not receive exactly three samples")
if [value.get("pressed") for _index, value in button_routes] != ["0", "1", "0"]:
    raise SystemExit("Button samples are not unpressed/press/release")
if not claim_index < button_routes[0][0] < button_routes[-1][0] < rpcs[0][0]:
    raise SystemExit("Button input is outside the authenticated worker lease")

if any(
    re.search(
        r"fatal exception:|kernel panic:|panicked at|panic!|boot error:|"
        r"USER_FAIL:|EL0_FAIL|THREAD_EXITED:|DATA_ABORT|INSTRUCTION_ABORT|"
        r"PAGE_FAULT|MOBILE_UI_PREVIEW_(?:TIMEOUT|FAULT|RUNTIME_FAIL)|"
        r"MOBILE_UI_(?:CHILD_DIAG|USER_FAULT)|ANDROID_APP_(?:PROCESS|RPC)_FAIL|"
        r"ANDROIDBOX_PROCESS0_PROFILE_FAIL",
        line,
        re.IGNORECASE,
    )
    for line in lines
):
    raise SystemExit("failure marker survived final recovery validation")

output_path.write_text(
    "\n".join(
        (
            "abi=47",
            "app_image=7",
            "android_app_image=10",
            f"app_pid={app_pid}",
            f"android_app_pid={android_app_pid}",
            f"app_asid={process['app_asid']}",
            f"android_app_asid={process['android_app_asid']}",
            "elf_distinct=1",
            "pid_distinct=1",
            "asid_distinct=1",
            "android_app_live=1",
            "android_app_handles=1",
            "android_app_channels=1",
            "private_endpoints=2",
            "private_pairs=1",
            "private_queues_empty=1",
            "rpc_protocol=BNDAPC01",
            "rpc_request_order=0/1/2/3",
            "rpc_authenticated=1",
            "rpc_ready_open_click_close=1",
            "graphics_identities=3",
            "graphics_handles=6",
            "graphics_producer_roles=surface-server+launcher+app",
            f"layered_commits={len(commits)}",
            f"app_layered_commits={len(app_commits)}",
            f"launcher_layered_commits={len(launcher_commits)}",
            "android_app_layered_commits=0",
            "android_app_input_samples=0",
            "top_chrome_samples_to_app=0",
            "panic_fault_fatal_free=1",
            "network=disabled",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

INITIAL_PPM_SHA256="$(sed -n 's/^initial_ppm_sha256=//p' "$RASTER_EVIDENCE")"
UPDATED_PPM_SHA256="$(sed -n 's/^updated_ppm_sha256=//p' "$RASTER_EVIDENCE")"
CONTENT_CHANGED_PIXELS="$(sed -n 's/^content_changed_pixels=//p' "$RASTER_EVIDENCE")"
LABEL_CHANGED_PIXELS="$(sed -n 's/^text_label_changed_pixels=//p' "$RASTER_EVIDENCE")"
APP_ASID="$(sed -n 's/^app_asid=//p' "$PROTOCOL_EVIDENCE")"
ANDROID_APP_ASID="$(sed -n 's/^android_app_asid=//p' "$PROTOCOL_EVIDENCE")"
APP_LAYERED_COMMITS="$(sed -n 's/^app_layered_commits=//p' "$PROTOCOL_EVIDENCE")"
LAUNCHER_LAYERED_COMMITS="$(sed -n 's/^launcher_layered_commits=//p' "$PROTOCOL_EVIDENCE")"
[[ "$INITIAL_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$UPDATED_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$INITIAL_PPM_SHA256" != "$UPDATED_PPM_SHA256" \
  && "$CONTENT_CHANGED_PIXELS" =~ ^[1-9][0-9]*$ \
  && "$LABEL_CHANGED_PIXELS" =~ ^[1-9][0-9]*$ \
  && "$APP_ASID" =~ ^[1-9][0-9]*$ \
  && "$ANDROID_APP_ASID" =~ ^[1-9][0-9]*$ \
  && "$APP_ASID" != "$ANDROID_APP_ASID" \
  && "$APP_LAYERED_COMMITS" =~ ^[1-9][0-9]*$ \
  && "$LAUNCHER_LAYERED_COMMITS" =~ ^[1-9][0-9]*$ ]] || {
  fail_gate "Could not retain canonical Process-0 evidence."
}

printf '%s\n' \
  'profile=androidbox-process0' \
  'abi=47' \
  'scope=bounded-interactiveactivity1-worker' \
  'physical_width=720' \
  'physical_height=1600' \
  'content_viewport=0/64/720/1448' \
  'top_chrome_rows=0-64' \
  'bottom_chrome_rows=1512-1600' \
  'graphics_identities=3' \
  'graphics_handles=6' \
  'graphics_producer_roles=surface-server+launcher+app' \
  'chrome_owner=surface-server' \
  'qemu_boots=2' \
  'qemu_nic_none_per_boot=1' \
  'install_source=qemu-fw_cfg' \
  'recovery_source=none' \
  "apk_bytes=$APK_BYTES" \
  "apk_sha256=$APK_SHA256" \
  "signer_cert_sha256=$EXPECTED_SIGNER_SHA256" \
  "package=$PACKAGE" \
  "activity=$ACTIVITY" \
  "version_code=$VERSION_CODE" \
  "app_elf_bytes=$APP_ELF_BYTES" \
  "android_app_elf_bytes=$ANDROID_APP_ELF_BYTES" \
  "app_elf_sha256=$APP_ELF_SHA256" \
  "android_app_elf_sha256=$ANDROID_APP_ELF_SHA256" \
  'elf_distinct=1' \
  "surface_server_pid=$SURFACE_PID" \
  "launcher_pid=$LAUNCHER_PID" \
  "app_pid=$APP_PID" \
  "android_app_pid=$ANDROID_APP_PID" \
  "app_asid=$APP_ASID" \
  "android_app_asid=$ANDROID_APP_ASID" \
  'pid_distinct=1' \
  'asid_distinct=1' \
  'android_app_live=1' \
  'android_app_handles=1' \
  'android_app_channels=1' \
  'private_channel_endpoints=2' \
  'private_channel_pairs=1' \
  'private_channel_queues_empty=1' \
  'rpc_protocol=BNDAPC01' \
  'rpc_request_order=0/1/2/3' \
  'rpc_ready=1' \
  'rpc_open=1' \
  'rpc_opened=1' \
  'rpc_label_chunks=2' \
  'rpc_label_chunk_bytes=24/2' \
  'rpc_button_chunks=1' \
  'rpc_button_chunk_bytes=11' \
  'rpc_click=1' \
  'rpc_updated=1' \
  'rpc_update_chunks=1' \
  'rpc_update_chunk_bytes=24' \
  'rpc_close=1' \
  'rpc_closed=1' \
  'rpc_errors=0' \
  'android_app_graphics_handles=0' \
  'android_app_input_samples=0' \
  "app_layered_commits=$APP_LAYERED_COMMITS" \
  "launcher_layered_commits=$LAUNCHER_LAYERED_COMMITS" \
  'android_app_layered_commits=0' \
  'top_chrome_byte_identical=1' \
  'bottom_chrome_byte_identical=1' \
  "content_changed_pixels=$CONTENT_CHANGED_PIXELS" \
  "text_label_changed_pixels=$LABEL_CHANGED_PIXELS" \
  "initial_ppm_sha256=$INITIAL_PPM_SHA256" \
  "updated_ppm_sha256=$UPDATED_PPM_SHA256" \
  "initial_disk_sha256=$INITIAL_DISK_SHA256" \
  "installed_disk_sha256=$INSTALLED_DISK_SHA256" \
  "recovery_before_disk_sha256=$RECOVERY_BEFORE_DISK_SHA256" \
  "recovery_after_disk_sha256=$RECOVERY_AFTER_DISK_SHA256" \
  'recovery_disk_unchanged=1' \
  'network=disabled' \
  'panic_fault_fatal_free=1' \
  'art=0' \
  'dalvik=0' \
  'binder=0' \
  'general_apk_claim=0' \
  'general_android_compatibility_claim=0' \
  'real_phone_claim=0' \
  >"$SUMMARY"

echo "ANDROIDBOX_PROCESS0_QEMU_OK artifact_dir=$ARTIFACT_DIR abi=47 scope=bounded-interactiveactivity1-worker physical_screen=720x1600 content_viewport=0/64/720/1448 graphics_identities=3 graphics_handles=6 graphics_producer_roles=surface-server+launcher+app chrome_owner=surface-server qemu_boots=2 qemu_nic_none_per_boot=1 install_source=qemu-fw_cfg recovery_source=none apk_bytes=$APK_BYTES apk_sha256=$APK_SHA256 signer_cert_sha256=$EXPECTED_SIGNER_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE app_elf_bytes=$APP_ELF_BYTES android_app_elf_bytes=$ANDROID_APP_ELF_BYTES app_elf_sha256=$APP_ELF_SHA256 android_app_elf_sha256=$ANDROID_APP_ELF_SHA256 elf_distinct=1 surface_server_pid=$SURFACE_PID launcher_pid=$LAUNCHER_PID app_pid=$APP_PID android_app_pid=$ANDROID_APP_PID app_asid=$APP_ASID android_app_asid=$ANDROID_APP_ASID pid_distinct=1 asid_distinct=1 android_app_live=1 android_app_handles=1 android_app_channels=1 private_channel_endpoints=2 private_channel_pairs=1 private_channel_queues_empty=1 rpc_protocol=BNDAPC01 rpc_request_order=0/1/2/3 rpc_ready=1 rpc_open=1 rpc_opened=1 rpc_label_chunks=2 rpc_label_chunk_bytes=24/2 rpc_button_chunks=1 rpc_button_chunk_bytes=11 rpc_click=1 rpc_updated=1 rpc_update_chunks=1 rpc_update_chunk_bytes=24 rpc_close=1 rpc_closed=1 rpc_errors=0 android_app_graphics_handles=0 android_app_input_samples=0 app_layered_commits=$APP_LAYERED_COMMITS launcher_layered_commits=$LAUNCHER_LAYERED_COMMITS android_app_layered_commits=0 top_chrome_byte_identical=1 bottom_chrome_byte_identical=1 content_changed_pixels=$CONTENT_CHANGED_PIXELS text_label_changed_pixels=$LABEL_CHANGED_PIXELS initial_ppm_sha256=$INITIAL_PPM_SHA256 updated_ppm_sha256=$UPDATED_PPM_SHA256 recovery_disk_unchanged=1 recovery_disk_sha256=$RECOVERY_AFTER_DISK_SHA256 network=disabled panic_fault_fatal_free=1 art=0 dalvik=0 binder=0 general_apk_claim=0 general_android_compatibility_claim=0 real_phone_claim=0"
