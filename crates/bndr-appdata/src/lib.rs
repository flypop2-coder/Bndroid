#![no_std]
#![deny(unsafe_code)]

//! Allocation-free, crash-safe application data for a fixed Bndroid volume.
//!
//! The on-disk format uses an immutable superblock, two checkpoints, and two
//! fixed snapshot banks. A mutation rewrites the inactive bank in full, flushes
//! it, writes the inactive checkpoint, flushes again, and then validates a full
//! readback. The selected bank is therefore always either the old complete
//! snapshot or the new complete snapshot; allocation and garbage collection are
//! deliberately outside this small storage core.

use core::array;

pub const SECTOR_SIZE: usize = 512;
pub type Sector = [u8; SECTOR_SIZE];

pub const VOLUME_SECTORS: u64 = 1_920;
pub const MAX_ENTRIES: usize = 32;
pub const MAX_PATH_BYTES: usize = 64;
pub const MAX_PATH_DEPTH: u8 = 4;
pub const MAX_FILE_BYTES: usize = 4_096;
pub const MAX_LIVE_PAYLOAD_BYTES: usize = 128 * 1_024;

pub const CHECKPOINT_RELATIVE_LBAS: [u64; 2] = [1, 2];
pub const BANK_RELATIVE_LBAS: [u64; 2] = [3, 961];
pub const BANK_SECTORS: u64 = 958;
pub const SNAPSHOT_SECTORS: u64 = 289;

/// GPT type UUID `B8F4D2A2-7C3E-4B91-A6D5-0F2E9C781355`, in GPT byte order.
pub const APPDATA_TYPE_GUID: [u8; 16] = [
    0xa2, 0xd2, 0xf4, 0xb8, 0x3e, 0x7c, 0x91, 0x4b, 0xa6, 0xd5, 0x0f, 0x2e, 0x9c, 0x78, 0x13, 0x55,
];

/// GPT partition UUID `D25CF154-A879-4E5A-9364-67B2D8905401`, in GPT byte order.
/// This is also the on-disk format epoch and prevents mounting another volume's
/// checkpoints under this format instance.
pub const FORMAT_EPOCH: [u8; 16] = [
    0x54, 0xf1, 0x5c, 0xd2, 0x79, 0xa8, 0x5a, 0x4e, 0x93, 0x64, 0x67, 0xb2, 0xd8, 0x90, 0x54, 0x01,
];

const FORMAT_VERSION: u32 = 1;
const SUPERBLOCK_MAGIC: [u8; 8] = *b"BNDAPS01";
const FORMAT_INTENT_MAGIC: [u8; 8] = *b"BNDAPI01";
const CHECKPOINT_MAGIC: [u8; 8] = *b"BNDAPC01";
const MANIFEST_MAGIC: [u8; 8] = *b"BNDAPM01";
const ENTRY_MAGIC: [u8; 8] = *b"BNDAPE01";

const MANIFEST_RELATIVE_LBA: u64 = 0;
const ENTRY_RELATIVE_LBA: u64 = 1;
const DATA_RELATIVE_LBA: u64 = 33;
const DATA_SECTORS_PER_ENTRY: u64 = 8;

const SEAL_FNV_OFFSET: usize = 496;
const SEAL_CRC_OFFSET: usize = 504;
const SEAL_WITNESS_OFFSET: usize = 508;
const SEAL_WITNESS: u32 = 0x4150_5044;
const GENERATION_WITNESS: u64 = 0x8d6e_c31a_5079_b4f2;

const _: () =
    assert!(DATA_RELATIVE_LBA + MAX_ENTRIES as u64 * DATA_SECTORS_PER_ENTRY == SNAPSHOT_SECTORS);
const _: () = assert!(BANK_RELATIVE_LBAS[0] + BANK_SECTORS <= BANK_RELATIVE_LBAS[1]);
const _: () = assert!(BANK_RELATIVE_LBAS[1] + BANK_SECTORS <= VOLUME_SECTORS);
const _: () = assert!(MAX_ENTRIES * MAX_FILE_BYTES == MAX_LIVE_PAYLOAD_BYTES);

/// Stable application identity. It is intentionally independent of a process
/// ID or process generation.
pub type Principal = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoError {
    OutOfBounds,
    Device,
    RequiresReset,
    OutcomeUnknown,
}

/// Synchronous sector boundary required by the durable transaction.
///
/// A successful `flush` must make every earlier successful write durable.
/// Operations are issued one at a time. Implementations must classify device
/// timeouts whose completion is not known as [`IoError::OutcomeUnknown`] and
/// devices that cannot accept more I/O as [`IoError::RequiresReset`].
///
/// Normal copy-on-write mutations tolerate torn writes because they target an
/// inactive bank. The first format intent has no earlier trusted on-disk bit:
/// full first-format power-loss recovery therefore requires an atomic 512-byte
/// write for relative sector zero. A torn intent is intentionally rejected
/// rather than guessed to be ours.
pub trait DurableVolumeIo {
    fn sector_count(&self) -> u64;
    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError>;
    fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), IoError>;
    fn flush(&mut self) -> Result<(), IoError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    Empty,
    TooLong,
    TooDeep,
    Absolute,
    TrailingSlash,
    EmptyComponent,
    DotComponent,
    ContainsNul,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Corruption {
    BadSuperblock,
    NoValidSnapshot,
    BadCheckpoint,
    BadManifest,
    BadEntry,
    DuplicateEntry,
    MissingParent,
    CountMismatch,
    InvalidEntryData,
    SnapshotDigestMismatch,
    AmbiguousGeneration,
    GenerationGap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplaceCondition {
    /// Create or atomically replace regardless of the current entry revision.
    Any,
    /// Succeed only when the path does not exist.
    Absent,
    /// Succeed only when the existing file has this entry revision.
    Generation(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    VolumeBounds,
    Unformatted,
    AlreadyFormatted,
    NotVirgin,
    Io(IoError),
    OutcomeUnknown,
    RequiresReset,
    Corrupt(Corruption),
    InvalidPrincipal,
    InvalidPath(PathError),
    NotFound,
    AlreadyExists,
    NotDirectory,
    IsDirectory,
    NotEmpty,
    FileTooLarge,
    BufferTooSmall {
        needed: usize,
    },
    OutOfSpace,
    Conflict {
        expected: ReplaceCondition,
        actual_generation: Option<u64>,
    },
    GenerationExhausted,
    VerificationFailed,
}

impl Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VolumeBounds => "app-data volume is outside the block device",
            Self::Unformatted => "app-data volume is unformatted",
            Self::AlreadyFormatted => "app-data volume is already formatted",
            Self::NotVirgin => "app-data volume is not all zero",
            Self::Io(_) => "app-data block I/O failed",
            Self::OutcomeUnknown => "app-data mutation outcome is unknown",
            Self::RequiresReset => "app-data device requires reset before more I/O",
            Self::Corrupt(_) => "app-data volume is corrupt",
            Self::InvalidPrincipal => "app-data principal must be nonzero",
            Self::InvalidPath(_) => "app-data path is not canonical",
            Self::NotFound => "app-data entry was not found",
            Self::AlreadyExists => "app-data entry already exists",
            Self::NotDirectory => "app-data parent is not a directory",
            Self::IsDirectory => "app-data entry is a directory",
            Self::NotEmpty => "app-data directory is not empty",
            Self::FileTooLarge => "app-data file exceeds the whole-file limit",
            Self::BufferTooSmall { .. } => "app-data output buffer is too small",
            Self::OutOfSpace => "app-data entry or live-payload quota is exhausted",
            Self::Conflict { .. } => "app-data compare-and-swap condition failed",
            Self::GenerationExhausted => "app-data generation is exhausted",
            Self::VerificationFailed => "app-data durable readback verification failed",
        }
    }
}

/// Maximum number of application principals described by one storage policy.
///
/// The fixed bound keeps policy validation allocation-free and suitable for
/// callers that cannot provide a heap.
pub const MAX_POLICY_PRINCIPALS: usize = 4;

/// A validated storage allowance for one stable application principal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrincipalQuota {
    principal: Principal,
    max_entries: u8,
    max_live_payload_bytes: u32,
}

impl PrincipalQuota {
    const EMPTY: Self = Self {
        principal: 0,
        max_entries: 0,
        max_live_payload_bytes: 0,
    };

    pub fn try_new(
        principal: Principal,
        max_entries: usize,
        max_live_payload_bytes: usize,
    ) -> Result<Self, PolicyError> {
        if principal == 0 {
            return Err(PolicyError::InvalidPrincipal);
        }
        if max_entries == 0 || max_entries > MAX_ENTRIES {
            return Err(PolicyError::InvalidEntryQuota {
                principal,
                limit: max_entries,
            });
        }
        if max_live_payload_bytes > MAX_LIVE_PAYLOAD_BYTES {
            return Err(PolicyError::InvalidLivePayloadQuota {
                principal,
                limit: max_live_payload_bytes,
            });
        }
        Ok(Self {
            principal,
            max_entries: u8::try_from(max_entries).unwrap(),
            max_live_payload_bytes: u32::try_from(max_live_payload_bytes).unwrap(),
        })
    }

    pub const fn principal(self) -> Principal {
        self.principal
    }

    pub const fn max_entries(self) -> usize {
        self.max_entries as usize
    }

    pub const fn max_live_payload_bytes(self) -> usize {
        self.max_live_payload_bytes as usize
    }

    fn validate(self) -> Result<(), PolicyError> {
        if self.principal == 0 {
            return Err(PolicyError::InvalidPrincipal);
        }
        if self.max_entries == 0 || usize::from(self.max_entries) > MAX_ENTRIES {
            return Err(PolicyError::InvalidEntryQuota {
                principal: self.principal,
                limit: usize::from(self.max_entries),
            });
        }
        if usize::try_from(self.max_live_payload_bytes).unwrap() > MAX_LIVE_PAYLOAD_BYTES {
            return Err(PolicyError::InvalidLivePayloadQuota {
                principal: self.principal,
                limit: usize::try_from(self.max_live_payload_bytes).unwrap(),
            });
        }
        Ok(())
    }
}

/// An allocation-free whitelist and per-principal quota table.
///
/// Construction and every policy-aware operation validate the complete table.
/// Aggregate allowances may not exceed the volume-wide limits, so one
/// principal cannot consume capacity promised to another principal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrincipalPolicy {
    quotas: [PrincipalQuota; MAX_POLICY_PRINCIPALS],
    len: u8,
}

impl PrincipalPolicy {
    pub fn try_new(quotas: &[PrincipalQuota]) -> Result<Self, PolicyError> {
        if quotas.is_empty() {
            return Err(PolicyError::EmptyPolicy);
        }
        if quotas.len() > MAX_POLICY_PRINCIPALS {
            return Err(PolicyError::TooManyPrincipals {
                maximum: MAX_POLICY_PRINCIPALS,
                actual: quotas.len(),
            });
        }
        let mut policy = Self {
            quotas: [PrincipalQuota::EMPTY; MAX_POLICY_PRINCIPALS],
            len: u8::try_from(quotas.len()).unwrap(),
        };
        policy.quotas[..quotas.len()].copy_from_slice(quotas);
        policy.validate()?;
        Ok(policy)
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn quotas(&self) -> &[PrincipalQuota] {
        &self.quotas[..usize::from(self.len)]
    }

    pub fn quota_for(&self, principal: Principal) -> Option<PrincipalQuota> {
        self.quotas()
            .iter()
            .copied()
            .find(|quota| quota.principal == principal)
    }

    pub fn validate(&self) -> Result<(), PolicyError> {
        let len = usize::from(self.len);
        if len == 0 {
            return Err(PolicyError::EmptyPolicy);
        }
        if len > MAX_POLICY_PRINCIPALS {
            return Err(PolicyError::TooManyPrincipals {
                maximum: MAX_POLICY_PRINCIPALS,
                actual: len,
            });
        }
        if self.quotas[len..]
            .iter()
            .any(|quota| *quota != PrincipalQuota::EMPTY)
        {
            return Err(PolicyError::InvalidUnusedSlot);
        }

        let mut total_entries = 0_usize;
        let mut total_live_payload_bytes = 0_usize;
        for (index, quota) in self.quotas[..len].iter().copied().enumerate() {
            quota.validate()?;
            if self.quotas[..index]
                .iter()
                .any(|candidate| candidate.principal == quota.principal)
            {
                return Err(PolicyError::DuplicatePrincipal {
                    principal: quota.principal,
                });
            }
            total_entries = total_entries.checked_add(quota.max_entries()).ok_or(
                PolicyError::AggregateEntryQuotaExceeded {
                    limit: MAX_ENTRIES,
                    actual: usize::MAX,
                },
            )?;
            total_live_payload_bytes = total_live_payload_bytes
                .checked_add(quota.max_live_payload_bytes())
                .ok_or(PolicyError::AggregateLivePayloadQuotaExceeded {
                    limit: MAX_LIVE_PAYLOAD_BYTES,
                    actual: usize::MAX,
                })?;
        }
        if total_entries > MAX_ENTRIES {
            return Err(PolicyError::AggregateEntryQuotaExceeded {
                limit: MAX_ENTRIES,
                actual: total_entries,
            });
        }
        if total_live_payload_bytes > MAX_LIVE_PAYLOAD_BYTES {
            return Err(PolicyError::AggregateLivePayloadQuotaExceeded {
                limit: MAX_LIVE_PAYLOAD_BYTES,
                actual: total_live_payload_bytes,
            });
        }
        Ok(())
    }

    fn require_quota(&self, principal: Principal) -> Result<PrincipalQuota, PolicyError> {
        self.quota_for(principal)
            .ok_or(PolicyError::UnknownPrincipal { principal })
    }
}

/// Failure returned by policy-aware recovery and mutation APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    /// The unchanged M54 storage layer rejected the operation.
    Volume(Error),
    EmptyPolicy,
    TooManyPrincipals {
        maximum: usize,
        actual: usize,
    },
    InvalidPrincipal,
    InvalidEntryQuota {
        principal: Principal,
        limit: usize,
    },
    InvalidLivePayloadQuota {
        principal: Principal,
        limit: usize,
    },
    DuplicatePrincipal {
        principal: Principal,
    },
    InvalidUnusedSlot,
    AggregateEntryQuotaExceeded {
        limit: usize,
        actual: usize,
    },
    AggregateLivePayloadQuotaExceeded {
        limit: usize,
        actual: usize,
    },
    UnknownPrincipal {
        principal: Principal,
    },
    PrincipalEntryQuotaExceeded {
        principal: Principal,
        limit: usize,
        actual: usize,
    },
    PrincipalLivePayloadQuotaExceeded {
        principal: Principal,
        limit: usize,
        actual: usize,
    },
}

