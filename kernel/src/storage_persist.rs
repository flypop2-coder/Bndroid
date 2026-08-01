//! IRQ-backed adapter for the host-testable M25 persistence transaction.

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
use bndroid_kernel::persist::{
    DeviceHealthCloseError, DeviceHealthCloseEvidence, close_device_health_for_clean_shutdown,
};
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
use bndroid_kernel::persist::{
    MaintenanceAuditError, MaintenanceAuditEvidence, MaintenanceAuditRequest,
    commit_maintenance_authorization,
};
#[cfg(feature = "unified-product-maintenance-execution-runtime")]
use bndroid_kernel::persist::{
    MaintenanceExecutionCompletionRequest, MaintenanceExecutionError, MaintenanceExecutionEvidence,
    admit_maintenance_authorization_with_execution, commit_maintenance_execution_completion,
};
#[cfg(feature = "unified-product-maintenance-plan-runtime")]
use bndroid_kernel::persist::{
    MaintenancePlanAdmissionEvidence, MaintenancePlanError, MaintenancePlanEvidence,
    MaintenancePlanTerminalEvidence, MaintenancePlanTerminalRequest,
    MaintenancePlanTransitionRequest, commit_maintenance_plan_transition,
    preflight_maintenance_plan_admission, validate_maintenance_plan_terminal,
};
#[cfg(feature = "unified-product-maintenance-step-runtime")]
use bndroid_kernel::persist::{
    MaintenanceStepAdmissionEvidence, MaintenanceStepCommitRequest, MaintenanceStepError,
    MaintenanceStepEvidence, MaintenanceStepTerminalEvidence, MaintenanceStepTerminalRequest,
    commit_maintenance_step, preflight_maintenance_step_admission,
    validate_maintenance_step_terminal,
};
#[cfg(feature = "unified-product-key-rotation-runtime")]
use bndroid_kernel::persist::{
    ManifestKeyRotationError, ManifestKeyRotationEvidence, ManifestKeyRotationRequest,
    enforce_and_rotate_manifest_key_policy,
};
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
use bndroid_kernel::persist::{
    ManifestRollbackBinding, ManifestRollbackError, ManifestRollbackEvidence,
    enforce_and_advance_manifest_rollback,
};
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
use bndroid_kernel::persist::{
    SignedMaintenancePlanAdmissionEvidence, SignedMaintenancePlanError,
    SignedMaintenancePlanEvidence, SignedMaintenancePlanRequest,
    SignedMaintenancePlanTerminalEvidence, SignedMaintenancePlanTerminalRequest,
    commit_signed_maintenance_plan, preflight_signed_maintenance_plan,
    validate_signed_maintenance_plan_terminal,
};
use bndroid_kernel::{
    block::Sector,
    persist::{
        AdvanceError, AdvanceEvidence, DurableSectorIo, FormatEpoch, PersistIoError,
        advance_boot_state,
    },
};
#[cfg(feature = "storage-server-persistent-health-runtime")]
use bndroid_kernel::{
    persist::{
        DeviceHealthAdvanceError, DeviceHealthAdvanceEvidence, DeviceHealthContract,
        advance_device_health_after_reprobe,
    },
    virtio::{BLOCK_SECTOR_SIZE, QUEUE_SIZE, WRITABLE_FEATURES},
};

