#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-resource-update-demo"
SOURCE_DIR="$FIXTURE_DIR/src"
RESOURCE_DIR="$FIXTURE_DIR/res"
SIGNING_FIXTURE_DIR="$ROOT/fixtures/androidbox-resource-demo"
WORK_DIR="$ROOT/target/androidbox-resource-update-demo-build"
COMPILED_RESOURCE_DIR="$WORK_DIR/compiled-resources"
GENERATED_SOURCE_DIR="$WORK_DIR/generated"
CLASSES_DIR="$WORK_DIR/classes"
DEX_DIR="$WORK_DIR/dex"
LINK_DIR="$WORK_DIR/link"
STAGE_DIR="$WORK_DIR/stage"
OUTPUT_APK="$FIXTURE_DIR/androidbox-resource-update-demo.apk"
OUTPUT_SHA="$FIXTURE_DIR/androidbox-resource-update-demo.apk.sha256"
SIGNING_KEY="$SIGNING_FIXTURE_DIR/test-only-apk-signing-key.pk8"
SIGNING_CERT="$SIGNING_FIXTURE_DIR/test-only-apk-signing-cert.pem"
EXPECTED_CERT_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
EXPECTED_PUBLIC_KEY_SHA256=c04dfd1d94654aa6dc5c861c2f15f224d9721c07f6c6c2f5447c7efdba05c236

SDK_ROOT=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}
BUILD_TOOLS="$SDK_ROOT/build-tools/36.1.0"
D8="$BUILD_TOOLS/d8"
AAPT2="$BUILD_TOOLS/aapt2"
DEXDUMP="$BUILD_TOOLS/dexdump"
ZIPALIGN="$BUILD_TOOLS/zipalign"
APKSIGNER="$BUILD_TOOLS/apksigner"
ANDROID_JAR="$SDK_ROOT/platforms/android-36/android.jar"
JAVAC=/usr/bin/javac

for tool in \
    "$JAVAC" \
    "$D8" \
    "$AAPT2" \
    "$DEXDUMP" \
    "$ZIPALIGN" \
    "$APKSIGNER" \
    /usr/bin/cmp \
    /usr/bin/grep \
    /usr/bin/shasum \
    /usr/bin/touch \
    /usr/bin/unzip \
    /usr/bin/zip; do
    if [ ! -x "$tool" ]; then
        echo "missing required offline tool: $tool" >&2
        exit 1
    fi
done
if [ ! -f "$ANDROID_JAR" ]; then
    echo "missing Android 36 platform jar: $ANDROID_JAR" >&2
    exit 1
fi
if [ ! -f "$SIGNING_KEY" ] || [ ! -f "$SIGNING_CERT" ]; then
    echo "missing repository-owned test-only APK signing material" >&2
    exit 1
fi
if [ -e "$FIXTURE_DIR/test-only-apk-signing-key.pk8" ] \
    || [ -e "$FIXTURE_DIR/test-only-apk-signing-cert.pem" ]; then
    echo "update fixture must reference, not copy, the existing signing material" >&2
    exit 1
fi

case "$WORK_DIR" in
    "$ROOT"/target/androidbox-resource-update-demo-build) ;;
    *)
        echo "refusing unsafe work directory: $WORK_DIR" >&2
        exit 1
        ;;
esac

rm -rf "$WORK_DIR"
mkdir -p \
    "$COMPILED_RESOURCE_DIR" \
    "$GENERATED_SOURCE_DIR" \
    "$CLASSES_DIR" \
    "$DEX_DIR" \
    "$LINK_DIR" \
    "$STAGE_DIR/res/layout" \
    "$WORK_DIR/tmp"
export LC_ALL=C
export TZ=UTC
export TMPDIR="$WORK_DIR/tmp"
export SOURCE_DATE_EPOCH=946684800

"$AAPT2" compile \
    --dir "$RESOURCE_DIR" \
    -o "$COMPILED_RESOURCE_DIR"

"$AAPT2" link \
    -o "$LINK_DIR/resources.apk" \
    --manifest "$FIXTURE_DIR/AndroidManifest.xml" \
    -I "$ANDROID_JAR" \
    --java "$GENERATED_SOURCE_DIR" \
    --min-sdk-version 26 \
    --target-sdk-version 36 \
    --version-code 3 \
    --version-name 3.0 \
    "$COMPILED_RESOURCE_DIR/layout_activity_main.xml.flat" \
    "$COMPILED_RESOURCE_DIR/values_strings.arsc.flat"

"$JAVAC" \
    --release 8 \
    -g:none \
    -Xlint:-options \
    -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$CLASSES_DIR" \
    "$SOURCE_DIR/org/bndroid/demo/Main.java" \
    "$SOURCE_DIR/org/bndroid/demo/MainActivity.java" \
    "$GENERATED_SOURCE_DIR/org/bndroid/demo/R.java"

