//! Permanent ownership for the boot-discovered block device.
//!
//! M22 keeps the live driver and both DMA frames here while exposing only
//! IRQ-masked, bounded operations. No `&mut VirtioBlock` or caller buffer is
//! retained across an IRQ-enabled wait: foreground submit/take and hard-IRQ
//! completion therefore serialize on the single CPU without mutable aliasing.

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU64, Ordering};

use bndroid_kernel::time::deadline_reached;
#[cfg(any(
    feature = "storage-server-recovery-runtime",
    feature = "app-data-async-recovery-runtime"
))]
use bndroid_kernel::virtio::BlockRequestKind;
use bndroid_kernel::virtio::{
    BLOCK_SECTOR_SIZE, BLOCK_STATUS_OK, block_interrupt_requires_recovery,
    request_pair_all_terminal,
};
#[cfg(not(bndroid_storage_irq_timeout_profile))]
use bndroid_kernel::{
    block::{BlockReadError, BlockReader, Sector},
    virtio::BLOCK_SECTOR_SIZE as PARSER_SECTOR_SIZE,
};

use crate::arch::aarch64;
#[cfg(feature = "cooperative-block-recovery")]
use crate::driver::virtio::block::CooperativeRecoveryProgress;
#[cfg(not(bndroid_storage_irq_timeout_profile))]
use crate::driver::virtio::block::deadline_after;
use crate::driver::virtio::block::{
    BlockError, BlockStats, InterruptService, IrqRearmObservation, PollClock, ReadTakeOutcome,
    ReadToken, VirtioBlock,
};
#[cfg(feature = "storage-server-fault-policy-runtime")]
pub use crate::driver::virtio::block::{PersistentRebuildFaultSnapshot, TerminalDmaSnapshot};
#[cfg(feature = "cooperative-block-recovery")]
use bndroid_kernel::virtio::BlockRecoveryPhase;

const EMPTY: u8 = 0;
const INSTALLING: u8 = 1;
const READY: u8 = 2;

#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub const DATA_PARTITION_FIRST_LBA: u64 = 64;
#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub const DATA_PARTITION_SECTORS: u64 = 64;
#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub const DATA_PARTITION_END_LBA: u64 = DATA_PARTITION_FIRST_LBA + DATA_PARTITION_SECTORS;
/// Raw GPT `unique_guid` bytes for the fixed `BNDROID_DATA` format epoch.
///
/// GPT stores the first three UUID fields little-endian, so this is
/// `d25cf134-a879-4e5a-9364-67b2d8902501` in its on-disk representation.
#[cfg(feature = "unified-product-persistent-rollback-runtime")]
pub const DATA_PARTITION_FORMAT_EPOCH: [u8; 16] = [
    0x34, 0xf1, 0x5c, 0xd2, 0x79, 0xa8, 0x5a, 0x4e, 0x93, 0x64, 0x67, 0xb2, 0xd8, 0x90, 0x25, 0x01,
];
#[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
pub const APPDATA_PARTITION_FIRST_LBA: u64 = 128;
#[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
pub const APPDATA_PARTITION_SECTORS: u64 = 1_920;
#[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
pub const APPDATA_PARTITION_END_LBA: u64 = APPDATA_PARTITION_FIRST_LBA + APPDATA_PARTITION_SECTORS;
#[cfg(feature = "androidbox-apk-install0")]
pub const PACKAGES_PARTITION_FIRST_LBA: u64 = 16_384;
#[cfg(feature = "androidbox-multipackage4")]
pub const PACKAGES_PARTITION_SECTORS: u64 = 1_024;
#[cfg(all(
    feature = "androidbox-apk-install0",
    not(feature = "androidbox-multipackage4")
))]
pub const PACKAGES_PARTITION_SECTORS: u64 = 512;
#[cfg(feature = "androidbox-apk-install0")]
pub const PACKAGES_PARTITION_END_LBA: u64 =
    PACKAGES_PARTITION_FIRST_LBA + PACKAGES_PARTITION_SECTORS;

struct PermanentBlockDevice {
    state: AtomicU8,
    irq_id: AtomicU16,
    value: UnsafeCell<MaybeUninit<VirtioBlock>>,
}

impl PermanentBlockDevice {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            irq_id: AtomicU16::new(0),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    fn install(&self, device: VirtioBlock, irq_id: u16) -> Result<(), InstallError> {
        if !(32..1020).contains(&irq_id) {
            return Err(InstallError::InvalidIrq);
        }
        self.state
            .compare_exchange(EMPTY, INSTALLING, Ordering::Acquire, Ordering::Acquire)
            .map_err(|_| InstallError::AlreadyInstalled)?;

        // SAFETY: the successful state transition grants this call the only
        // write to the slot. The value remains initialized for the lifetime of
        // the kernel and is published only by the following Release store.
        unsafe {
            (*self.value.get()).write(device);
        }
        self.irq_id.store(irq_id, Ordering::Relaxed);
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.state.load(Ordering::Acquire) == READY
    }

    fn irq_id(&self) -> Option<u16> {
        self.is_ready().then(|| self.irq_id.load(Ordering::Relaxed))
    }

    fn with_device_masked<T>(
        &self,
        operation: impl FnOnce(&mut VirtioBlock) -> T,
    ) -> Result<T, StorageError> {
        if !aarch64::irq_is_masked() {
            return Err(StorageError::IrqMustBeMasked);
        }
        if !self.is_ready() {
            return Err(StorageError::NotReady);
        }
        // SAFETY: install publishes exactly one initialized value. Every
        // mutable access in this module requires local IRQ masking; hard IRQs
        // enter with IRQ masked, and Bndroid is still single-core.
        let device = unsafe { &mut *(*self.value.get()).as_mut_ptr() };
        Ok(operation(device))
    }
}

// SAFETY: initialization is single-writer through the atomic state machine;
// every post-publication access is serialized by the single CPU's local IRQ
// mask, and no reference escapes `with_device_masked`.
unsafe impl Sync for PermanentBlockDevice {}

