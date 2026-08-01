//! Allocation-free FAT16 mount, lookup, and read support.

use crate::block::{BlockReadError, BlockReader, SECTOR_SIZE, Sector};

const DIRECTORY_ENTRY_SIZE: usize = 32;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_LONG_NAME: u8 = 0x0f;
const FAT16_MIN_CLUSTERS: u32 = 4_085;
const FAT16_MAX_CLUSTERS: u32 = 65_524;
const MAX_DIRECTORY_CLUSTERS: usize = 64;
const MAX_FILE_CLUSTERS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fat16Evidence {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub sectors_per_fat: u16,
    pub root_entries: u16,
    pub root_directory_sectors: u32,
    pub cluster_count: u32,
    pub first_data_lba: u64,
    pub mirror_verified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fat16NodeKind {
    File,
    Directory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fat16Metadata {
    pub kind: Fat16NodeKind,
    pub size: u32,
    pub first_cluster: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fat16Error {
    BlockRead(BlockReadError),
    PartitionTooSmall,
    InvalidBootSignature,
    UnsupportedBytesPerSector,
    InvalidSectorsPerCluster,
    InvalidReservedSectors,
    InvalidFatCount,
    InvalidRootEntries,
    InvalidTotalSectors,
    InvalidFatSize,
    InvalidHiddenSectors,
    LayoutOverflow,
    LayoutOutsidePartition,
    NotFat16,
    FatTooSmall,
    FatMirrorMismatch,
    InvalidFatReservedEntries,
    PathMustBeAbsolute,
    InvalidPath,
    PathTooDeep,
    InvalidShortName,
    NotFound,
    NotDirectory,
    IsDirectory,
    InvalidDirectoryEntry,
    InvalidCluster,
    BadCluster,
    ClusterChainCycle,
    ClusterChainTooLong,
    TruncatedClusterChain,
    OutputTooSmall,
}

impl Fat16Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BlockRead(error) => error.as_str(),
            Self::PartitionTooSmall => "partition is too small for FAT16",
            Self::InvalidBootSignature => "FAT boot signature is invalid",
            Self::UnsupportedBytesPerSector => "FAT sector size is unsupported",
            Self::InvalidSectorsPerCluster => "FAT sectors-per-cluster is invalid",
            Self::InvalidReservedSectors => "FAT reserved-sector count is invalid",
            Self::InvalidFatCount => "FAT copy count is invalid",
            Self::InvalidRootEntries => "FAT root-entry count is invalid",
            Self::InvalidTotalSectors => "FAT total-sector count is invalid",
            Self::InvalidFatSize => "FAT size is invalid",
            Self::InvalidHiddenSectors => "FAT hidden-sector count disagrees with the partition",
            Self::LayoutOverflow => "FAT layout arithmetic overflowed",
            Self::LayoutOutsidePartition => "FAT layout is outside the partition",
            Self::NotFat16 => "FAT cluster count does not identify FAT16",
            Self::FatTooSmall => "FAT cannot address every data cluster",
            Self::FatMirrorMismatch => "FAT copies disagree",
            Self::InvalidFatReservedEntries => "FAT reserved entries are invalid",
            Self::PathMustBeAbsolute => "FAT path must be absolute",
            Self::InvalidPath => "FAT path is not canonical",
            Self::PathTooDeep => "FAT path has too many components",
            Self::InvalidShortName => "FAT path component is not a valid 8.3 name",
            Self::NotFound => "FAT path was not found",
            Self::NotDirectory => "FAT path crosses a non-directory",
            Self::IsDirectory => "FAT path names a directory",
            Self::InvalidDirectoryEntry => "FAT directory entry is invalid",
            Self::InvalidCluster => "FAT cluster number is invalid",
            Self::BadCluster => "FAT chain contains a bad or reserved cluster",
            Self::ClusterChainCycle => "FAT cluster chain contains a cycle",
            Self::ClusterChainTooLong => "FAT cluster chain exceeds its fixed bound",
            Self::TruncatedClusterChain => "FAT cluster chain ends before the file size",
            Self::OutputTooSmall => "file output buffer is too small",
        }
    }
}

