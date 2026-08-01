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
  python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
    parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed env; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M81 signed maintenance-plan runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M81 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
M75_FEATURE="unified-product-persistent-rollback-runtime"
M76_FEATURE="unified-product-key-rotation-runtime"
M78_FEATURE="unified-product-maintenance-execution-runtime"
M81_FEATURE="unified-product-signed-maintenance-plan-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"

SEED_TARGET_ROOT="${BNDROID_M81_SEED_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-key-rotation-seed-check}"
TRANSITION_TARGET_ROOT="${BNDROID_M81_TRANSITION_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-key-rotation-transition-check}"
ACTIVE_TARGET_ROOT="${BNDROID_M81_ACTIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-key-rotation-active-check}"
SEQUENCE1_TARGET_ROOT="${BNDROID_M81_SEQUENCE1_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-maintenance-sequence1-check}"
NORMAL_TARGET_ROOT="${BNDROID_M81_NORMAL_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-signed-plan-normal-check}"
CUT_TARGET_ROOT="${BNDROID_M81_CUT_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-signed-plan-cut-check}"
NEGATIVE_TARGET_ROOT="${BNDROID_M81_NEGATIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/m81-signed-plan-negative-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m81"

SEED_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex"
TRANSITION_MANIFEST="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex"
ACTIVE_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex"
SEQUENCE1_AUTHORIZATION="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence1.bma1.hex"
SEQUENCE2_AUTHORIZATION="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex"
NORMAL_PLAN="$WORKSPACE_ROOT/boot/maintenance-plan-sequence2.bmp1.hex"
BAD_SIGNATURE_PLAN="$WORKSPACE_ROOT/boot/test-fixtures/maintenance-plan-sequence2-bad-signature.bmp1.hex"
WRONG_BINDING_PLAN="$WORKSPACE_ROOT/boot/test-fixtures/maintenance-plan-sequence2-wrong-binding.bmp1.hex"
INVALID_PROGRAM_PLAN="$WORKSPACE_ROOT/boot/test-fixtures/maintenance-plan-sequence2-invalid-program.bmp1.hex"

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
  "$NORMAL_TARGET_ROOT" \
  "$CUT_TARGET_ROOT" \
  "$NEGATIVE_TARGET_ROOT" \
  "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m81.XXXXXX")"
RUN_SUCCEEDED=0
cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M81 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/signed_maintenance_plan_interrupt_qemu.py" \
  "$SCRIPT_DIR/signed_maintenance_plan_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py"
python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" self-test

RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
BASE_IMAGE="$TMP_DIR/m78-sequence1.raw"
NORMAL_IMAGE="$TMP_DIR/m81-normal.raw"
CUT_BINDING_IMAGE="$TMP_DIR/m81-cut-binding.raw"
RECOVER_BINDING_IMAGE="$TMP_DIR/m81-recover-binding.raw"
BAD_SIGNATURE_IMAGE="$TMP_DIR/m81-bad-signature.raw"
WRONG_BINDING_IMAGE="$TMP_DIR/m81-wrong-binding.raw"
INVALID_PROGRAM_IMAGE="$TMP_DIR/m81-invalid-program.raw"
CORRUPT_PROGRAM_IMAGE="$TMP_DIR/m81-corrupt-program.raw"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata

build_kernel() {
  local target_root="$1"
  local feature="$2"
  local manifest="$3"
  local authorization="${4:-}"
  local plan="${5:-}"
  local test_mode="${6:-}"
  local -a command=(
    env
    -u BNDROID_M80_TEST_MODE
    -u BNDROID_M81_TEST_MODE
    -u BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX
    -u BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX
    CARGO_TARGET_DIR="$target_root"
    BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$manifest"
    BNDROID_PROFILE="$PROFILE"
    BNDROID_KERNEL_FEATURES="$feature"
    BNDROID_USERSPACE_FEATURES="$feature"
  )
  if [[ -n "$authorization" ]]; then
    command+=(BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX="$authorization")
  fi
  if [[ -n "$plan" ]]; then
    command+=(BNDROID_MAINTENANCE_PLAN_ARTIFACT_HEX="$plan")
  fi
  if [[ -n "$test_mode" ]]; then
    command+=(BNDROID_M81_TEST_MODE="$test_mode")
  fi
  command+=("$SCRIPT_DIR/build-kernel.sh")
  "${command[@]}"
}

