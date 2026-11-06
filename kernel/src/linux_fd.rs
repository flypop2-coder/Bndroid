//! POSIX descriptor lifetime and shared open-description offsets. The runtime
//! serializes operations in its IRQ-masked process domain; references survive
//! dup/close and are released automatically when the owning table is dropped.
use crate::linux_fs::{Kind, Node};
use alloc::alloc::{alloc, dealloc};
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering, fence},
};

// app_process's linker keeps hundreds of dependency files open while building
// its load group. Smaller personalities retain the original descriptor bound.
pub const CAPACITY: usize = if cfg!(feature = "framework-runtime") {
    512
} else {
    64
};
pub const EBADF: i64 = 9;
pub const ENOMEM: i64 = 12;
pub const EINVAL: i64 = 22;
pub const EMFILE: i64 = 24;
pub const ESPIPE: i64 = 29;
static LIVE_DESCRIPTIONS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug)]
pub enum Target {
    Null,
    Console,
    Random,
    Node(Node<'static>),
    Snapshot { path: &'static str, inode: u64 },
    UnixDatagram,
    Memory { inode: u64, length: u64 },
}

struct Description {
    strong: AtomicUsize,
    position: AtomicU64,
    flags: AtomicU32,
    target: Target,
    snapshot: Option<Vec<u8>>,
    log_connected: AtomicBool,
    memory: Option<crate::linux_memfd::MemoryFile>,
}
pub struct FileRef(NonNull<Description>);
// Descriptions contain immutable bytes, atomic shared descriptor state, or a
// memory file with serialized metadata access and runtime-owned mapped pages.
unsafe impl Send for FileRef {}
unsafe impl Sync for FileRef {}
impl FileRef {
    pub fn try_new(target: Target, flags: u32) -> Result<Self, i64> {
        Self::create(target, flags, None, None)
    }
    pub fn try_snapshot(
        path: &'static str,
        inode: u64,
        flags: u32,
        bytes: Vec<u8>,
    ) -> Result<Self, i64> {
        Self::create(Target::Snapshot { path, inode }, flags, Some(bytes), None)
    }
    pub fn try_memory(name: &[u8], flags: u32) -> Result<Self, i64> {
        let memory = crate::linux_memfd::MemoryFile::new(name, flags)?;
        Self::create(
            Target::Memory {
                inode: memory.inode(),
                length: 0,
            },
            2 | 0x20000,
            None,
            Some(memory),
        )
    }
    fn create(
        target: Target,
        flags: u32,
        snapshot: Option<Vec<u8>>,
        memory: Option<crate::linux_memfd::MemoryFile>,
    ) -> Result<Self, i64> {
        let pointer =
            NonNull::new(unsafe { alloc(Layout::new::<Description>()) }.cast::<Description>())
                .ok_or(ENOMEM)?;
        unsafe {
            pointer.as_ptr().write(Description {
                strong: AtomicUsize::new(1),
                position: AtomicU64::new(0),
                flags: AtomicU32::new(flags),
                target,
                snapshot,
                log_connected: AtomicBool::new(false),
                memory,
            });
        }
        LIVE_DESCRIPTIONS.fetch_add(1, Ordering::Relaxed);
        Ok(Self(pointer))
    }
    fn inner(&self) -> &Description {
        unsafe { self.0.as_ref() }
    }
    pub fn target(&self) -> Target {
        match self.memory() {
            Some(memory) => Target::Memory {
                inode: memory.inode(),
                length: memory.len(),
            },
            None => self.inner().target,
        }
    }
    pub fn memory(&self) -> Option<&crate::linux_memfd::MemoryFile> {
        self.inner().memory.as_ref()
    }
    pub fn snapshot(&self) -> Option<&[u8]> {
        self.inner().snapshot.as_deref()
    }
    pub fn connect_log(&self) -> Result<(), i64> {
        if !matches!(self.target(), Target::UnixDatagram) {
            return Err(88);
        }
        self.inner().log_connected.store(true, Ordering::Release);
        Ok(())
    }
    pub fn log_connected(&self) -> bool {
        self.inner().log_connected.load(Ordering::Acquire)
    }
    pub fn flags(&self) -> u32 {
        self.inner().flags.load(Ordering::Acquire)
    }
    pub fn position(&self) -> u64 {
        self.inner().position.load(Ordering::Acquire)
    }
    pub fn set_position(&self, value: u64) {
        self.inner().position.store(value, Ordering::Release);
    }
    pub fn set_status_flags(&self, requested: u32) -> Result<(), i64> {
        // APPEND/NONBLOCK are shared open-description status, not fd flags.
        if requested & !(self.flags() | 3 | 0x400 | 0x800 | 0x20000) != 0 {
            return Err(EINVAL);
        }
        let previous = self.flags();
        self.inner().flags.store(
            (previous & !(0x400 | 0x800)) | (requested & (0x400 | 0x800)),
            Ordering::Release,
        );
        Ok(())
    }
    pub fn readable(&self) -> bool {
        self.flags() & 3 != 1 && self.flags() & 0x200000 == 0
    }
    pub fn writable(&self) -> bool {
        matches!(self.flags() & 3, 1 | 2) && self.flags() & 0x200000 == 0
    }
    pub fn seek(&self, offset: i64, whence: u64) -> Result<u64, i64> {
        if self.flags() & 0x200000 != 0 {
            return Err(EBADF);
        }
        let end = match self.target() {
            Target::Node(node) if node.kind == Kind::File => Some(node.data.len() as u64),
            Target::Node(_) => return Err(EBADF),
            Target::Snapshot { .. } => None, // proc seq-file: SEEK_SET/CUR only.
            Target::Memory { length, .. } => Some(length),
            _ => return Err(ESPIPE),
        };
        let base = match whence {
            0 => 0,
            1 => self.position(),
            2 => end.ok_or(EINVAL)?,
            _ => return Err(EINVAL),
        };
        let position = base
            .checked_add_signed(offset)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or(EINVAL)?;
        self.set_position(position);
        Ok(position)
    }
}
impl Clone for FileRef {
    fn clone(&self) -> Self {
        let old = self.inner().strong.fetch_add(1, Ordering::Relaxed);
        assert!(
            old > 0 && old < isize::MAX as usize,
            "Linux descriptor reference overflow"
        );
        Self(self.0)
    }
}
impl Drop for FileRef {
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            LIVE_DESCRIPTIONS.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                self.0.as_ptr().drop_in_place();
                dealloc(self.0.as_ptr().cast(), Layout::new::<Description>());
            }
        }
    }
}

