//! Allocation-free, read-only GUID Partition Table discovery.
//!
//! The parser intentionally accepts a small, strict GPT profile suitable for
//! boot-time discovery: UEFI revision 1.0 headers, 128--512 byte power-of-two
//! entries, and at most [`MAX_PARTITION_ENTRIES`] entries. Both header copies
//! and both entry arrays are checked before any partition is returned.

use crate::block::{BlockReadError, BlockReader, SECTOR_SIZE, Sector};

pub type RawGuid = [u8; 16];

/// On-disk byte order of the Microsoft Basic Data partition type GUID
/// `EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`.
pub const MICROSOFT_BASIC_DATA_PARTITION_TYPE_GUID: RawGuid = [
    0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
];

/// Bndroid currently stores its system volume in a Microsoft Basic Data GPT
/// partition. This alias keeps the policy choice explicit at integration sites.
pub const BNDROID_SYSTEM_PARTITION_TYPE_GUID: RawGuid = MICROSOFT_BASIC_DATA_PARTITION_TYPE_GUID;

/// On-disk byte order of Bndroid's private data partition type GUID
/// `B8F4D2A1-7C3E-4B91-A6D5-0F2E9C781355`.
pub const BNDROID_DATA_PARTITION_TYPE_GUID: RawGuid = [
    0xa1, 0xd2, 0xf4, 0xb8, 0x3e, 0x7c, 0x91, 0x4b, 0xa6, 0xd5, 0x0f, 0x2e, 0x9c, 0x78, 0x13, 0x55,
];

/// On-disk byte order of Bndroid's private application-data partition type GUID
/// `B8F4D2A2-7C3E-4B91-A6D5-0F2E9C781355`.
pub const BNDROID_APPDATA_PARTITION_TYPE_GUID: RawGuid = [
    0xa2, 0xd2, 0xf4, 0xb8, 0x3e, 0x7c, 0x91, 0x4b, 0xa6, 0xd5, 0x0f, 0x2e, 0x9c, 0x78, 0x13, 0x55,
];

/// On-disk byte order of Bndroid's private installed-package partition type
/// GUID `B8F4D2A3-7C3E-4B91-A6D5-0F2E9C781355`.
pub const BNDROID_PACKAGES_PARTITION_TYPE_GUID: RawGuid = [
    0xa3, 0xd2, 0xf4, 0xb8, 0x3e, 0x7c, 0x91, 0x4b, 0xa6, 0xd5, 0x0f, 0x2e, 0x9c, 0x78, 0x13, 0x55,
];

pub const MAX_PARTITION_ENTRIES: u32 = 128;
pub const MAX_PARTITION_ENTRY_SIZE: u32 = 512;
pub const GPT_PARTITION_NAME_CODE_UNITS: usize = 36;

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const GPT_REVISION_1_0: u32 = 0x0001_0000;
const GPT_HEADER_MIN_SIZE: u32 = 92;
const GPT_HEADER_LBA: u64 = 1;
const PRIMARY_ENTRY_ARRAY_LBA: u64 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GptPartition {
    pub entry_index: u32,
    pub type_guid: RawGuid,
    pub unique_guid: RawGuid,
    pub first_lba: u64,
    pub last_lba: u64,
    pub attributes: u64,
    /// NUL-trimmed UTF-16 code units, retained without heap allocation.
    pub name_utf16: [u16; GPT_PARTITION_NAME_CODE_UNITS],
    pub name_len: u8,
}

impl GptPartition {
    pub const fn sector_count(&self) -> u64 {
        self.last_lba - self.first_lba + 1
    }

