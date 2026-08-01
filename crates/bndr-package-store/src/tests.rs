extern crate std;

use super::*;
use std::vec;
use std::vec::Vec;

const BASE: u64 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    WriteBefore {
        index: usize,
        error: IoError,
    },
    TornWrite {
        index: usize,
        bytes: usize,
    },
    Flush {
        index: usize,
        commit_before_error: bool,
        error: IoError,
    },
    ReadBefore {
        index: usize,
        error: IoError,
    },
    ReadAfterCommit {
        lba: u64,
    },
    ReadAfterFlush {
        lba: u64,
        minimum_successful_flushes: usize,
    },
}

#[derive(Clone)]
struct MemoryIo {
    visible: Vec<Sector>,
    durable: Vec<Sector>,
    fault: Fault,
    reads: usize,
    writes: usize,
    flushes: usize,
    successful_flushes: usize,
    touched_writes: Vec<u64>,
}

impl MemoryIo {
    fn new(sectors: usize) -> Self {
        Self {
            visible: vec![[0; SECTOR_SIZE]; sectors],
            durable: vec![[0; SECTOR_SIZE]; sectors],
            fault: Fault::None,
            reads: 0,
            writes: 0,
            flushes: 0,
            successful_flushes: 0,
            touched_writes: Vec::new(),
        }
    }

    fn formatted() -> Self {
        let mut io = Self::new((BASE + VOLUME_SECTORS + 9) as usize);
        format(&mut io, BASE).unwrap();
        io.reset_observation();
        io
    }

    fn reset_observation(&mut self) {
        self.fault = Fault::None;
        self.reads = 0;
        self.writes = 0;
        self.flushes = 0;
        self.successful_flushes = 0;
        self.touched_writes.clear();
    }

    fn reboot(&mut self) {
        self.visible.clone_from(&self.durable);
        self.reset_observation();
    }

    fn set_fault(&mut self, fault: Fault) {
        self.reset_observation();
        self.fault = fault;
    }

    fn corrupt_durable(&mut self, lba: u64, offset: usize, mask: u8) {
        self.durable[lba as usize][offset] ^= mask;
        self.visible[lba as usize][offset] ^= mask;
    }

    fn overwrite_durable(&mut self, lba: u64, sector: Sector) {
        self.durable[lba as usize] = sector;
        self.visible[lba as usize] = sector;
    }
}

impl SectorIo for MemoryIo {
    fn sector_count(&self) -> u64 {
        self.visible.len() as u64
    }

    fn read(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
        if lba >= self.sector_count() {
            return Err(IoError::OutOfBounds);
        }
        let index = self.reads;
        self.reads += 1;
        if let Fault::ReadBefore {
            index: fault_index,
            error,
        } = self.fault
            && fault_index == index
        {
            return Err(error);
        }
        if matches!(
            self.fault,
            Fault::ReadAfterCommit { lba: fault_lba }
                if fault_lba == lba && self.successful_flushes >= 2
        ) {
            return Err(IoError::Device);
        }
        if matches!(
            self.fault,
            Fault::ReadAfterFlush {
                lba: fault_lba,
                minimum_successful_flushes,
            } if fault_lba == lba && self.successful_flushes >= minimum_successful_flushes
        ) {
            return Err(IoError::Device);
        }
        let _ = index;
        *output = self.visible[lba as usize];
        Ok(())
    }

    fn write(&mut self, lba: u64, input: &Sector) -> Result<(), IoError> {
        if lba >= self.sector_count() {
            return Err(IoError::OutOfBounds);
        }
        let index = self.writes;
        self.writes += 1;
        self.touched_writes.push(lba);
        match self.fault {
            Fault::WriteBefore {
                index: fault_index,
                error,
            } if fault_index == index => Err(error),
            Fault::TornWrite {
                index: fault_index,
                bytes,
            } if fault_index == index => {
                let copied = core::cmp::min(bytes, SECTOR_SIZE);
                self.visible[lba as usize][..copied].copy_from_slice(&input[..copied]);
                self.durable[lba as usize][..copied].copy_from_slice(&input[..copied]);
                Err(IoError::OutcomeUnknown)
            }
            _ => {
                self.visible[lba as usize] = *input;
                Ok(())
            }
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
        let index = self.flushes;
        self.flushes += 1;
        match self.fault {
            Fault::Flush {
                index: fault_index,
                commit_before_error,
                error,
            } if fault_index == index => {
                if commit_before_error {
                    self.durable.clone_from(&self.visible);
                }
                Err(error)
            }
            _ => {
                self.durable.clone_from(&self.visible);
                self.successful_flushes += 1;
                Ok(())
            }
        }
    }
}

fn apk(seed: u8, length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| seed.wrapping_add((index as u8).wrapping_mul(31)))
        .collect()
}

fn metadata(
    transaction_id: u64,
    package: &str,
    activity: &str,
    version_code: u64,
    bytes: &[u8],
    signer: u8,
) -> InstallMetadata {
    InstallMetadata::try_new(
        transaction_id,
        package,
        activity,
        version_code,
        sha256(bytes),
        [signer; 32],
        CompatibilityProfile::Resources1,
    )
    .unwrap()
}

fn first_package(io: &mut MemoryIo) -> (Vec<u8>, InstallMetadata, InstalledPackage) {
    let bytes = apk(0x21, 1_173);
    let admitted = metadata(
        11,
        "com.bndroid.fixture",
        "com.bndroid.fixture.MainActivity",
        1,
        &bytes,
        0x5a,
    );
    let installed = install(io, BASE, admitted, &bytes).unwrap();
    io.reset_observation();
    (bytes, admitted, installed)
}

fn second_package() -> (Vec<u8>, InstallMetadata) {
    let bytes = apk(0x83, 2_077);
    let admitted = metadata(
        12,
        "com.bndroid.fixture",
        "com.bndroid.fixture.SettingsActivity",
        2,
        &bytes,
        0x5a,
    );
    (bytes, admitted)
}

fn third_package() -> (Vec<u8>, InstallMetadata) {
    let bytes = apk(0xc7, 3_149);
    let admitted = metadata(
        13,
        "com.bndroid.fixture",
        "com.bndroid.fixture.MainActivity",
        3,
        &bytes,
        0x5a,
    );
    (bytes, admitted)
}

#[cfg(feature = "multi-package-store1")]
fn distinct_package(
    transaction_id: u64,
    package: &str,
    activity: &str,
    version_code: u64,
    seed: u8,
) -> (Vec<u8>, InstallMetadata) {
    let bytes = apk(seed, 1_701 + usize::from(seed));
    let admitted = metadata(
        transaction_id,
        package,
        activity,
        version_code,
        &bytes,
        seed,
    );
    (bytes, admitted)
}

