//! Monitor-side execution of M55's policy-free sector broker.

#[cfg(feature = "storage-server-fault-policy-runtime")]
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use bndr_abi::{STORAGE_BLOCK_DATA_MAX_BYTES, STORAGE_SECTOR_SIZE, StorageBlockOperation};
use bndroid_kernel::storage_broker::{self, CompletionCode, RequestError};
#[cfg(feature = "storage-server-owner-liveness-runtime")]
use bndroid_kernel::storage_owner_liveness::{
    Action as OwnerLivenessAction, OwnerIdentity, OwnerLivenessPolicy,
    Snapshot as OwnerLivenessSnapshot, TerminationOutcome,
};
#[cfg(feature = "storage-server-fault-policy-runtime")]
use bndroid_kernel::storage_recovery_policy::{
    AttemptTicket, FailureDisposition, RecoveryPolicy, RecoveryPolicySnapshot, RecoveryState,
    RejectReason,
};
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
use bndroid_kernel::storage_terminal_quarantine::{
    Decision as TerminalProofDecision, Snapshot as TerminalProofSnapshot, TerminalProofGate,
};
#[cfg(feature = "storage-server-recovery-runtime")]
use bndroid_kernel::virtio::BlockRequestKind;

use crate::driver::virtio::block::{PhysicalCounter, deadline_after};
use crate::storage::{self, StorageError};

static READS: AtomicU64 = AtomicU64::new(0);
static WRITES: AtomicU64 = AtomicU64::new(0);
static READ_SECTORS: AtomicU64 = AtomicU64::new(0);
static WRITE_SECTORS: AtomicU64 = AtomicU64::new(0);
static FLUSHES: AtomicU64 = AtomicU64::new(0);
static COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static ABANDONED_SERVICES: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static RECOVERY_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static RECOVERY_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static RECOVERY_FAILURES: AtomicU64 = AtomicU64::new(0);
static RECOVERY_REARM_ABORTS: AtomicU64 = AtomicU64::new(0);
static RECOVERY_FAIL_CLOSED_RETRIES: AtomicU64 = AtomicU64::new(0);
static RECOVERY_RETRY_PENDING: AtomicBool = AtomicBool::new(false);
static ASYNC_RECOVERY_STARTS: AtomicU64 = AtomicU64::new(0);
static ASYNC_PHYSICAL_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static ASYNC_PHYSICAL_FAILURES: AtomicU64 = AtomicU64::new(0);
static ASYNC_COORDINATOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static ASYNC_ACTIVE_ATTEMPT: AtomicU64 = AtomicU64::new(0);
static ASYNC_TIMER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_WORKER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_EL0_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static ASYNC_ACQUIRE_WAITS: AtomicU64 = AtomicU64::new(0);
static ASYNC_ACQUIRE_DISPATCH_CHANGES: AtomicU64 = AtomicU64::new(0);
static ASYNC_LAST_ACQUIRE_DISPATCH: AtomicU64 = AtomicU64::new(u64::MAX);
static ASYNC_BASE_TIMER_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_WORKER_0: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_WORKER_1: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_ACQUIRE_WAITS: AtomicU64 = AtomicU64::new(0);
static ASYNC_BASE_ACQUIRE_DISPATCH_CHANGES: AtomicU64 = AtomicU64::new(0);
static ASYNC_MAX_CONTROL_MASKED_TICKS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static BACKOFF_BASE_TIMER_DISPATCHES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static BACKOFF_BASE_WORKER_0: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static BACKOFF_BASE_WORKER_1: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static BACKOFF_TIMER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static BACKOFF_WORKER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-owner-liveness-runtime")]
static OWNER_BACKOFF_EARLY_REJECTION_PROBED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_QUARANTINE_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_QUARANTINE_STARTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_QUARANTINE_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_QUARANTINE_PHYSICAL_ERRORS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_IRQ_ROLLBACKS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static TERMINAL_DMA_VERIFICATIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_STEPS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_PENDING_RETURNS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_BASE_TIMER_DISPATCHES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_BASE_WORKER_0: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_BASE_WORKER_1: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_TIMER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_QUARANTINE_WORKER_PROGRESS_WINDOWS: AtomicU64 = AtomicU64::new(0);
static OUTCOME_UNKNOWN_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static REQUIRES_RESET_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static FAULT_CONTROL_STATE: AtomicU8 = AtomicU8::new(0);
static FAULT_CONTROL_EPOCH: AtomicU64 = AtomicU64::new(0);
static FAULT_CONTROL_SEQUENCES: AtomicU64 = AtomicU64::new(0);
static FAULT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static FAULT_INJECTED_READS: AtomicU64 = AtomicU64::new(0);
static FAULT_INJECTED_WRITES: AtomicU64 = AtomicU64::new(0);
static FAULT_INJECTED_FLUSHES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static KERNEL_PERMANENT_OWNER_REQUESTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static KERNEL_PERMANENT_FAULT_ARMS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static KERNEL_PERMANENT_READS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-fault-policy-runtime")]
static PROBATION_IO_FAILURES: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "storage-server-owner-liveness-runtime")]
const OWNER_RETIRE_GRACE_MILLISECONDS: u64 = 250;

#[cfg(feature = "storage-server-owner-liveness-runtime")]
struct OwnerLivenessSlot(UnsafeCell<Option<OwnerLivenessPolicy>>);

#[cfg(feature = "storage-server-owner-liveness-runtime")]
unsafe impl Sync for OwnerLivenessSlot {}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
static OWNER_LIVENESS: OwnerLivenessSlot = OwnerLivenessSlot(UnsafeCell::new(None));

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
struct TerminalProofGateSlot(UnsafeCell<TerminalProofGate>);

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
unsafe impl Sync for TerminalProofGateSlot {}

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
static TERMINAL_PROOF_GATE: TerminalProofGateSlot =
    TerminalProofGateSlot(UnsafeCell::new(TerminalProofGate::new()));

#[cfg(feature = "storage-server-fault-policy-runtime")]
const FAULT_POLICY_BACKOFF_BASE_TICKS: u64 = 2;

#[cfg(feature = "storage-server-fault-policy-runtime")]
struct FaultPolicySlot(UnsafeCell<RecoveryPolicy>);

#[cfg(feature = "storage-server-fault-policy-runtime")]
unsafe impl Sync for FaultPolicySlot {}

#[cfg(feature = "storage-server-fault-policy-runtime")]
static FAULT_POLICY: FaultPolicySlot = FaultPolicySlot(UnsafeCell::new(match RecoveryPolicy::new(
    FAULT_POLICY_BACKOFF_BASE_TICKS,
) {
    Ok(policy) => policy,
    Err(_) => panic!("M60 fault policy has an invalid static backoff"),
}));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub reads: u64,
    pub writes: u64,
    pub read_sectors: u64,
    pub write_sectors: u64,
    pub flushes: u64,
    pub completions: u64,
    pub abandoned_services: u64,
    pub errors: u64,
    pub recovery_attempts: u64,
    pub recovery_successes: u64,
    pub recovery_failures: u64,
    pub recovery_rearm_aborts: u64,
    pub recovery_fail_closed_retries: u64,
    pub recovery_retry_pending: bool,
    pub async_recovery_starts: u64,
    pub async_physical_completions: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub async_physical_failures: u64,
    pub async_coordinator_active: bool,
    pub async_active_attempt: u64,
    pub async_timer_progress_windows: u64,
    pub async_worker_progress_windows: u64,
    pub async_el0_progress_windows: u64,
    pub async_acquire_waits: u64,
    pub async_acquire_dispatch_changes: u64,
    pub async_max_control_masked_ticks: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub backoff_timer_progress_windows: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub backoff_worker_progress_windows: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_quarantine_active: bool,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_quarantine_starts: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_quarantine_completions: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_quarantine_physical_errors: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_irq_rollbacks: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub terminal_dma_verifications: u64,
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    pub terminal_quarantine_steps: u64,
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    pub terminal_quarantine_pending_returns: u64,
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    pub terminal_quarantine_timer_progress_windows: u64,
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    pub terminal_quarantine_worker_progress_windows: u64,
    pub outcome_unknown_completions: u64,
    pub requires_reset_completions: u64,
    pub fault_control_sequences: u64,
    pub fault_control_state: u8,
    pub fault_control_epoch: u64,
    pub fault_sequence: u64,
    pub fault_injected_reads: u64,
    pub fault_injected_writes: u64,
    pub fault_injected_flushes: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub kernel_permanent_owner_requests: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub kernel_permanent_fault_arms: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub kernel_permanent_reads: u64,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub probation_io_failures: u64,
}

/// Runs one short access to the library-owned global broker without allowing
/// the single CPU to schedule an EL0 syscall that could spin on the same
/// lock. The result is fully detached from the guard before DAIF is restored.
fn with_broker_irq_masked<T>(operation: impl FnOnce() -> T) -> T {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let result = operation();
    crate::arch::aarch64::restore_daif(saved_daif);
    result
}

pub fn broker_snapshot() -> storage_broker::BrokerSnapshot {
    with_broker_irq_masked(storage_broker::snapshot)
}

