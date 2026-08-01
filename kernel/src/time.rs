#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicAdvance {
    pub next_deadline: u64,
    pub elapsed_periods: u64,
}

pub const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelativeDeadlineError {
    ZeroDuration,
    ZeroFrequency,
    TooFar,
}

/// Converts a positive relative nanosecond duration into an architectural
/// counter deadline. The conversion rounds up, so a finite object wait can
/// never expire before the requested duration. Deadlines farther than half of
/// the serial-number space are rejected because their ordering is ambiguous.
///
/// The `u64::MAX` infinite-wait sentinel is intentionally handled by the
/// syscall layer and must not be passed here.
pub fn counter_deadline_after_nanoseconds(
    now: u64,
    frequency_hz: u64,
    duration_ns: u64,
) -> Result<u64, RelativeDeadlineError> {
    if duration_ns == 0 {
        return Err(RelativeDeadlineError::ZeroDuration);
    }
    if frequency_hz == 0 {
        return Err(RelativeDeadlineError::ZeroFrequency);
    }

    let numerator = u128::from(duration_ns) * u128::from(frequency_hz);
    let delta = numerator.div_ceil(u128::from(NANOSECONDS_PER_SECOND));
    if delta == 0 || delta > i64::MAX as u128 {
        return Err(RelativeDeadlineError::TooFar);
    }
    Ok(now.wrapping_add(delta as u64))
}

/// Advances a periodic absolute deadline beyond `now` in O(1) time and
/// reports how many logical periods elapsed.
///
/// Counter wrap is handled as long as the deadline distance is less than half
/// the 64-bit counter range, which is vastly longer than a device lifetime at
/// architectural timer frequencies.
pub fn advance_periodic_deadline(now: u64, current: u64, period: u64) -> Option<PeriodicAdvance> {
    if period == 0 {
        return None;
    }

    let elapsed = now.wrapping_sub(current);
    if (elapsed as i64) < 0 {
        return Some(PeriodicAdvance {
            next_deadline: current,
            elapsed_periods: 0,
        });
    }
    let periods = elapsed / period + 1;
    Some(PeriodicAdvance {
        next_deadline: current.wrapping_add(periods.wrapping_mul(period)),
        elapsed_periods: periods,
    })
}

pub fn next_periodic_deadline(now: u64, current: u64, period: u64) -> Option<u64> {
    advance_periodic_deadline(now, current, period).map(|advance| advance.next_deadline)
}

/// Returns true when an absolute tick deadline has been reached or passed.
///
/// The deadline must be within half the `u64` range of `now`, matching the
/// standard serial-number arithmetic rule used by kernel timer queues.
pub const fn deadline_reached(now: u64, deadline: u64) -> bool {
    now.wrapping_sub(deadline) as i64 >= 0
}

/// Returns true when two non-empty half-range counter intervals overlap.
///
/// Each interval is interpreted as `[start, end)` in serial-number space. A
/// distance greater than half the `u64` range is ambiguous and therefore
/// rejected. This remains correct when either interval crosses counter wrap.
pub const fn counter_intervals_overlap(start_a: u64, end_a: u64, start_b: u64, end_b: u64) -> bool {
    let span_a = end_a.wrapping_sub(start_a);
    let span_b = end_b.wrapping_sub(start_b);
    if span_a == 0 || span_a > i64::MAX as u64 || span_b == 0 || span_b > i64::MAX as u64 {
        return false;
    }

    start_a.wrapping_sub(start_b) < span_b || start_b.wrapping_sub(start_a) < span_a
}

/// Builds a serial-number-safe absolute counter deadline for a relative
/// number of timer periods.
pub fn counter_deadline_after_periods(now: u64, period: u64, periods: u64) -> Option<u64> {
    if period == 0 || periods == 0 {
        return None;
    }
    let distance = period.checked_mul(periods)?;
    if distance > i64::MAX as u64 {
        return None;
    }
    Some(now.wrapping_add(distance))
}

