#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found; cannot verify the M56 recovery contracts." >&2
  exit 1
fi

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])

shared_feature = "cooperative-block-recovery"
storage_async_feature = "storage-server-async-recovery-runtime"
appdata_async_feature = "app-data-async-recovery-runtime"


def feature_dependencies(text: str, name: str, relative: str) -> tuple[str, ...]:
    matches = list(
        re.finditer(
            rf"^{re.escape(name)}\s*=\s*\[(.*?)\]\s*$",
            text,
            flags=re.MULTILINE | re.DOTALL,
        )
    )
    if len(matches) != 1:
        raise SystemExit(f"Cargo feature is not exact and unique: {relative}:{name}")
    body = matches[0].group(1)
    dependencies = tuple(re.findall(r'"([^"]+)"', body))
    residue = re.sub(r'"[^"]+"\s*,?', "", body)
    residue = re.sub(r"#[^\n]*", "", residue)
    if residue.strip():
        raise SystemExit(f"Cargo feature has an unparsed dependency: {relative}:{name}")
    return dependencies

required = {
    "kernel/Cargo.toml": [
        'cooperative-block-recovery = []',
        'storage-server-recovery-runtime = ["storage-server-runtime"]',
        'app-data-async-recovery-runtime = ["app-data-runtime"]',
    ],
    "user/init/Cargo.toml": [
        'storage-server-recovery-runtime = ["storage-server-runtime"]',
    ],
    "kernel/build.rs": [
        "CARGO_FEATURE_STORAGE_SERVER_RECOVERY_RUNTIME",
        "CARGO_FEATURE_STORAGE_IRQ_TIMEOUT_SELF_TEST",
        "bndroid_storage_irq_timeout_profile",
        'feature=\\\"storage-server-recovery-runtime\\\"',
    ],
    "kernel/src/virtio.rs": [
        "validate_block_irq_rearm_snapshot",
        "every_nonempty_rebuilt_queue_ledger_fails_closed",
    ],
    "kernel/src/driver/virtio/block.rs": [
        "cleanup_failed_recovery",
        "validate_block_identity(&self.transport)?;",
        "acknowledge_recovery_interrupt_tail",
        "observe_irq_rearm_tail",
        "suppress_next_notification_for_recovery_test",
    ],
    "kernel/src/driver/interrupt/gicv2.rs": [
        "reconfigure_enable_spi_preserving_pending",
        "self.write_distributor(GICD_ICACTIVER",
    ],
    "kernel/src/storage.rs": [
        "prepare_irq_rearm",
        "validate_irq_rearm_tail",
        "commit_irq_rearm",
        "rollback_irq_rearm",
        "suppress_next_notification_for_recovery_test",
    ],
    "kernel/src/interrupt.rs": [
        "crate::storage::prepare_irq_rearm",
        "crate::storage::validate_irq_rearm_tail",
        "crate::storage::commit_irq_rearm",
        "crate::storage::rollback_irq_rearm",
        "rollback_block_irq_rearm_masked",
        ".reconfigure_enable_spi_preserving_pending(",
    ],
    "kernel/src/storage_broker.rs": [
        "ServiceAbandoned",
        "abandoned_running_token",
        "RequestError::RecoveryRequired",
        "SessionError::RecoveryRequired",
        "self.abandoned_running_token != 0",
        "!matches!(&self.state, BlockState::RecoveryRequired)",
    ],
    "kernel/src/storage_server_io.rs": [
        "ABANDONED_SERVICES",
        "Err(RequestError::ServiceAbandoned)",
        "prepare_recovery_fault(request, broker_epoch)",
        "pub fn service_recovery()",
        "broker.state != 4 || broker.bound",
        "with_broker_irq_masked(storage_broker::complete_recovery)",
        "pub fn broker_snapshot() -> storage_broker::BrokerSnapshot",
    ],
    "kernel/src/syscall.rs": [
        "if crate::storage::recovery_admission_closed()",
        "RequestError::RecoveryRequired => Status::RequiresReset",
        "SessionError::RecoveryRequired => Status::RequiresReset",
        "m56_storage_ready_proof_valid",
    ],
    "user/init/src/storage_server_runtime.rs": [
        "fn fail_io(&mut self, error: IoError)",
        "self.pending_writes.fill([0; STORAGE_SECTOR_SIZE]);",
        "status.is_session_fatal()",
        "exit_child(SERVER_RECOVERY_EXIT_CODE)",
        "run_recovery_stage(startup, &mut io)",
    ],
    "crates/bndr-storage/src/lib.rs": [
        "pub const fn is_session_fatal(self) -> bool",
        "Self::OutcomeUnknown | Self::RequiresReset",
    ],
    "scripts/check-storage-server-recovery-runtime.sh": [
        'FEATURES="storage-server-runtime,storage-server-recovery-runtime"',
        "-nic none",
        "requests != completions + 3",
        "M56 checker observed M59 evidence",
        "M56 checker observed M60 evidence",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "BOOT_OK: M60",
        "BOOT_OK: M56 fail-stop StorageServer recovery verified",
    ],
}

texts = {}
for relative, needles in required.items():
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M56 static contract source is missing: {relative}")
    text = path.read_text()
    texts[relative] = text
    for needle in needles:
        if needle not in text:
            raise SystemExit(f"M56 static contract lost {needle!r}: {relative}")

