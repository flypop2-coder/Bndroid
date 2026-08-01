#![no_std]

use core::ops::Range;

const ELF_HEADER_SIZE: usize = 64;
const PROGRAM_HEADER_SIZE: usize = 56;
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LITTLE_ENDIAN: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_OS_ABI_SYSTEM_V: u8 = 0;
const ELF_ABI_VERSION_NONE: u8 = 0;
const ELF_TYPE_EXECUTABLE: u16 = 2;
const ELF_MACHINE_AARCH64: u16 = 183;
const PROGRAM_TYPE_NULL: u32 = 0;
const PROGRAM_TYPE_LOAD: u32 = 1;
const PROGRAM_TYPE_DYNAMIC: u32 = 2;
const PROGRAM_TYPE_INTERPRETER: u32 = 3;
const PROGRAM_TYPE_TLS: u32 = 7;
const PROGRAM_TYPE_GNU_STACK: u32 = 0x6474_e551;
const PROGRAM_FLAG_EXECUTE: u32 = 1;
const PROGRAM_FLAG_WRITE: u32 = 2;
const PROGRAM_FLAG_READ: u32 = 4;
const PROGRAM_FLAGS_KNOWN: u32 = PROGRAM_FLAG_EXECUTE | PROGRAM_FLAG_WRITE | PROGRAM_FLAG_READ;