static BLOCK_DEVICE: PermanentBlockDevice = PermanentBlockDevice::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallError {
    AlreadyInstalled,
    InvalidIrq,
}

impl InstallError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyInstalled => "permanent block device is already installed",
            Self::InvalidIrq => "permanent block device IRQ is not a valid SPI",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    NotReady,
    IrqMustBeMasked,
    IrqNotArmed,
    IrqMustBeDisarmed,
    IrqMismatch,
    InterruptFailure,
    RecoveryRequired,
    /// A write or flush descriptor was published before the failure. The
    /// device must be reset, and callers must recover durable state instead of
    /// assuming that the mutation did or did not take effect.
    #[cfg_attr(bndroid_storage_irq_timeout_profile, allow(dead_code))]
    SubmittedMutationOutcomeUnknown,
    #[cfg(not(bndroid_storage_irq_timeout_profile))]
    WriteOutsideDataPartition,
    #[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
    WriteOutsideAppDataPartition,
    #[cfg(feature = "androidbox-apk-install0")]
    WriteOutsidePackagesPartition,
    Block(BlockError),
}

impl StorageError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotReady => "permanent block device is not ready",
            Self::IrqMustBeMasked => "block driver access requires local IRQ masking",
            Self::IrqNotArmed => "block device IRQ is not armed",
            Self::IrqMustBeDisarmed => "block device IRQ must be disarmed before reset",
            Self::IrqMismatch => "block device IRQ registration does not match the FDT",
            Self::InterruptFailure => "block device hard-IRQ completion failed",
            Self::RecoveryRequired => "block device requires reset before another request",
            Self::SubmittedMutationOutcomeUnknown => {
                "submitted block mutation outcome is unknown and requires reset"
            }
            #[cfg(not(bndroid_storage_irq_timeout_profile))]
            Self::WriteOutsideDataPartition => {
                "block write is outside the dedicated persistent-data partition"
            }
            #[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
            Self::WriteOutsideAppDataPartition => {
                "block write is outside the dedicated application-data partition"
            }
            #[cfg(feature = "androidbox-apk-install0")]
            Self::WriteOutsidePackagesPartition => {
                "block write is outside the dedicated installed-package partition"
            }
            Self::Block(error) => error.as_str(),
        }
    }
}

impl From<BlockError> for StorageError {
    fn from(error: BlockError) -> Self {
        Self::Block(error)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IrqSnapshot {
    pub entries: u64,
    pub queue_events: u64,
    pub config_events: u64,
    pub spurious: u64,
    pub completions: u64,
    pub generation: u64,
    pub failed: bool,
    pub armed: bool,
    pub rearm_prepared: bool,
    pub recovery_required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqReadEvidence {
    pub tokens: [ReadToken; 2],
    pub pending_at_submit: u16,
    pub generation_before: u64,
    pub generation_after: u64,
    pub deadline_race_completions: u16,
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CleanShutdownGateSnapshot {
    pub active: bool,
    pub admission_closed: bool,
    pub recovery_required: bool,
    pub irq_armed: bool,
    pub irq_failed: bool,
    pub in_flight: u16,
    pub requests_terminal: bool,
}

#[cfg(feature = "cooperative-block-recovery")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AsyncRecoverySnapshot {
    pub active: bool,
    pub phase: u8,
    pub steps: u64,
    pub pending_returns: u64,
    pub max_masked_counter_ticks: u64,
    pub masked_poll_iterations: u64,
}

static IRQ_ARMED: AtomicBool = AtomicBool::new(false);
static IRQ_REARM_PREPARED: AtomicBool = AtomicBool::new(false);
static IRQ_FAILED: AtomicBool = AtomicBool::new(false);
static RECOVERY_REQUIRED: AtomicBool = AtomicBool::new(false);
static RECOVERY_ADMISSION_CLOSED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
static CLEAN_SHUTDOWN_ACTIVE: AtomicBool = AtomicBool::new(false);
static IRQ_ENTRIES: AtomicU64 = AtomicU64::new(0);
static IRQ_QUEUE_EVENTS: AtomicU64 = AtomicU64::new(0);
static IRQ_CONFIG_EVENTS: AtomicU64 = AtomicU64::new(0);
static IRQ_SPURIOUS: AtomicU64 = AtomicU64::new(0);
static IRQ_COMPLETIONS: AtomicU64 = AtomicU64::new(0);
static COMPLETION_GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "cooperative-block-recovery")]
static ASYNC_RECOVERY_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "cooperative-block-recovery")]
static ASYNC_RECOVERY_PHASE: AtomicU8 = AtomicU8::new(0);
#[cfg(feature = "cooperative-block-recovery")]
static ASYNC_RECOVERY_STEPS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "cooperative-block-recovery")]
static ASYNC_RECOVERY_PENDING_RETURNS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "cooperative-block-recovery")]
static ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS: AtomicU64 = AtomicU64::new(0);

pub fn install(device: VirtioBlock, irq_id: u16) -> Result<(), InstallError> {
    BLOCK_DEVICE.install(device, irq_id)
}

pub fn is_ready() -> bool {
    BLOCK_DEVICE.is_ready()
}

pub fn irq_id() -> Option<u16> {
    BLOCK_DEVICE.irq_id()
}

#[cfg(any(
    feature = "app-data-runtime",
    feature = "storage-server-runtime",
    feature = "cooperative-block-recovery"
))]
pub fn recovery_required() -> bool {
    RECOVERY_REQUIRED.load(Ordering::Acquire)
}

#[cfg(any(
    feature = "app-data-runtime",
    feature = "storage-server-runtime",
    feature = "cooperative-block-recovery"
))]
pub fn recovery_admission_closed() -> bool {
    RECOVERY_ADMISSION_CLOSED.load(Ordering::Acquire) || RECOVERY_REQUIRED.load(Ordering::Acquire)
}

/// Closes the submission barrier before an ownerless broker recovery. This is
/// kernel-internal policy state; EL0 never receives authority to set or clear
/// the physical-device recovery latch.
#[cfg(any(
    feature = "app-data-runtime",
    feature = "storage-server-runtime",
    feature = "cooperative-block-recovery"
))]
pub(crate) fn require_recovery() {
    RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);
    RECOVERY_REQUIRED.store(true, Ordering::Release);
}

