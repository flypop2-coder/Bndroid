//! Allocation-free wire helpers for QEMU's firmware-configuration DMA ABI.
//!
//! The MMIO transport itself lives in the kernel binary because it performs
//! volatile device accesses.  Keeping the byte-order and directory parsing
//! here makes the externally supplied wire format exhaustively host-testable.

use core::mem::{align_of, size_of};

pub const SIGNATURE_SELECTOR: u16 = 0x0000;
pub const FEATURES_SELECTOR: u16 = 0x0001;
pub const FILE_DIRECTORY_SELECTOR: u16 = 0x0019;
pub const DMA_FEATURE: u32 = 1 << 1;

pub const DMA_CONTROL_ERROR: u32 = 1 << 0;
pub const DMA_CONTROL_READ: u32 = 1 << 1;
pub const DMA_CONTROL_SKIP: u32 = 1 << 2;
pub const DMA_CONTROL_SELECT: u32 = 1 << 3;
pub const DMA_CONTROL_WRITE: u32 = 1 << 4;

pub const FILE_DIRECTORY_HEADER_BYTES: usize = 4;
pub const FILE_ENTRY_BYTES: usize = 64;
pub const FILE_NAME_BYTES: usize = 56;
pub const RAMFB_FILE_NAME: &[u8] = b"etc/ramfb";
pub const RAMFB_CONFIG_BYTES: u32 = 28;
/// Explicit, read-only QEMU-local APK source used by Install-0.
///
/// Merely finding this file grants no install authority. The package host must
/// still verify the bounded APK profile and commit it through package storage.
pub const APK_SOURCE_FILE_NAME: &[u8] = b"opt/bndroid/apk";
/// Explicit, read-only QEMU-local Uninstall-0 request.
///
/// This is a fixed canonical wire input, not a host directory or an authority
/// grant. The package manager must still bind every expected identity field to
/// the current durable package before committing a tombstone.
pub const PACKAGE_UNINSTALL_FILE_NAME: &[u8] = b"opt/bndroid/package-uninstall";
pub const PACKAGE_UNINSTALL_REQUEST_BYTES: usize = 256;
pub const PACKAGE_UNINSTALL_PACKAGE_BYTES: usize = 96;
pub const PACKAGE_UNINSTALL_MAX_APK_BYTES: u32 = 65_024;

const PACKAGE_UNINSTALL_MAGIC: [u8; 8] = *b"BNDUNS01";
const PACKAGE_UNINSTALL_VERSION: u32 = 1;
const PACKAGE_UNINSTALL_FLAGS: u32 = 0;
const PACKAGE_UNINSTALL_CRC_OFFSET: usize = 252;
const PACKAGE_UNINSTALL_RESERVED_RANGE: core::ops::Range<usize> = 208..252;

pub const DRM_FORMAT_XRGB8888: u32 = fourcc(b'X', b'R', b'2', b'4');

#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DmaAccess {
    pub control_be: u32,
    pub length_be: u32,
    pub address_be: u64,
}

impl DmaAccess {
    pub const fn new(control: u32, length: u32, address: u64) -> Self {
        Self {
            control_be: control.to_be(),
            length_be: length.to_be(),
            address_be: address.to_be(),
        }
    }

