//! Bounded anonymous memory files. File descriptors and VMAs retain the same
//! open description. The runtime serializes syscalls and EL0 access on one CPU.
//! No host filesystem, disk, or cross-process sharing is involved.
use crate::{
    linux_pages::Page,
    memory::{PAGE_SIZE, PhysFrame},
};
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

pub const MAX_BYTES: u64 = 128 * 1024 * 1024;
pub const SEAL_SEAL: u32 = 1;
pub const SEAL_SHRINK: u32 = 2;
pub const SEAL_GROW: u32 = 4;
pub const SEAL_WRITE: u32 = 8;
pub const SEAL_FUTURE_WRITE: u32 = 16;
static NEXT_INODE: AtomicU64 = AtomicU64::new(0x10_0000);

struct State {
    length: u64,
    seals: u32,
    pages: Vec<Page>,
}
pub struct MemoryFile {
    inode: u64,
    name: Vec<u8>,
    proc_name: Vec<u8>,
    locked: AtomicBool,
    state: UnsafeCell<State>,
}
// All metadata access is locked; actual mapped bytes are accessed only in the
// runtime's serialized address-space domain. No Rust slices alias user memory.
unsafe impl Send for MemoryFile {}
unsafe impl Sync for MemoryFile {}

pub struct Resize {
    previous: u64,
    length: u64,
    pages: Vec<Page>,
}
impl MemoryFile {
    pub fn new(name: &[u8], flags: u32) -> Result<Self, i64> {
        if flags & !3 != 0 || name.len() > 249 || name.contains(&0) {
            return Err(22);
        }
        let mut owned_name = Vec::new();
        owned_name.try_reserve_exact(name.len()).map_err(|_| 12)?;
        owned_name.extend_from_slice(name);
        let mut proc_name = Vec::new();
        proc_name
            .try_reserve_exact(16 + name.len() * 4)
            .map_err(|_| 12)?;
        proc_name.extend_from_slice(b"memfd:");
        for &byte in name {
            if byte.is_ascii_graphic() && byte != b'\\' || byte == b' ' {
                proc_name.push(byte);
            } else {
                proc_name.extend_from_slice(&[
                    b'\\',
                    b'0' + (byte >> 6),
                    b'0' + ((byte >> 3) & 7),
                    b'0' + (byte & 7),
                ]);
            }
        }
        proc_name.extend_from_slice(b" (deleted)");
        Ok(Self {
            inode: NEXT_INODE.fetch_add(1, Ordering::Relaxed),
            name: owned_name,
            proc_name,
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(State {
                length: 0,
                seals: if flags & 2 == 0 { SEAL_SEAL } else { 0 },
                pages: Vec::new(),
            }),
        })
    }
    fn with<R>(&self, operation: impl FnOnce(&mut State) -> R) -> R {
        assert!(
            self.locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok(),
            "concurrent memfd operation outside serialized runtime"
        );
        struct Unlock<'a>(&'a AtomicBool);
        impl Drop for Unlock<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _unlock = Unlock(&self.locked);
        operation(unsafe { &mut *self.state.get() })
    }
    pub fn inode(&self) -> u64 {
        self.inode
    }
    pub fn name(&self) -> &[u8] {
        &self.name
    }
    pub fn proc_name(&self) -> &str {
        core::str::from_utf8(&self.proc_name).expect("escaped ASCII memfd name")
    }
    pub fn len(&self) -> u64 {
        self.with(|s| s.length)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn seals(&self) -> u32 {
        self.with(|s| s.seals)
    }
    pub fn may_write(&self) -> bool {
        self.seals() & (SEAL_WRITE | SEAL_FUTURE_WRITE) == 0
    }
    pub fn add_seals(&self, seals: u64, writable_mapping: bool) -> Result<u64, i64> {
        if seals & !31 != 0 {
            return Err(22);
        }
        self.with(|s| {
            if s.seals & SEAL_SEAL != 0 {
                return Err(1);
            }
            if seals & SEAL_WRITE as u64 != 0 && writable_mapping {
                return Err(16);
            }
            s.seals |= seals as u32;
            Ok(0)
        })
    }
    pub fn prepare_resize(&self, length: u64) -> Result<Resize, i64> {
        if length > i64::MAX as u64 {
            return Err(22);
        }
        if length > MAX_BYTES {
            return Err(27);
        }
        self.with(|s| {
            if length < s.length && s.seals & SEAL_SHRINK != 0
                || length > s.length && s.seals & SEAL_GROW != 0
            {
                return Err(1);
            }
            let count = length.div_ceil(PAGE_SIZE as u64) as usize;
            let extra = count.saturating_sub(s.pages.len());
            // Reserve the destination as well, so commit cannot allocate after
            // translations have been withdrawn. Failure leaves file bytes intact.
            s.pages.try_reserve(extra).map_err(|_| 12)?;
            let mut pages = Vec::new();
            pages.try_reserve_exact(extra).map_err(|_| 12)?;
            for _ in 0..extra {
                pages.push(Page::allocate()?);
            }
            Ok(Resize {
                previous: s.length,
                length,
                pages,
            })
        })
    }
    /// # Safety
    /// The caller must first invalidate every alias beyond the new EOF and
    /// serialize access to the owning address space, including user execution.
    pub unsafe fn commit_resize(&self, mut resize: Resize) {
        self.with(|s| {
            assert_eq!(s.length, resize.previous);
            s.pages.append(&mut resize.pages);
            s.pages
                .truncate(resize.length.div_ceil(PAGE_SIZE as u64) as usize);
            if resize.length < s.length && !resize.length.is_multiple_of(PAGE_SIZE as u64) {
                let offset = resize.length as usize % PAGE_SIZE;
                let page = s.pages.last().unwrap();
                unsafe {
                    core::ptr::write_bytes(
                        (page.start_address() + offset) as *mut u8,
                        0,
                        PAGE_SIZE - offset,
                    );
                }
            }
            s.length = resize.length;
        });
    }
    pub fn page(&self, offset: u64) -> Option<PhysFrame> {
        self.with(|s| {
            s.pages
                .get((offset / PAGE_SIZE as u64) as usize)
                .map(Page::frame)
        })
    }
    pub fn read(&self, offset: u64, bytes: &mut [u8]) -> usize {
        self.with(|s| {
            let count = bytes.len().min(s.length.saturating_sub(offset) as usize);
            let mut copied = 0;
            while copied < count {
                let at = offset as usize + copied;
                let n = (count - copied).min(PAGE_SIZE - at % PAGE_SIZE);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        (s.pages[at / PAGE_SIZE].start_address() + at % PAGE_SIZE) as *const u8,
                        bytes.as_mut_ptr().add(copied),
                        n,
                    );
                }
                copied += n;
            }
            count
        })
    }
    // Writes are staged in a kernel buffer. Resizing happens in the MMU owner,
    // because extending a file also changes EOF fault behavior of existing VMAs.
    pub fn write(&self, offset: u64, bytes: &[u8]) -> Result<usize, i64> {
        self.with(|s| {
            if s.seals & (SEAL_WRITE | SEAL_FUTURE_WRITE) != 0 {
                return Err(1);
            }
            if offset
                .checked_add(bytes.len() as u64)
                .is_none_or(|end| end > s.length)
            {
                return Err(22);
            }
            let mut copied = 0;
            while copied < bytes.len() {
                let at = offset as usize + copied;
                let n = (bytes.len() - copied).min(PAGE_SIZE - at % PAGE_SIZE);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        bytes.as_ptr().add(copied),
                        (s.pages[at / PAGE_SIZE].start_address() + at % PAGE_SIZE) as *mut u8,
                        n,
                    );
                }
                copied += n;
            }
            Ok(copied)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flags_names_and_seals_fail_without_mutation() {
        assert!(matches!(MemoryFile::new(b"name", 4), Err(22)));
        assert!(matches!(MemoryFile::new(&[b'a'; 250], 0), Err(22)));
        let fixed = MemoryFile::new(b"", 0).unwrap();
        assert_eq!(fixed.seals(), SEAL_SEAL);
        assert_eq!(fixed.add_seals(SEAL_WRITE as u64, false), Err(1));
        let memory = MemoryFile::new(b"jit-cache", 3).unwrap();
        assert_ne!(memory.inode(), fixed.inode());
        let escaped = MemoryFile::new(b"line\n\\\xff", 0).unwrap();
        assert_eq!(escaped.proc_name(), "memfd:line\\012\\134\\377 (deleted)");
        assert_eq!(memory.add_seals(32, false), Err(22));
        assert_eq!(memory.add_seals(SEAL_WRITE as u64, true), Err(16));
        assert_eq!(memory.seals(), 0);
        assert_eq!(memory.add_seals(SEAL_FUTURE_WRITE as u64, true), Ok(0));
        assert!(!memory.may_write());
        assert_eq!(memory.write(0, b"x"), Err(1));
        assert_eq!(
            memory.add_seals((SEAL_GROW | SEAL_SEAL) as u64, false),
            Ok(0)
        );
        assert!(matches!(memory.prepare_resize(4096), Err(1)));
        assert!(matches!(memory.prepare_resize(MAX_BYTES + 1), Err(27)));
        assert_eq!(memory.len(), 0);
        assert_eq!(memory.add_seals(0, false), Err(1));
    }
}
