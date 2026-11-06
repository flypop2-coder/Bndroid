//! Physical pages owned by the Linux personality, including anonymous files.
//! A shared alias consumes no second physical page. Owners must remove every
//! user translation and invalidate its ASID before dropping the last owner.
use crate::memory::{self, OwnedFrame, PAGE_SIZE, PhysFrame};
use core::sync::atomic::{AtomicU64, Ordering};

pub const MAX_PAGES: usize = if cfg!(feature = "framework-runtime") {
    131072
} else if cfg!(feature = "art-runtime") {
    65536
} else {
    8192
};
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static RECLAIMED: AtomicU64 = AtomicU64::new(0);

pub fn snapshot() -> (u64, u64, u64) {
    (
        LIVE.load(Ordering::Acquire),
        PEAK.load(Ordering::Acquire),
        RECLAIMED.load(Ordering::Acquire),
    )
}

pub struct Page(core::mem::ManuallyDrop<OwnedFrame>);
impl Page {
    pub fn allocate() -> Result<Self, i64> {
        let previous = LIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_PAGES as u64).then_some(n + 1)
            })
            .map_err(|_| 12)?;
        let frame = match memory::allocate_frame() {
            Ok(frame) => frame,
            Err(_) => {
                LIVE.fetch_sub(1, Ordering::AcqRel);
                return Err(12);
            }
        };
        PEAK.fetch_max(previous + 1, Ordering::Relaxed);
        unsafe {
            core::ptr::write_bytes(frame.start_address() as *mut u8, 0, PAGE_SIZE);
        }
        Ok(Self(core::mem::ManuallyDrop::new(frame)))
    }
    pub fn start_address(&self) -> usize {
        self.0.start_address()
    }
    pub fn frame(&self) -> PhysFrame {
        self.0.frame()
    }
    /// Transfer unique ownership into a compact page-aligned address token.
    pub fn into_address(self) -> usize {
        let page = core::mem::ManuallyDrop::new(self);
        page.start_address()
    }
    /// # Safety
    /// `address` must be the unconsumed result of `into_address`; no other
    /// owner may exist and physical-page accounting must still include it.
    pub unsafe fn from_address(address: usize) -> Self {
        Self(core::mem::ManuallyDrop::new(unsafe {
            OwnedFrame::from_allocated_address_unchecked(address)
        }))
    }
}
impl Drop for Page {
    fn drop(&mut self) {
        // Transfer the unique ownership token out exactly once.
        let frame = unsafe { core::mem::ManuallyDrop::take(&mut self.0) };
        assert_eq!(
            memory::deallocate_frame(frame),
            Ok(()),
            "Linux page release failed"
        );
        assert!(LIVE.fetch_sub(1, Ordering::AcqRel) > 0);
        RECLAIMED.fetch_add(1, Ordering::Relaxed);
    }
}
