//! Transactional evidence for the M40 two-buffer compositor schedule.
//!
//! The swapchain itself remains a userspace SurfaceServer policy layered over
//! two independently generation-qualified `GraphicsBuffer` objects.  This
//! module records only successful display commits.  A preflight token is
//! deliberately inert, so a rejected display transaction (including a missing
//! frame-clock grant) cannot advance the compositor schedule.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::graphics_buffer::GraphicsBufferIdentity;

const SCHEDULE_CAPACITY: usize = 4;
const EXPECTED_SLOTS: [u8; SCHEDULE_CAPACITY] = [0, 1, 0, 1];
const EXPECTED_WRITE_GENERATIONS: [u64; SCHEDULE_CAPACITY] = [2, 2, 3, 3];

static COMMITTED: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);
static SWITCHES: AtomicU64 = AtomicU64::new(0);
static SLOT_ZERO_COMMITS: AtomicU64 = AtomicU64::new(0);
static SLOT_ONE_COMMITS: AtomicU64 = AtomicU64::new(0);
static SLOT_ZERO_ALLOCATION_GENERATION: AtomicU64 = AtomicU64::new(0);
static SLOT_ONE_ALLOCATION_GENERATION: AtomicU64 = AtomicU64::new(0);
static COMMIT_SLOTS: [AtomicU64; SCHEDULE_CAPACITY] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static COMMIT_WRITE_GENERATIONS: [AtomicU64; SCHEDULE_CAPACITY] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwapchainScheduleError {
    Complete,
    UnexpectedSlot,
    UnexpectedWriteGeneration,
    AllocationIdentityChanged,
    StaleToken,
}

/// Immutable permission to commit the currently expected buffer generation.
/// Creating this value never changes global state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwapchainCommitToken {
    ordinal: u8,
    identity: GraphicsBufferIdentity,
    write_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwapchainScheduleSnapshot {
    pub committed: u64,
    pub errors: u64,
    pub registered: u8,
    pub switches: u64,
    pub per_slot_commits: [u64; 2],
    pub allocation_generations: [u64; 2],
    /// `None` denotes an uncommitted trace position.
    pub commit_slots: [Option<u8>; SCHEDULE_CAPACITY],
    pub commit_write_generations: [u64; SCHEDULE_CAPACITY],
    pub next_slot: Option<u8>,
    pub next_write_generation: u64,
}

/// Checks the next fixed M40 scheduling edge without advancing it.
pub fn preflight(
    identity: GraphicsBufferIdentity,
    write_generation: u64,
) -> Result<SwapchainCommitToken, SwapchainScheduleError> {
    let ordinal = usize::try_from(COMMITTED.load(Ordering::Acquire)).unwrap_or(SCHEDULE_CAPACITY);
    if ordinal >= SCHEDULE_CAPACITY {
        return Err(SwapchainScheduleError::Complete);
    }
    if identity.slot() != EXPECTED_SLOTS[ordinal] {
        return Err(SwapchainScheduleError::UnexpectedSlot);
    }
    if write_generation != EXPECTED_WRITE_GENERATIONS[ordinal] {
        return Err(SwapchainScheduleError::UnexpectedWriteGeneration);
    }
    let known_generation = allocation_generation(identity.slot());
    if known_generation != 0 && known_generation != identity.generation() {
        return Err(SwapchainScheduleError::AllocationIdentityChanged);
    }
    Ok(SwapchainCommitToken {
        ordinal: u8::try_from(ordinal)
            .unwrap_or_else(|_| panic!("swapchain schedule ordinal exceeded u8")),
        identity,
        write_generation,
    })
}