#[cfg(feature = "multi-package-store1")]
#[test]
fn multi_store_formats_resumes_and_preserves_the_legacy_first_volume() {
    let mut io = MemoryIo::new((BASE + MULTI_PACKAGE_VOLUME_SECTORS + 3) as usize);
    format(&mut io, BASE).unwrap();
    io.reset_observation();

    let evidence = ensure_multi_formatted(&mut io, BASE).unwrap();
    assert_eq!(evidence.already_formatted(), 1);
    assert_eq!(evidence.newly_formatted(), 1);
    assert_eq!(io.writes, 1);
    assert_eq!(io.flushes, 1);

    io.reset_observation();
    let replay = ensure_multi_formatted(&mut io, BASE).unwrap();
    assert_eq!(replay.already_formatted(), 2);
    assert_eq!(replay.newly_formatted(), 0);
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
    assert_eq!(
        recover_multi(&mut io, BASE).unwrap().states(),
        &[PackageState::Empty, PackageState::Empty]
    );
}

#[cfg(feature = "multi-package-store1")]
#[test]
fn multi_store_installs_reads_updates_and_recovers_two_distinct_packages() {
    let mut io = MemoryIo::new((BASE + MULTI_PACKAGE_VOLUME_SECTORS + 3) as usize);
    let formatted = ensure_multi_formatted(&mut io, BASE).unwrap();
    assert_eq!(formatted.newly_formatted(), 2);

    let (first_bytes, first_metadata) = distinct_package(
        101,
        "org.bndroid.first",
        "org.bndroid.first.MainActivity",
        1,
        0x31,
    );
    let (second_bytes, second_metadata) = distinct_package(
        102,
        "org.bndroid.second",
        "org.bndroid.second.MainActivity",
        1,
        0x62,
    );
    let first = install_multi(&mut io, BASE, first_metadata, &first_bytes).unwrap();
    let second = install_multi(&mut io, BASE, second_metadata, &second_bytes).unwrap();
    assert_eq!(first.volume_index(), 0);
    assert_eq!(second.volume_index(), 1);

    let catalog = recover_multi(&mut io, BASE).unwrap();
    assert_eq!(catalog.installed_count(), 2);
    assert_eq!(catalog.retained_identity_count(), 2);
    assert_eq!(catalog.find_installed("org.bndroid.first"), Some(first));
    assert_eq!(catalog.find_installed("org.bndroid.second"), Some(second));

    let mut output = vec![0_u8; MAX_APK_BYTES];
    let first_len = read_multi_blob(&mut io, BASE, first, &mut output).unwrap();
    assert_eq!(&output[..first_len], first_bytes.as_slice());
    output.fill(0);
    let second_len = read_multi_blob(&mut io, BASE, second, &mut output).unwrap();
    assert_eq!(&output[..second_len], second_bytes.as_slice());

    let first_v2_bytes = apk(0x33, 1_701 + 0x33);
    let first_v2_metadata = metadata(
        103,
        "org.bndroid.first",
        "org.bndroid.first.MainActivity",
        2,
        &first_v2_bytes,
        0x31,
    );
    io.reset_observation();
    let first_v2 = install_multi(&mut io, BASE, first_v2_metadata, &first_v2_bytes).unwrap();
    assert_eq!(first_v2.volume_index(), 0);
    assert_eq!(first_v2.package().generation(), 2);
    assert!(
        io.touched_writes
            .iter()
            .all(|lba| *lba < BASE + VOLUME_SECTORS)
    );

    io.reboot();
    let recovered = recover_multi(&mut io, BASE).unwrap();
    assert_eq!(recovered.installed_count(), 2);
    assert_eq!(
        recovered
            .find_installed("org.bndroid.first")
            .unwrap()
            .package()
            .metadata()
            .version_code(),
        2
    );
    assert_eq!(
        recovered
            .find_installed("org.bndroid.second")
            .unwrap()
            .package(),
        second.package()
    );
}