# M56 remains the synchronous child of the base StorageServer profile. The
# extracted cooperative engine is shared only by M58, M59 AppData, and the
# low-level IRQ timeout profile; none of those may enter the M56 closure.
kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
if feature_dependencies(kernel_cargo, shared_feature, "kernel/Cargo.toml"):
    raise SystemExit("the cooperative recovery feature unexpectedly has dependencies")
for relative, text in (
    ("kernel/Cargo.toml", kernel_cargo),
    ("user/init/Cargo.toml", user_cargo),
):
    if feature_dependencies(text, "storage-server-recovery-runtime", relative) != (
        "storage-server-runtime",
    ):
        raise SystemExit(f"M56 feature parent changed: {relative}")
    if feature_dependencies(text, "storage-server-repeated-recovery-runtime", relative) != (
        "storage-server-recovery-runtime",
    ):
        raise SystemExit(f"M57 feature parent changed: {relative}")

if feature_dependencies(kernel_cargo, storage_async_feature, "kernel/Cargo.toml") != (
    "storage-server-repeated-recovery-runtime",
    shared_feature,
):
    raise SystemExit("kernel M58 no longer has the exact M57 + shared closure")
if feature_dependencies(user_cargo, storage_async_feature, "user/init/Cargo.toml") != (
    "storage-server-repeated-recovery-runtime",
):
    raise SystemExit("userspace M58 no longer has the exact M57 parent")
appdata_dependencies = feature_dependencies(kernel_cargo, "app-data-runtime", "kernel/Cargo.toml")
if appdata_dependencies.count(shared_feature) != 1:
    raise SystemExit("kernel AppData runtime no longer selects shared cooperative recovery")
if feature_dependencies(kernel_cargo, appdata_async_feature, "kernel/Cargo.toml") != (
    "app-data-runtime",
):
    raise SystemExit("kernel M59 AppData child is not exact")
if feature_dependencies(user_cargo, appdata_async_feature, "user/init/Cargo.toml") != (
    "app-data-runtime",
):
    raise SystemExit("userspace M59 AppData child is not exact")
if feature_dependencies(
    kernel_cargo, "storage-irq-timeout-self-test", "kernel/Cargo.toml"
) != (shared_feature,):
    raise SystemExit("low-level timeout profile no longer selects shared recovery")

m56_features = re.search(
    r'^FEATURES="([^"]*)"$',
    texts["scripts/check-storage-server-recovery-runtime.sh"],
    re.MULTILINE,
)
if m56_features is None or m56_features.group(1).split(",") != [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
]:
    raise SystemExit("M56 runtime checker changed its exact feature closure")
for forbidden in (
    storage_async_feature,
    appdata_async_feature,
    "storage-irq-timeout-self-test",
    shared_feature,
):
    if forbidden in m56_features.group(1):
        raise SystemExit(f"M56 runtime checker enabled higher-profile evidence: {forbidden}")

for path in (root / "kernel/src").rglob("*.rs"):
    if 'feature = "storage-irq-timeout-self-test"' in path.read_text():
        raise SystemExit(
            "timeout/M56 precedence escaped the generated effective-profile cfg: "
            f"{path.relative_to(root)}"
        )

main = (root / "kernel/src/main.rs").read_text()
marker = (
    "STORAGE_SERVER_RECOVERY_OK cases=3 read_requires_reset=1 "
    "mutation_outcome_unknown=2 control_sequences=3 injected_reads=1 "
    "injected_writes=1 injected_flushes=1 owner_exits=3 broker_releases=3 "
    "broker_abandoned=3 reset_attempts=3 reset_successes=3 reset_failures=0 "
    "driver_timeouts=3 driver_resets=3 final_epoch=4"
)
if marker not in main:
    raise SystemExit("M56 kernel marker lost its exact fail-stop/reset prefix")
if "BOOT_OK: M56 fail-stop StorageServer recovery verified" not in main:
    raise SystemExit("M56 kernel lost its unique BOOT_OK marker")

launch_pattern = re.compile(r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
launches = 0
nic_none = 0
for path in sorted((root / "scripts").glob("*.sh")):
    lines = path.read_text().splitlines()
    index = 0
    while index < len(lines):
        if not launch_pattern.match(lines[index]):
            index += 1
            continue
        block = [lines[index]]
        while block[-1].rstrip().endswith("\\"):
            index += 1
            if index >= len(lines):
                raise SystemExit(f"unterminated QEMU launch block: {path}")
            block.append(lines[index])
        launches += 1
        count = sum(bool(nic_none_pattern.match(line)) for line in block)
        nic_none += count
        if count != 1:
            raise SystemExit(f"QEMU launch must contain exactly one -nic none: {path}")
        index += 1
if launches == 0 or nic_none != launches:
    raise SystemExit(
        f"M56 offline boundary failed: qemu_launches={launches} nic_none={nic_none}"
    )

print(
    "STORAGE_SERVER_RECOVERY_STATIC_OK "
    f"sources={len(required)} qemu_launches={launches} nic_none={nic_none} "
    "fault_kinds=3 owner_rotation=1 kernel_reset_authority=1"
)
PY
