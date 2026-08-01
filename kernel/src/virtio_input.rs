//! Virtio-input wire constants and deterministic keyboard/tablet validation.

use core::mem::{align_of, size_of};

pub const DEVICE_ID_INPUT: u32 = 18;
pub const EVENT_QUEUE_INDEX: u32 = 0;
pub const STATUS_QUEUE_INDEX: u32 = 1;
pub const EVENT_QUEUE_SIZE: u16 = 8;
pub const EVENT_BYTES: usize = 8;

pub const CONFIG_SELECT_UNSET: u8 = 0x00;
pub const CONFIG_SELECT_ID_NAME: u8 = 0x01;
pub const CONFIG_SELECT_ID_DEVIDS: u8 = 0x03;
pub const CONFIG_SELECT_EV_BITS: u8 = 0x11;
pub const CONFIG_SELECT_ABS_INFO: u8 = 0x12;
pub const CONFIG_BITMAP_BYTES: usize = 128;
pub const ABS_INFO_BYTES: usize = 20;

pub const EVENT_TYPE_SYN: u16 = 0x00;
pub const EVENT_TYPE_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const EVENT_CODE_SYN_REPORT: u16 = 0;
pub const ABS_X: u16 = 0;
pub const ABS_Y: u16 = 1;
pub const KEY_A: u16 = 30;
pub const KEY_ENTER: u16 = 28;
pub const KEY_BACKSPACE: u16 = 14;
pub const BTN_LEFT: u16 = 272;
pub const BTN_TOUCH: u16 = 330;
pub const KEY_VALUE_RELEASE: i32 = 0;
pub const KEY_VALUE_PRESS: i32 = 1;
pub const KEY_VALUE_REPEAT: i32 = 2;
pub const QMP_TOUCH_RAW_X: i32 = 1234;
pub const QMP_TOUCH_RAW_Y: i32 = 23456;
pub const KEY_REPORT_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputDeviceKind {
    Keyboard,
    AbsolutePointer,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub const fn from_wire(raw: Self) -> Self {
        Self {
            event_type: u16::from_le(raw.event_type),
            code: u16::from_le(raw.code),
            value: i32::from_le(raw.value),
        }
    }

    pub const fn to_wire(self) -> Self {
        Self {
            event_type: self.event_type.to_le(),
            code: self.code.to_le(),
            value: self.value.to_le(),
        }
    }

    pub const fn key(code: u16, value: i32) -> Self {
        Self {
            event_type: EVENT_TYPE_KEY,
            code,
            value,
        }
    }

    pub const fn absolute(code: u16, value: i32) -> Self {
        Self {
            event_type: EV_ABS,
            code,
            value,
        }
    }

    pub const fn syn_report() -> Self {
        Self {
            event_type: EVENT_TYPE_SYN,
            code: EVENT_CODE_SYN_REPORT,
            value: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyTransition {
    code: u16,
    value: u8,
}

impl KeyTransition {
    pub const fn code(self) -> u16 {
        self.code
    }

    pub const fn value(self) -> u8 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardReport {
    transitions: [KeyTransition; KEY_REPORT_CAPACITY],
    len: u8,
}

impl KeyboardReport {
    pub fn transitions(&self) -> &[KeyTransition] {
        &self.transitions[..usize::from(self.len)]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardReportError {
    InvalidCode,
    InvalidValue,
    InvalidSyn,
    ReportFull,
}

/// Collects lossless EV_KEY transitions and publishes them only at the next
/// canonical SYN_REPORT boundary. Other event types are irrelevant to the key
/// report and are left available to the existing raw-event monitor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardReportTracker {
    transitions: [KeyTransition; KEY_REPORT_CAPACITY],
    len: u8,
}

impl Default for KeyboardReportTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardReportTracker {
    pub const fn new() -> Self {
        Self {
            transitions: [KeyTransition { code: 0, value: 0 }; KEY_REPORT_CAPACITY],
            len: 0,
        }
    }

    pub const fn pending(self) -> usize {
        self.len as usize
    }

    pub fn observe(
        &mut self,
        event: InputEvent,
    ) -> Result<Option<KeyboardReport>, KeyboardReportError> {
        match event.event_type {
            EVENT_TYPE_KEY => {
                if event.code == 0 {
                    return Err(KeyboardReportError::InvalidCode);
                }
                let value = match event.value {
                    KEY_VALUE_RELEASE => 0,
                    KEY_VALUE_PRESS => 1,
                    KEY_VALUE_REPEAT => 2,
                    _ => return Err(KeyboardReportError::InvalidValue),
                };
                let index = usize::from(self.len);
                if index == KEY_REPORT_CAPACITY {
                    return Err(KeyboardReportError::ReportFull);
                }
                self.transitions[index] = KeyTransition {
                    code: event.code,
                    value,
                };
                self.len += 1;
                Ok(None)
            }
            EVENT_TYPE_SYN => {
                if event.code != EVENT_CODE_SYN_REPORT || event.value != 0 {
                    return Err(KeyboardReportError::InvalidSyn);
                }
                if self.len == 0 {
                    return Ok(None);
                }
                let report = KeyboardReport {
                    transitions: self.transitions,
                    len: self.len,
                };
                self.len = 0;
                Ok(Some(report))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsoluteAxisInfoError {
    InvalidWireLength,
    InvalidRange,
}

/// Virtio-input `virtio_input_absinfo`, represented in host endianness.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbsoluteAxisInfo {
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

impl AbsoluteAxisInfo {
    pub const fn validate(self) -> Result<Self, AbsoluteAxisInfoError> {
        if self.minimum < self.maximum {
            Ok(self)
        } else {
            Err(AbsoluteAxisInfoError::InvalidRange)
        }
    }

    pub const fn from_wire(raw: Self) -> Result<Self, AbsoluteAxisInfoError> {
        Self {
            minimum: i32::from_le(raw.minimum),
            maximum: i32::from_le(raw.maximum),
            fuzz: i32::from_le(raw.fuzz),
            flat: i32::from_le(raw.flat),
            resolution: i32::from_le(raw.resolution),
        }
        .validate()
    }

    pub const fn to_wire(self) -> Self {
        Self {
            minimum: self.minimum.to_le(),
            maximum: self.maximum.to_le(),
            fuzz: self.fuzz.to_le(),
            flat: self.flat.to_le(),
            resolution: self.resolution.to_le(),
        }
    }

    pub fn decode_le(bytes: &[u8]) -> Result<Self, AbsoluteAxisInfoError> {
        if bytes.len() != ABS_INFO_BYTES {
            return Err(AbsoluteAxisInfoError::InvalidWireLength);
        }
        Self {
            minimum: decode_i32_le(bytes, 0),
            maximum: decode_i32_le(bytes, 4),
            fuzz: decode_i32_le(bytes, 8),
            flat: decode_i32_le(bytes, 12),
            resolution: decode_i32_le(bytes, 16),
        }
        .validate()
    }

    pub fn encode_le(self) -> Result<[u8; ABS_INFO_BYTES], AbsoluteAxisInfoError> {
        let valid = self.validate()?;
        let mut bytes = [0_u8; ABS_INFO_BYTES];
        bytes[0..4].copy_from_slice(&valid.minimum.to_le_bytes());
        bytes[4..8].copy_from_slice(&valid.maximum.to_le_bytes());
        bytes[8..12].copy_from_slice(&valid.fuzz.to_le_bytes());
        bytes[12..16].copy_from_slice(&valid.flat.to_le_bytes());
        bytes[16..20].copy_from_slice(&valid.resolution.to_le_bytes());
        Ok(bytes)
    }

    pub const fn contains(self, value: i32) -> bool {
        self.minimum < self.maximum && value >= self.minimum && value <= self.maximum
    }
}

fn decode_i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsoluteMapError {
    InvalidAxis,
    ZeroExtent,
    ValueOutOfRange,
}

pub fn map_absolute_to_pixel(
    value: i32,
    axis: &AbsoluteAxisInfo,
    extent: usize,
) -> Result<usize, AbsoluteMapError> {
    if axis.minimum >= axis.maximum {
        return Err(AbsoluteMapError::InvalidAxis);
    }
    if extent == 0 {
        return Err(AbsoluteMapError::ZeroExtent);
    }
    if !axis.contains(value) {
        return Err(AbsoluteMapError::ValueOutOfRange);
    }

    let offset = (i64::from(value) - i64::from(axis.minimum)) as u128;
    let span = (i64::from(axis.maximum) - i64::from(axis.minimum)) as u128;
    let last_pixel = (extent - 1) as u128;
    Ok(((offset * last_pixel) / span) as usize)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerSample {
    pub raw_x: i32,
    pub raw_y: i32,
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsolutePointerError {
    InvalidAxis,
    ValueOutOfRange,
    InvalidEventValue,
    UnsupportedEvent,
    EmptyReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbsolutePointerTracker {
    x_axis: AbsoluteAxisInfo,
    y_axis: AbsoluteAxisInfo,
    raw_x: Option<i32>,
    raw_y: Option<i32>,
    pressed: bool,
    dirty: bool,
}

impl AbsolutePointerTracker {
    pub fn new(
        x_axis: AbsoluteAxisInfo,
        y_axis: AbsoluteAxisInfo,
    ) -> Result<Self, AbsolutePointerError> {
        let x_axis = x_axis
            .validate()
            .map_err(|_| AbsolutePointerError::InvalidAxis)?;
        let y_axis = y_axis
            .validate()
            .map_err(|_| AbsolutePointerError::InvalidAxis)?;
        Ok(Self {
            x_axis,
            y_axis,
            raw_x: None,
            raw_y: None,
            pressed: false,
            dirty: false,
        })
    }

    pub const fn axes(&self) -> (AbsoluteAxisInfo, AbsoluteAxisInfo) {
        (self.x_axis, self.y_axis)
    }

    pub const fn pending_coordinates(&self) -> (Option<i32>, Option<i32>) {
        (self.raw_x, self.raw_y)
    }

    pub const fn pressed(&self) -> bool {
        self.pressed
    }

    pub const fn has_pending_report(&self) -> bool {
        self.dirty
    }

    pub fn observe(
        &mut self,
        event: InputEvent,
    ) -> Result<Option<PointerSample>, AbsolutePointerError> {
        match event.event_type {
            EV_ABS => {
                let axis = match event.code {
                    ABS_X => self.x_axis,
                    ABS_Y => self.y_axis,
                    _ => return Err(AbsolutePointerError::UnsupportedEvent),
                };
                if !axis.contains(event.value) {
                    return Err(AbsolutePointerError::ValueOutOfRange);
                }
                match event.code {
                    ABS_X => self.raw_x = Some(event.value),
                    ABS_Y => self.raw_y = Some(event.value),
                    _ => unreachable!(),
                }
                self.dirty = true;
                Ok(None)
            }
            EVENT_TYPE_KEY => {
                if event.code != BTN_TOUCH && event.code != BTN_LEFT {
                    return Err(AbsolutePointerError::UnsupportedEvent);
                }
                let pressed = match event.value {
                    KEY_VALUE_RELEASE => false,
                    KEY_VALUE_PRESS => true,
                    _ => return Err(AbsolutePointerError::InvalidEventValue),
                };
                self.pressed = pressed;
                self.dirty = true;
                Ok(None)
            }
            EVENT_TYPE_SYN => {
                if event.code != EVENT_CODE_SYN_REPORT {
                    return Err(AbsolutePointerError::UnsupportedEvent);
                }
                if event.value != 0 {
                    return Err(AbsolutePointerError::InvalidEventValue);
                }
                if !self.dirty {
                    return Err(AbsolutePointerError::EmptyReport);
                }
                self.dirty = false;
                let (Some(raw_x), Some(raw_y)) = (self.raw_x, self.raw_y) else {
                    // Absolute devices may initialize one axis per SYN frame.
                    // Keep the known coordinate as device state and wait for
                    // the other axis instead of treating a legal incremental
                    // report as a fatal protocol error.
                    return Ok(None);
                };
                Ok(Some(PointerSample {
                    raw_x,
                    raw_y,
                    pressed: self.pressed,
                }))
            }
            _ => Err(AbsolutePointerError::UnsupportedEvent),
        }
    }
}

pub const fn bitmap_supports(bitmap: &[u8], code: u16) -> bool {
    let byte = code as usize / 8;
    let bit = code as usize % 8;
    byte < bitmap.len() && bitmap[byte] & (1 << bit) != 0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AKeySequenceError {
    UnexpectedEvent,
    AlreadyComplete,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AKeySequence {
    step: u8,
}

impl AKeySequence {
    pub const fn new() -> Self {
        Self { step: 0 }
    }

    pub const fn step(self) -> u8 {
        self.step
    }

    pub const fn is_complete(self) -> bool {
        self.step == 4
    }

    pub fn observe(&mut self, event: InputEvent) -> Result<bool, AKeySequenceError> {
        if self.is_complete() {
            return Err(AKeySequenceError::AlreadyComplete);
        }
        let expected = match self.step {
            0 => InputEvent::key(KEY_A, KEY_VALUE_PRESS),
            1 => InputEvent::syn_report(),
            2 => InputEvent::key(KEY_A, KEY_VALUE_RELEASE),
            3 => InputEvent::syn_report(),
            _ => return Err(AKeySequenceError::AlreadyComplete),
        };
        if event != expected {
            return Err(AKeySequenceError::UnexpectedEvent);
        }
        self.step += 1;
        Ok(self.is_complete())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchSequenceError {
    UnexpectedEvent,
    AlreadyComplete,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TouchSequence {
    step: u8,
}

impl TouchSequence {
    pub const fn new() -> Self {
        Self { step: 0 }
    }

    pub const fn step(self) -> u8 {
        self.step
    }

    pub const fn complete(self) -> bool {
        self.step == 7
    }

    pub const fn is_complete(self) -> bool {
        self.complete()
    }

    pub fn observe(&mut self, event: InputEvent) -> Result<bool, TouchSequenceError> {
        if self.complete() {
            return Err(TouchSequenceError::AlreadyComplete);
        }
        let expected = match self.step {
            0 => InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X),
            1 => InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y),
            2 => InputEvent::syn_report(),
            3 => InputEvent::key(BTN_TOUCH, KEY_VALUE_PRESS),
            4 => InputEvent::syn_report(),
            5 => InputEvent::key(BTN_TOUCH, KEY_VALUE_RELEASE),
            6 => InputEvent::syn_report(),
            _ => return Err(TouchSequenceError::AlreadyComplete),
        };
        if event != expected {
            return Err(TouchSequenceError::UnexpectedEvent);
        }
        self.step += 1;
        Ok(self.complete())
    }
}

const _: () = assert!(size_of::<InputEvent>() == EVENT_BYTES);
const _: () = assert!(align_of::<InputEvent>() == 4);
const _: () = assert!(size_of::<AbsoluteAxisInfo>() == ABS_INFO_BYTES);
const _: () = assert!(align_of::<AbsoluteAxisInfo>() == 4);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_wire_round_trip_is_little_endian_and_exact_size() {
        let event = InputEvent {
            event_type: 0x1234,
            code: 0x5678,
            value: -0x1234_567,
        };
        assert_eq!(InputEvent::from_wire(event.to_wire()), event);
        assert_eq!(size_of::<InputEvent>(), 8);
        assert_eq!(align_of::<InputEvent>(), 4);
    }

    #[test]
    fn capability_bitmap_checks_bounds_and_lsb_first_codes() {
        let mut bitmap = [0_u8; 4];
        bitmap[KEY_A as usize / 8] |= 1 << (KEY_A as usize % 8);
        assert!(bitmap_supports(&bitmap, KEY_A));
        assert!(!bitmap_supports(&bitmap, KEY_ENTER));
        assert!(!bitmap_supports(&bitmap, 32));
    }

    #[test]
    fn qmp_a_key_down_up_sequence_requires_two_syn_boundaries() {
        let mut sequence = AKeySequence::new();
        for (index, event) in [
            InputEvent::key(KEY_A, KEY_VALUE_PRESS),
            InputEvent::syn_report(),
            InputEvent::key(KEY_A, KEY_VALUE_RELEASE),
            InputEvent::syn_report(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(sequence.observe(event), Ok(index == 3));
        }
        assert!(sequence.is_complete());
        assert_eq!(
            sequence.observe(InputEvent::syn_report()),
            Err(AKeySequenceError::AlreadyComplete)
        );
    }

    #[test]
    fn sequence_rejection_is_transactional() {
        let mut sequence = AKeySequence::new();
        assert_eq!(
            sequence.observe(InputEvent::key(KEY_ENTER, KEY_VALUE_PRESS)),
            Err(AKeySequenceError::UnexpectedEvent)
        );
        assert_eq!(sequence.step(), 0);
        assert_eq!(
            sequence.observe(InputEvent::key(KEY_A, KEY_VALUE_PRESS)),
            Ok(false)
        );
        assert_eq!(sequence.step(), 1);
    }

    #[test]
    fn keyboard_tracker_publishes_lossless_transitions_only_at_syn() {
        let mut tracker = KeyboardReportTracker::new();
        assert_eq!(
            tracker.observe(InputEvent::key(KEY_A, KEY_VALUE_PRESS)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::key(KEY_ENTER, KEY_VALUE_REPEAT)),
            Ok(None)
        );
        assert_eq!(tracker.pending(), 2);
        let report = tracker.observe(InputEvent::syn_report()).unwrap().unwrap();
        assert_eq!(
            report.transitions(),
            &[
                KeyTransition {
                    code: KEY_A,
                    value: 1,
                },
                KeyTransition {
                    code: KEY_ENTER,
                    value: 2,
                },
            ]
        );
        assert_eq!(tracker.pending(), 0);
        assert_eq!(tracker.observe(InputEvent::syn_report()), Ok(None));
    }

    #[test]
    fn keyboard_tracker_rejections_preserve_pending_report() {
        let mut tracker = KeyboardReportTracker::new();
        tracker
            .observe(InputEvent::key(KEY_A, KEY_VALUE_PRESS))
            .unwrap();
        for (event, error) in [
            (
                InputEvent::key(0, KEY_VALUE_PRESS),
                KeyboardReportError::InvalidCode,
            ),
            (
                InputEvent::key(KEY_ENTER, 3),
                KeyboardReportError::InvalidValue,
            ),
            (
                InputEvent {
                    event_type: EVENT_TYPE_SYN,
                    code: EVENT_CODE_SYN_REPORT,
                    value: 1,
                },
                KeyboardReportError::InvalidSyn,
            ),
        ] {
            let before = tracker;
            assert_eq!(tracker.observe(event), Err(error));
            assert_eq!(tracker, before);
        }
        let report = tracker.observe(InputEvent::syn_report()).unwrap().unwrap();
        assert_eq!(report.transitions().len(), 1);
        assert_eq!(report.transitions()[0].code(), KEY_A);
        assert_eq!(report.transitions()[0].value(), 1);
    }

    #[test]
    fn keyboard_tracker_has_a_bounded_transactional_report() {
        let mut tracker = KeyboardReportTracker::new();
        for index in 0..KEY_REPORT_CAPACITY {
            tracker
                .observe(InputEvent::key(KEY_A + index as u16, KEY_VALUE_PRESS))
                .unwrap();
        }
        let before = tracker;
        assert_eq!(
            tracker.observe(InputEvent::key(KEY_ENTER, KEY_VALUE_RELEASE)),
            Err(KeyboardReportError::ReportFull)
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker
                .observe(InputEvent::syn_report())
                .unwrap()
                .unwrap()
                .transitions()
                .len(),
            KEY_REPORT_CAPACITY
        );
    }

    #[test]
    fn absolute_axis_wire_layout_and_little_endian_decode_are_exact() {
        let bytes = [
            0x18, 0xfc, 0xff, 0xff, 0x30, 0x75, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        ];
        let expected = AbsoluteAxisInfo {
            minimum: -1000,
            maximum: 30_000,
            fuzz: 2,
            flat: 3,
            resolution: 4,
        };
        assert_eq!(AbsoluteAxisInfo::decode_le(&bytes), Ok(expected));
        assert_eq!(expected.encode_le(), Ok(bytes));
        assert_eq!(
            AbsoluteAxisInfo::from_wire(expected.to_wire()),
            Ok(expected)
        );
        assert_eq!(size_of::<AbsoluteAxisInfo>(), ABS_INFO_BYTES);
        assert_eq!(align_of::<AbsoluteAxisInfo>(), 4);
    }

    #[test]
    fn absolute_axis_validation_rejects_bad_length_and_non_increasing_range() {
        assert_eq!(
            AbsoluteAxisInfo::decode_le(&[0_u8; ABS_INFO_BYTES - 1]),
            Err(AbsoluteAxisInfoError::InvalidWireLength)
        );
        for (minimum, maximum) in [(5, 5), (6, 5)] {
            assert_eq!(
                AbsoluteAxisInfo {
                    minimum,
                    maximum,
                    ..AbsoluteAxisInfo::default()
                }
                .validate(),
                Err(AbsoluteAxisInfoError::InvalidRange)
            );
        }
    }

    #[test]
    fn absolute_mapping_covers_boundaries_midpoint_and_single_pixel_extent() {
        let axis = AbsoluteAxisInfo {
            minimum: 0,
            maximum: 100,
            ..AbsoluteAxisInfo::default()
        };
        assert_eq!(map_absolute_to_pixel(0, &axis, 11), Ok(0));
        assert_eq!(map_absolute_to_pixel(50, &axis, 11), Ok(5));
        assert_eq!(map_absolute_to_pixel(100, &axis, 11), Ok(10));
        assert_eq!(map_absolute_to_pixel(73, &axis, 1), Ok(0));
    }

    #[test]
    fn absolute_mapping_rejects_invalid_axis_zero_extent_and_out_of_range() {
        let axis = AbsoluteAxisInfo {
            minimum: -10,
            maximum: 10,
            ..AbsoluteAxisInfo::default()
        };
        assert_eq!(
            map_absolute_to_pixel(0, &axis, 0),
            Err(AbsoluteMapError::ZeroExtent)
        );
        assert_eq!(
            map_absolute_to_pixel(11, &axis, 20),
            Err(AbsoluteMapError::ValueOutOfRange)
        );
        assert_eq!(
            map_absolute_to_pixel(
                0,
                &AbsoluteAxisInfo {
                    minimum: 1,
                    maximum: 1,
                    ..AbsoluteAxisInfo::default()
                },
                20
            ),
            Err(AbsoluteMapError::InvalidAxis)
        );
    }

    fn qemu_tablet_tracker() -> AbsolutePointerTracker {
        AbsolutePointerTracker::new(
            AbsoluteAxisInfo {
                minimum: 0,
                maximum: 32_767,
                ..AbsoluteAxisInfo::default()
            },
            AbsoluteAxisInfo {
                minimum: 0,
                maximum: 32_767,
                ..AbsoluteAxisInfo::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn pointer_tracker_publishes_samples_only_at_syn_boundaries() {
        let mut tracker = qemu_tablet_tracker();
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Ok(Some(PointerSample {
                raw_x: QMP_TOUCH_RAW_X,
                raw_y: QMP_TOUCH_RAW_Y,
                pressed: false,
            }))
        );
        assert_eq!(
            tracker.observe(InputEvent::key(BTN_TOUCH, KEY_VALUE_PRESS)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Ok(Some(PointerSample {
                raw_x: QMP_TOUCH_RAW_X,
                raw_y: QMP_TOUCH_RAW_Y,
                pressed: true,
            }))
        );
        assert_eq!(
            tracker.observe(InputEvent::key(BTN_TOUCH, KEY_VALUE_RELEASE)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Ok(Some(PointerSample {
                raw_x: QMP_TOUCH_RAW_X,
                raw_y: QMP_TOUCH_RAW_Y,
                pressed: false,
            }))
        );
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Err(AbsolutePointerError::EmptyReport)
        );
    }

    #[test]
    fn pointer_tracker_accepts_incremental_axis_initialization() {
        let mut tracker = qemu_tablet_tracker();
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X)),
            Ok(None)
        );
        assert_eq!(tracker.observe(InputEvent::syn_report()), Ok(None));
        assert_eq!(tracker.pending_coordinates(), (Some(QMP_TOUCH_RAW_X), None));
        assert!(!tracker.has_pending_report());
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Err(AbsolutePointerError::EmptyReport)
        );
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::syn_report()),
            Ok(Some(PointerSample {
                raw_x: QMP_TOUCH_RAW_X,
                raw_y: QMP_TOUCH_RAW_Y,
                pressed: false,
            }))
        );
    }

    #[test]
    fn pointer_tracker_rejection_is_transactional() {
        let mut tracker = qemu_tablet_tracker();
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_Y, 40_000)),
            Err(AbsolutePointerError::ValueOutOfRange)
        );
        assert_eq!(
            tracker.observe(InputEvent::key(BTN_TOUCH, KEY_VALUE_REPEAT)),
            Err(AbsolutePointerError::InvalidEventValue)
        );
        assert_eq!(tracker.pending_coordinates(), (Some(QMP_TOUCH_RAW_X), None));
        assert!(!tracker.pressed());
        assert!(tracker.has_pending_report());

        assert_eq!(
            tracker.observe(InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y)),
            Ok(None)
        );
        assert!(tracker.observe(InputEvent::syn_report()).unwrap().is_some());
        assert_eq!(
            tracker.observe(InputEvent::key(BTN_TOUCH, KEY_VALUE_PRESS)),
            Ok(None)
        );
        assert_eq!(
            tracker.observe(InputEvent {
                event_type: EVENT_TYPE_SYN,
                code: EVENT_CODE_SYN_REPORT,
                value: 1,
            }),
            Err(AbsolutePointerError::InvalidEventValue)
        );
        assert!(tracker.pressed());
        assert!(tracker.has_pending_report());
        assert_eq!(
            tracker
                .observe(InputEvent::syn_report())
                .unwrap()
                .unwrap()
                .pressed,
            true
        );
    }

    #[test]
    fn pointer_tracker_accepts_btn_left_as_touch_compatibility() {
        let mut tracker = qemu_tablet_tracker();
        tracker
            .observe(InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X))
            .unwrap();
        tracker
            .observe(InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y))
            .unwrap();
        tracker.observe(InputEvent::syn_report()).unwrap();
        tracker
            .observe(InputEvent::key(BTN_LEFT, KEY_VALUE_PRESS))
            .unwrap();
        assert!(
            tracker
                .observe(InputEvent::syn_report())
                .unwrap()
                .unwrap()
                .pressed
        );
    }

    #[test]
    fn qmp_touch_sequence_requires_exact_seven_events() {
        let mut sequence = TouchSequence::new();
        assert_eq!(
            sequence.observe(InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y)),
            Err(TouchSequenceError::UnexpectedEvent)
        );
        assert_eq!(sequence.step(), 0);

        for (index, event) in [
            InputEvent::absolute(ABS_X, QMP_TOUCH_RAW_X),
            InputEvent::absolute(ABS_Y, QMP_TOUCH_RAW_Y),
            InputEvent::syn_report(),
            InputEvent::key(BTN_TOUCH, KEY_VALUE_PRESS),
            InputEvent::syn_report(),
            InputEvent::key(BTN_TOUCH, KEY_VALUE_RELEASE),
            InputEvent::syn_report(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(sequence.observe(event), Ok(index == 6));
        }
        assert!(sequence.complete());
        assert!(sequence.is_complete());
        assert_eq!(
            sequence.observe(InputEvent::syn_report()),
            Err(TouchSequenceError::AlreadyComplete)
        );
    }
}
