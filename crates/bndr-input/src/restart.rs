//! Allocation-free InputServer restart budget and authorization state.
//!
//! The first recovery slice permits exactly one automatic replacement. A
//! second active-generation fault, a failed replacement, or a failed resync
//! enters terminal quarantine. Merely reaching `Active` does not replenish
//! the budget.

use crate::OwnerPid;

pub const INPUT_RESTART_BUDGET: u8 = 1;
pub const INPUT_RESTART_BACKOFF_NS: u64 = 30_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRestartState {
    Active,
    Backoff,
    AwaitingReplacement,
    Resyncing,
    Quarantined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputQuarantineReason {
    ActiveFaultBudgetExhausted,
    ReplacementFailed,
    ResyncFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRestartError {
    ZeroInputSession,
    ZeroRouteEpoch,
    SequenceExhausted,
    InvalidState,
    PidMismatch {
        expected: OwnerPid,
        observed: OwnerPid,
    },
    SessionMismatch {
        expected: u64,
        observed: u64,
    },
    EpochMismatch {
        expected: u64,
        observed: u64,
    },
    FloorMismatch {
        expected: u64,
        observed: u64,
    },
    FloorRegression {
        minimum: u64,
        observed: u64,
    },
    SameInputProcess,
    BackoffNotElapsed {
        required_ns: u64,
        observed_ns: u64,
    },
    AuthorizationMismatch,
}

/// One generation-qualified InputServer identity and its physical replay floor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputServiceGeneration {
    pid: OwnerPid,
    input_session_id: u64,
    route_epoch: u64,
    physical_sequence_floor: u64,
}

impl InputServiceGeneration {
    pub fn try_new(
        pid: OwnerPid,
        input_session_id: u64,
        route_epoch: u64,
        physical_sequence_floor: u64,
    ) -> Result<Self, InputRestartError> {
        if input_session_id == 0 {
            return Err(InputRestartError::ZeroInputSession);
        }
        if route_epoch == 0 {
            return Err(InputRestartError::ZeroRouteEpoch);
        }
        Ok(Self {
            pid,
            input_session_id,
            route_epoch,
            physical_sequence_floor,
        })
    }

    pub const fn pid(self) -> OwnerPid {
        self.pid
    }

    pub const fn input_session_id(self) -> u64 {
        self.input_session_id
    }

    pub const fn route_epoch(self) -> u64 {
        self.route_epoch
    }

    pub const fn physical_sequence_floor(self) -> u64 {
        self.physical_sequence_floor
    }
}

/// Exact authority handed to Init after the one backoff completes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputReplacementAuthorization {
    attempt: u8,
    failed_generation: InputServiceGeneration,
    required_input_session_id: u64,
    required_route_epoch: u64,
    minimum_physical_sequence_floor: u64,
}

impl InputReplacementAuthorization {
    pub const fn attempt(self) -> u8 {
        self.attempt
    }

    pub const fn failed_generation(self) -> InputServiceGeneration {
        self.failed_generation
    }

    pub const fn required_input_session_id(self) -> u64 {
        self.required_input_session_id
    }

    pub const fn required_route_epoch(self) -> u64 {
        self.required_route_epoch
    }

    pub const fn minimum_physical_sequence_floor(self) -> u64 {
        self.minimum_physical_sequence_floor
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRestartSnapshot {
    state: InputRestartState,
    current: InputServiceGeneration,
    candidate: Option<InputServiceGeneration>,
    attempts_used: u8,
    required_backoff_ns: u64,
    authorization: Option<InputReplacementAuthorization>,
    quarantine_reason: Option<InputQuarantineReason>,
}

impl InputRestartSnapshot {
    pub const fn state(self) -> InputRestartState {
        self.state
    }

    pub const fn current(self) -> InputServiceGeneration {
        self.current
    }

    pub const fn candidate(self) -> Option<InputServiceGeneration> {
        self.candidate
    }

    pub const fn attempts_used(self) -> u8 {
        self.attempts_used
    }

    pub const fn budget_remaining(self) -> u8 {
        INPUT_RESTART_BUDGET - self.attempts_used
    }

    pub const fn required_backoff_ns(self) -> u64 {
        self.required_backoff_ns
    }

    pub const fn authorization(self) -> Option<InputReplacementAuthorization> {
        self.authorization
    }

    pub const fn quarantine_reason(self) -> Option<InputQuarantineReason> {
        self.quarantine_reason
    }
}

/// One bounded, fail-closed InputServer restart policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRestartPolicy {
    state: InputRestartState,
    current: InputServiceGeneration,
    candidate: Option<InputServiceGeneration>,
    attempts_used: u8,
    quarantine_reason: Option<InputQuarantineReason>,
}

impl InputRestartPolicy {
    /// Creates an active policy. The first generation must leave room for the
    /// one authorized successor session and route epoch.
    pub fn try_new(current: InputServiceGeneration) -> Result<Self, InputRestartError> {
        if current.input_session_id.checked_add(1).is_none()
            || current.route_epoch.checked_add(1).is_none()
        {
            return Err(InputRestartError::SequenceExhausted);
        }
        Ok(Self {
            state: InputRestartState::Active,
            current,
            candidate: None,
            attempts_used: 0,
            quarantine_reason: None,
        })
    }