copy_built_kernel() {
  local target_root="$1"
  local destination="$2"
  local source="$target_root/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
  if [[ ! -f "$source" ]]; then
    echo "M81 kernel image is missing: $source" >&2
    exit 1
  fi
  cp "$source" "$destination"
}

build_kernel "$SEED_TARGET_ROOT" "$M75_FEATURE" "$SEED_MANIFEST"
build_kernel "$TRANSITION_TARGET_ROOT" "$M76_FEATURE" "$TRANSITION_MANIFEST"
build_kernel "$ACTIVE_TARGET_ROOT" "$M76_FEATURE" "$ACTIVE_MANIFEST"
build_kernel \
  "$SEQUENCE1_TARGET_ROOT" "$M78_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE1_AUTHORIZATION"
build_kernel \
  "$NORMAL_TARGET_ROOT" "$M81_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_AUTHORIZATION" "$NORMAL_PLAN"
copy_built_kernel "$NORMAL_TARGET_ROOT" "$TMP_DIR/m81-normal-kernel.img"
build_kernel \
  "$CUT_TARGET_ROOT" "$M81_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_AUTHORIZATION" "$NORMAL_PLAN" cut-binding
copy_built_kernel "$CUT_TARGET_ROOT" "$TMP_DIR/m81-cut-kernel.img"
build_kernel \
  "$NEGATIVE_TARGET_ROOT" "$M81_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_AUTHORIZATION" "$BAD_SIGNATURE_PLAN"
copy_built_kernel "$NEGATIVE_TARGET_ROOT" "$TMP_DIR/m81-bad-signature-kernel.img"
build_kernel \
  "$NEGATIVE_TARGET_ROOT" "$M81_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_AUTHORIZATION" "$WRONG_BINDING_PLAN"
copy_built_kernel "$NEGATIVE_TARGET_ROOT" "$TMP_DIR/m81-wrong-binding-kernel.img"
build_kernel \
  "$NEGATIVE_TARGET_ROOT" "$M81_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_AUTHORIZATION" "$INVALID_PROGRAM_PLAN"
copy_built_kernel "$NEGATIVE_TARGET_ROOT" "$TMP_DIR/m81-invalid-program-kernel.img"

SEED_KERNEL="$SEED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
TRANSITION_KERNEL="$TRANSITION_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
ACTIVE_KERNEL="$ACTIVE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SEQUENCE1_KERNEL="$SEQUENCE1_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
NORMAL_KERNEL="$TMP_DIR/m81-normal-kernel.img"
CUT_KERNEL="$TMP_DIR/m81-cut-kernel.img"
BAD_SIGNATURE_KERNEL="$TMP_DIR/m81-bad-signature-kernel.img"
WRONG_BINDING_KERNEL="$TMP_DIR/m81-wrong-binding-kernel.img"
INVALID_PROGRAM_KERNEL="$TMP_DIR/m81-invalid-program-kernel.img"
for image in \
  "$SEED_KERNEL" \
  "$TRANSITION_KERNEL" \
  "$ACTIVE_KERNEL" \
  "$SEQUENCE1_KERNEL" \
  "$NORMAL_KERNEL" \
  "$CUT_KERNEL" \
  "$BAD_SIGNATURE_KERNEL" \
  "$WRONG_BINDING_KERNEL" \
  "$INVALID_PROGRAM_KERNEL"; do
  if [[ ! -f "$image" ]]; then
    echo "M81 kernel image is missing: $image" >&2
    exit 1
  fi
done

