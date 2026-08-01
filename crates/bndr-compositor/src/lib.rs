#![no_std]
#![deny(unsafe_code)]

//! A bounded, allocation-free compositor core for Bndroid's phone surface.
//!
//! The compositor retains exactly two opaque windows plus one independent,
//! trusted system overlay. Pixel storage remains caller-owned: each successful
//! window or overlay installation lends one complete layer to the compositor,
//! and destruction or removal returns that layer.

pub use bndr_ui::{SURFACE_HEIGHT, SURFACE_WIDTH, ShellRect, WindowId};

/// The protocol and compositor deliberately share the same fixed capacity.
pub const WINDOW_CAPACITY: usize = bndr_ui::WINDOW_CAPACITY;
/// The number of XRGB pixels in the selected fixed output surface.
pub const SURFACE_PIXEL_COUNT: usize = SURFACE_WIDTH as usize * SURFACE_HEIGHT as usize;
/// The canonical opaque background used when no window covers a pixel.
pub const BACKGROUND_COLOR: u32 = 0x0000_0000;

const _: () = assert!(WINDOW_CAPACITY == 2);
#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(SURFACE_WIDTH == 208);
#[cfg(not(feature = "mobile-ui-runtime"))]
const _: () = assert!(SURFACE_HEIGHT == 368);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(SURFACE_WIDTH == 720);
#[cfg(feature = "mobile-ui-runtime")]
const _: () = assert!(SURFACE_HEIGHT == 1_600);

/// A rejected compositor transaction. Every rejection is side-effect free.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositorError {
    ZeroOwner,
    SlotOutOfRange,
    SlotOccupied,
    CapacityExhausted,
    CounterExhausted,
    InvalidGeometry,
    EmptyDamage,
    DamageOutOfBounds,
    NonCanonicalColor,
    LayerLength { expected: usize, actual: usize },
    PixelSourceLength { expected: usize, actual: usize },
    OutputLength { expected: usize, actual: usize },
    StaleWindow,
    WindowNotFound,
    NotOwner,
    GlobalCoordinateOutOfBounds,
    PointerAlreadyCaptured,
    SystemOverlayAlreadyInstalled,
    SystemOverlayNotInstalled,
}

/// Exact pixel accounting for one bounded composition operation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PixelLedger {
    pub content_pixels: u32,
    pub visible_pixels: u32,
    pub occluded_pixels: u32,
    pub recomposed_pixels: u32,
    /// True only when at least one output pixel changed value.
    pub scene_changed: bool,
}

impl PixelLedger {
    pub const EMPTY: Self = Self {
        content_pixels: 0,
        visible_pixels: 0,
        occluded_pixels: 0,
        recomposed_pixels: 0,
        scene_changed: false,
    };
}

/// The result of creating one topmost window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreateReport {
    pub window_id: WindowId,
    pub content_generation: u64,
    pub pixels: PixelLedger,
}

/// A failed create returns the caller's layer immediately.
#[derive(Debug)]
pub struct CreateFailure<'a> {
    error: CompositorError,
    layer: &'a mut [u32],
}

impl<'a> CreateFailure<'a> {
    pub const fn error(&self) -> CompositorError {
        self.error
    }

    pub fn layer(&self) -> &[u32] {
        self.layer
    }

    pub fn into_parts(self) -> (CompositorError, &'a mut [u32]) {
        (self.error, self.layer)
    }
}

/// The result of applying one solid-color local damage transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentReport {
    pub window_id: WindowId,
    pub content_generation: u64,
    pub global_damage: ShellRect,
    pub pixels: PixelLedger,
}

/// Public immutable metadata for the trusted system overlay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemOverlayInfo {
    pub bounds: ShellRect,
    pub content_generation: u64,
}

/// The result of installing or updating the trusted system overlay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemOverlayReport {
    pub content_generation: u64,
    pub global_damage: ShellRect,
    pub pixels: PixelLedger,
}

/// A failed overlay installation returns the caller's layer immediately.
#[derive(Debug)]
pub struct SystemOverlayInstallFailure<'a> {
    error: CompositorError,
    layer: &'a mut [u32],
}

impl<'a> SystemOverlayInstallFailure<'a> {
    pub const fn error(&self) -> CompositorError {
        self.error
    }

    pub fn layer(&self) -> &[u32] {
        self.layer
    }

    pub fn into_parts(self) -> (CompositorError, &'a mut [u32]) {
        (self.error, self.layer)
    }
}

/// A removed overlay together with its returned caller-owned layer.
#[derive(Debug)]
pub struct RemovedSystemOverlay<'a> {
    bounds: ShellRect,
    content_generation: u64,
    pixels: PixelLedger,
    layer: &'a mut [u32],
}

impl<'a> RemovedSystemOverlay<'a> {
    pub const fn bounds(&self) -> ShellRect {
        self.bounds
    }

    pub const fn content_generation(&self) -> u64 {
        self.content_generation
    }

    pub const fn pixels(&self) -> PixelLedger {
        self.pixels
    }

    pub fn layer(&self) -> &[u32] {
        self.layer
    }

    pub fn into_layer(self) -> &'a mut [u32] {
        self.layer
    }
}

/// Public immutable metadata for one retained window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowInfo {
    pub window_id: WindowId,
    pub owner: u64,
    pub bounds: ShellRect,
    pub content_generation: u64,
    /// Zero is bottommost; larger values are closer to the user.
    pub z_index: usize,
}

/// The current keyboard/pointer focus and its monotonic generation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FocusState {
    pub window_id: Option<WindowId>,
    /// Zero means focus has never been assigned.
    pub generation: u64,
}

/// One owner-independent route selected by hit testing or pointer capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerRoute {
    pub window_id: WindowId,
    pub global_x: u16,
    pub global_y: u16,
    pub local_x: i32,
    pub local_y: i32,
    pub pressed: bool,
    pub captured: bool,
    pub focus_generation: u64,
}

/// A successful destroy transaction, including the returned caller layer.
#[derive(Debug)]
pub struct DestroyedWindow<'a> {
    window_id: WindowId,
    owner: u64,
    bounds: ShellRect,
    content_generation: u64,
    pixels: PixelLedger,
    layer: &'a mut [u32],
}

impl<'a> DestroyedWindow<'a> {
    pub const fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub const fn owner(&self) -> u64 {
        self.owner
    }

    pub const fn bounds(&self) -> ShellRect {
        self.bounds
    }

    pub const fn content_generation(&self) -> u64 {
        self.content_generation
    }

    pub const fn pixels(&self) -> PixelLedger {
        self.pixels
    }

    pub fn layer(&self) -> &[u32] {
        self.layer
    }

    pub fn into_layer(self) -> &'a mut [u32] {
        self.layer
    }
}

struct Window<'a> {
    id: WindowId,
    owner: u64,
    bounds: ShellRect,
    content_generation: u64,
    layer: &'a mut [u32],
}

struct SystemOverlay<'a> {
    bounds: ShellRect,
    content_generation: u64,
    layer: &'a mut [u32],
}

struct Slot<'a> {
    /// The last successfully allocated generation, including retired windows.
    generation: u32,
    window: Option<Window<'a>>,
}

#[derive(Clone, Copy)]
enum CreateSlotSelection {
    FirstAvailable,
    Exact(usize),
}

/// Fixed-capacity compositor state. No layer or output pixels are embedded.
pub struct Compositor<'a> {
    slots: [Slot<'a>; WINDOW_CAPACITY],
    /// Slot indices, ordered bottom-to-top; only `[..z_len]` is live.
    z_order: [usize; WINDOW_CAPACITY],
    z_len: usize,
    focused: Option<WindowId>,
    focus_generation: u64,
    captured: Option<WindowId>,
    system_overlay: Option<SystemOverlay<'a>>,
}

