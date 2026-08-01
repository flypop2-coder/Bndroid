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
  python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 openssl cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed cmp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M76 key rotation runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M76 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
M75_FEATURE="unified-product-persistent-rollback-runtime"
M76_FEATURE="unified-product-key-rotation-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
SEED_TARGET_ROOT="${BNDROID_KEY_ROTATION_SEED_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-seed-check}"
TRANSITION_TARGET_ROOT="${BNDROID_KEY_ROTATION_TRANSITION_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-transition-check}"
ACTIVE_TARGET_ROOT="${BNDROID_KEY_ROTATION_ACTIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-key-rotation-runtime-check}"
SIGNATURE_TARGET_ROOT="${BNDROID_KEY_ROTATION_BAD_SIGNATURE_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-bad-signature-check}"
RETIRED_TARGET_ROOT="${BNDROID_KEY_ROTATION_RETIRED_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-retired-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m76"

SEED_ARTIFACT="$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex"
TRANSITION_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex"
ACTIVE_ARTIFACT="$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex"
SIGNATURE_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v5-key4-bad-signature.bms1.hex"
RETIRED_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v6-retired-key3.bms1.hex"
OFFLINE_REQUEST="$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.request.hex"
OFFLINE_SIGNATURE="$WORKSPACE_ROOT/boot/offline/product-service-manifest-v5-key4.signature.hex"
ACTIVE_PUBLIC_KEY="$WORKSPACE_ROOT/boot/trust/fixture-key4-public.pem"
ACTIVE_MODULUS_SHA256="36b7c88dc40ba32c77729d04ac7ceb90a21691d39abe94b17b12b1242f33c52f"

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

mkdir -p \
  "$SEED_TARGET_ROOT" \
  "$TRANSITION_TARGET_ROOT" \
  "$ACTIVE_TARGET_ROOT" \
  "$SIGNATURE_TARGET_ROOT" \
  "$RETIRED_TARGET_ROOT" \
  "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m76.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
SEEDED_IMAGE="$TMP_DIR/seeded.raw"
SIGNATURE_IMAGE="$TMP_DIR/signature.raw"
RETIRED_IMAGE="$TMP_DIR/retired.raw"
OFFLINE_GENERATED_REQUEST="$TMP_DIR/generated.request.hex"
OFFLINE_ASSEMBLED_ARTIFACT="$TMP_DIR/assembled.bms1.hex"

SEED_SERIAL="$TMP_DIR/seed.serial.log"
TRANSITION_SERIAL="$TMP_DIR/transition.serial.log"
ACTIVATE_SERIAL="$TMP_DIR/activate.serial.log"
REPAIR_SERIAL="$TMP_DIR/repair.serial.log"
STEADY_SERIAL="$TMP_DIR/steady.serial.log"
SIGNATURE_SERIAL="$TMP_DIR/signature.serial.log"
RETIRED_SERIAL="$TMP_DIR/retired.serial.log"
SEED_DRIVER="$TMP_DIR/seed.driver.log"
TRANSITION_DRIVER="$TMP_DIR/transition.driver.log"
ACTIVATE_DRIVER="$TMP_DIR/activate.driver.log"
REPAIR_DRIVER="$TMP_DIR/repair.driver.log"
STEADY_DRIVER="$TMP_DIR/steady.driver.log"
SIGNATURE_DRIVER="$TMP_DIR/signature.driver.log"
RETIRED_DRIVER="$TMP_DIR/retired.driver.log"
SEED_QMP="$TMP_DIR/seed.qmp"
TRANSITION_QMP="$TMP_DIR/transition.qmp"
ACTIVATE_QMP="$TMP_DIR/activate.qmp"
REPAIR_QMP="$TMP_DIR/repair.qmp"
STEADY_QMP="$TMP_DIR/steady.qmp"
SIGNATURE_QMP="$TMP_DIR/signature.qmp"
RETIRED_QMP="$TMP_DIR/retired.qmp"
SEED_SCREENSHOT="$TMP_DIR/seed.ppm"
TRANSITION_SCREENSHOT="$TMP_DIR/transition.ppm"
ACTIVATE_SCREENSHOT="$TMP_DIR/activate.ppm"
REPAIR_SCREENSHOT="$TMP_DIR/repair.ppm"
STEADY_SCREENSHOT="$TMP_DIR/steady.ppm"
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M76 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/key_rotation_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" \
  "$SCRIPT_DIR/offline_service_manifest_signing.py"
python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" self-test

