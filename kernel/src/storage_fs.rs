//! M23 boot-time GPT, FAT16, and read-only VFS validation.

#[cfg(any(
    feature = "androidbox-apk-install0",
    feature = "app-data-runtime",
    feature = "storage-server-runtime"
))]
use bndroid_kernel::gpt::find_appdata_partition;
#[cfg(feature = "androidbox-apk-install0")]
use bndroid_kernel::gpt::{BNDROID_PACKAGES_PARTITION_TYPE_GUID, find_packages_partition};
use bndroid_kernel::{
    fat16::{Fat16, Fat16Error, Fat16Evidence, Fat16Metadata, Fat16NodeKind},
    gpt::{GptError, GptEvidence, find_bndroid_data_partition, find_bndroid_system_partition},
    system_files::{self, InstallError},
    vfs::{Vfs, VfsError},
};

use crate::{
    driver::virtio::block::PhysicalCounter,
    storage::{IrqBlockReader, StorageError},
};

const HELLO_PATH: &str = "/system/HELLO.TXT";
const BUILD_PATH: &str = "/system/SYSTEM/BUILD.TXT";
const HELLO_CONTENT: &[u8] = b"Bndroid M23 FAT16 is alive.\n";
const BUILD_CONTENT: &[u8] = b"build=m23\nfilesystem=fat16\nvfs=readonly\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageFsError {
    Storage(StorageError),
    Gpt(GptError),
    Fat16(Fat16Error),
    Vfs(VfsError),
    GptContract,
    DataGptContract,
    #[cfg(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    ))]
    AppDataGptContract,
    #[cfg(feature = "androidbox-apk-install0")]
    PackagesGptContract,
    Fat16Contract,
    VfsContract,
    IoAccounting,
    SystemCatalog(InstallError),
    SystemCatalogContract,
}

impl StorageFsError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Storage(error) => error.as_str(),
            Self::Gpt(error) => error.as_str(),
            Self::Fat16(error) => error.as_str(),
            Self::Vfs(error) => error.as_str(),
            Self::GptContract => "GPT does not match the deterministic M25 system partition",
            Self::DataGptContract => "GPT does not match the bounded M25 persistent-data partition",
            #[cfg(any(
                feature = "androidbox-apk-install0",
                feature = "app-data-runtime",
                feature = "storage-server-runtime"
            ))]
            Self::AppDataGptContract => {
                "GPT does not match the bounded M54 application-data partition"
            }
            #[cfg(feature = "androidbox-apk-install0")]
            Self::PackagesGptContract => {
                "GPT does not match the bounded AndroidBox installed-package partition"
            }
            Self::Fat16Contract => "FAT16 layout does not match the M23 system volume",
            Self::VfsContract => "read-only VFS lookup, content, or rejection evidence is invalid",
            Self::IoAccounting => "partition/filesystem parser sector-read accounting is invalid",
            Self::SystemCatalog(InstallError::AlreadyInstalled) => {
                "system file catalog was already installed"
            }
            Self::SystemCatalog(InstallError::FileTooLarge) => {
                "verified system file exceeds the immutable VMO limit"
            }
            Self::SystemCatalog(InstallError::OutOfMemory) => {
                "kernel heap cannot allocate the immutable system catalog"
            }
            Self::SystemCatalog(InstallError::Catalog(_)) => {
                "verified system file catalog has an invalid path layout"
            }
            Self::SystemCatalogContract => "published system file catalog evidence is invalid",
        }
    }

    pub const fn reason(self) -> &'static str {
        match self {
            Self::Storage(_) | Self::IoAccounting => "block_layer_invalid",
            Self::Gpt(_) | Self::GptContract => "gpt_invalid",
            Self::DataGptContract => "data_partition_invalid",
            #[cfg(any(
                feature = "androidbox-apk-install0",
                feature = "app-data-runtime",
                feature = "storage-server-runtime"
            ))]
            Self::AppDataGptContract => "appdata_partition_invalid",
            #[cfg(feature = "androidbox-apk-install0")]
            Self::PackagesGptContract => "packages_partition_invalid",
            Self::Fat16(_) | Self::Fat16Contract => "fat16_invalid",
            Self::Vfs(_) | Self::VfsContract => "vfs_invalid",
            Self::SystemCatalog(_) | Self::SystemCatalogContract => "system_catalog_invalid",
        }
    }
}

impl From<StorageError> for StorageFsError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<GptError> for StorageFsError {
    fn from(error: GptError) -> Self {
        Self::Gpt(error)
    }
}

impl From<Fat16Error> for StorageFsError {
    fn from(error: Fat16Error) -> Self {
        Self::Fat16(error)
    }
}

