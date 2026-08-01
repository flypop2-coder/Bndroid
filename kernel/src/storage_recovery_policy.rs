//! Pure bounded-recovery policy for a block device.
//!
//! A physical reset is not sufficient evidence that storage is usable. A
//! successful physical recovery therefore enters [`RecoveryState::Probation`]
//! and retains its [`AttemptTicket`]. Only a successful post-recovery I/O may
//! return the policy to [`RecoveryState::Healthy`] and clear the consecutive
//! failure streak.
//!
//! The policy has no allocation, architecture, or device dependencies. A
//! kernel adapter supplies architectural counter values and serializes calls.

use crate::time::deadline_reached;

/// Maximum number of consecutive, unconfirmed recovery attempts.
pub const ATTEMPT_LIMIT: u8 = 3;
/// Multiplier between the first and second retry delays.
pub const BACKOFF_MULTIPLIER: u64 = 2;

/// Durable policy state for the current boot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RecoveryState {
    Healthy = 0,
    Recovering = 1,
    Backoff = 2,
    Probation = 3,
    Offline = 4,
}

/// A non-reusable identity for one physical recovery attempt.
///
/// `generation` never wraps or aliases an earlier attempt. `ordinal` is the
/// one-based position in the current consecutive-failure streak.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttemptTicket {
    pub generation: u64,
    pub ordinal: u8,
}

/// Invalid policy configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    ZeroBaseBackoff,
    BackoffTooLarge,
}

/// Why an attempted transition was rejected without changing policy state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    /// The retry deadline has not yet been reached.
    BackoffPending { deadline: u64 },
    /// The operation is not valid in the current state.
    InvalidState { state: RecoveryState },
    /// The ticket is not the active recovery attempt.
    StaleTicket { active_generation: Option<u64> },
    /// No further tickets can be issued without reusing an identity.
    AttemptGenerationExhausted,
    /// Offline is sticky for the remainder of this boot.
    Offline,
}

/// Result of a failed recovery attempt or failed probation I/O.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureDisposition {
    Backoff {
        failed_attempt: u8,
        delay_ticks: u64,
        deadline: u64,
    },
    Offline {
        failed_attempt: u8,
        consecutive_failures: u8,
    },
}

/// Immutable state and exact bounded-campaign telemetry.
///
/// Telemetry counters saturate instead of wrapping. They are exact until
/// `u64::MAX`, well beyond any realizable boot-local campaign.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryPolicySnapshot {
    pub state: RecoveryState,
    pub active_attempt_generation: u64,
    pub active_attempt_ordinal: u8,
    pub consecutive_failures: u8,
    pub backoff_deadline: Option<u64>,
    pub base_backoff_ticks: u64,
    pub backoff_multiplier: u64,
    pub attempt_limit: u8,
    pub attempts_started: u64,
    pub physical_successes: u64,
    pub physical_failures: u64,
    pub attempt_failures: u64,
    pub backoffs_scheduled: u64,
    pub backoffs_completed: u64,
    pub total_backoff_ticks: u64,
    pub early_attempt_rejections: u64,
    pub probation_entries: u64,
    pub probation_successes: u64,
    pub probation_failures: u64,
    pub healthy_transitions: u64,
    pub offline_transitions: u64,
    pub offline_rejections: u64,
    pub stale_ticket_rejections: u64,
    pub invalid_transition_rejections: u64,
    pub generation_exhaustions: u64,
}

/// Boot-local bounded-recovery state machine.
pub struct RecoveryPolicy {
    state: RecoveryState,
    base_backoff_ticks: u64,
    active_attempt: Option<AttemptTicket>,
    last_attempt_generation: u64,
    consecutive_failures: u8,
    backoff_deadline: Option<u64>,
    physical_successes: u64,
    physical_failures: u64,
    attempt_failures: u64,
    backoffs_scheduled: u64,
    backoffs_completed: u64,
    total_backoff_ticks: u64,
    early_attempt_rejections: u64,
    probation_entries: u64,
    probation_successes: u64,
    probation_failures: u64,
    healthy_transitions: u64,
    offline_transitions: u64,
    offline_rejections: u64,
    stale_ticket_rejections: u64,
    invalid_transition_rejections: u64,
    generation_exhaustions: u64,
}

