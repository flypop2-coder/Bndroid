use core::alloc::Layout;
use core::mem::{align_of, size_of};
use core::ptr;

#[repr(C)]
struct FreeNode {
    size: usize,
    next: *mut FreeNode,
}

const ALLOCATION_UNIT: usize = size_of::<FreeNode>();
const _: () = assert!(ALLOCATION_UNIT.is_power_of_two());
const _: () = assert!(ALLOCATION_UNIT >= align_of::<FreeNode>());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapInitError {
    AlreadyInitialized,
    AddressOverflow,
    RegionTooSmall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeapStats {
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub allocations: usize,
    pub deallocations: usize,
}

/// A first-fit free-list allocator whose metadata lives inside free regions.
///
/// Synchronization is intentionally external: the kernel wrapper masks local
/// IRQ and serializes this object. Every returned allocation is rounded to a
/// `FreeNode` multiple, which makes any later free region capable of storing
/// its own metadata and permits exact adjacent-region coalescing.
pub struct FreeListAllocator {
    head: FreeNode,
    heap_start: usize,
    heap_end: usize,
    total_bytes: usize,
    free_bytes: usize,
    allocations: usize,
    deallocations: usize,
    initialized: bool,
}

impl FreeListAllocator {
    pub const fn new() -> Self {
        Self {
            head: FreeNode {
                size: 0,
                next: ptr::null_mut(),
            },
            heap_start: 0,
            heap_end: 0,
            total_bytes: 0,
            free_bytes: 0,
            allocations: 0,
            deallocations: 0,
            initialized: false,
        }
    }

    /// # Safety
    ///
    /// The byte range must be uniquely owned, writable, continuously mapped,
    /// and remain valid for the allocator lifetime.
    pub unsafe fn init(&mut self, start: usize, size: usize) -> Result<(), HeapInitError> {
        if self.initialized {
            return Err(HeapInitError::AlreadyInitialized);
        }
        let raw_end = start
            .checked_add(size)
            .ok_or(HeapInitError::AddressOverflow)?;
        let aligned_start =
            align_up(start, ALLOCATION_UNIT).ok_or(HeapInitError::AddressOverflow)?;
        let aligned_end = align_down(raw_end, ALLOCATION_UNIT);
        let usable_size = aligned_end.saturating_sub(aligned_start);
        if usable_size < ALLOCATION_UNIT {
            return Err(HeapInitError::RegionTooSmall);
        }

        self.heap_start = aligned_start;
        self.heap_end = aligned_end;
        self.total_bytes = usable_size;
        self.free_bytes = usable_size;
        self.allocations = 0;
        self.deallocations = 0;
        self.head.next = ptr::null_mut();
        unsafe { self.add_free_region(aligned_start, usable_size) };
        self.initialized = true;
        Ok(())
    }

    /// # Safety
    ///
    /// The allocator must be initialized and exclusively borrowed.
    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        if !self.initialized {
            return ptr::null_mut();
        }
        let Some((size, align)) = adjusted_layout(layout) else {
            return ptr::null_mut();
        };

        let mut previous = &mut self.head as *mut FreeNode;
        while !unsafe { (*previous).next }.is_null() {
            let current = unsafe { (*previous).next };
            let region_start = current as usize;
            let region_size = unsafe { (*current).size };
            let region_end = region_start + region_size;
            let Some(allocation_start) = allocation_start(region_start, region_end, size, align)
            else {
                previous = current;
                continue;
            };
            let allocation_end = allocation_start + size;
            let next = unsafe { (*current).next };
            unsafe { (*previous).next = next };

            let prefix_size = allocation_start - region_start;
            let suffix_size = region_end - allocation_end;
            if prefix_size != 0 {
                unsafe { self.add_free_region(region_start, prefix_size) };
            }
            if suffix_size != 0 {
                unsafe { self.add_free_region(allocation_end, suffix_size) };
            }

            self.free_bytes -= size;
            self.allocations += 1;
            return allocation_start as *mut u8;
        }
        ptr::null_mut()
    }

    /// # Safety
    ///
    /// `address` and `layout` must describe one live allocation returned by
    /// this allocator, and the allocation must no longer be accessed.
    pub unsafe fn deallocate(&mut self, address: *mut u8, layout: Layout) {
        let (size, _) = adjusted_layout(layout).expect("a live allocation has a valid layout");
        let start = address as usize;
        let end = start
            .checked_add(size)
            .expect("live allocation address cannot overflow");
        assert!(start >= self.heap_start && end <= self.heap_end);
        unsafe { self.add_free_region(start, size) };
        self.free_bytes += size;
        self.deallocations += 1;
    }

    pub const fn stats(&self) -> HeapStats {
        HeapStats {
            total_bytes: self.total_bytes,
            free_bytes: self.free_bytes,
            allocations: self.allocations,
            deallocations: self.deallocations,
        }
    }

    unsafe fn add_free_region(&mut self, address: usize, size: usize) {
        assert!(address.is_multiple_of(ALLOCATION_UNIT));
        assert!(size >= ALLOCATION_UNIT);
        assert!(size.is_multiple_of(ALLOCATION_UNIT));

        let mut previous = &mut self.head as *mut FreeNode;
        while !unsafe { (*previous).next }.is_null()
            && unsafe { (*previous).next as usize } < address
        {
            previous = unsafe { (*previous).next };
        }
        let next = unsafe { (*previous).next };
        if !ptr::eq(previous, &self.head) {
            assert!(unsafe { previous as usize + (*previous).size } <= address);
        }
        if !next.is_null() {
            assert!(address + size <= next as usize);
        }

        let node = address as *mut FreeNode;
        unsafe {
            node.write(FreeNode { size, next });
            (*previous).next = node;
        }

        unsafe {
            if !(*node).next.is_null() && node as usize + (*node).size == (*node).next as usize {
                let following = (*node).next;
                (*node).size += (*following).size;
                (*node).next = (*following).next;
            }
            if !ptr::eq(previous, &self.head)
                && previous as usize + (*previous).size == node as usize
            {
                (*previous).size += (*node).size;
                (*previous).next = (*node).next;
            }
        }
    }
}

