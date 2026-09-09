#!/usr/bin/env bash
set -euo pipefail

# Offline local-QEMU acceptance gate for the deliberately restricted
# AndroidBox Resources-1 slice. The repository-owned APK is immutable Launcher
# rodata; this gate proves its two allowlisted DEX-0 integer entry points,
# binary-manifest launcher selection, real onCreate code-item execution, and
# one bounded resources.arsc -> compiled TextView layout -> string-reference
# path. It does not claim APK installation, ART/Dalvik, ActivityThread,
# Android Framework, ResourceManager, Binder, Bionic, JNI, arbitrary layouts,
# or general Android application compatibility.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
source "$SCRIPT_DIR/lib/storage-evidence.sh"
export CARGO_NET_OFFLINE=true

for tool in qemu-system-aarch64 python3 mktemp tr grep kill tail mkdir sleep awk cmp rm; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool not found; cannot verify AndroidBox Resources-1 in local QEMU." >&2
    exit 1
  }
done

TARGET_ROOT="$WORKSPACE_ROOT/target/androidbox-dex0-build"
KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/release/bndroid-kernel.img"
STORAGE_IMAGE="$TARGET_ROOT/bndroid-storage-m25.raw"
FIXTURE_APK="$WORKSPACE_ROOT/fixtures/androidbox-resource-demo/androidbox-resource-demo.apk"
QMP_HELPER="$SCRIPT_DIR/mobile_ui_qmp.py"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-60}"
EXPECTED_APK_SHA256=2cf96bb6a0de3bba9b981539014c29d2c0adcc17cc9738b24f69cf3046ce4ac7
EXPECTED_DEX_CRC32=888685706
EXPECTED_DEX_ADLER32=815745726
EXPECTED_MANIFEST_CRC32=1151030049
EXPECTED_RESOURCES_ARSC_CRC32=2674378676
EXPECTED_LAYOUT_XML_CRC32=919508420
EXPECTED_LAYOUT_RESOURCE_ID=2130837504
EXPECTED_STRING_RESOURCE_ID=2130903040
SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}"
AAPT2="$SDK_ROOT/build-tools/36.1.0/aapt2"

