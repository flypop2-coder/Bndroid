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
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
    parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M75 persistent rollback runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M75 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
FEATURE="unified-product-persistent-rollback-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
TARGET_ROOT="${BNDROID_UNIFIED_PRODUCT_PERSISTENT_ROLLBACK_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-persistent-rollback-runtime-check}"
SIGNATURE_TARGET_ROOT="${BNDROID_PERSISTENT_ROLLBACK_BAD_SIGNATURE_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-persistent-rollback-bad-signature-check}"
ROLLBACK_TARGET_ROOT="${BNDROID_PERSISTENT_ROLLBACK_OLD_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-persistent-rollback-old-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m75"
CURRENT_ARTIFACT="$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex"
OLD_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v2-persistent-rollback.bms1.hex"
SIGNATURE_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v3-persistent-bad-signature.bms1.hex"

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

mkdir -p "$TARGET_ROOT" "$SIGNATURE_TARGET_ROOT" "$ROLLBACK_TARGET_ROOT" "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m75.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
SIGNATURE_IMAGE="$TMP_DIR/signature.raw"
ROLLBACK_IMAGE="$TMP_DIR/rollback.raw"
FIRST_SERIAL="$TMP_DIR/boot-1.serial.log"
SECOND_SERIAL="$TMP_DIR/boot-2.serial.log"
STEADY_SERIAL="$TMP_DIR/boot-3.serial.log"
SIGNATURE_SERIAL="$TMP_DIR/signature.serial.log"
ROLLBACK_SERIAL="$TMP_DIR/rollback.serial.log"
FIRST_DRIVER="$TMP_DIR/boot-1.driver.log"
SECOND_DRIVER="$TMP_DIR/boot-2.driver.log"
STEADY_DRIVER="$TMP_DIR/boot-3.driver.log"
SIGNATURE_DRIVER="$TMP_DIR/signature.driver.log"
ROLLBACK_DRIVER="$TMP_DIR/rollback.driver.log"
FIRST_QMP="$TMP_DIR/boot-1.qmp"
SECOND_QMP="$TMP_DIR/boot-2.qmp"
STEADY_QMP="$TMP_DIR/boot-3.qmp"
SIGNATURE_QMP="$TMP_DIR/signature.qmp"
ROLLBACK_QMP="$TMP_DIR/rollback.qmp"
FIRST_SCREENSHOT="$TMP_DIR/boot-1.ppm"
SECOND_SCREENSHOT="$TMP_DIR/boot-2.ppm"
STEADY_SCREENSHOT="$TMP_DIR/boot-3.ppm"
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M75 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/persistent_rollback_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py"
python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" self-test
ARTIFACT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" artifacts \
    "$CURRENT_ARTIFACT" "$OLD_ARTIFACT" "$SIGNATURE_ARTIFACT"
)"
printf '%s\n' "$ARTIFACT_MARKER"
python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

build_kernel() {
  local target_root="$1"
  local artifact="$2"

  CARGO_TARGET_DIR="$target_root" \
    BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$artifact" \
    BNDROID_PROFILE="$PROFILE" \
    BNDROID_KERNEL_FEATURES="$FEATURE" \
    BNDROID_USERSPACE_FEATURES="$FEATURE" \
    "$SCRIPT_DIR/build-kernel.sh"
}

build_kernel "$TARGET_ROOT" "$CURRENT_ARTIFACT"
build_kernel "$SIGNATURE_TARGET_ROOT" "$SIGNATURE_ARTIFACT"
build_kernel "$ROLLBACK_TARGET_ROOT" "$OLD_ARTIFACT"

KERNEL_IMAGE="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SIGNATURE_KERNEL="$SIGNATURE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
ROLLBACK_KERNEL="$ROLLBACK_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
for image in "$KERNEL_IMAGE" "$SIGNATURE_KERNEL" "$ROLLBACK_KERNEL"; do
  if [[ ! -f "$image" ]]; then
    echo "M75 kernel image is missing: $image" >&2
    exit 1
  fi
done

run_boot() {
  local phase="$1"
  local serial="$2"
  local driver="$3"
  local qmp="$4"
  local screenshot="$5"
  local expected_driver

  expected_driver="UNIFIED_PRODUCT_QMP_BOOT_OK phase=$phase qemu_self_exit=1 "
  expected_driver+="power_key=116 ui_sha256=$FINAL_SCREEN_SHA256 "
  expected_driver+="maintenance_rotations=2 cancel_pending=4"
  python3 "$SCRIPT_DIR/unified_product_qmp.py" \
    --kernel "$KERNEL_IMAGE" \
    --disk "$RUNTIME_IMAGE" \
    --serial "$serial" \
    --qmp "$qmp" \
    --screenshot "$screenshot" \
    --phase "$phase" \
    --milestone M75 \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M75 $phase boot driver evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
    parse "$phase" "$serial"
}

