//! Bounded, interrupt-driven virtio keyboard and absolute-pointer driver.

use core::ptr::{read_volatile, write_bytes, write_volatile};

use bndroid_kernel::{
    memory::{self, AllocateError, OwnedFrame, PAGE_SIZE},
    time::deadline_reached,
    virtio::{
        DESCRIPTOR_FLAG_WRITE, DEVICE_ID_PLACEHOLDER, Descriptor, FEATURE_VERSION_1, MMIO_MAGIC,
        MMIO_VERSION_MODERN, STATUS_ACKNOWLEDGE, STATUS_DEVICE_NEEDS_RESET, STATUS_DRIVER,
        STATUS_DRIVER_OK, STATUS_FAILED, STATUS_FEATURES_OK, SplitQueueLayout, UsedElement,
    },
    virtio_input::{
        ABS_INFO_BYTES, ABS_X, ABS_Y, AbsoluteAxisInfo, BTN_LEFT, BTN_TOUCH, CONFIG_BITMAP_BYTES,
        CONFIG_SELECT_ABS_INFO, CONFIG_SELECT_EV_BITS, CONFIG_SELECT_ID_NAME, DEVICE_ID_INPUT,
        EV_ABS, EVENT_BYTES, EVENT_QUEUE_INDEX, EVENT_QUEUE_SIZE, EVENT_TYPE_KEY, InputDeviceKind,
        InputEvent, KEY_A, KEY_ENTER, KeyboardReportTracker, STATUS_QUEUE_INDEX, bitmap_supports,
    },
};

use super::{
    block::PollClock,
    mmio::{MmioError, MmioTransport},
};
use crate::arch::aarch64::dma;

const CONFIG_SELECT_OFFSET: usize = 0;
const CONFIG_SUBSELECT_OFFSET: usize = 1;
const CONFIG_SIZE_OFFSET: usize = 2;
const CONFIG_DATA_OFFSET: usize = 8;
const MIN_INPUT_MMIO_BYTES: usize = 0x100 + CONFIG_DATA_OFFSET + CONFIG_BITMAP_BYTES;
const EVENT_BUFFER_ALIGN: usize = core::mem::align_of::<InputEvent>();
const SOFTWARE_EVENT_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    Mmio(MmioError),
    BadMagic,
    LegacyTransport,
    UnsupportedVersion,
    NonCoherentDma,
    ConfigWindowTooSmall,
    MissingVersion1,
    FeaturesRejected,
    DeviceNeedsReset,
    QueueUnavailable,
    StatusQueueUnavailable,
    QueueTooSmall,
    QueueAlreadyReady,
    Allocation(AllocateError),
    ResetTimeout,
    ConfigUnstable,
    InvalidName,
    InvalidCapabilityBitmap,
    UnsupportedCapabilities,
    AmbiguousCapabilities,
    InvalidAbsoluteAxis,
    ProtocolViolation,
    KeyDeliveryFailure,
    ConfigChanged,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    DriverStopped,
}

impl InputError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mmio(error) => error.as_str(),
            Self::BadMagic => "invalid virtio-input MMIO magic",
            Self::LegacyTransport => "legacy virtio-input transport is unsupported",
            Self::UnsupportedVersion => "unsupported virtio-input MMIO version",
            Self::NonCoherentDma => "non-coherent virtio-input DMA is unsupported",
            Self::ConfigWindowTooSmall => {
                "virtio-input MMIO window cannot expose the 128-byte config union"
            }
            Self::MissingVersion1 => "virtio-input does not offer VIRTIO_F_VERSION_1",
            Self::FeaturesRejected => "virtio-input rejected negotiated features",
            Self::DeviceNeedsReset => "virtio-input requested reset",
            Self::QueueUnavailable => "virtio-input event queue is unavailable",
            Self::StatusQueueUnavailable => "virtio-input status queue is unavailable",
            Self::QueueTooSmall => "virtio-input event queue is smaller than eight entries",
            Self::QueueAlreadyReady => "virtio-input event queue is already active",
            Self::Allocation(error) => error.as_str(),
            Self::ResetTimeout => "virtio-input reset did not complete",
            Self::ConfigUnstable => "virtio-input configuration did not stabilize",
            Self::InvalidName => "virtio-input device name is invalid",
            Self::InvalidCapabilityBitmap => "virtio-input capability bitmap is invalid",
            Self::UnsupportedCapabilities => {
                "virtio-input capabilities match neither a keyboard nor an absolute pointer"
            }
            Self::AmbiguousCapabilities => {
                "virtio-input capabilities ambiguously match multiple device roles"
            }
            Self::InvalidAbsoluteAxis => "virtio-input absolute axis metadata is invalid",
            Self::ProtocolViolation => "virtio-input device violated the event queue protocol",
            Self::KeyDeliveryFailure => "virtio-input key report could not reach the display",
            Self::ConfigChanged => "virtio-input configuration changed after activation",
            Self::DriverStopped => "virtio-input driver is stopped",
        }
    }
}

