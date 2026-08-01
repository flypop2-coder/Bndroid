use crate::digest::crc32;
#[cfg(feature = "androidbox-apk-envelope4")]
use crate::{AndroidBoxEnvelopeScratch, MAX_ENVELOPE_XML_INFLATED_BYTES};
use crate::{Error, MAX_APK_BYTES, MAX_DEX_BYTES, MAX_MANIFEST_BYTES};

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
#[cfg(feature = "androidbox-apk-envelope4")]
const DATA_DESCRIPTOR_SIGNATURE: u32 = 0x0807_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EXTRA_ID: u16 = 0x0001;
const EOCD_FIXED_SIZE: usize = 22;
const CENTRAL_FIXED_SIZE: usize = 46;
const LOCAL_FIXED_SIZE: usize = 30;
const MAX_COMMENT_BYTES: usize = u16::MAX as usize;
const MAX_ENTRIES: usize = 32;
const MAX_NAME_BYTES: usize = 128;
const FLAG_ENCRYPTED: u16 = 1 << 0;
const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;
const FLAG_UTF8: u16 = 1 << 11;
const STORED: u16 = 0;
#[cfg(feature = "androidbox-apk-envelope4")]
const DEFLATED: u16 = 8;
#[cfg(feature = "androidbox-apk-envelope4")]
const FLAG_DEFLATE_OPTION_1: u16 = 1 << 1;
#[cfg(feature = "androidbox-apk-envelope4")]
const FLAG_DEFLATE_OPTION_2: u16 = 1 << 2;
const CLASSES_DEX: &[u8] = b"classes.dex";
const ANDROID_MANIFEST: &[u8] = b"AndroidManifest.xml";
#[cfg(feature = "androidbox-apk-envelope4")]
const RESOURCES_ARSC: &[u8] = b"resources.arsc";

#[derive(Clone, Copy)]
pub(crate) struct StoredEntry<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) crc32: u32,
}

pub(crate) struct ApkEntries<'a> {
    pub(crate) dex: StoredEntry<'a>,
    pub(crate) manifest: StoredEntry<'a>,
}

#[cfg(feature = "androidbox-apk-envelope4")]
#[derive(Clone, Copy)]
pub(crate) struct EnvelopeEntry<'a> {
    compressed_bytes: &'a [u8],
    method: u16,
    crc32: u32,
    uncompressed_size: usize,
}

#[cfg(feature = "androidbox-apk-envelope4")]
impl EnvelopeEntry<'_> {
    pub(crate) const fn is_deflated(self) -> bool {
        self.method == DEFLATED
    }

    pub(crate) const fn crc32(self) -> u32 {
        self.crc32
    }
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) enum DecodedEntry<'apk, 'scratch> {
    Stored(&'apk [u8]),
    Inflated(&'scratch [u8]),
}

#[cfg(feature = "androidbox-apk-envelope4")]
impl DecodedEntry<'_, '_> {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Stored(bytes) => bytes,
            Self::Inflated(bytes) => bytes,
        }
    }
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) struct EnvelopeApkEntries<'a> {
    pub(crate) dex: StoredEntry<'a>,
    pub(crate) manifest: EnvelopeEntry<'a>,
    pub(crate) data_descriptor_count: u8,
    pub(crate) deflated_entry_count: u8,
}

#[derive(Clone, Copy)]
struct Directory {
    central_start: usize,
    central_end: usize,
    entry_count: usize,
}

#[derive(Clone, Copy)]
struct CentralEntry<'a> {
    flags: u16,
    method: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_offset: u32,
    name: &'a [u8],
}

#[cfg(feature = "androidbox-apk-envelope4")]
#[derive(Clone, Copy)]
struct LocalEntry<'a> {
    compressed_bytes: &'a [u8],
    record_start: usize,
    record_end: usize,
}

#[cfg(feature = "androidbox-apk-envelope4")]
#[derive(Clone, Copy)]
struct LocalRange {
    start: usize,
    end: usize,
}

#[cfg(feature = "androidbox-apk-envelope4")]
const EMPTY_LOCAL_RANGE: LocalRange = LocalRange { start: 0, end: 0 };

