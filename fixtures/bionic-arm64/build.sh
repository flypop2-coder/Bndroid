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
OUT="${BNDROID_BIONIC_PROBE_DIR:-$ROOT/target/bionic-arm64}"
[[ -x "$TOOLCHAIN/bin/aarch64-linux-android35-clang" ]] || {
  echo "Install NDK 27.1.12297006 or set BNDROID_NDK_ROOT; no downloads are performed." >&2
  exit 1
}
mkdir -p "$OUT"
"$TOOLCHAIN/bin/aarch64-linux-android35-clang" \
  -std=c11 -O2 -g -Wall -Wextra -Werror -static \
  -Wl,--image-base=0x200000000 -Wl,-z,max-page-size=4096 \
  -Wl,-z,separate-loadable-segments -Wl,--strip-debug -Wl,-Map,"$OUT/link.map" \
  "$ROOT/fixtures/bionic-arm64/main.c" -o "$OUT/bionic-probe.elf"
"$TOOLCHAIN/bin/llvm-readelf" -h -l "$OUT/bionic-probe.elf" > "$OUT/elf.txt"
"$TOOLCHAIN/bin/clang" --version > "$OUT/compiler.txt"
python3 - "$NDK_ROOT" "$TOOLCHAIN" "$OUT" <<'PY'
import hashlib, json, pathlib, sys
ndk, toolchain, out = map(pathlib.Path, sys.argv[1:])
root = toolchain / 'sysroot/usr/lib/aarch64-linux-android'
inputs = ['libc.a', 'libm.a', 'libdl.a', '35/crtbegin_static.o', '35/crtend_android.o']
receipt = {'ndk': (ndk / 'source.properties').read_text(), 'android_api': 35,
           'link_mode': 'static-unmodified-ndk-crt-libc',
           'input_sha256': {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in inputs}}
(out / 'build-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
PY
printf 'BIONIC_PROBE_BUILD_OK elf=%s\n' "$OUT/bionic-probe.elf"
