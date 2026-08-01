#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"
export CARGO_NET_OFFLINE=true

for tool in bash python3 cargo rustc sed mktemp mkdir rm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found; cannot verify the M61 owner-liveness contracts." >&2
    exit 1
  fi
done

# This is syntax-only. It cannot execute a runtime checker or reach QEMU.
for script in "$SCRIPT_DIR"/*.sh; do
  bash -n "$script"
done

python3 - "$WORKSPACE_ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
feature = "storage-server-owner-liveness-runtime"
parent = "storage-server-fault-policy-runtime"


def source(relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"M61 static contract source is missing: {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, needle: str, description: str, relative: str) -> None:
    if needle not in text:
        raise SystemExit(
            f"M61 static contract lost {description}: {relative}: {needle!r}"
        )


def require_once(text: str, needle: str, description: str, relative: str) -> None:
    count = text.count(needle)
    if count != 1:
        raise SystemExit(
            f"M61 static contract requires exactly one {description}: "
            f"{relative}: found={count} needle={needle!r}"
        )


def forbid(text: str, needle: str, description: str, relative: str) -> None:
    if needle in text:
        raise SystemExit(
            f"M61 static contract gained forbidden {description}: "
            f"{relative}: {needle!r}"
        )


def ordered(text: str, needles: tuple[str, ...], description: str, relative: str) -> None:
    cursor = 0
    for needle in needles:
        position = text.find(needle, cursor)
        if position < 0:
            raise SystemExit(
                f"M61 static contract lost ordered {description}: "
                f"{relative}: {needle!r}"
            )
        cursor = position + len(needle)


def between(text: str, start: str, end: str, description: str, relative: str) -> str:
    start_count = text.count(start)
    if start_count != 1:
        raise SystemExit(
            f"M61 static contract could not isolate {description}: "
            f"{relative}: start_count={start_count}"
        )
    begin = text.index(start)
    finish = text.find(end, begin + len(start))
    if finish < 0:
        raise SystemExit(
            f"M61 static contract lost the end of {description}: {relative}: {end!r}"
        )
    return text[begin:finish]


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


relative_sources = (
    "kernel/Cargo.toml",
    "user/init/Cargo.toml",
    "scripts/build-kernel.sh",
    "scripts/build-userspace.sh",
    "kernel/build.rs",
    "kernel/src/lib.rs",
    "kernel/src/storage_owner_liveness.rs",
    "kernel/src/storage_server_io.rs",
    "kernel/src/process.rs",
    "kernel/src/scheduler.rs",
    "kernel/src/syscall.rs",
    "kernel/src/main.rs",
    "user/init/src/storage_server_runtime.rs",
    "user/init/src/main.rs",
    "scripts/check-storage-server-owner-liveness-runtime.sh",
)
texts = {relative: source(relative) for relative in relative_sources}

kernel_cargo = texts["kernel/Cargo.toml"]
user_cargo = texts["user/init/Cargo.toml"]
build_kernel = texts["scripts/build-kernel.sh"]
build_userspace = texts["scripts/build-userspace.sh"]
build_rs = texts["kernel/build.rs"]
lib_rs = texts["kernel/src/lib.rs"]
policy = texts["kernel/src/storage_owner_liveness.rs"]
coordinator = texts["kernel/src/storage_server_io.rs"]
process = texts["kernel/src/process.rs"]
scheduler = texts["kernel/src/scheduler.rs"]
syscall = texts["kernel/src/syscall.rs"]
main = texts["kernel/src/main.rs"]
user_runtime = texts["user/init/src/storage_server_runtime.rs"]
user_main = texts["user/init/src/main.rs"]
runtime = texts["scripts/check-storage-server-owner-liveness-runtime.sh"]

# M61 is one opt-in leaf over M60 in both halves of the image. build-kernel
# expands the complete inherited closure, rejects a userspace-only leaf before
# Cargo, mirrors kernel-selected features into userspace, and exports the
# StorageServer ELF that build.rs actually embeds.
for relative, manifest in (
    ("kernel/Cargo.toml", kernel_cargo),
    ("user/init/Cargo.toml", user_cargo),
):
    dependencies = feature_dependencies(manifest, feature, relative)
    if dependencies != (parent,):
        raise SystemExit(
            f"M61 feature must have exactly one M60 parent: "
            f"{relative}: dependencies={dependencies!r}"
        )

for variable in (
    "KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=0",
    "USER_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=0",
):
    require_once(build_kernel, variable, "zero-initialized feature ledger", "scripts/build-kernel.sh")

kernel_leaf = between(
    build_kernel,
    'if feature_list_contains "$KERNEL_FEATURES" "storage-server-owner-liveness-runtime"; then',
    'elif feature_list_contains "$KERNEL_FEATURES" "storage-server-fault-policy-runtime"; then',
    "kernel M61 closure",
    "scripts/build-kernel.sh",
)
ordered(
    kernel_leaf,
    (
        "KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RECOVERY_RUNTIME=1",
        "KERNEL_STORAGE_SERVER_RUNTIME=1",
    ),
    "kernel M61->M60->M58->M57->M56->M55 closure",
    "scripts/build-kernel.sh",
)
user_leaf = between(
    build_kernel,
    'if feature_list_contains "$USERSPACE_FEATURES" "storage-server-owner-liveness-runtime"; then',
    'elif feature_list_contains "$USERSPACE_FEATURES" "storage-server-fault-policy-runtime"; then',
    "userspace M61 closure",
    "scripts/build-kernel.sh",
)
ordered(
    user_leaf,
    (
        "USER_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME=1",
        "USER_STORAGE_SERVER_FAULT_POLICY_RUNTIME=1",
        "USER_STORAGE_SERVER_ASYNC_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_REPEATED_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_RECOVERY_RUNTIME=1",
        "USER_STORAGE_SERVER_RUNTIME=1",
    ),
    "userspace M61->M60->M58->M57->M56->M55 closure",
    "scripts/build-kernel.sh",
)
for needle in (
    'if [[ "$USER_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME" -eq 1 \\\n  && "$KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME" -ne 1 ]]; then',
    'echo "userspace owner-liveness profile exceeds the kernel storage profile." >&2',
    'if [[ "$KERNEL_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME" -eq 1 ]]; then',
    'USERSPACE_FEATURES="$(append_feature "$USERSPACE_FEATURES" "storage-server-owner-liveness-runtime")"',
    'export BNDROID_STORAGE_SERVER_ELF="$TARGET_ROOT/aarch64-unknown-none/$PROFILE/bndroid-storage-server"',
):
    require(build_kernel, needle, "M61 feature mismatch/mirror/image wiring", "scripts/build-kernel.sh")

for needle in (
    'if feature_list_contains "${BNDROID_USERSPACE_FEATURES:-}" "storage-server-runtime"; then',
    "ELF_NAMES+=(bndroid-storage-server)",
    "ELF_VARIABLES+=(BNDROID_STORAGE_SERVER_ELF)",
    'CARGO_ARGS+=(--features "$BNDROID_USERSPACE_FEATURES")',
):
    require(build_userspace, needle, "M61 userspace image build wiring", "scripts/build-userspace.sh")

require_once(
    build_rs,
    'println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME");',
    "M61 Cargo feature rerun edge",
    "kernel/build.rs",
)
require_once(
    build_rs,
    'if env::var_os("CARGO_FEATURE_STORAGE_SERVER_OWNER_LIVENESS_RUNTIME").is_some() {',
    "M61 userspace cfg forwarding condition",
    "kernel/build.rs",
)
require_once(
    build_rs,
    '.arg("feature=\\\"storage-server-owner-liveness-runtime\\\"");',
    "M61 userspace cfg forwarding argument",
    "kernel/build.rs",
)
for needle in (
    'let include_storage = env::var_os("CARGO_FEATURE_STORAGE_SERVER_RUNTIME").is_some();',
    "*index != STORAGE_SERVER_IMAGE_INDEX || include_storage",
    '("BNDROID_STORAGE_SERVER_ELF", "BNDR_STORAGE_SERVER_ELF")',
):
    require(build_rs, needle, "M61 embedded StorageServer image selection", "kernel/build.rs")

# The policy file is a host-testable, allocation-free state machine. It has no
# scheduler, process, storage, architecture, locking, or EL0 dependency.
require_once(lib_rs, "pub mod storage_owner_liveness;", "policy module export", "kernel/src/lib.rs")
for needle, description in (
    ("crate::", "kernel adapter dependency"),
    ("std::", "std dependency"),
    ("alloc::", "allocator dependency"),
    ("UnsafeCell", "interior-unsafe storage"),
    ("unsafe ", "unsafe code"),
    ("Atomic", "atomic synchronization"),
    ("Mutex", "mutex synchronization"),
    ("spin_loop", "busy wait"),
    ("Vec<", "dynamic vector"),
    ("Box<", "heap box"),
):
    forbid(policy, needle, description, "kernel/src/storage_owner_liveness.rs")

for needle, description in (
    ("pub struct OwnerIdentity", "generation-qualified owner identity"),
    ("pid: u64", "ticket PID"),
    ("broker_epoch: u64", "ticket broker epoch"),
    ("pub enum Phase", "explicit phase machine"),
    ("Idle = 0", "idle phase"),
    ("Grace = 1", "grace phase"),
    ("TerminationRequested = 2", "termination-requested phase"),
    ("lease_generation: u64", "lease generation"),
    ("deadline: u64", "physical deadline"),
    ("grace_counter_units: u64", "physical grace units"),
    ("pub const fn retirement_barrier_active", "retirement barrier"),
    ("!matches!(self.phase, Phase::Idle)", "phase-owned barrier"),
    ("now.wrapping_sub(deadline) < (1_u64 << 63)", "wrap-safe deadline comparison"),
):
    require(policy, needle, description, "kernel/src/storage_owner_liveness.rs")

if re.search(r"\b(?:pub\s+)?fn\s+(?:renew|heartbeat)\b", policy):
    raise SystemExit("M61 pure policy gained an EL0-renewable lease/heartbeat method")
require_once(
    policy,
    "self.deadline = now.wrapping_add(self.grace_counter_units);",
    "one-shot deadline assignment",
    "kernel/src/storage_owner_liveness.rs",
)
active_observation = between(
    policy,
    "Phase::Grace | Phase::TerminationRequested => {",
    "    pub fn record_termination_outcome(",
    "active-ticket observation",
    "kernel/src/storage_owner_liveness.rs",
)
forbid(
    active_observation,
    "self.deadline =",
    "deadline renewal after arm",
    "kernel/src/storage_owner_liveness.rs",
)
ordered(
    active_observation,
    (
        "if !recovery_latched",
        "PolicyError::RecoveryLatchCleared",
        "if bound_owner.is_some_and(|bound| bound != owner)",
        "PolicyError::OwnerChanged",
        "if !tracked_process_present",
        "if bound_owner.is_some()",
        "PolicyError::BindingSurvivedReap",
        "self.clear_active();",
        "if self.phase == Phase::Grace && deadline_reached(now, self.deadline)",
        "self.phase = Phase::TerminationRequested;",
        "Action::ForceRetire",
        "lease_generation: self.lease_generation",
    ),
    "fail-closed reap barrier and exact deadline transition",
    "kernel/src/storage_owner_liveness.rs",
)

expected_tests = (
    "fault_latch_arms_once_and_forces_only_at_deadline",
    "broker_unbind_does_not_clear_a_live_owner_ticket",
    "cooperative_reap_clears_the_barrier_without_expiry",
    "repeated_owner_activity_cannot_renew_the_deadline",
    "stale_termination_outcome_cannot_commit_a_new_ticket",
    "identity_change_and_latch_clear_fail_closed",
    "counter_wrap_keeps_the_deadline_exact",
    "already_terminal_race_is_accounted_separately",
    "rejects_zero_and_ambiguous_configuration",
)
if policy.count("#[test]") != len(expected_tests):
    raise SystemExit(
        f"M61 pure policy host-test ledger changed: "
        f"found={policy.count('#[test]')} expected={len(expected_tests)}"
    )
for test in expected_tests:
    require_once(
        policy,
        f"fn {test}()",
        f"policy host test {test}",
        "kernel/src/storage_owner_liveness.rs",
    )

# Fatal completion and ticket publication are one atomic broker window. The
# arm must occur after finish_service publishes the poison result, but before
# DAIF restoration and before any waiter is woken.
service_pending = between(
    coordinator,
    "pub fn service_pending() {",
    '#[cfg(feature = "storage-server-async-recovery-runtime")]\npub fn record_async_acquire_wait()',
    "StorageServer completion path",
    "kernel/src/storage_server_io.rs",
)
finish_window = between(
    service_pending,
    '#[cfg(feature = "storage-server-owner-liveness-runtime")]\n    let finish_result = {',
    '#[cfg(not(feature = "storage-server-owner-liveness-runtime"))]',
    "M61 fatal-completion DAIF window",
    "kernel/src/storage_server_io.rs",
)
ordered(
    finish_window,
    (
        "save_and_mask_irq()",
        "storage_broker::finish_service(request, code, read_data)",
        "if result.is_ok() && recovery_required",
        "arm_owner_liveness_after_fatal_completion_irq_masked();",
        "restore_daif(saved_daif)",
    ),
    "finish->arm->restore publication",
    "kernel/src/storage_server_io.rs",
)
if finish_window.count("restore_daif(") != 1 or finish_window.count("finish_service(") != 1:
    raise SystemExit("M61 fatal-completion window is no longer a single finish/restore transaction")
ordered(
    service_pending,
    (
        "arm_owner_liveness_after_fatal_completion_irq_masked();",
        "restore_daif(saved_daif);",
        "crate::scheduler::wake_object_waiters_for_storage();",
    ),
    "arm before restore and waiter wake",
    "kernel/src/storage_server_io.rs",
)
if coordinator.count("arm_owner_liveness_after_fatal_completion_irq_masked") != 2:
    raise SystemExit("M61 fatal-completion arm must have one definition and one call site")

arm_adapter = between(
    coordinator,
    "fn arm_owner_liveness_after_fatal_completion_irq_masked() {",
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\nfn start_backoff_progress_window()',
    "fatal-completion arm adapter",
    "kernel/src/storage_server_io.rs",
)
ordered(
    arm_adapter,
    (
        "irq_is_masked()",
        "storage::recovery_required()",
        "storage_broker::snapshot()",
        "broker.owner_pid",
        "broker.epoch",
        "storage_watchdog_target_present(owner.pid(), owner.broker_epoch())",
        "crate::arch::aarch64::timer::counter_value()",
        "Some(owner)",
        "true,",
        "true,",
    ),
    "authenticated physical-counter ticket arm",
    "kernel/src/storage_server_io.rs",
)
for forbidden in ("terminate_storage_owner", "service_recovery", "release_process"):
    forbid(arm_adapter, forbidden, "arm-time retirement/recovery authority", "kernel/src/storage_server_io.rs")

grace_conversion = between(
    coordinator,
    "fn owner_retire_grace_counter_units() -> u64 {",
    '#[cfg(feature = "storage-server-owner-liveness-runtime")]\nfn owner_liveness_irq_masked',
    "physical grace conversion",
    "kernel/src/storage_server_io.rs",
)
for needle in (
    "OWNER_RETIRE_GRACE_MILLISECONDS: u64 = 250",
    "timer::frequency_hz()",
    "u128::from(frequency)",
    ".div_ceil(1_000)",
    "units == 0 || units >= (1_u64 << 63)",
):
    require(coordinator if "OWNER_RETIRE" in needle else grace_conversion, needle, "non-early physical grace", "kernel/src/storage_server_io.rs")

# Expiry is committed entirely inside the authenticated monitor's masked
# domain. The complete policy ticket is recorded after the exact process-table
# target is made terminal, and the policy cannot clear until a later reap proof.
owner_service = between(
    coordinator,
    "pub fn service_owner_liveness() -> bool {",
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\nfn with_fault_policy',
    "owner-liveness monitor service",
    "kernel/src/storage_server_io.rs",
)
ordered(
    owner_service,
    (
        "save_and_mask_irq()",
        "storage_watchdog_monitor_active()",
        "timer::counter_value()",
        "storage_broker::snapshot()",
        "storage::recovery_required()",
        "policy.observe(now, bound_owner, recovery_latched, tracked_process_present)",
        "OwnerLivenessAction::ForceRetire",
        "lease_generation",
        "storage_watchdog_target_present(",
        "terminate_storage_owner_from_watchdog(",
        "record_termination_outcome(owner, lease_generation, outcome)",
        "restore_daif(saved_daif)",
    ),
    "observe->ticket revalidation->terminate->record->restore",
    "kernel/src/storage_server_io.rs",
)
if owner_service.count("restore_daif(") != 1:
    raise SystemExit("M61 owner-liveness expiry escaped its one DAIF-masked transaction")
for needle in (
    "verified_broker.owner_pid == owner.pid()",
    "verified_broker.epoch == owner.broker_epoch()",
    "binding_matches",
    "!storage::recovery_required()",
    "StorageWatchdogTerminateError::AlreadyTerminal",
):
    require(owner_service, needle, "complete expiry ticket/race validation", "kernel/src/storage_server_io.rs")

recovery = coordinator[coordinator.rindex(
    '#[cfg(feature = "storage-server-fault-policy-runtime")]\npub fn service_recovery()'
):]
ordered(
    recovery,
    (
        "if owner_retirement_barrier_active()",
        "panic!(\"M61 physical recovery crossed a live owner-retirement barrier\")",
        "return;",
        "let mut broker = broker_snapshot();",
        "storage::begin_async_recovery",
    ),
    "retirement barrier before all physical recovery",
    "kernel/src/storage_server_io.rs",
)
for needle in (
    "static OWNER_BACKOFF_EARLY_REJECTION_PROBED: AtomicBool = AtomicBool::new(false);",
    "fn record_one_owner_backoff_early_rejection(delay_ticks: u64, deadline: u64)",
    "OWNER_BACKOFF_EARLY_REJECTION_PROBED.swap(true, Ordering::AcqRel)",
    "let probe_now = deadline.wrapping_sub(delay_ticks);",
    "policy.begin_attempt(probe_now)",
    "Err(RejectReason::BackoffPending { deadline: observed }) if observed == deadline",
    "if !deadline_reached",
):
    require(
        coordinator,
        needle,
        "one-shot M61 early-backoff rejection witness",
        "kernel/src/storage_server_io.rs",
    )
if coordinator.count("record_one_owner_backoff_early_rejection") != 2:
    raise SystemExit("M61 early-backoff witness must have one definition and one call site")

# Process and scheduler adapters authenticate monitor context, generation,
# image role, broker epoch, and capability ownership. They only mark the target
# terminal. release_process remains unique to the ordinary process reaper.
watchdog_process = between(
    process,
    "pub fn storage_watchdog_target_present(",
    "/// Classifies a generation-qualified child wait",
    "process watchdog adapter",
    "kernel/src/process.rs",
)
for needle in (
    "scheduler::storage_watchdog_monitor_active()",
    "ProcessId(raw)",
    "id.generation() == 0",
    "id.generation() != slot.generation",
    "validate_storage_watchdog_target(process, id, broker_epoch)",
    "process.id != id",
    "process.is_init",
    "process.image_id != UserImageId::StorageServer",
    "capability.owner_pid() != id.raw()",
    "capability.epoch() != broker_epoch",
    "scheduler::terminate_user_process_from_storage_watchdog(id.raw())",
):
    require(watchdog_process, needle, "monitor-only exact process target", "kernel/src/process.rs")
for forbidden in (
    "storage_broker::release_process",
    "reap_terminal_child(",
    "address_space.take()",
):
    forbid(watchdog_process, forbidden, "watchdog-side process teardown", "kernel/src/process.rs")

# StorageVolume is process-lifetime only in M61. A live M61 owner cannot unbind
# itself with HandleClose and turn missing broker state into fake reap proof,
# while the not(M61) profiles retain their historical release-before-commit ABI.
handle_close = between(
    syscall,
    "fn handle_close(frame: *mut TrapFrame, raw: u64) -> *mut TrapFrame {",
    "fn init_ready(",
    "HandleClose implementation",
    "kernel/src/syscall.rs",
)
m61_close_cfg = '#[cfg(feature = "storage-server-owner-liveness-runtime")]'
historical_close_cfg = '''#[cfg(all(
            feature = "storage-server-runtime",
            not(feature = "storage-server-owner-liveness-runtime")
        ))]'''
close_transaction = between(
    handle_close,
    "    let closed = with_table(|table| {",
    "    match closed {",
    "transactional HandleClose body",
    "kernel/src/syscall.rs",
)
ordered(
    close_transaction,
    (
        "table.begin_close(handle)",
        m61_close_cfg,
        "if pending.object().as_storage_volume().is_some()",
        "return Err(CloseFailure::Storage);",
        historical_close_cfg,
        "let storage_released = if pending.object().as_storage_volume().is_some()",
        "bndroid_kernel::storage_broker::release_process(process_id)",
        "true",
        "pending.commit()",
    ),
    "M61 rejection before historical release-before-commit branch",
    "kernel/src/syscall.rs",
)
m61_close_branch = close_transaction[
    close_transaction.index(m61_close_cfg) : close_transaction.index(historical_close_cfg)
]
forbid(
    m61_close_branch,
    "storage_broker::release_process",
    "M61 HandleClose broker process release",
    "kernel/src/syscall.rs",
)
historical_close_end = '''#[cfg(any(
            not(feature = "storage-server-runtime"),
            feature = "storage-server-owner-liveness-runtime"
        ))]'''
historical_close_branch = between(
    close_transaction,
    historical_close_cfg,
    historical_close_end,
    "historical StorageVolume close branch",
    "kernel/src/syscall.rs",
)
require_once(
    historical_close_branch,
    "bndroid_kernel::storage_broker::release_process(process_id)",
    "historical HandleClose broker release",
    "kernel/src/syscall.rs",
)
for needle in (
    "Err(CloseFailure::Storage) => complete(frame, Status::InvalidState, 0, 0)",
    "feature = \"storage-server-owner-liveness-runtime\"",
):
    require(handle_close, needle, "profile-isolated StorageVolume close result", "kernel/src/syscall.rs")

release_call = "bndroid_kernel::storage_broker::release_process(id.raw())"
require_once(process, release_call, "StorageServer broker process release", "kernel/src/process.rs")
reaper = between(
    process,
    "fn reap_terminal_child(terminal: scheduler::TerminalUserSnapshot) {",
    "/// Marks one live generation-qualified dynamic child for monitor-side reap.",
    "ordinary terminal child reaper",
    "kernel/src/process.rs",
)
ordered(
    reaper,
    (
        "scheduler::detach_terminal_user(id.raw())",
        release_call,
        "let address_space = process",
        ".address_space",
        ".take()",
    ),
    "detach->broker release->address-space retirement",
    "kernel/src/process.rs",
)

monitor_auth = between(
    scheduler,
    "pub fn storage_watchdog_monitor_active() -> bool {",
    "/// Reports whether one generation-qualified user process",
    "scheduler monitor authentication",
    "kernel/src/scheduler.rs",
)
ordered(
    monitor_auth,
    (
        "irq_is_masked()",
        "current_running_context() == MONITOR_CONTEXT",
        "CONTEXTS[MONITOR_CONTEXT].state.load(Ordering::Relaxed) == STATE_RUNNING",
        "current_translation_context()",
        "kernel_translation_context()",
    ),
    "IRQ/context/state/translation authentication",
    "kernel/src/scheduler.rs",
)

scheduler_termination = between(
    scheduler,
    "enum TerminationAuthority {",
    "pub fn terminal_user_snapshot()",
    "scheduler termination authority",
    "kernel/src/scheduler.rs",
)
for needle in (
    "Init,",
    "MonitorStorageWatchdog,",
    "terminate_user_process_inner(process_id, TerminationAuthority::Init)",
    "terminate_user_process_inner(process_id, TerminationAuthority::MonitorStorageWatchdog)",
    "TerminationAuthority::MonitorStorageWatchdog =>",
    "current == MONITOR_CONTEXT",
    "kernel_translation_context()",
    "STATE_RUNNABLE =>",
    "STATE_WAITING =>",
    "take_exact_token(&token)",
    "OBJECT_WAIT_ABANDONED_BY_TERMINATION.fetch_add(1",
    "STATE_ZOMBIE | STATE_FAULTED => return Err(TerminateUserError::AlreadyTerminal)",
    "STATE_RUNNING | STATE_SLEEPING => return Err(TerminateUserError::InvalidState)",
):
    require(scheduler_termination, needle, "separate monitor-only termination path", "kernel/src/scheduler.rs")
for forbidden in ("storage_broker", "release_process", "reap_terminal"):
    forbid(scheduler_termination, forbidden, "scheduler-side broker/reaper authority", "kernel/src/scheduler.rs")

# The monitor loop explicitly performs kill -> ordinary reap -> exact ticket
# re-observation -> recovery. A cooperative reap is allowed before the first
# observation, but recovery is not.
monitor_loop = between(
    main,
    "// A process-table reap, not broker unbinding, closes accepted",
    "        let syscalls = syscall::snapshot();",
    "M61 monitor convergence loop",
    "kernel/src/main.rs",
)
ordered(
    monitor_loop,
    (
        "process::reap_terminal_children();",
        "if storage_server_io::service_owner_liveness() {",
        "process::reap_terminal_children();",
        "if storage_server_io::service_owner_liveness() {",
        "panic!(\"M61 owner-retirement ticket expired twice\")",
        "storage_server_io::service_recovery();",
    ),
    "cooperative-reap/kill->reap->reobserve->recovery",
    "kernel/src/main.rs",
)
if monitor_loop.count("storage_server_io::service_recovery();") != 1:
    raise SystemExit("M61 monitor block must expose exactly one post-reobserve recovery call")

# Epoch six performs a real flush, publishes OutcomeUnknown, then blocks in a
# real single-object ObjectWait. Init only waits for the kernel-killed process;
# its M61 orchestration function has no ProcessTerminate call.
require_once(
    user_runtime,
    "const RECOVERY_STAGE_OWNER_STALL_FLUSH: u64 = 7;",
    "owner-stall stage selector",
    "user/init/src/storage_server_runtime.rs",
)
sequence = between(
    user_runtime,
    '#[cfg(feature = "storage-server-owner-liveness-runtime")]\n    let sequence = [',
    "    let mut generation = 0;",
    "M61 six-owner sequence",
    "user/init/src/storage_server_runtime.rs",
)
ordered(
    sequence,
    (
        "RECOVERY_STAGE_WRITE",
        "RECOVERY_STAGE_READ",
        "RECOVERY_STAGE_FLUSH",
        "RECOVERY_STAGE_WRITE",
        "RECOVERY_STAGE_READ",
        "RECOVERY_STAGE_OWNER_STALL_FLUSH",
    ),
    "WRFWR plus stalled epoch-six flush",
    "user/init/src/storage_server_runtime.rs",
)
if sequence.count("RECOVERY_RESULT_OUTCOME_UNKNOWN") != 4:
    raise SystemExit("M61 owner sequence lost its four mutation OutcomeUnknown results")

stall_stage = between(
    user_runtime,
    "RECOVERY_STAGE_OWNER_STALL_FLUSH => {",
    "        RECOVERY_STAGE_FINAL => {",
    "stalled StorageServer flush",
    "user/init/src/storage_server_runtime.rs",
)
ordered(
    stall_stage,
    (
        "recovery_fault_control(io, CONTROL_PREFIX_LBA, CONTROL_FLUSH_LBA)",
        "io.execute(StorageBlockRequest::flush(), None)",
        "Err(IoError::OutcomeUnknown) => RECOVERY_RESULT_OUTCOME_UNKNOWN",
    ),
    "real epoch-six fatal flush",
    "user/init/src/storage_server_runtime.rs",
)
mounted_stage = between(
    user_runtime,
    "fn run_mounted_recovery_stage(startup: u64, io: &mut VolumeIo, stage: u64) {",
    "/// The seventh M60 owner deliberately does not mount AppData.",
    "mounted recovery stage",
    "user/init/src/storage_server_runtime.rs",
)
stall_wait_start = mounted_stage.rindex(
    '#[cfg(feature = "storage-server-owner-liveness-runtime")]\n    if stage == RECOVERY_STAGE_OWNER_STALL_FLUSH {'
)
stall_wait_end = mounted_stage.index(
    "        exit_child(SERVER_RECOVERY_EXIT_CODE);", stall_wait_start
)
stall_wait = mounted_stage[stall_wait_start:stall_wait_end]
ordered(
    mounted_stage,
    (
        "RECOVERY_STAGE_OWNER_STALL_FLUSH => {",
        "io.execute(StorageBlockRequest::flush(), None)",
        "Err(IoError::OutcomeUnknown) => RECOVERY_RESULT_OUTCOME_UNKNOWN",
        'syscall(SyscallNumber::HandleClose, io.handle, 0, 0)',
        "close.status != KernelStatus::InvalidState.raw()",
        "write_scalar(startup, SERVER_RECOVERY_RESULT_TAG",
        "object_wait(startup, ObjectSignals::READABLE)",
    ),
    "fatal flush->live close denial->result->ObjectWait",
    "user/init/src/storage_server_runtime.rs",
)
for needle in (
    "object_wait(startup, ObjectSignals::READABLE)",
    "fail(FAIL_SERVER_PROTOCOL)",
):
    require(stall_wait, needle, "non-cooperative single ObjectWait", "user/init/src/storage_server_runtime.rs")
for forbidden in ("ProcessTerminate", "ProcessWait", "exit_child("):
    forbid(stall_wait, forbidden, "EL0 retirement shortcut", "user/init/src/storage_server_runtime.rs")

object_wait_wrapper = between(
    user_main,
    "fn object_wait(handle: u64, requested: ObjectSignals) -> SyscallResult {",
    "fn exercise_object_wait_many_array()",
    "Init ObjectWait syscall wrapper",
    "user/init/src/main.rs",
)
require(
    object_wait_wrapper,
    "SyscallNumber::ObjectWait",
    "real single-object wait syscall",
    "user/init/src/main.rs",
)

m61_init = between(
    user_runtime,
    '''#[cfg(all(
    feature = "storage-server-fault-policy-runtime",
    not(feature = "storage-server-shutdown-orchestration-runtime")
))]
pub(super) fn init_runtime(system_root: u64) -> ! {''',
    '#[cfg(feature = "storage-server-recovery-runtime")]\nfn expect_recovery_result',
    "M61/M60 Init orchestration",
    "user/init/src/storage_server_runtime.rs",
)
forbid(m61_init, "ProcessTerminate", "Init-side forced retirement", "user/init/src/storage_server_runtime.rs")
forbid(m61_init, "terminate_server(", "Init-side termination helper", "user/init/src/storage_server_runtime.rs")
require(
    m61_init,
    "wait_owner_liveness_retirement(server)",
    "kernel-owned retirement observation",
    "user/init/src/storage_server_runtime.rs",
)
owner_wait = between(
    user_runtime,
    "fn wait_owner_liveness_retirement(server: Child) {",
    "fn validate_phase_one_proof(",
    "Init owner-retirement wait",
    "user/init/src/storage_server_runtime.rs",
)
for needle in (
    "SyscallNumber::ProcessWait",
    "PROCESS_KILLED_EXIT_CODE",
    "ProcessTerminationReason::Killed",
):
    require(owner_wait, needle, "kernel-owned retirement result", "user/init/src/storage_server_runtime.rs")
forbid(
    owner_wait,
    "SyscallNumber::ProcessTerminate",
    "Init-side forced retirement syscall",
    "user/init/src/storage_server_runtime.rs",
)

# Kernel proof fields and claim boundaries are byte-stable. They deliberately
# say simulated/single-core/non-general and do not overclaim hardware, SMP,
# soak, power-cut, concurrency, or production runtime coverage.
owner_markers = re.findall(r'"(STORAGE_SERVER_OWNER_LIVENESS_OK [^"\n]+)"', main)
if len(owner_markers) != 1:
    raise SystemExit(f"M61 owner evidence marker is not exact and unique: {len(owner_markers)}")
owner_marker = owner_markers[0]
for needle in (
    "authority=kernel-fatal-completion",
    "ticket=pid-epoch-lease-generation",
    "deadline=physical-counter grace_ms=250 grace_counter_units={}",
    "arms=7 cooperative_retirements=6 deadlines_expired=1 early_expirations=0",
    "termination_requests=1 already_terminal=0 forced_retirements=1 forced_reaps=1",
    "stalled_epoch=6 stalled_operation=flush stalled_completion=outcome-unknown live_volume_close=denied target_wait=single",
    "object_wait_abandoned=1 el0_process_terminate_calls=0",
    "terminated_exited=7 terminated_killed=1 recovery_after_forced_retirement=1 replacement_epoch=7",
    "final_phase=0 final_owner_pid=0 final_broker_epoch=0 next_lease_generation=8",
    "el0_arm_controls=0 el0_renew_controls=0 simulated_fault=1 hardware_claim=0 smp_claim=0",
    "arbitrary_soak_claim=0 powercut_claim=0 concurrency_claim=0 general_runtime=0 invariant_errors=0",
):
    require(owner_marker, needle, "bounded owner-liveness evidence", "kernel/src/main.rs")

for needle in (
    "owner_liveness.arms == 7",
    "owner_liveness.cooperative_retirements == 6",
    "owner_liveness.deadlines_expired == 1",
    "owner_liveness.termination_requests == 1",
    "owner_liveness.forced_retirements == 1",
    "object_wait.abandoned_by_termination == 1",
    "syscalls.process_terminate_calls == 0",
    "syscalls.process_terminate_successes == 0",
):
    require(main, needle, "independently checked owner-liveness ledger", "kernel/src/main.rs")

runtime_literal = (
    "STORAGE_SERVER_RUNTIME_OK abi={} sector_bytes={} batch_max={} volume_sectors={} "
    "fail_stop=1 kernel_reset_authority=1 repeated_recovery=1 async_recovery=1 "
    "fault_policy=1 attempt_cap=3 device_offline=1 owner_liveness=1 exit_grace_ms=250"
)
boot_literal = (
    "BOOT_OK: M61 fault-latched StorageServer owner retirement and recovery convergence verified"
)
require_once(main, f'"{runtime_literal}"', "M61 runtime boundary marker", "kernel/src/main.rs")
require_once(main, boot_literal, "M61 BOOT_OK boundary marker", "kernel/src/main.rs")
ordered(
    main,
    (
        "STORAGE_SERVER_FAULT_POLICY_OK recoverable_cases=6",
        "STORAGE_SERVER_OWNER_LIVENESS_OK authority=kernel-fatal-completion",
        runtime_literal,
        boot_literal,
    ),
    "base->owner->runtime->boot evidence order",
    "kernel/src/main.rs",
)

# The runtime checker has one inherited M61 profile, one shared production
# parser, a parser-only early exit, and exactly one offline QEMU launch.
feature_line = re.search(r'^FEATURES="([^"]*)"$', runtime, flags=re.MULTILINE)
expected_features = [
    "storage-server-runtime",
    "storage-server-recovery-runtime",
    "storage-server-repeated-recovery-runtime",
    "storage-server-async-recovery-runtime",
    "storage-server-fault-policy-runtime",
    "storage-server-owner-liveness-runtime",
]
if feature_line is None or feature_line.group(1).split(",") != expected_features:
    raise SystemExit("M61 runtime checker changed its exact inherited feature closure")
for needle in (
    "--parser-self-test",
    "STORAGE_SERVER_OWNER_LIVENESS_PARSER_OK",
    "negative_cases=",
    "base_fields=",
    "owner_fields=",
    "runtime_fields=",
    "STORAGE_SERVER_OWNER_LIVENESS_OK",
    '("ticket", "pid-epoch-lease-generation")',
    '("deadline", "physical-counter")',
    '("stalled_epoch", "6")',
    '("live_volume_close", "denied")',
    '("object_wait_abandoned", "1")',
    '("el0_process_terminate_calls", "0")',
    '("el0_arm_controls", "0")',
    '("el0_renew_controls", "0")',
    '("simulated_fault", "1")',
    '("hardware_claim", "0")',
    '("smp_claim", "0")',
    '("arbitrary_soak_claim", "0")',
    '("powercut_claim", "0")',
    '("concurrency_claim", "0")',
    '("general_runtime", "0")',
    "CARGO_NET_OFFLINE=true",
    'base_numbers["acquire_waits"] not in {25, 26}',
    'base_numbers["acquire_dispatch_changes"] not in {25, 26}',
    'base_numbers["acquire_dispatch_changes"] != base_numbers["acquire_waits"]',
    '"acquire_waits_above_phase_set"',
    'mutate_field(canonical, 0, base_index["acquire_waits"], "27")',
    '"acquire_changes_above_phase_set"',
    'mutate_field(canonical, 0, base_index["acquire_dispatch_changes"], "27")',
    '"acquire_waits_exceed_changes"',
    '"acquire_changes_exceed_waits"',
):
    require(runtime, needle, "runtime parser/authority boundary", "scripts/check-storage-server-owner-liveness-runtime.sh")
ordered(
    runtime,
    (
        'if [[ "${1:-}" == "--parser-self-test" ]]; then',
        "run_evidence_parser --self-test",
        "exit 0",
        "for tool in qemu-system-aarch64",
        '"$SCRIPT_DIR/build-kernel.sh"',
        "qemu-system-aarch64 \\",
    ),
    "parser-only exit before build and QEMU",
    "scripts/check-storage-server-owner-liveness-runtime.sh",
)

launch_pattern = re.compile(r"^\s*qemu-system-aarch64\s+\\\s*$")
nic_pattern = re.compile(r"^\s*-nic(?:\s|$)")
nic_none_pattern = re.compile(r"^\s*-nic\s+none(?:\s+\\)?\s*$")
lines = runtime.splitlines()
launches: list[list[str]] = []
index = 0
while index < len(lines):
    if not launch_pattern.match(lines[index]):
        index += 1
        continue
    block = [lines[index]]
    while block[-1].rstrip().endswith("\\"):
        index += 1
        if index >= len(lines):
            raise SystemExit("M61 runtime checker has an unterminated QEMU launch")
        block.append(lines[index])
    launches.append(block)
    index += 1
if len(launches) != 1:
    raise SystemExit(f"M61 runtime checker must contain one QEMU launch: found={len(launches)}")
launch = launches[0]
if sum(bool(nic_pattern.match(line)) for line in launch) != 1:
    raise SystemExit("M61 QEMU launch must contain exactly one NIC option")
if sum(bool(nic_none_pattern.match(line)) for line in launch) != 1:
    raise SystemExit("M61 QEMU launch's sole NIC option must be exactly `-nic none`")
if not any(re.match(r"^\s*-smp\s+1(?:\s+\\)?\s*$", line) for line in launch):
    raise SystemExit("M61 bounded runtime checker lost its explicit single-core QEMU boundary")

print(
    "STORAGE_SERVER_OWNER_LIVENESS_STATIC_SOURCE_OK "
    f"sources={len(relative_sources)} host_tests={len(expected_tests)} "
    "feature_chain=M61-M60-M58-M57-M56-M55 pure_policy=1 nonrenewable=1 "
    "finish_arm_daif_window=1 ticket=pid-epoch-lease-generation-physical-deadline "
    "m61_reaper_release_only=1 historical_handle_close_release=1 "
    "recovery_barrier=1 monitor_only=1 "
    "live_volume_close=denied epoch6_flush_object_wait=1 "
    "init_process_terminate=0 qemu_launches=1 nic_none=1"
)
PY

# Exercise every real pre-Cargo storage-profile mismatch, including the new
# M61 userspace-over-M60 kernel edge. None of these cases can build or launch.
mkdir -p "$WORKSPACE_ROOT/target"
TMP_DIR="$(mktemp -d "$WORKSPACE_ROOT/target/bndroid-storage-server-m61-static.XXXXXX")"
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
    echo "build-kernel did not reject the expected M61 storage feature mismatch." >&2
    exit 1
  fi
}

expect_profile_rejection \
  "storage-server-fault-policy-runtime" \
  "storage-server-owner-liveness-runtime" \
  "userspace owner-liveness profile exceeds the kernel storage profile."
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

# This mode shares the production evidence parser but exits before tool checks,
# builds, disk mutation, or QEMU. Keep its dynamic fixture counts in the final
# result instead of maintaining a weaker second parser here.
PARSER_OUTPUT="$($SCRIPT_DIR/check-storage-server-owner-liveness-runtime.sh --parser-self-test)"
if [[ "$PARSER_OUTPUT" =~ ^STORAGE_SERVER_OWNER_LIVENESS_PARSER_OK\ negative_cases=([1-9][0-9]*)\ base_fields=([1-9][0-9]*)\ owner_fields=([1-9][0-9]*)\ runtime_fields=([1-9][0-9]*)$ ]]; then
  PARSER_NEGATIVE_CASES="${BASH_REMATCH[1]}"
  PARSER_BASE_FIELDS="${BASH_REMATCH[2]}"
  PARSER_OWNER_FIELDS="${BASH_REMATCH[3]}"
  PARSER_RUNTIME_FIELDS="${BASH_REMATCH[4]}"
else
  echo "$PARSER_OUTPUT" >&2
  echo "M61 runtime parser self-test did not publish its exact marker." >&2
  exit 1
fi
printf '%s\n' "$PARSER_OUTPUT"

# Run only the allocation-free liveness state machine's host tests. The
# explicit host target avoids attempting to execute an AArch64 bare-metal test.
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$HOST_TRIPLE" ]]; then
  echo "rustc did not report a host triple for the M61 policy tests." >&2
  exit 1
fi
cargo test --locked --target "$HOST_TRIPLE" -p bndroid-kernel --lib \
  storage_owner_liveness::tests

echo "STORAGE_SERVER_OWNER_LIVENESS_STATIC_OK feature_mismatch_cases=6 parser_negative_cases=$PARSER_NEGATIVE_CASES parser_base_fields=$PARSER_BASE_FIELDS parser_owner_fields=$PARSER_OWNER_FIELDS parser_runtime_fields=$PARSER_RUNTIME_FIELDS host_tests=9 pure_policy=1 nonrenewable=1 finish_arm_daif_window=1 monitor_only=1 m61_reaper_release_only=1 historical_handle_close_release=1 recovery_barrier=1 live_volume_close=denied epoch6_flush_object_wait=1 init_process_terminate=0 qemu_launches=1 nic_none=1"
