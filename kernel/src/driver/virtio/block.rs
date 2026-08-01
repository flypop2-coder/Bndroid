use core::hint::spin_loop;
use core::ptr::{copy_nonoverlapping, read_volatile, write_bytes, write_volatile};

use bndroid_kernel::memory::{self, AllocateError, DeallocateError, OwnedFrame, PAGE_SIZE};
use bndroid_kernel::time::deadline_reached;
pub use bndroid_kernel::virtio::RequestToken as BlockToken;
pub type ReadToken = BlockToken;
#[allow(dead_code)]
pub type WriteToken = BlockToken;
#[allow(dead_code)]
pub type FlushToken = BlockToken;
use bndroid_kernel::virtio::{
    BLOCK_DESCRIPTOR_COUNT, BLOCK_REQUEST_SLOT_COUNT, BLOCK_SECTOR_SIZE, BLOCK_STATUS_IO_ERROR,
    BLOCK_STATUS_OK, BLOCK_STATUS_PENDING, BLOCK_STATUS_UNSUPPORTED, BlockDeviceMode,
    BlockIrqRearmError, BlockRecoveryTracker, BlockRequestHeader, BlockRequestKind,
    BlockRequestLayout, DEVICE_ID_BLOCK, DEVICE_ID_PLACEHOLDER, Descriptor, FEATURE_VERSION_1,
    FeatureError, LayoutError, MMIO_MAGIC, MMIO_VERSION_MODERN, NegotiatedFeatures, QUEUE_INDEX,
    QUEUE_SIZE, RequestTracker, RequestTrackerError, STATUS_ACKNOWLEDGE, STATUS_DEVICE_NEEDS_RESET,
    STATUS_DRIVER, STATUS_DRIVER_OK, STATUS_FAILED, STATUS_FEATURES_OK, SplitQueueLayout,
    UsedElement, block_interrupt_requires_recovery, negotiate_features_for_mode, sector_in_bounds,
    valid_block_used_length, validate_block_irq_rearm_snapshot,
};
#[cfg(feature = "cooperative-block-recovery")]
use bndroid_kernel::virtio::{
    BlockRecoveryPhase, BlockRecoveryStart, BlockRecoveryTransitionError,
};
#[cfg(any(
    feature = "storage-server-recovery-runtime",
    feature = "app-data-async-recovery-runtime"
))]
use bndroid_kernel::virtio::{RecoveryNotificationFault, RecoveryNotificationFaultError};

use super::mmio::{MmioError, MmioTransport};
use crate::arch::aarch64::{dma, timer};

const REQUEST_SLOTS: usize = BLOCK_REQUEST_SLOT_COUNT as usize;

pub trait PollClock {
    fn now(&mut self) -> u64;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicalCounter;

impl PollClock for PhysicalCounter {
    fn now(&mut self) -> u64 {
        timer::counter_value()
    }
}

/// Builds a serial-number-safe absolute deadline for an injectable clock.
pub fn deadline_after<C: PollClock + ?Sized>(clock: &mut C, budget: u64) -> Option<u64> {
    if budget == 0 || budget > i64::MAX as u64 {
        return None;
    }
    Some(clock.now().wrapping_add(budget))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    Mmio(MmioError),
    BadMagic,
    LegacyTransport,
    UnsupportedVersion,
    NonCoherentDma,
    MissingVersion1,
    MissingReadOnly,
    ReadOnlyDevice,
    MissingFlush,
    FeaturesRejected,
    DeviceNeedsReset,
    QueueUnavailable,
    QueueTooSmall,
    QueueAlreadyReady,
    Layout(LayoutError),
    Allocation(AllocateError),
    FrameRelease(DeallocateError),
    ResetTimeout,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    InjectedPersistentRebuildFailure,
    CapacityUnstable,
    DeadlineExpired,
    SectorOutOfBounds,
    DeviceIo,
    UnsupportedRequest,
    ProtocolViolation,
    DriverStopped,
    QueueFull,
    RequestPending,
    StaleRequest,
    RequestKindMismatch,
}

impl BlockError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mmio(error) => error.as_str(),
            Self::BadMagic => "invalid virtio MMIO magic",
            Self::LegacyTransport => "legacy virtio MMIO transport is unsupported",
            Self::UnsupportedVersion => "unsupported virtio MMIO version",
            Self::NonCoherentDma => "non-coherent DMA is unsupported",
            Self::MissingVersion1 => "device does not offer VIRTIO_F_VERSION_1",
            Self::MissingReadOnly => "read-only block device does not offer VIRTIO_BLK_F_RO",
            Self::ReadOnlyDevice => "block device is physically read-only",
            Self::MissingFlush => "writable block device does not offer VIRTIO_BLK_F_FLUSH",
            Self::FeaturesRejected => "device rejected negotiated virtio features",
            Self::DeviceNeedsReset => "virtio device requested reset",
            Self::QueueUnavailable => "virtio block request queue is unavailable",
            Self::QueueTooSmall => "virtio block request queue is too small",
            Self::QueueAlreadyReady => "virtio block request queue is already active",
            Self::Layout(error) => error.as_str(),
            Self::Allocation(error) => error.as_str(),
            Self::FrameRelease(error) => error.as_str(),
            Self::ResetTimeout => "virtio device reset timed out",
            #[cfg(feature = "storage-server-fault-policy-runtime")]
            Self::InjectedPersistentRebuildFailure => {
                "test profile injected a persistent virtio queue rebuild failure"
            }
            Self::CapacityUnstable => "virtio block capacity did not stabilize",
            Self::DeadlineExpired => "virtio block request timed out",
            Self::SectorOutOfBounds => "virtio block sector is out of bounds",
            Self::DeviceIo => "virtio block device reported an I/O error",
            Self::UnsupportedRequest => "virtio block device rejected the request",
            Self::ProtocolViolation => "virtio block device violated the queue protocol",
            Self::DriverStopped => "virtio block driver is stopped",
            Self::QueueFull => "virtio block request slots are full",
            Self::RequestPending => "virtio block request is still pending",
            Self::StaleRequest => "virtio block request token is stale",
            Self::RequestKindMismatch => "virtio block request token has the wrong operation kind",
        }
    }
}

impl From<MmioError> for BlockError {
    fn from(error: MmioError) -> Self {
        Self::Mmio(error)
    }
}

impl From<LayoutError> for BlockError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<AllocateError> for BlockError {
    fn from(error: AllocateError) -> Self {
        Self::Allocation(error)
    }
}

