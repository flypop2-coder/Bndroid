#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Library/Android/sdk}}"
OUT="${BNDROID_ART_PROBE_DIR:-$ROOT/target/art-runtime/fixture}"
D8="$SDK_ROOT/build-tools/36.1.0/d8"
[[ -x "$D8" ]] || { echo "Existing SDK build-tools 36.1.0 are required" >&2; exit 1; }
mkdir -p "$OUT/classes" "$OUT/dex"
javac --release 8 -d "$OUT/classes" "$ROOT/fixtures/art-runtime/Main.java"
"$D8" --min-api 35 --output "$OUT/dex" "$OUT/classes/Main.class"
cp "$OUT/dex/classes.dex" "$OUT/probe.dex"
if [[ "${BNDROID_ART_MODE:-java}" == framework* ]]; then
  mkdir -p "$OUT/framework-classes" "$OUT/framework-dex"
  javac --release 8 -classpath "$SDK_ROOT/platforms/android-36/android.jar" \
    -d "$OUT/framework-classes" "$ROOT/fixtures/art-runtime/FrameworkMain.java"
  "$D8" --min-api 35 --lib "$SDK_ROOT/platforms/android-36/android.jar" \
    --output "$OUT/framework-dex" "$OUT/framework-classes/FrameworkMain.class"
  cp "$OUT/framework-dex/classes.dex" "$OUT/framework.dex"
fi
NDK_ROOT="${BNDROID_NDK_ROOT:-$SDK_ROOT/ndk/27.1.12297006}"
case "$(uname -s)" in
  Darwin) HOST_TAG=darwin-x86_64 ;;
  Linux) HOST_TAG=linux-x86_64 ;;
  *) exit 2 ;;
esac
"$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -pthread -fPIE -pie \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
  "$ROOT/fixtures/art-runtime/memory.c" "$ROOT/fixtures/art-runtime/proc.c" "$ROOT/fixtures/art-runtime/paging.c" \
  "$ROOT/fixtures/art-runtime/remap.c" -o "$OUT/memory-probe"
"$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -pthread -fPIE -pie \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
  "$ROOT/fixtures/art-runtime/signals.c" "$ROOT/fixtures/art-runtime/signals.S" -o "$OUT/signals-probe"
"$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -fPIE -pie \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
  "$ROOT/fixtures/art-runtime/properties.c" -o "$OUT/properties-probe"
"$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -fPIE -pie \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
  "$ROOT/fixtures/art-runtime/memfd.c" -o "$OUT/memfd-probe"
if [[ "${BNDROID_ART_MODE:-java}" == managed ]]; then
  mkdir -p "$OUT/managed-classes" "$OUT/managed-dex"
  javac --release 8 -d "$OUT/managed-classes" "$ROOT/fixtures/art-runtime/ManagedMain.java"
  "$D8" --min-api 35 --output "$OUT/managed-dex" "$OUT/managed-classes/ManagedMain.class"
  cp "$OUT/managed-dex/classes.dex" "$OUT/managed.dex"
  "$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
    -O2 -g -Wall -Wextra -Werror -pthread -fPIC -shared \
    -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
    "$ROOT/fixtures/art-runtime/jni.c" -ldl -o "$OUT/libbndroidjni.so"
  BUILD_TOOLS="$SDK_ROOT/build-tools/36.1.0"
  "$BUILD_TOOLS/aapt2" link -o "$OUT/managed-resources.apk" \
    --manifest "$ROOT/fixtures/art-runtime/AndroidManifest.xml" \
    -I "$SDK_ROOT/platforms/android-36/android.jar"
  python3 "$ROOT/fixtures/art-runtime/package-apk.py" "$OUT"
  "$BUILD_TOOLS/zipalign" -P 16 -f 4 "$OUT/managed-unaligned.apk" "$OUT/managed-aligned.apk"
  "$BUILD_TOOLS/apksigner" sign \
    --key "$ROOT/fixtures/androidbox-resource-demo/test-only-apk-signing-key.pk8" \
    --cert "$ROOT/fixtures/androidbox-resource-demo/test-only-apk-signing-cert.pem" \
    --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled false --v4-signing-enabled false \
    --min-sdk-version 35 --out "$OUT/managed.apk" "$OUT/managed-aligned.apk"
  "$BUILD_TOOLS/apksigner" verify --verbose --print-certs "$OUT/managed.apk" > "$OUT/apk-signature.txt"
  "$BUILD_TOOLS/zipalign" -c -P 16 4 "$OUT/managed.apk"
  "$BUILD_TOOLS/aapt2" dump badging "$OUT/managed.apk" > "$OUT/apk-badging.txt"
fi
printf 'ART_DEX_BUILD_OK path=%s\n' "$OUT/probe.dex"