/// Records one display commit after its scene/scanout copy has succeeded.
///
/// The syscall path is single-core with IRQ masked, so a token cannot race a
/// second successful present between preflight and this commit point.
pub fn commit(token: SwapchainCommitToken) -> Result<(), SwapchainScheduleError> {
    let ordinal = usize::from(token.ordinal);
    if COMMITTED.load(Ordering::Acquire) != ordinal as u64 {
        ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(SwapchainScheduleError::StaleToken);
    }
    if token.identity.slot() != EXPECTED_SLOTS[ordinal] {
        ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(SwapchainScheduleError::UnexpectedSlot);
    }
    if token.write_generation != EXPECTED_WRITE_GENERATIONS[ordinal] {
        ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(SwapchainScheduleError::UnexpectedWriteGeneration);
    }

    let allocation_generation = allocation_generation_atomic(token.identity.slot());
    let known = allocation_generation.load(Ordering::Acquire);
    if known == 0 {
        allocation_generation.store(token.identity.generation(), Ordering::Release);
    } else if known != token.identity.generation() {
        ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err(SwapchainScheduleError::AllocationIdentityChanged);
    }

    COMMIT_SLOTS[ordinal].store(u64::from(token.identity.slot()) + 1, Ordering::Release);
    COMMIT_WRITE_GENERATIONS[ordinal].store(token.write_generation, Ordering::Release);
    match token.identity.slot() {
        0 => {
            SLOT_ZERO_COMMITS.fetch_add(1, Ordering::Relaxed);
        }
        1 => {
            SLOT_ONE_COMMITS.fetch_add(1, Ordering::Relaxed);
        }
        _ => unreachable!("preflight accepted an out-of-pool swapchain slot"),
    }
    if ordinal != 0 {
        SWITCHES.fetch_add(1, Ordering::Relaxed);
    }
    COMMITTED.store(
        u64::try_from(ordinal + 1)
            .unwrap_or_else(|_| panic!("swapchain committed count exceeded u64")),
        Ordering::Release,
    );
    Ok(())
}

pub fn snapshot() -> SwapchainScheduleSnapshot {
    let committed = COMMITTED.load(Ordering::Acquire);
    let slot_zero_generation = SLOT_ZERO_ALLOCATION_GENERATION.load(Ordering::Acquire);
    let slot_one_generation = SLOT_ONE_ALLOCATION_GENERATION.load(Ordering::Acquire);
    let next = usize::try_from(committed)
        .ok()
        .filter(|index| *index < SCHEDULE_CAPACITY);
    SwapchainScheduleSnapshot {
        committed,
        errors: ERRORS.load(Ordering::Acquire),
        registered: u8::from(slot_zero_generation != 0) + u8::from(slot_one_generation != 0),
        switches: SWITCHES.load(Ordering::Acquire),
        per_slot_commits: [
            SLOT_ZERO_COMMITS.load(Ordering::Acquire),
            SLOT_ONE_COMMITS.load(Ordering::Acquire),
        ],
        allocation_generations: [slot_zero_generation, slot_one_generation],
        commit_slots: core::array::from_fn(|index| {
            let encoded = COMMIT_SLOTS[index].load(Ordering::Acquire);
            (encoded != 0).then(|| {
                u8::try_from(encoded - 1)
                    .unwrap_or_else(|_| panic!("swapchain slot evidence exceeded u8"))
            })
        }),
        commit_write_generations: core::array::from_fn(|index| {
            COMMIT_WRITE_GENERATIONS[index].load(Ordering::Acquire)
        }),
        next_slot: next.map(|index| EXPECTED_SLOTS[index]),
        next_write_generation: next.map_or(0, |index| EXPECTED_WRITE_GENERATIONS[index]),
    }
}

fn allocation_generation(slot: u8) -> u64 {
    allocation_generation_atomic(slot).load(Ordering::Acquire)
}

fn allocation_generation_atomic(slot: u8) -> &'static AtomicU64 {
    match slot {
        0 => &SLOT_ZERO_ALLOCATION_GENERATION,
        1 => &SLOT_ONE_ALLOCATION_GENERATION,
        _ => &SLOT_ONE_ALLOCATION_GENERATION,
    }
}

