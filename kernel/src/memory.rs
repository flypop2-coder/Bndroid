use core::ops::Range;
#[cfg(feature = "frame-no-reclaim-self-test")]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crate::fdt::{MAX_RESERVED_REGIONS, MemoryRegion};

pub const PAGE_SIZE: usize = 4096;
const MAX_MANAGED_BYTES: usize = 1024 * 1024 * 1024;
const MAX_MANAGED_FRAMES: usize = MAX_MANAGED_BYTES / PAGE_SIZE;
const FRAME_STATE_BITS: usize = 2;
const FRAMES_PER_STATE_WORD: usize = u64::BITS as usize / FRAME_STATE_BITS;
const STATE_WORD_COUNT: usize = MAX_MANAGED_FRAMES / FRAMES_PER_STATE_WORD;

const STATE_UNAVAILABLE: u64 = 0b00;
const STATE_FREE: u64 = 0b01;
const STATE_ALLOCATED: u64 = 0b10;
const INIT_UNINITIALIZED: u8 = 0;
const INIT_INITIALIZING: u8 = 1;
const INIT_READY: u8 = 2;

static FRAME_ALLOCATOR: FrameAllocator = FrameAllocator::new();
#[cfg(feature = "frame-no-reclaim-self-test")]
static NO_RECLAIM_INJECTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysFrame(usize);

impl PhysFrame {
    pub const fn start_address(self) -> usize {
        self.0
    }

    /// # Safety
    ///
    /// `address` must identify a page-aligned physical frame in the active
    /// translation regime. This creates an address coordinate, not ownership.
    pub const unsafe fn from_start_address_unchecked(address: usize) -> Self {
        Self(address)
    }
}

#[must_use = "release the frame or transfer it to a mapping owner"]
#[derive(Debug, Eq, PartialEq)]
pub struct OwnedFrame {
    frame: PhysFrame,
}

impl OwnedFrame {
    pub const fn start_address(&self) -> usize {
        self.frame.start_address()
    }

    pub const fn frame(&self) -> PhysFrame {
        self.frame
    }

