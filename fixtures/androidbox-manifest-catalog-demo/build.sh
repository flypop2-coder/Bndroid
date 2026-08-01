#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(/usr/bin/dirname -- "$0")" && /bin/pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && /bin/pwd)
FIXTURE_DIR="$ROOT/fixtures/androidbox-manifest-catalog-demo"
SIGNING_DIR="$ROOT/fixtures/androidbox-resource-demo"
WORK_DIR="$ROOT/target/androidbox-manifest-catalog-demo-build"
CANONICAL_APK="$FIXTURE_DIR/androidbox-manifest-catalog-demo.apk"
CANONICAL_SHA="$FIXTURE_DIR/androidbox-manifest-catalog-demo.apk.sha256"
VERSION_CODE=${BNDROID_ENVELOPE_VERSION_CODE:-3}
VERSION_NAME=${BNDROID_ENVELOPE_VERSION_NAME:-3.0}
OUTPUT_APK=${BNDROID_ENVELOPE_OUTPUT_APK:-"$CANONICAL_APK"}
ICON_RESOURCES=${BNDROID_ANDROID_ICON_RESOURCES:-0}
DEX_METHODS=${BNDROID_ANDROID_DEX_METHODS:-0}
LAYOUT_ROWS=${BNDROID_ANDROID_LAYOUT_ROWS:-0}
SIGNING_KEY="$SIGNING_DIR/test-only-apk-signing-key.pk8"
SIGNING_CERT="$SIGNING_DIR/test-only-apk-signing-cert.pem"
EXPECTED_CERT_SHA256=e7412e1cc0ffbd21000ece5176d00837ce276e42e6b52bed99cb5e62857b77bf
EXPECTED_PUBLIC_KEY_SHA256=c04dfd1d94654aa6dc5c861c2f15f224d9721c07f6c6c2f5447c7efdba05c236
MAX_APK_BYTES=65024

