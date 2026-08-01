#![no_std]
#![deny(unsafe_code)]

//! Allocation-free, crash-safe storage for one installed APK.
//!
//! The format owns exactly 512 512-byte sectors at a caller-supplied base LBA.
//! An immutable superblock describes the format. Two registry records select
//! between two complete blob slots. Installation writes and flushes an inactive
//! blob before publishing its matching inactive registry, so recovery exposes
//! either the previous complete package or the new complete package.

use bndr_sm::verified_manifest::{Sha256, sha256};

#[cfg(feature = "multi-package-store1")]
mod multi;
#[cfg(feature = "multi-package-store1")]
pub use multi::{
    MULTI_PACKAGE_CAPACITY, MULTI_PACKAGE_VOLUME_SECTORS, MultiFormatEvidence,
    MultiInstalledPackage, MultiPackageCatalog, MultiPackageError, ensure_multi_formatted,
    inspect_multi, install_multi, read_multi_blob, recover_multi, uninstall_multi,
};

pub const SECTOR_SIZE: usize = 512;
pub type Sector = [u8; SECTOR_SIZE];

pub const VOLUME_SECTORS: u64 = 512;
pub const SUPERBLOCK_RELATIVE_LBA: u64 = 0;
pub const REGISTRY_RELATIVE_LBAS: [u64; 2] = [1, 2];
pub const BLOB_RELATIVE_LBAS: [u64; 2] = [16, 144];
pub const BLOB_SECTORS: u64 = 128;
pub const BLOB_PAYLOAD_SECTORS: u64 = BLOB_SECTORS - 1;
pub const MAX_APK_BYTES: usize = BLOB_PAYLOAD_SECTORS as usize * SECTOR_SIZE;
pub const MAX_PACKAGE_NAME_BYTES: usize = 96;
pub const MAX_ACTIVITY_NAME_BYTES: usize = 128;

/// On-disk namespace for package-store format version 1.
pub const FORMAT_EPOCH: [u8; 16] = [
    0xc7, 0x35, 0x42, 0x91, 0x6b, 0xec, 0x4a, 0xdd, 0x95, 0x0e, 0x50, 0xb5, 0xe2, 0xb1, 0x23, 0x01,
];

const FORMAT_VERSION: u32 = 1;
const SUPERBLOCK_MAGIC: [u8; 8] = *b"BNDPKS01";
const REGISTRY_MAGIC: [u8; 8] = *b"BNDPRG01";
const TOMBSTONE_MAGIC: [u8; 8] = *b"BNDPRM01";
const BLOB_MAGIC: [u8; 8] = *b"BNDPBL01";
const COMMITTED_FLAG: u32 = 1;
const RECORD_CRC_OFFSET: usize = 508;

const RECORD_GENERATION_OFFSET: usize = 32;
const RECORD_TRANSACTION_OFFSET: usize = 40;
const RECORD_VERSION_CODE_OFFSET: usize = 48;
const RECORD_APK_LENGTH_OFFSET: usize = 56;
const RECORD_PROFILE_OFFSET: usize = 60;
const RECORD_SLOT_OFFSET: usize = 62;
const RECORD_PACKAGE_LENGTH_OFFSET: usize = 63;
const RECORD_ACTIVITY_LENGTH_OFFSET: usize = 64;
const RECORD_FIRST_RESERVED_RANGE: core::ops::Range<usize> = 65..68;
const RECORD_APK_DIGEST_RANGE: core::ops::Range<usize> = 68..100;
const RECORD_SIGNER_DIGEST_RANGE: core::ops::Range<usize> = 100..132;
const RECORD_BLOB_HEADER_DIGEST_RANGE: core::ops::Range<usize> = 132..164;
const RECORD_PACKAGE_RANGE: core::ops::Range<usize> = 164..260;
const RECORD_ACTIVITY_RANGE: core::ops::Range<usize> = 260..388;
const RECORD_FINAL_RESERVED_RANGE: core::ops::Range<usize> = 388..RECORD_CRC_OFFSET;

const TOMBSTONE_GENERATION_OFFSET: usize = 32;
const TOMBSTONE_OPERATION_OFFSET: usize = 40;
const TOMBSTONE_LAST_GENERATION_OFFSET: usize = 48;
const TOMBSTONE_VERSION_CODE_OFFSET: usize = 56;
const TOMBSTONE_APK_LENGTH_OFFSET: usize = 64;
const TOMBSTONE_DISPOSITION_OFFSET: usize = 68;
const TOMBSTONE_LAST_SLOT_OFFSET: usize = 70;
const TOMBSTONE_PACKAGE_LENGTH_OFFSET: usize = 71;
const TOMBSTONE_APK_DIGEST_RANGE: core::ops::Range<usize> = 72..104;
const TOMBSTONE_SIGNER_DIGEST_RANGE: core::ops::Range<usize> = 104..136;
const TOMBSTONE_PACKAGE_RANGE: core::ops::Range<usize> = 136..232;
const TOMBSTONE_ACTIVITY_RANGE: core::ops::Range<usize> = 232..360;
const TOMBSTONE_INSTALL_TRANSACTION_OFFSET: usize = 360;
const TOMBSTONE_PROFILE_OFFSET: usize = 368;
const TOMBSTONE_ACTIVITY_LENGTH_OFFSET: usize = 370;
const TOMBSTONE_RESERVED_RANGE: core::ops::Range<usize> = 371..RECORD_CRC_OFFSET;

const _: () = assert!(MAX_APK_BYTES == 65_024);
const _: () = assert!(BLOB_RELATIVE_LBAS[0] + BLOB_SECTORS <= BLOB_RELATIVE_LBAS[1]);
const _: () = assert!(BLOB_RELATIVE_LBAS[1] + BLOB_SECTORS <= VOLUME_SECTORS);

