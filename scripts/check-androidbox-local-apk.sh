#!/usr/bin/env bash
set -euo pipefail

# Strict offline local-Mac APK -> Bndroid Resources-1 acceptance gate.
#
# The sole input is one explicit APK file. Host SDK tools and ZIP parsing
# independently derive and validate its bounded identity before QEMU sees it.
# The exact manifest launcher must exist in classes.dex, while the retired
# repository-specific Lorg/bndroid/demo/Main; probe class must be absent.
# A deterministic empty package store is then used for generation-1 install,
# one source-free recovery, an All apps launch, and one generation-bound
# Overview activation. Each entry freshly rereads and revalidates the durable
# blob without writing it. Home retains only identity; Back finishes it.
# This proves only component-driven AndroidBox Resources-1; it is not evidence
# of ART/Dalvik, Binder/Bionic/JNI, native libraries, broad Android Framework
# behavior, networking, hardware, or general APK support.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  printf '%s\n' \
    'Usage: check-androidbox-local-apk.sh --apk /absolute/path.apk' \
    '' \
    'Validates one explicit local v2-only, one-signer Resources-1 APK, then' \
    'installs, recovers, launches from All apps, and re-verifies an Overview' \
    'recent activation in offline local QEMU. Back must clear the recent. The' \
    'path must be absolute; the gate never scans a directory for APK files.'
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
    echo "--apk must be an absolute path: $APK_PATH" >&2
    exit 2
    ;;
esac
[[ -f "$APK_PATH" ]] || {
  echo "--apk must name one regular file: $APK_PATH" >&2
  exit 2
}

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep \
  awk cmp cp shasum wc sed /usr/bin/unzip; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot run the offline local-APK gate." >&2
    exit 1
  }
done

SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
APKSIGNER="${BNDROID_APKSIGNER:-"$SDK_ROOT/build-tools/36.1.0/apksigner"}"
AAPT2="${BNDROID_AAPT2:-"$SDK_ROOT/build-tools/36.1.0/aapt2"}"
DEXDUMP="${BNDROID_DEXDUMP:-"$SDK_ROOT/build-tools/36.1.0/dexdump"}"
for sdk_tool in "$APKSIGNER" "$AAPT2" "$DEXDUMP"; do
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

mkdir -p "$WORKSPACE_ROOT/target/androidbox-local-apk"
ARTIFACT_DIR="$(
  mktemp -d "$WORKSPACE_ROOT/target/androidbox-local-apk/check.XXXXXX"
)"
# Keep every helper's transient output inside the project evidence tree.
TMPDIR="$ARTIFACT_DIR/tmp"
mkdir -p "$TMPDIR"
export TMPDIR
TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-local-apk-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
SOURCE_APK="$ARTIFACT_DIR/source.apk"
INITIAL_IMAGE="$ARTIFACT_DIR/initial.raw"
PERSISTENT_IMAGE="$ARTIFACT_DIR/packages.raw"
INSTALLED_IMAGE="$ARTIFACT_DIR/generation1.raw"
SIGNER_LOG="$ARTIFACT_DIR/apksigner.txt"
BADGING_LOG="$ARTIFACT_DIR/badging.txt"
RESOURCES_LOG="$ARTIFACT_DIR/resources.txt"
LAYOUT_LOG="$ARTIFACT_DIR/layout.txt"
METADATA_LOG="$ARTIFACT_DIR/metadata.txt"
ZIP_LOG="$ARTIFACT_DIR/zip-evidence.txt"
DEX_FILE="$ARTIFACT_DIR/classes.dex"
DEX_LOG="$ARTIFACT_DIR/classes-dexdump.txt"
BUILD_LOG="$ARTIFACT_DIR/build.log"
STORAGE_BUILD_LOG="$ARTIFACT_DIR/storage-build.log"
QEMU_PID=""
BOOT_SERIAL_LOG=""
BOOT_NORMALIZED_LOG=""
BOOT_QEMU_LOG=""
BOOT_QMP_SOCKET=""

[[ -x "$QMP_HELPER" ]] || {
  echo "Mobile UI QMP helper is not executable: $QMP_HELPER" >&2
  exit 1
}

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
  echo "Local-APK evidence retained at: $ARTIFACT_DIR" >&2
  exit 1
}

cp "$APK_PATH" "$SOURCE_APK"
APK_BYTES="$(wc -c <"$SOURCE_APK" | tr -d '[:space:]')"
[[ "$APK_BYTES" =~ ^[0-9]+$ ]] || {
  echo "Could not determine the APK byte length." >&2
  exit 1
}
if ((APK_BYTES == 0 || APK_BYTES > 65024)); then
  echo "Resources-1 APK must contain 1..65024 bytes; observed $APK_BYTES." >&2
  exit 2
fi
APK_SHA256="$(shasum -a 256 "$SOURCE_APK" | awk '{print $1}')"
[[ "$APK_SHA256" =~ ^[0-9a-f]{64}$ ]] || {
  echo "Could not determine the exact APK SHA-256." >&2
  exit 1
}

# apksigner is the independent host admission oracle for the intentionally
# narrow APK Signature Scheme v2-only, one-signer profile.
if ! "$APKSIGNER" verify --verbose --print-certs "$SOURCE_APK" \
  >"$SIGNER_LOG" 2>&1; then
  tail -n 140 "$SIGNER_LOG" >&2
  echo "Official apksigner rejected the explicit APK." >&2
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
  [[ "$(grep -Fxc "$exact" "$SIGNER_LOG" || true)" == "1" ]] || {
    tail -n 140 "$SIGNER_LOG" >&2
    echo "APK is not the exact v2-only, one-signer contract." >&2
    exit 1
  }
done
CERT_SHA256="$(
  sed -n 's/^Signer #1 certificate SHA-256 digest: //p' "$SIGNER_LOG" \
    | tr '[:upper:]' '[:lower:]'
)"
[[ "$CERT_SHA256" =~ ^[0-9a-f]{64}$ ]] || {
  tail -n 140 "$SIGNER_LOG" >&2
  echo "Could not derive the one signer certificate SHA-256." >&2
  exit 1
}
[[ "$(grep -c '^Signer #1 certificate SHA-256 digest: ' "$SIGNER_LOG" || true)" == "1" ]] || {
  echo "apksigner emitted an ambiguous signer certificate digest." >&2
  exit 1
}

"$AAPT2" dump badging "$SOURCE_APK" >"$BADGING_LOG"
"$AAPT2" dump resources "$SOURCE_APK" >"$RESOURCES_LOG"
"$AAPT2" dump xmltree --file res/layout/activity_main.xml "$SOURCE_APK" \
  >"$LAYOUT_LOG"
/usr/bin/unzip -p "$SOURCE_APK" classes.dex >"$DEX_FILE"
"$DEXDUMP" -d "$DEX_FILE" >"$DEX_LOG"

# Parse aapt2 output and the ZIP independently. Besides deriving all expected
# runtime text and CRCs, this rejects profiles wider than Resources-1 before
# the first package-store write is possible.
python3 - \
  "$SOURCE_APK" \
  "$BADGING_LOG" \
  "$RESOURCES_LOG" \
  "$LAYOUT_LOG" \
  "$METADATA_LOG" \
  "$ZIP_LOG" <<'PY'
from pathlib import Path
import re
import struct
import sys
import zipfile
import zlib

apk_path = Path(sys.argv[1])
badging = Path(sys.argv[2]).read_text(encoding="utf-8")
resources = Path(sys.argv[3]).read_text(encoding="utf-8")
layout = Path(sys.argv[4]).read_text(encoding="utf-8")
metadata_output = Path(sys.argv[5])
zip_output = Path(sys.argv[6])

def one(pattern: str, value: str, description: str, flags: int = 0) -> re.Match[str]:
    matches = list(re.finditer(pattern, value, flags))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one {description}; observed {len(matches)}")
    return matches[0]

package_match = one(
    r"^package: name='([^']+)' versionCode='([0-9]+)'(?: versionName='([^']*)')?.*$",
    badging,
    "aapt2 package/version line",
    re.MULTILINE,
)
package = package_match.group(1)
version_code = int(package_match.group(2), 10)
version_name = package_match.group(3) or ""
if not 1 <= version_code <= (1 << 64) - 1:
    raise SystemExit("versionCode is outside the package-store u64 contract")
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+", package):
    raise SystemExit("package name is outside the bounded AndroidBox contract")
if len(package.encode("ascii")) > 96:
    raise SystemExit("package name exceeds the 96-byte package-store bound")

activity = one(
    r"^launchable-activity: name='([^']+)' .*$",
    badging,
    "launcher Activity",
    re.MULTILINE,
).group(1)
if activity.startswith("."):
    activity = package + activity
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+", activity):
    raise SystemExit("launcher Activity is outside the bounded AndroidBox contract")
