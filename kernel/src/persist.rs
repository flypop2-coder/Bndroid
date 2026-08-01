//! Allocation-free, crash-recoverable record format for `BNDROID_DATA`.
//!
//! The immutable superblock names two one-sector record slots. Updates always
//! replace the inactive slot and leave the previously selected slot untouched;
//! a sector-wide CRC rejects torn or corrupted candidates on the next boot.

use bndr_sm::verified_manifest::sha256;

use crate::block::{SECTOR_SIZE, Sector};

pub const DATA_SUPERBLOCK_MAGIC: [u8; 8] = *b"BNDRDAT1";
pub const DATA_RECORD_MAGIC: [u8; 8] = *b"BNDRREC1";
pub const BOOT_STATE_MAGIC: [u8; 8] = *b"BNDRBOOT";
pub const DATA_FORMAT_VERSION: u32 = 1;
pub const DATA_SLOT_COUNT: usize = 2;
pub const DATA_SUPERBLOCK_RELATIVE_LBA: u64 = 0;
pub const DATA_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [1, 2];
pub const FORMAT_EPOCH_BYTES: usize = 16;
pub type FormatEpoch = [u8; FORMAT_EPOCH_BYTES];
pub const DATA_RECORD_HEADER_BYTES: usize = 80;
pub const DATA_RECORD_CRC_OFFSET: usize = SECTOR_SIZE - size_of::<u32>();
pub const DATA_RECORD_MAX_PAYLOAD: usize = DATA_RECORD_CRC_OFFSET - DATA_RECORD_HEADER_BYTES;
pub const BOOT_STATE_BYTES: usize = 32;
pub const DEVICE_HEALTH_STATE_MAGIC: [u8; 8] = *b"BNDRHLT1";
pub const DEVICE_HEALTH_STATE_VERSION: u32 = 1;
pub const DEVICE_HEALTH_STATE_BYTES: usize = 80;
pub const MANIFEST_ROLLBACK_STATE_MAGIC: [u8; 8] = *b"BNDRRBK1";
pub const MANIFEST_ROLLBACK_STATE_VERSION: u32 = 1;
pub const MANIFEST_ROLLBACK_STATE_BYTES: usize = 64;
pub const MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [3, 4];
pub const MANIFEST_KEY_POLICY_STATE_MAGIC: [u8; 8] = *b"BNDRKEY1";
pub const MANIFEST_KEY_POLICY_STATE_VERSION: u32 = 1;
pub const MANIFEST_KEY_POLICY_STATE_BYTES: usize = 96;
pub const MAINTENANCE_AUDIT_STATE_MAGIC: [u8; 8] = *b"BNDRMAU1";
pub const MAINTENANCE_AUDIT_STATE_VERSION: u32 = 1;
pub const MAINTENANCE_AUDIT_STATE_BYTES: usize = 320;
pub const MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [5, 6];
pub const MAINTENANCE_EXECUTION_STATE_MAGIC: [u8; 8] = *b"BNDRMEX1";
pub const MAINTENANCE_EXECUTION_STATE_VERSION: u32 = 1;
pub const MAINTENANCE_EXECUTION_STATE_BYTES: usize = 368;
pub const MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [7, 8];
pub const MAINTENANCE_EXECUTION_STOP_DRAINED: u32 = 1;
pub const MAINTENANCE_EXECUTION_CLEAN_SHUTDOWN: u32 = 1 << 1;
pub const MAINTENANCE_EXECUTION_REQUIRED_FLAGS: u32 =
    MAINTENANCE_EXECUTION_STOP_DRAINED | MAINTENANCE_EXECUTION_CLEAN_SHUTDOWN;
pub const MAINTENANCE_STEP_STATE_MAGIC: [u8; 8] = *b"BNDRMST1";
pub const MAINTENANCE_STEP_STATE_VERSION: u32 = 1;
pub const MAINTENANCE_STEP_STATE_BYTES: usize = 376;
pub const MAINTENANCE_STEP_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [9, 10];
pub const MAINTENANCE_STEP_COUNT: u32 = 3;
pub const MAINTENANCE_STEP_ROTATION_ONE: u32 = 1;
pub const MAINTENANCE_STEP_ROTATION_TWO: u32 = 2;
pub const MAINTENANCE_STEP_DRAIN: u32 = 3;
pub const MAINTENANCE_STEP_OPERATION_STORAGE_ROTATION: u64 = 1;
pub const MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START: u32 = 1;
pub const MAINTENANCE_STEP_DRAIN_VALIDATED: u32 = 1 << 1;
pub const MAINTENANCE_PLAN_STATE_MAGIC: [u8; 8] = *b"BNDRMPL1";
pub const MAINTENANCE_PLAN_STATE_VERSION: u32 = 1;
pub const MAINTENANCE_PLAN_STATE_BYTES: usize = 424;
pub const MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [11, 12];
pub const MAINTENANCE_PLAN_PHASE_PREPARED: u32 = 1;
pub const MAINTENANCE_PLAN_PHASE_APPLYING: u32 = 2;
pub const MAINTENANCE_PLAN_PHASE_CONFIRMED: u32 = 3;
pub const MAINTENANCE_PLAN_PHASE_COMPENSATED: u32 = 4;
pub const MAINTENANCE_PLAN_IDEMPOTENT_EFFECT: u32 = 1;
pub const MAINTENANCE_PLAN_RESULT_MAY_BE_UNKNOWN: u32 = 1 << 1;
pub const MAINTENANCE_PLAN_EFFECT_CONFIRMED: u32 = 1 << 2;
pub const MAINTENANCE_PLAN_COMPENSATED_WITHOUT_EFFECT: u32 = 1 << 3;
pub const MAINTENANCE_PLAN_TRANSITIONS_PER_SEQUENCE: u64 = 9;
pub const SIGNED_MAINTENANCE_PLAN_STATE_MAGIC: [u8; 8] = *b"BNDRMPB1";
pub const SIGNED_MAINTENANCE_PLAN_STATE_VERSION: u32 = 1;
pub const SIGNED_MAINTENANCE_PLAN_STATE_BYTES: usize = 336;
pub const SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS: [u64; DATA_SLOT_COUNT] = [13, 14];
pub const SIGNED_MAINTENANCE_PLAN_OPERATION_COUNT: u32 = 3;
pub const SIGNED_MAINTENANCE_PLAN_TRANSITION_COUNT: u32 = 9;
pub const SIGNED_MAINTENANCE_PLAN_PROGRAM_VERSION: u32 = 1;
pub const SIGNED_MAINTENANCE_PLAN_PREAPPLY_CANCEL_MASK: u32 = 1;

const RECORD_COMMITTED: u32 = 1;
const RECORD_COMMIT_COOKIE: u64 = 0x434f_4d4d_4954_2131;
const BOOT_STATE_WITNESS: u64 = 0xb24d_25da_7a5e_c001;
const DEVICE_HEALTH_BOOT_OPEN: u32 = 1;
const DEVICE_HEALTH_READ_ONLY: u32 = 1;
const DEVICE_HEALTH_FLUSH_SUPPORTED: u32 = 1 << 1;
const DEVICE_HEALTH_MODE_FLAGS: u32 = DEVICE_HEALTH_READ_ONLY | DEVICE_HEALTH_FLUSH_SUPPORTED;
const DEVICE_HEALTH_WITNESS: u64 = 0xb24d_63da_7a5e_c001;
const MANIFEST_ROLLBACK_WITNESS: u64 = 0xb24d_75da_7a5e_c001;
const MANIFEST_KEY_POLICY_WITNESS: u64 = 0xb24d_76da_7a5e_c001;
const MAINTENANCE_AUDIT_WITNESS: u64 = 0xb24d_77da_7a5e_c001;
const MAINTENANCE_EXECUTION_WITNESS: u64 = 0xb24d_78da_7a5e_c001;
const MAINTENANCE_STEP_WITNESS: u64 = 0xb24d_79da_7a5e_c001;
const MAINTENANCE_PLAN_WITNESS: u64 = 0xb24d_80da_7a5e_c001;
const SIGNED_MAINTENANCE_PLAN_WITNESS: u64 = 0xb24d_81da_7a5e_c001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuperblockError {
    BadMagic,
    UnsupportedVersion,
    BadSectorSize,
    BadSlotCount,
    BadRecordSize,
    BadSlotLayout,
    BadPartitionSize,
    ZeroFormatEpoch,
    FormatEpochMismatch,
    NonZeroReserved,
    CrcMismatch,
}

impl SuperblockError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadMagic => "persistent-data superblock magic is invalid",
            Self::UnsupportedVersion => "persistent-data format version is unsupported",
            Self::BadSectorSize => "persistent-data sector size is invalid",
            Self::BadSlotCount => "persistent-data slot count is invalid",
            Self::BadRecordSize => "persistent-data record size is invalid",
            Self::BadSlotLayout => "persistent-data record slots are invalid",
            Self::BadPartitionSize => "persistent-data partition size is invalid",
            Self::ZeroFormatEpoch => "persistent-data format epoch is zero",
            Self::FormatEpochMismatch => "persistent-data superblock format epoch is foreign",
            Self::NonZeroReserved => "persistent-data superblock reserved bytes are nonzero",
            Self::CrcMismatch => "persistent-data superblock CRC32 is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataSuperblock {
    pub partition_sectors: u64,
    pub slot_relative_lbas: [u64; DATA_SLOT_COUNT],
    pub format_epoch: FormatEpoch,
}

impl DataSuperblock {
    pub fn encode(
        partition_sectors: u64,
        format_epoch: FormatEpoch,
    ) -> Result<Sector, SuperblockError> {
        validate_superblock_epoch(format_epoch)?;
        validate_slot_layout(partition_sectors, DATA_SLOT_RELATIVE_LBAS)?;
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&DATA_SUPERBLOCK_MAGIC);
        put_u32(&mut sector, 8, DATA_FORMAT_VERSION);
        put_u32(&mut sector, 12, SECTOR_SIZE as u32);
        put_u32(&mut sector, 16, DATA_SLOT_COUNT as u32);
        put_u32(&mut sector, 20, SECTOR_SIZE as u32);
        put_u64(&mut sector, 24, DATA_SLOT_RELATIVE_LBAS[0]);
        put_u64(&mut sector, 32, DATA_SLOT_RELATIVE_LBAS[1]);
        put_u64(&mut sector, 40, partition_sectors);
        sector[48..48 + FORMAT_EPOCH_BYTES].copy_from_slice(&format_epoch);
        seal_sector(&mut sector);
        Ok(sector)
    }

    pub fn decode(
        sector: &Sector,
        expected_partition_sectors: u64,
        expected_format_epoch: FormatEpoch,
    ) -> Result<Self, SuperblockError> {
        validate_superblock_epoch(expected_format_epoch)?;
        if sector[..8] != DATA_SUPERBLOCK_MAGIC {
            return Err(SuperblockError::BadMagic);
        }
        if le_u32(sector, 8) != DATA_FORMAT_VERSION {
            return Err(SuperblockError::UnsupportedVersion);
        }
        if le_u32(sector, 12) != SECTOR_SIZE as u32 {
            return Err(SuperblockError::BadSectorSize);
        }
        if le_u32(sector, 16) != DATA_SLOT_COUNT as u32 {
            return Err(SuperblockError::BadSlotCount);
        }
        if le_u32(sector, 20) != SECTOR_SIZE as u32 {
            return Err(SuperblockError::BadRecordSize);
        }
        let slot_relative_lbas = [le_u64(sector, 24), le_u64(sector, 32)];
        let partition_sectors = le_u64(sector, 40);
        if partition_sectors != expected_partition_sectors {
            return Err(SuperblockError::BadPartitionSize);
        }
        validate_slot_layout(partition_sectors, slot_relative_lbas)?;
        let format_epoch: FormatEpoch = sector[48..48 + FORMAT_EPOCH_BYTES]
            .try_into()
            .expect("fixed format epoch range");
        validate_superblock_epoch(format_epoch)?;
        if format_epoch != expected_format_epoch {
            return Err(SuperblockError::FormatEpochMismatch);
        }
        if sector[48 + FORMAT_EPOCH_BYTES..DATA_RECORD_CRC_OFFSET]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(SuperblockError::NonZeroReserved);
        }
        if !sector_crc_is_valid(sector) {
            return Err(SuperblockError::CrcMismatch);
        }
        Ok(Self {
            partition_sectors,
            slot_relative_lbas,
            format_epoch,
        })
    }
}

fn validate_superblock_epoch(format_epoch: FormatEpoch) -> Result<(), SuperblockError> {
    if format_epoch.iter().all(|byte| *byte == 0) {
        Err(SuperblockError::ZeroFormatEpoch)
    } else {
        Ok(())
    }
}

fn validate_slot_layout(
    partition_sectors: u64,
    slots: [u64; DATA_SLOT_COUNT],
) -> Result<(), SuperblockError> {
    if partition_sectors < 3 {
        return Err(SuperblockError::BadPartitionSize);
    }
    if slots != DATA_SLOT_RELATIVE_LBAS
        || slots[0] == DATA_SUPERBLOCK_RELATIVE_LBA
        || slots[0] == slots[1]
        || slots.iter().any(|slot| *slot >= partition_sectors)
    {
        return Err(SuperblockError::BadSlotLayout);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordError {
    Empty,
    BadMagic,
    UnsupportedVersion,
    BadHeaderSize,
    BadFlags,
    BadCommitCookie,
    GenerationMismatch,
    ZeroFormatEpoch,
    FormatEpochMismatch,
    PayloadTooLarge,
    PayloadCrcMismatch,
    PayloadDigestMismatch,
    NonZeroPadding,
    RecordCrcMismatch,
}

impl RecordError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "persistent-data record slot is empty",
            Self::BadMagic => "persistent-data record magic is invalid",
            Self::UnsupportedVersion => "persistent-data record version is unsupported",
            Self::BadHeaderSize => "persistent-data record header size is invalid",
            Self::BadFlags => "persistent-data record commit flags are invalid",
            Self::BadCommitCookie => "persistent-data record commit cookie is invalid",
            Self::GenerationMismatch => "persistent-data record generation witness is invalid",
            Self::ZeroFormatEpoch => "persistent-data record format epoch is zero",
            Self::FormatEpochMismatch => "persistent-data record format epoch is foreign",
            Self::PayloadTooLarge => "persistent-data record payload is too large",
            Self::PayloadCrcMismatch => "persistent-data payload CRC32 is invalid",
            Self::PayloadDigestMismatch => "persistent-data payload digest is invalid",
            Self::NonZeroPadding => "persistent-data record padding is nonzero",
            Self::RecordCrcMismatch => "persistent-data record CRC32 is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataRecord<'a> {
    pub generation: u64,
    pub format_epoch: FormatEpoch,
    pub payload: &'a [u8],
}

impl<'a> DataRecord<'a> {
    pub fn decode(
        sector: &'a Sector,
        expected_format_epoch: FormatEpoch,
    ) -> Result<Self, RecordError> {
        validate_record_epoch(expected_format_epoch)?;
        if sector.iter().all(|byte| *byte == 0) {
            return Err(RecordError::Empty);
        }
        if sector[..8] != DATA_RECORD_MAGIC {
            return Err(RecordError::BadMagic);
        }
        if le_u32(sector, 8) != DATA_FORMAT_VERSION {
            return Err(RecordError::UnsupportedVersion);
        }
        if le_u32(sector, 12) != DATA_RECORD_HEADER_BYTES as u32 {
            return Err(RecordError::BadHeaderSize);
        }
        let generation = le_u64(sector, 16);
        let payload_len = le_u32(sector, 24) as usize;
        if payload_len > DATA_RECORD_MAX_PAYLOAD {
            return Err(RecordError::PayloadTooLarge);
        }
        if le_u32(sector, 28) != RECORD_COMMITTED {
            return Err(RecordError::BadFlags);
        }
        if le_u64(sector, 48) != !generation {
            return Err(RecordError::GenerationMismatch);
        }
        if le_u64(sector, 56) != RECORD_COMMIT_COOKIE {
            return Err(RecordError::BadCommitCookie);
        }
        if sector[36..40].iter().any(|byte| *byte != 0) {
            return Err(RecordError::NonZeroPadding);
        }
        let format_epoch: FormatEpoch = sector[64..64 + FORMAT_EPOCH_BYTES]
            .try_into()
            .expect("fixed format epoch range");
        validate_record_epoch(format_epoch)?;
        if format_epoch != expected_format_epoch {
            return Err(RecordError::FormatEpochMismatch);
        }
        let payload_end = DATA_RECORD_HEADER_BYTES + payload_len;
        let payload = &sector[DATA_RECORD_HEADER_BYTES..payload_end];
        if crc32(payload) != le_u32(sector, 32) {
            return Err(RecordError::PayloadCrcMismatch);
        }
        if fnv1a64(payload) != le_u64(sector, 40) {
            return Err(RecordError::PayloadDigestMismatch);
        }
        if sector[payload_end..DATA_RECORD_CRC_OFFSET]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(RecordError::NonZeroPadding);
        }
        if !sector_crc_is_valid(sector) {
            return Err(RecordError::RecordCrcMismatch);
        }
        Ok(Self {
            generation,
            format_epoch,
            payload,
        })
    }

    pub fn encode(
        format_epoch: FormatEpoch,
        generation: u64,
        payload: &[u8],
    ) -> Result<Sector, RecordError> {
        validate_record_epoch(format_epoch)?;
        if payload.len() > DATA_RECORD_MAX_PAYLOAD {
            return Err(RecordError::PayloadTooLarge);
        }
        let mut sector = [0_u8; SECTOR_SIZE];
        sector[..8].copy_from_slice(&DATA_RECORD_MAGIC);
        put_u32(&mut sector, 8, DATA_FORMAT_VERSION);
        put_u32(&mut sector, 12, DATA_RECORD_HEADER_BYTES as u32);
        put_u64(&mut sector, 16, generation);
        put_u32(&mut sector, 24, payload.len() as u32);
        put_u32(&mut sector, 28, RECORD_COMMITTED);
        put_u32(&mut sector, 32, crc32(payload));
        put_u64(&mut sector, 40, fnv1a64(payload));
        put_u64(&mut sector, 48, !generation);
        put_u64(&mut sector, 56, RECORD_COMMIT_COOKIE);
        sector[64..64 + FORMAT_EPOCH_BYTES].copy_from_slice(&format_epoch);
        sector[DATA_RECORD_HEADER_BYTES..DATA_RECORD_HEADER_BYTES + payload.len()]
            .copy_from_slice(payload);
        seal_sector(&mut sector);
        Ok(sector)
    }
}

fn validate_record_epoch(format_epoch: FormatEpoch) -> Result<(), RecordError> {
    if format_epoch.iter().all(|byte| *byte == 0) {
        Err(RecordError::ZeroFormatEpoch)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    ZeroFormatEpoch,
    NoValidRecord,
    AmbiguousGeneration,
    GenerationGap,
    GenerationExhausted,
}

impl RecoveryError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZeroFormatEpoch => "persistent-data recovery format epoch is zero",
            Self::NoValidRecord => "both persistent-data record slots are invalid",
            Self::AmbiguousGeneration => "persistent-data record slots have the same generation",
            Self::GenerationGap => "persistent-data record generations are not adjacent",
            Self::GenerationExhausted => "persistent-data generation counter is exhausted",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveredRecord<'a> {
    pub slot: usize,
    pub generation: u64,
    pub format_epoch: FormatEpoch,
    pub payload: &'a [u8],
    pub valid_slots: u8,
    pub rejected_slots: u8,
}

impl RecoveredRecord<'_> {
    pub const fn inactive_slot(self) -> usize {
        self.slot ^ 1
    }

    pub fn next_generation(self) -> Result<u64, RecoveryError> {
        self.generation
            .checked_add(1)
            .ok_or(RecoveryError::GenerationExhausted)
    }
}

pub fn recover<'a>(
    slots: [&'a Sector; DATA_SLOT_COUNT],
    expected_format_epoch: FormatEpoch,
) -> Result<RecoveredRecord<'a>, RecoveryError> {
    if expected_format_epoch.iter().all(|byte| *byte == 0) {
        return Err(RecoveryError::ZeroFormatEpoch);
    }
    let decoded = slots.map(|sector| DataRecord::decode(sector, expected_format_epoch));
    let valid_slots = decoded.iter().filter(|record| record.is_ok()).count() as u8;
    let rejected_slots = DATA_SLOT_COUNT as u8 - valid_slots;
    match (decoded[0], decoded[1]) {
        (Ok(first), Ok(second)) => {
            if first.generation == second.generation {
                return Err(RecoveryError::AmbiguousGeneration);
            }
            if first.generation.checked_add(1) != Some(second.generation)
                && second.generation.checked_add(1) != Some(first.generation)
            {
                return Err(RecoveryError::GenerationGap);
            }
            let (slot, record) = if first.generation > second.generation {
                (0, first)
            } else {
                (1, second)
            };
            Ok(RecoveredRecord {
                slot,
                generation: record.generation,
                format_epoch: record.format_epoch,
                payload: record.payload,
                valid_slots,
                rejected_slots,
            })
        }
        (Ok(record), Err(_)) => Ok(RecoveredRecord {
            slot: 0,
            generation: record.generation,
            format_epoch: record.format_epoch,
            payload: record.payload,
            valid_slots,
            rejected_slots,
        }),
        (Err(_), Ok(record)) => Ok(RecoveredRecord {
            slot: 1,
            generation: record.generation,
            format_epoch: record.format_epoch,
            payload: record.payload,
            valid_slots,
            rejected_slots,
        }),
        (Err(_), Err(_)) => Err(RecoveryError::NoValidRecord),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    NonZeroReserved,
    WitnessMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootState {
    pub boot_count: u64,
}

impl BootState {
    pub fn encode(self) -> [u8; BOOT_STATE_BYTES] {
        let mut bytes = [0_u8; BOOT_STATE_BYTES];
        bytes[..8].copy_from_slice(&BOOT_STATE_MAGIC);
        put_u32(&mut bytes, 8, DATA_FORMAT_VERSION);
        put_u64(&mut bytes, 16, self.boot_count);
        put_u64(&mut bytes, 24, self.boot_count ^ BOOT_STATE_WITNESS);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, BootStateError> {
        if bytes.len() != BOOT_STATE_BYTES {
            return Err(BootStateError::WrongSize);
        }
        if bytes[..8] != BOOT_STATE_MAGIC {
            return Err(BootStateError::BadMagic);
        }
        if le_u32(bytes, 8) != DATA_FORMAT_VERSION {
            return Err(BootStateError::UnsupportedVersion);
        }
        if le_u32(bytes, 12) != 0 {
            return Err(BootStateError::NonZeroReserved);
        }
        let boot_count = le_u64(bytes, 16);
        if le_u64(bytes, 24) != boot_count ^ BOOT_STATE_WITNESS {
            return Err(BootStateError::WitnessMismatch);
        }
        Ok(Self { boot_count })
    }
}

/// Stable, non-secret device contract recorded after a fresh kernel probe.
///
/// This is deliberately a contract fingerprint rather than a hardware
/// identity. A later mismatch requests another full boot probe; it is never
/// sufficient by itself to publish `Offline` or grant a userspace control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceHealthContract {
    pub capacity_sectors: u64,
    pub selected_features: u64,
    pub transport_base: u64,
    pub irq_id: u16,
    pub sector_bytes: u32,
    pub queue_size: u32,
    pub device_read_only: bool,
    pub flush_supported: bool,
}

impl DeviceHealthContract {
    fn validate(self) -> Result<(), DeviceHealthStateError> {
        if self.capacity_sectors == 0
            || self.selected_features == 0
            || self.transport_base == 0
            || self.irq_id == 0
            || self.sector_bytes == 0
            || self.queue_size == 0
        {
            return Err(DeviceHealthStateError::InvalidContract);
        }
        Ok(())
    }

    fn mode_flags(self) -> u32 {
        let read_only = if self.device_read_only {
            DEVICE_HEALTH_READ_ONLY
        } else {
            0
        };
        let flush_supported = if self.flush_supported {
            DEVICE_HEALTH_FLUSH_SUPPORTED
        } else {
            0
        };
        read_only | flush_supported
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceHealthStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    BadFlags,
    InvalidContract,
    ContractDigestMismatch,
    StateWitnessMismatch,
}

impl DeviceHealthStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "persistent device-health state has the wrong size",
            Self::BadMagic => "persistent device-health state magic is invalid",
            Self::UnsupportedVersion => "persistent device-health state version is unsupported",
            Self::BadFlags => "persistent device-health state flags are invalid",
            Self::InvalidContract => "persistent device-health contract is invalid",
            Self::ContractDigestMismatch => "persistent device-health contract digest is invalid",
            Self::StateWitnessMismatch => "persistent device-health generation witness is invalid",
        }
    }
}

/// M63's durable unclosed-boot hint.
///
/// `boot_open` means only that the previous kernel did not durably publish a
/// clean shutdown. It is not a persistent hardware-failure or Offline bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceHealthState {
    pub boot_count: u64,
    pub boot_open: bool,
    pub contract: DeviceHealthContract,
}

impl DeviceHealthState {
    pub fn encode(self) -> Result<[u8; DEVICE_HEALTH_STATE_BYTES], DeviceHealthStateError> {
        self.contract.validate()?;
        let mut bytes = [0_u8; DEVICE_HEALTH_STATE_BYTES];
        bytes[..8].copy_from_slice(&DEVICE_HEALTH_STATE_MAGIC);
        put_u32(&mut bytes, 8, DEVICE_HEALTH_STATE_VERSION);
        put_u32(
            &mut bytes,
            12,
            if self.boot_open {
                DEVICE_HEALTH_BOOT_OPEN
            } else {
                0
            },
        );
        put_u64(&mut bytes, 16, self.boot_count);
        put_u64(&mut bytes, 24, self.contract.capacity_sectors);
        put_u64(&mut bytes, 32, self.contract.selected_features);
        put_u64(&mut bytes, 40, self.contract.transport_base);
        put_u32(&mut bytes, 48, u32::from(self.contract.irq_id));
        put_u32(&mut bytes, 52, self.contract.sector_bytes);
        put_u32(&mut bytes, 56, self.contract.queue_size);
        put_u32(&mut bytes, 60, self.contract.mode_flags());
        let contract_digest = fnv1a64(&bytes[24..64]);
        put_u64(&mut bytes, 64, contract_digest);
        put_u64(
            &mut bytes,
            72,
            self.boot_count ^ contract_digest ^ DEVICE_HEALTH_WITNESS,
        );
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DeviceHealthStateError> {
        if bytes.len() != DEVICE_HEALTH_STATE_BYTES {
            return Err(DeviceHealthStateError::WrongSize);
        }
        if bytes[..8] != DEVICE_HEALTH_STATE_MAGIC {
            return Err(DeviceHealthStateError::BadMagic);
        }
        if le_u32(bytes, 8) != DEVICE_HEALTH_STATE_VERSION {
            return Err(DeviceHealthStateError::UnsupportedVersion);
        }
        let flags = le_u32(bytes, 12);
        if flags & !DEVICE_HEALTH_BOOT_OPEN != 0 {
            return Err(DeviceHealthStateError::BadFlags);
        }
        let irq_id = u16::try_from(le_u32(bytes, 48))
            .map_err(|_| DeviceHealthStateError::InvalidContract)?;
        let mode_flags = le_u32(bytes, 60);
        if mode_flags & !DEVICE_HEALTH_MODE_FLAGS != 0 {
            return Err(DeviceHealthStateError::BadFlags);
        }
        let contract = DeviceHealthContract {
            capacity_sectors: le_u64(bytes, 24),
            selected_features: le_u64(bytes, 32),
            transport_base: le_u64(bytes, 40),
            irq_id,
            sector_bytes: le_u32(bytes, 52),
            queue_size: le_u32(bytes, 56),
            device_read_only: mode_flags & DEVICE_HEALTH_READ_ONLY != 0,
            flush_supported: mode_flags & DEVICE_HEALTH_FLUSH_SUPPORTED != 0,
        };
        contract.validate()?;
        let contract_digest = le_u64(bytes, 64);
        if contract_digest != fnv1a64(&bytes[24..64]) {
            return Err(DeviceHealthStateError::ContractDigestMismatch);
        }
        let boot_count = le_u64(bytes, 16);
        if le_u64(bytes, 72) != boot_count ^ contract_digest ^ DEVICE_HEALTH_WITNESS {
            return Err(DeviceHealthStateError::StateWitnessMismatch);
        }
        Ok(Self {
            boot_count,
            boot_open: flags & DEVICE_HEALTH_BOOT_OPEN != 0,
            contract,
        })
    }
}

/// Public-key namespace bound to one persistent manifest rollback ledger.
///
/// The digest is the SHA-256 of the pinned RSA modulus. It is not secret and
/// does not authenticate writable storage; it only prevents a record from one
/// compiled trust-anchor namespace being silently reused by another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestRollbackBinding {
    pub key_id: u8,
    pub trust_anchor_sha256: [u8; 32],
}