    pub fn name(&self) -> &[u16] {
        &self.name_utf16[..self.name_len as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GptEvidence {
    pub device_sectors: u64,
    pub disk_guid: RawGuid,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub primary_header_crc32: u32,
    pub backup_header_crc32: u32,
    pub partition_entry_crc32: u32,
    pub partition_entry_count: u32,
    pub partition_entry_size: u32,
    pub primary_entry_array_lba: u64,
    pub backup_entry_array_lba: u64,
    pub partition: GptPartition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GptError {
    BlockRead(BlockReadError),
    DeviceTooSmall,
    InvalidTargetTypeGuid,
    ProtectiveMbrSignature,
    ProtectiveMbrLayout,
    PrimaryHeaderSignature,
    BackupHeaderSignature,
    UnsupportedRevision,
    InvalidHeaderSize,
    InvalidHeaderReservedBytes,
    PrimaryHeaderCrc,
    BackupHeaderCrc,
    PrimaryHeaderLayout,
    BackupHeaderLayout,
    InvalidDiskGuid,
    InvalidUsableRange,
    InvalidEntryCount,
    InvalidEntrySize,
    EntryArrayOverflow,
    PrimaryEntryArrayBounds,
    BackupEntryArrayBounds,
    PrimaryEntryArrayCrc,
    BackupEntryArrayCrc,
    HeaderCopiesMismatch,
    EntryArraysMismatch,
    InvalidPartitionGuid,
    PartitionFirstAfterLast,
    PartitionOutsideUsableRange,
    DuplicateTargetPartition,
    TargetPartitionNotFound,
    TargetPartitionCopiesMismatch,
}

impl GptError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BlockRead(error) => error.as_str(),
            Self::DeviceTooSmall => "device is too small for GPT",
            Self::InvalidTargetTypeGuid => "target partition type GUID is zero",
            Self::ProtectiveMbrSignature => "protective MBR signature is invalid",
            Self::ProtectiveMbrLayout => "protective MBR layout is invalid",
            Self::PrimaryHeaderSignature => "primary GPT header signature is invalid",
            Self::BackupHeaderSignature => "backup GPT header signature is invalid",
            Self::UnsupportedRevision => "GPT header revision is unsupported",
            Self::InvalidHeaderSize => "GPT header size is invalid",
            Self::InvalidHeaderReservedBytes => "GPT header reserved bytes are nonzero",
            Self::PrimaryHeaderCrc => "primary GPT header CRC32 is invalid",
            Self::BackupHeaderCrc => "backup GPT header CRC32 is invalid",
            Self::PrimaryHeaderLayout => "primary GPT header layout is invalid",
            Self::BackupHeaderLayout => "backup GPT header layout is invalid",
            Self::InvalidDiskGuid => "GPT disk GUID is zero",
            Self::InvalidUsableRange => "GPT usable LBA range is invalid",
            Self::InvalidEntryCount => "GPT partition entry count is invalid",
            Self::InvalidEntrySize => "GPT partition entry size is invalid",
            Self::EntryArrayOverflow => "GPT partition entry array size overflows",
            Self::PrimaryEntryArrayBounds => "primary GPT entry array is out of bounds",
            Self::BackupEntryArrayBounds => "backup GPT entry array is out of bounds",
            Self::PrimaryEntryArrayCrc => "primary GPT entry array CRC32 is invalid",
            Self::BackupEntryArrayCrc => "backup GPT entry array CRC32 is invalid",
            Self::HeaderCopiesMismatch => "primary and backup GPT headers disagree",
            Self::EntryArraysMismatch => "primary and backup GPT entry arrays disagree",
            Self::InvalidPartitionGuid => "GPT partition has a zero unique GUID",
            Self::PartitionFirstAfterLast => "GPT partition first LBA is after last LBA",
            Self::PartitionOutsideUsableRange => "GPT partition is outside the usable range",
            Self::DuplicateTargetPartition => "target GPT partition type is duplicated",
            Self::TargetPartitionNotFound => "target GPT partition was not found",
            Self::TargetPartitionCopiesMismatch => {
                "target GPT partition differs between primary and backup copies"
            }
        }
    }
}

impl From<BlockReadError> for GptError {
    fn from(value: BlockReadError) -> Self {
        Self::BlockRead(value)
    }
}

#[derive(Clone, Copy)]
struct Header {
    header_crc32: u32,
    current_lba: u64,
    backup_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: RawGuid,
    entry_array_lba: u64,
    entry_count: u32,
    entry_size: u32,
    entry_array_crc32: u32,
}

#[derive(Clone, Copy)]
enum HeaderCopy {
    Primary,
    Backup,
}

impl HeaderCopy {
    const fn signature_error(self) -> GptError {
        match self {
            Self::Primary => GptError::PrimaryHeaderSignature,
            Self::Backup => GptError::BackupHeaderSignature,
        }
    }