impl From<MmioError> for InputError {
    fn from(error: MmioError) -> Self {
        Self::Mmio(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputDeviceInfo {
    name: [u8; CONFIG_BITMAP_BYTES],
    name_len: u8,
    pub kind: InputDeviceKind,
    pub key_bitmap_bytes: u8,
    pub abs_bitmap_bytes: u8,
    pub supports_a: bool,
    pub supports_enter: bool,
    pub supports_abs_x: bool,
    pub supports_abs_y: bool,
    pub supports_left: bool,
    pub supports_touch: bool,
    pub abs_x: AbsoluteAxisInfo,
    pub abs_y: AbsoluteAxisInfo,
}

impl InputDeviceInfo {
    pub fn name(&self) -> &[u8] {
        &self.name[..usize::from(self.name_len)]
    }

    pub const fn kind(&self) -> InputDeviceKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputStats {
    pub interrupt_notifications: u64,
    pub completions: u64,
    pub recycled: u64,
    pub buffered: u16,
    pub delivered: u64,
    pub dropped: u64,
    pub invalid: u64,
    pub avail_idx: u16,
    pub used_idx: u16,
    pub max_buffered: u16,
    pub key_reports: u64,
    pub key_transitions: u64,
    pub key_surface_rejections: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputInterruptService {
    pub bits: u32,
    pub completions: u16,
}

#[allow(clippy::large_enum_variant)]
pub enum InputProbeOutcome {
    Placeholder,
    OtherDevice(u32),
    Input(VirtioInput),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverState {
    Ready,
    Poisoned,
}

pub struct VirtioInput {
    transport: MmioTransport,
    queue_frame: Option<OwnedFrame>,
    queue_layout: SplitQueueLayout,
    event_buffer_offset: usize,
    descriptor_in_device: [bool; EVENT_QUEUE_SIZE as usize],
    events: [InputEvent; SOFTWARE_EVENT_CAPACITY],
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    event_read: u16,
    event_write: u16,
    state: DriverState,
    info: InputDeviceInfo,
    keyboard_tracker: KeyboardReportTracker,
    stats: InputStats,
}

impl VirtioInput {
    pub const fn info(&self) -> &InputDeviceInfo {
        &self.info
    }

    pub const fn stats(&self) -> InputStats {
        self.stats
    }

    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    pub fn take_event(&mut self) -> Result<Option<InputEvent>, InputError> {
        if self.state != DriverState::Ready {
            return Err(InputError::DriverStopped);
        }
        if self.stats.buffered == 0 {
            return Ok(None);
        }
        let slot = usize::from(self.event_read) % SOFTWARE_EVENT_CAPACITY;
        let event = self.events[slot];
        self.event_read = self.event_read.wrapping_add(1);
        self.stats.buffered -= 1;
        self.stats.delivered = self.stats.delivered.saturating_add(1);
        Ok(Some(event))
    }

    pub fn service_interrupt(&mut self) -> Result<InputInterruptService, InputError> {
        if self.state != DriverState::Ready {
            return Ok(InputInterruptService::default());
        }
        let mut bits = self.transport.interrupt_status() & 0x3;
        if bits != 0 {
            self.transport.acknowledge_interrupt(bits);
        }
        if bits & 2 != 0 {
            self.poison();
            return Err(InputError::ConfigChanged);
        }
        let mut completions = if bits & 1 != 0 {
            self.reap_events()?
        } else {
            0
        };

        let late_bits = self.transport.interrupt_status() & 0x3;
        if late_bits != 0 {
            self.transport.acknowledge_interrupt(late_bits);
            bits |= late_bits;
            if late_bits & 2 != 0 {
                self.poison();
                return Err(InputError::ConfigChanged);
            }
            if late_bits & 1 != 0 {
                completions = completions.saturating_add(self.reap_events()?);
            }
        }
        if bits != 0 {
            self.stats.interrupt_notifications =
                self.stats.interrupt_notifications.saturating_add(1);
        }
        Ok(InputInterruptService { bits, completions })
    }

    fn reap_events(&mut self) -> Result<u16, InputError> {
        let observed_used = self.read_used_index();
        let available = observed_used.wrapping_sub(self.stats.used_idx);
        if available > EVENT_QUEUE_SIZE {
            return self.protocol_violation();
        }
        if available == 0 {
            return Ok(0);
        }
        dma::acquire_from_device();

        let mut completed = 0_u16;
        let mut batch_seen = [false; EVENT_QUEUE_SIZE as usize];
        while completed < available {
            let used_slot = self.stats.used_idx & (EVENT_QUEUE_SIZE - 1);
            let used = self.read_used_element(used_slot);
            let descriptor = match usize::try_from(used.id) {
                Ok(descriptor) => descriptor,
                Err(_) => return self.protocol_violation(),
            };
            if descriptor >= EVENT_QUEUE_SIZE as usize
                || used.length != EVENT_BYTES as u32
                || !self.descriptor_in_device[descriptor]
                || batch_seen[descriptor]
            {
                return self.protocol_violation();
            }
            batch_seen[descriptor] = true;
            self.descriptor_in_device[descriptor] = false;
            let raw = unsafe { read_volatile(self.event_address(descriptor) as *const InputEvent) };
            let event = InputEvent::from_wire(raw);
            if event.event_type == EVENT_TYPE_KEY && !matches!(event.value, 0..=2) {
                return self.protocol_violation();
            }
            self.buffer_event(event);
            if self.info.kind == InputDeviceKind::Keyboard {
                let report = match self.keyboard_tracker.observe(event) {
                    Ok(report) => report,
                    Err(_) => return self.protocol_violation(),
                };
                if let Some(report) = report {
                    self.stats.key_reports = self.stats.key_reports.saturating_add(1);
                    for transition in report.transitions() {
                        self.stats.key_transitions = self.stats.key_transitions.saturating_add(1);
                        #[cfg(feature = "input-server-runtime")]
                        if crate::input_stream::enqueue_key(transition.code(), transition.value())
                            .is_err()
                        {
                            self.poison();
                            return Err(InputError::KeyDeliveryFailure);
                        }
                        #[cfg(not(feature = "input-server-runtime"))]
                        match crate::display::process_key_transition(
                            transition.code(),
                            transition.value(),
                        ) {
                            Ok(crate::display::KeyRouteEvidence::UserspaceRejected { .. }) => {
                                self.stats.key_surface_rejections =
                                    self.stats.key_surface_rejections.saturating_add(1);
                            }
                            Ok(_) => {}
                            Err(_) => {
                                self.poison();
                                return Err(InputError::KeyDeliveryFailure);
                            }
                        }
                    }
                }
            }

            let available_slot = self.stats.avail_idx & (EVENT_QUEUE_SIZE - 1);
            let offset = match self.queue_layout.available_element_offset(available_slot) {
                Some(offset) => offset,
                None => return self.protocol_violation(),
            };
            unsafe {
                write_volatile(
                    (self.queue_address() + offset) as *mut u16,
                    (descriptor as u16).to_le(),
                );
            }
            self.descriptor_in_device[descriptor] = true;
            self.stats.avail_idx = self.stats.avail_idx.wrapping_add(1);
            self.stats.used_idx = self.stats.used_idx.wrapping_add(1);
            self.stats.completions = self.stats.completions.saturating_add(1);
            self.stats.recycled = self.stats.recycled.saturating_add(1);
            completed += 1;
        }
        dma::publish_to_device();
        unsafe {
            write_volatile(
                (self.queue_address() + self.queue_layout.available_index_offset()) as *mut u16,
                self.stats.avail_idx.to_le(),
            );
        }
        dma::publish_to_device();
        self.transport.notify_queue(EVENT_QUEUE_INDEX);
        Ok(completed)
    }

    fn protocol_violation<T>(&mut self) -> Result<T, InputError> {
        self.stats.invalid = self.stats.invalid.saturating_add(1);
        self.poison();
        Err(InputError::ProtocolViolation)
    }

    fn poison(&mut self) {
        // A poisoned queue must stop DMA immediately. Its frame remains owned
        // permanently because reset completion cannot be awaited in hard IRQ.
        self.transport.write_status(0);
        self.state = DriverState::Poisoned;
    }

    fn buffer_event(&mut self, event: InputEvent) {
        if usize::from(self.stats.buffered) == SOFTWARE_EVENT_CAPACITY {
            self.stats.dropped = self.stats.dropped.saturating_add(1);
            return;
        }
        let slot = usize::from(self.event_write) % SOFTWARE_EVENT_CAPACITY;
        self.events[slot] = event;
        self.event_write = self.event_write.wrapping_add(1);
        self.stats.buffered += 1;
        self.stats.max_buffered = self.stats.max_buffered.max(self.stats.buffered);
    }

    fn queue_address(&self) -> usize {
        self.queue_frame
            .as_ref()
            .expect("live input driver owns its queue frame")
            .start_address()
    }

    fn event_address(&self, descriptor: usize) -> usize {
        self.queue_address() + self.event_buffer_offset + descriptor * EVENT_BYTES
    }

    fn read_used_index(&self) -> u16 {
        unsafe {
            u16::from_le(read_volatile(
                (self.queue_address() + self.queue_layout.used_index_offset()) as *const u16,
            ))
        }
    }

    fn read_used_element(&self, slot: u16) -> UsedElement {
        let offset = self
            .queue_layout
            .used_element_offset(slot)
            .expect("masked input used-ring slot");
        let raw = unsafe { read_volatile((self.queue_address() + offset) as *const UsedElement) };
        UsedElement {
            id: u32::from_le(raw.id),
            length: u32::from_le(raw.length),
        }
    }
}

impl Drop for VirtioInput {
    fn drop(&mut self) {
        self.transport.write_status(0);
        self.state = DriverState::Poisoned;
        // OwnedFrame deliberately has no reclaiming Drop. If this live DMA
        // owner escapes installation, leaking is safer than reuse-before-reset.
    }
}

/// # Safety
///
/// The supplied MMIO range must be uniquely owned, identity mapped as Device
/// memory, and `dma_coherent` must truthfully describe the transport.
pub unsafe fn probe<C: PollClock + ?Sized>(
    base: usize,
    size: usize,
    dma_coherent: bool,
    clock: &mut C,
    deadline: u64,
) -> Result<InputProbeOutcome, InputError> {
    let queue_layout = SplitQueueLayout::new(EVENT_QUEUE_SIZE, PAGE_SIZE)
        .map_err(|_| InputError::ProtocolViolation)?;
    let mut transport = unsafe { MmioTransport::new(base, size)? };
    let header = transport.header();
    if header.magic != MMIO_MAGIC {
        return Err(InputError::BadMagic);
    }
    if header.device_id == DEVICE_ID_PLACEHOLDER {
        return Ok(InputProbeOutcome::Placeholder);
    }
    if header.device_id != DEVICE_ID_INPUT {
        return Ok(InputProbeOutcome::OtherDevice(header.device_id));
    }
    if header.version == 1 {
        return Err(InputError::LegacyTransport);
    }
    if header.version != MMIO_VERSION_MODERN {
        return Err(InputError::UnsupportedVersion);
    }
    if !dma_coherent {
        return Err(InputError::NonCoherentDma);
    }
    if size < MIN_INPUT_MMIO_BYTES {
        return Err(InputError::ConfigWindowTooSmall);
    }

    reset_and_wait(&mut transport, clock, deadline)?;
    transport.write_status(STATUS_ACKNOWLEDGE);
    transport.add_status(STATUS_DRIVER);
    let offered = transport.device_features();
    if offered & FEATURE_VERSION_1 == 0 {
        return fail_without_frame(&mut transport, clock, deadline, InputError::MissingVersion1);
    }
    transport.set_driver_features(FEATURE_VERSION_1);
    transport.add_status(STATUS_FEATURES_OK);
    let feature_status = transport.status();
    if feature_status & STATUS_FEATURES_OK == 0 {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::FeaturesRejected,
        );
    }
    if feature_status & STATUS_DEVICE_NEEDS_RESET != 0 {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::DeviceNeedsReset,
        );
    }
    if feature_status & STATUS_FAILED != 0 {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::FeaturesRejected,
        );
    }

    let info = match read_device_info(&mut transport, clock, deadline) {
        Ok(info) => info,
        Err(error) => return fail_without_frame(&mut transport, clock, deadline, error),
    };
    transport.select_queue(STATUS_QUEUE_INDEX);
    if transport.queue_size_max() == 0 {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::StatusQueueUnavailable,
        );
    }
    transport.select_queue(EVENT_QUEUE_INDEX);
    let queue_size_max = transport.queue_size_max();
    if queue_size_max == 0 {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::QueueUnavailable,
        );
    }
    if queue_size_max < u32::from(EVENT_QUEUE_SIZE) {
        return fail_without_frame(&mut transport, clock, deadline, InputError::QueueTooSmall);
    }
    if transport.queue_ready() {
        return fail_without_frame(
            &mut transport,
            clock,
            deadline,
            InputError::QueueAlreadyReady,
        );
    }

    let queue_frame = match memory::allocate_frame() {
        Ok(frame) => frame,
        Err(error) => {
            return fail_without_frame(
                &mut transport,
                clock,
                deadline,
                InputError::Allocation(error),
            );
        }
    };
    let queue_address = queue_frame.start_address();
    let event_buffer_offset = match align_up(queue_layout.total_size, EVENT_BUFFER_ALIGN) {
        Some(offset) => offset,
        None => {
            return fail_with_frame(
                &mut transport,
                queue_frame,
                clock,
                deadline,
                InputError::ProtocolViolation,
            );
        }
    };
    let event_end = match event_buffer_offset.checked_add(EVENT_QUEUE_SIZE as usize * EVENT_BYTES) {
        Some(end) => end,
        None => {
            return fail_with_frame(
                &mut transport,
                queue_frame,
                clock,
                deadline,
                InputError::ProtocolViolation,
            );
        }
    };
    if !queue_address.is_multiple_of(PAGE_SIZE) || event_end > PAGE_SIZE {
        return fail_with_frame(
            &mut transport,
            queue_frame,
            clock,
            deadline,
            InputError::ProtocolViolation,
        );
    }

    unsafe {
        write_bytes(queue_address as *mut u8, 0, PAGE_SIZE);
        for descriptor in 0..EVENT_QUEUE_SIZE {
            let descriptor_offset = queue_layout
                .descriptor_offset(descriptor)
                .expect("eight input descriptors fit the queue");
            write_volatile(
                (queue_address + descriptor_offset) as *mut Descriptor,
                Descriptor {
                    address: ((queue_address
                        + event_buffer_offset
                        + usize::from(descriptor) * EVENT_BYTES)
                        as u64)
                        .to_le(),
                    length: (EVENT_BYTES as u32).to_le(),
                    flags: DESCRIPTOR_FLAG_WRITE.to_le(),
                    next: 0,
                },
            );
            let available_offset = queue_layout
                .available_element_offset(descriptor)
                .expect("eight input available entries fit the queue");
            write_volatile(
                (queue_address + available_offset) as *mut u16,
                descriptor.to_le(),
            );
        }
        write_volatile(
            (queue_address + queue_layout.available_index_offset()) as *mut u16,
            EVENT_QUEUE_SIZE.to_le(),
        );
    }

    let descriptor_address = (queue_address + queue_layout.descriptor_table) as u64;
    let available_address = (queue_address + queue_layout.available_ring) as u64;
    let used_address = (queue_address + queue_layout.used_ring) as u64;
    transport.configure_queue(
        EVENT_QUEUE_SIZE,
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
        return fail_with_frame(
            &mut transport,
            queue_frame,
            clock,
            deadline,
            InputError::ProtocolViolation,
        );
    }
    let mut device = VirtioInput {
        transport,
        queue_frame: Some(queue_frame),
        queue_layout,
        event_buffer_offset,
        descriptor_in_device: [true; EVENT_QUEUE_SIZE as usize],
        events: [InputEvent::default(); SOFTWARE_EVENT_CAPACITY],
        event_read: 0,
        event_write: 0,
        state: DriverState::Ready,
        info,
        keyboard_tracker: KeyboardReportTracker::new(),
        stats: InputStats {
            avail_idx: EVENT_QUEUE_SIZE,
            ..InputStats::default()
        },
    };
    device.transport.notify_queue(EVENT_QUEUE_INDEX);
    if let Err(error) = device.service_interrupt() {
        let frame = device
            .queue_frame
            .take()
            .expect("fresh input driver owns its DMA frame");
        return fail_with_frame(&mut device.transport, frame, clock, deadline, error);
    }

    Ok(InputProbeOutcome::Input(device))
}

fn read_device_info<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    clock: &mut C,
    deadline: u64,
) -> Result<InputDeviceInfo, InputError> {
    let mut name = [0_u8; CONFIG_BITMAP_BYTES];
    let raw_name_len = read_config(
        transport,
        CONFIG_SELECT_ID_NAME,
        0,
        &mut name,
        clock,
        deadline,
    )?;
    let name_len = if raw_name_len > 0 && name[raw_name_len - 1] == 0 {
        raw_name_len - 1
    } else {
        raw_name_len
    };
    if name_len == 0
        || name[..name_len]
            .iter()
            .any(|byte| !(0x20..=0x7e).contains(byte))
        || name[name_len..raw_name_len].iter().any(|byte| *byte != 0)
    {
        return Err(InputError::InvalidName);
    }

    let mut key_bitmap = [0_u8; CONFIG_BITMAP_BYTES];
    let key_bitmap_bytes = read_config(
        transport,
        CONFIG_SELECT_EV_BITS,
        EVENT_TYPE_KEY as u8,
        &mut key_bitmap,
        clock,
        deadline,
    )?;
    if key_bitmap_bytes > CONFIG_BITMAP_BYTES {
        return Err(InputError::InvalidCapabilityBitmap);
    }
    let key_bitmap = &key_bitmap[..key_bitmap_bytes];
    let supports_a = bitmap_supports(key_bitmap, KEY_A);
    let supports_enter = bitmap_supports(key_bitmap, KEY_ENTER);

    let mut abs_bitmap = [0_u8; CONFIG_BITMAP_BYTES];
    let abs_bitmap_bytes = read_config(
        transport,
        CONFIG_SELECT_EV_BITS,
        EV_ABS as u8,
        &mut abs_bitmap,
        clock,
        deadline,
    )?;
    if abs_bitmap_bytes > CONFIG_BITMAP_BYTES {
        return Err(InputError::InvalidCapabilityBitmap);
    }
    let abs_bitmap = &abs_bitmap[..abs_bitmap_bytes];
    let supports_abs_x = bitmap_supports(abs_bitmap, ABS_X);
    let supports_abs_y = bitmap_supports(abs_bitmap, ABS_Y);
    let supports_left = bitmap_supports(key_bitmap, BTN_LEFT);
    let supports_touch = bitmap_supports(key_bitmap, BTN_TOUCH);
    let is_keyboard = supports_a && supports_enter;
    let is_pointer = supports_abs_x && supports_abs_y && supports_touch;
    let kind = match (is_keyboard, is_pointer) {
        (true, false) => InputDeviceKind::Keyboard,
        (false, true) => InputDeviceKind::AbsolutePointer,
        (false, false) => return Err(InputError::UnsupportedCapabilities),
        (true, true) => return Err(InputError::AmbiguousCapabilities),
    };
    let (abs_x, abs_y) = if kind == InputDeviceKind::AbsolutePointer {
        (
            read_absolute_axis(transport, ABS_X, clock, deadline)?,
            read_absolute_axis(transport, ABS_Y, clock, deadline)?,
        )
    } else {
        (AbsoluteAxisInfo::default(), AbsoluteAxisInfo::default())
    };
    Ok(InputDeviceInfo {
        name,
        name_len: name_len as u8,
        kind,
        key_bitmap_bytes: key_bitmap_bytes as u8,
        abs_bitmap_bytes: abs_bitmap_bytes as u8,
        supports_a,
        supports_enter,
        supports_abs_x,
        supports_abs_y,
        supports_left,
        supports_touch,
        abs_x,
        abs_y,
    })
}