impl ManifestRollbackBinding {
    fn validate(self) -> Result<(), ManifestRollbackStateError> {
        if self.key_id == 0 {
            return Err(ManifestRollbackStateError::InvalidKeyId);
        }
        if self.trust_anchor_sha256.iter().all(|byte| *byte == 0) {
            return Err(ManifestRollbackStateError::ZeroTrustAnchor);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestRollbackStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroFloor,
    InvalidKeyId,
    ZeroTrustAnchor,
    NonZeroReserved,
    WitnessMismatch,
}

impl ManifestRollbackStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "persistent manifest rollback state has the wrong size",
            Self::BadMagic => "persistent manifest rollback state magic is invalid",
            Self::UnsupportedVersion => "persistent manifest rollback state version is unsupported",
            Self::ZeroFloor => "persistent manifest rollback floor is zero",
            Self::InvalidKeyId => "persistent manifest rollback key id is invalid",
            Self::ZeroTrustAnchor => "persistent manifest rollback trust anchor is zero",
            Self::NonZeroReserved => "persistent manifest rollback reserved bytes are nonzero",
            Self::WitnessMismatch => "persistent manifest rollback witness is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestRollbackState {
    pub floor: u32,
    pub binding: ManifestRollbackBinding,
}

impl ManifestRollbackState {
    pub fn encode(self) -> Result<[u8; MANIFEST_ROLLBACK_STATE_BYTES], ManifestRollbackStateError> {
        if self.floor == 0 {
            return Err(ManifestRollbackStateError::ZeroFloor);
        }
        self.binding.validate()?;
        let mut bytes = [0_u8; MANIFEST_ROLLBACK_STATE_BYTES];
        bytes[..8].copy_from_slice(&MANIFEST_ROLLBACK_STATE_MAGIC);
        put_u32(&mut bytes, 8, MANIFEST_ROLLBACK_STATE_VERSION);
        put_u32(&mut bytes, 12, self.floor);
        put_u32(&mut bytes, 16, u32::from(self.binding.key_id));
        bytes[24..56].copy_from_slice(&self.binding.trust_anchor_sha256);
        let witness = fnv1a64(&bytes[12..56]) ^ MANIFEST_ROLLBACK_WITNESS;
        put_u64(&mut bytes, 56, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ManifestRollbackStateError> {
        if bytes.len() != MANIFEST_ROLLBACK_STATE_BYTES {
            return Err(ManifestRollbackStateError::WrongSize);
        }
        if bytes[..8] != MANIFEST_ROLLBACK_STATE_MAGIC {
            return Err(ManifestRollbackStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MANIFEST_ROLLBACK_STATE_VERSION {
            return Err(ManifestRollbackStateError::UnsupportedVersion);
        }
        let floor = le_u32(bytes, 12);
        if floor == 0 {
            return Err(ManifestRollbackStateError::ZeroFloor);
        }
        let key_id = u8::try_from(le_u32(bytes, 16))
            .map_err(|_| ManifestRollbackStateError::InvalidKeyId)?;
        if bytes[20..24].iter().any(|byte| *byte != 0) {
            return Err(ManifestRollbackStateError::NonZeroReserved);
        }
        let trust_anchor_sha256 = bytes[24..56]
            .try_into()
            .expect("fixed manifest trust-anchor range");
        let binding = ManifestRollbackBinding {
            key_id,
            trust_anchor_sha256,
        };
        binding.validate()?;
        if le_u64(bytes, 56) != fnv1a64(&bytes[12..56]) ^ MANIFEST_ROLLBACK_WITNESS {
            return Err(ManifestRollbackStateError::WitnessMismatch);
        }
        Ok(Self { floor, binding })
    }
}

/// Active public-key epoch and compiled keyring namespace persisted by M76.
///
/// Public-key material is not secret. The two digests bind writable state to
/// one reviewed keyring order and one active trust anchor, while `key_epoch`
/// makes reverse transitions fail closed independently of manifest version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestKeyPolicyBinding {
    pub key_id: u8,
    pub key_epoch: u32,
    pub trust_anchor_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
}

impl ManifestKeyPolicyBinding {
    fn validate(self) -> Result<(), ManifestKeyPolicyStateError> {
        if self.key_id == 0 {
            return Err(ManifestKeyPolicyStateError::InvalidKeyId);
        }
        if self.key_epoch == 0 {
            return Err(ManifestKeyPolicyStateError::ZeroKeyEpoch);
        }
        if self.trust_anchor_sha256.iter().all(|byte| *byte == 0) {
            return Err(ManifestKeyPolicyStateError::ZeroTrustAnchor);
        }
        if self.policy_sha256.iter().all(|byte| *byte == 0) {
            return Err(ManifestKeyPolicyStateError::ZeroPolicyDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestKeyPolicyStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroFloor,
    InvalidKeyId,
    ZeroKeyEpoch,
    ZeroTrustAnchor,
    ZeroPolicyDigest,
    NonZeroReserved,
    WitnessMismatch,
}

impl ManifestKeyPolicyStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "persistent manifest key-policy state has the wrong size",
            Self::BadMagic => "persistent manifest key-policy magic is invalid",
            Self::UnsupportedVersion => "persistent manifest key-policy version is unsupported",
            Self::ZeroFloor => "persistent manifest key-policy floor is zero",
            Self::InvalidKeyId => "persistent manifest key-policy key id is invalid",
            Self::ZeroKeyEpoch => "persistent manifest key-policy epoch is zero",
            Self::ZeroTrustAnchor => "persistent manifest key-policy trust anchor is zero",
            Self::ZeroPolicyDigest => "persistent manifest keyring digest is zero",
            Self::NonZeroReserved => "persistent manifest key-policy reserved bytes are nonzero",
            Self::WitnessMismatch => "persistent manifest key-policy witness is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestKeyPolicyState {
    pub floor: u32,
    pub binding: ManifestKeyPolicyBinding,
}

impl ManifestKeyPolicyState {
    pub fn encode(
        self,
    ) -> Result<[u8; MANIFEST_KEY_POLICY_STATE_BYTES], ManifestKeyPolicyStateError> {
        if self.floor == 0 {
            return Err(ManifestKeyPolicyStateError::ZeroFloor);
        }
        self.binding.validate()?;
        let mut bytes = [0_u8; MANIFEST_KEY_POLICY_STATE_BYTES];
        bytes[..8].copy_from_slice(&MANIFEST_KEY_POLICY_STATE_MAGIC);
        put_u32(&mut bytes, 8, MANIFEST_KEY_POLICY_STATE_VERSION);
        put_u32(&mut bytes, 12, self.floor);
        put_u32(&mut bytes, 16, self.binding.key_epoch);
        put_u32(&mut bytes, 20, u32::from(self.binding.key_id));
        bytes[24..56].copy_from_slice(&self.binding.trust_anchor_sha256);
        bytes[56..88].copy_from_slice(&self.binding.policy_sha256);
        let witness = fnv1a64(&bytes[12..88]) ^ MANIFEST_KEY_POLICY_WITNESS;
        put_u64(&mut bytes, 88, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ManifestKeyPolicyStateError> {
        if bytes.len() != MANIFEST_KEY_POLICY_STATE_BYTES {
            return Err(ManifestKeyPolicyStateError::WrongSize);
        }
        if bytes[..8] != MANIFEST_KEY_POLICY_STATE_MAGIC {
            return Err(ManifestKeyPolicyStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MANIFEST_KEY_POLICY_STATE_VERSION {
            return Err(ManifestKeyPolicyStateError::UnsupportedVersion);
        }
        let floor = le_u32(bytes, 12);
        if floor == 0 {
            return Err(ManifestKeyPolicyStateError::ZeroFloor);
        }
        let key_epoch = le_u32(bytes, 16);
        let key_id = u8::try_from(le_u32(bytes, 20))
            .map_err(|_| ManifestKeyPolicyStateError::InvalidKeyId)?;
        let binding = ManifestKeyPolicyBinding {
            key_id,
            key_epoch,
            trust_anchor_sha256: bytes[24..56]
                .try_into()
                .expect("fixed active trust-anchor range"),
            policy_sha256: bytes[56..88]
                .try_into()
                .expect("fixed manifest key-policy range"),
        };
        binding.validate()?;
        if le_u64(bytes, 88) != fnv1a64(&bytes[12..88]) ^ MANIFEST_KEY_POLICY_WITNESS {
            return Err(ManifestKeyPolicyStateError::WitnessMismatch);
        }
        Ok(Self { floor, binding })
    }
}

/// Immutable inputs authenticated by one BMA1 authorization.
///
/// The writable audit state never substitutes for signature verification. It
/// only binds an already-verified authorization to one root, policy, device
/// namespace, product manifest, operation set, and monotonic sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuthorizationAuditBinding {
    pub sequence: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
}

impl MaintenanceAuthorizationAuditBinding {
    fn validate(self) -> Result<(), MaintenanceAuditStateError> {
        if self.sequence == 0 {
            return Err(MaintenanceAuditStateError::ZeroSequence);
        }
        if self.operations == 0 {
            return Err(MaintenanceAuditStateError::ZeroOperations);
        }
        if self.max_uses == 0 {
            return Err(MaintenanceAuditStateError::ZeroUseLimit);
        }
        for (digest, error) in [
            (
                self.authorization_sha256,
                MaintenanceAuditStateError::ZeroAuthorizationDigest,
            ),
            (
                self.authorization_id,
                MaintenanceAuditStateError::ZeroAuthorizationId,
            ),
            (
                self.manifest_sha256,
                MaintenanceAuditStateError::ZeroManifestDigest,
            ),
            (
                self.policy_sha256,
                MaintenanceAuditStateError::ZeroPolicyDigest,
            ),
            (self.root_sha256, MaintenanceAuditStateError::ZeroRootDigest),
            (
                self.device_binding_sha256,
                MaintenanceAuditStateError::ZeroDeviceBinding,
            ),
        ] {
            if digest.iter().all(|byte| *byte == 0) {
                return Err(error);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceAuditStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroSequence,
    ZeroAcceptedCount,
    CountMismatch,
    ZeroOperations,
    ZeroUseLimit,
    ZeroAuthorizationDigest,
    ZeroAuthorizationId,
    ZeroManifestDigest,
    ZeroPolicyDigest,
    ZeroRootDigest,
    ZeroDeviceBinding,
    ZeroChainDigest,
    NonZeroReserved,
    ChainMismatch,
    WitnessMismatch,
}

impl MaintenanceAuditStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "maintenance audit state has the wrong size",
            Self::BadMagic => "maintenance audit state magic is invalid",
            Self::UnsupportedVersion => "maintenance audit state version is unsupported",
            Self::ZeroSequence => "maintenance audit sequence is zero",
            Self::ZeroAcceptedCount => "maintenance accepted count is zero",
            Self::CountMismatch => "maintenance accepted count does not match the sequence",
            Self::ZeroOperations => "maintenance operation mask is zero",
            Self::ZeroUseLimit => "maintenance use limit is zero",
            Self::ZeroAuthorizationDigest => "maintenance authorization digest is zero",
            Self::ZeroAuthorizationId => "maintenance authorization id is zero",
            Self::ZeroManifestDigest => "maintenance manifest digest is zero",
            Self::ZeroPolicyDigest => "maintenance policy digest is zero",
            Self::ZeroRootDigest => "maintenance root digest is zero",
            Self::ZeroDeviceBinding => "maintenance device binding is zero",
            Self::ZeroChainDigest => "maintenance audit-chain digest is zero",
            Self::NonZeroReserved => "maintenance audit reserved bytes are nonzero",
            Self::ChainMismatch => "maintenance audit-chain digest is invalid",
            Self::WitnessMismatch => "maintenance audit witness is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuditState {
    pub sequence: u64,
    pub accepted_count: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub previous_chain_sha256: [u8; 32],
    pub chain_sha256: [u8; 32],
}

impl MaintenanceAuditState {
    pub fn encode(self) -> Result<[u8; MAINTENANCE_AUDIT_STATE_BYTES], MaintenanceAuditStateError> {
        self.validate()?;
        let mut bytes = [0_u8; MAINTENANCE_AUDIT_STATE_BYTES];
        bytes[..8].copy_from_slice(&MAINTENANCE_AUDIT_STATE_MAGIC);
        put_u32(&mut bytes, 8, MAINTENANCE_AUDIT_STATE_VERSION);
        put_u64(&mut bytes, 16, self.sequence);
        put_u64(&mut bytes, 24, self.accepted_count);
        put_u64(&mut bytes, 32, self.operations);
        put_u32(&mut bytes, 40, self.max_uses);
        bytes[48..80].copy_from_slice(&self.authorization_sha256);
        bytes[80..112].copy_from_slice(&self.authorization_id);
        bytes[112..144].copy_from_slice(&self.manifest_sha256);
        bytes[144..176].copy_from_slice(&self.policy_sha256);
        bytes[176..208].copy_from_slice(&self.root_sha256);
        bytes[208..240].copy_from_slice(&self.device_binding_sha256);
        bytes[240..272].copy_from_slice(&self.previous_chain_sha256);
        bytes[272..304].copy_from_slice(&self.chain_sha256);
        let witness = fnv1a64(&bytes[16..304]) ^ MAINTENANCE_AUDIT_WITNESS;
        put_u64(&mut bytes, 304, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MaintenanceAuditStateError> {
        if bytes.len() != MAINTENANCE_AUDIT_STATE_BYTES {
            return Err(MaintenanceAuditStateError::WrongSize);
        }
        if bytes[..8] != MAINTENANCE_AUDIT_STATE_MAGIC {
            return Err(MaintenanceAuditStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MAINTENANCE_AUDIT_STATE_VERSION {
            return Err(MaintenanceAuditStateError::UnsupportedVersion);
        }
        if bytes[12..16]
            .iter()
            .chain(bytes[44..48].iter())
            .chain(bytes[312..320].iter())
            .any(|byte| *byte != 0)
        {
            return Err(MaintenanceAuditStateError::NonZeroReserved);
        }
        let state = Self {
            sequence: le_u64(bytes, 16),
            accepted_count: le_u64(bytes, 24),
            operations: le_u64(bytes, 32),
            max_uses: le_u32(bytes, 40),
            authorization_sha256: bytes[48..80]
                .try_into()
                .expect("fixed maintenance authorization digest range"),
            authorization_id: bytes[80..112]
                .try_into()
                .expect("fixed maintenance authorization id range"),
            manifest_sha256: bytes[112..144]
                .try_into()
                .expect("fixed maintenance manifest digest range"),
            policy_sha256: bytes[144..176]
                .try_into()
                .expect("fixed maintenance policy digest range"),
            root_sha256: bytes[176..208]
                .try_into()
                .expect("fixed maintenance root digest range"),
            device_binding_sha256: bytes[208..240]
                .try_into()
                .expect("fixed maintenance device-binding range"),
            previous_chain_sha256: bytes[240..272]
                .try_into()
                .expect("fixed previous maintenance chain range"),
            chain_sha256: bytes[272..304]
                .try_into()
                .expect("fixed maintenance chain range"),
        };
        state.validate()?;
        if le_u64(bytes, 304) != fnv1a64(&bytes[16..304]) ^ MAINTENANCE_AUDIT_WITNESS {
            return Err(MaintenanceAuditStateError::WitnessMismatch);
        }
        Ok(state)
    }

    fn validate(self) -> Result<(), MaintenanceAuditStateError> {
        let binding = self.binding();
        binding.validate()?;
        if self.accepted_count == 0 {
            return Err(MaintenanceAuditStateError::ZeroAcceptedCount);
        }
        if self.accepted_count != self.sequence {
            return Err(MaintenanceAuditStateError::CountMismatch);
        }
        if self.chain_sha256.iter().all(|byte| *byte == 0) {
            return Err(MaintenanceAuditStateError::ZeroChainDigest);
        }
        if self.chain_sha256 != maintenance_audit_chain_sha256(self.previous_chain_sha256, binding)
        {
            return Err(MaintenanceAuditStateError::ChainMismatch);
        }
        Ok(())
    }

    fn binding(self) -> MaintenanceAuthorizationAuditBinding {
        MaintenanceAuthorizationAuditBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
        }
    }
}

pub fn maintenance_audit_chain_sha256(
    previous_chain_sha256: [u8; 32],
    binding: MaintenanceAuthorizationAuditBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 156];
    material[..8].copy_from_slice(b"BNDRMAU1");
    material[8..40].copy_from_slice(&previous_chain_sha256);
    material[40..72].copy_from_slice(&binding.authorization_sha256);
    material[72..104].copy_from_slice(&binding.authorization_id);
    material[104..112].copy_from_slice(&binding.sequence.to_le_bytes());
    material[112..120].copy_from_slice(&binding.operations.to_le_bytes());
    material[120..124].copy_from_slice(&binding.max_uses.to_le_bytes());
    material[124..156].copy_from_slice(&binding.manifest_sha256);
    sha256(&material)
}

/// Kernel-validated facts sealed when one maintenance execution completes.
///
/// The authorization fields must exactly match the already committed BMA1
/// audit head. The runtime fields are supplied only after the kernel has
/// validated the two bounded rotations, service drain, and clean-shutdown
/// boundary. This record supports a narrowly bounded retry of an interrupted
/// QEMU maintenance session; it is not a general exactly-once primitive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceExecutionCompletionBinding {
    pub sequence: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub uses_consumed: u32,
    pub rotations_completed: u32,
    pub flags: u32,
    pub appdata_generation: u64,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub runtime_evidence_sha256: [u8; 32],
}

impl MaintenanceExecutionCompletionBinding {
    fn validate(self) -> Result<(), MaintenanceExecutionStateError> {
        if self.sequence == 0 {
            return Err(MaintenanceExecutionStateError::ZeroSequence);
        }
        if self.operations == 0 {
            return Err(MaintenanceExecutionStateError::ZeroOperations);
        }
        if self.max_uses == 0 {
            return Err(MaintenanceExecutionStateError::ZeroUseLimit);
        }
        if self.uses_consumed != self.max_uses {
            return Err(MaintenanceExecutionStateError::UseCountMismatch);
        }
        if self.rotations_completed != self.uses_consumed {
            return Err(MaintenanceExecutionStateError::RotationCountMismatch);
        }
        if self.flags != MAINTENANCE_EXECUTION_REQUIRED_FLAGS {
            return Err(MaintenanceExecutionStateError::InvalidFlags);
        }
        if self.appdata_generation == 0 {
            return Err(MaintenanceExecutionStateError::ZeroAppdataGeneration);
        }
        for (digest, error) in [
            (
                self.authorization_sha256,
                MaintenanceExecutionStateError::ZeroAuthorizationDigest,
            ),
            (
                self.authorization_id,
                MaintenanceExecutionStateError::ZeroAuthorizationId,
            ),
            (
                self.manifest_sha256,
                MaintenanceExecutionStateError::ZeroManifestDigest,
            ),
            (
                self.policy_sha256,
                MaintenanceExecutionStateError::ZeroPolicyDigest,
            ),
            (
                self.root_sha256,
                MaintenanceExecutionStateError::ZeroRootDigest,
            ),
            (
                self.device_binding_sha256,
                MaintenanceExecutionStateError::ZeroDeviceBinding,
            ),
            (
                self.runtime_evidence_sha256,
                MaintenanceExecutionStateError::ZeroRuntimeEvidenceDigest,
            ),
        ] {
            if digest.iter().all(|byte| *byte == 0) {
                return Err(error);
            }
        }
        Ok(())
    }

    fn authorization_binding(self) -> MaintenanceAuthorizationAuditBinding {
        MaintenanceAuthorizationAuditBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceExecutionStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroSequence,
    ZeroCompletedCount,
    CountMismatch,
    ZeroOperations,
    ZeroUseLimit,
    UseCountMismatch,
    RotationCountMismatch,
    InvalidFlags,
    ZeroAppdataGeneration,
    ZeroAuthorizationDigest,
    ZeroAuthorizationId,
    ZeroManifestDigest,
    ZeroPolicyDigest,
    ZeroRootDigest,
    ZeroDeviceBinding,
    ZeroRuntimeEvidenceDigest,
    ZeroChainDigest,
    NonZeroReserved,
    ChainMismatch,
    WitnessMismatch,
}

impl MaintenanceExecutionStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "maintenance execution state has the wrong size",
            Self::BadMagic => "maintenance execution state magic is invalid",
            Self::UnsupportedVersion => "maintenance execution state version is unsupported",
            Self::ZeroSequence => "maintenance execution sequence is zero",
            Self::ZeroCompletedCount => "maintenance completed count is zero",
            Self::CountMismatch => {
                "maintenance completed count does not match the execution sequence"
            }
            Self::ZeroOperations => "maintenance execution operation mask is zero",
            Self::ZeroUseLimit => "maintenance execution use limit is zero",
            Self::UseCountMismatch => {
                "maintenance execution did not consume its exact bounded use count"
            }
            Self::RotationCountMismatch => {
                "maintenance execution rotations do not match consumed uses"
            }
            Self::InvalidFlags => {
                "maintenance execution lacks the exact drain and clean-shutdown flags"
            }
            Self::ZeroAppdataGeneration => "maintenance execution app-data generation is zero",
            Self::ZeroAuthorizationDigest => "maintenance execution authorization digest is zero",
            Self::ZeroAuthorizationId => "maintenance execution authorization id is zero",
            Self::ZeroManifestDigest => "maintenance execution manifest digest is zero",
            Self::ZeroPolicyDigest => "maintenance execution policy digest is zero",
            Self::ZeroRootDigest => "maintenance execution root digest is zero",
            Self::ZeroDeviceBinding => "maintenance execution device binding is zero",
            Self::ZeroRuntimeEvidenceDigest => {
                "maintenance execution runtime-evidence digest is zero"
            }
            Self::ZeroChainDigest => "maintenance execution-chain digest is zero",
            Self::NonZeroReserved => "maintenance execution reserved bytes are nonzero",
            Self::ChainMismatch => "maintenance execution-chain digest is invalid",
            Self::WitnessMismatch => "maintenance execution witness is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceExecutionState {
    pub sequence: u64,
    pub completed_count: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub uses_consumed: u32,
    pub rotations_completed: u32,
    pub flags: u32,
    pub appdata_generation: u64,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub previous_chain_sha256: [u8; 32],
    pub chain_sha256: [u8; 32],
    pub runtime_evidence_sha256: [u8; 32],
}

impl MaintenanceExecutionState {
    pub fn encode(
        self,
    ) -> Result<[u8; MAINTENANCE_EXECUTION_STATE_BYTES], MaintenanceExecutionStateError> {
        self.validate()?;
        let mut bytes = [0_u8; MAINTENANCE_EXECUTION_STATE_BYTES];
        bytes[..8].copy_from_slice(&MAINTENANCE_EXECUTION_STATE_MAGIC);
        put_u32(&mut bytes, 8, MAINTENANCE_EXECUTION_STATE_VERSION);
        put_u64(&mut bytes, 16, self.sequence);
        put_u64(&mut bytes, 24, self.completed_count);
        put_u64(&mut bytes, 32, self.operations);
        put_u32(&mut bytes, 40, self.max_uses);
        put_u32(&mut bytes, 44, self.uses_consumed);
        put_u32(&mut bytes, 48, self.rotations_completed);
        put_u32(&mut bytes, 52, self.flags);
        put_u64(&mut bytes, 56, self.appdata_generation);
        bytes[64..96].copy_from_slice(&self.authorization_sha256);
        bytes[96..128].copy_from_slice(&self.authorization_id);
        bytes[128..160].copy_from_slice(&self.manifest_sha256);
        bytes[160..192].copy_from_slice(&self.policy_sha256);
        bytes[192..224].copy_from_slice(&self.root_sha256);
        bytes[224..256].copy_from_slice(&self.device_binding_sha256);
        bytes[256..288].copy_from_slice(&self.previous_chain_sha256);
        bytes[288..320].copy_from_slice(&self.chain_sha256);
        bytes[320..352].copy_from_slice(&self.runtime_evidence_sha256);
        let witness = fnv1a64(&bytes[16..352]) ^ MAINTENANCE_EXECUTION_WITNESS;
        put_u64(&mut bytes, 352, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MaintenanceExecutionStateError> {
        if bytes.len() != MAINTENANCE_EXECUTION_STATE_BYTES {
            return Err(MaintenanceExecutionStateError::WrongSize);
        }
        if bytes[..8] != MAINTENANCE_EXECUTION_STATE_MAGIC {
            return Err(MaintenanceExecutionStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MAINTENANCE_EXECUTION_STATE_VERSION {
            return Err(MaintenanceExecutionStateError::UnsupportedVersion);
        }
        if bytes[12..16]
            .iter()
            .chain(bytes[360..368].iter())
            .any(|byte| *byte != 0)
        {
            return Err(MaintenanceExecutionStateError::NonZeroReserved);
        }
        let state = Self {
            sequence: le_u64(bytes, 16),
            completed_count: le_u64(bytes, 24),
            operations: le_u64(bytes, 32),
            max_uses: le_u32(bytes, 40),
            uses_consumed: le_u32(bytes, 44),
            rotations_completed: le_u32(bytes, 48),
            flags: le_u32(bytes, 52),
            appdata_generation: le_u64(bytes, 56),
            authorization_sha256: bytes[64..96]
                .try_into()
                .expect("fixed maintenance execution authorization digest range"),
            authorization_id: bytes[96..128]
                .try_into()
                .expect("fixed maintenance execution authorization id range"),
            manifest_sha256: bytes[128..160]
                .try_into()
                .expect("fixed maintenance execution manifest digest range"),
            policy_sha256: bytes[160..192]
                .try_into()
                .expect("fixed maintenance execution policy digest range"),
            root_sha256: bytes[192..224]
                .try_into()
                .expect("fixed maintenance execution root digest range"),
            device_binding_sha256: bytes[224..256]
                .try_into()
                .expect("fixed maintenance execution device-binding range"),
            previous_chain_sha256: bytes[256..288]
                .try_into()
                .expect("fixed previous maintenance execution chain range"),
            chain_sha256: bytes[288..320]
                .try_into()
                .expect("fixed maintenance execution chain range"),
            runtime_evidence_sha256: bytes[320..352]
                .try_into()
                .expect("fixed maintenance runtime-evidence digest range"),
        };
        state.validate()?;
        if le_u64(bytes, 352) != fnv1a64(&bytes[16..352]) ^ MAINTENANCE_EXECUTION_WITNESS {
            return Err(MaintenanceExecutionStateError::WitnessMismatch);
        }
        Ok(state)
    }

    fn completion_binding(self) -> MaintenanceExecutionCompletionBinding {
        MaintenanceExecutionCompletionBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            uses_consumed: self.uses_consumed,
            rotations_completed: self.rotations_completed,
            flags: self.flags,
            appdata_generation: self.appdata_generation,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
            runtime_evidence_sha256: self.runtime_evidence_sha256,
        }
    }

    fn validate(self) -> Result<(), MaintenanceExecutionStateError> {
        let binding = self.completion_binding();
        binding.validate()?;
        if self.completed_count == 0 {
            return Err(MaintenanceExecutionStateError::ZeroCompletedCount);
        }
        if self.completed_count != self.sequence {
            return Err(MaintenanceExecutionStateError::CountMismatch);
        }
        if self.chain_sha256.iter().all(|byte| *byte == 0) {
            return Err(MaintenanceExecutionStateError::ZeroChainDigest);
        }
        if self.chain_sha256
            != maintenance_execution_chain_sha256(self.previous_chain_sha256, binding)
        {
            return Err(MaintenanceExecutionStateError::ChainMismatch);
        }
        Ok(())
    }
}

pub fn maintenance_execution_chain_sha256(
    previous_chain_sha256: [u8; 32],
    binding: MaintenanceExecutionCompletionBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 304];
    material[..8].copy_from_slice(b"BNDRMEX1");
    material[8..40].copy_from_slice(&previous_chain_sha256);
    material[40..72].copy_from_slice(&binding.authorization_sha256);
    material[72..104].copy_from_slice(&binding.authorization_id);
    material[104..112].copy_from_slice(&binding.sequence.to_le_bytes());
    material[112..120].copy_from_slice(&binding.operations.to_le_bytes());
    material[120..124].copy_from_slice(&binding.max_uses.to_le_bytes());
    material[124..128].copy_from_slice(&binding.uses_consumed.to_le_bytes());
    material[128..132].copy_from_slice(&binding.rotations_completed.to_le_bytes());
    material[132..136].copy_from_slice(&binding.flags.to_le_bytes());
    material[136..144].copy_from_slice(&binding.appdata_generation.to_le_bytes());
    material[144..176].copy_from_slice(&binding.runtime_evidence_sha256);
    material[176..208].copy_from_slice(&binding.manifest_sha256);
    material[208..240].copy_from_slice(&binding.policy_sha256);
    material[240..272].copy_from_slice(&binding.root_sha256);
    material[272..304].copy_from_slice(&binding.device_binding_sha256);
    sha256(&material)
}

/// One kernel-observed step in the fixed M79 maintenance program.
///
/// The program is deliberately narrow: two restart-based StorageServer
/// convergence steps followed by one resident drain step. Each boot starts
/// from the same bounded resident topology, so a previously committed step may
/// be replayed from step one without another journal mutation. This is an
/// explicit idempotent reconciliation rule, not arbitrary instruction resume
/// or a claim that unrelated external effects are exactly-once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepCommitBinding {
    pub sequence: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub step_ordinal: u32,
    pub observed_rotations: u32,
    pub flags: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub effect_sha256: [u8; 32],
}

impl MaintenanceStepCommitBinding {
    pub fn for_observation(
        authorization: MaintenanceAuthorizationAuditBinding,
        step_ordinal: u32,
        observed_rotations: u32,
        drain_validated: bool,
    ) -> Result<Self, MaintenanceStepStateError> {
        let flags = MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START
            | if drain_validated {
                MAINTENANCE_STEP_DRAIN_VALIDATED
            } else {
                0
            };
        let mut binding = Self {
            sequence: authorization.sequence,
            operations: authorization.operations,
            max_uses: authorization.max_uses,
            step_ordinal,
            observed_rotations,
            flags,
            authorization_sha256: authorization.authorization_sha256,
            authorization_id: authorization.authorization_id,
            manifest_sha256: authorization.manifest_sha256,
            policy_sha256: authorization.policy_sha256,
            root_sha256: authorization.root_sha256,
            device_binding_sha256: authorization.device_binding_sha256,
            effect_sha256: [0; 32],
        };
        binding.effect_sha256 = maintenance_step_effect_sha256(binding);
        binding.validate()?;
        Ok(binding)
    }

    fn validate(self) -> Result<(), MaintenanceStepStateError> {
        if self.sequence == 0 {
            return Err(MaintenanceStepStateError::ZeroSequence);
        }
        if self.operations != MAINTENANCE_STEP_OPERATION_STORAGE_ROTATION {
            return Err(MaintenanceStepStateError::UnsupportedOperations);
        }
        if self.max_uses != 2 {
            return Err(MaintenanceStepStateError::UnexpectedUseLimit);
        }
        let expected = match self.step_ordinal {
            MAINTENANCE_STEP_ROTATION_ONE => (1, MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START),
            MAINTENANCE_STEP_ROTATION_TWO => (2, MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START),
            MAINTENANCE_STEP_DRAIN => (
                2,
                MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START | MAINTENANCE_STEP_DRAIN_VALIDATED,
            ),
            _ => return Err(MaintenanceStepStateError::InvalidStep),
        };
        if self.observed_rotations != expected.0 {
            return Err(MaintenanceStepStateError::ObservationMismatch);
        }
        if self.flags != expected.1 {
            return Err(MaintenanceStepStateError::InvalidFlags);
        }
        for (digest, error) in [
            (
                self.authorization_sha256,
                MaintenanceStepStateError::ZeroAuthorizationDigest,
            ),
            (
                self.authorization_id,
                MaintenanceStepStateError::ZeroAuthorizationId,
            ),
            (
                self.manifest_sha256,
                MaintenanceStepStateError::ZeroManifestDigest,
            ),
            (
                self.policy_sha256,
                MaintenanceStepStateError::ZeroPolicyDigest,
            ),
            (self.root_sha256, MaintenanceStepStateError::ZeroRootDigest),
            (
                self.device_binding_sha256,
                MaintenanceStepStateError::ZeroDeviceBinding,
            ),
            (
                self.effect_sha256,
                MaintenanceStepStateError::ZeroEffectDigest,
            ),
        ] {
            if digest.iter().all(|byte| *byte == 0) {
                return Err(error);
            }
        }
        if self.effect_sha256 != maintenance_step_effect_sha256(self) {
            return Err(MaintenanceStepStateError::EffectMismatch);
        }
        Ok(())
    }

    fn authorization_binding(self) -> MaintenanceAuthorizationAuditBinding {
        MaintenanceAuthorizationAuditBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
        }
    }
}

pub fn maintenance_step_effect_sha256(binding: MaintenanceStepCommitBinding) -> [u8; 32] {
    let mut material = [0_u8; 232];
    material[..8].copy_from_slice(b"BNDRMST1");
    material[8..40].copy_from_slice(&binding.authorization_sha256);
    material[40..72].copy_from_slice(&binding.authorization_id);
    material[72..80].copy_from_slice(&binding.sequence.to_le_bytes());
    material[80..88].copy_from_slice(&binding.operations.to_le_bytes());
    material[88..92].copy_from_slice(&binding.max_uses.to_le_bytes());
    material[92..96].copy_from_slice(&binding.step_ordinal.to_le_bytes());
    material[96..100].copy_from_slice(&binding.observed_rotations.to_le_bytes());
    material[100..104].copy_from_slice(&binding.flags.to_le_bytes());
    material[104..136].copy_from_slice(&binding.manifest_sha256);
    material[136..168].copy_from_slice(&binding.policy_sha256);
    material[168..200].copy_from_slice(&binding.root_sha256);
    material[200..232].copy_from_slice(&binding.device_binding_sha256);
    sha256(&material)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceStepStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroSequence,
    InvalidBaseSequence,
    ZeroEntryCount,
    EntryCountMismatch,
    UnsupportedOperations,
    UnexpectedUseLimit,
    InvalidStep,
    ObservationMismatch,
    InvalidFlags,
    ZeroAuthorizationDigest,
    ZeroAuthorizationId,
    ZeroManifestDigest,
    ZeroPolicyDigest,
    ZeroRootDigest,
    ZeroDeviceBinding,
    ZeroEffectDigest,
    EffectMismatch,
    ZeroChainDigest,
    NonZeroReserved,
    ChainMismatch,
    WitnessMismatch,
}

impl MaintenanceStepStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "maintenance step state has the wrong size",
            Self::BadMagic => "maintenance step state magic is invalid",
            Self::UnsupportedVersion => "maintenance step state version is unsupported",
            Self::ZeroSequence => "maintenance step authorization sequence is zero",
            Self::InvalidBaseSequence => "maintenance step base sequence is invalid",
            Self::ZeroEntryCount => "maintenance step entry count is zero",
            Self::EntryCountMismatch => {
                "maintenance step entry count does not match its sequence and ordinal"
            }
            Self::UnsupportedOperations => {
                "maintenance step program is not the fixed storage-rotation operation"
            }
            Self::UnexpectedUseLimit => "maintenance step program requires exactly two uses",
            Self::InvalidStep => "maintenance step ordinal is outside the fixed program",
            Self::ObservationMismatch => {
                "maintenance step observation does not match the fixed program"
            }
            Self::InvalidFlags => "maintenance step flags do not match its fixed semantics",
            Self::ZeroAuthorizationDigest => "maintenance step authorization digest is zero",
            Self::ZeroAuthorizationId => "maintenance step authorization id is zero",
            Self::ZeroManifestDigest => "maintenance step manifest digest is zero",
            Self::ZeroPolicyDigest => "maintenance step policy digest is zero",
            Self::ZeroRootDigest => "maintenance step root digest is zero",
            Self::ZeroDeviceBinding => "maintenance step device binding is zero",
            Self::ZeroEffectDigest => "maintenance step effect digest is zero",
            Self::EffectMismatch => "maintenance step effect digest is invalid",
            Self::ZeroChainDigest => "maintenance step chain digest is zero",
            Self::NonZeroReserved => "maintenance step reserved bytes are nonzero",
            Self::ChainMismatch => "maintenance step chain digest is invalid",
            Self::WitnessMismatch => "maintenance step witness is invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepState {
    pub sequence: u64,
    pub base_sequence: u64,
    pub entry_count: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub step_ordinal: u32,
    pub observed_rotations: u32,
    pub flags: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub effect_sha256: [u8; 32],
    pub previous_chain_sha256: [u8; 32],
    pub chain_sha256: [u8; 32],
}

impl MaintenanceStepState {
    pub fn encode(self) -> Result<[u8; MAINTENANCE_STEP_STATE_BYTES], MaintenanceStepStateError> {
        self.validate()?;
        let mut bytes = [0_u8; MAINTENANCE_STEP_STATE_BYTES];
        bytes[..8].copy_from_slice(&MAINTENANCE_STEP_STATE_MAGIC);
        put_u32(&mut bytes, 8, MAINTENANCE_STEP_STATE_VERSION);
        put_u64(&mut bytes, 16, self.sequence);
        put_u64(&mut bytes, 24, self.base_sequence);
        put_u64(&mut bytes, 32, self.entry_count);
        put_u64(&mut bytes, 40, self.operations);
        put_u32(&mut bytes, 48, self.max_uses);
        put_u32(&mut bytes, 52, self.step_ordinal);
        put_u32(&mut bytes, 56, self.observed_rotations);
        put_u32(&mut bytes, 60, self.flags);
        bytes[64..96].copy_from_slice(&self.authorization_sha256);
        bytes[96..128].copy_from_slice(&self.authorization_id);
        bytes[128..160].copy_from_slice(&self.manifest_sha256);
        bytes[160..192].copy_from_slice(&self.policy_sha256);
        bytes[192..224].copy_from_slice(&self.root_sha256);
        bytes[224..256].copy_from_slice(&self.device_binding_sha256);
        bytes[256..288].copy_from_slice(&self.effect_sha256);
        bytes[288..320].copy_from_slice(&self.previous_chain_sha256);
        bytes[320..352].copy_from_slice(&self.chain_sha256);
        let witness = fnv1a64(&bytes[16..352]) ^ MAINTENANCE_STEP_WITNESS;
        put_u64(&mut bytes, 352, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MaintenanceStepStateError> {
        if bytes.len() != MAINTENANCE_STEP_STATE_BYTES {
            return Err(MaintenanceStepStateError::WrongSize);
        }
        if bytes[..8] != MAINTENANCE_STEP_STATE_MAGIC {
            return Err(MaintenanceStepStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MAINTENANCE_STEP_STATE_VERSION {
            return Err(MaintenanceStepStateError::UnsupportedVersion);
        }
        if bytes[12..16]
            .iter()
            .chain(bytes[360..376].iter())
            .any(|byte| *byte != 0)
        {
            return Err(MaintenanceStepStateError::NonZeroReserved);
        }
        let state = Self {
            sequence: le_u64(bytes, 16),
            base_sequence: le_u64(bytes, 24),
            entry_count: le_u64(bytes, 32),
            operations: le_u64(bytes, 40),
            max_uses: le_u32(bytes, 48),
            step_ordinal: le_u32(bytes, 52),
            observed_rotations: le_u32(bytes, 56),
            flags: le_u32(bytes, 60),
            authorization_sha256: bytes[64..96]
                .try_into()
                .expect("fixed maintenance step authorization digest range"),
            authorization_id: bytes[96..128]
                .try_into()
                .expect("fixed maintenance step authorization id range"),
            manifest_sha256: bytes[128..160]
                .try_into()
                .expect("fixed maintenance step manifest digest range"),
            policy_sha256: bytes[160..192]
                .try_into()
                .expect("fixed maintenance step policy digest range"),
            root_sha256: bytes[192..224]
                .try_into()
                .expect("fixed maintenance step root digest range"),
            device_binding_sha256: bytes[224..256]
                .try_into()
                .expect("fixed maintenance step device-binding range"),
            effect_sha256: bytes[256..288]
                .try_into()
                .expect("fixed maintenance step effect digest range"),
            previous_chain_sha256: bytes[288..320]
                .try_into()
                .expect("fixed previous maintenance step chain range"),
            chain_sha256: bytes[320..352]
                .try_into()
                .expect("fixed maintenance step chain range"),
        };
        state.validate()?;
        if le_u64(bytes, 352) != fnv1a64(&bytes[16..352]) ^ MAINTENANCE_STEP_WITNESS {
            return Err(MaintenanceStepStateError::WitnessMismatch);
        }
        Ok(state)
    }

    fn binding(self) -> MaintenanceStepCommitBinding {
        MaintenanceStepCommitBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            step_ordinal: self.step_ordinal,
            observed_rotations: self.observed_rotations,
            flags: self.flags,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
            effect_sha256: self.effect_sha256,
        }
    }

    fn validate(self) -> Result<(), MaintenanceStepStateError> {
        let binding = self.binding();
        binding.validate()?;
        if self.base_sequence == 0 || self.base_sequence > self.sequence {
            return Err(MaintenanceStepStateError::InvalidBaseSequence);
        }
        if self.entry_count == 0 {
            return Err(MaintenanceStepStateError::ZeroEntryCount);
        }
        let expected_entry_count = self
            .sequence
            .checked_sub(self.base_sequence)
            .and_then(|delta| delta.checked_mul(u64::from(MAINTENANCE_STEP_COUNT)))
            .and_then(|count| count.checked_add(u64::from(self.step_ordinal)))
            .ok_or(MaintenanceStepStateError::EntryCountMismatch)?;
        if self.entry_count != expected_entry_count {
            return Err(MaintenanceStepStateError::EntryCountMismatch);
        }
        if self.chain_sha256.iter().all(|byte| *byte == 0) {
            return Err(MaintenanceStepStateError::ZeroChainDigest);
        }
        if self.chain_sha256
            != maintenance_step_chain_sha256(
                self.previous_chain_sha256,
                self.base_sequence,
                self.entry_count,
                binding,
            )
        {
            return Err(MaintenanceStepStateError::ChainMismatch);
        }
        Ok(())
    }
}

pub fn maintenance_step_chain_sha256(
    previous_chain_sha256: [u8; 32],
    base_sequence: u64,
    entry_count: u64,
    binding: MaintenanceStepCommitBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 312];
    material[..8].copy_from_slice(b"BNDRMST1");
    material[8..40].copy_from_slice(&previous_chain_sha256);
    material[40..72].copy_from_slice(&binding.effect_sha256);
    material[72..104].copy_from_slice(&binding.authorization_sha256);
    material[104..136].copy_from_slice(&binding.authorization_id);
    material[136..144].copy_from_slice(&binding.sequence.to_le_bytes());
    material[144..152].copy_from_slice(&base_sequence.to_le_bytes());
    material[152..160].copy_from_slice(&entry_count.to_le_bytes());
    material[160..168].copy_from_slice(&binding.operations.to_le_bytes());
    material[168..172].copy_from_slice(&binding.max_uses.to_le_bytes());
    material[172..176].copy_from_slice(&binding.step_ordinal.to_le_bytes());
    material[176..180].copy_from_slice(&binding.observed_rotations.to_le_bytes());
    material[180..184].copy_from_slice(&binding.flags.to_le_bytes());
    material[184..216].copy_from_slice(&binding.manifest_sha256);
    material[216..248].copy_from_slice(&binding.policy_sha256);
    material[248..280].copy_from_slice(&binding.root_sha256);
    material[280..312].copy_from_slice(&binding.device_binding_sha256);
    sha256(&material)
}

/// One durable phase transition in M80's bounded maintenance plan.
///
/// The operation instance and idempotency key are derived from the exact
/// signed authorization and M79 effect descriptor. `APPLYING` is an intent
/// record: after a restart the resident effect may be absent or already
/// durable in `BNDRMST1`, so the caller must reconcile that ledger before
/// confirming. This protocol does not claim exactly-once behavior for
/// unrelated external participants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanTransitionBinding {
    pub sequence: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub operation_ordinal: u32,
    pub phase: u32,
    pub flags: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub effect_sha256: [u8; 32],
    pub operation_instance_id: [u8; 32],
    pub idempotency_key: [u8; 32],
}

impl MaintenancePlanTransitionBinding {
    pub fn for_transition(
        authorization: MaintenanceAuthorizationAuditBinding,
        operation_ordinal: u32,
        phase: u32,
    ) -> Result<Self, MaintenancePlanStateError> {
        let step = maintenance_plan_step_binding(authorization, operation_ordinal)?;
        let flags =
            maintenance_plan_phase_flags(phase).ok_or(MaintenancePlanStateError::InvalidPhase)?;
        let plan_id = maintenance_plan_id_sha256(authorization);
        let operation_instance_id = maintenance_plan_operation_instance_id_sha256(
            plan_id,
            operation_ordinal,
            step.effect_sha256,
        );
        let idempotency_key = maintenance_plan_idempotency_key_sha256(
            plan_id,
            operation_instance_id,
            step.effect_sha256,
        );
        let binding = Self {
            sequence: authorization.sequence,
            operations: authorization.operations,
            max_uses: authorization.max_uses,
            operation_ordinal,
            phase,
            flags,
            authorization_sha256: authorization.authorization_sha256,
            authorization_id: authorization.authorization_id,
            manifest_sha256: authorization.manifest_sha256,
            policy_sha256: authorization.policy_sha256,
            root_sha256: authorization.root_sha256,
            device_binding_sha256: authorization.device_binding_sha256,
            effect_sha256: step.effect_sha256,
            operation_instance_id,
            idempotency_key,
        };
        binding.validate()?;
        Ok(binding)
    }

    fn validate(self) -> Result<(), MaintenancePlanStateError> {
        let authorization = self.authorization_binding();
        authorization
            .validate()
            .map_err(MaintenancePlanStateError::Authorization)?;
        if self.operations != MAINTENANCE_STEP_OPERATION_STORAGE_ROTATION {
            return Err(MaintenancePlanStateError::UnsupportedOperations);
        }
        if self.max_uses != 2 {
            return Err(MaintenancePlanStateError::UnexpectedUseLimit);
        }
        if !(MAINTENANCE_STEP_ROTATION_ONE..=MAINTENANCE_STEP_DRAIN)
            .contains(&self.operation_ordinal)
        {
            return Err(MaintenancePlanStateError::InvalidOperation);
        }
        let expected_flags = maintenance_plan_phase_flags(self.phase)
            .ok_or(MaintenancePlanStateError::InvalidPhase)?;
        if self.flags != expected_flags {
            return Err(MaintenancePlanStateError::InvalidFlags);
        }
        let step = maintenance_plan_step_binding(authorization, self.operation_ordinal)?;
        if self.effect_sha256 != step.effect_sha256 {
            return Err(MaintenancePlanStateError::EffectMismatch);
        }
        let plan_id = maintenance_plan_id_sha256(authorization);
        let operation_instance_id = maintenance_plan_operation_instance_id_sha256(
            plan_id,
            self.operation_ordinal,
            self.effect_sha256,
        );
        if self.operation_instance_id != operation_instance_id {
            return Err(MaintenancePlanStateError::OperationInstanceMismatch);
        }
        if self.idempotency_key
            != maintenance_plan_idempotency_key_sha256(
                plan_id,
                operation_instance_id,
                self.effect_sha256,
            )
        {
            return Err(MaintenancePlanStateError::IdempotencyKeyMismatch);
        }
        for (digest, error) in [
            (
                self.effect_sha256,
                MaintenancePlanStateError::ZeroEffectDigest,
            ),
            (
                self.operation_instance_id,
                MaintenancePlanStateError::ZeroOperationInstance,
            ),
            (
                self.idempotency_key,
                MaintenancePlanStateError::ZeroIdempotencyKey,
            ),
        ] {
            if digest.iter().all(|byte| *byte == 0) {
                return Err(error);
            }
        }
        Ok(())
    }

    fn authorization_binding(self) -> MaintenanceAuthorizationAuditBinding {
        MaintenanceAuthorizationAuditBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
        }
    }
}

fn maintenance_plan_step_binding(
    authorization: MaintenanceAuthorizationAuditBinding,
    operation_ordinal: u32,
) -> Result<MaintenanceStepCommitBinding, MaintenancePlanStateError> {
    let (observed_rotations, drain_validated) = match operation_ordinal {
        MAINTENANCE_STEP_ROTATION_ONE => (1, false),
        MAINTENANCE_STEP_ROTATION_TWO => (2, false),
        MAINTENANCE_STEP_DRAIN => (2, true),
        _ => return Err(MaintenancePlanStateError::InvalidOperation),
    };
    MaintenanceStepCommitBinding::for_observation(
        authorization,
        operation_ordinal,
        observed_rotations,
        drain_validated,
    )
    .map_err(MaintenancePlanStateError::Step)
}

const fn maintenance_plan_phase_flags(phase: u32) -> Option<u32> {
    match phase {
        MAINTENANCE_PLAN_PHASE_PREPARED => Some(MAINTENANCE_PLAN_IDEMPOTENT_EFFECT),
        MAINTENANCE_PLAN_PHASE_APPLYING => {
            Some(MAINTENANCE_PLAN_IDEMPOTENT_EFFECT | MAINTENANCE_PLAN_RESULT_MAY_BE_UNKNOWN)
        }
        MAINTENANCE_PLAN_PHASE_CONFIRMED => {
            Some(MAINTENANCE_PLAN_IDEMPOTENT_EFFECT | MAINTENANCE_PLAN_EFFECT_CONFIRMED)
        }
        MAINTENANCE_PLAN_PHASE_COMPENSATED => {
            Some(MAINTENANCE_PLAN_IDEMPOTENT_EFFECT | MAINTENANCE_PLAN_COMPENSATED_WITHOUT_EFFECT)
        }
        _ => None,
    }
}

const fn maintenance_plan_phase_position(phase: u32) -> Option<u64> {
    match phase {
        MAINTENANCE_PLAN_PHASE_PREPARED => Some(1),
        MAINTENANCE_PLAN_PHASE_APPLYING | MAINTENANCE_PLAN_PHASE_COMPENSATED => Some(2),
        MAINTENANCE_PLAN_PHASE_CONFIRMED => Some(3),
        _ => None,
    }
}

pub fn maintenance_plan_id_sha256(authorization: MaintenanceAuthorizationAuditBinding) -> [u8; 32] {
    let mut material = [0_u8; 220];
    material[..8].copy_from_slice(b"BNDRMPL1");
    material[8..16].copy_from_slice(&authorization.sequence.to_le_bytes());
    material[16..24].copy_from_slice(&authorization.operations.to_le_bytes());
    material[24..28].copy_from_slice(&authorization.max_uses.to_le_bytes());
    material[28..60].copy_from_slice(&authorization.authorization_sha256);
    material[60..92].copy_from_slice(&authorization.authorization_id);
    material[92..124].copy_from_slice(&authorization.manifest_sha256);
    material[124..156].copy_from_slice(&authorization.policy_sha256);
    material[156..188].copy_from_slice(&authorization.root_sha256);
    material[188..220].copy_from_slice(&authorization.device_binding_sha256);
    sha256(&material)
}

pub fn maintenance_plan_operation_instance_id_sha256(
    plan_id_sha256: [u8; 32],
    operation_ordinal: u32,
    effect_sha256: [u8; 32],
) -> [u8; 32] {
    let mut material = [0_u8; 76];
    material[..8].copy_from_slice(b"M80-OPI1");
    material[8..40].copy_from_slice(&plan_id_sha256);
    material[40..44].copy_from_slice(&operation_ordinal.to_le_bytes());
    material[44..76].copy_from_slice(&effect_sha256);
    sha256(&material)
}

pub fn maintenance_plan_idempotency_key_sha256(
    plan_id_sha256: [u8; 32],
    operation_instance_id: [u8; 32],
    effect_sha256: [u8; 32],
) -> [u8; 32] {
    let mut material = [0_u8; 104];
    material[..8].copy_from_slice(b"M80-IDK1");
    material[8..40].copy_from_slice(&plan_id_sha256);
    material[40..72].copy_from_slice(&operation_instance_id);
    material[72..104].copy_from_slice(&effect_sha256);
    sha256(&material)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenancePlanStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    InvalidBaseSequence,
    ZeroTransitionCount,
    TransitionCountMismatch,
    UnsupportedOperations,
    UnexpectedUseLimit,
    InvalidOperation,
    InvalidPhase,
    InvalidFlags,
    ZeroEffectDigest,
    ZeroOperationInstance,
    ZeroIdempotencyKey,
    EffectMismatch,
    OperationInstanceMismatch,
    IdempotencyKeyMismatch,
    ZeroChainDigest,
    NonZeroReserved,
    ChainMismatch,
    WitnessMismatch,
    Authorization(MaintenanceAuditStateError),
    Step(MaintenanceStepStateError),
}

impl MaintenancePlanStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "maintenance plan state has the wrong size",
            Self::BadMagic => "maintenance plan state magic is invalid",
            Self::UnsupportedVersion => "maintenance plan state version is unsupported",
            Self::InvalidBaseSequence => "maintenance plan base sequence is invalid",
            Self::ZeroTransitionCount => "maintenance plan transition count is zero",
            Self::TransitionCountMismatch => {
                "maintenance plan transition count does not match its sequence, operation, and phase"
            }
            Self::UnsupportedOperations => {
                "maintenance plan is not the fixed storage-rotation operation"
            }
            Self::UnexpectedUseLimit => "maintenance plan requires exactly two uses",
            Self::InvalidOperation => "maintenance plan operation is outside the fixed program",
            Self::InvalidPhase => "maintenance plan phase is invalid",
            Self::InvalidFlags => "maintenance plan flags do not match its phase",
            Self::ZeroEffectDigest => "maintenance plan effect digest is zero",
            Self::ZeroOperationInstance => "maintenance plan operation instance id is zero",
            Self::ZeroIdempotencyKey => "maintenance plan idempotency key is zero",
            Self::EffectMismatch => "maintenance plan effect digest is invalid",
            Self::OperationInstanceMismatch => "maintenance plan operation instance id is invalid",
            Self::IdempotencyKeyMismatch => "maintenance plan idempotency key is invalid",
            Self::ZeroChainDigest => "maintenance plan chain digest is zero",
            Self::NonZeroReserved => "maintenance plan reserved bytes are nonzero",
            Self::ChainMismatch => "maintenance plan chain digest is invalid",
            Self::WitnessMismatch => "maintenance plan witness is invalid",
            Self::Authorization(error) => error.as_str(),
            Self::Step(error) => error.as_str(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanState {
    pub sequence: u64,
    pub base_sequence: u64,
    pub transition_count: u64,
    pub operations: u64,
    pub max_uses: u32,
    pub operation_ordinal: u32,
    pub phase: u32,
    pub flags: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub manifest_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
    pub effect_sha256: [u8; 32],
    pub operation_instance_id: [u8; 32],
    pub idempotency_key: [u8; 32],
    pub previous_chain_sha256: [u8; 32],
    pub chain_sha256: [u8; 32],
}

impl MaintenancePlanState {
    pub fn encode(self) -> Result<[u8; MAINTENANCE_PLAN_STATE_BYTES], MaintenancePlanStateError> {
        self.validate()?;
        let mut bytes = [0_u8; MAINTENANCE_PLAN_STATE_BYTES];
        bytes[..8].copy_from_slice(&MAINTENANCE_PLAN_STATE_MAGIC);
        put_u32(&mut bytes, 8, MAINTENANCE_PLAN_STATE_VERSION);
        put_u64(&mut bytes, 16, self.sequence);
        put_u64(&mut bytes, 24, self.base_sequence);
        put_u64(&mut bytes, 32, self.transition_count);
        put_u64(&mut bytes, 40, self.operations);
        put_u32(&mut bytes, 48, self.max_uses);
        put_u32(&mut bytes, 52, self.operation_ordinal);
        put_u32(&mut bytes, 56, self.phase);
        put_u32(&mut bytes, 60, self.flags);
        bytes[64..96].copy_from_slice(&self.authorization_sha256);
        bytes[96..128].copy_from_slice(&self.authorization_id);
        bytes[128..160].copy_from_slice(&self.manifest_sha256);
        bytes[160..192].copy_from_slice(&self.policy_sha256);
        bytes[192..224].copy_from_slice(&self.root_sha256);
        bytes[224..256].copy_from_slice(&self.device_binding_sha256);
        bytes[256..288].copy_from_slice(&self.effect_sha256);
        bytes[288..320].copy_from_slice(&self.operation_instance_id);
        bytes[320..352].copy_from_slice(&self.idempotency_key);
        bytes[352..384].copy_from_slice(&self.previous_chain_sha256);
        bytes[384..416].copy_from_slice(&self.chain_sha256);
        let witness = fnv1a64(&bytes[16..416]) ^ MAINTENANCE_PLAN_WITNESS;
        put_u64(&mut bytes, 416, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MaintenancePlanStateError> {
        if bytes.len() != MAINTENANCE_PLAN_STATE_BYTES {
            return Err(MaintenancePlanStateError::WrongSize);
        }
        if bytes[..8] != MAINTENANCE_PLAN_STATE_MAGIC {
            return Err(MaintenancePlanStateError::BadMagic);
        }
        if le_u32(bytes, 8) != MAINTENANCE_PLAN_STATE_VERSION {
            return Err(MaintenancePlanStateError::UnsupportedVersion);
        }
        if bytes[12..16].iter().any(|byte| *byte != 0) {
            return Err(MaintenancePlanStateError::NonZeroReserved);
        }
        let state = Self {
            sequence: le_u64(bytes, 16),
            base_sequence: le_u64(bytes, 24),
            transition_count: le_u64(bytes, 32),
            operations: le_u64(bytes, 40),
            max_uses: le_u32(bytes, 48),
            operation_ordinal: le_u32(bytes, 52),
            phase: le_u32(bytes, 56),
            flags: le_u32(bytes, 60),
            authorization_sha256: bytes[64..96]
                .try_into()
                .expect("fixed maintenance plan authorization digest range"),
            authorization_id: bytes[96..128]
                .try_into()
                .expect("fixed maintenance plan authorization id range"),
            manifest_sha256: bytes[128..160]
                .try_into()
                .expect("fixed maintenance plan manifest digest range"),
            policy_sha256: bytes[160..192]
                .try_into()
                .expect("fixed maintenance plan policy digest range"),
            root_sha256: bytes[192..224]
                .try_into()
                .expect("fixed maintenance plan root digest range"),
            device_binding_sha256: bytes[224..256]
                .try_into()
                .expect("fixed maintenance plan device-binding range"),
            effect_sha256: bytes[256..288]
                .try_into()
                .expect("fixed maintenance plan effect range"),
            operation_instance_id: bytes[288..320]
                .try_into()
                .expect("fixed maintenance plan operation-instance range"),
            idempotency_key: bytes[320..352]
                .try_into()
                .expect("fixed maintenance plan idempotency-key range"),
            previous_chain_sha256: bytes[352..384]
                .try_into()
                .expect("fixed previous maintenance plan chain range"),
            chain_sha256: bytes[384..416]
                .try_into()
                .expect("fixed maintenance plan chain range"),
        };
        state.validate()?;
        if le_u64(bytes, 416) != fnv1a64(&bytes[16..416]) ^ MAINTENANCE_PLAN_WITNESS {
            return Err(MaintenancePlanStateError::WitnessMismatch);
        }
        Ok(state)
    }

    fn binding(self) -> MaintenancePlanTransitionBinding {
        MaintenancePlanTransitionBinding {
            sequence: self.sequence,
            operations: self.operations,
            max_uses: self.max_uses,
            operation_ordinal: self.operation_ordinal,
            phase: self.phase,
            flags: self.flags,
            authorization_sha256: self.authorization_sha256,
            authorization_id: self.authorization_id,
            manifest_sha256: self.manifest_sha256,
            policy_sha256: self.policy_sha256,
            root_sha256: self.root_sha256,
            device_binding_sha256: self.device_binding_sha256,
            effect_sha256: self.effect_sha256,
            operation_instance_id: self.operation_instance_id,
            idempotency_key: self.idempotency_key,
        }
    }

    fn validate(self) -> Result<(), MaintenancePlanStateError> {
        let binding = self.binding();
        binding.validate()?;
        if self.base_sequence == 0 || self.base_sequence > self.sequence {
            return Err(MaintenancePlanStateError::InvalidBaseSequence);
        }
        if self.transition_count == 0 {
            return Err(MaintenancePlanStateError::ZeroTransitionCount);
        }
        let phase_position = maintenance_plan_phase_position(self.phase)
            .ok_or(MaintenancePlanStateError::InvalidPhase)?;
        let expected_transition_count = self
            .sequence
            .checked_sub(self.base_sequence)
            .and_then(|delta| delta.checked_mul(MAINTENANCE_PLAN_TRANSITIONS_PER_SEQUENCE))
            .and_then(|count| {
                count.checked_add(u64::from(self.operation_ordinal - 1).checked_mul(3)?)
            })
            .and_then(|count| count.checked_add(phase_position))
            .ok_or(MaintenancePlanStateError::TransitionCountMismatch)?;
        if self.transition_count != expected_transition_count {
            return Err(MaintenancePlanStateError::TransitionCountMismatch);
        }
        if self.chain_sha256.iter().all(|byte| *byte == 0) {
            return Err(MaintenancePlanStateError::ZeroChainDigest);
        }
        if self.chain_sha256
            != maintenance_plan_chain_sha256(
                self.previous_chain_sha256,
                self.base_sequence,
                self.transition_count,
                binding,
            )
        {
            return Err(MaintenancePlanStateError::ChainMismatch);
        }
        Ok(())
    }
}

pub fn maintenance_plan_chain_sha256(
    previous_chain_sha256: [u8; 32],
    base_sequence: u64,
    transition_count: u64,
    binding: MaintenancePlanTransitionBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 376];
    material[..8].copy_from_slice(b"BNDRMPL1");
    material[8..40].copy_from_slice(&previous_chain_sha256);
    material[40..48].copy_from_slice(&binding.sequence.to_le_bytes());
    material[48..56].copy_from_slice(&base_sequence.to_le_bytes());
    material[56..64].copy_from_slice(&transition_count.to_le_bytes());
    material[64..72].copy_from_slice(&binding.operations.to_le_bytes());
    material[72..76].copy_from_slice(&binding.max_uses.to_le_bytes());
    material[76..80].copy_from_slice(&binding.operation_ordinal.to_le_bytes());
    material[80..84].copy_from_slice(&binding.phase.to_le_bytes());
    material[84..88].copy_from_slice(&binding.flags.to_le_bytes());
    material[88..120].copy_from_slice(&binding.authorization_sha256);
    material[120..152].copy_from_slice(&binding.authorization_id);
    material[152..184].copy_from_slice(&binding.manifest_sha256);
    material[184..216].copy_from_slice(&binding.policy_sha256);
    material[216..248].copy_from_slice(&binding.root_sha256);
    material[248..280].copy_from_slice(&binding.device_binding_sha256);
    material[280..312].copy_from_slice(&binding.effect_sha256);
    material[312..344].copy_from_slice(&binding.operation_instance_id);
    material[344..376].copy_from_slice(&binding.idempotency_key);
    sha256(&material)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignedMaintenancePlanStateError {
    WrongSize,
    BadMagic,
    UnsupportedVersion,
    ZeroSequence,
    InvalidOperationCount,
    InvalidTransitionCount,
    InvalidProgramVersion,
    InvalidCancelMask,
    ZeroAuthorizationDigest,
    ZeroAuthorizationId,
    ZeroProgramDigest,
    ZeroPlanId,
    ZeroDescriptorDigest,
    ZeroRootDigest,
    ZeroDeviceBinding,
    AuthorizationSequenceMismatch,
    AuthorizationDigestMismatch,
    AuthorizationIdMismatch,
    DeviceBindingMismatch,
    PlanIdMismatch,
    ZeroChainDigest,
    NonZeroReserved,
    ChainMismatch,
    WitnessMismatch,
    Authorization(MaintenanceAuditStateError),
}

impl SignedMaintenancePlanStateError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongSize => "signed maintenance-plan state has the wrong size",
            Self::BadMagic => "signed maintenance-plan state magic is invalid",
            Self::UnsupportedVersion => "signed maintenance-plan state version is unsupported",
            Self::ZeroSequence => "signed maintenance-plan authorization sequence is zero",
            Self::InvalidOperationCount => {
                "signed maintenance-plan operation count is not the bounded product count"
            }
            Self::InvalidTransitionCount => {
                "signed maintenance-plan transition count is not the bounded product count"
            }
            Self::InvalidProgramVersion => "signed maintenance-plan program version is unsupported",
            Self::InvalidCancelMask => {
                "signed maintenance-plan cancellation mask violates product policy"
            }
            Self::ZeroAuthorizationDigest => "signed maintenance-plan authorization digest is zero",
            Self::ZeroAuthorizationId => "signed maintenance-plan authorization id is zero",
            Self::ZeroProgramDigest => "signed maintenance-plan artifact digest is zero",
            Self::ZeroPlanId => "signed maintenance-plan legacy plan id is zero",
            Self::ZeroDescriptorDigest => "signed maintenance-plan descriptor digest is zero",
            Self::ZeroRootDigest => "signed maintenance-plan trust-root digest is zero",
            Self::ZeroDeviceBinding => "signed maintenance-plan device binding is zero",
            Self::AuthorizationSequenceMismatch => {
                "signed maintenance-plan sequence does not match its authorization"
            }
            Self::AuthorizationDigestMismatch => {
                "signed maintenance-plan digest does not match its authorization"
            }
            Self::AuthorizationIdMismatch => {
                "signed maintenance-plan id does not match its authorization"
            }
            Self::DeviceBindingMismatch => {
                "signed maintenance-plan device namespace does not match its authorization"
            }
            Self::PlanIdMismatch => {
                "signed maintenance-plan does not bind the selected M80 plan identity"
            }
            Self::ZeroChainDigest => "signed maintenance-plan chain digest is zero",
            Self::NonZeroReserved => "signed maintenance-plan reserved bytes are nonzero",
            Self::ChainMismatch => "signed maintenance-plan chain digest is invalid",
            Self::WitnessMismatch => "signed maintenance-plan witness is invalid",
            Self::Authorization(error) => error.as_str(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanBinding {
    pub authorization_sequence: u64,
    pub operation_count: u32,
    pub transition_count: u32,
    pub program_version: u32,
    pub preapply_cancel_mask: u32,
    pub authorization_sha256: [u8; 32],
    pub authorization_id: [u8; 32],
    pub program_sha256: [u8; 32],
    pub plan_id_sha256: [u8; 32],
    pub descriptor_sha256: [u8; 32],
    pub root_sha256: [u8; 32],
    pub device_binding_sha256: [u8; 32],
}

impl SignedMaintenancePlanBinding {
    fn validate_envelope(self) -> Result<(), SignedMaintenancePlanStateError> {
        if self.authorization_sequence == 0 {
            return Err(SignedMaintenancePlanStateError::ZeroSequence);
        }
        if self.operation_count != SIGNED_MAINTENANCE_PLAN_OPERATION_COUNT {
            return Err(SignedMaintenancePlanStateError::InvalidOperationCount);
        }
        if self.transition_count != SIGNED_MAINTENANCE_PLAN_TRANSITION_COUNT {
            return Err(SignedMaintenancePlanStateError::InvalidTransitionCount);
        }
        if self.program_version != SIGNED_MAINTENANCE_PLAN_PROGRAM_VERSION {
            return Err(SignedMaintenancePlanStateError::InvalidProgramVersion);
        }
        if self.preapply_cancel_mask != SIGNED_MAINTENANCE_PLAN_PREAPPLY_CANCEL_MASK {
            return Err(SignedMaintenancePlanStateError::InvalidCancelMask);
        }
        for (digest, error) in [
            (
                self.authorization_sha256,
                SignedMaintenancePlanStateError::ZeroAuthorizationDigest,
            ),
            (
                self.authorization_id,
                SignedMaintenancePlanStateError::ZeroAuthorizationId,
            ),
            (
                self.program_sha256,
                SignedMaintenancePlanStateError::ZeroProgramDigest,
            ),
            (
                self.plan_id_sha256,
                SignedMaintenancePlanStateError::ZeroPlanId,
            ),
            (
                self.descriptor_sha256,
                SignedMaintenancePlanStateError::ZeroDescriptorDigest,
            ),
            (
                self.root_sha256,
                SignedMaintenancePlanStateError::ZeroRootDigest,
            ),
            (
                self.device_binding_sha256,
                SignedMaintenancePlanStateError::ZeroDeviceBinding,
            ),
        ] {
            if digest.iter().all(|byte| *byte == 0) {
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn validate(
        self,
        authorization: MaintenanceAuthorizationAuditBinding,
    ) -> Result<(), SignedMaintenancePlanStateError> {
        authorization
            .validate()
            .map_err(SignedMaintenancePlanStateError::Authorization)?;
        self.validate_envelope()?;
        if self.authorization_sequence != authorization.sequence {
            return Err(SignedMaintenancePlanStateError::AuthorizationSequenceMismatch);
        }
        if self.authorization_sha256 != authorization.authorization_sha256 {
            return Err(SignedMaintenancePlanStateError::AuthorizationDigestMismatch);
        }
        if self.authorization_id != authorization.authorization_id {
            return Err(SignedMaintenancePlanStateError::AuthorizationIdMismatch);
        }
        if self.device_binding_sha256 != authorization.device_binding_sha256 {
            return Err(SignedMaintenancePlanStateError::DeviceBindingMismatch);
        }
        if self.plan_id_sha256 != maintenance_plan_id_sha256(authorization) {
            return Err(SignedMaintenancePlanStateError::PlanIdMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanState {
    pub binding: SignedMaintenancePlanBinding,
    pub previous_chain_sha256: [u8; 32],
    pub chain_sha256: [u8; 32],
}

impl SignedMaintenancePlanState {
    pub fn encode(
        self,
        authorization: MaintenanceAuthorizationAuditBinding,
    ) -> Result<[u8; SIGNED_MAINTENANCE_PLAN_STATE_BYTES], SignedMaintenancePlanStateError> {
        self.validate(authorization)?;
        let mut bytes = [0_u8; SIGNED_MAINTENANCE_PLAN_STATE_BYTES];
        bytes[..8].copy_from_slice(&SIGNED_MAINTENANCE_PLAN_STATE_MAGIC);
        put_u32(&mut bytes, 8, SIGNED_MAINTENANCE_PLAN_STATE_VERSION);
        put_u64(&mut bytes, 16, self.binding.authorization_sequence);
        put_u32(&mut bytes, 24, self.binding.operation_count);
        put_u32(&mut bytes, 28, self.binding.transition_count);
        put_u32(&mut bytes, 32, self.binding.program_version);
        put_u32(&mut bytes, 36, self.binding.preapply_cancel_mask);
        bytes[40..72].copy_from_slice(&self.binding.authorization_sha256);
        bytes[72..104].copy_from_slice(&self.binding.authorization_id);
        bytes[104..136].copy_from_slice(&self.binding.program_sha256);
        bytes[136..168].copy_from_slice(&self.binding.plan_id_sha256);
        bytes[168..200].copy_from_slice(&self.binding.descriptor_sha256);
        bytes[200..232].copy_from_slice(&self.binding.root_sha256);
        bytes[232..264].copy_from_slice(&self.binding.device_binding_sha256);
        bytes[264..296].copy_from_slice(&self.previous_chain_sha256);
        bytes[296..328].copy_from_slice(&self.chain_sha256);
        let witness = fnv1a64(&bytes[16..328]) ^ SIGNED_MAINTENANCE_PLAN_WITNESS;
        put_u64(&mut bytes, 328, witness);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SignedMaintenancePlanStateError> {
        if bytes.len() != SIGNED_MAINTENANCE_PLAN_STATE_BYTES {
            return Err(SignedMaintenancePlanStateError::WrongSize);
        }
        if bytes[..8] != SIGNED_MAINTENANCE_PLAN_STATE_MAGIC {
            return Err(SignedMaintenancePlanStateError::BadMagic);
        }
        if le_u32(bytes, 8) != SIGNED_MAINTENANCE_PLAN_STATE_VERSION {
            return Err(SignedMaintenancePlanStateError::UnsupportedVersion);
        }
        if bytes[12..16].iter().any(|byte| *byte != 0) {
            return Err(SignedMaintenancePlanStateError::NonZeroReserved);
        }
        let state = Self {
            binding: SignedMaintenancePlanBinding {
                authorization_sequence: le_u64(bytes, 16),
                operation_count: le_u32(bytes, 24),
                transition_count: le_u32(bytes, 28),
                program_version: le_u32(bytes, 32),
                preapply_cancel_mask: le_u32(bytes, 36),
                authorization_sha256: bytes[40..72]
                    .try_into()
                    .expect("fixed signed-plan authorization digest range"),
                authorization_id: bytes[72..104]
                    .try_into()
                    .expect("fixed signed-plan authorization id range"),
                program_sha256: bytes[104..136]
                    .try_into()
                    .expect("fixed signed-plan program digest range"),
                plan_id_sha256: bytes[136..168]
                    .try_into()
                    .expect("fixed signed-plan legacy plan id range"),
                descriptor_sha256: bytes[168..200]
                    .try_into()
                    .expect("fixed signed-plan descriptor digest range"),
                root_sha256: bytes[200..232]
                    .try_into()
                    .expect("fixed signed-plan root digest range"),
                device_binding_sha256: bytes[232..264]
                    .try_into()
                    .expect("fixed signed-plan device-binding range"),
            },
            previous_chain_sha256: bytes[264..296]
                .try_into()
                .expect("fixed signed-plan previous-chain range"),
            chain_sha256: bytes[296..328]
                .try_into()
                .expect("fixed signed-plan chain range"),
        };
        state.validate_stored()?;
        if le_u64(bytes, 328) != fnv1a64(&bytes[16..328]) ^ SIGNED_MAINTENANCE_PLAN_WITNESS {
            return Err(SignedMaintenancePlanStateError::WitnessMismatch);
        }
        Ok(state)
    }

    fn validate(
        self,
        authorization: MaintenanceAuthorizationAuditBinding,
    ) -> Result<(), SignedMaintenancePlanStateError> {
        self.binding.validate(authorization)?;
        self.validate_stored()
    }

    fn validate_stored(self) -> Result<(), SignedMaintenancePlanStateError> {
        self.binding.validate_envelope()?;
        if self.chain_sha256.iter().all(|byte| *byte == 0) {
            return Err(SignedMaintenancePlanStateError::ZeroChainDigest);
        }
        if self.chain_sha256
            != signed_maintenance_plan_chain_sha256(self.previous_chain_sha256, self.binding)
        {
            return Err(SignedMaintenancePlanStateError::ChainMismatch);
        }
        Ok(())
    }
}

pub fn signed_maintenance_plan_chain_sha256(
    previous_chain_sha256: [u8; 32],
    binding: SignedMaintenancePlanBinding,
) -> [u8; 32] {
    let mut material = [0_u8; 288];
    material[..8].copy_from_slice(b"BNDRMPB1");
    material[8..40].copy_from_slice(&previous_chain_sha256);
    material[40..48].copy_from_slice(&binding.authorization_sequence.to_le_bytes());
    material[48..52].copy_from_slice(&binding.operation_count.to_le_bytes());
    material[52..56].copy_from_slice(&binding.transition_count.to_le_bytes());
    material[56..60].copy_from_slice(&binding.program_version.to_le_bytes());
    material[60..64].copy_from_slice(&binding.preapply_cancel_mask.to_le_bytes());
    material[64..96].copy_from_slice(&binding.authorization_sha256);
    material[96..128].copy_from_slice(&binding.authorization_id);
    material[128..160].copy_from_slice(&binding.program_sha256);
    material[160..192].copy_from_slice(&binding.plan_id_sha256);
    material[192..224].copy_from_slice(&binding.descriptor_sha256);
    material[224..256].copy_from_slice(&binding.root_sha256);
    material[256..288].copy_from_slice(&binding.device_binding_sha256);
    sha256(&material)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistIoError {
    OutOfBounds,
    RequiresReset,
    OutcomeUnknown,
    Device,
}

impl PersistIoError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OutOfBounds => "persistent-data I/O is outside the block device",
            Self::RequiresReset => "persistent-data device requires reset before more I/O",
            Self::OutcomeUnknown => "persistent-data mutation outcome is unknown",
            Self::Device => "persistent-data block I/O failed",
        }
    }
}

/// Small synchronous boundary used by the crash-recovery transaction.
///
/// Implementations must not report `flush` success until all earlier completed
/// writes are stable. The transaction never issues a write while another
/// operation is outstanding.
pub trait DurableSectorIo {
    fn sector_count(&self) -> u64;
    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), PersistIoError>;
    fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), PersistIoError>;
    fn flush(&mut self) -> Result<(), PersistIoError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoPhase {
    ReadSuperblock,
    ReadInitialSlot,
    WriteInactiveSlot,
    Flush,
    ReadVerificationSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvanceError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Superblock(SuperblockError),
    Recovery(RecoveryError),
    BootState(BootStateError),
    DeviceHealthState(DeviceHealthStateError),
    StateGenerationMismatch,
    Encode(RecordError),
    VerificationFailed,
}

impl AdvanceError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "persistent-data partition is outside the block device",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                error: PersistIoError::RequiresReset,
            }
            | Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "persistent-data read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent-data record write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent-data flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "persistent-data superblock read failed",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "persistent-data initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "persistent-data inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "persistent-data cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "persistent-data verification read failed",
            Self::Superblock(error) => error.as_str(),
            Self::Recovery(error) => error.as_str(),
            Self::BootState(BootStateError::WrongSize) => {
                "persistent boot-state payload has the wrong size"
            }
            Self::BootState(BootStateError::BadMagic) => "persistent boot-state magic is invalid",
            Self::BootState(BootStateError::UnsupportedVersion) => {
                "persistent boot-state version is unsupported"
            }
            Self::BootState(BootStateError::NonZeroReserved) => {
                "persistent boot-state reserved bytes are nonzero"
            }
            Self::BootState(BootStateError::WitnessMismatch) => {
                "persistent boot-state witness is invalid"
            }
            Self::DeviceHealthState(error) => error.as_str(),
            Self::StateGenerationMismatch => {
                "persistent boot count does not match its record generation"
            }
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "persistent-data write did not survive flush and readback verification"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceHealthAdvanceError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Superblock(SuperblockError),
    Recovery(RecoveryError),
    LegacyBootState(BootStateError),
    DeviceHealthState(DeviceHealthStateError),
    StateGenerationMismatch,
    Encode(RecordError),
    VerificationFailed,
}

impl DeviceHealthAdvanceError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => {
                "persistent device-health partition is outside the block device"
            }
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                error: PersistIoError::RequiresReset,
            }
            | Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "persistent device-health read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent device-health write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent device-health flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "persistent device-health superblock read failed",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "persistent device-health initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "persistent device-health inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "persistent device-health cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "persistent device-health verification read failed",
            Self::Superblock(error) => error.as_str(),
            Self::Recovery(error) => error.as_str(),
            Self::LegacyBootState(BootStateError::WrongSize) => {
                "legacy persistent boot-state has the wrong size"
            }
            Self::LegacyBootState(BootStateError::BadMagic) => {
                "legacy persistent boot-state magic is invalid"
            }
            Self::LegacyBootState(BootStateError::UnsupportedVersion) => {
                "legacy persistent boot-state version is unsupported"
            }
            Self::LegacyBootState(BootStateError::NonZeroReserved) => {
                "legacy persistent boot-state reserved bytes are nonzero"
            }
            Self::LegacyBootState(BootStateError::WitnessMismatch) => {
                "legacy persistent boot-state witness is invalid"
            }
            Self::DeviceHealthState(error) => error.as_str(),
            Self::StateGenerationMismatch => {
                "persistent device-health boot count does not match its record generation"
            }
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "persistent device-health write did not survive flush and readback verification"
            }
        }
    }
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceHealthCloseError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Superblock(SuperblockError),
    Recovery(RecoveryError),
    DeviceHealthState(DeviceHealthStateError),
    StateGenerationMismatch,
    SessionGenerationMismatch,
    SessionAlreadyClosed,
    ContractMismatch,
    Encode(RecordError),
    VerificationFailed,
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
impl DeviceHealthCloseError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => {
                "persistent clean-shutdown partition is outside the block device"
            }
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                error: PersistIoError::RequiresReset,
            }
            | Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "persistent clean-shutdown read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent clean-shutdown write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent clean-shutdown flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "persistent clean-shutdown superblock read failed",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "persistent clean-shutdown initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "persistent clean-shutdown inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "persistent clean-shutdown cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "persistent clean-shutdown verification read failed",
            Self::Superblock(error) => error.as_str(),
            Self::Recovery(error) => error.as_str(),
            Self::DeviceHealthState(error) => error.as_str(),
            Self::StateGenerationMismatch => {
                "persistent clean-shutdown state generation does not match its record"
            }
            Self::SessionGenerationMismatch => {
                "persistent clean-shutdown record is not the current kernel boot session"
            }
            Self::SessionAlreadyClosed => {
                "persistent clean-shutdown boot session is already closed"
            }
            Self::ContractMismatch => {
                "persistent clean-shutdown device contract changed after the boot probe"
            }
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "persistent clean-shutdown write did not survive flush and readback verification"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestRollbackError {
    PartitionBounds,
    ZeroBootstrapFloor,
    ZeroArtifactIndex,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Recovery(RecoveryError),
    State(ManifestRollbackStateError),
    BindingMismatch,
    Rollback {
        actual: u32,
        minimum: u32,
    },
    Encode(RecordError),
    VerificationFailed,
}

impl ManifestRollbackError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => {
                "persistent manifest rollback slots are outside the data partition"
            }
            Self::ZeroBootstrapFloor => "manifest rollback bootstrap floor is zero",
            Self::ZeroArtifactIndex => "manifest rollback artifact index is zero",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "persistent manifest rollback read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent manifest rollback write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent manifest rollback flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "persistent manifest rollback initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "persistent manifest rollback inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "persistent manifest rollback cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "persistent manifest rollback verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "persistent manifest rollback used an invalid superblock phase",
            Self::Recovery(error) => error.as_str(),
            Self::State(error) => error.as_str(),
            Self::BindingMismatch => {
                "persistent manifest rollback record belongs to another trust anchor"
            }
            Self::Rollback { .. } => "signed manifest is below the persistent rollback floor",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "persistent manifest rollback write did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestRollbackEvidence {
    pub provisioned_before: bool,
    pub bootstrap_floor: u32,
    pub persisted_floor_before: u32,
    pub effective_floor: u32,
    pub artifact_index: u32,
    pub committed_floor: u32,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub floor_advanced: bool,
    pub record_written: bool,
    pub redundancy_repaired: bool,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub binding: ManifestRollbackBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestKeyRotationError {
    PartitionBounds,
    ZeroBootstrapFloor,
    ZeroArtifactIndex,
    ZeroLegacyKeyEpoch,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Recovery(RecoveryError),
    LegacyState(ManifestRollbackStateError),
    PolicyState(ManifestKeyPolicyStateError),
    LegacyBindingMismatch,
    PolicyDigestMismatch,
    ActiveKeyBindingMismatch,
    RetiredKey {
        artifact_epoch: u32,
        active_epoch: u32,
    },
    SkippedKeyEpoch {
        artifact_epoch: u32,
        active_epoch: u32,
    },
    TransitionWithoutFloorAdvance {
        artifact_index: u32,
        persisted_floor: u32,
    },
    Rollback {
        actual: u32,
        minimum: u32,
    },
    Encode(RecordError),
    VerificationFailed,
}

impl ManifestKeyRotationError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => {
                "persistent manifest key-policy slots are outside the data partition"
            }
            Self::ZeroBootstrapFloor => "manifest key-policy bootstrap floor is zero",
            Self::ZeroArtifactIndex => "manifest key-policy artifact index is zero",
            Self::ZeroLegacyKeyEpoch => "legacy manifest key epoch is zero",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "persistent manifest key-policy read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent manifest key-policy write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "persistent manifest key-policy flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "persistent manifest key-policy initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "persistent manifest key-policy inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "persistent manifest key-policy cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "persistent manifest key-policy verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "persistent manifest key-policy used an invalid superblock phase",
            Self::Recovery(error) => error.as_str(),
            Self::LegacyState(error) => error.as_str(),
            Self::PolicyState(error) => error.as_str(),
            Self::LegacyBindingMismatch => {
                "legacy rollback record is not the authorized key-policy predecessor"
            }
            Self::PolicyDigestMismatch => {
                "persistent manifest record belongs to another compiled keyring policy"
            }
            Self::ActiveKeyBindingMismatch => {
                "persistent manifest active-key digest or id changed within one epoch"
            }
            Self::RetiredKey { .. } => "signed manifest uses a retired key epoch",
            Self::SkippedKeyEpoch { .. } => "signed manifest skipped a required key epoch",
            Self::TransitionWithoutFloorAdvance { .. } => {
                "manifest key transition did not advance the rollback floor"
            }
            Self::Rollback { .. } => "signed manifest is below the persistent key-policy floor",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "persistent manifest key-policy write did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestKeyRotationEvidence {
    pub provisioned_before: bool,
    pub legacy_migrated: bool,
    pub key_transition: bool,
    pub previous_key_id: u8,
    pub previous_key_epoch: u32,
    pub active_key_id: u8,
    pub active_key_epoch: u32,
    pub bootstrap_floor: u32,
    pub persisted_floor_before: u32,
    pub effective_floor: u32,
    pub artifact_index: u32,
    pub committed_floor: u32,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub initial_active_policy_slots: u8,
    pub committed_active_policy_slots: u8,
    pub floor_advanced: bool,
    pub record_written: bool,
    pub redundancy_repaired: bool,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub binding: ManifestKeyPolicyBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestKeyRotationRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub bootstrap_floor: u32,
    pub artifact_index: u32,
    pub artifact_binding: ManifestKeyPolicyBinding,
    pub legacy_predecessor: ManifestRollbackBinding,
    pub legacy_predecessor_epoch: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceAuditError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    Recovery(RecoveryError),
    State(MaintenanceAuditStateError),
    BootstrapSequence {
        actual: u64,
    },
    Replay {
        actual: u64,
        minimum: u64,
    },
    SequenceGap {
        actual: u64,
        expected: u64,
    },
    NamespaceMismatch,
    SequenceExhausted,
    Encode(RecordError),
    VerificationFailed,
}

impl MaintenanceAuditError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "maintenance audit slots are outside the data partition",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "maintenance audit read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance audit write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance audit flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "maintenance audit initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "maintenance audit inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "maintenance audit cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "maintenance audit verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "maintenance audit used an invalid superblock phase",
            Self::Recovery(error) => error.as_str(),
            Self::State(error) => error.as_str(),
            Self::BootstrapSequence { .. } => "first maintenance authorization sequence is not one",
            Self::Replay { .. } => "maintenance authorization sequence was already consumed",
            Self::SequenceGap { .. } => "maintenance authorization skipped a sequence",
            Self::NamespaceMismatch => {
                "maintenance audit belongs to another root, policy, or device namespace"
            }
            Self::SequenceExhausted => "maintenance authorization sequence is exhausted",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "maintenance audit write did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuditEvidence {
    pub provisioned_before: bool,
    /// True only when M78 admitted the exact unfinished audit head without
    /// mutating the M77 audit ledger.
    pub resumed: bool,
    /// Durable execution head observed before admission (zero if absent).
    pub completed_sequence_before: u64,
    /// M78 cross-ledger admission reads performed before the audit result.
    pub preflight_reads: u8,
    pub execution_initial_generation: u64,
    pub execution_initial_slot: u8,
    pub execution_initial_valid_slots: u8,
    pub execution_initial_rejected_slots: u8,
    /// M79 read-only validation performed before M78 may mutate the audit.
    pub step_preflight_reads: u8,
    pub step_provisioned_before: bool,
    pub step_legacy_anchor: bool,
    pub step_sequence_before: u64,
    pub step_base_sequence_before: u64,
    pub step_entry_count_before: u64,
    pub step_ordinal_before: u32,
    pub step_initial_generation: u64,
    pub step_initial_slot: u8,
    pub step_initial_valid_slots: u8,
    pub step_initial_rejected_slots: u8,
    pub previous_sequence: u64,
    pub committed_sequence: u64,
    pub previous_accepted_count: u64,
    pub committed_accepted_count: u64,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub previous_chain_sha256: [u8; 32],
    pub committed_chain_sha256: [u8; 32],
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceAuditRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceExecutionError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    AuditRecovery(RecoveryError),
    AuditState(MaintenanceAuditStateError),
    ExecutionRecovery(RecoveryError),
    ExecutionState(MaintenanceExecutionStateError),
    AuditTransaction(MaintenanceAuditError),
    BootstrapSequence {
        actual: u64,
    },
    Replay {
        actual: u64,
        minimum: u64,
    },
    CompletedReplay {
        actual: u64,
        minimum: u64,
    },
    SequenceGap {
        actual: u64,
        expected: u64,
    },
    IncompletePredecessor {
        actual: u64,
        unfinished: u64,
    },
    AuthorizationBindingMismatch,
    NamespaceMismatch,
    LedgerDivergence,
    SequenceExhausted,
    Encode(RecordError),
    VerificationFailed,
}

impl MaintenanceExecutionError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "maintenance execution slots are outside the data partition",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "maintenance execution read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance execution write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance execution flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "maintenance execution initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "maintenance execution inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "maintenance execution cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "maintenance execution verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "maintenance execution used an invalid superblock phase",
            Self::AuditRecovery(error) | Self::ExecutionRecovery(error) => error.as_str(),
            Self::AuditState(error) => error.as_str(),
            Self::ExecutionState(error) => error.as_str(),
            Self::AuditTransaction(error) => error.as_str(),
            Self::BootstrapSequence { .. } => {
                "first maintenance execution authorization sequence is not one"
            }
            Self::Replay { .. } => "maintenance authorization predates the unfinished audit head",
            Self::CompletedReplay { .. } => {
                "maintenance authorization execution was already completed"
            }
            Self::SequenceGap { .. } => "maintenance authorization skipped an execution sequence",
            Self::IncompletePredecessor { .. } => {
                "maintenance authorization predecessor is not durably completed"
            }
            Self::AuthorizationBindingMismatch => {
                "maintenance resume authorization does not exactly match the unfinished audit head"
            }
            Self::NamespaceMismatch => {
                "maintenance execution belongs to another root, policy, or device namespace"
            }
            Self::LedgerDivergence => {
                "maintenance audit and execution ledgers violate their sequential invariant"
            }
            Self::SequenceExhausted => "maintenance execution sequence is exhausted",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "maintenance execution completion did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceExecutionCompletionRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub binding: MaintenanceExecutionCompletionBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceExecutionEvidence {
    pub provisioned_before: bool,
    pub previous_sequence: u64,
    pub committed_sequence: u64,
    pub previous_completed_count: u64,
    pub committed_completed_count: u64,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub previous_chain_sha256: [u8; 32],
    pub committed_chain_sha256: [u8; 32],
    pub audit_chain_sha256: [u8; 32],
    pub binding: MaintenanceExecutionCompletionBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceStepError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    AuditRecovery(RecoveryError),
    AuditState(MaintenanceAuditStateError),
    ExecutionRecovery(RecoveryError),
    ExecutionState(MaintenanceExecutionStateError),
    StepRecovery(RecoveryError),
    StepState(MaintenanceStepStateError),
    BootstrapStep {
        sequence: u64,
        step: u32,
    },
    Replay {
        actual: u64,
        minimum: u64,
    },
    SequenceGap {
        actual: u64,
        expected: u64,
    },
    StepGap {
        actual: u32,
        expected: u32,
    },
    AuthorizationBindingMismatch,
    NamespaceMismatch,
    LedgerDivergence,
    ExecutionAlreadyComplete,
    TerminalStepIncomplete,
    SequenceExhausted,
    Encode(RecordError),
    VerificationFailed,
}

impl MaintenanceStepError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "maintenance step slots are outside the data partition",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "maintenance step read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance step write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance step flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "maintenance step initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "maintenance step inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "maintenance step cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "maintenance step verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "maintenance step used an invalid superblock phase",
            Self::AuditRecovery(error)
            | Self::ExecutionRecovery(error)
            | Self::StepRecovery(error) => error.as_str(),
            Self::AuditState(error) => error.as_str(),
            Self::ExecutionState(error) => error.as_str(),
            Self::StepState(error) => error.as_str(),
            Self::BootstrapStep { .. } => {
                "maintenance step journal did not start at its first fixed step"
            }
            Self::Replay { .. } => "maintenance step request predates the selected journal head",
            Self::SequenceGap { .. } => "maintenance step request skipped an authorization",
            Self::StepGap { .. } => "maintenance step request skipped a fixed program step",
            Self::AuthorizationBindingMismatch => {
                "maintenance step authorization does not match the durable audit head"
            }
            Self::NamespaceMismatch => {
                "maintenance step journal belongs to another root, policy, or device namespace"
            }
            Self::LedgerDivergence => {
                "maintenance audit, execution, and step ledgers violate their sequential invariant"
            }
            Self::ExecutionAlreadyComplete => {
                "maintenance steps cannot mutate after execution completion"
            }
            Self::TerminalStepIncomplete => {
                "maintenance execution cannot complete before the durable drain step"
            }
            Self::SequenceExhausted => "maintenance step sequence or entry count is exhausted",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "maintenance step commit did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepAdmissionEvidence {
    pub provisioned_before: bool,
    pub legacy_anchor: bool,
    pub resumes_current_sequence: bool,
    pub audit_sequence: u64,
    pub execution_sequence: u64,
    pub step_sequence: u64,
    pub step_base_sequence: u64,
    pub step_entry_count: u64,
    pub step_ordinal: u32,
    pub step_generation: u64,
    pub step_slot: u8,
    pub step_valid_slots: u8,
    pub step_rejected_slots: u8,
    pub reads: u8,
    pub step_effect_sha256: [u8; 32],
    pub step_chain_sha256: [u8; 32],
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepCommitRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub binding: MaintenanceStepCommitBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepEvidence {
    pub provisioned_before: bool,
    pub replayed: bool,
    pub requested_sequence: u64,
    pub requested_step_ordinal: u32,
    pub previous_sequence: u64,
    pub committed_sequence: u64,
    pub base_sequence: u64,
    pub previous_entry_count: u64,
    pub committed_entry_count: u64,
    pub previous_step_ordinal: u32,
    pub committed_step_ordinal: u32,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub previous_chain_sha256: [u8; 32],
    pub committed_chain_sha256: [u8; 32],
    pub binding: MaintenanceStepCommitBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepTerminalEvidence {
    pub sequence: u64,
    pub base_sequence: u64,
    pub entry_count: u64,
    pub step_ordinal: u32,
    pub generation: u64,
    pub slot: u8,
    pub valid_slots: u8,
    pub rejected_slots: u8,
    pub reads: u8,
    pub chain_sha256: [u8; 32],
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenanceStepTerminalRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub authorization: MaintenanceAuthorizationAuditBinding,
    pub rotations_completed: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenancePlanError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    StepPreflight(MaintenanceStepError),
    PlanRecovery(RecoveryError),
    PlanState(MaintenancePlanStateError),
    BootstrapTransition,
    Replay {
        actual: u64,
        minimum: u64,
    },
    TransitionGap,
    InvalidTransition,
    AuthorizationBindingMismatch,
    LedgerDivergence,
    EffectNotApplied,
    EffectAlreadyApplied,
    PlanCompensated,
    TerminalPlanIncomplete,
    SequenceExhausted,
    Encode(RecordError),
    VerificationFailed,
}

impl MaintenancePlanError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "maintenance plan slots are outside the data partition",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "maintenance plan read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance plan write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "maintenance plan flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "maintenance plan initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "maintenance plan inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "maintenance plan cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "maintenance plan verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "maintenance plan used an invalid superblock phase",
            Self::StepPreflight(error) => error.as_str(),
            Self::PlanRecovery(error) => error.as_str(),
            Self::PlanState(error) => error.as_str(),
            Self::BootstrapTransition => {
                "maintenance plan did not begin with operation one prepared"
            }
            Self::Replay { .. } => "maintenance plan request predates the selected plan head",
            Self::TransitionGap => "maintenance plan request skipped a phase or operation",
            Self::InvalidTransition => "maintenance plan phase transition is not legal",
            Self::AuthorizationBindingMismatch => {
                "maintenance plan authorization does not exactly match its selected head"
            }
            Self::LedgerDivergence => {
                "maintenance audit, execution, step, and plan ledgers diverged"
            }
            Self::EffectNotApplied => {
                "maintenance plan cannot confirm before the exact M79 effect is durable"
            }
            Self::EffectAlreadyApplied => {
                "maintenance plan cannot prepare, apply, or compensate after the effect is durable"
            }
            Self::PlanCompensated => "maintenance plan was terminally compensated before apply",
            Self::TerminalPlanIncomplete => {
                "maintenance execution cannot complete before operation three is confirmed"
            }
            Self::SequenceExhausted => "maintenance plan sequence or transition count is exhausted",
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "maintenance plan commit did not survive flush and readback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanAdmissionEvidence {
    pub step: MaintenanceStepAdmissionEvidence,
    pub provisioned_before: bool,
    pub legacy_anchor: bool,
    pub resumes_current_sequence: bool,
    pub result_unknown: bool,
    pub effect_observed_unconfirmed: bool,
    pub sequence: u64,
    pub base_sequence: u64,
    pub transition_count: u64,
    pub operation_ordinal: u32,
    pub phase: u32,
    pub generation: u64,
    pub slot: u8,
    pub valid_slots: u8,
    pub rejected_slots: u8,
    pub reads: u8,
    pub effect_sha256: [u8; 32],
    pub operation_instance_id: [u8; 32],
    pub idempotency_key: [u8; 32],
    pub chain_sha256: [u8; 32],
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanTransitionRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub binding: MaintenancePlanTransitionBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanEvidence {
    pub provisioned_before: bool,
    pub replayed: bool,
    pub requested_sequence: u64,
    pub requested_operation_ordinal: u32,
    pub requested_phase: u32,
    pub previous_sequence: u64,
    pub committed_sequence: u64,
    pub base_sequence: u64,
    pub previous_transition_count: u64,
    pub committed_transition_count: u64,
    pub previous_operation_ordinal: u32,
    pub committed_operation_ordinal: u32,
    pub previous_phase: u32,
    pub committed_phase: u32,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub result_unknown_before: bool,
    pub effect_observed_before: bool,
    pub previous_chain_sha256: [u8; 32],
    pub committed_chain_sha256: [u8; 32],
    pub requested_binding: MaintenancePlanTransitionBinding,
    pub committed_binding: MaintenancePlanTransitionBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanTerminalRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub authorization: MaintenanceAuthorizationAuditBinding,
    pub expected_step_chain_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaintenancePlanTerminalEvidence {
    pub sequence: u64,
    pub base_sequence: u64,
    pub transition_count: u64,
    pub operation_ordinal: u32,
    pub phase: u32,
    pub generation: u64,
    pub slot: u8,
    pub valid_slots: u8,
    pub rejected_slots: u8,
    pub reads: u8,
    pub effect_sha256: [u8; 32],
    pub operation_instance_id: [u8; 32],
    pub idempotency_key: [u8; 32],
    pub chain_sha256: [u8; 32],
    pub binding: MaintenanceAuthorizationAuditBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignedMaintenancePlanError {
    PartitionBounds,
    Io {
        phase: IoPhase,
        error: PersistIoError,
    },
    PlanPreflight(MaintenancePlanError),
    ProgramRecovery(RecoveryError),
    ProgramState(SignedMaintenancePlanStateError),
    Replay {
        actual: u64,
        minimum: u64,
    },
    SequenceGap,
    ProgramBindingMismatch,
    LedgerDivergence,
    SequenceExhausted,
    Encode(RecordError),
    VerificationFailed,
    TerminalProgramIncomplete,
}

impl SignedMaintenancePlanError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartitionBounds => "signed maintenance-plan slots are outside the data partition",
            Self::Io {
                phase: IoPhase::ReadInitialSlot | IoPhase::ReadVerificationSlot,
                error: PersistIoError::RequiresReset,
            } => "signed maintenance-plan read requires a device reset",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                error: PersistIoError::OutcomeUnknown,
            } => "signed maintenance-plan write outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::OutcomeUnknown,
            } => "signed maintenance-plan flush outcome is unknown and requires reset",
            Self::Io {
                phase: IoPhase::ReadInitialSlot,
                ..
            } => "signed maintenance-plan initial record read failed",
            Self::Io {
                phase: IoPhase::WriteInactiveSlot,
                ..
            } => "signed maintenance-plan inactive-slot write failed",
            Self::Io {
                phase: IoPhase::Flush,
                ..
            } => "signed maintenance-plan cache flush failed",
            Self::Io {
                phase: IoPhase::ReadVerificationSlot,
                ..
            } => "signed maintenance-plan verification read failed",
            Self::Io {
                phase: IoPhase::ReadSuperblock,
                ..
            } => "signed maintenance-plan used an invalid superblock phase",
            Self::PlanPreflight(error) => error.as_str(),
            Self::ProgramRecovery(error) => error.as_str(),
            Self::ProgramState(error) => error.as_str(),
            Self::Replay { .. } => "signed maintenance-plan predates the selected program binding",
            Self::SequenceGap => "signed maintenance-plan skipped an authorization sequence",
            Self::ProgramBindingMismatch => {
                "signed maintenance-plan differs from the durable binding at this sequence"
            }
            Self::LedgerDivergence => {
                "signed maintenance-plan binding and M80 plan ledgers diverged"
            }
            Self::SequenceExhausted => {
                "signed maintenance-plan sequence or evidence count is exhausted"
            }
            Self::Encode(error) => error.as_str(),
            Self::VerificationFailed => {
                "signed maintenance-plan binding did not survive flush and readback"
            }
            Self::TerminalProgramIncomplete => {
                "maintenance execution cannot complete without its exact durable signed program"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub authorization: MaintenanceAuthorizationAuditBinding,
    pub binding: SignedMaintenancePlanBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanAdmissionEvidence {
    pub plan: MaintenancePlanAdmissionEvidence,
    pub provisioned_before: bool,
    pub exact_replay: bool,
    pub predecessor_complete: bool,
    pub sequence: u64,
    pub generation: u64,
    pub slot: u8,
    pub valid_slots: u8,
    pub rejected_slots: u8,
    pub reads: u8,
    pub chain_sha256: [u8; 32],
    pub binding: SignedMaintenancePlanBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanEvidence {
    pub provisioned_before: bool,
    pub replayed: bool,
    pub predecessor_complete: bool,
    pub previous_sequence: u64,
    pub committed_sequence: u64,
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub committed_valid_slots: u8,
    pub committed_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
    pub previous_chain_sha256: [u8; 32],
    pub committed_chain_sha256: [u8; 32],
    pub binding: SignedMaintenancePlanBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanTerminalRequest {
    pub partition_first_lba: u64,
    pub partition_sectors: u64,
    pub expected_format_epoch: FormatEpoch,
    pub authorization: MaintenanceAuthorizationAuditBinding,
    pub binding: SignedMaintenancePlanBinding,
    pub expected_plan_chain_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedMaintenancePlanTerminalEvidence {
    pub sequence: u64,
    pub generation: u64,
    pub slot: u8,
    pub valid_slots: u8,
    pub rejected_slots: u8,
    pub reads: u8,
    pub program_chain_sha256: [u8; 32],
    pub plan_chain_sha256: [u8; 32],
    pub binding: SignedMaintenancePlanBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdvanceEvidence {
    pub initial_generation: u64,
    pub committed_generation: u64,
    pub initial_slot: u8,
    pub committed_slot: u8,
    pub initial_valid_slots: u8,
    pub initial_rejected_slots: u8,
    pub reads: u8,
    pub writes: u8,
    pub flushes: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceHealthAdvanceEvidence {
    pub persistence: AdvanceEvidence,
    pub legacy_upgrade: bool,
    pub persisted_contract_present: bool,
    pub prior_boot_open: bool,
    pub contract_changed: bool,
    pub reprobe_required: bool,
    pub reprobe_verified: bool,
    pub current_boot_open: bool,
    pub offline_persisted: bool,
    pub offline_from_record: bool,
    pub current_contract: DeviceHealthContract,
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceHealthCloseEvidence {
    pub persistence: AdvanceEvidence,
    pub open_generation: u64,
    pub prior_boot_open: bool,
    pub contract_matched: bool,
    pub current_boot_open: bool,
    pub offline_persisted: bool,
    pub offline_from_record: bool,
    pub current_contract: DeviceHealthContract,
}

/// Enforces and, when necessary, advances the crash-recoverable manifest floor.
///
/// The caller must cryptographically verify the complete signed manifest
/// before entering this transaction. The two fixed slots are outside the
/// legacy boot/device-health slots but remain inside kernel-only
/// `BNDROID_DATA`. A mutation always writes the inactive slot, flushes, reads
/// both slots back, and proves that recovery selects the new record while the
/// previously selected sector remains byte-for-byte unchanged.
///
/// This protects against ordinary guest-visible downgrade and torn/corrupted
/// inactive records. The CRC-backed QEMU disk record is not RPMB/eFuse and
/// cannot resist a privileged host rolling the whole disk back or erasing it.
pub fn enforce_and_advance_manifest_rollback<D: DurableSectorIo>(
    io: &mut D,
    partition_first_lba: u64,
    partition_sectors: u64,
    expected_format_epoch: FormatEpoch,
    bootstrap_floor: u32,
    artifact_index: u32,
    binding: ManifestRollbackBinding,
) -> Result<ManifestRollbackEvidence, ManifestRollbackError> {
    if bootstrap_floor == 0 {
        return Err(ManifestRollbackError::ZeroBootstrapFloor);
    }
    if artifact_index == 0 {
        return Err(ManifestRollbackError::ZeroArtifactIndex);
    }
    binding.validate().map_err(ManifestRollbackError::State)?;
    validate_record_epoch(expected_format_epoch).map_err(ManifestRollbackError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(ManifestRollbackError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS
            .iter()
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(ManifestRollbackError::PartitionBounds);
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(ManifestRollbackError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| ManifestRollbackError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let unprovisioned = initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let recovered = if unprovisioned {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(ManifestRollbackError::Recovery)?,
        )
    };
    let persisted_state = recovered
        .map(|record| ManifestRollbackState::decode(record.payload))
        .transpose()
        .map_err(ManifestRollbackError::State)?;
    if persisted_state.is_some_and(|state| state.binding != binding) {
        return Err(ManifestRollbackError::BindingMismatch);
    }

    let provisioned_before = persisted_state.is_some();
    let persisted_floor_before = persisted_state.map_or(0, |state| state.floor);
    let effective_floor = bootstrap_floor.max(persisted_floor_before);
    if artifact_index < effective_floor {
        return Err(ManifestRollbackError::Rollback {
            actual: artifact_index,
            minimum: effective_floor,
        });
    }

    let initial_generation = recovered.map_or(0, |record| record.generation);
    let initial_slot = recovered.map_or(u8::MAX, |record| record.slot as u8);
    let initial_valid_slots = recovered.map_or(0, |record| record.valid_slots);
    let initial_rejected_slots = recovered.map_or(0, |record| record.rejected_slots);
    let floor_advanced = artifact_index > effective_floor;
    let redundancy_repaired = provisioned_before
        && artifact_index == persisted_floor_before
        && initial_rejected_slots > 0;
    let record_written = !provisioned_before
        || artifact_index > persisted_floor_before
        || initial_rejected_slots > 0;

    if !record_written {
        return Ok(ManifestRollbackEvidence {
            provisioned_before,
            bootstrap_floor,
            persisted_floor_before,
            effective_floor,
            artifact_index,
            committed_floor: persisted_floor_before,
            initial_generation,
            committed_generation: initial_generation,
            initial_slot,
            committed_slot: initial_slot,
            initial_valid_slots,
            initial_rejected_slots,
            committed_valid_slots: initial_valid_slots,
            committed_rejected_slots: initial_rejected_slots,
            floor_advanced,
            record_written,
            redundancy_repaired,
            reads: 2,
            writes: 0,
            flushes: 0,
            binding,
        });
    }

    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(ManifestRollbackError::Recovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_state = ManifestRollbackState {
        floor: artifact_index,
        binding,
    };
    let committed_payload = committed_state
        .encode()
        .map_err(ManifestRollbackError::State)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(ManifestRollbackError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(ManifestRollbackError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| ManifestRollbackError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| ManifestRollbackError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(ManifestRollbackError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| ManifestRollbackError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(ManifestRollbackError::Recovery)?;
    let verified_state =
        ManifestRollbackState::decode(verified.payload).map_err(ManifestRollbackError::State)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(ManifestRollbackError::VerificationFailed);
    }

    Ok(ManifestRollbackEvidence {
        provisioned_before,
        bootstrap_floor,
        persisted_floor_before,
        effective_floor,
        artifact_index,
        committed_floor: artifact_index,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        floor_advanced,
        record_written,
        redundancy_repaired,
        reads: 4,
        writes: 1,
        flushes: 1,
        binding,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExistingManifestKeyState {
    Legacy(ManifestRollbackState),
    Policy(ManifestKeyPolicyState),
}

fn decode_existing_manifest_key_state(
    payload: &[u8],
) -> Result<ExistingManifestKeyState, ManifestKeyRotationError> {
    if payload.len() >= MANIFEST_KEY_POLICY_STATE_MAGIC.len()
        && payload[..MANIFEST_KEY_POLICY_STATE_MAGIC.len()] == MANIFEST_KEY_POLICY_STATE_MAGIC
    {
        return ManifestKeyPolicyState::decode(payload)
            .map(ExistingManifestKeyState::Policy)
            .map_err(ManifestKeyRotationError::PolicyState);
    }
    ManifestRollbackState::decode(payload)
        .map(ExistingManifestKeyState::Legacy)
        .map_err(ManifestKeyRotationError::LegacyState)
}

fn manifest_policy_slot_count(
    slots: &[Sector; DATA_SLOT_COUNT],
    expected_format_epoch: FormatEpoch,
    expected: ManifestKeyPolicyState,
) -> u8 {
    slots
        .iter()
        .filter(|sector| {
            DataRecord::decode(sector, expected_format_epoch)
                .ok()
                .and_then(|record| ManifestKeyPolicyState::decode(record.payload).ok())
                == Some(expected)
        })
        .count() as u8
}

/// Migrates M75's single-key record into M76's ordered persistent key policy.
///
/// A cryptographically verified artifact may retain the active key epoch or
/// advance exactly one epoch while also raising the rollback floor. Reverse
/// transitions, skipped epochs, policy-namespace changes, and same-epoch key
/// substitutions fail before any write. Mixed old/new slots after a successful
/// transition are repaired on the next boot, followed by a read-only steady
/// state once both slots carry the active policy.
pub fn enforce_and_rotate_manifest_key_policy<D: DurableSectorIo>(
    io: &mut D,
    request: ManifestKeyRotationRequest,
) -> Result<ManifestKeyRotationEvidence, ManifestKeyRotationError> {
    let ManifestKeyRotationRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        bootstrap_floor,
        artifact_index,
        artifact_binding,
        legacy_predecessor,
        legacy_predecessor_epoch,
    } = request;
    if bootstrap_floor == 0 {
        return Err(ManifestKeyRotationError::ZeroBootstrapFloor);
    }
    if artifact_index == 0 {
        return Err(ManifestKeyRotationError::ZeroArtifactIndex);
    }
    if legacy_predecessor_epoch == 0 {
        return Err(ManifestKeyRotationError::ZeroLegacyKeyEpoch);
    }
    artifact_binding
        .validate()
        .map_err(ManifestKeyRotationError::PolicyState)?;
    legacy_predecessor
        .validate()
        .map_err(ManifestKeyRotationError::LegacyState)?;
    validate_record_epoch(expected_format_epoch).map_err(ManifestKeyRotationError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(ManifestKeyRotationError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS
            .iter()
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(ManifestKeyRotationError::PartitionBounds);
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(ManifestKeyRotationError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| ManifestKeyRotationError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let unprovisioned = initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let recovered = if unprovisioned {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(ManifestKeyRotationError::Recovery)?,
        )
    };
    let existing = recovered
        .map(|record| decode_existing_manifest_key_state(record.payload))
        .transpose()?;

    let (
        previous_key_id,
        previous_key_epoch,
        previous_trust_anchor,
        persisted_floor_before,
        legacy_migrated,
        selected_policy,
    ) = match existing {
        None => (0, 0, [0_u8; 32], 0, false, None),
        Some(ExistingManifestKeyState::Legacy(state)) => {
            if state.binding != legacy_predecessor {
                return Err(ManifestKeyRotationError::LegacyBindingMismatch);
            }
            (
                state.binding.key_id,
                legacy_predecessor_epoch,
                state.binding.trust_anchor_sha256,
                state.floor,
                true,
                None,
            )
        }
        Some(ExistingManifestKeyState::Policy(state)) => {
            if state.binding.policy_sha256 != artifact_binding.policy_sha256 {
                return Err(ManifestKeyRotationError::PolicyDigestMismatch);
            }
            (
                state.binding.key_id,
                state.binding.key_epoch,
                state.binding.trust_anchor_sha256,
                state.floor,
                false,
                Some(state),
            )
        }
    };

    if existing.is_some() {
        if artifact_binding.key_epoch < previous_key_epoch {
            return Err(ManifestKeyRotationError::RetiredKey {
                artifact_epoch: artifact_binding.key_epoch,
                active_epoch: previous_key_epoch,
            });
        }
        if artifact_binding.key_epoch > previous_key_epoch.saturating_add(1) {
            return Err(ManifestKeyRotationError::SkippedKeyEpoch {
                artifact_epoch: artifact_binding.key_epoch,
                active_epoch: previous_key_epoch,
            });
        }
        if artifact_binding.key_epoch == previous_key_epoch
            && (artifact_binding.key_id != previous_key_id
                || artifact_binding.trust_anchor_sha256 != previous_trust_anchor)
        {
            return Err(ManifestKeyRotationError::ActiveKeyBindingMismatch);
        }
        if artifact_binding.key_epoch > previous_key_epoch
            && artifact_index <= persisted_floor_before
        {
            return Err(ManifestKeyRotationError::TransitionWithoutFloorAdvance {
                artifact_index,
                persisted_floor: persisted_floor_before,
            });
        }
    }

    let effective_floor = bootstrap_floor.max(persisted_floor_before);
    if artifact_index < effective_floor {
        return Err(ManifestKeyRotationError::Rollback {
            actual: artifact_index,
            minimum: effective_floor,
        });
    }
    let committed_state = ManifestKeyPolicyState {
        floor: artifact_index,
        binding: artifact_binding,
    };
    let initial_active_policy_slots =
        manifest_policy_slot_count(&initial_slots, expected_format_epoch, committed_state);
    let provisioned_before = existing.is_some();
    let key_transition = provisioned_before && artifact_binding.key_epoch > previous_key_epoch;
    let floor_advanced = artifact_index > effective_floor;
    let redundancy_repaired = selected_policy == Some(committed_state)
        && initial_active_policy_slots < DATA_SLOT_COUNT as u8;
    let record_written = !provisioned_before
        || selected_policy != Some(committed_state)
        || initial_active_policy_slots < DATA_SLOT_COUNT as u8;
    let initial_generation = recovered.map_or(0, |record| record.generation);
    let initial_slot = recovered.map_or(u8::MAX, |record| record.slot as u8);
    let initial_valid_slots = recovered.map_or(0, |record| record.valid_slots);
    let initial_rejected_slots = recovered.map_or(0, |record| record.rejected_slots);

    if !record_written {
        return Ok(ManifestKeyRotationEvidence {
            provisioned_before,
            legacy_migrated,
            key_transition,
            previous_key_id,
            previous_key_epoch,
            active_key_id: artifact_binding.key_id,
            active_key_epoch: artifact_binding.key_epoch,
            bootstrap_floor,
            persisted_floor_before,
            effective_floor,
            artifact_index,
            committed_floor: persisted_floor_before,
            initial_generation,
            committed_generation: initial_generation,
            initial_slot,
            committed_slot: initial_slot,
            initial_valid_slots,
            initial_rejected_slots,
            initial_active_policy_slots,
            committed_active_policy_slots: initial_active_policy_slots,
            floor_advanced,
            record_written,
            redundancy_repaired,
            reads: 2,
            writes: 0,
            flushes: 0,
            binding: artifact_binding,
        });
    }

    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(ManifestKeyRotationError::Recovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_payload = committed_state
        .encode()
        .map_err(ManifestKeyRotationError::PolicyState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(ManifestKeyRotationError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(ManifestKeyRotationError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| ManifestKeyRotationError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| ManifestKeyRotationError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MANIFEST_ROLLBACK_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(ManifestKeyRotationError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| ManifestKeyRotationError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(ManifestKeyRotationError::Recovery)?;
    let verified_state = ManifestKeyPolicyState::decode(verified.payload)
        .map_err(ManifestKeyRotationError::PolicyState)?;
    let preserved_slot = committed_slot ^ 1;
    let committed_active_policy_slots =
        manifest_policy_slot_count(&verified_slots, expected_format_epoch, committed_state);
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
        || committed_active_policy_slots == 0
    {
        return Err(ManifestKeyRotationError::VerificationFailed);
    }

    Ok(ManifestKeyRotationEvidence {
        provisioned_before,
        legacy_migrated,
        key_transition,
        previous_key_id,
        previous_key_epoch,
        active_key_id: artifact_binding.key_id,
        active_key_epoch: artifact_binding.key_epoch,
        bootstrap_floor,
        persisted_floor_before,
        effective_floor,
        artifact_index,
        committed_floor: artifact_index,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        initial_active_policy_slots,
        committed_active_policy_slots,
        floor_advanced,
        record_written,
        redundancy_repaired,
        reads: 4,
        writes: 1,
        flushes: 1,
        binding: artifact_binding,
    })
}

/// Consumes one verified maintenance authorization into a durable hash chain.
///
/// The first accepted sequence must be one and every later sequence must be
/// exactly the prior value plus one. Replays, gaps, namespace substitutions,
/// and malformed state are rejected before any write. A successful commit
/// replaces only the inactive audit slot, flushes, reads both slots back, and
/// verifies both the selected state and byte-for-byte preservation of the old
/// slot. This QEMU-disk implementation is not a trusted monotonic backend; the
/// transaction boundary is intentionally suitable for a future RPMB adapter.
pub fn commit_maintenance_authorization<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceAuditEvidence, MaintenanceAuditError> {
    let MaintenanceAuditRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding.validate().map_err(MaintenanceAuditError::State)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenanceAuditError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceAuditError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceAuditError::PartitionBounds);
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceAuditError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| MaintenanceAuditError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let unprovisioned = initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let recovered = if unprovisioned {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceAuditError::Recovery)?,
        )
    };
    let existing = recovered
        .map(|record| MaintenanceAuditState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceAuditError::State)?;

    let (
        previous_sequence,
        previous_accepted_count,
        previous_chain_sha256,
        initial_generation,
        initial_slot,
        initial_valid_slots,
        initial_rejected_slots,
    ) = match (recovered, existing) {
        (None, None) => {
            if binding.sequence != 1 {
                return Err(MaintenanceAuditError::BootstrapSequence {
                    actual: binding.sequence,
                });
            }
            (0, 0, [0_u8; 32], 0, u8::MAX, 0, 0)
        }
        (Some(record), Some(state)) => {
            if state.root_sha256 != binding.root_sha256
                || state.policy_sha256 != binding.policy_sha256
                || state.device_binding_sha256 != binding.device_binding_sha256
            {
                return Err(MaintenanceAuditError::NamespaceMismatch);
            }
            let expected = state
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceAuditError::SequenceExhausted)?;
            if binding.sequence <= state.sequence {
                return Err(MaintenanceAuditError::Replay {
                    actual: binding.sequence,
                    minimum: expected,
                });
            }
            if binding.sequence != expected {
                return Err(MaintenanceAuditError::SequenceGap {
                    actual: binding.sequence,
                    expected,
                });
            }
            (
                state.sequence,
                state.accepted_count,
                state.chain_sha256,
                record.generation,
                record.slot as u8,
                record.valid_slots,
                record.rejected_slots,
            )
        }
        _ => unreachable!("maintenance recovery and state decode remain paired"),
    };
    let committed_accepted_count = previous_accepted_count
        .checked_add(1)
        .ok_or(MaintenanceAuditError::SequenceExhausted)?;
    let committed_chain_sha256 = maintenance_audit_chain_sha256(previous_chain_sha256, binding);
    let committed_state = MaintenanceAuditState {
        sequence: binding.sequence,
        accepted_count: committed_accepted_count,
        operations: binding.operations,
        max_uses: binding.max_uses,
        authorization_sha256: binding.authorization_sha256,
        authorization_id: binding.authorization_id,
        manifest_sha256: binding.manifest_sha256,
        policy_sha256: binding.policy_sha256,
        root_sha256: binding.root_sha256,
        device_binding_sha256: binding.device_binding_sha256,
        previous_chain_sha256,
        chain_sha256: committed_chain_sha256,
    };
    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(MaintenanceAuditError::Recovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_payload = committed_state
        .encode()
        .map_err(MaintenanceAuditError::State)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(MaintenanceAuditError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(MaintenanceAuditError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| MaintenanceAuditError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| MaintenanceAuditError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceAuditError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| MaintenanceAuditError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(MaintenanceAuditError::Recovery)?;
    let verified_state =
        MaintenanceAuditState::decode(verified.payload).map_err(MaintenanceAuditError::State)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(MaintenanceAuditError::VerificationFailed);
    }

    Ok(MaintenanceAuditEvidence {
        provisioned_before: existing.is_some(),
        resumed: false,
        completed_sequence_before: 0,
        preflight_reads: 0,
        execution_initial_generation: 0,
        execution_initial_slot: u8::MAX,
        execution_initial_valid_slots: 0,
        execution_initial_rejected_slots: 0,
        step_preflight_reads: 0,
        step_provisioned_before: false,
        step_legacy_anchor: false,
        step_sequence_before: 0,
        step_base_sequence_before: 0,
        step_entry_count_before: 0,
        step_ordinal_before: 0,
        step_initial_generation: 0,
        step_initial_slot: u8::MAX,
        step_initial_valid_slots: 0,
        step_initial_rejected_slots: 0,
        previous_sequence,
        committed_sequence: binding.sequence,
        previous_accepted_count,
        committed_accepted_count,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        reads: 4,
        writes: 1,
        flushes: 1,
        previous_chain_sha256,
        committed_chain_sha256,
        binding,
    })
}

/// Admits a verified BMA1 against both the authorization and execution heads.
///
/// A new authorization may advance the audit ledger only after its immediate
/// predecessor has an execution-completion record. If the audit head is one
/// sequence ahead of the completion head, only the exact same authorization
/// may resume, and that path is read-only for the audit ledger. Once both
/// heads match, the same sequence is a completed replay. The two disk ledgers
/// narrow QEMU interruption recovery but do not provide hardware rollback
/// resistance or a general exactly-once guarantee.
pub fn admit_maintenance_authorization_with_execution<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceAuditEvidence, MaintenanceExecutionError> {
    let MaintenanceAuditRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenanceExecutionError::AuditState)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenanceExecutionError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceExecutionError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .chain(MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter())
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceExecutionError::PartitionBounds);
    }

    let mut audit_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceExecutionError::PartitionBounds)?;
        io.read_sector(lba, &mut audit_slots[slot])
            .map_err(|error| MaintenanceExecutionError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let mut execution_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceExecutionError::PartitionBounds)?;
        io.read_sector(lba, &mut execution_slots[slot])
            .map_err(|error| MaintenanceExecutionError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }

    let audit_unprovisioned = audit_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let audit_recovered = if audit_unprovisioned {
        None
    } else {
        Some(
            recover([&audit_slots[0], &audit_slots[1]], expected_format_epoch)
                .map_err(MaintenanceExecutionError::AuditRecovery)?,
        )
    };
    let audit_state = audit_recovered
        .map(|record| MaintenanceAuditState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceExecutionError::AuditState)?;

    let execution_unprovisioned = execution_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let execution_recovered = if execution_unprovisioned {
        None
    } else {
        Some(
            recover(
                [&execution_slots[0], &execution_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceExecutionError::ExecutionRecovery)?,
        )
    };
    let execution_state = execution_recovered
        .map(|record| MaintenanceExecutionState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceExecutionError::ExecutionState)?;

    let completed_sequence_before = execution_state.map_or(0, |state| state.sequence);
    let execution_initial_generation = execution_recovered.map_or(0, |record| record.generation);
    let execution_initial_slot = execution_recovered.map_or(u8::MAX, |record| record.slot as u8);
    let execution_initial_valid_slots = execution_recovered.map_or(0, |record| record.valid_slots);
    let execution_initial_rejected_slots =
        execution_recovered.map_or(0, |record| record.rejected_slots);

    let mut committed = match (audit_recovered, audit_state, execution_state) {
        (None, None, None) => {
            if binding.sequence != 1 {
                return Err(MaintenanceExecutionError::BootstrapSequence {
                    actual: binding.sequence,
                });
            }
            commit_maintenance_authorization(
                io,
                MaintenanceAuditRequest {
                    partition_first_lba,
                    partition_sectors,
                    expected_format_epoch,
                    binding,
                },
            )
            .map_err(MaintenanceExecutionError::AuditTransaction)?
        }
        (None, None, Some(_)) => {
            return Err(MaintenanceExecutionError::LedgerDivergence);
        }
        (Some(audit_record), Some(audit), completion) => {
            let audit_binding = audit.binding();
            if audit.root_sha256 != binding.root_sha256
                || audit.policy_sha256 != binding.policy_sha256
                || audit.device_binding_sha256 != binding.device_binding_sha256
            {
                return Err(MaintenanceExecutionError::NamespaceMismatch);
            }

            if let Some(completed) = completion {
                if completed.root_sha256 != audit.root_sha256
                    || completed.policy_sha256 != audit.policy_sha256
                    || completed.device_binding_sha256 != audit.device_binding_sha256
                    || completed.sequence > audit.sequence
                    || audit
                        .sequence
                        .checked_sub(completed.sequence)
                        .unwrap_or(u64::MAX)
                        > 1
                    || (completed.sequence == audit.sequence
                        && completed.completion_binding().authorization_binding() != audit_binding)
                {
                    return Err(MaintenanceExecutionError::LedgerDivergence);
                }
            } else if audit.sequence != 1 {
                return Err(MaintenanceExecutionError::LedgerDivergence);
            }

            if binding.sequence < audit.sequence {
                return Err(MaintenanceExecutionError::Replay {
                    actual: binding.sequence,
                    minimum: audit.sequence,
                });
            }
            if binding.sequence == audit.sequence {
                if binding != audit_binding {
                    return Err(MaintenanceExecutionError::AuthorizationBindingMismatch);
                }
                if completed_sequence_before == audit.sequence {
                    let minimum = audit
                        .sequence
                        .checked_add(1)
                        .ok_or(MaintenanceExecutionError::SequenceExhausted)?;
                    return Err(MaintenanceExecutionError::CompletedReplay {
                        actual: binding.sequence,
                        minimum,
                    });
                }
                return Ok(MaintenanceAuditEvidence {
                    provisioned_before: true,
                    resumed: true,
                    completed_sequence_before,
                    preflight_reads: 4,
                    execution_initial_generation,
                    execution_initial_slot,
                    execution_initial_valid_slots,
                    execution_initial_rejected_slots,
                    step_preflight_reads: 0,
                    step_provisioned_before: false,
                    step_legacy_anchor: false,
                    step_sequence_before: 0,
                    step_base_sequence_before: 0,
                    step_entry_count_before: 0,
                    step_ordinal_before: 0,
                    step_initial_generation: 0,
                    step_initial_slot: u8::MAX,
                    step_initial_valid_slots: 0,
                    step_initial_rejected_slots: 0,
                    previous_sequence: audit.sequence,
                    committed_sequence: audit.sequence,
                    previous_accepted_count: audit.accepted_count,
                    committed_accepted_count: audit.accepted_count,
                    initial_generation: audit_record.generation,
                    committed_generation: audit_record.generation,
                    initial_slot: audit_record.slot as u8,
                    committed_slot: audit_record.slot as u8,
                    initial_valid_slots: audit_record.valid_slots,
                    initial_rejected_slots: audit_record.rejected_slots,
                    committed_valid_slots: audit_record.valid_slots,
                    committed_rejected_slots: audit_record.rejected_slots,
                    reads: 0,
                    writes: 0,
                    flushes: 0,
                    previous_chain_sha256: audit.previous_chain_sha256,
                    committed_chain_sha256: audit.chain_sha256,
                    binding,
                });
            }

            let expected = audit
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceExecutionError::SequenceExhausted)?;
            if binding.sequence != expected {
                return Err(MaintenanceExecutionError::SequenceGap {
                    actual: binding.sequence,
                    expected,
                });
            }
            if completed_sequence_before != audit.sequence {
                return Err(MaintenanceExecutionError::IncompletePredecessor {
                    actual: binding.sequence,
                    unfinished: audit.sequence,
                });
            }
            commit_maintenance_authorization(
                io,
                MaintenanceAuditRequest {
                    partition_first_lba,
                    partition_sectors,
                    expected_format_epoch,
                    binding,
                },
            )
            .map_err(MaintenanceExecutionError::AuditTransaction)?
        }
        _ => unreachable!("maintenance audit recovery and decode remain paired"),
    };

    committed.resumed = false;
    committed.completed_sequence_before = completed_sequence_before;
    committed.preflight_reads = 4;
    committed.execution_initial_generation = execution_initial_generation;
    committed.execution_initial_slot = execution_initial_slot;
    committed.execution_initial_valid_slots = execution_initial_valid_slots;
    committed.execution_initial_rejected_slots = execution_initial_rejected_slots;
    Ok(committed)
}

/// Seals one kernel-validated maintenance execution into the inactive M78 slot.
///
/// The current audit head must match every authorization field, and the
/// execution head must be its immediate predecessor. Completion is persisted
/// only after callers have supplied the exact bounded use/rotation counts,
/// drain and clean-shutdown flags, app-data generation, and a nonzero digest
/// over the runtime evidence.
pub fn commit_maintenance_execution_completion<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceExecutionCompletionRequest,
) -> Result<MaintenanceExecutionEvidence, MaintenanceExecutionError> {
    let MaintenanceExecutionCompletionRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenanceExecutionError::ExecutionState)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenanceExecutionError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceExecutionError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .chain(MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter())
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceExecutionError::PartitionBounds);
    }

    let mut audit_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceExecutionError::PartitionBounds)?;
        io.read_sector(lba, &mut audit_slots[slot])
            .map_err(|error| MaintenanceExecutionError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    if audit_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        return Err(MaintenanceExecutionError::LedgerDivergence);
    }
    let audit_recovered = recover([&audit_slots[0], &audit_slots[1]], expected_format_epoch)
        .map_err(MaintenanceExecutionError::AuditRecovery)?;
    let audit_state = MaintenanceAuditState::decode(audit_recovered.payload)
        .map_err(MaintenanceExecutionError::AuditState)?;
    if audit_state.binding() != binding.authorization_binding() {
        return Err(MaintenanceExecutionError::AuthorizationBindingMismatch);
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceExecutionError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| MaintenanceExecutionError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let unprovisioned = initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0));
    let recovered = if unprovisioned {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceExecutionError::ExecutionRecovery)?,
        )
    };
    let existing = recovered
        .map(|record| MaintenanceExecutionState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceExecutionError::ExecutionState)?;

    let (
        previous_sequence,
        previous_completed_count,
        previous_chain_sha256,
        initial_generation,
        initial_slot,
        initial_valid_slots,
        initial_rejected_slots,
    ) = match (recovered, existing) {
        (None, None) => {
            if binding.sequence != 1 {
                return Err(MaintenanceExecutionError::BootstrapSequence {
                    actual: binding.sequence,
                });
            }
            (0, 0, [0_u8; 32], 0, u8::MAX, 0, 0)
        }
        (Some(record), Some(state)) => {
            if state.root_sha256 != binding.root_sha256
                || state.policy_sha256 != binding.policy_sha256
                || state.device_binding_sha256 != binding.device_binding_sha256
            {
                return Err(MaintenanceExecutionError::NamespaceMismatch);
            }
            let expected = state
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceExecutionError::SequenceExhausted)?;
            if binding.sequence <= state.sequence {
                return Err(MaintenanceExecutionError::CompletedReplay {
                    actual: binding.sequence,
                    minimum: expected,
                });
            }
            if binding.sequence != expected {
                return Err(MaintenanceExecutionError::SequenceGap {
                    actual: binding.sequence,
                    expected,
                });
            }
            (
                state.sequence,
                state.completed_count,
                state.chain_sha256,
                record.generation,
                record.slot as u8,
                record.valid_slots,
                record.rejected_slots,
            )
        }
        _ => unreachable!("maintenance execution recovery and decode remain paired"),
    };
    if previous_sequence
        .checked_add(1)
        .ok_or(MaintenanceExecutionError::SequenceExhausted)?
        != audit_state.sequence
    {
        return Err(MaintenanceExecutionError::LedgerDivergence);
    }

    let committed_completed_count = previous_completed_count
        .checked_add(1)
        .ok_or(MaintenanceExecutionError::SequenceExhausted)?;
    let committed_chain_sha256 = maintenance_execution_chain_sha256(previous_chain_sha256, binding);
    let committed_state = MaintenanceExecutionState {
        sequence: binding.sequence,
        completed_count: committed_completed_count,
        operations: binding.operations,
        max_uses: binding.max_uses,
        uses_consumed: binding.uses_consumed,
        rotations_completed: binding.rotations_completed,
        flags: binding.flags,
        appdata_generation: binding.appdata_generation,
        authorization_sha256: binding.authorization_sha256,
        authorization_id: binding.authorization_id,
        manifest_sha256: binding.manifest_sha256,
        policy_sha256: binding.policy_sha256,
        root_sha256: binding.root_sha256,
        device_binding_sha256: binding.device_binding_sha256,
        previous_chain_sha256,
        chain_sha256: committed_chain_sha256,
        runtime_evidence_sha256: binding.runtime_evidence_sha256,
    };
    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(MaintenanceExecutionError::ExecutionRecovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_payload = committed_state
        .encode()
        .map_err(MaintenanceExecutionError::ExecutionState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(MaintenanceExecutionError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(MaintenanceExecutionError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| MaintenanceExecutionError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| MaintenanceExecutionError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceExecutionError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| MaintenanceExecutionError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(MaintenanceExecutionError::ExecutionRecovery)?;
    let verified_state = MaintenanceExecutionState::decode(verified.payload)
        .map_err(MaintenanceExecutionError::ExecutionState)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(MaintenanceExecutionError::VerificationFailed);
    }

    Ok(MaintenanceExecutionEvidence {
        provisioned_before: existing.is_some(),
        previous_sequence,
        committed_sequence: binding.sequence,
        previous_completed_count,
        committed_completed_count,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        reads: 6,
        writes: 1,
        flushes: 1,
        previous_chain_sha256,
        committed_chain_sha256,
        audit_chain_sha256: audit_state.chain_sha256,
        binding,
    })
}

/// Validates all three maintenance ledgers before M78 is allowed to advance
/// the authorization audit.
///
/// This read-only pass runs before any M79 mutation. It accepts an empty step
/// ledger as an explicit M78 migration anchor, but once `BNDRMST1` exists its
/// selected head must be terminal for the preceding authorization or belong
/// to the exact unfinished authorization being resumed.
pub fn preflight_maintenance_step_admission<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceStepAdmissionEvidence, MaintenanceStepError> {
    let MaintenanceAuditRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenanceStepError::AuditState)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenanceStepError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceStepError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .chain(MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter())
            .chain(MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter())
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceStepError::PartitionBounds);
    }

    let mut audit_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut audit_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let audit_recovered = if audit_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover([&audit_slots[0], &audit_slots[1]], expected_format_epoch)
                .map_err(MaintenanceStepError::AuditRecovery)?,
        )
    };
    let audit_state = audit_recovered
        .map(|record| MaintenanceAuditState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceStepError::AuditState)?;

    let mut execution_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut execution_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let execution_recovered = if execution_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&execution_slots[0], &execution_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceStepError::ExecutionRecovery)?,
        )
    };
    let execution_state = execution_recovered
        .map(|record| MaintenanceExecutionState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceStepError::ExecutionState)?;

    let mut step_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut step_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let step_recovered = if step_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover([&step_slots[0], &step_slots[1]], expected_format_epoch)
                .map_err(MaintenanceStepError::StepRecovery)?,
        )
    };
    let step_state = step_recovered
        .map(|record| MaintenanceStepState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceStepError::StepState)?;

    let audit_sequence = audit_state.map_or(0, |state| state.sequence);
    let execution_sequence = execution_state.map_or(0, |state| state.sequence);
    if let Some(audit) = audit_state {
        if audit.root_sha256 != binding.root_sha256
            || audit.policy_sha256 != binding.policy_sha256
            || audit.device_binding_sha256 != binding.device_binding_sha256
        {
            return Err(MaintenanceStepError::NamespaceMismatch);
        }
        if binding.sequence < audit.sequence {
            return Err(MaintenanceStepError::Replay {
                actual: binding.sequence,
                minimum: audit.sequence,
            });
        }
        if binding.sequence == audit.sequence && binding != audit.binding() {
            return Err(MaintenanceStepError::AuthorizationBindingMismatch);
        }
        let next = audit
            .sequence
            .checked_add(1)
            .ok_or(MaintenanceStepError::SequenceExhausted)?;
        if binding.sequence > next {
            return Err(MaintenanceStepError::SequenceGap {
                actual: binding.sequence,
                expected: next,
            });
        }
        match execution_state {
            Some(execution) => {
                if execution.root_sha256 != audit.root_sha256
                    || execution.policy_sha256 != audit.policy_sha256
                    || execution.device_binding_sha256 != audit.device_binding_sha256
                    || execution.sequence > audit.sequence
                    || audit.sequence - execution.sequence > 1
                    || (execution.sequence == audit.sequence
                        && execution.completion_binding().authorization_binding()
                            != audit.binding())
                {
                    return Err(MaintenanceStepError::LedgerDivergence);
                }
            }
            None if audit.sequence == 1 => {}
            None => return Err(MaintenanceStepError::LedgerDivergence),
        }
    } else if execution_state.is_some() || step_state.is_some() || binding.sequence != 1 {
        return Err(MaintenanceStepError::LedgerDivergence);
    }

    let mut legacy_anchor = false;
    let mut resumes_current_sequence = false;
    if let Some(step) = step_state {
        let audit = audit_state.ok_or(MaintenanceStepError::LedgerDivergence)?;
        if step.root_sha256 != audit.root_sha256
            || step.policy_sha256 != audit.policy_sha256
            || step.device_binding_sha256 != audit.device_binding_sha256
            || step.sequence > audit.sequence
        {
            return Err(MaintenanceStepError::LedgerDivergence);
        }
        if step.sequence == audit.sequence {
            if step.binding().authorization_binding() != audit.binding() {
                return Err(MaintenanceStepError::LedgerDivergence);
            }
            resumes_current_sequence = binding.sequence == audit.sequence;
            if execution_sequence == audit.sequence && step.step_ordinal != MAINTENANCE_STEP_DRAIN {
                return Err(MaintenanceStepError::LedgerDivergence);
            }
        } else if step.sequence == execution_sequence && step.step_ordinal == MAINTENANCE_STEP_DRAIN
        {
            // The selected step head is the terminal predecessor.
        } else {
            return Err(MaintenanceStepError::LedgerDivergence);
        }
        if binding.sequence
            == audit
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceStepError::SequenceExhausted)?
            && (step.sequence != audit.sequence || step.step_ordinal != MAINTENANCE_STEP_DRAIN)
        {
            return Err(MaintenanceStepError::TerminalStepIncomplete);
        }
    } else {
        legacy_anchor = audit_state.is_some() || execution_state.is_some();
    }

    Ok(MaintenanceStepAdmissionEvidence {
        provisioned_before: step_state.is_some(),
        legacy_anchor,
        resumes_current_sequence,
        audit_sequence,
        execution_sequence,
        step_sequence: step_state.map_or(0, |state| state.sequence),
        step_base_sequence: step_state.map_or(0, |state| state.base_sequence),
        step_entry_count: step_state.map_or(0, |state| state.entry_count),
        step_ordinal: step_state.map_or(0, |state| state.step_ordinal),
        step_generation: step_recovered.map_or(0, |record| record.generation),
        step_slot: step_recovered.map_or(u8::MAX, |record| record.slot as u8),
        step_valid_slots: step_recovered.map_or(0, |record| record.valid_slots),
        step_rejected_slots: step_recovered.map_or(0, |record| record.rejected_slots),
        reads: 6,
        step_effect_sha256: step_state.map_or([0; 32], |state| state.effect_sha256),
        step_chain_sha256: step_state.map_or([0; 32], |state| state.chain_sha256),
        binding,
    })
}

/// Commits or idempotently replays one fixed maintenance-program step.
///
/// Requests below the selected ordinal are accepted read-only only for the
/// exact same authorization and deterministic effect descriptor. A request
/// above the immediate next ordinal, a new authorization before the prior
/// drain, or any request after execution completion fails without mutation.
pub fn commit_maintenance_step<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceStepCommitRequest,
) -> Result<MaintenanceStepEvidence, MaintenanceStepError> {
    let MaintenanceStepCommitRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenanceStepError::StepState)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenanceStepError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceStepError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .chain(MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter())
            .chain(MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter())
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceStepError::PartitionBounds);
    }

    let mut audit_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut audit_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    if audit_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        return Err(MaintenanceStepError::LedgerDivergence);
    }
    let audit_recovered = recover([&audit_slots[0], &audit_slots[1]], expected_format_epoch)
        .map_err(MaintenanceStepError::AuditRecovery)?;
    let audit_state = MaintenanceAuditState::decode(audit_recovered.payload)
        .map_err(MaintenanceStepError::AuditState)?;
    if audit_state.binding() != binding.authorization_binding() {
        return Err(MaintenanceStepError::AuthorizationBindingMismatch);
    }

    let mut execution_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_EXECUTION_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut execution_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let execution_recovered = if execution_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&execution_slots[0], &execution_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceStepError::ExecutionRecovery)?,
        )
    };
    let execution_state = execution_recovered
        .map(|record| MaintenanceExecutionState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceStepError::ExecutionState)?;
    match execution_state {
        Some(state) if state.sequence == binding.sequence => {
            return Err(MaintenanceStepError::ExecutionAlreadyComplete);
        }
        Some(state)
            if state
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceStepError::SequenceExhausted)?
                != binding.sequence =>
        {
            return Err(MaintenanceStepError::LedgerDivergence);
        }
        None if binding.sequence != 1 => {
            return Err(MaintenanceStepError::LedgerDivergence);
        }
        _ => {}
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let recovered = if initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenanceStepError::StepRecovery)?,
        )
    };
    let existing = recovered
        .map(|record| MaintenanceStepState::decode(record.payload))
        .transpose()
        .map_err(MaintenanceStepError::StepState)?;

    if let (Some(record), Some(state)) = (recovered, existing) {
        if state.root_sha256 != binding.root_sha256
            || state.policy_sha256 != binding.policy_sha256
            || state.device_binding_sha256 != binding.device_binding_sha256
        {
            return Err(MaintenanceStepError::NamespaceMismatch);
        }
        if binding.sequence < state.sequence {
            return Err(MaintenanceStepError::Replay {
                actual: binding.sequence,
                minimum: state.sequence,
            });
        }
        if binding.sequence == state.sequence {
            if state.binding().authorization_binding() != binding.authorization_binding() {
                return Err(MaintenanceStepError::AuthorizationBindingMismatch);
            }
            if binding.step_ordinal <= state.step_ordinal {
                return Ok(MaintenanceStepEvidence {
                    provisioned_before: true,
                    replayed: true,
                    requested_sequence: binding.sequence,
                    requested_step_ordinal: binding.step_ordinal,
                    previous_sequence: state.sequence,
                    committed_sequence: state.sequence,
                    base_sequence: state.base_sequence,
                    previous_entry_count: state.entry_count,
                    committed_entry_count: state.entry_count,
                    previous_step_ordinal: state.step_ordinal,
                    committed_step_ordinal: state.step_ordinal,
                    initial_generation: record.generation,
                    committed_generation: record.generation,
                    initial_slot: record.slot as u8,
                    committed_slot: record.slot as u8,
                    initial_valid_slots: record.valid_slots,
                    initial_rejected_slots: record.rejected_slots,
                    committed_valid_slots: record.valid_slots,
                    committed_rejected_slots: record.rejected_slots,
                    reads: 6,
                    writes: 0,
                    flushes: 0,
                    previous_chain_sha256: state.previous_chain_sha256,
                    committed_chain_sha256: state.chain_sha256,
                    binding,
                });
            }
            let expected = state
                .step_ordinal
                .checked_add(1)
                .ok_or(MaintenanceStepError::SequenceExhausted)?;
            if binding.step_ordinal != expected {
                return Err(MaintenanceStepError::StepGap {
                    actual: binding.step_ordinal,
                    expected,
                });
            }
        } else {
            let expected_sequence = state
                .sequence
                .checked_add(1)
                .ok_or(MaintenanceStepError::SequenceExhausted)?;
            if binding.sequence != expected_sequence {
                return Err(MaintenanceStepError::SequenceGap {
                    actual: binding.sequence,
                    expected: expected_sequence,
                });
            }
            if state.step_ordinal != MAINTENANCE_STEP_DRAIN
                || binding.step_ordinal != MAINTENANCE_STEP_ROTATION_ONE
            {
                return Err(MaintenanceStepError::TerminalStepIncomplete);
            }
        }
    } else if binding.step_ordinal != MAINTENANCE_STEP_ROTATION_ONE {
        return Err(MaintenanceStepError::BootstrapStep {
            sequence: binding.sequence,
            step: binding.step_ordinal,
        });
    }

    let (
        previous_sequence,
        base_sequence,
        previous_entry_count,
        previous_step_ordinal,
        previous_chain_sha256,
        initial_generation,
        initial_slot,
        initial_valid_slots,
        initial_rejected_slots,
    ) = match (recovered, existing) {
        (Some(record), Some(state)) => (
            state.sequence,
            state.base_sequence,
            state.entry_count,
            state.step_ordinal,
            state.chain_sha256,
            record.generation,
            record.slot as u8,
            record.valid_slots,
            record.rejected_slots,
        ),
        (None, None) => (0, binding.sequence, 0, 0, [0; 32], 0, u8::MAX, 0, 0),
        _ => unreachable!("maintenance step recovery and decode remain paired"),
    };
    let committed_entry_count = previous_entry_count
        .checked_add(1)
        .ok_or(MaintenanceStepError::SequenceExhausted)?;
    let committed_chain_sha256 = maintenance_step_chain_sha256(
        previous_chain_sha256,
        base_sequence,
        committed_entry_count,
        binding,
    );
    let committed_state = MaintenanceStepState {
        sequence: binding.sequence,
        base_sequence,
        entry_count: committed_entry_count,
        operations: binding.operations,
        max_uses: binding.max_uses,
        step_ordinal: binding.step_ordinal,
        observed_rotations: binding.observed_rotations,
        flags: binding.flags,
        authorization_sha256: binding.authorization_sha256,
        authorization_id: binding.authorization_id,
        manifest_sha256: binding.manifest_sha256,
        policy_sha256: binding.policy_sha256,
        root_sha256: binding.root_sha256,
        device_binding_sha256: binding.device_binding_sha256,
        effect_sha256: binding.effect_sha256,
        previous_chain_sha256,
        chain_sha256: committed_chain_sha256,
    };
    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(MaintenanceStepError::StepRecovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_payload = committed_state
        .encode()
        .map_err(MaintenanceStepError::StepState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(MaintenanceStepError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MAINTENANCE_STEP_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(MaintenanceStepError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| MaintenanceStepError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| MaintenanceStepError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(MaintenanceStepError::StepRecovery)?;
    let verified_state =
        MaintenanceStepState::decode(verified.payload).map_err(MaintenanceStepError::StepState)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(MaintenanceStepError::VerificationFailed);
    }

    Ok(MaintenanceStepEvidence {
        provisioned_before: existing.is_some(),
        replayed: false,
        requested_sequence: binding.sequence,
        requested_step_ordinal: binding.step_ordinal,
        previous_sequence,
        committed_sequence: binding.sequence,
        base_sequence,
        previous_entry_count,
        committed_entry_count,
        previous_step_ordinal,
        committed_step_ordinal: binding.step_ordinal,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        reads: 8,
        writes: 1,
        flushes: 1,
        previous_chain_sha256,
        committed_chain_sha256,
        binding,
    })
}

/// Reads and binds the selected terminal drain step immediately before M78's
/// aggregate execution-completion transaction.
pub fn validate_maintenance_step_terminal<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceStepTerminalRequest,
) -> Result<MaintenanceStepTerminalEvidence, MaintenanceStepError> {
    let MaintenanceStepTerminalRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        authorization,
        rotations_completed,
    } = request;
    authorization
        .validate()
        .map_err(MaintenanceStepError::AuditState)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenanceStepError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS
            .iter()
            .chain(MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter())
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenanceStepError::PartitionBounds);
    }

    let mut audit_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_AUDIT_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut audit_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let audit = recover([&audit_slots[0], &audit_slots[1]], expected_format_epoch)
        .map_err(MaintenanceStepError::AuditRecovery)?;
    let audit_state =
        MaintenanceAuditState::decode(audit.payload).map_err(MaintenanceStepError::AuditState)?;
    if audit_state.binding() != authorization {
        return Err(MaintenanceStepError::AuthorizationBindingMismatch);
    }

    let mut step_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_STEP_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenanceStepError::PartitionBounds)?;
        io.read_sector(lba, &mut step_slots[slot])
            .map_err(|error| MaintenanceStepError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    if step_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        return Err(MaintenanceStepError::TerminalStepIncomplete);
    }
    let step = recover([&step_slots[0], &step_slots[1]], expected_format_epoch)
        .map_err(MaintenanceStepError::StepRecovery)?;
    let state =
        MaintenanceStepState::decode(step.payload).map_err(MaintenanceStepError::StepState)?;
    if state.binding().authorization_binding() != authorization
        || state.sequence != authorization.sequence
        || state.step_ordinal != MAINTENANCE_STEP_DRAIN
        || state.observed_rotations != rotations_completed
        || state.flags
            != (MAINTENANCE_STEP_REPLAY_SAFE_FROM_BOOT_START | MAINTENANCE_STEP_DRAIN_VALIDATED)
    {
        return Err(MaintenanceStepError::TerminalStepIncomplete);
    }

    Ok(MaintenanceStepTerminalEvidence {
        sequence: state.sequence,
        base_sequence: state.base_sequence,
        entry_count: state.entry_count,
        step_ordinal: state.step_ordinal,
        generation: step.generation,
        slot: step.slot as u8,
        valid_slots: step.valid_slots,
        rejected_slots: step.rejected_slots,
        reads: 4,
        chain_sha256: state.chain_sha256,
        binding: state.binding().authorization_binding(),
    })
}

