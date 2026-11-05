//! Sparse private Linux anonymous/file mappings. No user-controlled address may
//! select a shared kernel table or the native image/stack window.
use super::*;
use bndroid_kernel::linux_vm::{
    self, Change, EACCES, EINVAL, ENOMEM, Layout as VmLayout, Protection,
};

// ART's first low-address allocation is 64 MiB. Leave room for its DSO pages
// while retaining a hard 256 MiB per-process bound in this bring-up profile.
const MAX_PAGES: usize = if cfg!(feature = "art-runtime") {
    65536
} else {
    8192
};
const MAX_TABLES: usize = 256;
static LIVE_PAGES: AtomicU64 = AtomicU64::new(0);
static LIVE_TABLES: AtomicU64 = AtomicU64::new(0);
static PEAK_PAGES: AtomicU64 = AtomicU64::new(0);
static RECLAIMED_PAGES: AtomicU64 = AtomicU64::new(0);

pub struct LinuxMemorySnapshot {
    pub pages: u64,
    pub tables: u64,
    pub peak_pages: u64,
    pub reclaimed_pages: u64,
}
pub fn linux_memory_snapshot() -> LinuxMemorySnapshot {
    LinuxMemorySnapshot {
        pages: LIVE_PAGES.load(Ordering::Acquire),
        tables: LIVE_TABLES.load(Ordering::Acquire),
        peak_pages: PEAK_PAGES.load(Ordering::Acquire),
        reclaimed_pages: RECLAIMED_PAGES.load(Ordering::Acquire),
    }
}

struct Page {
    address: usize,
    frame: OwnedFrame,
    protection: Protection,
}
struct Table {
    key: usize,
    level: u8,
    frame: OwnedFrame,
}

#[derive(Default)]
pub(super) struct LinuxMappings {
    layout: VmLayout,
    pages: Vec<Page>,
    tables: Vec<Table>,
}

#[derive(Default)]
struct Prepared {
    pages: Vec<Page>,
    tables: Vec<Table>,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        for page in self.pages.drain(..) {
            release_destroyed_address_space_frame(page.frame, "unpublished Linux page");
        }
        for table in self.tables.drain(..) {
            release_destroyed_address_space_frame(table.frame, "unpublished Linux table");
        }
    }
}

impl LinuxMappings {
    pub(super) fn map(&mut self, l1: &OwnedFrame, asid: u8, args: [u64; 6]) -> Result<u64, i64> {
        let change = self
            .layout
            .map(args[0], args[1], args[2], args[3], args[5])?;
        let start = change.start;
        self.apply(l1, asid, change, None)?;
        Ok(start)
    }

    pub(super) fn protect(
        &mut self,
        l1: &OwnedFrame,
        asid: u8,
        address: u64,
        length: u64,
        protection: u64,
    ) -> Result<u64, i64> {
        if length == 0 {
            return if address.is_multiple_of(linux_vm::PAGE) {
                Ok(0)
            } else {
                Err(EINVAL)
            };
        }
        let change = self.layout.protect(address, length, protection)?;
        self.apply(l1, asid, change, None)?;
        Ok(0)
    }

    pub(super) fn unmap(
        &mut self,
        l1: &OwnedFrame,
        asid: u8,
        address: u64,
        length: u64,
    ) -> Result<u64, i64> {
        let change = self.layout.unmap(address, length)?;
        self.apply(l1, asid, change, None)?;
        Ok(0)
    }