pub const MAX_ELF_FILE_SIZE: usize = 2 * 1024 * 1024;
pub const MAX_PROGRAM_HEADERS: usize = 8;
pub const MAX_LOAD_PAGES: usize = 256;
pub const ELF_PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfError {
    TruncatedHeader,
    FileTooLarge,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndianness,
    UnsupportedIdentVersion,
    UnsupportedOsAbi,
    UnsupportedAbiVersion,
    InvalidIdentPadding,
    UnsupportedType,
    UnsupportedMachine,
    UnsupportedVersion,
    UnsupportedFlags,
    BadHeaderSize,
    BadProgramHeaderSize,
    MisalignedEntry,
    MissingProgramHeaders,
    TooManyProgramHeaders,
    ProgramHeaderTableOverflow,
    ProgramHeaderTableOverlapsHeader,
    ProgramHeaderTableOutOfBounds,
    DynamicSegmentUnsupported,
    InterpreterUnsupported,
    TlsUnsupported,
    UnsupportedProgramHeaderType,
    DuplicateGnuStack,
    InvalidStackFlags,
    ExecutableStack,
    MissingLoadSegments,
    EmptyLoadSegment,
    SegmentFileLargerThanMemory,
    SegmentFileOutOfBounds,
    SegmentVirtualAddressOverflow,
    InvalidSegmentAlignment,
    InvalidSegmentFlags,
    WritableExecutableSegment,
    LoadSegmentsOutOfOrder,
    LoadPagesOverlap,
    TooManyLoadPages,
    EntryNotExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentFlags(u32);

impl SegmentFlags {
    pub const fn readable(self) -> bool {
        self.0 & PROGRAM_FLAG_READ != 0
    }

    pub const fn writable(self) -> bool {
        self.0 & PROGRAM_FLAG_WRITE != 0
    }

    pub const fn executable(self) -> bool {
        self.0 & PROGRAM_FLAG_EXECUTE != 0
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadSegment<'a> {
    virtual_address: u64,
    memory_size: u64,
    alignment: u64,
    flags: SegmentFlags,
    file_data: &'a [u8],
}

impl<'a> LoadSegment<'a> {
    pub const fn virtual_address(self) -> u64 {
        self.virtual_address
    }

    pub const fn memory_size(self) -> u64 {
        self.memory_size
    }

    pub const fn alignment(self) -> u64 {
        self.alignment
    }

    pub const fn flags(self) -> SegmentFlags {
        self.flags
    }

    pub const fn file_data(self) -> &'a [u8] {
        self.file_data
    }

    pub fn virtual_range(self) -> Range<u64> {
        self.virtual_address..self.virtual_address + self.memory_size
    }

    pub fn page_range(self) -> Range<u64> {
        let start = align_down(self.virtual_address, ELF_PAGE_SIZE);
        let end = align_up(self.virtual_address + self.memory_size, ELF_PAGE_SIZE)
            .expect("validated ELF segment page range overflowed");
        start..end
    }

    pub fn file_backed_virtual_range(self) -> Range<u64> {
        let end = self
            .virtual_address
            .checked_add(self.file_data.len() as u64)
            .expect("validated ELF segment file-backed range overflowed");
        self.virtual_address..end
    }

    pub fn zero_fill_virtual_range(self) -> Range<u64> {
        self.file_backed_virtual_range().end..self.virtual_range().end
    }

    pub fn page_count(self) -> usize {
        let page_range = self.page_range();
        let pages = (page_range.end - page_range.start) / ELF_PAGE_SIZE;
        usize::try_from(pages).expect("validated ELF segment page count does not fit usize")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElfImage<'a> {
    bytes: &'a [u8],
    entry: u64,
    program_header_offset: usize,
    program_header_count: usize,
    load_segment_count: usize,
    load_page_count: usize,
}

impl<'a> ElfImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ElfError> {
        if bytes.len() > MAX_ELF_FILE_SIZE {
            return Err(ElfError::FileTooLarge);
        }
        if bytes.len() < ELF_HEADER_SIZE {
            return Err(ElfError::TruncatedHeader);
        }
        if bytes[0..4] != [0x7f, b'E', b'L', b'F'] {
            return Err(ElfError::BadMagic);
        }
        if bytes[4] != ELF_CLASS_64 {
            return Err(ElfError::UnsupportedClass);
        }
        if bytes[5] != ELF_DATA_LITTLE_ENDIAN {
            return Err(ElfError::UnsupportedEndianness);
        }
        if bytes[6] != ELF_VERSION_CURRENT {
            return Err(ElfError::UnsupportedIdentVersion);
        }
        if bytes[7] != ELF_OS_ABI_SYSTEM_V {
            return Err(ElfError::UnsupportedOsAbi);
        }
        if bytes[8] != ELF_ABI_VERSION_NONE {
            return Err(ElfError::UnsupportedAbiVersion);
        }
        if bytes[9..16].iter().any(|byte| *byte != 0) {
            return Err(ElfError::InvalidIdentPadding);
        }
        if read_u16(bytes, 16) != Some(ELF_TYPE_EXECUTABLE) {
            return Err(ElfError::UnsupportedType);
        }
        if read_u16(bytes, 18) != Some(ELF_MACHINE_AARCH64) {
            return Err(ElfError::UnsupportedMachine);
        }
        if read_u32(bytes, 20) != Some(u32::from(ELF_VERSION_CURRENT)) {
            return Err(ElfError::UnsupportedVersion);
        }
        if read_u32(bytes, 48) != Some(0) {
            return Err(ElfError::UnsupportedFlags);
        }
        if read_u16(bytes, 52) != Some(ELF_HEADER_SIZE as u16) {
            return Err(ElfError::BadHeaderSize);
        }
        if read_u16(bytes, 54) != Some(PROGRAM_HEADER_SIZE as u16) {
            return Err(ElfError::BadProgramHeaderSize);
        }

        let entry = read_u64(bytes, 24).unwrap_or(0);
        if !entry.is_multiple_of(4) {
            return Err(ElfError::MisalignedEntry);
        }

        let program_header_count = usize::from(read_u16(bytes, 56).unwrap_or(0));
        if program_header_count == 0 {
            return Err(ElfError::MissingProgramHeaders);
        }
        if program_header_count > MAX_PROGRAM_HEADERS {
            return Err(ElfError::TooManyProgramHeaders);
        }
        let program_header_offset = usize::try_from(read_u64(bytes, 32).unwrap_or(u64::MAX))
            .map_err(|_| ElfError::ProgramHeaderTableOverflow)?;
        if program_header_offset < ELF_HEADER_SIZE {
            return Err(ElfError::ProgramHeaderTableOverlapsHeader);
        }
        let table_bytes = PROGRAM_HEADER_SIZE
            .checked_mul(program_header_count)
            .ok_or(ElfError::ProgramHeaderTableOverflow)?;
        let table_end = program_header_offset
            .checked_add(table_bytes)
            .ok_or(ElfError::ProgramHeaderTableOverflow)?;
        if table_end > bytes.len() {
            return Err(ElfError::ProgramHeaderTableOutOfBounds);
        }

        let mut load_segment_count = 0_usize;
        let mut load_page_count = 0_usize;
        let mut entry_is_executable = false;
        let mut previous_load_address = None;
        let mut saw_gnu_stack = false;
        for index in 0..program_header_count {
            let header = program_header_at(program_header_offset, index)?;
            let program_type =
                read_u32(bytes, header).ok_or(ElfError::ProgramHeaderTableOutOfBounds)?;
            let segment = match program_type {
                PROGRAM_TYPE_NULL => continue,
                PROGRAM_TYPE_LOAD => parse_load_segment_at(bytes, header)?,
                PROGRAM_TYPE_GNU_STACK => {
                    if saw_gnu_stack {
                        return Err(ElfError::DuplicateGnuStack);
                    }
                    validate_gnu_stack(bytes, header)?;
                    saw_gnu_stack = true;
                    continue;
                }
                PROGRAM_TYPE_DYNAMIC => return Err(ElfError::DynamicSegmentUnsupported),
                PROGRAM_TYPE_INTERPRETER => return Err(ElfError::InterpreterUnsupported),
                PROGRAM_TYPE_TLS => return Err(ElfError::TlsUnsupported),
                _ => return Err(ElfError::UnsupportedProgramHeaderType),
            };
            if previous_load_address.is_some_and(|previous| segment.virtual_address < previous) {
                return Err(ElfError::LoadSegmentsOutOfOrder);
            }
            previous_load_address = Some(segment.virtual_address);
            load_segment_count += 1;
            let segment_pages = segment.page_count();
            load_page_count = load_page_count
                .checked_add(segment_pages)
                .ok_or(ElfError::TooManyLoadPages)?;
            if load_page_count > MAX_LOAD_PAGES {
                return Err(ElfError::TooManyLoadPages);
            }
            if segment.flags.executable() && segment.file_backed_virtual_range().contains(&entry) {
                entry_is_executable = true;
            }
            for previous in 0..index {
                let Some(previous) = parse_load_segment(bytes, program_header_offset, previous)?
                else {
                    continue;
                };
                if ranges_overlap(segment.page_range(), previous.page_range()) {
                    return Err(ElfError::LoadPagesOverlap);
                }
            }
        }
        if load_segment_count == 0 {
            return Err(ElfError::MissingLoadSegments);
        }
        if !entry_is_executable {
            return Err(ElfError::EntryNotExecutable);
        }

        Ok(Self {
            bytes,
            entry,
            program_header_offset,
            program_header_count,
            load_segment_count,
            load_page_count,
        })
    }

    pub const fn entry(&self) -> u64 {
        self.entry
    }

    pub const fn load_segment_count(&self) -> usize {
        self.load_segment_count
    }

    pub const fn load_page_count(&self) -> usize {
        self.load_page_count
    }

    pub fn load_segments(&self) -> LoadSegments<'_, 'a> {
        LoadSegments {
            image: self,
            next_program_header: 0,
        }
    }
}

