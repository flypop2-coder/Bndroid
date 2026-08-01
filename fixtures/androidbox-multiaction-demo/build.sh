#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && /bin/pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-multiaction-demo"
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
DEX_METHODS=${BNDROID_ANDROID_DEX_METHODS:-0}
case "$DEX_METHODS" in
    0) SOURCE_ROOT="$FIXTURE_DIR/src" ;;
    1) SOURCE_ROOT="$FIXTURE_DIR/src-methods" ;;
    *) echo "BNDROID_ANDROID_DEX_METHODS must be 0 or 1" >&2; exit 2 ;;
esac
OUTPUT_APK=${BNDROID_MULTIACTION_OUTPUT_APK:-"$FIXTURE_DIR/androidbox-multiaction-demo.apk"}
case "$OUTPUT_APK" in
    "$FIXTURE_DIR/androidbox-multiaction-demo.apk"|"$ROOT"/target/*) ;;
    *) echo "output APK must be canonical or below target/: $OUTPUT_APK" >&2; exit 2 ;;
esac

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

WORK_DIR=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/bndroid-multiaction-apk.XXXXXX")
cleanup() {
    if [ "${KEEP_BUILD_DIR:-0}" = 1 ]; then
        echo "androidbox_multiaction_demo_build_dir=$WORK_DIR"
    else
        /bin/rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT HUP INT TERM
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
    "$WORK_DIR/compiled/layout_activity_multiaction.xml.flat" \
    "$WORK_DIR/compiled/values_strings.arsc.flat"

"$JAVAC" \
    --release 8 -g:none -Xlint:-options -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$WORK_DIR/classes" \
    "$SOURCE_ROOT/org/bndroid/multiaction/MainActivity.java" \
    "$WORK_DIR/generated/org/bndroid/multiaction/R.java"

"$D8" \
    --release --min-api 26 --lib "$ANDROID_JAR" \
    --output "$WORK_DIR/dex" \
    "$WORK_DIR/classes/org/bndroid/multiaction/MainActivity.class" \
    "$WORK_DIR/classes/org/bndroid/multiaction/R.class" \
    "$WORK_DIR/classes/org/bndroid/multiaction/R\$id.class" \
    "$WORK_DIR/classes/org/bndroid/multiaction/R\$layout.class" \
    "$WORK_DIR/classes/org/bndroid/multiaction/R\$string.class"

/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" AndroidManifest.xml \
    > "$WORK_DIR/stage/AndroidManifest.xml"
/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" resources.arsc \
    > "$WORK_DIR/stage/resources.arsc"
/usr/bin/unzip -p "$WORK_DIR/link/resources.apk" res/layout/activity_multiaction.xml \
    > "$WORK_DIR/stage/res/layout/activity_multiaction.xml"
/bin/cp "$WORK_DIR/dex/classes.dex" "$WORK_DIR/stage/classes.dex"
/usr/bin/touch -t 200001010000.00 \
    "$WORK_DIR/stage/AndroidManifest.xml" \
    "$WORK_DIR/stage/resources.arsc" \
    "$WORK_DIR/stage/res/layout/activity_multiaction.xml" \
    "$WORK_DIR/stage/classes.dex"

(
    cd "$WORK_DIR/stage"
    /usr/bin/zip -X -q -0 "$WORK_DIR/unaligned.apk" \
        AndroidManifest.xml resources.arsc res/layout/activity_multiaction.xml classes.dex
)
"$ZIPALIGN" -p -f 4 "$WORK_DIR/unaligned.apk" "$WORK_DIR/aligned-unsigned.apk"

sign_fixture() {
    "$APKSIGNER" sign \
        --key "$SIGNING_KEY" --cert "$SIGNING_CERT" \
        --v1-signing-enabled false --v2-signing-enabled true \
        --v3-signing-enabled false --v4-signing-enabled false \
        --min-sdk-version 26 --out "$1" "$WORK_DIR/aligned-unsigned.apk"
}
sign_fixture "$WORK_DIR/multiaction.apk"
sign_fixture "$WORK_DIR/multiaction-second.apk"
/usr/bin/cmp -s "$WORK_DIR/multiaction.apk" "$WORK_DIR/multiaction-second.apk" \
    || { echo "non-deterministic APK v2 signature" >&2; exit 1; }

"$AAPT2" dump badging "$WORK_DIR/multiaction.apk" > "$WORK_DIR/badging.txt"
"$AAPT2" dump resources "$WORK_DIR/multiaction.apk" > "$WORK_DIR/resources.txt"
"$AAPT2" dump xmltree --file AndroidManifest.xml \
    "$WORK_DIR/multiaction.apk" > "$WORK_DIR/manifest-xmltree.txt"
"$AAPT2" dump xmltree --file res/layout/activity_multiaction.xml \
    "$WORK_DIR/multiaction.apk" > "$WORK_DIR/layout-xmltree.txt"
"$DEXDUMP" -d "$WORK_DIR/stage/classes.dex" > "$WORK_DIR/classes-dexdump.txt"
/usr/bin/awk '
    index($0, "MainActivity.onClick:") { capture = 1 }
    index($0, "MainActivity.onCreate:") { capture = 0 }
    capture { print }
' "$WORK_DIR/classes-dexdump.txt" > "$WORK_DIR/on-click-dexdump.txt"

EXPECTED_ENTRIES=$(printf '%s\n' \
    AndroidManifest.xml resources.arsc res/layout/activity_multiaction.xml classes.dex)
ACTUAL_ENTRIES=$(/usr/bin/unzip -Z1 "$WORK_DIR/multiaction.apk")
[ "$ACTUAL_ENTRIES" = "$EXPECTED_ENTRIES" ] \
    || { echo "unexpected APK entry set" >&2; exit 1; }
NON_STORED=$(/usr/bin/unzip -lv "$WORK_DIR/multiaction.apk" \
    | /usr/bin/awk \
        'NF >= 8 && $NF != "Name" && $NF != "----" && $2 != "Stored" && $1 ~ /^[0-9]+$/ { print $NF }')
[ -z "$NON_STORED" ] || { echo "all APK entries must be STORED" >&2; exit 1; }

for expected in \
    "package: name='org.bndroid.multiaction' versionCode='1' versionName='1.0'" \
    "launchable-activity: name='org.bndroid.multiaction.MainActivity'"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/badging.txt" \
        || { echo "badging shape missing: $expected" >&2; exit 1; }
done
[ "$(/usr/bin/grep -Fc 'E: uses-permission' "$WORK_DIR/manifest-xmltree.txt")" -eq 0 ] \
    || { echo "fixture must request no Android permission" >&2; exit 1; }

for expected in \
    "Class descriptor  : 'Lorg/bndroid/multiaction/MainActivity;'" \
    "#0              : 'Landroid/view/View\$OnClickListener;'" \
    "name          : 'onCreate'" \
    "name          : 'onClick'" \
    "findViewById:(I)Landroid/view/View;" \
    "getId:()I" \
    "setOnClickListener:(Landroid/view/View\$OnClickListener;)V" \
    "setText:(I)V" \
    "check-cast"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
        || { echo "DEX shape missing: $expected" >&2; exit 1; }
done
[ "$(/usr/bin/grep -Fc 'findViewById:(I)Landroid/view/View; // method@' \
    "$WORK_DIR/classes-dexdump.txt")" -eq 3 ] \
    || { echo "DEX must call findViewById exactly three times" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'setOnClickListener:(Landroid/view/View$OnClickListener;)V // method@' \
    "$WORK_DIR/classes-dexdump.txt")" -eq 2 ] \
    || { echo "DEX must bind exactly two Button listeners" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'getId:()I // method@' \
    "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
    || { echo "DEX callback must read the clicked View ID exactly once" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'setText:(I)V // method@' \
    "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
    || { echo "DEX callback must converge on one shared status mutation" >&2; exit 1; }
if [ "$DEX_METHODS" -eq 0 ]; then
    [ "$(/usr/bin/grep -Ec '\\|[0-9a-f]+: if-(eq|ne) ' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 2 ] \
        || { echo "DEX callback must contain two View-ID branches" >&2; exit 1; }
    for expected in \
        "const/high16 v1, #int 2130771968 // #7f01" \
        "const v1, #float 1.7147e+38 // #7f010001" \
        "const v3, #float 1.74129e+38 // #7f030003" \
        "const v3, #float 1.74129e+38 // #7f030005" \
        "goto 0014"; do
        /usr/bin/grep -Fq "$expected" "$WORK_DIR/on-click-dexdump.txt" \
            || { echo "DEX callback branch shape missing: $expected" >&2; exit 1; }
    done
    [ "$(/usr/bin/grep -Fc 'invoke-static' "$WORK_DIR/on-click-dexdump.txt")" -eq 0 ] \
        || { echo "canonical callback must remain direct" >&2; exit 1; }
else
    for expected in \
        "name          : 'statusTextFor'" \
        "type          : '(I)I'" \
        "access        : 0x000a (PRIVATE STATIC)" \
        "invoke-static" \
        "MainActivity;.statusTextFor:(I)I"; do
        /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
            || { echo "DEX app-defined method shape missing: $expected" >&2; exit 1; }
    done
    [ "$(/usr/bin/grep -Fc 'invoke-static' "$WORK_DIR/on-click-dexdump.txt")" -eq 1 ] \
        || { echo "ABI58 callback must invoke one APK-defined helper" >&2; exit 1; }
    [ "$(/usr/bin/grep -Ec '\\|[0-9a-f]+: if-(eq|ne) ' \
        "$WORK_DIR/on-click-dexdump.txt")" -eq 0 ] \
        || { echo "ABI58 onClick must delegate its branch" >&2; exit 1; }
fi

[ "$(/usr/bin/grep -Fc 'E: LinearLayout' "$WORK_DIR/layout-xmltree.txt")" -eq 1 ] \
    || { echo "layout must contain exactly one root LinearLayout" >&2; exit 1; }
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

for expected in \
    "id/action_approve" \
    "id/action_reject" \
    "id/status" \
    "id/title" \
    "layout/activity_multiaction" \
    "string/app_name" \
    "string/approve_label" \
    "string/reject_label" \
    "string/status_approved" \
    "string/status_initial" \
    "string/status_rejected" \
    "string/title_text" \
    "Multi-action Android app" \
    "Approve" \
    "Reject" \
    "Decision: approved" \
    "Decision: pending" \
    "Decision: rejected" \
    "Review request"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/resources.txt" \
        || { echo "resource shape missing: $expected" >&2; exit 1; }
done

SIGNATURE_REPORT=$("$APKSIGNER" verify --verbose --print-certs \
    --min-sdk-version 26 "$WORK_DIR/multiaction.apk")
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

APK_BYTES=$(/usr/bin/wc -c < "$WORK_DIR/multiaction.apk" | /usr/bin/tr -d '[:space:]')
[ "$APK_BYTES" -gt 0 ] && [ "$APK_BYTES" -le 65024 ] \
    || { echo "APK outside bounded package size: $APK_BYTES" >&2; exit 1; }

/bin/mkdir -p "$(/usr/bin/dirname "$OUTPUT_APK")"
/bin/cp "$WORK_DIR/multiaction.apk" "$OUTPUT_APK"
HASH=$(/usr/bin/shasum -a 256 "$OUTPUT_APK" \
    | /usr/bin/awk '{print $1}')
if [ "$OUTPUT_APK" = "$FIXTURE_DIR/androidbox-multiaction-demo.apk" ]; then
    printf '%s  androidbox-multiaction-demo.apk\n' "$HASH" \
        > "$FIXTURE_DIR/androidbox-multiaction-demo.apk.sha256"
fi
echo "androidbox_multiaction_demo_sha256=$HASH"
echo "androidbox_multiaction_demo_bytes=$APK_BYTES"
echo "androidbox_multiaction_demo_output=$OUTPUT_APK"
echo "androidbox_multiaction_demo_dex_methods=$DEX_METHODS"
echo "androidbox_multiaction_demo_d8=$("$D8" --version)"