pub(crate) fn apk_entries(apk: &[u8]) -> Result<ApkEntries<'_>, Error> {
    let directory = archive_directory(apk)?;

    let mut cursor = directory.central_start;
    let mut dex: Option<StoredEntry<'_>> = None;
    let mut manifest: Option<StoredEntry<'_>> = None;
    let mut index = 0usize;
    while index < directory.entry_count {
        let (entry, next) = parse_central(apk, cursor, directory.central_end)?;
        cursor = next;
        let data = validate_local(apk, directory.central_start, entry)?;
        if entry.name == CLASSES_DEX {
            if dex.is_some() {
                return Err(Error::ZipDuplicateDex);
            }
            if entry.method != STORED {
                return Err(Error::ZipCompressedDex);
            }
            if data.len() > MAX_DEX_BYTES {
                return Err(Error::DexTooLarge);
            }
            dex = Some(StoredEntry {
                bytes: data,
                crc32: entry.crc32,
            });
        } else if entry.name == ANDROID_MANIFEST {
            if manifest.is_some() {
                return Err(Error::ZipDuplicateManifest);
            }
            if entry.method != STORED {
                return Err(Error::ZipCompressedManifest);
            }
            if data.len() > MAX_MANIFEST_BYTES {
                return Err(Error::ManifestTooLarge);
            }
            manifest = Some(StoredEntry {
                bytes: data,
                crc32: entry.crc32,
            });
        }
        index += 1;
    }
    if cursor != directory.central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    Ok(ApkEntries {
        dex: dex.ok_or(Error::ZipMissingDex)?,
        manifest: manifest.ok_or(Error::ZipMissingManifest)?,
    })
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) fn apk_entries_envelope(apk: &[u8]) -> Result<EnvelopeApkEntries<'_>, Error> {
    let directory = archive_directory(apk)?;
    let mut cursor = directory.central_start;
    let mut dex: Option<StoredEntry<'_>> = None;
    let mut manifest: Option<EnvelopeEntry<'_>> = None;
    let mut data_descriptor_count = 0u8;
    let mut deflated_entry_count = 0u8;
    let mut ranges = [EMPTY_LOCAL_RANGE; MAX_ENTRIES];
    let mut index = 0usize;
    while index < directory.entry_count {
        let (entry, next) = parse_central_envelope(apk, cursor, directory.central_end)?;
        cursor = next;
        let local = validate_local_envelope(apk, directory.central_start, entry)?;
        validate_nonoverlapping_range(
            &ranges[..index],
            LocalRange {
                start: local.record_start,
                end: local.record_end,
            },
        )?;
        ranges[index] = LocalRange {
            start: local.record_start,
            end: local.record_end,
        };
        if entry.flags & FLAG_DATA_DESCRIPTOR != 0 {
            data_descriptor_count = data_descriptor_count
                .checked_add(1)
                .ok_or(Error::ZipTooManyEntries)?;
        }
        if entry.method == DEFLATED {
            deflated_entry_count = deflated_entry_count
                .checked_add(1)
                .ok_or(Error::ZipTooManyEntries)?;
        }

        if entry.name == CLASSES_DEX {
            if dex.is_some() {
                return Err(Error::ZipDuplicateDex);
            }
            if entry.method != STORED {
                return Err(Error::ZipCompressedDex);
            }
            let uncompressed_size =
                usize::try_from(entry.uncompressed_size).map_err(|_| Error::DexTooLarge)?;
            if uncompressed_size > MAX_DEX_BYTES {
                return Err(Error::DexTooLarge);
            }
            dex = Some(StoredEntry {
                bytes: local.compressed_bytes,
                crc32: entry.crc32,
            });
        } else if entry.name == ANDROID_MANIFEST {
            if manifest.is_some() {
                return Err(Error::ZipDuplicateManifest);
            }
            let uncompressed_size =
                usize::try_from(entry.uncompressed_size).map_err(|_| Error::ManifestTooLarge)?;
            if uncompressed_size > MAX_MANIFEST_BYTES {
                return Err(Error::ManifestTooLarge);
            }
            manifest = Some(EnvelopeEntry {
                compressed_bytes: local.compressed_bytes,
                method: entry.method,
                crc32: entry.crc32,
                uncompressed_size,
            });
        } else if entry.name == RESOURCES_ARSC && entry.method != STORED {
            // ResourceTable keeps borrowing this entry for the lifetime of the
            // image, so Envelope-4 deliberately does not inflate it.
            return Err(Error::ZipCompressionUnsupported);
        }
        index += 1;
    }
    if cursor != directory.central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    Ok(EnvelopeApkEntries {
        dex: dex.ok_or(Error::ZipMissingDex)?,
        manifest: manifest.ok_or(Error::ZipMissingManifest)?,
        data_descriptor_count,
        deflated_entry_count,
    })
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) fn required_envelope_entry<'a>(
    apk: &'a [u8],
    name: &[u8],
    maximum_size: usize,
    missing: Error,
    duplicate: Error,
    too_large: Error,
) -> Result<EnvelopeEntry<'a>, Error> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(Error::ZipNameTooLong);
    }
    let directory = archive_directory(apk)?;
    let mut cursor = directory.central_start;
    let mut found: Option<EnvelopeEntry<'a>> = None;
    let mut ranges = [EMPTY_LOCAL_RANGE; MAX_ENTRIES];
    let mut index = 0usize;
    while index < directory.entry_count {
        let (entry, next) = parse_central_envelope(apk, cursor, directory.central_end)?;
        cursor = next;
        let local = validate_local_envelope(apk, directory.central_start, entry)?;
        validate_nonoverlapping_range(
            &ranges[..index],
            LocalRange {
                start: local.record_start,
                end: local.record_end,
            },
        )?;
        ranges[index] = LocalRange {
            start: local.record_start,
            end: local.record_end,
        };
        if entry.name == name {
            if found.is_some() {
                return Err(duplicate);
            }
            let uncompressed_size =
                usize::try_from(entry.uncompressed_size).map_err(|_| too_large)?;
            if uncompressed_size > maximum_size {
                return Err(too_large);
            }
            found = Some(EnvelopeEntry {
                compressed_bytes: local.compressed_bytes,
                method: entry.method,
                crc32: entry.crc32,
                uncompressed_size,
            });
        }
        index += 1;
    }
    if cursor != directory.central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    found.ok_or(missing)
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) fn required_envelope_stored_entry<'a>(
    apk: &'a [u8],
    name: &[u8],
    maximum_size: usize,
    missing: Error,
    duplicate: Error,
    too_large: Error,
) -> Result<StoredEntry<'a>, Error> {
    let entry = required_envelope_entry(apk, name, maximum_size, missing, duplicate, too_large)?;
    if entry.method != STORED {
        return Err(Error::ZipCompressionUnsupported);
    }
    Ok(StoredEntry {
        bytes: entry.compressed_bytes,
        crc32: entry.crc32,
    })
}

