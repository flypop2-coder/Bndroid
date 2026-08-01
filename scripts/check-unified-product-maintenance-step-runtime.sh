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
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M79 maintenance-step runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M79 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
M75_FEATURE="unified-product-persistent-rollback-runtime"
M76_FEATURE="unified-product-key-rotation-runtime"
M78_FEATURE="unified-product-maintenance-execution-runtime"
M79_FEATURE="unified-product-maintenance-step-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"

SEED_TARGET_ROOT="${BNDROID_M79_SEED_TARGET_DIR:-$WORKSPACE_ROOT/target/m79-key-rotation-seed-check}"
TRANSITION_TARGET_ROOT="${BNDROID_M79_TRANSITION_TARGET_DIR:-$WORKSPACE_ROOT/target/m79-key-rotation-transition-check}"
ACTIVE_TARGET_ROOT="${BNDROID_M79_ACTIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/m79-key-rotation-active-check}"
SEQUENCE1_TARGET_ROOT="${BNDROID_M79_SEQUENCE1_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-step-sequence1-check}"
SEQUENCE2_TARGET_ROOT="${BNDROID_M79_SEQUENCE2_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-step-sequence2-check}"
SIGNATURE_TARGET_ROOT="${BNDROID_M79_BAD_SIGNATURE_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-step-bad-signature-check}"
BINDING_TARGET_ROOT="${BNDROID_M79_WRONG_BINDING_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-step-wrong-binding-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m79"

SEED_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex"
TRANSITION_MANIFEST="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex"
ACTIVE_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex"
SEQUENCE1_ARTIFACT="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence1.bma1.hex"
SEQUENCE2_ARTIFACT="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex"
BAD_SIGNATURE_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/maintenance-authorization-sequence2-bad-signature.bma1.hex"
WRONG_BINDING_ARTIFACT="$WORKSPACE_ROOT/boot/test-fixtures/maintenance-authorization-sequence3-wrong-binding.bma1.hex"
REQUEST1="$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence1.request.hex"
SIGNATURE1="$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence1.signature.hex"
REQUEST2="$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence2.request.hex"
SIGNATURE2="$WORKSPACE_ROOT/boot/maintenance/maintenance-authorization-sequence2.signature.hex"

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
  "$SEQUENCE1_TARGET_ROOT" \
  "$SEQUENCE2_TARGET_ROOT" \
  "$SIGNATURE_TARGET_ROOT" \
  "$BINDING_TARGET_ROOT" \
  "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m79.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
BASE_IMAGE="$TMP_DIR/m78-sequence1.raw"
NORMAL_IMAGE="$TMP_DIR/m79-normal.raw"
CUT1_IMAGE="$TMP_DIR/m79-cut1.raw"
RECOVER1_IMAGE="$TMP_DIR/m79-recover1.raw"
CUT2_IMAGE="$TMP_DIR/m79-cut2.raw"
RECOVER2_IMAGE="$TMP_DIR/m79-recover2.raw"
CUT3_IMAGE="$TMP_DIR/m79-cut3.raw"
RECOVER3_IMAGE="$TMP_DIR/m79-recover3.raw"
CORRUPT_IMAGE="$TMP_DIR/m79-corrupt-step2.raw"
RECOVER_CORRUPT_IMAGE="$TMP_DIR/m79-recover-corrupt.raw"
SIGNATURE_IMAGE="$TMP_DIR/m79-signature.raw"
BINDING_IMAGE="$TMP_DIR/m79-binding.raw"
REPLAY_IMAGE="$TMP_DIR/m79-completed-replay.raw"

SEED_SERIAL="$TMP_DIR/seed.serial.log"
TRANSITION_SERIAL="$TMP_DIR/transition.serial.log"
ACTIVATE_SERIAL="$TMP_DIR/activate.serial.log"
REPAIR_SERIAL="$TMP_DIR/repair.serial.log"
SEQUENCE1_SERIAL="$TMP_DIR/sequence1.serial.log"
NORMAL_SERIAL="$TMP_DIR/normal.serial.log"
CUT1_SERIAL="$TMP_DIR/cut1.serial.log"
RECOVER1_SERIAL="$TMP_DIR/recover1.serial.log"
CUT2_SERIAL="$TMP_DIR/cut2.serial.log"
RECOVER2_SERIAL="$TMP_DIR/recover2.serial.log"
CUT3_SERIAL="$TMP_DIR/cut3.serial.log"
RECOVER3_SERIAL="$TMP_DIR/recover3.serial.log"
RECOVER_CORRUPT_SERIAL="$TMP_DIR/recover-corrupt.serial.log"
SIGNATURE_SERIAL="$TMP_DIR/signature.serial.log"
BINDING_SERIAL="$TMP_DIR/binding.serial.log"
REPLAY_SERIAL="$TMP_DIR/completed-replay.serial.log"