/// Validates the M77-M80 ledger set before the authorization audit may move.
///
/// An empty `BNDRMPL1` ledger is accepted only alongside an empty M79 step
/// ledger at a fresh sequence-one bootstrap or immediately after a durably
/// completed predecessor. Once provisioned, the plan head must describe the
/// exact authorization and be reconcilable with the selected M79 step head.
pub fn preflight_maintenance_plan_admission<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenanceAuditRequest,
) -> Result<MaintenancePlanAdmissionEvidence, MaintenancePlanError> {
    let MaintenanceAuditRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenancePlanStateError::Authorization)
        .map_err(MaintenancePlanError::PlanState)?;
    validate_record_epoch(expected_format_epoch).map_err(MaintenancePlanError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(MaintenancePlanError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS
            .iter()
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(MaintenancePlanError::PartitionBounds);
    }

    let step = preflight_maintenance_step_admission(
        io,
        MaintenanceAuditRequest {
            partition_first_lba,
            partition_sectors,
            expected_format_epoch,
            binding,
        },
    )
    .map_err(MaintenancePlanError::StepPreflight)?;

    let mut plan_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut plan_slots[slot])
            .map_err(|error| MaintenancePlanError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let recovered = if plan_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover([&plan_slots[0], &plan_slots[1]], expected_format_epoch)
                .map_err(MaintenancePlanError::PlanRecovery)?,
        )
    };
    let state = recovered
        .map(|record| MaintenancePlanState::decode(record.payload))
        .transpose()
        .map_err(MaintenancePlanError::PlanState)?;

    let mut legacy_anchor = false;
    let mut resumes_current_sequence = false;
    let mut result_unknown = false;
    let mut effect_observed_unconfirmed = false;
    if let Some(plan) = state {
        let plan_authorization = plan.binding().authorization_binding();
        if binding.sequence < plan.sequence {
            return Err(MaintenancePlanError::Replay {
                actual: binding.sequence,
                minimum: plan.sequence,
            });
        }
        if binding.sequence == plan.sequence {
            if binding != plan_authorization {
                return Err(MaintenancePlanError::AuthorizationBindingMismatch);
            }
            resumes_current_sequence = true;
        } else {
            let next = plan
                .sequence
                .checked_add(1)
                .ok_or(MaintenancePlanError::SequenceExhausted)?;
            if binding.sequence != next
                || plan.operation_ordinal != MAINTENANCE_STEP_DRAIN
                || plan.phase != MAINTENANCE_PLAN_PHASE_CONFIRMED
                || step.execution_sequence != plan.sequence
                || step.step_sequence != plan.sequence
                || step.step_ordinal != MAINTENANCE_STEP_DRAIN
            {
                return Err(MaintenancePlanError::LedgerDivergence);
            }
        }

        if plan.sequence > step.audit_sequence
            || (plan.sequence != step.audit_sequence && plan.sequence != step.execution_sequence)
        {
            return Err(MaintenancePlanError::LedgerDivergence);
        }
        let current_step_ordinal = if step.step_sequence == plan.sequence {
            step.step_ordinal
        } else {
            0
        };
        let prior_step_ordinal = plan.operation_ordinal - 1;
        match plan.phase {
            MAINTENANCE_PLAN_PHASE_PREPARED | MAINTENANCE_PLAN_PHASE_COMPENSATED => {
                if current_step_ordinal != prior_step_ordinal {
                    return Err(MaintenancePlanError::LedgerDivergence);
                }
            }
            MAINTENANCE_PLAN_PHASE_APPLYING => {
                if current_step_ordinal != prior_step_ordinal
                    && current_step_ordinal != plan.operation_ordinal
                {
                    return Err(MaintenancePlanError::LedgerDivergence);
                }
                result_unknown = current_step_ordinal == prior_step_ordinal;
                effect_observed_unconfirmed = current_step_ordinal == plan.operation_ordinal;
            }
            MAINTENANCE_PLAN_PHASE_CONFIRMED => {
                if current_step_ordinal < plan.operation_ordinal {
                    return Err(MaintenancePlanError::LedgerDivergence);
                }
            }
            _ => return Err(MaintenancePlanError::InvalidTransition),
        }
        if step.execution_sequence == plan.sequence
            && (plan.operation_ordinal != MAINTENANCE_STEP_DRAIN
                || plan.phase != MAINTENANCE_PLAN_PHASE_CONFIRMED)
        {
            return Err(MaintenancePlanError::LedgerDivergence);
        }
    } else {
        let fresh = binding.sequence == 1
            && step.audit_sequence == 0
            && step.execution_sequence == 0
            && !step.provisioned_before;
        let completed_predecessor = step
            .execution_sequence
            .checked_add(1)
            .is_some_and(|next| next == binding.sequence)
            && (step.audit_sequence == step.execution_sequence
                || step.audit_sequence == binding.sequence)
            && step.legacy_anchor
            && !step.provisioned_before;
        if !fresh && !completed_predecessor {
            return Err(MaintenancePlanError::LedgerDivergence);
        }
        legacy_anchor = completed_predecessor;
    }

    Ok(MaintenancePlanAdmissionEvidence {
        step,
        provisioned_before: state.is_some(),
        legacy_anchor,
        resumes_current_sequence,
        result_unknown,
        effect_observed_unconfirmed,
        sequence: state.map_or(0, |plan| plan.sequence),
        base_sequence: state.map_or(0, |plan| plan.base_sequence),
        transition_count: state.map_or(0, |plan| plan.transition_count),
        operation_ordinal: state.map_or(0, |plan| plan.operation_ordinal),
        phase: state.map_or(0, |plan| plan.phase),
        generation: recovered.map_or(0, |record| record.generation),
        slot: recovered.map_or(u8::MAX, |record| record.slot as u8),
        valid_slots: recovered.map_or(0, |record| record.valid_slots),
        rejected_slots: recovered.map_or(0, |record| record.rejected_slots),
        reads: step
            .reads
            .checked_add(2)
            .ok_or(MaintenancePlanError::SequenceExhausted)?,
        effect_sha256: state.map_or([0; 32], |plan| plan.effect_sha256),
        operation_instance_id: state.map_or([0; 32], |plan| plan.operation_instance_id),
        idempotency_key: state.map_or([0; 32], |plan| plan.idempotency_key),
        chain_sha256: state.map_or([0; 32], |plan| plan.chain_sha256),
        binding,
    })
}

