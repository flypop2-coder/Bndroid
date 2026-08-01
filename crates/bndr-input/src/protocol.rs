//! Canonical SurfaceServer/InputServer messages.
//!
//! Sender identity is intentionally absent: the kernel channel envelope is
//! authoritative. Window references below are destinations or scene objects,
//! never claims about the process that sent the wire.

use core::str;

use bndr_ui::{SHELL_HEIGHT, SHELL_WIDTH};

use crate::{
    BoundsError, IdentityError, InputCaptureSnapshot, InputKey, InputRoute, InputWindowId,
    OwnerPid, RouteBounds, RouteDefinitionError, RoutedPointer, TextContext, TextContextError,
    TextPurpose, WindowRef,
};

pub const INPUT_COMMAND_WIRE_SIZE: usize = 64;
pub const INPUT_COMMAND_MAGIC: u32 = u32::from_le_bytes(*b"BIC1");
pub const INPUT_COMMAND_VERSION: u16 = 1;
pub const INPUT_EVENT_WIRE_SIZE: usize = 64;
pub const INPUT_EVENT_MAGIC: u32 = u32::from_le_bytes(*b"BIE1");
pub const INPUT_EVENT_VERSION: u16 = 1;
pub const INPUT_EVENT_TEXT_CAPACITY: usize = 16;
pub const INPUT_ROUTE_CONTROL_WIRE_SIZE: usize = 64;
pub const INPUT_ROUTE_CONTROL_MAGIC: u32 = u32::from_le_bytes(*b"BIR1");
pub const INPUT_ROUTE_CONTROL_VERSION: u16 = 1;
pub const INPUT_GAP_QUEUE_CAPACITY: usize = 16;
pub const INPUT_GAP_PHYSICAL_BUDGET: u16 = 64;

const _: () = assert!(INPUT_COMMAND_WIRE_SIZE == 64);
const _: () = assert!(INPUT_EVENT_WIRE_SIZE == 64);
const _: () = assert!(INPUT_ROUTE_CONTROL_WIRE_SIZE == 64);
const _: () = assert!(INPUT_GAP_QUEUE_CAPACITY <= u16::MAX as usize);

const HEADER_MAGIC: usize = 0;
const HEADER_VERSION: usize = 4;
const HEADER_KIND: usize = 6;
const HEADER_RESERVED: usize = 7;

const COMMAND_SEQUENCE: usize = 8;
const COMMAND_WINDOW: usize = 16;
const COMMAND_GENERATION: usize = 24;
const COMMAND_OWNER_OR_CONTEXT: usize = 32;
const COMMAND_X: usize = 40;
const COMMAND_Y: usize = 42;
const COMMAND_WIDTH: usize = 44;
const COMMAND_HEIGHT: usize = 46;
const COMMAND_Z_ORDER: usize = 48;
const COMMAND_FLAGS: usize = 52;
const COMMAND_RESERVED: usize = 53;

const ROUTE_VISIBLE: u8 = 1 << 0;
const ROUTE_FOCUSABLE: u8 = 1 << 1;
const ROUTE_TRUSTED_OVERLAY: u8 = 1 << 2;
const ROUTE_FLAG_MASK: u8 = ROUTE_VISIBLE | ROUTE_FOCUSABLE | ROUTE_TRUSTED_OVERLAY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputCommandKind {
    SnapshotReset = 1,
    RouteUpsert = 2,
    RouteRemove = 3,
    SetFocus = 4,
    SetTextContext = 5,
    SceneBegin = 6,
    SceneCommit = 7,
}

impl InputCommandKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::SnapshotReset),
            2 => Some(Self::RouteUpsert),
            3 => Some(Self::RouteRemove),
            4 => Some(Self::SetFocus),
            5 => Some(Self::SetTextContext),
            6 => Some(Self::SceneBegin),
            7 => Some(Self::SceneCommit),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCommandPayload {
    SnapshotReset,
    RouteUpsert(InputRoute),
    RouteRemove(WindowRef),
    SetFocus(Option<WindowRef>),
    SetTextContext(Option<TextContext>),
    SceneBegin,
    SceneCommit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCommandError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSequence,
    InvalidIdentity(IdentityError),
    InvalidBounds(BoundsError),
    InvalidRoute(RouteDefinitionError),
    InvalidTextContext(TextContextError),
    InvalidRouteFlags,
    InvalidTextPurpose,
    UnexpectedPayload,
}

/// BIC1: one canonical 64-byte SurfaceServer-to-InputServer command.
///
/// The `sequence` belongs to the route stream for scene begin/commit,
/// snapshot/upsert/remove, and to the focus/control stream for
/// focus/text-context commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputCommand {
    sequence: u64,
    payload: InputCommandPayload,
}