/// Synchronous sector I/O used by the durable package transaction.
///
/// A successful flush makes all earlier successful writes durable. `Device`
/// means the requested operation definitely did not complete.
/// `OutcomeUnknown` means the caller must recover before deciding whether a
/// commit became durable. Implementations must reject LBAs outside their
/// device with `OutOfBounds`.
pub trait SectorIo {
    fn sector_count(&self) -> u64;
    fn read(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError>;
    fn write(&mut self, lba: u64, input: &Sector) -> Result<(), IoError>;
    fn flush(&mut self) -> Result<(), IoError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoError {
    OutOfBounds,
    Device,
    RequiresReset,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    TransactionIdZero,
    UninstallOperationIdZero,
    VersionCodeZero,
    EmptyPackage,
    PackageTooLong,
    EmptyActivity,
    ActivityTooLong,
    NonAsciiPackage,
    NonAsciiActivity,
    ZeroSignerDigest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Corruption {
    BadSuperblock,
    NoValidRegistry,
    AmbiguousGeneration,
    BadRegistry,
    BadBlobHeader,
    BlobDigestMismatch,
    BlobPaddingNonZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    VolumeBounds,
    Unformatted,
    AlreadyFormatted,
    NotVirgin,
    Io(IoError),
    OutcomeUnknown,
    InvalidMetadata(MetadataError),
    EmptyApk,
    ApkTooLarge,
    ApkDigestMismatch,
    PackageChanged,
    SignerChanged,
    VersionNotIncreasing,
    TransactionConflict,
    GenerationExhausted,
    NotInstalled,
    StalePackage,
    BufferTooSmall { needed: usize },
    Corrupt(Corruption),
    VerificationFailed,
}

impl Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VolumeBounds => "package-store volume exceeds the block device",
            Self::Unformatted => "package-store volume is unformatted",
            Self::AlreadyFormatted => "package-store volume is already formatted",
            Self::NotVirgin => "package-store volume is not all zero",
            Self::Io(_) => "package-store I/O failed",
            Self::OutcomeUnknown => "package install outcome is unknown; recover before retrying",
            Self::InvalidMetadata(_) => "package metadata is invalid",
            Self::EmptyApk => "APK is empty",
            Self::ApkTooLarge => "APK exceeds the fixed blob payload",
            Self::ApkDigestMismatch => "APK does not match the admitted digest",
            Self::PackageChanged => "single-package store cannot change package identity",
            Self::SignerChanged => "package update signer does not match",
            Self::VersionNotIncreasing => "package update version must strictly increase",
            Self::TransactionConflict => "transaction id was reused with different content",
            Self::GenerationExhausted => "package-store generation is exhausted",
            Self::NotInstalled => "no package is installed",
            Self::StalePackage => "installed-package handle is no longer current",
            Self::BufferTooSmall { .. } => "APK output buffer is too small",
            Self::Corrupt(_) => "package-store metadata is corrupt",
            Self::VerificationFailed => "durable package-store readback failed",
        }
    }
}

/// Compatibility admission profile persisted with an APK.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityProfile {
    Activity0 = 1,
    Resources1 = 2,
}

/// What happened to package-owned mutable data during uninstall.
///
/// Uninstall-0 does not yet manage per-package mutable data, so the only
/// truthful durable disposition is that no such data existed.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataDisposition {
    NoManagedPackageData = 1,
}

impl DataDisposition {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::NoManagedPackageData),
            _ => None,
        }
    }
}

impl CompatibilityProfile {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::Activity0),
            2 => Some(Self::Resources1),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixedAscii<const N: usize> {
    length: u8,
    bytes: [u8; N],
}

impl<const N: usize> FixedAscii<N> {
    const EMPTY: Self = Self {
        length: 0,
        bytes: [0; N],
    };

    fn try_new(
        value: &str,
        empty: MetadataError,
        too_long: MetadataError,
        non_ascii: MetadataError,
    ) -> Result<Self, MetadataError> {
        if value.is_empty() {
            return Err(empty);
        }
        if value.len() > N {
            return Err(too_long);
        }
        if !value
            .as_bytes()
            .iter()
            .all(|byte| matches!(*byte, 0x21..=0x7e))
        {
            return Err(non_ascii);
        }
        let mut result = Self::EMPTY;
        result.length = value.len() as u8;
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(result)
    }

    fn decode(
        length: usize,
        source: &[u8],
        empty: MetadataError,
        too_long: MetadataError,
        non_ascii: MetadataError,
    ) -> Result<Self, MetadataError> {
        if length == 0 {
            return Err(empty);
        }
        if length > N || length > source.len() {
            return Err(too_long);
        }
        if source[length..].iter().any(|byte| *byte != 0)
            || !source[..length]
                .iter()
                .all(|byte| matches!(*byte, 0x21..=0x7e))
        {
            return Err(non_ascii);
        }
        let mut result = Self::EMPTY;
        result.length = length as u8;
        result.bytes[..length].copy_from_slice(&source[..length]);
        Ok(result)
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.length)])
            .expect("validated fixed ASCII")
    }
}

/// Identity and verification metadata admitted before an APK installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstallMetadata {
    transaction_id: u64,
    package: FixedAscii<MAX_PACKAGE_NAME_BYTES>,
    activity: FixedAscii<MAX_ACTIVITY_NAME_BYTES>,
    version_code: u64,
    apk_sha256: [u8; 32],
    signer_cert_sha256: [u8; 32],
    compatibility_profile: CompatibilityProfile,
}

impl InstallMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        transaction_id: u64,
        package: &str,
        activity: &str,
        version_code: u64,
        apk_sha256: [u8; 32],
        signer_cert_sha256: [u8; 32],
        compatibility_profile: CompatibilityProfile,
    ) -> Result<Self, MetadataError> {
        if transaction_id == 0 {
            return Err(MetadataError::TransactionIdZero);
        }
        if version_code == 0 {
            return Err(MetadataError::VersionCodeZero);
        }
        if signer_cert_sha256.iter().all(|byte| *byte == 0) {
            return Err(MetadataError::ZeroSignerDigest);
        }
        Ok(Self {
            transaction_id,
            package: FixedAscii::try_new(
                package,
                MetadataError::EmptyPackage,
                MetadataError::PackageTooLong,
                MetadataError::NonAsciiPackage,
            )?,
            activity: FixedAscii::try_new(
                activity,
                MetadataError::EmptyActivity,
                MetadataError::ActivityTooLong,
                MetadataError::NonAsciiActivity,
            )?,
            version_code,
            apk_sha256,
            signer_cert_sha256,
            compatibility_profile,
        })
    }

    pub const fn transaction_id(&self) -> u64 {
        self.transaction_id
    }

    pub fn package(&self) -> &str {
        self.package.as_str()
    }

    pub fn activity(&self) -> &str {
        self.activity.as_str()
    }

    pub const fn version_code(&self) -> u64 {
        self.version_code
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.apk_sha256
    }

    pub const fn signer_cert_sha256(&self) -> &[u8; 32] {
        &self.signer_cert_sha256
    }

    pub const fn compatibility_profile(&self) -> CompatibilityProfile {
        self.compatibility_profile
    }
}

