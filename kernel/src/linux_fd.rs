//! POSIX descriptor lifetime and shared open-description offsets. The runtime
//! serializes operations in its IRQ-masked process domain; references survive
//! dup/close and are released automatically when the owning table is dropped.
use crate::linux_fs::{Kind, Node};
use alloc::alloc::{alloc, dealloc};
use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering, fence},
};

pub const CAPACITY: usize = 64;
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
}

struct Description {
    strong: AtomicUsize,
    position: AtomicU64,
    flags: AtomicU32,
    target: Target,
}
pub struct FileRef(NonNull<Description>);
// Descriptions contain only immutable static bytes and atomic shared state.
unsafe impl Send for FileRef {}
unsafe impl Sync for FileRef {}
impl FileRef {
    pub fn try_new(target: Target, flags: u32) -> Result<Self, i64> {
        let pointer =
            NonNull::new(unsafe { alloc(Layout::new::<Description>()) }.cast::<Description>())
                .ok_or(ENOMEM)?;
        unsafe {
            pointer.as_ptr().write(Description {
                strong: AtomicUsize::new(1),
                position: AtomicU64::new(0),
                flags: AtomicU32::new(flags),
                target,
            });
        }
        LIVE_DESCRIPTIONS.fetch_add(1, Ordering::Relaxed);
        Ok(Self(pointer))
    }
    fn inner(&self) -> &Description {
        unsafe { self.0.as_ref() }
    }
    pub fn target(&self) -> Target {
        self.inner().target
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
        let Target::Node(node) = self.target() else {
            return Err(ESPIPE);
        };
        if node.kind != Kind::File || self.flags() & 0x200000 != 0 {
            return Err(EBADF);
        }
        let base = match whence {
            0 => 0,
            1 => self.position(),
            2 => node.data.len() as u64,
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
    entries: [Option<Descriptor>; CAPACITY],
}
impl Table {
    pub const fn empty() -> Self {
        Self {
            entries: [const { None }; CAPACITY],
        }
    }
    pub fn standard() -> Result<Self, i64> {
        let mut result = Self::empty();
        for (index, (target, flags)) in [
            (Target::Null, 0),
            (Target::Console, 1),
            (Target::Console, 1),
        ]
        .into_iter()
        .enumerate()
        {
            result.entries[index] = Some(Descriptor {
                file: FileRef::try_new(target, flags)?,
                close_on_exec: false,
            });
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