    fn apply(
        &mut self,
        l1: &OwnedFrame,
        asid: u8,
        change: Change,
        source: Option<&[u8]>,
    ) -> Result<(), i64> {
        assert!(crate::arch::aarch64::irq_is_masked());
        let start = change.start as usize;
        let end = change.end as usize;
        let remove = |page: &Page| change.replace && start <= page.address && page.address < end;
        let remaining = self.pages.iter().filter(|page| !remove(page)).count();
        let mut pending = Prepared::default();
        if change.protection.is_some_and(|p| p != Protection::None) || source.is_some() {
            let count = (end - start) / PAGE_SIZE;
            if count > MAX_PAGES {
                return Err(ENOMEM);
            }
            pending.pages.try_reserve_exact(count).map_err(|_| ENOMEM)?;
            pending
                .tables
                .try_reserve_exact(MAX_TABLES)
                .map_err(|_| ENOMEM)?;
            for address in (start..end).step_by(PAGE_SIZE) {
                if change.layout.regions.iter().any(|region| {
                    region.file_backed
                        && region.start <= address as u64
                        && (address as u64) < region.end
                        && address as u64 >= region.file_page_end
                }) {
                    continue;
                }
                if !change.replace && self.pages.iter().any(|page| page.address == address) {
                    continue;
                }
                if remaining + pending.pages.len() >= MAX_PAGES {
                    return Err(ENOMEM);
                }
                let frame = memory::allocate_frame().map_err(|_| ENOMEM)?;
                unsafe {
                    ptr::write_bytes(frame.start_address() as *mut u8, 0, PAGE_SIZE);
                }
                if let Some(bytes) = source {
                    let offset = address - start;
                    let count = bytes.len().saturating_sub(offset).min(PAGE_SIZE);
                    if count != 0 {
                        unsafe {
                            ptr::copy_nonoverlapping(
                                bytes.as_ptr().add(offset),
                                frame.start_address() as *mut u8,
                                count,
                            );
                        }
                    }
                }
                pending.pages.push(Page {
                    address,
                    frame,
                    protection: change.protection.unwrap(),
                });
                for (level, key) in [(2, address >> 30), (3, address >> 21)] {
                    if self
                        .tables
                        .iter()
                        .chain(pending.tables.iter())
                        .any(|table| table.level == level && table.key == key)
                    {
                        continue;
                    }
                    if self.tables.len() + pending.tables.len() >= MAX_TABLES {
                        return Err(ENOMEM);
                    }
                    let frame = memory::allocate_frame().map_err(|_| ENOMEM)?;
                    unsafe {
                        zero_table(&frame);
                    }
                    pending.tables.push(Table { level, key, frame });
                }
            }
        }
        // Reserve every metadata mutation before publishing any descriptor.
        self.pages
            .try_reserve(pending.pages.len())
            .map_err(|_| ENOMEM)?;
        self.tables
            .try_reserve(pending.tables.len())
            .map_err(|_| ENOMEM)?;
        let mut removed = Vec::new();
        removed
            .try_reserve_exact(self.pages.len() - remaining)
            .map_err(|_| ENOMEM)?;
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        let added_pages = pending.pages.len() as u64;
        let added_tables = pending.tables.len() as u64;
        for page in &self.pages {
            if start <= page.address && page.address < end {
                assert_eq!(
                    unsafe { ptr::read_volatile(self.leaf(page.address).unwrap()) },
                    page_descriptor(page)
                );
            }
        }
        // Break old mappings before releasing/reusing frames or rights.
        for page in &self.pages {
            if start <= page.address && page.address < end {
                unsafe {
                    ptr::write_volatile(self.leaf(page.address).unwrap(), 0);
                }
            }
        }
        unsafe {
            invalidate_asid(asid);
        }
        let mut index = 0;
        while index < self.pages.len() {
            if remove(&self.pages[index]) {
                removed.push(self.pages.swap_remove(index));
            } else {
                index += 1;
            }
        }
        // L2 parents precede their L3 children in the prepared list.
        for table in pending.tables.drain(..) {
            let entry = if table.level == 2 {
                unsafe {
                    (*(l1.start_address() as *mut PageTable))
                        .0
                        .as_mut_ptr()
                        .add(table.key)
                }
            } else {
                let parent = self.table(2, table.key >> 9).expect("prepared L2 parent");
                unsafe {
                    (*(parent as *mut PageTable))
                        .0
                        .as_mut_ptr()
                        .add(table.key & 511)
                }
            };
            assert_eq!(unsafe { ptr::read_volatile(entry) }, 0);
            unsafe {
                publish_new_hierarchy(
                    entry,
                    table.frame.start_address() as u64 | DESCRIPTOR_TABLE_OR_PAGE,
                );
            }
            self.tables.push(table);
        }
        self.pages.append(&mut pending.pages);
        if let Some(protection) = change.protection {
            for page in &mut self.pages {
                if start <= page.address && page.address < end {
                    page.protection = protection;
                }
            }
        }
        for page in &self.pages {
            if start <= page.address && page.address < end {
                if page.protection == Protection::ReadExecute {
                    crate::userboot::synchronize_instruction_cache(
                        page.frame.start_address(),
                        PAGE_SIZE,
                    );
                }
                unsafe {
                    ptr::write_volatile(self.leaf(page.address).unwrap(), page_descriptor(page));
                }
            }
        }
        unsafe {
            invalidate_asid(asid);
        }
        drop(guard);
        let live = LIVE_PAGES.fetch_add(added_pages, Ordering::AcqRel) + added_pages;
        PEAK_PAGES.fetch_max(live, Ordering::Relaxed);
        LIVE_TABLES.fetch_add(added_tables, Ordering::AcqRel);
        self.layout = change.layout;
        reclaim_pages(removed.len() as u64);
        for page in removed {
            release_destroyed_address_space_frame(page.frame, "Linux munmap");
        }
        Ok(())
    }