ARTIFACT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" artifacts \
    "$TRANSITION_ARTIFACT" "$ACTIVE_ARTIFACT" "$RETIRED_ARTIFACT" \
    "$SIGNATURE_ARTIFACT" "$OFFLINE_REQUEST" "$OFFLINE_SIGNATURE"
)"
printf '%s\n' "$ARTIFACT_MARKER"
OFFLINE_REQUEST_MARKER="$(
  python3 "$SCRIPT_DIR/offline_service_manifest_signing.py" prepare \
    --generation 5 \
    --rollback-index 5 \
    --key-id 4 \
    --request "$OFFLINE_GENERATED_REQUEST"
)"
printf '%s\n' "$OFFLINE_REQUEST_MARKER"
cmp "$OFFLINE_REQUEST" "$OFFLINE_GENERATED_REQUEST"
OFFLINE_ASSEMBLY_MARKER="$(
  python3 "$SCRIPT_DIR/offline_service_manifest_signing.py" assemble \
    --request "$OFFLINE_REQUEST" \
    --signature "$OFFLINE_SIGNATURE" \
    --public-key "$ACTIVE_PUBLIC_KEY" \
    --key-id 4 \
    --expected-modulus-sha256 "$ACTIVE_MODULUS_SHA256" \
    --output "$OFFLINE_ASSEMBLED_ARTIFACT"
)"
printf '%s\n' "$OFFLINE_ASSEMBLY_MARKER"
cmp "$ACTIVE_ARTIFACT" "$OFFLINE_ASSEMBLED_ARTIFACT"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

build_kernel() {
  local target_root="$1"
  local feature="$2"
  local artifact="$3"

  CARGO_TARGET_DIR="$target_root" \
    BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$artifact" \
    BNDROID_PROFILE="$PROFILE" \
    BNDROID_KERNEL_FEATURES="$feature" \
    BNDROID_USERSPACE_FEATURES="$feature" \
    "$SCRIPT_DIR/build-kernel.sh"
}

build_kernel "$SEED_TARGET_ROOT" "$M75_FEATURE" "$SEED_ARTIFACT"
build_kernel "$TRANSITION_TARGET_ROOT" "$M76_FEATURE" "$TRANSITION_ARTIFACT"
build_kernel "$ACTIVE_TARGET_ROOT" "$M76_FEATURE" "$ACTIVE_ARTIFACT"
build_kernel "$SIGNATURE_TARGET_ROOT" "$M76_FEATURE" "$SIGNATURE_ARTIFACT"
build_kernel "$RETIRED_TARGET_ROOT" "$M76_FEATURE" "$RETIRED_ARTIFACT"

SEED_KERNEL="$SEED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
TRANSITION_KERNEL="$TRANSITION_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
ACTIVE_KERNEL="$ACTIVE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SIGNATURE_KERNEL="$SIGNATURE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
RETIRED_KERNEL="$RETIRED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
for image in \
  "$SEED_KERNEL" \
  "$TRANSITION_KERNEL" \
  "$ACTIVE_KERNEL" \
  "$SIGNATURE_KERNEL" \
  "$RETIRED_KERNEL"; do
  if [[ ! -f "$image" ]]; then
    echo "M76 kernel image is missing: $image" >&2
    exit 1
  fi
done

run_positive() {
  local phase="$1"
  local milestone="$2"
  local kernel="$3"
  local serial="$4"
  local driver="$5"
  local qmp="$6"
  local screenshot="$7"
  local expected_driver

  expected_driver="UNIFIED_PRODUCT_QMP_BOOT_OK phase=$phase qemu_self_exit=1 "
  expected_driver+="power_key=116 ui_sha256=$FINAL_SCREEN_SHA256 "
  expected_driver+="maintenance_rotations=2 cancel_pending=4"
  python3 "$SCRIPT_DIR/unified_product_qmp.py" \
    --kernel "$kernel" \
    --disk "$RUNTIME_IMAGE" \
    --serial "$serial" \
    --qmp "$qmp" \
    --screenshot "$screenshot" \
    --phase "$phase" \
    --milestone "$milestone" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M76 $phase boot driver evidence changed." >&2
    exit 1
  fi
  if [[ "$milestone" == "M75" ]]; then
    python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
      parse first "$serial"
  else
    python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" \
      parse "$phase" "$serial"
  fi
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

  if [[ "$reason" == "retired-key" ]]; then
    signature_valid=1
  else
    signature_valid=0
  fi
  expected_driver="KEY_ROTATION_NEGATIVE_BOOT_OK reason=$reason "
  expected_driver+="pre_el0=1 signature_valid=$signature_valid "
  expected_driver+="manifest_published=0 init_ready=0 policy_slots_mutated=0 "
  expected_driver+="qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
  python3 "$SCRIPT_DIR/key_rotation_negative_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --reason "$reason" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M76 $reason negative host evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" \
    parse-negative "$reason" "$serial" "$driver"
}

run_positive first M75 "$SEED_KERNEL" \
  "$SEED_SERIAL" "$SEED_DRIVER" "$SEED_QMP" "$SEED_SCREENSHOT"
