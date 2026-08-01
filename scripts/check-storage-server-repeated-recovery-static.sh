#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found; cannot verify the M57 repeated-recovery contracts." >&2
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
        'storage-server-repeated-recovery-runtime = ["storage-server-recovery-runtime"]',
        'app-data-async-recovery-runtime = ["app-data-runtime"]',
    ],
    "user/init/Cargo.toml": [
        'storage-server-recovery-runtime = ["storage-server-runtime"]',
        'storage-server-repeated-recovery-runtime = ["storage-server-recovery-runtime"]',
        'storage-server-async-recovery-runtime = ["storage-server-repeated-recovery-runtime"]',
    ],
    "kernel/build.rs": [
        "CARGO_FEATURE_STORAGE_SERVER_RECOVERY_RUNTIME",
        "CARGO_FEATURE_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
        "CARGO_FEATURE_STORAGE_IRQ_TIMEOUT_SELF_TEST",
        'feature=\\\"storage-server-repeated-recovery-runtime\\\"',
    ],
    "scripts/build-kernel.sh": [
        "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
        "USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
        "userspace repeated-recovery profile exceeds the kernel storage profile.",
        "userspace recovery profile exceeds the kernel storage profile.",
        "userspace storage profile requires a storage-enabled kernel.",
    ],
    "user/init/src/storage_server_runtime.rs": [
        "M57_STORAGE_READY_PREFIX",
        "M57_STORAGE_PROOF",
        'const RECOVERY_NAMESPACE: &str = "m57-repeated-recovery";',
        'not(feature = "storage-server-repeated-recovery-runtime")',
    ],
    "kernel/src/syscall.rs": [
        "if crate::storage::recovery_admission_closed()",
        "M57_STORAGE_READY_PREFIX",
        "M57_STORAGE_PROOF",
        "m57_storage_ready_proof_valid",
        "processes.created == 8",
        "broker.epoch == 7",
        "io.recovery_attempts == 7",
    ],
    "kernel/src/main.rs": [
        "validate_storage_server_repeated_recovery_runtime",
        "FAULT_SEQUENCE_WRF_WRF",
        "generation_chain_valid",
        "block.suppressed_read_notifications == 2",
        "block.suppressed_write_notifications == 2",
        "block.suppressed_flush_notifications == 2",
        "!storage::recovery_notification_fault_armed()",
        "!crate::interrupt::block_irq_rearm_commit_abort_armed()",
        "io.abandoned_services == 0",
        "!broker.abandoned_running",
        "BOOT_OK: M57 repeated fail-stop StorageServer recovery and retry verified",
    ],
    "kernel/src/storage_server_io.rs": [
        "with_broker_irq_masked",
        "pub fn broker_snapshot() -> storage_broker::BrokerSnapshot",
        "prepare_recovery_fault(request, broker_epoch)",
        "Err(RequestError::ServiceAbandoned)",
        "FAULT_CONTROL_EPOCH",
        "FAULT_SEQUENCE",
        "RECOVERY_REARM_ABORTS",
        "RECOVERY_FAIL_CLOSED_RETRIES",
        "RECOVERY_RETRY_PENDING",
        "RECOVERY_RETRY_PENDING.swap(false",
        "abort_next_block_irq_rearm_commit_for_test",
        "M57 IRQ rearm abort did not remain fail closed",
    ],
    "kernel/src/driver/virtio/block.rs": [
        "suppressed_read_notifications",
        "suppressed_write_notifications",
        "suppressed_flush_notifications",
        "consume_published(kind)",
        "recovery_notification_fault_armed",
    ],
    "kernel/src/interrupt.rs": [
        "abort_next_block_irq_rearm_commit_for_test",
        "block_irq_rearm_commit_abort_armed",
        "rollback_irq_rearm",
        "rollback_block_irq_rearm_masked",
    ],
    "kernel/src/storage.rs": [
        "suppress_next_notification_for_recovery_test",
        "recovery_notification_fault_armed",
        "recover_after_timeout",
    ],
    "kernel/src/storage_broker.rs": [
        "ServiceAbandoned",
        "abandoned_running_token",
        "Err(RequestError::ServiceAbandoned)",
    ],
    "scripts/check-storage-server-repeated-recovery-runtime.sh": [
        'FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime"',
        "requests != completions + 6",
        "M57 checker observed M58 evidence",
        "M57 checker observed M59 evidence",
        "M57 checker observed M60 evidence",
        "^STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "^BOOT_OK: M58 ",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "BOOT_OK: M60",
        "-nic none",
        "CARGO_NET_OFFLINE=true",
        "BOOT_OK: M57 repeated fail-stop StorageServer recovery and retry verified",
    ],
    "scripts/check-storage-server-recovery-runtime.sh": [
        'FEATURES="storage-server-runtime,storage-server-recovery-runtime"',
        "M56 checker observed M57 evidence",
        "M56 checker observed M59 evidence",
        "^STORAGE_SERVER_REPEATED_RECOVERY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "requests != completions + 3",
    ],
    "scripts/test.sh": [
        '"$SCRIPT_DIR/check-storage-server-repeated-recovery-static.sh"',
        'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-repeated-recovery-runtime.sh"',
        "storage_server_repeated_recovery=1",
    ],
}