impl From<FeatureError> for BlockError {
    fn from(error: FeatureError) -> Self {
        match error {
            FeatureError::MissingVersion1 => Self::MissingVersion1,
            FeatureError::MissingReadOnly => Self::MissingReadOnly,
            FeatureError::ReadOnlyDevice => Self::ReadOnlyDevice,
            FeatureError::MissingFlush => Self::MissingFlush,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockStats {
    pub requests: u64,
    pub completions: u64,
    pub read_requests: u64,
    pub write_requests: u64,
    pub flush_requests: u64,
    pub read_completions: u64,
    pub write_completions: u64,
    pub flush_completions: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub successful_flushes: u64,
    pub timeouts: u64,
    pub resets: u64,
    pub out_of_range_rejections: u64,
    pub avail_idx: u16,
    pub used_idx: u16,
    pub last_status: u8,
    pub interrupt_notifications: u64,
    pub interrupt_completions: u64,
    pub max_outstanding: u16,
    pub queue_full_rejections: u64,
    pub stale_token_rejections: u64,
    pub foreground_drains: u64,
    pub suppressed_read_notifications: u64,
    pub suppressed_write_notifications: u64,
    pub suppressed_flush_notifications: u64,
}

impl BlockStats {
    const fn new() -> Self {
        Self {
            requests: 0,
            completions: 0,
            read_requests: 0,
            write_requests: 0,
            flush_requests: 0,
            read_completions: 0,
            write_completions: 0,
            flush_completions: 0,
            bytes_read: 0,
            bytes_written: 0,
            successful_flushes: 0,
            timeouts: 0,
            resets: 0,
            out_of_range_rejections: 0,
            avail_idx: 0,
            used_idx: 0,
            last_status: BLOCK_STATUS_PENDING,
            interrupt_notifications: 0,
            interrupt_completions: 0,
            max_outstanding: 0,
            queue_full_rejections: 0,
            stale_token_rejections: 0,
            foreground_drains: 0,
            suppressed_read_notifications: 0,
            suppressed_write_notifications: 0,
            suppressed_flush_notifications: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InterruptService {
    pub bits: u32,
    pub completions: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReadTakeOutcome {
    Pending,
    Success,
    Failed(BlockError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TimeoutRaceResolution {
    pub bits: u32,
    pub completions: Result<u16, BlockError>,
}

impl TimeoutRaceResolution {
    pub const fn config_changed(self) -> bool {
        block_interrupt_requires_recovery(self.bits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IrqRearmObservation {
    pub bits: u32,
    pub validation: Result<(), BlockError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "cooperative-block-recovery")]
pub enum CooperativeRecoveryProgress {
    Pending(BlockRecoveryPhase),
    Complete,
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PersistentRebuildFaultSnapshot {
    pub armed: bool,
    pub hits: u64,
}

/// Read-only evidence used before M60 publishes a terminal Offline state.
///
/// `requests_terminal` is deliberately narrower than "the device is reset":
/// it proves that the cooperative engine retains no scratch state and that the
/// device owns no published request descriptor. A Ready device is acceptable
/// only while its negotiated driver status is healthy; ResetConfirmed is
/// acceptable only at virtio status zero. In both cases the caller must also
/// prove that the GIC route is disabled and the admission gate is closed.
#[cfg(feature = "storage-server-fault-policy-runtime")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalDmaSnapshot {
    pub driver_state: u8,
    pub transport_status: u32,
    pub in_flight: u16,
    pub recovery_active: bool,
    pub recovery_scratch_clear: bool,
    pub requests_terminal: bool,
}

#[cfg(feature = "storage-server-fault-policy-runtime")]
const TERMINAL_DRIVER_READY: u8 = 1;
#[cfg(feature = "storage-server-fault-policy-runtime")]
const TERMINAL_DRIVER_RESET_CONFIRMED: u8 = 2;

// Early boot intentionally avoids heap allocation for the long-lived driver;
// the probe result is consumed immediately, so enum size is not persistent.
#[allow(clippy::large_enum_variant)]
pub enum ProbeOutcome {
    Placeholder,
    OtherDevice(u32),
    Block(VirtioBlock),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverState {
    Ready,
    ResetConfirmed,
    Recovering,
    Poisoned,
    Released,
}

pub struct VirtioBlock {
    transport: MmioTransport,
    queue_frame: Option<OwnedFrame>,
    request_frame: Option<OwnedFrame>,
    queue_layout: SplitQueueLayout,
    request_layout: BlockRequestLayout,
    mode: BlockDeviceMode,
    features: NegotiatedFeatures,
    capacity_sectors: u64,
    state: DriverState,
    cooperative_recovery: BlockRecoveryTracker,
    cooperative_recovery_cause: Option<BlockError>,
    cooperative_recovered_features: Option<NegotiatedFeatures>,
    stats: BlockStats,
    requests: RequestTracker<REQUEST_SLOTS>,
    #[cfg(bndroid_storage_irq_timeout_profile)]
    suppress_notifications: bool,
    #[cfg(any(
        feature = "storage-server-recovery-runtime",
        feature = "app-data-async-recovery-runtime"
    ))]
    recovery_notification_fault: RecoveryNotificationFault,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    persistent_rebuild_fault_armed: bool,
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    persistent_rebuild_fault_hits: u64,
}

impl VirtioBlock {
    pub const fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    #[allow(dead_code)]
    pub const fn negotiated_features(&self) -> NegotiatedFeatures {
        self.features
    }

    pub const fn device_read_only(&self) -> bool {
        self.features.device_read_only()
    }

    #[allow(dead_code)]
    pub const fn flush_supported(&self) -> bool {
        self.features.flush_supported()
    }

    pub const fn stats(&self) -> BlockStats {
        self.stats
    }

    /// Arms exactly one notification loss for an opt-in recovery proof.
    /// Production profiles do not compile this control surface.
    #[cfg(any(
        feature = "storage-server-recovery-runtime",
        feature = "app-data-async-recovery-runtime"
    ))]
    pub fn suppress_next_notification_for_recovery_test(
        &mut self,
        expected: BlockRequestKind,
    ) -> Result<(), BlockError> {
        if self.state != DriverState::Ready {
            return Err(BlockError::DriverStopped);
        }
        self.recovery_notification_fault
            .arm(expected, self.requests.in_flight())
            .map_err(map_recovery_notification_fault_error)
    }

    #[cfg(any(
        feature = "storage-server-recovery-runtime",
        feature = "app-data-async-recovery-runtime"
    ))]
    pub const fn recovery_notification_fault_armed(&self) -> bool {
        self.recovery_notification_fault.is_armed()
    }

    /// Atomically arms M60's terminal ordinary-read loss and persistent queue
    /// rebuild failure. The recovery coordinator invokes this while the
    /// seventh campaign is still ownerless; no EL0 selector, request, or
    /// syscall reaches this hook.
    ///
    /// This kernel-only control surface exists solely in the M60 fault-policy
    /// runtime. The latch is deliberately persistent: recovery attempts cannot
    /// clear it, so policy code must eventually stop retrying and fail closed.
    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub(crate) fn arm_m60_permanent_read_fault_for_test(&mut self) -> Result<(), BlockError> {
        if self.state != DriverState::Ready
            || self.persistent_rebuild_fault_armed
            || self.requests.in_flight() != 0
            || self.recovery_notification_fault.is_armed()
        {
            return Err(BlockError::DriverStopped);
        }
        self.recovery_notification_fault
            .arm(BlockRequestKind::Read, 0)
            .map_err(map_recovery_notification_fault_error)?;
        self.persistent_rebuild_fault_armed = true;
        Ok(())
    }

    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub const fn persistent_rebuild_fault_snapshot(&self) -> PersistentRebuildFaultSnapshot {
        PersistentRebuildFaultSnapshot {
            armed: self.persistent_rebuild_fault_armed,
            hits: self.persistent_rebuild_fault_hits,
        }
    }

    #[cfg(feature = "storage-server-fault-policy-runtime")]
    pub fn terminal_dma_snapshot(&self) -> TerminalDmaSnapshot {
        let driver_state = match self.state {
            DriverState::Ready => TERMINAL_DRIVER_READY,
            DriverState::ResetConfirmed => TERMINAL_DRIVER_RESET_CONFIRMED,
            DriverState::Recovering => 3,
            DriverState::Poisoned => 4,
            DriverState::Released => 5,
        };
        let transport_status = self.transport.status();
        let in_flight = self.requests.in_flight();
        let recovery_active = self.cooperative_recovery.is_active();
        let recovery_scratch_clear = self.cooperative_recovery_cause.is_none()
            && self.cooperative_recovered_features.is_none();
        let ready_idle = driver_state == TERMINAL_DRIVER_READY
            && transport_status & STATUS_DRIVER_OK != 0
            && transport_status & (STATUS_DEVICE_NEEDS_RESET | STATUS_FAILED) == 0;
        let reset_idle = driver_state == TERMINAL_DRIVER_RESET_CONFIRMED && transport_status == 0;
        TerminalDmaSnapshot {
            driver_state,
            transport_status,
            in_flight,
            recovery_active,
            recovery_scratch_clear,
            requests_terminal: in_flight == 0
                && !recovery_active
                && recovery_scratch_clear
                && (ready_idle || reset_idle),
        }
    }

    pub const fn in_flight(&self) -> u16 {
        self.requests.in_flight()
    }

    pub fn submit_read(&mut self, sector: u64) -> Result<ReadToken, BlockError> {
        self.submit_request(BlockRequestKind::Read, sector, None)
    }

    #[allow(dead_code)]
    pub fn submit_write(
        &mut self,
        sector: u64,
        input: &[u8; BLOCK_SECTOR_SIZE],
    ) -> Result<WriteToken, BlockError> {
        if self.state != DriverState::Ready {
            return Err(BlockError::DriverStopped);
        }
        if self.device_read_only() || self.mode != BlockDeviceMode::Writable {
            return Err(BlockError::ReadOnlyDevice);
        }
        self.submit_request(BlockRequestKind::Write, sector, Some(input))
    }

    /// Submits a cache flush only after every earlier request has been taken.
    ///
    /// Virtio block requests may complete out of order. Requiring every slot
    /// to be free prevents a flush from racing an earlier write and gives the
    /// caller a strict write-completion -> flush-submission durability edge.
    #[allow(dead_code)]
    pub fn submit_flush(&mut self) -> Result<FlushToken, BlockError> {
        if self.state != DriverState::Ready {
            return Err(BlockError::DriverStopped);
        }
        if self.device_read_only() || self.mode != BlockDeviceMode::Writable {
            return Err(BlockError::ReadOnlyDevice);
        }
        if !self.flush_supported() {
            return Err(BlockError::MissingFlush);
        }
        if self.requests.free_count() != REQUEST_SLOTS {
            return Err(BlockError::RequestPending);
        }
        self.submit_request(BlockRequestKind::Flush, 0, None)
    }

    fn submit_request(
        &mut self,
        kind: BlockRequestKind,
        sector: u64,
        write_data: Option<&[u8; BLOCK_SECTOR_SIZE]>,
    ) -> Result<BlockToken, BlockError> {
        if self.state != DriverState::Ready {
            return Err(BlockError::DriverStopped);
        }
        if kind != BlockRequestKind::Flush && !sector_in_bounds(self.capacity_sectors, sector) {
            self.stats.out_of_range_rejections =
                self.stats.out_of_range_rejections.saturating_add(1);
            return Err(BlockError::SectorOutOfBounds);
        }
        if matches!(kind, BlockRequestKind::Write) != write_data.is_some() {
            return Err(BlockError::ProtocolViolation);
        }
        #[cfg(any(
            feature = "storage-server-recovery-runtime",
            feature = "app-data-async-recovery-runtime"
        ))]
        self.recovery_notification_fault
            .validate_submission(kind)
            .map_err(map_recovery_notification_fault_error)?;

        let token = match self.requests.reserve_kind(kind) {
            Ok(token) => token,
            Err(RequestTrackerError::Full) => {
                self.stats.queue_full_rejections =
                    self.stats.queue_full_rejections.saturating_add(1);
                return Err(BlockError::QueueFull);
            }
            Err(error) => return Err(map_tracker_error(error)),
        };
        let slot_index = usize::from(token.slot());
        let descriptor_head = (slot_index * BLOCK_DESCRIPTOR_COUNT) as u16;

        self.prepare_request(slot_index, kind, sector, write_data);
        self.write_request_descriptors(slot_index, descriptor_head, kind);
        let available_slot = self.stats.avail_idx & (QUEUE_SIZE - 1);
        self.write_available_element(available_slot, descriptor_head);
        dma::publish_to_device();
        let next_available = self.stats.avail_idx.wrapping_add(1);
        self.stats.max_outstanding = self.requests.max_in_flight();
        self.write_available_index(next_available);
        dma::publish_to_device();

        self.stats.requests = self.stats.requests.saturating_add(1);
        match kind {
            BlockRequestKind::Read => {
                self.stats.read_requests = self.stats.read_requests.saturating_add(1)
            }
            BlockRequestKind::Write => {
                self.stats.write_requests = self.stats.write_requests.saturating_add(1)
            }
            BlockRequestKind::Flush => {
                self.stats.flush_requests = self.stats.flush_requests.saturating_add(1)
            }
        }
        self.stats.avail_idx = next_available;
        self.stats.last_status = BLOCK_STATUS_PENDING;
        #[cfg(not(any(
            bndroid_storage_irq_timeout_profile,
            feature = "storage-server-recovery-runtime",
            feature = "app-data-async-recovery-runtime"
        )))]
        self.transport.notify_queue(QUEUE_INDEX);
        #[cfg(bndroid_storage_irq_timeout_profile)]
        if !self.suppress_notifications {
            self.transport.notify_queue(QUEUE_INDEX);
        }
        #[cfg(any(
            feature = "storage-server-recovery-runtime",
            feature = "app-data-async-recovery-runtime"
        ))]
        if self
            .recovery_notification_fault
            .consume_published(kind)
            .unwrap_or_else(|_| panic!("validated recovery notification fault changed kind"))
        {
            match kind {
                BlockRequestKind::Read => {
                    self.stats.suppressed_read_notifications =
                        self.stats.suppressed_read_notifications.saturating_add(1);
                }
                BlockRequestKind::Write => {
                    self.stats.suppressed_write_notifications =
                        self.stats.suppressed_write_notifications.saturating_add(1);
                }
                BlockRequestKind::Flush => {
                    self.stats.suppressed_flush_notifications =
                        self.stats.suppressed_flush_notifications.saturating_add(1);
                }
            }
        } else {
            self.transport.notify_queue(QUEUE_INDEX);
        }
        Ok(token)
    }

    /// Atomically validates queue capacity and both LBAs before exposing either
    /// descriptor chain. The caller must serialize this short operation with
    /// the hard-IRQ completion path.
    pub fn submit_read_pair(&mut self, sectors: [u64; 2]) -> Result<[ReadToken; 2], BlockError> {
        if self.state != DriverState::Ready {
            return Err(BlockError::DriverStopped);
        }
        if sectors
            .into_iter()
            .any(|sector| !sector_in_bounds(self.capacity_sectors, sector))
        {
            self.stats.out_of_range_rejections =
                self.stats.out_of_range_rejections.saturating_add(1);
            return Err(BlockError::SectorOutOfBounds);
        }
        if self.requests.free_count() < 2 {
            self.stats.queue_full_rejections = self.stats.queue_full_rejections.saturating_add(1);
            return Err(BlockError::QueueFull);
        }

        let first = self.submit_read(sectors[0])?;
        let second = self.submit_read(sectors[1])?;
        Ok([first, second])
    }

    /// Reads and ACKs the virtio interrupt source, then drains every published
    /// used entry. The GIC dispatcher must call this before EOIR.
    pub fn service_interrupt(&mut self) -> Result<InterruptService, BlockError> {
        if self.state != DriverState::Ready {
            return Ok(InterruptService::default());
        }
        let mut bits = self.transport.interrupt_status() & 0x3;
        if bits != 0 {
            self.transport.acknowledge_interrupt(bits);
        }
        let mut completions = if bits & 1 != 0 {
            self.reap_completions(true)?
        } else {
            0
        };

        // Close the ACK/drain race: a completion published while the first
        // source was being cleared must either be drained now or leave a new
        // asserted source for a later IRQ entry.
        let late_bits = self.transport.interrupt_status() & 0x3;
        if late_bits != 0 {
            self.transport.acknowledge_interrupt(late_bits);
            bits |= late_bits;
            if late_bits & 1 != 0 {
                completions = completions.saturating_add(self.reap_completions(true)?);
            }
        }
        if bits != 0 {
            self.stats.interrupt_notifications =
                self.stats.interrupt_notifications.saturating_add(1);
        }
        Ok(InterruptService { bits, completions })
    }

    /// Deadline-only drain which preserves the acknowledged interrupt bits
    /// even when used-ring validation fails. Storage owns the policy latch and
    /// must observe bit 1 before classifying any submitted operation.
    pub(crate) fn resolve_timeout_race_observed(&mut self) -> TimeoutRaceResolution {
        let bits = self.transport.interrupt_status() & 0x3;
        if bits != 0 {
            self.transport.acknowledge_interrupt(bits);
        }
        let completions = self.reap_completions(false);
        if let Ok(completed) = completions {
            self.stats.foreground_drains = self
                .stats
                .foreground_drains
                .saturating_add(u64::from(completed));
        }
        TimeoutRaceResolution { bits, completions }
    }

    /// Audits the rebuilt queue after the GIC route is live but before the
    /// storage layer clears its recovery latch. The GIC recovery enable path
    /// preserves pending state, so an edge arriving after the final ISR sample
    /// remains deliverable once the logical gate commits.
    pub(crate) fn observe_irq_rearm_tail(&mut self) -> IrqRearmObservation {
        if self.state != DriverState::Ready {
            return IrqRearmObservation {
                bits: 0,
                validation: Err(BlockError::DriverStopped),
            };
        }

        dma::acquire_from_device();
        let used_before = self.read_used_index();
        let initial_validation = validate_block_identity(&self.transport).and_then(|()| {
            validate_block_irq_rearm_snapshot(0, 0, 0, 0, [0; 3], self.transport.status())
                .map_err(map_irq_rearm_error)
        });

        // Two samples close the device ISR read/ACK race. The final sample is
        // intentionally after the first DMA/status audit so a configuration
        // edge raised during those checks is also observed immediately.
        let mut bits = self.acknowledge_recovery_interrupt_tail();
        dma::acquire_from_device();
        let used_after = self.read_used_index();
        let late_bits = self.transport.interrupt_status() & 0x3;
        bits |= late_bits;
        if late_bits != 0 {
            self.transport.acknowledge_interrupt(late_bits);
        }
        dma::acquire_from_device();
        let used_final = self.read_used_index();
        let final_status = self.transport.status();

        let validation = initial_validation
            .and_then(|()| validate_block_identity(&self.transport))
            .and_then(|()| {
                validate_block_irq_rearm_snapshot(
                    bits,
                    self.requests.in_flight(),
                    self.stats.avail_idx,
                    self.stats.used_idx,
                    [used_before, used_after, used_final],
                    final_status,
                )
                .map_err(map_irq_rearm_error)
            });
        IrqRearmObservation { bits, validation }
    }

    pub fn take_read(
        &mut self,
        token: ReadToken,
        output: &mut [u8; BLOCK_SECTOR_SIZE],
    ) -> Result<(), BlockError> {
        match self.take_read_completion(token, output)? {
            ReadTakeOutcome::Pending => Err(BlockError::RequestPending),
            ReadTakeOutcome::Success => Ok(()),
            ReadTakeOutcome::Failed(error) => Err(error),
        }
    }

    /// Attempts both reads without short-circuiting after the first terminal
    /// device status. The request tracker therefore releases every completed
    /// sibling slot while leaving a genuinely pending sibling live for the
    /// caller's deadline/recovery decision.
    pub(crate) fn take_read_pair_available(
        &mut self,
        tokens: [ReadToken; 2],
        outputs: &mut [[u8; BLOCK_SECTOR_SIZE]; 2],
        already_terminal: [bool; 2],
    ) -> [Option<Result<ReadTakeOutcome, BlockError>>; 2] {
        if tokens
            .iter()
            .any(|token| token.kind() != BlockRequestKind::Read)
        {
            return [
                Some(Err(BlockError::RequestKindMismatch)),
                Some(Err(BlockError::RequestKindMismatch)),
            ];
        }
        let statuses = self.requests.take_pair_available(tokens, already_terminal);
        let (first_output, second_output) = outputs.split_at_mut(1);
        let first = statuses[0]
            .map(|status| self.finish_read_take(tokens[0], status, &mut first_output[0]));
        let second = statuses[1]
            .map(|status| self.finish_read_take(tokens[1], status, &mut second_output[0]));
        [first, second]
    }

    fn take_read_completion(
        &mut self,
        token: ReadToken,
        output: &mut [u8; BLOCK_SECTOR_SIZE],
    ) -> Result<ReadTakeOutcome, BlockError> {
        if token.kind() != BlockRequestKind::Read {
            return Err(BlockError::RequestKindMismatch);
        }
        let status = self.requests.take(token);
        self.finish_read_take(token, status, output)
    }

    fn finish_read_take(
        &mut self,
        token: ReadToken,
        status: Result<u8, RequestTrackerError>,
        output: &mut [u8; BLOCK_SECTOR_SIZE],
    ) -> Result<ReadTakeOutcome, BlockError> {
        let status = match status {
            Ok(status) => status,
            Err(RequestTrackerError::RequestPending) => return Ok(ReadTakeOutcome::Pending),
            Err(RequestTrackerError::StaleToken) => {
                self.stats.stale_token_rejections =
                    self.stats.stale_token_rejections.saturating_add(1);
                return Err(BlockError::StaleRequest);
            }
            Err(error) => return Err(map_tracker_error(error)),
        };
        match status {
            BLOCK_STATUS_OK => {
                self.copy_request_data(usize::from(token.slot()), output);
                self.stats.bytes_read = self
                    .stats
                    .bytes_read
                    .saturating_add(BLOCK_SECTOR_SIZE as u64);
                Ok(ReadTakeOutcome::Success)
            }
            BLOCK_STATUS_IO_ERROR => Ok(ReadTakeOutcome::Failed(BlockError::DeviceIo)),
            BLOCK_STATUS_UNSUPPORTED => Ok(ReadTakeOutcome::Failed(BlockError::UnsupportedRequest)),
            _ => Err(BlockError::ProtocolViolation),
        }
    }

    #[allow(dead_code)]
    pub fn take_write(&mut self, token: WriteToken) -> Result<(), BlockError> {
        let (_, status) = self.take_status(token, BlockRequestKind::Write)?;
        match status {
            BLOCK_STATUS_OK => {
                self.stats.bytes_written = self
                    .stats
                    .bytes_written
                    .saturating_add(BLOCK_SECTOR_SIZE as u64);
                Ok(())
            }
            BLOCK_STATUS_IO_ERROR => Err(BlockError::DeviceIo),
            BLOCK_STATUS_UNSUPPORTED => Err(BlockError::UnsupportedRequest),
            _ => Err(BlockError::ProtocolViolation),
        }
    }

    #[allow(dead_code)]
    pub fn take_flush(&mut self, token: FlushToken) -> Result<(), BlockError> {
        let (_, status) = self.take_status(token, BlockRequestKind::Flush)?;
        match status {
            BLOCK_STATUS_OK => {
                self.stats.successful_flushes = self.stats.successful_flushes.saturating_add(1);
                Ok(())
            }
            BLOCK_STATUS_IO_ERROR => Err(BlockError::DeviceIo),
            BLOCK_STATUS_UNSUPPORTED => Err(BlockError::UnsupportedRequest),
            _ => Err(BlockError::ProtocolViolation),
        }
    }

    fn take_status(
        &mut self,
        token: BlockToken,
        expected_kind: BlockRequestKind,
    ) -> Result<(usize, u8), BlockError> {
        if token.kind() != expected_kind {
            return Err(BlockError::RequestKindMismatch);
        }
        let slot_index = usize::from(token.slot());
        let status = match self.requests.take(token) {
            Ok(status) => status,
            Err(RequestTrackerError::RequestPending) => return Err(BlockError::RequestPending),
            Err(RequestTrackerError::StaleToken) => {
                self.stats.stale_token_rejections =
                    self.stats.stale_token_rejections.saturating_add(1);
                return Err(BlockError::StaleRequest);
            }
            Err(error) => return Err(map_tracker_error(error)),
        };
        Ok((slot_index, status))
    }

    /// Compatibility polling path retained for early diagnostics and failure
    /// cleanup. M22 normal evidence never calls it after IRQ registration.
    #[allow(dead_code)]
    pub fn read_sector<C: PollClock + ?Sized>(
        &mut self,
        sector: u64,
        output: &mut [u8; BLOCK_SECTOR_SIZE],
        clock: &mut C,
        deadline: u64,
    ) -> Result<(), BlockError> {
        let token = self.submit_read(sector)?;

        loop {
            let interrupt_bits = self.transport.interrupt_status() & 0x3;
            if interrupt_bits != 0 {
                self.transport.acknowledge_interrupt(interrupt_bits);
            }
            if let Err(error) = self.reap_completions(false) {
                return self.fail_request(clock, deadline, error);
            }
            match self.take_read(token, output) {
                Ok(()) => return Ok(()),
                Err(BlockError::RequestPending) => {}
                Err(error) => return Err(error),
            }

            let status = self.transport.status();
            if status & STATUS_DEVICE_NEEDS_RESET != 0 {
                return self.fail_request(clock, deadline, BlockError::DeviceNeedsReset);
            }
            if status & STATUS_FAILED != 0 || status & STATUS_DRIVER_OK == 0 {
                return self.fail_request(clock, deadline, BlockError::ProtocolViolation);
            }
            if deadline_reached(clock.now(), deadline) {
                self.stats.timeouts = self.stats.timeouts.saturating_add(1);
                return self.fail_request(clock, deadline, BlockError::DeadlineExpired);
            }
            spin_loop();
        }
    }

    pub fn record_timeout(&mut self) {
        self.stats.timeouts = self.stats.timeouts.saturating_add(1);
    }

    /// Confirms a full device reset, invalidates every pre-reset token, and
    /// rebuilds the same queue over the still-owned DMA frames. The caller
    /// supplies a fresh reset deadline and keeps the GIC line disabled until
    /// this method succeeds.
    #[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
    pub fn recover_after_timeout<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        reset_deadline: u64,
    ) -> Result<(), BlockError> {
        if self.state == DriverState::Released
            || self.cooperative_recovery.is_active()
            || self.cooperative_recovery_cause.is_some()
            || self.cooperative_recovered_features.is_some()
        {
            return Err(BlockError::DriverStopped);
        }
        let recovery_started = clock.now();
        let remaining = reset_deadline.wrapping_sub(recovery_started);
        // A caller supplies a fresh serial-number-safe deadline. Retain that
        // original budget for failure cleanup; if a malformed/already-expired
        // deadline slips through, cleanup still gets one bounded tick instead
        // of an effectively unbounded wrapped interval.
        let cleanup_budget = if remaining == 0 || remaining > i64::MAX as u64 {
            1
        } else {
            remaining
        };
        match self.state {
            DriverState::Released => unreachable!("released state returned before recovery"),
            DriverState::Ready | DriverState::Poisoned => {
                if reset_and_wait(&mut self.transport, clock, reset_deadline).is_err() {
                    self.state = DriverState::Poisoned;
                    return Err(BlockError::ResetTimeout);
                }
                self.record_confirmed_reset();
            }
            DriverState::ResetConfirmed => {
                // ResetConfirmed is the retryable recovery boundary. If the
                // transport no longer reflects that invariant, establish it
                // again before touching either still-owned DMA page.
                if self.transport.status() != 0 {
                    if reset_and_wait(&mut self.transport, clock, reset_deadline).is_err() {
                        self.state = DriverState::Poisoned;
                        return Err(BlockError::ResetTimeout);
                    }
                    self.record_confirmed_reset();
                }
            }
            DriverState::Recovering => return Err(BlockError::DriverStopped),
        }

        // `fail_request` can also leave the driver at ResetConfirmed. Always
        // invalidate the pre-reset ledger here so every entry path provides
        // the same stale-token and empty-queue guarantees.
        self.requests.invalidate_all();
        self.stats.avail_idx = 0;
        self.stats.used_idx = 0;
        self.stats.last_status = BLOCK_STATUS_PENDING;

        let recovered_features = match self.rebuild_after_confirmed_reset(clock, reset_deadline) {
            Ok(features) => features,
            Err(cause) => {
                let cleanup_deadline = deadline_after(clock, cleanup_budget)
                    .expect("validated nonzero recovery cleanup budget");
                return self.cleanup_failed_recovery(clock, cleanup_deadline, cause);
            }
        };
        self.features = recovered_features;
        self.state = DriverState::Ready;
        #[cfg(bndroid_storage_irq_timeout_profile)]
        {
            self.suppress_notifications = false;
        }
        #[cfg(any(
            feature = "storage-server-recovery-runtime",
            feature = "app-data-async-recovery-runtime"
        ))]
        {
            self.recovery_notification_fault.reset();
        }
        Ok(())
    }

    /// Starts a cooperative reset without retaining `&mut VirtioBlock`
    /// across an IRQ-enabled scheduling boundary. The caller performs this
    /// one bounded transition with local IRQ masked, drops the device borrow,
    /// and invokes [`Self::poll_cooperative_recovery`] from a later monitor
    /// turn.
    #[cfg(feature = "cooperative-block-recovery")]
    pub fn begin_cooperative_recovery<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        reset_deadline: u64,
    ) -> Result<CooperativeRecoveryProgress, BlockError> {
        if self.state == DriverState::Released {
            return Err(BlockError::DriverStopped);
        }
        if self.cooperative_recovery.is_active()
            || self.cooperative_recovery_cause.is_some()
            || self.cooperative_recovered_features.is_some()
        {
            return Err(BlockError::DriverStopped);
        }

        let recovery_started = clock.now();
        let remaining = reset_deadline.wrapping_sub(recovery_started);
        if remaining == 0 || remaining > i64::MAX as u64 {
            return Err(BlockError::DeadlineExpired);
        }
        let cleanup_budget = remaining;
        let start = match self.state {
            DriverState::Released => unreachable!("released state returned before recovery"),
            DriverState::Ready | DriverState::Poisoned => {
                self.transport.write_status(0);
                self.state = DriverState::Poisoned;
                BlockRecoveryStart::AwaitReset
            }
            DriverState::ResetConfirmed if self.transport.status() == 0 => {
                BlockRecoveryStart::ResetConfirmed
            }
            DriverState::ResetConfirmed => {
                self.transport.write_status(0);
                self.state = DriverState::Poisoned;
                BlockRecoveryStart::AwaitReset
            }
            DriverState::Recovering => return Err(BlockError::DriverStopped),
        };
        self.cooperative_recovery
            .begin(start, reset_deadline, cleanup_budget)
            .map_err(map_block_recovery_transition_error)?;
        Ok(CooperativeRecoveryProgress::Pending(
            self.cooperative_recovery.phase(),
        ))
    }

    /// Advances exactly one cooperative recovery phase. There is no polling
    /// loop in this method: an unready reset or unstable capacity produces a
    /// `Pending` result so the storage facade can restore DAIF before retrying.
    #[cfg(feature = "cooperative-block-recovery")]
    pub fn poll_cooperative_recovery<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
    ) -> Result<CooperativeRecoveryProgress, BlockError> {
        match self.cooperative_recovery.phase() {
            BlockRecoveryPhase::Idle => Err(BlockError::ProtocolViolation),
            BlockRecoveryPhase::AwaitReset => {
                if self.transport.status() == 0 {
                    dma::acquire_from_device();
                    self.record_confirmed_reset();
                    self.cooperative_recovery
                        .confirm_reset()
                        .map_err(map_block_recovery_transition_error)?;
                    return Ok(CooperativeRecoveryProgress::Pending(
                        BlockRecoveryPhase::RebuildQueue,
                    ));
                }
                if deadline_reached(clock.now(), self.cooperative_recovery.deadline()) {
                    self.state = DriverState::Poisoned;
                    self.cooperative_recovery
                        .abort()
                        .map_err(map_block_recovery_transition_error)?;
                    return Err(BlockError::ResetTimeout);
                }
                Ok(CooperativeRecoveryProgress::Pending(
                    BlockRecoveryPhase::AwaitReset,
                ))
            }
            BlockRecoveryPhase::RebuildQueue => {
                // `fail_request` can enter recovery at ResetConfirmed. Always
                // invalidate the old ledger before the first queue write.
                self.requests.invalidate_all();
                self.stats.avail_idx = 0;
                self.stats.used_idx = 0;
                self.stats.last_status = BLOCK_STATUS_PENDING;
                #[cfg(feature = "storage-server-fault-policy-runtime")]
                if self.persistent_rebuild_fault_armed {
                    self.persistent_rebuild_fault_hits =
                        self.persistent_rebuild_fault_hits.saturating_add(1);
                    return self.begin_cooperative_cleanup(
                        clock,
                        BlockError::InjectedPersistentRebuildFailure,
                    );
                }
                match self.prepare_rebuild_after_confirmed_reset() {
                    Ok(features) => {
                        self.cooperative_recovered_features = Some(features);
                        self.cooperative_recovery
                            .queue_prepared()
                            .map_err(map_block_recovery_transition_error)?;
                        Ok(CooperativeRecoveryProgress::Pending(
                            BlockRecoveryPhase::AwaitStableCapacity,
                        ))
                    }
                    Err(cause) => self.begin_cooperative_cleanup(clock, cause),
                }
            }
            BlockRecoveryPhase::AwaitStableCapacity => {
                let Some(capacity) = self.observe_recovery_capacity_once() else {
                    if deadline_reached(clock.now(), self.cooperative_recovery.deadline()) {
                        return self.begin_cooperative_cleanup(clock, BlockError::CapacityUnstable);
                    }
                    return Ok(CooperativeRecoveryProgress::Pending(
                        BlockRecoveryPhase::AwaitStableCapacity,
                    ));
                };
                if let Err(cause) = self.finish_rebuild_after_stable_capacity(capacity) {
                    return self.begin_cooperative_cleanup(clock, cause);
                }
                let recovered_features = self
                    .cooperative_recovered_features
                    .take()
                    .ok_or(BlockError::ProtocolViolation)?;
                self.cooperative_recovery
                    .complete()
                    .map_err(map_block_recovery_transition_error)?;
                self.features = recovered_features;
                self.state = DriverState::Ready;
                #[cfg(bndroid_storage_irq_timeout_profile)]
                {
                    self.suppress_notifications = false;
                }
                #[cfg(any(
                    feature = "storage-server-recovery-runtime",
                    feature = "app-data-async-recovery-runtime"
                ))]
                {
                    self.recovery_notification_fault.reset();
                }
                Ok(CooperativeRecoveryProgress::Complete)
            }
            BlockRecoveryPhase::AwaitCleanupReset => {
                if self.transport.status() == 0 {
                    dma::acquire_from_device();
                    self.record_confirmed_reset();
                    let cause = self
                        .cooperative_recovery_cause
                        .take()
                        .ok_or(BlockError::ProtocolViolation)?;
                    self.cooperative_recovered_features = None;
                    self.cooperative_recovery
                        .confirm_cleanup()
                        .map_err(map_block_recovery_transition_error)?;
                    return Err(cause);
                }
                if deadline_reached(clock.now(), self.cooperative_recovery.deadline()) {
                    self.state = DriverState::Poisoned;
                    self.cooperative_recovery_cause = None;
                    self.cooperative_recovered_features = None;
                    self.cooperative_recovery
                        .abort()
                        .map_err(map_block_recovery_transition_error)?;
                    return Err(BlockError::ResetTimeout);
                }
                Ok(CooperativeRecoveryProgress::Pending(
                    BlockRecoveryPhase::AwaitCleanupReset,
                ))
            }
        }
    }

    #[cfg(feature = "cooperative-block-recovery")]
    pub const fn cooperative_recovery_phase(&self) -> BlockRecoveryPhase {
        self.cooperative_recovery.phase()
    }

    #[cfg(feature = "cooperative-block-recovery")]
    fn begin_cooperative_cleanup<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        cause: BlockError,
    ) -> Result<CooperativeRecoveryProgress, BlockError> {
        let cleanup_deadline = deadline_after(clock, self.cooperative_recovery.cleanup_budget())
            .ok_or(BlockError::ProtocolViolation)?;
        self.transport.add_status(STATUS_FAILED);
        self.transport.write_status(0);
        self.state = DriverState::Poisoned;
        self.cooperative_recovery_cause = Some(cause);
        self.cooperative_recovered_features = None;
        self.cooperative_recovery
            .begin_cleanup(cleanup_deadline)
            .map_err(map_block_recovery_transition_error)?;
        Ok(CooperativeRecoveryProgress::Pending(
            BlockRecoveryPhase::AwaitCleanupReset,
        ))
    }

    fn rebuild_after_confirmed_reset<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        reset_deadline: u64,
    ) -> Result<NegotiatedFeatures, BlockError> {
        let recovered_features = self.prepare_rebuild_after_confirmed_reset()?;
        let recovered_capacity = stable_capacity(&self.transport, clock, reset_deadline)?;
        self.finish_rebuild_after_stable_capacity(recovered_capacity)?;
        Ok(recovered_features)
    }

    fn prepare_rebuild_after_confirmed_reset(&mut self) -> Result<NegotiatedFeatures, BlockError> {
        if self.state != DriverState::ResetConfirmed || self.transport.status() != 0 {
            return Err(BlockError::ProtocolViolation);
        }
        validate_block_identity(&self.transport)?;
        self.state = DriverState::Recovering;

        let queue_address = self.queue_address();
        let request_address = self.request_address(0);
        unsafe {
            write_bytes(queue_address as *mut u8, 0, PAGE_SIZE);
            write_bytes(request_address as *mut u8, 0, PAGE_SIZE);
        }
        for slot in 0..REQUEST_SLOTS {
            let head = (slot * BLOCK_DESCRIPTOR_COUNT) as u16;
            let descriptors = self.request_layout.descriptors_for_slot(
                request_address as u64,
                slot as u16,
                head,
                QUEUE_SIZE,
            )?;
            for (chain_index, descriptor) in descriptors.into_iter().enumerate() {
                let descriptor_index = slot * BLOCK_DESCRIPTOR_COUNT + chain_index;
                let offset = self
                    .queue_layout
                    .descriptor_offset(descriptor_index as u16)
                    .ok_or(BlockError::ProtocolViolation)?;
                unsafe {
                    write_volatile((queue_address + offset) as *mut Descriptor, descriptor);
                }
            }
            unsafe {
                write_volatile(
                    (request_address
                        + self.request_layout.stride * slot
                        + self.request_layout.status) as *mut u8,
                    BLOCK_STATUS_PENDING,
                );
            }
        }
        unsafe {
            write_volatile(
                (queue_address + self.queue_layout.available_flags_offset()) as *mut u16,
                0,
            );
        }

        self.transport.write_status(STATUS_ACKNOWLEDGE);
        self.transport.add_status(STATUS_DRIVER);
        let setup_status = self.transport.status();
        if setup_status & STATUS_DEVICE_NEEDS_RESET != 0 {
            return Err(BlockError::DeviceNeedsReset);
        }
        if setup_status & STATUS_FAILED != 0
            || setup_status & (STATUS_ACKNOWLEDGE | STATUS_DRIVER)
                != (STATUS_ACKNOWLEDGE | STATUS_DRIVER)
        {
            return Err(BlockError::ProtocolViolation);
        }
        let recovered_features =
            negotiate_features_for_mode(self.transport.device_features(), self.mode)?;
        if recovered_features.selected() != self.features.selected() {
            return Err(BlockError::FeaturesRejected);
        }
        self.transport
            .set_driver_features(recovered_features.selected());
        self.transport.add_status(STATUS_FEATURES_OK);
        let feature_status = self.transport.status();
        if feature_status & (STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK)
            != (STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK)
        {
            return Err(BlockError::FeaturesRejected);
        }
        if feature_status & STATUS_DEVICE_NEEDS_RESET != 0 {
            return Err(BlockError::DeviceNeedsReset);
        }
        if feature_status & STATUS_FAILED != 0 {
            return Err(BlockError::ProtocolViolation);
        }
        Ok(recovered_features)
    }

    #[cfg(feature = "cooperative-block-recovery")]
    fn observe_recovery_capacity_once(&self) -> Option<u64> {
        let generation = self.transport.config_generation();
        let capacity = self.transport.block_capacity_once();
        let observed_generation = self.transport.config_generation();
        (generation == observed_generation).then_some(capacity)
    }

    fn finish_rebuild_after_stable_capacity(
        &mut self,
        recovered_capacity: u64,
    ) -> Result<(), BlockError> {
        if self.state != DriverState::Recovering {
            return Err(BlockError::ProtocolViolation);
        }
        if recovered_capacity != self.capacity_sectors {
            return Err(BlockError::CapacityUnstable);
        }

        let queue_address = self.queue_address();
        self.transport.select_queue(QUEUE_INDEX);
        if self.transport.queue_ready() {
            return Err(BlockError::QueueAlreadyReady);
        }
        if self.transport.queue_size_max() < u32::from(QUEUE_SIZE) {
            return Err(BlockError::QueueTooSmall);
        }
        let descriptor_address = (queue_address + self.queue_layout.descriptor_table)
            .try_into()
            .map_err(|_| BlockError::ProtocolViolation)?;
        let available_address = (queue_address + self.queue_layout.available_ring)
            .try_into()
            .map_err(|_| BlockError::ProtocolViolation)?;
        let used_address = (queue_address + self.queue_layout.used_ring)
            .try_into()
            .map_err(|_| BlockError::ProtocolViolation)?;
        self.transport.configure_queue(
            QUEUE_SIZE,
            descriptor_address,
            available_address,
            used_address,
        );
        dma::publish_to_device();
        self.transport.add_status(STATUS_DRIVER_OK);
        let driver_status = self.transport.status();
        if driver_status & STATUS_DRIVER_OK == 0
            || driver_status & (STATUS_DEVICE_NEEDS_RESET | STATUS_FAILED) != 0
        {
            return Err(BlockError::ProtocolViolation);
        }
        let interrupt_bits = self.acknowledge_recovery_interrupt_tail();
        if interrupt_bits & 2 != 0 {
            // This driver cannot revalidate an asynchronous configuration
            // change in place. Even during reset recovery, observing config
            // bit 1 must therefore leave the device fail-closed.
            return Err(BlockError::DeviceNeedsReset);
        }
        validate_block_identity(&self.transport)?;
        let final_status = self.transport.status();
        if final_status
            & (STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK)
            != (STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK)
            || final_status & (STATUS_DEVICE_NEEDS_RESET | STATUS_FAILED) != 0
        {
            return Err(BlockError::ProtocolViolation);
        }
        Ok(())
    }

    /// Reads and ACKs the standardized ISR twice. The fixed second pass closes
    /// the first read/ACK race without allowing a hostile device to keep
    /// recovery in an unbounded interrupt-drain loop.
    fn acknowledge_recovery_interrupt_tail(&mut self) -> u32 {
        let mut observed = 0;
        for _ in 0..2 {
            let bits = self.transport.interrupt_status() & 0x3;
            observed |= bits;
            if bits != 0 {
                self.transport.acknowledge_interrupt(bits);
            }
        }
        observed
    }

    fn cleanup_failed_recovery<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        reset_deadline: u64,
        cause: BlockError,
    ) -> Result<(), BlockError> {
        self.transport.add_status(STATUS_FAILED);
        if reset_and_wait(&mut self.transport, clock, reset_deadline).is_err() {
            self.state = DriverState::Poisoned;
            return Err(BlockError::ResetTimeout);
        }
        self.record_confirmed_reset();
        Err(cause)
    }

    fn record_confirmed_reset(&mut self) {
        self.stats.resets = self.stats.resets.saturating_add(1);
        self.state = DriverState::ResetConfirmed;
    }

    /// Stops DMA, waits for reset acknowledgement, and returns both frames to
    /// the physical allocator. This is the only reclaiming destruction path.
    #[allow(dead_code)]
    pub fn shutdown<C: PollClock + ?Sized>(
        mut self,
        clock: &mut C,
        deadline: u64,
    ) -> Result<(), BlockError> {
        if self.state != DriverState::ResetConfirmed {
            if reset_and_wait(&mut self.transport, clock, deadline).is_err() {
                self.state = DriverState::Poisoned;
                return Err(BlockError::ResetTimeout);
            }
            self.stats.resets = self.stats.resets.saturating_add(1);
            self.state = DriverState::ResetConfirmed;
        }

        let queue = self
            .queue_frame
            .take()
            .ok_or(BlockError::ProtocolViolation)?;
        let request = self
            .request_frame
            .take()
            .ok_or(BlockError::ProtocolViolation)?;
        self.state = DriverState::Released;
        release_frames(queue, request)
    }

    fn reap_completions(&mut self, interrupt_driven: bool) -> Result<u16, BlockError> {
        let observed_used = self.read_used_index();
        let available = observed_used.wrapping_sub(self.stats.used_idx);
        if usize::from(available) > REQUEST_SLOTS || available > self.requests.in_flight() {
            return Err(BlockError::ProtocolViolation);
        }
        if available == 0 {
            return Ok(0);
        }
        dma::acquire_from_device();

        let mut completed = 0_u16;
        while completed < available {
            let used_slot = self.stats.used_idx & (QUEUE_SIZE - 1);
            let used = self.read_used_element(used_slot);
            let slot_index = RequestTracker::<REQUEST_SLOTS>::slot_for_descriptor_head(
                used.id,
                BLOCK_DESCRIPTOR_COUNT as u16,
            )
            .map_err(map_tracker_error)?;
            let kind = self
                .requests
                .pending_kind_for_descriptor_head(used.id, BLOCK_DESCRIPTOR_COUNT as u16)
                .map_err(map_tracker_error)?;
            let status = self.read_request_status(slot_index);
            if !matches!(
                status,
                BLOCK_STATUS_OK | BLOCK_STATUS_IO_ERROR | BLOCK_STATUS_UNSUPPORTED
            ) || !valid_block_used_length(kind, status, used.length)
            {
                return Err(BlockError::ProtocolViolation);
            }

            self.requests
                .complete_descriptor_head(used.id, BLOCK_DESCRIPTOR_COUNT as u16, status)
                .map_err(map_tracker_error)?;
            self.stats.used_idx = self.stats.used_idx.wrapping_add(1);
            self.stats.completions = self.stats.completions.saturating_add(1);
            match kind {
                BlockRequestKind::Read => {
                    self.stats.read_completions = self.stats.read_completions.saturating_add(1)
                }
                BlockRequestKind::Write => {
                    self.stats.write_completions = self.stats.write_completions.saturating_add(1)
                }
                BlockRequestKind::Flush => {
                    self.stats.flush_completions = self.stats.flush_completions.saturating_add(1)
                }
            }
            self.stats.last_status = status;
            if interrupt_driven {
                self.stats.interrupt_completions =
                    self.stats.interrupt_completions.saturating_add(1);
            }
            completed += 1;
        }
        Ok(completed)
    }

    fn prepare_request(
        &mut self,
        slot: usize,
        kind: BlockRequestKind,
        sector: u64,
        write_data: Option<&[u8; BLOCK_SECTOR_SIZE]>,
    ) {
        let request_base = self.request_address(slot);
        unsafe {
            write_volatile(
                (request_base + self.request_layout.header) as *mut BlockRequestHeader,
                BlockRequestHeader {
                    request_type: kind.request_type().to_le(),
                    reserved: 0,
                    sector: sector.to_le(),
                },
            );
            let data = (request_base + self.request_layout.data) as *mut u8;
            match write_data {
                Some(input) => copy_nonoverlapping(input.as_ptr(), data, BLOCK_SECTOR_SIZE),
                None => write_bytes(data, 0, BLOCK_SECTOR_SIZE),
            }
            write_volatile(
                (request_base + self.request_layout.status) as *mut u8,
                BLOCK_STATUS_PENDING,
            );
        }
    }

    fn write_request_descriptors(
        &mut self,
        slot: usize,
        descriptor_head: u16,
        kind: BlockRequestKind,
    ) {
        let descriptors = self
            .request_layout
            .descriptors_for_slot_kind(
                self.request_address(0) as u64,
                slot as u16,
                descriptor_head,
                QUEUE_SIZE,
                kind,
            )
            .expect("validated request slot must fit the fixed descriptor table");
        for (chain_index, descriptor) in descriptors.into_iter().enumerate() {
            let descriptor_index = slot * BLOCK_DESCRIPTOR_COUNT + chain_index;
            let offset = self
                .queue_layout
                .descriptor_offset(descriptor_index as u16)
                .expect("validated request descriptor must fit the fixed queue");
            unsafe {
                write_volatile(
                    (self.queue_address() + offset) as *mut Descriptor,
                    descriptor,
                );
            }
        }
    }

    fn copy_request_data(&self, slot: usize, output: &mut [u8; BLOCK_SECTOR_SIZE]) {
        unsafe {
            copy_nonoverlapping(
                (self.request_address(slot) + self.request_layout.data) as *const u8,
                output.as_mut_ptr(),
                BLOCK_SECTOR_SIZE,
            );
        }
    }

    fn read_request_status(&self, slot: usize) -> u8 {
        unsafe {
            read_volatile((self.request_address(slot) + self.request_layout.status) as *const u8)
        }
    }

    fn write_available_element(&mut self, slot: u16, descriptor: u16) {
        let offset = self
            .queue_layout
            .available_element_offset(slot)
            .expect("masked queue slot");
        unsafe {
            write_volatile((self.queue_address() + offset) as *mut u16, descriptor);
        }
    }

    fn write_available_index(&mut self, index: u16) {
        unsafe {
            write_volatile(
                (self.queue_address() + self.queue_layout.available_index_offset()) as *mut u16,
                index,
            );
        }
    }

    fn read_used_index(&self) -> u16 {
        unsafe {
            read_volatile(
                (self.queue_address() + self.queue_layout.used_index_offset()) as *const u16,
            )
        }
    }

    fn read_used_element(&self, slot: u16) -> UsedElement {
        let offset = self
            .queue_layout
            .used_element_offset(slot)
            .expect("masked queue slot");
        unsafe { read_volatile((self.queue_address() + offset) as *const UsedElement) }
    }

    fn queue_address(&self) -> usize {
        self.queue_frame
            .as_ref()
            .expect("live driver owns queue frame")
            .start_address()
    }

    fn request_address(&self, slot: usize) -> usize {
        let base = self
            .request_frame
            .as_ref()
            .expect("live driver owns request frame")
            .start_address();
        base + self.request_layout.stride * slot
    }

    #[allow(dead_code)]
    fn fail_request<C: PollClock + ?Sized>(
        &mut self,
        clock: &mut C,
        deadline: u64,
        cause: BlockError,
    ) -> Result<(), BlockError> {
        if reset_and_wait(&mut self.transport, clock, deadline).is_ok() {
            self.stats.resets = self.stats.resets.saturating_add(1);
            self.state = DriverState::ResetConfirmed;
            Err(cause)
        } else {
            self.state = DriverState::Poisoned;
            Err(BlockError::ResetTimeout)
        }
    }
}