#[cfg(feature = "multi-package-store1")]
#[test]
fn multi_store_is_capacity_bounded_and_retains_uninstall_identity() {
    let mut io = MemoryIo::new((BASE + MULTI_PACKAGE_VOLUME_SECTORS + 3) as usize);
    ensure_multi_formatted(&mut io, BASE).unwrap();
    let (first_bytes, first_metadata) = distinct_package(
        201,
        "org.bndroid.first",
        "org.bndroid.first.MainActivity",
        1,
        0x41,
    );
    let (second_bytes, second_metadata) = distinct_package(
        202,
        "org.bndroid.second",
        "org.bndroid.second.MainActivity",
        1,
        0x42,
    );
    let first = install_multi(&mut io, BASE, first_metadata, &first_bytes).unwrap();
    install_multi(&mut io, BASE, second_metadata, &second_bytes).unwrap();

    let (third_bytes, third_metadata) = distinct_package(
        203,
        "org.bndroid.third",
        "org.bndroid.third.MainActivity",
        1,
        0x43,
    );
    assert_eq!(
        install_multi(&mut io, BASE, third_metadata, &third_bytes),
        Err(MultiPackageError::Capacity)
    );

    let removed = uninstall_multi(
        &mut io,
        BASE,
        301,
        first,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    assert_eq!(removed.package(), "org.bndroid.first");
    let catalog = recover_multi(&mut io, BASE).unwrap();
    assert_eq!(catalog.installed_count(), 1);
    assert_eq!(catalog.retained_identity_count(), 2);
    assert!(catalog.find_removed("org.bndroid.first").is_some());
    assert_eq!(
        install_multi(&mut io, BASE, third_metadata, &third_bytes),
        Err(MultiPackageError::Capacity)
    );

    let reinstall_bytes = first_bytes;
    let reinstall_metadata = metadata(
        204,
        "org.bndroid.first",
        "org.bndroid.first.MainActivity",
        1,
        &reinstall_bytes,
        0x41,
    );
    let reinstalled = install_multi(&mut io, BASE, reinstall_metadata, &reinstall_bytes).unwrap();
    assert_eq!(reinstalled.volume_index(), 0);
    assert_eq!(reinstalled.package().generation(), removed.generation() + 1);
}

#[cfg(feature = "multi-package-store1")]
#[test]
fn multi_store_rejects_duplicate_identity_and_checks_whole_extent_before_writes() {
    let mut too_small = MemoryIo::new((BASE + MULTI_PACKAGE_VOLUME_SECTORS - 1) as usize);
    assert_eq!(
        ensure_multi_formatted(&mut too_small, BASE),
        Err(MultiPackageError::Store(Error::VolumeBounds))
    );
    assert_eq!(too_small.writes, 0);
    assert_eq!(too_small.flushes, 0);

    let mut io = MemoryIo::new((BASE + MULTI_PACKAGE_VOLUME_SECTORS + 3) as usize);
    ensure_multi_formatted(&mut io, BASE).unwrap();
    let (bytes, first_metadata) = distinct_package(
        401,
        "org.bndroid.duplicate",
        "org.bndroid.duplicate.MainActivity",
        1,
        0x51,
    );
    let second_metadata = metadata(
        402,
        "org.bndroid.duplicate",
        "org.bndroid.duplicate.MainActivity",
        1,
        &bytes,
        0x51,
    );
    install(&mut io, BASE, first_metadata, &bytes).unwrap();
    install(&mut io, BASE + VOLUME_SECTORS, second_metadata, &bytes).unwrap();
    assert_eq!(
        recover_multi(&mut io, BASE),
        Err(MultiPackageError::DuplicatePackage)
    );
}

#[test]
fn fixed_layout_is_exact_and_non_overlapping() {
    assert_eq!(SECTOR_SIZE, 512);
    assert_eq!(VOLUME_SECTORS, 512);
    assert_eq!(SUPERBLOCK_RELATIVE_LBA, 0);
    assert_eq!(REGISTRY_RELATIVE_LBAS, [1, 2]);
    assert_eq!(BLOB_RELATIVE_LBAS, [16, 144]);
    assert_eq!(BLOB_SECTORS, 128);
    assert_eq!(BLOB_PAYLOAD_SECTORS, 127);
    assert_eq!(MAX_APK_BYTES, 65_024);
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
}

#[test]
fn metadata_is_bounded_ascii_and_exposes_exact_values() {
    let bytes = b"apk";
    let digest = sha256(bytes);
    let metadata = InstallMetadata::try_new(
        9,
        "com.example.app",
        "com.example.app.Main",
        42,
        digest,
        [7; 32],
        CompatibilityProfile::Activity0,
    )
    .unwrap();
    assert_eq!(metadata.transaction_id(), 9);
    assert_eq!(metadata.package(), "com.example.app");
    assert_eq!(metadata.activity(), "com.example.app.Main");
    assert_eq!(metadata.version_code(), 42);
    assert_eq!(metadata.apk_sha256(), &digest);
    assert_eq!(metadata.signer_cert_sha256(), &[7; 32]);
    assert_eq!(
        metadata.compatibility_profile(),
        CompatibilityProfile::Activity0
    );

    assert_eq!(
        InstallMetadata::try_new(
            0,
            "p",
            "a",
            1,
            digest,
            [1; 32],
            CompatibilityProfile::Activity0
        ),
        Err(MetadataError::TransactionIdZero)
    );
    assert_eq!(
        InstallMetadata::try_new(
            1,
            "",
            "a",
            1,
            digest,
            [1; 32],
            CompatibilityProfile::Activity0
        ),
        Err(MetadataError::EmptyPackage)
    );
    assert_eq!(
        InstallMetadata::try_new(
            1,
            "p",
            "a\n",
            1,
            digest,
            [1; 32],
            CompatibilityProfile::Activity0
        ),
        Err(MetadataError::NonAsciiActivity)
    );
    assert_eq!(
        InstallMetadata::try_new(
            1,
            "p",
            "a",
            0,
            digest,
            [1; 32],
            CompatibilityProfile::Activity0
        ),
        Err(MetadataError::VersionCodeZero)
    );
    assert_eq!(
        InstallMetadata::try_new(
            1,
            "p",
            "a",
            1,
            digest,
            [0; 32],
            CompatibilityProfile::Activity0
        ),
        Err(MetadataError::ZeroSignerDigest)
    );
}

#[test]
fn format_and_empty_recovery_are_stable_and_write_free() {
    let mut io = MemoryIo::new((BASE + VOLUME_SECTORS) as usize);
    format(&mut io, BASE).unwrap();
    assert_eq!(format(&mut io, BASE), Err(Error::AlreadyFormatted));
    io.reset_observation();
    assert_eq!(recover(&mut io, BASE).unwrap(), None);
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
    assert_eq!(recover(&mut io, BASE).unwrap(), None);
    assert_eq!(io.writes, 0);
}

#[test]
fn format_rejects_nonvirgin_volume_without_writing() {
    let mut io = MemoryIo::new((BASE + VOLUME_SECTORS) as usize);
    io.visible[(BASE + 300) as usize][4] = 1;
    io.durable.clone_from(&io.visible);
    assert_eq!(format(&mut io, BASE), Err(Error::NotVirgin));
    assert_eq!(io.writes, 0);
}

#[test]
fn volume_bounds_fail_before_any_io() {
    let mut io = MemoryIo::new(VOLUME_SECTORS as usize);
    assert_eq!(format(&mut io, 1), Err(Error::VolumeBounds));
    assert_eq!(recover(&mut io, 1), Err(Error::VolumeBounds));
    assert_eq!(io.reads, 0);
    assert_eq!(io.writes, 0);
    assert_eq!(recover(&mut io, u64::MAX), Err(Error::VolumeBounds));
    assert_eq!(io.reads, 0);
}

#[test]
fn install_recover_and_read_blob_round_trip() {
    let mut io = MemoryIo::formatted();
    let (bytes, admitted, installed) = first_package(&mut io);
    assert_eq!(installed.generation(), 1);
    assert_eq!(installed.slot(), 0);
    assert_eq!(installed.apk_length(), bytes.len());
    assert_eq!(installed.metadata(), &admitted);

    let recovered = recover(&mut io, BASE).unwrap().unwrap();
    assert_eq!(recovered, installed);
    let mut output = vec![0xa5; bytes.len() + 19];
    assert_eq!(
        read_blob(&mut io, BASE, &recovered, &mut output).unwrap(),
        bytes.len()
    );
    assert_eq!(&output[..bytes.len()], bytes.as_slice());
    assert!(output[bytes.len()..].iter().all(|byte| *byte == 0xa5));
}

#[test]
fn every_write_stays_inside_the_declared_volume_and_slots() {
    let mut io = MemoryIo::formatted();
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    assert!(
        io.touched_writes
            .iter()
            .all(|lba| (BASE..BASE + VOLUME_SECTORS).contains(lba))
    );
    assert_eq!(io.writes, BLOB_PAYLOAD_SECTORS as usize + 2);
    let blob_start = BASE + BLOB_RELATIVE_LBAS[0];
    assert!(
        io.touched_writes
            .iter()
            .take(BLOB_PAYLOAD_SECTORS as usize)
            .all(|lba| (blob_start + 1..blob_start + BLOB_SECTORS).contains(lba))
    );
    assert_eq!(io.touched_writes[BLOB_PAYLOAD_SECTORS as usize], blob_start);
    assert_eq!(
        io.touched_writes[BLOB_PAYLOAD_SECTORS as usize + 1],
        BASE + REGISTRY_RELATIVE_LBAS[0]
    );
}

#[test]
fn exact_install_and_update_transaction_retries_are_idempotent_and_write_free() {
    let mut io = MemoryIo::formatted();
    let (bytes, admitted, installed) = first_package(&mut io);
    let retried = install(&mut io, BASE, admitted, &bytes).unwrap();
    assert_eq!(retried, installed);
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);

    let (updated_bytes, updated_metadata) = second_package();
    let updated = install(&mut io, BASE, updated_metadata, &updated_bytes).unwrap();
    io.reboot();
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(updated));
    io.reset_observation();
    let retried_update = install(&mut io, BASE, updated_metadata, &updated_bytes).unwrap();
    assert_eq!(retried_update, updated);
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn reused_transaction_with_different_content_is_rejected() {
    let mut io = MemoryIo::formatted();
    let (_, admitted, _) = first_package(&mut io);
    let changed = apk(0x44, 900);
    let conflict = metadata(
        admitted.transaction_id(),
        admitted.package(),
        admitted.activity(),
        admitted.version_code(),
        &changed,
        0x5a,
    );
    assert_eq!(
        install(&mut io, BASE, conflict, &changed),
        Err(Error::TransactionConflict)
    );
    assert_eq!(io.writes, 0);
}

