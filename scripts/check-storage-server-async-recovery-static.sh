#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 mktemp rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M58 asynchronous-recovery contracts." >&2
    exit 1
  fi
done

# This is non-executing syntax validation. In particular it never reaches any
# QEMU launch contained in the checked scripts.
for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
feature = "storage-server-async-recovery-runtime"
parent = "storage-server-repeated-recovery-runtime"
shared_feature = "cooperative-block-recovery"
appdata_async_feature = "app-data-async-recovery-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M58 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(relative: str, needle: str, description: str) -> None:
    if needle not in source(relative):
        raise SystemExit(f"M58 static contract lost {description}: {relative}")


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
        f'{shared_feature} = []',
        f'{appdata_async_feature} = ["app-data-runtime"]',
    ],
    "user/init/Cargo.toml": [
        f'{feature} = ["{parent}"]',
    ],
    "kernel/build.rs": [
        "CARGO_FEATURE_COOPERATIVE_BLOCK_RECOVERY",
        "CARGO_FEATURE_APP_DATA_ASYNC_RECOVERY_RUNTIME",
        "CARGO_FEATURE_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
        'feature=\\\"storage-server-async-recovery-runtime\\\"',
    ],
    "scripts/build-kernel.sh": [
        "KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
        "USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME",
        "userspace async-recovery profile exceeds the kernel storage profile.",
        "AppData runtime and StorageServer runtime profiles are mutually exclusive.",
    ],
    "kernel/src/virtio.rs": [
        "pub struct BlockRecoveryTracker",
        "BlockRecoveryPhase::AwaitReset",
        "BlockRecoveryPhase::AwaitStableCapacity",
        "BlockRecoveryPhase::AwaitCleanupReset",
        "cooperative_recovery_requires_every_success_phase_and_clears_deadlines",
        "cooperative_recovery_cleanup_is_yieldable_and_retryable",
    ],
    "kernel/src/driver/virtio/block.rs": [
        "pub fn begin_cooperative_recovery",
        "pub fn poll_cooperative_recovery",
        "fn observe_recovery_capacity_once",
        "fn begin_cooperative_cleanup",
        "CooperativeRecoveryProgress::Pending",
    ],
    "kernel/src/storage.rs": [
        "RECOVERY_ADMISSION_CLOSED",
        "pub fn recovery_admission_closed()",
        "pub fn begin_async_recovery",
        "pub fn poll_async_recovery",
        "fn with_async_recovery_device",
        "ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS.fetch_max",
        "masked_poll_iterations: 0",
        "pub(crate) fn open_recovery_admission",
    ],
    "kernel/src/storage_server_io.rs": [
        "M58 async storage recovery monitor entered with local IRQ masked",
        "storage::begin_async_recovery",
        "storage::poll_async_recovery",
        "disable_block_irq_masked",
        "reenable_block_irq_masked",
        "rollback_block_irq_rearm_masked",
        "storage::open_recovery_admission",
        "record_async_acquire_wait",
        "ASYNC_TIMER_PROGRESS_WINDOWS",
        "ASYNC_WORKER_PROGRESS_WINDOWS",
        "ASYNC_EL0_PROGRESS_WINDOWS",
    ],
    "kernel/src/syscall.rs": [
        "if crate::storage::recovery_admission_closed()",
        "crate::storage_server_io::record_async_acquire_wait();",
        "M58_STORAGE_READY_PREFIX",
        "M58_STORAGE_PROOF",
        "m58_storage_ready_proof_valid",
    ],
    "kernel/src/main.rs": [
        "validate_storage_server_async_recovery_runtime",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF",
        "masked_poll_iterations={}",
        "long_daif_masks=0",
        "BOOT_OK: M58 cooperative fail-stop StorageServer recovery verified",
    ],
    "user/init/src/storage_server_runtime.rs": [
        "M58_STORAGE_READY_PREFIX",
        "M58_STORAGE_PROOF",
        'const RECOVERY_NAMESPACE: &str = "m58-async-recovery";',
    ],
    "scripts/check-storage-server-async-recovery-runtime.sh": [
        'FEATURES="storage-server-runtime,storage-server-recovery-runtime,storage-server-repeated-recovery-runtime,storage-server-async-recovery-runtime"',
        "--parser-self-test",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "STORAGE_SERVER_FAULT_POLICY_OK",
        "BOOT_OK: M60",
        "negative_cases=18",
        "masked_poll_iterations=0",
        "long_daif_masks=0",
        "-nic none",
        "CARGO_NET_OFFLINE=true",
    ],
    "scripts/check-storage-server-repeated-recovery-runtime.sh": [
        "M57 checker observed M58 evidence",
        "M57 checker observed M59 evidence",
        "STORAGE_SERVER_ASYNC_RECOVERY_OK",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
        "BOOT_OK: M58 ",
    ],
    "scripts/check-storage-server-repeated-recovery-static.sh": [
        "storage-server-async-recovery-runtime",
        "M57 checker observed M58 evidence",
        "M57 checker observed M59 evidence",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
    ],
    "scripts/check-storage-server-recovery-runtime.sh": [
        "M56 checker observed M59 evidence",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
    ],
    "scripts/check-storage-server-recovery-static.sh": [
        "M56 checker observed M59 evidence",
        "APPDATA_ASYNC_RECOVERY_OK",
        "STORAGE_IRQ_COOPERATIVE_RECOVERY_OK",
    ],
    "scripts/test.sh": [
        '"$SCRIPT_DIR/check-storage-server-async-recovery-static.sh"',
        'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-async-recovery-runtime.sh"',
        "storage_server_async_recovery_static=1",
        "storage_server_async_recovery=1",
    ],
}