impl Drop for VirtioBlock {
    fn drop(&mut self) {
        if matches!(
            self.state,
            DriverState::Ready | DriverState::Recovering | DriverState::Poisoned
        ) {
            // Drop has no clock and cannot prove that the device observed
            // reset. Issue it, but deliberately do not call deallocate_frame:
            // OwnedFrame itself has no reclaiming Drop, so the DMA pages leak
            // instead of becoming reusable while a device might still write.
            self.transport.write_status(0);
            self.state = DriverState::Poisoned;
        }
    }
}

/// Probes and activates one physically read-only virtio block device.
///
/// # Safety
///
/// The MMIO range must satisfy `MmioTransport::new`, be uniquely owned, and
/// stay identity mapped. `dma_coherent` must truthfully describe the platform;
/// this first driver rejects non-coherent DMA because it has no cache
/// maintenance implementation.
#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
pub unsafe fn probe_read_only<C: PollClock + ?Sized>(
    base: usize,
    size: usize,
    dma_coherent: bool,
    clock: &mut C,
    deadline: u64,
) -> Result<ProbeOutcome, BlockError> {
    unsafe {
        probe_mode(
            base,
            size,
            dma_coherent,
            clock,
            deadline,
            BlockDeviceMode::ReadOnly,
        )
    }
}

