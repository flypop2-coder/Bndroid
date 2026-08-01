//! One-shot publication of the immutable files verified from `/system` at boot.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::bootfs::{BootfsCatalog, BootfsError, validate_path};
use crate::vmo::{Vmo, VmoCreateError};

pub const SYSTEM_FILE_COUNT: usize = 2;
pub const HELLO_PATH: &str = "HELLO.TXT";
pub const BUILD_PATH: &str = "SYSTEM/BUILD.TXT";

const EMPTY: u8 = 0;
const INSTALLING: u8 = 1;
const READY: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallError {
    AlreadyInstalled,
    FileTooLarge,
    OutOfMemory,
    Catalog(BootfsError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenError {
    NotReady,
    InvalidPath,
    NotFound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemFilesSnapshot {
    pub ready: bool,
    pub files: usize,
    pub bytes: usize,
}

struct CatalogSlot {
    state: AtomicU8,
    catalog: UnsafeCell<MaybeUninit<BootfsCatalog<SYSTEM_FILE_COUNT>>>,
}

// `catalog` is written by the unique EMPTY -> INSTALLING owner and is never
// mutated after the READY release-store. Readers acquire READY before taking
// a shared reference, so no reader can race construction.
unsafe impl Sync for CatalogSlot {}

impl CatalogSlot {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            catalog: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    fn install(&self, hello: &[u8], build: &[u8]) -> Result<(), InstallError> {
        // All allocation and validation happens before the publication state
        // changes. A failed attempt therefore leaves the global slot empty.
        let catalog = build_catalog(hello, build)?;
        if self
            .state
            .compare_exchange(EMPTY, INSTALLING, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(InstallError::AlreadyInstalled);
        }
        unsafe { (*self.catalog.get()).write(catalog) };
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    fn catalog(&self) -> Result<&BootfsCatalog<SYSTEM_FILE_COUNT>, OpenError> {
        if self.state.load(Ordering::Acquire) != READY {
            return Err(OpenError::NotReady);
        }
        Ok(unsafe { (&*self.catalog.get()).assume_init_ref() })
    }

    fn open(&self, path: &str) -> Result<(Vmo, usize), OpenError> {
        validate_path(path).map_err(|_| OpenError::InvalidPath)?;
        self.catalog()?.open(path).map_err(|error| match error {
            BootfsError::NotFound => OpenError::NotFound,
            _ => OpenError::InvalidPath,
        })
    }

    fn snapshot(&self) -> SystemFilesSnapshot {
        let Ok(catalog) = self.catalog() else {
            return SystemFilesSnapshot {
                ready: false,
                files: 0,
                bytes: 0,
            };
        };
        let hello = catalog
            .open(HELLO_PATH)
            .unwrap_or_else(|_| panic!("published system catalog lost HELLO.TXT"));
        let build = catalog
            .open(BUILD_PATH)
            .unwrap_or_else(|_| panic!("published system catalog lost BUILD.TXT"));
        SystemFilesSnapshot {
            ready: true,
            files: catalog.len(),
            bytes: hello.1 + build.1,
        }
    }
}

impl Drop for CatalogSlot {
    fn drop(&mut self) {
        if *self.state.get_mut() == READY {
            unsafe { self.catalog.get_mut().assume_init_drop() };
        }
    }
}

static SYSTEM_FILES: CatalogSlot = CatalogSlot::new();

pub fn install(hello: &[u8], build: &[u8]) -> Result<(), InstallError> {
    SYSTEM_FILES.install(hello, build)
}

pub fn open(path: &str) -> Result<(Vmo, usize), OpenError> {
    SYSTEM_FILES.open(path)
}

pub fn snapshot() -> SystemFilesSnapshot {
    SYSTEM_FILES.snapshot()
}

fn build_catalog(
    hello: &[u8],
    build: &[u8],
) -> Result<BootfsCatalog<SYSTEM_FILE_COUNT>, InstallError> {
    let hello = Vmo::try_from_slice(hello).map_err(vmo_install_error)?;
    let build = Vmo::try_from_slice(build).map_err(vmo_install_error)?;
    BootfsCatalog::try_from_entries([(HELLO_PATH, hello), (BUILD_PATH, build)])
        .map_err(InstallError::Catalog)
}

const fn vmo_install_error(error: VmoCreateError) -> InstallError {
    match error {
        VmoCreateError::TooLarge => InstallError::FileTooLarge,
        VmoCreateError::OutOfMemory => InstallError::OutOfMemory,
    }
}

#[cfg(test)]
mod tests {
    use super::{BUILD_PATH, CatalogSlot, HELLO_PATH, InstallError, OpenError, SYSTEM_FILE_COUNT};
    use crate::vmo::VMO_MAX_BYTES;

    #[test]
    fn publishes_exact_files_once_and_shares_vmo_identity() {
        let slot = CatalogSlot::new();
        assert_eq!(slot.open(HELLO_PATH).unwrap_err(), OpenError::NotReady);
        slot.install(b"hello\n", b"build=m24\n").unwrap();
        let first = slot.open(HELLO_PATH).unwrap();
        let second = slot.open(HELLO_PATH).unwrap();
        assert_eq!(first.1, 6);
        assert_eq!(first.0.read_range(0, first.1).unwrap(), b"hello\n");
        assert!(first.0.same_vmo(&second.0));
        assert_eq!(slot.open(BUILD_PATH).unwrap().1, 10);
        assert_eq!(slot.snapshot().files, SYSTEM_FILE_COUNT);
        assert_eq!(
            slot.install(b"replacement", b"replacement"),
            Err(InstallError::AlreadyInstalled)
        );
    }

    #[test]
    fn rejects_noncanonical_and_unknown_paths_distinctly() {
        let slot = CatalogSlot::new();
        slot.install(b"hello", b"build").unwrap();
        for path in ["", "/HELLO.TXT", "../HELLO.TXT", "SYSTEM//BUILD.TXT"] {
            assert_eq!(slot.open(path).unwrap_err(), OpenError::InvalidPath);
        }
        assert_eq!(slot.open("MISSING.TXT").unwrap_err(), OpenError::NotFound);
    }

    #[test]
    fn failed_construction_does_not_poison_publication() {
        let slot = CatalogSlot::new();
        let oversized = [0_u8; VMO_MAX_BYTES + 1];
        assert_eq!(
            slot.install(&oversized, b"build"),
            Err(InstallError::FileTooLarge)
        );
        slot.install(b"hello", b"build").unwrap();
        assert!(slot.snapshot().ready);
    }
}