use crate::{
    driver::virtio::block::{BlockError, PhysicalCounter, deadline_after},
    storage::{
        self, DATA_PARTITION_FIRST_LBA, DATA_PARTITION_SECTORS, StorageError, flush_irq,
        read_sector_irq, write_data_sector_irq,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoragePersistError {
    DataPartitionContract,
    InvalidClock,
    ReadOnlyDevice,
    MissingFlush,
    #[cfg(feature = "storage-server-persistent-health-runtime")]
    InvalidTransportAddress,
    #[cfg(feature = "storage-server-persistent-health-runtime")]
    ReprobeNotVerified,
    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    BootSessionAlreadyPublished,
    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    BootSessionUnavailable,
    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    CleanShutdownNotQuiescent,
    Storage(StorageError),
    Transaction(AdvanceError),
    #[cfg(feature = "unified-product-persistent-rollback-runtime")]
    ManifestRollbackTransaction(ManifestRollbackError),
    #[cfg(feature = "unified-product-key-rotation-runtime")]
    ManifestKeyRotationTransaction(ManifestKeyRotationError),
    #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
    MaintenanceAuditTransaction(MaintenanceAuditError),
    #[cfg(feature = "unified-product-maintenance-execution-runtime")]
    MaintenanceExecutionTransaction(MaintenanceExecutionError),
    #[cfg(feature = "unified-product-maintenance-step-runtime")]
    MaintenanceStepTransaction(MaintenanceStepError),
    #[cfg(feature = "unified-product-maintenance-plan-runtime")]
    MaintenancePlanTransaction(MaintenancePlanError),
    #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
    SignedMaintenancePlanTransaction(SignedMaintenancePlanError),
    #[cfg(feature = "storage-server-persistent-health-runtime")]
    DeviceHealthTransaction(DeviceHealthAdvanceError),
    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    DeviceHealthCloseTransaction(DeviceHealthCloseError),
}

impl StoragePersistError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DataPartitionContract => {
                "GPT data partition does not match the bounded persistence window"
            }
            Self::InvalidClock => "architectural counter cannot bound persistent-data I/O",
            Self::ReadOnlyDevice => "persistent-data device is physically read-only",
            Self::MissingFlush => "writable block device did not retain cache-flush support",
            #[cfg(feature = "storage-server-persistent-health-runtime")]
            Self::InvalidTransportAddress => {
                "persistent device-health transport address is invalid"
            }
            #[cfg(feature = "storage-server-persistent-health-runtime")]
            Self::ReprobeNotVerified => {
                "persistent device-health update lacked a fresh kernel reprobe"
            }
            #[cfg(feature = "storage-server-clean-shutdown-runtime")]
            Self::BootSessionAlreadyPublished => {
                "clean-shutdown boot session was already published in this kernel"
            }
            #[cfg(feature = "storage-server-clean-shutdown-runtime")]
            Self::BootSessionUnavailable => {
                "clean-shutdown boot session is unavailable or already consumed"
            }
            #[cfg(feature = "storage-server-clean-shutdown-runtime")]
            Self::CleanShutdownNotQuiescent => {
                "clean-shutdown persistence requires a healthy idle block device"
            }
            Self::Storage(error) => error.as_str(),
            Self::Transaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-persistent-rollback-runtime")]
            Self::ManifestRollbackTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-key-rotation-runtime")]
            Self::ManifestKeyRotationTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-maintenance-authorization-runtime")]
            Self::MaintenanceAuditTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-maintenance-execution-runtime")]
            Self::MaintenanceExecutionTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-maintenance-step-runtime")]
            Self::MaintenanceStepTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-maintenance-plan-runtime")]
            Self::MaintenancePlanTransaction(error) => error.as_str(),
            #[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
            Self::SignedMaintenancePlanTransaction(error) => error.as_str(),
            #[cfg(feature = "storage-server-persistent-health-runtime")]
            Self::DeviceHealthTransaction(error) => error.as_str(),
            #[cfg(feature = "storage-server-clean-shutdown-runtime")]
            Self::DeviceHealthCloseTransaction(error) => error.as_str(),
        }
    }
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
static OPEN_SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
static OPEN_SESSION_EPOCH_LOW: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
static OPEN_SESSION_EPOCH_HIGH: AtomicU64 = AtomicU64::new(0);

struct IrqDurableIo<'a> {
    clock: &'a mut PhysicalCounter,
    deadline_budget: u64,
    sector_count: u64,
}

impl IrqDurableIo<'_> {
    fn deadline(&mut self) -> Result<u64, PersistIoError> {
        deadline_after(self.clock, self.deadline_budget).ok_or(PersistIoError::Device)
    }
}

impl DurableSectorIo for IrqDurableIo<'_> {
    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), PersistIoError> {
        let deadline = self.deadline()?;
        read_sector_irq(lba, output, self.clock, deadline).map_err(map_read_error)
    }

    fn write_sector(&mut self, lba: u64, input: &Sector) -> Result<(), PersistIoError> {
        let deadline = self.deadline()?;
        write_data_sector_irq(lba, input, self.clock, deadline).map_err(map_mutating_error)
    }

    fn flush(&mut self) -> Result<(), PersistIoError> {
        let deadline = self.deadline()?;
        flush_irq(self.clock, deadline).map_err(map_mutating_error)
    }
}