#[cfg(feature = "androidbox-apk-envelope4")]
pub(crate) fn decode_envelope_entry<'apk, 'scratch>(
    entry: EnvelopeEntry<'apk>,
    scratch: &'scratch mut AndroidBoxEnvelopeScratch,
) -> Result<DecodedEntry<'apk, 'scratch>, Error> {
    if entry.method == STORED {
        return Ok(DecodedEntry::Stored(entry.compressed_bytes));
    }
    if entry.method != DEFLATED {
        return Err(Error::ZipCompressionUnsupported);
    }
    if entry.uncompressed_size > MAX_ENVELOPE_XML_INFLATED_BYTES {
        return Err(Error::ZipInflatedEntryTooLarge);
    }

    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{
        decompress, inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    };

    let (decompressor, output_buffer) = scratch.inflate_parts_mut();
    let (status, input_read, output_written) = decompress(
        decompressor,
        entry.compressed_bytes,
        output_buffer,
        0,
        TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    if status != TINFLStatus::Done
        || input_read != entry.compressed_bytes.len()
        || output_written != entry.uncompressed_size
    {
        return Err(Error::ZipDeflateInvalid);
    }
    let output = &output_buffer[..output_written];
    if crc32(output) != entry.crc32 {
        return Err(Error::ZipCrcMismatch);
    }
    Ok(DecodedEntry::Inflated(output))
}

pub(crate) fn required_stored_entry<'a>(
    apk: &'a [u8],
    name: &[u8],
    maximum_size: usize,
    missing: Error,
    duplicate: Error,
    too_large: Error,
) -> Result<StoredEntry<'a>, Error> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(Error::ZipNameTooLong);
    }
    let directory = archive_directory(apk)?;
    let mut cursor = directory.central_start;
    let mut found: Option<StoredEntry<'a>> = None;
    let mut index = 0usize;
    while index < directory.entry_count {
        let (entry, next) = parse_central(apk, cursor, directory.central_end)?;
        cursor = next;
        let data = validate_local(apk, directory.central_start, entry)?;
        if entry.name == name {
            if found.is_some() {
                return Err(duplicate);
            }
            if data.len() > maximum_size {
                return Err(too_large);
            }
            found = Some(StoredEntry {
                bytes: data,
                crc32: entry.crc32,
            });
        }
        index += 1;
    }
    if cursor != directory.central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    found.ok_or(missing)
}

