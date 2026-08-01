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
  python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" self-test
  exit 0
fi
if [[ "${1:-}" == "--parse" && "$#" == 3 ]]; then
  python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
    parse "$2" "$3"
  exit 0
fi
if [[ "$#" -ne 0 ]]; then
  usage
  exit 2
fi

for tool in bash python3 openssl cargo rustc qemu-system-aarch64 mktemp mkdir cp rm tee grep tr sed cmp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M77 maintenance authorization runtime." >&2
    exit 1
  fi
done

if [[ "${BNDROID_PROFILE:-release}" != "release" ]]; then
  echo "M77 runtime evidence is release-only; BNDROID_PROFILE must be 'release'." >&2
  exit 2
fi

PROFILE="release"
M75_FEATURE="unified-product-persistent-rollback-runtime"
M76_FEATURE="unified-product-key-rotation-runtime"
M77_FEATURE="unified-product-maintenance-authorization-runtime"
BOOT_TIMEOUT_SECONDS="${BNDROID_QEMU_TIMEOUT_SECONDS:-300}"
QEMU_CPU="${BNDROID_QEMU_CPU:-cortex-a72}"
KEEP_TEMP="${BNDROID_KEEP_TEMP:-0}"
FINAL_SCREEN_SHA256="1d466ebb9c519c31f9c2f7a86c742efb36d2493e0283e2d532b10de549ed1994"
ROOT_SHA256="683eafa41277e75ae8cd1332490839a9cab204c8e8b808e2ebdf77f37259b005"

SEED_TARGET_ROOT="${BNDROID_MAINTENANCE_SEED_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-seed-check}"
TRANSITION_TARGET_ROOT="${BNDROID_MAINTENANCE_TRANSITION_TARGET_DIR:-$WORKSPACE_ROOT/target/key-rotation-transition-check}"
ACTIVE_TARGET_ROOT="${BNDROID_MAINTENANCE_ACTIVE_TARGET_DIR:-$WORKSPACE_ROOT/target/unified-product-key-rotation-runtime-check}"
SEQUENCE1_TARGET_ROOT="${BNDROID_MAINTENANCE_SEQUENCE1_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-authorization-sequence1-check}"
SEQUENCE2_TARGET_ROOT="${BNDROID_MAINTENANCE_SEQUENCE2_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-authorization-sequence2-check}"
SIGNATURE_TARGET_ROOT="${BNDROID_MAINTENANCE_BAD_SIGNATURE_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-authorization-bad-signature-check}"
BINDING_TARGET_ROOT="${BNDROID_MAINTENANCE_WRONG_BINDING_TARGET_DIR:-$WORKSPACE_ROOT/target/maintenance-authorization-wrong-binding-check}"
OUTPUT_DIR="$WORKSPACE_ROOT/target/m77"

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
PUBLIC_KEY="$WORKSPACE_ROOT/boot/trust/fixture-maintenance-key1-public.pem"

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
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-unified-product-m77.XXXXXX")"
RUNTIME_IMAGE="$TMP_DIR/runtime.raw"
PRISTINE_IMAGE="$TMP_DIR/pristine.raw"
BASE_IMAGE="$TMP_DIR/base-key4.raw"
SEQUENCE1_IMAGE="$TMP_DIR/sequence1.raw"
SEQUENCE2_IMAGE="$TMP_DIR/sequence2.raw"
SIGNATURE_IMAGE="$TMP_DIR/signature.raw"
BINDING_IMAGE="$TMP_DIR/binding.raw"
REPLAY_IMAGE="$TMP_DIR/replay.raw"

GENERATED_REQUEST1="$TMP_DIR/generated-sequence1.request.hex"
GENERATED_REQUEST2="$TMP_DIR/generated-sequence2.request.hex"
ASSEMBLED1="$TMP_DIR/assembled-sequence1.bma1.hex"
ASSEMBLED2="$TMP_DIR/assembled-sequence2.bma1.hex"

SEED_SERIAL="$TMP_DIR/seed.serial.log"
TRANSITION_SERIAL="$TMP_DIR/transition.serial.log"
ACTIVATE_SERIAL="$TMP_DIR/activate.serial.log"
REPAIR_SERIAL="$TMP_DIR/repair.serial.log"
SEQUENCE1_SERIAL="$TMP_DIR/sequence1.serial.log"
SEQUENCE2_SERIAL="$TMP_DIR/sequence2.serial.log"
SIGNATURE_SERIAL="$TMP_DIR/signature.serial.log"
BINDING_SERIAL="$TMP_DIR/binding.serial.log"
REPLAY_SERIAL="$TMP_DIR/replay.serial.log"