impl From<Error> for PolicyError {
    fn from(error: Error) -> Self {
        Self::Volume(error)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File = 1,
    Directory = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    pub kind: EntryKind,
    pub length: usize,
    pub revision: u64,
    pub snapshot_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mutation {
    pub previous_generation: u64,
    pub committed_generation: u64,
    pub entry_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadResult {
    pub bytes: usize,
    pub revision: u64,
    pub snapshot_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryInfo {
    pub generation: u64,
    pub checkpoint_slot: u8,
    pub bank: u8,
    pub valid_snapshots: u8,
    pub rejected_snapshots: u8,
    pub entry_count: u8,
    pub live_payload_bytes: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirEntry {
    name: [u8; MAX_PATH_BYTES],
    name_len: u8,
    pub kind: EntryKind,
    pub length: u32,
    pub revision: u64,
}

impl DirEntry {
    pub const EMPTY: Self = Self {
        name: [0; MAX_PATH_BYTES],
        name_len: 0,
        kind: EntryKind::Directory,
        length: 0,
        revision: 0,
    };

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..usize::from(self.name_len)]).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ListResult {
    pub written: usize,
    pub total: usize,
    pub snapshot_generation: u64,
}

/// A fixed-geometry APPDATA partition view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppDataVolume {
    first_lba: u64,
}

impl AppDataVolume {
    pub const fn new(first_lba: u64) -> Self {
        Self { first_lba }
    }

    pub const fn first_lba(self) -> u64 {
        self.first_lba
    }

    /// Formats an exactly all-zero volume and installs generation zero.
    ///
    /// The first durable write is a sealed format intent. After that exact
    /// intent has reached the medium, a power loss before the immutable
    /// superblock is published can be retried by replaying only checkpoint zero
    /// and bank zero. Existing, malformed, torn-intent, or partially initialized
    /// media without that exact intent are never reformatted implicitly. See
    /// [`DurableVolumeIo`] for the sector-zero atomicity boundary.
    pub fn format_virgin<D: DurableVolumeIo>(self, io: &mut D) -> Result<RecoveryInfo, Error> {
        self.check_bounds(io)?;
        let mut sector = [0_u8; SECTOR_SIZE];
        read_at(io, self.absolute(0)?, &mut sector)?;
        let intent = FormatIntent::encode();
        if is_zero(&sector) {
            for relative in 1..VOLUME_SECTORS {
                read_at(io, self.absolute(relative)?, &mut sector)?;
                if !is_zero(&sector) {
                    return Err(Error::NotVirgin);
                }
            }
            write_at(io, self.absolute(0)?, &intent)?;
            flush_format(io)?;
        } else if sector == intent {
            self.validate_format_replay_scope(io)?;
        } else if Superblock::decode(&sector).is_ok() {
            return Err(Error::AlreadyFormatted);
        } else {
            return Err(Error::NotVirgin);
        }

        let superblock = Superblock::encode();
        let entries = [Entry::empty(0); MAX_ENTRIES];
        self.write_initial_bank(io, &entries)?;
        flush_before_checkpoint(io)?;
        let provisional = match self.validate_bank(io, 0, 0, None) {
            Ok(snapshot) => snapshot,
            Err(Error::Corrupt(_)) => return Err(Error::VerificationFailed),
            Err(error) => return Err(error),
        };
        let checkpoint = Checkpoint::encode(
            0,
            0,
            0,
            provisional.entry_count,
            provisional.live_payload_bytes,
            provisional.digest,
            provisional.manifest_seal,
            &superblock,
        );
        write_at(io, self.absolute(CHECKPOINT_RELATIVE_LBAS[0])?, &checkpoint)?;
        flush_after_checkpoint(io)?;

        write_at(io, self.absolute(0)?, &superblock)?;
        flush_after_checkpoint(io)?;
        read_at(io, self.absolute(0)?, &mut sector).map_err(|_| Error::OutcomeUnknown)?;
        if sector != superblock || Superblock::decode(&sector).is_err() {
            return Err(Error::OutcomeUnknown);
        }
        self.confirm_checkpoint(io, 0, 0, &superblock, &checkpoint, &entries)
    }

    pub fn recover<D: DurableVolumeIo>(self, io: &mut D) -> Result<RecoveryInfo, Error> {
        Ok(self.recover_snapshot(io)?.info())
    }

    /// Recovers the newest valid snapshot, then audits every live entry against
    /// `policy` before returning it to the caller.
    ///
    /// An otherwise valid snapshot containing an unknown principal or usage
    /// above a principal's quota is rejected rather than silently accepted or
    /// replaced by an older snapshot.
    pub fn recover_with_policy<D: DurableVolumeIo>(
        self,
        io: &mut D,
        policy: &PrincipalPolicy,
    ) -> Result<RecoveryInfo, PolicyError> {
        policy.validate()?;
        let snapshot = self.recover_snapshot(io)?;
        snapshot.audit_policy(policy)?;
        Ok(snapshot.info())
    }

    pub fn create_dir<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
    ) -> Result<Mutation, Error> {
        validate_principal(principal)?;
        let depth = validate_path(path, false).map_err(Error::InvalidPath)?;
        let current = self.recover_snapshot(io)?;
        if current.find(principal, path.as_bytes()).is_some() {
            return Err(Error::AlreadyExists);
        }
        current.require_parent_directory(principal, path.as_bytes())?;
        let slot = current.first_free().ok_or(Error::OutOfSpace)?;
        let generation = current.next_generation()?;
        let mut entries = current.entries;
        entries[slot] = Entry::directory(principal, path.as_bytes(), depth, generation);
        self.commit(io, current, entries, None, Some(generation))
    }

    /// Creates a directory only if the recovered volume and projected usage
    /// satisfy `policy`. Policy rejection happens before the first disk write.
    pub fn create_dir_with_policy<D: DurableVolumeIo>(
        self,
        io: &mut D,
        policy: &PrincipalPolicy,
        principal: Principal,
        path: &str,
    ) -> Result<Mutation, PolicyError> {
        policy.validate()?;
        validate_principal(principal)?;
        let quota = policy.require_quota(principal)?;
        let depth = validate_path(path, false).map_err(Error::InvalidPath)?;
        let current = self.recover_snapshot(io)?;
        current.audit_policy(policy)?;
        if current.find(principal, path.as_bytes()).is_some() {
            return Err(Error::AlreadyExists.into());
        }
        current.require_parent_directory(principal, path.as_bytes())?;

        let usage = current.principal_usage(principal);
        let projected_entries =
            usage
                .entries
                .checked_add(1)
                .ok_or(PolicyError::PrincipalEntryQuotaExceeded {
                    principal,
                    limit: quota.max_entries(),
                    actual: usize::MAX,
                })?;
        if projected_entries > quota.max_entries() {
            return Err(PolicyError::PrincipalEntryQuotaExceeded {
                principal,
                limit: quota.max_entries(),
                actual: projected_entries,
            });
        }

        let slot = current.first_free().ok_or(Error::OutOfSpace)?;
        let generation = current.next_generation()?;
        let mut entries = current.entries;
        entries[slot] = Entry::directory(principal, path.as_bytes(), depth, generation);
        self.commit(io, current, entries, None, Some(generation))
            .map_err(PolicyError::from)
    }

    pub fn replace<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
        bytes: &[u8],
        condition: ReplaceCondition,
    ) -> Result<Mutation, Error> {
        validate_principal(principal)?;
        let depth = validate_path(path, false).map_err(Error::InvalidPath)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(Error::FileTooLarge);
        }
        let current = self.recover_snapshot(io)?;
        current.require_parent_directory(principal, path.as_bytes())?;
        let existing = current.find(principal, path.as_bytes());
        if let Some(slot) = existing
            && current.entries[slot].kind == StoredKind::Directory
        {
            return Err(Error::IsDirectory);
        }
        let actual_generation = existing.map(|slot| current.entries[slot].revision);
        let condition_matches = match condition {
            ReplaceCondition::Any => true,
            ReplaceCondition::Absent => existing.is_none(),
            ReplaceCondition::Generation(expected) => actual_generation == Some(expected),
        };
        if !condition_matches {
            return Err(Error::Conflict {
                expected: condition,
                actual_generation,
            });
        }

        let slot = match existing {
            Some(slot) => slot,
            None => current.first_free().ok_or(Error::OutOfSpace)?,
        };
        let old_length = existing
            .map(|index| usize::try_from(current.entries[index].length).unwrap())
            .unwrap_or(0);
        let new_total = usize::try_from(current.live_payload_bytes)
            .unwrap()
            .checked_sub(old_length)
            .and_then(|total| total.checked_add(bytes.len()))
            .ok_or(Error::OutOfSpace)?;
        if new_total > MAX_LIVE_PAYLOAD_BYTES {
            return Err(Error::OutOfSpace);
        }

        let generation = current.next_generation()?;
        let mut entries = current.entries;
        entries[slot] = Entry::file(principal, path.as_bytes(), depth, generation, bytes);
        self.commit(
            io,
            current,
            entries,
            Some(Replacement { slot, bytes }),
            Some(generation),
        )
    }

    /// Creates or atomically replaces a file only if the recovered volume and
    /// projected per-principal entry and live-byte usage satisfy `policy`.
    /// Policy rejection happens before the first disk write.
    pub fn replace_with_policy<D: DurableVolumeIo>(
        self,
        io: &mut D,
        policy: &PrincipalPolicy,
        principal: Principal,
        path: &str,
        bytes: &[u8],
        condition: ReplaceCondition,
    ) -> Result<Mutation, PolicyError> {
        policy.validate()?;
        validate_principal(principal)?;
        let quota = policy.require_quota(principal)?;
        let depth = validate_path(path, false).map_err(Error::InvalidPath)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(Error::FileTooLarge.into());
        }

        let current = self.recover_snapshot(io)?;
        current.audit_policy(policy)?;
        current.require_parent_directory(principal, path.as_bytes())?;
        let existing = current.find(principal, path.as_bytes());
        if let Some(slot) = existing
            && current.entries[slot].kind == StoredKind::Directory
        {
            return Err(Error::IsDirectory.into());
        }
        let actual_generation = existing.map(|slot| current.entries[slot].revision);
        let condition_matches = match condition {
            ReplaceCondition::Any => true,
            ReplaceCondition::Absent => existing.is_none(),
            ReplaceCondition::Generation(expected) => actual_generation == Some(expected),
        };
        if !condition_matches {
            return Err(Error::Conflict {
                expected: condition,
                actual_generation,
            }
            .into());
        }

        let usage = current.principal_usage(principal);
        let projected_entries = if existing.is_some() {
            usage.entries
        } else {
            usage
                .entries
                .checked_add(1)
                .ok_or(PolicyError::PrincipalEntryQuotaExceeded {
                    principal,
                    limit: quota.max_entries(),
                    actual: usize::MAX,
                })?
        };
        if projected_entries > quota.max_entries() {
            return Err(PolicyError::PrincipalEntryQuotaExceeded {
                principal,
                limit: quota.max_entries(),
                actual: projected_entries,
            });
        }

        let old_length = existing
            .map(|index| usize::try_from(current.entries[index].length).unwrap())
            .unwrap_or(0);
        let projected_principal_bytes = usage
            .live_payload_bytes
            .checked_sub(old_length)
            .and_then(|total| total.checked_add(bytes.len()))
            .ok_or(PolicyError::PrincipalLivePayloadQuotaExceeded {
                principal,
                limit: quota.max_live_payload_bytes(),
                actual: usize::MAX,
            })?;
        if projected_principal_bytes > quota.max_live_payload_bytes() {
            return Err(PolicyError::PrincipalLivePayloadQuotaExceeded {
                principal,
                limit: quota.max_live_payload_bytes(),
                actual: projected_principal_bytes,
            });
        }

        let slot = match existing {
            Some(slot) => slot,
            None => current.first_free().ok_or(Error::OutOfSpace)?,
        };
        let new_total = usize::try_from(current.live_payload_bytes)
            .unwrap()
            .checked_sub(old_length)
            .and_then(|total| total.checked_add(bytes.len()))
            .ok_or(Error::OutOfSpace)?;
        if new_total > MAX_LIVE_PAYLOAD_BYTES {
            return Err(Error::OutOfSpace.into());
        }

