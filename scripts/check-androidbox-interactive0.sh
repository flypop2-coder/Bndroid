#!/usr/bin/env bash
set -euo pipefail

# Strict offline AndroidBox Interactive-0 end-to-end gate.
#
# This gate builds the checked-in real Java Activity twice, proves that both
# APK v2 outputs are byte-identical, builds the ABI-46 kernel/userspace image,
# and starts from one deterministic empty 16 MiB package store. The first boot
# receives the APK only through qemu fw_cfg and installs generation 1. The
# second boot has no APK source and must recover, claim, execute, render, and
# interact with the durable image without changing one disk byte.
#
# The accepted Activity remains the deliberately bounded InteractiveActivity-1
# subset. Passing this gate is not evidence of ART/Dalvik, Binder/Bionic/JNI,
# Android Framework completeness, native libraries, or general APK support.
#
# Process ownership is intentionally narrow: cleanup sends TERM only to the
# exact QEMU PID started by this script. It never searches for, attaches to, or
# terminates any pre-existing emulator.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-interactive0.sh' \
    '' \
    'Builds the fixed androidbox-interactive-demo fixture and runs two local,' \
    'strictly offline ABI-46 QEMU boots: sourced install, then source-free' \
    'recovery plus Button/system-chrome/Overview interaction.'
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
  awk cmp cp shasum wc sed chmod; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the Interactive-0 gate." >&2
    exit 1
  }
done

FIXTURE_DIR="$WORKSPACE_ROOT/fixtures/androidbox-interactive-demo"
FIXTURE_BUILD="$FIXTURE_DIR/build.sh"
FIXTURE_APK="$FIXTURE_DIR/androidbox-interactive-demo.apk"
BUILD_KERNEL="$SCRIPT_DIR/build-kernel.sh"
BUILD_STORAGE="$SCRIPT_DIR/build-storage-image.sh"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
for helper in \
  "$FIXTURE_BUILD" "$BUILD_KERNEL" "$BUILD_STORAGE" "$QMP_HELPER"; do
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

mkdir -p "$WORKSPACE_ROOT/target/androidbox-interactive0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-interactive0/check.XXXXXX"
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
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-interactive0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"

ACTIVITY_BEFORE="$ARTIFACT_DIR/activity.before.ppm"
ACTIVITY_AFTER="$ARTIFACT_DIR/activity.after.ppm"
ACTIVITY_RELAUNCH="$ARTIFACT_DIR/activity.relaunch.ppm"
TOP_BEFORE="$ARTIFACT_DIR/activity.top-before.ppm"
TOP_AFTER="$ARTIFACT_DIR/activity.top-after.ppm"
BOTTOM_CORNER_BEFORE="$ARTIFACT_DIR/activity.bottom-corner-before.ppm"
BOTTOM_CORNER_AFTER="$ARTIFACT_DIR/activity.bottom-corner-after.ppm"
OVERVIEW="$ARTIFACT_DIR/overview.ppm"
RASTER_EVIDENCE="$ARTIFACT_DIR/raster-evidence.txt"
PROTOCOL_EVIDENCE="$ARTIFACT_DIR/protocol-evidence.txt"
SUMMARY="$ARTIFACT_DIR/summary.txt"

QEMU_PID=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""
BOOT_QMP_SOCKET=""

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
  tr -d '\r' <"$BOOT_SERIAL_LOG" >"$BOOT_NORMALIZED_LOG"
}