case "$VERSION_CODE/$VERSION_NAME" in
    1/1.0|2/2.0|3/3.0) ;;
    *) echo "supported catalog versions are 1/1.0, 2/2.0, or 3/3.0" >&2; exit 1 ;;
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
    "$CANONICAL_APK")
        [ "$VERSION_CODE/$VERSION_NAME" = 3/3.0 ] \
            || { echo "the canonical catalog fixture is reserved for version 3" >&2; exit 1; }
        [ "$ICON_RESOURCES" = 0 ] \
            || { echo "the historical canonical fixture does not carry the ABI 56 icon" >&2; exit 1; }
        [ "$DEX_METHODS" = 0 ] \
            || { echo "the historical canonical fixture does not use alternate app methods" >&2; exit 1; }
        [ "$LAYOUT_ROWS" = 0 ] \
            || { echo "the historical canonical fixture does not use nested layout rows" >&2; exit 1; }
        ;;
    "$ROOT"/target/androidbox-manifest-catalog3/*|"$ROOT"/target/androidbox-runtime-install2/*|"$ROOT"/target/androidbox-multipackage4/*|"$ROOT"/target/androidbox-multipackage4-*/*|"$ROOT"/target/androidbox-icon-resources5/*|"$ROOT"/target/androidbox-density-icons7/*|"$ROOT"/target/androidbox-dex-methods8/*|"$ROOT"/target/androidbox-dex-instance9/*|"$ROOT"/target/androidbox-activity-fields10/*|"$ROOT"/target/activity-fields10/*|"$ROOT"/target/activity-state11/*|"$ROOT"/target/string-text12/*|"$ROOT"/target/androidbox-string-text12/*|"$ROOT"/target/string-builder13/*|"$ROOT"/target/androidbox-string-builder13/*|"$ROOT"/target/layout-row14/*|"$ROOT"/target/androidbox-layout-row14/*|"$ROOT"/target/layout-weight15/*|"$ROOT"/target/androidbox-layout-weight15/*|"$ROOT"/target/layout-spacing16/*|"$ROOT"/target/androidbox-layout-spacing16/*|"$ROOT"/target/layout-directional17/*|"$ROOT"/target/androidbox-layout-directional17/*|"$ROOT"/target/layout-size18/*|"$ROOT"/target/androidbox-layout-size18/*|"$ROOT"/target/layout-mixed19/*|"$ROOT"/target/androidbox-layout-mixed19/*) ;;
    *) echo "custom catalog output must stay in a catalog gate target directory" >&2; exit 1 ;;
esac

SDK_ROOT=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-"$HOME/Library/Android/sdk"}}
case "$SDK_ROOT" in
    /*) ;;
    *) echo "Android SDK root must be absolute: $SDK_ROOT" >&2; exit 1 ;;
esac
BUILD_TOOLS="$SDK_ROOT/build-tools/36.1.0"
AAPT2="$BUILD_TOOLS/aapt2"
D8="$BUILD_TOOLS/d8"
DEXDUMP="$BUILD_TOOLS/dexdump"
ZIPALIGN="$BUILD_TOOLS/zipalign"
APKSIGNER="$BUILD_TOOLS/apksigner"
ANDROID_JAR="$SDK_ROOT/platforms/android-36/android.jar"
JAVAC=/usr/bin/javac

for tool in \
    "$AAPT2" "$D8" "$DEXDUMP" "$ZIPALIGN" "$APKSIGNER" "$JAVAC" \
    /bin/cp /bin/mkdir /bin/rm \
    /usr/bin/awk /usr/bin/base64 /usr/bin/cmp /usr/bin/dirname /usr/bin/grep \
    /usr/bin/shasum /usr/bin/touch /usr/bin/tr /usr/bin/unzip \
    /usr/bin/wc /usr/bin/zip; do
    [ -x "$tool" ] || { echo "missing required offline tool: $tool" >&2; exit 1; }
done
[ -f "$ANDROID_JAR" ] || { echo "missing Android 36 platform jar" >&2; exit 1; }
[ -f "$SIGNING_KEY" ] && [ -f "$SIGNING_CERT" ] \
    || { echo "missing repository-owned test signing material" >&2; exit 1; }
case "$WORK_DIR" in
    "$ROOT"/target/androidbox-manifest-catalog-demo-build) ;;
    *) echo "refusing unsafe work directory: $WORK_DIR" >&2; exit 1 ;;
esac

umask 022
/bin/rm -rf "$WORK_DIR"
/bin/mkdir -p \
    "$WORK_DIR/compiled" "$WORK_DIR/generated" "$WORK_DIR/classes" \
    "$WORK_DIR/dex" "$WORK_DIR/link" "$WORK_DIR/stage" "$WORK_DIR/tmp" \
    "$WORK_DIR/res/drawable" "$WORK_DIR/res/drawable-mdpi" \
    "$WORK_DIR/res/drawable-xhdpi" "$WORK_DIR/res/layout" "$WORK_DIR/res/values"
export LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=946684800 TMPDIR="$WORK_DIR/tmp"

LAYOUT_SOURCE="$FIXTURE_DIR/res/layout/activity_catalog.xml"
if [ "$LAYOUT_ROWS" = 1 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-row/layout/activity_catalog.xml"
elif [ "$LAYOUT_ROWS" = 2 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-weight/layout/activity_catalog.xml"
elif [ "$LAYOUT_ROWS" = 3 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-spacing/layout/activity_catalog.xml"
elif [ "$LAYOUT_ROWS" = 4 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-directional/layout/activity_catalog.xml"
elif [ "$LAYOUT_ROWS" = 5 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-size/layout/activity_catalog.xml"
elif [ "$LAYOUT_ROWS" = 6 ]; then
    LAYOUT_SOURCE="$FIXTURE_DIR/res-mixed/layout/activity_catalog.xml"
fi
/bin/cp "$LAYOUT_SOURCE" "$WORK_DIR/res/layout/activity_catalog.xml"
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
    "$WORK_DIR/compiled/layout_activity_catalog.xml.flat" \
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

ACTIVITY_SOURCE="$FIXTURE_DIR/src/org/bndroid/catalog/MainActivity.java"
if [ "$DEX_METHODS" = 1 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-methods/org/bndroid/catalog/MainActivity.java"
elif [ "$DEX_METHODS" = 2 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-instance/org/bndroid/catalog/MainActivity.java"
elif [ "$DEX_METHODS" = 3 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-fields/org/bndroid/catalog/MainActivity.java"
elif [ "$DEX_METHODS" = 4 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-state/org/bndroid/catalog/MainActivity.java"
elif [ "$DEX_METHODS" = 5 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-string/org/bndroid/catalog/MainActivity.java"
elif [ "$DEX_METHODS" = 6 ]; then
    ACTIVITY_SOURCE="$FIXTURE_DIR/src-concat/org/bndroid/catalog/MainActivity.java"
fi
"$JAVAC" \
    --release 8 -g:none -Xlint:-options -encoding UTF-8 \
    -classpath "$ANDROID_JAR" \
    -d "$WORK_DIR/classes" \
    "$ACTIVITY_SOURCE" \
    "$WORK_DIR/generated/org/bndroid/catalog/R.java"

"$D8" \
    --release --min-api 26 --lib "$ANDROID_JAR" \
    --output "$WORK_DIR/dex" \
    "$WORK_DIR/classes/org/bndroid/catalog/MainActivity.class" \
    "$WORK_DIR/classes/org/bndroid/catalog/R.class" \
    "$WORK_DIR/classes/org/bndroid/catalog/R\$id.class" \
    "$WORK_DIR/classes/org/bndroid/catalog/R\$layout.class" \
    "$WORK_DIR/classes/org/bndroid/catalog/R\$string.class"

# Retain aapt2's normal DEFLATE binary XML entries and append a STORED,
# aligned classes.dex. This is a real Android SDK archive shape rather than a
# hand-authored binary Manifest fixture.
/bin/cp "$WORK_DIR/link/resources.apk" "$WORK_DIR/catalog-with-dex.apk"
/bin/cp "$WORK_DIR/dex/classes.dex" "$WORK_DIR/stage/classes.dex"
/usr/bin/touch -t 200001010000.00 "$WORK_DIR/stage/classes.dex"
(
    cd "$WORK_DIR/stage"
    /usr/bin/zip -X -q -0 "$WORK_DIR/catalog-with-dex.apk" classes.dex
)
"$ZIPALIGN" -p -f 4 \
    "$WORK_DIR/catalog-with-dex.apk" "$WORK_DIR/aligned-unsigned.apk"
"$ZIPALIGN" -c -p 4 "$WORK_DIR/aligned-unsigned.apk"

sign_fixture() {
    "$APKSIGNER" sign \
        --key "$SIGNING_KEY" --cert "$SIGNING_CERT" \
        --v1-signing-enabled false --v2-signing-enabled true \
        --v3-signing-enabled false --v4-signing-enabled false \
        --min-sdk-version 26 --out "$1" "$WORK_DIR/aligned-unsigned.apk"
}
sign_fixture "$WORK_DIR/catalog.apk"
sign_fixture "$WORK_DIR/catalog-second.apk"
/usr/bin/cmp -s "$WORK_DIR/catalog.apk" "$WORK_DIR/catalog-second.apk" \
    || { echo "non-deterministic APK v2 signature" >&2; exit 1; }

/usr/bin/unzip -t "$WORK_DIR/catalog.apk" >/dev/null
"$ZIPALIGN" -c -p 4 "$WORK_DIR/catalog.apk"
"$AAPT2" dump badging "$WORK_DIR/catalog.apk" > "$WORK_DIR/badging.txt"
"$AAPT2" dump xmltree --file AndroidManifest.xml \
    "$WORK_DIR/catalog.apk" > "$WORK_DIR/manifest-xmltree.txt"
"$AAPT2" dump xmltree --file res/layout/activity_catalog.xml \
    "$WORK_DIR/catalog.apk" > "$WORK_DIR/layout-xmltree.txt"
"$DEXDUMP" -d "$WORK_DIR/stage/classes.dex" > "$WORK_DIR/classes-dexdump.txt"
"$APKSIGNER" verify --verbose --print-certs --min-sdk-version 26 \
    "$WORK_DIR/catalog.apk" > "$WORK_DIR/signature-report.txt"

for expected in \
    "package: name='org.bndroid.catalog' versionCode='$VERSION_CODE' versionName='$VERSION_NAME'" \
    "application-label:'Component catalog'" \
    "launchable-activity: name='org.bndroid.catalog.MainActivity'"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/badging.txt" \
        || { echo "badging shape missing: $expected" >&2; exit 1; }
done

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
if [ "$DEX_METHODS" = 1 ]; then
    /usr/bin/grep -Fq "name          : 'statusTextFor'" \
        "$WORK_DIR/classes-dexdump.txt" \
        || { echo "DEX helper method is missing" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'invoke-static' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
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
    [ "$(/usr/bin/grep -Fc \
        'invoke-direct {v1, v2}, Lorg/bndroid/catalog/MainActivity;.statusTextFor:(Landroid/view/View;)I' \
        "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must contain one exact instance helper call" >&2; exit 1; }
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
        [ "$(/usr/bin/grep -Fc 'iget-object' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
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
        [ "$(/usr/bin/grep -Fc 'iget v' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
            || { echo "DEX callback must read the int field exactly once" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'iput v' "$WORK_DIR/classes-dexdump.txt")" -eq 2 ] \
            || { echo "DEX must initialize and then update the int field exactly once" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'add-int/lit8' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
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
    [ "$(/usr/bin/grep -Fc 'getId:()I // method@' "$WORK_DIR/classes-dexdump.txt")" -eq 0 ] \
        || { echo "DEX direct-string callback must not depend on a View ID" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'iget v' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must read the int field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'iput v' "$WORK_DIR/classes-dexdump.txt")" -eq 2 ] \
        || { echo "DEX must initialize and then update the int field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'iget-object' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must read the TextView field exactly once" >&2; exit 1; }
    [ "$(/usr/bin/grep -Ec 'add-int/(2addr|lit8)' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
        || { echo "DEX callback must increment the int field exactly once" >&2; exit 1; }
    if [ "$DEX_METHODS" = 5 ]; then
        [ "$(/usr/bin/grep -Fc 'const-string' "$WORK_DIR/classes-dexdump.txt")" -eq 2 ] \
            || { echo "DEX callback must select exactly two direct string literals" >&2; exit 1; }
    else
        for expected in \
            "new-instance" \
            "Ljava/lang/StringBuilder;.<init>:(Ljava/lang/String;)V" \
            "Ljava/lang/StringBuilder;.append:(I)Ljava/lang/StringBuilder;" \
            "Ljava/lang/StringBuilder;.toString:()Ljava/lang/String;"; do
            /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
                || { echo "DEX StringBuilder shape missing: $expected" >&2; exit 1; }
        done
        [ "$(/usr/bin/grep -Fc 'const-string' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
            || { echo "DEX dynamic callback must use one prefix literal" >&2; exit 1; }
        [ "$(/usr/bin/grep -Fc 'new-instance' "$WORK_DIR/classes-dexdump.txt")" -eq 1 ] \
            || { echo "DEX dynamic callback must allocate one StringBuilder" >&2; exit 1; }
    fi
fi
for expected in \
    "android.permission.INTERNET" \
    "android.permission.RECEIVE_BOOT_COMPLETED" \
    '".DetailActivity"' \
    '".AliasActivity"' \
    '".SyncService"' \
    '".BootReceiver"' \
    '".CatalogProvider"' \
    "org.bndroid.catalog.provider"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/manifest-xmltree.txt" \
        || { echo "compiled Manifest is missing: $expected" >&2; exit 1; }
done
[ "$(/usr/bin/grep -Fc 'E: activity ' "$WORK_DIR/manifest-xmltree.txt")" -eq 2 ] \
    || { echo "compiled Manifest must contain two activity declarations" >&2; exit 1; }
[ "$(/usr/bin/grep -Fc 'E: intent-filter ' "$WORK_DIR/manifest-xmltree.txt")" -eq 4 ] \
    || { echo "compiled Manifest must contain four intent filters" >&2; exit 1; }
for expected in \
    "Class descriptor  : 'Lorg/bndroid/catalog/MainActivity;'" \
    "name          : 'onCreate'" \
    "name          : 'onClick'" \
    "findViewById:(I)Landroid/view/View;" \
    "setOnClickListener:(Landroid/view/View\$OnClickListener;)V" \
    "setContentView:(I)V"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/classes-dexdump.txt" \
        || { echo "classes.dex is missing: $expected" >&2; exit 1; }
done
if [ "$DEX_METHODS" = 5 ] || [ "$DEX_METHODS" = 6 ]; then
    /usr/bin/grep -Fq "setText:(Ljava/lang/CharSequence;)V" \
        "$WORK_DIR/classes-dexdump.txt" \
        || { echo "classes.dex is missing CharSequence setText" >&2; exit 1; }
    [ "$(/usr/bin/grep -Fc 'setText:(I)V // method@' "$WORK_DIR/classes-dexdump.txt")" -eq 0 ] \
        || { echo "DEX direct-string mode must not call resource-ID setText" >&2; exit 1; }
else
    /usr/bin/grep -Fq "setText:(I)V" "$WORK_DIR/classes-dexdump.txt" \
        || { echo "classes.dex is missing resource-ID setText" >&2; exit 1; }
fi
for expected in \
    "Verified using v1 scheme (JAR signing): false" \
    "Verified using v2 scheme (APK Signature Scheme v2): true" \
    "Verified using v3 scheme (APK Signature Scheme v3): false" \
    "Verified using v4 scheme (APK Signature Scheme v4): false" \
    "Number of signers: 1" \
    "Signer #1 certificate SHA-256 digest: $EXPECTED_CERT_SHA256" \
    "Signer #1 public key SHA-256 digest: $EXPECTED_PUBLIC_KEY_SHA256"; do
    /usr/bin/grep -Fq "$expected" "$WORK_DIR/signature-report.txt" \
        || { echo "signature report is missing: $expected" >&2; exit 1; }
done

if [ "$ICON_RESOURCES" = 2 ]; then
    EXPECTED_ENTRIES=$(printf '%s\n' \
        AndroidManifest.xml \
        res/drawable-mdpi-v4/app_icon.png \
        res/drawable-xhdpi-v4/app_icon.png \
        res/layout/activity_catalog.xml \
        resources.arsc \
        classes.dex)
elif [ "$ICON_RESOURCES" = 1 ]; then
    EXPECTED_ENTRIES=$(printf '%s\n' \
        AndroidManifest.xml \
        res/drawable/app_icon.png \
        res/layout/activity_catalog.xml \
        resources.arsc \
        classes.dex)
else
    EXPECTED_ENTRIES=$(printf '%s\n' \
        AndroidManifest.xml \
        res/layout/activity_catalog.xml \
        resources.arsc \
        classes.dex)
fi
ACTUAL_ENTRIES=$(/usr/bin/unzip -Z1 "$WORK_DIR/catalog.apk")
[ "$ACTUAL_ENTRIES" = "$EXPECTED_ENTRIES" ] \
    || { echo "unexpected APK entry set" >&2; exit 1; }

APK_BYTES=$(/usr/bin/wc -c < "$WORK_DIR/catalog.apk" | /usr/bin/tr -d '[:space:]')
case "$APK_BYTES" in
    ''|*[!0-9]*) echo "could not determine APK size" >&2; exit 1 ;;
esac
[ "$APK_BYTES" -gt 0 ] && [ "$APK_BYTES" -le "$MAX_APK_BYTES" ] \
    || { echo "APK exceeds package-store bound: $APK_BYTES" >&2; exit 1; }

/bin/cp "$WORK_DIR/catalog.apk" "$OUTPUT_APK"
HASH=$(/usr/bin/shasum -a 256 "$OUTPUT_APK" | /usr/bin/awk '{print $1}')
if [ "$OUTPUT_APK" = "$CANONICAL_APK" ]; then
    printf '%s  %s\n' "$HASH" "androidbox-manifest-catalog-demo.apk" > "$CANONICAL_SHA"
fi

echo "androidbox_manifest_catalog_demo_apk=$OUTPUT_APK"
echo "androidbox_manifest_catalog_demo_bytes=$APK_BYTES"
echo "androidbox_manifest_catalog_demo_sha256=$HASH"
echo "androidbox_manifest_catalog_demo_certificate_sha256=$EXPECTED_CERT_SHA256"
echo "androidbox_manifest_catalog_demo_signature_scheme=v2-only-rsa2048-sha256"
echo "androidbox_manifest_catalog_demo_package=org.bndroid.catalog"
echo "androidbox_manifest_catalog_demo_version_code=$VERSION_CODE"
echo "androidbox_manifest_catalog_demo_components=6"
echo "androidbox_manifest_catalog_demo_launchers=2"
echo "androidbox_manifest_catalog_demo_permissions=2"
echo "androidbox_manifest_catalog_demo_icon_resources=$ICON_RESOURCES"
echo "androidbox_manifest_catalog_demo_dex_methods=$DEX_METHODS"
echo "androidbox_manifest_catalog_demo_layout_rows=$LAYOUT_ROWS"
echo "androidbox_manifest_catalog_demo_d8=$("$D8" --version)"