fn maintenance_plan_requested_transition_count(
    base_sequence: u64,
    binding: MaintenancePlanTransitionBinding,
) -> Result<u64, MaintenancePlanError> {
    let phase_position = maintenance_plan_phase_position(binding.phase)
        .ok_or(MaintenancePlanError::InvalidTransition)?;
    binding
        .sequence
        .checked_sub(base_sequence)
        .and_then(|delta| delta.checked_mul(MAINTENANCE_PLAN_TRANSITIONS_PER_SEQUENCE))
        .and_then(|count| {
            count.checked_add(u64::from(binding.operation_ordinal - 1).checked_mul(3)?)
        })
        .and_then(|count| count.checked_add(phase_position))
        .ok_or(MaintenancePlanError::SequenceExhausted)
}

fn maintenance_plan_transition_is_next(
    previous: MaintenancePlanTransitionBinding,
    requested: MaintenancePlanTransitionBinding,
) -> bool {
    if requested.sequence == previous.sequence {
        match (previous.phase, requested.phase) {
            (MAINTENANCE_PLAN_PHASE_PREPARED, MAINTENANCE_PLAN_PHASE_APPLYING)
            | (MAINTENANCE_PLAN_PHASE_PREPARED, MAINTENANCE_PLAN_PHASE_COMPENSATED)
            | (MAINTENANCE_PLAN_PHASE_APPLYING, MAINTENANCE_PLAN_PHASE_CONFIRMED) => {
                requested.operation_ordinal == previous.operation_ordinal
            }
            (MAINTENANCE_PLAN_PHASE_CONFIRMED, MAINTENANCE_PLAN_PHASE_PREPARED) => {
                previous.operation_ordinal < MAINTENANCE_STEP_DRAIN
                    && requested.operation_ordinal == previous.operation_ordinal + 1
            }
            _ => false,
        }
    } else {
        previous.phase == MAINTENANCE_PLAN_PHASE_CONFIRMED
            && previous.operation_ordinal == MAINTENANCE_STEP_DRAIN
            && previous.sequence.checked_add(1) == Some(requested.sequence)
            && requested.operation_ordinal == MAINTENANCE_STEP_ROTATION_ONE
            && requested.phase == MAINTENANCE_PLAN_PHASE_PREPARED
    }
}

