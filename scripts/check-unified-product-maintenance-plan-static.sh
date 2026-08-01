#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M80 maintenance-plan contracts." >&2
    exit 1
  fi
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import sys
import tomllib
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M80 contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(
    text: str,
    needle: str,
    label: str,
    *,
    count: int | None = None,
) -> None:
    actual = text.count(needle)
    if actual == 0:
        raise SystemExit(f"{label} is missing: {needle}")
    if count is not None and actual != count:
        raise SystemExit(
            f"{label} count changed for {needle}: {actual}, expected {count}"
        )


def features(relative: str) -> dict[str, list[str]]:
    return tomllib.loads(read(relative))["features"]


abi_features = features("crates/bndr-abi/Cargo.toml")
kernel_features = features("kernel/Cargo.toml")
user_features = features("user/init/Cargo.toml")
feature = "unified-product-maintenance-plan-runtime"
parent = "unified-product-maintenance-step-runtime"
if abi_features.get(feature) != [parent]:
    raise SystemExit("M80 ABI feature must extend exactly the M79 ABI")
expected_forwarding = [parent, f"bndr-abi/{feature}"]
if kernel_features.get(feature) != expected_forwarding:
    raise SystemExit("M80 kernel feature closure changed")
if user_features.get(feature) != expected_forwarding:
    raise SystemExit("M80 userspace feature closure changed")

abi = read("crates/bndr-abi/src/lib.rs")
for needle in (
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-plan-runtime",\n'
    '    not(feature = "unified-product-signed-maintenance-plan-runtime")\n'
    '))]\n'
    "pub const ABI_VERSION: u64 = 41;",
    'not(feature = "unified-product-maintenance-plan-runtime")',
    "MaintenanceSessionOpen = 57",
    "57 => Some(Self::MaintenanceSessionOpen)",
    "assert_eq!(ABI_VERSION, 41);",
    "assert_eq!(SyscallNumber::from_raw(58), None);",
):
    require(abi, needle, "M80 ABI 41 with stable syscall 0-57")
if "MaintenancePlan" in abi:
    raise SystemExit("M80 unexpectedly added a maintenance-plan syscall")

user_main = read("user/init/src/main.rs")
require(
    user_main,
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-plan-runtime",\n'
    '    not(feature = "unified-product-signed-maintenance-plan-runtime")\n'
    '))]\n'
    "const _: [(); 41] = [(); ABI_VERSION as usize];",
    "M80 userspace ABI pin",
    count=1,
)

build = read("scripts/build-kernel.sh")
for needle in (
    "KERNEL_UNIFIED_PRODUCT_MAINTENANCE_PLAN_RUNTIME=0",
    "USER_UNIFIED_PRODUCT_MAINTENANCE_PLAN_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" '
    '"unified-product-maintenance-plan-runtime"',
    'feature_list_contains "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-plan-runtime"',
    'append_feature "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-plan-runtime"',
    "userspace maintenance-plan profile requires the matching kernel profile.",
):
    require(build, needle, "M80 kernel/userspace feature forwarding")

build_rs = read("kernel/build.rs")
require(
    build_rs,
    'println!("cargo:rerun-if-env-changed=BNDROID_M80_TEST_MODE");',
    "M80 build-time deterministic test mode",
    count=1,
)

persist = read("kernel/src/persist.rs")
for needle in (
    'pub const MAINTENANCE_PLAN_STATE_MAGIC: [u8; 8] = *b"BNDRMPL1";',
    "pub const MAINTENANCE_PLAN_STATE_BYTES: usize = 424;",
    "pub const MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS: "
    "[u64; DATA_SLOT_COUNT] = [11, 12];",
    "pub const MAINTENANCE_PLAN_TRANSITIONS_PER_SEQUENCE: u64 = 9;",
    "pub const MAINTENANCE_PLAN_PHASE_PREPARED: u32 = 1;",
    "pub const MAINTENANCE_PLAN_PHASE_APPLYING: u32 = 2;",
    "pub const MAINTENANCE_PLAN_PHASE_CONFIRMED: u32 = 3;",
    "pub const MAINTENANCE_PLAN_PHASE_COMPENSATED: u32 = 4;",
    "pub struct MaintenancePlanTransitionBinding",
    "pub struct MaintenancePlanState",
    "pub struct MaintenancePlanAdmissionEvidence",
    "pub fn maintenance_plan_id_sha256",
    "pub fn maintenance_plan_operation_instance_id_sha256",
    "pub fn maintenance_plan_idempotency_key_sha256",
    "pub fn maintenance_plan_chain_sha256",
    "pub fn preflight_maintenance_plan_admission",
    "pub fn commit_maintenance_plan_transition",
    "pub fn validate_maintenance_plan_terminal",
    "MaintenancePlanError::EffectNotApplied",
    "MaintenancePlanError::EffectAlreadyApplied",
    "MaintenancePlanError::PlanCompensated",
    "maintenance_plan_state_round_trips_and_seals_identity_phase_and_chain",
    "maintenance_plan_reconciles_result_unknown_and_confirms_all_three_effects",
    "maintenance_plan_compensates_only_before_apply_and_then_fails_closed",
    "maintenance_plan_rejects_gaps_and_repairs_a_corrupt_newest_slot",
):
    require(persist, needle, "M80 four-ledger phase state machine")
