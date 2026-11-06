//! Timerfd expiration accounting, independent of scheduling and user pointers.
//! Nanoseconds use u128 so every nonnegative signed-64-bit timespec fits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Setting {
    pub interval: u128,
    pub value: u128,
}
impl Setting {
    pub fn decode(bytes: &[u8; 32]) -> Result<Self, i64> {
        fn time(bytes: &[u8]) -> Result<u128, i64> {
            let seconds = i64::from_le_bytes(bytes[..8].try_into().unwrap());
            let nanos = i64::from_le_bytes(bytes[8..16].try_into().unwrap());
            if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
                return Err(22);
            }
            Ok(seconds as u128 * 1_000_000_000 + nanos as u128)
        }
        Ok(Self {
            interval: time(&bytes[..16])?,
            value: time(&bytes[16..])?,
        })
    }
    pub fn encode(self) -> [u8; 32] {
        let mut bytes = [0; 32];
        for (at, time) in [(0, self.interval), (16, self.value)] {
            bytes[at..at + 8].copy_from_slice(&((time / 1_000_000_000) as u64).to_le_bytes());
            bytes[at + 8..at + 16].copy_from_slice(&((time % 1_000_000_000) as u64).to_le_bytes());
        }
        bytes
    }
}
pub struct Timer {
    clock: u32,
    next: Option<u128>,
    interval: u128,
    expirations: u64,
}
impl Timer {
    pub fn new(clock: u32) -> Result<Self, i64> {
        if !matches!(clock, 0 | 1 | 7) {
            return Err(22);
        }
        Ok(Self {
            clock,
            next: None,
            interval: 0,
            expirations: 0,
        })
    }
    fn now(&self, monotonic: u128, realtime: u128) -> u128 {
        if self.clock == 0 { realtime } else { monotonic }
    }
    pub fn advance(&mut self, monotonic: u128, realtime: u128) -> bool {
        let now = self.now(monotonic, realtime);
        let Some(next) = self.next.filter(|next| *next <= now) else {
            return false;
        };
        let elapsed = if self.interval == 0 {
            1
        } else {
            (now - next) / self.interval + 1
        };
        self.expirations = self
            .expirations
            .saturating_add(elapsed.min(u64::MAX as u128) as u64);
        self.next = (self.interval != 0).then_some(next + elapsed * self.interval);
        true
    }
    pub fn setting(&self, monotonic: u128, realtime: u128) -> Setting {
        Setting {
            interval: self.interval,
            value: self
                .next
                .map_or(0, |next| next.saturating_sub(self.now(monotonic, realtime))),
        }
    }
    pub fn set(
        &mut self,
        flags: u32,
        setting: Setting,
        monotonic: u128,
        realtime: u128,
    ) -> Result<Setting, i64> {
        // Wall-clock changes/cancellation and alarm wakeups need separate
        // platform support. They must never appear to succeed as no-ops.
        if flags & !1 != 0 {
            return Err(22);
        }
        let maximum = i64::MAX as u128 * 1_000_000_000 + 999_999_999;
        if setting.interval > maximum || setting.value > maximum {
            return Err(22);
        }
        self.advance(monotonic, realtime);
        let old = self.setting(monotonic, realtime);
        self.interval = setting.interval;
        self.expirations = 0;
        self.next = (setting.value != 0).then_some(
            setting.value
                + if flags & 1 == 0 {
                    self.now(monotonic, realtime)
                } else {
                    0
                },
        );
        Ok(old)
    }
    pub fn ready(&self) -> bool {
        self.expirations != 0
    }
    pub fn read(&mut self) -> Option<u64> {
        self.ready().then(|| core::mem::take(&mut self.expirations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_expirations_accumulate_without_drift_or_gettime_consumption() {
        let mut t = Timer::new(1).unwrap();
        t.set(
            0,
            Setting {
                interval: 7,
                value: 3,
            },
            100,
            10000,
        )
        .unwrap();
        assert!(!t.advance(102, 10000));
        assert!(t.advance(124, 10000));
        assert_eq!(
            t.setting(124, 0),
            Setting {
                interval: 7,
                value: 7
            }
        );
        assert_eq!(t.read(), Some(4));
        assert_eq!(t.read(), None);
        assert!(!t.advance(130, 0));
        assert!(t.advance(131, 0));
        assert_eq!(t.read(), Some(1));
    }
    #[test]
    fn reprogram_disarm_and_failed_settings_preserve_contract() {
        let mut t = Timer::new(0).unwrap();
        t.set(
            1,
            Setting {
                interval: 0,
                value: 40,
            },
            0,
            50,
        )
        .unwrap();
        assert!(t.advance(0, 50));
        assert_eq!(t.set(2, Setting::default(), 0, 50), Err(22));
        assert_eq!(t.read(), Some(1));
        t.set(
            0,
            Setting {
                interval: 20,
                value: 5,
            },
            10,
            100,
        )
        .unwrap();
        assert!(t.advance(10, 146));
        assert_eq!(
            t.set(
                0,
                Setting {
                    interval: 1,
                    value: 0
                },
                10,
                146
            )
            .unwrap(),
            Setting {
                interval: 20,
                value: 19
            }
        );
        assert!(!t.ready());
        assert!(!t.advance(10, 1000));
        assert_eq!(
            t.setting(10, 1000),
            Setting {
                interval: 1,
                value: 0
            }
        );
    }
    #[test]
    fn timespec_bounds_and_clock_selection_are_checked() {
        let maximum = Setting {
            interval: 0,
            value: i64::MAX as u128 * 1_000_000_000 + 999_999_999,
        };
        assert_eq!(Setting::decode(&maximum.encode()), Ok(maximum));
        let mut bytes = maximum.encode();
        bytes[24..].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        assert_eq!(Setting::decode(&bytes), Err(22));
        assert_eq!(Setting::decode(&[255; 32]), Err(22));
        for clock in [2, 3, 8, 9, u32::MAX] {
            assert!(Timer::new(clock).is_err());
        }
        let mut t = Timer::new(7).unwrap();
        t.set(1, maximum, 1, 99).unwrap();
        assert_eq!(t.setting(1, 99).value, maximum.value - 1);
        assert_eq!(
            t.set(
                0,
                Setting {
                    value: u128::MAX,
                    interval: 0
                },
                1,
                99
            ),
            Err(22)
        );
    }
}
