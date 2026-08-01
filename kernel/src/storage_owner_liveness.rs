//! Kernel-owned retirement grace for a fault-latched StorageServer owner.
//!
//! This policy does not accept an EL0 heartbeat and cannot be renewed by I/O.
//! Once a physical recovery latch is observed for an authenticated `(pid,
//! broker_epoch)` binding, the exact process generation must disappear before
//! recovery may continue.  The runtime adapter is responsible for terminating
//! the process and for proving that the ordinary process reaper removed it.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnerIdentity {
    pid: u64,
    broker_epoch: u64,
}

impl OwnerIdentity {
    pub const fn new(pid: u64, broker_epoch: u64) -> Result<Self, PolicyError> {
        if pid == 0 || broker_epoch == 0 {
            return Err(PolicyError::InvalidIdentity);
        }
        Ok(Self { pid, broker_epoch })
    }

    pub const fn pid(self) -> u64 {
        self.pid
    }

    pub const fn broker_epoch(self) -> u64 {
        self.broker_epoch
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Phase {
    Idle = 0,
    Grace = 1,
    TerminationRequested = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    None,
    ForceRetire {
        owner: OwnerIdentity,
        lease_generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminationOutcome {
    Accepted,
    AlreadyTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    InvalidGrace,
    InvalidIdentity,
    CounterExhausted,
    OwnerChanged,
    OwnerMissingBeforeArm,
    BindingSurvivedReap,
    RecoveryLatchCleared,
    InvalidTransition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub phase: Phase,
    pub owner_pid: u64,
    pub broker_epoch: u64,
    pub lease_generation: u64,
    pub next_lease_generation: u64,
    pub deadline: Option<u64>,
    pub grace_counter_units: u64,
    pub arms: u64,
    pub cooperative_retirements: u64,
    pub deadlines_expired: u64,
    pub termination_requests: u64,
    pub already_terminal: u64,
    pub forced_retirements: u64,
    pub terminal_race_retirements: u64,
    pub invariant_errors: u64,
}

/// Allocation-free state machine. All runtime access is serialized by the
/// caller's single-core DAIF-masked monitor domain.
#[derive(Debug)]
pub struct OwnerLivenessPolicy {
    grace_counter_units: u64,
    phase: Phase,
    owner: Option<OwnerIdentity>,
    lease_generation: u64,
    next_lease_generation: u64,
    deadline: u64,
    termination_outcome: Option<TerminationOutcome>,
    arms: u64,
    cooperative_retirements: u64,
    deadlines_expired: u64,
    termination_requests: u64,
    already_terminal: u64,
    forced_retirements: u64,
    terminal_race_retirements: u64,
    invariant_errors: u64,
}

impl OwnerLivenessPolicy {
    /// `grace_counter_units` must fit in the serial-number half range so
    /// wrapping counter comparisons remain unambiguous.
    pub const fn new(grace_counter_units: u64) -> Result<Self, PolicyError> {
        if grace_counter_units == 0 || grace_counter_units >= (1_u64 << 63) {
            return Err(PolicyError::InvalidGrace);
        }
        Ok(Self {
            grace_counter_units,
            phase: Phase::Idle,
            owner: None,
            lease_generation: 0,
            next_lease_generation: 1,
            deadline: 0,
            termination_outcome: None,
            arms: 0,
            cooperative_retirements: 0,
            deadlines_expired: 0,
            termination_requests: 0,
            already_terminal: 0,
            forced_retirements: 0,
            terminal_race_retirements: 0,
            invariant_errors: 0,
        })
    }

    /// Observes one stable monitor snapshot.
    ///
    /// `bound_owner` is the broker binding after authentication.
    /// `tracked_process_present` reports whether the exact ticket PID still
    /// exists in the process table. The M61 runtime rejects explicit close of
    /// the process-lifetime volume capability; the policy nevertheless keeps
    /// recovery blocked until the process reaper closes accepted session
    /// handles and destroys the address space.
    pub fn observe(
        &mut self,
        now: u64,
        bound_owner: Option<OwnerIdentity>,
        recovery_latched: bool,
        tracked_process_present: bool,
    ) -> Result<Action, PolicyError> {
        let result =
            self.observe_inner(now, bound_owner, recovery_latched, tracked_process_present);
        if result.is_err() {
            self.invariant_errors = self.invariant_errors.saturating_add(1);
        }
        result
    }

    fn observe_inner(
        &mut self,
        now: u64,
        bound_owner: Option<OwnerIdentity>,
        recovery_latched: bool,
        tracked_process_present: bool,
    ) -> Result<Action, PolicyError> {
        match self.phase {
            Phase::Idle => {
                if !recovery_latched {
                    return Ok(Action::None);
                }
                let Some(owner) = bound_owner else {
                    // Ownerless recovery is outside this policy. The ordinary
                    // recovery coordinator may claim it immediately.
                    return Ok(Action::None);
                };
                if !tracked_process_present {
                    return Err(PolicyError::OwnerMissingBeforeArm);
                }
                let generation = self.next_lease_generation;
                self.next_lease_generation = generation
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
                self.arms = self
                    .arms
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
                self.phase = Phase::Grace;
                self.owner = Some(owner);
                self.lease_generation = generation;
                self.deadline = now.wrapping_add(self.grace_counter_units);
                self.termination_outcome = None;
                Ok(Action::None)
            }
            Phase::Grace | Phase::TerminationRequested => {
                let owner = self.owner.ok_or(PolicyError::InvalidTransition)?;
                if !recovery_latched {
                    return Err(PolicyError::RecoveryLatchCleared);
                }
                if bound_owner.is_some_and(|bound| bound != owner) {
                    return Err(PolicyError::OwnerChanged);
                }
                if !tracked_process_present {
                    if bound_owner.is_some() {
                        return Err(PolicyError::BindingSurvivedReap);
                    }
                    match self.phase {
                        Phase::Grace => {
                            self.cooperative_retirements = self
                                .cooperative_retirements
                                .checked_add(1)
                                .ok_or(PolicyError::CounterExhausted)?;
                        }
                        Phase::TerminationRequested => match self.termination_outcome {
                            Some(TerminationOutcome::Accepted) => {
                                self.forced_retirements = self
                                    .forced_retirements
                                    .checked_add(1)
                                    .ok_or(PolicyError::CounterExhausted)?;
                            }
                            Some(TerminationOutcome::AlreadyTerminal) => {
                                self.terminal_race_retirements = self
                                    .terminal_race_retirements
                                    .checked_add(1)
                                    .ok_or(PolicyError::CounterExhausted)?;
                            }
                            None => return Err(PolicyError::InvalidTransition),
                        },
                        Phase::Idle => return Err(PolicyError::InvalidTransition),
                    }
                    self.clear_active();
                    return Ok(Action::None);
                }
                if self.phase == Phase::Grace && deadline_reached(now, self.deadline) {
                    self.deadlines_expired = self
                        .deadlines_expired
                        .checked_add(1)
                        .ok_or(PolicyError::CounterExhausted)?;
                    self.phase = Phase::TerminationRequested;
                    return Ok(Action::ForceRetire {
                        owner,
                        lease_generation: self.lease_generation,
                    });
                }
                Ok(Action::None)
            }
        }
    }

    pub fn record_termination_outcome(
        &mut self,
        owner: OwnerIdentity,
        lease_generation: u64,
        outcome: TerminationOutcome,
    ) -> Result<(), PolicyError> {
        let valid = self.phase == Phase::TerminationRequested
            && self.owner == Some(owner)
            && self.lease_generation == lease_generation
            && self.termination_outcome.is_none();
        if !valid {
            self.invariant_errors = self.invariant_errors.saturating_add(1);
            return Err(PolicyError::InvalidTransition);
        }
        match outcome {
            TerminationOutcome::Accepted => {
                self.termination_requests = self
                    .termination_requests
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
            }
            TerminationOutcome::AlreadyTerminal => {
                self.already_terminal = self
                    .already_terminal
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
            }
        }
        self.termination_outcome = Some(outcome);
        Ok(())
    }

    pub const fn retirement_barrier_active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    pub const fn snapshot(&self) -> Snapshot {
        let (owner_pid, broker_epoch) = match self.owner {
            Some(owner) => (owner.pid, owner.broker_epoch),
            None => (0, 0),
        };
        Snapshot {
            phase: self.phase,
            owner_pid,
            broker_epoch,
            lease_generation: self.lease_generation,
            next_lease_generation: self.next_lease_generation,
            deadline: if matches!(self.phase, Phase::Idle) {
                None
            } else {
                Some(self.deadline)
            },
            grace_counter_units: self.grace_counter_units,
            arms: self.arms,
            cooperative_retirements: self.cooperative_retirements,
            deadlines_expired: self.deadlines_expired,
            termination_requests: self.termination_requests,
            already_terminal: self.already_terminal,
            forced_retirements: self.forced_retirements,
            terminal_race_retirements: self.terminal_race_retirements,
            invariant_errors: self.invariant_errors,
        }
    }

    fn clear_active(&mut self) {
        self.phase = Phase::Idle;
        self.owner = None;
        self.lease_generation = 0;
        self.deadline = 0;
        self.termination_outcome = None;
    }
}

const fn deadline_reached(now: u64, deadline: u64) -> bool {
    now.wrapping_sub(deadline) < (1_u64 << 63)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: OwnerIdentity = OwnerIdentity {
        pid: 0x0000_0001_0000_0007,
        broker_epoch: 6,
    };

    #[test]
    fn fault_latch_arms_once_and_forces_only_at_deadline() {
        let mut policy = OwnerLivenessPolicy::new(10).unwrap();
        assert_eq!(
            policy.observe(100, Some(OWNER), true, true),
            Ok(Action::None)
        );
        assert_eq!(
            policy.observe(109, Some(OWNER), true, true),
            Ok(Action::None)
        );
        assert_eq!(
            policy.observe(110, Some(OWNER), true, true),
            Ok(Action::ForceRetire {
                owner: OWNER,
                lease_generation: 1,
            })
        );
        policy
            .record_termination_outcome(OWNER, 1, TerminationOutcome::Accepted)
            .unwrap();
        assert_eq!(policy.observe(111, None, true, false), Ok(Action::None));
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.phase, Phase::Idle);
        assert_eq!(snapshot.arms, 1);
        assert_eq!(snapshot.deadlines_expired, 1);
        assert_eq!(snapshot.termination_requests, 1);
        assert_eq!(snapshot.forced_retirements, 1);
        assert_eq!(snapshot.cooperative_retirements, 0);
        assert_eq!(snapshot.invariant_errors, 0);
    }

    #[test]
    fn broker_unbind_does_not_clear_a_live_owner_ticket() {
        let mut policy = OwnerLivenessPolicy::new(4).unwrap();
        policy.observe(10, Some(OWNER), true, true).unwrap();
        assert_eq!(policy.observe(11, None, true, true), Ok(Action::None));
        assert!(policy.retirement_barrier_active());
        assert!(matches!(
            policy.observe(14, None, true, true),
            Ok(Action::ForceRetire { owner: OWNER, .. })
        ));
    }

    #[test]
    fn cooperative_reap_clears_the_barrier_without_expiry() {
        let mut policy = OwnerLivenessPolicy::new(10).unwrap();
        policy.observe(20, Some(OWNER), true, true).unwrap();
        policy.observe(21, None, true, false).unwrap();
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.phase, Phase::Idle);
        assert_eq!(snapshot.cooperative_retirements, 1);
        assert_eq!(snapshot.deadlines_expired, 0);
    }

    #[test]
    fn repeated_owner_activity_cannot_renew_the_deadline() {
        let mut policy = OwnerLivenessPolicy::new(10).unwrap();
        policy.observe(40, Some(OWNER), true, true).unwrap();
        let deadline = policy.snapshot().deadline;
        for now in 41..50 {
            assert_eq!(
                policy.observe(now, Some(OWNER), true, true),
                Ok(Action::None)
            );
            assert_eq!(policy.snapshot().deadline, deadline);
            assert_eq!(policy.snapshot().arms, 1);
        }
        assert!(matches!(
            policy.observe(50, Some(OWNER), true, true),
            Ok(Action::ForceRetire {
                lease_generation: 1,
                ..
            })
        ));
    }

    #[test]
    fn stale_termination_outcome_cannot_commit_a_new_ticket() {
        let mut policy = OwnerLivenessPolicy::new(1).unwrap();
        policy.observe(1, Some(OWNER), true, true).unwrap();
        policy.observe(2, Some(OWNER), true, true).unwrap();
        assert_eq!(
            policy.record_termination_outcome(OWNER, 2, TerminationOutcome::Accepted),
            Err(PolicyError::InvalidTransition)
        );
        let replacement = OwnerIdentity::new(0x0000_0002_0000_0007, 7).unwrap();
        assert_eq!(
            policy.record_termination_outcome(replacement, 1, TerminationOutcome::Accepted),
            Err(PolicyError::InvalidTransition)
        );
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.termination_requests, 0);
        assert_eq!(snapshot.invariant_errors, 2);
    }

    #[test]
    fn identity_change_and_latch_clear_fail_closed() {
        let replacement = OwnerIdentity::new(0x0000_0002_0000_0007, 7).unwrap();
        let mut changed = OwnerLivenessPolicy::new(10).unwrap();
        changed.observe(1, Some(OWNER), true, true).unwrap();
        assert_eq!(
            changed.observe(2, Some(replacement), true, true),
            Err(PolicyError::OwnerChanged)
        );
        assert_eq!(changed.snapshot().invariant_errors, 1);

        let mut cleared = OwnerLivenessPolicy::new(10).unwrap();
        cleared.observe(1, Some(OWNER), true, true).unwrap();
        assert_eq!(
            cleared.observe(2, Some(OWNER), false, true),
            Err(PolicyError::RecoveryLatchCleared)
        );
    }

    #[test]
    fn counter_wrap_keeps_the_deadline_exact() {
        let mut policy = OwnerLivenessPolicy::new(4).unwrap();
        policy
            .observe(u64::MAX - 2, Some(OWNER), true, true)
            .unwrap();
        assert_eq!(policy.snapshot().deadline, Some(1));
        assert_eq!(policy.observe(0, Some(OWNER), true, true), Ok(Action::None));
        assert!(matches!(
            policy.observe(1, Some(OWNER), true, true),
            Ok(Action::ForceRetire { .. })
        ));
    }

    #[test]
    fn already_terminal_race_is_accounted_separately() {
        let mut policy = OwnerLivenessPolicy::new(1).unwrap();
        policy.observe(1, Some(OWNER), true, true).unwrap();
        policy.observe(2, Some(OWNER), true, true).unwrap();
        policy
            .record_termination_outcome(OWNER, 1, TerminationOutcome::AlreadyTerminal)
            .unwrap();
        policy.observe(3, None, true, false).unwrap();
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.already_terminal, 1);
        assert_eq!(snapshot.terminal_race_retirements, 1);
        assert_eq!(snapshot.forced_retirements, 0);
    }

    #[test]
    fn rejects_zero_and_ambiguous_configuration() {
        assert_eq!(OwnerIdentity::new(0, 1), Err(PolicyError::InvalidIdentity));
        assert_eq!(OwnerIdentity::new(1, 0), Err(PolicyError::InvalidIdentity));
        assert!(matches!(
            OwnerLivenessPolicy::new(0),
            Err(PolicyError::InvalidGrace)
        ));
        assert!(matches!(
            OwnerLivenessPolicy::new(1_u64 << 63),
            Err(PolicyError::InvalidGrace)
        ));
    }
}
