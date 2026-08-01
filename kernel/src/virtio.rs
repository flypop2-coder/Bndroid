//! Allocation-free protocol helpers shared by the virtio driver and host tests.
//!
//! This module deliberately contains no MMIO or architecture-specific code.
//! The kernel binary owns those unsafe boundaries; keeping wire layouts here
//! makes their sizes, alignments, and bounds testable on the host.

use core::mem::{align_of, size_of};

pub const MMIO_MAGIC: u32 = 0x7472_6976;
pub const MMIO_VERSION_MODERN: u32 = 2;
pub const DEVICE_ID_PLACEHOLDER: u32 = 0;
pub const DEVICE_ID_BLOCK: u32 = 2;

pub const FEATURE_BLOCK_READ_ONLY: u64 = 1 << 5;
pub const FEATURE_BLOCK_FLUSH: u64 = 1 << 9;
pub const FEATURE_VERSION_1: u64 = 1 << 32;
pub const FEATURE_ACCESS_PLATFORM: u64 = 1 << 33;
pub const READ_ONLY_FEATURES: u64 = FEATURE_VERSION_1 | FEATURE_BLOCK_READ_ONLY;
pub const WRITABLE_FEATURES: u64 = FEATURE_VERSION_1 | FEATURE_BLOCK_FLUSH;
// Kept for source compatibility with the original read-only negotiation API.
pub const SUPPORTED_FEATURES: u64 = READ_ONLY_FEATURES;

pub const STATUS_ACKNOWLEDGE: u32 = 1;
pub const STATUS_DRIVER: u32 = 2;
pub const STATUS_DRIVER_OK: u32 = 4;
pub const STATUS_FEATURES_OK: u32 = 8;
pub const STATUS_DEVICE_NEEDS_RESET: u32 = 64;
pub const STATUS_FAILED: u32 = 128;

pub const QUEUE_INDEX: u32 = 0;
pub const QUEUE_SIZE: u16 = 8;
pub const BLOCK_SECTOR_SIZE: usize = 512;
pub const BLOCK_REQUEST_READ: u32 = 0;
pub const BLOCK_REQUEST_WRITE: u32 = 1;
pub const BLOCK_REQUEST_FLUSH: u32 = 4;
pub const BLOCK_STATUS_OK: u8 = 0;
pub const BLOCK_STATUS_IO_ERROR: u8 = 1;
pub const BLOCK_STATUS_UNSUPPORTED: u8 = 2;
pub const BLOCK_STATUS_PENDING: u8 = 0xff;