/// Seals the block-submission gate after M64 has durably closed the current
/// boot session. The caller keeps local IRQ masked through physical route
/// disable, so no request can enter between the idle proof and the gate.
/// Unlike failure recovery, this does not assert `RECOVERY_REQUIRED`.
#[cfg(feature = "storage-server-clean-shutdown-runtime")]
pub(crate) fn seal_clean_shutdown_admission_masked(
    irq_id: u16,
) -> Result<CleanShutdownGateSnapshot, StorageError> {
    if !aarch64::irq_is_masked() {
        return Err(StorageError::IrqMustBeMasked);
    }
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    let async_recovery_active = ASYNC_RECOVERY_ACTIVE.load(Ordering::Acquire);
    let terminal = BLOCK_DEVICE.with_device_masked(|device| device.terminal_dma_snapshot())?;
    if !IRQ_ARMED.load(Ordering::Acquire)
        || IRQ_REARM_PREPARED.load(Ordering::Acquire)
        || IRQ_FAILED.load(Ordering::Acquire)
        || RECOVERY_REQUIRED.load(Ordering::Acquire)
        || RECOVERY_ADMISSION_CLOSED.load(Ordering::Acquire)
        || async_recovery_active
        || terminal.driver_state != 1
        || terminal.in_flight != 0
        || terminal.recovery_active
        || !terminal.recovery_scratch_clear
        || !terminal.requests_terminal
    {
        return Err(StorageError::RecoveryRequired);
    }
    CLEAN_SHUTDOWN_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| StorageError::RecoveryRequired)?;
    RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);
    Ok(clean_shutdown_gate_snapshot())
}

#[cfg(feature = "storage-server-clean-shutdown-runtime")]
pub fn clean_shutdown_gate_snapshot() -> CleanShutdownGateSnapshot {
    let terminal = terminal_dma_snapshot().unwrap_or_default();
    CleanShutdownGateSnapshot {
        active: CLEAN_SHUTDOWN_ACTIVE.load(Ordering::Acquire),
        admission_closed: RECOVERY_ADMISSION_CLOSED.load(Ordering::Acquire),
        recovery_required: RECOVERY_REQUIRED.load(Ordering::Acquire),
        irq_armed: IRQ_ARMED.load(Ordering::Acquire),
        irq_failed: IRQ_FAILED.load(Ordering::Acquire),
        in_flight: terminal.in_flight,
        requests_terminal: terminal.requests_terminal,
    }
}

#[cfg(any(
    feature = "cooperative-block-recovery",
    feature = "storage-server-runtime"
))]
pub(crate) fn open_recovery_admission(irq_id: u16) -> Result<(), StorageError> {
    if !aarch64::irq_is_masked() {
        return Err(StorageError::IrqMustBeMasked);
    }
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    #[cfg(feature = "cooperative-block-recovery")]
    let async_recovery_active = ASYNC_RECOVERY_ACTIVE.load(Ordering::Acquire);
    #[cfg(not(feature = "cooperative-block-recovery"))]
    let async_recovery_active = false;
    if RECOVERY_REQUIRED.load(Ordering::Acquire)
        || IRQ_FAILED.load(Ordering::Acquire)
        || !IRQ_ARMED.load(Ordering::Acquire)
        || IRQ_REARM_PREPARED.load(Ordering::Acquire)
        || async_recovery_active
    {
        return Err(StorageError::RecoveryRequired);
    }
    RECOVERY_ADMISSION_CLOSED.store(false, Ordering::Release);
    Ok(())
}

pub fn arm_irq(irq_id: u16) -> Result<(), StorageError> {
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    // This one-step entry point is only for the initial boot registration.
    // Once recovery is latched, only prepare/commit may reopen the logical
    // IRQ gate after the GIC line has actually been enabled.
    if IRQ_REARM_PREPARED.load(Ordering::Acquire)
        || IRQ_FAILED.load(Ordering::Acquire)
        || RECOVERY_REQUIRED.load(Ordering::Acquire)
    {
        return Err(StorageError::RecoveryRequired);
    }
    IRQ_ARMED.store(true, Ordering::Release);
    Ok(())
}

#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
pub fn disarm_irq(irq_id: u16) -> Result<(), StorageError> {
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    IRQ_ARMED.store(false, Ordering::Release);
    IRQ_REARM_PREPARED.store(false, Ordering::Release);
    Ok(())
}

/// Begins logical IRQ rearming while the caller keeps local IRQ masked and
/// the GIC line disabled. Recovery/failure latches deliberately remain set
/// until [`commit_irq_rearm`] proves that GIC enable succeeded.
pub(crate) fn prepare_irq_rearm(irq_id: u16) -> Result<(), StorageError> {
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    if IRQ_ARMED.load(Ordering::Acquire)
        || IRQ_REARM_PREPARED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return Err(StorageError::IrqMustBeDisarmed);
    }
    Ok(())
}

/// Audits the rebuilt virtio source after the GIC route is enabled and before
/// the recovery latch is cleared. The caller keeps DAIF masked and must roll
/// back the physical route if this method fails.
pub(crate) fn validate_irq_rearm_tail(irq_id: u16) -> Result<(), StorageError> {
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    if IRQ_ARMED.load(Ordering::Acquire)
        || !IRQ_REARM_PREPARED.load(Ordering::Acquire)
        || !RECOVERY_REQUIRED.load(Ordering::Acquire)
    {
        return Err(StorageError::RecoveryRequired);
    }

    let IrqRearmObservation { bits, validation } =
        BLOCK_DEVICE.with_device_masked(VirtioBlock::observe_irq_rearm_tail)?;
    record_interrupt_service(
        InterruptService {
            bits,
            completions: 0,
        },
        false,
    );
    if block_interrupt_requires_recovery(bits) {
        return Err(BlockError::DeviceNeedsReset.into());
    }
    if let Err(error) = validation {
        record_interrupt_failure();
        return Err(error.into());
    }
    Ok(())
}