show_failure() {
  if [[ -n "$BOOT_NORMALIZED_LOG" && -f "$BOOT_NORMALIZED_LOG" ]]; then
    tail -n 320 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -n "$BOOT_SERIAL_LOG" && -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 320 >&2
  fi
  if [[ -n "$BOOT_QEMU_LOG" && -f "$BOOT_QEMU_LOG" ]]; then
    tail -n 140 "$BOOT_QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Interactive-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

reject_panic_fault_or_fatal() {
  normalize_log
  if grep -Eqi \
    'fatal exception:|kernel panic:|panicked at|panic!|boot error:|USER_FAIL:|EL0_FAIL|THREAD_EXITED:|DATA_ABORT|INSTRUCTION_ABORT|PAGE_FAULT|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)' \
    "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU emitted a panic, fault, fatal, or userspace failure marker."
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
  local app_pid="$1"
  log_count \
    "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${app_pid} "
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
    "overview": {
        (360, 300): (13, 22, 41),
        (360, 580): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    },
}.get(kind)
fallback_icon_colors = {
    (16, 118, 111), (11, 87, 208), (142, 36, 170), (0, 108, 76),
    (179, 38, 30), (122, 79, 1), (64, 81, 181),
}
def matches(point, color):
    actual = pixel(*point)
    if kind == "drawer" and point == (104, 630):
        return actual in fallback_icon_colors
    return actual == color
if expected is None or any(not matches(point, color) for point, color in expected.items()):
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
  # Fix QEMU's tablet cursor at one non-interactive content coordinate. Every
  # cross-frame raster comparison below therefore measures guest pixels with
  # identical host cursor placement.
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

wait_for_identical_screenshot() {
  local baseline="$1"
  local screenshot="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  # The caller deliberately leaves the tablet cursor at the same coordinate
  # used for `baseline`. Do not inject a new absolute sample here: this helper
  # is also used to prove that a trusted bottom-corner stream never escapes to
  # App.
  while ((SECONDS < deadline)); do
    reject_panic_fault_or_fatal
    qmp screenshot "$screenshot"
    if cmp -s "$baseline" "$screenshot"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

# One launch site is deliberately reused for both boots. The command contains
# exactly one literal network-disable argument, so every invocation is
# independently offline. `source_args` adds fw_cfg only to the install boot.
start_qemu() {
  local name="$1"
  local source_apk="$2"
  local -a source_args=(-name "Bndroid Interactive-0 Gate $name")
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
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill -TERM "$QEMU_PID"
    wait "$QEMU_PID" 2>/dev/null || true
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

# The fixture builder itself uses only the installed SDK/JDK and compares two
# independent signatures. Running the complete builder twice adds a second
# whole-build determinism check before either output is admitted to QEMU.
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

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0,androidbox-el0-runtime0,androidbox-interactive0 \
  "$BUILD_KERNEL" >"$BUILD_LOG" 2>&1; then
  tail -n 280 "$BUILD_LOG" >&2
  echo "ABI-46 Interactive-0 kernel/userspace build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || {
  echo "ABI-46 build did not produce the expected kernel image." >&2
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

# Boot 1: the only sourced boot installs generation 1 through fw_cfg.
start_qemu install "$SOURCE_APK"
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=1 source_admitted=1 .* operation=install .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled " \
  "the sourced generation-1 install"
wait_for_pattern \
  '^ANDROIDBOX_INTERACTIVE0_PROFILE_OK format=1 abi=46 .* surface_present_layers_syscall=62 .* content_viewport=0/64/720/1448 .* user_stack_pages=16 user_stack_bytes=65536 stack_guard_pages=2 .* network=disabled .* real_phone_claim=0$' \
  "the ABI-46 Interactive-0 profile"
stop_qemu
INSTALL_LOG="$BOOT_NORMALIZED_LOG"

require_once_ere \
  "$INSTALL_LOG" \
  "^APK_SOURCE_OK .* present=1 bytes=${APK_BYTES} .* sha256=${APK_SHA256} .* host_directory_scan=0 network=disabled install_mutation=0$" \
  "sourced fw_cfg APK marker"
require_once_ere \
  "$INSTALL_LOG" \
  "^BLOCK_LAYER_OK .* package_reads=1032 package_writes=130 package_flushes=3 .* timeouts=0 .*" \
  "install block-layer marker"
require_once_ere \
  "$INSTALL_LOG" \
  "^APK_PACKAGE_STORE_OK .* operation=install previous_generation=0 previous_version_code=0 installed=1 removed=0 generation=1 slot=0 apk_bytes=${APK_BYTES} version_code=${VERSION_CODE} package=${PACKAGE} activity=${ACTIVITY} profile=Resources-1 .* mutation_performed=1 reads=1032 writes=130 flushes=3 .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled .*" \
  "durable generation-1 install marker"
require_once_ere \
  "$INSTALL_LOG" \
  "^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=${APP_TITLE// /[[:space:]]} text=${INITIAL_LABEL// /[[:space:]]} constructor=1 .* constructor_instructions=2 on_create=1 .* on_create_instructions=8 .* framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$" \
  "installed interactive Activity preflight"
require_once_ere \
  "$INSTALL_LOG" \
  '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=46 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 buffers=3 .* buffer_present_version=3 .* network=disabled .* real_phone_claim=0$' \
  "install ABI-46 720x1600 preview"

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
changed = [index for index, values in enumerate(zip(before, after)) if values[0] != values[1]]
if not changed:
    raise SystemExit("install changed no disk bytes")
outside = [index for index in changed if not first <= index < last]
if outside:
    raise SystemExit(
        f"install changed byte {outside[0]} outside BNDROID_PACKAGES"
    )
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

# Boot 2: no fw_cfg source exists. Recover the exact durable APK, launch it,
# dispatch its verified View.OnClickListener path, exercise both trusted chrome
# regions, and reopen through identity-only Overview.
RECOVERY_BEFORE_DISK_SHA256="$INSTALLED_DISK_SHA256"
start_qemu recovery ""
wait_for_pattern \
  "^APK_PACKAGE_STORE_OK .* source_present=0 source_admitted=0 .* operation=recovery .* installed=1 .* generation=1 .* apk_bytes=${APK_BYTES} .* package=${PACKAGE} activity=${ACTIVITY} .* writes=0 flushes=0 source_free_zero_writes=1 .* apk_sha256=${APK_SHA256} signer_cert_sha256=${EXPECTED_SIGNER_SHA256} network=disabled " \
  "source-free durable package recovery"
wait_for_pattern \
  '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=46 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 buffers=3 .* buffer_present_version=3 .* network=disabled .* real_phone_claim=0$' \
  "the recovery ABI-46 720x1600 preview"
wait_for_pattern \
  '^ANDROIDBOX_INTERACTIVE0_PROFILE_OK format=1 abi=46 parent_profile=androidbox-el0-runtime0 surface_present_layers_syscall=62 sources=2 content_owner=launcher-or-app chrome_owner=surface-server content_viewport=0/64/720/1448 chrome_regions=0-64/1512-1600 .* buffer_present_version=3 buffer_present_wire_bytes=64 .* system_chrome_input_capture=surface-server app_content_input_bounds=0/64/720/1448 physical_screen=720x1600 user_stack_pages=16 user_stack_bytes=65536 stack_guard_pages=2 art=0 binder=0 general_apk_claim=0 network=disabled emulator_only=1 real_phone_claim=0$' \
  "the complete recovery Interactive-0 ownership profile"
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
CHROME_PID="$(
  awk '/^USER_SURFACE_LAYERED_COMMIT_OK / {
    for (field = 1; field <= NF; field++) {
      if ($field ~ /^chrome_producer_pid=/) {
        sub(/^chrome_producer_pid=/, "", $field)
        print $field
        exit
      }
    }
  }' "$BOOT_NORMALIZED_LOG"
)"
[[ "$SURFACE_PID" =~ ^[1-9][0-9]*$ \
  && "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ \
  && "$CHROME_PID" == "$SURFACE_PID" \
  && "$LAUNCHER_PID" != "$SURFACE_PID" ]] || {
  fail_gate "Could not derive distinct Launcher and system-chrome owners."
}

# Unlock and settle the installed row in All apps.
qmp drag 360 1390 360 620
wait_for_pattern \
  "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=${SURFACE_PID} action=unlock app=none request_id=1 " \
  "the authenticated unlock"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' \
  "the unlocked Home state"
qmp tap 700 900
qmp drag 360 1280 360 520
wait_for_screenshot drawer "$ARTIFACT_DIR/drawer.ppm" "the installed All apps row"

# First installed launch: syscall 60 binds the session to App, syscall 61
# grants one read-only image, and five enter-transition commits settle to the
# initial real-APK TextView/Button raster.
qmp tap 106 674
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} app_owner=[1-9][0-9]* compatible_session_id=1 request_sequence=1 generation=1 bytes=640 reads=389 writes=0 flushes=0 authority_granted=app-read-only-vmo apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=1 storage_authority_granted=0$" \
  "the first App-bound durable relaunch collection"
normalize_log
APP_PID="$(
  sed -n \
    "s/^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} app_owner=\\([1-9][0-9]*\\) compatible_session_id=1 request_sequence=1 .*/\\1/p" \
    "$BOOT_NORMALIZED_LOG"
)"
[[ "$APP_PID" =~ ^[1-9][0-9]*$ \
  && "$APP_PID" != "$LAUNCHER_PID" \
  && "$APP_PID" != "$SURFACE_PID" ]] || {
  fail_gate "Could not derive one App producer distinct from trusted owners."
}
wait_for_pattern \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${APP_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} .* rights=READ duplicate=0 transfer=0 map=0 write=0 execute=0 wait=0 storage_authority_granted=0$" \
  "the first one-shot App image claim"
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=app .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "the first compatible Activity foreground"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  5 \
  "five App enter-transition layered commits"
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the initial interactive Activity raster"