impl<'a> Default for Compositor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Compositor<'a> {
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| Slot {
                generation: 0,
                window: None,
            }),
            z_order: [0; WINDOW_CAPACITY],
            z_len: 0,
            focused: None,
            focus_generation: 0,
            captured: None,
            system_overlay: None,
        }
    }

    pub const fn window_count(&self) -> usize {
        self.z_len
    }

    pub const fn focus_state(&self) -> FocusState {
        FocusState {
            window_id: self.focused,
            generation: self.focus_generation,
        }
    }

    pub const fn captured_window(&self) -> Option<WindowId> {
        self.captured
    }

    /// Returns metadata for the independently retained trusted system overlay.
    pub fn system_overlay_info(&self) -> Option<SystemOverlayInfo> {
        self.system_overlay
            .as_ref()
            .map(|overlay| SystemOverlayInfo {
                bounds: overlay.bounds,
                content_generation: overlay.content_generation,
            })
    }

    /// Read-only access to the caller-owned trusted system overlay pixels.
    pub fn system_overlay_layer(&self) -> Option<&[u32]> {
        self.system_overlay.as_ref().map(|overlay| &*overlay.layer)
    }

    /// Tests the overlay's half-open bounds without changing input routing.
    pub fn system_overlay_hit_test(&self, x: u16, y: u16) -> bool {
        self.system_overlay
            .as_ref()
            .is_some_and(|overlay| rect_contains(overlay.bounds, x, y))
    }

    /// Installs the single trusted system overlay above every retained window.
    ///
    /// The layer is tightly packed in row-major order and remains caller-owned.
    /// Every pixel must be canonical XRGB. Any rejection returns the untouched
    /// layer and leaves compositor state and output pixels unchanged.
    pub fn install_system_overlay(
        &mut self,
        bounds: ShellRect,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> Result<SystemOverlayReport, SystemOverlayInstallFailure<'a>> {
        let validation = (|| {
            validate_surface_rect(bounds)?;
            let expected = rect_area_usize(bounds)?;
            if layer.len() != expected {
                return Err(CompositorError::LayerLength {
                    expected,
                    actual: layer.len(),
                });
            }
            for &pixel in layer.iter() {
                validate_color(pixel)?;
            }
            validate_output(output)?;
            if self.system_overlay.is_some() {
                return Err(CompositorError::SystemOverlayAlreadyInstalled);
            }
            rect_area_u32(bounds)
        })();

        let area = match validation {
            Ok(area) => area,
            Err(error) => return Err(SystemOverlayInstallFailure { error, layer }),
        };

        self.system_overlay = Some(SystemOverlay {
            bounds,
            content_generation: 1,
            layer,
        });
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(bounds, output);
        Ok(SystemOverlayReport {
            content_generation: 1,
            global_damage: bounds,
            pixels: PixelLedger {
                content_pixels: area,
                visible_pixels: area,
                occluded_pixels: 0,
                recomposed_pixels,
                scene_changed,
            },
        })
    }

    /// Copies one local XRGB pixel rectangle into the installed system overlay.
    pub fn apply_system_overlay_pixels(
        &mut self,
        local_damage: ShellRect,
        pixels: &[u32],
        output: &mut [u32],
    ) -> Result<SystemOverlayReport, CompositorError> {
        validate_output(output)?;
        let overlay = self
            .system_overlay
            .as_ref()
            .ok_or(CompositorError::SystemOverlayNotInstalled)?;
        let global_damage = validate_local_damage(overlay.bounds, local_damage)?;
        let expected = rect_area_usize(local_damage)?;
        if pixels.len() != expected {
            return Err(CompositorError::PixelSourceLength {
                expected,
                actual: pixels.len(),
            });
        }
        for &pixel in pixels {
            validate_color(pixel)?;
        }
        let content_generation = overlay
            .content_generation
            .checked_add(1)
            .ok_or(CompositorError::CounterExhausted)?;
        let content_pixels = rect_area_u32(local_damage)?;

        {
            let overlay = self
                .system_overlay
                .as_mut()
                .ok_or(CompositorError::SystemOverlayNotInstalled)?;
            copy_overlay_pixels(overlay, local_damage, pixels);
            overlay.content_generation = content_generation;
        }
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(global_damage, output);
        Ok(SystemOverlayReport {
            content_generation,
            global_damage,
            pixels: PixelLedger {
                content_pixels,
                visible_pixels: content_pixels,
                occluded_pixels: 0,
                recomposed_pixels,
                scene_changed,
            },
        })
    }

    /// Removes the system overlay, restores the retained scene, and returns
    /// the caller-owned layer.
    pub fn remove_system_overlay(
        &mut self,
        output: &mut [u32],
    ) -> Result<RemovedSystemOverlay<'a>, CompositorError> {
        if self.system_overlay.is_none() {
            return Err(CompositorError::SystemOverlayNotInstalled);
        }
        validate_output(output)?;
        let overlay = self
            .system_overlay
            .take()
            .ok_or(CompositorError::SystemOverlayNotInstalled)?;
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(overlay.bounds, output);
        Ok(RemovedSystemOverlay {
            bounds: overlay.bounds,
            content_generation: overlay.content_generation,
            pixels: PixelLedger {
                content_pixels: 0,
                visible_pixels: 0,
                occluded_pixels: 0,
                recomposed_pixels,
                scene_changed,
            },
            layer: overlay.layer,
        })
    }

    /// Returns the live z order bottom-to-top without allocating.
    pub fn z_order(&self) -> [Option<WindowId>; WINDOW_CAPACITY] {
        let mut result = [None; WINDOW_CAPACITY];
        let mut rank = 0;
        while rank < self.z_len {
            result[rank] = self.slots[self.z_order[rank]]
                .window
                .as_ref()
                .map(|window| window.id);
            rank += 1;
        }
        result
    }

    /// Creates a fully initialized, topmost opaque window.
    ///
    /// The layer must contain exactly `bounds.width * bounds.height` pixels.
    /// It is filled with `initial_color` before the affected output is
    /// recomposed. A failure returns the untouched layer.
    pub fn create(
        &mut self,
        owner: u64,
        bounds: ShellRect,
        initial_color: u32,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> Result<CreateReport, CreateFailure<'a>> {
        self.create_transaction(
            CreateSlotSelection::FirstAvailable,
            owner,
            bounds,
            initial_color,
            layer,
            output,
        )
    }

    /// Creates a fully initialized, topmost opaque window in an exact slot.
    ///
    /// Unlike [`Self::create`], this operation never falls back to another
    /// slot. This lets a caller preserve a stable logical-owner-to-slot
    /// assignment across destruction and recreation while the generation
    /// still advances monotonically to prevent ABA reuse.
    ///
    /// The slot must be in `0..WINDOW_CAPACITY` and vacant. The layer and
    /// output requirements are identical to [`Self::create`]. Every failure,
    /// including an exhausted generation, returns the untouched layer without
    /// modifying compositor state or output pixels.
    pub fn create_in_slot(
        &mut self,
        slot_index: usize,
        owner: u64,
        bounds: ShellRect,
        initial_color: u32,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> Result<CreateReport, CreateFailure<'a>> {
        self.create_transaction(
            CreateSlotSelection::Exact(slot_index),
            owner,
            bounds,
            initial_color,
            layer,
            output,
        )
    }

    fn create_transaction(
        &mut self,
        selection: CreateSlotSelection,
        owner: u64,
        bounds: ShellRect,
        initial_color: u32,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> Result<CreateReport, CreateFailure<'a>> {
        let validation = (|| {
            validate_owner(owner)?;
            validate_surface_rect(bounds)?;
            validate_color(initial_color)?;
            let expected = rect_area_usize(bounds)?;
            if layer.len() != expected {
                return Err(CompositorError::LayerLength {
                    expected,
                    actual: layer.len(),
                });
            }
            validate_output(output)?;

            let (slot_index, generation) = match selection {
                CreateSlotSelection::FirstAvailable => {
                    let mut saw_empty = false;
                    let mut selected = None;
                    let mut slot_index = 0;
                    while slot_index < WINDOW_CAPACITY {
                        let slot = &self.slots[slot_index];
                        if slot.window.is_none() {
                            saw_empty = true;
                            if let Some(generation) = slot.generation.checked_add(1) {
                                selected = Some((slot_index, generation));
                                break;
                            }
                        }
                        slot_index += 1;
                    }
                    match selected {
                        Some(selected) => selected,
                        None if saw_empty => return Err(CompositorError::CounterExhausted),
                        None => return Err(CompositorError::CapacityExhausted),
                    }
                }
                CreateSlotSelection::Exact(slot_index) => {
                    if slot_index >= WINDOW_CAPACITY {
                        return Err(CompositorError::SlotOutOfRange);
                    }
                    let slot = &self.slots[slot_index];
                    if slot.window.is_some() {
                        return Err(CompositorError::SlotOccupied);
                    }
                    let generation = slot
                        .generation
                        .checked_add(1)
                        .ok_or(CompositorError::CounterExhausted)?;
                    (slot_index, generation)
                }
            };
            let window_id = WindowId::try_new(slot_index, generation)
                .map_err(|_| CompositorError::CounterExhausted)?;
            let area = rect_area_u32(bounds)?;
            Ok((slot_index, generation, window_id, area))
        })();

        let (slot_index, generation, window_id, area) = match validation {
            Ok(validated) => validated,
            Err(error) => return Err(CreateFailure { error, layer }),
        };
        let overlay_occluded_pixels = self
            .system_overlay
            .as_ref()
            .map_or(0, |overlay| rect_intersection_area(bounds, overlay.bounds));
        let visible_pixels = area - overlay_occluded_pixels;

        layer.fill(initial_color);
        self.slots[slot_index].generation = generation;
        self.slots[slot_index].window = Some(Window {
            id: window_id,
            owner,
            bounds,
            content_generation: 1,
            layer,
        });
        self.z_order[self.z_len] = slot_index;
        self.z_len += 1;

        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(bounds, output);
        Ok(CreateReport {
            window_id,
            content_generation: 1,
            pixels: PixelLedger {
                content_pixels: area,
                visible_pixels,
                occluded_pixels: overlay_occluded_pixels,
                recomposed_pixels,
                scene_changed,
            },
        })
    }

    /// Applies one local solid-color damage and immediately recomposes it.
    pub fn apply_present(
        &mut self,
        owner: u64,
        window_id: WindowId,
        local_damage: ShellRect,
        color: u32,
        output: &mut [u32],
    ) -> Result<PresentReport, CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        validate_color(color)?;
        validate_output(output)?;

        let window = self.slots[slot_index]
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?;
        let global_damage = validate_local_damage(window.bounds, local_damage)?;
        let content_generation = window
            .content_generation
            .checked_add(1)
            .ok_or(CompositorError::CounterExhausted)?;
        let content_pixels = rect_area_u32(local_damage)?;
        let visible_pixels = self.visible_pixels(slot_index, global_damage)?;
        let occluded_pixels = content_pixels
            .checked_sub(visible_pixels)
            .ok_or(CompositorError::CounterExhausted)?;

        {
            let window = self.slots[slot_index]
                .window
                .as_mut()
                .ok_or(CompositorError::WindowNotFound)?;
            fill_local_damage(window, local_damage, color);
            window.content_generation = content_generation;
        }
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(global_damage, output);

        Ok(PresentReport {
            window_id,
            content_generation,
            global_damage,
            pixels: PixelLedger {
                content_pixels,
                visible_pixels,
                occluded_pixels,
                recomposed_pixels,
                scene_changed,
            },
        })
    }

    /// Compatibility spelling for [`Self::apply_present`].
    pub fn present(
        &mut self,
        owner: u64,
        window_id: WindowId,
        local_damage: ShellRect,
        color: u32,
        output: &mut [u32],
    ) -> Result<PresentReport, CompositorError> {
        self.apply_present(owner, window_id, local_damage, color, output)
    }

    /// Copies one caller-owned XRGB pixel rectangle into retained content and
    /// immediately recomposes its visible portion.
    ///
    /// The source is tightly packed in row-major order and must contain
    /// exactly `local_damage.width * local_damage.height` canonical XRGB
    /// pixels. All geometry, ownership, generation, source, output, and
    /// counter checks complete before either retained content or output is
    /// changed, so every error is transactionally inert.
    pub fn apply_pixels(
        &mut self,
        owner: u64,
        window_id: WindowId,
        local_damage: ShellRect,
        pixels: &[u32],
        output: &mut [u32],
    ) -> Result<PresentReport, CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        validate_output(output)?;

        let window = self.slots[slot_index]
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?;
        let global_damage = validate_local_damage(window.bounds, local_damage)?;
        let expected = rect_area_usize(local_damage)?;
        if pixels.len() != expected {
            return Err(CompositorError::PixelSourceLength {
                expected,
                actual: pixels.len(),
            });
        }
        for &pixel in pixels {
            validate_color(pixel)?;
        }
        let content_generation = window
            .content_generation
            .checked_add(1)
            .ok_or(CompositorError::CounterExhausted)?;
        let content_pixels = rect_area_u32(local_damage)?;
        let visible_pixels = self.visible_pixels(slot_index, global_damage)?;
        let occluded_pixels = content_pixels
            .checked_sub(visible_pixels)
            .ok_or(CompositorError::CounterExhausted)?;

        {
            let window = self.slots[slot_index]
                .window
                .as_mut()
                .ok_or(CompositorError::WindowNotFound)?;
            copy_local_pixels(window, local_damage, pixels);
            window.content_generation = content_generation;
        }
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(global_damage, output);

        Ok(PresentReport {
            window_id,
            content_generation,
            global_damage,
            pixels: PixelLedger {
                content_pixels,
                visible_pixels,
                occluded_pixels,
                recomposed_pixels,
                scene_changed,
            },
        })
    }

    /// Moves a live owner-qualified window to the top and recomposes its bounds.
    pub fn raise(
        &mut self,
        owner: u64,
        window_id: WindowId,
        output: &mut [u32],
    ) -> Result<PixelLedger, CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        validate_output(output)?;
        let rank = self
            .z_position(slot_index)
            .ok_or(CompositorError::WindowNotFound)?;
        if rank + 1 == self.z_len {
            return Ok(PixelLedger::EMPTY);
        }
        let bounds = self.slots[slot_index]
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?
            .bounds;
        let area = rect_area_u32(bounds)?;

        let mut cursor = rank;
        while cursor + 1 < self.z_len {
            self.z_order[cursor] = self.z_order[cursor + 1];
            cursor += 1;
        }
        self.z_order[self.z_len - 1] = slot_index;
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(bounds, output);
        Ok(PixelLedger {
            content_pixels: 0,
            visible_pixels: 0,
            occluded_pixels: 0,
            recomposed_pixels: area.min(recomposed_pixels),
            scene_changed,
        })
    }

    /// Removes a live owner-qualified window and returns its caller-owned layer.
    pub fn destroy(
        &mut self,
        owner: u64,
        window_id: WindowId,
        output: &mut [u32],
    ) -> Result<DestroyedWindow<'a>, CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        validate_output(output)?;
        let rank = self
            .z_position(slot_index)
            .ok_or(CompositorError::WindowNotFound)?;
        let next_focus_generation = if self.focused == Some(window_id) {
            Some(
                self.focus_generation
                    .checked_add(1)
                    .ok_or(CompositorError::CounterExhausted)?,
            )
        } else {
            None
        };

        let window = self.slots[slot_index]
            .window
            .take()
            .ok_or(CompositorError::WindowNotFound)?;
        let mut cursor = rank;
        while cursor + 1 < self.z_len {
            self.z_order[cursor] = self.z_order[cursor + 1];
            cursor += 1;
        }
        self.z_len -= 1;
        self.z_order[self.z_len] = 0;
        if let Some(generation) = next_focus_generation {
            self.focused = None;
            self.focus_generation = generation;
        }
        if self.captured == Some(window_id) {
            self.captured = None;
        }
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(window.bounds, output);
        let pixels = PixelLedger {
            content_pixels: 0,
            visible_pixels: 0,
            occluded_pixels: 0,
            recomposed_pixels,
            scene_changed,
        };
        Ok(DestroyedWindow {
            window_id: window.id,
            owner: window.owner,
            bounds: window.bounds,
            content_generation: window.content_generation,
            pixels,
            layer: window.layer,
        })
    }

    /// Rebuilds exactly one global damage rectangle from retained layers.
    pub fn compose(
        &self,
        damage: ShellRect,
        output: &mut [u32],
    ) -> Result<PixelLedger, CompositorError> {
        validate_surface_rect(damage)?;
        validate_output(output)?;
        let area = rect_area_u32(damage)?;
        let (recomposed_pixels, scene_changed) = self.recompose_unchecked(damage, output);
        Ok(PixelLedger {
            content_pixels: 0,
            visible_pixels: 0,
            occluded_pixels: 0,
            recomposed_pixels: area.min(recomposed_pixels),
            scene_changed,
        })
    }

    /// Rebuilds the complete fixed output surface.
    pub fn compose_full(&self, output: &mut [u32]) -> Result<PixelLedger, CompositorError> {
        self.compose(ShellRect::new(0, 0, SURFACE_WIDTH, SURFACE_HEIGHT), output)
    }

    /// Resolves metadata only when both generation and owner match.
    pub fn window_info(
        &self,
        owner: u64,
        window_id: WindowId,
    ) -> Result<WindowInfo, CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        let window = self.slots[slot_index]
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?;
        Ok(WindowInfo {
            window_id: window.id,
            owner: window.owner,
            bounds: window.bounds,
            content_generation: window.content_generation,
            z_index: self
                .z_position(slot_index)
                .ok_or(CompositorError::WindowNotFound)?,
        })
    }

    /// Read-only access to retained content for diagnostics or transfer.
    pub fn layer(&self, owner: u64, window_id: WindowId) -> Result<&[u32], CompositorError> {
        let slot_index = self.resolve(owner, window_id)?;
        Ok(self.slots[slot_index]
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?
            .layer)
    }

    /// Assigns focus explicitly. Re-focusing the same window is idempotent.
    pub fn focus(
        &mut self,
        owner: u64,
        window_id: WindowId,
    ) -> Result<FocusState, CompositorError> {
        self.resolve(owner, window_id)?;
        if self.focused == Some(window_id) {
            return Ok(self.focus_state());
        }
        let generation = self
            .focus_generation
            .checked_add(1)
            .ok_or(CompositorError::CounterExhausted)?;
        self.focused = Some(window_id);
        self.focus_generation = generation;
        Ok(self.focus_state())
    }

    /// Finds the topmost opaque window containing one surface coordinate.
    pub fn topmost_hit_test(&self, x: u16, y: u16) -> Option<WindowId> {
        if x >= SURFACE_WIDTH || y >= SURFACE_HEIGHT {
            return None;
        }
        let mut rank = self.z_len;
        while rank != 0 {
            rank -= 1;
            let slot_index = self.z_order[rank];
            let Some(window) = self.slots[slot_index].window.as_ref() else {
                continue;
            };
            if rect_contains(window.bounds, x, y) {
                return Some(window.id);
            }
        }
        None
    }

    /// Hit tests, focuses and captures on a pointer down transition.
    pub fn pointer_down(
        &mut self,
        x: u16,
        y: u16,
    ) -> Result<Option<PointerRoute>, CompositorError> {
        validate_global_coordinate(x, y)?;
        if self.captured.is_some() {
            return Err(CompositorError::PointerAlreadyCaptured);
        }
        let Some(window_id) = self.topmost_hit_test(x, y) else {
            return Ok(None);
        };
        let next_focus_generation = if self.focused == Some(window_id) {
            self.focus_generation
        } else {
            self.focus_generation
                .checked_add(1)
                .ok_or(CompositorError::CounterExhausted)?
        };
        let route = self.pointer_route(window_id, x, y, true, true, next_focus_generation)?;
        self.focused = Some(window_id);
        self.focus_generation = next_focus_generation;
        self.captured = Some(window_id);
        Ok(Some(route))
    }

    /// Routes motion to the captured window even when outside its bounds.
    pub fn pointer_move(&self, x: u16, y: u16) -> Result<Option<PointerRoute>, CompositorError> {
        validate_global_coordinate(x, y)?;
        self.pointer_move_unbounded(i32::from(x), i32::from(y))
    }

    /// Routes captured motion from signed surface coordinates.
    ///
    /// The wire-visible global coordinate is clamped to the bounded surface,
    /// while local coordinates are derived from the original signed values.
    /// This preserves the full drag displacement after a contact leaves the
    /// phone rectangle without admitting an out-of-bounds pointer-down hit.
    pub fn pointer_move_unbounded(
        &self,
        surface_x: i32,
        surface_y: i32,
    ) -> Result<Option<PointerRoute>, CompositorError> {
        let Some(window_id) = self.captured else {
            return Ok(None);
        };
        Ok(Some(self.pointer_route_unbounded(
            window_id,
            surface_x,
            surface_y,
            true,
            true,
            self.focus_generation,
        )?))
    }

    /// Delivers pointer up to the captured window, then releases capture.
    pub fn pointer_up(&mut self, x: u16, y: u16) -> Result<Option<PointerRoute>, CompositorError> {
        validate_global_coordinate(x, y)?;
        self.pointer_up_unbounded(i32::from(x), i32::from(y))
    }

    /// Delivers an unbounded captured pointer-up, then releases capture.
    /// Failed coordinate arithmetic leaves the capture transaction intact.
    pub fn pointer_up_unbounded(
        &mut self,
        surface_x: i32,
        surface_y: i32,
    ) -> Result<Option<PointerRoute>, CompositorError> {
        let Some(window_id) = self.captured else {
            return Ok(None);
        };
        let route = self.pointer_route_unbounded(
            window_id,
            surface_x,
            surface_y,
            false,
            true,
            self.focus_generation,
        )?;
        self.captured = None;
        Ok(Some(route))
    }

    fn resolve(&self, owner: u64, window_id: WindowId) -> Result<usize, CompositorError> {
        validate_owner(owner)?;
        let slot_index = window_id.slot();
        let slot = &self.slots[slot_index];
        if slot.generation != window_id.generation() {
            return Err(CompositorError::StaleWindow);
        }
        let window = slot
            .window
            .as_ref()
            .ok_or(CompositorError::WindowNotFound)?;
        if window.id != window_id {
            return Err(CompositorError::StaleWindow);
        }
        if window.owner != owner {
            return Err(CompositorError::NotOwner);
        }
        Ok(slot_index)
    }

    fn z_position(&self, slot_index: usize) -> Option<usize> {
        let mut rank = 0;
        while rank < self.z_len {
            if self.z_order[rank] == slot_index {
                return Some(rank);
            }
            rank += 1;
        }
        None
    }

    fn visible_pixels(
        &self,
        slot_index: usize,
        global_damage: ShellRect,
    ) -> Result<u32, CompositorError> {
        let target_rank = self
            .z_position(slot_index)
            .ok_or(CompositorError::WindowNotFound)?;
        let right = global_damage.x + global_damage.width;
        let bottom = global_damage.y + global_damage.height;
        let mut visible = 0_u32;
        let mut y = global_damage.y;
        while y < bottom {
            let mut x = global_damage.x;
            while x < right {
                let mut covered = false;
                if self
                    .system_overlay
                    .as_ref()
                    .is_some_and(|overlay| rect_contains(overlay.bounds, x, y))
                {
                    covered = true;
                }
                let mut rank = target_rank + 1;
                while !covered && rank < self.z_len {
                    let upper = self.slots[self.z_order[rank]]
                        .window
                        .as_ref()
                        .ok_or(CompositorError::WindowNotFound)?;
                    if rect_contains(upper.bounds, x, y) {
                        covered = true;
                        break;
                    }
                    rank += 1;
                }
                if !covered {
                    visible = visible
                        .checked_add(1)
                        .ok_or(CompositorError::CounterExhausted)?;
                }
                x += 1;
            }
            y += 1;
        }
        Ok(visible)
    }

    fn recompose_unchecked(&self, damage: ShellRect, output: &mut [u32]) -> (u32, bool) {
        let right = damage.x + damage.width;
        let bottom = damage.y + damage.height;
        let mut recomposed = 0_u32;
        let mut changed = false;
        let mut y = damage.y;
        while y < bottom {
            let mut x = damage.x;
            while x < right {
                let color = self.composed_pixel(x, y);
                let output_index = usize::from(y) * usize::from(SURFACE_WIDTH) + usize::from(x);
                if output[output_index] != color {
                    output[output_index] = color;
                    changed = true;
                }
                // Valid phone-surface geometry makes this counter infallible.
                recomposed += 1;
                x += 1;
            }
            y += 1;
        }
        (recomposed, changed)
    }

    fn composed_pixel(&self, x: u16, y: u16) -> u32 {
        if let Some(overlay) = self
            .system_overlay
            .as_ref()
            .filter(|overlay| rect_contains(overlay.bounds, x, y))
        {
            let local_x = usize::from(x - overlay.bounds.x);
            let local_y = usize::from(y - overlay.bounds.y);
            let index = local_y * usize::from(overlay.bounds.width) + local_x;
            return overlay.layer[index];
        }
        let mut rank = self.z_len;
        while rank != 0 {
            rank -= 1;
            let Some(window) = self.slots[self.z_order[rank]].window.as_ref() else {
                continue;
            };
            if rect_contains(window.bounds, x, y) {
                let local_x = usize::from(x - window.bounds.x);
                let local_y = usize::from(y - window.bounds.y);
                let index = local_y * usize::from(window.bounds.width) + local_x;
                return window.layer[index];
            }
        }
        BACKGROUND_COLOR
    }

    fn pointer_route(
        &self,
        window_id: WindowId,
        x: u16,
        y: u16,
        pressed: bool,
        captured: bool,
        focus_generation: u64,
    ) -> Result<PointerRoute, CompositorError> {
        self.pointer_route_unbounded(
            window_id,
            i32::from(x),
            i32::from(y),
            pressed,
            captured,
            focus_generation,
        )
    }

    fn pointer_route_unbounded(
        &self,
        window_id: WindowId,
        surface_x: i32,
        surface_y: i32,
        pressed: bool,
        captured: bool,
        focus_generation: u64,
    ) -> Result<PointerRoute, CompositorError> {
        let window = self.slots[window_id.slot()]
            .window
            .as_ref()
            .filter(|window| window.id == window_id)
            .ok_or(CompositorError::StaleWindow)?;
        let local_x = surface_x
            .checked_sub(i32::from(window.bounds.x))
            .ok_or(CompositorError::CounterExhausted)?;
        let local_y = surface_y
            .checked_sub(i32::from(window.bounds.y))
            .ok_or(CompositorError::CounterExhausted)?;
        Ok(PointerRoute {
            window_id,
            global_x: surface_x.clamp(0, i32::from(SURFACE_WIDTH) - 1) as u16,
            global_y: surface_y.clamp(0, i32::from(SURFACE_HEIGHT) - 1) as u16,
            local_x,
            local_y,
            pressed,
            captured,
            focus_generation,
        })
    }
}

