//! Fixed-capacity composition of independent package-store v1 volumes.
//!
//! Each package keeps the existing double-registry/double-blob transaction.
//! Slot zero starts at the legacy volume base, so an ABI-54 single-package
//! disk remains valid as the first entry after the partition is extended.

use crate::{
    DataDisposition, Error, InstallMetadata, InstalledPackage, PackageState, RemovedPackage,
    SectorIo, VOLUME_SECTORS, format, install, read_blob, recover_state, uninstall,
};

pub const MULTI_PACKAGE_CAPACITY: usize = 2;
pub const MULTI_PACKAGE_VOLUME_SECTORS: u64 = VOLUME_SECTORS * MULTI_PACKAGE_CAPACITY as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultiPackageError {
    Store(Error),
    Capacity,
    DuplicatePackage,
    PackageNotFound,
    SlotOutOfRange,
}

impl MultiPackageError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Store(error) => error.as_str(),
            Self::Capacity => "multi-package store has no empty package slot",
            Self::DuplicatePackage => "the same package identity appears in multiple package slots",
            Self::PackageNotFound => "package is not installed in the multi-package store",
            Self::SlotOutOfRange => "multi-package slot index is out of range",
        }
    }
}

impl From<Error> for MultiPackageError {
    fn from(value: Error) -> Self {
        Self::Store(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiFormatEvidence {
    newly_formatted: u8,
    already_formatted: u8,
}

impl MultiFormatEvidence {
    pub const fn newly_formatted(self) -> u8 {
        self.newly_formatted
    }

    pub const fn already_formatted(self) -> u8 {
        self.already_formatted
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiInstalledPackage {
    volume_index: u8,
    package: InstalledPackage,
}

impl MultiInstalledPackage {
    pub const fn from_parts(volume_index: u8, package: InstalledPackage) -> Self {
        Self {
            volume_index,
            package,
        }
    }

    pub const fn volume_index(self) -> u8 {
        self.volume_index
    }

    pub const fn package(self) -> InstalledPackage {
        self.package
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiPackageCatalog {
    states: [PackageState; MULTI_PACKAGE_CAPACITY],
    formatted: [bool; MULTI_PACKAGE_CAPACITY],
}

impl MultiPackageCatalog {
    pub const fn states(&self) -> &[PackageState; MULTI_PACKAGE_CAPACITY] {
        &self.states
    }

    pub const fn formatted_slots(&self) -> &[bool; MULTI_PACKAGE_CAPACITY] {
        &self.formatted
    }

    pub fn all_formatted(&self) -> bool {
        self.formatted.iter().all(|formatted| *formatted)
    }

    pub fn any_formatted(&self) -> bool {
        self.formatted.iter().any(|formatted| *formatted)
    }

    pub fn installed_count(&self) -> u8 {
        self.states
            .iter()
            .filter(|state| matches!(state, PackageState::Installed(_)))
            .count() as u8
    }

    pub fn retained_identity_count(&self) -> u8 {
        self.states
            .iter()
            .filter(|state| !matches!(state, PackageState::Empty))
            .count() as u8
    }

    pub fn find_installed(&self, package: &str) -> Option<MultiInstalledPackage> {
        self.states
            .iter()
            .enumerate()
            .find_map(|(index, state)| match state {
                PackageState::Installed(installed) if installed.metadata().package() == package => {
                    Some(MultiInstalledPackage {
                        volume_index: index as u8,
                        package: *installed,
                    })
                }
                _ => None,
            })
    }

    pub fn find_removed(&self, package: &str) -> Option<(u8, RemovedPackage)> {
        self.states
            .iter()
            .enumerate()
            .find_map(|(index, state)| match state {
                PackageState::Removed(removed) if removed.package() == package => {
                    Some((index as u8, *removed))
                }
                _ => None,
            })
    }
}

/// Makes both subvolumes available without rewriting an already formatted
/// slot. If power is lost after formatting only one slot, retry resumes at the
/// remaining virgin slot.
pub fn ensure_multi_formatted<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
) -> Result<MultiFormatEvidence, MultiPackageError> {
    check_multi_bounds(io, volume_start)?;
    let mut newly_formatted = 0_u8;
    let mut already_formatted = 0_u8;
    for index in 0..MULTI_PACKAGE_CAPACITY {
        let base = slot_base(volume_start, index)?;
        match recover_state(io, base) {
            Ok(_) => already_formatted = already_formatted.saturating_add(1),
            Err(Error::Unformatted) => {
                format(io, base)?;
                newly_formatted = newly_formatted.saturating_add(1);
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(MultiFormatEvidence {
        newly_formatted,
        already_formatted,
    })
}

pub fn recover_multi<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
) -> Result<MultiPackageCatalog, MultiPackageError> {
    let catalog = inspect_multi(io, volume_start)?;
    if !catalog.all_formatted() {
        return Err(Error::Unformatted.into());
    }
    Ok(catalog)
}

/// Inspects every fixed subvolume without formatting virgin slots.
///
/// This is used by the runtime-confirmed install flow: an existing legacy
/// first volume remains readable while the second extended slot stays
/// untouched until the user accepts an install transaction.
pub fn inspect_multi<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
) -> Result<MultiPackageCatalog, MultiPackageError> {
    check_multi_bounds(io, volume_start)?;
    let mut states = [PackageState::Empty; MULTI_PACKAGE_CAPACITY];
    let mut formatted = [false; MULTI_PACKAGE_CAPACITY];
    for (index, state) in states.iter_mut().enumerate() {
        match recover_state(io, slot_base(volume_start, index)?) {
            Ok(recovered) => {
                *state = recovered;
                formatted[index] = true;
            }
            Err(Error::Unformatted) => {}
            Err(error) => return Err(error.into()),
        }
    }
    validate_unique_identities(&states)?;
    Ok(MultiPackageCatalog { states, formatted })
}

/// Installs a new identity into the first empty subvolume, or updates/reinstalls
/// the unique subvolume already retaining that package identity.
pub fn install_multi<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    metadata: InstallMetadata,
    apk: &[u8],
) -> Result<MultiInstalledPackage, MultiPackageError> {
    let catalog = inspect_multi(io, volume_start)?;
    let package = metadata.package();
    let mut identity_index = None;
    let mut empty_index = None;
    for (index, state) in catalog.states.iter().enumerate() {
        if state_package(state).is_some_and(|existing| existing == package) {
            if identity_index.replace(index).is_some() {
                return Err(MultiPackageError::DuplicatePackage);
            }
        } else if matches!(state, PackageState::Empty) && empty_index.is_none() {
            empty_index = Some(index);
        }
    }
    let index = identity_index
        .or(empty_index)
        .ok_or(MultiPackageError::Capacity)?;
    let installed = install(io, slot_base(volume_start, index)?, metadata, apk)?;
    Ok(MultiInstalledPackage {
        volume_index: index as u8,
        package: installed,
    })
}

pub fn uninstall_multi<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    operation_id: u64,
    expected: MultiInstalledPackage,
    disposition: DataDisposition,
) -> Result<RemovedPackage, MultiPackageError> {
    let index = usize::from(expected.volume_index);
    if index >= MULTI_PACKAGE_CAPACITY {
        return Err(MultiPackageError::SlotOutOfRange);
    }
    let catalog = inspect_multi(io, volume_start)?;
    let identity_matches = match catalog.states[index] {
        PackageState::Installed(installed) => installed == expected.package,
        PackageState::Removed(removed) => removed.last_installed() == expected.package,
        PackageState::Empty => false,
    };
    if !identity_matches {
        return Err(MultiPackageError::PackageNotFound);
    }
    Ok(uninstall(
        io,
        slot_base(volume_start, index)?,
        operation_id,
        &expected.package,
        disposition,
    )?)
}

pub fn read_multi_blob<I: SectorIo>(
    io: &mut I,
    volume_start: u64,
    package: MultiInstalledPackage,
    output: &mut [u8],
) -> Result<usize, MultiPackageError> {
    let index = usize::from(package.volume_index);
    if index >= MULTI_PACKAGE_CAPACITY {
        return Err(MultiPackageError::SlotOutOfRange);
    }
    Ok(read_blob(
        io,
        slot_base(volume_start, index)?,
        &package.package,
        output,
    )?)
}

fn validate_unique_identities(
    states: &[PackageState; MULTI_PACKAGE_CAPACITY],
) -> Result<(), MultiPackageError> {
    for first in 0..MULTI_PACKAGE_CAPACITY {
        let Some(first_package) = state_package(&states[first]) else {
            continue;
        };
        for second_state in states.iter().skip(first + 1) {
            if state_package(second_state)
                .is_some_and(|second_package| second_package == first_package)
            {
                return Err(MultiPackageError::DuplicatePackage);
            }
        }
    }
    Ok(())
}

fn state_package(state: &PackageState) -> Option<&str> {
    match state {
        PackageState::Empty => None,
        PackageState::Installed(installed) => Some(installed.metadata().package()),
        PackageState::Removed(removed) => Some(removed.package()),
    }
}

fn check_multi_bounds<I: SectorIo>(io: &I, volume_start: u64) -> Result<(), MultiPackageError> {
    let end = volume_start
        .checked_add(MULTI_PACKAGE_VOLUME_SECTORS)
        .ok_or(MultiPackageError::Store(Error::VolumeBounds))?;
    if end > io.sector_count() {
        return Err(MultiPackageError::Store(Error::VolumeBounds));
    }
    Ok(())
}

fn slot_base(volume_start: u64, index: usize) -> Result<u64, MultiPackageError> {
    if index >= MULTI_PACKAGE_CAPACITY {
        return Err(MultiPackageError::SlotOutOfRange);
    }
    volume_start
        .checked_add(index as u64 * VOLUME_SECTORS)
        .ok_or(MultiPackageError::Store(Error::VolumeBounds))
}
