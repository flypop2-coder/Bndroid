#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && /bin/pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-envelope-demo"
SIGNING_DIR="$ROOT/fixtures/androidbox-resource-demo"
SDK_ROOT=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}
BUILD_TOOLS="$SDK_ROOT/build-tools/36.1.0"
AAPT2="$BUILD_TOOLS/aapt2"
D8="$BUILD_TOOLS/d8"
DEXDUMP="$BUILD_TOOLS/dexdump"
ZIPALIGN="$BUILD_TOOLS/zipalign"
APKSIGNER="$BUILD_TOOLS/apksigner"
ANDROID_JAR="$SDK_ROOT/platforms/android-36/android.jar"
JAVAC=/usr/bin/javac
JAR=/usr/bin/jar
PYTHON3=/usr/bin/python3
ZIP=/usr/bin/zip
UNZIP=/usr/bin/unzip
SIGNING_KEY="$SIGNING_DIR/test-only-apk-signing-key.pk8"
SIGNING_CERT="$SIGNING_DIR/test-only-apk-signing-cert.pem"
EXPECTED_CERT_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
VERSION_CODE=${BNDROID_ENVELOPE_VERSION_CODE:-1}
VERSION_NAME=${BNDROID_ENVELOPE_VERSION_NAME:-1.0}
OUTPUT_APK=${BNDROID_ENVELOPE_OUTPUT_APK:-"$FIXTURE_DIR/androidbox-envelope-demo.apk"}
ICON_RESOURCES=${BNDROID_ANDROID_ICON_RESOURCES:-0}
DEX_METHODS=${BNDROID_ANDROID_DEX_METHODS:-0}
LAYOUT_ROWS=${BNDROID_ANDROID_LAYOUT_ROWS:-0}

case "$VERSION_CODE/$VERSION_NAME" in
    1/1.0|2/2.0) ;;
    *) echo "supported envelope versions are exactly 1/1.0 or 2/2.0" >&2; exit 1 ;;
esac
case "$ICON_RESOURCES" in
    0|1|2) ;;
    *) echo "BNDROID_ANDROID_ICON_RESOURCES must be exactly 0, 1, or 2" >&2; exit 1 ;;
esac
case "$DEX_METHODS" in
    0|1|2|3|4|5|6) ;;
    *) echo "BNDROID_ANDROID_DEX_METHODS must be exactly 0, 1, 2, 3, 4, 5, or 6" >&2; exit 1 ;;
esac
case "$LAYOUT_ROWS" in
    0|1|2|3|4|5|6) ;;
    *) echo "BNDROID_ANDROID_LAYOUT_ROWS must be exactly 0, 1, 2, 3, 4, 5, or 6" >&2; exit 1 ;;
