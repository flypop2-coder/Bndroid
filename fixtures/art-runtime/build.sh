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
NDK_ROOT="${BNDROID_NDK_ROOT:-$SDK_ROOT/ndk/27.1.12297006}"
case "$(uname -s)" in
  Darwin) HOST_TAG=darwin-x86_64 ;;
  Linux) HOST_TAG=linux-x86_64 ;;
  *) exit 2 ;;
esac
"$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -pthread -fPIE -pie \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments \
  "$ROOT/fixtures/art-runtime/memory.c" "$ROOT/fixtures/art-runtime/proc.c" -o "$OUT/memory-probe"
printf 'ART_DEX_BUILD_OK path=%s\n' "$OUT/probe.dex"