/// A package whose registry, blob header, payload digest, and padding all
/// validated during recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstalledPackage {
    metadata: InstallMetadata,
    apk_length: u32,
    generation: u64,
    slot: u8,
}

impl InstalledPackage {
    pub const fn metadata(&self) -> &InstallMetadata {
        &self.metadata
    }

    pub const fn apk_length(&self) -> usize {
        self.apk_length as usize
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn slot(&self) -> u8 {
        self.slot
    }
}

/// Durable proof that a previously installed package was removed.
///
/// The record retains the complete identity needed to reject package or
/// signer replacement and version rollback on a later reinstall. APK bytes
/// are deliberately not erased by Uninstall-0, but they are no longer
/// reachable through the installed-package API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemovedPackage {
    operation_id: u64,
    last_metadata: InstallMetadata,
    generation: u64,
    last_generation: u64,
    apk_length: u32,
    last_blob_slot: u8,
    disposition: DataDisposition,
}

impl RemovedPackage {
    pub const fn operation_id(&self) -> u64 {
        self.operation_id
    }

    pub fn package(&self) -> &str {
        self.last_metadata.package()
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn last_generation(&self) -> u64 {
        self.last_generation
    }

    pub const fn expected_generation(&self) -> u64 {
        self.last_generation
    }

    pub const fn last_metadata(&self) -> &InstallMetadata {
        &self.last_metadata
    }

    pub const fn version_code(&self) -> u64 {
        self.last_metadata.version_code
    }

    pub const fn apk_length(&self) -> usize {
        self.apk_length as usize
    }

    pub const fn apk_sha256(&self) -> &[u8; 32] {
        &self.last_metadata.apk_sha256
    }

    pub const fn signer_cert_sha256(&self) -> &[u8; 32] {
        &self.last_metadata.signer_cert_sha256
    }

    pub const fn last_blob_slot(&self) -> u8 {
        self.last_blob_slot
    }

    pub const fn disposition(&self) -> DataDisposition {
        self.disposition
    }

    /// Reconstructs the exact last installed handle for idempotent uninstall
    /// replay after reboot. The handle remains stale for [`read_blob`].
    pub const fn last_installed(&self) -> InstalledPackage {
        InstalledPackage {
            metadata: self.last_metadata,
            apk_length: self.apk_length,
            generation: self.last_generation,
            slot: self.last_blob_slot,
        }
    }
}

/// Complete logical state of the single-package volume.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageState {
    Empty,
    Installed(InstalledPackage),
    Removed(RemovedPackage),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BlobHeader {
    metadata: InstallMetadata,
    apk_length: u32,
    generation: u64,
    slot: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegistryRecord {
    header: BlobHeader,
    blob_header_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveRecord {
    registry_index: u8,
    record: RegistryRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveTombstone {
    registry_index: u8,
    record: RemovedPackage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableRecord {
    Installed(ActiveRecord),
    Removed(ActiveTombstone),
}

impl DurableRecord {
    const fn generation(self) -> u64 {
        match self {
            Self::Installed(active) => active.record.header.generation,
            Self::Removed(active) => active.record.generation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// Keeping the validated record inline is deliberate: the storage core is
// allocation-free, and at most two candidates exist on a bounded stack.
#[allow(clippy::large_enum_variant)]
enum Candidate {
    Empty,
    Invalid,
    Valid(DurableRecord),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Current {
    Empty,
    Installed(ActiveRecord),
    Removed(ActiveTombstone),
}

/// Formats a virgin 512-sector package-store volume.
pub fn format<I: SectorIo>(io: &mut I, volume_start: u64) -> Result<(), Error> {
    check_volume(io, volume_start)?;
    let mut sector = [0_u8; SECTOR_SIZE];
    read_relative(io, volume_start, SUPERBLOCK_RELATIVE_LBA, &mut sector)?;
    if !is_zero(&sector) {
        if parse_superblock(&sector).is_ok() {
            return Err(Error::AlreadyFormatted);
        }
        return Err(Error::NotVirgin);
    }
    for relative in 1..VOLUME_SECTORS {
        read_relative(io, volume_start, relative, &mut sector)?;
        if !is_zero(&sector) {
            return Err(Error::NotVirgin);
        }
    }

    let expected = encode_superblock();
    write_relative(io, volume_start, SUPERBLOCK_RELATIVE_LBA, &expected)?;
    io.flush().map_err(map_outcome_io)?;
    sector.fill(0);
    read_relative(io, volume_start, SUPERBLOCK_RELATIVE_LBA, &mut sector)?;
    if sector != expected || parse_superblock(&sector).is_err() {
        return Err(Error::VerificationFailed);
    }
    Ok(())
}

/// Recovers the newest complete committed package without writing any sector.
pub fn recover<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
) -> Result<Option<InstalledPackage>, Error> {
    match recover_state(io, volume_start)? {
        PackageState::Installed(package) => Ok(Some(package)),
        PackageState::Empty | PackageState::Removed(_) => Ok(None),
    }
}

/// Recovers the complete package-store state without writing any sector.
///
/// Unlike the legacy [`recover`] API, this preserves the distinction between
/// a virgin formatted store and a store whose package was durably removed.
pub fn recover_state<I: SectorIo>(io: &mut I, volume_start: u64) -> Result<PackageState, Error> {
    Ok(match load_current(io, volume_start)? {
        Current::Empty => PackageState::Empty,
        Current::Installed(active) => PackageState::Installed(installed_from_active(active)),
        Current::Removed(active) => PackageState::Removed(active.record),
    })
}

/// Atomically installs or updates the one package owned by this volume.
///
/// A retry with the same transaction id and exact content is idempotent. An
/// update must preserve package and signer identity and strictly increase
/// `version_code`.
pub fn install<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    metadata: InstallMetadata,
    apk: &[u8],
) -> Result<InstalledPackage, Error> {
    if apk.is_empty() {
        return Err(Error::EmptyApk);
    }
    if apk.len() > MAX_APK_BYTES {
        return Err(Error::ApkTooLarge);
    }
    if sha256(apk) != metadata.apk_sha256 {
        return Err(Error::ApkDigestMismatch);
    }

    let current = load_current(io, volume_start)?;
    match current {
        Current::Installed(active) => {
            let installed = installed_from_active(active);
            if metadata.transaction_id == installed.metadata.transaction_id {
                if metadata == installed.metadata && apk.len() == installed.apk_length() {
                    return Ok(installed);
                }
                return Err(Error::TransactionConflict);
            }
            if metadata.package != installed.metadata.package {
                return Err(Error::PackageChanged);
            }
            if metadata.signer_cert_sha256 != installed.metadata.signer_cert_sha256 {
                return Err(Error::SignerChanged);
            }
            if metadata.version_code <= installed.metadata.version_code {
                return Err(Error::VersionNotIncreasing);
            }
        }
        Current::Removed(active) => {
            let removed = active.record;
            if metadata.package != removed.last_metadata.package {
                return Err(Error::PackageChanged);
            }
            if metadata.signer_cert_sha256 != removed.last_metadata.signer_cert_sha256 {
                return Err(Error::SignerChanged);
            }
            if metadata.version_code < removed.last_metadata.version_code
                || (metadata.version_code == removed.last_metadata.version_code
                    && metadata.apk_sha256 != removed.last_metadata.apk_sha256)
            {
                return Err(Error::VersionNotIncreasing);
            }
        }
        Current::Empty => {}
    }

    let (generation, target) = match current {
        Current::Installed(active) => (
            active
                .record
                .header
                .generation
                .checked_add(1)
                .ok_or(Error::GenerationExhausted)?,
            1_u8 - active.record.header.slot,
        ),
        Current::Removed(active) => (
            active
                .record
                .generation
                .checked_add(1)
                .ok_or(Error::GenerationExhausted)?,
            1_u8 - active.record.last_blob_slot,
        ),
        Current::Empty => (1, 0),
    };
    let header = BlobHeader {
        metadata,
        apk_length: apk.len() as u32,
        generation,
        slot: target,
    };
    write_inactive_blob(io, volume_start, header, apk)?;
    io.flush().map_err(map_io)?;

    let (verified_header, header_sector) =
        read_blob_header(io, volume_start, target).map_err(|error| match error {
            Error::Corrupt(_) => Error::VerificationFailed,
            other => other,
        })?;
    if verified_header != header {
        return Err(Error::VerificationFailed);
    }
    validate_blob_payload(io, volume_start, &verified_header, None).map_err(
        |error| match error {
            Error::Corrupt(_) => Error::VerificationFailed,
            other => other,
        },
    )?;

    let registry = RegistryRecord {
        header,
        blob_header_sha256: sha256(&header_sector),
    };
    let registry_sector = encode_registry(&registry);
    write_relative(
        io,
        volume_start,
        REGISTRY_RELATIVE_LBAS[usize::from(target)],
        &registry_sector,
    )?;
    if io.flush().is_err() {
        return Err(Error::OutcomeUnknown);
    }

    let final_record = match read_candidate(io, volume_start, target) {
        Ok(Candidate::Valid(DurableRecord::Installed(active))) if active.record == registry => {
            active
        }
        Ok(_) | Err(_) => return Err(Error::OutcomeUnknown),
    };
    Ok(installed_from_active(final_record))
}

/// Atomically removes the currently installed package.
///
/// The first durable write publishes a higher-generation tombstone in the
/// inactive registry. Before success is returned, the exact same logical
/// tombstone is mirrored into the other registry. Retrying the same operation
/// against a complete pair performs no writes; retrying after a crash between
/// the two commits repairs the missing mirror.
pub fn uninstall<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    operation_id: u64,
    expected: &InstalledPackage,
    disposition: DataDisposition,
) -> Result<RemovedPackage, Error> {
    if operation_id == 0 {
        return Err(Error::InvalidMetadata(
            MetadataError::UninstallOperationIdZero,
        ));
    }

    let candidates = load_candidates(io, volume_start)?;
    match select_current(candidates[0], candidates[1])? {
        Current::Empty => Err(Error::NotInstalled),
        Current::Installed(active) => {
            let installed = installed_from_active(active);
            if installed != *expected {
                return Err(Error::StalePackage);
            }
            let generation = installed
                .generation
                .checked_add(1)
                .ok_or(Error::GenerationExhausted)?;
            let removed = RemovedPackage {
                operation_id,
                last_metadata: installed.metadata,
                generation,
                last_generation: installed.generation,
                apk_length: installed.apk_length,
                last_blob_slot: installed.slot,
                disposition,
            };
            let inactive = 1_u8 - active.registry_index;
            commit_tombstone(io, volume_start, inactive, &removed)?;
            commit_tombstone(io, volume_start, active.registry_index, &removed)?;
            Ok(removed)
        }
        Current::Removed(active) => {
            let removed = active.record;
            if operation_id != removed.operation_id {
                return Err(Error::NotInstalled);
            }
            if !removed_matches_expected(&removed, expected) || disposition != removed.disposition {
                return Err(Error::TransactionConflict);
            }
            if mirrored_tombstone_pair(candidates, &removed) {
                return Ok(removed);
            }
            commit_tombstone(io, volume_start, 1_u8 - active.registry_index, &removed)?;
            Ok(removed)
        }
    }
}

/// Reads the current APK after revalidating its committed registry and blob.
///
/// The supplied handle must still identify the newest recovered package.
pub fn read_blob<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    package: &InstalledPackage,
    output: &mut [u8],
) -> Result<usize, Error> {
    let active = match load_current(io, volume_start)? {
        Current::Installed(active) => active,
        Current::Empty => return Err(Error::NotInstalled),
        Current::Removed(_) => return Err(Error::StalePackage),
    };
    let current = installed_from_active(active);
    if current != *package {
        return Err(Error::StalePackage);
    }
    let needed = current.apk_length();
    if output.len() < needed {
        return Err(Error::BufferTooSmall { needed });
    }
    validate_blob_payload(
        io,
        volume_start,
        &active.record.header,
        Some(&mut output[..needed]),
    )?;
    Ok(needed)
}

fn load_candidates<I: SectorIo>(io: &mut I, volume_start: u64) -> Result<[Candidate; 2], Error> {
    check_volume(io, volume_start)?;
    let mut superblock = [0_u8; SECTOR_SIZE];
    read_relative(io, volume_start, SUPERBLOCK_RELATIVE_LBA, &mut superblock)?;
    if is_zero(&superblock) {
        return Err(Error::Unformatted);
    }
    parse_superblock(&superblock)?;

    Ok([
        read_candidate(io, volume_start, 0)?,
        read_candidate(io, volume_start, 1)?,
    ])
}

fn load_current<I: SectorIo>(io: &mut I, volume_start: u64) -> Result<Current, Error> {
    let candidates = load_candidates(io, volume_start)?;
    select_current(candidates[0], candidates[1])
}

fn select_current(first: Candidate, second: Candidate) -> Result<Current, Error> {
    match (first, second) {
        (Candidate::Empty, Candidate::Empty) => Ok(Current::Empty),
        (Candidate::Valid(active), Candidate::Empty | Candidate::Invalid)
        | (Candidate::Empty | Candidate::Invalid, Candidate::Valid(active)) => {
            Ok(current_from_durable(active))
        }
        (Candidate::Valid(first), Candidate::Valid(second)) => {
            if first.generation() == second.generation() {
                if let (DurableRecord::Removed(first), DurableRecord::Removed(second)) =
                    (first, second)
                    && first.record == second.record
                {
                    return Ok(Current::Removed(first));
                }
                return Err(Error::Corrupt(Corruption::AmbiguousGeneration));
            }
            if first.generation() > second.generation() {
                Ok(current_from_durable(first))
            } else {
                Ok(current_from_durable(second))
            }
        }
        _ => Err(Error::Corrupt(Corruption::NoValidRegistry)),
    }
}

const fn current_from_durable(record: DurableRecord) -> Current {
    match record {
        DurableRecord::Installed(active) => Current::Installed(active),
        DurableRecord::Removed(active) => Current::Removed(active),
    }
}

fn read_candidate<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    registry_index: u8,
) -> Result<Candidate, Error> {
    let mut sector = [0_u8; SECTOR_SIZE];
    read_relative(
        io,
        volume_start,
        REGISTRY_RELATIVE_LBAS[usize::from(registry_index)],
        &mut sector,
    )?;
    if is_zero(&sector) {
        return Ok(Candidate::Empty);
    }
    if sector[..8] == TOMBSTONE_MAGIC {
        let record = match parse_tombstone(&sector) {
            Ok(record) => record,
            Err(_) => return Ok(Candidate::Invalid),
        };
        return Ok(Candidate::Valid(DurableRecord::Removed(ActiveTombstone {
            registry_index,
            record,
        })));
    }
    let record = match parse_registry(&sector, registry_index) {
        Ok(record) => record,
        Err(_) => return Ok(Candidate::Invalid),
    };
    let (header, header_sector) = match read_blob_header(io, volume_start, registry_index) {
        Ok(header) => header,
        Err(error @ (Error::Io(_) | Error::OutcomeUnknown)) => return Err(error),
        Err(_) => return Ok(Candidate::Invalid),
    };
    if header != record.header || sha256(&header_sector) != record.blob_header_sha256 {
        return Ok(Candidate::Invalid);
    }
    match validate_blob_payload(io, volume_start, &header, None) {
        Ok(()) => Ok(Candidate::Valid(DurableRecord::Installed(ActiveRecord {
            registry_index,
            record,
        }))),
        Err(error @ (Error::Io(_) | Error::OutcomeUnknown)) => Err(error),
        Err(_) => Ok(Candidate::Invalid),
    }
}

fn commit_tombstone<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    registry_index: u8,
    removed: &RemovedPackage,
) -> Result<(), Error> {
    let expected = encode_tombstone(removed);
    write_relative(
        io,
        volume_start,
        REGISTRY_RELATIVE_LBAS[usize::from(registry_index)],
        &expected,
    )?;
    if io.flush().is_err() {
        return Err(Error::OutcomeUnknown);
    }
    let mut actual = [0_u8; SECTOR_SIZE];
    if read_relative(
        io,
        volume_start,
        REGISTRY_RELATIVE_LBAS[usize::from(registry_index)],
        &mut actual,
    )
    .is_err()
        || actual != expected
        || parse_tombstone(&actual) != Ok(*removed)
    {
        return Err(Error::OutcomeUnknown);
    }
    Ok(())
}

fn removed_matches_expected(removed: &RemovedPackage, expected: &InstalledPackage) -> bool {
    removed.last_installed() == *expected
}

fn mirrored_tombstone_pair(candidates: [Candidate; 2], removed: &RemovedPackage) -> bool {
    matches!(
        candidates,
        [
            Candidate::Valid(DurableRecord::Removed(first)),
            Candidate::Valid(DurableRecord::Removed(second))
        ] if first.record == *removed && second.record == *removed
    )
}

fn installed_from_active(active: ActiveRecord) -> InstalledPackage {
    let _ = active.registry_index;
    InstalledPackage {
        metadata: active.record.header.metadata,
        apk_length: active.record.header.apk_length,
        generation: active.record.header.generation,
        slot: active.record.header.slot,
    }
}

fn write_inactive_blob<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    header: BlobHeader,
    apk: &[u8],
) -> Result<(), Error> {
    let blob_start = BLOB_RELATIVE_LBAS[usize::from(header.slot)];
    for payload_index in 0..BLOB_PAYLOAD_SECTORS {
        let mut sector = [0_u8; SECTOR_SIZE];
        let start = payload_index as usize * SECTOR_SIZE;
        if start < apk.len() {
            let end = core::cmp::min(start + SECTOR_SIZE, apk.len());
            sector[..end - start].copy_from_slice(&apk[start..end]);
        }
        write_relative(io, volume_start, blob_start + 1 + payload_index, &sector)?;
    }
    let sector = encode_blob_header(&header);
    write_relative(io, volume_start, blob_start, &sector)
}

fn read_blob_header<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    slot: u8,
) -> Result<(BlobHeader, Sector), Error> {
    let mut sector = [0_u8; SECTOR_SIZE];
    read_relative(
        io,
        volume_start,
        BLOB_RELATIVE_LBAS[usize::from(slot)],
        &mut sector,
    )?;
    let header = parse_blob_header(&sector, slot)?;
    Ok((header, sector))
}

fn validate_blob_payload<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    header: &BlobHeader,
    mut output: Option<&mut [u8]>,
) -> Result<(), Error> {
    let blob_start = BLOB_RELATIVE_LBAS[usize::from(header.slot)];
    let apk_length = header.apk_length as usize;
    let mut hasher = Sha256::new();
    let mut copied = 0_usize;
    for payload_index in 0..BLOB_PAYLOAD_SECTORS {
        let mut sector = [0_u8; SECTOR_SIZE];
        read_relative(
            io,
            volume_start,
            blob_start + 1 + payload_index,
            &mut sector,
        )?;
        let remaining = apk_length.saturating_sub(copied);
        let meaningful = core::cmp::min(remaining, SECTOR_SIZE);
        if meaningful != 0 {
            hasher.update(&sector[..meaningful]);
        }
        if let Some(destination) = output.as_deref_mut() {
            destination[copied..copied + meaningful].copy_from_slice(&sector[..meaningful]);
        }
        if sector[meaningful..].iter().any(|byte| *byte != 0) {
            return Err(Error::Corrupt(Corruption::BlobPaddingNonZero));
        }
        copied += meaningful;
    }
    if copied != apk_length || hasher.finalize() != header.metadata.apk_sha256 {
        return Err(Error::Corrupt(Corruption::BlobDigestMismatch));
    }
    Ok(())
}