impl RecoveryPolicy {
    /// Creates a healthy policy with a caller-selected first retry delay.
    ///
    /// The second retry delay is `base_backoff_ticks * 2`. Both delays must
    /// fit within the unambiguous half of the architectural counter space.
    pub const fn new(base_backoff_ticks: u64) -> Result<Self, ConfigError> {
        if base_backoff_ticks == 0 {
            return Err(ConfigError::ZeroBaseBackoff);
        }
        if base_backoff_ticks > (i64::MAX as u64) / BACKOFF_MULTIPLIER {
            return Err(ConfigError::BackoffTooLarge);
        }

        Ok(Self {
            state: RecoveryState::Healthy,
            base_backoff_ticks,
            active_attempt: None,
            last_attempt_generation: 0,
            consecutive_failures: 0,
            backoff_deadline: None,
            physical_successes: 0,
            physical_failures: 0,
            attempt_failures: 0,
            backoffs_scheduled: 0,
            backoffs_completed: 0,
            total_backoff_ticks: 0,
            early_attempt_rejections: 0,
            probation_entries: 0,
            probation_successes: 0,
            probation_failures: 0,
            healthy_transitions: 0,
            offline_transitions: 0,
            offline_rejections: 0,
            stale_ticket_rejections: 0,
            invalid_transition_rejections: 0,
            generation_exhaustions: 0,
        })
    }

    pub const fn state(&self) -> RecoveryState {
        self.state
    }

    pub const fn active_attempt(&self) -> Option<AttemptTicket> {
        self.active_attempt
    }

    pub const fn snapshot(&self) -> RecoveryPolicySnapshot {
        let (active_attempt_generation, active_attempt_ordinal) = match self.active_attempt {
            Some(ticket) => (ticket.generation, ticket.ordinal),
            None => (0, 0),
        };
        RecoveryPolicySnapshot {
            state: self.state,
            active_attempt_generation,
            active_attempt_ordinal,
            consecutive_failures: self.consecutive_failures,
            backoff_deadline: self.backoff_deadline,
            base_backoff_ticks: self.base_backoff_ticks,
            backoff_multiplier: BACKOFF_MULTIPLIER,
            attempt_limit: ATTEMPT_LIMIT,
            attempts_started: self.last_attempt_generation,
            physical_successes: self.physical_successes,
            physical_failures: self.physical_failures,
            attempt_failures: self.attempt_failures,
            backoffs_scheduled: self.backoffs_scheduled,
            backoffs_completed: self.backoffs_completed,
            total_backoff_ticks: self.total_backoff_ticks,
            early_attempt_rejections: self.early_attempt_rejections,
            probation_entries: self.probation_entries,
            probation_successes: self.probation_successes,
            probation_failures: self.probation_failures,
            healthy_transitions: self.healthy_transitions,
            offline_transitions: self.offline_transitions,
            offline_rejections: self.offline_rejections,
            stale_ticket_rejections: self.stale_ticket_rejections,
            invalid_transition_rejections: self.invalid_transition_rejections,
            generation_exhaustions: self.generation_exhaustions,
        }
    }

    /// Starts the first attempt for an incident, or the next attempt after an
    /// elapsed backoff. Calling this before the deadline records and rejects
    /// an early attempt without issuing a ticket.
    pub fn begin_attempt(&mut self, now: u64) -> Result<AttemptTicket, RejectReason> {
        match self.state {
            RecoveryState::Healthy => self.issue_ticket(),
            RecoveryState::Backoff => {
                let deadline = self
                    .backoff_deadline
                    .expect("Backoff state always owns a deadline");
                if !deadline_reached(now, deadline) {
                    increment(&mut self.early_attempt_rejections);
                    return Err(RejectReason::BackoffPending { deadline });
                }
                let ticket = self.issue_ticket()?;
                self.backoff_deadline = None;
                increment(&mut self.backoffs_completed);
                Ok(ticket)
            }
            RecoveryState::Offline => {
                increment(&mut self.offline_rejections);
                Err(RejectReason::Offline)
            }
            state @ (RecoveryState::Recovering | RecoveryState::Probation) => {
                increment(&mut self.invalid_transition_rejections);
                Err(RejectReason::InvalidState { state })
            }
        }
    }

    /// Records completion of the physical recovery and enters probation.
    pub fn complete_physical_success(&mut self, ticket: AttemptTicket) -> Result<(), RejectReason> {
        self.validate_ticket(ticket, RecoveryState::Recovering)?;
        self.state = RecoveryState::Probation;
        increment(&mut self.physical_successes);
        increment(&mut self.probation_entries);
        Ok(())
    }

    /// Records a failed physical recovery attempt.
    pub fn complete_physical_failure(
        &mut self,
        ticket: AttemptTicket,
        now: u64,
    ) -> Result<FailureDisposition, RejectReason> {
        self.validate_ticket(ticket, RecoveryState::Recovering)?;
        increment(&mut self.physical_failures);
        Ok(self.fail_active_attempt(ticket, now))
    }