    pub fn snapshot(&self) -> InputRestartSnapshot {
        InputRestartSnapshot {
            state: self.state,
            current: self.current,
            candidate: self.candidate,
            attempts_used: self.attempts_used,
            required_backoff_ns: if self.state == InputRestartState::Backoff {
                INPUT_RESTART_BACKOFF_NS
            } else {
                0
            },
            authorization: self.replacement_authorization(),
            quarantine_reason: self.quarantine_reason,
        }
    }

    pub fn active_fault(
        &mut self,
        observed: InputServiceGeneration,
    ) -> Result<(), InputRestartError> {
        if self.state != InputRestartState::Active {
            return Err(InputRestartError::InvalidState);
        }
        validate_exact_generation(self.current, observed)?;
        if self.attempts_used == INPUT_RESTART_BUDGET {
            self.state = InputRestartState::Quarantined;
            self.quarantine_reason = Some(InputQuarantineReason::ActiveFaultBudgetExhausted);
            return Ok(());
        }
        if self.current.input_session_id.checked_add(1).is_none()
            || self.current.route_epoch.checked_add(1).is_none()
        {
            return Err(InputRestartError::SequenceExhausted);
        }
        self.attempts_used += 1;
        self.state = InputRestartState::Backoff;
        Ok(())
    }

    pub fn backoff_elapsed(&mut self, elapsed_ns: u64) -> Result<(), InputRestartError> {
        if self.state != InputRestartState::Backoff {
            return Err(InputRestartError::InvalidState);
        }
        if elapsed_ns < INPUT_RESTART_BACKOFF_NS {
            return Err(InputRestartError::BackoffNotElapsed {
                required_ns: INPUT_RESTART_BACKOFF_NS,
                observed_ns: elapsed_ns,
            });
        }
        self.state = InputRestartState::AwaitingReplacement;
        Ok(())
    }

    pub fn replacement_authorization(&self) -> Option<InputReplacementAuthorization> {
        if self.state != InputRestartState::AwaitingReplacement {
            return None;
        }
        Some(InputReplacementAuthorization {
            attempt: self.attempts_used,
            failed_generation: self.current,
            required_input_session_id: self
                .current
                .input_session_id
                .checked_add(1)
                .expect("active policy reserved one replacement session"),
            required_route_epoch: self
                .current
                .route_epoch
                .checked_add(1)
                .expect("active policy reserved one replacement epoch"),
            minimum_physical_sequence_floor: self.current.physical_sequence_floor,
        })
    }

    pub fn begin_replacement(
        &mut self,
        candidate: InputServiceGeneration,
    ) -> Result<(), InputRestartError> {
        let authorization = self
            .replacement_authorization()
            .ok_or(InputRestartError::InvalidState)?;
        if candidate.pid == authorization.failed_generation.pid {
            return Err(InputRestartError::SameInputProcess);
        }
        if candidate.input_session_id != authorization.required_input_session_id {
            return Err(InputRestartError::SessionMismatch {
                expected: authorization.required_input_session_id,
                observed: candidate.input_session_id,
            });
        }
        if candidate.route_epoch != authorization.required_route_epoch {
            return Err(InputRestartError::EpochMismatch {
                expected: authorization.required_route_epoch,
                observed: candidate.route_epoch,
            });
        }
        if candidate.physical_sequence_floor < authorization.minimum_physical_sequence_floor {
            return Err(InputRestartError::FloorRegression {
                minimum: authorization.minimum_physical_sequence_floor,
                observed: candidate.physical_sequence_floor,
            });
        }
        self.candidate = Some(candidate);
        self.state = InputRestartState::Resyncing;
        Ok(())
    }

