//! Kernel-authenticated M73 event-supervisor evidence.
//!
//! Init reports only bounded state transitions. The report syscall grants no
//! capability; this ledger cross-checks every transition against the kernel's
//! own generation-qualified process counters and current StorageServer PID.

use core::sync::atomic::{AtomicU64, Ordering};

use bndr_abi::{
    SERVICE_SUPERVISOR_REPORT_ACTIVE, SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW,
    SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN, SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE,
    SERVICE_SUPERVISOR_REPORT_STOP_DRAINED, SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED,
};

const PRODUCT_SERVICE_COUNT: u64 = 5;
const MIN_ACTIVE_BATCHES: u64 = 3;
const REQUIRED_ROTATIONS: u64 = 2;
const REQUIRED_CANCEL_PENDING: u64 = 4;

const PHASE_INITIAL: u64 = 0;
const PHASE_ACTIVE: u64 = 1;
const PHASE_ROTATING: u64 = 2;
const PHASE_CANCEL_WINDOW: u64 = 3;
const PHASE_STOP_REQUESTED: u64 = 4;
const PHASE_STOP_DRAINED: u64 = 5;

static PHASE: AtomicU64 = AtomicU64::new(PHASE_INITIAL);
static CALLS: AtomicU64 = AtomicU64::new(0);
static SUCCESSES: AtomicU64 = AtomicU64::new(0);
static ARGUMENT_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static PERMISSION_DENIALS: AtomicU64 = AtomicU64::new(0);
static STATE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static UI_QUERY_CALLS: AtomicU64 = AtomicU64::new(0);
static UI_QUERY_WAITS: AtomicU64 = AtomicU64::new(0);
static UI_QUERY_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static ACTIVE_BATCHES: AtomicU64 = AtomicU64::new(0);
static ROTATIONS_STARTED: AtomicU64 = AtomicU64::new(0);
static ROTATIONS_COMPLETED: AtomicU64 = AtomicU64::new(0);
static ROTATION_OLD_PIDS: [AtomicU64; REQUIRED_ROTATIONS as usize] =
    [const { AtomicU64::new(0) }; REQUIRED_ROTATIONS as usize];
static ROTATION_NEW_PIDS: [AtomicU64; REQUIRED_ROTATIONS as usize] =
    [const { AtomicU64::new(0) }; REQUIRED_ROTATIONS as usize];