fn archive_directory(apk: &[u8]) -> Result<Directory, Error> {
    if apk.len() > MAX_APK_BYTES {
        return Err(Error::ApkTooLarge);
    }
    let eocd = find_eocd(apk)?;
    if eocd >= 20 && read_u32(apk, eocd - 20)? == ZIP64_LOCATOR_SIGNATURE {
        return Err(Error::Zip64Unsupported);
    }

    let disk = read_u16(apk, eocd + 4)?;
    let central_disk = read_u16(apk, eocd + 6)?;
    let entries_on_disk = read_u16(apk, eocd + 8)?;
    let entries_total = read_u16(apk, eocd + 10)?;
    let central_size = read_u32(apk, eocd + 12)?;
    let central_offset = read_u32(apk, eocd + 16)?;
    if disk != 0 || central_disk != 0 || entries_on_disk != entries_total {
        return Err(Error::ZipEndRecordInvalid);
    }
    if entries_total == u16::MAX || central_size == u32::MAX || central_offset == u32::MAX {
        return Err(Error::Zip64Unsupported);
    }
    let entry_count = usize::from(entries_total);
    if entry_count == 0 || entry_count > MAX_ENTRIES {
        return Err(Error::ZipTooManyEntries);
    }
    let central_start =
        usize::try_from(central_offset).map_err(|_| Error::ZipCentralDirectoryBounds)?;
    let central_bytes =
        usize::try_from(central_size).map_err(|_| Error::ZipCentralDirectoryBounds)?;
    let central_end = checked_add(
        central_start,
        central_bytes,
        Error::ZipCentralDirectoryBounds,
    )?;
    if central_end != eocd {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    Ok(Directory {
        central_start,
        central_end,
        entry_count,
    })
}

fn find_eocd(bytes: &[u8]) -> Result<usize, Error> {
    if bytes.len() < EOCD_FIXED_SIZE {
        return Err(Error::ZipEndRecordMissing);
    }
    let lower = bytes
        .len()
        .saturating_sub(EOCD_FIXED_SIZE + MAX_COMMENT_BYTES);
    let mut cursor = bytes.len() - EOCD_FIXED_SIZE;
    loop {
        if read_u32(bytes, cursor)? == EOCD_SIGNATURE {
            let comment_len = usize::from(read_u16(bytes, cursor + 20)?);
            if checked_add(
                cursor + EOCD_FIXED_SIZE,
                comment_len,
                Error::ZipEndRecordInvalid,
            )? == bytes.len()
            {
                return Ok(cursor);
            }
        }
        if cursor == lower {
            break;
        }
        cursor -= 1;
    }
    Err(Error::ZipEndRecordMissing)
}

fn parse_central<'a>(
    bytes: &'a [u8],
    offset: usize,
    central_end: usize,
) -> Result<(CentralEntry<'a>, usize), Error> {
    let fixed_end = checked_add(offset, CENTRAL_FIXED_SIZE, Error::ZipCentralDirectoryBounds)?;
    if fixed_end > central_end || read_u32(bytes, offset)? != CENTRAL_SIGNATURE {
        return Err(Error::ZipCentralHeaderInvalid);
    }
    let flags = read_u16(bytes, offset + 8)?;
    validate_flags(flags)?;
    let method = read_u16(bytes, offset + 10)?;
    let crc = read_u32(bytes, offset + 16)?;
    let compressed_size = read_u32(bytes, offset + 20)?;
    let uncompressed_size = read_u32(bytes, offset + 24)?;
    let name_len = usize::from(read_u16(bytes, offset + 28)?);
    let extra_len = usize::from(read_u16(bytes, offset + 30)?);
    let comment_len = usize::from(read_u16(bytes, offset + 32)?);
    let disk_start = read_u16(bytes, offset + 34)?;
    let local_offset = read_u32(bytes, offset + 42)?;
    if compressed_size == u32::MAX
        || uncompressed_size == u32::MAX
        || local_offset == u32::MAX
        || disk_start == u16::MAX
    {
        return Err(Error::Zip64Unsupported);
    }
    if disk_start != 0 {
        return Err(Error::ZipEndRecordInvalid);
    }
    if name_len == 0 || name_len > MAX_NAME_BYTES {
        return Err(Error::ZipNameTooLong);
    }
    if method != STORED {
        let name_start = fixed_end;
        let name_end = checked_add(name_start, name_len, Error::ZipCentralDirectoryBounds)?;
        if name_end <= central_end {
            let name = &bytes[name_start..name_end];
            if name == CLASSES_DEX {
                return Err(Error::ZipCompressedDex);
            }
            if name == ANDROID_MANIFEST {
                return Err(Error::ZipCompressedManifest);
            }
        }
        return Err(Error::ZipCompressionUnsupported);
    }
    if compressed_size != uncompressed_size {
        return Err(Error::ZipEntryMismatch);
    }

    let name_start = fixed_end;
    let name_end = checked_add(name_start, name_len, Error::ZipCentralDirectoryBounds)?;
    let extra_end = checked_add(name_end, extra_len, Error::ZipCentralDirectoryBounds)?;
    let next = checked_add(extra_end, comment_len, Error::ZipCentralDirectoryBounds)?;
    if next > central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    validate_extra(&bytes[name_end..extra_end])?;
    Ok((
        CentralEntry {
            flags,
            method,
            crc32: crc,
            compressed_size,
            uncompressed_size,
            local_offset,
            name: &bytes[name_start..name_end],
        },
        next,
    ))
}

