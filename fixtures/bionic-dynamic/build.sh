#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Library/Android/sdk}}"
NDK_ROOT="${BNDROID_NDK_ROOT:-$SDK_ROOT/ndk/27.1.12297006}"
case "$(uname -s)" in
  Darwin) HOST_TAG=darwin-x86_64 ;;
  Linux) HOST_TAG=linux-x86_64 ;;
  *) echo "Unsupported NDK host" >&2; exit 2 ;;
esac
TOOLCHAIN="$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG"
OUT="${BNDROID_DYNAMIC_PROBE_DIR:-$ROOT/target/bionic-dynamic}"
[[ -x "$TOOLCHAIN/bin/aarch64-linux-android35-clang" ]] || {
  echo "Install NDK 27.1.12297006 or set BNDROID_NDK_ROOT; no downloads are performed." >&2; exit 1;
}
mkdir -p "$OUT"
CC="$TOOLCHAIN/bin/aarch64-linux-android35-clang"
FLAGS=(-std=c11 -O2 -g -Wall -Wextra -Werror -Wl,-z,max-page-size=4096 -Wl,-z,separate-loadable-segments)
case "${BNDROID_DYNAMIC_FAULT_CHECK:-0}" in
  0) ;;
  1) FLAGS+=(-DBNDROID_TEST_EOF_FAULT) ;;
  *) echo "BNDROID_DYNAMIC_FAULT_CHECK must be 0 or 1" >&2; exit 2 ;;
esac
EXTRA_SOURCES=()
THREAD_MODE=0
case "${BNDROID_THREAD_TEST_MODE:-normal}" in
  normal) ;;
  leader-exit) THREAD_MODE=1 ;;
  group-exit) THREAD_MODE=2 ;;
  worker-fault) THREAD_MODE=3 ;;
  *) echo "Unknown BNDROID_THREAD_TEST_MODE" >&2; exit 2 ;;
esac
case "${BNDROID_DYNAMIC_THREADS_CHECK:-0}" in
  0) ;;
  1) FLAGS+=(-DBNDROID_TEST_THREADS -DBNDROID_THREAD_TEST_MODE="$THREAD_MODE" -pthread); EXTRA_SOURCES+=("$ROOT/fixtures/bionic-dynamic/threads.c") ;;
  *) echo "BNDROID_DYNAMIC_THREADS_CHECK must be 0 or 1" >&2; exit 2 ;;
esac
"$CC" "${FLAGS[@]}" -fPIC -shared -Wl,-soname,libbndroidplugin.so \
  "$ROOT/fixtures/bionic-dynamic/plugin.c" -o "$OUT/libbndroidplugin.so"
"$CC" "${FLAGS[@]}" -fPIE -pie "$ROOT/fixtures/bionic-dynamic/main.c" ${EXTRA_SOURCES[@]+"${EXTRA_SOURCES[@]}"} -ldl -lm -o "$OUT/dynamic-probe"
"$TOOLCHAIN/bin/llvm-readelf" -h -l -d "$OUT/dynamic-probe" > "$OUT/elf.txt"
"$CC" --version > "$OUT/compiler.txt"
cp "$NDK_ROOT/source.properties" "$OUT/ndk.properties"
printf 'DYNAMIC_PROBE_BUILD_OK elf=%s\n' "$OUT/dynamic-probe"