fn encode_superblock() -> Sector {
    let mut sector = [0_u8; SECTOR_SIZE];
    sector[..8].copy_from_slice(&SUPERBLOCK_MAGIC);
    put_u32(&mut sector, 8, FORMAT_VERSION);
    put_u32(&mut sector, 12, SECTOR_SIZE as u32);
    put_u64(&mut sector, 16, VOLUME_SECTORS);
    sector[24..40].copy_from_slice(&FORMAT_EPOCH);
    put_u64(&mut sector, 40, REGISTRY_RELATIVE_LBAS[0]);
    put_u64(&mut sector, 48, REGISTRY_RELATIVE_LBAS[1]);
    put_u64(&mut sector, 56, BLOB_RELATIVE_LBAS[0]);
    put_u64(&mut sector, 64, BLOB_RELATIVE_LBAS[1]);
    put_u64(&mut sector, 72, BLOB_SECTORS);
    put_u32(&mut sector, 80, MAX_APK_BYTES as u32);
    put_u32(&mut sector, 84, 0);
    seal_record(&mut sector);
    sector
}

fn parse_superblock(sector: &Sector) -> Result<(), Error> {
    if !valid_seal(sector)
        || sector[..8] != SUPERBLOCK_MAGIC
        || le_u32(sector, 8) != FORMAT_VERSION
        || le_u32(sector, 12) != SECTOR_SIZE as u32
        || le_u64(sector, 16) != VOLUME_SECTORS
        || sector[24..40] != FORMAT_EPOCH
        || le_u64(sector, 40) != REGISTRY_RELATIVE_LBAS[0]
        || le_u64(sector, 48) != REGISTRY_RELATIVE_LBAS[1]
        || le_u64(sector, 56) != BLOB_RELATIVE_LBAS[0]
        || le_u64(sector, 64) != BLOB_RELATIVE_LBAS[1]
        || le_u64(sector, 72) != BLOB_SECTORS
        || le_u32(sector, 80) != MAX_APK_BYTES as u32
        || le_u32(sector, 84) != 0
        || sector[88..RECORD_CRC_OFFSET].iter().any(|byte| *byte != 0)
    {
        return Err(Error::Corrupt(Corruption::BadSuperblock));
    }
    Ok(())
}