#[cfg(feature = "androidbox-apk-envelope4")]
fn parse_central_envelope<'a>(
    bytes: &'a [u8],
    offset: usize,
    central_end: usize,
) -> Result<(CentralEntry<'a>, usize), Error> {
    let fixed_end = checked_add(offset, CENTRAL_FIXED_SIZE, Error::ZipCentralDirectoryBounds)?;
    if fixed_end > central_end || read_u32(bytes, offset)? != CENTRAL_SIGNATURE {
        return Err(Error::ZipCentralHeaderInvalid);
    }
    let flags = read_u16(bytes, offset + 8)?;
    let method = read_u16(bytes, offset + 10)?;
    validate_envelope_flags(flags, method)?;
    let crc = read_u32(bytes, offset + 16)?;
    let compressed_size = read_u32(bytes, offset + 20)?;
    let uncompressed_size = read_u32(bytes, offset + 24)?;
    let name_len = usize::from(read_u16(bytes, offset + 28)?);
    let extra_len = usize::from(read_u16(bytes, offset + 30)?);
    let comment_len = usize::from(read_u16(bytes, offset + 32)?);
    let disk_start = read_u16(bytes, offset + 34)?;
    let local_offset = read_u32(bytes, offset + 42)?;
    if compressed_size == u32::MAX
        || uncompressed_size == u32::MAX
        || local_offset == u32::MAX
        || disk_start == u16::MAX
    {
        return Err(Error::Zip64Unsupported);
    }
    if disk_start != 0 {
        return Err(Error::ZipEndRecordInvalid);
    }
    if name_len == 0 || name_len > MAX_NAME_BYTES {
        return Err(Error::ZipNameTooLong);
    }
    if method != STORED && method != DEFLATED {
        return Err(Error::ZipCompressionUnsupported);
    }
    if method == STORED && compressed_size != uncompressed_size {
        return Err(Error::ZipEntryMismatch);
    }

    let name_start = fixed_end;
    let name_end = checked_add(name_start, name_len, Error::ZipCentralDirectoryBounds)?;
    let extra_end = checked_add(name_end, extra_len, Error::ZipCentralDirectoryBounds)?;
    let next = checked_add(extra_end, comment_len, Error::ZipCentralDirectoryBounds)?;
    if next > central_end {
        return Err(Error::ZipCentralDirectoryBounds);
    }
    validate_extra(&bytes[name_end..extra_end])?;
    Ok((
        CentralEntry {
            flags,
            method,
            crc32: crc,
            compressed_size,
            uncompressed_size,
            local_offset,
            name: &bytes[name_start..name_end],
        },
        next,
    ))
}