    fn table(&self, level: u8, key: usize) -> Option<usize> {
        self.tables
            .iter()
            .find(|table| table.level == level && table.key == key)
            .map(|table| table.frame.start_address())
    }

    fn leaf(&self, address: usize) -> Option<*mut u64> {
        let table = self.table(3, address >> 21)?;
        Some(unsafe {
            (*(table as *mut PageTable))
                .0
                .as_mut_ptr()
                .add((address >> 12) & 511)
        })
    }

    pub(super) fn access(&self, address: usize) -> Option<UserPageAccess> {
        self.pages
            .iter()
            .find(|page| page.address == address)
            .and_then(|page| match page.protection {
                Protection::None => None,
                Protection::Read => Some(UserPageAccess::ReadOnly),
                Protection::ReadWrite => Some(UserPageAccess::ReadWrite),
                Protection::ReadExecute => Some(UserPageAccess::ReadExecute),
            })
    }

    pub(super) fn verify(&self, l1: &OwnedFrame) -> bool {
        for key in (0..ENTRY_COUNT).filter(|key| linux_vm::is_arena_address((*key as u64) << 30)) {
            let expected = self
                .table(2, key)
                .map_or(0, |address| address as u64 | DESCRIPTOR_TABLE_OR_PAGE);
            if unsafe { ptr::read_volatile((l1.start_address() as *const u64).add(key)) }
                != expected
            {
                return false;
            }
        }
        for table in &self.tables {
            for index in 0..ENTRY_COUNT {
                let expected = if table.level == 2 {
                    self.table(3, (table.key << 9) + index)
                        .map_or(0, |address| address as u64 | DESCRIPTOR_TABLE_OR_PAGE)
                } else {
                    let address = (table.key << 21) + index * PAGE_SIZE;
                    self.pages
                        .iter()
                        .find(|page| page.address == address)
                        .map_or(0, page_descriptor)
                };
                if unsafe {
                    ptr::read_volatile((table.frame.start_address() as *const u64).add(index))
                } != expected
                {
                    return false;
                }
            }
        }
        true
    }

    pub(super) unsafe fn detach(&self, l1: &OwnedFrame) {
        for table in &self.tables {
            if table.level == 2 {
                unsafe {
                    ptr::write_volatile((l1.start_address() as *mut u64).add(table.key), 0);
                }
            }
        }
    }