struct Descriptor {
    file: FileRef,
    close_on_exec: bool,
}
pub struct Table {
    // Process construction runs on a fixed kernel stack. Keep larger linker
    // descriptor tables in fallibly allocated storage, not in copied frames.
    entries: Vec<Option<Descriptor>>,
}
impl Table {
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    pub fn standard() -> Result<Self, i64> {
        let mut result = Self::empty();
        for (target, flags) in [
            (Target::Null, 0),
            (Target::Console, 1),
            (Target::Console, 1),
        ] {
            result.insert(FileRef::try_new(target, flags)?, 0, false)?;
        }
        Ok(result)
    }
    pub fn len(&self) -> usize {
        self.entries.iter().flatten().count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, descriptor: u64) -> Result<&Descriptor, i64> {
        self.entries
            .get(usize::try_from(descriptor).map_err(|_| EBADF)?)
            .and_then(Option::as_ref)
            .ok_or(EBADF)
    }
    pub fn file(&self, descriptor: u64) -> Result<FileRef, i64> {
        Ok(self.get(descriptor)?.file.clone())
    }
    pub fn fd_flags(&self, descriptor: u64) -> Result<u64, i64> {
        Ok(u64::from(self.get(descriptor)?.close_on_exec))
    }
    pub fn set_fd_flags(&mut self, descriptor: u64, flags: u64) -> Result<(), i64> {
        self.get(descriptor)?;
        if flags & !1 != 0 {
            return Err(EINVAL);
        }
        self.entries[descriptor as usize]
            .as_mut()
            .unwrap()
            .close_on_exec = flags != 0;
        Ok(())
    }
    pub fn close(&mut self, descriptor: u64) -> Result<(), i64> {
        self.get(descriptor)?;
        self.entries[descriptor as usize] = None;
        Ok(())
    }
    pub fn insert(&mut self, file: FileRef, minimum: u64, close_on_exec: bool) -> Result<u64, i64> {
        let first = usize::try_from(minimum).map_err(|_| EINVAL)?;
        if first >= CAPACITY {
            return Err(EINVAL);
        }
        self.ensure_storage()?;
        let index = self.entries[first..]
            .iter()
            .position(Option::is_none)
            .ok_or(EMFILE)?
            + first;
        self.entries[index] = Some(Descriptor {
            file,
            close_on_exec,
        });
        Ok(index as u64)
    }
    fn ensure_storage(&mut self) -> Result<(), i64> {
        if self.entries.is_empty() {
            self.entries
                .try_reserve_exact(CAPACITY)
                .map_err(|_| ENOMEM)?;
            self.entries.resize_with(CAPACITY, || None);
        }
        Ok(())
    }
    pub fn duplicate(&mut self, old: u64, minimum: u64, close_on_exec: bool) -> Result<u64, i64> {
        self.insert(self.file(old)?, minimum, close_on_exec)
    }
    pub fn duplicate_to(&mut self, old: u64, new: u64, close_on_exec: bool) -> Result<u64, i64> {
        if old == new {
            return Err(EINVAL);
        }
        if new >= CAPACITY as u64 {
            return Err(EBADF);
        }
        let file = self.file(old)?;
        self.entries[new as usize] = Some(Descriptor {
            file,
            close_on_exec,
        });
        Ok(new)
    }
}

