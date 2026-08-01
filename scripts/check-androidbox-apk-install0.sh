#!/usr/bin/env bash
set -euo pipefail

# Offline local-QEMU acceptance gate for the deliberately bounded AndroidBox
# APK Install-0 profile. The supplied APK is admitted as an exact, explicit
# fw_cfg file; no directory is scanned and QEMU has no network interface.
#
# This proves only the repository's v2-signed Resources-1 fixture, one trusted
# kernel package transaction, durable package-store recovery, and execution
# from the durable readback. It does not claim arbitrary APK, ART/Dalvik,
# Binder/Bionic/JNI, Android Framework, native library, hardware, or general
# Android application compatibility.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  cat <<'EOF'
Usage: check-androidbox-apk-install0.sh --apk /absolute/path/to/fixture.apk

Runs the offline AndroidBox APK Install-0 QEMU gate. --apk is mandatory and
must name one absolute, non-empty regular file of at most 65024 bytes.
EOF
}

APK_PATH=""
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
    echo "--apk must be an absolute path." >&2
    exit 2
    ;;
esac
[[ -f "$APK_PATH" ]] || {
  echo "--apk must name a regular file: $APK_PATH" >&2
  exit 2
}

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp rm cp shasum wc; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the offline Install-0 gate." >&2
    exit 1
  }
done

APK_BYTES="$(wc -c <"$APK_PATH" | tr -d '[:space:]')"
[[ "$APK_BYTES" =~ ^[0-9]+$ ]] || {
  echo "Could not determine the APK byte length." >&2
  exit 1
}
if ((APK_BYTES == 0 || APK_BYTES > 65024)); then
  echo "--apk must contain 1..65024 bytes; observed $APK_BYTES." >&2
  exit 2
fi

SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
APKSIGNER="$SDK_ROOT/build-tools/36.1.0/apksigner"
[[ -x "$APKSIGNER" ]] || {
  echo "Official Android SDK apksigner 36.1.0 is required: $APKSIGNER" >&2
  exit 1
}

EXPECTED_APK_SHA256=2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7
EXPECTED_CERT_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
EXPECTED_PACKAGE=org.bndroid.demo
EXPECTED_ACTIVITY='Lorg/bndroid/demo/MainActivity;'
EXPECTED_VERSION_CODE=2
EXPECTED_APK_BYTES=12566
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-90}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

mkdir -p "$WORKSPACE_ROOT/target/androidbox-apk-install0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-apk-install0/check.XXXXXX"
)"
# Keep every helper's transient output inside the project evidence tree.
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-apk-install0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
INITIAL_IMAGE="$ARTIFACT_DIR/initial.raw"
NEGATIVE_IMAGE="$ARTIFACT_DIR/negative.raw"
PERSISTENT_IMAGE="$ARTIFACT_DIR/packages.raw"
SOURCE_APK="$ARTIFACT_DIR/source.apk"
TAMPERED_APK="$ARTIFACT_DIR/tampered.apk"
APKSIGNER_LOG="$ARTIFACT_DIR/apksigner.txt"
TAMPERED_APKSIGNER_LOG="$ARTIFACT_DIR/tampered-apksigner.txt"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_BUILD_LOG="$ARTIFACT_DIR/storage-build.log"
QEMU_PID=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill -TERM "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