/// Probes a physically writable device and requires cache-flush support.
///
/// # Safety
///
/// The caller must satisfy the same unique-MMIO, identity-mapping, and DMA
/// coherency requirements as [`probe_read_only`].
#[cfg_attr(bndroid_storage_irq_timeout_profile, allow(dead_code))]
pub unsafe fn probe_writable<C: PollClock + ?Sized>(
    base: usize,
    size: usize,
    dma_coherent: bool,
    clock: &mut C,
    deadline: u64,
) -> Result<ProbeOutcome, BlockError> {
    unsafe {
        probe_mode(
            base,
            size,
            dma_coherent,
            clock,
            deadline,
            BlockDeviceMode::Writable,
        )
    }
}

unsafe fn probe_mode<C: PollClock + ?Sized>(
    base: usize,
    size: usize,
    dma_coherent: bool,
    clock: &mut C,
    deadline: u64,
    mode: BlockDeviceMode,
) -> Result<ProbeOutcome, BlockError> {
    let queue_layout = SplitQueueLayout::new(QUEUE_SIZE, PAGE_SIZE)?;
    let request_layout = BlockRequestLayout::new_slots(BLOCK_REQUEST_SLOT_COUNT, PAGE_SIZE)?;
    let mut transport = unsafe { MmioTransport::new(base, size)? };
    let header = transport.header();

    if header.magic != MMIO_MAGIC {
        return Err(BlockError::BadMagic);
    }
    // QEMU exposes many empty legacy-version slots. Device ID zero is the
    // normative placeholder and wins over version validation while scanning.
    if header.device_id == DEVICE_ID_PLACEHOLDER {
        return Ok(ProbeOutcome::Placeholder);
    }
    if header.device_id != DEVICE_ID_BLOCK {
        return Ok(ProbeOutcome::OtherDevice(header.device_id));
    }
    if header.version == 1 {
        return Err(BlockError::LegacyTransport);
    }
    if header.version != MMIO_VERSION_MODERN {
        return Err(BlockError::UnsupportedVersion);
    }
    if !dma_coherent {
        return Err(BlockError::NonCoherentDma);
    }

    reset_and_wait(&mut transport, clock, deadline)?;
    transport.write_status(STATUS_ACKNOWLEDGE);
    transport.add_status(STATUS_DRIVER);

    let features = match negotiate_features_for_mode(transport.device_features(), mode) {
        Ok(features) => features,
        Err(error) => return Err(fail_active(&mut transport, clock, deadline, error.into())),
    };
    debug_assert_ne!(features.selected() & FEATURE_VERSION_1, 0);
    transport.set_driver_features(features.selected());
    transport.add_status(STATUS_FEATURES_OK);
    let feature_status = transport.status();
    if feature_status & STATUS_FEATURES_OK == 0 {
        return Err(fail_active(
            &mut transport,
            clock,
            deadline,
            BlockError::FeaturesRejected,
        ));
    }
    if feature_status & STATUS_DEVICE_NEEDS_RESET != 0 {
        return Err(fail_active(
            &mut transport,
            clock,
            deadline,
            BlockError::DeviceNeedsReset,
        ));
    }

    let capacity_sectors = match stable_capacity(&transport, clock, deadline) {
        Ok(capacity) => capacity,
        Err(error) => return Err(fail_active(&mut transport, clock, deadline, error)),
    };

    transport.select_queue(QUEUE_INDEX);
    let queue_size_max = transport.queue_size_max();
    if queue_size_max == 0 {
        return Err(fail_active(
            &mut transport,
            clock,
            deadline,
            BlockError::QueueUnavailable,
        ));
    }
    if queue_size_max < u32::from(QUEUE_SIZE) {
        return Err(fail_active(
            &mut transport,
            clock,
            deadline,
            BlockError::QueueTooSmall,
        ));
    }
    if transport.queue_ready() {
        return Err(fail_active(
            &mut transport,
            clock,
            deadline,
            BlockError::QueueAlreadyReady,
        ));
    }

    let queue_frame = match memory::allocate_frame() {
        Ok(frame) => frame,
        Err(error) => {
            return Err(fail_active(
                &mut transport,
                clock,
                deadline,
                BlockError::Allocation(error),
            ));
        }
    };
    let request_frame = match memory::allocate_frame() {
        Ok(frame) => frame,
        Err(error) => {
            return Err(reset_and_release(
                &mut transport,
                Some(queue_frame),
                None,
                clock,
                deadline,
                BlockError::Allocation(error),
            ));
        }
    };

    let queue_address = queue_frame.start_address();
    let request_address = request_frame.start_address();
    if !queue_address.is_multiple_of(PAGE_SIZE) || !request_address.is_multiple_of(PAGE_SIZE) {
        return Err(reset_and_release(
            &mut transport,
            Some(queue_frame),
            Some(request_frame),
            clock,
            deadline,
            BlockError::ProtocolViolation,
        ));
    }
    let request_physical = request_address as u64;
    let first_descriptors =
        match request_layout.descriptors_for_slot(request_physical, 0, 0, QUEUE_SIZE) {
            Ok(descriptors) => descriptors,
            Err(error) => {
                return Err(reset_and_release(
                    &mut transport,
                    Some(queue_frame),
                    Some(request_frame),
                    clock,
                    deadline,
                    BlockError::Layout(error),
                ));
            }
        };
    let second_descriptors = match request_layout.descriptors_for_slot(
        request_physical,
        1,
        BLOCK_DESCRIPTOR_COUNT as u16,
        QUEUE_SIZE,
    ) {
        Ok(descriptors) => descriptors,
        Err(error) => {
            return Err(reset_and_release(
                &mut transport,
                Some(queue_frame),
                Some(request_frame),
                clock,
                deadline,
                BlockError::Layout(error),
            ));
        }
    };

    unsafe {
        write_bytes(queue_address as *mut u8, 0, PAGE_SIZE);
        write_bytes(request_address as *mut u8, 0, PAGE_SIZE);
        for (slot, descriptors) in [first_descriptors, second_descriptors]
            .into_iter()
            .enumerate()
        {
            for (index, descriptor) in descriptors.into_iter().enumerate() {
                let descriptor_index = slot * BLOCK_DESCRIPTOR_COUNT + index;
                let offset = queue_layout
                    .descriptor_offset(descriptor_index as u16)
                    .expect("two three-descriptor requests fit queue");
                write_volatile((queue_address + offset) as *mut Descriptor, descriptor);
            }
        }
        write_volatile(
            (queue_address + queue_layout.available_flags_offset()) as *mut u16,
            0,
        );
        for slot in 0..REQUEST_SLOTS {
            write_volatile(
                (request_address + request_layout.stride * slot + request_layout.status) as *mut u8,
                BLOCK_STATUS_PENDING,
            );
        }
    }

    let descriptor_address = match (queue_address + queue_layout.descriptor_table).try_into() {
        Ok(address) => address,
        Err(_) => {
            return Err(reset_and_release(
                &mut transport,
                Some(queue_frame),
                Some(request_frame),
                clock,
                deadline,
                BlockError::ProtocolViolation,
            ));
        }
    };
    let available_address = match (queue_address + queue_layout.available_ring).try_into() {
        Ok(address) => address,
        Err(_) => {
            return Err(reset_and_release(
                &mut transport,
                Some(queue_frame),
                Some(request_frame),
                clock,
                deadline,
                BlockError::ProtocolViolation,
            ));
        }
    };
    let used_address = match (queue_address + queue_layout.used_ring).try_into() {
        Ok(address) => address,
        Err(_) => {
            return Err(reset_and_release(
                &mut transport,
                Some(queue_frame),
                Some(request_frame),
                clock,
                deadline,
                BlockError::ProtocolViolation,
            ));
        }
    };
    transport.configure_queue(
        QUEUE_SIZE,
        descriptor_address,
        available_address,
        used_address,
    );
    dma::publish_to_device();
    transport.add_status(STATUS_DRIVER_OK);
    let driver_status = transport.status();
    if driver_status & STATUS_DRIVER_OK == 0
        || driver_status & (STATUS_DEVICE_NEEDS_RESET | STATUS_FAILED) != 0
    {
        let cause = if driver_status & STATUS_DEVICE_NEEDS_RESET != 0 {
            BlockError::DeviceNeedsReset
        } else {
            BlockError::ProtocolViolation
        };
        return Err(reset_and_release(
            &mut transport,
            Some(queue_frame),
            Some(request_frame),
            clock,
            deadline,
            cause,
        ));
    }

    Ok(ProbeOutcome::Block(VirtioBlock {
        transport,
        queue_frame: Some(queue_frame),
        request_frame: Some(request_frame),
        queue_layout,
        request_layout,
        mode,
        features,
        capacity_sectors,
        state: DriverState::Ready,
        cooperative_recovery: BlockRecoveryTracker::new(),
        cooperative_recovery_cause: None,
        cooperative_recovered_features: None,
        stats: BlockStats::new(),
        requests: RequestTracker::new(),
        #[cfg(bndroid_storage_irq_timeout_profile)]
        suppress_notifications: true,
        #[cfg(any(
            feature = "storage-server-recovery-runtime",
            feature = "app-data-async-recovery-runtime"
        ))]
        recovery_notification_fault: RecoveryNotificationFault::new(),
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        persistent_rebuild_fault_armed: false,
        #[cfg(feature = "storage-server-fault-policy-runtime")]
        persistent_rebuild_fault_hits: 0,
    }))
}