    /// Confirms that real I/O after recovery succeeded.
    ///
    /// This is the only transition that clears the failure streak and returns
    /// the policy to healthy.
    pub fn complete_probation_success(
        &mut self,
        ticket: AttemptTicket,
    ) -> Result<(), RejectReason> {
        self.validate_ticket(ticket, RecoveryState::Probation)?;
        self.state = RecoveryState::Healthy;
        self.active_attempt = None;
        self.consecutive_failures = 0;
        increment(&mut self.probation_successes);
        increment(&mut self.healthy_transitions);
        Ok(())
    }

    /// Treats failed post-recovery I/O as failure of the same attempt.
    pub fn complete_probation_failure(
        &mut self,
        ticket: AttemptTicket,
        now: u64,
    ) -> Result<FailureDisposition, RejectReason> {
        self.validate_ticket(ticket, RecoveryState::Probation)?;
        increment(&mut self.probation_failures);
        Ok(self.fail_active_attempt(ticket, now))
    }

    fn issue_ticket(&mut self) -> Result<AttemptTicket, RejectReason> {
        let Some(generation) = self.last_attempt_generation.checked_add(1) else {
            increment(&mut self.generation_exhaustions);
            return Err(RejectReason::AttemptGenerationExhausted);
        };
        let ticket = AttemptTicket {
            generation,
            ordinal: self.consecutive_failures + 1,
        };
        self.last_attempt_generation = generation;
        self.active_attempt = Some(ticket);
        self.state = RecoveryState::Recovering;
        Ok(ticket)
    }

    fn validate_ticket(
        &mut self,
        ticket: AttemptTicket,
        expected_state: RecoveryState,
    ) -> Result<(), RejectReason> {
        if self.state == RecoveryState::Offline {
            increment(&mut self.offline_rejections);
            return Err(RejectReason::Offline);
        }
        if self.active_attempt != Some(ticket) {
            increment(&mut self.stale_ticket_rejections);
            return Err(RejectReason::StaleTicket {
                active_generation: self.active_attempt.map(|active| active.generation),
            });
        }
        if self.state != expected_state {
            increment(&mut self.invalid_transition_rejections);
            return Err(RejectReason::InvalidState { state: self.state });
        }
        Ok(())
    }

    fn fail_active_attempt(&mut self, ticket: AttemptTicket, now: u64) -> FailureDisposition {
        self.active_attempt = None;
        increment(&mut self.attempt_failures);
        self.consecutive_failures += 1;

        if self.consecutive_failures == ATTEMPT_LIMIT {
            self.state = RecoveryState::Offline;
            self.backoff_deadline = None;
            increment(&mut self.offline_transitions);
            return FailureDisposition::Offline {
                failed_attempt: ticket.ordinal,
                consecutive_failures: self.consecutive_failures,
            };
        }

        let delay_ticks = if self.consecutive_failures == 1 {
            self.base_backoff_ticks
        } else {
            self.base_backoff_ticks * BACKOFF_MULTIPLIER
        };
        let deadline = now.wrapping_add(delay_ticks);
        self.state = RecoveryState::Backoff;
        self.backoff_deadline = Some(deadline);
        increment(&mut self.backoffs_scheduled);
        self.total_backoff_ticks = self.total_backoff_ticks.saturating_add(delay_ticks);
        FailureDisposition::Backoff {
            failed_attempt: ticket.ordinal,
            delay_ticks,
            deadline,
        }
    }
}