impl From<VfsError> for StorageFsError {
    fn from(error: VfsError) -> Self {
        Self::Vfs(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageFsEvidence {
    pub gpt: GptEvidence,
    pub data_gpt: GptEvidence,
    #[cfg(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    ))]
    pub appdata_gpt: GptEvidence,
    #[cfg(feature = "androidbox-apk-install0")]
    pub packages_gpt: GptEvidence,
    pub fat16: Fat16Evidence,
    pub hello: Fat16Metadata,
    pub build: Fat16Metadata,
    pub hello_bytes: usize,
    pub build_bytes: usize,
    pub hello_digest: u64,
    pub build_digest: u64,
    pub parser_reads: u64,
    pub traversal_rejected: bool,
    pub mount_escape_rejected: bool,
    pub write_rejected: bool,
    pub catalog_files: usize,
    pub catalog_bytes: usize,
}

pub fn validate(counter_frequency: u64) -> Result<StorageFsEvidence, StorageFsError> {
    let mut clock = PhysicalCounter;
    let mut reader = IrqBlockReader::new(&mut clock, counter_frequency)?;
    let gpt = find_bndroid_system_partition(&mut reader)?;
    let data_gpt = find_bndroid_data_partition(&mut reader)?;
    #[cfg(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    ))]
    let appdata_gpt = find_appdata_partition(&mut reader)?;
    #[cfg(feature = "androidbox-apk-install0")]
    let packages_gpt = find_packages_partition(&mut reader)?;
    #[cfg(feature = "androidbox-multipackage4")]
    let (expected_primary_header_crc32, expected_backup_header_crc32, expected_entry_crc32) =
        (0x0038_e58c, 0xf9ce_b432, 0x074e_7bb0);
    #[cfg(all(
        feature = "androidbox-apk-install0",
        not(feature = "androidbox-multipackage4")
    ))]
    let (expected_primary_header_crc32, expected_backup_header_crc32, expected_entry_crc32) =
        (0xcf27_87ad, 0x36d1_d613, 0x1364_8066);
    #[cfg(all(
        not(feature = "androidbox-apk-install0"),
        any(feature = "app-data-runtime", feature = "storage-server-runtime")
    ))]
    let (expected_primary_header_crc32, expected_backup_header_crc32, expected_entry_crc32) =
        (0x61a2_4d8e, 0xd597_58bb, 0xe8ba_27af);
    #[cfg(not(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    )))]
    let (expected_primary_header_crc32, expected_backup_header_crc32, expected_entry_crc32) =
        (0xead6_50f4, 0x5ee3_45c1, 0xb37f_1277);
    #[cfg(feature = "androidbox-apk-install0")]
    let (expected_device_sectors, expected_last_usable_lba, expected_backup_entry_array_lba) =
        (32_768, 32_734, 32_735);
    #[cfg(not(feature = "androidbox-apk-install0"))]
    let (expected_device_sectors, expected_last_usable_lba, expected_backup_entry_array_lba) =
        (16_384, 16_350, 16_351);
    let expected_name = [
        b'B' as u16,
        b'N' as u16,
        b'D' as u16,
        b'R' as u16,
        b'O' as u16,
        b'I' as u16,
        b'D' as u16,
        b'_' as u16,
        b'S' as u16,
        b'Y' as u16,
        b'S' as u16,
    ];
    if gpt.device_sectors != expected_device_sectors
        || gpt.first_usable_lba != 34
        || gpt.last_usable_lba != expected_last_usable_lba
        || gpt.primary_header_crc32 != expected_primary_header_crc32
        || gpt.backup_header_crc32 != expected_backup_header_crc32
        || gpt.partition_entry_crc32 != expected_entry_crc32
        || gpt.partition_entry_count != 128
        || gpt.partition_entry_size != 128
        || gpt.primary_entry_array_lba != 2
        || gpt.backup_entry_array_lba != expected_backup_entry_array_lba
        || gpt.partition.entry_index != 0
        || gpt.partition.first_lba != 2_048
        || gpt.partition.last_lba != 16_350
        || gpt.partition.sector_count() != 14_303
        || gpt.partition.attributes != 0
        || gpt.partition.name() != expected_name
    {
        return Err(StorageFsError::GptContract);
    }
    let expected_data_name = [
        b'B' as u16,
        b'N' as u16,
        b'D' as u16,
        b'R' as u16,
        b'O' as u16,
        b'I' as u16,
        b'D' as u16,
        b'_' as u16,
        b'D' as u16,
        b'A' as u16,
        b'T' as u16,
        b'A' as u16,
    ];
    if data_gpt.device_sectors != gpt.device_sectors
        || data_gpt.disk_guid != gpt.disk_guid
        || data_gpt.first_usable_lba != gpt.first_usable_lba
        || data_gpt.last_usable_lba != gpt.last_usable_lba
        || data_gpt.primary_header_crc32 != gpt.primary_header_crc32
        || data_gpt.backup_header_crc32 != gpt.backup_header_crc32
        || data_gpt.partition_entry_crc32 != gpt.partition_entry_crc32
        || data_gpt.partition_entry_count != gpt.partition_entry_count
        || data_gpt.partition_entry_size != gpt.partition_entry_size
        || data_gpt.primary_entry_array_lba != gpt.primary_entry_array_lba
        || data_gpt.backup_entry_array_lba != gpt.backup_entry_array_lba
        || data_gpt.partition.entry_index != 1
        || data_gpt.partition.first_lba != 64
        || data_gpt.partition.last_lba != 127
        || data_gpt.partition.sector_count() != 64
        || data_gpt.partition.attributes != 0
        || data_gpt.partition.name() != expected_data_name
        || data_gpt.partition.last_lba >= gpt.partition.first_lba
    {
        return Err(StorageFsError::DataGptContract);
    }

    #[cfg(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    ))]
    {
        let expected_appdata_name = [
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
        ];
        if appdata_gpt.device_sectors != gpt.device_sectors
            || appdata_gpt.disk_guid != gpt.disk_guid
            || appdata_gpt.first_usable_lba != gpt.first_usable_lba
            || appdata_gpt.last_usable_lba != gpt.last_usable_lba
            || appdata_gpt.primary_header_crc32 != gpt.primary_header_crc32
            || appdata_gpt.backup_header_crc32 != gpt.backup_header_crc32
            || appdata_gpt.partition_entry_crc32 != gpt.partition_entry_crc32
            || appdata_gpt.partition_entry_count != gpt.partition_entry_count
            || appdata_gpt.partition_entry_size != gpt.partition_entry_size
            || appdata_gpt.primary_entry_array_lba != gpt.primary_entry_array_lba
            || appdata_gpt.backup_entry_array_lba != gpt.backup_entry_array_lba
            || appdata_gpt.partition.entry_index != 2
            || appdata_gpt.partition.first_lba != 128
            || appdata_gpt.partition.last_lba != 2_047
            || appdata_gpt.partition.sector_count() != 1_920
            || appdata_gpt.partition.attributes != 0
            || appdata_gpt.partition.name() != expected_appdata_name
            || data_gpt.partition.last_lba >= appdata_gpt.partition.first_lba
            || appdata_gpt.partition.last_lba >= gpt.partition.first_lba
        {
            return Err(StorageFsError::AppDataGptContract);
        }
    }

    #[cfg(feature = "androidbox-apk-install0")]
    {
        let expected_packages_guid = [
            0x74, 0xf1, 0x5c, 0xd2, 0x79, 0xa8, 0x5a, 0x4e, 0x93, 0x64, 0x67, 0xb2, 0xd8, 0x90,
            0x58, 0x01,
        ];
        let expected_packages_name = [
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
        ];
        if packages_gpt.device_sectors != gpt.device_sectors
            || packages_gpt.disk_guid != gpt.disk_guid
            || packages_gpt.first_usable_lba != gpt.first_usable_lba
            || packages_gpt.last_usable_lba != gpt.last_usable_lba
            || packages_gpt.primary_header_crc32 != gpt.primary_header_crc32
            || packages_gpt.backup_header_crc32 != gpt.backup_header_crc32
            || packages_gpt.partition_entry_crc32 != gpt.partition_entry_crc32
            || packages_gpt.partition_entry_count != gpt.partition_entry_count
            || packages_gpt.partition_entry_size != gpt.partition_entry_size
            || packages_gpt.primary_entry_array_lba != gpt.primary_entry_array_lba
            || packages_gpt.backup_entry_array_lba != gpt.backup_entry_array_lba
            || packages_gpt.partition.entry_index != 3
            || packages_gpt.partition.type_guid != BNDROID_PACKAGES_PARTITION_TYPE_GUID
            || packages_gpt.partition.unique_guid != expected_packages_guid
            || packages_gpt.partition.first_lba != 16_384
            || packages_gpt.partition.last_lba
                != if cfg!(feature = "androidbox-multipackage4") {
                    17_407
                } else {
                    16_895
                }
            || packages_gpt.partition.sector_count()
                != if cfg!(feature = "androidbox-multipackage4") {
                    1_024
                } else {
                    512
                }
            || packages_gpt.partition.attributes != 0
            || packages_gpt.partition.name() != expected_packages_name
            || gpt.partition.last_lba >= packages_gpt.partition.first_lba
        {
            return Err(StorageFsError::PackagesGptContract);
        }
    }

    let fat16 = Fat16::mount(
        &mut reader,
        gpt.partition.first_lba,
        gpt.partition.sector_count(),
    )?;
    let fat = fat16.evidence();
    if fat.partition_first_lba != 2_048
        || fat.partition_sectors != 14_303
        || fat.bytes_per_sector != 512
        || fat.sectors_per_cluster != 1
        || fat.reserved_sectors != 1
        || fat.fat_count != 2
        || fat.sectors_per_fat != 56
        || fat.root_entries != 64
        || fat.root_directory_sectors != 4
        || fat.cluster_count != 14_186
        || fat.first_data_lba != 2_165
        || !fat.mirror_verified
    {
        return Err(StorageFsError::Fat16Contract);
    }

    let vfs = Vfs::mount_system(fat16);
    let hello = vfs.metadata(&mut reader, HELLO_PATH)?;
    let build = vfs.metadata(&mut reader, BUILD_PATH)?;
    let mut hello_output = [0_u8; 64];
    let mut build_output = [0_u8; 64];
    let hello_bytes = vfs.read(&mut reader, HELLO_PATH, &mut hello_output)?;
    let build_bytes = vfs.read(&mut reader, BUILD_PATH, &mut build_output)?;
    let traversal_rejected = matches!(
        vfs.metadata(&mut reader, "/system/../HELLO.TXT"),
        Err(VfsError::FileSystem(Fat16Error::InvalidShortName))
    );
    let mount_escape_rejected = matches!(
        vfs.metadata(&mut reader, "/systematic/HELLO.TXT"),
        Err(VfsError::InvalidMountPath)
    );
    let write_rejected = matches!(vfs.write(HELLO_PATH, b"mutation"), Err(VfsError::ReadOnly));
    let hello_digest = fnv1a64(&hello_output[..hello_bytes]);
    let build_digest = fnv1a64(&build_output[..build_bytes]);
    if !vfs.is_read_only()
        || hello.kind != Fat16NodeKind::File
        || hello.size != HELLO_CONTENT.len() as u32
        || hello.first_cluster != 2
        || build.kind != Fat16NodeKind::File
        || build.size != BUILD_CONTENT.len() as u32
        || build.first_cluster != 4
        || hello_bytes != HELLO_CONTENT.len()
        || build_bytes != BUILD_CONTENT.len()
        || &hello_output[..hello_bytes] != HELLO_CONTENT
        || &build_output[..build_bytes] != BUILD_CONTENT
        || hello_digest != 0xdd2f_7134_2016_eede
        || build_digest != 0xe57c_e4ce_9f1b_4ec0
        || !traversal_rejected
        || !mount_escape_rejected
        || !write_rejected
    {
        return Err(StorageFsError::VfsContract);
    }
    let parser_reads = reader.reads();
    #[cfg(feature = "androidbox-apk-install0")]
    let expected_parser_reads = 535;
    #[cfg(all(
        not(feature = "androidbox-apk-install0"),
        any(feature = "app-data-runtime", feature = "storage-server-runtime")
    ))]
    let expected_parser_reads = 404;
    #[cfg(not(any(
        feature = "androidbox-apk-install0",
        feature = "app-data-runtime",
        feature = "storage-server-runtime"
    )))]
    let expected_parser_reads = 273;
    if parser_reads != expected_parser_reads {
        return Err(StorageFsError::IoAccounting);
    }
    system_files::install(&hello_output[..hello_bytes], &build_output[..build_bytes])
        .map_err(StorageFsError::SystemCatalog)?;
    let catalog = system_files::snapshot();
    if !catalog.ready
        || catalog.files != system_files::SYSTEM_FILE_COUNT
        || catalog.bytes != hello_bytes + build_bytes
    {
        return Err(StorageFsError::SystemCatalogContract);
    }

    Ok(StorageFsEvidence {
        gpt,
        data_gpt,
        #[cfg(any(
            feature = "androidbox-apk-install0",
            feature = "app-data-runtime",
            feature = "storage-server-runtime"
        ))]
        appdata_gpt,
        #[cfg(feature = "androidbox-apk-install0")]
        packages_gpt,
        fat16: fat,
        hello,
        build,
        hello_bytes,
        build_bytes,
        hello_digest,
        build_digest,
        parser_reads,
        traversal_rejected,
        mount_escape_rejected,
        write_rejected,
        catalog_files: catalog.files,
        catalog_bytes: catalog.bytes,
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