#[cfg(feature = "androidbox-apk-envelope4")]
fn validate_local_envelope<'a>(
    bytes: &'a [u8],
    central_start: usize,
    central: CentralEntry<'_>,
) -> Result<LocalEntry<'a>, Error> {
    let offset = usize::try_from(central.local_offset).map_err(|_| Error::ZipEntryBounds)?;
    let fixed_end = checked_add(offset, LOCAL_FIXED_SIZE, Error::ZipEntryBounds)?;
    if fixed_end > central_start || read_u32(bytes, offset)? != LOCAL_SIGNATURE {
        return Err(Error::ZipLocalHeaderInvalid);
    }
    let flags = read_u16(bytes, offset + 6)?;
    let method = read_u16(bytes, offset + 8)?;
    validate_envelope_flags(flags, method)?;
    let crc = read_u32(bytes, offset + 14)?;
    let compressed_size = read_u32(bytes, offset + 18)?;
    let uncompressed_size = read_u32(bytes, offset + 22)?;
    let name_len = usize::from(read_u16(bytes, offset + 26)?);
    let extra_len = usize::from(read_u16(bytes, offset + 28)?);
    if compressed_size == u32::MAX || uncompressed_size == u32::MAX {
        return Err(Error::Zip64Unsupported);
    }
    if flags != central.flags || method != central.method || name_len != central.name.len() {
        return Err(Error::ZipEntryMismatch);
    }
    let has_descriptor = flags & FLAG_DATA_DESCRIPTOR != 0;
    if has_descriptor {
        let local_tuple_is_zero = crc == 0 && compressed_size == 0 && uncompressed_size == 0;
        let local_tuple_matches_central = crc == central.crc32
            && compressed_size == central.compressed_size
            && uncompressed_size == central.uncompressed_size;
        if !local_tuple_is_zero && !local_tuple_matches_central {
            return Err(Error::ZipDataDescriptorInvalid);
        }
    } else if crc != central.crc32
        || compressed_size != central.compressed_size
        || uncompressed_size != central.uncompressed_size
    {
        return Err(Error::ZipEntryMismatch);
    }

    let name_end = checked_add(fixed_end, name_len, Error::ZipEntryBounds)?;
    let extra_end = checked_add(name_end, extra_len, Error::ZipEntryBounds)?;
    if extra_end > central_start || &bytes[fixed_end..name_end] != central.name {
        return Err(Error::ZipEntryMismatch);
    }
    validate_extra(&bytes[name_end..extra_end])?;
    let data_len = usize::try_from(central.compressed_size).map_err(|_| Error::ZipEntryBounds)?;
    let data_end = checked_add(extra_end, data_len, Error::ZipEntryBounds)?;
    if data_end > central_start {
        return Err(Error::ZipEntryBounds);
    }
    let record_end = if has_descriptor {
        validate_data_descriptor(bytes, data_end, central_start, central)?
    } else {
        data_end
    };
    let data = &bytes[extra_end..data_end];
    if central.method == STORED && crc32(data) != central.crc32 {
        return Err(Error::ZipCrcMismatch);
    }
    Ok(LocalEntry {
        compressed_bytes: data,
        record_start: offset,
        record_end,
    })
}