/// Publishes the fully rearmed state. `interrupt::reenable_block_irq` calls
/// this only after the pending-preserving GIC enable and post-enable source
/// audit both succeed, before restoring DAIF. Observing a cleared recovery
/// latch therefore proves a live route with no discarded new-epoch edge; the
/// independent submission barrier remains closed until the owning policy
/// coordinator explicitly opens it in the same masked commit window.
pub(crate) fn commit_irq_rearm(irq_id: u16) -> Result<(), StorageError> {
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    if IRQ_ARMED.load(Ordering::Acquire) || !IRQ_REARM_PREPARED.swap(false, Ordering::AcqRel) {
        return Err(StorageError::RecoveryRequired);
    }
    IRQ_ARMED.store(true, Ordering::Relaxed);
    IRQ_FAILED.store(false, Ordering::Relaxed);
    // Release is the commit point for the preceding armed/failure stores.
    RECOVERY_REQUIRED.store(false, Ordering::Release);
    Ok(())
}

/// Cancels either a prepared or partially committed rearm and relatches both
/// fail-closed indicators. This operation is intentionally idempotent so the
/// interrupt layer can use it for every error path after disabling the GIC.
pub(crate) fn rollback_irq_rearm(irq_id: u16) -> Result<(), StorageError> {
    IRQ_ARMED.store(false, Ordering::Release);
    IRQ_REARM_PREPARED.store(false, Ordering::Release);
    IRQ_FAILED.store(true, Ordering::Release);
    RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);
    RECOVERY_REQUIRED.store(true, Ordering::Release);
    if BLOCK_DEVICE.irq_id() != Some(irq_id) {
        return Err(StorageError::IrqMismatch);
    }
    Ok(())
}

pub fn handles_irq(irq_id: u16) -> bool {
    IRQ_ARMED.load(Ordering::Acquire) && BLOCK_DEVICE.irq_id() == Some(irq_id)
}

/// Services the registered block IRQ. The exception vector already masks IRQ,
/// and the dispatcher must call this before writing GICC_EOIR.
pub fn handle_irq(irq_id: u16) -> bool {
    if !handles_irq(irq_id) {
        return false;
    }
    IRQ_ENTRIES.fetch_add(1, Ordering::Relaxed);
    let service = BLOCK_DEVICE.with_device_masked(VirtioBlock::service_interrupt);
    match service {
        Ok(Ok(service)) => record_interrupt_service(service, true),
        Ok(Err(_)) | Err(_) => {
            record_interrupt_failure();
        }
    }
    true
}

fn record_interrupt_service(
    InterruptService { bits, completions }: InterruptService,
    count_spurious: bool,
) {
    if count_spurious && bits == 0 {
        IRQ_SPURIOUS.fetch_add(1, Ordering::Relaxed);
    }
    if bits & 1 != 0 {
        IRQ_QUEUE_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
    if bits & 2 != 0 {
        IRQ_CONFIG_EVENTS.fetch_add(1, Ordering::Relaxed);
        // This bounded driver cannot revalidate capacity or features in
        // place. The same latch is used by hard IRQ and deadline drains so a
        // configuration change cannot disappear merely because foreground
        // code won the interrupt race.
        IRQ_FAILED.store(true, Ordering::Release);
        RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);
        RECOVERY_REQUIRED.store(true, Ordering::Release);
    }
    if completions != 0 {
        IRQ_COMPLETIONS.fetch_add(u64::from(completions), Ordering::Relaxed);
    }
    if completions != 0 || bits & 2 != 0 {
        COMPLETION_GENERATION.fetch_add(1, Ordering::Release);
    }
}

fn record_interrupt_failure() {
    IRQ_FAILED.store(true, Ordering::Release);
    RECOVERY_ADMISSION_CLOSED.store(true, Ordering::Release);
    RECOVERY_REQUIRED.store(true, Ordering::Release);
    COMPLETION_GENERATION.fetch_add(1, Ordering::Release);
}

pub fn irq_snapshot() -> IrqSnapshot {
    IrqSnapshot {
        entries: IRQ_ENTRIES.load(Ordering::Relaxed),
        queue_events: IRQ_QUEUE_EVENTS.load(Ordering::Relaxed),
        config_events: IRQ_CONFIG_EVENTS.load(Ordering::Relaxed),
        spurious: IRQ_SPURIOUS.load(Ordering::Relaxed),
        completions: IRQ_COMPLETIONS.load(Ordering::Relaxed),
        generation: COMPLETION_GENERATION.load(Ordering::Acquire),
        failed: IRQ_FAILED.load(Ordering::Acquire),
        armed: IRQ_ARMED.load(Ordering::Acquire),
        rearm_prepared: IRQ_REARM_PREPARED.load(Ordering::Acquire),
        recovery_required: RECOVERY_REQUIRED.load(Ordering::Acquire),
    }
}

pub fn device_contract() -> Result<(u64, bool), StorageError> {
    with_foreground_device(|device| (device.capacity_sectors(), device.device_read_only()))
}

#[cfg(feature = "storage-server-persistent-health-runtime")]
pub fn selected_block_features() -> Result<u64, StorageError> {
    with_foreground_device(|device| device.negotiated_features().selected())
}

#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub fn durability_contract() -> Result<bool, StorageError> {
    with_foreground_device(|device| device.flush_supported())
}

pub fn stats() -> Result<BlockStats, StorageError> {
    with_foreground_device(|device| device.stats())
}

/// Arms the next queue notification loss for an isolated recovery QEMU proof.
/// This symbol and the driver hook are absent from every production profile.
#[cfg(any(
    feature = "storage-server-recovery-runtime",
    feature = "app-data-async-recovery-runtime"
))]
pub fn suppress_next_notification_for_recovery_test(
    expected: BlockRequestKind,
) -> Result<(), StorageError> {
    ensure_io_ready()?;
    with_foreground_device(|device| {
        device.suppress_next_notification_for_recovery_test(expected)
    })??;
    Ok(())
}