    /// # Safety
    ///
    /// The allocator state for `address` must still be allocated, and no other
    /// ownership token may exist for it. Page-table code may call this only
    /// after the old mapping and every stale hardware translation are gone.
    pub const unsafe fn from_allocated_address_unchecked(address: usize) -> Self {
        Self {
            frame: unsafe { PhysFrame::from_start_address_unchecked(address) },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocatorError {
    EmptyRegion,
    KernelOutsideMemory,
    NoUsableFrames,
    RegionTooLarge,
    AddressOverflow,
    AlreadyInitialized,
    TooManyReservedRegions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocateError {
    NotInitialized,
    OutOfMemory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeallocateError {
    NotInitialized,
    Misaligned,
    OutOfRange,
    Unavailable,
    DoubleFree,
    CorruptState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAllocatorStats {
    pub managed_frames: usize,
    pub free_frames: usize,
    pub allocated_frames: usize,
    pub unavailable_frames: usize,
}

/// A fixed-metadata, reclaiming 4 KiB physical-frame allocator.
///
/// Two atomic state bits represent each frame: unavailable, free, or
/// allocated. The static bitmap covers at most the 1 GiB early identity map,
/// so allocation and deallocation need neither a heap nor writes into free
/// frames. Atomic state transitions make duplicate allocation and double-free
/// detection deterministic; this is not yet a NUMA-aware or contiguous-page
/// allocator.
struct FrameAllocator {
    init_state: AtomicU8,
    base: AtomicUsize,
    end: AtomicUsize,
    frame_count: AtomicUsize,
    next_hint: AtomicUsize,
    free_count: AtomicUsize,
    allocated_count: AtomicUsize,
    states: [AtomicU64; STATE_WORD_COUNT],
}

impl FrameAllocator {
    const fn new() -> Self {
        Self {
            init_state: AtomicU8::new(INIT_UNINITIALIZED),
            base: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            frame_count: AtomicUsize::new(0),
            next_hint: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            allocated_count: AtomicUsize::new(0),
            states: [const { AtomicU64::new(0) }; STATE_WORD_COUNT],
        }
    }

    fn init(
        &self,
        memory: MemoryRegion,
        kernel: Range<usize>,
        reserved: Range<usize>,
    ) -> Result<(), AllocatorError> {
        let reserved_size = reserved
            .end
            .checked_sub(reserved.start)
            .ok_or(AllocatorError::AddressOverflow)?;
        if reserved_size == 0 {
            return self.init_with_reserved(memory, kernel, &[]);
        }
        self.init_with_reserved(
            memory,
            kernel,
            &[MemoryRegion {
                start: reserved.start,
                size: reserved_size,
            }],
        )
    }

    fn init_with_reserved(
        &self,
        memory: MemoryRegion,
        kernel: Range<usize>,
        reserved: &[MemoryRegion],
    ) -> Result<(), AllocatorError> {
        if self.init_state.load(Ordering::Acquire) != INIT_UNINITIALIZED {
            return Err(AllocatorError::AlreadyInitialized);
        }
        if reserved.len() > MAX_RESERVED_REGIONS {
            return Err(AllocatorError::TooManyReservedRegions);
        }

        let memory_end = memory
            .start
            .checked_add(memory.size)
            .ok_or(AllocatorError::AddressOverflow)?;
        let base = align_up(memory.start, PAGE_SIZE).ok_or(AllocatorError::AddressOverflow)?;
        let end = align_down(memory_end, PAGE_SIZE);
        if memory.size < PAGE_SIZE || base >= end {
            return Err(AllocatorError::EmptyRegion);
        }

        let first_free = align_up(kernel.end, PAGE_SIZE).ok_or(AllocatorError::AddressOverflow)?;
        if kernel.start > kernel.end
            || kernel.start < memory.start
            || kernel.end > memory_end
            || first_free < base
            || first_free >= end
        {
            return Err(AllocatorError::KernelOutsideMemory);
        }

        let frame_count = (end - base) / PAGE_SIZE;
        if frame_count > MAX_MANAGED_FRAMES {
            return Err(AllocatorError::RegionTooLarge);
        }
        for region in reserved {
            region
                .start
                .checked_add(region.size)
                .ok_or(AllocatorError::AddressOverflow)?;
        }

        if self
            .init_state
            .compare_exchange(
                INIT_UNINITIALIZED,
                INIT_INITIALIZING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return Err(AllocatorError::AlreadyInitialized);
        }

        for word in &self.states {
            word.store(0, Ordering::Relaxed);
        }

        let mut free_count = 0;
        let mut first_free_index = frame_count;
        for index in 0..frame_count {
            let address = base + index * PAGE_SIZE;
            if address >= first_free && !frame_overlaps_reserved(address, reserved) {
                self.set_initial_state(index, STATE_FREE);
                free_count += 1;
                first_free_index = first_free_index.min(index);
            }
        }
        if free_count == 0 {
            self.init_state.store(INIT_UNINITIALIZED, Ordering::Release);
            return Err(AllocatorError::NoUsableFrames);
        }

        self.base.store(base, Ordering::Relaxed);
        self.end.store(end, Ordering::Relaxed);
        self.frame_count.store(frame_count, Ordering::Relaxed);
        self.next_hint.store(first_free_index, Ordering::Relaxed);
        self.free_count.store(free_count, Ordering::Relaxed);
        self.allocated_count.store(0, Ordering::Relaxed);
        self.init_state.store(INIT_READY, Ordering::Release);
        Ok(())
    }

    fn allocate(&self) -> Result<OwnedFrame, AllocateError> {
        if self.init_state.load(Ordering::Acquire) != INIT_READY {
            return Err(AllocateError::NotInitialized);
        }
        if self.free_count.load(Ordering::Acquire) == 0 {
            return Err(AllocateError::OutOfMemory);
        }

        let frame_count = self.frame_count.load(Ordering::Relaxed);
        let start = self.next_hint.load(Ordering::Relaxed) % frame_count;
        for offset in 0..frame_count {
            let index = (start + offset) % frame_count;
            let (word_index, shift) = state_location(index);
            let mask = 0b11_u64 << shift;
            let free = STATE_FREE << shift;
            let allocated = STATE_ALLOCATED << shift;
            let word = &self.states[word_index];
            let mut current = word.load(Ordering::Acquire);
            loop {
                if current & mask != free {
                    break;
                }
                let next = (current & !mask) | allocated;
                match word.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire)
                {
                    Ok(_) => {
                        self.next_hint
                            .store((index + 1) % frame_count, Ordering::Relaxed);
                        self.free_count.fetch_sub(1, Ordering::AcqRel);
                        self.allocated_count.fetch_add(1, Ordering::AcqRel);
                        let base = self.base.load(Ordering::Relaxed);
                        return Ok(unsafe {
                            OwnedFrame::from_allocated_address_unchecked(base + index * PAGE_SIZE)
                        });
                    }
                    Err(observed) => current = observed,
                }
            }
        }
        Err(AllocateError::OutOfMemory)
    }

    fn deallocate(&self, frame: OwnedFrame) -> Result<(), DeallocateError> {
        if self.init_state.load(Ordering::Acquire) != INIT_READY {
            return Err(DeallocateError::NotInitialized);
        }
        let address = frame.start_address();
        if !address.is_multiple_of(PAGE_SIZE) {
            return Err(DeallocateError::Misaligned);
        }
        let base = self.base.load(Ordering::Relaxed);
        let end = self.end.load(Ordering::Relaxed);
        if address < base || address >= end {
            return Err(DeallocateError::OutOfRange);
        }

        let index = (address - base) / PAGE_SIZE;
        let (word_index, shift) = state_location(index);
        let mask = 0b11_u64 << shift;
        let word = &self.states[word_index];
        let mut current = word.load(Ordering::Acquire);
        loop {
            match (current & mask) >> shift {
                STATE_UNAVAILABLE => return Err(DeallocateError::Unavailable),
                STATE_FREE => return Err(DeallocateError::DoubleFree),
                STATE_ALLOCATED => {
                    #[cfg(feature = "frame-no-reclaim-self-test")]
                    if !NO_RECLAIM_INJECTED.swap(true, Ordering::AcqRel) {
                        return Ok(());
                    }
                    let next = (current & !mask) | (STATE_FREE << shift);
                    match word.compare_exchange_weak(
                        current,
                        next,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            self.allocated_count.fetch_sub(1, Ordering::AcqRel);
                            self.free_count.fetch_add(1, Ordering::AcqRel);
                            self.next_hint.fetch_min(index, Ordering::Relaxed);
                            return Ok(());
                        }
                        Err(observed) => current = observed,
                    }
                }
                _ => return Err(DeallocateError::CorruptState),
            }
        }
    }

    fn stats(&self) -> FrameAllocatorStats {
        // Acquire the initialization publication before consuming any of the
        // relaxed field/bitmap writes performed by `init_with_reserved`.
        if self.init_state.load(Ordering::Acquire) != INIT_READY {
            return FrameAllocatorStats {
                managed_frames: 0,
                free_frames: 0,
                allocated_frames: 0,
                unavailable_frames: 0,
            };
        }
        let managed_frames = self.frame_count.load(Ordering::Acquire);
        let mut free_frames = 0;
        let mut allocated_frames = 0;
        let mut unavailable_frames = 0;

        // The counters are updated immediately around each successful bitmap
        // transition, but they cannot be read as one atomic pair. Derive this
        // diagnostic snapshot from the source-of-truth states instead so even
        // a concurrent allocation/deallocation always produces a complete,
        // non-underflowing partition of the managed frames. Each word is read
        // once; the result can span instants, but its totals remain coherent.
        let word_count = managed_frames.div_ceil(FRAMES_PER_STATE_WORD);
        for word_index in 0..word_count {
            let states = self.states[word_index].load(Ordering::Acquire);
            let frames_in_word =
                (managed_frames - word_index * FRAMES_PER_STATE_WORD).min(FRAMES_PER_STATE_WORD);
            for index_in_word in 0..frames_in_word {
                let shift = index_in_word * FRAME_STATE_BITS;
                match (states >> shift) & 0b11 {
                    STATE_FREE => free_frames += 1,
                    STATE_ALLOCATED => allocated_frames += 1,
                    // Treat the reserved 0b11 encoding conservatively as
                    // unavailable in diagnostics. Allocation/deallocation
                    // still reject it as corrupt when they encounter it.
                    _ => unavailable_frames += 1,
                }
            }
        }
        FrameAllocatorStats {
            managed_frames,
            free_frames,
            allocated_frames,
            unavailable_frames,
        }
    }

    fn set_initial_state(&self, index: usize, state: u64) {
        let (word_index, shift) = state_location(index);
        self.states[word_index].fetch_or(state << shift, Ordering::Relaxed);
    }
}

impl Default for FrameAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl AllocatorError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyRegion => "empty physical memory region",
            Self::KernelOutsideMemory => "kernel end is outside physical memory",
            Self::NoUsableFrames => "physical memory region has no usable frames",
            Self::RegionTooLarge => "physical memory region exceeds allocator metadata",
            Self::AddressOverflow => "physical address overflow",
            Self::AlreadyInitialized => "allocator already initialized",
            Self::TooManyReservedRegions => "too many reserved physical memory regions",
        }
    }
}

impl AllocateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotInitialized => "frame allocator is not initialized",
            Self::OutOfMemory => "physical frame allocator is out of memory",
        }
    }
}