esac
case "$OUTPUT_APK" in
    "$FIXTURE_DIR/androidbox-envelope-demo.apk")
        [ "$VERSION_CODE/$VERSION_NAME" = 1/1.0 ] \
            || { echo "the canonical fixture output is reserved for version 1" >&2; exit 1; }
        [ "$ICON_RESOURCES" = 0 ] \
            || { echo "the historical canonical fixture does not carry the ABI 56 icon" >&2; exit 1; }
        [ "$DEX_METHODS" = 0 ] \
            || { echo "the historical canonical fixture does not use alternate app methods" >&2; exit 1; }
        [ "$LAYOUT_ROWS" = 0 ] \
            || { echo "the historical canonical fixture does not use nested layout rows" >&2; exit 1; }
        ;;
    "$ROOT"/target/androidbox-runtime-install2/*|"$ROOT"/target/androidbox-multipackage4/*|"$ROOT"/target/androidbox-multipackage4-*/*|"$ROOT"/target/androidbox-icon-resources5/*|"$ROOT"/target/androidbox-density-icons7/*|"$ROOT"/target/androidbox-dex-methods8/*|"$ROOT"/target/androidbox-dex-instance9/*|"$ROOT"/target/androidbox-activity-fields10/*|"$ROOT"/target/activity-fields10/*|"$ROOT"/target/activity-state11/*|"$ROOT"/target/string-text12/*|"$ROOT"/target/androidbox-string-text12/*|"$ROOT"/target/string-builder13/*|"$ROOT"/target/androidbox-string-builder13/*|"$ROOT"/target/layout-row14/*|"$ROOT"/target/androidbox-layout-row14/*|"$ROOT"/target/layout-weight15/*|"$ROOT"/target/androidbox-layout-weight15/*|"$ROOT"/target/layout-spacing16/*|"$ROOT"/target/androidbox-layout-spacing16/*|"$ROOT"/target/layout-directional17/*|"$ROOT"/target/androidbox-layout-directional17/*|"$ROOT"/target/layout-size18/*|"$ROOT"/target/androidbox-layout-size18/*|"$ROOT"/target/layout-mixed19/*|"$ROOT"/target/androidbox-layout-mixed19/*) ;;
    *) echo "custom envelope output must stay in target/androidbox-runtime-install2" >&2; exit 1 ;;
esac

case "$SDK_ROOT" in
    /*) ;;
    *) echo "Android SDK root must be absolute: $SDK_ROOT" >&2; exit 1 ;;
esac
for tool in \
    "$AAPT2" "$D8" "$DEXDUMP" "$ZIPALIGN" "$APKSIGNER" \
    "$JAVAC" "$JAR" "$PYTHON3" "$ZIP" "$UNZIP" /usr/bin/base64; do
    [ -x "$tool" ] || { echo "missing offline build tool: $tool" >&2; exit 1; }
done
[ -f "$ANDROID_JAR" ] || { echo "missing Android platform jar" >&2; exit 1; }
[ -f "$SIGNING_KEY" ] && [ -f "$SIGNING_CERT" ] \
    || { echo "missing test-only signing material" >&2; exit 1; }

WORK_DIR=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/bndroid-envelope-apk.XXXXXX")
cleanup() {
    if [ "${KEEP_BUILD_DIR:-0}" = 1 ]; then
        echo "androidbox_envelope_demo_build_dir=$WORK_DIR"
    else
        /bin/rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT HUP INT TERM
/bin/mkdir -p \
    "$WORK_DIR/compiled" "$WORK_DIR/generated" "$WORK_DIR/classes" \
    "$WORK_DIR/dex" "$WORK_DIR/link" "$WORK_DIR/stage" "$WORK_DIR/tmp" \
    "$WORK_DIR/res/drawable" "$WORK_DIR/res/drawable-mdpi" \
    "$WORK_DIR/res/drawable-xhdpi" "$WORK_DIR/res/layout" "$WORK_DIR/res/values"
export LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=946684800 TMPDIR="$WORK_DIR/tmp"

LAYOUT_SOURCE="$FIXTURE_DIR/res/layout/activity_envelope.xml"
if [ "$LAYOUT_ROWS" = 1 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-row/layout/activity_envelope.xml"
elif [ "$LAYOUT_ROWS" = 2 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-weight/layout/activity_envelope.xml"
elif [ "$LAYOUT_ROWS" = 3 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-spacing/layout/activity_envelope.xml"
elif [ "$LAYOUT_ROWS" = 4 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-directional/layout/activity_envelope.xml"
elif [ "$LAYOUT_ROWS" = 5 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-size/layout/activity_envelope.xml"
elif [ "$LAYOUT_ROWS" = 6 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-mixed/layout/activity_envelope.xml"
fi
/bin/cp "$LAYOUT_SOURCE" "$WORK_DIR/res/layout/activity_envelope.xml"
/bin/cp "$FIXTURE_DIR/res/values/strings.xml" "$WORK_DIR/res/values/"
MANIFEST="$FIXTURE_DIR/AndroidManifest.xml"
if [ "$ICON_RESOURCES" = 1 ]; then
    MANIFEST="$FIXTURE_DIR/AndroidManifest.icon.xml"
    /usr/bin/base64 -D \
        -i "$FIXTURE_DIR/res/drawable/app_icon.png.b64" \
        -o "$WORK_DIR/res/drawable/app_icon.png"
elif [ "$ICON_RESOURCES" = 2 ]; then
    MANIFEST="$FIXTURE_DIR/AndroidManifest.icon.xml"
    /usr/bin/base64 -D \
        -i "$FIXTURE_DIR/res/drawable/app_icon_mdpi.png.b64" \
        -o "$WORK_DIR/res/drawable-mdpi/app_icon.png"
    /usr/bin/base64 -D \
        -i "$FIXTURE_DIR/res/drawable/app_icon_xhdpi.png.b64" \
        -o "$WORK_DIR/res/drawable-xhdpi/app_icon.png"
fi
"$AAPT2" compile --no-crunch --dir "$WORK_DIR/res" -o "$WORK_DIR/compiled"
set -- \
    "$WORK_DIR/compiled/layout_activity_envelope.xml.flat" \
    "$WORK_DIR/compiled/values_strings.arsc.flat"
if [ "$ICON_RESOURCES" = 1 ]; then
    set -- "$WORK_DIR/compiled/drawable_app_icon.png.flat" "$@"
elif [ "$ICON_RESOURCES" = 2 ]; then
    set -- \
        "$WORK_DIR/compiled/drawable-mdpi_app_icon.png.flat" \
        "$WORK_DIR/compiled/drawable-xhdpi_app_icon.png.flat" \
        "$@"
fi
"$AAPT2" link \
    -o "$WORK_DIR/link/resources.apk" \
    --manifest "$MANIFEST" \
    -I "$ANDROID_JAR" \
    --java "$WORK_DIR/generated" \
    --min-sdk-version 26 \
    --target-sdk-version 36 \
    --version-code "$VERSION_CODE" \
    --version-name "$VERSION_NAME" \
    "$@"

ACTIVITY_SOURCE="$FIXTURE_DIR/src/org/bndroid/envelope/MainActivity.java"
if [ "$DEX_METHODS" = 1 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-methods/org/bndroid/envelope/MainActivity.java"
elif [ "$DEX_METHODS" = 2 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-instance/org/bndroid/envelope/MainActivity.java"
elif [ "$DEX_METHODS" = 3 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-fields/org/bndroid/envelope/MainActivity.java"
elif [ "$DEX_METHODS" = 4 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-state/org/bndroid/envelope/MainActivity.java"
elif [ "$DEX_METHODS" = 5 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-string/org/bndroid/envelope/MainActivity.java"
elif [ "$DEX_METHODS" = 6 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-concat/org/bndroid/envelope/MainActivity.java"
fi
"$JAVAC" \
    --release 8 -g:none -Xlint:-options -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$WORK_DIR/classes" \
    "$ACTIVITY_SOURCE" \
    "$WORK_DIR/generated/org/bndroid/envelope/R.java"

"$D8" \
    --release --min-api 26 --lib "$ANDROID_JAR" \
    --output "$WORK_DIR/dex" \
    "$WORK_DIR/classes/org/bndroid/envelope/MainActivity.class" \
    "$WORK_DIR/classes/org/bndroid/envelope/R.class" \
    "$WORK_DIR/classes/org/bndroid/envelope/R\$id.class" \
    "$WORK_DIR/classes/org/bndroid/envelope/R\$layout.class" \
    "$WORK_DIR/classes/org/bndroid/envelope/R\$string.class"

# Keep aapt2's APK as the archive base. In particular, do not unzip its
# compiled binary Manifest/layout and rebuild every entry as STORED.
/bin/cp "$WORK_DIR/link/resources.apk" "$WORK_DIR/envelope-with-dex.apk"
/bin/cp "$WORK_DIR/dex/classes.dex" "$WORK_DIR/stage/classes.dex"
/usr/bin/touch -t 200001010000.00 "$WORK_DIR/stage/classes.dex"
(
    cd "$WORK_DIR/stage"
    "$ZIP" -X -q -0 "$WORK_DIR/envelope-with-dex.apk" classes.dex
)

# The JDK archiver writes DEFLATE entries with data descriptors. Updating the
# aapt2-derived archive also leaves resources.arsc/classes.dex STORED; zipalign
# preserves the descriptors and emits central-exact local tuples. A fixed
# timestamp makes this envelope reproducible.
"$JAR" --update \
    --file "$WORK_DIR/envelope-with-dex.apk" \
    --no-manifest \
    --date=2000-01-01T00:00:00Z \
    -C "$FIXTURE_DIR" assets/envelope-note.txt

"$ZIPALIGN" -p -f 4 \
    "$WORK_DIR/envelope-with-dex.apk" "$WORK_DIR/aligned-unsigned.apk"
"$ZIPALIGN" -c -p 4 "$WORK_DIR/aligned-unsigned.apk"

sign_fixture() {
    "$APKSIGNER" sign \
        --key "$SIGNING_KEY" --cert "$SIGNING_CERT" \
        --v1-signing-enabled false --v2-signing-enabled true \
        --v3-signing-enabled false --v4-signing-enabled false \
        --min-sdk-version 26 --out "$1" "$WORK_DIR/aligned-unsigned.apk"
}
sign_fixture "$WORK_DIR/envelope.apk"
sign_fixture "$WORK_DIR/envelope-second.apk"
/usr/bin/cmp -s "$WORK_DIR/envelope.apk" "$WORK_DIR/envelope-second.apk" \
    || { echo "non-deterministic APK v2 signature" >&2; exit 1; }

"$UNZIP" -t "$WORK_DIR/envelope.apk" >/dev/null
"$ZIPALIGN" -c -p 4 "$WORK_DIR/envelope.apk"
"$AAPT2" dump badging "$WORK_DIR/envelope.apk" > "$WORK_DIR/badging.txt"
"$AAPT2" dump resources "$WORK_DIR/envelope.apk" > "$WORK_DIR/resources.txt"
"$AAPT2" dump xmltree --file AndroidManifest.xml \
    "$WORK_DIR/envelope.apk" > "$WORK_DIR/manifest-xmltree.txt"
"$AAPT2" dump xmltree --file res/layout/activity_envelope.xml \
    "$WORK_DIR/envelope.apk" > "$WORK_DIR/layout-xmltree.txt"
"$DEXDUMP" -d "$WORK_DIR/stage/classes.dex" > "$WORK_DIR/classes-dexdump.txt"
/usr/bin/awk '
    index($0, "MainActivity.onClick:") { capture = 1 }
    index($0, "MainActivity.onCreate:") { capture = 0 }
    capture { print }
' "$WORK_DIR/classes-dexdump.txt" > "$WORK_DIR/on-click-dexdump.txt"

"$PYTHON3" - \
    "$WORK_DIR/link/resources.apk" \
    "$WORK_DIR/envelope.apk" \
    "$FIXTURE_DIR/assets/envelope-note.txt" \
    "$WORK_DIR/archive-evidence.txt" \
    "$ICON_RESOURCES" <<'PY'
from __future__ import annotations

from pathlib import Path
import struct
import sys
import zipfile

source_path, apk_path, asset_path, evidence_path = map(Path, sys.argv[1:5])
icon_mode = sys.argv[5]
icon_resources = icon_mode != "0"
density_icons = icon_mode == "2"
expected_names = [
    "AndroidManifest.xml",
    "res/layout/activity_envelope.xml",
    "resources.arsc",
    "classes.dex",
    "assets/envelope-note.txt",
]
if density_icons:
    expected_names[1:1] = [
        "res/drawable-mdpi-v4/app_icon.png",
        "res/drawable-xhdpi-v4/app_icon.png",
    ]
elif icon_resources:
    expected_names.insert(1, "res/drawable/app_icon.png")
expected_methods = {
    "AndroidManifest.xml": zipfile.ZIP_DEFLATED,
    "res/layout/activity_envelope.xml": zipfile.ZIP_DEFLATED,
    "resources.arsc": zipfile.ZIP_STORED,
    "classes.dex": zipfile.ZIP_STORED,
    "assets/envelope-note.txt": zipfile.ZIP_DEFLATED,
}
if density_icons:
    expected_methods["res/drawable-mdpi-v4/app_icon.png"] = zipfile.ZIP_STORED
    expected_methods["res/drawable-xhdpi-v4/app_icon.png"] = zipfile.ZIP_STORED
elif icon_resources:
    expected_methods["res/drawable/app_icon.png"] = zipfile.ZIP_STORED
expected_source_names = [
    name for name in expected_names
    if name not in ("classes.dex", "assets/envelope-note.txt")
]

with zipfile.ZipFile(source_path) as source:
    source_names = [entry.filename for entry in source.infolist()]
    if source_names != expected_source_names:
        raise SystemExit(f"unexpected aapt2 entry order: {source_names!r}")
    if source.testzip() is not None:
        raise SystemExit("aapt2 resource archive failed CRC validation")
    for name in source_names:
        expected = expected_methods[name]
        if source.getinfo(name).compress_type != expected:
            raise SystemExit(f"aapt2 compression method changed for {name}")
    source_payloads = {name: source.read(name) for name in source_names}

apk_bytes = apk_path.read_bytes()
with zipfile.ZipFile(apk_path) as archive:
    entries = archive.infolist()
    names = [entry.filename for entry in entries]
    if names != expected_names:
        raise SystemExit(f"unexpected APK entry order: {names!r}")
    if sum(name == "classes.dex" for name in names) != 1:
        raise SystemExit("APK must contain exactly one classes.dex")
    bad_crc = archive.testzip()
    if bad_crc is not None:
        raise SystemExit(f"APK CRC validation failed for {bad_crc}")
    descriptor_entries: list[str] = []
    for entry in entries:
        if entry.compress_type != expected_methods[entry.filename]:
            raise SystemExit(
                f"wrong method for {entry.filename}: {entry.compress_type}"
            )
        if entry.flag_bits & 0x1:
            raise SystemExit(f"encrypted entry is forbidden: {entry.filename}")
        if entry.file_size >= 0xFFFFFFFF or entry.compress_size >= 0xFFFFFFFF:
            raise SystemExit(f"ZIP64-sized entry is forbidden: {entry.filename}")
        if b"\x01\x00" in entry.extra:
            raise SystemExit(f"ZIP64 central extra is forbidden: {entry.filename}")

        offset = entry.header_offset
        header = struct.unpack_from("<IHHHHHIIIHH", apk_bytes, offset)
        signature, local_flags, local_method = header[0], header[2], header[3]
        local_crc32, local_compressed, local_uncompressed = header[6:9]
        local_name_length, local_extra_length = header[9], header[10]
        if signature != 0x04034B50:
            raise SystemExit(f"bad local header for {entry.filename}")
        if local_flags != entry.flag_bits or local_method != entry.compress_type:
            raise SystemExit(f"local/central mismatch for {entry.filename}")
        name_start = offset + 30
        name_end = name_start + local_name_length
        if apk_bytes[name_start:name_end] != entry.filename.encode("utf-8"):
            raise SystemExit(f"local name mismatch for {entry.filename}")
        extra_end = name_end + local_extra_length
        if b"\x01\x00" in apk_bytes[name_end:extra_end]:
            raise SystemExit(f"ZIP64 local extra is forbidden: {entry.filename}")
        data_end = extra_end + entry.compress_size
        if entry.flag_bits & 0x8:
            if (
                local_crc32 != entry.CRC
                or local_compressed != entry.compress_size
                or local_uncompressed != entry.file_size
            ):
                raise SystemExit(
                    f"bit-3 local values are not central-exact for {entry.filename}"
                )
            descriptor = apk_bytes[data_end:data_end + 16]
            if len(descriptor) != 16:
                raise SystemExit(f"truncated descriptor for {entry.filename}")
            marker, crc32, compressed, uncompressed = struct.unpack(
                "<IIII", descriptor
            )
            if (
                marker != 0x08074B50
                or crc32 != entry.CRC
                or compressed != entry.compress_size
                or uncompressed != entry.file_size
            ):
                raise SystemExit(f"noncanonical descriptor for {entry.filename}")
            descriptor_entries.append(entry.filename)
        elif (
            local_crc32 != entry.CRC
            or local_compressed != entry.compress_size
            or local_uncompressed != entry.file_size
        ):
            raise SystemExit(f"local values mismatch for {entry.filename}")

    if not descriptor_entries:
        raise SystemExit("APK must retain at least one bit-3 data descriptor")
    if "assets/envelope-note.txt" not in descriptor_entries:
        raise SystemExit("DEFLATE envelope asset must retain its data descriptor")
    if archive.read("assets/envelope-note.txt") != asset_path.read_bytes():
        raise SystemExit("envelope asset payload changed")
    for name, payload in source_payloads.items():
        if archive.read(name) != payload:
            raise SystemExit(f"aapt2 compiled payload changed for {name}")
    for name in ("resources.arsc", "classes.dex"):
        entry = archive.getinfo(name)
        offset = entry.header_offset
        local_name_length, local_extra_length = struct.unpack_from(
            "<HH", apk_bytes, offset + 26
        )
        data_offset = offset + 30 + local_name_length + local_extra_length
        if data_offset % 4 != 0:
            raise SystemExit(f"STORED entry is not 4-byte aligned: {name}")

evidence_path.write_text(
    "\n".join((
        "entry_order=" + ",".join(expected_names),
        "entry_methods=" + ",".join(
            "stored" if expected_methods[name] == zipfile.ZIP_STORED else "deflate"
            for name in expected_names
        ),
        "descriptor_entries=" + ",".join(descriptor_entries),
        "bit3_local_tuple=central-exact",
        "asset_interpreted=0",
        "zip64=0",
        "encrypted=0",
        "stored_alignment=4",
    )) + "\n",
    encoding="utf-8",
)
PY

for expected in \
    "package: name='org.bndroid.envelope' versionCode='$VERSION_CODE' versionName='$VERSION_NAME'" \
    "launchable-activity: name='org.bndroid.envelope.MainActivity'"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/badging.txt" \
        || { echo "badging shape missing: $expected" >&2; exit 1; }
done
[ "$(/usr/bin/grep -Fc 'E: uses-permission' "$WORK_DIR/manifest-xmltree.txt")" -eq 0 ] \
    || { echo "fixture must request no Android permission" >&2; exit 1; }

for expected in \
    "Class descriptor  : 'Lorg/bndroid/envelope/MainActivity;'" \
    "#0              : 'Landroid/view/View\$OnClickListener;'" \
    "name          : 'onCreate'" \
    "name          : 'onClick'" \
    "findViewById:(I)Landroid/view/View;" \
    "setContentView:(I)V" \
    "setOnClickListener:(Landroid/view/View\$OnClickListener;)V" \
    "check-cast"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
        || { echo "DEX shape missing: $expected" >&2; exit 1; }
done
[ "$(/usr/bin/grep -Fc "Class descriptor  : 'Lorg/bndroid/envelope/MainActivity;'" \
    "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
    || { echo "DEX must contain exactly one selected Activity descriptor" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'findViewById:(I)Landroid/view/View; // method@' \
    "$WORK_DIR/classes-dexdump.txt")" -eq 3 ] \
    || { echo "DEX must call findViewById exactly three times" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'setOnClickListener:(Landroid/view/View$OnClickListener;)V // method@' \
    "$WORK_DIR/classes-dexdump.txt")" -eq 2 ] \
    || { echo "DEX must bind exactly two Button listeners" >&2; exit 1; }
if [ "$DEX_METHODS" = 5 ] || [ "$DEX_METHODS" = 6 ]; then
    [ "$(/usr/bin/grep -Fc 'getId:()I // method@' \
        "$WORK_DIR/classes-dexdump.txt")" -eq 0 ] \
        || { echo "DEX direct-string callback must not synthesize a View ID dependency" >&2; exit 1; }
elif [ "$DEX_METHODS" = 2 ] || [ "$DEX_METHODS" = 3 ] || [ "$DEX_METHODS" = 4 ]; then
    [ "$(/usr/bin/grep -Fc 'getId:()I // method@' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 0 ] \
        || { echo "DEX callback must delegate View.getId to the instance helper" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'getId:()I // method@' \
        "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
        || { echo "DEX instance helper must read the clicked View ID once" >&2; exit 1; }
else
    [ "$(/usr/bin/grep -Fc 'getId:()I // method@' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must read the clicked View ID exactly once" >&2; exit 1; }
fi
if [ "$DEX_METHODS" = 5 ] || [ "$DEX_METHODS" = 6 ]; then
    [ "$(/usr/bin/grep -Fc 'setText:(Ljava/lang/CharSequence;)V // method@' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must converge on one CharSequence status mutation" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'setText:(I)V // method@' \
        "$WORK_DIR/classes-dexdump.txt")" -eq 0 ] \
        || { echo "DEX direct-string mode must not call resource-ID setText" >&2; exit 1; }
else
    [ "$(/usr/bin/grep -Fc 'setText:(I)V // method@' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must converge on one shared status mutation" >&2; exit 1; }
fi
if [ "$DEX_METHODS" = 1 ]; then
    /usr/bin/grep -Fq "name          : 'statusTextFor'" \
        "$WORK_DIR/classes-dexdump.txt" \
        || { echo "DEX helper method is missing" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'invoke-static' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must invoke one APK-owned static method" >&2; exit 1; }
elif [ "$DEX_METHODS" = 2 ] || [ "$DEX_METHODS" = 3 ] || [ "$DEX_METHODS" = 4 ]; then
    for expected in \
        "name          : 'statusTextFor'" \
        "type          : '(Landroid/view/View;)I'" \
        "access        : 0x0002 (PRIVATE)" \
        "invoke-direct" \
        "MainActivity;.statusTextFor:(Landroid/view/View;)I"; do
        /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
            || { echo "DEX instance method shape missing: $expected" >&2; exit 1; }
    done
    [ "$(/usr/bin/grep -Fc 'invoke-direct' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must invoke one APK-owned instance method" >&2; exit 1; }
    if [ "$DEX_METHODS" = 3 ] || [ "$DEX_METHODS" = 4 ]; then
        for expected in \
            "name          : 'statusView'" \
            "type          : 'Landroid/widget/TextView;'" \
            "access        : 0x0002 (PRIVATE)" \
            "iput-object" \
            "iget-object"; do
            /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
                || { echo "DEX Activity field shape missing: $expected" >&2; exit 1; }
        done
        [ "$(/usr/bin/grep -Fc 'iput-object' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
            || { echo "DEX onCreate must write one Activity field" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'iget-object' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX callback must read one Activity field" >&2; exit 1; }
    fi
    if [ "$DEX_METHODS" = 4 ]; then
        for expected in \
            "name          : 'clickCount'" \
            "type          : 'I'" \
            "iget " \
            "iput " \
            "add-int/lit8"; do
            /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
                || { echo "DEX persistent int state shape missing: $expected" >&2; exit 1; }
        done
        [ "$(/usr/bin/grep -Fc 'iget v' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX callback must read the int field exactly once" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'iput v' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX callback must write the int field exactly once" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'add-int/lit8' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX callback must increment the int field exactly once" >&2; exit 1; }
    fi
elif [ "$DEX_METHODS" = 5 ] || [ "$DEX_METHODS" = 6 ]; then
    for expected in \
        "name          : 'statusView'" \
        "type          : 'Landroid/widget/TextView;'" \
        "name          : 'clickCount'" \
        "type          : 'I'" \
        "access        : 0x0002 (PRIVATE)" \
        "iput-object" \
        "iget-object" \
        "iget " \
        "iput " \
        "add-int" \
        "const-string" \
        "setText:(Ljava/lang/CharSequence;)V"; do
        /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
            || { echo "DEX direct-string state shape missing: $expected" >&2; exit 1; }
    done
    [ "$(/usr/bin/grep -Fc 'iget v' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must read the int field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'iput v' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must write the int field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'iget-object' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must read the TextView field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Ec 'add-int/(2addr|lit8)' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must increment the int field exactly once" >&2; exit 1; }
    if [ "$DEX_METHODS" = 5 ]; then
        [ "$(/usr/bin/grep -Fc 'const-string' "$WORK_DIR/on-click-dexdump.txt")" -eq 2 ] \
            || { echo "DEX callback must select exactly two direct string literals" >&2; exit 1; }
    else
        for expected in \
            "new-instance" \
            "Ljava/lang/StringBuilder;.<init>:(Ljava/lang/String;)V" \
            "Ljava/lang/StringBuilder;.append:(I)Ljava/lang/StringBuilder;" \
            "Ljava/lang/StringBuilder;.toString:()Ljava/lang/String;"; do
            /usr/bin/grep -Fq "$expected" "$WORK_DIR/on-click-dexdump.txt" \
                || { echo "DEX StringBuilder shape missing: $expected" >&2; exit 1; }
        done
        [ "$(/usr/bin/grep -Fc 'const-string' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX dynamic callback must use one prefix literal" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'new-instance' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
            || { echo "DEX dynamic callback must allocate one StringBuilder" >&2; exit 1; }
    fi
else
    [ "$(/usr/bin/grep -Ec '\\|[0-9a-f]+: if-(eq|ne) ' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 2 ] \
        || { echo "DEX callback must contain two View-ID branches" >&2; exit 1; }
fi

EXPECTED_LINEAR_LAYOUTS=1
if [ "$LAYOUT_ROWS" = 1 ] || [ "$LAYOUT_ROWS" = 2 ] || [ "$LAYOUT_ROWS" = 3 ] || [ "$LAYOUT_ROWS" = 4 ] || [ "$LAYOUT_ROWS" = 5 ] || [ "$LAYOUT_ROWS" = 6 ]; then
    EXPECTED_LINEAR_LAYOUTS=2
fi
[ "$(/usr/bin/grep -Fc 'E: LinearLayout' "$WORK_DIR/layout-xmltree.txt")" -eq "$EXPECTED_LINEAR_LAYOUTS" ] \
    || { echo "layout contains an unexpected LinearLayout count" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'E: TextView' "$WORK_DIR/layout-xmltree.txt")" -eq 2 ] \
    || { echo "layout must contain exactly two TextViews" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'E: Button' "$WORK_DIR/layout-xmltree.txt")" -eq 2 ] \
    || { echo "layout must contain exactly two Buttons" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'android:id(0x010100d0)=@' "$WORK_DIR/layout-xmltree.txt")" -eq 4 ] \
    || { echo "all four leaf Views must have compiled IDs" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'android:text(0x0101014f)=@' "$WORK_DIR/layout-xmltree.txt")" -eq 4 ] \
    || { echo "all four leaf Views must use compiled text resources" >&2; exit 1; }
/usr/bin/grep -Fq "android:orientation(0x010100c4)=1" \
    "$WORK_DIR/layout-xmltree.txt" \
    || { echo "root LinearLayout must be vertical" >&2; exit 1; }
if [ "$LAYOUT_ROWS" = 1 ] || [ "$LAYOUT_ROWS" = 2 ] || [ "$LAYOUT_ROWS" = 3 ] || [ "$LAYOUT_ROWS" = 4 ] || [ "$LAYOUT_ROWS" = 5 ] || [ "$LAYOUT_ROWS" = 6 ]; then
    [ "$(/usr/bin/grep -Fc 'android:orientation(0x010100c4)=0' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
        || { echo "nested LinearLayout must be horizontal" >&2; exit 1; }
fi
if [ "$LAYOUT_ROWS" = 2 ] || [ "$LAYOUT_ROWS" = 3 ] || [ "$LAYOUT_ROWS" = 4 ] || [ "$LAYOUT_ROWS" = 5 ]; then
    [ "$(/usr/bin/grep -Fc 'android:layout_width(0x010100f4)=0.000000dp' "$WORK_DIR/layout-xmltree.txt")" -eq 2 ] \
        || { echo "weighted buttons must use compiled 0dp widths" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'android:layout_weight(0x01010181)=1' "$WORK_DIR/layout-xmltree.txt")" -eq 2 ] \
        || { echo "weighted buttons must use compiled unit weights" >&2; exit 1; }
fi
if [ "$LAYOUT_ROWS" = 6 ]; then
    [ "$(/usr/bin/grep -Fc 'android:layout_width(0x010100f4)=0.000000dp' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
        || { echo "mixed row must contain one compiled 0dp weighted width" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'android:layout_weight(0x01010181)=1' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
        || { echo "mixed row must contain one compiled unit weight" >&2; exit 1; }
fi
if [ "$LAYOUT_ROWS" = 4 ] || [ "$LAYOUT_ROWS" = 5 ] || [ "$LAYOUT_ROWS" = 6 ]; then
    for expected in \
        'android:paddingLeft(0x010100d6)=6.000000dp' \
        'android:paddingTop(0x010100d7)=4.000000dp' \
        'android:paddingRight(0x010100d8)=2.000000dp' \
        'android:paddingBottom(0x010100d9)=8.000000dp'; do
        [ "$(/usr/bin/grep -Fc "$expected" "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
            || { echo "directional row padding missing: $expected" >&2; exit 1; }
    done
    for expected in \
        'android:layout_marginLeft(0x010100f7)=2.000000dp' \
        'android:layout_marginTop(0x010100f8)=1.000000dp' \
        'android:layout_marginRight(0x010100f9)=4.000000dp' \
        'android:layout_marginBottom(0x010100fa)=3.000000dp' \
        'android:layout_marginLeft(0x010100f7)=6.000000dp' \
        'android:layout_marginTop(0x010100f8)=5.000000dp' \
        'android:layout_marginRight(0x010100f9)=2.000000dp' \
        'android:layout_marginBottom(0x010100fa)=1.000000dp'; do
        [ "$(/usr/bin/grep -Fc "$expected" "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
            || { echo "directional button margin missing: $expected" >&2; exit 1; }
    done
fi
if [ "$LAYOUT_ROWS" = 5 ] || [ "$LAYOUT_ROWS" = 6 ]; then
    for expected in \
        'android:layout_width(0x010100f4)=240.000000dp' \
        'android:layout_height(0x010100f5)=120.000000dp' \
        'android:layout_height(0x010100f5)=64.000000dp' \
        'android:layout_height(0x010100f5)=56.000000dp'; do
        [ "$(/usr/bin/grep -Fc "$expected" "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
            || { echo "exact layout size missing: $expected" >&2; exit 1; }
    done
fi
if [ "$LAYOUT_ROWS" = 6 ]; then
    [ "$(/usr/bin/grep -Fc 'android:layout_width(0x010100f4)=132.000000dp' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
        || { echo "mixed row must contain one compiled 132dp fixed width" >&2; exit 1; }
fi
if [ "$LAYOUT_ROWS" = 3 ]; then
    [ "$(/usr/bin/grep -Fc 'android:padding(0x010100d5)=4.000000dp' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
        || { echo "horizontal row must use one compiled uniform 4dp padding" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'android:layout_margin(0x010100f6)=2.000000dp' "$WORK_DIR/layout-xmltree.txt")" -eq 2 ] \
        || { echo "weighted buttons must use two compiled uniform 2dp margins" >&2; exit 1; }
fi

for expected in \
    "id/action_approve" \
    "id/action_reject" \
    "id/status" \
    "id/title" \
    "layout/activity_envelope" \
    "string/app_name" \
    "string/approve_label" \
    "string/reject_label" \
    "string/status_approved" \
    "string/status_initial" \
    "string/status_rejected" \
    "string/title_text" \
    "Envelope Android app" \
    "Approve" \
    "Reject" \
    "Decision: approved" \
    "Decision: pending" \
    "Decision: rejected" \
    "Envelope review"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/resources.txt" \
        || { echo "resource shape missing: $expected" >&2; exit 1; }
done

SIGNATURE_REPORT=$("$APKSIGNER" verify --verbose --print-certs \
    --min-sdk-version 26 "$WORK_DIR/envelope.apk")
for expected in \
    "Verified using v1 scheme (JAR signing): false" \
    "Verified using v2 scheme (APK Signature Scheme v2): true" \
    "Verified using v3 scheme (APK Signature Scheme v3): false" \
    "Verified using v4 scheme (APK Signature Scheme v4): false" \
    "Number of signers: 1" \
    "Signer #1 certificate SHA-256 digest: $EXPECTED_CERT_SHA256"; do
    printf '%s\n' "$SIGNATURE_REPORT" | /usr/bin/grep -Fq "$expected" \
        || { echo "signature shape missing: $expected" >&2; exit 1; }
done

APK_BYTES=$(/usr/bin/wc -c < "$WORK_DIR/envelope.apk" \
    | /usr/bin/tr -d '[:space:]')
[ "$APK_BYTES" -gt 0 ] && [ "$APK_BYTES" -le 65024 ] \
    || { echo "APK outside bounded package size: $APK_BYTES" >&2; exit 1; }

/bin/cp "$WORK_DIR/envelope.apk" "$OUTPUT_APK"
HASH=$(/usr/bin/shasum -a 256 "$OUTPUT_APK" \
    | /usr/bin/awk '{print $1}')
if [ "$OUTPUT_APK" = "$FIXTURE_DIR/androidbox-envelope-demo.apk" ]; then
    printf '%s  androidbox-envelope-demo.apk\n' "$HASH" \
        > "$FIXTURE_DIR/androidbox-envelope-demo.apk.sha256"
    (
        cd "$FIXTURE_DIR"
        /usr/bin/shasum -a 256 -c androidbox-envelope-demo.apk.sha256 >/dev/null
    )
fi
echo "androidbox_envelope_demo_sha256=$HASH"
echo "androidbox_envelope_demo_bytes=$APK_BYTES"
echo "androidbox_envelope_demo_version_code=$VERSION_CODE"
echo "androidbox_envelope_demo_output=$OUTPUT_APK"
if [ "$ICON_RESOURCES" = 2 ]; then
    echo "androidbox_envelope_demo_entry_methods=deflate,stored,stored,deflate,stored,stored,deflate"
elif [ "$ICON_RESOURCES" = 1 ]; then
    echo "androidbox_envelope_demo_entry_methods=deflate,stored,deflate,stored,stored,deflate"
else
    echo "androidbox_envelope_demo_entry_methods=deflate,deflate,stored,stored,deflate"
fi
echo "androidbox_envelope_demo_icon_resources=$ICON_RESOURCES"
echo "androidbox_envelope_demo_dex_methods=$DEX_METHODS"
echo "androidbox_envelope_demo_layout_rows=$LAYOUT_ROWS"
echo "androidbox_envelope_demo_asset_descriptor=bit3+signature"
echo "androidbox_envelope_demo_bit3_local_tuple=central-exact"
echo "androidbox_envelope_demo_d8=$("$D8" --version)"