fn encode_blob_header(header: &BlobHeader) -> Sector {
    encode_record(BLOB_MAGIC, header, None)
}

fn encode_registry(record: &RegistryRecord) -> Sector {
    encode_record(
        REGISTRY_MAGIC,
        &record.header,
        Some(record.blob_header_sha256),
    )
}

fn encode_tombstone(removed: &RemovedPackage) -> Sector {
    let mut sector = [0_u8; SECTOR_SIZE];
    sector[..8].copy_from_slice(&TOMBSTONE_MAGIC);
    put_u32(&mut sector, 8, FORMAT_VERSION);
    put_u32(&mut sector, 12, COMMITTED_FLAG);
    sector[16..32].copy_from_slice(&FORMAT_EPOCH);
    put_u64(&mut sector, TOMBSTONE_GENERATION_OFFSET, removed.generation);
    put_u64(
        &mut sector,
        TOMBSTONE_OPERATION_OFFSET,
        removed.operation_id,
    );
    put_u64(
        &mut sector,
        TOMBSTONE_LAST_GENERATION_OFFSET,
        removed.last_generation,
    );
    put_u64(
        &mut sector,
        TOMBSTONE_VERSION_CODE_OFFSET,
        removed.last_metadata.version_code,
    );
    put_u32(&mut sector, TOMBSTONE_APK_LENGTH_OFFSET, removed.apk_length);
    put_u16(
        &mut sector,
        TOMBSTONE_DISPOSITION_OFFSET,
        removed.disposition.raw(),
    );
    sector[TOMBSTONE_LAST_SLOT_OFFSET] = removed.last_blob_slot;
    sector[TOMBSTONE_PACKAGE_LENGTH_OFFSET] = removed.last_metadata.package.length;
    sector[TOMBSTONE_APK_DIGEST_RANGE].copy_from_slice(&removed.last_metadata.apk_sha256);
    sector[TOMBSTONE_SIGNER_DIGEST_RANGE]
        .copy_from_slice(&removed.last_metadata.signer_cert_sha256);
    sector[TOMBSTONE_PACKAGE_RANGE.start
        ..TOMBSTONE_PACKAGE_RANGE.start + usize::from(removed.last_metadata.package.length)]
        .copy_from_slice(
            &removed.last_metadata.package.bytes
                [..usize::from(removed.last_metadata.package.length)],
        );
    sector[TOMBSTONE_ACTIVITY_RANGE.start
        ..TOMBSTONE_ACTIVITY_RANGE.start + usize::from(removed.last_metadata.activity.length)]
        .copy_from_slice(
            &removed.last_metadata.activity.bytes
                [..usize::from(removed.last_metadata.activity.length)],
        );
    put_u64(
        &mut sector,
        TOMBSTONE_INSTALL_TRANSACTION_OFFSET,
        removed.last_metadata.transaction_id,
    );
    put_u16(
        &mut sector,
        TOMBSTONE_PROFILE_OFFSET,
        removed.last_metadata.compatibility_profile.raw(),
    );
    sector[TOMBSTONE_ACTIVITY_LENGTH_OFFSET] = removed.last_metadata.activity.length;
    seal_record(&mut sector);
    sector
}