#[cfg(any(
    feature = "storage-server-recovery-runtime",
    feature = "app-data-async-recovery-runtime"
))]
pub fn recovery_notification_fault_armed() -> Result<bool, StorageError> {
    with_foreground_device(|device| device.recovery_notification_fault_armed())
}

/// Atomically arms M60's ordinary-read notification loss and persistent
/// queue-rebuild failure while the device is healthy and idle. The sole caller
/// is the kernel recovery coordinator before it publishes the ownerless
/// seventh campaign to the broker.
#[cfg(feature = "storage-server-fault-policy-runtime")]
pub(crate) fn arm_m60_permanent_read_fault_for_test() -> Result<(), StorageError> {
    ensure_io_ready()?;
    with_foreground_device(|device| device.arm_m60_permanent_read_fault_for_test())??;
    Ok(())
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
pub fn persistent_rebuild_fault_snapshot() -> Result<PersistentRebuildFaultSnapshot, StorageError> {
    with_foreground_device(|device| device.persistent_rebuild_fault_snapshot())
}

/// Captures the final request/DMA ownership state before the broker is made
/// boot-locally Offline. This does not by itself disable the IRQ route; the
/// terminal coordinator must prove that independently in the same transition.
#[cfg(feature = "storage-server-fault-policy-runtime")]
pub fn terminal_dma_snapshot() -> Result<TerminalDmaSnapshot, StorageError> {
    with_foreground_device(|device| device.terminal_dma_snapshot())
}

pub fn verify_out_of_range_rejected(sector: u64) -> Result<(), StorageError> {
    ensure_io_ready()?;
    match with_foreground_device(|device| device.submit_read(sector))? {
        Err(BlockError::SectorOutOfBounds) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(BlockError::ProtocolViolation.into()),
    }
}

pub fn read_pair_irq<C: PollClock + ?Sized>(
    sectors: [u64; 2],
    outputs: &mut [[u8; BLOCK_SECTOR_SIZE]; 2],
    clock: &mut C,
    deadline: u64,
) -> Result<IrqReadEvidence, StorageError> {
    if aarch64::irq_is_masked() {
        return Err(StorageError::IrqNotArmed);
    }
    if !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let (tokens, pending_at_submit) = with_foreground_device(|device| {
        let tokens = device.submit_read_pair(sectors)?;
        Ok::<_, BlockError>((tokens, device.in_flight()))
    })??;
    if pending_at_submit != 2 || tokens[0].slot() == tokens[1].slot() {
        return Err(BlockError::ProtocolViolation.into());
    }

    let mut terminal = [false; 2];
    let mut first_error = None;
    let mut deadline_race_completions = 0_u16;
    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(StorageError::InterruptFailure);
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before {
            take_available(tokens, outputs, &mut terminal, &mut first_error)?;
            if request_pair_all_terminal(terminal) {
                if let Some(error) = first_error {
                    return Err(error.into());
                }
                return Ok(IrqReadEvidence {
                    tokens,
                    pending_at_submit,
                    generation_before,
                    generation_after: COMPLETION_GENERATION.load(Ordering::Acquire),
                    deadline_race_completions,
                });
            }
        }
        if deadline_reached(clock.now(), deadline) {
            let resolution = with_foreground_device(|device| {
                let drained = resolve_deadline_race(device)?;
                drain_read_pair_masked(device, tokens, outputs, &mut terminal, &mut first_error)?;
                let all_terminal = request_pair_all_terminal(terminal);
                if !all_terminal {
                    device.record_timeout();
                    RECOVERY_REQUIRED.store(true, Ordering::Release);
                }
                Ok::<_, BlockError>((drained, all_terminal))
            });
            let (drained, all_terminal) = match resolution {
                Ok(Ok(result)) => result,
                Ok(Err(BlockError::DeviceNeedsReset)) => {
                    return Err(BlockError::DeviceNeedsReset.into());
                }
                Ok(Err(_)) | Err(_) => return Err(read_pair_recovery_required()),
            };
            deadline_race_completions = drained;
            if all_terminal {
                if let Some(error) = first_error {
                    return Err(error.into());
                }
                return Ok(IrqReadEvidence {
                    tokens,
                    pending_at_submit,
                    generation_before,
                    generation_after: COMPLETION_GENERATION.load(Ordering::Acquire),
                    deadline_race_completions,
                });
            }
            return Err(BlockError::DeadlineExpired.into());
        }
        spin_loop();
    }
}

/// Reads one sector through the same IRQ-only completion path used by the
/// two-request boot proof. This is the synchronous boundary consumed by the
/// allocation-free partition and filesystem parsers; it does not expose the
/// driver's DMA buffer.
#[cfg(any(
    not(bndroid_storage_irq_timeout_profile),
    feature = "app-data-runtime",
    feature = "storage-server-runtime"
))]
pub fn read_sector_irq<C: PollClock + ?Sized>(
    sector: u64,
    output: &mut [u8; BLOCK_SECTOR_SIZE],
    clock: &mut C,
    deadline: u64,
) -> Result<(), StorageError> {
    if aarch64::irq_is_masked() || !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let token = with_foreground_device(|device| device.submit_read(sector))??;

    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(StorageError::InterruptFailure);
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before {
            match with_foreground_device(|device| device.take_read(token, output))? {
                Ok(()) => return Ok(()),
                Err(BlockError::RequestPending) => {}
                Err(error) => return Err(error.into()),
            }
        }
        if deadline_reached(clock.now(), deadline) {
            let completed = with_foreground_device(|device| {
                resolve_deadline_race(device)?;
                match device.take_read(token, output) {
                    Ok(()) => Ok(true),
                    Err(BlockError::RequestPending) => {
                        device.record_timeout();
                        RECOVERY_REQUIRED.store(true, Ordering::Release);
                        Ok(false)
                    }
                    Err(error) => Err(error),
                }
            })??;
            if completed {
                return Ok(());
            }
            return Err(BlockError::DeadlineExpired.into());
        }
        spin_loop();
    }
}