# A top-chrome tap is generated as an unpressed absolute sample, press, and
# release. All three samples must remain owned by SurfaceServer: no App route,
# frame, claim, or pixel is allowed to change.
qmp move 360 32
qmp screenshot "$TOP_BEFORE"
ppm_matches activity "$TOP_BEFORE" || {
  fail_gate "Top-chrome baseline is not the settled Activity."
}
TOP_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
TOP_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
TOP_CLAIMS_BEFORE="$(claim_count "$APP_PID")"
qmp tap 360 32
qmp screenshot "$TOP_AFTER"
normalize_log
[[ "$(app_route_count "$APP_PID")" == "$TOP_ROUTES_BEFORE" \
  && "$(layered_commit_count "$APP_PID")" == "$TOP_COMMITS_BEFORE" \
  && "$(claim_count "$APP_PID")" == "$TOP_CLAIMS_BEFORE" ]] || {
  fail_gate "Trusted top chrome routed to or mutated the App session."
}
cmp "$TOP_BEFORE" "$TOP_AFTER" || {
  fail_gate "Trusted top-chrome tap changed the displayed Activity raster."
}

# The physical bottom-left rounded-corner pixel is outside the visible phone
# shape even though it lies inside the 720x1600 scanout. A complete tablet
# stream at (0,1599) may transiently publish system-navigation pressed/released
# state, but all three samples remain SurfaceServer-owned, no image claim is
# consumed, and the cancelled gesture must settle to the exact initial raster.
qmp move 0 1599
qmp screenshot "$BOTTOM_CORNER_BEFORE"
ppm_matches activity "$BOTTOM_CORNER_BEFORE" || {
  fail_gate "Bottom-corner baseline is not the settled Activity."
}
BOTTOM_CORNER_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
BOTTOM_CORNER_CLAIMS_BEFORE="$(claim_count "$APP_PID")"
BOTTOM_CORNER_PRESSED_PATTERN='^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=app .* mode=foreground recent_app=android-compatible nav_pressed=1 nav_reveal_px=0 .* compatible_session=1 package_generation=1 '
BOTTOM_CORNER_STABLE_PATTERN='^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=app .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 '
BOTTOM_CORNER_PRESSED_BEFORE="$(log_count "$BOTTOM_CORNER_PRESSED_PATTERN")"
BOTTOM_CORNER_STABLE_BEFORE="$(log_count "$BOTTOM_CORNER_STABLE_PATTERN")"
qmp tap 0 1599
wait_for_count \
  "$BOTTOM_CORNER_PRESSED_PATTERN" \
  "$((BOTTOM_CORNER_PRESSED_BEFORE + 1))" \
  "the system-owned bottom-corner pressed state"