    pub(super) fn release(mut self) {
        reclaim_pages(self.pages.len() as u64);
        LIVE_TABLES.fetch_sub(self.tables.len() as u64, Ordering::AcqRel);
        for page in self.pages.drain(..) {
            release_destroyed_address_space_frame(page.frame, "Linux user leaf");
        }
        for table in self.tables.drain(..) {
            release_destroyed_address_space_frame(table.frame, "Linux private table");
        }
    }

    pub(super) fn owns_frame(&self, frame: usize) -> bool {
        self.pages
            .iter()
            .any(|page| page.frame.start_address() == frame)
    }
    pub(super) fn disjoint_from(&self, other: &Self, image: &[OwnedUserPage]) -> bool {
        self.pages.iter().all(|page| {
            !other.owns_frame(page.frame.start_address())
                && image
                    .iter()
                    .all(|other| other.frame.start_address() != page.frame.start_address())
        })
    }
    pub(super) fn table_count(&self) -> usize {
        self.tables.len()
    }
}

fn reclaim_pages(count: u64) {
    let old = LIVE_PAGES.fetch_sub(count, Ordering::AcqRel);
    assert!(old >= count, "Linux live page accounting underflow");
    RECLAIMED_PAGES.fetch_add(count, Ordering::Relaxed);
}

fn page_descriptor(page: &Page) -> u64 {
    match page.protection {
        Protection::None => 0,
        Protection::Read => user_ro_nx_page_descriptor(page.frame.start_address()).unwrap(),
        Protection::ReadWrite => user_rw_nx_page_descriptor(page.frame.start_address()).unwrap(),
        Protection::ReadExecute => user_ro_x_page_descriptor(page.frame.start_address()).unwrap(),
    }
}

impl UserAddressSpace {
    #[cfg(feature = "art-runtime")]
    pub fn linux_msync(&self, address: u64, length: u64, flags: u64) -> Result<u64, i64> {
        // Supported file mappings are private and cannot dirty their source;
        // shared anonymous pages have no persistent backing. Reject holes and
        // privileged identity mappings despite their visibility to the kernel.
        let result = self.linux.layout.msync(address, length, flags);
        if result != Err(ENOMEM) {
            return result;
        }
        let end = address
            .checked_add(linux_vm::rounded_length(length)?)
            .ok_or(ENOMEM)?;
        if end > INIT_USER_WINDOW_END as u64 || address < INIT_USER_WINDOW_START as u64 {
            return Err(ENOMEM);
        }
        for page in (address..end).step_by(PAGE_SIZE) {
            if !self
                .mappings
                .pages
                .iter()
                .any(|entry| entry.virtual_address as u64 == page)
            {
                return Err(ENOMEM);
            }
        }
        Ok(0)
    }
    #[cfg(feature = "bionic-dynamic-runtime")]
    pub fn linux_madvise(&mut self, address: u64, length: u64, advice: u64) -> Result<u64, i64> {
        if !matches!(advice, 18 | 19) {
            return Err(EINVAL);
        }
        self.linux.layout = self
            .linux
            .layout
            .advise_fork(address, length, advice == 18)?;
        Ok(0)
    }