pub fn advance(
    counter_frequency: u64,
    partition_first_lba: u64,
    partition_sectors: u64,
    format_epoch: FormatEpoch,
) -> Result<AdvanceEvidence, StoragePersistError> {
    if partition_first_lba != DATA_PARTITION_FIRST_LBA
        || partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    advance_boot_state(
        &mut io,
        partition_first_lba,
        partition_sectors,
        format_epoch,
    )
    .map_err(StoragePersistError::Transaction)
}

/// Enforces the signed-manifest rollback floor against the kernel-only
/// `BNDROID_DATA` double-slot ledger.
///
/// This adapter is called from the pre-EL0 boot monitor while local IRQs are
/// enabled. It deliberately reuses the same bounded virtio-blk completion,
/// FLUSH, timeout and mutation-outcome rules as the other persistence
/// transactions; no sector or mutable device reference escapes this module.
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
pub fn enforce_verified_manifest_rollback(
    counter_frequency: u64,
    partition_first_lba: u64,
    partition_sectors: u64,
    format_epoch: FormatEpoch,
    bootstrap_floor: u32,
    artifact_index: u32,
    binding: ManifestRollbackBinding,
) -> Result<ManifestRollbackEvidence, StoragePersistError> {
    if partition_first_lba != DATA_PARTITION_FIRST_LBA
        || partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    enforce_and_advance_manifest_rollback(
        &mut io,
        partition_first_lba,
        partition_sectors,
        format_epoch,
        bootstrap_floor,
        artifact_index,
        binding,
    )
    .map_err(StoragePersistError::ManifestRollbackTransaction)
}

/// Enforces M76's ordered public-key epoch on the same kernel-only slots.
///
/// The cryptographic verifier supplies the active key and compiled policy
/// digests. Only the reviewed M75 key-2 binding may enter as a legacy
/// predecessor, and no block authority is exposed to EL0.
#[cfg(feature = "unified-product-key-rotation-runtime")]
pub fn enforce_verified_manifest_key_rotation(
    counter_frequency: u64,
    request: ManifestKeyRotationRequest,
) -> Result<ManifestKeyRotationEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    enforce_and_rotate_manifest_key_policy(&mut io, request)
        .map_err(StoragePersistError::ManifestKeyRotationTransaction)
}

/// Commits one already-verified BMA1 authorization into the kernel-only audit
/// ledger. EL0 receives neither the block device nor a writable persistence
/// capability; only the verified sequence and operation mask are published by
/// the later boot-local session syscall.
#[cfg(feature = "unified-product-maintenance-authorization-runtime")]
pub fn commit_verified_maintenance_authorization(
    counter_frequency: u64,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceAuditEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    commit_maintenance_authorization(&mut io, request)
        .map_err(StoragePersistError::MaintenanceAuditTransaction)
}

/// Cross-checks the M77 audit head with M78's durable completion head before
/// admitting a new or narrowly resumed maintenance session.
#[cfg(feature = "unified-product-maintenance-execution-runtime")]
pub fn admit_verified_maintenance_authorization(
    counter_frequency: u64,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceAuditEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    admit_maintenance_authorization_with_execution(&mut io, request)
        .map_err(StoragePersistError::MaintenanceExecutionTransaction)
}

/// Performs M79's read-only three-ledger validation before M78 may mutate the
/// authorization audit.
#[cfg(feature = "unified-product-maintenance-step-runtime")]
pub fn preflight_verified_maintenance_steps(
    counter_frequency: u64,
    request: MaintenanceAuditRequest,
) -> Result<MaintenanceStepAdmissionEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    preflight_maintenance_step_admission(&mut io, request)
        .map_err(StoragePersistError::MaintenanceStepTransaction)
}

/// Performs M80's read-only four-ledger validation before any audit mutation.
#[cfg(feature = "unified-product-maintenance-plan-runtime")]
pub fn preflight_verified_maintenance_plan(
    counter_frequency: u64,
    request: MaintenanceAuditRequest,
) -> Result<MaintenancePlanAdmissionEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    preflight_maintenance_plan_admission(&mut io, request)
        .map_err(StoragePersistError::MaintenancePlanTransaction)
}

/// Performs M81's read-only five-ledger validation before any program or
/// authorization mutation.
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
pub fn preflight_verified_signed_maintenance_plan(
    counter_frequency: u64,
    request: SignedMaintenancePlanRequest,
) -> Result<SignedMaintenancePlanAdmissionEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    preflight_signed_maintenance_plan(&mut io, request)
        .map_err(StoragePersistError::SignedMaintenancePlanTransaction)
}