if persist.count("MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS") < 7:
    raise SystemExit("M80 plan slots are not enforced at every transaction boundary")

storage = read("kernel/src/storage_persist.rs")
for needle in (
    "MaintenancePlanTransaction(MaintenancePlanError)",
    "pub fn preflight_verified_maintenance_plan",
    "pub fn commit_verified_maintenance_plan_transition",
    "pub fn validate_verified_maintenance_plan_terminal",
    "preflight_maintenance_plan_admission(&mut io, request)",
    "commit_maintenance_plan_transition(&mut io, request)",
    "validate_maintenance_plan_terminal(&mut io, request)",
):
    require(storage, needle, "M80 IRQ-backed kernel-only plan adapter")

syscall = read("kernel/src/syscall.rs")
for needle in (
    "struct MaintenancePlanAdmissionPreparation",
    "MAINTENANCE_PLAN_ADMISSION_PREPARATION.publish(plan_admission);",
    "preflight_verified_maintenance_plan(",
    "reason=plan-journal",
    "plan_mutations=0",
    "MAINTENANCE_PLAN_ADMISSION_OK",
    "preflight_before_audit_mutation=1",
    "result_unknown=",
    "effect_observed_unconfirmed=",
):
    require(syscall, needle, "M80 pre-EL0 four-ledger admission")
if not (
    syscall.index("verify_maintenance_authorization(")
    < syscall.index("preflight_verified_maintenance_plan(")
    < syscall.index("admit_verified_maintenance_authorization(")
    < syscall.index("MAINTENANCE_PLAN_ADMISSION_PREPARATION.publish(plan_admission)")
    < syscall.index("MAINTENANCE_AUTHORIZATION_PREPARATION.publish(evidence)")
):
    raise SystemExit("M80 signature, plan preflight, audit, and publication order changed")

main = read("kernel/src/main.rs")
for needle in (
    "fn commit_m80_maintenance_plan_phase(",
    "fn validate_m80_maintenance_plan_terminal(",
    "fn m80_pause_for_host(",
    "MAINTENANCE_PLAN_PHASE_OK",
    "MAINTENANCE_PLAN_TEST_PAUSE",
    "MAINTENANCE_PLAN_CANCEL_OK",
    "MAINTENANCE_PLAN_TERMINAL_OK",
    "runtime_material[192..224]",
    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK",
    "terminal_plan_bound_into_aggregate=1",
    "external_effect_exactly_once_claim=0",
    "arbitrary_resume_claim=0",
    "BOOT_OK: M80 unified real UI is interactive",
    "BOOT_OK: M80 durable maintenance plan phases",
):
    require(main, needle, "M80 resident fixed-plan boundary")
require(
    main,
    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK format=1",
    "single M80 final marker",
    count=1,
)
if "#[cfg(any())]" in main:
    raise SystemExit("M80 retained dead duplicate runtime code")
if not (
    main.index("let maintenance_step_terminal = {")
    < main.index("let maintenance_plan_terminal = {")
    < main.index("let maintenance_execution = {")
    < main.index("close_verified_device_health(", main.index("let maintenance_execution = {"))
):
    raise SystemExit("M80 step, plan, aggregate, and health-close order changed")
resident_start = main.index("fn validate_resident_platform_shutdown_runtime(")
resident_end = main.index(
    "fn validate_storage_server_shutdown_orchestration_runtime(", resident_start
)
resident_shutdown = main[resident_start:resident_end]
if not (
    resident_shutdown.index(
        "let shutdown = bndroid_kernel::shutdown::snapshot();"
    )
    < resident_shutdown.index("let syscalls = syscall::snapshot();")
    < resident_shutdown.index("let processes = process::snapshot();")
    < resident_shutdown.index(
        "if shutdown.phase == bndroid_kernel::shutdown::Phase::Requested"
    )
):
    raise SystemExit("M80 shutdown proof regained a cross-phase snapshot race")
if "requested_generation()" in resident_shutdown:
    raise SystemExit("M80 shutdown proof paired stale evidence with a fresh phase read")

qmp = read("scripts/unified_product_qmp.py")
for needle in (
    '"M80": (',
    '"recover-prepared",',
    '"recover-applying",',
    '"recover-effect",',
    '"recover-plan-corrupt",',
    '"MAINTENANCE_PLAN_ADMISSION_OK "',
    '"MAINTENANCE_PLAN_TERMINAL_OK "',
    '"UNIFIED_PRODUCT_MAINTENANCE_PLAN_OK "',
):
    require(qmp, needle, "M80 positive QMP driver")
if qmp.count('"qemu-system-aarch64"') != 1 or qmp.count('"-nic"') != 1:
    raise SystemExit("M80 positive QMP launch must contain exactly one -nic none")

