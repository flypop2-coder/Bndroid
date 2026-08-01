//! Canonical SurfaceServer/Init input-service recovery control.
//!
//! `BIP1` is deliberately separate from the `BIR1` route-lifecycle wire.
//! The latter is scoped to Init and one live InputServer generation, while
//! this protocol survives an InputServer process replacement on the stable
//! SurfaceServer/Init supervisor channel.

use crate::{DEFAULT_ROUTE_CAPACITY, IdentityError, OwnerPid};

pub const INPUT_SERVICE_RECOVERY_WIRE_SIZE: usize = 64;
pub const INPUT_SERVICE_RECOVERY_MAGIC: u32 = u32::from_le_bytes(*b"BIP1");
pub const INPUT_SERVICE_RECOVERY_VERSION: u16 = 1;

const _: () = assert!(INPUT_SERVICE_RECOVERY_WIRE_SIZE == 64);
const _: () = assert!(DEFAULT_ROUTE_CAPACITY <= u8::MAX as usize);

const OFFSET_MAGIC: usize = 0;
const OFFSET_VERSION: usize = 4;
const OFFSET_KIND: usize = 6;
const OFFSET_HEADER_RESERVED: usize = 7;
const OFFSET_SEQUENCE: usize = 8;
const OFFSET_SURFACE_SESSION: usize = 16;
const OFFSET_ROUTE_EPOCH: usize = 24;
const OFFSET_VALUE_0: usize = 32;
const OFFSET_VALUE_1: usize = 40;
const OFFSET_VALUE_2: usize = 48;
const OFFSET_VALUE_3: usize = 56;

const STATUS_PHASE_SHIFT: u32 = 0;
const STATUS_ROUTE_COUNT_SHIFT: u32 = 8;
const STATUS_FOCUS_SHIFT: u32 = 16;
const STATUS_FLAGS_SHIFT: u32 = 24;
const STATUS_CAPTURE_ACTIVE: u8 = 1 << 0;
const STATUS_TEXT_ACTIVE: u8 = 1 << 1;
const STATUS_FLAG_MASK: u8 = STATUS_CAPTURE_ACTIVE | STATUS_TEXT_ACTIVE;
const STATUS_PACKED_MASK: u64 = u32::MAX as u64;

/// Authenticated authority direction for one BIP1 message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputServiceRecoveryDirection {
    SurfaceToInit,
    InitToSurface,
}

/// Closed BIP1 message vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputServiceRecoveryKind {
    SurfaceStatus = 1,
    RebindOffer = 2,
}

impl InputServiceRecoveryKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::SurfaceStatus),
            2 => Some(Self::RebindOffer),
            _ => None,
        }
    }

    pub const fn direction(self) -> InputServiceRecoveryDirection {
        match self {
            Self::SurfaceStatus => InputServiceRecoveryDirection::SurfaceToInit,
            Self::RebindOffer => InputServiceRecoveryDirection::InitToSurface,
        }
    }
}

/// Closed SurfaceServer view of one InputServer recovery phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputServiceRecoveryPhase {
    Armed = 1,
    RestartRequested = 2,
    RouteLost = 3,
    ResyncPrepared = 4,
    Active = 5,
}

impl InputServiceRecoveryPhase {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Armed),
            2 => Some(Self::RestartRequested),
            3 => Some(Self::RouteLost),
            4 => Some(Self::ResyncPrepared),
            5 => Some(Self::Active),
            _ => None,
        }
    }
}

/// Bounded focus identity carried by the M47 service-recovery witness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputServiceFocus {
    None = 0,
    Launcher = 1,
    App = 2,
}