texts = {relative: source(relative) for relative in required}
for relative, needles in required.items():
    for needle in needles:
        if needle not in texts[relative]:
            raise SystemExit(f"M58 static contract lost {needle!r}: {relative}")

# The kernel M58 feature is the exact union of its historical M57 parent and
# the shared cooperative driver/storage engine. Userspace still has only the
# M57 parent. M59 AppData and the low-level IRQ profile select the same engine
# without acquiring any StorageServer coordinator feature.
kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
if feature_dependencies(kernel_cargo, shared_feature, "kernel/Cargo.toml"):
    raise SystemExit("the cooperative recovery feature unexpectedly has dependencies")
if feature_dependencies(kernel_cargo, feature, "kernel/Cargo.toml") != (
    parent,
    shared_feature,
):
    raise SystemExit("kernel M58 no longer has the exact M57 + shared closure")
if feature_dependencies(user_cargo, feature, "user/init/Cargo.toml") != (parent,):
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

build = texts["scripts/build-kernel.sh"]
closure_patterns = (
    r'if feature_list_contains "\$KERNEL_FEATURES" "storage-server-async-recovery-runtime"; then\s+'
    r'KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'KERNEL_STORAGE_SERVER_RUNTIME=1',
    r'if feature_list_contains "\$USERSPACE_FEATURES" "storage-server-async-recovery-runtime"; then\s+'
    r'USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
    r'USER_STORAGE_SERVER_RUNTIME=1',
)
for pattern in closure_patterns:
    if re.search(pattern, build) is None:
        raise SystemExit("build-kernel lost the exact M58->M57->M56->M55 closure")

for mirrored in (
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
    "storage-server-async-recovery-runtime",
):
    append = f'append_feature "$USERSPACE_FEATURES" "{mirrored}"'
    if build.count(append) != 1:
        raise SystemExit(f"build-kernel did not mirror {mirrored} exactly once")

rejection = re.compile(
    r'"\$USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME" -eq 1\s+\\\s*\n\s*'
    r'&& "\$KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME" -ne 1'
)
if rejection.search(build) is None:
    raise SystemExit("build-kernel lost the userspace-above-kernel M58 rejection")

mutual_exclusion = re.compile(
    r'"\$KERNEL_APP_DATA_RUNTIME" -eq 1\s+\\\s*\n\s*'
    r'&& "\$KERNEL_STORAGE_SERVER_RUNTIME" -eq 1'
)
if mutual_exclusion.search(build) is None:
    raise SystemExit("build-kernel no longer excludes the AppData and StorageServer branches")

m58_runtime_features = re.search(
    r'^FEATURES="([^"]*)"$',
    texts["scripts/check-storage-server-async-recovery-runtime.sh"],
    re.MULTILINE,
)
if m58_runtime_features is None or m58_runtime_features.group(1).split(",") != [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
    feature,
]:
    raise SystemExit("M58 runtime checker changed its exact feature closure")
for forbidden in (appdata_async_feature, "storage-irq-timeout-self-test", shared_feature):
    if forbidden in m58_runtime_features.group(1):
        raise SystemExit(f"M58 runtime checker directly enabled a foreign profile: {forbidden}")


def cfg_window(text: str, symbol: str, width: int = 500) -> str:
    position = text.index(symbol)
    return text[max(0, position - width):position]


