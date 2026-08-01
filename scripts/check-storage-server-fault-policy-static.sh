#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 mktemp mkdir rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M60 bounded fault-policy contracts." >&2
    exit 1
  fi
done

# Syntax validation is non-executing. In particular it cannot reach any QEMU
# launch contained in the scripts inspected below.
for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
feature = "storage-server-fault-policy-runtime"
parent = "storage-server-async-recovery-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M60 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(f"M60 static contract lost {description}: {relative}: {needle!r}")


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M60 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


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


def without_comments(text: str) -> str:
    text = re.sub(r"//[^\n]*", "", text)
    return re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)


def item_attributes(text: str, symbol: str) -> tuple[str, ...]:
    if text.count(symbol) != 1:
        raise SystemExit(f"M60 item is not exact and unique: {symbol}")
    lines = text[: text.index(symbol)].rstrip().splitlines()
    attributes: list[str] = []
    while lines and lines[-1].lstrip().startswith("#["):
        attributes.append(lines.pop().strip())
    return tuple(attributes)


def require_direct_m60_cfg(text: str, symbol: str, relative: str) -> None:
    expected = f'#[cfg(feature = "{feature}")]'
    attributes = item_attributes(text, symbol)
    profile_cfgs = tuple(attribute for attribute in attributes if attribute.startswith("#[cfg("))
    if profile_cfgs != (expected,):
        raise SystemExit(
            f"M60-only item has a missing or foreign cfg: {relative}:{symbol}:{profile_cfgs}"
        )


relative_sources = (
    "kernel/Cargo.toml",
    "user/init/Cargo.toml",
    "kernel/build.rs",
    "scripts/build-kernel.sh",
    "kernel/src/lib.rs",
    "kernel/src/storage_recovery_policy.rs",
    "kernel/src/storage_broker.rs",
    "kernel/src/driver/virtio/block.rs",
    "kernel/src/storage.rs",
    "kernel/src/storage_server_io.rs",
    "kernel/src/syscall.rs",
    "kernel/src/main.rs",
    "user/init/src/storage_server_runtime.rs",
    "scripts/check-storage-server-fault-policy-runtime.sh",
    "scripts/test.sh",
)
texts = {relative: source(relative) for relative in relative_sources}

# Cargo and both build layers must preserve the exact M60 -> M58 inheritance.
for relative in ("kernel/Cargo.toml", "user/init/Cargo.toml"):
    actual = feature_dependencies(texts[relative], feature, relative)
    if actual != (parent,):
        raise SystemExit(
            f"M60 Cargo feature no longer has exactly one M58 parent: "
            f"{relative}: {actual}"
        )

build_rs = texts["kernel/build.rs"]
require_once(
    build_rs,
    "cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
    "build.rs M60 rerun declaration",
    "kernel/build.rs",
)
require_once(
    build_rs,
    'feature=\\"storage-server-fault-policy-runtime\\"',
    "build.rs userspace M60 cfg forwarding",
    "kernel/build.rs",
)

wrapper = texts["scripts/build-kernel.sh"]
for variable in (
    "KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
    "USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME",
):
    require_once(wrapper, f"{variable}=0", f"zero initialization of {variable}", "scripts/build-kernel.sh")

closure_patterns = {
    "kernel M60->M58->M57->M56->M55 closure": (
        r'if feature_list_contains "\$KERNEL_FEATURES" "storage-server-fault-policy-runtime"; then\s+'
        r'KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'KERNEL_STORAGE_SERVER_RUNTIME=1'
    ),
    "userspace M60->M58->M57->M56->M55 closure": (
        r'if feature_list_contains "\$USERSPACE_FEATURES" "storage-server-fault-policy-runtime"; then\s+'
        r'USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RECOVERY_RUNTIME=1\s+'
        r'USER_STORAGE_SERVER_RUNTIME=1'
    ),
}
for description, pattern in closure_patterns.items():
    if re.search(pattern, wrapper) is None:
        raise SystemExit(f"build-kernel lost its exact {description}")
require_once(
    wrapper,
    'append_feature "$USERSPACE_FEATURES" "storage-server-fault-policy-runtime"',
    "M60 userspace feature mirroring",
    "scripts/build-kernel.sh",
)
if re.search(
    r'"\$USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME" -eq 1\s+\\\s*\n\s*'
    r'&& "\$KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME" -ne 1',
    wrapper,
) is None:
    raise SystemExit("build-kernel lost the userspace-above-kernel M60 rejection")

# The policy is a pure, allocation-free ticketed state machine. Physical
# success is probationary; only confirmed I/O clears the streak. Three
# unconfirmed attempts lead to a boot-local sticky Offline state.
lib_rs = texts["kernel/src/lib.rs"]
policy = texts["kernel/src/storage_recovery_policy.rs"]
require_once(lib_rs, "pub mod storage_recovery_policy;", "policy module export", "kernel/src/lib.rs")
for forbidden in (
    "crate::arch",
    "crate::driver",
    "crate::storage",
    "alloc::",
    "UnsafeCell",
    "spin_loop",
):
    if forbidden in policy:
        raise SystemExit(f"pure M60 policy acquired a kernel/device dependency: {forbidden}")
for needle, description in (
    ("pub const ATTEMPT_LIMIT: u8 = 3;", "attempt cap three"),
    ("pub const BACKOFF_MULTIPLIER: u64 = 2;", "backoff multiplier two"),
    ("pub struct AttemptTicket", "attempt ticket identity"),
    ("pub generation: u64", "non-reusable ticket generation"),
    ("pub ordinal: u8", "incident attempt ordinal"),
    ("Probation = 3", "Probation state"),
    ("Offline = 4", "Offline state"),
    ("RejectReason::BackoffPending", "early backoff rejection"),
    ("RejectReason::Offline", "Offline rejection"),
    ("self.last_attempt_generation.checked_add(1)", "non-wrapping ticket allocation"),
    ("if self.active_attempt != Some(ticket)", "stale ticket validation"),
    ("self.state = RecoveryState::Probation;", "physical-success probation entry"),
    ("self.state = RecoveryState::Healthy;", "probation health commit"),
    ("self.active_attempt = None;", "ticket retirement"),
    ("self.consecutive_failures = 0;", "confirmed-health streak reset"),
    ("if self.consecutive_failures == ATTEMPT_LIMIT", "exact attempt cap"),
    ("self.state = RecoveryState::Offline;", "Offline transition"),
    ("self.backoff_deadline = None;", "Offline deadline removal"),
    ("self.base_backoff_ticks * BACKOFF_MULTIPLIER", "second exponential delay"),
    ("deadline_reached(now, deadline)", "wrap-safe backoff deadline"),
):
    require(policy, needle, description, "kernel/src/storage_recovery_policy.rs")
for test in (
    "first_failure_enforces_base_backoff_without_issuing_an_early_ticket",
    "second_failure_uses_double_backoff_and_third_failure_is_offline",
    "physical_success_requires_real_io_success_to_clear_the_streak",
    "stale_ticket_cannot_complete_a_new_attempt",
    "duplicate_completion_is_rejected_transactionally",
    "backoff_deadline_comparison_is_wrap_safe",
    "exhausted_generation_never_aliases_an_old_ticket",
):
    require_once(policy, f"fn {test}()", f"policy host test {test}", "kernel/src/storage_recovery_policy.rs")