    const fn crc_error(self) -> GptError {
        match self {
            Self::Primary => GptError::PrimaryHeaderCrc,
            Self::Backup => GptError::BackupHeaderCrc,
        }
    }
}

/// Validate a complete GPT and return the unique partition matching an on-disk
/// raw type GUID.
pub fn find_partition<R: BlockReader>(
    reader: &mut R,
    target_type_guid: RawGuid,
) -> Result<GptEvidence, GptError> {
    if is_zero_guid(&target_type_guid) {
        return Err(GptError::InvalidTargetTypeGuid);
    }

    let device_sectors = reader.sector_count();
    if device_sectors < 6 {
        return Err(GptError::DeviceTooSmall);
    }
    let backup_header_lba = device_sectors - 1;

    let mut sector = [0u8; SECTOR_SIZE];
    reader.read_sector(0, &mut sector)?;
    validate_protective_mbr(&sector, device_sectors)?;

    reader.read_sector(GPT_HEADER_LBA, &mut sector)?;
    let primary = parse_header(&sector, HeaderCopy::Primary)?;
    validate_primary_header(&primary, device_sectors)?;
    let entry_bytes = entry_array_bytes(&primary)?;
    let entry_sectors = div_ceil_sector(entry_bytes)?;
    validate_primary_entry_bounds(&primary, entry_sectors, device_sectors)?;

    reader.read_sector(backup_header_lba, &mut sector)?;
    let backup = parse_header(&sector, HeaderCopy::Backup)?;
    validate_backup_header(&backup, &primary, entry_sectors, device_sectors)?;

    let primary_partition = scan_entry_array(
        reader,
        &primary,
        entry_bytes,
        target_type_guid,
        HeaderCopy::Primary,
    )?;
    let backup_partition = scan_entry_array(
        reader,
        &backup,
        entry_bytes,
        target_type_guid,
        HeaderCopy::Backup,
    )?;
    if primary_partition != backup_partition {
        return Err(GptError::TargetPartitionCopiesMismatch);
    }
    compare_entry_arrays(
        reader,
        primary.entry_array_lba,
        backup.entry_array_lba,
        entry_bytes,
    )?;

    Ok(GptEvidence {
        device_sectors,
        disk_guid: primary.disk_guid,
        first_usable_lba: primary.first_usable_lba,
        last_usable_lba: primary.last_usable_lba,
        primary_header_crc32: primary.header_crc32,
        backup_header_crc32: backup.header_crc32,
        partition_entry_crc32: primary.entry_array_crc32,
        partition_entry_count: primary.entry_count,
        partition_entry_size: primary.entry_size,
        primary_entry_array_lba: primary.entry_array_lba,
        backup_entry_array_lba: backup.entry_array_lba,
        partition: primary_partition.ok_or(GptError::TargetPartitionNotFound)?,
    })
}

/// Convenience policy entry point for Bndroid's current system-volume type.
pub fn find_bndroid_system_partition<R: BlockReader>(
    reader: &mut R,
) -> Result<GptEvidence, GptError> {
    find_partition(reader, BNDROID_SYSTEM_PARTITION_TYPE_GUID)
}

/// Convenience policy entry point for Bndroid's bounded persistent-data area.
pub fn find_bndroid_data_partition<R: BlockReader>(
    reader: &mut R,
) -> Result<GptEvidence, GptError> {
    find_partition(reader, BNDROID_DATA_PARTITION_TYPE_GUID)
}

/// Convenience policy entry point for Bndroid's private application-data area.
pub fn find_appdata_partition<R: BlockReader>(reader: &mut R) -> Result<GptEvidence, GptError> {
    find_partition(reader, BNDROID_APPDATA_PARTITION_TYPE_GUID)
}

/// Convenience policy entry point for the bounded installed-package store.
pub fn find_packages_partition<R: BlockReader>(reader: &mut R) -> Result<GptEvidence, GptError> {
    find_partition(reader, BNDROID_PACKAGES_PARTITION_TYPE_GUID)
}

fn validate_protective_mbr(mbr: &Sector, device_sectors: u64) -> Result<(), GptError> {
    if mbr[510..512] != [0x55, 0xaa] {
        return Err(GptError::ProtectiveMbrSignature);
    }
    let protective = &mbr[446..462];
    let expected_sectors = core::cmp::min(device_sectors - 1, u32::MAX as u64) as u32;
    if protective[0] != 0
        || protective[4] != 0xee
        || le_u32(protective, 8) != 1
        || le_u32(protective, 12) != expected_sectors
        || mbr[462..510].iter().any(|byte| *byte != 0)
    {
        return Err(GptError::ProtectiveMbrLayout);
    }
    Ok(())
}

fn parse_header(sector: &Sector, copy: HeaderCopy) -> Result<Header, GptError> {
    if &sector[0..8] != GPT_SIGNATURE {
        return Err(copy.signature_error());
    }
    if le_u32(sector, 8) != GPT_REVISION_1_0 {
        return Err(GptError::UnsupportedRevision);
    }
    let header_size = le_u32(sector, 12);
    if !(GPT_HEADER_MIN_SIZE..=SECTOR_SIZE as u32).contains(&header_size) {
        return Err(GptError::InvalidHeaderSize);
    }
    if le_u32(sector, 20) != 0 || sector[header_size as usize..].iter().any(|byte| *byte != 0) {
        return Err(GptError::InvalidHeaderReservedBytes);
    }
    let expected_crc = le_u32(sector, 16);
    if header_crc32(sector, header_size as usize) != expected_crc {
        return Err(copy.crc_error());
    }

    let mut disk_guid = [0u8; 16];
    disk_guid.copy_from_slice(&sector[56..72]);
    Ok(Header {
        header_crc32: expected_crc,
        current_lba: le_u64(sector, 24),
        backup_lba: le_u64(sector, 32),
        first_usable_lba: le_u64(sector, 40),
        last_usable_lba: le_u64(sector, 48),
        disk_guid,
        entry_array_lba: le_u64(sector, 72),
        entry_count: le_u32(sector, 80),
        entry_size: le_u32(sector, 84),
        entry_array_crc32: le_u32(sector, 88),
    })
}

fn validate_primary_header(header: &Header, device_sectors: u64) -> Result<(), GptError> {
    if header.current_lba != GPT_HEADER_LBA
        || header.backup_lba != device_sectors - 1
        || header.entry_array_lba != PRIMARY_ENTRY_ARRAY_LBA
    {
        return Err(GptError::PrimaryHeaderLayout);
    }
    if is_zero_guid(&header.disk_guid) {
        return Err(GptError::InvalidDiskGuid);
    }
    if header.first_usable_lba > header.last_usable_lba
        || header.last_usable_lba >= header.backup_lba
    {
        return Err(GptError::InvalidUsableRange);
    }
    validate_entry_shape(header)
}

fn validate_entry_shape(header: &Header) -> Result<(), GptError> {
    if header.entry_count == 0 || header.entry_count > MAX_PARTITION_ENTRIES {
        return Err(GptError::InvalidEntryCount);
    }
    if header.entry_size < 128
        || header.entry_size > MAX_PARTITION_ENTRY_SIZE
        || !header.entry_size.is_power_of_two()
    {
        return Err(GptError::InvalidEntrySize);
    }
    Ok(())
}

fn entry_array_bytes(header: &Header) -> Result<u64, GptError> {
    u64::from(header.entry_count)
        .checked_mul(u64::from(header.entry_size))
        .ok_or(GptError::EntryArrayOverflow)
}

fn div_ceil_sector(bytes: u64) -> Result<u64, GptError> {
    bytes
        .checked_add(SECTOR_SIZE as u64 - 1)
        .map(|rounded| rounded / SECTOR_SIZE as u64)
        .ok_or(GptError::EntryArrayOverflow)
}

fn validate_primary_entry_bounds(
    header: &Header,
    entry_sectors: u64,
    device_sectors: u64,
) -> Result<(), GptError> {
    let end = header
        .entry_array_lba
        .checked_add(entry_sectors)
        .ok_or(GptError::EntryArrayOverflow)?;
    if end > device_sectors || end > header.first_usable_lba {
        return Err(GptError::PrimaryEntryArrayBounds);
    }
    Ok(())
}

fn validate_backup_header(
    backup: &Header,
    primary: &Header,
    entry_sectors: u64,
    device_sectors: u64,
) -> Result<(), GptError> {
    validate_entry_shape(backup)?;
    if backup.current_lba != device_sectors - 1 || backup.backup_lba != GPT_HEADER_LBA {
        return Err(GptError::BackupHeaderLayout);
    }
    if backup.first_usable_lba != primary.first_usable_lba
        || backup.last_usable_lba != primary.last_usable_lba
        || backup.disk_guid != primary.disk_guid
        || backup.entry_count != primary.entry_count
        || backup.entry_size != primary.entry_size
        || backup.entry_array_crc32 != primary.entry_array_crc32
    {
        return Err(GptError::HeaderCopiesMismatch);
    }
    let array_end = backup
        .entry_array_lba
        .checked_add(entry_sectors)
        .ok_or(GptError::EntryArrayOverflow)?;
    if backup.entry_array_lba <= backup.last_usable_lba || array_end != backup.current_lba {
        return Err(GptError::BackupEntryArrayBounds);
    }
    Ok(())
}

fn scan_entry_array<R: BlockReader>(
    reader: &mut R,
    header: &Header,
    entry_bytes: u64,
    target_type_guid: RawGuid,
    copy: HeaderCopy,
) -> Result<Option<GptPartition>, GptError> {
    let entry_size = header.entry_size as usize;
    let entries_per_sector = SECTOR_SIZE / entry_size;
    let mut remaining_bytes = entry_bytes;
    let mut entry_index = 0u32;
    let mut crc_state = !0u32;
    let mut target = None;
    let sector_count = div_ceil_sector(entry_bytes)?;
    let mut sector = [0u8; SECTOR_SIZE];

    for sector_offset in 0..sector_count {
        let lba = header
            .entry_array_lba
            .checked_add(sector_offset)
            .ok_or(GptError::EntryArrayOverflow)?;
        reader.read_sector(lba, &mut sector)?;
        let crc_bytes = core::cmp::min(remaining_bytes, SECTOR_SIZE as u64) as usize;
        crc_state = crc32_update_slice(crc_state, &sector[..crc_bytes]);
        remaining_bytes -= crc_bytes as u64;

        for slot in 0..entries_per_sector {
            if entry_index == header.entry_count {
                break;
            }
            let start = slot * entry_size;
            inspect_entry(
                &sector[start..start + entry_size],
                entry_index,
                header,
                target_type_guid,
                &mut target,
            )?;
            entry_index += 1;
        }
    }

    if !crc_state != header.entry_array_crc32 {
        return Err(match copy {
            HeaderCopy::Primary => GptError::PrimaryEntryArrayCrc,
            HeaderCopy::Backup => GptError::BackupEntryArrayCrc,
        });
    }
    Ok(target)
}

fn inspect_entry(
    entry: &[u8],
    entry_index: u32,
    header: &Header,
    target_type_guid: RawGuid,
    target: &mut Option<GptPartition>,
) -> Result<(), GptError> {
    let mut type_guid = [0u8; 16];
    type_guid.copy_from_slice(&entry[..16]);
    if is_zero_guid(&type_guid) {
        return Ok(());
    }

    let mut unique_guid = [0u8; 16];
    unique_guid.copy_from_slice(&entry[16..32]);
    if is_zero_guid(&unique_guid) {
        return Err(GptError::InvalidPartitionGuid);
    }
    let first_lba = le_u64(entry, 32);
    let last_lba = le_u64(entry, 40);
    if first_lba > last_lba {
        return Err(GptError::PartitionFirstAfterLast);
    }
    if first_lba < header.first_usable_lba || last_lba > header.last_usable_lba {
        return Err(GptError::PartitionOutsideUsableRange);
    }

    if type_guid == target_type_guid {
        if target.is_some() {
            return Err(GptError::DuplicateTargetPartition);
        }
        let mut name_utf16 = [0u16; GPT_PARTITION_NAME_CODE_UNITS];
        let mut name_len = 0u8;
        for (index, code_unit) in name_utf16.iter_mut().enumerate() {
            *code_unit = le_u16(entry, 56 + index * 2);
            if *code_unit != 0 {
                name_len = index as u8 + 1;
            }
        }
        *target = Some(GptPartition {
            entry_index,
            type_guid,
            unique_guid,
            first_lba,
            last_lba,
            attributes: le_u64(entry, 48),
            name_utf16,
            name_len,
        });
    }
    Ok(())
}

fn compare_entry_arrays<R: BlockReader>(
    reader: &mut R,
    primary_lba: u64,
    backup_lba: u64,
    entry_bytes: u64,
) -> Result<(), GptError> {
    let sectors = div_ceil_sector(entry_bytes)?;
    let mut remaining = entry_bytes;
    let mut primary = [0u8; SECTOR_SIZE];
    let mut backup = [0u8; SECTOR_SIZE];
    for offset in 0..sectors {
        reader.read_sector(
            primary_lba
                .checked_add(offset)
                .ok_or(GptError::EntryArrayOverflow)?,
            &mut primary,
        )?;
        reader.read_sector(
            backup_lba
                .checked_add(offset)
                .ok_or(GptError::EntryArrayOverflow)?,
            &mut backup,
        )?;
        let bytes = core::cmp::min(remaining, SECTOR_SIZE as u64) as usize;
        if primary[..bytes] != backup[..bytes] {
            return Err(GptError::EntryArraysMismatch);
        }
        remaining -= bytes as u64;
    }
    Ok(())
}

fn is_zero_guid(guid: &RawGuid) -> bool {
    guid.iter().all(|byte| *byte == 0)
}

fn header_crc32(sector: &Sector, header_size: usize) -> u32 {
    let mut state = !0u32;
    for (index, byte) in sector[..header_size].iter().enumerate() {
        state = crc32_update_byte(state, if (16..20).contains(&index) { 0 } else { *byte });
    }
    !state
}

fn crc32_update_slice(mut state: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        state = crc32_update_byte(state, *byte);
    }
    state
}