m58_cfg = 'not(feature = "storage-server-async-recovery-runtime")'
main = texts["kernel/src/main.rs"]
syscall = texts["kernel/src/syscall.rs"]
userspace = texts["user/init/src/storage_server_runtime.rs"]
for relative, text, symbol in (
    ("kernel/src/main.rs", main, "fn validate_storage_server_repeated_recovery_runtime"),
    ("kernel/src/syscall.rs", syscall, "const M57_STORAGE_READY_PREFIX"),
    ("kernel/src/syscall.rs", syscall, "fn m57_storage_ready_proof_valid"),
    ("user/init/src/storage_server_runtime.rs", userspace, "const M57_STORAGE_READY_PREFIX"),
    ("user/init/src/storage_server_runtime.rs", userspace, 'const RECOVERY_NAMESPACE: &str = "m57-repeated-recovery"'),
):
    if m58_cfg not in cfg_window(text, symbol):
        raise SystemExit(f"M57 additive profile branch does not exclude M58: {relative}:{symbol}")

if re.search(
    r'#\[cfg\(all\(\s*feature = "storage-server-repeated-recovery-runtime",\s*'
    r'not\(feature = "storage-server-async-recovery-runtime"\)\s*\)\)\]\s*'
    r'pub\(super\) fn init_runtime',
    userspace,
) is None:
    raise SystemExit("M57 userspace init branch does not exclude M58")
if re.search(
    r'#\[cfg\(all\(\s*feature = "storage-server-repeated-recovery-runtime",\s*'
    r'not\(feature = "storage-server-async-recovery-runtime"\)\s*\)\)\]\s*'
    r'validate_storage_server_repeated_recovery_runtime\(\);',
    main,
) is None:
    raise SystemExit("M57 kernel dispatch does not exclude M58")
if re.search(
    r'#\[cfg\(all\(\s*feature = "storage-server-async-recovery-runtime",\s*'
    r'not\(feature = "storage-server-fault-policy-runtime"\)\s*\)\)\]\s*'
    r'validate_storage_server_async_recovery_runtime\(\);',
    main,
) is None:
    raise SystemExit("M58 kernel dispatch is missing or ambiguous")

# Select the exact implementation slices instead of merely looking for async
# names somewhere in the tree. Comments are removed before searching for loop
# tokens, because the implementation comments deliberately explain which old
# loops are forbidden.
driver = texts["kernel/src/driver/virtio/block.rs"]
storage = texts["kernel/src/storage.rs"]
coordinator = texts["kernel/src/storage_server_io.rs"]
driver_slice = driver[
    driver.index("pub fn begin_cooperative_recovery"):
    driver.index("fn rebuild_after_confirmed_reset", driver.index("pub fn begin_cooperative_recovery"))
]
storage_slice = storage[
    storage.index("pub fn begin_async_recovery"):
    storage.index("pub fn recover_after_timeout", storage.index("pub fn begin_async_recovery"))
]
coordinator_slice = coordinator[
    coordinator.index("/// Advances M58 recovery"):
    coordinator.index("fn active_fault_policy_ticket", coordinator.index("/// Advances M58 recovery"))
]

shared_cfg = '#[cfg(feature = "cooperative-block-recovery")]'


def item_attributes(text: str, symbol: str) -> list[str]:
    prefix = text[:text.index(symbol)].rstrip()
    lines = prefix.splitlines()
    attributes = []
    while lines and lines[-1].lstrip().startswith("#["):
        attributes.append(lines.pop().strip())
    return attributes


for relative, text, symbol in (
    ("kernel/src/driver/virtio/block.rs", driver, "pub enum CooperativeRecoveryProgress"),
    ("kernel/src/driver/virtio/block.rs", driver, "pub fn begin_cooperative_recovery"),
    ("kernel/src/driver/virtio/block.rs", driver, "pub fn poll_cooperative_recovery"),
    ("kernel/src/driver/virtio/block.rs", driver, "fn begin_cooperative_cleanup"),
    ("kernel/src/driver/virtio/block.rs", driver, "fn observe_recovery_capacity_once"),
    ("kernel/src/storage.rs", storage, "pub struct AsyncRecoverySnapshot"),
    ("kernel/src/storage.rs", storage, "pub fn begin_async_recovery"),
    ("kernel/src/storage.rs", storage, "pub fn poll_async_recovery"),
    ("kernel/src/storage.rs", storage, "fn with_async_recovery_device"),
):
    attributes = item_attributes(text, symbol)
    if shared_cfg not in attributes:
        raise SystemExit(f"shared cooperative implementation lost its cfg: {relative}:{symbol}")
    foreign_cfgs = [
        attribute
        for attribute in attributes
        if attribute.startswith("#[cfg(") and attribute != shared_cfg
    ]
    if foreign_cfgs:
        raise SystemExit(
            f"shared cooperative implementation gained a profile cfg: {relative}:{symbol}"
        )