/// Advances M61's non-renewable, fault-latched owner retirement grace.
///
/// The ticket is armed only from an EL1-observed physical recovery latch and
/// an authenticated broker binding. Broker unbinding cannot clear it while
/// the exact process-table generation remains present, so accepted sessions
/// are closed by the ordinary reaper before physical recovery can begin.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn service_owner_liveness() -> bool {
    use crate::process::StorageWatchdogTerminateError;

    if crate::arch::aarch64::irq_is_masked() {
        panic!("M61 owner-liveness monitor entered with local IRQ masked");
    }
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    if !crate::scheduler::storage_watchdog_monitor_active() {
        panic!("M61 owner-liveness policy escaped monitor context");
    }
    let now = crate::arch::aarch64::timer::counter_value();
    let broker = storage_broker::snapshot();
    let recovery_latched = storage::recovery_required();
    let bound_owner = if broker.bound {
        Some(
            OwnerIdentity::new(broker.owner_pid, broker.epoch)
                .unwrap_or_else(|_| panic!("M61 broker published a zero owner identity")),
        )
    } else {
        None
    };
    let before = owner_liveness_irq_masked(|policy| policy.snapshot());
    let tracked_owner = if before.owner_pid != 0 {
        Some(
            OwnerIdentity::new(before.owner_pid, before.broker_epoch)
                .unwrap_or_else(|_| panic!("M61 active ticket lost its owner identity")),
        )
    } else if recovery_latched {
        bound_owner
    } else {
        None
    };
    let tracked_process_present = tracked_owner.is_some_and(|owner| {
        crate::process::storage_watchdog_target_present(owner.pid(), owner.broker_epoch())
            .unwrap_or_else(|_| panic!("M61 could not authenticate its process-table target"))
    });
    let action = owner_liveness_irq_masked(|policy| {
        policy.observe(now, bound_owner, recovery_latched, tracked_process_present)
    })
    .unwrap_or_else(|error| panic!("M61 owner-liveness invariant failed: {error:?}"));

    let forced = match action {
        OwnerLivenessAction::None => false,
        OwnerLivenessAction::ForceRetire {
            owner,
            lease_generation,
        } => {
            // Revalidate the complete ticket boundary in the same DAIF-masked
            // window that marks the exact process terminal. An already-unbound
            // broker is legal only as a separately detected broker anomaly;
            // the ticket remains live until ordinary process reaping proves
            // that the complete owner authority set is gone.
            let verified_broker = storage_broker::snapshot();
            let binding_matches = !verified_broker.bound
                || (verified_broker.owner_pid == owner.pid()
                    && verified_broker.epoch == owner.broker_epoch());
            if !binding_matches
                || !storage::recovery_required()
                || !crate::process::storage_watchdog_target_present(
                    owner.pid(),
                    owner.broker_epoch(),
                )
                .unwrap_or_else(|_| panic!("M61 expiry target authentication failed"))
            {
                panic!("M61 retirement ticket changed at its expiry commit");
            }
            let outcome = match crate::process::terminate_storage_owner_from_watchdog(
                owner.pid(),
                owner.broker_epoch(),
            ) {
                Ok(()) => TerminationOutcome::Accepted,
                Err(StorageWatchdogTerminateError::AlreadyTerminal) => {
                    TerminationOutcome::AlreadyTerminal
                }
                Err(StorageWatchdogTerminateError::NotFound) => {
                    panic!("M61 expiry target disappeared before its reaper proof")
                }
                Err(StorageWatchdogTerminateError::InvalidState) => {
                    panic!("M61 expiry target failed monitor-only termination authentication")
                }
            };
            owner_liveness_irq_masked(|policy| {
                policy.record_termination_outcome(owner, lease_generation, outcome)
            })
            .unwrap_or_else(|error| panic!("M61 expiry outcome lost its ticket: {error:?}"));
            true
        }
    };
    crate::arch::aarch64::restore_daif(saved_daif);
    forced
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn with_fault_policy<T>(operation: impl FnOnce(&mut RecoveryPolicy) -> T) -> T {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    // SAFETY: Bndroid's current execution profile is single-core. Every
    // access to this cell masks local IRQ, and no IRQ handler accesses it.
    let result = operation(unsafe { &mut *FAULT_POLICY.0.get() });
    crate::arch::aarch64::restore_daif(saved_daif);
    result
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
pub fn fault_policy_snapshot() -> RecoveryPolicySnapshot {
    with_fault_policy(|policy| policy.snapshot())
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn owner_retire_grace_counter_units() -> u64 {
    let frequency = crate::arch::aarch64::timer::frequency_hz();
    let numerator = u128::from(frequency) * u128::from(OWNER_RETIRE_GRACE_MILLISECONDS);
    let rounded_up = numerator.div_ceil(1_000);
    let units = u64::try_from(rounded_up).unwrap_or(0);
    if units == 0 || units >= (1_u64 << 63) {
        panic!("M61 owner-retirement grace cannot be represented by the physical counter");
    }
    units
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn owner_liveness_irq_masked<T>(operation: impl FnOnce(&mut OwnerLivenessPolicy) -> T) -> T {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("M61 owner-liveness state accessed with local IRQ enabled");
    }
    // SAFETY: the current runtime is single-core and every access is made by
    // the monitor with local IRQ masked. No IRQ handler touches this cell.
    let slot = unsafe { &mut *OWNER_LIVENESS.0.get() };
    let policy = slot.get_or_insert_with(|| {
        OwnerLivenessPolicy::new(owner_retire_grace_counter_units())
            .unwrap_or_else(|_| panic!("M61 owner-liveness policy rejected its fixed grace"))
    });
    operation(policy)
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
pub fn owner_liveness_snapshot() -> OwnerLivenessSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = owner_liveness_irq_masked(|policy| policy.snapshot());
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
fn terminal_proof_gate_irq_masked<T>(operation: impl FnOnce(&mut TerminalProofGate) -> T) -> T {
    if !crate::arch::aarch64::irq_is_masked() {
        panic!("M62 terminal-proof gate accessed with local IRQ enabled");
    }
    // SAFETY: M62 remains single-core. Every access is made by the monitor
    // while local IRQ is masked, and no interrupt handler touches this gate.
    operation(unsafe { &mut *TERMINAL_PROOF_GATE.0.get() })
}

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
pub fn terminal_proof_gate_snapshot() -> TerminalProofSnapshot {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let snapshot = terminal_proof_gate_irq_masked(|gate| gate.snapshot());
    crate::arch::aarch64::restore_daif(saved_daif);
    snapshot
}

#[cfg(feature = "storage-server-terminal-quarantine-runtime")]
fn arm_terminal_proof_deferral() {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    terminal_proof_gate_irq_masked(|gate| gate.arm())
        .unwrap_or_else(|error| panic!("M62 terminal-proof gate arm failed: {error:?}"));
    crate::arch::aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn owner_retirement_barrier_active() -> bool {
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let active = owner_liveness_irq_masked(|policy| policy.retirement_barrier_active());
    crate::arch::aarch64::restore_daif(saved_daif);
    active
}

/// Arms the M61 ticket in the same DAIF-masked publication window as a fatal
/// broker completion. No EL0 owner can Take/HandleClose between the poison
/// result becoming visible and the exact `(pid, epoch)` becoming irrevocably
/// tracked by the recovery barrier.
#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn arm_owner_liveness_after_fatal_completion_irq_masked() {
    if !crate::arch::aarch64::irq_is_masked() || !storage::recovery_required() {
        panic!("M61 fatal-completion arm lacked its physical recovery latch");
    }
    let broker = storage_broker::snapshot();
    if !broker.bound || broker.owner_pid == 0 || broker.epoch == 0 {
        panic!("M61 fatal completion lost its authenticated broker owner");
    }
    let owner = OwnerIdentity::new(broker.owner_pid, broker.epoch)
        .unwrap_or_else(|_| panic!("M61 fatal completion published an invalid owner identity"));
    let present =
        crate::process::storage_watchdog_target_present(owner.pid(), owner.broker_epoch())
            .unwrap_or_else(|_| panic!("M61 fatal completion could not authenticate its owner"));
    if !present {
        panic!("M61 fatal completion owner disappeared before ticket publication");
    }
    let action = owner_liveness_irq_masked(|policy| {
        policy.observe(
            crate::arch::aarch64::timer::counter_value(),
            Some(owner),
            true,
            true,
        )
    })
    .unwrap_or_else(|error| panic!("M61 fatal-completion ticket failed: {error:?}"));
    if action != OwnerLivenessAction::None {
        panic!("M61 fatal-completion publication unexpectedly expired an existing ticket");
    }
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn start_backoff_progress_window() {
    let scheduler = crate::scheduler::snapshot();
    BACKOFF_BASE_TIMER_DISPATCHES.store(scheduler.timer_dispatches, Ordering::Release);
    BACKOFF_BASE_WORKER_0.store(scheduler.worker_work[0], Ordering::Release);
    BACKOFF_BASE_WORKER_1.store(scheduler.worker_work[1], Ordering::Release);
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn backoff_progress_window_ready() -> bool {
    let scheduler = crate::scheduler::snapshot();
    scheduler.timer_dispatches > BACKOFF_BASE_TIMER_DISPATCHES.load(Ordering::Acquire)
        && scheduler.worker_work[0] > BACKOFF_BASE_WORKER_0.load(Ordering::Acquire)
        && scheduler.worker_work[1] > BACKOFF_BASE_WORKER_1.load(Ordering::Acquire)
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn finish_backoff_progress_window() {
    if !backoff_progress_window_ready() {
        panic!("M60 backoff completed before timer and both workers made progress");
    }
    BACKOFF_TIMER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
    BACKOFF_WORKER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
}

#[cfg(feature = "storage-server-owner-liveness-runtime")]
fn record_one_owner_backoff_early_rejection(delay_ticks: u64, deadline: u64) {
    if OWNER_BACKOFF_EARLY_REJECTION_PROBED.swap(true, Ordering::AcqRel) {
        return;
    }
    let probe_now = deadline.wrapping_sub(delay_ticks);
    match with_fault_policy(|policy| policy.begin_attempt(probe_now)) {
        Err(RejectReason::BackoffPending { deadline: observed }) if observed == deadline => {}
        _ => panic!("M61 one-shot early backoff probe did not fail closed"),
    }
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn record_policy_disposition(disposition: FailureDisposition) {
    match disposition {
        #[cfg(feature = "storage-server-owner-liveness-runtime")]
        FailureDisposition::Backoff {
            delay_ticks,
            deadline,
            ..
        } => {
            start_backoff_progress_window();
            record_one_owner_backoff_early_rejection(delay_ticks, deadline);
        }
        #[cfg(not(feature = "storage-server-owner-liveness-runtime"))]
        FailureDisposition::Backoff { .. } => start_backoff_progress_window(),
        FailureDisposition::Offline { .. } => {}
    }
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn record_fault_policy_io_result(succeeded: bool) -> bool {
    let snapshot = fault_policy_snapshot();
    if snapshot.state != RecoveryState::Probation {
        return false;
    }
    let ticket = AttemptTicket {
        generation: snapshot.active_attempt_generation,
        ordinal: snapshot.active_attempt_ordinal,
    };
    if succeeded {
        with_fault_policy(|policy| policy.complete_probation_success(ticket))
            .unwrap_or_else(|_| panic!("M60 probation health confirmation lost its ticket"));
        false
    } else {
        storage::require_recovery();
        PROBATION_IO_FAILURES.fetch_add(1, Ordering::Relaxed);
        let now = crate::arch::aarch64::timer::tick_count();
        let disposition =
            with_fault_policy(|policy| policy.complete_probation_failure(ticket, now))
                .unwrap_or_else(|_| panic!("M60 probation failure lost its ticket"));
        record_policy_disposition(disposition);
        true
    }
}

/// Executes at most one accepted 512-byte operation with local IRQ enabled.
/// No EL0 exception frame is live here and no namespace state enters EL1.
pub fn service_pending() {
    if crate::arch::aarch64::irq_is_masked() {
        panic!("M55 storage monitor entered with local IRQ masked");
    }
    let (request, broker_epoch) = with_broker_irq_masked(|| {
        let request = storage_broker::begin_service();
        let epoch = request
            .as_ref()
            .map(|_| storage_broker::snapshot().epoch)
            .unwrap_or(0);
        (request, epoch)
    });
    let Some(request) = request else {
        return;
    };
    #[cfg(feature = "storage-server-recovery-runtime")]
    prepare_recovery_fault(request, broker_epoch);
    #[cfg(not(feature = "storage-server-recovery-runtime"))]
    let _ = broker_epoch;
    let mut read_data = [0_u8; STORAGE_BLOCK_DATA_MAX_BYTES];
    let mut clock = PhysicalCounter;
    let frequency = crate::arch::aarch64::timer::frequency_hz();
    let sector_count = request.sector_count();
    let relative_end = request
        .relative_lba()
        .checked_add(sector_count as u64)
        .filter(|end| *end <= bndr_abi::APP_DATA_VOLUME_SECTORS);
    let result = deadline_after(&mut clock, frequency)
        .ok_or(StorageError::NotReady)
        .and_then(|deadline| match request.operation() {
            StorageBlockOperation::Read | StorageBlockOperation::Write => {
                let relative_end = relative_end.ok_or(StorageError::Block(
                    crate::driver::virtio::block::BlockError::SectorOutOfBounds,
                ))?;
                let physical_start = storage::APPDATA_PARTITION_FIRST_LBA
                    .checked_add(request.relative_lba())
                    .ok_or(StorageError::Block(
                        crate::driver::virtio::block::BlockError::SectorOutOfBounds,
                    ))?;
                let physical_end = storage::APPDATA_PARTITION_FIRST_LBA
                    .checked_add(relative_end)
                    .filter(|end| *end <= storage::APPDATA_PARTITION_END_LBA)
                    .ok_or(StorageError::Block(
                        crate::driver::virtio::block::BlockError::SectorOutOfBounds,
                    ))?;
                if physical_start >= physical_end {
                    return Err(StorageError::Block(
                        crate::driver::virtio::block::BlockError::SectorOutOfBounds,
                    ));
                }
                for index in 0..sector_count {
                    let physical = physical_start + index as u64;
                    let start = index * STORAGE_SECTOR_SIZE;
                    let end = start + STORAGE_SECTOR_SIZE;
                    match request.operation() {
                        StorageBlockOperation::Read => {
                            let sector = (&mut read_data[start..end])
                                .try_into()
                                .unwrap_or_else(|_| panic!("M55 read batch lost sector geometry"));
                            storage::read_sector_irq(physical, sector, &mut clock, deadline)?;
                        }
                        StorageBlockOperation::Write => {
                            let sector = (&request.data()[start..end])
                                .try_into()
                                .unwrap_or_else(|_| panic!("M55 write batch lost sector geometry"));
                            if let Err(error) = storage::write_appdata_sector_irq(
                                physical, sector, &mut clock, deadline,
                            ) {
                                return Err(
                                    if index != 0
                                        && !matches!(
                                            error,
                                            StorageError::SubmittedMutationOutcomeUnknown
                                                | StorageError::RecoveryRequired
                                                | StorageError::InterruptFailure
                                        )
                                    {
                                        StorageError::SubmittedMutationOutcomeUnknown
                                    } else {
                                        error
                                    },
                                );
                            }
                        }
                        StorageBlockOperation::Flush => unreachable!(),
                    }
                }
                Ok(())
            }
            StorageBlockOperation::Flush => storage::flush_irq(&mut clock, deadline),
        });
    if result.is_err() {
        // A failed read batch is all-or-nothing at the ABI boundary. Earlier
        // sectors may have completed internally, but no partial bytes become
        // observable through StorageTake.
        read_data.fill(0);
    }
    // A read deadline and a device configuration fault both latch recovery in
    // the lower block layer. Preserve that fact at the ABI boundary instead
    // of degrading it to a retryable `Unavailable`. Submitted mutations keep
    // the stronger OutcomeUnknown classification while still leaving the
    // broker closed until explicit recovery.
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    let probation_failed = record_fault_policy_io_result(result.is_ok());
    #[cfg(not(feature = "storage-server-fault-policy-runtime"))]
    let probation_failed = false;
    // The policy hook may have closed admission even when the lower block
    // error itself did not carry a reset classification. Sample the latch only
    // after that hook so a probationary DeviceIo/Unsupported failure cannot be
    // returned as retryable Unavailable while its owner remains bound.
    let recovery_required = storage::recovery_required();
    let code = match result {
        Ok(()) => {
            match request.operation() {
                StorageBlockOperation::Read => {
                    READ_SECTORS.fetch_add(sector_count as u64, Ordering::Relaxed);
                    READS.fetch_add(1, Ordering::Relaxed)
                }
                StorageBlockOperation::Write => {
                    WRITE_SECTORS.fetch_add(sector_count as u64, Ordering::Relaxed);
                    WRITES.fetch_add(1, Ordering::Relaxed)
                }
                StorageBlockOperation::Flush => FLUSHES.fetch_add(1, Ordering::Relaxed),
            };
            CompletionCode::Ok
        }
        Err(StorageError::SubmittedMutationOutcomeUnknown) => {
            ERRORS.fetch_add(1, Ordering::Relaxed);
            OUTCOME_UNKNOWN_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            CompletionCode::OutcomeUnknown
        }
        Err(StorageError::RecoveryRequired | StorageError::InterruptFailure) => {
            ERRORS.fetch_add(1, Ordering::Relaxed);
            REQUIRES_RESET_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            CompletionCode::RequiresReset
        }
        Err(_) if probation_failed || recovery_required => {
            ERRORS.fetch_add(1, Ordering::Relaxed);
            REQUIRES_RESET_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
            CompletionCode::RequiresReset
        }
        Err(_) => {
            ERRORS.fetch_add(1, Ordering::Relaxed);
            CompletionCode::Unavailable
        }
    };
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    let finish_result = {
        let saved_daif = crate::arch::aarch64::save_and_mask_irq();
        let result = storage_broker::finish_service(request, code, read_data);
        if result.is_ok() && recovery_required {
            arm_owner_liveness_after_fatal_completion_irq_masked();
        }
        crate::arch::aarch64::restore_daif(saved_daif);
        result
    };
    #[cfg(not(feature = "storage-server-owner-liveness-runtime"))]
    let finish_result =
        with_broker_irq_masked(|| storage_broker::finish_service(request, code, read_data));
    match finish_result {
        Ok(()) => {
            COMPLETIONS.fetch_add(1, Ordering::Relaxed);
        }
        Err(RequestError::ServiceAbandoned) => {
            ABANDONED_SERVICES.fetch_add(1, Ordering::Relaxed);
        }
        Err(_) => panic!("M55 storage broker rejected its running completion"),
    }
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    crate::scheduler::wake_object_waiters_for_storage();
    crate::arch::aarch64::timer::request_reschedule();
    crate::arch::aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "storage-server-async-recovery-runtime")]
pub fn record_async_acquire_wait() {
    if !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    ASYNC_ACQUIRE_WAITS.fetch_add(1, Ordering::Relaxed);
    let dispatches = crate::scheduler::snapshot().timer_dispatches;
    if ASYNC_LAST_ACQUIRE_DISPATCH.swap(dispatches, Ordering::AcqRel) != dispatches {
        ASYNC_ACQUIRE_DISPATCH_CHANGES.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(feature = "storage-server-recovery-runtime")]
fn prepare_recovery_fault(request: bndr_abi::StorageBlockRequest, epoch: u64) {
    const PREFIX_LBA: u64 = bndr_abi::APP_DATA_VOLUME_SECTORS - 1;
    const WRITE_SELECTOR_LBA: u64 = bndr_abi::APP_DATA_VOLUME_SECTORS - 2;
    const READ_SELECTOR_LBA: u64 = bndr_abi::APP_DATA_VOLUME_SECTORS - 3;
    const FLUSH_SELECTOR_LBA: u64 = bndr_abi::APP_DATA_VOLUME_SECTORS - 4;
    const PREFIX_SEEN: u8 = 1;
    const ARM_WRITE: u8 = 2;
    const ARM_READ: u8 = 3;
    const ARM_FLUSH: u8 = 4;

    #[cfg(feature = "storage-server-fault-policy-runtime")]
    if observe_m60_kernel_permanent_request(request, epoch) {
        return;
    }

    if epoch == 0 {
        panic!("M56 recovery fault control observed an unbound request");
    }
    let mut state = FAULT_CONTROL_STATE.load(Ordering::Acquire);
    if state != 0 && FAULT_CONTROL_EPOCH.load(Ordering::Acquire) != epoch {
        // A server can die after the prefix or selector but before publishing
        // the target operation. Never carry that authority into a replacement
        // epoch: clear it and parse the current request from the idle state.
        FAULT_CONTROL_STATE.store(0, Ordering::Release);
        FAULT_CONTROL_EPOCH.store(0, Ordering::Release);
        state = 0;
    }

    let single_read_at = |lba| {
        request.operation() == StorageBlockOperation::Read
            && request.relative_lba() == lba
            && request.sector_count() == 1
    };
    match state {
        0 => {
            if single_read_at(PREFIX_LBA) {
                FAULT_CONTROL_EPOCH.store(epoch, Ordering::Release);
                FAULT_CONTROL_STATE.store(PREFIX_SEEN, Ordering::Release);
            }
        }
        PREFIX_SEEN => {
            let armed = if single_read_at(WRITE_SELECTOR_LBA) {
                Some(ARM_WRITE)
            } else if single_read_at(READ_SELECTOR_LBA) {
                Some(ARM_READ)
            } else if single_read_at(FLUSH_SELECTOR_LBA) {
                Some(ARM_FLUSH)
            } else {
                None
            };
            if let Some(armed) = armed {
                FAULT_CONTROL_SEQUENCES.fetch_add(1, Ordering::Relaxed);
                FAULT_CONTROL_STATE.store(armed, Ordering::Release);
            } else {
                // A lone prefix read can occur during exhaustive virgin-media
                // inspection. It is not authority to perturb the next I/O.
                FAULT_CONTROL_STATE.store(0, Ordering::Release);
                FAULT_CONTROL_EPOCH.store(0, Ordering::Release);
            }
        }
        armed @ (ARM_WRITE | ARM_READ | ARM_FLUSH) => {
            let expected = match armed {
                ARM_WRITE => StorageBlockOperation::Write,
                ARM_READ => StorageBlockOperation::Read,
                ARM_FLUSH => StorageBlockOperation::Flush,
                _ => unreachable!(),
            };
            if request.operation() != expected {
                // The control protocol is test-only but still user driven.
                // A malformed sequence must revoke its pending authority, not
                // turn an EL0 mistake into a kernel panic.
                FAULT_CONTROL_STATE.store(0, Ordering::Release);
                FAULT_CONTROL_EPOCH.store(0, Ordering::Release);
                return;
            }
            let expected_kind = match expected {
                StorageBlockOperation::Read => BlockRequestKind::Read,
                StorageBlockOperation::Write => BlockRequestKind::Write,
                StorageBlockOperation::Flush => BlockRequestKind::Flush,
            };
            match storage::suppress_next_notification_for_recovery_test(expected_kind) {
                Ok(()) => {}
                Err(
                    StorageError::RecoveryRequired
                    | StorageError::InterruptFailure
                    | StorageError::IrqNotArmed,
                ) => {
                    // A device/configuration fault can race broker submit and
                    // monitor accept. Let the normal I/O path report the
                    // session-fatal recovery status without leaving control
                    // authority armed for the replacement owner.
                    FAULT_CONTROL_STATE.store(0, Ordering::Release);
                    FAULT_CONTROL_EPOCH.store(0, Ordering::Release);
                    return;
                }
                Err(_) => panic!("M56 could not arm one-shot notification loss"),
            }
            let sequence_code = match expected {
                StorageBlockOperation::Write => 1,
                StorageBlockOperation::Read => 2,
                StorageBlockOperation::Flush => 3,
            };
            let sequence = FAULT_SEQUENCE.load(Ordering::Relaxed);
            FAULT_SEQUENCE.store(sequence.wrapping_shl(2) | sequence_code, Ordering::Relaxed);
            match expected {
                StorageBlockOperation::Read => {
                    FAULT_INJECTED_READS.fetch_add(1, Ordering::Relaxed);
                }
                StorageBlockOperation::Write => {
                    FAULT_INJECTED_WRITES.fetch_add(1, Ordering::Relaxed);
                }
                StorageBlockOperation::Flush => {
                    FAULT_INJECTED_FLUSHES.fetch_add(1, Ordering::Relaxed);
                }
            }
            FAULT_CONTROL_STATE.store(0, Ordering::Release);
            FAULT_CONTROL_EPOCH.store(0, Ordering::Release);
        }
        _ => panic!("M56 recovery fault control escaped its fixed state machine"),
    }
}

/// Records the seventh StorageServer's first ordinary single-sector read as
/// M60's permanent-failure observation. The kernel recovery coordinator has
/// already armed read loss plus persistent RebuildQueue failure before this
/// owner can be acquired; this request path cannot arm either fault.
///
/// Every epoch-seven request bypasses the inherited EL0 prefix/selector parser.
/// A malformed first request closes recovery admission and is reported through
/// the normal session-fatal path instead of letting EL0 trigger a kernel panic.
#[cfg(feature = "storage-server-fault-policy-runtime")]
fn observe_m60_kernel_permanent_request(
    request: bndr_abi::StorageBlockRequest,
    epoch: u64,
) -> bool {
    const TRANSIENT_FAULT_SEQUENCE_WRFWRF: u64 = 0x06db;
    const PERMANENT_OWNER_EPOCH: u64 = 7;

    if epoch != PERMANENT_OWNER_EPOCH {
        return false;
    }

    let policy = fault_policy_snapshot();
    let broker = broker_snapshot();
    let campaign_ready = broker.bound
        && broker.epoch == PERMANENT_OWNER_EPOCH
        && broker.state == 2
        && broker.pending_sessions == 0
        && policy.state == RecoveryState::Probation
        && policy.active_attempt_generation == 7
        && policy.active_attempt_ordinal == 1
        && policy.consecutive_failures == 0
        && policy.attempts_started == 7
        && policy.physical_successes == 7
        && policy.physical_failures == 0
        && policy.probation_entries == 7
        && policy.probation_successes == 5
        && policy.probation_failures == 1
        && policy.healthy_transitions == 5
        && RECOVERY_ATTEMPTS.load(Ordering::Acquire) == 7
        && RECOVERY_SUCCESSES.load(Ordering::Acquire) == 6
        && RECOVERY_FAILURES.load(Ordering::Acquire) == 1
        && RECOVERY_REARM_ABORTS.load(Ordering::Acquire) == 1
        && FAULT_CONTROL_SEQUENCES.load(Ordering::Acquire) == 6
        && FAULT_CONTROL_STATE.load(Ordering::Acquire) == 0
        && FAULT_CONTROL_EPOCH.load(Ordering::Acquire) == 0
        && FAULT_SEQUENCE.load(Ordering::Acquire) == TRANSIENT_FAULT_SEQUENCE_WRFWRF
        && FAULT_INJECTED_READS.load(Ordering::Acquire) == 2
        && FAULT_INJECTED_WRITES.load(Ordering::Acquire) == 2
        && FAULT_INJECTED_FLUSHES.load(Ordering::Acquire) == 2
        && KERNEL_PERMANENT_FAULT_ARMS.load(Ordering::Acquire) == 1;
    let owner_requests = KERNEL_PERMANENT_OWNER_REQUESTS.fetch_add(1, Ordering::AcqRel) + 1;
    if !campaign_ready
        || owner_requests != 1
        || request.operation() != StorageBlockOperation::Read
        || request.sector_count() != 1
        || KERNEL_PERMANENT_READS.load(Ordering::Acquire) != 0
    {
        storage::require_recovery();
        return true;
    }

    KERNEL_PERMANENT_READS.store(1, Ordering::Release);
    FAULT_SEQUENCE.store(
        TRANSIENT_FAULT_SEQUENCE_WRFWRF.wrapping_shl(2) | 2,
        Ordering::Release,
    );
    FAULT_INJECTED_READS.fetch_add(1, Ordering::Relaxed);
    true
}

/// Arms M60's permanent campaign while the seventh physical recovery is
/// committing, before the ownerless broker becomes acquirable. EL0 cannot
/// reach this hook or influence whether it runs; failure leaves the IRQ,
/// admission gate, and broker on the existing fail-closed rollback path.
#[cfg(feature = "storage-server-fault-policy-runtime")]
fn prearm_m60_kernel_permanent_campaign(recovery_attempt: u64) -> bool {
    const TRANSIENT_FAULT_SEQUENCE_WRFWRF: u64 = 0x06db;
    const PERMANENT_RECOVERY_ATTEMPT: u64 = 7;

    if recovery_attempt != PERMANENT_RECOVERY_ATTEMPT {
        return true;
    }

    let policy = fault_policy_snapshot();
    let broker = broker_snapshot();
    let campaign_ready = !broker.bound
        && broker.epoch == 0
        && broker.next_epoch == 7
        && broker.state == 4
        && broker.pending_sessions == 0
        && !broker.abandoned_running
        && policy.state == RecoveryState::Probation
        && policy.active_attempt_generation == 7
        && policy.active_attempt_ordinal == 1
        && policy.consecutive_failures == 0
        && policy.attempts_started == 7
        && policy.physical_successes == 7
        && policy.physical_failures == 0
        && policy.probation_entries == 7
        && policy.probation_successes == 5
        && policy.probation_failures == 1
        && policy.healthy_transitions == 5
        && RECOVERY_ATTEMPTS.load(Ordering::Acquire) == 7
        && RECOVERY_SUCCESSES.load(Ordering::Acquire) == 5
        && RECOVERY_FAILURES.load(Ordering::Acquire) == 1
        && RECOVERY_REARM_ABORTS.load(Ordering::Acquire) == 1
        && FAULT_CONTROL_SEQUENCES.load(Ordering::Acquire) == 6
        && FAULT_CONTROL_STATE.load(Ordering::Acquire) == 0
        && FAULT_CONTROL_EPOCH.load(Ordering::Acquire) == 0
        && FAULT_SEQUENCE.load(Ordering::Acquire) == TRANSIENT_FAULT_SEQUENCE_WRFWRF
        && FAULT_INJECTED_READS.load(Ordering::Acquire) == 2
        && FAULT_INJECTED_WRITES.load(Ordering::Acquire) == 2
        && FAULT_INJECTED_FLUSHES.load(Ordering::Acquire) == 2
        && KERNEL_PERMANENT_OWNER_REQUESTS.load(Ordering::Acquire) == 0
        && KERNEL_PERMANENT_FAULT_ARMS.load(Ordering::Acquire) == 0
        && KERNEL_PERMANENT_READS.load(Ordering::Acquire) == 0;
    if !campaign_ready || storage::arm_m60_permanent_read_fault_for_test().is_err() {
        return false;
    }
    KERNEL_PERMANENT_FAULT_ARMS.store(1, Ordering::Release);
    true
}

/// Completes one fail-stop recovery attempt after the poisoned StorageServer
/// owner has exited. The physical device, queue, IRQ route, and broker epoch
/// remain kernel-owned throughout: no EL0 capability can request or bypass a
/// reset, and the old bound capability is never made writable again.
#[cfg(not(feature = "storage-server-async-recovery-runtime"))]
pub fn service_recovery() {
    if crate::arch::aarch64::irq_is_masked() {
        panic!("M56 storage recovery monitor entered with local IRQ masked");
    }
    let mut broker = broker_snapshot();
    if !broker.bound && broker.state == 0 && storage::recovery_required() {
        with_broker_irq_masked(storage_broker::require_ownerless_recovery).unwrap_or_else(|_| {
            panic!("storage broker could not reconcile an ownerless physical recovery latch")
        });
        broker = broker_snapshot();
    }
    if broker.state != 4 || broker.bound {
        return;
    }

    let _recovery_attempt = RECOVERY_ATTEMPTS.fetch_add(1, Ordering::Relaxed) + 1;
    storage::require_recovery();
    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if storage::irq_id() != Some(block_irq.id)
        || crate::interrupt::disable_block_irq(block_irq).is_err()
    {
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let mut clock = PhysicalCounter;
    let budget = crate::arch::aarch64::timer::frequency_hz();
    let physical_recovery = deadline_after(&mut clock, budget)
        .ok_or(StorageError::NotReady)
        .and_then(|deadline| storage::recover_after_timeout(&mut clock, deadline));
    if physical_recovery.is_err() {
        storage::require_recovery();
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    #[cfg(feature = "storage-server-repeated-recovery-runtime")]
    if _recovery_attempt == 4 {
        if !crate::interrupt::abort_next_block_irq_rearm_commit_for_test() {
            panic!("M57 could not arm its one-shot IRQ rearm commit abort");
        }
        RECOVERY_REARM_ABORTS.fetch_add(1, Ordering::Relaxed);
    }

    // Keep DAIF masked from GIC enable and source-tail audit through the
    // broker commit. No replacement can observe an Idle broker before the
    // physical recovery latch and IRQ route have committed coherently.
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let rearmed = crate::interrupt::reenable_block_irq_masked(block_irq);
    if rearmed.is_err() {
        #[cfg(feature = "storage-server-repeated-recovery-runtime")]
        if _recovery_attempt == 4 {
            let irq = storage::irq_snapshot();
            let broker = broker_snapshot();
            if irq.armed
                || irq.rearm_prepared
                || !irq.failed
                || !irq.recovery_required
                || broker.bound
                || broker.state != 4
                || crate::interrupt::block_irq_rearm_commit_abort_armed()
            {
                panic!("M57 IRQ rearm abort did not remain fail closed");
            }
            if RECOVERY_RETRY_PENDING
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                panic!("M57 fail-closed retry evidence was already pending");
            }
        }
        crate::arch::aarch64::restore_daif(saved_daif);
        storage::require_recovery();
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let admission_opened = storage::open_recovery_admission(block_irq.id).is_ok();
    let broker_committed = admission_opened
        && !storage::recovery_required()
        && with_broker_irq_masked(storage_broker::complete_recovery).is_ok();
    if !broker_committed
        && let Err(error) = crate::interrupt::rollback_block_irq_rearm_masked(block_irq)
    {
        panic!(
            "storage broker recovery commit failed and IRQ rollback was unsafe: {}",
            error.as_str()
        );
    }
    crate::arch::aarch64::restore_daif(saved_daif);
    if !broker_committed {
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    #[cfg(feature = "storage-server-repeated-recovery-runtime")]
    if RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel) {
        RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed);
    }
    RECOVERY_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    crate::scheduler::wake_object_waiters_for_storage();
    crate::arch::aarch64::timer::request_reschedule();
    crate::arch::aarch64::restore_daif(saved_daif);
}

/// Advances M58 recovery by at most one physical-driver phase. Every pending
/// result returns with DAIF restored and without retaining a device borrow, so
/// periodic timer IRQs, unrelated workers, and the replacement StorageServer
/// can execute while the block SPI and admission gate remain fail closed.
#[cfg(all(
    feature = "storage-server-async-recovery-runtime",
    not(feature = "storage-server-fault-policy-runtime")
))]
pub fn service_recovery() {
    use crate::driver::virtio::block::CooperativeRecoveryProgress;

    if crate::arch::aarch64::irq_is_masked() {
        panic!("M58 async storage recovery monitor entered with local IRQ masked");
    }
    let mut broker = broker_snapshot();
    if !broker.bound && broker.state == 0 && storage::recovery_required() {
        with_broker_irq_masked(storage_broker::require_ownerless_recovery).unwrap_or_else(|_| {
            panic!("storage broker could not reconcile an ownerless physical recovery latch")
        });
        broker = broker_snapshot();
    }
    if broker.state != 4 || broker.bound {
        if storage::async_recovery_active() || ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
            panic!("M58 async recovery escaped the ownerless broker barrier");
        }
        return;
    }
    if broker.pending_sessions != 0 || broker.abandoned_running {
        panic!("M58 async recovery entered before the old broker session retired");
    }

    storage::require_recovery();
    let coordinator_active = ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire);
    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        if coordinator_active || storage::async_recovery_active() {
            panic!("M58 block IRQ registration disappeared during recovery");
        }
        RECOVERY_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if storage::irq_id() != Some(block_irq.id) {
        if coordinator_active || storage::async_recovery_active() {
            panic!("M58 block IRQ identity changed during recovery");
        }
        RECOVERY_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    if !storage::async_recovery_active() {
        if ASYNC_COORDINATOR_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("M58 recovery coordinator lost its physical phase");
        }
        let recovery_attempt = RECOVERY_ATTEMPTS.fetch_add(1, Ordering::Relaxed) + 1;
        ASYNC_ACTIVE_ATTEMPT.store(recovery_attempt, Ordering::Release);
        let saved_daif = crate::arch::aarch64::save_and_mask_irq();
        let control_started = crate::arch::aarch64::timer::counter_value();
        let disabled = crate::interrupt::disable_block_irq_masked(block_irq);
        record_async_control_masked_since(control_started);
        crate::arch::aarch64::restore_daif(saved_daif);
        if disabled.is_err() {
            ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
            ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
            RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let scheduler = crate::scheduler::snapshot();
        ASYNC_BASE_TIMER_DISPATCHES.store(scheduler.timer_dispatches, Ordering::Release);
        ASYNC_BASE_WORKER_0.store(scheduler.worker_work[0], Ordering::Release);
        ASYNC_BASE_WORKER_1.store(scheduler.worker_work[1], Ordering::Release);
        ASYNC_BASE_ACQUIRE_WAITS.store(
            ASYNC_ACQUIRE_WAITS.load(Ordering::Acquire),
            Ordering::Release,
        );
        ASYNC_BASE_ACQUIRE_DISPATCH_CHANGES.store(
            ASYNC_ACQUIRE_DISPATCH_CHANGES.load(Ordering::Acquire),
            Ordering::Release,
        );
        ASYNC_RECOVERY_STARTS.fetch_add(1, Ordering::Relaxed);

        let mut clock = PhysicalCounter;
        let budget = crate::arch::aarch64::timer::frequency_hz();
        let started = deadline_after(&mut clock, budget)
            .ok_or(StorageError::NotReady)
            .and_then(|deadline| storage::begin_async_recovery(&mut clock, deadline));
        match started {
            Ok(CooperativeRecoveryProgress::Pending(_)) => return,
            Ok(CooperativeRecoveryProgress::Complete) => {
                panic!("M58 cooperative recovery completed without a scheduling boundary")
            }
            Err(_) => {
                storage::require_recovery();
                ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
                ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
                RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    } else if !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire)
        || ASYNC_ACTIVE_ATTEMPT.load(Ordering::Acquire) == 0
    {
        panic!("M58 physical recovery ran without an outer coordinator attempt");
    }

    let mut clock = PhysicalCounter;
    match storage::poll_async_recovery(&mut clock) {
        Ok(CooperativeRecoveryProgress::Pending(_)) => return,
        Err(_) => {
            storage::require_recovery();
            ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
            ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
            RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
            return;
        }
        Ok(CooperativeRecoveryProgress::Complete) => {}
    }

    ASYNC_PHYSICAL_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
    record_async_progress_window();
    let recovery_attempt = ASYNC_ACTIVE_ATTEMPT.load(Ordering::Acquire);
    if recovery_attempt == 0 {
        panic!("M58 physical completion lost its coordinator attempt identity");
    }
    if recovery_attempt == 4 {
        if !crate::interrupt::abort_next_block_irq_rearm_commit_for_test() {
            panic!("M58 could not arm its inherited one-shot IRQ rearm commit abort");
        }
        RECOVERY_REARM_ABORTS.fetch_add(1, Ordering::Relaxed);
    }

    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let control_started = crate::arch::aarch64::timer::counter_value();
    let rearmed = crate::interrupt::reenable_block_irq_masked(block_irq);
    if rearmed.is_err() {
        if recovery_attempt == 4 {
            let irq = storage::irq_snapshot();
            let broker = broker_snapshot();
            if irq.armed
                || irq.rearm_prepared
                || !irq.failed
                || !irq.recovery_required
                || broker.bound
                || broker.state != 4
                || crate::interrupt::block_irq_rearm_commit_abort_armed()
            {
                panic!("M58 inherited IRQ rearm abort did not remain fail closed");
            }
            if RECOVERY_RETRY_PENDING
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                panic!("M58 fail-closed retry evidence was already pending");
            }
        }
        record_async_control_masked_since(control_started);
        crate::arch::aarch64::restore_daif(saved_daif);
        storage::require_recovery();
        ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
        ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let broker_committed = !storage::recovery_required()
        && with_broker_irq_masked(storage_broker::complete_recovery).is_ok();
    if !broker_committed {
        if let Err(error) = crate::interrupt::rollback_block_irq_rearm_masked(block_irq) {
            panic!(
                "M58 broker recovery commit failed and IRQ rollback was unsafe: {}",
                error.as_str()
            );
        }
    } else if let Err(error) = storage::open_recovery_admission(block_irq.id) {
        panic!(
            "M58 broker committed before the recovery admission gate could open: {}",
            error.as_str()
        );
    }
    record_async_control_masked_since(control_started);
    ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
    ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
    crate::arch::aarch64::restore_daif(saved_daif);
    if !broker_committed {
        RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    if RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel) {
        RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed);
    }
    RECOVERY_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    crate::scheduler::wake_object_waiters_for_storage();
    crate::arch::aarch64::timer::request_reschedule();
    crate::arch::aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn active_fault_policy_ticket(expected: RecoveryState) -> AttemptTicket {
    let snapshot = fault_policy_snapshot();
    if snapshot.state != expected
        || snapshot.active_attempt_generation == 0
        || snapshot.active_attempt_ordinal == 0
    {
        panic!("M60 fault policy lost its active attempt");
    }
    AttemptTicket {
        generation: snapshot.active_attempt_generation,
        ordinal: snapshot.active_attempt_ordinal,
    }
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn broker_is_ownerless_recovery(broker: storage_broker::BrokerSnapshot) -> bool {
    !broker.bound && broker.state == 4 && broker.pending_sessions == 0 && !broker.abandoned_running
}

/// Attempts the only legal M60 broker Offline publication.
///
/// The GIC rollback, logical latch audit, request/DMA terminal proof, and
/// broker transition share one DAIF-masked window. Returning `false` means a
/// physical cleanup is still required; no weaker Offline state is exposed.
#[cfg(feature = "storage-server-fault-policy-runtime")]
fn publish_verified_terminal_offline() -> bool {
    storage::require_recovery();
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        panic!("M60 terminal quarantine lost the registered block IRQ");
    };
    if storage::irq_id() != Some(block_irq.id) {
        panic!("M60 terminal quarantine found a mismatched block IRQ");
    }
    crate::interrupt::rollback_block_irq_rearm_masked(block_irq)
        .unwrap_or_else(|error| panic!("M60 terminal IRQ quarantine failed: {}", error.as_str()));
    TERMINAL_IRQ_ROLLBACKS.fetch_add(1, Ordering::Relaxed);

    let irq = storage::irq_snapshot();
    let dma = storage::terminal_dma_snapshot()
        .unwrap_or_else(|error| panic!("M60 terminal DMA audit failed: {}", error.as_str()));
    let broker = broker_snapshot();
    if broker.device_offline {
        crate::arch::aarch64::restore_daif(saved_daif);
        return true;
    }
    if !broker_is_ownerless_recovery(broker) {
        crate::arch::aarch64::restore_daif(saved_daif);
        return false;
    }
    let safe = !irq.armed
        && !irq.rearm_prepared
        && irq.failed
        && irq.recovery_required
        && storage::recovery_admission_closed()
        && !storage::async_recovery_active()
        && !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire)
        && dma.requests_terminal;
    #[cfg(not(feature = "storage-server-terminal-quarantine-runtime"))]
    if !safe {
        crate::arch::aarch64::restore_daif(saved_daif);
        return false;
    }
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    match terminal_proof_gate_irq_masked(|gate| gate.evaluate(safe))
        .unwrap_or_else(|error| panic!("M62 terminal-proof decision failed: {error:?}"))
    {
        TerminalProofDecision::Unsafe | TerminalProofDecision::RunFallback => {
            crate::arch::aarch64::restore_daif(saved_daif);
            return false;
        }
        TerminalProofDecision::Publish => {}
    }
    with_broker_irq_masked(storage_broker::enter_device_offline)
        .unwrap_or_else(|_| panic!("M60 broker rejected its verified terminal Offline transition"));
    TERMINAL_DMA_VERIFICATIONS.fetch_add(1, Ordering::Relaxed);
    crate::scheduler::wake_object_waiters_for_storage();
    crate::arch::aarch64::timer::request_reschedule();
    crate::arch::aarch64::restore_daif(saved_daif);
    true
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn finish_terminal_quarantine(
    progress: Result<crate::driver::virtio::block::CooperativeRecoveryProgress, StorageError>,
) {
    use crate::driver::virtio::block::CooperativeRecoveryProgress;

    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    TERMINAL_QUARANTINE_STEPS.fetch_add(1, Ordering::Relaxed);
    match progress {
        Ok(CooperativeRecoveryProgress::Pending(_)) => {
            #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
            TERMINAL_QUARANTINE_PENDING_RETURNS.fetch_add(1, Ordering::Relaxed);
            return;
        }
        Ok(CooperativeRecoveryProgress::Complete) => {
            TERMINAL_QUARANTINE_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
        }
        Err(_) => {
            // A cooperative failure is publishable only if its cleanup reset
            // nevertheless left a terminal request/DMA snapshot. The verifier
            // below rejects ResetTimeout/Poisoned and every partial cleanup.
            TERMINAL_QUARANTINE_PHYSICAL_ERRORS.fetch_add(1, Ordering::Relaxed);
        }
    }
    TERMINAL_QUARANTINE_ACTIVE.store(false, Ordering::Release);
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    {
        let scheduler = crate::scheduler::snapshot();
        if scheduler.timer_dispatches
            > TERMINAL_QUARANTINE_BASE_TIMER_DISPATCHES.load(Ordering::Acquire)
        {
            TERMINAL_QUARANTINE_TIMER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
        }
        if scheduler.worker_work[0] > TERMINAL_QUARANTINE_BASE_WORKER_0.load(Ordering::Acquire)
            && scheduler.worker_work[1] > TERMINAL_QUARANTINE_BASE_WORKER_1.load(Ordering::Acquire)
        {
            TERMINAL_QUARANTINE_WORKER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
        }
    }
    if !publish_verified_terminal_offline() {
        panic!("M60 terminal quarantine ended without a safe DMA boundary");
    }
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn publish_device_offline_if_ownerless() {
    storage::require_recovery();
    let broker = broker_snapshot();
    if broker.device_offline || !broker_is_ownerless_recovery(broker) {
        return;
    }
    if ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
        panic!("M60 attempted terminal quarantine with a policy recovery active");
    }

    if TERMINAL_QUARANTINE_ACTIVE.load(Ordering::Acquire) {
        if !storage::async_recovery_active() {
            panic!("M60 terminal quarantine lost its cooperative physical phase");
        }
        let mut clock = PhysicalCounter;
        finish_terminal_quarantine(storage::poll_async_recovery(&mut clock));
        return;
    }
    if storage::async_recovery_active() {
        panic!("M60 observed unowned cooperative recovery before Offline publication");
    }
    if publish_verified_terminal_offline() {
        return;
    }

    if TERMINAL_QUARANTINE_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        panic!("M60 terminal quarantine acquired twice");
    }
    TERMINAL_QUARANTINE_STARTS.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
    {
        let scheduler = crate::scheduler::snapshot();
        TERMINAL_QUARANTINE_BASE_TIMER_DISPATCHES
            .store(scheduler.timer_dispatches, Ordering::Release);
        TERMINAL_QUARANTINE_BASE_WORKER_0.store(scheduler.worker_work[0], Ordering::Release);
        TERMINAL_QUARANTINE_BASE_WORKER_1.store(scheduler.worker_work[1], Ordering::Release);
    }
    let mut clock = PhysicalCounter;
    let budget = crate::arch::aarch64::timer::frequency_hz();
    let progress = deadline_after(&mut clock, budget)
        .ok_or(StorageError::NotReady)
        .and_then(|deadline| storage::begin_async_recovery(&mut clock, deadline));
    finish_terminal_quarantine(progress);
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
fn finish_fault_policy_attempt_failure(ticket: AttemptTicket, physical_failure: bool) {
    storage::require_recovery();
    ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
    ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
    RECOVERY_FAILURES.fetch_add(1, Ordering::Relaxed);
    if physical_failure {
        ASYNC_PHYSICAL_FAILURES.fetch_add(1, Ordering::Relaxed);
        record_async_progress_window();
    }
    let now = crate::arch::aarch64::timer::tick_count();
    let disposition = with_fault_policy(|policy| {
        if physical_failure {
            policy.complete_physical_failure(ticket, now)
        } else {
            policy.complete_probation_failure(ticket, now)
        }
    })
    .unwrap_or_else(|_| panic!("M60 failed attempt did not own the policy ticket"));
    record_policy_disposition(disposition);
    if matches!(disposition, FailureDisposition::Offline { .. }) {
        #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
        {
            if !physical_failure {
                panic!("M62 terminal-proof deferral escaped the final physical failure");
            }
            arm_terminal_proof_deferral();
        }
        publish_device_offline_if_ownerless();
    }
}

/// M60 adds a boot-local bounded policy around M58's cooperative engine.
/// Backoff uses the 100 Hz logical timer, never a DAIF-masked delay loop. A
/// physical success remains probationary until one real replacement-owner I/O
/// succeeds; three unconfirmed failures make the broker sticky Offline.
#[cfg(feature = "storage-server-fault-policy-runtime")]
pub fn service_recovery() {
    use crate::driver::virtio::block::CooperativeRecoveryProgress;

    if crate::arch::aarch64::irq_is_masked() {
        panic!("M60 fault-policy recovery monitor entered with local IRQ masked");
    }
    #[cfg(feature = "storage-server-owner-liveness-runtime")]
    if owner_retirement_barrier_active() {
        if storage::async_recovery_active() || ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
            panic!("M61 physical recovery crossed a live owner-retirement barrier");
        }
        return;
    }
    let mut broker = broker_snapshot();
    if !broker.bound && broker.state == 0 && storage::recovery_required() {
        with_broker_irq_masked(storage_broker::require_ownerless_recovery).unwrap_or_else(|_| {
            panic!("M60 broker could not reconcile an ownerless physical recovery latch")
        });
        broker = broker_snapshot();
    }

    let mut policy = fault_policy_snapshot();
    if policy.state == RecoveryState::Offline {
        publish_device_offline_if_ownerless();
        return;
    }
    if broker.state != 4 || broker.bound {
        if storage::async_recovery_active() || ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
            panic!("M60 recovery escaped the ownerless broker barrier");
        }
        return;
    }
    if broker.pending_sessions != 0 || broker.abandoned_running {
        panic!("M60 recovery entered before the old broker session retired");
    }

    storage::require_recovery();
    if policy.state == RecoveryState::Probation {
        let ticket = active_fault_policy_ticket(RecoveryState::Probation);
        let now = crate::arch::aarch64::timer::tick_count();
        let disposition =
            with_fault_policy(|policy| policy.complete_probation_failure(ticket, now))
                .unwrap_or_else(|_| panic!("M60 probation fault lost its active ticket"));
        record_policy_disposition(disposition);
        if matches!(disposition, FailureDisposition::Offline { .. }) {
            publish_device_offline_if_ownerless();
        }
        return;
    }

    if !storage::async_recovery_active() {
        if ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire) {
            panic!("M60 coordinator remained active without a physical phase");
        }
        let before = policy;
        let now = crate::arch::aarch64::timer::tick_count();
        if before.state == RecoveryState::Backoff {
            let deadline = before
                .backoff_deadline
                .unwrap_or_else(|| panic!("M60 Backoff state lost its deadline"));
            let deadline_reached = bndroid_kernel::time::deadline_reached(now, deadline);
            #[cfg(feature = "storage-server-owner-liveness-runtime")]
            if !deadline_reached {
                return;
            }
            if deadline_reached && !backoff_progress_window_ready() {
                return;
            }
        }
        let ticket = match with_fault_policy(|policy| policy.begin_attempt(now)) {
            Ok(ticket) => ticket,
            Err(RejectReason::BackoffPending { .. }) => return,
            Err(RejectReason::Offline) => {
                publish_device_offline_if_ownerless();
                return;
            }
            Err(_) => panic!("M60 fault policy rejected a valid recovery claim"),
        };
        if before.state == RecoveryState::Backoff {
            finish_backoff_progress_window();
        }
        policy = fault_policy_snapshot();
        let recovery_attempt = RECOVERY_ATTEMPTS.fetch_add(1, Ordering::Relaxed) + 1;
        if ticket.generation != recovery_attempt || policy.attempts_started != recovery_attempt {
            panic!("M60 policy and coordinator attempt ledgers diverged");
        }
        ASYNC_ACTIVE_ATTEMPT.store(recovery_attempt, Ordering::Release);
        ASYNC_COORDINATOR_ACTIVE.store(true, Ordering::Release);

        let Some(block_irq) = crate::interrupt::registered_block_irq() else {
            finish_fault_policy_attempt_failure(ticket, true);
            return;
        };
        if storage::irq_id() != Some(block_irq.id) {
            finish_fault_policy_attempt_failure(ticket, true);
            return;
        }
        let saved_daif = crate::arch::aarch64::save_and_mask_irq();
        let control_started = crate::arch::aarch64::timer::counter_value();
        let disabled = crate::interrupt::disable_block_irq_masked(block_irq);
        record_async_control_masked_since(control_started);
        crate::arch::aarch64::restore_daif(saved_daif);
        if disabled.is_err() {
            finish_fault_policy_attempt_failure(ticket, true);
            return;
        }

        let scheduler = crate::scheduler::snapshot();
        ASYNC_BASE_TIMER_DISPATCHES.store(scheduler.timer_dispatches, Ordering::Release);
        ASYNC_BASE_WORKER_0.store(scheduler.worker_work[0], Ordering::Release);
        ASYNC_BASE_WORKER_1.store(scheduler.worker_work[1], Ordering::Release);
        ASYNC_BASE_ACQUIRE_WAITS.store(
            ASYNC_ACQUIRE_WAITS.load(Ordering::Acquire),
            Ordering::Release,
        );
        ASYNC_BASE_ACQUIRE_DISPATCH_CHANGES.store(
            ASYNC_ACQUIRE_DISPATCH_CHANGES.load(Ordering::Acquire),
            Ordering::Release,
        );
        ASYNC_RECOVERY_STARTS.fetch_add(1, Ordering::Relaxed);

        let mut clock = PhysicalCounter;
        let budget = crate::arch::aarch64::timer::frequency_hz();
        let started = deadline_after(&mut clock, budget)
            .ok_or(StorageError::NotReady)
            .and_then(|deadline| storage::begin_async_recovery(&mut clock, deadline));
        match started {
            Ok(CooperativeRecoveryProgress::Pending(_)) => return,
            Ok(CooperativeRecoveryProgress::Complete) => {
                panic!("M60 cooperative recovery completed without a scheduling boundary")
            }
            Err(_) => {
                finish_fault_policy_attempt_failure(ticket, true);
                return;
            }
        }
    } else if !ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire)
        || ASYNC_ACTIVE_ATTEMPT.load(Ordering::Acquire) == 0
    {
        panic!("M60 physical recovery ran without an outer coordinator attempt");
    }

    let ticket = active_fault_policy_ticket(RecoveryState::Recovering);
    let mut clock = PhysicalCounter;
    match storage::poll_async_recovery(&mut clock) {
        Ok(CooperativeRecoveryProgress::Pending(_)) => return,
        Err(_) => {
            finish_fault_policy_attempt_failure(ticket, true);
            return;
        }
        Ok(CooperativeRecoveryProgress::Complete) => {}
    }

    ASYNC_PHYSICAL_COMPLETIONS.fetch_add(1, Ordering::Relaxed);
    record_async_progress_window();
    with_fault_policy(|policy| policy.complete_physical_success(ticket))
        .unwrap_or_else(|_| panic!("M60 physical completion lost its policy ticket"));
    let recovery_attempt = ASYNC_ACTIVE_ATTEMPT.load(Ordering::Acquire);
    if recovery_attempt == 0 {
        panic!("M60 physical completion lost its coordinator identity");
    }
    if recovery_attempt == 4 {
        if !crate::interrupt::abort_next_block_irq_rearm_commit_for_test() {
            panic!("M60 could not arm its inherited IRQ rearm commit abort");
        }
        RECOVERY_REARM_ABORTS.fetch_add(1, Ordering::Relaxed);
    }

    let Some(block_irq) = crate::interrupt::registered_block_irq() else {
        finish_fault_policy_attempt_failure(ticket, false);
        return;
    };
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    let control_started = crate::arch::aarch64::timer::counter_value();
    let rearmed = crate::interrupt::reenable_block_irq_masked(block_irq);
    if rearmed.is_err() {
        if recovery_attempt == 4 {
            let irq = storage::irq_snapshot();
            let broker = broker_snapshot();
            if irq.armed
                || irq.rearm_prepared
                || !irq.failed
                || !irq.recovery_required
                || broker.bound
                || broker.state != 4
                || crate::interrupt::block_irq_rearm_commit_abort_armed()
            {
                panic!("M60 inherited IRQ rearm abort did not remain fail closed");
            }
            if RECOVERY_RETRY_PENDING
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                panic!("M60 fail-closed retry evidence was already pending");
            }
        }
        record_async_control_masked_since(control_started);
        ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
        ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
        crate::arch::aarch64::restore_daif(saved_daif);
        finish_fault_policy_attempt_failure(ticket, false);
        return;
    }

    // M60 removes M58's post-broker fallible admission open. Rearm, admission,
    // and ownerless broker commit all occur under one DAIF-masked window; any
    // failure before the last step can still rollback to the closed state.
    let admission_opened = storage::open_recovery_admission(block_irq.id).is_ok();
    // Attempt seven pre-arms the permanent campaign while the broker is still
    // ownerless. Only after that kernel-owned commit may epoch seven become
    // acquirable; an EL0 request can observe the injected failure but cannot
    // authorize or arm it.
    let permanent_campaign_armed =
        admission_opened && prearm_m60_kernel_permanent_campaign(recovery_attempt);
    let broker_committed = permanent_campaign_armed
        && !storage::recovery_required()
        && with_broker_irq_masked(storage_broker::complete_recovery).is_ok();
    if !broker_committed
        && let Err(error) = crate::interrupt::rollback_block_irq_rearm_masked(block_irq)
    {
        panic!(
            "M60 coordinator commit failed and IRQ rollback was unsafe: {}",
            error.as_str()
        );
    }
    record_async_control_masked_since(control_started);
    ASYNC_ACTIVE_ATTEMPT.store(0, Ordering::Release);
    ASYNC_COORDINATOR_ACTIVE.store(false, Ordering::Release);
    if broker_committed {
        // Publish every ledger consumed by the replacement owner's first
        // request before restoring DAIF. Otherwise epoch seven could run
        // between broker publication and campaign evidence publication and
        // spuriously fail closed despite a valid kernel pre-arm.
        if RECOVERY_RETRY_PENDING.swap(false, Ordering::AcqRel) {
            RECOVERY_FAIL_CLOSED_RETRIES.fetch_add(1, Ordering::Relaxed);
        }
        RECOVERY_SUCCESSES.fetch_add(1, Ordering::Release);
    }
    crate::arch::aarch64::restore_daif(saved_daif);
    if !broker_committed {
        finish_fault_policy_attempt_failure(ticket, false);
        return;
    }
    let saved_daif = crate::arch::aarch64::save_and_mask_irq();
    crate::scheduler::wake_object_waiters_for_storage();
    crate::arch::aarch64::timer::request_reschedule();
    crate::arch::aarch64::restore_daif(saved_daif);
}

#[cfg(feature = "storage-server-async-recovery-runtime")]
fn record_async_control_masked_since(started: u64) {
    let elapsed = crate::arch::aarch64::timer::counter_value().wrapping_sub(started);
    ASYNC_MAX_CONTROL_MASKED_TICKS.fetch_max(elapsed, Ordering::Relaxed);
}

#[cfg(feature = "storage-server-async-recovery-runtime")]
fn record_async_progress_window() {
    let scheduler = crate::scheduler::snapshot();
    if scheduler
        .timer_dispatches
        .saturating_sub(ASYNC_BASE_TIMER_DISPATCHES.load(Ordering::Acquire))
        >= 2
    {
        ASYNC_TIMER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
    }
    if scheduler.worker_work[0] > ASYNC_BASE_WORKER_0.load(Ordering::Acquire)
        && scheduler.worker_work[1] > ASYNC_BASE_WORKER_1.load(Ordering::Acquire)
    {
        ASYNC_WORKER_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
    }
    if ASYNC_ACQUIRE_WAITS
        .load(Ordering::Acquire)
        .saturating_sub(ASYNC_BASE_ACQUIRE_WAITS.load(Ordering::Acquire))
        >= 2
        && ASYNC_ACQUIRE_DISPATCH_CHANGES
            .load(Ordering::Acquire)
            .saturating_sub(ASYNC_BASE_ACQUIRE_DISPATCH_CHANGES.load(Ordering::Acquire))
            >= 2
    {
        ASYNC_EL0_PROGRESS_WINDOWS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        reads: READS.load(Ordering::Acquire),
        writes: WRITES.load(Ordering::Acquire),
        read_sectors: READ_SECTORS.load(Ordering::Acquire),
        write_sectors: WRITE_SECTORS.load(Ordering::Acquire),
        flushes: FLUSHES.load(Ordering::Acquire),
        completions: COMPLETIONS.load(Ordering::Acquire),
        abandoned_services: ABANDONED_SERVICES.load(Ordering::Acquire),
        errors: ERRORS.load(Ordering::Acquire),
        recovery_attempts: RECOVERY_ATTEMPTS.load(Ordering::Acquire),
        recovery_successes: RECOVERY_SUCCESSES.load(Ordering::Acquire),
        recovery_failures: RECOVERY_FAILURES.load(Ordering::Acquire),
        recovery_rearm_aborts: RECOVERY_REARM_ABORTS.load(Ordering::Acquire),
        recovery_fail_closed_retries: RECOVERY_FAIL_CLOSED_RETRIES.load(Ordering::Acquire),
        recovery_retry_pending: RECOVERY_RETRY_PENDING.load(Ordering::Acquire),
        async_recovery_starts: ASYNC_RECOVERY_STARTS.load(Ordering::Acquire),
        async_physical_completions: ASYNC_PHYSICAL_COMPLETIONS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        async_physical_failures: ASYNC_PHYSICAL_FAILURES.load(Ordering::Acquire),
        async_coordinator_active: ASYNC_COORDINATOR_ACTIVE.load(Ordering::Acquire),
        async_active_attempt: ASYNC_ACTIVE_ATTEMPT.load(Ordering::Acquire),
        async_timer_progress_windows: ASYNC_TIMER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_worker_progress_windows: ASYNC_WORKER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_el0_progress_windows: ASYNC_EL0_PROGRESS_WINDOWS.load(Ordering::Acquire),
        async_acquire_waits: ASYNC_ACQUIRE_WAITS.load(Ordering::Acquire),
        async_acquire_dispatch_changes: ASYNC_ACQUIRE_DISPATCH_CHANGES.load(Ordering::Acquire),
        async_max_control_masked_ticks: ASYNC_MAX_CONTROL_MASKED_TICKS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        backoff_timer_progress_windows: BACKOFF_TIMER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        backoff_worker_progress_windows: BACKOFF_WORKER_PROGRESS_WINDOWS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_quarantine_active: TERMINAL_QUARANTINE_ACTIVE.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_quarantine_starts: TERMINAL_QUARANTINE_STARTS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_quarantine_completions: TERMINAL_QUARANTINE_COMPLETIONS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_quarantine_physical_errors: TERMINAL_QUARANTINE_PHYSICAL_ERRORS
            .load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_irq_rollbacks: TERMINAL_IRQ_ROLLBACKS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        terminal_dma_verifications: TERMINAL_DMA_VERIFICATIONS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
        terminal_quarantine_steps: TERMINAL_QUARANTINE_STEPS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
        terminal_quarantine_pending_returns: TERMINAL_QUARANTINE_PENDING_RETURNS
            .load(Ordering::Acquire),
        #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
        terminal_quarantine_timer_progress_windows: TERMINAL_QUARANTINE_TIMER_PROGRESS_WINDOWS
            .load(Ordering::Acquire),
        #[cfg(feature = "storage-server-terminal-quarantine-runtime")]
        terminal_quarantine_worker_progress_windows: TERMINAL_QUARANTINE_WORKER_PROGRESS_WINDOWS
            .load(Ordering::Acquire),
        outcome_unknown_completions: OUTCOME_UNKNOWN_COMPLETIONS.load(Ordering::Acquire),
        requires_reset_completions: REQUIRES_RESET_COMPLETIONS.load(Ordering::Acquire),
        fault_control_sequences: FAULT_CONTROL_SEQUENCES.load(Ordering::Acquire),
        fault_control_state: FAULT_CONTROL_STATE.load(Ordering::Acquire),
        fault_control_epoch: FAULT_CONTROL_EPOCH.load(Ordering::Acquire),
        fault_sequence: FAULT_SEQUENCE.load(Ordering::Acquire),
        fault_injected_reads: FAULT_INJECTED_READS.load(Ordering::Acquire),
        fault_injected_writes: FAULT_INJECTED_WRITES.load(Ordering::Acquire),
        fault_injected_flushes: FAULT_INJECTED_FLUSHES.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        kernel_permanent_owner_requests: KERNEL_PERMANENT_OWNER_REQUESTS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        kernel_permanent_fault_arms: KERNEL_PERMANENT_FAULT_ARMS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        kernel_permanent_reads: KERNEL_PERMANENT_READS.load(Ordering::Acquire),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        probation_io_failures: PROBATION_IO_FAILURES.load(Ordering::Acquire),
    }
}