fn parse_tombstone(sector: &Sector) -> Result<RemovedPackage, Error> {
    let generation = le_u64(sector, TOMBSTONE_GENERATION_OFFSET);
    let last_generation = le_u64(sector, TOMBSTONE_LAST_GENERATION_OFFSET);
    let apk_length = le_u32(sector, TOMBSTONE_APK_LENGTH_OFFSET);
    if !valid_seal(sector)
        || sector[..8] != TOMBSTONE_MAGIC
        || le_u32(sector, 8) != FORMAT_VERSION
        || le_u32(sector, 12) != COMMITTED_FLAG
        || sector[16..32] != FORMAT_EPOCH
        || generation == 0
        || le_u64(sector, TOMBSTONE_OPERATION_OFFSET) == 0
        || last_generation == 0
        || last_generation.checked_add(1) != Some(generation)
        || le_u64(sector, TOMBSTONE_INSTALL_TRANSACTION_OFFSET) == 0
        || le_u64(sector, TOMBSTONE_VERSION_CODE_OFFSET) == 0
        || apk_length == 0
        || apk_length as usize > MAX_APK_BYTES
        || sector[TOMBSTONE_LAST_SLOT_OFFSET] > 1
        || sector[TOMBSTONE_RESERVED_RANGE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(Error::Corrupt(Corruption::BadRegistry));
    }
    let disposition = DataDisposition::from_raw(le_u16(sector, TOMBSTONE_DISPOSITION_OFFSET))
        .ok_or(Error::Corrupt(Corruption::BadRegistry))?;
    let package = FixedAscii::decode(
        usize::from(sector[TOMBSTONE_PACKAGE_LENGTH_OFFSET]),
        &sector[TOMBSTONE_PACKAGE_RANGE],
        MetadataError::EmptyPackage,
        MetadataError::PackageTooLong,
        MetadataError::NonAsciiPackage,
    )
    .map_err(|_| Error::Corrupt(Corruption::BadRegistry))?;
    let activity = FixedAscii::decode(
        usize::from(sector[TOMBSTONE_ACTIVITY_LENGTH_OFFSET]),
        &sector[TOMBSTONE_ACTIVITY_RANGE],
        MetadataError::EmptyActivity,
        MetadataError::ActivityTooLong,
        MetadataError::NonAsciiActivity,
    )
    .map_err(|_| Error::Corrupt(Corruption::BadRegistry))?;
    let compatibility_profile =
        CompatibilityProfile::from_raw(le_u16(sector, TOMBSTONE_PROFILE_OFFSET))
            .ok_or(Error::Corrupt(Corruption::BadRegistry))?;
    let mut apk_sha256 = [0_u8; 32];
    apk_sha256.copy_from_slice(&sector[TOMBSTONE_APK_DIGEST_RANGE]);
    let mut signer_cert_sha256 = [0_u8; 32];
    signer_cert_sha256.copy_from_slice(&sector[TOMBSTONE_SIGNER_DIGEST_RANGE]);
    if signer_cert_sha256.iter().all(|byte| *byte == 0) {
        return Err(Error::Corrupt(Corruption::BadRegistry));
    }
    Ok(RemovedPackage {
        operation_id: le_u64(sector, TOMBSTONE_OPERATION_OFFSET),
        last_metadata: InstallMetadata {
            transaction_id: le_u64(sector, TOMBSTONE_INSTALL_TRANSACTION_OFFSET),
            package,
            activity,
            version_code: le_u64(sector, TOMBSTONE_VERSION_CODE_OFFSET),
            apk_sha256,
            signer_cert_sha256,
            compatibility_profile,
        },
        generation,
        last_generation,
        apk_length,
        last_blob_slot: sector[TOMBSTONE_LAST_SLOT_OFFSET],
        disposition,
    })
}

