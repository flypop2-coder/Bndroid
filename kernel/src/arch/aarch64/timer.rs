use core::arch::asm;
#[cfg(feature = "storage-server-runtime")]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicU64, Ordering};

use bndroid_kernel::time::{advance_periodic_deadline, counter_deadline_after_periods};

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
static PERIOD: AtomicU64 = AtomicU64::new(0);
static NEXT_DEADLINE: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-runtime")]
static RESCHEDULE_PENDING: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "storage-server-runtime")]
static RESCHEDULE_REQUESTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-runtime")]
static RESCHEDULE_INTERRUPTS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub enum TimerError {
    ZeroFrequency,
    InvalidTickRate,
}

#[derive(Clone, Copy)]
pub struct SleepDeadline {
    pub request_counter: u64,
    pub deadline_counter: u64,
    pub logical_tick: u64,
}

impl TimerError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZeroFrequency => "architectural timer frequency is zero",
            Self::InvalidTickRate => "timer tick rate is invalid",
        }
    }
}

pub fn frequency_hz() -> u64 {
    let frequency: u64;
    unsafe {
        asm!(
            "mrs {frequency}, CNTFRQ_EL0",
            frequency = out(reg) frequency,
            options(nomem, nostack, preserves_flags)
        );
    }
    frequency
}

pub fn start_periodic(ticks_per_second: u32) -> Result<u64, TimerError> {
    let frequency = frequency_hz();
    if frequency == 0 {
        return Err(TimerError::ZeroFrequency);
    }
    if ticks_per_second == 0 {
        return Err(TimerError::InvalidTickRate);
    }
    let period = frequency / ticks_per_second as u64;
    if period == 0 {
        return Err(TimerError::InvalidTickRate);
    }

    let deadline = counter().wrapping_add(period);
    PERIOD.store(period, Ordering::Relaxed);
    NEXT_DEADLINE.store(deadline, Ordering::Relaxed);
    write_compare_value(deadline);
    unsafe {
        asm!(
            "msr CNTP_CTL_EL0, {control}",
            "isb",
            control = in(reg) 1_u64,
            options(nostack, preserves_flags)
        );
    }
    Ok(frequency)
}

pub fn handle_interrupt() {
    let period = PERIOD.load(Ordering::Relaxed);
    if period == 0 {
        unsafe {
            asm!(
                "msr CNTP_CTL_EL0, {control}",
                "isb",
                control = in(reg) 0_u64,
                options(nostack, preserves_flags)
            );
        }
        return;
    }
    let now = counter();
    let periodic_deadline = NEXT_DEADLINE.load(Ordering::Relaxed);
    #[cfg(feature = "storage-server-runtime")]
    if RESCHEDULE_PENDING.swap(false, Ordering::AcqRel)
        && !bndroid_kernel::time::deadline_reached(now, periodic_deadline)
    {
        // This interrupt was deliberately pulled forward to preempt the
        // foreground monitor after it made a blocked context runnable. Keep
        // the original periodic deadline and logical clock unchanged.
        write_compare_value(periodic_deadline);
        RESCHEDULE_INTERRUPTS.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let advance = advance_periodic_deadline(now, periodic_deadline, period)
        .expect("period was checked above");

    NEXT_DEADLINE.store(advance.next_deadline, Ordering::Relaxed);
    write_compare_value(advance.next_deadline);
    TICK_COUNT.fetch_add(advance.elapsed_periods, Ordering::Relaxed);
}

/// Pulls the physical timer forward once without advancing the 100 Hz
/// logical clock.
///
/// The caller must have made a blocked context runnable while local IRQ is
/// masked. The ensuing timer exception supplies the existing, audited
/// exception-return context-switch boundary; the handler restores the
/// original periodic compare value before selecting another context.
#[cfg(feature = "storage-server-runtime")]
pub fn request_reschedule() -> bool {
    if !super::irq_is_masked() {
        panic!("timer reschedule request entered with IRQ enabled");
    }
    let period = PERIOD.load(Ordering::Relaxed);
    let periodic_deadline = NEXT_DEADLINE.load(Ordering::Relaxed);
    if period == 0 || periodic_deadline == 0 || RESCHEDULE_PENDING.load(Ordering::Acquire) {
        return false;
    }
    let now = counter();
    if bndroid_kernel::time::deadline_reached(now, periodic_deadline) {
        // The periodic interrupt is already due and will provide the same
        // scheduling boundary without rewriting its compare value.
        return false;
    }
    if RESCHEDULE_PENDING.swap(true, Ordering::AcqRel) {
        return false;
    }
    write_compare_value(now.wrapping_add(1));
    RESCHEDULE_REQUESTS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "storage-server-runtime")]
pub fn reschedule_counts() -> (u64, u64) {
    (
        RESCHEDULE_REQUESTS.load(Ordering::Acquire),
        RESCHEDULE_INTERRUPTS.load(Ordering::Acquire),
    )
}

pub fn tick_count() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

/// Returns the completed 100 Hz logical timer tick used by software pacing.
#[cfg_attr(
    not(all(
        feature = "surface-frame-pacing",
        not(feature = "graphics-owner-death-runtime"),
        not(feature = "app-crash-recovery-runtime")
    )),
    allow(dead_code)
)]
pub fn logical_tick() -> u64 {
    tick_count()
}

/// Captures an absolute physical-counter deadline while IRQ is masked.
///
/// `logical_tick` includes timer periods that have elapsed in hardware but
/// whose IRQ has not yet been serviced. The actual wait queue uses the
/// physical deadline, so a catch-up IRQ cannot make a relative sleep expire
/// before the requested number of full periods has elapsed.
pub fn sleep_deadline_after_ticks(ticks: u64) -> Option<SleepDeadline> {
    let period = PERIOD.load(Ordering::Relaxed);
    let request_counter = counter();
    let deadline_counter = counter_deadline_after_periods(request_counter, period, ticks)?;
    let pending = advance_periodic_deadline(
        request_counter,
        NEXT_DEADLINE.load(Ordering::Relaxed),
        period,
    )?
    .elapsed_periods;
    let logical_tick = TICK_COUNT.load(Ordering::Relaxed).wrapping_add(pending);

    Some(SleepDeadline {
        request_counter,
        deadline_counter,
        logical_tick,
    })
}

pub fn counter_value() -> u64 {
    counter()
}

pub fn period_counter_ticks() -> u64 {
    PERIOD.load(Ordering::Relaxed)
}

fn counter() -> u64 {
    let counter: u64;
    unsafe {
        asm!(
            "isb",
            "mrs {counter}, CNTPCT_EL0",
            counter = out(reg) counter,
            options(nomem, nostack, preserves_flags)
        );
    }
    counter
}

fn write_compare_value(value: u64) {
    unsafe {
        asm!(
            "msr CNTP_CVAL_EL0, {value}",
            "isb",
            value = in(reg) value,
            options(nostack, preserves_flags)
        );
    }
}