begin_slice = policy[
    policy.index("pub fn begin_attempt") : policy.index("pub fn complete_physical_success")
]
if begin_slice.index("RecoveryState::Offline") > begin_slice.index("state @ (RecoveryState::Recovering"):
    raise SystemExit("policy no longer rejects sticky Offline before invalid active states")
physical_success_slice = policy[
    policy.index("pub fn complete_physical_success") : policy.index("pub fn complete_physical_failure")
]
if "RecoveryState::Healthy" in physical_success_slice:
    raise SystemExit("physical recovery can bypass M60 Probation")
probation_success_slice = policy[
    policy.index("pub fn complete_probation_success") : policy.index("pub fn complete_probation_failure")
]
for needle in (
    "self.validate_ticket(ticket, RecoveryState::Probation)?",
    "self.state = RecoveryState::Healthy",
    "self.active_attempt = None",
    "self.consecutive_failures = 0",
):
    require(
        probation_success_slice,
        needle,
        "probation-only healthy commit",
        "kernel/src/storage_recovery_policy.rs",
    )

# Broker authority is permission-first. Legitimate StorageServers see exactly
# Unavailable once Offline; invalid callers cannot use acquire as a health
# oracle. Offline is committed only from ownerless state 4 and cannot reopen.
broker = texts["kernel/src/storage_broker.rs"]
syscall = texts["kernel/src/syscall.rs"]
for needle, description in (
    ("DeviceOffline", "broker DeviceOffline error"),
    ("pub device_offline: bool", "broker Offline snapshot"),
    ("pub offline_transitions: u64", "Offline transition ledger"),
    ("pub offline_acquire_denials: u64", "Offline denial ledger"),
    ("BlockState::RecoveryRequired => 4", "ownerless recovery state code four"),
    ("pub fn enter_device_offline", "ownerless Offline transition"),
    ("device_offline_is_sticky_permission_first_and_counted_exactly", "broker sticky/permission test"),
):
    require(broker, needle, description, "kernel/src/storage_broker.rs")
acquire_slice = broker[broker.index("pub fn acquire(") : broker.index("fn validates", broker.index("pub fn acquire("))]
permission = acquire_slice.index("owner_pid == 0 || image_id != UserImageId::StorageServer")
offline = acquire_slice.index("if self.device_offline")
binding = acquire_slice.index("if self.binding.is_some()")
if not permission < offline < binding:
    raise SystemExit("broker acquire no longer orders permission, Offline, then binding checks")
offline_slice = broker[
    broker.index("pub fn enter_device_offline") : broker.index("pub fn require_ownerless_recovery")
]
for needle in (
    "if self.device_offline",
    "self.binding.is_some()",
    "self.sessions.len != 0",
    "self.abandoned_running_token != 0",
    "BlockState::RecoveryRequired",
    "self.device_offline = true",
    "self.offline_transitions = self.offline_transitions.saturating_add(1)",
):
    require(offline_slice, needle, "fail-closed Offline broker guard", "kernel/src/storage_broker.rs")
complete_recovery_slice = broker[
    broker.index("pub fn complete_recovery") : broker.index("pub fn enter_device_offline")
]
require(complete_recovery_slice, "if self.device_offline", "Offline recovery-reopen rejection", "kernel/src/storage_broker.rs")

storage_acquire_slice = syscall[
    syscall.index("fn storage_acquire(") : syscall.index("fn storage_submit(", syscall.index("fn storage_acquire("))
]
for needle in (
    "if image_id != UserImageId::StorageServer",
    "storage_broker::snapshot().device_offline",
    "storage_broker::acquire(owner_pid, image_id)",
    "recovery_admission_closed()",
):
    require(storage_acquire_slice, needle, "permission-first StorageAcquire Offline path", "kernel/src/syscall.rs")
if not (
    storage_acquire_slice.index("if image_id != UserImageId::StorageServer")
    < storage_acquire_slice.index("storage_broker::snapshot().device_offline")
    < storage_acquire_slice.index("recovery_admission_closed()")
):
    raise SystemExit("StorageAcquire exposes Offline before authenticating StorageServer")
mapping_slice = syscall[
    syscall.index("fn storage_acquire_error_status") : syscall.index("fn storage_request_error_status")
]
require(
    mapping_slice,
    "AcquireError::DeviceOffline => Status::Unavailable",
    "DeviceOffline to Unavailable mapping",
    "kernel/src/syscall.rs",
)

connect_slice = broker[
    broker.index("pub fn connect(") : broker.index("pub fn accept(", broker.index("pub fn connect("))
]
for needle in (
    "principal_for_image(image_id).ok_or(SessionError::PermissionDenied)?",
    "if client_pid == 0",
    "granted_rights.bits()",
    "STORAGE_CONNECT_RIGHTS_MASK",
    "if matches!(&self.state, BlockState::RecoveryRequired)",
    "let Some(binding) = self.binding",
):
    require(connect_slice, needle, "permission-first StorageConnect oracle seal", "kernel/src/storage_broker.rs")
if not (
    connect_slice.index("principal_for_image(image_id)")
    < connect_slice.index("if client_pid == 0")
    < connect_slice.index("granted_rights.bits()")
    < connect_slice.index("BlockState::RecoveryRequired")
    < connect_slice.index("let Some(binding) = self.binding")
):
    raise SystemExit("StorageConnect exposes recovery/binding health before authenticating authority")
require_once(
    broker,
    "fn connect_authentication_precedes_healthy_recovery_and_offline_state()",
    "StorageConnect healthy/recovery/Offline oracle host test",
    "kernel/src/storage_broker.rs",
)

# M60's permanent campaign is kernel-prearmed. Recovery attempt seven validates
# the ownerless broker/policy and six inherited WRFWRF ledgers, then invokes one
# combined driver hook before committing the broker acquirable. The seventh
# owner contributes only an ordinary one-sector observation. The older six
# transient incidents still use their M56 EL0 prefix/selector controls, but no
# EL0 permanent selector or syscall can arm either half of the combined hook.
driver = texts["kernel/src/driver/virtio/block.rs"]
storage = texts["kernel/src/storage.rs"]
coordinator = texts["kernel/src/storage_server_io.rs"]
for text, symbol, relative in (
    (driver, "pub struct PersistentRebuildFaultSnapshot", "kernel/src/driver/virtio/block.rs"),
    (driver, "pub struct TerminalDmaSnapshot", "kernel/src/driver/virtio/block.rs"),
    (driver, "pub(crate) fn arm_m60_permanent_read_fault_for_test", "kernel/src/driver/virtio/block.rs"),
    (driver, "pub const fn persistent_rebuild_fault_snapshot", "kernel/src/driver/virtio/block.rs"),
    (driver, "pub fn terminal_dma_snapshot", "kernel/src/driver/virtio/block.rs"),
    (storage, "pub(crate) fn arm_m60_permanent_read_fault_for_test", "kernel/src/storage.rs"),
    (storage, "pub fn persistent_rebuild_fault_snapshot", "kernel/src/storage.rs"),
    (storage, "pub fn terminal_dma_snapshot", "kernel/src/storage.rs"),
    (coordinator, "fn observe_m60_kernel_permanent_request(", "kernel/src/storage_server_io.rs"),
    (coordinator, "fn prearm_m60_kernel_permanent_campaign(", "kernel/src/storage_server_io.rs"),
    (coordinator, "fn record_fault_policy_io_result(", "kernel/src/storage_server_io.rs"),
    (coordinator, "fn publish_verified_terminal_offline(", "kernel/src/storage_server_io.rs"),
    (coordinator, "fn publish_device_offline_if_ownerless(", "kernel/src/storage_server_io.rs"),
):
    require_direct_m60_cfg(text, symbol, relative)