fn encode_record(
    magic: [u8; 8],
    header: &BlobHeader,
    blob_header_sha256: Option<[u8; 32]>,
) -> Sector {
    let mut sector = [0_u8; SECTOR_SIZE];
    sector[..8].copy_from_slice(&magic);
    put_u32(&mut sector, 8, FORMAT_VERSION);
    put_u32(&mut sector, 12, COMMITTED_FLAG);
    sector[16..32].copy_from_slice(&FORMAT_EPOCH);
    put_u64(&mut sector, RECORD_GENERATION_OFFSET, header.generation);
    put_u64(
        &mut sector,
        RECORD_TRANSACTION_OFFSET,
        header.metadata.transaction_id,
    );
    put_u64(
        &mut sector,
        RECORD_VERSION_CODE_OFFSET,
        header.metadata.version_code,
    );
    put_u32(&mut sector, RECORD_APK_LENGTH_OFFSET, header.apk_length);
    put_u16(
        &mut sector,
        RECORD_PROFILE_OFFSET,
        header.metadata.compatibility_profile.raw(),
    );
    sector[RECORD_SLOT_OFFSET] = header.slot;
    sector[RECORD_PACKAGE_LENGTH_OFFSET] = header.metadata.package.length;
    sector[RECORD_ACTIVITY_LENGTH_OFFSET] = header.metadata.activity.length;
    sector[RECORD_APK_DIGEST_RANGE].copy_from_slice(&header.metadata.apk_sha256);
    sector[RECORD_SIGNER_DIGEST_RANGE].copy_from_slice(&header.metadata.signer_cert_sha256);
    if let Some(digest) = blob_header_sha256 {
        sector[RECORD_BLOB_HEADER_DIGEST_RANGE].copy_from_slice(&digest);
    }
    sector[RECORD_PACKAGE_RANGE.start
        ..RECORD_PACKAGE_RANGE.start + usize::from(header.metadata.package.length)]
        .copy_from_slice(
            &header.metadata.package.bytes[..usize::from(header.metadata.package.length)],
        );
    sector[RECORD_ACTIVITY_RANGE.start
        ..RECORD_ACTIVITY_RANGE.start + usize::from(header.metadata.activity.length)]
        .copy_from_slice(
            &header.metadata.activity.bytes[..usize::from(header.metadata.activity.length)],
        );
    seal_record(&mut sector);
    sector
}

