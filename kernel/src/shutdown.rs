//! M65's bounded, init-owned shutdown orchestration gate.
//!
//! The gate closes new process and StorageServer-session admission at
//! `Prepare`, accepts `Commit` only for the same AppData generation, and is
//! finally sealed by the kernel monitor after durable device-health close.
//! It is deliberately boot-local: no authority or phase is reconstructed from
//! persistent storage.

use core::sync::atomic::{AtomicU64, Ordering};

const PHASE_MASK: u64 = 0b11;
pub const MAX_GENERATION: u64 = u64::MAX >> 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Phase {
    Open = 0,
    Quiescing = 1,
    Requested = 2,
    Sealed = 3,
}

impl Phase {
    const fn from_raw(raw: u64) -> Self {
        match raw {
            0 => Self::Open,
            1 => Self::Quiescing,
            2 => Self::Requested,
            3 => Self::Sealed,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    Prepare,
    Commit,
    Seal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidGeneration,
    InvalidPhase,
    GenerationMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub phase: Phase,
    pub generation: u64,
    pub calls: u64,
    pub prepare_calls: u64,
    pub prepares: u64,
    pub commit_calls: u64,
    pub commits: u64,
    pub permission_denied: u64,
    pub invalid_arguments: u64,
    pub not_ready: u64,
    pub replays: u64,
    pub spawn_rejections: u64,
    pub connect_rejections: u64,
    pub invariant_errors: u64,
}

static STATE: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
static PREPARE_CALLS: AtomicU64 = AtomicU64::new(0);
static PREPARES: AtomicU64 = AtomicU64::new(0);
static COMMIT_CALLS: AtomicU64 = AtomicU64::new(0);
static COMMITS: AtomicU64 = AtomicU64::new(0);
static PERMISSION_DENIED: AtomicU64 = AtomicU64::new(0);
static INVALID_ARGUMENTS: AtomicU64 = AtomicU64::new(0);
static NOT_READY: AtomicU64 = AtomicU64::new(0);
static REPLAYS: AtomicU64 = AtomicU64::new(0);
static SPAWN_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static CONNECT_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static INVARIANT_ERRORS: AtomicU64 = AtomicU64::new(0);

const fn encode(phase: Phase, generation: u64) -> u64 {
    (generation << 2) | phase as u64
}

const fn decode(raw: u64) -> (Phase, u64) {
    (Phase::from_raw(raw & PHASE_MASK), raw >> 2)
}

/// Pure transition reducer shared by the kernel gate and host tests.
pub const fn reduce(
    current: u64,
    transition: Transition,
    generation: u64,
) -> Result<u64, TransitionError> {
    if generation == 0 || generation > MAX_GENERATION {
        return Err(TransitionError::InvalidGeneration);
    }
    let (phase, stored_generation) = decode(current);
    match transition {
        Transition::Prepare => {
            if !matches!(phase, Phase::Open) {
                return Err(TransitionError::InvalidPhase);
            }
            Ok(encode(Phase::Quiescing, generation))
        }
        Transition::Commit => {
            if !matches!(phase, Phase::Quiescing) {
                return Err(TransitionError::InvalidPhase);
            }
            if stored_generation != generation {
                return Err(TransitionError::GenerationMismatch);
            }
            Ok(encode(Phase::Requested, generation))
        }
        Transition::Seal => {
            if !matches!(phase, Phase::Requested) {
                return Err(TransitionError::InvalidPhase);
            }
            if stored_generation != generation {
                return Err(TransitionError::GenerationMismatch);
            }
            Ok(encode(Phase::Sealed, generation))
        }
    }
}

fn transition(operation: Transition, generation: u64) -> Result<(), TransitionError> {
    let current = STATE.load(Ordering::Acquire);
    let next = reduce(current, operation, generation)?;
    STATE
        .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
        .map(|_| ())
        .map_err(|_| TransitionError::InvalidPhase)
}

pub fn prepare(generation: u64) -> Result<(), TransitionError> {
    let result = transition(Transition::Prepare, generation);
    if result.is_ok() {
        PREPARES.fetch_add(1, Ordering::Relaxed);
    }
    result
}

pub fn commit(generation: u64) -> Result<(), TransitionError> {
    let result = transition(Transition::Commit, generation);
    if result.is_ok() {
        COMMITS.fetch_add(1, Ordering::Relaxed);
    }
    result
}

pub fn seal(generation: u64) -> Result<(), TransitionError> {
    transition(Transition::Seal, generation)
}

pub fn admission_closed() -> bool {
    !matches!(decode(STATE.load(Ordering::Acquire)).0, Phase::Open)
}

pub fn requested_generation() -> Option<u64> {
    let (phase, generation) = decode(STATE.load(Ordering::Acquire));
    matches!(phase, Phase::Requested).then_some(generation)
}

pub fn record_call(prepare: bool) {
    CALLS.fetch_add(1, Ordering::Relaxed);
    if prepare {
        PREPARE_CALLS.fetch_add(1, Ordering::Relaxed);
    } else {
        COMMIT_CALLS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_permission_denied() {
    PERMISSION_DENIED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_invalid_argument() {
    INVALID_ARGUMENTS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_not_ready() {
    NOT_READY.fetch_add(1, Ordering::Relaxed);
}

pub fn record_replay() {
    REPLAYS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_spawn_rejection() {
    SPAWN_REJECTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_connect_rejection() {
    CONNECT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_invariant_error() {
    INVARIANT_ERRORS.fetch_add(1, Ordering::Relaxed);
}

pub fn snapshot() -> Snapshot {
    let (phase, generation) = decode(STATE.load(Ordering::Acquire));
    Snapshot {
        phase,
        generation,
        calls: CALLS.load(Ordering::Acquire),
        prepare_calls: PREPARE_CALLS.load(Ordering::Acquire),
        prepares: PREPARES.load(Ordering::Acquire),
        commit_calls: COMMIT_CALLS.load(Ordering::Acquire),
        commits: COMMITS.load(Ordering::Acquire),
        permission_denied: PERMISSION_DENIED.load(Ordering::Acquire),
        invalid_arguments: INVALID_ARGUMENTS.load(Ordering::Acquire),
        not_ready: NOT_READY.load(Ordering::Acquire),
        replays: REPLAYS.load(Ordering::Acquire),
        spawn_rejections: SPAWN_REJECTIONS.load(Ordering::Acquire),
        connect_rejections: CONNECT_REJECTIONS.load(Ordering::Acquire),
        invariant_errors: INVARIANT_ERRORS.load(Ordering::Acquire),
    }
}

#[cfg(test)]
mod tests {
    use super::{Phase, Transition, TransitionError, decode, reduce};

    #[test]
    fn strict_prepare_commit_seal_sequence_preserves_generation() {
        let prepared = reduce(0, Transition::Prepare, 7).unwrap();
        assert_eq!(decode(prepared), (Phase::Quiescing, 7));
        let requested = reduce(prepared, Transition::Commit, 7).unwrap();
        assert_eq!(decode(requested), (Phase::Requested, 7));
        let sealed = reduce(requested, Transition::Seal, 7).unwrap();
        assert_eq!(decode(sealed), (Phase::Sealed, 7));
    }

    #[test]
    fn commit_before_prepare_and_replay_fail_closed() {
        assert_eq!(
            reduce(0, Transition::Commit, 1),
            Err(TransitionError::InvalidPhase)
        );
        let prepared = reduce(0, Transition::Prepare, 1).unwrap();
        assert_eq!(
            reduce(prepared, Transition::Prepare, 1),
            Err(TransitionError::InvalidPhase)
        );
    }

    #[test]
    fn generation_mismatch_never_advances_the_gate() {
        let prepared = reduce(0, Transition::Prepare, 9).unwrap();
        assert_eq!(
            reduce(prepared, Transition::Commit, 10),
            Err(TransitionError::GenerationMismatch)
        );
        let requested = reduce(prepared, Transition::Commit, 9).unwrap();
        assert_eq!(
            reduce(requested, Transition::Seal, 10),
            Err(TransitionError::GenerationMismatch)
        );
    }

    #[test]
    fn zero_and_unencodable_generations_are_rejected() {
        assert_eq!(
            reduce(0, Transition::Prepare, 0),
            Err(TransitionError::InvalidGeneration)
        );
        assert_eq!(
            reduce(0, Transition::Prepare, super::MAX_GENERATION + 1),
            Err(TransitionError::InvalidGeneration)
        );
    }
}