"$D8" \
    --release \
    --min-api 26 \
    --lib "$ANDROID_JAR" \
    --output "$DEX_DIR" \
    "$CLASSES_DIR/org/bndroid/demo/Main.class" \
    "$CLASSES_DIR/org/bndroid/demo/MainActivity.class" \
    "$CLASSES_DIR/org/bndroid/demo/R.class" \
    "$CLASSES_DIR/org/bndroid/demo/R\$id.class" \
    "$CLASSES_DIR/org/bndroid/demo/R\$layout.class" \
    "$CLASSES_DIR/org/bndroid/demo/R\$string.class"

/usr/bin/unzip -p "$LINK_DIR/resources.apk" AndroidManifest.xml \
    > "$STAGE_DIR/AndroidManifest.xml"
/usr/bin/unzip -p "$LINK_DIR/resources.apk" resources.arsc \
    > "$STAGE_DIR/resources.arsc"
/usr/bin/unzip -p "$LINK_DIR/resources.apk" res/layout/activity_main.xml \
    > "$STAGE_DIR/res/layout/activity_main.xml"
cp "$DEX_DIR/classes.dex" "$STAGE_DIR/classes.dex"
/usr/bin/touch -t 200001010000.00 \
    "$STAGE_DIR/AndroidManifest.xml" \
    "$STAGE_DIR/resources.arsc" \
    "$STAGE_DIR/res/layout/activity_main.xml" \
    "$STAGE_DIR/classes.dex"

(
    cd "$STAGE_DIR"
    /usr/bin/zip -X -q -0 \
        "$WORK_DIR/androidbox-resource-update-demo.unaligned.apk" \
        AndroidManifest.xml \
        resources.arsc \
        res/layout/activity_main.xml \
        classes.dex
)
"$ZIPALIGN" -p -f 4 \
    "$WORK_DIR/androidbox-resource-update-demo.unaligned.apk" \
    "$WORK_DIR/androidbox-resource-update-demo.aligned-unsigned.apk"
"$ZIPALIGN" -c -p 4 \
    "$WORK_DIR/androidbox-resource-update-demo.aligned-unsigned.apk"

sign_fixture() {
    output=$1
    "$APKSIGNER" sign \
        --key "$SIGNING_KEY" \
        --cert "$SIGNING_CERT" \
        --v1-signing-enabled false \
        --v2-signing-enabled true \
        --v3-signing-enabled false \
        --v4-signing-enabled false \
        --min-sdk-version 26 \
        --out "$output" \
        "$WORK_DIR/androidbox-resource-update-demo.aligned-unsigned.apk"
}

sign_fixture "$WORK_DIR/androidbox-resource-update-demo.apk"
sign_fixture "$WORK_DIR/androidbox-resource-update-demo.second.apk"
if ! /usr/bin/cmp -s \
    "$WORK_DIR/androidbox-resource-update-demo.apk" \
    "$WORK_DIR/androidbox-resource-update-demo.second.apk"; then
    echo "v2 signing output is not deterministic" >&2
    exit 1
fi
"$ZIPALIGN" -c -p 4 "$WORK_DIR/androidbox-resource-update-demo.apk"

SIGNATURE_REPORT=$("$APKSIGNER" verify \
    --verbose \
    --print-certs \
    --min-sdk-version 26 \
    "$WORK_DIR/androidbox-resource-update-demo.apk")
for expected_signature_line in \
    "Verified using v1 scheme (JAR signing): false" \
    "Verified using v2 scheme (APK Signature Scheme v2): true" \
    "Verified using v3 scheme (APK Signature Scheme v3): false" \
    "Verified using v3.1 scheme (APK Signature Scheme v3.1): false" \
    "Verified using v4 scheme (APK Signature Scheme v4): false" \
    "Verified for SourceStamp: false" \
    "Number of signers: 1" \
    "Signer #1 certificate SHA-256 digest: $EXPECTED_CERT_SHA256" \
    "Signer #1 key algorithm: RSA" \
    "Signer #1 key size (bits): 2048" \
    "Signer #1 public key SHA-256 digest: $EXPECTED_PUBLIC_KEY_SHA256"; do
    if ! printf '%s\n' "$SIGNATURE_REPORT" \
        | /usr/bin/grep -Fq "$expected_signature_line"; then
        echo "APK signature report is missing: $expected_signature_line" >&2
        printf '%s\n' "$SIGNATURE_REPORT" >&2
        exit 1
    fi
done

EXPECTED_ENTRIES=$(printf '%s\n' \
    AndroidManifest.xml \
    resources.arsc \
    res/layout/activity_main.xml \
    classes.dex)
ACTUAL_ENTRIES=$(/usr/bin/unzip -Z1 \
    "$WORK_DIR/androidbox-resource-update-demo.apk")
