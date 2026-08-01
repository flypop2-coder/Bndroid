//! Explicit, bounded QEMU `fw_cfg` package inputs.
//!
//! The optional APK and fixed Uninstall-0 request are discovered together from
//! one directory snapshot. At most one may be present. The selected input is
//! copied once on the boot CPU into kernel-owned read-only staging memory.
//! Discovery grants no EL0 authority; package admission and durable mutation
//! remain separate steps.

use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use bndroid_kernel::{
    fdt::FwCfgMmioRegion,
    fw_cfg::{
        DirectoryError, FILE_DIRECTORY_HEADER_BYTES, FILE_DIRECTORY_SELECTOR, FILE_ENTRY_BYTES,
        PACKAGE_UNINSTALL_REQUEST_BYTES, PackageUninstallRequestError, directory_count,
        find_package_inputs, parse_package_uninstall_request,
    },
};

use crate::driver::fw_cfg::{FwCfgError, FwCfgMmio};

pub const MAX_APK_SOURCE_BYTES: usize = 65_024;
const MAX_FW_CFG_FILES: usize = 64;
const DIRECTORY_BYTES: usize = FILE_DIRECTORY_HEADER_BYTES + MAX_FW_CFG_FILES * FILE_ENTRY_BYTES;

#[repr(C, align(4096))]
struct ApkSourceStorage(UnsafeCell<[u8; MAX_APK_SOURCE_BYTES]>);

// The boot CPU is the only writer. SOURCE_INITIALIZED publishes the immutable
// bytes before any later kernel reader can borrow them.
unsafe impl Sync for ApkSourceStorage {}

#[repr(C, align(256))]
struct UninstallRequestStorage(UnsafeCell<[u8; PACKAGE_UNINSTALL_REQUEST_BYTES]>);

// The boot CPU is the only writer. SOURCE_INITIALIZED publishes these
// immutable bytes together with the optional APK staging buffer.
unsafe impl Sync for UninstallRequestStorage {}

#[repr(C, align(8))]
struct DirectoryStorage(UnsafeCell<[u8; DIRECTORY_BYTES]>);

// This scratch area is used synchronously on the boot CPU before publication.
unsafe impl Sync for DirectoryStorage {}

static APK_SOURCE: ApkSourceStorage = ApkSourceStorage(UnsafeCell::new([0; MAX_APK_SOURCE_BYTES]));
static UNINSTALL_REQUEST: UninstallRequestStorage =
    UninstallRequestStorage(UnsafeCell::new([0; PACKAGE_UNINSTALL_REQUEST_BYTES]));
static DIRECTORY: DirectoryStorage = DirectoryStorage(UnsafeCell::new([0; DIRECTORY_BYTES]));
static SOURCE_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SOURCE_LENGTH: AtomicUsize = AtomicUsize::new(0);
static UNINSTALL_REQUEST_PRESENT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageSourceEvidence {
    pub present: bool,
    pub length: u32,
    pub sha256: [u8; 32],
    pub selector: u16,
    pub uninstall_request_present: bool,
    pub uninstall_request_length: u32,
    pub uninstall_request_sha256: [u8; 32],
    pub uninstall_request_selector: u16,
    pub directory_files: u32,
    pub dma_operations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageSourceError {
    AlreadyInitialized,
    MissingTransport,
    FwCfg(FwCfgError),
    Directory(DirectoryError),
    TooManyFiles,
    ConflictingInputs,
    EmptySource,
    SourceTooLarge,
    InvalidUninstallRequestSize,
    InvalidUninstallRequest(PackageUninstallRequestError),
}

impl PackageSourceError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyInitialized => "APK source was initialized more than once",
            Self::MissingTransport => "APK source profile requires QEMU fw_cfg",
            Self::FwCfg(error) => error.as_str(),
            Self::Directory(error) => error.as_str(),
            Self::TooManyFiles => "fw_cfg file directory exceeds the Install-0 bound",
            Self::ConflictingInputs => {
                "fw_cfg APK and package-uninstall inputs are mutually exclusive"
            }
            Self::EmptySource => "fw_cfg APK source is empty",
            Self::SourceTooLarge => "fw_cfg APK source exceeds the Install-0 bound",
            Self::InvalidUninstallRequestSize => {
                "fw_cfg package-uninstall request is not exactly 256 bytes"
            }
            Self::InvalidUninstallRequest(error) => error.as_str(),
        }
    }
}

