#!/usr/bin/env bash
set -euo pipefail

# Offline local-QEMU acceptance gate for the deliberately bounded AndroidBox
# boot-time APK Uninstall-0 and reinstall profile. Both APKs and every removal
# request are explicit files; no directory is searched and no network device
# is exposed.
#
# This proves one v2-signed Resources-1 package moving through:
#   v2/gen1 -> v3/gen2 -> removed/gen3 -> v3/gen4.
# It also proves identical mirrored tombstones, exact uninstall replay,
# source-free removed recovery, request-target rejection, CRC rejection,
# conflicting-input rejection, rollback rejection, and durable Activity
# recovery after the bounded reinstall. It does not claim arbitrary Android
# compatibility, runtime Settings uninstall, ART/Dalvik, Binder/Bionic/JNI,
# native libraries, networking, real hardware, or a real phone.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-apk-uninstall0.sh \' \
    '  --base-apk /absolute/path/to/version-2.apk \' \
    '  --update-apk /absolute/path/to/version-3.apk' \
    '' \
    'Runs the offline AndroidBox boot-time Uninstall-0 QEMU gate. Both APK' \
    'arguments are mandatory absolute paths to explicit, non-empty regular' \
    'files of at most 65024 bytes. No APK directory is searched.'
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
  awk cmp rm cp shasum wc sed chmod; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the offline Uninstall-0 gate." >&2
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

mkdir -p "$WORKSPACE_ROOT/target/androidbox-apk-uninstall0"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-apk-uninstall0/check.XXXXXX"
)"
# Keep every helper's transient output inside the project evidence tree.
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-apk-uninstall0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
BASE_SOURCE_APK="$ARTIFACT_DIR/base-source.apk"
UPDATE_SOURCE_APK="$ARTIFACT_DIR/update-source.apk"
UNINSTALL_REQUEST="$ARTIFACT_DIR/uninstall-request.bin"
WRONG_TARGET_REQUEST="$ARTIFACT_DIR/wrong-target-request.bin"
STALE_GENERATION_REQUEST="$ARTIFACT_DIR/stale-generation-request.bin"
BAD_CRC_REQUEST="$ARTIFACT_DIR/bad-crc-request.bin"
INITIAL_IMAGE="$ARTIFACT_DIR/initial.raw"
PERSISTENT_IMAGE="$ARTIFACT_DIR/packages.raw"
GENERATION1_IMAGE="$ARTIFACT_DIR/generation1.raw"
GENERATION2_IMAGE="$ARTIFACT_DIR/generation2.raw"
GENERATION3_IMAGE="$ARTIFACT_DIR/generation3-removed.raw"
GENERATION4_IMAGE="$ARTIFACT_DIR/generation4-reinstalled.raw"
WRONG_TARGET_IMAGE="$ARTIFACT_DIR/wrong-target-negative.raw"
STALE_GENERATION_IMAGE="$ARTIFACT_DIR/stale-generation-negative.raw"
BAD_CRC_IMAGE="$ARTIFACT_DIR/bad-crc-negative.raw"
CONFLICT_IMAGE="$ARTIFACT_DIR/conflict-negative.raw"
ROLLBACK_IMAGE="$ARTIFACT_DIR/rollback-negative.raw"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_BUILD_LOG="$ARTIFACT_DIR/storage-build.log"
UNINSTALL_OPERATION_ID=7001
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
    tail -n 260 "$BOOT_NORMALIZED_LOG" >&2
  elif [[ -f "$BOOT_SERIAL_LOG" ]]; then
    tr -d '\r' <"$BOOT_SERIAL_LOG" | tail -n 260 >&2
  fi
  if [[ -f "$BOOT_QEMU_LOG" ]]; then
    tail -n 120 "$BOOT_QEMU_LOG" >&2
  fi
}

fail_gate() {
  show_failure
  echo "$1" >&2
  echo "Uninstall-0 evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

cp "$BASE_APK_PATH" "$BASE_SOURCE_APK"
cp "$UPDATE_APK_PATH" "$UPDATE_SOURCE_APK"

# Official SDK tools derive all source identities. The ZIP central directory
# and aapt2 output also derive the exact Resources-1 Activity evidence expected
# after durable readback.
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
    raise SystemExit("Activity name is outside the admitted AndroidBox contract")
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
  echo "Uninstall-0 base must have versionCode 2; observed $BASE_VERSION_CODE." >&2
  exit 2
}
[[ "$UPDATE_VERSION_CODE" == "3" ]] || {
  echo "Uninstall-0 update must have versionCode 3; observed $UPDATE_VERSION_CODE." >&2
  exit 2
}
[[ "$BASE_PACKAGE" == "$UPDATE_PACKAGE" ]] || {
  echo "Update and reinstall require the same package identity." >&2
  exit 2
}
[[ "$BASE_ACTIVITY" == "$UPDATE_ACTIVITY" ]] || {
  echo "The bounded fixture unexpectedly changed its admitted Activity." >&2
  exit 2
}
[[ "$BASE_CERT_SHA256" == "$UPDATE_CERT_SHA256" ]] || {
  echo "Update and reinstall require the same v2 signer certificate." >&2
  exit 2
}
[[ "$BASE_APK_SHA256" != "$UPDATE_APK_SHA256" ]] || {
  echo "Base and update APK content identities must differ." >&2
  exit 2
}
[[ "$BASE_TEXT" != "$UPDATE_TEXT" ]] || {
  echo "The version-3 fixture must visibly change the Activity TextView." >&2
  exit 2
}