/// Writes exactly one sector inside the fixed `BNDROID_DATA` GPT window and
/// waits for its hard-IRQ completion. The write is not durable until a later
/// successful [`flush_irq`] call; callers must treat a timeout as an unknown
/// outcome and must not blindly retry it.
#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub fn write_data_sector_irq<C: PollClock + ?Sized>(
    sector: u64,
    input: &[u8; BLOCK_SECTOR_SIZE],
    clock: &mut C,
    deadline: u64,
) -> Result<(), StorageError> {
    if !(DATA_PARTITION_FIRST_LBA..DATA_PARTITION_END_LBA).contains(&sector) {
        return Err(StorageError::WriteOutsideDataPartition);
    }
    if aarch64::irq_is_masked() || !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let token = with_foreground_device(|device| device.submit_write(sector, input))??;

    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(submitted_mutation_failed());
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before
            && take_submitted_mutation(|device| device.take_write(token))?
        {
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return resolve_submitted_mutation_timeout(|device| device.take_write(token));
        }
        spin_loop();
    }
}

/// Writes exactly one sector inside the fixed M54 `BNDROID_APPDATA` window.
///
/// This deliberately remains a separate policy entry point from
/// [`write_data_sector_irq`]: neither caller can select the other's partition,
/// and a rejected request reaches no virtio descriptor or accounting ledger.
/// A successful write is durable only after a later successful [`flush_irq`].
#[cfg(any(feature = "app-data-runtime", feature = "storage-server-runtime"))]
pub fn write_appdata_sector_irq<C: PollClock + ?Sized>(
    sector: u64,
    input: &[u8; BLOCK_SECTOR_SIZE],
    clock: &mut C,
    deadline: u64,
) -> Result<(), StorageError> {
    if !(APPDATA_PARTITION_FIRST_LBA..APPDATA_PARTITION_END_LBA).contains(&sector) {
        return Err(StorageError::WriteOutsideAppDataPartition);
    }
    if aarch64::irq_is_masked() || !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let token = with_foreground_device(|device| device.submit_write(sector, input))??;

    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(submitted_mutation_failed());
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before
            && take_submitted_mutation(|device| device.take_write(token))?
        {
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return resolve_submitted_mutation_timeout(|device| device.take_write(token));
        }
        spin_loop();
    }
}

/// Writes exactly one sector inside the fixed Install-0
/// `BNDROID_PACKAGES` window.
///
/// Package mutation has a distinct policy entry point so neither the package
/// host nor an AppData caller can select the other's partition. Durability and
/// unknown-outcome handling remain the caller's responsibility through
/// [`flush_irq`] and package-store recovery.
#[cfg(feature = "androidbox-apk-install0")]
pub fn write_packages_sector_irq<C: PollClock + ?Sized>(
    sector: u64,
    input: &[u8; BLOCK_SECTOR_SIZE],
    clock: &mut C,
    deadline: u64,
) -> Result<(), StorageError> {
    if !(PACKAGES_PARTITION_FIRST_LBA..PACKAGES_PARTITION_END_LBA).contains(&sector) {
        return Err(StorageError::WriteOutsidePackagesPartition);
    }
    if aarch64::irq_is_masked() || !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let token = with_foreground_device(|device| device.submit_write(sector, input))??;

    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(submitted_mutation_failed());
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before
            && take_submitted_mutation(|device| device.take_write(token))?
        {
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return resolve_submitted_mutation_timeout(|device| device.take_write(token));
        }
        spin_loop();
    }
}

/// Waits for one explicit virtio cache-flush request through the IRQ-only
/// path. The driver refuses submission while any prior request remains
/// untaken, establishing the required WRITE completion -> FLUSH ordering.
#[cfg(any(
    not(bndroid_storage_irq_timeout_profile),
    feature = "app-data-runtime",
    feature = "storage-server-runtime",
    feature = "androidbox-apk-install0"
))]
pub fn flush_irq<C: PollClock + ?Sized>(clock: &mut C, deadline: u64) -> Result<(), StorageError> {
    if aarch64::irq_is_masked() || !IRQ_ARMED.load(Ordering::Acquire) {
        return Err(StorageError::IrqNotArmed);
    }
    ensure_io_ready()?;
    let generation_before = COMPLETION_GENERATION.load(Ordering::Acquire);
    let token = with_foreground_device(VirtioBlock::submit_flush)??;

    loop {
        if IRQ_FAILED.load(Ordering::Acquire) {
            return Err(submitted_mutation_failed());
        }
        if COMPLETION_GENERATION.load(Ordering::Acquire) != generation_before
            && take_submitted_mutation(|device| device.take_flush(token))?
        {
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return resolve_submitted_mutation_timeout(|device| device.take_flush(token));
        }
        spin_loop();
    }
}

/// Sector-reader facade used by M23 disk parsers. Every call gets a fresh
/// physical-counter deadline, so a hostile on-disk structure cannot turn one
/// expired budget into unbounded subsequent I/O.
#[cfg(not(bndroid_storage_irq_timeout_profile))]
pub struct IrqBlockReader<'a, C: PollClock + ?Sized> {
    clock: &'a mut C,
    deadline_budget: u64,
    sector_count: u64,
    reads: u64,
}

#[cfg(not(bndroid_storage_irq_timeout_profile))]
impl<'a, C: PollClock + ?Sized> IrqBlockReader<'a, C> {
    pub fn new(clock: &'a mut C, deadline_budget: u64) -> Result<Self, StorageError> {
        if deadline_budget == 0 {
            return Err(BlockError::ProtocolViolation.into());
        }
        let (sector_count, _) = device_contract()?;
        Ok(Self {
            clock,
            deadline_budget,
            sector_count,
            reads: 0,
        })
    }

    pub const fn reads(&self) -> u64 {
        self.reads
    }
}