[[ "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be a positive integer." >&2
  exit 2
}
[[ -f "$FIXTURE_APK" ]] || {
  echo "Repository-owned AndroidBox fixture is missing: $FIXTURE_APK" >&2
  exit 1
}
[[ -x "$AAPT2" ]] || {
  echo "Android SDK aapt2 is missing; cannot independently inspect the binary manifest: $AAPT2" >&2
  exit 1
}

if [[ "${BNDROID_SKIP_BUILD:-0}" != "1" ]]; then
  CARGO_TARGET_DIR="$TARGET_ROOT" \
    BNDROID_PROFILE=release \
    BNDROID_KERNEL_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    BNDROID_USERSPACE_FEATURES=mobile-ui-runtime,androidbox-dex0 \
    "$SCRIPT_DIR/build-kernel.sh" >/dev/null
  BNDROID_STORAGE_IMAGE="$STORAGE_IMAGE" \
    "$SCRIPT_DIR/build-storage-image.sh" >/dev/null
fi

[[ -f "$KERNEL_IMAGE" && -x "$QMP_HELPER" ]] || {
  echo "AndroidBox kernel or QMP helper is missing." >&2
  exit 1
}
validate_storage_image_geometry "$STORAGE_IMAGE" || {
  echo "Storage image does not have the required 8 MiB geometry: $STORAGE_IMAGE" >&2
  exit 1
}
build_storage_qemu_args "$STORAGE_IMAGE" modern

mkdir -p "$WORKSPACE_ROOT/target/androidbox-dex0"
ARTIFACT_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/androidbox-dex0/check.XXXXXX")"
SERIAL_LOG="$ARTIFACT_DIR/serial.log"
NORMALIZED_LOG="$ARTIFACT_DIR/serial.normalized.log"
QEMU_LOG="$ARTIFACT_DIR/qemu.log"
QMP_SOCKET="$ARTIFACT_DIR/qmp.sock"
MANIFEST_BADGING="$ARTIFACT_DIR/manifest.badging.txt"
MANIFEST_TREE="$ARTIFACT_DIR/manifest.tree.txt"
RESOURCES_DUMP="$ARTIFACT_DIR/resources.dump.txt"
LAYOUT_TREE="$ARTIFACT_DIR/layout.tree.txt"
QEMU_PID=""
: >"$SERIAL_LOG"
: >"$QEMU_LOG"
"$AAPT2" dump badging "$FIXTURE_APK" >"$MANIFEST_BADGING"
"$AAPT2" dump xmltree --file AndroidManifest.xml "$FIXTURE_APK" >"$MANIFEST_TREE"
"$AAPT2" dump resources "$FIXTURE_APK" >"$RESOURCES_DUMP"
"$AAPT2" dump xmltree --file res/layout/activity_main.xml "$FIXTURE_APK" >"$LAYOUT_TREE"

python3 - \
  "$FIXTURE_APK" \
  "$WORKSPACE_ROOT" \
  "$MANIFEST_BADGING" \
  "$MANIFEST_TREE" \
  "$RESOURCES_DUMP" \
  "$LAYOUT_TREE" \
  "$EXPECTED_APK_SHA256" <<'PY'
from hashlib import sha256
from pathlib import Path
import re
import struct
import sys
import zipfile
import zlib

fixture_path = Path(sys.argv[1]).resolve()
workspace_root = Path(sys.argv[2]).resolve()
badging = Path(sys.argv[3]).read_text(encoding="utf-8")
manifest_tree = Path(sys.argv[4]).read_text(encoding="utf-8")
resources_dump = Path(sys.argv[5]).read_text(encoding="utf-8")
layout_tree = Path(sys.argv[6]).read_text(encoding="utf-8")
expected_apk_sha256 = sys.argv[7]

expected_fixture = (
    workspace_root
    / "fixtures/androidbox-resource-demo/androidbox-resource-demo.apk"
).resolve()
if fixture_path != expected_fixture:
    raise SystemExit("the gate did not use the repository-owned Resources-1 fixture")
apk = fixture_path.read_bytes()
if sha256(apk).hexdigest() != expected_apk_sha256:
    raise SystemExit("the Resources-1 APK SHA-256 identity changed")

expected_entries = (
    "AndroidManifest.xml",
    "resources.arsc",
    "res/layout/activity_main.xml",
    "classes.dex",
)
expected_crcs = {
    "AndroidManifest.xml": 0x449B5321,
    "resources.arsc": 0x9F67C7B4,
    "res/layout/activity_main.xml": 0x36CE95C4,
    "classes.dex": 0x34F8448A,
}
with zipfile.ZipFile(fixture_path, "r") as archive:
    if archive.testzip() is not None:
        raise SystemExit("the Resources-1 APK ZIP CRC check failed")
    entries = archive.infolist()
    if tuple(entry.filename for entry in entries) != expected_entries:
        raise SystemExit(
            "Resources-1 must contain exactly the four canonical APK entries"
        )
    payloads = {}
    for entry in entries:
        if entry.compress_type != zipfile.ZIP_STORED:
            raise SystemExit(f"{entry.filename} is not STORED")
        if entry.flag_bits & 0x09:
            raise SystemExit(f"{entry.filename} uses encryption or a data descriptor")
        payload = archive.read(entry)
        payloads[entry.filename] = payload
        crc32 = zlib.crc32(payload) & 0xFFFFFFFF
        if crc32 != entry.CRC or crc32 != expected_crcs[entry.filename]:
            raise SystemExit(f"{entry.filename} CRC32 identity changed: {crc32:#010x}")

dex = payloads["classes.dex"]
if len(dex) < 112 or dex[:4] != b"dex\n" or dex[7] != 0:
    raise SystemExit("classes.dex has no supported DEX header")
if dex[4:7] not in {b"035", b"036", b"037", b"038", b"039", b"040", b"041"}:
    raise SystemExit(f"unsupported DEX version {dex[4:7]!r}")
if struct.unpack_from("<I", dex, 32)[0] != len(dex):
    raise SystemExit("DEX header file_size does not match the APK entry")
dex_adler32 = zlib.adler32(dex[12:]) & 0xFFFFFFFF
if dex_adler32 != struct.unpack_from("<I", dex, 8)[0] or dex_adler32 != 0x309F4ABE:
    raise SystemExit(f"classes.dex Adler32 identity changed: {dex_adler32:#010x}")

for exact in (
    "package: name='org.bndroid.demo'",
    "application-label:'AndroidBox Resources-1 Demo'",
    "application: label='AndroidBox Resources-1 Demo'",
    "launchable-activity: name='org.bndroid.demo.MainActivity'",
):
    if exact not in badging:
        raise SystemExit(f"aapt2 badging did not confirm {exact!r}")
if not re.search(
    r"android:label\(0x01010001\)=@0x7f030001(?:\s|$)", manifest_tree
):
    raise SystemExit("aapt2 did not confirm application @string/app_name")

resource_lines = tuple(
    line.strip()
    for line in resources_dump.splitlines()
    if line.lstrip().startswith("resource 0x")
)
if resource_lines != (
    "resource 0x7f010000 id/fixture_label",
    "resource 0x7f020000 layout/activity_main",
    "resource 0x7f030000 string/activity_message",
    "resource 0x7f030001 string/app_name",
):
    raise SystemExit("aapt2 resource IDs or names changed")
for exact in (
    '() "AndroidBox resource-backed view"',
    '() "AndroidBox Resources-1 Demo"',
):
    if exact not in resources_dump:
        raise SystemExit(f"aapt2 did not resolve resource value {exact!r}")

element_lines = tuple(
    line.strip()
    for line in layout_tree.splitlines()
    if line.lstrip().startswith("E: ")
)
if len(element_lines) != 1 or not element_lines[0].startswith("E: TextView "):
    raise SystemExit("aapt2 did not confirm the single TextView layout root")
for pattern, description in (
    (r"android:id\(0x010100d0\)=@0x7f010000(?:\s|$)", "TextView ID"),
    (r"android:text\(0x0101014f\)=@0x7f030000(?:\s|$)", "TextView text reference"),
):
    if not re.search(pattern, layout_tree):
        raise SystemExit(f"aapt2 did not confirm the {description}")

print(
    "ANDROIDBOX_RESOURCES1_STATIC_OK "
    f"apk_sha256={expected_apk_sha256} entries=4 stored=4 "
    "manifest_crc=0x449b5321 resources_crc=0x9f67c7b4 "
    "layout_crc=0x36ce95c4 dex_crc=0x34f8448a dex_adler=0x309f4abe "
    "layout_id=2130837504 string_id=2130903040"
)
PY

cleanup() {
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
  fi
}
normalize_log() {
  tr -d '\r' <"$SERIAL_LOG" >"$NORMALIZED_LOG"
}
show_failure() {
  normalize_log
  tail -n 200 "$NORMALIZED_LOG" >&2
  tail -n 100 "$QEMU_LOG" >&2
}
reject_failure() {
  if grep -Eqi 'fatal exception:|kernel panic:|panicked at|boot error:|MOBILE_UI_PREVIEW_(TIMEOUT|FAULT|RUNTIME_FAIL)|MOBILE_UI_(CHILD_DIAG|USER_FAULT)|FRAMEBUFFER_FAIL:|INPUT_FAIL:|UI_FAIL:|USER_FAIL:|EL0_FAIL' "$NORMALIZED_LOG"; then
    show_failure
    echo "The AndroidBox preview emitted a fatal runtime failure." >&2
    exit 1
  fi
}
wait_for_pattern() {
  local pattern="$1" description="$2" deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    normalize_log
    reject_failure
    if grep -Eq "$pattern" "$NORMALIZED_LOG"; then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      show_failure
      echo "QEMU exited while waiting for $description." >&2
      exit 1
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
}
line_count() {
  local pattern="$1"
  normalize_log
  grep -Ec "$pattern" "$NORMALIZED_LOG" || true
}
wait_for_count_at_least() {
  local pattern="$1" minimum="$2" description="$3"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    normalize_log
    reject_failure
    local now
    now=$(grep -Ec "$pattern" "$NORMALIZED_LOG" || true)
    if (( now >= minimum )); then
      return
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
      show_failure
      echo "QEMU exited while waiting for $description." >&2
      exit 1
    fi
    sleep 0.05
  done
  show_failure
  echo "Timed out waiting for $description." >&2
  exit 1
}
launcher_commit_count() {
  line_count "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} "
}
wait_for_launcher_commit_after() {
  local before="$1"
  wait_for_count_at_least \
    "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} " \
    "$((before + 1))" \
    "a new Launcher frame"
}
wait_for_stable_androidbox_frame() {
  local name="$1"
  local output="$ARTIFACT_DIR/$name.ppm"
  local comparison="$ARTIFACT_DIR/.$name.next.ppm"
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    "$QMP_HELPER" "$QMP_SOCKET" move 719 1599
    "$QMP_HELPER" "$QMP_SOCKET" screenshot "$output"
    sleep 0.05
    "$QMP_HELPER" "$QMP_SOCKET" screenshot "$comparison"
    if cmp -s "$output" "$comparison" && python3 - "$output" <<'PY'
from pathlib import Path
import sys

parts = Path(sys.argv[1]).read_bytes().split(b"\n", 3)
if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
    raise SystemExit(1)
pixels = parts[3]
if len(pixels) != 720 * 1600 * 3:
    raise SystemExit(1)

# A stable AndroidBox page has the teal execution-button field at this
# unadorned point. Home, Lock, Drawer, and intermediate transition frames
# cannot satisfy this exact raster predicate.
offset = (1300 * 720 + 80) * 3
raise SystemExit(0 if tuple(pixels[offset:offset + 3]) == (16, 118, 111) else 1)
PY
    then
      rm -f "$comparison"
      return
    fi
    sleep 0.05
  done
  rm -f "$comparison"
  show_failure
  echo "Timed out waiting for the stable $name AndroidBox frame." >&2
  exit 1
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qemu-system-aarch64 \
  -machine virt,gic-version=2,secure=off,virtualization=off \
  -cpu cortex-a72 -smp 1 -m 256M -display none -monitor none \
  -nic none \
  -rtc base=2026-07-29T09:41:00,clock=vm \
  -serial "file:$SERIAL_LOG" \
  -qmp "unix:$QMP_SOCKET,server=on,wait=off" \
  -no-reboot -kernel "$KERNEL_IMAGE" -device ramfb \
  "${BNDROID_STORAGE_QEMU_ARGS[@]}" \
  -device virtio-keyboard-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off \
  -device virtio-tablet-device,event_idx=off,indirect_desc=off,in_order=off,packed=off,queue_reset=off,wheel-axis=on \
  >"$QEMU_LOG" 2>&1 &
QEMU_PID=$!

wait_for_pattern '^MOBILE_UI_PREVIEW_OK ' "the isolated 720x1600 preview readiness marker"
wait_for_pattern '^ANDROIDBOX_DEX0_PROFILE_OK ' "the bounded AndroidBox profile marker"
wait_for_pattern '^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=[0-9]+ ' "the initial Launcher frame"
normalize_log
LAUNCHER_PID=$(awk '/^USER_SURFACE_BUFFER_COMMIT_OK / { for (i = 1; i <= NF; i++) if ($i ~ /^producer_pid=/) { sub(/^producer_pid=/, "", $i); print $i; exit } }' "$NORMALIZED_LOG")
[[ "$LAUNCHER_PID" =~ ^[1-9][0-9]*$ ]] || {
  echo "Could not derive the Launcher frame producer." >&2
  exit 1
}

"$QMP_HELPER" "$QMP_SOCKET" probe
wait_for_pattern '^POINTER_EVENT_OK ' "the canonical tablet probe"
wait_for_pattern '^USER_INPUT_READ_OK .* pressed=0 ' "the drained tablet probe"

# Unlock is a Launcher-to-SurfaceServer request. It is unrelated to Android
# credential security and only changes this preview session's visible mode.
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1200
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 800
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_pattern '^UI_SYSTEM_UI_REQUEST_OK .* action=unlock app=none request_id=1 ' "the authenticated preview unlock"
wait_for_pattern '^UI_SYSTEM_UI_CHANGED_OK .* mode=home recent_app=none nav_pressed=0 nav_reveal_px=0 ' "the unlocked Home state"

# Open All apps with the exact 240-output-pixel upward threshold.
launcher_before=$(launcher_commit_count)
"$QMP_HELPER" "$QMP_SOCKET" touch-down 360 1000
"$QMP_HELPER" "$QMP_SOCKET" touch-move 360 760
wait_for_launcher_commit_after "$launcher_before"
launcher_before=$(launcher_commit_count)
"$QMP_HELPER" "$QMP_SOCKET" touch-up
wait_for_launcher_commit_after "$launcher_before"

# AndroidBox is the fifth All apps item. Opening it runs the real DEX boot()
# code item, parses the binary manifest, selects its exported MAIN/LAUNCHER
# Activity, executes the real onCreate code item, and resolves the one
# allowlisted resources.arsc -> compiled layout -> TextView string path before
# the result-bearing transition is presented.
launcher_before=$(launcher_commit_count)
"$QMP_HELPER" "$QMP_SOCKET" tap 105 700
wait_for_pattern \
  "^ANDROIDBOX_DEX_EXECUTED_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=[1-9][0-9]* request=1 kind=boot crc=${EXPECTED_DEX_CRC32} adler=${EXPECTED_DEX_ADLER32} result=20260729 instructions=2 tap_count=0$" \
  "the authenticated boot() DEX execution"
wait_for_pattern \
  "^ANDROIDBOX_ACTIVITY_EXECUTED_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=[1-9][0-9]* request=1 lifecycle=on-create-complete manifest_verified=1 manifest_crc=${EXPECTED_MANIFEST_CRC32} dex_crc=${EXPECTED_DEX_CRC32} dex_adler=${EXPECTED_DEX_ADLER32} activity=org.bndroid.demo.MainActivity activity_descriptor_crc=2979227792 on_create_method=7 on_create_code_offset=712 instructions=4 view_text_id=resource-string view_text_label=9218 view_text_length=31 activitythread=0 framework=0 general_apk_claim=0 android_compatibility_claim=0$" \
  "the authenticated manifest-selected onCreate Activity execution"
wait_for_pattern \
  "^ANDROIDBOX_RESOURCES_RESOLVED_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=[1-9][0-9]* request=1 resources_arsc_crc=${EXPECTED_RESOURCES_ARSC_CRC32} layout_xml_crc=${EXPECTED_LAYOUT_XML_CRC32} layout_resource_id=${EXPECTED_LAYOUT_RESOURCE_ID} string_resource_id=${EXPECTED_STRING_RESOURCE_ID} view_text_id=resource-string view_text_label=9218 view_text_length=31 success_flags=31 table_parsed=1 layout_entry_resolved=1 binary_xml_parsed=1 text_view_verified=1 string_reference_resolved=1 resource_manager=0 arbitrary_layout_claim=0 general_apk_claim=0 android_compatibility_claim=0$" \
  "the authenticated compiled-resource resolution"
wait_for_count_at_least \
  "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} " \
  "$((launcher_before + 7))" \
  "the stable resource-backed Activity transition"
wait_for_stable_androidbox_frame androidbox-boot

# The page transition deliberately quarantines input until a release. A
# neutral tap clears that state without hitting an interactive target.
"$QMP_HELPER" "$QMP_SOCKET" tap 700 900

# The bounded onTap(I)I entry receives the preceding boot result and returns
# boot+7. Resource resolution is not repeated; the runtime reports the tap
# before presenting the updated result frame.
launcher_before=$(launcher_commit_count)
"$QMP_HELPER" "$QMP_SOCKET" tap 360 1312
wait_for_pattern \
  "^ANDROIDBOX_DEX_EXECUTED_OK sender_image=launcher sender_pid=${LAUNCHER_PID} receiver_image=surface-server receiver_pid=[1-9][0-9]* request=2 kind=tap crc=${EXPECTED_DEX_CRC32} adler=${EXPECTED_DEX_ADLER32} result=20260736 instructions=2 tap_count=1$" \
  "the authenticated onTap(I)I DEX execution"
wait_for_count_at_least \
  "^USER_SURFACE_BUFFER_COMMIT_OK .* producer_pid=${LAUNCHER_PID} " \
  "$((launcher_before + 3))" \
  "the stable tap-result frame"
wait_for_stable_androidbox_frame androidbox-tap

normalize_log
reject_failure
python3 - \
  "$FIXTURE_APK" \
  "$NORMALIZED_LOG" \
  "$ARTIFACT_DIR" \
  "$LAUNCHER_PID" \
  "$MANIFEST_BADGING" \
  "$MANIFEST_TREE" \
  "$RESOURCES_DUMP" \
  "$LAYOUT_TREE" \
  "$EXPECTED_APK_SHA256" <<'PY'
from hashlib import sha256
from pathlib import Path
import re
import struct
import sys
import zipfile
import zlib

fixture_path = Path(sys.argv[1]).resolve()
log_path = Path(sys.argv[2])
artifact_dir = Path(sys.argv[3])
launcher_pid = sys.argv[4]
manifest_badging_path = Path(sys.argv[5])
manifest_tree_path = Path(sys.argv[6])
resources_dump_path = Path(sys.argv[7])
layout_tree_path = Path(sys.argv[8])
expected_apk_sha256 = sys.argv[9]
workspace_root = artifact_dir.parents[2]

expected_fixture = (
    workspace_root
    / "fixtures/androidbox-resource-demo/androidbox-resource-demo.apk"
).resolve()
if fixture_path != expected_fixture:
    raise SystemExit("the gate did not use the repository-owned Resources-1 fixture")
apk = fixture_path.read_bytes()
if sha256(apk).hexdigest() != expected_apk_sha256:
    raise SystemExit("the Resources-1 APK SHA-256 identity changed")

with zipfile.ZipFile(fixture_path, "r") as archive:
    if archive.testzip() is not None:
        raise SystemExit("the APK ZIP CRC check failed")
    expected_entries = (
        "AndroidManifest.xml",
        "resources.arsc",
        "res/layout/activity_main.xml",
        "classes.dex",
    )
    entries = archive.infolist()
    if tuple(entry.filename for entry in entries) != expected_entries:
        raise SystemExit(
            "Resources-1 must contain exactly the four canonical APK entries"
        )
    expected_crcs = {
        "AndroidManifest.xml": 0x449B5321,
        "resources.arsc": 0x9F67C7B4,
        "res/layout/activity_main.xml": 0x36CE95C4,
        "classes.dex": 0x34F8448A,
    }
    payloads = {}
    entry_by_name = {}
    for entry in entries:
        if entry.compress_type != zipfile.ZIP_STORED:
            raise SystemExit(f"{entry.filename} is not STORED")
        if entry.flag_bits & 0x09:
            raise SystemExit(f"{entry.filename} uses encryption or a data descriptor")
        payload = archive.read(entry)
        payloads[entry.filename] = payload
        entry_by_name[entry.filename] = entry
        crc32 = zlib.crc32(payload) & 0xFFFFFFFF
        if crc32 != entry.CRC or crc32 != expected_crcs[entry.filename]:
            raise SystemExit(f"{entry.filename} CRC32 identity changed: {crc32:#010x}")

manifest = payloads["AndroidManifest.xml"]
resources_arsc = payloads["resources.arsc"]
layout_xml = payloads["res/layout/activity_main.xml"]
dex = payloads["classes.dex"]
manifest_entry = entry_by_name["AndroidManifest.xml"]
dex_entry = entry_by_name["classes.dex"]

manifest_crc32 = zlib.crc32(manifest) & 0xFFFFFFFF
if manifest_crc32 != manifest_entry.CRC or manifest_crc32 != 0x449B5321:
    raise SystemExit(
        f"binary manifest identity changed: crc={manifest_crc32:#010x}"
    )
resources_arsc_crc32 = zlib.crc32(resources_arsc) & 0xFFFFFFFF
layout_xml_crc32 = zlib.crc32(layout_xml) & 0xFFFFFFFF
badging = manifest_badging_path.read_text(encoding="utf-8")
manifest_tree = manifest_tree_path.read_text(encoding="utf-8")
resources_dump = resources_dump_path.read_text(encoding="utf-8")
layout_tree = layout_tree_path.read_text(encoding="utf-8")
for exact in (
    "package: name='org.bndroid.demo'",
    "application-label:'AndroidBox Resources-1 Demo'",
    "application: label='AndroidBox Resources-1 Demo'",
    "launchable-activity: name='org.bndroid.demo.MainActivity'",
):
    if exact not in badging:
        raise SystemExit(f"aapt2 badging did not confirm {exact!r}")
if not re.search(
    r"android:label\(0x01010001\)=@0x7f030001(?:\s|$)", manifest_tree
):
    raise SystemExit("aapt2 did not confirm application @string/app_name")
resource_lines = tuple(
    line.strip()
    for line in resources_dump.splitlines()
    if line.lstrip().startswith("resource 0x")
)
if resource_lines != (
    "resource 0x7f010000 id/fixture_label",
    "resource 0x7f020000 layout/activity_main",
    "resource 0x7f030000 string/activity_message",
    "resource 0x7f030001 string/app_name",
):
    raise SystemExit("aapt2 resource IDs or names changed")
for exact in (
    '() "AndroidBox resource-backed view"',
    '() "AndroidBox Resources-1 Demo"',
):
    if exact not in resources_dump:
        raise SystemExit(f"aapt2 did not resolve resource value {exact!r}")
element_lines = tuple(
    line.strip()
    for line in layout_tree.splitlines()
    if line.lstrip().startswith("E: ")
)
if len(element_lines) != 1 or not element_lines[0].startswith("E: TextView "):
    raise SystemExit("aapt2 did not confirm the single TextView layout root")
for pattern, description in (
    (r"android:id\(0x010100d0\)=@0x7f010000(?:\s|$)", "TextView ID"),
    (r"android:text\(0x0101014f\)=@0x7f030000(?:\s|$)", "TextView text reference"),
):
    if not re.search(pattern, layout_tree):
        raise SystemExit(f"aapt2 did not confirm the {description}")

if len(dex) < 112 or dex[:4] != b"dex\n" or dex[7] != 0:
    raise SystemExit("classes.dex has no supported DEX header")
if dex[4:7] not in {b"035", b"036", b"037", b"038", b"039", b"040", b"041"}:
    raise SystemExit(f"unsupported DEX version {dex[4:7]!r}")
if struct.unpack_from("<I", dex, 32)[0] != len(dex):
    raise SystemExit("DEX header file_size does not match the APK entry")

dex_crc32 = zlib.crc32(dex) & 0xFFFFFFFF
zip_crc32 = dex_entry.CRC
dex_adler32 = zlib.adler32(dex[12:]) & 0xFFFFFFFF
header_adler32 = struct.unpack_from("<I", dex, 8)[0]
if dex_crc32 != zip_crc32 or dex_adler32 != header_adler32:
    raise SystemExit("classes.dex CRC32 or Adler32 is not internally consistent")
if (dex_crc32, dex_adler32) != (0x34F8448A, 0x309F4ABE):
    raise SystemExit(
        f"fixture identity changed: crc={dex_crc32:#010x} adler={dex_adler32:#010x}"
    )

lines = log_path.read_text(encoding="utf-8").splitlines()

def field(line: str, name: str) -> str:
    match = re.search(rf"(?:^| ){re.escape(name)}=([^ ]+)", line)
    if not match:
        raise SystemExit(f"missing {name} in {line}")
    return match.group(1)

def all_lines(prefix: str) -> list[str]:
    return [line for line in lines if line.startswith(prefix)]

preview = all_lines("MOBILE_UI_PREVIEW_OK ")
if len(preview) != 1:
    raise SystemExit(f"expected one mobile preview marker, found {len(preview)}")
for name, expected in {
    "profile": "local-qemu",
    "abi": "24",
    "width": "720",
    "height": "1600",
    "design_width": "360",
    "design_height": "800",
    "scale": "2",
    "aspect": "20:9",
    "ui_client_control_version": "8",
    "ui_server_event_version": "6",
    "buffer_present_version": "6",
    "network": "disabled",
    "validation_scope": "ui-preview",
    "buffers": "4",
    "write_calls": "0",
    "write_bytes": "0",
    "presents": "1",
    "copy_path": "mapped-double-buffer",
    "rows_per_write": "0",
    "writes_per_frame": "0",
    "client_buffers_per_producer": "2",
    "frame_transaction_depth": "2",
    "transition_scheduling": "async-one-ahead",
    "stale_prepared_policy": "server-discard",
    "maps": "8",
    "map_successes": "8",
    "queues": "5",
    "queue_successes": "5",
    "acquires": "5",
    "acquire_successes": "5",
    "release_calls": "4",
    "release_successes": "4",
    "releases": "5",
    "mapped_presents": "1",
    "android_gesture_claim": "0",
    "background_execution_claim": "0",
    "frame_pacing": "software",
    "logical_timer_hz": "100",
    "software_frame_rate_hz": "50",
    "frame_divider": "2",
    "frame_acquires": "1",
    "frame_acquire_successes": "1",
    "frame_epoch": "1",
    "timer_pacing": "1",
    "hardware_vsync_claim": "0",
    "fps_claim": "0",
    "real_phone_claim": "0",
}.items():
    if field(preview[0], name) != expected:
        raise SystemExit(f"preview boundary {name} changed")
if field(preview[0], "frame_pending") not in {"0", "1"}:
    raise SystemExit("preview frame clock escaped its Waiting/Ready quiescent boundary")

address_spaces = all_lines("ASPACE_OK ")
if len(address_spaces) != 1 or field(address_spaces[0], "private_tables") != "14":
    raise SystemExit("AndroidBox DEX-0 preview did not use the mapped mobile address-space shape")
user_maps = all_lines("USER_MAP_OK ")
if len(user_maps) != 1 or field(user_maps[0], "stack_pages") != "8" \
        or field(user_maps[0], "guards_unmapped") != "2":
    raise SystemExit("AndroidBox DEX-0 stack workspace or two-guard contract changed")

created_buffers = all_lines("GRAPHICS_BUFFER_CREATE_OK ")
if len(created_buffers) != 4 or any(field(line, "mapped") != "1" for line in created_buffers):
    raise SystemExit("AndroidBox DEX-0 did not create exactly four mapped buffers")
maps = all_lines("GRAPHICS_BUFFER_MAP_OK ")
if len(maps) != 8 or {field(line, "pages") for line in maps} != {"1125"}:
    raise SystemExit("AndroidBox DEX-0 mapped buffer geometry changed")
if [field(line, "role") for line in maps].count("producer") != 4 \
        or [field(line, "role") for line in maps].count("consumer") != 4:
    raise SystemExit("AndroidBox DEX-0 producer/consumer mapping shape changed")
if {field(line, "address") for line in maps} != {
    "0x0000000200400000", "0x0000000200880000",
    "0x0000000200d00000", "0x0000000201180000",
}:
    raise SystemExit("AndroidBox DEX-0 mapped buffer addresses changed")

double_buffer = all_lines("MOBILE_DOUBLE_BUFFER_RUNTIME_OK ")
if len(double_buffer) != 1:
    raise SystemExit(f"expected one asynchronous double-buffer marker, found {len(double_buffer)}")
for name, expected in {
    "format": "1", "client_buffers_per_producer": "2", "producer_count": "2",
    "transaction_depth": "2", "scheduling": "async-one-ahead",
    "stale_prepared_policy": "server-discard", "counter_overflowed": "0",
}.items():
    if field(double_buffer[0], name) != expected:
        raise SystemExit(f"AndroidBox double-buffer boundary {name} changed")
if int(field(double_buffer[0], "peak_queued")) < 2 \
        or int(field(double_buffer[0], "peak_in_flight")) < 2 \
        or int(field(double_buffer[0], "dual_in_flight_publications")) < 1:
    raise SystemExit("AndroidBox transition never held two bounded frames in flight")

profiles = all_lines("ANDROIDBOX_DEX0_PROFILE_OK ")
if len(profiles) != 1:
    raise SystemExit(f"expected one AndroidBox profile marker, found {len(profiles)}")
for name, expected in {
    "format": "3",
    "apk_source": "launcher-readonly-rodata",
    "apk_install": "0",
    "apk_signature_check": "0",
    "manifest_parser": "binary-bounded",
    "manifest_label": "resource-string",
    "manifest_code_execution": "0",
    "dex_interpreter": "bounded-integer-activity-resource-subset",
    "activity_lifecycle": "constructor-then-onCreate",
    "constructor_required": "1",
    "framework_shim": "Activity+TextView+setContentView-int-only",
    "activity_descriptor": "org.bndroid.demo.MainActivity",
    "activity_report_protocol": "ui-client-control-v8",
    "activity_report_count": "one-per-surface-session",
    "resource_parser": "bounded-default-config",
    "resource_layout": "binary-TextView-only",
    "resource_report_protocol": "ui-client-control-v8",
    "resource_report_count": "one-per-surface-session",
    "view_text_id": "resource-string",
    "resource_manager": "0",
    "qualifiers": "0",
    "aliases": "0",
    "arbitrary_layout_claim": "0",
    "art": "0",
    "dalvik": "0",
    "activitythread": "0",
    "framework": "0",
    "binder": "0",
    "bionic": "0",
    "jni": "0",
    "native_lib": "0",
    "general_apk_claim": "0",
    "android_compatibility_claim": "0",
    "network": "disabled",
    "emulator_only": "1",
    "real_phone_claim": "0",
}.items():
    if field(profiles[0], name) != expected:
        raise SystemExit(f"AndroidBox profile boundary {name} changed")

reports = all_lines("ANDROIDBOX_DEX_EXECUTED_OK ")
if len(reports) != 2:
    raise SystemExit(f"expected exactly two AndroidBox reports, found {len(reports)}")
expected_reports = [
    {
        "request": "1",
        "kind": "boot",
        "result": "20260729",
        "instructions": "2",
        "tap_count": "0",
    },
    {
        "request": "2",
        "kind": "tap",
        "result": "20260736",
        "instructions": "2",
        "tap_count": "1",
    },
]
receiver_pid = None
for report, expected in zip(reports, expected_reports, strict=True):
    for name, value in expected.items():
        if field(report, name) != value:
            raise SystemExit(f"AndroidBox report {name}: expected {value}, observed {field(report, name)}")
    if field(report, "sender_image") != "launcher":
        raise SystemExit("AndroidBox report did not originate from Launcher")
    if field(report, "sender_pid") != launcher_pid:
        raise SystemExit("AndroidBox sender PID is not the rendered Launcher PID")
    if field(report, "receiver_image") != "surface-server":
        raise SystemExit("AndroidBox report did not terminate at SurfaceServer")
    current_receiver_pid = field(report, "receiver_pid")
    if not current_receiver_pid.isdecimal() or int(current_receiver_pid) <= 0:
        raise SystemExit("AndroidBox report has no positive SurfaceServer PID")
    if receiver_pid is None:
        receiver_pid = current_receiver_pid
    elif receiver_pid != current_receiver_pid:
        raise SystemExit("AndroidBox reports reached different SurfaceServer processes")
    if int(field(report, "crc")) != dex_crc32:
        raise SystemExit("AndroidBox report CRC32 does not identify fixture classes.dex")
    if int(field(report, "adler")) != dex_adler32:
        raise SystemExit("AndroidBox report Adler32 does not identify fixture classes.dex")

activity_reports = all_lines("ANDROIDBOX_ACTIVITY_EXECUTED_OK ")
if len(activity_reports) != 1:
    raise SystemExit(
        f"expected exactly one AndroidBox Activity report, found {len(activity_reports)}"
    )
activity = activity_reports[0]
for name, expected in {
    "request": "1",
    "lifecycle": "on-create-complete",
    "manifest_verified": "1",
    "manifest_crc": str(manifest_crc32),
    "dex_crc": str(dex_crc32),
    "dex_adler": str(dex_adler32),
    "activity": "org.bndroid.demo.MainActivity",
    "activity_descriptor_crc": str(0xB1936890),
    "on_create_method": "7",
    "on_create_code_offset": str(0x2C8),
    "instructions": "4",
    "view_text_id": "resource-string",
    "view_text_label": str(0x2402),
    "view_text_length": "31",
    "activitythread": "0",
    "framework": "0",
    "general_apk_claim": "0",
    "android_compatibility_claim": "0",
}.items():
    if field(activity, name) != expected:
        raise SystemExit(
            f"AndroidBox Activity report {name}: expected {expected}, "
            f"observed {field(activity, name)}"
        )
if field(activity, "sender_image") != "launcher":
    raise SystemExit("AndroidBox Activity report did not originate from Launcher")
if field(activity, "sender_pid") != launcher_pid:
    raise SystemExit("AndroidBox Activity sender PID is not the rendered Launcher PID")
if field(activity, "receiver_image") != "surface-server":
    raise SystemExit("AndroidBox Activity report did not terminate at SurfaceServer")
if field(activity, "receiver_pid") != receiver_pid:
    raise SystemExit("AndroidBox Activity and DEX reports reached different SurfaceServers")

resource_reports = all_lines("ANDROIDBOX_RESOURCES_RESOLVED_OK ")
if len(resource_reports) != 1:
    raise SystemExit(
        f"expected exactly one AndroidBox resource report, found {len(resource_reports)}"
    )
resource = resource_reports[0]
for name, expected in {
    "request": "1",
    "resources_arsc_crc": str(resources_arsc_crc32),
    "layout_xml_crc": str(layout_xml_crc32),
    "layout_resource_id": str(0x7F020000),
    "string_resource_id": str(0x7F030000),
    "view_text_id": "resource-string",
    "view_text_label": str(0x2402),
    "view_text_length": "31",
    "success_flags": "31",
    "table_parsed": "1",
    "layout_entry_resolved": "1",
    "binary_xml_parsed": "1",
    "text_view_verified": "1",
    "string_reference_resolved": "1",
    "resource_manager": "0",
    "arbitrary_layout_claim": "0",
    "general_apk_claim": "0",
    "android_compatibility_claim": "0",
}.items():
    if field(resource, name) != expected:
        raise SystemExit(
            f"AndroidBox resource report {name}: expected {expected}, "
            f"observed {field(resource, name)}"
        )
if field(resource, "sender_image") != "launcher":
    raise SystemExit("AndroidBox resource report did not originate from Launcher")
if field(resource, "sender_pid") != launcher_pid:
    raise SystemExit("AndroidBox resource sender PID is not the rendered Launcher PID")
if field(resource, "receiver_image") != "surface-server":
    raise SystemExit("AndroidBox resource report did not terminate at SurfaceServer")
if field(resource, "receiver_pid") != receiver_pid:
    raise SystemExit("AndroidBox resource and DEX reports reached different SurfaceServers")

if not (
    lines.index(profiles[0])
    < lines.index(reports[0])
    < lines.index(activity)
    < lines.index(resource)
    < lines.index(reports[1])
):
    raise SystemExit("AndroidBox readiness/execution ordering changed")

commits = all_lines("USER_SURFACE_BUFFER_COMMIT_OK ")
frame_acquires = all_lines("SURFACE_FRAME_ACQUIRE_OK ")
frame_commits = all_lines("SURFACE_FRAME_COMMIT_OK ")
if len(frame_acquires) != len(commits) or len(frame_commits) != len(commits):
    raise SystemExit(
        "every accepted AndroidBox buffer commit must own exactly one software-frame grant"
    )
previous_boundary = 0
damage_commits = 0
visible_damage_commits = 0
damage_pixels_total = 0
max_damage_pixels = 0
min_visible_damage_pixels = None
previous_scene_digest = None

def verified_damage(line):
    try:
        count = int(field(line, "damage_rects"))
        raw = [tuple(map(int, field(line, f"damage{index}").split("/")))
               for index in range(2)]
        global_rect = tuple(map(int, field(line, "global_damage").split("/")))
        composition = tuple(map(int, field(line, "composition").split("/")))
    except ValueError as error:
        raise SystemExit("AndroidBox emitted malformed multi-region damage") from error
    if count not in (1, 2) or any(len(rect) != 4 for rect in raw):
        raise SystemExit("AndroidBox emitted an invalid damage-region count")
    if count == 1 and raw[1] != (0, 0, 0, 0):
        raise SystemExit("AndroidBox single-region commit exposed a nonzero second slot")
    rects = raw[:count]
    for x, y, width, height in rects:
        if width <= 0 or height <= 0 or x < 0 or y < 0 \
                or x + width > 720 or y + height > 1600:
            raise SystemExit("AndroidBox buffer damage escaped the physical surface")
    if rects != sorted(rects, key=lambda rect: (rect[1], rect[0], rect[3], rect[2])):
        raise SystemExit("AndroidBox damage regions are not canonically ordered")
    if count == 2:
        ax, ay, aw, ah = rects[0]
        bx, by, bw, bh = rects[1]
        if ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah:
            raise SystemExit("AndroidBox damage regions overlap")
    left = min(rect[0] for rect in rects)
    top = min(rect[1] for rect in rects)
    right = max(rect[0] + rect[2] for rect in rects)
    bottom = max(rect[1] + rect[3] for rect in rects)
    bounds = (left, top, right - left, bottom - top)
    pixels = sum(rect[2] * rect[3] for rect in rects)
    if global_rect != bounds:
        raise SystemExit("AndroidBox global damage is not the exact region-set bound")
    if int(field(line, "damage_pixels")) != pixels \
            or int(field(line, "raster_writes")) != pixels:
        raise SystemExit("AndroidBox raster writes do not equal the disjoint damage pixels")
    cx, cy, cw, ch = composition
    if cw <= 0 or ch <= 0 or cx < 0 or cy < 0 \
            or cx + cw > 720 or cy + ch > 1600 \
            or cx > left or cy > top or cx + cw < right or cy + ch < bottom:
        raise SystemExit("AndroidBox compositor evidence does not contain the damage regions")
    composition_rects = int(field(line, "composition_rects"))
    composition_pixels = int(field(line, "composition_pixels"))
    if composition_rects not in (count, count + 1) \
            or not pixels <= composition_pixels <= pixels + 12 * 22:
        raise SystemExit("AndroidBox cursor-preserving composition accounting changed")
    return field(line, "mode"), bounds, pixels

for number, (line, acquire, frame_commit) in enumerate(
    zip(commits, frame_acquires, frame_commits, strict=True), 1
):
    boundary = int(field(acquire, "boundary"))
    if int(field(acquire, "epoch")) != number or int(field(frame_commit, "epoch")) != number:
        raise SystemExit("AndroidBox software-frame epochs are not contiguous")
    if boundary <= previous_boundary or (previous_boundary and boundary - previous_boundary < 2):
        raise SystemExit("AndroidBox commits crossed fewer than two 100 Hz logical ticks")
    if any(
        field(evidence, name) != field(line, name)
        for evidence in (acquire, frame_commit)
        for name in ("pid", "session")
    ):
        raise SystemExit("AndroidBox frame evidence escaped its SurfaceServer session")
    if field(frame_commit, "frame_id") != field(line, "frame_id"):
        raise SystemExit("AndroidBox frame commit identified a different global frame")
    if field(frame_commit, "client_buffer_slot") != field(line, "client_buffer_slot"):
        raise SystemExit("AndroidBox frame commit identified a different client slot")
    if not lines.index(acquire) < lines.index(line) < lines.index(frame_commit):
        raise SystemExit("AndroidBox frame grant/commit publication ordering changed")
    mode, (x, y, width, height), damage_pixels = verified_damage(line)
    if mode == "full":
        if (x, y, width, height) != (0, 0, 720, 1600) \
                or damage_pixels != 720 * 1600:
            raise SystemExit("AndroidBox full frame did not cover the complete surface")
    elif mode == "damage":
        if damage_pixels >= 720 * 1600 // 2:
            raise SystemExit(
                f"AndroidBox damage was not component-tight: pixels={damage_pixels} "
                f"bounds={x}/{y}/{width}/{height}"
            )
        damage_commits += 1
        damage_pixels_total += damage_pixels
        max_damage_pixels = max(max_damage_pixels, damage_pixels)
        if previous_scene_digest is not None \
                and field(line, "scene_digest") != previous_scene_digest:
            visible_damage_commits += 1
            min_visible_damage_pixels = (
                damage_pixels
                if min_visible_damage_pixels is None
                else min(min_visible_damage_pixels, damage_pixels)
            )
    else:
        raise SystemExit(f"unknown AndroidBox buffer-present mode {mode}")
    if number == 1 and mode != "full":
        raise SystemExit("AndroidBox initial committed frame must establish a full base")
    previous_boundary = boundary
    previous_scene_digest = field(line, "scene_digest")
if damage_commits == 0:
    raise SystemExit("AndroidBox interaction never exercised a real damage transaction")
if visible_damage_commits == 0:
    raise SystemExit("AndroidBox damage transactions never changed the committed UI scene")
if min_visible_damage_pixels is None or min_visible_damage_pixels > 100_000:
    raise SystemExit(
        "no visible AndroidBox interaction used a component-sized damage rectangle"
    )

acquisitions = all_lines("GRAPHICS_BUFFER_ACQUIRE_OK ")
discard_releases = all_lines("GRAPHICS_BUFFER_RELEASE_OK ")
if len(acquisitions) != len(commits) + len(discard_releases):
    raise SystemExit("every AndroidBox buffer acquisition must commit or explicitly discard")
allocation_slots = {}
for line in created_buffers:
    allocation_slots.setdefault(field(line, "producer_pid"), []).append(
        int(field(line, "slot"))
    )
if len(allocation_slots) != 2 or any(len(slots) != 2 for slots in allocation_slots.values()):
    raise SystemExit("AndroidBox Launcher and App must each own two allocation identities")
for producer, slots in allocation_slots.items():
    observed = [
        int(field(line, "allocation_slot"))
        for line in acquisitions
        if field(line, "producer_pid") == producer
    ]
    if not observed or any(slot not in slots for slot in observed):
        raise SystemExit("an AndroidBox acquisition escaped its producer's two slots")
    if any(left == right for left, right in zip(observed, observed[1:])):
        raise SystemExit("an AndroidBox producer did not alternate acquisition slots")

pending = None
for index, line in enumerate(lines):
    if line.startswith("GRAPHICS_BUFFER_ACQUIRE_OK "):
        if pending is not None:
            raise SystemExit("AndroidBox began a second acquisition before completing the first")
        pending = (index, line)
    elif line.startswith("GRAPHICS_BUFFER_RELEASE_OK "):
        if pending is None:
            raise SystemExit("AndroidBox discard release had no matching acquisition")
        acquire_index, acquire = pending
        for name in ("consumer_pid", "producer_pid", "allocation_slot",
                     "allocation_generation", "buffer_generation"):
            if field(acquire, name) != field(line, name):
                raise SystemExit(f"AndroidBox discard changed acquired {name}")
        if any(candidate.startswith("SURFACE_FRAME_ACQUIRE_OK ")
               for candidate in lines[acquire_index + 1:index]):
            raise SystemExit("AndroidBox discarded frame consumed a display-frame grant")
        pending = None
    elif line.startswith("USER_SURFACE_BUFFER_COMMIT_OK "):
        if pending is None:
            raise SystemExit("AndroidBox visible commit had no matching acquisition")
        _, acquire = pending
        if field(acquire, "producer_pid") != field(line, "producer_pid") \
                or field(acquire, "buffer_generation") != field(line, "buffer_generation"):
            raise SystemExit("AndroidBox commit changed its acquired producer or generation")
        slots = allocation_slots[field(line, "producer_pid")]
        logical_slot = slots.index(int(field(acquire, "allocation_slot")))
        if logical_slot != int(field(line, "client_buffer_slot")):
            raise SystemExit("AndroidBox client slot selected the wrong allocation")
        pending = None
if pending is not None:
    raise SystemExit("AndroidBox final buffer acquisition was left incomplete")
launcher_commits = [
    line for line in commits if field(line, "producer_pid") == launcher_pid
]
if len(launcher_commits) < 12:
    raise SystemExit(f"expected a nontrivial Launcher interaction, found {len(launcher_commits)} commits")
for line in launcher_commits:
    if (field(line, "width"), field(line, "height")) != ("720", "1600"):
        raise SystemExit("Launcher scanout geometry changed")
    if field(line, "format") != "xrgb8888":
        raise SystemExit("Launcher pixel format changed")
    if field(line, "mode") not in {"full", "damage"}:
        raise SystemExit("Launcher buffer-present mode changed")

def ppm(name: str) -> bytes:
    parts = (artifact_dir / name).read_bytes().split(b"\n", 3)
    if len(parts) != 4 or parts[:3] != [b"P6", b"720 1600", b"255"]:
        raise SystemExit(f"{name} is not an exact 720x1600 PPM")
    if len(parts[3]) != 720 * 1600 * 3:
        raise SystemExit(f"{name} has an invalid pixel payload")
    return parts[3]

boot = ppm("androidbox-boot.ppm")
tap = ppm("androidbox-tap.ppm")

def pixel(image: bytes, x: int, y: int) -> tuple[int, int, int]:
    offset = (y * 720 + x) * 3
    return tuple(image[offset:offset + 3])

androidbox_color = (16, 118, 111)
for name, image in (("boot", boot), ("tap", tap)):
    if pixel(image, 80, 1300) != androidbox_color:
        raise SystemExit(f"{name} screenshot is not the stable AndroidBox page")
    for x, y in (
        (54, 880),
        (54, 924),
        (54, 968),
        (54, 1012),
        (54, 1056),
    ):
        if pixel(image, x, y) != androidbox_color:
            raise SystemExit(
                f"{name} screenshot lacks the complete resource-success indicators"
            )
    colors = {image[offset:offset + 3] for offset in range(0, len(image), 3)}
    if len(colors) < 32:
        raise SystemExit(f"{name} screenshot is unexpectedly flat")
    teal_pixels = sum(
        image[offset:offset + 3] == bytes(androidbox_color)
        for offset in range(0, len(image), 3)
    )
    if teal_pixels < 40_000:
        raise SystemExit(f"{name} screenshot lacks the AndroidBox execution surface")

changed = []
for index in range(720 * 1600):
    offset = index * 3
    if boot[offset:offset + 3] != tap[offset:offset + 3]:
        changed.append((index % 720, index // 720))
if not 100 <= len(changed) <= 5_000:
    raise SystemExit(f"boot/tap screenshot change count is implausible: {len(changed)}")

result_changes = sum(190 <= x < 326 and 1180 <= y < 1228 for x, y in changed)
tap_count_changes = sum(628 <= x < 658 and 1180 <= y < 1228 for x, y in changed)
if result_changes < 40 or tap_count_changes < 20:
    raise SystemExit("updated runtime result and tap count are not both visible")
if any(
    not (
        (190 <= x < 326 and 1180 <= y < 1228)
        or (628 <= x < 658 and 1180 <= y < 1228)
    )
    for x, y in changed
):
    raise SystemExit("tap changed pixels outside the two runtime-owned value fields")

summary = {
    "fixture_apk_sha256": sha256(apk).hexdigest(),
    "fixture_entries": 4,
    "fixture_stored_entries": 4,
    "manifest_sha256": sha256(manifest).hexdigest(),
    "manifest_bytes": len(manifest),
    "manifest_crc32": f"0x{manifest_crc32:08x}",
    "application_label": "AndroidBox_Resources-1_Demo",
    "launcher_activity": "org.bndroid.demo.MainActivity",
    "resources_arsc_sha256": sha256(resources_arsc).hexdigest(),
    "resources_arsc_bytes": len(resources_arsc),
    "resources_arsc_crc32": f"0x{resources_arsc_crc32:08x}",
    "layout_xml_sha256": sha256(layout_xml).hexdigest(),
    "layout_xml_bytes": len(layout_xml),
    "layout_xml_crc32": f"0x{layout_xml_crc32:08x}",
    "layout_resource_id": 0x7F020000,
    "string_resource_id": 0x7F030000,
    "classes_dex_sha256": sha256(dex).hexdigest(),
    "classes_dex_bytes": len(dex),
    "classes_dex_crc32": f"0x{dex_crc32:08x}",
    "classes_dex_adler32": f"0x{dex_adler32:08x}",
    "launcher_pid": launcher_pid,
    "surface_server_pid": receiver_pid,
    "launcher_commits": len(launcher_commits),
    "frame_acquires": len(frame_acquires),
    "damage_commits": damage_commits,
    "visible_damage_commits": visible_damage_commits,
    "damage_pixels_total": damage_pixels_total,
    "max_damage_pixels": max_damage_pixels,
    "min_visible_damage_pixels": min_visible_damage_pixels,
    "androidbox_reports": len(reports),
    "activity_reports": len(activity_reports),
    "resource_reports": len(resource_reports),
    "resource_success_flags": 31,
    "on_create_method": 7,
    "on_create_code_offset": "0x2c8",
    "on_create_instructions": 4,
    "text_view": "AndroidBox_resource-backed_view",
    "boot_result": 20260729,
    "tap_result": 20260736,
    "tap_count": 1,
    "boot_pixel_payload_sha256": sha256(boot).hexdigest(),
    "tap_pixel_payload_sha256": sha256(tap).hexdigest(),
    "changed_pixels": len(changed),
}
(artifact_dir / "summary.txt").write_text(
    "\n".join(f"{key}={value}" for key, value in summary.items()) + "\n",
    encoding="utf-8",
)
print("ANDROIDBOX_RESOURCES1_QEMU_CHECK_OK " + " ".join(f"{key}={value}" for key, value in summary.items()))
PY

echo "AndroidBox Resources-1 QEMU evidence: $ARTIFACT_DIR"
