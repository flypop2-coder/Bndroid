#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify M78 maintenance execution contracts." >&2
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
        raise SystemExit(f"M78 contract source is missing: {relative}")
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
    "unified-product-maintenance-execution-runtime = [\n"
    '    "unified-product-maintenance-authorization-runtime",\n]',
    "ABI exact M78 feature closure",
    count=1,
)
for relative, text in (
    ("kernel/Cargo.toml", kernel_toml),
    ("user/init/Cargo.toml", user_toml),
):
    require(
        text,
        "unified-product-maintenance-execution-runtime = [",
        f"{relative} M78 feature declaration",
        count=1,
    )
    require(
        text,
        '"unified-product-maintenance-authorization-runtime",',
        f"{relative} M78 inherited M77 feature",
    )
    require(
        text,
        '"bndr-abi/unified-product-maintenance-execution-runtime",',
        f"{relative} M78 ABI feature forwarding",
        count=1,
    )

abi = read("crates/bndr-abi/src/lib.rs")
for needle in (
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-execution-runtime",\n'
    '    not(feature = "unified-product-maintenance-step-runtime")\n'
    "))]\n"
    "pub const ABI_VERSION: u64 = 39;",
    'not(feature = "unified-product-maintenance-execution-runtime")',
    "MaintenanceSessionOpen = 57",
    "57 => Some(Self::MaintenanceSessionOpen)",
    "assert_eq!(ABI_VERSION, 39);",
    "assert_eq!(SyscallNumber::from_raw(58), None);",
):
    require(abi, needle, "M78 ABI 39 with stable syscall 0-57")
if "MaintenanceExecution" in abi:
    raise SystemExit("M78 unexpectedly added an execution syscall")

user_main = read("user/init/src/main.rs")
require(
    user_main,
    '#[cfg(all(\n'
    '    feature = "unified-product-maintenance-execution-runtime",\n'
    '    not(feature = "unified-product-maintenance-step-runtime")\n'
    "))]\n"
    "const _: [(); 39] = [(); ABI_VERSION as usize];",
    "M78 userspace ABI pin",
    count=1,
)

build_kernel = read("scripts/build-kernel.sh")
for needle in (
    "KERNEL_UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_RUNTIME=0",
    "USER_UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_RUNTIME=0",
    'feature_list_contains "$KERNEL_FEATURES" '
    '"unified-product-maintenance-execution-runtime"',
    'feature_list_contains "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-execution-runtime"',
    'append_feature "$USERSPACE_FEATURES" '
    '"unified-product-maintenance-execution-runtime"',
    "userspace maintenance-execution profile requires the matching kernel profile.",
):
    require(build_kernel, needle, "M78 kernel/userspace feature forwarding")

persist = read("kernel/src/persist.rs")
for needle in (
    'pub const MAINTENANCE_EXECUTION_STATE_MAGIC: [u8; 8] = *b"BNDRMEX1";',
    "pub const MAINTENANCE_EXECUTION_STATE_BYTES: usize = 368;",
    "pub const MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS: "
    "[u64; DATA_SLOT_COUNT] = [7, 8];",
    "pub struct MaintenanceExecutionCompletionBinding",
    "pub enum MaintenanceExecutionStateError",
    "pub struct MaintenanceExecutionState",
    "pub fn maintenance_execution_chain_sha256",
    "pub enum MaintenanceExecutionError",
    "pub fn admit_maintenance_authorization_with_execution",
    "pub fn commit_maintenance_execution_completion",
    "MaintenanceExecutionError::CompletedReplay",
    "MaintenanceExecutionError::IncompletePredecessor",
    "MaintenanceExecutionError::AuthorizationBindingMismatch",
    "maintenance execution did not consume its exact bounded use count",
    "This record supports a narrowly bounded retry",
    "it is not a general exactly-once primitive",
    "maintenance_execution_admission_resumes_only_the_unfinished_exact_head",
    "maintenance_execution_rejects_resume_binding_substitution_without_mutation",
    "maintenance_execution_torn_completion_preserves_the_prior_head",
):
    require(persist, needle, "M78 dual-ledger persistence state machine")
if persist.count("MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS") < 7:
    raise SystemExit("M78 execution slots are not enforced at every transaction boundary")

storage_persist = read("kernel/src/storage_persist.rs")
for needle in (
    "MaintenanceExecutionTransaction(MaintenanceExecutionError)",
    "pub fn admit_verified_maintenance_authorization",
    "pub fn commit_verified_maintenance_execution",
    "admit_maintenance_authorization_with_execution(&mut io, request)",
    "commit_maintenance_execution_completion(&mut io, request)",
):
    require(storage_persist, needle, "M78 IRQ-backed kernel-only persistence adapter")

syscall = read("kernel/src/syscall.rs")
for needle in (
    "admit_verified_maintenance_authorization(",
    "MaintenanceExecutionError::CompletedReplay",
    "reason=completed-replay",
    "MAINTENANCE_EXECUTION_ADMISSION_OK",
    "completed_sequence_before",
    "audit_write_flush_readback={}",
    "exactly_once_claim=0",
):
    require(syscall, needle, "M78 pre-EL0 exact-head admission")
if not (
    syscall.index("verify_maintenance_authorization(")
    < syscall.index("admit_verified_maintenance_authorization(")
    < syscall.index("MAINTENANCE_AUTHORIZATION_PREPARATION.publish(evidence)")
):
    raise SystemExit("M78 signature, durable admission, and publication order changed")