SEED_DRIVER="$TMP_DIR/seed.driver.log"
TRANSITION_DRIVER="$TMP_DIR/transition.driver.log"
ACTIVATE_DRIVER="$TMP_DIR/activate.driver.log"
REPAIR_DRIVER="$TMP_DIR/repair.driver.log"
SEQUENCE1_DRIVER="$TMP_DIR/sequence1.driver.log"
NORMAL_DRIVER="$TMP_DIR/normal.driver.log"
CUT1_DRIVER="$TMP_DIR/cut1.driver.log"
RECOVER1_DRIVER="$TMP_DIR/recover1.driver.log"
CUT2_DRIVER="$TMP_DIR/cut2.driver.log"
RECOVER2_DRIVER="$TMP_DIR/recover2.driver.log"
CUT3_DRIVER="$TMP_DIR/cut3.driver.log"
RECOVER3_DRIVER="$TMP_DIR/recover3.driver.log"
RECOVER_CORRUPT_DRIVER="$TMP_DIR/recover-corrupt.driver.log"
SIGNATURE_DRIVER="$TMP_DIR/signature.driver.log"
BINDING_DRIVER="$TMP_DIR/binding.driver.log"
REPLAY_DRIVER="$TMP_DIR/completed-replay.driver.log"

RUN_SUCCEEDED=0
cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M79 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_step_interrupt_qemu.py" \
  "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" self-test

ARTIFACT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    artifacts \
    "$SEQUENCE1_ARTIFACT" "$SEQUENCE2_ARTIFACT" \
    "$BAD_SIGNATURE_ARTIFACT" "$WRONG_BINDING_ARTIFACT" \
    "$REQUEST1" "$SIGNATURE1" "$REQUEST2" "$SIGNATURE2"
)"
printf '%s\n' "$ARTIFACT_MARKER"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata

build_kernel() {
  local target_root="$1"
  local feature="$2"
  local manifest="$3"
  local authorization="${4:-}"

  if [[ -n "$authorization" ]]; then
    CARGO_TARGET_DIR="$target_root" \
      BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$manifest" \
      BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX="$authorization" \
      BNDROID_PROFILE="$PROFILE" \
      BNDROID_KERNEL_FEATURES="$feature" \
      BNDROID_USERSPACE_FEATURES="$feature" \
      "$SCRIPT_DIR/build-kernel.sh"
  else
    CARGO_TARGET_DIR="$target_root" \
      BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$manifest" \
      BNDROID_PROFILE="$PROFILE" \
      BNDROID_KERNEL_FEATURES="$feature" \
      BNDROID_USERSPACE_FEATURES="$feature" \
      "$SCRIPT_DIR/build-kernel.sh"
  fi
}

build_kernel "$SEED_TARGET_ROOT" "$M75_FEATURE" "$SEED_MANIFEST"
build_kernel "$TRANSITION_TARGET_ROOT" "$M76_FEATURE" "$TRANSITION_MANIFEST"
build_kernel "$ACTIVE_TARGET_ROOT" "$M76_FEATURE" "$ACTIVE_MANIFEST"
build_kernel "$SEQUENCE1_TARGET_ROOT" "$M78_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE1_ARTIFACT"
build_kernel "$SEQUENCE2_TARGET_ROOT" "$M79_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE2_ARTIFACT"
build_kernel "$SIGNATURE_TARGET_ROOT" "$M79_FEATURE" "$ACTIVE_MANIFEST" "$BAD_SIGNATURE_ARTIFACT"
build_kernel "$BINDING_TARGET_ROOT" "$M79_FEATURE" "$ACTIVE_MANIFEST" "$WRONG_BINDING_ARTIFACT"

