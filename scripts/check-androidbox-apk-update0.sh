#!/usr/bin/env bash
set -euo pipefail

# Offline local-QEMU acceptance gate for the deliberately bounded AndroidBox
# APK Update-0 profile. Both APK inputs are exact explicit files. No directory
# is searched for an APK and QEMU receives no network interface.
#
# This proves one v2-signed Resources-1 package moving from versionCode 2 /
# generation 1 to versionCode 3 / generation 2, atomic durable recovery,
# idempotent exact-source replay, rollback rejection, and signature-tamper
# rejection. It does not claim arbitrary APK compatibility, ART/Dalvik,
# Binder/Bionic/JNI, Android Framework, native libraries, hardware, networking,
# an update UI, or a real phone.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-apk-update0.sh \' \
    '  --base-apk /absolute/path/to/version-2.apk \' \
    '  --update-apk /absolute/path/to/version-3.apk' \
    '' \
    'Runs the offline AndroidBox APK Update-0 QEMU gate. Both arguments are' \
    'mandatory absolute paths to explicit, non-empty regular APK files of at' \
    'most 65024 bytes. The gate never searches a directory for APK inputs.'
}

BASE_APK_PATH=""
UPDATE_APK_PATH=""
while (($#)); do
  case "$1" in
    --base-apk)
      if [[ -n "$BASE_APK_PATH" || $# -lt 2 ]]; then
        usage >&2
        exit 2
      fi
      BASE_APK_PATH="$2"
      shift 2
      ;;
    --update-apk)
      if [[ -n "$UPDATE_APK_PATH" || $# -lt 2 ]]; then
        usage >&2
        exit 2
      fi
      UPDATE_APK_PATH="$2"
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

if [[ -z "$BASE_APK_PATH" || -z "$UPDATE_APK_PATH" ]]; then
  usage >&2
  exit 2
fi
for explicit_path in "$BASE_APK_PATH" "$UPDATE_APK_PATH"; do
  case "$explicit_path" in
    /*) ;;
    *)
      echo "Both APK paths must be absolute: $explicit_path" >&2
      exit 2
      ;;
  esac
  [[ -f "$explicit_path" ]] || {
    echo "APK input must name a regular file: $explicit_path" >&2
    exit 2
  }
done
[[ "$BASE_APK_PATH" != "$UPDATE_APK_PATH" ]] || {
  echo "--base-apk and --update-apk must be different explicit files." >&2
  exit 2
}

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp rm cp shasum wc sed; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the offline Update-0 gate." >&2
    exit 1
  }
done

SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
APKSIGNER="${BNDROID_APKSIGNER:-"$SDK_ROOT/build-tools/36.1.0/apksigner"}"
AAPT2="${BNDROID_AAPT2:-"$SDK_ROOT/build-tools/36.1.0/aapt2"}"
for sdk_tool in "$APKSIGNER" "$AAPT2"; do
  case "$sdk_tool" in
    /*) ;;
    *)
      echo "Android SDK tool paths must be absolute: $sdk_tool" >&2
      exit 2
      ;;
  esac
  [[ -x "$sdk_tool" ]] || {
    echo "Required Android SDK tool is not executable: $sdk_tool" >&2
    exit 1
  }
done

BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-90}"
[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}

mkdir -p "$WORKSPACE_ROOT/target/androidbox-apk-update0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-apk-update0/check.XXXXXX"
)"
# Keep every helper's transient output inside the project evidence tree.
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-apk-update0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
BASE_SOURCE_APK="$ARTIFACT_DIR/base-source.apk"
UPDATE_SOURCE_APK="$ARTIFACT_DIR/update-source.apk"
TAMPERED_UPDATE_APK="$ARTIFACT_DIR/tampered-update.apk"
INITIAL_IMAGE="$ARTIFACT_DIR/initial.raw"
PERSISTENT_IMAGE="$ARTIFACT_DIR/packages.raw"
GENERATION1_IMAGE="$ARTIFACT_DIR/generation1.raw"
GENERATION2_IMAGE="$ARTIFACT_DIR/generation2.raw"
ROLLBACK_IMAGE="$ARTIFACT_DIR/rollback-negative.raw"
TAMPER_IMAGE="$ARTIFACT_DIR/tamper-negative.raw"
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
    tail -n 240 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 240 >&2
  fi
  if [[ -f "$BOOT_QEMU_LOG" ]]; then
    tail -n 120 "$BOOT_QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Update-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

cp "$BASE_APK_PATH" "$BASE_SOURCE_APK"
cp "$UPDATE_APK_PATH" "$UPDATE_SOURCE_APK"

# Official SDK tools provide the signer, package, Activity, and version
# identities. aapt2 and the ZIP central directory also derive the exact
# Resources-1 output expected from the durable readback; no fixture hash or
# package identity is baked into this gate.
inspect_apk() {
  local prefix="$1"
  local label="$2"
  local apk="$3"
  local signer_log="$ARTIFACT_DIR/$label.apksigner.txt"
  local badging_log="$ARTIFACT_DIR/$label.badging.txt"
  local resources_log="$ARTIFACT_DIR/$label.resources.txt"
  local layout_log="$ARTIFACT_DIR/$label.layout.txt"
  local metadata_log="$ARTIFACT_DIR/$label.metadata.txt"
  local apk_bytes
  local apk_sha256
  local cert_sha256

  apk_bytes="$(wc -c <"$apk" | tr -d '[:space:]')"
  [[ "$apk_bytes" =~ ^[0-9]+$ ]] || {
    echo "Could not determine $label APK byte length." >&2
    exit 1
  }
  if ((apk_bytes == 0 || apk_bytes > 65024)); then
    echo "$label APK must contain 1..65024 bytes; observed $apk_bytes." >&2
    exit 2
  fi
  apk_sha256="$(shasum -a 256 "$apk" | awk '{print $1}')"
  [[ "$apk_sha256" =~ ^[0-9a-f]{64}$ ]] || {
    echo "Could not determine the exact $label APK SHA-256." >&2
    exit 1
  }

  if ! "$APKSIGNER" verify --verbose --print-certs "$apk" \
    >"$signer_log" 2>&1; then
    tail -n 120 "$signer_log" >&2
    echo "Official apksigner rejected the $label APK." >&2
    exit 1
  fi
  for exact in \
    'Verified using v1 scheme (JAR signing): false' \
    'Verified using v2 scheme (APK Signature Scheme v2): true' \
    'Verified using v3 scheme (APK Signature Scheme v3): false' \
    'Verified using v3.1 scheme (APK Signature Scheme v3.1): false' \
    'Verified using v4 scheme (APK Signature Scheme v4): false' \
    'Verified for SourceStamp: false' \
    'Number of signers: 1'; do
    [[ "$(grep -Fxc "$exact" "$signer_log" || true)" == "1" ]] || {
      tail -n 120 "$signer_log" >&2
      echo "$label APK is not the exact v2-only, one-signer contract." >&2
      exit 1
    }
  done
  cert_sha256="$(
    sed -n 's/^Signer #1 certificate SHA-256 digest: //p' "$signer_log" \
      | tr '[:upper:]' '[:lower:]'
  )"
  [[ "$cert_sha256" =~ ^[0-9a-f]{64}$ ]] || {
    tail -n 120 "$signer_log" >&2
    echo "Could not derive the one $label signer certificate SHA-256." >&2
    exit 1
  }
  [[ "$(grep -c '^Signer #1 certificate SHA-256 digest: ' "$signer_log" || true)" == "1" ]] || {
    echo "$label APK emitted an ambiguous signer certificate digest." >&2
    exit 1
  }

  "$AAPT2" dump badging "$apk" >"$badging_log"
  "$AAPT2" dump resources "$apk" >"$resources_log"
  "$AAPT2" dump xmltree --file res/layout/activity_main.xml "$apk" \
    >"$layout_log"
  python3 - \
    "$apk" \
    "$badging_log" \
    "$resources_log" \
    "$layout_log" \
    "$metadata_log" <<'PY'
from pathlib import Path
import re
import sys
import zipfile

apk_path = Path(sys.argv[1])
badging = Path(sys.argv[2]).read_text(encoding="utf-8")
resources = Path(sys.argv[3]).read_text(encoding="utf-8")
layout = Path(sys.argv[4]).read_text(encoding="utf-8")
output = Path(sys.argv[5])

def one(pattern: str, value: str, description: str, flags: int = 0) -> re.Match[str]:
    matches = list(re.finditer(pattern, value, flags))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one {description}; observed {len(matches)}")
    return matches[0]

package_match = one(
    r"^package: name='([^']+)' versionCode='([0-9]+)' .*$",
    badging,
    "aapt2 package/version line",
    re.MULTILINE,
)
package = package_match.group(1)
version_code = int(package_match.group(2), 10)
activity = one(
    r"^launchable-activity: name='([^']+)' .*$",
    badging,
    "launchable Activity",
    re.MULTILINE,
).group(1)
title = one(
    r"^application-label:'([^']+)'$",
    badging,
    "application label",
    re.MULTILINE,
).group(1)
layout_match = one(
    r"^\s*resource (0x[0-9a-fA-F]+) layout/[^\s]+\n"
    r"\s+\(\) \(file\) (res/[^\s]+) type=XML$",
    resources,
    "compiled layout resource",
    re.MULTILINE,
)
layout_id_hex = layout_match.group(1).lower()
layout_entry = layout_match.group(2)
text_id_hex = one(
    r"android:text\(0x[0-9a-fA-F]+\)=@(0x[0-9a-fA-F]+)$",
    layout,
    "TextView text resource reference",
    re.MULTILINE,
).group(1).lower()
text = one(
    rf"^\s*resource {re.escape(text_id_hex)} string/[^\s]+\n\s+\(\) \"([^\"]*)\"$",
    resources,
    "TextView string resource",
    re.MULTILINE,
).group(1)

if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+", package):
    raise SystemExit("package name is outside the bounded AndroidBox contract")
if activity.startswith("."):
    activity = package + activity
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+", activity):
    raise SystemExit("Activity name is outside the bounded AndroidBox contract")
if not activity.startswith(package + "."):
    raise SystemExit("launchable Activity is outside the admitted package")
descriptor = "L" + activity.replace(".", "/") + ";"
for name, value in (("title", title), ("TextView text", text)):
    if not value or not value.isascii() or any(ord(char) < 0x20 for char in value):
        raise SystemExit(f"{name} is not one non-empty printable ASCII line")

with zipfile.ZipFile(apk_path, "r") as archive:
    resources_crc = archive.getinfo("resources.arsc").CRC
    layout_crc = archive.getinfo(layout_entry).CRC

lines = (
    package,
    descriptor,
    str(version_code),
    title,
    text,
    str(resources_crc),
    str(layout_crc),
    str(int(layout_id_hex, 16)),
    str(int(text_id_hex, 16)),
)
output.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
  [[ "$(wc -l <"$metadata_log" | tr -d '[:space:]')" == "9" ]] || {
    echo "Could not derive the exact $label Resources-1 metadata." >&2
    exit 1
  }

  printf -v "${prefix}_APK_BYTES" '%s' "$apk_bytes"
  printf -v "${prefix}_APK_SHA256" '%s' "$apk_sha256"
  printf -v "${prefix}_CERT_SHA256" '%s' "$cert_sha256"
  printf -v "${prefix}_PACKAGE" '%s' "$(sed -n '1p' "$metadata_log")"
  printf -v "${prefix}_ACTIVITY" '%s' "$(sed -n '2p' "$metadata_log")"
  printf -v "${prefix}_VERSION_CODE" '%s' "$(sed -n '3p' "$metadata_log")"
  printf -v "${prefix}_TITLE" '%s' "$(sed -n '4p' "$metadata_log")"
  printf -v "${prefix}_TEXT" '%s' "$(sed -n '5p' "$metadata_log")"
  printf -v "${prefix}_RESOURCES_CRC" '%s' "$(sed -n '6p' "$metadata_log")"
  printf -v "${prefix}_LAYOUT_CRC" '%s' "$(sed -n '7p' "$metadata_log")"
  printf -v "${prefix}_LAYOUT_ID" '%s' "$(sed -n '8p' "$metadata_log")"
  printf -v "${prefix}_TEXT_ID" '%s' "$(sed -n '9p' "$metadata_log")"
}

inspect_apk BASE base "$BASE_SOURCE_APK"
inspect_apk UPDATE update "$UPDATE_SOURCE_APK"

[[ "$BASE_VERSION_CODE" == "2" ]] || {
  echo "Update-0 base must have versionCode 2; observed $BASE_VERSION_CODE." >&2
  exit 2
}
[[ "$UPDATE_VERSION_CODE" == "3" ]] || {
  echo "Update-0 source must have versionCode 3; observed $UPDATE_VERSION_CODE." >&2
  exit 2
}
((UPDATE_VERSION_CODE > BASE_VERSION_CODE)) || {
  echo "Update-0 source version is not strictly greater than the base." >&2
  exit 2
}
[[ "$BASE_PACKAGE" == "$UPDATE_PACKAGE" ]] || {
  echo "Update-0 requires the same package identity." >&2
  exit 2
}
[[ "$BASE_ACTIVITY" == "$UPDATE_ACTIVITY" ]] || {
  echo "Update-0 fixture unexpectedly changed its admitted Activity." >&2
  exit 2
}
[[ "$BASE_CERT_SHA256" == "$UPDATE_CERT_SHA256" ]] || {
  echo "Update-0 requires the same v2 signer certificate." >&2
  exit 2
}
[[ "$BASE_APK_SHA256" != "$UPDATE_APK_SHA256" ]] || {
  echo "Base and update APK content identities must differ." >&2
  exit 2
}
[[ "$BASE_TEXT" != "$UPDATE_TEXT" ]] || {
  echo "Update-0 fixture must change the rendered Activity TextView." >&2
  exit 2
}

# Flip one signed content byte without changing the update APK length or its
# v2 signing block. Official apksigner must reject the target-only artifact.
python3 - "$UPDATE_SOURCE_APK" "$TAMPERED_UPDATE_APK" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_bytes()
if len(source) <= 64:
    raise SystemExit("update APK is too short for the deterministic tamper")
tampered = bytearray(source)
tampered[64] ^= 0x01
Path(sys.argv[2]).write_bytes(tampered)
PY
TAMPERED_UPDATE_BYTES="$(wc -c <"$TAMPERED_UPDATE_APK" | tr -d '[:space:]')"
TAMPERED_UPDATE_SHA256="$(
  shasum -a 256 "$TAMPERED_UPDATE_APK" | awk '{print $1}'
)"
[[ "$TAMPERED_UPDATE_BYTES" == "$UPDATE_APK_BYTES" ]] || {
  echo "Signature tamper unexpectedly changed the update APK length." >&2
  exit 1
}
[[ "$TAMPERED_UPDATE_SHA256" != "$UPDATE_APK_SHA256" ]] || {
  echo "Signature tamper did not change update APK identity." >&2
  exit 1
}
if "$APKSIGNER" verify --verbose --print-certs "$TAMPERED_UPDATE_APK" \
  >"$ARTIFACT_DIR/tampered-update.apksigner.txt" 2>&1; then
  echo "Official apksigner unexpectedly accepted the tampered update APK." >&2
  exit 1
fi

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  "$SCRIPT_DIR/build-kernel.sh" >"$BUILD_LOG" 2>&1; then
  tail -n 200 "$BUILD_LOG" >&2
  echo "Update-0 kernel/userspace build failed." >&2
  exit 1
fi
if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$SCRIPT_DIR/build-storage-image.sh" --with-package-store \
  >"$STORAGE_BUILD_LOG" 2>&1; then
  tail -n 120 "$STORAGE_BUILD_LOG" >&2
  echo "The deterministic 16 MiB package-store image build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || {
  echo "Update-0 build did not produce the expected kernel image." >&2
  exit 1
}
INITIAL_BYTES="$(wc -c <"$INITIAL_IMAGE" | tr -d '[:space:]')"
[[ "$INITIAL_BYTES" == "16777216" ]] || {
  echo "Package-store image is not exactly 16 MiB: $INITIAL_BYTES bytes." >&2
  exit 1
}
cp "$INITIAL_IMAGE" "$PERSISTENT_IMAGE"
INITIAL_SHA256="$(shasum -a 256 "$INITIAL_IMAGE" | awk '{print $1}')"

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

# This is the gate's only literal QEMU launch. Every invocation operates on
# one explicit raw disk path. fw_cfg is present only for a non-empty source.
run_qemu_boot() {
  local name="$1"
  local disk="$2"
  local source_apk="$3"
  local completion_pattern="$4"
  local completion_description="$5"
  # Keep the array non-empty for macOS Bash 3.2 with `set -u`.
  local -a source_args=(-name "Bndroid Update-0 Gate $name")
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

require_once_fixed() {
  local log="$1"
  local expected="$2"
  local description="$3"
  if [[ "$(grep -Fxc "$expected" "$log" || true)" != "1" ]]; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Missing, duplicated, or non-exact $description."
  fi
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
  local formatted="$3"
  local operation="$4"
  local previous_generation="$5"
  local previous_version="$6"
  local generation="$7"
  local slot="$8"
  local apk_bytes="$9"
  shift 9
  local version="$1"
  local package="$2"
  local activity="$3"
  local apk_sha256="$4"
  local cert_sha256="$5"
  local reads="$6"
  local writes="$7"
  local flushes="$8"
  local source_free="$9"
  shift 9
  local source_replay="$1"
  local title="$2"
  local text="$3"
  local resources_crc="$4"
  local layout_crc="$5"
  local layout_id="$6"
  local text_id="$7"
  local mutation=0
  local requests
  local bytes_read
  local bytes_written
  if ((writes != 0)); then
    mutation=1
  fi
  requests=$((537 + reads + writes + flushes))
  bytes_read=$(((537 + reads) * 512))
  bytes_written=$((writes * 512))

  require_once_fixed \
    "$log" \
    "BLOCK_LAYER_OK sector_size=512 device_sectors=32768 parser_reads=535 package_reads=$reads package_writes=$writes package_flushes=$flushes requests=$requests completions=$requests bytes_read=$bytes_read bytes_written=$bytes_written irq_completions=$requests poll_fallbacks=0 timeouts=0 dma_frames=2" \
    "block-layer marker"
  require_once_fixed \
    "$log" \
    'PACKAGES_GPT_OK partition_index=3 partition_lba=16384-16895 partition_sectors=512 name=BNDROID_PACKAGES type=private fixed_identity=1 data_overlap=0 appdata_overlap=0 system_overlap=0' \
    "package GPT marker"
  require_once_fixed \
    "$log" \
    "APK_PACKAGE_STORE_OK format=3 formatted_this_boot=$formatted source_present=$source_present source_admitted=$source_present uninstall_request_present=0 uninstall_request_used=0 operation=$operation previous_generation=$previous_generation previous_version_code=$previous_version installed=1 removed=0 generation=$generation slot=$slot apk_bytes=$apk_bytes version_code=$version package=$package activity=$activity profile=Resources-1 registry_blob_bound=1 full_readback=1 durable_reverification=1 mutation_performed=$mutation reads=$reads writes=$writes flushes=$flushes source_free_zero_writes=$source_free source_replay_zero_writes=$source_replay reinstall_from_tombstone=0 apk_sha256=$apk_sha256 signer_cert_sha256=$cert_sha256 network=disabled el0_package_write=0 general_android_compatibility=0" \
    "durable package decision marker"
  require_once_ere \
    "$log" \
    "^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=$title text=$text constructor=1 constructor_method=[0-9]+ constructor_code_offset=[0-9]+ constructor_instructions=2 on_create=1 on_create_method=[0-9]+ on_create_code_offset=[0-9]+ on_create_instructions=4 resources_arsc_crc=$resources_crc layout_xml_crc=$layout_crc layout_resource_id=$layout_id string_resource_id=$text_id set_content_view_int=1 apk_v2=1 signer_count=1 art=0 dalvik=0 binder=0 jni=0 native_lib=0 framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$" \
    "durable Resources-1 Activity marker"
  require_once_fixed \
    "$log" \
    "STORAGE_LIMITS writes=$mutation partitions=4 filesystems=1 vfs=1 persistence=package-store-only flush=1 package_store=single-package-stateful package_states=empty-installed-removed apk_max_bytes=65024 appdata_mounted=0 app_data_policy=no-managed-package-data data_persistence_advanced=0 el0_package_storage=0 apk_blob_erase=0 crash_consistency=double-registry-double-blob+mirrored-tombstone host_powercut_claim=0 physical_powerloss_claim=0 general_runtime=0" \
    "storage boundary marker"
  require_once_fixed "$log" "$PROFILE_MARKER" "Update-0 profile marker"
  require_once_ere \
    "$log" \
    '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=44 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 .* network=disabled .* real_phone_claim=0$' \
    "720x1600 mobile UI marker"
  if grep -Eq '^STORAGE_FAIL |^boot error:' "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "A valid Update-0 boot emitted a storage or boot failure."
  fi
}

reject_success_on_negative() {
  local log="$1"
  if grep -Eq \
    '^APK_PACKAGE_STORE_OK |^APK_PACKAGE_STORE_REMOVED_OK |^ANDROIDBOX_INSTALLED_ACTIVITY_OK |^BLOCK_LAYER_OK |^STORAGE_LIMITS |^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
    "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "A rejected source emitted post-transaction success evidence."
  fi
}

# Fresh -> base v2 -> generation 1 / slot 0.
run_qemu_boot \
  base-install \
  "$PERSISTENT_IMAGE" \
  "$BASE_SOURCE_APK" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the generation-1 base installation"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" 1 "$BASE_APK_BYTES" "$BASE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 1 install 0 0 1 0 \
  "$BASE_APK_BYTES" "$BASE_VERSION_CODE" "$BASE_PACKAGE" "$BASE_ACTIVITY" \
  "$BASE_APK_SHA256" "$BASE_CERT_SHA256" \
  1032 130 3 0 0 \
  "$BASE_TITLE" "$BASE_TEXT" "$BASE_RESOURCES_CRC" "$BASE_LAYOUT_CRC" \
  "$BASE_LAYOUT_ID" "$BASE_TEXT_ID"
cp "$PERSISTENT_IMAGE" "$GENERATION1_IMAGE"
cp "$GENERATION1_IMAGE" "$TAMPER_IMAGE"
GENERATION1_SHA256="$(
  shasum -a 256 "$GENERATION1_IMAGE" | awk '{print $1}'
)"
[[ "$GENERATION1_SHA256" != "$INITIAL_SHA256" ]] || {
  fail_gate "Base installation did not persist generation 1."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/base-install.serial.normalized.log" \
  >"$ARTIFACT_DIR/base.activity"

# generation 1 -> same package/signer v3 -> generation 2 / slot 1.
run_qemu_boot \
  update \
  "$PERSISTENT_IMAGE" \
  "$UPDATE_SOURCE_APK" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the generation-2 package update"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" 1 "$UPDATE_APK_BYTES" "$UPDATE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 0 update 1 "$BASE_VERSION_CODE" 2 1 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  905 129 2 0 0 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
cp "$PERSISTENT_IMAGE" "$GENERATION2_IMAGE"
cp "$GENERATION2_IMAGE" "$ROLLBACK_IMAGE"
GENERATION2_SHA256="$(
  shasum -a 256 "$GENERATION2_IMAGE" | awk '{print $1}'
)"
[[ "$GENERATION2_SHA256" != "$GENERATION1_SHA256" ]] || {
  fail_gate "Version-3 update did not persist generation 2."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/update.serial.normalized.log" \
  >"$ARTIFACT_DIR/update.activity"
if cmp -s "$ARTIFACT_DIR/base.activity" "$ARTIFACT_DIR/update.activity"; then
  fail_gate "Durable Activity evidence did not change from base to update."
fi

# Refeeding the exact committed v3 transaction must be a zero-write,
# zero-flush source replay that preserves generation 2.
REPLAY_BEFORE_SHA256="$(
  shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}'
)"
run_qemu_boot \
  replay \
  "$PERSISTENT_IMAGE" \
  "$UPDATE_SOURCE_APK" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the exact-source idempotent replay"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" 1 "$UPDATE_APK_BYTES" "$UPDATE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 0 source-replay 2 "$UPDATE_VERSION_CODE" 2 1 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  904 0 0 0 1 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
REPLAY_AFTER_SHA256="$(
  shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}'
)"
[[ "$REPLAY_AFTER_SHA256" == "$REPLAY_BEFORE_SHA256" ]] || {
  fail_gate "Exact update-source replay changed the generation-2 disk."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/replay.serial.normalized.log" \
  >"$ARTIFACT_DIR/replay.activity"
cmp "$ARTIFACT_DIR/update.activity" "$ARTIFACT_DIR/replay.activity"

# A source-free boot must recover and launch generation 2 without mutation.
RECOVERY_BEFORE_SHA256="$REPLAY_AFTER_SHA256"
run_qemu_boot \
  recovery \
  "$PERSISTENT_IMAGE" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the source-free generation-2 recovery"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" \
  0 \
  0 \
  0000000000000000000000000000000000000000000000000000000000000000
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  0 0 recovery 2 "$UPDATE_VERSION_CODE" 2 1 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  645 0 0 1 0 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
RECOVERY_AFTER_SHA256="$(
  shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}'
)"
[[ "$RECOVERY_AFTER_SHA256" == "$RECOVERY_BEFORE_SHA256" ]] || {
  fail_gate "Source-free generation-2 recovery changed the package disk."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/recovery.serial.normalized.log" \
  >"$ARTIFACT_DIR/recovery.activity"
cmp "$ARTIFACT_DIR/update.activity" "$ARTIFACT_DIR/recovery.activity"

# Feeding old v2 to generation 2 must fail before any disk mutation.
ROLLBACK_BEFORE_SHA256="$(
  shasum -a 256 "$ROLLBACK_IMAGE" | awk '{print $1}'
)"
run_qemu_boot \
  rollback-negative \
  "$ROLLBACK_IMAGE" \
  "$BASE_SOURCE_APK" \
  '^boot error: storage validation failed: package update version must strictly increase$' \
  "the monotonic-version rollback rejection"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" 1 "$BASE_APK_BYTES" "$BASE_APK_SHA256"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'STORAGE_FAIL reason=package_manager_invalid' \
  "rollback package-manager rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: storage validation failed: package update version must strictly increase' \
  "rollback boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
ROLLBACK_AFTER_SHA256="$(
  shasum -a 256 "$ROLLBACK_IMAGE" | awk '{print $1}'
)"
[[ "$ROLLBACK_AFTER_SHA256" == "$ROLLBACK_BEFORE_SHA256" ]] || {
  fail_gate "Rejected version-2 rollback changed the generation-2 disk."
}

# A content-tampered v3 source against saved generation 1 must fail signature
# admission before any package-store mutation.
TAMPER_BEFORE_SHA256="$(
  shasum -a 256 "$TAMPER_IMAGE" | awk '{print $1}'
)"
run_qemu_boot \
  tamper-negative \
  "$TAMPER_IMAGE" \
  "$TAMPERED_UPDATE_APK" \
  '^boot error: storage validation failed: APK v2 signature admission failed$' \
  "the tampered-update signature rejection"
validate_source_marker \
  "$BOOT_NORMALIZED_LOG" \
  1 \
  "$TAMPERED_UPDATE_BYTES" \
  "$TAMPERED_UPDATE_SHA256"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'STORAGE_FAIL reason=package_manager_invalid' \
  "tampered-update package-manager rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: storage validation failed: APK v2 signature admission failed' \
  "tampered-update boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
TAMPER_AFTER_SHA256="$(
  shasum -a 256 "$TAMPER_IMAGE" | awk '{print $1}'
)"
[[ "$TAMPER_AFTER_SHA256" == "$TAMPER_BEFORE_SHA256" ]] || {
  fail_gate "Rejected tampered v3 source changed the saved generation-1 disk."
}

# Compare complete images. Each positive transaction must change at least one
# byte, and every changed byte must stay within BNDROID_PACKAGES.
check_package_partition_diff() {
  local before="$1"
  local after="$2"
  local label="$3"
  local output="$4"
  python3 - "$before" "$after" "$label" "$output" <<'PY'
from pathlib import Path
import sys

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
label = sys.argv[3]
output = Path(sys.argv[4])
if len(before) != 16 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit(f"{label}: expected two exact 16 MiB disk images")
start = 16384 * 512
end = (16895 + 1) * 512
changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
if not changed:
    raise SystemExit(f"{label}: transaction changed no disk bytes")
outside = [index for index in changed if not start <= index < end]
if outside:
    raise SystemExit(
        f"{label}: changed byte {outside[0]} outside package LBA 16384..16895"
    )
output.write_text(
    "\n".join(
        (
            f"label={label}",
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
}

check_package_partition_diff \
  "$INITIAL_IMAGE" \
  "$GENERATION1_IMAGE" \
  base-install \
  "$ARTIFACT_DIR/base-install.disk-diff.txt"
check_package_partition_diff \
  "$GENERATION1_IMAGE" \
  "$GENERATION2_IMAGE" \
  update \
  "$ARTIFACT_DIR/update.disk-diff.txt"

printf '%s\n' \
  "base_apk_path=$BASE_APK_PATH" \
  "base_apk_bytes=$BASE_APK_BYTES" \
  "base_apk_sha256=$BASE_APK_SHA256" \
  "update_apk_path=$UPDATE_APK_PATH" \
  "update_apk_bytes=$UPDATE_APK_BYTES" \
  "update_apk_sha256=$UPDATE_APK_SHA256" \
  "signer_cert_sha256=$UPDATE_CERT_SHA256" \
  "package=$UPDATE_PACKAGE" \
  "activity=$UPDATE_ACTIVITY" \
  "base_version_code=$BASE_VERSION_CODE" \
  "update_version_code=$UPDATE_VERSION_CODE" \
  "base_activity_text=$BASE_TEXT" \
  "update_activity_text=$UPDATE_TEXT" \
  "initial_disk_sha256=$INITIAL_SHA256" \
  "generation1_disk_sha256=$GENERATION1_SHA256" \
  "generation2_disk_sha256=$GENERATION2_SHA256" \
  "replay_before_disk_sha256=$REPLAY_BEFORE_SHA256" \
  "replay_after_disk_sha256=$REPLAY_AFTER_SHA256" \
  "recovery_before_disk_sha256=$RECOVERY_BEFORE_SHA256" \
  "recovery_after_disk_sha256=$RECOVERY_AFTER_SHA256" \
  "rollback_before_disk_sha256=$ROLLBACK_BEFORE_SHA256" \
  "rollback_after_disk_sha256=$ROLLBACK_AFTER_SHA256" \
  "tamper_before_disk_sha256=$TAMPER_BEFORE_SHA256" \
  "tamper_after_disk_sha256=$TAMPER_AFTER_SHA256" \
  'base_generation=1' \
  'base_slot=0' \
  'update_generation=2' \
  'update_slot=1' \
  'update_writes=129' \
  'update_flushes=2' \
  'replay_writes=0' \
  'replay_flushes=0' \
  'recovery_writes=0' \
  'recovery_flushes=0' \
  'rollback_rejected_before_disk_change=1' \
  'tamper_rejected_before_disk_change=1' \
  'network=disabled' \
  'general_android_compatibility=0' \
  >"$ARTIFACT_DIR/summary.txt"

echo "ANDROIDBOX_APK_UPDATE0_QEMU_OK artifact_dir=$ARTIFACT_DIR base_apk_sha256=$BASE_APK_SHA256 update_apk_sha256=$UPDATE_APK_SHA256 signer_cert_sha256=$UPDATE_CERT_SHA256 base_generation=1 update_generation=2 update_slot=1 replay_writes=0 replay_flushes=0 recovery_writes=0 recovery_flushes=0 rollback_disk_unchanged=1 tamper_disk_unchanged=1"