#[cfg(feature = "androidbox-apk-envelope4")]
fn validate_data_descriptor(
    bytes: &[u8],
    offset: usize,
    central_start: usize,
    central: CentralEntry<'_>,
) -> Result<usize, Error> {
    let has_signature = read_u32(bytes, offset).ok() == Some(DATA_DESCRIPTOR_SIGNATURE);
    let values_start = if has_signature {
        checked_add(offset, 4, Error::ZipDataDescriptorInvalid)?
    } else {
        offset
    };
    let descriptor_end = checked_add(values_start, 12, Error::ZipDataDescriptorInvalid)?;
    if descriptor_end > central_start {
        return Err(Error::ZipDataDescriptorInvalid);
    }
    let crc = read_u32(bytes, values_start).map_err(|_| Error::ZipDataDescriptorInvalid)?;
    let compressed_size =
        read_u32(bytes, values_start + 4).map_err(|_| Error::ZipDataDescriptorInvalid)?;
    let uncompressed_size =
        read_u32(bytes, values_start + 8).map_err(|_| Error::ZipDataDescriptorInvalid)?;
    if crc != central.crc32
        || compressed_size != central.compressed_size
        || uncompressed_size != central.uncompressed_size
    {
        return Err(Error::ZipDataDescriptorInvalid);
    }
    Ok(descriptor_end)
}

#[cfg(feature = "androidbox-apk-envelope4")]
fn validate_envelope_flags(flags: u16, method: u16) -> Result<(), Error> {
    if flags & FLAG_ENCRYPTED != 0 {
        return Err(Error::ZipEncrypted);
    }
    let deflate_options = FLAG_DEFLATE_OPTION_1 | FLAG_DEFLATE_OPTION_2;
    let allowed = FLAG_UTF8
        | FLAG_DATA_DESCRIPTOR
        | if method == DEFLATED {
            deflate_options
        } else {
            0
        };
    if flags & !allowed != 0 || (method != DEFLATED && flags & deflate_options != 0) {
        return Err(Error::ZipEntryMismatch);
    }
    Ok(())
}

#[cfg(feature = "androidbox-apk-envelope4")]
fn validate_nonoverlapping_range(previous: &[LocalRange], next: LocalRange) -> Result<(), Error> {
    if next.start >= next.end {
        return Err(Error::ZipEntryBounds);
    }
    for range in previous {
        if next.start < range.end && range.start < next.end {
            return Err(Error::ZipEntryOverlap);
        }
    }
    Ok(())
}

fn validate_local<'a>(
    bytes: &'a [u8],
    central_start: usize,
    central: CentralEntry<'_>,
) -> Result<&'a [u8], Error> {
    let offset = usize::try_from(central.local_offset).map_err(|_| Error::ZipEntryBounds)?;
    let fixed_end = checked_add(offset, LOCAL_FIXED_SIZE, Error::ZipEntryBounds)?;
    if fixed_end > central_start || read_u32(bytes, offset)? != LOCAL_SIGNATURE {
        return Err(Error::ZipLocalHeaderInvalid);
    }
    let flags = read_u16(bytes, offset + 6)?;
    validate_flags(flags)?;
    let method = read_u16(bytes, offset + 8)?;
    let crc = read_u32(bytes, offset + 14)?;
    let compressed_size = read_u32(bytes, offset + 18)?;
    let uncompressed_size = read_u32(bytes, offset + 22)?;
    let name_len = usize::from(read_u16(bytes, offset + 26)?);
    let extra_len = usize::from(read_u16(bytes, offset + 28)?);
    if compressed_size == u32::MAX || uncompressed_size == u32::MAX {
        return Err(Error::Zip64Unsupported);
    }
    if flags != central.flags
        || method != central.method
        || crc != central.crc32
        || compressed_size != central.compressed_size
        || uncompressed_size != central.uncompressed_size
        || name_len != central.name.len()
    {
        return Err(Error::ZipEntryMismatch);
    }
    let name_end = checked_add(fixed_end, name_len, Error::ZipEntryBounds)?;
    let extra_end = checked_add(name_end, extra_len, Error::ZipEntryBounds)?;
    if extra_end > central_start || &bytes[fixed_end..name_end] != central.name {
        return Err(Error::ZipEntryMismatch);
    }
    validate_extra(&bytes[name_end..extra_end])?;
    let data_len = usize::try_from(compressed_size).map_err(|_| Error::ZipEntryBounds)?;
    let data_end = checked_add(extra_end, data_len, Error::ZipEntryBounds)?;
    if data_end > central_start {
        return Err(Error::ZipEntryBounds);
    }
    let data = &bytes[extra_end..data_end];
    if crc32(data) != crc {
        return Err(Error::ZipCrcMismatch);
    }
    Ok(data)
}