impl InputCommand {
    pub fn scene_begin(sequence: u64) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::SceneBegin,
        })
    }

    pub fn scene_commit(sequence: u64) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::SceneCommit,
        })
    }

    pub fn snapshot_reset(sequence: u64) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::SnapshotReset,
        })
    }

    pub fn route_upsert(sequence: u64, route: InputRoute) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::RouteUpsert(route),
        })
    }

    pub fn route_remove(sequence: u64, target: WindowRef) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::RouteRemove(target),
        })
    }

    pub fn set_focus(sequence: u64, target: Option<WindowRef>) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::SetFocus(target),
        })
    }

    pub fn set_text_context(
        sequence: u64,
        context: Option<TextContext>,
    ) -> Result<Self, InputCommandError> {
        validate_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputCommandPayload::SetTextContext(context),
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn kind(self) -> InputCommandKind {
        match self.payload {
            InputCommandPayload::SnapshotReset => InputCommandKind::SnapshotReset,
            InputCommandPayload::RouteUpsert(_) => InputCommandKind::RouteUpsert,
            InputCommandPayload::RouteRemove(_) => InputCommandKind::RouteRemove,
            InputCommandPayload::SetFocus(_) => InputCommandKind::SetFocus,
            InputCommandPayload::SetTextContext(_) => InputCommandKind::SetTextContext,
            InputCommandPayload::SceneBegin => InputCommandKind::SceneBegin,
            InputCommandPayload::SceneCommit => InputCommandKind::SceneCommit,
        }
    }

    pub const fn payload(self) -> InputCommandPayload {
        self.payload
    }

    pub fn encode(self) -> [u8; INPUT_COMMAND_WIRE_SIZE] {
        let mut wire = [0_u8; INPUT_COMMAND_WIRE_SIZE];
        encode_header(
            &mut wire,
            INPUT_COMMAND_MAGIC,
            INPUT_COMMAND_VERSION,
            self.kind().raw(),
        );
        write_u64(&mut wire, COMMAND_SEQUENCE, self.sequence);
        match self.payload {
            InputCommandPayload::SnapshotReset
            | InputCommandPayload::SceneBegin
            | InputCommandPayload::SceneCommit => {}
            InputCommandPayload::RouteUpsert(route) => {
                encode_target(&mut wire, route.target());
                write_u64(&mut wire, COMMAND_OWNER_OR_CONTEXT, route.owner_pid().get());
                let bounds = route.bounds();
                write_u16(&mut wire, COMMAND_X, bounds.x());
                write_u16(&mut wire, COMMAND_Y, bounds.y());
                write_u16(&mut wire, COMMAND_WIDTH, bounds.width());
                write_u16(&mut wire, COMMAND_HEIGHT, bounds.height());
                write_i32(&mut wire, COMMAND_Z_ORDER, route.z_order());
                let mut flags = 0;
                if route.is_visible() {
                    flags |= ROUTE_VISIBLE;
                }
                if route.is_focusable() {
                    flags |= ROUTE_FOCUSABLE;
                }
                if route.is_trusted_overlay() {
                    flags |= ROUTE_TRUSTED_OVERLAY;
                }
                wire[COMMAND_FLAGS] = flags;
            }
            InputCommandPayload::RouteRemove(target)
            | InputCommandPayload::SetFocus(Some(target)) => encode_target(&mut wire, target),
            InputCommandPayload::SetTextContext(Some(context)) => {
                encode_target(&mut wire, context.target());
                write_u64(&mut wire, COMMAND_OWNER_OR_CONTEXT, context.context_id());
                wire[COMMAND_FLAGS] = context.purpose().raw();
            }
            InputCommandPayload::SetFocus(None) | InputCommandPayload::SetTextContext(None) => {}
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, InputCommandError> {
        validate_header(
            wire,
            INPUT_COMMAND_MAGIC,
            INPUT_COMMAND_VERSION,
            InputCommandKind::from_raw,
        )?;
        if wire[COMMAND_RESERVED..].iter().any(|byte| *byte != 0) {
            return Err(InputCommandError::NonZeroBodyReserved);
        }
        let kind =
            InputCommandKind::from_raw(wire[HEADER_KIND]).ok_or(InputCommandError::InvalidKind)?;
        let sequence = read_u64(wire, COMMAND_SEQUENCE);
        validate_sequence(sequence)?;
        let raw_window = read_u64(wire, COMMAND_WINDOW);
        let generation = read_u64(wire, COMMAND_GENERATION);
        let owner_or_context = read_u64(wire, COMMAND_OWNER_OR_CONTEXT);
        let x = read_u16(wire, COMMAND_X);
        let y = read_u16(wire, COMMAND_Y);
        let width = read_u16(wire, COMMAND_WIDTH);
        let height = read_u16(wire, COMMAND_HEIGHT);
        let z_order = read_i32(wire, COMMAND_Z_ORDER);
        let flags = wire[COMMAND_FLAGS];

        match kind {
            InputCommandKind::SnapshotReset
            | InputCommandKind::SceneBegin
            | InputCommandKind::SceneCommit => {
                if wire[COMMAND_WINDOW..COMMAND_RESERVED]
                    .iter()
                    .any(|byte| *byte != 0)
                {
                    return Err(InputCommandError::UnexpectedPayload);
                }
                match kind {
                    InputCommandKind::SnapshotReset => Self::snapshot_reset(sequence),
                    InputCommandKind::SceneBegin => Self::scene_begin(sequence),
                    InputCommandKind::SceneCommit => Self::scene_commit(sequence),
                    _ => unreachable!(),
                }
            }
            InputCommandKind::RouteUpsert => {
                if flags & !ROUTE_FLAG_MASK != 0 {
                    return Err(InputCommandError::InvalidRouteFlags);
                }
                let target = decode_required_target(raw_window, generation)?;
                let owner_pid = OwnerPid::try_new(owner_or_context)
                    .map_err(InputCommandError::InvalidIdentity)?;
                let bounds = RouteBounds::try_new(x, y, width, height)
                    .map_err(InputCommandError::InvalidBounds)?;
                let route = InputRoute::try_new(
                    target,
                    owner_pid,
                    bounds,
                    z_order,
                    flags & ROUTE_VISIBLE != 0,
                    flags & ROUTE_FOCUSABLE != 0,
                    flags & ROUTE_TRUSTED_OVERLAY != 0,
                )
                .map_err(InputCommandError::InvalidRoute)?;
                Self::route_upsert(sequence, route)
            }
            InputCommandKind::RouteRemove => {
                require_zero_route_extras(owner_or_context, x, y, width, height, z_order, flags)?;
                Self::route_remove(sequence, decode_required_target(raw_window, generation)?)
            }
            InputCommandKind::SetFocus => {
                require_zero_route_extras(owner_or_context, x, y, width, height, z_order, flags)?;
                Self::set_focus(sequence, decode_optional_target(raw_window, generation)?)
            }
            InputCommandKind::SetTextContext => {
                if x != 0 || y != 0 || width != 0 || height != 0 || z_order != 0 {
                    return Err(InputCommandError::UnexpectedPayload);
                }
                let target = decode_optional_target(raw_window, generation)?;
                let context = match target {
                    None if owner_or_context == 0 && flags == 0 => None,
                    None => return Err(InputCommandError::UnexpectedPayload),
                    Some(target) => {
                        let purpose = TextPurpose::from_raw(flags)
                            .ok_or(InputCommandError::InvalidTextPurpose)?;
                        Some(
                            TextContext::try_new(target, owner_or_context, purpose)
                                .map_err(InputCommandError::InvalidTextContext)?,
                        )
                    }
                };
                Self::set_text_context(sequence, context)
            }
        }
    }
}

fn require_zero_route_extras(
    owner: u64,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    z_order: i32,
    flags: u8,
) -> Result<(), InputCommandError> {
    if owner != 0 || x != 0 || y != 0 || width != 0 || height != 0 || z_order != 0 || flags != 0 {
        return Err(InputCommandError::UnexpectedPayload);
    }
    Ok(())
}

const EVENT_SEQUENCE: usize = 8;
const EVENT_RELATED_SEQUENCE: usize = 16;
const EVENT_WINDOW: usize = 24;
const EVENT_GENERATION: usize = 32;
const EVENT_REVISION_OR_COORDINATES: usize = 40;
const EVENT_ARGUMENT_0: usize = 44;
const EVENT_ARGUMENT_1: usize = 45;
const EVENT_TEXT_LENGTH: usize = 46;
const EVENT_BODY_RESERVED: usize = 47;
const EVENT_TEXT: usize = 48;

const POINTER_CAPTURED: u8 = 1 << 0;
const POINTER_TRUSTED_OVERLAY: u8 = 1 << 1;
const POINTER_FLAG_MASK: u8 = POINTER_CAPTURED | POINTER_TRUSTED_OVERLAY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputEventKind {
    Ready = 1,
    Ack = 2,
    RoutedPointer = 3,
    RoutedKey = 4,
    OverlayShown = 5,
    OverlayHidden = 6,
    Preedit = 7,
    Commit = 8,
    DeleteSurrounding = 9,
}

impl InputEventKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Ready),
            2 => Some(Self::Ack),
            3 => Some(Self::RoutedPointer),
            4 => Some(Self::RoutedKey),
            5 => Some(Self::OverlayShown),
            6 => Some(Self::OverlayHidden),
            7 => Some(Self::Preedit),
            8 => Some(Self::Commit),
            9 => Some(Self::DeleteSurrounding),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CommandStream {
    Route = 1,
    Focus = 2,
}

impl CommandStream {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Route),
            2 => Some(Self::Focus),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InlineText16 {
    bytes: [u8; INPUT_EVENT_TEXT_CAPACITY],
    length: u8,
}

impl InlineText16 {
    pub fn try_from_str(value: &str) -> Result<Self, InlineTextError> {
        if value.is_empty() {
            return Err(InlineTextError::Empty);
        }
        if value.len() > INPUT_EVENT_TEXT_CAPACITY {
            return Err(InlineTextError::TooLong);
        }
        let mut bytes = [0_u8; INPUT_EVENT_TEXT_CAPACITY];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            length: value.len() as u8,
        })
    }

    pub fn from_scalar(value: char) -> Self {
        let mut encoded = [0_u8; 4];
        Self::try_from_str(value.encode_utf8(&mut encoded)).expect("one scalar fits inline text")
    }

    fn decode(bytes: &[u8], length: u8) -> Result<Option<Self>, InlineTextError> {
        let length = usize::from(length);
        if length > INPUT_EVENT_TEXT_CAPACITY {
            return Err(InlineTextError::TooLong);
        }
        if bytes[length..].iter().any(|byte| *byte != 0) {
            return Err(InlineTextError::NonZeroTail);
        }
        if length == 0 {
            return Ok(None);
        }
        let value = str::from_utf8(&bytes[..length]).map_err(|_| InlineTextError::InvalidUtf8)?;
        Self::try_from_str(value).map(Some)
    }

    pub const fn len(self) -> usize {
        self.length as usize
    }

    pub const fn is_empty(self) -> bool {
        false
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.bytes[..usize::from(self.length)])
            .expect("InlineText16 is validated UTF-8")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineTextError {
    Empty,
    TooLong,
    InvalidUtf8,
    NonZeroTail,
    NotSingleScalar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventPayload {
    Ready,
    Ack {
        stream: CommandStream,
        acknowledged_sequence: u64,
    },
    RoutedPointer(RoutedPointer),
    RoutedKey {
        input_sequence: u64,
        target: Option<WindowRef>,
        key: InputKey,
        pressed: bool,
    },
    OverlayShown {
        context_id: u64,
    },
    OverlayHidden {
        context_id: u64,
    },
    Preedit {
        context_id: u64,
        target: WindowRef,
        revision: u32,
        text: InlineText16,
    },
    Commit {
        context_id: u64,
        target: WindowRef,
        revision: u32,
        text: InlineText16,
    },
    DeleteSurrounding {
        context_id: u64,
        target: WindowRef,
        revision: u32,
        before_scalars: u8,
        after_scalars: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroEventSequence,
    ZeroRelatedSequence,
    ZeroContextId,
    ZeroRevision,
    InvalidIdentity(IdentityError),
    InvalidCommandStream,
    InvalidKey,
    InvalidBoolean,
    InvalidPointerFlags,
    CoordinateOutOfBounds,
    InvalidText(InlineTextError),
    EmptyDelete,
    UnexpectedPayload,
}

/// BIE1: one canonical 64-byte InputServer-to-SurfaceServer event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputEvent {
    sequence: u64,
    payload: InputEventPayload,
}

impl InputEvent {
    pub fn ready(sequence: u64) -> Result<Self, InputEventError> {
        validate_event_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: InputEventPayload::Ready,
        })
    }

    pub fn ack(
        sequence: u64,
        stream: CommandStream,
        acknowledged_sequence: u64,
    ) -> Result<Self, InputEventError> {
        validate_event_sequence(sequence)?;
        validate_related(acknowledged_sequence)?;
        Ok(Self {
            sequence,
            payload: InputEventPayload::Ack {
                stream,
                acknowledged_sequence,
            },
        })
    }

    pub fn routed_pointer(sequence: u64, pointer: RoutedPointer) -> Result<Self, InputEventError> {
        validate_event_sequence(sequence)?;
        validate_related(pointer.input_sequence)?;
        validate_coordinates(pointer.x, pointer.y)?;
        if pointer.target.is_none() && (pointer.captured || pointer.trusted_overlay) {
            return Err(InputEventError::UnexpectedPayload);
        }
        Ok(Self {
            sequence,
            payload: InputEventPayload::RoutedPointer(pointer),
        })
    }

    pub fn routed_key(
        sequence: u64,
        input_sequence: u64,
        target: Option<WindowRef>,
        key: InputKey,
        pressed: bool,
    ) -> Result<Self, InputEventError> {
        validate_event_sequence(sequence)?;
        validate_related(input_sequence)?;
        Ok(Self {
            sequence,
            payload: InputEventPayload::RoutedKey {
                input_sequence,
                target,
                key,
                pressed,
            },
        })
    }

    pub fn overlay_shown(sequence: u64, context_id: u64) -> Result<Self, InputEventError> {
        Self::overlay_visibility(sequence, context_id, true)
    }

    pub fn overlay_hidden(sequence: u64, context_id: u64) -> Result<Self, InputEventError> {
        Self::overlay_visibility(sequence, context_id, false)
    }

    fn overlay_visibility(
        sequence: u64,
        context_id: u64,
        visible: bool,
    ) -> Result<Self, InputEventError> {
        validate_event_sequence(sequence)?;
        validate_context_id(context_id)?;
        Ok(Self {
            sequence,
            payload: if visible {
                InputEventPayload::OverlayShown { context_id }
            } else {
                InputEventPayload::OverlayHidden { context_id }
            },
        })
    }

    pub fn preedit(
        sequence: u64,
        context_id: u64,
        target: WindowRef,
        revision: u32,
        scalar: char,
    ) -> Result<Self, InputEventError> {
        validate_text_event(sequence, context_id, revision)?;
        Ok(Self {
            sequence,
            payload: InputEventPayload::Preedit {
                context_id,
                target,
                revision,
                text: InlineText16::from_scalar(scalar),
            },
        })
    }

    pub fn commit(
        sequence: u64,
        context_id: u64,
        target: WindowRef,
        revision: u32,
        text: &str,
    ) -> Result<Self, InputEventError> {
        validate_text_event(sequence, context_id, revision)?;
        let text = InlineText16::try_from_str(text).map_err(InputEventError::InvalidText)?;
        Ok(Self {
            sequence,
            payload: InputEventPayload::Commit {
                context_id,
                target,
                revision,
                text,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn delete_surrounding(
        sequence: u64,
        context_id: u64,
        target: WindowRef,
        revision: u32,
        before_scalars: u8,
        after_scalars: u8,
    ) -> Result<Self, InputEventError> {
        validate_text_event(sequence, context_id, revision)?;
        if before_scalars == 0 && after_scalars == 0 {
            return Err(InputEventError::EmptyDelete);
        }
        Ok(Self {
            sequence,
            payload: InputEventPayload::DeleteSurrounding {
                context_id,
                target,
                revision,
                before_scalars,
                after_scalars,
            },
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn kind(self) -> InputEventKind {
        match self.payload {
            InputEventPayload::Ready => InputEventKind::Ready,
            InputEventPayload::Ack { .. } => InputEventKind::Ack,
            InputEventPayload::RoutedPointer(_) => InputEventKind::RoutedPointer,
            InputEventPayload::RoutedKey { .. } => InputEventKind::RoutedKey,
            InputEventPayload::OverlayShown { .. } => InputEventKind::OverlayShown,
            InputEventPayload::OverlayHidden { .. } => InputEventKind::OverlayHidden,
            InputEventPayload::Preedit { .. } => InputEventKind::Preedit,
            InputEventPayload::Commit { .. } => InputEventKind::Commit,
            InputEventPayload::DeleteSurrounding { .. } => InputEventKind::DeleteSurrounding,
        }
    }

    pub const fn payload(self) -> InputEventPayload {
        self.payload
    }

    pub fn encode(self) -> [u8; INPUT_EVENT_WIRE_SIZE] {
        let mut wire = [0_u8; INPUT_EVENT_WIRE_SIZE];
        encode_header(
            &mut wire,
            INPUT_EVENT_MAGIC,
            INPUT_EVENT_VERSION,
            self.kind().raw(),
        );
        write_u64(&mut wire, EVENT_SEQUENCE, self.sequence);
        match self.payload {
            InputEventPayload::Ready => {}
            InputEventPayload::Ack {
                stream,
                acknowledged_sequence,
            } => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, acknowledged_sequence);
                wire[EVENT_ARGUMENT_0] = stream.raw();
            }
            InputEventPayload::RoutedPointer(pointer) => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, pointer.input_sequence);
                if let Some(target) = pointer.target {
                    encode_event_target(&mut wire, target);
                }
                write_u32(
                    &mut wire,
                    EVENT_REVISION_OR_COORDINATES,
                    u32::from(pointer.x) | (u32::from(pointer.y) << 16),
                );
                wire[EVENT_ARGUMENT_0] = u8::from(pointer.pressed);
                let mut flags = 0;
                if pointer.captured {
                    flags |= POINTER_CAPTURED;
                }
                if pointer.trusted_overlay {
                    flags |= POINTER_TRUSTED_OVERLAY;
                }
                wire[EVENT_ARGUMENT_1] = flags;
            }
            InputEventPayload::RoutedKey {
                input_sequence,
                target,
                key,
                pressed,
            } => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, input_sequence);
                if let Some(target) = target {
                    encode_event_target(&mut wire, target);
                }
                wire[EVENT_ARGUMENT_0] = key.raw();
                wire[EVENT_ARGUMENT_1] = u8::from(pressed);
            }
            InputEventPayload::OverlayShown { context_id }
            | InputEventPayload::OverlayHidden { context_id } => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, context_id);
            }
            InputEventPayload::Preedit {
                context_id,
                target,
                revision,
                text,
            }
            | InputEventPayload::Commit {
                context_id,
                target,
                revision,
                text,
            } => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, context_id);
                encode_event_target(&mut wire, target);
                write_u32(&mut wire, EVENT_REVISION_OR_COORDINATES, revision);
                encode_inline_text(&mut wire, text);
            }
            InputEventPayload::DeleteSurrounding {
                context_id,
                target,
                revision,
                before_scalars,
                after_scalars,
            } => {
                write_u64(&mut wire, EVENT_RELATED_SEQUENCE, context_id);
                encode_event_target(&mut wire, target);
                write_u32(&mut wire, EVENT_REVISION_OR_COORDINATES, revision);
                wire[EVENT_ARGUMENT_0] = before_scalars;
                wire[EVENT_ARGUMENT_1] = after_scalars;
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, InputEventError> {
        validate_event_header(wire)?;
        if wire[EVENT_BODY_RESERVED] != 0 {
            return Err(InputEventError::NonZeroBodyReserved);
        }
        let kind =
            InputEventKind::from_raw(wire[HEADER_KIND]).ok_or(InputEventError::InvalidKind)?;
        let sequence = read_u64(wire, EVENT_SEQUENCE);
        validate_event_sequence(sequence)?;
        let related = read_u64(wire, EVENT_RELATED_SEQUENCE);
        let raw_window = read_u64(wire, EVENT_WINDOW);
        let generation = read_u64(wire, EVENT_GENERATION);
        let revision_or_coordinates = read_u32(wire, EVENT_REVISION_OR_COORDINATES);
        let argument_0 = wire[EVENT_ARGUMENT_0];
        let argument_1 = wire[EVENT_ARGUMENT_1];
        let text = InlineText16::decode(
            &wire[EVENT_TEXT..EVENT_TEXT + INPUT_EVENT_TEXT_CAPACITY],
            wire[EVENT_TEXT_LENGTH],
        )
        .map_err(InputEventError::InvalidText)?;

        match kind {
            InputEventKind::Ready => {
                require_empty_event_body(wire)?;
                Self::ready(sequence)
            }
            InputEventKind::Ack => {
                if raw_window != 0
                    || generation != 0
                    || revision_or_coordinates != 0
                    || argument_1 != 0
                    || text.is_some()
                {
                    return Err(InputEventError::UnexpectedPayload);
                }
                let stream = CommandStream::from_raw(argument_0)
                    .ok_or(InputEventError::InvalidCommandStream)?;
                Self::ack(sequence, stream, related)
            }
            InputEventKind::RoutedPointer => {
                validate_related(related)?;
                let target = decode_event_optional_target(raw_window, generation)?;
                let x = revision_or_coordinates as u16;
                let y = (revision_or_coordinates >> 16) as u16;
                validate_coordinates(x, y)?;
                let pressed = decode_bool(argument_0)?;
                if argument_1 & !POINTER_FLAG_MASK != 0 {
                    return Err(InputEventError::InvalidPointerFlags);
                }
                if text.is_some() {
                    return Err(InputEventError::UnexpectedPayload);
                }
                Self::routed_pointer(
                    sequence,
                    RoutedPointer {
                        input_sequence: related,
                        target,
                        x,
                        y,
                        pressed,
                        captured: argument_1 & POINTER_CAPTURED != 0,
                        trusted_overlay: argument_1 & POINTER_TRUSTED_OVERLAY != 0,
                    },
                )
            }
            InputEventKind::RoutedKey => {
                if revision_or_coordinates != 0 || text.is_some() {
                    return Err(InputEventError::UnexpectedPayload);
                }
                let target = decode_event_optional_target(raw_window, generation)?;
                let key = InputKey::from_raw(argument_0).ok_or(InputEventError::InvalidKey)?;
                Self::routed_key(sequence, related, target, key, decode_bool(argument_1)?)
            }
            InputEventKind::OverlayShown | InputEventKind::OverlayHidden => {
                if raw_window != 0
                    || generation != 0
                    || revision_or_coordinates != 0
                    || argument_0 != 0
                    || argument_1 != 0
                    || text.is_some()
                {
                    return Err(InputEventError::UnexpectedPayload);
                }
                if kind == InputEventKind::OverlayShown {
                    Self::overlay_shown(sequence, related)
                } else {
                    Self::overlay_hidden(sequence, related)
                }
            }
            InputEventKind::Preedit | InputEventKind::Commit => {
                if argument_0 != 0 || argument_1 != 0 {
                    return Err(InputEventError::UnexpectedPayload);
                }
                let target = decode_event_required_target(raw_window, generation)?;
                let text = text.ok_or(InputEventError::InvalidText(InlineTextError::Empty))?;
                if kind == InputEventKind::Preedit {
                    let mut chars = text.as_str().chars();
                    let scalar = chars
                        .next()
                        .ok_or(InputEventError::InvalidText(InlineTextError::Empty))?;
                    if chars.next().is_some() {
                        return Err(InputEventError::InvalidText(
                            InlineTextError::NotSingleScalar,
                        ));
                    }
                    Self::preedit(sequence, related, target, revision_or_coordinates, scalar)
                } else {
                    Self::commit(
                        sequence,
                        related,
                        target,
                        revision_or_coordinates,
                        text.as_str(),
                    )
                }
            }
            InputEventKind::DeleteSurrounding => {
                if text.is_some() {
                    return Err(InputEventError::UnexpectedPayload);
                }
                let target = decode_event_required_target(raw_window, generation)?;
                Self::delete_surrounding(
                    sequence,
                    related,
                    target,
                    revision_or_coordinates,
                    argument_0,
                    argument_1,
                )
            }
        }
    }
}