/// Persists or read-only replays the exact verified M81 BMP1 binding.
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
pub fn commit_verified_signed_maintenance_plan(
    counter_frequency: u64,
    request: SignedMaintenancePlanRequest,
) -> Result<SignedMaintenancePlanEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    commit_signed_maintenance_plan(&mut io, request)
        .map_err(StoragePersistError::SignedMaintenancePlanTransaction)
}

/// Persists one kernel-observed M79 fixed-program step.
#[cfg(feature = "unified-product-maintenance-step-runtime")]
pub fn commit_verified_maintenance_step(
    counter_frequency: u64,
    request: MaintenanceStepCommitRequest,
) -> Result<MaintenanceStepEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    commit_maintenance_step(&mut io, request)
        .map_err(StoragePersistError::MaintenanceStepTransaction)
}

/// Persists or read-only replays one M80 plan phase.
#[cfg(feature = "unified-product-maintenance-plan-runtime")]
pub fn commit_verified_maintenance_plan_transition(
    counter_frequency: u64,
    request: MaintenancePlanTransitionRequest,
) -> Result<MaintenancePlanEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    commit_maintenance_plan_transition(&mut io, request)
        .map_err(StoragePersistError::MaintenancePlanTransaction)
}

/// Binds M79's selected terminal drain step immediately before M78 completion.
#[cfg(feature = "unified-product-maintenance-step-runtime")]
pub fn validate_verified_maintenance_step_terminal(
    counter_frequency: u64,
    request: MaintenanceStepTerminalRequest,
) -> Result<MaintenanceStepTerminalEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    validate_maintenance_step_terminal(&mut io, request)
        .map_err(StoragePersistError::MaintenanceStepTransaction)
}

/// Binds M80's confirmed terminal plan head before aggregate completion.
#[cfg(feature = "unified-product-maintenance-plan-runtime")]
pub fn validate_verified_maintenance_plan_terminal(
    counter_frequency: u64,
    request: MaintenancePlanTerminalRequest,
) -> Result<MaintenancePlanTerminalEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    validate_maintenance_plan_terminal(&mut io, request)
        .map_err(StoragePersistError::MaintenancePlanTransaction)
}

/// Binds M81's selected signed program to the confirmed M80 terminal head.
#[cfg(feature = "unified-product-signed-maintenance-plan-runtime")]
pub fn validate_verified_signed_maintenance_plan_terminal(
    counter_frequency: u64,
    request: SignedMaintenancePlanTerminalRequest,
) -> Result<SignedMaintenancePlanTerminalEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    validate_signed_maintenance_plan_terminal(&mut io, request)
        .map_err(StoragePersistError::SignedMaintenancePlanTransaction)
}

/// Persists M78 completion only after the resident kernel monitor has
/// validated the bounded runtime and clean-shutdown prerequisites.
#[cfg(feature = "unified-product-maintenance-execution-runtime")]
pub fn commit_verified_maintenance_execution(
    counter_frequency: u64,
    request: MaintenanceExecutionCompletionRequest,
) -> Result<MaintenanceExecutionEvidence, StoragePersistError> {
    if request.partition_first_lba != DATA_PARTITION_FIRST_LBA
        || request.partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    if !storage::durability_contract().map_err(StoragePersistError::Storage)? {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }

    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    commit_maintenance_execution_completion(&mut io, request)
        .map_err(StoragePersistError::MaintenanceExecutionTransaction)
}