if not activity.startswith(package + "."):
    raise SystemExit("launcher Activity is outside the admitted package")
descriptor = "L" + activity.replace(".", "/") + ";"
if len(descriptor.encode("ascii")) > 128:
    raise SystemExit("Activity descriptor exceeds the 128-byte package-store bound")

badging_title = one(
    r"^application-label:'([^']+)'$",
    badging,
    "application label",
    re.MULTILINE,
).group(1)
layout_match = one(
    r"^\s*resource (0x[0-9a-fA-F]+) layout/([^\s]+)\n"
    r"\s+\(\) \(file\) (res/[^\s]+) type=XML$",
    resources,
    "compiled layout resource",
    re.MULTILINE,
)
layout_id = int(layout_match.group(1), 16)
layout_name = layout_match.group(2)
layout_entry = layout_match.group(3)
if (layout_id, layout_name, layout_entry) != (
    0x7F020000,
    "activity_main",
    "res/layout/activity_main.xml",
):
    raise SystemExit("Resources-1 requires layout/activity_main at 0x7f020000")

app_match = one(
    r"^\s*resource (0x[0-9a-fA-F]+) string/app_name\n\s+\(\) \"([^\"]*)\"$",
    resources,
    "compiled string/app_name",
    re.MULTILINE,
)
app_name_id = int(app_match.group(1), 16)
app_name = app_match.group(2)
if app_name_id != 0x7F030001:
    raise SystemExit("Resources-1 requires string/app_name at 0x7f030001")
if badging_title != app_name:
    raise SystemExit("binary manifest application label does not resolve to string/app_name")

text_ref = one(
    r"android:text\(0x0101014f\)=@(0x[0-9a-fA-F]+)$",
    layout,
    "TextView text resource reference",
    re.MULTILINE,
)
text_id = int(text_ref.group(1), 16)
if text_id != 0x7F030000:
    raise SystemExit("Resources-1 requires the TextView string at 0x7f030000")
text_match = one(
    rf"^\s*resource 0x{text_id:08x} string/([^\s]+)\n\s+\(\) \"([^\"]*)\"$",
    resources,
    "compiled TextView string",
    re.MULTILINE | re.IGNORECASE,
)
text_name = text_match.group(1)
text = text_match.group(2)

element_lines = re.findall(r"^\s*E:\s+([^\s]+).*$", layout, re.MULTILINE)
if element_lines != ["TextView"]:
    raise SystemExit("Resources-1 requires exactly one root TextView and no children")
view_id = int(
    one(
        r"android:id\(0x010100d0\)=@(0x[0-9a-fA-F]+)$",
        layout,
        "TextView id",
        re.MULTILINE,
    ).group(1),
    16,
)
if view_id != 0x7F010000:
    raise SystemExit("Resources-1 requires the root TextView id 0x7f010000")
one(
    r"android:layout_width\(0x010100f4\)=-1$",
    layout,
    "match-parent TextView width",
    re.MULTILINE,
)
one(
    r"android:layout_height\(0x010100f5\)=-2$",
    layout,
    "wrap-content TextView height",
    re.MULTILINE,
)
attribute_lines = re.findall(r"^\s*A:\s+.*$", layout, re.MULTILINE)
if len(attribute_lines) != 4:
    raise SystemExit("Resources-1 root TextView must have exactly four attributes")

for name, value in (("app_name", app_name), ("TextView text", text)):
    if (
        not value
        or not value.isascii()
        or any(ord(char) < 0x20 or ord(char) > 0x7e for char in value)
    ):
        raise SystemExit(f"{name} is not one non-empty printable ASCII line")

expected_entries = (
    "AndroidManifest.xml",
    "resources.arsc",
    "res/layout/activity_main.xml",
    "classes.dex",
)
raw_apk = apk_path.read_bytes()
with zipfile.ZipFile(apk_path, "r") as archive:
    infos = archive.infolist()
    names = tuple(info.filename for info in infos)
    if names != expected_entries or len(set(names)) != len(names):
        raise SystemExit(
            "Resources-1 APK must contain exactly four canonical, ordered entries"
        )
    evidence = []
    for info in infos:
        if (
            info.is_dir()
            or info.compress_type != zipfile.ZIP_STORED
            or info.compress_size != info.file_size
            or info.flag_bits != 0
        ):
            raise SystemExit(f"ZIP entry is not canonical STORED data: {info.filename}")
        if raw_apk[info.header_offset:info.header_offset + 4] != b"PK\x03\x04":
            raise SystemExit(f"ZIP local header is invalid: {info.filename}")
        name_length, extra_length = struct.unpack_from(
            "<HH", raw_apk, info.header_offset + 26
        )
        data_offset = info.header_offset + 30 + name_length + extra_length
        if data_offset % 4 != 0:
            raise SystemExit(f"ZIP entry data is not 4-byte aligned: {info.filename}")
        data = archive.read(info.filename)
        actual_crc = zlib.crc32(data) & 0xFFFFFFFF
        if actual_crc != info.CRC:
            raise SystemExit(f"ZIP entry CRC mismatch: {info.filename}")
        evidence.append(
            f"entry={info.filename} method=STORED bytes={info.file_size} "
            f"crc32={info.CRC} data_offset={data_offset} aligned4=1"
        )
    resources_crc = archive.getinfo("resources.arsc").CRC
    layout_crc = archive.getinfo(layout_entry).CRC

metadata_output.write_text(
    "\n".join(
        (
            package,
            descriptor,
            str(version_code),
            version_name,
            app_name,
            text,
            str(resources_crc),
            str(layout_crc),
            str(layout_id),
            str(text_id),
            str(app_name_id),
            str(view_id),
            text_name,
            layout_entry,
        )
    )
    + "\n",
    encoding="utf-8",
)
zip_output.write_text(
    "\n".join(
        (
            "entries=4",
            "entry_order=AndroidManifest.xml,resources.arsc,res/layout/activity_main.xml,classes.dex",
            *evidence,
            f"resources_arsc_crc32={resources_crc}",
            f"layout_xml_crc32={layout_crc}",
        )
    )
    + "\n",
    encoding="utf-8",
)
PY

[[ "$(wc -l <"$METADATA_LOG" | tr -d '[:space:]')" == "14" ]] || {
  echo "Could not derive the exact Resources-1 metadata." >&2
  exit 1
}
PACKAGE="$(sed -n '1p' "$METADATA_LOG")"
ACTIVITY="$(sed -n '2p' "$METADATA_LOG")"
VERSION_CODE="$(sed -n '3p' "$METADATA_LOG")"
VERSION_NAME="$(sed -n '4p' "$METADATA_LOG")"
APP_NAME="$(sed -n '5p' "$METADATA_LOG")"
TEXT="$(sed -n '6p' "$METADATA_LOG")"
RESOURCES_CRC="$(sed -n '7p' "$METADATA_LOG")"
LAYOUT_CRC="$(sed -n '8p' "$METADATA_LOG")"
LAYOUT_ID="$(sed -n '9p' "$METADATA_LOG")"
TEXT_ID="$(sed -n '10p' "$METADATA_LOG")"
APP_NAME_ID="$(sed -n '11p' "$METADATA_LOG")"
VIEW_ID="$(sed -n '12p' "$METADATA_LOG")"
TEXT_RESOURCE_NAME="$(sed -n '13p' "$METADATA_LOG")"
LAYOUT_ENTRY="$(sed -n '14p' "$METADATA_LOG")"

[[ "$(grep -Fxc "  Class descriptor  : '$ACTIVITY'" "$DEX_LOG" || true)" == "1" ]] || {
  echo "classes.dex does not contain exactly one manifest-selected Activity class: $ACTIVITY" >&2
  exit 1
}
if grep -Fq "Class descriptor  : 'Lorg/bndroid/demo/Main;'" "$DEX_LOG"; then
  echo "The local APK still carries the retired fixed AndroidBox DEX-0 probe class." >&2
  exit 1
fi

[[ "$LAYOUT_ID" == "2130837504" ]] || {
  echo "Host parser did not retain fixed layout ID 0x7f020000." >&2
  exit 1
}
[[ "$TEXT_ID" == "2130903040" ]] || {
  echo "Host parser did not retain fixed TextView string ID 0x7f030000." >&2
  exit 1
}
[[ "$APP_NAME_ID" == "2130903041" ]] || {
  echo "Host parser did not retain fixed app_name ID 0x7f030001." >&2
  exit 1
}
[[ "$VIEW_ID" == "2130771968" ]] || {
  echo "Host parser did not retain fixed TextView ID 0x7f010000." >&2
  exit 1
}

if ! CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE=release \
  BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0,androidbox-apk-install0 \
  "$SCRIPT_DIR/build-kernel.sh" >"$BUILD_LOG" 2>&1; then
  tail -n 240 "$BUILD_LOG" >&2
  echo "Local-APK kernel/userspace build failed." >&2
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
  echo "Local-APK build did not produce the expected kernel image." >&2
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

