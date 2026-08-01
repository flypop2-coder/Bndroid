#![no_std]

#[cfg(feature = "androidbox-scene-rpc2")]
pub mod android_scene;
pub mod mobile;

pub const PRESENT_WIRE_SIZE: usize = 64;
pub const PRESENT_HEADER_SIZE: usize = 20;
pub const SOLID_RECT_WIRE_SIZE: usize = 11;
pub const MAX_SOLID_RECTS: usize = 4;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SURFACE_WIDTH: u16 = 208;
#[cfg(feature = "mobile-ui-runtime")]
pub const SURFACE_WIDTH: u16 = 720;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SURFACE_HEIGHT: u16 = 368;
#[cfg(feature = "mobile-ui-runtime")]
pub const SURFACE_HEIGHT: u16 = 1_600;
pub const SURFACE_ID: u16 = 1;
pub const PROTOCOL_VERSION: u16 = 2;
pub const PRESENT_OPCODE: u8 = 1;
pub const PRESENT_MAGIC: u32 = u32::from_le_bytes(*b"BUI1");
pub const UI_SERVER_EVENT_WIRE_SIZE: usize = 64;
pub const UI_SERVER_EVENT_MAGIC: u32 = u32::from_le_bytes(*b"BUE1");
#[cfg(feature = "androidbox-el0-runtime0")]
pub const UI_SERVER_EVENT_VERSION: u16 = 7;
#[cfg(not(feature = "androidbox-el0-runtime0"))]
pub const UI_SERVER_EVENT_VERSION: u16 = 6;
pub const UI_CLIENT_CONTROL_WIRE_SIZE: usize = 64;
pub const UI_CLIENT_CONTROL_MAGIC: u32 = u32::from_le_bytes(*b"BUC1");
pub const UI_CLIENT_CONTROL_VERSION: u16 = 8;
pub const BUFFER_PRESENT_WIRE_SIZE: usize = 64;
pub const BUFFER_PRESENT_MAGIC: u32 = u32::from_le_bytes(*b"BUP1");
#[cfg(not(feature = "mobile-system-chrome0"))]
pub const BUFFER_PRESENT_VERSION: u16 = 2;
#[cfg(feature = "mobile-system-chrome0")]
pub const BUFFER_PRESENT_VERSION: u16 = 3;
pub const BUFFER_PRESENT_KIND: u8 = 1;
pub const BUFFER_PRESENT_MODE_FULL: u8 = 1;
/// Canonical physical application-content viewport for the opt-in trusted
/// mobile System UI chrome split.
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_VIEWPORT_X: u16 = 0;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_VIEWPORT_Y: u16 = 64;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_VIEWPORT_WIDTH: u16 = 720;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_VIEWPORT_HEIGHT: u16 = 1_448;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_VIEWPORT_BOTTOM: u16 =
    MOBILE_CONTENT_VIEWPORT_Y + MOBILE_CONTENT_VIEWPORT_HEIGHT;
/// Short compositor-facing aliases for the same half-open content rectangle.
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_X: u16 = MOBILE_CONTENT_VIEWPORT_X;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_TOP: u16 = MOBILE_CONTENT_VIEWPORT_Y;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_WIDTH: u16 = MOBILE_CONTENT_VIEWPORT_WIDTH;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_HEIGHT: u16 = MOBILE_CONTENT_VIEWPORT_HEIGHT;
#[cfg(feature = "mobile-system-chrome0")]
pub const MOBILE_CONTENT_BOTTOM: u16 = MOBILE_CONTENT_VIEWPORT_BOTTOM;
#[cfg(not(feature = "mobile-system-chrome0"))]
pub const BUFFER_PRESENT_X: u16 = 0;
#[cfg(feature = "mobile-system-chrome0")]
pub const BUFFER_PRESENT_X: u16 = MOBILE_CONTENT_VIEWPORT_X;
#[cfg(not(feature = "mobile-system-chrome0"))]
pub const BUFFER_PRESENT_Y: u16 = 0;
#[cfg(feature = "mobile-system-chrome0")]
pub const BUFFER_PRESENT_Y: u16 = MOBILE_CONTENT_VIEWPORT_Y;
#[cfg(not(feature = "mobile-system-chrome0"))]
pub const BUFFER_PRESENT_WIDTH: u16 = SURFACE_WIDTH;
#[cfg(feature = "mobile-system-chrome0")]
pub const BUFFER_PRESENT_WIDTH: u16 = MOBILE_CONTENT_VIEWPORT_WIDTH;
#[cfg(not(feature = "mobile-system-chrome0"))]
pub const BUFFER_PRESENT_HEIGHT: u16 = SURFACE_HEIGHT;
#[cfg(feature = "mobile-system-chrome0")]
pub const BUFFER_PRESENT_HEIGHT: u16 = MOBILE_CONTENT_VIEWPORT_HEIGHT;
pub const APP_LIFECYCLE_WIRE_SIZE: usize = 64;
pub const APP_LIFECYCLE_MAGIC: u32 = u32::from_le_bytes(*b"ALC1");
pub const APP_LIFECYCLE_VERSION: u16 = 1;
pub const UI_BOOTSTRAP_WIRE_SIZE: usize = 64;
pub const UI_BOOTSTRAP_MAGIC: u32 = u32::from_le_bytes(*b"UBP1");
pub const UI_BOOTSTRAP_VERSION: u16 = 1;
pub const SURFACE_RECOVERY_WIRE_SIZE: usize = 64;
pub const SURFACE_RECOVERY_MAGIC: u32 = u32::from_le_bytes(*b"BSR1");
pub const SURFACE_RECOVERY_VERSION: u16 = 1;
pub const UI_SUPERVISOR_CONTROL_WIRE_SIZE: usize = 64;
pub const UI_SUPERVISOR_CONTROL_MAGIC: u32 = u32::from_le_bytes(*b"USC1");
pub const UI_SUPERVISOR_CONTROL_VERSION: u16 = 1;
pub const WINDOW_COMMAND_WIRE_SIZE: usize = 64;
pub const WINDOW_COMMAND_MAGIC: u32 = u32::from_le_bytes(*b"BWC1");
pub const WINDOW_COMMAND_VERSION: u16 = 1;
pub const WINDOW_EVENT_WIRE_SIZE: usize = 64;
pub const WINDOW_EVENT_MAGIC: u32 = u32::from_le_bytes(*b"BWE1");
pub const WINDOW_EVENT_VERSION: u16 = 1;
pub const WINDOW_CAPACITY: usize = 2;
pub const TEXT_INPUT_COMMAND_WIRE_SIZE: usize = 64;
pub const TEXT_INPUT_COMMAND_MAGIC: u32 = u32::from_le_bytes(*b"BTI1");
pub const TEXT_INPUT_COMMAND_VERSION: u16 = 1;
pub const TEXT_INPUT_EVENT_WIRE_SIZE: usize = 64;
pub const TEXT_INPUT_EVENT_MAGIC: u32 = u32::from_le_bytes(*b"BTE1");
pub const TEXT_INPUT_EVENT_VERSION: u16 = 1;
pub const TEXT_INPUT_CAPACITY: usize = 8;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SHELL_WIDTH: u16 = 320;
#[cfg(feature = "mobile-ui-runtime")]
pub const SHELL_WIDTH: u16 = 720;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SHELL_HEIGHT: u16 = 480;
#[cfg(feature = "mobile-ui-runtime")]
pub const SHELL_HEIGHT: u16 = 1_600;
const _: () = assert!(PRESENT_HEADER_SIZE + MAX_SOLID_RECTS * SOLID_RECT_WIRE_SIZE == 64);
const _: () = assert!(UI_SERVER_EVENT_WIRE_SIZE == 64);
const _: () = assert!(UI_CLIENT_CONTROL_WIRE_SIZE == 64);
const _: () = assert!(BUFFER_PRESENT_WIRE_SIZE == 64);
const _: () = assert!(APP_LIFECYCLE_WIRE_SIZE == 64);
const _: () = assert!(UI_BOOTSTRAP_WIRE_SIZE == 64);
const _: () = assert!(SURFACE_RECOVERY_WIRE_SIZE == 64);
const _: () = assert!(UI_SUPERVISOR_CONTROL_WIRE_SIZE == 64);
const _: () = assert!(WINDOW_COMMAND_WIRE_SIZE == 64);
const _: () = assert!(WINDOW_EVENT_WIRE_SIZE == 64);
const _: () = assert!(WINDOW_CAPACITY <= u8::MAX as usize);
const _: () = assert!(TEXT_INPUT_COMMAND_WIRE_SIZE == 64);
const _: () = assert!(TEXT_INPUT_EVENT_WIRE_SIZE == 64);
const _: () = assert!(TEXT_INPUT_CAPACITY <= u8::MAX as usize);
#[cfg(feature = "mobile-system-chrome0")]
const _: () = {
    assert!(MOBILE_CONTENT_VIEWPORT_X == 0);
    assert!(MOBILE_CONTENT_VIEWPORT_Y == 64);
    assert!(MOBILE_CONTENT_VIEWPORT_WIDTH == SURFACE_WIDTH);
    assert!(MOBILE_CONTENT_VIEWPORT_BOTTOM == 1_512);
    assert!(MOBILE_CONTENT_VIEWPORT_BOTTOM <= SURFACE_HEIGHT);
};

#[cfg(not(feature = "mobile-ui-runtime"))]
pub const PHONE_TARGET: ShellRect = ShellRect::new(72, 132, 176, 72);
#[cfg(feature = "mobile-ui-runtime")]
pub const PHONE_TARGET: ShellRect = ShellRect::new(20, 1_340, 170, 190);
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const MESSAGES_TARGET: ShellRect = ShellRect::new(72, 220, 176, 72);
#[cfg(feature = "mobile-ui-runtime")]
pub const MESSAGES_TARGET: ShellRect = ShellRect::new(190, 1_340, 170, 190);
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const SETTINGS_TARGET: ShellRect = ShellRect::new(72, 308, 176, 72);
#[cfg(feature = "mobile-ui-runtime")]
pub const SETTINGS_TARGET: ShellRect = ShellRect::new(530, 1_340, 170, 190);
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const HOME_TARGET: ShellRect = ShellRect::new(128, 448, 64, 24);
#[cfg(feature = "mobile-ui-runtime")]
pub const HOME_TARGET: ShellRect = ShellRect::new(240, 1_548, 240, 52);

#[cfg(feature = "mobile-ui-runtime")]
const _: () = {
    assert!(PHONE_TARGET.x == mobile::HOME_PHONE_TARGET.x);
    assert!(PHONE_TARGET.y == mobile::HOME_PHONE_TARGET.y);
    assert!(PHONE_TARGET.width == mobile::HOME_PHONE_TARGET.width);
    assert!(PHONE_TARGET.height == mobile::HOME_PHONE_TARGET.height);
    assert!(MESSAGES_TARGET.x == mobile::HOME_MESSAGES_TARGET.x);
    assert!(MESSAGES_TARGET.y == mobile::HOME_MESSAGES_TARGET.y);
    assert!(MESSAGES_TARGET.width == mobile::HOME_MESSAGES_TARGET.width);
    assert!(MESSAGES_TARGET.height == mobile::HOME_MESSAGES_TARGET.height);
    assert!(SETTINGS_TARGET.x == mobile::HOME_SETTINGS_TARGET.x);
    assert!(SETTINGS_TARGET.y == mobile::HOME_SETTINGS_TARGET.y);
    assert!(SETTINGS_TARGET.width == mobile::HOME_SETTINGS_TARGET.width);
    assert!(SETTINGS_TARGET.height == mobile::HOME_SETTINGS_TARGET.height);
    assert!(HOME_TARGET.x == mobile::SYSTEM_HOME_TARGET.x);
    assert!(HOME_TARGET.y == mobile::SYSTEM_HOME_TARGET.y);
    assert!(HOME_TARGET.width == mobile::SYSTEM_HOME_TARGET.width);
    assert!(HOME_TARGET.height == mobile::SYSTEM_HOME_TARGET.height);
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl ShellRect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn contains(self, x: u16, y: u16) -> bool {
        let Some(right) = self.x.checked_add(self.width) else {
            return false;
        };
        let Some(bottom) = self.y.checked_add(self.height) else {
            return false;
        };
        x >= self.x && x < right && y >= self.y && y < bottom
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ShellAppId {
    Phone = 1,
    Messages = 2,
    Settings = 3,
}

impl ShellAppId {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Phone),
            2 => Some(Self::Messages),
            3 => Some(Self::Settings),
            _ => None,
        }
    }
}

/// One boot-local compatible-Activity identity retained by System UI.
///
/// The trusted Launcher allocates `session_id` before fresh package
/// verification so the reservation and its terminal commit/abort can carry one
/// exact identity. The identity is not eligible to become a recent or bound
/// Activity session until the fresh relaunch succeeds and SurfaceServer accepts
/// the matching commit. `package_generation` binds the transaction to the
/// catalog generation being re-read and verified. Neither value is a process
/// handle, storage capability, background-execution claim, or persistent
/// Android task identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiCompatibleActivityIdentity {
    session_id: u64,
    package_generation: u64,
}

impl UiCompatibleActivityIdentity {
    pub const fn new(session_id: u64, package_generation: u64) -> Option<Self> {
        if session_id == 0 || package_generation == 0 {
            return None;
        }
        Some(Self {
            session_id,
            package_generation,
        })
    }

    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    pub const fn package_generation(self) -> u64 {
        self.package_generation
    }
}

/// The single bounded recent identity retained by SurfaceServer.
///
/// A compatible Activity entry deliberately carries identity only. System UI
/// stores no Activity pixels and this enum grants no lifecycle, package-store,
/// or execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiRecentIdentity {
    Shell(ShellAppId),
    CompatibleAndroid(UiCompatibleActivityIdentity),
}

impl UiRecentIdentity {
    pub const fn shell_app(self) -> Option<ShellAppId> {
        match self {
            Self::Shell(app) => Some(app),
            Self::CompatibleAndroid(_) => None,
        }
    }

    pub const fn compatible_android(self) -> Option<UiCompatibleActivityIdentity> {
        match self {
            Self::Shell(_) => None,
            Self::CompatibleAndroid(identity) => Some(identity),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShellView {
    #[default]
    Home,
    App(ShellAppId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellTarget {
    App(ShellAppId),
    Home,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellTransition {
    pub from: ShellView,
    pub to: ShellView,
    pub target: ShellTarget,
    pub transition_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellError {
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShellController {
    view: ShellView,
    armed: Option<ShellTarget>,
    pointer_pressed: bool,
    reports: u64,
    taps: u64,
    transitions: u64,
}

impl ShellController {
    pub const fn new() -> Self {
        Self {
            view: ShellView::Home,
            armed: None,
            pointer_pressed: false,
            reports: 0,
            taps: 0,
            transitions: 0,
        }
    }

    pub fn observe(
        &mut self,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> Result<Option<ShellTransition>, ShellError> {
        let reports = self
            .reports
            .checked_add(1)
            .ok_or(ShellError::CounterExhausted)?;
        match (self.pointer_pressed, pressed) {
            (false, true) => {
                self.reports = reports;
                self.armed = shell_hit_test(self.view, x, y);
                self.pointer_pressed = true;
                Ok(None)
            }
            (true, false) => {
                let armed = self.armed;
                let released = shell_hit_test(self.view, x, y);
                let Some(target) = armed.filter(|target| Some(*target) == released) else {
                    self.reports = reports;
                    self.pointer_pressed = false;
                    self.armed = None;
                    return Ok(None);
                };
                let taps = self
                    .taps
                    .checked_add(1)
                    .ok_or(ShellError::CounterExhausted)?;
                let from = self.view;
                let to = match target {
                    ShellTarget::App(app) => ShellView::App(app),
                    ShellTarget::Home => ShellView::Home,
                };
                if to == from {
                    self.reports = reports;
                    self.taps = taps;
                    self.pointer_pressed = false;
                    self.armed = None;
                    return Ok(None);
                }
                let transitions = self
                    .transitions
                    .checked_add(1)
                    .ok_or(ShellError::CounterExhausted)?;
                self.reports = reports;
                self.taps = taps;
                self.transitions = transitions;
                self.view = to;
                self.pointer_pressed = false;
                self.armed = None;
                Ok(Some(ShellTransition {
                    from,
                    to,
                    target,
                    transition_id: transitions,
                }))
            }
            (_, _) => {
                self.reports = reports;
                self.pointer_pressed = pressed;
                Ok(None)
            }
        }
    }

    pub const fn view(self) -> ShellView {
        self.view
    }

    pub const fn armed(self) -> Option<ShellTarget> {
        self.armed
    }

    pub const fn pointer_pressed(self) -> bool {
        self.pointer_pressed
    }

    pub const fn reports(self) -> u64 {
        self.reports
    }

    pub const fn taps(self) -> u64 {
        self.taps
    }

    pub const fn transitions(self) -> u64 {
        self.transitions
    }
}

pub const fn shell_hit_test(view: ShellView, x: u16, y: u16) -> Option<ShellTarget> {
    match view {
        ShellView::Home if PHONE_TARGET.contains(x, y) => Some(ShellTarget::App(ShellAppId::Phone)),
        ShellView::Home if MESSAGES_TARGET.contains(x, y) => {
            Some(ShellTarget::App(ShellAppId::Messages))
        }
        ShellView::Home if SETTINGS_TARGET.contains(x, y) => {
            Some(ShellTarget::App(ShellAppId::Settings))
        }
        ShellView::App(_) if HOME_TARGET.contains(x, y) => Some(ShellTarget::Home),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSampleError {
    ZeroSequence,
    CoordinateOutOfBounds,
    NonCanonicalState,
}

/// One normalized, generation-qualified pointer report returned in the two
/// output registers of `SurfaceReadInput`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputSample {
    sequence: u64,
    x: u16,
    y: u16,
    pressed: bool,
}

impl InputSample {
    pub fn try_new(sequence: u64, x: u16, y: u16, pressed: bool) -> Result<Self, InputSampleError> {
        if sequence == 0 {
            return Err(InputSampleError::ZeroSequence);
        }
        if x >= SHELL_WIDTH || y >= SHELL_HEIGHT {
            return Err(InputSampleError::CoordinateOutOfBounds);
        }
        Ok(Self {
            sequence,
            x,
            y,
            pressed,
        })
    }

    /// Packs x/y/pressed into x1; x2 carries the complete nonzero sequence.
    pub const fn encode_registers(self) -> (u64, u64) {
        let state = self.x as u64 | ((self.y as u64) << 16) | ((self.pressed as u64) << 32);
        (state, self.sequence)
    }

    pub fn decode_registers(state: u64, sequence: u64) -> Result<Self, InputSampleError> {
        if state >> 33 != 0 {
            return Err(InputSampleError::NonCanonicalState);
        }
        Self::try_new(
            sequence,
            state as u16,
            (state >> 16) as u16,
            (state >> 32) != 0,
        )
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn x(self) -> u16 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }

    pub const fn pressed(self) -> bool {
        self.pressed
    }
}

/// Stable identities used by the UI server protocol. Zero is deliberately
/// invalid, so an all-zero payload can never identify a client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiClientId {
    Launcher = 1,
    App = 2,
}

impl UiClientId {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Launcher),
            2 => Some(Self::App),
            _ => None,
        }
    }
}

/// A bounded software-only dimming level for one UI session.
///
/// These levels affect rendered pixels only. They do not represent panel,
/// backlight, power-management, or physical-display authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSoftwareDimming {
    #[default]
    Off = 0,
    Light = 1,
    Medium = 2,
    Strong = 3,
    Maximum = 4,
}

impl UiSoftwareDimming {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            0 => Some(Self::Off),
            1 => Some(Self::Light),
            2 => Some(Self::Medium),
            3 => Some(Self::Strong),
            4 => Some(Self::Maximum),
            _ => None,
        }
    }
}

/// The complete appearance state shared by one SurfaceServer UI session.
///
/// This value contains only the bounded settings implemented by the mobile
/// preview. It is session state, not persistent user or device state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAppearance {
    dark_theme: bool,
    alternate_accent: bool,
    software_dimming: UiSoftwareDimming,
}

impl Default for UiAppearance {
    fn default() -> Self {
        Self::new(true, false)
    }
}

impl UiAppearance {
    const DARK_THEME_FLAG: u64 = 1 << 0;
    const ALTERNATE_ACCENT_FLAG: u64 = 1 << 1;
    const SOFTWARE_DIMMING_SHIFT: u32 = 2;
    const SOFTWARE_DIMMING_MASK: u64 = 0b111 << Self::SOFTWARE_DIMMING_SHIFT;
    const KNOWN_FLAGS: u64 =
        Self::DARK_THEME_FLAG | Self::ALTERNATE_ACCENT_FLAG | Self::SOFTWARE_DIMMING_MASK;

    pub const fn new(dark_theme: bool, alternate_accent: bool) -> Self {
        Self {
            dark_theme,
            alternate_accent,
            software_dimming: UiSoftwareDimming::Off,
        }
    }

    pub const fn dark_theme(self) -> bool {
        self.dark_theme
    }

    pub const fn alternate_accent(self) -> bool {
        self.alternate_accent
    }

    pub const fn software_dimming(self) -> UiSoftwareDimming {
        self.software_dimming
    }

    pub const fn with_software_dimming(self, software_dimming: UiSoftwareDimming) -> Self {
        Self {
            dark_theme: self.dark_theme,
            alternate_accent: self.alternate_accent,
            software_dimming,
        }
    }

    pub const fn flags(self) -> u64 {
        ((self.dark_theme as u64) * Self::DARK_THEME_FLAG)
            | ((self.alternate_accent as u64) * Self::ALTERNATE_ACCENT_FLAG)
            | ((self.software_dimming.raw() as u64) << Self::SOFTWARE_DIMMING_SHIFT)
    }

    pub fn from_flags(flags: u64) -> Result<Self, UiAppearanceError> {
        if flags & !Self::KNOWN_FLAGS != 0 {
            return Err(UiAppearanceError::UnknownFlags);
        }
        let software_dimming_raw =
            (flags & Self::SOFTWARE_DIMMING_MASK) >> Self::SOFTWARE_DIMMING_SHIFT;
        let software_dimming = UiSoftwareDimming::from_raw(software_dimming_raw)
            .ok_or(UiAppearanceError::InvalidSoftwareDimming)?;
        Ok(Self::new(
            flags & Self::DARK_THEME_FLAG != 0,
            flags & Self::ALTERNATE_ACCENT_FLAG != 0,
        )
        .with_software_dimming(software_dimming))
    }

    const fn applying(self, action: UiAppearanceAction) -> Self {
        match action {
            UiAppearanceAction::ToggleTheme => Self {
                dark_theme: !self.dark_theme,
                alternate_accent: self.alternate_accent,
                software_dimming: self.software_dimming,
            },
            UiAppearanceAction::ToggleAccent => Self {
                dark_theme: self.dark_theme,
                alternate_accent: !self.alternate_accent,
                software_dimming: self.software_dimming,
            },
            UiAppearanceAction::SetSoftwareDimmingOff => {
                self.with_software_dimming(UiSoftwareDimming::Off)
            }
            UiAppearanceAction::SetSoftwareDimmingLight => {
                self.with_software_dimming(UiSoftwareDimming::Light)
            }
            UiAppearanceAction::SetSoftwareDimmingMedium => {
                self.with_software_dimming(UiSoftwareDimming::Medium)
            }
            UiAppearanceAction::SetSoftwareDimmingStrong => {
                self.with_software_dimming(UiSoftwareDimming::Strong)
            }
            UiAppearanceAction::SetSoftwareDimmingMaximum => {
                self.with_software_dimming(UiSoftwareDimming::Maximum)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAppearanceError {
    UnknownFlags,
    InvalidSoftwareDimming,
}

/// A bounded appearance mutation. SurfaceServer serializes these actions so
/// rapid toggles do not depend on a client having observed the prior result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiAppearanceAction {
    ToggleTheme = 1,
    ToggleAccent = 2,
    SetSoftwareDimmingOff = 3,
    SetSoftwareDimmingLight = 4,
    SetSoftwareDimmingMedium = 5,
    SetSoftwareDimmingStrong = 6,
    SetSoftwareDimmingMaximum = 7,
}

impl UiAppearanceAction {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::ToggleTheme),
            2 => Some(Self::ToggleAccent),
            3 => Some(Self::SetSoftwareDimmingOff),
            4 => Some(Self::SetSoftwareDimmingLight),
            5 => Some(Self::SetSoftwareDimmingMedium),
            6 => Some(Self::SetSoftwareDimmingStrong),
            7 => Some(Self::SetSoftwareDimmingMaximum),
            _ => None,
        }
    }

    pub const fn set_software_dimming(software_dimming: UiSoftwareDimming) -> Self {
        match software_dimming {
            UiSoftwareDimming::Off => Self::SetSoftwareDimmingOff,
            UiSoftwareDimming::Light => Self::SetSoftwareDimmingLight,
            UiSoftwareDimming::Medium => Self::SetSoftwareDimmingMedium,
            UiSoftwareDimming::Strong => Self::SetSoftwareDimmingStrong,
            UiSoftwareDimming::Maximum => Self::SetSoftwareDimmingMaximum,
        }
    }

    pub const fn software_dimming(self) -> Option<UiSoftwareDimming> {
        match self {
            Self::ToggleTheme | Self::ToggleAccent => None,
            Self::SetSoftwareDimmingOff => Some(UiSoftwareDimming::Off),
            Self::SetSoftwareDimmingLight => Some(UiSoftwareDimming::Light),
            Self::SetSoftwareDimmingMedium => Some(UiSoftwareDimming::Medium),
            Self::SetSoftwareDimmingStrong => Some(UiSoftwareDimming::Strong),
            Self::SetSoftwareDimmingMaximum => Some(UiSoftwareDimming::Maximum),
        }
    }
}

pub const UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX: u16 = 8;
pub const UI_SYSTEM_UI_NAV_REVEAL_MAX_PX: u16 = 480;
const UI_SYSTEM_UI_NAV_REVEAL_MAX_STEPS: u64 =
    (UI_SYSTEM_UI_NAV_REVEAL_MAX_PX / UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX) as u64;
const _: () =
    assert!(UI_SYSTEM_UI_NAV_REVEAL_MAX_PX.is_multiple_of(UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX));
const _: () = assert!(UI_SYSTEM_UI_NAV_REVEAL_MAX_STEPS <= 0x3f);

/// SurfaceServer-owned mobile shell mode for one boot-local UI session.
///
/// `Foreground` names only the fixed foreground app route. It does not claim
/// a background task, process-lifecycle, persistence, or snapshot facility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSystemUiMode {
    Locked = 1,
    Home = 2,
    Foreground = 3,
    Overview = 4,
}

impl UiSystemUiMode {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Locked),
            2 => Some(Self::Home),
            3 => Some(Self::Foreground),
            4 => Some(Self::Overview),
            _ => None,
        }
    }
}

/// The bounded Launcher requests understood by the system-UI session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSystemUiAction {
    Unlock = 1,
    CloseOverview = 2,
    ActivateRecent = 3,
    PresentCompatibleActivity = 4,
    HomeCompatibleActivity = 5,
    FinishCompatibleActivity = 6,
    ReserveCompatibleActivity = 7,
    AbortCompatibleActivityVerification = 8,
}

impl UiSystemUiAction {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Unlock),
            2 => Some(Self::CloseOverview),
            3 => Some(Self::ActivateRecent),
            4 => Some(Self::PresentCompatibleActivity),
            5 => Some(Self::HomeCompatibleActivity),
            6 => Some(Self::FinishCompatibleActivity),
            7 => Some(Self::ReserveCompatibleActivity),
            8 => Some(Self::AbortCompatibleActivityVerification),
            _ => None,
        }
    }
}

const SYSTEM_UI_ACTION_SHIFT: u32 = 0;
const SYSTEM_UI_ACTION_MASK: u64 = 0xff << SYSTEM_UI_ACTION_SHIFT;
const SYSTEM_UI_ACTION_RECENT_KIND_SHIFT: u32 = 8;
const SYSTEM_UI_ACTION_RECENT_KIND_MASK: u64 = 0xff << SYSTEM_UI_ACTION_RECENT_KIND_SHIFT;
const SYSTEM_UI_ACTION_SHELL_APP_SHIFT: u32 = 16;
const SYSTEM_UI_ACTION_SHELL_APP_MASK: u64 = 0xff << SYSTEM_UI_ACTION_SHELL_APP_SHIFT;
const SYSTEM_UI_ACTION_KNOWN_MASK: u64 =
    SYSTEM_UI_ACTION_MASK | SYSTEM_UI_ACTION_RECENT_KIND_MASK | SYSTEM_UI_ACTION_SHELL_APP_MASK;

const SYSTEM_UI_RECENT_KIND_NONE: u8 = 0;
const SYSTEM_UI_RECENT_KIND_SHELL: u8 = 1;
const SYSTEM_UI_RECENT_KIND_COMPATIBLE_ANDROID: u8 = 2;

fn pack_system_ui_action(
    action: UiSystemUiAction,
    recent: Option<UiRecentIdentity>,
) -> (u64, u64, u64) {
    let (kind, shell_app, session_id, package_generation) = match recent {
        None => (SYSTEM_UI_RECENT_KIND_NONE, 0, 0, 0),
        Some(UiRecentIdentity::Shell(app)) => {
            (SYSTEM_UI_RECENT_KIND_SHELL, u64::from(app.raw()), 0, 0)
        }
        Some(UiRecentIdentity::CompatibleAndroid(identity)) => (
            SYSTEM_UI_RECENT_KIND_COMPATIBLE_ANDROID,
            0,
            identity.session_id(),
            identity.package_generation(),
        ),
    };
    (
        u64::from(action.raw())
            | (u64::from(kind) << SYSTEM_UI_ACTION_RECENT_KIND_SHIFT)
            | (shell_app << SYSTEM_UI_ACTION_SHELL_APP_SHIFT),
        session_id,
        package_generation,
    )
}

fn unpack_system_ui_action(
    packed: u64,
    session_id: u64,
    package_generation: u64,
) -> Result<(UiSystemUiAction, Option<UiRecentIdentity>), UiClientControlError> {
    if packed & !SYSTEM_UI_ACTION_KNOWN_MASK != 0 {
        return Err(UiClientControlError::NonCanonicalSystemUiAction);
    }
    let action_raw = (packed & SYSTEM_UI_ACTION_MASK) >> SYSTEM_UI_ACTION_SHIFT;
    let action = UiSystemUiAction::from_raw(action_raw)
        .ok_or(UiClientControlError::InvalidSystemUiAction)?;
    let recent_kind =
        (packed & SYSTEM_UI_ACTION_RECENT_KIND_MASK) >> SYSTEM_UI_ACTION_RECENT_KIND_SHIFT;
    let shell_app_raw =
        (packed & SYSTEM_UI_ACTION_SHELL_APP_MASK) >> SYSTEM_UI_ACTION_SHELL_APP_SHIFT;
    let recent = match recent_kind {
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_NONE) => {
            if shell_app_raw != 0 || session_id != 0 || package_generation != 0 {
                return Err(UiClientControlError::NonCanonicalSystemUiAction);
            }
            None
        }
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_SHELL) => {
            if shell_app_raw == 0 || session_id != 0 || package_generation != 0 {
                return Err(UiClientControlError::NonCanonicalSystemUiAction);
            }
            let app =
                ShellAppId::from_raw(shell_app_raw).ok_or(UiClientControlError::InvalidApp)?;
            Some(UiRecentIdentity::Shell(app))
        }
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_COMPATIBLE_ANDROID) => {
            if shell_app_raw != 0 {
                return Err(UiClientControlError::NonCanonicalSystemUiAction);
            }
            let identity = UiCompatibleActivityIdentity::new(session_id, package_generation)
                .ok_or(UiClientControlError::InvalidCompatibleActivityIdentity)?;
            Some(UiRecentIdentity::CompatibleAndroid(identity))
        }
        _ => return Err(UiClientControlError::InvalidRecentIdentity),
    };
    validate_system_ui_action_recent(action, recent)?;
    Ok((action, recent))
}

fn validate_system_ui_action_recent(
    action: UiSystemUiAction,
    recent: Option<UiRecentIdentity>,
) -> Result<(), UiClientControlError> {
    match (action, recent) {
        (UiSystemUiAction::ActivateRecent, None) => {
            Err(UiClientControlError::SystemUiActionRequiresApp)
        }
        (UiSystemUiAction::Unlock | UiSystemUiAction::CloseOverview, Some(_)) => {
            Err(UiClientControlError::SystemUiActionMustNotSpecifyApp)
        }
        (
            UiSystemUiAction::PresentCompatibleActivity
            | UiSystemUiAction::HomeCompatibleActivity
            | UiSystemUiAction::FinishCompatibleActivity
            | UiSystemUiAction::ReserveCompatibleActivity
            | UiSystemUiAction::AbortCompatibleActivityVerification,
            Some(UiRecentIdentity::CompatibleAndroid(_)),
        ) => Ok(()),
        (
            UiSystemUiAction::PresentCompatibleActivity
            | UiSystemUiAction::HomeCompatibleActivity
            | UiSystemUiAction::FinishCompatibleActivity
            | UiSystemUiAction::ReserveCompatibleActivity
            | UiSystemUiAction::AbortCompatibleActivityVerification,
            None | Some(UiRecentIdentity::Shell(_)),
        ) => Err(UiClientControlError::SystemUiActionRequiresCompatibleActivity),
        _ => Ok(()),
    }
}

/// The two execution points admitted by the bounded AndroidBox DEX-0 audit.
///
/// This is execution evidence for one fixed boot-local `classes.dex` fixture;
/// it is not an Android framework, APK compatibility, or arbitrary-code claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiAndroidBoxExecutionKind {
    Boot = 1,
    Tap = 2,
}

impl UiAndroidBoxExecutionKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::Boot),
            2 => Some(Self::Tap),
            _ => None,
        }
    }
}

pub const UI_ANDROIDBOX_MAX_INSTRUCTION_COUNT: u8 = 64;

/// One canonical, allocation-free report from the fixed AndroidBox DEX-0
/// interpreter.
///
/// Both the archive-side CRC-32 and the DEX-header Adler-32 are carried so the
/// SurfaceServer audit session can bind every later tap to the exact fixture
/// accepted at boot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxExecutionReport {
    request_id: u64,
    kind: UiAndroidBoxExecutionKind,
    dex_crc32: u32,
    dex_adler32: u32,
    result: i32,
    instruction_count: u8,
    tap_count: u16,
}

impl UiAndroidBoxExecutionReport {
    pub fn new(
        request_id: u64,
        kind: UiAndroidBoxExecutionKind,
        dex_crc32: u32,
        dex_adler32: u32,
        result: i32,
        instruction_count: u8,
        tap_count: u16,
    ) -> Result<Self, UiClientControlError> {
        if request_id == 0 {
            return Err(UiClientControlError::ZeroAndroidBoxRequestId);
        }
        if instruction_count == 0 {
            return Err(UiClientControlError::ZeroAndroidBoxInstructionCount);
        }
        if instruction_count > UI_ANDROIDBOX_MAX_INSTRUCTION_COUNT {
            return Err(UiClientControlError::AndroidBoxInstructionCountOutOfRange);
        }
        match (kind, tap_count) {
            (UiAndroidBoxExecutionKind::Boot, 0)
            | (UiAndroidBoxExecutionKind::Tap, 1..=u16::MAX) => {}
            (UiAndroidBoxExecutionKind::Boot, _) => {
                return Err(UiClientControlError::AndroidBoxBootTapCountMustBeZero);
            }
            (UiAndroidBoxExecutionKind::Tap, 0) => {
                return Err(UiClientControlError::AndroidBoxTapCountMustBeNonZero);
            }
        }
        Ok(Self {
            request_id,
            kind,
            dex_crc32,
            dex_adler32,
            result,
            instruction_count,
            tap_count,
        })
    }

    pub const fn request_id(self) -> u64 {
        self.request_id
    }

    pub const fn kind(self) -> UiAndroidBoxExecutionKind {
        self.kind
    }

    pub const fn dex_crc32(self) -> u32 {
        self.dex_crc32
    }

    pub const fn dex_adler32(self) -> u32 {
        self.dex_adler32
    }

    pub const fn result(self) -> i32 {
        self.result
    }

    pub const fn instruction_count(self) -> u8 {
        self.instruction_count
    }

    pub const fn tap_count(self) -> u16 {
        self.tap_count
    }
}

const ANDROIDBOX_RESULT_MASK: u64 = u32::MAX as u64;
const ANDROIDBOX_KIND_SHIFT: u32 = 32;
const ANDROIDBOX_KIND_MASK: u64 = 0xff << ANDROIDBOX_KIND_SHIFT;
const ANDROIDBOX_INSTRUCTION_COUNT_SHIFT: u32 = 40;
const ANDROIDBOX_INSTRUCTION_COUNT_MASK: u64 = 0x7f << ANDROIDBOX_INSTRUCTION_COUNT_SHIFT;
const ANDROIDBOX_TAP_COUNT_SHIFT: u32 = 48;
const ANDROIDBOX_TAP_COUNT_MASK: u64 = 0xffff << ANDROIDBOX_TAP_COUNT_SHIFT;
const ANDROIDBOX_EXECUTION_KNOWN_MASK: u64 = ANDROIDBOX_RESULT_MASK
    | ANDROIDBOX_KIND_MASK
    | ANDROIDBOX_INSTRUCTION_COUNT_MASK
    | ANDROIDBOX_TAP_COUNT_MASK;
const _: () = assert!(ANDROIDBOX_EXECUTION_KNOWN_MASK == u64::MAX ^ (1 << 47));

fn pack_androidbox_checksums(report: UiAndroidBoxExecutionReport) -> u64 {
    u64::from(report.dex_crc32()) | (u64::from(report.dex_adler32()) << 32)
}

fn pack_androidbox_execution(report: UiAndroidBoxExecutionReport) -> u64 {
    u64::from(report.result() as u32)
        | (u64::from(report.kind().raw()) << ANDROIDBOX_KIND_SHIFT)
        | (u64::from(report.instruction_count()) << ANDROIDBOX_INSTRUCTION_COUNT_SHIFT)
        | (u64::from(report.tap_count()) << ANDROIDBOX_TAP_COUNT_SHIFT)
}

fn unpack_androidbox_execution(
    request_id: u64,
    checksums: u64,
    execution: u64,
) -> Result<UiAndroidBoxExecutionReport, UiClientControlError> {
    if execution & !ANDROIDBOX_EXECUTION_KNOWN_MASK != 0 {
        return Err(UiClientControlError::NonCanonicalAndroidBoxExecution);
    }
    let kind_raw = (execution & ANDROIDBOX_KIND_MASK) >> ANDROIDBOX_KIND_SHIFT;
    let kind = UiAndroidBoxExecutionKind::from_raw(kind_raw)
        .ok_or(UiClientControlError::InvalidAndroidBoxExecutionKind)?;
    let instruction_count = ((execution & ANDROIDBOX_INSTRUCTION_COUNT_MASK)
        >> ANDROIDBOX_INSTRUCTION_COUNT_SHIFT) as u8;
    let tap_count = ((execution & ANDROIDBOX_TAP_COUNT_MASK) >> ANDROIDBOX_TAP_COUNT_SHIFT) as u16;
    UiAndroidBoxExecutionReport::new(
        request_id,
        kind,
        checksums as u32,
        (checksums >> 32) as u32,
        execution as u32 as i32,
        instruction_count,
        tap_count,
    )
}

/// The only Activity lifecycle boundary admitted by AndroidBox Activity-0.
///
/// This is a bounded launcher-local shim report. It is not evidence that ART,
/// ActivityThread, Android Framework, or a general APK lifecycle exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiAndroidBoxActivityLifecycleKind {
    OnCreateComplete = 1,
}

impl UiAndroidBoxActivityLifecycleKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::OnCreateComplete),
            _ => None,
        }
    }
}

/// CRC-32 of the fixed DEX descriptor `Lorg/bndroid/demo/MainActivity;`.
///
/// v7 intentionally does not spend wire bits on a caller-selected Activity:
/// control kind 7 always denotes this exact descriptor. Changing the
/// descriptor requires a protocol version change.
pub const UI_ANDROIDBOX_ACTIVITY_DESCRIPTOR_CRC32: u32 = 0xb193_6890;
pub const UI_ANDROIDBOX_ACTIVITY_MAX_REQUEST_ID: u32 = u32::MAX;
pub const UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX: u16 = 0x0fff;
pub const UI_ANDROIDBOX_ACTIVITY_MAX_CODE_OFFSET: u32 = 0x001f_fffc;
pub const UI_ANDROIDBOX_ACTIVITY_MAX_INSTRUCTION_COUNT: u8 = 64;
pub const UI_ANDROIDBOX_ACTIVITY_MAX_VIEW_TEXT_LENGTH: u8 = 96;
pub const UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT: &[u8] = b"AndroidBox resource-backed view";
pub const UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LABEL: u16 = 0x2402;
pub const UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LENGTH: u8 = 31;
const _: () = assert!(
    UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT.len() == UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LENGTH as usize
);

/// A small, deterministic consistency label for bounded Activity-0 view text.
///
/// This folded FNV-1a value is deliberately only 16 bits. It helps QEMU
/// evidence detect accidental text drift, but is not cryptographic
/// authentication and must not be used as an authority or security decision.
pub fn androidbox_activity_view_text_label(text: &[u8]) -> Result<u16, UiClientControlError> {
    if text.is_empty() {
        return Err(UiClientControlError::EmptyAndroidBoxActivityViewText);
    }
    if text.len() > usize::from(UI_ANDROIDBOX_ACTIVITY_MAX_VIEW_TEXT_LENGTH) {
        return Err(UiClientControlError::AndroidBoxActivityViewTextTooLong);
    }
    let mut hash = 0x811c_9dc5_u32;
    for byte in text {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    Ok((hash ^ (hash >> 16)) as u16)
}

/// One canonical Activity-0 execution report for the fixed demo fixture.
///
/// The report attests that the bounded binary-manifest parser selected the
/// protocol-fixed launcher Activity and that its restricted `onCreate` shim
/// completed. The report carries no handle, acknowledgement, capability, or
/// generalized Android authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxActivityExecutionReport {
    request_id: u32,
    lifecycle: UiAndroidBoxActivityLifecycleKind,
    manifest_crc32: u32,
    dex_crc32: u32,
    dex_adler32: u32,
    on_create_method_index: u16,
    on_create_code_offset: u32,
    instruction_count: u8,
    view_text_label: u16,
    view_text_length: u8,
}

impl UiAndroidBoxActivityExecutionReport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: u32,
        lifecycle: UiAndroidBoxActivityLifecycleKind,
        manifest_crc32: u32,
        dex_crc32: u32,
        dex_adler32: u32,
        on_create_method_index: u16,
        on_create_code_offset: u32,
        instruction_count: u8,
        view_text: &[u8],
    ) -> Result<Self, UiClientControlError> {
        let view_text_length = u8::try_from(view_text.len())
            .map_err(|_| UiClientControlError::AndroidBoxActivityViewTextTooLong)?;
        let view_text_label = androidbox_activity_view_text_label(view_text)?;
        if view_text != UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT {
            return Err(UiClientControlError::AndroidBoxActivityViewTextIdentityMismatch);
        }
        Self::from_wire_fields(
            request_id,
            lifecycle,
            manifest_crc32,
            dex_crc32,
            dex_adler32,
            on_create_method_index,
            on_create_code_offset,
            instruction_count,
            view_text_label,
            view_text_length,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_wire_fields(
        request_id: u32,
        lifecycle: UiAndroidBoxActivityLifecycleKind,
        manifest_crc32: u32,
        dex_crc32: u32,
        dex_adler32: u32,
        on_create_method_index: u16,
        on_create_code_offset: u32,
        instruction_count: u8,
        view_text_label: u16,
        view_text_length: u8,
    ) -> Result<Self, UiClientControlError> {
        if request_id == 0 {
            return Err(UiClientControlError::ZeroAndroidBoxActivityRequestId);
        }
        if on_create_method_index > UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX {
            return Err(UiClientControlError::AndroidBoxActivityMethodIndexOutOfRange);
        }
        if on_create_code_offset == 0
            || on_create_code_offset > UI_ANDROIDBOX_ACTIVITY_MAX_CODE_OFFSET
        {
            return Err(UiClientControlError::AndroidBoxActivityCodeOffsetOutOfRange);
        }
        if on_create_code_offset & 3 != 0 {
            return Err(UiClientControlError::UnalignedAndroidBoxActivityCodeOffset);
        }
        if instruction_count == 0 {
            return Err(UiClientControlError::ZeroAndroidBoxActivityInstructionCount);
        }
        if instruction_count > UI_ANDROIDBOX_ACTIVITY_MAX_INSTRUCTION_COUNT {
            return Err(UiClientControlError::AndroidBoxActivityInstructionCountOutOfRange);
        }
        if view_text_length == 0 {
            return Err(UiClientControlError::EmptyAndroidBoxActivityViewText);
        }
        if view_text_length > UI_ANDROIDBOX_ACTIVITY_MAX_VIEW_TEXT_LENGTH {
            return Err(UiClientControlError::AndroidBoxActivityViewTextTooLong);
        }
        if view_text_label != UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LABEL
            || view_text_length != UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LENGTH
        {
            return Err(UiClientControlError::AndroidBoxActivityViewTextIdentityMismatch);
        }
        Ok(Self {
            request_id,
            lifecycle,
            manifest_crc32,
            dex_crc32,
            dex_adler32,
            on_create_method_index,
            on_create_code_offset,
            instruction_count,
            view_text_label,
            view_text_length,
        })
    }

    pub const fn request_id(self) -> u32 {
        self.request_id
    }

    pub const fn lifecycle(self) -> UiAndroidBoxActivityLifecycleKind {
        self.lifecycle
    }

    pub const fn manifest_crc32(self) -> u32 {
        self.manifest_crc32
    }

    pub const fn dex_crc32(self) -> u32 {
        self.dex_crc32
    }

    pub const fn dex_adler32(self) -> u32 {
        self.dex_adler32
    }

    pub const fn activity_descriptor_crc32(self) -> u32 {
        UI_ANDROIDBOX_ACTIVITY_DESCRIPTOR_CRC32
    }

    pub const fn on_create_method_index(self) -> u16 {
        self.on_create_method_index
    }

    pub const fn on_create_code_offset(self) -> u32 {
        self.on_create_code_offset
    }

    pub const fn instruction_count(self) -> u8 {
        self.instruction_count
    }

    pub const fn view_text_label(self) -> u16 {
        self.view_text_label
    }

    pub const fn view_text_length(self) -> u8 {
        self.view_text_length
    }
}

const ANDROIDBOX_ACTIVITY_REQUEST_MASK: u64 = u32::MAX as u64;
const ANDROIDBOX_ACTIVITY_MANIFEST_SHIFT: u32 = 32;
const ANDROIDBOX_ACTIVITY_METHOD_MASK: u64 = UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX as u64;
const ANDROIDBOX_ACTIVITY_CODE_UNITS_SHIFT: u32 = 12;
const ANDROIDBOX_ACTIVITY_CODE_UNITS_MASK: u64 = 0x0007_ffff << 12;
const ANDROIDBOX_ACTIVITY_VIEW_LABEL_SHIFT: u32 = 32;
const ANDROIDBOX_ACTIVITY_VIEW_LABEL_MASK: u64 = 0xffff << 32;
const ANDROIDBOX_ACTIVITY_VIEW_LENGTH_SHIFT: u32 = 48;
const ANDROIDBOX_ACTIVITY_VIEW_LENGTH_MASK: u64 = 0x7f << 48;
const ANDROIDBOX_ACTIVITY_INSTRUCTIONS_SHIFT: u32 = 55;
const ANDROIDBOX_ACTIVITY_INSTRUCTIONS_MASK: u64 = 0x7f << 55;
const ANDROIDBOX_ACTIVITY_LIFECYCLE_SHIFT: u32 = 62;
const ANDROIDBOX_ACTIVITY_LIFECYCLE_MASK: u64 = 0x3 << 62;
const ANDROIDBOX_ACTIVITY_EXECUTION_KNOWN_MASK: u64 = ANDROIDBOX_ACTIVITY_METHOD_MASK
    | ANDROIDBOX_ACTIVITY_CODE_UNITS_MASK
    | ANDROIDBOX_ACTIVITY_VIEW_LABEL_MASK
    | ANDROIDBOX_ACTIVITY_VIEW_LENGTH_MASK
    | ANDROIDBOX_ACTIVITY_INSTRUCTIONS_MASK
    | ANDROIDBOX_ACTIVITY_LIFECYCLE_MASK;
const _: () = assert!(ANDROIDBOX_ACTIVITY_EXECUTION_KNOWN_MASK == u64::MAX ^ (1 << 31));

fn pack_androidbox_activity_identity(report: UiAndroidBoxActivityExecutionReport) -> u64 {
    u64::from(report.request_id())
        | (u64::from(report.manifest_crc32()) << ANDROIDBOX_ACTIVITY_MANIFEST_SHIFT)
}

fn pack_androidbox_activity_checksums(report: UiAndroidBoxActivityExecutionReport) -> u64 {
    u64::from(report.dex_crc32()) | (u64::from(report.dex_adler32()) << 32)
}

fn pack_androidbox_activity_execution(report: UiAndroidBoxActivityExecutionReport) -> u64 {
    u64::from(report.on_create_method_index())
        | (u64::from(report.on_create_code_offset() / 4) << ANDROIDBOX_ACTIVITY_CODE_UNITS_SHIFT)
        | (u64::from(report.view_text_label()) << ANDROIDBOX_ACTIVITY_VIEW_LABEL_SHIFT)
        | (u64::from(report.view_text_length()) << ANDROIDBOX_ACTIVITY_VIEW_LENGTH_SHIFT)
        | (u64::from(report.instruction_count()) << ANDROIDBOX_ACTIVITY_INSTRUCTIONS_SHIFT)
        | (u64::from(report.lifecycle().raw()) << ANDROIDBOX_ACTIVITY_LIFECYCLE_SHIFT)
}

fn unpack_androidbox_activity_execution(
    identity: u64,
    checksums: u64,
    execution: u64,
) -> Result<UiAndroidBoxActivityExecutionReport, UiClientControlError> {
    if execution & !ANDROIDBOX_ACTIVITY_EXECUTION_KNOWN_MASK != 0 {
        return Err(UiClientControlError::NonCanonicalAndroidBoxActivityExecution);
    }
    let lifecycle = UiAndroidBoxActivityLifecycleKind::from_raw(
        (execution & ANDROIDBOX_ACTIVITY_LIFECYCLE_MASK) >> ANDROIDBOX_ACTIVITY_LIFECYCLE_SHIFT,
    )
    .ok_or(UiClientControlError::InvalidAndroidBoxActivityLifecycle)?;
    UiAndroidBoxActivityExecutionReport::from_wire_fields(
        (identity & ANDROIDBOX_ACTIVITY_REQUEST_MASK) as u32,
        lifecycle,
        (identity >> ANDROIDBOX_ACTIVITY_MANIFEST_SHIFT) as u32,
        checksums as u32,
        (checksums >> 32) as u32,
        (execution & ANDROIDBOX_ACTIVITY_METHOD_MASK) as u16,
        (((execution & ANDROIDBOX_ACTIVITY_CODE_UNITS_MASK) >> ANDROIDBOX_ACTIVITY_CODE_UNITS_SHIFT)
            as u32)
            * 4,
        ((execution & ANDROIDBOX_ACTIVITY_INSTRUCTIONS_MASK)
            >> ANDROIDBOX_ACTIVITY_INSTRUCTIONS_SHIFT) as u8,
        ((execution & ANDROIDBOX_ACTIVITY_VIEW_LABEL_MASK) >> ANDROIDBOX_ACTIVITY_VIEW_LABEL_SHIFT)
            as u16,
        ((execution & ANDROIDBOX_ACTIVITY_VIEW_LENGTH_MASK)
            >> ANDROIDBOX_ACTIVITY_VIEW_LENGTH_SHIFT) as u8,
    )
}

/// CRC-32 identities and resource IDs of the repository-owned Activity-1 APK.
///
/// These constants identify one bounded resources.arsc -> compiled layout ->
/// string-reference path. They are evidence labels for this exact fixture, not
/// a general Android resource-table or framework compatibility claim.
pub const UI_ANDROIDBOX_RESOURCE_ARSC_CRC32: u32 = 0x9f67_c7b4;
pub const UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32: u32 = 0x36ce_95c4;
pub const UI_ANDROIDBOX_RESOURCE_LAYOUT_ID: u32 = 0x7f02_0000;
pub const UI_ANDROIDBOX_RESOURCE_STRING_ID: u32 = 0x7f03_0000;
pub const UI_ANDROIDBOX_RESOURCE_VIEW_TEXT: &[u8] = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT;
pub const UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LABEL: u16 = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LABEL;
pub const UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LENGTH: u8 = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LENGTH;

/// Fixed Activity-1 success facts encoded in kind 8.
///
/// Every accepted report says that the bounded runtime parsed the resource
/// table, resolved the layout entry, parsed its binary XML, verified the
/// single TextView root, and resolved that view's string reference. These bits
/// carry no authority and must all be present; unknown bits are rejected.
pub const UI_ANDROIDBOX_RESOURCE_TABLE_PARSED_FLAG: u8 = 1 << 0;
pub const UI_ANDROIDBOX_RESOURCE_LAYOUT_RESOLVED_FLAG: u8 = 1 << 1;
pub const UI_ANDROIDBOX_RESOURCE_BINARY_XML_PARSED_FLAG: u8 = 1 << 2;
pub const UI_ANDROIDBOX_RESOURCE_TEXT_VIEW_VERIFIED_FLAG: u8 = 1 << 3;
pub const UI_ANDROIDBOX_RESOURCE_STRING_RESOLVED_FLAG: u8 = 1 << 4;
pub const UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS: u8 = UI_ANDROIDBOX_RESOURCE_TABLE_PARSED_FLAG
    | UI_ANDROIDBOX_RESOURCE_LAYOUT_RESOLVED_FLAG
    | UI_ANDROIDBOX_RESOURCE_BINARY_XML_PARSED_FLAG
    | UI_ANDROIDBOX_RESOURCE_TEXT_VIEW_VERIFIED_FLAG
    | UI_ANDROIDBOX_RESOURCE_STRING_RESOLVED_FLAG;
const _: () = assert!(UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS == 0x1f);
const _: () = assert!(
    UI_ANDROIDBOX_RESOURCE_VIEW_TEXT.len() == UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LENGTH as usize
);

/// One canonical Activity-1 compiled-resource resolution report.
///
/// Text bytes stay in the trusted Launcher runtime. The wire carries only
/// their bounded folded-FNV label and length, which the independent resource
/// audit cross-checks against the already accepted Activity report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxResourceExecutionReport {
    request_id: u32,
    resources_arsc_crc32: u32,
    layout_xml_crc32: u32,
    layout_resource_id: u32,
    string_resource_id: u32,
    view_text_label: u16,
    view_text_length: u8,
    success_flags: u8,
}

impl UiAndroidBoxResourceExecutionReport {
    pub fn new(
        request_id: u32,
        resources_arsc_crc32: u32,
        layout_xml_crc32: u32,
        layout_resource_id: u32,
        string_resource_id: u32,
        view_text: &[u8],
    ) -> Result<Self, UiClientControlError> {
        let view_text_length = u8::try_from(view_text.len())
            .map_err(|_| UiClientControlError::AndroidBoxResourceTextIdentityMismatch)?;
        let view_text_label = androidbox_activity_view_text_label(view_text)
            .map_err(|_| UiClientControlError::AndroidBoxResourceTextIdentityMismatch)?;
        if view_text != UI_ANDROIDBOX_RESOURCE_VIEW_TEXT {
            return Err(UiClientControlError::AndroidBoxResourceTextIdentityMismatch);
        }
        Self::from_wire_fields(
            request_id,
            resources_arsc_crc32,
            layout_xml_crc32,
            layout_resource_id,
            string_resource_id,
            view_text_label,
            view_text_length,
            UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_wire_fields(
        request_id: u32,
        resources_arsc_crc32: u32,
        layout_xml_crc32: u32,
        layout_resource_id: u32,
        string_resource_id: u32,
        view_text_label: u16,
        view_text_length: u8,
        success_flags: u8,
    ) -> Result<Self, UiClientControlError> {
        if request_id == 0 {
            return Err(UiClientControlError::ZeroAndroidBoxResourceRequestId);
        }
        if resources_arsc_crc32 != UI_ANDROIDBOX_RESOURCE_ARSC_CRC32 {
            return Err(UiClientControlError::AndroidBoxResourceArscIdentityMismatch);
        }
        if layout_xml_crc32 != UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32
            || layout_resource_id != UI_ANDROIDBOX_RESOURCE_LAYOUT_ID
        {
            return Err(UiClientControlError::AndroidBoxResourceLayoutIdentityMismatch);
        }
        if string_resource_id != UI_ANDROIDBOX_RESOURCE_STRING_ID
            || view_text_label != UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LABEL
            || view_text_length != UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LENGTH
        {
            return Err(UiClientControlError::AndroidBoxResourceTextIdentityMismatch);
        }
        if success_flags != UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS {
            return Err(UiClientControlError::AndroidBoxResourceSuccessFlagsMismatch);
        }
        Ok(Self {
            request_id,
            resources_arsc_crc32,
            layout_xml_crc32,
            layout_resource_id,
            string_resource_id,
            view_text_label,
            view_text_length,
            success_flags,
        })
    }

    pub const fn request_id(self) -> u32 {
        self.request_id
    }

    pub const fn resources_arsc_crc32(self) -> u32 {
        self.resources_arsc_crc32
    }

    pub const fn layout_xml_crc32(self) -> u32 {
        self.layout_xml_crc32
    }

    pub const fn layout_resource_id(self) -> u32 {
        self.layout_resource_id
    }

    pub const fn string_resource_id(self) -> u32 {
        self.string_resource_id
    }

    pub const fn view_text_label(self) -> u16 {
        self.view_text_label
    }

    pub const fn view_text_length(self) -> u8 {
        self.view_text_length
    }

    pub const fn success_flags(self) -> u8 {
        self.success_flags
    }
}

const ANDROIDBOX_RESOURCE_REQUEST_MASK: u64 = u32::MAX as u64;
const ANDROIDBOX_RESOURCE_ARSC_SHIFT: u32 = 32;
const ANDROIDBOX_RESOURCE_LAYOUT_ID_SHIFT: u32 = 32;
const ANDROIDBOX_RESOURCE_VIEW_LABEL_SHIFT: u32 = 32;
const ANDROIDBOX_RESOURCE_VIEW_LABEL_MASK: u64 = 0xffff << ANDROIDBOX_RESOURCE_VIEW_LABEL_SHIFT;
const ANDROIDBOX_RESOURCE_VIEW_LENGTH_SHIFT: u32 = 48;
const ANDROIDBOX_RESOURCE_VIEW_LENGTH_MASK: u64 = 0x7f << ANDROIDBOX_RESOURCE_VIEW_LENGTH_SHIFT;
const ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_SHIFT: u32 = 55;
const ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_MASK: u64 = 0x1f << ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_SHIFT;
const ANDROIDBOX_RESOURCE_EXECUTION_KNOWN_MASK: u64 = u32::MAX as u64
    | ANDROIDBOX_RESOURCE_VIEW_LABEL_MASK
    | ANDROIDBOX_RESOURCE_VIEW_LENGTH_MASK
    | ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_MASK;
const _: () = assert!(ANDROIDBOX_RESOURCE_EXECUTION_KNOWN_MASK == u64::MAX >> 4);

fn pack_androidbox_resource_identity(report: UiAndroidBoxResourceExecutionReport) -> u64 {
    u64::from(report.request_id())
        | (u64::from(report.resources_arsc_crc32()) << ANDROIDBOX_RESOURCE_ARSC_SHIFT)
}

fn pack_androidbox_resource_layout(report: UiAndroidBoxResourceExecutionReport) -> u64 {
    u64::from(report.layout_xml_crc32())
        | (u64::from(report.layout_resource_id()) << ANDROIDBOX_RESOURCE_LAYOUT_ID_SHIFT)
}

fn pack_androidbox_resource_execution(report: UiAndroidBoxResourceExecutionReport) -> u64 {
    u64::from(report.string_resource_id())
        | (u64::from(report.view_text_label()) << ANDROIDBOX_RESOURCE_VIEW_LABEL_SHIFT)
        | (u64::from(report.view_text_length()) << ANDROIDBOX_RESOURCE_VIEW_LENGTH_SHIFT)
        | (u64::from(report.success_flags()) << ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_SHIFT)
}

fn unpack_androidbox_resource_execution(
    identity: u64,
    layout: u64,
    execution: u64,
) -> Result<UiAndroidBoxResourceExecutionReport, UiClientControlError> {
    if execution & !ANDROIDBOX_RESOURCE_EXECUTION_KNOWN_MASK != 0 {
        return Err(UiClientControlError::NonCanonicalAndroidBoxResourceExecution);
    }
    UiAndroidBoxResourceExecutionReport::from_wire_fields(
        (identity & ANDROIDBOX_RESOURCE_REQUEST_MASK) as u32,
        (identity >> ANDROIDBOX_RESOURCE_ARSC_SHIFT) as u32,
        layout as u32,
        (layout >> ANDROIDBOX_RESOURCE_LAYOUT_ID_SHIFT) as u32,
        execution as u32,
        ((execution & ANDROIDBOX_RESOURCE_VIEW_LABEL_MASK) >> ANDROIDBOX_RESOURCE_VIEW_LABEL_SHIFT)
            as u16,
        ((execution & ANDROIDBOX_RESOURCE_VIEW_LENGTH_MASK)
            >> ANDROIDBOX_RESOURCE_VIEW_LENGTH_SHIFT) as u8,
        ((execution & ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_MASK)
            >> ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_SHIFT) as u8,
    )
}

const CONTROL_OFFSET_MAGIC: usize = 0;
const CONTROL_OFFSET_VERSION: usize = 4;
const CONTROL_OFFSET_KIND: usize = 6;
const CONTROL_OFFSET_HEADER_RESERVED: usize = 7;
const CONTROL_OFFSET_CLIENT: usize = 8;
const CONTROL_OFFSET_APP: usize = 16;
const CONTROL_OFFSET_TRANSITION_ID: usize = 24;
const CONTROL_OFFSET_ARGUMENT_3: usize = 32;
const CONTROL_OFFSET_ARGUMENT_4: usize = 40;
const CONTROL_OFFSET_BODY_RESERVED: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiClientControlKind {
    AttachAppEndpoint = 1,
    SetFocus = 2,
    UpdateAppearance = 3,
    DismissBootNotification = 4,
    UpdateSystemUi = 5,
    ReportAndroidBoxExecution = 6,
    ReportAndroidBoxActivityExecution = 7,
    ReportAndroidBoxResourceExecution = 8,
}

impl UiClientControlKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::AttachAppEndpoint),
            2 => Some(Self::SetFocus),
            3 => Some(Self::UpdateAppearance),
            4 => Some(Self::DismissBootNotification),
            5 => Some(Self::UpdateSystemUi),
            6 => Some(Self::ReportAndroidBoxExecution),
            7 => Some(Self::ReportAndroidBoxActivityExecution),
            8 => Some(Self::ReportAndroidBoxResourceExecution),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiClientControlError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    InvalidClient,
    InvalidApp,
    AttachRequiresAppClient,
    LauncherMustNotSpecifyApp,
    AppRequiresShellApp,
    ZeroTransitionId,
    InvalidAppearanceAction,
    ZeroAppearanceRequestId,
    ZeroBootNotificationRequestId,
    InvalidSystemUiAction,
    NonCanonicalSystemUiAction,
    InvalidRecentIdentity,
    InvalidCompatibleActivityIdentity,
    SystemUiActionRequiresApp,
    SystemUiActionMustNotSpecifyApp,
    SystemUiActionRequiresCompatibleActivity,
    ZeroSystemUiRequestId,
    ZeroObservedSystemUiRevision,
    ZeroAndroidBoxRequestId,
    InvalidAndroidBoxExecutionKind,
    NonCanonicalAndroidBoxExecution,
    ZeroAndroidBoxInstructionCount,
    AndroidBoxInstructionCountOutOfRange,
    AndroidBoxBootTapCountMustBeZero,
    AndroidBoxTapCountMustBeNonZero,
    ZeroAndroidBoxActivityRequestId,
    InvalidAndroidBoxActivityLifecycle,
    NonCanonicalAndroidBoxActivityExecution,
    AndroidBoxActivityMethodIndexOutOfRange,
    AndroidBoxActivityCodeOffsetOutOfRange,
    UnalignedAndroidBoxActivityCodeOffset,
    ZeroAndroidBoxActivityInstructionCount,
    AndroidBoxActivityInstructionCountOutOfRange,
    EmptyAndroidBoxActivityViewText,
    AndroidBoxActivityViewTextTooLong,
    AndroidBoxActivityViewTextIdentityMismatch,
    ZeroAndroidBoxResourceRequestId,
    NonCanonicalAndroidBoxResourceExecution,
    AndroidBoxResourceArscIdentityMismatch,
    AndroidBoxResourceLayoutIdentityMismatch,
    AndroidBoxResourceTextIdentityMismatch,
    AndroidBoxResourceSuccessFlagsMismatch,
    UnexpectedPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiClientControlPayload {
    AttachAppEndpoint {
        client: UiClientId,
    },
    SetFocus {
        target: UiClientId,
        app: Option<ShellAppId>,
        transition_id: u64,
    },
    UpdateAppearance {
        action: UiAppearanceAction,
        request_id: u64,
    },
    DismissBootNotification {
        request_id: u64,
    },
    UpdateSystemUi {
        action: UiSystemUiAction,
        recent: Option<UiRecentIdentity>,
        request_id: u64,
        observed_revision: u64,
    },
    ReportAndroidBoxExecution {
        report: UiAndroidBoxExecutionReport,
    },
    ReportAndroidBoxActivityExecution {
        report: UiAndroidBoxActivityExecutionReport,
    },
    ReportAndroidBoxResourceExecution {
        report: UiAndroidBoxResourceExecutionReport,
    },
}

/// One canonical 64-byte client-to-SurfaceServer control message.
///
/// Bytes 0..8 are the common header. The five little-endian arguments at
/// 8..48 carry kind-specific little-endian arguments. Bytes 48..64 must be
/// zero. Attach only names the App client. Focus carries client, optional
/// app, and a nonzero transition id. Appearance updates carry one fixed action
/// and a nonzero client-local request id. A boot-notification dismissal carries
/// only its nonzero client-local request id. A system-UI update packs its
/// closed action and recent-identity kind into argument zero, followed by a
/// nonzero contiguous request id, nonzero observed revision, and (only for a
/// compatible Activity) a nonzero boot-local session id and package
/// generation. Unused arguments are
/// zero. An AndroidBox execution report carries a contiguous request id, both
/// fixed-fixture checksums, signed result, bounded instruction count, closed
/// execution kind, and boot-local tap count in the same three arguments. An
/// Activity report uses an independent bounded request sequence and carries
/// the binary-manifest CRC-32, both DEX identities, restricted onCreate
/// method/code identity, bounded execution count, and a non-cryptographic
/// bounded view-text consistency label. Its Activity descriptor is fixed by
/// v8 rather than selected by the sender. A resource report adds one exact
/// resources.arsc/layout/string path in another independently sequenced kind;
/// its text label and length are cross-checked against the Activity report by
/// SurfaceServer audit state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiClientControl {
    payload: UiClientControlPayload,
}

impl UiClientControl {
    pub fn attach_app_endpoint(client: UiClientId) -> Result<Self, UiClientControlError> {
        if client != UiClientId::App {
            return Err(UiClientControlError::AttachRequiresAppClient);
        }
        Ok(Self {
            payload: UiClientControlPayload::AttachAppEndpoint { client },
        })
    }

    pub fn set_focus(
        target: UiClientId,
        app: Option<ShellAppId>,
        transition_id: u64,
    ) -> Result<Self, UiClientControlError> {
        if transition_id == 0 {
            return Err(UiClientControlError::ZeroTransitionId);
        }
        match (target, app) {
            (UiClientId::Launcher, Some(_)) => {
                return Err(UiClientControlError::LauncherMustNotSpecifyApp);
            }
            (UiClientId::App, None) => return Err(UiClientControlError::AppRequiresShellApp),
            _ => {}
        }
        Ok(Self {
            payload: UiClientControlPayload::SetFocus {
                target,
                app,
                transition_id,
            },
        })
    }

    pub fn update_appearance(
        action: UiAppearanceAction,
        request_id: u64,
    ) -> Result<Self, UiClientControlError> {
        if request_id == 0 {
            return Err(UiClientControlError::ZeroAppearanceRequestId);
        }
        Ok(Self {
            payload: UiClientControlPayload::UpdateAppearance { action, request_id },
        })
    }

    /// Requests the sole SurfaceServer-owned boot notification be dismissed.
    ///
    /// There is deliberately no inverse operation and no client-supplied
    /// notification payload. The fixed boot-local content can only transition
    /// from visible to dismissed inside its current SurfaceServer session.
    pub fn dismiss_boot_notification(request_id: u64) -> Result<Self, UiClientControlError> {
        if request_id == 0 {
            return Err(UiClientControlError::ZeroBootNotificationRequestId);
        }
        Ok(Self {
            payload: UiClientControlPayload::DismissBootNotification { request_id },
        })
    }

    /// Requests one bounded system-UI transition from SurfaceServer.
    ///
    /// Sender identity is authenticated by the channel owner and enforced by
    /// [`UiSystemUiSession`]. The wire itself never grants this authority.
    pub fn update_system_ui(
        action: UiSystemUiAction,
        app: Option<ShellAppId>,
        request_id: u64,
        observed_revision: u64,
    ) -> Result<Self, UiClientControlError> {
        Self::update_system_ui_recent(
            action,
            app.map(UiRecentIdentity::Shell),
            request_id,
            observed_revision,
        )
    }

    pub fn update_system_ui_recent(
        action: UiSystemUiAction,
        recent: Option<UiRecentIdentity>,
        request_id: u64,
        observed_revision: u64,
    ) -> Result<Self, UiClientControlError> {
        validate_system_ui_action_recent(action, recent)?;
        if request_id == 0 {
            return Err(UiClientControlError::ZeroSystemUiRequestId);
        }
        if observed_revision == 0 {
            return Err(UiClientControlError::ZeroObservedSystemUiRevision);
        }
        Ok(Self {
            payload: UiClientControlPayload::UpdateSystemUi {
                action,
                recent,
                request_id,
                observed_revision,
            },
        })
    }

    pub fn report_androidbox_execution(
        request_id: u64,
        kind: UiAndroidBoxExecutionKind,
        dex_crc32: u32,
        dex_adler32: u32,
        result: i32,
        instruction_count: u8,
        tap_count: u16,
    ) -> Result<Self, UiClientControlError> {
        Ok(Self {
            payload: UiClientControlPayload::ReportAndroidBoxExecution {
                report: UiAndroidBoxExecutionReport::new(
                    request_id,
                    kind,
                    dex_crc32,
                    dex_adler32,
                    result,
                    instruction_count,
                    tap_count,
                )?,
            },
        })
    }

    /// Reports completion of the fixed Activity-0 `onCreate` shim.
    ///
    /// `view_text` is consumed only to derive a bounded consistency label and
    /// length; no text bytes are sent over the control channel.
    #[allow(clippy::too_many_arguments)]
    pub fn report_androidbox_activity_execution(
        request_id: u32,
        lifecycle: UiAndroidBoxActivityLifecycleKind,
        manifest_crc32: u32,
        dex_crc32: u32,
        dex_adler32: u32,
        on_create_method_index: u16,
        on_create_code_offset: u32,
        instruction_count: u8,
        view_text: &[u8],
    ) -> Result<Self, UiClientControlError> {
        Ok(Self {
            payload: UiClientControlPayload::ReportAndroidBoxActivityExecution {
                report: UiAndroidBoxActivityExecutionReport::new(
                    request_id,
                    lifecycle,
                    manifest_crc32,
                    dex_crc32,
                    dex_adler32,
                    on_create_method_index,
                    on_create_code_offset,
                    instruction_count,
                    view_text,
                )?,
            },
        })
    }

    /// Reports completion of the fixed Activity-1 compiled-resource path.
    ///
    /// `view_text` is consumed only to derive the exact bounded consistency
    /// label and length. The five resource success bits are fixed by the
    /// constructor and are not caller-selectable.
    pub fn report_androidbox_resource_execution(
        request_id: u32,
        resources_arsc_crc32: u32,
        layout_xml_crc32: u32,
        layout_resource_id: u32,
        string_resource_id: u32,
        view_text: &[u8],
    ) -> Result<Self, UiClientControlError> {
        Ok(Self {
            payload: UiClientControlPayload::ReportAndroidBoxResourceExecution {
                report: UiAndroidBoxResourceExecutionReport::new(
                    request_id,
                    resources_arsc_crc32,
                    layout_xml_crc32,
                    layout_resource_id,
                    string_resource_id,
                    view_text,
                )?,
            },
        })
    }

    pub const fn kind(self) -> UiClientControlKind {
        match self.payload {
            UiClientControlPayload::AttachAppEndpoint { .. } => {
                UiClientControlKind::AttachAppEndpoint
            }
            UiClientControlPayload::SetFocus { .. } => UiClientControlKind::SetFocus,
            UiClientControlPayload::UpdateAppearance { .. } => {
                UiClientControlKind::UpdateAppearance
            }
            UiClientControlPayload::DismissBootNotification { .. } => {
                UiClientControlKind::DismissBootNotification
            }
            UiClientControlPayload::UpdateSystemUi { .. } => UiClientControlKind::UpdateSystemUi,
            UiClientControlPayload::ReportAndroidBoxExecution { .. } => {
                UiClientControlKind::ReportAndroidBoxExecution
            }
            UiClientControlPayload::ReportAndroidBoxActivityExecution { .. } => {
                UiClientControlKind::ReportAndroidBoxActivityExecution
            }
            UiClientControlPayload::ReportAndroidBoxResourceExecution { .. } => {
                UiClientControlKind::ReportAndroidBoxResourceExecution
            }
        }
    }

    pub const fn payload(self) -> UiClientControlPayload {
        self.payload
    }

    pub fn encode(self) -> [u8; UI_CLIENT_CONTROL_WIRE_SIZE] {
        let mut wire = [0; UI_CLIENT_CONTROL_WIRE_SIZE];
        wire[CONTROL_OFFSET_MAGIC..CONTROL_OFFSET_MAGIC + 4]
            .copy_from_slice(&UI_CLIENT_CONTROL_MAGIC.to_le_bytes());
        wire[CONTROL_OFFSET_VERSION..CONTROL_OFFSET_VERSION + 2]
            .copy_from_slice(&UI_CLIENT_CONTROL_VERSION.to_le_bytes());
        wire[CONTROL_OFFSET_KIND] = self.kind().raw();
        match self.payload {
            UiClientControlPayload::AttachAppEndpoint { client } => {
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, u64::from(client.raw()));
            }
            UiClientControlPayload::SetFocus {
                target,
                app,
                transition_id,
            } => {
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, u64::from(target.raw()));
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_APP,
                    app.map_or(0, |app| u64::from(app.raw())),
                );
                write_control_u64(&mut wire, CONTROL_OFFSET_TRANSITION_ID, transition_id);
            }
            UiClientControlPayload::UpdateAppearance { action, request_id } => {
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, u64::from(action.raw()));
                write_control_u64(&mut wire, CONTROL_OFFSET_APP, request_id);
            }
            UiClientControlPayload::DismissBootNotification { request_id } => {
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, request_id);
            }
            UiClientControlPayload::UpdateSystemUi {
                action,
                recent,
                request_id,
                observed_revision,
            } => {
                let (packed, session_id, package_generation) =
                    pack_system_ui_action(action, recent);
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, packed);
                write_control_u64(&mut wire, CONTROL_OFFSET_APP, request_id);
                write_control_u64(&mut wire, CONTROL_OFFSET_TRANSITION_ID, observed_revision);
                write_control_u64(&mut wire, CONTROL_OFFSET_ARGUMENT_3, session_id);
                write_control_u64(&mut wire, CONTROL_OFFSET_ARGUMENT_4, package_generation);
            }
            UiClientControlPayload::ReportAndroidBoxExecution { report } => {
                write_control_u64(&mut wire, CONTROL_OFFSET_CLIENT, report.request_id());
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_APP,
                    pack_androidbox_checksums(report),
                );
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_TRANSITION_ID,
                    pack_androidbox_execution(report),
                );
            }
            UiClientControlPayload::ReportAndroidBoxActivityExecution { report } => {
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_CLIENT,
                    pack_androidbox_activity_identity(report),
                );
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_APP,
                    pack_androidbox_activity_checksums(report),
                );
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_TRANSITION_ID,
                    pack_androidbox_activity_execution(report),
                );
            }
            UiClientControlPayload::ReportAndroidBoxResourceExecution { report } => {
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_CLIENT,
                    pack_androidbox_resource_identity(report),
                );
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_APP,
                    pack_androidbox_resource_layout(report),
                );
                write_control_u64(
                    &mut wire,
                    CONTROL_OFFSET_TRANSITION_ID,
                    pack_androidbox_resource_execution(report),
                );
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, UiClientControlError> {
        if wire.len() != UI_CLIENT_CONTROL_WIRE_SIZE {
            return Err(UiClientControlError::InvalidWireLength);
        }
        if read_u32(wire, CONTROL_OFFSET_MAGIC) != UI_CLIENT_CONTROL_MAGIC {
            return Err(UiClientControlError::InvalidMagic);
        }
        if read_u16(wire, CONTROL_OFFSET_VERSION) != UI_CLIENT_CONTROL_VERSION {
            return Err(UiClientControlError::InvalidVersion);
        }
        let kind = UiClientControlKind::from_raw(wire[CONTROL_OFFSET_KIND])
            .ok_or(UiClientControlError::InvalidKind)?;
        if wire[CONTROL_OFFSET_HEADER_RESERVED] != 0 {
            return Err(UiClientControlError::NonZeroHeaderReserved);
        }
        if wire[CONTROL_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(UiClientControlError::NonZeroBodyReserved);
        }
        let argument_0 = read_u64(wire, CONTROL_OFFSET_CLIENT);
        let argument_1 = read_u64(wire, CONTROL_OFFSET_APP);
        let argument_2 = read_u64(wire, CONTROL_OFFSET_TRANSITION_ID);
        let argument_3 = read_u64(wire, CONTROL_OFFSET_ARGUMENT_3);
        let argument_4 = read_u64(wire, CONTROL_OFFSET_ARGUMENT_4);
        match kind {
            UiClientControlKind::AttachAppEndpoint => {
                if argument_1 != 0 || argument_2 != 0 || argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                let client =
                    UiClientId::from_raw(argument_0).ok_or(UiClientControlError::InvalidClient)?;
                Self::attach_app_endpoint(client)
            }
            UiClientControlKind::SetFocus => {
                if argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                let client =
                    UiClientId::from_raw(argument_0).ok_or(UiClientControlError::InvalidClient)?;
                let app = if argument_1 == 0 {
                    None
                } else {
                    Some(ShellAppId::from_raw(argument_1).ok_or(UiClientControlError::InvalidApp)?)
                };
                Self::set_focus(client, app, argument_2)
            }
            UiClientControlKind::UpdateAppearance => {
                if argument_2 != 0 || argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                let action = UiAppearanceAction::from_raw(argument_0)
                    .ok_or(UiClientControlError::InvalidAppearanceAction)?;
                Self::update_appearance(action, argument_1)
            }
            UiClientControlKind::DismissBootNotification => {
                if argument_1 != 0 || argument_2 != 0 || argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                Self::dismiss_boot_notification(argument_0)
            }
            UiClientControlKind::UpdateSystemUi => {
                let (action, recent) = unpack_system_ui_action(argument_0, argument_3, argument_4)?;
                Self::update_system_ui_recent(action, recent, argument_1, argument_2)
            }
            UiClientControlKind::ReportAndroidBoxExecution => {
                if argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                Ok(Self {
                    payload: UiClientControlPayload::ReportAndroidBoxExecution {
                        report: unpack_androidbox_execution(argument_0, argument_1, argument_2)?,
                    },
                })
            }
            UiClientControlKind::ReportAndroidBoxActivityExecution => {
                if argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                Ok(Self {
                    payload: UiClientControlPayload::ReportAndroidBoxActivityExecution {
                        report: unpack_androidbox_activity_execution(
                            argument_0, argument_1, argument_2,
                        )?,
                    },
                })
            }
            UiClientControlKind::ReportAndroidBoxResourceExecution => {
                if argument_3 != 0 || argument_4 != 0 {
                    return Err(UiClientControlError::UnexpectedPayload);
                }
                Ok(Self {
                    payload: UiClientControlPayload::ReportAndroidBoxResourceExecution {
                        report: unpack_androidbox_resource_execution(
                            argument_0, argument_1, argument_2,
                        )?,
                    },
                })
            }
        }
    }
}

fn write_control_u64(wire: &mut [u8; UI_CLIENT_CONTROL_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiClientControlTrackerError {
    TransitionExhausted,
    TransitionReplay,
}

/// Transactional ordering state for focus controls. Transition ids may skip,
/// but every accepted SetFocus must be strictly greater than the preceding
/// one. Non-focus control messages do not affect focus ordering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiClientControlTracker {
    last_transition_id: Option<u64>,
    active_client: Option<UiClientId>,
    active_app: Option<ShellAppId>,
}

impl UiClientControlTracker {
    pub const fn new() -> Self {
        Self {
            last_transition_id: None,
            active_client: None,
            active_app: None,
        }
    }

    pub fn accept(&mut self, control: UiClientControl) -> Result<(), UiClientControlTrackerError> {
        let UiClientControlPayload::SetFocus {
            target,
            app,
            transition_id,
        } = control.payload()
        else {
            return Ok(());
        };
        if let Some(previous) = self.last_transition_id {
            if previous == u64::MAX {
                return Err(UiClientControlTrackerError::TransitionExhausted);
            }
            if transition_id <= previous {
                return Err(UiClientControlTrackerError::TransitionReplay);
            }
        }
        self.last_transition_id = Some(transition_id);
        self.active_client = Some(target);
        self.active_app = app;
        Ok(())
    }

    pub const fn last_transition_id(self) -> Option<u64> {
        self.last_transition_id
    }

    pub const fn active_client(self) -> Option<UiClientId> {
        self.active_client
    }

    pub const fn active_app(self) -> Option<ShellAppId> {
        self.active_app
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAndroidBoxAuditSessionError {
    LauncherOnly,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    FirstReportMustBeBoot,
    SubsequentReportMustBeTap,
    FixtureChecksumMismatch,
    TapCountExhausted,
    TapReplay,
    TapGap,
}

/// SurfaceServer-owned audit state for one fixed AndroidBox DEX-0 fixture.
///
/// This state is allocation-free, accepts only Launcher's authenticated
/// control stream, and commits transactionally after all request, fixture, and
/// tap sequence checks pass. It neither echoes the report nor grants a handle,
/// capability, or execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxAuditSession {
    last_request_id: u64,
    fixture_crc32: u32,
    fixture_adler32: u32,
    last_tap_count: u16,
    last_report: Option<UiAndroidBoxExecutionReport>,
}

impl Default for UiAndroidBoxAuditSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiAndroidBoxAuditSession {
    pub const fn new() -> Self {
        Self {
            last_request_id: 0,
            fixture_crc32: 0,
            fixture_adler32: 0,
            last_tap_count: 0,
            last_report: None,
        }
    }

    pub fn accept(
        &mut self,
        client: UiClientId,
        report: UiAndroidBoxExecutionReport,
    ) -> Result<(), UiAndroidBoxAuditSessionError> {
        if client != UiClientId::Launcher {
            return Err(UiAndroidBoxAuditSessionError::LauncherOnly);
        }
        let expected_request_id = self
            .last_request_id
            .checked_add(1)
            .ok_or(UiAndroidBoxAuditSessionError::RequestIdExhausted)?;
        if report.request_id() < expected_request_id {
            return Err(UiAndroidBoxAuditSessionError::RequestReplay);
        }
        if report.request_id() > expected_request_id {
            return Err(UiAndroidBoxAuditSessionError::RequestGap);
        }

        if self.last_report.is_none() {
            if report.kind() != UiAndroidBoxExecutionKind::Boot {
                return Err(UiAndroidBoxAuditSessionError::FirstReportMustBeBoot);
            }
        } else {
            if report.kind() != UiAndroidBoxExecutionKind::Tap {
                return Err(UiAndroidBoxAuditSessionError::SubsequentReportMustBeTap);
            }
            if report.dex_crc32() != self.fixture_crc32
                || report.dex_adler32() != self.fixture_adler32
            {
                return Err(UiAndroidBoxAuditSessionError::FixtureChecksumMismatch);
            }
            let expected_tap_count = self
                .last_tap_count
                .checked_add(1)
                .ok_or(UiAndroidBoxAuditSessionError::TapCountExhausted)?;
            if report.tap_count() < expected_tap_count {
                return Err(UiAndroidBoxAuditSessionError::TapReplay);
            }
            if report.tap_count() > expected_tap_count {
                return Err(UiAndroidBoxAuditSessionError::TapGap);
            }
        }

        if self.last_report.is_none() {
            self.fixture_crc32 = report.dex_crc32();
            self.fixture_adler32 = report.dex_adler32();
        }
        self.last_request_id = report.request_id();
        self.last_tap_count = report.tap_count();
        self.last_report = Some(report);
        Ok(())
    }

    pub const fn last_request_id(self) -> u64 {
        self.last_request_id
    }

    pub const fn fixture_crc32(self) -> Option<u32> {
        if self.last_report.is_some() {
            Some(self.fixture_crc32)
        } else {
            None
        }
    }

    pub const fn fixture_adler32(self) -> Option<u32> {
        if self.last_report.is_some() {
            Some(self.fixture_adler32)
        } else {
            None
        }
    }

    pub const fn last_tap_count(self) -> Option<u16> {
        if self.last_report.is_some() {
            Some(self.last_tap_count)
        } else {
            None
        }
    }

    pub const fn last_report(self) -> Option<UiAndroidBoxExecutionReport> {
        self.last_report
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAndroidBoxActivityAuditSessionError {
    LauncherOnly,
    MissingDexBoot,
    DexPredecessorMustBeBoot,
    DexIdentityMismatch,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    IdentityDrift,
    LifecycleAlreadyReported,
}

/// SurfaceServer-owned audit state for the fixed AndroidBox Activity-0 shim.
///
/// Activity request ids form their own contiguous sequence beginning at one.
/// The single accepted `on-create-complete` report must immediately follow an
/// accepted DEX-0 Boot identity in the companion audit session. No reply,
/// handle, endpoint, capability, or lifecycle authority is produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxActivityAuditSession {
    last_request_id: u32,
    dex_boot_request_id: u64,
    last_report: Option<UiAndroidBoxActivityExecutionReport>,
}

impl Default for UiAndroidBoxActivityAuditSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiAndroidBoxActivityAuditSession {
    pub const fn new() -> Self {
        Self {
            last_request_id: 0,
            dex_boot_request_id: 0,
            last_report: None,
        }
    }

    pub fn accept(
        &mut self,
        client: UiClientId,
        dex_audit: &UiAndroidBoxAuditSession,
        report: UiAndroidBoxActivityExecutionReport,
    ) -> Result<(), UiAndroidBoxActivityAuditSessionError> {
        if client != UiClientId::Launcher {
            return Err(UiAndroidBoxActivityAuditSessionError::LauncherOnly);
        }
        let Some(dex_predecessor) = dex_audit.last_report() else {
            return Err(UiAndroidBoxActivityAuditSessionError::MissingDexBoot);
        };
        if dex_predecessor.kind() != UiAndroidBoxExecutionKind::Boot {
            return Err(UiAndroidBoxActivityAuditSessionError::DexPredecessorMustBeBoot);
        }
        if report.dex_crc32() != dex_predecessor.dex_crc32()
            || report.dex_adler32() != dex_predecessor.dex_adler32()
        {
            return Err(UiAndroidBoxActivityAuditSessionError::DexIdentityMismatch);
        }

        let expected_request_id = self
            .last_request_id
            .checked_add(1)
            .ok_or(UiAndroidBoxActivityAuditSessionError::RequestIdExhausted)?;
        if report.request_id() < expected_request_id {
            return Err(UiAndroidBoxActivityAuditSessionError::RequestReplay);
        }
        if report.request_id() > expected_request_id {
            return Err(UiAndroidBoxActivityAuditSessionError::RequestGap);
        }

        if let Some(previous) = self.last_report {
            if !androidbox_activity_identity_matches(previous, report) {
                return Err(UiAndroidBoxActivityAuditSessionError::IdentityDrift);
            }
            return Err(UiAndroidBoxActivityAuditSessionError::LifecycleAlreadyReported);
        }

        self.last_request_id = report.request_id();
        self.dex_boot_request_id = dex_predecessor.request_id();
        self.last_report = Some(report);
        Ok(())
    }

    pub const fn last_request_id(self) -> u32 {
        self.last_request_id
    }

    pub const fn dex_boot_request_id(self) -> Option<u64> {
        if self.last_report.is_some() {
            Some(self.dex_boot_request_id)
        } else {
            None
        }
    }

    pub const fn last_report(self) -> Option<UiAndroidBoxActivityExecutionReport> {
        self.last_report
    }
}

fn androidbox_activity_identity_matches(
    previous: UiAndroidBoxActivityExecutionReport,
    next: UiAndroidBoxActivityExecutionReport,
) -> bool {
    previous.lifecycle() == next.lifecycle()
        && previous.manifest_crc32() == next.manifest_crc32()
        && previous.dex_crc32() == next.dex_crc32()
        && previous.dex_adler32() == next.dex_adler32()
        && previous.activity_descriptor_crc32() == next.activity_descriptor_crc32()
        && previous.on_create_method_index() == next.on_create_method_index()
        && previous.on_create_code_offset() == next.on_create_code_offset()
        && previous.instruction_count() == next.instruction_count()
        && previous.view_text_label() == next.view_text_label()
        && previous.view_text_length() == next.view_text_length()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAndroidBoxResourceAuditSessionError {
    LauncherOnly,
    MissingDexBoot,
    DexPredecessorMustBeBoot,
    MissingActivity,
    ActivityBootMismatch,
    ViewIdentityMismatch,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    IdentityDrift,
    ResourceAlreadyReported,
}

/// SurfaceServer-owned audit state for one Activity-1 resource resolution.
///
/// Resource request ids form a third independent contiguous sequence beginning
/// at one. The sole report must follow the accepted DEX Boot and Activity
/// reports from the same audit chain, and its text label/length must equal the
/// Activity result. Rejection is transactional and produces no reply, handle,
/// endpoint, capability, or execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAndroidBoxResourceAuditSession {
    last_request_id: u32,
    dex_boot_request_id: u64,
    activity_request_id: u32,
    last_report: Option<UiAndroidBoxResourceExecutionReport>,
}

impl Default for UiAndroidBoxResourceAuditSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiAndroidBoxResourceAuditSession {
    pub const fn new() -> Self {
        Self {
            last_request_id: 0,
            dex_boot_request_id: 0,
            activity_request_id: 0,
            last_report: None,
        }
    }

    pub fn accept(
        &mut self,
        client: UiClientId,
        dex_audit: &UiAndroidBoxAuditSession,
        activity_audit: &UiAndroidBoxActivityAuditSession,
        report: UiAndroidBoxResourceExecutionReport,
    ) -> Result<(), UiAndroidBoxResourceAuditSessionError> {
        if client != UiClientId::Launcher {
            return Err(UiAndroidBoxResourceAuditSessionError::LauncherOnly);
        }
        let Some(dex_predecessor) = dex_audit.last_report() else {
            return Err(UiAndroidBoxResourceAuditSessionError::MissingDexBoot);
        };
        if dex_predecessor.kind() != UiAndroidBoxExecutionKind::Boot {
            return Err(UiAndroidBoxResourceAuditSessionError::DexPredecessorMustBeBoot);
        }
        let Some(activity_predecessor) = activity_audit.last_report() else {
            return Err(UiAndroidBoxResourceAuditSessionError::MissingActivity);
        };
        if activity_audit.dex_boot_request_id() != Some(dex_predecessor.request_id()) {
            return Err(UiAndroidBoxResourceAuditSessionError::ActivityBootMismatch);
        }
        if report.view_text_label() != activity_predecessor.view_text_label()
            || report.view_text_length() != activity_predecessor.view_text_length()
        {
            return Err(UiAndroidBoxResourceAuditSessionError::ViewIdentityMismatch);
        }

        let expected_request_id = self
            .last_request_id
            .checked_add(1)
            .ok_or(UiAndroidBoxResourceAuditSessionError::RequestIdExhausted)?;
        if report.request_id() < expected_request_id {
            return Err(UiAndroidBoxResourceAuditSessionError::RequestReplay);
        }
        if report.request_id() > expected_request_id {
            return Err(UiAndroidBoxResourceAuditSessionError::RequestGap);
        }

        if let Some(previous) = self.last_report {
            if !androidbox_resource_identity_matches(previous, report) {
                return Err(UiAndroidBoxResourceAuditSessionError::IdentityDrift);
            }
            return Err(UiAndroidBoxResourceAuditSessionError::ResourceAlreadyReported);
        }

        self.last_request_id = report.request_id();
        self.dex_boot_request_id = dex_predecessor.request_id();
        self.activity_request_id = activity_predecessor.request_id();
        self.last_report = Some(report);
        Ok(())
    }

    pub const fn last_request_id(self) -> u32 {
        self.last_request_id
    }

    pub const fn dex_boot_request_id(self) -> Option<u64> {
        if self.last_report.is_some() {
            Some(self.dex_boot_request_id)
        } else {
            None
        }
    }

    pub const fn activity_request_id(self) -> Option<u32> {
        if self.last_report.is_some() {
            Some(self.activity_request_id)
        } else {
            None
        }
    }

    pub const fn last_report(self) -> Option<UiAndroidBoxResourceExecutionReport> {
        self.last_report
    }
}

fn androidbox_resource_identity_matches(
    previous: UiAndroidBoxResourceExecutionReport,
    next: UiAndroidBoxResourceExecutionReport,
) -> bool {
    previous.resources_arsc_crc32() == next.resources_arsc_crc32()
        && previous.layout_xml_crc32() == next.layout_xml_crc32()
        && previous.layout_resource_id() == next.layout_resource_id()
        && previous.string_resource_id() == next.string_resource_id()
        && previous.view_text_label() == next.view_text_label()
        && previous.view_text_length() == next.view_text_length()
        && previous.success_flags() == next.success_flags()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAppearanceSessionError {
    ZeroRequestId,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    RevisionExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAppearanceUpdate {
    appearance: UiAppearance,
    revision: u64,
}

impl UiAppearanceUpdate {
    pub const fn appearance(self) -> UiAppearance {
        self.appearance
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// SurfaceServer-owned state for one non-persistent UI session.
///
/// Launcher and App have independent, contiguous request-id streams. Every
/// accepted action advances the one shared appearance revision exactly once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAppearanceSession {
    appearance: UiAppearance,
    revision: u64,
    launcher_request_id: u64,
    app_request_id: u64,
}

impl Default for UiAppearanceSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiAppearanceSession {
    pub const fn new() -> Self {
        Self {
            appearance: UiAppearance::new(true, false),
            revision: 1,
            launcher_request_id: 0,
            app_request_id: 0,
        }
    }

    pub fn apply(
        &mut self,
        client: UiClientId,
        action: UiAppearanceAction,
        request_id: u64,
    ) -> Result<UiAppearanceUpdate, UiAppearanceSessionError> {
        if request_id == 0 {
            return Err(UiAppearanceSessionError::ZeroRequestId);
        }
        let previous_request_id = match client {
            UiClientId::Launcher => self.launcher_request_id,
            UiClientId::App => self.app_request_id,
        };
        let expected_request_id = previous_request_id
            .checked_add(1)
            .ok_or(UiAppearanceSessionError::RequestIdExhausted)?;
        if request_id < expected_request_id {
            return Err(UiAppearanceSessionError::RequestReplay);
        }
        if request_id > expected_request_id {
            return Err(UiAppearanceSessionError::RequestGap);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(UiAppearanceSessionError::RevisionExhausted)?;
        let appearance = self.appearance.applying(action);

        self.appearance = appearance;
        self.revision = revision;
        match client {
            UiClientId::Launcher => self.launcher_request_id = request_id,
            UiClientId::App => self.app_request_id = request_id,
        }
        Ok(UiAppearanceUpdate {
            appearance,
            revision,
        })
    }

    pub const fn appearance(self) -> UiAppearance {
        self.appearance
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn last_request_id(self, client: UiClientId) -> u64 {
        match client {
            UiClientId::Launcher => self.launcher_request_id,
            UiClientId::App => self.app_request_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBootNotificationSessionError {
    ZeroRequestId,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    AlreadyDismissed,
    RevisionExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiBootNotificationUpdate {
    visible: bool,
    revision: u64,
}

impl UiBootNotificationUpdate {
    pub const fn visible(self) -> bool {
        self.visible
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// SurfaceServer-owned state for the sole fixed boot-local notification.
///
/// Every new SurfaceServer session begins visible at revision one. Launcher
/// and App have independent contiguous request-id streams, but only the first
/// accepted dismissal can change the one shared state. There is intentionally
/// no post, text, update, or re-show operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiBootNotificationSession {
    visible: bool,
    revision: u64,
    launcher_request_id: u64,
    app_request_id: u64,
}

impl Default for UiBootNotificationSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiBootNotificationSession {
    pub const fn new() -> Self {
        Self {
            visible: true,
            revision: 1,
            launcher_request_id: 0,
            app_request_id: 0,
        }
    }

    pub fn dismiss(
        &mut self,
        client: UiClientId,
        request_id: u64,
    ) -> Result<UiBootNotificationUpdate, UiBootNotificationSessionError> {
        if request_id == 0 {
            return Err(UiBootNotificationSessionError::ZeroRequestId);
        }
        let previous_request_id = match client {
            UiClientId::Launcher => self.launcher_request_id,
            UiClientId::App => self.app_request_id,
        };
        let expected_request_id = previous_request_id
            .checked_add(1)
            .ok_or(UiBootNotificationSessionError::RequestIdExhausted)?;
        if request_id < expected_request_id {
            return Err(UiBootNotificationSessionError::RequestReplay);
        }
        if request_id > expected_request_id {
            return Err(UiBootNotificationSessionError::RequestGap);
        }
        if !self.visible {
            return Err(UiBootNotificationSessionError::AlreadyDismissed);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(UiBootNotificationSessionError::RevisionExhausted)?;

        self.visible = false;
        self.revision = revision;
        match client {
            UiClientId::Launcher => self.launcher_request_id = request_id,
            UiClientId::App => self.app_request_id = request_id,
        }
        Ok(UiBootNotificationUpdate {
            visible: false,
            revision,
        })
    }

    pub const fn visible(self) -> bool {
        self.visible
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn last_request_id(self, client: UiClientId) -> u64 {
        match client {
            UiClientId::Launcher => self.launcher_request_id,
            UiClientId::App => self.app_request_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSystemUiStateError {
    NavRevealOutOfRange,
    NavRevealNotQuantized,
    NavRevealRequiresPress,
    ForegroundRequiresRecentApp,
}

fn validate_system_ui_state(
    mode: UiSystemUiMode,
    recent: Option<UiRecentIdentity>,
    nav_pressed: bool,
    nav_reveal_px: u16,
) -> Result<(), UiSystemUiStateError> {
    if nav_reveal_px > UI_SYSTEM_UI_NAV_REVEAL_MAX_PX {
        return Err(UiSystemUiStateError::NavRevealOutOfRange);
    }
    if !nav_reveal_px.is_multiple_of(UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX) {
        return Err(UiSystemUiStateError::NavRevealNotQuantized);
    }
    if !nav_pressed && nav_reveal_px != 0 {
        return Err(UiSystemUiStateError::NavRevealRequiresPress);
    }
    if mode == UiSystemUiMode::Foreground && recent.is_none() {
        return Err(UiSystemUiStateError::ForegroundRequiresRecentApp);
    }
    Ok(())
}

const SYSTEM_UI_STATE_MODE_SHIFT: u32 = 0;
const SYSTEM_UI_STATE_MODE_MASK: u64 = 0xff << SYSTEM_UI_STATE_MODE_SHIFT;
const SYSTEM_UI_STATE_RECENT_KIND_SHIFT: u32 = 8;
const SYSTEM_UI_STATE_RECENT_KIND_MASK: u64 = 0xff << SYSTEM_UI_STATE_RECENT_KIND_SHIFT;
const SYSTEM_UI_STATE_SHELL_APP_SHIFT: u32 = 16;
const SYSTEM_UI_STATE_SHELL_APP_MASK: u64 = 0xff << SYSTEM_UI_STATE_SHELL_APP_SHIFT;
const SYSTEM_UI_STATE_NAV_PRESSED_SHIFT: u32 = 24;
const SYSTEM_UI_STATE_NAV_PRESSED_MASK: u64 = 1 << SYSTEM_UI_STATE_NAV_PRESSED_SHIFT;
const SYSTEM_UI_STATE_NAV_REVEAL_SHIFT: u32 = 25;
const SYSTEM_UI_STATE_NAV_REVEAL_MASK: u64 = 0x3f << SYSTEM_UI_STATE_NAV_REVEAL_SHIFT;
const SYSTEM_UI_STATE_KNOWN_MASK: u64 = SYSTEM_UI_STATE_MODE_MASK
    | SYSTEM_UI_STATE_RECENT_KIND_MASK
    | SYSTEM_UI_STATE_SHELL_APP_MASK
    | SYSTEM_UI_STATE_NAV_PRESSED_MASK
    | SYSTEM_UI_STATE_NAV_REVEAL_MASK;

fn pack_system_ui_state(
    mode: UiSystemUiMode,
    recent: Option<UiRecentIdentity>,
    nav_pressed: bool,
    nav_reveal_px: u16,
) -> (u64, u64, u64) {
    let reveal_steps = u64::from(nav_reveal_px / UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX);
    let (kind, shell_app, session_id, package_generation) = match recent {
        None => (SYSTEM_UI_RECENT_KIND_NONE, 0, 0, 0),
        Some(UiRecentIdentity::Shell(app)) => {
            (SYSTEM_UI_RECENT_KIND_SHELL, u64::from(app.raw()), 0, 0)
        }
        Some(UiRecentIdentity::CompatibleAndroid(identity)) => (
            SYSTEM_UI_RECENT_KIND_COMPATIBLE_ANDROID,
            0,
            identity.session_id(),
            identity.package_generation(),
        ),
    };
    (
        u64::from(mode.raw())
            | (u64::from(kind) << SYSTEM_UI_STATE_RECENT_KIND_SHIFT)
            | (shell_app << SYSTEM_UI_STATE_SHELL_APP_SHIFT)
            | (u64::from(nav_pressed) << SYSTEM_UI_STATE_NAV_PRESSED_SHIFT)
            | (reveal_steps << SYSTEM_UI_STATE_NAV_REVEAL_SHIFT),
        session_id,
        package_generation,
    )
}

fn unpack_system_ui_state(
    packed: u64,
    session_id: u64,
    package_generation: u64,
) -> Result<(UiSystemUiMode, Option<UiRecentIdentity>, bool, u16), UiServerEventError> {
    if packed & !SYSTEM_UI_STATE_KNOWN_MASK != 0 {
        return Err(UiServerEventError::NonCanonicalSystemUiState);
    }
    let mode_raw = (packed & SYSTEM_UI_STATE_MODE_MASK) >> SYSTEM_UI_STATE_MODE_SHIFT;
    let mode = UiSystemUiMode::from_raw(mode_raw).ok_or(UiServerEventError::InvalidSystemUiMode)?;
    let recent_kind =
        (packed & SYSTEM_UI_STATE_RECENT_KIND_MASK) >> SYSTEM_UI_STATE_RECENT_KIND_SHIFT;
    let shell_app_raw =
        (packed & SYSTEM_UI_STATE_SHELL_APP_MASK) >> SYSTEM_UI_STATE_SHELL_APP_SHIFT;
    let recent = match recent_kind {
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_NONE) => {
            if shell_app_raw != 0 || session_id != 0 || package_generation != 0 {
                return Err(UiServerEventError::NonCanonicalSystemUiState);
            }
            None
        }
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_SHELL) => {
            if shell_app_raw == 0 || session_id != 0 || package_generation != 0 {
                return Err(UiServerEventError::NonCanonicalSystemUiState);
            }
            let app =
                ShellAppId::from_raw(shell_app_raw).ok_or(UiServerEventError::InvalidRecentApp)?;
            Some(UiRecentIdentity::Shell(app))
        }
        raw if raw == u64::from(SYSTEM_UI_RECENT_KIND_COMPATIBLE_ANDROID) => {
            if shell_app_raw != 0 {
                return Err(UiServerEventError::NonCanonicalSystemUiState);
            }
            let identity = UiCompatibleActivityIdentity::new(session_id, package_generation)
                .ok_or(UiServerEventError::InvalidCompatibleActivityIdentity)?;
            Some(UiRecentIdentity::CompatibleAndroid(identity))
        }
        _ => return Err(UiServerEventError::InvalidRecentIdentity),
    };
    let nav_pressed = packed & SYSTEM_UI_STATE_NAV_PRESSED_MASK != 0;
    let reveal_steps =
        (packed & SYSTEM_UI_STATE_NAV_REVEAL_MASK) >> SYSTEM_UI_STATE_NAV_REVEAL_SHIFT;
    let reveal_steps =
        u16::try_from(reveal_steps).map_err(|_| UiServerEventError::NonCanonicalSystemUiState)?;
    let nav_reveal_px = reveal_steps
        .checked_mul(UI_SYSTEM_UI_NAV_REVEAL_QUANTUM_PX)
        .ok_or(UiServerEventError::InvalidSystemUiState(
            UiSystemUiStateError::NavRevealOutOfRange,
        ))?;
    validate_system_ui_state(mode, recent, nav_pressed, nav_reveal_px)
        .map_err(UiServerEventError::InvalidSystemUiState)?;
    Ok((mode, recent, nav_pressed, nav_reveal_px))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSystemUiSessionError {
    LauncherOnly,
    ZeroRequestId,
    RequestIdExhausted,
    RequestReplay,
    RequestGap,
    ZeroObservedRevision,
    ObservedRevisionMismatch,
    RevisionExhausted,
    FocusWhileLocked,
    FocusAppRequiresHome,
    NavigationAlreadyPressed,
    NavigationNotPressed,
    NavigationInProgress,
    InvalidNavigationFinishMode,
    LockedNavigationMustCancel,
    InvalidState(UiSystemUiStateError),
    UnlockRequiresLocked,
    CloseOverviewRequiresOverview,
    ActivateRecentRequiresOverview,
    PresentCompatibleRequiresHome,
    HomeCompatibleRequiresForeground,
    FinishCompatibleRequiresStableMode,
    ReserveCompatibleRequiresHomeOrOverview,
    CompatibleActivityReservationInProgress,
    CompatibleActivityReservationRequired,
    CompatibleActivityReservationIdentityMismatch,
    CompatibleActivityReservationOriginMismatch,
    RecentAppMismatch,
}

/// The stable system-UI surface on which compatible-Activity verification was
/// reserved.
///
/// The origin is immutable for the reservation lifetime. It prevents a
/// successful verifier result obtained for a Home launch from being committed
/// as an Overview reactivation, or vice versa.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiCompatibleActivityReservationOrigin {
    Home = 1,
    Overview = 2,
}

impl UiCompatibleActivityReservationOrigin {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Home),
            2 => Some(Self::Overview),
            _ => None,
        }
    }
}

/// One canonical, boot-local compatible-Activity verification reservation.
///
/// A reservation is identity-only authority. It contains no package bytes,
/// verifier result, input capture state, pixels, or background-execution
/// capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiCompatibleActivityReservation {
    identity: UiCompatibleActivityIdentity,
    origin: UiCompatibleActivityReservationOrigin,
    request_id: u64,
    revision: u64,
}

impl UiCompatibleActivityReservation {
    pub const fn identity(self) -> UiCompatibleActivityIdentity {
        self.identity
    }

    pub const fn origin(self) -> UiCompatibleActivityReservationOrigin {
        self.origin
    }

    pub const fn request_id(self) -> u64 {
        self.request_id
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// One immutable publication emitted after a committed system-UI mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSystemUiUpdate {
    mode: UiSystemUiMode,
    recent: Option<UiRecentIdentity>,
    nav_pressed: bool,
    nav_reveal_px: u16,
    revision: u64,
}

impl UiSystemUiUpdate {
    pub const fn mode(self) -> UiSystemUiMode {
        self.mode
    }

    pub const fn recent(self) -> Option<UiRecentIdentity> {
        self.recent
    }

    pub const fn recent_app(self) -> Option<ShellAppId> {
        match self.recent {
            Some(recent) => recent.shell_app(),
            None => None,
        }
    }

    pub const fn nav_pressed(self) -> bool {
        self.nav_pressed
    }

    pub const fn nav_reveal_px(self) -> u16 {
        self.nav_reveal_px
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// SurfaceServer-owned, boot-local system-UI state.
///
/// The session retains at most one logical recent app. It deliberately has no
/// task termination, background-execution, snapshot, or persistence surface.
/// Every successful mutation advances the shared revision exactly once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSystemUiSession {
    mode: UiSystemUiMode,
    recent: Option<UiRecentIdentity>,
    nav_pressed: bool,
    nav_reveal_px: u16,
    revision: u64,
    launcher_request_id: u64,
    compatible_activity_reservation: Option<UiCompatibleActivityReservation>,
}

impl Default for UiSystemUiSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UiSystemUiSession {
    pub const fn new() -> Self {
        Self {
            mode: UiSystemUiMode::Locked,
            recent: None,
            nav_pressed: false,
            nav_reveal_px: 0,
            revision: 1,
            launcher_request_id: 0,
            compatible_activity_reservation: None,
        }
    }

    pub fn focus_app(
        &mut self,
        app: ShellAppId,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if self.mode == UiSystemUiMode::Locked {
            return Err(UiSystemUiSessionError::FocusWhileLocked);
        }
        if self.mode != UiSystemUiMode::Home {
            return Err(UiSystemUiSessionError::FocusAppRequiresHome);
        }
        self.commit(
            UiSystemUiMode::Foreground,
            Some(UiRecentIdentity::Shell(app)),
            false,
            0,
        )
    }

    pub fn focus_home(&mut self) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if self.mode == UiSystemUiMode::Locked {
            return Err(UiSystemUiSessionError::FocusWhileLocked);
        }
        self.commit(UiSystemUiMode::Home, self.recent, false, 0)
    }

    pub fn begin_nav(&mut self) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if self.nav_pressed {
            return Err(UiSystemUiSessionError::NavigationAlreadyPressed);
        }
        self.commit(self.mode, self.recent, true, 0)
    }

    pub fn update_nav(
        &mut self,
        nav_reveal_px: u16,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if !self.nav_pressed {
            return Err(UiSystemUiSessionError::NavigationNotPressed);
        }
        validate_system_ui_state(self.mode, self.recent, true, nav_reveal_px)
            .map_err(UiSystemUiSessionError::InvalidState)?;
        self.commit(self.mode, self.recent, true, nav_reveal_px)
    }

    /// Commits one server-classified navigation gesture atomically.
    ///
    /// Only stable Home and Overview destinations are admitted. A navigation
    /// capture that began while Locked must be cancelled instead.
    pub fn finish_nav(
        &mut self,
        target_mode: UiSystemUiMode,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if !matches!(target_mode, UiSystemUiMode::Home | UiSystemUiMode::Overview) {
            return Err(UiSystemUiSessionError::InvalidNavigationFinishMode);
        }
        if !self.nav_pressed {
            return Err(UiSystemUiSessionError::NavigationNotPressed);
        }
        if self.mode == UiSystemUiMode::Locked {
            return Err(UiSystemUiSessionError::LockedNavigationMustCancel);
        }
        self.commit(target_mode, self.recent, false, 0)
    }

    pub fn cancel_nav(&mut self) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.require_no_compatible_activity_reservation()?;
        if !self.nav_pressed {
            return Err(UiSystemUiSessionError::NavigationNotPressed);
        }
        self.commit(self.mode, self.recent, false, 0)
    }

    /// Applies one authenticated client request transactionally.
    ///
    /// Only Launcher is authorized. Every action observes the exact current
    /// revision, and request ids form one contiguous Launcher-local stream.
    pub fn apply_client_action(
        &mut self,
        client: UiClientId,
        action: UiSystemUiAction,
        app: Option<ShellAppId>,
        request_id: u64,
        observed_revision: u64,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        self.apply_client_recent_action(
            client,
            action,
            app.map(UiRecentIdentity::Shell),
            request_id,
            observed_revision,
        )
    }

    pub fn apply_client_recent_action(
        &mut self,
        client: UiClientId,
        action: UiSystemUiAction,
        recent: Option<UiRecentIdentity>,
        request_id: u64,
        observed_revision: u64,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        if client != UiClientId::Launcher {
            return Err(UiSystemUiSessionError::LauncherOnly);
        }
        if request_id == 0 {
            return Err(UiSystemUiSessionError::ZeroRequestId);
        }
        let expected_request_id = self
            .launcher_request_id
            .checked_add(1)
            .ok_or(UiSystemUiSessionError::RequestIdExhausted)?;
        if request_id < expected_request_id {
            return Err(UiSystemUiSessionError::RequestReplay);
        }
        if request_id > expected_request_id {
            return Err(UiSystemUiSessionError::RequestGap);
        }
        if observed_revision == 0 {
            return Err(UiSystemUiSessionError::ZeroObservedRevision);
        }
        if observed_revision != self.revision {
            return Err(UiSystemUiSessionError::ObservedRevisionMismatch);
        }
        if self.nav_pressed {
            return Err(UiSystemUiSessionError::NavigationInProgress);
        }

        let update = if self.compatible_activity_reservation.is_some() {
            self.apply_compatible_activity_reservation_action(action, recent)?
        } else {
            match action {
                UiSystemUiAction::Unlock => {
                    if recent.is_some() {
                        return Err(UiSystemUiSessionError::RecentAppMismatch);
                    }
                    if self.mode != UiSystemUiMode::Locked {
                        return Err(UiSystemUiSessionError::UnlockRequiresLocked);
                    }
                    self.commit(UiSystemUiMode::Home, None, false, 0)?
                }
                UiSystemUiAction::CloseOverview => {
                    if recent.is_some() {
                        return Err(UiSystemUiSessionError::RecentAppMismatch);
                    }
                    if self.mode != UiSystemUiMode::Overview {
                        return Err(UiSystemUiSessionError::CloseOverviewRequiresOverview);
                    }
                    self.commit(UiSystemUiMode::Home, self.recent, false, 0)?
                }
                UiSystemUiAction::ActivateRecent => {
                    if self.mode != UiSystemUiMode::Overview {
                        return Err(UiSystemUiSessionError::ActivateRecentRequiresOverview);
                    }
                    let requested = recent.ok_or(UiSystemUiSessionError::RecentAppMismatch)?;
                    if self.recent != Some(requested) {
                        return Err(UiSystemUiSessionError::RecentAppMismatch);
                    }
                    if requested.compatible_android().is_some() {
                        return Err(UiSystemUiSessionError::CompatibleActivityReservationRequired);
                    }
                    self.commit(UiSystemUiMode::Foreground, Some(requested), false, 0)?
                }
                UiSystemUiAction::PresentCompatibleActivity => {
                    return Err(UiSystemUiSessionError::CompatibleActivityReservationRequired);
                }
                UiSystemUiAction::HomeCompatibleActivity => {
                    if self.mode != UiSystemUiMode::Foreground {
                        return Err(UiSystemUiSessionError::HomeCompatibleRequiresForeground);
                    }
                    let requested = recent
                        .and_then(UiRecentIdentity::compatible_android)
                        .ok_or(UiSystemUiSessionError::RecentAppMismatch)?;
                    let requested = UiRecentIdentity::CompatibleAndroid(requested);
                    if self.recent != Some(requested) {
                        return Err(UiSystemUiSessionError::RecentAppMismatch);
                    }
                    self.commit(UiSystemUiMode::Home, Some(requested), false, 0)?
                }
                UiSystemUiAction::FinishCompatibleActivity => {
                    if !matches!(
                        self.mode,
                        UiSystemUiMode::Home
                            | UiSystemUiMode::Foreground
                            | UiSystemUiMode::Overview
                    ) {
                        return Err(UiSystemUiSessionError::FinishCompatibleRequiresStableMode);
                    }
                    let requested = recent
                        .and_then(UiRecentIdentity::compatible_android)
                        .ok_or(UiSystemUiSessionError::RecentAppMismatch)?;
                    if self.recent != Some(UiRecentIdentity::CompatibleAndroid(requested)) {
                        return Err(UiSystemUiSessionError::RecentAppMismatch);
                    }
                    let target = if self.mode == UiSystemUiMode::Foreground {
                        UiSystemUiMode::Home
                    } else {
                        self.mode
                    };
                    self.commit(target, None, false, 0)?
                }
                UiSystemUiAction::ReserveCompatibleActivity => {
                    // A successful reserve must always leave one contiguous
                    // request id and one revision available for commit/abort.
                    request_id
                        .checked_add(1)
                        .ok_or(UiSystemUiSessionError::RequestIdExhausted)?;
                    self.revision
                        .checked_add(2)
                        .ok_or(UiSystemUiSessionError::RevisionExhausted)?;
                    let identity = recent
                        .and_then(UiRecentIdentity::compatible_android)
                        .ok_or(UiSystemUiSessionError::RecentAppMismatch)?;
                    let origin = match self.mode {
                        UiSystemUiMode::Home => {
                            if let Some(UiRecentIdentity::CompatibleAndroid(existing)) = self.recent
                                && existing != identity
                            {
                                return Err(UiSystemUiSessionError::RecentAppMismatch);
                            }
                            UiCompatibleActivityReservationOrigin::Home
                        }
                        UiSystemUiMode::Overview => {
                            if self.recent != Some(UiRecentIdentity::CompatibleAndroid(identity)) {
                                return Err(UiSystemUiSessionError::RecentAppMismatch);
                            }
                            UiCompatibleActivityReservationOrigin::Overview
                        }
                        UiSystemUiMode::Locked | UiSystemUiMode::Foreground => {
                            return Err(
                                UiSystemUiSessionError::ReserveCompatibleRequiresHomeOrOverview,
                            );
                        }
                    };
                    let update = self.commit(self.mode, self.recent, false, 0)?;
                    self.compatible_activity_reservation = Some(UiCompatibleActivityReservation {
                        identity,
                        origin,
                        request_id,
                        revision: update.revision(),
                    });
                    update
                }
                UiSystemUiAction::AbortCompatibleActivityVerification => {
                    return Err(UiSystemUiSessionError::CompatibleActivityReservationRequired);
                }
            }
        };
        self.launcher_request_id = request_id;
        Ok(update)
    }

    fn apply_compatible_activity_reservation_action(
        &mut self,
        action: UiSystemUiAction,
        recent: Option<UiRecentIdentity>,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        let reservation = self
            .compatible_activity_reservation
            .ok_or(UiSystemUiSessionError::CompatibleActivityReservationRequired)?;
        if self.revision != reservation.revision {
            return Err(UiSystemUiSessionError::ObservedRevisionMismatch);
        }
        let requested = match action {
            UiSystemUiAction::PresentCompatibleActivity
            | UiSystemUiAction::ActivateRecent
            | UiSystemUiAction::FinishCompatibleActivity
            | UiSystemUiAction::AbortCompatibleActivityVerification => recent
                .and_then(UiRecentIdentity::compatible_android)
                .ok_or(UiSystemUiSessionError::CompatibleActivityReservationIdentityMismatch)?,
            UiSystemUiAction::Unlock
            | UiSystemUiAction::CloseOverview
            | UiSystemUiAction::HomeCompatibleActivity
            | UiSystemUiAction::ReserveCompatibleActivity => {
                return Err(UiSystemUiSessionError::CompatibleActivityReservationInProgress);
            }
        };
        if requested != reservation.identity {
            return Err(UiSystemUiSessionError::CompatibleActivityReservationIdentityMismatch);
        }

        let update = match action {
            UiSystemUiAction::PresentCompatibleActivity => {
                if reservation.origin != UiCompatibleActivityReservationOrigin::Home {
                    return Err(
                        UiSystemUiSessionError::CompatibleActivityReservationOriginMismatch,
                    );
                }
                if self.mode != UiSystemUiMode::Home {
                    return Err(UiSystemUiSessionError::PresentCompatibleRequiresHome);
                }
                self.commit(
                    UiSystemUiMode::Foreground,
                    Some(UiRecentIdentity::CompatibleAndroid(requested)),
                    false,
                    0,
                )?
            }
            UiSystemUiAction::ActivateRecent => {
                if reservation.origin != UiCompatibleActivityReservationOrigin::Overview {
                    return Err(
                        UiSystemUiSessionError::CompatibleActivityReservationOriginMismatch,
                    );
                }
                if self.mode != UiSystemUiMode::Overview {
                    return Err(UiSystemUiSessionError::ActivateRecentRequiresOverview);
                }
                if self.recent != Some(UiRecentIdentity::CompatibleAndroid(requested)) {
                    return Err(UiSystemUiSessionError::RecentAppMismatch);
                }
                self.commit(
                    UiSystemUiMode::Foreground,
                    Some(UiRecentIdentity::CompatibleAndroid(requested)),
                    false,
                    0,
                )?
            }
            UiSystemUiAction::AbortCompatibleActivityVerification => {
                self.commit(self.mode, self.recent, false, 0)?
            }
            UiSystemUiAction::FinishCompatibleActivity => {
                if !matches!(self.mode, UiSystemUiMode::Home | UiSystemUiMode::Overview) {
                    return Err(UiSystemUiSessionError::FinishCompatibleRequiresStableMode);
                }
                self.commit(self.mode, None, false, 0)?
            }
            UiSystemUiAction::Unlock
            | UiSystemUiAction::CloseOverview
            | UiSystemUiAction::HomeCompatibleActivity
            | UiSystemUiAction::ReserveCompatibleActivity => unreachable!(),
        };
        self.compatible_activity_reservation = None;
        Ok(update)
    }

    fn require_no_compatible_activity_reservation(&self) -> Result<(), UiSystemUiSessionError> {
        if self.compatible_activity_reservation.is_some() {
            return Err(UiSystemUiSessionError::CompatibleActivityReservationInProgress);
        }
        Ok(())
    }

    fn commit(
        &mut self,
        mode: UiSystemUiMode,
        recent: Option<UiRecentIdentity>,
        nav_pressed: bool,
        nav_reveal_px: u16,
    ) -> Result<UiSystemUiUpdate, UiSystemUiSessionError> {
        validate_system_ui_state(mode, recent, nav_pressed, nav_reveal_px)
            .map_err(UiSystemUiSessionError::InvalidState)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(UiSystemUiSessionError::RevisionExhausted)?;
        let update = UiSystemUiUpdate {
            mode,
            recent,
            nav_pressed,
            nav_reveal_px,
            revision,
        };
        self.mode = mode;
        self.recent = recent;
        self.nav_pressed = nav_pressed;
        self.nav_reveal_px = nav_reveal_px;
        self.revision = revision;
        Ok(update)
    }

    pub const fn snapshot(self) -> UiSystemUiUpdate {
        UiSystemUiUpdate {
            mode: self.mode,
            recent: self.recent,
            nav_pressed: self.nav_pressed,
            nav_reveal_px: self.nav_reveal_px,
            revision: self.revision,
        }
    }

    pub const fn mode(self) -> UiSystemUiMode {
        self.mode
    }

    pub const fn recent(self) -> Option<UiRecentIdentity> {
        self.recent
    }

    pub const fn recent_app(self) -> Option<ShellAppId> {
        match self.recent {
            Some(recent) => recent.shell_app(),
            None => None,
        }
    }

    pub const fn nav_pressed(self) -> bool {
        self.nav_pressed
    }

    pub const fn nav_reveal_px(self) -> u16 {
        self.nav_reveal_px
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn last_request_id(self) -> u64 {
        self.launcher_request_id
    }

    pub const fn compatible_activity_reservation(self) -> Option<UiCompatibleActivityReservation> {
        self.compatible_activity_reservation
    }

    pub const fn has_compatible_activity_reservation(self) -> bool {
        self.compatible_activity_reservation.is_some()
    }
}

const EVENT_OFFSET_MAGIC: usize = 0;
const EVENT_OFFSET_VERSION: usize = 4;
const EVENT_OFFSET_KIND: usize = 6;
const EVENT_OFFSET_HEADER_RESERVED: usize = 7;
const EVENT_OFFSET_SESSION_ID: usize = 8;
const EVENT_OFFSET_ARGUMENT_0: usize = 16;
const EVENT_OFFSET_ARGUMENT_1: usize = 24;
const EVENT_OFFSET_ARGUMENT_2: usize = 32;
const EVENT_OFFSET_ARGUMENT_3: usize = 40;
const EVENT_OFFSET_BODY_RESERVED: usize = 48;
const SYSTEM_UI_COMPLETION_ACTION_MASK: u8 = 0x0f;
const SYSTEM_UI_COMPLETION_STATUS_SHIFT: u32 = 4;
const SYSTEM_UI_COMPLETION_STATUS_MASK: u8 = 0x03 << SYSTEM_UI_COMPLETION_STATUS_SHIFT;
const SYSTEM_UI_COMPLETION_ORIGIN_SHIFT: u32 = 6;
const SYSTEM_UI_COMPLETION_ORIGIN_MASK: u8 = 0x03 << SYSTEM_UI_COMPLETION_ORIGIN_SHIFT;

/// Stable server-to-client event discriminators carried by the UI IPC
/// channel. Zero remains invalid so an all-zero channel payload cannot be
/// interpreted as an event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiServerEventKind {
    Ready = 1,
    Input = 2,
    Presented = 3,
    Degraded = 4,
    FocusChanged = 5,
    PresentCancelled = 6,
    AppearanceChanged = 7,
    ClockChanged = 8,
    BootNotificationChanged = 9,
    SystemUiChanged = 10,
    SystemUiRequestCompleted = 11,
}

impl UiServerEventKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Ready),
            2 => Some(Self::Input),
            3 => Some(Self::Presented),
            4 => Some(Self::Degraded),
            5 => Some(Self::FocusChanged),
            6 => Some(Self::PresentCancelled),
            7 => Some(Self::AppearanceChanged),
            8 => Some(Self::ClockChanged),
            9 => Some(Self::BootNotificationChanged),
            10 => Some(Self::SystemUiChanged),
            11 => Some(Self::SystemUiRequestCompleted),
            _ => None,
        }
    }
}

/// Canonical outcome of one SurfaceServer-classified system-UI request.
///
/// `Conflict` and `CaptureBusy` are explicit acknowledgements constructed by
/// SurfaceServer. Input-capture state deliberately does not belong to
/// `UiSystemUiSession`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSystemUiRequestStatus {
    Accepted = 1,
    Conflict = 2,
    CaptureBusy = 3,
}

impl UiSystemUiRequestStatus {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Accepted),
            2 => Some(Self::Conflict),
            3 => Some(Self::CaptureBusy),
            _ => None,
        }
    }
}

/// Read-only fields carried by `SystemUiRequestCompleted`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSystemUiRequestCompletion {
    action: UiSystemUiAction,
    status: UiSystemUiRequestStatus,
    request_id: u64,
    revision: u64,
    compatible_identity: Option<UiCompatibleActivityIdentity>,
    reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
}

impl UiSystemUiRequestCompletion {
    pub const fn action(self) -> UiSystemUiAction {
        self.action
    }

    pub const fn status(self) -> UiSystemUiRequestStatus {
        self.status
    }

    pub const fn request_id(self) -> u64 {
        self.request_id
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn compatible_identity(self) -> Option<UiCompatibleActivityIdentity> {
        self.compatible_identity
    }

    pub const fn reservation_origin(self) -> Option<UiCompatibleActivityReservationOrigin> {
        self.reservation_origin
    }
}

/// Stable terminal reasons. Values are deliberately protocol-level rather
/// than kernel-specific so the Launcher can make a deterministic recovery
/// decision without interpreting private server status codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiServerDegradedReason {
    PeerClosed = 1,
    ServerExit = 2,
    KernelDegraded = 3,
    ProtocolViolation = 4,
}

impl UiServerDegradedReason {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::PeerClosed),
            2 => Some(Self::ServerExit),
            3 => Some(Self::KernelDegraded),
            4 => Some(Self::ProtocolViolation),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiServerEventError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSessionId,
    UnexpectedPayload,
    InvalidInput(InputSampleError),
    ZeroFrameId,
    FrameIdOutOfRange,
    ZeroCommit,
    InvalidDegradedReason,
    InvalidClient,
    InvalidApp,
    LauncherMustNotSpecifyApp,
    AppRequiresShellApp,
    ZeroFocusGeneration,
    InvalidAppearance(UiAppearanceError),
    ZeroAppearanceRevision,
    ZeroClockRevision,
    InvalidBootNotificationVisibility,
    ZeroBootNotificationRevision,
    InvalidSystemUiMode,
    InvalidRecentApp,
    InvalidRecentIdentity,
    InvalidCompatibleActivityIdentity,
    NonCanonicalSystemUiState,
    InvalidSystemUiState(UiSystemUiStateError),
    ZeroSystemUiRevision,
    InvalidSystemUiAction,
    InvalidSystemUiRequestStatus,
    InvalidCompatibleActivityReservationOrigin,
    ZeroSystemUiRequestId,
    ZeroSystemUiRequestRevision,
    NonCanonicalSystemUiRequestCompletion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiServerEventPayload {
    Ready,
    Input(InputSample),
    Presented {
        frame_id: u32,
        commit: u64,
    },
    Degraded {
        reason: UiServerDegradedReason,
    },
    FocusChanged {
        active_client: UiClientId,
        app: Option<ShellAppId>,
        focus_generation: u64,
    },
    PresentCancelled {
        frame_id: u32,
        focus_generation: u64,
    },
    AppearanceChanged {
        appearance: UiAppearance,
        revision: u64,
    },
    ClockChanged {
        unix_seconds: u64,
        revision: u64,
    },
    BootNotificationChanged {
        visible: bool,
        revision: u64,
    },
    SystemUiChanged {
        mode: UiSystemUiMode,
        recent: Option<UiRecentIdentity>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    },
    SystemUiRequestCompleted {
        action: UiSystemUiAction,
        status: UiSystemUiRequestStatus,
        request_id: u64,
        revision: u64,
        compatible_identity: Option<UiCompatibleActivityIdentity>,
        reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
    },
}

/// One canonical 64-byte SurfaceServer-to-client event.
///
/// The common header occupies bytes 0..16. Header byte 7 is zero for every
/// kind except `SystemUiRequestCompleted`, where it carries the closed
/// action/status/origin tuple. Bytes 16..48 are four kind-specific
/// little-endian arguments and bytes 48..64 are reserved zero. Constructors
/// keep the representation valid, while decoding additionally rejects every
/// cross-kind payload and nonzero reserved byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiServerEvent {
    session_id: u64,
    payload: UiServerEventPayload,
}

impl UiServerEvent {
    pub fn ready(session_id: u64) -> Result<Self, UiServerEventError> {
        Self::with_payload(session_id, UiServerEventPayload::Ready)
    }

    /// Builds an input event from the canonical state/sequence pair returned
    /// by `SurfaceReadInput`.
    pub fn input(
        session_id: u64,
        packed_state: u64,
        sequence: u64,
    ) -> Result<Self, UiServerEventError> {
        let sample = InputSample::decode_registers(packed_state, sequence)
            .map_err(UiServerEventError::InvalidInput)?;
        Self::input_sample(session_id, sample)
    }

    pub fn input_sample(session_id: u64, sample: InputSample) -> Result<Self, UiServerEventError> {
        Self::with_payload(session_id, UiServerEventPayload::Input(sample))
    }

    pub fn presented(
        session_id: u64,
        frame_id: u32,
        commit: u64,
    ) -> Result<Self, UiServerEventError> {
        if frame_id == 0 {
            return Err(UiServerEventError::ZeroFrameId);
        }
        if commit == 0 {
            return Err(UiServerEventError::ZeroCommit);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::Presented { frame_id, commit },
        )
    }

    pub fn degraded(
        session_id: u64,
        reason: UiServerDegradedReason,
    ) -> Result<Self, UiServerEventError> {
        Self::with_payload(session_id, UiServerEventPayload::Degraded { reason })
    }

    pub fn focus_changed(
        session_id: u64,
        active_client: UiClientId,
        app: Option<ShellAppId>,
        focus_generation: u64,
    ) -> Result<Self, UiServerEventError> {
        if focus_generation == 0 {
            return Err(UiServerEventError::ZeroFocusGeneration);
        }
        match (active_client, app) {
            (UiClientId::Launcher, Some(_)) => {
                return Err(UiServerEventError::LauncherMustNotSpecifyApp);
            }
            #[cfg(not(feature = "androidbox-el0-runtime0"))]
            (UiClientId::App, None) => return Err(UiServerEventError::AppRequiresShellApp),
            _ => {}
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::FocusChanged {
                active_client,
                app,
                focus_generation,
            },
        )
    }

    pub fn present_cancelled(
        session_id: u64,
        frame_id: u32,
        focus_generation: u64,
    ) -> Result<Self, UiServerEventError> {
        if frame_id == 0 {
            return Err(UiServerEventError::ZeroFrameId);
        }
        if focus_generation == 0 {
            return Err(UiServerEventError::ZeroFocusGeneration);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::PresentCancelled {
                frame_id,
                focus_generation,
            },
        )
    }

    pub fn appearance_changed(
        session_id: u64,
        appearance: UiAppearance,
        revision: u64,
    ) -> Result<Self, UiServerEventError> {
        if revision == 0 {
            return Err(UiServerEventError::ZeroAppearanceRevision);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::AppearanceChanged {
                appearance,
                revision,
            },
        )
    }

    /// Carries a SurfaceServer-owned wall-clock snapshot. Unix epoch zero is
    /// valid; only the session-local revision is required to be nonzero.
    pub fn clock_changed(
        session_id: u64,
        unix_seconds: u64,
        revision: u64,
    ) -> Result<Self, UiServerEventError> {
        if revision == 0 {
            return Err(UiServerEventError::ZeroClockRevision);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::ClockChanged {
                unix_seconds,
                revision,
            },
        )
    }

    /// Carries the complete state of the sole fixed boot-local notification.
    ///
    /// Visible is canonical boolean state rather than an open-ended count or
    /// notification identifier. Revision zero is reserved so clients can
    /// require an explicit initial revision-one snapshot.
    pub fn boot_notification_changed(
        session_id: u64,
        visible: bool,
        revision: u64,
    ) -> Result<Self, UiServerEventError> {
        if revision == 0 {
            return Err(UiServerEventError::ZeroBootNotificationRevision);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::BootNotificationChanged { visible, revision },
        )
    }

    /// Publishes one canonical SurfaceServer-owned system-UI snapshot.
    pub fn system_ui_changed(
        session_id: u64,
        mode: UiSystemUiMode,
        recent_app: Option<ShellAppId>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    ) -> Result<Self, UiServerEventError> {
        Self::system_ui_changed_with_recent(
            session_id,
            mode,
            recent_app.map(UiRecentIdentity::Shell),
            nav_pressed,
            nav_reveal_px,
            revision,
        )
    }

    pub fn system_ui_changed_with_recent(
        session_id: u64,
        mode: UiSystemUiMode,
        recent: Option<UiRecentIdentity>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    ) -> Result<Self, UiServerEventError> {
        validate_system_ui_state(mode, recent, nav_pressed, nav_reveal_px)
            .map_err(UiServerEventError::InvalidSystemUiState)?;
        if revision == 0 {
            return Err(UiServerEventError::ZeroSystemUiRevision);
        }
        Self::with_payload(
            session_id,
            UiServerEventPayload::SystemUiChanged {
                mode,
                recent,
                nav_pressed,
                nav_reveal_px,
                revision,
            },
        )
    }

    pub fn system_ui_update(
        session_id: u64,
        update: UiSystemUiUpdate,
    ) -> Result<Self, UiServerEventError> {
        Self::system_ui_changed_with_recent(
            session_id,
            update.mode(),
            update.recent(),
            update.nav_pressed(),
            update.nav_reveal_px(),
            update.revision(),
        )
    }

    /// Acknowledges one system-UI request without implying that the state
    /// changed. In particular, `Conflict` and `CaptureBusy` are valid
    /// completion events with the current nonzero revision.
    pub fn system_ui_request_completed(
        session_id: u64,
        action: UiSystemUiAction,
        status: UiSystemUiRequestStatus,
        request_id: u64,
        revision: u64,
        compatible_identity: Option<UiCompatibleActivityIdentity>,
        reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
    ) -> Result<Self, UiServerEventError> {
        validate_system_ui_request_completion(
            action,
            status,
            request_id,
            revision,
            compatible_identity,
            reservation_origin,
        )?;
        Self::with_payload(
            session_id,
            UiServerEventPayload::SystemUiRequestCompleted {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            },
        )
    }

    fn with_payload(
        session_id: u64,
        payload: UiServerEventPayload,
    ) -> Result<Self, UiServerEventError> {
        if session_id == 0 {
            return Err(UiServerEventError::ZeroSessionId);
        }
        Ok(Self {
            session_id,
            payload,
        })
    }

    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    pub const fn kind(self) -> UiServerEventKind {
        match self.payload {
            UiServerEventPayload::Ready => UiServerEventKind::Ready,
            UiServerEventPayload::Input(_) => UiServerEventKind::Input,
            UiServerEventPayload::Presented { .. } => UiServerEventKind::Presented,
            UiServerEventPayload::Degraded { .. } => UiServerEventKind::Degraded,
            UiServerEventPayload::FocusChanged { .. } => UiServerEventKind::FocusChanged,
            UiServerEventPayload::PresentCancelled { .. } => UiServerEventKind::PresentCancelled,
            UiServerEventPayload::AppearanceChanged { .. } => UiServerEventKind::AppearanceChanged,
            UiServerEventPayload::ClockChanged { .. } => UiServerEventKind::ClockChanged,
            UiServerEventPayload::BootNotificationChanged { .. } => {
                UiServerEventKind::BootNotificationChanged
            }
            UiServerEventPayload::SystemUiChanged { .. } => UiServerEventKind::SystemUiChanged,
            UiServerEventPayload::SystemUiRequestCompleted { .. } => {
                UiServerEventKind::SystemUiRequestCompleted
            }
        }
    }

    pub const fn payload(self) -> UiServerEventPayload {
        self.payload
    }

    pub const fn system_ui_request_completion(self) -> Option<UiSystemUiRequestCompletion> {
        match self.payload {
            UiServerEventPayload::SystemUiRequestCompleted {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            } => Some(UiSystemUiRequestCompletion {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            }),
            _ => None,
        }
    }

    pub fn encode(self) -> [u8; UI_SERVER_EVENT_WIRE_SIZE] {
        let mut wire = [0; UI_SERVER_EVENT_WIRE_SIZE];
        wire[EVENT_OFFSET_MAGIC..EVENT_OFFSET_MAGIC + 4]
            .copy_from_slice(&UI_SERVER_EVENT_MAGIC.to_le_bytes());
        wire[EVENT_OFFSET_VERSION..EVENT_OFFSET_VERSION + 2]
            .copy_from_slice(&UI_SERVER_EVENT_VERSION.to_le_bytes());
        wire[EVENT_OFFSET_KIND] = self.kind().raw();
        write_event_u64(&mut wire, EVENT_OFFSET_SESSION_ID, self.session_id);
        match self.payload {
            UiServerEventPayload::Ready => {}
            UiServerEventPayload::Input(sample) => {
                let (packed_state, sequence) = sample.encode_registers();
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, packed_state);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, sequence);
            }
            UiServerEventPayload::Presented { frame_id, commit } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, u64::from(frame_id));
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, commit);
            }
            UiServerEventPayload::Degraded { reason } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, u64::from(reason.raw()));
            }
            UiServerEventPayload::FocusChanged {
                active_client,
                app,
                focus_generation,
            } => {
                write_event_u64(
                    &mut wire,
                    EVENT_OFFSET_ARGUMENT_0,
                    u64::from(active_client.raw()),
                );
                write_event_u64(
                    &mut wire,
                    EVENT_OFFSET_ARGUMENT_1,
                    app.map_or(0, |app| u64::from(app.raw())),
                );
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_2, focus_generation);
            }
            UiServerEventPayload::PresentCancelled {
                frame_id,
                focus_generation,
            } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, u64::from(frame_id));
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, focus_generation);
            }
            UiServerEventPayload::AppearanceChanged {
                appearance,
                revision,
            } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, appearance.flags());
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, revision);
            }
            UiServerEventPayload::ClockChanged {
                unix_seconds,
                revision,
            } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, unix_seconds);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, revision);
            }
            UiServerEventPayload::BootNotificationChanged { visible, revision } => {
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, u64::from(visible));
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, revision);
            }
            UiServerEventPayload::SystemUiChanged {
                mode,
                recent,
                nav_pressed,
                nav_reveal_px,
                revision,
            } => {
                let (packed, compatible_session_id, compatible_package_generation) =
                    pack_system_ui_state(mode, recent, nav_pressed, nav_reveal_px);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, packed);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, revision);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_2, compatible_session_id);
                write_event_u64(
                    &mut wire,
                    EVENT_OFFSET_ARGUMENT_3,
                    compatible_package_generation,
                );
            }
            UiServerEventPayload::SystemUiRequestCompleted {
                action,
                status,
                request_id,
                revision,
                compatible_identity,
                reservation_origin,
            } => {
                wire[EVENT_OFFSET_HEADER_RESERVED] = action.raw()
                    | (status.raw() << SYSTEM_UI_COMPLETION_STATUS_SHIFT)
                    | (reservation_origin.map_or(0, |origin| origin.raw())
                        << SYSTEM_UI_COMPLETION_ORIGIN_SHIFT);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_0, request_id);
                write_event_u64(&mut wire, EVENT_OFFSET_ARGUMENT_1, revision);
                write_event_u64(
                    &mut wire,
                    EVENT_OFFSET_ARGUMENT_2,
                    compatible_identity.map_or(0, UiCompatibleActivityIdentity::session_id),
                );
                write_event_u64(
                    &mut wire,
                    EVENT_OFFSET_ARGUMENT_3,
                    compatible_identity.map_or(0, UiCompatibleActivityIdentity::package_generation),
                );
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, UiServerEventError> {
        if wire.len() != UI_SERVER_EVENT_WIRE_SIZE {
            return Err(UiServerEventError::InvalidWireLength);
        }
        if read_u32(wire, EVENT_OFFSET_MAGIC) != UI_SERVER_EVENT_MAGIC {
            return Err(UiServerEventError::InvalidMagic);
        }
        if read_u16(wire, EVENT_OFFSET_VERSION) != UI_SERVER_EVENT_VERSION {
            return Err(UiServerEventError::InvalidVersion);
        }
        let kind = UiServerEventKind::from_raw(wire[EVENT_OFFSET_KIND])
            .ok_or(UiServerEventError::InvalidKind)?;
        let header_detail = wire[EVENT_OFFSET_HEADER_RESERVED];
        if kind != UiServerEventKind::SystemUiRequestCompleted && header_detail != 0 {
            return Err(UiServerEventError::NonZeroHeaderReserved);
        }
        if wire[EVENT_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(UiServerEventError::NonZeroBodyReserved);
        }
        let session_id = read_u64(wire, EVENT_OFFSET_SESSION_ID);
        let argument_0 = read_u64(wire, EVENT_OFFSET_ARGUMENT_0);
        let argument_1 = read_u64(wire, EVENT_OFFSET_ARGUMENT_1);
        let argument_2 = read_u64(wire, EVENT_OFFSET_ARGUMENT_2);
        let argument_3 = read_u64(wire, EVENT_OFFSET_ARGUMENT_3);
        match kind {
            UiServerEventKind::Ready => {
                if argument_0 != 0 || argument_1 != 0 || argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                Self::ready(session_id)
            }
            UiServerEventKind::Input => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                Self::input(session_id, argument_0, argument_1)
            }
            UiServerEventKind::Presented => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let frame_id =
                    u32::try_from(argument_0).map_err(|_| UiServerEventError::FrameIdOutOfRange)?;
                Self::presented(session_id, frame_id, argument_1)
            }
            UiServerEventKind::Degraded => {
                if argument_1 != 0 || argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let reason = UiServerDegradedReason::from_raw(argument_0)
                    .ok_or(UiServerEventError::InvalidDegradedReason)?;
                Self::degraded(session_id, reason)
            }
            UiServerEventKind::FocusChanged => {
                if argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let active_client =
                    UiClientId::from_raw(argument_0).ok_or(UiServerEventError::InvalidClient)?;
                let app = if argument_1 == 0 {
                    None
                } else {
                    Some(ShellAppId::from_raw(argument_1).ok_or(UiServerEventError::InvalidApp)?)
                };
                Self::focus_changed(session_id, active_client, app, argument_2)
            }
            UiServerEventKind::PresentCancelled => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let frame_id =
                    u32::try_from(argument_0).map_err(|_| UiServerEventError::FrameIdOutOfRange)?;
                Self::present_cancelled(session_id, frame_id, argument_1)
            }
            UiServerEventKind::AppearanceChanged => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let appearance = UiAppearance::from_flags(argument_0)
                    .map_err(UiServerEventError::InvalidAppearance)?;
                Self::appearance_changed(session_id, appearance, argument_1)
            }
            UiServerEventKind::ClockChanged => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                Self::clock_changed(session_id, argument_0, argument_1)
            }
            UiServerEventKind::BootNotificationChanged => {
                if argument_2 != 0 || argument_3 != 0 {
                    return Err(UiServerEventError::UnexpectedPayload);
                }
                let visible = match argument_0 {
                    0 => false,
                    1 => true,
                    _ => return Err(UiServerEventError::InvalidBootNotificationVisibility),
                };
                Self::boot_notification_changed(session_id, visible, argument_1)
            }
            UiServerEventKind::SystemUiChanged => {
                let (mode, recent, nav_pressed, nav_reveal_px) =
                    unpack_system_ui_state(argument_0, argument_2, argument_3)?;
                Self::system_ui_changed_with_recent(
                    session_id,
                    mode,
                    recent,
                    nav_pressed,
                    nav_reveal_px,
                    argument_1,
                )
            }
            UiServerEventKind::SystemUiRequestCompleted => {
                let action = UiSystemUiAction::from_raw(u64::from(
                    header_detail & SYSTEM_UI_COMPLETION_ACTION_MASK,
                ))
                .ok_or(UiServerEventError::InvalidSystemUiAction)?;
                let status = UiSystemUiRequestStatus::from_raw(
                    (header_detail & SYSTEM_UI_COMPLETION_STATUS_MASK)
                        >> SYSTEM_UI_COMPLETION_STATUS_SHIFT,
                )
                .ok_or(UiServerEventError::InvalidSystemUiRequestStatus)?;
                let origin_raw = (header_detail & SYSTEM_UI_COMPLETION_ORIGIN_MASK)
                    >> SYSTEM_UI_COMPLETION_ORIGIN_SHIFT;
                let reservation_origin = if origin_raw == 0 {
                    None
                } else {
                    Some(
                        UiCompatibleActivityReservationOrigin::from_raw(origin_raw).ok_or(
                            UiServerEventError::InvalidCompatibleActivityReservationOrigin,
                        )?,
                    )
                };
                let compatible_identity = match (argument_2, argument_3) {
                    (0, 0) => None,
                    (0, _) | (_, 0) => {
                        return Err(UiServerEventError::NonCanonicalSystemUiRequestCompletion);
                    }
                    (session_id, package_generation) => Some(
                        UiCompatibleActivityIdentity::new(session_id, package_generation)
                            .ok_or(UiServerEventError::InvalidCompatibleActivityIdentity)?,
                    ),
                };
                Self::system_ui_request_completed(
                    session_id,
                    action,
                    status,
                    argument_0,
                    argument_1,
                    compatible_identity,
                    reservation_origin,
                )
            }
        }
    }
}

fn validate_system_ui_request_completion(
    action: UiSystemUiAction,
    _status: UiSystemUiRequestStatus,
    request_id: u64,
    revision: u64,
    compatible_identity: Option<UiCompatibleActivityIdentity>,
    reservation_origin: Option<UiCompatibleActivityReservationOrigin>,
) -> Result<(), UiServerEventError> {
    if request_id == 0 {
        return Err(UiServerEventError::ZeroSystemUiRequestId);
    }
    if revision == 0 {
        return Err(UiServerEventError::ZeroSystemUiRequestRevision);
    }
    let fields_are_canonical = match action {
        UiSystemUiAction::Unlock | UiSystemUiAction::CloseOverview => {
            compatible_identity.is_none() && reservation_origin.is_none()
        }
        UiSystemUiAction::ActivateRecent => {
            compatible_identity.is_some() == reservation_origin.is_some()
        }
        UiSystemUiAction::PresentCompatibleActivity
        | UiSystemUiAction::ReserveCompatibleActivity
        | UiSystemUiAction::AbortCompatibleActivityVerification => {
            compatible_identity.is_some() && reservation_origin.is_some()
        }
        UiSystemUiAction::HomeCompatibleActivity => {
            compatible_identity.is_some() && reservation_origin.is_none()
        }
        UiSystemUiAction::FinishCompatibleActivity => compatible_identity.is_some(),
    };
    if !fields_are_canonical {
        return Err(UiServerEventError::NonCanonicalSystemUiRequestCompletion);
    }
    Ok(())
}

fn write_event_u64(wire: &mut [u8; UI_SERVER_EVENT_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiServerTrackerError {
    EventBeforeReady,
    ReadyAlreadyObserved,
    SessionMismatch,
    Terminal,
    PresentAlreadyOutstanding,
    NoPresentOutstanding,
    FrameIdExhausted,
    FrameReplay,
    FrameGap,
    PresentedFrameReplay,
    PresentedFrameGap,
    CommitExhausted,
    CommitReplay,
    CommitGap,
    InputSequenceExhausted,
    InputReplay,
    InputGap,
    FocusGenerationExhausted,
    FocusReplay,
    FocusGap,
    AppearanceRevisionExhausted,
    AppearanceReplay,
    AppearanceGap,
    ClockRevisionExhausted,
    ClockReplay,
    ClockGap,
    BootNotificationRevisionExhausted,
    BootNotificationReplay,
    BootNotificationGap,
    SystemUiRevisionExhausted,
    SystemUiReplay,
    SystemUiGap,
    CompatibleAppFocusRequiresForeground,
    CancellationBeforeFocus,
}

/// Client-side ordering state for a single SurfaceServer session.
///
/// The tracker admits exactly one outstanding presentation, requires Ready
/// before any request or event, validates independent frame/commit/input
/// frame and input sequences without wrap, admits gaps in the global kernel
/// commit sequence, requires contiguous focus, appearance, and clock
/// generations, including boot-notification and system-UI revisions, and
/// treats Degraded as terminal. A generation-matched cancellation clears only
/// the outstanding request, allowing that client-local frame id to be retried.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiServerEventTracker {
    session_id: Option<u64>,
    last_input_sequence: Option<u64>,
    last_frame_id: Option<u32>,
    last_commit: Option<u64>,
    last_focus_generation: Option<u64>,
    last_appearance_revision: Option<u64>,
    appearance: Option<UiAppearance>,
    last_clock_revision: Option<u64>,
    clock_unix_seconds: Option<u64>,
    last_boot_notification_revision: Option<u64>,
    boot_notification_visible: Option<bool>,
    last_system_ui_revision: Option<u64>,
    system_ui_mode: Option<UiSystemUiMode>,
    recent: Option<UiRecentIdentity>,
    nav_pressed: Option<bool>,
    nav_reveal_px: Option<u16>,
    active_client: Option<UiClientId>,
    active_app: Option<ShellAppId>,
    outstanding_frame_id: Option<u32>,
    degraded_reason: Option<UiServerDegradedReason>,
}

impl UiServerEventTracker {
    pub const fn new() -> Self {
        Self {
            session_id: None,
            last_input_sequence: None,
            last_frame_id: None,
            last_commit: None,
            last_focus_generation: None,
            last_appearance_revision: None,
            appearance: None,
            last_clock_revision: None,
            clock_unix_seconds: None,
            last_boot_notification_revision: None,
            boot_notification_visible: None,
            last_system_ui_revision: None,
            system_ui_mode: None,
            recent: None,
            nav_pressed: None,
            nav_reveal_px: None,
            active_client: None,
            active_app: None,
            outstanding_frame_id: None,
            degraded_reason: None,
        }
    }

    /// Records a client request before it is sent to SurfaceServer.
    pub fn begin_present(&mut self, frame_id: u32) -> Result<(), UiServerTrackerError> {
        if self.session_id.is_none() {
            return Err(UiServerTrackerError::EventBeforeReady);
        }
        if self.degraded_reason.is_some() {
            return Err(UiServerTrackerError::Terminal);
        }
        if self.outstanding_frame_id.is_some() {
            return Err(UiServerTrackerError::PresentAlreadyOutstanding);
        }
        let expected = next_frame_id(self.last_frame_id)?;
        if frame_id < expected {
            return Err(UiServerTrackerError::FrameReplay);
        }
        if frame_id > expected {
            return Err(UiServerTrackerError::FrameGap);
        }
        self.outstanding_frame_id = Some(frame_id);
        Ok(())
    }

    /// Accepts one decoded server event transactionally.
    pub fn accept(&mut self, event: UiServerEvent) -> Result<(), UiServerTrackerError> {
        if self.degraded_reason.is_some() {
            return Err(UiServerTrackerError::Terminal);
        }
        if event.kind() == UiServerEventKind::Ready {
            if self.session_id.is_some() {
                return Err(UiServerTrackerError::ReadyAlreadyObserved);
            }
            self.session_id = Some(event.session_id());
            return Ok(());
        }
        let session_id = self
            .session_id
            .ok_or(UiServerTrackerError::EventBeforeReady)?;
        if event.session_id() != session_id {
            return Err(UiServerTrackerError::SessionMismatch);
        }
        match event.payload() {
            UiServerEventPayload::Ready => unreachable!(),
            UiServerEventPayload::Input(sample) => {
                let expected = next_input_sequence(self.last_input_sequence)?;
                if sample.sequence() < expected {
                    return Err(UiServerTrackerError::InputReplay);
                }
                if sample.sequence() > expected {
                    return Err(UiServerTrackerError::InputGap);
                }
                self.last_input_sequence = Some(sample.sequence());
            }
            UiServerEventPayload::Presented { frame_id, commit } => {
                let expected_frame = next_frame_id(self.last_frame_id)?;
                let Some(outstanding) = self.outstanding_frame_id else {
                    if frame_id < expected_frame {
                        return Err(UiServerTrackerError::PresentedFrameReplay);
                    }
                    if frame_id > expected_frame {
                        return Err(UiServerTrackerError::PresentedFrameGap);
                    }
                    return Err(UiServerTrackerError::NoPresentOutstanding);
                };
                if frame_id < outstanding {
                    return Err(UiServerTrackerError::PresentedFrameReplay);
                }
                if frame_id > outstanding {
                    return Err(UiServerTrackerError::PresentedFrameGap);
                }
                if let Some(previous) = self.last_commit {
                    if previous == u64::MAX {
                        return Err(UiServerTrackerError::CommitExhausted);
                    }
                    if commit <= previous {
                        return Err(UiServerTrackerError::CommitReplay);
                    }
                }
                self.last_frame_id = Some(frame_id);
                self.last_commit = Some(commit);
                self.outstanding_frame_id = None;
            }
            UiServerEventPayload::Degraded { reason } => {
                self.outstanding_frame_id = None;
                self.degraded_reason = Some(reason);
            }
            UiServerEventPayload::FocusChanged {
                active_client,
                app,
                focus_generation,
            } => {
                let expected = next_focus_generation(self.last_focus_generation)?;
                if focus_generation < expected {
                    return Err(UiServerTrackerError::FocusReplay);
                }
                if focus_generation > expected {
                    return Err(UiServerTrackerError::FocusGap);
                }
                #[cfg(feature = "androidbox-el0-runtime0")]
                if active_client == UiClientId::App
                    && app.is_none()
                    && (self.system_ui_mode != Some(UiSystemUiMode::Foreground)
                        || !matches!(self.recent, Some(UiRecentIdentity::CompatibleAndroid(_)))
                        || self.nav_pressed != Some(false)
                        || self.nav_reveal_px != Some(0))
                {
                    return Err(UiServerTrackerError::CompatibleAppFocusRequiresForeground);
                }
                self.last_focus_generation = Some(focus_generation);
                self.active_client = Some(active_client);
                self.active_app = app;
            }
            UiServerEventPayload::PresentCancelled {
                frame_id,
                focus_generation,
            } => {
                let Some(outstanding) = self.outstanding_frame_id else {
                    return Err(UiServerTrackerError::NoPresentOutstanding);
                };
                if frame_id < outstanding {
                    return Err(UiServerTrackerError::PresentedFrameReplay);
                }
                if frame_id > outstanding {
                    return Err(UiServerTrackerError::PresentedFrameGap);
                }
                let Some(current_focus_generation) = self.last_focus_generation else {
                    return Err(UiServerTrackerError::CancellationBeforeFocus);
                };
                if focus_generation < current_focus_generation {
                    return Err(UiServerTrackerError::FocusReplay);
                }
                if focus_generation > current_focus_generation {
                    return Err(UiServerTrackerError::FocusGap);
                }
                self.outstanding_frame_id = None;
            }
            UiServerEventPayload::AppearanceChanged {
                appearance,
                revision,
            } => {
                let expected = next_appearance_revision(self.last_appearance_revision)?;
                if revision < expected {
                    return Err(UiServerTrackerError::AppearanceReplay);
                }
                if revision > expected {
                    return Err(UiServerTrackerError::AppearanceGap);
                }
                self.last_appearance_revision = Some(revision);
                self.appearance = Some(appearance);
            }
            UiServerEventPayload::ClockChanged {
                unix_seconds,
                revision,
            } => {
                let expected = next_clock_revision(self.last_clock_revision)?;
                if revision < expected {
                    return Err(UiServerTrackerError::ClockReplay);
                }
                if revision > expected {
                    return Err(UiServerTrackerError::ClockGap);
                }
                self.last_clock_revision = Some(revision);
                self.clock_unix_seconds = Some(unix_seconds);
            }
            UiServerEventPayload::BootNotificationChanged { visible, revision } => {
                let expected =
                    next_boot_notification_revision(self.last_boot_notification_revision)?;
                if revision < expected {
                    return Err(UiServerTrackerError::BootNotificationReplay);
                }
                if revision > expected {
                    return Err(UiServerTrackerError::BootNotificationGap);
                }
                self.last_boot_notification_revision = Some(revision);
                self.boot_notification_visible = Some(visible);
            }
            UiServerEventPayload::SystemUiChanged {
                mode,
                recent,
                nav_pressed,
                nav_reveal_px,
                revision,
            } => {
                let expected = next_system_ui_revision(self.last_system_ui_revision)?;
                if revision < expected {
                    return Err(UiServerTrackerError::SystemUiReplay);
                }
                if revision > expected {
                    return Err(UiServerTrackerError::SystemUiGap);
                }
                self.last_system_ui_revision = Some(revision);
                self.system_ui_mode = Some(mode);
                self.recent = recent;
                self.nav_pressed = Some(nav_pressed);
                self.nav_reveal_px = Some(nav_reveal_px);
            }
            UiServerEventPayload::SystemUiRequestCompleted { .. } => {}
        }
        Ok(())
    }

    pub const fn session_id(self) -> Option<u64> {
        self.session_id
    }

    pub const fn last_input_sequence(self) -> Option<u64> {
        self.last_input_sequence
    }

    pub const fn last_frame_id(self) -> Option<u32> {
        self.last_frame_id
    }

    pub const fn last_commit(self) -> Option<u64> {
        self.last_commit
    }

    pub const fn last_focus_generation(self) -> Option<u64> {
        self.last_focus_generation
    }

    pub const fn last_appearance_revision(self) -> Option<u64> {
        self.last_appearance_revision
    }

    pub const fn appearance(self) -> Option<UiAppearance> {
        self.appearance
    }

    pub const fn last_clock_revision(self) -> Option<u64> {
        self.last_clock_revision
    }

    pub const fn clock_unix_seconds(self) -> Option<u64> {
        self.clock_unix_seconds
    }

    pub const fn last_boot_notification_revision(self) -> Option<u64> {
        self.last_boot_notification_revision
    }

    pub const fn boot_notification_visible(self) -> Option<bool> {
        self.boot_notification_visible
    }

    pub const fn last_system_ui_revision(self) -> Option<u64> {
        self.last_system_ui_revision
    }

    pub const fn system_ui_mode(self) -> Option<UiSystemUiMode> {
        self.system_ui_mode
    }

    pub const fn recent(self) -> Option<UiRecentIdentity> {
        self.recent
    }

    pub const fn recent_app(self) -> Option<ShellAppId> {
        match self.recent {
            Some(recent) => recent.shell_app(),
            None => None,
        }
    }

    pub const fn nav_pressed(self) -> Option<bool> {
        self.nav_pressed
    }

    pub const fn nav_reveal_px(self) -> Option<u16> {
        self.nav_reveal_px
    }

    pub const fn active_client(self) -> Option<UiClientId> {
        self.active_client
    }

    pub const fn active_app(self) -> Option<ShellAppId> {
        self.active_app
    }

    pub const fn outstanding_frame_id(self) -> Option<u32> {
        self.outstanding_frame_id
    }

    pub const fn degraded_reason(self) -> Option<UiServerDegradedReason> {
        self.degraded_reason
    }
}

fn next_frame_id(previous: Option<u32>) -> Result<u32, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::FrameIdExhausted),
        None => Ok(1),
    }
}

fn next_input_sequence(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::InputSequenceExhausted),
        None => Ok(1),
    }
}

fn next_focus_generation(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::FocusGenerationExhausted),
        None => Ok(1),
    }
}

fn next_appearance_revision(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::AppearanceRevisionExhausted),
        None => Ok(1),
    }
}

fn next_clock_revision(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::ClockRevisionExhausted),
        None => Ok(1),
    }
}

fn next_boot_notification_revision(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::BootNotificationRevisionExhausted),
        None => Ok(1),
    }
}

fn next_system_ui_revision(previous: Option<u64>) -> Result<u64, UiServerTrackerError> {
    match previous {
        Some(previous) => previous
            .checked_add(1)
            .ok_or(UiServerTrackerError::SystemUiRevisionExhausted),
        None => Ok(1),
    }
}

const BUFFER_PRESENT_OFFSET_MAGIC: usize = 0;
const BUFFER_PRESENT_OFFSET_VERSION: usize = 4;
const BUFFER_PRESENT_OFFSET_KIND: usize = 6;
const BUFFER_PRESENT_OFFSET_MODE: usize = 7;
const BUFFER_PRESENT_OFFSET_CLIENT_FRAME_ID: usize = 8;
const BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID: usize = 12;
const BUFFER_PRESENT_OFFSET_FOCUS_GENERATION: usize = 16;
const BUFFER_PRESENT_OFFSET_HEADER_RESERVED: usize = 20;
const BUFFER_PRESENT_HEADER_RESERVED_END: usize = 24;
const BUFFER_PRESENT_OFFSET_BUFFER_GENERATION: usize = 24;
const BUFFER_PRESENT_OFFSET_X: usize = 32;
const BUFFER_PRESENT_OFFSET_Y: usize = 34;
const BUFFER_PRESENT_OFFSET_WIDTH: usize = 36;
const BUFFER_PRESENT_OFFSET_HEIGHT: usize = 38;
const BUFFER_PRESENT_OFFSET_SYSTEM_UI_REVISION: usize = 40;
#[cfg(feature = "mobile-system-chrome0")]
const BUFFER_PRESENT_OFFSET_SYSTEM_CHROME_GENERATION: usize = 48;
#[cfg(not(feature = "mobile-system-chrome0"))]
const BUFFER_PRESENT_OFFSET_PADDING: usize = 48;
#[cfg(feature = "mobile-system-chrome0")]
const BUFFER_PRESENT_OFFSET_PADDING: usize = 56;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferPresentError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    InvalidMode,
    ZeroClientFrameId,
    ZeroGlobalFrameId,
    GlobalFramePrecedesClientFrame,
    ZeroFocusGeneration,
    ZeroBufferGeneration,
    ZeroSystemUiRevision,
    ZeroSystemChromeGeneration,
    NonZeroHeaderReserved,
    InvalidGeometry,
    NonZeroPadding,
    ClientFrameIdMismatch,
}

/// One canonical M32a buffer-backed full-surface present transaction.
///
/// A client initially sets both frame identifiers to its local identifier.
/// SurfaceServer retains `client_frame_id` for acknowledgements and replaces
/// `global_frame_id` with the monotonically increasing compositor identifier.
/// M32a deliberately supports only one configured geometry: the full shell
/// surface in protocol v2, or the canonical 720x1448 application-content
/// viewport when the opt-in protocol v3 trusted-chrome split is enabled.
/// Protocol v2 and v3 carry an optional System UI revision. Revision zero is
/// retained only for callers outside the mobile System UI contract; mobile
/// callers use the explicit nonzero-revision constructors. Introducing damage
/// or any additional geometry requires another protocol version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferPresent {
    client_frame_id: u32,
    global_frame_id: u32,
    focus_generation: u32,
    buffer_generation: u64,
    system_ui_revision: u64,
    system_chrome_generation: u64,
}

impl BufferPresent {
    /// Constructs the client form, before SurfaceServer allocates a global
    /// frame identifier.
    pub fn client(
        frame_id: u32,
        focus_generation: u32,
        buffer_generation: u64,
    ) -> Result<Self, BufferPresentError> {
        Self::try_new(frame_id, frame_id, focus_generation, buffer_generation)
    }

    /// Constructs a client present bound to one observed nonzero System UI
    /// revision.
    pub fn client_with_system_ui_revision(
        frame_id: u32,
        focus_generation: u32,
        buffer_generation: u64,
        system_ui_revision: u64,
    ) -> Result<Self, BufferPresentError> {
        Self::try_new_with_system_ui_revision(
            frame_id,
            frame_id,
            focus_generation,
            buffer_generation,
            system_ui_revision,
        )
    }

    /// Constructs the compatibility form with no System UI revision binding.
    ///
    /// `system_ui_revision()` is always zero for values returned here.
    pub fn try_new(
        client_frame_id: u32,
        global_frame_id: u32,
        focus_generation: u32,
        buffer_generation: u64,
    ) -> Result<Self, BufferPresentError> {
        Self::try_new_allowing_zero_system_ui_revision(
            client_frame_id,
            global_frame_id,
            focus_generation,
            buffer_generation,
            0,
            0,
        )
    }

    /// Constructs a present bound to one observed nonzero System UI revision.
    pub fn try_new_with_system_ui_revision(
        client_frame_id: u32,
        global_frame_id: u32,
        focus_generation: u32,
        buffer_generation: u64,
        system_ui_revision: u64,
    ) -> Result<Self, BufferPresentError> {
        if system_ui_revision == 0 {
            return Err(BufferPresentError::ZeroSystemUiRevision);
        }
        Self::try_new_allowing_zero_system_ui_revision(
            client_frame_id,
            global_frame_id,
            focus_generation,
            buffer_generation,
            system_ui_revision,
            0,
        )
    }

    fn try_new_allowing_zero_system_ui_revision(
        client_frame_id: u32,
        global_frame_id: u32,
        focus_generation: u32,
        buffer_generation: u64,
        system_ui_revision: u64,
        system_chrome_generation: u64,
    ) -> Result<Self, BufferPresentError> {
        validate_buffer_present_fields(
            client_frame_id,
            global_frame_id,
            focus_generation,
            buffer_generation,
        )?;
        Ok(Self {
            client_frame_id,
            global_frame_id,
            focus_generation,
            buffer_generation,
            system_ui_revision,
            system_chrome_generation,
        })
    }

    pub const fn client_frame_id(&self) -> u32 {
        self.client_frame_id
    }

    /// Returns the compositor/kernel-facing global frame identifier.
    pub const fn frame_id(&self) -> u32 {
        self.global_frame_id
    }

    pub const fn global_frame_id(&self) -> u32 {
        self.global_frame_id
    }

    pub const fn focus_generation(&self) -> u32 {
        self.focus_generation
    }

    pub const fn buffer_generation(&self) -> u64 {
        self.buffer_generation
    }

    /// Returns the System UI revision observed when this frame was produced.
    ///
    /// Zero denotes the compatibility form constructed by `client` or
    /// `try_new`; revision-bound mobile presents are always nonzero.
    pub const fn system_ui_revision(&self) -> u64 {
        self.system_ui_revision
    }

    /// Returns the independently generated trusted-chrome buffer generation.
    ///
    /// Protocol v2 always returns zero. Under protocol v3 constructors and the
    /// wire decoder may produce a provisional zero client form; the syscall 62
    /// submission boundary must reject it until a nonzero generation is bound.
    pub const fn system_chrome_generation(&self) -> u64 {
        self.system_chrome_generation
    }

    /// Binds the independently generated trusted-chrome buffer to this v3
    /// content present. Zero is valid only for the provisional client form and
    /// is never a valid syscall 62 submission generation.
    #[cfg(feature = "mobile-system-chrome0")]
    pub fn with_system_chrome_generation(
        mut self,
        system_chrome_generation: u64,
    ) -> Result<Self, BufferPresentError> {
        if system_chrome_generation == 0 {
            return Err(BufferPresentError::ZeroSystemChromeGeneration);
        }
        self.system_chrome_generation = system_chrome_generation;
        Ok(self)
    }

    /// Rewrites only the global identifier while requiring the caller to
    /// prove which client-local transaction is being mapped.
    pub fn with_frame_id(
        mut self,
        client_frame_id: u32,
        global_frame_id: u32,
    ) -> Result<Self, BufferPresentError> {
        if client_frame_id != self.client_frame_id {
            return Err(BufferPresentError::ClientFrameIdMismatch);
        }
        validate_buffer_present_fields(
            client_frame_id,
            global_frame_id,
            self.focus_generation,
            self.buffer_generation,
        )?;
        self.global_frame_id = global_frame_id;
        Ok(self)
    }

    pub fn with_focus_generation(
        mut self,
        focus_generation: u32,
    ) -> Result<Self, BufferPresentError> {
        validate_buffer_present_fields(
            self.client_frame_id,
            self.global_frame_id,
            focus_generation,
            self.buffer_generation,
        )?;
        self.focus_generation = focus_generation;
        Ok(self)
    }

    pub fn encode(&self) -> [u8; BUFFER_PRESENT_WIRE_SIZE] {
        let mut wire = [0_u8; BUFFER_PRESENT_WIRE_SIZE];
        wire[BUFFER_PRESENT_OFFSET_MAGIC..BUFFER_PRESENT_OFFSET_MAGIC + 4]
            .copy_from_slice(&BUFFER_PRESENT_MAGIC.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_VERSION..BUFFER_PRESENT_OFFSET_VERSION + 2]
            .copy_from_slice(&BUFFER_PRESENT_VERSION.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_KIND] = BUFFER_PRESENT_KIND;
        wire[BUFFER_PRESENT_OFFSET_MODE] = BUFFER_PRESENT_MODE_FULL;
        wire[BUFFER_PRESENT_OFFSET_CLIENT_FRAME_ID..BUFFER_PRESENT_OFFSET_CLIENT_FRAME_ID + 4]
            .copy_from_slice(&self.client_frame_id.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID..BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID + 4]
            .copy_from_slice(&self.global_frame_id.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_FOCUS_GENERATION..BUFFER_PRESENT_OFFSET_FOCUS_GENERATION + 4]
            .copy_from_slice(&self.focus_generation.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_BUFFER_GENERATION..BUFFER_PRESENT_OFFSET_BUFFER_GENERATION + 8]
            .copy_from_slice(&self.buffer_generation.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_X..BUFFER_PRESENT_OFFSET_X + 2]
            .copy_from_slice(&BUFFER_PRESENT_X.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_Y..BUFFER_PRESENT_OFFSET_Y + 2]
            .copy_from_slice(&BUFFER_PRESENT_Y.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_WIDTH..BUFFER_PRESENT_OFFSET_WIDTH + 2]
            .copy_from_slice(&BUFFER_PRESENT_WIDTH.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_HEIGHT..BUFFER_PRESENT_OFFSET_HEIGHT + 2]
            .copy_from_slice(&BUFFER_PRESENT_HEIGHT.to_le_bytes());
        wire[BUFFER_PRESENT_OFFSET_SYSTEM_UI_REVISION
            ..BUFFER_PRESENT_OFFSET_SYSTEM_UI_REVISION + 8]
            .copy_from_slice(&self.system_ui_revision.to_le_bytes());
        #[cfg(feature = "mobile-system-chrome0")]
        wire[BUFFER_PRESENT_OFFSET_SYSTEM_CHROME_GENERATION
            ..BUFFER_PRESENT_OFFSET_SYSTEM_CHROME_GENERATION + 8]
            .copy_from_slice(&self.system_chrome_generation.to_le_bytes());
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, BufferPresentError> {
        if wire.len() != BUFFER_PRESENT_WIRE_SIZE {
            return Err(BufferPresentError::InvalidWireLength);
        }
        if read_u32(wire, BUFFER_PRESENT_OFFSET_MAGIC) != BUFFER_PRESENT_MAGIC {
            return Err(BufferPresentError::InvalidMagic);
        }
        if read_u16(wire, BUFFER_PRESENT_OFFSET_VERSION) != BUFFER_PRESENT_VERSION {
            return Err(BufferPresentError::InvalidVersion);
        }
        if wire[BUFFER_PRESENT_OFFSET_KIND] != BUFFER_PRESENT_KIND {
            return Err(BufferPresentError::InvalidKind);
        }
        if wire[BUFFER_PRESENT_OFFSET_MODE] != BUFFER_PRESENT_MODE_FULL {
            return Err(BufferPresentError::InvalidMode);
        }
        if wire[BUFFER_PRESENT_OFFSET_HEADER_RESERVED..BUFFER_PRESENT_HEADER_RESERVED_END]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(BufferPresentError::NonZeroHeaderReserved);
        }
        if read_u16(wire, BUFFER_PRESENT_OFFSET_X) != BUFFER_PRESENT_X
            || read_u16(wire, BUFFER_PRESENT_OFFSET_Y) != BUFFER_PRESENT_Y
            || read_u16(wire, BUFFER_PRESENT_OFFSET_WIDTH) != BUFFER_PRESENT_WIDTH
            || read_u16(wire, BUFFER_PRESENT_OFFSET_HEIGHT) != BUFFER_PRESENT_HEIGHT
        {
            return Err(BufferPresentError::InvalidGeometry);
        }
        if wire[BUFFER_PRESENT_OFFSET_PADDING..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(BufferPresentError::NonZeroPadding);
        }
        #[cfg(feature = "mobile-system-chrome0")]
        let system_chrome_generation =
            read_u64(wire, BUFFER_PRESENT_OFFSET_SYSTEM_CHROME_GENERATION);
        #[cfg(not(feature = "mobile-system-chrome0"))]
        let system_chrome_generation = 0;
        Self::try_new_allowing_zero_system_ui_revision(
            read_u32(wire, BUFFER_PRESENT_OFFSET_CLIENT_FRAME_ID),
            read_u32(wire, BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID),
            read_u32(wire, BUFFER_PRESENT_OFFSET_FOCUS_GENERATION),
            read_u64(wire, BUFFER_PRESENT_OFFSET_BUFFER_GENERATION),
            read_u64(wire, BUFFER_PRESENT_OFFSET_SYSTEM_UI_REVISION),
            system_chrome_generation,
        )
    }
}

fn validate_buffer_present_fields(
    client_frame_id: u32,
    global_frame_id: u32,
    focus_generation: u32,
    buffer_generation: u64,
) -> Result<(), BufferPresentError> {
    if client_frame_id == 0 {
        return Err(BufferPresentError::ZeroClientFrameId);
    }
    if global_frame_id == 0 {
        return Err(BufferPresentError::ZeroGlobalFrameId);
    }
    if global_frame_id < client_frame_id {
        return Err(BufferPresentError::GlobalFramePrecedesClientFrame);
    }
    if focus_generation == 0 {
        return Err(BufferPresentError::ZeroFocusGeneration);
    }
    if buffer_generation == 0 {
        return Err(BufferPresentError::ZeroBufferGeneration);
    }
    Ok(())
}

const WINDOW_COMMAND_OFFSET_MAGIC: usize = 0;
const WINDOW_COMMAND_OFFSET_VERSION: usize = 4;
const WINDOW_COMMAND_OFFSET_OPCODE: usize = 6;
const WINDOW_COMMAND_OFFSET_HEADER_RESERVED: usize = 7;
const WINDOW_COMMAND_OFFSET_SEQUENCE: usize = 8;
const WINDOW_COMMAND_OFFSET_WINDOW: usize = 16;
const WINDOW_COMMAND_OFFSET_FRAME_ID: usize = 24;
const WINDOW_COMMAND_OFFSET_BOUNDS_X: usize = 28;
const WINDOW_COMMAND_OFFSET_DAMAGE_X: usize = 36;
const WINDOW_COMMAND_OFFSET_COLOR: usize = 44;
const WINDOW_COMMAND_OFFSET_BODY_RESERVED: usize = 48;

const WINDOW_EVENT_OFFSET_MAGIC: usize = 0;
const WINDOW_EVENT_OFFSET_VERSION: usize = 4;
const WINDOW_EVENT_OFFSET_OPCODE: usize = 6;
const WINDOW_EVENT_OFFSET_HEADER_RESERVED: usize = 7;
const WINDOW_EVENT_OFFSET_COMMAND_SEQUENCE: usize = 8;
const WINDOW_EVENT_OFFSET_SCENE_FRAME: usize = 16;
const WINDOW_EVENT_OFFSET_WINDOW: usize = 24;
const WINDOW_EVENT_OFFSET_ARGUMENT_0: usize = 32;
const WINDOW_EVENT_OFFSET_ARGUMENT_1: usize = 40;
const WINDOW_EVENT_OFFSET_ARGUMENT_2: usize = 48;
const WINDOW_EVENT_OFFSET_INPUT_STATE: usize = 56;
const WINDOW_EVENT_OFFSET_BODY_RESERVED: usize = 57;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowIdError {
    SlotOutOfRange,
    ZeroGeneration,
}

/// A generation-qualified identity for one of the two bounded window slots.
///
/// The canonical token stores the slot in the low 32 bits and the nonzero
/// allocation generation in the high 32 bits. Requiring the complete low word
/// to be either zero or one rejects tokens that hide data in unused slot bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowId {
    slot: u8,
    generation: u32,
}

impl WindowId {
    pub fn try_new(slot: usize, generation: u32) -> Result<Self, WindowIdError> {
        if slot >= WINDOW_CAPACITY {
            return Err(WindowIdError::SlotOutOfRange);
        }
        if generation == 0 {
            return Err(WindowIdError::ZeroGeneration);
        }
        Ok(Self {
            slot: slot as u8,
            generation,
        })
    }

    pub fn from_token(token: u64) -> Result<Self, WindowIdError> {
        Self::try_new((token as u32) as usize, (token >> 32) as u32)
    }

    pub const fn token(self) -> u64 {
        ((self.generation as u64) << 32) | self.slot as u64
    }

    pub const fn slot(self) -> usize {
        self.slot as usize
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowGeometryError {
    Empty,
    OutOfBounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WindowCommandOpcode {
    Create = 1,
    Present = 2,
    Raise = 3,
    Destroy = 4,
}

impl WindowCommandOpcode {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Create),
            2 => Some(Self::Present),
            3 => Some(Self::Raise),
            4 => Some(Self::Destroy),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCommandError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidOpcode,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSequence,
    InvalidWindowId(WindowIdError),
    InvalidGeometry(WindowGeometryError),
    ZeroFrameId,
    NonCanonicalColor,
    UnexpectedWindowId,
    UnexpectedFrameId,
    UnexpectedBounds,
    UnexpectedDamage,
    UnexpectedColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCommandPayload {
    Create {
        bounds: ShellRect,
    },
    Present {
        window_id: WindowId,
        frame_id: u32,
        damage: ShellRect,
        color: u32,
    },
    Raise {
        window_id: WindowId,
    },
    Destroy {
        window_id: WindowId,
    },
}

/// One canonical 64-byte client-to-SurfaceServer window transaction.
///
/// Every command carries a nonzero ordering sequence. The decoder accepts no
/// alternate representation: fields unused by an opcode and both reserved
/// regions must be zero, rectangles must be nonempty and contained by the
/// configured shell surface, and present colors must have a zero XRGB high
/// byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowCommand {
    sequence: u64,
    payload: WindowCommandPayload,
}

impl WindowCommand {
    pub fn create(sequence: u64, bounds: ShellRect) -> Result<Self, WindowCommandError> {
        validate_window_command_sequence(sequence)?;
        validate_window_geometry(bounds).map_err(WindowCommandError::InvalidGeometry)?;
        Ok(Self {
            sequence,
            payload: WindowCommandPayload::Create { bounds },
        })
    }

    pub fn present(
        sequence: u64,
        window_id: WindowId,
        frame_id: u32,
        damage: ShellRect,
        color: u32,
    ) -> Result<Self, WindowCommandError> {
        validate_window_command_sequence(sequence)?;
        if frame_id == 0 {
            return Err(WindowCommandError::ZeroFrameId);
        }
        validate_window_geometry(damage).map_err(WindowCommandError::InvalidGeometry)?;
        if color & 0xff00_0000 != 0 {
            return Err(WindowCommandError::NonCanonicalColor);
        }
        Ok(Self {
            sequence,
            payload: WindowCommandPayload::Present {
                window_id,
                frame_id,
                damage,
                color,
            },
        })
    }

    pub fn raise(sequence: u64, window_id: WindowId) -> Result<Self, WindowCommandError> {
        validate_window_command_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: WindowCommandPayload::Raise { window_id },
        })
    }

    pub fn destroy(sequence: u64, window_id: WindowId) -> Result<Self, WindowCommandError> {
        validate_window_command_sequence(sequence)?;
        Ok(Self {
            sequence,
            payload: WindowCommandPayload::Destroy { window_id },
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn opcode(self) -> WindowCommandOpcode {
        match self.payload {
            WindowCommandPayload::Create { .. } => WindowCommandOpcode::Create,
            WindowCommandPayload::Present { .. } => WindowCommandOpcode::Present,
            WindowCommandPayload::Raise { .. } => WindowCommandOpcode::Raise,
            WindowCommandPayload::Destroy { .. } => WindowCommandOpcode::Destroy,
        }
    }

    pub const fn payload(self) -> WindowCommandPayload {
        self.payload
    }

    pub const fn window_id(self) -> Option<WindowId> {
        match self.payload {
            WindowCommandPayload::Create { .. } => None,
            WindowCommandPayload::Present { window_id, .. }
            | WindowCommandPayload::Raise { window_id }
            | WindowCommandPayload::Destroy { window_id } => Some(window_id),
        }
    }

    pub fn encode(self) -> [u8; WINDOW_COMMAND_WIRE_SIZE] {
        let mut wire = [0_u8; WINDOW_COMMAND_WIRE_SIZE];
        wire[WINDOW_COMMAND_OFFSET_MAGIC..WINDOW_COMMAND_OFFSET_MAGIC + 4]
            .copy_from_slice(&WINDOW_COMMAND_MAGIC.to_le_bytes());
        wire[WINDOW_COMMAND_OFFSET_VERSION..WINDOW_COMMAND_OFFSET_VERSION + 2]
            .copy_from_slice(&WINDOW_COMMAND_VERSION.to_le_bytes());
        wire[WINDOW_COMMAND_OFFSET_OPCODE] = self.opcode().raw();
        write_window_command_u64(&mut wire, WINDOW_COMMAND_OFFSET_SEQUENCE, self.sequence);
        match self.payload {
            WindowCommandPayload::Create { bounds } => {
                write_window_command_rect(&mut wire, WINDOW_COMMAND_OFFSET_BOUNDS_X, bounds);
            }
            WindowCommandPayload::Present {
                window_id,
                frame_id,
                damage,
                color,
            } => {
                write_window_command_u64(
                    &mut wire,
                    WINDOW_COMMAND_OFFSET_WINDOW,
                    window_id.token(),
                );
                write_window_command_u32(&mut wire, WINDOW_COMMAND_OFFSET_FRAME_ID, frame_id);
                write_window_command_rect(&mut wire, WINDOW_COMMAND_OFFSET_DAMAGE_X, damage);
                write_window_command_u32(&mut wire, WINDOW_COMMAND_OFFSET_COLOR, color);
            }
            WindowCommandPayload::Raise { window_id }
            | WindowCommandPayload::Destroy { window_id } => {
                write_window_command_u64(
                    &mut wire,
                    WINDOW_COMMAND_OFFSET_WINDOW,
                    window_id.token(),
                );
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, WindowCommandError> {
        if wire.len() != WINDOW_COMMAND_WIRE_SIZE {
            return Err(WindowCommandError::InvalidWireLength);
        }
        if read_u32(wire, WINDOW_COMMAND_OFFSET_MAGIC) != WINDOW_COMMAND_MAGIC {
            return Err(WindowCommandError::InvalidMagic);
        }
        if read_u16(wire, WINDOW_COMMAND_OFFSET_VERSION) != WINDOW_COMMAND_VERSION {
            return Err(WindowCommandError::InvalidVersion);
        }
        let opcode = WindowCommandOpcode::from_raw(wire[WINDOW_COMMAND_OFFSET_OPCODE])
            .ok_or(WindowCommandError::InvalidOpcode)?;
        if wire[WINDOW_COMMAND_OFFSET_HEADER_RESERVED] != 0 {
            return Err(WindowCommandError::NonZeroHeaderReserved);
        }
        if wire[WINDOW_COMMAND_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(WindowCommandError::NonZeroBodyReserved);
        }

        let sequence = read_u64(wire, WINDOW_COMMAND_OFFSET_SEQUENCE);
        let window_token = read_u64(wire, WINDOW_COMMAND_OFFSET_WINDOW);
        let frame_id = read_u32(wire, WINDOW_COMMAND_OFFSET_FRAME_ID);
        let bounds = read_window_command_rect(wire, WINDOW_COMMAND_OFFSET_BOUNDS_X);
        let damage = read_window_command_rect(wire, WINDOW_COMMAND_OFFSET_DAMAGE_X);
        let color = read_u32(wire, WINDOW_COMMAND_OFFSET_COLOR);

        match opcode {
            WindowCommandOpcode::Create => {
                if window_token != 0 {
                    return Err(WindowCommandError::UnexpectedWindowId);
                }
                if frame_id != 0 {
                    return Err(WindowCommandError::UnexpectedFrameId);
                }
                if !window_rect_is_zero(damage) {
                    return Err(WindowCommandError::UnexpectedDamage);
                }
                if color != 0 {
                    return Err(WindowCommandError::UnexpectedColor);
                }
                Self::create(sequence, bounds)
            }
            WindowCommandOpcode::Present => {
                if !window_rect_is_zero(bounds) {
                    return Err(WindowCommandError::UnexpectedBounds);
                }
                let window_id = WindowId::from_token(window_token)
                    .map_err(WindowCommandError::InvalidWindowId)?;
                Self::present(sequence, window_id, frame_id, damage, color)
            }
            WindowCommandOpcode::Raise | WindowCommandOpcode::Destroy => {
                if frame_id != 0 {
                    return Err(WindowCommandError::UnexpectedFrameId);
                }
                if !window_rect_is_zero(bounds) {
                    return Err(WindowCommandError::UnexpectedBounds);
                }
                if !window_rect_is_zero(damage) {
                    return Err(WindowCommandError::UnexpectedDamage);
                }
                if color != 0 {
                    return Err(WindowCommandError::UnexpectedColor);
                }
                let window_id = WindowId::from_token(window_token)
                    .map_err(WindowCommandError::InvalidWindowId)?;
                if opcode == WindowCommandOpcode::Raise {
                    Self::raise(sequence, window_id)
                } else {
                    Self::destroy(sequence, window_id)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCommandTrackerError {
    SequenceReplay,
    SequenceExhausted,
}

/// Strictly increasing command-sequence state. Failed accepts do not mutate
/// the last committed sequence, so a caller may reject a transaction and
/// retry with the same expected successor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindowCommandTracker {
    last_sequence: Option<u64>,
}

impl WindowCommandTracker {
    pub const fn new() -> Self {
        Self {
            last_sequence: None,
        }
    }

    pub fn accept(&mut self, command: WindowCommand) -> Result<(), WindowCommandTrackerError> {
        if let Some(previous) = self.last_sequence {
            if previous == u64::MAX {
                return Err(WindowCommandTrackerError::SequenceExhausted);
            }
            if command.sequence <= previous {
                return Err(WindowCommandTrackerError::SequenceReplay);
            }
        }
        self.last_sequence = Some(command.sequence);
        Ok(())
    }

    pub const fn last_sequence(self) -> Option<u64> {
        self.last_sequence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WindowEventOpcode {
    Created = 1,
    Presented = 2,
    Raised = 3,
    InputRoute = 4,
    Destroyed = 5,
}

impl WindowEventOpcode {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Created),
            2 => Some(Self::Presented),
            3 => Some(Self::Raised),
            4 => Some(Self::InputRoute),
            5 => Some(Self::Destroyed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowEventError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidOpcode,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroCommandSequence,
    ZeroSceneFrame,
    SceneFrameOutOfRange,
    InvalidWindowId(WindowIdError),
    InvalidGeometry(WindowGeometryError),
    ZeroFrameId,
    FrameIdOutOfRange,
    GlobalCoordinateOutOfBounds,
    ZeroFocusGeneration,
    NonCanonicalGlobalCoordinates,
    NonCanonicalInputState,
    UnexpectedPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowEventPayload {
    Created {
        window_id: WindowId,
        bounds: ShellRect,
    },
    Presented {
        window_id: WindowId,
        frame_id: u32,
    },
    Raised {
        window_id: WindowId,
    },
    InputRoute {
        window_id: WindowId,
        global_x: u16,
        global_y: u16,
        local_x: i32,
        local_y: i32,
        pressed: bool,
        captured: bool,
        focus_generation: u64,
    },
    Destroyed {
        window_id: WindowId,
    },
}

/// One canonical 64-byte SurfaceServer-to-client window event.
///
/// `command_sequence` identifies the accepted command whose scene state is
/// being reported; input routes use the latest accepted command. Every event
/// also carries the nonzero compositor scene frame against which it was
/// evaluated. Local input coordinates retain their complete signed i32 form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowEvent {
    command_sequence: u64,
    scene_frame: u32,
    payload: WindowEventPayload,
}

impl WindowEvent {
    pub fn created(
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        bounds: ShellRect,
    ) -> Result<Self, WindowEventError> {
        validate_window_event_common(command_sequence, scene_frame)?;
        validate_window_geometry(bounds).map_err(WindowEventError::InvalidGeometry)?;
        Ok(Self {
            command_sequence,
            scene_frame,
            payload: WindowEventPayload::Created { window_id, bounds },
        })
    }

    pub fn presented(
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        frame_id: u32,
    ) -> Result<Self, WindowEventError> {
        validate_window_event_common(command_sequence, scene_frame)?;
        if frame_id == 0 {
            return Err(WindowEventError::ZeroFrameId);
        }
        Ok(Self {
            command_sequence,
            scene_frame,
            payload: WindowEventPayload::Presented {
                window_id,
                frame_id,
            },
        })
    }

    pub fn raised(
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
    ) -> Result<Self, WindowEventError> {
        validate_window_event_common(command_sequence, scene_frame)?;
        Ok(Self {
            command_sequence,
            scene_frame,
            payload: WindowEventPayload::Raised { window_id },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn input_route(
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
        global_x: u16,
        global_y: u16,
        local_x: i32,
        local_y: i32,
        pressed: bool,
        captured: bool,
        focus_generation: u64,
    ) -> Result<Self, WindowEventError> {
        validate_window_event_common(command_sequence, scene_frame)?;
        if global_x >= SURFACE_WIDTH || global_y >= SURFACE_HEIGHT {
            return Err(WindowEventError::GlobalCoordinateOutOfBounds);
        }
        if focus_generation == 0 {
            return Err(WindowEventError::ZeroFocusGeneration);
        }
        Ok(Self {
            command_sequence,
            scene_frame,
            payload: WindowEventPayload::InputRoute {
                window_id,
                global_x,
                global_y,
                local_x,
                local_y,
                pressed,
                captured,
                focus_generation,
            },
        })
    }

    pub fn destroyed(
        command_sequence: u64,
        scene_frame: u32,
        window_id: WindowId,
    ) -> Result<Self, WindowEventError> {
        validate_window_event_common(command_sequence, scene_frame)?;
        Ok(Self {
            command_sequence,
            scene_frame,
            payload: WindowEventPayload::Destroyed { window_id },
        })
    }

    pub const fn command_sequence(self) -> u64 {
        self.command_sequence
    }

    pub const fn scene_frame(self) -> u32 {
        self.scene_frame
    }

    pub const fn opcode(self) -> WindowEventOpcode {
        match self.payload {
            WindowEventPayload::Created { .. } => WindowEventOpcode::Created,
            WindowEventPayload::Presented { .. } => WindowEventOpcode::Presented,
            WindowEventPayload::Raised { .. } => WindowEventOpcode::Raised,
            WindowEventPayload::InputRoute { .. } => WindowEventOpcode::InputRoute,
            WindowEventPayload::Destroyed { .. } => WindowEventOpcode::Destroyed,
        }
    }

    pub const fn payload(self) -> WindowEventPayload {
        self.payload
    }

    pub const fn window_id(self) -> WindowId {
        match self.payload {
            WindowEventPayload::Created { window_id, .. }
            | WindowEventPayload::Presented { window_id, .. }
            | WindowEventPayload::Raised { window_id }
            | WindowEventPayload::InputRoute { window_id, .. }
            | WindowEventPayload::Destroyed { window_id } => window_id,
        }
    }

    pub fn encode(self) -> [u8; WINDOW_EVENT_WIRE_SIZE] {
        let mut wire = [0_u8; WINDOW_EVENT_WIRE_SIZE];
        wire[WINDOW_EVENT_OFFSET_MAGIC..WINDOW_EVENT_OFFSET_MAGIC + 4]
            .copy_from_slice(&WINDOW_EVENT_MAGIC.to_le_bytes());
        wire[WINDOW_EVENT_OFFSET_VERSION..WINDOW_EVENT_OFFSET_VERSION + 2]
            .copy_from_slice(&WINDOW_EVENT_VERSION.to_le_bytes());
        wire[WINDOW_EVENT_OFFSET_OPCODE] = self.opcode().raw();
        write_window_event_u64(
            &mut wire,
            WINDOW_EVENT_OFFSET_COMMAND_SEQUENCE,
            self.command_sequence,
        );
        write_window_event_u64(
            &mut wire,
            WINDOW_EVENT_OFFSET_SCENE_FRAME,
            u64::from(self.scene_frame),
        );
        write_window_event_u64(
            &mut wire,
            WINDOW_EVENT_OFFSET_WINDOW,
            self.window_id().token(),
        );
        match self.payload {
            WindowEventPayload::Created { bounds, .. } => {
                write_window_event_u64(
                    &mut wire,
                    WINDOW_EVENT_OFFSET_ARGUMENT_0,
                    pack_window_rect(bounds),
                );
            }
            WindowEventPayload::Presented { frame_id, .. } => {
                write_window_event_u64(
                    &mut wire,
                    WINDOW_EVENT_OFFSET_ARGUMENT_0,
                    u64::from(frame_id),
                );
            }
            WindowEventPayload::Raised { .. } | WindowEventPayload::Destroyed { .. } => {}
            WindowEventPayload::InputRoute {
                global_x,
                global_y,
                local_x,
                local_y,
                pressed,
                captured,
                focus_generation,
                ..
            } => {
                write_window_event_u64(&mut wire, WINDOW_EVENT_OFFSET_ARGUMENT_0, focus_generation);
                write_window_event_u64(
                    &mut wire,
                    WINDOW_EVENT_OFFSET_ARGUMENT_1,
                    u64::from(global_x) | (u64::from(global_y) << 16),
                );
                write_window_event_u64(
                    &mut wire,
                    WINDOW_EVENT_OFFSET_ARGUMENT_2,
                    u64::from(local_x as u32) | (u64::from(local_y as u32) << 32),
                );
                wire[WINDOW_EVENT_OFFSET_INPUT_STATE] =
                    u8::from(pressed) | (u8::from(captured) << 1);
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, WindowEventError> {
        if wire.len() != WINDOW_EVENT_WIRE_SIZE {
            return Err(WindowEventError::InvalidWireLength);
        }
        if read_u32(wire, WINDOW_EVENT_OFFSET_MAGIC) != WINDOW_EVENT_MAGIC {
            return Err(WindowEventError::InvalidMagic);
        }
        if read_u16(wire, WINDOW_EVENT_OFFSET_VERSION) != WINDOW_EVENT_VERSION {
            return Err(WindowEventError::InvalidVersion);
        }
        let opcode = WindowEventOpcode::from_raw(wire[WINDOW_EVENT_OFFSET_OPCODE])
            .ok_or(WindowEventError::InvalidOpcode)?;
        if wire[WINDOW_EVENT_OFFSET_HEADER_RESERVED] != 0 {
            return Err(WindowEventError::NonZeroHeaderReserved);
        }
        if wire[WINDOW_EVENT_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(WindowEventError::NonZeroBodyReserved);
        }

        let command_sequence = read_u64(wire, WINDOW_EVENT_OFFSET_COMMAND_SEQUENCE);
        let scene_frame = u32::try_from(read_u64(wire, WINDOW_EVENT_OFFSET_SCENE_FRAME))
            .map_err(|_| WindowEventError::SceneFrameOutOfRange)?;
        let window_id = WindowId::from_token(read_u64(wire, WINDOW_EVENT_OFFSET_WINDOW))
            .map_err(WindowEventError::InvalidWindowId)?;
        let argument_0 = read_u64(wire, WINDOW_EVENT_OFFSET_ARGUMENT_0);
        let argument_1 = read_u64(wire, WINDOW_EVENT_OFFSET_ARGUMENT_1);
        let argument_2 = read_u64(wire, WINDOW_EVENT_OFFSET_ARGUMENT_2);
        let input_state = wire[WINDOW_EVENT_OFFSET_INPUT_STATE];

        match opcode {
            WindowEventOpcode::Created => {
                if argument_1 != 0 || argument_2 != 0 || input_state != 0 {
                    return Err(WindowEventError::UnexpectedPayload);
                }
                Self::created(
                    command_sequence,
                    scene_frame,
                    window_id,
                    unpack_window_rect(argument_0),
                )
            }
            WindowEventOpcode::Presented => {
                if argument_1 != 0 || argument_2 != 0 || input_state != 0 {
                    return Err(WindowEventError::UnexpectedPayload);
                }
                let frame_id =
                    u32::try_from(argument_0).map_err(|_| WindowEventError::FrameIdOutOfRange)?;
                Self::presented(command_sequence, scene_frame, window_id, frame_id)
            }
            WindowEventOpcode::Raised | WindowEventOpcode::Destroyed => {
                if argument_0 != 0 || argument_1 != 0 || argument_2 != 0 || input_state != 0 {
                    return Err(WindowEventError::UnexpectedPayload);
                }
                if opcode == WindowEventOpcode::Raised {
                    Self::raised(command_sequence, scene_frame, window_id)
                } else {
                    Self::destroyed(command_sequence, scene_frame, window_id)
                }
            }
            WindowEventOpcode::InputRoute => {
                if argument_1 >> 32 != 0 {
                    return Err(WindowEventError::NonCanonicalGlobalCoordinates);
                }
                if input_state & !0b11 != 0 {
                    return Err(WindowEventError::NonCanonicalInputState);
                }
                Self::input_route(
                    command_sequence,
                    scene_frame,
                    window_id,
                    argument_1 as u16,
                    (argument_1 >> 16) as u16,
                    argument_2 as u32 as i32,
                    (argument_2 >> 32) as u32 as i32,
                    input_state & 1 != 0,
                    input_state & 2 != 0,
                    argument_0,
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowEventTrackerPhase {
    #[default]
    AwaitingCreated,
    Live,
    Destroyed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowEventTrackerError {
    CreatedRequired,
    DuplicateCreated,
    WindowMismatch,
    EventAfterDestroyed,
    LifecycleNotTerminated,
    WindowGenerationReplay,
    AcknowledgedCommandNotIncreasing,
    InputCommandBehind,
    InputCommandAhead,
    SceneFrameRegression,
    PresentedFrameNotIncreasing,
    FocusGenerationRegression,
}

/// Fail-closed client-side state for one window-event channel.
///
/// A channel starts with exactly one `Created` event and binds to that event's
/// generation-qualified `WindowId` until `Destroyed`. Command-sequence and
/// scene-frame floors span every lifecycle on the channel, while presentation
/// frame and focus generations are local to one lifecycle. Call
/// [`Self::begin_next_lifecycle`] after `Destroyed`; constructing a fresh
/// tracker is only appropriate for a fresh channel.
///
/// Every non-input event acknowledges a command strictly newer than the last
/// acknowledged command. `InputRoute` instead has to reference that exact
/// latest command, while scene frames never decrease. Consequently only input
/// routes may reuse an exact `(command_sequence, scene_frame)` pair. This lets
/// the first routed input share the scene transaction that selected its target
/// and lets a contiguous input batch reuse that pair, while no acknowledgement
/// or lifecycle event can be replayed under a later scene frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowEventTracker {
    phase: WindowEventTrackerPhase,
    window_id: Option<WindowId>,
    bounds: Option<ShellRect>,
    last_acknowledged_command_sequence: Option<u64>,
    last_scene_frame: Option<u32>,
    last_presented_frame_id: Option<u32>,
    last_focus_generation: Option<u64>,
    last_opcode: Option<WindowEventOpcode>,
    generation_floors: [u32; WINDOW_CAPACITY],
}

impl Default for WindowEventTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowEventTracker {
    pub const fn new() -> Self {
        Self {
            phase: WindowEventTrackerPhase::AwaitingCreated,
            window_id: None,
            bounds: None,
            last_acknowledged_command_sequence: None,
            last_scene_frame: None,
            last_presented_frame_id: None,
            last_focus_generation: None,
            last_opcode: None,
            generation_floors: [0; WINDOW_CAPACITY],
        }
    }

    /// Ends the terminal state for the same channel without discarding its
    /// cross-lifecycle ordering or per-slot generation floors.
    pub fn begin_next_lifecycle(&mut self) -> Result<(), WindowEventTrackerError> {
        if self.phase != WindowEventTrackerPhase::Destroyed {
            return Err(WindowEventTrackerError::LifecycleNotTerminated);
        }
        self.phase = WindowEventTrackerPhase::AwaitingCreated;
        self.window_id = None;
        self.bounds = None;
        self.last_presented_frame_id = None;
        self.last_focus_generation = None;
        self.last_opcode = None;
        Ok(())
    }

    /// Validates and commits one event. Validation is performed against a
    /// private copy so every error leaves all observable state unchanged.
    pub fn accept(&mut self, event: WindowEvent) -> Result<(), WindowEventTrackerError> {
        let mut candidate = *self;
        candidate.accept_inner(event)?;
        *self = candidate;
        Ok(())
    }

    fn accept_inner(&mut self, event: WindowEvent) -> Result<(), WindowEventTrackerError> {
        let opcode = event.opcode();
        match self.phase {
            WindowEventTrackerPhase::AwaitingCreated => {
                if opcode != WindowEventOpcode::Created {
                    return Err(WindowEventTrackerError::CreatedRequired);
                }
            }
            WindowEventTrackerPhase::Live => {
                if opcode == WindowEventOpcode::Created {
                    return Err(WindowEventTrackerError::DuplicateCreated);
                }
                if Some(event.window_id()) != self.window_id {
                    return Err(WindowEventTrackerError::WindowMismatch);
                }
            }
            WindowEventTrackerPhase::Destroyed => {
                return Err(WindowEventTrackerError::EventAfterDestroyed);
            }
        }

        if opcode == WindowEventOpcode::InputRoute {
            match self.last_acknowledged_command_sequence {
                Some(previous) if event.command_sequence() < previous => {
                    return Err(WindowEventTrackerError::InputCommandBehind);
                }
                Some(previous) if event.command_sequence() > previous => {
                    return Err(WindowEventTrackerError::InputCommandAhead);
                }
                Some(_) => {}
                None => return Err(WindowEventTrackerError::CreatedRequired),
            }
        } else if self
            .last_acknowledged_command_sequence
            .is_some_and(|previous| event.command_sequence() <= previous)
        {
            return Err(WindowEventTrackerError::AcknowledgedCommandNotIncreasing);
        }
        if self
            .last_scene_frame
            .is_some_and(|previous| event.scene_frame() < previous)
        {
            return Err(WindowEventTrackerError::SceneFrameRegression);
        }
        match event.payload() {
            WindowEventPayload::Created { window_id, bounds } => {
                if window_id.generation() <= self.generation_floors[window_id.slot()] {
                    return Err(WindowEventTrackerError::WindowGenerationReplay);
                }
                self.phase = WindowEventTrackerPhase::Live;
                self.window_id = Some(window_id);
                self.bounds = Some(bounds);
                self.generation_floors[window_id.slot()] = window_id.generation();
            }
            WindowEventPayload::Presented { frame_id, .. } => {
                if self
                    .last_presented_frame_id
                    .is_some_and(|previous| frame_id <= previous)
                {
                    return Err(WindowEventTrackerError::PresentedFrameNotIncreasing);
                }
                self.last_presented_frame_id = Some(frame_id);
            }
            WindowEventPayload::InputRoute {
                focus_generation, ..
            } => {
                if self
                    .last_focus_generation
                    .is_some_and(|previous| focus_generation < previous)
                {
                    return Err(WindowEventTrackerError::FocusGenerationRegression);
                }
                self.last_focus_generation = Some(focus_generation);
            }
            WindowEventPayload::Destroyed { .. } => {
                self.phase = WindowEventTrackerPhase::Destroyed;
            }
            WindowEventPayload::Raised { .. } => {}
        }

        if opcode != WindowEventOpcode::InputRoute {
            self.last_acknowledged_command_sequence = Some(event.command_sequence());
        }
        self.last_scene_frame = Some(event.scene_frame());
        self.last_opcode = Some(opcode);
        Ok(())
    }

    pub const fn phase(self) -> WindowEventTrackerPhase {
        self.phase
    }

    pub const fn window_id(self) -> Option<WindowId> {
        self.window_id
    }

    pub const fn bounds(self) -> Option<ShellRect> {
        self.bounds
    }

    pub const fn last_command_sequence(self) -> Option<u64> {
        self.last_acknowledged_command_sequence
    }

    pub const fn last_acknowledged_command_sequence(self) -> Option<u64> {
        self.last_acknowledged_command_sequence
    }

    pub const fn last_scene_frame(self) -> Option<u32> {
        self.last_scene_frame
    }

    pub const fn last_pair(self) -> Option<(u64, u32)> {
        match (
            self.last_acknowledged_command_sequence,
            self.last_scene_frame,
        ) {
            (Some(command_sequence), Some(scene_frame)) => Some((command_sequence, scene_frame)),
            _ => None,
        }
    }

    pub const fn last_presented_frame_id(self) -> Option<u32> {
        self.last_presented_frame_id
    }

    pub const fn last_focus_generation(self) -> Option<u64> {
        self.last_focus_generation
    }

    pub const fn last_opcode(self) -> Option<WindowEventOpcode> {
        self.last_opcode
    }

    pub const fn generation_floor(self, slot: usize) -> Option<u32> {
        if slot >= WINDOW_CAPACITY || self.generation_floors[slot] == 0 {
            None
        } else {
            Some(self.generation_floors[slot])
        }
    }

    pub const fn is_live(self) -> bool {
        matches!(self.phase, WindowEventTrackerPhase::Live)
    }

    pub const fn is_destroyed(self) -> bool {
        matches!(self.phase, WindowEventTrackerPhase::Destroyed)
    }
}

const TEXT_INPUT_OFFSET_MAGIC: usize = 0;
const TEXT_INPUT_OFFSET_VERSION: usize = 4;
const TEXT_INPUT_OFFSET_OPCODE: usize = 6;
const TEXT_INPUT_OFFSET_HEADER_RESERVED: usize = 7;
const TEXT_INPUT_OFFSET_SEQUENCE: usize = 8;
const TEXT_INPUT_OFFSET_WINDOW: usize = 16;
const TEXT_INPUT_OFFSET_SESSION: usize = 24;
const TEXT_INPUT_OFFSET_FOCUS: usize = 32;
const TEXT_INPUT_OFFSET_ACKNOWLEDGMENT: usize = 40;
const TEXT_INPUT_OFFSET_REVISION: usize = 48;
const TEXT_INPUT_OFFSET_ARGUMENT_0: usize = 52;
const TEXT_INPUT_OFFSET_ARGUMENT_1: usize = 53;
const TEXT_INPUT_OFFSET_TEXT_LENGTH: usize = 54;
const TEXT_INPUT_OFFSET_BODY_RESERVED: usize = 55;
const TEXT_INPUT_OFFSET_TEXT: usize = 56;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextStateError {
    TextTooLong,
    InvalidUtf8,
    NonCanonicalTail,
    InvalidSelection,
    EmptyText,
    NotSingleScalar,
}

/// An allocation-free UTF-8 value with one canonical representation.
///
/// The unused suffix is always zero. Selections elsewhere in the protocol are
/// byte offsets into this value and therefore have to land on UTF-8 character
/// boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextBytes8 {
    bytes: [u8; TEXT_INPUT_CAPACITY],
    len: u8,
}

impl TextBytes8 {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; TEXT_INPUT_CAPACITY],
            len: 0,
        }
    }

    pub fn try_from_str(text: &str) -> Result<Self, TextStateError> {
        if text.len() > TEXT_INPUT_CAPACITY {
            return Err(TextStateError::TextTooLong);
        }
        let mut bytes = [0_u8; TEXT_INPUT_CAPACITY];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Ok(Self {
            bytes,
            len: text.len() as u8,
        })
    }

    fn decode(bytes: [u8; TEXT_INPUT_CAPACITY], len: u8) -> Result<Self, TextStateError> {
        let len = len as usize;
        if len > TEXT_INPUT_CAPACITY {
            return Err(TextStateError::TextTooLong);
        }
        if bytes[len..].iter().any(|byte| *byte != 0) {
            return Err(TextStateError::NonCanonicalTail);
        }
        core::str::from_utf8(&bytes[..len]).map_err(|_| TextStateError::InvalidUtf8)?;
        Ok(Self {
            bytes,
            len: len as u8,
        })
    }

    fn try_from_scalar(scalar: char) -> Self {
        let mut encoded = [0_u8; TEXT_INPUT_CAPACITY];
        let len = scalar.encode_utf8(&mut encoded).len();
        Self {
            bytes: encoded,
            len: len as u8,
        }
    }

    pub const fn len(self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("TextBytes8 invariant")
    }

    pub const fn storage(self) -> [u8; TEXT_INPUT_CAPACITY] {
        self.bytes
    }

    fn single_scalar(self) -> Result<char, TextStateError> {
        let mut scalars = self.as_str().chars();
        let scalar = scalars.next().ok_or(TextStateError::NotSingleScalar)?;
        if scalars.next().is_some() {
            return Err(TextStateError::NotSingleScalar);
        }
        Ok(scalar)
    }
}

impl Default for TextBytes8 {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextState {
    revision: u32,
    committed: TextBytes8,
    selection_start: u8,
    selection_end: u8,
}

impl TextState {
    pub fn try_new(
        revision: u32,
        committed: &str,
        selection_start: u8,
        selection_end: u8,
    ) -> Result<Self, TextStateError> {
        let committed = TextBytes8::try_from_str(committed)?;
        Self::from_text(revision, committed, selection_start, selection_end)
    }

    fn from_text(
        revision: u32,
        committed: TextBytes8,
        selection_start: u8,
        selection_end: u8,
    ) -> Result<Self, TextStateError> {
        validate_text_selection(committed.as_str(), selection_start, selection_end)?;
        Ok(Self {
            revision,
            committed,
            selection_start,
            selection_end,
        })
    }

    pub const fn revision(self) -> u32 {
        self.revision
    }

    pub const fn committed(self) -> TextBytes8 {
        self.committed
    }

    pub const fn selection_start(self) -> u8 {
        self.selection_start
    }

    pub const fn selection_end(self) -> u8 {
        self.selection_end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputContextError {
    ZeroSession,
    ZeroFocusGeneration,
}

/// The immutable identity of one focused text-input session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputContext {
    window_id: WindowId,
    session_id: u64,
    focus_generation: u64,
}

impl TextInputContext {
    pub fn try_new(
        window_id: WindowId,
        session_id: u64,
        focus_generation: u64,
    ) -> Result<Self, TextInputContextError> {
        if session_id == 0 {
            return Err(TextInputContextError::ZeroSession);
        }
        if focus_generation == 0 {
            return Err(TextInputContextError::ZeroFocusGeneration);
        }
        Ok(Self {
            window_id,
            session_id,
            focus_generation,
        })
    }

    pub const fn window_id(self) -> WindowId {
        self.window_id
    }

    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    pub const fn focus_generation(self) -> u64 {
        self.focus_generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TextInputCommandOpcode {
    Activate = 1,
    StateAck = 2,
    Deactivate = 3,
}

impl TextInputCommandOpcode {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Activate),
            2 => Some(Self::StateAck),
            3 => Some(Self::Deactivate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputCommandError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidOpcode,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSequence,
    InvalidWindowId(WindowIdError),
    InvalidContext(TextInputContextError),
    ZeroAcknowledgedEventSequence,
    UnexpectedPayload,
    InvalidTextState(TextStateError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputCommandPayload {
    Activate,
    StateAck {
        acknowledged_event_sequence: u64,
        state: TextState,
    },
    Deactivate,
}

/// BTI1: one canonical, fixed-size client-to-text-service transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputCommand {
    sequence: u64,
    context: TextInputContext,
    payload: TextInputCommandPayload,
}

impl TextInputCommand {
    pub fn activate(
        sequence: u64,
        context: TextInputContext,
    ) -> Result<Self, TextInputCommandError> {
        validate_text_input_sequence(sequence).map_err(TextInputCommandError::from)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputCommandPayload::Activate,
        })
    }

    pub fn state_ack(
        sequence: u64,
        context: TextInputContext,
        acknowledged_event_sequence: u64,
        state: TextState,
    ) -> Result<Self, TextInputCommandError> {
        validate_text_input_sequence(sequence).map_err(TextInputCommandError::from)?;
        if acknowledged_event_sequence == 0 {
            return Err(TextInputCommandError::ZeroAcknowledgedEventSequence);
        }
        Ok(Self {
            sequence,
            context,
            payload: TextInputCommandPayload::StateAck {
                acknowledged_event_sequence,
                state,
            },
        })
    }

    pub fn deactivate(
        sequence: u64,
        context: TextInputContext,
    ) -> Result<Self, TextInputCommandError> {
        validate_text_input_sequence(sequence).map_err(TextInputCommandError::from)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputCommandPayload::Deactivate,
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn context(self) -> TextInputContext {
        self.context
    }

    pub const fn opcode(self) -> TextInputCommandOpcode {
        match self.payload {
            TextInputCommandPayload::Activate => TextInputCommandOpcode::Activate,
            TextInputCommandPayload::StateAck { .. } => TextInputCommandOpcode::StateAck,
            TextInputCommandPayload::Deactivate => TextInputCommandOpcode::Deactivate,
        }
    }

    pub const fn payload(self) -> TextInputCommandPayload {
        self.payload
    }

    pub fn encode(self) -> [u8; TEXT_INPUT_COMMAND_WIRE_SIZE] {
        let mut wire = [0_u8; TEXT_INPUT_COMMAND_WIRE_SIZE];
        encode_text_input_header(
            &mut wire,
            TEXT_INPUT_COMMAND_MAGIC,
            TEXT_INPUT_COMMAND_VERSION,
            self.opcode().raw(),
            self.sequence,
            self.context,
        );
        if let TextInputCommandPayload::StateAck {
            acknowledged_event_sequence,
            state,
        } = self.payload
        {
            write_u64_at(
                &mut wire,
                TEXT_INPUT_OFFSET_ACKNOWLEDGMENT,
                acknowledged_event_sequence,
            );
            encode_text_state(&mut wire, state);
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, TextInputCommandError> {
        if wire.len() != TEXT_INPUT_COMMAND_WIRE_SIZE {
            return Err(TextInputCommandError::InvalidWireLength);
        }
        if read_u32(wire, TEXT_INPUT_OFFSET_MAGIC) != TEXT_INPUT_COMMAND_MAGIC {
            return Err(TextInputCommandError::InvalidMagic);
        }
        if read_u16(wire, TEXT_INPUT_OFFSET_VERSION) != TEXT_INPUT_COMMAND_VERSION {
            return Err(TextInputCommandError::InvalidVersion);
        }
        let opcode = TextInputCommandOpcode::from_raw(wire[TEXT_INPUT_OFFSET_OPCODE])
            .ok_or(TextInputCommandError::InvalidOpcode)?;
        if wire[TEXT_INPUT_OFFSET_HEADER_RESERVED] != 0 {
            return Err(TextInputCommandError::NonZeroHeaderReserved);
        }
        if wire[TEXT_INPUT_OFFSET_BODY_RESERVED] != 0 {
            return Err(TextInputCommandError::NonZeroBodyReserved);
        }
        let sequence = read_u64(wire, TEXT_INPUT_OFFSET_SEQUENCE);
        let context = decode_text_input_context(wire).map_err(|error| match error {
            TextInputHeaderError::InvalidWindowId(error) => {
                TextInputCommandError::InvalidWindowId(error)
            }
            TextInputHeaderError::InvalidContext(error) => {
                TextInputCommandError::InvalidContext(error)
            }
            TextInputHeaderError::ZeroSequence => TextInputCommandError::ZeroSequence,
        })?;
        let acknowledgment = read_u64(wire, TEXT_INPUT_OFFSET_ACKNOWLEDGMENT);
        let revision = read_u32(wire, TEXT_INPUT_OFFSET_REVISION);
        let argument_0 = wire[TEXT_INPUT_OFFSET_ARGUMENT_0];
        let argument_1 = wire[TEXT_INPUT_OFFSET_ARGUMENT_1];
        let text = decode_text_tail(wire).map_err(TextInputCommandError::InvalidTextState)?;

        match opcode {
            TextInputCommandOpcode::Activate | TextInputCommandOpcode::Deactivate => {
                if acknowledgment != 0
                    || revision != 0
                    || argument_0 != 0
                    || argument_1 != 0
                    || !text.is_empty()
                {
                    return Err(TextInputCommandError::UnexpectedPayload);
                }
                if opcode == TextInputCommandOpcode::Activate {
                    Self::activate(sequence, context)
                } else {
                    Self::deactivate(sequence, context)
                }
            }
            TextInputCommandOpcode::StateAck => {
                let state = TextState::from_text(revision, text, argument_0, argument_1)
                    .map_err(TextInputCommandError::InvalidTextState)?;
                Self::state_ack(sequence, context, acknowledgment, state)
            }
        }
    }
}

impl From<TextInputHeaderError> for TextInputCommandError {
    fn from(error: TextInputHeaderError) -> Self {
        match error {
            TextInputHeaderError::ZeroSequence => Self::ZeroSequence,
            TextInputHeaderError::InvalidWindowId(error) => Self::InvalidWindowId(error),
            TextInputHeaderError::InvalidContext(error) => Self::InvalidContext(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TextInputEventOpcode {
    Activated = 1,
    Preedit = 2,
    Commit = 3,
    DeleteSurrounding = 4,
    Rendered = 5,
    Deactivated = 6,
}

impl TextInputEventOpcode {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Activated),
            2 => Some(Self::Preedit),
            3 => Some(Self::Commit),
            4 => Some(Self::DeleteSurrounding),
            5 => Some(Self::Rendered),
            6 => Some(Self::Deactivated),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputEventError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidOpcode,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSequence,
    InvalidWindowId(WindowIdError),
    InvalidContext(TextInputContextError),
    ZeroAcknowledgedCommandSequence,
    UnexpectedPayload,
    EmptyCommit,
    EmptyDelete,
    InvalidText(TextStateError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputEventPayload {
    Activated {
        acknowledged_command_sequence: u64,
        revision: u32,
    },
    Preedit {
        revision: u32,
        scalar: char,
    },
    Commit {
        revision: u32,
        text: TextBytes8,
    },
    DeleteSurrounding {
        revision: u32,
        before_scalars: u8,
        after_scalars: u8,
    },
    Rendered {
        acknowledged_command_sequence: u64,
        state: TextState,
    },
    Deactivated {
        acknowledged_command_sequence: u64,
        revision: u32,
    },
}

/// BTE1: one canonical, fixed-size text-service-to-client transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputEvent {
    sequence: u64,
    context: TextInputContext,
    payload: TextInputEventPayload,
}

impl TextInputEvent {
    pub fn activated(
        sequence: u64,
        context: TextInputContext,
        acknowledged_command_sequence: u64,
        revision: u32,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        validate_acknowledged_command_sequence(acknowledged_command_sequence)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::Activated {
                acknowledged_command_sequence,
                revision,
            },
        })
    }

    pub fn preedit(
        sequence: u64,
        context: TextInputContext,
        revision: u32,
        scalar: char,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::Preedit { revision, scalar },
        })
    }

    pub fn commit(
        sequence: u64,
        context: TextInputContext,
        revision: u32,
        text: &str,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        let text = TextBytes8::try_from_str(text).map_err(TextInputEventError::InvalidText)?;
        if text.is_empty() {
            return Err(TextInputEventError::EmptyCommit);
        }
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::Commit { revision, text },
        })
    }

    pub fn delete_surrounding(
        sequence: u64,
        context: TextInputContext,
        revision: u32,
        before_scalars: u8,
        after_scalars: u8,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        if before_scalars == 0 && after_scalars == 0 {
            return Err(TextInputEventError::EmptyDelete);
        }
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::DeleteSurrounding {
                revision,
                before_scalars,
                after_scalars,
            },
        })
    }

    pub fn rendered(
        sequence: u64,
        context: TextInputContext,
        acknowledged_command_sequence: u64,
        state: TextState,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        validate_acknowledged_command_sequence(acknowledged_command_sequence)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::Rendered {
                acknowledged_command_sequence,
                state,
            },
        })
    }

    pub fn deactivated(
        sequence: u64,
        context: TextInputContext,
        acknowledged_command_sequence: u64,
        revision: u32,
    ) -> Result<Self, TextInputEventError> {
        validate_text_input_sequence(sequence).map_err(TextInputEventError::from)?;
        validate_acknowledged_command_sequence(acknowledged_command_sequence)?;
        Ok(Self {
            sequence,
            context,
            payload: TextInputEventPayload::Deactivated {
                acknowledged_command_sequence,
                revision,
            },
        })
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub const fn context(self) -> TextInputContext {
        self.context
    }

    pub const fn opcode(self) -> TextInputEventOpcode {
        match self.payload {
            TextInputEventPayload::Activated { .. } => TextInputEventOpcode::Activated,
            TextInputEventPayload::Preedit { .. } => TextInputEventOpcode::Preedit,
            TextInputEventPayload::Commit { .. } => TextInputEventOpcode::Commit,
            TextInputEventPayload::DeleteSurrounding { .. } => {
                TextInputEventOpcode::DeleteSurrounding
            }
            TextInputEventPayload::Rendered { .. } => TextInputEventOpcode::Rendered,
            TextInputEventPayload::Deactivated { .. } => TextInputEventOpcode::Deactivated,
        }
    }

    pub const fn payload(self) -> TextInputEventPayload {
        self.payload
    }

    pub const fn revision(self) -> u32 {
        match self.payload {
            TextInputEventPayload::Activated { revision, .. }
            | TextInputEventPayload::Preedit { revision, .. }
            | TextInputEventPayload::Commit { revision, .. }
            | TextInputEventPayload::DeleteSurrounding { revision, .. }
            | TextInputEventPayload::Deactivated { revision, .. } => revision,
            TextInputEventPayload::Rendered { state, .. } => state.revision(),
        }
    }

    pub fn encode(self) -> [u8; TEXT_INPUT_EVENT_WIRE_SIZE] {
        let mut wire = [0_u8; TEXT_INPUT_EVENT_WIRE_SIZE];
        encode_text_input_header(
            &mut wire,
            TEXT_INPUT_EVENT_MAGIC,
            TEXT_INPUT_EVENT_VERSION,
            self.opcode().raw(),
            self.sequence,
            self.context,
        );
        match self.payload {
            TextInputEventPayload::Activated {
                acknowledged_command_sequence,
                revision,
            }
            | TextInputEventPayload::Deactivated {
                acknowledged_command_sequence,
                revision,
            } => {
                write_u64_at(
                    &mut wire,
                    TEXT_INPUT_OFFSET_ACKNOWLEDGMENT,
                    acknowledged_command_sequence,
                );
                write_u32_at(&mut wire, TEXT_INPUT_OFFSET_REVISION, revision);
            }
            TextInputEventPayload::Preedit { revision, scalar } => {
                write_u32_at(&mut wire, TEXT_INPUT_OFFSET_REVISION, revision);
                encode_text_tail(&mut wire, TextBytes8::try_from_scalar(scalar));
            }
            TextInputEventPayload::Commit { revision, text } => {
                write_u32_at(&mut wire, TEXT_INPUT_OFFSET_REVISION, revision);
                encode_text_tail(&mut wire, text);
            }
            TextInputEventPayload::DeleteSurrounding {
                revision,
                before_scalars,
                after_scalars,
            } => {
                write_u32_at(&mut wire, TEXT_INPUT_OFFSET_REVISION, revision);
                wire[TEXT_INPUT_OFFSET_ARGUMENT_0] = before_scalars;
                wire[TEXT_INPUT_OFFSET_ARGUMENT_1] = after_scalars;
            }
            TextInputEventPayload::Rendered {
                acknowledged_command_sequence,
                state,
            } => {
                write_u64_at(
                    &mut wire,
                    TEXT_INPUT_OFFSET_ACKNOWLEDGMENT,
                    acknowledged_command_sequence,
                );
                encode_text_state(&mut wire, state);
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, TextInputEventError> {
        if wire.len() != TEXT_INPUT_EVENT_WIRE_SIZE {
            return Err(TextInputEventError::InvalidWireLength);
        }
        if read_u32(wire, TEXT_INPUT_OFFSET_MAGIC) != TEXT_INPUT_EVENT_MAGIC {
            return Err(TextInputEventError::InvalidMagic);
        }
        if read_u16(wire, TEXT_INPUT_OFFSET_VERSION) != TEXT_INPUT_EVENT_VERSION {
            return Err(TextInputEventError::InvalidVersion);
        }
        let opcode = TextInputEventOpcode::from_raw(wire[TEXT_INPUT_OFFSET_OPCODE])
            .ok_or(TextInputEventError::InvalidOpcode)?;
        if wire[TEXT_INPUT_OFFSET_HEADER_RESERVED] != 0 {
            return Err(TextInputEventError::NonZeroHeaderReserved);
        }
        if wire[TEXT_INPUT_OFFSET_BODY_RESERVED] != 0 {
            return Err(TextInputEventError::NonZeroBodyReserved);
        }
        let sequence = read_u64(wire, TEXT_INPUT_OFFSET_SEQUENCE);
        let context = decode_text_input_context(wire).map_err(TextInputEventError::from)?;
        let acknowledgment = read_u64(wire, TEXT_INPUT_OFFSET_ACKNOWLEDGMENT);
        let revision = read_u32(wire, TEXT_INPUT_OFFSET_REVISION);
        let argument_0 = wire[TEXT_INPUT_OFFSET_ARGUMENT_0];
        let argument_1 = wire[TEXT_INPUT_OFFSET_ARGUMENT_1];
        let text = decode_text_tail(wire).map_err(TextInputEventError::InvalidText)?;

        match opcode {
            TextInputEventOpcode::Activated | TextInputEventOpcode::Deactivated => {
                if argument_0 != 0 || argument_1 != 0 || !text.is_empty() {
                    return Err(TextInputEventError::UnexpectedPayload);
                }
                if opcode == TextInputEventOpcode::Activated {
                    Self::activated(sequence, context, acknowledgment, revision)
                } else {
                    Self::deactivated(sequence, context, acknowledgment, revision)
                }
            }
            TextInputEventOpcode::Preedit => {
                if acknowledgment != 0 || argument_0 != 0 || argument_1 != 0 {
                    return Err(TextInputEventError::UnexpectedPayload);
                }
                let scalar = text
                    .single_scalar()
                    .map_err(TextInputEventError::InvalidText)?;
                Self::preedit(sequence, context, revision, scalar)
            }
            TextInputEventOpcode::Commit => {
                if acknowledgment != 0 || argument_0 != 0 || argument_1 != 0 {
                    return Err(TextInputEventError::UnexpectedPayload);
                }
                Self::commit(sequence, context, revision, text.as_str())
            }
            TextInputEventOpcode::DeleteSurrounding => {
                if acknowledgment != 0 || !text.is_empty() {
                    return Err(TextInputEventError::UnexpectedPayload);
                }
                Self::delete_surrounding(sequence, context, revision, argument_0, argument_1)
            }
            TextInputEventOpcode::Rendered => {
                let state = TextState::from_text(revision, text, argument_0, argument_1)
                    .map_err(TextInputEventError::InvalidText)?;
                Self::rendered(sequence, context, acknowledgment, state)
            }
        }
    }
}

impl From<TextInputHeaderError> for TextInputEventError {
    fn from(error: TextInputHeaderError) -> Self {
        match error {
            TextInputHeaderError::ZeroSequence => Self::ZeroSequence,
            TextInputHeaderError::InvalidWindowId(error) => Self::InvalidWindowId(error),
            TextInputHeaderError::InvalidContext(error) => Self::InvalidContext(error),
        }
    }
}

fn validate_acknowledged_command_sequence(sequence: u64) -> Result<(), TextInputEventError> {
    if sequence == 0 {
        return Err(TextInputEventError::ZeroAcknowledgedCommandSequence);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEditorError {
    AlreadyActive,
    Inactive,
    UnexpectedEvent,
    RevisionReplay,
    RevisionGap,
    RevisionExhausted,
    CapacityExceeded,
    DeleteOutOfRange,
    StateMismatch,
    InvalidState(TextStateError),
}

/// Transactional bounded editor used by the first text-input path.
///
/// Committed text occupies at most eight UTF-8 bytes, selections are canonical
/// byte boundaries, and preedit contains at most one Unicode scalar. Every
/// mutator validates a private copy before committing it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextEditor {
    state: TextState,
    preedit: Option<char>,
    active: bool,
}

impl TextEditor {
    pub const fn new() -> Self {
        Self {
            state: TextState {
                revision: 0,
                committed: TextBytes8::empty(),
                selection_start: 0,
                selection_end: 0,
            },
            preedit: None,
            active: false,
        }
    }

    pub fn activate(&mut self, state: TextState) -> Result<(), TextEditorError> {
        if self.active {
            return Err(TextEditorError::AlreadyActive);
        }
        self.state = state;
        self.preedit = None;
        self.active = true;
        Ok(())
    }

    pub fn apply(&mut self, event: TextInputEvent) -> Result<(), TextEditorError> {
        let mut candidate = *self;
        candidate.apply_inner(event)?;
        *self = candidate;
        Ok(())
    }

    fn apply_inner(&mut self, event: TextInputEvent) -> Result<(), TextEditorError> {
        match event.payload() {
            TextInputEventPayload::Activated { revision, .. } => {
                if self.active {
                    return Err(TextEditorError::AlreadyActive);
                }
                // The editor owns committed application text across input-method
                // sessions. Activating a newer focus/session resets only the
                // service-local revision and composition, never the document.
                self.state.revision = revision;
                self.preedit = None;
                self.active = true;
            }
            TextInputEventPayload::Preedit { revision, scalar } => {
                self.require_next_revision(revision)?;
                self.state.revision = revision;
                self.preedit = Some(scalar);
            }
            TextInputEventPayload::Commit { revision, text } => {
                self.require_next_revision(revision)?;
                self.commit_text(text, revision)?;
            }
            TextInputEventPayload::DeleteSurrounding {
                revision,
                before_scalars,
                after_scalars,
            } => {
                self.require_next_revision(revision)?;
                self.delete_text(before_scalars, after_scalars, revision)?;
            }
            TextInputEventPayload::Rendered { state, .. } => {
                self.require_active()?;
                if state != self.state {
                    return Err(TextEditorError::StateMismatch);
                }
            }
            TextInputEventPayload::Deactivated { revision, .. } => {
                self.require_active()?;
                compare_revision(self.state.revision, revision)?;
                self.preedit = None;
                self.active = false;
            }
        }
        Ok(())
    }

    fn require_active(self) -> Result<(), TextEditorError> {
        if !self.active {
            return Err(TextEditorError::Inactive);
        }
        Ok(())
    }

    fn require_next_revision(self, revision: u32) -> Result<(), TextEditorError> {
        self.require_active()?;
        let expected = self
            .state
            .revision
            .checked_add(1)
            .ok_or(TextEditorError::RevisionExhausted)?;
        if revision < expected {
            return Err(TextEditorError::RevisionReplay);
        }
        if revision > expected {
            return Err(TextEditorError::RevisionGap);
        }
        Ok(())
    }

    fn commit_text(&mut self, insert: TextBytes8, revision: u32) -> Result<(), TextEditorError> {
        let start = self.state.selection_start as usize;
        let end = self.state.selection_end as usize;
        let old_len = self.state.committed.len();
        let removed = end - start;
        let new_len = old_len - removed + insert.len();
        if new_len > TEXT_INPUT_CAPACITY {
            return Err(TextEditorError::CapacityExceeded);
        }
        let old = self.state.committed.storage();
        let mut bytes = [0_u8; TEXT_INPUT_CAPACITY];
        bytes[..start].copy_from_slice(&old[..start]);
        bytes[start..start + insert.len()].copy_from_slice(insert.as_bytes());
        bytes[start + insert.len()..new_len].copy_from_slice(&old[end..old_len]);
        let committed =
            TextBytes8::decode(bytes, new_len as u8).map_err(TextEditorError::InvalidState)?;
        let cursor = (start + insert.len()) as u8;
        self.state = TextState::from_text(revision, committed, cursor, cursor)
            .map_err(TextEditorError::InvalidState)?;
        self.preedit = None;
        Ok(())
    }

    fn delete_text(
        &mut self,
        before_scalars: u8,
        after_scalars: u8,
        revision: u32,
    ) -> Result<(), TextEditorError> {
        let text = self.state.committed.as_str();
        let mut start = self.state.selection_start as usize;
        let mut end = self.state.selection_end as usize;
        for _ in 0..before_scalars {
            start =
                previous_scalar_boundary(text, start).ok_or(TextEditorError::DeleteOutOfRange)?;
        }
        for _ in 0..after_scalars {
            end = next_scalar_boundary(text, end).ok_or(TextEditorError::DeleteOutOfRange)?;
        }
        let old = self.state.committed.storage();
        let old_len = self.state.committed.len();
        let new_len = old_len - (end - start);
        let mut bytes = [0_u8; TEXT_INPUT_CAPACITY];
        bytes[..start].copy_from_slice(&old[..start]);
        bytes[start..new_len].copy_from_slice(&old[end..old_len]);
        let committed =
            TextBytes8::decode(bytes, new_len as u8).map_err(TextEditorError::InvalidState)?;
        self.state = TextState::from_text(revision, committed, start as u8, start as u8)
            .map_err(TextEditorError::InvalidState)?;
        self.preedit = None;
        Ok(())
    }

    pub const fn state(self) -> TextState {
        self.state
    }

    pub const fn preedit(self) -> Option<char> {
        self.preedit
    }

    pub const fn is_active(self) -> bool {
        self.active
    }
}

fn compare_revision(current: u32, received: u32) -> Result<(), TextEditorError> {
    if received < current {
        return Err(TextEditorError::RevisionReplay);
    }
    if received > current {
        return Err(TextEditorError::RevisionGap);
    }
    Ok(())
}

fn previous_scalar_boundary(text: &str, offset: usize) -> Option<usize> {
    if offset == 0 || offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    text[..offset].char_indices().last().map(|(index, _)| index)
}

fn next_scalar_boundary(text: &str, offset: usize) -> Option<usize> {
    if offset >= text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    text[offset..]
        .chars()
        .next()
        .map(|scalar| offset + scalar.len_utf8())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextInputSessionPhase {
    #[default]
    Idle,
    Activating,
    Active,
    Deactivating,
    Deactivated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInputTrackerError {
    CommandSequenceReplay,
    CommandSequenceGap,
    CommandSequenceExhausted,
    EventSequenceReplay,
    EventSequenceGap,
    EventSequenceExhausted,
    AcknowledgmentBehind,
    AcknowledgmentAhead,
    OutstandingAcknowledgment,
    AcknowledgmentRequired,
    UnexpectedCommand,
    UnexpectedEvent,
    ContextMismatch,
    SessionNotIncreasing,
    RevisionReplay,
    RevisionGap,
    RevisionExhausted,
    StateMismatch,
    DeactivationTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingCommand {
    Activate { sequence: u64 },
    StateAck { sequence: u64, state: TextState },
    Deactivate { sequence: u64, revision: u32 },
}

/// Client-side BTI1/BTE1 ordering state.
///
/// Exactly one command response or edit-event state acknowledgment may be in
/// flight. Both channel sequences are contiguous, and a deactivated session is
/// terminal until a strictly newer session is explicitly activated.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextInputClientTracker {
    phase: TextInputSessionPhase,
    context: Option<TextInputContext>,
    last_session_id: u64,
    last_command_sequence: u64,
    last_event_sequence: u64,
    revision: u32,
    pending_command: Option<PendingCommand>,
    pending_event_ack: Option<u64>,
}

impl TextInputClientTracker {
    pub const fn new() -> Self {
        Self {
            phase: TextInputSessionPhase::Idle,
            context: None,
            last_session_id: 0,
            last_command_sequence: 0,
            last_event_sequence: 0,
            revision: 0,
            pending_command: None,
            pending_event_ack: None,
        }
    }

    pub fn send(&mut self, command: TextInputCommand) -> Result<(), TextInputTrackerError> {
        let mut candidate = *self;
        candidate.send_inner(command)?;
        *self = candidate;
        Ok(())
    }

    fn send_inner(&mut self, command: TextInputCommand) -> Result<(), TextInputTrackerError> {
        validate_command_sequence(self.last_command_sequence, command.sequence())?;
        match command.payload() {
            TextInputCommandPayload::Activate => {
                if !matches!(
                    self.phase,
                    TextInputSessionPhase::Idle | TextInputSessionPhase::Deactivated
                ) {
                    return Err(TextInputTrackerError::UnexpectedCommand);
                }
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                if command.context().session_id() <= self.last_session_id {
                    return Err(TextInputTrackerError::SessionNotIncreasing);
                }
                self.context = Some(command.context());
                self.last_session_id = command.context().session_id();
                self.revision = 0;
                self.phase = TextInputSessionPhase::Activating;
                self.pending_command = Some(PendingCommand::Activate {
                    sequence: command.sequence(),
                });
            }
            TextInputCommandPayload::StateAck {
                acknowledged_event_sequence,
                state,
            } => {
                self.require_active_context(command.context())?;
                if self.pending_command.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                let expected = self
                    .pending_event_ack
                    .ok_or(TextInputTrackerError::AcknowledgmentRequired)?;
                compare_acknowledgment(expected, acknowledged_event_sequence)?;
                compare_tracker_revision(self.revision, state.revision())?;
                self.pending_event_ack = None;
                self.pending_command = Some(PendingCommand::StateAck {
                    sequence: command.sequence(),
                    state,
                });
            }
            TextInputCommandPayload::Deactivate => {
                self.require_active_context(command.context())?;
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                self.phase = TextInputSessionPhase::Deactivating;
                self.pending_command = Some(PendingCommand::Deactivate {
                    sequence: command.sequence(),
                    revision: self.revision,
                });
            }
        }
        self.last_command_sequence = command.sequence();
        Ok(())
    }

    pub fn receive(&mut self, event: TextInputEvent) -> Result<(), TextInputTrackerError> {
        let mut candidate = *self;
        candidate.receive_inner(event)?;
        *self = candidate;
        Ok(())
    }

    fn receive_inner(&mut self, event: TextInputEvent) -> Result<(), TextInputTrackerError> {
        validate_event_sequence(self.last_event_sequence, event.sequence())?;
        if Some(event.context()) != self.context {
            return Err(TextInputTrackerError::ContextMismatch);
        }
        match event.payload() {
            TextInputEventPayload::Activated {
                acknowledged_command_sequence,
                revision,
            } => {
                if self.phase != TextInputSessionPhase::Activating {
                    return Err(if self.phase == TextInputSessionPhase::Deactivated {
                        TextInputTrackerError::DeactivationTerminal
                    } else {
                        TextInputTrackerError::UnexpectedEvent
                    });
                }
                let Some(PendingCommand::Activate { sequence }) = self.pending_command else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                self.pending_command = None;
                self.revision = revision;
                self.phase = TextInputSessionPhase::Active;
            }
            TextInputEventPayload::Preedit { revision, .. }
            | TextInputEventPayload::Commit { revision, .. }
            | TextInputEventPayload::DeleteSurrounding { revision, .. } => {
                if self.phase == TextInputSessionPhase::Deactivated {
                    return Err(TextInputTrackerError::DeactivationTerminal);
                }
                if self.phase != TextInputSessionPhase::Active {
                    return Err(TextInputTrackerError::UnexpectedEvent);
                }
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                require_next_tracker_revision(self.revision, revision)?;
                self.revision = revision;
                self.pending_event_ack = Some(event.sequence());
            }
            TextInputEventPayload::Rendered {
                acknowledged_command_sequence,
                state,
            } => {
                if self.phase != TextInputSessionPhase::Active {
                    return Err(TextInputTrackerError::UnexpectedEvent);
                }
                let Some(PendingCommand::StateAck {
                    sequence,
                    state: expected_state,
                }) = self.pending_command
                else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                if state != expected_state {
                    return Err(TextInputTrackerError::StateMismatch);
                }
                self.pending_command = None;
            }
            TextInputEventPayload::Deactivated {
                acknowledged_command_sequence,
                revision,
            } => {
                if self.phase != TextInputSessionPhase::Deactivating {
                    return Err(if self.phase == TextInputSessionPhase::Deactivated {
                        TextInputTrackerError::DeactivationTerminal
                    } else {
                        TextInputTrackerError::UnexpectedEvent
                    });
                }
                let Some(PendingCommand::Deactivate {
                    sequence,
                    revision: expected_revision,
                }) = self.pending_command
                else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                compare_tracker_revision(expected_revision, revision)?;
                self.pending_command = None;
                self.phase = TextInputSessionPhase::Deactivated;
            }
        }
        self.last_event_sequence = event.sequence();
        Ok(())
    }

    fn require_active_context(
        self,
        context: TextInputContext,
    ) -> Result<(), TextInputTrackerError> {
        if self.phase == TextInputSessionPhase::Deactivated {
            return Err(TextInputTrackerError::DeactivationTerminal);
        }
        if self.phase != TextInputSessionPhase::Active {
            return Err(TextInputTrackerError::UnexpectedCommand);
        }
        if Some(context) != self.context {
            return Err(TextInputTrackerError::ContextMismatch);
        }
        Ok(())
    }

    pub const fn phase(self) -> TextInputSessionPhase {
        self.phase
    }

    pub const fn context(self) -> Option<TextInputContext> {
        self.context
    }

    pub const fn revision(self) -> u32 {
        self.revision
    }

    pub const fn last_command_sequence(self) -> u64 {
        self.last_command_sequence
    }

    pub const fn last_event_sequence(self) -> u64 {
        self.last_event_sequence
    }

    pub const fn outstanding_acknowledgment(self) -> bool {
        self.pending_command.is_some() || self.pending_event_ack.is_some()
    }
}

impl Default for TextInputServerTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Server-side mirror of [`TextInputClientTracker`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputServerTracker {
    phase: TextInputSessionPhase,
    context: Option<TextInputContext>,
    last_session_id: u64,
    last_command_sequence: u64,
    last_event_sequence: u64,
    revision: u32,
    pending_command: Option<PendingCommand>,
    pending_event_ack: Option<u64>,
}

impl TextInputServerTracker {
    pub const fn new() -> Self {
        Self {
            phase: TextInputSessionPhase::Idle,
            context: None,
            last_session_id: 0,
            last_command_sequence: 0,
            last_event_sequence: 0,
            revision: 0,
            pending_command: None,
            pending_event_ack: None,
        }
    }

    pub fn receive(&mut self, command: TextInputCommand) -> Result<(), TextInputTrackerError> {
        let mut candidate = *self;
        candidate.receive_inner(command)?;
        *self = candidate;
        Ok(())
    }

    fn receive_inner(&mut self, command: TextInputCommand) -> Result<(), TextInputTrackerError> {
        validate_command_sequence(self.last_command_sequence, command.sequence())?;
        match command.payload() {
            TextInputCommandPayload::Activate => {
                if !matches!(
                    self.phase,
                    TextInputSessionPhase::Idle | TextInputSessionPhase::Deactivated
                ) {
                    return Err(TextInputTrackerError::UnexpectedCommand);
                }
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                if command.context().session_id() <= self.last_session_id {
                    return Err(TextInputTrackerError::SessionNotIncreasing);
                }
                self.context = Some(command.context());
                self.last_session_id = command.context().session_id();
                self.revision = 0;
                self.phase = TextInputSessionPhase::Activating;
                self.pending_command = Some(PendingCommand::Activate {
                    sequence: command.sequence(),
                });
            }
            TextInputCommandPayload::StateAck {
                acknowledged_event_sequence,
                state,
            } => {
                self.require_active_context(command.context())?;
                if self.pending_command.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                let expected = self
                    .pending_event_ack
                    .ok_or(TextInputTrackerError::AcknowledgmentRequired)?;
                compare_acknowledgment(expected, acknowledged_event_sequence)?;
                compare_tracker_revision(self.revision, state.revision())?;
                self.pending_event_ack = None;
                self.pending_command = Some(PendingCommand::StateAck {
                    sequence: command.sequence(),
                    state,
                });
            }
            TextInputCommandPayload::Deactivate => {
                self.require_active_context(command.context())?;
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                self.phase = TextInputSessionPhase::Deactivating;
                self.pending_command = Some(PendingCommand::Deactivate {
                    sequence: command.sequence(),
                    revision: self.revision,
                });
            }
        }
        self.last_command_sequence = command.sequence();
        Ok(())
    }

    pub fn send(&mut self, event: TextInputEvent) -> Result<(), TextInputTrackerError> {
        let mut candidate = *self;
        candidate.send_inner(event)?;
        *self = candidate;
        Ok(())
    }

    fn send_inner(&mut self, event: TextInputEvent) -> Result<(), TextInputTrackerError> {
        validate_event_sequence(self.last_event_sequence, event.sequence())?;
        if Some(event.context()) != self.context {
            return Err(TextInputTrackerError::ContextMismatch);
        }
        match event.payload() {
            TextInputEventPayload::Activated {
                acknowledged_command_sequence,
                revision,
            } => {
                if self.phase != TextInputSessionPhase::Activating {
                    return Err(TextInputTrackerError::UnexpectedEvent);
                }
                let Some(PendingCommand::Activate { sequence }) = self.pending_command else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                self.pending_command = None;
                self.revision = revision;
                self.phase = TextInputSessionPhase::Active;
            }
            TextInputEventPayload::Preedit { revision, .. }
            | TextInputEventPayload::Commit { revision, .. }
            | TextInputEventPayload::DeleteSurrounding { revision, .. } => {
                if self.phase == TextInputSessionPhase::Deactivated {
                    return Err(TextInputTrackerError::DeactivationTerminal);
                }
                if self.phase != TextInputSessionPhase::Active {
                    return Err(TextInputTrackerError::UnexpectedEvent);
                }
                if self.pending_command.is_some() || self.pending_event_ack.is_some() {
                    return Err(TextInputTrackerError::OutstandingAcknowledgment);
                }
                require_next_tracker_revision(self.revision, revision)?;
                self.revision = revision;
                self.pending_event_ack = Some(event.sequence());
            }
            TextInputEventPayload::Rendered {
                acknowledged_command_sequence,
                state,
            } => {
                if self.phase != TextInputSessionPhase::Active {
                    return Err(TextInputTrackerError::UnexpectedEvent);
                }
                let Some(PendingCommand::StateAck {
                    sequence,
                    state: expected_state,
                }) = self.pending_command
                else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                if state != expected_state {
                    return Err(TextInputTrackerError::StateMismatch);
                }
                self.pending_command = None;
            }
            TextInputEventPayload::Deactivated {
                acknowledged_command_sequence,
                revision,
            } => {
                if self.phase != TextInputSessionPhase::Deactivating {
                    return Err(if self.phase == TextInputSessionPhase::Deactivated {
                        TextInputTrackerError::DeactivationTerminal
                    } else {
                        TextInputTrackerError::UnexpectedEvent
                    });
                }
                let Some(PendingCommand::Deactivate {
                    sequence,
                    revision: expected_revision,
                }) = self.pending_command
                else {
                    return Err(TextInputTrackerError::AcknowledgmentRequired);
                };
                compare_acknowledgment(sequence, acknowledged_command_sequence)?;
                compare_tracker_revision(expected_revision, revision)?;
                self.pending_command = None;
                self.phase = TextInputSessionPhase::Deactivated;
            }
        }
        self.last_event_sequence = event.sequence();
        Ok(())
    }

    fn require_active_context(
        self,
        context: TextInputContext,
    ) -> Result<(), TextInputTrackerError> {
        if self.phase == TextInputSessionPhase::Deactivated {
            return Err(TextInputTrackerError::DeactivationTerminal);
        }
        if self.phase != TextInputSessionPhase::Active {
            return Err(TextInputTrackerError::UnexpectedCommand);
        }
        if Some(context) != self.context {
            return Err(TextInputTrackerError::ContextMismatch);
        }
        Ok(())
    }

    pub const fn phase(self) -> TextInputSessionPhase {
        self.phase
    }

    pub const fn context(self) -> Option<TextInputContext> {
        self.context
    }

    pub const fn revision(self) -> u32 {
        self.revision
    }

    pub const fn last_command_sequence(self) -> u64 {
        self.last_command_sequence
    }

    pub const fn last_event_sequence(self) -> u64 {
        self.last_event_sequence
    }

    pub const fn outstanding_acknowledgment(self) -> bool {
        self.pending_command.is_some() || self.pending_event_ack.is_some()
    }
}

fn validate_command_sequence(last: u64, sequence: u64) -> Result<(), TextInputTrackerError> {
    match validate_contiguous_sequence(last, sequence) {
        Err(SequenceOrderError::Replay) => Err(TextInputTrackerError::CommandSequenceReplay),
        Err(SequenceOrderError::Gap) => Err(TextInputTrackerError::CommandSequenceGap),
        Err(SequenceOrderError::Exhausted) => Err(TextInputTrackerError::CommandSequenceExhausted),
        Ok(()) => Ok(()),
    }
}

fn validate_event_sequence(last: u64, sequence: u64) -> Result<(), TextInputTrackerError> {
    match validate_contiguous_sequence(last, sequence) {
        Err(SequenceOrderError::Replay) => Err(TextInputTrackerError::EventSequenceReplay),
        Err(SequenceOrderError::Gap) => Err(TextInputTrackerError::EventSequenceGap),
        Err(SequenceOrderError::Exhausted) => Err(TextInputTrackerError::EventSequenceExhausted),
        Ok(()) => Ok(()),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SequenceOrderError {
    Replay,
    Gap,
    Exhausted,
}

fn validate_contiguous_sequence(last: u64, sequence: u64) -> Result<(), SequenceOrderError> {
    let expected = last.checked_add(1).ok_or(SequenceOrderError::Exhausted)?;
    if sequence < expected {
        return Err(SequenceOrderError::Replay);
    }
    if sequence > expected {
        return Err(SequenceOrderError::Gap);
    }
    Ok(())
}

fn compare_acknowledgment(expected: u64, received: u64) -> Result<(), TextInputTrackerError> {
    if received < expected {
        return Err(TextInputTrackerError::AcknowledgmentBehind);
    }
    if received > expected {
        return Err(TextInputTrackerError::AcknowledgmentAhead);
    }
    Ok(())
}

fn compare_tracker_revision(expected: u32, received: u32) -> Result<(), TextInputTrackerError> {
    if received < expected {
        return Err(TextInputTrackerError::RevisionReplay);
    }
    if received > expected {
        return Err(TextInputTrackerError::RevisionGap);
    }
    Ok(())
}

fn require_next_tracker_revision(current: u32, received: u32) -> Result<(), TextInputTrackerError> {
    let expected = current
        .checked_add(1)
        .ok_or(TextInputTrackerError::RevisionExhausted)?;
    compare_tracker_revision(expected, received)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextInputHeaderError {
    ZeroSequence,
    InvalidWindowId(WindowIdError),
    InvalidContext(TextInputContextError),
}

fn validate_text_input_sequence(sequence: u64) -> Result<(), TextInputHeaderError> {
    if sequence == 0 {
        return Err(TextInputHeaderError::ZeroSequence);
    }
    Ok(())
}

fn validate_text_selection(
    text: &str,
    selection_start: u8,
    selection_end: u8,
) -> Result<(), TextStateError> {
    let start = selection_start as usize;
    let end = selection_end as usize;
    if start > end
        || end > text.len()
        || !text.is_char_boundary(start)
        || !text.is_char_boundary(end)
    {
        return Err(TextStateError::InvalidSelection);
    }
    Ok(())
}

fn encode_text_input_header(
    wire: &mut [u8; 64],
    magic: u32,
    version: u16,
    opcode: u8,
    sequence: u64,
    context: TextInputContext,
) {
    write_u32_at(wire, TEXT_INPUT_OFFSET_MAGIC, magic);
    wire[TEXT_INPUT_OFFSET_VERSION..TEXT_INPUT_OFFSET_VERSION + 2]
        .copy_from_slice(&version.to_le_bytes());
    wire[TEXT_INPUT_OFFSET_OPCODE] = opcode;
    write_u64_at(wire, TEXT_INPUT_OFFSET_SEQUENCE, sequence);
    write_u64_at(wire, TEXT_INPUT_OFFSET_WINDOW, context.window_id().token());
    write_u64_at(wire, TEXT_INPUT_OFFSET_SESSION, context.session_id());
    write_u64_at(wire, TEXT_INPUT_OFFSET_FOCUS, context.focus_generation());
}

fn decode_text_input_context(wire: &[u8]) -> Result<TextInputContext, TextInputHeaderError> {
    validate_text_input_sequence(read_u64(wire, TEXT_INPUT_OFFSET_SEQUENCE))?;
    let window_id = WindowId::from_token(read_u64(wire, TEXT_INPUT_OFFSET_WINDOW))
        .map_err(TextInputHeaderError::InvalidWindowId)?;
    TextInputContext::try_new(
        window_id,
        read_u64(wire, TEXT_INPUT_OFFSET_SESSION),
        read_u64(wire, TEXT_INPUT_OFFSET_FOCUS),
    )
    .map_err(TextInputHeaderError::InvalidContext)
}

fn encode_text_state(wire: &mut [u8; 64], state: TextState) {
    write_u32_at(wire, TEXT_INPUT_OFFSET_REVISION, state.revision());
    wire[TEXT_INPUT_OFFSET_ARGUMENT_0] = state.selection_start();
    wire[TEXT_INPUT_OFFSET_ARGUMENT_1] = state.selection_end();
    encode_text_tail(wire, state.committed());
}

fn encode_text_tail(wire: &mut [u8; 64], text: TextBytes8) {
    wire[TEXT_INPUT_OFFSET_TEXT_LENGTH] = text.len;
    wire[TEXT_INPUT_OFFSET_TEXT..TEXT_INPUT_OFFSET_TEXT + TEXT_INPUT_CAPACITY]
        .copy_from_slice(&text.storage());
}

fn decode_text_tail(wire: &[u8]) -> Result<TextBytes8, TextStateError> {
    let mut bytes = [0_u8; TEXT_INPUT_CAPACITY];
    bytes.copy_from_slice(
        &wire[TEXT_INPUT_OFFSET_TEXT..TEXT_INPUT_OFFSET_TEXT + TEXT_INPUT_CAPACITY],
    );
    TextBytes8::decode(bytes, wire[TEXT_INPUT_OFFSET_TEXT_LENGTH])
}

fn write_u32_at(wire: &mut [u8], offset: usize, value: u32) {
    wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64_at(wire: &mut [u8], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn validate_window_command_sequence(sequence: u64) -> Result<(), WindowCommandError> {
    if sequence == 0 {
        return Err(WindowCommandError::ZeroSequence);
    }
    Ok(())
}

fn validate_window_event_common(
    command_sequence: u64,
    scene_frame: u32,
) -> Result<(), WindowEventError> {
    if command_sequence == 0 {
        return Err(WindowEventError::ZeroCommandSequence);
    }
    if scene_frame == 0 {
        return Err(WindowEventError::ZeroSceneFrame);
    }
    Ok(())
}

fn validate_window_geometry(rect: ShellRect) -> Result<(), WindowGeometryError> {
    if rect.width == 0 || rect.height == 0 {
        return Err(WindowGeometryError::Empty);
    }
    let right = rect
        .x
        .checked_add(rect.width)
        .ok_or(WindowGeometryError::OutOfBounds)?;
    let bottom = rect
        .y
        .checked_add(rect.height)
        .ok_or(WindowGeometryError::OutOfBounds)?;
    if right > SURFACE_WIDTH || bottom > SURFACE_HEIGHT {
        return Err(WindowGeometryError::OutOfBounds);
    }
    Ok(())
}

const fn window_rect_is_zero(rect: ShellRect) -> bool {
    rect.x == 0 && rect.y == 0 && rect.width == 0 && rect.height == 0
}

const fn pack_window_rect(rect: ShellRect) -> u64 {
    rect.x as u64
        | ((rect.y as u64) << 16)
        | ((rect.width as u64) << 32)
        | ((rect.height as u64) << 48)
}

const fn unpack_window_rect(packed: u64) -> ShellRect {
    ShellRect::new(
        packed as u16,
        (packed >> 16) as u16,
        (packed >> 32) as u16,
        (packed >> 48) as u16,
    )
}

fn write_window_command_u32(wire: &mut [u8; WINDOW_COMMAND_WIRE_SIZE], offset: usize, value: u32) {
    wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_window_command_u64(wire: &mut [u8; WINDOW_COMMAND_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_window_command_rect(
    wire: &mut [u8; WINDOW_COMMAND_WIRE_SIZE],
    offset: usize,
    rect: ShellRect,
) {
    for (index, value) in [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .enumerate()
    {
        let field_offset = offset + index * 2;
        wire[field_offset..field_offset + 2].copy_from_slice(&value.to_le_bytes());
    }
}

fn read_window_command_rect(wire: &[u8], offset: usize) -> ShellRect {
    ShellRect::new(
        read_u16(wire, offset),
        read_u16(wire, offset + 2),
        read_u16(wire, offset + 4),
        read_u16(wire, offset + 6),
    )
}

fn write_window_event_u64(wire: &mut [u8; WINDOW_EVENT_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

const LIFECYCLE_OFFSET_MAGIC: usize = 0;
const LIFECYCLE_OFFSET_VERSION: usize = 4;
const LIFECYCLE_OFFSET_KIND: usize = 6;
const LIFECYCLE_OFFSET_HEADER_RESERVED: usize = 7;
const LIFECYCLE_OFFSET_SENDER_SEQUENCE: usize = 8;
const LIFECYCLE_OFFSET_TRANSACTION_ID: usize = 16;
const LIFECYCLE_OFFSET_INSTANCE_ID: usize = 24;
const LIFECYCLE_OFFSET_PID: usize = 32;
const LIFECYCLE_OFFSET_PID_GENERATION: usize = 36;
const LIFECYCLE_OFFSET_APP: usize = 40;
const LIFECYCLE_OFFSET_ACTION_OR_STATE: usize = 41;
const LIFECYCLE_OFFSET_REASON_OR_STATUS: usize = 42;
const LIFECYCLE_OFFSET_BODY_RESERVED: usize = 43;

/// The four point-to-point directions in the M33a lifecycle protocol.
///
/// Each direction owns an independent, contiguous sender sequence. Keeping
/// the two Init-originated directions separate prevents a stalled App from
/// creating an artificial gap in Launcher notifications.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleDirection {
    LauncherToInit = 1,
    InitToLauncher = 2,
    InitToApp = 3,
    AppToInit = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleKind {
    Request = 1,
    StateChanged = 2,
    Command = 3,
    Ack = 4,
}

impl AppLifecycleKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Request),
            2 => Some(Self::StateChanged),
            3 => Some(Self::Command),
            4 => Some(Self::Ack),
            _ => None,
        }
    }

    pub const fn direction(self) -> AppLifecycleDirection {
        match self {
            Self::Request => AppLifecycleDirection::LauncherToInit,
            Self::StateChanged => AppLifecycleDirection::InitToLauncher,
            Self::Command => AppLifecycleDirection::InitToApp,
            Self::Ack => AppLifecycleDirection::AppToInit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleAction {
    Launch = 1,
    Activate = 2,
    Suspend = 3,
    Resume = 4,
    Terminate = 5,
}

impl AppLifecycleAction {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Launch),
            2 => Some(Self::Activate),
            3 => Some(Self::Suspend),
            4 => Some(Self::Resume),
            5 => Some(Self::Terminate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleState {
    NotRunning = 1,
    Launching = 2,
    Inactive = 3,
    Activating = 4,
    Active = 5,
    Suspending = 6,
    Suspended = 7,
    Resuming = 8,
    Terminating = 9,
    Crashed = 10,
    Failed = 11,
}

impl AppLifecycleState {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::NotRunning),
            2 => Some(Self::Launching),
            3 => Some(Self::Inactive),
            4 => Some(Self::Activating),
            5 => Some(Self::Active),
            6 => Some(Self::Suspending),
            7 => Some(Self::Suspended),
            8 => Some(Self::Resuming),
            9 => Some(Self::Terminating),
            10 => Some(Self::Crashed),
            11 => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleReason {
    Requested = 1,
    Completed = 2,
    ProcessExited = 3,
    SpawnFailed = 4,
    CommandRejected = 5,
    CommandFailed = 6,
    ProtocolViolation = 7,
}

impl AppLifecycleReason {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Requested),
            2 => Some(Self::Completed),
            3 => Some(Self::ProcessExited),
            4 => Some(Self::SpawnFailed),
            5 => Some(Self::CommandRejected),
            6 => Some(Self::CommandFailed),
            7 => Some(Self::ProtocolViolation),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppLifecycleStatus {
    Applied = 1,
    Rejected = 2,
    Failed = 3,
}

impl AppLifecycleStatus {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Applied),
            2 => Some(Self::Rejected),
            3 => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppIdentityError {
    ZeroPid,
    ZeroPidGeneration,
    ZeroInstanceId,
}

/// A PID is meaningful across process-table slot reuse only together with its
/// nonzero generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationQualifiedPid {
    pid: u32,
    generation: u32,
}

impl GenerationQualifiedPid {
    pub fn try_new(pid: u32, generation: u32) -> Result<Self, AppIdentityError> {
        if pid == 0 {
            return Err(AppIdentityError::ZeroPid);
        }
        if generation == 0 {
            return Err(AppIdentityError::ZeroPidGeneration);
        }
        Ok(Self { pid, generation })
    }

    pub const fn pid(self) -> u32 {
        self.pid
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// Stable identity of one launched App instance. Both components are
/// nonzero, making the all-zero wire triple the sole canonical `None` value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppInstanceIdentity {
    instance_id: u64,
    process: GenerationQualifiedPid,
}

impl AppInstanceIdentity {
    pub fn try_new(
        instance_id: u64,
        pid: u32,
        pid_generation: u32,
    ) -> Result<Self, AppIdentityError> {
        if instance_id == 0 {
            return Err(AppIdentityError::ZeroInstanceId);
        }
        Ok(Self {
            instance_id,
            process: GenerationQualifiedPid::try_new(pid, pid_generation)?,
        })
    }

    pub const fn instance_id(self) -> u64 {
        self.instance_id
    }

    pub const fn process(self) -> GenerationQualifiedPid {
        self.process
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecycleWireError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSenderSequence,
    ZeroTransactionId,
    InvalidApp,
    InvalidAction,
    InvalidState,
    InvalidReason,
    InvalidStatus,
    IncompleteIdentity,
    LaunchRequestHasIdentity,
    RequestRequiresIdentity,
    CommandRequiresIdentity,
    AckRequiresIdentity,
    StateRequiresIdentity,
    SpawnFailureHasIdentity,
    InvalidStateReason,
    UnexpectedReasonOrStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecyclePayload {
    /// Launcher -> Init. Launch is the only request without an existing
    /// instance identity.
    Request {
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: Option<AppInstanceIdentity>,
    },
    /// Init -> Launcher. The action byte is interpreted as a state and the
    /// following byte as a state-change reason.
    StateChanged {
        app: ShellAppId,
        state: AppLifecycleState,
        reason: AppLifecycleReason,
        identity: Option<AppInstanceIdentity>,
    },
    /// Init -> App.
    Command {
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: AppInstanceIdentity,
    },
    /// App -> Init.
    Ack {
        app: ShellAppId,
        action: AppLifecycleAction,
        status: AppLifecycleStatus,
        identity: AppInstanceIdentity,
    },
}

/// One canonical fixed-size M33a lifecycle message.
///
/// The common header is followed by sender sequence, transaction, instance,
/// generation-qualified PID, app, action/state and reason/status. Byte 7 and
/// bytes 43..64 are reserved zero. Every message carries nonzero sender and
/// transaction identifiers; only a Launch request and a pre-spawn failure use
/// the all-zero identity triple.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLifecycleMessage {
    sender_sequence: u64,
    transaction_id: u64,
    payload: AppLifecyclePayload,
}

impl AppLifecycleMessage {
    pub fn request(
        sender_sequence: u64,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<Self, AppLifecycleWireError> {
        validate_lifecycle_counters(sender_sequence, transaction_id)?;
        match (action, identity) {
            (AppLifecycleAction::Launch, Some(_)) => {
                return Err(AppLifecycleWireError::LaunchRequestHasIdentity);
            }
            (AppLifecycleAction::Launch, None) => {}
            (_, None) => return Err(AppLifecycleWireError::RequestRequiresIdentity),
            (_, Some(_)) => {}
        }
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: AppLifecyclePayload::Request {
                app,
                action,
                identity,
            },
        })
    }

    pub fn state_changed(
        sender_sequence: u64,
        transaction_id: u64,
        app: ShellAppId,
        state: AppLifecycleState,
        reason: AppLifecycleReason,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<Self, AppLifecycleWireError> {
        validate_lifecycle_counters(sender_sequence, transaction_id)?;
        if !valid_state_reason(state, reason) {
            return Err(AppLifecycleWireError::InvalidStateReason);
        }
        if state == AppLifecycleState::Failed && reason == AppLifecycleReason::SpawnFailed {
            if identity.is_some() {
                return Err(AppLifecycleWireError::SpawnFailureHasIdentity);
            }
        } else if identity.is_none() {
            return Err(AppLifecycleWireError::StateRequiresIdentity);
        }
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: AppLifecyclePayload::StateChanged {
                app,
                state,
                reason,
                identity,
            },
        })
    }

    pub fn command(
        sender_sequence: u64,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: AppInstanceIdentity,
    ) -> Result<Self, AppLifecycleWireError> {
        validate_lifecycle_counters(sender_sequence, transaction_id)?;
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: AppLifecyclePayload::Command {
                app,
                action,
                identity,
            },
        })
    }

    pub fn ack(
        sender_sequence: u64,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        status: AppLifecycleStatus,
        identity: AppInstanceIdentity,
    ) -> Result<Self, AppLifecycleWireError> {
        validate_lifecycle_counters(sender_sequence, transaction_id)?;
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: AppLifecyclePayload::Ack {
                app,
                action,
                status,
                identity,
            },
        })
    }

    pub const fn sender_sequence(self) -> u64 {
        self.sender_sequence
    }

    pub const fn transaction_id(self) -> u64 {
        self.transaction_id
    }

    pub const fn payload(self) -> AppLifecyclePayload {
        self.payload
    }

    pub const fn kind(self) -> AppLifecycleKind {
        match self.payload {
            AppLifecyclePayload::Request { .. } => AppLifecycleKind::Request,
            AppLifecyclePayload::StateChanged { .. } => AppLifecycleKind::StateChanged,
            AppLifecyclePayload::Command { .. } => AppLifecycleKind::Command,
            AppLifecyclePayload::Ack { .. } => AppLifecycleKind::Ack,
        }
    }

    pub const fn direction(self) -> AppLifecycleDirection {
        self.kind().direction()
    }

    pub fn encode(self) -> [u8; APP_LIFECYCLE_WIRE_SIZE] {
        let mut wire = [0_u8; APP_LIFECYCLE_WIRE_SIZE];
        wire[LIFECYCLE_OFFSET_MAGIC..LIFECYCLE_OFFSET_MAGIC + 4]
            .copy_from_slice(&APP_LIFECYCLE_MAGIC.to_le_bytes());
        wire[LIFECYCLE_OFFSET_VERSION..LIFECYCLE_OFFSET_VERSION + 2]
            .copy_from_slice(&APP_LIFECYCLE_VERSION.to_le_bytes());
        wire[LIFECYCLE_OFFSET_KIND] = self.kind().raw();
        write_lifecycle_u64(
            &mut wire,
            LIFECYCLE_OFFSET_SENDER_SEQUENCE,
            self.sender_sequence,
        );
        write_lifecycle_u64(
            &mut wire,
            LIFECYCLE_OFFSET_TRANSACTION_ID,
            self.transaction_id,
        );
        let (app, action_or_state, reason_or_status, identity) = match self.payload {
            AppLifecyclePayload::Request {
                app,
                action,
                identity,
            } => (app, action.raw(), 0, identity),
            AppLifecyclePayload::Command {
                app,
                action,
                identity,
            } => (app, action.raw(), 0, Some(identity)),
            AppLifecyclePayload::StateChanged {
                app,
                state,
                reason,
                identity,
            } => (app, state.raw(), reason.raw(), identity),
            AppLifecyclePayload::Ack {
                app,
                action,
                status,
                identity,
            } => (app, action.raw(), status.raw(), Some(identity)),
        };
        wire[LIFECYCLE_OFFSET_APP] = app.raw();
        wire[LIFECYCLE_OFFSET_ACTION_OR_STATE] = action_or_state;
        wire[LIFECYCLE_OFFSET_REASON_OR_STATUS] = reason_or_status;
        if let Some(identity) = identity {
            write_lifecycle_u64(
                &mut wire,
                LIFECYCLE_OFFSET_INSTANCE_ID,
                identity.instance_id(),
            );
            wire[LIFECYCLE_OFFSET_PID..LIFECYCLE_OFFSET_PID + 4]
                .copy_from_slice(&identity.process().pid().to_le_bytes());
            wire[LIFECYCLE_OFFSET_PID_GENERATION..LIFECYCLE_OFFSET_PID_GENERATION + 4]
                .copy_from_slice(&identity.process().generation().to_le_bytes());
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, AppLifecycleWireError> {
        if wire.len() != APP_LIFECYCLE_WIRE_SIZE {
            return Err(AppLifecycleWireError::InvalidWireLength);
        }
        if read_u32(wire, LIFECYCLE_OFFSET_MAGIC) != APP_LIFECYCLE_MAGIC {
            return Err(AppLifecycleWireError::InvalidMagic);
        }
        if read_u16(wire, LIFECYCLE_OFFSET_VERSION) != APP_LIFECYCLE_VERSION {
            return Err(AppLifecycleWireError::InvalidVersion);
        }
        let kind = AppLifecycleKind::from_raw(wire[LIFECYCLE_OFFSET_KIND])
            .ok_or(AppLifecycleWireError::InvalidKind)?;
        if wire[LIFECYCLE_OFFSET_HEADER_RESERVED] != 0 {
            return Err(AppLifecycleWireError::NonZeroHeaderReserved);
        }
        if wire[LIFECYCLE_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(AppLifecycleWireError::NonZeroBodyReserved);
        }
        let sender_sequence = read_u64(wire, LIFECYCLE_OFFSET_SENDER_SEQUENCE);
        let transaction_id = read_u64(wire, LIFECYCLE_OFFSET_TRANSACTION_ID);
        validate_lifecycle_counters(sender_sequence, transaction_id)?;
        let app = ShellAppId::from_raw(u64::from(wire[LIFECYCLE_OFFSET_APP]))
            .ok_or(AppLifecycleWireError::InvalidApp)?;
        let identity = decode_lifecycle_identity(wire)?;
        let action_or_state = wire[LIFECYCLE_OFFSET_ACTION_OR_STATE];
        let reason_or_status = wire[LIFECYCLE_OFFSET_REASON_OR_STATUS];
        match kind {
            AppLifecycleKind::Request => {
                if reason_or_status != 0 {
                    return Err(AppLifecycleWireError::UnexpectedReasonOrStatus);
                }
                let action = AppLifecycleAction::from_raw(action_or_state)
                    .ok_or(AppLifecycleWireError::InvalidAction)?;
                Self::request(sender_sequence, transaction_id, app, action, identity)
            }
            AppLifecycleKind::StateChanged => {
                let state = AppLifecycleState::from_raw(action_or_state)
                    .ok_or(AppLifecycleWireError::InvalidState)?;
                let reason = AppLifecycleReason::from_raw(reason_or_status)
                    .ok_or(AppLifecycleWireError::InvalidReason)?;
                Self::state_changed(
                    sender_sequence,
                    transaction_id,
                    app,
                    state,
                    reason,
                    identity,
                )
            }
            AppLifecycleKind::Command => {
                if reason_or_status != 0 {
                    return Err(AppLifecycleWireError::UnexpectedReasonOrStatus);
                }
                let action = AppLifecycleAction::from_raw(action_or_state)
                    .ok_or(AppLifecycleWireError::InvalidAction)?;
                let identity = identity.ok_or(AppLifecycleWireError::CommandRequiresIdentity)?;
                Self::command(sender_sequence, transaction_id, app, action, identity)
            }
            AppLifecycleKind::Ack => {
                let action = AppLifecycleAction::from_raw(action_or_state)
                    .ok_or(AppLifecycleWireError::InvalidAction)?;
                let status = AppLifecycleStatus::from_raw(reason_or_status)
                    .ok_or(AppLifecycleWireError::InvalidStatus)?;
                let identity = identity.ok_or(AppLifecycleWireError::AckRequiresIdentity)?;
                Self::ack(
                    sender_sequence,
                    transaction_id,
                    app,
                    action,
                    status,
                    identity,
                )
            }
        }
    }
}

fn validate_lifecycle_counters(
    sender_sequence: u64,
    transaction_id: u64,
) -> Result<(), AppLifecycleWireError> {
    if sender_sequence == 0 {
        return Err(AppLifecycleWireError::ZeroSenderSequence);
    }
    if transaction_id == 0 {
        return Err(AppLifecycleWireError::ZeroTransactionId);
    }
    Ok(())
}

const fn valid_state_reason(state: AppLifecycleState, reason: AppLifecycleReason) -> bool {
    match state {
        AppLifecycleState::NotRunning
        | AppLifecycleState::Inactive
        | AppLifecycleState::Active
        | AppLifecycleState::Suspended => matches!(reason, AppLifecycleReason::Completed),
        AppLifecycleState::Launching
        | AppLifecycleState::Activating
        | AppLifecycleState::Suspending
        | AppLifecycleState::Resuming
        | AppLifecycleState::Terminating => matches!(reason, AppLifecycleReason::Requested),
        AppLifecycleState::Crashed => matches!(reason, AppLifecycleReason::ProcessExited),
        AppLifecycleState::Failed => matches!(
            reason,
            AppLifecycleReason::SpawnFailed
                | AppLifecycleReason::CommandRejected
                | AppLifecycleReason::CommandFailed
                | AppLifecycleReason::ProtocolViolation
        ),
    }
}

fn decode_lifecycle_identity(
    wire: &[u8],
) -> Result<Option<AppInstanceIdentity>, AppLifecycleWireError> {
    let instance_id = read_u64(wire, LIFECYCLE_OFFSET_INSTANCE_ID);
    let pid = read_u32(wire, LIFECYCLE_OFFSET_PID);
    let pid_generation = read_u32(wire, LIFECYCLE_OFFSET_PID_GENERATION);
    match (instance_id, pid, pid_generation) {
        (0, 0, 0) => Ok(None),
        (0, _, _) | (_, 0, _) | (_, _, 0) => Err(AppLifecycleWireError::IncompleteIdentity),
        _ => Ok(Some(
            AppInstanceIdentity::try_new(instance_id, pid, pid_generation)
                .map_err(|_| AppLifecycleWireError::IncompleteIdentity)?,
        )),
    }
}

fn write_lifecycle_u64(wire: &mut [u8; APP_LIFECYCLE_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecycleSequenceError {
    SequenceExhausted,
    SequenceReplay,
    SequenceGap,
}

/// Independent contiguous receive cursors for the four protocol directions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppLifecycleSequenceTracker {
    launcher_to_init: Option<u64>,
    init_to_launcher: Option<u64>,
    init_to_app: Option<u64>,
    app_to_init: Option<u64>,
    app_endpoint_identity: Option<AppInstanceIdentity>,
}

impl AppLifecycleSequenceTracker {
    pub const fn new() -> Self {
        Self {
            launcher_to_init: None,
            init_to_launcher: None,
            init_to_app: None,
            app_to_init: None,
            app_endpoint_identity: None,
        }
    }

    /// Binds the two App endpoint directions to one generation-qualified
    /// instance. A fresh instance starts both directions at sequence one;
    /// rebinding the same endpoint preserves its receive cursors.
    pub fn bind_app_endpoint(&mut self, identity: AppInstanceIdentity) {
        if self.app_endpoint_identity == Some(identity) {
            return;
        }
        self.app_endpoint_identity = Some(identity);
        self.init_to_app = None;
        self.app_to_init = None;
    }

    pub fn accept(
        &mut self,
        message: AppLifecycleMessage,
    ) -> Result<(), AppLifecycleSequenceError> {
        let cursor = match message.direction() {
            AppLifecycleDirection::LauncherToInit => &mut self.launcher_to_init,
            AppLifecycleDirection::InitToLauncher => &mut self.init_to_launcher,
            AppLifecycleDirection::InitToApp => &mut self.init_to_app,
            AppLifecycleDirection::AppToInit => &mut self.app_to_init,
        };
        let expected = match *cursor {
            Some(previous) => previous
                .checked_add(1)
                .ok_or(AppLifecycleSequenceError::SequenceExhausted)?,
            None => 1,
        };
        if message.sender_sequence() < expected {
            return Err(AppLifecycleSequenceError::SequenceReplay);
        }
        if message.sender_sequence() > expected {
            return Err(AppLifecycleSequenceError::SequenceGap);
        }
        *cursor = Some(message.sender_sequence());
        Ok(())
    }

    pub const fn last(self, direction: AppLifecycleDirection) -> Option<u64> {
        match direction {
            AppLifecycleDirection::LauncherToInit => self.launcher_to_init,
            AppLifecycleDirection::InitToLauncher => self.init_to_launcher,
            AppLifecycleDirection::InitToApp => self.init_to_app,
            AppLifecycleDirection::AppToInit => self.app_to_init,
        }
    }

    pub const fn app_endpoint_identity(self) -> Option<AppInstanceIdentity> {
        self.app_endpoint_identity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecycleTransactionError {
    TransactionExhausted,
    TransactionReplay,
    TransactionBusy,
    NoPendingTransaction,
    TransactionMismatch,
    AppMismatch,
    ActionMismatch,
    NoRunningInstance,
    AppAlreadyRunning,
    StaleIdentity,
    IntermediateStateRequired,
    IntermediateStateReplay,
    InvalidStateForAction,
    CommandReplay,
    AckBeforeCommand,
    AckReplay,
    CompletionBeforeAck,
    AckStatusMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingLifecycleTransaction {
    transaction_id: u64,
    app: ShellAppId,
    action: AppLifecycleAction,
    identity: Option<AppInstanceIdentity>,
    intermediate_seen: bool,
    command_seen: bool,
    ack_status: Option<AppLifecycleStatus>,
}

/// Correlates Launcher requests, Init commands/state changes and App acks.
///
/// The tracker deliberately holds one outstanding transaction: the M33a
/// shell controls one foreground App process. Launch binds the new identity
/// on the first `Launching` notification; every later leg must repeat it
/// exactly. Terminal state changes retire the transaction, while spontaneous
/// crash notifications allocate their own strictly newer transaction id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLifecycleTransactionTracker {
    pending: Option<PendingLifecycleTransaction>,
    last_transaction_id: Option<u64>,
    running_app: Option<ShellAppId>,
    running_identity: Option<AppInstanceIdentity>,
    state: AppLifecycleState,
}

impl Default for AppLifecycleTransactionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AppLifecycleTransactionTracker {
    pub const fn new() -> Self {
        Self {
            pending: None,
            last_transaction_id: None,
            running_app: None,
            running_identity: None,
            state: AppLifecycleState::NotRunning,
        }
    }

    /// Accepts one message transactionally; a rejected leg changes no state.
    pub fn accept(
        &mut self,
        message: AppLifecycleMessage,
    ) -> Result<(), AppLifecycleTransactionError> {
        let mut next = *self;
        next.accept_inner(message)?;
        *self = next;
        Ok(())
    }

    fn accept_inner(
        &mut self,
        message: AppLifecycleMessage,
    ) -> Result<(), AppLifecycleTransactionError> {
        match message.payload() {
            AppLifecyclePayload::Request {
                app,
                action,
                identity,
            } => self.accept_request(message.transaction_id(), app, action, identity),
            AppLifecyclePayload::Command {
                app,
                action,
                identity,
            } => self.accept_command(message.transaction_id(), app, action, identity),
            AppLifecyclePayload::Ack {
                app,
                action,
                status,
                identity,
            } => self.accept_ack(message.transaction_id(), app, action, status, identity),
            AppLifecyclePayload::StateChanged {
                app,
                state,
                reason,
                identity,
            } => self.accept_state_changed(message.transaction_id(), app, state, reason, identity),
        }
    }

    fn accept_request(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleTransactionError> {
        if self.pending.is_some() {
            return Err(AppLifecycleTransactionError::TransactionBusy);
        }
        self.require_new_transaction(transaction_id)?;
        match action {
            AppLifecycleAction::Launch => {
                if !matches!(
                    self.state,
                    AppLifecycleState::NotRunning
                        | AppLifecycleState::Crashed
                        | AppLifecycleState::Failed
                ) {
                    return Err(AppLifecycleTransactionError::AppAlreadyRunning);
                }
            }
            _ => {
                if !action_allowed_from(self.state, action) {
                    return Err(AppLifecycleTransactionError::InvalidStateForAction);
                }
                let running_identity = self
                    .running_identity
                    .ok_or(AppLifecycleTransactionError::NoRunningInstance)?;
                if self.running_app != Some(app) {
                    return Err(AppLifecycleTransactionError::AppMismatch);
                }
                if identity != Some(running_identity) {
                    return Err(AppLifecycleTransactionError::StaleIdentity);
                }
            }
        }
        self.pending = Some(PendingLifecycleTransaction {
            transaction_id,
            app,
            action,
            identity,
            intermediate_seen: false,
            command_seen: false,
            ack_status: None,
        });
        Ok(())
    }

    fn accept_command(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        identity: AppInstanceIdentity,
    ) -> Result<(), AppLifecycleTransactionError> {
        let pending = self.pending_mut(transaction_id, app, action)?;
        if !pending.intermediate_seen {
            return Err(AppLifecycleTransactionError::IntermediateStateRequired);
        }
        if pending.command_seen {
            return Err(AppLifecycleTransactionError::CommandReplay);
        }
        match pending.identity {
            Some(expected) if expected != identity => {
                return Err(AppLifecycleTransactionError::StaleIdentity);
            }
            None => pending.identity = Some(identity),
            Some(_) => {}
        }
        pending.command_seen = true;
        Ok(())
    }

    fn accept_ack(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
        status: AppLifecycleStatus,
        identity: AppInstanceIdentity,
    ) -> Result<(), AppLifecycleTransactionError> {
        let pending = self.pending_mut(transaction_id, app, action)?;
        if !pending.command_seen {
            return Err(AppLifecycleTransactionError::AckBeforeCommand);
        }
        if pending.ack_status.is_some() {
            return Err(AppLifecycleTransactionError::AckReplay);
        }
        if pending.identity != Some(identity) {
            return Err(AppLifecycleTransactionError::StaleIdentity);
        }
        pending.ack_status = Some(status);
        Ok(())
    }

    fn accept_state_changed(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        state: AppLifecycleState,
        reason: AppLifecycleReason,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleTransactionError> {
        if state == AppLifecycleState::Crashed {
            return self.accept_crash(transaction_id, app, identity);
        }
        let Some(pending) = self.pending else {
            return Err(AppLifecycleTransactionError::NoPendingTransaction);
        };
        if pending.transaction_id != transaction_id {
            return Err(AppLifecycleTransactionError::TransactionMismatch);
        }
        if pending.app != app {
            return Err(AppLifecycleTransactionError::AppMismatch);
        }
        let intermediate = intermediate_state(pending.action);
        if state == intermediate {
            if reason != AppLifecycleReason::Requested {
                return Err(AppLifecycleTransactionError::InvalidStateForAction);
            }
            if pending.intermediate_seen {
                return Err(AppLifecycleTransactionError::IntermediateStateReplay);
            }
            self.bind_pending_identity(identity)?;
            self.pending
                .as_mut()
                .expect("pending checked above")
                .intermediate_seen = true;
            return Ok(());
        }
        if state == success_state(pending.action) {
            if reason != AppLifecycleReason::Completed {
                return Err(AppLifecycleTransactionError::InvalidStateForAction);
            }
            if !pending.intermediate_seen {
                return Err(AppLifecycleTransactionError::IntermediateStateRequired);
            }
            if pending.ack_status != Some(AppLifecycleStatus::Applied) {
                return Err(AppLifecycleTransactionError::CompletionBeforeAck);
            }
            self.bind_pending_identity(identity)?;
            self.complete_success(pending);
            return Ok(());
        }
        if state == AppLifecycleState::Failed {
            self.complete_failure(pending, reason, identity)?;
            return Ok(());
        }
        Err(AppLifecycleTransactionError::InvalidStateForAction)
    }

    fn accept_crash(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleTransactionError> {
        let identity = identity.ok_or(AppLifecycleTransactionError::StaleIdentity)?;
        if let Some(pending) = self.pending {
            if pending.transaction_id != transaction_id {
                return Err(AppLifecycleTransactionError::TransactionMismatch);
            }
            if pending.app != app {
                return Err(AppLifecycleTransactionError::AppMismatch);
            }
            if let Some(expected) = pending.identity {
                if expected != identity {
                    return Err(AppLifecycleTransactionError::StaleIdentity);
                }
            } else if pending.action == AppLifecycleAction::Launch {
                if self.running_identity == Some(identity) {
                    return Err(AppLifecycleTransactionError::StaleIdentity);
                }
            } else if self.running_identity != Some(identity) {
                return Err(AppLifecycleTransactionError::StaleIdentity);
            }
            self.running_app = Some(app);
            self.running_identity = Some(identity);
            self.pending = None;
        } else {
            self.require_new_transaction(transaction_id)?;
            if matches!(
                self.state,
                AppLifecycleState::NotRunning
                    | AppLifecycleState::Crashed
                    | AppLifecycleState::Failed
            ) {
                return Err(AppLifecycleTransactionError::InvalidStateForAction);
            }
            if self.running_app != Some(app) {
                return Err(AppLifecycleTransactionError::AppMismatch);
            }
            if self.running_identity != Some(identity) {
                return Err(AppLifecycleTransactionError::StaleIdentity);
            }
        }
        self.last_transaction_id = Some(transaction_id);
        self.state = AppLifecycleState::Crashed;
        Ok(())
    }

    fn complete_failure(
        &mut self,
        pending: PendingLifecycleTransaction,
        reason: AppLifecycleReason,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleTransactionError> {
        match reason {
            AppLifecycleReason::SpawnFailed
                if pending.action == AppLifecycleAction::Launch
                    && !pending.intermediate_seen
                    && !pending.command_seen
                    && pending.ack_status.is_none()
                    && identity.is_none() => {}
            AppLifecycleReason::CommandRejected
                if pending.ack_status == Some(AppLifecycleStatus::Rejected) =>
            {
                self.bind_pending_identity(identity)?;
            }
            AppLifecycleReason::CommandFailed | AppLifecycleReason::ProtocolViolation
                if pending.ack_status == Some(AppLifecycleStatus::Failed) =>
            {
                self.bind_pending_identity(identity)?;
            }
            _ => return Err(AppLifecycleTransactionError::AckStatusMismatch),
        }
        let failed_identity = self.pending.and_then(|current| current.identity);
        self.pending = None;
        self.last_transaction_id = Some(pending.transaction_id);
        if let (AppLifecycleAction::Launch, Some(identity)) = (pending.action, failed_identity) {
            self.running_app = Some(pending.app);
            self.running_identity = Some(identity);
        }
        self.state = AppLifecycleState::Failed;
        Ok(())
    }

    fn complete_success(&mut self, pending: PendingLifecycleTransaction) {
        let identity = self
            .pending
            .and_then(|current| current.identity)
            .expect("successful transaction has an acknowledged identity");
        self.pending = None;
        self.last_transaction_id = Some(pending.transaction_id);
        self.state = success_state(pending.action);
        match pending.action {
            AppLifecycleAction::Launch => {
                self.running_app = Some(pending.app);
                self.running_identity = Some(identity);
            }
            AppLifecycleAction::Terminate => {
                self.running_app = None;
                self.running_identity = None;
            }
            _ => {}
        }
    }

    fn bind_pending_identity(
        &mut self,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleTransactionError> {
        let previous_identity = self.running_identity;
        let pending = self
            .pending
            .as_mut()
            .ok_or(AppLifecycleTransactionError::NoPendingTransaction)?;
        let identity = identity.ok_or(AppLifecycleTransactionError::StaleIdentity)?;
        if pending.action == AppLifecycleAction::Launch
            && let Some(previous) = previous_identity
            && (previous.instance_id() == identity.instance_id()
                || previous.process() == identity.process())
        {
            return Err(AppLifecycleTransactionError::StaleIdentity);
        }
        match pending.identity {
            Some(expected) if expected != identity => {
                Err(AppLifecycleTransactionError::StaleIdentity)
            }
            None => {
                pending.identity = Some(identity);
                Ok(())
            }
            Some(_) => Ok(()),
        }
    }

    fn pending_mut(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        action: AppLifecycleAction,
    ) -> Result<&mut PendingLifecycleTransaction, AppLifecycleTransactionError> {
        let pending = self
            .pending
            .as_mut()
            .ok_or(AppLifecycleTransactionError::NoPendingTransaction)?;
        if pending.transaction_id != transaction_id {
            return Err(AppLifecycleTransactionError::TransactionMismatch);
        }
        if pending.app != app {
            return Err(AppLifecycleTransactionError::AppMismatch);
        }
        if pending.action != action {
            return Err(AppLifecycleTransactionError::ActionMismatch);
        }
        Ok(pending)
    }

    fn require_new_transaction(
        self,
        transaction_id: u64,
    ) -> Result<(), AppLifecycleTransactionError> {
        if let Some(previous) = self.last_transaction_id {
            if previous == u64::MAX {
                return Err(AppLifecycleTransactionError::TransactionExhausted);
            }
            if transaction_id <= previous {
                return Err(AppLifecycleTransactionError::TransactionReplay);
            }
        }
        Ok(())
    }

    pub const fn pending_transaction_id(self) -> Option<u64> {
        match self.pending {
            Some(pending) => Some(pending.transaction_id),
            None => None,
        }
    }

    pub const fn last_transaction_id(self) -> Option<u64> {
        self.last_transaction_id
    }

    pub const fn running_app(self) -> Option<ShellAppId> {
        self.running_app
    }

    pub const fn running_identity(self) -> Option<AppInstanceIdentity> {
        self.running_identity
    }

    pub const fn state(self) -> AppLifecycleState {
        self.state
    }
}

const fn intermediate_state(action: AppLifecycleAction) -> AppLifecycleState {
    match action {
        AppLifecycleAction::Launch => AppLifecycleState::Launching,
        AppLifecycleAction::Activate => AppLifecycleState::Activating,
        AppLifecycleAction::Suspend => AppLifecycleState::Suspending,
        AppLifecycleAction::Resume => AppLifecycleState::Resuming,
        AppLifecycleAction::Terminate => AppLifecycleState::Terminating,
    }
}

const fn success_state(action: AppLifecycleAction) -> AppLifecycleState {
    match action {
        AppLifecycleAction::Launch => AppLifecycleState::Inactive,
        AppLifecycleAction::Activate | AppLifecycleAction::Resume => AppLifecycleState::Active,
        AppLifecycleAction::Suspend => AppLifecycleState::Suspended,
        AppLifecycleAction::Terminate => AppLifecycleState::NotRunning,
    }
}

const fn action_allowed_from(state: AppLifecycleState, action: AppLifecycleAction) -> bool {
    matches!(
        (state, action),
        (
            AppLifecycleState::NotRunning | AppLifecycleState::Crashed | AppLifecycleState::Failed,
            AppLifecycleAction::Launch
        ) | (
            AppLifecycleState::Inactive,
            AppLifecycleAction::Activate | AppLifecycleAction::Terminate
        ) | (
            AppLifecycleState::Active,
            AppLifecycleAction::Suspend | AppLifecycleAction::Terminate
        ) | (
            AppLifecycleState::Suspended,
            AppLifecycleAction::Resume | AppLifecycleAction::Terminate
        )
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecycleTrackerError {
    Sequence(AppLifecycleSequenceError),
    Transaction(AppLifecycleTransactionError),
}

/// Atomically combines direction ordering and transaction correlation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppLifecycleTracker {
    sequence: AppLifecycleSequenceTracker,
    transaction: AppLifecycleTransactionTracker,
}

impl AppLifecycleTracker {
    pub const fn new() -> Self {
        Self {
            sequence: AppLifecycleSequenceTracker::new(),
            transaction: AppLifecycleTransactionTracker::new(),
        }
    }

    pub fn accept(&mut self, message: AppLifecycleMessage) -> Result<(), AppLifecycleTrackerError> {
        let mut next = *self;
        if let AppLifecyclePayload::StateChanged {
            state: AppLifecycleState::Launching,
            identity: Some(identity),
            ..
        } = message.payload()
        {
            next.sequence.bind_app_endpoint(identity);
        }
        next.sequence
            .accept(message)
            .map_err(AppLifecycleTrackerError::Sequence)?;
        next.transaction
            .accept(message)
            .map_err(AppLifecycleTrackerError::Transaction)?;
        *self = next;
        Ok(())
    }

    pub const fn sequence(self) -> AppLifecycleSequenceTracker {
        self.sequence
    }

    pub const fn transaction(self) -> AppLifecycleTransactionTracker {
        self.transaction
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLifecycleMachineError {
    TransactionExhausted,
    TransactionReplay,
    TransactionBusy,
    NoActiveTransaction,
    TransactionMismatch,
    InvalidTransition,
    MissingIdentity,
    UnexpectedIdentity,
    IdentityAlreadyBound,
    StaleIdentity,
}

/// Pure lifecycle state machine for one shell application.
///
/// It owns no kernel objects and performs no IPC. Callers can therefore apply
/// a decoded/validated transaction to a copy and commit only after their
/// external operations succeed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLifecycleStateMachine {
    app: ShellAppId,
    state: AppLifecycleState,
    identity: Option<AppInstanceIdentity>,
    retired_identity: Option<AppInstanceIdentity>,
    active_transaction_id: Option<u64>,
    last_transaction_id: Option<u64>,
}

impl AppLifecycleStateMachine {
    pub const fn new(app: ShellAppId) -> Self {
        Self {
            app,
            state: AppLifecycleState::NotRunning,
            identity: None,
            retired_identity: None,
            active_transaction_id: None,
            last_transaction_id: None,
        }
    }

    pub fn begin(
        &mut self,
        transaction_id: u64,
        action: AppLifecycleAction,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<AppLifecycleState, AppLifecycleMachineError> {
        if self.active_transaction_id.is_some() {
            return Err(AppLifecycleMachineError::TransactionBusy);
        }
        self.require_new_transaction(transaction_id)?;
        let next_state = match (self.state, action) {
            (
                AppLifecycleState::NotRunning
                | AppLifecycleState::Crashed
                | AppLifecycleState::Failed,
                AppLifecycleAction::Launch,
            ) => {
                if identity.is_some() {
                    return Err(AppLifecycleMachineError::UnexpectedIdentity);
                }
                if let Some(previous) = self.identity {
                    self.retired_identity = Some(previous);
                }
                self.identity = None;
                AppLifecycleState::Launching
            }
            (AppLifecycleState::Inactive, AppLifecycleAction::Activate) => {
                self.require_current_identity(identity)?;
                AppLifecycleState::Activating
            }
            (AppLifecycleState::Active, AppLifecycleAction::Suspend) => {
                self.require_current_identity(identity)?;
                AppLifecycleState::Suspending
            }
            (AppLifecycleState::Suspended, AppLifecycleAction::Resume) => {
                self.require_current_identity(identity)?;
                AppLifecycleState::Resuming
            }
            (
                AppLifecycleState::Inactive
                | AppLifecycleState::Active
                | AppLifecycleState::Suspended,
                AppLifecycleAction::Terminate,
            ) => {
                self.require_current_identity(identity)?;
                AppLifecycleState::Terminating
            }
            _ => return Err(AppLifecycleMachineError::InvalidTransition),
        };
        self.state = next_state;
        self.active_transaction_id = Some(transaction_id);
        Ok(next_state)
    }

    /// Completes the spawn half of a Launch transaction without completing
    /// the App's Launch command. This split represents the interval in which
    /// ProcessSpawn succeeded but the App may exit before Init publishes
    /// `Launching` or delivers the command.
    pub fn bind_identity(
        &mut self,
        transaction_id: u64,
        identity: AppInstanceIdentity,
    ) -> Result<(), AppLifecycleMachineError> {
        if self.active_transaction_id != Some(transaction_id) {
            return if self.active_transaction_id.is_some() {
                Err(AppLifecycleMachineError::TransactionMismatch)
            } else {
                Err(AppLifecycleMachineError::NoActiveTransaction)
            };
        }
        if self.state != AppLifecycleState::Launching {
            return Err(AppLifecycleMachineError::InvalidTransition);
        }
        if self.identity.is_some() {
            return Err(AppLifecycleMachineError::IdentityAlreadyBound);
        }
        if let Some(retired) = self.retired_identity
            && (retired.instance_id() == identity.instance_id()
                || retired.process() == identity.process())
        {
            return Err(AppLifecycleMachineError::StaleIdentity);
        }
        self.identity = Some(identity);
        Ok(())
    }

    pub fn complete(
        &mut self,
        transaction_id: u64,
        identity: AppInstanceIdentity,
    ) -> Result<AppLifecycleState, AppLifecycleMachineError> {
        self.require_active(transaction_id, identity)?;
        let next_state = match self.state {
            AppLifecycleState::Launching => AppLifecycleState::Inactive,
            AppLifecycleState::Activating | AppLifecycleState::Resuming => {
                AppLifecycleState::Active
            }
            AppLifecycleState::Suspending => AppLifecycleState::Suspended,
            AppLifecycleState::Terminating => AppLifecycleState::NotRunning,
            _ => return Err(AppLifecycleMachineError::InvalidTransition),
        };
        self.finish_transaction(transaction_id);
        self.state = next_state;
        if next_state == AppLifecycleState::NotRunning {
            self.retired_identity = self.identity;
            self.identity = None;
        }
        Ok(next_state)
    }

    pub fn fail(
        &mut self,
        transaction_id: u64,
        identity: AppInstanceIdentity,
    ) -> Result<AppLifecycleState, AppLifecycleMachineError> {
        self.require_active(transaction_id, identity)?;
        match self.state {
            AppLifecycleState::Launching
            | AppLifecycleState::Activating
            | AppLifecycleState::Suspending
            | AppLifecycleState::Resuming
            | AppLifecycleState::Terminating => {}
            _ => return Err(AppLifecycleMachineError::InvalidTransition),
        }
        self.finish_transaction(transaction_id);
        self.state = AppLifecycleState::Failed;
        Ok(self.state)
    }

    /// Fails a Launch transaction before ProcessSpawn produced an identity.
    pub fn fail_before_spawn(
        &mut self,
        transaction_id: u64,
    ) -> Result<AppLifecycleState, AppLifecycleMachineError> {
        let active = self
            .active_transaction_id
            .ok_or(AppLifecycleMachineError::NoActiveTransaction)?;
        if active != transaction_id {
            return Err(AppLifecycleMachineError::TransactionMismatch);
        }
        if self.state != AppLifecycleState::Launching {
            return Err(AppLifecycleMachineError::InvalidTransition);
        }
        if self.identity.is_some() {
            return Err(AppLifecycleMachineError::IdentityAlreadyBound);
        }
        self.finish_transaction(transaction_id);
        self.state = AppLifecycleState::Failed;
        Ok(self.state)
    }

    /// Records a process exit. During a transition it must carry that active
    /// transaction id; otherwise it must allocate a strictly newer id.
    pub fn crash(
        &mut self,
        transaction_id: u64,
        identity: AppInstanceIdentity,
    ) -> Result<AppLifecycleState, AppLifecycleMachineError> {
        if matches!(
            self.state,
            AppLifecycleState::NotRunning | AppLifecycleState::Crashed
        ) {
            return Err(AppLifecycleMachineError::InvalidTransition);
        }
        if self.identity != Some(identity) {
            return Err(AppLifecycleMachineError::StaleIdentity);
        }
        if let Some(active) = self.active_transaction_id {
            if transaction_id != active {
                return Err(AppLifecycleMachineError::TransactionMismatch);
            }
        } else {
            self.require_new_transaction(transaction_id)?;
        }
        self.finish_transaction(transaction_id);
        self.state = AppLifecycleState::Crashed;
        Ok(self.state)
    }

    fn require_active(
        self,
        transaction_id: u64,
        identity: AppInstanceIdentity,
    ) -> Result<(), AppLifecycleMachineError> {
        let active = self
            .active_transaction_id
            .ok_or(AppLifecycleMachineError::NoActiveTransaction)?;
        if active != transaction_id {
            return Err(AppLifecycleMachineError::TransactionMismatch);
        }
        if self.identity != Some(identity) {
            return Err(AppLifecycleMachineError::StaleIdentity);
        }
        Ok(())
    }

    fn require_current_identity(
        self,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), AppLifecycleMachineError> {
        let identity = identity.ok_or(AppLifecycleMachineError::MissingIdentity)?;
        if self.identity != Some(identity) {
            return Err(AppLifecycleMachineError::StaleIdentity);
        }
        Ok(())
    }

    fn require_new_transaction(self, transaction_id: u64) -> Result<(), AppLifecycleMachineError> {
        if transaction_id == 0 {
            return Err(AppLifecycleMachineError::TransactionReplay);
        }
        if let Some(previous) = self.last_transaction_id {
            if previous == u64::MAX {
                return Err(AppLifecycleMachineError::TransactionExhausted);
            }
            if transaction_id <= previous {
                return Err(AppLifecycleMachineError::TransactionReplay);
            }
        }
        Ok(())
    }

    fn finish_transaction(&mut self, transaction_id: u64) {
        self.active_transaction_id = None;
        self.last_transaction_id = Some(transaction_id);
    }

    pub const fn app(self) -> ShellAppId {
        self.app
    }

    pub const fn state(self) -> AppLifecycleState {
        self.state
    }

    pub const fn identity(self) -> Option<AppInstanceIdentity> {
        self.identity
    }

    pub const fn retired_identity(self) -> Option<AppInstanceIdentity> {
        self.retired_identity
    }

    pub const fn active_transaction_id(self) -> Option<u64> {
        self.active_transaction_id
    }

    pub const fn last_transaction_id(self) -> Option<u64> {
        self.last_transaction_id
    }
}

const BOOTSTRAP_OFFSET_MAGIC: usize = 0;
const BOOTSTRAP_OFFSET_VERSION: usize = 4;
const BOOTSTRAP_OFFSET_KIND: usize = 6;
const BOOTSTRAP_OFFSET_RESERVED: usize = 7;

/// The endpoint capability transferred alongside one UBP1 message.
///
/// The enum is deliberately closed: receivers must reject endpoint roles
/// introduced by a later protocol version instead of silently assigning
/// authority they do not understand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiBootstrapEndpointKind {
    SurfaceSupervisor = 1,
    LauncherUi = 2,
    AppUi = 3,
}

impl UiBootstrapEndpointKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::SurfaceSupervisor),
            2 => Some(Self::LauncherUi),
            3 => Some(Self::AppUi),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBootstrapWireError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroReserved,
}

/// Canonical UBP1 descriptor for a channel endpoint transferred out-of-band.
///
/// Only magic, version and kind are populated. Bytes 7..64 are reserved zero,
/// so a v1 decoder has one and only one byte representation for every role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiBootstrapMessage {
    kind: UiBootstrapEndpointKind,
}

impl UiBootstrapMessage {
    pub const fn new(kind: UiBootstrapEndpointKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> UiBootstrapEndpointKind {
        self.kind
    }

    pub fn encode(self) -> [u8; UI_BOOTSTRAP_WIRE_SIZE] {
        let mut wire = [0_u8; UI_BOOTSTRAP_WIRE_SIZE];
        wire[BOOTSTRAP_OFFSET_MAGIC..BOOTSTRAP_OFFSET_MAGIC + 4]
            .copy_from_slice(&UI_BOOTSTRAP_MAGIC.to_le_bytes());
        wire[BOOTSTRAP_OFFSET_VERSION..BOOTSTRAP_OFFSET_VERSION + 2]
            .copy_from_slice(&UI_BOOTSTRAP_VERSION.to_le_bytes());
        wire[BOOTSTRAP_OFFSET_KIND] = self.kind.raw();
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, UiBootstrapWireError> {
        if wire.len() != UI_BOOTSTRAP_WIRE_SIZE {
            return Err(UiBootstrapWireError::InvalidWireLength);
        }
        if read_u32(wire, BOOTSTRAP_OFFSET_MAGIC) != UI_BOOTSTRAP_MAGIC {
            return Err(UiBootstrapWireError::InvalidMagic);
        }
        if read_u16(wire, BOOTSTRAP_OFFSET_VERSION) != UI_BOOTSTRAP_VERSION {
            return Err(UiBootstrapWireError::InvalidVersion);
        }
        let kind = UiBootstrapEndpointKind::from_raw(wire[BOOTSTRAP_OFFSET_KIND])
            .ok_or(UiBootstrapWireError::InvalidKind)?;
        if wire[BOOTSTRAP_OFFSET_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(UiBootstrapWireError::NonZeroReserved);
        }
        Ok(Self::new(kind))
    }
}

const RECOVERY_OFFSET_MAGIC: usize = 0;
const RECOVERY_OFFSET_VERSION: usize = 4;
const RECOVERY_OFFSET_KIND: usize = 6;
const RECOVERY_OFFSET_HEADER_RESERVED: usize = 7;
const RECOVERY_OFFSET_SENDER_SEQUENCE: usize = 8;
const RECOVERY_OFFSET_SURFACE_SESSION: usize = 16;
const RECOVERY_OFFSET_ROUTE_EPOCH: usize = 24;
const RECOVERY_OFFSET_VALUE_0: usize = 32;
const RECOVERY_OFFSET_VALUE_1: usize = 40;
const RECOVERY_OFFSET_VALUE_2: usize = 48;
const RECOVERY_OFFSET_VALUE_3: usize = 56;

/// Authority direction for one BSR1 recovery message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SurfaceRecoveryDirection {
    InitToSurface = 1,
    SurfaceToInit = 2,
    SurfaceToClient = 3,
    ClientToSurface = 4,
}

/// Closed BSR1 message vocabulary. Endpoint transfer remains out-of-band.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SurfaceRecoveryKind {
    SurfaceStatus = 1,
    Bootstrap = 2,
    ClientRebindOffer = 3,
    ClientRebindAck = 4,
    ClientContact = 5,
    ClientContactAck = 6,
}

impl SurfaceRecoveryKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::SurfaceStatus),
            2 => Some(Self::Bootstrap),
            3 => Some(Self::ClientRebindOffer),
            4 => Some(Self::ClientRebindAck),
            5 => Some(Self::ClientContact),
            6 => Some(Self::ClientContactAck),
            _ => None,
        }
    }

    pub const fn direction(self) -> SurfaceRecoveryDirection {
        match self {
            Self::SurfaceStatus => SurfaceRecoveryDirection::SurfaceToInit,
            Self::Bootstrap => SurfaceRecoveryDirection::InitToSurface,
            Self::ClientRebindOffer | Self::ClientContact => {
                SurfaceRecoveryDirection::SurfaceToClient
            }
            Self::ClientRebindAck | Self::ClientContactAck => {
                SurfaceRecoveryDirection::ClientToSurface
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SurfaceRecoveryPhase {
    Armed = 1,
    RestartRequested = 2,
    GraphPrepared = 3,
    Active = 4,
}

impl SurfaceRecoveryPhase {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Armed),
            2 => Some(Self::RestartRequested),
            3 => Some(Self::GraphPrepared),
            4 => Some(Self::Active),
            _ => None,
        }
    }
}

/// Canonical restart point whose retained scene is reconstructed by M46.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SurfaceRecoveryCheckpoint {
    MultiWindowInteractive = 1,
}

impl SurfaceRecoveryCheckpoint {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::MultiWindowInteractive),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RecoveryClientRole {
    Launcher = 1,
    App = 2,
}

impl RecoveryClientRole {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Launcher),
            2 => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RecoveryCancelKind {
    None = 0,
    ClientPointer = 1,
    TrustedOverlayPointer = 2,
}

impl RecoveryCancelKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::ClientPointer),
            2 => Some(Self::TrustedOverlayPointer),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceRecoveryPayload {
    SurfaceStatus {
        checkpoint: SurfaceRecoveryCheckpoint,
        related_physical_sequence: u64,
        phase: SurfaceRecoveryPhase,
    },
    Bootstrap {
        input_server_pid: u64,
        input_session_id: u64,
        physical_floor: u64,
        checkpoint: SurfaceRecoveryCheckpoint,
        expected_gap_events: u8,
        cancel_kind: RecoveryCancelKind,
    },
    ClientRebindOffer {
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        old_window: WindowId,
        physical_floor: u64,
        cancel_after: u64,
    },
    ClientRebindAck {
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        replacement_window: WindowId,
        acknowledged_offer_sequence: u64,
        cancel_after: u64,
    },
    ClientContact {
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        old_window: WindowId,
        physical_sequence: u64,
    },
    ClientContactAck {
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        old_window: WindowId,
        acknowledged_contact_sequence: u64,
        physical_sequence: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceRecoveryError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSenderSequence,
    ZeroSurfaceSession,
    ZeroRouteEpoch,
    InvalidPhase,
    InvalidCheckpoint,
    InvalidClientRole,
    InvalidCancelKind,
    ZeroInputServerPid,
    ZeroInputSession,
    InvalidGapEventCount,
    InvalidWindowId(WindowIdError),
    LauncherCannotCancel,
    TrustedCancelCannotTargetClient,
    CancelAfterRequired,
    UnexpectedCancelAfter,
    CancelAfterBeyondFloor,
    ZeroAcknowledgedOffer,
    ContactRequiresApp,
    ContactRequiresClientPointer,
    ZeroContactPhysicalSequence,
    ZeroAcknowledgedContact,
    InvalidRelatedSequence,
}

/// Canonical 64-byte Surface restart, bootstrap, client-contact, and rebind message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceRecoveryMessage {
    sender_sequence: u64,
    surface_session: u64,
    route_epoch: u64,
    payload: SurfaceRecoveryPayload,
}

impl SurfaceRecoveryMessage {
    pub fn status(
        sender_sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        checkpoint: SurfaceRecoveryCheckpoint,
        related_physical_sequence: u64,
        phase: SurfaceRecoveryPhase,
    ) -> Result<Self, SurfaceRecoveryError> {
        validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
        match phase {
            SurfaceRecoveryPhase::Armed if related_physical_sequence != 0 => {
                return Err(SurfaceRecoveryError::InvalidRelatedSequence);
            }
            SurfaceRecoveryPhase::RestartRequested
            | SurfaceRecoveryPhase::GraphPrepared
            | SurfaceRecoveryPhase::Active
                if related_physical_sequence == 0 =>
            {
                return Err(SurfaceRecoveryError::InvalidRelatedSequence);
            }
            _ => {}
        }
        Ok(Self {
            sender_sequence,
            surface_session,
            route_epoch,
            payload: SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint,
                related_physical_sequence,
                phase,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn bootstrap(
        sender_sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        input_server_pid: u64,
        input_session_id: u64,
        physical_floor: u64,
        checkpoint: SurfaceRecoveryCheckpoint,
        expected_gap_events: u8,
        cancel_kind: RecoveryCancelKind,
    ) -> Result<Self, SurfaceRecoveryError> {
        validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
        if input_server_pid == 0 {
            return Err(SurfaceRecoveryError::ZeroInputServerPid);
        }
        if input_session_id == 0 {
            return Err(SurfaceRecoveryError::ZeroInputSession);
        }
        if expected_gap_events == 0 || expected_gap_events > 16 {
            return Err(SurfaceRecoveryError::InvalidGapEventCount);
        }
        Ok(Self {
            sender_sequence,
            surface_session,
            route_epoch,
            payload: SurfaceRecoveryPayload::Bootstrap {
                input_server_pid,
                input_session_id,
                physical_floor,
                checkpoint,
                expected_gap_events,
                cancel_kind,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn client_rebind_offer(
        sender_sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        old_window: WindowId,
        physical_floor: u64,
        cancel_after: u64,
    ) -> Result<Self, SurfaceRecoveryError> {
        validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
        validate_client_cancel(role, cancel_kind, cancel_after, Some(physical_floor))?;
        Ok(Self {
            sender_sequence,
            surface_session,
            route_epoch,
            payload: SurfaceRecoveryPayload::ClientRebindOffer {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                physical_floor,
                cancel_after,
            },
        })
    }

    pub fn client_rebind_ack(
        sender_sequence: u64,
        offer: Self,
        replacement_window: WindowId,
    ) -> Result<Self, SurfaceRecoveryError> {
        let SurfaceRecoveryPayload::ClientRebindOffer {
            role,
            cancel_kind,
            checkpoint,
            cancel_after,
            ..
        } = offer.payload
        else {
            return Err(SurfaceRecoveryError::ZeroAcknowledgedOffer);
        };
        validate_recovery_header(sender_sequence, offer.surface_session, offer.route_epoch)?;
        validate_client_cancel(role, cancel_kind, cancel_after, None)?;
        Ok(Self {
            sender_sequence,
            surface_session: offer.surface_session,
            route_epoch: offer.route_epoch,
            payload: SurfaceRecoveryPayload::ClientRebindAck {
                role,
                cancel_kind,
                checkpoint,
                replacement_window,
                acknowledged_offer_sequence: offer.sender_sequence,
                cancel_after,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn client_contact(
        sender_sequence: u64,
        surface_session: u64,
        route_epoch: u64,
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        checkpoint: SurfaceRecoveryCheckpoint,
        old_window: WindowId,
        physical_sequence: u64,
    ) -> Result<Self, SurfaceRecoveryError> {
        validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
        validate_client_contact(role, cancel_kind, physical_sequence)?;
        Ok(Self {
            sender_sequence,
            surface_session,
            route_epoch,
            payload: SurfaceRecoveryPayload::ClientContact {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                physical_sequence,
            },
        })
    }

    pub fn client_contact_ack(
        sender_sequence: u64,
        contact: Self,
    ) -> Result<Self, SurfaceRecoveryError> {
        let SurfaceRecoveryPayload::ClientContact {
            role,
            cancel_kind,
            checkpoint,
            old_window,
            physical_sequence,
        } = contact.payload
        else {
            return Err(SurfaceRecoveryError::ZeroAcknowledgedContact);
        };
        validate_recovery_header(
            sender_sequence,
            contact.surface_session,
            contact.route_epoch,
        )?;
        validate_client_contact(role, cancel_kind, physical_sequence)?;
        Ok(Self {
            sender_sequence,
            surface_session: contact.surface_session,
            route_epoch: contact.route_epoch,
            payload: SurfaceRecoveryPayload::ClientContactAck {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                acknowledged_contact_sequence: contact.sender_sequence,
                physical_sequence,
            },
        })
    }

    pub const fn sender_sequence(self) -> u64 {
        self.sender_sequence
    }

    pub const fn surface_session(self) -> u64 {
        self.surface_session
    }

    pub const fn route_epoch(self) -> u64 {
        self.route_epoch
    }

    pub const fn payload(self) -> SurfaceRecoveryPayload {
        self.payload
    }

    pub const fn kind(self) -> SurfaceRecoveryKind {
        match self.payload {
            SurfaceRecoveryPayload::SurfaceStatus { .. } => SurfaceRecoveryKind::SurfaceStatus,
            SurfaceRecoveryPayload::Bootstrap { .. } => SurfaceRecoveryKind::Bootstrap,
            SurfaceRecoveryPayload::ClientRebindOffer { .. } => {
                SurfaceRecoveryKind::ClientRebindOffer
            }
            SurfaceRecoveryPayload::ClientRebindAck { .. } => SurfaceRecoveryKind::ClientRebindAck,
            SurfaceRecoveryPayload::ClientContact { .. } => SurfaceRecoveryKind::ClientContact,
            SurfaceRecoveryPayload::ClientContactAck { .. } => {
                SurfaceRecoveryKind::ClientContactAck
            }
        }
    }

    pub const fn direction(self) -> SurfaceRecoveryDirection {
        self.kind().direction()
    }

    pub const fn acknowledges(self, offer: Self) -> bool {
        match (self.payload, offer.payload) {
            (
                SurfaceRecoveryPayload::ClientRebindAck {
                    role,
                    cancel_kind,
                    checkpoint,
                    acknowledged_offer_sequence,
                    cancel_after,
                    ..
                },
                SurfaceRecoveryPayload::ClientRebindOffer {
                    role: offered_role,
                    cancel_kind: offered_cancel,
                    checkpoint: offered_checkpoint,
                    cancel_after: offered_cancel_after,
                    ..
                },
            ) => {
                self.surface_session == offer.surface_session
                    && self.route_epoch == offer.route_epoch
                    && role as u8 == offered_role as u8
                    && cancel_kind as u8 == offered_cancel as u8
                    && checkpoint as u16 == offered_checkpoint as u16
                    && acknowledged_offer_sequence == offer.sender_sequence
                    && cancel_after == offered_cancel_after
            }
            _ => false,
        }
    }

    pub const fn acknowledges_contact(self, contact: Self) -> bool {
        match (self.payload, contact.payload) {
            (
                SurfaceRecoveryPayload::ClientContactAck {
                    role,
                    cancel_kind,
                    checkpoint,
                    old_window,
                    acknowledged_contact_sequence,
                    physical_sequence,
                },
                SurfaceRecoveryPayload::ClientContact {
                    role: contacted_role,
                    cancel_kind: contacted_cancel,
                    checkpoint: contacted_checkpoint,
                    old_window: contacted_window,
                    physical_sequence: contacted_physical_sequence,
                },
            ) => {
                self.surface_session == contact.surface_session
                    && self.route_epoch == contact.route_epoch
                    && role as u8 == contacted_role as u8
                    && cancel_kind as u8 == contacted_cancel as u8
                    && checkpoint as u16 == contacted_checkpoint as u16
                    && old_window.token() == contacted_window.token()
                    && acknowledged_contact_sequence == contact.sender_sequence
                    && physical_sequence == contacted_physical_sequence
            }
            _ => false,
        }
    }

    pub fn encode(self) -> [u8; SURFACE_RECOVERY_WIRE_SIZE] {
        let mut wire = [0_u8; SURFACE_RECOVERY_WIRE_SIZE];
        wire[RECOVERY_OFFSET_MAGIC..RECOVERY_OFFSET_MAGIC + 4]
            .copy_from_slice(&SURFACE_RECOVERY_MAGIC.to_le_bytes());
        wire[RECOVERY_OFFSET_VERSION..RECOVERY_OFFSET_VERSION + 2]
            .copy_from_slice(&SURFACE_RECOVERY_VERSION.to_le_bytes());
        wire[RECOVERY_OFFSET_KIND] = self.kind().raw();
        write_recovery_u64(
            &mut wire,
            RECOVERY_OFFSET_SENDER_SEQUENCE,
            self.sender_sequence,
        );
        write_recovery_u64(
            &mut wire,
            RECOVERY_OFFSET_SURFACE_SESSION,
            self.surface_session,
        );
        write_recovery_u64(&mut wire, RECOVERY_OFFSET_ROUTE_EPOCH, self.route_epoch);
        match self.payload {
            SurfaceRecoveryPayload::SurfaceStatus {
                checkpoint,
                related_physical_sequence,
                phase,
            } => {
                write_recovery_u64(
                    &mut wire,
                    RECOVERY_OFFSET_VALUE_0,
                    u64::from(checkpoint.raw()),
                );
                write_recovery_u64(
                    &mut wire,
                    RECOVERY_OFFSET_VALUE_1,
                    related_physical_sequence,
                );
                wire[RECOVERY_OFFSET_VALUE_2] = phase.raw();
            }
            SurfaceRecoveryPayload::Bootstrap {
                input_server_pid,
                input_session_id,
                physical_floor,
                checkpoint,
                expected_gap_events,
                cancel_kind,
            } => {
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_0, input_server_pid);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_1, input_session_id);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_2, physical_floor);
                wire[RECOVERY_OFFSET_VALUE_3..RECOVERY_OFFSET_VALUE_3 + 2]
                    .copy_from_slice(&checkpoint.raw().to_le_bytes());
                wire[RECOVERY_OFFSET_VALUE_3 + 2] = expected_gap_events;
                wire[RECOVERY_OFFSET_VALUE_3 + 3] = cancel_kind.raw();
            }
            SurfaceRecoveryPayload::ClientRebindOffer {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                physical_floor,
                cancel_after,
            } => {
                encode_recovery_client_metadata(&mut wire, role, cancel_kind, checkpoint);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_1, old_window.token());
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_2, physical_floor);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_3, cancel_after);
            }
            SurfaceRecoveryPayload::ClientRebindAck {
                role,
                cancel_kind,
                checkpoint,
                replacement_window,
                acknowledged_offer_sequence,
                cancel_after,
            } => {
                encode_recovery_client_metadata(&mut wire, role, cancel_kind, checkpoint);
                write_recovery_u64(
                    &mut wire,
                    RECOVERY_OFFSET_VALUE_1,
                    replacement_window.token(),
                );
                write_recovery_u64(
                    &mut wire,
                    RECOVERY_OFFSET_VALUE_2,
                    acknowledged_offer_sequence,
                );
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_3, cancel_after);
            }
            SurfaceRecoveryPayload::ClientContact {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                physical_sequence,
            } => {
                encode_recovery_client_metadata(&mut wire, role, cancel_kind, checkpoint);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_1, old_window.token());
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_2, physical_sequence);
            }
            SurfaceRecoveryPayload::ClientContactAck {
                role,
                cancel_kind,
                checkpoint,
                old_window,
                acknowledged_contact_sequence,
                physical_sequence,
            } => {
                encode_recovery_client_metadata(&mut wire, role, cancel_kind, checkpoint);
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_1, old_window.token());
                write_recovery_u64(
                    &mut wire,
                    RECOVERY_OFFSET_VALUE_2,
                    acknowledged_contact_sequence,
                );
                write_recovery_u64(&mut wire, RECOVERY_OFFSET_VALUE_3, physical_sequence);
            }
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, SurfaceRecoveryError> {
        if wire.len() != SURFACE_RECOVERY_WIRE_SIZE {
            return Err(SurfaceRecoveryError::InvalidWireLength);
        }
        if read_u32(wire, RECOVERY_OFFSET_MAGIC) != SURFACE_RECOVERY_MAGIC {
            return Err(SurfaceRecoveryError::InvalidMagic);
        }
        if read_u16(wire, RECOVERY_OFFSET_VERSION) != SURFACE_RECOVERY_VERSION {
            return Err(SurfaceRecoveryError::InvalidVersion);
        }
        let kind = SurfaceRecoveryKind::from_raw(wire[RECOVERY_OFFSET_KIND])
            .ok_or(SurfaceRecoveryError::InvalidKind)?;
        if wire[RECOVERY_OFFSET_HEADER_RESERVED] != 0 {
            return Err(SurfaceRecoveryError::NonZeroHeaderReserved);
        }
        let sender_sequence = read_u64(wire, RECOVERY_OFFSET_SENDER_SEQUENCE);
        let surface_session = read_u64(wire, RECOVERY_OFFSET_SURFACE_SESSION);
        let route_epoch = read_u64(wire, RECOVERY_OFFSET_ROUTE_EPOCH);
        match kind {
            SurfaceRecoveryKind::SurfaceStatus => {
                if wire[RECOVERY_OFFSET_VALUE_2 + 1..]
                    .iter()
                    .any(|byte| *byte != 0)
                {
                    return Err(SurfaceRecoveryError::NonZeroBodyReserved);
                }
                let checkpoint =
                    decode_recovery_checkpoint(read_u64(wire, RECOVERY_OFFSET_VALUE_0))?;
                let phase = SurfaceRecoveryPhase::from_raw(wire[RECOVERY_OFFSET_VALUE_2])
                    .ok_or(SurfaceRecoveryError::InvalidPhase)?;
                Self::status(
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    checkpoint,
                    read_u64(wire, RECOVERY_OFFSET_VALUE_1),
                    phase,
                )
            }
            SurfaceRecoveryKind::Bootstrap => {
                if wire[RECOVERY_OFFSET_VALUE_3 + 4..]
                    .iter()
                    .any(|byte| *byte != 0)
                {
                    return Err(SurfaceRecoveryError::NonZeroBodyReserved);
                }
                let checkpoint =
                    SurfaceRecoveryCheckpoint::from_raw(read_u16(wire, RECOVERY_OFFSET_VALUE_3))
                        .ok_or(SurfaceRecoveryError::InvalidCheckpoint)?;
                let cancel_kind = RecoveryCancelKind::from_raw(wire[RECOVERY_OFFSET_VALUE_3 + 3])
                    .ok_or(SurfaceRecoveryError::InvalidCancelKind)?;
                Self::bootstrap(
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    read_u64(wire, RECOVERY_OFFSET_VALUE_0),
                    read_u64(wire, RECOVERY_OFFSET_VALUE_1),
                    read_u64(wire, RECOVERY_OFFSET_VALUE_2),
                    checkpoint,
                    wire[RECOVERY_OFFSET_VALUE_3 + 2],
                    cancel_kind,
                )
            }
            SurfaceRecoveryKind::ClientRebindOffer => {
                let (role, cancel_kind, checkpoint) = decode_recovery_client_metadata(wire)?;
                let old_window = WindowId::from_token(read_u64(wire, RECOVERY_OFFSET_VALUE_1))
                    .map_err(SurfaceRecoveryError::InvalidWindowId)?;
                Self::client_rebind_offer(
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    role,
                    cancel_kind,
                    checkpoint,
                    old_window,
                    read_u64(wire, RECOVERY_OFFSET_VALUE_2),
                    read_u64(wire, RECOVERY_OFFSET_VALUE_3),
                )
            }
            SurfaceRecoveryKind::ClientRebindAck => {
                let (role, cancel_kind, checkpoint) = decode_recovery_client_metadata(wire)?;
                let replacement_window =
                    WindowId::from_token(read_u64(wire, RECOVERY_OFFSET_VALUE_1))
                        .map_err(SurfaceRecoveryError::InvalidWindowId)?;
                let acknowledged_offer_sequence = read_u64(wire, RECOVERY_OFFSET_VALUE_2);
                if acknowledged_offer_sequence == 0 {
                    return Err(SurfaceRecoveryError::ZeroAcknowledgedOffer);
                }
                validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
                let cancel_after = read_u64(wire, RECOVERY_OFFSET_VALUE_3);
                validate_client_cancel(role, cancel_kind, cancel_after, None)?;
                Ok(Self {
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    payload: SurfaceRecoveryPayload::ClientRebindAck {
                        role,
                        cancel_kind,
                        checkpoint,
                        replacement_window,
                        acknowledged_offer_sequence,
                        cancel_after,
                    },
                })
            }
            SurfaceRecoveryKind::ClientContact => {
                if read_u64(wire, RECOVERY_OFFSET_VALUE_3) != 0 {
                    return Err(SurfaceRecoveryError::NonZeroBodyReserved);
                }
                let (role, cancel_kind, checkpoint) = decode_recovery_client_metadata(wire)?;
                let old_window = WindowId::from_token(read_u64(wire, RECOVERY_OFFSET_VALUE_1))
                    .map_err(SurfaceRecoveryError::InvalidWindowId)?;
                Self::client_contact(
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    role,
                    cancel_kind,
                    checkpoint,
                    old_window,
                    read_u64(wire, RECOVERY_OFFSET_VALUE_2),
                )
            }
            SurfaceRecoveryKind::ClientContactAck => {
                let (role, cancel_kind, checkpoint) = decode_recovery_client_metadata(wire)?;
                let old_window = WindowId::from_token(read_u64(wire, RECOVERY_OFFSET_VALUE_1))
                    .map_err(SurfaceRecoveryError::InvalidWindowId)?;
                let acknowledged_contact_sequence = read_u64(wire, RECOVERY_OFFSET_VALUE_2);
                if acknowledged_contact_sequence == 0 {
                    return Err(SurfaceRecoveryError::ZeroAcknowledgedContact);
                }
                let physical_sequence = read_u64(wire, RECOVERY_OFFSET_VALUE_3);
                validate_recovery_header(sender_sequence, surface_session, route_epoch)?;
                validate_client_contact(role, cancel_kind, physical_sequence)?;
                Ok(Self {
                    sender_sequence,
                    surface_session,
                    route_epoch,
                    payload: SurfaceRecoveryPayload::ClientContactAck {
                        role,
                        cancel_kind,
                        checkpoint,
                        old_window,
                        acknowledged_contact_sequence,
                        physical_sequence,
                    },
                })
            }
        }
    }
}

fn validate_recovery_header(
    sender_sequence: u64,
    surface_session: u64,
    route_epoch: u64,
) -> Result<(), SurfaceRecoveryError> {
    if sender_sequence == 0 {
        return Err(SurfaceRecoveryError::ZeroSenderSequence);
    }
    if surface_session == 0 {
        return Err(SurfaceRecoveryError::ZeroSurfaceSession);
    }
    if route_epoch == 0 {
        return Err(SurfaceRecoveryError::ZeroRouteEpoch);
    }
    Ok(())
}

fn validate_client_cancel(
    role: RecoveryClientRole,
    cancel_kind: RecoveryCancelKind,
    cancel_after: u64,
    floor: Option<u64>,
) -> Result<(), SurfaceRecoveryError> {
    if role == RecoveryClientRole::Launcher && cancel_kind != RecoveryCancelKind::None {
        return Err(SurfaceRecoveryError::LauncherCannotCancel);
    }
    if cancel_kind == RecoveryCancelKind::TrustedOverlayPointer {
        return Err(SurfaceRecoveryError::TrustedCancelCannotTargetClient);
    }
    match cancel_kind {
        RecoveryCancelKind::None if cancel_after != 0 => {
            Err(SurfaceRecoveryError::UnexpectedCancelAfter)
        }
        RecoveryCancelKind::ClientPointer if cancel_after == 0 => {
            Err(SurfaceRecoveryError::CancelAfterRequired)
        }
        RecoveryCancelKind::ClientPointer
            if floor.is_some_and(|physical_floor| cancel_after > physical_floor) =>
        {
            Err(SurfaceRecoveryError::CancelAfterBeyondFloor)
        }
        RecoveryCancelKind::TrustedOverlayPointer => {
            Err(SurfaceRecoveryError::TrustedCancelCannotTargetClient)
        }
        _ => Ok(()),
    }
}

fn validate_client_contact(
    role: RecoveryClientRole,
    cancel_kind: RecoveryCancelKind,
    physical_sequence: u64,
) -> Result<(), SurfaceRecoveryError> {
    if role != RecoveryClientRole::App {
        return Err(SurfaceRecoveryError::ContactRequiresApp);
    }
    if cancel_kind != RecoveryCancelKind::ClientPointer {
        return Err(SurfaceRecoveryError::ContactRequiresClientPointer);
    }
    if physical_sequence == 0 {
        return Err(SurfaceRecoveryError::ZeroContactPhysicalSequence);
    }
    Ok(())
}

fn decode_recovery_checkpoint(raw: u64) -> Result<SurfaceRecoveryCheckpoint, SurfaceRecoveryError> {
    let raw = u16::try_from(raw).map_err(|_| SurfaceRecoveryError::InvalidCheckpoint)?;
    SurfaceRecoveryCheckpoint::from_raw(raw).ok_or(SurfaceRecoveryError::InvalidCheckpoint)
}

fn encode_recovery_client_metadata(
    wire: &mut [u8; SURFACE_RECOVERY_WIRE_SIZE],
    role: RecoveryClientRole,
    cancel_kind: RecoveryCancelKind,
    checkpoint: SurfaceRecoveryCheckpoint,
) {
    wire[RECOVERY_OFFSET_VALUE_0] = role.raw();
    wire[RECOVERY_OFFSET_VALUE_0 + 1] = cancel_kind.raw();
    wire[RECOVERY_OFFSET_VALUE_0 + 2..RECOVERY_OFFSET_VALUE_0 + 4]
        .copy_from_slice(&checkpoint.raw().to_le_bytes());
}

fn decode_recovery_client_metadata(
    wire: &[u8],
) -> Result<
    (
        RecoveryClientRole,
        RecoveryCancelKind,
        SurfaceRecoveryCheckpoint,
    ),
    SurfaceRecoveryError,
> {
    if wire[RECOVERY_OFFSET_VALUE_0 + 4..RECOVERY_OFFSET_VALUE_1]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(SurfaceRecoveryError::NonZeroBodyReserved);
    }
    let role = RecoveryClientRole::from_raw(wire[RECOVERY_OFFSET_VALUE_0])
        .ok_or(SurfaceRecoveryError::InvalidClientRole)?;
    let cancel_kind = RecoveryCancelKind::from_raw(wire[RECOVERY_OFFSET_VALUE_0 + 1])
        .ok_or(SurfaceRecoveryError::InvalidCancelKind)?;
    let checkpoint =
        SurfaceRecoveryCheckpoint::from_raw(read_u16(wire, RECOVERY_OFFSET_VALUE_0 + 2))
            .ok_or(SurfaceRecoveryError::InvalidCheckpoint)?;
    Ok((role, cancel_kind, checkpoint))
}

fn write_recovery_u64(wire: &mut [u8; SURFACE_RECOVERY_WIRE_SIZE], offset: usize, value: u64) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceRecoverySequenceError {
    SequenceExhausted,
    SequenceReplay,
    SequenceGap,
}

/// Independent contiguous receive cursors for one BSR1 session's four directions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SurfaceRecoverySequenceTracker {
    init_to_surface: Option<u64>,
    surface_to_init: Option<u64>,
    surface_to_client: Option<u64>,
    client_to_surface: Option<u64>,
}

impl SurfaceRecoverySequenceTracker {
    pub const fn new() -> Self {
        Self {
            init_to_surface: None,
            surface_to_init: None,
            surface_to_client: None,
            client_to_surface: None,
        }
    }

    pub fn accept(
        &mut self,
        message: SurfaceRecoveryMessage,
    ) -> Result<(), SurfaceRecoverySequenceError> {
        let cursor = match message.direction() {
            SurfaceRecoveryDirection::InitToSurface => &mut self.init_to_surface,
            SurfaceRecoveryDirection::SurfaceToInit => &mut self.surface_to_init,
            SurfaceRecoveryDirection::SurfaceToClient => &mut self.surface_to_client,
            SurfaceRecoveryDirection::ClientToSurface => &mut self.client_to_surface,
        };
        let expected = match *cursor {
            Some(previous) => previous
                .checked_add(1)
                .ok_or(SurfaceRecoverySequenceError::SequenceExhausted)?,
            None => 1,
        };
        if message.sender_sequence < expected {
            return Err(SurfaceRecoverySequenceError::SequenceReplay);
        }
        if message.sender_sequence > expected {
            return Err(SurfaceRecoverySequenceError::SequenceGap);
        }
        *cursor = Some(message.sender_sequence);
        Ok(())
    }

    pub const fn last(self, direction: SurfaceRecoveryDirection) -> Option<u64> {
        match direction {
            SurfaceRecoveryDirection::InitToSurface => self.init_to_surface,
            SurfaceRecoveryDirection::SurfaceToInit => self.surface_to_init,
            SurfaceRecoveryDirection::SurfaceToClient => self.surface_to_client,
            SurfaceRecoveryDirection::ClientToSurface => self.client_to_surface,
        }
    }
}

const SUPERVISOR_OFFSET_MAGIC: usize = 0;
const SUPERVISOR_OFFSET_VERSION: usize = 4;
const SUPERVISOR_OFFSET_KIND: usize = 6;
const SUPERVISOR_OFFSET_HEADER_RESERVED: usize = 7;
const SUPERVISOR_OFFSET_SENDER_SEQUENCE: usize = 8;
const SUPERVISOR_OFFSET_TRANSACTION_ID: usize = 16;
const SUPERVISOR_OFFSET_INSTANCE_ID: usize = 24;
const SUPERVISOR_OFFSET_PID: usize = 32;
const SUPERVISOR_OFFSET_PID_GENERATION: usize = 36;
const SUPERVISOR_OFFSET_APP: usize = 40;
const SUPERVISOR_OFFSET_OPERATION: usize = 41;
const SUPERVISOR_OFFSET_STATUS: usize = 42;
const SUPERVISOR_OFFSET_BODY_RESERVED: usize = 43;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSupervisorControlDirection {
    InitToSurface = 1,
    SurfaceToInit = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSupervisorControlKind {
    Command = 1,
    Ack = 2,
    OwnerDied = 3,
}

impl UiSupervisorControlKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Command),
            2 => Some(Self::Ack),
            3 => Some(Self::OwnerDied),
            _ => None,
        }
    }

    pub const fn direction(self) -> UiSupervisorControlDirection {
        match self {
            Self::Command => UiSupervisorControlDirection::InitToSurface,
            Self::Ack | Self::OwnerDied => UiSupervisorControlDirection::SurfaceToInit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSupervisorOperation {
    InstallAppEndpoint = 1,
    ActivateApp = 2,
    ShowLauncher = 3,
    RetireAppEndpoint = 4,
}

impl UiSupervisorOperation {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::InstallAppEndpoint),
            2 => Some(Self::ActivateApp),
            3 => Some(Self::ShowLauncher),
            4 => Some(Self::RetireAppEndpoint),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UiSupervisorStatus {
    Applied = 1,
    Rejected = 2,
    Failed = 3,
}

impl UiSupervisorStatus {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Applied),
            2 => Some(Self::Rejected),
            3 => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSupervisorWireError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidKind,
    NonZeroHeaderReserved,
    NonZeroBodyReserved,
    ZeroSenderSequence,
    ZeroTransactionId,
    InvalidApp,
    InvalidOperation,
    InvalidStatus,
    IncompleteIdentity,
    OperationRequiresApp,
    OperationRequiresIdentity,
    ShowLauncherHasApp,
    ShowLauncherHasIdentity,
    UnexpectedStatus,
    OwnerDiedInvalidOperation,
    OwnerDiedUnexpectedStatus,
    OwnerDiedRequiresApp,
    OwnerDiedRequiresIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSupervisorControlPayload {
    Command {
        operation: UiSupervisorOperation,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    },
    Ack {
        operation: UiSupervisorOperation,
        status: UiSupervisorStatus,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    },
    OwnerDied {
        app: ShellAppId,
        identity: AppInstanceIdentity,
    },
}

/// One canonical USC1 command, acknowledgement, or owner-death report.
///
/// Both directions own a nonzero contiguous sender sequence. The transaction
/// id is the originating ALC1 transaction id. Install, activate and retire
/// carry an app plus its complete generation-qualified identity. ShowLauncher
/// carries neither. Ack repeats the exact command tuple and adds one closed
/// status value. A spontaneous SurfaceServer-to-Init OwnerDied report uses the
/// RetireAppEndpoint operation byte, zero status, and the exact installed app
/// identity. Byte 7 and bytes 43..64 are reserved zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSupervisorControlMessage {
    sender_sequence: u64,
    transaction_id: u64,
    payload: UiSupervisorControlPayload,
}

impl UiSupervisorControlMessage {
    pub fn command(
        sender_sequence: u64,
        transaction_id: u64,
        operation: UiSupervisorOperation,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<Self, UiSupervisorWireError> {
        validate_supervisor_fields(sender_sequence, transaction_id, operation, app, identity)?;
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: UiSupervisorControlPayload::Command {
                operation,
                app,
                identity,
            },
        })
    }

    pub fn ack(
        sender_sequence: u64,
        transaction_id: u64,
        operation: UiSupervisorOperation,
        status: UiSupervisorStatus,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<Self, UiSupervisorWireError> {
        validate_supervisor_fields(sender_sequence, transaction_id, operation, app, identity)?;
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: UiSupervisorControlPayload::Ack {
                operation,
                status,
                app,
                identity,
            },
        })
    }

    pub fn owner_died(
        sender_sequence: u64,
        transaction_id: u64,
        app: ShellAppId,
        identity: AppInstanceIdentity,
    ) -> Result<Self, UiSupervisorWireError> {
        validate_supervisor_counters(sender_sequence, transaction_id)?;
        Ok(Self {
            sender_sequence,
            transaction_id,
            payload: UiSupervisorControlPayload::OwnerDied { app, identity },
        })
    }

    pub const fn sender_sequence(self) -> u64 {
        self.sender_sequence
    }

    pub const fn transaction_id(self) -> u64 {
        self.transaction_id
    }

    pub const fn payload(self) -> UiSupervisorControlPayload {
        self.payload
    }

    pub const fn operation(self) -> UiSupervisorOperation {
        match self.payload {
            UiSupervisorControlPayload::Command { operation, .. }
            | UiSupervisorControlPayload::Ack { operation, .. } => operation,
            UiSupervisorControlPayload::OwnerDied { .. } => {
                UiSupervisorOperation::RetireAppEndpoint
            }
        }
    }

    pub const fn app(self) -> Option<ShellAppId> {
        match self.payload {
            UiSupervisorControlPayload::Command { app, .. }
            | UiSupervisorControlPayload::Ack { app, .. } => app,
            UiSupervisorControlPayload::OwnerDied { app, .. } => Some(app),
        }
    }

    pub const fn identity(self) -> Option<AppInstanceIdentity> {
        match self.payload {
            UiSupervisorControlPayload::Command { identity, .. }
            | UiSupervisorControlPayload::Ack { identity, .. } => identity,
            UiSupervisorControlPayload::OwnerDied { identity, .. } => Some(identity),
        }
    }

    pub const fn status(self) -> Option<UiSupervisorStatus> {
        match self.payload {
            UiSupervisorControlPayload::Command { .. }
            | UiSupervisorControlPayload::OwnerDied { .. } => None,
            UiSupervisorControlPayload::Ack { status, .. } => Some(status),
        }
    }

    pub const fn kind(self) -> UiSupervisorControlKind {
        match self.payload {
            UiSupervisorControlPayload::Command { .. } => UiSupervisorControlKind::Command,
            UiSupervisorControlPayload::Ack { .. } => UiSupervisorControlKind::Ack,
            UiSupervisorControlPayload::OwnerDied { .. } => UiSupervisorControlKind::OwnerDied,
        }
    }

    pub const fn direction(self) -> UiSupervisorControlDirection {
        self.kind().direction()
    }

    pub fn encode(self) -> [u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE] {
        let mut wire = [0_u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE];
        wire[SUPERVISOR_OFFSET_MAGIC..SUPERVISOR_OFFSET_MAGIC + 4]
            .copy_from_slice(&UI_SUPERVISOR_CONTROL_MAGIC.to_le_bytes());
        wire[SUPERVISOR_OFFSET_VERSION..SUPERVISOR_OFFSET_VERSION + 2]
            .copy_from_slice(&UI_SUPERVISOR_CONTROL_VERSION.to_le_bytes());
        wire[SUPERVISOR_OFFSET_KIND] = self.kind().raw();
        write_supervisor_u64(
            &mut wire,
            SUPERVISOR_OFFSET_SENDER_SEQUENCE,
            self.sender_sequence,
        );
        write_supervisor_u64(
            &mut wire,
            SUPERVISOR_OFFSET_TRANSACTION_ID,
            self.transaction_id,
        );
        let (operation, status, app, identity) = match self.payload {
            UiSupervisorControlPayload::Command {
                operation,
                app,
                identity,
            } => (operation, 0, app, identity),
            UiSupervisorControlPayload::Ack {
                operation,
                status,
                app,
                identity,
            } => (operation, status.raw(), app, identity),
            UiSupervisorControlPayload::OwnerDied { app, identity } => (
                UiSupervisorOperation::RetireAppEndpoint,
                0,
                Some(app),
                Some(identity),
            ),
        };
        wire[SUPERVISOR_OFFSET_OPERATION] = operation.raw();
        wire[SUPERVISOR_OFFSET_STATUS] = status;
        if let Some(app) = app {
            wire[SUPERVISOR_OFFSET_APP] = app.raw();
        }
        if let Some(identity) = identity {
            write_supervisor_u64(
                &mut wire,
                SUPERVISOR_OFFSET_INSTANCE_ID,
                identity.instance_id(),
            );
            wire[SUPERVISOR_OFFSET_PID..SUPERVISOR_OFFSET_PID + 4]
                .copy_from_slice(&identity.process().pid().to_le_bytes());
            wire[SUPERVISOR_OFFSET_PID_GENERATION..SUPERVISOR_OFFSET_PID_GENERATION + 4]
                .copy_from_slice(&identity.process().generation().to_le_bytes());
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, UiSupervisorWireError> {
        if wire.len() != UI_SUPERVISOR_CONTROL_WIRE_SIZE {
            return Err(UiSupervisorWireError::InvalidWireLength);
        }
        if read_u32(wire, SUPERVISOR_OFFSET_MAGIC) != UI_SUPERVISOR_CONTROL_MAGIC {
            return Err(UiSupervisorWireError::InvalidMagic);
        }
        if read_u16(wire, SUPERVISOR_OFFSET_VERSION) != UI_SUPERVISOR_CONTROL_VERSION {
            return Err(UiSupervisorWireError::InvalidVersion);
        }
        let kind = UiSupervisorControlKind::from_raw(wire[SUPERVISOR_OFFSET_KIND])
            .ok_or(UiSupervisorWireError::InvalidKind)?;
        if wire[SUPERVISOR_OFFSET_HEADER_RESERVED] != 0 {
            return Err(UiSupervisorWireError::NonZeroHeaderReserved);
        }
        if wire[SUPERVISOR_OFFSET_BODY_RESERVED..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(UiSupervisorWireError::NonZeroBodyReserved);
        }
        let sender_sequence = read_u64(wire, SUPERVISOR_OFFSET_SENDER_SEQUENCE);
        let transaction_id = read_u64(wire, SUPERVISOR_OFFSET_TRANSACTION_ID);
        let operation_raw = wire[SUPERVISOR_OFFSET_OPERATION];
        let app = match wire[SUPERVISOR_OFFSET_APP] {
            0 => None,
            raw => Some(
                ShellAppId::from_raw(u64::from(raw)).ok_or(UiSupervisorWireError::InvalidApp)?,
            ),
        };
        let identity = decode_supervisor_identity(wire)?;
        let status = wire[SUPERVISOR_OFFSET_STATUS];
        match kind {
            UiSupervisorControlKind::Command => {
                let operation = UiSupervisorOperation::from_raw(operation_raw)
                    .ok_or(UiSupervisorWireError::InvalidOperation)?;
                if status != 0 {
                    return Err(UiSupervisorWireError::UnexpectedStatus);
                }
                Self::command(sender_sequence, transaction_id, operation, app, identity)
            }
            UiSupervisorControlKind::Ack => {
                let operation = UiSupervisorOperation::from_raw(operation_raw)
                    .ok_or(UiSupervisorWireError::InvalidOperation)?;
                let status = UiSupervisorStatus::from_raw(status)
                    .ok_or(UiSupervisorWireError::InvalidStatus)?;
                Self::ack(
                    sender_sequence,
                    transaction_id,
                    operation,
                    status,
                    app,
                    identity,
                )
            }
            UiSupervisorControlKind::OwnerDied => {
                if operation_raw != UiSupervisorOperation::RetireAppEndpoint.raw() {
                    return Err(UiSupervisorWireError::OwnerDiedInvalidOperation);
                }
                if status != 0 {
                    return Err(UiSupervisorWireError::OwnerDiedUnexpectedStatus);
                }
                let app = app.ok_or(UiSupervisorWireError::OwnerDiedRequiresApp)?;
                let identity = identity.ok_or(UiSupervisorWireError::OwnerDiedRequiresIdentity)?;
                Self::owner_died(sender_sequence, transaction_id, app, identity)
            }
        }
    }
}

fn validate_supervisor_counters(
    sender_sequence: u64,
    transaction_id: u64,
) -> Result<(), UiSupervisorWireError> {
    if sender_sequence == 0 {
        return Err(UiSupervisorWireError::ZeroSenderSequence);
    }
    if transaction_id == 0 {
        return Err(UiSupervisorWireError::ZeroTransactionId);
    }
    Ok(())
}

fn validate_supervisor_fields(
    sender_sequence: u64,
    transaction_id: u64,
    operation: UiSupervisorOperation,
    app: Option<ShellAppId>,
    identity: Option<AppInstanceIdentity>,
) -> Result<(), UiSupervisorWireError> {
    validate_supervisor_counters(sender_sequence, transaction_id)?;
    if operation == UiSupervisorOperation::ShowLauncher {
        if app.is_some() {
            return Err(UiSupervisorWireError::ShowLauncherHasApp);
        }
        if identity.is_some() {
            return Err(UiSupervisorWireError::ShowLauncherHasIdentity);
        }
    } else {
        if app.is_none() {
            return Err(UiSupervisorWireError::OperationRequiresApp);
        }
        if identity.is_none() {
            return Err(UiSupervisorWireError::OperationRequiresIdentity);
        }
    }
    Ok(())
}

fn decode_supervisor_identity(
    wire: &[u8],
) -> Result<Option<AppInstanceIdentity>, UiSupervisorWireError> {
    let instance_id = read_u64(wire, SUPERVISOR_OFFSET_INSTANCE_ID);
    let pid = read_u32(wire, SUPERVISOR_OFFSET_PID);
    let pid_generation = read_u32(wire, SUPERVISOR_OFFSET_PID_GENERATION);
    match (instance_id, pid, pid_generation) {
        (0, 0, 0) => Ok(None),
        (0, _, _) | (_, 0, _) | (_, _, 0) => Err(UiSupervisorWireError::IncompleteIdentity),
        _ => Ok(Some(
            AppInstanceIdentity::try_new(instance_id, pid, pid_generation)
                .map_err(|_| UiSupervisorWireError::IncompleteIdentity)?,
        )),
    }
}

fn write_supervisor_u64(
    wire: &mut [u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE],
    offset: usize,
    value: u64,
) {
    wire[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSupervisorSequenceError {
    SequenceExhausted,
    SequenceReplay,
    SequenceGap,
}

/// Independent contiguous receive cursors for the two USC1 directions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiSupervisorSequenceTracker {
    init_to_surface: Option<u64>,
    surface_to_init: Option<u64>,
}

impl UiSupervisorSequenceTracker {
    pub const fn new() -> Self {
        Self {
            init_to_surface: None,
            surface_to_init: None,
        }
    }

    pub fn accept(
        &mut self,
        message: UiSupervisorControlMessage,
    ) -> Result<(), UiSupervisorSequenceError> {
        let cursor = match message.direction() {
            UiSupervisorControlDirection::InitToSurface => &mut self.init_to_surface,
            UiSupervisorControlDirection::SurfaceToInit => &mut self.surface_to_init,
        };
        let expected = match *cursor {
            Some(previous) => previous
                .checked_add(1)
                .ok_or(UiSupervisorSequenceError::SequenceExhausted)?,
            None => 1,
        };
        if message.sender_sequence() < expected {
            return Err(UiSupervisorSequenceError::SequenceReplay);
        }
        if message.sender_sequence() > expected {
            return Err(UiSupervisorSequenceError::SequenceGap);
        }
        *cursor = Some(message.sender_sequence());
        Ok(())
    }

    pub const fn last(self, direction: UiSupervisorControlDirection) -> Option<u64> {
        match direction {
            UiSupervisorControlDirection::InitToSurface => self.init_to_surface,
            UiSupervisorControlDirection::SurfaceToInit => self.surface_to_init,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiSupervisorActiveTarget {
    #[default]
    Launcher,
    App {
        app: ShellAppId,
        identity: AppInstanceIdentity,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSupervisorTransactionError {
    TransactionExhausted,
    TransactionReplay,
    TransactionBusy,
    AckWithoutCommand,
    AckReplay,
    TransactionMismatch,
    AckMismatch,
    EndpointAlreadyInstalled,
    NoInstalledEndpoint,
    AppMismatch,
    StaleIdentity,
    InvalidStateForOperation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingUiSupervisorTransaction {
    transaction_id: u64,
    operation: UiSupervisorOperation,
    app: Option<ShellAppId>,
    identity: Option<AppInstanceIdentity>,
}

/// Correlates USC1 commands and acknowledgements and models the endpoint/focus
/// state which may be committed only by an Applied acknowledgement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSupervisorTransactionTracker {
    pending: Option<PendingUiSupervisorTransaction>,
    last_transaction_id: Option<u64>,
    installed_app: Option<ShellAppId>,
    installed_identity: Option<AppInstanceIdentity>,
    retired_identity: Option<AppInstanceIdentity>,
    last_instance_id: Option<u64>,
    active_target: UiSupervisorActiveTarget,
}

impl Default for UiSupervisorTransactionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UiSupervisorTransactionTracker {
    pub const fn new() -> Self {
        Self {
            pending: None,
            last_transaction_id: None,
            installed_app: None,
            installed_identity: None,
            retired_identity: None,
            last_instance_id: None,
            active_target: UiSupervisorActiveTarget::Launcher,
        }
    }

    pub fn accept(
        &mut self,
        message: UiSupervisorControlMessage,
    ) -> Result<(), UiSupervisorTransactionError> {
        match message.payload() {
            UiSupervisorControlPayload::Command {
                operation,
                app,
                identity,
            } => self.accept_command(message.transaction_id(), operation, app, identity),
            UiSupervisorControlPayload::Ack {
                operation,
                status,
                app,
                identity,
            } => self.accept_ack(message.transaction_id(), operation, status, app, identity),
            UiSupervisorControlPayload::OwnerDied { app, identity } => {
                self.accept_owner_died(message.transaction_id(), app, identity)
            }
        }
    }

    fn accept_owner_died(
        &mut self,
        transaction_id: u64,
        app: ShellAppId,
        identity: AppInstanceIdentity,
    ) -> Result<(), UiSupervisorTransactionError> {
        if self.pending.is_some() {
            return Err(UiSupervisorTransactionError::TransactionBusy);
        }
        self.require_new_transaction(transaction_id)?;
        self.require_installed(Some(app), Some(identity))?;

        self.retired_identity = Some(identity);
        self.installed_app = None;
        self.installed_identity = None;
        self.active_target = UiSupervisorActiveTarget::Launcher;
        self.last_transaction_id = Some(transaction_id);
        Ok(())
    }

    fn accept_command(
        &mut self,
        transaction_id: u64,
        operation: UiSupervisorOperation,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), UiSupervisorTransactionError> {
        if self.pending.is_some() {
            return Err(UiSupervisorTransactionError::TransactionBusy);
        }
        self.require_new_transaction(transaction_id)?;
        match operation {
            UiSupervisorOperation::InstallAppEndpoint => {
                if self.installed_identity.is_some() {
                    return Err(UiSupervisorTransactionError::EndpointAlreadyInstalled);
                }
                let identity = identity.expect("wire validation requires an install identity");
                if self
                    .last_instance_id
                    .is_some_and(|previous| identity.instance_id() <= previous)
                    || self.retired_identity.is_some_and(|retired| {
                        retired.instance_id() == identity.instance_id()
                            || retired.process() == identity.process()
                    })
                {
                    return Err(UiSupervisorTransactionError::StaleIdentity);
                }
            }
            UiSupervisorOperation::ActivateApp => {
                self.require_installed(app, identity)?;
                if self.active_target
                    == (UiSupervisorActiveTarget::App {
                        app: app.expect("wire validation requires an app"),
                        identity: identity.expect("wire validation requires an identity"),
                    })
                {
                    return Err(UiSupervisorTransactionError::InvalidStateForOperation);
                }
            }
            UiSupervisorOperation::ShowLauncher => {
                if self.active_target == UiSupervisorActiveTarget::Launcher {
                    return Err(UiSupervisorTransactionError::InvalidStateForOperation);
                }
            }
            UiSupervisorOperation::RetireAppEndpoint => {
                self.require_installed(app, identity)?;
            }
        }
        self.pending = Some(PendingUiSupervisorTransaction {
            transaction_id,
            operation,
            app,
            identity,
        });
        Ok(())
    }

    fn accept_ack(
        &mut self,
        transaction_id: u64,
        operation: UiSupervisorOperation,
        status: UiSupervisorStatus,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), UiSupervisorTransactionError> {
        let Some(pending) = self.pending else {
            if self
                .last_transaction_id
                .is_some_and(|previous| transaction_id <= previous)
            {
                return Err(UiSupervisorTransactionError::AckReplay);
            }
            return Err(UiSupervisorTransactionError::AckWithoutCommand);
        };
        if transaction_id != pending.transaction_id {
            return Err(UiSupervisorTransactionError::TransactionMismatch);
        }
        if operation != pending.operation || app != pending.app || identity != pending.identity {
            return Err(UiSupervisorTransactionError::AckMismatch);
        }
        if status == UiSupervisorStatus::Applied {
            self.apply(pending);
        }
        self.pending = None;
        self.last_transaction_id = Some(transaction_id);
        Ok(())
    }

    fn apply(&mut self, pending: PendingUiSupervisorTransaction) {
        match pending.operation {
            UiSupervisorOperation::InstallAppEndpoint => {
                let app = pending.app.expect("validated install app");
                let identity = pending.identity.expect("validated install identity");
                self.installed_app = Some(app);
                self.installed_identity = Some(identity);
                self.last_instance_id = Some(identity.instance_id());
            }
            UiSupervisorOperation::ActivateApp => {
                self.active_target = UiSupervisorActiveTarget::App {
                    app: pending.app.expect("validated activate app"),
                    identity: pending.identity.expect("validated activate identity"),
                };
            }
            UiSupervisorOperation::ShowLauncher => {
                self.active_target = UiSupervisorActiveTarget::Launcher;
            }
            UiSupervisorOperation::RetireAppEndpoint => {
                self.retired_identity = pending.identity;
                self.installed_app = None;
                self.installed_identity = None;
                self.active_target = UiSupervisorActiveTarget::Launcher;
            }
        }
    }

    fn require_installed(
        self,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> Result<(), UiSupervisorTransactionError> {
        let Some(installed_identity) = self.installed_identity else {
            return Err(UiSupervisorTransactionError::NoInstalledEndpoint);
        };
        if app != self.installed_app {
            return Err(UiSupervisorTransactionError::AppMismatch);
        }
        if identity != Some(installed_identity) {
            return Err(UiSupervisorTransactionError::StaleIdentity);
        }
        Ok(())
    }

    fn require_new_transaction(
        self,
        transaction_id: u64,
    ) -> Result<(), UiSupervisorTransactionError> {
        if transaction_id == 0 {
            return Err(UiSupervisorTransactionError::TransactionReplay);
        }
        if let Some(previous) = self.last_transaction_id {
            if previous == u64::MAX {
                return Err(UiSupervisorTransactionError::TransactionExhausted);
            }
            if transaction_id <= previous {
                return Err(UiSupervisorTransactionError::TransactionReplay);
            }
        }
        Ok(())
    }

    pub const fn pending_transaction_id(self) -> Option<u64> {
        match self.pending {
            Some(pending) => Some(pending.transaction_id),
            None => None,
        }
    }

    pub const fn last_transaction_id(self) -> Option<u64> {
        self.last_transaction_id
    }

    pub const fn installed_app(self) -> Option<ShellAppId> {
        self.installed_app
    }

    pub const fn installed_identity(self) -> Option<AppInstanceIdentity> {
        self.installed_identity
    }

    pub const fn retired_identity(self) -> Option<AppInstanceIdentity> {
        self.retired_identity
    }

    pub const fn active_target(self) -> UiSupervisorActiveTarget {
        self.active_target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSupervisorTrackerError {
    Sequence(UiSupervisorSequenceError),
    Transaction(UiSupervisorTransactionError),
}

/// Atomically combines USC1 direction sequencing and transaction correlation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiSupervisorTracker {
    sequence: UiSupervisorSequenceTracker,
    transaction: UiSupervisorTransactionTracker,
}

impl UiSupervisorTracker {
    pub const fn new() -> Self {
        Self {
            sequence: UiSupervisorSequenceTracker::new(),
            transaction: UiSupervisorTransactionTracker::new(),
        }
    }

    pub fn accept(
        &mut self,
        message: UiSupervisorControlMessage,
    ) -> Result<(), UiSupervisorTrackerError> {
        let mut next = *self;
        next.sequence
            .accept(message)
            .map_err(UiSupervisorTrackerError::Sequence)?;
        next.transaction
            .accept(message)
            .map_err(UiSupervisorTrackerError::Transaction)?;
        *self = next;
        Ok(())
    }

    pub const fn sequence(self) -> UiSupervisorSequenceTracker {
        self.sequence
    }

    pub const fn transaction(self) -> UiSupervisorTransactionTracker {
        self.transaction
    }
}

const OFFSET_MAGIC: usize = 0;
const OFFSET_VERSION: usize = 4;
const OFFSET_OPCODE: usize = 6;
const OFFSET_MODE: usize = 7;
const OFFSET_SURFACE_ID: usize = 8;
const OFFSET_RECT_COUNT: usize = 10;
const OFFSET_HEADER_RESERVED: usize = 11;
const OFFSET_FRAME_ID: usize = 12;
const OFFSET_CLEAR_COLOR: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PresentMode {
    Full = 1,
    Damage = 2,
}

impl PresentMode {
    const fn from_wire(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Full),
            2 => Some(Self::Damage),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidWireLength,
    InvalidMagic,
    InvalidVersion,
    InvalidOpcode,
    InvalidMode,
    InvalidSurfaceId,
    NonZeroHeaderReserved,
    TooManyRects,
    EmptyDamage,
    ZeroFrameId,
    NonCanonicalColor,
    ZeroRectExtent,
    RectOutOfBounds,
    NonZeroRectReserved,
    NonZeroUnusedRect,
    ZeroFocusGeneration,
    FirstFrameMustBeFull,
    FirstFrameIdMustBeOne,
    FrameIdExhausted,
    FrameReplay,
    FrameGap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SolidRect {
    x: u8,
    width: u8,
    y: u16,
    height: u16,
    color: u32,
}

impl SolidRect {
    const EMPTY: Self = Self {
        x: 0,
        width: 0,
        y: 0,
        height: 0,
        color: 0,
    };

    pub fn try_new(
        x: u8,
        y: u16,
        width: u8,
        height: u16,
        color: u32,
    ) -> Result<Self, ProtocolError> {
        if width == 0 || height == 0 {
            return Err(ProtocolError::ZeroRectExtent);
        }
        let right = u16::from(x)
            .checked_add(u16::from(width))
            .ok_or(ProtocolError::RectOutOfBounds)?;
        let bottom = y
            .checked_add(height)
            .ok_or(ProtocolError::RectOutOfBounds)?;
        if right > SURFACE_WIDTH || bottom > SURFACE_HEIGHT {
            return Err(ProtocolError::RectOutOfBounds);
        }
        validate_color(color)?;
        Ok(Self {
            x,
            width,
            y,
            height,
            color,
        })
    }

    pub const fn x(self) -> u8 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }

    pub const fn width(self) -> u8 {
        self.width
    }

    pub const fn height(self) -> u16 {
        self.height
    }

    pub const fn color(self) -> u32 {
        self.color
    }

    pub const fn damage(self) -> DamageRect {
        DamageRect {
            x: self.x as u16,
            y: self.y,
            width: self.width as u16,
            height: self.height,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl DamageRect {
    pub const FULL: Self = Self {
        x: 0,
        y: 0,
        width: SURFACE_WIDTH,
        height: SURFACE_HEIGHT,
    };

    pub fn union(self, other: Self) -> Self {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = self
            .x
            .saturating_add(self.width)
            .max(other.x.saturating_add(other.width));
        let bottom = self
            .y
            .saturating_add(self.height)
            .max(other.y.saturating_add(other.height));
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

/// One canonical 64-byte present transaction. Protocol v2 stores an optional
/// client focus generation in the tail byte of each fixed rect slot. Those
/// four bytes form one little-endian `u32`; every other byte in an unused rect
/// slot remains zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentFrame {
    mode: PresentMode,
    frame_id: u32,
    focus_generation: u32,
    clear_color: u32,
    rect_count: u8,
    rects: [SolidRect; MAX_SOLID_RECTS],
}

impl PresentFrame {
    pub fn full(
        frame_id: u32,
        clear_color: u32,
        rects: &[SolidRect],
    ) -> Result<Self, ProtocolError> {
        Self::new(PresentMode::Full, frame_id, clear_color, rects)
    }

    pub fn damage(frame_id: u32, rects: &[SolidRect]) -> Result<Self, ProtocolError> {
        Self::new(PresentMode::Damage, frame_id, 0, rects)
    }

    fn new(
        mode: PresentMode,
        frame_id: u32,
        clear_color: u32,
        rects: &[SolidRect],
    ) -> Result<Self, ProtocolError> {
        if frame_id == 0 {
            return Err(ProtocolError::ZeroFrameId);
        }
        if rects.len() > MAX_SOLID_RECTS {
            return Err(ProtocolError::TooManyRects);
        }
        if mode == PresentMode::Damage && rects.is_empty() {
            return Err(ProtocolError::EmptyDamage);
        }
        if mode == PresentMode::Damage && clear_color != 0 {
            return Err(ProtocolError::NonCanonicalColor);
        }
        validate_color(clear_color)?;
        let mut canonical_rects = [SolidRect::EMPTY; MAX_SOLID_RECTS];
        canonical_rects[..rects.len()].copy_from_slice(rects);
        Ok(Self {
            mode,
            frame_id,
            focus_generation: 0,
            clear_color,
            rect_count: rects.len() as u8,
            rects: canonical_rects,
        })
    }

    pub const fn mode(&self) -> PresentMode {
        self.mode
    }

    pub const fn frame_id(&self) -> u32 {
        self.frame_id
    }

    /// Tags a client-to-SurfaceServer frame with the nonzero focus generation
    /// under which it was produced. Kernel-facing frames retain the canonical
    /// zero generation returned by `full` and `damage`.
    pub fn with_focus_generation(mut self, focus_generation: u32) -> Result<Self, ProtocolError> {
        if focus_generation == 0 {
            return Err(ProtocolError::ZeroFocusGeneration);
        }
        self.focus_generation = focus_generation;
        Ok(self)
    }

    pub const fn focus_generation(&self) -> u32 {
        self.focus_generation
    }

    pub const fn clear_color(&self) -> u32 {
        self.clear_color
    }

    pub const fn rect_count(&self) -> usize {
        self.rect_count as usize
    }

    pub fn rects(&self) -> &[SolidRect] {
        &self.rects[..self.rect_count()]
    }

    pub fn damage_rect(&self) -> DamageRect {
        if self.mode == PresentMode::Full {
            return DamageRect::FULL;
        }
        let mut damage = self.rects[0].damage();
        for rect in &self.rects[1..self.rect_count()] {
            damage = damage.union(rect.damage());
        }
        damage
    }

    pub fn encode(&self) -> [u8; PRESENT_WIRE_SIZE] {
        let mut wire = [0_u8; PRESENT_WIRE_SIZE];
        wire[OFFSET_MAGIC..OFFSET_MAGIC + 4].copy_from_slice(&PRESENT_MAGIC.to_le_bytes());
        wire[OFFSET_VERSION..OFFSET_VERSION + 2].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        wire[OFFSET_OPCODE] = PRESENT_OPCODE;
        wire[OFFSET_MODE] = self.mode as u8;
        wire[OFFSET_SURFACE_ID..OFFSET_SURFACE_ID + 2].copy_from_slice(&SURFACE_ID.to_le_bytes());
        wire[OFFSET_RECT_COUNT] = self.rect_count;
        wire[OFFSET_FRAME_ID..OFFSET_FRAME_ID + 4].copy_from_slice(&self.frame_id.to_le_bytes());
        wire[OFFSET_CLEAR_COLOR..OFFSET_CLEAR_COLOR + 4]
            .copy_from_slice(&self.clear_color.to_le_bytes());
        for (index, rect) in self.rects().iter().enumerate() {
            let offset = rect_offset(index);
            wire[offset] = rect.x;
            wire[offset + 1] = rect.width;
            wire[offset + 2..offset + 4].copy_from_slice(&rect.y.to_le_bytes());
            wire[offset + 4..offset + 6].copy_from_slice(&rect.height.to_le_bytes());
            wire[offset + 6..offset + 10].copy_from_slice(&rect.color.to_le_bytes());
        }
        let focus_generation = self.focus_generation.to_le_bytes();
        for (index, byte) in focus_generation.iter().enumerate() {
            wire[rect_offset(index) + 10] = *byte;
        }
        wire
    }

    pub fn decode(wire: &[u8]) -> Result<Self, ProtocolError> {
        if wire.len() != PRESENT_WIRE_SIZE {
            return Err(ProtocolError::InvalidWireLength);
        }
        if read_u32(wire, OFFSET_MAGIC) != PRESENT_MAGIC {
            return Err(ProtocolError::InvalidMagic);
        }
        if read_u16(wire, OFFSET_VERSION) != PROTOCOL_VERSION {
            return Err(ProtocolError::InvalidVersion);
        }
        if wire[OFFSET_OPCODE] != PRESENT_OPCODE {
            return Err(ProtocolError::InvalidOpcode);
        }
        let mode = PresentMode::from_wire(wire[OFFSET_MODE]).ok_or(ProtocolError::InvalidMode)?;
        if read_u16(wire, OFFSET_SURFACE_ID) != SURFACE_ID {
            return Err(ProtocolError::InvalidSurfaceId);
        }
        if wire[OFFSET_HEADER_RESERVED] != 0 {
            return Err(ProtocolError::NonZeroHeaderReserved);
        }
        let rect_count = usize::from(wire[OFFSET_RECT_COUNT]);
        if rect_count > MAX_SOLID_RECTS {
            return Err(ProtocolError::TooManyRects);
        }
        let frame_id = read_u32(wire, OFFSET_FRAME_ID);
        let clear_color = read_u32(wire, OFFSET_CLEAR_COLOR);
        let focus_generation = u32::from_le_bytes([
            wire[rect_offset(0) + 10],
            wire[rect_offset(1) + 10],
            wire[rect_offset(2) + 10],
            wire[rect_offset(3) + 10],
        ]);
        let mut rects = [SolidRect::EMPTY; MAX_SOLID_RECTS];
        for (index, slot) in rects.iter_mut().enumerate() {
            let offset = rect_offset(index);
            let bytes = &wire[offset..offset + SOLID_RECT_WIRE_SIZE];
            if index >= rect_count {
                if bytes[..10].iter().any(|byte| *byte != 0) {
                    return Err(ProtocolError::NonZeroUnusedRect);
                }
                continue;
            }
            *slot = SolidRect::try_new(
                bytes[0],
                read_u16(bytes, 2),
                bytes[1],
                read_u16(bytes, 4),
                read_u32(bytes, 6),
            )?;
        }
        let mut frame = Self::new(mode, frame_id, clear_color, &rects[..rect_count])?;
        frame.focus_generation = focus_generation;
        Ok(frame)
    }
}

pub fn validate_sequence(
    previous_frame_id: Option<u32>,
    frame: &PresentFrame,
) -> Result<(), ProtocolError> {
    let Some(previous) = previous_frame_id else {
        if frame.mode != PresentMode::Full {
            return Err(ProtocolError::FirstFrameMustBeFull);
        }
        if frame.frame_id != 1 {
            return Err(ProtocolError::FirstFrameIdMustBeOne);
        }
        return Ok(());
    };
    let expected = previous
        .checked_add(1)
        .ok_or(ProtocolError::FrameIdExhausted)?;
    if frame.frame_id < expected {
        return Err(ProtocolError::FrameReplay);
    }
    if frame.frame_id > expected {
        return Err(ProtocolError::FrameGap);
    }
    Ok(())
}

const fn rect_offset(index: usize) -> usize {
    PRESENT_HEADER_SIZE + index * SOLID_RECT_WIRE_SIZE
}

fn validate_color(color: u32) -> Result<(), ProtocolError> {
    if color & 0xff00_0000 != 0 {
        return Err(ProtocolError::NonCanonicalColor);
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submitted_buffer_present(present: BufferPresent) -> BufferPresent {
        #[cfg(feature = "mobile-system-chrome0")]
        {
            present
                .with_system_chrome_generation(0x2122_2324_2526_2728)
                .unwrap()
        }
        #[cfg(not(feature = "mobile-system-chrome0"))]
        {
            present
        }
    }

    fn rect(x: u8, y: u16, width: u8, height: u16, color: u32) -> SolidRect {
        SolidRect::try_new(x, y, width, height, color).unwrap()
    }

    fn input_event(session_id: u64, sequence: u64) -> UiServerEvent {
        let sample = InputSample::try_new(sequence, 12, 34, sequence & 1 != 0).unwrap();
        UiServerEvent::input_sample(session_id, sample).unwrap()
    }

    fn app_identity(instance_id: u64, pid: u32, generation: u32) -> AppInstanceIdentity {
        AppInstanceIdentity::try_new(instance_id, pid, generation).unwrap()
    }

    fn lifecycle_request(
        sequence: u64,
        transaction: u64,
        action: AppLifecycleAction,
        identity: Option<AppInstanceIdentity>,
    ) -> AppLifecycleMessage {
        AppLifecycleMessage::request(
            sequence,
            transaction,
            ShellAppId::Settings,
            action,
            identity,
        )
        .unwrap()
    }

    fn lifecycle_state(
        sequence: u64,
        transaction: u64,
        state: AppLifecycleState,
        reason: AppLifecycleReason,
        identity: Option<AppInstanceIdentity>,
    ) -> AppLifecycleMessage {
        AppLifecycleMessage::state_changed(
            sequence,
            transaction,
            ShellAppId::Settings,
            state,
            reason,
            identity,
        )
        .unwrap()
    }

    fn lifecycle_command(
        sequence: u64,
        transaction: u64,
        action: AppLifecycleAction,
        identity: AppInstanceIdentity,
    ) -> AppLifecycleMessage {
        AppLifecycleMessage::command(
            sequence,
            transaction,
            ShellAppId::Settings,
            action,
            identity,
        )
        .unwrap()
    }

    fn lifecycle_ack(
        sequence: u64,
        transaction: u64,
        action: AppLifecycleAction,
        status: AppLifecycleStatus,
        identity: AppInstanceIdentity,
    ) -> AppLifecycleMessage {
        AppLifecycleMessage::ack(
            sequence,
            transaction,
            ShellAppId::Settings,
            action,
            status,
            identity,
        )
        .unwrap()
    }

    fn supervisor_command(
        sequence: u64,
        transaction: u64,
        operation: UiSupervisorOperation,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> UiSupervisorControlMessage {
        UiSupervisorControlMessage::command(sequence, transaction, operation, app, identity)
            .unwrap()
    }

    fn supervisor_ack(
        sequence: u64,
        transaction: u64,
        operation: UiSupervisorOperation,
        status: UiSupervisorStatus,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) -> UiSupervisorControlMessage {
        UiSupervisorControlMessage::ack(sequence, transaction, operation, status, app, identity)
            .unwrap()
    }

    fn supervisor_owner_died(
        sequence: u64,
        transaction: u64,
        app: ShellAppId,
        identity: AppInstanceIdentity,
    ) -> UiSupervisorControlMessage {
        UiSupervisorControlMessage::owner_died(sequence, transaction, app, identity).unwrap()
    }

    fn accept_supervisor_pair(
        tracker: &mut UiSupervisorTracker,
        sequence: u64,
        transaction: u64,
        operation: UiSupervisorOperation,
        status: UiSupervisorStatus,
        app: Option<ShellAppId>,
        identity: Option<AppInstanceIdentity>,
    ) {
        let command = supervisor_command(sequence, transaction, operation, app, identity);
        let command = UiSupervisorControlMessage::decode(&command.encode()).unwrap();
        tracker.accept(command).unwrap();
        let ack = supervisor_ack(sequence, transaction, operation, status, app, identity);
        let ack = UiSupervisorControlMessage::decode(&ack.encode()).unwrap();
        tracker.accept(ack).unwrap();
    }

    fn test_window_id() -> WindowId {
        WindowId::try_new(1, 0x1122_3344).unwrap()
    }

    fn test_window_bounds() -> ShellRect {
        ShellRect::new(48, 80, 112, 160)
    }

    fn assert_window_event_rejected(
        tracker: &mut WindowEventTracker,
        event: WindowEvent,
        expected: WindowEventTrackerError,
    ) {
        let before = *tracker;
        assert_eq!(tracker.accept(event), Err(expected));
        assert_eq!(*tracker, before);
    }

    #[test]
    fn window_id_token_is_generation_qualified_closed_and_canonical() {
        assert_eq!(WINDOW_CAPACITY, 2);
        let first = WindowId::try_new(0, 0x1122_3344).unwrap();
        let second = WindowId::try_new(1, 0xaabb_ccdd).unwrap();
        assert_eq!(first.token(), 0x1122_3344_0000_0000);
        assert_eq!(second.token(), 0xaabb_ccdd_0000_0001);
        assert_eq!(WindowId::from_token(first.token()), Ok(first));
        assert_eq!(WindowId::from_token(second.token()), Ok(second));
        assert_eq!(first.slot(), 0);
        assert_eq!(second.slot(), 1);
        assert_eq!(first.generation(), 0x1122_3344);
        assert_eq!(second.generation(), 0xaabb_ccdd);
        assert_eq!(
            WindowId::try_new(WINDOW_CAPACITY, 1),
            Err(WindowIdError::SlotOutOfRange)
        );
        assert_eq!(WindowId::try_new(0, 0), Err(WindowIdError::ZeroGeneration));
        assert_eq!(WindowId::from_token(0), Err(WindowIdError::ZeroGeneration));
        assert_eq!(WindowId::from_token(1), Err(WindowIdError::ZeroGeneration));
        for token in [0x0000_0001_0000_0002, 0x0000_0001_0000_0100] {
            assert_eq!(
                WindowId::from_token(token),
                Err(WindowIdError::SlotOutOfRange)
            );
        }
    }

    #[test]
    fn window_command_wire_is_exact_little_endian_and_all_opcodes_roundtrip() {
        assert_eq!(WINDOW_COMMAND_WIRE_SIZE, 64);
        let id = test_window_id();
        let create =
            WindowCommand::create(0x0102_0304_0506_0708, ShellRect::new(1, 2, 3, 4)).unwrap();
        let present = WindowCommand::present(
            0x1112_1314_1516_1718,
            id,
            0x2122_2324,
            ShellRect::new(5, 6, 7, 8),
            0x0011_2233,
        )
        .unwrap();
        let raised = WindowCommand::raise(0x3132_3334_3536_3738, id).unwrap();
        let destroyed = WindowCommand::destroy(0x4142_4344_4546_4748, id).unwrap();

        for command in [create, present, raised, destroyed] {
            let wire = command.encode();
            assert_eq!(&wire[0..4], b"BWC1");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(wire[6], command.opcode().raw());
            assert_eq!(wire[7], 0);
            assert_eq!(&wire[8..16], &command.sequence().to_le_bytes());
            assert!(wire[48..].iter().all(|byte| *byte == 0));
            assert_eq!(WindowCommand::decode(&wire), Ok(command));
        }

        let create_wire = create.encode();
        assert!(create_wire[16..28].iter().all(|byte| *byte == 0));
        assert_eq!(&create_wire[28..30], &1_u16.to_le_bytes());
        assert_eq!(&create_wire[30..32], &2_u16.to_le_bytes());
        assert_eq!(&create_wire[32..34], &3_u16.to_le_bytes());
        assert_eq!(&create_wire[34..36], &4_u16.to_le_bytes());
        assert!(create_wire[36..48].iter().all(|byte| *byte == 0));

        let present_wire = present.encode();
        assert_eq!(&present_wire[16..24], &id.token().to_le_bytes());
        assert_eq!(&present_wire[24..28], &0x2122_2324_u32.to_le_bytes());
        assert!(present_wire[28..36].iter().all(|byte| *byte == 0));
        assert_eq!(&present_wire[36..38], &5_u16.to_le_bytes());
        assert_eq!(&present_wire[38..40], &6_u16.to_le_bytes());
        assert_eq!(&present_wire[40..42], &7_u16.to_le_bytes());
        assert_eq!(&present_wire[42..44], &8_u16.to_le_bytes());
        assert_eq!(&present_wire[44..48], &0x0011_2233_u32.to_le_bytes());
        for command in [raised, destroyed] {
            let wire = command.encode();
            assert_eq!(&wire[16..24], &id.token().to_le_bytes());
            assert!(wire[24..].iter().all(|byte| *byte == 0));
        }

        for (raw, opcode) in [
            (1, WindowCommandOpcode::Create),
            (2, WindowCommandOpcode::Present),
            (3, WindowCommandOpcode::Raise),
            (4, WindowCommandOpcode::Destroy),
        ] {
            assert_eq!(WindowCommandOpcode::from_raw(raw), Some(opcode));
        }
        assert_eq!(WindowCommandOpcode::from_raw(0), None);
        assert_eq!(WindowCommandOpcode::from_raw(5), None);
    }

    #[test]
    fn window_command_constructors_reject_sequences_geometry_frames_and_colors() {
        let id = test_window_id();
        let bounds = test_window_bounds();
        assert_eq!(
            WindowCommand::create(0, bounds),
            Err(WindowCommandError::ZeroSequence)
        );
        assert_eq!(
            WindowCommand::present(0, id, 1, bounds, 0),
            Err(WindowCommandError::ZeroSequence)
        );
        assert_eq!(
            WindowCommand::raise(0, id),
            Err(WindowCommandError::ZeroSequence)
        );
        assert_eq!(
            WindowCommand::destroy(0, id),
            Err(WindowCommandError::ZeroSequence)
        );
        for invalid in [ShellRect::new(0, 0, 0, 1), ShellRect::new(0, 0, 1, 0)] {
            assert_eq!(
                WindowCommand::create(1, invalid),
                Err(WindowCommandError::InvalidGeometry(
                    WindowGeometryError::Empty
                ))
            );
        }
        for invalid in [
            ShellRect::new(SURFACE_WIDTH - 1, 0, 2, 1),
            ShellRect::new(0, SURFACE_HEIGHT - 1, 1, 2),
            ShellRect::new(u16::MAX, 0, 2, 1),
        ] {
            assert_eq!(
                WindowCommand::create(1, invalid),
                Err(WindowCommandError::InvalidGeometry(
                    WindowGeometryError::OutOfBounds
                ))
            );
        }
        assert!(
            WindowCommand::create(
                1,
                ShellRect::new(SURFACE_WIDTH - 1, SURFACE_HEIGHT - 1, 1, 1)
            )
            .is_ok()
        );
        assert_eq!(
            WindowCommand::present(1, id, 0, bounds, 0),
            Err(WindowCommandError::ZeroFrameId)
        );
        assert_eq!(
            WindowCommand::present(1, id, 1, ShellRect::new(0, 0, 0, 1), 0),
            Err(WindowCommandError::InvalidGeometry(
                WindowGeometryError::Empty
            ))
        );
        assert_eq!(
            WindowCommand::present(1, id, 1, ShellRect::new(SURFACE_WIDTH - 1, 0, 2, 1), 0,),
            Err(WindowCommandError::InvalidGeometry(
                WindowGeometryError::OutOfBounds
            ))
        );
        assert_eq!(
            WindowCommand::present(1, id, 1, bounds, 0x0100_0000),
            Err(WindowCommandError::NonCanonicalColor)
        );
    }

    #[test]
    fn window_command_decoder_rejects_lengths_headers_and_every_reserved_byte() {
        let canonical = WindowCommand::create(1, test_window_bounds())
            .unwrap()
            .encode();
        assert_eq!(
            WindowCommand::decode(&canonical[..WINDOW_COMMAND_WIRE_SIZE - 1]),
            Err(WindowCommandError::InvalidWireLength)
        );
        let mut oversized = [0_u8; WINDOW_COMMAND_WIRE_SIZE + 1];
        oversized[..WINDOW_COMMAND_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            WindowCommand::decode(&oversized),
            Err(WindowCommandError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, WindowCommandError::InvalidMagic),
            (4, WindowCommandError::InvalidVersion),
            (6, WindowCommandError::InvalidOpcode),
            (7, WindowCommandError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(WindowCommand::decode(&wire), Err(expected));
        }
        for offset in WINDOW_COMMAND_OFFSET_BODY_RESERVED..WINDOW_COMMAND_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                WindowCommand::decode(&wire),
                Err(WindowCommandError::NonZeroBodyReserved)
            );
        }
    }

    #[test]
    fn window_command_decoder_rejects_noncanonical_tokens_and_revalidates_fields() {
        let id = test_window_id();
        let canonical = WindowCommand::present(1, id, 1, ShellRect::new(1, 2, 3, 4), 0x0012_3456)
            .unwrap()
            .encode();
        for (token, expected) in [
            (0_u64, WindowIdError::ZeroGeneration),
            (1_u64, WindowIdError::ZeroGeneration),
            (0x0000_0001_0000_0002, WindowIdError::SlotOutOfRange),
            (0x0000_0001_0000_0100, WindowIdError::SlotOutOfRange),
        ] {
            let mut wire = canonical;
            wire[16..24].copy_from_slice(&token.to_le_bytes());
            assert_eq!(
                WindowCommand::decode(&wire),
                Err(WindowCommandError::InvalidWindowId(expected))
            );
        }
        let mut zero_sequence = canonical;
        zero_sequence[8..16].fill(0);
        assert_eq!(
            WindowCommand::decode(&zero_sequence),
            Err(WindowCommandError::ZeroSequence)
        );
        let mut zero_frame = canonical;
        zero_frame[24..28].fill(0);
        assert_eq!(
            WindowCommand::decode(&zero_frame),
            Err(WindowCommandError::ZeroFrameId)
        );
        let mut empty_damage = canonical;
        empty_damage[40..42].fill(0);
        assert_eq!(
            WindowCommand::decode(&empty_damage),
            Err(WindowCommandError::InvalidGeometry(
                WindowGeometryError::Empty
            ))
        );
        let mut bad_color = canonical;
        bad_color[47] = 1;
        assert_eq!(
            WindowCommand::decode(&bad_color),
            Err(WindowCommandError::NonCanonicalColor)
        );
    }

    #[test]
    fn window_command_decoder_rejects_every_cross_opcode_payload() {
        let id = test_window_id();
        let create = WindowCommand::create(1, test_window_bounds())
            .unwrap()
            .encode();
        for (offset, expected) in [
            (16, WindowCommandError::UnexpectedWindowId),
            (24, WindowCommandError::UnexpectedFrameId),
            (36, WindowCommandError::UnexpectedDamage),
            (44, WindowCommandError::UnexpectedColor),
        ] {
            let mut wire = create;
            wire[offset] = 1;
            assert_eq!(WindowCommand::decode(&wire), Err(expected));
        }

        let present = WindowCommand::present(2, id, 1, ShellRect::new(1, 1, 1, 1), 0)
            .unwrap()
            .encode();
        let mut present_with_bounds = present;
        present_with_bounds[28] = 1;
        assert_eq!(
            WindowCommand::decode(&present_with_bounds),
            Err(WindowCommandError::UnexpectedBounds)
        );

        for canonical in [
            WindowCommand::raise(3, id).unwrap().encode(),
            WindowCommand::destroy(4, id).unwrap().encode(),
        ] {
            for (offset, expected) in [
                (24, WindowCommandError::UnexpectedFrameId),
                (28, WindowCommandError::UnexpectedBounds),
                (36, WindowCommandError::UnexpectedDamage),
                (44, WindowCommandError::UnexpectedColor),
            ] {
                let mut wire = canonical;
                wire[offset] = 1;
                assert_eq!(WindowCommand::decode(&wire), Err(expected));
            }
        }

        let mut invalid_create_bounds = create;
        invalid_create_bounds[32..34].fill(0);
        assert_eq!(
            WindowCommand::decode(&invalid_create_bounds),
            Err(WindowCommandError::InvalidGeometry(
                WindowGeometryError::Empty
            ))
        );
    }

    #[test]
    fn window_command_tracker_is_strictly_monotonic_and_transactional() {
        let bounds = test_window_bounds();
        let mut tracker = WindowCommandTracker::new();
        tracker
            .accept(WindowCommand::create(1, bounds).unwrap())
            .unwrap();
        tracker
            .accept(WindowCommand::create(3, bounds).unwrap())
            .unwrap();
        assert_eq!(tracker.last_sequence(), Some(3));
        for replay in [2, 3] {
            let before = tracker;
            assert_eq!(
                tracker.accept(WindowCommand::create(replay, bounds).unwrap()),
                Err(WindowCommandTrackerError::SequenceReplay)
            );
            assert_eq!(tracker, before);
        }

        let mut exhausted = WindowCommandTracker::new();
        exhausted
            .accept(WindowCommand::create(u64::MAX, bounds).unwrap())
            .unwrap();
        let before = exhausted;
        assert_eq!(
            exhausted.accept(WindowCommand::create(u64::MAX, bounds).unwrap()),
            Err(WindowCommandTrackerError::SequenceExhausted)
        );
        assert_eq!(exhausted, before);
    }

    #[test]
    fn window_event_wire_is_exact_little_endian_and_all_opcodes_roundtrip() {
        assert_eq!(WINDOW_EVENT_WIRE_SIZE, 64);
        let id = test_window_id();
        let bounds = ShellRect::new(1, 2, 3, 4);
        let created = WindowEvent::created(0x0102_0304_0506_0708, 0x1112_1314, id, bounds).unwrap();
        let presented =
            WindowEvent::presented(0x2122_2324_2526_2728, 0x3132_3334, id, 0x4142_4344).unwrap();
        let raised = WindowEvent::raised(0x5152_5354_5556_5758, 0x6162_6364, id).unwrap();
        let input = WindowEvent::input_route(
            0x7172_7374_7576_7778,
            0x0102_0304,
            id,
            80,
            120,
            -32,
            -48,
            true,
            true,
            0x1112_1314_1516_1718,
        )
        .unwrap();
        let destroyed = WindowEvent::destroyed(0x2122_2324_2526_2728, 0x3132_3334, id).unwrap();

        for event in [created, presented, raised, input, destroyed] {
            let wire = event.encode();
            assert_eq!(&wire[0..4], b"BWE1");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(wire[6], event.opcode().raw());
            assert_eq!(wire[7], 0);
            assert_eq!(&wire[8..16], &event.command_sequence().to_le_bytes());
            assert_eq!(&wire[16..24], &u64::from(event.scene_frame()).to_le_bytes());
            assert_eq!(&wire[24..32], &id.token().to_le_bytes());
            assert!(wire[57..].iter().all(|byte| *byte == 0));
            assert_eq!(WindowEvent::decode(&wire), Ok(event));
        }

        let created_wire = created.encode();
        assert_eq!(&created_wire[32..34], &1_u16.to_le_bytes());
        assert_eq!(&created_wire[34..36], &2_u16.to_le_bytes());
        assert_eq!(&created_wire[36..38], &3_u16.to_le_bytes());
        assert_eq!(&created_wire[38..40], &4_u16.to_le_bytes());
        assert!(created_wire[40..57].iter().all(|byte| *byte == 0));

        let presented_wire = presented.encode();
        assert_eq!(
            &presented_wire[32..40],
            &u64::from(0x4142_4344_u32).to_le_bytes()
        );
        assert!(presented_wire[40..57].iter().all(|byte| *byte == 0));
        for event in [raised, destroyed] {
            assert!(event.encode()[32..57].iter().all(|byte| *byte == 0));
        }

        let input_wire = input.encode();
        assert_eq!(
            &input_wire[32..40],
            &0x1112_1314_1516_1718_u64.to_le_bytes()
        );
        assert_eq!(&input_wire[40..42], &80_u16.to_le_bytes());
        assert_eq!(&input_wire[42..44], &120_u16.to_le_bytes());
        assert!(input_wire[44..48].iter().all(|byte| *byte == 0));
        assert_eq!(&input_wire[48..52], &(-32_i32).to_le_bytes());
        assert_eq!(&input_wire[52..56], &(-48_i32).to_le_bytes());
        assert_eq!(input_wire[56], 0b11);

        for (raw, opcode) in [
            (1, WindowEventOpcode::Created),
            (2, WindowEventOpcode::Presented),
            (3, WindowEventOpcode::Raised),
            (4, WindowEventOpcode::InputRoute),
            (5, WindowEventOpcode::Destroyed),
        ] {
            assert_eq!(WindowEventOpcode::from_raw(raw), Some(opcode));
        }
        assert_eq!(WindowEventOpcode::from_raw(0), None);
        assert_eq!(WindowEventOpcode::from_raw(6), None);
    }

    #[test]
    fn window_event_constructors_reject_common_geometry_frame_and_input_fields() {
        let id = test_window_id();
        let bounds = test_window_bounds();
        assert_eq!(
            WindowEvent::created(0, 1, id, bounds),
            Err(WindowEventError::ZeroCommandSequence)
        );
        assert_eq!(
            WindowEvent::created(1, 0, id, bounds),
            Err(WindowEventError::ZeroSceneFrame)
        );
        assert_eq!(
            WindowEvent::created(1, 1, id, ShellRect::new(0, 0, 0, 1)),
            Err(WindowEventError::InvalidGeometry(
                WindowGeometryError::Empty
            ))
        );
        assert_eq!(
            WindowEvent::created(1, 1, id, ShellRect::new(SURFACE_WIDTH - 1, 0, 2, 1),),
            Err(WindowEventError::InvalidGeometry(
                WindowGeometryError::OutOfBounds
            ))
        );
        assert_eq!(
            WindowEvent::presented(1, 1, id, 0),
            Err(WindowEventError::ZeroFrameId)
        );
        assert_eq!(
            WindowEvent::input_route(1, 1, id, SURFACE_WIDTH, 0, 0, 0, false, false, 1),
            Err(WindowEventError::GlobalCoordinateOutOfBounds)
        );
        assert_eq!(
            WindowEvent::input_route(1, 1, id, 0, SURFACE_HEIGHT, 0, 0, false, false, 1),
            Err(WindowEventError::GlobalCoordinateOutOfBounds)
        );
        assert_eq!(
            WindowEvent::input_route(1, 1, id, 0, 0, 0, 0, false, false, 0),
            Err(WindowEventError::ZeroFocusGeneration)
        );
        assert_eq!(
            WindowEvent::raised(0, 1, id),
            Err(WindowEventError::ZeroCommandSequence)
        );
        assert_eq!(
            WindowEvent::destroyed(1, 0, id),
            Err(WindowEventError::ZeroSceneFrame)
        );
    }

    #[test]
    fn window_event_decoder_rejects_lengths_headers_and_every_reserved_byte() {
        let canonical = WindowEvent::created(1, 1, test_window_id(), test_window_bounds())
            .unwrap()
            .encode();
        assert_eq!(
            WindowEvent::decode(&canonical[..WINDOW_EVENT_WIRE_SIZE - 1]),
            Err(WindowEventError::InvalidWireLength)
        );
        let mut oversized = [0_u8; WINDOW_EVENT_WIRE_SIZE + 1];
        oversized[..WINDOW_EVENT_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            WindowEvent::decode(&oversized),
            Err(WindowEventError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, WindowEventError::InvalidMagic),
            (4, WindowEventError::InvalidVersion),
            (6, WindowEventError::InvalidOpcode),
            (7, WindowEventError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(WindowEvent::decode(&wire), Err(expected));
        }
        for offset in WINDOW_EVENT_OFFSET_BODY_RESERVED..WINDOW_EVENT_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                WindowEvent::decode(&wire),
                Err(WindowEventError::NonZeroBodyReserved)
            );
        }
    }

    #[test]
    fn window_event_decoder_rejects_tokens_counters_and_geometry() {
        let id = test_window_id();
        let canonical = WindowEvent::created(1, 1, id, test_window_bounds())
            .unwrap()
            .encode();
        for (token, expected) in [
            (0_u64, WindowIdError::ZeroGeneration),
            (1_u64, WindowIdError::ZeroGeneration),
            (0x0000_0001_0000_0002, WindowIdError::SlotOutOfRange),
            (0x0000_0001_0000_0100, WindowIdError::SlotOutOfRange),
        ] {
            let mut wire = canonical;
            wire[24..32].copy_from_slice(&token.to_le_bytes());
            assert_eq!(
                WindowEvent::decode(&wire),
                Err(WindowEventError::InvalidWindowId(expected))
            );
        }
        let mut zero_sequence = canonical;
        zero_sequence[8..16].fill(0);
        assert_eq!(
            WindowEvent::decode(&zero_sequence),
            Err(WindowEventError::ZeroCommandSequence)
        );
        let mut zero_scene = canonical;
        zero_scene[16..24].fill(0);
        assert_eq!(
            WindowEvent::decode(&zero_scene),
            Err(WindowEventError::ZeroSceneFrame)
        );
        let mut wide_scene = canonical;
        wide_scene[16..24].copy_from_slice(&(u64::from(u32::MAX) + 1).to_le_bytes());
        assert_eq!(
            WindowEvent::decode(&wide_scene),
            Err(WindowEventError::SceneFrameOutOfRange)
        );
        let mut empty_bounds = canonical;
        empty_bounds[36..38].fill(0);
        assert_eq!(
            WindowEvent::decode(&empty_bounds),
            Err(WindowEventError::InvalidGeometry(
                WindowGeometryError::Empty
            ))
        );

        let presented = WindowEvent::presented(1, 1, id, 1).unwrap().encode();
        let mut zero_frame = presented;
        zero_frame[32..40].fill(0);
        assert_eq!(
            WindowEvent::decode(&zero_frame),
            Err(WindowEventError::ZeroFrameId)
        );
        let mut wide_frame = presented;
        wide_frame[32..40].copy_from_slice(&(u64::from(u32::MAX) + 1).to_le_bytes());
        assert_eq!(
            WindowEvent::decode(&wide_frame),
            Err(WindowEventError::FrameIdOutOfRange)
        );
    }

    #[test]
    fn window_input_event_preserves_signed_coordinates_and_rejects_noncanonical_state() {
        let id = test_window_id();
        for event in [
            WindowEvent::input_route(
                1,
                1,
                id,
                SURFACE_WIDTH - 1,
                SURFACE_HEIGHT - 1,
                i32::MIN,
                i32::MAX,
                false,
                true,
                1,
            )
            .unwrap(),
            WindowEvent::input_route(2, 2, id, 0, 0, -1, -2, true, false, u64::MAX).unwrap(),
        ] {
            assert_eq!(WindowEvent::decode(&event.encode()), Ok(event));
        }

        let canonical = WindowEvent::input_route(1, 1, id, 1, 2, -3, -4, true, true, 1)
            .unwrap()
            .encode();
        let mut hidden_global_bits = canonical;
        hidden_global_bits[44] = 1;
        assert_eq!(
            WindowEvent::decode(&hidden_global_bits),
            Err(WindowEventError::NonCanonicalGlobalCoordinates)
        );
        let mut invalid_state = canonical;
        invalid_state[56] = 0b100;
        assert_eq!(
            WindowEvent::decode(&invalid_state),
            Err(WindowEventError::NonCanonicalInputState)
        );
        let mut bad_global_x = canonical;
        bad_global_x[40..42].copy_from_slice(&SURFACE_WIDTH.to_le_bytes());
        assert_eq!(
            WindowEvent::decode(&bad_global_x),
            Err(WindowEventError::GlobalCoordinateOutOfBounds)
        );
        let mut bad_global_y = canonical;
        bad_global_y[42..44].copy_from_slice(&SURFACE_HEIGHT.to_le_bytes());
        assert_eq!(
            WindowEvent::decode(&bad_global_y),
            Err(WindowEventError::GlobalCoordinateOutOfBounds)
        );
        let mut zero_focus = canonical;
        zero_focus[32..40].fill(0);
        assert_eq!(
            WindowEvent::decode(&zero_focus),
            Err(WindowEventError::ZeroFocusGeneration)
        );
    }

    #[test]
    fn window_event_decoder_rejects_every_cross_opcode_payload() {
        let id = test_window_id();
        let created = WindowEvent::created(1, 1, id, test_window_bounds())
            .unwrap()
            .encode();
        for offset in [40, 48, 56] {
            let mut wire = created;
            wire[offset] = 1;
            assert_eq!(
                WindowEvent::decode(&wire),
                Err(WindowEventError::UnexpectedPayload)
            );
        }

        let presented = WindowEvent::presented(2, 2, id, 1).unwrap().encode();
        for offset in [40, 48, 56] {
            let mut wire = presented;
            wire[offset] = 1;
            assert_eq!(
                WindowEvent::decode(&wire),
                Err(WindowEventError::UnexpectedPayload)
            );
        }

        for canonical in [
            WindowEvent::raised(3, 3, id).unwrap().encode(),
            WindowEvent::destroyed(4, 4, id).unwrap().encode(),
        ] {
            for offset in [32, 40, 48, 56] {
                let mut wire = canonical;
                wire[offset] = 1;
                assert_eq!(
                    WindowEvent::decode(&wire),
                    Err(WindowEventError::UnexpectedPayload)
                );
            }
        }
    }

    #[test]
    fn window_event_tracker_requires_created_and_exposes_live_state() {
        let id = test_window_id();
        let bounds = test_window_bounds();
        let mut tracker = WindowEventTracker::new();
        assert_eq!(tracker, WindowEventTracker::default());
        assert_eq!(tracker.phase(), WindowEventTrackerPhase::AwaitingCreated);
        assert_eq!(tracker.window_id(), None);
        assert_eq!(tracker.bounds(), None);
        assert_eq!(tracker.last_pair(), None);
        assert_eq!(tracker.last_presented_frame_id(), None);
        assert_eq!(tracker.last_focus_generation(), None);
        assert_eq!(tracker.last_opcode(), None);
        assert_eq!(tracker.generation_floor(0), None);
        assert_eq!(tracker.generation_floor(WINDOW_CAPACITY), None);
        assert!(!tracker.is_live());
        assert!(!tracker.is_destroyed());

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::presented(1, 1, id, 1).unwrap(),
            WindowEventTrackerError::CreatedRequired,
        );
        let before = tracker;
        assert_eq!(
            tracker.begin_next_lifecycle(),
            Err(WindowEventTrackerError::LifecycleNotTerminated)
        );
        assert_eq!(tracker, before);

        tracker
            .accept(WindowEvent::created(2, 3, id, bounds).unwrap())
            .unwrap();
        assert_eq!(tracker.phase(), WindowEventTrackerPhase::Live);
        assert_eq!(tracker.window_id(), Some(id));
        assert_eq!(tracker.bounds(), Some(bounds));
        assert_eq!(tracker.last_command_sequence(), Some(2));
        assert_eq!(tracker.last_acknowledged_command_sequence(), Some(2));
        assert_eq!(tracker.last_scene_frame(), Some(3));
        assert_eq!(tracker.last_pair(), Some((2, 3)));
        assert_eq!(tracker.last_opcode(), Some(WindowEventOpcode::Created));
        assert_eq!(tracker.generation_floor(id.slot()), Some(id.generation()));
        assert!(tracker.is_live());
    }

    #[test]
    fn window_event_tracker_binds_one_id_and_rejects_duplicate_created() {
        let id = WindowId::try_new(0, 1).unwrap();
        let other = WindowId::try_new(1, 1).unwrap();
        let bounds = test_window_bounds();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(1, 1, id, bounds).unwrap())
            .unwrap();

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(2, 2, id, bounds).unwrap(),
            WindowEventTrackerError::DuplicateCreated,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(2, 2, other, bounds).unwrap(),
            WindowEventTrackerError::DuplicateCreated,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::raised(2, 2, other).unwrap(),
            WindowEventTrackerError::WindowMismatch,
        );
        tracker
            .accept(WindowEvent::raised(2, 2, id).unwrap())
            .unwrap();
        assert_eq!(tracker.last_pair(), Some((2, 2)));
    }

    #[test]
    fn window_event_tracker_requires_strict_ack_commands_and_monotonic_scenes() {
        let id = test_window_id();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(10, 20, id, test_window_bounds()).unwrap())
            .unwrap();

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::raised(9, 21, id).unwrap(),
            WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::raised(10, 21, id).unwrap(),
            WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::raised(11, 19, id).unwrap(),
            WindowEventTrackerError::SceneFrameRegression,
        );

        tracker
            .accept(WindowEvent::raised(11, 20, id).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::raised(12, 21, id).unwrap())
            .unwrap();
        assert_eq!(tracker.last_pair(), Some((12, 21)));
    }

    #[test]
    fn window_event_tracker_allows_only_input_routes_to_repeat_an_exact_pair() {
        let id = test_window_id();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(1, 1, id, test_window_bounds()).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::raised(2, 2, id).unwrap())
            .unwrap();

        for event in [
            WindowEvent::input_route(2, 2, id, 80, 120, 32, 40, true, true, 1).unwrap(),
            WindowEvent::input_route(2, 2, id, 16, 32, -32, -48, true, true, 1).unwrap(),
            WindowEvent::input_route(2, 2, id, 16, 32, -32, -48, false, true, 1).unwrap(),
        ] {
            tracker.accept(event).unwrap();
        }
        assert_eq!(tracker.last_pair(), Some((2, 2)));
        assert_eq!(tracker.last_opcode(), Some(WindowEventOpcode::InputRoute));

        for event in [
            WindowEvent::raised(2, 2, id).unwrap(),
            WindowEvent::presented(2, 3, id, 1).unwrap(),
            WindowEvent::destroyed(2, 3, id).unwrap(),
        ] {
            assert_window_event_rejected(
                &mut tracker,
                event,
                WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
            );
        }

        tracker
            .accept(WindowEvent::presented(3, 3, id, 1).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::input_route(3, 3, id, 1, 2, -3, -4, false, false, 1).unwrap())
            .unwrap();
        assert_eq!(tracker.last_pair(), Some((3, 3)));
    }

    #[test]
    fn window_event_tracker_rejects_duplicate_acks_and_input_command_drift_transactionally() {
        let id = test_window_id();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(4, 10, id, test_window_bounds()).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::presented(5, 11, id, 1).unwrap())
            .unwrap();

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::presented(5, 12, id, 2).unwrap(),
            WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::input_route(4, 12, id, 1, 2, 3, 4, false, false, 1).unwrap(),
            WindowEventTrackerError::InputCommandBehind,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::input_route(6, 12, id, 1, 2, 3, 4, false, false, 1).unwrap(),
            WindowEventTrackerError::InputCommandAhead,
        );
        assert_eq!(tracker.last_acknowledged_command_sequence(), Some(5));
        assert_eq!(tracker.last_pair(), Some((5, 11)));
        assert_eq!(tracker.last_presented_frame_id(), Some(1));

        tracker
            .accept(WindowEvent::input_route(5, 11, id, 1, 2, 3, 4, true, true, 1).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::input_route(5, 12, id, 2, 3, 4, 5, false, true, 1).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::raised(6, 12, id).unwrap())
            .unwrap();

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::input_route(5, 13, id, 3, 4, 5, 6, false, false, 1).unwrap(),
            WindowEventTrackerError::InputCommandBehind,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::input_route(7, 13, id, 3, 4, 5, 6, false, false, 1).unwrap(),
            WindowEventTrackerError::InputCommandAhead,
        );
        assert_eq!(tracker.last_acknowledged_command_sequence(), Some(6));
        assert_eq!(tracker.last_pair(), Some((6, 12)));
    }

    #[test]
    fn window_event_tracker_requires_strict_presented_frames() {
        let id = test_window_id();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(1, 1, id, test_window_bounds()).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::presented(2, 2, id, 4).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::raised(3, 3, id).unwrap())
            .unwrap();

        for frame_id in [3, 4] {
            assert_window_event_rejected(
                &mut tracker,
                WindowEvent::presented(4, 4, id, frame_id).unwrap(),
                WindowEventTrackerError::PresentedFrameNotIncreasing,
            );
        }
        assert_eq!(tracker.last_presented_frame_id(), Some(4));
        tracker
            .accept(WindowEvent::presented(4, 4, id, 5).unwrap())
            .unwrap();
        assert_eq!(tracker.last_presented_frame_id(), Some(5));
    }

    #[test]
    fn window_event_tracker_focus_generation_is_nondecreasing() {
        let id = test_window_id();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(1, 1, id, test_window_bounds()).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::input_route(1, 2, id, 1, 2, 3, 4, true, true, 7).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::input_route(1, 2, id, 2, 3, 4, 5, false, true, 7).unwrap())
            .unwrap();
        assert_eq!(tracker.last_focus_generation(), Some(7));

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::input_route(1, 3, id, 3, 4, 5, 6, false, false, 6).unwrap(),
            WindowEventTrackerError::FocusGenerationRegression,
        );
        tracker
            .accept(WindowEvent::input_route(1, 3, id, 3, 4, 5, 6, false, false, 8).unwrap())
            .unwrap();
        assert_eq!(tracker.last_focus_generation(), Some(8));
    }

    #[test]
    fn window_event_tracker_restart_preserves_order_and_generation_floors() {
        let first = WindowId::try_new(0, 1).unwrap();
        let second = WindowId::try_new(0, 2).unwrap();
        let other_slot = WindowId::try_new(1, 1).unwrap();
        let bounds = test_window_bounds();
        let mut tracker = WindowEventTracker::new();
        tracker
            .accept(WindowEvent::created(1, 1, first, bounds).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::presented(2, 2, first, 9).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::input_route(2, 3, first, 1, 2, 3, 4, false, false, 11).unwrap())
            .unwrap();
        tracker
            .accept(WindowEvent::destroyed(3, 4, first).unwrap())
            .unwrap();
        assert!(tracker.is_destroyed());
        assert_eq!(tracker.bounds(), Some(bounds));
        assert_eq!(tracker.last_opcode(), Some(WindowEventOpcode::Destroyed));

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(4, 5, second, bounds).unwrap(),
            WindowEventTrackerError::EventAfterDestroyed,
        );
        tracker.begin_next_lifecycle().unwrap();
        assert_eq!(tracker.phase(), WindowEventTrackerPhase::AwaitingCreated);
        assert_eq!(tracker.window_id(), None);
        assert_eq!(tracker.bounds(), None);
        assert_eq!(tracker.last_presented_frame_id(), None);
        assert_eq!(tracker.last_focus_generation(), None);
        assert_eq!(tracker.last_opcode(), None);
        assert_eq!(tracker.last_pair(), Some((3, 4)));
        assert_eq!(tracker.generation_floor(0), Some(1));

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::presented(4, 5, second, 1).unwrap(),
            WindowEventTrackerError::CreatedRequired,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(2, 5, second, bounds).unwrap(),
            WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(4, 3, second, bounds).unwrap(),
            WindowEventTrackerError::SceneFrameRegression,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(3, 4, second, bounds).unwrap(),
            WindowEventTrackerError::AcknowledgedCommandNotIncreasing,
        );
        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(4, 5, first, bounds).unwrap(),
            WindowEventTrackerError::WindowGenerationReplay,
        );

        tracker
            .accept(WindowEvent::created(4, 5, second, bounds).unwrap())
            .unwrap();
        assert_eq!(tracker.last_presented_frame_id(), None);
        assert_eq!(tracker.last_focus_generation(), None);
        assert_eq!(tracker.generation_floor(0), Some(2));
        tracker
            .accept(WindowEvent::destroyed(5, 6, second).unwrap())
            .unwrap();
        tracker.begin_next_lifecycle().unwrap();
        tracker
            .accept(WindowEvent::created(6, 7, other_slot, bounds).unwrap())
            .unwrap();
        assert_eq!(tracker.generation_floor(0), Some(2));
        assert_eq!(tracker.generation_floor(1), Some(1));
        tracker
            .accept(WindowEvent::destroyed(7, 8, other_slot).unwrap())
            .unwrap();
        tracker.begin_next_lifecycle().unwrap();

        assert_window_event_rejected(
            &mut tracker,
            WindowEvent::created(8, 9, second, bounds).unwrap(),
            WindowEventTrackerError::WindowGenerationReplay,
        );
        let third = WindowId::try_new(0, 3).unwrap();
        tracker
            .accept(WindowEvent::created(8, 9, third, bounds).unwrap())
            .unwrap();
        assert_eq!(tracker.window_id(), Some(third));
        assert_eq!(tracker.generation_floor(0), Some(3));
    }

    #[test]
    fn bootstrap_wire_is_exact_closed_and_canonical() {
        assert_eq!(UI_BOOTSTRAP_WIRE_SIZE, 64);
        for kind in [
            UiBootstrapEndpointKind::SurfaceSupervisor,
            UiBootstrapEndpointKind::LauncherUi,
            UiBootstrapEndpointKind::AppUi,
        ] {
            let message = UiBootstrapMessage::new(kind);
            let wire = message.encode();
            assert_eq!(&wire[0..4], b"UBP1");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(wire[6], kind.raw());
            assert!(wire[7..].iter().all(|byte| *byte == 0));
            assert_eq!(UiBootstrapMessage::decode(&wire), Ok(message));
            assert_eq!(UiBootstrapEndpointKind::from_raw(kind.raw()), Some(kind));
        }
        assert_eq!(UiBootstrapEndpointKind::from_raw(0), None);
        assert_eq!(UiBootstrapEndpointKind::from_raw(4), None);

        let canonical = UiBootstrapMessage::new(UiBootstrapEndpointKind::LauncherUi).encode();
        assert_eq!(
            UiBootstrapMessage::decode(&canonical[..UI_BOOTSTRAP_WIRE_SIZE - 1]),
            Err(UiBootstrapWireError::InvalidWireLength)
        );
        let mut oversized = [0_u8; UI_BOOTSTRAP_WIRE_SIZE + 1];
        oversized[..UI_BOOTSTRAP_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            UiBootstrapMessage::decode(&oversized),
            Err(UiBootstrapWireError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, UiBootstrapWireError::InvalidMagic),
            (4, UiBootstrapWireError::InvalidVersion),
            (6, UiBootstrapWireError::InvalidKind),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(UiBootstrapMessage::decode(&wire), Err(expected));
        }
        for offset in BOOTSTRAP_OFFSET_RESERVED..UI_BOOTSTRAP_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                UiBootstrapMessage::decode(&wire),
                Err(UiBootstrapWireError::NonZeroReserved)
            );
        }
    }

    fn recovery_offer(
        role: RecoveryClientRole,
        cancel_kind: RecoveryCancelKind,
        cancel_after: u64,
    ) -> SurfaceRecoveryMessage {
        SurfaceRecoveryMessage::client_rebind_offer(
            1,
            2,
            2,
            role,
            cancel_kind,
            SurfaceRecoveryCheckpoint::MultiWindowInteractive,
            WindowId::try_new(usize::from(role == RecoveryClientRole::App), 1).unwrap(),
            2,
            cancel_after,
        )
        .unwrap()
    }

    fn recovery_contact() -> SurfaceRecoveryMessage {
        SurfaceRecoveryMessage::client_contact(
            1,
            1,
            1,
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            SurfaceRecoveryCheckpoint::MultiWindowInteractive,
            WindowId::try_new(1, 1).unwrap(),
            2,
        )
        .unwrap()
    }

    #[test]
    fn surface_recovery_all_six_kinds_have_exact_canonical_wires() {
        let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
        let status =
            SurfaceRecoveryMessage::status(1, 1, 1, checkpoint, 0, SurfaceRecoveryPhase::Armed)
                .unwrap();
        let bootstrap = SurfaceRecoveryMessage::bootstrap(
            1,
            2,
            2,
            0x1122_3344_5566_7788,
            7,
            2,
            checkpoint,
            1,
            RecoveryCancelKind::ClientPointer,
        )
        .unwrap();
        let offer = recovery_offer(
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            2,
        );
        let ack =
            SurfaceRecoveryMessage::client_rebind_ack(1, offer, WindowId::try_new(1, 1).unwrap())
                .unwrap();
        let contact = recovery_contact();
        let contact_ack = SurfaceRecoveryMessage::client_contact_ack(1, contact).unwrap();

        for message in [status, bootstrap, offer, ack, contact, contact_ack] {
            let wire = message.encode();
            assert_eq!(wire.len(), SURFACE_RECOVERY_WIRE_SIZE);
            assert_eq!(&wire[0..4], b"BSR1");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(wire[6], message.kind().raw());
            assert_eq!(wire[7], 0);
            assert_eq!(read_u64(&wire, 8), message.sender_sequence());
            assert_eq!(read_u64(&wire, 16), message.surface_session());
            assert_eq!(read_u64(&wire, 24), message.route_epoch());
            assert_eq!(SurfaceRecoveryMessage::decode(&wire), Ok(message));
            assert_eq!(
                SurfaceRecoveryMessage::decode(&wire).unwrap().encode(),
                wire
            );
        }
        assert_eq!(status.direction(), SurfaceRecoveryDirection::SurfaceToInit);
        assert_eq!(
            bootstrap.direction(),
            SurfaceRecoveryDirection::InitToSurface
        );
        assert_eq!(offer.direction(), SurfaceRecoveryDirection::SurfaceToClient);
        assert_eq!(ack.direction(), SurfaceRecoveryDirection::ClientToSurface);
        assert_eq!(
            contact.direction(),
            SurfaceRecoveryDirection::SurfaceToClient
        );
        assert_eq!(
            contact_ack.direction(),
            SurfaceRecoveryDirection::ClientToSurface
        );
        assert!(ack.acknowledges(offer));
        assert!(contact_ack.acknowledges_contact(contact));
    }

    #[test]
    fn surface_recovery_golden_offsets_are_stable() {
        let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
        let status = SurfaceRecoveryMessage::status(
            9,
            11,
            13,
            checkpoint,
            17,
            SurfaceRecoveryPhase::RestartRequested,
        )
        .unwrap()
        .encode();
        assert_eq!(read_u64(&status, 32), 1);
        assert_eq!(read_u64(&status, 40), 17);
        assert_eq!(status[48], SurfaceRecoveryPhase::RestartRequested.raw());
        assert!(status[49..].iter().all(|byte| *byte == 0));

        let bootstrap = SurfaceRecoveryMessage::bootstrap(
            1,
            2,
            3,
            5,
            7,
            11,
            checkpoint,
            1,
            RecoveryCancelKind::ClientPointer,
        )
        .unwrap()
        .encode();
        assert_eq!(read_u64(&bootstrap, 32), 5);
        assert_eq!(read_u64(&bootstrap, 40), 7);
        assert_eq!(read_u64(&bootstrap, 48), 11);
        assert_eq!(&bootstrap[56..58], &1_u16.to_le_bytes());
        assert_eq!(bootstrap[58], 1);
        assert_eq!(bootstrap[59], RecoveryCancelKind::ClientPointer.raw());
        assert!(bootstrap[60..].iter().all(|byte| *byte == 0));

        let offer = recovery_offer(
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            2,
        )
        .encode();
        assert_eq!(&offer[32..36], &[2, 1, 1, 0]);
        assert!(offer[36..40].iter().all(|byte| *byte == 0));
        assert_eq!(
            read_u64(&offer, 40),
            WindowId::try_new(1, 1).unwrap().token()
        );
        assert_eq!(read_u64(&offer, 48), 2);
        assert_eq!(read_u64(&offer, 56), 2);

        let contact = recovery_contact();
        let contact_wire = contact.encode();
        assert_eq!(&contact_wire[32..36], &[2, 1, 1, 0]);
        assert!(contact_wire[36..40].iter().all(|byte| *byte == 0));
        assert_eq!(
            read_u64(&contact_wire, 40),
            WindowId::try_new(1, 1).unwrap().token()
        );
        assert_eq!(read_u64(&contact_wire, 48), 2);
        assert_eq!(read_u64(&contact_wire, 56), 0);

        let contact_ack = SurfaceRecoveryMessage::client_contact_ack(1, contact)
            .unwrap()
            .encode();
        assert_eq!(&contact_ack[32..36], &[2, 1, 1, 0]);
        assert_eq!(
            read_u64(&contact_ack, 40),
            WindowId::try_new(1, 1).unwrap().token()
        );
        assert_eq!(read_u64(&contact_ack, 48), 1);
        assert_eq!(read_u64(&contact_ack, 56), 2);
    }

    #[test]
    fn surface_recovery_decoder_rejects_lengths_headers_and_reserved_bytes() {
        let canonical =
            recovery_offer(RecoveryClientRole::Launcher, RecoveryCancelKind::None, 0).encode();
        assert_eq!(
            SurfaceRecoveryMessage::decode(&canonical[..63]),
            Err(SurfaceRecoveryError::InvalidWireLength)
        );
        let mut oversized = [0_u8; 65];
        oversized[..64].copy_from_slice(&canonical);
        assert_eq!(
            SurfaceRecoveryMessage::decode(&oversized),
            Err(SurfaceRecoveryError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, SurfaceRecoveryError::InvalidMagic),
            (4, SurfaceRecoveryError::InvalidVersion),
            (6, SurfaceRecoveryError::InvalidKind),
            (7, SurfaceRecoveryError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(SurfaceRecoveryMessage::decode(&wire), Err(expected));
        }
        for offset in 36..40 {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                SurfaceRecoveryMessage::decode(&wire),
                Err(SurfaceRecoveryError::NonZeroBodyReserved)
            );
        }

        let status = SurfaceRecoveryMessage::status(
            1,
            1,
            1,
            SurfaceRecoveryCheckpoint::MultiWindowInteractive,
            0,
            SurfaceRecoveryPhase::Armed,
        )
        .unwrap()
        .encode();
        for offset in 49..64 {
            let mut wire = status;
            wire[offset] = 1;
            assert_eq!(
                SurfaceRecoveryMessage::decode(&wire),
                Err(SurfaceRecoveryError::NonZeroBodyReserved)
            );
        }
    }

    #[test]
    fn surface_recovery_cancel_contract_is_closed_and_transactional() {
        let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
        let launcher = WindowId::try_new(0, 1).unwrap();
        assert_eq!(
            SurfaceRecoveryMessage::client_rebind_offer(
                1,
                2,
                2,
                RecoveryClientRole::Launcher,
                RecoveryCancelKind::ClientPointer,
                checkpoint,
                launcher,
                2,
                2,
            ),
            Err(SurfaceRecoveryError::LauncherCannotCancel)
        );
        for (kind, cancel_after, expected) in [
            (
                RecoveryCancelKind::None,
                1,
                SurfaceRecoveryError::UnexpectedCancelAfter,
            ),
            (
                RecoveryCancelKind::ClientPointer,
                0,
                SurfaceRecoveryError::CancelAfterRequired,
            ),
            (
                RecoveryCancelKind::ClientPointer,
                3,
                SurfaceRecoveryError::CancelAfterBeyondFloor,
            ),
            (
                RecoveryCancelKind::TrustedOverlayPointer,
                2,
                SurfaceRecoveryError::TrustedCancelCannotTargetClient,
            ),
        ] {
            assert_eq!(
                SurfaceRecoveryMessage::client_rebind_offer(
                    1,
                    2,
                    2,
                    RecoveryClientRole::App,
                    kind,
                    checkpoint,
                    WindowId::try_new(1, 1).unwrap(),
                    2,
                    cancel_after,
                ),
                Err(expected)
            );
        }
    }

    #[test]
    fn surface_recovery_contact_contract_is_app_pointer_only_and_canonical() {
        let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
        let app = WindowId::try_new(1, 1).unwrap();
        for (role, cancel_kind, physical_sequence, expected) in [
            (
                RecoveryClientRole::Launcher,
                RecoveryCancelKind::ClientPointer,
                2,
                SurfaceRecoveryError::ContactRequiresApp,
            ),
            (
                RecoveryClientRole::App,
                RecoveryCancelKind::None,
                2,
                SurfaceRecoveryError::ContactRequiresClientPointer,
            ),
            (
                RecoveryClientRole::App,
                RecoveryCancelKind::ClientPointer,
                0,
                SurfaceRecoveryError::ZeroContactPhysicalSequence,
            ),
        ] {
            assert_eq!(
                SurfaceRecoveryMessage::client_contact(
                    1,
                    1,
                    1,
                    role,
                    cancel_kind,
                    checkpoint,
                    app,
                    physical_sequence,
                ),
                Err(expected)
            );
        }
        assert_eq!(
            SurfaceRecoveryMessage::client_contact_ack(
                1,
                recovery_offer(
                    RecoveryClientRole::App,
                    RecoveryCancelKind::ClientPointer,
                    2
                ),
            ),
            Err(SurfaceRecoveryError::ZeroAcknowledgedContact)
        );

        let canonical = recovery_contact().encode();
        for (offset, value, expected) in [
            (
                32,
                RecoveryClientRole::Launcher.raw(),
                SurfaceRecoveryError::ContactRequiresApp,
            ),
            (
                33,
                RecoveryCancelKind::None.raw(),
                SurfaceRecoveryError::ContactRequiresClientPointer,
            ),
        ] {
            let mut wire = canonical;
            wire[offset] = value;
            assert_eq!(SurfaceRecoveryMessage::decode(&wire), Err(expected));
        }
        let mut zero_physical = canonical;
        zero_physical[48..56].fill(0);
        assert_eq!(
            SurfaceRecoveryMessage::decode(&zero_physical),
            Err(SurfaceRecoveryError::ZeroContactPhysicalSequence)
        );
        for offset in 56..64 {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                SurfaceRecoveryMessage::decode(&wire),
                Err(SurfaceRecoveryError::NonZeroBodyReserved)
            );
        }

        let mut zero_ack = SurfaceRecoveryMessage::client_contact_ack(1, recovery_contact())
            .unwrap()
            .encode();
        zero_ack[48..56].fill(0);
        assert_eq!(
            SurfaceRecoveryMessage::decode(&zero_ack),
            Err(SurfaceRecoveryError::ZeroAcknowledgedContact)
        );
    }

    #[test]
    fn surface_recovery_sequence_directions_are_independent_and_strict() {
        let checkpoint = SurfaceRecoveryCheckpoint::MultiWindowInteractive;
        let armed =
            SurfaceRecoveryMessage::status(1, 1, 1, checkpoint, 0, SurfaceRecoveryPhase::Armed)
                .unwrap();
        let requested = SurfaceRecoveryMessage::status(
            2,
            1,
            1,
            checkpoint,
            2,
            SurfaceRecoveryPhase::RestartRequested,
        )
        .unwrap();
        let bootstrap = SurfaceRecoveryMessage::bootstrap(
            1,
            2,
            2,
            5,
            1,
            2,
            checkpoint,
            1,
            RecoveryCancelKind::ClientPointer,
        )
        .unwrap();
        let offer = recovery_offer(
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            2,
        );
        let ack =
            SurfaceRecoveryMessage::client_rebind_ack(1, offer, WindowId::try_new(1, 1).unwrap())
                .unwrap();
        let contact = recovery_contact();
        let contact_ack = SurfaceRecoveryMessage::client_contact_ack(1, contact).unwrap();
        let mut tracker = SurfaceRecoverySequenceTracker::new();
        for message in [armed, contact, contact_ack, requested] {
            tracker.accept(message).unwrap();
        }
        let before = tracker;
        assert_eq!(
            tracker.accept(requested),
            Err(SurfaceRecoverySequenceError::SequenceReplay)
        );
        assert_eq!(tracker, before);
        let gap =
            SurfaceRecoveryMessage::status(4, 2, 2, checkpoint, 3, SurfaceRecoveryPhase::Active)
                .unwrap();
        assert_eq!(
            tracker.accept(gap),
            Err(SurfaceRecoverySequenceError::SequenceGap)
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker.last(SurfaceRecoveryDirection::SurfaceToInit),
            Some(2)
        );

        // Sender cursors are scoped to one recovery session/endpoint set. The
        // replacement graph starts fresh at sequence one in every direction.
        let mut replacement_tracker = SurfaceRecoverySequenceTracker::new();
        for message in [bootstrap, offer, ack] {
            replacement_tracker.accept(message).unwrap();
        }
    }

    #[test]
    fn surface_recovery_ack_repeats_offer_authority() {
        let offer = recovery_offer(
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            2,
        );
        let ack =
            SurfaceRecoveryMessage::client_rebind_ack(1, offer, WindowId::try_new(1, 1).unwrap())
                .unwrap();
        assert!(ack.acknowledges(offer));
        let other = recovery_offer(RecoveryClientRole::App, RecoveryCancelKind::None, 0);
        assert!(!ack.acknowledges(other));
        let decoded = SurfaceRecoveryMessage::decode(&ack.encode()).unwrap();
        assert!(decoded.acknowledges(offer));
        assert_eq!(
            decoded.direction(),
            SurfaceRecoveryDirection::ClientToSurface
        );
    }

    #[test]
    fn surface_recovery_contact_ack_repeats_established_contact_authority() {
        let contact = recovery_contact();
        let ack = SurfaceRecoveryMessage::client_contact_ack(1, contact).unwrap();
        assert!(ack.acknowledges_contact(contact));
        let other = SurfaceRecoveryMessage::client_contact(
            1,
            1,
            1,
            RecoveryClientRole::App,
            RecoveryCancelKind::ClientPointer,
            SurfaceRecoveryCheckpoint::MultiWindowInteractive,
            WindowId::try_new(1, 1).unwrap(),
            3,
        )
        .unwrap();
        assert!(!ack.acknowledges_contact(other));
        let decoded = SurfaceRecoveryMessage::decode(&ack.encode()).unwrap();
        assert!(decoded.acknowledges_contact(contact));
        let SurfaceRecoveryPayload::ClientContactAck {
            old_window,
            acknowledged_contact_sequence,
            physical_sequence,
            ..
        } = decoded.payload()
        else {
            panic!("contact ack kind changed")
        };
        assert_eq!(old_window, WindowId::try_new(1, 1).unwrap());
        assert_eq!(acknowledged_contact_sequence, 1);
        assert_eq!(physical_sequence, 2);
    }

    #[test]
    fn supervisor_wire_has_exact_little_endian_command_ack_and_owner_died_branches() {
        assert_eq!(UI_SUPERVISOR_CONTROL_WIRE_SIZE, 64);
        let identity = app_identity(0x0102_0304_0506_0708, 0x1122_3344, 0x5566_7788);
        let messages = [
            supervisor_command(
                0x1112_1314_1516_1718,
                0x2122_2324_2526_2728,
                UiSupervisorOperation::InstallAppEndpoint,
                Some(ShellAppId::Settings),
                Some(identity),
            ),
            supervisor_command(
                2,
                2,
                UiSupervisorOperation::ActivateApp,
                Some(ShellAppId::Settings),
                Some(identity),
            ),
            supervisor_command(3, 3, UiSupervisorOperation::ShowLauncher, None, None),
            supervisor_command(
                4,
                4,
                UiSupervisorOperation::RetireAppEndpoint,
                Some(ShellAppId::Settings),
                Some(identity),
            ),
            supervisor_ack(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                Some(ShellAppId::Settings),
                Some(identity),
            ),
            supervisor_ack(
                2,
                2,
                UiSupervisorOperation::ActivateApp,
                UiSupervisorStatus::Rejected,
                Some(ShellAppId::Settings),
                Some(identity),
            ),
            supervisor_ack(
                3,
                3,
                UiSupervisorOperation::ShowLauncher,
                UiSupervisorStatus::Failed,
                None,
                None,
            ),
            supervisor_owner_died(4, 4, ShellAppId::Settings, identity),
        ];
        for message in messages {
            let wire = message.encode();
            assert_eq!(&wire[0..4], b"USC1");
            assert_eq!(&wire[4..6], &1_u16.to_le_bytes());
            assert_eq!(wire[6], message.kind().raw());
            assert_eq!(wire[7], 0);
            assert!(wire[43..].iter().all(|byte| *byte == 0));
            assert_eq!(UiSupervisorControlMessage::decode(&wire), Ok(message));
            assert_eq!(message.operation().raw(), wire[41]);
            assert_eq!(message.status().map(UiSupervisorStatus::raw), {
                let raw = wire[42];
                (raw != 0).then_some(raw)
            });
        }

        let wire = messages[0].encode();
        assert_eq!(&wire[8..16], &0x1112_1314_1516_1718_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &0x2122_2324_2526_2728_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&wire[32..36], &0x1122_3344_u32.to_le_bytes());
        assert_eq!(&wire[36..40], &0x5566_7788_u32.to_le_bytes());
        assert_eq!(wire[40], ShellAppId::Settings.raw());
        assert_eq!(wire[41], UiSupervisorOperation::InstallAppEndpoint.raw());
        assert_eq!(wire[42], 0);
        assert_eq!(
            messages[0].direction(),
            UiSupervisorControlDirection::InitToSurface
        );
        assert_eq!(
            messages[4].direction(),
            UiSupervisorControlDirection::SurfaceToInit
        );
        assert_eq!(messages[7].kind(), UiSupervisorControlKind::OwnerDied);
        assert_eq!(messages[7].status(), None);
        assert_eq!(messages[7].app(), Some(ShellAppId::Settings));
        assert_eq!(messages[7].identity(), Some(identity));
        assert_eq!(
            messages[7].operation(),
            UiSupervisorOperation::RetireAppEndpoint
        );
    }

    #[test]
    fn supervisor_protocol_enums_are_closed() {
        for (raw, operation) in [
            (1, UiSupervisorOperation::InstallAppEndpoint),
            (2, UiSupervisorOperation::ActivateApp),
            (3, UiSupervisorOperation::ShowLauncher),
            (4, UiSupervisorOperation::RetireAppEndpoint),
        ] {
            assert_eq!(operation.raw(), raw);
            assert_eq!(UiSupervisorOperation::from_raw(raw), Some(operation));
        }
        for (raw, status) in [
            (1, UiSupervisorStatus::Applied),
            (2, UiSupervisorStatus::Rejected),
            (3, UiSupervisorStatus::Failed),
        ] {
            assert_eq!(status.raw(), raw);
            assert_eq!(UiSupervisorStatus::from_raw(raw), Some(status));
        }
        for (raw, kind) in [
            (1, UiSupervisorControlKind::Command),
            (2, UiSupervisorControlKind::Ack),
            (3, UiSupervisorControlKind::OwnerDied),
        ] {
            assert_eq!(kind.raw(), raw);
            assert_eq!(UiSupervisorControlKind::from_raw(raw), Some(kind));
        }
        assert_eq!(UiSupervisorControlKind::from_raw(0), None);
        assert_eq!(UiSupervisorControlKind::from_raw(4), None);
        assert_eq!(UiSupervisorOperation::from_raw(0), None);
        assert_eq!(UiSupervisorOperation::from_raw(5), None);
        assert_eq!(UiSupervisorStatus::from_raw(0), None);
        assert_eq!(UiSupervisorStatus::from_raw(4), None);
    }

    #[test]
    fn supervisor_constructors_enforce_cross_field_canonical_form() {
        let identity = app_identity(1, 2, 3);
        assert_eq!(
            UiSupervisorControlMessage::command(
                0,
                1,
                UiSupervisorOperation::ShowLauncher,
                None,
                None,
            ),
            Err(UiSupervisorWireError::ZeroSenderSequence)
        );
        assert_eq!(
            UiSupervisorControlMessage::command(
                1,
                0,
                UiSupervisorOperation::ShowLauncher,
                None,
                None,
            ),
            Err(UiSupervisorWireError::ZeroTransactionId)
        );
        assert_eq!(
            UiSupervisorControlMessage::command(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                None,
                Some(identity),
            ),
            Err(UiSupervisorWireError::OperationRequiresApp)
        );
        assert_eq!(
            UiSupervisorControlMessage::command(
                1,
                1,
                UiSupervisorOperation::ActivateApp,
                Some(ShellAppId::Phone),
                None,
            ),
            Err(UiSupervisorWireError::OperationRequiresIdentity)
        );
        assert_eq!(
            UiSupervisorControlMessage::command(
                1,
                1,
                UiSupervisorOperation::ShowLauncher,
                Some(ShellAppId::Phone),
                None,
            ),
            Err(UiSupervisorWireError::ShowLauncherHasApp)
        );
        assert_eq!(
            UiSupervisorControlMessage::command(
                1,
                1,
                UiSupervisorOperation::ShowLauncher,
                None,
                Some(identity),
            ),
            Err(UiSupervisorWireError::ShowLauncherHasIdentity)
        );
    }

    #[test]
    fn supervisor_decoder_rejects_noncanonical_headers_bodies_and_fields() {
        let identity = app_identity(0x1111, 0x2222, 0x3333);
        let canonical = supervisor_command(
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            Some(ShellAppId::Phone),
            Some(identity),
        )
        .encode();
        assert_eq!(
            UiSupervisorControlMessage::decode(&canonical[..UI_SUPERVISOR_CONTROL_WIRE_SIZE - 1]),
            Err(UiSupervisorWireError::InvalidWireLength)
        );
        let mut oversized = [0_u8; UI_SUPERVISOR_CONTROL_WIRE_SIZE + 1];
        oversized[..UI_SUPERVISOR_CONTROL_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            UiSupervisorControlMessage::decode(&oversized),
            Err(UiSupervisorWireError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, UiSupervisorWireError::InvalidMagic),
            (4, UiSupervisorWireError::InvalidVersion),
            (6, UiSupervisorWireError::InvalidKind),
            (7, UiSupervisorWireError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0xff;
            assert_eq!(UiSupervisorControlMessage::decode(&wire), Err(expected));
        }
        for offset in SUPERVISOR_OFFSET_BODY_RESERVED..UI_SUPERVISOR_CONTROL_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                UiSupervisorControlMessage::decode(&wire),
                Err(UiSupervisorWireError::NonZeroBodyReserved)
            );
        }

        for (offset, width, expected) in [
            (
                SUPERVISOR_OFFSET_SENDER_SEQUENCE,
                8,
                UiSupervisorWireError::ZeroSenderSequence,
            ),
            (
                SUPERVISOR_OFFSET_TRANSACTION_ID,
                8,
                UiSupervisorWireError::ZeroTransactionId,
            ),
            (
                SUPERVISOR_OFFSET_INSTANCE_ID,
                8,
                UiSupervisorWireError::IncompleteIdentity,
            ),
            (
                SUPERVISOR_OFFSET_PID,
                4,
                UiSupervisorWireError::IncompleteIdentity,
            ),
            (
                SUPERVISOR_OFFSET_PID_GENERATION,
                4,
                UiSupervisorWireError::IncompleteIdentity,
            ),
        ] {
            let mut wire = canonical;
            wire[offset..offset + width].fill(0);
            assert_eq!(UiSupervisorControlMessage::decode(&wire), Err(expected));
        }
        let mut invalid_app = canonical;
        invalid_app[SUPERVISOR_OFFSET_APP] = 4;
        assert_eq!(
            UiSupervisorControlMessage::decode(&invalid_app),
            Err(UiSupervisorWireError::InvalidApp)
        );
        let mut invalid_operation = canonical;
        invalid_operation[SUPERVISOR_OFFSET_OPERATION] = 5;
        assert_eq!(
            UiSupervisorControlMessage::decode(&invalid_operation),
            Err(UiSupervisorWireError::InvalidOperation)
        );
        let mut unexpected_status = canonical;
        unexpected_status[SUPERVISOR_OFFSET_STATUS] = UiSupervisorStatus::Applied.raw();
        assert_eq!(
            UiSupervisorControlMessage::decode(&unexpected_status),
            Err(UiSupervisorWireError::UnexpectedStatus)
        );
        let mut invalid_status = supervisor_ack(
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            Some(ShellAppId::Phone),
            Some(identity),
        )
        .encode();
        invalid_status[SUPERVISOR_OFFSET_STATUS] = 0;
        assert_eq!(
            UiSupervisorControlMessage::decode(&invalid_status),
            Err(UiSupervisorWireError::InvalidStatus)
        );

        let show =
            supervisor_command(1, 1, UiSupervisorOperation::ShowLauncher, None, None).encode();
        let mut show_with_app = show;
        show_with_app[SUPERVISOR_OFFSET_APP] = ShellAppId::Messages.raw();
        assert_eq!(
            UiSupervisorControlMessage::decode(&show_with_app),
            Err(UiSupervisorWireError::ShowLauncherHasApp)
        );
        let mut show_with_identity = show;
        show_with_identity[SUPERVISOR_OFFSET_INSTANCE_ID..SUPERVISOR_OFFSET_INSTANCE_ID + 8]
            .copy_from_slice(&identity.instance_id().to_le_bytes());
        show_with_identity[SUPERVISOR_OFFSET_PID..SUPERVISOR_OFFSET_PID + 4]
            .copy_from_slice(&identity.process().pid().to_le_bytes());
        show_with_identity[SUPERVISOR_OFFSET_PID_GENERATION..SUPERVISOR_OFFSET_PID_GENERATION + 4]
            .copy_from_slice(&identity.process().generation().to_le_bytes());
        assert_eq!(
            UiSupervisorControlMessage::decode(&show_with_identity),
            Err(UiSupervisorWireError::ShowLauncherHasIdentity)
        );
    }

    #[test]
    fn supervisor_owner_died_wire_is_exact_and_canonical() {
        let identity = app_identity(0x0102_0304_0506_0708, 0x1122_3344, 0x5566_7788);
        let message = supervisor_owner_died(
            0x1112_1314_1516_1718,
            0x2122_2324_2526_2728,
            ShellAppId::Messages,
            identity,
        );
        let wire = message.encode();
        assert_eq!(&wire[0..4], b"USC1");
        assert_eq!(&wire[4..6], &UI_SUPERVISOR_CONTROL_VERSION.to_le_bytes());
        assert_eq!(wire[SUPERVISOR_OFFSET_KIND], 3);
        assert_eq!(wire[SUPERVISOR_OFFSET_HEADER_RESERVED], 0);
        assert_eq!(
            &wire[SUPERVISOR_OFFSET_SENDER_SEQUENCE..SUPERVISOR_OFFSET_SENDER_SEQUENCE + 8],
            &message.sender_sequence().to_le_bytes()
        );
        assert_eq!(
            &wire[SUPERVISOR_OFFSET_TRANSACTION_ID..SUPERVISOR_OFFSET_TRANSACTION_ID + 8],
            &message.transaction_id().to_le_bytes()
        );
        assert_eq!(wire[SUPERVISOR_OFFSET_APP], ShellAppId::Messages.raw());
        assert_eq!(
            wire[SUPERVISOR_OFFSET_OPERATION],
            UiSupervisorOperation::RetireAppEndpoint.raw()
        );
        assert_eq!(wire[SUPERVISOR_OFFSET_STATUS], 0);
        assert!(
            wire[SUPERVISOR_OFFSET_BODY_RESERVED..]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(UiSupervisorControlMessage::decode(&wire), Ok(message));
        assert_eq!(message.kind(), UiSupervisorControlKind::OwnerDied);
        assert_eq!(
            message.direction(),
            UiSupervisorControlDirection::SurfaceToInit
        );
        assert_eq!(
            message.payload(),
            UiSupervisorControlPayload::OwnerDied {
                app: ShellAppId::Messages,
                identity,
            }
        );
        assert_eq!(
            UiSupervisorControlMessage::owner_died(0, 1, ShellAppId::Messages, identity),
            Err(UiSupervisorWireError::ZeroSenderSequence)
        );
        assert_eq!(
            UiSupervisorControlMessage::owner_died(1, 0, ShellAppId::Messages, identity),
            Err(UiSupervisorWireError::ZeroTransactionId)
        );
    }

    #[test]
    fn supervisor_owner_died_decoder_rejects_every_noncanonical_tuple() {
        let identity = app_identity(9, 17, 4);
        let canonical = supervisor_owner_died(1, 2, ShellAppId::Phone, identity).encode();

        let mut wrong_operation = canonical;
        wrong_operation[SUPERVISOR_OFFSET_OPERATION] = UiSupervisorOperation::ActivateApp.raw();
        assert_eq!(
            UiSupervisorControlMessage::decode(&wrong_operation),
            Err(UiSupervisorWireError::OwnerDiedInvalidOperation)
        );
        let mut unknown_operation = canonical;
        unknown_operation[SUPERVISOR_OFFSET_OPERATION] = 0xff;
        assert_eq!(
            UiSupervisorControlMessage::decode(&unknown_operation),
            Err(UiSupervisorWireError::OwnerDiedInvalidOperation)
        );
        let mut nonzero_status = canonical;
        nonzero_status[SUPERVISOR_OFFSET_STATUS] = UiSupervisorStatus::Applied.raw();
        assert_eq!(
            UiSupervisorControlMessage::decode(&nonzero_status),
            Err(UiSupervisorWireError::OwnerDiedUnexpectedStatus)
        );
        let mut missing_app = canonical;
        missing_app[SUPERVISOR_OFFSET_APP] = 0;
        assert_eq!(
            UiSupervisorControlMessage::decode(&missing_app),
            Err(UiSupervisorWireError::OwnerDiedRequiresApp)
        );
        let mut missing_identity = canonical;
        missing_identity[SUPERVISOR_OFFSET_INSTANCE_ID..SUPERVISOR_OFFSET_APP].fill(0);
        assert_eq!(
            UiSupervisorControlMessage::decode(&missing_identity),
            Err(UiSupervisorWireError::OwnerDiedRequiresIdentity)
        );
        let mut partial_identity = missing_identity;
        partial_identity[SUPERVISOR_OFFSET_PID..SUPERVISOR_OFFSET_PID + 4]
            .copy_from_slice(&identity.process().pid().to_le_bytes());
        assert_eq!(
            UiSupervisorControlMessage::decode(&partial_identity),
            Err(UiSupervisorWireError::IncompleteIdentity)
        );
    }

    #[test]
    fn supervisor_owner_died_retires_atomically_then_allows_fresh_generation() {
        let old = app_identity(1, 41, 7);
        let fresh = app_identity(2, 41, 8);
        let app = Some(ShellAppId::Settings);
        let mut tracker = UiSupervisorTracker::new();
        accept_supervisor_pair(
            &mut tracker,
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        accept_supervisor_pair(
            &mut tracker,
            2,
            2,
            UiSupervisorOperation::ActivateApp,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );

        let died = supervisor_owner_died(3, 3, ShellAppId::Settings, old);
        tracker
            .accept(UiSupervisorControlMessage::decode(&died.encode()).unwrap())
            .unwrap();
        assert_eq!(tracker.transaction().installed_app(), None);
        assert_eq!(tracker.transaction().installed_identity(), None);
        assert_eq!(tracker.transaction().retired_identity(), Some(old));
        assert_eq!(tracker.transaction().last_transaction_id(), Some(3));
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::Launcher
        );

        tracker
            .accept(supervisor_command(
                3,
                4,
                UiSupervisorOperation::InstallAppEndpoint,
                app,
                Some(fresh),
            ))
            .unwrap();
        tracker
            .accept(supervisor_ack(
                4,
                4,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                app,
                Some(fresh),
            ))
            .unwrap();
        tracker
            .accept(supervisor_command(
                4,
                5,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(fresh),
            ))
            .unwrap();
        tracker
            .accept(supervisor_ack(
                5,
                5,
                UiSupervisorOperation::ActivateApp,
                UiSupervisorStatus::Applied,
                app,
                Some(fresh),
            ))
            .unwrap();
        assert_eq!(tracker.transaction().installed_identity(), Some(fresh));
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::App {
                app: ShellAppId::Settings,
                identity: fresh,
            }
        );
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::InitToSurface),
            Some(4)
        );
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::SurfaceToInit),
            Some(5)
        );
    }

    #[test]
    fn supervisor_owner_died_rejects_pending_mismatch_stale_and_replay_transactionally() {
        let identity = app_identity(10, 50, 3);
        let app = Some(ShellAppId::Phone);
        let mut tracker = UiSupervisorTracker::new();
        accept_supervisor_pair(
            &mut tracker,
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(identity),
        );
        tracker
            .accept(supervisor_command(
                2,
                2,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(identity),
            ))
            .unwrap();

        let pending = tracker;
        assert_eq!(
            tracker.accept(supervisor_owner_died(2, 3, ShellAppId::Phone, identity,)),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::TransactionBusy
            ))
        );
        assert_eq!(tracker, pending);
        tracker
            .accept(supervisor_ack(
                2,
                2,
                UiSupervisorOperation::ActivateApp,
                UiSupervisorStatus::Applied,
                app,
                Some(identity),
            ))
            .unwrap();

        for (reported_app, reported_identity, expected) in [
            (
                ShellAppId::Messages,
                identity,
                UiSupervisorTransactionError::AppMismatch,
            ),
            (
                ShellAppId::Phone,
                app_identity(10, 50, 4),
                UiSupervisorTransactionError::StaleIdentity,
            ),
        ] {
            let before = tracker;
            assert_eq!(
                tracker.accept(supervisor_owner_died(3, 3, reported_app, reported_identity,)),
                Err(UiSupervisorTrackerError::Transaction(expected))
            );
            assert_eq!(tracker, before);
        }

        let died = supervisor_owner_died(3, 3, ShellAppId::Phone, identity);
        tracker.accept(died).unwrap();
        assert_eq!(
            tracker.accept(died),
            Err(UiSupervisorTrackerError::Sequence(
                UiSupervisorSequenceError::SequenceReplay
            ))
        );
        let before = tracker;
        assert_eq!(
            tracker.accept(supervisor_owner_died(4, 3, ShellAppId::Phone, identity,)),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::TransactionReplay
            ))
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker.accept(supervisor_owner_died(4, 4, ShellAppId::Phone, identity,)),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::NoInstalledEndpoint
            ))
        );
        assert_eq!(tracker, before);
    }

    #[test]
    fn supervisor_owner_died_shares_surface_direction_sequence_with_acks() {
        let identity = app_identity(1, 2, 3);
        let mut sequence = UiSupervisorSequenceTracker::new();
        sequence
            .accept(supervisor_command(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                Some(ShellAppId::Phone),
                Some(identity),
            ))
            .unwrap();
        sequence
            .accept(supervisor_owner_died(1, 2, ShellAppId::Phone, identity))
            .unwrap();
        sequence
            .accept(supervisor_ack(
                2,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                Some(ShellAppId::Phone),
                Some(identity),
            ))
            .unwrap();
        assert_eq!(
            sequence.accept(supervisor_owner_died(2, 3, ShellAppId::Phone, identity,)),
            Err(UiSupervisorSequenceError::SequenceReplay)
        );
        assert_eq!(
            sequence.accept(supervisor_owner_died(4, 3, ShellAppId::Phone, identity,)),
            Err(UiSupervisorSequenceError::SequenceGap)
        );
        sequence
            .accept(supervisor_owner_died(3, 3, ShellAppId::Phone, identity))
            .unwrap();
        assert_eq!(
            sequence.last(UiSupervisorControlDirection::InitToSurface),
            Some(1)
        );
        assert_eq!(
            sequence.last(UiSupervisorControlDirection::SurfaceToInit),
            Some(3)
        );
    }

    #[test]
    fn supervisor_tracker_proves_full_endpoint_focus_replacement_sequence() {
        let old = app_identity(1, 41, 7);
        let fresh = app_identity(2, 41, 8);
        let app = Some(ShellAppId::Settings);
        let mut tracker = UiSupervisorTracker::new();

        accept_supervisor_pair(
            &mut tracker,
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        assert_eq!(tracker.transaction().installed_identity(), Some(old));
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::Launcher
        );
        accept_supervisor_pair(
            &mut tracker,
            2,
            2,
            UiSupervisorOperation::ActivateApp,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::App {
                app: ShellAppId::Settings,
                identity: old,
            }
        );
        accept_supervisor_pair(
            &mut tracker,
            3,
            3,
            UiSupervisorOperation::ShowLauncher,
            UiSupervisorStatus::Applied,
            None,
            None,
        );
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::Launcher
        );
        assert_eq!(tracker.transaction().installed_identity(), Some(old));
        accept_supervisor_pair(
            &mut tracker,
            4,
            4,
            UiSupervisorOperation::ActivateApp,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        accept_supervisor_pair(
            &mut tracker,
            5,
            5,
            UiSupervisorOperation::RetireAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        assert_eq!(tracker.transaction().installed_identity(), None);
        assert_eq!(tracker.transaction().retired_identity(), Some(old));
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::Launcher
        );
        accept_supervisor_pair(
            &mut tracker,
            6,
            6,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(fresh),
        );
        assert_eq!(tracker.transaction().installed_identity(), Some(fresh));
        assert_eq!(tracker.transaction().installed_app(), app);
        assert_eq!(tracker.transaction().last_transaction_id(), Some(6));
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::InitToSurface),
            Some(6)
        );
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::SurfaceToInit),
            Some(6)
        );
    }

    #[test]
    fn supervisor_tracker_rejects_stale_identity_and_pid_generation() {
        let old = app_identity(10, 50, 3);
        let fresh = app_identity(11, 50, 4);
        let app = Some(ShellAppId::Messages);
        let mut tracker = UiSupervisorTracker::new();
        accept_supervisor_pair(
            &mut tracker,
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );
        accept_supervisor_pair(
            &mut tracker,
            2,
            2,
            UiSupervisorOperation::RetireAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(old),
        );

        for stale in [
            old,
            app_identity(old.instance_id(), 51, 4),
            app_identity(11, old.process().pid(), old.process().generation()),
        ] {
            assert_eq!(
                tracker.accept(supervisor_command(
                    3,
                    3,
                    UiSupervisorOperation::InstallAppEndpoint,
                    app,
                    Some(stale),
                )),
                Err(UiSupervisorTrackerError::Transaction(
                    UiSupervisorTransactionError::StaleIdentity
                ))
            );
            assert_eq!(
                tracker
                    .sequence()
                    .last(UiSupervisorControlDirection::InitToSurface),
                Some(2)
            );
        }
        accept_supervisor_pair(
            &mut tracker,
            3,
            3,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(fresh),
        );

        for stale in [
            app_identity(fresh.instance_id(), fresh.process().pid(), 3),
            app_identity(12, fresh.process().pid(), fresh.process().generation()),
        ] {
            assert_eq!(
                tracker.accept(supervisor_command(
                    4,
                    4,
                    UiSupervisorOperation::ActivateApp,
                    app,
                    Some(stale),
                )),
                Err(UiSupervisorTrackerError::Transaction(
                    UiSupervisorTransactionError::StaleIdentity
                ))
            );
        }
        tracker
            .accept(supervisor_command(
                4,
                4,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(fresh),
            ))
            .unwrap();
    }

    #[test]
    fn supervisor_tracker_rejects_replay_gap_and_wrong_ack_transactionally() {
        let identity = app_identity(1, 7, 2);
        let app = Some(ShellAppId::Phone);
        let mut tracker = UiSupervisorTracker::new();
        tracker
            .accept(supervisor_command(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                app,
                Some(identity),
            ))
            .unwrap();

        assert_eq!(
            tracker.accept(supervisor_ack(
                2,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                app,
                Some(identity),
            )),
            Err(UiSupervisorTrackerError::Sequence(
                UiSupervisorSequenceError::SequenceGap
            ))
        );
        assert_eq!(
            tracker.accept(supervisor_ack(
                1,
                2,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                app,
                Some(identity),
            )),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::TransactionMismatch
            ))
        );
        assert_eq!(
            tracker.accept(supervisor_ack(
                1,
                1,
                UiSupervisorOperation::ActivateApp,
                UiSupervisorStatus::Applied,
                app,
                Some(identity),
            )),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::AckMismatch
            ))
        );
        assert_eq!(
            tracker.accept(supervisor_ack(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                UiSupervisorStatus::Applied,
                app,
                Some(app_identity(1, 7, 3)),
            )),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::AckMismatch
            ))
        );
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::SurfaceToInit),
            None
        );
        let correct_ack = supervisor_ack(
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(identity),
        );
        tracker.accept(correct_ack).unwrap();
        assert_eq!(
            tracker.accept(correct_ack),
            Err(UiSupervisorTrackerError::Sequence(
                UiSupervisorSequenceError::SequenceReplay
            ))
        );
        assert_eq!(
            tracker.accept(supervisor_command(
                3,
                2,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(identity),
            )),
            Err(UiSupervisorTrackerError::Sequence(
                UiSupervisorSequenceError::SequenceGap
            ))
        );
        assert_eq!(
            tracker.accept(supervisor_command(
                2,
                1,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(identity),
            )),
            Err(UiSupervisorTrackerError::Transaction(
                UiSupervisorTransactionError::TransactionReplay
            ))
        );
        assert_eq!(
            tracker
                .sequence()
                .last(UiSupervisorControlDirection::InitToSurface),
            Some(1)
        );
        tracker
            .accept(supervisor_command(
                2,
                2,
                UiSupervisorOperation::ActivateApp,
                app,
                Some(identity),
            ))
            .unwrap();
    }

    #[test]
    fn supervisor_rejected_and_failed_acks_do_not_apply_state() {
        let identity = app_identity(1, 9, 1);
        let app = Some(ShellAppId::Settings);
        let mut tracker = UiSupervisorTracker::new();
        accept_supervisor_pair(
            &mut tracker,
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Rejected,
            app,
            Some(identity),
        );
        assert_eq!(tracker.transaction().installed_identity(), None);
        accept_supervisor_pair(
            &mut tracker,
            2,
            2,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Failed,
            app,
            Some(identity),
        );
        assert_eq!(tracker.transaction().installed_identity(), None);
        accept_supervisor_pair(
            &mut tracker,
            3,
            3,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            app,
            Some(identity),
        );
        accept_supervisor_pair(
            &mut tracker,
            4,
            4,
            UiSupervisorOperation::ActivateApp,
            UiSupervisorStatus::Rejected,
            app,
            Some(identity),
        );
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::Launcher
        );
        accept_supervisor_pair(
            &mut tracker,
            5,
            5,
            UiSupervisorOperation::ActivateApp,
            UiSupervisorStatus::Applied,
            app,
            Some(identity),
        );
        accept_supervisor_pair(
            &mut tracker,
            6,
            6,
            UiSupervisorOperation::RetireAppEndpoint,
            UiSupervisorStatus::Failed,
            app,
            Some(identity),
        );
        assert_eq!(tracker.transaction().installed_identity(), Some(identity));
        assert_eq!(
            tracker.transaction().active_target(),
            UiSupervisorActiveTarget::App {
                app: ShellAppId::Settings,
                identity,
            }
        );
    }

    #[test]
    fn supervisor_sequence_and_transaction_exhaustion_are_explicit() {
        let identity = app_identity(1, 2, 3);
        let command = supervisor_command(
            u64::MAX,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            Some(ShellAppId::Phone),
            Some(identity),
        );
        let mut sequence = UiSupervisorSequenceTracker {
            init_to_surface: Some(u64::MAX),
            surface_to_init: None,
        };
        assert_eq!(
            sequence.accept(command),
            Err(UiSupervisorSequenceError::SequenceExhausted)
        );

        let mut transaction = UiSupervisorTransactionTracker {
            last_transaction_id: Some(u64::MAX),
            ..UiSupervisorTransactionTracker::new()
        };
        assert_eq!(
            transaction.accept(command),
            Err(UiSupervisorTransactionError::TransactionExhausted)
        );

        let mut completed = UiSupervisorTransactionTracker::new();
        completed
            .accept(supervisor_command(
                1,
                1,
                UiSupervisorOperation::InstallAppEndpoint,
                Some(ShellAppId::Phone),
                Some(identity),
            ))
            .unwrap();
        let ack = supervisor_ack(
            1,
            1,
            UiSupervisorOperation::InstallAppEndpoint,
            UiSupervisorStatus::Applied,
            Some(ShellAppId::Phone),
            Some(identity),
        );
        completed.accept(ack).unwrap();
        assert_eq!(
            completed.accept(ack),
            Err(UiSupervisorTransactionError::AckReplay)
        );
    }

    #[test]
    fn lifecycle_wire_has_four_exact_little_endian_canonical_branches() {
        assert_eq!(APP_LIFECYCLE_WIRE_SIZE, 64);
        let identity = app_identity(0x0102_0304_0506_0708, 0x1122_3344, 0x5566_7788);
        let messages = [
            AppLifecycleMessage::request(
                0x1112_1314_1516_1718,
                0x2122_2324_2526_2728,
                ShellAppId::Settings,
                AppLifecycleAction::Launch,
                None,
            )
            .unwrap(),
            AppLifecycleMessage::state_changed(
                2,
                7,
                ShellAppId::Settings,
                AppLifecycleState::Active,
                AppLifecycleReason::Completed,
                Some(identity),
            )
            .unwrap(),
            AppLifecycleMessage::command(
                3,
                8,
                ShellAppId::Settings,
                AppLifecycleAction::Suspend,
                identity,
            )
            .unwrap(),
            AppLifecycleMessage::ack(
                4,
                8,
                ShellAppId::Settings,
                AppLifecycleAction::Suspend,
                AppLifecycleStatus::Applied,
                identity,
            )
            .unwrap(),
        ];
        for message in messages {
            let wire = message.encode();
            assert_eq!(&wire[0..4], b"ALC1");
            assert_eq!(AppLifecycleMessage::decode(&wire), Ok(message));
            assert_eq!(wire[7], 0);
            assert!(wire[43..].iter().all(|byte| *byte == 0));
        }

        let request = messages[0].encode();
        assert_eq!(&request[4..8], &[1, 0, 1, 0]);
        assert_eq!(&request[8..16], &0x1112_1314_1516_1718_u64.to_le_bytes());
        assert_eq!(&request[16..24], &0x2122_2324_2526_2728_u64.to_le_bytes());
        assert!(request[24..40].iter().all(|byte| *byte == 0));
        assert_eq!(&request[40..43], &[3, 1, 0]);

        let command = messages[2].encode();
        assert_eq!(&command[24..32], &identity.instance_id().to_le_bytes());
        assert_eq!(&command[32..36], &identity.process().pid().to_le_bytes());
        assert_eq!(
            &command[36..40],
            &identity.process().generation().to_le_bytes()
        );
        assert_eq!(&command[40..43], &[3, 3, 0]);
        assert_eq!(
            messages[0].direction(),
            AppLifecycleDirection::LauncherToInit
        );
        assert_eq!(
            messages[1].direction(),
            AppLifecycleDirection::InitToLauncher
        );
        assert_eq!(messages[2].direction(), AppLifecycleDirection::InitToApp);
        assert_eq!(messages[3].direction(), AppLifecycleDirection::AppToInit);
    }

    #[test]
    fn lifecycle_enum_and_identity_domains_are_closed_and_nonzero() {
        for (raw, action) in [
            AppLifecycleAction::Launch,
            AppLifecycleAction::Activate,
            AppLifecycleAction::Suspend,
            AppLifecycleAction::Resume,
            AppLifecycleAction::Terminate,
        ]
        .into_iter()
        .map(|value| (value.raw(), value))
        {
            assert_eq!(AppLifecycleAction::from_raw(raw), Some(action));
        }
        assert_eq!(AppLifecycleAction::from_raw(0), None);
        assert_eq!(AppLifecycleAction::from_raw(6), None);
        assert_eq!(AppLifecycleState::from_raw(0), None);
        assert_eq!(AppLifecycleState::from_raw(12), None);
        assert_eq!(AppLifecycleReason::from_raw(0), None);
        assert_eq!(AppLifecycleReason::from_raw(8), None);
        assert_eq!(AppLifecycleStatus::from_raw(0), None);
        assert_eq!(AppLifecycleStatus::from_raw(4), None);
        assert_eq!(AppLifecycleKind::from_raw(0), None);
        assert_eq!(AppLifecycleKind::from_raw(5), None);
        assert_eq!(
            GenerationQualifiedPid::try_new(0, 1),
            Err(AppIdentityError::ZeroPid)
        );
        assert_eq!(
            GenerationQualifiedPid::try_new(1, 0),
            Err(AppIdentityError::ZeroPidGeneration)
        );
        assert_eq!(
            AppInstanceIdentity::try_new(0, 1, 1),
            Err(AppIdentityError::ZeroInstanceId)
        );
        let identity = app_identity(9, 10, 11);
        assert_eq!(identity.instance_id(), 9);
        assert_eq!(identity.process().pid(), 10);
        assert_eq!(identity.process().generation(), 11);
    }

    #[test]
    fn lifecycle_constructors_enforce_counters_identity_and_kind_cross_fields() {
        let identity = app_identity(1, 2, 3);
        assert_eq!(
            AppLifecycleMessage::request(0, 1, ShellAppId::Phone, AppLifecycleAction::Launch, None,),
            Err(AppLifecycleWireError::ZeroSenderSequence)
        );
        assert_eq!(
            AppLifecycleMessage::request(1, 0, ShellAppId::Phone, AppLifecycleAction::Launch, None,),
            Err(AppLifecycleWireError::ZeroTransactionId)
        );
        assert_eq!(
            AppLifecycleMessage::request(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleAction::Launch,
                Some(identity),
            ),
            Err(AppLifecycleWireError::LaunchRequestHasIdentity)
        );
        assert_eq!(
            AppLifecycleMessage::request(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleAction::Suspend,
                None,
            ),
            Err(AppLifecycleWireError::RequestRequiresIdentity)
        );
        assert_eq!(
            AppLifecycleMessage::state_changed(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleState::Active,
                AppLifecycleReason::Requested,
                Some(identity),
            ),
            Err(AppLifecycleWireError::InvalidStateReason)
        );
        assert_eq!(
            AppLifecycleMessage::state_changed(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleState::Active,
                AppLifecycleReason::Completed,
                None,
            ),
            Err(AppLifecycleWireError::StateRequiresIdentity)
        );
        assert_eq!(
            AppLifecycleMessage::state_changed(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleState::Failed,
                AppLifecycleReason::SpawnFailed,
                Some(identity),
            ),
            Err(AppLifecycleWireError::SpawnFailureHasIdentity)
        );
    }

    #[test]
    fn lifecycle_decoder_rejects_lengths_headers_and_every_reserved_byte() {
        let canonical = lifecycle_request(1, 1, AppLifecycleAction::Launch, None).encode();
        assert_eq!(
            AppLifecycleMessage::decode(&canonical[..APP_LIFECYCLE_WIRE_SIZE - 1]),
            Err(AppLifecycleWireError::InvalidWireLength)
        );
        let mut oversized = [0_u8; APP_LIFECYCLE_WIRE_SIZE + 1];
        oversized[..APP_LIFECYCLE_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            AppLifecycleMessage::decode(&oversized),
            Err(AppLifecycleWireError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, AppLifecycleWireError::InvalidMagic),
            (4, AppLifecycleWireError::InvalidVersion),
            (6, AppLifecycleWireError::InvalidKind),
            (7, AppLifecycleWireError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(AppLifecycleMessage::decode(&wire), Err(expected));
        }
        for offset in LIFECYCLE_OFFSET_BODY_RESERVED..APP_LIFECYCLE_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                AppLifecycleMessage::decode(&wire),
                Err(AppLifecycleWireError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn lifecycle_decoder_rejects_each_invalid_or_noncanonical_field() {
        let identity = app_identity(1, 2, 3);
        let launch = lifecycle_request(1, 1, AppLifecycleAction::Launch, None).encode();
        for (offset, expected) in [
            (
                LIFECYCLE_OFFSET_SENDER_SEQUENCE,
                AppLifecycleWireError::ZeroSenderSequence,
            ),
            (
                LIFECYCLE_OFFSET_TRANSACTION_ID,
                AppLifecycleWireError::ZeroTransactionId,
            ),
        ] {
            let mut wire = launch;
            wire[offset..offset + 8].fill(0);
            assert_eq!(AppLifecycleMessage::decode(&wire), Err(expected));
        }
        let mut invalid_app = launch;
        invalid_app[LIFECYCLE_OFFSET_APP] = 4;
        assert_eq!(
            AppLifecycleMessage::decode(&invalid_app),
            Err(AppLifecycleWireError::InvalidApp)
        );
        let mut invalid_action = launch;
        invalid_action[LIFECYCLE_OFFSET_ACTION_OR_STATE] = 6;
        assert_eq!(
            AppLifecycleMessage::decode(&invalid_action),
            Err(AppLifecycleWireError::InvalidAction)
        );
        let mut unexpected_status = launch;
        unexpected_status[LIFECYCLE_OFFSET_REASON_OR_STATUS] = 1;
        assert_eq!(
            AppLifecycleMessage::decode(&unexpected_status),
            Err(AppLifecycleWireError::UnexpectedReasonOrStatus)
        );
        for offset in [
            LIFECYCLE_OFFSET_INSTANCE_ID,
            LIFECYCLE_OFFSET_PID,
            LIFECYCLE_OFFSET_PID_GENERATION,
        ] {
            let mut partial = launch;
            partial[offset] = 1;
            assert_eq!(
                AppLifecycleMessage::decode(&partial),
                Err(AppLifecycleWireError::IncompleteIdentity),
                "offset {offset}"
            );
        }

        let state = lifecycle_state(
            1,
            1,
            AppLifecycleState::Active,
            AppLifecycleReason::Completed,
            Some(identity),
        )
        .encode();
        let mut invalid_state = state;
        invalid_state[LIFECYCLE_OFFSET_ACTION_OR_STATE] = 12;
        assert_eq!(
            AppLifecycleMessage::decode(&invalid_state),
            Err(AppLifecycleWireError::InvalidState)
        );
        let mut invalid_reason = state;
        invalid_reason[LIFECYCLE_OFFSET_REASON_OR_STATUS] = 8;
        assert_eq!(
            AppLifecycleMessage::decode(&invalid_reason),
            Err(AppLifecycleWireError::InvalidReason)
        );
        let mut wrong_reason = state;
        wrong_reason[LIFECYCLE_OFFSET_REASON_OR_STATUS] = AppLifecycleReason::Requested.raw();
        assert_eq!(
            AppLifecycleMessage::decode(&wrong_reason),
            Err(AppLifecycleWireError::InvalidStateReason)
        );

        let ack = lifecycle_ack(
            1,
            1,
            AppLifecycleAction::Launch,
            AppLifecycleStatus::Applied,
            identity,
        )
        .encode();
        let mut invalid_status = ack;
        invalid_status[LIFECYCLE_OFFSET_REASON_OR_STATUS] = 4;
        assert_eq!(
            AppLifecycleMessage::decode(&invalid_status),
            Err(AppLifecycleWireError::InvalidStatus)
        );
    }

    #[test]
    fn lifecycle_state_reason_matrix_is_closed() {
        let identity = app_identity(1, 2, 3);
        let valid = [
            (AppLifecycleState::NotRunning, AppLifecycleReason::Completed),
            (AppLifecycleState::Launching, AppLifecycleReason::Requested),
            (AppLifecycleState::Inactive, AppLifecycleReason::Completed),
            (AppLifecycleState::Activating, AppLifecycleReason::Requested),
            (AppLifecycleState::Active, AppLifecycleReason::Completed),
            (AppLifecycleState::Suspending, AppLifecycleReason::Requested),
            (AppLifecycleState::Suspended, AppLifecycleReason::Completed),
            (AppLifecycleState::Resuming, AppLifecycleReason::Requested),
            (
                AppLifecycleState::Terminating,
                AppLifecycleReason::Requested,
            ),
            (
                AppLifecycleState::Crashed,
                AppLifecycleReason::ProcessExited,
            ),
            (
                AppLifecycleState::Failed,
                AppLifecycleReason::CommandRejected,
            ),
            (AppLifecycleState::Failed, AppLifecycleReason::CommandFailed),
            (
                AppLifecycleState::Failed,
                AppLifecycleReason::ProtocolViolation,
            ),
        ];
        for (state, reason) in valid {
            assert!(
                AppLifecycleMessage::state_changed(
                    1,
                    1,
                    ShellAppId::Phone,
                    state,
                    reason,
                    Some(identity),
                )
                .is_ok(),
                "{state:?} {reason:?}"
            );
        }
        assert!(
            AppLifecycleMessage::state_changed(
                1,
                1,
                ShellAppId::Phone,
                AppLifecycleState::Failed,
                AppLifecycleReason::SpawnFailed,
                None,
            )
            .is_ok()
        );
    }

    #[test]
    fn lifecycle_direction_sequences_are_independent_contiguous_and_transactional() {
        let identity = app_identity(1, 2, 3);
        let mut tracker = AppLifecycleSequenceTracker::new();
        for message in [
            lifecycle_request(1, 1, AppLifecycleAction::Launch, None),
            lifecycle_state(
                1,
                1,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(identity),
            ),
            lifecycle_command(1, 1, AppLifecycleAction::Launch, identity),
            lifecycle_ack(
                1,
                1,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                identity,
            ),
            lifecycle_state(
                2,
                1,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(identity),
            ),
        ] {
            tracker.accept(message).unwrap();
        }
        for direction in [
            AppLifecycleDirection::LauncherToInit,
            AppLifecycleDirection::InitToApp,
            AppLifecycleDirection::AppToInit,
        ] {
            assert_eq!(tracker.last(direction), Some(1));
        }
        assert_eq!(tracker.last(AppLifecycleDirection::InitToLauncher), Some(2));
        let before = tracker;
        assert_eq!(
            tracker.accept(lifecycle_request(
                1,
                2,
                AppLifecycleAction::Terminate,
                Some(identity),
            )),
            Err(AppLifecycleSequenceError::SequenceReplay)
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker.accept(lifecycle_request(
                3,
                2,
                AppLifecycleAction::Terminate,
                Some(identity),
            )),
            Err(AppLifecycleSequenceError::SequenceGap)
        );
        assert_eq!(tracker, before);

        let mut exhausted = AppLifecycleSequenceTracker {
            launcher_to_init: Some(u64::MAX),
            ..AppLifecycleSequenceTracker::new()
        };
        let snapshot = exhausted;
        assert_eq!(
            exhausted.accept(lifecycle_request(
                u64::MAX,
                9,
                AppLifecycleAction::Launch,
                None,
            )),
            Err(AppLifecycleSequenceError::SequenceExhausted)
        );
        assert_eq!(exhausted, snapshot);
    }

    #[test]
    fn lifecycle_transaction_tracker_rejects_out_of_order_stale_and_replayed_legs() {
        let identity = app_identity(1, 2, 3);
        let stale = app_identity(1, 2, 4);
        let mut tracker = AppLifecycleTransactionTracker::new();
        tracker
            .accept(lifecycle_request(1, 7, AppLifecycleAction::Launch, None))
            .unwrap();
        let before = tracker;
        assert_eq!(
            tracker.accept(lifecycle_command(
                1,
                7,
                AppLifecycleAction::Launch,
                identity,
            )),
            Err(AppLifecycleTransactionError::IntermediateStateRequired)
        );
        assert_eq!(tracker, before);
        tracker
            .accept(lifecycle_state(
                1,
                7,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(identity),
            ))
            .unwrap();
        let before = tracker;
        assert_eq!(
            tracker.accept(lifecycle_command(
                1,
                8,
                AppLifecycleAction::Launch,
                identity,
            )),
            Err(AppLifecycleTransactionError::TransactionMismatch)
        );
        assert_eq!(tracker, before);
        assert_eq!(
            tracker.accept(lifecycle_command(1, 7, AppLifecycleAction::Launch, stale,)),
            Err(AppLifecycleTransactionError::StaleIdentity)
        );
        assert_eq!(tracker, before);
        tracker
            .accept(lifecycle_command(
                1,
                7,
                AppLifecycleAction::Launch,
                identity,
            ))
            .unwrap();
        let before_ack = tracker;
        assert_eq!(
            tracker.accept(lifecycle_state(
                2,
                7,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(identity),
            )),
            Err(AppLifecycleTransactionError::CompletionBeforeAck)
        );
        assert_eq!(tracker, before_ack);
        tracker
            .accept(lifecycle_ack(
                1,
                7,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                identity,
            ))
            .unwrap();
        let acked = tracker;
        assert_eq!(
            tracker.accept(lifecycle_ack(
                2,
                7,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                identity,
            )),
            Err(AppLifecycleTransactionError::AckReplay)
        );
        assert_eq!(tracker, acked);
        tracker
            .accept(lifecycle_state(
                2,
                7,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(identity),
            ))
            .unwrap();
        assert_eq!(tracker.last_transaction_id(), Some(7));
        assert_eq!(tracker.running_identity(), Some(identity));
        let completed = tracker;
        assert_eq!(
            tracker.accept(lifecycle_request(
                2,
                7,
                AppLifecycleAction::Activate,
                Some(identity),
            )),
            Err(AppLifecycleTransactionError::TransactionReplay)
        );
        assert_eq!(tracker, completed);
    }

    #[test]
    fn lifecycle_transaction_tracker_rejects_actions_from_the_wrong_stable_state() {
        let identity = app_identity(1, 2, 3);
        let mut tracker = AppLifecycleTransactionTracker::new();
        for message in [
            lifecycle_request(1, 1, AppLifecycleAction::Launch, None),
            lifecycle_state(
                1,
                1,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(identity),
            ),
            lifecycle_command(1, 1, AppLifecycleAction::Launch, identity),
            lifecycle_ack(
                1,
                1,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                identity,
            ),
            lifecycle_state(
                2,
                1,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(identity),
            ),
        ] {
            tracker.accept(message).unwrap();
        }
        assert_eq!(tracker.state(), AppLifecycleState::Inactive);
        for action in [AppLifecycleAction::Suspend, AppLifecycleAction::Resume] {
            let before = tracker;
            assert_eq!(
                tracker.accept(lifecycle_request(2, 2, action, Some(identity))),
                Err(AppLifecycleTransactionError::InvalidStateForAction)
            );
            assert_eq!(tracker, before);
        }
    }

    #[test]
    fn lifecycle_composite_resets_only_app_directions_for_a_fresh_instance() {
        let old = app_identity(1, 7, 1);
        let stale_pid = app_identity(2, 7, 1);
        let fresh = app_identity(2, 7, 2);
        let mut tracker = AppLifecycleTracker::new();
        for message in [
            lifecycle_request(1, 1, AppLifecycleAction::Launch, None),
            lifecycle_state(
                1,
                1,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(old),
            ),
            lifecycle_command(1, 1, AppLifecycleAction::Launch, old),
            lifecycle_ack(
                1,
                1,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                old,
            ),
            lifecycle_state(
                2,
                1,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(old),
            ),
            lifecycle_state(
                3,
                2,
                AppLifecycleState::Crashed,
                AppLifecycleReason::ProcessExited,
                Some(old),
            ),
            lifecycle_request(2, 3, AppLifecycleAction::Launch, None),
        ] {
            tracker.accept(message).unwrap();
        }
        let before_stale_bind = tracker;
        assert_eq!(
            tracker.accept(lifecycle_state(
                4,
                3,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(stale_pid),
            )),
            Err(AppLifecycleTrackerError::Transaction(
                AppLifecycleTransactionError::StaleIdentity
            ))
        );
        assert_eq!(tracker, before_stale_bind);
        tracker
            .accept(lifecycle_state(
                4,
                3,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(fresh),
            ))
            .unwrap();
        assert_eq!(tracker.sequence().app_endpoint_identity(), Some(fresh));
        assert_eq!(
            tracker
                .sequence()
                .last(AppLifecycleDirection::LauncherToInit),
            Some(2)
        );
        assert_eq!(
            tracker
                .sequence()
                .last(AppLifecycleDirection::InitToLauncher),
            Some(4)
        );
        assert_eq!(
            tracker.sequence().last(AppLifecycleDirection::InitToApp),
            None
        );
        assert_eq!(
            tracker.sequence().last(AppLifecycleDirection::AppToInit),
            None
        );
        let reset = tracker;
        assert_eq!(
            tracker.accept(lifecycle_command(1, 3, AppLifecycleAction::Launch, old,)),
            Err(AppLifecycleTrackerError::Transaction(
                AppLifecycleTransactionError::StaleIdentity
            ))
        );
        assert_eq!(tracker, reset);
        tracker
            .accept(lifecycle_command(1, 3, AppLifecycleAction::Launch, fresh))
            .unwrap();
        tracker
            .accept(lifecycle_ack(
                1,
                3,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                fresh,
            ))
            .unwrap();
    }

    #[test]
    fn lifecycle_tracker_runs_launch_suspend_resume_and_terminate_end_to_end() {
        let identity = app_identity(10, 20, 30);
        let mut tracker = AppLifecycleTracker::new();
        let transactions = [
            (
                1,
                AppLifecycleAction::Launch,
                AppLifecycleState::Launching,
                AppLifecycleState::Inactive,
                None,
            ),
            (
                2,
                AppLifecycleAction::Activate,
                AppLifecycleState::Activating,
                AppLifecycleState::Active,
                Some(identity),
            ),
            (
                3,
                AppLifecycleAction::Suspend,
                AppLifecycleState::Suspending,
                AppLifecycleState::Suspended,
                Some(identity),
            ),
            (
                4,
                AppLifecycleAction::Resume,
                AppLifecycleState::Resuming,
                AppLifecycleState::Active,
                Some(identity),
            ),
            (
                5,
                AppLifecycleAction::Terminate,
                AppLifecycleState::Terminating,
                AppLifecycleState::NotRunning,
                Some(identity),
            ),
        ];
        let mut state_sequence = 1;
        for (index, (transaction, action, intermediate, completed, request_identity)) in
            transactions.into_iter().enumerate()
        {
            let direction_sequence = index as u64 + 1;
            for message in [
                lifecycle_request(direction_sequence, transaction, action, request_identity),
                lifecycle_state(
                    state_sequence,
                    transaction,
                    intermediate,
                    AppLifecycleReason::Requested,
                    Some(identity),
                ),
                lifecycle_command(direction_sequence, transaction, action, identity),
                lifecycle_ack(
                    direction_sequence,
                    transaction,
                    action,
                    AppLifecycleStatus::Applied,
                    identity,
                ),
                lifecycle_state(
                    state_sequence + 1,
                    transaction,
                    completed,
                    AppLifecycleReason::Completed,
                    Some(identity),
                ),
            ] {
                let wire = message.encode();
                tracker
                    .accept(AppLifecycleMessage::decode(&wire).unwrap())
                    .unwrap();
            }
            state_sequence += 2;
        }
        assert_eq!(tracker.transaction().last_transaction_id(), Some(5));
        assert_eq!(tracker.transaction().running_app(), None);
        assert_eq!(tracker.transaction().running_identity(), None);
        assert_eq!(
            tracker
                .sequence()
                .last(AppLifecycleDirection::InitToLauncher),
            Some(10)
        );
    }

    #[test]
    fn lifecycle_tracker_closes_crash_and_requires_fresh_restart_identity() {
        let old = app_identity(1, 2, 3);
        let fresh = app_identity(2, 2, 4);
        let mut tracker = AppLifecycleTransactionTracker::new();
        for message in [
            lifecycle_request(1, 1, AppLifecycleAction::Launch, None),
            lifecycle_state(
                1,
                1,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(old),
            ),
            lifecycle_command(1, 1, AppLifecycleAction::Launch, old),
            lifecycle_ack(
                1,
                1,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Applied,
                old,
            ),
            lifecycle_state(
                2,
                1,
                AppLifecycleState::Inactive,
                AppLifecycleReason::Completed,
                Some(old),
            ),
            lifecycle_state(
                3,
                2,
                AppLifecycleState::Crashed,
                AppLifecycleReason::ProcessExited,
                Some(old),
            ),
        ] {
            tracker.accept(message).unwrap();
        }
        assert_eq!(tracker.last_transaction_id(), Some(2));
        assert_eq!(tracker.state(), AppLifecycleState::Crashed);
        assert_eq!(tracker.running_identity(), Some(old));
        tracker
            .accept(lifecycle_request(2, 3, AppLifecycleAction::Launch, None))
            .unwrap();
        tracker
            .accept(lifecycle_state(
                4,
                3,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(fresh),
            ))
            .unwrap();
        let before = tracker;
        assert_eq!(
            tracker.accept(lifecycle_command(2, 3, AppLifecycleAction::Launch, old,)),
            Err(AppLifecycleTransactionError::StaleIdentity)
        );
        assert_eq!(tracker, before);
        tracker
            .accept(lifecycle_command(2, 3, AppLifecycleAction::Launch, fresh))
            .unwrap();
    }

    #[test]
    fn lifecycle_tracker_accepts_canonical_failure_and_rejects_ack_reason_mismatch() {
        let identity = app_identity(1, 2, 3);
        let mut spawn_failure = AppLifecycleTransactionTracker::new();
        spawn_failure
            .accept(lifecycle_request(1, 1, AppLifecycleAction::Launch, None))
            .unwrap();
        spawn_failure
            .accept(lifecycle_state(
                1,
                1,
                AppLifecycleState::Failed,
                AppLifecycleReason::SpawnFailed,
                None,
            ))
            .unwrap();
        assert_eq!(spawn_failure.last_transaction_id(), Some(1));

        let mut rejected = AppLifecycleTransactionTracker::new();
        for message in [
            lifecycle_request(1, 1, AppLifecycleAction::Launch, None),
            lifecycle_state(
                1,
                1,
                AppLifecycleState::Launching,
                AppLifecycleReason::Requested,
                Some(identity),
            ),
            lifecycle_command(1, 1, AppLifecycleAction::Launch, identity),
            lifecycle_ack(
                1,
                1,
                AppLifecycleAction::Launch,
                AppLifecycleStatus::Rejected,
                identity,
            ),
        ] {
            rejected.accept(message).unwrap();
        }
        let before = rejected;
        assert_eq!(
            rejected.accept(lifecycle_state(
                2,
                1,
                AppLifecycleState::Failed,
                AppLifecycleReason::CommandFailed,
                Some(identity),
            )),
            Err(AppLifecycleTransactionError::AckStatusMismatch)
        );
        assert_eq!(rejected, before);
        rejected
            .accept(lifecycle_state(
                2,
                1,
                AppLifecycleState::Failed,
                AppLifecycleReason::CommandRejected,
                Some(identity),
            ))
            .unwrap();
    }

    #[test]
    fn lifecycle_state_machine_completes_the_happy_mobile_lifecycle() {
        let identity = app_identity(1, 2, 3);
        let mut machine = AppLifecycleStateMachine::new(ShellAppId::Settings);
        assert_eq!(machine.app(), ShellAppId::Settings);
        for (transaction, action, intermediate, completed) in [
            (
                1,
                AppLifecycleAction::Launch,
                AppLifecycleState::Launching,
                AppLifecycleState::Inactive,
            ),
            (
                2,
                AppLifecycleAction::Activate,
                AppLifecycleState::Activating,
                AppLifecycleState::Active,
            ),
            (
                3,
                AppLifecycleAction::Suspend,
                AppLifecycleState::Suspending,
                AppLifecycleState::Suspended,
            ),
            (
                4,
                AppLifecycleAction::Resume,
                AppLifecycleState::Resuming,
                AppLifecycleState::Active,
            ),
            (
                5,
                AppLifecycleAction::Terminate,
                AppLifecycleState::Terminating,
                AppLifecycleState::NotRunning,
            ),
        ] {
            let begin_identity = if action == AppLifecycleAction::Launch {
                None
            } else {
                Some(identity)
            };
            assert_eq!(
                machine.begin(transaction, action, begin_identity),
                Ok(intermediate)
            );
            if action == AppLifecycleAction::Launch {
                machine.bind_identity(transaction, identity).unwrap();
            }
            assert_eq!(machine.complete(transaction, identity), Ok(completed));
        }
        assert_eq!(machine.state(), AppLifecycleState::NotRunning);
        assert_eq!(machine.identity(), None);
        assert_eq!(machine.retired_identity(), Some(identity));
        assert_eq!(machine.active_transaction_id(), None);
        assert_eq!(machine.last_transaction_id(), Some(5));
    }

    #[test]
    fn lifecycle_state_machine_crash_restart_rejects_old_instance_and_pid_generation() {
        let old = app_identity(1, 7, 1);
        let stale_generation = app_identity(2, 7, 1);
        let fresh = app_identity(2, 7, 2);
        let mut machine = AppLifecycleStateMachine::new(ShellAppId::Phone);
        machine.begin(1, AppLifecycleAction::Launch, None).unwrap();
        machine.bind_identity(1, old).unwrap();
        machine.complete(1, old).unwrap();
        assert_eq!(machine.crash(2, old), Ok(AppLifecycleState::Crashed));
        machine.begin(3, AppLifecycleAction::Launch, None).unwrap();
        let launching = machine;
        assert_eq!(
            machine.bind_identity(3, old),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(machine, launching);
        assert_eq!(
            machine.bind_identity(3, stale_generation),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(machine, launching);
        machine.bind_identity(3, fresh).unwrap();
        machine.complete(3, fresh).unwrap();
        assert_eq!(machine.state(), AppLifecycleState::Inactive);
        assert_eq!(machine.identity(), Some(fresh));
        assert_eq!(machine.retired_identity(), Some(old));
    }

    #[test]
    fn lifecycle_state_machine_rejects_illegal_transition_transaction_and_identity() {
        let identity = app_identity(1, 2, 3);
        let stale = app_identity(1, 2, 4);
        let mut machine = AppLifecycleStateMachine::new(ShellAppId::Messages);
        let initial = machine;
        assert_eq!(
            machine.begin(1, AppLifecycleAction::Activate, Some(identity)),
            Err(AppLifecycleMachineError::InvalidTransition)
        );
        assert_eq!(machine, initial);
        assert_eq!(
            machine.begin(1, AppLifecycleAction::Launch, Some(identity)),
            Err(AppLifecycleMachineError::UnexpectedIdentity)
        );
        assert_eq!(machine, initial);
        machine.begin(1, AppLifecycleAction::Launch, None).unwrap();
        let launching = machine;
        assert_eq!(
            machine.begin(2, AppLifecycleAction::Launch, Some(stale)),
            Err(AppLifecycleMachineError::TransactionBusy)
        );
        assert_eq!(machine, launching);
        machine.bind_identity(1, identity).unwrap();
        let bound = machine;
        assert_eq!(
            machine.complete(2, identity),
            Err(AppLifecycleMachineError::TransactionMismatch)
        );
        assert_eq!(machine, bound);
        assert_eq!(
            machine.complete(1, stale),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(machine, bound);
        machine.complete(1, identity).unwrap();
        let inactive = machine;
        assert_eq!(
            machine.begin(1, AppLifecycleAction::Activate, Some(identity)),
            Err(AppLifecycleMachineError::TransactionReplay)
        );
        assert_eq!(machine, inactive);
        assert_eq!(
            machine.begin(2, AppLifecycleAction::Suspend, Some(identity)),
            Err(AppLifecycleMachineError::InvalidTransition)
        );
        assert_eq!(machine, inactive);
        assert_eq!(
            machine.begin(2, AppLifecycleAction::Activate, Some(stale)),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(machine, inactive);
    }

    #[test]
    fn lifecycle_state_machine_failure_and_mid_transition_crash_are_terminal_and_checked() {
        let identity = app_identity(1, 2, 3);
        let mut failed = AppLifecycleStateMachine::new(ShellAppId::Phone);
        failed.begin(1, AppLifecycleAction::Launch, None).unwrap();
        let unbound = failed;
        assert_eq!(
            failed.fail(1, identity),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(failed, unbound);
        failed.bind_identity(1, identity).unwrap();
        assert_eq!(failed.fail(1, identity), Ok(AppLifecycleState::Failed));
        assert_eq!(failed.last_transaction_id(), Some(1));

        let fresh = app_identity(2, 2, 4);
        failed.begin(2, AppLifecycleAction::Launch, None).unwrap();
        failed.bind_identity(2, fresh).unwrap();
        let transitioning = failed;
        assert_eq!(
            failed.crash(3, fresh),
            Err(AppLifecycleMachineError::TransactionMismatch)
        );
        assert_eq!(failed, transitioning);
        assert_eq!(failed.crash(2, fresh), Ok(AppLifecycleState::Crashed));
        let crashed = failed;
        assert_eq!(
            failed.crash(3, fresh),
            Err(AppLifecycleMachineError::InvalidTransition)
        );
        assert_eq!(failed, crashed);
    }

    #[test]
    fn lifecycle_state_machine_handles_pre_spawn_failure_and_pre_publish_exit() {
        let identity = app_identity(1, 2, 3);
        let mut machine = AppLifecycleStateMachine::new(ShellAppId::Phone);
        machine.begin(1, AppLifecycleAction::Launch, None).unwrap();
        let unbound = machine;
        assert_eq!(
            machine.crash(1, identity),
            Err(AppLifecycleMachineError::StaleIdentity)
        );
        assert_eq!(machine, unbound);
        assert_eq!(machine.fail_before_spawn(1), Ok(AppLifecycleState::Failed));
        assert_eq!(machine.identity(), None);

        machine.begin(2, AppLifecycleAction::Launch, None).unwrap();
        machine.bind_identity(2, identity).unwrap();
        assert_eq!(machine.crash(2, identity), Ok(AppLifecycleState::Crashed));
        assert_eq!(machine.identity(), Some(identity));
        assert_eq!(machine.last_transaction_id(), Some(2));
    }

    #[test]
    fn appearance_flags_are_closed_and_canonical() {
        let base_states = [
            (0_u64, UiAppearance::new(false, false)),
            (1, UiAppearance::new(true, false)),
            (2, UiAppearance::new(false, true)),
            (3, UiAppearance::new(true, true)),
        ];
        let dimming_levels = [
            UiSoftwareDimming::Off,
            UiSoftwareDimming::Light,
            UiSoftwareDimming::Medium,
            UiSoftwareDimming::Strong,
            UiSoftwareDimming::Maximum,
        ];
        for (base_flags, base) in base_states {
            for software_dimming in dimming_levels {
                let appearance = base.with_software_dimming(software_dimming);
                let flags = base_flags | (u64::from(software_dimming.raw()) << 2);
                assert_eq!(appearance.flags(), flags);
                assert_eq!(UiAppearance::from_flags(flags), Ok(appearance));
                assert_eq!(appearance.software_dimming(), software_dimming);
                assert_eq!(appearance.dark_theme(), base.dark_theme());
                assert_eq!(appearance.alternate_accent(), base.alternate_accent());
            }
        }
        assert_eq!(UiAppearance::default(), UiAppearance::new(true, false));
        assert!(UiAppearance::default().dark_theme());
        assert!(!UiAppearance::default().alternate_accent());
        assert_eq!(
            UiAppearance::default().software_dimming(),
            UiSoftwareDimming::Off
        );
        for (raw, level) in dimming_levels.into_iter().enumerate() {
            assert_eq!(
                UiSoftwareDimming::from_raw(raw as u64),
                Some(level),
                "raw {raw}"
            );
        }
        for raw in [5_u64, 6, 7, u64::MAX] {
            assert_eq!(UiSoftwareDimming::from_raw(raw), None);
        }
        for raw in 5_u64..=7 {
            assert_eq!(
                UiAppearance::from_flags(raw << 2),
                Err(UiAppearanceError::InvalidSoftwareDimming)
            );
        }
        for flags in [1_u64 << 5, 1 << 63, u64::MAX] {
            assert_eq!(
                UiAppearance::from_flags(flags),
                Err(UiAppearanceError::UnknownFlags)
            );
        }
    }

    #[test]
    fn androidbox_audit_session_binds_boot_and_contiguous_taps_transactionally() {
        let boot = UiAndroidBoxExecutionReport::new(
            1,
            UiAndroidBoxExecutionKind::Boot,
            0x1122_3344,
            0x5566_7788,
            -7,
            11,
            0,
        )
        .unwrap();
        let tap_one = UiAndroidBoxExecutionReport::new(
            2,
            UiAndroidBoxExecutionKind::Tap,
            boot.dex_crc32(),
            boot.dex_adler32(),
            42,
            13,
            1,
        )
        .unwrap();
        let tap_two = UiAndroidBoxExecutionReport::new(
            3,
            UiAndroidBoxExecutionKind::Tap,
            boot.dex_crc32(),
            boot.dex_adler32(),
            i32::MIN,
            UI_ANDROIDBOX_MAX_INSTRUCTION_COUNT,
            2,
        )
        .unwrap();

        let mut session = UiAndroidBoxAuditSession::new();
        assert_eq!(session, UiAndroidBoxAuditSession::default());
        assert_eq!(session.last_request_id(), 0);
        assert_eq!(session.fixture_crc32(), None);
        assert_eq!(session.fixture_adler32(), None);
        assert_eq!(session.last_tap_count(), None);
        assert_eq!(session.last_report(), None);

        session.accept(UiClientId::Launcher, boot).unwrap();
        assert_eq!(session.last_request_id(), 1);
        assert_eq!(session.fixture_crc32(), Some(0x1122_3344));
        assert_eq!(session.fixture_adler32(), Some(0x5566_7788));
        assert_eq!(session.last_tap_count(), Some(0));
        assert_eq!(session.last_report(), Some(boot));
        session.accept(UiClientId::Launcher, tap_one).unwrap();
        session.accept(UiClientId::Launcher, tap_two).unwrap();
        assert_eq!(session.last_request_id(), 3);
        assert_eq!(session.last_tap_count(), Some(2));
        assert_eq!(session.last_report(), Some(tap_two));
        assert_eq!(session.last_report().unwrap().result(), i32::MIN);

        let accepted = session;
        for (report, expected) in [
            (
                UiAndroidBoxExecutionReport::new(
                    3,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    2,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::RequestReplay,
            ),
            (
                UiAndroidBoxExecutionReport::new(
                    5,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    3,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::RequestGap,
            ),
            (
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Boot,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    0,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::SubsequentReportMustBeTap,
            ),
            (
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32() ^ 1,
                    boot.dex_adler32(),
                    0,
                    1,
                    3,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::FixtureChecksumMismatch,
            ),
            (
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    2,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::TapReplay,
            ),
            (
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    4,
                )
                .unwrap(),
                UiAndroidBoxAuditSessionError::TapGap,
            ),
        ] {
            assert_eq!(session.accept(UiClientId::Launcher, report), Err(expected));
            assert_eq!(session, accepted);
        }
        assert_eq!(
            session.accept(
                UiClientId::App,
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    3,
                )
                .unwrap(),
            ),
            Err(UiAndroidBoxAuditSessionError::LauncherOnly)
        );
        assert_eq!(session, accepted);

        let mut first = UiAndroidBoxAuditSession::new();
        let before = first;
        assert_eq!(
            first.accept(
                UiClientId::Launcher,
                UiAndroidBoxExecutionReport::new(1, UiAndroidBoxExecutionKind::Tap, 1, 2, 3, 4, 1,)
                    .unwrap(),
            ),
            Err(UiAndroidBoxAuditSessionError::FirstReportMustBeBoot)
        );
        assert_eq!(first, before);

        let mut request_exhausted = accepted;
        request_exhausted.last_request_id = u64::MAX;
        let before = request_exhausted;
        assert_eq!(
            request_exhausted.accept(UiClientId::Launcher, tap_two),
            Err(UiAndroidBoxAuditSessionError::RequestIdExhausted)
        );
        assert_eq!(request_exhausted, before);

        let mut tap_exhausted = accepted;
        tap_exhausted.last_request_id = 3;
        tap_exhausted.last_tap_count = u16::MAX;
        let before = tap_exhausted;
        assert_eq!(
            tap_exhausted.accept(
                UiClientId::Launcher,
                UiAndroidBoxExecutionReport::new(
                    4,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    0,
                    1,
                    u16::MAX,
                )
                .unwrap(),
            ),
            Err(UiAndroidBoxAuditSessionError::TapCountExhausted)
        );
        assert_eq!(tap_exhausted, before);
    }

    #[test]
    fn androidbox_activity_report_has_one_exact_bounded_v8_wire_format() {
        const VIEW_TEXT: &[u8] = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT;
        let control = UiClientControl::report_androidbox_activity_execution(
            0x1122_3344,
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            0x5566_7788,
            0x99aa_bbcc,
            0xddee_ff00,
            UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX,
            UI_ANDROIDBOX_ACTIVITY_MAX_CODE_OFFSET,
            UI_ANDROIDBOX_ACTIVITY_MAX_INSTRUCTION_COUNT,
            VIEW_TEXT,
        )
        .unwrap();
        let UiClientControlPayload::ReportAndroidBoxActivityExecution { report } =
            control.payload()
        else {
            panic!("wrong control payload");
        };
        assert_eq!(report.request_id(), 0x1122_3344);
        assert_eq!(
            report.lifecycle(),
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete
        );
        assert_eq!(report.manifest_crc32(), 0x5566_7788);
        assert_eq!(report.dex_crc32(), 0x99aa_bbcc);
        assert_eq!(report.dex_adler32(), 0xddee_ff00);
        assert_eq!(
            report.activity_descriptor_crc32(),
            UI_ANDROIDBOX_ACTIVITY_DESCRIPTOR_CRC32
        );
        assert_eq!(
            report.on_create_method_index(),
            UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX
        );
        assert_eq!(
            report.on_create_code_offset(),
            UI_ANDROIDBOX_ACTIVITY_MAX_CODE_OFFSET
        );
        assert_eq!(
            report.instruction_count(),
            UI_ANDROIDBOX_ACTIVITY_MAX_INSTRUCTION_COUNT
        );
        assert_eq!(
            report.view_text_label(),
            androidbox_activity_view_text_label(VIEW_TEXT).unwrap()
        );
        assert_eq!(
            report.view_text_label(),
            UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT_LABEL
        );
        assert_eq!(usize::from(report.view_text_length()), VIEW_TEXT.len());

        let wire = control.encode();
        assert_eq!(&wire[0..4], b"BUC1");
        assert_eq!(&wire[4..8], &[8, 0, 7, 0]);
        assert_eq!(&wire[8..16], &0x5566_7788_1122_3344_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &0xddee_ff00_99aa_bbcc_u64.to_le_bytes());
        assert_eq!(
            read_u64(&wire, CONTROL_OFFSET_TRANSITION_ID),
            pack_androidbox_activity_execution(report)
        );
        assert!(wire[32..].iter().all(|byte| *byte == 0));
        assert_eq!(UiClientControl::decode(&wire), Ok(control));

        let mut version_six = wire;
        version_six[CONTROL_OFFSET_VERSION..CONTROL_OFFSET_VERSION + 2]
            .copy_from_slice(&6_u16.to_le_bytes());
        assert_eq!(
            UiClientControl::decode(&version_six),
            Err(UiClientControlError::InvalidVersion)
        );
        for offset in CONTROL_OFFSET_BODY_RESERVED..UI_CLIENT_CONTROL_WIRE_SIZE {
            let mut noncanonical = wire;
            noncanonical[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&noncanonical),
                Err(UiClientControlError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn androidbox_activity_report_rejects_invalid_ranges_and_bit_fields() {
        const TEXT: &[u8] = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT;
        let make = |request_id, method_index, code_offset, instructions, text: &[u8]| {
            UiClientControl::report_androidbox_activity_execution(
                request_id,
                UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
                1,
                2,
                3,
                method_index,
                code_offset,
                instructions,
                text,
            )
        };
        assert_eq!(
            make(0, 1, 4, 1, TEXT),
            Err(UiClientControlError::ZeroAndroidBoxActivityRequestId)
        );
        assert_eq!(
            make(1, UI_ANDROIDBOX_ACTIVITY_MAX_METHOD_INDEX + 1, 4, 1, TEXT,),
            Err(UiClientControlError::AndroidBoxActivityMethodIndexOutOfRange)
        );
        assert_eq!(
            make(1, 1, 0, 1, TEXT),
            Err(UiClientControlError::AndroidBoxActivityCodeOffsetOutOfRange)
        );
        assert_eq!(
            make(1, 1, 6, 1, TEXT),
            Err(UiClientControlError::UnalignedAndroidBoxActivityCodeOffset)
        );
        assert_eq!(
            make(1, 1, UI_ANDROIDBOX_ACTIVITY_MAX_CODE_OFFSET + 4, 1, TEXT,),
            Err(UiClientControlError::AndroidBoxActivityCodeOffsetOutOfRange)
        );
        assert_eq!(
            make(1, 1, 4, 0, TEXT),
            Err(UiClientControlError::ZeroAndroidBoxActivityInstructionCount)
        );
        assert_eq!(
            make(
                1,
                1,
                4,
                UI_ANDROIDBOX_ACTIVITY_MAX_INSTRUCTION_COUNT + 1,
                TEXT,
            ),
            Err(UiClientControlError::AndroidBoxActivityInstructionCountOutOfRange)
        );
        assert_eq!(
            make(1, 1, 4, 1, b""),
            Err(UiClientControlError::EmptyAndroidBoxActivityViewText)
        );
        let too_long = [b'x'; UI_ANDROIDBOX_ACTIVITY_MAX_VIEW_TEXT_LENGTH as usize + 1];
        assert_eq!(
            make(1, 1, 4, 1, &too_long),
            Err(UiClientControlError::AndroidBoxActivityViewTextTooLong)
        );
        assert_eq!(
            make(1, 1, 4, 1, b"changed"),
            Err(UiClientControlError::AndroidBoxActivityViewTextIdentityMismatch)
        );
        assert_eq!(
            UiAndroidBoxActivityLifecycleKind::from_raw(1),
            Some(UiAndroidBoxActivityLifecycleKind::OnCreateComplete)
        );
        for raw in [0, 2, 3, u64::MAX] {
            assert_eq!(UiAndroidBoxActivityLifecycleKind::from_raw(raw), None);
        }

        let wire = make(1, 1, 4, 1, TEXT).unwrap().encode();
        let mut reserved_execution_bit = wire;
        reserved_execution_bit[CONTROL_OFFSET_TRANSITION_ID + 3] |= 0x80;
        assert_eq!(
            UiClientControl::decode(&reserved_execution_bit),
            Err(UiClientControlError::NonCanonicalAndroidBoxActivityExecution)
        );
        for raw in [0_u64, 2, 3] {
            let mut invalid = wire;
            let mut execution = read_u64(&invalid, CONTROL_OFFSET_TRANSITION_ID);
            execution &= !ANDROIDBOX_ACTIVITY_LIFECYCLE_MASK;
            execution |= raw << ANDROIDBOX_ACTIVITY_LIFECYCLE_SHIFT;
            write_control_u64(&mut invalid, CONTROL_OFFSET_TRANSITION_ID, execution);
            assert_eq!(
                UiClientControl::decode(&invalid),
                Err(UiClientControlError::InvalidAndroidBoxActivityLifecycle)
            );
        }
        let mut zero_code = wire;
        let mut execution = read_u64(&zero_code, CONTROL_OFFSET_TRANSITION_ID);
        execution &= !ANDROIDBOX_ACTIVITY_CODE_UNITS_MASK;
        write_control_u64(&mut zero_code, CONTROL_OFFSET_TRANSITION_ID, execution);
        assert_eq!(
            UiClientControl::decode(&zero_code),
            Err(UiClientControlError::AndroidBoxActivityCodeOffsetOutOfRange)
        );
        let mut zero_instructions = wire;
        let mut execution = read_u64(&zero_instructions, CONTROL_OFFSET_TRANSITION_ID);
        execution &= !ANDROIDBOX_ACTIVITY_INSTRUCTIONS_MASK;
        write_control_u64(
            &mut zero_instructions,
            CONTROL_OFFSET_TRANSITION_ID,
            execution,
        );
        assert_eq!(
            UiClientControl::decode(&zero_instructions),
            Err(UiClientControlError::ZeroAndroidBoxActivityInstructionCount)
        );
        let mut too_many_instructions = wire;
        let mut execution = read_u64(&too_many_instructions, CONTROL_OFFSET_TRANSITION_ID);
        execution &= !ANDROIDBOX_ACTIVITY_INSTRUCTIONS_MASK;
        execution |= 65 << ANDROIDBOX_ACTIVITY_INSTRUCTIONS_SHIFT;
        write_control_u64(
            &mut too_many_instructions,
            CONTROL_OFFSET_TRANSITION_ID,
            execution,
        );
        assert_eq!(
            UiClientControl::decode(&too_many_instructions),
            Err(UiClientControlError::AndroidBoxActivityInstructionCountOutOfRange)
        );
        for length in [0_u64, 97, 127] {
            let mut invalid = wire;
            let mut execution = read_u64(&invalid, CONTROL_OFFSET_TRANSITION_ID);
            execution &= !ANDROIDBOX_ACTIVITY_VIEW_LENGTH_MASK;
            execution |= length << ANDROIDBOX_ACTIVITY_VIEW_LENGTH_SHIFT;
            write_control_u64(&mut invalid, CONTROL_OFFSET_TRANSITION_ID, execution);
            assert_eq!(
                UiClientControl::decode(&invalid),
                Err(if length == 0 {
                    UiClientControlError::EmptyAndroidBoxActivityViewText
                } else {
                    UiClientControlError::AndroidBoxActivityViewTextTooLong
                })
            );
        }
    }

    #[test]
    fn androidbox_activity_audit_requires_boot_then_one_stable_on_create() {
        const VIEW_TEXT: &[u8] = UI_ANDROIDBOX_ACTIVITY_VIEW_TEXT;
        let boot = UiAndroidBoxExecutionReport::new(
            1,
            UiAndroidBoxExecutionKind::Boot,
            0x1122_3344,
            0x5566_7788,
            7,
            2,
            0,
        )
        .unwrap();
        let activity = |request_id, manifest, method, code, text: &[u8]| {
            UiAndroidBoxActivityExecutionReport::new(
                request_id,
                UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
                manifest,
                boot.dex_crc32(),
                boot.dex_adler32(),
                method,
                code,
                9,
                text,
            )
            .unwrap()
        };
        let report = activity(1, 0x99aa_bbcc, 12, 0x240, VIEW_TEXT);

        let empty_dex = UiAndroidBoxAuditSession::new();
        let mut session = UiAndroidBoxActivityAuditSession::new();
        assert_eq!(session, UiAndroidBoxActivityAuditSession::default());
        assert_eq!(session.last_request_id(), 0);
        assert_eq!(session.dex_boot_request_id(), None);
        assert_eq!(session.last_report(), None);
        let before = session;
        assert_eq!(
            session.accept(UiClientId::Launcher, &empty_dex, report),
            Err(UiAndroidBoxActivityAuditSessionError::MissingDexBoot)
        );
        assert_eq!(session, before);

        let mut dex = UiAndroidBoxAuditSession::new();
        dex.accept(UiClientId::Launcher, boot).unwrap();
        assert_eq!(
            session.accept(UiClientId::App, &dex, report),
            Err(UiAndroidBoxActivityAuditSessionError::LauncherOnly)
        );
        assert_eq!(session, before);
        assert_eq!(
            session.accept(
                UiClientId::Launcher,
                &dex,
                activity(2, 0x99aa_bbcc, 12, 0x240, VIEW_TEXT),
            ),
            Err(UiAndroidBoxActivityAuditSessionError::RequestGap)
        );
        assert_eq!(session, before);
        let mismatched_dex = UiAndroidBoxActivityExecutionReport::new(
            1,
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            0x99aa_bbcc,
            boot.dex_crc32() ^ 1,
            boot.dex_adler32(),
            12,
            0x240,
            9,
            VIEW_TEXT,
        )
        .unwrap();
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, mismatched_dex),
            Err(UiAndroidBoxActivityAuditSessionError::DexIdentityMismatch)
        );
        assert_eq!(session, before);

        session.accept(UiClientId::Launcher, &dex, report).unwrap();
        assert_eq!(session.last_request_id(), 1);
        assert_eq!(session.dex_boot_request_id(), Some(1));
        assert_eq!(session.last_report(), Some(report));
        let accepted = session;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, report),
            Err(UiAndroidBoxActivityAuditSessionError::RequestReplay)
        );
        assert_eq!(session, accepted);
        assert_eq!(
            session.accept(
                UiClientId::Launcher,
                &dex,
                activity(2, 0x99aa_bbcc, 12, 0x240, VIEW_TEXT),
            ),
            Err(UiAndroidBoxActivityAuditSessionError::LifecycleAlreadyReported)
        );
        assert_eq!(session, accepted);
        for drift in [
            activity(2, 0x99aa_bbcd, 12, 0x240, VIEW_TEXT),
            activity(2, 0x99aa_bbcc, 13, 0x240, VIEW_TEXT),
            activity(2, 0x99aa_bbcc, 12, 0x244, VIEW_TEXT),
        ] {
            assert_eq!(
                session.accept(UiClientId::Launcher, &dex, drift),
                Err(UiAndroidBoxActivityAuditSessionError::IdentityDrift)
            );
            assert_eq!(session, accepted);
        }
        let mut view_drift = activity(2, 0x99aa_bbcc, 12, 0x240, VIEW_TEXT);
        view_drift.view_text_label ^= 1;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, view_drift),
            Err(UiAndroidBoxActivityAuditSessionError::IdentityDrift)
        );
        assert_eq!(session, accepted);

        dex.accept(
            UiClientId::Launcher,
            UiAndroidBoxExecutionReport::new(
                2,
                UiAndroidBoxExecutionKind::Tap,
                boot.dex_crc32(),
                boot.dex_adler32(),
                8,
                2,
                1,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            session.accept(
                UiClientId::Launcher,
                &dex,
                activity(2, 0x99aa_bbcc, 12, 0x240, VIEW_TEXT),
            ),
            Err(UiAndroidBoxActivityAuditSessionError::DexPredecessorMustBeBoot)
        );
        assert_eq!(session, accepted);

        let mut exhausted = UiAndroidBoxActivityAuditSession {
            last_request_id: u32::MAX,
            dex_boot_request_id: 0,
            last_report: None,
        };
        let mut boot_only = UiAndroidBoxAuditSession::new();
        boot_only.accept(UiClientId::Launcher, boot).unwrap();
        let before = exhausted;
        assert_eq!(
            exhausted.accept(UiClientId::Launcher, &boot_only, report),
            Err(UiAndroidBoxActivityAuditSessionError::RequestIdExhausted)
        );
        assert_eq!(exhausted, before);
    }

    #[test]
    fn androidbox_resource_report_has_one_exact_bounded_v8_wire_format() {
        let control = UiClientControl::report_androidbox_resource_execution(
            0x1122_3344,
            UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
            UI_ANDROIDBOX_RESOURCE_STRING_ID,
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
        )
        .unwrap();
        let UiClientControlPayload::ReportAndroidBoxResourceExecution { report } =
            control.payload()
        else {
            panic!("wrong control payload");
        };
        assert_eq!(report.request_id(), 0x1122_3344);
        assert_eq!(
            report.resources_arsc_crc32(),
            UI_ANDROIDBOX_RESOURCE_ARSC_CRC32
        );
        assert_eq!(
            report.layout_xml_crc32(),
            UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32
        );
        assert_eq!(
            report.layout_resource_id(),
            UI_ANDROIDBOX_RESOURCE_LAYOUT_ID
        );
        assert_eq!(
            report.string_resource_id(),
            UI_ANDROIDBOX_RESOURCE_STRING_ID
        );
        assert_eq!(
            report.view_text_label(),
            androidbox_activity_view_text_label(UI_ANDROIDBOX_RESOURCE_VIEW_TEXT).unwrap()
        );
        assert_eq!(
            report.view_text_label(),
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LABEL
        );
        assert_eq!(
            report.view_text_length(),
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT_LENGTH
        );
        assert_eq!(report.success_flags(), UI_ANDROIDBOX_RESOURCE_SUCCESS_FLAGS);

        let wire = control.encode();
        assert_eq!(&wire[0..4], b"BUC1");
        assert_eq!(&wire[4..8], &[8, 0, 8, 0]);
        assert_eq!(&wire[8..16], &0x9f67_c7b4_1122_3344_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &0x7f02_0000_36ce_95c4_u64.to_le_bytes());
        assert_eq!(
            read_u64(&wire, CONTROL_OFFSET_TRANSITION_ID),
            pack_androidbox_resource_execution(report)
        );
        assert_eq!(read_u64(&wire, CONTROL_OFFSET_TRANSITION_ID) >> 60, 0);
        assert!(wire[32..].iter().all(|byte| *byte == 0));
        assert_eq!(UiClientControl::decode(&wire), Ok(control));

        let mut version_six = wire;
        version_six[CONTROL_OFFSET_VERSION..CONTROL_OFFSET_VERSION + 2]
            .copy_from_slice(&6_u16.to_le_bytes());
        assert_eq!(
            UiClientControl::decode(&version_six),
            Err(UiClientControlError::InvalidVersion)
        );
        for offset in CONTROL_OFFSET_BODY_RESERVED..UI_CLIENT_CONTROL_WIRE_SIZE {
            let mut noncanonical = wire;
            noncanonical[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&noncanonical),
                Err(UiClientControlError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn androidbox_resource_report_rejects_fixture_drift_and_noncanonical_bits() {
        let make = |request_id, arsc_crc, layout_crc, layout_id, string_id, text: &[u8]| {
            UiAndroidBoxResourceExecutionReport::new(
                request_id, arsc_crc, layout_crc, layout_id, string_id, text,
            )
        };
        assert_eq!(
            make(
                0,
                UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
                UI_ANDROIDBOX_RESOURCE_STRING_ID,
                UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
            ),
            Err(UiClientControlError::ZeroAndroidBoxResourceRequestId)
        );
        assert_eq!(
            make(
                1,
                UI_ANDROIDBOX_RESOURCE_ARSC_CRC32 ^ 1,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
                UI_ANDROIDBOX_RESOURCE_STRING_ID,
                UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
            ),
            Err(UiClientControlError::AndroidBoxResourceArscIdentityMismatch)
        );
        for (layout_crc, layout_id) in [
            (
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32 ^ 1,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
            ),
            (
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID ^ 1,
            ),
        ] {
            assert_eq!(
                make(
                    1,
                    UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
                    layout_crc,
                    layout_id,
                    UI_ANDROIDBOX_RESOURCE_STRING_ID,
                    UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
                ),
                Err(UiClientControlError::AndroidBoxResourceLayoutIdentityMismatch)
            );
        }
        assert_eq!(
            make(
                1,
                UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
                UI_ANDROIDBOX_RESOURCE_STRING_ID ^ 1,
                UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
            ),
            Err(UiClientControlError::AndroidBoxResourceTextIdentityMismatch)
        );
        for text in [b"".as_slice(), b"changed".as_slice()] {
            assert_eq!(
                make(
                    1,
                    UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
                    UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                    UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
                    UI_ANDROIDBOX_RESOURCE_STRING_ID,
                    text,
                ),
                Err(UiClientControlError::AndroidBoxResourceTextIdentityMismatch)
            );
        }

        let wire = UiClientControl::report_androidbox_resource_execution(
            1,
            UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
            UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
            UI_ANDROIDBOX_RESOURCE_STRING_ID,
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
        )
        .unwrap()
        .encode();
        let mut reserved = wire;
        reserved[CONTROL_OFFSET_TRANSITION_ID + 7] |= 0x80;
        assert_eq!(
            UiClientControl::decode(&reserved),
            Err(UiClientControlError::NonCanonicalAndroidBoxResourceExecution)
        );

        let mut missing_flag = wire;
        let execution = read_u64(&missing_flag, CONTROL_OFFSET_TRANSITION_ID)
            & !(1 << ANDROIDBOX_RESOURCE_SUCCESS_FLAGS_SHIFT);
        write_control_u64(&mut missing_flag, CONTROL_OFFSET_TRANSITION_ID, execution);
        assert_eq!(
            UiClientControl::decode(&missing_flag),
            Err(UiClientControlError::AndroidBoxResourceSuccessFlagsMismatch)
        );

        let mut wrong_length = wire;
        let execution = (read_u64(&wrong_length, CONTROL_OFFSET_TRANSITION_ID)
            & !ANDROIDBOX_RESOURCE_VIEW_LENGTH_MASK)
            | (1 << ANDROIDBOX_RESOURCE_VIEW_LENGTH_SHIFT);
        write_control_u64(&mut wrong_length, CONTROL_OFFSET_TRANSITION_ID, execution);
        assert_eq!(
            UiClientControl::decode(&wrong_length),
            Err(UiClientControlError::AndroidBoxResourceTextIdentityMismatch)
        );
    }

    #[test]
    fn androidbox_resource_audit_requires_one_boot_activity_resource_chain() {
        let boot = UiAndroidBoxExecutionReport::new(
            1,
            UiAndroidBoxExecutionKind::Boot,
            0x1122_3344,
            0x5566_7788,
            7,
            2,
            0,
        )
        .unwrap();
        let activity_report = UiAndroidBoxActivityExecutionReport::new(
            1,
            UiAndroidBoxActivityLifecycleKind::OnCreateComplete,
            0x99aa_bbcc,
            boot.dex_crc32(),
            boot.dex_adler32(),
            7,
            0x2c8,
            4,
            UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
        )
        .unwrap();
        let resource = |request_id| {
            UiAndroidBoxResourceExecutionReport::new(
                request_id,
                UI_ANDROIDBOX_RESOURCE_ARSC_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_XML_CRC32,
                UI_ANDROIDBOX_RESOURCE_LAYOUT_ID,
                UI_ANDROIDBOX_RESOURCE_STRING_ID,
                UI_ANDROIDBOX_RESOURCE_VIEW_TEXT,
            )
            .unwrap()
        };
        let report = resource(1);

        let empty_dex = UiAndroidBoxAuditSession::new();
        let empty_activity = UiAndroidBoxActivityAuditSession::new();
        let mut session = UiAndroidBoxResourceAuditSession::new();
        assert_eq!(session, UiAndroidBoxResourceAuditSession::default());
        assert_eq!(session.last_request_id(), 0);
        assert_eq!(session.dex_boot_request_id(), None);
        assert_eq!(session.activity_request_id(), None);
        assert_eq!(session.last_report(), None);
        let before = session;
        assert_eq!(
            session.accept(UiClientId::Launcher, &empty_dex, &empty_activity, report,),
            Err(UiAndroidBoxResourceAuditSessionError::MissingDexBoot)
        );
        assert_eq!(session, before);

        let mut dex = UiAndroidBoxAuditSession::new();
        dex.accept(UiClientId::Launcher, boot).unwrap();
        let mut activity = UiAndroidBoxActivityAuditSession::new();
        activity
            .accept(UiClientId::Launcher, &dex, activity_report)
            .unwrap();
        assert_eq!(
            session.accept(UiClientId::App, &dex, &activity, report),
            Err(UiAndroidBoxResourceAuditSessionError::LauncherOnly)
        );
        assert_eq!(session, before);
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &empty_activity, report),
            Err(UiAndroidBoxResourceAuditSessionError::MissingActivity)
        );
        assert_eq!(session, before);
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &activity, resource(2)),
            Err(UiAndroidBoxResourceAuditSessionError::RequestGap)
        );
        assert_eq!(session, before);

        let mut wrong_boot_activity = activity;
        wrong_boot_activity.dex_boot_request_id ^= 1;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &wrong_boot_activity, report,),
            Err(UiAndroidBoxResourceAuditSessionError::ActivityBootMismatch)
        );
        assert_eq!(session, before);

        let mut wrong_view_activity = activity;
        wrong_view_activity
            .last_report
            .as_mut()
            .unwrap()
            .view_text_label ^= 1;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &wrong_view_activity, report,),
            Err(UiAndroidBoxResourceAuditSessionError::ViewIdentityMismatch)
        );
        assert_eq!(session, before);

        session
            .accept(UiClientId::Launcher, &dex, &activity, report)
            .unwrap();
        assert_eq!(session.last_request_id(), 1);
        assert_eq!(session.dex_boot_request_id(), Some(1));
        assert_eq!(session.activity_request_id(), Some(1));
        assert_eq!(session.last_report(), Some(report));
        let accepted = session;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &activity, report),
            Err(UiAndroidBoxResourceAuditSessionError::RequestReplay)
        );
        assert_eq!(session, accepted);
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &activity, resource(2)),
            Err(UiAndroidBoxResourceAuditSessionError::ResourceAlreadyReported)
        );
        assert_eq!(session, accepted);
        let mut drift = resource(2);
        drift.layout_xml_crc32 ^= 1;
        assert_eq!(
            session.accept(UiClientId::Launcher, &dex, &activity, drift),
            Err(UiAndroidBoxResourceAuditSessionError::IdentityDrift)
        );
        assert_eq!(session, accepted);

        let mut late_dex = dex;
        late_dex
            .accept(
                UiClientId::Launcher,
                UiAndroidBoxExecutionReport::new(
                    2,
                    UiAndroidBoxExecutionKind::Tap,
                    boot.dex_crc32(),
                    boot.dex_adler32(),
                    8,
                    2,
                    1,
                )
                .unwrap(),
            )
            .unwrap();
        let mut late = UiAndroidBoxResourceAuditSession::new();
        let late_before = late;
        assert_eq!(
            late.accept(UiClientId::Launcher, &late_dex, &activity, report),
            Err(UiAndroidBoxResourceAuditSessionError::DexPredecessorMustBeBoot)
        );
        assert_eq!(late, late_before);

        let mut exhausted = UiAndroidBoxResourceAuditSession {
            last_request_id: u32::MAX,
            dex_boot_request_id: 0,
            activity_request_id: 0,
            last_report: None,
        };
        let exhausted_before = exhausted;
        assert_eq!(
            exhausted.accept(UiClientId::Launcher, &dex, &activity, report),
            Err(UiAndroidBoxResourceAuditSessionError::RequestIdExhausted)
        );
        assert_eq!(exhausted, exhausted_before);
    }

    #[test]
    fn appearance_session_serializes_two_client_request_streams_transactionally() {
        let mut session = UiAppearanceSession::new();
        assert_eq!(session, UiAppearanceSession::default());
        assert_eq!(session.appearance(), UiAppearance::new(true, false));
        assert_eq!(session.revision(), 1);
        assert_eq!(session.last_request_id(UiClientId::Launcher), 0);
        assert_eq!(session.last_request_id(UiClientId::App), 0);

        let light = session
            .apply(UiClientId::Launcher, UiAppearanceAction::ToggleTheme, 1)
            .unwrap();
        assert_eq!(light.appearance(), UiAppearance::new(false, false));
        assert_eq!(light.revision(), 2);
        let violet = session
            .apply(UiClientId::App, UiAppearanceAction::ToggleAccent, 1)
            .unwrap();
        assert_eq!(violet.appearance(), UiAppearance::new(false, true));
        assert_eq!(violet.revision(), 3);
        let medium = session
            .apply(
                UiClientId::App,
                UiAppearanceAction::SetSoftwareDimmingMedium,
                2,
            )
            .unwrap();
        assert_eq!(
            medium.appearance(),
            UiAppearance::new(false, true).with_software_dimming(UiSoftwareDimming::Medium)
        );
        assert_eq!(medium.revision(), 4);
        let dark = session
            .apply(UiClientId::Launcher, UiAppearanceAction::ToggleTheme, 2)
            .unwrap();
        assert_eq!(
            dark.appearance(),
            UiAppearance::new(true, true).with_software_dimming(UiSoftwareDimming::Medium)
        );
        assert_eq!(dark.revision(), 5);
        assert_eq!(session.last_request_id(UiClientId::Launcher), 2);
        assert_eq!(session.last_request_id(UiClientId::App), 2);

        for (client, action, request_id, expected) in [
            (
                UiClientId::Launcher,
                UiAppearanceAction::ToggleAccent,
                0,
                UiAppearanceSessionError::ZeroRequestId,
            ),
            (
                UiClientId::Launcher,
                UiAppearanceAction::ToggleAccent,
                2,
                UiAppearanceSessionError::RequestReplay,
            ),
            (
                UiClientId::Launcher,
                UiAppearanceAction::ToggleAccent,
                4,
                UiAppearanceSessionError::RequestGap,
            ),
        ] {
            let before = session;
            assert_eq!(session.apply(client, action, request_id), Err(expected));
            assert_eq!(session, before);
        }

        session.launcher_request_id = u64::MAX;
        let request_exhausted = session;
        assert_eq!(
            session.apply(UiClientId::Launcher, UiAppearanceAction::ToggleTheme, 1,),
            Err(UiAppearanceSessionError::RequestIdExhausted)
        );
        assert_eq!(session, request_exhausted);

        session.launcher_request_id = 2;
        session.revision = u64::MAX;
        let revision_exhausted = session;
        assert_eq!(
            session.apply(UiClientId::App, UiAppearanceAction::ToggleAccent, 3),
            Err(UiAppearanceSessionError::RevisionExhausted)
        );
        assert_eq!(session, revision_exhausted);
    }

    #[test]
    fn appearance_session_applies_every_software_dimming_level_without_field_loss() {
        let mut session = UiAppearanceSession::new();
        for (index, software_dimming) in [
            UiSoftwareDimming::Off,
            UiSoftwareDimming::Light,
            UiSoftwareDimming::Medium,
            UiSoftwareDimming::Strong,
            UiSoftwareDimming::Maximum,
        ]
        .into_iter()
        .enumerate()
        {
            let request_id = index as u64 + 1;
            let update = session
                .apply(
                    UiClientId::Launcher,
                    UiAppearanceAction::set_software_dimming(software_dimming),
                    request_id,
                )
                .unwrap();
            assert_eq!(
                update.appearance(),
                UiAppearance::new(true, false).with_software_dimming(software_dimming)
            );
            assert_eq!(update.revision(), request_id + 1);
            assert_eq!(session.last_request_id(UiClientId::Launcher), request_id);
            assert_eq!(session.last_request_id(UiClientId::App), 0);
        }
    }

    #[test]
    fn boot_notification_session_is_one_way_shared_and_transactional() {
        let mut session = UiBootNotificationSession::new();
        assert_eq!(session, UiBootNotificationSession::default());
        assert!(session.visible());
        assert_eq!(session.revision(), 1);
        assert_eq!(session.last_request_id(UiClientId::Launcher), 0);
        assert_eq!(session.last_request_id(UiClientId::App), 0);

        let dismissed = session.dismiss(UiClientId::Launcher, 1).unwrap();
        assert!(!dismissed.visible());
        assert_eq!(dismissed.revision(), 2);
        assert!(!session.visible());
        assert_eq!(session.revision(), 2);
        assert_eq!(session.last_request_id(UiClientId::Launcher), 1);
        assert_eq!(session.last_request_id(UiClientId::App), 0);

        for (client, request_id, expected) in [
            (
                UiClientId::Launcher,
                0,
                UiBootNotificationSessionError::ZeroRequestId,
            ),
            (
                UiClientId::Launcher,
                1,
                UiBootNotificationSessionError::RequestReplay,
            ),
            (
                UiClientId::Launcher,
                3,
                UiBootNotificationSessionError::RequestGap,
            ),
            (
                UiClientId::App,
                1,
                UiBootNotificationSessionError::AlreadyDismissed,
            ),
        ] {
            let before = session;
            assert_eq!(session.dismiss(client, request_id), Err(expected));
            assert_eq!(session, before);
        }

        let mut request_exhausted = UiBootNotificationSession::new();
        request_exhausted.launcher_request_id = u64::MAX;
        let before = request_exhausted;
        assert_eq!(
            request_exhausted.dismiss(UiClientId::Launcher, 1),
            Err(UiBootNotificationSessionError::RequestIdExhausted)
        );
        assert_eq!(request_exhausted, before);

        let mut revision_exhausted = UiBootNotificationSession::new();
        revision_exhausted.revision = u64::MAX;
        let before = revision_exhausted;
        assert_eq!(
            revision_exhausted.dismiss(UiClientId::App, 1),
            Err(UiBootNotificationSessionError::RevisionExhausted)
        );
        assert_eq!(revision_exhausted, before);
    }

    #[test]
    fn system_ui_session_is_single_recent_revisioned_and_transactional() {
        let mut session = UiSystemUiSession::new();
        assert_eq!(session, UiSystemUiSession::default());
        assert_eq!(session.mode(), UiSystemUiMode::Locked);
        assert_eq!(session.recent_app(), None);
        assert!(!session.nav_pressed());
        assert_eq!(session.nav_reveal_px(), 0);
        assert_eq!(session.revision(), 1);
        assert_eq!(session.last_request_id(), 0);
        assert_eq!(
            session.snapshot(),
            UiSystemUiUpdate {
                mode: UiSystemUiMode::Locked,
                recent: None,
                nav_pressed: false,
                nav_reveal_px: 0,
                revision: 1,
            }
        );

        session.begin_nav().unwrap();
        assert_eq!(session.mode(), UiSystemUiMode::Locked);
        assert!(session.nav_pressed());
        assert_eq!(session.revision(), 2);
        session.update_nav(UI_SYSTEM_UI_NAV_REVEAL_MAX_PX).unwrap();
        assert_eq!(session.nav_reveal_px(), UI_SYSTEM_UI_NAV_REVEAL_MAX_PX);
        let locked_drag = session;
        assert_eq!(
            session.finish_nav(UiSystemUiMode::Home),
            Err(UiSystemUiSessionError::LockedNavigationMustCancel)
        );
        assert_eq!(session, locked_drag);
        session.cancel_nav().unwrap();
        assert_eq!(session.mode(), UiSystemUiMode::Locked);
        assert!(!session.nav_pressed());
        assert_eq!(session.nav_reveal_px(), 0);
        assert_eq!(session.revision(), 4);

        let unlocked = session
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 4)
            .unwrap();
        assert_eq!(unlocked.mode(), UiSystemUiMode::Home);
        assert_eq!(unlocked.revision(), 5);
        session.focus_app(ShellAppId::Phone).unwrap();
        assert_eq!(session.mode(), UiSystemUiMode::Foreground);
        assert_eq!(session.recent_app(), Some(ShellAppId::Phone));
        assert_eq!(session.revision(), 6);

        session.begin_nav().unwrap();
        session.update_nav(232).unwrap();
        let overview = session.finish_nav(UiSystemUiMode::Overview).unwrap();
        assert_eq!(overview.mode(), UiSystemUiMode::Overview);
        assert_eq!(overview.recent_app(), Some(ShellAppId::Phone));
        assert!(!overview.nav_pressed());
        assert_eq!(overview.nav_reveal_px(), 0);
        assert_eq!(overview.revision(), 9);

        for (client, app, request_id, observed_revision, expected) in [
            (
                UiClientId::App,
                Some(ShellAppId::Phone),
                2,
                9,
                UiSystemUiSessionError::LauncherOnly,
            ),
            (
                UiClientId::Launcher,
                Some(ShellAppId::Phone),
                1,
                9,
                UiSystemUiSessionError::RequestReplay,
            ),
            (
                UiClientId::Launcher,
                Some(ShellAppId::Phone),
                3,
                9,
                UiSystemUiSessionError::RequestGap,
            ),
            (
                UiClientId::Launcher,
                Some(ShellAppId::Phone),
                2,
                0,
                UiSystemUiSessionError::ZeroObservedRevision,
            ),
            (
                UiClientId::Launcher,
                Some(ShellAppId::Phone),
                2,
                8,
                UiSystemUiSessionError::ObservedRevisionMismatch,
            ),
            (
                UiClientId::Launcher,
                Some(ShellAppId::Messages),
                2,
                9,
                UiSystemUiSessionError::RecentAppMismatch,
            ),
        ] {
            let before = session;
            assert_eq!(
                session.apply_client_action(
                    client,
                    UiSystemUiAction::ActivateRecent,
                    app,
                    request_id,
                    observed_revision,
                ),
                Err(expected)
            );
            assert_eq!(session, before);
        }

        let activated = session
            .apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::ActivateRecent,
                Some(ShellAppId::Phone),
                2,
                9,
            )
            .unwrap();
        assert_eq!(activated.mode(), UiSystemUiMode::Foreground);
        assert_eq!(activated.recent_app(), Some(ShellAppId::Phone));
        assert_eq!(activated.revision(), 10);
        assert_eq!(session.last_request_id(), 2);

        session.focus_home().unwrap();
        session.focus_app(ShellAppId::Settings).unwrap();
        assert_eq!(session.recent_app(), Some(ShellAppId::Settings));
        assert_eq!(session.revision(), 12);
        let foreground = session;
        assert_eq!(
            session.focus_app(ShellAppId::Messages),
            Err(UiSystemUiSessionError::FocusAppRequiresHome)
        );
        assert_eq!(session, foreground);
    }

    #[test]
    fn system_ui_session_validates_nav_actions_and_exhaustion_without_partial_commit() {
        let mut session = UiSystemUiSession::new();
        let initial = session;
        assert_eq!(
            session
                .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 0, 1,),
            Err(UiSystemUiSessionError::ZeroRequestId)
        );
        assert_eq!(session, initial);
        for reveal in [1, 7, 9, 479, 481, u16::MAX] {
            session.begin_nav().unwrap();
            let pressed = session;
            assert_eq!(
                session.begin_nav(),
                Err(UiSystemUiSessionError::NavigationAlreadyPressed)
            );
            assert_eq!(session, pressed);
            let expected = if reveal > UI_SYSTEM_UI_NAV_REVEAL_MAX_PX {
                UiSystemUiStateError::NavRevealOutOfRange
            } else {
                UiSystemUiStateError::NavRevealNotQuantized
            };
            assert_eq!(
                session.update_nav(reveal),
                Err(UiSystemUiSessionError::InvalidState(expected))
            );
            assert_eq!(session, pressed);
            session.cancel_nav().unwrap();
        }

        let locked = session;
        assert_eq!(
            session.focus_home(),
            Err(UiSystemUiSessionError::FocusWhileLocked)
        );
        assert_eq!(session, locked);
        assert_eq!(
            session.focus_app(ShellAppId::Phone),
            Err(UiSystemUiSessionError::FocusWhileLocked)
        );
        assert_eq!(session, locked);
        assert_eq!(
            session.cancel_nav(),
            Err(UiSystemUiSessionError::NavigationNotPressed)
        );
        assert_eq!(session, locked);

        session
            .apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::Unlock,
                None,
                1,
                session.revision(),
            )
            .unwrap();
        let home = session;
        assert_eq!(
            session.apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::Unlock,
                None,
                2,
                session.revision(),
            ),
            Err(UiSystemUiSessionError::UnlockRequiresLocked)
        );
        assert_eq!(session, home);
        assert_eq!(
            session.apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::CloseOverview,
                None,
                2,
                session.revision(),
            ),
            Err(UiSystemUiSessionError::CloseOverviewRequiresOverview)
        );
        assert_eq!(session, home);
        assert_eq!(
            session.apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::ActivateRecent,
                Some(ShellAppId::Phone),
                2,
                session.revision(),
            ),
            Err(UiSystemUiSessionError::ActivateRecentRequiresOverview)
        );
        assert_eq!(session, home);

        session.focus_app(ShellAppId::Messages).unwrap();
        session.begin_nav().unwrap();
        let dragging = session;
        for target in [UiSystemUiMode::Locked, UiSystemUiMode::Foreground] {
            assert_eq!(
                session.finish_nav(target),
                Err(UiSystemUiSessionError::InvalidNavigationFinishMode)
            );
            assert_eq!(session, dragging);
        }
        session.finish_nav(UiSystemUiMode::Overview).unwrap();
        let closed = session
            .apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::CloseOverview,
                None,
                2,
                session.revision(),
            )
            .unwrap();
        assert_eq!(closed.mode(), UiSystemUiMode::Home);

        let mut request_exhausted = session;
        request_exhausted.launcher_request_id = u64::MAX;
        let before = request_exhausted;
        assert_eq!(
            request_exhausted.apply_client_action(
                UiClientId::Launcher,
                UiSystemUiAction::Unlock,
                None,
                1,
                request_exhausted.revision(),
            ),
            Err(UiSystemUiSessionError::RequestIdExhausted)
        );
        assert_eq!(request_exhausted, before);

        let mut revision_exhausted = session;
        revision_exhausted.revision = u64::MAX;
        let before = revision_exhausted;
        assert_eq!(
            revision_exhausted.focus_home(),
            Err(UiSystemUiSessionError::RevisionExhausted)
        );
        assert_eq!(revision_exhausted, before);

        for reveal in [0, 8, 240, UI_SYSTEM_UI_NAV_REVEAL_MAX_PX] {
            let mut valid = UiSystemUiSession::new();
            valid
                .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
                .unwrap();
            valid.focus_app(ShellAppId::Phone).unwrap();
            valid.begin_nav().unwrap();
            let update = valid.update_nav(reveal).unwrap();
            assert!(update.nav_pressed());
            assert_eq!(update.nav_reveal_px(), reveal);
            valid.cancel_nav().unwrap();
            assert_eq!(valid.mode(), UiSystemUiMode::Foreground);
            assert!(!valid.nav_pressed());
            assert_eq!(valid.nav_reveal_px(), 0);
        }
    }

    #[test]
    fn system_ui_session_rejects_every_client_action_during_navigation_transactionally() {
        fn assert_shell_action_rejected(
            session: &mut UiSystemUiSession,
            action: UiSystemUiAction,
            app: Option<ShellAppId>,
            request_id: u64,
        ) {
            assert!(session.nav_pressed());
            let before = *session;
            let observed_revision = session.revision();
            assert_eq!(
                session.apply_client_action(
                    UiClientId::Launcher,
                    action,
                    app,
                    request_id,
                    observed_revision,
                ),
                Err(UiSystemUiSessionError::NavigationInProgress)
            );
            assert_eq!(*session, before);
        }

        fn assert_recent_action_rejected(
            session: &mut UiSystemUiSession,
            action: UiSystemUiAction,
            recent: UiRecentIdentity,
            request_id: u64,
        ) {
            assert!(session.nav_pressed());
            let before = *session;
            let observed_revision = session.revision();
            assert_eq!(
                session.apply_client_recent_action(
                    UiClientId::Launcher,
                    action,
                    Some(recent),
                    request_id,
                    observed_revision,
                ),
                Err(UiSystemUiSessionError::NavigationInProgress)
            );
            assert_eq!(*session, before);
        }

        let mut locked = UiSystemUiSession::new();
        locked.begin_nav().unwrap();
        assert_shell_action_rejected(&mut locked, UiSystemUiAction::Unlock, None, 1);

        let mut overview = UiSystemUiSession::new();
        overview
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        overview.focus_app(ShellAppId::Phone).unwrap();
        overview.begin_nav().unwrap();
        overview.finish_nav(UiSystemUiMode::Overview).unwrap();
        overview.begin_nav().unwrap();
        assert_shell_action_rejected(&mut overview, UiSystemUiAction::CloseOverview, None, 2);
        assert_shell_action_rejected(
            &mut overview,
            UiSystemUiAction::ActivateRecent,
            Some(ShellAppId::Phone),
            2,
        );

        let compatible_identity = UiCompatibleActivityIdentity::new(7, 11).unwrap();
        let compatible_recent = UiRecentIdentity::CompatibleAndroid(compatible_identity);

        let mut home = UiSystemUiSession::new();
        home.apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        home.begin_nav().unwrap();
        assert_recent_action_rejected(
            &mut home,
            UiSystemUiAction::PresentCompatibleActivity,
            compatible_recent,
            2,
        );

        let mut foreground = UiSystemUiSession::new();
        foreground
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        foreground
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(compatible_recent),
                2,
                foreground.revision(),
            )
            .unwrap();
        foreground
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                Some(compatible_recent),
                3,
                foreground.revision(),
            )
            .unwrap();
        foreground.begin_nav().unwrap();
        assert_recent_action_rejected(
            &mut foreground,
            UiSystemUiAction::HomeCompatibleActivity,
            compatible_recent,
            4,
        );
        assert_recent_action_rejected(
            &mut foreground,
            UiSystemUiAction::FinishCompatibleActivity,
            compatible_recent,
            4,
        );
    }

    #[test]
    fn compatible_activity_identity_wire_and_session_lifecycle_are_exact() {
        assert_eq!(UiCompatibleActivityIdentity::new(0, 1), None);
        assert_eq!(UiCompatibleActivityIdentity::new(1, 0), None);
        let identity =
            UiCompatibleActivityIdentity::new(0x1112_1314_1516_1718, 0x2122_2324_2526_2728)
                .unwrap();
        let recent = UiRecentIdentity::CompatibleAndroid(identity);
        assert_eq!(recent.compatible_android(), Some(identity));
        assert_eq!(recent.shell_app(), None);

        let control = UiClientControl::update_system_ui_recent(
            UiSystemUiAction::PresentCompatibleActivity,
            Some(recent),
            0x3132_3334_3536_3738,
            0x4142_4344_4546_4748,
        )
        .unwrap();
        let wire = control.encode();
        assert_eq!(&wire[0..4], b"BUC1");
        assert_eq!(&wire[4..8], &[8, 0, 5, 0]);
        assert_eq!(&wire[8..16], &0x0204_u64.to_le_bytes());
        assert_eq!(&wire[16..24], &0x3132_3334_3536_3738_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &0x4142_4344_4546_4748_u64.to_le_bytes());
        assert_eq!(&wire[32..40], &0x1112_1314_1516_1718_u64.to_le_bytes());
        assert_eq!(&wire[40..48], &0x2122_2324_2526_2728_u64.to_le_bytes());
        assert!(wire[48..].iter().all(|byte| *byte == 0));
        assert_eq!(UiClientControl::decode(&wire), Ok(control));

        let event = UiServerEvent::system_ui_changed_with_recent(
            0x5152_5354_5556_5758,
            UiSystemUiMode::Overview,
            Some(recent),
            false,
            0,
            0x6162_6364_6566_6768,
        )
        .unwrap();
        let event_wire = event.encode();
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(&event_wire[4..8], &[6, 0, 10, 0]);
        #[cfg(feature = "androidbox-el0-runtime0")]
        assert_eq!(&event_wire[4..8], &[7, 0, 10, 0]);
        assert_eq!(&event_wire[16..24], &0x0204_u64.to_le_bytes());
        assert_eq!(
            &event_wire[24..32],
            &0x6162_6364_6566_6768_u64.to_le_bytes()
        );
        assert_eq!(
            &event_wire[32..40],
            &0x1112_1314_1516_1718_u64.to_le_bytes()
        );
        assert_eq!(
            &event_wire[40..48],
            &0x2122_2324_2526_2728_u64.to_le_bytes()
        );
        assert!(event_wire[48..].iter().all(|byte| *byte == 0));
        assert_eq!(UiServerEvent::decode(&event_wire), Ok(event));

        let mut session = UiSystemUiSession::new();
        session
            .apply_client_recent_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        let reserved = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                2,
                session.revision(),
            )
            .unwrap();
        assert_eq!(reserved.mode(), UiSystemUiMode::Home);
        assert_eq!(reserved.recent(), None);
        let presented = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                Some(recent),
                3,
                session.revision(),
            )
            .unwrap();
        assert_eq!(presented.mode(), UiSystemUiMode::Foreground);
        assert_eq!(presented.recent(), Some(recent));

        let homed = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::HomeCompatibleActivity,
                Some(recent),
                4,
                session.revision(),
            )
            .unwrap();
        assert_eq!(homed.mode(), UiSystemUiMode::Home);
        assert_eq!(homed.recent(), Some(recent));

        session.begin_nav().unwrap();
        session.update_nav(240).unwrap();
        let overview = session.finish_nav(UiSystemUiMode::Overview).unwrap();
        assert_eq!(overview.recent(), Some(recent));
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                5,
                session.revision(),
            )
            .unwrap();
        let activated = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ActivateRecent,
                Some(recent),
                6,
                session.revision(),
            )
            .unwrap();
        assert_eq!(activated.mode(), UiSystemUiMode::Foreground);
        assert_eq!(activated.recent(), Some(recent));

        let before_mismatch = session;
        let other = UiCompatibleActivityIdentity::new(
            identity.session_id() + 1,
            identity.package_generation(),
        )
        .unwrap();
        assert_eq!(
            session.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::FinishCompatibleActivity,
                Some(UiRecentIdentity::CompatibleAndroid(other)),
                7,
                session.revision(),
            ),
            Err(UiSystemUiSessionError::RecentAppMismatch)
        );
        assert_eq!(session, before_mismatch);

        let finished = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::FinishCompatibleActivity,
                Some(recent),
                7,
                session.revision(),
            )
            .unwrap();
        assert_eq!(finished.mode(), UiSystemUiMode::Home);
        assert_eq!(finished.recent(), None);
    }

    #[test]
    fn compatible_activity_reservation_lifecycle_blocks_mutators_and_commits_by_origin() {
        let identity = UiCompatibleActivityIdentity::new(41, 7).unwrap();
        let recent = UiRecentIdentity::CompatibleAndroid(identity);
        let mut home = UiSystemUiSession::new();
        home.apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        home.focus_app(ShellAppId::Phone).unwrap();
        home.focus_home().unwrap();
        let original_recent = home.recent();
        let reserved = home
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                2,
                home.revision(),
            )
            .unwrap();
        assert_eq!(reserved.mode(), UiSystemUiMode::Home);
        assert_eq!(reserved.recent(), original_recent);
        let reservation = home.compatible_activity_reservation().unwrap();
        assert!(home.has_compatible_activity_reservation());
        assert_eq!(reservation.identity(), identity);
        assert_eq!(
            reservation.origin(),
            UiCompatibleActivityReservationOrigin::Home
        );
        assert_eq!(reservation.request_id(), 2);
        assert_eq!(reservation.revision(), reserved.revision());

        for result in [
            home.focus_app(ShellAppId::Messages),
            home.focus_home(),
            home.begin_nav(),
            home.update_nav(8),
            home.finish_nav(UiSystemUiMode::Overview),
            home.cancel_nav(),
        ] {
            assert_eq!(
                result,
                Err(UiSystemUiSessionError::CompatibleActivityReservationInProgress)
            );
            assert_eq!(home.compatible_activity_reservation(), Some(reservation));
            assert_eq!(home.snapshot(), reserved);
        }
        for (action, requested, expected) in [
            (
                UiSystemUiAction::Unlock,
                None,
                UiSystemUiSessionError::CompatibleActivityReservationInProgress,
            ),
            (
                UiSystemUiAction::CloseOverview,
                None,
                UiSystemUiSessionError::CompatibleActivityReservationInProgress,
            ),
            (
                UiSystemUiAction::HomeCompatibleActivity,
                Some(UiRecentIdentity::CompatibleAndroid(identity)),
                UiSystemUiSessionError::CompatibleActivityReservationInProgress,
            ),
            (
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(UiRecentIdentity::CompatibleAndroid(identity)),
                UiSystemUiSessionError::CompatibleActivityReservationInProgress,
            ),
            (
                UiSystemUiAction::ActivateRecent,
                Some(UiRecentIdentity::Shell(ShellAppId::Phone)),
                UiSystemUiSessionError::CompatibleActivityReservationIdentityMismatch,
            ),
        ] {
            let before = home;
            assert_eq!(
                home.apply_client_recent_action(
                    UiClientId::Launcher,
                    action,
                    requested,
                    3,
                    reservation.revision(),
                ),
                Err(expected)
            );
            assert_eq!(home, before);
        }
        let before_wrong_origin = home;
        assert_eq!(
            home.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ActivateRecent,
                Some(recent),
                3,
                reservation.revision(),
            ),
            Err(UiSystemUiSessionError::CompatibleActivityReservationOriginMismatch)
        );
        assert_eq!(home, before_wrong_origin);

        let foreground = home
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                Some(recent),
                3,
                reservation.revision(),
            )
            .unwrap();
        assert_eq!(foreground.mode(), UiSystemUiMode::Foreground);
        assert_eq!(foreground.recent(), Some(recent));
        assert!(!home.has_compatible_activity_reservation());

        home.apply_client_recent_action(
            UiClientId::Launcher,
            UiSystemUiAction::HomeCompatibleActivity,
            Some(recent),
            4,
            home.revision(),
        )
        .unwrap();
        home.begin_nav().unwrap();
        home.finish_nav(UiSystemUiMode::Overview).unwrap();
        let overview_reserve = home
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                5,
                home.revision(),
            )
            .unwrap();
        assert_eq!(
            home.compatible_activity_reservation().unwrap().origin(),
            UiCompatibleActivityReservationOrigin::Overview
        );
        let before_wrong_origin = home;
        assert_eq!(
            home.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                Some(recent),
                6,
                overview_reserve.revision(),
            ),
            Err(UiSystemUiSessionError::CompatibleActivityReservationOriginMismatch)
        );
        assert_eq!(home, before_wrong_origin);
        let reactivated = home
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ActivateRecent,
                Some(recent),
                6,
                overview_reserve.revision(),
            )
            .unwrap();
        assert_eq!(reactivated.mode(), UiSystemUiMode::Foreground);
        assert_eq!(reactivated.recent(), Some(recent));
        assert!(!home.has_compatible_activity_reservation());
    }

    #[test]
    fn compatible_activity_reservation_failures_abort_and_stale_finish_are_atomic() {
        let identity = UiCompatibleActivityIdentity::new(51, 9).unwrap();
        let other = UiCompatibleActivityIdentity::new(52, 9).unwrap();
        let recent = UiRecentIdentity::CompatibleAndroid(identity);
        let mut session = UiSystemUiSession::new();
        session
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        session.focus_app(ShellAppId::Settings).unwrap();
        session.focus_home().unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                2,
                session.revision(),
            )
            .unwrap();
        let reservation = session.compatible_activity_reservation().unwrap();

        for (client, action, requested, request_id, observed_revision, expected) in [
            (
                UiClientId::App,
                UiSystemUiAction::AbortCompatibleActivityVerification,
                recent,
                3,
                reservation.revision(),
                UiSystemUiSessionError::LauncherOnly,
            ),
            (
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                UiRecentIdentity::CompatibleAndroid(other),
                3,
                reservation.revision(),
                UiSystemUiSessionError::CompatibleActivityReservationIdentityMismatch,
            ),
            (
                UiClientId::Launcher,
                UiSystemUiAction::AbortCompatibleActivityVerification,
                recent,
                2,
                reservation.revision(),
                UiSystemUiSessionError::RequestReplay,
            ),
            (
                UiClientId::Launcher,
                UiSystemUiAction::AbortCompatibleActivityVerification,
                recent,
                4,
                reservation.revision(),
                UiSystemUiSessionError::RequestGap,
            ),
            (
                UiClientId::Launcher,
                UiSystemUiAction::AbortCompatibleActivityVerification,
                recent,
                3,
                reservation.revision() - 1,
                UiSystemUiSessionError::ObservedRevisionMismatch,
            ),
        ] {
            let before = session;
            assert_eq!(
                session.apply_client_recent_action(
                    client,
                    action,
                    Some(requested),
                    request_id,
                    observed_revision,
                ),
                Err(expected)
            );
            assert_eq!(session, before);
        }

        let before_abort = session.snapshot();
        let aborted = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::AbortCompatibleActivityVerification,
                Some(recent),
                3,
                reservation.revision(),
            )
            .unwrap();
        assert_eq!(aborted.mode(), before_abort.mode());
        assert_eq!(aborted.recent(), before_abort.recent());
        assert_eq!(aborted.revision(), before_abort.revision() + 1);
        assert!(!session.has_compatible_activity_reservation());

        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                4,
                session.revision(),
            )
            .unwrap();
        let stale_finished = session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::FinishCompatibleActivity,
                Some(recent),
                5,
                session.revision(),
            )
            .unwrap();
        assert_eq!(stale_finished.mode(), UiSystemUiMode::Home);
        assert_eq!(stale_finished.recent(), None);
        assert!(!session.has_compatible_activity_reservation());

        let mut request_exhausted = UiSystemUiSession::new();
        request_exhausted
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        request_exhausted.launcher_request_id = u64::MAX - 1;
        let before = request_exhausted;
        assert_eq!(
            request_exhausted.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                u64::MAX,
                request_exhausted.revision(),
            ),
            Err(UiSystemUiSessionError::RequestIdExhausted)
        );
        assert_eq!(request_exhausted, before);

        let mut revision_exhausted = UiSystemUiSession::new();
        revision_exhausted
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        revision_exhausted.revision = u64::MAX - 1;
        let before = revision_exhausted;
        assert_eq!(
            revision_exhausted.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                2,
                revision_exhausted.revision(),
            ),
            Err(UiSystemUiSessionError::RevisionExhausted)
        );
        assert_eq!(revision_exhausted, before);
    }

    #[test]
    fn home_compatible_reservation_must_reuse_an_existing_compatible_identity() {
        let identity = UiCompatibleActivityIdentity::new(61, 11).unwrap();
        let other = UiCompatibleActivityIdentity::new(62, 11).unwrap();
        let recent = UiRecentIdentity::CompatibleAndroid(identity);
        let mut session = UiSystemUiSession::new();
        session
            .apply_client_action(UiClientId::Launcher, UiSystemUiAction::Unlock, None, 1, 1)
            .unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                2,
                session.revision(),
            )
            .unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::PresentCompatibleActivity,
                Some(recent),
                3,
                session.revision(),
            )
            .unwrap();
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::HomeCompatibleActivity,
                Some(recent),
                4,
                session.revision(),
            )
            .unwrap();

        let before = session;
        assert_eq!(
            session.apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(UiRecentIdentity::CompatibleAndroid(other)),
                5,
                session.revision(),
            ),
            Err(UiSystemUiSessionError::RecentAppMismatch)
        );
        assert_eq!(session, before);
        session
            .apply_client_recent_action(
                UiClientId::Launcher,
                UiSystemUiAction::ReserveCompatibleActivity,
                Some(recent),
                5,
                session.revision(),
            )
            .unwrap();
        assert_eq!(
            session
                .compatible_activity_reservation()
                .unwrap()
                .identity(),
            identity
        );
    }

    #[test]
    fn client_control_branches_have_one_exact_little_endian_wire_format() {
        assert_eq!(UI_CLIENT_CONTROL_WIRE_SIZE, 64);
        let attach = UiClientControl::attach_app_endpoint(UiClientId::App).unwrap();
        let launcher_focus = UiClientControl::set_focus(UiClientId::Launcher, None, 7).unwrap();
        let app_focus =
            UiClientControl::set_focus(UiClientId::App, Some(ShellAppId::Settings), 9).unwrap();
        let appearance = UiClientControl::update_appearance(
            UiAppearanceAction::ToggleAccent,
            0x1112_1314_1516_1718,
        )
        .unwrap();
        let maximum_dimming = UiClientControl::update_appearance(
            UiAppearanceAction::SetSoftwareDimmingMaximum,
            0x2122_2324_2526_2728,
        )
        .unwrap();
        let dismiss_boot_notification =
            UiClientControl::dismiss_boot_notification(0x3132_3334_3536_3738).unwrap();
        let activate_recent = UiClientControl::update_system_ui(
            UiSystemUiAction::ActivateRecent,
            Some(ShellAppId::Settings),
            0x4142_4344_4546_4748,
            0x5152_5354_5556_5758,
        )
        .unwrap();
        let androidbox_tap = UiClientControl::report_androidbox_execution(
            0x6162_6364_6566_6768,
            UiAndroidBoxExecutionKind::Tap,
            0x1122_3344,
            0x5566_7788,
            -123,
            UI_ANDROIDBOX_MAX_INSTRUCTION_COUNT,
            0xabcd,
        )
        .unwrap();
        for control in [
            attach,
            launcher_focus,
            app_focus,
            appearance,
            maximum_dimming,
            dismiss_boot_notification,
            activate_recent,
            androidbox_tap,
        ] {
            assert_eq!(UiClientControl::decode(&control.encode()), Ok(control));
        }

        let attach_wire = attach.encode();
        assert_eq!(&attach_wire[0..4], b"BUC1");
        assert_eq!(&attach_wire[4..8], &[8, 0, 1, 0]);
        assert_eq!(&attach_wire[8..16], &2_u64.to_le_bytes());
        assert!(attach_wire[16..].iter().all(|byte| *byte == 0));

        let launcher_wire = launcher_focus.encode();
        assert_eq!(&launcher_wire[8..16], &1_u64.to_le_bytes());
        assert_eq!(&launcher_wire[16..24], &0_u64.to_le_bytes());
        assert_eq!(&launcher_wire[24..32], &7_u64.to_le_bytes());
        assert!(launcher_wire[32..].iter().all(|byte| *byte == 0));

        let app_wire = app_focus.encode();
        assert_eq!(&app_wire[8..16], &2_u64.to_le_bytes());
        assert_eq!(&app_wire[16..24], &3_u64.to_le_bytes());
        assert_eq!(&app_wire[24..32], &9_u64.to_le_bytes());

        let appearance_wire = appearance.encode();
        assert_eq!(&appearance_wire[0..4], b"BUC1");
        assert_eq!(&appearance_wire[4..8], &[8, 0, 3, 0]);
        assert_eq!(&appearance_wire[8..16], &2_u64.to_le_bytes());
        assert_eq!(
            &appearance_wire[16..24],
            &0x1112_1314_1516_1718_u64.to_le_bytes()
        );
        assert!(appearance_wire[24..].iter().all(|byte| *byte == 0));

        let maximum_dimming_wire = maximum_dimming.encode();
        assert_eq!(&maximum_dimming_wire[8..16], &7_u64.to_le_bytes());
        assert_eq!(
            &maximum_dimming_wire[16..24],
            &0x2122_2324_2526_2728_u64.to_le_bytes()
        );
        assert!(maximum_dimming_wire[24..].iter().all(|byte| *byte == 0));

        let dismiss_wire = dismiss_boot_notification.encode();
        assert_eq!(&dismiss_wire[0..4], b"BUC1");
        assert_eq!(&dismiss_wire[4..8], &[8, 0, 4, 0]);
        assert_eq!(
            &dismiss_wire[8..16],
            &0x3132_3334_3536_3738_u64.to_le_bytes()
        );
        assert!(dismiss_wire[16..].iter().all(|byte| *byte == 0));

        let system_ui_wire = activate_recent.encode();
        assert_eq!(&system_ui_wire[0..4], b"BUC1");
        assert_eq!(&system_ui_wire[4..8], &[8, 0, 5, 0]);
        assert_eq!(&system_ui_wire[8..16], &0x0003_0103_u64.to_le_bytes());
        assert_eq!(
            &system_ui_wire[16..24],
            &0x4142_4344_4546_4748_u64.to_le_bytes()
        );
        assert_eq!(
            &system_ui_wire[24..32],
            &0x5152_5354_5556_5758_u64.to_le_bytes()
        );
        assert!(system_ui_wire[32..].iter().all(|byte| *byte == 0));

        let androidbox_wire = androidbox_tap.encode();
        assert_eq!(&androidbox_wire[0..4], b"BUC1");
        assert_eq!(&androidbox_wire[4..8], &[8, 0, 6, 0]);
        assert_eq!(
            &androidbox_wire[8..16],
            &0x6162_6364_6566_6768_u64.to_le_bytes()
        );
        assert_eq!(
            &androidbox_wire[16..24],
            &0x5566_7788_1122_3344_u64.to_le_bytes()
        );
        assert_eq!(
            &androidbox_wire[24..32],
            &0xabcd_4002_ffff_ff85_u64.to_le_bytes()
        );
        assert!(androidbox_wire[32..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn client_control_constructors_enforce_cross_field_invariants() {
        assert_eq!(UiClientId::from_raw(1), Some(UiClientId::Launcher));
        assert_eq!(UiClientId::from_raw(2), Some(UiClientId::App));
        assert_eq!(UiClientId::from_raw(0), None);
        assert_eq!(UiClientId::from_raw(3), None);
        assert_eq!(ShellAppId::from_raw(1), Some(ShellAppId::Phone));
        assert_eq!(ShellAppId::from_raw(3), Some(ShellAppId::Settings));
        assert_eq!(ShellAppId::from_raw(0), None);
        assert_eq!(ShellAppId::from_raw(4), None);
        assert_eq!(UiClientId::Launcher.raw(), 1);
        assert_eq!(UiClientId::App.raw(), 2);
        assert_eq!(ShellAppId::Messages.raw(), 2);
        assert_eq!(
            UiAppearanceAction::from_raw(1),
            Some(UiAppearanceAction::ToggleTheme)
        );
        assert_eq!(
            UiAppearanceAction::from_raw(2),
            Some(UiAppearanceAction::ToggleAccent)
        );
        for (raw, software_dimming, action) in [
            (
                3,
                UiSoftwareDimming::Off,
                UiAppearanceAction::SetSoftwareDimmingOff,
            ),
            (
                4,
                UiSoftwareDimming::Light,
                UiAppearanceAction::SetSoftwareDimmingLight,
            ),
            (
                5,
                UiSoftwareDimming::Medium,
                UiAppearanceAction::SetSoftwareDimmingMedium,
            ),
            (
                6,
                UiSoftwareDimming::Strong,
                UiAppearanceAction::SetSoftwareDimmingStrong,
            ),
            (
                7,
                UiSoftwareDimming::Maximum,
                UiAppearanceAction::SetSoftwareDimmingMaximum,
            ),
        ] {
            assert_eq!(UiAppearanceAction::from_raw(raw), Some(action));
            assert_eq!(action.raw(), raw as u8);
            assert_eq!(action.software_dimming(), Some(software_dimming));
            assert_eq!(
                UiAppearanceAction::set_software_dimming(software_dimming),
                action
            );
        }
        assert_eq!(UiAppearanceAction::ToggleTheme.software_dimming(), None);
        assert_eq!(UiAppearanceAction::ToggleAccent.software_dimming(), None);
        assert_eq!(UiAppearanceAction::from_raw(0), None);
        assert_eq!(UiAppearanceAction::from_raw(8), None);
        for (raw, mode) in [
            (1, UiSystemUiMode::Locked),
            (2, UiSystemUiMode::Home),
            (3, UiSystemUiMode::Foreground),
            (4, UiSystemUiMode::Overview),
        ] {
            assert_eq!(UiSystemUiMode::from_raw(raw), Some(mode));
            assert_eq!(mode.raw(), raw as u8);
        }
        assert_eq!(UiSystemUiMode::from_raw(0), None);
        assert_eq!(UiSystemUiMode::from_raw(5), None);
        for (raw, action) in [
            (1, UiSystemUiAction::Unlock),
            (2, UiSystemUiAction::CloseOverview),
            (3, UiSystemUiAction::ActivateRecent),
            (4, UiSystemUiAction::PresentCompatibleActivity),
            (5, UiSystemUiAction::HomeCompatibleActivity),
            (6, UiSystemUiAction::FinishCompatibleActivity),
            (7, UiSystemUiAction::ReserveCompatibleActivity),
            (8, UiSystemUiAction::AbortCompatibleActivityVerification),
        ] {
            assert_eq!(UiSystemUiAction::from_raw(raw), Some(action));
            assert_eq!(action.raw(), raw as u8);
        }
        assert_eq!(UiSystemUiAction::from_raw(0), None);
        assert_eq!(UiSystemUiAction::from_raw(9), None);
        assert_eq!(
            UiAndroidBoxExecutionKind::from_raw(1),
            Some(UiAndroidBoxExecutionKind::Boot)
        );
        assert_eq!(
            UiAndroidBoxExecutionKind::from_raw(2),
            Some(UiAndroidBoxExecutionKind::Tap)
        );
        assert_eq!(UiAndroidBoxExecutionKind::from_raw(0), None);
        assert_eq!(UiAndroidBoxExecutionKind::from_raw(3), None);
        assert_eq!(
            UiClientControl::attach_app_endpoint(UiClientId::Launcher),
            Err(UiClientControlError::AttachRequiresAppClient)
        );
        assert_eq!(
            UiClientControl::set_focus(UiClientId::Launcher, Some(ShellAppId::Phone), 1),
            Err(UiClientControlError::LauncherMustNotSpecifyApp)
        );
        assert_eq!(
            UiClientControl::set_focus(UiClientId::App, None, 1),
            Err(UiClientControlError::AppRequiresShellApp)
        );
        assert_eq!(
            UiClientControl::set_focus(UiClientId::Launcher, None, 0),
            Err(UiClientControlError::ZeroTransitionId)
        );
        assert_eq!(
            UiClientControl::update_appearance(UiAppearanceAction::ToggleTheme, 0),
            Err(UiClientControlError::ZeroAppearanceRequestId)
        );
        assert_eq!(
            UiClientControl::dismiss_boot_notification(0),
            Err(UiClientControlError::ZeroBootNotificationRequestId)
        );
        assert_eq!(
            UiClientControl::update_system_ui(
                UiSystemUiAction::Unlock,
                Some(ShellAppId::Phone),
                1,
                1,
            ),
            Err(UiClientControlError::SystemUiActionMustNotSpecifyApp)
        );
        assert_eq!(
            UiClientControl::update_system_ui(
                UiSystemUiAction::CloseOverview,
                Some(ShellAppId::Phone),
                1,
                1,
            ),
            Err(UiClientControlError::SystemUiActionMustNotSpecifyApp)
        );
        assert_eq!(
            UiClientControl::update_system_ui(UiSystemUiAction::ActivateRecent, None, 1, 1,),
            Err(UiClientControlError::SystemUiActionRequiresApp)
        );
        assert_eq!(
            UiClientControl::update_system_ui(UiSystemUiAction::Unlock, None, 0, 1),
            Err(UiClientControlError::ZeroSystemUiRequestId)
        );
        assert_eq!(
            UiClientControl::update_system_ui(UiSystemUiAction::Unlock, None, 1, 0),
            Err(UiClientControlError::ZeroObservedSystemUiRevision)
        );
        assert_eq!(
            UiClientControl::report_androidbox_execution(
                0,
                UiAndroidBoxExecutionKind::Boot,
                1,
                2,
                0,
                1,
                0,
            ),
            Err(UiClientControlError::ZeroAndroidBoxRequestId)
        );
        assert_eq!(
            UiClientControl::report_androidbox_execution(
                1,
                UiAndroidBoxExecutionKind::Boot,
                1,
                2,
                0,
                0,
                0,
            ),
            Err(UiClientControlError::ZeroAndroidBoxInstructionCount)
        );
        assert_eq!(
            UiClientControl::report_androidbox_execution(
                1,
                UiAndroidBoxExecutionKind::Boot,
                1,
                2,
                0,
                UI_ANDROIDBOX_MAX_INSTRUCTION_COUNT + 1,
                0,
            ),
            Err(UiClientControlError::AndroidBoxInstructionCountOutOfRange)
        );
        assert_eq!(
            UiClientControl::report_androidbox_execution(
                1,
                UiAndroidBoxExecutionKind::Boot,
                1,
                2,
                0,
                1,
                1,
            ),
            Err(UiClientControlError::AndroidBoxBootTapCountMustBeZero)
        );
        assert_eq!(
            UiClientControl::report_androidbox_execution(
                1,
                UiAndroidBoxExecutionKind::Tap,
                1,
                2,
                0,
                1,
                0,
            ),
            Err(UiClientControlError::AndroidBoxTapCountMustBeNonZero)
        );
        for control in [
            UiClientControl::update_system_ui(UiSystemUiAction::Unlock, None, 1, 1).unwrap(),
            UiClientControl::update_system_ui(UiSystemUiAction::CloseOverview, None, 2, 2).unwrap(),
            UiClientControl::update_system_ui(
                UiSystemUiAction::ActivateRecent,
                Some(ShellAppId::Messages),
                3,
                3,
            )
            .unwrap(),
        ] {
            assert_eq!(UiClientControl::decode(&control.encode()), Ok(control));
        }
    }

    #[test]
    fn client_control_decoder_rejects_lengths_headers_and_all_reserved_bytes() {
        let canonical = UiClientControl::attach_app_endpoint(UiClientId::App)
            .unwrap()
            .encode();
        assert_eq!(
            UiClientControl::decode(&canonical[..UI_CLIENT_CONTROL_WIRE_SIZE - 1]),
            Err(UiClientControlError::InvalidWireLength)
        );
        let mut oversized = [0; UI_CLIENT_CONTROL_WIRE_SIZE + 1];
        oversized[..UI_CLIENT_CONTROL_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            UiClientControl::decode(&oversized),
            Err(UiClientControlError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, UiClientControlError::InvalidMagic),
            (4, UiClientControlError::InvalidVersion),
            (6, UiClientControlError::InvalidKind),
            (7, UiClientControlError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(UiClientControl::decode(&wire), Err(expected));
        }
        let mut version_six = canonical;
        version_six[CONTROL_OFFSET_VERSION..CONTROL_OFFSET_VERSION + 2]
            .copy_from_slice(&6_u16.to_le_bytes());
        assert_eq!(
            UiClientControl::decode(&version_six),
            Err(UiClientControlError::InvalidVersion)
        );
        for offset in CONTROL_OFFSET_BODY_RESERVED..UI_CLIENT_CONTROL_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&wire),
                Err(UiClientControlError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
        assert_eq!(UiClientControlKind::from_raw(0), None);
        assert_eq!(
            UiClientControlKind::from_raw(3),
            Some(UiClientControlKind::UpdateAppearance)
        );
        assert_eq!(
            UiClientControlKind::from_raw(4),
            Some(UiClientControlKind::DismissBootNotification)
        );
        assert_eq!(
            UiClientControlKind::from_raw(5),
            Some(UiClientControlKind::UpdateSystemUi)
        );
        assert_eq!(
            UiClientControlKind::from_raw(6),
            Some(UiClientControlKind::ReportAndroidBoxExecution)
        );
        assert_eq!(
            UiClientControlKind::from_raw(7),
            Some(UiClientControlKind::ReportAndroidBoxActivityExecution)
        );
        assert_eq!(
            UiClientControlKind::from_raw(8),
            Some(UiClientControlKind::ReportAndroidBoxResourceExecution)
        );
        assert_eq!(UiClientControlKind::from_raw(9), None);
    }

    #[test]
    fn client_control_decoder_rejects_noncanonical_and_cross_kind_arguments() {
        let attach = UiClientControl::attach_app_endpoint(UiClientId::App)
            .unwrap()
            .encode();
        let mut launcher_attach = attach;
        launcher_attach[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8]
            .copy_from_slice(&1_u64.to_le_bytes());
        assert_eq!(
            UiClientControl::decode(&launcher_attach),
            Err(UiClientControlError::AttachRequiresAppClient)
        );
        let mut invalid_client = attach;
        invalid_client[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8]
            .copy_from_slice(&3_u64.to_le_bytes());
        assert_eq!(
            UiClientControl::decode(&invalid_client),
            Err(UiClientControlError::InvalidClient)
        );
        for offset in CONTROL_OFFSET_APP..CONTROL_OFFSET_BODY_RESERVED {
            let mut cross_kind = attach;
            cross_kind[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&cross_kind),
                Err(UiClientControlError::UnexpectedPayload),
                "offset {offset}"
            );
        }

        let system_ui = UiClientControl::update_system_ui(
            UiSystemUiAction::ActivateRecent,
            Some(ShellAppId::Messages),
            1,
            7,
        )
        .unwrap()
        .encode();
        for (raw, expected) in [
            (0_u64, UiClientControlError::InvalidSystemUiAction),
            (
                4,
                UiClientControlError::SystemUiActionRequiresCompatibleActivity,
            ),
            (0x0100, UiClientControlError::InvalidSystemUiAction),
            (0x0403, UiClientControlError::InvalidRecentIdentity),
            (1 << 16, UiClientControlError::InvalidSystemUiAction),
            (u64::MAX, UiClientControlError::NonCanonicalSystemUiAction),
        ] {
            let mut invalid = system_ui;
            invalid[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8]
                .copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                UiClientControl::decode(&invalid),
                Err(expected),
                "raw {raw:#x}"
            );
        }
        let mut unlock_with_app =
            UiClientControl::update_system_ui(UiSystemUiAction::Unlock, None, 1, 1)
                .unwrap()
                .encode();
        unlock_with_app[CONTROL_OFFSET_CLIENT + 1] = SYSTEM_UI_RECENT_KIND_SHELL;
        unlock_with_app[CONTROL_OFFSET_CLIENT + 2] = ShellAppId::Phone.raw();
        assert_eq!(
            UiClientControl::decode(&unlock_with_app),
            Err(UiClientControlError::SystemUiActionMustNotSpecifyApp)
        );
        let mut activate_without_app = system_ui;
        activate_without_app[CONTROL_OFFSET_CLIENT + 1] = 0;
        activate_without_app[CONTROL_OFFSET_CLIENT + 2] = 0;
        assert_eq!(
            UiClientControl::decode(&activate_without_app),
            Err(UiClientControlError::SystemUiActionRequiresApp)
        );
        let mut zero_system_ui_request = system_ui;
        zero_system_ui_request[CONTROL_OFFSET_APP..CONTROL_OFFSET_APP + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_system_ui_request),
            Err(UiClientControlError::ZeroSystemUiRequestId)
        );
        let mut zero_observed_revision = system_ui;
        zero_observed_revision[CONTROL_OFFSET_TRANSITION_ID..CONTROL_OFFSET_TRANSITION_ID + 8]
            .fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_observed_revision),
            Err(UiClientControlError::ZeroObservedSystemUiRevision)
        );

        let androidbox = UiClientControl::report_androidbox_execution(
            1,
            UiAndroidBoxExecutionKind::Boot,
            0x1122_3344,
            0x5566_7788,
            -1,
            64,
            0,
        )
        .unwrap()
        .encode();
        let mut zero_androidbox_request = androidbox;
        zero_androidbox_request[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_androidbox_request),
            Err(UiClientControlError::ZeroAndroidBoxRequestId)
        );
        for raw in [0_u8, 3, u8::MAX] {
            let mut invalid_kind = androidbox;
            invalid_kind[CONTROL_OFFSET_TRANSITION_ID + 4] = raw;
            assert_eq!(
                UiClientControl::decode(&invalid_kind),
                Err(UiClientControlError::InvalidAndroidBoxExecutionKind)
            );
        }
        let mut zero_instructions = androidbox;
        zero_instructions[CONTROL_OFFSET_TRANSITION_ID + 5] = 0;
        assert_eq!(
            UiClientControl::decode(&zero_instructions),
            Err(UiClientControlError::ZeroAndroidBoxInstructionCount)
        );
        let mut too_many_instructions = androidbox;
        too_many_instructions[CONTROL_OFFSET_TRANSITION_ID + 5] = 65;
        assert_eq!(
            UiClientControl::decode(&too_many_instructions),
            Err(UiClientControlError::AndroidBoxInstructionCountOutOfRange)
        );
        let mut noncanonical_reserved = androidbox;
        noncanonical_reserved[CONTROL_OFFSET_TRANSITION_ID + 5] |= 0x80;
        assert_eq!(
            UiClientControl::decode(&noncanonical_reserved),
            Err(UiClientControlError::NonCanonicalAndroidBoxExecution)
        );
        let mut boot_with_tap = androidbox;
        boot_with_tap[CONTROL_OFFSET_TRANSITION_ID + 6] = 1;
        assert_eq!(
            UiClientControl::decode(&boot_with_tap),
            Err(UiClientControlError::AndroidBoxBootTapCountMustBeZero)
        );
        let mut tap_without_count = androidbox;
        tap_without_count[CONTROL_OFFSET_TRANSITION_ID + 4] = UiAndroidBoxExecutionKind::Tap.raw();
        assert_eq!(
            UiClientControl::decode(&tap_without_count),
            Err(UiClientControlError::AndroidBoxTapCountMustBeNonZero)
        );

        let launcher = UiClientControl::set_focus(UiClientId::Launcher, None, 1)
            .unwrap()
            .encode();
        let mut launcher_app = launcher;
        launcher_app[CONTROL_OFFSET_APP] = 1;
        assert_eq!(
            UiClientControl::decode(&launcher_app),
            Err(UiClientControlError::LauncherMustNotSpecifyApp)
        );
        let mut zero_transition = launcher;
        zero_transition[CONTROL_OFFSET_TRANSITION_ID..CONTROL_OFFSET_TRANSITION_ID + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_transition),
            Err(UiClientControlError::ZeroTransitionId)
        );
        let app = UiClientControl::set_focus(UiClientId::App, Some(ShellAppId::Phone), 1)
            .unwrap()
            .encode();
        let mut app_without_id = app;
        app_without_id[CONTROL_OFFSET_APP..CONTROL_OFFSET_APP + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&app_without_id),
            Err(UiClientControlError::AppRequiresShellApp)
        );

        let appearance = UiClientControl::update_appearance(
            UiAppearanceAction::ToggleTheme,
            0x0102_0304_0506_0708,
        )
        .unwrap()
        .encode();
        for raw in [0_u64, 8, 0x101] {
            let mut invalid_action = appearance;
            invalid_action[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8]
                .copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                UiClientControl::decode(&invalid_action),
                Err(UiClientControlError::InvalidAppearanceAction)
            );
        }
        let mut zero_request = appearance;
        zero_request[CONTROL_OFFSET_APP..CONTROL_OFFSET_APP + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_request),
            Err(UiClientControlError::ZeroAppearanceRequestId)
        );
        for offset in CONTROL_OFFSET_TRANSITION_ID..CONTROL_OFFSET_BODY_RESERVED {
            let mut cross_kind = appearance;
            cross_kind[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&cross_kind),
                Err(UiClientControlError::UnexpectedPayload),
                "offset {offset}"
            );
        }

        let dismissal = UiClientControl::dismiss_boot_notification(1)
            .unwrap()
            .encode();
        let mut zero_request = dismissal;
        zero_request[CONTROL_OFFSET_CLIENT..CONTROL_OFFSET_CLIENT + 8].fill(0);
        assert_eq!(
            UiClientControl::decode(&zero_request),
            Err(UiClientControlError::ZeroBootNotificationRequestId)
        );
        for offset in CONTROL_OFFSET_APP..CONTROL_OFFSET_BODY_RESERVED {
            let mut cross_kind = dismissal;
            cross_kind[offset] = 1;
            assert_eq!(
                UiClientControl::decode(&cross_kind),
                Err(UiClientControlError::UnexpectedPayload),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn client_control_tracker_allows_gaps_and_rejects_replay_transactionally() {
        let attach = UiClientControl::attach_app_endpoint(UiClientId::App).unwrap();
        let mut tracker = UiClientControlTracker::new();
        tracker.accept(attach).unwrap();
        assert_eq!(tracker, UiClientControlTracker::new());
        tracker
            .accept(UiClientControl::update_appearance(UiAppearanceAction::ToggleTheme, 1).unwrap())
            .unwrap();
        assert_eq!(tracker, UiClientControlTracker::new());
        tracker
            .accept(UiClientControl::dismiss_boot_notification(1).unwrap())
            .unwrap();
        assert_eq!(tracker, UiClientControlTracker::new());
        tracker
            .accept(
                UiClientControl::update_system_ui(UiSystemUiAction::Unlock, None, 1, 1).unwrap(),
            )
            .unwrap();
        assert_eq!(tracker, UiClientControlTracker::new());
        tracker
            .accept(
                UiClientControl::report_androidbox_execution(
                    1,
                    UiAndroidBoxExecutionKind::Boot,
                    1,
                    2,
                    3,
                    4,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker, UiClientControlTracker::new());
        tracker
            .accept(UiClientControl::set_focus(UiClientId::Launcher, None, 7).unwrap())
            .unwrap();
        tracker
            .accept(
                UiClientControl::set_focus(UiClientId::App, Some(ShellAppId::Messages), 42)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.last_transition_id(), Some(42));
        assert_eq!(tracker.active_client(), Some(UiClientId::App));
        assert_eq!(tracker.active_app(), Some(ShellAppId::Messages));
        let accepted = tracker;
        for transition_id in [41, 42] {
            assert_eq!(
                tracker.accept(
                    UiClientControl::set_focus(UiClientId::Launcher, None, transition_id).unwrap()
                ),
                Err(UiClientControlTrackerError::TransitionReplay)
            );
            assert_eq!(tracker, accepted);
        }
        tracker.last_transition_id = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker
                .accept(UiClientControl::set_focus(UiClientId::Launcher, None, u64::MAX).unwrap()),
            Err(UiClientControlTrackerError::TransitionExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_branches_have_one_exact_little_endian_wire_format() {
        assert_eq!(UI_SERVER_EVENT_WIRE_SIZE, 64);
        let session_id = 0x0102_0304_0506_0708;
        let sample = InputSample::try_new(0x2122_2324_2526_2728, 0x0123, 0x0145, true).unwrap();
        let events = [
            UiServerEvent::ready(session_id).unwrap(),
            UiServerEvent::input_sample(session_id, sample).unwrap(),
            UiServerEvent::presented(session_id, 0x1122_3344, 0x3132_3334_3536_3738).unwrap(),
            UiServerEvent::degraded(session_id, UiServerDegradedReason::KernelDegraded).unwrap(),
            UiServerEvent::focus_changed(
                session_id,
                UiClientId::App,
                Some(ShellAppId::Messages),
                0x4142_4344_4546_4748,
            )
            .unwrap(),
            UiServerEvent::present_cancelled(session_id, 0x5152_5354, 0x6162_6364_6566_6768)
                .unwrap(),
            UiServerEvent::appearance_changed(
                session_id,
                UiAppearance::new(false, true).with_software_dimming(UiSoftwareDimming::Maximum),
                0x7172_7374_7576_7778,
            )
            .unwrap(),
            UiServerEvent::clock_changed(session_id, 0x8182_8384_8586_8788, 0x9192_9394_9596_9798)
                .unwrap(),
            UiServerEvent::boot_notification_changed(session_id, false, 0xa1a2_a3a4_a5a6_a7a8)
                .unwrap(),
            UiServerEvent::system_ui_changed(
                session_id,
                UiSystemUiMode::Foreground,
                Some(ShellAppId::Settings),
                true,
                480,
                0xb1b2_b3b4_b5b6_b7b8,
            )
            .unwrap(),
        ];
        for event in events {
            assert_eq!(UiServerEvent::decode(&event.encode()), Ok(event));
        }

        let ready = events[0].encode();
        assert_eq!(&ready[0..4], b"BUE1");
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(&ready[4..8], &[6, 0, 1, 0]);
        #[cfg(feature = "androidbox-el0-runtime0")]
        assert_eq!(&ready[4..8], &[7, 0, 1, 0]);
        assert_eq!(&ready[8..16], &session_id.to_le_bytes());
        assert!(ready[16..].iter().all(|byte| *byte == 0));

        let input = events[1].encode();
        let (packed_state, sequence) = sample.encode_registers();
        assert_eq!(input[EVENT_OFFSET_KIND], UiServerEventKind::Input.raw());
        assert_eq!(&input[16..24], &packed_state.to_le_bytes());
        assert_eq!(&input[24..32], &sequence.to_le_bytes());
        assert!(input[32..].iter().all(|byte| *byte == 0));

        let presented = events[2].encode();
        assert_eq!(
            &presented[16..24],
            &u64::from(0x1122_3344_u32).to_le_bytes()
        );
        assert_eq!(&presented[24..32], &0x3132_3334_3536_3738_u64.to_le_bytes());
        let degraded = events[3].encode();
        assert_eq!(&degraded[16..24], &3_u64.to_le_bytes());
        assert!(degraded[24..].iter().all(|byte| *byte == 0));
        let focus = events[4].encode();
        assert_eq!(&focus[16..24], &2_u64.to_le_bytes());
        assert_eq!(&focus[24..32], &2_u64.to_le_bytes());
        assert_eq!(&focus[32..40], &0x4142_4344_4546_4748_u64.to_le_bytes());
        assert!(focus[40..].iter().all(|byte| *byte == 0));
        let cancelled = events[5].encode();
        assert_eq!(&cancelled[16..24], &0x5152_5354_u64.to_le_bytes());
        assert_eq!(&cancelled[24..32], &0x6162_6364_6566_6768_u64.to_le_bytes());
        assert!(cancelled[32..].iter().all(|byte| *byte == 0));
        let appearance = events[6].encode();
        assert_eq!(
            appearance[EVENT_OFFSET_KIND],
            UiServerEventKind::AppearanceChanged.raw()
        );
        assert_eq!(&appearance[16..24], &18_u64.to_le_bytes());
        assert_eq!(
            &appearance[24..32],
            &0x7172_7374_7576_7778_u64.to_le_bytes()
        );
        assert!(appearance[32..].iter().all(|byte| *byte == 0));
        let clock = events[7].encode();
        assert_eq!(
            clock[EVENT_OFFSET_KIND],
            UiServerEventKind::ClockChanged.raw()
        );
        assert_eq!(&clock[16..24], &0x8182_8384_8586_8788_u64.to_le_bytes());
        assert_eq!(&clock[24..32], &0x9192_9394_9596_9798_u64.to_le_bytes());
        assert!(clock[32..].iter().all(|byte| *byte == 0));
        let boot_notification = events[8].encode();
        assert_eq!(
            boot_notification[EVENT_OFFSET_KIND],
            UiServerEventKind::BootNotificationChanged.raw()
        );
        assert_eq!(&boot_notification[16..24], &0_u64.to_le_bytes());
        assert_eq!(
            &boot_notification[24..32],
            &0xa1a2_a3a4_a5a6_a7a8_u64.to_le_bytes()
        );
        assert!(boot_notification[32..].iter().all(|byte| *byte == 0));
        let system_ui = events[9].encode();
        assert_eq!(
            system_ui[EVENT_OFFSET_KIND],
            UiServerEventKind::SystemUiChanged.raw()
        );
        assert_eq!(&system_ui[16..24], &0x7903_0103_u64.to_le_bytes());
        assert_eq!(&system_ui[24..32], &0xb1b2_b3b4_b5b6_b7b8_u64.to_le_bytes());
        assert!(system_ui[32..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn system_ui_request_completion_wire_is_exact_and_statuses_are_canonical() {
        let session_id = 0x0102_0304_0506_0708;
        let identity =
            UiCompatibleActivityIdentity::new(0x1112_1314_1516_1718, 0x2122_2324_2526_2728)
                .unwrap();
        let accepted = UiServerEvent::system_ui_request_completed(
            session_id,
            UiSystemUiAction::ReserveCompatibleActivity,
            UiSystemUiRequestStatus::Accepted,
            0x3132_3334_3536_3738,
            0x4142_4344_4546_4748,
            Some(identity),
            Some(UiCompatibleActivityReservationOrigin::Home),
        )
        .unwrap();
        let completion = accepted.system_ui_request_completion().unwrap();
        assert_eq!(
            completion.action(),
            UiSystemUiAction::ReserveCompatibleActivity
        );
        assert_eq!(completion.status(), UiSystemUiRequestStatus::Accepted);
        assert_eq!(completion.request_id(), 0x3132_3334_3536_3738);
        assert_eq!(completion.revision(), 0x4142_4344_4546_4748);
        assert_eq!(completion.compatible_identity(), Some(identity));
        assert_eq!(
            completion.reservation_origin(),
            Some(UiCompatibleActivityReservationOrigin::Home)
        );

        let wire = accepted.encode();
        assert_eq!(&wire[0..4], b"BUE1");
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(&wire[4..7], &[6, 0, 11]);
        #[cfg(feature = "androidbox-el0-runtime0")]
        assert_eq!(&wire[4..7], &[7, 0, 11]);
        assert_eq!(wire[7], 0x57);
        assert_eq!(&wire[8..16], &session_id.to_le_bytes());
        assert_eq!(&wire[16..24], &0x3132_3334_3536_3738_u64.to_le_bytes());
        assert_eq!(&wire[24..32], &0x4142_4344_4546_4748_u64.to_le_bytes());
        assert_eq!(&wire[32..40], &identity.session_id().to_le_bytes());
        assert_eq!(&wire[40..48], &identity.package_generation().to_le_bytes());
        assert!(wire[48..].iter().all(|byte| *byte == 0));
        assert_eq!(UiServerEvent::decode(&wire), Ok(accepted));

        for (action, status, compatible_identity, origin) in [
            (
                UiSystemUiAction::Unlock,
                UiSystemUiRequestStatus::Conflict,
                None,
                None,
            ),
            (
                UiSystemUiAction::PresentCompatibleActivity,
                UiSystemUiRequestStatus::CaptureBusy,
                Some(identity),
                Some(UiCompatibleActivityReservationOrigin::Overview),
            ),
            (
                UiSystemUiAction::FinishCompatibleActivity,
                UiSystemUiRequestStatus::Accepted,
                Some(identity),
                None,
            ),
        ] {
            let event = UiServerEvent::system_ui_request_completed(
                session_id,
                action,
                status,
                9,
                13,
                compatible_identity,
                origin,
            )
            .unwrap();
            assert_eq!(UiServerEvent::decode(&event.encode()), Ok(event));
            assert_eq!(
                event.system_ui_request_completion().unwrap().status(),
                status
            );
        }
        for (raw, status) in [
            (1, UiSystemUiRequestStatus::Accepted),
            (2, UiSystemUiRequestStatus::Conflict),
            (3, UiSystemUiRequestStatus::CaptureBusy),
        ] {
            assert_eq!(UiSystemUiRequestStatus::from_raw(raw), Some(status));
            assert_eq!(status.raw(), raw);
        }
        assert_eq!(UiSystemUiRequestStatus::from_raw(0), None);
        assert_eq!(UiSystemUiRequestStatus::from_raw(4), None);
        for (raw, origin) in [
            (1, UiCompatibleActivityReservationOrigin::Home),
            (2, UiCompatibleActivityReservationOrigin::Overview),
        ] {
            assert_eq!(
                UiCompatibleActivityReservationOrigin::from_raw(raw),
                Some(origin)
            );
            assert_eq!(origin.raw(), raw);
        }
        assert_eq!(UiCompatibleActivityReservationOrigin::from_raw(0), None);
        assert_eq!(UiCompatibleActivityReservationOrigin::from_raw(3), None);
        assert_eq!(
            UiServerEvent::system_ui_request_completed(
                session_id,
                UiSystemUiAction::ReserveCompatibleActivity,
                UiSystemUiRequestStatus::Conflict,
                0,
                1,
                None,
                None,
            ),
            Err(UiServerEventError::ZeroSystemUiRequestId)
        );
        assert_eq!(
            UiServerEvent::system_ui_request_completed(
                session_id,
                UiSystemUiAction::ReserveCompatibleActivity,
                UiSystemUiRequestStatus::Conflict,
                1,
                0,
                None,
                None,
            ),
            Err(UiServerEventError::ZeroSystemUiRequestRevision)
        );
        for (compatible_identity, origin) in [
            (Some(identity), None),
            (None, Some(UiCompatibleActivityReservationOrigin::Home)),
        ] {
            assert_eq!(
                UiServerEvent::system_ui_request_completed(
                    session_id,
                    UiSystemUiAction::ReserveCompatibleActivity,
                    UiSystemUiRequestStatus::Conflict,
                    1,
                    1,
                    compatible_identity,
                    origin,
                ),
                Err(UiServerEventError::NonCanonicalSystemUiRequestCompletion)
            );
        }

        let mut invalid_action = wire;
        invalid_action[EVENT_OFFSET_HEADER_RESERVED] &= !SYSTEM_UI_COMPLETION_ACTION_MASK;
        assert_eq!(
            UiServerEvent::decode(&invalid_action),
            Err(UiServerEventError::InvalidSystemUiAction)
        );
        let mut invalid_status = wire;
        invalid_status[EVENT_OFFSET_HEADER_RESERVED] &= !SYSTEM_UI_COMPLETION_STATUS_MASK;
        assert_eq!(
            UiServerEvent::decode(&invalid_status),
            Err(UiServerEventError::InvalidSystemUiRequestStatus)
        );
        let mut invalid_origin = wire;
        invalid_origin[EVENT_OFFSET_HEADER_RESERVED] |= SYSTEM_UI_COMPLETION_ORIGIN_MASK;
        assert_eq!(
            UiServerEvent::decode(&invalid_origin),
            Err(UiServerEventError::InvalidCompatibleActivityReservationOrigin)
        );
        let mut partial_identity = wire;
        partial_identity[EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_ARGUMENT_2 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&partial_identity),
            Err(UiServerEventError::NonCanonicalSystemUiRequestCompletion)
        );
    }

    #[test]
    fn server_event_constructors_reject_zero_and_noncanonical_identifiers() {
        assert_eq!(
            UiServerEvent::ready(0),
            Err(UiServerEventError::ZeroSessionId)
        );
        let sample = InputSample::try_new(1, 0, 0, false).unwrap();
        assert_eq!(
            UiServerEvent::input_sample(0, sample),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::input(1, 1 << 40, 1),
            Err(UiServerEventError::InvalidInput(
                InputSampleError::NonCanonicalState
            ))
        );
        assert_eq!(
            UiServerEvent::input(1, 0, 0),
            Err(UiServerEventError::InvalidInput(
                InputSampleError::ZeroSequence
            ))
        );
        assert_eq!(
            UiServerEvent::presented(1, 0, 1),
            Err(UiServerEventError::ZeroFrameId)
        );
        assert_eq!(
            UiServerEvent::presented(1, 1, 0),
            Err(UiServerEventError::ZeroCommit)
        );
        assert_eq!(
            UiServerEvent::degraded(0, UiServerDegradedReason::PeerClosed),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::present_cancelled(1, 0, 1),
            Err(UiServerEventError::ZeroFrameId)
        );
        assert_eq!(
            UiServerEvent::present_cancelled(1, 1, 0),
            Err(UiServerEventError::ZeroFocusGeneration)
        );
        assert_eq!(
            UiServerEvent::appearance_changed(1, UiAppearance::default(), 0),
            Err(UiServerEventError::ZeroAppearanceRevision)
        );
        assert_eq!(
            UiServerEvent::appearance_changed(0, UiAppearance::default(), 1),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::clock_changed(1, 0, 0),
            Err(UiServerEventError::ZeroClockRevision)
        );
        assert_eq!(
            UiServerEvent::clock_changed(0, 0, 1),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::boot_notification_changed(1, true, 0),
            Err(UiServerEventError::ZeroBootNotificationRevision)
        );
        assert_eq!(
            UiServerEvent::boot_notification_changed(0, true, 1),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(
                1,
                UiSystemUiMode::Foreground,
                Some(ShellAppId::Phone),
                true,
                481,
                1,
            ),
            Err(UiServerEventError::InvalidSystemUiState(
                UiSystemUiStateError::NavRevealOutOfRange
            ))
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(
                1,
                UiSystemUiMode::Foreground,
                Some(ShellAppId::Phone),
                true,
                479,
                1,
            ),
            Err(UiServerEventError::InvalidSystemUiState(
                UiSystemUiStateError::NavRevealNotQuantized
            ))
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(
                1,
                UiSystemUiMode::Foreground,
                Some(ShellAppId::Phone),
                false,
                8,
                1,
            ),
            Err(UiServerEventError::InvalidSystemUiState(
                UiSystemUiStateError::NavRevealRequiresPress
            ))
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(1, UiSystemUiMode::Foreground, None, false, 0, 1,),
            Err(UiServerEventError::InvalidSystemUiState(
                UiSystemUiStateError::ForegroundRequiresRecentApp
            ))
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(1, UiSystemUiMode::Locked, None, false, 0, 0,),
            Err(UiServerEventError::ZeroSystemUiRevision)
        );
        assert_eq!(
            UiServerEvent::system_ui_changed(0, UiSystemUiMode::Locked, None, false, 0, 1,),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(
            UiServerEvent::clock_changed(1, 0, 1),
            Ok(UiServerEvent {
                session_id: 1,
                payload: UiServerEventPayload::ClockChanged {
                    unix_seconds: 0,
                    revision: 1,
                },
            })
        );
    }

    #[test]
    fn server_event_decoder_rejects_lengths_header_and_zero_session() {
        let canonical = UiServerEvent::ready(7).unwrap().encode();
        assert_eq!(
            UiServerEvent::decode(&canonical[..UI_SERVER_EVENT_WIRE_SIZE - 1]),
            Err(UiServerEventError::InvalidWireLength)
        );
        let mut oversized = [0; UI_SERVER_EVENT_WIRE_SIZE + 1];
        oversized[..UI_SERVER_EVENT_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            UiServerEvent::decode(&oversized),
            Err(UiServerEventError::InvalidWireLength)
        );
        for (offset, expected) in [
            (0, UiServerEventError::InvalidMagic),
            (4, UiServerEventError::InvalidVersion),
            (6, UiServerEventError::InvalidKind),
            (7, UiServerEventError::NonZeroHeaderReserved),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(
                UiServerEvent::decode(&wire),
                Err(expected),
                "offset {offset}"
            );
        }
        let mut version_three = canonical;
        version_three[EVENT_OFFSET_VERSION..EVENT_OFFSET_VERSION + 2]
            .copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&version_three),
            Err(UiServerEventError::InvalidVersion)
        );
        let mut zero_session = canonical;
        zero_session[EVENT_OFFSET_SESSION_ID..EVENT_OFFSET_SESSION_ID + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_session),
            Err(UiServerEventError::ZeroSessionId)
        );
        assert_eq!(UiServerEventKind::from_raw(0), None);
        assert_eq!(
            UiServerEventKind::from_raw(5),
            Some(UiServerEventKind::FocusChanged)
        );
        assert_eq!(
            UiServerEventKind::from_raw(6),
            Some(UiServerEventKind::PresentCancelled)
        );
        assert_eq!(
            UiServerEventKind::from_raw(7),
            Some(UiServerEventKind::AppearanceChanged)
        );
        assert_eq!(
            UiServerEventKind::from_raw(8),
            Some(UiServerEventKind::ClockChanged)
        );
        assert_eq!(
            UiServerEventKind::from_raw(9),
            Some(UiServerEventKind::BootNotificationChanged)
        );
        assert_eq!(
            UiServerEventKind::from_raw(10),
            Some(UiServerEventKind::SystemUiChanged)
        );
        assert_eq!(
            UiServerEventKind::from_raw(11),
            Some(UiServerEventKind::SystemUiRequestCompleted)
        );
        assert_eq!(UiServerEventKind::from_raw(12), None);
    }

    #[test]
    fn server_event_decoder_rejects_every_nonzero_body_reserved_byte() {
        let canonical = input_event(1, 1).encode();
        for offset in EVENT_OFFSET_BODY_RESERVED..UI_SERVER_EVENT_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&wire),
                Err(UiServerEventError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn ready_event_rejects_cross_kind_payload_fields() {
        let canonical = UiServerEvent::ready(1).unwrap().encode();
        for offset in [
            EVENT_OFFSET_ARGUMENT_0,
            EVENT_OFFSET_ARGUMENT_1,
            EVENT_OFFSET_ARGUMENT_2,
        ] {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&wire),
                Err(UiServerEventError::UnexpectedPayload)
            );
        }
    }

    #[test]
    fn input_event_decoder_reuses_the_canonical_input_codec() {
        let canonical = input_event(1, 1).encode();
        let mut noncanonical = canonical;
        noncanonical[EVENT_OFFSET_ARGUMENT_0 + 5] = 1;
        assert_eq!(
            UiServerEvent::decode(&noncanonical),
            Err(UiServerEventError::InvalidInput(
                InputSampleError::NonCanonicalState
            ))
        );
        let mut out_of_bounds = canonical;
        out_of_bounds[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&u64::from(SHELL_WIDTH).to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&out_of_bounds),
            Err(UiServerEventError::InvalidInput(
                InputSampleError::CoordinateOutOfBounds
            ))
        );
        let mut zero_sequence = canonical;
        zero_sequence[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_sequence),
            Err(UiServerEventError::InvalidInput(
                InputSampleError::ZeroSequence
            ))
        );
    }

    #[test]
    fn presented_event_decoder_rejects_wide_or_zero_counters() {
        let canonical = UiServerEvent::presented(1, 1, 1).unwrap().encode();
        let mut wide_frame = canonical;
        wide_frame[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&(u64::from(u32::MAX) + 1).to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&wide_frame),
            Err(UiServerEventError::FrameIdOutOfRange)
        );
        let mut zero_frame = canonical;
        zero_frame[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_frame),
            Err(UiServerEventError::ZeroFrameId)
        );
        let mut zero_commit = canonical;
        zero_commit[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_commit),
            Err(UiServerEventError::ZeroCommit)
        );
    }

    #[test]
    fn present_cancelled_decoder_rejects_wide_zero_and_cross_kind_fields() {
        let canonical = UiServerEvent::present_cancelled(1, 7, 9).unwrap().encode();
        assert_eq!(
            UiServerEvent::decode(&canonical),
            UiServerEvent::present_cancelled(1, 7, 9)
        );
        let mut wide_frame = canonical;
        wide_frame[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&(u64::from(u32::MAX) + 1).to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&wide_frame),
            Err(UiServerEventError::FrameIdOutOfRange)
        );
        let mut zero_frame = canonical;
        zero_frame[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_frame),
            Err(UiServerEventError::ZeroFrameId)
        );
        let mut zero_generation = canonical;
        zero_generation[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_generation),
            Err(UiServerEventError::ZeroFocusGeneration)
        );
        for offset in EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_BODY_RESERVED {
            let mut cross_kind = canonical;
            cross_kind[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&cross_kind),
                Err(UiServerEventError::UnexpectedPayload),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn degraded_event_rejects_unknown_reason_and_cross_kind_payload() {
        for (raw, reason) in [
            UiServerDegradedReason::PeerClosed,
            UiServerDegradedReason::ServerExit,
            UiServerDegradedReason::KernelDegraded,
            UiServerDegradedReason::ProtocolViolation,
        ]
        .into_iter()
        .enumerate()
        {
            let raw = raw as u64 + 1;
            assert_eq!(u64::from(reason.raw()), raw);
            assert_eq!(UiServerDegradedReason::from_raw(raw), Some(reason));
            let event = UiServerEvent::degraded(1, reason).unwrap();
            assert_eq!(UiServerEvent::decode(&event.encode()), Ok(event));
        }
        let canonical = UiServerEvent::degraded(1, UiServerDegradedReason::ProtocolViolation)
            .unwrap()
            .encode();
        for raw in [0_u64, 5, 0x101] {
            let mut wire = canonical;
            wire[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&wire),
                Err(UiServerEventError::InvalidDegradedReason)
            );
        }
        let mut cross_kind = canonical;
        cross_kind[EVENT_OFFSET_ARGUMENT_1] = 1;
        assert_eq!(
            UiServerEvent::decode(&cross_kind),
            Err(UiServerEventError::UnexpectedPayload)
        );
        assert_eq!(UiServerDegradedReason::from_raw(0), None);
        assert_eq!(UiServerDegradedReason::from_raw(5), None);
    }

    #[test]
    fn focus_changed_event_enforces_client_app_and_generation_cross_fields() {
        let launcher = UiServerEvent::focus_changed(1, UiClientId::Launcher, None, 1).unwrap();
        let app = UiServerEvent::focus_changed(1, UiClientId::App, Some(ShellAppId::Settings), 2)
            .unwrap();
        assert_eq!(UiServerEvent::decode(&launcher.encode()), Ok(launcher));
        assert_eq!(UiServerEvent::decode(&app.encode()), Ok(app));
        assert_eq!(
            UiServerEvent::focus_changed(1, UiClientId::Launcher, Some(ShellAppId::Phone), 1),
            Err(UiServerEventError::LauncherMustNotSpecifyApp)
        );
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(
            UiServerEvent::focus_changed(1, UiClientId::App, None, 1),
            Err(UiServerEventError::AppRequiresShellApp)
        );
        #[cfg(feature = "androidbox-el0-runtime0")]
        {
            let compatible_app = UiServerEvent::focus_changed(1, UiClientId::App, None, 1).unwrap();
            assert_eq!(
                UiServerEvent::decode(&compatible_app.encode()),
                Ok(compatible_app)
            );
        }
        assert_eq!(
            UiServerEvent::focus_changed(1, UiClientId::Launcher, None, 0),
            Err(UiServerEventError::ZeroFocusGeneration)
        );

        let canonical = app.encode();
        let mut invalid_client = canonical;
        invalid_client[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&3_u64.to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&invalid_client),
            Err(UiServerEventError::InvalidClient)
        );
        let mut invalid_app = canonical;
        invalid_app[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8]
            .copy_from_slice(&4_u64.to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&invalid_app),
            Err(UiServerEventError::InvalidApp)
        );
        let mut zero_generation = canonical;
        zero_generation[EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_ARGUMENT_2 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_generation),
            Err(UiServerEventError::ZeroFocusGeneration)
        );
        let mut launcher_with_app = canonical;
        launcher_with_app[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&1_u64.to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&launcher_with_app),
            Err(UiServerEventError::LauncherMustNotSpecifyApp)
        );
        let mut app_without_app = canonical;
        app_without_app[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        #[cfg(not(feature = "androidbox-el0-runtime0"))]
        assert_eq!(
            UiServerEvent::decode(&app_without_app),
            Err(UiServerEventError::AppRequiresShellApp)
        );
        #[cfg(feature = "androidbox-el0-runtime0")]
        assert_eq!(
            UiServerEvent::decode(&app_without_app),
            UiServerEvent::focus_changed(1, UiClientId::App, None, 2)
        );
    }

    #[cfg(not(feature = "androidbox-el0-runtime0"))]
    #[test]
    fn bue_v6_parent_profile_rejects_app_focus_without_a_shell_app() {
        assert_eq!(UI_SERVER_EVENT_VERSION, 6);
        assert_eq!(UI_CLIENT_CONTROL_VERSION, 8);
        assert_eq!(
            UiServerEvent::focus_changed(5, UiClientId::App, None, 1),
            Err(UiServerEventError::AppRequiresShellApp)
        );
        assert_eq!(
            UiClientControl::set_focus(UiClientId::App, None, 1),
            Err(UiClientControlError::AppRequiresShellApp)
        );

        let mut wire = UiServerEvent::focus_changed(5, UiClientId::App, Some(ShellAppId::Phone), 1)
            .unwrap()
            .encode();
        wire[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&wire),
            Err(UiServerEventError::AppRequiresShellApp)
        );
    }

    #[cfg(feature = "androidbox-el0-runtime0")]
    #[test]
    fn bue_v7_compatible_app_focus_requires_published_stable_foreground_state() {
        assert_eq!(UI_SERVER_EVENT_VERSION, 7);
        assert_eq!(UI_CLIENT_CONTROL_VERSION, 8);

        let session_id = 41;
        let identity = UiCompatibleActivityIdentity::new(17, 23).unwrap();
        let recent = Some(UiRecentIdentity::CompatibleAndroid(identity));
        let focus = UiServerEvent::focus_changed(session_id, UiClientId::App, None, 1).unwrap();
        let wire = focus.encode();
        assert_eq!(&wire[0..8], &[b'B', b'U', b'E', b'1', 7, 0, 5, 0]);
        assert_eq!(
            &wire[16..24],
            &u64::from(UiClientId::App.raw()).to_le_bytes()
        );
        assert_eq!(&wire[24..32], &0_u64.to_le_bytes());
        assert_eq!(&wire[32..40], &1_u64.to_le_bytes());
        assert_eq!(UiServerEvent::decode(&wire), Ok(focus));

        let mut tracker = UiServerEventTracker::new();
        tracker
            .accept(UiServerEvent::ready(session_id).unwrap())
            .unwrap();
        let ready = tracker;
        assert_eq!(
            tracker.accept(focus),
            Err(UiServerTrackerError::CompatibleAppFocusRequiresForeground)
        );
        assert_eq!(tracker, ready);

        tracker
            .accept(
                UiServerEvent::system_ui_changed_with_recent(
                    session_id,
                    UiSystemUiMode::Home,
                    recent,
                    false,
                    0,
                    1,
                )
                .unwrap(),
            )
            .unwrap();
        let home = tracker;
        assert_eq!(
            tracker.accept(focus),
            Err(UiServerTrackerError::CompatibleAppFocusRequiresForeground)
        );
        assert_eq!(tracker, home);

        tracker
            .accept(
                UiServerEvent::system_ui_changed(
                    session_id,
                    UiSystemUiMode::Foreground,
                    Some(ShellAppId::Settings),
                    false,
                    0,
                    2,
                )
                .unwrap(),
            )
            .unwrap();
        let shell_foreground = tracker;
        assert_eq!(
            tracker.accept(focus),
            Err(UiServerTrackerError::CompatibleAppFocusRequiresForeground)
        );
        assert_eq!(tracker, shell_foreground);

        tracker
            .accept(
                UiServerEvent::system_ui_changed_with_recent(
                    session_id,
                    UiSystemUiMode::Foreground,
                    recent,
                    true,
                    0,
                    3,
                )
                .unwrap(),
            )
            .unwrap();
        let navigation_in_progress = tracker;
        assert_eq!(
            tracker.accept(focus),
            Err(UiServerTrackerError::CompatibleAppFocusRequiresForeground)
        );
        assert_eq!(tracker, navigation_in_progress);

        tracker
            .accept(
                UiServerEvent::system_ui_changed_with_recent(
                    session_id,
                    UiSystemUiMode::Foreground,
                    recent,
                    false,
                    0,
                    4,
                )
                .unwrap(),
            )
            .unwrap();
        tracker.accept(focus).unwrap();
        assert_eq!(tracker.last_focus_generation(), Some(1));
        assert_eq!(tracker.active_client(), Some(UiClientId::App));
        assert_eq!(tracker.active_app(), None);
        assert_eq!(tracker.recent(), recent);

        // EL0Runtime-0 changes only the server event. Client-to-server BUC
        // remains v8 and still grants no generic App focus/system-UI control.
        assert_eq!(
            UiClientControl::set_focus(UiClientId::App, None, 1),
            Err(UiClientControlError::AppRequiresShellApp)
        );
        let mut system_ui = UiSystemUiSession::new();
        assert_eq!(
            system_ui.apply_client_recent_action(
                UiClientId::App,
                UiSystemUiAction::Unlock,
                None,
                1,
                1,
            ),
            Err(UiSystemUiSessionError::LauncherOnly)
        );
    }

    #[test]
    fn appearance_changed_event_rejects_unknown_flags_zero_revision_and_hidden_payload() {
        let appearance =
            UiAppearance::new(false, true).with_software_dimming(UiSoftwareDimming::Strong);
        let canonical = UiServerEvent::appearance_changed(7, appearance, 9).unwrap();
        assert_eq!(UiServerEvent::decode(&canonical.encode()), Ok(canonical));
        assert_eq!(
            canonical.payload(),
            UiServerEventPayload::AppearanceChanged {
                appearance,
                revision: 9,
            }
        );

        for raw in 5_u64..=7 {
            let mut invalid_flags = canonical.encode();
            invalid_flags[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&(raw << 2).to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&invalid_flags),
                Err(UiServerEventError::InvalidAppearance(
                    UiAppearanceError::InvalidSoftwareDimming
                ))
            );
        }
        for flags in [1_u64 << 5, 1 << 63, u64::MAX] {
            let mut invalid_flags = canonical.encode();
            invalid_flags[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&flags.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&invalid_flags),
                Err(UiServerEventError::InvalidAppearance(
                    UiAppearanceError::UnknownFlags
                ))
            );
        }
        let mut zero_revision = canonical.encode();
        zero_revision[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_revision),
            Err(UiServerEventError::ZeroAppearanceRevision)
        );
        for offset in EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_BODY_RESERVED {
            let mut cross_kind = canonical.encode();
            cross_kind[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&cross_kind),
                Err(UiServerEventError::UnexpectedPayload),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn clock_changed_event_accepts_epoch_zero_and_rejects_zero_revision_and_hidden_payload() {
        let epoch = UiServerEvent::clock_changed(7, 0, 1).unwrap();
        assert_eq!(UiServerEvent::decode(&epoch.encode()), Ok(epoch));
        assert_eq!(
            epoch.payload(),
            UiServerEventPayload::ClockChanged {
                unix_seconds: 0,
                revision: 1,
            }
        );

        let canonical = UiServerEvent::clock_changed(7, 0x0102_0304_0506_0708, u64::MAX).unwrap();
        assert_eq!(UiServerEvent::decode(&canonical.encode()), Ok(canonical));
        let mut zero_revision = canonical.encode();
        zero_revision[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_revision),
            Err(UiServerEventError::ZeroClockRevision)
        );
        for offset in EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_BODY_RESERVED {
            let mut cross_kind = canonical.encode();
            cross_kind[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&cross_kind),
                Err(UiServerEventError::UnexpectedPayload),
                "offset {offset}"
            );
        }
        for offset in EVENT_OFFSET_BODY_RESERVED..UI_SERVER_EVENT_WIRE_SIZE {
            let mut reserved = canonical.encode();
            reserved[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&reserved),
                Err(UiServerEventError::NonZeroBodyReserved),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn boot_notification_changed_event_is_boolean_revisioned_and_canonical() {
        for visible in [false, true] {
            let event = UiServerEvent::boot_notification_changed(7, visible, 9).unwrap();
            assert_eq!(UiServerEvent::decode(&event.encode()), Ok(event));
            assert_eq!(
                event.payload(),
                UiServerEventPayload::BootNotificationChanged {
                    visible,
                    revision: 9,
                }
            );
        }

        let canonical = UiServerEvent::boot_notification_changed(7, true, 9)
            .unwrap()
            .encode();
        for raw in [2_u64, u64::MAX] {
            let mut invalid_visible = canonical;
            invalid_visible[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&invalid_visible),
                Err(UiServerEventError::InvalidBootNotificationVisibility)
            );
        }
        let mut zero_revision = canonical;
        zero_revision[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_revision),
            Err(UiServerEventError::ZeroBootNotificationRevision)
        );
        for offset in EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_BODY_RESERVED {
            let mut cross_kind = canonical;
            cross_kind[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&cross_kind),
                Err(UiServerEventError::UnexpectedPayload),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn system_ui_changed_event_is_bounded_packed_and_canonical() {
        for (mode, recent_app, nav_pressed, nav_reveal_px) in [
            (UiSystemUiMode::Locked, None, false, 0),
            (
                UiSystemUiMode::Locked,
                None,
                true,
                UI_SYSTEM_UI_NAV_REVEAL_MAX_PX,
            ),
            (UiSystemUiMode::Home, Some(ShellAppId::Phone), false, 0),
            (
                UiSystemUiMode::Foreground,
                Some(ShellAppId::Messages),
                true,
                240,
            ),
            (
                UiSystemUiMode::Overview,
                Some(ShellAppId::Settings),
                false,
                0,
            ),
        ] {
            let event = UiServerEvent::system_ui_changed(
                23,
                mode,
                recent_app,
                nav_pressed,
                nav_reveal_px,
                7,
            )
            .unwrap();
            assert_eq!(UiServerEvent::decode(&event.encode()), Ok(event));
            assert_eq!(
                event.payload(),
                UiServerEventPayload::SystemUiChanged {
                    mode,
                    recent: recent_app.map(UiRecentIdentity::Shell),
                    nav_pressed,
                    nav_reveal_px,
                    revision: 7,
                }
            );
        }

        let canonical = UiServerEvent::system_ui_changed(
            23,
            UiSystemUiMode::Foreground,
            Some(ShellAppId::Settings),
            true,
            480,
            7,
        )
        .unwrap()
        .encode();
        assert_eq!(read_u64(&canonical, EVENT_OFFSET_ARGUMENT_0), 0x7903_0103);

        for raw in [0_u64, 5] {
            let mut invalid_mode = canonical;
            let packed = (read_u64(&invalid_mode, EVENT_OFFSET_ARGUMENT_0)
                & !SYSTEM_UI_STATE_MODE_MASK)
                | raw;
            invalid_mode[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&packed.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&invalid_mode),
                Err(UiServerEventError::InvalidSystemUiMode)
            );
        }

        let mut invalid_recent = canonical;
        invalid_recent[EVENT_OFFSET_ARGUMENT_0 + 2] = 4;
        assert_eq!(
            UiServerEvent::decode(&invalid_recent),
            Err(UiServerEventError::InvalidRecentApp)
        );

        for bit in [31_u32, 32, 63] {
            let mut unknown = canonical;
            let packed = read_u64(&unknown, EVENT_OFFSET_ARGUMENT_0) | (1_u64 << bit);
            unknown[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&packed.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&unknown),
                Err(UiServerEventError::NonCanonicalSystemUiState),
                "bit {bit}"
            );
        }

        for reveal_steps in [61_u64, 62, 63] {
            let mut excessive_reveal = canonical;
            let packed = (read_u64(&excessive_reveal, EVENT_OFFSET_ARGUMENT_0)
                & !SYSTEM_UI_STATE_NAV_REVEAL_MASK)
                | (reveal_steps << SYSTEM_UI_STATE_NAV_REVEAL_SHIFT);
            excessive_reveal[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
                .copy_from_slice(&packed.to_le_bytes());
            assert_eq!(
                UiServerEvent::decode(&excessive_reveal),
                Err(UiServerEventError::InvalidSystemUiState(
                    UiSystemUiStateError::NavRevealOutOfRange
                )),
                "steps {reveal_steps}"
            );
        }

        let mut reveal_without_press = canonical;
        let packed = read_u64(&reveal_without_press, EVENT_OFFSET_ARGUMENT_0)
            & !SYSTEM_UI_STATE_NAV_PRESSED_MASK;
        reveal_without_press[EVENT_OFFSET_ARGUMENT_0..EVENT_OFFSET_ARGUMENT_0 + 8]
            .copy_from_slice(&packed.to_le_bytes());
        assert_eq!(
            UiServerEvent::decode(&reveal_without_press),
            Err(UiServerEventError::InvalidSystemUiState(
                UiSystemUiStateError::NavRevealRequiresPress
            ))
        );

        let mut zero_revision = canonical;
        zero_revision[EVENT_OFFSET_ARGUMENT_1..EVENT_OFFSET_ARGUMENT_1 + 8].fill(0);
        assert_eq!(
            UiServerEvent::decode(&zero_revision),
            Err(UiServerEventError::ZeroSystemUiRevision)
        );
        for offset in EVENT_OFFSET_ARGUMENT_2..EVENT_OFFSET_BODY_RESERVED {
            let mut cross_kind = canonical;
            cross_kind[offset] = 1;
            assert_eq!(
                UiServerEvent::decode(&cross_kind),
                Err(UiServerEventError::NonCanonicalSystemUiState),
                "offset {offset}"
            );
        }

        let session = UiSystemUiSession::new();
        assert_eq!(
            UiServerEvent::system_ui_update(23, session.snapshot()),
            UiServerEvent::system_ui_changed(23, UiSystemUiMode::Locked, None, false, 0, 1,)
        );
    }

    #[test]
    fn server_event_tracker_requires_one_ready_and_one_session() {
        let mut tracker = UiServerEventTracker::new();
        let before = tracker;
        assert_eq!(
            tracker.accept(input_event(9, 1)),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        assert_eq!(tracker, before);
        tracker.accept(UiServerEvent::ready(9).unwrap()).unwrap();
        assert_eq!(tracker.session_id(), Some(9));
        let ready = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::ready(9).unwrap()),
            Err(UiServerTrackerError::ReadyAlreadyObserved)
        );
        assert_eq!(tracker, ready);
        assert_eq!(
            tracker.accept(input_event(10, 1)),
            Err(UiServerTrackerError::SessionMismatch)
        );
        assert_eq!(tracker, ready);
    }

    #[test]
    fn server_event_tracker_rejects_input_replay_gap_and_wrap() {
        let mut tracker = UiServerEventTracker::new();
        tracker.accept(UiServerEvent::ready(3).unwrap()).unwrap();
        tracker.accept(input_event(3, 1)).unwrap();
        assert_eq!(tracker.last_input_sequence(), Some(1));
        let after_one = tracker;
        assert_eq!(
            tracker.accept(input_event(3, 1)),
            Err(UiServerTrackerError::InputReplay)
        );
        assert_eq!(tracker, after_one);
        assert_eq!(
            tracker.accept(input_event(3, 3)),
            Err(UiServerTrackerError::InputGap)
        );
        assert_eq!(tracker, after_one);
        tracker.accept(input_event(3, 2)).unwrap();
        tracker.last_input_sequence = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(input_event(3, 1)),
            Err(UiServerTrackerError::InputSequenceExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_serializes_present_and_validates_both_ack_counters() {
        let mut tracker = UiServerEventTracker::new();
        assert_eq!(
            tracker.begin_present(1),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        tracker.accept(UiServerEvent::ready(4).unwrap()).unwrap();
        assert_eq!(
            tracker.begin_present(0),
            Err(UiServerTrackerError::FrameReplay)
        );
        assert_eq!(
            tracker.begin_present(2),
            Err(UiServerTrackerError::FrameGap)
        );
        tracker.begin_present(1).unwrap();
        assert_eq!(tracker.outstanding_frame_id(), Some(1));
        assert_eq!(
            tracker.begin_present(1),
            Err(UiServerTrackerError::PresentAlreadyOutstanding)
        );
        let outstanding = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 2, 1).unwrap()),
            Err(UiServerTrackerError::PresentedFrameGap)
        );
        assert_eq!(tracker, outstanding);
        tracker
            .accept(UiServerEvent::presented(4, 1, 7).unwrap())
            .unwrap();
        assert_eq!(tracker.last_frame_id(), Some(1));
        assert_eq!(tracker.last_commit(), Some(7));
        assert_eq!(tracker.outstanding_frame_id(), None);
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 1, 1).unwrap()),
            Err(UiServerTrackerError::PresentedFrameReplay)
        );
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 3, 2).unwrap()),
            Err(UiServerTrackerError::PresentedFrameGap)
        );
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 2, 2).unwrap()),
            Err(UiServerTrackerError::NoPresentOutstanding)
        );
        tracker.begin_present(2).unwrap();
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 2, 7).unwrap()),
            Err(UiServerTrackerError::CommitReplay)
        );
        tracker
            .accept(UiServerEvent::presented(4, 2, 42).unwrap())
            .unwrap();
        assert_eq!(tracker.last_commit(), Some(42));

        tracker.last_frame_id = Some(u32::MAX);
        assert_eq!(
            tracker.begin_present(1),
            Err(UiServerTrackerError::FrameIdExhausted)
        );
        tracker.last_frame_id = Some(2);
        tracker.last_commit = Some(u64::MAX);
        tracker.outstanding_frame_id = Some(3);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::presented(4, 3, u64::MAX).unwrap()),
            Err(UiServerTrackerError::CommitExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_requires_contiguous_focus_and_is_transactional() {
        let mut tracker = UiServerEventTracker::new();
        let focus_one = UiServerEvent::focus_changed(8, UiClientId::Launcher, None, 1).unwrap();
        assert_eq!(
            tracker.accept(focus_one),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        tracker.accept(UiServerEvent::ready(8).unwrap()).unwrap();
        tracker.accept(focus_one).unwrap();
        assert_eq!(tracker.last_focus_generation(), Some(1));
        assert_eq!(tracker.active_client(), Some(UiClientId::Launcher));
        assert_eq!(tracker.active_app(), None);
        let generation_one = tracker;
        assert_eq!(
            tracker.accept(focus_one),
            Err(UiServerTrackerError::FocusReplay)
        );
        assert_eq!(tracker, generation_one);
        assert_eq!(
            tracker.accept(
                UiServerEvent::focus_changed(8, UiClientId::App, Some(ShellAppId::Phone), 3)
                    .unwrap()
            ),
            Err(UiServerTrackerError::FocusGap)
        );
        assert_eq!(tracker, generation_one);
        tracker
            .accept(
                UiServerEvent::focus_changed(8, UiClientId::App, Some(ShellAppId::Phone), 2)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.last_focus_generation(), Some(2));
        assert_eq!(tracker.active_client(), Some(UiClientId::App));
        assert_eq!(tracker.active_app(), Some(ShellAppId::Phone));

        tracker.last_focus_generation = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(focus_one),
            Err(UiServerTrackerError::FocusGenerationExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_requires_contiguous_appearance_and_is_transactional() {
        let mut tracker = UiServerEventTracker::new();
        let dark =
            UiServerEvent::appearance_changed(13, UiAppearance::new(true, false), 1).unwrap();
        assert_eq!(
            tracker.accept(dark),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        tracker.accept(UiServerEvent::ready(13).unwrap()).unwrap();
        assert_eq!(tracker.last_appearance_revision(), None);
        assert_eq!(tracker.appearance(), None);
        tracker.accept(dark).unwrap();
        assert_eq!(tracker.last_appearance_revision(), Some(1));
        assert_eq!(tracker.appearance(), Some(UiAppearance::new(true, false)));

        let revision_one = tracker;
        assert_eq!(
            tracker.accept(dark),
            Err(UiServerTrackerError::AppearanceReplay)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(
                UiServerEvent::appearance_changed(13, UiAppearance::new(false, true), 3).unwrap()
            ),
            Err(UiServerTrackerError::AppearanceGap)
        );
        assert_eq!(tracker, revision_one);
        tracker
            .accept(
                UiServerEvent::appearance_changed(
                    13,
                    UiAppearance::new(false, false)
                        .with_software_dimming(UiSoftwareDimming::Strong),
                    2,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.last_appearance_revision(), Some(2));
        assert_eq!(
            tracker.appearance(),
            Some(UiAppearance::new(false, false).with_software_dimming(UiSoftwareDimming::Strong))
        );

        tracker.last_appearance_revision = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(dark),
            Err(UiServerTrackerError::AppearanceRevisionExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_requires_contiguous_clock_revisions_and_preserves_epoch_zero() {
        let mut tracker = UiServerEventTracker::new();
        let epoch = UiServerEvent::clock_changed(17, 0, 1).unwrap();
        assert_eq!(
            tracker.accept(epoch),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        assert_eq!(tracker.last_clock_revision(), None);
        assert_eq!(tracker.clock_unix_seconds(), None);

        tracker.accept(UiServerEvent::ready(17).unwrap()).unwrap();
        tracker.accept(epoch).unwrap();
        assert_eq!(tracker.last_clock_revision(), Some(1));
        assert_eq!(tracker.clock_unix_seconds(), Some(0));

        let revision_one = tracker;
        assert_eq!(
            tracker.accept(epoch),
            Err(UiServerTrackerError::ClockReplay)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(UiServerEvent::clock_changed(17, 20, 3).unwrap()),
            Err(UiServerTrackerError::ClockGap)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(UiServerEvent::clock_changed(18, 10, 2).unwrap()),
            Err(UiServerTrackerError::SessionMismatch)
        );
        assert_eq!(tracker, revision_one);

        tracker
            .accept(UiServerEvent::clock_changed(17, 10, 2).unwrap())
            .unwrap();
        assert_eq!(tracker.last_clock_revision(), Some(2));
        assert_eq!(tracker.clock_unix_seconds(), Some(10));

        tracker.last_clock_revision = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::clock_changed(17, 20, 1).unwrap()),
            Err(UiServerTrackerError::ClockRevisionExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_requires_contiguous_boot_notification_revisions() {
        let mut tracker = UiServerEventTracker::new();
        let visible = UiServerEvent::boot_notification_changed(19, true, 1).unwrap();
        assert_eq!(
            tracker.accept(visible),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        assert_eq!(tracker.last_boot_notification_revision(), None);
        assert_eq!(tracker.boot_notification_visible(), None);

        tracker.accept(UiServerEvent::ready(19).unwrap()).unwrap();
        tracker.accept(visible).unwrap();
        assert_eq!(tracker.last_boot_notification_revision(), Some(1));
        assert_eq!(tracker.boot_notification_visible(), Some(true));

        let revision_one = tracker;
        assert_eq!(
            tracker.accept(visible),
            Err(UiServerTrackerError::BootNotificationReplay)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(UiServerEvent::boot_notification_changed(19, false, 3).unwrap()),
            Err(UiServerTrackerError::BootNotificationGap)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(UiServerEvent::boot_notification_changed(20, false, 2).unwrap()),
            Err(UiServerTrackerError::SessionMismatch)
        );
        assert_eq!(tracker, revision_one);

        tracker
            .accept(UiServerEvent::boot_notification_changed(19, false, 2).unwrap())
            .unwrap();
        assert_eq!(tracker.last_boot_notification_revision(), Some(2));
        assert_eq!(tracker.boot_notification_visible(), Some(false));

        tracker.last_boot_notification_revision = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::boot_notification_changed(19, false, 1).unwrap()),
            Err(UiServerTrackerError::BootNotificationRevisionExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_requires_contiguous_system_ui_revisions_transactionally() {
        let mut tracker = UiServerEventTracker::new();
        let locked =
            UiServerEvent::system_ui_changed(29, UiSystemUiMode::Locked, None, false, 0, 1)
                .unwrap();
        assert_eq!(
            tracker.accept(locked),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        assert_eq!(tracker.last_system_ui_revision(), None);
        assert_eq!(tracker.system_ui_mode(), None);
        assert_eq!(tracker.recent_app(), None);
        assert_eq!(tracker.nav_pressed(), None);
        assert_eq!(tracker.nav_reveal_px(), None);

        tracker.accept(UiServerEvent::ready(29).unwrap()).unwrap();
        tracker.accept(locked).unwrap();
        assert_eq!(tracker.last_system_ui_revision(), Some(1));
        assert_eq!(tracker.system_ui_mode(), Some(UiSystemUiMode::Locked));
        assert_eq!(tracker.recent_app(), None);
        assert_eq!(tracker.nav_pressed(), Some(false));
        assert_eq!(tracker.nav_reveal_px(), Some(0));

        let revision_one = tracker;
        assert_eq!(
            tracker.accept(locked),
            Err(UiServerTrackerError::SystemUiReplay)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(
                UiServerEvent::system_ui_changed(29, UiSystemUiMode::Home, None, false, 0, 3,)
                    .unwrap()
            ),
            Err(UiServerTrackerError::SystemUiGap)
        );
        assert_eq!(tracker, revision_one);
        assert_eq!(
            tracker.accept(
                UiServerEvent::system_ui_changed(30, UiSystemUiMode::Home, None, false, 0, 2,)
                    .unwrap()
            ),
            Err(UiServerTrackerError::SessionMismatch)
        );
        assert_eq!(tracker, revision_one);

        tracker
            .accept(
                UiServerEvent::system_ui_changed(
                    29,
                    UiSystemUiMode::Home,
                    Some(ShellAppId::Phone),
                    false,
                    0,
                    2,
                )
                .unwrap(),
            )
            .unwrap();
        tracker
            .accept(
                UiServerEvent::system_ui_changed(
                    29,
                    UiSystemUiMode::Foreground,
                    Some(ShellAppId::Settings),
                    true,
                    240,
                    3,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.last_system_ui_revision(), Some(3));
        assert_eq!(tracker.system_ui_mode(), Some(UiSystemUiMode::Foreground));
        assert_eq!(tracker.recent_app(), Some(ShellAppId::Settings));
        assert_eq!(tracker.nav_pressed(), Some(true));
        assert_eq!(tracker.nav_reveal_px(), Some(240));

        // System state is deliberately published before the corresponding
        // focus transition, so Launcher/App cannot render stale shell mode.
        tracker
            .accept(
                UiServerEvent::focus_changed(29, UiClientId::App, Some(ShellAppId::Settings), 1)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.active_client(), Some(UiClientId::App));
        assert_eq!(tracker.active_app(), Some(ShellAppId::Settings));

        tracker.last_system_ui_revision = Some(u64::MAX);
        let exhausted = tracker;
        assert_eq!(
            tracker.accept(locked),
            Err(UiServerTrackerError::SystemUiRevisionExhausted)
        );
        assert_eq!(tracker, exhausted);
    }

    #[test]
    fn server_event_tracker_cancels_only_the_matching_frame_and_focus_generation() {
        let mut tracker = UiServerEventTracker::new();
        let cancel_one = UiServerEvent::present_cancelled(11, 1, 1).unwrap();
        assert_eq!(
            tracker.accept(cancel_one),
            Err(UiServerTrackerError::EventBeforeReady)
        );
        tracker.accept(UiServerEvent::ready(11).unwrap()).unwrap();
        tracker.begin_present(1).unwrap();
        let no_focus = tracker;
        assert_eq!(
            tracker.accept(cancel_one),
            Err(UiServerTrackerError::CancellationBeforeFocus)
        );
        assert_eq!(tracker, no_focus);

        tracker
            .accept(UiServerEvent::focus_changed(11, UiClientId::Launcher, None, 1).unwrap())
            .unwrap();
        let outstanding_one = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(12, 1, 1).unwrap()),
            Err(UiServerTrackerError::SessionMismatch)
        );
        assert_eq!(tracker, outstanding_one);
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(11, 2, 1).unwrap()),
            Err(UiServerTrackerError::PresentedFrameGap)
        );
        assert_eq!(tracker, outstanding_one);
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(11, 1, 2).unwrap()),
            Err(UiServerTrackerError::FocusGap)
        );
        assert_eq!(tracker, outstanding_one);

        tracker.accept(cancel_one).unwrap();
        assert_eq!(tracker.outstanding_frame_id(), None);
        assert_eq!(tracker.last_frame_id(), None);
        assert_eq!(tracker.last_commit(), None);
        assert_eq!(tracker.last_focus_generation(), Some(1));
        assert_eq!(tracker.active_client(), Some(UiClientId::Launcher));
        let cancelled = tracker;
        assert_eq!(
            tracker.accept(cancel_one),
            Err(UiServerTrackerError::NoPresentOutstanding)
        );
        assert_eq!(tracker, cancelled);

        tracker.begin_present(1).unwrap();
        tracker
            .accept(UiServerEvent::presented(11, 1, 10).unwrap())
            .unwrap();
        tracker.begin_present(2).unwrap();
        let outstanding_two = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(11, 1, 1).unwrap()),
            Err(UiServerTrackerError::PresentedFrameReplay)
        );
        assert_eq!(tracker, outstanding_two);
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(11, 3, 1).unwrap()),
            Err(UiServerTrackerError::PresentedFrameGap)
        );
        assert_eq!(tracker, outstanding_two);

        tracker
            .accept(
                UiServerEvent::focus_changed(11, UiClientId::App, Some(ShellAppId::Phone), 2)
                    .unwrap(),
            )
            .unwrap();
        let generation_two = tracker;
        assert_eq!(
            tracker.accept(UiServerEvent::present_cancelled(11, 2, 1).unwrap()),
            Err(UiServerTrackerError::FocusReplay)
        );
        assert_eq!(tracker, generation_two);
        tracker
            .accept(UiServerEvent::present_cancelled(11, 2, 2).unwrap())
            .unwrap();
        assert_eq!(tracker.outstanding_frame_id(), None);
        assert_eq!(tracker.last_frame_id(), Some(1));
        assert_eq!(tracker.last_commit(), Some(10));
        tracker.begin_present(2).unwrap();
    }

    #[test]
    fn server_event_tracker_treats_degraded_as_transactional_terminal_state() {
        let mut tracker = UiServerEventTracker::new();
        tracker.accept(UiServerEvent::ready(5).unwrap()).unwrap();
        tracker.begin_present(1).unwrap();
        tracker
            .accept(UiServerEvent::degraded(5, UiServerDegradedReason::KernelDegraded).unwrap())
            .unwrap();
        assert_eq!(tracker.outstanding_frame_id(), None);
        assert_eq!(
            tracker.degraded_reason(),
            Some(UiServerDegradedReason::KernelDegraded)
        );
        let terminal = tracker;
        assert_eq!(
            tracker.accept(input_event(5, 1)),
            Err(UiServerTrackerError::Terminal)
        );
        assert_eq!(tracker, terminal);
        assert_eq!(
            tracker.begin_present(1),
            Err(UiServerTrackerError::Terminal)
        );
        assert_eq!(tracker, terminal);
    }

    #[test]
    fn buffer_present_has_one_exact_sixty_four_byte_little_endian_wire() {
        assert_eq!(BUFFER_PRESENT_WIRE_SIZE, 64);
        let provisional =
            BufferPresent::try_new(0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0x0102_0304_0506_0708)
                .unwrap();
        assert_eq!(provisional.system_chrome_generation(), 0);
        let present = submitted_buffer_present(provisional);
        let wire = present.encode();
        assert_eq!(&wire[0..4], b"BUP1");
        assert_eq!(
            &wire[4..8],
            &[
                BUFFER_PRESENT_VERSION.to_le_bytes()[0],
                BUFFER_PRESENT_VERSION.to_le_bytes()[1],
                BUFFER_PRESENT_KIND,
                BUFFER_PRESENT_MODE_FULL,
            ]
        );
        assert_eq!(&wire[8..12], &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(&wire[12..16], &[0x88, 0x77, 0x66, 0x55]);
        assert_eq!(&wire[16..20], &[0xcc, 0xbb, 0xaa, 0x99]);
        assert_eq!(&wire[20..24], &[0; 4]);
        assert_eq!(&wire[24..32], &[8, 7, 6, 5, 4, 3, 2, 1]);
        let x = BUFFER_PRESENT_X.to_le_bytes();
        let y = BUFFER_PRESENT_Y.to_le_bytes();
        let width = BUFFER_PRESENT_WIDTH.to_le_bytes();
        let height = BUFFER_PRESENT_HEIGHT.to_le_bytes();
        assert_eq!(
            &wire[32..40],
            &[
                x[0], x[1], y[0], y[1], width[0], width[1], height[0], height[1],
            ]
        );
        assert_eq!(&wire[40..48], &[0; 8]);
        #[cfg(feature = "mobile-system-chrome0")]
        {
            assert_eq!(
                &wire[48..56],
                &[0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21]
            );
            assert!(wire[56..].iter().all(|byte| *byte == 0));
        }
        #[cfg(not(feature = "mobile-system-chrome0"))]
        assert!(wire[48..].iter().all(|byte| *byte == 0));
        assert_eq!(BufferPresent::decode(&wire), Ok(present));
        assert_eq!(present.client_frame_id(), 0x1122_3344);
        assert_eq!(present.frame_id(), 0x5566_7788);
        assert_eq!(present.global_frame_id(), 0x5566_7788);
        assert_eq!(present.focus_generation(), 0x99aa_bbcc);
        assert_eq!(present.buffer_generation(), 0x0102_0304_0506_0708);
        assert_eq!(present.system_ui_revision(), 0);
    }

    #[test]
    fn buffer_present_revision_bound_wire_round_trips_one_nonzero_system_ui_revision() {
        let present = submitted_buffer_present(
            BufferPresent::try_new_with_system_ui_revision(
                0x1122_3344,
                0x5566_7788,
                0x99aa_bbcc,
                0x0102_0304_0506_0708,
                0x1112_1314_1516_1718,
            )
            .unwrap(),
        );
        let wire = present.encode();
        assert_eq!(&wire[0..4], b"BUP1");
        assert_eq!(
            &wire[4..8],
            &[
                BUFFER_PRESENT_VERSION.to_le_bytes()[0],
                BUFFER_PRESENT_VERSION.to_le_bytes()[1],
                BUFFER_PRESENT_KIND,
                BUFFER_PRESENT_MODE_FULL,
            ]
        );
        assert_eq!(
            &wire[40..48],
            &[0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11]
        );
        #[cfg(feature = "mobile-system-chrome0")]
        assert_eq!(
            &wire[48..56],
            &[0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21]
        );
        #[cfg(not(feature = "mobile-system-chrome0"))]
        assert!(wire[48..].iter().all(|byte| *byte == 0));
        assert_eq!(BufferPresent::decode(&wire), Ok(present));
        assert_eq!(present.system_ui_revision(), 0x1112_1314_1516_1718);
    }

    #[test]
    fn buffer_present_construction_and_rewrites_preserve_all_bindings() {
        let client = BufferPresent::client(7, 11, 13).unwrap();
        assert_eq!(client.client_frame_id(), 7);
        assert_eq!(client.global_frame_id(), 7);
        assert_eq!(client.system_ui_revision(), 0);

        let client = submitted_buffer_present(
            BufferPresent::client_with_system_ui_revision(7, 11, 13, 23).unwrap(),
        );
        assert_eq!(client.client_frame_id(), 7);
        assert_eq!(client.global_frame_id(), 7);
        assert_eq!(client.system_ui_revision(), 23);
        let mapped = client.with_frame_id(7, 19).unwrap();
        assert_eq!(mapped.client_frame_id(), 7);
        assert_eq!(mapped.global_frame_id(), 19);
        assert_eq!(mapped.focus_generation(), 11);
        assert_eq!(mapped.buffer_generation(), 13);
        assert_eq!(mapped.system_ui_revision(), 23);
        let refocused = mapped.with_focus_generation(17).unwrap();
        assert_eq!(refocused.client_frame_id(), 7);
        assert_eq!(refocused.global_frame_id(), 19);
        assert_eq!(refocused.focus_generation(), 17);
        assert_eq!(refocused.buffer_generation(), 13);
        assert_eq!(refocused.system_ui_revision(), 23);
        assert_eq!(
            refocused.system_chrome_generation(),
            client.system_chrome_generation()
        );

        assert_eq!(
            client.with_frame_id(8, 19),
            Err(BufferPresentError::ClientFrameIdMismatch)
        );
        assert_eq!(
            client.with_frame_id(7, 6),
            Err(BufferPresentError::GlobalFramePrecedesClientFrame)
        );
        assert_eq!(
            client.with_frame_id(7, 0),
            Err(BufferPresentError::ZeroGlobalFrameId)
        );
        assert_eq!(
            client.with_focus_generation(0),
            Err(BufferPresentError::ZeroFocusGeneration)
        );
    }

    #[test]
    fn buffer_present_rejects_every_zero_and_cross_field_violation() {
        for (present, expected) in [
            (
                BufferPresent::try_new(0, 1, 1, 1),
                BufferPresentError::ZeroClientFrameId,
            ),
            (
                BufferPresent::try_new(1, 0, 1, 1),
                BufferPresentError::ZeroGlobalFrameId,
            ),
            (
                BufferPresent::try_new(2, 1, 1, 1),
                BufferPresentError::GlobalFramePrecedesClientFrame,
            ),
            (
                BufferPresent::try_new(1, 1, 0, 1),
                BufferPresentError::ZeroFocusGeneration,
            ),
            (
                BufferPresent::try_new(1, 1, 1, 0),
                BufferPresentError::ZeroBufferGeneration,
            ),
            (
                BufferPresent::try_new_with_system_ui_revision(1, 1, 1, 1, 0),
                BufferPresentError::ZeroSystemUiRevision,
            ),
        ] {
            assert_eq!(present, Err(expected));
        }
        assert_eq!(
            BufferPresent::client_with_system_ui_revision(1, 1, 1, 0),
            Err(BufferPresentError::ZeroSystemUiRevision)
        );

        let canonical = submitted_buffer_present(BufferPresent::client(2, 1, 1).unwrap()).encode();
        for (offset, width, expected) in [
            (
                BUFFER_PRESENT_OFFSET_CLIENT_FRAME_ID,
                4,
                BufferPresentError::ZeroClientFrameId,
            ),
            (
                BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID,
                4,
                BufferPresentError::ZeroGlobalFrameId,
            ),
            (
                BUFFER_PRESENT_OFFSET_FOCUS_GENERATION,
                4,
                BufferPresentError::ZeroFocusGeneration,
            ),
            (
                BUFFER_PRESENT_OFFSET_BUFFER_GENERATION,
                8,
                BufferPresentError::ZeroBufferGeneration,
            ),
        ] {
            let mut wire = canonical;
            wire[offset..offset + width].fill(0);
            assert_eq!(BufferPresent::decode(&wire), Err(expected));
        }
        let mut global_precedes_client = canonical;
        global_precedes_client
            [BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID..BUFFER_PRESENT_OFFSET_GLOBAL_FRAME_ID + 4]
            .copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            BufferPresent::decode(&global_precedes_client),
            Err(BufferPresentError::GlobalFramePrecedesClientFrame)
        );
    }

    #[test]
    fn buffer_present_decoder_rejects_lengths_headers_and_unknown_variants() {
        let canonical = submitted_buffer_present(BufferPresent::client(1, 1, 1).unwrap()).encode();
        assert_eq!(
            BufferPresent::decode(&canonical[..BUFFER_PRESENT_WIRE_SIZE - 1]),
            Err(BufferPresentError::InvalidWireLength)
        );
        let mut oversized = [0_u8; BUFFER_PRESENT_WIRE_SIZE + 1];
        oversized[..BUFFER_PRESENT_WIRE_SIZE].copy_from_slice(&canonical);
        assert_eq!(
            BufferPresent::decode(&oversized),
            Err(BufferPresentError::InvalidWireLength)
        );
        let mut version_one = canonical;
        version_one[BUFFER_PRESENT_OFFSET_VERSION..BUFFER_PRESENT_OFFSET_VERSION + 2]
            .copy_from_slice(&1_u16.to_le_bytes());
        assert_eq!(
            BufferPresent::decode(&version_one),
            Err(BufferPresentError::InvalidVersion)
        );
        for (offset, expected) in [
            (
                BUFFER_PRESENT_OFFSET_MAGIC,
                BufferPresentError::InvalidMagic,
            ),
            (
                BUFFER_PRESENT_OFFSET_VERSION,
                BufferPresentError::InvalidVersion,
            ),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            assert_eq!(BufferPresent::decode(&wire), Err(expected));
        }
        for unknown in [0, 2, u8::MAX] {
            let mut wire = canonical;
            wire[BUFFER_PRESENT_OFFSET_KIND] = unknown;
            assert_eq!(
                BufferPresent::decode(&wire),
                Err(BufferPresentError::InvalidKind)
            );
            let mut wire = canonical;
            wire[BUFFER_PRESENT_OFFSET_MODE] = unknown;
            assert_eq!(
                BufferPresent::decode(&wire),
                Err(BufferPresentError::InvalidMode)
            );
        }
    }

    #[test]
    fn buffer_present_decoder_rejects_every_reserved_and_padding_byte() {
        let canonical = submitted_buffer_present(
            BufferPresent::client_with_system_ui_revision(1, 1, 1, 7).unwrap(),
        )
        .encode();
        assert_eq!(
            read_u64(&canonical, BUFFER_PRESENT_OFFSET_SYSTEM_UI_REVISION),
            7
        );
        assert_eq!(
            BufferPresent::decode(&canonical)
                .unwrap()
                .system_ui_revision(),
            7
        );
        for offset in BUFFER_PRESENT_OFFSET_HEADER_RESERVED..BUFFER_PRESENT_HEADER_RESERVED_END {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                BufferPresent::decode(&wire),
                Err(BufferPresentError::NonZeroHeaderReserved),
                "offset {offset}"
            );
        }
        for offset in BUFFER_PRESENT_OFFSET_PADDING..BUFFER_PRESENT_WIRE_SIZE {
            let mut wire = canonical;
            wire[offset] = 1;
            assert_eq!(
                BufferPresent::decode(&wire),
                Err(BufferPresentError::NonZeroPadding),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn buffer_present_decoder_rejects_every_non_full_geometry_field() {
        let canonical = submitted_buffer_present(BufferPresent::client(1, 1, 1).unwrap()).encode();
        for offset in [
            BUFFER_PRESENT_OFFSET_X,
            BUFFER_PRESENT_OFFSET_Y,
            BUFFER_PRESENT_OFFSET_WIDTH,
            BUFFER_PRESENT_OFFSET_HEIGHT,
        ] {
            let mut wire = canonical;
            wire[offset] ^= 1;
            assert_eq!(
                BufferPresent::decode(&wire),
                Err(BufferPresentError::InvalidGeometry),
                "offset {offset}"
            );
        }
    }

    #[cfg(not(feature = "mobile-system-chrome0"))]
    #[test]
    fn buffer_present_parent_profile_remains_v2_full_surface() {
        assert_eq!(BUFFER_PRESENT_VERSION, 2);
        assert_eq!(
            (
                BUFFER_PRESENT_X,
                BUFFER_PRESENT_Y,
                BUFFER_PRESENT_WIDTH,
                BUFFER_PRESENT_HEIGHT,
            ),
            (0, 0, SURFACE_WIDTH, SURFACE_HEIGHT)
        );
        let wire = BufferPresent::client_with_system_ui_revision(1, 1, 1, 1)
            .unwrap()
            .encode();
        assert_eq!(&wire[4..6], &2_u16.to_le_bytes());
        assert_eq!(&wire[32..34], &0_u16.to_le_bytes());
        assert_eq!(&wire[34..36], &0_u16.to_le_bytes());
        assert_eq!(&wire[36..38], &SURFACE_WIDTH.to_le_bytes());
        assert_eq!(&wire[38..40], &SURFACE_HEIGHT.to_le_bytes());
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn buffer_present_child_profile_is_v3_content_viewport_only() {
        assert_eq!(BUFFER_PRESENT_VERSION, 3);
        assert_eq!(
            (
                BUFFER_PRESENT_X,
                BUFFER_PRESENT_Y,
                BUFFER_PRESENT_WIDTH,
                BUFFER_PRESENT_HEIGHT,
            ),
            (
                MOBILE_CONTENT_VIEWPORT_X,
                MOBILE_CONTENT_VIEWPORT_Y,
                MOBILE_CONTENT_VIEWPORT_WIDTH,
                MOBILE_CONTENT_VIEWPORT_HEIGHT,
            )
        );
        let provisional = BufferPresent::client_with_system_ui_revision(1, 1, 1, 1).unwrap();
        assert_eq!(provisional.system_chrome_generation(), 0);
        assert_eq!(
            BufferPresent::decode(&provisional.encode()),
            Ok(provisional),
            "v3 decoder preserves the provisional client form; syscall 62 rejects it"
        );
        assert_eq!(
            provisional.with_system_chrome_generation(0),
            Err(BufferPresentError::ZeroSystemChromeGeneration)
        );
        let canonical = provisional
            .with_system_chrome_generation(0x3132_3334_3536_3738)
            .unwrap()
            .encode();
        assert_eq!(&canonical[4..6], &3_u16.to_le_bytes());
        assert_eq!(&canonical[32..40], &[0, 0, 64, 0, 0xd0, 0x02, 0xa8, 0x05,]);
        assert_eq!(
            &canonical[48..56],
            &[0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31]
        );
        assert!(canonical[56..].iter().all(|byte| *byte == 0));
        assert_eq!(
            BufferPresent::decode(&canonical)
                .unwrap()
                .system_chrome_generation(),
            0x3132_3334_3536_3738
        );

        let mut legacy_full = canonical;
        legacy_full[34..36].copy_from_slice(&0_u16.to_le_bytes());
        legacy_full[38..40].copy_from_slice(&SURFACE_HEIGHT.to_le_bytes());
        assert_eq!(
            BufferPresent::decode(&legacy_full),
            Err(BufferPresentError::InvalidGeometry)
        );

        let mut legacy_version = canonical;
        legacy_version[4..6].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            BufferPresent::decode(&legacy_version),
            Err(BufferPresentError::InvalidVersion)
        );
    }

    #[test]
    fn wire_is_exactly_sixty_four_bytes_and_little_endian() {
        assert_eq!(
            PRESENT_HEADER_SIZE + MAX_SOLID_RECTS * SOLID_RECT_WIRE_SIZE,
            64
        );
        let frame = PresentFrame::full(
            0x1122_3344,
            0x0011_2233,
            &[rect(7, 0x0123, 8, 0x0045, 0x00aa_bbcc)],
        )
        .unwrap();
        let wire = frame.encode();
        assert_eq!(&wire[0..4], b"BUI1");
        assert_eq!(&wire[4..6], &[2, 0]);
        assert_eq!(&wire[6..12], &[1, 1, 1, 0, 1, 0]);
        assert_eq!(&wire[12..16], &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(&wire[16..20], &[0x33, 0x22, 0x11, 0]);
        assert_eq!(
            &wire[20..31],
            &[7, 8, 0x23, 0x01, 0x45, 0, 0xcc, 0xbb, 0xaa, 0, 0]
        );
        assert!(wire[31..].iter().all(|byte| *byte == 0));
        assert_eq!(PresentFrame::decode(&wire), Ok(frame));
    }

    #[test]
    fn focus_generation_uses_the_four_slot_tail_bytes_little_endian() {
        let untagged = PresentFrame::full(7, 0, &[rect(1, 2, 3, 4, 5)]).unwrap();
        assert_eq!(untagged.focus_generation(), 0);
        assert_eq!(
            untagged.with_focus_generation(0),
            Err(ProtocolError::ZeroFocusGeneration)
        );
        let frame = untagged.with_focus_generation(0x1122_3344).unwrap();
        assert_eq!(frame.focus_generation(), 0x1122_3344);
        let wire = frame.encode();
        assert_eq!(
            [
                wire[rect_offset(0) + 10],
                wire[rect_offset(1) + 10],
                wire[rect_offset(2) + 10],
                wire[rect_offset(3) + 10],
            ],
            [0x44, 0x33, 0x22, 0x11]
        );
        for index in 1..MAX_SOLID_RECTS {
            let offset = rect_offset(index);
            assert!(wire[offset..offset + 10].iter().all(|byte| *byte == 0));
        }
        assert_eq!(PresentFrame::decode(&wire), Ok(frame));

        for (index, expected) in [1_u32, 1 << 8, 1 << 16, 1 << 24].into_iter().enumerate() {
            let mut one_byte = untagged.encode();
            one_byte[rect_offset(index) + 10] = 1;
            let decoded = PresentFrame::decode(&one_byte).unwrap();
            assert_eq!(decoded.focus_generation(), expected);
            assert_eq!(decoded.encode(), one_byte);
        }
    }

    #[test]
    fn full_frame_may_clear_without_rects_and_always_damages_every_pixel() {
        let frame = PresentFrame::full(1, 0x0012_3456, &[]).unwrap();
        assert_eq!(frame.rect_count(), 0);
        assert_eq!(frame.damage_rect(), DamageRect::FULL);
        assert_eq!(PresentFrame::decode(&frame.encode()), Ok(frame));
    }

    #[test]
    fn damage_union_is_exact_and_preserves_rect_order() {
        let rects = [
            rect(10, 20, 30, 40, 0x0001_0203),
            rect(100, 200, 20, 50, 0x0004_0506),
            rect(20, 30, 5, 5, 0x0007_0809),
        ];
        let frame = PresentFrame::damage(2, &rects).unwrap();
        assert_eq!(frame.rects(), &rects);
        assert_eq!(
            frame.damage_rect(),
            DamageRect {
                x: 10,
                y: 20,
                width: 110,
                height: 230,
            }
        );
    }

    #[test]
    fn constructors_reject_bad_counts_frame_ids_and_colors() {
        let valid = rect(0, 0, 1, 1, 0);
        assert_eq!(
            PresentFrame::full(0, 0, &[]),
            Err(ProtocolError::ZeroFrameId)
        );
        assert_eq!(
            PresentFrame::damage(1, &[]),
            Err(ProtocolError::EmptyDamage)
        );
        assert_eq!(
            PresentFrame::full(1, 0, &[valid; 5]),
            Err(ProtocolError::TooManyRects)
        );
        assert_eq!(
            PresentFrame::full(1, 0xff00_0000, &[]),
            Err(ProtocolError::NonCanonicalColor)
        );
        assert_eq!(
            PresentFrame::full(1, 0, &[])
                .unwrap()
                .with_focus_generation(0),
            Err(ProtocolError::ZeroFocusGeneration)
        );
    }

    #[test]
    fn rectangles_reject_zero_noncanonical_and_out_of_bounds_geometry() {
        assert_eq!(
            SolidRect::try_new(0, 0, 0, 1, 0),
            Err(ProtocolError::ZeroRectExtent)
        );
        assert_eq!(
            SolidRect::try_new(0, 0, 1, 0, 0),
            Err(ProtocolError::ZeroRectExtent)
        );
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(
            SolidRect::try_new(200, 0, 9, 1, 0),
            Err(ProtocolError::RectOutOfBounds)
        );
        assert_eq!(
            SolidRect::try_new(0, SURFACE_HEIGHT - 1, 1, 2, 0),
            Err(ProtocolError::RectOutOfBounds)
        );
        assert_eq!(
            SolidRect::try_new(0, u16::MAX, 1, 2, 0),
            Err(ProtocolError::RectOutOfBounds)
        );
        assert_eq!(
            SolidRect::try_new(0, 0, 1, 1, 0xff00_0000),
            Err(ProtocolError::NonCanonicalColor)
        );
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert!(SolidRect::try_new(207, 367, 1, 1, 0x00ff_ffff).is_ok());
        #[cfg(feature = "mobile-ui-runtime")]
        assert!(SolidRect::try_new(u8::MAX, 1_599, 1, 1, 0x00ff_ffff).is_ok());
    }

    #[test]
    fn decoder_rejects_every_header_discriminator_and_reserved_byte() {
        let canonical = PresentFrame::full(1, 0, &[]).unwrap().encode();
        for (offset, expected) in [
            (0, ProtocolError::InvalidMagic),
            (4, ProtocolError::InvalidVersion),
            (6, ProtocolError::InvalidOpcode),
            (7, ProtocolError::InvalidMode),
            (8, ProtocolError::InvalidSurfaceId),
            (11, ProtocolError::NonZeroHeaderReserved),
            (12, ProtocolError::ZeroFrameId),
        ] {
            let mut wire = canonical;
            wire[offset] ^= 0x80;
            if offset == 12 {
                wire[offset] = 0;
            }
            assert_eq!(
                PresentFrame::decode(&wire),
                Err(expected),
                "offset {offset}"
            );
        }
        let mut count = canonical;
        count[OFFSET_RECT_COUNT] = 5;
        assert_eq!(
            PresentFrame::decode(&count),
            Err(ProtocolError::TooManyRects)
        );
    }

    #[test]
    fn decoder_accepts_generation_tails_but_rejects_nonzero_unused_geometry() {
        let canonical = PresentFrame::full(1, 0, &[rect(0, 0, 1, 1, 0)])
            .unwrap()
            .encode();
        for index in 1..MAX_SOLID_RECTS {
            let slot = rect_offset(index);
            for offset in slot..slot + 10 {
                let mut wire = canonical;
                wire[offset] = 1;
                assert_eq!(
                    PresentFrame::decode(&wire),
                    Err(ProtocolError::NonZeroUnusedRect),
                    "offset {offset}"
                );
            }
        }
    }

    #[test]
    fn decoder_revalidates_rect_geometry_and_color() {
        let canonical = PresentFrame::full(1, 0, &[rect(0, 0, 1, 1, 0)])
            .unwrap()
            .encode();
        let mut zero_width = canonical;
        zero_width[rect_offset(0) + 1] = 0;
        assert_eq!(
            PresentFrame::decode(&zero_width),
            Err(ProtocolError::ZeroRectExtent)
        );
        let mut out_of_bounds = canonical;
        #[cfg(not(feature = "mobile-ui-runtime"))]
        {
            out_of_bounds[rect_offset(0)] = 208;
        }
        #[cfg(feature = "mobile-ui-runtime")]
        out_of_bounds[rect_offset(0) + 2..rect_offset(0) + 4]
            .copy_from_slice(&SURFACE_HEIGHT.to_le_bytes());
        assert_eq!(
            PresentFrame::decode(&out_of_bounds),
            Err(ProtocolError::RectOutOfBounds)
        );
        let mut bad_color = canonical;
        bad_color[rect_offset(0) + 9] = 1;
        assert_eq!(
            PresentFrame::decode(&bad_color),
            Err(ProtocolError::NonCanonicalColor)
        );
    }

    #[test]
    fn first_frame_requires_full_mode_and_identifier_one() {
        let full_one = PresentFrame::full(1, 0, &[]).unwrap();
        assert_eq!(validate_sequence(None, &full_one), Ok(()));
        assert_eq!(
            validate_sequence(
                None,
                &PresentFrame::damage(1, &[rect(0, 0, 1, 1, 0)]).unwrap()
            ),
            Err(ProtocolError::FirstFrameMustBeFull)
        );
        assert_eq!(
            validate_sequence(None, &PresentFrame::full(2, 0, &[]).unwrap()),
            Err(ProtocolError::FirstFrameIdMustBeOne)
        );
    }

    #[test]
    fn sequence_rejects_replay_gap_and_exhaustion_without_wrap() {
        assert_eq!(
            validate_sequence(
                Some(1),
                &PresentFrame::damage(2, &[rect(0, 0, 1, 1, 0)]).unwrap()
            ),
            Ok(())
        );
        assert_eq!(
            validate_sequence(Some(2), &PresentFrame::full(2, 0, &[]).unwrap()),
            Err(ProtocolError::FrameReplay)
        );
        assert_eq!(
            validate_sequence(Some(2), &PresentFrame::full(4, 0, &[]).unwrap()),
            Err(ProtocolError::FrameGap)
        );
        assert_eq!(
            validate_sequence(
                Some(u32::MAX),
                &PresentFrame::full(u32::MAX, 0, &[]).unwrap()
            ),
            Err(ProtocolError::FrameIdExhausted)
        );
    }

    #[test]
    fn decoder_rejects_non_exact_wire_lengths() {
        let wire = PresentFrame::full(1, 0, &[]).unwrap().encode();
        assert_eq!(
            PresentFrame::decode(&wire[..PRESENT_WIRE_SIZE - 1]),
            Err(ProtocolError::InvalidWireLength)
        );
        let mut oversized = [0_u8; PRESENT_WIRE_SIZE + 1];
        oversized[..PRESENT_WIRE_SIZE].copy_from_slice(&wire);
        assert_eq!(
            PresentFrame::decode(&oversized),
            Err(ProtocolError::InvalidWireLength)
        );
    }

    #[test]
    fn shell_targets_use_exact_half_open_scanout_coordinates() {
        assert_eq!(
            shell_hit_test(ShellView::Home, PHONE_TARGET.x, PHONE_TARGET.y),
            Some(ShellTarget::App(ShellAppId::Phone))
        );
        let settings_right = SETTINGS_TARGET.x + SETTINGS_TARGET.width;
        let settings_bottom = SETTINGS_TARGET.y + SETTINGS_TARGET.height;
        assert_eq!(
            shell_hit_test(ShellView::Home, settings_right - 1, settings_bottom - 1),
            Some(ShellTarget::App(ShellAppId::Settings))
        );
        assert_eq!(
            shell_hit_test(ShellView::Home, settings_right, settings_bottom - 1),
            None
        );
        assert_eq!(
            shell_hit_test(
                ShellView::App(ShellAppId::Messages),
                HOME_TARGET.x,
                HOME_TARGET.y
            ),
            Some(ShellTarget::Home)
        );
    }

    #[test]
    fn shell_capture_requires_release_on_the_armed_target() {
        let mut shell = ShellController::new();
        assert_eq!(shell.observe(160, 342, true), Ok(None));
        assert_eq!(shell.observe(160, 250, false), Ok(None));
        assert_eq!(shell.view(), ShellView::Home);
        assert_eq!(shell.taps(), 0);
        assert_eq!(shell.reports(), 2);
        assert_eq!(shell.armed(), None);
        assert!(!shell.pointer_pressed());
    }

    #[test]
    fn shell_navigation_has_checked_monotonic_transition_ids() {
        let mut shell = ShellController::new();
        let settings_x = SETTINGS_TARGET.x + SETTINGS_TARGET.width / 2;
        let settings_y = SETTINGS_TARGET.y + SETTINGS_TARGET.height / 2;
        shell.observe(settings_x, settings_y, true).unwrap();
        let open = shell
            .observe(settings_x, settings_y, false)
            .unwrap()
            .unwrap();
        assert_eq!(open.from, ShellView::Home);
        assert_eq!(open.to, ShellView::App(ShellAppId::Settings));
        assert_eq!(open.transition_id, 1);
        let home_x = HOME_TARGET.x + HOME_TARGET.width / 2;
        let home_y = HOME_TARGET.y + HOME_TARGET.height / 2;
        shell.observe(home_x, home_y, true).unwrap();
        let home = shell.observe(home_x, home_y, false).unwrap().unwrap();
        assert_eq!(home.target, ShellTarget::Home);
        assert_eq!(home.to, ShellView::Home);
        assert_eq!(home.transition_id, 2);
        assert_eq!(shell.taps(), 2);
    }

    #[test]
    fn shell_counter_exhaustion_is_transactional() {
        let mut shell = ShellController {
            reports: u64::MAX,
            ..ShellController::new()
        };
        let before = shell;
        assert_eq!(
            shell.observe(160, 342, true),
            Err(ShellError::CounterExhausted)
        );
        assert_eq!(shell, before);
    }

    #[test]
    fn input_sample_register_codec_is_canonical() {
        let sample = InputSample::try_new(7, 319, 479, true).unwrap();
        let registers = sample.encode_registers();
        assert_eq!(
            InputSample::decode_registers(registers.0, registers.1),
            Ok(sample)
        );
        assert_eq!(registers.0, 319 | (479 << 16) | (1 << 32));
        assert_eq!(registers.1, 7);
        assert_eq!(
            InputSample::decode_registers(registers.0 | (1 << 40), registers.1),
            Err(InputSampleError::NonCanonicalState)
        );
    }

    #[test]
    fn input_sample_rejects_zero_sequence_and_out_of_bounds_coordinates() {
        assert_eq!(
            InputSample::try_new(0, 0, 0, false),
            Err(InputSampleError::ZeroSequence)
        );
        assert_eq!(
            InputSample::try_new(1, SHELL_WIDTH, 0, false),
            Err(InputSampleError::CoordinateOutOfBounds)
        );
        assert_eq!(
            InputSample::try_new(1, 0, SHELL_HEIGHT, false),
            Err(InputSampleError::CoordinateOutOfBounds)
        );
    }

    fn text_context(session_id: u64) -> TextInputContext {
        TextInputContext::try_new(WindowId::try_new(1, 7).unwrap(), session_id, 11).unwrap()
    }

    #[test]
    fn text_input_command_all_opcodes_round_trip_in_strict_64_byte_wire() {
        let context = text_context(3);
        let state = TextState::try_new(9, "a你", 1, 4).unwrap();
        let commands = [
            TextInputCommand::activate(1, context).unwrap(),
            TextInputCommand::state_ack(2, context, 4, state).unwrap(),
            TextInputCommand::deactivate(3, context).unwrap(),
        ];
        assert_eq!(TEXT_INPUT_COMMAND_WIRE_SIZE, 64);
        for command in commands {
            let wire = command.encode();
            assert_eq!(&wire[0..4], b"BTI1");
            assert_eq!(TextInputCommand::decode(&wire), Ok(command));
        }
        assert_eq!(commands[0].opcode(), TextInputCommandOpcode::Activate);
        assert_eq!(commands[1].opcode(), TextInputCommandOpcode::StateAck);
        assert_eq!(commands[2].opcode(), TextInputCommandOpcode::Deactivate);
        for (raw, opcode) in [
            (1, TextInputCommandOpcode::Activate),
            (2, TextInputCommandOpcode::StateAck),
            (3, TextInputCommandOpcode::Deactivate),
        ] {
            assert_eq!(TextInputCommandOpcode::from_raw(raw), Some(opcode));
        }
        assert_eq!(TextInputCommandOpcode::from_raw(0), None);
        assert_eq!(TextInputCommandOpcode::from_raw(4), None);
    }

    #[test]
    fn text_input_event_all_opcodes_round_trip_in_strict_64_byte_wire() {
        let context = text_context(3);
        let state = TextState::try_new(4, "a你", 1, 4).unwrap();
        let events = [
            TextInputEvent::activated(1, context, 1, 0).unwrap(),
            TextInputEvent::preedit(2, context, 1, '好').unwrap(),
            TextInputEvent::commit(3, context, 2, "🙂").unwrap(),
            TextInputEvent::delete_surrounding(4, context, 3, 1, 2).unwrap(),
            TextInputEvent::rendered(5, context, 3, state).unwrap(),
            TextInputEvent::deactivated(6, context, 4, 4).unwrap(),
        ];
        assert_eq!(TEXT_INPUT_EVENT_WIRE_SIZE, 64);
        for event in events {
            let wire = event.encode();
            assert_eq!(&wire[0..4], b"BTE1");
            assert_eq!(TextInputEvent::decode(&wire), Ok(event));
        }
        for (raw, opcode) in [
            (1, TextInputEventOpcode::Activated),
            (2, TextInputEventOpcode::Preedit),
            (3, TextInputEventOpcode::Commit),
            (4, TextInputEventOpcode::DeleteSurrounding),
            (5, TextInputEventOpcode::Rendered),
            (6, TextInputEventOpcode::Deactivated),
        ] {
            assert_eq!(TextInputEventOpcode::from_raw(raw), Some(opcode));
        }
        assert_eq!(TextInputEventOpcode::from_raw(0), None);
        assert_eq!(TextInputEventOpcode::from_raw(7), None);
    }

    #[test]
    fn text_state_enforces_utf8_capacity_and_selection_boundaries() {
        let state = TextState::try_new(2, "a你b", 1, 4).unwrap();
        assert_eq!(state.committed().as_str(), "a你b");
        assert_eq!(state.committed().len(), 5);
        assert_eq!(state.selection_start(), 1);
        assert_eq!(state.selection_end(), 4);
        assert_eq!(TextBytes8::try_from_str("12345678").unwrap().len(), 8);
        assert_eq!(
            TextBytes8::try_from_str("123456789"),
            Err(TextStateError::TextTooLong)
        );
        assert_eq!(
            TextState::try_new(1, "你", 1, 3),
            Err(TextStateError::InvalidSelection)
        );
        assert_eq!(
            TextState::try_new(1, "abc", 2, 1),
            Err(TextStateError::InvalidSelection)
        );
    }

    #[test]
    fn text_input_command_decoder_rejects_reserved_tail_utf8_and_hidden_payload() {
        let context = text_context(3);
        let state = TextState::try_new(1, "a", 1, 1).unwrap();
        let canonical = TextInputCommand::state_ack(2, context, 1, state)
            .unwrap()
            .encode();
        let mut wire = canonical;
        wire[7] = 1;
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::NonZeroHeaderReserved)
        );
        let mut wire = canonical;
        wire[55] = 1;
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::NonZeroBodyReserved)
        );
        let mut wire = canonical;
        wire[57] = 1;
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::InvalidTextState(
                TextStateError::NonCanonicalTail
            ))
        );
        let mut wire = canonical;
        wire[54] = 1;
        wire[56] = 0xff;
        wire[57..64].fill(0);
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::InvalidTextState(
                TextStateError::InvalidUtf8
            ))
        );
        let mut wire = TextInputCommand::activate(1, context).unwrap().encode();
        wire[48] = 1;
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::UnexpectedPayload)
        );
        let mut wire = canonical;
        wire[52] = 2;
        assert_eq!(
            TextInputCommand::decode(&wire),
            Err(TextInputCommandError::InvalidTextState(
                TextStateError::InvalidSelection
            ))
        );
    }

    #[test]
    fn text_input_event_decoder_rejects_noncanonical_opcode_payloads() {
        let context = text_context(3);
        let mut preedit = TextInputEvent::preedit(1, context, 1, 'a')
            .unwrap()
            .encode();
        preedit[54] = 2;
        preedit[57] = b'b';
        assert_eq!(
            TextInputEvent::decode(&preedit),
            Err(TextInputEventError::InvalidText(
                TextStateError::NotSingleScalar
            ))
        );
        let mut commit = TextInputEvent::commit(1, context, 1, "a").unwrap().encode();
        commit[54] = 0;
        commit[56] = 0;
        assert_eq!(
            TextInputEvent::decode(&commit),
            Err(TextInputEventError::EmptyCommit)
        );
        let mut delete = TextInputEvent::delete_surrounding(1, context, 1, 1, 0)
            .unwrap()
            .encode();
        delete[52] = 0;
        assert_eq!(
            TextInputEvent::decode(&delete),
            Err(TextInputEventError::EmptyDelete)
        );
        let mut activated = TextInputEvent::activated(1, context, 1, 0)
            .unwrap()
            .encode();
        activated[56] = b'x';
        assert_eq!(
            TextInputEvent::decode(&activated),
            Err(TextInputEventError::InvalidText(
                TextStateError::NonCanonicalTail
            ))
        );
        assert_eq!(
            TextInputEvent::commit(1, context, 1, ""),
            Err(TextInputEventError::EmptyCommit)
        );
        assert_eq!(
            TextInputEvent::delete_surrounding(1, context, 1, 0, 0),
            Err(TextInputEventError::EmptyDelete)
        );
    }

    #[test]
    fn text_editor_handles_single_scalar_preedit_commit_and_scalar_delete() {
        let context = text_context(3);
        let mut editor = TextEditor::new();
        editor
            .activate(TextState::try_new(0, "a你b", 1, 4).unwrap())
            .unwrap();
        editor
            .apply(TextInputEvent::preedit(1, context, 1, '好').unwrap())
            .unwrap();
        assert_eq!(editor.preedit(), Some('好'));
        assert_eq!(editor.state().committed().as_str(), "a你b");
        editor
            .apply(TextInputEvent::commit(2, context, 2, "🙂").unwrap())
            .unwrap();
        assert_eq!(editor.state().committed().as_str(), "a🙂b");
        assert_eq!(editor.state().selection_start(), 5);
        assert_eq!(editor.preedit(), None);
        editor
            .apply(TextInputEvent::delete_surrounding(3, context, 3, 1, 0).unwrap())
            .unwrap();
        assert_eq!(editor.state().committed().as_str(), "ab");
        assert_eq!(editor.state().selection_start(), 1);
        assert_eq!(editor.state().selection_end(), 1);
    }

    #[test]
    fn text_editor_capacity_revision_and_delete_failures_are_transactional() {
        let context = text_context(3);
        let mut editor = TextEditor::new();
        editor
            .activate(TextState::try_new(0, "12345678", 8, 8).unwrap())
            .unwrap();
        let before = editor;
        assert_eq!(
            editor.apply(TextInputEvent::commit(1, context, 1, "x").unwrap()),
            Err(TextEditorError::CapacityExceeded)
        );
        assert_eq!(editor, before);
        assert_eq!(
            editor.apply(TextInputEvent::preedit(1, context, 2, 'x').unwrap()),
            Err(TextEditorError::RevisionGap)
        );
        assert_eq!(editor, before);
        assert_eq!(
            editor.apply(TextInputEvent::delete_surrounding(1, context, 1, 9, 0).unwrap()),
            Err(TextEditorError::DeleteOutOfRange)
        );
        assert_eq!(editor, before);
    }

    #[test]
    fn text_editor_render_and_deactivation_checks_are_transactional() {
        let context = text_context(3);
        let mut editor = TextEditor::new();
        editor
            .apply(TextInputEvent::activated(1, context, 1, 0).unwrap())
            .unwrap();
        let before = editor;
        let wrong = TextState::try_new(0, "x", 1, 1).unwrap();
        assert_eq!(
            editor.apply(TextInputEvent::rendered(2, context, 2, wrong).unwrap()),
            Err(TextEditorError::StateMismatch)
        );
        assert_eq!(editor, before);
        editor
            .apply(TextInputEvent::deactivated(2, context, 2, 0).unwrap())
            .unwrap();
        assert!(!editor.is_active());
        let terminal = editor;
        assert_eq!(
            editor.apply(TextInputEvent::preedit(3, context, 1, 'x').unwrap()),
            Err(TextEditorError::Inactive)
        );
        assert_eq!(editor, terminal);
    }

    #[test]
    fn text_editor_reactivation_preserves_committed_text_and_resets_composition() {
        let first = text_context(3);
        let second = TextInputContext::try_new(WindowId::try_new(1, 8).unwrap(), 2, 7).unwrap();
        let mut editor = TextEditor::new();
        editor
            .activate(TextState::try_new(4, "a你", 4, 4).unwrap())
            .unwrap();
        editor
            .apply(TextInputEvent::preedit(1, first, 5, '好').unwrap())
            .unwrap();
        editor
            .apply(TextInputEvent::deactivated(2, first, 2, 5).unwrap())
            .unwrap();

        editor
            .apply(TextInputEvent::activated(3, second, 3, 0).unwrap())
            .unwrap();

        assert!(editor.is_active());
        assert_eq!(editor.state().revision(), 0);
        assert_eq!(editor.state().committed().as_str(), "a你");
        assert_eq!(editor.state().selection_start(), 4);
        assert_eq!(editor.state().selection_end(), 4);
        assert_eq!(editor.preedit(), None);
    }

    #[test]
    fn text_input_client_and_server_trackers_complete_edit_and_reactivation() {
        let context = text_context(3);
        let mut client = TextInputClientTracker::new();
        let mut server = TextInputServerTracker::new();
        let activate = TextInputCommand::activate(1, context).unwrap();
        client.send(activate).unwrap();
        server.receive(activate).unwrap();
        let activated = TextInputEvent::activated(1, context, 1, 0).unwrap();
        server.send(activated).unwrap();
        client.receive(activated).unwrap();

        let preedit = TextInputEvent::preedit(2, context, 1, '你').unwrap();
        server.send(preedit).unwrap();
        client.receive(preedit).unwrap();
        let state1 = TextState::try_new(1, "", 0, 0).unwrap();
        let ack1 = TextInputCommand::state_ack(2, context, 2, state1).unwrap();
        client.send(ack1).unwrap();
        server.receive(ack1).unwrap();
        let rendered1 = TextInputEvent::rendered(3, context, 2, state1).unwrap();
        server.send(rendered1).unwrap();
        client.receive(rendered1).unwrap();

        let commit = TextInputEvent::commit(4, context, 2, "你").unwrap();
        server.send(commit).unwrap();
        client.receive(commit).unwrap();
        let state2 = TextState::try_new(2, "你", 3, 3).unwrap();
        let ack2 = TextInputCommand::state_ack(3, context, 4, state2).unwrap();
        client.send(ack2).unwrap();
        server.receive(ack2).unwrap();
        let rendered2 = TextInputEvent::rendered(5, context, 3, state2).unwrap();
        server.send(rendered2).unwrap();
        client.receive(rendered2).unwrap();

        let deactivate = TextInputCommand::deactivate(4, context).unwrap();
        client.send(deactivate).unwrap();
        server.receive(deactivate).unwrap();
        let deactivated = TextInputEvent::deactivated(6, context, 4, 2).unwrap();
        server.send(deactivated).unwrap();
        client.receive(deactivated).unwrap();
        assert_eq!(client.phase(), TextInputSessionPhase::Deactivated);
        assert_eq!(server.phase(), TextInputSessionPhase::Deactivated);
        assert!(!client.outstanding_acknowledgment());
        assert!(!server.outstanding_acknowledgment());

        let next_context = text_context(4);
        let next_activate = TextInputCommand::activate(5, next_context).unwrap();
        client.send(next_activate).unwrap();
        server.receive(next_activate).unwrap();
        let next_activated = TextInputEvent::activated(7, next_context, 5, 0).unwrap();
        server.send(next_activated).unwrap();
        client.receive(next_activated).unwrap();
        assert_eq!(client.phase(), TextInputSessionPhase::Active);
        assert_eq!(server.phase(), TextInputSessionPhase::Active);
    }

    #[test]
    fn text_input_trackers_reject_sequence_ack_and_outstanding_violations_transactionally() {
        let context = text_context(3);
        let mut client = TextInputClientTracker::new();
        let gap = TextInputCommand::activate(2, context).unwrap();
        let before = client;
        assert_eq!(
            client.send(gap),
            Err(TextInputTrackerError::CommandSequenceGap)
        );
        assert_eq!(client, before);
        let activate = TextInputCommand::activate(1, context).unwrap();
        client.send(activate).unwrap();
        let before = client;
        assert_eq!(
            client.send(activate),
            Err(TextInputTrackerError::CommandSequenceReplay)
        );
        assert_eq!(client, before);
        let event_gap = TextInputEvent::activated(2, context, 1, 0).unwrap();
        assert_eq!(
            client.receive(event_gap),
            Err(TextInputTrackerError::EventSequenceGap)
        );
        assert_eq!(client, before);
        let ahead = TextInputEvent::activated(1, context, 2, 0).unwrap();
        assert_eq!(
            client.receive(ahead),
            Err(TextInputTrackerError::AcknowledgmentAhead)
        );
        assert_eq!(client, before);
        client
            .receive(TextInputEvent::activated(1, context, 1, 0).unwrap())
            .unwrap();
        let edit = TextInputEvent::preedit(2, context, 1, 'a').unwrap();
        client.receive(edit).unwrap();
        let outstanding = client;
        assert_eq!(
            client.receive(TextInputEvent::commit(3, context, 2, "a").unwrap()),
            Err(TextInputTrackerError::OutstandingAcknowledgment)
        );
        assert_eq!(client, outstanding);
        let state = TextState::try_new(1, "", 0, 0).unwrap();
        assert_eq!(
            client.send(TextInputCommand::state_ack(2, context, 1, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentBehind)
        );
        assert_eq!(client, outstanding);
        assert_eq!(
            client.send(TextInputCommand::state_ack(2, context, 3, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentAhead)
        );
        assert_eq!(client, outstanding);
    }

    #[test]
    fn text_input_server_tracker_rejects_bad_ack_and_sequence_without_mutation() {
        let context = text_context(3);
        let mut server = TextInputServerTracker::new();
        server
            .receive(TextInputCommand::activate(1, context).unwrap())
            .unwrap();
        server
            .send(TextInputEvent::activated(1, context, 1, 0).unwrap())
            .unwrap();
        server
            .send(TextInputEvent::preedit(2, context, 1, 'a').unwrap())
            .unwrap();
        let state = TextState::try_new(1, "", 0, 0).unwrap();
        let pending = server;
        assert_eq!(
            server.receive(TextInputCommand::state_ack(2, context, 1, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentBehind)
        );
        assert_eq!(server, pending);
        assert_eq!(
            server.receive(TextInputCommand::state_ack(2, context, 3, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentAhead)
        );
        assert_eq!(server, pending);
        server
            .receive(TextInputCommand::state_ack(2, context, 2, state).unwrap())
            .unwrap();
        let awaiting_render = server;
        assert_eq!(
            server.send(TextInputEvent::rendered(3, context, 1, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentBehind)
        );
        assert_eq!(server, awaiting_render);
        assert_eq!(
            server.send(TextInputEvent::rendered(3, context, 3, state).unwrap()),
            Err(TextInputTrackerError::AcknowledgmentAhead)
        );
        assert_eq!(server, awaiting_render);
        server
            .send(TextInputEvent::rendered(3, context, 2, state).unwrap())
            .unwrap();
        let ordered = server;
        assert_eq!(
            server.send(TextInputEvent::commit(5, context, 2, "a").unwrap()),
            Err(TextInputTrackerError::EventSequenceGap)
        );
        assert_eq!(server, ordered);
        assert_eq!(
            server.send(TextInputEvent::commit(3, context, 2, "a").unwrap()),
            Err(TextInputTrackerError::EventSequenceReplay)
        );
        assert_eq!(server, ordered);
    }

    #[test]
    fn text_input_deactivation_is_terminal_and_new_sessions_are_monotonic() {
        let context = text_context(3);
        let mut client = TextInputClientTracker::new();
        client
            .send(TextInputCommand::activate(1, context).unwrap())
            .unwrap();
        client
            .receive(TextInputEvent::activated(1, context, 1, 0).unwrap())
            .unwrap();
        client
            .send(TextInputCommand::deactivate(2, context).unwrap())
            .unwrap();
        client
            .receive(TextInputEvent::deactivated(2, context, 2, 0).unwrap())
            .unwrap();
        let terminal = client;
        assert_eq!(
            client.receive(TextInputEvent::preedit(3, context, 1, 'x').unwrap()),
            Err(TextInputTrackerError::DeactivationTerminal)
        );
        assert_eq!(client, terminal);
        assert_eq!(
            client.send(TextInputCommand::activate(3, context).unwrap()),
            Err(TextInputTrackerError::SessionNotIncreasing)
        );
        assert_eq!(client, terminal);
        let next = TextInputContext::try_new(WindowId::try_new(1, 8).unwrap(), 4, 12).unwrap();
        client
            .send(TextInputCommand::activate(3, next).unwrap())
            .unwrap();
        assert_eq!(client.context(), Some(next));
        assert_eq!(client.phase(), TextInputSessionPhase::Activating);
    }
}