arm_counts = tuple(
    text.count("arm_m60_permanent_read_fault_for_test")
    for text in (driver, storage, coordinator)
)
if arm_counts != (1, 2, 1):
    raise SystemExit("M60 combined permanent-fault path is not exactly driver->storage->coordinator")
for relative in ("kernel/src/syscall.rs", "kernel/src/main.rs", "user/init/src/storage_server_runtime.rs"):
    if "arm_m60_permanent_read_fault_for_test" in texts[relative]:
        raise SystemExit(f"EL0/proof layer gained direct combined fault-arm authority: {relative}")
for relative, text in texts.items():
    if "arm_persistent_rebuild_fault_for_test" in text:
        raise SystemExit(f"obsolete split persistent-fault hook survived: {relative}")
for needle in (
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n    InjectedPersistentRebuildFailure,',
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n    persistent_rebuild_fault_armed: bool,',
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n    persistent_rebuild_fault_hits: u64,',
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n        persistent_rebuild_fault_armed: false,',
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n        persistent_rebuild_fault_hits: 0,',
):
    require(driver, needle, "M60-only persistent driver state", "kernel/src/driver/virtio/block.rs")
driver_arm = driver[
    driver.index("pub(crate) fn arm_m60_permanent_read_fault_for_test") : driver.index(
        "pub const fn persistent_rebuild_fault_snapshot",
        driver.index("pub(crate) fn arm_m60_permanent_read_fault_for_test"),
    )
]
for needle in (
    "self.state != DriverState::Ready",
    "self.persistent_rebuild_fault_armed",
    "self.requests.in_flight() != 0",
    "self.recovery_notification_fault.is_armed()",
    ".arm(BlockRequestKind::Read, 0)",
    "self.persistent_rebuild_fault_armed = true",
):
    require(driver_arm, needle, "atomic ordinary-read plus rebuild-fault arm", "kernel/src/driver/virtio/block.rs")
if driver_arm.index(".arm(BlockRequestKind::Read, 0)") > driver_arm.index(
    "self.persistent_rebuild_fault_armed = true"
):
    raise SystemExit("M60 combined hook sets persistence before its read-loss arm succeeds")
rebuild_slice = driver[
    driver.index("BlockRecoveryPhase::RebuildQueue =>") : driver.index("BlockRecoveryPhase::AwaitStableCapacity =>")
]
for needle in (
    "self.requests.invalidate_all()",
    "self.stats.avail_idx = 0",
    "self.stats.used_idx = 0",
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\n                if self.persistent_rebuild_fault_armed',
    "self.persistent_rebuild_fault_hits.saturating_add(1)",
    "self.begin_cooperative_cleanup(",
    "BlockError::InjectedPersistentRebuildFailure",
):
    require(rebuild_slice, needle, "persistent RebuildQueue cleanup injection", "kernel/src/driver/virtio/block.rs")
cleanup_slice = driver[
    driver.index("BlockRecoveryPhase::AwaitCleanupReset =>") : driver.index("pub const fn cooperative_recovery_phase")
]
for needle in (
    "self.record_confirmed_reset()",
    "self.cooperative_recovery_cause",
    ".confirm_cleanup()",
    "return Err(cause)",
):
    require(cleanup_slice, needle, "cleanup-reset failure return", "kernel/src/driver/virtio/block.rs")

prepare_recovery = coordinator[
    coordinator.index("fn prepare_recovery_fault(") : coordinator.index(
        "fn observe_m60_kernel_permanent_request(", coordinator.index("fn prepare_recovery_fault(")
    )
]
if prepare_recovery.index("observe_m60_kernel_permanent_request(request, epoch)") > prepare_recovery.index(
    "if epoch == 0"
):
    raise SystemExit("M60 permanent request observer moved behind inherited EL0 control parsing")
for needle in (
    "const PREFIX_LBA:",
    "const WRITE_SELECTOR_LBA:",
    "const READ_SELECTOR_LBA:",
    "const FLUSH_SELECTOR_LBA:",
    "FAULT_CONTROL_SEQUENCES.fetch_add(1",
    "storage::suppress_next_notification_for_recovery_test(expected_kind)",
):
    require(prepare_recovery, needle, "six inherited transient EL0 controls", "kernel/src/storage_server_io.rs")

request_observer = coordinator[
    coordinator.index("fn observe_m60_kernel_permanent_request(") : coordinator.index(
        "fn prearm_m60_kernel_permanent_campaign(",
        coordinator.index("fn observe_m60_kernel_permanent_request("),
    )
]
request_observer_code = without_comments(request_observer)
for needle in (
    "const PERMANENT_OWNER_EPOCH: u64 = 7;",
    "if epoch != PERMANENT_OWNER_EPOCH",
    "broker.bound",
    "broker.epoch == PERMANENT_OWNER_EPOCH",
    "broker.state == 2",
    "policy.state == RecoveryState::Probation",
    "policy.active_attempt_generation == 7",
    "policy.attempts_started == 7",
    "FAULT_CONTROL_SEQUENCES.load(Ordering::Acquire) == 6",
    "FAULT_CONTROL_STATE.load(Ordering::Acquire) == 0",
    "FAULT_CONTROL_EPOCH.load(Ordering::Acquire) == 0",
    "KERNEL_PERMANENT_FAULT_ARMS.load(Ordering::Acquire) == 1",
    "KERNEL_PERMANENT_OWNER_REQUESTS.fetch_add(1",
    "request.operation() != StorageBlockOperation::Read",
    "request.sector_count() != 1",
    "storage::require_recovery();",
    "KERNEL_PERMANENT_READS.store(1",
):
    require(request_observer, needle, "non-authoritative epoch-seven request observation", "kernel/src/storage_server_io.rs")
for forbidden in (
    "relative_lba",
    "PREFIX_LBA",
    "SELECTOR_LBA",
    "recovery_fault_control",
    "arm_m60_permanent_read_fault_for_test",
    "KERNEL_PERMANENT_FAULT_ARMS.store",
    "panic!",
):
    if forbidden in request_observer_code:
        raise SystemExit(f"M60 permanent request observer regained fault authority: {forbidden}")
if request_observer_code.count("return false;") != 1 or request_observer_code.count("return true;") != 1:
    raise SystemExit("M60 epoch-seven observer can fall into the inherited magic parser")
if request_observer.index("storage::require_recovery();") > request_observer.index("return true;"):
    raise SystemExit("M60 malformed permanent request bypasses fail-closed recovery")