/// Commits M63's unclosed-boot hint only after the current kernel has proved
/// the live virtio/IRQ/read-completion contract. Persisted input never opens
/// admission, clears recovery, or publishes the broker's boot-local Offline
/// state.
#[cfg(feature = "storage-server-persistent-health-runtime")]
pub fn advance_verified_device_health(
    counter_frequency: u64,
    partition_first_lba: u64,
    partition_sectors: u64,
    format_epoch: FormatEpoch,
    transport_base: usize,
    expected_irq_id: u16,
) -> Result<DeviceHealthAdvanceEvidence, StoragePersistError> {
    if partition_first_lba != DATA_PARTITION_FIRST_LBA
        || partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    let transport_base =
        u64::try_from(transport_base).map_err(|_| StoragePersistError::InvalidTransportAddress)?;
    if transport_base == 0 || transport_base & 0x1ff != 0 {
        return Err(StoragePersistError::InvalidTransportAddress);
    }

    let flush_supported = storage::durability_contract().map_err(StoragePersistError::Storage)?;
    if !flush_supported {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }
    let selected_features =
        storage::selected_block_features().map_err(StoragePersistError::Storage)?;
    let irq = storage::irq_snapshot();
    let stats = storage::stats().map_err(StoragePersistError::Storage)?;
    let status_ok = storage::last_status_ok().map_err(StoragePersistError::Storage)?;
    if storage::irq_id() != Some(expected_irq_id)
        || selected_features != WRITABLE_FEATURES
        || !irq.armed
        || irq.failed
        || irq.rearm_prepared
        || irq.recovery_required
        || irq.entries == 0
        || irq.queue_events == 0
        || irq.config_events != 0
        || irq.completions < 2
        || storage::recovery_required()
        || storage::recovery_admission_closed()
        || stats.requests == 0
        || stats.requests != stats.completions
        || stats.completions != stats.interrupt_completions
        || stats.read_completions < 2
        || stats.bytes_read < (2 * BLOCK_SECTOR_SIZE) as u64
        || stats.timeouts != 0
        || stats.resets != 0
        || !status_ok
    {
        return Err(StoragePersistError::ReprobeNotVerified);
    }

    let current_contract = DeviceHealthContract {
        capacity_sectors: sector_count,
        selected_features,
        transport_base,
        irq_id: expected_irq_id,
        sector_bytes: BLOCK_SECTOR_SIZE as u32,
        queue_size: u32::from(QUEUE_SIZE),
        device_read_only: read_only,
        flush_supported,
    };
    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    let evidence = advance_device_health_after_reprobe(
        &mut io,
        partition_first_lba,
        partition_sectors,
        format_epoch,
        current_contract,
    )
    .map_err(StoragePersistError::DeviceHealthTransaction)?;
    #[cfg(feature = "storage-server-clean-shutdown-runtime")]
    publish_open_session(format_epoch, evidence.persistence.committed_generation)?;
    Ok(evidence)
}