wait_for_count \
  "$BOTTOM_CORNER_STABLE_PATTERN" \
  "$((BOTTOM_CORNER_STABLE_BEFORE + 1))" \
  "the cancelled bottom-corner stable foreground"
wait_for_identical_screenshot \
  "$BOTTOM_CORNER_BEFORE" \
  "$BOTTOM_CORNER_AFTER" \
  "the byte-identical Activity after bottom-corner cancellation"
[[ "$(app_route_count "$APP_PID")" == "$BOTTOM_CORNER_ROUTES_BEFORE" \
  && "$(claim_count "$APP_PID")" == "$BOTTOM_CORNER_CLAIMS_BEFORE" ]] || {
  fail_gate "Trusted bottom corner routed to or consumed an App claim."
}

# Restore the fixed comparison cursor, then dispatch the one admitted Button.
# QEMU supplies exactly three routed samples at the Button coordinate and the
# callback produces exactly two new App-owned layered frames: pressed feedback
# followed by the revised TextView.
wait_for_screenshot activity "$ACTIVITY_BEFORE" "the restored pre-click raster"
BUTTON_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
BUTTON_COORD_ROUTES_BEFORE="$(
  log_count "^UI_ROUTE_INPUT_OK .* receiver_image=app receiver_pid=${APP_PID} .* x=360 y=728 "
)"
BUTTON_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
BUTTON_CLAIMS_BEFORE="$(claim_count "$APP_PID")"
qmp tap 360 728
wait_for_count \
  "^UI_ROUTE_INPUT_OK .* receiver_image=app receiver_pid=${APP_PID} .* x=360 y=728 " \
  "$((BUTTON_COORD_ROUTES_BEFORE + 3))" \
  "all three Button pointer samples"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((BUTTON_COMMITS_BEFORE + 2))" \
  "the Button pressed and callback frames"
[[ "$(app_route_count "$APP_PID")" == "$((BUTTON_ROUTES_BEFORE + 3))" \
  && "$(layered_commit_count "$APP_PID")" == "$((BUTTON_COMMITS_BEFORE + 2))" \
  && "$(claim_count "$APP_PID")" == "$BUTTON_CLAIMS_BEFORE" ]] || {
  fail_gate "Button dispatch did not have the exact 3-route/2-frame transaction."
}
wait_for_screenshot activity "$ACTIVITY_AFTER" "the clicked TextView raster"

# System Home is another three-sample trusted-chrome transaction. It must not
# route even its initial unpressed tablet sample to App. Launcher then owns the
# Home frame while only the compatible identity is retained.
HOME_APP_ROUTES_BEFORE="$(app_route_count "$APP_PID")"
HOME_APP_COMMITS_BEFORE="$(layered_commit_count "$APP_PID")"
HOME_CLAIMS_BEFORE="$(claim_count "$APP_PID")"
LAUNCHER_COMMITS_BEFORE="$(layered_commit_count "$LAUNCHER_PID")"
qmp tap 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 ' \
  "Home retaining only the compatible identity"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
  "$((LAUNCHER_COMMITS_BEFORE + 1))" \
  "the Launcher Home layered frame"