run_negative() {
  local reason="$1"
  local kernel="$2"
  local disk="$3"
  local serial="$4"
  local driver="$5"
  local qmp="$6"
  local signature_valid
  local expected_driver

  if [[ "$reason" == "rollback" ]]; then
    signature_valid=1
  else
    signature_valid=0
  fi
  expected_driver="PERSISTENT_ROLLBACK_NEGATIVE_BOOT_OK reason=$reason "
  expected_driver+="pre_el0=1 signature_valid=$signature_valid "
  expected_driver+="manifest_published=0 init_ready=0 "
  expected_driver+="qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
  python3 "$SCRIPT_DIR/persistent_rollback_negative_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --reason "$reason" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M75 $reason negative host evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
    parse-negative "$reason" "$serial" "$driver"
}

run_boot first "$FIRST_SERIAL" "$FIRST_DRIVER" "$FIRST_QMP" "$FIRST_SCREENSHOT"
run_boot second "$SECOND_SERIAL" "$SECOND_DRIVER" "$SECOND_QMP" "$SECOND_SCREENSHOT"
run_boot steady "$STEADY_SERIAL" "$STEADY_DRIVER" "$STEADY_QMP" "$STEADY_SCREENSHOT"

# Both rejected boots start from the exact fully provisioned two-slot state.
# Their ordinary boot-health record may advance before rejection, so the
# evidence parser compares only the reserved rollback sectors byte-for-byte.
cp "$RUNTIME_IMAGE" "$SIGNATURE_IMAGE"
cp "$RUNTIME_IMAGE" "$ROLLBACK_IMAGE"
run_negative signature "$SIGNATURE_KERNEL" "$SIGNATURE_IMAGE" \
  "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" "$SIGNATURE_QMP"
run_negative rollback "$ROLLBACK_KERNEL" "$ROLLBACK_IMAGE" \
  "$ROLLBACK_SERIAL" "$ROLLBACK_DRIVER" "$ROLLBACK_QMP"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" reboot \
    "$PRISTINE_IMAGE" "$RUNTIME_IMAGE" "$SIGNATURE_IMAGE" "$ROLLBACK_IMAGE" \
    "$FIRST_SERIAL" "$SECOND_SERIAL" "$STEADY_SERIAL" \
    "$FIRST_DRIVER" "$SECOND_DRIVER" "$STEADY_DRIVER" \
    "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" \
    "$ROLLBACK_SERIAL" "$ROLLBACK_DRIVER"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "$ARTIFACT_MARKER"
  printf '%s\n' "M75_BOOT_SEQUENCE phase=first qemu_psci_self_exit=1"
  tr -d '\r' <"$FIRST_SERIAL"
  sed '/^$/d' "$FIRST_DRIVER"
  printf '%s\n' "M75_BOOT_SEQUENCE phase=second qemu_psci_self_exit=1"
  tr -d '\r' <"$SECOND_SERIAL"
  sed '/^$/d' "$SECOND_DRIVER"
  printf '%s\n' "M75_BOOT_SEQUENCE phase=steady qemu_psci_self_exit=1"
  tr -d '\r' <"$STEADY_SERIAL"
  sed '/^$/d' "$STEADY_DRIVER"
  printf '%s\n' "M75_NEGATIVE_BOOT_SEQUENCE reason=signature host_terminated=1"
  tr -d '\r' <"$SIGNATURE_SERIAL"
  sed '/^$/d' "$SIGNATURE_DRIVER"
  printf '%s\n' "M75_NEGATIVE_BOOT_SEQUENCE reason=rollback host_terminated=1"
  tr -d '\r' <"$ROLLBACK_SERIAL"
  sed '/^$/d' "$ROLLBACK_DRIVER"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-persistent-rollback-runtime.log"
cp "$FIRST_SCREENSHOT" "$OUTPUT_DIR/boot-1.ppm"
cp "$SECOND_SCREENSHOT" "$OUTPUT_DIR/boot-2.ppm"
cp "$STEADY_SCREENSHOT" "$OUTPUT_DIR/boot-3.ppm"

RUN_SUCCEEDED=1
echo "M75 persistent rollback three-boot and fail-closed gate passed."