kernel_prearm = coordinator[
    coordinator.index("fn prearm_m60_kernel_permanent_campaign(") : coordinator.index(
        "/// Completes one fail-stop recovery attempt",
        coordinator.index("fn prearm_m60_kernel_permanent_campaign("),
    )
]
kernel_prearm_code = without_comments(kernel_prearm)
for needle in (
    "const PERMANENT_RECOVERY_ATTEMPT: u64 = 7;",
    "if recovery_attempt != PERMANENT_RECOVERY_ATTEMPT",
    "!broker.bound",
    "broker.epoch == 0",
    "broker.next_epoch == 7",
    "broker.state == 4",
    "policy.state == RecoveryState::Probation",
    "policy.active_attempt_generation == 7",
    "policy.attempts_started == 7",
    "FAULT_CONTROL_SEQUENCES.load(Ordering::Acquire) == 6",
    "FAULT_CONTROL_STATE.load(Ordering::Acquire) == 0",
    "FAULT_CONTROL_EPOCH.load(Ordering::Acquire) == 0",
    "KERNEL_PERMANENT_OWNER_REQUESTS.load(Ordering::Acquire) == 0",
    "KERNEL_PERMANENT_FAULT_ARMS.load(Ordering::Acquire) == 0",
    "KERNEL_PERMANENT_READS.load(Ordering::Acquire) == 0",
    "storage::arm_m60_permanent_read_fault_for_test()",
    "KERNEL_PERMANENT_FAULT_ARMS.store(1",
):
    require(kernel_prearm, needle, "ownerless recovery-attempt-seven permanent prearm", "kernel/src/storage_server_io.rs")
for forbidden in ("StorageBlockRequest", "relative_lba", "PREFIX_LBA", "SELECTOR_LBA"):
    if forbidden in kernel_prearm_code:
        raise SystemExit(f"M60 kernel prearm depends on an EL0 request selector: {forbidden}")
if coordinator.count("storage::arm_m60_permanent_read_fault_for_test()") != 1:
    raise SystemExit("M60 coordinator lost the unique kernel prearm hook call")

# A failed real I/O while Probation is active closes recovery admission before
# its ABI classification is sampled, consumes the active probation ticket, and
# is surfaced as RequiresReset (except the stronger mutation OutcomeUnknown).
probation_result = coordinator[
    coordinator.index("fn record_fault_policy_io_result(") : coordinator.index(
        "/// Executes at most one accepted", coordinator.index("fn record_fault_policy_io_result(")
    )
]
for needle in (
    "if snapshot.state != RecoveryState::Probation",
    "storage::require_recovery();",
    "PROBATION_IO_FAILURES.fetch_add(1",
    "policy.complete_probation_failure(ticket, now)",
    "record_policy_disposition(disposition);",
    "true",
):
    require(probation_result, needle, "Probation I/O failure convergence", "kernel/src/storage_server_io.rs")
service_pending = coordinator[
    coordinator.index("pub fn service_pending()") : coordinator.index(
        "pub fn record_async_acquire_wait", coordinator.index("pub fn service_pending()")
    )
]
probation_hook = service_pending.index("record_fault_policy_io_result(result.is_ok())")
recovery_sample = service_pending.index("let recovery_required = storage::recovery_required()")
outcome_unknown = service_pending.index("Err(StorageError::SubmittedMutationOutcomeUnknown)")
generic_reset = service_pending.index("Err(_) if probation_failed || recovery_required")
generic_unavailable = service_pending.index("Err(_) =>", generic_reset)
if not probation_hook < recovery_sample < outcome_unknown < generic_reset < generic_unavailable:
    raise SystemExit("Probation I/O failure no longer converges before ABI reset classification")

# Offline publication is unified behind one verifier. It disables/rolls back
# the registered IRQ route, audits the admission/coordinator state and terminal
# request/DMA snapshot in the same masked window, and only then enters sticky
# broker Offline. If that proof is initially unsafe, an independent terminal
# quarantine finishes cooperative cleanup without consuming a policy ticket.
terminal_snapshot = driver[
    driver.index("pub fn terminal_dma_snapshot") : driver.index(
        "pub const fn in_flight", driver.index("pub fn terminal_dma_snapshot")
    )
]
for needle in (
    "let in_flight = self.requests.in_flight();",
    "let recovery_active = self.cooperative_recovery.is_active();",
    "self.cooperative_recovery_cause.is_none()",
    "self.cooperative_recovered_features.is_none()",
    "driver_state == TERMINAL_DRIVER_READY",
    "transport_status & STATUS_DRIVER_OK != 0",
    "transport_status & (STATUS_DEVICE_NEEDS_RESET | STATUS_FAILED) == 0",
    "driver_state == TERMINAL_DRIVER_RESET_CONFIRMED && transport_status == 0",
    "requests_terminal: in_flight == 0",
    "&& !recovery_active",
    "&& recovery_scratch_clear",
):
    require(terminal_snapshot, needle, "terminal request/DMA snapshot", "kernel/src/driver/virtio/block.rs")

terminal_publisher = coordinator[
    coordinator.index("fn publish_verified_terminal_offline(") : coordinator.index(
        "fn finish_terminal_quarantine(", coordinator.index("fn publish_verified_terminal_offline(")
    )
]
for needle in (
    "storage::require_recovery();",
    "save_and_mask_irq()",
    "registered_block_irq()",
    "rollback_block_irq_rearm_masked(block_irq)",
    "TERMINAL_IRQ_ROLLBACKS.fetch_add(1",
    "let irq = storage::irq_snapshot();",
    "let dma = storage::terminal_dma_snapshot()",
    "!irq.armed",
    "!irq.rearm_prepared",
    "irq.failed",
    "irq.recovery_required",
    "storage::recovery_admission_closed()",
    "!storage::async_recovery_active()",
    "!ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire)",
    "dma.requests_terminal",
    "with_broker_irq_masked(storage_broker::enter_device_offline)",
    "TERMINAL_DMA_VERIFICATIONS.fetch_add(1",
):
    require(terminal_publisher, needle, "verified terminal Offline publisher", "kernel/src/storage_server_io.rs")
rollback = terminal_publisher.index("rollback_block_irq_rearm_masked(block_irq)")
dma_audit = terminal_publisher.index("let dma = storage::terminal_dma_snapshot()")
safe_gate = terminal_publisher.index("let safe =")
offline_commit = terminal_publisher.index("storage_broker::enter_device_offline")
if not rollback < dma_audit < safe_gate < offline_commit:
    raise SystemExit("M60 terminal Offline publication lost rollback -> DMA audit -> safe gate -> broker order")
if coordinator.count("storage_broker::enter_device_offline") != 1:
    raise SystemExit("M60 coordinator gained an Offline publication path outside its verifier")

terminal_quarantine = coordinator[
    coordinator.index("fn finish_terminal_quarantine(") : coordinator.index(
        "fn finish_fault_policy_attempt_failure(", coordinator.index("fn finish_terminal_quarantine(")
    )
]
for needle in (
    "TERMINAL_QUARANTINE_ACTIVE",
    "TERMINAL_QUARANTINE_STARTS.fetch_add(1",
    "TERMINAL_QUARANTINE_COMPLETIONS.fetch_add(1",
    "TERMINAL_QUARANTINE_PHYSICAL_ERRORS.fetch_add(1",
    "storage::begin_async_recovery",
    "storage::poll_async_recovery",
    "publish_verified_terminal_offline()",
):
    require(terminal_quarantine, needle, "independent terminal quarantine", "kernel/src/storage_server_io.rs")