[[ "$(app_route_count "$APP_PID")" == "$HOME_APP_ROUTES_BEFORE" \
  && "$(layered_commit_count "$APP_PID")" == "$HOME_APP_COMMITS_BEFORE" \
  && "$(claim_count "$APP_PID")" == "$HOME_CLAIMS_BEFORE" ]] || {
  fail_gate "Bottom Home samples routed to or mutated the App session."
}

# Open identity-only Overview entirely through the trusted navigation capture.
LAUNCHER_COMMITS_BEFORE="$(layered_commit_count "$LAUNCHER_PID")"
qmp touch-down 360 1570
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "the trusted Overview gesture capture"
qmp touch-move 360 1330
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=240 .* compatible_session=1 package_generation=1 ' \
  "the Overview finger-follow state"
qmp touch-up
wait_for_pattern \
  '^UI_SYSTEM_UI_CHANGED_OK .* receiver_image=launcher .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* compatible_session=1 package_generation=1 ' \
  "stable identity-only compatible Overview"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${LAUNCHER_PID} " \
  "$((LAUNCHER_COMMITS_BEFORE + 1))" \
  "the Launcher Overview layered frame"
wait_for_screenshot overview "$OVERVIEW" "the compatible Overview identity card"

# Reopening Overview must repeat both durable readback and the one-shot claim.
# The new retained ActivitySession starts at revision 1, so its final five-frame
# enter transition must recover the exact byte-identical initial raster.
APP_COMMITS_BEFORE_RELAUNCH="$(layered_commit_count "$APP_PID")"
qmp tap 360 920
wait_for_pattern \
  "^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=${LAUNCHER_PID} app_owner=${APP_PID} compatible_session_id=1 request_sequence=2 generation=1 bytes=640 reads=389 writes=0 flushes=0 authority_granted=app-read-only-vmo apk_bytes_exposed_to_launcher=0 apk_bytes_exposed_to_app=1 storage_authority_granted=0$" \
  "the second durable relaunch collection"
wait_for_count \
  "^ANDROID_PACKAGE_IMAGE_CLAIM_OK owner=${APP_PID} compatible_session_id=1 generation=1 apk_length=${APK_BYTES} .* rights=READ duplicate=0 transfer=0 map=0 write=0 execute=0 wait=0 storage_authority_granted=0$" \
  2 \
  "the second one-shot App image claim"
wait_for_count \
  "^USER_SURFACE_LAYERED_COMMIT_OK .* content_producer_pid=${APP_PID} " \
  "$((APP_COMMITS_BEFORE_RELAUNCH + 5))" \
  "five App relaunch-transition layered commits"
wait_for_screenshot activity "$ACTIVITY_RELAUNCH" "the reset initial Activity raster"

