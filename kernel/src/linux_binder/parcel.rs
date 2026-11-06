//! A transaction owns captured open descriptions until the receiving process
//! installs them. Never retain a Binder device inside its own router: that
//! would permit ownership cycles and recursive close under the router lock.
use super::wire::*;
use crate::linux_fd::{FileRef, Table};
use alloc::vec::Vec;

pub struct FileTransfer {
    offset: usize,
    file: FileRef,
}
impl FileTransfer {
    pub fn new(offset: usize, file: FileRef) -> Result<Self, i64> {
        if file.binder().is_some() {
            return Err(95);
        }
        Ok(Self { offset, file })
    }
    pub fn file(&self) -> &FileRef {
        &self.file
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
}
pub struct Parcel {
    pub bytes: Vec<u8>,
    pub files: Vec<FileTransfer>,
}
impl From<Vec<u8>> for Parcel {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            files: Vec::new(),
        }
    }
}
impl Parcel {
    pub fn capture(tx: Transaction, bytes: Vec<u8>, table: &Table) -> Result<Self, i64> {
        let offsets = tx.object_offsets(&bytes)?;
        let mut files = Vec::new();
        files.try_reserve_exact(offsets.len()).map_err(|_| 12)?;
        for at in offsets {
            if u32_at(&bytes, at) == TYPE_FD {
                files.push(FileTransfer::new(
                    at + 8,
                    table.file(u32_at(&bytes, at + 8) as u64)?,
                )?);
            }
        }
        Ok(Self { bytes, files })
    }
}