if [ "$ACTUAL_ENTRIES" != "$EXPECTED_ENTRIES" ]; then
    echo "update fixture has an unexpected APK entry set" >&2
    printf 'expected:\n%s\nactual:\n%s\n' \
        "$EXPECTED_ENTRIES" "$ACTUAL_ENTRIES" >&2
    exit 1
fi
NON_STORED=$(/usr/bin/unzip -lv \
    "$WORK_DIR/androidbox-resource-update-demo.apk" \
    | awk 'NF >= 8 && $NF != "Name" && $NF != "----" && $2 != "Stored" && $1 ~ /^[0-9]+$/ { print $NF }')
if [ -n "$NON_STORED" ]; then
    echo "every update fixture APK entry must use ZIP method STORED" >&2
    printf '%s\n' "$NON_STORED" >&2
    exit 1
fi

"$AAPT2" dump xmltree \
    --file AndroidManifest.xml \
    "$WORK_DIR/androidbox-resource-update-demo.apk" \
    > "$WORK_DIR/manifest-xmltree.txt"
"$AAPT2" dump xmltree \
    --file res/layout/activity_main.xml \
    "$WORK_DIR/androidbox-resource-update-demo.apk" \
    > "$WORK_DIR/layout-xmltree.txt"
"$AAPT2" dump resources \
    "$WORK_DIR/androidbox-resource-update-demo.apk" \
    > "$WORK_DIR/resources.txt"
"$DEXDUMP" -d \
    "$STAGE_DIR/classes.dex" \
    > "$WORK_DIR/classes-dexdump.txt"
BADGING=$("$AAPT2" dump badging \
    "$WORK_DIR/androidbox-resource-update-demo.apk")
for expected_badging_line in \
    "package: name='org.bndroid.demo' versionCode='3' versionName='3.0'" \
    "application-label:'AndroidBox Resources-1 Demo v3'" \
    "launchable-activity: name='org.bndroid.demo.MainActivity'"; do
    if ! printf '%s\n' "$BADGING" \
        | /usr/bin/grep -Fq "$expected_badging_line"; then
        echo "binary manifest badging is missing: $expected_badging_line" >&2
        printf '%s\n' "$BADGING" >&2
        exit 1
    fi
done

for expected_resource in \
    "resource 0x7f010000 id/fixture_label" \
    "resource 0x7f020000 layout/activity_main" \
    "resource 0x7f030000 string/activity_message" \
    "resource 0x7f030001 string/app_name" \
    "AndroidBox updated resource view" \
    "AndroidBox Resources-1 Demo v3"; do
    if ! /usr/bin/grep -Fq \
        "$expected_resource" "$WORK_DIR/resources.txt"; then
        echo "compiled resource table is missing: $expected_resource" >&2
        exit 1
    fi
done
for expected_layout_item in \
    "E: TextView" \
    "android:id(0x010100d0)=@0x7f010000" \
    "android:layout_width(0x010100f4)=-1" \
    "android:layout_height(0x010100f5)=-2" \
    "android:text(0x0101014f)=@0x7f030000"; do
    if ! /usr/bin/grep -Fq \
        "$expected_layout_item" "$WORK_DIR/layout-xmltree.txt"; then
        echo "compiled layout is missing: $expected_layout_item" >&2
        exit 1
    fi
done
for expected_dex_item in \
    "Class descriptor  : 'Lorg/bndroid/demo/Main;'" \
    "name          : 'boot'" \
    "name          : 'onTap'" \
    "Class descriptor  : 'Lorg/bndroid/demo/MainActivity;'" \
    "name          : 'onCreate'" \
    "const/high16 v1, #int 2130837504 // #7f02" \
    "setContentView:(I)V"; do
    if ! /usr/bin/grep -Fq \
        "$expected_dex_item" "$WORK_DIR/classes-dexdump.txt"; then
        echo "classes.dex is missing: $expected_dex_item" >&2
        exit 1
    fi
done

cp "$WORK_DIR/androidbox-resource-update-demo.apk" "$OUTPUT_APK"
HASH=$(/usr/bin/shasum -a 256 "$OUTPUT_APK" | awk '{print $1}')
printf '%s  %s\n' \
    "$HASH" \
    "androidbox-resource-update-demo.apk" \
    > "$OUTPUT_SHA"

echo "androidbox_resource_update_demo_apk=$OUTPUT_APK"
echo "androidbox_resource_update_demo_sha256=$HASH"
echo "androidbox_resource_update_demo_certificate_sha256=$EXPECTED_CERT_SHA256"
echo "androidbox_resource_update_demo_signature_scheme=v2-only-rsa2048-sha256"
echo "androidbox_resource_update_demo_version_code=3"
echo "androidbox_resource_update_demo_version_name=3.0"
echo "androidbox_resource_update_demo_d8=$("$D8" --version)"
