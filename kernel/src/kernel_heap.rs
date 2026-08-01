use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use bndroid_kernel::heap::{FreeListAllocator, HeapInitError, HeapStats};
use bndroid_kernel::memory::{self, AllocateError, PAGE_SIZE};

use crate::arch::aarch64::mmu::{self, MapError};

const STATE_UNINITIALIZED: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_READY: u8 = 2;

pub const HEAP_START: usize = mmu::DYNAMIC_MAP_START + 32 * 1024 * 1024;
// M67 combines M66's nine-child graph with larger exception stacks for the
// bounded 4,160-byte StorageSubmit wire path. Give only that product profile a
// 1 MiB heap; keep M66 and all older profiles on their evidence-locked bounds.
#[cfg(feature = "unified-product-runtime")]
pub const HEAP_PAGES: usize = 256;
#[cfg(all(
    feature = "resident-platform-shutdown-runtime",
    not(feature = "unified-product-runtime")
))]
pub const HEAP_PAGES: usize = 128;
#[cfg(not(feature = "resident-platform-shutdown-runtime"))]
pub const HEAP_PAGES: usize = 64;
pub const HEAP_BYTES: usize = HEAP_PAGES * PAGE_SIZE;
#[cfg(feature = "heap-partial-map-failure-self-test")]
pub const HEAP_ROLLBACK_TEST_PAGES: usize = 17;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelHeapInitError {
    AlreadyInitialized,
    Frame(AllocateError),
    Mapping(MapError),
    Allocator(HeapInitError),
}

impl KernelHeapInitError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyInitialized => "kernel heap was already initialized",
            Self::Frame(error) => error.as_str(),
            Self::Mapping(error) => error.as_str(),
            Self::Allocator(HeapInitError::AlreadyInitialized) => {
                "kernel heap allocator was already initialized"
            }
            Self::Allocator(HeapInitError::AddressOverflow) => "kernel heap address overflow",
            Self::Allocator(HeapInitError::RegionTooSmall) => "kernel heap region is too small",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelHeapInfo {
    pub start: usize,
    pub bytes: usize,
    pub backing_pages: usize,
}

/// A single-core allocator wrapper that excludes local IRQ re-entry.
///
/// The current kernel intentionally boots one CPU. Masking IRQ before taking
/// the guard prevents timer preemption from re-entering `GlobalAlloc`; the
/// atomic guard detects accidental recursion instead of deadlocking. SMP will
/// require a real cross-core spin lock before secondary CPUs are enabled.
struct IrqSafeHeap {
    allocator: UnsafeCell<FreeListAllocator>,
    locked: AtomicBool,
    state: AtomicU8,
}

// Every access to `allocator` is made with local IRQ masked and `locked` held.
// The kernel is single-core until this wrapper is replaced by an SMP lock.
unsafe impl Sync for IrqSafeHeap {}

impl IrqSafeHeap {
    const fn new() -> Self {
        Self {
            allocator: UnsafeCell::new(FreeListAllocator::new()),
            locked: AtomicBool::new(false),
            state: AtomicU8::new(STATE_UNINITIALIZED),
        }
    }

    fn try_lock(&self) -> Option<HeapGuard<'_>> {
        let saved_daif = crate::arch::aarch64::save_and_mask_irq();
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            crate::arch::aarch64::restore_daif(saved_daif);
            return None;
        }
        Some(HeapGuard {
            heap: self,
            saved_daif,
        })
    }

    fn stats(&self) -> Option<HeapStats> {
        if self.state.load(Ordering::Acquire) != STATE_READY {
            return None;
        }
        let guard = self
            .try_lock()
            .unwrap_or_else(|| panic!("kernel heap lock was recursively acquired"));
        Some(guard.allocator().stats())
    }
}

unsafe impl GlobalAlloc for IrqSafeHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if self.state.load(Ordering::Acquire) != STATE_READY {
            return ptr::null_mut();
        }
        let Some(mut guard) = self.try_lock() else {
            return ptr::null_mut();
        };
        if self.state.load(Ordering::Acquire) != STATE_READY {
            return ptr::null_mut();
        }
        unsafe { guard.allocator_mut().allocate(layout) }
    }

    unsafe fn dealloc(&self, address: *mut u8, layout: Layout) {
        let mut guard = self
            .try_lock()
            .unwrap_or_else(|| panic!("kernel heap lock was recursively acquired"));
        assert_eq!(self.state.load(Ordering::Acquire), STATE_READY);
        #[cfg(feature = "heap-no-reclaim-self-test")]
        if !HEAP_NO_RECLAIM_INJECTED.swap(true, Ordering::AcqRel) {
            return;
        }
        unsafe { guard.allocator_mut().deallocate(address, layout) };
    }
}

struct HeapGuard<'a> {
    heap: &'a IrqSafeHeap,
    saved_daif: u64,
}

