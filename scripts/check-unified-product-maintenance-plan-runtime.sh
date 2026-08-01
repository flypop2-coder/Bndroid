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
  python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" \
    parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed env; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M80 maintenance-plan runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M80 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
M75_FEATURE="unified-product-persistent-rollback-runtime"
M76_FEATURE="unified-product-key-rotation-runtime"
M78_FEATURE="unified-product-maintenance-execution-runtime"
M80_FEATURE="unified-product-maintenance-plan-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"

SEED_TARGET_ROOT="${BNDROID_M80_SEED_TARGET_DIR:-$WORKSPACE_ROOT/target/m80-key-rotation-seed-check}"
TRANSITION_TARGET_ROOT="${BNDROID_M80_TRANSITION_TARGET_DIR:-$WORKSPACE_ROOT/target/m80-key-rotation-transition-check}"
ACTIVE_TARGET_ROOT="${BNDROID_M80_ACTIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/m80-key-rotation-active-check}"
SEQUENCE1_TARGET_ROOT="${BNDROID_M80_SEQUENCE1_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-sequence1-check}"
SEQUENCE2_TARGET_ROOT="${BNDROID_M80_SEQUENCE2_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-sequence2-check}"
CUT_PREPARED_TARGET_ROOT="${BNDROID_M80_CUT_PREPARED_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-cut-prepared-check}"
CUT_APPLYING_TARGET_ROOT="${BNDROID_M80_CUT_APPLYING_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-cut-applying-check}"
CUT_EFFECT_TARGET_ROOT="${BNDROID_M80_CUT_EFFECT_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-cut-effect-check}"
CANCEL_TARGET_ROOT="${BNDROID_M80_CANCEL_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-plan-cancel-prepared-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m80"

SEED_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v3.bms1.hex"
TRANSITION_MANIFEST="$WORKSPACE_ROOT/boot/test-fixtures/product-service-manifest-v4-key3-transition.bms1.hex"
ACTIVE_MANIFEST="$WORKSPACE_ROOT/boot/product-service-manifest-v5-key4.bms1.hex"
SEQUENCE1_ARTIFACT="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence1.bma1.hex"
SEQUENCE2_ARTIFACT="$WORKSPACE_ROOT/boot/maintenance-authorization-sequence2.bma1.hex"

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
  "$CUT_PREPARED_TARGET_ROOT" \
  "$CUT_APPLYING_TARGET_ROOT" \
  "$CUT_EFFECT_TARGET_ROOT" \
  "$CANCEL_TARGET_ROOT" \
  "$OUTPUT_DIR"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m80.XXXXXX")"
RUN_SUCCEEDED=0
cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M80 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_plan_interrupt_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" self-test

RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
BASE_IMAGE="$TMP_DIR/m78-sequence1.raw"
NORMAL_IMAGE="$TMP_DIR/m80-normal.raw"
CUT_PREPARED_IMAGE="$TMP_DIR/m80-cut-prepared.raw"
RECOVER_PREPARED_IMAGE="$TMP_DIR/m80-recover-prepared.raw"
CUT_APPLYING_IMAGE="$TMP_DIR/m80-cut-applying.raw"
RECOVER_APPLYING_IMAGE="$TMP_DIR/m80-recover-applying.raw"
CUT_EFFECT_IMAGE="$TMP_DIR/m80-cut-effect.raw"
RECOVER_EFFECT_IMAGE="$TMP_DIR/m80-recover-effect.raw"
CORRUPT_IMAGE="$TMP_DIR/m80-corrupt-plan.raw"
RECOVER_CORRUPT_IMAGE="$TMP_DIR/m80-recover-corrupt.raw"
CANCEL_IMAGE="$TMP_DIR/m80-cancel-prepared.raw"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata

build_kernel() {
  local target_root="$1"
  local feature="$2"
  local manifest="$3"
  local authorization="${4:-}"
  local test_mode="${5:-}"
  local -a command=(env)
  if [[ -z "$test_mode" ]]; then
    command+=(-u BNDROID_M80_TEST_MODE)
  fi
  command+=(
    CARGO_TARGET_DIR="$target_root"
    BNDROID_VERIFIED_MANIFEST_ARTIFACT_HEX="$manifest"
    BNDROID_PROFILE="$PROFILE"
    BNDROID_KERNEL_FEATURES="$feature"
    BNDROID_USERSPACE_FEATURES="$feature"
  )
  if [[ -n "$authorization" ]]; then
    command+=(BNDROID_MAINTENANCE_AUTHORIZATION_ARTIFACT_HEX="$authorization")
  fi
  if [[ -n "$test_mode" ]]; then
    command+=(BNDROID_M80_TEST_MODE="$test_mode")
  fi
  command+=("$SCRIPT_DIR/build-kernel.sh")
  "${command[@]}"
}