explicit_admission_signature = '''#[cfg(any(
    feature = "cooperative-block-recovery",
    feature = "storage-server-runtime"
))]
pub(crate) fn open_recovery_admission'''
if storage.count(explicit_admission_signature) != 1:
    raise SystemExit(
        "the explicit admission gate is not shared by cooperative and every StorageServer profile"
    )

# The shared feature exposes only the bounded physical engine. StorageServer's
# policy, broker ownership, timing windows, and commit ordering remain behind
# exactly one M58 coordinator; M59 cannot acquire it through the shared cfg.
if 'feature = "cooperative-block-recovery"' in coordinator:
    raise SystemExit("StorageServer coordinator leaked into the shared recovery cfg")
async_service_pattern = re.compile(
    r'#\[cfg\(all\(\s*feature = "storage-server-async-recovery-runtime",\s*'
    r'not\(feature = "storage-server-fault-policy-runtime"\)\s*\)\)\]\s*'
    r'pub fn service_recovery\(\)'
)
sync_service_pattern = re.compile(
    r'#\[cfg\(not\(feature = "storage-server-async-recovery-runtime"\)\)\]\s*'
    r'pub fn service_recovery\(\)'
)
if len(async_service_pattern.findall(coordinator)) != 1:
    raise SystemExit("M58 StorageServer async coordinator is not exact and unique")
if len(sync_service_pattern.findall(coordinator)) != 1:
    raise SystemExit("M56/M57 synchronous coordinator is not exact and unique")
for call in ("storage::begin_async_recovery", "storage::poll_async_recovery"):
    if coordinator_slice.count(call) != 1:
        raise SystemExit(f"M58 coordinator call is not exact and unique: {call}")


def executable_tokens(text: str) -> str:
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    return text


for label, implementation in (
    ("cooperative driver", driver_slice),
    ("storage async facade", storage_slice),
    ("M58 coordinator", coordinator_slice),
):
    code = executable_tokens(implementation)
    for forbidden in (
        "recover_after_timeout",
        "rebuild_after_confirmed_reset",
        "reset_and_wait",
        "stable_capacity",
        "cleanup_failed_recovery",
        "with_foreground_device",
    ):
        if re.search(rf"\b{forbidden}\s*\(", code):
            raise SystemExit(f"{label} calls the forbidden blocking path {forbidden}")
    if re.search(r"\b(?:loop|while)\b", code):
        raise SystemExit(f"{label} contains a loop/while polling token")

for needle in (
    "BlockRecoveryPhase::AwaitReset",
    "BlockRecoveryPhase::AwaitStableCapacity",
    "BlockRecoveryPhase::AwaitCleanupReset",
    "CooperativeRecoveryProgress::Pending",
    "observe_recovery_capacity_once()",
):
    if needle not in driver_slice:
        raise SystemExit(f"cooperative driver lost one-step phase evidence: {needle}")
for needle in (
    "save_and_mask_irq()",
    "timer::counter_value()",
    "ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS.fetch_max",
    "restore_daif(saved_daif)",
    "masked_poll_iterations: 0",
):
    if needle not in storage_slice:
        raise SystemExit(f"M58 storage facade lost bounded mask evidence: {needle}")

# Both acquisition and physical submission must stay closed until the final
# IRQ/broker/admission commit. An async driver alone is not an admission proof.
acquire_slice = syscall[
    syscall.index("fn storage_acquire"):
    syscall.index("fn storage_submit", syscall.index("fn storage_acquire"))
]
submit_slice = syscall[
    syscall.index("fn storage_submit"):
    syscall.index("fn storage_take", syscall.index("fn storage_submit"))
]
if "recovery_admission_closed()" not in acquire_slice:
    raise SystemExit("StorageAcquire bypasses the M58 admission gate")
if "recovery_admission_closed()" not in submit_slice:
    raise SystemExit("StorageSubmit bypasses the M58 admission gate")