pub struct LoadSegments<'image, 'data> {
    image: &'image ElfImage<'data>,
    next_program_header: usize,
}

impl<'data> Iterator for LoadSegments<'_, 'data> {
    type Item = LoadSegment<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next_program_header < self.image.program_header_count {
            let index = self.next_program_header;
            self.next_program_header += 1;
            match parse_load_segment(self.image.bytes, self.image.program_header_offset, index) {
                Ok(Some(segment)) => return Some(segment),
                Ok(None) => {}
                Err(_) => unreachable!("ElfImage retained an invalid program header"),
            }
        }
        None
    }
}

fn parse_load_segment<'a>(
    bytes: &'a [u8],
    table_offset: usize,
    index: usize,
) -> Result<Option<LoadSegment<'a>>, ElfError> {
    let offset = program_header_at(table_offset, index)?;
    if read_u32(bytes, offset) != Some(PROGRAM_TYPE_LOAD) {
        return Ok(None);
    }
    parse_load_segment_at(bytes, offset).map(Some)
}

fn parse_load_segment_at(bytes: &[u8], offset: usize) -> Result<LoadSegment<'_>, ElfError> {
    let raw_flags = read_u32(bytes, offset + 4).unwrap_or(u32::MAX);
    if raw_flags & !PROGRAM_FLAGS_KNOWN != 0 || raw_flags & PROGRAM_FLAG_READ == 0 {
        return Err(ElfError::InvalidSegmentFlags);
    }
    if raw_flags & (PROGRAM_FLAG_WRITE | PROGRAM_FLAG_EXECUTE)
        == (PROGRAM_FLAG_WRITE | PROGRAM_FLAG_EXECUTE)
    {
        return Err(ElfError::WritableExecutableSegment);
    }

    let file_offset = read_u64(bytes, offset + 8).unwrap_or(u64::MAX);
    let virtual_address = read_u64(bytes, offset + 16).unwrap_or(u64::MAX);
    let file_size = read_u64(bytes, offset + 32).unwrap_or(u64::MAX);
    let memory_size = read_u64(bytes, offset + 40).unwrap_or(u64::MAX);
    let alignment = read_u64(bytes, offset + 48).unwrap_or(u64::MAX);
    if memory_size == 0 {
        return Err(ElfError::EmptyLoadSegment);
    }
    if file_size > memory_size {
        return Err(ElfError::SegmentFileLargerThanMemory);
    }
    let virtual_end = virtual_address
        .checked_add(memory_size)
        .ok_or(ElfError::SegmentVirtualAddressOverflow)?;
    align_up(virtual_end, ELF_PAGE_SIZE).ok_or(ElfError::SegmentVirtualAddressOverflow)?;
    if alignment < ELF_PAGE_SIZE
        || !alignment.is_power_of_two()
        || !file_offset.is_multiple_of(ELF_PAGE_SIZE)
        || !virtual_address.is_multiple_of(ELF_PAGE_SIZE)
        || file_offset % alignment != virtual_address % alignment
    {
        return Err(ElfError::InvalidSegmentAlignment);
    }
    let file_start = usize::try_from(file_offset).map_err(|_| ElfError::SegmentFileOutOfBounds)?;
    let file_len = usize::try_from(file_size).map_err(|_| ElfError::SegmentFileOutOfBounds)?;
    let file_end = file_start
        .checked_add(file_len)
        .ok_or(ElfError::SegmentFileOutOfBounds)?;
    let file_data = bytes
        .get(file_start..file_end)
        .ok_or(ElfError::SegmentFileOutOfBounds)?;

    Ok(LoadSegment {
        virtual_address,
        memory_size,
        alignment,
        flags: SegmentFlags(raw_flags),
        file_data,
    })
}