stop_qemu
RECOVERY_LOG="$BOOT_NORMALIZED_LOG"
RECOVERY_AFTER_DISK_SHA256="$(shasum -a 256 "$DISK_IMAGE" | awk '{print $1}')"
[[ "$RECOVERY_AFTER_DISK_SHA256" == "$RECOVERY_BEFORE_DISK_SHA256" ]] || {
  fail_gate "Source-free recovery, click, or navigation changed the package disk."
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
  "recovered interactive Activity preflight"

# Prove exact pixel ownership and the callback/relaunch semantics. The click
# must change only the publisher TextView; the internal revision remains
# observable in state/logs but produces no Activity pixels. Trusted chrome is
# byte-identical and Overview relaunch restores the exact initial scanout.
python3 - \
  "$ACTIVITY_BEFORE" \
  "$ACTIVITY_AFTER" \
  "$ACTIVITY_RELAUNCH" \
  "$RASTER_EVIDENCE" <<'PY'
from hashlib import sha256
from pathlib import Path
import sys

before_path, after_path, relaunch_path, output_path = map(Path, sys.argv[1:5])

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
relaunch_payload, relaunch = read_ppm(relaunch_path)

def rows(pixels: bytes, first: int, last: int) -> bytes:
    stride = 720 * 3
    return pixels[first * stride:last * stride]

top_equal = rows(before, 0, 64) == rows(after, 0, 64)
bottom_equal = rows(before, 1512, 1600) == rows(after, 1512, 1600)
if not top_equal or not bottom_equal:
    raise SystemExit("Button callback changed trusted top or bottom chrome")

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
if content_changed < 3_000:
    raise SystemExit("Button callback did not materially change client content")
if label_changed < 3_000:
    raise SystemExit("the installed TextView raster did not change")
if content_changed != label_changed:
    raise SystemExit("Button callback changed pixels outside the publisher TextView")
if before_payload == after_payload:
    raise SystemExit("pre-click and post-click screenshots are identical")
if before_payload != relaunch_payload:
    raise SystemExit("Overview relaunch did not restore the exact initial raster")

values = {
    "width": "720",
    "height": "1600",
    "content_viewport": "0/64/720/1448",
    "top_chrome_rows": "0-64",
    "bottom_chrome_rows": "1512-1600",
    "top_chrome_byte_identical": "1",
    "bottom_chrome_byte_identical": "1",
    "content_changed_pixels": str(content_changed),
    "text_label_changed_pixels": str(label_changed),
    "revision_changed_pixels": "0",
    "revision_pixels_hidden": "1",
    "initial_ppm_sha256": sha256(before_payload).hexdigest(),
    "clicked_ppm_sha256": sha256(after_payload).hexdigest(),
    "relaunch_ppm_sha256": sha256(relaunch_payload).hexdigest(),
    "relaunch_initial_raster_identical": "1",
}
output_path.write_text(
    "\n".join(f"{name}={value}" for name, value in values.items()) + "\n",
    encoding="utf-8",
)
PY

# Canonically parse the recovery log once more. This checks ownership on every
# layered commit, exact claim ordering, Button sample order, and the absence of
# any App route from either trusted chrome region.
python3 - \
  "$RECOVERY_LOG" \
  "$PROTOCOL_EVIDENCE" \
  "$SURFACE_PID" \
  "$LAUNCHER_PID" \
  "$APP_PID" \
  "$APK_BYTES" <<'PY'
from pathlib import Path
import re
import sys

log_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
surface_pid, launcher_pid, app_pid, apk_bytes = sys.argv[3:7]
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
profile = exact("ANDROIDBOX_INTERACTIVE0_PROFILE_OK ")
if len(preview) != 1 or len(profile) != 1:
    raise SystemExit("expected one ABI-46 preview and one Interactive-0 profile")
for name, expected in {
    "abi": "46",
    "width": "720",
    "height": "1600",
    "buffers": "3",
    "buffer_present_version": "3",
    "network": "disabled",
}.items():
    if preview[0][1].get(name) != expected:
        raise SystemExit(f"preview field mismatch: {name}")
for name, expected in {
    "abi": "46",
    "surface_present_layers_syscall": "62",
    "sources": "2",
    "content_owner": "launcher-or-app",
    "chrome_owner": "surface-server",
    "content_viewport": "0/64/720/1448",
    "chrome_regions": "0-64/1512-1600",
    "buffer_present_version": "3",
    "buffer_present_wire_bytes": "64",
    "graphics_handle_rights": "verified",
    "graphics_identities": "3",
    "graphics_handles": "6",
    "graphics_identity_handles": "2/2/2",
    "graphics_producer_handles": "1/1/1",
    "graphics_server_handles": "3",
    "graphics_producer_roles": "surface-server+launcher+app",
    "graphics_role_counts": "1/1/1",
    "system_chrome_input_capture": "surface-server",
    "app_content_input_bounds": "0/64/720/1448",
    "user_stack_pages": "16",
    "user_stack_bytes": "65536",
    "stack_guard_pages": "2",
    "network": "disabled",
}.items():
    if profile[0][1].get(name) != expected:
        raise SystemExit(f"Interactive-0 profile field mismatch: {name}")

commits = exact("USER_SURFACE_LAYERED_COMMIT_OK ")
if not commits:
    raise SystemExit("recovery emitted no layered commit")
app_commits: list[tuple[int, dict[str, str]]] = []
launcher_commits: list[tuple[int, dict[str, str]]] = []
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
    if value.get("content_producer_pid") not in {launcher_pid, app_pid}:
        raise SystemExit("layered content came from an unauthorised producer")
    for generation in ("content_generation", "chrome_generation"):
        try:
            if int(value.get(generation, "0"), 10) <= 0:
                raise ValueError
        except ValueError as error:
            raise SystemExit(f"invalid {generation}") from error
    if value["content_producer_pid"] == app_pid:
        app_commits.append((index, value))
    else:
        launcher_commits.append((index, value))
if len(app_commits) < 12 or not launcher_commits:
    raise SystemExit("missing App or Launcher layered producer evidence")

collects = exact("ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ")
claims = exact("ANDROID_PACKAGE_IMAGE_CLAIM_OK ")
if len(collects) != 2 or len(claims) != 2:
    raise SystemExit("expected exactly two durable collections and image claims")
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
        "reads": "389",
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
        raise SystemExit(f"durable collection {sequence} mismatch")
    if any(claim.get(name) != value for name, value in expected_claim.items()):
        raise SystemExit(f"image claim {sequence} mismatch")
    if collect_index >= claim_index:
        raise SystemExit(f"image claim {sequence} preceded its collection")

routes = exact("UI_ROUTE_INPUT_OK ")
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
if not claims[0][0] < button_routes[0][0] < button_routes[-1][0] < claims[1][0]:
    raise SystemExit("Button dispatch is outside the first retained Activity lease")

states = exact("UI_SYSTEM_UI_CHANGED_OK ")
def stable_state(mode: str, recent: str, after: int) -> int:
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
    raise SystemExit(f"missing stable state {mode}/{recent}")

foreground_one = stable_state("foreground", "android-compatible", collects[0][0])
home = stable_state("home", "android-compatible", button_routes[-1][0])
overview = stable_state("overview", "android-compatible", home)
foreground_two = stable_state("foreground", "android-compatible", collects[1][0])
if not (
    collects[0][0] < foreground_one < claims[0][0]
    < button_routes[0][0] < button_routes[-1][0]
    < home < overview < collects[1][0] < foreground_two < claims[1][0]
):
    raise SystemExit("interactive Activity lifecycle evidence is out of order")
for boundary in (home, overview):
    if not any(index > boundary for index, _value in launcher_commits):
        raise SystemExit("Home/Overview has no later Launcher-owned frame")
for claim_index, _claim in claims:
    if sum(index > claim_index for index, _value in app_commits) < 5:
        raise SystemExit("an image claim lacks five App enter-transition commits")

if any(
    re.search(
        r"fatal exception:|kernel panic:|panicked at|panic!|boot error:|"
        r"USER_FAIL:|EL0_FAIL|THREAD_EXITED:|DATA_ABORT|INSTRUCTION_ABORT|"
        r"PAGE_FAULT|MOBILE_UI_PREVIEW_(?:TIMEOUT|FAULT|RUNTIME_FAIL)|"
        r"MOBILE_UI_(?:CHILD_DIAG|USER_FAULT)",
        line,
        re.IGNORECASE,
    )
    for line in lines
):
    raise SystemExit("failure marker survived final recovery validation")

output_path.write_text(
    "\n".join(
        (
            "abi=46",
            "surface_present_layers_syscall=62",
            "buffer_present_version=3",
            "surface_sources=2",
            "graphics_handle_rights=verified",
            "graphics_identities=3",
            "graphics_handles=6",
            "graphics_identity_handles=2/2/2",
            "graphics_producer_handles=1/1/1",
            "graphics_server_handles=3",
            "graphics_producer_roles=surface-server+launcher+app",
            "graphics_role_counts=1/1/1",
            f"surface_server_pid={surface_pid}",
            f"launcher_pid={launcher_pid}",
            f"app_pid={app_pid}",
            f"layered_commits={len(commits)}",
            f"app_layered_commits={len(app_commits)}",
            f"launcher_layered_commits={len(launcher_commits)}",
            "image_claim_count=2",
            "durable_relaunch_count=2",
            "button_samples=3",
            "button_sample_order=unpressed-press-release",
            "top_chrome_samples_to_app=0",
            "bottom_chrome_samples_to_app=0",
            "home_samples_to_app=0",
            "bottom_corner_samples_to_app=0",
            "chrome_owner=surface-server",
            "content_owner=launcher-or-app",
            "panic_fault_fatal_free=1",
            "network=disabled",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

INITIAL_PPM_SHA256="$(sed -n 's/^initial_ppm_sha256=//p' "$RASTER_EVIDENCE")"
CLICKED_PPM_SHA256="$(sed -n 's/^clicked_ppm_sha256=//p' "$RASTER_EVIDENCE")"
RELAUNCH_PPM_SHA256="$(sed -n 's/^relaunch_ppm_sha256=//p' "$RASTER_EVIDENCE")"
CONTENT_CHANGED_PIXELS="$(sed -n 's/^content_changed_pixels=//p' "$RASTER_EVIDENCE")"
LABEL_CHANGED_PIXELS="$(sed -n 's/^text_label_changed_pixels=//p' "$RASTER_EVIDENCE")"
APP_LAYERED_COMMITS="$(sed -n 's/^app_layered_commits=//p' "$PROTOCOL_EVIDENCE")"
LAUNCHER_LAYERED_COMMITS="$(sed -n 's/^launcher_layered_commits=//p' "$PROTOCOL_EVIDENCE")"
[[ "$INITIAL_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$CLICKED_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$RELAUNCH_PPM_SHA256" == "$INITIAL_PPM_SHA256" \
  && "$CONTENT_CHANGED_PIXELS" =~ ^[1-9][0-9]*$ \
  && "$LABEL_CHANGED_PIXELS" =~ ^[1-9][0-9]*$ \
  && "$APP_LAYERED_COMMITS" =~ ^[1-9][0-9]*$ \
  && "$LAUNCHER_LAYERED_COMMITS" =~ ^[1-9][0-9]*$ ]] || {
  fail_gate "Could not retain canonical Interactive-0 evidence."
}

printf '%s\n' \
  'profile=androidbox-interactive0' \
  'abi=46' \
  'physical_width=720' \
  'physical_height=1600' \
  'content_viewport=0/64/720/1448' \
  'top_chrome_rows=0-64' \
  'bottom_chrome_rows=1512-1600' \
  'surface_present_layers_syscall=62' \
  'buffer_present_version=3' \
  'buffer_present_wire_bytes=64' \
  'surface_sources=2' \
  'graphics_handle_rights=verified' \
  'graphics_identities=3' \
  'graphics_handles=6' \
  'graphics_identity_handles=2/2/2' \
  'graphics_producer_handles=1/1/1' \
  'graphics_server_handles=3' \
  'graphics_producer_roles=surface-server+launcher+app' \
  'graphics_role_counts=1/1/1' \
  'chrome_owner=surface-server' \
  'content_owner=launcher-or-app' \
  'user_stack_pages=16' \
  'user_stack_bytes=65536' \
  'stack_guard_pages=2' \
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
  "label_view_id=$LABEL_VIEW_ID" \
  "button_view_id=$BUTTON_VIEW_ID" \
  "button_coordinate=360/728" \
  "initial_label=$INITIAL_LABEL" \
  "clicked_label=$CLICKED_LABEL" \
  "button_label=$BUTTON_LABEL" \
  "initial_disk_sha256=$INITIAL_DISK_SHA256" \
  "installed_disk_sha256=$INSTALLED_DISK_SHA256" \
  "recovery_before_disk_sha256=$RECOVERY_BEFORE_DISK_SHA256" \
  "recovery_after_disk_sha256=$RECOVERY_AFTER_DISK_SHA256" \
  'recovery_disk_unchanged=1' \
  'durable_relaunch_count=2' \
  'image_claim_count=2' \
  'button_samples=3' \
  'button_sample_order=unpressed-press-release' \
  'button_callback_frames=2' \
  "app_layered_commits=$APP_LAYERED_COMMITS" \
  "launcher_layered_commits=$LAUNCHER_LAYERED_COMMITS" \
  'top_chrome_injected_samples=3' \
  'top_chrome_samples_to_app=0' \
  'bottom_home_injected_samples=3' \
  'bottom_home_samples_to_app=0' \
  'bottom_corner_injected_samples=3' \
  'bottom_corner_samples_to_app=0' \
  'bottom_corner_claim_unchanged=1' \
  'bottom_corner_raster_identical=1' \
  'top_chrome_byte_identical=1' \
  'bottom_chrome_byte_identical=1' \
  "content_changed_pixels=$CONTENT_CHANGED_PIXELS" \
  "text_label_changed_pixels=$LABEL_CHANGED_PIXELS" \
  "initial_ppm_sha256=$INITIAL_PPM_SHA256" \
  "clicked_ppm_sha256=$CLICKED_PPM_SHA256" \
  "relaunch_ppm_sha256=$RELAUNCH_PPM_SHA256" \
  'relaunch_initial_raster_identical=1' \
  'network=disabled' \
  'panic_fault_fatal_free=1' \
  'art=0' \
  'binder=0' \
  'general_apk_claim=0' \
  >"$SUMMARY"

echo "ANDROIDBOX_INTERACTIVE0_QEMU_OK artifact_dir=$ARTIFACT_DIR abi=46 physical_screen=720x1600 content_viewport=0/64/720/1448 surface_present_layers_syscall=62 buffer_present_version=3 surface_sources=2 graphics_handle_rights=verified graphics_identities=3 graphics_handles=6 graphics_identity_handles=2/2/2 graphics_producer_handles=1/1/1 graphics_server_handles=3 graphics_producer_roles=surface-server+launcher+app graphics_role_counts=1/1/1 chrome_owner=surface-server content_owner=launcher-or-app user_stack_pages=16 user_stack_bytes=65536 stack_guard_pages=2 qemu_boots=2 qemu_nic_none_per_boot=1 install_source=qemu-fw_cfg recovery_source=none apk_bytes=$APK_BYTES apk_sha256=$APK_SHA256 signer_cert_sha256=$EXPECTED_SIGNER_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE generation=1 surface_server_pid=$SURFACE_PID launcher_pid=$LAUNCHER_PID app_pid=$APP_PID durable_relaunch_count=2 image_claim_count=2 button_coordinate=360/728 button_samples=3 button_callback_frames=2 app_layered_commits=$APP_LAYERED_COMMITS launcher_layered_commits=$LAUNCHER_LAYERED_COMMITS top_chrome_samples_to_app=0 bottom_home_samples_to_app=0 bottom_corner_samples_to_app=0 bottom_corner_claim_unchanged=1 bottom_corner_raster_identical=1 top_chrome_byte_identical=1 bottom_chrome_byte_identical=1 content_changed_pixels=$CONTENT_CHANGED_PIXELS text_label_changed_pixels=$LABEL_CHANGED_PIXELS initial_ppm_sha256=$INITIAL_PPM_SHA256 clicked_ppm_sha256=$CLICKED_PPM_SHA256 relaunch_ppm_sha256=$RELAUNCH_PPM_SHA256 relaunch_initial_raster_identical=1 recovery_disk_unchanged=1 recovery_disk_sha256=$RECOVERY_AFTER_DISK_SHA256 network=disabled panic_fault_fatal_free=1 art=0 binder=0 general_apk_claim=0"