#[test]
fn update_policy_preserves_package_and_signer_and_increases_version() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let bytes = apk(0x99, 701);
    let package_changed = metadata(20, "other.package", "other.Main", 2, &bytes, 0x5a);
    assert_eq!(
        install(&mut io, BASE, package_changed, &bytes),
        Err(Error::PackageChanged)
    );
    let signer_changed = metadata(
        21,
        old.metadata().package(),
        old.metadata().activity(),
        2,
        &bytes,
        0x91,
    );
    assert_eq!(
        install(&mut io, BASE, signer_changed, &bytes),
        Err(Error::SignerChanged)
    );
    let same_version = metadata(
        22,
        old.metadata().package(),
        old.metadata().activity(),
        1,
        &bytes,
        0x5a,
    );
    assert_eq!(
        install(&mut io, BASE, same_version, &bytes),
        Err(Error::VersionNotIncreasing)
    );
    assert_eq!(io.writes, 0);

    let (updated_bytes, updated_metadata) = second_package();
    install(&mut io, BASE, updated_metadata, &updated_bytes).unwrap();
    io.reset_observation();
    let lower_version = metadata(
        23,
        old.metadata().package(),
        old.metadata().activity(),
        1,
        &bytes,
        0x5a,
    );
    assert_eq!(
        install(&mut io, BASE, lower_version, &bytes),
        Err(Error::VersionNotIncreasing)
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn successful_updates_advance_generation_alternate_slots_and_stale_old_handles() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    let updated = install(&mut io, BASE, admitted, &bytes).unwrap();
    assert_eq!(updated.generation(), 2);
    assert_eq!(updated.slot(), 1);
    let mut output = vec![0; MAX_APK_BYTES];
    assert_eq!(
        read_blob(&mut io, BASE, &old, &mut output),
        Err(Error::StalePackage)
    );
    assert_eq!(
        read_blob(&mut io, BASE, &updated, &mut output).unwrap(),
        bytes.len()
    );
    assert_eq!(&output[..bytes.len()], bytes.as_slice());

    let (third_bytes, third_metadata) = third_package();
    let third = install(&mut io, BASE, third_metadata, &third_bytes).unwrap();
    assert_eq!(third.generation(), 3);
    assert_eq!(third.slot(), 0);
    io.reboot();
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(third));
    assert_eq!(
        read_blob(&mut io, BASE, &old, &mut output),
        Err(Error::StalePackage)
    );
    assert_eq!(
        read_blob(&mut io, BASE, &updated, &mut output),
        Err(Error::StalePackage)
    );
    assert_eq!(
        read_blob(&mut io, BASE, &third, &mut output).unwrap(),
        third_bytes.len()
    );
    assert_eq!(&output[..third_bytes.len()], third_bytes.as_slice());
}

#[test]
fn maximum_sized_apk_uses_all_payload_sectors() {
    let mut io = MemoryIo::formatted();
    let bytes = apk(0x61, MAX_APK_BYTES);
    let admitted = metadata(31, "max.apk", "max.apk.Main", 1, &bytes, 8);
    let installed = install(&mut io, BASE, admitted, &bytes).unwrap();
    let mut output = vec![0; MAX_APK_BYTES];
    assert_eq!(
        read_blob(&mut io, BASE, &installed, &mut output).unwrap(),
        MAX_APK_BYTES
    );
    assert_eq!(output, bytes);
}

#[test]
fn invalid_apk_inputs_do_not_touch_storage() {
    let mut io = MemoryIo::formatted();
    let empty_metadata = metadata(1, "p", "a", 1, b"x", 1);
    assert_eq!(
        install(&mut io, BASE, empty_metadata, b""),
        Err(Error::EmptyApk)
    );
    let oversized = vec![1; MAX_APK_BYTES + 1];
    let oversized_metadata = metadata(2, "p", "a", 1, &oversized, 1);
    assert_eq!(
        install(&mut io, BASE, oversized_metadata, &oversized),
        Err(Error::ApkTooLarge)
    );
    let bytes = b"actual";
    let wrong_digest = metadata(3, "p", "a", 1, b"different", 1);
    assert_eq!(
        install(&mut io, BASE, wrong_digest, bytes),
        Err(Error::ApkDigestMismatch)
    );
    assert_eq!(io.reads, 0);
    assert_eq!(io.writes, 0);
}

#[test]
fn precommit_blob_orphan_is_never_visible() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut io, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();
    io.set_fault(Fault::Flush {
        index: 0,
        commit_before_error: true,
        error: IoError::OutcomeUnknown,
    });
    assert_eq!(
        install(&mut io, BASE, admitted, &bytes),
        Err(Error::OutcomeUnknown)
    );
    io.reboot();
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn postcommit_ack_loss_recovers_the_new_package() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    let (second_bytes, second_metadata) = second_package();
    install(&mut io, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();
    let target_registry = BASE + REGISTRY_RELATIVE_LBAS[0];
    io.set_fault(Fault::ReadAfterCommit {
        lba: target_registry,
    });
    assert_eq!(
        install(&mut io, BASE, admitted, &bytes),
        Err(Error::OutcomeUnknown)
    );
    io.reboot();
    let recovered = recover(&mut io, BASE).unwrap().unwrap();
    assert_eq!(recovered.metadata(), &admitted);
    assert_eq!(recovered.generation(), 3);
    io.reset_observation();
    assert_eq!(install(&mut io, BASE, admitted, &bytes).unwrap(), recovered);
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn every_update_write_failure_preserves_the_previous_package() {
    let mut baseline = MemoryIo::formatted();
    first_package(&mut baseline);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut baseline, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();
    let write_count = BLOB_PAYLOAD_SECTORS as usize + 2;

    for fault_index in 0..write_count {
        let mut io = baseline.clone();
        io.set_fault(Fault::WriteBefore {
            index: fault_index,
            error: IoError::Device,
        });
        assert_eq!(
            install(&mut io, BASE, admitted, &bytes),
            Err(Error::Io(IoError::Device)),
            "write fault {fault_index}"
        );
        io.reboot();
        assert_eq!(
            recover(&mut io, BASE).unwrap(),
            Some(old),
            "write fault {fault_index}"
        );
    }
}

#[test]
fn every_torn_update_write_preserves_the_previous_package() {
    let mut baseline = MemoryIo::formatted();
    first_package(&mut baseline);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut baseline, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();
    let write_count = BLOB_PAYLOAD_SECTORS as usize + 2;

    for fault_index in 0..write_count {
        for torn_bytes in [0, 1, 37, SECTOR_SIZE - 1] {
            let mut io = baseline.clone();
            io.set_fault(Fault::TornWrite {
                index: fault_index,
                bytes: torn_bytes,
            });
            assert_eq!(
                install(&mut io, BASE, admitted, &bytes),
                Err(Error::OutcomeUnknown),
                "torn write {fault_index} after {torn_bytes} bytes"
            );
            io.reboot();
            assert_eq!(
                recover(&mut io, BASE).unwrap(),
                Some(old),
                "torn write {fault_index} after {torn_bytes} bytes"
            );
        }
    }
}

#[test]
fn each_flush_failure_has_only_old_or_complete_new_outcome() {
    let mut baseline = MemoryIo::formatted();
    first_package(&mut baseline);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut baseline, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();

    for (fault_index, commit_before_error) in [(0, false), (0, true), (1, false), (1, true)] {
        let mut io = baseline.clone();
        io.set_fault(Fault::Flush {
            index: fault_index,
            commit_before_error,
            error: IoError::OutcomeUnknown,
        });
        assert_eq!(
            install(&mut io, BASE, admitted, &bytes),
            Err(Error::OutcomeUnknown)
        );
        io.reboot();
        let recovered = recover(&mut io, BASE).unwrap().unwrap();
        if fault_index == 1 && commit_before_error {
            assert_eq!(recovered.metadata(), &admitted);
            assert_eq!(recovered.generation(), 3);
            io.reset_observation();
            assert_eq!(install(&mut io, BASE, admitted, &bytes).unwrap(), recovered);
            assert_eq!(io.writes, 0);
            assert_eq!(io.flushes, 0);
        } else {
            assert_eq!(recovered, old);
        }
    }
}

#[test]
fn definite_blob_flush_failures_preserve_the_previous_package() {
    let mut baseline = MemoryIo::formatted();
    first_package(&mut baseline);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut baseline, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();

    for error in [IoError::Device, IoError::RequiresReset] {
        let mut io = baseline.clone();
        io.set_fault(Fault::Flush {
            index: 0,
            commit_before_error: false,
            error,
        });
        assert_eq!(
            install(&mut io, BASE, admitted, &bytes),
            Err(Error::Io(error))
        );
        io.reboot();
        assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
    }
}

#[test]
fn commit_flush_device_error_is_still_reported_as_outcome_unknown() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    let (second_bytes, second_metadata) = second_package();
    let old = install(&mut io, BASE, second_metadata, &second_bytes).unwrap();
    let (bytes, admitted) = third_package();
    io.set_fault(Fault::Flush {
        index: 1,
        commit_before_error: false,
        error: IoError::Device,
    });
    assert_eq!(
        install(&mut io, BASE, admitted, &bytes),
        Err(Error::OutcomeUnknown)
    );
    io.reboot();
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn recovery_propagates_an_outcome_unknown_read() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    io.set_fault(Fault::ReadBefore {
        index: 1,
        error: IoError::OutcomeUnknown,
    });
    assert_eq!(recover(&mut io, BASE), Err(Error::OutcomeUnknown));
    assert_eq!(io.writes, 0);
}