SEED_DRIVER="$TMP_DIR/seed.driver.log"
TRANSITION_DRIVER="$TMP_DIR/transition.driver.log"
ACTIVATE_DRIVER="$TMP_DIR/activate.driver.log"
REPAIR_DRIVER="$TMP_DIR/repair.driver.log"
SEQUENCE1_DRIVER="$TMP_DIR/sequence1.driver.log"
SEQUENCE2_DRIVER="$TMP_DIR/sequence2.driver.log"
SIGNATURE_DRIVER="$TMP_DIR/signature.driver.log"
BINDING_DRIVER="$TMP_DIR/binding.driver.log"
REPLAY_DRIVER="$TMP_DIR/replay.driver.log"

SEED_QMP="$TMP_DIR/seed.qmp"
TRANSITION_QMP="$TMP_DIR/transition.qmp"
ACTIVATE_QMP="$TMP_DIR/activate.qmp"
REPAIR_QMP="$TMP_DIR/repair.qmp"
SEQUENCE1_QMP="$TMP_DIR/sequence1.qmp"
SEQUENCE2_QMP="$TMP_DIR/sequence2.qmp"
SIGNATURE_QMP="$TMP_DIR/signature.qmp"
BINDING_QMP="$TMP_DIR/binding.qmp"
REPLAY_QMP="$TMP_DIR/replay.qmp"

SEED_SCREENSHOT="$TMP_DIR/seed.ppm"
TRANSITION_SCREENSHOT="$TMP_DIR/transition.ppm"
ACTIVATE_SCREENSHOT="$TMP_DIR/activate.ppm"
REPAIR_SCREENSHOT="$TMP_DIR/repair.ppm"
SEQUENCE1_SCREENSHOT="$TMP_DIR/sequence1.ppm"
SEQUENCE2_SCREENSHOT="$TMP_DIR/sequence2.ppm"
RUN_SUCCEEDED=0

cleanup() {
  local exit_status="$1"
  trap - EXIT INT TERM
  if [[ "$RUN_SUCCEEDED" == "1" && "$KEEP_TEMP" == "0" ]]; then
    rm -rf "$TMP_DIR"
  else
    echo "M77 diagnostic artifacts retained at $TMP_DIR" >&2
  fi
  exit "$exit_status"
}
trap 'cleanup "$?"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
  "$SCRIPT_DIR/offline_maintenance_authorization.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" self-test

ARTIFACT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
    artifacts \
    "$SEQUENCE1_ARTIFACT" "$SEQUENCE2_ARTIFACT" \
    "$BAD_SIGNATURE_ARTIFACT" "$WRONG_BINDING_ARTIFACT" \
    "$REQUEST1" "$SIGNATURE1" "$REQUEST2" "$SIGNATURE2"
)"
printf '%s\n' "$ARTIFACT_MARKER"

REQUEST_MARKER1="$(
  python3 "$SCRIPT_DIR/offline_maintenance_authorization.py" prepare \
    --sequence 1 \
    --request "$GENERATED_REQUEST1"
)"
printf '%s\n' "$REQUEST_MARKER1"
cmp "$REQUEST1" "$GENERATED_REQUEST1"
ASSEMBLY_MARKER1="$(
  python3 "$SCRIPT_DIR/offline_maintenance_authorization.py" assemble \
    --request "$REQUEST1" \
    --signature "$SIGNATURE1" \
    --public-key "$PUBLIC_KEY" \
    --expected-modulus-sha256 "$ROOT_SHA256" \
    --output "$ASSEMBLED1"
)"
printf '%s\n' "$ASSEMBLY_MARKER1"
cmp "$SEQUENCE1_ARTIFACT" "$ASSEMBLED1"

REQUEST_MARKER2="$(
  python3 "$SCRIPT_DIR/offline_maintenance_authorization.py" prepare \
    --sequence 2 \
    --request "$GENERATED_REQUEST2"
)"
printf '%s\n' "$REQUEST_MARKER2"
cmp "$REQUEST2" "$GENERATED_REQUEST2"
ASSEMBLY_MARKER2="$(
  python3 "$SCRIPT_DIR/offline_maintenance_authorization.py" assemble \
    --request "$REQUEST2" \
    --signature "$SIGNATURE2" \
    --public-key "$PUBLIC_KEY" \
    --expected-modulus-sha256 "$ROOT_SHA256" \
    --output "$ASSEMBLED2"
)"
printf '%s\n' "$ASSEMBLY_MARKER2"
cmp "$SEQUENCE2_ARTIFACT" "$ASSEMBLED2"