for forbidden in ("policy.begin_attempt", "complete_physical_failure", "RECOVERY_ATTEMPTS.fetch_add"):
    if forbidden in without_comments(terminal_quarantine):
        raise SystemExit(f"terminal quarantine incorrectly consumes a policy attempt: {forbidden}")

# M60 wraps M58's cooperative engine with a short, non-busy policy monitor.
# Backoff returns to the scheduler, Offline never starts another ticket, and a
# successful commit is rearm -> admission-open -> ownerless broker commit.
m60_service_pattern = re.compile(
    r'#\[cfg\(feature = "storage-server-fault-policy-runtime"\)\]\s*'
    r'pub fn service_recovery\(\)'
)
if len(m60_service_pattern.findall(coordinator)) != 1:
    raise SystemExit("M60 fault-policy coordinator is not exact and uniquely cfg-gated")
m60_slice = coordinator[
    coordinator.index("/// M60 adds a boot-local bounded policy") : coordinator.index(
        "fn record_async_control_masked_since",
        coordinator.index("/// M60 adds a boot-local bounded policy"),
    )
]
m60_code = without_comments(m60_slice)
for forbidden in (
    r"\bloop\b",
    r"\bwhile\b",
    r"\bspin_loop\s*\(",
    r"\brecover_after_timeout\s*\(",
    r"\breset_and_wait\s*\(",
    r"\bstable_capacity\s*\(",
):
    if re.search(forbidden, m60_code):
        raise SystemExit(f"M60 coordinator contains a busy/blocking recovery path: {forbidden}")
for needle in (
    "const FAULT_POLICY_BACKOFF_BASE_TICKS: u64 = 2;",
    "RecoveryPolicy::new(\n    FAULT_POLICY_BACKOFF_BASE_TICKS",
    "if policy.state == RecoveryState::Offline",
    "publish_device_offline_if_ownerless();\n        return;",
    "Err(RejectReason::BackoffPending { .. }) => return",
    "policy.begin_attempt(now)",
    "let deadline_reached = bndroid_kernel::time::deadline_reached(now, deadline);",
    "finish_backoff_progress_window();",
    "policy.complete_physical_success(ticket)",
    "policy.complete_physical_failure(ticket, now)",
    "policy.complete_probation_failure(ticket, now)",
    "BACKOFF_TIMER_PROGRESS_WINDOWS",
    "BACKOFF_WORKER_PROGRESS_WINDOWS",
):
    require(coordinator, needle, "bounded fault-policy coordinator evidence", "kernel/src/storage_server_io.rs")
for needle in (
    "scheduler.timer_dispatches > BACKOFF_BASE_TIMER_DISPATCHES.load(Ordering::Acquire)",
    "scheduler.worker_work[0] > BACKOFF_BASE_WORKER_0.load(Ordering::Acquire)",
    "scheduler.worker_work[1] > BACKOFF_BASE_WORKER_1.load(Ordering::Acquire)",
    "M60 backoff completed before timer and both workers made progress",
):
    require(
        coordinator,
        needle,
        "deterministic timer-and-worker backoff witness",
        "kernel/src/storage_server_io.rs",
    )
offline_guard = m60_slice.index("if policy.state == RecoveryState::Offline")
attempt_begin = m60_slice.index("policy.begin_attempt(now)")
if offline_guard >= attempt_begin:
    raise SystemExit("M60 Offline guard moved after recovery ticket issuance")
rearm = m60_slice.index("reenable_block_irq_masked")
admission = m60_slice.index("storage::open_recovery_admission", rearm)
prearm = m60_slice.index("prearm_m60_kernel_permanent_campaign(recovery_attempt)", admission)
broker_commit = m60_slice.index("storage_broker::complete_recovery", prearm)
masked = m60_slice.rfind("save_and_mask_irq()", 0, rearm)
rollback = m60_slice.index("rollback_block_irq_rearm_masked(block_irq)", broker_commit)
success_branch = m60_slice.index("if broker_committed {", rollback)
retry_retire = m60_slice.index(
    "RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel)", success_branch
)
retry_publish = m60_slice.index(
    "RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed)", retry_retire
)
success_publish = m60_slice.index(
    "RECOVERY_SUCCESSES.fetch_add(1, Ordering::Release)", retry_publish
)
unmasked = m60_slice.index("restore_daif(saved_daif)", rollback)
if not (
    masked
    < rearm
    < admission
    < prearm
    < broker_commit
    < rollback
    < success_branch
    < retry_retire
    < retry_publish
    < success_publish
    < unmasked
):
    raise SystemExit(
        "M60 commit no longer orders rearm -> admission -> prearm -> broker -> "
        "success ledgers -> DAIF restore"
    )
for needle in (
    "let permanent_campaign_armed =\n        admission_opened && prearm_m60_kernel_permanent_campaign(recovery_attempt);",
    "let broker_committed = permanent_campaign_armed",
    "if !broker_committed\n        && let Err(error) = crate::interrupt::rollback_block_irq_rearm_masked(block_irq)",
    "if broker_committed {",
    "RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel)",
    "RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed)",
    "RECOVERY_SUCCESSES.fetch_add(1, Ordering::Release)",
):
    require(m60_slice, needle, "fail-closed prearm commit window", "kernel/src/storage_server_io.rs")
for unique in (
    "if broker_committed {",
    "RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel)",
    "RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed)",
    "RECOVERY_SUCCESSES.fetch_add(1, Ordering::Release)",
):
    if m60_slice.count(unique) != 1:
        raise SystemExit(f"M60 success ledger publication is not unique: {unique}")
if "arm_m60_permanent_read_fault_for_test" in m60_slice:
    raise SystemExit("M60 recovery commit bypasses its sealed prearm helper")
for needle in (
    "rollback_block_irq_rearm_masked",
    "storage::require_recovery()",
    "broker.state != 4",
    "broker.bound",
    "ASYNC_COORDINATOR_ACTIVE.store(false",
):
    require(m60_slice, needle, "M60 fail-closed coordinator invariant", "kernel/src/storage_server_io.rs")

# Userspace drives six transient WRFWRF controls, then asks a seventh owner to
# perform an ordinary read and starts exactly one terminal Offline probe. The
# permanent-stage number is orchestration only: the server sends no permanent
# LBA selector and exposes no driver-control syscall.
userspace = texts["user/init/src/storage_server_runtime.rs"]
m60_init_start = userspace.index(
    f'''#[cfg(all(
    feature = "{feature}",
    not(feature = "storage-server-shutdown-orchestration-runtime")
))]
pub(super) fn init_runtime'''
)
m60_init = userspace[
    m60_init_start : userspace.index(
        '#[cfg(feature = "storage-server-recovery-runtime")]\nfn init_recovery_runtime',
        m60_init_start,
    )
]
for needle in (
    "RECOVERY_STAGE_PERMANENT_READ",
    "RECOVERY_RESULT_REQUIRES_RESET",
    "let permanent = spawn(UserImageId::StorageServer);",
    "let probe = spawn(UserImageId::StorageServer);",
    "SERVER_OFFLINE_RESULT_TAG",
    "SERVER_OFFLINE_EXIT_CODE",
    "M60_STORAGE_READY_PREFIX | generation",
    "M60_STORAGE_PROOF",
):
    require(m60_init, needle, "M60 permanent-read/offline campaign", "user/init/src/storage_server_runtime.rs")
