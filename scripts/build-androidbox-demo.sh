#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-demo"
SOURCE_DIR="$FIXTURE_DIR/src"
WORK_DIR="$ROOT/target/androidbox-demo-build"
CLASSES_DIR="$WORK_DIR/classes"
DEX_DIR="$WORK_DIR/dex"
LINK_DIR="$WORK_DIR/link"
STAGE_DIR="$WORK_DIR/stage"
OUTPUT_APK="$FIXTURE_DIR/androidbox-demo.apk"
OUTPUT_SHA="$FIXTURE_DIR/androidbox-demo.apk.sha256"

SDK_ROOT=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}
BUILD_TOOLS="$SDK_ROOT/build-tools/36.1.0"
D8="$BUILD_TOOLS/d8"
AAPT2="$BUILD_TOOLS/aapt2"
ZIPALIGN="$BUILD_TOOLS/zipalign"
ANDROID_JAR="$SDK_ROOT/platforms/android-36/android.jar"
JAVAC=/usr/bin/javac

for tool in "$JAVAC" "$D8" "$AAPT2" "$ZIPALIGN" /usr/bin/unzip /usr/bin/zip /usr/bin/touch /usr/bin/shasum; do
    if [ ! -x "$tool" ]; then
        echo "missing required offline tool: $tool" >&2
        exit 1
    fi
done
if [ ! -f "$ANDROID_JAR" ]; then
    echo "missing Android 36 platform jar: $ANDROID_JAR" >&2
    exit 1
fi

case "$WORK_DIR" in
    "$ROOT"/target/androidbox-demo-build) ;;
    *)
        echo "refusing unsafe work directory: $WORK_DIR" >&2
        exit 1
        ;;
esac

rm -rf "$WORK_DIR"
mkdir -p "$CLASSES_DIR" "$DEX_DIR" "$LINK_DIR" "$STAGE_DIR" "$WORK_DIR/tmp"
export LC_ALL=C
export TZ=UTC
export TMPDIR="$WORK_DIR/tmp"
export SOURCE_DATE_EPOCH=946684800

"$JAVAC" \
    --release 8 \
    -g:none \
    -Xlint:-options \
    -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$CLASSES_DIR" \
    "$SOURCE_DIR/org/bndroid/demo/Main.java" \
    "$SOURCE_DIR/org/bndroid/demo/MainActivity.java"

"$D8" \
    --release \
    --min-api 26 \
    --lib "$ANDROID_JAR" \
    --output "$DEX_DIR" \
    "$CLASSES_DIR/org/bndroid/demo/Main.class" \
    "$CLASSES_DIR/org/bndroid/demo/MainActivity.class"

"$AAPT2" link \
    -o "$LINK_DIR/manifest.apk" \
    --manifest "$FIXTURE_DIR/AndroidManifest.xml" \
    -I "$ANDROID_JAR" \
    --min-sdk-version 26 \
    --target-sdk-version 36 \
    --version-code 1 \
    --version-name 1.0

/usr/bin/unzip -p "$LINK_DIR/manifest.apk" AndroidManifest.xml > "$STAGE_DIR/AndroidManifest.xml"
cp "$DEX_DIR/classes.dex" "$STAGE_DIR/classes.dex"
/usr/bin/touch -t 200001010000.00 "$STAGE_DIR/AndroidManifest.xml" "$STAGE_DIR/classes.dex"

rm -f "$WORK_DIR/androidbox-demo.unaligned.apk" "$WORK_DIR/androidbox-demo.apk"
(
    cd "$STAGE_DIR"
    /usr/bin/zip -X -q -0 "$WORK_DIR/androidbox-demo.unaligned.apk" AndroidManifest.xml classes.dex
)
"$ZIPALIGN" -p -f 4 "$WORK_DIR/androidbox-demo.unaligned.apk" "$WORK_DIR/androidbox-demo.apk"
"$ZIPALIGN" -c -p 4 "$WORK_DIR/androidbox-demo.apk"

DEX_NAMES=$(/usr/bin/unzip -Z1 "$WORK_DIR/androidbox-demo.apk" | grep -E '^classes([0-9]+)?[.]dex$' || true)
if [ "$DEX_NAMES" != "classes.dex" ]; then
    echo "fixture must contain classes.dex and no secondary DEX files" >&2
    exit 1
fi
if [ "$(/usr/bin/unzip -lv "$WORK_DIR/androidbox-demo.apk" | awk '$NF == "classes.dex" { print $2 }')" != "Stored" ]; then
    echo "classes.dex must use ZIP method STORED" >&2
    exit 1
fi
"$AAPT2" dump xmltree --file AndroidManifest.xml "$WORK_DIR/androidbox-demo.apk" >/dev/null
if ! "$AAPT2" dump badging "$WORK_DIR/androidbox-demo.apk" \
    | grep -q "^launchable-activity: name='org.bndroid.demo.MainActivity'"; then
    echo "binary manifest does not expose the expected launcher activity" >&2
    exit 1
fi

cp "$WORK_DIR/androidbox-demo.apk" "$OUTPUT_APK"
HASH=$(/usr/bin/shasum -a 256 "$OUTPUT_APK" | awk '{print $1}')
printf '%s  %s\n' "$HASH" "androidbox-demo.apk" > "$OUTPUT_SHA"

echo "androidbox_demo_apk=$OUTPUT_APK"
echo "androidbox_demo_sha256=$HASH"
echo "androidbox_demo_d8=$("$D8" --version)"
