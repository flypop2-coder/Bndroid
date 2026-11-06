//! Sparse Linux mappings with private or retained anonymous-file backing.
//! User addresses cannot select kernel tables or the native image/stack window.
use super::*;
use bndroid_kernel::linux_vm::{
    self, Change, EACCES, EINVAL, ENOMEM, Layout as VmLayout, Protection,
};

use bndroid_kernel::linux_pages;
const MAX_PAGES: usize = linux_pages::MAX_PAGES;
const MAX_TABLES: usize = if cfg!(feature = "framework-runtime") {
    512
} else {
    256
};
static LIVE_TABLES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "art-runtime")]
static DISCARDED_PAGES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "art-runtime")]
static DEMAND_ZERO_PAGES: AtomicU64 = AtomicU64::new(0);

pub struct LinuxMemorySnapshot {
    pub pages: u64,
    pub tables: u64,
    pub peak_pages: u64,
    pub reclaimed_pages: u64,
    #[cfg(feature = "art-runtime")]
    pub discarded_pages: u64,
    #[cfg(feature = "art-runtime")]
    pub demand_zero_pages: u64,
}
pub fn linux_memory_snapshot() -> LinuxMemorySnapshot {
    let (pages, peak_pages, reclaimed_pages) = linux_pages::snapshot();
    LinuxMemorySnapshot {
        pages,
        tables: LIVE_TABLES.load(Ordering::Acquire),
        peak_pages,
        reclaimed_pages,
        #[cfg(feature = "art-runtime")]
        discarded_pages: DISCARDED_PAGES.load(Ordering::Acquire),
        #[cfg(feature = "art-runtime")]
        demand_zero_pages: DEMAND_ZERO_PAGES.load(Ordering::Acquire),
    }
}

// Physical frames are page aligned. Keep ownership in the low bit rather than
// a two-word enum: each of up to 131072 mapping records stays 24 bytes, avoiding
// large vector reallocations that fragment the bounded 8 MiB kernel heap.
struct Frame(usize);
impl Frame {
    fn private(page: linux_pages::Page) -> Self {
        Self(page.into_address() | 1)
    }
    #[cfg(feature = "art-runtime")]
    fn shared(frame: PhysFrame) -> Self {
        Self(frame.start_address())
    }
    fn start_address(&self) -> usize {
        self.0 & !1
    }
}
impl Drop for Frame {
    fn drop(&mut self) {
        if self.0 & 1 != 0 {
            // This token was transferred by private(), never copied, and is
            // released only after the owning mapping has invalidated its PTE.
            drop(unsafe { linux_pages::Page::from_address(self.start_address()) });
        }
    }
}
struct Page {
    address: usize,
    frame: Frame,
    protection: Protection,
}
const _: () = assert!(core::mem::size_of::<Page>() == 24);
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
    #[cfg(feature = "art-runtime")]
    memories: Vec<bndroid_kernel::linux_fd::FileRef>,
}