python3 "$SCRIPT_DIR/build_storage_image.py" build "$RUNTIME_IMAGE" --appdata
python3 "$SCRIPT_DIR/build_storage_image.py" verify "$RUNTIME_IMAGE" --appdata
cp "$RUNTIME_IMAGE" "$PRISTINE_IMAGE"

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
build_kernel "$SEQUENCE1_TARGET_ROOT" "$M77_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE1_ARTIFACT"
build_kernel "$SEQUENCE2_TARGET_ROOT" "$M77_FEATURE" "$ACTIVE_MANIFEST" "$SEQUENCE2_ARTIFACT"
build_kernel "$SIGNATURE_TARGET_ROOT" "$M77_FEATURE" "$ACTIVE_MANIFEST" "$BAD_SIGNATURE_ARTIFACT"
build_kernel "$BINDING_TARGET_ROOT" "$M77_FEATURE" "$ACTIVE_MANIFEST" "$WRONG_BINDING_ARTIFACT"

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
    echo "M77 kernel image is missing: $image" >&2
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
    echo "M77 $phase boot driver evidence changed." >&2
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
    M77)
      python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
        parse "$phase" "$serial"
      ;;
    *)
      echo "unexpected M77 staged milestone: $milestone" >&2
      exit 1
      ;;
  esac
}

run_negative() {
  local reason="$1"
  local kernel="$2"
  local disk="$3"
  local serial="$4"
  local driver="$5"
  local qmp="$6"
  local signature_valid
  local binding_valid
  local audit_attempted
  local expected_driver

  signature_valid=1
  binding_valid=0
  audit_attempted=0
  if [[ "$reason" == "signature" ]]; then
    signature_valid=0
  elif [[ "$reason" == "replay" ]]; then
    binding_valid=1
    audit_attempted=1
  fi
  expected_driver="MAINTENANCE_AUTHORIZATION_NEGATIVE_BOOT_OK "
  expected_driver+="reason=$reason pre_el0=1 signature_valid=$signature_valid "
  expected_driver+="binding_valid=$binding_valid audit_attempted=$audit_attempted "
  expected_driver+="audit_slots_mutated=0 manifest_published=0 init_ready=0 "
  expected_driver+="qemu_terminated_by_host=1 emulator_only=1 real_phone_claim=0"
  python3 "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
    --kernel "$kernel" \
    --disk "$disk" \
    --serial "$serial" \
    --qmp "$qmp" \
    --reason "$reason" \
    --cpu "$QEMU_CPU" \
    --timeout "$BOOT_TIMEOUT_SECONDS" \
    | tee "$driver"
  if [[ "$(grep -Fxc "$expected_driver" "$driver" || true)" != 1 ]]; then
    echo "M77 $reason negative host evidence changed." >&2
    exit 1
  fi
  python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
    parse-negative "$reason" "$serial" "$driver"
}

run_positive first M75 "$SEED_KERNEL" \
  "$SEED_SERIAL" "$SEED_DRIVER" "$SEED_QMP" "$SEED_SCREENSHOT"
run_positive transition M76 "$TRANSITION_KERNEL" \
  "$TRANSITION_SERIAL" "$TRANSITION_DRIVER" "$TRANSITION_QMP" "$TRANSITION_SCREENSHOT"
run_positive activate M76 "$ACTIVE_KERNEL" \
  "$ACTIVATE_SERIAL" "$ACTIVATE_DRIVER" "$ACTIVATE_QMP" "$ACTIVATE_SCREENSHOT"
run_positive repair M76 "$ACTIVE_KERNEL" \
  "$REPAIR_SERIAL" "$REPAIR_DRIVER" "$REPAIR_QMP" "$REPAIR_SCREENSHOT"
cp "$RUNTIME_IMAGE" "$BASE_IMAGE"

run_positive sequence1 M77 "$SEQUENCE1_KERNEL" \
  "$SEQUENCE1_SERIAL" "$SEQUENCE1_DRIVER" "$SEQUENCE1_QMP" "$SEQUENCE1_SCREENSHOT"