/// Commits or read-only replays one M80 maintenance-plan phase.
///
/// Every mutating path first revalidates all four maintenance ledgers. The
/// `APPLYING` intent is durable before the resident effect, while
/// `CONFIRMED` is legal only after the matching M79 effect is selected from
/// disk. Compensation is terminal and legal only directly after `PREPARED`.
pub fn commit_maintenance_plan_transition<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenancePlanTransitionRequest,
) -> Result<MaintenancePlanEvidence, MaintenancePlanError> {
    let MaintenancePlanTransitionRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        binding,
    } = request;
    binding
        .validate()
        .map_err(MaintenancePlanError::PlanState)?;
    let admission = preflight_maintenance_plan_admission(
        io,
        MaintenanceAuditRequest {
            partition_first_lba,
            partition_sectors,
            expected_format_epoch,
            binding: binding.authorization_binding(),
        },
    )?;

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| MaintenancePlanError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let recovered = if initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                expected_format_epoch,
            )
            .map_err(MaintenancePlanError::PlanRecovery)?,
        )
    };
    let existing = recovered
        .map(|record| MaintenancePlanState::decode(record.payload))
        .transpose()
        .map_err(MaintenancePlanError::PlanState)?;

    let (
        base_sequence,
        previous_sequence,
        previous_transition_count,
        previous_operation_ordinal,
        previous_phase,
        previous_chain_sha256,
        initial_generation,
        initial_slot,
        initial_valid_slots,
        initial_rejected_slots,
    ) = if let (Some(record), Some(state)) = (recovered, existing) {
        if binding.sequence < state.sequence {
            return Err(MaintenancePlanError::Replay {
                actual: binding.sequence,
                minimum: state.sequence,
            });
        }
        if binding.sequence == state.sequence
            && binding.authorization_binding() != state.binding().authorization_binding()
        {
            return Err(MaintenancePlanError::AuthorizationBindingMismatch);
        }
        let requested_count =
            maintenance_plan_requested_transition_count(state.base_sequence, binding)?;
        if binding.sequence == state.sequence && requested_count <= state.transition_count {
            let exact = requested_count == state.transition_count && binding == state.binding();
            let prefix = requested_count < state.transition_count
                && (state.phase != MAINTENANCE_PLAN_PHASE_COMPENSATED
                    || (binding.operation_ordinal == state.operation_ordinal
                        && binding.phase == MAINTENANCE_PLAN_PHASE_PREPARED));
            if !exact && !prefix {
                return Err(if state.phase == MAINTENANCE_PLAN_PHASE_COMPENSATED {
                    MaintenancePlanError::PlanCompensated
                } else {
                    MaintenancePlanError::InvalidTransition
                });
            }
            let reads = admission
                .reads
                .checked_add(2)
                .ok_or(MaintenancePlanError::SequenceExhausted)?;
            return Ok(MaintenancePlanEvidence {
                provisioned_before: true,
                replayed: true,
                requested_sequence: binding.sequence,
                requested_operation_ordinal: binding.operation_ordinal,
                requested_phase: binding.phase,
                previous_sequence: state.sequence,
                committed_sequence: state.sequence,
                base_sequence: state.base_sequence,
                previous_transition_count: state.transition_count,
                committed_transition_count: state.transition_count,
                previous_operation_ordinal: state.operation_ordinal,
                committed_operation_ordinal: state.operation_ordinal,
                previous_phase: state.phase,
                committed_phase: state.phase,
                initial_generation: record.generation,
                committed_generation: record.generation,
                initial_slot: record.slot as u8,
                committed_slot: record.slot as u8,
                initial_valid_slots: record.valid_slots,
                initial_rejected_slots: record.rejected_slots,
                committed_valid_slots: record.valid_slots,
                committed_rejected_slots: record.rejected_slots,
                reads,
                writes: 0,
                flushes: 0,
                result_unknown_before: admission.result_unknown,
                effect_observed_before: admission.effect_observed_unconfirmed,
                previous_chain_sha256: state.chain_sha256,
                committed_chain_sha256: state.chain_sha256,
                requested_binding: binding,
                committed_binding: state.binding(),
            });
        }
        if state.phase == MAINTENANCE_PLAN_PHASE_COMPENSATED {
            return Err(MaintenancePlanError::PlanCompensated);
        }
        if !maintenance_plan_transition_is_next(state.binding(), binding)
            || requested_count
                != state
                    .transition_count
                    .checked_add(1)
                    .ok_or(MaintenancePlanError::SequenceExhausted)?
        {
            return Err(MaintenancePlanError::TransitionGap);
        }
        (
            state.base_sequence,
            state.sequence,
            state.transition_count,
            state.operation_ordinal,
            state.phase,
            state.chain_sha256,
            record.generation,
            record.slot as u8,
            record.valid_slots,
            record.rejected_slots,
        )
    } else {
        if binding.operation_ordinal != MAINTENANCE_STEP_ROTATION_ONE
            || binding.phase != MAINTENANCE_PLAN_PHASE_PREPARED
            || (!admission.legacy_anchor && binding.sequence != 1)
        {
            return Err(MaintenancePlanError::BootstrapTransition);
        }
        (binding.sequence, 0, 0, 0, 0, [0; 32], 0, u8::MAX, 0, 0)
    };

    let current_step_ordinal = if admission.step.step_sequence == binding.sequence {
        admission.step.step_ordinal
    } else {
        0
    };
    match binding.phase {
        MAINTENANCE_PLAN_PHASE_PREPARED => {
            if current_step_ordinal != binding.operation_ordinal - 1 {
                return Err(MaintenancePlanError::EffectAlreadyApplied);
            }
        }
        MAINTENANCE_PLAN_PHASE_APPLYING | MAINTENANCE_PLAN_PHASE_COMPENSATED => {
            if current_step_ordinal != binding.operation_ordinal - 1 {
                return Err(MaintenancePlanError::EffectAlreadyApplied);
            }
        }
        MAINTENANCE_PLAN_PHASE_CONFIRMED => {
            if admission.step.step_sequence != binding.sequence
                || admission.step.step_ordinal != binding.operation_ordinal
                || admission.step.step_effect_sha256 != binding.effect_sha256
            {
                return Err(MaintenancePlanError::EffectNotApplied);
            }
        }
        _ => return Err(MaintenancePlanError::InvalidTransition),
    }

    let committed_transition_count =
        maintenance_plan_requested_transition_count(base_sequence, binding)?;
    let committed_chain_sha256 = maintenance_plan_chain_sha256(
        previous_chain_sha256,
        base_sequence,
        committed_transition_count,
        binding,
    );
    let committed_state = MaintenancePlanState {
        sequence: binding.sequence,
        base_sequence,
        transition_count: committed_transition_count,
        operations: binding.operations,
        max_uses: binding.max_uses,
        operation_ordinal: binding.operation_ordinal,
        phase: binding.phase,
        flags: binding.flags,
        authorization_sha256: binding.authorization_sha256,
        authorization_id: binding.authorization_id,
        manifest_sha256: binding.manifest_sha256,
        policy_sha256: binding.policy_sha256,
        root_sha256: binding.root_sha256,
        device_binding_sha256: binding.device_binding_sha256,
        effect_sha256: binding.effect_sha256,
        operation_instance_id: binding.operation_instance_id,
        idempotency_key: binding.idempotency_key,
        previous_chain_sha256,
        chain_sha256: committed_chain_sha256,
    };
    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(MaintenancePlanError::PlanRecovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let committed_payload = committed_state
        .encode()
        .map_err(MaintenancePlanError::PlanState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_payload,
    )
    .map_err(MaintenancePlanError::Encode)?;
    let committed_lba = partition_first_lba
        .checked_add(MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(MaintenancePlanError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| MaintenancePlanError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| MaintenancePlanError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(MaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| MaintenancePlanError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(MaintenancePlanError::PlanRecovery)?;
    let verified_state =
        MaintenancePlanState::decode(verified.payload).map_err(MaintenancePlanError::PlanState)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(MaintenancePlanError::VerificationFailed);
    }
    let reads = admission
        .reads
        .checked_add(4)
        .ok_or(MaintenancePlanError::SequenceExhausted)?;
    Ok(MaintenancePlanEvidence {
        provisioned_before: existing.is_some(),
        replayed: false,
        requested_sequence: binding.sequence,
        requested_operation_ordinal: binding.operation_ordinal,
        requested_phase: binding.phase,
        previous_sequence,
        committed_sequence: binding.sequence,
        base_sequence,
        previous_transition_count,
        committed_transition_count,
        previous_operation_ordinal,
        committed_operation_ordinal: binding.operation_ordinal,
        previous_phase,
        committed_phase: binding.phase,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        reads,
        writes: 1,
        flushes: 1,
        result_unknown_before: admission.result_unknown,
        effect_observed_before: admission.effect_observed_unconfirmed,
        previous_chain_sha256,
        committed_chain_sha256,
        requested_binding: binding,
        committed_binding: binding,
    })
}

/// Selects the confirmed terminal plan head immediately before M78 completion.
pub fn validate_maintenance_plan_terminal<D: DurableSectorIo>(
    io: &mut D,
    request: MaintenancePlanTerminalRequest,
) -> Result<MaintenancePlanTerminalEvidence, MaintenancePlanError> {
    let MaintenancePlanTerminalRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        authorization,
        expected_step_chain_sha256,
    } = request;
    let admission = preflight_maintenance_plan_admission(
        io,
        MaintenanceAuditRequest {
            partition_first_lba,
            partition_sectors,
            expected_format_epoch,
            binding: authorization,
        },
    )?;
    if !admission.provisioned_before
        || admission.sequence != authorization.sequence
        || admission.operation_ordinal != MAINTENANCE_STEP_DRAIN
        || admission.phase != MAINTENANCE_PLAN_PHASE_CONFIRMED
        || admission.step.step_sequence != authorization.sequence
        || admission.step.step_ordinal != MAINTENANCE_STEP_DRAIN
        || admission.step.step_chain_sha256 != expected_step_chain_sha256
        || admission.effect_sha256 != admission.step.step_effect_sha256
    {
        return Err(MaintenancePlanError::TerminalPlanIncomplete);
    }
    Ok(MaintenancePlanTerminalEvidence {
        sequence: admission.sequence,
        base_sequence: admission.base_sequence,
        transition_count: admission.transition_count,
        operation_ordinal: admission.operation_ordinal,
        phase: admission.phase,
        generation: admission.generation,
        slot: admission.slot,
        valid_slots: admission.valid_slots,
        rejected_slots: admission.rejected_slots,
        reads: admission.reads,
        effect_sha256: admission.effect_sha256,
        operation_instance_id: admission.operation_instance_id,
        idempotency_key: admission.idempotency_key,
        chain_sha256: admission.chain_sha256,
        binding: authorization,
    })
}

/// Validates the M77-M81 ledger set before the signed program binding or
/// authorization audit may mutate.
///
/// A fresh binding is legal only at the same predecessor boundary accepted by
/// M80. Once provisioned, an exact artifact replay is read-only; a different
/// artifact at the same authorization sequence fails closed.
pub fn preflight_signed_maintenance_plan<D: DurableSectorIo>(
    io: &mut D,
    request: SignedMaintenancePlanRequest,
) -> Result<SignedMaintenancePlanAdmissionEvidence, SignedMaintenancePlanError> {
    let SignedMaintenancePlanRequest {
        partition_first_lba,
        partition_sectors,
        expected_format_epoch,
        authorization,
        binding,
    } = request;
    binding
        .validate(authorization)
        .map_err(SignedMaintenancePlanError::ProgramState)?;
    validate_record_epoch(expected_format_epoch).map_err(SignedMaintenancePlanError::Encode)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(SignedMaintenancePlanError::PartitionBounds)?;
    if partition_end > io.sector_count()
        || SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS
            .iter()
            .any(|relative_lba| *relative_lba >= partition_sectors)
    {
        return Err(SignedMaintenancePlanError::PartitionBounds);
    }

    let plan = preflight_maintenance_plan_admission(
        io,
        MaintenanceAuditRequest {
            partition_first_lba,
            partition_sectors,
            expected_format_epoch,
            binding: authorization,
        },
    )
    .map_err(SignedMaintenancePlanError::PlanPreflight)?;

    let mut program_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS
        .iter()
        .enumerate()
    {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(SignedMaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut program_slots[slot])
            .map_err(|error| SignedMaintenancePlanError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let recovered = if program_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&program_slots[0], &program_slots[1]],
                expected_format_epoch,
            )
            .map_err(SignedMaintenancePlanError::ProgramRecovery)?,
        )
    };
    let state = recovered
        .map(|record| SignedMaintenancePlanState::decode(record.payload))
        .transpose()
        .map_err(SignedMaintenancePlanError::ProgramState)?;

    let mut exact_replay = false;
    let predecessor_complete;
    if let Some(program) = state {
        if binding.authorization_sequence < program.binding.authorization_sequence {
            return Err(SignedMaintenancePlanError::Replay {
                actual: binding.authorization_sequence,
                minimum: program.binding.authorization_sequence,
            });
        }
        if binding.authorization_sequence == program.binding.authorization_sequence {
            if binding != program.binding {
                return Err(SignedMaintenancePlanError::ProgramBindingMismatch);
            }
            exact_replay = true;
            predecessor_complete = plan.legacy_anchor
                || (plan.provisioned_before && plan.sequence == binding.authorization_sequence);
            if !predecessor_complete {
                return Err(SignedMaintenancePlanError::LedgerDivergence);
            }
        } else {
            if program.binding.authorization_sequence.checked_add(1)
                != Some(binding.authorization_sequence)
            {
                return Err(SignedMaintenancePlanError::SequenceGap);
            }
            predecessor_complete = plan.provisioned_before
                && plan.sequence == program.binding.authorization_sequence
                && plan.operation_ordinal == MAINTENANCE_STEP_DRAIN
                && plan.phase == MAINTENANCE_PLAN_PHASE_CONFIRMED
                && plan.step.execution_sequence == program.binding.authorization_sequence;
            if !predecessor_complete {
                return Err(SignedMaintenancePlanError::LedgerDivergence);
            }
        }
    } else {
        predecessor_complete = plan.legacy_anchor
            || (binding.authorization_sequence == 1
                && !plan.provisioned_before
                && plan.step.audit_sequence == 0
                && plan.step.execution_sequence == 0);
        if !predecessor_complete {
            return Err(SignedMaintenancePlanError::LedgerDivergence);
        }
    }

    Ok(SignedMaintenancePlanAdmissionEvidence {
        plan,
        provisioned_before: state.is_some(),
        exact_replay,
        predecessor_complete,
        sequence: state.map_or(0, |program| program.binding.authorization_sequence),
        generation: recovered.map_or(0, |record| record.generation),
        slot: recovered.map_or(u8::MAX, |record| record.slot as u8),
        valid_slots: recovered.map_or(0, |record| record.valid_slots),
        rejected_slots: recovered.map_or(0, |record| record.rejected_slots),
        reads: plan
            .reads
            .checked_add(2)
            .ok_or(SignedMaintenancePlanError::SequenceExhausted)?,
        chain_sha256: state.map_or([0; 32], |program| program.chain_sha256),
        binding,
    })
}