impl HeapGuard<'_> {
    fn allocator(&self) -> &FreeListAllocator {
        unsafe { &*self.heap.allocator.get() }
    }

    fn allocator_mut(&mut self) -> &mut FreeListAllocator {
        unsafe { &mut *self.heap.allocator.get() }
    }
}

impl Drop for HeapGuard<'_> {
    fn drop(&mut self) {
        self.heap.locked.store(false, Ordering::Release);
        crate::arch::aarch64::restore_daif(self.saved_daif);
    }
}

#[global_allocator]
static KERNEL_HEAP: IrqSafeHeap = IrqSafeHeap::new();
#[cfg(feature = "heap-no-reclaim-self-test")]
static HEAP_NO_RECLAIM_INJECTED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "heap-partial-map-failure-self-test")]
static HEAP_PARTIAL_MAP_FAILURE_INJECTED: AtomicBool = AtomicBool::new(false);

/// Maps and initializes the permanent kernel heap.
///
/// # Safety
///
/// Must run once on the boot CPU after the MMU and physical-frame allocator
/// are ready, before any code attempts a heap allocation.
pub unsafe fn init() -> Result<KernelHeapInfo, KernelHeapInitError> {
    if KERNEL_HEAP
        .state
        .compare_exchange(
            STATE_UNINITIALIZED,
            STATE_INITIALIZING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return Err(KernelHeapInitError::AlreadyInitialized);
    }

    let mut mapped_pages = 0;
    while mapped_pages < HEAP_PAGES {
        let frame = match allocate_backing_frame(mapped_pages) {
            Ok(frame) => frame,
            Err(error) => {
                rollback_mappings(mapped_pages);
                KERNEL_HEAP
                    .state
                    .store(STATE_UNINITIALIZED, Ordering::Release);
                return Err(KernelHeapInitError::Frame(error));
            }
        };
        // The ownership token proves this identity-mapped physical page is not
        // live elsewhere. Clear it before exposing recycled kernel contents to
        // future safe heap abstractions.
        unsafe { ptr::write_bytes(frame.start_address() as *mut u8, 0, PAGE_SIZE) };
        let virtual_address = HEAP_START + mapped_pages * PAGE_SIZE;
        if let Err(failure) = mmu::map_kernel_rw_page(virtual_address, frame) {
            memory::deallocate_frame(failure.frame)
                .unwrap_or_else(|_| panic!("failed to release a rejected heap backing frame"));
            rollback_mappings(mapped_pages);
            KERNEL_HEAP
                .state
                .store(STATE_UNINITIALIZED, Ordering::Release);
            return Err(KernelHeapInitError::Mapping(failure.error));
        }
        mapped_pages += 1;
    }

    let init_result = {
        let mut guard = KERNEL_HEAP
            .try_lock()
            .unwrap_or_else(|| panic!("kernel heap lock was busy during initialization"));
        unsafe { guard.allocator_mut().init(HEAP_START, HEAP_BYTES) }
    };
    if let Err(error) = init_result {
        rollback_mappings(mapped_pages);
        KERNEL_HEAP
            .state
            .store(STATE_UNINITIALIZED, Ordering::Release);
        return Err(KernelHeapInitError::Allocator(error));
    }

    KERNEL_HEAP.state.store(STATE_READY, Ordering::Release);
    Ok(KernelHeapInfo {
        start: HEAP_START,
        bytes: HEAP_BYTES,
        backing_pages: HEAP_PAGES,
    })
}

pub fn stats() -> Option<HeapStats> {
    KERNEL_HEAP.stats()
}

fn allocate_backing_frame(mapped_pages: usize) -> Result<memory::OwnedFrame, AllocateError> {
    #[cfg(feature = "heap-partial-map-failure-self-test")]
    if mapped_pages == HEAP_ROLLBACK_TEST_PAGES
        && !HEAP_PARTIAL_MAP_FAILURE_INJECTED.swap(true, Ordering::AcqRel)
    {
        return Err(AllocateError::OutOfMemory);
    }
    #[cfg(not(feature = "heap-partial-map-failure-self-test"))]
    let _ = mapped_pages;

    memory::allocate_frame()
}

fn rollback_mappings(mapped_pages: usize) {
    for page in (0..mapped_pages).rev() {
        // SAFETY: rollback runs before the allocator reaches READY, so no
        // allocation, reference, or metadata can depend on these mappings.
        let frame = unsafe { mmu::unmap_kernel_page(HEAP_START + page * PAGE_SIZE) }
            .unwrap_or_else(|_| panic!("failed to roll back a kernel heap mapping"));
        memory::deallocate_frame(frame)
            .unwrap_or_else(|_| panic!("failed to release a kernel heap backing frame"));
        spin_loop();
    }
}