SEED_KERNEL="$SEED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
TRANSITION_KERNEL="$TRANSITION_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
ACTIVE_KERNEL="$ACTIVE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SEQUENCE1_KERNEL="$SEQUENCE1_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SEQUENCE2_KERNEL="$SEQUENCE2_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SIGNATURE_KERNEL="$SIGNATURE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
BINDING_KERNEL="$BINDING_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
for image in \
  "$SEED_KERNEL" \
  "$TRANSITION_KERNEL" \
  "$ACTIVE_KERNEL" \
  "$SEQUENCE1_KERNEL" \
  "$SEQUENCE2_KERNEL" \
  "$SIGNATURE_KERNEL" \
  "$BINDING_KERNEL"; do
  if [[ ! -f "$image" ]]; then
    echo "M79 kernel image is missing: $image" >&2
    exit 1
  fi
done

run_positive() {
  local phase="$1"
  local milestone="$2"
  local kernel="$3"
  local disk="$4"
  local serial="$5"
  local driver="$6"
  local expected_driver
  local qmp="$TMP_DIR/$phase.qmp"
  local screenshot="$TMP_DIR/$phase.ppm"

  expected_driver="UNIFIED_PRODUCT_QMP_BOOT_OK phase=$phase qemu_self_exit=1 "
  expected_driver+="power_key=116 ui_sha256=$FINAL_SCREEN_SHA256 "
  expected_driver+="maintenance_rotations=2 cancel_pending=4"
  python3 "$SCRIPT_DIR/unified_product_qmp.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --screenshot "$screenshot" \
    --phase "$phase" \
    --milestone "$milestone" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M79 $phase boot driver evidence changed." >&2
    exit 1
  fi
  case "$milestone" in
    M75)
      python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
        parse first "$serial"
      ;;
    M76)
      python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" \
        parse "$phase" "$serial"
      ;;
    M78)
      python3 "$SCRIPT_DIR/unified_product_maintenance_execution_evidence.py" \
        parse sequence1 "$serial"
      ;;
    M79)
      python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
        parse "$phase" "$serial"
      ;;
    *)
      echo "unexpected M79 staged milestone: $milestone" >&2
      exit 1
      ;;
  esac
}

run_interrupt() {
  local step="$1"
  local disk="$2"
  local serial="$3"
  local driver="$4"
  local qmp="$TMP_DIR/cut$step.qmp"

  python3 "$SCRIPT_DIR/maintenance_step_interrupt_qemu.py" \
    --kernel "$SEQUENCE2_KERNEL" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --sequence 2 \
    --step "$step" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    parse-interrupt "$step" "$serial" "$driver"
}

run_negative() {
  local reason="$1"
  local kernel="$2"
  local disk="$3"
  local serial="$4"
  local driver="$5"
  local qmp="$TMP_DIR/negative-$reason.qmp"
  local signature_valid=1
  local binding_valid=0
  local audit_attempted=0
  local execution_evidence=""
  local expected_driver

  if [[ "$reason" == "signature" ]]; then
    signature_valid=0
  elif [[ "$reason" == "completed-replay" ]]; then
    binding_valid=1
    audit_attempted=1
    execution_evidence=" execution_slots_mutated=0"
  fi
  expected_driver="MAINTENANCE_AUTHORIZATION_NEGATIVE_BOOT_OK "
  expected_driver+="reason=$reason pre_el0=1 signature_valid=$signature_valid "
  expected_driver+="binding_valid=$binding_valid audit_attempted=$audit_attempted "
  expected_driver+="audit_slots_mutated=0$execution_evidence step_slots_mutated=0 "
  expected_driver+="manifest_published=0 init_ready=0 "
  expected_driver+="qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
  python3 "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --reason "$reason" \
    --step-runtime \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M79 $reason negative host evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    parse-negative "$reason" "$serial" "$driver"
}

run_positive first M75 "$SEED_KERNEL" "$RUNTIME_IMAGE" \
  "$SEED_SERIAL" "$SEED_DRIVER"
run_positive transition M76 "$TRANSITION_KERNEL" "$RUNTIME_IMAGE" \
  "$TRANSITION_SERIAL" "$TRANSITION_DRIVER"
run_positive activate M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE" \
  "$ACTIVATE_SERIAL" "$ACTIVATE_DRIVER"
run_positive repair M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE" \
  "$REPAIR_SERIAL" "$REPAIR_DRIVER"
run_positive sequence1 M78 "$SEQUENCE1_KERNEL" "$RUNTIME_IMAGE" \
  "$SEQUENCE1_SERIAL" "$SEQUENCE1_DRIVER"