build_kernel "$SEED_TARGET_ROOT" "$M75_FEATURE" "$SEED_MANIFEST"
build_kernel "$TRANSITION_TARGET_ROOT" "$M76_FEATURE" "$TRANSITION_MANIFEST"
build_kernel "$ACTIVE_TARGET_ROOT" "$M76_FEATURE" "$ACTIVE_MANIFEST"
build_kernel \
  "$SEQUENCE1_TARGET_ROOT" "$M78_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE1_ARTIFACT"
build_kernel \
  "$SEQUENCE2_TARGET_ROOT" "$M80_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE2_ARTIFACT"
build_kernel \
  "$CUT_PREPARED_TARGET_ROOT" "$M80_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_ARTIFACT" cut-prepared
build_kernel \
  "$CUT_APPLYING_TARGET_ROOT" "$M80_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_ARTIFACT" cut-applying
build_kernel \
  "$CUT_EFFECT_TARGET_ROOT" "$M80_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_ARTIFACT" cut-effect
build_kernel \
  "$CANCEL_TARGET_ROOT" "$M80_FEATURE" "$ACTIVE_MANIFEST" \
  "$SEQUENCE2_ARTIFACT" cancel-prepared

SEED_KERNEL="$SEED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
TRANSITION_KERNEL="$TRANSITION_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
ACTIVE_KERNEL="$ACTIVE_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SEQUENCE1_KERNEL="$SEQUENCE1_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
SEQUENCE2_KERNEL="$SEQUENCE2_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
CUT_PREPARED_KERNEL="$CUT_PREPARED_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
CUT_APPLYING_KERNEL="$CUT_APPLYING_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
CUT_EFFECT_KERNEL="$CUT_EFFECT_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
CANCEL_KERNEL="$CANCEL_TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-kernel.img"
for image in \
  "$SEED_KERNEL" \
  "$TRANSITION_KERNEL" \
  "$ACTIVE_KERNEL" \
  "$SEQUENCE1_KERNEL" \
  "$SEQUENCE2_KERNEL" \
  "$CUT_PREPARED_KERNEL" \
  "$CUT_APPLYING_KERNEL" \
  "$CUT_EFFECT_KERNEL" \
  "$CANCEL_KERNEL"; do
  if [[ ! -f "$image" ]]; then
    echo "M80 kernel image is missing: $image" >&2
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
  local qmp="$TMP_DIR/$phase.qmp"
  local screenshot="$TMP_DIR/$phase.ppm"
  local expected_driver
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
    echo "M80 $phase boot driver evidence changed." >&2
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
    M80)
      python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" \
        parse "$phase" "$serial"
      ;;
    *)
      echo "unexpected M80 staged milestone: $milestone" >&2
      exit 1
      ;;
  esac
}

run_interrupt() {
  local mode="$1"
  local kernel="$2"
  local disk="$3"
  local serial="$TMP_DIR/$mode.serial.log"
  local driver="$TMP_DIR/$mode.driver.log"
  python3 "$SCRIPT_DIR/maintenance_plan_interrupt_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$TMP_DIR/$mode.qmp" \
    --mode "$mode" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" \
    parse-interrupt "$mode" "$serial" "$driver"
}

run_positive first M75 "$SEED_KERNEL" "$RUNTIME_IMAGE"
run_positive transition M76 "$TRANSITION_KERNEL" "$RUNTIME_IMAGE"
run_positive activate M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE"
run_positive repair M76 "$ACTIVE_KERNEL" "$RUNTIME_IMAGE"
run_positive sequence1 M78 "$SEQUENCE1_KERNEL" "$RUNTIME_IMAGE"
cp "$RUNTIME_IMAGE" "$BASE_IMAGE"

cp "$BASE_IMAGE" "$NORMAL_IMAGE"
run_positive normal M80 "$SEQUENCE2_KERNEL" "$NORMAL_IMAGE"