for needle in (
    "broker.state != 4 || broker.bound",
    "storage::require_recovery();",
    "disable_block_irq_masked",
    "storage::begin_async_recovery",
    "storage::poll_async_recovery",
    "reenable_block_irq_masked",
    "storage_broker::complete_recovery",
    "storage::open_recovery_admission",
):
    if needle not in coordinator_slice:
        raise SystemExit(f"M58 coordinator lost fail-closed ordering: {needle}")

# Keep the runtime schema exact at the stable prefix and make the timing fields
# compulsory. Dynamic values are checked relationally by the runtime parser.
marker_prefix = (
    "STORAGE_SERVER_ASYNC_RECOVERY_OK cycles=2 cases=6 fault_order=WRFWRF "
    "read_requires_reset=2 mutation_outcome_unknown=4 control_sequences=6 "
    "injected_reads=2 injected_writes=2 injected_flushes=2 owner_exits=6 "
    "broker_releases=6 broker_abandoned=6 recovery_attempts=7 recovery_commits=6 "
    "recovery_rollbacks=1 injected_rearm_aborts=1 fail_closed_retries=1 "
    "driver_timeouts=6 driver_resets=7 async_starts=7 physical_completions=7"
)
if main.count(marker_prefix) != 1:
    raise SystemExit("M58 kernel marker lost its exact unique recovery prefix")
for field in (
    "async_steps={}",
    "recovery_yields={}",
    "timer_progress_windows={}",
    "worker_progress_windows={}",
    "el0_progress_windows={}",
    "acquire_waits={}",
    "acquire_dispatch_changes={}",
    "masked_poll_iterations={}",
    "max_step_masked_ticks={}",
    "max_control_masked_ticks={}",
    "timer_period_ticks={}",
):
    if field not in main:
        raise SystemExit(f"M58 marker lost dynamic evidence field {field}")

launch_pattern = re.compile(r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
launches = 0
for path in sorted((root / "scripts").glob("*.sh")):
    lines = path.read_text(encoding="utf-8").splitlines()
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
        if sum(bool(nic_none_pattern.match(line)) for line in block) != 1:
            raise SystemExit(f"QEMU launch must contain exactly one -nic none: {path}")
        index += 1
if launches == 0:
    raise SystemExit("M58 static contract found no QEMU launch blocks")

print(
    "STORAGE_SERVER_ASYNC_RECOVERY_STATIC_SOURCE_OK "
    f"sources={len(required)} qemu_launches={launches} nic_none={launches} "
    "cooperative_phases=4 admission_gate=1 masked_poll_iterations=0"
)
PY

# Exercise the real rejection path without reaching Cargo. Each invocation
# exits at the build-wrapper profile check, uses a disposable target directory,
# and remains offline.
mkdir -p "$WORKSPACE_ROOT/target"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m58-static.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

expect_profile_rejection() {
  local kernel_features="$1"
  local userspace_features="$2"
  local expected="$3"
  local output status
  set +e
  output="$(
    CARGO_TARGET_DIR="$TMP_DIR" \
      CARGO_NET_OFFLINE=true \
      BNDROID_KERNEL_FEATURES="$kernel_features" \
      BNDROID_USERSPACE_FEATURES="$userspace_features" \
      "$SCRIPT_DIR/build-kernel.sh" 2>&1
  )"
  status=$?
  set -e
  if [[ "$status" -ne 2 || "$output" != *"$expected"* ]]; then
    echo "$output" >&2
    echo "build-kernel did not reject the expected storage feature mismatch." >&2
    exit 1
  fi
}

expect_profile_rejection \
  "storage-server-repeated-recovery-runtime" \
  "storage-server-async-recovery-runtime" \
  "userspace async-recovery profile exceeds the kernel storage profile."
expect_profile_rejection \
  "storage-server-recovery-runtime" \
  "storage-server-repeated-recovery-runtime" \
  "userspace repeated-recovery profile exceeds the kernel storage profile."
expect_profile_rejection \
  "storage-server-runtime" \
  "storage-server-recovery-runtime" \
  "userspace recovery profile exceeds the kernel storage profile."
expect_profile_rejection \
  "" \
  "storage-server-runtime" \
  "userspace storage profile requires a storage-enabled kernel."

# This mode is deliberately parser-only: it neither builds nor launches QEMU.
"$SCRIPT_DIR/check-storage-server-async-recovery-runtime.sh" --parser-self-test

echo "STORAGE_SERVER_ASYNC_RECOVERY_STATIC_OK feature_mismatch_cases=4 parser_negative_cases=18 kernel_reset_authority=1"