#[test]
fn newest_registry_corruption_falls_back_to_previous_commit() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    io.corrupt_durable(BASE + REGISTRY_RELATIVE_LBAS[1], 48, 0x80);
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn newest_blob_header_corruption_falls_back_to_previous_commit() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    io.corrupt_durable(BASE + BLOB_RELATIVE_LBAS[1], 12, 0x40);
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn blob_header_epoch_is_enforced_even_with_a_valid_crc() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    let header_lba = BASE + BLOB_RELATIVE_LBAS[1];
    let mut header = io.visible[header_lba as usize];
    header[16] ^= 1;
    seal_record(&mut header);
    io.overwrite_durable(header_lba, header);
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn newest_blob_payload_corruption_falls_back_to_previous_commit() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    io.corrupt_durable(BASE + BLOB_RELATIVE_LBAS[1] + 1, 9, 0x01);
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn nonzero_padding_invalidates_newest_blob_and_falls_back() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    install(&mut io, BASE, admitted, &bytes).unwrap();
    let padding_lba = BASE + BLOB_RELATIVE_LBAS[1] + 1 + (bytes.len() / SECTOR_SIZE) as u64;
    let padding_offset = bytes.len() % SECTOR_SIZE;
    io.corrupt_durable(padding_lba, padding_offset, 0x01);
    assert_eq!(recover(&mut io, BASE).unwrap(), Some(old));
}

