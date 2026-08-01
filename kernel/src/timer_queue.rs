use crate::time::deadline_reached;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerWait {
    pub context: usize,
    pub deadline: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerQueueError {
    Full,
    DuplicateContext,
    DeadlineNotInFuture,
    DeadlineTooFar,
}

/// A fixed-capacity, allocation-free queue ordered by absolute timer deadline.
///
/// All deadlines must be less than half the `u64` tick range into the future.
/// This gives comparisons well-defined wraparound semantics without requiring
/// a wider counter. Equal deadlines retain insertion order.
pub struct TimerWaitQueue<const CAPACITY: usize> {
    entries: [Option<TimerWait>; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> TimerWaitQueue<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, now: u64, wait: TimerWait) -> Result<(), TimerQueueError> {
        if self.len == CAPACITY {
            return Err(TimerQueueError::Full);
        }
        if self.entries[..self.len]
            .iter()
            .flatten()
            .any(|entry| entry.context == wait.context)
        {
            return Err(TimerQueueError::DuplicateContext);
        }
        if deadline_reached(now, wait.deadline) {
            return Err(TimerQueueError::DeadlineNotInFuture);
        }
        let distance = wait.deadline.wrapping_sub(now);
        if distance > i64::MAX as u64 {
            return Err(TimerQueueError::DeadlineTooFar);
        }

        let insert_at = (0..self.len)
            .find(|&index| {
                let existing = self.entries[index].expect("occupied queue prefix");
                // An expired entry must remain at the head until it is
                // drained. Treating its wrapped distance as a future value
                // would allow a newly inserted waiter to hide it.
                let existing_distance = if deadline_reached(now, existing.deadline) {
                    0
                } else {
                    existing.deadline.wrapping_sub(now)
                };
                distance < existing_distance
            })
            .unwrap_or(self.len);
        for index in (insert_at..self.len).rev() {
            self.entries[index + 1] = self.entries[index];
        }
        self.entries[insert_at] = Some(wait);
        self.len += 1;
        Ok(())
    }

    pub fn pop_expired(&mut self, now: u64) -> Option<TimerWait> {
        let first = self.entries.first().copied().flatten()?;
        if !deadline_reached(now, first.deadline) {
            return None;
        }
        for index in 1..self.len {
            self.entries[index - 1] = self.entries[index];
        }
        self.len -= 1;
        self.entries[self.len] = None;
        Some(first)
    }

    pub fn first(&self) -> Option<TimerWait> {
        self.entries.first().copied().flatten()
    }

    pub fn contains(&self, context: usize, deadline: u64) -> bool {
        self.entries[..self.len]
            .iter()
            .flatten()
            .any(|entry| entry.context == context && entry.deadline == deadline)
    }
}

impl<const CAPACITY: usize> Default for TimerWaitQueue<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{TimerQueueError, TimerWait, TimerWaitQueue};

    #[test]
    fn orders_deadlines_and_preserves_equal_deadline_fifo() {
        let mut queue = TimerWaitQueue::<4>::new();
        queue
            .insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 30,
                },
            )
            .unwrap();
        queue
            .insert(
                10,
                TimerWait {
                    context: 2,
                    deadline: 20,
                },
            )
            .unwrap();
        queue
            .insert(
                10,
                TimerWait {
                    context: 3,
                    deadline: 20,
                },
            )
            .unwrap();

        assert_eq!(queue.pop_expired(19), None);
        assert_eq!(
            queue.pop_expired(20),
            Some(TimerWait {
                context: 2,
                deadline: 20
            })
        );
        assert_eq!(
            queue.pop_expired(20),
            Some(TimerWait {
                context: 3,
                deadline: 20
            })
        );
        assert_eq!(
            queue.pop_expired(30),
            Some(TimerWait {
                context: 1,
                deadline: 30
            })
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn handles_deadlines_across_counter_wrap() {
        let mut queue = TimerWaitQueue::<2>::new();
        queue
            .insert(
                u64::MAX - 2,
                TimerWait {
                    context: 0,
                    deadline: 1,
                },
            )
            .unwrap();
        assert_eq!(queue.pop_expired(u64::MAX), None);
        assert_eq!(
            queue.pop_expired(1),
            Some(TimerWait {
                context: 0,
                deadline: 1
            })
        );
    }

    #[test]
    fn a_new_waiter_cannot_hide_an_expired_head() {
        let mut queue = TimerWaitQueue::<2>::new();
        queue
            .insert(
                10,
                TimerWait {
                    context: 0,
                    deadline: 15,
                },
            )
            .unwrap();
        queue
            .insert(
                20,
                TimerWait {
                    context: 1,
                    deadline: 25,
                },
            )
            .unwrap();

        assert_eq!(
            queue.pop_expired(20),
            Some(TimerWait {
                context: 0,
                deadline: 15,
            })
        );
        assert_eq!(queue.pop_expired(20), None);
        assert_eq!(
            queue.pop_expired(25),
            Some(TimerWait {
                context: 1,
                deadline: 25,
            })
        );
    }

    #[test]
    fn rejects_capacity_duplicates_and_invalid_deadlines() {
        let mut queue = TimerWaitQueue::<1>::new();
        assert_eq!(
            queue.insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 10,
                }
            ),
            Err(TimerQueueError::DeadlineNotInFuture)
        );
        queue
            .insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 11,
                },
            )
            .unwrap();
        assert_eq!(
            queue.insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 12,
                }
            ),
            Err(TimerQueueError::Full)
        );

        let mut duplicate_queue = TimerWaitQueue::<2>::new();
        duplicate_queue
            .insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 11,
                },
            )
            .unwrap();
        assert_eq!(
            duplicate_queue.insert(
                10,
                TimerWait {
                    context: 1,
                    deadline: 12,
                }
            ),
            Err(TimerQueueError::DuplicateContext)
        );
        assert_eq!(
            duplicate_queue.insert(
                10,
                TimerWait {
                    context: 2,
                    deadline: 10_u64.wrapping_add(1_u64 << 63),
                }
            ),
            Err(TimerQueueError::DeadlineTooFar)
        );
    }
}