cp "$RUNTIME_IMAGE" "$BASE_IMAGE"

cp "$BASE_IMAGE" "$NORMAL_IMAGE"
run_positive normal M79 "$SEQUENCE2_KERNEL" "$NORMAL_IMAGE" \
  "$NORMAL_SERIAL" "$NORMAL_DRIVER"

cp "$BASE_IMAGE" "$CUT1_IMAGE"
run_interrupt 1 "$CUT1_IMAGE" "$CUT1_SERIAL" "$CUT1_DRIVER"
cp "$CUT1_IMAGE" "$RECOVER1_IMAGE"
run_positive recover-step1 M79 "$SEQUENCE2_KERNEL" "$RECOVER1_IMAGE" \
  "$RECOVER1_SERIAL" "$RECOVER1_DRIVER"

cp "$BASE_IMAGE" "$CUT2_IMAGE"
run_interrupt 2 "$CUT2_IMAGE" "$CUT2_SERIAL" "$CUT2_DRIVER"
cp "$CUT2_IMAGE" "$RECOVER2_IMAGE"
run_positive recover-step2 M79 "$SEQUENCE2_KERNEL" "$RECOVER2_IMAGE" \
  "$RECOVER2_SERIAL" "$RECOVER2_DRIVER"

cp "$BASE_IMAGE" "$CUT3_IMAGE"
run_interrupt 3 "$CUT3_IMAGE" "$CUT3_SERIAL" "$CUT3_DRIVER"
cp "$CUT3_IMAGE" "$RECOVER3_IMAGE"
run_positive recover-step3 M79 "$SEQUENCE2_KERNEL" "$RECOVER3_IMAGE" \
  "$RECOVER3_SERIAL" "$RECOVER3_DRIVER"

cp "$CUT2_IMAGE" "$CORRUPT_IMAGE"
CORRUPTION_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    corrupt-newest-step "$CORRUPT_IMAGE"
)"
printf '%s\n' "$CORRUPTION_MARKER"
cp "$CORRUPT_IMAGE" "$RECOVER_CORRUPT_IMAGE"
run_positive recover-corrupt M79 "$SEQUENCE2_KERNEL" "$RECOVER_CORRUPT_IMAGE" \
  "$RECOVER_CORRUPT_SERIAL" "$RECOVER_CORRUPT_DRIVER"

cp "$BASE_IMAGE" "$SIGNATURE_IMAGE"
cp "$BASE_IMAGE" "$BINDING_IMAGE"
cp "$NORMAL_IMAGE" "$REPLAY_IMAGE"
run_negative signature "$SIGNATURE_KERNEL" "$SIGNATURE_IMAGE" \
  "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER"
run_negative binding "$BINDING_KERNEL" "$BINDING_IMAGE" \
  "$BINDING_SERIAL" "$BINDING_DRIVER"