    pub const fn control(self) -> u32 {
        u32::from_be(self.control_be)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RamFbConfig {
    bytes: [u8; RAMFB_CONFIG_BYTES as usize],
}

impl RamFbConfig {
    pub const fn xrgb8888(address: u64, width: u32, height: u32, stride: u32) -> Self {
        let address = address.to_be_bytes();
        let fourcc = DRM_FORMAT_XRGB8888.to_be_bytes();
        let flags = 0_u32.to_be_bytes();
        let width = width.to_be_bytes();
        let height = height.to_be_bytes();
        let stride = stride.to_be_bytes();
        Self {
            bytes: [
                address[0], address[1], address[2], address[3], address[4], address[5], address[6],
                address[7], fourcc[0], fourcc[1], fourcc[2], fourcc[3], flags[0], flags[1],
                flags[2], flags[3], width[0], width[1], width[2], width[3], height[0], height[1],
                height[2], height[3], stride[0], stride[1], stride[2], stride[3],
            ],
        }
    }

    pub const fn as_bytes(&self) -> &[u8; RAMFB_CONFIG_BYTES as usize] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileEntry<'a> {
    pub size: u32,
    pub selector: u16,
    pub name: &'a [u8],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PackageInputFiles<'a> {
    pub apk: Option<FileEntry<'a>>,
    pub uninstall_request: Option<FileEntry<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryError {
    Truncated,
    CountTooLarge,
    ReservedNonZero,
    EmptyName,
    UnterminatedName,
    InvalidNamePadding,
    DuplicateRamFb,
    InvalidRamFbSize,
    DuplicateRequestedFile,
    InvalidRequestedName,
}

impl DirectoryError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Truncated => "fw_cfg file directory is truncated",
            Self::CountTooLarge => "fw_cfg file count exceeds the supplied directory bytes",
            Self::ReservedNonZero => "fw_cfg file entry reserved field is nonzero",
            Self::EmptyName => "fw_cfg file entry has an empty name",
            Self::UnterminatedName => "fw_cfg file entry name is not NUL terminated",
            Self::InvalidNamePadding => "fw_cfg file entry name padding is nonzero",
            Self::DuplicateRamFb => "fw_cfg contains duplicate etc/ramfb entries",
            Self::InvalidRamFbSize => "fw_cfg etc/ramfb entry is not 28 bytes",
            Self::DuplicateRequestedFile => {
                "fw_cfg contains duplicate entries for the requested file"
            }
            Self::InvalidRequestedName => "requested fw_cfg file name is invalid",
        }
    }
}

pub fn directory_count(bytes: &[u8]) -> Result<u32, DirectoryError> {
    let raw = bytes
        .get(..FILE_DIRECTORY_HEADER_BYTES)
        .ok_or(DirectoryError::Truncated)?;
    Ok(u32::from_be_bytes(
        raw.try_into().map_err(|_| DirectoryError::Truncated)?,
    ))
}

pub fn directory_entry(bytes: &[u8], index: u32) -> Result<FileEntry<'_>, DirectoryError> {
    let count = directory_count(bytes)?;
    if index >= count {
        return Err(DirectoryError::CountTooLarge);
    }
    let offset = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            usize::try_from(index)
                .map_err(|_| DirectoryError::CountTooLarge)?
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(DirectoryError::CountTooLarge)?,
        )
        .ok_or(DirectoryError::CountTooLarge)?;
    let entry = bytes
        .get(offset..offset + FILE_ENTRY_BYTES)
        .ok_or(DirectoryError::CountTooLarge)?;
    let size = u32::from_be_bytes(entry[0..4].try_into().expect("fixed slice"));
    let selector = u16::from_be_bytes(entry[4..6].try_into().expect("fixed slice"));
    if entry[6..8] != [0, 0] {
        return Err(DirectoryError::ReservedNonZero);
    }
    let name_field = &entry[8..8 + FILE_NAME_BYTES];
    let terminator = name_field
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(DirectoryError::UnterminatedName)?;
    if terminator == 0 {
        return Err(DirectoryError::EmptyName);
    }
    if name_field[terminator + 1..].iter().any(|byte| *byte != 0) {
        return Err(DirectoryError::InvalidNamePadding);
    }
    Ok(FileEntry {
        size,
        selector,
        name: &name_field[..terminator],
    })
}

pub fn find_ramfb(bytes: &[u8]) -> Result<Option<FileEntry<'_>>, DirectoryError> {
    let count = directory_count(bytes)?;
    let required = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            usize::try_from(count)
                .map_err(|_| DirectoryError::CountTooLarge)?
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(DirectoryError::CountTooLarge)?,
        )
        .ok_or(DirectoryError::CountTooLarge)?;
    if required > bytes.len() {
        return Err(DirectoryError::CountTooLarge);
    }

    let mut found = None;
    for index in 0..count {
        let entry = directory_entry(bytes, index)?;
        if entry.name == RAMFB_FILE_NAME {
            if found.is_some() {
                return Err(DirectoryError::DuplicateRamFb);
            }
            if entry.size != RAMFB_CONFIG_BYTES {
                return Err(DirectoryError::InvalidRamFbSize);
            }
            found = Some(entry);
        }
    }
    Ok(found)
}

