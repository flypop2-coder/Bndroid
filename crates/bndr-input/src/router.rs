//! Allocation-free, generation-qualified input routing state.

use bndr_abi::{InputEvent as PhysicalInputEvent, InputEventPayload as PhysicalInputPayload};
use bndr_ui::{InputSample, SHELL_HEIGHT, SHELL_WIDTH};

use crate::protocol::{INPUT_GAP_PHYSICAL_BUDGET, INPUT_GAP_QUEUE_CAPACITY, InputRouteGapMetrics};

/// The intended production route-table capacity.
pub const DEFAULT_ROUTE_CAPACITY: usize = 8;

/// Stable SurfaceServer-assigned window name. Allocation lifetime is carried
/// separately by [`WindowRef::generation`].
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InputWindowId(u64);

impl InputWindowId {
    pub fn try_new(raw: u64) -> Result<Self, IdentityError> {
        if raw == 0 {
            return Err(IdentityError::ZeroWindowId);
        }
        Ok(Self(raw))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One nonzero process identity supplied by the authenticated outer channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnerPid(u64);

impl OwnerPid {
    pub fn try_new(raw: u64) -> Result<Self, IdentityError> {
        if raw == 0 {
            return Err(IdentityError::ZeroOwnerPid);
        }
        Ok(Self(raw))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    ZeroWindowId,
    ZeroGeneration,
    ZeroOwnerPid,
}

/// A window name qualified by its allocation generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowRef {
    window_id: InputWindowId,
    generation: u64,
}

impl WindowRef {
    pub fn try_new(window_id: InputWindowId, generation: u64) -> Result<Self, IdentityError> {
        if generation == 0 {
            return Err(IdentityError::ZeroGeneration);
        }
        Ok(Self {
            window_id,
            generation,
        })
    }

    pub const fn window_id(self) -> InputWindowId {
        self.window_id
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// Nonempty route bounds inside the normalized phone scanout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteBounds {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl RouteBounds {
    pub fn try_new(x: u16, y: u16, width: u16, height: u16) -> Result<Self, BoundsError> {
        if width == 0 || height == 0 {
            return Err(BoundsError::Empty);
        }
        let right = x.checked_add(width).ok_or(BoundsError::OutOfBounds)?;
        let bottom = y.checked_add(height).ok_or(BoundsError::OutOfBounds)?;
        if right > SHELL_WIDTH || bottom > SHELL_HEIGHT {
            return Err(BoundsError::OutOfBounds);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub const fn x(self) -> u16 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }

    pub const fn width(self) -> u16 {
        self.width
    }

    pub const fn height(self) -> u16 {
        self.height
    }

    pub const fn contains(self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundsError {
    Empty,
    OutOfBounds,
}

/// One complete, immutable route-table row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRoute {
    target: WindowRef,
    owner_pid: OwnerPid,
    bounds: RouteBounds,
    z_order: i32,
    visible: bool,
    focusable: bool,
    trusted_overlay: bool,
}

impl InputRoute {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        target: WindowRef,
        owner_pid: OwnerPid,
        bounds: RouteBounds,
        z_order: i32,
        visible: bool,
        focusable: bool,
        trusted_overlay: bool,
    ) -> Result<Self, RouteDefinitionError> {
        if trusted_overlay && focusable {
            return Err(RouteDefinitionError::TrustedOverlayMustNotFocus);
        }
        Ok(Self {
            target,
            owner_pid,
            bounds,
            z_order,
            visible,
            focusable,
            trusted_overlay,
        })
    }

    pub const fn target(self) -> WindowRef {
        self.target
    }

    pub const fn owner_pid(self) -> OwnerPid {
        self.owner_pid
    }

    pub const fn bounds(self) -> RouteBounds {
        self.bounds
    }

    pub const fn z_order(self) -> i32 {
        self.z_order
    }

    pub const fn is_visible(self) -> bool {
        self.visible
    }

    pub const fn is_focusable(self) -> bool {
        self.focusable
    }

    pub const fn is_trusted_overlay(self) -> bool {
        self.trusted_overlay
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteDefinitionError {
    TrustedOverlayMustNotFocus,
}

/// Closed set of physical/semantic keys routed by the first InputServer ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InputKey {
    A = 1,
    Backspace = 2,
    Enter = 3,
}

impl InputKey {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::A),
            2 => Some(Self::Backspace),
            3 => Some(Self::Enter),
            _ => None,
        }
    }
}

/// Closed text-purpose vocabulary; arbitrary integers are never accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TextPurpose {
    Normal = 1,
    Password = 2,
    Email = 3,
    Phone = 4,
}

impl TextPurpose {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Normal),
            2 => Some(Self::Password),
            3 => Some(Self::Email),
            4 => Some(Self::Phone),
            _ => None,
        }
    }
}

/// Active editable context. Context ids are scoped by the target generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextContext {
    target: WindowRef,
    context_id: u64,
    purpose: TextPurpose,
}

impl TextContext {
    pub fn try_new(
        target: WindowRef,
        context_id: u64,
        purpose: TextPurpose,
    ) -> Result<Self, TextContextError> {
        if context_id == 0 {
            return Err(TextContextError::ZeroContextId);
        }
        Ok(Self {
            target,
            context_id,
            purpose,
        })
    }

    pub const fn target(self) -> WindowRef {
        self.target
    }

    pub const fn context_id(self) -> u64 {
        self.context_id
    }