/// Closes the exact boot session published by
/// [`advance_verified_device_health`] while the current block contract is
/// still healthy and idle. This path is kernel-only and consumes its in-memory
/// session capability after one successful durable close.
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
pub fn close_verified_device_health(
    counter_frequency: u64,
    partition_first_lba: u64,
    partition_sectors: u64,
    transport_base: usize,
    expected_irq_id: u16,
) -> Result<DeviceHealthCloseEvidence, StoragePersistError> {
    if partition_first_lba != DATA_PARTITION_FIRST_LBA
        || partition_sectors != DATA_PARTITION_SECTORS
    {
        return Err(StoragePersistError::DataPartitionContract);
    }
    if counter_frequency == 0 {
        return Err(StoragePersistError::InvalidClock);
    }
    let transport_base =
        u64::try_from(transport_base).map_err(|_| StoragePersistError::InvalidTransportAddress)?;
    if transport_base == 0 || transport_base & 0x1ff != 0 {
        return Err(StoragePersistError::InvalidTransportAddress);
    }

    let open_generation = OPEN_SESSION_GENERATION.load(Ordering::Acquire);
    if open_generation == 0 {
        return Err(StoragePersistError::BootSessionUnavailable);
    }
    let mut format_epoch = [0_u8; 16];
    format_epoch[..8]
        .copy_from_slice(&OPEN_SESSION_EPOCH_LOW.load(Ordering::Relaxed).to_le_bytes());
    format_epoch[8..].copy_from_slice(
        &OPEN_SESSION_EPOCH_HIGH
            .load(Ordering::Relaxed)
            .to_le_bytes(),
    );

    let flush_supported = storage::durability_contract().map_err(StoragePersistError::Storage)?;
    if !flush_supported {
        return Err(StoragePersistError::MissingFlush);
    }
    let (sector_count, read_only) =
        storage::device_contract().map_err(StoragePersistError::Storage)?;
    if read_only {
        return Err(StoragePersistError::ReadOnlyDevice);
    }
    let selected_features =
        storage::selected_block_features().map_err(StoragePersistError::Storage)?;
    let irq = storage::irq_snapshot();
    let stats = storage::stats().map_err(StoragePersistError::Storage)?;
    let terminal = storage::terminal_dma_snapshot().map_err(StoragePersistError::Storage)?;
    let recovery = storage::async_recovery_snapshot();
    let status_ok = storage::last_status_ok().map_err(StoragePersistError::Storage)?;
    if storage::irq_id() != Some(expected_irq_id)
        || selected_features != WRITABLE_FEATURES
        || !irq.armed
        || irq.failed
        || irq.rearm_prepared
        || irq.recovery_required
        || irq.config_events != 0
        || storage::recovery_required()
        || storage::recovery_admission_closed()
        || recovery.active
        || terminal.driver_state != 1
        || terminal.in_flight != 0
        || terminal.recovery_active
        || !terminal.recovery_scratch_clear
        || !terminal.requests_terminal
        || stats.requests == 0
        || stats.requests != stats.completions
        || stats.completions != stats.interrupt_completions
        || stats.timeouts != 0
        || stats.resets != 0
        || !status_ok
    {
        return Err(StoragePersistError::CleanShutdownNotQuiescent);
    }

    let current_contract = DeviceHealthContract {
        capacity_sectors: sector_count,
        selected_features,
        transport_base,
        irq_id: expected_irq_id,
        sector_bytes: BLOCK_SECTOR_SIZE as u32,
        queue_size: u32::from(QUEUE_SIZE),
        device_read_only: read_only,
        flush_supported,
    };
    let mut clock = PhysicalCounter;
    let mut io = IrqDurableIo {
        clock: &mut clock,
        deadline_budget: counter_frequency,
        sector_count,
    };
    let evidence = close_device_health_for_clean_shutdown(
        &mut io,
        partition_first_lba,
        partition_sectors,
        format_epoch,
        open_generation,
        current_contract,
    )
    .map_err(StoragePersistError::DeviceHealthCloseTransaction)?;
    OPEN_SESSION_GENERATION.store(0, Ordering::Release);
    Ok(evidence)
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
fn publish_open_session(
    format_epoch: FormatEpoch,
    generation: u64,
) -> Result<(), StoragePersistError> {
    if generation == 0 || OPEN_SESSION_GENERATION.load(Ordering::Acquire) != 0 {
        return Err(StoragePersistError::BootSessionAlreadyPublished);
    }
    OPEN_SESSION_EPOCH_LOW.store(
        u64::from_le_bytes(format_epoch[..8].try_into().unwrap()),
        Ordering::Relaxed,
    );
    OPEN_SESSION_EPOCH_HIGH.store(
        u64::from_le_bytes(format_epoch[8..].try_into().unwrap()),
        Ordering::Relaxed,
    );
    OPEN_SESSION_GENERATION
        .compare_exchange(0, generation, Ordering::Release, Ordering::Acquire)
        .map_err(|_| StoragePersistError::BootSessionAlreadyPublished)?;
    Ok(())
}

const fn map_read_error(error: StorageError) -> PersistIoError {
    match error {
        StorageError::WriteOutsideDataPartition
        | StorageError::Block(BlockError::SectorOutOfBounds) => PersistIoError::OutOfBounds,
        StorageError::IrqNotArmed
        | StorageError::InterruptFailure
        | StorageError::RecoveryRequired
        | StorageError::Block(BlockError::DeviceNeedsReset)
        | StorageError::Block(BlockError::DeadlineExpired) => PersistIoError::RequiresReset,
        _ => PersistIoError::Device,
    }
}

const fn map_mutating_error(error: StorageError) -> PersistIoError {
    match error {
        StorageError::WriteOutsideDataPartition
        | StorageError::Block(BlockError::SectorOutOfBounds) => PersistIoError::OutOfBounds,
        StorageError::IrqNotArmed
        | StorageError::InterruptFailure
        | StorageError::RecoveryRequired
        | StorageError::Block(BlockError::DeviceNeedsReset) => PersistIoError::RequiresReset,
        StorageError::SubmittedMutationOutcomeUnknown
        | StorageError::Block(BlockError::DeadlineExpired) => PersistIoError::OutcomeUnknown,
        _ => PersistIoError::Device,
    }
}

const _: () = {
    assert!(matches!(
        map_mutating_error(StorageError::SubmittedMutationOutcomeUnknown),
        PersistIoError::OutcomeUnknown
    ));
    assert!(matches!(
        map_mutating_error(StorageError::InterruptFailure),
        PersistIoError::RequiresReset
    ));
    assert!(matches!(
        map_read_error(StorageError::SubmittedMutationOutcomeUnknown),
        PersistIoError::Device
    ));
};