if m60_init.count("let probe = spawn(UserImageId::StorageServer);") != 1:
    raise SystemExit("M60 userspace no longer has exactly one Offline probe")
if "expect_server_ready(permanent)" in m60_init:
    raise SystemExit("M60 permanent owner mounted AppData before its ordinary first read")
sequence_match = re.search(r"let sequence = \[(.*?)\];", m60_init, flags=re.DOTALL)
if sequence_match is None:
    raise SystemExit("M60 userspace lost its recoverable campaign array")
sequence = re.findall(r"RECOVERY_STAGE_(WRITE|READ|FLUSH)", sequence_match.group(1))
if sequence != ["WRITE", "READ", "FLUSH", "WRITE", "READ", "FLUSH"]:
    raise SystemExit(f"M60 recoverable campaign is not exact WRFWRF: {sequence}")
server_slice = userspace[
    userspace.index("pub(super) fn server_runtime") : userspace.index(
        "fn acquire_storage_volume", userspace.index("pub(super) fn server_runtime")
    )
]
for needle in (
    "let recovery_stage = read_recovery_stage_command(startup);",
    "acquired.status == KernelStatus::Unavailable.raw()",
    "acquired.out1 == 0",
    "acquired.out2 == 0",
    "recovery_stage != RECOVERY_STAGE_OFFLINE_PROBE",
    "write_scalar(startup, SERVER_OFFLINE_RESULT_TAG, 0)",
    "exit_child(SERVER_OFFLINE_EXIT_CODE)",
    "if recovery_stage == RECOVERY_STAGE_PERMANENT_READ",
    "run_m60_permanent_read(startup, &mut io);",
    "let volume = AppDataVolume::new(0);",
):
    require(server_slice, needle, "M60 staged server/Offline behavior", "user/init/src/storage_server_runtime.rs")
if not (
    server_slice.index("read_recovery_stage_command(startup)")
    < server_slice.index("acquire_storage_volume()")
    < server_slice.index("run_m60_permanent_read(startup, &mut io)")
    < server_slice.index("let volume = AppDataVolume::new(0)")
):
    raise SystemExit("M60 permanent ordinary read is no longer the first post-acquire block request")
acquire_retry_start = userspace.index(
    '#[cfg(feature = "storage-server-recovery-runtime")]\nfn acquire_storage_volume'
)
acquire_retry = userspace[
    acquire_retry_start : userspace.index("fn read_recovery_stage_command", acquire_retry_start)
]
for needle in (
    "KernelStatus::ShouldWait.raw()",
    "asm!(\"wfi\"",
    "continue;",
    "return acquired;",
):
    require(acquire_retry, needle, "scheduler-yielding acquire retry", "user/init/src/storage_server_runtime.rs")
mounted_stage = userspace[
    userspace.index("fn run_mounted_recovery_stage") : userspace.index(
        "fn run_m60_permanent_read", userspace.index("fn run_mounted_recovery_stage")
    )
]
for needle in (
    "CONTROL_PREFIX_LBA",
    "CONTROL_WRITE_LBA",
    "CONTROL_READ_LBA",
    "CONTROL_FLUSH_LBA",
    "recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_WRITE_LBA)",
    "recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_READ_LBA)",
    "recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_FLUSH_LBA)",
    "Err(IoError::RequiresReset) => RECOVERY_RESULT_REQUIRES_RESET",
):
    require(mounted_stage, needle, "six inherited transient EL0 controls", "user/init/src/storage_server_runtime.rs")
if "RECOVERY_STAGE_PERMANENT_READ" in without_comments(mounted_stage):
    raise SystemExit("M60 permanent stage re-entered the mounted transient-control path")
permanent_read = userspace[
    userspace.index("fn run_m60_permanent_read") : userspace.index(
        "fn recovery_fault_control", userspace.index("fn run_m60_permanent_read")
    )
]
for needle in (
    "recovery_raw_read(io, RECOVERY_TEST_LBA)",
    "Err(IoError::RequiresReset) => RECOVERY_RESULT_REQUIRES_RESET",
    "RECOVERY_STAGE_PERMANENT_READ << 32",
    "exit_child(SERVER_RECOVERY_EXIT_CODE)",
):
    require(permanent_read, needle, "ordinary permanent-owner read", "user/init/src/storage_server_runtime.rs")
for forbidden in ("recovery_fault_control", "CONTROL_PREFIX_LBA", "SELECTOR_LBA"):
    if forbidden in without_comments(permanent_read):
        raise SystemExit(f"M60 permanent userspace read regained control authority: {forbidden}")
for forbidden in ("CONTROL_PERMANENT_READ_LBA", "PERMANENT_SELECTOR_LBA"):
    if forbidden in userspace:
        raise SystemExit(f"M60 userspace retained an EL0 permanent selector: {forbidden}")

# InitReady has a syscall-local seal, while main independently re-reads and
# checks the ledgers and observes a quiet post-Offline window.
main = texts["kernel/src/main.rs"]
require_direct_m60_cfg(syscall, "fn m60_storage_ready_proof_valid()", "kernel/src/syscall.rs")
require_direct_m60_cfg(main, "fn validate_storage_server_fault_policy_runtime()", "kernel/src/main.rs")
syscall_proof = syscall[
    syscall.index("fn m60_storage_ready_proof_valid") : syscall.index(
        "fn complete(", syscall.index("fn m60_storage_ready_proof_valid")
    )
]
main_proof = main[
    main.index("fn validate_storage_server_fault_policy_runtime") : main.index(
        '#[cfg(feature = "service-dependency-runtime")]',
        main.index("fn validate_storage_server_fault_policy_runtime"),
    )
]
for label, proof, relative in (
    ("syscall-local", syscall_proof, "kernel/src/syscall.rs"),
    ("main independent", main_proof, "kernel/src/main.rs"),
):
    for needle in (
        "fault_policy_snapshot()",
        "persistent_rebuild_fault_snapshot()",
        "terminal_dma_snapshot()",
        "broker.state == 4",
        "broker.device_offline",
        "broker.offline_transitions == 1",
        "broker.offline_acquire_denials == 1",
        "policy.state == RecoveryState::Offline",
        "policy.attempt_limit == 3",
        "policy.base_backoff_ticks == 2",
        "policy.backoff_multiplier == 2",
        "policy.backoffs_scheduled == 3",
        "policy.backoffs_completed == 3",
        "policy.total_backoff_ticks == 8",
        "policy.attempts_started == 9",
        "policy.physical_successes == 7",
        "policy.physical_failures == 2",
        "policy.attempt_failures == 4",
        "policy.probation_entries == 7",
        "policy.probation_successes == 5",
        "policy.probation_failures == 2",
        "policy.healthy_transitions == 5",
        "policy.stale_ticket_rejections == 0",
        "io.fault_control_sequences == 6",
        "io.kernel_permanent_owner_requests == 1",
        "io.kernel_permanent_fault_arms == 1",
        "io.kernel_permanent_reads == 1",
        "io.probation_io_failures == 1",
        "io.recovery_attempts == 9",
        "io.recovery_successes == 6",
        "io.recovery_failures == 3",
        "io.async_recovery_starts == 9",
        "io.async_physical_completions == 7",
        "io.async_physical_failures == 2",
        "!io.terminal_quarantine_active",
        "io.terminal_quarantine_starts == 0",
        "io.terminal_quarantine_completions == 0",
        "io.terminal_quarantine_physical_errors == 0",
        "io.terminal_irq_rollbacks == 1",
        "io.terminal_dma_verifications == 1",
        "terminal.driver_state == 2",
        "terminal.transport_status == 0",
        "terminal.in_flight == 0",
        "!terminal.recovery_active",
        "terminal.recovery_scratch_clear",
        "terminal.requests_terminal",
        "recovery.masked_poll_iterations == 0",
        "irq.failed",
        "!irq.armed",
        "!irq.rearm_prepared",
        "irq.recovery_required",
        "recovery_admission_closed()",
    ):
        require(proof, needle, f"{label} M60 proof field", relative)
    if re.search(
        r'persistent\.hits\s*==\s*if cfg!\(feature = '
        r'"storage-server-terminal-quarantine-runtime"\)\s*\{\s*3\s*\}\s*else\s*\{\s*2\s*\}',
        proof,
    ) is None:
        raise SystemExit(
            f"M60 static contract lost the historical hits=2 branch or its isolated "
            f"M62 hits=3 child branch: {relative}"
        )