fn validate_owner(owner: u64) -> Result<(), CompositorError> {
    if owner == 0 {
        return Err(CompositorError::ZeroOwner);
    }
    Ok(())
}

fn validate_color(color: u32) -> Result<(), CompositorError> {
    if color & 0xff00_0000 != 0 {
        return Err(CompositorError::NonCanonicalColor);
    }
    Ok(())
}

fn validate_output(output: &[u32]) -> Result<(), CompositorError> {
    if output.len() != SURFACE_PIXEL_COUNT {
        return Err(CompositorError::OutputLength {
            expected: SURFACE_PIXEL_COUNT,
            actual: output.len(),
        });
    }
    Ok(())
}

fn validate_surface_rect(rect: ShellRect) -> Result<(), CompositorError> {
    if rect.width == 0 || rect.height == 0 {
        return Err(CompositorError::InvalidGeometry);
    }
    let right = rect
        .x
        .checked_add(rect.width)
        .ok_or(CompositorError::InvalidGeometry)?;
    let bottom = rect
        .y
        .checked_add(rect.height)
        .ok_or(CompositorError::InvalidGeometry)?;
    if right > SURFACE_WIDTH || bottom > SURFACE_HEIGHT {
        return Err(CompositorError::InvalidGeometry);
    }
    Ok(())
}