impl From<BlockReadError> for Fat16Error {
    fn from(error: BlockReadError) -> Self {
        Self::BlockRead(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fat16 {
    evidence: Fat16Evidence,
    first_fat_lba: u64,
    second_fat_lba: u64,
    root_directory_lba: u64,
    max_cluster: u16,
}

impl Fat16 {
    pub fn mount<R: BlockReader>(
        reader: &mut R,
        partition_first_lba: u64,
        partition_sectors: u64,
    ) -> Result<Self, Fat16Error> {
        if partition_sectors < 4 || partition_first_lba >= reader.sector_count() {
            return Err(Fat16Error::PartitionTooSmall);
        }
        let partition_end = partition_first_lba
            .checked_add(partition_sectors)
            .ok_or(Fat16Error::LayoutOverflow)?;
        if partition_end > reader.sector_count() {
            return Err(Fat16Error::LayoutOutsidePartition);
        }

        let mut boot = [0_u8; SECTOR_SIZE];
        reader.read_sector(partition_first_lba, &mut boot)?;
        if boot[510..512] != [0x55, 0xaa] {
            return Err(Fat16Error::InvalidBootSignature);
        }
        let bytes_per_sector = le_u16(&boot, 11);
        if usize::from(bytes_per_sector) != SECTOR_SIZE {
            return Err(Fat16Error::UnsupportedBytesPerSector);
        }
        let sectors_per_cluster = boot[13];
        if sectors_per_cluster == 0
            || !sectors_per_cluster.is_power_of_two()
            || sectors_per_cluster > 128
        {
            return Err(Fat16Error::InvalidSectorsPerCluster);
        }
        let reserved_sectors = le_u16(&boot, 14);
        if reserved_sectors == 0 {
            return Err(Fat16Error::InvalidReservedSectors);
        }
        let fat_count = boot[16];
        if fat_count != 2 {
            return Err(Fat16Error::InvalidFatCount);
        }
        let root_entries = le_u16(&boot, 17);
        if root_entries == 0
            || !(usize::from(root_entries) * DIRECTORY_ENTRY_SIZE).is_multiple_of(SECTOR_SIZE)
        {
            return Err(Fat16Error::InvalidRootEntries);
        }
        let total16 = u32::from(le_u16(&boot, 19));
        let total32 = le_u32(&boot, 32);
        let total_sectors = match (total16, total32) {
            (0, value) if value != 0 => value,
            (value, 0) if value != 0 => value,
            _ => return Err(Fat16Error::InvalidTotalSectors),
        };
        if u64::from(total_sectors) != partition_sectors {
            return Err(Fat16Error::InvalidTotalSectors);
        }
        let sectors_per_fat = le_u16(&boot, 22);
        if sectors_per_fat == 0 {
            return Err(Fat16Error::InvalidFatSize);
        }
        if u64::from(le_u32(&boot, 28)) != partition_first_lba {
            return Err(Fat16Error::InvalidHiddenSectors);
        }

        let root_bytes = u32::from(root_entries)
            .checked_mul(DIRECTORY_ENTRY_SIZE as u32)
            .ok_or(Fat16Error::LayoutOverflow)?;
        let root_directory_sectors = root_bytes.div_ceil(SECTOR_SIZE as u32);
        let fat_area = u32::from(fat_count)
            .checked_mul(u32::from(sectors_per_fat))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let overhead = u32::from(reserved_sectors)
            .checked_add(fat_area)
            .and_then(|value| value.checked_add(root_directory_sectors))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let data_sectors = total_sectors
            .checked_sub(overhead)
            .ok_or(Fat16Error::LayoutOutsidePartition)?;
        let cluster_count = data_sectors / u32::from(sectors_per_cluster);
        if !(FAT16_MIN_CLUSTERS..=FAT16_MAX_CLUSTERS).contains(&cluster_count) {
            return Err(Fat16Error::NotFat16);
        }
        let fat_entries = u32::from(sectors_per_fat)
            .checked_mul(SECTOR_SIZE as u32)
            .ok_or(Fat16Error::LayoutOverflow)?
            / 2;
        if fat_entries < cluster_count + 2 {
            return Err(Fat16Error::FatTooSmall);
        }

        let first_fat_lba = partition_first_lba
            .checked_add(u64::from(reserved_sectors))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let second_fat_lba = first_fat_lba
            .checked_add(u64::from(sectors_per_fat))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let root_directory_lba = second_fat_lba
            .checked_add(u64::from(sectors_per_fat))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let first_data_lba = root_directory_lba
            .checked_add(u64::from(root_directory_sectors))
            .ok_or(Fat16Error::LayoutOverflow)?;
        let layout_end = first_data_lba
            .checked_add(u64::from(data_sectors))
            .ok_or(Fat16Error::LayoutOverflow)?;
        if layout_end > partition_end {
            return Err(Fat16Error::LayoutOutsidePartition);
        }

        let mut first = [0_u8; SECTOR_SIZE];
        let mut second = [0_u8; SECTOR_SIZE];
        reader.read_sector(first_fat_lba, &mut first)?;
        reader.read_sector(second_fat_lba, &mut second)?;
        if first != second {
            return Err(Fat16Error::FatMirrorMismatch);
        }
        let media = boot[21];
        if (le_u16(&first, 0) & 0xff) != u16::from(media)
            || le_u16(&first, 0) < 0xfff8
            || le_u16(&first, 2) < 0xfff8
        {
            return Err(Fat16Error::InvalidFatReservedEntries);
        }

        let max_cluster_u32 = cluster_count + 1;
        let max_cluster = u16::try_from(max_cluster_u32).map_err(|_| Fat16Error::NotFat16)?;
        Ok(Self {
            evidence: Fat16Evidence {
                partition_first_lba,
                partition_sectors,
                bytes_per_sector,
                sectors_per_cluster,
                reserved_sectors,
                fat_count,
                sectors_per_fat,
                root_entries,
                root_directory_sectors,
                cluster_count,
                first_data_lba,
                mirror_verified: true,
            },
            first_fat_lba,
            second_fat_lba,
            root_directory_lba,
            max_cluster,
        })
    }

    pub const fn evidence(&self) -> Fat16Evidence {
        self.evidence
    }

    pub fn lookup<R: BlockReader>(
        &self,
        reader: &mut R,
        path: &str,
    ) -> Result<Fat16Metadata, Fat16Error> {
        if path == "/" {
            return Ok(Fat16Metadata {
                kind: Fat16NodeKind::Directory,
                size: 0,
                first_cluster: 0,
            });
        }
        if !path.starts_with('/') {
            return Err(Fat16Error::PathMustBeAbsolute);
        }
        if path.ends_with('/') || path.contains("//") {
            return Err(Fat16Error::InvalidPath);
        }

        let mut directory_cluster = None;
        let mut result = None;
        let mut count = 0_usize;
        let mut components = path[1..].split('/').peekable();
        while let Some(component) = components.next() {
            count += 1;
            if count > 16 {
                return Err(Fat16Error::PathTooDeep);
            }
            let short_name = encode_short_name(component)?;
            let entry = self.find_in_directory(reader, directory_cluster, short_name)?;
            if components.peek().is_some() {
                if entry.kind != Fat16NodeKind::Directory {
                    return Err(Fat16Error::NotDirectory);
                }
                self.validate_cluster(entry.first_cluster)?;
                directory_cluster = Some(entry.first_cluster);
            }
            result = Some(entry);
        }
        result.ok_or(Fat16Error::InvalidPath)
    }

    pub fn read<R: BlockReader>(
        &self,
        reader: &mut R,
        path: &str,
        output: &mut [u8],
    ) -> Result<usize, Fat16Error> {
        let metadata = self.lookup(reader, path)?;
        if metadata.kind == Fat16NodeKind::Directory {
            return Err(Fat16Error::IsDirectory);
        }
        let size = metadata.size as usize;
        if output.len() < size {
            return Err(Fat16Error::OutputTooSmall);
        }
        if size == 0 {
            return Ok(0);
        }
        self.validate_cluster(metadata.first_cluster)?;
        let cluster_bytes = usize::from(self.evidence.sectors_per_cluster) * SECTOR_SIZE;
        let required_clusters = size.div_ceil(cluster_bytes);
        if required_clusters > MAX_FILE_CLUSTERS {
            return Err(Fat16Error::ClusterChainTooLong);
        }

        let mut visited = [0_u16; MAX_FILE_CLUSTERS];
        let mut visited_len = 0_usize;
        let mut cluster = metadata.first_cluster;
        let mut written = 0_usize;
        let mut sector = [0_u8; SECTOR_SIZE];
        while written < size {
            record_cluster(&mut visited, &mut visited_len, cluster)?;
            let first_lba = self.cluster_lba(cluster)?;
            for offset in 0..u64::from(self.evidence.sectors_per_cluster) {
                reader.read_sector(first_lba + offset, &mut sector)?;
                let copy = core::cmp::min(SECTOR_SIZE, size - written);
                output[written..written + copy].copy_from_slice(&sector[..copy]);
                written += copy;
                if written == size {
                    return Ok(written);
                }
            }
            cluster = match self.next_cluster(reader, cluster)? {
                Some(next) => next,
                None => return Err(Fat16Error::TruncatedClusterChain),
            };
        }
        Ok(written)
    }

    fn find_in_directory<R: BlockReader>(
        &self,
        reader: &mut R,
        directory_cluster: Option<u16>,
        short_name: [u8; 11],
    ) -> Result<Fat16Metadata, Fat16Error> {
        match directory_cluster {
            None => {
                let mut sector = [0_u8; SECTOR_SIZE];
                for sector_offset in 0..u64::from(self.evidence.root_directory_sectors) {
                    reader.read_sector(self.root_directory_lba + sector_offset, &mut sector)?;
                    match scan_sector(&sector, &short_name, self.max_cluster)? {
                        ScanResult::Found(metadata) => return Ok(metadata),
                        ScanResult::End => return Err(Fat16Error::NotFound),
                        ScanResult::Continue => {}
                    }
                }
                Err(Fat16Error::NotFound)
            }
            Some(mut cluster) => {
                let mut visited = [0_u16; MAX_DIRECTORY_CLUSTERS];
                let mut visited_len = 0_usize;
                let mut sector = [0_u8; SECTOR_SIZE];
                loop {
                    record_cluster(&mut visited, &mut visited_len, cluster)?;
                    let first_lba = self.cluster_lba(cluster)?;
                    for offset in 0..u64::from(self.evidence.sectors_per_cluster) {
                        reader.read_sector(first_lba + offset, &mut sector)?;
                        match scan_sector(&sector, &short_name, self.max_cluster)? {
                            ScanResult::Found(metadata) => return Ok(metadata),
                            ScanResult::End => return Err(Fat16Error::NotFound),
                            ScanResult::Continue => {}
                        }
                    }
                    cluster = match self.next_cluster(reader, cluster)? {
                        Some(next) => next,
                        None => return Err(Fat16Error::NotFound),
                    };
                }
            }
        }
    }

    fn cluster_lba(&self, cluster: u16) -> Result<u64, Fat16Error> {
        self.validate_cluster(cluster)?;
        let relative = u64::from(cluster - 2)
            .checked_mul(u64::from(self.evidence.sectors_per_cluster))
            .ok_or(Fat16Error::LayoutOverflow)?;
        self.evidence
            .first_data_lba
            .checked_add(relative)
            .ok_or(Fat16Error::LayoutOverflow)
    }

    fn validate_cluster(&self, cluster: u16) -> Result<(), Fat16Error> {
        if !(2..=self.max_cluster).contains(&cluster) {
            return Err(Fat16Error::InvalidCluster);
        }
        Ok(())
    }

    fn next_cluster<R: BlockReader>(
        &self,
        reader: &mut R,
        cluster: u16,
    ) -> Result<Option<u16>, Fat16Error> {
        self.validate_cluster(cluster)?;
        let byte_offset = u64::from(cluster) * 2;
        let sector_offset = byte_offset / SECTOR_SIZE as u64;
        if sector_offset >= u64::from(self.evidence.sectors_per_fat) {
            return Err(Fat16Error::InvalidCluster);
        }
        let offset = (byte_offset % SECTOR_SIZE as u64) as usize;
        let mut first = [0_u8; SECTOR_SIZE];
        let mut second = [0_u8; SECTOR_SIZE];
        reader.read_sector(self.first_fat_lba + sector_offset, &mut first)?;
        reader.read_sector(self.second_fat_lba + sector_offset, &mut second)?;
        if first != second {
            return Err(Fat16Error::FatMirrorMismatch);
        }
        let value = le_u16(&first, offset);
        match value {
            0xfff8..=0xffff => Ok(None),
            2..=0xffef if value <= self.max_cluster => Ok(Some(value)),
            0xfff7 => Err(Fat16Error::BadCluster),
            _ => Err(Fat16Error::BadCluster),
        }
    }
}

enum ScanResult {
    Found(Fat16Metadata),
    End,
    Continue,
}

fn scan_sector(
    sector: &Sector,
    target: &[u8; 11],
    max_cluster: u16,
) -> Result<ScanResult, Fat16Error> {
    for entry in sector.chunks_exact(DIRECTORY_ENTRY_SIZE) {
        if entry[0] == 0 {
            return Ok(ScanResult::End);
        }
        if entry[0] == 0xe5 || entry[11] == ATTR_LONG_NAME || entry[11] & ATTR_VOLUME_ID != 0 {
            continue;
        }
        if &entry[..11] != target {
            continue;
        }
        if le_u16(entry, 20) != 0 {
            return Err(Fat16Error::InvalidDirectoryEntry);
        }
        let first_cluster = le_u16(entry, 26);
        let size = le_u32(entry, 28);
        let kind = if entry[11] & ATTR_DIRECTORY != 0 {
            Fat16NodeKind::Directory
        } else {
            Fat16NodeKind::File
        };
        if (kind == Fat16NodeKind::Directory || size != 0)
            && !(2..=max_cluster).contains(&first_cluster)
        {
            return Err(Fat16Error::InvalidDirectoryEntry);
        }
        return Ok(ScanResult::Found(Fat16Metadata {
            kind,
            size,
            first_cluster,
        }));
    }
    Ok(ScanResult::Continue)
}

fn record_cluster<const N: usize>(
    visited: &mut [u16; N],
    visited_len: &mut usize,
    cluster: u16,
) -> Result<(), Fat16Error> {
    if visited[..*visited_len].contains(&cluster) {
        return Err(Fat16Error::ClusterChainCycle);
    }
    if *visited_len == N {
        return Err(Fat16Error::ClusterChainTooLong);
    }
    visited[*visited_len] = cluster;
    *visited_len += 1;
    Ok(())
}

fn encode_short_name(component: &str) -> Result<[u8; 11], Fat16Error> {
    if component.is_empty() || component == "." || component == ".." || !component.is_ascii() {
        return Err(Fat16Error::InvalidShortName);
    }
    let mut parts = component.split('.');
    let base = parts.next().ok_or(Fat16Error::InvalidShortName)?;
    let extension = parts.next().unwrap_or("");
    if parts.next().is_some() || base.is_empty() || base.len() > 8 || extension.len() > 3 {
        return Err(Fat16Error::InvalidShortName);
    }
    let mut encoded = [b' '; 11];
    encode_name_part(base, &mut encoded[..8])?;
    encode_name_part(extension, &mut encoded[8..])?;
    Ok(encoded)
}

fn encode_name_part(source: &str, destination: &mut [u8]) -> Result<(), Fat16Error> {
    for (index, byte) in source.bytes().enumerate() {
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')) {
            return Err(Fat16Error::InvalidShortName);
        }
        destination[index] = byte.to_ascii_uppercase();
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    struct MemoryDisk {
        sectors: Vec<Sector>,
    }

    impl BlockReader for MemoryDisk {
        fn sector_count(&self) -> u64 {
            self.sectors.len() as u64
        }

        fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), BlockReadError> {
            *output = *self
                .sectors
                .get(lba as usize)
                .ok_or(BlockReadError::OutOfBounds)?;
            Ok(())
        }
    }

    fn fixture() -> MemoryDisk {
        const TOTAL: usize = 5_000;
        const FAT_SECTORS: usize = 20;
        let mut sectors = vec![[0_u8; SECTOR_SIZE]; TOTAL];
        let boot = &mut sectors[0];
        boot[11..13].copy_from_slice(&512_u16.to_le_bytes());
        boot[13] = 1;
        boot[14..16].copy_from_slice(&1_u16.to_le_bytes());
        boot[16] = 2;
        boot[17..19].copy_from_slice(&32_u16.to_le_bytes());
        boot[19..21].copy_from_slice(&(TOTAL as u16).to_le_bytes());
        boot[21] = 0xf8;
        boot[22..24].copy_from_slice(&(FAT_SECTORS as u16).to_le_bytes());
        boot[28..32].copy_from_slice(&0_u32.to_le_bytes());
        boot[510..512].copy_from_slice(&[0x55, 0xaa]);

        let mut fat = [0_u8; SECTOR_SIZE];
        fat[0..2].copy_from_slice(&0xfff8_u16.to_le_bytes());
        fat[2..4].copy_from_slice(&0xffff_u16.to_le_bytes());
        fat[4..6].copy_from_slice(&0xffff_u16.to_le_bytes());
        fat[6..8].copy_from_slice(&0xffff_u16.to_le_bytes());
        fat[8..10].copy_from_slice(&0xffff_u16.to_le_bytes());
        fat[10..12].copy_from_slice(&6_u16.to_le_bytes());
        fat[12..14].copy_from_slice(&0xffff_u16.to_le_bytes());
        sectors[1] = fat;
        sectors[1 + FAT_SECTORS] = fat;
        let root_lba = 1 + FAT_SECTORS * 2;
        sectors[root_lba][..11].copy_from_slice(b"HELLO   TXT");
        sectors[root_lba][11] = 0x20;
        sectors[root_lba][26..28].copy_from_slice(&2_u16.to_le_bytes());
        sectors[root_lba][28..32].copy_from_slice(&5_u32.to_le_bytes());
        sectors[root_lba][32..43].copy_from_slice(b"SYSTEM     ");
        sectors[root_lba][43] = ATTR_DIRECTORY;
        sectors[root_lba][58..60].copy_from_slice(&3_u16.to_le_bytes());
        sectors[root_lba][64..75].copy_from_slice(b"BIG     BIN");
        sectors[root_lba][75] = 0x20;
        sectors[root_lba][90..92].copy_from_slice(&5_u16.to_le_bytes());
        sectors[root_lba][92..96].copy_from_slice(&600_u32.to_le_bytes());
        let data_lba = root_lba + 2;
        sectors[data_lba][..5].copy_from_slice(b"hello");
        sectors[data_lba + 1][..11].copy_from_slice(b"BUILD   TXT");
        sectors[data_lba + 1][11] = 0x20;
        sectors[data_lba + 1][26..28].copy_from_slice(&4_u16.to_le_bytes());
        sectors[data_lba + 1][28..32].copy_from_slice(&5_u32.to_le_bytes());
        sectors[data_lba + 2][..5].copy_from_slice(b"build");
        sectors[data_lba + 3].fill(b'A');
        sectors[data_lba + 4][..88].fill(b'B');
        MemoryDisk { sectors }
    }

    #[test]
    fn mounts_and_reads_root_and_nested_files_case_insensitively() {
        let mut disk = fixture();
        let fs = Fat16::mount(&mut disk, 0, 5_000).unwrap();
        assert_eq!(fs.evidence().cluster_count, 4_957);
        let mut output = [0_u8; 8];
        assert_eq!(fs.read(&mut disk, "/hello.txt", &mut output).unwrap(), 5);
        assert_eq!(&output[..5], b"hello");
        assert_eq!(
            fs.read(&mut disk, "/system/build.txt", &mut output)
                .unwrap(),
            5
        );
        assert_eq!(&output[..5], b"build");
    }

    #[test]
    fn rejects_bad_bpb_and_fat_mirror() {
        let mut disk = fixture();
        disk.sectors[0][13] = 3;
        assert_eq!(
            Fat16::mount(&mut disk, 0, 5_000),
            Err(Fat16Error::InvalidSectorsPerCluster)
        );
        let mut disk = fixture();
        disk.sectors[21][4] ^= 1;
        assert_eq!(
            Fat16::mount(&mut disk, 0, 5_000),
            Err(Fat16Error::FatMirrorMismatch)
        );
    }

    #[test]
    fn rejects_noncanonical_paths_and_small_output() {
        let mut disk = fixture();
        let fs = Fat16::mount(&mut disk, 0, 5_000).unwrap();
        assert_eq!(
            fs.lookup(&mut disk, "/SYSTEM/../HELLO.TXT"),
            Err(Fat16Error::InvalidShortName)
        );
        assert_eq!(
            fs.lookup(&mut disk, "/SYSTEM//BUILD.TXT"),
            Err(Fat16Error::InvalidPath)
        );
        assert_eq!(
            fs.read(&mut disk, "/HELLO.TXT", &mut [0_u8; 4]),
            Err(Fat16Error::OutputTooSmall)
        );
    }

    #[test]
    fn reads_multiple_clusters_and_rejects_a_cycle() {
        let mut disk = fixture();
        let fs = Fat16::mount(&mut disk, 0, 5_000).unwrap();
        let mut output = [0_u8; 600];
        assert_eq!(fs.read(&mut disk, "/BIG.BIN", &mut output).unwrap(), 600);
        assert!(output[..512].iter().all(|byte| *byte == b'A'));
        assert!(output[512..].iter().all(|byte| *byte == b'B'));

        disk.sectors[1][10..12].copy_from_slice(&5_u16.to_le_bytes());
        disk.sectors[21][10..12].copy_from_slice(&5_u16.to_le_bytes());
        assert_eq!(
            fs.read(&mut disk, "/BIG.BIN", &mut output),
            Err(Fat16Error::ClusterChainCycle)
        );
    }
}
