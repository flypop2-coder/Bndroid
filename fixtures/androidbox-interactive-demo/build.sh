#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && /bin/pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-interactive-demo"
WORK_DIR="$ROOT/target/androidbox-interactive-demo-build"
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
SIGNING_KEY="$SIGNING_DIR/test-only-apk-signing-key.pk8"
SIGNING_CERT="$SIGNING_DIR/test-only-apk-signing-cert.pem"
EXPECTED_CERT_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf

case "$SDK_ROOT" in
    /*) ;;
    *) echo "Android SDK root must be absolute: $SDK_ROOT" >&2; exit 1 ;;
esac
for tool in "$AAPT2" "$D8" "$DEXDUMP" "$ZIPALIGN" "$APKSIGNER" "$JAVAC"; do
    [ -x "$tool" ] || { echo "missing offline tool: $tool" >&2; exit 1; }
done
[ -f "$ANDROID_JAR" ] || { echo "missing Android platform jar" >&2; exit 1; }
[ -f "$SIGNING_KEY" ] && [ -f "$SIGNING_CERT" ] \
    || { echo "missing test-only signing material" >&2; exit 1; }

/bin/rm -rf "$WORK_DIR"
/bin/mkdir -p \
    "$WORK_DIR/compiled" "$WORK_DIR/generated" "$WORK_DIR/classes" \
    "$WORK_DIR/dex" "$WORK_DIR/link" "$WORK_DIR/stage/res/layout" "$WORK_DIR/tmp"
export LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=946684800 TMPDIR="$WORK_DIR/tmp"

"$AAPT2" compile --dir "$FIXTURE_DIR/res" -o "$WORK_DIR/compiled"
"$AAPT2" link \
    -o "$WORK_DIR/link/resources.apk" \
    --manifest "$FIXTURE_DIR/AndroidManifest.xml" \
    -I "$ANDROID_JAR" \
    --java "$WORK_DIR/generated" \
    --min-sdk-version 26 \
    --target-sdk-version 36 \
    --version-code 1 \
    --version-name 1.0 \
    "$WORK_DIR/compiled/layout_activity_main.xml.flat" \
    "$WORK_DIR/compiled/values_strings.arsc.flat"

"$JAVAC" \
    --release 8 -g:none -Xlint:-options -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$WORK_DIR/classes" \
    "$FIXTURE_DIR/src/org/bndroid/interactive/MainActivity.java" \
    "$WORK_DIR/generated/org/bndroid/interactive/R.java"

"$D8" \
    --release --min-api 26 --lib "$ANDROID_JAR" \
    --output "$WORK_DIR/dex" \
    "$WORK_DIR/classes/org/bndroid/interactive/MainActivity.class" \
    "$WORK_DIR/classes/org/bndroid/interactive/R.class" \
    "$WORK_DIR/classes/org/bndroid/interactive/R\$id.class" \
    "$WORK_DIR/classes/org/bndroid/interactive/R\$layout.class" \
    "$WORK_DIR/classes/org/bndroid/interactive/R\$string.class"

/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" AndroidManifest.xml \
    > "$WORK_DIR/stage/AndroidManifest.xml"
/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" resources.arsc \
    > "$WORK_DIR/stage/resources.arsc"
/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" res/layout/activity_main.xml \
    > "$WORK_DIR/stage/res/layout/activity_main.xml"
/bin/cp "$WORK_DIR/dex/classes.dex" "$WORK_DIR/stage/classes.dex"
/usr/bin/touch -t 200001010000.00 \
    "$WORK_DIR/stage/AndroidManifest.xml" \
    "$WORK_DIR/stage/resources.arsc" \
    "$WORK_DIR/stage/res/layout/activity_main.xml" \
    "$WORK_DIR/stage/classes.dex"

(
    cd "$WORK_DIR/stage"
    /usr/bin/zip -X -q -0 "$WORK_DIR/unaligned.apk" \
        AndroidManifest.xml resources.arsc res/layout/activity_main.xml classes.dex
)
"$ZIPALIGN" -p -f 4 "$WORK_DIR/unaligned.apk" "$WORK_DIR/aligned-unsigned.apk"

sign_fixture() {
    "$APKSIGNER" sign \
        --key "$SIGNING_KEY" --cert "$SIGNING_CERT" \
        --v1-signing-enabled false --v2-signing-enabled true \
        --v3-signing-enabled false --v4-signing-enabled false \
        --min-sdk-version 26 --out "$1" "$WORK_DIR/aligned-unsigned.apk"
}
sign_fixture "$WORK_DIR/interactive.apk"
sign_fixture "$WORK_DIR/interactive-second.apk"
/usr/bin/cmp -s "$WORK_DIR/interactive.apk" "$WORK_DIR/interactive-second.apk" \
    || { echo "non-deterministic APK v2 signature" >&2; exit 1; }

"$AAPT2" dump badging "$WORK_DIR/interactive.apk" > "$WORK_DIR/badging.txt"
"$AAPT2" dump resources "$WORK_DIR/interactive.apk" > "$WORK_DIR/resources.txt"
"$AAPT2" dump xmltree --file res/layout/activity_main.xml \
    "$WORK_DIR/interactive.apk" > "$WORK_DIR/layout-xmltree.txt"
"$DEXDUMP" -d "$WORK_DIR/stage/classes.dex" > "$WORK_DIR/classes-dexdump.txt"

EXPECTED_ENTRIES=$(printf '%s\n' \
    AndroidManifest.xml resources.arsc res/layout/activity_main.xml classes.dex)
ACTUAL_ENTRIES=$(/usr/bin/unzip -Z1 "$WORK_DIR/interactive.apk")
[ "$ACTUAL_ENTRIES" = "$EXPECTED_ENTRIES" ] \
    || { echo "unexpected APK entry set" >&2; exit 1; }
NON_STORED=$(/usr/bin/unzip -lv "$WORK_DIR/interactive.apk" \
    | /usr/bin/awk \
        'NF >= 8 && $NF != "Name" && $NF != "----" && $2 != "Stored" && $1 ~ /^[0-9]+$/ { print $NF }')
[ -z "$NON_STORED" ] || { echo "all APK entries must be STORED" >&2; exit 1; }

for expected in \
    "Class descriptor  : 'Lorg/bndroid/interactive/MainActivity;'" \
    "#0              : 'Landroid/view/View\$OnClickListener;'" \
    "name          : 'onCreate'" \
    "name          : 'onClick'" \
    "const/high16 v1, #int 2130771968 // #7f01" \
    "const v2, #float 1.7147e+38 // #7f010001" \
    "const v0, #float 1.74129e+38 // #7f030003" \
    "findViewById:(I)Landroid/view/View;" \
    "setOnClickListener:(Landroid/view/View\$OnClickListener;)V" \
    "setText:(I)V" \
    "check-cast"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
        || { echo "DEX shape missing: $expected" >&2; exit 1; }
done
for expected in \
    "E: LinearLayout" \
    "android:orientation(0x010100c4)=1" \
    "E: TextView" \
    "android:id(0x010100d0)=@0x7f010001" \
    "android:text(0x0101014f)=@0x7f030000" \
    "E: Button" \
    "android:id(0x010100d0)=@0x7f010000" \
    "android:text(0x0101014f)=@0x7f030002"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/layout-xmltree.txt" \
        || { echo "layout shape missing: $expected" >&2; exit 1; }
done
for expected in \
    "resource 0x7f010000 id/action" \
    "resource 0x7f010001 id/label" \
    "resource 0x7f020000 layout/activity_main" \
    "resource 0x7f030000 string/activity_initial" \
    "resource 0x7f030001 string/app_name" \
    "resource 0x7f030002 string/button_label" \
    "resource 0x7f030003 string/clicked_message" \
    "Ready for a real APK click" \
    "Button callback executed"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/resources.txt" \
        || { echo "resource shape missing: $expected" >&2; exit 1; }
done
SIGNATURE_REPORT=$("$APKSIGNER" verify --verbose --print-certs \
    --min-sdk-version 26 "$WORK_DIR/interactive.apk")
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

APK_BYTES=$(/usr/bin/wc -c < "$WORK_DIR/interactive.apk" | /usr/bin/tr -d '[:space:]')
[ "$APK_BYTES" -gt 0 ] && [ "$APK_BYTES" -le 65024 ] \
    || { echo "APK outside bounded package size: $APK_BYTES" >&2; exit 1; }

/bin/cp "$WORK_DIR/interactive.apk" "$FIXTURE_DIR/androidbox-interactive-demo.apk"
HASH=$(/usr/bin/shasum -a 256 "$FIXTURE_DIR/androidbox-interactive-demo.apk" \
    | /usr/bin/awk '{print $1}')
printf '%s  androidbox-interactive-demo.apk\n' "$HASH" \
    > "$FIXTURE_DIR/androidbox-interactive-demo.apk.sha256"
echo "androidbox_interactive_demo_sha256=$HASH"
echo "androidbox_interactive_demo_bytes=$APK_BYTES"
echo "androidbox_interactive_demo_d8=$("$D8" --version)"