cp "$RUNTIME_IMAGE" "$SEQUENCE1_IMAGE"
run_positive sequence2 M77 "$SEQUENCE2_KERNEL" \
  "$SEQUENCE2_SERIAL" "$SEQUENCE2_DRIVER" "$SEQUENCE2_QMP" "$SEQUENCE2_SCREENSHOT"
cp "$RUNTIME_IMAGE" "$SEQUENCE2_IMAGE"

cp "$RUNTIME_IMAGE" "$SIGNATURE_IMAGE"
cp "$RUNTIME_IMAGE" "$BINDING_IMAGE"
cp "$RUNTIME_IMAGE" "$REPLAY_IMAGE"
run_negative signature "$SIGNATURE_KERNEL" "$SIGNATURE_IMAGE" \
  "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" "$SIGNATURE_QMP"
run_negative binding "$BINDING_KERNEL" "$BINDING_IMAGE" \
  "$BINDING_SERIAL" "$BINDING_DRIVER" "$BINDING_QMP"
run_negative replay "$SEQUENCE2_KERNEL" "$REPLAY_IMAGE" \
  "$REPLAY_SERIAL" "$REPLAY_DRIVER" "$REPLAY_QMP"

REBOOT_MARKER="$(
  python3 "$SCRIPT_DIR/unified_product_maintenance_authorization_evidence.py" \
    reboot \
    "$BASE_IMAGE" "$SEQUENCE1_IMAGE" "$SEQUENCE2_IMAGE" \
    "$SIGNATURE_IMAGE" "$BINDING_IMAGE" "$REPLAY_IMAGE" \
    "$SEQUENCE1_SERIAL" "$SEQUENCE1_DRIVER" \
    "$SEQUENCE2_SERIAL" "$SEQUENCE2_DRIVER" \
    "$SIGNATURE_SERIAL" "$SIGNATURE_DRIVER" \
    "$BINDING_SERIAL" "$BINDING_DRIVER" \
    "$REPLAY_SERIAL" "$REPLAY_DRIVER"
)"
printf '%s\n' "$REBOOT_MARKER"

{
  printf '%s\n' "$ARTIFACT_MARKER"
  printf '%s\n' "$REQUEST_MARKER1"
  printf '%s\n' "$ASSEMBLY_MARKER1"
  printf '%s\n' "$REQUEST_MARKER2"
  printf '%s\n' "$ASSEMBLY_MARKER2"
  for phase in seed-m75 transition-m76 activate-m76 repair-m76; do
    printf '%s\n' "M77_PREREQUISITE_BOOT_SEQUENCE phase=$phase qemu_psci_self_exit=1"
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
    esac
  done
  printf '%s\n' "M77_BOOT_SEQUENCE phase=sequence1 qemu_psci_self_exit=1"
  tr -d '\r' <"$SEQUENCE1_SERIAL"
  sed '/^$/d' "$SEQUENCE1_DRIVER"
  printf '%s\n' "M77_BOOT_SEQUENCE phase=sequence2 qemu_psci_self_exit=1"
  tr -d '\r' <"$SEQUENCE2_SERIAL"
  sed '/^$/d' "$SEQUENCE2_DRIVER"
  for reason in signature binding replay; do
    printf '%s\n' "M77_NEGATIVE_BOOT_SEQUENCE reason=$reason host_terminated=1"
    case "$reason" in
      signature)
        tr -d '\r' <"$SIGNATURE_SERIAL"
        sed '/^$/d' "$SIGNATURE_DRIVER"
        ;;
      binding)
        tr -d '\r' <"$BINDING_SERIAL"
        sed '/^$/d' "$BINDING_DRIVER"
        ;;
      replay)
        tr -d '\r' <"$REPLAY_SERIAL"
        sed '/^$/d' "$REPLAY_DRIVER"
        ;;
    esac
  done
  printf '%s\n' "$REBOOT_MARKER"
} >"$OUTPUT_DIR/unified-product-maintenance-authorization-runtime.log"
cp "$SEQUENCE1_SCREENSHOT" "$OUTPUT_DIR/sequence1.ppm"
cp "$SEQUENCE2_SCREENSHOT" "$OUTPUT_DIR/sequence2.ppm"

RUN_SUCCEEDED=1
echo "M77 signed maintenance authorization and durable anti-replay gate passed."