interrupt = read("scripts/maintenance_plan_interrupt_qemu.py")
for needle in (
    "cut-prepared",
    "cut-applying",
    "cut-effect",
    "cancel-prepared",
    "MAINTENANCE_PLAN_TEST_PAUSE",
    "MAINTENANCE_PLAN_INTERRUPT_BOOT_OK",
    "hardware_powercut_claim=0",
    '"-nic",\n            "none",',
):
    require(interrupt, needle, "M80 deterministic host-interruption driver")
if interrupt.count('"qemu-system-aarch64"') != 1 or interrupt.count('"-nic"') != 1:
    raise SystemExit("M80 interruption launch must contain exactly one -nic none")

evidence = read("scripts/unified_product_maintenance_plan_evidence.py")
for needle in (
    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_PARSER_SELF_TEST_OK",
    "decode_plan_record",
    "expected_chains",
    "corrupt_newest_plan",
    "validate_interrupt",
    "runtime_digest",
    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_REBOOT_OK",
    "external_effect_exactly_once_claim=0",
    "hardware_powercut_claim=0",
):
    require(evidence, needle, "strict M80 log, record, and reboot parser")

runtime = read("scripts/check-unified-product-maintenance-plan-runtime.sh")
for needle in (
    "run_positive sequence1 M78",
    "run_positive normal M80",
    "run_interrupt cut-prepared",
    "run_interrupt cut-applying",
    "run_interrupt cut-effect",
    "run_interrupt cancel-prepared",
    "corrupt-newest-plan",
    "run_positive recover-plan-corrupt M80",
    "unified_product_maintenance_plan_evidence.py",
):
    require(runtime, needle, "five-positive/four-interrupted M80 runtime gate")
for forbidden in ("curl ", "wget ", "requests.", "urllib."):
    if forbidden in evidence or forbidden in runtime:
        raise SystemExit(f"M80 local-only gate gained network access: {forbidden}")

test_sh = read("scripts/test.sh")
for needle in (
    '"$SCRIPT_DIR/check-unified-product-maintenance-plan-static.sh"',
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndr-abi "
    "--features unified-product-maintenance-plan-runtime --lib",
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndroid-kernel "
    "--features unified-product-maintenance-plan-runtime --lib",
    'BNDROID_PROFILE=release "$SCRIPT_DIR/'
    'check-unified-product-maintenance-plan-runtime.sh"',
    "unified_product_maintenance_plan_static=1",
    "unified_product_maintenance_plan_reboot=1",
    "maintenance_plan_positive_boots=5",
    "maintenance_plan_interrupted_boots=4",
    "maintenance_plan_corruption_fallbacks=1",
    "maintenance_plan_result_unknown_reconciliations=1",
    "maintenance_plan_effect_observed_reconciliations=1",
    "maintenance_plan_preapply_compensations=1",
    "maintenance_plan_fixed_transitions=9",
):
    require(test_sh, needle, "full-suite M80 integration", count=1)

print(
    "UNIFIED_PRODUCT_MAINTENANCE_PLAN_SOURCE_OK "
    "abi=41 syscalls=0-57 audit_state=BNDRMAU1 execution_state=BNDRMEX1 "
    "step_state=BNDRMST1 plan_state=BNDRMPL1 plan_slots=11/12 "
    "program_operations=3 normal_transitions=9 phase_states=4 "
    "result_unknown_reconciliation=1 effect_observed_reconciliation=1 "
    "preapply_compensation=1 corruption_fallback=1 "
    "terminal_plan_bound_into_aggregate=1 network_access=0 "
    "external_effect_exactly_once_claim=0 arbitrary_resume_claim=0 "
    "trusted_monotonic_backend=0 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
    "efuse_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 "
    "emulator_only=1 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_plan_interrupt_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_plan_evidence.py" self-test
bash -n \
  "$SCRIPT_DIR/build-kernel.sh" \
  "$SCRIPT_DIR/check-unified-product-maintenance-plan-runtime.sh"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
ABI_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features unified-product-maintenance-plan-runtime --lib 2>&1
)"
printf '%s\n' "$ABI_LOG"
if ! grep -Eq \
  'test result: ok\. [0-9]+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' \
  <<<"$ABI_LOG"; then
  echo "M80 ABI host-test ledger changed." >&2
  exit 1
fi

M80_TEST_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features unified-product-maintenance-plan-runtime \
    persist::tests::maintenance_plan_ --lib 2>&1
)"
printf '%s\n' "$M80_TEST_LOG"
if ! grep -Eq \
  'test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$M80_TEST_LOG"; then
  echo "M80 maintenance-plan host-test ledger changed." >&2
  exit 1
fi

M79_TEST_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features unified-product-maintenance-step-runtime \
    persist::tests::maintenance_ --lib 2>&1
)"
printf '%s\n' "$M79_TEST_LOG"
if ! grep -Eq \
  'test result: ok\. 12 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$M79_TEST_LOG"; then
  echo "M79 historical maintenance host-test ledger changed under M80." >&2
  exit 1
fi

cargo check --locked -p bndroid-init \
  --features unified-product-maintenance-plan-runtime
cargo check --locked -p bndroid-kernel \
  --features unified-product-maintenance-plan-runtime \
  --bin bndroid-kernel

echo "M80 maintenance-plan static contract passed."