texts = {}
for relative, needles in required.items():
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M57 static contract source is missing: {relative}")
    text = path.read_text()
    texts[relative] = text
    for needle in needles:
        if needle not in text:
            raise SystemExit(f"M57 static contract lost {needle!r}: {relative}")

# Cargo features are additive. Prove that M57 is a child of M56 in both
# packages, then prove every profile-specific M56 branch excludes M57.
for relative in ("kernel/Cargo.toml", "user/init/Cargo.toml"):
    text = texts[relative]
    if text.count('storage-server-repeated-recovery-runtime = ["storage-server-recovery-runtime"]') != 1:
        raise SystemExit(f"M57 feature parent is not exact and unique: {relative}")

kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
if feature_dependencies(kernel_cargo, shared_feature, "kernel/Cargo.toml"):
    raise SystemExit("the cooperative recovery feature unexpectedly has dependencies")
if feature_dependencies(kernel_cargo, storage_async_feature, "kernel/Cargo.toml") != (
    "storage-server-repeated-recovery-runtime",
    shared_feature,
):
    raise SystemExit("kernel M58 no longer has the exact M57 + shared closure")
if feature_dependencies(user_cargo, storage_async_feature, "user/init/Cargo.toml") != (
    "storage-server-repeated-recovery-runtime",
):
    raise SystemExit("userspace M58 no longer has the exact M57 parent")
if feature_dependencies(kernel_cargo, "app-data-runtime", "kernel/Cargo.toml").count(
    shared_feature
) != 1:
    raise SystemExit("kernel AppData runtime no longer selects shared cooperative recovery")
for relative, text in (
    ("kernel/Cargo.toml", kernel_cargo),
    ("user/init/Cargo.toml", user_cargo),
):
    if feature_dependencies(text, appdata_async_feature, relative) != ("app-data-runtime",):
        raise SystemExit(f"M59 AppData feature parent changed: {relative}")
if feature_dependencies(
    kernel_cargo, "storage-irq-timeout-self-test", "kernel/Cargo.toml"
) != (shared_feature,):
    raise SystemExit("low-level timeout profile no longer selects shared recovery")

main = texts["kernel/src/main.rs"]
syscall = texts["kernel/src/syscall.rs"]
userspace = texts["user/init/src/storage_server_runtime.rs"]
for relative, text, symbol in (
    ("kernel/src/main.rs", main, "validate_storage_server_recovery_runtime"),
    ("kernel/src/syscall.rs", syscall, "m56_storage_ready_proof_valid"),
):
    symbol_at = text.index(f"fn {symbol}")
    guard = text[max(0, symbol_at - 240):symbol_at]
    if 'not(feature = "storage-server-repeated-recovery-runtime")' not in guard:
        raise SystemExit(f"M56 profile branch is not excluded from M57: {relative}:{symbol}")

m56_init = re.search(
    r'#\[cfg\(all\(\s*feature = "storage-server-recovery-runtime",\s*'
    r'not\(feature = "storage-server-repeated-recovery-runtime"\)\s*\)\)\]\s*'
    r'pub\(super\) fn init_runtime',
    userspace,
)
if m56_init is None:
    raise SystemExit("M56 userspace InitReady branch is not excluded from M57")

m57_start = userspace.index(
    '#[cfg(all(\n'
    '    feature = "storage-server-repeated-recovery-runtime",\n'
    '    not(feature = "storage-server-async-recovery-runtime")\n'
    '))]\n'
    'pub(super) fn init_runtime'
)
m57_end = userspace.index(
    '#[cfg(all(\n'
    '    feature = "storage-server-async-recovery-runtime",\n'
    '    not(feature = "storage-server-fault-policy-runtime")\n'
    '))]\n'
    'pub(super) fn init_runtime',
    m57_start,
)
m57_sequence = userspace[m57_start:m57_end]
for stage in ("WRITE", "READ", "FLUSH"):
    if m57_sequence.count(f"RECOVERY_STAGE_{stage}") != 2:
        raise SystemExit(f"M57 sequence did not contain exactly two {stage} faults")