#[cfg(any(
    feature = "storage-server-recovery-runtime",
    feature = "app-data-async-recovery-runtime"
))]
const fn map_recovery_notification_fault_error(
    error: RecoveryNotificationFaultError,
) -> BlockError {
    match error {
        RecoveryNotificationFaultError::KindMismatch => BlockError::RequestKindMismatch,
        RecoveryNotificationFaultError::AlreadyArmed | RecoveryNotificationFaultError::Busy => {
            BlockError::DriverStopped
        }
    }
}

#[cfg(feature = "cooperative-block-recovery")]
const fn map_block_recovery_transition_error(_error: BlockRecoveryTransitionError) -> BlockError {
    BlockError::ProtocolViolation
}

const fn map_tracker_error(error: RequestTrackerError) -> BlockError {
    match error {
        RequestTrackerError::Full => BlockError::QueueFull,
        RequestTrackerError::RequestPending => BlockError::RequestPending,
        RequestTrackerError::StaleToken => BlockError::StaleRequest,
        RequestTrackerError::InvalidDescriptorHead | RequestTrackerError::CompletionNotPending => {
            BlockError::ProtocolViolation
        }
    }
}

fn validate_block_identity(transport: &MmioTransport) -> Result<(), BlockError> {
    let header = transport.header();
    if header.magic != MMIO_MAGIC {
        return Err(BlockError::BadMagic);
    }
    if header.device_id != DEVICE_ID_BLOCK {
        return Err(BlockError::ProtocolViolation);
    }
    if header.version == 1 {
        return Err(BlockError::LegacyTransport);
    }
    if header.version != MMIO_VERSION_MODERN {
        return Err(BlockError::UnsupportedVersion);
    }
    Ok(())
}