show_failure() {
  if [[ -f "$BOOT_NORMALIZED_LOG" ]]; then
    tail -n 220 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 220 >&2
  fi
  if [[ -f "$BOOT_QEMU_LOG" ]]; then
    tail -n 100 "$BOOT_QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Install-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

cp "$APK_PATH" "$SOURCE_APK"
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_SHA256" == "$EXPECTED_APK_SHA256" ]] || {
  echo "The fixed Resources-1 fixture SHA-256 changed: $APK_SHA256" >&2
  echo "Install-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}
[[ "$APK_BYTES" == "$EXPECTED_APK_BYTES" ]] || {
  echo "The fixed Resources-1 fixture byte length changed: $APK_BYTES" >&2
  echo "Install-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

if ! "$APKSIGNER" verify --verbose --print-certs "$SOURCE_APK" \
  >"$APKSIGNER_LOG" 2>&1; then
  tail -n 100 "$APKSIGNER_LOG" >&2
  echo "Official apksigner rejected the fixed fixture." >&2
  exit 1
fi
for exact in \
  'Verified using v1 scheme (JAR signing): false' \
  'Verified using v2 scheme (APK Signature Scheme v2): true' \
  'Verified using v3 scheme (APK Signature Scheme v3): false' \
  'Verified using v3.1 scheme (APK Signature Scheme v3.1): false' \
  'Verified using v4 scheme (APK Signature Scheme v4): false' \
  'Verified for SourceStamp: false' \
  'Number of signers: 1' \
  "Signer #1 certificate SHA-256 digest: $EXPECTED_CERT_SHA256"; do
  [[ "$(grep -Fxc "$exact" "$APKSIGNER_LOG" || true)" == "1" ]] || {
    tail -n 100 "$APKSIGNER_LOG" >&2
    echo "Official apksigner did not confirm the fixed v2-only signer contract." >&2
    exit 1
  }
done

# Change one signed content byte while retaining the APK length and v2 block.
# This is a target-only artifact; the repository fixture remains untouched.
python3 - "$SOURCE_APK" "$TAMPERED_APK" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_bytes()
if len(source) <= 64:
    raise SystemExit("fixture is too short for the deterministic tamper")
tampered = bytearray(source)
tampered[64] ^= 0x01
Path(sys.argv[2]).write_bytes(tampered)
PY
if "$APKSIGNER" verify --verbose --print-certs "$TAMPERED_APK" \
  >"$TAMPERED_APKSIGNER_LOG" 2>&1; then
  echo "Official apksigner unexpectedly accepted the tampered APK." >&2
  exit 1
fi
TAMPERED_SHA256="$(shasum -a 256 "$TAMPERED_APK" | awk '{print $1}')"
[[ "$TAMPERED_SHA256" != "$EXPECTED_APK_SHA256" ]] || {
  echo "The deterministic signature negative did not change APK identity." >&2
  exit 1
}

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  "$SCRIPT_DIR/build-kernel.sh" >"$BUILD_LOG" 2>&1; then
  tail -n 180 "$BUILD_LOG" >&2
  echo "Install-0 kernel/userspace build failed." >&2
  exit 1
fi
if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$SCRIPT_DIR/build-storage-image.sh" --with-package-store \
  >"$STORAGE_BUILD_LOG" 2>&1; then
  tail -n 100 "$STORAGE_BUILD_LOG" >&2
  echo "The deterministic 16 MiB package-store image build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || {
  echo "Install-0 build did not produce the expected kernel image." >&2
  exit 1
}
INITIAL_BYTES="$(wc -c <"$INITIAL_IMAGE" | tr -d '[:space:]')"
[[ "$INITIAL_BYTES" == "16777216" ]] || {
  echo "Package-store image is not exactly 16 MiB: $INITIAL_BYTES bytes." >&2
  exit 1
}

cp "$INITIAL_IMAGE" "$NEGATIVE_IMAGE"
cp "$INITIAL_IMAGE" "$PERSISTENT_IMAGE"
INITIAL_SHA256="$(shasum -a 256 "$INITIAL_IMAGE" | awk '{print $1}')"
NEGATIVE_BEFORE_SHA256="$(shasum -a 256 "$NEGATIVE_IMAGE" | awk '{print $1}')"
[[ "$NEGATIVE_BEFORE_SHA256" == "$INITIAL_SHA256" ]] || {
  echo "Fresh negative disk does not match the deterministic initial image." >&2
  exit 1
}

normalize_boot_log() {
  tr -d '\r' <"$BOOT_SERIAL_LOG" >"$BOOT_NORMALIZED_LOG"
}

reject_panic_or_fatal() {
  normalize_boot_log
  if grep -Eqi \
    'fatal exception:|kernel panic:|panicked at|panic!|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)' \
    "$BOOT_NORMALIZED_LOG" \
    || grep -Eqi 'fatal|panic' "$BOOT_QEMU_LOG"; then
    fail_gate "QEMU boot emitted a panic or fatal marker."
  fi
}

wait_for_boot_pattern() {
  local pattern="$1"
  local description="$2"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
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

stop_qemu() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill -TERM "$QEMU_PID"
    wait "$QEMU_PID" 2>/dev/null || true
  fi
  QEMU_PID=""
  normalize_boot_log
  reject_panic_or_fatal
}

# The only literal QEMU launch in this gate. Every call uses the same
# persistent raw block path supplied by the caller. fw_cfg is present only
# when source_apk is non-empty; recovery boots therefore have no APK source.
run_qemu_boot() {
  local name="$1"
  local disk="$2"
  local source_apk="$3"
  local completion_pattern="$4"
  local completion_description="$5"
  # Keep the array non-empty for the macOS system Bash 3.2 + `set -u`
  # combination; only the conditional tail carries an APK source.
  local -a source_args=(-name "Bndroid Install-0 Gate $name")
  if [[ -n "$source_apk" ]]; then
    source_args+=(-fw_cfg "name=opt/bndroid/apk,file=$source_apk")
  fi

  BOOT_SERIAL_LOG="$ARTIFACT_DIR/$name.serial.log"
  BOOT_NORMALIZED_LOG="$ARTIFACT_DIR/$name.serial.normalized.log"
  BOOT_QEMU_LOG="$ARTIFACT_DIR/$name.qemu.log"
  : >"$BOOT_SERIAL_LOG"
  : >"$BOOT_NORMALIZED_LOG"
  : >"$BOOT_QEMU_LOG"

  qemu-system-aarch64 \
    -machine virt,gic-version=2,secure=off,virtualization=off \
    -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
    -nic none \
    -rtc base=2026-07-29T09:41:00,clock=vm \
    -serial "file:$BOOT_SERIAL_LOG" \
    -no-reboot -kernel "$KERNEL_IMAGE" -device ramfb \
    -global virtio-mmio.force-legacy=false \
    -drive "if=none,file=$disk,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
    -device virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off \
    -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
    -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
    "${source_args[@]}" \
    >"$BOOT_QEMU_LOG" 2>&1 &
  QEMU_PID=$!

  wait_for_boot_pattern "$completion_pattern" "$completion_description"
  stop_qemu
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

require_once_fixed() {
  local log="$1"
  local expected="$2"
  local description="$3"
  if [[ "$(grep -Fxc "$expected" "$log" || true)" != "1" ]]; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Missing, duplicated, or non-exact $description."
  fi
}

reject_success_on_negative() {
  local log="$1"
  if grep -Eq \
    '^APK_PACKAGE_STORE_OK |^APK_PACKAGE_STORE_REMOVED_OK |^ANDROIDBOX_INSTALLED_ACTIVITY_OK |^BLOCK_LAYER_OK |^STORAGE_LIMITS |^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
    "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Tampered APK rejection emitted post-install success evidence."
  fi
}

validate_source_marker() {
  local log="$1"
  local present="$2"
  local bytes="$3"
  local digest="$4"
  local selector="0x0000"
  local directory_files=11
  local dma_ops=2
  if [[ "$present" == "1" ]]; then
    selector="0x002a"
    directory_files=12
    dma_ops=3
  fi
  require_once_fixed \
    "$log" \
    "APK_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/apk present=$present bytes=$bytes selector=$selector directory_files=$directory_files dma_ops=$dma_ops sha256=$digest explicit_source=1 host_directory_scan=0 network=disabled install_mutation=0" \
    "APK source marker"
  require_once_fixed \
    "$log" \
    "APK_UNINSTALL_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/package-uninstall present=0 bytes=0 selector=0x0000 directory_files=$directory_files dma_ops=$dma_ops sha256=0000000000000000000000000000000000000000000000000000000000000000 wire=BNDUNS01 wire_bytes=256 canonical_validation=1 explicit_source=1 host_directory_scan=0 network=disabled package_mutation=0" \
    "absent package-uninstall source marker"
}

PROFILE_MARKER='ANDROIDBOX_APK_INSTALL0_PROFILE_OK format=3 apk_source=qemu-fw_cfg-or-package-store signature=apk-v2-single-signer package_store=single-package-crash-consistent-stateful launch_source=boot-and-click-durable-readback update0=same-package-same-signer-monotonic-version uninstall0=canonical-host-request-identical-dual-tombstone reinstall0=retained-package-signer-no-version-rollback atomic_old-new-switch=1 atomic_installed-removed-switch=1 exact-source-replay=idempotent-zero-write exact-uninstall-replay=idempotent-zero-write rollback-source=reject-before-write app_data_policy=no-managed-package-data apk_blob_erased=0 old_kernel_downgrade_safe=0 snapshot_syscall=59 snapshot_wire=BNDAPS01 snapshot_bytes=640 relaunch_syscall=60 relaunch_request_wire=BNDARQ01 relaunch_response_wire=BNDAPS01 relaunch_mode=async-exact-retry relaunch_owner=launcher-generation relaunch_writes=0 ui_catalog=kernel-supplied ui_installed_launch=fresh-durable-reexecution launcher_resolution=manifest-main-launcher activity_class_binding=exact-dex-descriptor fixed_dex_probe_required=0 activity_lifecycle=constructor-then-onCreate constructor_required=1 apk_bytes_exposed_to_el0=0 storage_authority_granted_to_el0=0 install_ui=0 uninstall_ui=0 update_ui=0 package_manager_api=0 art=0 dalvik=0 activitythread=0 framework=Resources-1-subset binder=0 bionic=0 jni=0 native_lib=0 permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled emulator_only=1 real_phone_claim=0'

validate_installed_boot() {
  local log="$1"
  local source_present="$2"
  local source_admitted="$3"
  local formatted="$4"
  local reads="$5"
  local writes="$6"
  local flushes="$7"
  local source_free="$8"
  local operation
  local previous_generation
  local previous_version_code
  local mutation_performed
  if [[ "$source_present" == "1" ]]; then
    operation=install
    previous_generation=0
    previous_version_code=0
    mutation_performed=1
  else
    operation=recovery
    previous_generation=1
    previous_version_code="$EXPECTED_VERSION_CODE"
    mutation_performed=0
  fi

  require_once_ere \
    "$log" \
    "^BLOCK_LAYER_OK .* device_sectors=32768 .* package_reads=${reads} package_writes=${writes} package_flushes=${flushes} .* timeouts=0 dma_frames=2$" \
    "Install-0 block-layer marker"
  require_once_ere \
    "$log" \
    '^PACKAGES_GPT_OK partition_index=3 partition_lba=16384-16895 partition_sectors=512 name=BNDROID_PACKAGES type=private fixed_identity=1 data_overlap=0 appdata_overlap=0 system_overlap=0$' \
    "package GPT marker"
  require_once_fixed \
    "$log" \
    "APK_PACKAGE_STORE_OK format=3 formatted_this_boot=${formatted} source_present=${source_present} source_admitted=${source_admitted} uninstall_request_present=0 uninstall_request_used=0 operation=${operation} previous_generation=${previous_generation} previous_version_code=${previous_version_code} installed=1 removed=0 generation=1 slot=0 apk_bytes=${EXPECTED_APK_BYTES} version_code=${EXPECTED_VERSION_CODE} package=${EXPECTED_PACKAGE} activity=${EXPECTED_ACTIVITY} profile=Resources-1 registry_blob_bound=1 full_readback=1 durable_reverification=1 mutation_performed=${mutation_performed} reads=${reads} writes=${writes} flushes=${flushes} source_free_zero_writes=${source_free} source_replay_zero_writes=0 reinstall_from_tombstone=0 apk_sha256=${EXPECTED_APK_SHA256} signer_cert_sha256=${EXPECTED_CERT_SHA256} network=disabled el0_package_write=0 general_android_compatibility=0" \
    "durable installed-package marker"
  require_once_ere \
    "$log" \
    '^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=AndroidBox Resources-1 Demo text=AndroidBox resource-backed view constructor=1 constructor_method=[0-9]+ constructor_code_offset=[0-9]+ constructor_instructions=2 on_create=1 on_create_method=[0-9]+ on_create_code_offset=[0-9]+ on_create_instructions=4 resources_arsc_crc=2674378676 layout_xml_crc=919508420 layout_resource_id=2130837504 string_resource_id=2130903040 set_content_view_int=1 apk_v2=1 signer_count=1 art=0 dalvik=0 binder=0 jni=0 native_lib=0 framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$' \
    "durable Resources-1 Activity marker"
  require_once_fixed \
    "$log" \
    "STORAGE_LIMITS writes=$((writes != 0 ? 1 : 0)) partitions=4 filesystems=1 vfs=1 persistence=package-store-only flush=1 package_store=single-package-stateful package_states=empty-installed-removed apk_max_bytes=65024 appdata_mounted=0 app_data_policy=no-managed-package-data data_persistence_advanced=0 el0_package_storage=0 apk_blob_erase=0 crash_consistency=double-registry-double-blob+mirrored-tombstone host_powercut_claim=0 physical_powerloss_claim=0 general_runtime=0" \
    "Install-0 storage limits marker"
  require_once_fixed "$log" "$PROFILE_MARKER" "Install-0 profile marker"
  if grep -Eq '^STORAGE_FAIL |^boot error:' "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "A valid Install-0 boot emitted a storage or boot failure."
  fi
}

# A fresh package disk plus a content-tampered v2 APK must fail before the
# first package write. Exact whole-disk equality proves that admission order.
run_qemu_boot \
  negative \
  "$NEGATIVE_IMAGE" \
  "$TAMPERED_APK" \
  '^boot error: storage validation failed: APK v2 signature admission failed$' \
  "the expected APK v2 signature rejection"
validate_source_marker "$BOOT_NORMALIZED_LOG" 1 "$EXPECTED_APK_BYTES" "$TAMPERED_SHA256"
require_once_ere \
  "$BOOT_NORMALIZED_LOG" \
  '^STORAGE_FAIL reason=package_manager_invalid$' \
  "tampered-APK package-manager rejection"
require_once_ere \
  "$BOOT_NORMALIZED_LOG" \
  '^boot error: storage validation failed: APK v2 signature admission failed$' \
  "tampered-APK boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
NEGATIVE_AFTER_SHA256="$(shasum -a 256 "$NEGATIVE_IMAGE" | awk '{print $1}')"
[[ "$NEGATIVE_AFTER_SHA256" == "$NEGATIVE_BEFORE_SHA256" ]] || {
  fail_gate "Tampered APK rejection changed the fresh package disk."
}

# The first valid boot receives the only positive fw_cfg APK source and commits
# generation 1. The following boots recover and execute solely from disk.
run_qemu_boot \
  boot1 \
  "$PERSISTENT_IMAGE" \
  "$SOURCE_APK" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the first complete mobile userspace boot"
validate_source_marker "$BOOT_NORMALIZED_LOG" 1 "$EXPECTED_APK_BYTES" "$EXPECTED_APK_SHA256"
validate_installed_boot "$BOOT_NORMALIZED_LOG" 1 1 1 1032 130 3 0
BOOT1_IMAGE_SHA256="$(shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}')"
[[ "$BOOT1_IMAGE_SHA256" != "$INITIAL_SHA256" ]] || {
  fail_gate "Valid first boot did not persist a package transaction."
}

run_qemu_boot \
  boot2 \
  "$PERSISTENT_IMAGE" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the first source-free recovery boot"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" \
  0 \
  0 \
  0000000000000000000000000000000000000000000000000000000000000000
validate_installed_boot "$BOOT_NORMALIZED_LOG" 0 0 0 389 0 0 1
BOOT2_IMAGE_SHA256="$(shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}')"
[[ "$BOOT2_IMAGE_SHA256" == "$BOOT1_IMAGE_SHA256" ]] || {
  fail_gate "The first source-free recovery boot changed the package disk."
}

BOOT3_BEFORE_SHA256="$BOOT2_IMAGE_SHA256"
run_qemu_boot \
  boot3 \
  "$PERSISTENT_IMAGE" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the second source-free recovery boot"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" \
  0 \
  0 \
  0000000000000000000000000000000000000000000000000000000000000000
validate_installed_boot "$BOOT_NORMALIZED_LOG" 0 0 0 389 0 0 1
BOOT3_AFTER_SHA256="$(shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}')"
[[ "$BOOT3_AFTER_SHA256" == "$BOOT3_BEFORE_SHA256" ]] || {
  fail_gate "The stable third boot changed the persistent package disk."
}

# Activity evidence on all three valid boots must be byte-identical; package
# identity is already checked against the same exact generation-1 contract.
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/boot1.serial.normalized.log" \
  >"$ARTIFACT_DIR/boot1.activity"
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/boot2.serial.normalized.log" \
  >"$ARTIFACT_DIR/boot2.activity"
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/boot3.serial.normalized.log" \
  >"$ARTIFACT_DIR/boot3.activity"
cmp "$ARTIFACT_DIR/boot1.activity" "$ARTIFACT_DIR/boot2.activity"
cmp "$ARTIFACT_DIR/boot2.activity" "$ARTIFACT_DIR/boot3.activity"

# Compare the complete images, not just GPT metadata. Every changed byte must
# lie strictly within LBA 16384..16895, and at least one package byte changed.
python3 - \
  "$INITIAL_IMAGE" \
  "$PERSISTENT_IMAGE" \
  "$ARTIFACT_DIR/disk-diff.txt" <<'PY'
from pathlib import Path
import sys

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
output = Path(sys.argv[3])
if len(before) != 16 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit("Install-0 disk comparison did not receive two 16 MiB images")
start = 16384 * 512
end = (16895 + 1) * 512
changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
if not changed:
    raise SystemExit("Install-0 package transaction changed no disk bytes")
outside = [index for index in changed if not start <= index < end]
if outside:
    raise SystemExit(
        f"Install-0 changed byte {outside[0]} outside LBA 16384..16895"
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

cat >"$ARTIFACT_DIR/summary.txt" <<EOF
apk_path=$APK_PATH
apk_bytes=$EXPECTED_APK_BYTES
apk_sha256=$EXPECTED_APK_SHA256
signer_cert_sha256=$EXPECTED_CERT_SHA256
signature_scheme=v2-only
signers=1
initial_disk_sha256=$INITIAL_SHA256
negative_disk_before_sha256=$NEGATIVE_BEFORE_SHA256
negative_disk_after_sha256=$NEGATIVE_AFTER_SHA256
boot1_disk_sha256=$BOOT1_IMAGE_SHA256
boot2_disk_sha256=$BOOT2_IMAGE_SHA256
boot3_before_disk_sha256=$BOOT3_BEFORE_SHA256
boot3_after_disk_sha256=$BOOT3_AFTER_SHA256
package=$EXPECTED_PACKAGE
activity=$EXPECTED_ACTIVITY
version_code=$EXPECTED_VERSION_CODE
generation=1
first_boot_package_writes=130
first_boot_package_flushes=3
recovery_boot_package_writes=0
recovery_boot_package_flushes=0
network=disabled
general_android_compatibility=0
EOF

echo "ANDROIDBOX_APK_INSTALL0_QEMU_OK artifact_dir=$ARTIFACT_DIR apk_sha256=$EXPECTED_APK_SHA256 signer_cert_sha256=$EXPECTED_CERT_SHA256 generation=1 boot1_writes=130 boot1_flushes=3 recovery_writes=0 recovery_flushes=0"
