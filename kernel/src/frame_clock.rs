//! Deterministic software frame pacing over the 100 Hz logical kernel tick.
//!
//! This module owns no IRQ, wait-queue, or display state. It is a small,
//! auditable state machine that lets the display layer turn timer boundaries
//! into one level-triggered frame opportunity at a time.

use crate::time::advance_periodic_deadline;

pub const LOGICAL_TIMER_HZ: u64 = 100;
pub const FRAME_CLOCK_DIVIDER: u64 = 2;
pub const SOFTWARE_FRAME_RATE_HZ: u64 = LOGICAL_TIMER_HZ / FRAME_CLOCK_DIVIDER;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameClockPhase {
    Disarmed,
    Waiting,
    Ready,
    Outstanding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameGrant {
    pub epoch: u64,
    pub boundary_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameClockError {
    InvalidState,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameClockAcquireError {
    Disarmed,
    ShouldWait,
    AlreadyOutstanding,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameClockPresentError {
    Disarmed,
    NoOutstandingGrant,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameClockDegradeEvidence {
    pub discarded_ready: bool,
    pub cancelled_outstanding: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameClockSnapshot {
    pub phase: FrameClockPhase,
    pub raw_boundaries: u64,
    pub opportunities: u64,
    pub suppressed: u64,
    pub signal_edges: u64,
    pub acquired: u64,
    pub presented: u64,
    pub discarded_ready: u64,
    pub cancelled_outstanding: u64,
    pub pending_ready: u64,
    pub outstanding: u64,
    pub last_acquired_epoch: u64,
    pub last_presented_epoch: u64,
    pub last_cancelled_epoch: u64,
    pub next_boundary: u64,
    pub overflowed: bool,
    pub phase_valid: bool,
    pub accounting_valid: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FrameClock {
    phase: FrameClockPhase,
    grant: Option<FrameGrant>,
    next_boundary: u64,
    raw_boundaries: u64,
    opportunities: u64,
    suppressed: u64,
    signal_edges: u64,
    acquired: u64,
    presented: u64,
    discarded_ready: u64,
    cancelled_outstanding: u64,
    last_acquired_epoch: u64,
    last_presented_epoch: u64,
    last_cancelled_epoch: u64,
    overflowed: bool,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameClock {
    pub const fn new() -> Self {
        Self {
            phase: FrameClockPhase::Disarmed,
            grant: None,
            next_boundary: 0,
            raw_boundaries: 0,
            opportunities: 0,
            suppressed: 0,
            signal_edges: 0,
            acquired: 0,
            presented: 0,
            discarded_ready: 0,
            cancelled_outstanding: 0,
            last_acquired_epoch: 0,
            last_presented_epoch: 0,
            last_cancelled_epoch: 0,
            overflowed: false,
        }
    }

    /// Starts a fixed-phase clock whose first opportunity is two logical
    /// timer ticks after `now` (50 Hz over the kernel's 100 Hz timer).
    pub fn arm(&mut self, now: u64) -> Result<(), FrameClockError> {
        if self.overflowed {
            return Err(FrameClockError::CounterExhausted);
        }
        if self.phase != FrameClockPhase::Disarmed {
            return Err(FrameClockError::InvalidState);
        }
        self.phase = FrameClockPhase::Waiting;
        self.next_boundary = now.wrapping_add(FRAME_CLOCK_DIVIDER);
        Ok(())
    }

    /// Advances absolute frame boundaries in O(1). The returned boolean is
    /// true only when a new level-triggered FRAME_READY edge must be raised.
    pub fn on_tick(&mut self, now: u64) -> Result<bool, FrameClockError> {
        if self.overflowed {
            return Err(FrameClockError::CounterExhausted);
        }
        if self.phase == FrameClockPhase::Disarmed {
            return Ok(false);
        }

        let advance = advance_periodic_deadline(now, self.next_boundary, FRAME_CLOCK_DIVIDER)
            .unwrap_or_else(|| unreachable!("the frame-clock divider is nonzero"));
        if advance.elapsed_periods == 0 {
            return Ok(false);
        }

        let Some(raw_boundaries) = self.raw_boundaries.checked_add(advance.elapsed_periods) else {
            return self.fail_closed();
        };
        let (opportunities, signal_edges, suppressed, grant, raised) = if self.phase
            == FrameClockPhase::Waiting
        {
            let Some(opportunities) = self.opportunities.checked_add(1) else {
                return self.fail_closed();
            };
            let Some(signal_edges) = self.signal_edges.checked_add(1) else {
                return self.fail_closed();
            };
            let Some(suppressed) = self.suppressed.checked_add(advance.elapsed_periods - 1) else {
                return self.fail_closed();
            };
            let boundary_tick = advance.next_deadline.wrapping_sub(FRAME_CLOCK_DIVIDER);
            (
                opportunities,
                signal_edges,
                suppressed,
                Some(FrameGrant {
                    epoch: opportunities,
                    boundary_tick,
                }),
                true,
            )
        } else {
            let Some(suppressed) = self.suppressed.checked_add(advance.elapsed_periods) else {
                return self.fail_closed();
            };
            (
                self.opportunities,
                self.signal_edges,
                suppressed,
                self.grant,
                false,
            )
        };

        self.next_boundary = advance.next_deadline;
        self.raw_boundaries = raw_boundaries;
        self.opportunities = opportunities;
        self.signal_edges = signal_edges;
        self.suppressed = suppressed;
        self.grant = grant;
        if raised {
            self.phase = FrameClockPhase::Ready;
        }
        Ok(raised)
    }

    pub fn acquire(&mut self) -> Result<FrameGrant, FrameClockAcquireError> {
        if self.overflowed {
            return Err(FrameClockAcquireError::CounterExhausted);
        }
        match self.phase {
            FrameClockPhase::Disarmed => Err(FrameClockAcquireError::Disarmed),
            FrameClockPhase::Waiting => Err(FrameClockAcquireError::ShouldWait),
            FrameClockPhase::Outstanding => Err(FrameClockAcquireError::AlreadyOutstanding),
            FrameClockPhase::Ready => {
                let Some(acquired) = self.acquired.checked_add(1) else {
                    self.fail_closed_acquire()?;
                    unreachable!("fail_closed_acquire always returns an error")
                };
                let grant = self
                    .grant
                    .unwrap_or_else(|| panic!("ready frame clock had no grant"));
                self.acquired = acquired;
                self.last_acquired_epoch = grant.epoch;
                self.phase = FrameClockPhase::Outstanding;
                Ok(grant)
            }
        }
    }

    /// Consumes the outstanding grant only after the display commit succeeds.
    pub fn present_succeeded(&mut self) -> Result<(), FrameClockPresentError> {
        if self.overflowed {
            return Err(FrameClockPresentError::CounterExhausted);
        }
        match self.phase {
            FrameClockPhase::Disarmed => Err(FrameClockPresentError::Disarmed),
            FrameClockPhase::Waiting | FrameClockPhase::Ready => {
                Err(FrameClockPresentError::NoOutstandingGrant)
            }
            FrameClockPhase::Outstanding => {
                let Some(presented) = self.presented.checked_add(1) else {
                    self.fail_closed_present()?;
                    unreachable!("fail_closed_present always returns an error")
                };
                let grant = self
                    .grant
                    .take()
                    .unwrap_or_else(|| panic!("outstanding frame clock had no grant"));
                self.presented = presented;
                self.last_presented_epoch = grant.epoch;
                self.phase = FrameClockPhase::Waiting;
                Ok(())
            }
        }
    }

    /// Records no state change: validation failures deliberately preserve the
    /// outstanding grant so userspace can correct and retry the same frame.
    pub fn present_failed(&self) -> Result<FrameGrant, FrameClockPresentError> {
        if self.overflowed {
            return Err(FrameClockPresentError::CounterExhausted);
        }
        match self.phase {
            FrameClockPhase::Disarmed => Err(FrameClockPresentError::Disarmed),
            FrameClockPhase::Waiting | FrameClockPhase::Ready => {
                Err(FrameClockPresentError::NoOutstandingGrant)
            }
            FrameClockPhase::Outstanding => Ok(self
                .grant
                .unwrap_or_else(|| panic!("outstanding frame clock had no grant"))),
        }
    }

    /// Stops pacing. A pending opportunity is accounted as discarded, while
    /// an acquired opportunity is accounted as cancelled by owner death.
    pub fn degrade(&mut self) -> Result<FrameClockDegradeEvidence, FrameClockError> {
        if self.overflowed {
            return Err(FrameClockError::CounterExhausted);
        }
        let mut evidence = FrameClockDegradeEvidence::default();
        match self.phase {
            FrameClockPhase::Ready => {
                let Some(discarded_ready) = self.discarded_ready.checked_add(1) else {
                    return self.fail_closed_degrade();
                };
                self.discarded_ready = discarded_ready;
                evidence.discarded_ready = true;
            }
            FrameClockPhase::Outstanding => {
                let Some(cancelled_outstanding) = self.cancelled_outstanding.checked_add(1) else {
                    return self.fail_closed_degrade();
                };
                let epoch = self
                    .grant
                    .unwrap_or_else(|| panic!("outstanding frame clock had no grant"))
                    .epoch;
                self.cancelled_outstanding = cancelled_outstanding;
                self.last_cancelled_epoch = epoch;
                evidence.cancelled_outstanding = true;
            }
            FrameClockPhase::Disarmed | FrameClockPhase::Waiting => {}
        }
        self.phase = FrameClockPhase::Disarmed;
        self.grant = None;
        self.next_boundary = 0;
        Ok(evidence)
    }

    pub fn snapshot(&self) -> FrameClockSnapshot {
        let pending_ready = u64::from(self.phase == FrameClockPhase::Ready);
        let outstanding = u64::from(self.phase == FrameClockPhase::Outstanding);
        let phase_valid = match self.phase {
            FrameClockPhase::Disarmed => self.grant.is_none() && self.next_boundary == 0,
            FrameClockPhase::Waiting => self.grant.is_none(),
            FrameClockPhase::Ready | FrameClockPhase::Outstanding => self.grant.is_some(),
        };
        let accounting_valid = self
            .opportunities
            .checked_add(self.suppressed)
            .is_some_and(|raw| raw == self.raw_boundaries)
            && self
                .acquired
                .checked_add(self.discarded_ready)
                .and_then(|accounted| accounted.checked_add(pending_ready))
                .is_some_and(|accounted| accounted == self.opportunities)
            && self
                .presented
                .checked_add(self.cancelled_outstanding)
                .and_then(|accounted| accounted.checked_add(outstanding))
                .is_some_and(|accounted| accounted == self.acquired)
            && self.signal_edges == self.opportunities;
        FrameClockSnapshot {
            phase: self.phase,
            raw_boundaries: self.raw_boundaries,
            opportunities: self.opportunities,
            suppressed: self.suppressed,
            signal_edges: self.signal_edges,
            acquired: self.acquired,
            presented: self.presented,
            discarded_ready: self.discarded_ready,
            cancelled_outstanding: self.cancelled_outstanding,
            pending_ready,
            outstanding,
            last_acquired_epoch: self.last_acquired_epoch,
            last_presented_epoch: self.last_presented_epoch,
            last_cancelled_epoch: self.last_cancelled_epoch,
            next_boundary: self.next_boundary,
            overflowed: self.overflowed,
            phase_valid,
            accounting_valid,
        }
    }

    fn poison(&mut self) {
        match self.phase {
            FrameClockPhase::Ready => {
                if let Some(discarded_ready) = self.discarded_ready.checked_add(1) {
                    self.discarded_ready = discarded_ready;
                }
            }
            FrameClockPhase::Outstanding => {
                if let Some(cancelled_outstanding) = self.cancelled_outstanding.checked_add(1) {
                    self.cancelled_outstanding = cancelled_outstanding;
                    if let Some(grant) = self.grant {
                        self.last_cancelled_epoch = grant.epoch;
                    }
                }
            }
            FrameClockPhase::Disarmed | FrameClockPhase::Waiting => {}
        }
        self.phase = FrameClockPhase::Disarmed;
        self.grant = None;
        self.next_boundary = 0;
        self.overflowed = true;
    }

    fn fail_closed<T>(&mut self) -> Result<T, FrameClockError> {
        self.poison();
        Err(FrameClockError::CounterExhausted)
    }

    fn fail_closed_acquire<T>(&mut self) -> Result<T, FrameClockAcquireError> {
        self.poison();
        Err(FrameClockAcquireError::CounterExhausted)
    }

    fn fail_closed_present<T>(&mut self) -> Result<T, FrameClockPresentError> {
        self.poison();
        Err(FrameClockPresentError::CounterExhausted)
    }

    fn fail_closed_degrade<T>(&mut self) -> Result<T, FrameClockError> {
        self.poison();
        Err(FrameClockError::CounterExhausted)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FRAME_CLOCK_DIVIDER, FrameClock, FrameClockAcquireError, FrameClockError, FrameClockPhase,
        FrameClockPresentError, FrameGrant, SOFTWARE_FRAME_RATE_HZ,
    };

    #[test]
    fn new_clock_is_disarmed_and_balanced() {
        let snapshot = FrameClock::new().snapshot();
        assert_eq!(snapshot.phase, FrameClockPhase::Disarmed);
        assert_eq!(snapshot.raw_boundaries, 0);
        assert!(snapshot.phase_valid);
        assert!(snapshot.accounting_valid);
        assert_eq!(FRAME_CLOCK_DIVIDER, 2);
        assert_eq!(SOFTWARE_FRAME_RATE_HZ, 50);
    }

    #[test]
    fn arm_preserves_future_boundary_and_raises_at_exact_tick() {
        let mut clock = FrameClock::new();
        clock.arm(10).unwrap();
        assert_eq!(clock.snapshot().next_boundary, 12);
        assert!(!clock.on_tick(11).unwrap());
        assert!(clock.on_tick(12).unwrap());
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.phase, FrameClockPhase::Ready);
        assert_eq!(snapshot.next_boundary, 14);
        assert_eq!(snapshot.pending_ready, 1);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn ready_opportunity_acquires_exact_epoch_and_boundary() {
        let mut clock = FrameClock::new();
        clock.arm(20).unwrap();
        assert_eq!(clock.acquire(), Err(FrameClockAcquireError::ShouldWait));
        assert!(clock.on_tick(22).unwrap());
        assert_eq!(
            clock.acquire(),
            Ok(FrameGrant {
                epoch: 1,
                boundary_tick: 22,
            })
        );
        assert_eq!(clock.snapshot().phase, FrameClockPhase::Outstanding);
        assert_eq!(
            clock.acquire(),
            Err(FrameClockAcquireError::AlreadyOutstanding)
        );
    }

    #[test]
    fn successful_present_consumes_grant_and_reopens_waiting_phase() {
        let mut clock = outstanding_clock();
        clock.present_succeeded().unwrap();
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.phase, FrameClockPhase::Waiting);
        assert_eq!(snapshot.presented, 1);
        assert_eq!(snapshot.last_presented_epoch, 1);
        assert_eq!(snapshot.outstanding, 0);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn failed_present_preserves_the_same_outstanding_grant() {
        let mut clock = outstanding_clock();
        let before = clock.snapshot();
        assert_eq!(
            clock.present_failed(),
            Ok(FrameGrant {
                epoch: 1,
                boundary_tick: 2,
            })
        );
        assert_eq!(clock.snapshot(), before);
        clock.present_succeeded().unwrap();
    }

    #[test]
    fn late_waiting_tick_coalesces_boundaries_with_fixed_phase() {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        assert!(clock.on_tick(9).unwrap());
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.raw_boundaries, 4);
        assert_eq!(snapshot.opportunities, 1);
        assert_eq!(snapshot.suppressed, 3);
        assert_eq!(snapshot.next_boundary, 10);
        assert_eq!(
            clock.acquire().unwrap(),
            FrameGrant {
                epoch: 1,
                boundary_tick: 8,
            }
        );
    }

    #[test]
    fn ready_level_suppresses_new_edges_until_acquired() {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        assert!(clock.on_tick(2).unwrap());
        assert!(!clock.on_tick(8).unwrap());
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.raw_boundaries, 4);
        assert_eq!(snapshot.signal_edges, 1);
        assert_eq!(snapshot.suppressed, 3);
        assert_eq!(snapshot.pending_ready, 1);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn outstanding_grant_suppresses_boundaries_until_commit() {
        let mut clock = outstanding_clock();
        assert!(!clock.on_tick(10).unwrap());
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.raw_boundaries, 5);
        assert_eq!(snapshot.suppressed, 4);
        assert_eq!(snapshot.outstanding, 1);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn degrade_discards_a_ready_opportunity() {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        clock.on_tick(2).unwrap();
        let evidence = clock.degrade().unwrap();
        assert!(evidence.discarded_ready);
        assert!(!evidence.cancelled_outstanding);
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.phase, FrameClockPhase::Disarmed);
        assert_eq!(snapshot.discarded_ready, 1);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn degrade_cancels_an_outstanding_opportunity() {
        let mut clock = outstanding_clock();
        let evidence = clock.degrade().unwrap();
        assert!(!evidence.discarded_ready);
        assert!(evidence.cancelled_outstanding);
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.cancelled_outstanding, 1);
        assert_eq!(snapshot.last_cancelled_epoch, 1);
        assert_eq!(snapshot.outstanding, 0);
        assert!(snapshot.accounting_valid);
    }

    #[test]
    fn invalid_phase_operations_do_not_mutate_state() {
        let mut clock = FrameClock::new();
        assert_eq!(clock.acquire(), Err(FrameClockAcquireError::Disarmed));
        assert_eq!(
            clock.present_succeeded(),
            Err(FrameClockPresentError::Disarmed)
        );
        clock.arm(1).unwrap();
        let before = clock.snapshot();
        assert_eq!(clock.arm(2), Err(FrameClockError::InvalidState));
        assert_eq!(
            clock.present_failed(),
            Err(FrameClockPresentError::NoOutstandingGrant)
        );
        assert_eq!(clock.snapshot(), before);
    }

    #[test]
    fn fixed_phase_survives_logical_tick_wrap() {
        let mut clock = FrameClock::new();
        clock.arm(u64::MAX - 1).unwrap();
        assert_eq!(clock.snapshot().next_boundary, 0);
        assert!(clock.on_tick(0).unwrap());
        assert_eq!(clock.snapshot().next_boundary, 2);
        assert_eq!(clock.acquire().unwrap().boundary_tick, 0);
    }

    #[test]
    fn counter_overflow_disarms_and_permanently_fails_closed() {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        clock.raw_boundaries = u64::MAX;
        clock.suppressed = u64::MAX;
        assert!(clock.snapshot().accounting_valid);
        assert_eq!(clock.on_tick(2), Err(FrameClockError::CounterExhausted));
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.phase, FrameClockPhase::Disarmed);
        assert!(snapshot.overflowed);
        assert!(snapshot.phase_valid);
        assert!(snapshot.accounting_valid);
        assert_eq!(clock.arm(3), Err(FrameClockError::CounterExhausted));
        assert_eq!(
            clock.acquire(),
            Err(FrameClockAcquireError::CounterExhausted)
        );
    }

    #[test]
    fn three_outcomes_keep_both_ledger_equations_exact() {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        clock.on_tick(2).unwrap();
        clock.acquire().unwrap();
        clock.present_succeeded().unwrap();
        clock.on_tick(4).unwrap();
        clock.acquire().unwrap();
        clock.present_succeeded().unwrap();
        clock.on_tick(6).unwrap();
        clock.acquire().unwrap();
        clock.degrade().unwrap();
        let snapshot = clock.snapshot();
        assert_eq!(snapshot.opportunities, 3);
        assert_eq!(snapshot.acquired, 3);
        assert_eq!(snapshot.presented, 2);
        assert_eq!(snapshot.cancelled_outstanding, 1);
        assert_eq!(snapshot.last_acquired_epoch, 3);
        assert_eq!(snapshot.last_presented_epoch, 2);
        assert_eq!(snapshot.last_cancelled_epoch, 3);
        assert!(snapshot.phase_valid);
        assert!(snapshot.accounting_valid);
    }

    fn outstanding_clock() -> FrameClock {
        let mut clock = FrameClock::new();
        clock.arm(0).unwrap();
        clock.on_tick(2).unwrap();
        clock.acquire().unwrap();
        clock
    }
}