#[cfg(not(bndroid_storage_irq_timeout_profile))]
impl<C: PollClock + ?Sized> BlockReader for IrqBlockReader<'_, C> {
    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u64, output: &mut Sector) -> Result<(), BlockReadError> {
        if PARSER_SECTOR_SIZE != BLOCK_SECTOR_SIZE || lba >= self.sector_count {
            return Err(BlockReadError::OutOfBounds);
        }
        let deadline =
            deadline_after(self.clock, self.deadline_budget).ok_or(BlockReadError::Device)?;
        read_sector_irq(lba, output, self.clock, deadline).map_err(|error| match error {
            StorageError::Block(BlockError::SectorOutOfBounds) => BlockReadError::OutOfBounds,
            _ => BlockReadError::Device,
        })?;
        self.reads = self.reads.saturating_add(1);
        Ok(())
    }
}

#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
#[cfg(feature = "cooperative-block-recovery")]
pub fn begin_async_recovery<C: PollClock + ?Sized>(
    clock: &mut C,
    reset_deadline: u64,
) -> Result<CooperativeRecoveryProgress, StorageError> {
    if IRQ_ARMED.load(Ordering::Acquire) || IRQ_REARM_PREPARED.load(Ordering::Acquire) {
        return Err(StorageError::IrqMustBeDisarmed);
    }
    if !RECOVERY_REQUIRED.load(Ordering::Acquire)
        || !RECOVERY_ADMISSION_CLOSED.load(Ordering::Acquire)
    {
        return Err(StorageError::RecoveryRequired);
    }
    if ASYNC_RECOVERY_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(StorageError::RecoveryRequired);
    }
    let progress = with_async_recovery_device(|device| {
        device.begin_cooperative_recovery(clock, reset_deadline)
    });
    publish_async_recovery_progress(progress)
}

#[cfg(feature = "cooperative-block-recovery")]
pub fn poll_async_recovery<C: PollClock + ?Sized>(
    clock: &mut C,
) -> Result<CooperativeRecoveryProgress, StorageError> {
    if !ASYNC_RECOVERY_ACTIVE.load(Ordering::Acquire) {
        return Err(StorageError::RecoveryRequired);
    }
    let progress = with_async_recovery_device(|device| device.poll_cooperative_recovery(clock));
    publish_async_recovery_progress(progress)
}

#[cfg(feature = "cooperative-block-recovery")]
pub fn async_recovery_active() -> bool {
    ASYNC_RECOVERY_ACTIVE.load(Ordering::Acquire)
}

#[cfg(feature = "cooperative-block-recovery")]
pub fn async_recovery_snapshot() -> AsyncRecoverySnapshot {
    AsyncRecoverySnapshot {
        active: ASYNC_RECOVERY_ACTIVE.load(Ordering::Acquire),
        phase: ASYNC_RECOVERY_PHASE.load(Ordering::Acquire),
        steps: ASYNC_RECOVERY_STEPS.load(Ordering::Acquire),
        pending_returns: ASYNC_RECOVERY_PENDING_RETURNS.load(Ordering::Acquire),
        max_masked_counter_ticks: ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS.load(Ordering::Acquire),
        masked_poll_iterations: 0,
    }
}

#[cfg(feature = "cooperative-block-recovery")]
fn with_async_recovery_device<T>(
    operation: impl FnOnce(&mut VirtioBlock) -> T,
) -> Result<T, StorageError> {
    if aarch64::irq_is_masked() {
        panic!("cooperative block phase entered with local IRQ already masked");
    }
    let saved_daif = aarch64::save_and_mask_irq();
    let started = aarch64::timer::counter_value();
    let result = BLOCK_DEVICE.with_device_masked(operation);
    let elapsed = aarch64::timer::counter_value().wrapping_sub(started);
    ASYNC_RECOVERY_STEPS.fetch_add(1, Ordering::Relaxed);
    ASYNC_RECOVERY_MAX_MASKED_COUNTER_TICKS.fetch_max(elapsed, Ordering::Relaxed);
    aarch64::restore_daif(saved_daif);
    result
}

#[cfg(feature = "cooperative-block-recovery")]
fn publish_async_recovery_progress(
    progress: Result<Result<CooperativeRecoveryProgress, BlockError>, StorageError>,
) -> Result<CooperativeRecoveryProgress, StorageError> {
    match progress {
        Ok(Ok(CooperativeRecoveryProgress::Pending(phase))) => {
            ASYNC_RECOVERY_PHASE.store(async_recovery_phase_code(phase), Ordering::Release);
            ASYNC_RECOVERY_PENDING_RETURNS.fetch_add(1, Ordering::Relaxed);
            Ok(CooperativeRecoveryProgress::Pending(phase))
        }
        Ok(Ok(CooperativeRecoveryProgress::Complete)) => {
            ASYNC_RECOVERY_PHASE.store(0, Ordering::Release);
            ASYNC_RECOVERY_ACTIVE.store(false, Ordering::Release);
            Ok(CooperativeRecoveryProgress::Complete)
        }
        Ok(Err(error)) => {
            ASYNC_RECOVERY_PHASE.store(0, Ordering::Release);
            ASYNC_RECOVERY_ACTIVE.store(false, Ordering::Release);
            IRQ_FAILED.store(true, Ordering::Release);
            RECOVERY_REQUIRED.store(true, Ordering::Release);
            Err(error.into())
        }
        Err(error) => {
            ASYNC_RECOVERY_PHASE.store(0, Ordering::Release);
            ASYNC_RECOVERY_ACTIVE.store(false, Ordering::Release);
            IRQ_FAILED.store(true, Ordering::Release);
            RECOVERY_REQUIRED.store(true, Ordering::Release);
            Err(error)
        }
    }
}

#[cfg(feature = "cooperative-block-recovery")]
const fn async_recovery_phase_code(phase: BlockRecoveryPhase) -> u8 {
    match phase {
        BlockRecoveryPhase::Idle => 0,
        BlockRecoveryPhase::AwaitReset => 1,
        BlockRecoveryPhase::RebuildQueue => 2,
        BlockRecoveryPhase::AwaitStableCapacity => 3,
        BlockRecoveryPhase::AwaitCleanupReset => 4,
    }
}