run_positive() {
  local phase="$1"
  local milestone="$2"
  local kernel="$3"
  local disk="$4"
  local serial="$TMP_DIR/$phase.serial.log"
  local driver="$TMP_DIR/$phase.driver.log"
  local evidence="$TMP_DIR/$phase.evidence.log"
  local expected_driver
  expected_driver="UNIFIED_PRODUCT_QMP_BOOT_OK phase=$phase qemu_self_exit=1 "
  expected_driver+="power_key=116 ui_sha256=$FINAL_SCREEN_SHA256 "
  expected_driver+="maintenance_rotations=2 cancel_pending=4"
  python3 "$SCRIPT_DIR/unified_product_qmp.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$TMP_DIR/$phase.qmp" \
    --screenshot "$TMP_DIR/$phase.ppm" \
    --phase "$phase" \
    --milestone "$milestone" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M81 $phase boot driver evidence changed." >&2
    exit 1
  fi
  case "$milestone" in
    M75)
      python3 "$SCRIPT_DIR/unified_product_persistent_rollback_evidence.py" \
        parse first "$serial" | tee "$evidence"
      ;;
    M76)
      python3 "$SCRIPT_DIR/unified_product_key_rotation_evidence.py" \
        parse "$phase" "$serial" | tee "$evidence"
      ;;
    M78)
      python3 "$SCRIPT_DIR/unified_product_maintenance_execution_evidence.py" \
        parse sequence1 "$serial" | tee "$evidence"
      ;;
    M81)
      python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
        parse "$phase" "$serial" | tee "$evidence"
      ;;
    *)
      echo "unexpected M81 staged milestone: $milestone" >&2
      exit 1
      ;;
  esac
}

run_interrupt() {
  local kernel="$1"
  local disk="$2"
  python3 "$SCRIPT_DIR/signed_maintenance_plan_interrupt_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$TMP_DIR/cut-binding.serial.log" \
    --qmp "$TMP_DIR/cut-binding.qmp" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$TMP_DIR/cut-binding.driver.log"
  python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
    inspect-disk "$disk" | tee "$TMP_DIR/cut-binding.evidence.log"
}

run_negative() {
  local reason="$1"
  local kernel="$2"
  local disk="$3"
  local before
  local after
  before="$(
    python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
      program-digest "$disk"
  )"
  python3 "$SCRIPT_DIR/signed_maintenance_plan_negative_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$TMP_DIR/$reason.serial.log" \
    --qmp "$TMP_DIR/$reason.qmp" \
    --reason "$reason" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$TMP_DIR/$reason.driver.log"
  after="$(
    python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
      program-digest "$disk"
  )"
  if [[ "$before" != "$after" ]]; then
    echo "M81 $reason rejection mutated the program slots." >&2
    exit 1
  fi
  printf '%s\n' \
    "SIGNED_MAINTENANCE_PLAN_NEGATIVE_DISK_OK reason=$reason program_slots_mutated=0 sha256=$after" \
    | tee "$TMP_DIR/$reason.evidence.log"
}

run_positive first M75 "$SEED_KERNEL" "$RUNTIME_IMAGE"
run_positive transition M76 "$TRANSITION_KERNEL" "$RUNTIME_IMAGE"
run_positive activate M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE"
run_positive repair M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE"
run_positive sequence1 M78 "$SEQUENCE1_KERNEL" "$RUNTIME_IMAGE"
cp "$RUNTIME_IMAGE" "$BASE_IMAGE"

for image in \
  "$NORMAL_IMAGE" \
  "$CUT_BINDING_IMAGE" \
  "$BAD_SIGNATURE_IMAGE" \
  "$WRONG_BINDING_IMAGE" \
  "$INVALID_PROGRAM_IMAGE"; do
  cp "$BASE_IMAGE" "$image"
  python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
    assert-empty "$image"
done

run_positive normal M81 "$NORMAL_KERNEL" "$NORMAL_IMAGE"
python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
  inspect-disk "$NORMAL_IMAGE" | tee "$TMP_DIR/normal.disk.log"