main = read("kernel/src/main.rs")
for needle in (
    "let maintenance_execution = {",
    "MaintenanceExecutionCompletionBinding",
    "runtime_material[128..160]",
    "commit_verified_maintenance_execution(",
    "MAINTENANCE_EXECUTION_COMMIT_OK",
    "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_OK",
    "completion_before_device_health_close=1",
    "audit_unchanged_on_resume={}",
    "exactly_once_claim=0",
    "arbitrary_resume_claim=0",
    "BOOT_OK: M78 unified real UI is interactive",
    "BOOT_OK: M78 durable maintenance execution completion",
):
    require(main, needle, "M78 kernel-validated completion boundary")
if not (
    main.index("let maintenance_execution = {")
    < main.index("close_verified_device_health(", main.index("let maintenance_execution = {"))
    < main.index(
        "seal_clean_shutdown_admission_masked",
        main.index("let maintenance_execution = {"),
    )
):
    raise SystemExit("M78 completion, device-health close, and final seal order changed")

qmp = read("scripts/unified_product_qmp.py")
for needle in (
    '"M78": (',
    '"MAINTENANCE_EXECUTION_ADMISSION_OK "',
    '"MAINTENANCE_EXECUTION_COMMIT_OK "',
    '"UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_OK "',
    '"resume",',
):
    require(qmp, needle, "M78 positive QMP driver")
if qmp.count('"qemu-system-aarch64"') != 1 or qmp.count('"-nic"') != 1:
    raise SystemExit("M78 positive QMP launch must contain exactly one -nic none")

interrupt = read("scripts/maintenance_execution_interrupt_qemu.py")
for needle in (
    "MAINTENANCE_EXECUTION_ADMISSION_OK",
    "MAINTENANCE_EXECUTION_COMMIT_OK",
    "host_interrupted_before_completion=1",
    "powercut_claim=0",
    '"-nic",\n            "none",',
):
    require(interrupt, needle, "M78 host-interruption driver")
if interrupt.count('"qemu-system-aarch64"') != 1 or interrupt.count('"-nic"') != 1:
    raise SystemExit("M78 interruption launch must contain exactly one -nic none")

negative = read("scripts/maintenance_authorization_negative_qemu.py")
for needle in (
    '"completed-replay"',
    "marker_format = 2 if reason == \"completed-replay\" else 1",
    "execution_slots_mutated=0",
):
    require(negative, needle, "M78 completed-replay negative driver")
if negative.count('"qemu-system-aarch64"') != 1 or negative.count('"-nic"') != 1:
    raise SystemExit("M78 negative QEMU launch must contain exactly one -nic none")

evidence = read("scripts/unified_product_maintenance_execution_evidence.py")
for needle in (
    "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_PARSER_SELF_TEST_OK",
    "decode_execution_record",
    "interruption_state=audit2/execution1",
    "resume_state=audit2/execution2",
    "audit_unchanged_on_resume=1",
    "exactly_once_claim=0",
    "host_rollback_resistance=0",
):
    require(evidence, needle, "strict M78 log and dual-ledger parser")

runtime = read("scripts/check-unified-product-maintenance-execution-runtime.sh")
for needle in (
    "run_positive sequence1 M78",
    "run_interrupted_sequence2",
    "run_positive resume M78",
    'run_negative signature "$SIGNATURE_KERNEL"',
    'run_negative binding "$BINDING_KERNEL"',
    'run_negative completed-replay "$SEQUENCE2_KERNEL"',
    "unified_product_maintenance_execution_evidence.py",
    "M78_INTERRUPT_BOOT_SEQUENCE",
):
    require(runtime, needle, "six-positive/four-host-terminated M78 runtime gate")

print(
    "UNIFIED_PRODUCT_MAINTENANCE_EXECUTION_SOURCE_OK "
    "abi=39 syscalls=0-57 audit_state=BNDRMAU1 execution_state=BNDRMEX1 "
    "audit_slots=5/6 execution_slots=7/8 exact_unfinished_head=1 "
    "predecessor_completion_required=1 bounded_resume=1 completed_replay_rejection=1 "
    "runtime_validated_completion=1 write_flush_readback=1 old_slot_preserved=1 "
    "network_access=0 exactly_once_claim=0 arbitrary_resume_claim=0 "
    "trusted_monotonic_backend=0 production_key_claim=0 hsm_claim=0 rpmb_claim=0 "
    "efuse_claim=0 host_rollback_resistance=0 hardware_powercut_claim=0 "
    "emulator_only=1 real_phone_claim=0"
)
PY

python3 -m py_compile \
  "$SCRIPT_DIR/unified_product_qmp.py" \
  "$SCRIPT_DIR/maintenance_execution_interrupt_qemu.py" \
  "$SCRIPT_DIR/maintenance_authorization_negative_qemu.py" \
  "$SCRIPT_DIR/unified_product_maintenance_execution_evidence.py"
python3 "$SCRIPT_DIR/unified_product_maintenance_execution_evidence.py" self-test
bash -n \
  "$SCRIPT_DIR/build-kernel.sh" \
  "$SCRIPT_DIR/check-unified-product-maintenance-execution-runtime.sh"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
ABI_LOG="$(
  cargo test --locked --target "$HOST_TRIPLE" -p bndr-abi \
    --features unified-product-maintenance-execution-runtime --lib 2>&1
)"
printf '%s\n' "$ABI_LOG"
if [[ "$(grep -Fxc "test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s" <<<"$ABI_LOG" || true)" != 1 ]]; then
  echo "M78 ABI host-test ledger changed." >&2
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
  echo "M78 maintenance host-test ledger changed." >&2
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
  echo "M77 historical maintenance host-test ledger changed under M78." >&2
  exit 1
fi

cargo check --locked -p bndroid-init \
  --features unified-product-maintenance-execution-runtime
cargo check --locked -p bndroid-kernel \
  --features unified-product-maintenance-execution-runtime \
  --bin bndroid-kernel

echo "M78 maintenance execution static contract passed."