#[test]
fn stable_recovery_and_blob_reads_never_write() {
    let mut io = MemoryIo::formatted();
    let (bytes, _, installed) = first_package(&mut io);
    for _ in 0..3 {
        assert_eq!(recover(&mut io, BASE).unwrap(), Some(installed));
    }
    let mut output = vec![0; bytes.len()];
    assert_eq!(
        read_blob(&mut io, BASE, &installed, &mut output).unwrap(),
        bytes.len()
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn read_blob_checks_buffer_and_current_handle() {
    let mut io = MemoryIo::formatted();
    let (bytes, _, installed) = first_package(&mut io);
    let mut short = vec![0; bytes.len() - 1];
    assert_eq!(
        read_blob(&mut io, BASE, &installed, &mut short),
        Err(Error::BufferTooSmall {
            needed: bytes.len()
        })
    );
}

#[test]
fn superblock_crc_and_epoch_are_both_enforced() {
    let mut crc_io = MemoryIo::formatted();
    crc_io.corrupt_durable(BASE, 24, 0x01);
    assert_eq!(
        recover(&mut crc_io, BASE),
        Err(Error::Corrupt(Corruption::BadSuperblock))
    );

    let mut epoch_io = MemoryIo::formatted();
    let mut superblock = epoch_io.visible[BASE as usize];
    superblock[24] ^= 0x80;
    seal_record(&mut superblock);
    epoch_io.overwrite_durable(BASE, superblock);
    assert_eq!(
        recover(&mut epoch_io, BASE),
        Err(Error::Corrupt(Corruption::BadSuperblock))
    );
}

#[test]
fn registry_epoch_is_enforced_even_with_a_valid_crc() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    let registry_lba = BASE + REGISTRY_RELATIVE_LBAS[0];
    let mut registry = io.visible[registry_lba as usize];
    registry[16] ^= 1;
    seal_record(&mut registry);
    io.overwrite_durable(registry_lba, registry);
    assert_eq!(
        recover(&mut io, BASE),
        Err(Error::Corrupt(Corruption::NoValidRegistry))
    );
}

#[test]
fn all_published_record_types_carry_valid_crc_and_epoch() {
    let mut io = MemoryIo::formatted();
    first_package(&mut io);
    for relative in [
        SUPERBLOCK_RELATIVE_LBA,
        REGISTRY_RELATIVE_LBAS[0],
        BLOB_RELATIVE_LBAS[0],
    ] {
        let sector = &io.visible[(BASE + relative) as usize];
        assert!(valid_seal(sector));
        let epoch_range = if relative == SUPERBLOCK_RELATIVE_LBA {
            24..40
        } else {
            16..32
        };
        assert_eq!(sector[epoch_range], FORMAT_EPOCH);
    }
}

#[test]
fn exhausted_generation_rejects_update_before_writing() {
    let mut io = MemoryIo::formatted();
    let (_, _, _) = first_package(&mut io);
    let registry_lba = BASE + REGISTRY_RELATIVE_LBAS[0];
    let blob_lba = BASE + BLOB_RELATIVE_LBAS[0];
    let mut record = parse_registry(&io.visible[registry_lba as usize], 0).unwrap();
    record.header.generation = u64::MAX;
    let blob_header = encode_blob_header(&record.header);
    record.blob_header_sha256 = sha256(&blob_header);
    io.overwrite_durable(blob_lba, blob_header);
    io.overwrite_durable(registry_lba, encode_registry(&record));
    io.reset_observation();

    let (bytes, admitted) = second_package();
    assert_eq!(
        install(&mut io, BASE, admitted, &bytes),
        Err(Error::GenerationExhausted)
    );
    assert_eq!(io.writes, 0);
}

#[test]
fn unformatted_and_corrupt_nonempty_registry_are_distinct() {
    let mut unformatted = MemoryIo::new((BASE + VOLUME_SECTORS) as usize);
    assert_eq!(recover(&mut unformatted, BASE), Err(Error::Unformatted));

    let mut corrupt = MemoryIo::formatted();
    corrupt.visible[(BASE + REGISTRY_RELATIVE_LBAS[0]) as usize][0] = 1;
    corrupt.durable.clone_from(&corrupt.visible);
    assert_eq!(
        recover(&mut corrupt, BASE),
        Err(Error::Corrupt(Corruption::NoValidRegistry))
    );
}

#[test]
fn recover_state_distinguishes_empty_installed_and_removed_with_exact_evidence() {
    let mut io = MemoryIo::formatted();
    assert_eq!(recover_state(&mut io, BASE).unwrap(), PackageState::Empty);

    let (bytes, admitted, installed) = first_package(&mut io);
    assert_eq!(
        recover_state(&mut io, BASE).unwrap(),
        PackageState::Installed(installed)
    );
    io.reset_observation();
    let removed = uninstall(
        &mut io,
        BASE,
        0xfeed_0001,
        &installed,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();

    assert_eq!(removed.operation_id(), 0xfeed_0001);
    assert_eq!(removed.package(), admitted.package());
    assert_eq!(removed.generation(), 2);
    assert_eq!(removed.last_generation(), 1);
    assert_eq!(removed.expected_generation(), 1);
    assert_eq!(removed.last_metadata(), &admitted);
    assert_eq!(removed.version_code(), admitted.version_code());
    assert_eq!(removed.apk_length(), bytes.len());
    assert_eq!(removed.apk_sha256(), admitted.apk_sha256());
    assert_eq!(removed.signer_cert_sha256(), admitted.signer_cert_sha256());
    assert_eq!(removed.last_blob_slot(), installed.slot());
    assert_eq!(removed.disposition(), DataDisposition::NoManagedPackageData);
    assert_eq!(removed.last_installed(), installed);
    assert_eq!(io.writes, 2);
    assert_eq!(io.flushes, 2);
    assert_eq!(
        io.touched_writes,
        vec![
            BASE + REGISTRY_RELATIVE_LBAS[1],
            BASE + REGISTRY_RELATIVE_LBAS[0]
        ]
    );

    let first = io.durable[(BASE + REGISTRY_RELATIVE_LBAS[0]) as usize];
    let second = io.durable[(BASE + REGISTRY_RELATIVE_LBAS[1]) as usize];
    assert_eq!(first, second);
    assert_eq!(first[..8], TOMBSTONE_MAGIC);
    assert!(valid_seal(&first));
    assert_eq!(first[16..32], FORMAT_EPOCH);
    assert_eq!(parse_tombstone(&first).unwrap(), removed);

    io.reboot();
    assert_eq!(
        recover_state(&mut io, BASE).unwrap(),
        PackageState::Removed(removed)
    );
    assert_eq!(recover(&mut io, BASE).unwrap(), None);
    let mut output = vec![0; bytes.len()];
    assert_eq!(
        read_blob(&mut io, BASE, &removed.last_installed(), &mut output),
        Err(Error::StalePackage)
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn uninstall_validation_and_exact_full_mirror_replay_are_write_free() {
    let mut io = MemoryIo::formatted();
    let (_, _, old) = first_package(&mut io);
    let (bytes, admitted) = second_package();
    let current = install(&mut io, BASE, admitted, &bytes).unwrap();
    io.reset_observation();

    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            0,
            &current,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::InvalidMetadata(
            MetadataError::UninstallOperationIdZero
        ))
    );
    assert_eq!(io.reads, 0);
    assert_eq!(io.writes, 0);

    let removed = uninstall(
        &mut io,
        BASE,
        71,
        &current,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    io.reboot();
    let reconstructed = match recover_state(&mut io, BASE).unwrap() {
        PackageState::Removed(removed) => removed.last_installed(),
        other => panic!("expected removed state, got {other:?}"),
    };
    io.reset_observation();
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            71,
            &reconstructed,
            DataDisposition::NoManagedPackageData
        )
        .unwrap(),
        removed
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);

    io.reset_observation();
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            72,
            &reconstructed,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::NotInstalled)
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);

    io.reset_observation();
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            71,
            &old,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::TransactionConflict)
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);

    let mut empty = MemoryIo::formatted();
    assert_eq!(
        uninstall(
            &mut empty,
            BASE,
            71,
            &current,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::NotInstalled)
    );
    assert_eq!(empty.writes, 0);
}

#[test]
fn partial_tombstone_replay_repairs_the_second_mirror_then_becomes_write_free() {
    let mut io = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut io);
    io.set_fault(Fault::WriteBefore {
        index: 1,
        error: IoError::Device,
    });
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            81,
            &installed,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::Io(IoError::Device))
    );

    io.reboot();
    let removed = match recover_state(&mut io, BASE).unwrap() {
        PackageState::Removed(removed) => removed,
        other => panic!("expected partial removed state, got {other:?}"),
    };
    assert_ne!(
        io.durable[(BASE + REGISTRY_RELATIVE_LBAS[0]) as usize],
        io.durable[(BASE + REGISTRY_RELATIVE_LBAS[1]) as usize]
    );
    io.reset_observation();
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            81,
            &removed.last_installed(),
            DataDisposition::NoManagedPackageData
        )
        .unwrap(),
        removed
    );
    assert_eq!(io.writes, 1);
    assert_eq!(io.flushes, 1);
    assert_eq!(
        io.durable[(BASE + REGISTRY_RELATIVE_LBAS[0]) as usize],
        io.durable[(BASE + REGISTRY_RELATIVE_LBAS[1]) as usize]
    );

    io.reset_observation();
    assert_eq!(
        uninstall(
            &mut io,
            BASE,
            81,
            &removed.last_installed(),
            DataDisposition::NoManagedPackageData
        )
        .unwrap(),
        removed
    );
    assert_eq!(io.writes, 0);
    assert_eq!(io.flushes, 0);
}