    #[cfg(feature = "bionic-dynamic-runtime")]
    pub fn linux_executable(&self, address: usize) -> bool {
        let page = address & !(PAGE_SIZE - 1);
        self.mappings
            .pages
            .iter()
            .any(|p| p.virtual_address == page && p.access == UserPageAccess::ReadExecute)
            || self.linux.access(page) == Some(UserPageAccess::ReadExecute)
    }
    #[cfg(feature = "bionic-dynamic-runtime")]
    pub fn linux_mmap_file(&mut self, args: [u64; 6], bytes: &'static [u8]) -> Result<u64, i64> {
        if args[3] & 0x20 != 0 || args[3] & 3 != 2 || !args[5].is_multiple_of(PAGE_SIZE as u64) {
            return Err(EINVAL);
        }
        let length = linux_vm::rounded_length(args[1])?;
        let available = (bytes.len() as u64).saturating_sub(args[5]);
        // Linux permits overmapping EOF. Keep complete pages beyond the file
        // absent, including on later mprotect, instead of exposing zero pages.
        // Access is contained as a user fault until SIGBUS delivery exists.
        let change = self.linux.layout.map_with_backing(
            args[0],
            args[1],
            args[2],
            args[3] | 0x20,
            args[5],
            Some(available),
        )?;
        let start = change.start;
        let count = length.min(available) as usize;
        self.linux.apply(
            &self.level_1,
            self.asid(),
            change,
            Some(
                &bytes[(args[5] as usize).min(bytes.len())
                    ..(args[5] as usize).min(bytes.len()) + count],
            ),
        )?;
        Ok(start)
    }

    pub fn linux_mmap(&mut self, args: [u64; 6]) -> Result<u64, i64> {
        self.linux.map(&self.level_1, self.asid(), args)
    }
    pub fn linux_munmap(&mut self, address: u64, length: u64) -> Result<u64, i64> {
        self.linux
            .unmap(&self.level_1, self.asid(), address, length)
    }
    pub fn linux_mprotect(
        &mut self,
        address: u64,
        length: u64,
        protection: u64,
    ) -> Result<u64, i64> {
        if address >= linux_vm::ARENA_START || linux_vm::is_arena_address(address) {
            return self
                .linux
                .protect(&self.level_1, self.asid(), address, length, protection);
        }
        // Bionic protects RELRO and temporarily unlocks its mutable globals.
        // Only originally writable LOAD pages may switch between R and RW;
        // original code/rodata never acquire write permission.
        if !matches!(protection, 1 | 3) || !address.is_multiple_of(PAGE_SIZE as u64) {
            return Err(EACCES);
        }
        let size = linux_vm::rounded_length(length)?;
        let end = address.checked_add(size).ok_or(ENOMEM)?;
        for page in (address..end).step_by(PAGE_SIZE) {
            if !self
                .mappings
                .metadata
                .layout
                .vmas
                .iter()
                .flatten()
                .any(|vma| {
                    vma.kind == UserVmaKind::Load
                        && vma.access == UserPageAccess::ReadWrite
                        && vma.start as u64 <= page
                        && page < vma.end as u64
                })
            {
                return Err(ENOMEM);
            }
        }
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        for page in &self.mappings.pages {
            if address <= page.virtual_address as u64 && (page.virtual_address as u64) < end {
                let table =
                    user_level_3_table_mut(&self.user_level_3, page.virtual_address).unwrap();
                let leaf = unsafe {
                    (*table)
                        .0
                        .as_mut_ptr()
                        .add((page.virtual_address >> 12) & 511)
                };
                assert_eq!(
                    unsafe { ptr::read_volatile(leaf) },
                    user_leaf_descriptor(page).unwrap()
                );
                unsafe {
                    ptr::write_volatile(leaf, 0);
                }
            }
        }
        unsafe {
            invalidate_asid(self.asid());
        }
        for page in &mut self.mappings.pages {
            if address <= page.virtual_address as u64 && (page.virtual_address as u64) < end {
                page.access = if protection == 1 {
                    UserPageAccess::ReadOnly
                } else {
                    UserPageAccess::ReadWrite
                };
                let table =
                    user_level_3_table_mut(&self.user_level_3, page.virtual_address).unwrap();
                unsafe {
                    ptr::write_volatile(
                        (*table)
                            .0
                            .as_mut_ptr()
                            .add((page.virtual_address >> 12) & 511),
                        user_leaf_descriptor(page).unwrap(),
                    );
                }
            }
        }
        unsafe {
            invalidate_asid(self.asid());
        }
        drop(guard);
        Ok(0)
    }
}