# Build a canonical BNDUNS01 request and three target/parser negatives.
python3 - \
  "$UNINSTALL_REQUEST" \
  "$WRONG_TARGET_REQUEST" \
  "$STALE_GENERATION_REQUEST" \
  "$BAD_CRC_REQUEST" \
  "$UNINSTALL_OPERATION_ID" \
  "$UPDATE_PACKAGE" \
  "$UPDATE_VERSION_CODE" \
  "$UPDATE_APK_BYTES" \
  "$UPDATE_APK_SHA256" \
  "$UPDATE_CERT_SHA256" \
  "$ARTIFACT_DIR/uninstall-request-layout.txt" <<'PY'
from pathlib import Path
import struct
import sys
import zlib

canonical_path = Path(sys.argv[1])
wrong_path = Path(sys.argv[2])
stale_path = Path(sys.argv[3])
bad_crc_path = Path(sys.argv[4])
operation_id = int(sys.argv[5], 10)
package = sys.argv[6]
version_code = int(sys.argv[7], 10)
apk_length = int(sys.argv[8], 10)
apk_digest = bytes.fromhex(sys.argv[9])
signer_digest = bytes.fromhex(sys.argv[10])
layout_path = Path(sys.argv[11])

def build(*, target_package: str, generation: int) -> bytes:
    package_bytes = target_package.encode("ascii")
    if not 1 <= len(package_bytes) <= 96:
        raise SystemExit("request package is outside the 1..96 byte bound")
    result = bytearray(256)
    result[0:8] = b"BNDUNS01"
    struct.pack_into("<I", result, 8, 1)
    struct.pack_into("<I", result, 12, 0)
    struct.pack_into("<Q", result, 16, operation_id)
    struct.pack_into("<Q", result, 24, generation)
    struct.pack_into("<Q", result, 32, version_code)
    struct.pack_into("<I", result, 40, apk_length)
    struct.pack_into("<H", result, 44, len(package_bytes))
    struct.pack_into("<H", result, 46, 1)
    result[48:80] = apk_digest
    result[80:112] = signer_digest
    result[112:112 + len(package_bytes)] = package_bytes
    struct.pack_into("<I", result, 252, zlib.crc32(result[:252]) & 0xffffffff)
    return bytes(result)

canonical = build(target_package=package, generation=2)
wrong_first = "x" if package[0] != "x" else "y"
wrong = build(target_package=wrong_first + package[1:], generation=2)
stale = build(target_package=package, generation=1)
bad_crc = bytearray(canonical)
bad_crc[252] ^= 0x01

canonical_path.write_bytes(canonical)
wrong_path.write_bytes(wrong)
stale_path.write_bytes(stale)
bad_crc_path.write_bytes(bad_crc)
layout_path.write_text(
    "\n".join(
        (
            "wire=BNDUNS01",
            "wire_bytes=256",
            "byte_order=little-endian",
            "crc32_range=0..251",
            f"operation_id={operation_id}",
            "expected_generation=2",
            f"expected_version_code={version_code}",
            f"expected_apk_length={apk_length}",
            f"package={package}",
            f"apk_sha256={apk_digest.hex()}",
            f"signer_sha256={signer_digest.hex()}",
            "data_disposition=no-managed-package-data",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

for request in \
  "$UNINSTALL_REQUEST" \
  "$WRONG_TARGET_REQUEST" \
  "$STALE_GENERATION_REQUEST" \
  "$BAD_CRC_REQUEST"; do
  [[ "$(wc -c <"$request" | tr -d '[:space:]')" == "256" ]] || {
    echo "Generated uninstall request is not exactly 256 bytes: $request" >&2
    exit 1
  }
done
UNINSTALL_REQUEST_SHA256="$(
  shasum -a 256 "$UNINSTALL_REQUEST" | awk '{print $1}'
)"
WRONG_TARGET_REQUEST_SHA256="$(
  shasum -a 256 "$WRONG_TARGET_REQUEST" | awk '{print $1}'
)"
STALE_GENERATION_REQUEST_SHA256="$(
  shasum -a 256 "$STALE_GENERATION_REQUEST" | awk '{print $1}'
)"
BAD_CRC_REQUEST_SHA256="$(
  shasum -a 256 "$BAD_CRC_REQUEST" | awk '{print $1}'
)"

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  "$SCRIPT_DIR/build-kernel.sh" >"$BUILD_LOG" 2>&1; then
  tail -n 240 "$BUILD_LOG" >&2
  echo "Uninstall-0 kernel/userspace build failed." >&2
  exit 1
fi
if ! BNDROID_STORAGE_IMAGE="$INITIAL_IMAGE" \
  "$SCRIPT_DIR/build-storage-image.sh" --with-package-store \
  >"$STORAGE_BUILD_LOG" 2>&1; then
  tail -n 160 "$STORAGE_BUILD_LOG" >&2
  echo "The deterministic 16 MiB package-store image build failed." >&2
  exit 1