#[test]
fn every_uninstall_write_and_torn_write_has_installed_or_removed_recovery() {
    let mut baseline = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut baseline);

    for fault_index in 0..2 {
        let mut io = baseline.clone();
        io.set_fault(Fault::WriteBefore {
            index: fault_index,
            error: IoError::Device,
        });
        assert_eq!(
            uninstall(
                &mut io,
                BASE,
                91,
                &installed,
                DataDisposition::NoManagedPackageData
            ),
            Err(Error::Io(IoError::Device)),
            "write fault {fault_index}"
        );
        io.reboot();
        match recover_state(&mut io, BASE).unwrap() {
            PackageState::Installed(recovered) => {
                assert_eq!(fault_index, 0);
                assert_eq!(recovered, installed);
            }
            PackageState::Removed(removed) => {
                assert_eq!(fault_index, 1);
                io.reset_observation();
                uninstall(
                    &mut io,
                    BASE,
                    91,
                    &removed.last_installed(),
                    DataDisposition::NoManagedPackageData,
                )
                .unwrap();
                assert_eq!(io.writes, 1);
            }
            PackageState::Empty => panic!("uninstall fault produced empty state"),
        }
    }

    for fault_index in 0..2 {
        for torn_bytes in [0, 1, 37, SECTOR_SIZE - 1] {
            let mut io = baseline.clone();
            io.set_fault(Fault::TornWrite {
                index: fault_index,
                bytes: torn_bytes,
            });
            assert_eq!(
                uninstall(
                    &mut io,
                    BASE,
                    92,
                    &installed,
                    DataDisposition::NoManagedPackageData
                ),
                Err(Error::OutcomeUnknown),
                "torn write {fault_index} after {torn_bytes} bytes"
            );
            io.reboot();
            match recover_state(&mut io, BASE).unwrap() {
                PackageState::Installed(recovered) => {
                    assert_eq!(fault_index, 0);
                    assert_eq!(recovered, installed);
                }
                PackageState::Removed(removed) => {
                    assert_eq!(fault_index, 1);
                    io.reset_observation();
                    uninstall(
                        &mut io,
                        BASE,
                        92,
                        &removed.last_installed(),
                        DataDisposition::NoManagedPackageData,
                    )
                    .unwrap();
                    assert_eq!(io.writes, 1);
                }
                PackageState::Empty => panic!("torn uninstall produced empty state"),
            }
        }
    }
}

#[test]
fn uninstall_flush_and_readback_ack_failures_are_recoverable_and_replayable() {
    let mut baseline = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut baseline);

    for (fault_index, commit_before_error) in [(0, false), (0, true), (1, false), (1, true)] {
        let mut io = baseline.clone();
        io.set_fault(Fault::Flush {
            index: fault_index,
            commit_before_error,
            error: IoError::Device,
        });
        assert_eq!(
            uninstall(
                &mut io,
                BASE,
                101,
                &installed,
                DataDisposition::NoManagedPackageData
            ),
            Err(Error::OutcomeUnknown)
        );
        io.reboot();
        let state = recover_state(&mut io, BASE).unwrap();
        if fault_index == 0 && !commit_before_error {
            assert_eq!(state, PackageState::Installed(installed));
        } else {
            let PackageState::Removed(removed) = state else {
                panic!("durable tombstone was not recovered: {state:?}");
            };
            io.reset_observation();
            uninstall(
                &mut io,
                BASE,
                101,
                &removed.last_installed(),
                DataDisposition::NoManagedPackageData,
            )
            .unwrap();
            let expected_writes = usize::from(!(fault_index == 1 && commit_before_error));
            assert_eq!(io.writes, expected_writes);
        }
    }

    for (minimum_successful_flushes, registry_index, expected_repair_writes) in
        [(1, 1_u8, 1), (2, 0_u8, 0)]
    {
        let mut io = baseline.clone();
        io.set_fault(Fault::ReadAfterFlush {
            lba: BASE + REGISTRY_RELATIVE_LBAS[usize::from(registry_index)],
            minimum_successful_flushes,
        });
        assert_eq!(
            uninstall(
                &mut io,
                BASE,
                102,
                &installed,
                DataDisposition::NoManagedPackageData
            ),
            Err(Error::OutcomeUnknown)
        );
        io.reboot();
        let PackageState::Removed(removed) = recover_state(&mut io, BASE).unwrap() else {
            panic!("ack loss failed to publish removed state");
        };
        io.reset_observation();
        uninstall(
            &mut io,
            BASE,
            102,
            &removed.last_installed(),
            DataDisposition::NoManagedPackageData,
        )
        .unwrap();
        assert_eq!(io.writes, expected_repair_writes);
    }
}

#[test]
fn one_cleared_or_corrupt_tombstone_never_revives_the_old_package() {
    let mut baseline = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut baseline);
    let removed = uninstall(
        &mut baseline,
        BASE,
        111,
        &installed,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();

    for relative_lba in REGISTRY_RELATIVE_LBAS {
        let lba = BASE + relative_lba;
        let mut cleared = baseline.clone();
        cleared.overwrite_durable(lba, [0; SECTOR_SIZE]);
        assert_eq!(
            recover_state(&mut cleared, BASE).unwrap(),
            PackageState::Removed(removed)
        );
        assert_eq!(recover(&mut cleared, BASE).unwrap(), None);

        let mut corrupt = baseline.clone();
        corrupt.corrupt_durable(lba, 15, 0x80);
        assert_eq!(
            recover_state(&mut corrupt, BASE).unwrap(),
            PackageState::Removed(removed)
        );
        assert_eq!(recover(&mut corrupt, BASE).unwrap(), None);
    }
}

#[test]
fn only_logically_identical_same_generation_tombstones_are_unambiguous() {
    let mut io = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut io);
    let removed = uninstall(
        &mut io,
        BASE,
        121,
        &installed,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    assert_eq!(
        recover_state(&mut io, BASE).unwrap(),
        PackageState::Removed(removed)
    );

    let mut conflicting = removed;
    conflicting.operation_id += 1;
    io.overwrite_durable(
        BASE + REGISTRY_RELATIVE_LBAS[1],
        encode_tombstone(&conflicting),
    );
    assert_eq!(
        recover_state(&mut io, BASE),
        Err(Error::Corrupt(Corruption::AmbiguousGeneration))
    );
}

