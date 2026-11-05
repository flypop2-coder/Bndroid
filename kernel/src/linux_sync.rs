//! Linux thread flags, futex identities and deadline calculations without
//! scheduler or page-table side effects.
pub const EINVAL: i64 = 22;
pub const EAGAIN: i64 = 11;

pub const CLONE_VM: u64 = 0x100;
pub const CLONE_FS: u64 = 0x200;
pub const CLONE_FILES: u64 = 0x400;
pub const CLONE_SIGHAND: u64 = 0x800;
pub const CLONE_THREAD: u64 = 0x10000;
pub const CLONE_SYSVSEM: u64 = 0x40000;
pub const CLONE_SETTLS: u64 = 0x80000;
pub const CLONE_PARENT_SETTID: u64 = 0x100000;
pub const CLONE_CHILD_CLEARTID: u64 = 0x200000;
pub const CLONE_CHILD_SETTID: u64 = 0x1000000;

/// RTC seconds sampled with an architectural counter. The RTC sets the Unix
/// epoch; elapsed counter ticks supply subsecond time without repeated RTC
/// reads causing jumps. Initial accuracy is limited by the one-second RTC.
#[derive(Clone, Copy)]
pub struct RealtimeAnchor {
    pub seconds: u64,
    pub counter: u64,
}
impl RealtimeAnchor {
    pub fn timespec(self, now: u64, frequency: u64) -> Option<[u8; 16]> {
        let mut bytes =
            crate::linux_abi::monotonic_timespec(now.wrapping_sub(self.counter), frequency)?;
        let elapsed = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        let seconds = i64::try_from(self.seconds.checked_add(elapsed)?).ok()?;
        bytes[..8].copy_from_slice(&seconds.to_le_bytes());
        Some(bytes)
    }
    pub fn deadline(self, seconds: i64, nanos: i64, frequency: u64) -> Result<u64, i64> {
        if seconds < 0 || !(0..1_000_000_000).contains(&nanos) || frequency == 0 {
            return Err(EINVAL);
        }
        if (seconds as u64) < self.seconds {
            return Ok(self.counter); // Already expired before this anchor.
        }
        deadline(
            (seconds as u64 - self.seconds) as i64,
            nanos,
            frequency,
            self.counter,
            false,
        )
    }
}