    pub const fn purpose(self) -> TextPurpose {
        self.purpose
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextContextError {
    ZeroContextId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceError {
    Zero,
    Replay { last: u64 },
    Gap { expected: u64 },
    Exhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouterError {
    Sequence(SequenceError),
    CapacityExceeded,
    UnknownWindow,
    StaleGeneration,
    OwnerChangedWithoutGeneration,
    TrustChangedWithoutGeneration,
    TargetNotVisible,
    TargetNotFocusable,
    TrustedOverlayCannotFocus,
    TextContextDoesNotMatchFocus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutedPointer {
    pub input_sequence: u64,
    pub target: Option<WindowRef>,
    pub x: u16,
    pub y: u16,
    pub pressed: bool,
    pub captured: bool,
    pub trusted_overlay: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutedKey {
    pub input_sequence: u64,
    pub target: Option<WindowRef>,
    pub key: InputKey,
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputCaptureSnapshot {
    target: WindowRef,
    owner_pid: OwnerPid,
    trusted_overlay: bool,
}

impl InputCaptureSnapshot {
    pub(crate) const fn new(target: WindowRef, owner_pid: OwnerPid, trusted_overlay: bool) -> Self {
        Self {
            target,
            owner_pid,
            trusted_overlay,
        }
    }

    pub const fn target(self) -> WindowRef {
        self.target
    }

    pub const fn owner_pid(self) -> OwnerPid {
        self.owner_pid
    }

    pub const fn trusted_overlay(self) -> bool {
        self.trusted_overlay
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub struct SurfaceEpochInputSnapshot {
    physical_sequence_floor: u64,
    capture: Option<InputCaptureSnapshot>,
}

impl SurfaceEpochInputSnapshot {
    pub const fn physical_sequence_floor(self) -> u64 {
        self.physical_sequence_floor
    }

    pub const fn capture(self) -> Option<InputCaptureSnapshot> {
        self.capture
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerDeviceState {
    device_id: u8,
    pressed: bool,
}

impl PointerDeviceState {
    pub fn try_new(device_id: u8, pressed: bool) -> Result<Self, InputGapQueueError> {
        if device_id == 0 {
            return Err(InputGapQueueError::InvalidPointerDevice);
        }
        Ok(Self { device_id, pressed })
    }

    pub const fn device_id(self) -> u8 {
        self.device_id
    }

    pub const fn pressed(self) -> bool {
        self.pressed
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferedInputEvent {
    first_sequence: u64,
    event: PhysicalInputEvent,
    pointer_edge: bool,
}

impl BufferedInputEvent {
    pub const fn first_sequence(self) -> u64 {
        self.first_sequence
    }

    pub const fn last_sequence(self) -> u64 {
        self.event.sequence()
    }

    pub const fn event(self) -> PhysicalInputEvent {
        self.event
    }

    pub const fn pointer_edge(self) -> bool {
        self.pointer_edge
    }

    pub const fn physical_count(self) -> u64 {
        self.last_sequence() - self.first_sequence + 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputGapEnqueue {
    Stored,
    Coalesced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputGapQueueError {
    Sequence(SequenceError),
    Backpressured,
    InvalidPointerDevice,
    UnexpectedPointerDevice { expected: u8, observed: u8 },
}

pub type InputGapQueueMetrics = InputRouteGapMetrics;

/// Allocation-free recovery queue for the authenticated physical-input stream.
///
/// The queue stores at most 16 semantic entries and observes at most 64
/// physical events per recovery gap. Pointer edges and every key remain
/// distinct. Adjacent motion from the one bound pointer device may coalesce,
/// but its complete physical sequence span is retained for ordered replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputGapQueue {
    entries: [Option<BufferedInputEvent>; INPUT_GAP_QUEUE_CAPACITY],
    head: usize,
    len: usize,
    physical_floor: u64,
    last_sequence: u64,
    pointer_state: Option<PointerDeviceState>,
    physical_enqueued: u64,
    physical_dequeued: u64,
    logical_high_water: u16,
    coalesced: u16,
}

impl InputGapQueue {
    pub const fn new(
        physical_sequence_floor: u64,
        pointer_state: Option<PointerDeviceState>,
    ) -> Self {
        Self {
            entries: [None; INPUT_GAP_QUEUE_CAPACITY],
            head: 0,
            len: 0,
            physical_floor: physical_sequence_floor,
            last_sequence: physical_sequence_floor,
            pointer_state,
            physical_enqueued: 0,
            physical_dequeued: 0,
            logical_high_water: 0,
            coalesced: 0,
        }
    }

    pub const fn can_read_capability(&self) -> bool {
        self.len < INPUT_GAP_QUEUE_CAPACITY
            && self.physical_enqueued < INPUT_GAP_PHYSICAL_BUDGET as u64
            && self.last_sequence != u64::MAX
    }

    pub fn enqueue(
        &mut self,
        event: PhysicalInputEvent,
    ) -> Result<InputGapEnqueue, InputGapQueueError> {
        check_next(self.last_sequence, event.sequence()).map_err(InputGapQueueError::Sequence)?;
        if !self.can_read_capability() {
            return Err(InputGapQueueError::Backpressured);
        }

        let mut next_pointer_state = self.pointer_state;
        let pointer_edge = match event.payload() {
            PhysicalInputPayload::Pointer { pressed, .. } => {
                let observed = event.device_id();
                let previous_pressed = match self.pointer_state {
                    Some(previous) if previous.device_id == observed => previous.pressed,
                    Some(previous) => {
                        return Err(InputGapQueueError::UnexpectedPointerDevice {
                            expected: previous.device_id,
                            observed,
                        });
                    }
                    None => false,
                };
                next_pointer_state = Some(PointerDeviceState {
                    device_id: observed,
                    pressed,
                });
                previous_pressed != pressed
            }
            PhysicalInputPayload::Key { .. } => false,
        };

        let tail_index = if self.len == 0 {
            None
        } else {
            Some((self.head + self.len - 1) % INPUT_GAP_QUEUE_CAPACITY)
        };
        let coalesce = tail_index.is_some_and(|index| {
            self.entries[index].is_some_and(|previous| can_coalesce(previous, event, pointer_edge))
        });
        if coalesce {
            let index = tail_index.expect("nonempty queue has a tail");
            let previous = self.entries[index].expect("tail entry is occupied");
            self.entries[index] = Some(BufferedInputEvent {
                first_sequence: previous.first_sequence,
                event,
                pointer_edge: false,
            });
            self.coalesced += 1;
        } else {
            let index = (self.head + self.len) % INPUT_GAP_QUEUE_CAPACITY;
            self.entries[index] = Some(BufferedInputEvent {
                first_sequence: event.sequence(),
                event,
                pointer_edge,
            });
            self.len += 1;
            self.logical_high_water = self.logical_high_water.max(self.len as u16);
        }
        self.pointer_state = next_pointer_state;
        self.last_sequence = event.sequence();
        self.physical_enqueued += 1;
        Ok(if coalesce {
            InputGapEnqueue::Coalesced
        } else {
            InputGapEnqueue::Stored
        })
    }

    pub const fn front(&self) -> Option<BufferedInputEvent> {
        if self.len == 0 {
            None
        } else {
            self.entries[self.head]
        }
    }

    pub fn pop_front(&mut self) -> Option<BufferedInputEvent> {
        let event = self.front()?;
        self.entries[self.head] = None;
        self.head = (self.head + 1) % INPUT_GAP_QUEUE_CAPACITY;
        self.len -= 1;
        self.physical_dequeued += event.physical_count();
        Some(event)
    }

    pub fn metrics(&self) -> InputGapQueueMetrics {
        InputRouteGapMetrics::try_new(
            self.physical_enqueued,
            self.physical_dequeued,
            self.len as u16,
            self.logical_high_water,
            self.coalesced,
        )
        .expect("gap queue metrics preserve their construction invariants")
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn physical_sequence_floor(&self) -> u64 {
        self.physical_floor
    }

    pub const fn last_enqueued_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub const fn pointer_state(&self) -> Option<PointerDeviceState> {
        self.pointer_state
    }
}

fn can_coalesce(
    previous: BufferedInputEvent,
    current: PhysicalInputEvent,
    current_pointer_edge: bool,
) -> bool {
    if previous.pointer_edge || current_pointer_edge {
        return false;
    }
    match (previous.event.payload(), current.payload()) {
        (
            PhysicalInputPayload::Pointer {
                pressed: previous_pressed,
                ..
            },
            PhysicalInputPayload::Pointer {
                pressed: current_pressed,
                ..
            },
        ) => {
            previous.event.device_id() == current.device_id() && previous_pressed == current_pressed
        }
        _ => false,
    }
}

/// Fixed-capacity InputServer routing core.
///
/// Route, focus/control, and device-input streams have independent contiguous
/// sequences. The first accepted value is 1. A rejected call is transactional:
/// neither its sequence nor any table/focus/capture state is committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputRouter<const N: usize = DEFAULT_ROUTE_CAPACITY> {
    routes: [Option<InputRoute>; N],
    /// Highest generation observed for each stable window name. A removed row
    /// remains as a tombstone until the next authoritative snapshot reset.
    latest_generations: [Option<WindowRef>; N],
    route_count: usize,
    focus: Option<WindowRef>,
    capture: Option<WindowRef>,
    text_context: Option<TextContext>,
    last_route_sequence: u64,
    last_focus_sequence: u64,
    last_input_sequence: u64,
}

impl<const N: usize> InputRouter<N> {
    pub const fn new() -> Self {
        Self::with_physical_floor(0)
    }

    /// Creates one completely empty route/focus epoch whose authenticated
    /// physical stream resumes strictly after `physical_sequence_floor`.
    ///
    /// This is a construction-only InputServer recovery boundary: there is no
    /// corresponding method that can jump the floor of a live router.
    pub const fn with_physical_floor(physical_sequence_floor: u64) -> Self {
        Self {
            routes: [None; N],
            latest_generations: [None; N],
            route_count: 0,
            focus: None,
            capture: None,
            text_context: None,
            last_route_sequence: 0,
            last_focus_sequence: 0,
            last_input_sequence: physical_sequence_floor,
        }
    }

    pub const fn route_count(&self) -> usize {
        self.route_count
    }

    pub const fn focused(&self) -> Option<WindowRef> {
        self.focus
    }

    pub const fn captured(&self) -> Option<WindowRef> {
        self.capture
    }

    pub const fn text_context(&self) -> Option<TextContext> {
        self.text_context
    }

    pub const fn last_route_sequence(&self) -> u64 {
        self.last_route_sequence
    }

    pub const fn last_focus_sequence(&self) -> u64 {
        self.last_focus_sequence
    }

    pub const fn last_input_sequence(&self) -> u64 {
        self.last_input_sequence
    }

    pub fn route(&self, window_id: InputWindowId) -> Option<InputRoute> {
        self.routes
            .iter()
            .flatten()
            .copied()
            .find(|route| route.target.window_id == window_id)
    }

    /// Reports whether the committed scene still contains a visible trusted
    /// input overlay.  This is independent of text-context activation: focus
    /// can clear the context before SurfaceServer transactionally removes the
    /// corresponding visual route.
    pub fn has_visible_trusted_overlay(&self) -> bool {
        self.routes
            .iter()
            .flatten()
            .any(|route| route.visible && route.trusted_overlay)
    }

    /// Starts a new SurfaceServer-owned scene epoch without rebasing the
    /// authenticated physical-device stream.
    ///
    /// Route and focus/control sequences are scoped to one SurfaceServer
    /// session, while physical input is owned by the longer-lived InputServer.
    /// A SurfaceServer replacement must therefore discard all scene objects
    /// and generation tombstones but continue accepting exactly the next
    /// physical sequence.
    pub fn reset_surface_epoch(&mut self) {
        let _ = self.begin_surface_recovery();
    }

    /// Atomically snapshots the input state that must be reconciled by a
    /// replacement SurfaceServer and starts its empty scene epoch.
    pub fn begin_surface_recovery(&mut self) -> SurfaceEpochInputSnapshot {
        let capture = self.capture.map(|target| {
            let route = self
                .route_exact(target)
                .expect("captured target always names one committed route");
            InputCaptureSnapshot::new(target, route.owner_pid, route.trusted_overlay)
        });
        let snapshot = SurfaceEpochInputSnapshot {
            physical_sequence_floor: self.last_input_sequence,
            capture,
        };
        self.routes = [None; N];
        self.latest_generations = [None; N];
        self.route_count = 0;
        self.focus = None;
        self.capture = None;
        self.text_context = None;
        self.last_route_sequence = 0;
        self.last_focus_sequence = 0;
        snapshot
    }

    /// Advances the ordered route stream at an empty scene barrier without
    /// changing the currently committed routing state.
    pub fn accept_scene_barrier(&mut self, sequence: u64) -> Result<(), RouterError> {
        check_next(self.last_route_sequence, sequence).map_err(RouterError::Sequence)?;
        self.last_route_sequence = sequence;
        Ok(())
    }

    pub fn reset_snapshot(&mut self, sequence: u64) -> Result<(), RouterError> {
        check_next(self.last_route_sequence, sequence).map_err(RouterError::Sequence)?;
        self.routes = [None; N];
        self.latest_generations = [None; N];
        self.route_count = 0;
        self.focus = None;
        self.capture = None;
        self.text_context = None;
        self.last_route_sequence = sequence;
        Ok(())
    }

    pub fn upsert_route(&mut self, sequence: u64, route: InputRoute) -> Result<(), RouterError> {
        check_next(self.last_route_sequence, sequence).map_err(RouterError::Sequence)?;

        if let Some(index) = self.identity_index(route.target.window_id) {
            let latest = self.latest_generations[index].expect("occupied identity index");
            let previous = self.routes[index];
            if route.target.generation < latest.generation
                || (route.target.generation == latest.generation && previous.is_none())
            {
                return Err(RouterError::StaleGeneration);
            }
            if let Some(previous) = previous
                && route.target.generation == previous.target.generation
            {
                if route.owner_pid != previous.owner_pid {
                    return Err(RouterError::OwnerChangedWithoutGeneration);
                }
                if route.trusted_overlay != previous.trusted_overlay {
                    return Err(RouterError::TrustChangedWithoutGeneration);
                }
            } else if let Some(previous) = previous {
                self.clear_target(previous.target);
            }
            if previous.is_none() {
                self.route_count += 1;
            }
            self.routes[index] = Some(route);
            self.latest_generations[index] = Some(route.target);
        } else {
            let Some(index) = self.latest_generations.iter().position(Option::is_none) else {
                return Err(RouterError::CapacityExceeded);
            };
            self.routes[index] = Some(route);
            self.latest_generations[index] = Some(route.target);
            self.route_count += 1;
        }

        if !route.visible {
            self.clear_target(route.target);
        } else if (!route.focusable || route.trusted_overlay) && self.focus == Some(route.target) {
            self.focus = None;
            self.text_context = None;
        }
        self.last_route_sequence = sequence;
        Ok(())
    }

    pub fn remove_route(&mut self, sequence: u64, target: WindowRef) -> Result<(), RouterError> {
        check_next(self.last_route_sequence, sequence).map_err(RouterError::Sequence)?;
        let Some(index) = self.identity_index(target.window_id) else {
            return Err(RouterError::UnknownWindow);
        };
        let Some(current) = self.routes[index] else {
            let latest = self.latest_generations[index].expect("occupied identity index");
            return if target.generation <= latest.generation {
                Err(RouterError::StaleGeneration)
            } else {
                Err(RouterError::UnknownWindow)
            };
        };
        if current.target.generation != target.generation {
            return Err(RouterError::StaleGeneration);
        }
        self.routes[index] = None;
        self.route_count -= 1;
        self.clear_target(target);
        self.last_route_sequence = sequence;
        Ok(())
    }

    pub fn set_focus(
        &mut self,
        sequence: u64,
        target: Option<WindowRef>,
    ) -> Result<(), RouterError> {
        check_next(self.last_focus_sequence, sequence).map_err(RouterError::Sequence)?;
        if let Some(target) = target {
            let route = self.route_exact(target)?;
            if !route.visible {
                return Err(RouterError::TargetNotVisible);
            }
            if route.trusted_overlay {
                return Err(RouterError::TrustedOverlayCannotFocus);
            }
            if !route.focusable {
                return Err(RouterError::TargetNotFocusable);
            }
        }
        if self.focus != target {
            self.text_context = None;
        }
        self.focus = target;
        self.last_focus_sequence = sequence;
        Ok(())
    }

    /// Installs or clears the text context on the same ordered control stream
    /// as focus. A context may only qualify the currently focused generation.
    pub fn set_text_context(
        &mut self,
        sequence: u64,
        context: Option<TextContext>,
    ) -> Result<(), RouterError> {
        check_next(self.last_focus_sequence, sequence).map_err(RouterError::Sequence)?;
        if let Some(context) = context {
            if self.focus != Some(context.target) {
                return Err(RouterError::TextContextDoesNotMatchFocus);
            }
            let route = self.route_exact(context.target)?;
            if !route.visible || !route.focusable || route.trusted_overlay {
                return Err(RouterError::TextContextDoesNotMatchFocus);
            }
        }
        self.text_context = context;
        self.last_focus_sequence = sequence;
        Ok(())
    }

    pub fn route_pointer(&mut self, sample: InputSample) -> Result<RoutedPointer, RouterError> {
        let sequence = sample.sequence();
        check_next(self.last_input_sequence, sequence).map_err(RouterError::Sequence)?;

        let (target, captured, trusted_overlay, focusable) = if let Some(capture) = self.capture {
            let route = self.route_exact(capture)?;
            (Some(capture), true, route.trusted_overlay, route.focusable)
        } else if sample.pressed() {
            let route = self.hit_test(sample.x(), sample.y());
            let target = route.map(InputRoute::target);
            let trusted = route.is_some_and(InputRoute::is_trusted_overlay);
            let focusable = route.is_some_and(InputRoute::is_focusable);
            (target, target.is_some(), trusted, focusable)
        } else {
            (None, false, false, false)
        };

        if self.capture.is_none() && sample.pressed() {
            self.capture = target;
            if focusable && !trusted_overlay && self.focus != target {
                self.focus = target;
                self.text_context = None;
            }
        } else if self.capture.is_some() && !sample.pressed() {
            self.capture = None;
        }
        self.last_input_sequence = sequence;
        Ok(RoutedPointer {
            input_sequence: sequence,
            target,
            x: sample.x(),
            y: sample.y(),
            pressed: sample.pressed(),
            captured,
            trusted_overlay,
        })
    }

    pub fn route_key(
        &mut self,
        sequence: u64,
        key: InputKey,
        pressed: bool,
    ) -> Result<RoutedKey, RouterError> {
        check_next(self.last_input_sequence, sequence).map_err(RouterError::Sequence)?;
        let target = if let Some(focus) = self.focus {
            let route = self.route_exact(focus)?;
            if route.visible && route.focusable && !route.trusted_overlay {
                Some(focus)
            } else {
                None
            }
        } else {
            None
        };
        self.last_input_sequence = sequence;
        Ok(RoutedKey {
            input_sequence: sequence,
            target,
            key,
            pressed,
        })
    }

    /// Advances the authenticated physical-input stream for an event that is
    /// intentionally unsupported by the current semantic vocabulary. This is
    /// a safe drop, not a replay escape hatch: the same contiguous sequence
    /// checks as routed pointer/key events apply transactionally.
    pub fn discard_input(&mut self, sequence: u64) -> Result<(), RouterError> {
        check_next(self.last_input_sequence, sequence).map_err(RouterError::Sequence)?;
        self.last_input_sequence = sequence;
        Ok(())
    }

    fn identity_index(&self, window_id: InputWindowId) -> Option<usize> {
        self.latest_generations
            .iter()
            .position(|slot| slot.is_some_and(|target| target.window_id == window_id))
    }

    fn route_exact(&self, target: WindowRef) -> Result<InputRoute, RouterError> {
        let Some(route) = self.route(target.window_id) else {
            return Err(RouterError::UnknownWindow);
        };
        if route.target.generation != target.generation {
            return Err(RouterError::StaleGeneration);
        }
        Ok(route)
    }

    fn clear_target(&mut self, target: WindowRef) {
        if self.focus == Some(target) {
            self.focus = None;
            self.text_context = None;
        }
        if self.capture == Some(target) {
            self.capture = None;
        }
        if self
            .text_context
            .is_some_and(|context| context.target == target)
        {
            self.text_context = None;
        }
    }

    fn hit_test(&self, x: u16, y: u16) -> Option<InputRoute> {
        let mut winner: Option<InputRoute> = None;
        for candidate in self.routes.iter().flatten().copied() {
            if !candidate.visible || !candidate.bounds.contains(x, y) {
                continue;
            }
            let replaces = winner.is_none_or(|current| {
                (candidate.trusted_overlay && !current.trusted_overlay)
                    || (candidate.trusted_overlay == current.trusted_overlay
                        && (candidate.z_order > current.z_order
                            || (candidate.z_order == current.z_order
                                && candidate.target.window_id > current.target.window_id)))
            });
            if replaces {
                winner = Some(candidate);
            }
        }
        winner
    }
}

impl<const N: usize> Default for InputRouter<N> {
    fn default() -> Self {
        Self::new()
    }
}

fn check_next(last: u64, sequence: u64) -> Result<(), SequenceError> {
    if sequence == 0 {
        return Err(SequenceError::Zero);
    }
    let Some(expected) = last.checked_add(1) else {
        return Err(SequenceError::Exhausted);
    };
    if sequence <= last {
        return Err(SequenceError::Replay { last });
    }
    if sequence != expected {
        return Err(SequenceError::Gap { expected });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(raw: u64) -> InputWindowId {
        InputWindowId::try_new(raw).unwrap()
    }

    fn target(raw: u64, generation: u64) -> WindowRef {
        WindowRef::try_new(id(raw), generation).unwrap()
    }

    fn bounds(x: u16, y: u16, width: u16, height: u16) -> RouteBounds {
        RouteBounds::try_new(x, y, width, height).unwrap()
    }

    fn route(raw: u64, generation: u64, z: i32) -> InputRoute {
        InputRoute::try_new(
            target(raw, generation),
            OwnerPid::try_new(raw + 100).unwrap(),
            bounds(0, 0, 100, 100),
            z,
            true,
            true,
            false,
        )
        .unwrap()
    }

    fn overlay(raw: u64, generation: u64, z: i32) -> InputRoute {
        InputRoute::try_new(
            target(raw, generation),
            OwnerPid::try_new(raw + 100).unwrap(),
            bounds(10, 10, 40, 40),
            z,
            true,
            false,
            true,
        )
        .unwrap()
    }

    fn sample(sequence: u64, x: u16, y: u16, pressed: bool) -> InputSample {
        InputSample::try_new(sequence, x, y, pressed).unwrap()
    }

    fn physical_pointer(
        sequence: u64,
        device_id: u8,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> PhysicalInputEvent {
        PhysicalInputEvent::try_pointer(sequence, device_id, x, y, pressed).unwrap()
    }

    fn physical_key(sequence: u64, device_id: u8, code: u16) -> PhysicalInputEvent {
        PhysicalInputEvent::try_key(
            sequence,
            device_id,
            code,
            PhysicalInputEvent::KEY_VALUE_PRESS,
        )
        .unwrap()
    }

    #[test]
    fn semantic_identities_and_bounds_reject_zero_empty_and_overflow() {
        assert_eq!(InputWindowId::try_new(0), Err(IdentityError::ZeroWindowId));
        assert_eq!(OwnerPid::try_new(0), Err(IdentityError::ZeroOwnerPid));
        assert_eq!(
            WindowRef::try_new(id(1), 0),
            Err(IdentityError::ZeroGeneration)
        );
        assert_eq!(RouteBounds::try_new(0, 0, 0, 1), Err(BoundsError::Empty));
        assert_eq!(
            RouteBounds::try_new(SHELL_WIDTH - 1, 0, 2, 1),
            Err(BoundsError::OutOfBounds)
        );
        assert_eq!(
            RouteBounds::try_new(0, SHELL_HEIGHT - 1, 1, 2),
            Err(BoundsError::OutOfBounds)
        );
    }

    #[test]
    fn trusted_overlay_definition_cannot_be_focusable() {
        assert_eq!(
            InputRoute::try_new(
                target(1, 1),
                OwnerPid::try_new(2).unwrap(),
                bounds(0, 0, 1, 1),
                0,
                true,
                true,
                true,
            ),
            Err(RouteDefinitionError::TrustedOverlayMustNotFocus)
        );
    }

    #[test]
    fn visible_trusted_overlay_presence_tracks_committed_route_state() {
        let mut router = InputRouter::<2>::new();
        assert!(!router.has_visible_trusted_overlay());
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        assert!(!router.has_visible_trusted_overlay());
        router.upsert_route(2, overlay(2, 1, 1)).unwrap();
        assert!(router.has_visible_trusted_overlay());
        router.remove_route(3, target(2, 1)).unwrap();
        assert!(!router.has_visible_trusted_overlay());
    }

    #[test]
    fn route_sequences_reject_zero_replay_and_gap_transactionally() {
        let mut router = InputRouter::<2>::new();
        assert_eq!(
            router.upsert_route(2, route(1, 1, 0)),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 1 }))
        );
        assert_eq!(router.route_count(), 0);
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        assert_eq!(
            router.remove_route(1, target(1, 1)),
            Err(RouterError::Sequence(SequenceError::Replay { last: 1 }))
        );
        assert_eq!(router.route_count(), 1);
        assert_eq!(
            router.remove_route(0, target(1, 1)),
            Err(RouterError::Sequence(SequenceError::Zero))
        );
        assert_eq!(router.last_route_sequence(), 1);
    }

    #[test]
    fn capacity_failure_does_not_consume_expected_sequence() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        assert_eq!(
            router.upsert_route(2, route(2, 1, 1)),
            Err(RouterError::CapacityExceeded)
        );
        assert_eq!(router.last_route_sequence(), 1);
        router.remove_route(2, target(1, 1)).unwrap();
        router.upsert_route(3, route(1, 2, 1)).unwrap();
    }

    #[test]
    fn zero_capacity_router_rejects_first_upsert_without_commit() {
        let mut router = InputRouter::<0>::new();
        assert_eq!(
            router.upsert_route(1, route(1, 1, 0)),
            Err(RouterError::CapacityExceeded)
        );
        assert_eq!(router.last_route_sequence(), 0);
    }

    #[test]
    fn stale_generation_and_unknown_removal_are_transactional() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 2, 0)).unwrap();
        assert_eq!(
            router.upsert_route(2, route(1, 1, 0)),
            Err(RouterError::StaleGeneration)
        );
        assert_eq!(
            router.remove_route(2, target(2, 1)),
            Err(RouterError::UnknownWindow)
        );
        assert_eq!(
            router.remove_route(2, target(1, 1)),
            Err(RouterError::StaleGeneration)
        );
        assert_eq!(router.last_route_sequence(), 1);
        router.remove_route(2, target(1, 2)).unwrap();
    }

    #[test]
    fn removal_tombstone_rejects_generation_revival_until_newer_allocation() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 4, 0)).unwrap();
        router.remove_route(2, target(1, 4)).unwrap();
        assert_eq!(
            router.upsert_route(3, route(1, 4, 0)),
            Err(RouterError::StaleGeneration)
        );
        assert_eq!(router.last_route_sequence(), 2);
        router.upsert_route(3, route(1, 5, 0)).unwrap();
        assert_eq!(router.route(id(1)).unwrap().target(), target(1, 5));
    }

    #[test]
    fn tombstones_are_identity_capacity_and_snapshot_reset_starts_a_new_epoch() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.remove_route(2, target(1, 1)).unwrap();
        assert_eq!(
            router.upsert_route(3, route(2, 1, 0)),
            Err(RouterError::CapacityExceeded)
        );
        router.reset_snapshot(3).unwrap();
        router.upsert_route(4, route(2, 1, 0)).unwrap();
        assert_eq!(router.route_count(), 1);
    }

    #[test]
    fn owner_and_trust_changes_require_a_new_generation() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        let changed_owner = InputRoute::try_new(
            target(1, 1),
            OwnerPid::try_new(999).unwrap(),
            bounds(0, 0, 100, 100),
            1,
            true,
            true,
            false,
        )
        .unwrap();
        assert_eq!(
            router.upsert_route(2, changed_owner),
            Err(RouterError::OwnerChangedWithoutGeneration)
        );
        let changed_trust = overlay(1, 1, 5);
        assert_eq!(
            router.upsert_route(2, changed_trust),
            Err(RouterError::TrustChangedWithoutGeneration)
        );
        let changed_trust_same_owner = InputRoute::try_new(
            target(1, 1),
            OwnerPid::try_new(101).unwrap(),
            bounds(10, 10, 40, 40),
            5,
            true,
            false,
            true,
        )
        .unwrap();
        assert_eq!(
            router.upsert_route(2, changed_trust_same_owner),
            Err(RouterError::TrustChangedWithoutGeneration)
        );
        router.upsert_route(2, overlay(1, 2, 5)).unwrap();
    }

    #[test]
    fn only_visible_focusable_app_routes_can_receive_focus() {
        let mut router = InputRouter::<3>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.upsert_route(2, overlay(2, 1, 5)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        assert_eq!(router.focused(), Some(target(1, 1)));
        assert_eq!(
            router.set_focus(2, Some(target(2, 1))),
            Err(RouterError::TrustedOverlayCannotFocus)
        );
        assert_eq!(router.last_focus_sequence(), 1);
        assert_eq!(router.focused(), Some(target(1, 1)));
    }

    #[test]
    fn focus_and_text_context_share_one_contiguous_control_stream() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        let context = TextContext::try_new(target(1, 1), 7, TextPurpose::Normal).unwrap();
        router.set_text_context(2, Some(context)).unwrap();
        assert_eq!(router.text_context(), Some(context));
        router.set_focus(3, None).unwrap();
        assert_eq!(router.text_context(), None);
    }

    #[test]
    fn mismatched_text_context_rejection_preserves_state_and_sequence() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.upsert_route(2, route(2, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        let bad = TextContext::try_new(target(2, 1), 7, TextPurpose::Normal).unwrap();
        assert_eq!(
            router.set_text_context(2, Some(bad)),
            Err(RouterError::TextContextDoesNotMatchFocus)
        );
        assert_eq!(router.last_focus_sequence(), 1);
        assert_eq!(router.text_context(), None);
    }

    #[test]
    fn higher_z_route_wins_and_ties_are_deterministic() {
        let mut router = InputRouter::<3>::new();
        router.upsert_route(1, route(1, 1, 1)).unwrap();
        router.upsert_route(2, route(2, 1, 2)).unwrap();
        let routed = router.route_pointer(sample(1, 20, 20, true)).unwrap();
        assert_eq!(routed.target, Some(target(2, 1)));
        router.route_pointer(sample(2, 20, 20, false)).unwrap();
        let same_z = InputRoute::try_new(
            target(1, 1),
            OwnerPid::try_new(101).unwrap(),
            bounds(0, 0, 100, 100),
            2,
            true,
            true,
            false,
        )
        .unwrap();
        router.upsert_route(3, same_z).unwrap();
        let routed = router.route_pointer(sample(3, 20, 20, true)).unwrap();
        assert_eq!(routed.target, Some(target(2, 1)));
    }

    #[test]
    fn trusted_overlay_wins_even_below_app_z_without_stealing_focus() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, i32::MAX)).unwrap();
        router.upsert_route(2, overlay(2, 1, i32::MIN)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        let routed = router.route_pointer(sample(1, 20, 20, true)).unwrap();
        assert_eq!(routed.target, Some(target(2, 1)));
        assert!(routed.trusted_overlay);
        assert_eq!(router.focused(), Some(target(1, 1)));
    }

    #[test]
    fn pointer_capture_survives_motion_and_release_outside_bounds() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        assert_eq!(
            router
                .route_pointer(sample(1, 20, 20, true))
                .unwrap()
                .target,
            Some(target(1, 1))
        );
        assert_eq!(router.captured(), Some(target(1, 1)));
        assert_eq!(
            router
                .route_pointer(sample(2, 200, 200, true))
                .unwrap()
                .target,
            Some(target(1, 1))
        );
        let release = router.route_pointer(sample(3, 200, 200, false)).unwrap();
        assert_eq!(release.target, Some(target(1, 1)));
        assert!(release.captured);
        assert_eq!(router.captured(), None);
    }

    #[test]
    fn orphan_release_is_accepted_but_has_no_target() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        let release = router.route_pointer(sample(1, 20, 20, false)).unwrap();
        assert_eq!(release.target, None);
        assert_eq!(router.last_input_sequence(), 1);
    }

    #[test]
    fn input_replay_and_gap_do_not_change_capture() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();
        assert_eq!(
            router.route_pointer(sample(3, 200, 200, false)),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 2 }))
        );
        assert_eq!(
            router.route_pointer(sample(1, 200, 200, false)),
            Err(RouterError::Sequence(SequenceError::Replay { last: 1 }))
        );
        assert_eq!(router.captured(), Some(target(1, 1)));
        assert_eq!(router.last_input_sequence(), 1);
    }

    #[test]
    fn unsupported_input_is_dropped_without_breaking_the_sequence() {
        let mut router = InputRouter::<1>::new();
        router.discard_input(1).unwrap();
        assert_eq!(router.last_input_sequence(), 1);
        assert_eq!(
            router.discard_input(3),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 2 }))
        );
        assert_eq!(router.last_input_sequence(), 1);
        router.discard_input(2).unwrap();
        assert_eq!(router.last_input_sequence(), 2);
    }

    #[test]
    fn route_replacement_and_removal_clear_generation_bound_state() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();
        router.upsert_route(2, route(1, 2, 0)).unwrap();
        assert_eq!(router.focused(), None);
        assert_eq!(router.captured(), None);
        router.set_focus(2, Some(target(1, 2))).unwrap();
        router.remove_route(3, target(1, 2)).unwrap();
        assert_eq!(router.focused(), None);
    }

    #[test]
    fn hiding_same_generation_route_clears_focus_capture_and_context() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        router
            .set_text_context(
                2,
                Some(TextContext::try_new(target(1, 1), 9, TextPurpose::Email).unwrap()),
            )
            .unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();
        let hidden = InputRoute::try_new(
            target(1, 1),
            OwnerPid::try_new(101).unwrap(),
            bounds(0, 0, 100, 100),
            0,
            false,
            true,
            false,
        )
        .unwrap();
        router.upsert_route(2, hidden).unwrap();
        assert_eq!(
            (router.focused(), router.captured(), router.text_context()),
            (None, None, None)
        );
    }

    #[test]
    fn keys_route_only_to_the_single_focused_generation() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        assert_eq!(router.route_key(1, InputKey::A, true).unwrap().target, None);
        router.set_focus(1, Some(target(1, 1))).unwrap();
        let key = router.route_key(2, InputKey::Enter, false).unwrap();
        assert_eq!(key.target, Some(target(1, 1)));
        assert_eq!(key.key, InputKey::Enter);
    }

    #[test]
    fn snapshot_reset_clears_all_live_state() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();
        router.reset_snapshot(2).unwrap();
        assert_eq!(router.route_count(), 0);
        assert_eq!((router.focused(), router.captured()), (None, None));
        assert_eq!(router.last_route_sequence(), 2);
    }

    #[test]
    fn surface_epoch_reset_preserves_the_physical_sequence_floor() {
        let mut router = InputRouter::<2>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        router
            .set_text_context(
                2,
                Some(TextContext::try_new(target(1, 1), 9, TextPurpose::Normal).unwrap()),
            )
            .unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();

        router.reset_surface_epoch();
        assert_eq!(router.route_count(), 0);
        assert_eq!(
            (router.focused(), router.captured(), router.text_context()),
            (None, None, None)
        );
        assert_eq!(router.last_route_sequence(), 0);
        assert_eq!(router.last_focus_sequence(), 0);
        assert_eq!(router.last_input_sequence(), 1);

        assert_eq!(
            router.route_key(3, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 2 }))
        );
        assert_eq!(
            router.route_key(1, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Replay { last: 1 }))
        );
        assert_eq!(router.last_input_sequence(), 1);

        router.upsert_route(1, route(2, 1, 0)).unwrap();
        router.set_focus(1, Some(target(2, 1))).unwrap();
        let key = router.route_key(2, InputKey::A, true).unwrap();
        assert_eq!(key.target, Some(target(2, 1)));
        assert_eq!(router.last_input_sequence(), 2);
    }

    #[test]
    fn recovery_constructor_is_empty_and_accepts_only_floor_successor() {
        let mut router = InputRouter::<2>::with_physical_floor(41);
        assert_eq!(router.route_count(), 0);
        assert_eq!(router.focused(), None);
        assert_eq!(router.captured(), None);
        assert_eq!(router.text_context(), None);
        assert_eq!(router.last_route_sequence(), 0);
        assert_eq!(router.last_focus_sequence(), 0);
        assert_eq!(router.last_input_sequence(), 41);

        assert_eq!(
            router.route_key(41, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Replay { last: 41 }))
        );
        assert_eq!(
            router.route_key(43, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 42 }))
        );
        assert_eq!(router.last_input_sequence(), 41);
        let routed = router.route_key(42, InputKey::A, true).unwrap();
        assert_eq!(routed.input_sequence, 42);
        assert_eq!(routed.target, None);
        assert_eq!(router.last_input_sequence(), 42);
    }

    #[test]
    fn maximum_recovery_floor_is_explicitly_exhausted() {
        let mut router = InputRouter::<1>::with_physical_floor(u64::MAX);
        let before = router;
        assert_eq!(
            router.route_key(1, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Exhausted))
        );
        assert_eq!(router, before);
        assert_eq!(router.last_input_sequence(), u64::MAX);
    }

    #[test]
    fn scene_barriers_advance_only_the_route_stream() {
        let mut router = InputRouter::<1>::new();
        router.upsert_route(1, route(1, 1, 0)).unwrap();
        router.set_focus(1, Some(target(1, 1))).unwrap();
        router.route_key(1, InputKey::A, true).unwrap();

        router.accept_scene_barrier(2).unwrap();
        assert_eq!(router.route_count(), 1);
        assert_eq!(router.focused(), Some(target(1, 1)));
        assert_eq!(router.last_route_sequence(), 2);
        assert_eq!(router.last_focus_sequence(), 1);
        assert_eq!(router.last_input_sequence(), 1);
        assert_eq!(
            router.accept_scene_barrier(4),
            Err(RouterError::Sequence(SequenceError::Gap { expected: 3 }))
        );
        router.accept_scene_barrier(3).unwrap();
    }

    #[test]
    fn exhausted_sequences_reject_without_mutation() {
        let mut router = InputRouter::<1>::new();
        router.last_route_sequence = u64::MAX;
        assert_eq!(
            router.reset_snapshot(1),
            Err(RouterError::Sequence(SequenceError::Exhausted))
        );
        router.last_focus_sequence = u64::MAX;
        assert_eq!(
            router.set_focus(1, None),
            Err(RouterError::Sequence(SequenceError::Exhausted))
        );
        router.last_input_sequence = u64::MAX;
        assert_eq!(
            router.route_key(1, InputKey::A, true),
            Err(RouterError::Sequence(SequenceError::Exhausted))
        );
    }

    #[test]
    fn surface_recovery_atomically_snapshots_client_capture_and_floor() {
        let mut router = InputRouter::<2>::new();
        let client_route = route(1, 3, 0);
        router.upsert_route(1, client_route).unwrap();
        router.set_focus(1, Some(client_route.target())).unwrap();
        router
            .set_text_context(
                2,
                Some(TextContext::try_new(client_route.target(), 7, TextPurpose::Normal).unwrap()),
            )
            .unwrap();
        router.route_pointer(sample(1, 20, 20, true)).unwrap();

        let snapshot = router.begin_surface_recovery();
        assert_eq!(snapshot.physical_sequence_floor(), 1);
        assert_eq!(
            snapshot.capture(),
            Some(InputCaptureSnapshot::new(
                client_route.target(),
                client_route.owner_pid(),
                false,
            ))
        );
        assert_eq!(router.route_count(), 0);
        assert_eq!(
            (router.focused(), router.captured(), router.text_context()),
            (None, None, None)
        );
        assert_eq!(router.last_route_sequence(), 0);
        assert_eq!(router.last_focus_sequence(), 0);
        assert_eq!(router.last_input_sequence(), 1);
    }

    #[test]
    fn surface_recovery_distinguishes_trusted_capture_and_no_capture() {
        let mut trusted = InputRouter::<1>::new();
        let trusted_route = overlay(9, 4, 10);
        trusted.upsert_route(1, trusted_route).unwrap();
        trusted.route_pointer(sample(1, 20, 20, true)).unwrap();
        let capture = trusted
            .begin_surface_recovery()
            .capture()
            .expect("trusted pointer is captured");
        assert_eq!(capture.target(), trusted_route.target());
        assert_eq!(capture.owner_pid(), trusted_route.owner_pid());
        assert!(capture.trusted_overlay());

        let mut orphan = InputRouter::<1>::new();
        orphan.discard_input(1).unwrap();
        let snapshot = orphan.begin_surface_recovery();
        assert_eq!(snapshot.physical_sequence_floor(), 1);
        assert_eq!(snapshot.capture(), None);
    }

    #[test]
    fn gap_queue_preserves_edges_and_coalesces_only_adjacent_motion() {
        let pointer = PointerDeviceState::try_new(2, false).unwrap();
        let mut queue = InputGapQueue::new(0, Some(pointer));
        assert_eq!(
            queue.enqueue(physical_pointer(1, 2, 10, 10, true)),
            Ok(InputGapEnqueue::Stored)
        );
        assert_eq!(
            queue.enqueue(physical_pointer(2, 2, 11, 11, true)),
            Ok(InputGapEnqueue::Stored)
        );
        assert_eq!(
            queue.enqueue(physical_pointer(3, 2, 12, 12, true)),
            Ok(InputGapEnqueue::Coalesced)
        );
        assert_eq!(
            queue.enqueue(physical_pointer(4, 2, 13, 13, false)),
            Ok(InputGapEnqueue::Stored)
        );

        let metrics = queue.metrics();
        assert_eq!(metrics.physical_enqueued(), 4);
        assert_eq!(metrics.physical_dequeued(), 0);
        assert_eq!(metrics.logical_pending(), 3);
        assert_eq!(metrics.logical_high_water(), 3);
        assert_eq!(metrics.coalesced(), 1);

        let down = queue.pop_front().unwrap();
        assert_eq!((down.first_sequence(), down.last_sequence()), (1, 1));
        assert!(down.pointer_edge());
        let motion = queue.pop_front().unwrap();
        assert_eq!((motion.first_sequence(), motion.last_sequence()), (2, 3));
        assert!(!motion.pointer_edge());
        assert_eq!(motion.event().pointer(), Some((12, 12, true)));
        let release = queue.pop_front().unwrap();
        assert_eq!((release.first_sequence(), release.last_sequence()), (4, 4));
        assert!(release.pointer_edge());
        assert!(queue.is_empty());
        assert_eq!(queue.metrics().physical_dequeued(), 4);
    }

    #[test]
    fn gap_queue_keys_never_coalesce_or_change_pointer_state() {
        let pointer = PointerDeviceState::try_new(2, true).unwrap();
        let mut queue = InputGapQueue::new(10, Some(pointer));
        queue.enqueue(physical_key(11, 1, 30)).unwrap();
        queue.enqueue(physical_key(12, 1, 30)).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.metrics().coalesced(), 0);
        assert_eq!(queue.pointer_state(), Some(pointer));
        assert_eq!(queue.front().unwrap().event().device_id(), 1);
    }

    #[test]
    fn gap_queue_rejects_foreign_pointer_device_transactionally() {
        assert_eq!(
            PointerDeviceState::try_new(0, false),
            Err(InputGapQueueError::InvalidPointerDevice)
        );
        let pointer = PointerDeviceState::try_new(2, true).unwrap();
        let mut queue = InputGapQueue::new(5, Some(pointer));
        let before = queue;
        assert_eq!(
            queue.enqueue(physical_pointer(6, 3, 10, 10, false)),
            Err(InputGapQueueError::UnexpectedPointerDevice {
                expected: 2,
                observed: 3,
            })
        );
        assert_eq!(queue, before);
    }

    #[test]
    fn gap_queue_sequence_errors_and_backpressure_are_transactional() {
        let mut queue = InputGapQueue::new(5, None);
        let empty = queue;
        assert_eq!(
            queue.enqueue(physical_key(7, 1, 30)),
            Err(InputGapQueueError::Sequence(SequenceError::Gap {
                expected: 6,
            }))
        );
        assert_eq!(queue, empty);
        queue.enqueue(physical_key(6, 1, 30)).unwrap();
        let one = queue;
        assert_eq!(
            queue.enqueue(physical_key(6, 1, 30)),
            Err(InputGapQueueError::Sequence(SequenceError::Replay {
                last: 6,
            }))
        );
        assert_eq!(queue, one);

        for sequence in 7..=21 {
            queue
                .enqueue(physical_key(sequence, 1, sequence as u16))
                .unwrap();
        }
        assert_eq!(queue.len(), INPUT_GAP_QUEUE_CAPACITY);
        assert!(!queue.can_read_capability());
        let full = queue;
        assert_eq!(
            queue.enqueue(physical_key(22, 1, 22)),
            Err(InputGapQueueError::Backpressured)
        );
        assert_eq!(queue, full);
        queue.pop_front().unwrap();
        assert!(queue.can_read_capability());
        queue.enqueue(physical_key(22, 1, 22)).unwrap();
    }

    #[test]
    fn gap_queue_ring_order_survives_pop_and_refill() {
        let mut queue = InputGapQueue::new(0, None);
        for sequence in 1..=16 {
            queue
                .enqueue(physical_key(sequence, 1, sequence as u16))
                .unwrap();
        }
        for expected in 1..=8 {
            assert_eq!(queue.pop_front().unwrap().last_sequence(), expected);
        }
        for sequence in 17..=24 {
            queue
                .enqueue(physical_key(sequence, 1, sequence as u16))
                .unwrap();
        }
        for expected in 9..=24 {
            assert_eq!(queue.pop_front().unwrap().last_sequence(), expected);
        }
        assert!(queue.is_empty());
        assert_eq!(queue.metrics().physical_dequeued(), 24);
        assert_eq!(queue.metrics().logical_high_water(), 16);
    }

    #[test]
    fn gap_queue_physical_budget_bounds_a_coalescing_motion_storm() {
        let pointer = PointerDeviceState::try_new(2, false).unwrap();
        let mut queue = InputGapQueue::new(0, Some(pointer));
        for sequence in 1..=u64::from(INPUT_GAP_PHYSICAL_BUDGET) {
            assert!(
                queue
                    .enqueue(physical_pointer(sequence, 2, 10, 10, false))
                    .is_ok()
            );
        }
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.metrics().coalesced(), INPUT_GAP_PHYSICAL_BUDGET - 1);
        assert!(!queue.can_read_capability());
        let before = queue;
        assert_eq!(
            queue.enqueue(physical_pointer(
                u64::from(INPUT_GAP_PHYSICAL_BUDGET) + 1,
                2,
                11,
                11,
                false,
            )),
            Err(InputGapQueueError::Backpressured)
        );
        assert_eq!(queue, before);
        let span = queue.pop_front().unwrap();
        assert_eq!(span.first_sequence(), 1);
        assert_eq!(span.last_sequence(), u64::from(INPUT_GAP_PHYSICAL_BUDGET));
        assert_eq!(span.physical_count(), u64::from(INPUT_GAP_PHYSICAL_BUDGET));
    }

    #[test]
    fn gap_queue_active_contact_release_remains_an_edge() {
        let pointer = PointerDeviceState::try_new(2, true).unwrap();
        let mut queue = InputGapQueue::new(41, Some(pointer));
        queue
            .enqueue(physical_pointer(42, 2, 200, 200, true))
            .unwrap();
        queue
            .enqueue(physical_pointer(43, 2, 200, 200, false))
            .unwrap();
        assert!(!queue.pop_front().unwrap().pointer_edge());
        assert!(queue.pop_front().unwrap().pointer_edge());
        assert!(!queue.pointer_state().unwrap().pressed());
    }
}
