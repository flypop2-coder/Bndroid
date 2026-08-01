//! Minimal single-mount, read-only virtual filesystem facade.

use crate::block::BlockReader;
use crate::fat16::{Fat16, Fat16Error, Fat16Metadata};

pub const SYSTEM_MOUNT_PATH: &str = "/system";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VfsError {
    InvalidMountPath,
    FileSystem(Fat16Error),
    ReadOnly,
}

impl VfsError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidMountPath => "path is outside the /system mount",
            Self::FileSystem(error) => error.as_str(),
            Self::ReadOnly => "VFS mount is read-only",
        }
    }
}

impl From<Fat16Error> for VfsError {
    fn from(error: Fat16Error) -> Self {
        Self::FileSystem(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vfs {
    system: Fat16,
}

impl Vfs {
    pub const fn mount_system(system: Fat16) -> Self {
        Self { system }
    }

    pub const fn is_read_only(&self) -> bool {
        true
    }

    pub const fn filesystem(&self) -> &Fat16 {
        &self.system
    }

    pub fn metadata<R: BlockReader>(
        &self,
        reader: &mut R,
        path: &str,
    ) -> Result<Fat16Metadata, VfsError> {
        Ok(self.system.lookup(reader, system_relative(path)?)?)
    }

    pub fn read<R: BlockReader>(
        &self,
        reader: &mut R,
        path: &str,
        output: &mut [u8],
    ) -> Result<usize, VfsError> {
        Ok(self.system.read(reader, system_relative(path)?, output)?)
    }

    pub const fn write(&self, _path: &str, _input: &[u8]) -> Result<usize, VfsError> {
        Err(VfsError::ReadOnly)
    }
}

fn system_relative(path: &str) -> Result<&str, VfsError> {
    if path == SYSTEM_MOUNT_PATH {
        return Ok("/");
    }
    match path.strip_prefix(SYSTEM_MOUNT_PATH) {
        Some(relative) if relative.starts_with('/') && relative.len() > 1 => Ok(relative),
        _ => Err(VfsError::InvalidMountPath),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_the_exact_system_mount() {
        assert_eq!(system_relative("/system"), Ok("/"));
        assert_eq!(system_relative("/system/HELLO.TXT"), Ok("/HELLO.TXT"));
        assert_eq!(
            system_relative("/systematic/HELLO.TXT"),
            Err(VfsError::InvalidMountPath)
        );
        assert_eq!(
            system_relative("/HELLO.TXT"),
            Err(VfsError::InvalidMountPath)
        );
    }
}
