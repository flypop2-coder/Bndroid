#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

usage() {
  echo "usage: $0 [--parser-self-test | --parse PHASE LOG]" >&2
}

if [[ "${1:-}" == "--parser-self-test" && "$#" == 1 ]]; then
  python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M68 unified product liveness runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M68 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
FEATURE="unified-product-liveness-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
TARGET_ROOT="${BNDROID_UNIFIED_PRODUCT_LIVENESS_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-liveness-runtime-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m68"

if [[ ! "$BOOT_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] \
  || ((BOOT_TIMEOUT_SECONDS > 600)); then
  echo "BNDROID_QEMU_TIMEOUT_SECONDS must be an integer from 1 through 600." >&2
  exit 2
fi
if [[ "$QEMU_CPU" != "cortex-a72" && "$QEMU_CPU" != "max" ]]; then
  echo "BNDROID_QEMU_CPU must be 'cortex-a72' or 'max'." >&2
  exit 2
fi
if [[ "$KEEP_TEMP" != "0" && "$KEEP_TEMP" != "1" ]]; then
  echo "BNDROID_KEEP_TEMP must be 0 or 1." >&2
  exit 2
fi

mkdir -p "$TARGET_ROOT" "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m68.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
FIRST_SERIAL="$TMP_DIR/boot-1.serial.log"
SECOND_SERIAL="$TMP_DIR/boot-2.serial.log"
FIRST_DRIVER="$TMP_DIR/boot-1.driver.log"
SECOND_DRIVER="$TMP_DIR/boot-2.driver.log"
FIRST_QMP="$TMP_DIR/boot-1.qmp"
SECOND_QMP="$TMP_DIR/boot-2.qmp"
FIRST_SCREENSHOT="$TMP_DIR/boot-1.ppm"
SECOND_SCREENSHOT="$TMP_DIR/boot-2.ppm"
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M68 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/unified_product_liveness_evidence.py"
python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" self-test
python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

CARGO_TARGET_DIR="$TARGET_ROOT" \
  BNDROID_PROFILE="$PROFILE" \
  BNDROID_KERNEL_FEATURES="$FEATURE" \
  BNDROID_USERSPACE_FEATURES="$FEATURE" \
  "$SCRIPT_DIR/build-kernel.sh"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "M68 kernel image is missing: $KERNEL_IMAGE" >&2
  exit 1
fi

run_boot() {
  local phase="$1"
  local serial="$2"
  local driver="$3"
  local qmp="$4"
  local screenshot="$5"
  local expected_driver

  expected_driver="UNIFIED_PRODUCT_QMP_BOOT_OK phase=$phase qemu_self_exit=1 "
  expected_driver+="power_key=116 ui_sha256=$FINAL_SCREEN_SHA256"
  python3 "$SCRIPT_DIR/unified_product_qmp.py" \
    --kernel "$KERNEL_IMAGE" \
    --disk "$RUNTIME_IMAGE" \
    --serial "$serial" \
    --qmp "$qmp" \
    --screenshot "$screenshot" \
    --phase "$phase" \
    --milestone M68 \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M68 $phase boot driver evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" parse "$phase" "$serial"
}

run_boot first "$FIRST_SERIAL" "$FIRST_DRIVER" "$FIRST_QMP" "$FIRST_SCREENSHOT"
run_boot second "$SECOND_SERIAL" "$SECOND_DRIVER" "$SECOND_QMP" "$SECOND_SCREENSHOT"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_liveness_evidence.py" \
    disk "$PRISTINE_IMAGE" "$RUNTIME_IMAGE"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "M68_BOOT_SEQUENCE phase=first qemu_self_exit=1"
  tr -d '\r' <"$FIRST_SERIAL"
  sed '/^$/d' "$FIRST_DRIVER"
  printf '%s\n' "M68_BOOT_SEQUENCE phase=second qemu_self_exit=1"
  tr -d '\r' <"$SECOND_SERIAL"
  sed '/^$/d' "$SECOND_DRIVER"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-liveness-runtime.log"

RUN_SUCCEEDED=1
echo "M68 unified product two-boot liveness/UI/AppData/shutdown self-exit test passed."