run_interrupt "$CUT_KERNEL" "$CUT_BINDING_IMAGE"
cp "$CUT_BINDING_IMAGE" "$RECOVER_BINDING_IMAGE"
run_positive recover-binding M81 "$NORMAL_KERNEL" "$RECOVER_BINDING_IMAGE"
python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
  inspect-disk "$RECOVER_BINDING_IMAGE" | tee "$TMP_DIR/recover-binding.disk.log"

run_negative signature "$BAD_SIGNATURE_KERNEL" "$BAD_SIGNATURE_IMAGE"
run_negative binding "$WRONG_BINDING_KERNEL" "$WRONG_BINDING_IMAGE"
run_negative program "$INVALID_PROGRAM_KERNEL" "$INVALID_PROGRAM_IMAGE"

cp "$CUT_BINDING_IMAGE" "$CORRUPT_PROGRAM_IMAGE"
python3 "$SCRIPT_DIR/unified_product_signed_maintenance_plan_evidence.py" \
  corrupt-selected "$CORRUPT_PROGRAM_IMAGE" \
  | tee "$TMP_DIR/program-ledger-corruption.log"
run_negative program-ledger "$NORMAL_KERNEL" "$CORRUPT_PROGRAM_IMAGE"

{
  printf '%s\n' \
    "M81_PREREQUISITE_BOOT_SEQUENCE phases=first-m75/transition-m76/activate-m76/repair-m76/sequence1-m78 qemu_psci_self_exits=5"
  for phase in first transition activate repair sequence1; do
    tr -d '\r' <"$TMP_DIR/$phase.serial.log"
    sed '/^$/d' "$TMP_DIR/$phase.driver.log"
    sed '/^$/d' "$TMP_DIR/$phase.evidence.log"
  done
  for phase in normal recover-binding; do
    printf '%s\n' "M81_BOOT_SEQUENCE phase=$phase qemu_psci_self_exit=1"
    tr -d '\r' <"$TMP_DIR/$phase.serial.log"
    sed '/^$/d' "$TMP_DIR/$phase.driver.log"
    sed '/^$/d' "$TMP_DIR/$phase.evidence.log"
    sed '/^$/d' "$TMP_DIR/$phase.disk.log"
  done
  printf '%s\n' \
    "M81_INTERRUPT_BOOT_SEQUENCE mode=cut-binding host_terminated=1"
  tr -d '\r' <"$TMP_DIR/cut-binding.serial.log"
  sed '/^$/d' "$TMP_DIR/cut-binding.driver.log"
  sed '/^$/d' "$TMP_DIR/cut-binding.evidence.log"
  for reason in signature binding program program-ledger; do
    printf '%s\n' \
      "M81_NEGATIVE_BOOT_SEQUENCE reason=$reason host_terminated=1"
    tr -d '\r' <"$TMP_DIR/$reason.serial.log"
    sed '/^$/d' "$TMP_DIR/$reason.driver.log"
    sed '/^$/d' "$TMP_DIR/$reason.evidence.log"
  done
  sed '/^$/d' "$TMP_DIR/program-ledger-corruption.log"
  printf '%s\n' \
    "UNIFIED_PRODUCT_SIGNED_MAINTENANCE_PLAN_REBOOT_OK positive_boots=2 interrupted_boots=1 negative_boots=4 signed_program_writes=2 signed_program_replays=1 signature_rejections=1 binding_rejections=1 program_rejections=1 program_ledger_rejections=1 descriptor_phases=18 qemu_psci_self_exits=7 qemu_host_terminations=5 hardware_powercut_claim=0 emulator_only=1 real_phone_claim=0"
} >"$OUTPUT_DIR/unified-product-signed-maintenance-plan-runtime.log"
cp "$TMP_DIR/normal.ppm" "$OUTPUT_DIR/normal.ppm"
cp "$TMP_DIR/recover-binding.ppm" "$OUTPUT_DIR/recover-binding.ppm"

RUN_SUCCEEDED=1
echo "M81 signed descriptor-bound maintenance-plan runtime passed."