fi
[[ -f "$KERNEL_IMAGE" ]] || {
  echo "Uninstall-0 build did not produce the expected kernel image." >&2
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

# This is the gate's sole literal emulator launch. Every boot uses one explicit
# raw disk. At most one APK and one canonical request are passed through fw_cfg.
run_qemu_boot() {
  local name="$1"
  local disk="$2"
  local source_apk="$3"
  local uninstall_request="$4"
  local completion_pattern="$5"
  local completion_description="$6"
  # Keep the array non-empty for macOS Bash 3.2 with `set -u`.
  local -a source_args=(-name "Bndroid Uninstall-0 Gate $name")
  if [[ -n "$source_apk" ]]; then
    source_args+=(-fw_cfg "name=opt/bndroid/apk,file=$source_apk")
  fi
  if [[ -n "$uninstall_request" ]]; then
    source_args+=(
      -fw_cfg "name=opt/bndroid/package-uninstall,file=$uninstall_request"
    )
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
    -rtc base=2026-07-30T09:41:00,clock=vm \
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

ZERO_SHA256=0000000000000000000000000000000000000000000000000000000000000000

validate_input_markers() {
  local log="$1"
  local kind="$2"
  local apk_bytes=0
  local apk_digest="$ZERO_SHA256"
  local uninstall_digest="$ZERO_SHA256"
  local apk_present=0
  local uninstall_present=0
  local apk_selector=0x0000
  local uninstall_selector=0x0000
  local directory_files=11
  local dma_ops=2
  case "$kind" in
    none)
      ;;
    apk)
      apk_present=1
      apk_bytes="$3"
      apk_digest="$4"
      apk_selector=0x002a
      directory_files=12
      dma_ops=3
      ;;
    uninstall)
      uninstall_present=1
      uninstall_digest="$3"
      uninstall_selector=0x002a
      directory_files=12
      dma_ops=3
      ;;
    *)
      fail_gate "Internal input-marker kind is invalid: $kind"
      ;;
  esac
  require_once_fixed \
    "$log" \
    "APK_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/apk present=$apk_present bytes=$apk_bytes selector=$apk_selector directory_files=$directory_files dma_ops=$dma_ops sha256=$apk_digest explicit_source=1 host_directory_scan=0 network=disabled install_mutation=0" \
    "APK source marker"
  require_once_fixed \
    "$log" \
    "APK_UNINSTALL_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/package-uninstall present=$uninstall_present bytes=$((uninstall_present * 256)) selector=$uninstall_selector directory_files=$directory_files dma_ops=$dma_ops sha256=$uninstall_digest wire=BNDUNS01 wire_bytes=256 canonical_validation=1 explicit_source=1 host_directory_scan=0 network=disabled package_mutation=0" \
    "uninstall source marker"
}

PROFILE_MARKER='ANDROIDBOX_APK_INSTALL0_PROFILE_OK format=3 apk_source=qemu-fw_cfg-or-package-store signature=apk-v2-single-signer package_store=single-package-crash-consistent-stateful launch_source=boot-and-click-durable-readback update0=same-package-same-signer-monotonic-version uninstall0=canonical-host-request-identical-dual-tombstone reinstall0=retained-package-signer-no-version-rollback atomic_old-new-switch=1 atomic_installed-removed-switch=1 exact-source-replay=idempotent-zero-write exact-uninstall-replay=idempotent-zero-write rollback-source=reject-before-write app_data_policy=no-managed-package-data apk_blob_erased=0 old_kernel_downgrade_safe=0 snapshot_syscall=59 snapshot_wire=BNDAPS01 snapshot_bytes=640 relaunch_syscall=60 relaunch_request_wire=BNDARQ01 relaunch_response_wire=BNDAPS01 relaunch_mode=async-exact-retry relaunch_owner=launcher-generation relaunch_writes=0 ui_catalog=kernel-supplied ui_installed_launch=fresh-durable-reexecution launcher_resolution=manifest-main-launcher activity_class_binding=exact-dex-descriptor fixed_dex_probe_required=0 activity_lifecycle=constructor-then-onCreate constructor_required=1 apk_bytes_exposed_to_el0=0 storage_authority_granted_to_el0=0 install_ui=0 uninstall_ui=0 update_ui=0 package_manager_api=0 art=0 dalvik=0 activitythread=0 framework=Resources-1-subset binder=0 bionic=0 jni=0 native_lib=0 permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled emulator_only=1 real_phone_claim=0'