        let generation = current.next_generation()?;
        let mut entries = current.entries;
        entries[slot] = Entry::file(principal, path.as_bytes(), depth, generation, bytes);
        self.commit(
            io,
            current,
            entries,
            Some(Replacement { slot, bytes }),
            Some(generation),
        )
        .map_err(PolicyError::from)
    }

    pub fn replace_cas<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
        bytes: &[u8],
        expected_generation: u64,
    ) -> Result<Mutation, Error> {
        self.replace(
            io,
            principal,
            path,
            bytes,
            ReplaceCondition::Generation(expected_generation),
        )
    }

    pub fn unlink<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
    ) -> Result<Mutation, Error> {
        validate_principal(principal)?;
        validate_path(path, false).map_err(Error::InvalidPath)?;
        let current = self.recover_snapshot(io)?;
        let slot = current
            .find(principal, path.as_bytes())
            .ok_or(Error::NotFound)?;
        if current.entries[slot].kind == StoredKind::Directory
            && current.has_descendant(principal, path.as_bytes())
        {
            return Err(Error::NotEmpty);
        }
        let generation = current.next_generation()?;
        let mut entries = current.entries;
        entries[slot] = Entry::empty(generation);
        self.commit(io, current, entries, None, None)
    }

    pub fn read<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
        output: &mut [u8],
    ) -> Result<ReadResult, Error> {
        validate_principal(principal)?;
        validate_path(path, false).map_err(Error::InvalidPath)?;
        let snapshot = self.recover_snapshot(io)?;
        let slot = snapshot
            .find(principal, path.as_bytes())
            .ok_or(Error::NotFound)?;
        let entry = snapshot.entries[slot];
        if entry.kind == StoredKind::Directory {
            return Err(Error::IsDirectory);
        }
        let length = usize::try_from(entry.length).unwrap();
        if output.len() < length {
            return Err(Error::BufferTooSmall { needed: length });
        }
        let mut sector = [0_u8; SECTOR_SIZE];
        let mut copied = 0;
        for data_sector in 0..DATA_SECTORS_PER_ENTRY {
            let relative = bank_data_lba(snapshot.bank, slot, data_sector)?;
            read_at(io, self.absolute(relative)?, &mut sector)?;
            let take = core::cmp::min(SECTOR_SIZE, length - copied);
            output[copied..copied + take].copy_from_slice(&sector[..take]);
            copied += take;
            if copied == length {
                break;
            }
        }
        Ok(ReadResult {
            bytes: length,
            revision: entry.revision,
            snapshot_generation: snapshot.generation,
        })
    }

    pub fn stat<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        path: &str,
    ) -> Result<Metadata, Error> {
        validate_principal(principal)?;
        validate_path(path, true).map_err(Error::InvalidPath)?;
        let snapshot = self.recover_snapshot(io)?;
        if path.is_empty() {
            return Ok(Metadata {
                kind: EntryKind::Directory,
                length: 0,
                revision: snapshot.generation,
                snapshot_generation: snapshot.generation,
            });
        }
        let slot = snapshot
            .find(principal, path.as_bytes())
            .ok_or(Error::NotFound)?;
        Ok(snapshot.entries[slot].metadata(snapshot.generation))
    }

    pub fn list<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        directory: &str,
        output: &mut [DirEntry],
    ) -> Result<ListResult, Error> {
        validate_principal(principal)?;
        validate_path(directory, true).map_err(Error::InvalidPath)?;
        let snapshot = self.recover_snapshot(io)?;
        if !directory.is_empty() {
            let slot = snapshot
                .find(principal, directory.as_bytes())
                .ok_or(Error::NotFound)?;
            if snapshot.entries[slot].kind != StoredKind::Directory {
                return Err(Error::NotDirectory);
            }
        }

        let mut written = 0;
        let mut total = 0;
        for entry in &snapshot.entries {
            if entry.kind != StoredKind::Free
                && entry.principal == principal
                && is_direct_child(entry.path_bytes(), directory.as_bytes())
            {
                if written < output.len() {
                    output[written] = entry.dir_entry(directory.as_bytes());
                    written += 1;
                }
                total += 1;
            }
        }
        Ok(ListResult {
            written,
            total,
            snapshot_generation: snapshot.generation,
        })
    }

    /// Enumerates every entry owned by `principal` in stable on-disk slot order.
    ///
    /// Unlike [`Self::list`], each [`DirEntry::name`] is the complete canonical
    /// root-relative path. This lets a capability adapter expose a cursor-based
    /// flat namespace without recursively reopening directories.
    pub fn list_all<D: DurableVolumeIo>(
        self,
        io: &mut D,
        principal: Principal,
        output: &mut [DirEntry],
    ) -> Result<ListResult, Error> {
        validate_principal(principal)?;
        let snapshot = self.recover_snapshot(io)?;
        let mut written = 0;
        let mut total = 0;
        for entry in &snapshot.entries {
            if entry.kind != StoredKind::Free && entry.principal == principal {
                if written < output.len() {
                    output[written] = entry.dir_entry(&[]);
                    written += 1;
                }
                total += 1;
            }
        }
        Ok(ListResult {
            written,
            total,
            snapshot_generation: snapshot.generation,
        })
    }

    fn commit<D: DurableVolumeIo>(
        self,
        io: &mut D,
        current: Snapshot,
        mut entries: [Entry; MAX_ENTRIES],
        replacement: Option<Replacement<'_>>,
        entry_revision: Option<u64>,
    ) -> Result<Mutation, Error> {
        let generation = current.next_generation()?;
        let inactive = 1 - current.checkpoint_slot;
        for entry in &mut entries {
            entry.snapshot_generation = generation;
        }

        self.write_bank(
            io,
            Some(&current),
            inactive,
            generation,
            &entries,
            replacement,
        )?;
        flush_before_checkpoint(io)?;
        let provisional = match self.validate_bank(io, inactive, generation, None) {
            Ok(snapshot) => snapshot,
            Err(Error::Corrupt(_)) => return Err(Error::VerificationFailed),
            Err(error) => return Err(error),
        };
        if provisional.entries != entries {
            return Err(Error::VerificationFailed);
        }

        let mut superblock = [0_u8; SECTOR_SIZE];
        read_at(io, self.absolute(0)?, &mut superblock)?;
        Superblock::decode(&superblock).map_err(Error::Corrupt)?;
        let checkpoint = Checkpoint::encode(
            inactive,
            inactive,
            generation,
            provisional.entry_count,
            provisional.live_payload_bytes,
            provisional.digest,
            provisional.manifest_seal,
            &superblock,
        );
        write_at(
            io,
            self.absolute(CHECKPOINT_RELATIVE_LBAS[usize::from(inactive)])?,
            &checkpoint,
        )?;
        flush_after_checkpoint(io)?;

        self.confirm_checkpoint(io, inactive, generation, &superblock, &checkpoint, &entries)?;
        Ok(Mutation {
            previous_generation: current.generation,
            committed_generation: generation,
            entry_revision,
        })
    }

    /// Confirms the newly published checkpoint and its complete snapshot
    /// without recursively running two-candidate recovery while the mutation
    /// caller still owns its old/new snapshot values. Besides bounding kernel
    /// stack use, this keeps the post-publication rule explicit: once the
    /// checkpoint flush succeeds, any failure to prove the exact result is an
    /// `OutcomeUnknown`, never a misleading pre-commit device error.
    fn confirm_checkpoint<D: DurableVolumeIo>(
        self,
        io: &mut D,
        checkpoint_slot: u8,
        generation: u64,
        superblock: &Sector,
        expected_checkpoint: &Sector,
        expected_entries: &[Entry; MAX_ENTRIES],
    ) -> Result<RecoveryInfo, Error> {
        let checkpoint_lba = CHECKPOINT_RELATIVE_LBAS
            .get(usize::from(checkpoint_slot))
            .copied()
            .ok_or(Error::OutcomeUnknown)?;
        let checkpoint_lba = self
            .absolute(checkpoint_lba)
            .map_err(|_| Error::OutcomeUnknown)?;
        let mut readback = [0_u8; SECTOR_SIZE];
        read_at(io, checkpoint_lba, &mut readback).map_err(|_| Error::OutcomeUnknown)?;
        if readback != *expected_checkpoint {
            return Err(Error::OutcomeUnknown);
        }
        let checkpoint = Checkpoint::decode(&readback, checkpoint_slot, superblock)
            .map_err(|_| Error::OutcomeUnknown)?;
        if checkpoint.bank != checkpoint_slot || checkpoint.generation != generation {
            return Err(Error::OutcomeUnknown);
        }
        let verified = self
            .validate_bank(io, checkpoint_slot, generation, Some(checkpoint))
            .map_err(|_| Error::OutcomeUnknown)?;
        if verified.checkpoint_slot != checkpoint_slot
            || verified.bank != checkpoint_slot
            || verified.entries != *expected_entries
        {
            return Err(Error::OutcomeUnknown);
        }
        Ok(RecoveryInfo {
            generation: verified.generation,
            checkpoint_slot,
            bank: verified.bank,
            valid_snapshots: 1,
            rejected_snapshots: 1,
            entry_count: verified.entry_count,
            live_payload_bytes: verified.live_payload_bytes,
        })
    }

    fn write_bank<D: DurableVolumeIo>(
        self,
        io: &mut D,
        source: Option<&Snapshot>,
        bank: u8,
        generation: u64,
        entries: &[Entry; MAX_ENTRIES],
        replacement: Option<Replacement<'_>>,
    ) -> Result<(), Error> {
        let mut sector = [0_u8; SECTOR_SIZE];
        for (slot, entry) in entries.iter().enumerate() {
            for data_sector in 0..DATA_SECTORS_PER_ENTRY {
                sector.fill(0);
                if replacement.is_some_and(|item| item.slot == slot) {
                    let bytes = replacement.unwrap().bytes;
                    let start = usize::try_from(data_sector).unwrap() * SECTOR_SIZE;
                    if start < bytes.len() {
                        let take = core::cmp::min(SECTOR_SIZE, bytes.len() - start);
                        sector[..take].copy_from_slice(&bytes[start..start + take]);
                    }
                } else if entry.kind == StoredKind::File {
                    let active = source.ok_or(Error::VerificationFailed)?;
                    let old = active.entries[slot];
                    if old.kind != StoredKind::File
                        || old.principal != entry.principal
                        || old.path_bytes() != entry.path_bytes()
                        || old.length != entry.length
                        || old.data_crc != entry.data_crc
                        || old.data_fnv != entry.data_fnv
                    {
                        return Err(Error::VerificationFailed);
                    }
                    let relative = bank_data_lba(active.bank, slot, data_sector)?;
                    read_at(io, self.absolute(relative)?, &mut sector)?;
                }
                let relative = bank_data_lba(bank, slot, data_sector)?;
                write_at(io, self.absolute(relative)?, &sector)?;
            }
        }

        let mut entries_checksum = Checksum::new();
        for (slot, entry) in entries.iter().enumerate() {
            sector = entry.encode(generation);
            entries_checksum.update(&sector);
            let relative = BANK_RELATIVE_LBAS[usize::from(bank)]
                .checked_add(ENTRY_RELATIVE_LBA)
                .and_then(|lba| lba.checked_add(slot as u64))
                .ok_or(Error::VolumeBounds)?;
            write_at(io, self.absolute(relative)?, &sector)?;
        }

        let counts = SnapshotCounts::from_entries(entries)?;
        sector = Manifest::encode(bank, generation, counts, entries_checksum.finish());
        let relative = BANK_RELATIVE_LBAS[usize::from(bank)] + MANIFEST_RELATIVE_LBA;
        write_at(io, self.absolute(relative)?, &sector)
    }

    /// Replays a deterministic, empty generation-zero bank and scrubs the
    /// complete reserved bank extent. This makes every sector that may be
    /// nonzero beneath a valid format intent deterministic before publication.
    fn write_initial_bank<D: DurableVolumeIo>(
        self,
        io: &mut D,
        entries: &[Entry; MAX_ENTRIES],
    ) -> Result<(), Error> {
        let mut entries_checksum = Checksum::new();
        for entry in entries {
            entries_checksum.update(&entry.encode(0));
        }
        let counts = SnapshotCounts::from_entries(entries)?;
        let manifest = Manifest::encode(0, 0, counts, entries_checksum.finish());
        let zero = [0_u8; SECTOR_SIZE];
        for offset in 0..BANK_SECTORS {
            let sector = if offset == MANIFEST_RELATIVE_LBA {
                &manifest
            } else if (ENTRY_RELATIVE_LBA..DATA_RELATIVE_LBA).contains(&offset) {
                let slot = usize::try_from(offset - ENTRY_RELATIVE_LBA).unwrap();
                // Encoding is deterministic and allocation-free; keep the
                // temporary alive for the write rather than retaining a bank.
                let encoded = entries[slot].encode(0);
                write_at(io, self.absolute(BANK_RELATIVE_LBAS[0] + offset)?, &encoded)?;
                continue;
            } else {
                &zero
            };
            write_at(io, self.absolute(BANK_RELATIVE_LBAS[0] + offset)?, sector)?;
        }
        Ok(())
    }

    /// An exact intent can only have been published after a whole-volume zero
    /// scan. During formatting, only checkpoint zero and bank zero may then
    /// change. Anything elsewhere is evidence that this is not our interrupted
    /// transaction, so recovery remains fail-closed.
    fn validate_format_replay_scope<D: DurableVolumeIo>(self, io: &mut D) -> Result<(), Error> {
        let bank_start = BANK_RELATIVE_LBAS[0];
        let bank_end = bank_start + BANK_SECTORS;
        let mut sector = [0_u8; SECTOR_SIZE];
        for relative in 1..VOLUME_SECTORS {
            if relative == CHECKPOINT_RELATIVE_LBAS[0] || (bank_start..bank_end).contains(&relative)
            {
                continue;
            }
            read_at(io, self.absolute(relative)?, &mut sector)?;
            if !is_zero(&sector) {
                return Err(Error::NotVirgin);
            }
        }
        Ok(())
    }

    fn validate_bank<D: DurableVolumeIo>(
        self,
        io: &mut D,
        bank: u8,
        generation: u64,
        checkpoint: Option<Checkpoint>,
    ) -> Result<Snapshot, Error> {
        let bank_index = usize::from(bank);
        if bank_index >= BANK_RELATIVE_LBAS.len() {
            return Err(Error::Corrupt(Corruption::BadCheckpoint));
        }
        let mut digest = Checksum::new();
        let mut sector = [0_u8; SECTOR_SIZE];
        read_at(
            io,
            self.absolute(BANK_RELATIVE_LBAS[bank_index] + MANIFEST_RELATIVE_LBA)?,
            &mut sector,
        )?;
        let manifest_sector = sector;
        digest.update(&manifest_sector);
        let manifest =
            Manifest::decode(&manifest_sector, bank, generation).map_err(Error::Corrupt)?;

        let mut entries = [Entry::empty(generation); MAX_ENTRIES];
        let mut entries_checksum = Checksum::new();
        for (slot, output) in entries.iter_mut().enumerate() {
            let relative = BANK_RELATIVE_LBAS[bank_index]
                .checked_add(ENTRY_RELATIVE_LBA)
                .and_then(|lba| lba.checked_add(slot as u64))
                .ok_or(Error::VolumeBounds)?;
            read_at(io, self.absolute(relative)?, &mut sector)?;
            digest.update(&sector);
            entries_checksum.update(&sector);
            *output = Entry::decode(&sector, generation).map_err(Error::Corrupt)?;
        }
        if entries_checksum.finish() != manifest.entries_digest {
            return Err(Error::Corrupt(Corruption::BadManifest));
        }

        let counts = validate_entries(&entries, generation)?;
        if counts != manifest.counts {
            return Err(Error::Corrupt(Corruption::CountMismatch));
        }

        let mut file_checksums: [Checksum; MAX_ENTRIES] = array::from_fn(|_| Checksum::new());
        let mut file_remaining: [usize; MAX_ENTRIES] =
            array::from_fn(|slot| usize::try_from(entries[slot].length).unwrap());
        for slot in 0..MAX_ENTRIES {
            for data_sector in 0..DATA_SECTORS_PER_ENTRY {
                let relative = bank_data_lba(bank, slot, data_sector)?;
                read_at(io, self.absolute(relative)?, &mut sector)?;
                digest.update(&sector);
                if entries[slot].kind == StoredKind::File {
                    let take = core::cmp::min(SECTOR_SIZE, file_remaining[slot]);
                    file_checksums[slot].update(&sector[..take]);
                    file_remaining[slot] -= take;
                    if sector[take..].iter().any(|byte| *byte != 0) {
                        return Err(Error::Corrupt(Corruption::InvalidEntryData));
                    }
                } else if !is_zero(&sector) {
                    return Err(Error::Corrupt(Corruption::InvalidEntryData));
                }
            }
        }
        for (slot, entry) in entries.iter().enumerate() {
            if entry.kind == StoredKind::File
                && (file_remaining[slot] != 0
                    || file_checksums[slot].finish().crc != entry.data_crc
                    || file_checksums[slot].finish().fnv != entry.data_fnv)
            {
                return Err(Error::Corrupt(Corruption::InvalidEntryData));
            }
        }

        let digest = digest.finish();
        if let Some(checkpoint) = checkpoint
            && (checkpoint.snapshot_digest != digest
                || checkpoint.entry_count != counts.entry_count
                || checkpoint.live_payload_bytes != counts.live_payload_bytes
                || checkpoint.manifest_seal != seal_of(&manifest_sector))
        {
            return Err(Error::Corrupt(Corruption::SnapshotDigestMismatch));
        }

        Ok(Snapshot {
            generation,
            checkpoint_slot: bank,
            bank,
            valid_snapshots: 1,
            rejected_snapshots: 1,
            entry_count: counts.entry_count,
            live_payload_bytes: counts.live_payload_bytes,
            digest,
            manifest_seal: seal_of(&manifest_sector),
            entries,
        })
    }

    fn recover_snapshot<D: DurableVolumeIo>(self, io: &mut D) -> Result<Snapshot, Error> {
        self.check_bounds(io)?;
        let mut superblock_sector = [0_u8; SECTOR_SIZE];
        read_at(io, self.absolute(0)?, &mut superblock_sector)?;
        if is_zero(&superblock_sector) {
            return Err(Error::Unformatted);
        }
        if superblock_sector == FormatIntent::encode() {
            return Err(Error::Unformatted);
        }
        Superblock::decode(&superblock_sector).map_err(Error::Corrupt)?;

        let mut candidates: [Option<Snapshot>; 2] = [None, None];
        let mut checkpoint_sector = [0_u8; SECTOR_SIZE];
        for slot in 0..2_u8 {
            read_at(
                io,
                self.absolute(CHECKPOINT_RELATIVE_LBAS[usize::from(slot)])?,
                &mut checkpoint_sector,
            )?;
            let checkpoint = match Checkpoint::decode(&checkpoint_sector, slot, &superblock_sector)
            {
                Ok(checkpoint) => checkpoint,
                Err(_) => continue,
            };
            match self.validate_bank(io, checkpoint.bank, checkpoint.generation, Some(checkpoint)) {
                Ok(mut snapshot) => {
                    snapshot.checkpoint_slot = slot;
                    candidates[usize::from(slot)] = Some(snapshot);
                }
                Err(Error::Corrupt(_)) => {}
                Err(error) => return Err(error),
            }
        }

        let valid = candidates
            .iter()
            .filter(|candidate| candidate.is_some())
            .count() as u8;
        let mut selected = match (candidates[0], candidates[1]) {
            (None, None) => return Err(Error::Corrupt(Corruption::NoValidSnapshot)),
            (Some(snapshot), None) | (None, Some(snapshot)) => snapshot,
            (Some(first), Some(second)) => {
                if first.generation == second.generation {
                    return Err(Error::Corrupt(Corruption::AmbiguousGeneration));
                }
                if first.generation.abs_diff(second.generation) != 1 {
                    return Err(Error::Corrupt(Corruption::GenerationGap));
                }
                if first.generation > second.generation {
                    first
                } else {
                    second
                }
            }
        };
        selected.valid_snapshots = valid;
        selected.rejected_snapshots = 2 - valid;
        Ok(selected)
    }

    fn check_bounds<D: DurableVolumeIo>(self, io: &D) -> Result<(), Error> {
        let end = self
            .first_lba
            .checked_add(VOLUME_SECTORS)
            .ok_or(Error::VolumeBounds)?;
        if end > io.sector_count() {
            return Err(Error::VolumeBounds);
        }
        Ok(())
    }

    fn absolute(self, relative: u64) -> Result<u64, Error> {
        if relative >= VOLUME_SECTORS {
            return Err(Error::VolumeBounds);
        }
        self.first_lba
            .checked_add(relative)
            .ok_or(Error::VolumeBounds)
    }
}