/// Finds one exact QEMU `fw_cfg` file without accepting an ambiguous source.
///
/// The directory must be complete and canonical even when the requested file
/// is absent. Names are supplied by trusted kernel code, never by an EL0
/// caller; empty names, embedded NUL bytes, and names wider than the wire field
/// are rejected here so callers cannot accidentally request a prefix.
pub fn find_unique_file<'a>(
    bytes: &'a [u8],
    requested_name: &[u8],
) -> Result<Option<FileEntry<'a>>, DirectoryError> {
    if requested_name.is_empty()
        || requested_name.len() >= FILE_NAME_BYTES
        || requested_name.contains(&0)
    {
        return Err(DirectoryError::InvalidRequestedName);
    }
    let count = directory_count(bytes)?;
    let required = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            usize::try_from(count)
                .map_err(|_| DirectoryError::CountTooLarge)?
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(DirectoryError::CountTooLarge)?,
        )
        .ok_or(DirectoryError::CountTooLarge)?;
    if required > bytes.len() {
        return Err(DirectoryError::CountTooLarge);
    }

    let mut found = None;
    for index in 0..count {
        let entry = directory_entry(bytes, index)?;
        if entry.name == requested_name {
            if found.is_some() {
                return Err(DirectoryError::DuplicateRequestedFile);
            }
            found = Some(entry);
        }
    }
    Ok(found)
}

/// Finds the two explicit package inputs in one canonical directory scan.
///
/// Either file is optional, but duplicate entries for either exact name are
/// rejected. Whether both distinct inputs conflict is a boot-policy decision
/// made by `package_source` before it copies either payload.
pub fn find_package_inputs(bytes: &[u8]) -> Result<PackageInputFiles<'_>, DirectoryError> {
    let count = directory_count(bytes)?;
    let required = FILE_DIRECTORY_HEADER_BYTES
        .checked_add(
            usize::try_from(count)
                .map_err(|_| DirectoryError::CountTooLarge)?
                .checked_mul(FILE_ENTRY_BYTES)
                .ok_or(DirectoryError::CountTooLarge)?,
        )
        .ok_or(DirectoryError::CountTooLarge)?;
    if required > bytes.len() {
        return Err(DirectoryError::CountTooLarge);
    }

    let mut inputs = PackageInputFiles::default();
    for index in 0..count {
        let entry = directory_entry(bytes, index)?;
        let target = if entry.name == APK_SOURCE_FILE_NAME {
            &mut inputs.apk
        } else if entry.name == PACKAGE_UNINSTALL_FILE_NAME {
            &mut inputs.uninstall_request
        } else {
            continue;
        };
        if target.is_some() {
            return Err(DirectoryError::DuplicateRequestedFile);
        }
        *target = Some(entry);
    }
    Ok(inputs)
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageUninstallDataDisposition {
    NoManagedPackageData = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageUninstallRequest<'a> {
    pub operation_id: u64,
    pub expected_generation: u64,
    pub expected_version_code: u64,
    pub expected_apk_length: u32,
    pub disposition: PackageUninstallDataDisposition,
    pub expected_apk_sha256: [u8; 32],
    pub expected_signer_sha256: [u8; 32],
    pub package: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageUninstallRequestError {
    InvalidLength,
    CrcMismatch,
    InvalidMagic,
    UnsupportedVersion,
    FlagsNonZero,
    OperationIdZero,
    ExpectedGenerationZero,
    ExpectedVersionCodeZero,
    ExpectedApkLengthZero,
    ExpectedApkTooLarge,
    EmptyPackage,
    PackageTooLong,
    UnsupportedDisposition,
    ZeroApkSha256,
    ZeroSignerSha256,
    NonPrintablePackage,
    NonZeroPackagePadding,
    ReservedNonZero,
}

impl PackageUninstallRequestError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidLength => "package uninstall request is not exactly 256 bytes",
            Self::CrcMismatch => "package uninstall request CRC32 is invalid",
            Self::InvalidMagic => "package uninstall request magic is invalid",
            Self::UnsupportedVersion => "package uninstall request version is unsupported",
            Self::FlagsNonZero => "package uninstall request flags are nonzero",
            Self::OperationIdZero => "package uninstall request operation id is zero",
            Self::ExpectedGenerationZero => "package uninstall request expected generation is zero",
            Self::ExpectedVersionCodeZero => {
                "package uninstall request expected version code is zero"
            }
            Self::ExpectedApkLengthZero => "package uninstall request expected APK length is zero",
            Self::ExpectedApkTooLarge => {
                "package uninstall request expected APK length exceeds the package-store bound"
            }
            Self::EmptyPackage => "package uninstall request package is empty",
            Self::PackageTooLong => "package uninstall request package is too long",
            Self::UnsupportedDisposition => {
                "package uninstall request data disposition is unsupported"
            }
            Self::ZeroApkSha256 => "package uninstall request APK SHA-256 is zero",
            Self::ZeroSignerSha256 => "package uninstall request signer SHA-256 is zero",
            Self::NonPrintablePackage => "package uninstall request package is not printable ASCII",
            Self::NonZeroPackagePadding => "package uninstall request package padding is nonzero",
            Self::ReservedNonZero => "package uninstall request reserved bytes are nonzero",
        }
    }
}