validate_common_success() {
  local log="$1"
  local reads="$2"
  local writes="$3"
  local flushes="$4"
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
    "STORAGE_LIMITS writes=$mutation partitions=4 filesystems=1 vfs=1 persistence=package-store-only flush=1 package_store=single-package-stateful package_states=empty-installed-removed apk_max_bytes=65024 appdata_mounted=0 app_data_policy=no-managed-package-data data_persistence_advanced=0 el0_package_storage=0 apk_blob_erase=0 crash_consistency=double-registry-double-blob+mirrored-tombstone host_powercut_claim=0 physical_powerloss_claim=0 general_runtime=0" \
    "storage boundary marker"
  require_once_fixed "$log" "$PROFILE_MARKER" "Uninstall-0 profile marker"
  require_once_ere \
    "$log" \
    '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=44 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 .* network=disabled .* real_phone_claim=0$' \
    "720x1600 mobile UI marker"
  if grep -Eq '^STORAGE_FAIL |^boot error:' "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "A valid Uninstall-0 state boot emitted a storage or boot failure."
  fi
}

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
  local reinstall="$2"
  local title="$3"
  local text="$4"
  local resources_crc="$5"
  local layout_crc="$6"
  local layout_id="$7"
  local text_id="$8"
  local mutation=0
  if ((writes != 0)); then
    mutation=1
  fi

  validate_common_success "$log" "$reads" "$writes" "$flushes"
  require_once_fixed \
    "$log" \
    "APK_PACKAGE_STORE_OK format=3 formatted_this_boot=$formatted source_present=$source_present source_admitted=$source_present uninstall_request_present=0 uninstall_request_used=0 operation=$operation previous_generation=$previous_generation previous_version_code=$previous_version installed=1 removed=0 generation=$generation slot=$slot apk_bytes=$apk_bytes version_code=$version package=$package activity=$activity profile=Resources-1 registry_blob_bound=1 full_readback=1 durable_reverification=1 mutation_performed=$mutation reads=$reads writes=$writes flushes=$flushes source_free_zero_writes=$source_free source_replay_zero_writes=$source_replay reinstall_from_tombstone=$reinstall apk_sha256=$apk_sha256 signer_cert_sha256=$cert_sha256 network=disabled el0_package_write=0 general_android_compatibility=0" \
    "installed package decision marker"
  require_once_ere \
    "$log" \
    "^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=$title text=$text constructor=1 constructor_method=[0-9]+ constructor_code_offset=[0-9]+ constructor_instructions=2 on_create=1 on_create_method=[0-9]+ on_create_code_offset=[0-9]+ on_create_instructions=4 resources_arsc_crc=$resources_crc layout_xml_crc=$layout_crc layout_resource_id=$layout_id string_resource_id=$text_id set_content_view_int=1 apk_v2=1 signer_count=1 art=0 dalvik=0 binder=0 jni=0 native_lib=0 framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$" \
    "durable Resources-1 Activity marker"
  if grep -Eq '^APK_PACKAGE_STORE_REMOVED_OK |^APK_PACKAGE_STORE_EMPTY_OK ' "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Installed boot also published a removed or empty state."
  fi
}

validate_removed_boot() {
  local log="$1"
  local request_present="$2"
  local request_used="$3"
  local operation="$4"
  local previous_generation="$5"
  local previous_version="$6"
  local removal_generation="$7"
  local last_generation="$8"
  local last_version="$9"
  shift 9
  local last_apk_bytes="$1"
  local last_blob_slot="$2"
  local package="$3"
  local reads="$4"
  local writes="$5"
  local flushes="$6"
  local source_free="$7"
  local replay_zero="$8"
  local mutation=0
  if ((writes != 0)); then
    mutation=1
  fi

  validate_common_success "$log" "$reads" "$writes" "$flushes"
  require_once_fixed \
    "$log" \
    "APK_PACKAGE_STORE_REMOVED_OK format=3 formatted_this_boot=0 source_present=0 source_admitted=0 uninstall_request_present=$request_present uninstall_request_used=$request_used operation=$operation previous_generation=$previous_generation previous_version_code=$previous_version installed=0 removed=1 removal_generation=$removal_generation last_generation=$last_generation last_version_code=$last_version last_apk_bytes=$last_apk_bytes last_blob_slot=$last_blob_slot package=$package data_disposition=no-managed-package-data tombstone_policy=identical-dual-registry logical_apk_reachable=0 apk_blob_erased=0 mutation_performed=$mutation reads=$reads writes=$writes flushes=$flushes source_free_zero_writes=$source_free uninstall_replay_zero_writes=$replay_zero old_kernel_downgrade_safe=0 network=disabled el0_package_write=0 general_android_compatibility=0" \
    "removed package decision marker"
  if grep -Eq \
    '^APK_PACKAGE_STORE_OK |^APK_PACKAGE_STORE_EMPTY_OK |^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
    "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Removed boot exposed an installed package, Activity, or empty state."
  fi
}

reject_success_on_negative() {
  local log="$1"
  if grep -Eq \
    '^APK_PACKAGE_STORE_OK |^APK_PACKAGE_STORE_REMOVED_OK |^APK_PACKAGE_STORE_EMPTY_OK |^ANDROIDBOX_INSTALLED_ACTIVITY_OK |^BLOCK_LAYER_OK |^STORAGE_LIMITS |^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
    "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "A rejected package input emitted post-transaction success evidence."
  fi
}

disk_sha256() {
  shasum -a 256 "$1" | awk '{print $1}'
}

assert_disk_unchanged() {
  local before="$1"
  local after="$2"
  local description="$3"
  [[ "$before" == "$after" ]] || {
    fail_gate "$description changed the complete disk image."
  }
}

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

# Fresh -> base v2 -> generation 1 / slot 0.
run_qemu_boot \
  base-install \
  "$PERSISTENT_IMAGE" \
  "$BASE_SOURCE_APK" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the generation-1 base installation"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" apk "$BASE_APK_BYTES" "$BASE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 1 install 0 0 1 0 \
  "$BASE_APK_BYTES" "$BASE_VERSION_CODE" "$BASE_PACKAGE" "$BASE_ACTIVITY" \
  "$BASE_APK_SHA256" "$BASE_CERT_SHA256" \
  1032 130 3 0 0 0 \
  "$BASE_TITLE" "$BASE_TEXT" "$BASE_RESOURCES_CRC" "$BASE_LAYOUT_CRC" \
  "$BASE_LAYOUT_ID" "$BASE_TEXT_ID"