#[cfg(test)]
fn reset() {
    COMMITTED.store(0, Ordering::Relaxed);
    ERRORS.store(0, Ordering::Relaxed);
    SWITCHES.store(0, Ordering::Relaxed);
    SLOT_ZERO_COMMITS.store(0, Ordering::Relaxed);
    SLOT_ONE_COMMITS.store(0, Ordering::Relaxed);
    SLOT_ZERO_ALLOCATION_GENERATION.store(0, Ordering::Relaxed);
    SLOT_ONE_ALLOCATION_GENERATION.store(0, Ordering::Relaxed);
    for value in &COMMIT_SLOTS {
        value.store(0, Ordering::Relaxed);
    }
    for value in &COMMIT_WRITE_GENERATIONS {
        value.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::{SwapchainScheduleError, commit, preflight, reset, snapshot};
    use crate::graphics_buffer::{GraphicsBuffer, test_serial_guard};

    fn buffers() -> (GraphicsBuffer, GraphicsBuffer) {
        let first = GraphicsBuffer::try_new_mappable(91).unwrap();
        let second = GraphicsBuffer::try_new_mappable(91).unwrap();
        assert_eq!(first.slot(), 0);
        assert_eq!(second.slot(), 1);
        (first, second)
    }

    #[test]
    fn failed_preflights_and_uncommitted_tokens_are_transactionally_inert() {
        let _serial = test_serial_guard();
        reset();
        let (first, second) = buffers();
        assert_eq!(
            preflight(second.identity(), 2),
            Err(SwapchainScheduleError::UnexpectedSlot)
        );
        assert_eq!(
            preflight(first.identity(), 3),
            Err(SwapchainScheduleError::UnexpectedWriteGeneration)
        );
        let _display_rejected = preflight(first.identity(), 2).unwrap();
        assert_eq!(snapshot().committed, 0);
        assert_eq!(snapshot().errors, 0);
    }

    #[test]
    fn successful_commits_publish_aba_and_leave_b3_as_the_next_frame() {
        let _serial = test_serial_guard();
        reset();
        let (first, second) = buffers();
        let allocation_generations = [first.slot_generation(), second.slot_generation()];
        for (identity, generation) in [
            (first.identity(), 2),
            (second.identity(), 2),
            (first.identity(), 3),
        ] {
            let token = preflight(identity, generation).unwrap();
            commit(token).unwrap();
        }
        let evidence = snapshot();
        assert_eq!(evidence.committed, 3);
        assert_eq!(evidence.errors, 0);
        assert_eq!(evidence.registered, 2);
        assert_eq!(evidence.switches, 2);
        assert_eq!(evidence.per_slot_commits, [2, 1]);
        assert_eq!(evidence.allocation_generations, allocation_generations);
        assert_eq!(evidence.commit_slots, [Some(0), Some(1), Some(0), None]);
        assert_eq!(evidence.commit_write_generations, [2, 2, 3, 0]);
        assert_eq!(evidence.next_slot, Some(1));
        assert_eq!(evidence.next_write_generation, 3);

        // A no-grant attempt can hold this inert token without advancing B3.
        let _rejected = preflight(second.identity(), 3).unwrap();
        assert_eq!(snapshot(), evidence);
    }

    #[test]
    fn allocation_generation_cannot_change_mid_schedule() {
        let _serial = test_serial_guard();
        reset();
        let (first, second) = buffers();
        let old_first = first.identity();
        commit(preflight(old_first, 2).unwrap()).unwrap();
        commit(preflight(second.identity(), 2).unwrap()).unwrap();
        drop(first);
        drop(second);
        let replacement = GraphicsBuffer::try_new_mappable(92).unwrap();
        assert_eq!(replacement.slot(), old_first.slot());
        assert_ne!(replacement.slot_generation(), old_first.generation());
        assert_eq!(
            preflight(replacement.identity(), 3),
            Err(SwapchainScheduleError::AllocationIdentityChanged)
        );
        assert_eq!(snapshot().committed, 2);
        assert_eq!(snapshot().errors, 0);
    }
}