if "m60_storage_ready_proof_valid" in main_proof:
    raise SystemExit("main M60 validator delegates to the syscall proof instead of checking independently")
for proof, relative in (
    (syscall_proof, "kernel/src/syscall.rs"),
    (main_proof, "kernel/src/main.rs"),
):
    if "policy.early_attempt_rejections != 0" in proof:
        raise SystemExit(
            f"M60 proof incorrectly requires an observable pre-deadline monitor poll: {relative}"
        )
for needle in (
    "OFFLINE_QUIET_TICKS",
    "offline_baseline",
    "io.recovery_attempts != attempts",
    "block.resets != resets",
    "broker.submissions != submissions",
    "post_offline_attempt_delta=0",
    "post_offline_reset_delta=0",
    "post_offline_submission_delta=0",
    "io.backoff_timer_progress_windows == 3",
    "io.backoff_worker_progress_windows == 3",
    "block.timeouts == 7",
    "block.requests == block.completions.saturating_add(7)",
    "io.async_timer_progress_windows >= 7",
    "io.async_timer_progress_windows <= 9",
    "io.async_acquire_waits >= 18",
    "early_rejections={} early_physical_starts=0",
    "simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0",
):
    require(main_proof, needle, "independent quiet/offline claim", "kernel/src/main.rs")
if re.search(
    r'block\.resets\s*==\s*if cfg!\(feature = '
    r'"storage-server-terminal-quarantine-runtime"\)\s*\{\s*11\s*\}\s*else\s*\{\s*10\s*\}',
    main_proof,
) is None:
    raise SystemExit(
        "M60 static contract lost the historical resets=10 branch or its isolated "
        "M62 resets=11 child branch: kernel/src/main.rs"
    )
marker_literals = re.findall(r'"(STORAGE_SERVER_FAULT_POLICY_OK [^"\n]+)"', main)
if len(marker_literals) != 2:
    raise SystemExit(
        f"M60 kernel evidence marker plus its isolated M62 child are not exact: "
        f"{len(marker_literals)}"
    )
historical_markers = [
    marker
    for marker in marker_literals
    if "persistent_fault_hits=2 driver_timeouts=7 driver_resets=10" in marker
    and "terminal_quarantine_starts=0" in marker
]
child_markers = [
    marker
    for marker in marker_literals
    if "persistent_fault_hits=3 driver_timeouts=7 driver_resets=11" in marker
    and "terminal_quarantine_starts=1" in marker
]
if len(historical_markers) != 1 or len(child_markers) != 1:
    raise SystemExit(
        "M60 historical evidence or the isolated M62 child evidence is not exact and unique"
    )
marker_literal = historical_markers[0]
for needle in (
    "recoverable_cases=6 permanent_cases=1 fault_order=WRFWRFR read_requires_reset=3 mutation_outcome_unknown=4",
    "transient_control_sequences=6 kernel_permanent_owner_requests=1 kernel_permanent_fault_arms=1 kernel_permanent_reads=1 el0_permanent_fault_arm_controls=0 permanent_epoch=7 permanent_authority=kernel-prearmed",
    "injected_reads=3 injected_writes=2 injected_flushes=2 owner_exits=7 offline_probe_exits=1",
    "recovery_attempts=9 recovery_commits=6 recovery_failures=3 recovery_rollbacks=1 fail_closed_retries=1 physical_successes=7 physical_failures=2 attempt_failures=4 probation_io_failures=1",
    "attempt_cap=3 backoffs=3 backoffs_completed=3 backoff_base_ticks=2 backoff_multiplier=2 backoff_ticks=8 probation=7/5/2 healthy_transitions=5",
    "persistent_fault_armed=1 persistent_fault_hits=2 driver_timeouts=7 driver_resets=10 async_starts=9 physical_completions=7 async_physical_failures=2",
    "terminal_quarantine_starts=0 terminal_quarantine_completions=0 terminal_quarantine_physical_errors=0 terminal_irq_rollbacks=1 terminal_dma_verifications=1 terminal_driver_state=2 terminal_transport_status=0 terminal_in_flight=0",
    "final_irq_armed=0 final_irq_failed=1 final_recovery_required=1 final_admission_open=0 final_broker_state=4 final_broker_bound=0 final_broker_pending=0",
    "simulated_permanent=1 hardware_claim=0 arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0",
):
    if needle not in marker_literal:
        raise SystemExit(f"M60 kernel marker lost its revised ledger: {needle!r}")
for stale in (
    "control_sequences=7",
    "el0_permanent_controls=0",
    "permanent_authority=kernel-campaign",
    "recovery_attempts=10",
    "physical_failures=3",
    "probation=7/6/1",
    "persistent_fault_hits=3",
    "driver_resets=11",
    "async_starts=10",
):
    if stale in marker_literal:
        raise SystemExit(f"M60 kernel marker retained a pre-convergence ledger: {stale}")
require_once(
    main,
    "BOOT_OK: M60 bounded StorageServer fault policy and boot-local Offline state verified",
    "M60 BOOT_OK marker",
    "kernel/src/main.rs",
)
dispatch_pattern = re.compile(
    r'#\[cfg\(all\(\s*'
    r'feature = "storage-server-fault-policy-runtime",\s*'
    r'not\(feature = "storage-server-shutdown-orchestration-runtime"\)\s*'
    r'\)\)\]\s*'
    r'validate_storage_server_fault_policy_runtime\(\);'
)
if len(dispatch_pattern.findall(main)) != 1:
    raise SystemExit("main lost the exact M60 validator dispatch")
init_ready_slice = syscall[
    syscall.index("fn init_ready") : syscall.index("fn m55_storage_ready_proof_valid")
]
for needle in (
    'feature = "storage-server-fault-policy-runtime"',
    "M60_STORAGE_READY_PREFIX",
    "M60_STORAGE_PROOF",
    "!with_table(|table| table.is_empty())",
    "!m60_storage_ready_proof_valid()",
):
    require(init_ready_slice, needle, "syscall-local M60 InitReady seal", "kernel/src/syscall.rs")