impl InputServiceFocus {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::Launcher),
            2 => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputServiceRecoveryPayload {
    SurfaceStatus {
        input_pid: OwnerPid,
        input_session_id: u64,
        physical_sequence_floor: u64,
        phase: InputServiceRecoveryPhase,
        route_count: u8,
        focus: InputServiceFocus,
        capture_active: bool,
        text_active: bool,
    },
    RebindOffer {
        old_input_pid: OwnerPid,
        new_input_pid: OwnerPid,
        new_input_session_id: u64,
        physical_sequence_floor: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputServiceRecoveryError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSequence,
    ZeroSurfaceSession,
    ZeroRouteEpoch,
    ZeroInputSession,
    InvalidIdentity(IdentityError),
    SameInputProcess,
    InvalidPhase,
    InvalidFocus,
    InvalidFlags,
    RouteCapacityExceeded,
    FocusWithoutRoute,
    StateWithoutFocus,
}

/// One strict, fixed-size BIP1 message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputServiceRecoveryMessage {
    sequence: u64,
    surface_session: u64,
    route_epoch: u64,
    payload: InputServiceRecoveryPayload,
}

impl InputServiceRecoveryMessage {
    #[allow(clippy::too_many_arguments)]
    pub fn surface_status(
        sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        input_pid: OwnerPid,
        input_session_id: u64,
        physical_sequence_floor: u64,
        phase: InputServiceRecoveryPhase,
        route_count: u8,
        focus: InputServiceFocus,
        capture_active: bool,
        text_active: bool,
    ) -> Result<Self, InputServiceRecoveryError> {
        validate_common(sequence, surface_session, route_epoch)?;
        validate_input_session(input_session_id)?;
        validate_status(route_count, focus, capture_active, text_active)?;
        Ok(Self {
            sequence,
            surface_session,
            route_epoch,
            payload: InputServiceRecoveryPayload::SurfaceStatus {
                input_pid,
                input_session_id,
                physical_sequence_floor,
                phase,
                route_count,
                focus,
                capture_active,
                text_active,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rebind_offer(
        sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        old_input_pid: OwnerPid,
        new_input_pid: OwnerPid,
        new_input_session_id: u64,
        physical_sequence_floor: u64,
    ) -> Result<Self, InputServiceRecoveryError> {
        validate_common(sequence, surface_session, route_epoch)?;
        validate_input_session(new_input_session_id)?;
        if old_input_pid == new_input_pid {
            return Err(InputServiceRecoveryError::SameInputProcess);
        }
        Ok(Self {
            sequence,
            surface_session,
            route_epoch,
            payload: InputServiceRecoveryPayload::RebindOffer {
                old_input_pid,
                new_input_pid,
                new_input_session_id,
                physical_sequence_floor,
            },
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn surface_session(self) -> u64 {
        self.surface_session
    }

    pub const fn route_epoch(self) -> u64 {
        self.route_epoch
    }

    pub const fn payload(self) -> InputServiceRecoveryPayload {
        self.payload
    }

    pub const fn kind(self) -> InputServiceRecoveryKind {
        match self.payload {
            InputServiceRecoveryPayload::SurfaceStatus { .. } => {
                InputServiceRecoveryKind::SurfaceStatus
            }
            InputServiceRecoveryPayload::RebindOffer { .. } => {
                InputServiceRecoveryKind::RebindOffer
            }
        }
    }

    pub const fn direction(self) -> InputServiceRecoveryDirection {
        self.kind().direction()
    }

    pub fn encode(self) -> [u8; INPUT_SERVICE_RECOVERY_WIRE_SIZE] {
        let mut wire = [0_u8; INPUT_SERVICE_RECOVERY_WIRE_SIZE];
        write_u32(&mut wire, OFFSET_MAGIC, INPUT_SERVICE_RECOVERY_MAGIC);
        write_u16(&mut wire, OFFSET_VERSION, INPUT_SERVICE_RECOVERY_VERSION);
        wire[OFFSET_KIND] = self.kind().raw();
        write_u64(&mut wire, OFFSET_SEQUENCE, self.sequence);
        write_u64(&mut wire, OFFSET_SURFACE_SESSION, self.surface_session);
        write_u64(&mut wire, OFFSET_ROUTE_EPOCH, self.route_epoch);
        match self.payload {
            InputServiceRecoveryPayload::SurfaceStatus {
                input_pid,
                input_session_id,
                physical_sequence_floor,
                phase,
                route_count,
                focus,
                capture_active,
                text_active,
            } => {
                write_u64(&mut wire, OFFSET_VALUE_0, input_pid.get());
                write_u64(&mut wire, OFFSET_VALUE_1, input_session_id);
                write_u64(&mut wire, OFFSET_VALUE_2, physical_sequence_floor);
                let flags = (u8::from(capture_active) * STATUS_CAPTURE_ACTIVE)
                    | (u8::from(text_active) * STATUS_TEXT_ACTIVE);
                let packed = u64::from(phase.raw()) << STATUS_PHASE_SHIFT
                    | u64::from(route_count) << STATUS_ROUTE_COUNT_SHIFT
                    | u64::from(focus.raw()) << STATUS_FOCUS_SHIFT
                    | u64::from(flags) << STATUS_FLAGS_SHIFT;
                write_u64(&mut wire, OFFSET_VALUE_3, packed);
            }
            InputServiceRecoveryPayload::RebindOffer {
                old_input_pid,
                new_input_pid,
                new_input_session_id,
                physical_sequence_floor,
            } => {
                write_u64(&mut wire, OFFSET_VALUE_0, old_input_pid.get());
                write_u64(&mut wire, OFFSET_VALUE_1, new_input_pid.get());
                write_u64(&mut wire, OFFSET_VALUE_2, new_input_session_id);
                write_u64(&mut wire, OFFSET_VALUE_3, physical_sequence_floor);
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, InputServiceRecoveryError> {
        if wire.len() != INPUT_SERVICE_RECOVERY_WIRE_SIZE {
            return Err(InputServiceRecoveryError::InvalidWireLength);
        }
        if read_u32(wire, OFFSET_MAGIC) != INPUT_SERVICE_RECOVERY_MAGIC {
            return Err(InputServiceRecoveryError::InvalidMagic);
        }
        if read_u16(wire, OFFSET_VERSION) != INPUT_SERVICE_RECOVERY_VERSION {
            return Err(InputServiceRecoveryError::InvalidVersion);
        }
        let kind = InputServiceRecoveryKind::from_raw(wire[OFFSET_KIND])
            .ok_or(InputServiceRecoveryError::InvalidKind)?;
        if wire[OFFSET_HEADER_RESERVED] != 0 {
            return Err(InputServiceRecoveryError::NonZeroHeaderReserved);
        }
        let sequence = read_u64(wire, OFFSET_SEQUENCE);
        let surface_session = read_u64(wire, OFFSET_SURFACE_SESSION);
        let route_epoch = read_u64(wire, OFFSET_ROUTE_EPOCH);
        validate_common(sequence, surface_session, route_epoch)?;
        let value_0 = read_u64(wire, OFFSET_VALUE_0);
        let value_1 = read_u64(wire, OFFSET_VALUE_1);
        let value_2 = read_u64(wire, OFFSET_VALUE_2);
        let value_3 = read_u64(wire, OFFSET_VALUE_3);
        match kind {
            InputServiceRecoveryKind::SurfaceStatus => {
                if value_3 & !STATUS_PACKED_MASK != 0 {
                    return Err(InputServiceRecoveryError::NonZeroBodyReserved);
                }
                let phase = InputServiceRecoveryPhase::from_raw(
                    ((value_3 >> STATUS_PHASE_SHIFT) & u64::from(u8::MAX)) as u8,
                )
                .ok_or(InputServiceRecoveryError::InvalidPhase)?;
                let route_count =
                    ((value_3 >> STATUS_ROUTE_COUNT_SHIFT) & u64::from(u8::MAX)) as u8;
                let focus = InputServiceFocus::from_raw(
                    ((value_3 >> STATUS_FOCUS_SHIFT) & u64::from(u8::MAX)) as u8,
                )
                .ok_or(InputServiceRecoveryError::InvalidFocus)?;
                let flags = ((value_3 >> STATUS_FLAGS_SHIFT) & u64::from(u8::MAX)) as u8;
                if flags & !STATUS_FLAG_MASK != 0 {
                    return Err(InputServiceRecoveryError::InvalidFlags);
                }
                let input_pid = OwnerPid::try_new(value_0)
                    .map_err(InputServiceRecoveryError::InvalidIdentity)?;
                Self::surface_status(
                    sequence,
                    surface_session,
                    route_epoch,
                    input_pid,
                    value_1,
                    value_2,
                    phase,
                    route_count,
                    focus,
                    flags & STATUS_CAPTURE_ACTIVE != 0,
                    flags & STATUS_TEXT_ACTIVE != 0,
                )
            }
            InputServiceRecoveryKind::RebindOffer => {
                let old_input_pid = OwnerPid::try_new(value_0)
                    .map_err(InputServiceRecoveryError::InvalidIdentity)?;
                let new_input_pid = OwnerPid::try_new(value_1)
                    .map_err(InputServiceRecoveryError::InvalidIdentity)?;
                Self::rebind_offer(
                    sequence,
                    surface_session,
                    route_epoch,
                    old_input_pid,
                    new_input_pid,
                    value_2,
                    value_3,
                )
            }
        }
    }
}

fn validate_common(
    sequence: u64,
    surface_session: u64,
    route_epoch: u64,
) -> Result<(), InputServiceRecoveryError> {
    if sequence == 0 {
        return Err(InputServiceRecoveryError::ZeroSequence);
    }
    if surface_session == 0 {
        return Err(InputServiceRecoveryError::ZeroSurfaceSession);
    }
    if route_epoch == 0 {
        return Err(InputServiceRecoveryError::ZeroRouteEpoch);
    }
    Ok(())
}

fn validate_input_session(input_session_id: u64) -> Result<(), InputServiceRecoveryError> {
    if input_session_id == 0 {
        return Err(InputServiceRecoveryError::ZeroInputSession);
    }
    Ok(())
}

fn validate_status(
    route_count: u8,
    focus: InputServiceFocus,
    capture_active: bool,
    text_active: bool,
) -> Result<(), InputServiceRecoveryError> {
    if usize::from(route_count) > DEFAULT_ROUTE_CAPACITY {
        return Err(InputServiceRecoveryError::RouteCapacityExceeded);
    }
    if focus != InputServiceFocus::None && route_count == 0 {
        return Err(InputServiceRecoveryError::FocusWithoutRoute);
    }
    if (capture_active || text_active) && focus == InputServiceFocus::None {
        return Err(InputServiceRecoveryError::StateWithoutFocus);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputServiceRecoveryTrackerError {
    SequenceReplay {
        direction: InputServiceRecoveryDirection,
        last: u64,
    },
    SequenceGap {
        direction: InputServiceRecoveryDirection,
        expected: u64,
    },
    SequenceExhausted {
        direction: InputServiceRecoveryDirection,
    },
}

/// Independent contiguous receive cursors for the two BIP1 directions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputServiceRecoverySequenceTracker {
    surface_to_init_last: u64,
    init_to_surface_last: u64,
}

impl InputServiceRecoverySequenceTracker {
    pub const fn new() -> Self {
        Self {
            surface_to_init_last: 0,
            init_to_surface_last: 0,
        }
    }

    pub fn accept(
        &mut self,
        message: InputServiceRecoveryMessage,
    ) -> Result<(), InputServiceRecoveryTrackerError> {
        let direction = message.direction();
        let last = self.last_sequence(direction);
        let Some(expected) = last.checked_add(1) else {
            return Err(InputServiceRecoveryTrackerError::SequenceExhausted { direction });
        };
        if message.sequence <= last {
            return Err(InputServiceRecoveryTrackerError::SequenceReplay { direction, last });
        }
        if message.sequence != expected {
            return Err(InputServiceRecoveryTrackerError::SequenceGap {
                direction,
                expected,
            });
        }
        match direction {
            InputServiceRecoveryDirection::SurfaceToInit => {
                self.surface_to_init_last = message.sequence;
            }
            InputServiceRecoveryDirection::InitToSurface => {
                self.init_to_surface_last = message.sequence;
            }
        }
        Ok(())
    }

    pub const fn last_sequence(self, direction: InputServiceRecoveryDirection) -> u64 {
        match direction {
            InputServiceRecoveryDirection::SurfaceToInit => self.surface_to_init_last,
            InputServiceRecoveryDirection::InitToSurface => self.init_to_surface_last,
        }
    }
}

fn write_u16(wire: &mut [u8; 64], offset: usize, value: u16) {
    wire[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(wire: &mut [u8; 64], offset: usize, value: u32) {
    wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(wire: &mut [u8; 64], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u16(wire: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(wire[offset..offset + 2].try_into().expect("two bytes"))
}

fn read_u32(wire: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(wire[offset..offset + 4].try_into().expect("four bytes"))
}

fn read_u64(wire: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(wire[offset..offset + 8].try_into().expect("eight bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(raw: u64) -> OwnerPid {
        OwnerPid::try_new(raw).unwrap()
    }

    fn status(sequence: u64, phase: InputServiceRecoveryPhase) -> InputServiceRecoveryMessage {
        InputServiceRecoveryMessage::surface_status(
            sequence,
            1,
            1,
            pid(0x1_0000_0006),
            7,
            9,
            phase,
            2,
            InputServiceFocus::App,
            true,
            false,
        )
        .unwrap()
    }

    fn offer(sequence: u64) -> InputServiceRecoveryMessage {
        InputServiceRecoveryMessage::rebind_offer(
            sequence,
            1,
            2,
            pid(0x1_0000_0006),
            pid(0x2_0000_0006),
            8,
            9,
        )
        .unwrap()
    }

    #[test]
    fn wire_constants_and_common_offsets_are_exact() {
        assert_eq!(INPUT_SERVICE_RECOVERY_WIRE_SIZE, 64);
        assert_eq!(INPUT_SERVICE_RECOVERY_MAGIC.to_le_bytes(), *b"BIP1");
        assert_eq!(INPUT_SERVICE_RECOVERY_VERSION, 1);
        let wire = status(0x1112_1314_1516_1718, InputServiceRecoveryPhase::Armed).encode();
        assert_eq!(&wire[..4], b"BIP1");
        assert_eq!(read_u16(&wire, 4), 1);
        assert_eq!(wire[6], InputServiceRecoveryKind::SurfaceStatus.raw());
        assert_eq!(wire[7], 0);
        assert_eq!(read_u64(&wire, 8), 0x1112_1314_1516_1718);
        assert_eq!(read_u64(&wire, 16), 1);
        assert_eq!(read_u64(&wire, 24), 1);
    }

    #[test]
    fn all_surface_phases_and_rebind_offer_round_trip_canonically() {
        let messages = [
            status(1, InputServiceRecoveryPhase::Armed),
            status(2, InputServiceRecoveryPhase::RestartRequested),
            status(3, InputServiceRecoveryPhase::RouteLost),
            status(4, InputServiceRecoveryPhase::ResyncPrepared),
            status(5, InputServiceRecoveryPhase::Active),
            offer(1),
        ];
        for message in messages {
            let wire = message.encode();
            assert_eq!(InputServiceRecoveryMessage::decode(&wire), Ok(message));
            assert_eq!(
                InputServiceRecoveryMessage::decode(&wire).unwrap().encode(),
                wire
            );
        }
    }

    #[test]
    fn status_packing_and_directions_are_exact() {
        let message = status(1, InputServiceRecoveryPhase::RouteLost);
        let wire = message.encode();
        assert_eq!(read_u64(&wire, 32), 0x1_0000_0006);
        assert_eq!(read_u64(&wire, 40), 7);
        assert_eq!(read_u64(&wire, 48), 9);
        assert_eq!(read_u64(&wire, 56), 3 | (2 << 8) | (2 << 16) | (1 << 24));
        assert_eq!(
            message.direction(),
            InputServiceRecoveryDirection::SurfaceToInit
        );
        assert_eq!(
            offer(1).direction(),
            InputServiceRecoveryDirection::InitToSurface
        );
    }

    #[test]
    fn header_length_magic_version_kind_and_reserved_are_strict() {
        let canonical = status(1, InputServiceRecoveryPhase::Armed).encode();
        assert_eq!(
            InputServiceRecoveryMessage::decode(&canonical[..63]),
            Err(InputServiceRecoveryError::InvalidWireLength)
        );
        let mut oversized = [0_u8; 65];
        oversized[..64].copy_from_slice(&canonical);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&oversized),
            Err(InputServiceRecoveryError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, InputServiceRecoveryError::InvalidMagic),
            (4, InputServiceRecoveryError::InvalidVersion),
            (6, InputServiceRecoveryError::InvalidKind),
            (7, InputServiceRecoveryError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(InputServiceRecoveryMessage::decode(&wire), Err(expected));
        }
    }

    #[test]
    fn common_zero_fields_and_input_identities_are_rejected() {
        assert_eq!(
            InputServiceRecoveryMessage::surface_status(
                0,
                1,
                1,
                pid(1),
                1,
                0,
                InputServiceRecoveryPhase::Armed,
                0,
                InputServiceFocus::None,
                false,
                false,
            ),
            Err(InputServiceRecoveryError::ZeroSequence)
        );
        let mut wire = status(1, InputServiceRecoveryPhase::Armed).encode();
        write_u64(&mut wire, OFFSET_SURFACE_SESSION, 0);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&wire),
            Err(InputServiceRecoveryError::ZeroSurfaceSession)
        );
        let mut wire = status(1, InputServiceRecoveryPhase::Armed).encode();
        write_u64(&mut wire, OFFSET_ROUTE_EPOCH, 0);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&wire),
            Err(InputServiceRecoveryError::ZeroRouteEpoch)
        );
        let mut wire = status(1, InputServiceRecoveryPhase::Armed).encode();
        write_u64(&mut wire, OFFSET_VALUE_0, 0);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&wire),
            Err(InputServiceRecoveryError::InvalidIdentity(
                IdentityError::ZeroOwnerPid
            ))
        );
        let mut wire = status(1, InputServiceRecoveryPhase::Armed).encode();
        write_u64(&mut wire, OFFSET_VALUE_1, 0);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&wire),
            Err(InputServiceRecoveryError::ZeroInputSession)
        );
    }

    #[test]
    fn status_closed_fields_capacity_and_focus_state_are_strict() {
        let canonical = status(1, InputServiceRecoveryPhase::Armed).encode();
        let mut invalid_phase = canonical;
        invalid_phase[OFFSET_VALUE_3] = 0xff;
        assert_eq!(
            InputServiceRecoveryMessage::decode(&invalid_phase),
            Err(InputServiceRecoveryError::InvalidPhase)
        );
        let mut invalid_focus = canonical;
        invalid_focus[OFFSET_VALUE_3 + 2] = 0xff;
        assert_eq!(
            InputServiceRecoveryMessage::decode(&invalid_focus),
            Err(InputServiceRecoveryError::InvalidFocus)
        );
        let mut invalid_flags = canonical;
        invalid_flags[OFFSET_VALUE_3 + 3] = 0x80;
        assert_eq!(
            InputServiceRecoveryMessage::decode(&invalid_flags),
            Err(InputServiceRecoveryError::InvalidFlags)
        );
        let mut reserved = canonical;
        reserved[OFFSET_VALUE_3 + 4] = 1;
        assert_eq!(
            InputServiceRecoveryMessage::decode(&reserved),
            Err(InputServiceRecoveryError::NonZeroBodyReserved)
        );
        assert_eq!(
            InputServiceRecoveryMessage::surface_status(
                1,
                1,
                1,
                pid(1),
                1,
                0,
                InputServiceRecoveryPhase::Armed,
                DEFAULT_ROUTE_CAPACITY as u8 + 1,
                InputServiceFocus::None,
                false,
                false,
            ),
            Err(InputServiceRecoveryError::RouteCapacityExceeded)
        );
        assert_eq!(
            InputServiceRecoveryMessage::surface_status(
                1,
                1,
                1,
                pid(1),
                1,
                0,
                InputServiceRecoveryPhase::Armed,
                0,
                InputServiceFocus::App,
                false,
                false,
            ),
            Err(InputServiceRecoveryError::FocusWithoutRoute)
        );
        assert_eq!(
            InputServiceRecoveryMessage::surface_status(
                1,
                1,
                1,
                pid(1),
                1,
                0,
                InputServiceRecoveryPhase::Armed,
                2,
                InputServiceFocus::None,
                true,
                false,
            ),
            Err(InputServiceRecoveryError::StateWithoutFocus)
        );
    }

    #[test]
    fn rebind_offer_requires_distinct_nonzero_processes_and_session() {
        assert_eq!(
            InputServiceRecoveryMessage::rebind_offer(1, 1, 2, pid(7), pid(7), 2, 0),
            Err(InputServiceRecoveryError::SameInputProcess)
        );
        assert_eq!(
            InputServiceRecoveryMessage::rebind_offer(1, 1, 2, pid(7), pid(8), 0, 0),
            Err(InputServiceRecoveryError::ZeroInputSession)
        );
        let mut wire = offer(1).encode();
        write_u64(&mut wire, OFFSET_VALUE_1, 0);
        assert_eq!(
            InputServiceRecoveryMessage::decode(&wire),
            Err(InputServiceRecoveryError::InvalidIdentity(
                IdentityError::ZeroOwnerPid
            ))
        );
    }

    #[test]
    fn sequence_tracker_orders_directions_independently() {
        let mut tracker = InputServiceRecoverySequenceTracker::new();
        tracker
            .accept(status(1, InputServiceRecoveryPhase::Armed))
            .unwrap();
        tracker.accept(offer(1)).unwrap();
        tracker
            .accept(status(2, InputServiceRecoveryPhase::RouteLost))
            .unwrap();
        assert_eq!(
            tracker.last_sequence(InputServiceRecoveryDirection::SurfaceToInit),
            2
        );
        assert_eq!(
            tracker.last_sequence(InputServiceRecoveryDirection::InitToSurface),
            1
        );
    }

    #[test]
    fn sequence_tracker_failures_are_transactional() {
        let mut tracker = InputServiceRecoverySequenceTracker::new();
        tracker
            .accept(status(1, InputServiceRecoveryPhase::Armed))
            .unwrap();
        let before = tracker;
        assert_eq!(
            tracker.accept(status(1, InputServiceRecoveryPhase::RouteLost)),
            Err(InputServiceRecoveryTrackerError::SequenceReplay {
                direction: InputServiceRecoveryDirection::SurfaceToInit,
                last: 1,
            })
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker.accept(status(3, InputServiceRecoveryPhase::RouteLost)),
            Err(InputServiceRecoveryTrackerError::SequenceGap {
                direction: InputServiceRecoveryDirection::SurfaceToInit,
                expected: 2,
            })
        );
        assert_eq!(tracker, before);
    }

    #[test]
    fn sequence_tracker_exhaustion_is_transactional() {
        let mut tracker = InputServiceRecoverySequenceTracker {
            surface_to_init_last: u64::MAX,
            init_to_surface_last: 0,
        };
        let before = tracker;
        assert_eq!(
            tracker.accept(status(1, InputServiceRecoveryPhase::Armed)),
            Err(InputServiceRecoveryTrackerError::SequenceExhausted {
                direction: InputServiceRecoveryDirection::SurfaceToInit,
            })
        );
        assert_eq!(tracker, before);
    }
}