fn validate_local_damage(
    bounds: ShellRect,
    damage: ShellRect,
) -> Result<ShellRect, CompositorError> {
    if damage.width == 0 || damage.height == 0 {
        return Err(CompositorError::EmptyDamage);
    }
    let right = damage
        .x
        .checked_add(damage.width)
        .ok_or(CompositorError::DamageOutOfBounds)?;
    let bottom = damage
        .y
        .checked_add(damage.height)
        .ok_or(CompositorError::DamageOutOfBounds)?;
    if right > bounds.width || bottom > bounds.height {
        return Err(CompositorError::DamageOutOfBounds);
    }
    let x = bounds
        .x
        .checked_add(damage.x)
        .ok_or(CompositorError::DamageOutOfBounds)?;
    let y = bounds
        .y
        .checked_add(damage.y)
        .ok_or(CompositorError::DamageOutOfBounds)?;
    Ok(ShellRect::new(x, y, damage.width, damage.height))
}

fn validate_global_coordinate(x: u16, y: u16) -> Result<(), CompositorError> {
    if x >= SURFACE_WIDTH || y >= SURFACE_HEIGHT {
        return Err(CompositorError::GlobalCoordinateOutOfBounds);
    }
    Ok(())
}

fn rect_area_usize(rect: ShellRect) -> Result<usize, CompositorError> {
    usize::from(rect.width)
        .checked_mul(usize::from(rect.height))
        .ok_or(CompositorError::CounterExhausted)
}

fn rect_area_u32(rect: ShellRect) -> Result<u32, CompositorError> {
    u32::from(rect.width)
        .checked_mul(u32::from(rect.height))
        .ok_or(CompositorError::CounterExhausted)
}

fn rect_contains(rect: ShellRect, x: u16, y: u16) -> bool {
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    x >= rect.x && x < right && y >= rect.y && y < bottom
}

fn rect_intersection_area(first: ShellRect, second: ShellRect) -> u32 {
    let left = first.x.max(second.x);
    let top = first.y.max(second.y);
    let right = (first.x + first.width).min(second.x + second.width);
    let bottom = (first.y + first.height).min(second.y + second.height);
    if left >= right || top >= bottom {
        return 0;
    }
    u32::from(right - left) * u32::from(bottom - top)
}

fn fill_local_damage(window: &mut Window<'_>, damage: ShellRect, color: u32) {
    let right = damage.x + damage.width;
    let bottom = damage.y + damage.height;
    let stride = usize::from(window.bounds.width);
    let mut y = damage.y;
    while y < bottom {
        let row = usize::from(y) * stride;
        let mut x = damage.x;
        while x < right {
            window.layer[row + usize::from(x)] = color;
            x += 1;
        }
        y += 1;
    }
}

fn copy_local_pixels(window: &mut Window<'_>, damage: ShellRect, pixels: &[u32]) {
    let stride = usize::from(window.bounds.width);
    let source_stride = usize::from(damage.width);
    let mut source_row = 0_usize;
    while source_row < usize::from(damage.height) {
        let destination_row = usize::from(damage.y) + source_row;
        let destination_start = destination_row * stride + usize::from(damage.x);
        let source_start = source_row * source_stride;
        window.layer[destination_start..destination_start + source_stride]
            .copy_from_slice(&pixels[source_start..source_start + source_stride]);
        source_row += 1;
    }
}