const ROUTE_CONTROL_SEQUENCE: usize = 8;
const ROUTE_CONTROL_EPOCH: usize = 16;
const ROUTE_CONTROL_INPUT_SESSION: usize = 24;
const ROUTE_CONTROL_VALUE_0: usize = 32;
const ROUTE_CONTROL_VALUE_1: usize = 40;
const ROUTE_CONTROL_VALUE_2: usize = 48;
const ROUTE_CONTROL_VALUE_3: usize = 56;
const ROUTE_CONTROL_GAP_PENDING: usize = 56;
const ROUTE_CONTROL_GAP_HIGH_WATER: usize = 58;
const ROUTE_CONTROL_GAP_COALESCED: usize = 60;
const ROUTE_CONTROL_GAP_CAPACITY: usize = 62;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRouteControlDirection {
    InitToInput,
    InputToInit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputRouteControlKind {
    Acquired = 1,
    Bind = 2,
    Ready = 3,
    RouteLost = 4,
    RouteLostClientCapture = 5,
    RouteLostTrustedCapture = 6,
    GapStatus = 7,
}

impl InputRouteControlKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Acquired),
            2 => Some(Self::Bind),
            3 => Some(Self::Ready),
            4 => Some(Self::RouteLost),
            5 => Some(Self::RouteLostClientCapture),
            6 => Some(Self::RouteLostTrustedCapture),
            7 => Some(Self::GapStatus),
            _ => None,
        }
    }

    pub const fn direction(self) -> InputRouteControlDirection {
        match self {
            Self::Bind => InputRouteControlDirection::InitToInput,
            Self::Acquired
            | Self::Ready
            | Self::RouteLost
            | Self::RouteLostClientCapture
            | Self::RouteLostTrustedCapture
            | Self::GapStatus => InputRouteControlDirection::InputToInit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRouteControlError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    ZeroSequence,
    UnexpectedRouteEpoch,
    ZeroInputSession,
    InvalidIdentity(IdentityError),
    InvalidGapCapacity,
    InvalidPhysicalBudget,
    InvalidGapMetrics,
    UnexpectedPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRouteGapMetrics {
    physical_enqueued: u64,
    physical_dequeued: u64,
    logical_pending: u16,
    logical_high_water: u16,
    coalesced: u16,
}

impl InputRouteGapMetrics {
    pub fn try_new(
        physical_enqueued: u64,
        physical_dequeued: u64,
        logical_pending: u16,
        logical_high_water: u16,
        coalesced: u16,
    ) -> Result<Self, InputRouteControlError> {
        let stored_total = physical_enqueued.saturating_sub(u64::from(coalesced));
        if physical_enqueued > u64::from(INPUT_GAP_PHYSICAL_BUDGET)
            || physical_dequeued > physical_enqueued
            || usize::from(logical_pending) > INPUT_GAP_QUEUE_CAPACITY
            || logical_pending > logical_high_water
            || usize::from(logical_high_water) > INPUT_GAP_QUEUE_CAPACITY
            || u64::from(coalesced) > physical_enqueued
            || u64::from(logical_high_water) > stored_total
            || u64::from(logical_pending) > physical_enqueued - physical_dequeued
            || (physical_enqueued == physical_dequeued && logical_pending != 0)
            || (physical_enqueued > physical_dequeued && logical_pending == 0)
            || (physical_enqueued == 0 && (logical_high_water != 0 || coalesced != 0))
            || (physical_enqueued != 0 && logical_high_water == 0)
        {
            return Err(InputRouteControlError::InvalidGapMetrics);
        }
        Ok(Self {
            physical_enqueued,
            physical_dequeued,
            logical_pending,
            logical_high_water,
            coalesced,
        })
    }

    pub const fn physical_enqueued(self) -> u64 {
        self.physical_enqueued
    }

    pub const fn physical_dequeued(self) -> u64 {
        self.physical_dequeued
    }

    pub const fn logical_pending(self) -> u16 {
        self.logical_pending
    }

    pub const fn logical_high_water(self) -> u16 {
        self.logical_high_water
    }

    pub const fn coalesced(self) -> u16 {
        self.coalesced
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRouteControlPayload {
    Acquired {
        physical_sequence_floor: u64,
    },
    Bind {
        expected_surface_pid: OwnerPid,
        expected_physical_floor: u64,
    },
    Ready {
        bound_surface_pid: OwnerPid,
        physical_sequence_floor: u64,
    },
    RouteLost {
        physical_sequence_floor: u64,
        capture: Option<InputCaptureSnapshot>,
    },
    GapStatus {
        physical_sequence_floor: u64,
        metrics: InputRouteGapMetrics,
    },
}

/// BIR1: one canonical 64-byte Init/InputServer route-lifecycle message.
///
/// Sender identity remains outside the payload: callers must authenticate the
/// kernel channel envelope and enforce [`InputRouteControlKind::direction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRouteControl {
    sequence: u64,
    route_epoch: u64,
    input_session_id: u64,
    payload: InputRouteControlPayload,
}

impl InputRouteControl {
    pub fn acquired(
        sequence: u64,
        input_session_id: u64,
        physical_sequence_floor: u64,
    ) -> Result<Self, InputRouteControlError> {
        validate_route_control_sequence(sequence)?;
        validate_input_session(input_session_id)?;
        Ok(Self {
            sequence,
            route_epoch: 0,
            input_session_id,
            payload: InputRouteControlPayload::Acquired {
                physical_sequence_floor,
            },
        })
    }

    pub fn bind(
        sequence: u64,
        route_epoch: u64,
        input_session_id: u64,
        expected_surface_pid: OwnerPid,
        expected_physical_floor: u64,
    ) -> Result<Self, InputRouteControlError> {
        validate_route_control_common(sequence, route_epoch, input_session_id)?;
        Ok(Self {
            sequence,
            route_epoch,
            input_session_id,
            payload: InputRouteControlPayload::Bind {
                expected_surface_pid,
                expected_physical_floor,
            },
        })
    }

    pub fn ready(
        sequence: u64,
        route_epoch: u64,
        input_session_id: u64,
        bound_surface_pid: OwnerPid,
        physical_sequence_floor: u64,
    ) -> Result<Self, InputRouteControlError> {
        validate_route_control_common(sequence, route_epoch, input_session_id)?;
        Ok(Self {
            sequence,
            route_epoch,
            input_session_id,
            payload: InputRouteControlPayload::Ready {
                bound_surface_pid,
                physical_sequence_floor,
            },
        })
    }

    pub fn route_lost(
        sequence: u64,
        route_epoch: u64,
        input_session_id: u64,
        physical_sequence_floor: u64,
        capture: Option<InputCaptureSnapshot>,
    ) -> Result<Self, InputRouteControlError> {
        validate_route_control_common(sequence, route_epoch, input_session_id)?;
        Ok(Self {
            sequence,
            route_epoch,
            input_session_id,
            payload: InputRouteControlPayload::RouteLost {
                physical_sequence_floor,
                capture,
            },
        })
    }

    pub fn gap_status(
        sequence: u64,
        route_epoch: u64,
        input_session_id: u64,
        physical_sequence_floor: u64,
        metrics: InputRouteGapMetrics,
    ) -> Result<Self, InputRouteControlError> {
        validate_route_control_common(sequence, route_epoch, input_session_id)?;
        Ok(Self {
            sequence,
            route_epoch,
            input_session_id,
            payload: InputRouteControlPayload::GapStatus {
                physical_sequence_floor,
                metrics,
            },
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn route_epoch(self) -> u64 {
        self.route_epoch
    }

    pub const fn input_session_id(self) -> u64 {
        self.input_session_id
    }

    pub const fn payload(self) -> InputRouteControlPayload {
        self.payload
    }

    pub const fn kind(self) -> InputRouteControlKind {
        match self.payload {
            InputRouteControlPayload::Acquired { .. } => InputRouteControlKind::Acquired,
            InputRouteControlPayload::Bind { .. } => InputRouteControlKind::Bind,
            InputRouteControlPayload::Ready { .. } => InputRouteControlKind::Ready,
            InputRouteControlPayload::RouteLost { capture: None, .. } => {
                InputRouteControlKind::RouteLost
            }
            InputRouteControlPayload::RouteLost {
                capture: Some(capture),
                ..
            } if capture.trusted_overlay() => InputRouteControlKind::RouteLostTrustedCapture,
            InputRouteControlPayload::RouteLost {
                capture: Some(_), ..
            } => InputRouteControlKind::RouteLostClientCapture,
            InputRouteControlPayload::GapStatus { .. } => InputRouteControlKind::GapStatus,
        }
    }

    pub fn encode(self) -> [u8; INPUT_ROUTE_CONTROL_WIRE_SIZE] {
        let mut wire = [0_u8; INPUT_ROUTE_CONTROL_WIRE_SIZE];
        encode_header(
            &mut wire,
            INPUT_ROUTE_CONTROL_MAGIC,
            INPUT_ROUTE_CONTROL_VERSION,
            self.kind().raw(),
        );
        write_u64(&mut wire, ROUTE_CONTROL_SEQUENCE, self.sequence);
        write_u64(&mut wire, ROUTE_CONTROL_EPOCH, self.route_epoch);
        write_u64(
            &mut wire,
            ROUTE_CONTROL_INPUT_SESSION,
            self.input_session_id,
        );
        match self.payload {
            InputRouteControlPayload::Acquired {
                physical_sequence_floor,
            } => write_u64(&mut wire, ROUTE_CONTROL_VALUE_0, physical_sequence_floor),
            InputRouteControlPayload::Bind {
                expected_surface_pid,
                expected_physical_floor,
            } => {
                encode_bound_route(&mut wire, expected_surface_pid, expected_physical_floor);
            }
            InputRouteControlPayload::Ready {
                bound_surface_pid,
                physical_sequence_floor,
            } => {
                encode_bound_route(&mut wire, bound_surface_pid, physical_sequence_floor);
            }
            InputRouteControlPayload::RouteLost {
                physical_sequence_floor,
                capture,
            } => {
                write_u64(&mut wire, ROUTE_CONTROL_VALUE_0, physical_sequence_floor);
                if let Some(capture) = capture {
                    write_u64(&mut wire, ROUTE_CONTROL_VALUE_1, capture.owner_pid().get());
                    write_u64(
                        &mut wire,
                        ROUTE_CONTROL_VALUE_2,
                        capture.target().window_id().get(),
                    );
                    write_u64(
                        &mut wire,
                        ROUTE_CONTROL_VALUE_3,
                        capture.target().generation(),
                    );
                }
            }
            InputRouteControlPayload::GapStatus {
                physical_sequence_floor,
                metrics,
            } => {
                write_u64(&mut wire, ROUTE_CONTROL_VALUE_0, physical_sequence_floor);
                write_u64(&mut wire, ROUTE_CONTROL_VALUE_1, metrics.physical_enqueued);
                write_u64(&mut wire, ROUTE_CONTROL_VALUE_2, metrics.physical_dequeued);
                write_u16(
                    &mut wire,
                    ROUTE_CONTROL_GAP_PENDING,
                    metrics.logical_pending,
                );
                write_u16(
                    &mut wire,
                    ROUTE_CONTROL_GAP_HIGH_WATER,
                    metrics.logical_high_water,
                );
                write_u16(&mut wire, ROUTE_CONTROL_GAP_COALESCED, metrics.coalesced);
                write_u16(
                    &mut wire,
                    ROUTE_CONTROL_GAP_CAPACITY,
                    INPUT_GAP_QUEUE_CAPACITY as u16,
                );
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, InputRouteControlError> {
        if wire.len() != INPUT_ROUTE_CONTROL_WIRE_SIZE {
            return Err(InputRouteControlError::InvalidWireLength);
        }
        if read_u32(wire, HEADER_MAGIC) != INPUT_ROUTE_CONTROL_MAGIC {
            return Err(InputRouteControlError::InvalidMagic);
        }
        if read_u16(wire, HEADER_VERSION) != INPUT_ROUTE_CONTROL_VERSION {
            return Err(InputRouteControlError::InvalidVersion);
        }
        let kind = InputRouteControlKind::from_raw(wire[HEADER_KIND])
            .ok_or(InputRouteControlError::InvalidKind)?;
        if wire[HEADER_RESERVED] != 0 {
            return Err(InputRouteControlError::NonZeroHeaderReserved);
        }
        let sequence = read_u64(wire, ROUTE_CONTROL_SEQUENCE);
        let route_epoch = read_u64(wire, ROUTE_CONTROL_EPOCH);
        let input_session_id = read_u64(wire, ROUTE_CONTROL_INPUT_SESSION);
        validate_route_control_sequence(sequence)?;
        validate_input_session(input_session_id)?;
        if (kind == InputRouteControlKind::Acquired && route_epoch != 0)
            || (kind != InputRouteControlKind::Acquired && route_epoch == 0)
        {
            return Err(InputRouteControlError::UnexpectedRouteEpoch);
        }
        let value_0 = read_u64(wire, ROUTE_CONTROL_VALUE_0);
        let value_1 = read_u64(wire, ROUTE_CONTROL_VALUE_1);
        let value_2 = read_u64(wire, ROUTE_CONTROL_VALUE_2);
        let value_3 = read_u64(wire, ROUTE_CONTROL_VALUE_3);

        match kind {
            InputRouteControlKind::Acquired => {
                require_zero_route_control_values(value_1, value_2, value_3)?;
                Self::acquired(sequence, input_session_id, value_0)
            }
            InputRouteControlKind::Bind | InputRouteControlKind::Ready => {
                if value_2 != INPUT_GAP_QUEUE_CAPACITY as u64 {
                    return Err(InputRouteControlError::InvalidGapCapacity);
                }
                if value_3 != u64::from(INPUT_GAP_PHYSICAL_BUDGET) {
                    return Err(InputRouteControlError::InvalidPhysicalBudget);
                }
                let surface_pid =
                    OwnerPid::try_new(value_0).map_err(InputRouteControlError::InvalidIdentity)?;
                if kind == InputRouteControlKind::Bind {
                    Self::bind(
                        sequence,
                        route_epoch,
                        input_session_id,
                        surface_pid,
                        value_1,
                    )
                } else {
                    Self::ready(
                        sequence,
                        route_epoch,
                        input_session_id,
                        surface_pid,
                        value_1,
                    )
                }
            }
            InputRouteControlKind::RouteLost => {
                require_zero_route_control_values(value_1, value_2, value_3)?;
                Self::route_lost(sequence, route_epoch, input_session_id, value_0, None)
            }
            InputRouteControlKind::RouteLostClientCapture
            | InputRouteControlKind::RouteLostTrustedCapture => {
                let owner_pid =
                    OwnerPid::try_new(value_1).map_err(InputRouteControlError::InvalidIdentity)?;
                let window_id = InputWindowId::try_new(value_2)
                    .map_err(InputRouteControlError::InvalidIdentity)?;
                let target = WindowRef::try_new(window_id, value_3)
                    .map_err(InputRouteControlError::InvalidIdentity)?;
                let capture = InputCaptureSnapshot::new(
                    target,
                    owner_pid,
                    kind == InputRouteControlKind::RouteLostTrustedCapture,
                );
                Self::route_lost(
                    sequence,
                    route_epoch,
                    input_session_id,
                    value_0,
                    Some(capture),
                )
            }
            InputRouteControlKind::GapStatus => {
                let capacity = read_u16(wire, ROUTE_CONTROL_GAP_CAPACITY);
                if usize::from(capacity) != INPUT_GAP_QUEUE_CAPACITY {
                    return Err(InputRouteControlError::InvalidGapCapacity);
                }
                let metrics = InputRouteGapMetrics::try_new(
                    value_1,
                    value_2,
                    read_u16(wire, ROUTE_CONTROL_GAP_PENDING),
                    read_u16(wire, ROUTE_CONTROL_GAP_HIGH_WATER),
                    read_u16(wire, ROUTE_CONTROL_GAP_COALESCED),
                )?;
                Self::gap_status(sequence, route_epoch, input_session_id, value_0, metrics)
            }
        }
    }
}

fn encode_bound_route(wire: &mut [u8; 64], surface_pid: OwnerPid, physical_floor: u64) {
    write_u64(wire, ROUTE_CONTROL_VALUE_0, surface_pid.get());
    write_u64(wire, ROUTE_CONTROL_VALUE_1, physical_floor);
    write_u64(wire, ROUTE_CONTROL_VALUE_2, INPUT_GAP_QUEUE_CAPACITY as u64);
    write_u64(
        wire,
        ROUTE_CONTROL_VALUE_3,
        u64::from(INPUT_GAP_PHYSICAL_BUDGET),
    );
}

fn require_zero_route_control_values(
    value_1: u64,
    value_2: u64,
    value_3: u64,
) -> Result<(), InputRouteControlError> {
    if value_1 != 0 || value_2 != 0 || value_3 != 0 {
        return Err(InputRouteControlError::UnexpectedPayload);
    }
    Ok(())
}

fn validate_route_control_sequence(sequence: u64) -> Result<(), InputRouteControlError> {
    if sequence == 0 {
        return Err(InputRouteControlError::ZeroSequence);
    }
    Ok(())
}

fn validate_input_session(input_session_id: u64) -> Result<(), InputRouteControlError> {
    if input_session_id == 0 {
        return Err(InputRouteControlError::ZeroInputSession);
    }
    Ok(())
}

fn validate_route_control_common(
    sequence: u64,
    route_epoch: u64,
    input_session_id: u64,
) -> Result<(), InputRouteControlError> {
    validate_route_control_sequence(sequence)?;
    if route_epoch == 0 {
        return Err(InputRouteControlError::UnexpectedRouteEpoch);
    }
    validate_input_session(input_session_id)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRouteControlTrackerError {
    SequenceReplay {
        direction: InputRouteControlDirection,
        last: u64,
    },
    SequenceGap {
        direction: InputRouteControlDirection,
        expected: u64,
    },
    SequenceExhausted {
        direction: InputRouteControlDirection,
    },
}

/// Independent contiguous ordering for both directions of the stable control
/// channel. Failed accepts are transactional.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InputRouteControlSequenceTracker {
    init_to_input_last: u64,
    input_to_init_last: u64,
}

impl InputRouteControlSequenceTracker {
    pub const fn new() -> Self {
        Self {
            init_to_input_last: 0,
            input_to_init_last: 0,
        }
    }

    pub fn accept(
        &mut self,
        control: InputRouteControl,
    ) -> Result<(), InputRouteControlTrackerError> {
        let direction = control.kind().direction();
        let last = self.last_sequence(direction);
        if last == u64::MAX {
            return Err(InputRouteControlTrackerError::SequenceExhausted { direction });
        }
        let expected = last + 1;
        if control.sequence <= last {
            return Err(InputRouteControlTrackerError::SequenceReplay { direction, last });
        }
        if control.sequence != expected {
            return Err(InputRouteControlTrackerError::SequenceGap {
                direction,
                expected,
            });
        }
        match direction {
            InputRouteControlDirection::InitToInput => {
                self.init_to_input_last = control.sequence;
            }
            InputRouteControlDirection::InputToInit => {
                self.input_to_init_last = control.sequence;
            }
        }
        Ok(())
    }

    pub const fn last_sequence(self, direction: InputRouteControlDirection) -> u64 {
        match direction {
            InputRouteControlDirection::InitToInput => self.init_to_input_last,
            InputRouteControlDirection::InputToInit => self.input_to_init_last,
        }
    }
}

fn validate_sequence(sequence: u64) -> Result<(), InputCommandError> {
    if sequence == 0 {
        return Err(InputCommandError::ZeroSequence);
    }
    Ok(())
}

fn validate_event_sequence(sequence: u64) -> Result<(), InputEventError> {
    if sequence == 0 {
        return Err(InputEventError::ZeroEventSequence);
    }
    Ok(())
}

fn validate_related(sequence: u64) -> Result<(), InputEventError> {
    if sequence == 0 {
        return Err(InputEventError::ZeroRelatedSequence);
    }
    Ok(())
}

fn validate_context_id(context_id: u64) -> Result<(), InputEventError> {
    if context_id == 0 {
        return Err(InputEventError::ZeroContextId);
    }
    Ok(())
}

fn validate_text_event(
    sequence: u64,
    context_id: u64,
    revision: u32,
) -> Result<(), InputEventError> {
    validate_event_sequence(sequence)?;
    validate_context_id(context_id)?;
    if revision == 0 {
        return Err(InputEventError::ZeroRevision);
    }
    Ok(())
}

fn validate_coordinates(x: u16, y: u16) -> Result<(), InputEventError> {
    if x >= SHELL_WIDTH || y >= SHELL_HEIGHT {
        return Err(InputEventError::CoordinateOutOfBounds);
    }
    Ok(())
}

fn decode_bool(raw: u8) -> Result<bool, InputEventError> {
    match raw {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(InputEventError::InvalidBoolean),
    }
}

fn decode_required_target(raw: u64, generation: u64) -> Result<WindowRef, InputCommandError> {
    let window_id = InputWindowId::try_new(raw).map_err(InputCommandError::InvalidIdentity)?;
    WindowRef::try_new(window_id, generation).map_err(InputCommandError::InvalidIdentity)
}

fn decode_optional_target(
    raw: u64,
    generation: u64,
) -> Result<Option<WindowRef>, InputCommandError> {
    if raw == 0 && generation == 0 {
        return Ok(None);
    }
    if raw == 0 || generation == 0 {
        return Err(InputCommandError::UnexpectedPayload);
    }
    decode_required_target(raw, generation).map(Some)
}

fn decode_event_required_target(raw: u64, generation: u64) -> Result<WindowRef, InputEventError> {
    let window_id = InputWindowId::try_new(raw).map_err(InputEventError::InvalidIdentity)?;
    WindowRef::try_new(window_id, generation).map_err(InputEventError::InvalidIdentity)
}

fn decode_event_optional_target(
    raw: u64,
    generation: u64,
) -> Result<Option<WindowRef>, InputEventError> {
    if raw == 0 && generation == 0 {
        return Ok(None);
    }
    if raw == 0 || generation == 0 {
        return Err(InputEventError::UnexpectedPayload);
    }
    decode_event_required_target(raw, generation).map(Some)
}

fn encode_target(wire: &mut [u8; 64], target: WindowRef) {
    write_u64(wire, COMMAND_WINDOW, target.window_id().get());
    write_u64(wire, COMMAND_GENERATION, target.generation());
}

fn encode_event_target(wire: &mut [u8; 64], target: WindowRef) {
    write_u64(wire, EVENT_WINDOW, target.window_id().get());
    write_u64(wire, EVENT_GENERATION, target.generation());
}

fn encode_inline_text(wire: &mut [u8; 64], text: InlineText16) {
    wire[EVENT_TEXT_LENGTH] = text.length;
    wire[EVENT_TEXT..EVENT_TEXT + text.len()].copy_from_slice(&text.bytes[..text.len()]);
}

fn require_empty_event_body(wire: &[u8]) -> Result<(), InputEventError> {
    if wire[EVENT_RELATED_SEQUENCE..].iter().any(|byte| *byte != 0) {
        return Err(InputEventError::UnexpectedPayload);
    }
    Ok(())
}

fn encode_header(wire: &mut [u8; 64], magic: u32, version: u16, kind: u8) {
    write_u32(wire, HEADER_MAGIC, magic);
    write_u16(wire, HEADER_VERSION, version);
    wire[HEADER_KIND] = kind;
}

fn validate_header<K>(
    wire: &[u8],
    magic: u32,
    version: u16,
    kind: impl FnOnce(u8) -> Option<K>,
) -> Result<(), InputCommandError> {
    if wire.len() != INPUT_COMMAND_WIRE_SIZE {
        return Err(InputCommandError::InvalidWireLength);
    }
    if read_u32(wire, HEADER_MAGIC) != magic {
        return Err(InputCommandError::InvalidMagic);
    }
    if read_u16(wire, HEADER_VERSION) != version {
        return Err(InputCommandError::InvalidVersion);
    }
    if kind(wire[HEADER_KIND]).is_none() {
        return Err(InputCommandError::InvalidKind);
    }
    if wire[HEADER_RESERVED] != 0 {
        return Err(InputCommandError::NonZeroHeaderReserved);
    }
    Ok(())
}

fn validate_event_header(wire: &[u8]) -> Result<(), InputEventError> {
    if wire.len() != INPUT_EVENT_WIRE_SIZE {
        return Err(InputEventError::InvalidWireLength);
    }
    if read_u32(wire, HEADER_MAGIC) != INPUT_EVENT_MAGIC {
        return Err(InputEventError::InvalidMagic);
    }
    if read_u16(wire, HEADER_VERSION) != INPUT_EVENT_VERSION {
        return Err(InputEventError::InvalidVersion);
    }
    if InputEventKind::from_raw(wire[HEADER_KIND]).is_none() {
        return Err(InputEventError::InvalidKind);
    }
    if wire[HEADER_RESERVED] != 0 {
        return Err(InputEventError::NonZeroHeaderReserved);
    }
    Ok(())
}

fn write_u16(wire: &mut [u8; 64], offset: usize, value: u16) {
    wire[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(wire: &mut [u8; 64], offset: usize, value: u32) {
    wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(wire: &mut [u8; 64], offset: usize, value: i32) {
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

fn read_i32(wire: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(wire[offset..offset + 4].try_into().expect("four bytes"))
}

fn read_u64(wire: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(wire[offset..offset + 8].try_into().expect("eight bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(raw: u64, generation: u64) -> WindowRef {
        WindowRef::try_new(InputWindowId::try_new(raw).unwrap(), generation).unwrap()
    }

    fn route() -> InputRoute {
        InputRoute::try_new(
            target(9, 4),
            OwnerPid::try_new(0x2000_0003).unwrap(),
            RouteBounds::try_new(12, 34, 56, 78).unwrap(),
            -17,
            true,
            true,
            false,
        )
        .unwrap()
    }

    fn context() -> TextContext {
        TextContext::try_new(target(9, 4), 22, TextPurpose::Email).unwrap()
    }

    fn capture(trusted_overlay: bool) -> InputCaptureSnapshot {
        InputCaptureSnapshot::new(
            target(9, 4),
            OwnerPid::try_new(0x2000_0003).unwrap(),
            trusted_overlay,
        )
    }

    #[test]
    fn wire_constants_are_exact_and_magics_are_little_endian_ascii() {
        assert_eq!(INPUT_COMMAND_WIRE_SIZE, 64);
        assert_eq!(INPUT_EVENT_WIRE_SIZE, 64);
        assert_eq!(INPUT_ROUTE_CONTROL_WIRE_SIZE, 64);
        assert_eq!(INPUT_COMMAND_MAGIC.to_le_bytes(), *b"BIC1");
        assert_eq!(INPUT_EVENT_MAGIC.to_le_bytes(), *b"BIE1");
        assert_eq!(INPUT_ROUTE_CONTROL_MAGIC.to_le_bytes(), *b"BIR1");
        assert_eq!(INPUT_GAP_QUEUE_CAPACITY, 16);
        assert_eq!(INPUT_GAP_PHYSICAL_BUDGET, 64);
    }

    #[test]
    fn every_command_kind_round_trips_canonically() {
        let commands = [
            InputCommand::snapshot_reset(1).unwrap(),
            InputCommand::route_upsert(2, route()).unwrap(),
            InputCommand::route_remove(3, target(9, 4)).unwrap(),
            InputCommand::set_focus(4, Some(target(9, 4))).unwrap(),
            InputCommand::set_focus(5, None).unwrap(),
            InputCommand::set_text_context(6, Some(context())).unwrap(),
            InputCommand::set_text_context(7, None).unwrap(),
            InputCommand::scene_begin(8).unwrap(),
            InputCommand::scene_commit(9).unwrap(),
        ];
        for command in commands {
            let wire = command.encode();
            assert_eq!(InputCommand::decode(&wire), Ok(command));
            assert_eq!(InputCommand::decode(&wire).unwrap().encode(), wire);
        }
    }

    #[test]
    fn command_header_and_reserved_bytes_are_strict() {
        let canonical = InputCommand::snapshot_reset(1).unwrap().encode();
        assert_eq!(
            InputCommand::decode(&canonical[..63]),
            Err(InputCommandError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, InputCommandError::InvalidMagic),
            (4, InputCommandError::InvalidVersion),
            (6, InputCommandError::InvalidKind),
            (7, InputCommandError::NonZeroHeaderReserved),
            (63, InputCommandError::NonZeroBodyReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(InputCommand::decode(&wire), Err(expected));
        }
    }

    #[test]
    fn command_zero_sequence_and_cross_kind_payload_are_rejected() {
        assert_eq!(
            InputCommand::snapshot_reset(0),
            Err(InputCommandError::ZeroSequence)
        );
        assert_eq!(
            InputCommand::scene_begin(0),
            Err(InputCommandError::ZeroSequence)
        );
        assert_eq!(
            InputCommand::scene_commit(0),
            Err(InputCommandError::ZeroSequence)
        );
        let mut wire = InputCommand::snapshot_reset(1).unwrap().encode();
        wire[COMMAND_WINDOW] = 1;
        assert_eq!(
            InputCommand::decode(&wire),
            Err(InputCommandError::UnexpectedPayload)
        );
        let mut wire = InputCommand::route_remove(1, target(1, 1))
            .unwrap()
            .encode();
        wire[COMMAND_X] = 1;
        assert_eq!(
            InputCommand::decode(&wire),
            Err(InputCommandError::UnexpectedPayload)
        );
    }

    #[test]
    fn scene_transaction_kinds_are_closed_and_require_an_empty_payload() {
        assert_eq!(InputCommandKind::SceneBegin.raw(), 6);
        assert_eq!(InputCommandKind::SceneCommit.raw(), 7);
        assert_eq!(
            InputCommandKind::from_raw(6),
            Some(InputCommandKind::SceneBegin)
        );
        assert_eq!(
            InputCommandKind::from_raw(7),
            Some(InputCommandKind::SceneCommit)
        );
        assert_eq!(InputCommandKind::from_raw(8), None);

        for canonical in [
            InputCommand::scene_begin(1).unwrap().encode(),
            InputCommand::scene_commit(2).unwrap().encode(),
        ] {
            assert!(canonical[COMMAND_WINDOW..].iter().all(|byte| *byte == 0));
            for offset in COMMAND_WINDOW..COMMAND_RESERVED {
                let mut wire = canonical;
                wire[offset] = 1;
                assert_eq!(
                    InputCommand::decode(&wire),
                    Err(InputCommandError::UnexpectedPayload)
                );
            }
        }
    }

    #[test]
    fn command_optional_target_must_be_an_exact_zero_pair() {
        let mut wire = InputCommand::set_focus(1, None).unwrap().encode();
        write_u64(&mut wire, COMMAND_WINDOW, 1);
        assert_eq!(
            InputCommand::decode(&wire),
            Err(InputCommandError::UnexpectedPayload)
        );
        let mut wire = InputCommand::set_focus(1, None).unwrap().encode();
        write_u64(&mut wire, COMMAND_GENERATION, 1);
        assert_eq!(
            InputCommand::decode(&wire),
            Err(InputCommandError::UnexpectedPayload)
        );
    }

    #[test]
    fn command_route_validates_flags_geometry_owner_and_overlay_focus() {
        let canonical = InputCommand::route_upsert(1, route()).unwrap().encode();
        let mut bad_flags = canonical;
        bad_flags[COMMAND_FLAGS] |= 0x80;
        assert_eq!(
            InputCommand::decode(&bad_flags),
            Err(InputCommandError::InvalidRouteFlags)
        );
        let mut empty = canonical;
        write_u16(&mut empty, COMMAND_WIDTH, 0);
        assert_eq!(
            InputCommand::decode(&empty),
            Err(InputCommandError::InvalidBounds(BoundsError::Empty))
        );
        let mut no_owner = canonical;
        write_u64(&mut no_owner, COMMAND_OWNER_OR_CONTEXT, 0);
        assert_eq!(
            InputCommand::decode(&no_owner),
            Err(InputCommandError::InvalidIdentity(
                IdentityError::ZeroOwnerPid
            ))
        );
        let mut focusable_overlay = canonical;
        focusable_overlay[COMMAND_FLAGS] |= ROUTE_TRUSTED_OVERLAY;
        assert_eq!(
            InputCommand::decode(&focusable_overlay),
            Err(InputCommandError::InvalidRoute(
                RouteDefinitionError::TrustedOverlayMustNotFocus
            ))
        );
    }

    #[test]
    fn text_context_requires_complete_target_context_and_closed_purpose() {
        let canonical = InputCommand::set_text_context(1, Some(context()))
            .unwrap()
            .encode();
        let mut no_context = canonical;
        write_u64(&mut no_context, COMMAND_OWNER_OR_CONTEXT, 0);
        assert_eq!(
            InputCommand::decode(&no_context),
            Err(InputCommandError::InvalidTextContext(
                TextContextError::ZeroContextId
            ))
        );
        let mut bad_purpose = canonical;
        bad_purpose[COMMAND_FLAGS] = 0xff;
        assert_eq!(
            InputCommand::decode(&bad_purpose),
            Err(InputCommandError::InvalidTextPurpose)
        );
        let mut clear_with_context = InputCommand::set_text_context(1, None).unwrap().encode();
        write_u64(&mut clear_with_context, COMMAND_OWNER_OR_CONTEXT, 1);
        assert_eq!(
            InputCommand::decode(&clear_with_context),
            Err(InputCommandError::UnexpectedPayload)
        );
    }

    #[test]
    fn event_kinds_round_trip_with_canonical_reencoding() {
        let pointer = RoutedPointer {
            input_sequence: 3,
            target: Some(target(9, 4)),
            x: 319,
            y: 479,
            pressed: false,
            captured: true,
            trusted_overlay: false,
        };
        let events = [
            InputEvent::ready(1).unwrap(),
            InputEvent::ack(2, CommandStream::Route, 7).unwrap(),
            InputEvent::routed_pointer(3, pointer).unwrap(),
            InputEvent::routed_key(4, 4, Some(target(9, 4)), InputKey::Backspace, true).unwrap(),
            InputEvent::overlay_shown(5, 22).unwrap(),
            InputEvent::overlay_hidden(6, 22).unwrap(),
            InputEvent::preedit(7, 22, target(9, 4), 1, '中').unwrap(),
            InputEvent::commit(8, 22, target(9, 4), 2, "a中").unwrap(),
            InputEvent::delete_surrounding(9, 22, target(9, 4), 3, 1, 0).unwrap(),
        ];
        for event in events {
            let wire = event.encode();
            assert_eq!(InputEvent::decode(&wire), Ok(event));
            assert_eq!(InputEvent::decode(&wire).unwrap().encode(), wire);
        }
    }

    #[test]
    fn event_header_and_reserved_bytes_are_strict() {
        let canonical = InputEvent::ready(1).unwrap().encode();
        assert_eq!(
            InputEvent::decode(&canonical[..63]),
            Err(InputEventError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, InputEventError::InvalidMagic),
            (4, InputEventError::InvalidVersion),
            (6, InputEventError::InvalidKind),
            (7, InputEventError::NonZeroHeaderReserved),
            (47, InputEventError::NonZeroBodyReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(InputEvent::decode(&wire), Err(expected));
        }
    }

    #[test]
    fn ready_and_ack_reject_foreign_payload_and_invalid_stream() {
        let mut ready = InputEvent::ready(1).unwrap().encode();
        ready[EVENT_RELATED_SEQUENCE] = 1;
        assert_eq!(
            InputEvent::decode(&ready),
            Err(InputEventError::UnexpectedPayload)
        );
        let mut ack = InputEvent::ack(1, CommandStream::Focus, 1)
            .unwrap()
            .encode();
        ack[EVENT_ARGUMENT_0] = 9;
        assert_eq!(
            InputEvent::decode(&ack),
            Err(InputEventError::InvalidCommandStream)
        );
        assert_eq!(
            InputEvent::ack(1, CommandStream::Route, 0),
            Err(InputEventError::ZeroRelatedSequence)
        );
    }

    #[test]
    fn pointer_event_rejects_noncanonical_boolean_flags_and_coordinates() {
        let pointer = RoutedPointer {
            input_sequence: 1,
            target: Some(target(1, 1)),
            x: 1,
            y: 2,
            pressed: true,
            captured: true,
            trusted_overlay: false,
        };
        let canonical = InputEvent::routed_pointer(1, pointer).unwrap().encode();
        let mut bad_bool = canonical;
        bad_bool[EVENT_ARGUMENT_0] = 2;
        assert_eq!(
            InputEvent::decode(&bad_bool),
            Err(InputEventError::InvalidBoolean)
        );
        let mut bad_flags = canonical;
        bad_flags[EVENT_ARGUMENT_1] = 0x80;
        assert_eq!(
            InputEvent::decode(&bad_flags),
            Err(InputEventError::InvalidPointerFlags)
        );
        let mut bad_x = canonical;
        write_u32(
            &mut bad_x,
            EVENT_REVISION_OR_COORDINATES,
            u32::from(SHELL_WIDTH),
        );
        assert_eq!(
            InputEvent::decode(&bad_x),
            Err(InputEventError::CoordinateOutOfBounds)
        );
    }

    #[test]
    fn pointer_without_target_cannot_claim_capture_or_overlay_trust() {
        let pointer = RoutedPointer {
            input_sequence: 1,
            target: None,
            x: 1,
            y: 2,
            pressed: true,
            captured: true,
            trusted_overlay: false,
        };
        assert_eq!(
            InputEvent::routed_pointer(1, pointer),
            Err(InputEventError::UnexpectedPayload)
        );
        let valid = RoutedPointer {
            captured: false,
            ..pointer
        };
        let mut wire = InputEvent::routed_pointer(1, valid).unwrap().encode();
        wire[EVENT_ARGUMENT_1] = POINTER_TRUSTED_OVERLAY;
        assert_eq!(
            InputEvent::decode(&wire),
            Err(InputEventError::UnexpectedPayload)
        );
    }

    #[test]
    fn routed_key_validates_optional_target_closed_key_and_boolean() {
        let canonical = InputEvent::routed_key(1, 1, Some(target(1, 1)), InputKey::A, false)
            .unwrap()
            .encode();
        let mut bad_key = canonical;
        bad_key[EVENT_ARGUMENT_0] = 0;
        assert_eq!(
            InputEvent::decode(&bad_key),
            Err(InputEventError::InvalidKey)
        );
        let mut bad_bool = canonical;
        bad_bool[EVENT_ARGUMENT_1] = 2;
        assert_eq!(
            InputEvent::decode(&bad_bool),
            Err(InputEventError::InvalidBoolean)
        );
        let mut partial_target = canonical;
        write_u64(&mut partial_target, EVENT_GENERATION, 0);
        assert_eq!(
            InputEvent::decode(&partial_target),
            Err(InputEventError::UnexpectedPayload)
        );
    }

    #[test]
    fn routed_key_without_focus_round_trips_as_canonical_zero_target_pair() {
        let event = InputEvent::routed_key(1, 7, None, InputKey::Enter, true).unwrap();
        let wire = event.encode();
        assert_eq!(&wire[EVENT_WINDOW..EVENT_GENERATION + 8], &[0_u8; 16]);
        assert_eq!(InputEvent::decode(&wire), Ok(event));
        assert_eq!(InputEvent::decode(&wire).unwrap().encode(), wire);

        let mut window_without_generation = wire;
        write_u64(&mut window_without_generation, EVENT_WINDOW, 9);
        assert_eq!(
            InputEvent::decode(&window_without_generation),
            Err(InputEventError::UnexpectedPayload)
        );

        let mut generation_without_window = wire;
        write_u64(&mut generation_without_window, EVENT_GENERATION, 4);
        assert_eq!(
            InputEvent::decode(&generation_without_window),
            Err(InputEventError::UnexpectedPayload)
        );
    }

    #[test]
    fn inline_text_is_bounded_utf8_with_zero_tail() {
        assert_eq!(InlineText16::try_from_str(""), Err(InlineTextError::Empty));
        assert_eq!(
            InlineText16::try_from_str("12345678901234567"),
            Err(InlineTextError::TooLong)
        );
        let canonical = InputEvent::commit(1, 1, target(1, 1), 1, "你")
            .unwrap()
            .encode();
        let mut invalid_utf8 = canonical;
        invalid_utf8[EVENT_TEXT] = 0xff;
        assert_eq!(
            InputEvent::decode(&invalid_utf8),
            Err(InputEventError::InvalidText(InlineTextError::InvalidUtf8))
        );
        let mut nonzero_tail = canonical;
        nonzero_tail[63] = 1;
        assert_eq!(
            InputEvent::decode(&nonzero_tail),
            Err(InputEventError::InvalidText(InlineTextError::NonZeroTail))
        );
    }

    #[test]
    fn preedit_is_exactly_one_scalar_and_text_revisions_are_nonzero() {
        assert_eq!(
            InputEvent::preedit(1, 1, target(1, 1), 0, 'a'),
            Err(InputEventError::ZeroRevision)
        );
        let mut two_scalars = InputEvent::commit(1, 1, target(1, 1), 1, "ab")
            .unwrap()
            .encode();
        two_scalars[HEADER_KIND] = InputEventKind::Preedit.raw();
        assert_eq!(
            InputEvent::decode(&two_scalars),
            Err(InputEventError::InvalidText(
                InlineTextError::NotSingleScalar
            ))
        );
        assert_eq!(
            InputEvent::commit(1, 1, target(1, 1), 1, ""),
            Err(InputEventError::InvalidText(InlineTextError::Empty))
        );
    }

    #[test]
    fn delete_requires_nonzero_context_revision_and_extent() {
        assert_eq!(
            InputEvent::delete_surrounding(1, 0, target(1, 1), 1, 1, 0),
            Err(InputEventError::ZeroContextId)
        );
        assert_eq!(
            InputEvent::delete_surrounding(1, 1, target(1, 1), 0, 1, 0),
            Err(InputEventError::ZeroRevision)
        );
        assert_eq!(
            InputEvent::delete_surrounding(1, 1, target(1, 1), 1, 0, 0),
            Err(InputEventError::EmptyDelete)
        );
    }

    #[test]
    fn overlay_events_require_context_and_no_cross_kind_payload() {
        assert_eq!(
            InputEvent::overlay_shown(1, 0),
            Err(InputEventError::ZeroContextId)
        );
        let mut wire = InputEvent::overlay_hidden(1, 3).unwrap().encode();
        wire[EVENT_ARGUMENT_0] = 1;
        assert_eq!(
            InputEvent::decode(&wire),
            Err(InputEventError::UnexpectedPayload)
        );
    }

    #[test]
    fn legacy_bic1_and_bie1_representative_wires_are_sealed() {
        let mut expected_command = [0_u8; 64];
        expected_command[..4].copy_from_slice(b"BIC1");
        expected_command[4..6].copy_from_slice(&1_u16.to_le_bytes());
        expected_command[6] = 1;
        expected_command[8..16].copy_from_slice(&1_u64.to_le_bytes());
        assert_eq!(
            InputCommand::snapshot_reset(1).unwrap().encode(),
            expected_command
        );

        let mut expected_event = [0_u8; 64];
        expected_event[..4].copy_from_slice(b"BIE1");
        expected_event[4..6].copy_from_slice(&1_u16.to_le_bytes());
        expected_event[6] = 1;
        expected_event[8..16].copy_from_slice(&1_u64.to_le_bytes());
        assert_eq!(InputEvent::ready(1).unwrap().encode(), expected_event);
        assert_eq!(InputCommandKind::from_raw(8), None);
        assert_eq!(InputEventKind::from_raw(10), None);
    }

    #[test]
    fn every_route_control_kind_round_trips_canonically() {
        let session = 0x99;
        let surface = OwnerPid::try_new(0x2000_0006).unwrap();
        let metrics = InputRouteGapMetrics::try_new(3, 1, 1, 2, 1).unwrap();
        let controls = [
            InputRouteControl::acquired(1, session, 0).unwrap(),
            InputRouteControl::bind(1, 1, session, surface, 0).unwrap(),
            InputRouteControl::ready(2, 1, session, surface, 0).unwrap(),
            InputRouteControl::route_lost(3, 1, session, 1, None).unwrap(),
            InputRouteControl::route_lost(4, 1, session, 1, Some(capture(false))).unwrap(),
            InputRouteControl::route_lost(5, 1, session, 1, Some(capture(true))).unwrap(),
            InputRouteControl::gap_status(6, 2, session, 1, metrics).unwrap(),
        ];
        assert_eq!(
            controls.map(InputRouteControl::kind),
            [
                InputRouteControlKind::Acquired,
                InputRouteControlKind::Bind,
                InputRouteControlKind::Ready,
                InputRouteControlKind::RouteLost,
                InputRouteControlKind::RouteLostClientCapture,
                InputRouteControlKind::RouteLostTrustedCapture,
                InputRouteControlKind::GapStatus,
            ]
        );
        for control in controls {
            let wire = control.encode();
            assert_eq!(InputRouteControl::decode(&wire), Ok(control));
            assert_eq!(InputRouteControl::decode(&wire).unwrap().encode(), wire);
        }
    }

    #[test]
    fn route_control_offsets_and_directions_are_exact() {
        let bind = InputRouteControl::bind(
            2,
            2,
            0x1112_1314_1516_1718,
            OwnerPid::try_new(0x2122_2324_2526_2728).unwrap(),
            0x3132_3334_3536_3738,
        )
        .unwrap()
        .encode();
        assert_eq!(&bind[..4], b"BIR1");
        assert_eq!(read_u16(&bind, 4), 1);
        assert_eq!(bind[6], InputRouteControlKind::Bind.raw());
        assert_eq!(bind[7], 0);
        assert_eq!(read_u64(&bind, 8), 2);
        assert_eq!(read_u64(&bind, 16), 2);
        assert_eq!(read_u64(&bind, 24), 0x1112_1314_1516_1718);
        assert_eq!(read_u64(&bind, 32), 0x2122_2324_2526_2728);
        assert_eq!(read_u64(&bind, 40), 0x3132_3334_3536_3738);
        assert_eq!(read_u64(&bind, 48), 16);
        assert_eq!(read_u64(&bind, 56), 64);

        assert_eq!(
            InputRouteControlKind::Bind.direction(),
            InputRouteControlDirection::InitToInput
        );
        for kind in [
            InputRouteControlKind::Acquired,
            InputRouteControlKind::Ready,
            InputRouteControlKind::RouteLost,
            InputRouteControlKind::RouteLostClientCapture,
            InputRouteControlKind::RouteLostTrustedCapture,
            InputRouteControlKind::GapStatus,
        ] {
            assert_eq!(kind.direction(), InputRouteControlDirection::InputToInit);
        }
    }

    #[test]
    fn route_control_header_length_and_reserved_bytes_are_strict() {
        let canonical = InputRouteControl::acquired(1, 7, 0).unwrap().encode();
        assert_eq!(
            InputRouteControl::decode(&canonical[..63]),
            Err(InputRouteControlError::InvalidWireLength)
        );
        let mut oversized = [0_u8; 65];
        oversized[..64].copy_from_slice(&canonical);
        assert_eq!(
            InputRouteControl::decode(&oversized),
            Err(InputRouteControlError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, InputRouteControlError::InvalidMagic),
            (4, InputRouteControlError::InvalidVersion),
            (6, InputRouteControlError::InvalidKind),
            (7, InputRouteControlError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(InputRouteControl::decode(&wire), Err(expected));
        }
        for offset in ROUTE_CONTROL_VALUE_1..INPUT_ROUTE_CONTROL_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                InputRouteControl::decode(&wire),
                Err(InputRouteControlError::UnexpectedPayload)
            );
        }
    }

    #[test]
    fn route_control_rejects_noncanonical_epoch_session_and_capacity() {
        let surface = OwnerPid::try_new(6).unwrap();
        assert_eq!(
            InputRouteControl::acquired(0, 1, 0),
            Err(InputRouteControlError::ZeroSequence)
        );
        assert_eq!(
            InputRouteControl::acquired(1, 0, 0),
            Err(InputRouteControlError::ZeroInputSession)
        );
        assert_eq!(
            InputRouteControl::bind(1, 0, 1, surface, 0),
            Err(InputRouteControlError::UnexpectedRouteEpoch)
        );
        let independent_bind = InputRouteControl::bind(2, 3, 1, surface, 0).unwrap();
        assert_eq!(independent_bind.sequence(), 2);
        assert_eq!(independent_bind.route_epoch(), 3);
        assert_eq!(
            InputRouteControl::decode(&independent_bind.encode()),
            Ok(independent_bind)
        );

        let mut acquired = InputRouteControl::acquired(1, 1, 0).unwrap().encode();
        write_u64(&mut acquired, ROUTE_CONTROL_EPOCH, 1);
        assert_eq!(
            InputRouteControl::decode(&acquired),
            Err(InputRouteControlError::UnexpectedRouteEpoch)
        );
        let canonical = InputRouteControl::bind(1, 1, 1, surface, 0)
            .unwrap()
            .encode();
        let mut capacity = canonical;
        write_u64(&mut capacity, ROUTE_CONTROL_VALUE_2, 15);
        assert_eq!(
            InputRouteControl::decode(&capacity),
            Err(InputRouteControlError::InvalidGapCapacity)
        );
        let mut budget = canonical;
        write_u64(&mut budget, ROUTE_CONTROL_VALUE_3, 63);
        assert_eq!(
            InputRouteControl::decode(&budget),
            Err(InputRouteControlError::InvalidPhysicalBudget)
        );
        let mut zero_pid = canonical;
        write_u64(&mut zero_pid, ROUTE_CONTROL_VALUE_0, 0);
        assert_eq!(
            InputRouteControl::decode(&zero_pid),
            Err(InputRouteControlError::InvalidIdentity(
                IdentityError::ZeroOwnerPid
            ))
        );
    }

    #[test]
    fn route_lost_capture_is_complete_and_kind_qualifies_trust() {
        let session = 9;
        for trusted in [false, true] {
            let control =
                InputRouteControl::route_lost(1, 1, session, 7, Some(capture(trusted))).unwrap();
            let decoded = InputRouteControl::decode(&control.encode()).unwrap();
            let InputRouteControlPayload::RouteLost {
                physical_sequence_floor,
                capture: Some(decoded_capture),
            } = decoded.payload()
            else {
                panic!("capture route-lost payload expected");
            };
            assert_eq!(physical_sequence_floor, 7);
            assert_eq!(decoded_capture.trusted_overlay(), trusted);
            assert_eq!(decoded_capture.target(), target(9, 4));
        }

        let mut missing_owner =
            InputRouteControl::route_lost(1, 1, session, 7, Some(capture(false)))
                .unwrap()
                .encode();
        write_u64(&mut missing_owner, ROUTE_CONTROL_VALUE_1, 0);
        assert_eq!(
            InputRouteControl::decode(&missing_owner),
            Err(InputRouteControlError::InvalidIdentity(
                IdentityError::ZeroOwnerPid
            ))
        );
        let mut foreign_payload = InputRouteControl::route_lost(1, 1, session, 7, None)
            .unwrap()
            .encode();
        write_u64(&mut foreign_payload, ROUTE_CONTROL_VALUE_2, 1);
        assert_eq!(
            InputRouteControl::decode(&foreign_payload),
            Err(InputRouteControlError::UnexpectedPayload)
        );
    }

    #[test]
    fn gap_metrics_and_wire_packing_are_strict() {
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 2, 0, 0, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 0, 2, 1, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 0, 1, 17, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 0, 1, 1, 2),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 1, 1, 1, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(65, 0, 1, 1, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(1, 0, 0, 1, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );
        assert_eq!(
            InputRouteGapMetrics::try_new(0, 0, 0, 1, 0),
            Err(InputRouteControlError::InvalidGapMetrics)
        );

        let metrics = InputRouteGapMetrics::try_new(3, 1, 1, 2, 1).unwrap();
        let wire = InputRouteControl::gap_status(1, 2, 8, 9, metrics)
            .unwrap()
            .encode();
        assert_eq!(read_u64(&wire, ROUTE_CONTROL_VALUE_0), 9);
        assert_eq!(read_u64(&wire, ROUTE_CONTROL_VALUE_1), 3);
        assert_eq!(read_u64(&wire, ROUTE_CONTROL_VALUE_2), 1);
        assert_eq!(read_u16(&wire, ROUTE_CONTROL_GAP_PENDING), 1);
        assert_eq!(read_u16(&wire, ROUTE_CONTROL_GAP_HIGH_WATER), 2);
        assert_eq!(read_u16(&wire, ROUTE_CONTROL_GAP_COALESCED), 1);
        assert_eq!(read_u16(&wire, ROUTE_CONTROL_GAP_CAPACITY), 16);

        let mut bad_capacity = wire;
        write_u16(&mut bad_capacity, ROUTE_CONTROL_GAP_CAPACITY, 15);
        assert_eq!(
            InputRouteControl::decode(&bad_capacity),
            Err(InputRouteControlError::InvalidGapCapacity)
        );
    }

    #[test]
    fn route_control_tracker_orders_both_directions_independently() {
        let session = 9;
        let surface = OwnerPid::try_new(6).unwrap();
        let acquired = InputRouteControl::acquired(1, session, 0).unwrap();
        let ready = InputRouteControl::ready(2, 1, session, surface, 0).unwrap();
        let lost = InputRouteControl::route_lost(3, 1, session, 1, None).unwrap();
        let bind1 = InputRouteControl::bind(1, 1, session, surface, 0).unwrap();
        let bind2 = InputRouteControl::bind(2, 2, session, surface, 1).unwrap();
        let mut tracker = InputRouteControlSequenceTracker::new();
        tracker.accept(acquired).unwrap();
        tracker.accept(bind1).unwrap();
        tracker.accept(ready).unwrap();
        tracker.accept(bind2).unwrap();
        assert_eq!(
            tracker.last_sequence(InputRouteControlDirection::InputToInit),
            2
        );
        assert_eq!(
            tracker.last_sequence(InputRouteControlDirection::InitToInput),
            2
        );

        assert_eq!(
            tracker.accept(acquired),
            Err(InputRouteControlTrackerError::SequenceReplay {
                direction: InputRouteControlDirection::InputToInit,
                last: 2,
            })
        );
        let before = tracker;
        assert_eq!(
            tracker.accept(InputRouteControl::route_lost(4, 1, session, 1, None).unwrap()),
            Err(InputRouteControlTrackerError::SequenceGap {
                direction: InputRouteControlDirection::InputToInit,
                expected: 3,
            })
        );
        assert_eq!(tracker, before);
        tracker.accept(lost).unwrap();
    }

    #[test]
    fn route_control_tracker_exhaustion_is_transactional() {
        let mut tracker = InputRouteControlSequenceTracker {
            init_to_input_last: 0,
            input_to_init_last: u64::MAX,
        };
        let control = InputRouteControl::acquired(1, 1, 0).unwrap();
        let before = tracker;
        assert_eq!(
            tracker.accept(control),
            Err(InputRouteControlTrackerError::SequenceExhausted {
                direction: InputRouteControlDirection::InputToInit,
            })
        );
        assert_eq!(tracker, before);
    }
}