#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
pub fn recover_after_timeout<C: PollClock + ?Sized>(
    clock: &mut C,
    reset_deadline: u64,
) -> Result<(), StorageError> {
    if IRQ_ARMED.load(Ordering::Acquire) || IRQ_REARM_PREPARED.load(Ordering::Acquire) {
        return Err(StorageError::IrqMustBeDisarmed);
    }
    let recovered =
        with_foreground_device(|device| device.recover_after_timeout(clock, reset_deadline));
    match recovered {
        Ok(Ok(())) => {
            // Physical recovery alone does not make interrupt-driven I/O
            // safe. The latches are cleared only by commit_irq_rearm after
            // GIC enable succeeds.
            Ok(())
        }
        Ok(Err(error)) => {
            IRQ_FAILED.store(true, Ordering::Release);
            RECOVERY_REQUIRED.store(true, Ordering::Release);
            Err(error.into())
        }
        Err(error) => {
            IRQ_FAILED.store(true, Ordering::Release);
            RECOVERY_REQUIRED.store(true, Ordering::Release);
            Err(error)
        }
    }
}

fn take_available(
    tokens: [ReadToken; 2],
    outputs: &mut [[u8; BLOCK_SECTOR_SIZE]; 2],
    terminal: &mut [bool; 2],
    first_error: &mut Option<BlockError>,
) -> Result<(), StorageError> {
    match with_foreground_device(|device| {
        drain_read_pair_masked(device, tokens, outputs, terminal, first_error)
    }) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) | Err(_) => Err(read_pair_recovery_required()),
    }
}

/// Takes every available member of the pair before reporting a terminal
/// device status. `Failed` means the corresponding token was already freed;
/// structural tracker errors do not carry that proof and force reset recovery.
fn drain_read_pair_masked(
    device: &mut VirtioBlock,
    tokens: [ReadToken; 2],
    outputs: &mut [[u8; BLOCK_SECTOR_SIZE]; 2],
    terminal: &mut [bool; 2],
    first_error: &mut Option<BlockError>,
) -> Result<(), BlockError> {
    let outcomes = device.take_read_pair_available(tokens, outputs, *terminal);
    let mut structural_error = None;
    for (index, outcome) in outcomes.into_iter().enumerate() {
        let Some(outcome) = outcome else {
            continue;
        };
        match outcome {
            Ok(ReadTakeOutcome::Pending) => {}
            Ok(ReadTakeOutcome::Success) => terminal[index] = true,
            Ok(ReadTakeOutcome::Failed(error)) => {
                terminal[index] = true;
                if first_error.is_none() {
                    *first_error = Some(error);
                }
            }
            Err(error) => {
                if structural_error.is_none() {
                    structural_error = Some(error);
                }
            }
        }
    }
    match structural_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn read_pair_recovery_required() -> StorageError {
    RECOVERY_REQUIRED.store(true, Ordering::Release);
    StorageError::RecoveryRequired
}

/// Takes a previously published write/flush completion. `RequestPending` is
/// the only non-terminal result: every other failure loses the proof needed to
/// classify the device-side mutation and therefore latches reset recovery.
#[cfg_attr(bndroid_storage_irq_timeout_profile, allow(dead_code))]
fn take_submitted_mutation(
    take: impl FnOnce(&mut VirtioBlock) -> Result<(), BlockError>,
) -> Result<bool, StorageError> {
    match with_foreground_device(take) {
        Ok(Ok(())) => Ok(true),
        Ok(Err(BlockError::RequestPending)) => Ok(false),
        Ok(Err(_)) | Err(_) => Err(submitted_mutation_failed()),
    }
}

/// Performs the one permitted foreground drain at the deadline. A completed
/// success remains success; a still-pending request or any parsing/take error
/// is an unknown submitted mutation, never a pre-submission device error.
#[cfg_attr(bndroid_storage_irq_timeout_profile, allow(dead_code))]
fn resolve_submitted_mutation_timeout(
    take: impl FnOnce(&mut VirtioBlock) -> Result<(), BlockError>,
) -> Result<(), StorageError> {
    let resolution = with_foreground_device(|device| {
        resolve_deadline_race(device)?;
        match take(device) {
            Ok(()) => Ok(true),
            Err(BlockError::RequestPending) => {
                device.record_timeout();
                Ok(false)
            }
            Err(error) => Err(error),
        }
    });
    match resolution {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false) | Err(_)) | Err(_) => Err(submitted_mutation_failed()),
    }
}

#[cfg_attr(bndroid_storage_irq_timeout_profile, allow(dead_code))]
fn submitted_mutation_failed() -> StorageError {
    RECOVERY_REQUIRED.store(true, Ordering::Release);
    StorageError::SubmittedMutationOutcomeUnknown
}

/// Resolves the final completion race while preserving every acknowledged
/// interrupt bit. A config-change notification is latched before the method
/// reports `DeviceNeedsReset`; a used-ring failure is likewise fail-closed.
fn resolve_deadline_race(device: &mut VirtioBlock) -> Result<u16, BlockError> {
    let resolution = device.resolve_timeout_race_observed();
    let completions = resolution.completions.unwrap_or(0);
    record_interrupt_service(
        InterruptService {
            bits: resolution.bits,
            completions,
        },
        false,
    );
    if resolution.config_changed() {
        return Err(BlockError::DeviceNeedsReset);
    }
    match resolution.completions {
        Ok(completions) => Ok(completions),
        Err(error) => {
            record_interrupt_failure();
            Err(error)
        }
    }
}

fn with_foreground_device<T>(
    operation: impl FnOnce(&mut VirtioBlock) -> T,
) -> Result<T, StorageError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let result = BLOCK_DEVICE.with_device_masked(operation);
    aarch64::restore_daif(saved_daif);
    result
}

fn ensure_io_ready() -> Result<(), StorageError> {
    if RECOVERY_ADMISSION_CLOSED.load(Ordering::Acquire)
        || RECOVERY_REQUIRED.load(Ordering::Acquire)
    {
        return Err(StorageError::RecoveryRequired);
    }
    if IRQ_FAILED.load(Ordering::Acquire) {
        return Err(StorageError::InterruptFailure);
    }
    Ok(())
}

pub fn last_status_ok() -> Result<bool, StorageError> {
    Ok(stats()?.last_status == BLOCK_STATUS_OK)
}