fn increment(counter: &mut u64) {
    *counter = counter.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::{
        ATTEMPT_LIMIT, AttemptTicket, BACKOFF_MULTIPLIER, ConfigError, FailureDisposition,
        RecoveryPolicy, RecoveryState, RejectReason,
    };

    fn policy(base: u64) -> RecoveryPolicy {
        RecoveryPolicy::new(base).unwrap()
    }

    #[test]
    fn validates_unambiguous_backoff_configuration() {
        assert!(matches!(
            RecoveryPolicy::new(0),
            Err(ConfigError::ZeroBaseBackoff)
        ));
        assert!(matches!(
            RecoveryPolicy::new((i64::MAX as u64) / BACKOFF_MULTIPLIER + 1),
            Err(ConfigError::BackoffTooLarge)
        ));
        assert!(RecoveryPolicy::new((i64::MAX as u64) / BACKOFF_MULTIPLIER).is_ok());

        let snapshot = policy(20).snapshot();
        assert_eq!(snapshot.state, RecoveryState::Healthy);
        assert_eq!(snapshot.base_backoff_ticks, 20);
        assert_eq!(snapshot.backoff_multiplier, 2);
        assert_eq!(snapshot.attempt_limit, ATTEMPT_LIMIT);
        assert_eq!(snapshot.active_attempt_generation, 0);
    }

    #[test]
    fn first_failure_enforces_base_backoff_without_issuing_an_early_ticket() {
        let mut policy = policy(20);
        let first = policy.begin_attempt(100).unwrap();
        assert_eq!(first.generation, 1);
        assert_eq!(first.ordinal, 1);
        assert_eq!(
            policy.complete_physical_failure(first, 100),
            Ok(FailureDisposition::Backoff {
                failed_attempt: 1,
                delay_ticks: 20,
                deadline: 120,
            })
        );

        assert_eq!(
            policy.begin_attempt(119),
            Err(RejectReason::BackoffPending { deadline: 120 })
        );
        let early = policy.snapshot();
        assert_eq!(early.attempts_started, 1);
        assert_eq!(early.early_attempt_rejections, 1);
        assert_eq!(early.backoffs_scheduled, 1);
        assert_eq!(early.backoffs_completed, 0);

        let second = policy.begin_attempt(120).unwrap();
        assert_eq!(second.generation, 2);
        assert_eq!(second.ordinal, 2);
        let started = policy.snapshot();
        assert_eq!(started.state, RecoveryState::Recovering);
        assert_eq!(started.backoffs_completed, 1);
        assert_eq!(started.active_attempt_generation, 2);
        assert_eq!(started.active_attempt_ordinal, 2);
    }

    #[test]
    fn second_failure_uses_double_backoff_and_third_failure_is_offline() {
        let mut policy = policy(20);
        let first = policy.begin_attempt(0).unwrap();
        policy.complete_physical_failure(first, 10).unwrap();
        let second = policy.begin_attempt(30).unwrap();
        assert_eq!(
            policy.complete_physical_failure(second, 40),
            Ok(FailureDisposition::Backoff {
                failed_attempt: 2,
                delay_ticks: 40,
                deadline: 80,
            })
        );
        let third = policy.begin_attempt(80).unwrap();
        assert_eq!(third.ordinal, 3);
        assert_eq!(
            policy.complete_physical_failure(third, 90),
            Ok(FailureDisposition::Offline {
                failed_attempt: 3,
                consecutive_failures: 3,
            })
        );

        let offline = policy.snapshot();
        assert_eq!(offline.state, RecoveryState::Offline);
        assert_eq!(offline.attempts_started, 3);
        assert_eq!(offline.physical_failures, 3);
        assert_eq!(offline.attempt_failures, 3);
        assert_eq!(offline.backoffs_scheduled, 2);
        assert_eq!(offline.backoffs_completed, 2);
        assert_eq!(offline.total_backoff_ticks, 60);
        assert_eq!(offline.offline_transitions, 1);
        assert_eq!(offline.active_attempt_generation, 0);
        assert_eq!(offline.backoff_deadline, None);

        assert_eq!(policy.begin_attempt(1_000), Err(RejectReason::Offline));
        assert_eq!(
            policy.complete_physical_success(third),
            Err(RejectReason::Offline)
        );
        let still_offline = policy.snapshot();
        assert_eq!(still_offline.state, RecoveryState::Offline);
        assert_eq!(still_offline.offline_transitions, 1);
        assert_eq!(still_offline.offline_rejections, 2);
    }

    #[test]
    fn physical_success_requires_real_io_success_to_clear_the_streak() {
        let mut policy = policy(10);
        let first = policy.begin_attempt(0).unwrap();
        policy.complete_physical_failure(first, 0).unwrap();
        let second = policy.begin_attempt(10).unwrap();
        policy.complete_physical_success(second).unwrap();

        let probation = policy.snapshot();
        assert_eq!(probation.state, RecoveryState::Probation);
        assert_eq!(probation.consecutive_failures, 1);
        assert_eq!(probation.probation_entries, 1);
        assert_eq!(probation.probation_successes, 0);
        assert_eq!(probation.active_attempt_generation, second.generation);

        assert_eq!(
            policy.complete_physical_success(second),
            Err(RejectReason::InvalidState {
                state: RecoveryState::Probation,
            })
        );
        assert_eq!(
            policy.complete_probation_failure(second, 20),
            Ok(FailureDisposition::Backoff {
                failed_attempt: 2,
                delay_ticks: 20,
                deadline: 40,
            })
        );
        let third = policy.begin_attempt(40).unwrap();
        policy.complete_physical_success(third).unwrap();
        policy.complete_probation_success(third).unwrap();

        let healthy = policy.snapshot();
        assert_eq!(healthy.state, RecoveryState::Healthy);
        assert_eq!(healthy.consecutive_failures, 0);
        assert_eq!(healthy.physical_successes, 2);
        assert_eq!(healthy.physical_failures, 1);
        assert_eq!(healthy.attempt_failures, 2);
        assert_eq!(healthy.probation_entries, 2);
        assert_eq!(healthy.probation_failures, 1);
        assert_eq!(healthy.probation_successes, 1);
        assert_eq!(healthy.healthy_transitions, 1);
        assert_eq!(healthy.invalid_transition_rejections, 1);

        let next_incident = policy.begin_attempt(100).unwrap();
        assert_eq!(next_incident.generation, 4);
        assert_eq!(next_incident.ordinal, 1);
    }

    #[test]
    fn stale_ticket_cannot_complete_a_new_attempt() {
        let mut policy = policy(5);
        let first = policy.begin_attempt(0).unwrap();
        policy.complete_physical_failure(first, 0).unwrap();
        let second = policy.begin_attempt(5).unwrap();

        assert_eq!(
            policy.complete_physical_failure(first, 5),
            Err(RejectReason::StaleTicket {
                active_generation: Some(second.generation),
            })
        );
        assert_eq!(policy.active_attempt(), Some(second));
        assert_eq!(policy.state(), RecoveryState::Recovering);
        assert_eq!(policy.snapshot().stale_ticket_rejections, 1);
        policy.complete_physical_success(second).unwrap();
    }

    #[test]
    fn duplicate_completion_is_rejected_transactionally() {
        let mut policy = policy(5);
        let ticket = policy.begin_attempt(0).unwrap();
        policy.complete_physical_success(ticket).unwrap();
        let before_duplicate = policy.snapshot();
        assert_eq!(
            policy.complete_physical_success(ticket),
            Err(RejectReason::InvalidState {
                state: RecoveryState::Probation,
            })
        );
        let after_duplicate = policy.snapshot();
        assert_eq!(after_duplicate.state, before_duplicate.state);
        assert_eq!(
            after_duplicate.physical_successes,
            before_duplicate.physical_successes
        );
        assert_eq!(
            after_duplicate.probation_entries,
            before_duplicate.probation_entries
        );
        assert_eq!(after_duplicate.invalid_transition_rejections, 1);

        policy.complete_probation_success(ticket).unwrap();
        assert_eq!(
            policy.complete_probation_success(ticket),
            Err(RejectReason::StaleTicket {
                active_generation: None,
            })
        );
        assert_eq!(policy.state(), RecoveryState::Healthy);
        assert_eq!(policy.snapshot().healthy_transitions, 1);
    }

    #[test]
    fn backoff_deadline_comparison_is_wrap_safe() {
        let mut policy = policy(5);
        let first = policy.begin_attempt(u64::MAX - 3).unwrap();
        assert_eq!(
            policy.complete_physical_failure(first, u64::MAX - 3),
            Ok(FailureDisposition::Backoff {
                failed_attempt: 1,
                delay_ticks: 5,
                deadline: 1,
            })
        );
        assert_eq!(
            policy.begin_attempt(u64::MAX),
            Err(RejectReason::BackoffPending { deadline: 1 })
        );
        assert_eq!(
            policy.begin_attempt(0),
            Err(RejectReason::BackoffPending { deadline: 1 })
        );
        assert_eq!(policy.begin_attempt(1).unwrap().ordinal, 2);
        assert_eq!(policy.snapshot().early_attempt_rejections, 2);
    }

    #[test]
    fn exhausted_generation_never_aliases_an_old_ticket() {
        let mut policy = policy(1);
        policy.last_attempt_generation = u64::MAX;
        assert_eq!(
            policy.begin_attempt(0),
            Err(RejectReason::AttemptGenerationExhausted)
        );
        let snapshot = policy.snapshot();
        assert_eq!(snapshot.state, RecoveryState::Healthy);
        assert_eq!(snapshot.active_attempt_generation, 0);
        assert_eq!(snapshot.generation_exhaustions, 1);
    }

    #[test]
    fn ticket_is_copyable_and_allocation_free() {
        let mut policy = policy(1);
        let ticket = policy.begin_attempt(0).unwrap();
        let copied = AttemptTicket { ..ticket };
        assert_eq!(ticket, copied);
        assert!(!core::mem::needs_drop::<RecoveryPolicy>());
    }
}
