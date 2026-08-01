//! Pure, allocation-free validation for the post-M15 service-cycle transcript.
//!
//! The syscall layer remains responsible for recognizing transcript tag
//! prefixes and for mapping a generation-qualified startup-peer handle to a
//! [`Role`]. This module owns the protocol state machine itself. A transition
//! depends only on a [`Snapshot`], [`Gate`], and [`Event`], which keeps the
//! evidence rules host-testable and lets an atomic adapter commit accepted or
//! rejected snapshots without partially updating protocol state.

/// Number of commits in one dynamic-service cycle.
pub const STEP_COUNT: u8 = 5;
/// Number of distinct transaction identifiers retained per cycle.
pub const TRANSACTION_ID_COUNT: usize = 4;
/// Largest round encodable in the M16 commit tag's 24-bit round field.
pub const MAX_ROUND: u32 = 0x00ff_ffff;
/// Bitmap produced after all three resident roles have reported completion.
pub const ALL_DONE: u8 = 0b111;

/// A resident service-cycle participant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Role {
    Provider = 1,
    Client = 2,
    ServiceManager = 3,
}

impl Role {
    /// Decodes the complete DONE payload. Values with non-zero upper bits are
    /// invalid rather than aliases of a role.
    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Provider),
            2 => Some(Self::Client),
            3 => Some(Self::ServiceManager),
            _ => None,
        }
    }

    pub const fn raw(self) -> u64 {
        self as u64
    }

    const fn done_bit(self) -> u8 {
        1 << (self as u8 - 1)
    }
}

/// External evidence required before cycle events may advance.
///
/// `predecessor_complete` represents both init readiness and a complete M15
/// transcript. The remaining fields are consulted only when round one starts;
/// later rounds derive their predecessor and high-water marks from `Snapshot`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gate {
    pub predecessor_complete: bool,
    pub predecessor_instance: u32,
    pub transaction_high_water: u32,
}

impl Gate {
    pub const fn open(predecessor_instance: u32, transaction_high_water: u32) -> Self {
        Self {
            predecessor_complete: true,
            predecessor_instance,
            transaction_high_water,
        }
    }

    pub const fn closed() -> Self {
        Self {
            predecessor_complete: false,
            predecessor_instance: 0,
            transaction_high_water: 0,
        }
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::closed()
    }
}

/// A target-prefix event after the syscall adapter has decoded its tag.
///
/// `source` is `None` when the source handle is not a live, generation-matched
/// startup peer. DONE keeps the claimed payload role separate from the source
/// so the reducer can reject forged or mismatched role claims transactionally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Commit {
        round: u32,
        step: u8,
        source: Option<Role>,
        transaction_id: u32,
        instance: u32,
    },
    Done {
        round: u32,
        role: Option<Role>,
        source: Option<Role>,
    },
}

impl Event {
    pub const fn commit(
        round: u32,
        step: u8,
        source: Option<Role>,
        transaction_id: u32,
        instance: u32,
    ) -> Self {
        Self::Commit {
            round,
            step,
            source,
            transaction_id,
            instance,
        }
    }

    pub const fn done(round: u32, role: Option<Role>, source: Option<Role>) -> Self {
        Self::Done {
            round,
            role,
            source,
        }
    }
}

/// Constant-space transcript state for the current or latest complete round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub round: u32,
    pub step: u8,
    pub completed_rounds: u32,
    pub errors: u64,
    pub instance: u32,
    pub transaction_ids: [u32; TRANSACTION_ID_COUNT],
    pub done_reads: u64,
    pub done_bitmap: u8,
}

impl Snapshot {
    pub const INITIAL: Self = Self {
        round: 0,
        step: 0,
        completed_rounds: 0,
        errors: 0,
        instance: 0,
        transaction_ids: [0; TRANSACTION_ID_COUNT],
        done_reads: 0,
        done_bitmap: 0,
    };

    pub const fn is_round_complete(&self) -> bool {
        self.round != 0
            && self.step == STEP_COUNT
            && self.completed_rounds == self.round
            && self.done_reads == 3
            && self.done_bitmap == ALL_DONE
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::INITIAL
    }
}