fn validate_gnu_stack(bytes: &[u8], offset: usize) -> Result<(), ElfError> {
    let raw_flags = read_u32(bytes, offset + 4).unwrap_or(u32::MAX);
    if raw_flags & !PROGRAM_FLAGS_KNOWN != 0 {
        return Err(ElfError::InvalidStackFlags);
    }
    if raw_flags & PROGRAM_FLAG_EXECUTE != 0 {
        return Err(ElfError::ExecutableStack);
    }
    Ok(())
}

fn program_header_at(table_offset: usize, index: usize) -> Result<usize, ElfError> {
    let relative = index
        .checked_mul(PROGRAM_HEADER_SIZE)
        .ok_or(ElfError::ProgramHeaderTableOverflow)?;
    table_offset
        .checked_add(relative)
        .ok_or(ElfError::ProgramHeaderTableOverflow)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    Some(u64::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

const fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    value
        .checked_add(alignment - 1)
        .map(|end| align_down(end, alignment))
}

fn ranges_overlap(left: Range<u64>, right: Range<u64>) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec;
    use std::vec::Vec;

    use super::{
        ElfError, ElfImage, MAX_ELF_FILE_SIZE, MAX_LOAD_PAGES, MAX_PROGRAM_HEADERS,
        PROGRAM_TYPE_DYNAMIC, PROGRAM_TYPE_GNU_STACK, PROGRAM_TYPE_INTERPRETER, PROGRAM_TYPE_NULL,
        PROGRAM_TYPE_TLS,
    };

    const TEXT_VA: u64 = 0x1800_0000;
    const DATA_VA: u64 = 0x1800_2000;

    #[derive(Clone, Copy)]
    struct TestSegment {
        flags: u32,
        offset: u64,
        virtual_address: u64,
        file_size: u64,
        memory_size: u64,
        alignment: u64,
    }

    fn valid_elf() -> Vec<u8> {
        build_elf(
            TEXT_VA,
            &[
                TestSegment {
                    flags: 5,
                    offset: 0x1000,
                    virtual_address: TEXT_VA,
                    file_size: 8,
                    memory_size: 8,
                    alignment: 0x1000,
                },
                TestSegment {
                    flags: 6,
                    offset: 0x2000,
                    virtual_address: DATA_VA,
                    file_size: 4,
                    memory_size: 0x1000,
                    alignment: 0x1000,
                },
            ],
        )
    }

    #[test]
    fn parses_aarch64_executable_and_yields_load_segments() {
        let bytes = valid_elf();
        let image = ElfImage::parse(&bytes).unwrap();
        assert_eq!(image.entry(), TEXT_VA);
        assert_eq!(image.load_segment_count(), 2);
        assert_eq!(image.load_page_count(), 2);
        let segments: Vec<_> = image.load_segments().collect();
        assert!(segments[0].flags().readable());
        assert!(segments[0].flags().executable());
        assert!(!segments[0].flags().writable());
        assert_eq!(segments[0].file_data(), &[0xa5; 8]);
        assert_eq!(segments[1].file_data(), &[0xa5; 4]);
        assert_eq!(segments[1].memory_size(), 0x1000);
    }

    #[test]
    fn parses_three_multpage_segments_with_exact_permissions_and_bss_ranges() {
        let rodata_va = TEXT_VA + 0x4000;
        let writable_va = TEXT_VA + 0x8000;
        let bytes = build_elf(
            TEXT_VA,
            &[
                TestSegment {
                    flags: 5,
                    offset: 0x1000,
                    virtual_address: TEXT_VA,
                    file_size: 0x1800,
                    memory_size: 0x2000,
                    alignment: 0x1000,
                },
                TestSegment {
                    flags: 4,
                    offset: 0x3000,
                    virtual_address: rodata_va,
                    file_size: 0x1400,
                    memory_size: 0x2000,
                    alignment: 0x1000,
                },
                TestSegment {
                    flags: 6,
                    offset: 0x5000,
                    virtual_address: writable_va,
                    file_size: 0x1001,
                    memory_size: 0x3000,
                    alignment: 0x1000,
                },
            ],
        );

        let image = ElfImage::parse(&bytes).unwrap();
        assert_eq!(image.load_segment_count(), 3);
        assert_eq!(image.load_page_count(), 7);
        let segments: Vec<_> = image.load_segments().collect();

        assert_eq!(segments[0].flags().bits(), 5);
        assert!(segments[0].flags().readable());
        assert!(!segments[0].flags().writable());
        assert!(segments[0].flags().executable());
        assert_eq!(segments[0].page_count(), 2);
        assert_eq!(
            segments[0].file_backed_virtual_range(),
            TEXT_VA..TEXT_VA + 0x1800
        );
        assert_eq!(
            segments[0].zero_fill_virtual_range(),
            TEXT_VA + 0x1800..TEXT_VA + 0x2000
        );

        assert_eq!(segments[1].flags().bits(), 4);
        assert!(segments[1].flags().readable());
        assert!(!segments[1].flags().writable());
        assert!(!segments[1].flags().executable());
        assert_eq!(segments[1].page_count(), 2);
        assert_eq!(
            segments[1].file_backed_virtual_range(),
            rodata_va..rodata_va + 0x1400
        );
        assert_eq!(
            segments[1].zero_fill_virtual_range(),
            rodata_va + 0x1400..rodata_va + 0x2000
        );

        assert_eq!(segments[2].flags().bits(), 6);
        assert!(segments[2].flags().readable());
        assert!(segments[2].flags().writable());
        assert!(!segments[2].flags().executable());
        assert_eq!(segments[2].page_count(), 3);
        assert_eq!(
            segments[2].file_backed_virtual_range(),
            writable_va..writable_va + 0x1001
        );
        assert_eq!(
            segments[2].zero_fill_virtual_range(),
            writable_va + 0x1001..writable_va + 0x3000
        );
    }

    #[test]
    fn rejects_wrong_identity_and_architecture() {
        let mut bytes = valid_elf();
        bytes[0] = 0;
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::BadMagic));
        let mut bytes = valid_elf();
        write_u16(&mut bytes, 18, 62);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedMachine));
    }

    #[test]
    fn rejects_truncated_or_overflowing_program_header_tables() {
        assert_eq!(ElfImage::parse(&[0; 8]), Err(ElfError::TruncatedHeader));
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 32, u64::MAX);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::ProgramHeaderTableOverflow)
        );
        let mut bytes = valid_elf();
        let truncated_table_offset = bytes.len() as u64 - 8;
        write_u64(&mut bytes, 32, truncated_table_offset);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::ProgramHeaderTableOutOfBounds)
        );
    }

    #[test]
    fn rejects_file_ranges_outside_the_image() {
        let mut bytes = valid_elf();
        let out_of_bounds = 0x3000;
        write_u64(&mut bytes, 64 + 8, out_of_bounds);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::SegmentFileOutOfBounds)
        );
    }

    #[test]
    fn rejects_writable_executable_or_unknown_permissions() {
        let mut bytes = valid_elf();
        write_u32(&mut bytes, 64 + 4, 7);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::WritableExecutableSegment)
        );
        let mut bytes = valid_elf();
        write_u32(&mut bytes, 64 + 4, 0x8000_0005);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::InvalidSegmentFlags));
    }

    #[test]
    fn rejects_file_larger_than_memory_and_virtual_overflow() {
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 40, 4);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::SegmentFileLargerThanMemory)
        );
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 16, u64::MAX - 3);
        write_u64(&mut bytes, 64 + 40, 8);
        write_u64(&mut bytes, 64 + 48, 1);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::SegmentVirtualAddressOverflow)
        );
    }

    #[test]
    fn rejects_overlapping_load_pages() {
        let bytes = build_elf(
            TEXT_VA,
            &[
                TestSegment {
                    flags: 5,
                    offset: 0x1000,
                    virtual_address: TEXT_VA,
                    file_size: 0x800,
                    memory_size: 0x2000,
                    alignment: 0x1000,
                },
                TestSegment {
                    flags: 6,
                    offset: 0x2000,
                    virtual_address: TEXT_VA + 0x1000,
                    file_size: 0x100,
                    memory_size: 0x100,
                    alignment: 0x1000,
                },
            ],
        );
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::LoadPagesOverlap));
    }

    #[test]
    fn requires_entry_inside_an_executable_segment() {
        let bytes = build_elf(
            DATA_VA,
            &[TestSegment {
                flags: 6,
                offset: 0x1000,
                virtual_address: DATA_VA,
                file_size: 8,
                memory_size: 8,
                alignment: 0x1000,
            }],
        );
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::EntryNotExecutable));
    }

    #[test]
    fn validates_offset_address_alignment_congruence() {
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 16, TEXT_VA + 1);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::InvalidSegmentAlignment)
        );
    }

    #[test]
    fn rejects_every_non_profile_ident_and_header_field() {
        let mut bytes = valid_elf();
        bytes[4] = 1;
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedClass));

        let mut bytes = valid_elf();
        bytes[5] = 2;
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::UnsupportedEndianness)
        );

        let mut bytes = valid_elf();
        bytes[6] = 0;
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::UnsupportedIdentVersion)
        );

        let mut bytes = valid_elf();
        bytes[7] = 3;
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedOsAbi));

        let mut bytes = valid_elf();
        bytes[8] = 1;
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::UnsupportedAbiVersion)
        );

        let mut bytes = valid_elf();
        bytes[9] = 1;
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::InvalidIdentPadding));

        let mut bytes = valid_elf();
        write_u16(&mut bytes, 16, 3);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedType));

        let mut bytes = valid_elf();
        write_u32(&mut bytes, 20, 0);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedVersion));

        let mut bytes = valid_elf();
        write_u32(&mut bytes, 48, 1);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::UnsupportedFlags));

        let mut bytes = valid_elf();
        write_u16(&mut bytes, 52, 63);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::BadHeaderSize));

        let mut bytes = valid_elf();
        write_u16(&mut bytes, 54, 55);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::BadProgramHeaderSize));
    }

    #[test]
    fn enforces_file_and_program_header_hard_limits() {
        let valid = valid_elf();
        for length in 0..64 {
            assert_eq!(
                ElfImage::parse(&valid[..length]),
                Err(ElfError::TruncatedHeader)
            );
        }

        let mut exact_maximum = valid_elf();
        exact_maximum.resize(MAX_ELF_FILE_SIZE, 0);
        assert!(ElfImage::parse(&exact_maximum).is_ok());
        exact_maximum.push(0);
        assert_eq!(ElfImage::parse(&exact_maximum), Err(ElfError::FileTooLarge));

        let mut bytes = valid_elf();
        write_u16(&mut bytes, 56, 0);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::MissingProgramHeaders)
        );

        let mut bytes = valid_elf();
        write_u16(&mut bytes, 56, MAX_PROGRAM_HEADERS as u16);
        assert!(ElfImage::parse(&bytes).is_ok());
        write_u16(&mut bytes, 56, (MAX_PROGRAM_HEADERS + 1) as u16);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::TooManyProgramHeaders)
        );

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 32, 0);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::ProgramHeaderTableOverlapsHeader)
        );
    }

    #[test]
    fn accepts_only_null_load_and_non_executable_gnu_stack_headers() {
        let second_header = 64 + 56;

        let mut bytes = valid_elf();
        write_u32(&mut bytes, second_header, PROGRAM_TYPE_NULL);
        let image = ElfImage::parse(&bytes).unwrap();
        assert_eq!(image.load_segment_count(), 1);

        let mut bytes = valid_elf();
        write_u32(&mut bytes, second_header, PROGRAM_TYPE_GNU_STACK);
        write_u32(&mut bytes, second_header + 4, 6);
        let image = ElfImage::parse(&bytes).unwrap();
        assert_eq!(image.load_segment_count(), 1);

        for (program_type, expected) in [
            (PROGRAM_TYPE_DYNAMIC, ElfError::DynamicSegmentUnsupported),
            (PROGRAM_TYPE_INTERPRETER, ElfError::InterpreterUnsupported),
            (PROGRAM_TYPE_TLS, ElfError::TlsUnsupported),
            (0x6474_e552, ElfError::UnsupportedProgramHeaderType),
        ] {
            let mut bytes = valid_elf();
            write_u32(&mut bytes, second_header, program_type);
            assert_eq!(ElfImage::parse(&bytes), Err(expected));
        }

        let mut bytes = valid_elf();
        write_u32(&mut bytes, second_header, PROGRAM_TYPE_GNU_STACK);
        write_u32(&mut bytes, second_header + 4, 7);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::ExecutableStack));

        let mut bytes = valid_elf();
        write_u32(&mut bytes, second_header, PROGRAM_TYPE_GNU_STACK);
        write_u32(&mut bytes, second_header + 4, 0x8000_0006);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::InvalidStackFlags));
    }

    #[test]
    fn rejects_duplicate_gnu_stack_and_images_without_loads() {
        let segments = [
            TestSegment {
                flags: 5,
                offset: 0x1000,
                virtual_address: TEXT_VA,
                file_size: 8,
                memory_size: 8,
                alignment: 0x1000,
            },
            TestSegment {
                flags: 6,
                offset: 0x2000,
                virtual_address: DATA_VA,
                file_size: 4,
                memory_size: 4,
                alignment: 0x1000,
            },
            TestSegment {
                flags: 6,
                offset: 0x3000,
                virtual_address: DATA_VA + 0x2000,
                file_size: 4,
                memory_size: 4,
                alignment: 0x1000,
            },
        ];
        let mut bytes = build_elf(TEXT_VA, &segments);
        for index in [1, 2] {
            let header = 64 + index * 56;
            write_u32(&mut bytes, header, PROGRAM_TYPE_GNU_STACK);
            write_u32(&mut bytes, header + 4, 6);
        }
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::DuplicateGnuStack));

        let mut bytes = valid_elf();
        write_u32(&mut bytes, 64, PROGRAM_TYPE_NULL);
        write_u32(&mut bytes, 64 + 56, PROGRAM_TYPE_NULL);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::MissingLoadSegments));
    }

    #[test]
    fn entry_must_be_aligned_and_inside_file_backed_rx_bytes() {
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 24, TEXT_VA + 2);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::MisalignedEntry));

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 40, 0x1000);
        write_u64(&mut bytes, 24, TEXT_VA + 8);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::EntryNotExecutable));

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 24, TEXT_VA + 4);
        assert!(ElfImage::parse(&bytes).is_ok());
    }

    #[test]
    fn load_permissions_are_exact_read_only_read_write_or_read_execute() {
        for flags in [0, 1, 2] {
            let mut bytes = valid_elf();
            write_u32(&mut bytes, 64 + 4, flags);
            assert_eq!(ElfImage::parse(&bytes), Err(ElfError::InvalidSegmentFlags));
        }

        let mut bytes = valid_elf();
        write_u32(&mut bytes, 64 + 56 + 4, 4);
        assert!(ElfImage::parse(&bytes).is_ok());
    }

    #[test]
    fn enforces_page_alignment_power_of_two_and_congruence() {
        for alignment in [0, 1, 0x1800] {
            let mut bytes = valid_elf();
            write_u64(&mut bytes, 64 + 48, alignment);
            assert_eq!(
                ElfImage::parse(&bytes),
                Err(ElfError::InvalidSegmentAlignment)
            );
        }

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 8, 0x1001);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::InvalidSegmentAlignment)
        );

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 16, TEXT_VA + 1);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::InvalidSegmentAlignment)
        );

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 48, 0x2000);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::InvalidSegmentAlignment)
        );
    }

    #[test]
    fn rejects_empty_loads_align_up_overflow_and_out_of_order_loads() {
        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 40, 0);
        assert_eq!(ElfImage::parse(&bytes), Err(ElfError::EmptyLoadSegment));

        let mut bytes = valid_elf();
        write_u64(&mut bytes, 64 + 16, u64::MAX & !0xfff);
        write_u64(&mut bytes, 64 + 40, 0xfff);
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::SegmentVirtualAddressOverflow)
        );

        let bytes = build_elf(
            TEXT_VA,
            &[
                TestSegment {
                    flags: 6,
                    offset: 0x1000,
                    virtual_address: DATA_VA,
                    file_size: 4,
                    memory_size: 4,
                    alignment: 0x1000,
                },
                TestSegment {
                    flags: 5,
                    offset: 0x2000,
                    virtual_address: TEXT_VA,
                    file_size: 8,
                    memory_size: 8,
                    alignment: 0x1000,
                },
            ],
        );
        assert_eq!(
            ElfImage::parse(&bytes),
            Err(ElfError::LoadSegmentsOutOfOrder)
        );
    }

    #[test]
    fn enforces_total_load_page_limit() {
        let exact = build_elf(
            TEXT_VA,
            &[TestSegment {
                flags: 5,
                offset: 0x1000,
                virtual_address: TEXT_VA,
                file_size: 8,
                memory_size: MAX_LOAD_PAGES as u64 * 0x1000,
                alignment: 0x1000,
            }],
        );
        let image = ElfImage::parse(&exact).unwrap();
        assert_eq!(image.load_page_count(), MAX_LOAD_PAGES);

        let too_many = build_elf(
            TEXT_VA,
            &[TestSegment {
                flags: 5,
                offset: 0x1000,
                virtual_address: TEXT_VA,
                file_size: 8,
                memory_size: (MAX_LOAD_PAGES as u64 + 1) * 0x1000,
                alignment: 0x1000,
            }],
        );
        assert_eq!(ElfImage::parse(&too_many), Err(ElfError::TooManyLoadPages));
    }

    fn build_elf(entry: u64, segments: &[TestSegment]) -> Vec<u8> {
        let file_end = segments
            .iter()
            .map(|segment| segment.offset + segment.file_size)
            .max()
            .unwrap_or(64 + 56 * segments.len() as u64);
        let mut bytes = vec![
            0;
            usize::try_from(file_end)
                .unwrap()
                .max(64 + 56 * segments.len())
        ];
        bytes[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        write_u16(&mut bytes, 16, 2);
        write_u16(&mut bytes, 18, 183);
        write_u32(&mut bytes, 20, 1);
        write_u64(&mut bytes, 24, entry);
        write_u64(&mut bytes, 32, 64);
        write_u16(&mut bytes, 52, 64);
        write_u16(&mut bytes, 54, 56);
        write_u16(&mut bytes, 56, segments.len() as u16);
        for (index, segment) in segments.iter().enumerate() {
            let header = 64 + index * 56;
            write_u32(&mut bytes, header, 1);
            write_u32(&mut bytes, header + 4, segment.flags);
            write_u64(&mut bytes, header + 8, segment.offset);
            write_u64(&mut bytes, header + 16, segment.virtual_address);
            write_u64(&mut bytes, header + 32, segment.file_size);
            write_u64(&mut bytes, header + 40, segment.memory_size);
            write_u64(&mut bytes, header + 48, segment.alignment);
            let start = usize::try_from(segment.offset).unwrap();
            let end = start + usize::try_from(segment.file_size).unwrap();
            bytes[start..end].fill(0xa5);
        }
        bytes
    }

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