fn crc32_update_byte(mut crc: u32, byte: u8) -> u32 {
    crc ^= u32::from(byte);
    for _ in 0..8 {
        let mask = 0u32.wrapping_sub(crc & 1);
        crc = (crc >> 1) ^ (0xedb8_8320 & mask);
    }
    crc
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    const TEST_SECTORS: usize = 128;
    const TEST_ENTRY_COUNT: u32 = 8;
    const TEST_ENTRY_SIZE: u32 = 128;
    const TEST_ARRAY_SECTORS: u64 = 2;
    const TEST_FIRST_USABLE: u64 = 4;
    const TEST_LAST_USABLE: u64 = 124;
    const TEST_BACKUP_ARRAY_LBA: u64 = 125;
    const TEST_BACKUP_HEADER_LBA: u64 = 127;
    const TEST_DISK_GUID: RawGuid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    const TEST_UNIQUE_GUID: RawGuid = [16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

    struct TestDisk {
        sectors: Vec<Sector>,
    }

    impl BlockReader for TestDisk {
        fn sector_count(&self) -> u64 {
            self.sectors.len() as u64
        }

        fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), BlockReadError> {
            let sector = self
                .sectors
                .get(lba as usize)
                .ok_or(BlockReadError::OutOfBounds)?;
            output.copy_from_slice(sector);
            Ok(())
        }
    }

    fn valid_disk() -> TestDisk {
        let mut disk = TestDisk {
            sectors: vec![[0u8; SECTOR_SIZE]; TEST_SECTORS],
        };
        disk.sectors[0][446 + 4] = 0xee;
        put_u32(&mut disk.sectors[0], 446 + 8, 1);
        put_u32(&mut disk.sectors[0], 446 + 12, TEST_SECTORS as u32 - 1);
        disk.sectors[0][510..512].copy_from_slice(&[0x55, 0xaa]);

        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            0,
            BNDROID_SYSTEM_PARTITION_TYPE_GUID,
            TEST_UNIQUE_GUID,
            10,
            20,
            &[b's' as u16, b'y' as u16, b's' as u16],
        );
        copy_table(&mut disk);
        reseal(&mut disk);
        disk
    }

    fn write_entry(
        disk: &mut TestDisk,
        table_lba: u64,
        index: usize,
        type_guid: RawGuid,
        unique_guid: RawGuid,
        first_lba: u64,
        last_lba: u64,
        name: &[u16],
    ) {
        let byte_offset = index * TEST_ENTRY_SIZE as usize;
        let lba = table_lba as usize + byte_offset / SECTOR_SIZE;
        let offset = byte_offset % SECTOR_SIZE;
        let entry = &mut disk.sectors[lba][offset..offset + TEST_ENTRY_SIZE as usize];
        entry.fill(0);
        entry[..16].copy_from_slice(&type_guid);
        entry[16..32].copy_from_slice(&unique_guid);
        put_u64(entry, 32, first_lba);
        put_u64(entry, 40, last_lba);
        for (index, code_unit) in name.iter().take(GPT_PARTITION_NAME_CODE_UNITS).enumerate() {
            entry[56 + index * 2..58 + index * 2].copy_from_slice(&code_unit.to_le_bytes());
        }
    }

    fn copy_table(disk: &mut TestDisk) {
        for offset in 0..TEST_ARRAY_SECTORS as usize {
            disk.sectors[TEST_BACKUP_ARRAY_LBA as usize + offset] =
                disk.sectors[PRIMARY_ENTRY_ARRAY_LBA as usize + offset];
        }
    }

    fn table_crc(disk: &TestDisk, table_lba: u64, count: u32, size: u32) -> u32 {
        let bytes = count as usize * size as usize;
        let mut state = !0u32;
        let mut remaining = bytes;
        let mut lba = table_lba as usize;
        while remaining != 0 {
            let take = core::cmp::min(remaining, SECTOR_SIZE);
            state = crc32_update_slice(state, &disk.sectors[lba][..take]);
            remaining -= take;
            lba += 1;
        }
        !state
    }

    fn reseal(disk: &mut TestDisk) {
        let array_crc = table_crc(
            disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            TEST_ENTRY_COUNT,
            TEST_ENTRY_SIZE,
        );
        write_header(
            &mut disk.sectors[GPT_HEADER_LBA as usize],
            GPT_HEADER_LBA,
            TEST_BACKUP_HEADER_LBA,
            PRIMARY_ENTRY_ARRAY_LBA,
            TEST_ENTRY_COUNT,
            TEST_ENTRY_SIZE,
            array_crc,
        );
        write_header(
            &mut disk.sectors[TEST_BACKUP_HEADER_LBA as usize],
            TEST_BACKUP_HEADER_LBA,
            GPT_HEADER_LBA,
            TEST_BACKUP_ARRAY_LBA,
            TEST_ENTRY_COUNT,
            TEST_ENTRY_SIZE,
            array_crc,
        );
    }

    fn write_header(
        sector: &mut Sector,
        current_lba: u64,
        backup_lba: u64,
        entry_lba: u64,
        entry_count: u32,
        entry_size: u32,
        array_crc: u32,
    ) {
        sector.fill(0);
        sector[..8].copy_from_slice(GPT_SIGNATURE);
        put_u32(sector, 8, GPT_REVISION_1_0);
        put_u32(sector, 12, GPT_HEADER_MIN_SIZE);
        put_u64(sector, 24, current_lba);
        put_u64(sector, 32, backup_lba);
        put_u64(sector, 40, TEST_FIRST_USABLE);
        put_u64(sector, 48, TEST_LAST_USABLE);
        sector[56..72].copy_from_slice(&TEST_DISK_GUID);
        put_u64(sector, 72, entry_lba);
        put_u32(sector, 80, entry_count);
        put_u32(sector, 84, entry_size);
        put_u32(sector, 88, array_crc);
        let crc = header_crc32(sector, GPT_HEADER_MIN_SIZE as usize);
        put_u32(sector, 16, crc);
    }

    fn reseal_header(sector: &mut Sector) {
        put_u32(sector, 16, 0);
        let size = le_u32(sector, 12) as usize;
        let crc = header_crc32(sector, size);
        put_u32(sector, 16, crc);
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn crc32_matches_the_ieee_reference_vector() {
        assert_eq!(!crc32_update_slice(!0, b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn validates_primary_backup_and_returns_fixed_name() {
        let mut disk = valid_disk();
        let evidence = find_bndroid_system_partition(&mut disk).unwrap();
        assert_eq!(evidence.device_sectors, TEST_SECTORS as u64);
        assert_eq!(evidence.primary_entry_array_lba, 2);
        assert_eq!(evidence.backup_entry_array_lba, 125);
        assert_eq!(evidence.partition.entry_index, 0);
        assert_eq!(evidence.partition.first_lba, 10);
        assert_eq!(evidence.partition.last_lba, 20);
        assert_eq!(evidence.partition.sector_count(), 11);
        assert_eq!(
            evidence.partition.name(),
            &[b's' as u16, b'y' as u16, b's' as u16]
        );
    }

    #[test]
    fn discovers_system_and_private_data_partitions_independently() {
        let mut disk = valid_disk();
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            1,
            BNDROID_DATA_PARTITION_TYPE_GUID,
            [0x33; 16],
            30,
            40,
            &[b'd' as u16, b'a' as u16, b't' as u16, b'a' as u16],
        );
        copy_table(&mut disk);
        reseal(&mut disk);

        let system = find_bndroid_system_partition(&mut disk).unwrap();
        let data = find_bndroid_data_partition(&mut disk).unwrap();
        assert_eq!(system.partition.entry_index, 0);
        assert_eq!(data.partition.entry_index, 1);
        assert_eq!(data.partition.first_lba, 30);
        assert_eq!(data.partition.last_lba, 40);
        assert_eq!(
            data.partition.name(),
            &[b'd' as u16, b'a' as u16, b't' as u16, b'a' as u16]
        );

        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            2,
            BNDROID_DATA_PARTITION_TYPE_GUID,
            [0x44; 16],
            50,
            60,
            &[],
        );
        copy_table(&mut disk);
        reseal(&mut disk);
        assert_eq!(
            find_bndroid_data_partition(&mut disk),
            Err(GptError::DuplicateTargetPartition)
        );
    }

    #[test]
    fn discovers_appdata_partition_at_index_two() {
        let mut disk = valid_disk();
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            1,
            BNDROID_DATA_PARTITION_TYPE_GUID,
            [0x33; 16],
            30,
            40,
            &[b'd' as u16, b'a' as u16, b't' as u16, b'a' as u16],
        );
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            2,
            BNDROID_APPDATA_PARTITION_TYPE_GUID,
            [0x54; 16],
            41,
            80,
            &[
                b'B' as u16,
                b'N' as u16,
                b'D' as u16,
                b'R' as u16,
                b'O' as u16,
                b'I' as u16,
                b'D' as u16,
                b'_' as u16,
                b'A' as u16,
                b'P' as u16,
                b'P' as u16,
                b'D' as u16,
                b'A' as u16,
                b'T' as u16,
                b'A' as u16,
            ],
        );
        copy_table(&mut disk);
        reseal(&mut disk);

        let system = find_bndroid_system_partition(&mut disk).unwrap();
        let data = find_bndroid_data_partition(&mut disk).unwrap();
        let appdata = find_appdata_partition(&mut disk).unwrap();
        assert_eq!(system.partition.entry_index, 0);
        assert_eq!(data.partition.entry_index, 1);
        assert_eq!(appdata.partition.entry_index, 2);
        assert_eq!(
            appdata.partition.type_guid,
            BNDROID_APPDATA_PARTITION_TYPE_GUID
        );
        assert_eq!(appdata.partition.unique_guid, [0x54; 16]);
        assert_eq!(appdata.partition.first_lba, 41);
        assert_eq!(appdata.partition.last_lba, 80);
        assert_eq!(appdata.partition.sector_count(), 40);
        assert_eq!(
            appdata.partition.name(),
            &[
                b'B' as u16,
                b'N' as u16,
                b'D' as u16,
                b'R' as u16,
                b'O' as u16,
                b'I' as u16,
                b'D' as u16,
                b'_' as u16,
                b'A' as u16,
                b'P' as u16,
                b'P' as u16,
                b'D' as u16,
                b'A' as u16,
                b'T' as u16,
                b'A' as u16,
            ]
        );
    }

    #[test]
    fn discovers_packages_partition_without_aliasing_appdata() {
        let mut disk = valid_disk();
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            2,
            BNDROID_APPDATA_PARTITION_TYPE_GUID,
            [0x54; 16],
            30,
            60,
            &[b'a' as u16, b'p' as u16, b'p' as u16],
        );
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            3,
            BNDROID_PACKAGES_PARTITION_TYPE_GUID,
            [0x81; 16],
            61,
            100,
            &[
                b'B' as u16,
                b'N' as u16,
                b'D' as u16,
                b'R' as u16,
                b'O' as u16,
                b'I' as u16,
                b'D' as u16,
                b'_' as u16,
                b'P' as u16,
                b'A' as u16,
                b'C' as u16,
                b'K' as u16,
                b'A' as u16,
                b'G' as u16,
                b'E' as u16,
                b'S' as u16,
            ],
        );
        copy_table(&mut disk);
        reseal(&mut disk);

        let appdata = find_appdata_partition(&mut disk).unwrap();
        let packages = find_packages_partition(&mut disk).unwrap();
        assert_eq!(appdata.partition.entry_index, 2);
        assert_eq!(packages.partition.entry_index, 3);
        assert_eq!(
            packages.partition.type_guid,
            BNDROID_PACKAGES_PARTITION_TYPE_GUID
        );
        assert_eq!(packages.partition.unique_guid, [0x81; 16]);
        assert_eq!(packages.partition.first_lba, 61);
        assert_eq!(packages.partition.last_lba, 100);
        assert_eq!(packages.partition.sector_count(), 40);
    }

    #[test]
    fn appdata_discovery_keeps_existing_fail_closed_boundaries() {
        let mut missing = valid_disk();
        assert_eq!(
            find_appdata_partition(&mut missing),
            Err(GptError::TargetPartitionNotFound)
        );

        let mut outside = valid_disk();
        write_entry(
            &mut outside,
            PRIMARY_ENTRY_ARRAY_LBA,
            2,
            BNDROID_APPDATA_PARTITION_TYPE_GUID,
            [0x54; 16],
            TEST_FIRST_USABLE - 1,
            40,
            &[],
        );
        copy_table(&mut outside);
        reseal(&mut outside);
        assert_eq!(
            find_appdata_partition(&mut outside),
            Err(GptError::PartitionOutsideUsableRange)
        );

        let mut duplicate = valid_disk();
        for (index, guid, first_lba, last_lba) in [(2, [0x54; 16], 30, 40), (3, [0x55; 16], 50, 60)]
        {
            write_entry(
                &mut duplicate,
                PRIMARY_ENTRY_ARRAY_LBA,
                index,
                BNDROID_APPDATA_PARTITION_TYPE_GUID,
                guid,
                first_lba,
                last_lba,
                &[],
            );
        }
        copy_table(&mut duplicate);
        reseal(&mut duplicate);
        assert_eq!(
            find_appdata_partition(&mut duplicate),
            Err(GptError::DuplicateTargetPartition)
        );

        let mut primary_crc = valid_disk();
        write_entry(
            &mut primary_crc,
            PRIMARY_ENTRY_ARRAY_LBA,
            2,
            BNDROID_APPDATA_PARTITION_TYPE_GUID,
            [0x54; 16],
            30,
            40,
            &[],
        );
        copy_table(&mut primary_crc);
        reseal(&mut primary_crc);
        primary_crc.sectors[PRIMARY_ENTRY_ARRAY_LBA as usize][2 * TEST_ENTRY_SIZE as usize + 56] =
            1;
        assert_eq!(
            find_appdata_partition(&mut primary_crc),
            Err(GptError::PrimaryEntryArrayCrc)
        );
    }

    #[test]
    fn rejects_bad_protective_mbr() {
        let mut bad_signature = valid_disk();
        bad_signature.sectors[0][510] = 0;
        assert_eq!(
            find_bndroid_system_partition(&mut bad_signature),
            Err(GptError::ProtectiveMbrSignature)
        );

        let mut bad_extent = valid_disk();
        put_u32(&mut bad_extent.sectors[0], 446 + 12, 7);
        assert_eq!(
            find_bndroid_system_partition(&mut bad_extent),
            Err(GptError::ProtectiveMbrLayout)
        );
    }

    #[test]
    fn rejects_primary_header_crc() {
        let mut disk = valid_disk();
        disk.sectors[1][40] ^= 1;
        assert_eq!(
            find_bndroid_system_partition(&mut disk),
            Err(GptError::PrimaryHeaderCrc)
        );
    }

    #[test]
    fn rejects_header_version_size_locations_and_usable_bounds() {
        let mut revision = valid_disk();
        put_u32(&mut revision.sectors[1], 8, 0x0002_0000);
        assert_eq!(
            find_bndroid_system_partition(&mut revision),
            Err(GptError::UnsupportedRevision)
        );

        let mut size = valid_disk();
        put_u32(&mut size.sectors[1], 12, GPT_HEADER_MIN_SIZE - 1);
        assert_eq!(
            find_bndroid_system_partition(&mut size),
            Err(GptError::InvalidHeaderSize)
        );

        let mut location = valid_disk();
        put_u64(&mut location.sectors[1], 24, 2);
        reseal_header(&mut location.sectors[1]);
        assert_eq!(
            find_bndroid_system_partition(&mut location),
            Err(GptError::PrimaryHeaderLayout)
        );

        let mut usable = valid_disk();
        put_u64(&mut usable.sectors[1], 40, TEST_LAST_USABLE + 1);
        reseal_header(&mut usable.sectors[1]);
        assert_eq!(
            find_bndroid_system_partition(&mut usable),
            Err(GptError::InvalidUsableRange)
        );
    }

    #[test]
    fn rejects_primary_entry_array_crc() {
        let mut disk = valid_disk();
        disk.sectors[2][100] ^= 1;
        assert_eq!(
            find_bndroid_system_partition(&mut disk),
            Err(GptError::PrimaryEntryArrayCrc)
        );
    }

    #[test]
    fn rejects_bad_backup_header_and_array() {
        let mut bad_header = valid_disk();
        bad_header.sectors[TEST_BACKUP_HEADER_LBA as usize][0] = 0;
        assert_eq!(
            find_bndroid_system_partition(&mut bad_header),
            Err(GptError::BackupHeaderSignature)
        );

        let mut bad_array = valid_disk();
        bad_array.sectors[TEST_BACKUP_ARRAY_LBA as usize][100] ^= 1;
        assert_eq!(
            find_bndroid_system_partition(&mut bad_array),
            Err(GptError::BackupEntryArrayCrc)
        );
    }

    #[test]
    fn rejects_partition_order_and_bounds() {
        let mut reversed = valid_disk();
        write_entry(
            &mut reversed,
            PRIMARY_ENTRY_ARRAY_LBA,
            0,
            BNDROID_SYSTEM_PARTITION_TYPE_GUID,
            TEST_UNIQUE_GUID,
            30,
            20,
            &[],
        );
        copy_table(&mut reversed);
        reseal(&mut reversed);
        assert_eq!(
            find_bndroid_system_partition(&mut reversed),
            Err(GptError::PartitionFirstAfterLast)
        );

        let mut outside = valid_disk();
        write_entry(
            &mut outside,
            PRIMARY_ENTRY_ARRAY_LBA,
            0,
            BNDROID_SYSTEM_PARTITION_TYPE_GUID,
            TEST_UNIQUE_GUID,
            3,
            20,
            &[],
        );
        copy_table(&mut outside);
        reseal(&mut outside);
        assert_eq!(
            find_bndroid_system_partition(&mut outside),
            Err(GptError::PartitionOutsideUsableRange)
        );
    }

    #[test]
    fn rejects_duplicate_target_partition() {
        let mut disk = valid_disk();
        let second_guid = [0x5a; 16];
        write_entry(
            &mut disk,
            PRIMARY_ENTRY_ARRAY_LBA,
            1,
            BNDROID_SYSTEM_PARTITION_TYPE_GUID,
            second_guid,
            30,
            40,
            &[],
        );
        copy_table(&mut disk);
        reseal(&mut disk);
        assert_eq!(
            find_bndroid_system_partition(&mut disk),
            Err(GptError::DuplicateTargetPartition)
        );
    }

    #[test]
    fn rejects_hostile_count_size_and_array_overflow() {
        let mut count = valid_disk();
        put_u32(&mut count.sectors[1], 80, u32::MAX);
        reseal_header(&mut count.sectors[1]);
        assert_eq!(
            find_bndroid_system_partition(&mut count),
            Err(GptError::InvalidEntryCount)
        );

        let mut size = valid_disk();
        put_u32(&mut size.sectors[1], 84, u32::MAX);
        reseal_header(&mut size.sectors[1]);
        assert_eq!(
            find_bndroid_system_partition(&mut size),
            Err(GptError::InvalidEntrySize)
        );

        let mut overflow = valid_disk();
        put_u64(
            &mut overflow.sectors[TEST_BACKUP_HEADER_LBA as usize],
            72,
            u64::MAX,
        );
        reseal_header(&mut overflow.sectors[TEST_BACKUP_HEADER_LBA as usize]);
        assert_eq!(
            find_bndroid_system_partition(&mut overflow),
            Err(GptError::EntryArrayOverflow)
        );
    }

    #[test]
    fn rejects_backup_layout_mismatch_and_missing_target() {
        let mut backup = valid_disk();
        put_u64(
            &mut backup.sectors[TEST_BACKUP_HEADER_LBA as usize],
            72,
            TEST_BACKUP_ARRAY_LBA - 1,
        );
        reseal_header(&mut backup.sectors[TEST_BACKUP_HEADER_LBA as usize]);
        assert_eq!(
            find_bndroid_system_partition(&mut backup),
            Err(GptError::BackupEntryArrayBounds)
        );

        let mut missing = valid_disk();
        write_entry(
            &mut missing,
            PRIMARY_ENTRY_ARRAY_LBA,
            0,
            [0x77; 16],
            TEST_UNIQUE_GUID,
            10,
            20,
            &[],
        );
        copy_table(&mut missing);
        reseal(&mut missing);
        assert_eq!(
            find_bndroid_system_partition(&mut missing),
            Err(GptError::TargetPartitionNotFound)
        );
    }
}