run_negative completed-replay "$SEQUENCE2_KERNEL" "$REPLAY_IMAGE" \
  "$REPLAY_SERIAL" "$REPLAY_DRIVER"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" \
    reboot \
    "$BASE_IMAGE" "$NORMAL_IMAGE" \
    "$CUT1_IMAGE" "$RECOVER1_IMAGE" \
    "$CUT2_IMAGE" "$RECOVER2_IMAGE" \
    "$CUT3_IMAGE" "$RECOVER3_IMAGE" \
    "$CORRUPT_IMAGE" "$RECOVER_CORRUPT_IMAGE" \
    "$SIGNATURE_IMAGE" "$BINDING_IMAGE" "$REPLAY_IMAGE" \
    "$NORMAL_SERIAL" "$NORMAL_DRIVER" \
    "$CUT1_SERIAL" "$CUT1_DRIVER" \
    "$RECOVER1_SERIAL" "$RECOVER1_DRIVER" \
    "$CUT2_SERIAL" "$CUT2_DRIVER" \
    "$RECOVER2_SERIAL" "$RECOVER2_DRIVER" \
    "$CUT3_SERIAL" "$CUT3_DRIVER" \
    "$RECOVER3_SERIAL" "$RECOVER3_DRIVER" \
    "$RECOVER_CORRUPT_SERIAL" "$RECOVER_CORRUPT_DRIVER" \
    "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" \
    "$BINDING_SERIAL" "$BINDING_DRIVER" \
    "$REPLAY_SERIAL" "$REPLAY_DRIVER"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "$ARTIFACT_MARKER"
  for phase in seed-m75 transition-m76 activate-m76 repair-m76 sequence1-m78; do
    printf '%s\n' "M79_PREREQUISITE_BOOT_SEQUENCE phase=$phase qemu_psci_self_exit=1"
    case "$phase" in
      seed-m75)
        tr -d '\r' <"$SEED_SERIAL"
        sed '/^$/d' "$SEED_DRIVER"
        ;;
      transition-m76)
        tr -d '\r' <"$TRANSITION_SERIAL"
        sed '/^$/d' "$TRANSITION_DRIVER"
        ;;
      activate-m76)
        tr -d '\r' <"$ACTIVATE_SERIAL"
        sed '/^$/d' "$ACTIVATE_DRIVER"
        ;;
      repair-m76)
        tr -d '\r' <"$REPAIR_SERIAL"
        sed '/^$/d' "$REPAIR_DRIVER"
        ;;
      sequence1-m78)
        tr -d '\r' <"$SEQUENCE1_SERIAL"
        sed '/^$/d' "$SEQUENCE1_DRIVER"
        ;;
    esac
  done
  printf '%s\n' "M79_BOOT_SEQUENCE phase=normal qemu_psci_self_exit=1"
  tr -d '\r' <"$NORMAL_SERIAL"
  sed '/^$/d' "$NORMAL_DRIVER"
  for step in 1 2 3; do
    printf '%s\n' "M79_INTERRUPT_BOOT_SEQUENCE cut_after_step=$step host_terminated=1"
    case "$step" in
      1)
        tr -d '\r' <"$CUT1_SERIAL"
        sed '/^$/d' "$CUT1_DRIVER"
        printf '%s\n' "M79_BOOT_SEQUENCE phase=recover-step1 qemu_psci_self_exit=1"
        tr -d '\r' <"$RECOVER1_SERIAL"
        sed '/^$/d' "$RECOVER1_DRIVER"
        ;;
      2)
        tr -d '\r' <"$CUT2_SERIAL"
        sed '/^$/d' "$CUT2_DRIVER"
        printf '%s\n' "M79_BOOT_SEQUENCE phase=recover-step2 qemu_psci_self_exit=1"
        tr -d '\r' <"$RECOVER2_SERIAL"
        sed '/^$/d' "$RECOVER2_DRIVER"
        ;;
      3)
        tr -d '\r' <"$CUT3_SERIAL"
        sed '/^$/d' "$CUT3_DRIVER"
        printf '%s\n' "M79_BOOT_SEQUENCE phase=recover-step3 qemu_psci_self_exit=1"
        tr -d '\r' <"$RECOVER3_SERIAL"
        sed '/^$/d' "$RECOVER3_DRIVER"
        ;;
    esac
  done
  printf '%s\n' "$CORRUPTION_MARKER"
  printf '%s\n' "M79_BOOT_SEQUENCE phase=recover-corrupt qemu_psci_self_exit=1"
  tr -d '\r' <"$RECOVER_CORRUPT_SERIAL"
  sed '/^$/d' "$RECOVER_CORRUPT_DRIVER"
  for reason in signature binding completed-replay; do
    printf '%s\n' "M79_NEGATIVE_BOOT_SEQUENCE reason=$reason host_terminated=1"
    case "$reason" in
      signature)
        tr -d '\r' <"$SIGNATURE_SERIAL"
        sed '/^$/d' "$SIGNATURE_DRIVER"
        ;;
      binding)
        tr -d '\r' <"$BINDING_SERIAL"
        sed '/^$/d' "$BINDING_DRIVER"
        ;;
      completed-replay)
        tr -d '\r' <"$REPLAY_SERIAL"
        sed '/^$/d' "$REPLAY_DRIVER"
        ;;
    esac
  done
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-maintenance-step-runtime.log"
cp "$TMP_DIR/normal.ppm" "$OUTPUT_DIR/normal.ppm"
cp "$TMP_DIR/recover-step3.ppm" "$OUTPUT_DIR/recover-step3.ppm"
cp "$TMP_DIR/recover-corrupt.ppm" "$OUTPUT_DIR/recover-corrupt.ppm"

RUN_SUCCEEDED=1
echo "M79 durable maintenance-step journal and cut-point recovery passed."
