#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M79 maintenance-step contracts." >&2
    exit 1
  fi
done

python3 - "$WORKSPACE_ROOT" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path


root = Path(sys.argv[1])


def read(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M79 contract source is missing: {relative}")
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


abi_toml = read("crates/bndr-abi/Cargo.toml")
kernel_toml = read("kernel/Cargo.toml")
user_toml = read("user/init/Cargo.toml")
require(
    abi_toml,
    "unified-product-maintenance-step-runtime = [\n"
    '    "unified-product-maintenance-execution-runtime",\n]',
    "ABI exact M79 feature closure",
    count=1,
)
for relative, text in (
    ("kernel/Cargo.toml", kernel_toml),
    ("user/init/Cargo.toml", user_toml),
):
    require(
        text,
        "unified-product-maintenance-step-runtime = [",
        f"{relative} M79 feature declaration",
        count=1,
    )
    require(
        text,
        '"unified-product-maintenance-execution-runtime",',
        f"{relative} M79 inherited M78 feature",
    )
    require(
        text,
        '"bndr-abi/unified-product-maintenance-step-runtime",',
        f"{relative} M79 ABI feature forwarding",
        count=1,
    )

abi = read("crates/bndr-abi/src/lib.rs")
for needle in (
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-step-runtime",\n'
    '    not(feature = "unified-product-maintenance-plan-runtime")\n'
    "))]\n"
    "pub const ABI_VERSION: u64 = 40;",
    "MaintenanceSessionOpen = 57",
    "57 => Some(Self::MaintenanceSessionOpen)",
    "assert_eq!(ABI_VERSION, 40);",
    "assert_eq!(SyscallNumber::from_raw(58), None);",
):
    require(abi, needle, "M79 ABI 40 with stable syscall 0-57")
if "MaintenanceStep" in abi:
    raise SystemExit("M79 unexpectedly added a maintenance-step syscall")

user_main = read("user/init/src/main.rs")
require(
    user_main,
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-step-runtime",\n'
    '    not(feature = "unified-product-maintenance-plan-runtime")\n'
    "))]\n"
    "const _: [(); 40] = [(); ABI_VERSION as usize];",
    "M79 userspace ABI pin",
    count=1,
)

build_kernel = read("scripts/build-kernel.sh")
for needle in (
    "KERNEL_UNIFIED_PRODUCT_MAINTENANCE_STEP_RUNTIME=0",
    "USER_UNIFIED_PRODUCT_MAINTENANCE_STEP_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" '
    '"unified-product-maintenance-step-runtime"',
    'feature_list_contains "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-step-runtime"',
    'append_feature "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-step-runtime"',
    "userspace maintenance-step profile requires the matching kernel profile.",
):
    require(build_kernel, needle, "M79 kernel/userspace feature forwarding")

persist = read("kernel/src/persist.rs")
for needle in (
    'pub const MAINTENANCE_STEP_STATE_MAGIC: [u8; 8] = *b"BNDRMST1";',
    "pub const MAINTENANCE_STEP_STATE_BYTES: usize = 376;",
    "pub const MAINTENANCE_STEP_SLOT_RELATIVE_LBAS: "
    "[u64; DATA_SLOT_COUNT] = [9, 10];",
    "pub const MAINTENANCE_STEP_COUNT: u32 = 3;",
    "pub struct MaintenanceStepCommitBinding",
    "pub struct MaintenanceStepState",
    "pub fn maintenance_step_effect_sha256",
    "pub fn maintenance_step_chain_sha256",
    "pub enum MaintenanceStepError",
    "pub fn preflight_maintenance_step_admission",
    "pub fn commit_maintenance_step",
    "pub fn validate_maintenance_step_terminal",
    "MaintenanceStepError::StepGap",
    "MaintenanceStepError::ExecutionAlreadyComplete",
    "MaintenanceStepError::TerminalStepIncomplete",
    "explicit idempotent reconciliation rule",
    "not arbitrary instruction resume",
    "maintenance_step_state_round_trips_and_seals_every_program_binding",
    "maintenance_steps_migrate_from_m78_replay_and_advance_only_in_order",
    "maintenance_steps_reject_gaps_substitution_and_early_completion_without_mutation",
    "maintenance_step_torn_write_falls_back_and_reconciles_from_the_selected_head",
):
    require(persist, needle, "M79 three-ledger persistence state machine")
if persist.count("MAINTENANCE_STEP_SLOT_RELATIVE_LBAS") < 9:
    raise SystemExit("M79 step slots are not enforced at every transaction boundary")

storage_persist = read("kernel/src/storage_persist.rs")
for needle in (
    "MaintenanceStepTransaction(MaintenanceStepError)",
    "pub fn preflight_verified_maintenance_steps",
    "pub fn commit_verified_maintenance_step",
    "pub fn validate_verified_maintenance_step_terminal",
    "preflight_maintenance_step_admission(&mut io, request)",
    "commit_maintenance_step(&mut io, request)",
    "validate_maintenance_step_terminal(&mut io, request)",
):
    require(storage_persist, needle, "M79 IRQ-backed kernel-only step adapter")

syscall = read("kernel/src/syscall.rs")
for needle in (
    "preflight_verified_maintenance_steps(",
    "reason=step-journal",
    "step_mutations=0",
    "MAINTENANCE_STEP_ADMISSION_OK",
    "preflight_before_audit_mutation=1",
    "replay_safe_from_boot_start=1",
):
    require(syscall, needle, "M79 pre-EL0 three-ledger admission")
if not (
    syscall.index("verify_maintenance_authorization(")
    < syscall.index("preflight_verified_maintenance_steps(")
    < syscall.index("admit_verified_maintenance_authorization(")
    < syscall.index("MAINTENANCE_AUTHORIZATION_PREPARATION.publish(evidence)")
):
    raise SystemExit("M79 signature, step preflight, audit, and publication order changed")

main = read("kernel/src/main.rs")
for needle in (
    "fn commit_m79_maintenance_step(",
    "fn validate_m79_maintenance_step_terminal(",
    "MAINTENANCE_STEP_COMMIT_OK",
    "host_cut_window_ticks=50",
    "MAINTENANCE_STEP_TERMINAL_OK",
    "runtime_material[160..192]",
    "UNIFIED_PRODUCT_MAINTENANCE_STEP_OK",
    "terminal_bound_into_aggregate=1",
    "external_effect_exactly_once_claim=0",
    "arbitrary_resume_claim=0",
    "BOOT_OK: M79 unified real UI is interactive",
    "BOOT_OK: M79 durable three-step maintenance journal",
):
    require(main, needle, "M79 kernel-observed fixed-program boundary")
if not (
    main.index("let maintenance_step_three = {")
    < main.index("let maintenance_step_terminal = {")
    < main.index("let maintenance_execution = {")
    < main.index("close_verified_device_health(", main.index("let maintenance_execution = {"))
):
    raise SystemExit("M79 step, terminal, aggregate, and health-close order changed")

qmp = read("scripts/unified_product_qmp.py")
for needle in (
    '"M79": (',
    '"MAINTENANCE_STEP_ADMISSION_OK "',
    '"MAINTENANCE_STEP_TERMINAL_OK "',
    '"UNIFIED_PRODUCT_MAINTENANCE_STEP_OK "',
    '"recover-step3",',
):
    require(qmp, needle, "M79 positive QMP driver")
if qmp.count('"qemu-system-aarch64"') != 1 or qmp.count('"-nic"') != 1:
    raise SystemExit("M79 positive QMP launch must contain exactly one -nic none")

interrupt = read("scripts/maintenance_step_interrupt_qemu.py")
for needle in (
    "MAINTENANCE_STEP_COMMIT_OK",
    "cut_after_step=",
    "host_stop_after_marker=1",
    "hardware_powercut_claim=0",
    '"-nic",\n            "none",',
):
    require(interrupt, needle, "M79 multi-cutpoint host-interruption driver")
if interrupt.count('"qemu-system-aarch64"') != 1 or interrupt.count('"-nic"') != 1:
    raise SystemExit("M79 interruption launch must contain exactly one -nic none")

negative = read("scripts/maintenance_authorization_negative_qemu.py")
for needle in (
    '"--step-runtime"',
    "step_slots_mutated=0",
    "UNIFIED_PRODUCT_MAINTENANCE_STEP_OK",
):
    require(negative, needle, "M79 fail-closed negative driver")
if negative.count('"qemu-system-aarch64"') != 1 or negative.count('"-nic"') != 1:
    raise SystemExit("M79 negative QEMU launch must contain exactly one -nic none")

evidence = read("scripts/unified_product_maintenance_step_evidence.py")
for needle in (
    "UNIFIED_PRODUCT_MAINTENANCE_STEP_PARSER_SELF_TEST_OK",
    "decode_step_record",
    "expected_step_chains",
    "corrupt_newest_step",
    "cut_states=step1/step2/step3",
    "terminal_chain_bound_into_aggregate=1",
    "external_effect_exactly_once_claim=0",
    "hardware_powercut_claim=0",
):
    require(evidence, needle, "strict M79 log and three-ledger parser")

runtime = read("scripts/check-unified-product-maintenance-step-runtime.sh")
for needle in (
    "run_positive sequence1 M78",
    "run_positive normal M79",
    'run_interrupt 1 "$CUT1_IMAGE"',
    'run_interrupt 2 "$CUT2_IMAGE"',
    'run_interrupt 3 "$CUT3_IMAGE"',
    "corrupt-newest-step",
    "run_positive recover-corrupt M79",
    'run_negative completed-replay "$SEQUENCE2_KERNEL"',
    "unified_product_maintenance_step_evidence.py",
):
    require(runtime, needle, "five-positive/six-host-terminated M79 runtime gate")

test_sh = read("scripts/test.sh")
for needle in (
    '"$SCRIPT_DIR/check-unified-product-maintenance-step-static.sh"',
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndr-abi "
    "--features unified-product-maintenance-step-runtime --lib",
    "cargo test --locked --target \"$HOST_TRIPLE\" -p bndroid-kernel "
    "--features unified-product-maintenance-step-runtime --lib",
    'BNDROID_PROFILE=release "$SCRIPT_DIR/'
    'check-unified-product-maintenance-step-runtime.sh"',
    "unified_product_maintenance_step_static=1",
    "unified_product_maintenance_step_reboot=1",
    "unified_product_maintenance_step_positive_boots=5",
    "maintenance_step_interrupted_boots=3",
    "maintenance_step_fail_closed_pre_el0_boots=3",
    "maintenance_step_idempotent_replays=7",
    "maintenance_step_corruption_fallbacks=1",
    "maintenance_step_cutpoints=3",
    "maintenance_step_terminal_chain_bound=1",
    "maintenance_step_completed_replay_slot_mutations=0",
    "maintenance_step_fixed_program_steps=3",
    "qemu_psci_self_exits=57",
    "qemu_self_exits=65",
):
    require(test_sh, needle, "full-suite M79 integration", count=1)

print(
    "UNIFIED_PRODUCT_MAINTENANCE_STEP_SOURCE_OK "
    "abi=40 syscalls=0-57 audit_state=BNDRMAU1 execution_state=BNDRMEX1 "
    "step_state=BNDRMST1 audit_slots=5/6 execution_slots=7/8 step_slots=9/10 "
    "fixed_steps=3 migration_anchor=m78-sequence1 exact_authorization=1 "
    "deterministic_effects=3 no_step_gap=1 terminal_bound_into_aggregate=1 "
    "multi_cutpoint_recovery=3 corruption_fallback=1 write_flush_readback=1 "
    "network_access=0 idempotent_reconciliation_claim=1 "
    "external_effect_exactly_once_claim=0 arbitrary_resume_claim=0 "
    "trusted_monotonic_backend=0 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
    "efuse_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 "
    "emulator_only=1 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_step_interrupt_qemu.py" \
  "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_step_evidence.py" self-test
bash -n \
  "$SCRIPT_DIR/build-kernel.sh" \
  "$SCRIPT_DIR/check-unified-product-maintenance-step-runtime.sh"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
ABI_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features unified-product-maintenance-step-runtime --lib 2>&1
)"
printf '%s\n' "$ABI_LOG"
if [[ "$(grep -Fxc "test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" <<<"$ABI_LOG" || true)" != 1 ]]; then
  echo "M79 ABI host-test ledger changed." >&2
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
  echo "M79 maintenance host-test ledger changed." >&2
  exit 1
fi

M78_TEST_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features unified-product-maintenance-execution-runtime \
    persist::tests::maintenance_ --lib 2>&1
)"
printf '%s\n' "$M78_TEST_LOG"
if ! grep -Eq \
  'test result: ok\. 8 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$M78_TEST_LOG"; then
  echo "M78 historical maintenance host-test ledger changed under M79." >&2
  exit 1
fi

M77_TEST_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel \
    --features unified-product-maintenance-authorization-runtime \
    persist::tests::maintenance_ --lib 2>&1
)"
printf '%s\n' "$M77_TEST_LOG"
if ! grep -Eq \
  'test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out;' \
  <<<"$M77_TEST_LOG"; then
  echo "M77 historical maintenance host-test ledger changed under M79." >&2
  exit 1
fi

cargo check --locked -p bndroid-init \
  --features unified-product-maintenance-step-runtime
cargo check --locked -p bndroid-kernel \
  --features unified-product-maintenance-step-runtime \
  --bin bndroid-kernel

echo "M79 maintenance-step static contract passed."