#[cfg(test)]
mod tests {
    use super::{
        RelativeDeadlineError, advance_periodic_deadline, counter_deadline_after_nanoseconds,
        counter_deadline_after_periods, counter_intervals_overlap, deadline_reached,
        next_periodic_deadline,
    };

    #[test]
    fn advances_one_period_at_exact_deadline() {
        assert_eq!(next_periodic_deadline(100, 100, 10), Some(110));
    }

    #[test]
    fn catches_up_missed_periods_in_one_step() {
        assert_eq!(next_periodic_deadline(145, 100, 10), Some(150));
        let advance = advance_periodic_deadline(145, 100, 10).unwrap();
        assert_eq!(advance.next_deadline, 150);
        assert_eq!(advance.elapsed_periods, 5);
    }

    #[test]
    fn preserves_a_future_deadline() {
        assert_eq!(next_periodic_deadline(90, 100, 10), Some(100));
        assert_eq!(
            advance_periodic_deadline(90, 100, 10)
                .unwrap()
                .elapsed_periods,
            0
        );
    }

    #[test]
    fn handles_counter_wrap() {
        assert_eq!(next_periodic_deadline(5, u64::MAX - 4, 10), Some(15));
    }

    #[test]
    fn rejects_zero_period() {
        assert_eq!(next_periodic_deadline(100, 100, 0), None);
    }

    #[test]
    fn compares_absolute_deadlines_across_wrap() {
        assert!(!deadline_reached(u64::MAX, 1));
        assert!(deadline_reached(1, 1));
        assert!(deadline_reached(2, 1));

        assert!(counter_intervals_overlap(10, 20, 15, 25));
        assert!(!counter_intervals_overlap(10, 20, 20, 25));
        assert!(counter_intervals_overlap(u64::MAX - 4, 5, 2, 8));
        assert!(!counter_intervals_overlap(u64::MAX - 4, 2, 2, 8));
        assert!(!counter_intervals_overlap(10, 10, 10, 20));
        assert!(!counter_intervals_overlap(0, 1 << 63, 0, 1));
    }

    #[test]
    fn builds_counter_deadlines_without_early_rounding() {
        assert_eq!(counter_deadline_after_periods(95, 10, 2), Some(115));
        assert_eq!(
            counter_deadline_after_periods(u64::MAX - 5, 10, 2),
            Some(14)
        );
    }

    #[test]
    fn rejects_invalid_or_ambiguous_counter_deadlines() {
        assert_eq!(counter_deadline_after_periods(1, 0, 2), None);
        assert_eq!(counter_deadline_after_periods(1, 2, 0), None);
        assert_eq!(counter_deadline_after_periods(1, u64::MAX, 2), None);
        assert_eq!(counter_deadline_after_periods(1, 1 << 62, 2), None);
    }

    #[test]
    fn nanosecond_deadlines_round_up_and_wrap_safely() {
        assert_eq!(counter_deadline_after_nanoseconds(10, 1, 1), Ok(11));
        assert_eq!(
            counter_deadline_after_nanoseconds(10, 10, 1_500_000_000),
            Ok(25)
        );
        assert_eq!(
            counter_deadline_after_nanoseconds(u64::MAX - 2, 10, 300_000_000),
            Ok(0)
        );
    }

    #[test]
    fn nanosecond_deadlines_reject_invalid_or_ambiguous_inputs() {
        assert_eq!(
            counter_deadline_after_nanoseconds(1, 1, 0),
            Err(RelativeDeadlineError::ZeroDuration)
        );
        assert_eq!(
            counter_deadline_after_nanoseconds(1, 0, 1),
            Err(RelativeDeadlineError::ZeroFrequency)
        );
        assert_eq!(
            counter_deadline_after_nanoseconds(1, u64::MAX, u64::MAX),
            Err(RelativeDeadlineError::TooFar)
        );
    }
}