fn copy_overlay_pixels(overlay: &mut SystemOverlay<'_>, damage: ShellRect, pixels: &[u32]) {
    let stride = usize::from(overlay.bounds.width);
    let source_stride = usize::from(damage.width);
    let mut source_row = 0_usize;
    while source_row < usize::from(damage.height) {
        let destination_row = usize::from(damage.y) + source_row;
        let destination_start = destination_row * stride + usize::from(damage.x);
        let source_start = source_row * source_stride;
        overlay.layer[destination_start..destination_start + source_stride]
            .copy_from_slice(&pixels[source_start..source_start + source_stride]);
        source_row += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::vec;
    use std::vec::Vec;

    const OWNER_A: u64 = 0x11;
    const OWNER_B: u64 = 0x22;
    const RED: u32 = 0x00aa_0000;
    const GREEN: u32 = 0x0000_bb00;
    const BLUE: u32 = 0x0000_00cc;

    fn surface(color: u32) -> Vec<u32> {
        vec![color; SURFACE_PIXEL_COUNT]
    }

    fn pixel(output: &[u32], x: u16, y: u16) -> u32 {
        output[usize::from(y) * usize::from(SURFACE_WIDTH) + usize::from(x)]
    }

    fn create_window<'a>(
        compositor: &mut Compositor<'a>,
        owner: u64,
        bounds: ShellRect,
        color: u32,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> WindowId {
        compositor
            .create(owner, bounds, color, layer, output)
            .expect("create")
            .window_id
    }

    fn create_window_in_slot<'a>(
        compositor: &mut Compositor<'a>,
        slot_index: usize,
        owner: u64,
        bounds: ShellRect,
        color: u32,
        layer: &'a mut [u32],
        output: &mut [u32],
    ) -> WindowId {
        compositor
            .create_in_slot(slot_index, owner, bounds, color, layer, output)
            .expect("create in slot")
            .window_id
    }

    #[test]
    fn empty_core_is_small_and_full_compose_clears_output() {
        assert!(core::mem::size_of::<Compositor<'_>>() < 256);
        let mut output = surface(RED);
        let compositor = Compositor::new();

        let pixels = compositor.compose_full(&mut output).unwrap();

        assert_eq!(pixels.recomposed_pixels, SURFACE_PIXEL_COUNT as u32);
        assert!(pixels.scene_changed);
        assert!(output.iter().all(|pixel| *pixel == BACKGROUND_COLOR));
    }

    #[test]
    fn create_initializes_complete_layer_output_and_metadata() {
        let mut layer = [0xffff_ffff; 6];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(2, 3, 3, 2);

        let report = compositor
            .create(OWNER_A, bounds, RED, &mut layer, &mut output)
            .unwrap();

        assert_eq!(report.window_id.slot(), 0);
        assert_eq!(report.window_id.generation(), 1);
        assert_eq!(report.content_generation, 1);
        assert_eq!(report.pixels.content_pixels, 6);
        assert_eq!(report.pixels.visible_pixels, 6);
        assert_eq!(report.pixels.occluded_pixels, 0);
        assert_eq!(report.pixels.recomposed_pixels, 6);
        assert!(report.pixels.scene_changed);
        assert_eq!(
            compositor.layer(OWNER_A, report.window_id).unwrap(),
            &[RED; 6]
        );
        assert_eq!(pixel(&output, 2, 3), RED);
        assert_eq!(pixel(&output, 4, 4), RED);
        assert_eq!(pixel(&output, 1, 3), BACKGROUND_COLOR);
        assert_eq!(
            compositor.window_info(OWNER_A, report.window_id).unwrap(),
            WindowInfo {
                window_id: report.window_id,
                owner: OWNER_A,
                bounds,
                content_generation: 1,
                z_index: 0,
            }
        );
    }

    #[test]
    fn exact_slot_preserves_owner_assignment_when_both_slots_are_empty() {
        let mut app_layer = [0_u32; 1];
        let mut launcher_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();

        let app = create_window_in_slot(
            &mut compositor,
            1,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut app_layer,
            &mut output,
        );
        let launcher = create_window_in_slot(
            &mut compositor,
            0,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut launcher_layer,
            &mut output,
        );

        assert_eq!((app.slot(), app.generation()), (1, 1));
        assert_eq!((launcher.slot(), launcher.generation()), (0, 1));
        assert_eq!(compositor.z_order(), [Some(app), Some(launcher)]);
        assert_eq!(compositor.window_info(OWNER_B, app).unwrap().owner, OWNER_B);
        assert_eq!(
            compositor.window_info(OWNER_A, launcher).unwrap().owner,
            OWNER_A
        );
        assert_eq!(pixel(&output, 0, 0), RED);
        assert_eq!(pixel(&output, 1, 0), BLUE);
    }

    #[test]
    fn exact_app_slot_destroy_recreate_advances_generation_and_rejects_aba() {
        let mut app_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let old = create_window_in_slot(
            &mut compositor,
            1,
            OWNER_B,
            ShellRect::new(4, 5, 1, 1),
            BLUE,
            &mut app_layer,
            &mut output,
        );
        let returned = compositor
            .destroy(OWNER_B, old, &mut output)
            .unwrap()
            .into_layer();

        let fresh = create_window_in_slot(
            &mut compositor,
            1,
            OWNER_B,
            ShellRect::new(4, 5, 1, 1),
            GREEN,
            returned,
            &mut output,
        );

        assert_eq!((old.slot(), old.generation()), (1, 1));
        assert_eq!((fresh.slot(), fresh.generation()), (1, 2));
        assert_ne!(fresh, old);
        assert_eq!(
            compositor.present(OWNER_B, old, ShellRect::new(0, 0, 1, 1), RED, &mut output,),
            Err(CompositorError::StaleWindow)
        );
        assert_eq!(compositor.layer(OWNER_B, fresh).unwrap(), &[GREEN]);
        assert_eq!(pixel(&output, 4, 5), GREEN);
    }

    #[test]
    fn exact_slot_selection_errors_are_transactionally_inert() {
        let mut live_layer = [0_u32; 1];
        let mut occupied_layer = [0x1111_1111_u32; 1];
        let mut out_of_range_layer = [0x2222_2222_u32; 1];
        let mut exhausted_layer = [0x3333_3333_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let live = create_window_in_slot(
            &mut compositor,
            1,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut live_layer,
            &mut output,
        );
        compositor.slots[0].generation = u32::MAX;
        let expected_output = output.clone();
        let expected_z = compositor.z_order();

        let occupied = compositor
            .create_in_slot(
                1,
                OWNER_A,
                ShellRect::new(2, 0, 1, 1),
                RED,
                &mut occupied_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(occupied.error(), CompositorError::SlotOccupied);
        assert_eq!(occupied.layer(), &[0x1111_1111]);

        let out_of_range = compositor
            .create_in_slot(
                WINDOW_CAPACITY,
                OWNER_A,
                ShellRect::new(2, 0, 1, 1),
                RED,
                &mut out_of_range_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(out_of_range.error(), CompositorError::SlotOutOfRange);
        assert_eq!(out_of_range.layer(), &[0x2222_2222]);

        let exhausted = compositor
            .create_in_slot(
                0,
                OWNER_A,
                ShellRect::new(2, 0, 1, 1),
                RED,
                &mut exhausted_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(exhausted.error(), CompositorError::CounterExhausted);
        assert_eq!(exhausted.layer(), &[0x3333_3333]);

        assert_eq!(compositor.window_count(), 1);
        assert_eq!(compositor.z_order(), expected_z);
        assert_eq!(compositor.slots[0].generation, u32::MAX);
        assert_eq!(compositor.slots[1].generation, 1);
        assert_eq!(compositor.layer(OWNER_B, live).unwrap(), &[BLUE]);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn exact_slot_geometry_and_buffers_fail_without_consuming_generation() {
        let mut invalid_geometry_layer = [0x1111_u32; 1];
        let mut wrong_length_layer = [0x2222_u32; 1];
        let mut short_output_layer = [0x3333_u32; 1];
        let mut noncanonical_layer = [0x4444_u32; 1];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();

        let invalid_geometry = compositor
            .create_in_slot(
                0,
                OWNER_A,
                ShellRect::new(0, 0, 0, 1),
                RED,
                &mut invalid_geometry_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(invalid_geometry.error(), CompositorError::InvalidGeometry);
        assert_eq!(invalid_geometry.layer(), &[0x1111]);

        let wrong_length = compositor
            .create_in_slot(
                0,
                OWNER_A,
                ShellRect::new(0, 0, 2, 1),
                RED,
                &mut wrong_length_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(
            wrong_length.error(),
            CompositorError::LayerLength {
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(wrong_length.layer(), &[0x2222]);

        let short_output = compositor
            .create_in_slot(
                0,
                OWNER_A,
                ShellRect::new(0, 0, 1, 1),
                RED,
                &mut short_output_layer,
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            )
            .unwrap_err();
        assert_eq!(
            short_output.error(),
            CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            }
        );
        assert_eq!(short_output.layer(), &[0x3333]);

        let noncanonical = compositor
            .create_in_slot(
                0,
                OWNER_A,
                ShellRect::new(0, 0, 1, 1),
                0xff00_0000,
                &mut noncanonical_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(noncanonical.error(), CompositorError::NonCanonicalColor);
        assert_eq!(noncanonical.layer(), &[0x4444]);

        assert_eq!(compositor.window_count(), 0);
        assert_eq!(compositor.z_order(), [None, None]);
        assert_eq!(compositor.slots[0].generation, 0);
        assert_eq!(compositor.slots[1].generation, 0);
        assert!(output.iter().all(|pixel| *pixel == BLUE));
    }

    #[test]
    fn create_zero_owner_is_inert_and_returns_layer() {
        let mut layer = [0x1234_5678; 4];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();

        let failure = compositor
            .create(0, ShellRect::new(0, 0, 2, 2), RED, &mut layer, &mut output)
            .unwrap_err();

        assert_eq!(failure.error(), CompositorError::ZeroOwner);
        assert_eq!(failure.layer(), &[0x1234_5678; 4]);
        assert_eq!(compositor.window_count(), 0);
        assert!(output.iter().all(|pixel| *pixel == BLUE));
    }

    #[test]
    fn create_rejects_empty_and_overflowing_geometry() {
        let mut empty_layer = [7_u32; 1];
        let mut overflow_layer = [8_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();

        let empty = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 0, 1),
                RED,
                &mut empty_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(empty.error(), CompositorError::InvalidGeometry);
        assert_eq!(empty.layer(), &[7]);

        let overflow = compositor
            .create(
                OWNER_A,
                ShellRect::new(u16::MAX, 0, 1, 1),
                RED,
                &mut overflow_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(overflow.error(), CompositorError::InvalidGeometry);
        assert_eq!(overflow.layer(), &[8]);
        assert_eq!(compositor.window_count(), 0);
    }

    #[test]
    fn create_rejects_wrong_layer_length() {
        let mut layer = [9_u32; 3];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();

        let failure = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 2, 2),
                RED,
                &mut layer,
                &mut output,
            )
            .unwrap_err();

        assert_eq!(
            failure.error(),
            CompositorError::LayerLength {
                expected: 4,
                actual: 3,
            }
        );
        assert_eq!(failure.layer(), &[9; 3]);
        assert_eq!(compositor.window_count(), 0);
    }

    #[test]
    fn create_rejects_wrong_output_length_without_touching_layer() {
        let mut layer = [10_u32; 4];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();

        let failure = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 2, 2),
                RED,
                &mut layer,
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            )
            .unwrap_err();

        assert_eq!(
            failure.error(),
            CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            }
        );
        assert_eq!(failure.layer(), &[10; 4]);
        assert!(output.iter().all(|pixel| *pixel == BLUE));
    }

    #[test]
    fn create_rejects_noncanonical_xrgb() {
        let mut layer = [11_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();

        let failure = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 1, 1),
                0x0100_0000,
                &mut layer,
                &mut output,
            )
            .unwrap_err();

        assert_eq!(failure.error(), CompositorError::NonCanonicalColor);
        assert_eq!(failure.layer(), &[11]);
        assert_eq!(pixel(&output, 0, 0), BACKGROUND_COLOR);
    }

    #[test]
    fn fixed_capacity_two_rejects_third_create_inertly() {
        let mut first_layer = [1_u32; 1];
        let mut second_layer = [2_u32; 1];
        let mut third_layer = [3_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let first = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut first_layer,
            &mut output,
        );
        let second = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut second_layer,
            &mut output,
        );

        let failure = compositor
            .create(
                OWNER_A,
                ShellRect::new(2, 0, 1, 1),
                GREEN,
                &mut third_layer,
                &mut output,
            )
            .unwrap_err();

        assert_eq!(failure.error(), CompositorError::CapacityExhausted);
        assert_eq!(failure.layer(), &[3]);
        assert_eq!(compositor.window_count(), 2);
        assert_eq!(compositor.z_order(), [Some(first), Some(second)]);
        assert_eq!(pixel(&output, 2, 0), BACKGROUND_COLOR);
    }

    #[test]
    fn retired_slot_reuses_next_generation_and_rejects_aba_id() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let old = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        let returned = compositor
            .destroy(OWNER_A, old, &mut output)
            .unwrap()
            .into_layer();
        assert_eq!(
            compositor.present(OWNER_A, old, ShellRect::new(0, 0, 1, 1), GREEN, &mut output,),
            Err(CompositorError::WindowNotFound)
        );

        let fresh = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            BLUE,
            returned,
            &mut output,
        );

        assert_eq!(fresh.slot(), old.slot());
        assert_eq!(fresh.generation(), old.generation() + 1);
        assert_eq!(
            compositor.present(OWNER_A, old, ShellRect::new(0, 0, 1, 1), GREEN, &mut output,),
            Err(CompositorError::StaleWindow)
        );
        assert_eq!(compositor.layer(OWNER_A, fresh).unwrap(), &[BLUE]);
        assert_eq!(pixel(&output, 0, 0), BLUE);
    }

    #[test]
    fn owner_mismatch_present_is_transactionally_inert() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );

        let error = compositor.present(OWNER_B, id, ShellRect::new(0, 0, 2, 2), GREEN, &mut output);

        assert_eq!(error, Err(CompositorError::NotOwner));
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED; 4]);
        assert_eq!(
            compositor
                .window_info(OWNER_A, id)
                .unwrap()
                .content_generation,
            1
        );
        assert_eq!(pixel(&output, 0, 0), RED);
    }

    #[test]
    fn owner_mismatch_raise_and_destroy_preserve_topology() {
        let mut lower_layer = [0_u32; 4];
        let mut upper_layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut lower_layer,
            &mut output,
        );
        let upper = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(0, 0, 2, 2),
            BLUE,
            &mut upper_layer,
            &mut output,
        );

        assert_eq!(
            compositor.raise(OWNER_B, lower, &mut output),
            Err(CompositorError::NotOwner)
        );
        assert_eq!(
            compositor.destroy(OWNER_B, lower, &mut output).unwrap_err(),
            CompositorError::NotOwner
        );
        assert_eq!(compositor.z_order(), [Some(lower), Some(upper)]);
        assert_eq!(pixel(&output, 0, 0), BLUE);
    }

    #[test]
    fn present_rejects_empty_out_of_bounds_and_overflow_damage() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(4, 5, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );

        assert_eq!(
            compositor.present(OWNER_A, id, ShellRect::new(0, 0, 0, 1), GREEN, &mut output,),
            Err(CompositorError::EmptyDamage)
        );
        assert_eq!(
            compositor.present(OWNER_A, id, ShellRect::new(1, 0, 2, 1), GREEN, &mut output,),
            Err(CompositorError::DamageOutOfBounds)
        );
        assert_eq!(
            compositor.present(
                OWNER_A,
                id,
                ShellRect::new(u16::MAX, 0, 1, 1),
                GREEN,
                &mut output,
            ),
            Err(CompositorError::DamageOutOfBounds)
        );
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED; 4]);
        assert_eq!(
            compositor
                .window_info(OWNER_A, id)
                .unwrap()
                .content_generation,
            1
        );
        assert_eq!(pixel(&output, 4, 5), RED);
    }

    #[test]
    fn present_rejects_noncanonical_color_and_short_output() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );

        assert_eq!(
            compositor.present(
                OWNER_A,
                id,
                ShellRect::new(0, 0, 1, 1),
                0xff00_0000,
                &mut output,
            ),
            Err(CompositorError::NonCanonicalColor)
        );
        assert_eq!(
            compositor.present(
                OWNER_A,
                id,
                ShellRect::new(0, 0, 1, 1),
                GREEN,
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            ),
            Err(CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            })
        );
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED]);
        assert_eq!(pixel(&output, 0, 0), RED);
    }

    #[test]
    fn pixel_present_copies_tightly_packed_rows_and_reports_exact_damage() {
        let mut layer = [0_u32; 9];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(10, 20, 3, 3),
            RED,
            &mut layer,
            &mut output,
        );
        let source = [GREEN, BLUE, BLUE, GREEN];

        let report = compositor
            .apply_pixels(
                OWNER_A,
                id,
                ShellRect::new(1, 1, 2, 2),
                &source,
                &mut output,
            )
            .unwrap();

        assert_eq!(report.content_generation, 2);
        assert_eq!(report.global_damage, ShellRect::new(11, 21, 2, 2));
        assert_eq!(
            report.pixels,
            PixelLedger {
                content_pixels: 4,
                visible_pixels: 4,
                occluded_pixels: 0,
                recomposed_pixels: 4,
                scene_changed: true,
            }
        );
        assert_eq!(
            compositor.layer(OWNER_A, id).unwrap(),
            &[RED, RED, RED, RED, GREEN, BLUE, RED, BLUE, GREEN]
        );
        assert_eq!(pixel(&output, 11, 21), GREEN);
        assert_eq!(pixel(&output, 12, 21), BLUE);
        assert_eq!(pixel(&output, 11, 22), BLUE);
        assert_eq!(pixel(&output, 12, 22), GREEN);
    }

    #[test]
    fn pixel_present_rejects_length_and_noncanonical_source_transactionally() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );

        assert_eq!(
            compositor.apply_pixels(
                OWNER_A,
                id,
                ShellRect::new(0, 0, 2, 2),
                &[GREEN; 3],
                &mut output,
            ),
            Err(CompositorError::PixelSourceLength {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            compositor.apply_pixels(
                OWNER_A,
                id,
                ShellRect::new(0, 0, 2, 2),
                &[GREEN, GREEN, 0xff00_0000, GREEN],
                &mut output,
            ),
            Err(CompositorError::NonCanonicalColor)
        );
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED; 4]);
        assert_eq!(
            compositor
                .window_info(OWNER_A, id)
                .unwrap()
                .content_generation,
            1
        );
        assert_eq!(pixel(&output, 0, 0), RED);
        assert_eq!(pixel(&output, 1, 1), RED);
    }

    #[test]
    fn fully_occluded_pixel_present_updates_only_retained_content() {
        let mut lower_layer = [0_u32; 4];
        let mut upper_layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(0, 0, 2, 2);
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            bounds,
            RED,
            &mut lower_layer,
            &mut output,
        );
        create_window(
            &mut compositor,
            OWNER_B,
            bounds,
            BLUE,
            &mut upper_layer,
            &mut output,
        );
        let source = [GREEN, RED, RED, GREEN];

        let report = compositor
            .apply_pixels(
                OWNER_A,
                lower,
                ShellRect::new(0, 0, 2, 2),
                &source,
                &mut output,
            )
            .unwrap();

        assert_eq!(report.pixels.content_pixels, 4);
        assert_eq!(report.pixels.visible_pixels, 0);
        assert_eq!(report.pixels.occluded_pixels, 4);
        assert_eq!(report.pixels.recomposed_pixels, 4);
        assert!(!report.pixels.scene_changed);
        assert_eq!(compositor.layer(OWNER_A, lower).unwrap(), &source);
        assert_eq!(pixel(&output, 0, 0), BLUE);
        assert_eq!(pixel(&output, 1, 1), BLUE);
    }

    #[test]
    fn present_content_counter_overflow_rolls_back() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.slots[id.slot()]
            .window
            .as_mut()
            .unwrap()
            .content_generation = u64::MAX;

        assert_eq!(
            compositor.present(OWNER_A, id, ShellRect::new(0, 0, 1, 1), GREEN, &mut output,),
            Err(CompositorError::CounterExhausted)
        );
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED]);
        assert_eq!(pixel(&output, 0, 0), RED);
    }

    #[test]
    fn fully_occluded_lower_present_updates_retained_layer_only() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 16];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(0, 0, 4, 4);
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            bounds,
            RED,
            &mut lower_layer,
            &mut output,
        );
        create_window(
            &mut compositor,
            OWNER_B,
            bounds,
            BLUE,
            &mut upper_layer,
            &mut output,
        );

        let report = compositor
            .present(
                OWNER_A,
                lower,
                ShellRect::new(0, 0, 4, 4),
                GREEN,
                &mut output,
            )
            .unwrap();

        assert_eq!(report.pixels.content_pixels, 16);
        assert_eq!(report.pixels.visible_pixels, 0);
        assert_eq!(report.pixels.occluded_pixels, 16);
        assert_eq!(report.pixels.recomposed_pixels, 16);
        assert!(!report.pixels.scene_changed);
        assert_eq!(compositor.layer(OWNER_A, lower).unwrap(), &[GREEN; 16]);
        assert_eq!(pixel(&output, 0, 0), BLUE);
        assert_eq!(pixel(&output, 3, 3), BLUE);
    }

    #[test]
    fn partial_occlusion_has_exact_pixel_ledger() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 8];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 4, 4),
            RED,
            &mut lower_layer,
            &mut output,
        );
        create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(2, 0, 2, 4),
            BLUE,
            &mut upper_layer,
            &mut output,
        );

        let report = compositor
            .present(
                OWNER_A,
                lower,
                ShellRect::new(0, 0, 4, 4),
                GREEN,
                &mut output,
            )
            .unwrap();

        assert_eq!(report.pixels.content_pixels, 16);
        assert_eq!(report.pixels.visible_pixels, 8);
        assert_eq!(report.pixels.occluded_pixels, 8);
        assert_eq!(report.pixels.recomposed_pixels, 16);
        assert!(report.pixels.scene_changed);
        assert_eq!(pixel(&output, 0, 3), GREEN);
        assert_eq!(pixel(&output, 1, 3), GREEN);
        assert_eq!(pixel(&output, 2, 3), BLUE);
        assert_eq!(pixel(&output, 3, 3), BLUE);
    }

    #[test]
    fn partial_damage_changes_only_requested_local_pixels() {
        let mut layer = [0_u32; 25];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(10, 20, 5, 5),
            RED,
            &mut layer,
            &mut output,
        );

        let report = compositor
            .present(OWNER_A, id, ShellRect::new(1, 2, 2, 2), GREEN, &mut output)
            .unwrap();

        assert_eq!(report.global_damage, ShellRect::new(11, 22, 2, 2));
        assert_eq!(report.pixels.content_pixels, 4);
        assert_eq!(report.pixels.visible_pixels, 4);
        assert_eq!(report.pixels.occluded_pixels, 0);
        assert_eq!(report.pixels.recomposed_pixels, 4);
        let retained = compositor.layer(OWNER_A, id).unwrap();
        assert_eq!(retained[0], RED);
        assert_eq!(retained[11], GREEN);
        assert_eq!(retained[12], GREEN);
        assert_eq!(retained[16], GREEN);
        assert_eq!(retained[17], GREEN);
        assert_eq!(retained[24], RED);
        assert_eq!(pixel(&output, 10, 22), RED);
        assert_eq!(pixel(&output, 11, 22), GREEN);
        assert_eq!(pixel(&output, 13, 22), RED);
    }

    #[test]
    fn same_color_present_recomposes_without_scene_change() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );

        let report = compositor
            .present(OWNER_A, id, ShellRect::new(0, 0, 2, 2), RED, &mut output)
            .unwrap();

        assert_eq!(report.content_generation, 2);
        assert_eq!(report.pixels.visible_pixels, 4);
        assert_eq!(report.pixels.recomposed_pixels, 4);
        assert!(!report.pixels.scene_changed);
    }

    #[test]
    fn raise_reveals_latest_retained_hidden_content() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 16];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(0, 0, 4, 4);
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            bounds,
            RED,
            &mut lower_layer,
            &mut output,
        );
        let upper = create_window(
            &mut compositor,
            OWNER_B,
            bounds,
            BLUE,
            &mut upper_layer,
            &mut output,
        );
        compositor
            .present(
                OWNER_A,
                lower,
                ShellRect::new(0, 0, 4, 4),
                GREEN,
                &mut output,
            )
            .unwrap();

        let pixels = compositor.raise(OWNER_A, lower, &mut output).unwrap();

        assert_eq!(pixels.recomposed_pixels, 16);
        assert!(pixels.scene_changed);
        assert_eq!(compositor.z_order(), [Some(upper), Some(lower)]);
        assert_eq!(pixel(&output, 0, 0), GREEN);
        assert_eq!(pixel(&output, 3, 3), GREEN);
    }

    #[test]
    fn raising_topmost_window_is_a_noop() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );

        let pixels = compositor.raise(OWNER_A, id, &mut output).unwrap();

        assert_eq!(pixels, PixelLedger::EMPTY);
        assert_eq!(compositor.z_order(), [Some(id), None]);
        assert_eq!(pixel(&output, 0, 0), RED);
    }

    #[test]
    fn destroy_recomposes_exposed_lower_window_then_background() {
        let mut lower_layer = [0_u32; 4];
        let mut upper_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut lower_layer,
            &mut output,
        );
        let upper = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(0, 0, 1, 1),
            BLUE,
            &mut upper_layer,
            &mut output,
        );

        let destroyed_upper = compositor.destroy(OWNER_B, upper, &mut output).unwrap();
        assert_eq!(destroyed_upper.layer(), &[BLUE]);
        assert_eq!(destroyed_upper.pixels().recomposed_pixels, 1);
        assert!(destroyed_upper.pixels().scene_changed);
        assert_eq!(pixel(&output, 0, 0), RED);

        let destroyed_lower = compositor.destroy(OWNER_A, lower, &mut output).unwrap();
        assert_eq!(destroyed_lower.layer(), &[RED; 4]);
        assert_eq!(destroyed_lower.pixels().recomposed_pixels, 4);
        assert!(destroyed_lower.pixels().scene_changed);
        assert_eq!(pixel(&output, 0, 0), BACKGROUND_COLOR);
        assert_eq!(pixel(&output, 1, 1), BACKGROUND_COLOR);
    }

    #[test]
    fn z_order_tracks_create_and_raise_exactly() {
        let mut first_layer = [0_u32; 1];
        let mut second_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let first = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut first_layer,
            &mut output,
        );
        let second = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut second_layer,
            &mut output,
        );
        assert_eq!(compositor.z_order(), [Some(first), Some(second)]);
        assert_eq!(compositor.window_info(OWNER_A, first).unwrap().z_index, 0);
        assert_eq!(compositor.window_info(OWNER_B, second).unwrap().z_index, 1);

        compositor.raise(OWNER_A, first, &mut output).unwrap();

        assert_eq!(compositor.z_order(), [Some(second), Some(first)]);
        assert_eq!(compositor.window_info(OWNER_A, first).unwrap().z_index, 1);
        assert_eq!(compositor.window_info(OWNER_B, second).unwrap().z_index, 0);
    }

    #[test]
    fn hit_test_selects_topmost_and_respects_half_open_edges() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(1, 1, 4, 4),
            RED,
            &mut lower_layer,
            &mut output,
        );
        let upper = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(2, 2, 2, 2),
            BLUE,
            &mut upper_layer,
            &mut output,
        );

        assert_eq!(compositor.topmost_hit_test(2, 2), Some(upper));
        assert_eq!(compositor.topmost_hit_test(1, 1), Some(lower));
        assert_eq!(compositor.topmost_hit_test(4, 4), Some(lower));
        assert_eq!(compositor.topmost_hit_test(5, 5), None);
        assert_eq!(compositor.topmost_hit_test(SURFACE_WIDTH, 0), None);
    }

    #[test]
    fn pointer_down_focuses_and_captures_topmost_target() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(10, 20, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );

        let route = compositor.pointer_down(11, 20).unwrap().unwrap();

        assert_eq!(route.window_id, id);
        assert_eq!(route.local_x, 1);
        assert_eq!(route.local_y, 0);
        assert!(route.pressed);
        assert!(route.captured);
        assert_eq!(route.focus_generation, 1);
        assert_eq!(compositor.captured_window(), Some(id));
        assert_eq!(
            compositor.focus_state(),
            FocusState {
                window_id: Some(id),
                generation: 1,
            }
        );
    }

    #[test]
    fn captured_move_routes_outside_window_with_signed_local_coordinates() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(10, 20, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.pointer_down(10, 20).unwrap();

        let route = compositor.pointer_move(0, 0).unwrap().unwrap();

        assert_eq!(route.window_id, id);
        assert_eq!(route.local_x, -10);
        assert_eq!(route.local_y, -20);
        assert!(route.pressed);
        assert!(route.captured);
        assert_eq!(compositor.captured_window(), Some(id));
    }

    #[test]
    fn unbounded_capture_clamps_global_but_preserves_full_signed_local_coordinates() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(48, 80, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.pointer_down(48, 80).unwrap();

        let route = compositor
            .pointer_move_unbounded(-36, -44)
            .unwrap()
            .unwrap();

        assert_eq!(route.window_id, id);
        assert_eq!((route.global_x, route.global_y), (0, 0));
        assert_eq!((route.local_x, route.local_y), (-84, -124));
        assert!(route.pressed);
        assert!(route.captured);
        assert_eq!(compositor.captured_window(), Some(id));

        let release = compositor.pointer_up_unbounded(-36, -44).unwrap().unwrap();
        assert_eq!((release.global_x, release.global_y), (0, 0));
        assert_eq!((release.local_x, release.local_y), (-84, -124));
        assert!(!release.pressed);
        assert_eq!(compositor.captured_window(), None);
    }

    #[test]
    fn unbounded_capture_overflow_is_transactionally_inert() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(1, 1, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.pointer_down(1, 1).unwrap();

        assert_eq!(
            compositor.pointer_up_unbounded(i32::MIN, 0),
            Err(CompositorError::CounterExhausted)
        );
        assert_eq!(compositor.captured_window(), Some(id));
        assert!(compositor.pointer_up(1, 1).unwrap().is_some());
        assert_eq!(compositor.captured_window(), None);
    }

    #[test]
    fn pointer_up_routes_then_clears_capture() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(1, 1, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.pointer_down(1, 1).unwrap();

        let route = compositor.pointer_up(20, 30).unwrap().unwrap();

        assert_eq!(route.window_id, id);
        assert_eq!(route.local_x, 19);
        assert_eq!(route.local_y, 29);
        assert!(!route.pressed);
        assert!(route.captured);
        assert_eq!(compositor.captured_window(), None);
        assert_eq!(compositor.pointer_move(1, 1).unwrap(), None);
        assert_eq!(compositor.pointer_up(1, 1).unwrap(), None);
    }

    #[test]
    fn duplicate_pointer_down_is_inert() {
        let mut first_layer = [0_u32; 1];
        let mut second_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let first = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut first_layer,
            &mut output,
        );
        create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut second_layer,
            &mut output,
        );
        compositor.pointer_down(0, 0).unwrap();
        let focus = compositor.focus_state();

        assert_eq!(
            compositor.pointer_down(1, 0),
            Err(CompositorError::PointerAlreadyCaptured)
        );
        assert_eq!(compositor.captured_window(), Some(first));
        assert_eq!(compositor.focus_state(), focus);
    }

    #[test]
    fn pointer_down_miss_and_invalid_coordinate_are_inert() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );

        assert_eq!(compositor.pointer_down(2, 2).unwrap(), None);
        assert_eq!(compositor.focus_state(), FocusState::default());
        assert_eq!(compositor.captured_window(), None);
        assert_eq!(
            compositor.pointer_down(SURFACE_WIDTH, 0),
            Err(CompositorError::GlobalCoordinateOutOfBounds)
        );
        assert_eq!(compositor.focus_state(), FocusState::default());
        assert_eq!(compositor.captured_window(), None);
    }

    #[test]
    fn focus_generation_is_monotonic_and_refocus_is_idempotent() {
        let mut first_layer = [0_u32; 1];
        let mut second_layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let first = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut first_layer,
            &mut output,
        );
        let second = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(1, 0, 1, 1),
            BLUE,
            &mut second_layer,
            &mut output,
        );

        assert_eq!(compositor.focus(OWNER_A, first).unwrap().generation, 1);
        assert_eq!(compositor.focus(OWNER_A, first).unwrap().generation, 1);
        assert_eq!(compositor.focus(OWNER_B, second).unwrap().generation, 2);
        assert_eq!(
            compositor.focus_state(),
            FocusState {
                window_id: Some(second),
                generation: 2,
            }
        );
    }

    #[test]
    fn focus_counter_overflow_prevents_pointer_capture() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.focus_generation = u64::MAX;

        assert_eq!(
            compositor.pointer_down(0, 0),
            Err(CompositorError::CounterExhausted)
        );
        assert_eq!(compositor.focused, None);
        assert_eq!(compositor.captured, None);
        assert_eq!(compositor.focus_generation, u64::MAX);
    }

    #[test]
    fn focused_destroy_counter_overflow_rolls_back() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.focus(OWNER_A, id).unwrap();
        compositor.focus_generation = u64::MAX;

        assert_eq!(
            compositor.destroy(OWNER_A, id, &mut output).unwrap_err(),
            CompositorError::CounterExhausted
        );
        assert_eq!(compositor.window_count(), 1);
        assert_eq!(compositor.focused, Some(id));
        assert_eq!(compositor.layer(OWNER_A, id).unwrap(), &[RED]);
        assert_eq!(pixel(&output, 0, 0), RED);
    }

    #[test]
    fn destroying_captured_window_clears_capture_and_advances_focus() {
        let mut layer = [0_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let id = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 1, 1),
            RED,
            &mut layer,
            &mut output,
        );
        compositor.pointer_down(0, 0).unwrap();

        let destroyed = compositor.destroy(OWNER_A, id, &mut output).unwrap();

        assert_eq!(destroyed.window_id(), id);
        assert_eq!(compositor.captured_window(), None);
        assert_eq!(
            compositor.focus_state(),
            FocusState {
                window_id: None,
                generation: 2,
            }
        );
    }

    #[test]
    fn compose_is_strictly_limited_to_requested_damage() {
        let mut layer = [0_u32; 4];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();
        create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 2, 2),
            RED,
            &mut layer,
            &mut output,
        );
        output.fill(GREEN);

        let pixels = compositor
            .compose(ShellRect::new(0, 0, 1, 1), &mut output)
            .unwrap();

        assert_eq!(pixels.recomposed_pixels, 1);
        assert!(pixels.scene_changed);
        assert_eq!(pixel(&output, 0, 0), RED);
        assert_eq!(pixel(&output, 1, 0), GREEN);
        assert_eq!(pixel(&output, 0, 1), GREEN);
        assert_eq!(pixel(&output, 10, 10), GREEN);
    }

    #[test]
    fn compose_validation_failure_does_not_touch_output() {
        let mut output = surface(BLUE);
        let compositor = Compositor::new();

        assert_eq!(
            compositor.compose(ShellRect::new(u16::MAX, 0, 1, 1), &mut output),
            Err(CompositorError::InvalidGeometry)
        );
        assert_eq!(
            compositor.compose(
                ShellRect::new(0, 0, 1, 1),
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            ),
            Err(CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            })
        );
        assert!(output.iter().all(|pixel| *pixel == BLUE));
    }

    #[test]
    fn exhausted_slot_generation_uses_other_slot_then_fails_cleanly() {
        let mut first_layer = [9_u32; 1];
        let mut second_layer = [10_u32; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        compositor.slots[0].generation = u32::MAX;

        let report = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 1, 1),
                RED,
                &mut first_layer,
                &mut output,
            )
            .unwrap();
        assert_eq!(report.window_id.slot(), 1);
        let returned = compositor
            .destroy(OWNER_A, report.window_id, &mut output)
            .unwrap()
            .into_layer();
        assert_eq!(returned, &[RED]);
        compositor.slots[1].generation = u32::MAX;

        let failure = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 1, 1),
                BLUE,
                &mut second_layer,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(failure.error(), CompositorError::CounterExhausted);
        assert_eq!(failure.layer(), &[10]);
        assert_eq!(compositor.window_count(), 0);
        assert_eq!(pixel(&output, 0, 0), BACKGROUND_COLOR);
    }

    #[test]
    fn system_overlay_install_update_remove_restores_retained_scene() {
        let mut window_layer = [0_u32; 12];
        let mut overlay_layer = [GREEN, BLUE, BLUE, GREEN];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(10, 20, 4, 3),
            RED,
            &mut window_layer,
            &mut output,
        );
        let bounds = ShellRect::new(11, 20, 2, 2);

        let installed = compositor
            .install_system_overlay(bounds, &mut overlay_layer, &mut output)
            .unwrap();

        assert_eq!(
            installed,
            SystemOverlayReport {
                content_generation: 1,
                global_damage: bounds,
                pixels: PixelLedger {
                    content_pixels: 4,
                    visible_pixels: 4,
                    occluded_pixels: 0,
                    recomposed_pixels: 4,
                    scene_changed: true,
                },
            }
        );
        assert_eq!(
            compositor.system_overlay_info(),
            Some(SystemOverlayInfo {
                bounds,
                content_generation: 1,
            })
        );
        assert_eq!(
            compositor.system_overlay_layer(),
            Some(&[GREEN, BLUE, BLUE, GREEN][..])
        );
        assert_eq!(pixel(&output, 11, 20), GREEN);
        assert_eq!(pixel(&output, 12, 20), BLUE);

        let updated = compositor
            .apply_system_overlay_pixels(ShellRect::new(1, 0, 1, 2), &[RED, RED], &mut output)
            .unwrap();

        assert_eq!(updated.content_generation, 2);
        assert_eq!(updated.global_damage, ShellRect::new(12, 20, 1, 2));
        assert_eq!(
            updated.pixels,
            PixelLedger {
                content_pixels: 2,
                visible_pixels: 2,
                occluded_pixels: 0,
                recomposed_pixels: 2,
                scene_changed: true,
            }
        );
        assert_eq!(
            compositor.system_overlay_layer(),
            Some(&[GREEN, RED, BLUE, RED][..])
        );

        let removed = compositor.remove_system_overlay(&mut output).unwrap();

        assert_eq!(removed.bounds(), bounds);
        assert_eq!(removed.content_generation(), 2);
        assert_eq!(removed.layer(), &[GREEN, RED, BLUE, RED]);
        assert_eq!(
            removed.pixels(),
            PixelLedger {
                content_pixels: 0,
                visible_pixels: 0,
                occluded_pixels: 0,
                recomposed_pixels: 4,
                scene_changed: true,
            }
        );
        assert_eq!(pixel(&output, 11, 20), RED);
        assert_eq!(pixel(&output, 12, 21), RED);
        assert_eq!(compositor.system_overlay_info(), None);
    }

    #[test]
    fn system_overlay_is_outside_window_topology_focus_and_capture() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 4];
        let mut overlay_layer = [GREEN; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let lower = create_window(
            &mut compositor,
            OWNER_A,
            ShellRect::new(0, 0, 4, 4),
            RED,
            &mut lower_layer,
            &mut output,
        );
        let upper = create_window(
            &mut compositor,
            OWNER_B,
            ShellRect::new(1, 1, 2, 2),
            BLUE,
            &mut upper_layer,
            &mut output,
        );
        compositor.pointer_down(1, 1).unwrap();
        let expected_focus = compositor.focus_state();
        let expected_capture = compositor.captured_window();
        let expected_z = compositor.z_order();

        compositor
            .install_system_overlay(ShellRect::new(1, 1, 2, 2), &mut overlay_layer, &mut output)
            .unwrap();

        assert_eq!(compositor.window_count(), WINDOW_CAPACITY);
        assert_eq!(compositor.z_order(), expected_z);
        assert_eq!(compositor.z_order(), [Some(lower), Some(upper)]);
        assert_eq!(compositor.focus_state(), expected_focus);
        assert_eq!(compositor.captured_window(), expected_capture);
        assert!(compositor.system_overlay_hit_test(1, 1));
        assert!(compositor.system_overlay_hit_test(2, 2));
        assert!(!compositor.system_overlay_hit_test(3, 1));
        assert!(!compositor.system_overlay_hit_test(1, 3));
        assert!(!compositor.system_overlay_hit_test(0, 1));
        assert_eq!(compositor.topmost_hit_test(1, 1), Some(upper));
        assert_eq!(pixel(&output, 1, 1), GREEN);

        compositor.remove_system_overlay(&mut output).unwrap();
        assert_eq!(compositor.window_count(), WINDOW_CAPACITY);
        assert_eq!(compositor.z_order(), expected_z);
        assert_eq!(compositor.focus_state(), expected_focus);
        assert_eq!(compositor.captured_window(), expected_capture);
        assert_eq!(pixel(&output, 1, 1), BLUE);
    }

    #[test]
    fn window_mutations_keep_overlay_topmost_and_report_overlay_occlusion() {
        let mut lower_layer = [0_u32; 16];
        let mut upper_layer = [0_u32; 16];
        let mut overlay_layer = [GREEN; 4];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        compositor
            .install_system_overlay(ShellRect::new(1, 1, 2, 2), &mut overlay_layer, &mut output)
            .unwrap();

        let lower_report = compositor
            .create(
                OWNER_A,
                ShellRect::new(0, 0, 4, 4),
                RED,
                &mut lower_layer,
                &mut output,
            )
            .unwrap();
        let lower = lower_report.window_id;
        assert_eq!(lower_report.pixels.content_pixels, 16);
        assert_eq!(lower_report.pixels.visible_pixels, 12);
        assert_eq!(lower_report.pixels.occluded_pixels, 4);
        assert_eq!(lower_report.pixels.recomposed_pixels, 16);
        assert_eq!(pixel(&output, 0, 0), RED);
        assert_eq!(pixel(&output, 1, 1), GREEN);

        let present = compositor
            .present(
                OWNER_A,
                lower,
                ShellRect::new(0, 0, 4, 4),
                BLUE,
                &mut output,
            )
            .unwrap();
        assert_eq!(present.pixels.visible_pixels, 12);
        assert_eq!(present.pixels.occluded_pixels, 4);
        assert_eq!(pixel(&output, 0, 0), BLUE);
        assert_eq!(pixel(&output, 1, 1), GREEN);

        let pixels = compositor
            .apply_pixels(
                OWNER_A,
                lower,
                ShellRect::new(1, 1, 2, 2),
                &[RED; 4],
                &mut output,
            )
            .unwrap();
        assert_eq!(pixels.pixels.visible_pixels, 0);
        assert_eq!(pixels.pixels.occluded_pixels, 4);
        assert_eq!(pixels.pixels.recomposed_pixels, 4);
        assert!(!pixels.pixels.scene_changed);
        assert_eq!(pixel(&output, 1, 1), GREEN);

        let upper_report = compositor
            .create(
                OWNER_B,
                ShellRect::new(0, 0, 4, 4),
                BLUE,
                &mut upper_layer,
                &mut output,
            )
            .unwrap();
        assert_eq!(upper_report.pixels.visible_pixels, 12);
        assert_eq!(upper_report.pixels.occluded_pixels, 4);
        assert_eq!(pixel(&output, 1, 1), GREEN);

        let raised = compositor.raise(OWNER_A, lower, &mut output).unwrap();
        assert_eq!(raised.recomposed_pixels, 16);
        assert_eq!(pixel(&output, 1, 1), GREEN);
        let destroyed = compositor.destroy(OWNER_A, lower, &mut output).unwrap();
        assert_eq!(destroyed.pixels().recomposed_pixels, 16);
        assert_eq!(pixel(&output, 1, 1), GREEN);
        assert_eq!(pixel(&output, 0, 0), BLUE);
    }

    #[test]
    fn invalid_system_overlay_installs_return_untouched_layers() {
        let mut invalid_geometry = [0x1111_u32; 1];
        let mut wrong_length = [0x2222_u32; 1];
        let mut noncanonical = [0xff00_0000_u32; 1];
        let mut short_output_layer = [0x3333_u32; 1];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();

        let failure = compositor
            .install_system_overlay(
                ShellRect::new(0, 0, 0, 1),
                &mut invalid_geometry,
                &mut output,
            )
            .unwrap_err();
        assert_eq!(failure.error(), CompositorError::InvalidGeometry);
        assert_eq!(failure.into_parts().1, &[0x1111]);

        let failure = compositor
            .install_system_overlay(ShellRect::new(0, 0, 2, 1), &mut wrong_length, &mut output)
            .unwrap_err();
        assert_eq!(
            failure.error(),
            CompositorError::LayerLength {
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(failure.into_parts().1, &[0x2222]);

        let failure = compositor
            .install_system_overlay(ShellRect::new(0, 0, 1, 1), &mut noncanonical, &mut output)
            .unwrap_err();
        assert_eq!(failure.error(), CompositorError::NonCanonicalColor);
        assert_eq!(failure.into_parts().1, &[0xff00_0000]);

        let failure = compositor
            .install_system_overlay(
                ShellRect::new(0, 0, 1, 1),
                &mut short_output_layer,
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            }
        );
        assert_eq!(failure.into_parts().1, &[0x3333]);
        assert_eq!(compositor.system_overlay_info(), None);
        assert!(output.iter().all(|pixel| *pixel == BLUE));
    }

    #[test]
    fn duplicate_overlay_install_returns_new_layer_without_state_change() {
        let mut installed_layer = [GREEN; 4];
        let mut duplicate_layer = [RED; 1];
        let mut output = surface(BACKGROUND_COLOR);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(2, 3, 2, 2);
        compositor
            .install_system_overlay(bounds, &mut installed_layer, &mut output)
            .unwrap();
        let expected_output = output.clone();

        let failure = compositor
            .install_system_overlay(
                ShellRect::new(0, 0, 1, 1),
                &mut duplicate_layer,
                &mut output,
            )
            .unwrap_err();

        assert_eq!(
            failure.error(),
            CompositorError::SystemOverlayAlreadyInstalled
        );
        let returned = failure.into_parts().1;
        assert_eq!(returned, &[RED]);
        returned[0] = BLUE;
        assert_eq!(returned, &[BLUE]);
        assert_eq!(
            compositor.system_overlay_info(),
            Some(SystemOverlayInfo {
                bounds,
                content_generation: 1,
            })
        );
        assert_eq!(compositor.system_overlay_layer(), Some(&[GREEN; 4][..]));
        assert_eq!(output, expected_output);
    }

    #[test]
    fn overlay_update_and_remove_failures_are_transactionally_inert() {
        let mut overlay_layer = [GREEN; 4];
        let mut output = surface(BLUE);
        let mut compositor = Compositor::new();
        let bounds = ShellRect::new(2, 3, 2, 2);
        compositor
            .install_system_overlay(bounds, &mut overlay_layer, &mut output)
            .unwrap();
        let expected_output = output.clone();

        assert_eq!(
            compositor.apply_system_overlay_pixels(
                ShellRect::new(0, 0, 2, 2),
                &[RED; 3],
                &mut output,
            ),
            Err(CompositorError::PixelSourceLength {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            compositor.apply_system_overlay_pixels(
                ShellRect::new(0, 0, 2, 2),
                &[RED, RED, 0xff00_0000, RED],
                &mut output,
            ),
            Err(CompositorError::NonCanonicalColor)
        );
        assert_eq!(
            compositor.apply_system_overlay_pixels(
                ShellRect::new(1, 0, 2, 1),
                &[RED; 2],
                &mut output,
            ),
            Err(CompositorError::DamageOutOfBounds)
        );
        assert_eq!(
            compositor.apply_system_overlay_pixels(
                ShellRect::new(0, 0, 1, 1),
                &[RED],
                &mut output[..SURFACE_PIXEL_COUNT - 1],
            ),
            Err(CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            })
        );
        assert_eq!(
            compositor
                .remove_system_overlay(&mut output[..SURFACE_PIXEL_COUNT - 1])
                .unwrap_err(),
            CompositorError::OutputLength {
                expected: SURFACE_PIXEL_COUNT,
                actual: SURFACE_PIXEL_COUNT - 1,
            }
        );
        assert_eq!(
            compositor.system_overlay_info(),
            Some(SystemOverlayInfo {
                bounds,
                content_generation: 1,
            })
        );
        assert_eq!(compositor.system_overlay_layer(), Some(&[GREEN; 4][..]));
        assert_eq!(output, expected_output);

        compositor
            .system_overlay
            .as_mut()
            .unwrap()
            .content_generation = u64::MAX;
        assert_eq!(
            compositor
                .apply_system_overlay_pixels(ShellRect::new(0, 0, 1, 1), &[RED], &mut output,),
            Err(CompositorError::CounterExhausted)
        );
        assert_eq!(compositor.system_overlay_layer(), Some(&[GREEN; 4][..]));
        assert_eq!(output, expected_output);
    }

    #[test]
    fn overlay_absence_is_explicit_and_inert() {
        let mut output = surface(BLUE);
        let expected_output = output.clone();
        let mut compositor = Compositor::new();

        assert_eq!(
            compositor
                .apply_system_overlay_pixels(ShellRect::new(0, 0, 1, 1), &[RED], &mut output,),
            Err(CompositorError::SystemOverlayNotInstalled)
        );
        assert_eq!(
            compositor.remove_system_overlay(&mut output).unwrap_err(),
            CompositorError::SystemOverlayNotInstalled
        );
        assert!(!compositor.system_overlay_hit_test(0, 0));
        assert_eq!(output, expected_output);
    }
}