fn parse_blob_header(sector: &Sector, expected_slot: u8) -> Result<BlobHeader, Error> {
    parse_record(sector, BLOB_MAGIC, expected_slot, true).map(|record| record.header)
}

fn parse_registry(sector: &Sector, expected_slot: u8) -> Result<RegistryRecord, Error> {
    parse_record(sector, REGISTRY_MAGIC, expected_slot, false)
}

fn parse_record(
    sector: &Sector,
    magic: [u8; 8],
    expected_slot: u8,
    blob: bool,
) -> Result<RegistryRecord, Error> {
    let corruption = if blob {
        Corruption::BadBlobHeader
    } else {
        Corruption::BadRegistry
    };
    if !valid_seal(sector)
        || sector[..8] != magic
        || le_u32(sector, 8) != FORMAT_VERSION
        || le_u32(sector, 12) != COMMITTED_FLAG
        || sector[16..32] != FORMAT_EPOCH
        || le_u64(sector, RECORD_GENERATION_OFFSET) == 0
        || le_u64(sector, RECORD_TRANSACTION_OFFSET) == 0
        || le_u64(sector, RECORD_VERSION_CODE_OFFSET) == 0
        || le_u32(sector, RECORD_APK_LENGTH_OFFSET) == 0
        || le_u32(sector, RECORD_APK_LENGTH_OFFSET) as usize > MAX_APK_BYTES
        || sector[RECORD_SLOT_OFFSET] != expected_slot
        || sector[RECORD_FIRST_RESERVED_RANGE]
            .iter()
            .any(|byte| *byte != 0)
        || sector[RECORD_FINAL_RESERVED_RANGE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(Error::Corrupt(corruption));
    }
    let compatibility_profile =
        CompatibilityProfile::from_raw(le_u16(sector, RECORD_PROFILE_OFFSET))
            .ok_or(Error::Corrupt(corruption))?;
    let package = FixedAscii::decode(
        usize::from(sector[RECORD_PACKAGE_LENGTH_OFFSET]),
        &sector[RECORD_PACKAGE_RANGE],
        MetadataError::EmptyPackage,
        MetadataError::PackageTooLong,
        MetadataError::NonAsciiPackage,
    )
    .map_err(|_| Error::Corrupt(corruption))?;
    let activity = FixedAscii::decode(
        usize::from(sector[RECORD_ACTIVITY_LENGTH_OFFSET]),
        &sector[RECORD_ACTIVITY_RANGE],
        MetadataError::EmptyActivity,
        MetadataError::ActivityTooLong,
        MetadataError::NonAsciiActivity,
    )
    .map_err(|_| Error::Corrupt(corruption))?;
    let mut apk_sha256 = [0_u8; 32];
    apk_sha256.copy_from_slice(&sector[RECORD_APK_DIGEST_RANGE]);
    let mut signer_cert_sha256 = [0_u8; 32];
    signer_cert_sha256.copy_from_slice(&sector[RECORD_SIGNER_DIGEST_RANGE]);
    if signer_cert_sha256.iter().all(|byte| *byte == 0) {
        return Err(Error::Corrupt(corruption));
    }
    let mut blob_header_digest = [0_u8; 32];
    blob_header_digest.copy_from_slice(&sector[RECORD_BLOB_HEADER_DIGEST_RANGE]);
    if blob {
        if blob_header_digest.iter().any(|byte| *byte != 0) {
            return Err(Error::Corrupt(corruption));
        }
    } else if blob_header_digest.iter().all(|byte| *byte == 0) {
        return Err(Error::Corrupt(corruption));
    }
    Ok(RegistryRecord {
        header: BlobHeader {
            metadata: InstallMetadata {
                transaction_id: le_u64(sector, RECORD_TRANSACTION_OFFSET),
                package,
                activity,
                version_code: le_u64(sector, RECORD_VERSION_CODE_OFFSET),
                apk_sha256,
                signer_cert_sha256,
                compatibility_profile,
            },
            apk_length: le_u32(sector, RECORD_APK_LENGTH_OFFSET),
            generation: le_u64(sector, RECORD_GENERATION_OFFSET),
            slot: expected_slot,
        },
        blob_header_sha256: blob_header_digest,
    })
}

fn check_volume<I: SectorIo>(io: &I, volume_start: u64) -> Result<(), Error> {
    let end = volume_start
        .checked_add(VOLUME_SECTORS)
        .ok_or(Error::VolumeBounds)?;
    if end > io.sector_count() {
        return Err(Error::VolumeBounds);
    }
    Ok(())
}

fn read_relative<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    relative: u64,
    output: &mut Sector,
) -> Result<(), Error> {
    if relative >= VOLUME_SECTORS {
        return Err(Error::VolumeBounds);
    }
    let absolute = volume_start
        .checked_add(relative)
        .ok_or(Error::VolumeBounds)?;
    io.read(absolute, output).map_err(map_io)
}

fn write_relative<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    relative: u64,
    input: &Sector,
) -> Result<(), Error> {
    if relative >= VOLUME_SECTORS {
        return Err(Error::VolumeBounds);
    }
    let absolute = volume_start
        .checked_add(relative)
        .ok_or(Error::VolumeBounds)?;
    io.write(absolute, input).map_err(map_io)
}

const fn map_io(error: IoError) -> Error {
    match error {
        IoError::OutcomeUnknown => Error::OutcomeUnknown,
        other => Error::Io(other),
    }
}

const fn map_outcome_io(_error: IoError) -> Error {
    Error::OutcomeUnknown
}

fn is_zero(sector: &Sector) -> bool {
    sector.iter().all(|byte| *byte == 0)
}

fn seal_record(sector: &mut Sector) {
    let crc = crc32(&sector[..RECORD_CRC_OFFSET]);
    put_u32(sector, RECORD_CRC_OFFSET, crc);
}

fn valid_seal(sector: &Sector) -> bool {
    le_u32(sector, RECORD_CRC_OFFSET) == crc32(&sector[..RECORD_CRC_OFFSET])
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn le_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn le_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests;