/// Commits the exact verified BMP1 binding with inactive-slot
/// write/flush/full-readback semantics, or replays it without mutation.
pub fn commit_signed_maintenance_plan<D: DurableSectorIo>(
    io: &mut D,
    request: SignedMaintenancePlanRequest,
) -> Result<SignedMaintenancePlanEvidence, SignedMaintenancePlanError> {
    let admission = preflight_signed_maintenance_plan(io, request)?;
    if admission.exact_replay {
        return Ok(SignedMaintenancePlanEvidence {
            provisioned_before: true,
            replayed: true,
            predecessor_complete: admission.predecessor_complete,
            previous_sequence: admission.sequence,
            committed_sequence: admission.sequence,
            initial_generation: admission.generation,
            committed_generation: admission.generation,
            initial_slot: admission.slot,
            committed_slot: admission.slot,
            initial_valid_slots: admission.valid_slots,
            initial_rejected_slots: admission.rejected_slots,
            committed_valid_slots: admission.valid_slots,
            committed_rejected_slots: admission.rejected_slots,
            reads: admission.reads,
            writes: 0,
            flushes: 0,
            previous_chain_sha256: admission.chain_sha256,
            committed_chain_sha256: admission.chain_sha256,
            binding: request.binding,
        });
    }

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS
        .iter()
        .enumerate()
    {
        let lba = request
            .partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(SignedMaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| SignedMaintenancePlanError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }
    let recovered = if initial_slots
        .iter()
        .all(|sector| sector.iter().all(|byte| *byte == 0))
    {
        None
    } else {
        Some(
            recover(
                [&initial_slots[0], &initial_slots[1]],
                request.expected_format_epoch,
            )
            .map_err(SignedMaintenancePlanError::ProgramRecovery)?,
        )
    };
    let previous_state = recovered
        .map(|record| SignedMaintenancePlanState::decode(record.payload))
        .transpose()
        .map_err(SignedMaintenancePlanError::ProgramState)?;
    if previous_state.is_some() != admission.provisioned_before
        || previous_state.map_or(0, |state| state.binding.authorization_sequence)
            != admission.sequence
        || previous_state.map_or([0; 32], |state| state.chain_sha256) != admission.chain_sha256
    {
        return Err(SignedMaintenancePlanError::LedgerDivergence);
    }

    let previous_sequence = previous_state.map_or(0, |state| state.binding.authorization_sequence);
    let previous_chain_sha256 = previous_state.map_or([0; 32], |state| state.chain_sha256);
    let initial_generation = recovered.map_or(0, |record| record.generation);
    let initial_slot = recovered.map_or(u8::MAX, |record| record.slot as u8);
    let initial_valid_slots = recovered.map_or(0, |record| record.valid_slots);
    let initial_rejected_slots = recovered.map_or(0, |record| record.rejected_slots);
    let committed_state = SignedMaintenancePlanState {
        binding: request.binding,
        previous_chain_sha256,
        chain_sha256: signed_maintenance_plan_chain_sha256(previous_chain_sha256, request.binding),
    };
    let committed_generation = match recovered {
        Some(record) => record
            .next_generation()
            .map_err(SignedMaintenancePlanError::ProgramRecovery)?,
        None => 1,
    };
    let committed_slot = recovered.map_or(0, RecoveredRecord::inactive_slot);
    let payload = committed_state
        .encode(request.authorization)
        .map_err(SignedMaintenancePlanError::ProgramState)?;
    let committed_sector = DataRecord::encode(
        request.expected_format_epoch,
        committed_generation,
        &payload,
    )
    .map_err(SignedMaintenancePlanError::Encode)?;
    let committed_lba = request
        .partition_first_lba
        .checked_add(SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS[committed_slot])
        .ok_or(SignedMaintenancePlanError::PartitionBounds)?;
    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| SignedMaintenancePlanError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| SignedMaintenancePlanError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in SIGNED_MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS
        .iter()
        .enumerate()
    {
        let lba = request
            .partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(SignedMaintenancePlanError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| SignedMaintenancePlanError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        request.expected_format_epoch,
    )
    .map_err(SignedMaintenancePlanError::ProgramRecovery)?;
    let verified_state = SignedMaintenancePlanState::decode(verified.payload)
        .map_err(SignedMaintenancePlanError::ProgramState)?;
    let preserved_slot = committed_slot ^ 1;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified_state != committed_state
        || verified_slots[committed_slot] != committed_sector
        || verified_slots[preserved_slot] != initial_slots[preserved_slot]
    {
        return Err(SignedMaintenancePlanError::VerificationFailed);
    }

    Ok(SignedMaintenancePlanEvidence {
        provisioned_before: previous_state.is_some(),
        replayed: false,
        predecessor_complete: admission.predecessor_complete,
        previous_sequence,
        committed_sequence: request.binding.authorization_sequence,
        initial_generation,
        committed_generation,
        initial_slot,
        committed_slot: committed_slot as u8,
        initial_valid_slots,
        initial_rejected_slots,
        committed_valid_slots: verified.valid_slots,
        committed_rejected_slots: verified.rejected_slots,
        reads: admission
            .reads
            .checked_add(4)
            .ok_or(SignedMaintenancePlanError::SequenceExhausted)?,
        writes: 1,
        flushes: 1,
        previous_chain_sha256,
        committed_chain_sha256: committed_state.chain_sha256,
        binding: request.binding,
    })
}

/// Selects the exact durable signed program and M80 terminal head immediately
/// before M78 aggregate completion.
pub fn validate_signed_maintenance_plan_terminal<D: DurableSectorIo>(
    io: &mut D,
    request: SignedMaintenancePlanTerminalRequest,
) -> Result<SignedMaintenancePlanTerminalEvidence, SignedMaintenancePlanError> {
    let admission = preflight_signed_maintenance_plan(
        io,
        SignedMaintenancePlanRequest {
            partition_first_lba: request.partition_first_lba,
            partition_sectors: request.partition_sectors,
            expected_format_epoch: request.expected_format_epoch,
            authorization: request.authorization,
            binding: request.binding,
        },
    )?;
    if !admission.provisioned_before
        || !admission.exact_replay
        || admission.sequence != request.binding.authorization_sequence
        || !admission.plan.provisioned_before
        || admission.plan.sequence != request.binding.authorization_sequence
        || admission.plan.operation_ordinal != MAINTENANCE_STEP_DRAIN
        || admission.plan.phase != MAINTENANCE_PLAN_PHASE_CONFIRMED
        || admission.plan.chain_sha256 != request.expected_plan_chain_sha256
    {
        return Err(SignedMaintenancePlanError::TerminalProgramIncomplete);
    }
    Ok(SignedMaintenancePlanTerminalEvidence {
        sequence: admission.sequence,
        generation: admission.generation,
        slot: admission.slot,
        valid_slots: admission.valid_slots,
        rejected_slots: admission.rejected_slots,
        reads: admission.reads,
        program_chain_sha256: admission.chain_sha256,
        plan_chain_sha256: admission.plan.chain_sha256,
        binding: request.binding,
    })
}

/// Advances the durable boot counter using the inactive record slot.
///
/// The old selected sector is kept byte-for-byte intact. A successful return
/// therefore proves the strict sequence WRITE completion -> FLUSH completion
/// -> readback and that recovery selects the new generation. It does not claim
/// general filesystem crash consistency or exactly-once behavior after an
/// indeterminate device timeout.
pub fn advance_boot_state<D: DurableSectorIo>(
    io: &mut D,
    partition_first_lba: u64,
    partition_sectors: u64,
    expected_format_epoch: FormatEpoch,
) -> Result<AdvanceEvidence, AdvanceError> {
    validate_superblock_epoch(expected_format_epoch).map_err(AdvanceError::Superblock)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(AdvanceError::PartitionBounds)?;
    if partition_sectors < 3 || partition_end > io.sector_count() {
        return Err(AdvanceError::PartitionBounds);
    }

    let mut superblock_sector = [0_u8; SECTOR_SIZE];
    io.read_sector(partition_first_lba, &mut superblock_sector)
        .map_err(|error| AdvanceError::Io {
            phase: IoPhase::ReadSuperblock,
            error,
        })?;
    let superblock =
        DataSuperblock::decode(&superblock_sector, partition_sectors, expected_format_epoch)
            .map_err(AdvanceError::Superblock)?;

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(AdvanceError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| AdvanceError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }

    let initial = recover(
        [&initial_slots[0], &initial_slots[1]],
        expected_format_epoch,
    )
    .map_err(AdvanceError::Recovery)?;
    let (initial_boot_count, initial_health) = if initial.payload.len() == BOOT_STATE_BYTES {
        let state = BootState::decode(initial.payload).map_err(AdvanceError::BootState)?;
        (state.boot_count, None)
    } else {
        let state =
            DeviceHealthState::decode(initial.payload).map_err(AdvanceError::DeviceHealthState)?;
        (state.boot_count, Some(state))
    };
    if initial_boot_count != initial.generation {
        return Err(AdvanceError::StateGenerationMismatch);
    }
    let committed_generation = initial.next_generation().map_err(AdvanceError::Recovery)?;
    let committed_sector = match initial_health {
        Some(state) => {
            let committed_state = DeviceHealthState {
                boot_count: committed_generation,
                ..state
            }
            .encode()
            .map_err(AdvanceError::DeviceHealthState)?;
            DataRecord::encode(
                expected_format_epoch,
                committed_generation,
                &committed_state,
            )
            .map_err(AdvanceError::Encode)?
        }
        None => {
            let committed_state = BootState {
                boot_count: committed_generation,
            }
            .encode();
            DataRecord::encode(
                expected_format_epoch,
                committed_generation,
                &committed_state,
            )
            .map_err(AdvanceError::Encode)?
        }
    };
    let committed_slot = initial.inactive_slot();
    let committed_lba = partition_first_lba
        .checked_add(superblock.slot_relative_lbas[committed_slot])
        .ok_or(AdvanceError::PartitionBounds)?;

    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| AdvanceError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| AdvanceError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(AdvanceError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| AdvanceError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(AdvanceError::Recovery)?;
    let verified_boot_count = match initial_health {
        Some(expected_state) => {
            let verified_state = DeviceHealthState::decode(verified.payload)
                .map_err(AdvanceError::DeviceHealthState)?;
            if verified_state
                != (DeviceHealthState {
                    boot_count: committed_generation,
                    ..expected_state
                })
            {
                return Err(AdvanceError::VerificationFailed);
            }
            verified_state.boot_count
        }
        None => {
            BootState::decode(verified.payload)
                .map_err(AdvanceError::BootState)?
                .boot_count
        }
    };
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified.format_epoch != expected_format_epoch
        || verified_boot_count != committed_generation
        || verified_slots[initial.slot] != initial_slots[initial.slot]
        || verified_slots[committed_slot] != committed_sector
    {
        return Err(AdvanceError::VerificationFailed);
    }

    Ok(AdvanceEvidence {
        initial_generation: initial.generation,
        committed_generation,
        initial_slot: initial.slot as u8,
        committed_slot: committed_slot as u8,
        initial_valid_slots: initial.valid_slots,
        initial_rejected_slots: initial.rejected_slots,
        reads: 5,
        writes: 1,
        flushes: 1,
    })
}

/// Upgrades or advances the durable boot record after a fresh kernel probe.
///
/// Legacy M25 payloads are accepted and upgraded in the inactive slot. An
/// M63 payload from a previous boot contributes only two hints: whether that
/// boot remained open and whether its recorded device contract differs from
/// the freshly probed contract. Either hint requires reprobe evidence, but the
/// caller must already have established that evidence before entering this
/// transaction. No persisted value can publish the boot-local broker Offline
/// state, clear a recovery latch, or bypass the current boot probe.
pub fn advance_device_health_after_reprobe<D: DurableSectorIo>(
    io: &mut D,
    partition_first_lba: u64,
    partition_sectors: u64,
    expected_format_epoch: FormatEpoch,
    current_contract: DeviceHealthContract,
) -> Result<DeviceHealthAdvanceEvidence, DeviceHealthAdvanceError> {
    current_contract
        .validate()
        .map_err(DeviceHealthAdvanceError::DeviceHealthState)?;
    validate_superblock_epoch(expected_format_epoch)
        .map_err(DeviceHealthAdvanceError::Superblock)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(DeviceHealthAdvanceError::PartitionBounds)?;
    if partition_sectors < 3 || partition_end > io.sector_count() {
        return Err(DeviceHealthAdvanceError::PartitionBounds);
    }

    let mut superblock_sector = [0_u8; SECTOR_SIZE];
    io.read_sector(partition_first_lba, &mut superblock_sector)
        .map_err(|error| DeviceHealthAdvanceError::Io {
            phase: IoPhase::ReadSuperblock,
            error,
        })?;
    let superblock =
        DataSuperblock::decode(&superblock_sector, partition_sectors, expected_format_epoch)
            .map_err(DeviceHealthAdvanceError::Superblock)?;

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(DeviceHealthAdvanceError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| DeviceHealthAdvanceError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }

    let initial = recover(
        [&initial_slots[0], &initial_slots[1]],
        expected_format_epoch,
    )
    .map_err(DeviceHealthAdvanceError::Recovery)?;
    let (
        initial_boot_count,
        legacy_upgrade,
        persisted_contract_present,
        prior_boot_open,
        contract_changed,
    ) = if initial.payload.len() == BOOT_STATE_BYTES {
        let legacy = BootState::decode(initial.payload)
            .map_err(DeviceHealthAdvanceError::LegacyBootState)?;
        (legacy.boot_count, true, false, false, false)
    } else {
        let state = DeviceHealthState::decode(initial.payload)
            .map_err(DeviceHealthAdvanceError::DeviceHealthState)?;
        (
            state.boot_count,
            false,
            true,
            state.boot_open,
            state.contract != current_contract,
        )
    };
    if initial_boot_count != initial.generation {
        return Err(DeviceHealthAdvanceError::StateGenerationMismatch);
    }

    let committed_generation = initial
        .next_generation()
        .map_err(DeviceHealthAdvanceError::Recovery)?;
    let committed_state = DeviceHealthState {
        boot_count: committed_generation,
        boot_open: true,
        contract: current_contract,
    }
    .encode()
    .map_err(DeviceHealthAdvanceError::DeviceHealthState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_state,
    )
    .map_err(DeviceHealthAdvanceError::Encode)?;
    let committed_slot = initial.inactive_slot();
    let committed_lba = partition_first_lba
        .checked_add(superblock.slot_relative_lbas[committed_slot])
        .ok_or(DeviceHealthAdvanceError::PartitionBounds)?;

    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| DeviceHealthAdvanceError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| DeviceHealthAdvanceError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(DeviceHealthAdvanceError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| DeviceHealthAdvanceError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(DeviceHealthAdvanceError::Recovery)?;
    let verified_state = DeviceHealthState::decode(verified.payload)
        .map_err(DeviceHealthAdvanceError::DeviceHealthState)?;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified.format_epoch != expected_format_epoch
        || verified_state
            != (DeviceHealthState {
                boot_count: committed_generation,
                boot_open: true,
                contract: current_contract,
            })
        || verified_slots[initial.slot] != initial_slots[initial.slot]
        || verified_slots[committed_slot] != committed_sector
    {
        return Err(DeviceHealthAdvanceError::VerificationFailed);
    }

    Ok(DeviceHealthAdvanceEvidence {
        persistence: AdvanceEvidence {
            initial_generation: initial.generation,
            committed_generation,
            initial_slot: initial.slot as u8,
            committed_slot: committed_slot as u8,
            initial_valid_slots: initial.valid_slots,
            initial_rejected_slots: initial.rejected_slots,
            reads: 5,
            writes: 1,
            flushes: 1,
        },
        legacy_upgrade,
        persisted_contract_present,
        prior_boot_open,
        contract_changed,
        reprobe_required: prior_boot_open || contract_changed,
        reprobe_verified: true,
        current_boot_open: true,
        offline_persisted: false,
        offline_from_record: false,
        current_contract,
    })
}

/// Durably closes the exact M63 boot session opened by the current kernel.
///
/// The caller supplies the in-memory generation returned by
/// [`advance_device_health_after_reprobe`] and the still-live probed device
/// contract. A stale generation, an already-closed session, or any contract
/// change is rejected before mutation. The transaction records only a clean
/// session boundary; it never persists or reconstructs the broker's
/// boot-local Offline state.
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
pub fn close_device_health_for_clean_shutdown<D: DurableSectorIo>(
    io: &mut D,
    partition_first_lba: u64,
    partition_sectors: u64,
    expected_format_epoch: FormatEpoch,
    expected_open_generation: u64,
    current_contract: DeviceHealthContract,
) -> Result<DeviceHealthCloseEvidence, DeviceHealthCloseError> {
    current_contract
        .validate()
        .map_err(DeviceHealthCloseError::DeviceHealthState)?;
    validate_superblock_epoch(expected_format_epoch).map_err(DeviceHealthCloseError::Superblock)?;
    let partition_end = partition_first_lba
        .checked_add(partition_sectors)
        .ok_or(DeviceHealthCloseError::PartitionBounds)?;
    if partition_sectors < 3 || partition_end > io.sector_count() {
        return Err(DeviceHealthCloseError::PartitionBounds);
    }

    let mut superblock_sector = [0_u8; SECTOR_SIZE];
    io.read_sector(partition_first_lba, &mut superblock_sector)
        .map_err(|error| DeviceHealthCloseError::Io {
            phase: IoPhase::ReadSuperblock,
            error,
        })?;
    let superblock =
        DataSuperblock::decode(&superblock_sector, partition_sectors, expected_format_epoch)
            .map_err(DeviceHealthCloseError::Superblock)?;

    let mut initial_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(DeviceHealthCloseError::PartitionBounds)?;
        io.read_sector(lba, &mut initial_slots[slot])
            .map_err(|error| DeviceHealthCloseError::Io {
                phase: IoPhase::ReadInitialSlot,
                error,
            })?;
    }

    let initial = recover(
        [&initial_slots[0], &initial_slots[1]],
        expected_format_epoch,
    )
    .map_err(DeviceHealthCloseError::Recovery)?;
    let initial_state = DeviceHealthState::decode(initial.payload)
        .map_err(DeviceHealthCloseError::DeviceHealthState)?;
    if initial_state.boot_count != initial.generation {
        return Err(DeviceHealthCloseError::StateGenerationMismatch);
    }
    if initial.generation != expected_open_generation {
        return Err(DeviceHealthCloseError::SessionGenerationMismatch);
    }
    if !initial_state.boot_open {
        return Err(DeviceHealthCloseError::SessionAlreadyClosed);
    }
    if initial_state.contract != current_contract {
        return Err(DeviceHealthCloseError::ContractMismatch);
    }

    let committed_generation = initial
        .next_generation()
        .map_err(DeviceHealthCloseError::Recovery)?;
    let committed_state = DeviceHealthState {
        boot_count: committed_generation,
        boot_open: false,
        contract: current_contract,
    }
    .encode()
    .map_err(DeviceHealthCloseError::DeviceHealthState)?;
    let committed_sector = DataRecord::encode(
        expected_format_epoch,
        committed_generation,
        &committed_state,
    )
    .map_err(DeviceHealthCloseError::Encode)?;
    let committed_slot = initial.inactive_slot();
    let committed_lba = partition_first_lba
        .checked_add(superblock.slot_relative_lbas[committed_slot])
        .ok_or(DeviceHealthCloseError::PartitionBounds)?;

    io.write_sector(committed_lba, &committed_sector)
        .map_err(|error| DeviceHealthCloseError::Io {
            phase: IoPhase::WriteInactiveSlot,
            error,
        })?;
    io.flush().map_err(|error| DeviceHealthCloseError::Io {
        phase: IoPhase::Flush,
        error,
    })?;

    let mut verified_slots = [[0_u8; SECTOR_SIZE]; DATA_SLOT_COUNT];
    for (slot, relative_lba) in superblock.slot_relative_lbas.iter().enumerate() {
        let lba = partition_first_lba
            .checked_add(*relative_lba)
            .ok_or(DeviceHealthCloseError::PartitionBounds)?;
        io.read_sector(lba, &mut verified_slots[slot])
            .map_err(|error| DeviceHealthCloseError::Io {
                phase: IoPhase::ReadVerificationSlot,
                error,
            })?;
    }
    let verified = recover(
        [&verified_slots[0], &verified_slots[1]],
        expected_format_epoch,
    )
    .map_err(DeviceHealthCloseError::Recovery)?;
    let verified_state = DeviceHealthState::decode(verified.payload)
        .map_err(DeviceHealthCloseError::DeviceHealthState)?;
    if verified.slot != committed_slot
        || verified.generation != committed_generation
        || verified.format_epoch != expected_format_epoch
        || verified_state
            != (DeviceHealthState {
                boot_count: committed_generation,
                boot_open: false,
                contract: current_contract,
            })
        || verified_slots[initial.slot] != initial_slots[initial.slot]
        || verified_slots[committed_slot] != committed_sector
    {
        return Err(DeviceHealthCloseError::VerificationFailed);
    }

    Ok(DeviceHealthCloseEvidence {
        persistence: AdvanceEvidence {
            initial_generation: initial.generation,
            committed_generation,
            initial_slot: initial.slot as u8,
            committed_slot: committed_slot as u8,
            initial_valid_slots: initial.valid_slots,
            initial_rejected_slots: initial.rejected_slots,
            reads: 5,
            writes: 1,
            flushes: 1,
        },
        open_generation: expected_open_generation,
        prior_boot_open: true,
        contract_matched: true,
        current_boot_open: false,
        offline_persisted: false,
        offline_from_record: false,
        current_contract,
    })
}

pub fn crc32(bytes: &[u8]) -> u32 {
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

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn seal_sector(sector: &mut Sector) {
    let crc = crc32(&sector[..DATA_RECORD_CRC_OFFSET]);
    put_u32(sector, DATA_RECORD_CRC_OFFSET, crc);
}

fn sector_crc_is_valid(sector: &Sector) -> bool {
    crc32(&sector[..DATA_RECORD_CRC_OFFSET]) == le_u32(sector, DATA_RECORD_CRC_OFFSET)
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
mod tests {
    use super::*;
    use std::vec::Vec;

    const FORMAT_EPOCH: FormatEpoch = *b"BNDROID-M25-E001";
    const FOREIGN_EPOCH: FormatEpoch = *b"BNDROID-M25-E999";

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Operation {
        Read(u64),
        Write(u64),
        Flush,
    }

    struct MemoryIo {
        sectors: [Sector; 16],
        operations: Vec<Operation>,
        fail_flush: bool,
        tear_write: bool,
    }

    impl MemoryIo {
        fn fixture() -> Self {
            let mut sectors = [[0_u8; SECTOR_SIZE]; 16];
            sectors[2] = DataSuperblock::encode(4, FORMAT_EPOCH).unwrap();
            sectors[3] = DataRecord::encode(FORMAT_EPOCH, 0, &state(0)).unwrap();
            Self {
                sectors,
                operations: Vec::new(),
                fail_flush: false,
                tear_write: false,
            }
        }
    }

    impl DurableSectorIo for MemoryIo {
        fn sector_count(&self) -> u64 {
            self.sectors.len() as u64
        }

        fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), PersistIoError> {
            let source = self
                .sectors
                .get(lba as usize)
                .ok_or(PersistIoError::OutOfBounds)?;
            *output = *source;
            self.operations.push(Operation::Read(lba));
            Ok(())
        }

        fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), PersistIoError> {
            let destination = self
                .sectors
                .get_mut(lba as usize)
                .ok_or(PersistIoError::OutOfBounds)?;
            *destination = *input;
            if self.tear_write {
                destination[100] ^= 1;
            }
            self.operations.push(Operation::Write(lba));
            Ok(())
        }

        fn flush(&mut self) -> Result<(), PersistIoError> {
            self.operations.push(Operation::Flush);
            if self.fail_flush {
                Err(PersistIoError::Device)
            } else {
                Ok(())
            }
        }
    }

    fn state(count: u64) -> [u8; BOOT_STATE_BYTES] {
        BootState { boot_count: count }.encode()
    }

    fn device_contract() -> DeviceHealthContract {
        DeviceHealthContract {
            capacity_sectors: 16_384,
            selected_features: (1_u64 << 32) | (1_u64 << 9),
            transport_base: 0x0a00_3e00,
            irq_id: 79,
            sector_bytes: 512,
            queue_size: 8,
            device_read_only: false,
            flush_supported: true,
        }
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    fn manifest_binding() -> ManifestRollbackBinding {
        ManifestRollbackBinding {
            key_id: 2,
            trust_anchor_sha256: [0x42; 32],
        }
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    fn manifest_key_policy_binding(key_id: u8, key_epoch: u32) -> ManifestKeyPolicyBinding {
        ManifestKeyPolicyBinding {
            key_id,
            key_epoch,
            trust_anchor_sha256: [key_id; 32],
            policy_sha256: [0x76; 32],
        }
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    fn manifest_key_rotation_request(
        key_id: u8,
        key_epoch: u32,
        artifact_index: u32,
    ) -> ManifestKeyRotationRequest {
        ManifestKeyRotationRequest {
            partition_first_lba: 2,
            partition_sectors: 6,
            expected_format_epoch: FORMAT_EPOCH,
            bootstrap_floor: 2,
            artifact_index,
            artifact_binding: manifest_key_policy_binding(key_id, key_epoch),
            legacy_predecessor: manifest_binding(),
            legacy_predecessor_epoch: 2,
        }
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    fn maintenance_binding(sequence: u64) -> MaintenanceAuthorizationAuditBinding {
        MaintenanceAuthorizationAuditBinding {
            sequence,
            operations: 1,
            max_uses: 2,
            authorization_sha256: [sequence as u8; 32],
            authorization_id: [sequence as u8 + 0x10; 32],
            manifest_sha256: [0x5a; 32],
            policy_sha256: [0xd5; 32],
            root_sha256: [0x68; 32],
            device_binding_sha256: [0x04; 32],
        }
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    fn maintenance_request(sequence: u64) -> MaintenanceAuditRequest {
        maintenance_request_with_sectors(sequence, 9)
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    fn maintenance_request_with_sectors(
        sequence: u64,
        partition_sectors: u64,
    ) -> MaintenanceAuditRequest {
        MaintenanceAuditRequest {
            partition_first_lba: 1,
            partition_sectors,
            expected_format_epoch: FORMAT_EPOCH,
            binding: maintenance_binding(sequence),
        }
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    fn maintenance_completion_binding(sequence: u64) -> MaintenanceExecutionCompletionBinding {
        let authorization = maintenance_binding(sequence);
        MaintenanceExecutionCompletionBinding {
            sequence,
            operations: authorization.operations,
            max_uses: authorization.max_uses,
            uses_consumed: authorization.max_uses,
            rotations_completed: authorization.max_uses,
            flags: MAINTENANCE_EXECUTION_REQUIRED_FLAGS,
            appdata_generation: sequence + 20,
            authorization_sha256: authorization.authorization_sha256,
            authorization_id: authorization.authorization_id,
            manifest_sha256: authorization.manifest_sha256,
            policy_sha256: authorization.policy_sha256,
            root_sha256: authorization.root_sha256,
            device_binding_sha256: authorization.device_binding_sha256,
            runtime_evidence_sha256: [sequence as u8 + 0x40; 32],
        }
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    fn maintenance_completion_request(sequence: u64) -> MaintenanceExecutionCompletionRequest {
        maintenance_completion_request_with_sectors(sequence, 9)
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    fn maintenance_completion_request_with_sectors(
        sequence: u64,
        partition_sectors: u64,
    ) -> MaintenanceExecutionCompletionRequest {
        MaintenanceExecutionCompletionRequest {
            partition_first_lba: 1,
            partition_sectors,
            expected_format_epoch: FORMAT_EPOCH,
            binding: maintenance_completion_binding(sequence),
        }
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    fn maintenance_step_binding(sequence: u64, step_ordinal: u32) -> MaintenanceStepCommitBinding {
        let (observed_rotations, drain_validated) = match step_ordinal {
            MAINTENANCE_STEP_ROTATION_ONE => (1, false),
            MAINTENANCE_STEP_ROTATION_TWO => (2, false),
            MAINTENANCE_STEP_DRAIN => (2, true),
            _ => (0, false),
        };
        MaintenanceStepCommitBinding::for_observation(
            maintenance_binding(sequence),
            step_ordinal,
            observed_rotations,
            drain_validated,
        )
        .unwrap()
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    fn maintenance_step_request(sequence: u64, step_ordinal: u32) -> MaintenanceStepCommitRequest {
        maintenance_step_request_with_sectors(sequence, step_ordinal, 11)
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    fn maintenance_step_request_with_sectors(
        sequence: u64,
        step_ordinal: u32,
        partition_sectors: u64,
    ) -> MaintenanceStepCommitRequest {
        MaintenanceStepCommitRequest {
            partition_first_lba: 1,
            partition_sectors,
            expected_format_epoch: FORMAT_EPOCH,
            binding: maintenance_step_binding(sequence, step_ordinal),
        }
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    fn maintenance_step_terminal_request(sequence: u64) -> MaintenanceStepTerminalRequest {
        MaintenanceStepTerminalRequest {
            partition_first_lba: 1,
            partition_sectors: 11,
            expected_format_epoch: FORMAT_EPOCH,
            authorization: maintenance_binding(sequence),
            rotations_completed: 2,
        }
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    fn maintenance_plan_binding(
        sequence: u64,
        operation_ordinal: u32,
        phase: u32,
    ) -> MaintenancePlanTransitionBinding {
        MaintenancePlanTransitionBinding::for_transition(
            maintenance_binding(sequence),
            operation_ordinal,
            phase,
        )
        .unwrap()
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    fn maintenance_plan_request(
        sequence: u64,
        operation_ordinal: u32,
        phase: u32,
    ) -> MaintenancePlanTransitionRequest {
        maintenance_plan_request_with_sectors(sequence, operation_ordinal, phase, 13)
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    fn maintenance_plan_request_with_sectors(
        sequence: u64,
        operation_ordinal: u32,
        phase: u32,
        partition_sectors: u64,
    ) -> MaintenancePlanTransitionRequest {
        MaintenancePlanTransitionRequest {
            partition_first_lba: 1,
            partition_sectors,
            expected_format_epoch: FORMAT_EPOCH,
            binding: maintenance_plan_binding(sequence, operation_ordinal, phase),
        }
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    fn prepare_m80_sequence_two(io: &mut MemoryIo) {
        admit_maintenance_authorization_with_execution(io, maintenance_request_with_sectors(1, 13))
            .unwrap();
        commit_maintenance_execution_completion(
            io,
            maintenance_completion_request_with_sectors(1, 13),
        )
        .unwrap();
        let admission =
            preflight_maintenance_plan_admission(io, maintenance_request_with_sectors(2, 13))
                .unwrap();
        assert!(!admission.provisioned_before);
        assert!(admission.legacy_anchor);
        assert_eq!(admission.reads, 8);
        admit_maintenance_authorization_with_execution(io, maintenance_request_with_sectors(2, 13))
            .unwrap();
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    fn signed_maintenance_plan_binding(sequence: u64) -> SignedMaintenancePlanBinding {
        let authorization = maintenance_binding(sequence);
        SignedMaintenancePlanBinding {
            authorization_sequence: sequence,
            operation_count: SIGNED_MAINTENANCE_PLAN_OPERATION_COUNT,
            transition_count: SIGNED_MAINTENANCE_PLAN_TRANSITION_COUNT,
            program_version: SIGNED_MAINTENANCE_PLAN_PROGRAM_VERSION,
            preapply_cancel_mask: SIGNED_MAINTENANCE_PLAN_PREAPPLY_CANCEL_MASK,
            authorization_sha256: authorization.authorization_sha256,
            authorization_id: authorization.authorization_id,
            program_sha256: [sequence as u8 + 0x50; 32],
            plan_id_sha256: maintenance_plan_id_sha256(authorization),
            descriptor_sha256: [sequence as u8 + 0x60; 32],
            root_sha256: [0x81; 32],
            device_binding_sha256: authorization.device_binding_sha256,
        }
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    fn signed_maintenance_plan_request(sequence: u64) -> SignedMaintenancePlanRequest {
        SignedMaintenancePlanRequest {
            partition_first_lba: 1,
            partition_sectors: 15,
            expected_format_epoch: FORMAT_EPOCH,
            authorization: maintenance_binding(sequence),
            binding: signed_maintenance_plan_binding(sequence),
        }
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    fn prepare_m81_predecessor(io: &mut MemoryIo) {
        admit_maintenance_authorization_with_execution(io, maintenance_request_with_sectors(1, 15))
            .unwrap();
        commit_maintenance_execution_completion(
            io,
            maintenance_completion_request_with_sectors(1, 15),
        )
        .unwrap();
    }

    #[test]
    fn superblock_round_trips_and_rejects_geometry_or_corruption() {
        let sector = DataSuperblock::encode(64, FORMAT_EPOCH).unwrap();
        assert_eq!(
            DataSuperblock::decode(&sector, 64, FORMAT_EPOCH),
            Ok(DataSuperblock {
                partition_sectors: 64,
                slot_relative_lbas: [1, 2],
                format_epoch: FORMAT_EPOCH,
            })
        );
        assert_eq!(
            DataSuperblock::decode(&sector, 63, FORMAT_EPOCH),
            Err(SuperblockError::BadPartitionSize)
        );
        let mut corrupt = sector;
        corrupt[100] = 1;
        assert_eq!(
            DataSuperblock::decode(&corrupt, 64, FORMAT_EPOCH),
            Err(SuperblockError::NonZeroReserved)
        );
        assert_eq!(
            DataSuperblock::encode(2, FORMAT_EPOCH),
            Err(SuperblockError::BadPartitionSize)
        );
    }

    #[test]
    fn superblock_requires_the_expected_nonzero_format_epoch() {
        let sector = DataSuperblock::encode(64, FORMAT_EPOCH).unwrap();
        assert_eq!(
            DataSuperblock::decode(&sector, 64, FOREIGN_EPOCH),
            Err(SuperblockError::FormatEpochMismatch)
        );
        assert_eq!(
            DataSuperblock::encode(64, [0; FORMAT_EPOCH_BYTES]),
            Err(SuperblockError::ZeroFormatEpoch)
        );
        assert_eq!(
            DataSuperblock::decode(&sector, 64, [0; FORMAT_EPOCH_BYTES]),
            Err(SuperblockError::ZeroFormatEpoch)
        );

        let mut zero_on_disk = sector;
        zero_on_disk[48..48 + FORMAT_EPOCH_BYTES].fill(0);
        seal_sector(&mut zero_on_disk);
        assert_eq!(
            DataSuperblock::decode(&zero_on_disk, 64, FORMAT_EPOCH),
            Err(SuperblockError::ZeroFormatEpoch)
        );
    }

    #[test]
    fn records_round_trip_at_empty_and_maximum_payload_sizes() {
        for payload in [&[][..], &[0x5a; DATA_RECORD_MAX_PAYLOAD][..]] {
            let sector = DataRecord::encode(FORMAT_EPOCH, 42, payload).unwrap();
            let record = DataRecord::decode(&sector, FORMAT_EPOCH).unwrap();
            assert_eq!(record.generation, 42);
            assert_eq!(record.format_epoch, FORMAT_EPOCH);
            assert_eq!(record.payload, payload);
        }
        assert_eq!(
            DataRecord::encode(FORMAT_EPOCH, 0, &[0; DATA_RECORD_MAX_PAYLOAD + 1]),
            Err(RecordError::PayloadTooLarge)
        );
    }

    #[test]
    fn records_require_the_expected_nonzero_format_epoch() {
        let sector = DataRecord::encode(FORMAT_EPOCH, 42, b"payload").unwrap();
        assert_eq!(
            DataRecord::decode(&sector, FOREIGN_EPOCH),
            Err(RecordError::FormatEpochMismatch)
        );
        assert_eq!(
            DataRecord::encode([0; FORMAT_EPOCH_BYTES], 42, b"payload"),
            Err(RecordError::ZeroFormatEpoch)
        );
        assert_eq!(
            DataRecord::decode(&sector, [0; FORMAT_EPOCH_BYTES]),
            Err(RecordError::ZeroFormatEpoch)
        );

        let mut zero_on_disk = sector;
        zero_on_disk[64..64 + FORMAT_EPOCH_BYTES].fill(0);
        seal_sector(&mut zero_on_disk);
        assert_eq!(
            DataRecord::decode(&zero_on_disk, FORMAT_EPOCH),
            Err(RecordError::ZeroFormatEpoch)
        );
    }

    #[test]
    fn record_crc_payload_crc_digest_and_generation_witness_reject_corruption() {
        let valid = DataRecord::encode(FORMAT_EPOCH, 7, b"persistent payload").unwrap();
        for offset in [16, 32, 40, 48, 64, DATA_RECORD_CRC_OFFSET] {
            let mut corrupt = valid;
            corrupt[offset] ^= 0x80;
            assert!(
                DataRecord::decode(&corrupt, FORMAT_EPOCH).is_err(),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn recovery_selects_the_newest_valid_slot_and_reports_rejections() {
        let old = DataRecord::encode(FORMAT_EPOCH, 4, &state(4)).unwrap();
        let new = DataRecord::encode(FORMAT_EPOCH, 5, &state(5)).unwrap();
        let selected = recover([&old, &new], FORMAT_EPOCH).unwrap();
        assert_eq!(selected.slot, 1);
        assert_eq!(selected.generation, 5);
        assert_eq!(selected.format_epoch, FORMAT_EPOCH);
        assert_eq!(selected.valid_slots, 2);
        assert_eq!(selected.rejected_slots, 0);
        assert_eq!(selected.inactive_slot(), 0);
        assert_eq!(selected.next_generation(), Ok(6));
        assert_eq!(
            BootState::decode(selected.payload),
            Ok(BootState { boot_count: 5 })
        );

        let mut torn = new;
        torn[80] ^= 1;
        let recovered = recover([&old, &torn], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.slot, 0);
        assert_eq!(recovered.generation, 4);
        assert_eq!(recovered.valid_slots, 1);
        assert_eq!(recovered.rejected_slots, 1);
    }

    #[test]
    fn recovery_rejects_no_valid_duplicate_and_exhausted_generations() {
        let empty = [0_u8; SECTOR_SIZE];
        assert_eq!(
            recover([&empty, &empty], FORMAT_EPOCH),
            Err(RecoveryError::NoValidRecord)
        );
        let first = DataRecord::encode(FORMAT_EPOCH, 9, b"first").unwrap();
        let second = DataRecord::encode(FORMAT_EPOCH, 9, b"second").unwrap();
        assert_eq!(
            recover([&first, &second], FORMAT_EPOCH),
            Err(RecoveryError::AmbiguousGeneration)
        );
        let exhausted = DataRecord::encode(FORMAT_EPOCH, u64::MAX, b"last").unwrap();
        let selected = recover([&exhausted, &empty], FORMAT_EPOCH).unwrap();
        assert_eq!(
            selected.next_generation(),
            Err(RecoveryError::GenerationExhausted)
        );
    }

    #[test]
    fn recovery_rejects_generation_gaps_in_either_slot_order() {
        let fourth = DataRecord::encode(FORMAT_EPOCH, 4, b"fourth").unwrap();
        let sixth = DataRecord::encode(FORMAT_EPOCH, 6, b"sixth").unwrap();
        assert_eq!(
            recover([&fourth, &sixth], FORMAT_EPOCH),
            Err(RecoveryError::GenerationGap)
        );
        assert_eq!(
            recover([&sixth, &fourth], FORMAT_EPOCH),
            Err(RecoveryError::GenerationGap)
        );
    }

    #[test]
    fn recovery_rejects_foreign_and_zero_format_epochs() {
        let matching = DataRecord::encode(FORMAT_EPOCH, 4, b"matching").unwrap();
        let foreign = DataRecord::encode(FOREIGN_EPOCH, 5, b"foreign").unwrap();
        let selected = recover([&matching, &foreign], FORMAT_EPOCH).unwrap();
        assert_eq!(selected.slot, 0);
        assert_eq!(selected.generation, 4);
        assert_eq!(selected.valid_slots, 1);
        assert_eq!(selected.rejected_slots, 1);
        assert_eq!(
            recover([&foreign, &foreign], FORMAT_EPOCH),
            Err(RecoveryError::NoValidRecord)
        );
        assert_eq!(
            recover([&matching, &foreign], [0; FORMAT_EPOCH_BYTES]),
            Err(RecoveryError::ZeroFormatEpoch)
        );
    }

    #[test]
    fn boot_state_has_exact_wire_size_and_witness() {
        let encoded = state(0x1020_3040_5060_7080);
        assert_eq!(encoded.len(), BOOT_STATE_BYTES);
        assert_eq!(
            BootState::decode(&encoded),
            Ok(BootState {
                boot_count: 0x1020_3040_5060_7080,
            })
        );
        let mut corrupt = encoded;
        corrupt[24] ^= 1;
        assert_eq!(
            BootState::decode(&corrupt),
            Err(BootStateError::WitnessMismatch)
        );
    }

    #[test]
    fn device_health_state_round_trips_exact_contract_and_open_hint() {
        let state = DeviceHealthState {
            boot_count: 9,
            boot_open: true,
            contract: device_contract(),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), DEVICE_HEALTH_STATE_BYTES);
        assert_eq!(DeviceHealthState::decode(&encoded), Ok(state));

        let closed = DeviceHealthState {
            boot_open: false,
            ..state
        };
        assert_eq!(
            DeviceHealthState::decode(&closed.encode().unwrap()),
            Ok(closed)
        );
    }

    #[test]
    fn device_health_state_rejects_flags_contract_corruption_and_witnesses() {
        let state = DeviceHealthState {
            boot_count: 9,
            boot_open: true,
            contract: device_contract(),
        };
        let encoded = state.encode().unwrap();

        let mut flags = encoded;
        flags[12] |= 0x80;
        assert_eq!(
            DeviceHealthState::decode(&flags),
            Err(DeviceHealthStateError::BadFlags)
        );

        let mut contract = encoded;
        contract[24] ^= 1;
        assert_eq!(
            DeviceHealthState::decode(&contract),
            Err(DeviceHealthStateError::ContractDigestMismatch)
        );

        let mut witness = encoded;
        witness[72] ^= 1;
        assert_eq!(
            DeviceHealthState::decode(&witness),
            Err(DeviceHealthStateError::StateWitnessMismatch)
        );

        let invalid = DeviceHealthState {
            contract: DeviceHealthContract {
                capacity_sectors: 0,
                ..device_contract()
            },
            ..state
        };
        assert_eq!(
            invalid.encode(),
            Err(DeviceHealthStateError::InvalidContract)
        );
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn manifest_rollback_state_round_trips_and_rejects_field_corruption() {
        let state = ManifestRollbackState {
            floor: 3,
            binding: manifest_binding(),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MANIFEST_ROLLBACK_STATE_BYTES);
        assert_eq!(ManifestRollbackState::decode(&encoded), Ok(state));

        for offset in [0, 8, 12, 16, 20, 24, 56] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                ManifestRollbackState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
        assert_eq!(
            ManifestRollbackState {
                floor: 0,
                binding: manifest_binding(),
            }
            .encode(),
            Err(ManifestRollbackStateError::ZeroFloor)
        );
        assert_eq!(
            ManifestRollbackState {
                floor: 3,
                binding: ManifestRollbackBinding {
                    key_id: 0,
                    trust_anchor_sha256: [0x42; 32],
                },
            }
            .encode(),
            Err(ManifestRollbackStateError::InvalidKeyId)
        );
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn manifest_floor_advances_repairs_redundancy_then_becomes_read_only() {
        let mut io = MemoryIo::fixture();
        let first = enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        assert_eq!(
            first,
            ManifestRollbackEvidence {
                provisioned_before: false,
                bootstrap_floor: 2,
                persisted_floor_before: 0,
                effective_floor: 2,
                artifact_index: 3,
                committed_floor: 3,
                initial_generation: 0,
                committed_generation: 1,
                initial_slot: u8::MAX,
                committed_slot: 0,
                initial_valid_slots: 0,
                initial_rejected_slots: 0,
                committed_valid_slots: 1,
                committed_rejected_slots: 1,
                floor_advanced: true,
                record_written: true,
                redundancy_repaired: false,
                reads: 4,
                writes: 1,
                flushes: 1,
                binding: manifest_binding(),
            }
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(5),
                Operation::Read(6),
                Operation::Write(5),
                Operation::Flush,
                Operation::Read(5),
                Operation::Read(6),
            ]
        );

        io.operations.clear();
        let second = enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        assert!(second.provisioned_before);
        assert_eq!(second.persisted_floor_before, 3);
        assert_eq!(second.initial_generation, 1);
        assert_eq!(second.committed_generation, 2);
        assert_eq!(second.initial_valid_slots, 1);
        assert_eq!(second.committed_valid_slots, 2);
        assert!(!second.floor_advanced);
        assert!(second.record_written);
        assert!(second.redundancy_repaired);
        assert_eq!(second.committed_slot, 1);

        io.operations.clear();
        let third = enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        assert_eq!(third.initial_generation, 2);
        assert_eq!(third.committed_generation, 2);
        assert_eq!(third.initial_valid_slots, 2);
        assert_eq!(third.committed_valid_slots, 2);
        assert!(!third.floor_advanced);
        assert!(!third.record_written);
        assert!(!third.redundancy_repaired);
        assert_eq!(io.operations, [Operation::Read(5), Operation::Read(6)]);
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn persistent_manifest_floor_rejects_a_valid_old_index_without_mutation() {
        let mut io = MemoryIo::fixture();
        enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        io.operations.clear();
        assert_eq!(
            enforce_and_advance_manifest_rollback(
                &mut io,
                2,
                6,
                FORMAT_EPOCH,
                2,
                2,
                manifest_binding(),
            ),
            Err(ManifestRollbackError::Rollback {
                actual: 2,
                minimum: 3,
            })
        );
        assert_eq!(io.operations, [Operation::Read(5), Operation::Read(6)]);
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn torn_inactive_manifest_record_preserves_and_repairs_the_old_floor() {
        let mut io = MemoryIo::fixture();
        enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        io.sectors[6] = io.sectors[5];
        io.sectors[6][100] ^= 1;
        io.operations.clear();
        let repaired = enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        assert_eq!(repaired.persisted_floor_before, 3);
        assert_eq!(repaired.initial_valid_slots, 1);
        assert_eq!(repaired.initial_rejected_slots, 1);
        assert!(repaired.redundancy_repaired);
        assert_eq!(repaired.committed_valid_slots, 2);
        assert_eq!(repaired.committed_rejected_slots, 0);
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn manifest_floor_fails_closed_on_foreign_binding_and_no_valid_record() {
        let mut io = MemoryIo::fixture();
        let foreign = ManifestRollbackState {
            floor: 3,
            binding: ManifestRollbackBinding {
                key_id: 7,
                trust_anchor_sha256: [0x77; 32],
            },
        }
        .encode()
        .unwrap();
        io.sectors[5] = DataRecord::encode(FORMAT_EPOCH, 1, &foreign).unwrap();
        assert_eq!(
            enforce_and_advance_manifest_rollback(
                &mut io,
                2,
                6,
                FORMAT_EPOCH,
                2,
                3,
                manifest_binding(),
            ),
            Err(ManifestRollbackError::BindingMismatch)
        );

        io.sectors[5].fill(0x55);
        io.sectors[6].fill(0xaa);
        assert_eq!(
            enforce_and_advance_manifest_rollback(
                &mut io,
                2,
                6,
                FORMAT_EPOCH,
                2,
                3,
                manifest_binding(),
            ),
            Err(ManifestRollbackError::Recovery(
                RecoveryError::NoValidRecord
            ))
        );
    }

    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    #[test]
    fn manifest_floor_mutation_failures_never_destroy_the_selected_slot() {
        let mut io = MemoryIo::fixture();
        enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        let selected = io.sectors[5];
        io.tear_write = true;
        assert_eq!(
            enforce_and_advance_manifest_rollback(
                &mut io,
                2,
                6,
                FORMAT_EPOCH,
                2,
                3,
                manifest_binding(),
            ),
            Err(ManifestRollbackError::VerificationFailed)
        );
        assert_eq!(io.sectors[5], selected);
        assert_eq!(
            recover([&io.sectors[5], &io.sectors[6]], FORMAT_EPOCH)
                .unwrap()
                .slot,
            0
        );
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    #[test]
    fn manifest_key_policy_state_round_trips_and_rejects_corruption() {
        let state = ManifestKeyPolicyState {
            floor: 5,
            binding: manifest_key_policy_binding(4, 4),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MANIFEST_KEY_POLICY_STATE_BYTES);
        assert_eq!(ManifestKeyPolicyState::decode(&encoded), Ok(state));
        for offset in [0, 8, 12, 16, 20, 24, 56, 88] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                ManifestKeyPolicyState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    #[test]
    fn manifest_key_policy_migrates_two_epochs_repairs_then_becomes_read_only() {
        let mut io = MemoryIo::fixture();
        enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();

        io.operations.clear();
        let key3 =
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(3, 3, 4))
                .unwrap();
        assert!(key3.provisioned_before);
        assert!(key3.legacy_migrated);
        assert!(key3.key_transition);
        assert_eq!(key3.previous_key_id, 2);
        assert_eq!(key3.previous_key_epoch, 2);
        assert_eq!(key3.active_key_id, 3);
        assert_eq!(key3.active_key_epoch, 3);
        assert_eq!(key3.persisted_floor_before, 3);
        assert_eq!(key3.committed_floor, 4);
        assert_eq!(key3.initial_generation, 1);
        assert_eq!(key3.committed_generation, 2);
        assert_eq!(key3.initial_active_policy_slots, 0);
        assert_eq!(key3.committed_active_policy_slots, 1);
        assert!(key3.record_written);
        assert!(!key3.redundancy_repaired);

        io.operations.clear();
        let key4 =
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(4, 4, 5))
                .unwrap();
        assert!(!key4.legacy_migrated);
        assert!(key4.key_transition);
        assert_eq!(key4.previous_key_id, 3);
        assert_eq!(key4.previous_key_epoch, 3);
        assert_eq!(key4.persisted_floor_before, 4);
        assert_eq!(key4.committed_floor, 5);
        assert_eq!(key4.initial_generation, 2);
        assert_eq!(key4.committed_generation, 3);
        assert_eq!(key4.initial_active_policy_slots, 0);
        assert_eq!(key4.committed_active_policy_slots, 1);

        io.operations.clear();
        let repair =
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(4, 4, 5))
                .unwrap();
        assert!(!repair.key_transition);
        assert!(repair.redundancy_repaired);
        assert_eq!(repair.initial_generation, 3);
        assert_eq!(repair.committed_generation, 4);
        assert_eq!(repair.initial_active_policy_slots, 1);
        assert_eq!(repair.committed_active_policy_slots, 2);
        assert_eq!(repair.writes, 1);
        assert_eq!(repair.flushes, 1);

        io.operations.clear();
        let steady =
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(4, 4, 5))
                .unwrap();
        assert!(!steady.record_written);
        assert!(!steady.redundancy_repaired);
        assert_eq!(steady.initial_generation, 4);
        assert_eq!(steady.committed_generation, 4);
        assert_eq!(steady.initial_active_policy_slots, 2);
        assert_eq!(steady.committed_active_policy_slots, 2);
        assert_eq!(steady.writes, 0);
        assert_eq!(steady.flushes, 0);
        assert_eq!(io.operations, [Operation::Read(5), Operation::Read(6)]);
    }

    #[cfg(feature = "unified-product-key-rotation-runtime")]
    #[test]
    fn retired_or_skipped_manifest_key_epochs_fail_without_mutation() {
        let mut io = MemoryIo::fixture();
        enforce_and_advance_manifest_rollback(
            &mut io,
            2,
            6,
            FORMAT_EPOCH,
            2,
            3,
            manifest_binding(),
        )
        .unwrap();
        io.operations.clear();
        assert_eq!(
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(4, 4, 5),),
            Err(ManifestKeyRotationError::SkippedKeyEpoch {
                artifact_epoch: 4,
                active_epoch: 2,
            })
        );
        assert_eq!(io.operations, [Operation::Read(5), Operation::Read(6)]);

        enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(3, 3, 4))
            .unwrap();
        enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(4, 4, 5))
            .unwrap();
        io.operations.clear();
        assert_eq!(
            enforce_and_rotate_manifest_key_policy(&mut io, manifest_key_rotation_request(3, 3, 6),),
            Err(ManifestKeyRotationError::RetiredKey {
                artifact_epoch: 3,
                active_epoch: 4,
            })
        );
        assert_eq!(io.operations, [Operation::Read(5), Operation::Read(6)]);
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    #[test]
    fn maintenance_audit_state_round_trips_and_seals_its_hash_chain() {
        let binding = maintenance_binding(1);
        let state = MaintenanceAuditState {
            sequence: 1,
            accepted_count: 1,
            operations: binding.operations,
            max_uses: binding.max_uses,
            authorization_sha256: binding.authorization_sha256,
            authorization_id: binding.authorization_id,
            manifest_sha256: binding.manifest_sha256,
            policy_sha256: binding.policy_sha256,
            root_sha256: binding.root_sha256,
            device_binding_sha256: binding.device_binding_sha256,
            previous_chain_sha256: [0; 32],
            chain_sha256: maintenance_audit_chain_sha256([0; 32], binding),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MAINTENANCE_AUDIT_STATE_BYTES);
        assert_eq!(MaintenanceAuditState::decode(&encoded), Ok(state));
        for offset in [
            0, 8, 12, 16, 24, 32, 40, 44, 48, 80, 112, 144, 176, 208, 272, 304, 312,
        ] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                MaintenanceAuditState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    #[test]
    fn maintenance_authorizations_commit_in_order_and_replays_are_read_only() {
        let mut io = MemoryIo::fixture();
        let first = commit_maintenance_authorization(&mut io, maintenance_request(1)).unwrap();
        assert_eq!(
            first,
            MaintenanceAuditEvidence {
                provisioned_before: false,
                resumed: false,
                completed_sequence_before: 0,
                preflight_reads: 0,
                execution_initial_generation: 0,
                execution_initial_slot: u8::MAX,
                execution_initial_valid_slots: 0,
                execution_initial_rejected_slots: 0,
                step_preflight_reads: 0,
                step_provisioned_before: false,
                step_legacy_anchor: false,
                step_sequence_before: 0,
                step_base_sequence_before: 0,
                step_entry_count_before: 0,
                step_ordinal_before: 0,
                step_initial_generation: 0,
                step_initial_slot: u8::MAX,
                step_initial_valid_slots: 0,
                step_initial_rejected_slots: 0,
                previous_sequence: 0,
                committed_sequence: 1,
                previous_accepted_count: 0,
                committed_accepted_count: 1,
                initial_generation: 0,
                committed_generation: 1,
                initial_slot: u8::MAX,
                committed_slot: 0,
                initial_valid_slots: 0,
                initial_rejected_slots: 0,
                committed_valid_slots: 1,
                committed_rejected_slots: 1,
                reads: 4,
                writes: 1,
                flushes: 1,
                previous_chain_sha256: [0; 32],
                committed_chain_sha256: maintenance_audit_chain_sha256(
                    [0; 32],
                    maintenance_binding(1),
                ),
                binding: maintenance_binding(1),
            }
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(6),
                Operation::Read(7),
                Operation::Write(6),
                Operation::Flush,
                Operation::Read(6),
                Operation::Read(7),
            ]
        );

        io.operations.clear();
        let second = commit_maintenance_authorization(&mut io, maintenance_request(2)).unwrap();
        assert!(second.provisioned_before);
        assert_eq!(second.previous_sequence, 1);
        assert_eq!(second.committed_sequence, 2);
        assert_eq!(second.previous_accepted_count, 1);
        assert_eq!(second.committed_accepted_count, 2);
        assert_eq!(second.initial_generation, 1);
        assert_eq!(second.committed_generation, 2);
        assert_eq!(second.initial_slot, 0);
        assert_eq!(second.committed_slot, 1);
        assert_eq!(second.committed_valid_slots, 2);
        assert_eq!(second.previous_chain_sha256, first.committed_chain_sha256);

        io.operations.clear();
        assert_eq!(
            commit_maintenance_authorization(&mut io, maintenance_request(2)),
            Err(MaintenanceAuditError::Replay {
                actual: 2,
                minimum: 3,
            })
        );
        assert_eq!(io.operations, [Operation::Read(6), Operation::Read(7)]);
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    #[test]
    fn maintenance_audit_rejects_bootstrap_gaps_and_namespace_substitution() {
        let mut io = MemoryIo::fixture();
        assert_eq!(
            commit_maintenance_authorization(&mut io, maintenance_request(2)),
            Err(MaintenanceAuditError::BootstrapSequence { actual: 2 })
        );
        assert_eq!(io.operations, [Operation::Read(6), Operation::Read(7)]);

        io.operations.clear();
        commit_maintenance_authorization(&mut io, maintenance_request(1)).unwrap();
        io.operations.clear();
        assert_eq!(
            commit_maintenance_authorization(&mut io, maintenance_request(3)),
            Err(MaintenanceAuditError::SequenceGap {
                actual: 3,
                expected: 2,
            })
        );
        assert_eq!(io.operations, [Operation::Read(6), Operation::Read(7)]);

        let mut foreign = maintenance_request(2);
        foreign.binding.root_sha256 = [0x99; 32];
        io.operations.clear();
        assert_eq!(
            commit_maintenance_authorization(&mut io, foreign),
            Err(MaintenanceAuditError::NamespaceMismatch)
        );
        assert_eq!(io.operations, [Operation::Read(6), Operation::Read(7)]);
    }

    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    #[test]
    fn maintenance_audit_mutation_failure_preserves_the_selected_chain_head() {
        let mut io = MemoryIo::fixture();
        commit_maintenance_authorization(&mut io, maintenance_request(1)).unwrap();
        let selected = io.sectors[6];
        io.tear_write = true;
        assert_eq!(
            commit_maintenance_authorization(&mut io, maintenance_request(2)),
            Err(MaintenanceAuditError::VerificationFailed)
        );
        assert_eq!(io.sectors[6], selected);
        let recovered = recover([&io.sectors[6], &io.sectors[7]], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.slot, 0);
        assert_eq!(
            MaintenanceAuditState::decode(recovered.payload)
                .unwrap()
                .sequence,
            1
        );
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    #[test]
    fn maintenance_execution_state_round_trips_and_seals_runtime_evidence() {
        let binding = maintenance_completion_binding(1);
        let state = MaintenanceExecutionState {
            sequence: 1,
            completed_count: 1,
            operations: binding.operations,
            max_uses: binding.max_uses,
            uses_consumed: binding.uses_consumed,
            rotations_completed: binding.rotations_completed,
            flags: binding.flags,
            appdata_generation: binding.appdata_generation,
            authorization_sha256: binding.authorization_sha256,
            authorization_id: binding.authorization_id,
            manifest_sha256: binding.manifest_sha256,
            policy_sha256: binding.policy_sha256,
            root_sha256: binding.root_sha256,
            device_binding_sha256: binding.device_binding_sha256,
            previous_chain_sha256: [0; 32],
            chain_sha256: maintenance_execution_chain_sha256([0; 32], binding),
            runtime_evidence_sha256: binding.runtime_evidence_sha256,
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MAINTENANCE_EXECUTION_STATE_BYTES);
        assert_eq!(MaintenanceExecutionState::decode(&encoded), Ok(state));
        for offset in [
            0, 8, 12, 16, 24, 32, 40, 44, 48, 52, 56, 64, 96, 128, 160, 192, 224, 256, 288, 320,
            352, 360,
        ] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                MaintenanceExecutionState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    #[test]
    fn maintenance_execution_admission_resumes_only_the_unfinished_exact_head() {
        let mut io = MemoryIo::fixture();
        let first = admit_maintenance_authorization_with_execution(&mut io, maintenance_request(1))
            .unwrap();
        assert!(!first.resumed);
        assert_eq!(first.completed_sequence_before, 0);
        assert_eq!(first.preflight_reads, 4);
        assert_eq!(first.writes, 1);

        let completed =
            commit_maintenance_execution_completion(&mut io, maintenance_completion_request(1))
                .unwrap();
        assert_eq!(completed.previous_sequence, 0);
        assert_eq!(completed.committed_sequence, 1);
        assert_eq!(completed.committed_completed_count, 1);
        assert_eq!(completed.reads, 6);
        assert_eq!(completed.writes, 1);
        assert_eq!(completed.flushes, 1);

        let second =
            admit_maintenance_authorization_with_execution(&mut io, maintenance_request(2))
                .unwrap();
        assert!(!second.resumed);
        assert_eq!(second.completed_sequence_before, 1);
        assert_eq!(second.previous_sequence, 1);
        assert_eq!(second.committed_sequence, 2);

        io.operations.clear();
        let resumed =
            admit_maintenance_authorization_with_execution(&mut io, maintenance_request(2))
                .unwrap();
        assert!(resumed.resumed);
        assert_eq!(resumed.completed_sequence_before, 1);
        assert_eq!(resumed.previous_sequence, 2);
        assert_eq!(resumed.committed_sequence, 2);
        assert_eq!(resumed.initial_generation, resumed.committed_generation);
        assert_eq!(resumed.reads, 0);
        assert_eq!(resumed.writes, 0);
        assert_eq!(resumed.flushes, 0);
        assert_eq!(
            io.operations,
            [
                Operation::Read(6),
                Operation::Read(7),
                Operation::Read(8),
                Operation::Read(9),
            ]
        );

        io.operations.clear();
        assert_eq!(
            admit_maintenance_authorization_with_execution(&mut io, maintenance_request(3)),
            Err(MaintenanceExecutionError::IncompletePredecessor {
                actual: 3,
                unfinished: 2,
            })
        );
        assert_eq!(io.operations.len(), 4);

        commit_maintenance_execution_completion(&mut io, maintenance_completion_request(2))
            .unwrap();
        io.operations.clear();
        assert_eq!(
            admit_maintenance_authorization_with_execution(&mut io, maintenance_request(2)),
            Err(MaintenanceExecutionError::CompletedReplay {
                actual: 2,
                minimum: 3,
            })
        );
        assert_eq!(io.operations.len(), 4);
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    #[test]
    fn maintenance_execution_rejects_resume_binding_substitution_without_mutation() {
        let mut io = MemoryIo::fixture();
        admit_maintenance_authorization_with_execution(&mut io, maintenance_request(1)).unwrap();
        let audit_before = [io.sectors[6], io.sectors[7]];
        let execution_before = [io.sectors[8], io.sectors[9]];
        let mut substituted = maintenance_request(1);
        substituted.binding.authorization_id = [0xa5; 32];
        io.operations.clear();
        assert_eq!(
            admit_maintenance_authorization_with_execution(&mut io, substituted),
            Err(MaintenanceExecutionError::AuthorizationBindingMismatch)
        );
        assert_eq!(io.operations.len(), 4);
        assert_eq!([io.sectors[6], io.sectors[7]], audit_before);
        assert_eq!([io.sectors[8], io.sectors[9]], execution_before);
    }

    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    #[test]
    fn maintenance_execution_torn_completion_preserves_the_prior_head() {
        let mut io = MemoryIo::fixture();
        admit_maintenance_authorization_with_execution(&mut io, maintenance_request(1)).unwrap();
        commit_maintenance_execution_completion(&mut io, maintenance_completion_request(1))
            .unwrap();
        admit_maintenance_authorization_with_execution(&mut io, maintenance_request(2)).unwrap();
        let selected = io.sectors[8];
        io.tear_write = true;
        assert_eq!(
            commit_maintenance_execution_completion(&mut io, maintenance_completion_request(2)),
            Err(MaintenanceExecutionError::VerificationFailed)
        );
        assert_eq!(io.sectors[8], selected);
        let recovered = recover([&io.sectors[8], &io.sectors[9]], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.slot, 0);
        assert_eq!(
            MaintenanceExecutionState::decode(recovered.payload)
                .unwrap()
                .sequence,
            1
        );
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    #[test]
    fn maintenance_step_state_round_trips_and_seals_every_program_binding() {
        let binding = maintenance_step_binding(2, MAINTENANCE_STEP_ROTATION_ONE);
        let state = MaintenanceStepState {
            sequence: 2,
            base_sequence: 2,
            entry_count: 1,
            operations: binding.operations,
            max_uses: binding.max_uses,
            step_ordinal: binding.step_ordinal,
            observed_rotations: binding.observed_rotations,
            flags: binding.flags,
            authorization_sha256: binding.authorization_sha256,
            authorization_id: binding.authorization_id,
            manifest_sha256: binding.manifest_sha256,
            policy_sha256: binding.policy_sha256,
            root_sha256: binding.root_sha256,
            device_binding_sha256: binding.device_binding_sha256,
            effect_sha256: binding.effect_sha256,
            previous_chain_sha256: [0; 32],
            chain_sha256: maintenance_step_chain_sha256([0; 32], 2, 1, binding),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MAINTENANCE_STEP_STATE_BYTES);
        assert_eq!(MaintenanceStepState::decode(&encoded), Ok(state));
        for offset in [
            0, 8, 12, 16, 24, 32, 40, 48, 52, 56, 60, 64, 96, 128, 160, 192, 224, 256, 288, 320,
            352, 360,
        ] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                MaintenanceStepState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }

        for step in [
            MAINTENANCE_STEP_ROTATION_ONE,
            MAINTENANCE_STEP_ROTATION_TWO,
            MAINTENANCE_STEP_DRAIN,
        ] {
            let binding = maintenance_step_binding(7, step);
            assert_eq!(
                binding.effect_sha256,
                maintenance_step_effect_sha256(binding)
            );
        }
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    #[test]
    fn maintenance_steps_migrate_from_m78_replay_and_advance_only_in_order() {
        let mut io = MemoryIo::fixture();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(1, 11),
        )
        .unwrap();
        commit_maintenance_execution_completion(
            &mut io,
            maintenance_completion_request_with_sectors(1, 11),
        )
        .unwrap();

        io.operations.clear();
        let migration =
            preflight_maintenance_step_admission(&mut io, maintenance_request_with_sectors(2, 11))
                .unwrap();
        assert!(!migration.provisioned_before);
        assert!(migration.legacy_anchor);
        assert!(!migration.resumes_current_sequence);
        assert_eq!(migration.audit_sequence, 1);
        assert_eq!(migration.execution_sequence, 1);
        assert_eq!(migration.step_sequence, 0);
        assert_eq!(migration.reads, 6);
        assert_eq!(io.operations.len(), 6);

        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(2, 11),
        )
        .unwrap();
        io.operations.clear();
        let first = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        assert!(!first.provisioned_before);
        assert!(!first.replayed);
        assert_eq!(first.base_sequence, 2);
        assert_eq!(first.previous_entry_count, 0);
        assert_eq!(first.committed_entry_count, 1);
        assert_eq!(first.committed_step_ordinal, MAINTENANCE_STEP_ROTATION_ONE);
        assert_eq!(first.initial_generation, 0);
        assert_eq!(first.committed_generation, 1);
        assert_eq!(first.initial_slot, u8::MAX);
        assert_eq!(first.committed_slot, 0);
        assert_eq!((first.reads, first.writes, first.flushes), (8, 1, 1));
        assert_eq!(
            io.operations,
            [
                Operation::Read(6),
                Operation::Read(7),
                Operation::Read(8),
                Operation::Read(9),
                Operation::Read(10),
                Operation::Read(11),
                Operation::Write(10),
                Operation::Flush,
                Operation::Read(10),
                Operation::Read(11),
            ]
        );

        io.operations.clear();
        let first_replay = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        assert!(first_replay.replayed);
        assert_eq!(first_replay.committed_entry_count, 1);
        assert_eq!(first_replay.initial_generation, 1);
        assert_eq!(first_replay.committed_generation, 1);
        assert_eq!(
            (
                first_replay.reads,
                first_replay.writes,
                first_replay.flushes
            ),
            (6, 0, 0)
        );
        assert_eq!(io.operations.len(), 6);

        let second = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_TWO),
        )
        .unwrap();
        assert_eq!(second.previous_entry_count, 1);
        assert_eq!(second.committed_entry_count, 2);
        assert_eq!(second.previous_step_ordinal, MAINTENANCE_STEP_ROTATION_ONE);
        assert_eq!(second.committed_step_ordinal, MAINTENANCE_STEP_ROTATION_TWO);
        assert_eq!(second.committed_generation, 2);
        assert_eq!(second.committed_slot, 1);

        let lower_step_replay = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        assert!(lower_step_replay.replayed);
        assert_eq!(
            lower_step_replay.requested_step_ordinal,
            MAINTENANCE_STEP_ROTATION_ONE
        );
        assert_eq!(
            lower_step_replay.committed_step_ordinal,
            MAINTENANCE_STEP_ROTATION_TWO
        );
        assert_eq!(lower_step_replay.committed_entry_count, 2);

        let third =
            commit_maintenance_step(&mut io, maintenance_step_request(2, MAINTENANCE_STEP_DRAIN))
                .unwrap();
        assert_eq!(third.previous_entry_count, 2);
        assert_eq!(third.committed_entry_count, 3);
        assert_eq!(third.previous_step_ordinal, MAINTENANCE_STEP_ROTATION_TWO);
        assert_eq!(third.committed_step_ordinal, MAINTENANCE_STEP_DRAIN);
        assert_eq!(third.committed_generation, 3);
        assert_eq!(third.committed_slot, 0);

        let terminal =
            validate_maintenance_step_terminal(&mut io, maintenance_step_terminal_request(2))
                .unwrap();
        assert_eq!(terminal.sequence, 2);
        assert_eq!(terminal.base_sequence, 2);
        assert_eq!(terminal.entry_count, 3);
        assert_eq!(terminal.step_ordinal, MAINTENANCE_STEP_DRAIN);
        assert_eq!(terminal.generation, 3);
        assert_eq!(terminal.reads, 4);
        assert_eq!(terminal.chain_sha256, third.committed_chain_sha256);

        commit_maintenance_execution_completion(
            &mut io,
            maintenance_completion_request_with_sectors(2, 11),
        )
        .unwrap();
        let step_slots = [io.sectors[10], io.sectors[11]];
        assert_eq!(
            commit_maintenance_step(
                &mut io,
                maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
            ),
            Err(MaintenanceStepError::ExecutionAlreadyComplete)
        );
        assert_eq!([io.sectors[10], io.sectors[11]], step_slots);

        let next =
            preflight_maintenance_step_admission(&mut io, maintenance_request_with_sectors(3, 11))
                .unwrap();
        assert!(next.provisioned_before);
        assert!(!next.legacy_anchor);
        assert!(!next.resumes_current_sequence);
        assert_eq!(next.step_sequence, 2);
        assert_eq!(next.step_entry_count, 3);
        assert_eq!(next.step_ordinal, MAINTENANCE_STEP_DRAIN);
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(3, 11),
        )
        .unwrap();
        let next_first = commit_maintenance_step(
            &mut io,
            maintenance_step_request(3, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        assert_eq!(next_first.base_sequence, 2);
        assert_eq!(next_first.previous_entry_count, 3);
        assert_eq!(next_first.committed_entry_count, 4);
        assert_eq!(next_first.previous_sequence, 2);
        assert_eq!(next_first.committed_sequence, 3);
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    #[test]
    fn maintenance_steps_reject_gaps_substitution_and_early_completion_without_mutation() {
        let mut io = MemoryIo::fixture();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(1, 11),
        )
        .unwrap();
        commit_maintenance_execution_completion(
            &mut io,
            maintenance_completion_request_with_sectors(1, 11),
        )
        .unwrap();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(2, 11),
        )
        .unwrap();

        let ledgers_before = [
            io.sectors[6],
            io.sectors[7],
            io.sectors[8],
            io.sectors[9],
            io.sectors[10],
            io.sectors[11],
        ];
        assert_eq!(
            commit_maintenance_step(
                &mut io,
                maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_TWO),
            ),
            Err(MaintenanceStepError::BootstrapStep {
                sequence: 2,
                step: MAINTENANCE_STEP_ROTATION_TWO,
            })
        );
        assert_eq!(
            [
                io.sectors[6],
                io.sectors[7],
                io.sectors[8],
                io.sectors[9],
                io.sectors[10],
                io.sectors[11],
            ],
            ledgers_before
        );

        commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        let step_slots = [io.sectors[10], io.sectors[11]];
        assert_eq!(
            commit_maintenance_step(&mut io, maintenance_step_request(2, MAINTENANCE_STEP_DRAIN),),
            Err(MaintenanceStepError::StepGap {
                actual: MAINTENANCE_STEP_DRAIN,
                expected: MAINTENANCE_STEP_ROTATION_TWO,
            })
        );
        assert_eq!([io.sectors[10], io.sectors[11]], step_slots);
        assert_eq!(
            validate_maintenance_step_terminal(&mut io, maintenance_step_terminal_request(2)),
            Err(MaintenanceStepError::TerminalStepIncomplete)
        );
        assert_eq!(
            preflight_maintenance_step_admission(&mut io, maintenance_request_with_sectors(3, 11),),
            Err(MaintenanceStepError::TerminalStepIncomplete)
        );

        let mut substituted_authorization = maintenance_binding(2);
        substituted_authorization.authorization_id = [0xa5; 32];
        let substituted_binding = MaintenanceStepCommitBinding::for_observation(
            substituted_authorization,
            MAINTENANCE_STEP_ROTATION_TWO,
            2,
            false,
        )
        .unwrap();
        let substituted = MaintenanceStepCommitRequest {
            partition_first_lba: 1,
            partition_sectors: 11,
            expected_format_epoch: FORMAT_EPOCH,
            binding: substituted_binding,
        };
        assert_eq!(
            commit_maintenance_step(&mut io, substituted),
            Err(MaintenanceStepError::AuthorizationBindingMismatch)
        );
        assert_eq!([io.sectors[10], io.sectors[11]], step_slots);
    }

    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    #[test]
    fn maintenance_step_torn_write_falls_back_and_reconciles_from_the_selected_head() {
        let mut io = MemoryIo::fixture();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(1, 11),
        )
        .unwrap();
        commit_maintenance_execution_completion(
            &mut io,
            maintenance_completion_request_with_sectors(1, 11),
        )
        .unwrap();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(2, 11),
        )
        .unwrap();
        commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        let selected = io.sectors[10];

        io.tear_write = true;
        assert_eq!(
            commit_maintenance_step(
                &mut io,
                maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_TWO),
            ),
            Err(MaintenanceStepError::VerificationFailed)
        );
        assert_eq!(io.sectors[10], selected);
        let recovered = recover([&io.sectors[10], &io.sectors[11]], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.slot, 0);
        let recovered_state = MaintenanceStepState::decode(recovered.payload).unwrap();
        assert_eq!(recovered_state.step_ordinal, MAINTENANCE_STEP_ROTATION_ONE);
        assert_eq!(recovered_state.entry_count, 1);

        io.tear_write = false;
        let resumed =
            preflight_maintenance_step_admission(&mut io, maintenance_request_with_sectors(2, 11))
                .unwrap();
        assert!(resumed.resumes_current_sequence);
        assert_eq!(resumed.step_ordinal, MAINTENANCE_STEP_ROTATION_ONE);
        let replay = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        assert!(replay.replayed);
        let second = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_TWO),
        )
        .unwrap();
        assert_eq!(second.committed_generation, 2);
        assert_eq!(second.committed_entry_count, 2);

        io.fail_flush = true;
        io.operations.clear();
        assert_eq!(
            commit_maintenance_step(&mut io, maintenance_step_request(2, MAINTENANCE_STEP_DRAIN),),
            Err(MaintenanceStepError::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::Device,
            })
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(6),
                Operation::Read(7),
                Operation::Read(8),
                Operation::Read(9),
                Operation::Read(10),
                Operation::Read(11),
                Operation::Write(10),
                Operation::Flush,
            ]
        );
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    #[test]
    fn maintenance_plan_state_round_trips_and_seals_identity_phase_and_chain() {
        let binding = maintenance_plan_binding(
            2,
            MAINTENANCE_STEP_ROTATION_ONE,
            MAINTENANCE_PLAN_PHASE_PREPARED,
        );
        let state = MaintenancePlanState {
            sequence: 2,
            base_sequence: 2,
            transition_count: 1,
            operations: binding.operations,
            max_uses: binding.max_uses,
            operation_ordinal: binding.operation_ordinal,
            phase: binding.phase,
            flags: binding.flags,
            authorization_sha256: binding.authorization_sha256,
            authorization_id: binding.authorization_id,
            manifest_sha256: binding.manifest_sha256,
            policy_sha256: binding.policy_sha256,
            root_sha256: binding.root_sha256,
            device_binding_sha256: binding.device_binding_sha256,
            effect_sha256: binding.effect_sha256,
            operation_instance_id: binding.operation_instance_id,
            idempotency_key: binding.idempotency_key,
            previous_chain_sha256: [0; 32],
            chain_sha256: maintenance_plan_chain_sha256([0; 32], 2, 1, binding),
        };
        let encoded = state.encode().unwrap();
        assert_eq!(encoded.len(), MAINTENANCE_PLAN_STATE_BYTES);
        assert_eq!(MaintenancePlanState::decode(&encoded), Ok(state));
        for offset in [
            0, 8, 12, 16, 24, 32, 40, 48, 52, 56, 60, 64, 96, 128, 160, 192, 224, 256, 288, 320,
            352, 384, 416,
        ] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                MaintenancePlanState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
        let authorization = maintenance_binding(2);
        let plan_id = maintenance_plan_id_sha256(authorization);
        assert_ne!(plan_id, [0; 32]);
        assert_eq!(
            binding.operation_instance_id,
            maintenance_plan_operation_instance_id_sha256(
                plan_id,
                binding.operation_ordinal,
                binding.effect_sha256,
            )
        );
        assert_eq!(
            binding.idempotency_key,
            maintenance_plan_idempotency_key_sha256(
                plan_id,
                binding.operation_instance_id,
                binding.effect_sha256,
            )
        );
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    #[test]
    fn maintenance_plan_reconciles_result_unknown_and_confirms_all_three_effects() {
        let mut io = MemoryIo::fixture();
        prepare_m80_sequence_two(&mut io);

        let prepared = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_PREPARED,
            ),
        )
        .unwrap();
        assert!(!prepared.replayed);
        assert_eq!(prepared.committed_transition_count, 1);
        assert_eq!(
            (prepared.reads, prepared.writes, prepared.flushes),
            (12, 1, 1)
        );
        let applying = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_APPLYING,
            ),
        )
        .unwrap();
        assert_eq!(applying.committed_transition_count, 2);

        let unknown =
            preflight_maintenance_plan_admission(&mut io, maintenance_request_with_sectors(2, 13))
                .unwrap();
        assert!(unknown.result_unknown);
        assert!(!unknown.effect_observed_unconfirmed);
        assert_eq!(unknown.phase, MAINTENANCE_PLAN_PHASE_APPLYING);

        let first = commit_maintenance_step(
            &mut io,
            maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_ONE),
        )
        .unwrap();
        let observed =
            preflight_maintenance_plan_admission(&mut io, maintenance_request_with_sectors(2, 13))
                .unwrap();
        assert!(!observed.result_unknown);
        assert!(observed.effect_observed_unconfirmed);
        assert_eq!(
            observed.step.step_effect_sha256,
            first.binding.effect_sha256
        );

        let mut last_plan = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_CONFIRMED,
            ),
        )
        .unwrap();
        assert_eq!(last_plan.committed_transition_count, 3);

        for (operation, step) in [
            (
                MAINTENANCE_STEP_ROTATION_TWO,
                maintenance_step_request(2, MAINTENANCE_STEP_ROTATION_TWO),
            ),
            (
                MAINTENANCE_STEP_DRAIN,
                maintenance_step_request(2, MAINTENANCE_STEP_DRAIN),
            ),
        ] {
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request(2, operation, MAINTENANCE_PLAN_PHASE_PREPARED),
            )
            .unwrap();
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request(2, operation, MAINTENANCE_PLAN_PHASE_APPLYING),
            )
            .unwrap();
            commit_maintenance_step(&mut io, step).unwrap();
            last_plan = commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request(2, operation, MAINTENANCE_PLAN_PHASE_CONFIRMED),
            )
            .unwrap();
        }
        assert_eq!(last_plan.committed_transition_count, 9);
        assert_eq!(last_plan.committed_generation, 9);
        let step_terminal =
            validate_maintenance_step_terminal(&mut io, maintenance_step_terminal_request(2))
                .unwrap();
        let terminal = validate_maintenance_plan_terminal(
            &mut io,
            MaintenancePlanTerminalRequest {
                partition_first_lba: 1,
                partition_sectors: 13,
                expected_format_epoch: FORMAT_EPOCH,
                authorization: maintenance_binding(2),
                expected_step_chain_sha256: step_terminal.chain_sha256,
            },
        )
        .unwrap();
        assert_eq!(terminal.transition_count, 9);
        assert_eq!(terminal.operation_ordinal, MAINTENANCE_STEP_DRAIN);
        assert_eq!(terminal.phase, MAINTENANCE_PLAN_PHASE_CONFIRMED);
        assert_eq!(terminal.chain_sha256, last_plan.committed_chain_sha256);
        assert_eq!(terminal.reads, 8);
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    #[test]
    fn maintenance_plan_compensates_only_before_apply_and_then_fails_closed() {
        let mut io = MemoryIo::fixture();
        prepare_m80_sequence_two(&mut io);
        commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_PREPARED,
            ),
        )
        .unwrap();
        let compensated = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_COMPENSATED,
            ),
        )
        .unwrap();
        assert_eq!(compensated.committed_transition_count, 2);
        assert_eq!(
            compensated.committed_phase,
            MAINTENANCE_PLAN_PHASE_COMPENSATED
        );
        assert_eq!(
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request(
                    2,
                    MAINTENANCE_STEP_ROTATION_ONE,
                    MAINTENANCE_PLAN_PHASE_APPLYING,
                ),
            ),
            Err(MaintenancePlanError::PlanCompensated)
        );
        assert_eq!(
            preflight_maintenance_plan_admission(&mut io, maintenance_request_with_sectors(3, 13)),
            Err(MaintenancePlanError::LedgerDivergence)
        );
        assert_eq!([io.sectors[10], io.sectors[11]], [[0; SECTOR_SIZE]; 2]);
    }

    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    #[test]
    fn maintenance_plan_rejects_gaps_and_repairs_a_corrupt_newest_slot() {
        let mut io = MemoryIo::fixture();
        prepare_m80_sequence_two(&mut io);
        commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_PREPARED,
            ),
        )
        .unwrap();
        assert_eq!(
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request(
                    2,
                    MAINTENANCE_STEP_ROTATION_ONE,
                    MAINTENANCE_PLAN_PHASE_CONFIRMED,
                ),
            ),
            Err(MaintenancePlanError::TransitionGap)
        );
        let applying = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_APPLYING,
            ),
        )
        .unwrap();
        let newest_lba =
            1 + MAINTENANCE_PLAN_SLOT_RELATIVE_LBAS[usize::from(applying.committed_slot)] as usize;
        io.sectors[newest_lba][100] ^= 1;
        let fallback =
            preflight_maintenance_plan_admission(&mut io, maintenance_request_with_sectors(2, 13))
                .unwrap();
        assert_eq!(fallback.phase, MAINTENANCE_PLAN_PHASE_PREPARED);
        assert_eq!(fallback.valid_slots, 1);
        assert_eq!(fallback.rejected_slots, 1);
        let repaired = commit_maintenance_plan_transition(
            &mut io,
            maintenance_plan_request(
                2,
                MAINTENANCE_STEP_ROTATION_ONE,
                MAINTENANCE_PLAN_PHASE_APPLYING,
            ),
        )
        .unwrap();
        assert!(!repaired.replayed);
        assert_eq!(repaired.initial_rejected_slots, 1);
        assert_eq!(repaired.committed_valid_slots, 2);
        assert_eq!(repaired.committed_rejected_slots, 0);
        assert_eq!(repaired.committed_transition_count, 2);
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    #[test]
    fn signed_maintenance_plan_state_round_trips_and_seals_program_chain() {
        let authorization = maintenance_binding(2);
        let binding = signed_maintenance_plan_binding(2);
        let state = SignedMaintenancePlanState {
            binding,
            previous_chain_sha256: [0; 32],
            chain_sha256: signed_maintenance_plan_chain_sha256([0; 32], binding),
        };
        let encoded = state.encode(authorization).unwrap();
        assert_eq!(encoded.len(), SIGNED_MAINTENANCE_PLAN_STATE_BYTES);
        assert_eq!(SignedMaintenancePlanState::decode(&encoded), Ok(state));
        for offset in [
            0, 8, 12, 16, 24, 28, 32, 36, 40, 72, 104, 136, 168, 200, 232, 264, 296, 328,
        ] {
            let mut corrupt = encoded;
            corrupt[offset] ^= 1;
            assert!(
                SignedMaintenancePlanState::decode(&corrupt).is_err(),
                "offset {offset}"
            );
        }
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    #[test]
    fn signed_maintenance_plan_commits_once_replays_read_only_and_rejects_substitution() {
        let mut io = MemoryIo::fixture();
        prepare_m81_predecessor(&mut io);
        io.operations.clear();

        let preflight =
            preflight_signed_maintenance_plan(&mut io, signed_maintenance_plan_request(2)).unwrap();
        assert!(!preflight.provisioned_before);
        assert!(!preflight.exact_replay);
        assert!(preflight.predecessor_complete);
        assert_eq!(preflight.reads, 10);

        io.operations.clear();
        let committed =
            commit_signed_maintenance_plan(&mut io, signed_maintenance_plan_request(2)).unwrap();
        assert!(!committed.provisioned_before);
        assert!(!committed.replayed);
        assert!(committed.predecessor_complete);
        assert_eq!(committed.committed_sequence, 2);
        assert_eq!(committed.committed_generation, 1);
        assert_eq!(
            (committed.reads, committed.writes, committed.flushes),
            (14, 1, 1)
        );

        io.operations.clear();
        let replay =
            commit_signed_maintenance_plan(&mut io, signed_maintenance_plan_request(2)).unwrap();
        assert!(replay.provisioned_before);
        assert!(replay.replayed);
        assert_eq!(replay.committed_generation, committed.committed_generation);
        assert_eq!(
            replay.committed_chain_sha256,
            committed.committed_chain_sha256
        );
        assert_eq!((replay.reads, replay.writes, replay.flushes), (10, 0, 0));

        let mut substituted = signed_maintenance_plan_request(2);
        substituted.binding.program_sha256[0] ^= 1;
        io.operations.clear();
        assert_eq!(
            preflight_signed_maintenance_plan(&mut io, substituted),
            Err(SignedMaintenancePlanError::ProgramBindingMismatch)
        );
        assert!(
            io.operations
                .iter()
                .all(|operation| matches!(operation, Operation::Read(_)))
        );
    }

    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    #[test]
    fn signed_maintenance_plan_terminal_binds_the_confirmed_m80_head() {
        let mut io = MemoryIo::fixture();
        prepare_m81_predecessor(&mut io);
        let program =
            commit_signed_maintenance_plan(&mut io, signed_maintenance_plan_request(2)).unwrap();
        admit_maintenance_authorization_with_execution(
            &mut io,
            maintenance_request_with_sectors(2, 15),
        )
        .unwrap();

        let mut last_plan = None;
        for operation in [
            MAINTENANCE_STEP_ROTATION_ONE,
            MAINTENANCE_STEP_ROTATION_TWO,
            MAINTENANCE_STEP_DRAIN,
        ] {
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request_with_sectors(
                    2,
                    operation,
                    MAINTENANCE_PLAN_PHASE_PREPARED,
                    15,
                ),
            )
            .unwrap();
            commit_maintenance_plan_transition(
                &mut io,
                maintenance_plan_request_with_sectors(
                    2,
                    operation,
                    MAINTENANCE_PLAN_PHASE_APPLYING,
                    15,
                ),
            )
            .unwrap();
            commit_maintenance_step(
                &mut io,
                maintenance_step_request_with_sectors(2, operation, 15),
            )
            .unwrap();
            last_plan = Some(
                commit_maintenance_plan_transition(
                    &mut io,
                    maintenance_plan_request_with_sectors(
                        2,
                        operation,
                        MAINTENANCE_PLAN_PHASE_CONFIRMED,
                        15,
                    ),
                )
                .unwrap(),
            );
        }
        let last_plan = last_plan.unwrap();
        let terminal = validate_signed_maintenance_plan_terminal(
            &mut io,
            SignedMaintenancePlanTerminalRequest {
                partition_first_lba: 1,
                partition_sectors: 15,
                expected_format_epoch: FORMAT_EPOCH,
                authorization: maintenance_binding(2),
                binding: signed_maintenance_plan_binding(2),
                expected_plan_chain_sha256: last_plan.committed_chain_sha256,
            },
        )
        .unwrap();
        assert_eq!(terminal.sequence, 2);
        assert_eq!(
            terminal.program_chain_sha256,
            program.committed_chain_sha256
        );
        assert_eq!(terminal.plan_chain_sha256, last_plan.committed_chain_sha256);
        assert_eq!(terminal.reads, 10);
    }

    #[test]
    fn device_health_transaction_upgrades_legacy_without_persisting_offline() {
        let mut io = MemoryIo::fixture();
        let evidence =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        assert_eq!(
            evidence.persistence,
            AdvanceEvidence {
                initial_generation: 0,
                committed_generation: 1,
                initial_slot: 0,
                committed_slot: 1,
                initial_valid_slots: 1,
                initial_rejected_slots: 1,
                reads: 5,
                writes: 1,
                flushes: 1,
            }
        );
        assert!(evidence.legacy_upgrade);
        assert!(!evidence.persisted_contract_present);
        assert!(!evidence.prior_boot_open);
        assert!(!evidence.contract_changed);
        assert!(!evidence.reprobe_required);
        assert!(evidence.reprobe_verified);
        assert!(evidence.current_boot_open);
        assert!(!evidence.offline_persisted);
        assert!(!evidence.offline_from_record);
        let committed = DataRecord::decode(&io.sectors[4], FORMAT_EPOCH).unwrap();
        assert_eq!(
            DeviceHealthState::decode(committed.payload),
            Ok(DeviceHealthState {
                boot_count: 1,
                boot_open: true,
                contract: device_contract(),
            })
        );
    }

    #[test]
    fn open_previous_boot_requires_and_records_a_fresh_reprobe() {
        let mut io = MemoryIo::fixture();
        advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
            .unwrap();
        io.operations.clear();
        let evidence =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        assert!(!evidence.legacy_upgrade);
        assert!(evidence.persisted_contract_present);
        assert!(evidence.prior_boot_open);
        assert!(!evidence.contract_changed);
        assert!(evidence.reprobe_required);
        assert!(evidence.reprobe_verified);
        assert_eq!(evidence.persistence.initial_generation, 1);
        assert_eq!(evidence.persistence.committed_generation, 2);
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(3),
                Operation::Flush,
                Operation::Read(3),
                Operation::Read(4),
            ]
        );
    }

    #[test]
    fn changed_contract_is_a_reprobe_hint_and_not_an_offline_decision() {
        let mut io = MemoryIo::fixture();
        advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
            .unwrap();
        let changed = DeviceHealthContract {
            transport_base: 0x0a00_3c00,
            ..device_contract()
        };
        let evidence =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, changed).unwrap();
        assert!(evidence.prior_boot_open);
        assert!(evidence.contract_changed);
        assert!(evidence.reprobe_required);
        assert!(evidence.reprobe_verified);
        assert!(!evidence.offline_persisted);
        assert!(!evidence.offline_from_record);
        assert_eq!(evidence.current_contract, changed);
    }

    #[test]
    fn ordinary_boot_counter_preserves_an_existing_health_payload() {
        let mut io = MemoryIo::fixture();
        advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
            .unwrap();
        io.operations.clear();

        let evidence = advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH).unwrap();
        assert_eq!(evidence.initial_generation, 1);
        assert_eq!(evidence.committed_generation, 2);
        assert_eq!(evidence.initial_slot, 1);
        assert_eq!(evidence.committed_slot, 0);
        let record = DataRecord::decode(&io.sectors[3], FORMAT_EPOCH).unwrap();
        assert_eq!(
            DeviceHealthState::decode(record.payload),
            Ok(DeviceHealthState {
                boot_count: 2,
                boot_open: true,
                contract: device_contract(),
            })
        );
    }

    #[test]
    fn malformed_health_payload_fails_closed_before_any_mutation() {
        let mut io = MemoryIo::fixture();
        io.sectors[4] = DataRecord::encode(FORMAT_EPOCH, 1, b"not-health").unwrap();
        assert_eq!(
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract()),
            Err(DeviceHealthAdvanceError::DeviceHealthState(
                DeviceHealthStateError::WrongSize
            ))
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_closes_the_exact_open_session_in_strict_order() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        io.operations.clear();

        let closed = close_device_health_for_clean_shutdown(
            &mut io,
            2,
            4,
            FORMAT_EPOCH,
            opened.persistence.committed_generation,
            device_contract(),
        )
        .unwrap();
        assert_eq!(
            closed.persistence,
            AdvanceEvidence {
                initial_generation: 1,
                committed_generation: 2,
                initial_slot: 1,
                committed_slot: 0,
                initial_valid_slots: 2,
                initial_rejected_slots: 0,
                reads: 5,
                writes: 1,
                flushes: 1,
            }
        );
        assert_eq!(closed.open_generation, 1);
        assert!(closed.prior_boot_open);
        assert!(closed.contract_matched);
        assert!(!closed.current_boot_open);
        assert!(!closed.offline_persisted);
        assert!(!closed.offline_from_record);
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(3),
                Operation::Flush,
                Operation::Read(3),
                Operation::Read(4),
            ]
        );
        let record = DataRecord::decode(&io.sectors[3], FORMAT_EPOCH).unwrap();
        assert_eq!(
            DeviceHealthState::decode(record.payload),
            Ok(DeviceHealthState {
                boot_count: 2,
                boot_open: false,
                contract: device_contract(),
            })
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn closed_session_removes_only_the_next_boot_unclosed_hint() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        close_device_health_for_clean_shutdown(
            &mut io,
            2,
            4,
            FORMAT_EPOCH,
            opened.persistence.committed_generation,
            device_contract(),
        )
        .unwrap();
        io.operations.clear();

        let next =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        assert!(!next.legacy_upgrade);
        assert!(next.persisted_contract_present);
        assert!(!next.prior_boot_open);
        assert!(!next.contract_changed);
        assert!(!next.reprobe_required);
        assert!(next.reprobe_verified);
        assert!(next.current_boot_open);
        assert!(!next.offline_persisted);
        assert!(!next.offline_from_record);
        assert_eq!(next.persistence.initial_generation, 2);
        assert_eq!(next.persistence.committed_generation, 3);
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_rejects_a_stale_session_generation_before_mutation() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        io.operations.clear();

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                opened.persistence.committed_generation + 1,
                device_contract(),
            ),
            Err(DeviceHealthCloseError::SessionGenerationMismatch)
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_rejects_a_changed_contract_before_mutation() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        io.operations.clear();
        let changed = DeviceHealthContract {
            transport_base: 0x0a00_3c00,
            ..device_contract()
        };

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                opened.persistence.committed_generation,
                changed,
            ),
            Err(DeviceHealthCloseError::ContractMismatch)
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_rejects_replay_of_an_already_closed_session() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        let first = close_device_health_for_clean_shutdown(
            &mut io,
            2,
            4,
            FORMAT_EPOCH,
            opened.persistence.committed_generation,
            device_contract(),
        )
        .unwrap();
        io.operations.clear();

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                first.persistence.committed_generation,
                device_contract(),
            ),
            Err(DeviceHealthCloseError::SessionAlreadyClosed)
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_does_not_verify_a_torn_close_record() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        io.operations.clear();
        io.tear_write = true;

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                opened.persistence.committed_generation,
                device_contract(),
            ),
            Err(DeviceHealthCloseError::VerificationFailed)
        );
        let recovered = recover([&io.sectors[3], &io.sectors[4]], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.generation, 1);
        assert_eq!(recovered.slot, 1);
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_stops_after_a_failed_flush() {
        let mut io = MemoryIo::fixture();
        let opened =
            advance_device_health_after_reprobe(&mut io, 2, 4, FORMAT_EPOCH, device_contract())
                .unwrap();
        io.operations.clear();
        io.fail_flush = true;

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                opened.persistence.committed_generation,
                device_contract(),
            ),
            Err(DeviceHealthCloseError::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::Device,
            })
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(3),
                Operation::Flush,
            ]
        );
    }

    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    #[test]
    fn clean_shutdown_rejects_generation_exhaustion_before_mutation() {
        let mut io = MemoryIo::fixture();
        let health = DeviceHealthState {
            boot_count: u64::MAX,
            boot_open: true,
            contract: device_contract(),
        }
        .encode()
        .unwrap();
        io.sectors[3] = DataRecord::encode(FORMAT_EPOCH, u64::MAX, &health).unwrap();

        assert_eq!(
            close_device_health_for_clean_shutdown(
                &mut io,
                2,
                4,
                FORMAT_EPOCH,
                u64::MAX,
                device_contract(),
            ),
            Err(DeviceHealthCloseError::Recovery(
                RecoveryError::GenerationExhausted
            ))
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[test]
    fn durable_transaction_updates_inactive_slots_in_strict_order() {
        let mut io = MemoryIo::fixture();
        let first = advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH).unwrap();
        assert_eq!(
            first,
            AdvanceEvidence {
                initial_generation: 0,
                committed_generation: 1,
                initial_slot: 0,
                committed_slot: 1,
                initial_valid_slots: 1,
                initial_rejected_slots: 1,
                reads: 5,
                writes: 1,
                flushes: 1,
            }
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(4),
                Operation::Flush,
                Operation::Read(3),
                Operation::Read(4),
            ]
        );
        let initial_sector = io.sectors[3];

        io.operations.clear();
        let second = advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH).unwrap();
        assert_eq!(second.initial_generation, 1);
        assert_eq!(second.committed_generation, 2);
        assert_eq!(second.initial_slot, 1);
        assert_eq!(second.committed_slot, 0);
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(3),
                Operation::Flush,
                Operation::Read(3),
                Operation::Read(4),
            ]
        );
        assert_ne!(io.sectors[3], initial_sector);
        assert_eq!(
            DataRecord::decode(&io.sectors[3], FORMAT_EPOCH)
                .unwrap()
                .generation,
            2
        );
        assert_eq!(
            DataRecord::decode(&io.sectors[4], FORMAT_EPOCH)
                .unwrap()
                .generation,
            1
        );
    }

    #[test]
    fn durable_transaction_does_not_verify_a_torn_write() {
        let mut io = MemoryIo::fixture();
        let old = io.sectors[3];
        io.tear_write = true;
        assert_eq!(
            advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH),
            Err(AdvanceError::VerificationFailed)
        );
        assert_eq!(io.sectors[3], old);
        let recovered = recover([&io.sectors[3], &io.sectors[4]], FORMAT_EPOCH).unwrap();
        assert_eq!(recovered.generation, 0);
        assert_eq!(recovered.slot, 0);
    }

    #[test]
    fn durable_transaction_stops_after_a_failed_flush() {
        let mut io = MemoryIo::fixture();
        io.fail_flush = true;
        assert_eq!(
            advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH),
            Err(AdvanceError::Io {
                phase: IoPhase::Flush,
                error: PersistIoError::Device,
            })
        );
        assert_eq!(
            io.operations,
            [
                Operation::Read(2),
                Operation::Read(3),
                Operation::Read(4),
                Operation::Write(4),
                Operation::Flush,
            ]
        );
    }

    #[test]
    fn durable_transaction_rejects_bounds_and_state_generation_mismatch() {
        let mut io = MemoryIo::fixture();
        assert_eq!(
            advance_boot_state(&mut io, 13, 4, FORMAT_EPOCH),
            Err(AdvanceError::PartitionBounds)
        );
        assert!(io.operations.is_empty());

        io.sectors[3] = DataRecord::encode(FORMAT_EPOCH, 4, &state(3)).unwrap();
        assert_eq!(
            advance_boot_state(&mut io, 2, 4, FORMAT_EPOCH),
            Err(AdvanceError::StateGenerationMismatch)
        );
        assert_eq!(
            io.operations,
            [Operation::Read(2), Operation::Read(3), Operation::Read(4)]
        );
    }

    #[test]
    fn durable_transaction_rejects_foreign_and_zero_format_epochs() {
        let mut io = MemoryIo::fixture();
        assert_eq!(
            advance_boot_state(&mut io, 2, 4, [0; FORMAT_EPOCH_BYTES]),
            Err(AdvanceError::Superblock(SuperblockError::ZeroFormatEpoch))
        );
        assert!(io.operations.is_empty());

        assert_eq!(
            advance_boot_state(&mut io, 2, 4, FOREIGN_EPOCH),
            Err(AdvanceError::Superblock(
                SuperblockError::FormatEpochMismatch
            ))
        );
        assert_eq!(io.operations, [Operation::Read(2)]);
    }

    #[test]
    fn crc32_matches_the_ieee_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