cp "$PERSISTENT_IMAGE" "$GENERATION1_IMAGE"
GENERATION1_SHA256="$(disk_sha256 "$GENERATION1_IMAGE")"
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
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the generation-2 package update"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" apk "$UPDATE_APK_BYTES" "$UPDATE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 0 update 1 "$BASE_VERSION_CODE" 2 1 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  905 129 2 0 0 0 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
cp "$PERSISTENT_IMAGE" "$GENERATION2_IMAGE"
GENERATION2_SHA256="$(disk_sha256 "$GENERATION2_IMAGE")"
[[ "$GENERATION2_SHA256" != "$GENERATION1_SHA256" ]] || {
  fail_gate "Version-3 update did not persist generation 2."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/update.serial.normalized.log" \
  >"$ARTIFACT_DIR/update.activity"
if cmp -s "$ARTIFACT_DIR/base.activity" "$ARTIFACT_DIR/update.activity"; then
  fail_gate "Durable Activity evidence did not change from v2 to v3."
fi

# generation 2 installed -> generation 3 removed. The package blob remains
# physically present but both registry sectors must become one byte-identical
# higher-generation BNDPRM01 tombstone.
run_qemu_boot \
  uninstall \
  "$PERSISTENT_IMAGE" \
  "" \
  "$UNINSTALL_REQUEST" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the generation-3 mirrored package tombstone"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" uninstall "$UNINSTALL_REQUEST_SHA256"
validate_removed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 1 uninstall 2 "$UPDATE_VERSION_CODE" 3 2 "$UPDATE_VERSION_CODE" \
  "$UPDATE_APK_BYTES" 1 "$UPDATE_PACKAGE" \
  520 2 2 0 0
cp "$PERSISTENT_IMAGE" "$GENERATION3_IMAGE"
GENERATION3_SHA256="$(disk_sha256 "$GENERATION3_IMAGE")"
[[ "$GENERATION3_SHA256" != "$GENERATION2_SHA256" ]] || {
  fail_gate "Uninstall did not persist generation 3."
}

python3 - \
  "$GENERATION2_IMAGE" \
  "$GENERATION3_IMAGE" \
  "$UNINSTALL_OPERATION_ID" \
  "$UPDATE_PACKAGE" \
  "$UPDATE_VERSION_CODE" \
  "$UPDATE_APK_BYTES" \
  "$UPDATE_APK_SHA256" \
  "$UPDATE_CERT_SHA256" \
  "$ARTIFACT_DIR/tombstone-evidence.txt" <<'PY'
from pathlib import Path
import struct
import sys
import zlib

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
operation_id = int(sys.argv[3], 10)
package = sys.argv[4].encode("ascii")
version = int(sys.argv[5], 10)
apk_length = int(sys.argv[6], 10)
apk_digest = bytes.fromhex(sys.argv[7])
signer_digest = bytes.fromhex(sys.argv[8])
output = Path(sys.argv[9])

if len(before) != 16 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit("tombstone inspection requires exact 16 MiB images")
partition_lba = 16384
registry_lbas = (partition_lba + 1, partition_lba + 2)
first = after[registry_lbas[0] * 512:(registry_lbas[0] + 1) * 512]
second = after[registry_lbas[1] * 512:(registry_lbas[1] + 1) * 512]
if len(first) != 512 or len(second) != 512 or first != second:
    raise SystemExit("registry sectors are not two byte-identical tombstones")
if first[:8] != b"BNDPRM01":
    raise SystemExit("removed registry does not use the independent BNDPRM01 magic")
if struct.unpack_from("<I", first, 8)[0] != 1:
    raise SystemExit("tombstone format version is not 1")
if struct.unpack_from("<I", first, 12)[0] != 1:
    raise SystemExit("tombstone committed flag is not 1")
expected_epoch = bytes.fromhex("c73542916bec4add950e50b5e2b12301")
if first[16:32] != expected_epoch:
    raise SystemExit("tombstone epoch differs from the package-store epoch")
fields = {
    "generation": struct.unpack_from("<Q", first, 32)[0],
    "operation_id": struct.unpack_from("<Q", first, 40)[0],
    "last_generation": struct.unpack_from("<Q", first, 48)[0],
    "version": struct.unpack_from("<Q", first, 56)[0],
    "apk_length": struct.unpack_from("<I", first, 64)[0],
    "disposition": struct.unpack_from("<H", first, 68)[0],
    "last_slot": first[70],
    "package_length": first[71],
}
expected_fields = {
    "generation": 3,
    "operation_id": operation_id,
    "last_generation": 2,
    "version": version,
    "apk_length": apk_length,
    "disposition": 1,
    "last_slot": 1,
    "package_length": len(package),
}
if fields != expected_fields:
    raise SystemExit(f"tombstone scalar mismatch: {fields!r}")
if first[72:104] != apk_digest:
    raise SystemExit("tombstone APK digest mismatch")
if first[104:136] != signer_digest:
    raise SystemExit("tombstone signer digest mismatch")
if first[136:136 + len(package)] != package:
    raise SystemExit("tombstone package mismatch")
if any(first[136 + len(package):232]):
    raise SystemExit("tombstone package padding is nonzero")
if any(first[371:508]):
    raise SystemExit("tombstone reserved range is nonzero")
stored_crc = struct.unpack_from("<I", first, 508)[0]
computed_crc = zlib.crc32(first[:508]) & 0xffffffff
if stored_crc != computed_crc:
    raise SystemExit("tombstone CRC32 mismatch")

changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
allowed = set()
for lba in registry_lbas:
    allowed.update(range(lba * 512, (lba + 1) * 512))
outside = [index for index in changed if index not in allowed]
if not changed or outside:
    raise SystemExit("uninstall changed bytes outside the two registry sectors")
for lba in registry_lbas:
    if not any(lba * 512 <= index < (lba + 1) * 512 for index in changed):
        raise SystemExit(f"uninstall did not change registry LBA {lba}")

blob_ranges = (
    ((partition_lba + 16) * 512, (partition_lba + 16 + 128) * 512),
    ((partition_lba + 144) * 512, (partition_lba + 144 + 128) * 512),
)
if any(before[start:end] != after[start:end] for start, end in blob_ranges):
    raise SystemExit("Uninstall-0 changed or erased an APK blob slot")

output.write_text(
    "\n".join(
        (
            "magic=BNDPRM01",
            "format=1",
            "committed=1",
            "registry_copies=2",
            "registry_bytes_identical=1",
            "generation=3",
            "last_generation=2",
            f"operation_id={operation_id}",
            f"version_code={version}",
            f"apk_length={apk_length}",
            "last_blob_slot=1",
            f"package={package.decode('ascii')}",
            f"apk_sha256={apk_digest.hex()}",
            f"signer_sha256={signer_digest.hex()}",
            f"crc32={stored_crc}",
            f"changed_bytes={len(changed)}",
            "changed_only_registry_lbas=16385,16386",
            "apk_blob_bytes_unchanged=1",
            "logical_apk_reachable=0",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

# Save independent generation-3 images for every rejected input.
for negative_image in \
  "$WRONG_TARGET_IMAGE" \
  "$STALE_GENERATION_IMAGE" \
  "$BAD_CRC_IMAGE" \
  "$CONFLICT_IMAGE" \
  "$ROLLBACK_IMAGE"; do
  cp "$GENERATION3_IMAGE" "$negative_image"
done

# Replaying the exact request against both complete tombstones is zero-write,
# zero-flush and byte-for-byte disk stable.
UNINSTALL_REPLAY_BEFORE_SHA256="$(disk_sha256 "$PERSISTENT_IMAGE")"
run_qemu_boot \
  uninstall-replay \
  "$PERSISTENT_IMAGE" \
  "" \
  "$UNINSTALL_REQUEST" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the exact zero-write uninstall replay"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" uninstall "$UNINSTALL_REQUEST_SHA256"
validate_removed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 1 uninstall-replay 3 "$UPDATE_VERSION_CODE" 3 2 "$UPDATE_VERSION_CODE" \
  "$UPDATE_APK_BYTES" 1 "$UPDATE_PACKAGE" \
  6 0 0 0 1
UNINSTALL_REPLAY_AFTER_SHA256="$(disk_sha256 "$PERSISTENT_IMAGE")"
assert_disk_unchanged \
  "$UNINSTALL_REPLAY_BEFORE_SHA256" \
  "$UNINSTALL_REPLAY_AFTER_SHA256" \
  "Exact uninstall replay"

# No source and no request recovers Removed without exposing an Activity or
# touching either registry.
REMOVED_RECOVERY_BEFORE_SHA256="$UNINSTALL_REPLAY_AFTER_SHA256"
run_qemu_boot \
  removed-recovery \
  "$PERSISTENT_IMAGE" \
  "" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the source-free Removed recovery"
validate_input_markers "$BOOT_NORMALIZED_LOG" none
validate_removed_boot \
  "$BOOT_NORMALIZED_LOG" \
  0 0 removed-recovery 3 "$UPDATE_VERSION_CODE" 3 2 "$UPDATE_VERSION_CODE" \
  "$UPDATE_APK_BYTES" 1 "$UPDATE_PACKAGE" \
  3 0 0 1 0
REMOVED_RECOVERY_AFTER_SHA256="$(disk_sha256 "$PERSISTENT_IMAGE")"
assert_disk_unchanged \
  "$REMOVED_RECOVERY_BEFORE_SHA256" \
  "$REMOVED_RECOVERY_AFTER_SHA256" \
  "Source-free Removed recovery"

# A printable but different package target is canonical wire data, yet cannot
# authorize removal of this durable identity.
WRONG_TARGET_BEFORE_SHA256="$(disk_sha256 "$WRONG_TARGET_IMAGE")"
run_qemu_boot \
  wrong-target-negative \
  "$WRONG_TARGET_IMAGE" \
  "" \
  "$WRONG_TARGET_REQUEST" \
  '^boot error: storage validation failed: package-uninstall request does not match durable package state$' \
  "the wrong uninstall target rejection"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" uninstall "$WRONG_TARGET_REQUEST_SHA256"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'STORAGE_FAIL reason=package_manager_invalid' \
  "wrong-target package-manager rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: storage validation failed: package-uninstall request does not match durable package state' \
  "wrong-target boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
WRONG_TARGET_AFTER_SHA256="$(disk_sha256 "$WRONG_TARGET_IMAGE")"
assert_disk_unchanged \
  "$WRONG_TARGET_BEFORE_SHA256" \
  "$WRONG_TARGET_AFTER_SHA256" \
  "Rejected wrong-target request"

# The same package/digests with stale generation 1 cannot target the retained
# generation-2 identity recorded by removal generation 3.
STALE_GENERATION_BEFORE_SHA256="$(disk_sha256 "$STALE_GENERATION_IMAGE")"
run_qemu_boot \
  stale-generation-negative \
  "$STALE_GENERATION_IMAGE" \
  "" \
  "$STALE_GENERATION_REQUEST" \
  '^boot error: storage validation failed: package-uninstall request does not match durable package state$' \
  "the stale expected-generation rejection"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" uninstall "$STALE_GENERATION_REQUEST_SHA256"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'STORAGE_FAIL reason=package_manager_invalid' \
  "stale-generation package-manager rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: storage validation failed: package-uninstall request does not match durable package state' \
  "stale-generation boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
STALE_GENERATION_AFTER_SHA256="$(disk_sha256 "$STALE_GENERATION_IMAGE")"
assert_disk_unchanged \
  "$STALE_GENERATION_BEFORE_SHA256" \
  "$STALE_GENERATION_AFTER_SHA256" \
  "Rejected stale-generation request"

# CRC rejection occurs during fw_cfg source admission, before package storage
# is initialized or any mutation-capable package transaction exists.
BAD_CRC_BEFORE_SHA256="$(disk_sha256 "$BAD_CRC_IMAGE")"
run_qemu_boot \
  bad-crc-negative \
  "$BAD_CRC_IMAGE" \
  "" \
  "$BAD_CRC_REQUEST" \
  '^boot error: APK source init failed: package uninstall request CRC32 is invalid$' \
  "the uninstall-request CRC32 rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: APK source init failed: package uninstall request CRC32 is invalid' \
  "bad-CRC source rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
BAD_CRC_AFTER_SHA256="$(disk_sha256 "$BAD_CRC_IMAGE")"
assert_disk_unchanged \
  "$BAD_CRC_BEFORE_SHA256" \
  "$BAD_CRC_AFTER_SHA256" \
  "Rejected bad-CRC request"

# APK plus request is ambiguous authority and must be rejected while the fw_cfg
# directory is still being admitted.
CONFLICT_BEFORE_SHA256="$(disk_sha256 "$CONFLICT_IMAGE")"
run_qemu_boot \
  apk-request-conflict-negative \
  "$CONFLICT_IMAGE" \
  "$UPDATE_SOURCE_APK" \
  "$UNINSTALL_REQUEST" \
  '^boot error: APK source init failed: fw_cfg APK and package-uninstall inputs are mutually exclusive$' \
  "the APK/request mutual-exclusion rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: APK source init failed: fw_cfg APK and package-uninstall inputs are mutually exclusive' \
  "APK/request conflict rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
CONFLICT_AFTER_SHA256="$(disk_sha256 "$CONFLICT_IMAGE")"
assert_disk_unchanged \
  "$CONFLICT_BEFORE_SHA256" \
  "$CONFLICT_AFTER_SHA256" \
  "Rejected APK/request conflict"

# Removal retains the last version and digest policy. Feeding v2 after removing
# v3 is therefore a rollback, not a fresh install.
ROLLBACK_BEFORE_SHA256="$(disk_sha256 "$ROLLBACK_IMAGE")"
run_qemu_boot \
  rollback-negative \
  "$ROLLBACK_IMAGE" \
  "$BASE_SOURCE_APK" \
  "" \
  '^boot error: storage validation failed: package update version must strictly increase$' \
  "the post-removal version-2 rollback rejection"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" apk "$BASE_APK_BYTES" "$BASE_APK_SHA256"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'STORAGE_FAIL reason=package_manager_invalid' \
  "post-removal rollback package-manager rejection"
require_once_fixed \
  "$BOOT_NORMALIZED_LOG" \
  'boot error: storage validation failed: package update version must strictly increase' \
  "post-removal rollback boot rejection"
reject_success_on_negative "$BOOT_NORMALIZED_LOG"
ROLLBACK_AFTER_SHA256="$(disk_sha256 "$ROLLBACK_IMAGE")"
assert_disk_unchanged \
  "$ROLLBACK_BEFORE_SHA256" \
  "$ROLLBACK_AFTER_SHA256" \
  "Rejected post-removal rollback"

# Reinstalling the exact retained v3 digest is the one same-version exception.
# It advances generation 3 -> 4, writes blob slot 0, and leaves the other
# generation-3 tombstone as the failure fallback.
run_qemu_boot \
  reinstall \
  "$PERSISTENT_IMAGE" \
  "$UPDATE_SOURCE_APK" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the bounded generation-4 version-3 reinstall"
validate_input_markers \
  "$BOOT_NORMALIZED_LOG" apk "$UPDATE_APK_BYTES" "$UPDATE_APK_SHA256"
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 0 reinstall 3 "$UPDATE_VERSION_CODE" 4 0 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  521 129 2 0 0 1 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
cp "$PERSISTENT_IMAGE" "$GENERATION4_IMAGE"
GENERATION4_SHA256="$(disk_sha256 "$GENERATION4_IMAGE")"
[[ "$GENERATION4_SHA256" != "$GENERATION3_SHA256" ]] || {
  fail_gate "Version-3 reinstall did not persist generation 4."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/reinstall.serial.normalized.log" \
  >"$ARTIFACT_DIR/reinstall.activity"
cmp "$ARTIFACT_DIR/update.activity" "$ARTIFACT_DIR/reinstall.activity"

# A final source-free boot must recover generation 4 and reproduce the exact
# durable Resources-1 Activity without mutation.
REINSTALL_RECOVERY_BEFORE_SHA256="$GENERATION4_SHA256"
run_qemu_boot \
  reinstall-recovery \
  "$PERSISTENT_IMAGE" \
  "" \
  "" \
  '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
  "the source-free generation-4 Activity recovery"
validate_input_markers "$BOOT_NORMALIZED_LOG" none
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  0 0 recovery 4 "$UPDATE_VERSION_CODE" 4 0 \
  "$UPDATE_APK_BYTES" "$UPDATE_VERSION_CODE" "$UPDATE_PACKAGE" "$UPDATE_ACTIVITY" \
  "$UPDATE_APK_SHA256" "$UPDATE_CERT_SHA256" \
  389 0 0 1 0 0 \
  "$UPDATE_TITLE" "$UPDATE_TEXT" "$UPDATE_RESOURCES_CRC" "$UPDATE_LAYOUT_CRC" \
  "$UPDATE_LAYOUT_ID" "$UPDATE_TEXT_ID"
REINSTALL_RECOVERY_AFTER_SHA256="$(disk_sha256 "$PERSISTENT_IMAGE")"
assert_disk_unchanged \
  "$REINSTALL_RECOVERY_BEFORE_SHA256" \
  "$REINSTALL_RECOVERY_AFTER_SHA256" \
  "Source-free generation-4 recovery"
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/reinstall-recovery.serial.normalized.log" \
  >"$ARTIFACT_DIR/reinstall-recovery.activity"
cmp "$ARTIFACT_DIR/reinstall.activity" "$ARTIFACT_DIR/reinstall-recovery.activity"

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
check_package_partition_diff \
  "$GENERATION3_IMAGE" \
  "$GENERATION4_IMAGE" \
  reinstall \
  "$ARTIFACT_DIR/reinstall.disk-diff.txt"

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
  "uninstall_operation_id=$UNINSTALL_OPERATION_ID" \
  "uninstall_request_sha256=$UNINSTALL_REQUEST_SHA256" \
  "initial_disk_sha256=$INITIAL_SHA256" \
  "generation1_disk_sha256=$GENERATION1_SHA256" \
  "generation2_disk_sha256=$GENERATION2_SHA256" \
  "generation3_removed_disk_sha256=$GENERATION3_SHA256" \
  "generation4_reinstalled_disk_sha256=$GENERATION4_SHA256" \
  "uninstall_replay_before_disk_sha256=$UNINSTALL_REPLAY_BEFORE_SHA256" \
  "uninstall_replay_after_disk_sha256=$UNINSTALL_REPLAY_AFTER_SHA256" \
  "removed_recovery_before_disk_sha256=$REMOVED_RECOVERY_BEFORE_SHA256" \
  "removed_recovery_after_disk_sha256=$REMOVED_RECOVERY_AFTER_SHA256" \
  "wrong_target_before_disk_sha256=$WRONG_TARGET_BEFORE_SHA256" \
  "wrong_target_after_disk_sha256=$WRONG_TARGET_AFTER_SHA256" \
  "stale_generation_before_disk_sha256=$STALE_GENERATION_BEFORE_SHA256" \
  "stale_generation_after_disk_sha256=$STALE_GENERATION_AFTER_SHA256" \
  "bad_crc_before_disk_sha256=$BAD_CRC_BEFORE_SHA256" \
  "bad_crc_after_disk_sha256=$BAD_CRC_AFTER_SHA256" \
  "conflict_before_disk_sha256=$CONFLICT_BEFORE_SHA256" \
  "conflict_after_disk_sha256=$CONFLICT_AFTER_SHA256" \
  "rollback_before_disk_sha256=$ROLLBACK_BEFORE_SHA256" \
  "rollback_after_disk_sha256=$ROLLBACK_AFTER_SHA256" \
  "reinstall_recovery_before_disk_sha256=$REINSTALL_RECOVERY_BEFORE_SHA256" \
  "reinstall_recovery_after_disk_sha256=$REINSTALL_RECOVERY_AFTER_SHA256" \
  'base_generation=1' \
  'update_generation=2' \
  'removal_generation=3' \
  'reinstall_generation=4' \
  'uninstall_reads=520' \
  'uninstall_registry_writes=2' \
  'uninstall_flushes=2' \
  'tombstone_copies=2' \
  'tombstone_bytes_identical=1' \
  'uninstall_replay_reads=6' \
  'uninstall_replay_writes=0' \
  'uninstall_replay_flushes=0' \
  'removed_recovery_reads=3' \
  'removed_recovery_writes=0' \
  'removed_recovery_flushes=0' \
  'wrong_target_disk_unchanged=1' \
  'stale_generation_disk_unchanged=1' \
  'bad_crc_disk_unchanged=1' \
  'apk_request_conflict_disk_unchanged=1' \
  'rollback_disk_unchanged=1' \
  'reinstall_reads=521' \
  'reinstall_writes=129' \
  'reinstall_flushes=2' \
  'reinstall_recovery_reads=389' \
  'reinstall_activity_persistent=1' \
  'network=disabled' \
  'trusted_host_boot_input_only=1' \
  'runtime_uninstall_ui=0' \
  'general_android_compatibility=0' \
  >"$ARTIFACT_DIR/summary.txt"

echo "ANDROIDBOX_APK_UNINSTALL0_QEMU_OK artifact_dir=$ARTIFACT_DIR base_apk_sha256=$BASE_APK_SHA256 update_apk_sha256=$UPDATE_APK_SHA256 signer_cert_sha256=$UPDATE_CERT_SHA256 install_generation=1 update_generation=2 removal_generation=3 reinstall_generation=4 tombstone_copies=2 tombstone_bytes_identical=1 uninstall_replay_writes=0 uninstall_replay_flushes=0 removed_recovery_writes=0 removed_recovery_flushes=0 wrong_target_disk_unchanged=1 stale_generation_disk_unchanged=1 bad_crc_disk_unchanged=1 apk_request_conflict_disk_unchanged=1 rollback_disk_unchanged=1 reinstall_activity_persistent=1 network=disabled general_android_compatibility=0"