/// Why an event was rejected. The reason is diagnostic; all rejection paths
/// have the same state contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    GateClosed,
    InvalidRound,
    InvalidStep,
    WrongSource,
    OutOfOrder,
    RoundNotQuiescent,
    InvalidInstance,
    InvalidTransaction,
    StepsIncomplete,
    InvalidRole,
    RoleSourceMismatch,
    DuplicateDone,
    RoundAlreadyComplete,
    CounterOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Accepted,
    Rejected(RejectReason),
}

/// Result of one pure reducer invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub struct Transition {
    pub snapshot: Snapshot,
    pub decision: Decision,
}

impl Transition {
    pub const fn accepted(&self) -> bool {
        matches!(self.decision, Decision::Accepted)
    }
}

/// Applies one decoded target-prefix event.
///
/// On rejection, `errors` is incremented exactly once and every other snapshot
/// field is preserved. The counter saturates at `u64::MAX` so malformed input
/// cannot make the evidence count wrap back to zero.
pub fn reduce(snapshot: Snapshot, gate: Gate, event: Event) -> Transition {
    if !gate.predecessor_complete {
        return rejected(snapshot, RejectReason::GateClosed);
    }

    match event {
        Event::Commit {
            round,
            step,
            source,
            transaction_id,
            instance,
        } => reduce_commit(
            snapshot,
            gate,
            round,
            step,
            source,
            transaction_id,
            instance,
        ),
        Event::Done {
            round,
            role,
            source,
        } => reduce_done(snapshot, round, role, source),
    }
}

fn reduce_commit(
    snapshot: Snapshot,
    gate: Gate,
    round: u32,
    step: u8,
    source: Option<Role>,
    transaction_id: u32,
    instance: u32,
) -> Transition {
    if round == 0 || round > MAX_ROUND {
        return rejected(snapshot, RejectReason::InvalidRound);
    }
    if step == 0 || step > STEP_COUNT {
        return rejected(snapshot, RejectReason::InvalidStep);
    }

    let expected_source = match step {
        1 | 3 | 4 => Role::Provider,
        2 | 5 => Role::Client,
        _ => unreachable!(),
    };
    if source != Some(expected_source) {
        return rejected(snapshot, RejectReason::WrongSource);
    }

    if step == 1 {
        return reduce_round_start(snapshot, gate, round, transaction_id, instance);
    }

    if round != snapshot.round || step != snapshot.step.saturating_add(1) {
        return rejected(snapshot, RejectReason::OutOfOrder);
    }
    if snapshot.instance == 0 {
        return rejected(snapshot, RejectReason::InvalidInstance);
    }

    let register_txid = snapshot.transaction_ids[0];
    let lookup_txid = snapshot.transaction_ids[1];
    let unregister_txid = snapshot.transaction_ids[2];
    let valid = match step {
        2 => instance == snapshot.instance && transaction_id > register_txid,
        3 => instance == snapshot.instance && transaction_id == lookup_txid,
        4 => instance == snapshot.instance && transaction_id > lookup_txid,
        5 => instance == 0 && transaction_id > unregister_txid,
        _ => false,
    };
    if !valid {
        let reason = if (step == 2 || step == 3 || step == 4) && instance != snapshot.instance
            || step == 5 && instance != 0
        {
            RejectReason::InvalidInstance
        } else {
            RejectReason::InvalidTransaction
        };
        return rejected(snapshot, reason);
    }

    let mut next = snapshot;
    match step {
        2 => next.transaction_ids[1] = transaction_id,
        3 => {}
        4 => next.transaction_ids[2] = transaction_id,
        5 => next.transaction_ids[3] = transaction_id,
        _ => unreachable!(),
    }
    next.step = step;
    accepted(next)
}