cp "$BASE_IMAGE" "$CUT_PREPARED_IMAGE"
run_interrupt cut-prepared "$CUT_PREPARED_KERNEL" "$CUT_PREPARED_IMAGE"
cp "$CUT_PREPARED_IMAGE" "$RECOVER_PREPARED_IMAGE"
run_positive recover-prepared M80 "$SEQUENCE2_KERNEL" "$RECOVER_PREPARED_IMAGE"

cp "$BASE_IMAGE" "$CUT_APPLYING_IMAGE"
run_interrupt cut-applying "$CUT_APPLYING_KERNEL" "$CUT_APPLYING_IMAGE"
cp "$CUT_APPLYING_IMAGE" "$RECOVER_APPLYING_IMAGE"
run_positive recover-applying M80 "$SEQUENCE2_KERNEL" "$RECOVER_APPLYING_IMAGE"

cp "$BASE_IMAGE" "$CUT_EFFECT_IMAGE"
run_interrupt cut-effect "$CUT_EFFECT_KERNEL" "$CUT_EFFECT_IMAGE"
cp "$CUT_EFFECT_IMAGE" "$RECOVER_EFFECT_IMAGE"
run_positive recover-effect M80 "$SEQUENCE2_KERNEL" "$RECOVER_EFFECT_IMAGE"

cp "$CUT_APPLYING_IMAGE" "$CORRUPT_IMAGE"
CORRUPTION_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" \
    corrupt-newest-plan "$CORRUPT_IMAGE"
)"
printf '%s\n' "$CORRUPTION_MARKER"
cp "$CORRUPT_IMAGE" "$RECOVER_CORRUPT_IMAGE"
run_positive recover-plan-corrupt M80 "$SEQUENCE2_KERNEL" "$RECOVER_CORRUPT_IMAGE"

cp "$BASE_IMAGE" "$CANCEL_IMAGE"
run_interrupt cancel-prepared "$CANCEL_KERNEL" "$CANCEL_IMAGE"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" reboot \
    "$BASE_IMAGE" \
    "$NORMAL_IMAGE" \
    "$CUT_PREPARED_IMAGE" \
    "$RECOVER_PREPARED_IMAGE" \
    "$CUT_APPLYING_IMAGE" \
    "$RECOVER_APPLYING_IMAGE" \
    "$CUT_EFFECT_IMAGE" \
    "$RECOVER_EFFECT_IMAGE" \
    "$CORRUPT_IMAGE" \
    "$RECOVER_CORRUPT_IMAGE" \
    "$CANCEL_IMAGE"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' \
    "M80_PREREQUISITE_BOOT_SEQUENCE phases=first-m75/transition-m76/activate-m76/repair-m76/sequence1-m78 qemu_psci_self_exits=5"
  for phase in first transition activate repair sequence1; do
    tr -d '\r' <"$TMP_DIR/$phase.serial.log"
    sed '/^$/d' "$TMP_DIR/$phase.driver.log"
  done
  for phase in normal recover-prepared recover-applying recover-effect recover-plan-corrupt; do
    printf '%s\n' "M80_BOOT_SEQUENCE phase=$phase qemu_psci_self_exit=1"
    tr -d '\r' <"$TMP_DIR/$phase.serial.log"
    sed '/^$/d' "$TMP_DIR/$phase.driver.log"
  done
  for mode in cut-prepared cut-applying cut-effect cancel-prepared; do
    printf '%s\n' "M80_INTERRUPT_BOOT_SEQUENCE mode=$mode host_terminated=1"
    tr -d '\r' <"$TMP_DIR/$mode.serial.log"
    sed '/^$/d' "$TMP_DIR/$mode.driver.log"
  done
  printf '%s\n' "$CORRUPTION_MARKER"
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-maintenance-plan-runtime.log"
cp "$TMP_DIR/normal.ppm" "$OUTPUT_DIR/normal.ppm"
cp "$TMP_DIR/recover-effect.ppm" "$OUTPUT_DIR/recover-effect.ppm"
cp "$TMP_DIR/recover-plan-corrupt.ppm" "$OUTPUT_DIR/recover-plan-corrupt.ppm"

RUN_SUCCEEDED=1
echo "M80 durable maintenance-plan phase and reconciliation runtime passed."