static CANCEL_BATCH: AtomicU64 = AtomicU64::new(0);
static CANCEL_PENDING: AtomicU64 = AtomicU64::new(0);
static STOP_BATCH: AtomicU64 = AtomicU64::new(0);
static STOP_PENDING: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessContext {
    pub current_storage_pid: u64,
    pub created: u64,
    pub exited: u64,
    pub reaped: u64,
    pub terminated_exited: u64,
    pub terminated_faulted: u64,
    pub terminated_killed: u64,
    pub live: u64,
    pub storage_server_live: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportError {
    InvalidArgument,
    InvalidState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub phase: u64,
    pub calls: u64,
    pub successes: u64,
    pub argument_rejections: u64,
    pub permission_denials: u64,
    pub state_rejections: u64,
    pub ui_query_calls: u64,
    pub ui_query_waits: u64,
    pub ui_query_successes: u64,
    pub active_batches: u64,
    pub rotations_started: u64,
    pub rotations_completed: u64,
    pub rotation_old_pids: [u64; REQUIRED_ROTATIONS as usize],
    pub rotation_new_pids: [u64; REQUIRED_ROTATIONS as usize],
    pub cancel_batch: u64,
    pub cancel_pending: u64,
    pub stop_batch: u64,
    pub stop_pending: u64,
    pub complete: bool,
}

pub fn snapshot() -> Snapshot {
    let phase = PHASE.load(Ordering::Acquire);
    let calls = CALLS.load(Ordering::Acquire);
    let successes = SUCCESSES.load(Ordering::Acquire);
    let argument_rejections = ARGUMENT_REJECTIONS.load(Ordering::Acquire);
    let permission_denials = PERMISSION_DENIALS.load(Ordering::Acquire);
    let state_rejections = STATE_REJECTIONS.load(Ordering::Acquire);
    let ui_query_calls = UI_QUERY_CALLS.load(Ordering::Acquire);
    let ui_query_waits = UI_QUERY_WAITS.load(Ordering::Acquire);
    let ui_query_successes = UI_QUERY_SUCCESSES.load(Ordering::Acquire);
    let active_batches = ACTIVE_BATCHES.load(Ordering::Acquire);
    let rotations_started = ROTATIONS_STARTED.load(Ordering::Acquire);
    let rotations_completed = ROTATIONS_COMPLETED.load(Ordering::Acquire);
    let rotation_old_pids =
        core::array::from_fn(|index| ROTATION_OLD_PIDS[index].load(Ordering::Acquire));
    let rotation_new_pids =
        core::array::from_fn(|index| ROTATION_NEW_PIDS[index].load(Ordering::Acquire));
    let cancel_batch = CANCEL_BATCH.load(Ordering::Acquire);
    let cancel_pending = CANCEL_PENDING.load(Ordering::Acquire);
    let stop_batch = STOP_BATCH.load(Ordering::Acquire);
    let stop_pending = STOP_PENDING.load(Ordering::Acquire);
    let complete = phase == PHASE_STOP_DRAINED
        && calls == 9
        && successes == 8
        && argument_rejections == 1
        && permission_denials == 0
        && state_rejections == 0
        && ui_query_calls >= 2
        && ui_query_waits >= 1
        && ui_query_successes == 1
        && ui_query_calls == ui_query_waits + ui_query_successes
        && active_batches >= MIN_ACTIVE_BATCHES
        && rotations_started == REQUIRED_ROTATIONS
        && rotations_completed == REQUIRED_ROTATIONS
        && rotation_old_pids.iter().all(|pid| *pid != 0)
        && rotation_new_pids.iter().all(|pid| *pid != 0)
        && rotation_new_pids[0] == rotation_old_pids[1]
        && cancel_batch > active_batches
        && cancel_pending == REQUIRED_CANCEL_PENDING
        && stop_batch == cancel_batch
        && stop_pending == cancel_pending;
    Snapshot {
        phase,
        calls,
        successes,
        argument_rejections,
        permission_denials,
        state_rejections,
        ui_query_calls,
        ui_query_waits,
        ui_query_successes,
        active_batches,
        rotations_started,
        rotations_completed,
        rotation_old_pids,
        rotation_new_pids,
        cancel_batch,
        cancel_pending,
        stop_batch,
        stop_pending,
        complete,
    }
}

pub fn record_permission_denied() {
    CALLS.fetch_add(1, Ordering::Relaxed);
    PERMISSION_DENIALS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_ui_query(converged: bool) {
    UI_QUERY_CALLS.fetch_add(1, Ordering::Relaxed);
    if converged {
        UI_QUERY_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    } else {
        UI_QUERY_WAITS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn report(
    operation: u64,
    argument1: u64,
    argument2: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let result = match operation {
        SERVICE_SUPERVISOR_REPORT_ACTIVE => record_active(argument1, argument2, context),
        SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN => {
            record_rotation_begin(argument1, argument2, context)
        }
        SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE => {
            record_rotation_complete(argument1, argument2, context)
        }
        SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW => {
            record_cancel_window(argument1, argument2, context)
        }
        SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED => {
            record_stop_requested(argument1, argument2, context)
        }
        SERVICE_SUPERVISOR_REPORT_STOP_DRAINED => {
            record_stop_drained(argument1, argument2, context)
        }
        _ => Err(ReportError::InvalidArgument),
    };
    match result {
        Ok(()) => {
            SUCCESSES.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        Err(ReportError::InvalidArgument) => {
            ARGUMENT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            Err(ReportError::InvalidArgument)
        }
        Err(ReportError::InvalidState) => {
            STATE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            Err(ReportError::InvalidState)
        }
    }
}

fn record_active(batches: u64, services: u64, context: ProcessContext) -> Result<(), ReportError> {
    if batches < MIN_ACTIVE_BATCHES || services != PRODUCT_SERVICE_COUNT {
        return Err(ReportError::InvalidArgument);
    }
    if PHASE.load(Ordering::Acquire) != PHASE_INITIAL || !context_matches(context, 0) {
        return Err(ReportError::InvalidState);
    }
    ACTIVE_BATCHES.store(batches, Ordering::Relaxed);
    PHASE.store(PHASE_ACTIVE, Ordering::Release);
    Ok(())
}

fn record_rotation_begin(
    ordinal: u64,
    old_pid: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    if ordinal == 0 || ordinal > REQUIRED_ROTATIONS || old_pid == 0 {
        return Err(ReportError::InvalidArgument);
    }
    let completed = ROTATIONS_COMPLETED.load(Ordering::Acquire);
    if PHASE.load(Ordering::Acquire) != PHASE_ACTIVE
        || ROTATIONS_STARTED.load(Ordering::Acquire) != completed
        || ordinal != completed + 1
        || old_pid != context.current_storage_pid
        || !context_matches(context, completed)
    {
        return Err(ReportError::InvalidState);
    }
    if ordinal > 1 && ROTATION_NEW_PIDS[ordinal as usize - 2].load(Ordering::Acquire) != old_pid {
        return Err(ReportError::InvalidState);
    }
    ROTATION_OLD_PIDS[ordinal as usize - 1].store(old_pid, Ordering::Relaxed);
    ROTATIONS_STARTED.store(ordinal, Ordering::Relaxed);
    PHASE.store(PHASE_ROTATING, Ordering::Release);
    Ok(())
}

fn record_rotation_complete(
    ordinal: u64,
    new_pid: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    if ordinal == 0 || ordinal > REQUIRED_ROTATIONS || new_pid == 0 {
        return Err(ReportError::InvalidArgument);
    }
    let old_pid = ROTATION_OLD_PIDS[ordinal as usize - 1].load(Ordering::Acquire);
    if PHASE.load(Ordering::Acquire) != PHASE_ROTATING
        || ROTATIONS_STARTED.load(Ordering::Acquire) != ordinal
        || ROTATIONS_COMPLETED.load(Ordering::Acquire) + 1 != ordinal
        || new_pid != context.current_storage_pid
        || !context_matches(context, ordinal)
        || !next_generation_same_slot(old_pid, new_pid)
    {
        return Err(ReportError::InvalidState);
    }
    ROTATION_NEW_PIDS[ordinal as usize - 1].store(new_pid, Ordering::Relaxed);
    ROTATIONS_COMPLETED.store(ordinal, Ordering::Relaxed);
    PHASE.store(PHASE_ACTIVE, Ordering::Release);
    Ok(())
}

fn record_cancel_window(
    batch: u64,
    pending: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    if batch == 0 || pending != REQUIRED_CANCEL_PENDING {
        return Err(ReportError::InvalidArgument);
    }
    if PHASE.load(Ordering::Acquire) != PHASE_ACTIVE
        || ROTATIONS_COMPLETED.load(Ordering::Acquire) != REQUIRED_ROTATIONS
        || batch <= ACTIVE_BATCHES.load(Ordering::Acquire)
        || !context_matches(context, REQUIRED_ROTATIONS)
    {
        return Err(ReportError::InvalidState);
    }
    CANCEL_BATCH.store(batch, Ordering::Relaxed);
    CANCEL_PENDING.store(pending, Ordering::Relaxed);
    PHASE.store(PHASE_CANCEL_WINDOW, Ordering::Release);
    Ok(())
}

fn record_stop_requested(
    batch: u64,
    pending: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    if batch == 0 || pending == 0 || pending > PRODUCT_SERVICE_COUNT {
        return Err(ReportError::InvalidArgument);
    }
    if PHASE.load(Ordering::Acquire) != PHASE_CANCEL_WINDOW
        || batch != CANCEL_BATCH.load(Ordering::Acquire)
        || pending != CANCEL_PENDING.load(Ordering::Acquire)
        || !context_matches(context, REQUIRED_ROTATIONS)
    {
        return Err(ReportError::InvalidState);
    }
    STOP_BATCH.store(batch, Ordering::Relaxed);
    STOP_PENDING.store(pending, Ordering::Relaxed);
    PHASE.store(PHASE_STOP_REQUESTED, Ordering::Release);
    Ok(())
}

fn record_stop_drained(
    batch: u64,
    pending: u64,
    context: ProcessContext,
) -> Result<(), ReportError> {
    if batch == 0 || pending != 0 {
        return Err(ReportError::InvalidArgument);
    }
    if PHASE.load(Ordering::Acquire) != PHASE_STOP_REQUESTED
        || batch != STOP_BATCH.load(Ordering::Acquire)
        || !context_matches(context, REQUIRED_ROTATIONS)
    {
        return Err(ReportError::InvalidState);
    }
    PHASE.store(PHASE_STOP_DRAINED, Ordering::Release);
    Ok(())
}

fn context_matches(context: ProcessContext, completed_rotations: u64) -> bool {
    let expected_created = 11 + completed_rotations;
    let expected_exited = 1 + completed_rotations;
    context.current_storage_pid != 0
        && context.created == expected_created
        && context.exited == expected_exited
        && context.reaped == expected_exited
        && context.terminated_exited == expected_exited
        && context.terminated_faulted == 0
        && context.terminated_killed == 0
        && context.live == 10
        && context.storage_server_live == 1
}

fn next_generation_same_slot(previous: u64, replacement: u64) -> bool {
    let previous_slot = previous as u32;
    let replacement_slot = replacement as u32;
    let previous_generation = (previous >> 32) as u32;
    let replacement_generation = (replacement >> 32) as u32;
    previous_slot != 0
        && previous_slot == replacement_slot
        && previous_generation != 0
        && previous_generation.checked_add(1) == Some(replacement_generation)
}

#[cfg(test)]
fn reset() {
    PHASE.store(PHASE_INITIAL, Ordering::Relaxed);
    CALLS.store(0, Ordering::Relaxed);
    SUCCESSES.store(0, Ordering::Relaxed);
    ARGUMENT_REJECTIONS.store(0, Ordering::Relaxed);
    PERMISSION_DENIALS.store(0, Ordering::Relaxed);
    STATE_REJECTIONS.store(0, Ordering::Relaxed);
    UI_QUERY_CALLS.store(0, Ordering::Relaxed);
    UI_QUERY_WAITS.store(0, Ordering::Relaxed);
    UI_QUERY_SUCCESSES.store(0, Ordering::Relaxed);
    ACTIVE_BATCHES.store(0, Ordering::Relaxed);
    ROTATIONS_STARTED.store(0, Ordering::Relaxed);
    ROTATIONS_COMPLETED.store(0, Ordering::Relaxed);
    for pid in &ROTATION_OLD_PIDS {
        pid.store(0, Ordering::Relaxed);
    }
    for pid in &ROTATION_NEW_PIDS {
        pid.store(0, Ordering::Relaxed);
    }
    CANCEL_BATCH.store(0, Ordering::Relaxed);
    CANCEL_PENDING.store(0, Ordering::Relaxed);
    STOP_BATCH.store(0, Ordering::Relaxed);
    STOP_PENDING.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn context(rotations: u64, pid: u64) -> ProcessContext {
        ProcessContext {
            current_storage_pid: pid,
            created: 11 + rotations,
            exited: 1 + rotations,
            reaped: 1 + rotations,
            terminated_exited: 1 + rotations,
            terminated_faulted: 0,
            terminated_killed: 0,
            live: 10,
            storage_server_live: 1,
        }
    }

    #[test]
    fn exact_two_rotation_and_inflight_cancel_transcript_completes() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let first = 1_u64 << 32 | 10;
        let second = 2_u64 << 32 | 10;
        let third = 3_u64 << 32 | 10;
        record_ui_query(false);
        record_ui_query(true);
        assert_eq!(
            report(SERVICE_SUPERVISOR_REPORT_ACTIVE, 0, 0, context(0, first)),
            Err(ReportError::InvalidArgument)
        );
        report(
            SERVICE_SUPERVISOR_REPORT_ACTIVE,
            3,
            PRODUCT_SERVICE_COUNT,
            context(0, first),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN,
            1,
            first,
            context(0, first),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE,
            1,
            second,
            context(1, second),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN,
            2,
            second,
            context(1, second),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_ROTATION_COMPLETE,
            2,
            third,
            context(2, third),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_CANCEL_WINDOW,
            9,
            REQUIRED_CANCEL_PENDING,
            context(2, third),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_STOP_REQUESTED,
            9,
            REQUIRED_CANCEL_PENDING,
            context(2, third),
        )
        .unwrap();
        report(
            SERVICE_SUPERVISOR_REPORT_STOP_DRAINED,
            9,
            0,
            context(2, third),
        )
        .unwrap();
        let snapshot = snapshot();
        assert!(snapshot.complete);
        assert_eq!(snapshot.calls, 9);
        assert_eq!(snapshot.successes, 8);
        assert_eq!(snapshot.argument_rejections, 1);
        assert_eq!(snapshot.ui_query_calls, 2);
        assert_eq!(snapshot.ui_query_waits, 1);
        assert_eq!(snapshot.ui_query_successes, 1);
        assert_eq!(snapshot.rotation_old_pids, [first, second]);
        assert_eq!(snapshot.rotation_new_pids, [second, third]);
    }

    #[test]
    fn wrong_process_ledger_or_generation_is_fail_closed() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let first = 1_u64 << 32 | 10;
        report(
            SERVICE_SUPERVISOR_REPORT_ACTIVE,
            3,
            PRODUCT_SERVICE_COUNT,
            context(0, first),
        )
        .unwrap();
        let mut wrong = context(0, first);
        wrong.created += 1;
        assert_eq!(
            report(SERVICE_SUPERVISOR_REPORT_ROTATION_BEGIN, 1, first, wrong),
            Err(ReportError::InvalidState)
        );
        assert_eq!(snapshot().rotations_started, 0);
        assert!(!snapshot().complete);
    }
}