boot_log_count() {
  local pattern="$1"
  normalize_boot_log
  grep -Ec "$pattern" "$BOOT_NORMALIZED_LOG" || true
}

wait_for_boot_count_at_least() {
  local pattern="$1"
  local minimum="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
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

run_qmp_action() {
  if ! "$QMP_HELPER" "$BOOT_QMP_SOCKET" "$@"; then
    fail_gate "QMP interaction failed: $*"
  fi
}

launcher_commit_count() {
  local launcher_pid="$1"
  boot_log_count \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} "
}

mobile_screenshot_matches() {
  local kind="$1"
  local screenshot="$2"
  python3 - "$kind" "$screenshot" <<'PY'
from pathlib import Path
import sys

kind = sys.argv[1]
path = Path(sys.argv[2])
try:
    payload = path.read_bytes().split(b"\n", 3)
except OSError:
    raise SystemExit(1)
if len(payload) != 4 or payload[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = payload[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)

def pixel(x: int, y: int) -> tuple[int, int, int]:
    offset = (y * 720 + x) * 3
    return tuple(pixels[offset : offset + 3])

expected_by_kind = {
    "drawer-installed": {
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
    "overview-compatible": {
        (360, 300): (13, 22, 41),
        (360, 580): (16, 118, 111),
        (719, 1599): (0, 0, 0),
    },
    "overview-empty": {
        (360, 300): (13, 22, 41),
        (360, 580): (24, 34, 56),
        (719, 1599): (0, 0, 0),
    },
}
expected = expected_by_kind.get(kind)
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
minimum_colors = 64 if kind == "activity" else 24
if len({pixels[offset : offset + 3] for offset in range(0, len(pixels), 3)}) < minimum_colors:
    raise SystemExit(1)
PY
}

wait_for_mobile_screenshot() {
  local kind="$1"
  local screenshot="$2"
  local description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  # QEMU's screendump includes the tablet cursor. Park the already released
  # pointer at one fixed, non-interactive coordinate so byte-for-byte raster
  # comparisons measure the guest frame rather than the preceding tap site.
  run_qmp_action move 700 1500
  while ((SECONDS < deadline)); do
    reject_panic_or_fatal
    run_qmp_action screenshot "$screenshot"
    if mobile_screenshot_matches "$kind" "$screenshot"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      fail_gate "QEMU exited while waiting for $description."
    fi
    sleep 0.05
  done
  fail_gate "Timed out waiting for $description."
}

wait_for_compatible_overview() {
  local launcher_pid="$1"
  local screenshot="$2"
  local before

  before="$(launcher_commit_count "$launcher_pid")"
  run_qmp_action touch-down 360 1570
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "compatible-Activity Overview gesture capture"
  run_qmp_action touch-move 360 1330
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=1 nav_reveal_px=240 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "compatible-Activity Overview finger-follow state"
  run_qmp_action touch-up
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "stable compatible-Activity Overview"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit for compatible-Activity Overview"
  wait_for_mobile_screenshot \
    overview-compatible \
    "$screenshot" \
    "the stable compatible-Activity identity card"
}

wait_for_empty_overview() {
  local launcher_pid="$1"
  local screenshot="$2"
  local before

  before="$(launcher_commit_count "$launcher_pid")"
  run_qmp_action touch-down 360 1570
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=1 nav_reveal_px=0 .* recent_kind=none compatible_session=0 package_generation=0 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "empty Overview gesture capture"
  run_qmp_action touch-move 360 1330
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=1 nav_reveal_px=240 .* recent_kind=none compatible_session=0 package_generation=0 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "empty Overview finger-follow state"
  run_qmp_action touch-up
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=overview recent_app=none nav_pressed=0 nav_reveal_px=0 .* recent_kind=none compatible_session=0 package_generation=0 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "stable empty Overview"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit for empty Overview"
  wait_for_mobile_screenshot \
    overview-empty \
    "$screenshot" \
    "the stable empty Overview card"
}

open_recovery_drawer_and_launch() {
  local launcher_pid="$1"
  local screenshot="$2"
  local before

  # Open All apps with a long upward gesture. A bounded raster predicate,
  # rather than a fixed frame delta, proves that the settled generation-1
  # item is actually under the subsequent tap.
  run_qmp_action drag 360 1280 360 520
  wait_for_mobile_screenshot \
    drawer-installed \
    "$ARTIFACT_DIR/recovery.drawer-installed.ppm" \
    "the settled All apps drawer"

  before="$(launcher_commit_count "$launcher_pid")"
  run_qmp_action tap 106 674
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=reserve-compatible app=android-compatible request_id=2 .* recent_kind=compatible-activity compatible_session=1 package_generation=1$" \
    "the authenticated Home compatible-Activity verification reservation"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=reserve-compatible status=accepted request_id=2 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 reservation_origin=home$" \
    "the accepted Home compatible-Activity verification reservation"
  wait_for_boot_pattern \
    '^ANDROID_PACKAGE_DURABLE_LAUNCH_OK owner=[1-9][0-9]* request_sequence=1 generation=1 .* profile=Resources-1 reads=389 writes=0 flushes=0 .* constructor_instructions=2 on_create_instructions=4$' \
    "durable installed-APK relaunch 1"
  wait_for_boot_pattern \
    '^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=[1-9][0-9]* request_sequence=1 generation=1 bytes=640 reads=389 writes=0 flushes=0 authority_granted=0 apk_bytes_exposed=0$' \
    "exact relaunch collection 1"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=present-compatible app=android-compatible request_id=3 .* recent_kind=compatible-activity compatible_session=1 package_generation=1$" \
    "the authenticated compatible-Activity presentation"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=present-compatible status=accepted request_id=3 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 reservation_origin=home$" \
    "the accepted compatible-Activity presentation"
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "the first compatible-Activity foreground session"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit after compatible-Activity presentation"
  wait_for_mobile_screenshot \
    activity \
    "$screenshot" \
    "the settled installed Activity frame for relaunch 1"
}

exercise_recovery_relaunches() {
  local launcher_pid
  local before
  local release_pattern
  local release_before
  local durable_before
  local collect_before
  local request_before
  local completion_before
  local system_ui_before
  local launcher_before
  local foreground_compatible_pattern
  local foreground_compatible_before
  local finished_home_pattern
  local finished_home_before

  wait_for_boot_pattern \
    '^MOBILE_UI_PREVIEW_OK .* abi=44 width=720 height=1600 design_width=360 design_height=800 .* network=disabled .* real_phone_claim=0$' \
    "the ABI-44 720x1600 mobile preview"
  wait_for_boot_pattern \
    '^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=[1-9][0-9]* ' \
    "the initial Launcher frame"
  normalize_boot_log
  launcher_pid="$(
    awk '/^USER_SURFACE_BUFFER_COMMIT_OK / {
      for (field = 1; field <= NF; field++) {
        if ($field ~ /^producer_pid=/) {
          sub(/^producer_pid=/, "", $field)
          print $field
          exit
        }
      }
    }' "$BOOT_NORMALIZED_LOG"
  )"
  [[ "$launcher_pid" =~ ^[1-9][0-9]*$ ]] || {
    fail_gate "Could not derive the generation-qualified Launcher owner."
  }

  # Unlock the preview session, clear transition input quarantine with a
  # neutral release, then launch the exact installed catalog generation.
  run_qmp_action drag 360 1390 360 620
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=unlock app=none request_id=1 " \
    "the authenticated preview unlock"
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' \
    "the unlocked Home state"
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=unlock status=accepted request_id=1 .* recent_kind=none compatible_session=0 package_generation=0 reservation_origin=none$' \
    "the accepted preview unlock"
  run_qmp_action tap 700 900
  open_recovery_drawer_and_launch \
    "$launcher_pid" \
    "$ARTIFACT_DIR/recovery.relaunch-sequence1.activity.ppm"

  # System Home retains only the compatible identity. The exact 240px
  # SurfaceServer gesture then opens an identity-only Overview card.
  before="$(launcher_commit_count "$launcher_pid")"
  run_qmp_action tap 360 1570
  wait_for_boot_pattern \
    '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$' \
    "Home retaining the compatible-Activity identity"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit for compatible-Activity Home"
  wait_for_compatible_overview \
    "$launcher_pid" \
    "$ARTIFACT_DIR/recovery.compatible-overview.ppm"

  # Activating the identity card cannot reuse cached Activity content. The
  # Launcher must issue syscall 60 sequence 2, collect the fresh read-only
  # proof, and only then return the same session identity to Foreground.
  before="$(launcher_commit_count "$launcher_pid")"
  foreground_compatible_pattern='^UI_SYSTEM_UI_CHANGED_OK .* mode=foreground recent_app=android-compatible nav_pressed=0 nav_reveal_px=0 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$'
  foreground_compatible_before="$(boot_log_count "$foreground_compatible_pattern")"
  run_qmp_action tap 360 920
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=reserve-compatible app=android-compatible request_id=4 .* recent_kind=compatible-activity compatible_session=1 package_generation=1$" \
    "the authenticated Overview compatible-Activity verification reservation"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=reserve-compatible status=accepted request_id=4 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 reservation_origin=overview$" \
    "the accepted Overview compatible-Activity verification reservation"
  wait_for_boot_pattern \
    '^ANDROID_PACKAGE_DURABLE_LAUNCH_OK owner=[1-9][0-9]* request_sequence=2 generation=1 .* profile=Resources-1 reads=389 writes=0 flushes=0 .* constructor_instructions=2 on_create_instructions=4$' \
    "durable installed-APK relaunch 2 from Overview"
  wait_for_boot_pattern \
    '^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK owner=[1-9][0-9]* request_sequence=2 generation=1 bytes=640 reads=389 writes=0 flushes=0 authority_granted=0 apk_bytes_exposed=0$' \
    "exact relaunch collection 2 from Overview"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=activate-recent app=android-compatible request_id=5 .* recent_kind=compatible-activity compatible_session=1 package_generation=1$" \
    "the authenticated compatible recent activation"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=activate-recent status=accepted request_id=5 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 reservation_origin=overview$" \
    "the accepted compatible recent activation"
  wait_for_boot_count_at_least \
    "$foreground_compatible_pattern" \
    "$((foreground_compatible_before + 1))" \
    "the freshly relaunched compatible-Activity foreground"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit after compatible recent activation"
  wait_for_mobile_screenshot \
    activity \
    "$ARTIFACT_DIR/recovery.relaunch-sequence2.activity.ppm" \
    "the settled installed Activity frame for relaunch 2"

  # Back finishes the compatible session instead of retaining a fake
  # background task. The next Overview is therefore empty.
  before="$(launcher_commit_count "$launcher_pid")"
  finished_home_pattern='^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 .* recent_kind=none compatible_session=0 package_generation=0 activity_pixels=0 thumbnail=0 live_preview=0 background_execution=0$'
  finished_home_before="$(boot_log_count "$finished_home_pattern")"
  run_qmp_action tap 56 120
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_OK sender_image=launcher sender_pid=${launcher_pid} .* action=finish-compatible app=android-compatible request_id=6 .* recent_kind=compatible-activity compatible_session=1 package_generation=1$" \
    "the authenticated compatible-Activity finish"
  wait_for_boot_pattern \
    "^UI_SYSTEM_UI_REQUEST_COMPLETED_OK .* receiver_image=launcher .* action=finish-compatible status=accepted request_id=6 .* recent_kind=compatible-activity compatible_session=1 package_generation=1 reservation_origin=none$" \
    "the accepted compatible-Activity finish"
  wait_for_boot_count_at_least \
    "$finished_home_pattern" \
    "$((finished_home_before + 1))" \
    "Home after compatible-Activity finish"
  wait_for_boot_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${launcher_pid} " \
    "$((before + 1))" \
    "a Launcher commit after compatible-Activity finish"
  wait_for_empty_overview \
    "$launcher_pid" \
    "$ARTIFACT_DIR/recovery.empty-overview-before-tap.ppm"

  # The center of an empty recent card is deliberately inert. Wait for the
  # routed release, then prove it produced no semantic request, state change,
  # durable readback, or frame and left the raster byte-identical.
  release_pattern='^UI_ROUTE_INPUT_OK .* receiver_image=launcher .* x=360 y=920 pressed=0$'
  release_before="$(boot_log_count "$release_pattern")"
  durable_before="$(boot_log_count '^ANDROID_PACKAGE_DURABLE_LAUNCH_OK ')"
  collect_before="$(boot_log_count '^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ')"
  request_before="$(boot_log_count '^UI_SYSTEM_UI_REQUEST_OK ')"
  completion_before="$(boot_log_count '^UI_SYSTEM_UI_REQUEST_COMPLETED_OK ')"
  system_ui_before="$(boot_log_count '^UI_SYSTEM_UI_CHANGED_OK ')"
  launcher_before="$(launcher_commit_count "$launcher_pid")"
  run_qmp_action tap 360 920
  wait_for_boot_count_at_least \
    "$release_pattern" \
    "$((release_before + 1))" \
    "the inert empty-Overview release"
  wait_for_mobile_screenshot \
    overview-empty \
    "$ARTIFACT_DIR/recovery.empty-overview-after-tap.ppm" \
    "empty Overview after the inert tap"
  [[ "$(boot_log_count '^ANDROID_PACKAGE_DURABLE_LAUNCH_OK ')" == "$durable_before" \
    && "$(boot_log_count '^ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ')" == "$collect_before" \
    && "$(boot_log_count '^UI_SYSTEM_UI_REQUEST_OK ')" == "$request_before" \
    && "$(boot_log_count '^UI_SYSTEM_UI_REQUEST_COMPLETED_OK ')" == "$completion_before" \
    && "$(boot_log_count '^UI_SYSTEM_UI_CHANGED_OK ')" == "$system_ui_before" \
    && "$(launcher_commit_count "$launcher_pid")" == "$launcher_before" ]] || {
    fail_gate "The empty Overview tap was not semantically inert."
  }
  cmp \
    "$ARTIFACT_DIR/recovery.empty-overview-before-tap.ppm" \
    "$ARTIFACT_DIR/recovery.empty-overview-after-tap.ppm" \
    || fail_gate "The inert empty Overview tap changed the rendered pixels."
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

# The gate has one literal emulator launch, reused for the sourced install and
# the source-free recovery. Every boot uses one explicit writable raw image.
run_qemu_boot() {
  local name="$1"
  local disk="$2"
  local source_apk="$3"
  # Keep the array non-empty for macOS Bash 3.2 with `set -u`.
  local -a source_args=(-name "Bndroid Local APK Gate $name")
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
    -drive "if=none,file=$disk,format=raw,readonly=off,snapshot=off,cache=writeback,id=bndroid-storage" \
    -device virtio-blk-device,drive=bndroid-storage,queue-size=8,event_idx=off,indirect_desc=off,config-wce=off,write-cache=on,discard=off,write-zeroes=off \
    -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
    -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
    "${source_args[@]}" \
    >"$BOOT_QEMU_LOG" 2>&1 &
  QEMU_PID=$!

  wait_for_boot_pattern \
    '^ANDROIDBOX_APK_INSTALL0_PROFILE_OK ' \
    "$name Resources-1 completion"
  if [[ "$name" == "recovery" ]]; then
    exercise_recovery_relaunches
  fi
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

validate_source_markers() {
  local log="$1"
  local present="$2"
  local bytes=0
  local digest="$ZERO_SHA256"
  local selector=0x0000
  local directory_files=11
  local dma_ops=2
  if [[ "$present" == "1" ]]; then
    bytes="$APK_BYTES"
    digest="$APK_SHA256"
    selector=0x002a
    directory_files=12
    dma_ops=3
  fi
  require_once_fixed \
    "$log" \
    "APK_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/apk present=$present bytes=$bytes selector=$selector directory_files=$directory_files dma_ops=$dma_ops sha256=$digest explicit_source=1 host_directory_scan=0 network=disabled install_mutation=0" \
    "APK source marker"
  require_once_fixed \
    "$log" \
    "APK_UNINSTALL_SOURCE_OK format=1 transport=qemu-fw_cfg name=opt/bndroid/package-uninstall present=0 bytes=0 selector=0x0000 directory_files=$directory_files dma_ops=$dma_ops sha256=$ZERO_SHA256 wire=BNDUNS01 wire_bytes=256 canonical_validation=1 explicit_source=1 host_directory_scan=0 network=disabled package_mutation=0" \
    "absent uninstall-request marker"
}

PROFILE_MARKER='ANDROIDBOX_APK_INSTALL0_PROFILE_OK format=3 apk_source=qemu-fw_cfg-or-package-store signature=apk-v2-single-signer package_store=single-package-crash-consistent-stateful launch_source=boot-and-click-durable-readback update0=same-package-same-signer-monotonic-version uninstall0=canonical-host-request-identical-dual-tombstone reinstall0=retained-package-signer-no-version-rollback atomic_old-new-switch=1 atomic_installed-removed-switch=1 exact-source-replay=idempotent-zero-write exact-uninstall-replay=idempotent-zero-write rollback-source=reject-before-write app_data_policy=no-managed-package-data apk_blob_erased=0 old_kernel_downgrade_safe=0 snapshot_syscall=59 snapshot_wire=BNDAPS01 snapshot_bytes=640 relaunch_syscall=60 relaunch_request_wire=BNDARQ01 relaunch_response_wire=BNDAPS01 relaunch_mode=async-exact-retry relaunch_owner=launcher-generation relaunch_writes=0 ui_catalog=kernel-supplied ui_installed_launch=fresh-durable-reexecution launcher_resolution=manifest-main-launcher activity_class_binding=exact-dex-descriptor fixed_dex_probe_required=0 activity_lifecycle=constructor-then-onCreate constructor_required=1 apk_bytes_exposed_to_el0=0 storage_authority_granted_to_el0=0 install_ui=0 uninstall_ui=0 update_ui=0 package_manager_api=0 art=0 dalvik=0 activitythread=0 framework=Resources-1-subset binder=0 bionic=0 jni=0 native_lib=0 permissions=0 general_apk_claim=0 android_compatibility_claim=0 network=disabled emulator_only=1 real_phone_claim=0'

validate_installed_boot() {
  local log="$1"
  local source_present="$2"
  local formatted="$3"
  local operation="$4"
  local previous_generation="$5"
  local previous_version="$6"
  local reads="$7"
  local writes="$8"
  local flushes="$9"
  local source_free="${10}"
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
    "APK_PACKAGE_STORE_OK format=3 formatted_this_boot=$formatted source_present=$source_present source_admitted=$source_present uninstall_request_present=0 uninstall_request_used=0 operation=$operation previous_generation=$previous_generation previous_version_code=$previous_version installed=1 removed=0 generation=1 slot=0 apk_bytes=$APK_BYTES version_code=$VERSION_CODE package=$PACKAGE activity=$ACTIVITY profile=Resources-1 registry_blob_bound=1 full_readback=1 durable_reverification=1 mutation_performed=$mutation reads=$reads writes=$writes flushes=$flushes source_free_zero_writes=$source_free source_replay_zero_writes=0 reinstall_from_tombstone=0 apk_sha256=$APK_SHA256 signer_cert_sha256=$CERT_SHA256 network=disabled el0_package_write=0 general_android_compatibility=0" \
    "durable installed-package marker"
  require_once_ere \
    "$log" \
    "^ANDROIDBOX_INSTALLED_ACTIVITY_OK source=package-store-readback title=$APP_NAME text=$TEXT constructor=1 constructor_method=[0-9]+ constructor_code_offset=[0-9]+ constructor_instructions=2 on_create=1 on_create_method=[0-9]+ on_create_code_offset=[0-9]+ on_create_instructions=4 resources_arsc_crc=$RESOURCES_CRC layout_xml_crc=$LAYOUT_CRC layout_resource_id=$LAYOUT_ID string_resource_id=$TEXT_ID set_content_view_int=1 apk_v2=1 signer_count=1 art=0 dalvik=0 binder=0 jni=0 native_lib=0 framework_subset=Resources-1 general_apk_claim=0 android_compatibility_claim=0 emulator_only=1 real_phone_claim=0$" \
    "durable Resources-1 Activity marker"
  require_once_fixed \
    "$log" \
    "STORAGE_LIMITS writes=$mutation partitions=4 filesystems=1 vfs=1 persistence=package-store-only flush=1 package_store=single-package-stateful package_states=empty-installed-removed apk_max_bytes=65024 appdata_mounted=0 app_data_policy=no-managed-package-data data_persistence_advanced=0 el0_package_storage=0 apk_blob_erase=0 crash_consistency=double-registry-double-blob+mirrored-tombstone host_powercut_claim=0 physical_powerloss_claim=0 general_runtime=0" \
    "storage boundary marker"
  require_once_fixed "$log" "$PROFILE_MARKER" "Resources-1 profile marker"
  require_once_ere \
    "$log" \
    '^MOBILE_UI_PREVIEW_OK profile=local-qemu abi=44 width=720 height=1600 design_width=360 design_height=800 scale=2 aspect=20:9 .* network=disabled .* real_phone_claim=0$' \
    "720x1600 mobile UI marker"
  if grep -Eq \
    '^APK_PACKAGE_STORE_REMOVED_OK |^APK_PACKAGE_STORE_EMPTY_OK |^STORAGE_FAIL |^boot error:' \
    "$log"; then
    BOOT_NORMALIZED_LOG="$log"
    fail_gate "Valid local APK boot emitted contradictory or failure evidence."
  fi
}

validate_relaunch_evidence() {
  local log="$1"
  local output="$ARTIFACT_DIR/relaunch-evidence.txt"
  python3 - \
    "$log" \
    "$ARTIFACT_DIR/recovery.relaunch-sequence1.activity.ppm" \
    "$ARTIFACT_DIR/recovery.compatible-overview.ppm" \
    "$ARTIFACT_DIR/recovery.relaunch-sequence2.activity.ppm" \
    "$ARTIFACT_DIR/recovery.empty-overview-before-tap.ppm" \
    "$ARTIFACT_DIR/recovery.empty-overview-after-tap.ppm" \
    "$output" \
    "$PACKAGE" \
    "$ACTIVITY" \
    "$VERSION_CODE" \
    "$APK_BYTES" <<'PY'
from hashlib import sha256
from pathlib import Path
import re
import sys

log_path = Path(sys.argv[1])
activity_paths = (Path(sys.argv[2]), Path(sys.argv[4]))
compatible_overview_path = Path(sys.argv[3])
empty_overview_paths = (Path(sys.argv[5]), Path(sys.argv[6]))
output_path = Path(sys.argv[7])
package = sys.argv[8]
activity = sys.argv[9]
version_code = sys.argv[10]
apk_length = sys.argv[11]
lines = log_path.read_text(encoding="utf-8").splitlines()


def marker_fields(line: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for value in line.split()[1:]:
        if "=" not in value:
            raise SystemExit(f"non-canonical marker field: {value!r}")
        name, field_value = value.split("=", 1)
        if name in fields:
            raise SystemExit(f"duplicated marker field: {name}")
        fields[name] = field_value
    return fields


def exact_lines(prefix: str) -> list[tuple[int, str, dict[str, str]]]:
    return [
        (index, line, marker_fields(line))
        for index, line in enumerate(lines)
        if line.startswith(prefix)
    ]


durable = exact_lines("ANDROID_PACKAGE_DURABLE_LAUNCH_OK ")
collect = exact_lines("ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ")
if len(durable) != 2 or len(collect) != 2:
    raise SystemExit(
        "source-free recovery must publish exactly two durable and collect markers"
    )
if any(line.startswith("ANDROID_PACKAGE_DURABLE_LAUNCH_FAIL ") for line in lines):
    raise SystemExit("source-free recovery emitted a durable relaunch failure")

launcher_commits = exact_lines("USER_SURFACE_BUFFER_COMMIT_OK ")
if not launcher_commits:
    raise SystemExit("source-free recovery emitted no Launcher frame")
launcher_owner = launcher_commits[0][2].get("producer_pid", "")
if not re.fullmatch(r"[1-9][0-9]*", launcher_owner):
    raise SystemExit("could not derive the generation-qualified Launcher owner")

durable_by_sequence = {
    fields.get("request_sequence"): (index, fields)
    for index, _line, fields in durable
}
collect_by_sequence = {
    fields.get("request_sequence"): (index, fields)
    for index, _line, fields in collect
}
if set(durable_by_sequence) != {"1", "2"} or set(collect_by_sequence) != {"1", "2"}:
    raise SystemExit("durable relaunch sequences are not exactly 1 then 2")

reads_by_sequence: dict[str, int] = {}
previous_collect_index = -1
for sequence in ("1", "2"):
    durable_index, durable_fields = durable_by_sequence[sequence]
    collect_index, collect_fields = collect_by_sequence[sequence]
    expected_durable = {
        "owner": launcher_owner,
        "request_sequence": sequence,
        "generation": "1",
        "version_code": version_code,
        "apk_length": apk_length,
        "profile": "Resources-1",
        "writes": "0",
        "flushes": "0",
        "package": package,
        "activity": activity,
        "constructor_instructions": "2",
        "on_create_instructions": "4",
    }
    expected_collect = {
        "owner": launcher_owner,
        "request_sequence": sequence,
        "generation": "1",
        "bytes": "640",
        "writes": "0",
        "flushes": "0",
        "authority_granted": "0",
        "apk_bytes_exposed": "0",
    }
    for name, expected in expected_durable.items():
        if durable_fields.get(name) != expected:
            raise SystemExit(
                f"relaunch {sequence} durable {name}: "
                f"expected {expected!r}, observed {durable_fields.get(name)!r}"
            )
    for name, expected in expected_collect.items():
        if collect_fields.get(name) != expected:
            raise SystemExit(
                f"relaunch {sequence} collect {name}: "
                f"expected {expected!r}, observed {collect_fields.get(name)!r}"
            )
    try:
        durable_reads = int(durable_fields.get("reads", "0"), 10)
        collect_reads = int(collect_fields.get("reads", "0"), 10)
    except ValueError as error:
        raise SystemExit(f"relaunch {sequence} has a non-decimal read count") from error
    if durable_reads != 389 or collect_reads != 389:
        raise SystemExit(
            f"relaunch {sequence} did not perform the exact 389 read-only reads"
        )
    if not previous_collect_index < durable_index < collect_index:
        raise SystemExit(f"relaunch {sequence} markers are out of order")
    previous_collect_index = collect_index
    reads_by_sequence[sequence] = durable_reads


requests = exact_lines("UI_SYSTEM_UI_REQUEST_OK ")
expected_requests = (
    ("unlock", "none", "none", "0", "0"),
    ("reserve-compatible", "android-compatible", "compatible-activity", "1", "1"),
    ("present-compatible", "android-compatible", "compatible-activity", "1", "1"),
    ("reserve-compatible", "android-compatible", "compatible-activity", "1", "1"),
    ("activate-recent", "android-compatible", "compatible-activity", "1", "1"),
    ("finish-compatible", "android-compatible", "compatible-activity", "1", "1"),
)
if len(requests) != len(expected_requests):
    raise SystemExit(
        f"expected exactly six System UI requests, observed {len(requests)}"
    )
for request_id, ((index, _line, fields), expected) in enumerate(
    zip(requests, expected_requests), 1
):
    action, recent_app, recent_kind, compatible_session, package_generation = expected
    expected_fields = {
        "sender_image": "launcher",
        "sender_pid": launcher_owner,
        "receiver_image": "surface-server",
        "action": action,
        "app": recent_app,
        "request_id": str(request_id),
        "recent_kind": recent_kind,
        "compatible_session": compatible_session,
        "package_generation": package_generation,
    }
    for name, expected_value in expected_fields.items():
        if fields.get(name) != expected_value:
            raise SystemExit(
                f"System UI request {request_id} {name}: expected "
                f"{expected_value!r}, observed {fields.get(name)!r}"
            )
    try:
        if int(fields.get("observed_revision", "0"), 10) <= 0:
            raise ValueError
    except ValueError as error:
        raise SystemExit(
            f"System UI request {request_id} has an invalid observed revision"
        ) from error

completions = exact_lines("UI_SYSTEM_UI_REQUEST_COMPLETED_OK ")
expected_completion_origins = (
    "none",
    "home",
    "home",
    "overview",
    "overview",
    "none",
)
if len(completions) != len(expected_requests):
    raise SystemExit(
        "every System UI request must have exactly one explicit completion"
    )
for request_id, ((index, _line, fields), expected, origin) in enumerate(
    zip(completions, expected_requests, expected_completion_origins), 1
):
    action, _recent_app, recent_kind, compatible_session, package_generation = (
        expected
    )
    expected_fields = {
        "sender_image": "surface-server",
        "receiver_image": "launcher",
        "session": "1",
        "action": action,
        "status": "accepted",
        "request_id": str(request_id),
        "recent_kind": recent_kind,
        "compatible_session": compatible_session,
        "package_generation": package_generation,
        "reservation_origin": origin,
    }
    for name, expected_value in expected_fields.items():
        if fields.get(name) != expected_value:
            raise SystemExit(
                f"System UI completion {request_id} {name}: expected "
                f"{expected_value!r}, observed {fields.get(name)!r}"
            )
    try:
        completion_revision = int(fields.get("revision", "0"), 10)
        observed_revision = int(
            requests[request_id - 1][2].get("observed_revision", "0"), 10
        )
    except ValueError as error:
        raise SystemExit(
            f"System UI completion {request_id} has a non-decimal revision"
        ) from error
    if completion_revision != observed_revision + 1:
        raise SystemExit(
            f"System UI completion {request_id} did not advance exactly one revision"
        )
    if index <= requests[request_id - 1][0]:
        raise SystemExit(
            f"System UI completion {request_id} preceded its request"
        )

changes = exact_lines("UI_SYSTEM_UI_CHANGED_OK ")
if not changes:
    raise SystemExit("source-free recovery emitted no System UI state")
by_revision: dict[int, dict[str, tuple[int, dict[str, str]]]] = {}
for index, _line, fields in changes:
    expected_boundary = {
        "sender_image": "surface-server",
        "session": "1",
        "activity_pixels": "0",
        "thumbnail": "0",
        "live_preview": "0",
        "background_execution": "0",
    }
    for name, expected_value in expected_boundary.items():
        if fields.get(name) != expected_value:
            raise SystemExit(
                f"System UI boundary {name}: expected {expected_value!r}, "
                f"observed {fields.get(name)!r}"
            )
    receiver = fields.get("receiver_image", "")
    if receiver not in {"launcher", "app"}:
        raise SystemExit(f"invalid System UI receiver: {receiver!r}")
    try:
        revision = int(fields.get("revision", "0"), 10)
    except ValueError as error:
        raise SystemExit("System UI revision is not decimal") from error
    if revision <= 0 or receiver in by_revision.setdefault(revision, {}):
        raise SystemExit(f"duplicated or invalid System UI revision {revision}")
    by_revision[revision][receiver] = (index, fields)

if sorted(by_revision) != list(range(1, max(by_revision) + 1)):
    raise SystemExit("System UI revisions are not contiguous from one")
state_fields = (
    "mode",
    "recent_app",
    "nav_pressed",
    "nav_reveal_px",
    "recent_kind",
    "compatible_session",
    "package_generation",
    "activity_pixels",
    "thumbnail",
    "live_preview",
    "background_execution",
)
launcher_states: list[tuple[int, dict[str, str]]] = []
for revision in sorted(by_revision):
    receivers = by_revision[revision]
    if set(receivers) != {"launcher", "app"}:
        raise SystemExit(f"System UI revision {revision} is not a two-client broadcast")
    launcher_index, launcher_fields = receivers["launcher"]
    _app_index, app_fields = receivers["app"]
    if tuple(launcher_fields.get(name) for name in state_fields) != tuple(
        app_fields.get(name) for name in state_fields
    ):
        raise SystemExit(f"System UI revision {revision} diverged between clients")
    launcher_states.append((launcher_index, launcher_fields))

for request_id, (_index, _line, fields) in enumerate(completions, 1):
    revision = int(fields["revision"], 10)
    receivers = by_revision.get(revision)
    if (
        receivers is None
        or receivers["launcher"][0] >= completions[request_id - 1][0]
    ):
        raise SystemExit(
            f"System UI completion {request_id} did not follow Launcher state"
        )

def next_stable_state(
    after_index: int,
    mode: str,
    recent_app: str,
    recent_kind: str,
    compatible_session: str,
    package_generation: str,
) -> tuple[int, dict[str, str]]:
    for index, fields in launcher_states:
        if (
            index > after_index
            and fields.get("mode") == mode
            and fields.get("recent_app") == recent_app
            and fields.get("nav_pressed") == "0"
            and fields.get("nav_reveal_px") == "0"
            and fields.get("recent_kind") == recent_kind
            and fields.get("compatible_session") == compatible_session
            and fields.get("package_generation") == package_generation
        ):
            return index, fields
    raise SystemExit(
        "missing stable System UI state "
        f"{mode}/{recent_app}/{recent_kind}/{compatible_session}/{package_generation}"
    )

unlock_index = requests[0][0]
unlocked_home_index, _ = next_stable_state(
    unlock_index, "home", "none", "none", "0", "0"
)
reserve_one_index = requests[1][0]
reserve_one_state_index, _ = next_stable_state(
    reserve_one_index, "home", "none", "none", "0", "0"
)
present_index = requests[2][0]
foreground_one_index, _ = next_stable_state(
    present_index,
    "foreground",
    "android-compatible",
    "compatible-activity",
    "1",
    "1",
)
home_recent_index, _ = next_stable_state(
    foreground_one_index,
    "home",
    "android-compatible",
    "compatible-activity",
    "1",
    "1",
)
overview_recent_index, _ = next_stable_state(
    home_recent_index,
    "overview",
    "android-compatible",
    "compatible-activity",
    "1",
    "1",
)
reserve_two_index = requests[3][0]
reserve_two_state_index, _ = next_stable_state(
    reserve_two_index,
    "overview",
    "android-compatible",
    "compatible-activity",
    "1",
    "1",
)
activate_index = requests[4][0]
foreground_two_index, _ = next_stable_state(
    activate_index,
    "foreground",
    "android-compatible",
    "compatible-activity",
    "1",
    "1",
)
finish_index = requests[5][0]
finished_home_index, _ = next_stable_state(
    finish_index, "home", "none", "none", "0", "0"
)
empty_overview_index, _ = next_stable_state(
    finished_home_index, "overview", "none", "none", "0", "0"
)

durable_one_index, _ = durable_by_sequence["1"]
collect_one_index, _ = collect_by_sequence["1"]
durable_two_index, _ = durable_by_sequence["2"]
collect_two_index, _ = collect_by_sequence["2"]
if not (
    unlock_index
    < unlocked_home_index
    < completions[0][0]
    < reserve_one_index
    < reserve_one_state_index
    < completions[1][0]
    < durable_one_index
    < collect_one_index
    < present_index
    < foreground_one_index
    < completions[2][0]
    < home_recent_index
    < overview_recent_index
    < reserve_two_index
    < reserve_two_state_index
    < completions[3][0]
    < durable_two_index
    < collect_two_index
    < activate_index
    < foreground_two_index
    < completions[4][0]
    < finish_index
    < finished_home_index
    < completions[5][0]
    < empty_overview_index
):
    raise SystemExit("compatible-Activity session evidence is out of order")

routed_inputs = exact_lines("UI_ROUTE_INPUT_OK ")
empty_releases = [
    index
    for index, _line, fields in routed_inputs
    if fields.get("receiver_image") == "launcher"
    and fields.get("x") == "360"
    and fields.get("y") == "920"
    and fields.get("pressed") == "0"
]
if not empty_releases:
    raise SystemExit("the empty Overview release was not routed to Launcher")
empty_release_index = empty_releases[-1]
if empty_release_index <= empty_overview_index:
    raise SystemExit("the inert release preceded the stable empty Overview")
for prefix in (
    "ANDROID_PACKAGE_DURABLE_LAUNCH_OK ",
    "ANDROID_PACKAGE_RELAUNCH_COLLECT_OK ",
    "UI_SYSTEM_UI_REQUEST_OK ",
    "UI_SYSTEM_UI_REQUEST_COMPLETED_OK ",
    "UI_SYSTEM_UI_CHANGED_OK ",
    "USER_SURFACE_BUFFER_COMMIT_OK ",
):
    if any(index > empty_release_index for index, _line, _fields in exact_lines(prefix)):
        raise SystemExit(f"the empty Overview release produced {prefix.strip()}")


def ppm_pixels(path: Path, kind: str) -> tuple[bytes, str]:
    payload = path.read_bytes()
    parts = payload.split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{path.name} is not an exact 720x1600 PPM")
    pixels = parts[3]
    if len(pixels) != 720 * 1600 * 3:
        raise SystemExit(f"{path.name} has a non-canonical pixel payload")

    def pixel(x: int, y: int) -> tuple[int, int, int]:
        offset = (y * 720 + x) * 3
        return tuple(pixels[offset : offset + 3])

    expected_by_kind = {
        "activity": {
            (108, 268): (16, 118, 111),
            (180, 267): (24, 34, 56),
            (360, 300): (24, 34, 56),
            (100, 500): (32, 45, 73),
            (719, 1599): (0, 0, 0),
        },
        "overview-compatible": {
            (360, 300): (13, 22, 41),
            (360, 580): (16, 118, 111),
            (719, 1599): (0, 0, 0),
        },
        "overview-empty": {
            (360, 300): (13, 22, 41),
            (360, 580): (24, 34, 56),
            (719, 1599): (0, 0, 0),
        },
    }
    expected_pixels = expected_by_kind[kind]
    for point, expected in expected_pixels.items():
        if pixel(*point) != expected:
            raise SystemExit(
                f"{path.name} is not stable {kind} at {point}"
            )
    minimum_colors = 64 if kind == "activity" else 24
    if (
        len({pixels[offset : offset + 3] for offset in range(0, len(pixels), 3)})
        < minimum_colors
    ):
        raise SystemExit(f"{path.name} is unexpectedly flat")
    return pixels, sha256(payload).hexdigest()


first_pixels, first_sha256 = ppm_pixels(activity_paths[0], "activity")
second_pixels, second_sha256 = ppm_pixels(activity_paths[1], "activity")
if first_pixels != second_pixels:
    raise SystemExit("the two generation-1 relaunches rendered different Activity pixels")
compatible_pixels, compatible_sha256 = ppm_pixels(
    compatible_overview_path, "overview-compatible"
)
empty_before_pixels, empty_before_sha256 = ppm_pixels(
    empty_overview_paths[0], "overview-empty"
)
empty_after_pixels, empty_after_sha256 = ppm_pixels(
    empty_overview_paths[1], "overview-empty"
)
if empty_before_pixels != empty_after_pixels:
    raise SystemExit("the inert empty Overview tap changed the raster")

def changed_pixels(
    before: bytes,
    after: bytes,
    bounds=None,
) -> int:
    changed = 0
    for index in range(720 * 1600):
        x, y = index % 720, index // 720
        if bounds is not None and not (
            bounds[0] <= x < bounds[2] and bounds[1] <= y < bounds[3]
        ):
            continue
        offset = index * 3
        changed += before[offset : offset + 3] != after[offset : offset + 3]
    return changed

recent_card = (48, 424, 672, 1184)
if changed_pixels(compatible_pixels, empty_before_pixels, recent_card) < 2_000:
    raise SystemExit("compatible and empty Overview identity cards are not distinct")
for index in range(720 * 1600):
    x, y = index % 720, index // 720
    if recent_card[0] <= x < recent_card[2] and recent_card[1] <= y < recent_card[3]:
        continue
    offset = index * 3
    if (
        compatible_pixels[offset : offset + 3]
        != empty_before_pixels[offset : offset + 3]
    ):
        raise SystemExit("compatible identity changed pixels outside the Overview card")
if changed_pixels(
    first_pixels, compatible_pixels, (0, 64, 720, 1548)
) < 200_000:
    raise SystemExit("Overview did not materially replace publisher Activity content")

evidence = {
    "durable_relaunches": "2",
    "relaunch_sequence1_generation": "1",
    "relaunch_sequence2_generation": "1",
    "relaunch_sequence1_reads": str(reads_by_sequence["1"]),
    "relaunch_sequence2_reads": str(reads_by_sequence["2"]),
    "relaunch_writes": "0",
    "relaunch_flushes": "0",
    "relaunch_response_bytes": "640",
    "relaunch_owner": launcher_owner,
    "activity_ppm_width": "720",
    "activity_ppm_height": "1600",
    "activity_sequence1_ppm_sha256": first_sha256,
    "activity_sequence2_ppm_sha256": second_sha256,
    "activity_raster_identical": "1",
    "compatible_activity_session": "1",
    "compatible_activity_generation": "1",
    "compatible_overview_ppm_sha256": compatible_sha256,
    "overview_activity_pixels": "0",
    "overview_thumbnail": "0",
    "overview_live_preview": "0",
    "overview_background_execution": "0",
    "compatible_finish_cleared_recent": "1",
    "empty_overview_before_ppm_sha256": empty_before_sha256,
    "empty_overview_after_ppm_sha256": empty_after_sha256,
    "empty_overview_tap_inert": "1",
}
output_path.write_text(
    "\n".join(f"{name}={value}" for name, value in evidence.items()) + "\n",
    encoding="utf-8",
)
PY
}

check_install_disk_diff() {
  python3 - "$INITIAL_IMAGE" "$INSTALLED_IMAGE" "$ARTIFACT_DIR/install.disk-diff.txt" <<'PY'
from pathlib import Path
import sys

before = Path(sys.argv[1]).read_bytes()
after = Path(sys.argv[2]).read_bytes()
output = Path(sys.argv[3])
if len(before) != 16 * 1024 * 1024 or len(after) != len(before):
    raise SystemExit("install diff requires exact 16 MiB images")
start = 16384 * 512
end = (16895 + 1) * 512
changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
if not changed:
    raise SystemExit("generation-1 install changed no disk bytes")
outside = [index for index in changed if not start <= index < end]
if outside:
    raise SystemExit(
        f"generation-1 install changed byte {outside[0]} outside BNDROID_PACKAGES"
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
}

# The only sourced boot starts from the deterministic virgin package-store
# image and publishes generation 1 / blob slot 0.
run_qemu_boot install "$PERSISTENT_IMAGE" "$SOURCE_APK"
validate_source_markers "$BOOT_NORMALIZED_LOG" 1
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  1 1 install 0 0 1032 130 3 0
INSTALL_DISK_SHA256="$(shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}')"
[[ "$INSTALL_DISK_SHA256" != "$INITIAL_SHA256" ]] || {
  fail_gate "Valid local APK install did not mutate the empty package store."
}
cp "$PERSISTENT_IMAGE" "$INSTALLED_IMAGE"
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/install.serial.normalized.log" \
  >"$ARTIFACT_DIR/install.activity"
check_install_disk_diff

# The second boot receives no APK source. The All apps entry and the subsequent
# compatible-Overview activation must independently revalidate and execute only
# the durable blob. Home retains an identity-only recent card, while Back
# finishes the session and makes the next Overview inert and empty.
RECOVERY_BEFORE_SHA256="$INSTALL_DISK_SHA256"
run_qemu_boot recovery "$PERSISTENT_IMAGE" ""
validate_source_markers "$BOOT_NORMALIZED_LOG" 0
validate_installed_boot \
  "$BOOT_NORMALIZED_LOG" \
  0 0 recovery 1 "$VERSION_CODE" 389 0 0 1
validate_relaunch_evidence "$BOOT_NORMALIZED_LOG"
RECOVERY_AFTER_SHA256="$(shasum -a 256 "$PERSISTENT_IMAGE" | awk '{print $1}')"
[[ "$RECOVERY_AFTER_SHA256" == "$RECOVERY_BEFORE_SHA256" ]] || {
  fail_gate "Source-free recovery or either installed-app click changed the complete package disk."
}
grep '^ANDROIDBOX_INSTALLED_ACTIVITY_OK ' \
  "$ARTIFACT_DIR/recovery.serial.normalized.log" \
  >"$ARTIFACT_DIR/recovery.activity"
cmp "$ARTIFACT_DIR/install.activity" "$ARTIFACT_DIR/recovery.activity"
RELAUNCH_SEQUENCE1_READS="$(
  sed -n 's/^relaunch_sequence1_reads=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
RELAUNCH_SEQUENCE2_READS="$(
  sed -n 's/^relaunch_sequence2_reads=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
ACTIVITY_SEQUENCE1_PPM_SHA256="$(
  sed -n 's/^activity_sequence1_ppm_sha256=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
ACTIVITY_SEQUENCE2_PPM_SHA256="$(
  sed -n 's/^activity_sequence2_ppm_sha256=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
COMPATIBLE_OVERVIEW_PPM_SHA256="$(
  sed -n 's/^compatible_overview_ppm_sha256=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
EMPTY_OVERVIEW_BEFORE_PPM_SHA256="$(
  sed -n 's/^empty_overview_before_ppm_sha256=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
EMPTY_OVERVIEW_AFTER_PPM_SHA256="$(
  sed -n 's/^empty_overview_after_ppm_sha256=//p' "$ARTIFACT_DIR/relaunch-evidence.txt"
)"
[[ "$RELAUNCH_SEQUENCE1_READS" == "389" \
  && "$RELAUNCH_SEQUENCE2_READS" == "389" \
  && "$ACTIVITY_SEQUENCE1_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$ACTIVITY_SEQUENCE2_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$COMPATIBLE_OVERVIEW_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$EMPTY_OVERVIEW_BEFORE_PPM_SHA256" =~ ^[0-9a-f]{64}$ \
  && "$EMPTY_OVERVIEW_AFTER_PPM_SHA256" == "$EMPTY_OVERVIEW_BEFORE_PPM_SHA256" ]] || {
  fail_gate "Could not retain the canonical durable relaunch evidence."
}

printf '%s\n' \
  "input_apk_path=$APK_PATH" \
  "apk_bytes=$APK_BYTES" \
  "apk_sha256=$APK_SHA256" \
  "signer_cert_sha256=$CERT_SHA256" \
  "signature_profile=apk-v2-only-single-signer" \
  "package=$PACKAGE" \
  "launcher_activity=$ACTIVITY" \
  "version_code=$VERSION_CODE" \
  "version_name=$VERSION_NAME" \
  "app_name=$APP_NAME" \
  "textview_text=$TEXT" \
  "text_resource_name=$TEXT_RESOURCE_NAME" \
  "zip_layout_entry=$LAYOUT_ENTRY" \
  "resources_arsc_crc32=$RESOURCES_CRC" \
  "layout_xml_crc32=$LAYOUT_CRC" \
  "view_resource_id=$VIEW_ID" \
  "layout_resource_id=$LAYOUT_ID" \
  "text_resource_id=$TEXT_ID" \
  "app_name_resource_id=$APP_NAME_ID" \
  "initial_disk_sha256=$INITIAL_SHA256" \
  "installed_disk_sha256=$INSTALL_DISK_SHA256" \
  "recovery_before_disk_sha256=$RECOVERY_BEFORE_SHA256" \
  "recovery_after_disk_sha256=$RECOVERY_AFTER_SHA256" \
  "relaunch_sequence1_reads=$RELAUNCH_SEQUENCE1_READS" \
  "relaunch_sequence2_reads=$RELAUNCH_SEQUENCE2_READS" \
  "activity_sequence1_ppm_sha256=$ACTIVITY_SEQUENCE1_PPM_SHA256" \
  "activity_sequence2_ppm_sha256=$ACTIVITY_SEQUENCE2_PPM_SHA256" \
  "compatible_overview_ppm_sha256=$COMPATIBLE_OVERVIEW_PPM_SHA256" \
  "empty_overview_before_ppm_sha256=$EMPTY_OVERVIEW_BEFORE_PPM_SHA256" \
  "empty_overview_after_ppm_sha256=$EMPTY_OVERVIEW_AFTER_PPM_SHA256" \
  'install_generation=1' \
  'install_slot=0' \
  'install_reads=1032' \
  'install_writes=130' \
  'install_flushes=3' \
  'recovery_reads=389' \
  'recovery_writes=0' \
  'recovery_flushes=0' \
  'recovery_disk_unchanged=1' \
  'durable_relaunches=2' \
  'relaunch_sequence1=1' \
  'relaunch_sequence2=2' \
  'relaunch_generation=1' \
  'relaunch_writes=0' \
  'relaunch_flushes=0' \
  'relaunch_disk_unchanged=1' \
  'relaunch_request_wire=BNDARQ01' \
  'relaunch_response_wire=BNDAPS01' \
  'relaunch_syscall=60' \
  'activity_ppm_width=720' \
  'activity_ppm_height=1600' \
  'activity_raster_identical=1' \
  'activity_readback_identical=1' \
  'compatible_activity_session=1' \
  'compatible_activity_generation=1' \
  'compatible_activity_home_recent=1' \
  'compatible_activity_overview_recent=1' \
  'compatible_activity_recent_relaunch=1' \
  'compatible_activity_finish_cleared_recent=1' \
  'overview_activity_pixels=0' \
  'overview_thumbnail=0' \
  'overview_live_preview=0' \
  'overview_background_execution=0' \
  'empty_overview_tap_inert=1' \
  'network=disabled' \
  'explicit_apk_file=1' \
  'host_directory_scan=0' \
  'framework_subset=Resources-1' \
  'launcher_resolution=manifest-main-launcher' \
  'activity_class_binding=exact-dex-descriptor' \
  'fixed_dex_probe_required=0' \
  'fixed_dex_probe_present=0' \
  'activity_lifecycle=constructor-then-onCreate' \
  'constructor_required=1' \
  'constructor_instructions=2' \
  'general_apk_claim=0' \
  'android_compatibility_claim=0' \
  >"$ARTIFACT_DIR/summary.txt"

echo "ANDROIDBOX_LOCAL_APK_QEMU_OK artifact_dir=$ARTIFACT_DIR apk_path=$APK_PATH apk_bytes=$APK_BYTES apk_sha256=$APK_SHA256 signer_cert_sha256=$CERT_SHA256 package=$PACKAGE activity=$ACTIVITY version_code=$VERSION_CODE generation=1 slot=0 install_writes=130 install_flushes=3 recovery_writes=0 recovery_flushes=0 recovery_disk_unchanged=1 durable_relaunches=2 relaunch_sequence1=1 relaunch_sequence2=2 relaunch_generation=1 relaunch_sequence1_reads=$RELAUNCH_SEQUENCE1_READS relaunch_sequence2_reads=$RELAUNCH_SEQUENCE2_READS relaunch_writes=0 relaunch_flushes=0 relaunch_disk_unchanged=1 relaunch_syscall=60 relaunch_request_wire=BNDARQ01 relaunch_response_wire=BNDAPS01 compatible_activity_session=1 compatible_activity_generation=1 compatible_activity_home_recent=1 compatible_activity_overview_recent=1 compatible_activity_recent_relaunch=1 compatible_activity_finish_cleared_recent=1 overview_activity_pixels=0 overview_thumbnail=0 overview_live_preview=0 overview_background_execution=0 empty_overview_tap_inert=1 activity_sequence1_ppm_sha256=$ACTIVITY_SEQUENCE1_PPM_SHA256 activity_sequence2_ppm_sha256=$ACTIVITY_SEQUENCE2_PPM_SHA256 compatible_overview_ppm_sha256=$COMPATIBLE_OVERVIEW_PPM_SHA256 empty_overview_ppm_sha256=$EMPTY_OVERVIEW_BEFORE_PPM_SHA256 activity_raster_identical=1 activity_readback_identical=1 resources_arsc_crc=$RESOURCES_CRC layout_xml_crc=$LAYOUT_CRC layout_resource_id=$LAYOUT_ID text_resource_id=$TEXT_ID app_name_resource_id=$APP_NAME_ID framework_subset=Resources-1 launcher_resolution=manifest-main-launcher activity_class_binding=exact-dex-descriptor fixed_dex_probe_required=0 fixed_dex_probe_present=0 activity_lifecycle=constructor-then-onCreate constructor_required=1 constructor_instructions=2 network=disabled general_apk_claim=0 android_compatibility_claim=0"