/// Parses and fully validates one canonical Uninstall-0 request.
///
/// The CRC32 is the IEEE value over bytes `0..252` and is stored little
/// endian in bytes `252..256`. All multibyte scalar fields are little endian.
pub fn parse_package_uninstall_request(
    bytes: &[u8],
) -> Result<PackageUninstallRequest<'_>, PackageUninstallRequestError> {
    if bytes.len() != PACKAGE_UNINSTALL_REQUEST_BYTES {
        return Err(PackageUninstallRequestError::InvalidLength);
    }
    let expected_crc = u32::from_le_bytes(
        bytes[PACKAGE_UNINSTALL_CRC_OFFSET..PACKAGE_UNINSTALL_REQUEST_BYTES]
            .try_into()
            .expect("fixed uninstall CRC slice"),
    );
    if package_uninstall_request_crc32(&bytes[..PACKAGE_UNINSTALL_CRC_OFFSET]) != expected_crc {
        return Err(PackageUninstallRequestError::CrcMismatch);
    }
    if bytes[..8] != PACKAGE_UNINSTALL_MAGIC {
        return Err(PackageUninstallRequestError::InvalidMagic);
    }
    if read_uninstall_u32(bytes, 8) != PACKAGE_UNINSTALL_VERSION {
        return Err(PackageUninstallRequestError::UnsupportedVersion);
    }
    if read_uninstall_u32(bytes, 12) != PACKAGE_UNINSTALL_FLAGS {
        return Err(PackageUninstallRequestError::FlagsNonZero);
    }

    let operation_id = read_uninstall_u64(bytes, 16);
    if operation_id == 0 {
        return Err(PackageUninstallRequestError::OperationIdZero);
    }
    let expected_generation = read_uninstall_u64(bytes, 24);
    if expected_generation == 0 {
        return Err(PackageUninstallRequestError::ExpectedGenerationZero);
    }
    let expected_version_code = read_uninstall_u64(bytes, 32);
    if expected_version_code == 0 {
        return Err(PackageUninstallRequestError::ExpectedVersionCodeZero);
    }
    let expected_apk_length = read_uninstall_u32(bytes, 40);
    if expected_apk_length == 0 {
        return Err(PackageUninstallRequestError::ExpectedApkLengthZero);
    }
    if expected_apk_length > PACKAGE_UNINSTALL_MAX_APK_BYTES {
        return Err(PackageUninstallRequestError::ExpectedApkTooLarge);
    }

    let package_length = usize::from(read_uninstall_u16(bytes, 44));
    if package_length == 0 {
        return Err(PackageUninstallRequestError::EmptyPackage);
    }
    if package_length > PACKAGE_UNINSTALL_PACKAGE_BYTES {
        return Err(PackageUninstallRequestError::PackageTooLong);
    }
    let disposition = match read_uninstall_u16(bytes, 46) {
        1 => PackageUninstallDataDisposition::NoManagedPackageData,
        _ => return Err(PackageUninstallRequestError::UnsupportedDisposition),
    };

    let expected_apk_sha256: [u8; 32] = bytes[48..80]
        .try_into()
        .expect("fixed uninstall APK digest slice");
    if expected_apk_sha256.iter().all(|byte| *byte == 0) {
        return Err(PackageUninstallRequestError::ZeroApkSha256);
    }
    let expected_signer_sha256: [u8; 32] = bytes[80..112]
        .try_into()
        .expect("fixed uninstall signer digest slice");
    if expected_signer_sha256.iter().all(|byte| *byte == 0) {
        return Err(PackageUninstallRequestError::ZeroSignerSha256);
    }

    let package_field = &bytes[112..208];
    let package_bytes = &package_field[..package_length];
    if package_bytes
        .iter()
        .any(|byte| !(0x20..=0x7e).contains(byte))
    {
        return Err(PackageUninstallRequestError::NonPrintablePackage);
    }
    if package_field[package_length..]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(PackageUninstallRequestError::NonZeroPackagePadding);
    }
    if bytes[PACKAGE_UNINSTALL_RESERVED_RANGE]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(PackageUninstallRequestError::ReservedNonZero);
    }
    let package =
        core::str::from_utf8(package_bytes).expect("validated printable ASCII package name");

    Ok(PackageUninstallRequest {
        operation_id,
        expected_generation,
        expected_version_code,
        expected_apk_length,
        disposition,
        expected_apk_sha256,
        expected_signer_sha256,
        package,
    })
}