/// Virtio MMIO interrupt bit 1 reports a device configuration change.
///
/// The bounded block driver cannot re-negotiate a changed capacity or feature
/// set while requests are live, so observing this bit always requires a full
/// reset before another request may be submitted.
pub const fn block_interrupt_requires_recovery(interrupt_bits: u32) -> bool {
    interrupt_bits & 2 != 0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockIrqRearmError {
    ConfigurationChanged,
    QueueNotEmpty,
    DeviceNeedsReset,
    DriverUnhealthy,
}

/// Validates the architecture-independent snapshot taken after a recovery GIC
/// route is enabled and before the logical block IRQ gate is committed.
pub fn validate_block_irq_rearm_snapshot(
    interrupt_bits: u32,
    in_flight: u16,
    available_index: u16,
    consumed_used_index: u16,
    observed_used_indices: [u16; 3],
    status: u32,
) -> Result<(), BlockIrqRearmError> {
    if block_interrupt_requires_recovery(interrupt_bits) {
        return Err(BlockIrqRearmError::ConfigurationChanged);
    }
    if in_flight != 0
        || available_index != 0
        || consumed_used_index != 0
        || observed_used_indices.into_iter().any(|index| index != 0)
    {
        return Err(BlockIrqRearmError::QueueNotEmpty);
    }
    const REQUIRED: u32 =
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK;
    if status & STATUS_DEVICE_NEEDS_RESET != 0 {
        return Err(BlockIrqRearmError::DeviceNeedsReset);
    }
    if status & STATUS_FAILED != 0 || status & REQUIRED != REQUIRED {
        return Err(BlockIrqRearmError::DriverUnhealthy);
    }
    Ok(())
}

/// A paired operation may surface an ordinary terminal status only after both
/// request tokens have left the pending state.
pub const fn request_pair_all_terminal(terminal: [bool; 2]) -> bool {
    terminal[0] && terminal[1]
}

pub const DESCRIPTOR_FLAG_NEXT: u16 = 1;
pub const DESCRIPTOR_FLAG_WRITE: u16 = 2;
pub const AVAILABLE_FLAG_NO_INTERRUPT: u16 = 1;
pub const BLOCK_DESCRIPTOR_COUNT: usize = 3;
pub const BLOCK_REQUEST_SLOT_COUNT: u16 = 2;
pub const BLOCK_READ_WRITTEN_BYTES: u32 = (BLOCK_SECTOR_SIZE + 1) as u32;
pub const BLOCK_STATUS_WRITTEN_BYTES: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockDeviceMode {
    ReadOnly,
    Writable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockRequestKind {
    Read,
    Write,
    Flush,
}

impl BlockRequestKind {
    pub const fn request_type(self) -> u32 {
        match self {
            Self::Read => BLOCK_REQUEST_READ,
            Self::Write => BLOCK_REQUEST_WRITE,
            Self::Flush => BLOCK_REQUEST_FLUSH,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryNotificationFaultError {
    AlreadyArmed,
    Busy,
    KindMismatch,
}

/// Pure one-shot state machine used only by the opt-in storage recovery
/// profiles. Keeping the expected request kind here makes the proof about an
/// actually suppressed, already-published request rather than merely an arm
/// command observed by the monitor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryNotificationFault {
    expected: Option<BlockRequestKind>,
}

impl RecoveryNotificationFault {
    pub const fn new() -> Self {
        Self { expected: None }
    }

    pub const fn is_armed(self) -> bool {
        self.expected.is_some()
    }

    pub fn arm(
        &mut self,
        expected: BlockRequestKind,
        in_flight: u16,
    ) -> Result<(), RecoveryNotificationFaultError> {
        if self.expected.is_some() {
            return Err(RecoveryNotificationFaultError::AlreadyArmed);
        }
        if in_flight != 0 {
            return Err(RecoveryNotificationFaultError::Busy);
        }
        self.expected = Some(expected);
        Ok(())
    }

    pub fn validate_submission(
        self,
        actual: BlockRequestKind,
    ) -> Result<(), RecoveryNotificationFaultError> {
        match self.expected {
            Some(expected) if expected != actual => {
                Err(RecoveryNotificationFaultError::KindMismatch)
            }
            _ => Ok(()),
        }
    }

    pub fn consume_published(
        &mut self,
        actual: BlockRequestKind,
    ) -> Result<bool, RecoveryNotificationFaultError> {
        self.validate_submission(actual)?;
        if self.expected.is_some() {
            self.expected = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn reset(&mut self) {
        self.expected = None;
    }
}

impl Default for RecoveryNotificationFault {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockRecoveryPhase {
    Idle,
    AwaitReset,
    RebuildQueue,
    AwaitStableCapacity,
    AwaitCleanupReset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockRecoveryStart {
    AwaitReset,
    ResetConfirmed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockRecoveryTransitionError {
    AlreadyActive,
    NotActive,
    InvalidPhase,
}

/// Architecture-independent phase ledger for cooperative block recovery.
///
/// The tracker deliberately contains no polling operation. The platform
/// driver performs one bounded MMIO/DMA action, records exactly one phase
/// transition, drops its mutable device borrow, and returns to the monitor.
/// This makes every `Await*` phase an actual scheduling boundary instead of a
/// renamed busy loop inside an IRQ-masked critical section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockRecoveryTracker {
    phase: BlockRecoveryPhase,
    deadline: u64,
    cleanup_budget: u64,
}

impl BlockRecoveryTracker {
    pub const fn new() -> Self {
        Self {
            phase: BlockRecoveryPhase::Idle,
            deadline: 0,
            cleanup_budget: 0,
        }
    }

    pub const fn phase(self) -> BlockRecoveryPhase {
        self.phase
    }

    pub const fn deadline(self) -> u64 {
        self.deadline
    }

    pub const fn cleanup_budget(self) -> u64 {
        self.cleanup_budget
    }

    pub const fn is_active(self) -> bool {
        !matches!(self.phase, BlockRecoveryPhase::Idle)
    }

    pub fn begin(
        &mut self,
        start: BlockRecoveryStart,
        deadline: u64,
        cleanup_budget: u64,
    ) -> Result<(), BlockRecoveryTransitionError> {
        if self.is_active() {
            return Err(BlockRecoveryTransitionError::AlreadyActive);
        }
        self.phase = match start {
            BlockRecoveryStart::AwaitReset => BlockRecoveryPhase::AwaitReset,
            BlockRecoveryStart::ResetConfirmed => BlockRecoveryPhase::RebuildQueue,
        };
        self.deadline = deadline;
        self.cleanup_budget = cleanup_budget;
        Ok(())
    }

    pub fn confirm_reset(&mut self) -> Result<(), BlockRecoveryTransitionError> {
        self.transition(
            BlockRecoveryPhase::AwaitReset,
            BlockRecoveryPhase::RebuildQueue,
        )
    }

    pub fn queue_prepared(&mut self) -> Result<(), BlockRecoveryTransitionError> {
        self.transition(
            BlockRecoveryPhase::RebuildQueue,
            BlockRecoveryPhase::AwaitStableCapacity,
        )
    }

    pub fn begin_cleanup(&mut self, deadline: u64) -> Result<(), BlockRecoveryTransitionError> {
        if !matches!(
            self.phase,
            BlockRecoveryPhase::RebuildQueue | BlockRecoveryPhase::AwaitStableCapacity
        ) {
            return Err(if self.is_active() {
                BlockRecoveryTransitionError::InvalidPhase
            } else {
                BlockRecoveryTransitionError::NotActive
            });
        }
        self.phase = BlockRecoveryPhase::AwaitCleanupReset;
        self.deadline = deadline;
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), BlockRecoveryTransitionError> {
        self.finish(BlockRecoveryPhase::AwaitStableCapacity)
    }

    pub fn confirm_cleanup(&mut self) -> Result<(), BlockRecoveryTransitionError> {
        self.finish(BlockRecoveryPhase::AwaitCleanupReset)
    }

    pub fn abort(&mut self) -> Result<(), BlockRecoveryTransitionError> {
        if !self.is_active() {
            return Err(BlockRecoveryTransitionError::NotActive);
        }
        self.clear();
        Ok(())
    }

    fn transition(
        &mut self,
        expected: BlockRecoveryPhase,
        next: BlockRecoveryPhase,
    ) -> Result<(), BlockRecoveryTransitionError> {
        if self.phase != expected {
            return Err(if self.is_active() {
                BlockRecoveryTransitionError::InvalidPhase
            } else {
                BlockRecoveryTransitionError::NotActive
            });
        }
        self.phase = next;
        Ok(())
    }

    fn finish(&mut self, expected: BlockRecoveryPhase) -> Result<(), BlockRecoveryTransitionError> {
        if self.phase != expected {
            return Err(if self.is_active() {
                BlockRecoveryTransitionError::InvalidPhase
            } else {
                BlockRecoveryTransitionError::NotActive
            });
        }
        self.clear();
        Ok(())
    }

    fn clear(&mut self) {
        self.phase = BlockRecoveryPhase::Idle;
        self.deadline = 0;
        self.cleanup_budget = 0;
    }
}

impl Default for BlockRecoveryTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureError {
    MissingVersion1,
    MissingReadOnly,
    ReadOnlyDevice,
    MissingFlush,
}

impl FeatureError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingVersion1 => "device does not offer VIRTIO_F_VERSION_1",
            Self::MissingReadOnly => "read-only block device does not offer VIRTIO_BLK_F_RO",
            Self::ReadOnlyDevice => "writable block device unexpectedly offers VIRTIO_BLK_F_RO",
            Self::MissingFlush => "writable block device does not offer VIRTIO_BLK_F_FLUSH",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegotiatedFeatures {
    offered: u64,
    selected: u64,
}

impl NegotiatedFeatures {
    pub const fn offered(self) -> u64 {
        self.offered
    }

    pub const fn selected(self) -> u64 {
        self.selected
    }

    pub const fn device_read_only(self) -> bool {
        self.selected & FEATURE_BLOCK_READ_ONLY != 0
    }

    pub const fn flush_supported(self) -> bool {
        self.selected & FEATURE_BLOCK_FLUSH != 0
    }
}

/// Selects the only two features understood by the first block driver.
///
/// Unknown optional feature bits are intentionally left unselected. A modern
/// transport is unusable without `VIRTIO_F_VERSION_1`; a device which requires
/// more semantics can reject `FEATURES_OK`, and the MMIO layer then resets it.
pub const fn negotiate_features(offered: u64) -> Result<NegotiatedFeatures, FeatureError> {
    if offered & FEATURE_VERSION_1 == 0 {
        return Err(FeatureError::MissingVersion1);
    }
    Ok(NegotiatedFeatures {
        offered,
        selected: offered & SUPPORTED_FEATURES,
    })
}

/// Selects the bounded feature set for one explicitly classified block device.
///
/// The M22 timeout/reset self-test keeps a physically read-only device, while
/// the M25 normal path requires a physically writable device with an explicit
/// cache-flush command. Keeping those contracts separate prevents a writable
/// boot from silently degrading to volatile writes and prevents any request
/// from being sent to a device which advertised `VIRTIO_BLK_F_RO`.
pub const fn negotiate_features_for_mode(
    offered: u64,
    mode: BlockDeviceMode,
) -> Result<NegotiatedFeatures, FeatureError> {
    if offered & FEATURE_VERSION_1 == 0 {
        return Err(FeatureError::MissingVersion1);
    }
    match mode {
        BlockDeviceMode::ReadOnly => {
            if offered & FEATURE_BLOCK_READ_ONLY == 0 {
                return Err(FeatureError::MissingReadOnly);
            }
            Ok(NegotiatedFeatures {
                offered,
                selected: offered & READ_ONLY_FEATURES,
            })
        }
        BlockDeviceMode::Writable => {
            if offered & FEATURE_BLOCK_READ_ONLY != 0 {
                return Err(FeatureError::ReadOnlyDevice);
            }
            if offered & FEATURE_BLOCK_FLUSH == 0 {
                return Err(FeatureError::MissingFlush);
            }
            Ok(NegotiatedFeatures {
                offered,
                selected: offered & WRITABLE_FEATURES,
            })
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Descriptor {
    pub address: u64,
    pub length: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UsedElement {
    pub id: u32,
    pub length: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlockRequestHeader {
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    ZeroQueue,
    ZeroRequestSlots,
    QueueNotPowerOfTwo,
    AddressOverflow,
    PageTooSmall,
    DescriptorChainOutOfBounds,
}

impl LayoutError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZeroQueue => "virtqueue size is zero",
            Self::ZeroRequestSlots => "virtio block request slot count is zero",
            Self::QueueNotPowerOfTwo => "virtqueue size is not a power of two",
            Self::AddressOverflow => "virtqueue address overflow",
            Self::PageTooSmall => "virtqueue does not fit in one page",
            Self::DescriptorChainOutOfBounds => "virtio descriptor chain does not fit in the queue",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitQueueLayout {
    pub descriptor_table: usize,
    pub available_ring: usize,
    pub used_ring: usize,
    pub total_size: usize,
    pub queue_size: u16,
}

impl SplitQueueLayout {
    pub const fn new(queue_size: u16, page_size: usize) -> Result<Self, LayoutError> {
        if queue_size == 0 {
            return Err(LayoutError::ZeroQueue);
        }
        if !queue_size.is_power_of_two() {
            return Err(LayoutError::QueueNotPowerOfTwo);
        }

        let descriptor_bytes = match size_of::<Descriptor>().checked_mul(queue_size as usize) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let descriptor_table = 0;
        let available_ring = match align_up(descriptor_bytes, align_of::<u16>()) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        // flags + idx + ring[queue_size]. EVENT_IDX is not negotiated.
        let available_bytes = match size_of::<u16>().checked_mul(2 + queue_size as usize) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let available_end = match available_ring.checked_add(available_bytes) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let used_ring = match align_up(available_end, align_of::<u32>()) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        // flags + idx + ring[queue_size]. EVENT_IDX is not negotiated.
        let used_header = match size_of::<u16>().checked_mul(2) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let used_elements = match size_of::<UsedElement>().checked_mul(queue_size as usize) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let used_bytes = match used_header.checked_add(used_elements) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let total_size = match used_ring.checked_add(used_bytes) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        if total_size > page_size {
            return Err(LayoutError::PageTooSmall);
        }

        Ok(Self {
            descriptor_table,
            available_ring,
            used_ring,
            total_size,
            queue_size,
        })
    }

    pub const fn descriptor_offset(self, index: u16) -> Option<usize> {
        if index >= self.queue_size {
            return None;
        }
        self.descriptor_table
            .checked_add(index as usize * size_of::<Descriptor>())
    }

    pub const fn available_index_offset(self) -> usize {
        self.available_ring + size_of::<u16>()
    }

    pub const fn available_flags_offset(self) -> usize {
        self.available_ring
    }

    pub const fn available_element_offset(self, index: u16) -> Option<usize> {
        if index >= self.queue_size {
            return None;
        }
        self.available_ring
            .checked_add(2 * size_of::<u16>() + index as usize * size_of::<u16>())
    }

    pub const fn used_index_offset(self) -> usize {
        self.used_ring + size_of::<u16>()
    }

    pub const fn used_element_offset(self, index: u16) -> Option<usize> {
        if index >= self.queue_size {
            return None;
        }
        self.used_ring
            .checked_add(2 * size_of::<u16>() + index as usize * size_of::<UsedElement>())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockRequestLayout {
    pub header: usize,
    pub data: usize,
    pub status: usize,
    pub stride: usize,
    pub slot_count: u16,
    pub total_size: usize,
}

impl BlockRequestLayout {
    pub const fn new(page_size: usize) -> Result<Self, LayoutError> {
        Self::new_slots(1, page_size)
    }

    pub const fn new_slots(slot_count: u16, page_size: usize) -> Result<Self, LayoutError> {
        if slot_count == 0 {
            return Err(LayoutError::ZeroRequestSlots);
        }
        let header = 0;
        let data = match align_up(size_of::<BlockRequestHeader>(), align_of::<u64>()) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let status = match data.checked_add(BLOCK_SECTOR_SIZE) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let slot_size = match status.checked_add(size_of::<u8>()) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let stride = match align_up(slot_size, align_of::<u64>()) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let total_size = match stride.checked_mul(slot_count as usize) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        if total_size > page_size {
            return Err(LayoutError::PageTooSmall);
        }
        Ok(Self {
            header,
            data,
            status,
            stride,
            slot_count,
            total_size,
        })
    }

    pub const fn descriptors(self, frame_address: u64) -> Result<[Descriptor; 3], LayoutError> {
        self.descriptors_at(frame_address, 0, BLOCK_DESCRIPTOR_COUNT as u16)
    }

    /// Builds one direct three-descriptor block request chain at `head`.
    ///
    /// Keeping descriptor IDs independent from the request DMA page lets a
    /// single split queue carry multiple requests concurrently while every
    /// returned used element still identifies its owning request slot.
    pub const fn descriptors_at(
        self,
        frame_address: u64,
        head: u16,
        queue_size: u16,
    ) -> Result<[Descriptor; 3], LayoutError> {
        self.descriptors_for_slot(frame_address, 0, head, queue_size)
    }

    pub const fn descriptors_for_slot(
        self,
        frame_address: u64,
        slot: u16,
        head: u16,
        queue_size: u16,
    ) -> Result<[Descriptor; 3], LayoutError> {
        self.descriptors_for_slot_kind(
            frame_address,
            slot,
            head,
            queue_size,
            BlockRequestKind::Read,
        )
    }

    /// Builds the descriptor table entries reserved for one request slot.
    ///
    /// Read and write requests use all three entries. A flush has no data, so
    /// its header skips the middle entry and points directly at the writable
    /// status byte. The unused middle entry is zeroed while the fixed three-ID
    /// slot stride keeps completion heads at 0 and 3.
    pub const fn descriptors_for_slot_kind(
        self,
        frame_address: u64,
        slot: u16,
        head: u16,
        queue_size: u16,
        kind: BlockRequestKind,
    ) -> Result<[Descriptor; 3], LayoutError> {
        if slot >= self.slot_count {
            return Err(LayoutError::DescriptorChainOutOfBounds);
        }
        let Some(data_descriptor) = head.checked_add(1) else {
            return Err(LayoutError::DescriptorChainOutOfBounds);
        };
        let Some(status_descriptor) = head.checked_add(2) else {
            return Err(LayoutError::DescriptorChainOutOfBounds);
        };
        if status_descriptor >= queue_size {
            return Err(LayoutError::DescriptorChainOutOfBounds);
        }
        let slot_offset = match self.stride.checked_mul(slot as usize) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let slot_address = match frame_address.checked_add(slot_offset as u64) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let header_address = match slot_address.checked_add(self.header as u64) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let data_address = match slot_address.checked_add(self.data as u64) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let status_address = match slot_address.checked_add(self.status as u64) {
            Some(value) => value,
            None => return Err(LayoutError::AddressOverflow),
        };
        let (header_next, data_descriptor_entry) = match kind {
            BlockRequestKind::Read => (
                data_descriptor,
                Descriptor {
                    address: data_address,
                    length: BLOCK_SECTOR_SIZE as u32,
                    flags: DESCRIPTOR_FLAG_NEXT | DESCRIPTOR_FLAG_WRITE,
                    next: status_descriptor,
                },
            ),
            BlockRequestKind::Write => (
                data_descriptor,
                Descriptor {
                    address: data_address,
                    length: BLOCK_SECTOR_SIZE as u32,
                    flags: DESCRIPTOR_FLAG_NEXT,
                    next: status_descriptor,
                },
            ),
            BlockRequestKind::Flush => (
                status_descriptor,
                Descriptor {
                    address: 0,
                    length: 0,
                    flags: 0,
                    next: 0,
                },
            ),
        };
        Ok([
            Descriptor {
                address: header_address,
                length: size_of::<BlockRequestHeader>() as u32,
                flags: DESCRIPTOR_FLAG_NEXT,
                next: header_next,
            },
            data_descriptor_entry,
            Descriptor {
                address: status_address,
                length: size_of::<u8>() as u32,
                flags: DESCRIPTOR_FLAG_WRITE,
                next: 0,
            },
        ])
    }
}

/// Validates the device-written byte count in one used-ring element.
///
/// A successful read writes a full sector plus the status byte. A failed read
/// may have written any prefix of the data before its final status. Write and
/// flush requests expose only their one-byte status as device-writable memory.
pub const fn valid_block_used_length(kind: BlockRequestKind, status: u8, used_length: u32) -> bool {
    match kind {
        BlockRequestKind::Read if status == BLOCK_STATUS_OK => {
            used_length == BLOCK_READ_WRITTEN_BYTES
        }
        BlockRequestKind::Read => {
            used_length >= BLOCK_STATUS_WRITTEN_BYTES && used_length <= BLOCK_READ_WRITTEN_BYTES
        }
        BlockRequestKind::Write | BlockRequestKind::Flush => {
            used_length == BLOCK_STATUS_WRITTEN_BYTES
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestToken {
    slot: u16,
    generation: u32,
    kind: BlockRequestKind,
}

impl RequestToken {
    pub const fn slot(self) -> u16 {
        self.slot
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn kind(self) -> BlockRequestKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestTrackerError {
    Full,
    InvalidDescriptorHead,
    CompletionNotPending,
    RequestPending,
    StaleToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrackedState {
    Free,
    Pending,
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrackedSlot {
    generation: u32,
    state: TrackedState,
    status: u8,
    kind: Option<BlockRequestKind>,
}

impl TrackedSlot {
    const EMPTY: Self = Self {
        generation: 0,
        state: TrackedState::Free,
        status: BLOCK_STATUS_PENDING,
        kind: None,
    };
}

/// Allocation-free generation tracker for direct descriptor chains.
///
/// DMA buffers stay owned by the driver; this pure state machine makes slot
/// reuse, out-of-order completions, stale tokens, and exactly-once take
/// semantics independently host-testable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestTracker<const N: usize> {
    slots: [TrackedSlot; N],
    in_flight: u16,
    max_in_flight: u16,
}

impl<const N: usize> RequestTracker<N> {
    pub const fn new() -> Self {
        Self {
            slots: [TrackedSlot::EMPTY; N],
            in_flight: 0,
            max_in_flight: 0,
        }
    }

    pub const fn in_flight(&self) -> u16 {
        self.in_flight
    }

    pub const fn max_in_flight(&self) -> u16 {
        self.max_in_flight
    }

    pub fn free_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.state == TrackedState::Free)
            .count()
    }

    pub fn reserve(&mut self) -> Result<RequestToken, RequestTrackerError> {
        self.reserve_kind(BlockRequestKind::Read)
    }

    pub fn reserve_kind(
        &mut self,
        kind: BlockRequestKind,
    ) -> Result<RequestToken, RequestTrackerError> {
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| slot.state == TrackedState::Free)
        else {
            return Err(RequestTrackerError::Full);
        };
        let generation = self.slots[slot_index].generation.wrapping_add(1).max(1);
        self.slots[slot_index] = TrackedSlot {
            generation,
            state: TrackedState::Pending,
            status: BLOCK_STATUS_PENDING,
            kind: Some(kind),
        };
        self.in_flight = self.in_flight.saturating_add(1);
        self.max_in_flight = self.max_in_flight.max(self.in_flight);
        Ok(RequestToken {
            slot: slot_index as u16,
            generation,
            kind,
        })
    }

    pub const fn slot_for_descriptor_head(
        descriptor_head: u32,
        descriptors_per_request: u16,
    ) -> Result<usize, RequestTrackerError> {
        if descriptors_per_request == 0
            || !descriptor_head.is_multiple_of(descriptors_per_request as u32)
        {
            return Err(RequestTrackerError::InvalidDescriptorHead);
        }
        let slot = descriptor_head / descriptors_per_request as u32;
        if slot >= N as u32 {
            return Err(RequestTrackerError::InvalidDescriptorHead);
        }
        Ok(slot as usize)
    }

    pub fn complete_descriptor_head(
        &mut self,
        descriptor_head: u32,
        descriptors_per_request: u16,
        status: u8,
    ) -> Result<RequestToken, RequestTrackerError> {
        let slot_index = Self::slot_for_descriptor_head(descriptor_head, descriptors_per_request)?;
        let slot = &mut self.slots[slot_index];
        if slot.state != TrackedState::Pending {
            return Err(RequestTrackerError::CompletionNotPending);
        }
        let kind = slot.kind.ok_or(RequestTrackerError::CompletionNotPending)?;
        slot.state = TrackedState::Completed;
        slot.status = status;
        self.in_flight = self
            .in_flight
            .checked_sub(1)
            .ok_or(RequestTrackerError::CompletionNotPending)?;
        Ok(RequestToken {
            slot: slot_index as u16,
            generation: slot.generation,
            kind,
        })
    }

    pub fn pending_kind_for_descriptor_head(
        &self,
        descriptor_head: u32,
        descriptors_per_request: u16,
    ) -> Result<BlockRequestKind, RequestTrackerError> {
        let slot_index = Self::slot_for_descriptor_head(descriptor_head, descriptors_per_request)?;
        let slot = &self.slots[slot_index];
        if slot.state != TrackedState::Pending {
            return Err(RequestTrackerError::CompletionNotPending);
        }
        slot.kind.ok_or(RequestTrackerError::CompletionNotPending)
    }

    pub fn take(&mut self, token: RequestToken) -> Result<u8, RequestTrackerError> {
        let slot_index = usize::from(token.slot);
        if slot_index >= N || self.slots[slot_index].generation != token.generation {
            return Err(RequestTrackerError::StaleToken);
        }
        match self.slots[slot_index].state {
            TrackedState::Pending => Err(RequestTrackerError::RequestPending),
            TrackedState::Free => Err(RequestTrackerError::StaleToken),
            TrackedState::Completed => {
                let status = self.slots[slot_index].status;
                self.slots[slot_index].state = TrackedState::Free;
                self.slots[slot_index].kind = None;
                Ok(status)
            }
        }
    }

    /// Attempts both tokens even when the first has a terminal error status.
    ///
    /// A completed request is freed by `take` regardless of its device status.
    /// Keeping this operation in the host-testable tracker prevents a paired
    /// read from leaking its second slot when the first completion reports an
    /// I/O or unsupported-request failure.
    pub fn take_pair_available(
        &mut self,
        tokens: [RequestToken; 2],
        already_terminal: [bool; 2],
    ) -> [Option<Result<u8, RequestTrackerError>>; 2] {
        let first = (!already_terminal[0]).then(|| self.take(tokens[0]));
        let second = (!already_terminal[1]).then(|| self.take(tokens[1]));
        [first, second]
    }

    pub fn invalidate_all(&mut self) {
        for slot in &mut self.slots {
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.state = TrackedState::Free;
            slot.status = BLOCK_STATUS_PENDING;
            slot.kind = None;
        }
        self.in_flight = 0;
    }
}

impl<const N: usize> Default for RequestTracker<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn sector_in_bounds(capacity_sectors: u64, sector: u64) -> bool {
    sector < capacity_sectors
}

const fn align_up(value: usize, alignment: usize) -> Option<usize> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }
    match value.checked_add(alignment - 1) {
        Some(value) => Some(value & !(alignment - 1)),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, offset_of, size_of};

    use super::{
        BLOCK_DESCRIPTOR_COUNT, BLOCK_READ_WRITTEN_BYTES, BLOCK_REQUEST_FLUSH, BLOCK_REQUEST_READ,
        BLOCK_REQUEST_SLOT_COUNT, BLOCK_REQUEST_WRITE, BLOCK_SECTOR_SIZE, BLOCK_STATUS_IO_ERROR,
        BLOCK_STATUS_OK, BLOCK_STATUS_UNSUPPORTED, BLOCK_STATUS_WRITTEN_BYTES, BlockDeviceMode,
        BlockIrqRearmError, BlockRecoveryPhase, BlockRecoveryStart, BlockRecoveryTracker,
        BlockRecoveryTransitionError, BlockRequestHeader, BlockRequestKind, BlockRequestLayout,
        DESCRIPTOR_FLAG_NEXT, DESCRIPTOR_FLAG_WRITE, Descriptor, FEATURE_ACCESS_PLATFORM,
        FEATURE_BLOCK_FLUSH, FEATURE_BLOCK_READ_ONLY, FEATURE_VERSION_1, FeatureError, LayoutError,
        QUEUE_SIZE, RecoveryNotificationFault, RecoveryNotificationFaultError, RequestTracker,
        RequestTrackerError, STATUS_ACKNOWLEDGE, STATUS_DEVICE_NEEDS_RESET, STATUS_DRIVER,
        STATUS_DRIVER_OK, STATUS_FAILED, STATUS_FEATURES_OK, SplitQueueLayout, UsedElement,
        block_interrupt_requires_recovery, negotiate_features, negotiate_features_for_mode,
        request_pair_all_terminal, sector_in_bounds, valid_block_used_length,
        validate_block_irq_rearm_snapshot,
    };
    use crate::memory::PAGE_SIZE;

    #[test]
    fn wire_structures_have_specified_layout() {
        assert_eq!(size_of::<Descriptor>(), 16);
        assert_eq!(align_of::<Descriptor>(), 8);
        assert_eq!(offset_of!(Descriptor, address), 0);
        assert_eq!(offset_of!(Descriptor, length), 8);
        assert_eq!(offset_of!(Descriptor, flags), 12);
        assert_eq!(offset_of!(Descriptor, next), 14);

        assert_eq!(size_of::<UsedElement>(), 8);
        assert_eq!(size_of::<BlockRequestHeader>(), 16);
        assert_eq!(offset_of!(BlockRequestHeader, sector), 8);
    }

    #[test]
    fn fixed_split_queue_fits_one_page_with_required_alignment() {
        let layout = SplitQueueLayout::new(QUEUE_SIZE, PAGE_SIZE).unwrap();
        assert_eq!(layout.descriptor_table, 0);
        assert_eq!(layout.available_ring, 128);
        assert_eq!(layout.used_ring, 148);
        assert_eq!(layout.total_size, 216);
        assert_eq!(layout.available_ring % 2, 0);
        assert_eq!(layout.used_ring % 4, 0);
        assert_eq!(layout.available_flags_offset(), 128);
        assert_eq!(layout.descriptor_offset(7), Some(112));
        assert_eq!(layout.descriptor_offset(8), None);
        assert_eq!(layout.available_element_offset(7), Some(146));
        assert_eq!(layout.available_element_offset(8), None);
        assert_eq!(layout.used_element_offset(7), Some(208));
        assert_eq!(layout.used_element_offset(8), None);
    }

    #[test]
    fn rejects_invalid_or_oversized_queues() {
        assert_eq!(
            SplitQueueLayout::new(0, PAGE_SIZE),
            Err(LayoutError::ZeroQueue)
        );
        assert_eq!(
            SplitQueueLayout::new(3, PAGE_SIZE),
            Err(LayoutError::QueueNotPowerOfTwo)
        );
        assert_eq!(
            SplitQueueLayout::new(512, PAGE_SIZE),
            Err(LayoutError::PageTooSmall)
        );
    }

    #[test]
    fn request_page_and_descriptor_chain_are_bounded() {
        let layout = BlockRequestLayout::new(PAGE_SIZE).unwrap();
        assert_eq!(layout.header, 0);
        assert_eq!(layout.data, 16);
        assert_eq!(layout.status, 16 + BLOCK_SECTOR_SIZE);
        assert_eq!(layout.stride, 536);
        assert_eq!(layout.slot_count, 1);
        assert_eq!(layout.total_size, 536);

        let descriptors = layout.descriptors(0x4000_0000).unwrap();
        assert_eq!(descriptors.len(), BLOCK_DESCRIPTOR_COUNT);
        assert_eq!(descriptors[0].address, 0x4000_0000);
        assert_eq!(descriptors[0].length, 16);
        assert_eq!(descriptors[0].flags, DESCRIPTOR_FLAG_NEXT);
        assert_eq!(descriptors[0].next, 1);
        assert_eq!(descriptors[1].address, 0x4000_0010);
        assert_eq!(descriptors[1].length, 512);
        assert_eq!(
            descriptors[1].flags,
            DESCRIPTOR_FLAG_NEXT | DESCRIPTOR_FLAG_WRITE
        );
        assert_eq!(descriptors[1].next, 2);
        assert_eq!(descriptors[2].address, 0x4000_0210);
        assert_eq!(descriptors[2].length, 1);
        assert_eq!(descriptors[2].flags, DESCRIPTOR_FLAG_WRITE);
        assert_eq!(descriptors[2].next, 0);
        let two_slots = BlockRequestLayout::new_slots(BLOCK_REQUEST_SLOT_COUNT, PAGE_SIZE).unwrap();
        assert_eq!(two_slots.total_size, 1072);
        let second = two_slots
            .descriptors_for_slot(0x4000_0000, 1, 3, QUEUE_SIZE)
            .unwrap();
        assert_eq!(second[0].address, 0x4000_0218);
        assert_eq!(second[1].address, 0x4000_0228);
        assert_eq!(second[2].address, 0x4000_0428);
        assert_eq!(second[0].next, 4);
        assert_eq!(second[1].next, 5);
        assert_eq!(second[2].next, 0);
        assert_eq!(BLOCK_REQUEST_SLOT_COUNT, 2);
        assert_eq!(
            two_slots.descriptors_for_slot(0x4000_0000, 1, 6, QUEUE_SIZE),
            Err(LayoutError::DescriptorChainOutOfBounds)
        );
        assert_eq!(
            two_slots.descriptors_for_slot(0x4000_0000, 2, 0, QUEUE_SIZE),
            Err(LayoutError::DescriptorChainOutOfBounds)
        );
        assert_eq!(
            BlockRequestLayout::new_slots(0, PAGE_SIZE),
            Err(LayoutError::ZeroRequestSlots)
        );
        assert_eq!(
            layout.descriptors(u64::MAX - 8),
            Err(LayoutError::AddressOverflow)
        );
    }

    #[test]
    fn request_kinds_build_directionally_safe_descriptor_chains() {
        let layout = BlockRequestLayout::new_slots(2, PAGE_SIZE).unwrap();
        let frame = 0x4000_0000;

        let read = layout
            .descriptors_for_slot_kind(frame, 0, 0, QUEUE_SIZE, BlockRequestKind::Read)
            .unwrap();
        assert_eq!(read[0].next, 1);
        assert_eq!(read[1].flags, DESCRIPTOR_FLAG_NEXT | DESCRIPTOR_FLAG_WRITE);
        assert_eq!(read[1].next, 2);
        assert_eq!(read[2].flags, DESCRIPTOR_FLAG_WRITE);

        let write = layout
            .descriptors_for_slot_kind(frame, 1, 3, QUEUE_SIZE, BlockRequestKind::Write)
            .unwrap();
        assert_eq!(write[0].next, 4);
        assert_eq!(write[1].address, 0x4000_0228);
        assert_eq!(write[1].length, BLOCK_SECTOR_SIZE as u32);
        assert_eq!(write[1].flags, DESCRIPTOR_FLAG_NEXT);
        assert_eq!(write[1].next, 5);
        assert_eq!(write[2].address, 0x4000_0428);
        assert_eq!(write[2].flags, DESCRIPTOR_FLAG_WRITE);

        let flush = layout
            .descriptors_for_slot_kind(frame, 1, 3, QUEUE_SIZE, BlockRequestKind::Flush)
            .unwrap();
        assert_eq!(flush[0].next, 5);
        assert_eq!(flush[1], Descriptor::default());
        assert_eq!(flush[2].address, 0x4000_0428);
        assert_eq!(flush[2].length, 1);
        assert_eq!(flush[2].flags, DESCRIPTOR_FLAG_WRITE);
        assert_eq!(flush[2].next, 0);

        assert_eq!(BlockRequestKind::Read.request_type(), BLOCK_REQUEST_READ);
        assert_eq!(BlockRequestKind::Write.request_type(), BLOCK_REQUEST_WRITE);
        assert_eq!(BlockRequestKind::Flush.request_type(), BLOCK_REQUEST_FLUSH);
    }

    #[test]
    fn validates_used_lengths_by_request_direction_and_status() {
        assert_eq!(BLOCK_READ_WRITTEN_BYTES, 513);
        assert_eq!(BLOCK_STATUS_WRITTEN_BYTES, 1);
        assert!(valid_block_used_length(
            BlockRequestKind::Read,
            BLOCK_STATUS_OK,
            513
        ));
        assert!(!valid_block_used_length(
            BlockRequestKind::Read,
            BLOCK_STATUS_OK,
            512
        ));
        for length in [1, 7, 512, 513] {
            assert!(valid_block_used_length(
                BlockRequestKind::Read,
                BLOCK_STATUS_IO_ERROR,
                length
            ));
        }
        assert!(!valid_block_used_length(
            BlockRequestKind::Read,
            BLOCK_STATUS_UNSUPPORTED,
            0
        ));
        assert!(!valid_block_used_length(
            BlockRequestKind::Read,
            BLOCK_STATUS_IO_ERROR,
            514
        ));
        for kind in [BlockRequestKind::Write, BlockRequestKind::Flush] {
            for status in [
                BLOCK_STATUS_OK,
                BLOCK_STATUS_IO_ERROR,
                BLOCK_STATUS_UNSUPPORTED,
            ] {
                assert!(valid_block_used_length(kind, status, 1));
                assert!(!valid_block_used_length(kind, status, 0));
                assert!(!valid_block_used_length(kind, status, 2));
                assert!(!valid_block_used_length(kind, status, 513));
            }
        }
    }

    #[test]
    fn negotiates_only_version_and_optional_read_only() {
        let unsupported = 1 << 61;
        let selected = negotiate_features(
            FEATURE_VERSION_1 | FEATURE_BLOCK_READ_ONLY | FEATURE_ACCESS_PLATFORM | unsupported,
        )
        .unwrap();
        assert_eq!(
            selected.selected(),
            FEATURE_VERSION_1 | FEATURE_BLOCK_READ_ONLY
        );
        assert!(selected.device_read_only());
        assert_ne!(selected.offered() & unsupported, 0);
        assert_eq!(
            negotiate_features(FEATURE_BLOCK_READ_ONLY),
            Err(FeatureError::MissingVersion1)
        );
    }

    #[test]
    fn mode_specific_negotiation_separates_read_only_and_durable_writable_devices() {
        let unsupported = 1 << 61;
        let read_only = negotiate_features_for_mode(
            FEATURE_VERSION_1
                | FEATURE_BLOCK_READ_ONLY
                | FEATURE_BLOCK_FLUSH
                | FEATURE_ACCESS_PLATFORM
                | unsupported,
            BlockDeviceMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(
            read_only.selected(),
            FEATURE_VERSION_1 | FEATURE_BLOCK_READ_ONLY
        );
        assert!(read_only.device_read_only());
        assert!(!read_only.flush_supported());
        assert_eq!(
            negotiate_features_for_mode(FEATURE_VERSION_1, BlockDeviceMode::ReadOnly),
            Err(FeatureError::MissingReadOnly)
        );

        let writable = negotiate_features_for_mode(
            FEATURE_VERSION_1 | FEATURE_BLOCK_FLUSH | FEATURE_ACCESS_PLATFORM | unsupported,
            BlockDeviceMode::Writable,
        )
        .unwrap();
        assert_eq!(writable.selected(), FEATURE_VERSION_1 | FEATURE_BLOCK_FLUSH);
        assert!(!writable.device_read_only());
        assert!(writable.flush_supported());
        assert_eq!(
            negotiate_features_for_mode(
                FEATURE_VERSION_1 | FEATURE_BLOCK_READ_ONLY | FEATURE_BLOCK_FLUSH,
                BlockDeviceMode::Writable,
            ),
            Err(FeatureError::ReadOnlyDevice)
        );
        assert_eq!(
            negotiate_features_for_mode(FEATURE_VERSION_1, BlockDeviceMode::Writable),
            Err(FeatureError::MissingFlush)
        );
        assert_eq!(
            negotiate_features_for_mode(FEATURE_BLOCK_FLUSH, BlockDeviceMode::Writable),
            Err(FeatureError::MissingVersion1)
        );
    }

    #[test]
    fn checks_sector_bounds_without_overflow() {
        assert!(sector_in_bounds(2, 0));
        assert!(sector_in_bounds(2, 1));
        assert!(!sector_in_bounds(2, 2));
        assert!(!sector_in_bounds(0, 0));
        assert!(sector_in_bounds(u64::MAX, u64::MAX - 1));
        assert!(!sector_in_bounds(u64::MAX, u64::MAX));
    }

    #[test]
    fn request_tracker_supports_two_out_of_order_completions() {
        let mut tracker = RequestTracker::<2>::new();
        let first = tracker.reserve().unwrap();
        let second = tracker.reserve().unwrap();
        assert_eq!(first.slot(), 0);
        assert_eq!(second.slot(), 1);
        assert_eq!(tracker.in_flight(), 2);
        assert_eq!(tracker.max_in_flight(), 2);
        assert_eq!(tracker.reserve(), Err(RequestTrackerError::Full));

        assert_eq!(
            tracker.complete_descriptor_head(3, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK),
            Ok(second)
        );
        assert_eq!(
            tracker.take(first),
            Err(RequestTrackerError::RequestPending)
        );
        assert_eq!(tracker.take(second), Ok(BLOCK_STATUS_OK));
        assert_eq!(
            tracker.complete_descriptor_head(
                0,
                BLOCK_DESCRIPTOR_COUNT as u16,
                BLOCK_STATUS_IO_ERROR
            ),
            Ok(first)
        );
        assert_eq!(tracker.take(first), Ok(BLOCK_STATUS_IO_ERROR));
        assert_eq!(tracker.in_flight(), 0);
        assert_eq!(tracker.free_count(), 2);
    }

    #[test]
    fn request_tracker_pair_take_reclaims_terminal_sibling_after_first_failure() {
        let mut tracker = RequestTracker::<2>::new();
        let tokens = [tracker.reserve().unwrap(), tracker.reserve().unwrap()];
        tracker
            .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_IO_ERROR)
            .unwrap();
        tracker
            .complete_descriptor_head(
                BLOCK_DESCRIPTOR_COUNT as u32,
                BLOCK_DESCRIPTOR_COUNT as u16,
                BLOCK_STATUS_OK,
            )
            .unwrap();

        assert_eq!(
            tracker.take_pair_available(tokens, [false, false]),
            [Some(Ok(BLOCK_STATUS_IO_ERROR)), Some(Ok(BLOCK_STATUS_OK)),]
        );
        assert_eq!(tracker.in_flight(), 0);
        assert_eq!(tracker.free_count(), 2);
    }

    #[test]
    fn request_tracker_pair_take_keeps_only_pending_sibling_live() {
        let mut tracker = RequestTracker::<2>::new();
        let tokens = [tracker.reserve().unwrap(), tracker.reserve().unwrap()];
        tracker
            .complete_descriptor_head(
                BLOCK_DESCRIPTOR_COUNT as u32,
                BLOCK_DESCRIPTOR_COUNT as u16,
                BLOCK_STATUS_UNSUPPORTED,
            )
            .unwrap();

        assert_eq!(
            tracker.take_pair_available(tokens, [false, false]),
            [
                Some(Err(RequestTrackerError::RequestPending)),
                Some(Ok(BLOCK_STATUS_UNSUPPORTED)),
            ]
        );
        assert_eq!(tracker.in_flight(), 1);
        assert_eq!(tracker.free_count(), 1);

        tracker
            .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK)
            .unwrap();
        assert_eq!(tracker.take(tokens[0]), Ok(BLOCK_STATUS_OK));
        assert_eq!(tracker.free_count(), 2);
    }

    #[test]
    fn request_tracker_pair_take_skips_already_terminal_token() {
        let mut tracker = RequestTracker::<2>::new();
        let tokens = [tracker.reserve().unwrap(), tracker.reserve().unwrap()];
        tracker
            .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK)
            .unwrap();
        assert_eq!(tracker.take(tokens[0]), Ok(BLOCK_STATUS_OK));
        tracker
            .complete_descriptor_head(
                BLOCK_DESCRIPTOR_COUNT as u32,
                BLOCK_DESCRIPTOR_COUNT as u16,
                BLOCK_STATUS_OK,
            )
            .unwrap();

        assert_eq!(
            tracker.take_pair_available(tokens, [true, false]),
            [None, Some(Ok(BLOCK_STATUS_OK))]
        );
        assert_eq!(tracker.free_count(), 2);
    }

    #[test]
    fn block_config_change_interrupt_always_requires_recovery() {
        assert!(!block_interrupt_requires_recovery(0));
        assert!(!block_interrupt_requires_recovery(1));
        assert!(block_interrupt_requires_recovery(2));
        assert!(block_interrupt_requires_recovery(3));
    }

    #[test]
    fn clean_or_stale_queue_only_rearm_tail_is_accepted() {
        let ready = STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK;
        assert_eq!(
            validate_block_irq_rearm_snapshot(0, 0, 0, 0, [0; 3], ready),
            Ok(())
        );
        assert_eq!(
            validate_block_irq_rearm_snapshot(1, 0, 0, 0, [0; 3], ready),
            Ok(())
        );
    }

    #[test]
    fn config_edges_and_unhealthy_rearm_status_fail_closed() {
        let ready = STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK;
        for bits in [2, 3] {
            assert_eq!(
                validate_block_irq_rearm_snapshot(bits, 0, 0, 0, [0; 3], ready),
                Err(BlockIrqRearmError::ConfigurationChanged)
            );
        }
        assert_eq!(
            validate_block_irq_rearm_snapshot(
                0,
                0,
                0,
                0,
                [0; 3],
                ready | STATUS_DEVICE_NEEDS_RESET,
            ),
            Err(BlockIrqRearmError::DeviceNeedsReset)
        );
        for status in [ready & !STATUS_DRIVER_OK, ready | STATUS_FAILED] {
            assert_eq!(
                validate_block_irq_rearm_snapshot(0, 0, 0, 0, [0; 3], status),
                Err(BlockIrqRearmError::DriverUnhealthy)
            );
        }
    }

    #[test]
    fn every_nonempty_rebuilt_queue_ledger_fails_closed() {
        let ready = STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK;
        for (in_flight, available, consumed, observed) in [
            (1, 0, 0, [0; 3]),
            (0, 1, 0, [0; 3]),
            (0, 0, 1, [0; 3]),
            (0, 0, 0, [1, 0, 0]),
            (0, 0, 0, [0, 1, 0]),
            (0, 0, 0, [0, 0, 1]),
        ] {
            assert_eq!(
                validate_block_irq_rearm_snapshot(
                    1, in_flight, available, consumed, observed, ready,
                ),
                Err(BlockIrqRearmError::QueueNotEmpty)
            );
        }
    }

    #[test]
    fn paired_terminal_error_waits_for_both_tokens() {
        assert!(!request_pair_all_terminal([false, false]));
        assert!(!request_pair_all_terminal([true, false]));
        assert!(!request_pair_all_terminal([false, true]));
        assert!(request_pair_all_terminal([true, true]));
    }

    #[test]
    fn request_tracker_preserves_operation_kind_until_take() {
        let mut tracker = RequestTracker::<2>::new();
        let write = tracker.reserve_kind(BlockRequestKind::Write).unwrap();
        let flush = tracker.reserve_kind(BlockRequestKind::Flush).unwrap();
        assert_eq!(write.kind(), BlockRequestKind::Write);
        assert_eq!(flush.kind(), BlockRequestKind::Flush);
        assert_eq!(
            tracker.pending_kind_for_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16),
            Ok(BlockRequestKind::Write)
        );
        assert_eq!(
            tracker.pending_kind_for_descriptor_head(3, BLOCK_DESCRIPTOR_COUNT as u16),
            Ok(BlockRequestKind::Flush)
        );
        assert_eq!(
            tracker.complete_descriptor_head(3, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK),
            Ok(flush)
        );
        assert_eq!(tracker.take(flush), Ok(BLOCK_STATUS_OK));
        assert_eq!(
            tracker.pending_kind_for_descriptor_head(3, BLOCK_DESCRIPTOR_COUNT as u16),
            Err(RequestTrackerError::CompletionNotPending)
        );
        assert_eq!(
            tracker.complete_descriptor_head(
                0,
                BLOCK_DESCRIPTOR_COUNT as u16,
                BLOCK_STATUS_IO_ERROR,
            ),
            Ok(write)
        );
        assert_eq!(tracker.take(write), Ok(BLOCK_STATUS_IO_ERROR));
        assert_eq!(tracker.free_count(), 2);
    }

    #[test]
    fn request_tracker_rejects_bad_duplicate_and_stale_completions() {
        let mut tracker = RequestTracker::<2>::new();
        let first = tracker.reserve().unwrap();
        assert_eq!(
            tracker.complete_descriptor_head(1, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK),
            Err(RequestTrackerError::InvalidDescriptorHead)
        );
        assert_eq!(
            tracker.complete_descriptor_head(6, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK),
            Err(RequestTrackerError::InvalidDescriptorHead)
        );
        tracker
            .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK)
            .unwrap();
        assert_eq!(
            tracker.complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK),
            Err(RequestTrackerError::CompletionNotPending)
        );
        assert_eq!(tracker.take(first), Ok(BLOCK_STATUS_OK));
        assert_eq!(tracker.take(first), Err(RequestTrackerError::StaleToken));
        let reused = tracker.reserve().unwrap();
        assert_eq!(reused.slot(), first.slot());
        assert_ne!(reused.generation(), first.generation());
        assert_eq!(tracker.take(first), Err(RequestTrackerError::StaleToken));
    }

    #[test]
    fn request_tracker_reset_epoch_rejects_late_work_and_generation_wraps_nonzero() {
        for kind in [
            BlockRequestKind::Read,
            BlockRequestKind::Write,
            BlockRequestKind::Flush,
        ] {
            let mut tracker = RequestTracker::<2>::new();
            let stale = tracker.reserve_kind(kind).unwrap();
            tracker.invalidate_all();
            assert_eq!(tracker.in_flight(), 0);
            assert_eq!(tracker.free_count(), 2);
            assert_eq!(tracker.take(stale), Err(RequestTrackerError::StaleToken));
            assert_eq!(
                tracker.pending_kind_for_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16),
                Err(RequestTrackerError::CompletionNotPending)
            );
            assert_eq!(
                tracker
                    .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK,),
                Err(RequestTrackerError::CompletionNotPending)
            );

            let replacement = tracker.reserve_kind(kind).unwrap();
            assert_eq!(replacement.slot(), stale.slot());
            assert_ne!(replacement.generation(), stale.generation());
            assert_eq!(replacement.kind(), kind);
            assert_eq!(
                tracker
                    .complete_descriptor_head(0, BLOCK_DESCRIPTOR_COUNT as u16, BLOCK_STATUS_OK,),
                Ok(replacement)
            );
            assert_eq!(tracker.take(replacement), Ok(BLOCK_STATUS_OK));
            assert_eq!(tracker.free_count(), 2);
        }

        let mut tracker = RequestTracker::<2>::new();
        tracker.slots[0].generation = u32::MAX;
        let wrapped = tracker.reserve().unwrap();
        assert_eq!(wrapped.generation(), 1);
    }

    #[test]
    fn repeated_recovery_reuses_tracker_without_accepting_any_old_token() {
        let mut tracker = RequestTracker::<2>::new();
        let mut stale = [None; 6];
        for (round, kind) in [
            BlockRequestKind::Write,
            BlockRequestKind::Read,
            BlockRequestKind::Flush,
            BlockRequestKind::Write,
            BlockRequestKind::Read,
            BlockRequestKind::Flush,
        ]
        .into_iter()
        .enumerate()
        {
            let token = tracker.reserve_kind(kind).unwrap();
            stale[round] = Some(token);
            tracker.invalidate_all();
            assert_eq!(tracker.in_flight(), 0);
            assert_eq!(tracker.free_count(), 2);
            for token in stale[..=round].iter().flatten() {
                assert_eq!(tracker.take(*token), Err(RequestTrackerError::StaleToken));
            }
        }
    }

    #[test]
    fn notification_fault_is_typed_one_shot_and_resettable_across_two_cycles() {
        let mut fault = RecoveryNotificationFault::new();
        for kind in [
            BlockRequestKind::Write,
            BlockRequestKind::Read,
            BlockRequestKind::Flush,
            BlockRequestKind::Write,
            BlockRequestKind::Read,
            BlockRequestKind::Flush,
        ] {
            fault.arm(kind, 0).unwrap();
            assert!(fault.is_armed());
            assert_eq!(
                fault.arm(kind, 0),
                Err(RecoveryNotificationFaultError::AlreadyArmed)
            );
            assert_eq!(fault.validate_submission(kind), Ok(()));
            assert_eq!(fault.consume_published(kind), Ok(true));
            assert!(!fault.is_armed());
            assert_eq!(fault.consume_published(kind), Ok(false));
        }

        fault.arm(BlockRequestKind::Read, 0).unwrap();
        assert_eq!(
            fault.validate_submission(BlockRequestKind::Write),
            Err(RecoveryNotificationFaultError::KindMismatch)
        );
        assert!(fault.is_armed());
        fault.reset();
        assert!(!fault.is_armed());
        assert_eq!(
            fault.arm(BlockRequestKind::Flush, 1),
            Err(RecoveryNotificationFaultError::Busy)
        );
    }

    #[test]
    fn cooperative_recovery_requires_every_success_phase_and_clears_deadlines() {
        let mut recovery = BlockRecoveryTracker::new();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::Idle);
        assert!(!recovery.is_active());
        assert_eq!(recovery.deadline(), 0);
        assert_eq!(recovery.cleanup_budget(), 0);
        assert_eq!(
            recovery.confirm_reset(),
            Err(BlockRecoveryTransitionError::NotActive)
        );

        recovery
            .begin(BlockRecoveryStart::AwaitReset, 1_000, 400)
            .unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::AwaitReset);
        assert_eq!(recovery.deadline(), 1_000);
        assert_eq!(recovery.cleanup_budget(), 400);
        assert_eq!(
            recovery.begin(BlockRecoveryStart::AwaitReset, 2_000, 800),
            Err(BlockRecoveryTransitionError::AlreadyActive)
        );
        assert_eq!(
            recovery.queue_prepared(),
            Err(BlockRecoveryTransitionError::InvalidPhase)
        );

        recovery.confirm_reset().unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::RebuildQueue);
        recovery.queue_prepared().unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::AwaitStableCapacity);
        recovery.complete().unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::Idle);
        assert!(!recovery.is_active());
        assert_eq!(recovery.deadline(), 0);
        assert_eq!(recovery.cleanup_budget(), 0);
        assert_eq!(
            recovery.complete(),
            Err(BlockRecoveryTransitionError::NotActive)
        );
    }

    #[test]
    fn cooperative_recovery_cleanup_is_yieldable_and_retryable() {
        let mut recovery = BlockRecoveryTracker::new();
        recovery
            .begin(BlockRecoveryStart::ResetConfirmed, 10_000, 2_000)
            .unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::RebuildQueue);
        recovery.begin_cleanup(12_000).unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::AwaitCleanupReset);
        assert_eq!(recovery.deadline(), 12_000);
        assert_eq!(
            recovery.complete(),
            Err(BlockRecoveryTransitionError::InvalidPhase)
        );
        recovery.confirm_cleanup().unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::Idle);

        recovery
            .begin(BlockRecoveryStart::ResetConfirmed, 20_000, 3_000)
            .unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::RebuildQueue);
        recovery.abort().unwrap();
        assert_eq!(recovery.phase(), BlockRecoveryPhase::Idle);
        assert_eq!(
            recovery.abort(),
            Err(BlockRecoveryTransitionError::NotActive)
        );
    }
}