# Check the build-kernel wrapper's explicit feature closure and reject rules.
build_kernel = texts["scripts/build-kernel.sh"]
closure_patterns = [
    r'if feature_list_contains "\$KERNEL_FEATURES" "storage-server-repeated-recovery-runtime"; then\s+'
    r'KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_RUNTIME=1',
    r'elif feature_list_contains "\$KERNEL_FEATURES" "storage-server-recovery-runtime"; then\s+'
    r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_RUNTIME=1',
    r'if feature_list_contains "\$USERSPACE_FEATURES" "storage-server-repeated-recovery-runtime"; then\s+'
    r'USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_RUNTIME=1',
    r'elif feature_list_contains "\$USERSPACE_FEATURES" "storage-server-recovery-runtime"; then\s+'
    r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_RUNTIME=1',
]
for pattern in closure_patterns:
    if re.search(pattern, build_kernel) is None:
        raise SystemExit("build-kernel lost base/M56/M57 feature closure")

for feature in (
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
):
    append = f'append_feature "$USERSPACE_FEATURES" "{feature}"'
    if build_kernel.count(append) != 1:
        raise SystemExit(f"build-kernel did not mirror {feature} exactly once")

for user_flag, kernel_flag in (
    ("USER_STORAGE_SERVER_RUNTIME", "KERNEL_STORAGE_SERVER_RUNTIME"),
    ("USER_STORAGE_SERVER_RECOVERY_RUNTIME", "KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME"),
    (
        "USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
        "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME",
    ),
):
    rejection = re.compile(
        rf'"\${user_flag}" -eq 1\s+\\\s*\n\s*&& "\${kernel_flag}" -ne 1'
    )
    if rejection.search(build_kernel) is None:
        raise SystemExit(f"build-kernel lost high-profile rejection: {user_flag}")

# Keep the historical M56 checker isolated and make the M57 three-line schema
# exact. Request/completion totals remain build-dependent; only their delta is
# stable.
m56_checker = texts["scripts/check-storage-server-recovery-runtime.sh"]
m56_features = re.search(r'^FEATURES="([^"]*)"$', m56_checker, re.MULTILINE)
if m56_features is None or m56_features.group(1).split(",") != [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
]:
    raise SystemExit("M56 runtime checker changed its exact feature closure")

m57_checker = texts["scripts/check-storage-server-repeated-recovery-runtime.sh"]
m57_features = re.search(r'^FEATURES="([^"]*)"$', m57_checker, re.MULTILINE)
if m57_features is None or m57_features.group(1).split(",") != [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
]:
    raise SystemExit("M57 runtime checker changed its exact feature closure")
for label, enabled in (("M56", m56_features.group(1)), ("M57", m57_features.group(1))):
    for forbidden in (
        storage_async_feature,
        appdata_async_feature,
        "storage-irq-timeout-self-test",
        shared_feature,
    ):
        if forbidden in enabled:
            raise SystemExit(f"{label} runtime checker enabled higher-profile evidence: {forbidden}")
for evidence in (
    "M57 checker observed M58 evidence",
    "M57 checker observed M59 evidence",
    "M57 checker observed M60 evidence",
    "^STORAGE_SERVER_ASYNC_RECOVERY_OK",
    "APPDATA_ASYNC_RECOVERY_OK",
    "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
    "^BOOT_OK: M58 ",
    "STORAGE_SERVER_FAULT_POLICY_OK",
    "BOOT_OK: M60",
):
    if evidence not in m57_checker:
        raise SystemExit(f"M57 runtime checker no longer rejects M58 evidence: {evidence}")

m57_marker = (
    "STORAGE_SERVER_REPEATED_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF "
    "read_requires_reset=2 mutation_outcome_unknown=4 control_sequences=6 "
    "injected_reads=2 injected_writes=2 injected_flushes=2 owner_exits=6 "
    "broker_releases=6 broker_abandoned=6 recovery_attempts=7 recovery_commits=6 "
    "recovery_rollbacks=1 injected_rearm_aborts=1 fail_closed_retries=1 "
    "driver_timeouts=6 driver_resets=7 final_epoch=7"
)
if main.count(m57_marker) != 1:
    raise SystemExit("M57 kernel marker lost its exact repeated-recovery prefix")
if main.count("BOOT_OK: M57 repeated fail-stop StorageServer recovery and retry verified") != 1:
    raise SystemExit("M57 kernel BOOT_OK marker was not exact and unique")

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
        f"M57 offline boundary failed: qemu_launches={launches} nic_none={nic_none}"
    )

print(
    "STORAGE_SERVER_REPEATED_RECOVERY_STATIC_OK "
    f"sources={len(required)} qemu_launches={launches} nic_none={nic_none} "
    "cycles=2 fault_cases=6 owner_rotations=6 fail_closed_retries=1 "
    "kernel_reset_authority=1"
)
PY