const fn map_irq_rearm_error(error: BlockIrqRearmError) -> BlockError {
    match error {
        BlockIrqRearmError::ConfigurationChanged | BlockIrqRearmError::DeviceNeedsReset => {
            BlockError::DeviceNeedsReset
        }
        BlockIrqRearmError::QueueNotEmpty | BlockIrqRearmError::DriverUnhealthy => {
            BlockError::ProtocolViolation
        }
    }
}

fn stable_capacity<C: PollClock + ?Sized>(
    transport: &MmioTransport,
    clock: &mut C,
    deadline: u64,
) -> Result<u64, BlockError> {
    loop {
        let generation = transport.config_generation();
        let capacity = transport.block_capacity_once();
        let observed_generation = transport.config_generation();
        if generation == observed_generation {
            return Ok(capacity);
        }
        if deadline_reached(clock.now(), deadline) {
            return Err(BlockError::CapacityUnstable);
        }
        spin_loop();
    }
}

fn reset_and_wait<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    clock: &mut C,
    deadline: u64,
) -> Result<(), BlockError> {
    transport.write_status(0);
    loop {
        if transport.status() == 0 {
            dma::acquire_from_device();
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return Err(BlockError::ResetTimeout);
        }
        spin_loop();
    }
}

fn fail_active<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    clock: &mut C,
    deadline: u64,
    cause: BlockError,
) -> BlockError {
    transport.add_status(STATUS_FAILED);
    if reset_and_wait(transport, clock, deadline).is_ok() {
        cause
    } else {
        BlockError::ResetTimeout
    }
}

fn reset_and_release<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    queue: Option<OwnedFrame>,
    request: Option<OwnedFrame>,
    clock: &mut C,
    deadline: u64,
    cause: BlockError,
) -> BlockError {
    transport.add_status(STATUS_FAILED);
    if reset_and_wait(transport, clock, deadline).is_err() {
        // OwnedFrame has no reclaiming Drop. Losing the ownership tokens here
        // intentionally leaks allocated pages when reset cannot be confirmed.
        return BlockError::ResetTimeout;
    }

    let queue_error = queue.and_then(|frame| memory::deallocate_frame(frame).err());
    let request_error = request.and_then(|frame| memory::deallocate_frame(frame).err());
    match queue_error.or(request_error) {
        Some(error) => BlockError::FrameRelease(error),
        None => cause,
    }
}

fn release_frames(queue: OwnedFrame, request: OwnedFrame) -> Result<(), BlockError> {
    let queue_error = memory::deallocate_frame(queue).err();
    let request_error = memory::deallocate_frame(request).err();
    match queue_error.or(request_error) {
        Some(error) => Err(BlockError::FrameRelease(error)),
        None => Ok(()),
    }
}