# The dynamic checker must select only the exact inherited profile and expose
# one production parser body shared by self-test and guest validation.
runtime = texts["scripts/check-storage-server-fault-policy-runtime.sh"]
feature_line = re.search(r'^FEATURES="([^"]*)"$', runtime, flags=re.MULTILINE)
expected_features = [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
    "storage-server-async-recovery-runtime",
    "storage-server-fault-policy-runtime",
]
if feature_line is None or feature_line.group(1).split(",") != expected_features:
    raise SystemExit("M60 runtime checker changed its exact inherited feature closure")
for needle in (
    "--parser-self-test",
    "STORAGE_SERVER_FAULT_POLICY_PARSER_OK",
    "STORAGE_SERVER_FAULT_POLICY_OK",
    "STORAGE_SERVER_RUNTIME_OK abi=25 sector_bytes=512 batch_max=8 volume_sectors=1920 fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 fault_policy=1 attempt_cap=3 device_offline=1",
    "BOOT_OK: M60 bounded StorageServer fault policy and boot-local Offline state verified",
    "simulated_permanent=1",
    "hardware_claim=0",
    "arbitrary_soak_claim=0",
    "powercut_claim=0",
    "concurrency_claim=0",
    "general_runtime=0",
    "early_rejections=0 early_physical_starts=0",
    '("early_rejections=0", "early_rejections=1")',
    '"transient_control_sequences": 6',
    '"kernel_permanent_owner_requests": 1',
    '"kernel_permanent_fault_arms": 1',
    '"kernel_permanent_reads": 1',
    '"el0_permanent_fault_arm_controls": 0',
    '"permanent_epoch": 7',
    '"recovery_attempts": 9',
    '"recovery_commits": 6',
    '"recovery_failures": 3',
    '"physical_successes": 7',
    '"physical_failures": 2',
    '"attempt_failures": 4',
    '"probation_io_failures": 1',
    '"probation_successes": 5',
    '"probation_failures": 2',
    '"persistent_fault_hits": 2',
    '"driver_resets": 10',
    '"async_starts": 9',
    '"async_physical_failures": 2',
    '"early_physical_starts": 0',
    '"backoff_base_ticks": 2',
    '"backoff_multiplier": 2',
    '"backoff_ticks": 8',
    '"backoff_timer_progress_windows": 3',
    '"backoff_worker_progress_windows": 3',
    '"terminal_quarantine_starts": 0',
    '"terminal_quarantine_completions": 0',
    '"terminal_quarantine_physical_errors": 0',
    '"terminal_irq_rollbacks": 1',
    '"terminal_dma_verifications": 1',
    '"terminal_driver_state": 2',
    '"terminal_transport_status": 0',
    '"terminal_in_flight": 0',
    'values["attempt_failures"]',
    'values["probation_io_failures"]',
    'values["probation_failures"]',
    "post_offline_attempt_delta=0",
    "final_broker_state=4",
    "-nic none",
    "CARGO_NET_OFFLINE=true",
):
    require(runtime, needle, "M60 runtime parser/authority contract", "scripts/check-storage-server-fault-policy-runtime.sh")
if 'reject(values["early_rejections"] == 0' in runtime:
    raise SystemExit("M60 runtime parser still requires a positive early-poll rejection count")

test_sh = texts["scripts/test.sh"]
for needle in (
    '"$SCRIPT_DIR/check-storage-server-fault-policy-static.sh"',
    'BNDROID_PROFILE=release "$SCRIPT_DIR/check-storage-server-fault-policy-runtime.sh"',
    "storage_server_fault_policy_static=1",
    "storage_server_fault_policy=1",
):
    require(test_sh, needle, "M60 full-suite integration", "scripts/test.sh")

# Every project QEMU block is offline by construction. Count the live source
# tree rather than hard-coding a milestone total; the M60 checker itself must
# contribute exactly one launch.
launch_pattern = re.compile(r"^\s*(?:exec\s+)?qemu-system-aarch64\s+\\\s*$")
nic_pattern = re.compile(r"^\s*-nic(?:\s|$)")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
launches = 0
m60_launches = 0
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
                raise SystemExit(f"unterminated QEMU launch block: {path.relative_to(root)}")
            block.append(lines[index])
        launches += 1
        if path.name == "check-storage-server-fault-policy-runtime.sh":
            m60_launches += 1
        nic_lines = sum(bool(nic_pattern.match(line)) for line in block)
        nic_none = sum(bool(nic_none_pattern.match(line)) for line in block)
        if nic_lines != 1 or nic_none != 1:
            raise SystemExit(
                "QEMU launch must contain exactly one NIC option, `-nic none`: "
                f"{path.relative_to(root)}"
            )
        index += 1
if launches == 0 or m60_launches != 1:
    raise SystemExit(
        f"M60 static contract found an invalid QEMU launch ledger: "
        f"all={launches} m60={m60_launches}"
    )

print(
    "STORAGE_SERVER_FAULT_POLICY_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} qemu_launches={launches} nic_none={launches} "
    "ticketed=1 probation=1 attempt_cap=3 backoff_base_ticks=2 "
    "backoff_multiplier=2 device_offline=sticky offline_probe=1 "
    "commit_order=rearm-open-prearm-broker transient_el0_controls=6 "
    "el0_permanent_fault_arm_controls=0 permanent_authority=kernel-prearmed "
    "terminal_irq_dma_proof=1"
)
PY

# Exercise five real pre-Cargo rejection paths. These invalid profiles stop in
# build-kernel.sh with status 2, so neither Cargo nor QEMU can be reached.
mkdir -p "$WORKSPACE_ROOT/target"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m60-static.XXXXXX")"
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
    echo "build-kernel did not reject the expected M60 storage feature mismatch." >&2
    exit 1
  fi
}

expect_profile_rejection \
  "storage-server-async-recovery-runtime" \
  "storage-server-fault-policy-runtime" \
  "userspace fault-policy profile exceeds the kernel storage profile."
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

# This mode is parser-only: it neither builds nor launches QEMU. Preserve its
# exact marker in this checker's output, and derive the negative count rather
# than duplicating it here.
PARSER_OUTPUT="$($SCRIPT_DIR/check-storage-server-fault-policy-runtime.sh --parser-self-test)"
if [[ "$PARSER_OUTPUT" =~ ^STORAGE_SERVER_FAULT_POLICY_PARSER_OK\ negative_cases=([1-9][0-9]*)$ ]]; then
  PARSER_NEGATIVE_CASES="${BASH_REMATCH[1]}"
else
  echo "$PARSER_OUTPUT" >&2
  echo "M60 runtime parser self-test did not publish its exact marker." >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"

echo "STORAGE_SERVER_FAULT_POLICY_STATIC_OK feature_mismatch_cases=5 parser_negative_cases=$PARSER_NEGATIVE_CASES ticketed=1 probation=1 probation_io_failure=requires-reset attempt_cap=3 backoff=2x2 offline=sticky offline_probe=1 transient_el0_controls=6 el0_permanent_fault_arm_controls=0 permanent_authority=kernel-prearmed terminal_irq_dma_proof=1"
