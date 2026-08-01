use alloc::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use core::cell::UnsafeCell;
use core::fmt;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering, fence};

use bndr_abi::ObjectSignals;

/// A level-triggered kernel event.
///
/// Clones refer to the same boolean state. `signal` and `clear` report whether
/// they changed that state, allowing callers to avoid scanning waiters after
/// idempotent operations.
pub struct Event {
    inner: NonNull<EventInner>,
}

struct EventInner {
    strong: AtomicUsize,
    signaled: SpinMutex<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventAllocError;

// The pointee is immutable except through its atomic count and SpinMutex. Each
// Event owns one strong count, so the allocation outlives every access.
unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
    /// Creates an event in the unsignaled state.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|_| handle_alloc_error(Layout::new::<EventInner>()))
    }

    /// Creates an unsignaled event without invoking the global OOM handler.
    pub fn try_new() -> Result<Self, EventAllocError> {
        Self::try_new_with(|layout| unsafe { alloc(layout) })
    }

    fn try_new_with(allocate: impl FnOnce(Layout) -> *mut u8) -> Result<Self, EventAllocError> {
        let inner = NonNull::new(allocate(Layout::new::<EventInner>()).cast::<EventInner>())
            .ok_or(EventAllocError)?;
        unsafe {
            inner.as_ptr().write(EventInner {
                strong: AtomicUsize::new(1),
                signaled: SpinMutex::new(false),
            });
        }
        Ok(Self { inner })
    }

    /// Raises the event, returning true only for the unsignaled-to-signaled
    /// edge.
    pub fn signal(&self) -> bool {
        let mut signaled = self.inner().signaled.lock();
        if *signaled {
            false
        } else {
            *signaled = true;
            true
        }
    }

    /// Lowers the event, returning true only for the signaled-to-unsignaled
    /// edge.
    pub fn clear(&self) -> bool {
        let mut signaled = self.inner().signaled.lock();
        if *signaled {
            *signaled = false;
            true
        } else {
            false
        }
    }

    /// Returns the complete level-triggered signal state.
    pub fn signals(&self) -> ObjectSignals {
        if *self.inner().signaled.lock() {
            ObjectSignals::SIGNALED
        } else {
            ObjectSignals::NONE
        }
    }

    pub fn same_event(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    #[cfg(test)]
    pub(crate) fn reference_count(&self) -> usize {
        self.inner().strong.load(Ordering::Relaxed)
    }

    fn inner(&self) -> &EventInner {
        unsafe { self.inner.as_ref() }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Event")
            .field("inner", &self.inner)
            .field("signals", &self.signals())
            .finish()
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        self.inner()
            .strong
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |strong| {
                strong
                    .checked_add(1)
                    .filter(|next| *next <= isize::MAX as usize)
            })
            .unwrap_or_else(|_| panic!("event strong-reference overflow"));
        Self { inner: self.inner }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
                dealloc(
                    self.inner.as_ptr().cast::<u8>(),
                    Layout::new::<EventInner>(),
                );
            }
        }
    }
}

struct SpinMutex<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinMutex<T> {}
unsafe impl<T: Send> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> SpinMutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        SpinMutexGuard { mutex: self }
    }
}

struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventAllocError};
    use bndr_abi::ObjectSignals;

    #[test]
    fn fallible_creation_reports_allocator_exhaustion() {
        assert!(matches!(
            Event::try_new_with(|_| core::ptr::null_mut()),
            Err(EventAllocError)
        ));
    }

    #[test]
    fn new_event_is_unsignaled_and_edges_are_idempotent() {
        let event = Event::new();
        assert_eq!(event.signals(), ObjectSignals::NONE);
        assert!(!event.clear());

        assert!(event.signal());
        assert_eq!(event.signals(), ObjectSignals::SIGNALED);
        assert!(!event.signal());

        assert!(event.clear());
        assert_eq!(event.signals(), ObjectSignals::NONE);
        assert!(!event.clear());
    }

    #[test]
    fn clones_share_state_but_distinct_events_do_not() {
        let event = Event::new();
        let clone = event.clone();
        let distinct = Event::new();

        assert!(event.same_event(&clone));
        assert!(!event.same_event(&distinct));
        assert!(clone.signal());
        assert_eq!(event.signals(), ObjectSignals::SIGNALED);
        assert_eq!(distinct.signals(), ObjectSignals::NONE);
        assert!(event.clear());
        assert_eq!(clone.signals(), ObjectSignals::NONE);
    }
}