pub fn validate_clone(flags: u64) -> Result<(), i64> {
    const REQUIRED: u64 =
        CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD | CLONE_SYSVSEM;
    const OPTIONAL: u64 =
        CLONE_SETTLS | CLONE_PARENT_SETTID | CLONE_CHILD_CLEARTID | CLONE_CHILD_SETTID;
    if flags & REQUIRED != REQUIRED || flags & !(REQUIRED | OPTIONAL) != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

/// CLOCK_MONOTONIC counter deadline. Round upward to avoid early expiry.
pub fn deadline(
    seconds: i64,
    nanos: i64,
    frequency: u64,
    now: u64,
    absolute: bool,
) -> Result<u64, i64> {
    if seconds < 0 || !(0..1_000_000_000).contains(&nanos) || frequency == 0 {
        return Err(EINVAL);
    }
    let ns = seconds as u128 * 1_000_000_000 + nanos as u128;
    let ticks = ns
        .checked_mul(frequency as u128)
        .ok_or(EINVAL)?
        .div_ceil(1_000_000_000);
    if ticks > i64::MAX as u128 {
        return Err(EINVAL);
    }
    if absolute {
        Ok(ticks as u64)
    } else {
        Ok(now.wrapping_add(ticks as u64))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Wait {
    pub process: u64,
    pub address: u64,
    pub bitset: u32,
    pub deadline: Option<u64>,
}

pub struct FutexQueue<const N: usize> {
    slots: [Option<Wait>; N],
}
impl<const N: usize> Default for FutexQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize> FutexQueue<N> {
    pub const fn new() -> Self {
        assert!(N <= 64);
        Self { slots: [None; N] }
    }
    pub fn insert(&mut self, context: usize, wait: Wait) -> Result<(), i64> {
        if wait.process == 0 || !wait.address.is_multiple_of(4) || wait.bitset == 0 {
            return Err(EINVAL);
        }
        let slot = self.slots.get_mut(context).ok_or(EINVAL)?;
        if slot.is_some() {
            return Err(EAGAIN);
        }
        *slot = Some(wait);
        Ok(())
    }
    pub fn cancel(&mut self, context: usize) -> bool {
        self.slots[context].take().is_some()
    }
    pub fn wake(&mut self, process: u64, address: u64, count: u32, bitset: u32) -> u64 {
        let mut selected = 0;
        let mut remaining = count;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if remaining == 0 {
                break;
            }
            if slot.is_some_and(|w| {
                w.process == process && w.address == address && w.bitset & bitset != 0
            }) {
                *slot = None;
                selected |= 1 << index;
                remaining -= 1;
            }
        }
        selected
    }
    /// Wake first, then transfer remaining waiters without changing their
    /// deadlines or bitsets. The caller serializes comparison and this update.
    pub fn requeue(
        &mut self,
        process: u64,
        source: u64,
        target: u64,
        wake_count: u32,
        move_count: u32,
    ) -> (u64, u32) {
        let selected = self.wake(process, source, wake_count, !0);
        let mut moved = 0;
        for wait in self.slots.iter_mut().flatten() {
            if moved == move_count {
                break;
            }
            if wait.process == process && wait.address == source {
                wait.address = target;
                moved += 1;
            }
        }
        (selected, moved)
    }
    pub fn expire(&mut self, now: u64) -> u64 {
        let mut selected = 0;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_some_and(|w| {
                w.deadline
                    .is_some_and(|end| crate::time::deadline_reached(now, end))
            }) {
                *slot = None;
                selected |= 1 << index;
            }
        }
        selected
    }
    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rtc_epoch_and_counter_share_read_and_absolute_wait_time() {
        let anchor = RealtimeAnchor {
            seconds: 1_800_000_000,
            counter: u64::MAX - 4,
        };
        let value = anchor.timespec(10, 10).unwrap();
        assert_eq!(
            i64::from_le_bytes(value[..8].try_into().unwrap()),
            1_800_000_001
        );
        assert_eq!(
            i64::from_le_bytes(value[8..].try_into().unwrap()),
            500_000_000
        );
        assert_eq!(anchor.deadline(1_800_000_001, 500_000_000, 10), Ok(10));
        assert_eq!(anchor.deadline(1_800_000_001, 500_000_001, 10), Ok(11));
        assert_eq!(anchor.deadline(1, 0, 10), Ok(anchor.counter));
        assert!(anchor.deadline(-1, 0, 10).is_err());
        assert!(anchor.deadline(1_800_000_001, 1_000_000_000, 10).is_err());
        assert!(anchor.deadline(i64::MAX, 0, u64::MAX).is_err());
        assert!(anchor.timespec(10, 0).is_none());
        assert!(
            RealtimeAnchor {
                seconds: i64::MAX as u64,
                counter: 0
            }
            .timespec(10, 10)
            .is_none()
        );
    }
    #[test]
    fn requires_one_shared_thread_group_without_process_fork() {
        let flags = 0x3d0f00;
        assert_eq!(validate_clone(flags), Ok(()));
        for bad in [
            flags | 17,
            flags & !CLONE_VM,
            flags & !CLONE_THREAD,
            flags | 0x40000000,
        ] {
            assert_eq!(validate_clone(bad), Err(EINVAL));
        }
    }
    #[test]
    fn generation_and_bitsets_separate_futex_waiters() {
        let mut q = FutexQueue::<8>::new();
        for (context, process, bitset) in [
            (1, 0x100000002, 1),
            (2, 0x200000002, 1),
            (3, 0x100000002, 2),
        ] {
            q.insert(
                context,
                Wait {
                    process,
                    address: 4096,
                    bitset,
                    deadline: None,
                },
            )
            .unwrap();
        }
        assert_eq!(q.wake(0x100000002, 4096, 0, !0), 0);
        assert_eq!(q.wake(0x100000002, 4096, 8, 1), 1 << 1);
        assert_eq!(q.wake(0x100000002, 4096, 8, 1), 0);
        assert_eq!(q.wake(0x100000002, 4096, 1, !0), 1 << 3);
        assert_eq!(q.len(), 1);
        assert!(q.cancel(2));
        assert!(q.is_empty());
    }
    #[test]
    fn deadlines_do_not_expire_early_or_reappear_after_cancel() {
        assert_eq!(deadline(0, 1, 62500000, 99, false), Ok(100));
        assert_eq!(deadline(1, 0, 100, 50, true), Ok(100));
        assert!(deadline(i64::MAX, 0, u64::MAX, 0, false).is_err());
        assert!(deadline(-1, 0, 100, 0, false).is_err());
        assert!(deadline(0, 1_000_000_000, 100, 0, false).is_err());
        let mut q = FutexQueue::<4>::new();
        q.insert(
            1,
            Wait {
                process: 2,
                address: 4096,
                bitset: !0,
                deadline: Some(100),
            },
        )
        .unwrap();
        assert_eq!(q.expire(99), 0);
        assert_eq!(q.expire(100), 2);
        assert_eq!(q.expire(101), 0);
        q.insert(
            1,
            Wait {
                process: 2,
                address: 4096,
                bitset: !0,
                deadline: Some(102),
            },
        )
        .unwrap();
        assert!(q.cancel(1));
        assert_eq!(q.expire(102), 0);
    }
    #[test]
    fn requeue_keeps_owner_deadline_bitset_and_counts_each_waiter_once() {
        let mut q = FutexQueue::<8>::new();
        for (context, process, bitset) in [(1, 2, 1), (2, 2, 2), (3, 2, 4), (4, 3, 1)] {
            q.insert(
                context,
                Wait {
                    process,
                    address: 4096,
                    bitset,
                    deadline: Some(100),
                },
            )
            .unwrap();
        }
        assert_eq!(q.requeue(2, 4096, 8192, 1, 2), (2, 2));
        assert_eq!(q.wake(2, 4096, 8, !0), 0);
        assert_eq!(q.wake(2, 8192, 8, 2), 1 << 2);
        assert_eq!(q.expire(99), 0);
        assert_eq!(q.expire(100), (1 << 3) | (1 << 4));
        assert!(q.is_empty());
        q.insert(
            0,
            Wait {
                process: 2,
                address: 4096,
                bitset: 1,
                deadline: None,
            },
        )
        .unwrap();
        assert_eq!(q.requeue(2, 4096, 4096, 0, 8), (0, 1));
        assert_eq!(q.wake(2, 4096, 8, !0), 1);
    }
}