/// Discovers both explicit package inputs and copies at most one.
pub fn init(region: Option<FwCfgMmioRegion>) -> Result<PackageSourceEvidence, PackageSourceError> {
    if SOURCE_INITIALIZED.load(Ordering::Acquire) {
        return Err(PackageSourceError::AlreadyInitialized);
    }
    let region = region.ok_or(PackageSourceError::MissingTransport)?;
    // SAFETY: FDT discovery accepted one direct-root coherent fw_cfg MMIO
    // region. Display initialization has completed, so this is the only
    // synchronous owner while it probes and copies the optional source.
    let mut fw_cfg = unsafe { FwCfgMmio::probe(region) }.map_err(PackageSourceError::FwCfg)?;

    let directory = unsafe { &mut *DIRECTORY.0.get() };
    directory.fill(0);
    fw_cfg
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            FILE_DIRECTORY_HEADER_BYTES,
        )
        .map_err(PackageSourceError::FwCfg)?;
    let file_count = directory_count(&directory[..FILE_DIRECTORY_HEADER_BYTES])
        .map_err(PackageSourceError::Directory)?;
    let file_count_usize =
        usize::try_from(file_count).map_err(|_| PackageSourceError::TooManyFiles)?;
    if file_count_usize > MAX_FW_CFG_FILES {
        return Err(PackageSourceError::TooManyFiles);
    }
    let directory_bytes = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            file_count_usize
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(PackageSourceError::TooManyFiles)?,
        )
        .ok_or(PackageSourceError::TooManyFiles)?;
    fw_cfg
        .dma_read(
            FILE_DIRECTORY_SELECTOR,
            directory.as_mut_ptr() as usize,
            directory_bytes,
        )
        .map_err(PackageSourceError::FwCfg)?;
    let inputs = find_package_inputs(&directory[..directory_bytes])
        .map_err(PackageSourceError::Directory)?;
    if inputs.apk.is_some() && inputs.uninstall_request.is_some() {
        return Err(PackageSourceError::ConflictingInputs);
    }

    if let Some(request) = inputs.uninstall_request {
        if request.size != PACKAGE_UNINSTALL_REQUEST_BYTES as u32 {
            return Err(PackageSourceError::InvalidUninstallRequestSize);
        }
        let bytes = unsafe { &mut *UNINSTALL_REQUEST.0.get() };
        bytes.fill(0);
        fw_cfg
            .dma_read(
                request.selector,
                bytes.as_mut_ptr() as usize,
                PACKAGE_UNINSTALL_REQUEST_BYTES,
            )
            .map_err(PackageSourceError::FwCfg)?;
        parse_package_uninstall_request(bytes)
            .map_err(PackageSourceError::InvalidUninstallRequest)?;
        let sha256 = bndr_sm::verified_manifest::sha256(bytes);
        SOURCE_LENGTH.store(0, Ordering::Relaxed);
        UNINSTALL_REQUEST_PRESENT.store(true, Ordering::Relaxed);
        SOURCE_INITIALIZED.store(true, Ordering::Release);
        return Ok(PackageSourceEvidence {
            present: false,
            length: 0,
            sha256: [0; 32],
            selector: 0,
            uninstall_request_present: true,
            uninstall_request_length: PACKAGE_UNINSTALL_REQUEST_BYTES as u32,
            uninstall_request_sha256: sha256,
            uninstall_request_selector: request.selector,
            directory_files: file_count,
            dma_operations: fw_cfg.dma_operations(),
        });
    }

    let Some(source) = inputs.apk else {
        SOURCE_LENGTH.store(0, Ordering::Relaxed);
        UNINSTALL_REQUEST_PRESENT.store(false, Ordering::Relaxed);
        SOURCE_INITIALIZED.store(true, Ordering::Release);
        return Ok(PackageSourceEvidence {
            present: false,
            length: 0,
            sha256: [0; 32],
            selector: 0,
            uninstall_request_present: false,
            uninstall_request_length: 0,
            uninstall_request_sha256: [0; 32],
            uninstall_request_selector: 0,
            directory_files: file_count,
            dma_operations: fw_cfg.dma_operations(),
        });
    };
    let length = usize::try_from(source.size).map_err(|_| PackageSourceError::SourceTooLarge)?;
    if length == 0 {
        return Err(PackageSourceError::EmptySource);
    }
    if length > MAX_APK_SOURCE_BYTES {
        return Err(PackageSourceError::SourceTooLarge);
    }

    let bytes = unsafe { &mut *APK_SOURCE.0.get() };
    bytes.fill(0);
    fw_cfg
        .dma_read(source.selector, bytes.as_mut_ptr() as usize, length)
        .map_err(PackageSourceError::FwCfg)?;
    let sha256 = bndr_sm::verified_manifest::sha256(&bytes[..length]);
    SOURCE_LENGTH.store(length, Ordering::Relaxed);
    UNINSTALL_REQUEST_PRESENT.store(false, Ordering::Relaxed);
    SOURCE_INITIALIZED.store(true, Ordering::Release);
    Ok(PackageSourceEvidence {
        present: true,
        length: source.size,
        sha256,
        selector: source.selector,
        uninstall_request_present: false,
        uninstall_request_length: 0,
        uninstall_request_sha256: [0; 32],
        uninstall_request_selector: 0,
        directory_files: file_count,
        dma_operations: fw_cfg.dma_operations(),
    })
}

/// Returns the immutable boot-local source after successful initialization.
pub fn bytes() -> Option<&'static [u8]> {
    if !SOURCE_INITIALIZED.load(Ordering::Acquire) {
        return None;
    }
    let length = SOURCE_LENGTH.load(Ordering::Relaxed);
    if length == 0 {
        return None;
    }
    // SAFETY: `init` is the only writer, it stores the final length before the
    // Release publication, and the buffer is never mutated afterward.
    Some(unsafe { &(&*APK_SOURCE.0.get())[..length] })
}

/// Returns the immutable canonical Uninstall-0 request after initialization.
pub fn uninstall_request_bytes() -> Option<&'static [u8; PACKAGE_UNINSTALL_REQUEST_BYTES]> {
    if !SOURCE_INITIALIZED.load(Ordering::Acquire)
        || !UNINSTALL_REQUEST_PRESENT.load(Ordering::Relaxed)
    {
        return None;
    }
    // SAFETY: `init` is the only writer. It validates all 256 bytes before the
    // Release publication and never mutates this buffer afterward.
    Some(unsafe { &*UNINSTALL_REQUEST.0.get() })
}