fn validate_flags(flags: u16) -> Result<(), Error> {
    if flags & FLAG_ENCRYPTED != 0 {
        return Err(Error::ZipEncrypted);
    }
    if flags & FLAG_DATA_DESCRIPTOR != 0 {
        return Err(Error::ZipDataDescriptorUnsupported);
    }
    if flags & !FLAG_UTF8 != 0 {
        return Err(Error::ZipEntryMismatch);
    }
    Ok(())
}

fn validate_extra(extra: &[u8]) -> Result<(), Error> {
    let mut cursor = 0usize;
    while cursor < extra.len() {
        if extra.len() - cursor < 4 {
            // Android's zipalign may use up to three raw zero bytes in the
            // local extra area to align stored entry data. No value is decoded
            // from this padding, and non-zero truncated fields remain invalid.
            return if extra[cursor..].iter().all(|byte| *byte == 0) {
                Ok(())
            } else {
                Err(Error::ZipExtraInvalid)
            };
        }
        let identifier = u16::from_le_bytes([extra[cursor], extra[cursor + 1]]);
        let size = usize::from(u16::from_le_bytes([extra[cursor + 2], extra[cursor + 3]]));
        cursor += 4;
        let end = cursor.checked_add(size).ok_or(Error::ZipExtraInvalid)?;
        if end > extra.len() {
            return Err(Error::ZipExtraInvalid);
        }
        if identifier == ZIP64_EXTRA_ID {
            return Err(Error::Zip64Unsupported);
        }
        cursor = end;
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    let end = offset.checked_add(2).ok_or(Error::ZipEntryBounds)?;
    let value = bytes.get(offset..end).ok_or(Error::ZipEntryBounds)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(Error::ZipEntryBounds)?;
    let value = bytes.get(offset..end).ok_or(Error::ZipEntryBounds)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(all(test, feature = "androidbox-apk-envelope4"))]
mod envelope_tests {
    use super::{EMPTY_LOCAL_RANGE, Error, LocalRange, validate_nonoverlapping_range};

    #[test]
    fn local_ranges_reject_same_and_partial_overlap() {
        let previous = [LocalRange { start: 10, end: 30 }];
        assert_eq!(
            validate_nonoverlapping_range(&previous, LocalRange { start: 10, end: 30 }),
            Err(Error::ZipEntryOverlap)
        );
        assert_eq!(
            validate_nonoverlapping_range(&previous, LocalRange { start: 20, end: 40 }),
            Err(Error::ZipEntryOverlap)
        );
        assert_eq!(
            validate_nonoverlapping_range(&previous, LocalRange { start: 0, end: 20 }),
            Err(Error::ZipEntryOverlap)
        );
        assert_eq!(
            validate_nonoverlapping_range(&previous, LocalRange { start: 30, end: 40 }),
            Ok(())
        );
        assert_eq!(
            validate_nonoverlapping_range(&[], EMPTY_LOCAL_RANGE),
            Err(Error::ZipEntryBounds)
        );
    }
}

fn checked_add(left: usize, right: usize, error: Error) -> Result<usize, Error> {
    left.checked_add(right).ok_or(error)
}