#[test]
fn reinstall_after_removed_enforces_identity_signer_version_and_digest() {
    let mut baseline = MemoryIo::formatted();
    let (_, _, first) = first_package(&mut baseline);
    let (last_bytes, last_metadata) = second_package();
    let last = install(&mut baseline, BASE, last_metadata, &last_bytes).unwrap();
    let removed = uninstall(
        &mut baseline,
        BASE,
        131,
        &last,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    baseline.reboot();

    let changed_bytes = apk(0x44, 901);
    for (candidate, expected_error) in [
        (
            metadata(90, "other.package", "other.Main", 3, &changed_bytes, 0x5a),
            Error::PackageChanged,
        ),
        (
            metadata(
                91,
                last.metadata().package(),
                last.metadata().activity(),
                3,
                &changed_bytes,
                0x91,
            ),
            Error::SignerChanged,
        ),
        (
            metadata(
                92,
                last.metadata().package(),
                last.metadata().activity(),
                first.metadata().version_code(),
                &changed_bytes,
                0x5a,
            ),
            Error::VersionNotIncreasing,
        ),
        (
            metadata(
                93,
                last.metadata().package(),
                last.metadata().activity(),
                last.metadata().version_code(),
                &changed_bytes,
                0x5a,
            ),
            Error::VersionNotIncreasing,
        ),
    ] {
        let mut io = baseline.clone();
        io.reset_observation();
        assert_eq!(
            install(&mut io, BASE, candidate, &changed_bytes),
            Err(expected_error)
        );
        assert_eq!(io.writes, 0);
        assert_eq!(io.flushes, 0);
        assert_eq!(
            recover_state(&mut io, BASE).unwrap(),
            PackageState::Removed(removed)
        );
    }

    let same_digest = metadata(
        94,
        last.metadata().package(),
        "com.bndroid.fixture.ReinstalledActivity",
        last.metadata().version_code(),
        &last_bytes,
        0x5a,
    );
    let mut same = baseline.clone();
    let reinstalled = install(&mut same, BASE, same_digest, &last_bytes).unwrap();
    assert_eq!(reinstalled.generation(), removed.generation() + 1);
    assert_eq!(reinstalled.slot(), 1 - removed.last_blob_slot());
    assert_eq!(
        recover_state(&mut same, BASE).unwrap(),
        PackageState::Installed(reinstalled)
    );

    let (higher_bytes, higher_metadata) = third_package();
    let mut higher = baseline;
    let upgraded = install(&mut higher, BASE, higher_metadata, &higher_bytes).unwrap();
    assert_eq!(upgraded.generation(), removed.generation() + 1);
    assert_eq!(
        recover_state(&mut higher, BASE).unwrap(),
        PackageState::Installed(upgraded)
    );
}

#[test]
fn failed_or_corrupt_reinstall_falls_back_to_removed_not_the_old_install() {
    let mut baseline = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut baseline);
    let removed = uninstall(
        &mut baseline,
        BASE,
        141,
        &installed,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    baseline.reboot();
    let (bytes, admitted) = second_package();
    let write_count = BLOB_PAYLOAD_SECTORS as usize + 2;

    for fault_index in 0..write_count {
        let mut io = baseline.clone();
        io.set_fault(Fault::WriteBefore {
            index: fault_index,
            error: IoError::Device,
        });
        assert_eq!(
            install(&mut io, BASE, admitted, &bytes),
            Err(Error::Io(IoError::Device)),
            "reinstall write fault {fault_index}"
        );
        io.reboot();
        assert_eq!(
            recover_state(&mut io, BASE).unwrap(),
            PackageState::Removed(removed),
            "reinstall write fault {fault_index}"
        );
    }

    for fault_index in 0..write_count {
        for torn_bytes in [0, 1, 37, SECTOR_SIZE - 1] {
            let mut io = baseline.clone();
            io.set_fault(Fault::TornWrite {
                index: fault_index,
                bytes: torn_bytes,
            });
            assert_eq!(
                install(&mut io, BASE, admitted, &bytes),
                Err(Error::OutcomeUnknown),
                "reinstall torn write {fault_index} after {torn_bytes} bytes"
            );
            io.reboot();
            assert_eq!(
                recover_state(&mut io, BASE).unwrap(),
                PackageState::Removed(removed),
                "reinstall torn write {fault_index} after {torn_bytes} bytes"
            );
        }
    }

    for (fault_index, commit_before_error) in [(0, false), (0, true), (1, false), (1, true)] {
        let mut io = baseline.clone();
        io.set_fault(Fault::Flush {
            index: fault_index,
            commit_before_error,
            error: IoError::OutcomeUnknown,
        });
        assert_eq!(
            install(&mut io, BASE, admitted, &bytes),
            Err(Error::OutcomeUnknown)
        );
        io.reboot();
        let state = recover_state(&mut io, BASE).unwrap();
        if fault_index == 1 && commit_before_error {
            let PackageState::Installed(package) = state else {
                panic!("durable reinstall commit was not recovered: {state:?}");
            };
            assert_eq!(package.metadata(), &admitted);
        } else {
            assert_eq!(state, PackageState::Removed(removed));
        }
    }

    let mut committed = baseline;
    let reinstalled = install(&mut committed, BASE, admitted, &bytes).unwrap();
    for (relative, offset) in [
        (REGISTRY_RELATIVE_LBAS[usize::from(reinstalled.slot())], 48),
        (BLOB_RELATIVE_LBAS[usize::from(reinstalled.slot())], 12),
        (BLOB_RELATIVE_LBAS[usize::from(reinstalled.slot())] + 1, 9),
    ] {
        let mut io = committed.clone();
        io.corrupt_durable(BASE + relative, offset, 0x80);
        assert_eq!(
            recover_state(&mut io, BASE).unwrap(),
            PackageState::Removed(removed)
        );
        assert_eq!(recover(&mut io, BASE).unwrap(), None);
    }
}

#[test]
fn uninstall_and_reinstall_generation_exhaustion_fail_before_writing() {
    let mut installed_io = MemoryIo::formatted();
    first_package(&mut installed_io);
    let registry_lba = BASE + REGISTRY_RELATIVE_LBAS[0];
    let blob_lba = BASE + BLOB_RELATIVE_LBAS[0];
    let mut record = parse_registry(&installed_io.visible[registry_lba as usize], 0).unwrap();
    record.header.generation = u64::MAX;
    let blob_header = encode_blob_header(&record.header);
    record.blob_header_sha256 = sha256(&blob_header);
    installed_io.overwrite_durable(blob_lba, blob_header);
    installed_io.overwrite_durable(registry_lba, encode_registry(&record));
    let PackageState::Installed(max_installed) = recover_state(&mut installed_io, BASE).unwrap()
    else {
        panic!("expected max-generation install");
    };
    installed_io.reset_observation();
    assert_eq!(
        uninstall(
            &mut installed_io,
            BASE,
            151,
            &max_installed,
            DataDisposition::NoManagedPackageData
        ),
        Err(Error::GenerationExhausted)
    );
    assert_eq!(installed_io.writes, 0);
    assert_eq!(installed_io.flushes, 0);

    let mut removed_io = MemoryIo::formatted();
    let (_, _, installed) = first_package(&mut removed_io);
    let mut removed = uninstall(
        &mut removed_io,
        BASE,
        152,
        &installed,
        DataDisposition::NoManagedPackageData,
    )
    .unwrap();
    removed.generation = u64::MAX;
    removed.last_generation = u64::MAX - 1;
    let tombstone = encode_tombstone(&removed);
    for relative in REGISTRY_RELATIVE_LBAS {
        removed_io.overwrite_durable(BASE + relative, tombstone);
    }
    assert_eq!(
        recover_state(&mut removed_io, BASE).unwrap(),
        PackageState::Removed(removed)
    );
    removed_io.reset_observation();
    let (bytes, admitted) = second_package();
    assert_eq!(
        install(&mut removed_io, BASE, admitted, &bytes),
        Err(Error::GenerationExhausted)
    );
    assert_eq!(removed_io.writes, 0);
    assert_eq!(removed_io.flushes, 0);

    removed_io.overwrite_durable(BASE + REGISTRY_RELATIVE_LBAS[0], [0; SECTOR_SIZE]);
    removed_io.reset_observation();
    assert_eq!(
        uninstall(
            &mut removed_io,
            BASE,
            152,
            &removed.last_installed(),
            DataDisposition::NoManagedPackageData
        )
        .unwrap(),
        removed
    );
    assert_eq!(removed_io.writes, 1);
    assert_eq!(removed_io.flushes, 1);
}