impl DeallocateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotInitialized => "frame allocator is not initialized",
            Self::Misaligned => "physical frame address is not page aligned",
            Self::OutOfRange => "physical frame is outside the managed region",
            Self::Unavailable => "physical frame is permanently unavailable",
            Self::DoubleFree => "physical frame was already free",
            Self::CorruptState => "physical frame state is corrupt",
        }
    }
}

/// Initializes the process-wide physical-frame allocator.
///
/// # Safety
///
/// `memory` must be a truthful, uniquely managed Normal-RAM range physically
/// reachable at this MMU-off boot stage. Before any returned frame address is
/// dereferenced, that range must enter the kernel's Normal-memory direct map.
/// `kernel` and `reserved` must cover every byte which cannot be allocated,
/// including the live kernel image, boot data, firmware reservations, and the
/// device tree. This function must be called exactly once before allocation.
pub unsafe fn init_frame_allocator(
    memory: MemoryRegion,
    kernel: Range<usize>,
    reserved: Range<usize>,
) -> Result<(), AllocatorError> {
    FRAME_ALLOCATOR.init(memory, kernel, reserved)
}

/// Initializes the process-wide physical-frame allocator from many reserved
/// ranges.
///
/// # Safety
///
/// `memory` must be a truthful, uniquely managed Normal-RAM range physically
/// reachable at this MMU-off boot stage. Before any returned frame address is
/// dereferenced, that range must enter the kernel's Normal-memory direct map.
/// `kernel` and `reserved` together must cover every byte which cannot be
/// allocated, including the live kernel image, boot data, firmware
/// reservations, and the device tree. This function must be called exactly
/// once before allocation begins.
pub unsafe fn init_frame_allocator_with_reserved(
    memory: MemoryRegion,
    kernel: Range<usize>,
    reserved: &[MemoryRegion],
) -> Result<(), AllocatorError> {
    FRAME_ALLOCATOR.init_with_reserved(memory, kernel, reserved)
}