pub fn live_descriptions() -> usize {
    LIVE_DESCRIPTIONS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_survive_dup_and_original_close_without_becoming_static() {
        let mut table = Table::empty();
        let opened = FileRef::try_snapshot("proc/self/stat", 10, 0, b"first".to_vec()).unwrap();
        let original = table.insert(opened, 0, false).unwrap();
        let duplicate = table.duplicate(original, 0, true).unwrap();
        table.file(original).unwrap().seek(2, 0).unwrap();
        table.close(original).unwrap();
        let kept = table.file(duplicate).unwrap();
        assert_eq!(kept.snapshot(), Some(b"first".as_slice()));
        assert_eq!(kept.position(), 2);
        assert_eq!(kept.seek(0, 2), Err(EINVAL));
        let fresh = FileRef::try_snapshot("proc/self/stat", 10, 0, b"new".to_vec()).unwrap();
        assert_eq!(fresh.snapshot(), Some(b"new".as_slice()));
        assert_eq!(kept.snapshot(), Some(b"first".as_slice()));
        let socket = FileRef::try_new(Target::UnixDatagram, 2).unwrap();
        let cloned = socket.clone();
        assert!(!cloned.log_connected());
        socket.connect_log().unwrap();
        drop(socket);
        assert!(cloned.log_connected());
        assert_eq!(kept.connect_log(), Err(88));
    }
    fn file() -> FileRef {
        FileRef::try_new(
            Target::Node(Node {
                path: "system/data",
                data: b"abcdef",
                kind: Kind::File,
                mode: 0o444,
                inode: 20,
            }),
            0,
        )
        .unwrap()
    }
    #[test]
    fn dup_shares_offset_but_has_independent_descriptor_flags() {
        let mut table = Table::standard().unwrap();
        let first = table.insert(file(), 0, false).unwrap();
        let duplicate = table.duplicate(first, 0, true).unwrap();
        assert_eq!(table.file(first).unwrap().seek(3, 0), Ok(3));
        assert_eq!(table.file(duplicate).unwrap().position(), 3);
        assert_eq!(table.fd_flags(first), Ok(0));
        assert_eq!(table.fd_flags(duplicate), Ok(1));
        table.close(first).unwrap();
        assert_eq!(table.file(duplicate).unwrap().seek(-1, 1), Ok(2));
        assert_eq!(table.insert(file(), 0, false).unwrap(), first);
        assert_eq!(table.file(first).unwrap().position(), 0);
    }
    #[test]
    fn invalid_dup_does_not_close_the_destination() {
        let mut table = Table::standard().unwrap();
        assert_eq!(table.duplicate_to(999, 1, false), Err(EBADF));
        assert!(table.file(1).unwrap().writable());
        assert_eq!(table.duplicate_to(1, 1, false), Err(EINVAL));
        assert_eq!(table.duplicate_to(1, 2, true), Ok(2));
        assert_eq!(table.fd_flags(2), Ok(1));
    }
    #[test]
    fn independent_opens_seek_past_eof_and_reject_underflow() {
        let first = file();
        let second = file();
        assert_eq!(first.seek(5, 2), Ok(11));
        assert_eq!(second.position(), 0);
        assert_eq!(first.seek(-12, 1), Err(EINVAL));
        assert_eq!(first.position(), 11);
        assert_eq!(first.seek(i64::MAX, 1), Err(EINVAL));
        let console = FileRef::try_new(Target::Console, 1).unwrap();
        assert_eq!(console.seek(0, 0), Err(ESPIPE));
    }
    #[test]
    fn descriptor_limit_and_closed_handles_fail_without_panic() {
        let mut table = Table::empty();
        for index in 0..CAPACITY {
            assert_eq!(table.insert(file(), 0, false), Ok(index as u64));
        }
        assert_eq!(table.insert(file(), 0, false), Err(EMFILE));
        assert_eq!(table.duplicate(0, u64::MAX, false), Err(EINVAL));
        table.close(0).unwrap();
        assert_eq!(table.close(0), Err(EBADF));
        assert!(table.file(0).is_err());
        assert_eq!(table.len(), CAPACITY - 1);
    }
}