fn reduce_round_start(
    snapshot: Snapshot,
    gate: Gate,
    round: u32,
    transaction_id: u32,
    instance: u32,
) -> Transition {
    let (expected_round, predecessor_instance, transaction_high_water) =
        if snapshot.completed_rounds == 0 {
            if snapshot.round != 0
                || snapshot.step != 0
                || snapshot.done_reads != 0
                || snapshot.done_bitmap != 0
            {
                return rejected(snapshot, RejectReason::RoundNotQuiescent);
            }
            (1, gate.predecessor_instance, gate.transaction_high_water)
        } else {
            if snapshot.round != snapshot.completed_rounds
                || snapshot.step != STEP_COUNT
                || snapshot.done_reads != 3
                || snapshot.done_bitmap != ALL_DONE
            {
                return rejected(snapshot, RejectReason::RoundNotQuiescent);
            }
            let Some(next_round) = snapshot.completed_rounds.checked_add(1) else {
                return rejected(snapshot, RejectReason::CounterOverflow);
            };
            (
                next_round,
                snapshot.instance,
                snapshot.transaction_ids[TRANSACTION_ID_COUNT - 1],
            )
        };

    if expected_round > MAX_ROUND {
        return rejected(snapshot, RejectReason::CounterOverflow);
    }
    if round != expected_round {
        return rejected(snapshot, RejectReason::OutOfOrder);
    }
    if predecessor_instance == 0 || instance <= predecessor_instance {
        return rejected(snapshot, RejectReason::InvalidInstance);
    }
    if transaction_id <= transaction_high_water {
        return rejected(snapshot, RejectReason::InvalidTransaction);
    }

    let mut next = snapshot;
    next.round = round;
    next.step = 1;
    next.instance = instance;
    next.transaction_ids = [0; TRANSACTION_ID_COUNT];
    next.transaction_ids[0] = transaction_id;
    next.done_reads = 0;
    next.done_bitmap = 0;
    accepted(next)
}

fn reduce_done(
    snapshot: Snapshot,
    round: u32,
    role: Option<Role>,
    source: Option<Role>,
) -> Transition {
    if round == 0 || round > MAX_ROUND {
        return rejected(snapshot, RejectReason::InvalidRound);
    }
    if round != snapshot.round {
        return rejected(snapshot, RejectReason::OutOfOrder);
    }
    if snapshot.step != STEP_COUNT {
        return rejected(snapshot, RejectReason::StepsIncomplete);
    }
    if snapshot.completed_rounds == round {
        return rejected(snapshot, RejectReason::RoundAlreadyComplete);
    }

    let Some(role) = role else {
        return rejected(snapshot, RejectReason::InvalidRole);
    };
    if source != Some(role) {
        return rejected(snapshot, RejectReason::RoleSourceMismatch);
    }
    let bit = role.done_bit();
    if snapshot.done_bitmap & bit != 0 {
        return rejected(snapshot, RejectReason::DuplicateDone);
    }
    let Some(done_reads) = snapshot.done_reads.checked_add(1) else {
        return rejected(snapshot, RejectReason::CounterOverflow);
    };

    let mut next = snapshot;
    next.done_reads = done_reads;
    next.done_bitmap |= bit;
    if next.done_bitmap == ALL_DONE {
        next.completed_rounds = round;
    }
    accepted(next)
}

const fn accepted(snapshot: Snapshot) -> Transition {
    Transition {
        snapshot,
        decision: Decision::Accepted,
    }
}