#[derive(Default)]
struct Prepared {
    pages: Vec<Page>,
    tables: Vec<Table>,
}
#[derive(Clone, Copy)]
enum Source<'a> {
    File(&'a [u8]),
    #[cfg(feature = "art-runtime")]
    Memory(&'a bndroid_kernel::linux_memfd::MemoryFile),
    #[cfg(feature = "art-runtime")]
    Move {
        start: usize,
        end: usize,
    },
}
impl Drop for Prepared {
    fn drop(&mut self) {
        for page in self.pages.drain(..) {
            drop(page);
        }
        for table in self.tables.drain(..) {
            release_destroyed_address_space_frame(table.frame, "unpublished Linux table");
        }
    }
}

impl LinuxMappings {
    #[cfg(feature = "art-runtime")]
    fn retain_memories(&mut self) {
        for opened in &self.memories {
            let inode = opened.memory().unwrap().inode();
            if let Some(mapping) =
                bndroid_kernel::linux_binder::with(|r| r.mapping_for_inode(inode))
            {
                let end = mapping.base + mapping.length;
                let mut cursor = mapping.base;
                for region in &self.layout.regions {
                    if region.end <= cursor {
                        continue;
                    }
                    if region.start > cursor
                        || !region.file.is_some_and(|f| {
                            f.memory
                                && f.inode == inode
                                && f.offset + cursor - region.file_origin == cursor - mapping.base
                        })
                    {
                        break;
                    }
                    cursor = region.end;
                    if cursor >= end {
                        break;
                    }
                }
                if cursor < end {
                    bndroid_kernel::linux_binder::with(|r| r.unmap_inode(inode));
                }
            }
        }
        self.memories.retain(|opened| {
            let inode = opened.memory().unwrap().inode();
            self.layout
                .regions
                .iter()
                .any(|r| r.file.is_some_and(|f| f.memory && f.inode == inode))
        });
    }

    #[cfg(feature = "art-runtime")]
    fn update_memory_aliases(
        &mut self,
        asid: u8,
        memory: &bndroid_kernel::linux_memfd::MemoryFile,
        length: u64,
    ) {
        if !self
            .memories
            .iter()
            .any(|f| f.memory().is_some_and(|m| m.inode() == memory.inode()))
        {
            return;
        }
        let eof = length.div_ceil(PAGE_SIZE as u64) * PAGE_SIZE as u64;
        let remove = |page: &Page| {
            self.layout.regions.iter().any(|r| {
                r.start <= page.address as u64
                    && (page.address as u64) < r.end
                    && r.file.is_some_and(|f| {
                        f.memory
                            && f.inode == memory.inode()
                            && f.offset + page.address as u64 - r.file_origin >= eof
                    })
            })
        };
        for page in &self.pages {
            if remove(page) {
                unsafe {
                    ptr::write_volatile(self.leaf(page.address).unwrap(), 0);
                }
            }
        }
        unsafe {
            invalidate_asid(asid);
        }
        // Shared Page entries do not own physical frames. Drop their aliases
        // before truncating the backing object's unique frame ownership.
        self.pages.retain(|page| !remove(page));
        for region in &mut self.layout.regions {
            if let Some(file) = region.file.as_mut()
                && file.memory
                && file.inode == memory.inode()
            {
                file.available = length.saturating_sub(file.offset);
                region.file_page_end = region.file_origin
                    + file.available.div_ceil(PAGE_SIZE as u64) * PAGE_SIZE as u64;
            }
        }
    }

    #[cfg(feature = "art-runtime")]
    fn demand_memory(
        &mut self,
        l1: &OwnedFrame,
        asid: u8,
        address: usize,
        write: bool,
        execute: bool,
    ) -> Result<bool, i64> {
        if self.pages.iter().any(|page| page.address == address) {
            return Ok(false);
        }
        let Some(region) = self.layout.regions.iter().find(|r| {
            r.start <= address as u64
                && (address as u64) < r.end
                && r.file.is_some_and(|f| f.memory)
        }) else {
            return Ok(false);
        };
        if address as u64 >= region.file_page_end
            || region.protection == Protection::None
            || write && region.protection != Protection::ReadWrite
            || execute && region.protection != Protection::ReadExecute
        {
            return Ok(false);
        }
        let protection = match region.protection {
            Protection::Read => 1,
            Protection::ReadWrite => 3,
            Protection::ReadExecute => 5,
            Protection::None => unreachable!(),
        };
        let change = self
            .layout
            .protect(address as u64, PAGE_SIZE as u64, protection)?;
        self.apply(l1, asid, change, None)?;
        Ok(true)
    }

    #[cfg(feature = "art-runtime")]
    fn discard(&mut self, asid: u8, address: u64, length: u64) -> Result<u64, i64> {
        if length == 0 {
            return if address.is_multiple_of(linux_vm::PAGE) {
                Ok(0)
            } else {
                Err(EINVAL)
            };
        }
        let end = self.layout.private_anonymous_range(address, length)?;
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        for page in &self.pages {
            if address <= page.address as u64 && (page.address as u64) < end {
                let leaf = self
                    .leaf(page.address)
                    .expect("resident page has a private table");
                assert_eq!(unsafe { ptr::read_volatile(leaf) }, page_descriptor(page));
                unsafe {
                    ptr::write_volatile(leaf, 0);
                }
            }
        }
        // Remove all translations before any physical frame can be reused.
        unsafe {
            invalidate_asid(asid);
        }
        drop(guard);
        let mut index = 0;
        let mut count = 0;
        while index < self.pages.len() {
            let page = &self.pages[index];
            if address <= page.address as u64 && (page.address as u64) < end {
                let page = self.pages.swap_remove(index);
                drop(page);
                count += 1;
            } else {
                index += 1;
            }
        }
        DISCARDED_PAGES.fetch_add(count, Ordering::Relaxed);
        // Keep the VMA's rights and private empty tables. A later permitted
        // access repopulates a fresh zero page; PROT_NONE stays inaccessible.
        Ok(0)
    }

    #[cfg(feature = "art-runtime")]
    #[inline(never)]
    fn demand_zero(
        &mut self,
        asid: u8,
        address: usize,
        write: bool,
        execute: bool,
    ) -> Result<bool, i64> {
        if self.pages.iter().any(|page| page.address == address) {
            return Ok(false);
        }
        let Some(region) = self
            .layout
            .regions
            .iter()
            .find(|r| r.start <= address as u64 && (address as u64) < r.end)
        else {
            return Ok(false);
        };
        let protection = region.protection;
        if region.file_backed
            || region.shared
            || protection == Protection::None
            || write && protection != Protection::ReadWrite
            || execute && protection != Protection::ReadExecute
        {
            return Ok(false);
        }
        if self.pages.len() >= MAX_PAGES {
            return Err(ENOMEM);
        }
        // Only discarded resident pages can reach this path; their tables
        // remain owned. Never create hierarchy in a privileged arena.
        let Some(leaf) = self.leaf(address) else {
            return Ok(false);
        };
        self.pages.try_reserve(1).map_err(|_| ENOMEM)?;
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        assert_eq!(unsafe { ptr::read_volatile(leaf) }, 0);
        let frame = Frame::private(linux_pages::Page::allocate()?);
        if execute {
            crate::userboot::synchronize_instruction_cache(frame.start_address(), PAGE_SIZE);
        }
        let page = Page {
            address,
            frame,
            protection,
        };
        unsafe {
            ptr::write_volatile(leaf, page_descriptor(&page));
            invalidate_asid(asid);
        }
        self.pages.push(page);
        drop(guard);
        DEMAND_ZERO_PAGES.fetch_add(1, Ordering::Relaxed);
        Ok(true)
    }
    #[cfg(feature = "art-runtime")]
    fn remap(&mut self, l1: &OwnedFrame, asid: u8, arguments: [u64; 5]) -> Result<u64, i64> {
        let [address, old_length, new_length, flags, destination] = arguments;
        if flags & !7 != 0 {
            return Err(EINVAL);
        }
        let fixed = flags & 2 != 0;
        if fixed && flags & 1 == 0 {
            return Err(EINVAL);
        }
        if flags & 4 != 0 {
            return Err(38);
        } // DONTUNMAP needs separate retained-source fault semantics.
        let old = linux_vm::rounded_length(old_length)?;
        let new = linux_vm::rounded_length(new_length)?;
        let old_end = linux_vm::checked_range(address, old)?;
        if fixed {
            let end = linux_vm::checked_range(destination, new)?;
            if destination < old_end && address < end {
                return Err(EINVAL);
            }
        }
        let region = *self
            .layout
            .regions
            .iter()
            .find(|r| r.start <= address && old_end <= r.end)
            .ok_or(EINVAL)?;
        if region.file_backed || region.shared {
            return Err(38);
        }
        if !fixed && old == new {
            return Ok(address);
        }
        if !fixed && new < old {
            let change = self.layout.unmap(address + new, old - new)?;
            self.apply(l1, asid, change, None)?;
            return Ok(address);
        }
        let protection = match region.protection {
            Protection::None => 0,
            Protection::Read => 1,
            Protection::ReadWrite => 3,
            Protection::ReadExecute => 5,
        };
        if !fixed {
            let mut growth = self.layout.map(old_end, new - old, protection, 0x100022, 0);
            if let Ok(change) = growth.as_mut()
                && region.wipe_on_fork
            {
                change.layout = change.layout.advise_fork(old_end, new - old, true)?;
            }
            if let Ok(change) = growth.as_mut()
                && region.dont_fork
            {
                change.layout = change.layout.advise_dontfork(old_end, new - old, true)?;
            }
            if let Ok(change) = growth {
                self.apply(l1, asid, change, None)?;
                return Ok(address);
            }
            if flags & 1 == 0 {
                return Err(ENOMEM);
            }
        }
        let hint = if fixed {
            destination
        } else if address < linux_vm::LOW_END {
            linux_vm::LOW_START
        } else {
            0
        };
        let mut change =
            self.layout
                .map(hint, new, protection, if fixed { 0x32 } else { 0x22 }, 0)?;
        let result = change.start;
        change.layout = change.layout.unmap(address, old)?.layout;
        if region.wipe_on_fork {
            change.layout = change.layout.advise_fork(result, new, true)?;
        }
        if region.dont_fork {
            change.layout = change.layout.advise_dontfork(result, new, true)?;
        }
        // Both layout changes, all destination pages/copies and metadata are
        // prepared before either address changes. Allocation failure leaves
        // the old mapping and contents intact; no rollback allocation occurs.
        self.apply(
            l1,
            asid,
            change,
            Some(Source::Move {
                start: address as usize,
                end: old_end as usize,
            }),
        )?;
        Ok(result)
    }
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
        let result = self
            .layout
            .protect(address, length, protection)
            .and_then(|change| self.apply(l1, asid, change, None));
        #[cfg(feature = "art-runtime")]
        if let Err(error) = result {
            crate::kprintln!(
                "LINUX_MPROTECT_FAILED address={:#x} length={} protection={} errno={} regions={} pages={} tables={}",
                address,
                length,
                protection,
                error,
                self.layout.regions.len(),
                self.pages.len(),
                self.tables.len()
            );
        }
        result?;
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
        source: Option<Source<'_>>,
    ) -> Result<(), i64> {
        assert!(crate::arch::aarch64::irq_is_masked());
        let start = change.start as usize;
        let end = change.end as usize;
        let moving = |address: usize| {
            #[cfg(feature = "art-runtime")]
            if let Some(Source::Move { start, end }) = source {
                return start <= address && address < end;
            }
            let _ = address;
            false
        };
        let remove = |page: &Page| {
            change.replace && start <= page.address && page.address < end || moving(page.address)
        };
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
                #[cfg(feature = "art-runtime")]
                let previous = match source {
                    Some(Source::Move {
                        start: from,
                        end: until,
                    }) => {
                        let old = from + address - start;
                        if old < until {
                            self.pages.iter().find(|page| page.address == old)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                #[cfg(feature = "art-runtime")]
                if matches!(source, Some(Source::Move { .. }))
                    && previous.is_none()
                    && change.protection == Some(Protection::None)
                {
                    continue;
                }
                if remaining + pending.pages.len() >= MAX_PAGES {
                    return Err(ENOMEM);
                }
                #[cfg(feature = "art-runtime")]
                let shared = change.layout.regions.iter().find_map(|region| {
                    let file = region.file?;
                    if !(file.memory
                        && region.start <= address as u64
                        && (address as u64) < region.end)
                    {
                        return None;
                    }
                    let memory = match source {
                        Some(Source::Memory(memory)) if memory.inode() == file.inode => memory,
                        _ => self
                            .memories
                            .iter()
                            .filter_map(|f| f.memory())
                            .find(|m| m.inode() == file.inode)
                            .expect("retained memfd mapping"),
                    };
                    Some(
                        memory
                            .page(file.offset + address as u64 - region.file_origin)
                            .expect("resident memfd page below EOF"),
                    )
                });
                #[cfg(feature = "art-runtime")]
                let frame = match shared {
                    Some(frame) => Frame::shared(frame),
                    None => Frame::private(linux_pages::Page::allocate()?),
                };
                #[cfg(not(feature = "art-runtime"))]
                let frame = Frame::private(linux_pages::Page::allocate()?);
                if let Some(Source::File(bytes)) = source {
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
                #[cfg(feature = "art-runtime")]
                if let Some(old) = previous {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            old.frame.start_address() as *const u8,
                            frame.start_address() as *mut u8,
                            PAGE_SIZE,
                        );
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
        // Replacements remove their old entries before appending new ones.
        // Reserving old+new would spuriously double the large page vector for
        // an equal-size MAP_FIXED update (ART's R-to-RX code-cache view).
        self.pages
            .try_reserve(
                pending
                    .pages
                    .len()
                    .saturating_sub(self.pages.len() - remaining),
            )
            .map_err(|_| ENOMEM)?;
        self.tables
            .try_reserve(pending.tables.len())
            .map_err(|_| ENOMEM)?;
        let mut removed = Vec::new();
        removed
            .try_reserve_exact(self.pages.len() - remaining)
            .map_err(|_| ENOMEM)?;
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        let added_tables = pending.tables.len() as u64;
        for page in &self.pages {
            if start <= page.address && page.address < end || moving(page.address) {
                assert_eq!(
                    unsafe { ptr::read_volatile(self.leaf(page.address).unwrap()) },
                    page_descriptor(page)
                );
            }
        }
        // Break old mappings before releasing/reusing frames or rights.
        for page in &self.pages {
            if start <= page.address && page.address < end || moving(page.address) {
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
        LIVE_TABLES.fetch_add(added_tables, Ordering::AcqRel);
        self.layout = change.layout;
        drop(removed);
        #[cfg(feature = "art-runtime")]
        self.retain_memories();
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
        let protection = self
            .pages
            .iter()
            .find(|page| page.address == address)
            .map(|page| page.protection);
        #[cfg(feature = "art-runtime")]
        let protection = protection.or_else(|| {
            self.layout
                .regions
                .iter()
                .find(|r| {
                    ((!r.file_backed && !r.shared)
                        || (r.file.is_some_and(|f| f.memory) && (address as u64) < r.file_page_end))
                        && r.start <= address as u64
                        && (address as u64) < r.end
                })
                .map(|r| r.protection)
        });
        protection.and_then(|protection| match protection {
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
        LIVE_TABLES.fetch_sub(self.tables.len() as u64, Ordering::AcqRel);
        for page in self.pages.drain(..) {
            drop(page);
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
    pub fn linux_resize_memory_file<'a>(
        memory: &bndroid_kernel::linux_memfd::MemoryFile,
        length: u64,
        spaces: impl Iterator<Item = &'a mut UserAddressSpace>,
    ) -> Result<u64, i64> {
        let prepared = memory.prepare_resize(length)?;
        let guard = PageTableGuard::acquire().map_err(|_| ENOMEM)?;
        // Every address space withdraws stale translations before any backing
        // frame can be freed. Allocation and lock failure precede all changes.
        for space in spaces {
            space
                .linux
                .update_memory_aliases(space.asid(), memory, length);
        }
        unsafe {
            memory.commit_resize(prepared);
        }
        drop(guard);
        Ok(0)
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_memfd_may_write(&self, inode: u64) -> bool {
        self.linux.layout.regions.iter().any(|r| {
            r.shared
                && r.file
                    .is_some_and(|f| f.memory && f.inode == inode && f.may_write)
        })
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_mmap_memory(
        &mut self,
        args: [u64; 6],
        opened: &bndroid_kernel::linux_fd::FileRef,
    ) -> Result<u64, i64> {
        self.linux_mmap_backing(args, opened, None)
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_mmap_binder(
        &mut self,
        mut args: [u64; 6],
        opened: &bndroid_kernel::linux_fd::FileRef,
        path: &'static str,
    ) -> Result<u64, i64> {
        // Binder accepts MAP_PRIVATE but its receive pool is kernel-owned;
        // userspace can never write it, obtain a COW view, or execute it.
        if args[2] != 1 || args[5] != 0 || !matches!(args[3] & 3, 1 | 2) {
            return Err(EACCES);
        }
        args[3] = (args[3] & !3) | 1;
        self.linux_mmap_backing(args, opened, Some(path))
    }
    #[cfg(feature = "art-runtime")]
    fn linux_mmap_backing(
        &mut self,
        args: [u64; 6],
        opened: &bndroid_kernel::linux_fd::FileRef,
        device_path: Option<&'static str>,
    ) -> Result<u64, i64> {
        let memory = opened.memory().ok_or(19)?;
        if args[5] > i64::MAX as u64 || !args[5].is_multiple_of(PAGE_SIZE as u64) {
            return Err(EINVAL);
        }
        // Private mutable-file mappings require copy-on-write, not independent
        // eager snapshots. Leave them unsupported until that behavior exists.
        if args[3] & 3 == 2 {
            return Err(38);
        }
        if args[3] & 3 != 1 || args[3] & 0x20 != 0 {
            return Err(EINVAL);
        }
        if args[2] & 2 != 0 && !opened.writable() {
            return Err(EACCES);
        }
        if args[2] & 2 != 0 && !memory.may_write() {
            return Err(1);
        }
        let mut change = self.linux.layout.map_with_backing(
            args[0],
            args[1],
            args[2],
            args[3] | 0x20,
            args[5],
            Some(linux_vm::FileBacking {
                available: memory.len().saturating_sub(args[5]),
                offset: args[5],
                inode: memory.inode(),
                path: device_path.unwrap_or("memfd:anonymous (deleted)"),
                memory: true,
                may_write: device_path.is_none() && opened.writable() && memory.may_write(),
                may_execute: device_path.is_none(),
            }),
        )?;
        if device_path.is_some() {
            for region in &mut change.layout.regions {
                if region.file.is_some_and(|f| f.inode == memory.inode()) {
                    region.dont_fork = true;
                }
            }
        }
        let start = change.start;
        if !self
            .linux
            .memories
            .iter()
            .any(|f| f.memory().unwrap().inode() == memory.inode())
        {
            self.linux.memories.try_reserve(1).map_err(|_| ENOMEM)?;
            self.linux.memories.push(opened.clone());
        }
        let result = self.linux.apply(
            &self.level_1,
            self.asid(),
            change,
            Some(Source::Memory(memory)),
        );
        self.linux.retain_memories();
        result?;
        Ok(start)
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_demand_fault(&mut self, esr: u64, address: u64) -> bool {
        let class = (esr >> 26) & 63;
        if !matches!(class, 0x20 | 0x24 | 0x25)
            || !(4..=7).contains(&(esr & 63))
            || esr & ((1 << 7) | (1 << 8) | (1 << 9) | (1 << 10)) != 0
        {
            return false;
        }
        let page = address as usize & !(PAGE_SIZE - 1);
        if self
            .linux
            .demand_memory(
                &self.level_1,
                self.asid(),
                page,
                class != 0x20 && esr & (1 << 6) != 0,
                class == 0x20,
            )
            .unwrap_or(false)
        {
            return true;
        }
        self.linux
            .demand_zero(
                self.asid(),
                page,
                class != 0x20 && esr & (1 << 6) != 0,
                class == 0x20,
            )
            .unwrap_or(false)
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_file_eof(&self, address: u64) -> bool {
        self.linux.layout.regions.iter().any(|r| {
            r.file_backed && r.start <= address && address < r.end && address >= r.file_page_end
        })
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_user_mapped(&self, address: u64) -> bool {
        self.mappings
            .pages
            .iter()
            .any(|page| page.virtual_address as u64 == address & !(PAGE_SIZE as u64 - 1))
            || self
                .linux
                .layout
                .regions
                .iter()
                .any(|region| region.start <= address && address < region.end)
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_signal_trampoline(&mut self) -> Result<u64, i64> {
        // mov x8, #139; svc #0; brk #0. Owned RX mapping, never a writable
        // stack trampoline or a shared privileged kernel page.
        static CODE: [u8; 12] = [0x68, 0x11, 0x80, 0xd2, 1, 0, 0, 0xd4, 0, 0, 0x20, 0xd4];
        self.linux_mmap_file(
            [0, PAGE_SIZE as u64, 5, 2, 0, 0],
            bndroid_kernel::linux_fs::Node {
                path: "[bndroid-sigreturn]",
                data: &CODE,
                kind: bndroid_kernel::linux_fs::Kind::File,
                mode: 0o555,
                inode: 0,
            },
        )
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_mremap(&mut self, r: &[u64; 31]) -> Result<u64, i64> {
        self.linux
            .remap(&self.level_1, self.asid(), r[..5].try_into().unwrap())
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_proc_snapshot(
        &self,
        stat: &mut bndroid_kernel::linux_proc::Stat,
        maps: bool,
    ) -> Result<Vec<u8>, i64> {
        use bndroid_kernel::linux_proc::{Map, Text};
        let failure = |stage: &str, bytes: usize, error: i64| {
            crate::kprintln!(
                "LINUX_PROC_SNAPSHOT_FAILED stage={} bytes={} limit={} regions={} native_pages={} heap_free={} errno={}",
                stage,
                bytes,
                bndroid_kernel::linux_proc::MAX_SNAPSHOT,
                self.linux.layout.regions.len(),
                self.mappings.pages.len(),
                crate::kernel_heap::stats().map_or(0, |s| s.free_bytes),
                error
            );
            error
        };
        let mut rows: Vec<Map> = Vec::new();
        rows.try_reserve_exact(self.mappings.pages.len() + self.linux.layout.regions.len())
            .map_err(|_| failure("rows", 0, ENOMEM))?;
        let layout = &self.mappings.metadata.layout;
        stat.resident_pages = (self.mappings.pages.len() + self.linux.pages.len()) as u64;
        stat.resident_limit = ((MAX_PAGES + self.mappings.pages.len()) * PAGE_SIZE) as u64;
        for page in &self.mappings.pages {
            let start = page.virtual_address as u64;
            let end = start + PAGE_SIZE as u64;
            if page.access == UserPageAccess::ReadExecute {
                stat.code_start = if stat.code_start == 0 {
                    start
                } else {
                    stat.code_start.min(start)
                };
                stat.code_end = stat.code_end.max(end);
            }
            rows.push(Map {
                start,
                end,
                read: page.access != UserPageAccess::NoAccess,
                write: page.access == UserPageAccess::ReadWrite,
                execute: page.access == UserPageAccess::ReadExecute,
                shared: false,
                offset: 0,
                inode: 0,
                name: if page.virtual_address >= layout.stack_start
                    && page.virtual_address < layout.stack_end
                {
                    "[stack]"
                } else {
                    "[bndroid-loader]"
                },
            });
        }
        for region in &self.linux.layout.regions {
            let (offset, inode, name) = region.file.map_or((0, 0, ""), |file| {
                (
                    file.offset + region.start - region.file_origin,
                    file.inode,
                    if file.memory && !file.path.starts_with("/dev/") {
                        self.linux
                            .memories
                            .iter()
                            .filter_map(|f| f.memory())
                            .find(|m| m.inode() == file.inode)
                            .expect("retained memfd proc name")
                            .proc_name()
                    } else {
                        file.path
                    },
                )
            });
            rows.push(Map {
                start: region.start,
                end: region.end,
                read: region.protection != Protection::None,
                write: region.protection == Protection::ReadWrite,
                execute: region.protection == Protection::ReadExecute,
                shared: region.shared,
                offset,
                inode,
                name,
            });
        }
        rows.sort_unstable_by_key(|row| row.start);
        stat.virtual_bytes = rows.iter().map(|row| row.end - row.start).sum();
        if !maps {
            return stat.text();
        }
        // Merge adjacent native pages only when their effective rights/name
        // agree. Sparse mappings already retain their file offset identity.
        let mut text = Text::new();
        let mut previous: Option<Map> = None;
        for row in rows {
            if let Some(p) = previous.as_mut() {
                if p.end == row.start
                    && p.read == row.read
                    && p.write == row.write
                    && p.execute == row.execute
                    && p.shared == row.shared
                    && p.name == row.name
                    && p.inode == row.inode
                    && ((p.inode == 0 && p.offset == row.offset)
                        || p.offset.checked_add(p.end - p.start) == Some(row.offset))
                {
                    p.end = row.end;
                    continue;
                }
                p.append(&mut text)
                    .map_err(|e| failure("text", text.bytes_written(), e))?;
            }
            previous = Some(row);
        }
        if let Some(row) = previous {
            row.append(&mut text)
                .map_err(|e| failure("text", text.bytes_written(), e))?;
        }
        if text.bytes_written() > 128 * 1024 {
            crate::kprintln!(
                "LINUX_PROC_SNAPSHOT_OK bytes={} limit={} regions={}",
                text.bytes_written(),
                bndroid_kernel::linux_proc::MAX_SNAPSHOT,
                self.linux.layout.regions.len()
            );
        }
        Ok(text.finish())
    }
    #[cfg(feature = "art-runtime")]
    pub fn linux_msync(&self, address: u64, length: u64, flags: u64) -> Result<u64, i64> {
        // Private files cannot dirty their source; shared files are immutable
        // and shared anonymous pages have no persistent backing. Reject holes and
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
        #[cfg(feature = "art-runtime")]
        if advice == 4 {
            return self.linux.discard(self.asid(), address, length);
        }
        #[cfg(feature = "art-runtime")]
        if matches!(advice, 0..=2) {
            // NORMAL/RANDOM/SEQUENTIAL are access-pattern hints. This profile
            // already populates pages eagerly and performs no speculative
            // read-ahead or eviction; all three retain the resident contents.
            // Validate the whole user-owned range, including inaccessible VMAs.
            return self.linux_msync(address, length, 0);
        }
        #[cfg(feature = "art-runtime")]
        if matches!(advice, 10 | 11) {
            self.linux.layout = self
                .linux
                .layout
                .advise_dontfork(address, length, advice == 10)?;
            return Ok(0);
        }
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
    pub fn linux_mmap_file(
        &mut self,
        args: [u64; 6],
        node: bndroid_kernel::linux_fs::Node<'static>,
    ) -> Result<u64, i64> {
        let bytes = node.data;
        if args[5] > i64::MAX as u64 {
            return Err(EINVAL);
        }
        let shared = cfg!(feature = "art-runtime") && args[3] & 3 == 1;
        if args[3] & 0x20 != 0
            || (args[3] & 3 != 2 && !shared)
            || !args[5].is_multiple_of(PAGE_SIZE as u64)
        {
            return Err(EINVAL);
        }
        if shared && args[2] & 2 != 0 {
            return Err(EACCES);
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
            Some(linux_vm::FileBacking {
                available,
                offset: args[5],
                path: node.path,
                inode: node.inode,
                memory: false,
                may_write: false,
                may_execute: true,
            }),
        )?;
        let start = change.start;
        let count = length.min(available) as usize;
        self.linux.apply(
            &self.level_1,
            self.asid(),
            change,
            Some(Source::File(
                &bytes[(args[5] as usize).min(bytes.len())
                    ..(args[5] as usize).min(bytes.len()) + count],
            )),
        )?;
        Ok(start)
    }

    pub fn linux_mmap(&mut self, args: [u64; 6]) -> Result<u64, i64> {
        self.linux.map(&self.level_1, self.asid(), args)
    }
    #[cfg(feature = "linux-syscall-trace")]
    pub fn linux_mapping_usage(&self) -> (usize, usize, usize) {
        (
            self.linux.pages.len(),
            self.linux.tables.len(),
            self.linux.layout.regions.len(),
        )
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
        // ART also installs a real inaccessible guard inside its initial stack.
        // Only originally writable pages can change; code/rodata stay immutable.
        if !(matches!(protection, 1 | 3) || cfg!(feature = "art-runtime") && protection == 0)
            || !address.is_multiple_of(PAGE_SIZE as u64)
        {
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
                    (vma.kind == UserVmaKind::Load
                        || cfg!(feature = "art-runtime") && vma.kind == UserVmaKind::Stack)
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
                page.access = match protection {
                    #[cfg(feature = "art-runtime")]
                    0 => UserPageAccess::NoAccess,
                    1 => UserPageAccess::ReadOnly,
                    _ => UserPageAccess::ReadWrite,
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
