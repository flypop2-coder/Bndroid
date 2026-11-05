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
BIN="$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG/bin"
OUT="${BNDROID_LINUX_PROBE_DIR:-$ROOT/target/linux-arm64-probe}"
[[ -x "$BIN/aarch64-linux-android35-clang" ]] || {
  echo "Install NDK 27.1.12297006 or set BNDROID_NDK_ROOT; no downloads are performed." >&2
  exit 1
}
mkdir -p "$OUT"
"$BIN/aarch64-linux-android35-clang" \
  -O2 -g -Wall -Wextra -Werror -ffreestanding -fno-builtin \
  -fno-stack-protector -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -nostdlib -static -Wl,--build-id=none \
  -Wl,--image-base=0x200000000 -Wl,-z,max-page-size=4096 \
  -Wl,-z,separate-loadable-segments -Wl,-e,_start \
  "$ROOT/fixtures/linux-arm64-probe/start.S" "$ROOT/fixtures/linux-arm64-probe/main.c" \
  -o "$OUT/linux-probe.elf"
"$BIN/llvm-readelf" -h -l "$OUT/linux-probe.elf" > "$OUT/elf.txt"
printf 'LINUX_PROBE_BUILD_OK elf=%s\n' "$OUT/linux-probe.elf"