const fn rejected(mut snapshot: Snapshot, reason: RejectReason) -> Transition {
    snapshot.errors = snapshot.errors.saturating_add(1);
    Transition {
        snapshot,
        decision: Decision::Rejected(reason),
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{needs_drop, size_of};

    use super::*;

    const GATE: Gate = Gate::open(0x20_002, 0x208);

    fn commits(round: u32, instance: u32, first_txid: u32) -> [Event; 5] {
        [
            Event::commit(round, 1, Some(Role::Provider), first_txid, instance),
            Event::commit(round, 2, Some(Role::Client), first_txid + 1, instance),
            Event::commit(round, 3, Some(Role::Provider), first_txid + 1, instance),
            Event::commit(round, 4, Some(Role::Provider), first_txid + 2, instance),
            Event::commit(round, 5, Some(Role::Client), first_txid + 3, 0),
        ]
    }

    fn apply_accepted(snapshot: Snapshot, gate: Gate, event: Event) -> Snapshot {
        let transition = reduce(snapshot, gate, event);
        assert_eq!(transition.decision, Decision::Accepted);
        transition.snapshot
    }

    fn apply_rejected(snapshot: Snapshot, gate: Gate, event: Event) -> Snapshot {
        let transition = reduce(snapshot, gate, event);
        assert!(matches!(transition.decision, Decision::Rejected(_)));
        let mut expected = snapshot;
        expected.errors = expected.errors.saturating_add(1);
        assert_eq!(transition.snapshot, expected);
        transition.snapshot
    }

    fn apply_commits(mut snapshot: Snapshot, gate: Gate, events: &[Event; 5]) -> Snapshot {
        for event in events {
            snapshot = apply_accepted(snapshot, gate, *event);
        }
        snapshot
    }

    fn apply_done(mut snapshot: Snapshot, gate: Gate, round: u32, role: Role) -> Snapshot {
        snapshot = apply_accepted(snapshot, gate, Event::done(round, Some(role), Some(role)));
        snapshot
    }

    fn complete_round(
        snapshot: Snapshot,
        gate: Gate,
        round: u32,
        instance: u32,
        first_txid: u32,
    ) -> Snapshot {
        let events = commits(round, instance, first_txid);
        let mut snapshot = apply_commits(snapshot, gate, &events);
        for role in [Role::Provider, Role::Client, Role::ServiceManager] {
            snapshot = apply_done(snapshot, gate, round, role);
        }
        snapshot
    }

    fn for_each_permutation<const N: usize, F: FnMut(&[usize; N])>(
        values: &mut [usize; N],
        index: usize,
        visitor: &mut F,
    ) {
        if index == N {
            visitor(values);
            return;
        }
        for swap_index in index..N {
            values.swap(index, swap_index);
            for_each_permutation(values, index + 1, visitor);
            values.swap(index, swap_index);
        }
    }

    #[test]
    fn role_decode_and_state_types_are_fixed_size_copy_values() {
        assert_eq!(Role::from_raw(1), Some(Role::Provider));
        assert_eq!(Role::from_raw(2), Some(Role::Client));
        assert_eq!(Role::from_raw(3), Some(Role::ServiceManager));
        assert_eq!(Role::from_raw(0), None);
        assert_eq!(Role::from_raw((1_u64 << 32) | 1), None);
        assert_eq!(Role::Provider.raw(), 1);

        assert_eq!(size_of::<Snapshot>(), 48);
        assert!(size_of::<Event>() <= 32);
        assert!(!needs_drop::<Snapshot>());
        assert!(!needs_drop::<Gate>());
        assert!(!needs_drop::<Event>());
        assert!(!needs_drop::<Transition>());
    }

    #[test]
    fn all_120_commit_permutations_have_one_successful_order() {
        let events = commits(1, 0x20_003, 0x301);
        let mut order = [0, 1, 2, 3, 4];
        let mut permutations = 0;
        let mut successful = 0;
        for_each_permutation(&mut order, 0, &mut |candidate| {
            permutations += 1;
            let mut snapshot = Snapshot::INITIAL;
            for index in candidate {
                snapshot = reduce(snapshot, GATE, events[*index]).snapshot;
            }
            if snapshot.step == STEP_COUNT {
                successful += 1;
                assert_eq!(*candidate, [0, 1, 2, 3, 4]);
                assert_eq!(snapshot.errors, 0);
            } else {
                assert!(snapshot.step < STEP_COUNT);
            }
        });
        assert_eq!(permutations, 120);
        assert_eq!(successful, 1);
    }

    #[test]
    fn duplicate_skip_source_instance_and_transaction_errors_are_transactional() {
        let events = commits(1, 0x20_003, 0x301);
        let mut snapshot = apply_accepted(Snapshot::INITIAL, GATE, events[0]);

        snapshot = apply_rejected(snapshot, GATE, events[0]);
        snapshot = apply_rejected(snapshot, GATE, events[3]);
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 2, Some(Role::Provider), 0x302, 0x20_003),
        );
        snapshot = apply_rejected(snapshot, GATE, Event::commit(1, 2, None, 0x302, 0x20_003));
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 2, Some(Role::Client), 0x302, 0x20_004),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 2, Some(Role::Client), 0x301, 0x20_003),
        );
        snapshot = apply_accepted(snapshot, GATE, events[1]);

        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 3, Some(Role::Provider), 0x303, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 3, Some(Role::Provider), 0x302, 0x20_004),
        );
        snapshot = apply_accepted(snapshot, GATE, events[2]);

        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 4, Some(Role::Provider), 0x302, 0x20_003),
        );
        snapshot = apply_accepted(snapshot, GATE, events[3]);

        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 5, Some(Role::Client), 0x304, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 5, Some(Role::Client), 0x303, 0),
        );
        snapshot = apply_accepted(snapshot, GATE, events[4]);

        assert_eq!(snapshot.step, STEP_COUNT);
        assert_eq!(snapshot.transaction_ids, [0x301, 0x302, 0x303, 0x304]);
        assert_eq!(snapshot.errors, 11);
    }

    #[test]
    fn every_done_order_completes_with_three_unique_roles() {
        let events = commits(1, 0x20_003, 0x301);
        let after_commits = apply_commits(Snapshot::INITIAL, GATE, &events);
        let roles = [Role::Provider, Role::Client, Role::ServiceManager];
        let mut order = [0, 1, 2];
        let mut permutations = 0;
        for_each_permutation(&mut order, 0, &mut |candidate| {
            permutations += 1;
            let mut snapshot = after_commits;
            for index in candidate {
                snapshot = apply_done(snapshot, GATE, 1, roles[*index]);
            }
            assert!(snapshot.is_round_complete());
            assert_eq!(snapshot.errors, 0);
        });
        assert_eq!(permutations, 6);
    }

    #[test]
    fn premature_invalid_mismatched_and_duplicate_done_are_rejected() {
        let events = commits(1, 0x20_003, 0x301);
        let mut snapshot = Snapshot::INITIAL;
        for event in &events[..4] {
            snapshot = apply_accepted(snapshot, GATE, *event);
        }
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::done(1, Some(Role::Provider), Some(Role::Provider)),
        );
        snapshot = apply_accepted(snapshot, GATE, events[4]);

        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::done(2, Some(Role::Provider), Some(Role::Provider)),
        );
        snapshot = apply_rejected(snapshot, GATE, Event::done(1, None, Some(Role::Provider)));
        snapshot = apply_rejected(snapshot, GATE, Event::done(1, Some(Role::Provider), None));
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::done(1, Some(Role::Provider), Some(Role::Client)),
        );

        snapshot = apply_done(snapshot, GATE, 1, Role::Provider);
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::done(1, Some(Role::Provider), Some(Role::Provider)),
        );
        snapshot = apply_done(snapshot, GATE, 1, Role::ServiceManager);
        snapshot = apply_done(snapshot, GATE, 1, Role::Client);
        assert!(snapshot.is_round_complete());
        assert_eq!(snapshot.errors, 6);
        assert_eq!(snapshot.done_reads, 3);

        let after_complete = apply_rejected(
            snapshot,
            GATE,
            Event::done(1, Some(Role::Client), Some(Role::Client)),
        );
        assert_eq!(after_complete.done_reads, 3);
        assert_eq!(after_complete.completed_rounds, 1);
    }

    #[test]
    fn round_two_uses_round_one_instance_and_transaction_high_water() {
        let mut snapshot = complete_round(Snapshot::INITIAL, GATE, 1, 0x20_003, 0x301);
        let second_gate = Gate::open(0, u32::MAX);

        snapshot = apply_rejected(
            snapshot,
            second_gate,
            Event::commit(1, 1, Some(Role::Provider), 0x305, 0x20_004),
        );
        snapshot = apply_rejected(
            snapshot,
            second_gate,
            Event::commit(2, 1, Some(Role::Provider), 0x305, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            second_gate,
            Event::commit(2, 1, Some(Role::Provider), 0x304, 0x20_004),
        );
        snapshot = apply_rejected(
            snapshot,
            second_gate,
            Event::commit(3, 1, Some(Role::Provider), 0x305, 0x20_004),
        );

        let second = commits(2, 0x20_004, 0x305);
        snapshot = apply_commits(snapshot, second_gate, &second);
        for role in [Role::Client, Role::ServiceManager, Role::Provider] {
            snapshot = apply_done(snapshot, second_gate, 2, role);
        }
        assert!(snapshot.is_round_complete());
        assert_eq!(snapshot.completed_rounds, 2);
        assert_eq!(snapshot.errors, 4);
        assert_eq!(snapshot.transaction_ids, [0x305, 0x306, 0x307, 0x308]);
    }

    #[test]
    fn rejection_does_not_poison_later_valid_progress() {
        let events = commits(1, 0x20_003, 0x301);
        let mut snapshot = apply_rejected(Snapshot::INITIAL, Gate::closed(), events[0]);
        snapshot = apply_rejected(snapshot, GATE, events[1]);
        snapshot = apply_commits(snapshot, GATE, &events);
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::done(1, Some(Role::Client), Some(Role::Provider)),
        );
        for role in [Role::Provider, Role::Client, Role::ServiceManager] {
            snapshot = apply_done(snapshot, GATE, 1, role);
        }
        assert!(snapshot.is_round_complete());
        assert_eq!(snapshot.errors, 3);
    }

    #[test]
    fn one_thousand_twenty_four_rounds_reuse_constant_space() {
        assert_eq!(size_of::<Snapshot>(), 48);
        let gate = Gate::open(100, 200);
        let mut snapshot = Snapshot::INITIAL;
        for round in 1..=1024 {
            let instance = 100 + round;
            let first_txid = 1_000 + (round - 1) * 4;
            snapshot = complete_round(snapshot, gate, round, instance, first_txid);
        }
        assert!(snapshot.is_round_complete());
        assert_eq!(snapshot.round, 1024);
        assert_eq!(snapshot.completed_rounds, 1024);
        assert_eq!(snapshot.instance, 1124);
        assert_eq!(snapshot.transaction_ids, [5092, 5093, 5094, 5095]);
        assert_eq!(snapshot.errors, 0);
    }

    #[test]
    fn round_identifier_instance_transaction_and_counter_boundaries_do_not_wrap() {
        let valid_start = Event::commit(1, 1, Some(Role::Provider), 0x301, 0x20_003);
        let mut snapshot = apply_rejected(
            Snapshot::INITIAL,
            GATE,
            Event::commit(0, 1, Some(Role::Provider), 0x301, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(MAX_ROUND + 1, 1, Some(Role::Provider), 0x301, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, 0, Some(Role::Provider), 0x301, 0x20_003),
        );
        snapshot = apply_rejected(
            snapshot,
            GATE,
            Event::commit(1, STEP_COUNT + 1, Some(Role::Provider), 0x301, 0x20_003),
        );
        snapshot = apply_rejected(snapshot, Gate::open(u32::MAX, 0), valid_start);
        snapshot = apply_rejected(snapshot, Gate::open(1, u32::MAX), valid_start);
        assert_eq!(snapshot.errors, 6);

        let at_max_minus_one = Snapshot {
            round: MAX_ROUND - 1,
            step: STEP_COUNT,
            completed_rounds: MAX_ROUND - 1,
            errors: 0,
            instance: 10,
            transaction_ids: [1, 2, 3, 4],
            done_reads: 3,
            done_bitmap: ALL_DONE,
        };
        let max_start = apply_accepted(
            at_max_minus_one,
            GATE,
            Event::commit(MAX_ROUND, 1, Some(Role::Provider), 5, 11),
        );
        let max_events = commits(MAX_ROUND, 11, 5);
        let mut max_complete = max_start;
        for event in &max_events[1..] {
            max_complete = apply_accepted(max_complete, GATE, *event);
        }
        for role in [Role::Provider, Role::Client, Role::ServiceManager] {
            max_complete = apply_done(max_complete, GATE, MAX_ROUND, role);
        }
        assert!(max_complete.is_round_complete());
        let overflow = reduce(
            max_complete,
            GATE,
            Event::commit(MAX_ROUND, 1, Some(Role::Provider), 9, 12),
        );
        assert_eq!(
            overflow.decision,
            Decision::Rejected(RejectReason::CounterOverflow)
        );
        let mut expected = max_complete;
        expected.errors = 1;
        assert_eq!(overflow.snapshot, expected);

        let max_instance = Snapshot {
            instance: u32::MAX,
            ..at_max_minus_one
        };
        apply_rejected(
            max_instance,
            GATE,
            Event::commit(MAX_ROUND, 1, Some(Role::Provider), 5, u32::MAX),
        );
        let max_transaction = Snapshot {
            transaction_ids: [1, 2, 3, u32::MAX],
            ..at_max_minus_one
        };
        apply_rejected(
            max_transaction,
            GATE,
            Event::commit(MAX_ROUND, 1, Some(Role::Provider), u32::MAX, 11),
        );

        let saturated = Snapshot {
            errors: u64::MAX,
            ..Snapshot::INITIAL
        };
        let transition = reduce(saturated, GATE, Event::commit(0, 0, None, 0, 0));
        assert!(matches!(transition.decision, Decision::Rejected(_)));
        assert_eq!(transition.snapshot, saturated);
    }
}