fn read_absolute_axis<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    axis: u16,
    clock: &mut C,
    deadline: u64,
) -> Result<AbsoluteAxisInfo, InputError> {
    let mut bytes = [0_u8; ABS_INFO_BYTES];
    let length = read_config(
        transport,
        CONFIG_SELECT_ABS_INFO,
        axis as u8,
        &mut bytes,
        clock,
        deadline,
    )?;
    if length != ABS_INFO_BYTES {
        return Err(InputError::InvalidAbsoluteAxis);
    }
    AbsoluteAxisInfo::decode_le(&bytes).map_err(|_| InputError::InvalidAbsoluteAxis)
}

fn read_config<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    select: u8,
    subselect: u8,
    output: &mut [u8],
    clock: &mut C,
    deadline: u64,
) -> Result<usize, InputError> {
    loop {
        transport.write_config_u8(CONFIG_SELECT_OFFSET, select)?;
        transport.write_config_u8(CONFIG_SUBSELECT_OFFSET, subselect)?;
        let generation = transport.config_generation();
        let length = usize::from(transport.read_config_u8(CONFIG_SIZE_OFFSET)?);
        if length > output.len() {
            return Err(InputError::ProtocolViolation);
        }
        for (index, byte) in output[..length].iter_mut().enumerate() {
            *byte = transport.read_config_u8(CONFIG_DATA_OFFSET + index)?;
        }
        if transport.config_generation() == generation {
            return Ok(length);
        }
        if deadline_reached(clock.now(), deadline) {
            return Err(InputError::ConfigUnstable);
        }
    }
}

fn reset_and_wait<C: PollClock + ?Sized>(
    transport: &mut MmioTransport,
    clock: &mut C,
    deadline: u64,
) -> Result<(), InputError> {
    transport.write_status(0);
    loop {
        if transport.status() == 0 {
            dma::acquire_from_device();
            return Ok(());
        }
        if deadline_reached(clock.now(), deadline) {
            return Err(InputError::ResetTimeout);
        }
        core::hint::spin_loop();
    }
}

fn fail_without_frame<C: PollClock + ?Sized, T>(
    transport: &mut MmioTransport,
    clock: &mut C,
    deadline: u64,
    cause: InputError,
) -> Result<T, InputError> {
    reset_and_wait(transport, clock, deadline)?;
    Err(cause)
}

fn fail_with_frame<C: PollClock + ?Sized, T>(
    transport: &mut MmioTransport,
    frame: OwnedFrame,
    clock: &mut C,
    deadline: u64,
    cause: InputError,
) -> Result<T, InputError> {
    if reset_and_wait(transport, clock, deadline).is_err() {
        return Err(InputError::ResetTimeout);
    }
    memory::deallocate_frame(frame).map_err(|_| InputError::ProtocolViolation)?;
    Err(cause)
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
}