pub fn allocate_frame() -> Result<OwnedFrame, AllocateError> {
    FRAME_ALLOCATOR.allocate()
}

pub fn deallocate_frame(frame: OwnedFrame) -> Result<(), DeallocateError> {
    FRAME_ALLOCATOR.deallocate(frame)
}

pub fn frame_allocator_stats() -> FrameAllocatorStats {
    FRAME_ALLOCATOR.stats()
}

fn frame_overlaps_reserved(address: usize, reserved: &[MemoryRegion]) -> bool {
    let frame_end = address + PAGE_SIZE;
    reserved.iter().any(|region| {
        let region_end = region.start + region.size;
        region.size != 0 && address < region_end && frame_end > region.start
    })
}

const fn state_location(index: usize) -> (usize, usize) {
    (
        index / FRAMES_PER_STATE_WORD,
        (index % FRAMES_PER_STATE_WORD) * FRAME_STATE_BITS,
    )
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
}

const fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::{
        AllocateError, AllocatorError, DeallocateError, FrameAllocator, OwnedFrame,
        STATE_WORD_COUNT,
    };
    use crate::fdt::MemoryRegion;

    #[test]
    fn starts_after_page_aligned_kernel_end() {
        let allocator = FrameAllocator::new();
        allocator
            .init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x9000,
                },
                0x1000..0x1801,
                0x8000..0x9000,
            )
            .expect("allocator init");

        assert_eq!(
            allocator.allocate().expect("first frame").start_address(),
            0x2000
        );
    }

    #[test]
    fn skips_reserved_device_tree_pages() {
        let allocator = FrameAllocator::new();
        allocator
            .init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x9000,
                },
                0x1000..0x2000,
                0x3001..0x4fff,
            )
            .expect("allocator init");

        assert_eq!(allocator.allocate().unwrap().start_address(), 0x2000);
        assert_eq!(allocator.allocate().unwrap().start_address(), 0x5000);
    }

    #[test]
    fn skips_multiple_unordered_reserved_regions() {
        let allocator = FrameAllocator::new();
        allocator
            .init_with_reserved(
                MemoryRegion {
                    start: 0x1000,
                    size: 0xb000,
                },
                0x1000..0x2000,
                &[
                    MemoryRegion {
                        start: 0x7001,
                        size: 0x1fff,
                    },
                    MemoryRegion {
                        start: 0x3001,
                        size: 0x1ffe,
                    },
                ],
            )
            .expect("allocator init");

        let addresses = [
            allocator.allocate().unwrap().start_address(),
            allocator.allocate().unwrap().start_address(),
            allocator.allocate().unwrap().start_address(),
            allocator.allocate().unwrap().start_address(),
        ];
        assert_eq!(addresses, [0x2000, 0x5000, 0x6000, 0x9000]);
    }

    #[test]
    fn reclaims_and_reuses_a_frame() {
        let allocator = allocator_with_four_frames();
        let first = allocator.allocate().unwrap();
        let first_address = first.start_address();
        let second = allocator.allocate().unwrap();
        allocator.deallocate(first).unwrap();
        assert_eq!(allocator.allocate().unwrap().start_address(), first_address);
        assert_eq!(allocator.stats().allocated_frames, 2);
        allocator.deallocate(second).unwrap();
    }

    #[test]
    fn detects_double_free_and_unavailable_frames() {
        let allocator = allocator_with_four_frames();
        let frame = allocator.allocate().unwrap();
        let address = frame.start_address();
        allocator.deallocate(frame).unwrap();
        assert_eq!(
            allocator.deallocate(unsafe { OwnedFrame::from_allocated_address_unchecked(address) }),
            Err(DeallocateError::DoubleFree)
        );
        assert_eq!(
            allocator.deallocate(unsafe { OwnedFrame::from_allocated_address_unchecked(0x1000) }),
            Err(DeallocateError::Unavailable)
        );
    }

    #[test]
    fn reports_exhaustion_and_consistent_counts() {
        let allocator = allocator_with_four_frames();
        let frames = [
            allocator.allocate().unwrap(),
            allocator.allocate().unwrap(),
            allocator.allocate().unwrap(),
            allocator.allocate().unwrap(),
        ];
        assert_eq!(allocator.allocate(), Err(super::AllocateError::OutOfMemory));
        assert_eq!(allocator.stats().free_frames, 0);
        assert_eq!(allocator.stats().allocated_frames, 4);
        for frame in frames {
            allocator.deallocate(frame).unwrap();
        }
        assert_eq!(allocator.stats().free_frames, 4);
        assert_eq!(allocator.stats().allocated_frames, 0);
    }

    #[test]
    fn rejects_invalid_deallocations() {
        let allocator = allocator_with_four_frames();
        assert_eq!(
            allocator.deallocate(unsafe { OwnedFrame::from_allocated_address_unchecked(0x2001) }),
            Err(DeallocateError::Misaligned)
        );
        assert_eq!(
            allocator.deallocate(unsafe { OwnedFrame::from_allocated_address_unchecked(0x9000) }),
            Err(DeallocateError::OutOfRange)
        );
        let uninitialized = FrameAllocator::new();
        assert_eq!(
            uninitialized
                .deallocate(unsafe { OwnedFrame::from_allocated_address_unchecked(0x2000) }),
            Err(DeallocateError::NotInitialized)
        );
    }

    #[test]
    fn rejects_kernel_end_outside_memory() {
        let allocator = FrameAllocator::new();
        assert_eq!(
            allocator.init(
                MemoryRegion {
                    start: 0x4000,
                    size: 0x4000,
                },
                0x3000..0x3500,
                0..0,
            ),
            Err(AllocatorError::KernelOutsideMemory)
        );
    }

    #[test]
    fn validates_the_complete_kernel_range_and_initialization_state() {
        let allocator = FrameAllocator::new();
        assert_eq!(allocator.allocate(), Err(AllocateError::NotInitialized));
        assert_eq!(
            allocator.init(
                MemoryRegion {
                    start: 0x4000,
                    size: 0xc000,
                },
                0x3000..0x5000,
                0..0,
            ),
            Err(AllocatorError::KernelOutsideMemory)
        );

        allocator
            .init(
                MemoryRegion {
                    start: 0x4000,
                    size: 0xc000,
                },
                0x4000..0x5000,
                0..0,
            )
            .unwrap();
        assert_eq!(
            allocator.init(
                MemoryRegion {
                    start: 0x4000,
                    size: 0xc000,
                },
                0x4000..0x5000,
                0..0,
            ),
            Err(AllocatorError::AlreadyInitialized)
        );
    }

    #[test]
    fn rejects_an_arena_with_no_usable_frames() {
        let allocator = FrameAllocator::new();
        assert_eq!(
            allocator.init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x3000,
                },
                0x1000..0x2000,
                0x2000..0x4000,
            ),
            Err(AllocatorError::NoUsableFrames)
        );
        assert_eq!(allocator.allocate(), Err(AllocateError::NotInitialized));
    }

    #[test]
    fn accounts_for_page_alignment_and_unavailable_frames() {
        let allocator = FrameAllocator::new();
        allocator
            .init(
                MemoryRegion {
                    start: 0x1003,
                    size: 0x9ffd,
                },
                0x1003..0x2801,
                0x5fff..0x6001,
            )
            .unwrap();
        let stats = allocator.stats();
        assert_eq!(stats.managed_frames, 9);
        assert_eq!(stats.unavailable_frames, 3);
        assert_eq!(stats.free_frames, 6);
        assert_eq!(stats.allocated_frames, 0);
    }

    #[test]
    fn rejects_a_region_larger_than_static_metadata() {
        let allocator = FrameAllocator::new();
        assert_eq!(
            allocator.init(
                MemoryRegion {
                    start: 0x4000_0000,
                    size: 1024 * 1024 * 1024 + 4096,
                },
                0x4000_0000..0x4000_1000,
                0..0,
            ),
            Err(AllocatorError::RegionTooLarge)
        );
    }

    #[test]
    fn deterministic_allocate_release_model_preserves_uniqueness() {
        let allocator = FrameAllocator::new();
        allocator
            .init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x41000,
                },
                0x1000..0x2000,
                0..0,
            )
            .unwrap();
        let mut owned = std::vec::Vec::new();
        let mut seed = 0x4d59_5df4_d0f3_3173_u64;
        for _ in 0..10_000 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            if seed & 1 == 0 || owned.is_empty() {
                if let Ok(frame) = allocator.allocate() {
                    assert!(!owned.iter().any(|other: &OwnedFrame| {
                        other.start_address() == frame.start_address()
                    }));
                    owned.push(frame);
                }
            } else {
                let index = (seed as usize) % owned.len();
                allocator.deallocate(owned.swap_remove(index)).unwrap();
            }
            let stats = allocator.stats();
            assert_eq!(stats.allocated_frames, owned.len());
            assert_eq!(
                stats.managed_frames,
                stats.unavailable_frames + stats.free_frames + stats.allocated_frames
            );
        }
        for frame in owned {
            allocator.deallocate(frame).unwrap();
        }
        assert_eq!(allocator.stats().allocated_frames, 0);
    }

    #[test]
    fn concurrent_stats_always_form_a_complete_partition() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Barrier};

        let allocator = Arc::new(FrameAllocator::new());
        allocator
            .init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x21000,
                },
                0x1000..0x2000,
                0..0,
            )
            .unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let start = Arc::new(Barrier::new(5));
        let mut workers = std::vec::Vec::new();
        for _ in 0..4 {
            let allocator = Arc::clone(&allocator);
            let running = Arc::clone(&running);
            let start = Arc::clone(&start);
            workers.push(std::thread::spawn(move || {
                start.wait();
                while running.load(Ordering::Acquire) {
                    if let Ok(frame) = allocator.allocate() {
                        std::thread::yield_now();
                        allocator.deallocate(frame).unwrap();
                    }
                }
            }));
        }

        start.wait();
        for _ in 0..10_000 {
            let stats = allocator.stats();
            assert_eq!(
                stats.managed_frames,
                stats.unavailable_frames + stats.free_frames + stats.allocated_frames
            );
        }
        running.store(false, Ordering::Release);
        for worker in workers {
            worker.join().unwrap();
        }
        let stats = allocator.stats();
        assert_eq!(stats.allocated_frames, 0);
        assert_eq!(
            stats.managed_frames,
            stats.unavailable_frames + stats.free_frames
        );
    }

    #[test]
    fn bitmap_capacity_matches_one_gibibyte() {
        assert_eq!(STATE_WORD_COUNT * 32 * 4096, 1024 * 1024 * 1024);
    }

    fn allocator_with_four_frames() -> FrameAllocator {
        let allocator = FrameAllocator::new();
        allocator
            .init(
                MemoryRegion {
                    start: 0x1000,
                    size: 0x5000,
                },
                0x1000..0x2000,
                0..0,
            )
            .unwrap();
        allocator
    }
}