cp "$RUNTIME_IMAGE" "$SEEDED_IMAGE"
run_positive transition M76 "$TRANSITION_KERNEL" \
  "$TRANSITION_SERIAL" "$TRANSITION_DRIVER" "$TRANSITION_QMP" "$TRANSITION_SCREENSHOT"
run_positive activate M76 "$ACTIVE_KERNEL" \
  "$ACTIVATE_SERIAL" "$ACTIVATE_DRIVER" "$ACTIVATE_QMP" "$ACTIVATE_SCREENSHOT"
run_positive repair M76 "$ACTIVE_KERNEL" \
  "$REPAIR_SERIAL" "$REPAIR_DRIVER" "$REPAIR_QMP" "$REPAIR_SCREENSHOT"
run_positive steady M76 "$ACTIVE_KERNEL" \
  "$STEADY_SERIAL" "$STEADY_DRIVER" "$STEADY_QMP" "$STEADY_SCREENSHOT"

# Both rejected boots start from the exact fully replicated key4 policy.
# Ordinary boot-health records may advance, so the parser compares only the
# kernel-reserved key-policy sectors byte-for-byte.
cp "$RUNTIME_IMAGE" "$SIGNATURE_IMAGE"
cp "$RUNTIME_IMAGE" "$RETIRED_IMAGE"
run_negative signature "$SIGNATURE_KERNEL" "$SIGNATURE_IMAGE" \
  "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" "$SIGNATURE_QMP"
run_negative retired-key "$RETIRED_KERNEL" "$RETIRED_IMAGE" \
  "$RETIRED_SERIAL" "$RETIRED_DRIVER" "$RETIRED_QMP"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" reboot \
    "$PRISTINE_IMAGE" "$SEEDED_IMAGE" "$RUNTIME_IMAGE" \
    "$SIGNATURE_IMAGE" "$RETIRED_IMAGE" \
    "$SEED_SERIAL" "$SEED_DRIVER" \
    "$TRANSITION_SERIAL" "$ACTIVATE_SERIAL" "$REPAIR_SERIAL" "$STEADY_SERIAL" \
    "$TRANSITION_DRIVER" "$ACTIVATE_DRIVER" "$REPAIR_DRIVER" "$STEADY_DRIVER" \
    "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" \
    "$RETIRED_SERIAL" "$RETIRED_DRIVER"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "$ARTIFACT_MARKER"
  printf '%s\n' "$OFFLINE_REQUEST_MARKER"
  printf '%s\n' "$OFFLINE_ASSEMBLY_MARKER"
  printf '%s\n' "M76_BOOT_SEQUENCE phase=seed-m75 qemu_psci_self_exit=1"
  tr -d '\r' <"$SEED_SERIAL"
  sed '/^$/d' "$SEED_DRIVER"
  for phase in transition activate repair steady; do
    printf '%s\n' "M76_BOOT_SEQUENCE phase=$phase qemu_psci_self_exit=1"
    case "$phase" in
      transition)
        tr -d '\r' <"$TRANSITION_SERIAL"
        sed '/^$/d' "$TRANSITION_DRIVER"
        ;;
      activate)
        tr -d '\r' <"$ACTIVATE_SERIAL"
        sed '/^$/d' "$ACTIVATE_DRIVER"
        ;;
      repair)
        tr -d '\r' <"$REPAIR_SERIAL"
        sed '/^$/d' "$REPAIR_DRIVER"
        ;;
      steady)
        tr -d '\r' <"$STEADY_SERIAL"
        sed '/^$/d' "$STEADY_DRIVER"
        ;;
    esac
  done
  printf '%s\n' "M76_NEGATIVE_BOOT_SEQUENCE reason=signature host_terminated=1"
  tr -d '\r' <"$SIGNATURE_SERIAL"
  sed '/^$/d' "$SIGNATURE_DRIVER"
  printf '%s\n' "M76_NEGATIVE_BOOT_SEQUENCE reason=retired-key host_terminated=1"
  tr -d '\r' <"$RETIRED_SERIAL"
  sed '/^$/d' "$RETIRED_DRIVER"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-key-rotation-runtime.log"
cp "$SEED_SCREENSHOT" "$OUTPUT_DIR/seed.ppm"
cp "$TRANSITION_SCREENSHOT" "$OUTPUT_DIR/transition.ppm"
cp "$ACTIVATE_SCREENSHOT" "$OUTPUT_DIR/activate.ppm"
cp "$REPAIR_SCREENSHOT" "$OUTPUT_DIR/repair.ppm"
cp "$STEADY_SCREENSHOT" "$OUTPUT_DIR/steady.ppm"

RUN_SUCCEEDED=1
echo "M76 staged key rotation, revocation, and offline-signing gate passed."