impl Default for FreeListAllocator {
    fn default() -> Self {
        Self::new()
    }
}

fn adjusted_layout(layout: Layout) -> Option<(usize, usize)> {
    let node_align = align_of::<FreeNode>();
    let align = layout.align().max(node_align);
    let size = layout.size().max(ALLOCATION_UNIT);
    Some((align_up(size, ALLOCATION_UNIT)?, align))
}

fn allocation_start(
    region_start: usize,
    region_end: usize,
    size: usize,
    align: usize,
) -> Option<usize> {
    let mut start = align_up(region_start, align)?;
    if start != region_start && start - region_start < ALLOCATION_UNIT {
        start = align_up(region_start.checked_add(ALLOCATION_UNIT)?, align)?;
    }
    let end = start.checked_add(size)?;
    if end > region_end {
        return None;
    }
    let prefix = start - region_start;
    let suffix = region_end - end;
    if (prefix != 0 && prefix < ALLOCATION_UNIT) || (suffix != 0 && suffix < ALLOCATION_UNIT) {
        return None;
    }
    Some(start)
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
    use core::alloc::Layout;

    use super::{FreeListAllocator, HeapInitError};

    #[repr(align(4096))]
    struct Arena([u8; 64 * 1024]);

    #[repr(align(16))]
    struct TinyArena([u8; 24]);

    #[test]
    fn allocates_aligned_regions_and_coalesces_everything() {
        let mut arena = Arena([0; 64 * 1024]);
        let mut heap = FreeListAllocator::new();
        unsafe {
            heap.init(arena.0.as_mut_ptr() as usize, arena.0.len())
                .unwrap();
        }
        let before = heap.stats();
        let small = Layout::from_size_align(37, 8).unwrap();
        let page = Layout::from_size_align(4096, 4096).unwrap();
        let a = unsafe { heap.allocate(small) };
        let b = unsafe { heap.allocate(page) };
        assert!(!a.is_null());
        assert!(!b.is_null());
        assert!((b as usize).is_multiple_of(4096));
        unsafe {
            a.write_bytes(0xa5, 37);
            b.write_bytes(0x5a, 4096);
            heap.deallocate(a, small);
            heap.deallocate(b, page);
        }
        assert_eq!(heap.stats().free_bytes, before.free_bytes);
        assert_eq!(heap.stats().allocations, 2);
        assert_eq!(heap.stats().deallocations, 2);
    }

    #[test]
    fn reports_exhaustion_then_reuses_a_release() {
        let mut arena = Arena([0; 64 * 1024]);
        let mut heap = FreeListAllocator::new();
        unsafe {
            heap.init(arena.0.as_mut_ptr() as usize, arena.0.len())
                .unwrap();
        }
        let layout = Layout::from_size_align(8192, 16).unwrap();
        let mut allocations = std::vec::Vec::new();
        loop {
            let address = unsafe { heap.allocate(layout) };
            if address.is_null() {
                break;
            }
            allocations.push(address);
        }
        assert!(!allocations.is_empty());
        let released = allocations.pop().unwrap();
        unsafe { heap.deallocate(released, layout) };
        assert_eq!(unsafe { heap.allocate(layout) }, released);
    }

    #[test]
    fn mixed_lifetimes_leave_no_fragmentation() {
        let mut arena = Arena([0; 64 * 1024]);
        let mut heap = FreeListAllocator::new();
        unsafe {
            heap.init(arena.0.as_mut_ptr() as usize, arena.0.len())
                .unwrap();
        }
        let before = heap.stats().free_bytes;
        let layouts = [
            Layout::from_size_align(24, 8).unwrap(),
            Layout::from_size_align(777, 64).unwrap(),
            Layout::from_size_align(2048, 256).unwrap(),
            Layout::from_size_align(33, 4096).unwrap(),
        ];
        let mut addresses = [ptr::null_mut(); 4];
        for (index, layout) in layouts.into_iter().enumerate() {
            addresses[index] = unsafe { heap.allocate(layout) };
            assert!(!addresses[index].is_null());
        }
        for index in [1, 3, 0, 2] {
            unsafe { heap.deallocate(addresses[index], layouts[index]) };
        }
        assert_eq!(heap.stats().free_bytes, before);
    }

    #[test]
    fn rejects_tiny_overflowing_or_duplicate_initialization() {
        let mut arena = Arena([0; 64 * 1024]);
        let mut heap = FreeListAllocator::new();
        assert_eq!(
            unsafe { heap.init(arena.0.as_mut_ptr() as usize, 1) },
            Err(HeapInitError::RegionTooSmall)
        );
        assert_eq!(
            unsafe { heap.init(usize::MAX - 7, 16) },
            Err(HeapInitError::AddressOverflow)
        );
        unsafe {
            heap.init(arena.0.as_mut_ptr() as usize, arena.0.len())
                .unwrap();
        }
        assert_eq!(
            unsafe { heap.init(arena.0.as_mut_ptr() as usize, arena.0.len()) },
            Err(HeapInitError::AlreadyInitialized)
        );
    }

    #[test]
    fn a_24_byte_arena_can_serve_and_reuse_a_small_allocation() {
        let mut arena = TinyArena([0; 24]);
        let mut heap = FreeListAllocator::new();
        unsafe {
            heap.init(arena.0.as_mut_ptr() as usize, arena.0.len())
                .unwrap();
        }
        assert_eq!(heap.stats().total_bytes, 16);
        let layout = Layout::from_size_align(1, 1).unwrap();
        let address = unsafe { heap.allocate(layout) };
        assert!(!address.is_null());
        assert_eq!(heap.stats().free_bytes, 0);
        unsafe { heap.deallocate(address, layout) };
        assert_eq!(heap.stats().free_bytes, 16);
        assert_eq!(unsafe { heap.allocate(layout) }, address);
    }

    use core::ptr;
}