pub fn package_uninstall_request_crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn read_uninstall_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("fixed uninstall u16 slice"),
    )
}

fn read_uninstall_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed uninstall u32 slice"),
    )
}

fn read_uninstall_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed uninstall u64 slice"),
    )
}

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

const _: () = assert!(size_of::<DmaAccess>() == 16);
const _: () = assert!(align_of::<DmaAccess>() == 8);
const _: () = assert!(size_of::<RamFbConfig>() == RAMFB_CONFIG_BYTES as usize);
const _: () = assert!(align_of::<RamFbConfig>() == 1);

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn directory(entries: &[(u32, u16, &[u8])]) -> std::vec::Vec<u8> {
        let mut bytes = vec![0_u8; 4 + entries.len() * FILE_ENTRY_BYTES];
        bytes[..4].copy_from_slice(&(entries.len() as u32).to_be_bytes());
        for (index, (size, selector, name)) in entries.iter().enumerate() {
            let offset = 4 + index * FILE_ENTRY_BYTES;
            bytes[offset..offset + 4].copy_from_slice(&size.to_be_bytes());
            bytes[offset + 4..offset + 6].copy_from_slice(&selector.to_be_bytes());
            bytes[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
        }
        bytes
    }

    fn canonical_uninstall_request() -> [u8; PACKAGE_UNINSTALL_REQUEST_BYTES] {
        let mut bytes = [0_u8; PACKAGE_UNINSTALL_REQUEST_BYTES];
        bytes[..8].copy_from_slice(&PACKAGE_UNINSTALL_MAGIC);
        bytes[8..12].copy_from_slice(&PACKAGE_UNINSTALL_VERSION.to_le_bytes());
        bytes[12..16].copy_from_slice(&PACKAGE_UNINSTALL_FLAGS.to_le_bytes());
        bytes[16..24].copy_from_slice(&0x1020_3040_5060_7080_u64.to_le_bytes());
        bytes[24..32].copy_from_slice(&7_u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&3_u64.to_le_bytes());
        bytes[40..44].copy_from_slice(&12_566_u32.to_le_bytes());
        bytes[44..46].copy_from_slice(&16_u16.to_le_bytes());
        bytes[46..48].copy_from_slice(&1_u16.to_le_bytes());
        bytes[48..80].fill(0xa5);
        bytes[80..112].fill(0x5a);
        bytes[112..128].copy_from_slice(b"org.bndroid.demo");
        rewrite_uninstall_crc(&mut bytes);
        bytes
    }

    fn rewrite_uninstall_crc(bytes: &mut [u8; PACKAGE_UNINSTALL_REQUEST_BYTES]) {
        let crc = package_uninstall_request_crc32(&bytes[..PACKAGE_UNINSTALL_CRC_OFFSET]);
        bytes[PACKAGE_UNINSTALL_CRC_OFFSET..].copy_from_slice(&crc.to_le_bytes());
    }

    #[test]
    fn wire_layouts_and_big_endian_fields_are_exact() {
        assert_eq!(DRM_FORMAT_XRGB8888, 0x3432_5258);
        let dma = DmaAccess::new(0x1234_001a, 28, 0x4008_1234);
        assert_eq!(dma.control(), 0x1234_001a);
        assert_eq!(dma.length_be.to_ne_bytes(), 28_u32.to_be_bytes());
        assert_eq!(dma.address_be.to_ne_bytes(), 0x4008_1234_u64.to_be_bytes());

        let config = RamFbConfig::xrgb8888(0x4010_0000, 320, 480, 1280);
        assert_eq!(&config.as_bytes()[0..8], &0x4010_0000_u64.to_be_bytes());
        assert_eq!(
            &config.as_bytes()[8..12],
            &DRM_FORMAT_XRGB8888.to_be_bytes()
        );
        assert_eq!(&config.as_bytes()[16..20], &320_u32.to_be_bytes());
        assert_eq!(&config.as_bytes()[20..24], &480_u32.to_be_bytes());
        assert_eq!(&config.as_bytes()[24..28], &1280_u32.to_be_bytes());
    }

    #[test]
    fn finds_one_exact_ramfb_entry() {
        let bytes = directory(&[
            (3, 0x20, b"etc/other\0"),
            (RAMFB_CONFIG_BYTES, 0x37, b"etc/ramfb\0"),
        ]);
        assert_eq!(
            find_ramfb(&bytes),
            Ok(Some(FileEntry {
                size: 28,
                selector: 0x37,
                name: RAMFB_FILE_NAME,
            }))
        );
    }

    #[test]
    fn distinguishes_absence_from_malformed_or_duplicate_entries() {
        assert_eq!(
            find_ramfb(&directory(&[(4, 0x20, b"etc/nope\0")])),
            Ok(None)
        );
        assert_eq!(
            find_ramfb(&directory(&[(27, 0x20, b"etc/ramfb\0")])),
            Err(DirectoryError::InvalidRamFbSize)
        );
        assert_eq!(
            find_ramfb(&directory(&[
                (28, 0x20, b"etc/ramfb\0"),
                (28, 0x21, b"etc/ramfb\0"),
            ])),
            Err(DirectoryError::DuplicateRamFb)
        );
    }

    #[test]
    fn exact_external_apk_source_is_optional_and_unambiguous() {
        let bytes = directory(&[
            (RAMFB_CONFIG_BYTES, 0x37, b"etc/ramfb\0"),
            (12_566, 0x38, b"opt/bndroid/apk\0"),
        ]);
        assert_eq!(
            find_unique_file(&bytes, APK_SOURCE_FILE_NAME),
            Ok(Some(FileEntry {
                size: 12_566,
                selector: 0x38,
                name: APK_SOURCE_FILE_NAME,
            }))
        );
        assert_eq!(find_unique_file(&bytes, b"opt/bndroid/missing"), Ok(None));

        let duplicate = directory(&[
            (1, 0x40, b"opt/bndroid/apk\0"),
            (2, 0x41, b"opt/bndroid/apk\0"),
        ]);
        assert_eq!(
            find_unique_file(&duplicate, APK_SOURCE_FILE_NAME),
            Err(DirectoryError::DuplicateRequestedFile)
        );
        assert_eq!(
            find_unique_file(&bytes, b""),
            Err(DirectoryError::InvalidRequestedName)
        );
        assert_eq!(
            find_unique_file(&bytes, b"opt/bndroid/\0apk"),
            Err(DirectoryError::InvalidRequestedName)
        );
    }

    #[test]
    fn finds_apk_and_uninstall_request_in_one_directory_scan() {
        let bytes = directory(&[
            (RAMFB_CONFIG_BYTES, 0x37, b"etc/ramfb\0"),
            (12_566, 0x38, b"opt/bndroid/apk\0"),
            (256, 0x39, b"opt/bndroid/package-uninstall\0"),
        ]);
        assert_eq!(
            find_package_inputs(&bytes),
            Ok(PackageInputFiles {
                apk: Some(FileEntry {
                    size: 12_566,
                    selector: 0x38,
                    name: APK_SOURCE_FILE_NAME,
                }),
                uninstall_request: Some(FileEntry {
                    size: 256,
                    selector: 0x39,
                    name: PACKAGE_UNINSTALL_FILE_NAME,
                }),
            })
        );
        assert_eq!(
            find_package_inputs(&directory(&[(4, 0x20, b"etc/nope\0")])),
            Ok(PackageInputFiles::default())
        );

        for duplicate_name in [
            b"opt/bndroid/apk\0".as_slice(),
            b"opt/bndroid/package-uninstall\0".as_slice(),
        ] {
            let duplicate = directory(&[(1, 0x40, duplicate_name), (2, 0x41, duplicate_name)]);
            assert_eq!(
                find_package_inputs(&duplicate),
                Err(DirectoryError::DuplicateRequestedFile)
            );
        }
    }

    #[test]
    fn parses_exact_canonical_uninstall_request_wire() {
        let bytes = canonical_uninstall_request();
        let request = parse_package_uninstall_request(&bytes).expect("canonical request");
        assert_eq!(request.operation_id, 0x1020_3040_5060_7080);
        assert_eq!(request.expected_generation, 7);
        assert_eq!(request.expected_version_code, 3);
        assert_eq!(request.expected_apk_length, 12_566);
        assert_eq!(
            request.disposition,
            PackageUninstallDataDisposition::NoManagedPackageData
        );
        assert_eq!(request.expected_apk_sha256, [0xa5; 32]);
        assert_eq!(request.expected_signer_sha256, [0x5a; 32]);
        assert_eq!(request.package, "org.bndroid.demo");
        assert_eq!(package_uninstall_request_crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(
            &bytes[252..256],
            &package_uninstall_request_crc32(&bytes[..252]).to_le_bytes()
        );
    }

    #[test]
    fn rejects_noncanonical_uninstall_request_structure() {
        let canonical = canonical_uninstall_request();
        assert_eq!(
            parse_package_uninstall_request(&canonical[..255]),
            Err(PackageUninstallRequestError::InvalidLength)
        );

        let mut bad_crc = canonical;
        bad_crc[48] ^= 1;
        assert_eq!(
            parse_package_uninstall_request(&bad_crc),
            Err(PackageUninstallRequestError::CrcMismatch)
        );

        let cases: &[(usize, &[u8], PackageUninstallRequestError)] = &[
            (0, b"BADMAGIC", PackageUninstallRequestError::InvalidMagic),
            (
                8,
                &2_u32.to_le_bytes(),
                PackageUninstallRequestError::UnsupportedVersion,
            ),
            (
                12,
                &1_u32.to_le_bytes(),
                PackageUninstallRequestError::FlagsNonZero,
            ),
            (
                16,
                &0_u64.to_le_bytes(),
                PackageUninstallRequestError::OperationIdZero,
            ),
            (
                24,
                &0_u64.to_le_bytes(),
                PackageUninstallRequestError::ExpectedGenerationZero,
            ),
            (
                32,
                &0_u64.to_le_bytes(),
                PackageUninstallRequestError::ExpectedVersionCodeZero,
            ),
            (
                40,
                &0_u32.to_le_bytes(),
                PackageUninstallRequestError::ExpectedApkLengthZero,
            ),
            (
                40,
                &(PACKAGE_UNINSTALL_MAX_APK_BYTES + 1).to_le_bytes(),
                PackageUninstallRequestError::ExpectedApkTooLarge,
            ),
            (
                44,
                &0_u16.to_le_bytes(),
                PackageUninstallRequestError::EmptyPackage,
            ),
            (
                44,
                &97_u16.to_le_bytes(),
                PackageUninstallRequestError::PackageTooLong,
            ),
            (
                46,
                &2_u16.to_le_bytes(),
                PackageUninstallRequestError::UnsupportedDisposition,
            ),
        ];
        for (offset, replacement, expected) in cases {
            let mut bytes = canonical;
            bytes[*offset..*offset + replacement.len()].copy_from_slice(replacement);
            rewrite_uninstall_crc(&mut bytes);
            assert_eq!(parse_package_uninstall_request(&bytes), Err(*expected));
        }
    }

    #[test]
    fn rejects_noncanonical_uninstall_identity_and_padding() {
        let canonical = canonical_uninstall_request();
        let mutations = [
            (48, PackageUninstallRequestError::ZeroApkSha256),
            (80, PackageUninstallRequestError::ZeroSignerSha256),
        ];
        for (offset, expected) in mutations {
            let mut bytes = canonical;
            bytes[offset..offset + 32].fill(0);
            rewrite_uninstall_crc(&mut bytes);
            assert_eq!(parse_package_uninstall_request(&bytes), Err(expected));
        }

        let mut non_printable = canonical;
        non_printable[112] = 0x1f;
        rewrite_uninstall_crc(&mut non_printable);
        assert_eq!(
            parse_package_uninstall_request(&non_printable),
            Err(PackageUninstallRequestError::NonPrintablePackage)
        );

        let mut package_padding = canonical;
        package_padding[128] = 1;
        rewrite_uninstall_crc(&mut package_padding);
        assert_eq!(
            parse_package_uninstall_request(&package_padding),
            Err(PackageUninstallRequestError::NonZeroPackagePadding)
        );

        let mut reserved = canonical;
        reserved[208] = 1;
        rewrite_uninstall_crc(&mut reserved);
        assert_eq!(
            parse_package_uninstall_request(&reserved),
            Err(PackageUninstallRequestError::ReservedNonZero)
        );
    }

    #[test]
    fn rejects_truncation_reserved_bits_and_noncanonical_names() {
        assert_eq!(directory_count(&[0, 0, 0]), Err(DirectoryError::Truncated));
        let mut truncated = directory(&[(28, 0x20, b"etc/ramfb\0")]);
        truncated.pop();
        assert_eq!(find_ramfb(&truncated), Err(DirectoryError::CountTooLarge));

        let mut reserved = directory(&[(28, 0x20, b"etc/ramfb\0")]);
        reserved[10] = 1;
        assert_eq!(find_ramfb(&reserved), Err(DirectoryError::ReservedNonZero));

        let mut padded = directory(&[(28, 0x20, b"etc/ramfb\0")]);
        padded[FILE_DIRECTORY_HEADER_BYTES + 8 + RAMFB_FILE_NAME.len() + 1] = 1;
        assert_eq!(find_ramfb(&padded), Err(DirectoryError::InvalidNamePadding));

        let unterminated_name = [b'x'; FILE_NAME_BYTES];
        assert_eq!(
            find_ramfb(&directory(&[(28, 0x20, &unterminated_name)])),
            Err(DirectoryError::UnterminatedName)
        );
    }
}