#[derive(Clone, Copy)]
struct Replacement<'a> {
    slot: usize,
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoredKind {
    Free,
    File,
    Directory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entry {
    kind: StoredKind,
    principal: Principal,
    snapshot_generation: u64,
    revision: u64,
    length: u32,
    data_crc: u32,
    data_fnv: u64,
    path_len: u8,
    depth: u8,
    path: [u8; MAX_PATH_BYTES],
}

impl Entry {
    const fn empty(generation: u64) -> Self {
        Self {
            kind: StoredKind::Free,
            principal: 0,
            snapshot_generation: generation,
            revision: 0,
            length: 0,
            data_crc: 0,
            data_fnv: 0xcbf2_9ce4_8422_2325,
            path_len: 0,
            depth: 0,
            path: [0; MAX_PATH_BYTES],
        }
    }

    fn directory(principal: Principal, path: &[u8], depth: u8, generation: u64) -> Self {
        let mut entry = Self::empty(generation);
        entry.kind = StoredKind::Directory;
        entry.principal = principal;
        entry.revision = generation;
        entry.path_len = u8::try_from(path.len()).unwrap();
        entry.depth = depth;
        entry.path[..path.len()].copy_from_slice(path);
        entry
    }

    fn file(principal: Principal, path: &[u8], depth: u8, generation: u64, bytes: &[u8]) -> Self {
        let mut entry = Self::empty(generation);
        entry.kind = StoredKind::File;
        entry.principal = principal;
        entry.revision = generation;
        entry.length = u32::try_from(bytes.len()).unwrap();
        entry.data_crc = crc32(bytes);
        entry.data_fnv = fnv1a64(bytes);
        entry.path_len = u8::try_from(path.len()).unwrap();
        entry.depth = depth;
        entry.path[..path.len()].copy_from_slice(path);
        entry
    }

    fn path_bytes(&self) -> &[u8] {
        &self.path[..usize::from(self.path_len)]
    }

    fn metadata(self, snapshot_generation: u64) -> Metadata {
        Metadata {
            kind: match self.kind {
                StoredKind::File => EntryKind::File,
                StoredKind::Directory => EntryKind::Directory,
                StoredKind::Free => unreachable!(),
            },
            length: usize::try_from(self.length).unwrap(),
            revision: self.revision,
            snapshot_generation,
        }
    }

    fn dir_entry(self, parent: &[u8]) -> DirEntry {
        let name = if parent.is_empty() {
            self.path_bytes()
        } else {
            &self.path_bytes()[parent.len() + 1..]
        };
        let mut result = DirEntry::EMPTY;
        result.name[..name.len()].copy_from_slice(name);
        result.name_len = u8::try_from(name.len()).unwrap();
        result.kind = match self.kind {
            StoredKind::File => EntryKind::File,
            StoredKind::Directory => EntryKind::Directory,
            StoredKind::Free => unreachable!(),
        };
        result.length = self.length;
        result.revision = self.revision;
        result
    }

    fn encode(self, generation: u64) -> Sector {
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&ENTRY_MAGIC);
        put_u32(&mut sector, 8, FORMAT_VERSION);
        sector[12] = match self.kind {
            StoredKind::Free => 0,
            StoredKind::File => 1,
            StoredKind::Directory => 2,
        };
        sector[13] = self.path_len;
        sector[14] = self.depth;
        put_u64(&mut sector, 16, self.principal);
        put_u64(&mut sector, 24, generation);
        put_u64(&mut sector, 32, self.revision);
        put_u32(&mut sector, 40, self.length);
        put_u32(&mut sector, 48, self.data_crc);
        put_u64(&mut sector, 56, self.data_fnv);
        sector[64..64 + MAX_PATH_BYTES].copy_from_slice(&self.path);
        seal(&mut sector);
        sector
    }

    fn decode(sector: &Sector, generation: u64) -> Result<Self, Corruption> {
        if !seal_is_valid(sector)
            || sector[..8] != ENTRY_MAGIC
            || le_u32(sector, 8) != FORMAT_VERSION
            || sector[15] != 0
            || le_u32(sector, 44) != 0
            || le_u32(sector, 52) != 0
            || sector[128..SEAL_FNV_OFFSET].iter().any(|byte| *byte != 0)
        {
            return Err(Corruption::BadEntry);
        }
        let kind = match sector[12] {
            0 => StoredKind::Free,
            1 => StoredKind::File,
            2 => StoredKind::Directory,
            _ => return Err(Corruption::BadEntry),
        };
        let path_len = usize::from(sector[13]);
        if path_len > MAX_PATH_BYTES
            || le_u64(sector, 24) != generation
            || sector[64 + path_len..64 + MAX_PATH_BYTES]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(Corruption::BadEntry);
        }
        let principal = le_u64(sector, 16);
        let revision = le_u64(sector, 32);
        let length = le_u32(sector, 40);
        let data_crc = le_u32(sector, 48);
        let data_fnv = le_u64(sector, 56);
        let mut path = [0_u8; MAX_PATH_BYTES];
        path.copy_from_slice(&sector[64..64 + MAX_PATH_BYTES]);

        if kind == StoredKind::Free {
            if principal != 0
                || revision != 0
                || length != 0
                || data_crc != crc32(&[])
                || data_fnv != fnv1a64(&[])
                || path_len != 0
                || sector[14] != 0
            {
                return Err(Corruption::BadEntry);
            }
        } else {
            if principal == 0 || revision == 0 || revision > generation {
                return Err(Corruption::BadEntry);
            }
            let path_str =
                core::str::from_utf8(&path[..path_len]).map_err(|_| Corruption::BadEntry)?;
            let depth = validate_path(path_str, false).map_err(|_| Corruption::BadEntry)?;
            if depth != sector[14] {
                return Err(Corruption::BadEntry);
            }
            match kind {
                StoredKind::File if usize::try_from(length).unwrap() <= MAX_FILE_BYTES => {}
                StoredKind::Directory
                    if length == 0 && data_crc == crc32(&[]) && data_fnv == fnv1a64(&[]) => {}
                _ => return Err(Corruption::BadEntry),
            }
        }

        Ok(Self {
            kind,
            principal,
            snapshot_generation: generation,
            revision,
            length,
            data_crc,
            data_fnv,
            path_len: u8::try_from(path_len).unwrap(),
            depth: sector[14],
            path,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Snapshot {
    generation: u64,
    checkpoint_slot: u8,
    bank: u8,
    valid_snapshots: u8,
    rejected_snapshots: u8,
    entry_count: u8,
    live_payload_bytes: u32,
    digest: Digest,
    manifest_seal: Digest,
    entries: [Entry; MAX_ENTRIES],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrincipalUsage {
    entries: usize,
    live_payload_bytes: usize,
}

impl Snapshot {
    fn info(self) -> RecoveryInfo {
        RecoveryInfo {
            generation: self.generation,
            checkpoint_slot: self.checkpoint_slot,
            bank: self.bank,
            valid_snapshots: self.valid_snapshots,
            rejected_snapshots: self.rejected_snapshots,
            entry_count: self.entry_count,
            live_payload_bytes: self.live_payload_bytes,
        }
    }

    fn next_generation(self) -> Result<u64, Error> {
        self.generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)
    }

    fn find(&self, principal: Principal, path: &[u8]) -> Option<usize> {
        self.entries.iter().position(|entry| {
            entry.kind != StoredKind::Free
                && entry.principal == principal
                && entry.path_bytes() == path
        })
    }

    fn first_free(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.kind == StoredKind::Free)
    }

    fn principal_usage(&self, principal: Principal) -> PrincipalUsage {
        let mut usage = PrincipalUsage {
            entries: 0,
            live_payload_bytes: 0,
        };
        for entry in &self.entries {
            if entry.kind != StoredKind::Free && entry.principal == principal {
                usage.entries += 1;
                if entry.kind == StoredKind::File {
                    usage.live_payload_bytes += usize::try_from(entry.length).unwrap();
                }
            }
        }
        usage
    }

    fn audit_policy(&self, policy: &PrincipalPolicy) -> Result<(), PolicyError> {
        for entry in &self.entries {
            if entry.kind != StoredKind::Free && policy.quota_for(entry.principal).is_none() {
                return Err(PolicyError::UnknownPrincipal {
                    principal: entry.principal,
                });
            }
        }

        for quota in policy.quotas().iter().copied() {
            let usage = self.principal_usage(quota.principal());
            if usage.entries > quota.max_entries() {
                return Err(PolicyError::PrincipalEntryQuotaExceeded {
                    principal: quota.principal(),
                    limit: quota.max_entries(),
                    actual: usage.entries,
                });
            }
            if usage.live_payload_bytes > quota.max_live_payload_bytes() {
                return Err(PolicyError::PrincipalLivePayloadQuotaExceeded {
                    principal: quota.principal(),
                    limit: quota.max_live_payload_bytes(),
                    actual: usage.live_payload_bytes,
                });
            }
        }
        Ok(())
    }

    fn require_parent_directory(&self, principal: Principal, path: &[u8]) -> Result<(), Error> {
        let parent = parent_path(path);
        if parent.is_empty() {
            return Ok(());
        }
        let slot = self.find(principal, parent).ok_or(Error::NotFound)?;
        if self.entries[slot].kind != StoredKind::Directory {
            return Err(Error::NotDirectory);
        }
        Ok(())
    }

    fn has_descendant(&self, principal: Principal, path: &[u8]) -> bool {
        self.entries.iter().any(|entry| {
            entry.kind != StoredKind::Free
                && entry.principal == principal
                && is_descendant(entry.path_bytes(), path)
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Digest {
    crc: u32,
    fnv: u64,
}

#[derive(Clone, Copy)]
struct Checksum {
    crc: u32,
    fnv: u64,
}

impl Checksum {
    const fn new() -> Self {
        Self {
            crc: !0,
            fnv: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.crc = crc32_update(self.crc, *byte);
            self.fnv = (self.fnv ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    const fn finish(self) -> Digest {
        Digest {
            crc: !self.crc,
            fnv: self.fnv,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SnapshotCounts {
    entry_count: u8,
    file_count: u8,
    directory_count: u8,
    live_payload_bytes: u32,
}

impl SnapshotCounts {
    fn from_entries(entries: &[Entry; MAX_ENTRIES]) -> Result<Self, Error> {
        let mut counts = Self {
            entry_count: 0,
            file_count: 0,
            directory_count: 0,
            live_payload_bytes: 0,
        };
        for entry in entries {
            match entry.kind {
                StoredKind::Free => {}
                StoredKind::File => {
                    counts.entry_count += 1;
                    counts.file_count += 1;
                    counts.live_payload_bytes = counts
                        .live_payload_bytes
                        .checked_add(entry.length)
                        .ok_or(Error::OutOfSpace)?;
                }
                StoredKind::Directory => {
                    counts.entry_count += 1;
                    counts.directory_count += 1;
                }
            }
        }
        if usize::try_from(counts.live_payload_bytes).unwrap() > MAX_LIVE_PAYLOAD_BYTES {
            return Err(Error::OutOfSpace);
        }
        Ok(counts)
    }
}

struct FormatIntent;

impl FormatIntent {
    fn encode() -> Sector {
        let superblock = Superblock::encode();
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&FORMAT_INTENT_MAGIC);
        put_u32(&mut sector, 8, FORMAT_VERSION);
        put_u32(&mut sector, 12, SECTOR_SIZE as u32);
        put_u64(&mut sector, 16, VOLUME_SECTORS);
        put_u64(&mut sector, 24, CHECKPOINT_RELATIVE_LBAS[0]);
        put_u64(&mut sector, 32, BANK_RELATIVE_LBAS[0]);
        put_u32(&mut sector, 40, BANK_SECTORS as u32);
        put_u32(&mut sector, 44, SNAPSHOT_SECTORS as u32);
        sector[48..64].copy_from_slice(&FORMAT_EPOCH);
        sector[64..80].copy_from_slice(&APPDATA_TYPE_GUID);
        put_u32(&mut sector, 80, crc32(&superblock));
        put_u64(&mut sector, 88, fnv1a64(&superblock));
        put_u64(&mut sector, 96, GENERATION_WITNESS ^ VOLUME_SECTORS);
        seal(&mut sector);
        sector
    }
}

struct Superblock;

impl Superblock {
    fn encode() -> Sector {
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&SUPERBLOCK_MAGIC);
        put_u32(&mut sector, 8, FORMAT_VERSION);
        put_u32(&mut sector, 12, SECTOR_SIZE as u32);
        put_u64(&mut sector, 16, VOLUME_SECTORS);
        put_u64(&mut sector, 24, CHECKPOINT_RELATIVE_LBAS[0]);
        put_u64(&mut sector, 32, CHECKPOINT_RELATIVE_LBAS[1]);
        put_u64(&mut sector, 40, BANK_RELATIVE_LBAS[0]);
        put_u64(&mut sector, 48, BANK_RELATIVE_LBAS[1]);
        put_u32(&mut sector, 56, BANK_SECTORS as u32);
        put_u32(&mut sector, 60, SNAPSHOT_SECTORS as u32);
        put_u32(&mut sector, 64, MAX_ENTRIES as u32);
        put_u32(&mut sector, 68, DATA_SECTORS_PER_ENTRY as u32);
        put_u32(&mut sector, 72, MAX_PATH_BYTES as u32);
        put_u32(&mut sector, 76, MAX_FILE_BYTES as u32);
        put_u32(&mut sector, 80, MAX_LIVE_PAYLOAD_BYTES as u32);
        put_u32(&mut sector, 84, u32::from(MAX_PATH_DEPTH));
        sector[88..104].copy_from_slice(&FORMAT_EPOCH);
        sector[104..120].copy_from_slice(&APPDATA_TYPE_GUID);
        put_u64(&mut sector, 120, GENERATION_WITNESS ^ VOLUME_SECTORS);
        seal(&mut sector);
        sector
    }

    fn decode(sector: &Sector) -> Result<(), Corruption> {
        if !seal_is_valid(sector)
            || sector[..8] != SUPERBLOCK_MAGIC
            || le_u32(sector, 8) != FORMAT_VERSION
            || le_u32(sector, 12) != SECTOR_SIZE as u32
            || le_u64(sector, 16) != VOLUME_SECTORS
            || le_u64(sector, 24) != CHECKPOINT_RELATIVE_LBAS[0]
            || le_u64(sector, 32) != CHECKPOINT_RELATIVE_LBAS[1]
            || le_u64(sector, 40) != BANK_RELATIVE_LBAS[0]
            || le_u64(sector, 48) != BANK_RELATIVE_LBAS[1]
            || le_u32(sector, 56) != BANK_SECTORS as u32
            || le_u32(sector, 60) != SNAPSHOT_SECTORS as u32
            || le_u32(sector, 64) != MAX_ENTRIES as u32
            || le_u32(sector, 68) != DATA_SECTORS_PER_ENTRY as u32
            || le_u32(sector, 72) != MAX_PATH_BYTES as u32
            || le_u32(sector, 76) != MAX_FILE_BYTES as u32
            || le_u32(sector, 80) != MAX_LIVE_PAYLOAD_BYTES as u32
            || le_u32(sector, 84) != u32::from(MAX_PATH_DEPTH)
            || sector[88..104] != FORMAT_EPOCH
            || sector[104..120] != APPDATA_TYPE_GUID
            || le_u64(sector, 120) != GENERATION_WITNESS ^ VOLUME_SECTORS
            || sector[128..SEAL_FNV_OFFSET].iter().any(|byte| *byte != 0)
        {
            return Err(Corruption::BadSuperblock);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Manifest {
    counts: SnapshotCounts,
    entries_digest: Digest,
}

impl Manifest {
    fn encode(bank: u8, generation: u64, counts: SnapshotCounts, entries: Digest) -> Sector {
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&MANIFEST_MAGIC);
        put_u32(&mut sector, 8, FORMAT_VERSION);
        sector[12] = bank;
        put_u64(&mut sector, 16, generation);
        put_u32(&mut sector, 24, u32::from(counts.entry_count));
        put_u32(&mut sector, 28, u32::from(counts.file_count));
        put_u32(&mut sector, 32, u32::from(counts.directory_count));
        put_u32(&mut sector, 36, counts.live_payload_bytes);
        put_u32(&mut sector, 40, entries.crc);
        put_u64(&mut sector, 48, entries.fnv);
        sector[56..72].copy_from_slice(&FORMAT_EPOCH);
        put_u32(&mut sector, 72, SNAPSHOT_SECTORS as u32);
        put_u32(&mut sector, 76, MAX_ENTRIES as u32);
        put_u64(&mut sector, 80, generation ^ GENERATION_WITNESS);
        seal(&mut sector);
        sector
    }

    fn decode(sector: &Sector, bank: u8, generation: u64) -> Result<Self, Corruption> {
        if !seal_is_valid(sector)
            || sector[..8] != MANIFEST_MAGIC
            || le_u32(sector, 8) != FORMAT_VERSION
            || sector[12] != bank
            || sector[13..16].iter().any(|byte| *byte != 0)
            || le_u64(sector, 16) != generation
            || le_u32(sector, 44) != 0
            || sector[56..72] != FORMAT_EPOCH
            || le_u32(sector, 72) != SNAPSHOT_SECTORS as u32
            || le_u32(sector, 76) != MAX_ENTRIES as u32
            || le_u64(sector, 80) != generation ^ GENERATION_WITNESS
            || sector[88..SEAL_FNV_OFFSET].iter().any(|byte| *byte != 0)
        {
            return Err(Corruption::BadManifest);
        }
        let entry_count = le_u32(sector, 24);
        let file_count = le_u32(sector, 28);
        let directory_count = le_u32(sector, 32);
        let live_payload_bytes = le_u32(sector, 36);
        if entry_count > MAX_ENTRIES as u32
            || file_count > entry_count
            || directory_count > entry_count
            || file_count + directory_count != entry_count
            || live_payload_bytes > MAX_LIVE_PAYLOAD_BYTES as u32
        {
            return Err(Corruption::BadManifest);
        }
        Ok(Self {
            counts: SnapshotCounts {
                entry_count: entry_count as u8,
                file_count: file_count as u8,
                directory_count: directory_count as u8,
                live_payload_bytes,
            },
            entries_digest: Digest {
                crc: le_u32(sector, 40),
                fnv: le_u64(sector, 48),
            },
        })
    }
}

#[derive(Clone, Copy)]
struct Checkpoint {
    bank: u8,
    generation: u64,
    entry_count: u8,
    live_payload_bytes: u32,
    snapshot_digest: Digest,
    manifest_seal: Digest,
}

impl Checkpoint {
    #[allow(clippy::too_many_arguments)]
    fn encode(
        slot: u8,
        bank: u8,
        generation: u64,
        entry_count: u8,
        live_payload_bytes: u32,
        snapshot: Digest,
        manifest: Digest,
        superblock: &Sector,
    ) -> Sector {
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&CHECKPOINT_MAGIC);
        put_u32(&mut sector, 8, FORMAT_VERSION);
        sector[12] = slot;
        sector[13] = bank;
        put_u64(&mut sector, 16, generation);
        put_u64(&mut sector, 24, generation ^ GENERATION_WITNESS);
        put_u32(&mut sector, 32, u32::from(entry_count));
        put_u32(&mut sector, 36, live_payload_bytes);
        put_u32(&mut sector, 40, snapshot.crc);
        put_u32(&mut sector, 44, manifest.crc);
        put_u64(&mut sector, 48, snapshot.fnv);
        put_u64(&mut sector, 56, manifest.fnv);
        sector[64..80].copy_from_slice(&FORMAT_EPOCH);
        put_u32(&mut sector, 80, crc32(superblock));
        put_u64(&mut sector, 88, fnv1a64(superblock));
        put_u32(&mut sector, 96, SNAPSHOT_SECTORS as u32);
        seal(&mut sector);
        sector
    }

    fn decode(sector: &Sector, slot: u8, superblock: &Sector) -> Result<Self, Corruption> {
        if !seal_is_valid(sector)
            || sector[..8] != CHECKPOINT_MAGIC
            || le_u32(sector, 8) != FORMAT_VERSION
            || sector[12] != slot
            || sector[13] != slot
            || sector[14..16].iter().any(|byte| *byte != 0)
            || le_u64(sector, 24) != le_u64(sector, 16) ^ GENERATION_WITNESS
            || le_u32(sector, 32) > MAX_ENTRIES as u32
            || le_u32(sector, 36) > MAX_LIVE_PAYLOAD_BYTES as u32
            || sector[64..80] != FORMAT_EPOCH
            || le_u32(sector, 80) != crc32(superblock)
            || le_u32(sector, 84) != 0
            || le_u64(sector, 88) != fnv1a64(superblock)
            || le_u32(sector, 96) != SNAPSHOT_SECTORS as u32
            || sector[100..SEAL_FNV_OFFSET].iter().any(|byte| *byte != 0)
        {
            return Err(Corruption::BadCheckpoint);
        }
        Ok(Self {
            bank: sector[13],
            generation: le_u64(sector, 16),
            entry_count: le_u32(sector, 32) as u8,
            live_payload_bytes: le_u32(sector, 36),
            snapshot_digest: Digest {
                crc: le_u32(sector, 40),
                fnv: le_u64(sector, 48),
            },
            manifest_seal: Digest {
                crc: le_u32(sector, 44),
                fnv: le_u64(sector, 56),
            },
        })
    }
}

fn validate_entries(
    entries: &[Entry; MAX_ENTRIES],
    generation: u64,
) -> Result<SnapshotCounts, Error> {
    for (index, entry) in entries.iter().enumerate() {
        if entry.snapshot_generation != generation {
            return Err(Error::Corrupt(Corruption::BadEntry));
        }
        if entry.kind == StoredKind::Free {
            continue;
        }
        for other in &entries[index + 1..] {
            if other.kind != StoredKind::Free
                && entry.principal == other.principal
                && entry.path_bytes() == other.path_bytes()
            {
                return Err(Error::Corrupt(Corruption::DuplicateEntry));
            }
        }
        let parent = parent_path(entry.path_bytes());
        if !parent.is_empty()
            && !entries.iter().any(|candidate| {
                candidate.kind == StoredKind::Directory
                    && candidate.principal == entry.principal
                    && candidate.path_bytes() == parent
            })
        {
            return Err(Error::Corrupt(Corruption::MissingParent));
        }
    }
    SnapshotCounts::from_entries(entries).map_err(|_| Error::Corrupt(Corruption::CountMismatch))
}

fn validate_principal(principal: Principal) -> Result<(), Error> {
    if principal == 0 {
        Err(Error::InvalidPrincipal)
    } else {
        Ok(())
    }
}

fn validate_path(path: &str, allow_root: bool) -> Result<u8, PathError> {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return if allow_root {
            Ok(0)
        } else {
            Err(PathError::Empty)
        };
    }
    if bytes.len() > MAX_PATH_BYTES {
        return Err(PathError::TooLong);
    }
    if bytes[0] == b'/' {
        return Err(PathError::Absolute);
    }
    if bytes[bytes.len() - 1] == b'/' {
        return Err(PathError::TrailingSlash);
    }
    if bytes.contains(&0) {
        return Err(PathError::ContainsNul);
    }

    let mut depth = 0_u8;
    for component in bytes.split(|byte| *byte == b'/') {
        if component.is_empty() {
            return Err(PathError::EmptyComponent);
        }
        if component == b"." || component == b".." {
            return Err(PathError::DotComponent);
        }
        depth = depth.checked_add(1).ok_or(PathError::TooDeep)?;
        if depth > MAX_PATH_DEPTH {
            return Err(PathError::TooDeep);
        }
    }
    Ok(depth)
}

fn parent_path(path: &[u8]) -> &[u8] {
    path.iter()
        .rposition(|byte| *byte == b'/')
        .map(|slash| &path[..slash])
        .unwrap_or(&[])
}

fn is_direct_child(path: &[u8], parent: &[u8]) -> bool {
    if parent.is_empty() {
        return !path.contains(&b'/');
    }
    path.len() > parent.len() + 1
        && path.starts_with(parent)
        && path[parent.len()] == b'/'
        && !path[parent.len() + 1..].contains(&b'/')
}

fn is_descendant(path: &[u8], parent: &[u8]) -> bool {
    path.len() > parent.len() + 1 && path.starts_with(parent) && path[parent.len()] == b'/'
}

fn bank_data_lba(bank: u8, slot: usize, sector: u64) -> Result<u64, Error> {
    if usize::from(bank) >= BANK_RELATIVE_LBAS.len()
        || slot >= MAX_ENTRIES
        || sector >= DATA_SECTORS_PER_ENTRY
    {
        return Err(Error::VolumeBounds);
    }
    BANK_RELATIVE_LBAS[usize::from(bank)]
        .checked_add(DATA_RELATIVE_LBA)
        .and_then(|lba| lba.checked_add(slot as u64 * DATA_SECTORS_PER_ENTRY))
        .and_then(|lba| lba.checked_add(sector))
        .ok_or(Error::VolumeBounds)
}

fn read_at<D: DurableVolumeIo>(io: &mut D, lba: u64, output: &mut Sector) -> Result<(), Error> {
    io.read_sector(lba, output).map_err(map_io_error)
}

fn write_at<D: DurableVolumeIo>(io: &mut D, lba: u64, input: &Sector) -> Result<(), Error> {
    io.write_sector(lba, input).map_err(map_io_error)
}

fn map_io_error(error: IoError) -> Error {
    match error {
        IoError::RequiresReset => Error::RequiresReset,
        IoError::OutcomeUnknown => Error::OutcomeUnknown,
        other => Error::Io(other),
    }
}

fn flush_format<D: DurableVolumeIo>(io: &mut D) -> Result<(), Error> {
    match io.flush() {
        Ok(()) => Ok(()),
        Err(IoError::RequiresReset) => Err(Error::RequiresReset),
        Err(IoError::OutcomeUnknown | IoError::Device) => Err(Error::OutcomeUnknown),
        Err(error) => Err(Error::Io(error)),
    }
}

fn flush_before_checkpoint<D: DurableVolumeIo>(io: &mut D) -> Result<(), Error> {
    io.flush().map_err(map_io_error)
}

fn flush_after_checkpoint<D: DurableVolumeIo>(io: &mut D) -> Result<(), Error> {
    match io.flush() {
        Ok(()) => Ok(()),
        Err(IoError::RequiresReset) => Err(Error::RequiresReset),
        Err(IoError::OutcomeUnknown | IoError::Device) => Err(Error::OutcomeUnknown),
        Err(error) => Err(Error::Io(error)),
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc = crc32_update(crc, *byte);
    }
    !crc
}

fn crc32_update(mut crc: u32, byte: u8) -> u32 {
    crc ^= u32::from(byte);
    for _ in 0..8 {
        let mask = 0_u32.wrapping_sub(crc & 1);
        crc = (crc >> 1) ^ (0xedb8_8320 & mask);
    }
    crc
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn seal(sector: &mut Sector) {
    sector[SEAL_FNV_OFFSET..].fill(0);
    let fnv = fnv1a64(&sector[..SEAL_FNV_OFFSET]);
    put_u64(sector, SEAL_FNV_OFFSET, fnv);
    let crc = crc32(&sector[..SEAL_CRC_OFFSET]);
    put_u32(sector, SEAL_CRC_OFFSET, crc);
    put_u32(sector, SEAL_WITNESS_OFFSET, crc ^ SEAL_WITNESS);
}

fn seal_is_valid(sector: &Sector) -> bool {
    le_u64(sector, SEAL_FNV_OFFSET) == fnv1a64(&sector[..SEAL_FNV_OFFSET])
        && le_u32(sector, SEAL_CRC_OFFSET) == crc32(&sector[..SEAL_CRC_OFFSET])
        && le_u32(sector, SEAL_WITNESS_OFFSET) == le_u32(sector, SEAL_CRC_OFFSET) ^ SEAL_WITNESS
}

fn seal_of(sector: &Sector) -> Digest {
    Digest {
        crc: le_u32(sector, SEAL_CRC_OFFSET),
        fnv: le_u64(sector, SEAL_FNV_OFFSET),
    }
}

fn is_zero(sector: &Sector) -> bool {
    sector.iter().all(|byte| *byte == 0)
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn le_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::vec;
    use std::vec::Vec;

    const FIRST_LBA: u64 = 4;
    const PRINCIPAL_A: Principal = 0x1001;
    const PRINCIPAL_B: Principal = 0x2002;
    const PRINCIPAL_C: Principal = 0x3003;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Persistence {
        Buffered,
        Eager,
        Torn,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Operation {
        Read(u64),
        Write(u64),
        Flush,
    }

    #[derive(Clone)]
    struct MemoryIo {
        durable: Vec<Sector>,
        visible: Vec<Sector>,
        operations: Vec<Operation>,
        mutating_operations: usize,
        cut_after: Option<usize>,
        persistence: Persistence,
        read_fault_lba: Option<u64>,
        read_fault_skip: usize,
        read_fault_error: IoError,
    }

    impl MemoryIo {
        fn zeroed() -> Self {
            let sectors = usize::try_from(FIRST_LBA + VOLUME_SECTORS + 4).unwrap();
            let durable = vec![[0_u8; SECTOR_SIZE]; sectors];
            Self {
                visible: durable.clone(),
                durable,
                operations: Vec::new(),
                mutating_operations: 0,
                cut_after: None,
                persistence: Persistence::Buffered,
                read_fault_lba: None,
                read_fault_skip: 0,
                read_fault_error: IoError::Device,
            }
        }

        fn formatted() -> Self {
            let mut io = Self::zeroed();
            volume().format_virgin(&mut io).unwrap();
            io.operations.clear();
            io.mutating_operations = 0;
            io
        }

        fn set_cut(&mut self, after: usize, persistence: Persistence) {
            self.cut_after = Some(after);
            self.persistence = persistence;
            self.operations.clear();
            self.mutating_operations = 0;
        }

        fn set_read_fault(&mut self, lba: u64, skip_matches: usize, error: IoError) {
            self.read_fault_lba = Some(lba);
            self.read_fault_skip = skip_matches;
            self.read_fault_error = error;
        }

        fn power_cycle(&mut self) {
            self.visible.clone_from(&self.durable);
            self.cut_after = None;
            self.operations.clear();
            self.mutating_operations = 0;
            self.read_fault_lba = None;
            self.read_fault_skip = 0;
        }

        fn cut_now(&self) -> bool {
            self.cut_after == Some(self.mutating_operations)
        }

        fn corrupt_durable(&mut self, relative_lba: u64, offset: usize) {
            let index = usize::try_from(FIRST_LBA + relative_lba).unwrap();
            self.durable[index][offset] ^= 0x80;
            self.visible[index][offset] ^= 0x80;
        }

        fn install_sector(&mut self, relative_lba: u64, sector: Sector) {
            let index = usize::try_from(FIRST_LBA + relative_lba).unwrap();
            self.durable[index] = sector;
            self.visible[index] = sector;
        }
    }

    impl DurableVolumeIo for MemoryIo {
        fn sector_count(&self) -> u64 {
            self.visible.len() as u64
        }

        fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), IoError> {
            self.operations.push(Operation::Read(lba));
            if self.read_fault_lba == Some(lba) {
                if self.read_fault_skip == 0 {
                    self.read_fault_lba = None;
                    return Err(self.read_fault_error);
                }
                self.read_fault_skip -= 1;
            }
            *output = *self
                .visible
                .get(usize::try_from(lba).map_err(|_| IoError::OutOfBounds)?)
                .ok_or(IoError::OutOfBounds)?;
            Ok(())
        }

        fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), IoError> {
            let index = usize::try_from(lba).map_err(|_| IoError::OutOfBounds)?;
            let visible = self.visible.get_mut(index).ok_or(IoError::OutOfBounds)?;
            *visible = *input;
            match self.persistence {
                Persistence::Buffered => {}
                Persistence::Eager => self.durable[index] = *input,
                Persistence::Torn => {
                    self.durable[index][..257].copy_from_slice(&input[..257]);
                }
            }
            self.operations.push(Operation::Write(lba));
            self.mutating_operations += 1;
            if self.cut_now() {
                Err(IoError::OutcomeUnknown)
            } else {
                Ok(())
            }
        }

        fn flush(&mut self) -> Result<(), IoError> {
            self.durable.clone_from(&self.visible);
            self.operations.push(Operation::Flush);
            self.mutating_operations += 1;
            if self.cut_now() {
                Err(IoError::OutcomeUnknown)
            } else {
                Ok(())
            }
        }
    }

    struct ResetIo;

    impl DurableVolumeIo for ResetIo {
        fn sector_count(&self) -> u64 {
            FIRST_LBA + VOLUME_SECTORS
        }

        fn read_sector(&mut self, _lba: u64, _output: &mut Sector) -> Result<(), IoError> {
            Err(IoError::RequiresReset)
        }

        fn write_sector(&mut self, _lba: u64, _input: &Sector) -> Result<(), IoError> {
            Err(IoError::RequiresReset)
        }

        fn flush(&mut self) -> Result<(), IoError> {
            Err(IoError::RequiresReset)
        }
    }

    fn volume() -> AppDataVolume {
        AppDataVolume::new(FIRST_LBA)
    }

    fn read_file(io: &mut MemoryIo, principal: Principal, path: &str) -> Vec<u8> {
        let mut bytes = [0_u8; MAX_FILE_BYTES];
        let read = volume().read(io, principal, path, &mut bytes).unwrap();
        bytes[..read.bytes].to_vec()
    }

    fn install_raw_snapshot(
        io: &mut MemoryIo,
        bank: u8,
        generation: u64,
        mut entries: [Entry; MAX_ENTRIES],
    ) {
        for entry in &mut entries {
            entry.snapshot_generation = generation;
        }
        let zero = [0_u8; SECTOR_SIZE];
        for slot in 0..MAX_ENTRIES {
            for data_sector in 0..DATA_SECTORS_PER_ENTRY {
                let relative = bank_data_lba(bank, slot, data_sector).unwrap();
                io.install_sector(relative, zero);
            }
        }
        let mut entries_checksum = Checksum::new();
        for (slot, entry) in entries.iter().enumerate() {
            let sector = entry.encode(generation);
            entries_checksum.update(&sector);
            io.install_sector(
                BANK_RELATIVE_LBAS[usize::from(bank)] + ENTRY_RELATIVE_LBA + slot as u64,
                sector,
            );
        }
        let counts = SnapshotCounts::from_entries(&entries).unwrap();
        let manifest = Manifest::encode(bank, generation, counts, entries_checksum.finish());
        io.install_sector(BANK_RELATIVE_LBAS[usize::from(bank)], manifest);

        let mut digest = Checksum::new();
        let base = usize::try_from(FIRST_LBA + BANK_RELATIVE_LBAS[usize::from(bank)]).unwrap();
        digest.update(&io.durable[base]);
        for slot in 0..MAX_ENTRIES {
            digest.update(&io.durable[base + usize::try_from(ENTRY_RELATIVE_LBA).unwrap() + slot]);
        }
        for slot in 0..MAX_ENTRIES {
            for data_sector in 0..usize::try_from(DATA_SECTORS_PER_ENTRY).unwrap() {
                let relative = usize::try_from(DATA_RELATIVE_LBA).unwrap()
                    + slot * usize::try_from(DATA_SECTORS_PER_ENTRY).unwrap()
                    + data_sector;
                digest.update(&io.durable[base + relative]);
            }
        }
        let superblock = io.durable[usize::try_from(FIRST_LBA).unwrap()];
        let checkpoint = Checkpoint::encode(
            bank,
            bank,
            generation,
            counts.entry_count,
            counts.live_payload_bytes,
            digest.finish(),
            seal_of(&manifest),
            &superblock,
        );
        io.install_sector(CHECKPOINT_RELATIVE_LBAS[usize::from(bank)], checkpoint);
    }

    #[test]
    fn public_geometry_and_guids_are_frozen() {
        assert_eq!(VOLUME_SECTORS, 1_920);
        assert_eq!(CHECKPOINT_RELATIVE_LBAS, [1, 2]);
        assert_eq!(BANK_RELATIVE_LBAS, [3, 961]);
        assert_eq!(BANK_SECTORS, 958);
        assert_eq!(SNAPSHOT_SECTORS, 289);
        assert_eq!(DATA_RELATIVE_LBA + 32 * 8, SNAPSHOT_SECTORS);
        assert_eq!(
            FORMAT_EPOCH,
            [
                0x54, 0xf1, 0x5c, 0xd2, 0x79, 0xa8, 0x5a, 0x4e, 0x93, 0x64, 0x67, 0xb2, 0xd8, 0x90,
                0x54, 0x01,
            ]
        );
        assert_eq!(
            APPDATA_TYPE_GUID,
            [
                0xa2, 0xd2, 0xf4, 0xb8, 0x3e, 0x7c, 0x91, 0x4b, 0xa6, 0xd5, 0x0f, 0x2e, 0x9c, 0x78,
                0x13, 0x55,
            ]
        );
    }

    #[test]
    fn principal_policy_rejects_malformed_or_overcommitted_tables() {
        assert_eq!(PrincipalPolicy::try_new(&[]), Err(PolicyError::EmptyPolicy));
        assert_eq!(
            PrincipalQuota::try_new(0, 1, 0),
            Err(PolicyError::InvalidPrincipal)
        );
        assert_eq!(
            PrincipalQuota::try_new(PRINCIPAL_A, 0, 0),
            Err(PolicyError::InvalidEntryQuota {
                principal: PRINCIPAL_A,
                limit: 0,
            })
        );
        assert_eq!(
            PrincipalQuota::try_new(PRINCIPAL_A, MAX_ENTRIES + 1, 0),
            Err(PolicyError::InvalidEntryQuota {
                principal: PRINCIPAL_A,
                limit: MAX_ENTRIES + 1,
            })
        );
        assert_eq!(
            PrincipalQuota::try_new(PRINCIPAL_A, 1, MAX_LIVE_PAYLOAD_BYTES + 1),
            Err(PolicyError::InvalidLivePayloadQuota {
                principal: PRINCIPAL_A,
                limit: MAX_LIVE_PAYLOAD_BYTES + 1,
            })
        );

        let a = PrincipalQuota::try_new(PRINCIPAL_A, 1, 1).unwrap();
        let b = PrincipalQuota::try_new(PRINCIPAL_B, 1, 1).unwrap();
        let c = PrincipalQuota::try_new(PRINCIPAL_C, 1, 1).unwrap();
        let d = PrincipalQuota::try_new(0x4004, 1, 1).unwrap();
        let e = PrincipalQuota::try_new(0x5005, 1, 1).unwrap();
        assert_eq!(
            PrincipalPolicy::try_new(&[a, b, c, d, e]),
            Err(PolicyError::TooManyPrincipals {
                maximum: MAX_POLICY_PRINCIPALS,
                actual: MAX_POLICY_PRINCIPALS + 1,
            })
        );
        assert_eq!(
            PrincipalPolicy::try_new(&[a, a]),
            Err(PolicyError::DuplicatePrincipal {
                principal: PRINCIPAL_A,
            })
        );

        let twenty_a = PrincipalQuota::try_new(PRINCIPAL_A, 20, 1).unwrap();
        let twenty_b = PrincipalQuota::try_new(PRINCIPAL_B, 20, 1).unwrap();
        assert_eq!(
            PrincipalPolicy::try_new(&[twenty_a, twenty_b]),
            Err(PolicyError::AggregateEntryQuotaExceeded {
                limit: MAX_ENTRIES,
                actual: 40,
            })
        );
        let eighty_kib_a = PrincipalQuota::try_new(PRINCIPAL_A, 1, 80 * 1_024).unwrap();
        let eighty_kib_b = PrincipalQuota::try_new(PRINCIPAL_B, 1, 80 * 1_024).unwrap();
        assert_eq!(
            PrincipalPolicy::try_new(&[eighty_kib_a, eighty_kib_b]),
            Err(PolicyError::AggregateLivePayloadQuotaExceeded {
                limit: MAX_LIVE_PAYLOAD_BYTES,
                actual: 160 * 1_024,
            })
        );

        let mut hidden_tail = PrincipalPolicy::try_new(&[a]).unwrap();
        hidden_tail.quotas[1] = b;
        assert_eq!(hidden_tail.validate(), Err(PolicyError::InvalidUnusedSlot));
    }

    #[test]
    fn unknown_principal_is_rejected_before_volume_io() {
        let policy = PrincipalPolicy::try_new(&[
            PrincipalQuota::try_new(PRINCIPAL_A, 16, 64 * 1_024).unwrap(),
            PrincipalQuota::try_new(PRINCIPAL_B, 16, 64 * 1_024).unwrap(),
        ])
        .unwrap();
        let mut io = MemoryIo::formatted();
        assert_eq!(
            volume().replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_C,
                "foreign",
                b"no",
                ReplaceCondition::Absent,
            ),
            Err(PolicyError::UnknownPrincipal {
                principal: PRINCIPAL_C,
            })
        );
        assert!(io.operations.is_empty());
        assert_eq!(io.mutating_operations, 0);

        assert_eq!(
            volume().create_dir_with_policy(&mut io, &policy, 0, "invalid"),
            Err(PolicyError::Volume(Error::InvalidPrincipal))
        );
        assert!(io.operations.is_empty());
        assert_eq!(io.mutating_operations, 0);
    }

    #[test]
    fn principal_quota_preserves_capacity_for_another_principal() {
        let policy = PrincipalPolicy::try_new(&[
            PrincipalQuota::try_new(PRINCIPAL_A, 16, 64 * 1_024).unwrap(),
            PrincipalQuota::try_new(PRINCIPAL_B, 16, 64 * 1_024).unwrap(),
        ])
        .unwrap();
        let mut io = MemoryIo::formatted();
        for index in 0..16 {
            let path = format!("a{index}");
            let bytes = [u8::try_from(index).unwrap(); MAX_FILE_BYTES];
            volume()
                .replace_with_policy(
                    &mut io,
                    &policy,
                    PRINCIPAL_A,
                    &path,
                    &bytes,
                    ReplaceCondition::Absent,
                )
                .unwrap();
        }

        io.operations.clear();
        let mutating_before = io.mutating_operations;
        assert_eq!(
            volume().replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_A,
                "a16",
                b"",
                ReplaceCondition::Absent,
            ),
            Err(PolicyError::PrincipalEntryQuotaExceeded {
                principal: PRINCIPAL_A,
                limit: 16,
                actual: 17,
            })
        );
        assert_eq!(io.mutating_operations, mutating_before);
        assert!(
            io.operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );

        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_B,
                "b0",
                b"available",
                ReplaceCondition::Absent,
            )
            .unwrap();
        assert_eq!(read_file(&mut io, PRINCIPAL_B, "b0"), b"available");
    }

    #[test]
    fn replace_policy_accounts_for_live_byte_delta_before_writing() {
        let policy =
            PrincipalPolicy::try_new(&[PrincipalQuota::try_new(PRINCIPAL_A, 2, 4).unwrap()])
                .unwrap();
        let mut io = MemoryIo::formatted();
        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_A,
                "value",
                b"1234",
                ReplaceCondition::Absent,
            )
            .unwrap();
        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_A,
                "value",
                b"xyz",
                ReplaceCondition::Any,
            )
            .unwrap();

        io.operations.clear();
        let mutating_before = io.mutating_operations;
        assert_eq!(
            volume().replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_A,
                "value",
                b"12345",
                ReplaceCondition::Any,
            ),
            Err(PolicyError::PrincipalLivePayloadQuotaExceeded {
                principal: PRINCIPAL_A,
                limit: 4,
                actual: 5,
            })
        );
        assert_eq!(io.mutating_operations, mutating_before);
        assert!(
            io.operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );
        assert_eq!(read_file(&mut io, PRINCIPAL_A, "value"), b"xyz");
    }

    #[test]
    fn policy_recovery_fails_closed_for_unknown_or_overquota_snapshots() {
        let only_a = PrincipalPolicy::try_new(&[PrincipalQuota::try_new(
            PRINCIPAL_A,
            2,
            MAX_LIVE_PAYLOAD_BYTES,
        )
        .unwrap()])
        .unwrap();
        let mut unknown = MemoryIo::formatted();
        let mut entries = [Entry::empty(1); MAX_ENTRIES];
        entries[0] = Entry::directory(PRINCIPAL_B, b"foreign", 1, 1);
        install_raw_snapshot(&mut unknown, 1, 1, entries);
        unknown.operations.clear();
        let mutating_before = unknown.mutating_operations;
        assert_eq!(
            volume().recover_with_policy(&mut unknown, &only_a),
            Err(PolicyError::UnknownPrincipal {
                principal: PRINCIPAL_B,
            })
        );
        assert_eq!(unknown.mutating_operations, mutating_before);
        assert!(
            unknown
                .operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );

        let one_entry = PrincipalPolicy::try_new(&[PrincipalQuota::try_new(
            PRINCIPAL_A,
            1,
            MAX_LIVE_PAYLOAD_BYTES,
        )
        .unwrap()])
        .unwrap();
        let mut too_many = MemoryIo::formatted();
        let mut entries = [Entry::empty(1); MAX_ENTRIES];
        entries[0] = Entry::directory(PRINCIPAL_A, b"one", 1, 1);
        entries[1] = Entry::directory(PRINCIPAL_A, b"two", 1, 1);
        install_raw_snapshot(&mut too_many, 1, 1, entries);
        too_many.operations.clear();
        let mutating_before = too_many.mutating_operations;
        assert_eq!(
            volume().recover_with_policy(&mut too_many, &one_entry),
            Err(PolicyError::PrincipalEntryQuotaExceeded {
                principal: PRINCIPAL_A,
                limit: 1,
                actual: 2,
            })
        );
        assert_eq!(too_many.mutating_operations, mutating_before);
        assert!(
            too_many
                .operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );

        let four_bytes =
            PrincipalPolicy::try_new(&[PrincipalQuota::try_new(PRINCIPAL_A, 1, 4).unwrap()])
                .unwrap();
        let mut too_large = MemoryIo::formatted();
        volume()
            .replace(
                &mut too_large,
                PRINCIPAL_A,
                "legacy",
                b"12345",
                ReplaceCondition::Absent,
            )
            .unwrap();
        too_large.operations.clear();
        let mutating_before = too_large.mutating_operations;
        assert_eq!(
            volume().recover_with_policy(&mut too_large, &four_bytes),
            Err(PolicyError::PrincipalLivePayloadQuotaExceeded {
                principal: PRINCIPAL_A,
                limit: 4,
                actual: 5,
            })
        );
        assert_eq!(too_large.mutating_operations, mutating_before);
        assert!(
            too_large
                .operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );
    }

    #[test]
    fn policy_keeps_equal_paths_and_parent_namespaces_isolated() {
        let policy = PrincipalPolicy::try_new(&[
            PrincipalQuota::try_new(PRINCIPAL_A, 4, 32 * 1_024).unwrap(),
            PrincipalQuota::try_new(PRINCIPAL_B, 4, 32 * 1_024).unwrap(),
        ])
        .unwrap();
        let mut io = MemoryIo::formatted();
        volume()
            .create_dir_with_policy(&mut io, &policy, PRINCIPAL_A, "parent")
            .unwrap();

        io.operations.clear();
        let mutating_before = io.mutating_operations;
        assert_eq!(
            volume().replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_B,
                "parent/file",
                b"wrong parent",
                ReplaceCondition::Absent,
            ),
            Err(PolicyError::Volume(Error::NotFound))
        );
        assert_eq!(io.mutating_operations, mutating_before);
        assert!(
            io.operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );

        volume()
            .create_dir_with_policy(&mut io, &policy, PRINCIPAL_B, "parent")
            .unwrap();
        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_B,
                "parent/file",
                b"owned by b",
                ReplaceCondition::Absent,
            )
            .unwrap();
        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_A,
                "settings",
                b"alpha",
                ReplaceCondition::Absent,
            )
            .unwrap();
        volume()
            .replace_with_policy(
                &mut io,
                &policy,
                PRINCIPAL_B,
                "settings",
                b"beta",
                ReplaceCondition::Absent,
            )
            .unwrap();
        assert_eq!(read_file(&mut io, PRINCIPAL_A, "settings"), b"alpha");
        assert_eq!(read_file(&mut io, PRINCIPAL_B, "settings"), b"beta");
        assert_eq!(
            read_file(&mut io, PRINCIPAL_B, "parent/file"),
            b"owned by b"
        );
    }

    #[test]
    fn virgin_format_installs_generation_zero_and_is_not_implicit() {
        let mut io = MemoryIo::zeroed();
        assert_eq!(volume().recover(&mut io), Err(Error::Unformatted));
        assert_eq!(
            volume().format_virgin(&mut io).unwrap(),
            RecoveryInfo {
                generation: 0,
                checkpoint_slot: 0,
                bank: 0,
                valid_snapshots: 1,
                rejected_snapshots: 1,
                entry_count: 0,
                live_payload_bytes: 0,
            }
        );
        assert_eq!(
            volume().format_virgin(&mut io),
            Err(Error::AlreadyFormatted)
        );

        let mut dirty = MemoryIo::zeroed();
        dirty.visible[usize::try_from(FIRST_LBA + 77).unwrap()][3] = 1;
        dirty.durable.clone_from(&dirty.visible);
        assert_eq!(volume().format_virgin(&mut dirty), Err(Error::NotVirgin));
    }

    #[test]
    fn every_format_write_and_flush_crash_point_can_finish_generation_zero() {
        let mut completed = MemoryIo::zeroed();
        volume().format_virgin(&mut completed).unwrap();
        let mutations: Vec<Operation> = completed
            .operations
            .iter()
            .copied()
            .filter(|operation| !matches!(operation, Operation::Read(_)))
            .collect();
        assert_eq!(mutations.len(), 965);
        assert_eq!(mutations[0], Operation::Write(FIRST_LBA));
        assert_eq!(mutations[1], Operation::Flush);
        assert_eq!(
            mutations[2],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[0])
        );
        assert_eq!(
            mutations[959],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[0] + BANK_SECTORS - 1)
        );
        assert_eq!(mutations[960], Operation::Flush);
        assert_eq!(
            mutations[961],
            Operation::Write(FIRST_LBA + CHECKPOINT_RELATIVE_LBAS[0])
        );
        assert_eq!(mutations[962], Operation::Flush);
        assert_eq!(mutations[963], Operation::Write(FIRST_LBA));
        assert_eq!(mutations[964], Operation::Flush);

        for cut in 1..=mutations.len() {
            let mut io = MemoryIo::zeroed();
            io.set_cut(cut, Persistence::Eager);
            assert_eq!(
                volume().format_virgin(&mut io),
                Err(Error::OutcomeUnknown),
                "cut {cut}"
            );
            io.power_cycle();
            let recovery = match volume().recover(&mut io) {
                Ok(recovery) => recovery,
                Err(Error::Unformatted) => {
                    volume().format_virgin(&mut io).unwrap();
                    volume().recover(&mut io).unwrap()
                }
                result => panic!("cut {cut}: unexpected recovery {result:?}"),
            };
            assert_eq!(recovery.generation, 0, "cut {cut}");
            assert_eq!(recovery.entry_count, 0, "cut {cut}");
            assert_eq!(recovery.live_payload_bytes, 0, "cut {cut}");
            let mut entries = [DirEntry::EMPTY; MAX_ENTRIES];
            let listed = volume()
                .list_all(&mut io, PRINCIPAL_A, &mut entries)
                .unwrap();
            assert_eq!((listed.written, listed.total), (0, 0), "cut {cut}");
        }
    }

    #[test]
    fn exact_intent_replay_is_scoped_and_malformed_or_post_format_media_fail_closed() {
        let intent = FormatIntent::encode();

        for reseal in [false, true] {
            let mut io = MemoryIo::zeroed();
            let mut fake = intent;
            fake[104] ^= 0x40;
            if reseal {
                seal(&mut fake);
            }
            io.install_sector(0, fake);
            assert_eq!(volume().format_virgin(&mut io), Err(Error::NotVirgin));
            assert_eq!(
                volume().recover(&mut io),
                Err(Error::Corrupt(Corruption::BadSuperblock))
            );
        }

        for polluted_relative in [
            CHECKPOINT_RELATIVE_LBAS[1],
            BANK_RELATIVE_LBAS[1],
            VOLUME_SECTORS - 1,
        ] {
            let mut io = MemoryIo::zeroed();
            io.install_sector(0, intent);
            let mut pollution = [0_u8; SECTOR_SIZE];
            pollution[17] = 0xa5;
            io.install_sector(polluted_relative, pollution);
            assert_eq!(volume().recover(&mut io), Err(Error::Unformatted));
            assert_eq!(volume().format_virgin(&mut io), Err(Error::NotVirgin));
            assert_eq!(io.durable[usize::try_from(FIRST_LBA).unwrap()], intent);
        }

        let mut replay = MemoryIo::zeroed();
        replay.install_sector(0, intent);
        replay.install_sector(CHECKPOINT_RELATIVE_LBAS[0], [0x6d; SECTOR_SIZE]);
        replay.install_sector(BANK_RELATIVE_LBAS[0] + 700, [0x91; SECTOR_SIZE]);
        volume().format_virgin(&mut replay).unwrap();
        assert_eq!(volume().recover(&mut replay).unwrap().generation, 0);
        assert!(is_zero(
            &replay.durable[usize::try_from(FIRST_LBA + BANK_RELATIVE_LBAS[0] + 700).unwrap()]
        ));

        let mut damaged_final = MemoryIo::formatted();
        damaged_final.corrupt_durable(0, 127);
        assert_eq!(
            volume().recover(&mut damaged_final),
            Err(Error::Corrupt(Corruption::BadSuperblock))
        );
        assert_eq!(
            volume().format_virgin(&mut damaged_final),
            Err(Error::NotVirgin)
        );
    }

    #[test]
    fn torn_intent_or_final_superblock_is_never_guessed_to_be_recoverable() {
        let mut torn_intent = MemoryIo::zeroed();
        torn_intent.set_cut(1, Persistence::Torn);
        assert_eq!(
            volume().format_virgin(&mut torn_intent),
            Err(Error::OutcomeUnknown)
        );
        torn_intent.power_cycle();
        assert_eq!(
            volume().recover(&mut torn_intent),
            Err(Error::Corrupt(Corruption::BadSuperblock))
        );
        assert_eq!(
            volume().format_virgin(&mut torn_intent),
            Err(Error::NotVirgin)
        );

        let mut io = MemoryIo::zeroed();
        io.set_cut(964, Persistence::Torn);
        assert_eq!(volume().format_virgin(&mut io), Err(Error::OutcomeUnknown));
        io.power_cycle();
        assert_eq!(
            volume().recover(&mut io),
            Err(Error::Corrupt(Corruption::BadSuperblock))
        );
        assert_eq!(volume().format_virgin(&mut io), Err(Error::NotVirgin));
    }

    #[test]
    fn volume_bounds_are_checked_before_io() {
        let mut io = MemoryIo::zeroed();
        let out = AppDataVolume::new(io.sector_count() - VOLUME_SECTORS + 1);
        assert_eq!(out.recover(&mut io), Err(Error::VolumeBounds));
        assert!(io.operations.is_empty());
    }

    #[test]
    fn reset_required_is_not_collapsed_into_a_generic_io_error() {
        assert_eq!(volume().recover(&mut ResetIo), Err(Error::RequiresReset));
    }

    #[test]
    fn create_replace_read_list_stat_and_unlink_are_data_driven() {
        let mut io = MemoryIo::formatted();
        let docs = volume().create_dir(&mut io, PRINCIPAL_A, "docs").unwrap();
        assert_eq!(docs.committed_generation, 1);
        volume()
            .create_dir(&mut io, PRINCIPAL_A, "docs/work")
            .unwrap();
        let write = volume()
            .replace(
                &mut io,
                PRINCIPAL_A,
                "docs/work/note.txt",
                b"durable note",
                ReplaceCondition::Absent,
            )
            .unwrap();
        assert_eq!(write.entry_revision, Some(3));
        assert_eq!(
            read_file(&mut io, PRINCIPAL_A, "docs/work/note.txt"),
            b"durable note"
        );
        assert_eq!(
            volume()
                .stat(&mut io, PRINCIPAL_A, "docs/work/note.txt")
                .unwrap(),
            Metadata {
                kind: EntryKind::File,
                length: 12,
                revision: 3,
                snapshot_generation: 3,
            }
        );

        let mut root = [DirEntry::EMPTY; MAX_ENTRIES];
        let root_list = volume().list(&mut io, PRINCIPAL_A, "", &mut root).unwrap();
        assert_eq!((root_list.total, root_list.written), (1, 1));
        assert_eq!(root[0].name(), "docs");
        let mut work = [DirEntry::EMPTY; 1];
        let work_list = volume()
            .list(&mut io, PRINCIPAL_A, "docs/work", &mut work)
            .unwrap();
        assert_eq!((work_list.total, work_list.written), (1, 1));
        assert_eq!(work[0].name(), "note.txt");

        let mut flat = [DirEntry::EMPTY; 2];
        let flat_list = volume().list_all(&mut io, PRINCIPAL_A, &mut flat).unwrap();
        assert_eq!((flat_list.total, flat_list.written), (3, 2));
        assert_eq!(flat[0].name(), "docs");
        assert_eq!(flat[1].name(), "docs/work");
        let mut complete = [DirEntry::EMPTY; MAX_ENTRIES];
        let complete_list = volume()
            .list_all(&mut io, PRINCIPAL_A, &mut complete)
            .unwrap();
        assert_eq!((complete_list.total, complete_list.written), (3, 3));
        assert_eq!(complete[2].name(), "docs/work/note.txt");

        assert_eq!(
            volume().unlink(&mut io, PRINCIPAL_A, "docs/work"),
            Err(Error::NotEmpty)
        );
        volume()
            .unlink(&mut io, PRINCIPAL_A, "docs/work/note.txt")
            .unwrap();
        volume().unlink(&mut io, PRINCIPAL_A, "docs/work").unwrap();
        assert_eq!(
            volume().read(&mut io, PRINCIPAL_A, "docs/work/note.txt", &mut [0; 16]),
            Err(Error::NotFound)
        );
    }

    #[test]
    fn principals_have_independent_roots_and_equal_paths() {
        let mut io = MemoryIo::formatted();
        volume()
            .replace(
                &mut io,
                PRINCIPAL_A,
                "settings",
                b"alpha",
                ReplaceCondition::Absent,
            )
            .unwrap();
        volume()
            .replace(
                &mut io,
                PRINCIPAL_B,
                "settings",
                b"beta",
                ReplaceCondition::Absent,
            )
            .unwrap();
        assert_eq!(read_file(&mut io, PRINCIPAL_A, "settings"), b"alpha");
        assert_eq!(read_file(&mut io, PRINCIPAL_B, "settings"), b"beta");
        assert_eq!(
            volume().stat(&mut io, 0, "settings"),
            Err(Error::InvalidPrincipal)
        );
    }

    #[test]
    fn replace_conditions_provide_create_and_generation_cas() {
        let mut io = MemoryIo::formatted();
        let first = volume()
            .replace(
                &mut io,
                PRINCIPAL_A,
                "counter",
                b"one",
                ReplaceCondition::Absent,
            )
            .unwrap();
        assert_eq!(first.entry_revision, Some(1));
        assert_eq!(
            volume().replace(
                &mut io,
                PRINCIPAL_A,
                "counter",
                b"duplicate",
                ReplaceCondition::Absent,
            ),
            Err(Error::Conflict {
                expected: ReplaceCondition::Absent,
                actual_generation: Some(1),
            })
        );
        assert_eq!(
            volume().replace_cas(&mut io, PRINCIPAL_A, "counter", b"two", 9),
            Err(Error::Conflict {
                expected: ReplaceCondition::Generation(9),
                actual_generation: Some(1),
            })
        );
        let second = volume()
            .replace_cas(&mut io, PRINCIPAL_A, "counter", b"two", 1)
            .unwrap();
        assert_eq!(second.entry_revision, Some(2));
        assert_eq!(read_file(&mut io, PRINCIPAL_A, "counter"), b"two");
    }

    #[test]
    fn explicit_parent_directories_are_required() {
        let mut io = MemoryIo::formatted();
        assert_eq!(
            volume().replace(
                &mut io,
                PRINCIPAL_A,
                "missing/file",
                b"x",
                ReplaceCondition::Any,
            ),
            Err(Error::NotFound)
        );
        volume()
            .replace(&mut io, PRINCIPAL_A, "plain", b"x", ReplaceCondition::Any)
            .unwrap();
        assert_eq!(
            volume().create_dir(&mut io, PRINCIPAL_A, "plain/child"),
            Err(Error::NotDirectory)
        );
        assert_eq!(
            volume().create_dir(&mut io, PRINCIPAL_A, "plain"),
            Err(Error::AlreadyExists)
        );
    }

    #[test]
    fn canonical_paths_reject_all_ambiguous_forms() {
        let mut io = MemoryIo::formatted();
        let cases = [
            ("", PathError::Empty),
            ("/absolute", PathError::Absolute),
            ("trailing/", PathError::TrailingSlash),
            ("two//parts", PathError::EmptyComponent),
            ("./part", PathError::DotComponent),
            ("part/../escape", PathError::DotComponent),
            ("a/b/c/d/e", PathError::TooDeep),
            ("nul\0byte", PathError::ContainsNul),
        ];
        for (path, expected) in cases {
            assert_eq!(
                volume().create_dir(&mut io, PRINCIPAL_A, path),
                Err(Error::InvalidPath(expected)),
                "{path:?}"
            );
        }
        let long = "x".repeat(MAX_PATH_BYTES + 1);
        assert_eq!(
            volume().create_dir(&mut io, PRINCIPAL_A, &long),
            Err(Error::InvalidPath(PathError::TooLong))
        );
    }

    #[test]
    fn file_size_buffer_and_entry_quotas_are_enforced() {
        let mut io = MemoryIo::formatted();
        let oversized = [0x55; MAX_FILE_BYTES + 1];
        assert_eq!(
            volume().replace(
                &mut io,
                PRINCIPAL_A,
                "large",
                &oversized,
                ReplaceCondition::Any,
            ),
            Err(Error::FileTooLarge)
        );
        volume()
            .replace(
                &mut io,
                PRINCIPAL_A,
                "small",
                b"1234",
                ReplaceCondition::Any,
            )
            .unwrap();
        assert_eq!(
            volume().read(&mut io, PRINCIPAL_A, "small", &mut [0; 3]),
            Err(Error::BufferTooSmall { needed: 4 })
        );

        let full = [0x5a; MAX_FILE_BYTES];
        for index in 1..MAX_ENTRIES {
            let path = format!("f{index:02}");
            volume()
                .replace(&mut io, PRINCIPAL_A, &path, &full, ReplaceCondition::Absent)
                .unwrap();
        }
        volume()
            .replace(&mut io, PRINCIPAL_A, "small", &full, ReplaceCondition::Any)
            .unwrap();
        let full_info = volume().recover(&mut io).unwrap();
        assert_eq!(full_info.entry_count, 32);
        assert_eq!(full_info.live_payload_bytes, 128 * 1_024);
        assert_eq!(
            volume().replace(
                &mut io,
                PRINCIPAL_A,
                "one-too-many",
                b"",
                ReplaceCondition::Absent,
            ),
            Err(Error::OutOfSpace)
        );
    }

    #[test]
    fn mutation_writes_inactive_snapshot_then_checkpoint_with_two_flushes() {
        let mut io = MemoryIo::formatted();
        volume()
            .replace(
                &mut io,
                PRINCIPAL_A,
                "ordered",
                b"payload",
                ReplaceCondition::Absent,
            )
            .unwrap();
        let mutations: Vec<Operation> = io
            .operations
            .iter()
            .copied()
            .filter(|operation| !matches!(operation, Operation::Read(_)))
            .collect();
        assert_eq!(mutations.len(), 292);
        assert_eq!(
            mutations[0],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[1] + DATA_RELATIVE_LBA)
        );
        assert_eq!(
            mutations[255],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[1] + DATA_RELATIVE_LBA + 255)
        );
        assert_eq!(
            mutations[256],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[1] + ENTRY_RELATIVE_LBA)
        );
        assert_eq!(
            mutations[288],
            Operation::Write(FIRST_LBA + BANK_RELATIVE_LBAS[1])
        );
        assert_eq!(mutations[289], Operation::Flush);
        assert_eq!(
            mutations[290],
            Operation::Write(FIRST_LBA + CHECKPOINT_RELATIVE_LBAS[1])
        );
        assert_eq!(mutations[291], Operation::Flush);
    }

    #[test]
    fn post_publication_confirmation_read_failures_are_outcome_unknown() {
        for error in [IoError::Device, IoError::RequiresReset] {
            let mut checkpoint_read = MemoryIo::formatted();
            checkpoint_read.set_read_fault(FIRST_LBA + CHECKPOINT_RELATIVE_LBAS[1], 1, error);
            assert_eq!(
                volume().replace(
                    &mut checkpoint_read,
                    PRINCIPAL_A,
                    "published",
                    b"checkpoint",
                    ReplaceCondition::Absent,
                ),
                Err(Error::OutcomeUnknown),
                "checkpoint read: {error:?}"
            );
            checkpoint_read.power_cycle();
            assert_eq!(
                read_file(&mut checkpoint_read, PRINCIPAL_A, "published"),
                b"checkpoint",
                "checkpoint read: {error:?}"
            );

            let mut bank_read = MemoryIo::formatted();
            bank_read.set_read_fault(FIRST_LBA + BANK_RELATIVE_LBAS[1], 1, error);
            assert_eq!(
                volume().replace(
                    &mut bank_read,
                    PRINCIPAL_A,
                    "published",
                    b"bank",
                    ReplaceCondition::Absent,
                ),
                Err(Error::OutcomeUnknown),
                "bank read: {error:?}"
            );
            bank_read.power_cycle();
            assert_eq!(
                read_file(&mut bank_read, PRINCIPAL_A, "published"),
                b"bank",
                "bank read: {error:?}"
            );
        }
    }

    #[test]
    fn every_write_and_flush_crash_point_recovers_old_or_new_never_mixed() {
        let mut baseline = MemoryIo::formatted();
        volume()
            .replace(
                &mut baseline,
                PRINCIPAL_A,
                "first",
                b"old-first",
                ReplaceCondition::Absent,
            )
            .unwrap();
        volume()
            .replace(
                &mut baseline,
                PRINCIPAL_A,
                "second",
                b"stable-second",
                ReplaceCondition::Absent,
            )
            .unwrap();

        for cut in 1..=292 {
            let mut io = baseline.clone();
            io.set_cut(cut, Persistence::Eager);
            assert_eq!(
                volume().replace(
                    &mut io,
                    PRINCIPAL_A,
                    "first",
                    b"new-first",
                    ReplaceCondition::Generation(1),
                ),
                Err(Error::OutcomeUnknown),
                "cut {cut}"
            );
            io.power_cycle();
            let info = volume().recover(&mut io).unwrap();
            let first = read_file(&mut io, PRINCIPAL_A, "first");
            let second = read_file(&mut io, PRINCIPAL_A, "second");
            assert_eq!(second, b"stable-second", "cut {cut}");
            assert!(
                (info.generation == 2 && first == b"old-first")
                    || (info.generation == 3 && first == b"new-first"),
                "cut {cut}, generation {}, first {:?}",
                info.generation,
                first
            );
        }
    }

    #[test]
    fn buffered_and_torn_checkpoint_boundaries_recover_safely() {
        let mut baseline = MemoryIo::formatted();
        volume()
            .replace(
                &mut baseline,
                PRINCIPAL_A,
                "value",
                b"old",
                ReplaceCondition::Absent,
            )
            .unwrap();

        for (cut, persistence, expected) in [
            (1, Persistence::Torn, &b"old"[..]),
            (257, Persistence::Torn, &b"old"[..]),
            (289, Persistence::Torn, &b"old"[..]),
            (290, Persistence::Buffered, &b"old"[..]),
            (291, Persistence::Buffered, &b"old"[..]),
            (292, Persistence::Buffered, &b"new"[..]),
        ] {
            let mut io = baseline.clone();
            io.set_cut(cut, persistence);
            assert!(
                volume()
                    .replace(
                        &mut io,
                        PRINCIPAL_A,
                        "value",
                        b"new",
                        ReplaceCondition::Generation(1),
                    )
                    .is_err()
            );
            io.power_cycle();
            assert_eq!(read_file(&mut io, PRINCIPAL_A, "value"), expected);
        }
    }

    #[test]
    fn corrupt_newest_data_entry_manifest_or_checkpoint_falls_back_whole_bank() {
        for (relative, offset) in [
            (BANK_RELATIVE_LBAS[0] + DATA_RELATIVE_LBA, 0),
            (BANK_RELATIVE_LBAS[0] + ENTRY_RELATIVE_LBA, 64),
            (BANK_RELATIVE_LBAS[0] + MANIFEST_RELATIVE_LBA, 36),
            (CHECKPOINT_RELATIVE_LBAS[0], 40),
        ] {
            let mut io = MemoryIo::formatted();
            volume()
                .replace(
                    &mut io,
                    PRINCIPAL_A,
                    "value",
                    b"old",
                    ReplaceCondition::Absent,
                )
                .unwrap();
            volume()
                .replace_cas(&mut io, PRINCIPAL_A, "value", b"new", 1)
                .unwrap();
            assert_eq!(volume().recover(&mut io).unwrap().generation, 2);
            io.corrupt_durable(relative, offset);
            let recovered = volume().recover(&mut io).unwrap();
            assert_eq!(
                recovered.generation, 1,
                "relative {relative}, offset {offset}"
            );
            assert_eq!(read_file(&mut io, PRINCIPAL_A, "value"), b"old");
        }
    }

    #[test]
    fn duplicate_entries_are_rejected_as_one_bad_bank() {
        let mut io = MemoryIo::formatted();
        let mut entries = [Entry::empty(1); MAX_ENTRIES];
        entries[0] = Entry::directory(PRINCIPAL_A, b"same", 1, 1);
        entries[1] = Entry::directory(PRINCIPAL_A, b"same", 1, 1);
        install_raw_snapshot(&mut io, 1, 1, entries);
        let recovered = volume().recover(&mut io).unwrap();
        assert_eq!(recovered.generation, 0);
        assert_eq!(recovered.valid_snapshots, 1);
        assert_eq!(recovered.rejected_snapshots, 1);

        io.install_sector(CHECKPOINT_RELATIVE_LBAS[0], [0; SECTOR_SIZE]);
        assert_eq!(
            volume().recover(&mut io),
            Err(Error::Corrupt(Corruption::NoValidSnapshot))
        );
    }

    #[test]
    fn fully_valid_checkpoint_generations_must_be_unique_and_adjacent() {
        let mut duplicate = MemoryIo::formatted();
        let entries = [Entry::empty(0); MAX_ENTRIES];
        install_raw_snapshot(&mut duplicate, 1, 0, entries);
        assert_eq!(
            volume().recover(&mut duplicate),
            Err(Error::Corrupt(Corruption::AmbiguousGeneration))
        );

        let mut io = MemoryIo::formatted();
        let entries = [Entry::empty(2); MAX_ENTRIES];
        install_raw_snapshot(&mut io, 1, 2, entries);
        assert_eq!(
            volume().recover(&mut io),
            Err(Error::Corrupt(Corruption::GenerationGap))
        );
    }

    #[test]
    fn missing_parent_and_invalid_utf8_entries_are_rejected() {
        let mut io = MemoryIo::formatted();
        let mut entries = [Entry::empty(1); MAX_ENTRIES];
        entries[0] = Entry::file(PRINCIPAL_A, b"missing/file", 2, 1, b"");
        install_raw_snapshot(&mut io, 1, 1, entries);
        assert_eq!(volume().recover(&mut io).unwrap().generation, 0);

        let mut io = MemoryIo::formatted();
        let mut entry = Entry::directory(PRINCIPAL_A, b"okay", 1, 1);
        entry.path[0] = 0xff;
        let mut entries = [Entry::empty(1); MAX_ENTRIES];
        entries[0] = entry;
        install_raw_snapshot(&mut io, 1, 1, entries);
        assert_eq!(volume().recover(&mut io).unwrap().generation, 0);
    }

    #[test]
    fn record_seals_and_reference_hashes_detect_corruption() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(fnv1a64(b"hello"), 0xa430_d846_80aa_bd0b);
        let superblock = Superblock::encode();
        assert_eq!(Superblock::decode(&superblock), Ok(()));
        for offset in [0, 16, 88, SEAL_FNV_OFFSET, SEAL_CRC_OFFSET] {
            let mut corrupt = superblock;
            corrupt[offset] ^= 1;
            assert_eq!(Superblock::decode(&corrupt), Err(Corruption::BadSuperblock));
        }
    }
}