    pub fn replacement_failed(
        &mut self,
        authorization: InputReplacementAuthorization,
    ) -> Result<(), InputRestartError> {
        let expected = self
            .replacement_authorization()
            .ok_or(InputRestartError::InvalidState)?;
        if authorization != expected {
            return Err(InputRestartError::AuthorizationMismatch);
        }
        self.state = InputRestartState::Quarantined;
        self.quarantine_reason = Some(InputQuarantineReason::ReplacementFailed);
        Ok(())
    }

    pub fn resync_succeeded(
        &mut self,
        observed: InputServiceGeneration,
    ) -> Result<(), InputRestartError> {
        if self.state != InputRestartState::Resyncing {
            return Err(InputRestartError::InvalidState);
        }
        let candidate = self.candidate.ok_or(InputRestartError::InvalidState)?;
        validate_exact_generation(candidate, observed)?;
        self.current = candidate;
        self.candidate = None;
        self.state = InputRestartState::Active;
        Ok(())
    }

    pub fn resync_failed(
        &mut self,
        observed: InputServiceGeneration,
    ) -> Result<(), InputRestartError> {
        if self.state != InputRestartState::Resyncing {
            return Err(InputRestartError::InvalidState);
        }
        let candidate = self.candidate.ok_or(InputRestartError::InvalidState)?;
        validate_exact_generation(candidate, observed)?;
        self.state = InputRestartState::Quarantined;
        self.quarantine_reason = Some(InputQuarantineReason::ResyncFailed);
        Ok(())
    }
}

fn validate_exact_generation(
    expected: InputServiceGeneration,
    observed: InputServiceGeneration,
) -> Result<(), InputRestartError> {
    if observed.pid != expected.pid {
        return Err(InputRestartError::PidMismatch {
            expected: expected.pid,
            observed: observed.pid,
        });
    }
    if observed.input_session_id != expected.input_session_id {
        return Err(InputRestartError::SessionMismatch {
            expected: expected.input_session_id,
            observed: observed.input_session_id,
        });
    }
    if observed.route_epoch != expected.route_epoch {
        return Err(InputRestartError::EpochMismatch {
            expected: expected.route_epoch,
            observed: observed.route_epoch,
        });
    }
    if observed.physical_sequence_floor != expected.physical_sequence_floor {
        return Err(InputRestartError::FloorMismatch {
            expected: expected.physical_sequence_floor,
            observed: observed.physical_sequence_floor,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(raw: u64) -> OwnerPid {
        OwnerPid::try_new(raw).unwrap()
    }

    fn generation(raw_pid: u64, session: u64, epoch: u64, floor: u64) -> InputServiceGeneration {
        InputServiceGeneration::try_new(pid(raw_pid), session, epoch, floor).unwrap()
    }

    fn active() -> InputServiceGeneration {
        generation(0x1_0000_0006, 1, 1, 9)
    }

    fn replacement() -> InputServiceGeneration {
        generation(0x2_0000_0006, 2, 2, 11)
    }

    fn awaiting_policy() -> InputRestartPolicy {
        let current = active();
        let mut policy = InputRestartPolicy::try_new(current).unwrap();
        policy.active_fault(current).unwrap();
        policy.backoff_elapsed(INPUT_RESTART_BACKOFF_NS).unwrap();
        policy
    }

    fn resyncing_policy() -> InputRestartPolicy {
        let mut policy = awaiting_policy();
        policy.begin_replacement(replacement()).unwrap();
        policy
    }

    #[test]
    fn generation_requires_nonzero_session_and_epoch() {
        assert_eq!(
            InputServiceGeneration::try_new(pid(1), 0, 1, 0),
            Err(InputRestartError::ZeroInputSession)
        );
        assert_eq!(
            InputServiceGeneration::try_new(pid(1), 1, 0, 0),
            Err(InputRestartError::ZeroRouteEpoch)
        );
    }

    #[test]
    fn policy_reserves_exactly_one_successor_generation() {
        assert_eq!(INPUT_RESTART_BUDGET, 1);
        assert_eq!(INPUT_RESTART_BACKOFF_NS, 30_000_000);
        assert_eq!(
            InputRestartPolicy::try_new(generation(1, u64::MAX, 1, 0)),
            Err(InputRestartError::SequenceExhausted)
        );
        assert_eq!(
            InputRestartPolicy::try_new(generation(1, 1, u64::MAX, 0)),
            Err(InputRestartError::SequenceExhausted)
        );
    }

    #[test]
    fn complete_happy_path_reaches_active_without_replenishing_budget() {
        let current = active();
        let candidate = replacement();
        let mut policy = InputRestartPolicy::try_new(current).unwrap();
        assert_eq!(policy.snapshot().state(), InputRestartState::Active);
        assert_eq!(policy.snapshot().budget_remaining(), 1);

        policy.active_fault(current).unwrap();
        assert_eq!(policy.snapshot().state(), InputRestartState::Backoff);
        assert_eq!(
            policy.snapshot().required_backoff_ns(),
            INPUT_RESTART_BACKOFF_NS
        );
        assert_eq!(policy.snapshot().attempts_used(), 1);
        assert_eq!(policy.snapshot().budget_remaining(), 0);

        policy.backoff_elapsed(INPUT_RESTART_BACKOFF_NS).unwrap();
        let authorization = policy.snapshot().authorization().unwrap();
        assert_eq!(authorization.attempt(), 1);
        assert_eq!(authorization.failed_generation(), current);
        assert_eq!(authorization.required_input_session_id(), 2);
        assert_eq!(authorization.required_route_epoch(), 2);
        assert_eq!(authorization.minimum_physical_sequence_floor(), 9);

        policy.begin_replacement(candidate).unwrap();
        assert_eq!(policy.snapshot().state(), InputRestartState::Resyncing);
        assert_eq!(policy.snapshot().candidate(), Some(candidate));
        policy.resync_succeeded(candidate).unwrap();
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.state(), InputRestartState::Active);
        assert_eq!(snapshot.current(), candidate);
        assert_eq!(snapshot.candidate(), None);
        assert_eq!(snapshot.attempts_used(), 1);
        assert_eq!(snapshot.budget_remaining(), 0);
        assert_eq!(snapshot.authorization(), None);
    }

    #[test]
    fn early_backoff_is_transactional() {
        let current = active();
        let mut policy = InputRestartPolicy::try_new(current).unwrap();
        policy.active_fault(current).unwrap();
        let before = policy;
        assert_eq!(
            policy.backoff_elapsed(INPUT_RESTART_BACKOFF_NS - 1),
            Err(InputRestartError::BackoffNotElapsed {
                required_ns: INPUT_RESTART_BACKOFF_NS,
                observed_ns: INPUT_RESTART_BACKOFF_NS - 1,
            })
        );
        assert_eq!(policy, before);
    }

    #[test]
    fn wrong_active_fault_identity_fields_are_transactional() {
        let current = active();
        for (observed, expected) in [
            (
                generation(0x2_0000_0006, 1, 1, 9),
                InputRestartError::PidMismatch {
                    expected: current.pid(),
                    observed: pid(0x2_0000_0006),
                },
            ),
            (
                generation(current.pid().get(), 2, 1, 9),
                InputRestartError::SessionMismatch {
                    expected: 1,
                    observed: 2,
                },
            ),
            (
                generation(current.pid().get(), 1, 2, 9),
                InputRestartError::EpochMismatch {
                    expected: 1,
                    observed: 2,
                },
            ),
            (
                generation(current.pid().get(), 1, 1, 10),
                InputRestartError::FloorMismatch {
                    expected: 9,
                    observed: 10,
                },
            ),
        ] {
            let mut policy = InputRestartPolicy::try_new(current).unwrap();
            let before = policy;
            assert_eq!(policy.active_fault(observed), Err(expected));
            assert_eq!(policy, before);
        }
    }

    #[test]
    fn replacement_requires_new_pid_exact_session_epoch_and_nonregressing_floor() {
        let cases = [
            (
                generation(active().pid().get(), 2, 2, 11),
                InputRestartError::SameInputProcess,
            ),
            (
                generation(replacement().pid().get(), 3, 2, 11),
                InputRestartError::SessionMismatch {
                    expected: 2,
                    observed: 3,
                },
            ),
            (
                generation(replacement().pid().get(), 2, 3, 11),
                InputRestartError::EpochMismatch {
                    expected: 2,
                    observed: 3,
                },
            ),
            (
                generation(replacement().pid().get(), 2, 2, 8),
                InputRestartError::FloorRegression {
                    minimum: 9,
                    observed: 8,
                },
            ),
        ];
        for (candidate, expected) in cases {
            let mut policy = awaiting_policy();
            let before = policy;
            assert_eq!(policy.begin_replacement(candidate), Err(expected));
            assert_eq!(policy, before);
        }
    }

    #[test]
    fn wrong_resync_identity_fields_are_transactional() {
        let candidate = replacement();
        let observations = [
            generation(0x3_0000_0006, 2, 2, 11),
            generation(candidate.pid().get(), 3, 2, 11),
            generation(candidate.pid().get(), 2, 3, 11),
            generation(candidate.pid().get(), 2, 2, 12),
        ];
        for observed in observations {
            let mut policy = resyncing_policy();
            let before = policy;
            assert!(policy.resync_succeeded(observed).is_err());
            assert_eq!(policy, before);
        }
    }

    #[test]
    fn second_active_fault_enters_terminal_quarantine() {
        let candidate = replacement();
        let mut policy = resyncing_policy();
        policy.resync_succeeded(candidate).unwrap();
        policy.active_fault(candidate).unwrap();
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.state(), InputRestartState::Quarantined);
        assert_eq!(
            snapshot.quarantine_reason(),
            Some(InputQuarantineReason::ActiveFaultBudgetExhausted)
        );
        assert_eq!(snapshot.attempts_used(), 1);
        let before = policy;
        assert_eq!(
            policy.active_fault(candidate),
            Err(InputRestartError::InvalidState)
        );
        assert_eq!(policy, before);
        assert_eq!(
            policy.backoff_elapsed(INPUT_RESTART_BACKOFF_NS),
            Err(InputRestartError::InvalidState)
        );
        assert_eq!(policy, before);
    }

    #[test]
    fn authorized_replacement_failure_enters_terminal_quarantine() {
        let mut policy = awaiting_policy();
        let authorization = policy.replacement_authorization().unwrap();
        policy.replacement_failed(authorization).unwrap();
        assert_eq!(policy.snapshot().state(), InputRestartState::Quarantined);
        assert_eq!(
            policy.snapshot().quarantine_reason(),
            Some(InputQuarantineReason::ReplacementFailed)
        );
    }

    #[test]
    fn wrong_replacement_failure_authorization_is_transactional() {
        let mut policy = awaiting_policy();
        let mut authorization = policy.replacement_authorization().unwrap();
        authorization.required_route_epoch += 1;
        let before = policy;
        assert_eq!(
            policy.replacement_failed(authorization),
            Err(InputRestartError::AuthorizationMismatch)
        );
        assert_eq!(policy, before);
    }

    #[test]
    fn exact_resync_failure_enters_terminal_quarantine() {
        let candidate = replacement();
        let mut policy = resyncing_policy();
        policy.resync_failed(candidate).unwrap();
        assert_eq!(policy.snapshot().state(), InputRestartState::Quarantined);
        assert_eq!(
            policy.snapshot().quarantine_reason(),
            Some(InputQuarantineReason::ResyncFailed)
        );
        assert_eq!(policy.snapshot().candidate(), Some(candidate));
    }

    #[test]
    fn out_of_order_transitions_are_transactional() {
        let current = active();
        let candidate = replacement();
        let mut policy = InputRestartPolicy::try_new(current).unwrap();
        let before = policy;
        assert_eq!(
            policy.backoff_elapsed(INPUT_RESTART_BACKOFF_NS),
            Err(InputRestartError::InvalidState)
        );
        assert_eq!(policy, before);
        assert_eq!(
            policy.begin_replacement(candidate),
            Err(InputRestartError::InvalidState)
        );
        assert_eq!(policy, before);
        assert_eq!(
            policy.resync_succeeded(candidate),
            Err(InputRestartError::InvalidState)
        );
        assert_eq!(policy, before);
    }
}
