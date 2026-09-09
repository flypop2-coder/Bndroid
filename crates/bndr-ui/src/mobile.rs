//! Allocation-free, code-drawn phone UI for the opt-in mobile runtime.
//!
//! The guest scanout is a common 720x1600, 20:9 phone-shaped profile. Layout
//! is authored on a 360x800 design grid and rasterized at an exact 2x scale.
//! Neither value claims a physical panel or density. The row renderer lets
//! the no-std client produce a complete frame without reserving a
//! multi-megabyte userspace framebuffer.

#[cfg(feature = "androidbox-layout-weight15")]
use super::android_scene::AndroidSceneLayoutSize;
#[cfg(feature = "androidbox-layout-row14")]
use super::android_scene::AndroidSceneOrientation;
#[cfg(feature = "androidbox-scene-rpc2")]
use super::android_scene::{
    AndroidInstalledActivitySceneNode, AndroidInstalledActivitySceneState, AndroidSceneViewKind,
};
#[cfg(feature = "mobile-ui-runtime")]
use super::{DamageRect, DamageRegions};
#[cfg(feature = "mobile-system-chrome0")]
pub use super::{
    MOBILE_CONTENT_BOTTOM, MOBILE_CONTENT_HEIGHT, MOBILE_CONTENT_TOP,
    MOBILE_CONTENT_VIEWPORT_BOTTOM, MOBILE_CONTENT_VIEWPORT_HEIGHT, MOBILE_CONTENT_VIEWPORT_WIDTH,
    MOBILE_CONTENT_VIEWPORT_X, MOBILE_CONTENT_VIEWPORT_Y, MOBILE_CONTENT_WIDTH, MOBILE_CONTENT_X,
};
use super::{
    ShellAppId, ShellRect, UiCompatibleActivityIdentity, UiRecentIdentity, UiSoftwareDimming,
    UiSystemUiMode,
};

#[path = "mobile_font_data.rs"]
mod mobile_font_data;

#[cfg(feature = "mobile-ui-runtime")]
#[path = "mobile_raster.rs"]
mod raster;
#[cfg(feature = "mobile-ui-runtime")]
pub use raster::{MobileRasterCache, clip_damage_plan, render_region};
#[cfg(feature = "mobile-system-chrome0")]
pub use raster::{render_content_region, render_system_chrome_region, system_chrome_damage_plan};

pub const WIDTH: usize = 720;
pub const HEIGHT: usize = 1_600;
pub const PIXEL_COUNT: usize = WIDTH * HEIGHT;
pub const DESIGN_WIDTH: u16 = 360;
pub const DESIGN_HEIGHT: u16 = 800;
const SCALE: i32 = 2;
/// Rounded display-mask radius in physical scanout pixels.
///
/// This is a UI safe-area profile for the local preview, not a claim about a
/// particular panel, glass shape, or physical display measurement.
pub const SCREEN_CORNER_RADIUS_PX: u16 = 64;

const FONT_ALPHA4: &[u8; mobile_font_data::FONT_ALPHA4_BYTES] =
    include_bytes!(concat!(env!("OUT_DIR"), "/mobile_font_alpha4.bin"));
const _: () = assert!(mobile_font_data::FONT_ALPHA4_BYTES <= 128 * 1024);

const COLOR_DARK_TOP: u32 = 0x0007_0d1c;
const COLOR_DARK_BOTTOM: u32 = 0x0010_1830;
const COLOR_LIGHT_TOP: u32 = 0x00e9_f1ff;
const COLOR_LIGHT_BOTTOM: u32 = 0x00f8_faff;
const COLOR_CYAN_GLOW: u32 = 0x001d_bbd1;
const COLOR_CARD_DARK: u32 = 0x0018_2238;
const COLOR_CARD_RAISED_DARK: u32 = 0x0020_2d49;
const COLOR_CARD_LIGHT: u32 = 0x00ff_ffff;
const COLOR_CARD_RAISED_LIGHT: u32 = 0x00f1_f5fc;
const COLOR_BORDER_DARK: u32 = 0x0032_4264;
const COLOR_BORDER_LIGHT: u32 = 0x00d3_dceb;
const COLOR_TEXT_DARK: u32 = 0x00f7_f9fd;
const COLOR_TEXT_LIGHT: u32 = 0x0012_1a2b;
const COLOR_TEXT_MUTED_DARK: u32 = 0x00a6_b2c8;
const COLOR_TEXT_MUTED_LIGHT: u32 = 0x005e_6a7e;
const COLOR_TEXT_WEAK_DARK: u32 = 0x0075_849f;
const COLOR_TEXT_WEAK_LIGHT: u32 = 0x0086_91a3;
const COLOR_BLUE: u32 = 0x004e_7fff;
const COLOR_BLUE_ALT: u32 = 0x0091_6cff;
const COLOR_PHONE: u32 = 0x0031_c85a;
const COLOR_MESSAGES: u32 = 0x003b_82f6;
const COLOR_CALCULATOR: u32 = 0x00f5_9e0b;
const COLOR_SETTINGS: u32 = 0x0079_879f;
const COLOR_ANDROIDBOX: u32 = 0x0010_766f;
const COLOR_TOGGLE_OFF: u32 = 0x0042_4d64;
const COLOR_WHITE: u32 = 0x00ff_ffff;
const COLOR_BLACK: u32 = 0x0000_0000;
const COLOR_SHADOW: u32 = 0x0004_0812;
const COLOR_SHADOW_LIGHT: u32 = 0x00c8_d3e4;
const COLOR_AMBER: u32 = 0x00f5_a524;
const COLOR_PANEL_DARK: u32 = 0x000d_1629;
const COLOR_PANEL_LIGHT: u32 = 0x00f3_f7ff;

/// Chooses one of the two canonical high-contrast inks for a solid color.
/// The threshold is intentionally deterministic integer sRGB luma; the test
/// suite independently enforces the stricter WCAG contrast ratio with linear
/// channels for every product icon/accent surface.
const fn accessible_surface_ink(surface: u32) -> u32 {
    let luma =
        ((surface >> 16) & 0xff) * 2_126 + ((surface >> 8) & 0xff) * 7_152 + (surface & 0xff) * 722;
    if luma >= 1_100_000 {
        COLOR_TEXT_LIGHT
    } else {
        COLOR_WHITE
    }
}

/// Semantic color roles shared by every native page and projected Android
/// Activity. Keeping these roles in one allocation-free value prevents a
/// page from inventing a second light/dark palette while still letting the
/// renderer select colors without storing theme state in global memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileThemeTokens {
    background_top: u32,
    background_bottom: u32,
    surface: u32,
    surface_raised: u32,
    panel: u32,
    outline: u32,
    text_primary: u32,
    text_secondary: u32,
    text_tertiary: u32,
    shadow: u32,
    accent: u32,
    accent_container: u32,
    on_accent: u32,
    wallpaper_glow_primary: u32,
    wallpaper_glow_secondary: u32,
}

/// Builds the complete allocation-free color scheme from the user's current
/// accent choice. The procedural wallpaper and every semantic surface share
/// this seed, so switching Accent changes one coherent scheme rather than a
/// handful of unrelated blue pixels.
const fn mobile_theme_tokens(
    dark: bool,
    alternate_accent: bool,
    high_contrast: bool,
) -> MobileThemeTokens {
    let accent = if alternate_accent {
        COLOR_BLUE_ALT
    } else {
        COLOR_BLUE
    };
    if dark {
        MobileThemeTokens {
            background_top: blend(COLOR_DARK_TOP, accent, 10),
            background_bottom: blend(COLOR_DARK_BOTTOM, accent, 8),
            surface: blend(COLOR_CARD_DARK, accent, if high_contrast { 8 } else { 14 }),
            surface_raised: blend(
                COLOR_CARD_RAISED_DARK,
                accent,
                if high_contrast { 14 } else { 22 },
            ),
            panel: blend(COLOR_PANEL_DARK, accent, if high_contrast { 6 } else { 12 }),
            outline: if high_contrast {
                blend(COLOR_TEXT_DARK, accent, 48)
            } else {
                blend(COLOR_BORDER_DARK, accent, 38)
            },
            text_primary: COLOR_TEXT_DARK,
            text_secondary: if high_contrast {
                COLOR_TEXT_DARK
            } else {
                COLOR_TEXT_MUTED_DARK
            },
            text_tertiary: if high_contrast {
                COLOR_TEXT_MUTED_DARK
            } else {
                COLOR_TEXT_WEAK_DARK
            },
            shadow: COLOR_SHADOW,
            accent,
            accent_container: blend(COLOR_CARD_RAISED_DARK, accent, 72),
            on_accent: accessible_surface_ink(accent),
            wallpaper_glow_primary: accent,
            wallpaper_glow_secondary: blend(COLOR_CYAN_GLOW, accent, 28),
        }
    } else {
        MobileThemeTokens {
            background_top: blend(COLOR_LIGHT_TOP, accent, 10),
            background_bottom: blend(COLOR_LIGHT_BOTTOM, accent, 4),
            surface: blend(COLOR_CARD_LIGHT, accent, if high_contrast { 2 } else { 6 }),
            surface_raised: blend(
                COLOR_CARD_RAISED_LIGHT,
                accent,
                if high_contrast { 4 } else { 10 },
            ),
            panel: blend(COLOR_PANEL_LIGHT, accent, if high_contrast { 2 } else { 8 }),
            outline: if high_contrast {
                blend(COLOR_TEXT_LIGHT, accent, 22)
            } else {
                blend(COLOR_BORDER_LIGHT, accent, 24)
            },
            text_primary: COLOR_TEXT_LIGHT,
            text_secondary: if high_contrast {
                COLOR_TEXT_LIGHT
            } else {
                COLOR_TEXT_MUTED_LIGHT
            },
            text_tertiary: if high_contrast {
                COLOR_TEXT_MUTED_LIGHT
            } else {
                COLOR_TEXT_WEAK_LIGHT
            },
            shadow: COLOR_SHADOW_LIGHT,
            accent,
            accent_container: blend(COLOR_CARD_RAISED_LIGHT, accent, 54),
            on_accent: accessible_surface_ink(accent),
            wallpaper_glow_primary: blend(COLOR_WHITE, accent, 70),
            wallpaper_glow_secondary: blend(COLOR_WHITE, COLOR_CYAN_GLOW, 58),
        }
    }
}

/// A small shape/elevation scale. Exact hit targets remain public physical
/// rectangles; these tokens only define how visual surfaces are drawn inside
/// those already-authoritative bounds.
struct MobileShapeTokens;

impl MobileShapeTokens {
    const OUTLINE_PX: i32 = MobileSpacingTokens::HAIRLINE;
    const ELEVATION_LOW_PX: i32 = MobileSpacingTokens::XS - MobileSpacingTokens::HAIRLINE;
    const RADIUS_SETTING_ICON: i32 = MobileSpacingTokens::MD - MobileSpacingTokens::HAIRLINE;
    const RADIUS_APP_ICON: i32 = MobileSpacingTokens::LG;
    const RADIUS_CONTROL: i32 = MobileSpacingTokens::APP_GUTTER;
    const RADIUS_LARGE: i32 = MobileSpacingTokens::CONTENT_GUTTER;
    const RADIUS_PANEL: i32 = MobileSpacingTokens::CONTENT_GUTTER + MobileSpacingTokens::XS;
}

/// Spacing roles used by the phone shell. Public hit rectangles stay the
/// authority; these values remove page-local magic numbers without moving a
/// target or weakening an existing damage contract.
struct MobileSpacingTokens;

impl MobileSpacingTokens {
    const HAIRLINE: i32 = 1;
    const XXS: i32 = 2;
    const XS: i32 = 4;
    const MD: i32 = 12;
    const LG: i32 = 16;
    const PAGE_GUTTER: i32 = 18;
    const APP_GUTTER: i32 = 20;
    const CONTENT_GUTTER: i32 = 24;
}

// Public targets are physical scanout coordinates. They intentionally match
// the top-level shell targets so render and input cannot drift apart.
pub const HOME_PHONE_TARGET: ShellRect = ShellRect::new(20, 1_340, 170, 190);
pub const HOME_MESSAGES_TARGET: ShellRect = ShellRect::new(190, 1_340, 170, 190);
pub const HOME_CALCULATOR_TARGET: ShellRect = ShellRect::new(360, 1_340, 170, 190);
pub const HOME_SETTINGS_TARGET: ShellRect = ShellRect::new(530, 1_340, 170, 190);
pub const APP_BACK_TARGET: ShellRect = ShellRect::new(0, 64, 160, 112);
/// Settings-home entry into the real Display subpage.
pub const SETTINGS_DISPLAY_TARGET: ShellRect = ShellRect::new(32, 408, 656, 132);
/// Display-page session theme row.
pub const DISPLAY_THEME_TARGET: ShellRect = ShellRect::new(32, 408, 656, 132);
/// Display-page session accent row.
pub const DISPLAY_ACCENT_TARGET: ShellRect = ShellRect::new(32, 540, 656, 132);
/// Five-stop software-surface dimming slider in the Settings appearance card.
///
/// This target controls only rendered pixels in the current UI session. It is
/// not a physical panel, backlight, power-saving, or hardware-brightness API.
pub const DISPLAY_DIMMING_TARGET: ShellRect = ShellRect::new(336, 744, 336, 112);
/// Display-owned entry into the real accessibility appearance page.
pub const DISPLAY_ACCESSIBILITY_TARGET: ShellRect = ShellRect::new(32, 1_304, 656, 192);
/// Accessibility-page two-stop text-size preference.
pub const ACCESSIBILITY_LARGE_TEXT_TARGET: ShellRect = ShellRect::new(32, 408, 656, 132);
/// Accessibility-page explicit contrast preference.
pub const ACCESSIBILITY_HIGH_CONTRAST_TARGET: ShellRect = ShellRect::new(32, 540, 656, 132);
pub const SETTINGS_APPS_TARGET: ShellRect = ShellRect::new(48, 1_284, 640, 132);
pub const SETTINGS_ABOUT_TARGET: ShellRect = ShellRect::new(48, 1_416, 640, 132);
pub const APPS_BACK_TARGET: ShellRect = APP_BACK_TARGET;
/// Destructive package action in Apps' scroll-content coordinate space.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const APPS_UNINSTALL_TARGET: ShellRect = ShellRect::new(36, 1_424, 648, 148);
/// Fixed dialog actions remain inside the untrusted App content viewport.
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const APPS_UNINSTALL_CANCEL_TARGET: ShellRect = ShellRect::new(80, 1_016, 250, 112);
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const APPS_UNINSTALL_CONFIRM_TARGET: ShellRect = ShellRect::new(390, 1_016, 250, 112);
/// ABI 53's install/update action is bound to the candidate card.
#[cfg(feature = "androidbox-runtime-install2")]
pub const APPS_INSTALL_TARGET: ShellRect = ShellRect::new(36, 520, 648, 164);
#[cfg(feature = "androidbox-runtime-install2")]
pub const APPS_UPDATE_TARGET: ShellRect = ShellRect::new(480, 336, 184, 88);
/// ABI 55's two fixed installed-package selectors inside the Apps summary
/// card. The half-open columns meet at x=360 without overlapping.
#[cfg(feature = "androidbox-multipackage4")]
pub const APPS_INSTALLED_FIRST_TARGET: ShellRect = ShellRect::new(36, 232, 324, 208);
#[cfg(feature = "androidbox-multipackage4")]
pub const APPS_INSTALLED_SECOND_TARGET: ShellRect = ShellRect::new(360, 232, 324, 208);
#[cfg(feature = "androidbox-runtime-install2")]
pub const APPS_INSTALL_CANCEL_TARGET: ShellRect = ShellRect::new(80, 1_016, 250, 112);
#[cfg(feature = "androidbox-runtime-install2")]
pub const APPS_INSTALL_CONFIRM_TARGET: ShellRect = ShellRect::new(390, 1_016, 250, 112);
pub const ABOUT_BACK_TARGET: ShellRect = APP_BACK_TARGET;
/// Fixed content viewport below the app header for Settings and Apps.
///
/// Coordinates are physical scanout pixels. The top is exactly the bottom of
/// the existing app-back target, while the bottom is the trusted navigation
/// boundary. Scroll content is clipped to this interval and therefore cannot
/// overwrite either system chrome region.
pub const PAGE_SCROLL_VIEWPORT_TOP_PX: u16 = 176;
pub const PAGE_SCROLL_VIEWPORT_BOTTOM_PX: u16 = SYSTEM_NAV_TOP_PX;
/// Minimum dominant vertical displacement before a tap may become a scroll.
pub const PAGE_SCROLL_ACTIVATION_PX: u16 = 24;
/// Every scroll position settles on this deterministic physical-pixel grid.
pub const PAGE_SCROLL_QUANTUM_PX: u16 = 8;
/// Settings' bounded content extent leaves its final honest System-status row
/// exactly at the viewport bottom at maximum scroll.
pub const SETTINGS_SCROLL_MAX_PX: u16 = 232;
/// Apps' bounded content extent leaves its compatibility-boundary card
/// exactly at the viewport bottom at maximum scroll.
#[cfg(not(feature = "androidbox-runtime-uninstall1"))]
pub const APPS_SCROLL_MAX_PX: u16 = 192;
#[cfg(feature = "androidbox-runtime-uninstall1")]
pub const APPS_SCROLL_MAX_PX: u16 = 280;
pub const QUICK_THEME_TARGET: ShellRect = ShellRect::new(32, 296, 312, 184);
pub const QUICK_ACCENT_TARGET: ShellRect = ShellRect::new(376, 296, 312, 184);
/// The same five-stop software-surface slider exposed in Quick Settings.
pub const QUICK_DIMMING_TARGET: ShellRect = ShellRect::new(336, 688, 336, 120);
pub const QUICK_CLOSE_TARGET: ShellRect = ShellRect::new(584, 104, 128, 112);
/// One bounded boot-local System UI notification in the settled shade.
///
/// This is not an application-posting, push-delivery, persistence, sound, or
/// vibration API. The SurfaceServer session owns only its visible/dismissed
/// state; the fixed content describes the local preview itself.
pub const QUICK_BOOT_NOTIFICATION_TARGET: ShellRect = ShellRect::new(32, 904, 656, 232);
/// The same boot-local System UI notice where it is rendered on Lock.
///
/// A tap remains inert while locked, but a horizontal swipe may dismiss the
/// canonical SurfaceServer-owned notice. This does not grant navigation,
/// unlock, application-notification, delivery, sound, or vibration authority.
pub const LOCK_BOOT_NOTIFICATION_TARGET: ShellRect = ShellRect::new(48, 652, 624, 232);
pub const SYSTEM_NAV_TOP_PX: u16 = 1_548;
pub const SYSTEM_HOME_TARGET: ShellRect = ShellRect::new(240, 1_548, 240, 52);
/// The single honest recent-app identity card shown by Overview.
///
/// The Launcher cannot read App pixels, so this target never represents a
/// framebuffer thumbnail, background task, or killable process card.
pub const OVERVIEW_RECENT_TARGET: ShellRect = ShellRect::new(48, 424, 624, 760);
/// Minimum vertical motion before an Overview-card tap becomes a drag.
pub const OVERVIEW_RECENT_SWIPE_ACTIVATION_PX: u16 = 32;
/// Upward card travel required to remove the exact recent identity.
pub const OVERVIEW_RECENT_DISMISS_THRESHOLD_PX: u16 = 224;
/// Bounded finger-follow travel; the card never escapes the display model.
pub const OVERVIEW_RECENT_MAX_OFFSET_PX: u16 = 320;
const OVERVIEW_RECENT_RENDER_QUANTUM_PX: u16 = 8;
pub const OVERVIEW_GESTURE_COMMIT_PX: u16 = 240;
pub const OVERVIEW_RENDER_MAX_PX: u16 = 320;
pub const OVERVIEW_HOME_COMMIT_PX: u16 = 480;
const OVERVIEW_RENDER_QUANTUM_PX: u16 = 8;
pub const DRAWER_PHONE_TARGET: ShellRect = ShellRect::new(20, 320, 170, 240);
pub const DRAWER_MESSAGES_TARGET: ShellRect = ShellRect::new(190, 320, 170, 240);
pub const DRAWER_CALCULATOR_TARGET: ShellRect = ShellRect::new(360, 320, 170, 240);
pub const DRAWER_SETTINGS_TARGET: ShellRect = ShellRect::new(530, 320, 170, 240);
/// Fifth All apps item, placed on the second row without replacing the four
/// stable Home-dock applications.
pub const DRAWER_ANDROIDBOX_TARGET: ShellRect = ShellRect::new(20, 580, 170, 240);
/// ABI 55's second fixed-capacity installed-app cell.
#[cfg(feature = "androidbox-multipackage4")]
pub const DRAWER_ANDROIDBOX_SECOND_TARGET: ShellRect = ShellRect::new(190, 580, 170, 240);
pub const DRAWER_HANDLE_TARGET: ShellRect = ShellRect::new(240, 120, 240, 112);
pub const DRAWER_HOME_TARGET: ShellRect = ShellRect::new(240, 1_512, 240, 88);
/// Launcher-local request button for the bounded AndroidBox DEX-0 demo.
///
/// A matching release emits `MobileAction::ExecuteAndroidBoxDex`; the UI
/// model never executes bytecode or changes its own result counters.
pub const ANDROIDBOX_EXECUTE_TARGET: ShellRect = ShellRect::new(48, 1_240, 624, 144);
/// Fixed physical bounds of the one admitted installed-Activity Button.
#[cfg(feature = "androidbox-interactive0")]
pub const INSTALLED_ANDROID_BUTTON_TARGET: ShellRect = ShellRect::new(68, 664, 584, 128);
/// Maximum visible launcher-activity identity mirrored from AndroidBox.
///
/// The fixed-capacity value is deliberately allocation-free and is not a
/// PackageManager, component registry, or arbitrary Android application ABI.
pub const ANDROIDBOX_ACTIVITY_IDENTITY_CAPACITY: usize = 48;
/// Maximum caption advance for the one-line Activity identity, in output
/// pixels. This keeps it inside the Activity card's inset content region.
const ANDROIDBOX_ACTIVITY_IDENTITY_MAX_WIDTH_PX: u16 = 480;
/// Maximum visible TextView content mirrored from the restricted framework
/// shim after a completed `onCreate`.
pub const ANDROIDBOX_TEXT_VIEW_CONTENT_CAPACITY: usize = 64;
/// Maximum caption advance for either of the two TextView lines, in output
/// pixels.
const ANDROIDBOX_TEXT_VIEW_LINE_MAX_WIDTH_PX: u16 = 560;
const ANDROIDBOX_TEXT_VIEW_FIRST_LINE_BYTES: usize = 32;
// These capacities intentionally match the canonical BNDAPS01 snapshot and
// the package-store admission maxima. A package accepted by EL1 must never
// become unrepresentable when EL0 mirrors its read-only catalog metadata.
pub const ANDROID_INSTALLED_PACKAGE_CAPACITY: usize = 96;
pub const ANDROID_INSTALLED_ACTIVITY_CAPACITY: usize = 128;
pub const ANDROID_INSTALLED_TITLE_CAPACITY: usize = 128;
pub const ANDROID_INSTALLED_TEXT_CAPACITY: usize = 128;
#[cfg(feature = "androidbox-multipackage4")]
pub const ANDROID_INSTALLED_DIRECTORY_CAPACITY: usize = 2;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_INSTALLED_ICON_WIDTH: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_INSTALLED_ICON_HEIGHT: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
pub const ANDROID_INSTALLED_ICON_PIXEL_COUNT: usize =
    ANDROID_INSTALLED_ICON_WIDTH * ANDROID_INSTALLED_ICON_HEIGHT;
#[cfg(feature = "androidbox-icon-resources5")]
const ANDROID_INSTALLED_ICON_PALETTE_CAPACITY: usize = 16;
#[cfg(feature = "androidbox-icon-resources5")]
const ANDROID_INSTALLED_ICON_PACKED_BYTES: usize = ANDROID_INSTALLED_ICON_PIXEL_COUNT / 2;
const _: () = assert!(ANDROID_INSTALLED_PACKAGE_CAPACITY <= u8::MAX as usize);
const _: () = assert!(ANDROID_INSTALLED_ACTIVITY_CAPACITY <= u8::MAX as usize);
const _: () = assert!(ANDROID_INSTALLED_TITLE_CAPACITY <= u8::MAX as usize);
const _: () = assert!(ANDROID_INSTALLED_TEXT_CAPACITY <= u8::MAX as usize);
pub const PHONE_KEY_TARGETS: [ShellRect; 12] = [
    ShellRect::new(84, 420, 160, 176),
    ShellRect::new(280, 420, 160, 176),
    ShellRect::new(476, 420, 160, 176),
    ShellRect::new(84, 628, 160, 176),
    ShellRect::new(280, 628, 160, 176),
    ShellRect::new(476, 628, 160, 176),
    ShellRect::new(84, 836, 160, 176),
    ShellRect::new(280, 836, 160, 176),
    ShellRect::new(476, 836, 160, 176),
    ShellRect::new(84, 1_044, 160, 176),
    ShellRect::new(280, 1_044, 160, 176),
    ShellRect::new(476, 1_044, 160, 176),
];
pub const PHONE_BACKSPACE_TARGET: ShellRect = ShellRect::new(436, 1_296, 160, 160);
pub const CALCULATOR_KEY_TARGETS: [ShellRect; 20] = [
    ShellRect::new(36, 480, 144, 168),
    ShellRect::new(198, 480, 144, 168),
    ShellRect::new(360, 480, 144, 168),
    ShellRect::new(522, 480, 144, 168),
    ShellRect::new(36, 680, 144, 168),
    ShellRect::new(198, 680, 144, 168),
    ShellRect::new(360, 680, 144, 168),
    ShellRect::new(522, 680, 144, 168),
    ShellRect::new(36, 880, 144, 168),
    ShellRect::new(198, 880, 144, 168),
    ShellRect::new(360, 880, 144, 168),
    ShellRect::new(522, 880, 144, 168),
    ShellRect::new(36, 1_080, 144, 168),
    ShellRect::new(198, 1_080, 144, 168),
    ShellRect::new(360, 1_080, 144, 168),
    ShellRect::new(522, 1_080, 144, 168),
    ShellRect::new(36, 1_280, 306, 168),
    ShellRect::new(360, 1_280, 144, 168),
    ShellRect::new(522, 1_280, 144, 168),
    ShellRect::new(520, 360, 152, 96),
];
pub const MESSAGE_GUIDE_TARGET: ShellRect = ShellRect::new(36, 668, 648, 152);
pub const MESSAGE_OFFLINE_TARGET: ShellRect = ShellRect::new(36, 820, 648, 152);

#[cfg(feature = "mobile-ui-runtime")]
const STATUS_TIME_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 12,
    width: 200,
    height: 52,
};
#[cfg(feature = "mobile-ui-runtime")]
const HOME_TIME_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 120,
    width: 300,
    height: 152,
};
#[cfg(feature = "mobile-ui-runtime")]
const HOME_DATE_DAMAGE: DamageRect = DamageRect {
    x: 36,
    y: 328,
    width: 648,
    height: 160,
};
#[cfg(feature = "mobile-ui-runtime")]
const LOCK_TIME_DATE_DAMAGE: DamageRect = DamageRect {
    x: 96,
    y: 232,
    width: 528,
    height: 212,
};
#[cfg(feature = "mobile-ui-runtime")]
const SHADE_TIME_DATE_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 96,
    width: 328,
    height: 128,
};
#[cfg(feature = "mobile-ui-runtime")]
const LOCK_BOOT_NOTIFICATION_DAMAGE: DamageRect = DamageRect {
    x: 48,
    y: 652,
    width: 624,
    height: 240,
};
#[cfg(feature = "mobile-ui-runtime")]
const SHADE_BOOT_NOTIFICATION_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 904,
    width: 656,
    height: 240,
};
#[cfg(feature = "mobile-ui-runtime")]
const SHADE_EMPTY_NOTIFICATION_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 904,
    width: 656,
    height: 344,
};
#[cfg(feature = "mobile-ui-runtime")]
const PHONE_NUMBER_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 240,
    width: 656,
    height: 144,
};
#[cfg(feature = "mobile-ui-runtime")]
const CALCULATOR_DISPLAY_DAMAGE: DamageRect = DamageRect {
    x: 32,
    y: 192,
    width: 656,
    height: 264,
};
#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-interactive0"))]
const INSTALLED_ANDROID_LABEL_DAMAGE: DamageRect = DamageRect {
    x: 68,
    y: 448,
    width: 584,
    height: 176,
};
#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-interactive0"))]
const INSTALLED_ANDROID_BUTTON_DAMAGE: DamageRect = DamageRect {
    x: INSTALLED_ANDROID_BUTTON_TARGET.x,
    y: INSTALLED_ANDROID_BUTTON_TARGET.y,
    width: INSTALLED_ANDROID_BUTTON_TARGET.width,
    height: INSTALLED_ANDROID_BUTTON_TARGET.height,
};
pub const SHADE_GESTURE_START_MAX_Y: u16 = 120;
pub const SHADE_GESTURE_MIN_TRAVEL: u16 = 240;
/// Maximum visible quick-settings extent, measured in physical scanout pixels.
///
/// The value reaches the top of the 52-output-pixel navigation area, so a
/// settled shade owns the full content surface without covering system Home.
pub const SHADE_REVEAL_MAX: u16 = 1_548;
const DIMMING_TRACK_START_X_PX: u16 = 356;
const DIMMING_TRACK_END_X_PX: u16 = 648;
pub const DRAWER_GESTURE_START_MIN_Y: u16 = 480;
pub const DRAWER_GESTURE_START_MAX_Y: u16 = 1_340;
pub const DRAWER_GESTURE_MIN_TRAVEL: u16 = 240;
pub const DRAWER_RENDER_QUANTUM: u16 = 16;
/// Visible All apps extent from its 64-design-pixel top edge to system nav.
pub const DRAWER_REVEAL_MAX: u16 = SYSTEM_NAV_TOP_PX - 128;
pub const BACK_GESTURE_START_MAX_X: u16 = 48;
pub const BACK_GESTURE_START_MIN_Y: u16 = 128;
pub const BACK_GESTURE_ACTIVATION_PX: u16 = 24;
pub const BACK_GESTURE_COMMIT_PX: u16 = 144;
pub const BACK_GESTURE_MAX_VERTICAL_DRIFT_PX: u16 = 192;
pub const BACK_GESTURE_RENDER_QUANTUM: u16 = 8;
pub const BACK_GESTURE_REVEAL_MAX: u16 = 192;
pub const UNLOCK_GESTURE_START_MIN_Y: u16 = 960;
pub const UNLOCK_GESTURE_ACTIVATION_PX: u16 = 24;
pub const UNLOCK_GESTURE_COMMIT_PX: u16 = 240;
pub const UNLOCK_GESTURE_MAX_HORIZONTAL_DRIFT_PX: u16 = 192;
pub const UNLOCK_RENDER_QUANTUM: u16 = 16;
pub const UNLOCK_REVEAL_MAX: u16 = 720;
pub const BOOT_NOTIFICATION_SWIPE_ACTIVATION_PX: u16 = 24;
pub const BOOT_NOTIFICATION_DISMISS_THRESHOLD_PX: u16 = 160;
pub const BOOT_NOTIFICATION_RENDER_QUANTUM_PX: u16 = 8;
pub const BOOT_NOTIFICATION_MAX_OFFSET_PX: i16 = 720;
/// Versioned, deterministic full-page transition used by the local mobile
/// preview.
///
/// These are software-rendered intermediate positions, not timestamps,
/// refresh-rate samples, or a hardware-vsync contract. A runtime presents the
/// four entering positions in order and then presents the exact stable page.
pub const PAGE_TRANSITION_VERSION: u8 = 1;
pub const PAGE_TRANSITION_ENTER_OFFSETS: [u16; 4] = [1_248, 832, 416, 128];
pub const PAGE_TRANSITION_EXIT_OFFSETS: [u16; 4] = [128, 416, 832, 1_248];
pub const PAGE_TRANSITION_MAX_OFFSET: u16 = SYSTEM_NAV_TOP_PX;
pub const PHONE_DIGIT_CAPACITY: usize = 16;
pub const CALCULATOR_SCALE: i64 = 1_000_000;
pub const CALCULATOR_FRACTION_DIGITS: u8 = 6;
pub const CALCULATOR_VALUE_LIMIT: i64 = 999_999_999_999_000_000;
const SECONDS_PER_DAY: u64 = 86_400;
const MAX_SUPPORTED_UNIX_SECONDS: u64 = 253_402_300_799;

const WEEKDAY_SHORT_NAMES: [&[u8]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];
const WEEKDAY_LONG_NAMES: [&[u8]; 7] = [
    b"Sunday",
    b"Monday",
    b"Tuesday",
    b"Wednesday",
    b"Thursday",
    b"Friday",
    b"Saturday",
];
const MONTH_SHORT_NAMES: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];
const MONTH_LONG_NAMES: [&[u8]; 12] = [
    b"January",
    b"February",
    b"March",
    b"April",
    b"May",
    b"June",
    b"July",
    b"August",
    b"September",
    b"October",
    b"November",
    b"December",
];

/// Validated civil-time snapshot supplied by the system UI session.
///
/// The renderer never guesses a date when the source is unavailable. The
/// snapshot deliberately has no timezone, trust, persistence, or ticking
/// semantics: callers decide which Unix-second source they accept and replace
/// the whole value when a later revision is available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MobileTimeSnapshot {
    available: bool,
    year: u16,
    month: u8,
    day: u8,
    weekday: u8,
    hour: u8,
    minute: u8,
}

impl Default for MobileTimeSnapshot {
    fn default() -> Self {
        Self::unavailable()
    }
}

impl MobileTimeSnapshot {
    pub const fn unavailable() -> Self {
        Self {
            available: false,
            year: 0,
            month: 0,
            day: 0,
            weekday: 0,
            hour: 0,
            minute: 0,
        }
    }

    /// Converts a non-negative Unix-second snapshot to Gregorian civil time.
    ///
    /// Values after 9999-12-31 fail closed instead of wrapping fields. Leap
    /// seconds are outside this preview contract.
    pub fn from_unix_seconds(unix_seconds: u64) -> Self {
        if unix_seconds > MAX_SUPPORTED_UNIX_SECONDS {
            return Self::unavailable();
        }

        let days = (unix_seconds / SECONDS_PER_DAY) as i64;
        let seconds_in_day = unix_seconds % SECONDS_PER_DAY;
        let hour = (seconds_in_day / 3_600) as u8;
        let minute = (seconds_in_day % 3_600 / 60) as u8;

        // Howard Hinnant's days-from-civil inverse, with day zero at
        // 1970-01-01. All accepted inputs are non-negative, so the shifted
        // value and era are non-negative as well.
        let shifted = days + 719_468;
        let era = shifted / 146_097;
        let day_of_era = shifted - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let mut year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_prime = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
        let month = month_prime + if month_prime < 10 { 3 } else { -9 };
        if month <= 2 {
            year += 1;
        }
        if !(1970..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day)
        {
            return Self::unavailable();
        }

        Self {
            available: true,
            year: year as u16,
            month: month as u8,
            day: day as u8,
            weekday: ((days + 4) % 7) as u8,
            hour,
            minute,
        }
    }

    pub const fn is_available(self) -> bool {
        self.available
    }

    fn time_text(self) -> MobileAsciiText<5> {
        let mut text = MobileAsciiText::new();
        if !self.available {
            text.push_bytes(b"--:--");
            return text;
        }
        text.push_two_digits(self.hour);
        text.push_byte(b':');
        text.push_two_digits(self.minute);
        text
    }

    fn short_date_text(self) -> MobileAsciiText<24> {
        self.date_text(&WEEKDAY_SHORT_NAMES, &MONTH_SHORT_NAMES)
    }

    fn long_date_text(self) -> MobileAsciiText<24> {
        self.date_text(&WEEKDAY_LONG_NAMES, &MONTH_LONG_NAMES)
    }

    fn date_text<const N: usize>(
        self,
        weekday_names: &[&[u8]; 7],
        month_names: &[&[u8]; 12],
    ) -> MobileAsciiText<N> {
        let mut text = MobileAsciiText::new();
        if !self.available {
            text.push_bytes(b"Time unavailable");
            return text;
        }
        text.push_bytes(weekday_names[usize::from(self.weekday)]);
        text.push_bytes(b", ");
        text.push_bytes(month_names[usize::from(self.month - 1)]);
        text.push_byte(b' ');
        text.push_decimal_day(self.day);
        text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MobileAsciiText<const N: usize> {
    bytes: [u8; N],
    len: u8,
}

impl<const N: usize> MobileAsciiText<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn push_byte(&mut self, byte: u8) {
        let index = usize::from(self.len);
        assert!(index < N, "mobile time text buffer is too small");
        self.bytes[index] = byte;
        self.len += 1;
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.push_byte(*byte);
        }
    }

    fn push_two_digits(&mut self, value: u8) {
        debug_assert!(value < 100);
        self.push_byte(b'0' + value / 10);
        self.push_byte(b'0' + value % 10);
    }

    fn push_decimal_day(&mut self, value: u8) {
        debug_assert!((1..=31).contains(&value));
        if value >= 10 {
            self.push_byte(b'0' + value / 10);
        }
        self.push_byte(b'0' + value % 10);
    }

    fn push_u32(&mut self, mut value: u32) {
        let mut reverse = [0_u8; 10];
        let mut length = 0;
        loop {
            reverse[length] = b'0' + u8::try_from(value % 10).unwrap_or(0);
            length += 1;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        for index in (0..length).rev() {
            self.push_byte(reverse[index]);
        }
    }

    fn push_u64(&mut self, mut value: u64) {
        let mut reverse = [0_u8; 20];
        let mut length = 0;
        loop {
            reverse[length] = b'0' + u8::try_from(value % 10).unwrap_or(0);
            length += 1;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        for index in (0..length).rev() {
            self.push_byte(reverse[index]);
        }
    }

    fn push_hex_prefix(&mut self, bytes: &[u8], count: usize) {
        for byte in bytes.iter().copied().take(count) {
            for nibble in [byte >> 4, byte & 0x0f] {
                self.push_byte(if nibble < 10 {
                    b'0' + nibble
                } else {
                    b'a' + nibble - 10
                });
            }
        }
    }

    fn push_hex_u32(&mut self, value: u32) {
        self.push_bytes(b"0x");
        for shift in (0..8).rev() {
            let nibble = u8::try_from((value >> (shift * 4)) & 0x0f).unwrap_or(0);
            self.push_byte(if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            });
        }
    }

    fn push_i32(&mut self, value: i32) {
        if value < 0 {
            self.push_byte(b'-');
        }
        self.push_u32(value.unsigned_abs());
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("mobile time text is always ASCII")
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MobilePage {
    Lock,
    #[default]
    Home,
    Phone,
    Messages,
    Calculator,
    /// Launcher-local, deliberately restricted DEX-0 execution demo.
    AndroidDemo,
    Settings,
    Apps,
    About,
    /// Settings-owned appearance controls and exact display facts.
    Display,
    /// Session-wide implemented text-size and contrast preferences.
    Accessibility,
}

/// Coarse, UI-facing failure class supplied by the AndroidBox runtime owner.
///
/// These variants describe only the bounded DEX-0 demo. They do not model
/// Android ART exceptions, package installation, Binder, JNI, or framework
/// behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AndroidBoxError {
    #[default]
    None,
    InvalidDex,
    VerificationFailed,
    UnsupportedOpcode,
    StepLimit,
    RuntimeTrap,
}

/// Allocation-free, printable-ASCII text supplied by the AndroidBox runtime.
///
/// The bytes are private so renderable values can only be constructed after
/// the fixed capacity, printable-ASCII, and caption-font invariants have been
/// checked. This is a UI data mirror, not an Android `String`, Java object, or
/// JNI handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidBoxBoundedText<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    len: u8,
}

impl<const CAPACITY: usize> AndroidBoxBoundedText<CAPACITY> {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; CAPACITY],
            len: 0,
        }
    }

    /// Copies one bounded value the caption renderer can reproduce exactly.
    ///
    /// Empty content is accepted because an actual TextView may be empty.
    /// Capacities above `u8::MAX` are rejected so `len` remains canonical.
    pub fn from_ascii(value: &str) -> Option<Self> {
        let source = value.as_bytes();
        if CAPACITY > u8::MAX as usize || source.len() > CAPACITY {
            return None;
        }
        let mut bytes = [0; CAPACITY];
        let mut index = 0;
        while index < source.len() {
            let byte = source[index];
            if !(b' '..=b'~').contains(&byte) {
                return None;
            }
            if !mobile_text_has_glyph(MobileTextRole::Caption, char::from(byte)) {
                return None;
            }
            bytes[index] = byte;
            index += 1;
        }
        if CAPACITY == ANDROIDBOX_ACTIVITY_IDENTITY_CAPACITY
            && measure_mobile_text_px(MobileTextRole::Caption, value)
                > ANDROIDBOX_ACTIVITY_IDENTITY_MAX_WIDTH_PX
        {
            return None;
        }
        if CAPACITY == ANDROIDBOX_TEXT_VIEW_CONTENT_CAPACITY {
            let first_end = source.len().min(ANDROIDBOX_TEXT_VIEW_FIRST_LINE_BYTES);
            let (first, second) = value.split_at(first_end);
            if measure_mobile_text_px(MobileTextRole::Caption, first)
                > ANDROIDBOX_TEXT_VIEW_LINE_MAX_WIDTH_PX
                || measure_mobile_text_px(MobileTextRole::Caption, second)
                    > ANDROIDBOX_TEXT_VIEW_LINE_MAX_WIDTH_PX
            {
                return None;
            }
        }
        Some(Self {
            bytes,
            len: source.len() as u8,
        })
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("AndroidBoxBoundedText constructors preserve printable ASCII")
    }
}

impl<const CAPACITY: usize> Default for AndroidBoxBoundedText<CAPACITY> {
    fn default() -> Self {
        Self::empty()
    }
}

/// Bounded launcher activity identity decoded and verified by the runtime.
pub type AndroidBoxActivityIdentity = AndroidBoxBoundedText<ANDROIDBOX_ACTIVITY_IDENTITY_CAPACITY>;
/// Bounded TextView content produced by the restricted framework shim.
pub type AndroidBoxTextViewContent = AndroidBoxBoundedText<ANDROIDBOX_TEXT_VIEW_CONTENT_CAPACITY>;

/// Coarse Activity-stage failure supplied by the AndroidBox runtime owner.
///
/// This intentionally does not mimic Java exceptions or claim ART/framework
/// compatibility. It only explains which bounded fixture stage failed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AndroidBoxActivityError {
    #[default]
    None,
    ManifestRejected,
    LauncherActivityMissing,
    ActivityVerificationFailed,
    UnsupportedFrameworkCall,
    StepLimit,
    RuntimeTrap,
}

/// Runtime-supplied proof for the bounded Resources-1 Activity path.
///
/// This is an allocation-free UI mirror. It does not parse `resources.arsc`,
/// inflate arbitrary layouts, resolve qualifiers, or infer any milestone from
/// the resource IDs. Every field remains false/zero until the runtime reports
/// the corresponding verified fact.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AndroidBoxResourceStatus {
    pub resource_table_parsed: bool,
    pub layout_entry_resolved: bool,
    pub binary_xml_parsed: bool,
    pub text_view_verified: bool,
    pub string_reference_resolved: bool,
    pub layout_resource_id: u32,
    pub string_resource_id: u32,
}

impl AndroidBoxResourceStatus {
    pub const fn empty() -> Self {
        Self {
            resource_table_parsed: false,
            layout_entry_resolved: false,
            binary_xml_parsed: false,
            text_view_verified: false,
            string_reference_resolved: false,
            layout_resource_id: 0,
            string_resource_id: 0,
        }
    }
}

/// Allocation-free, printable-ASCII metadata supplied by the package runtime.
///
/// The private bytes prevent an unchecked string from reaching the offline
/// raster font. This is a UI transport value, not an Android `String`, Java
/// object, package parser, or proof that an application is installed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledAscii<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    len: u8,
}

impl<const CAPACITY: usize> AndroidInstalledAscii<CAPACITY> {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; CAPACITY],
            len: 0,
        }
    }

    /// Copies one strictly printable-ASCII value with explicit glyph support.
    pub fn from_ascii(value: &str) -> Option<Self> {
        let source = value.as_bytes();
        if CAPACITY > u8::MAX as usize || source.len() > CAPACITY {
            return None;
        }
        let mut bytes = [0; CAPACITY];
        for (index, byte) in source.iter().copied().enumerate() {
            if !(b' '..=b'~').contains(&byte)
                || !mobile_text_has_glyph(MobileTextRole::Label, char::from(byte))
                || !mobile_text_has_glyph(MobileTextRole::Caption, char::from(byte))
                || !mobile_text_has_glyph(MobileTextRole::Body, char::from(byte))
            {
                return None;
            }
            bytes[index] = byte;
        }
        Some(Self {
            bytes,
            len: source.len() as u8,
        })
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("AndroidInstalledAscii constructors preserve printable ASCII")
    }
}

impl<const CAPACITY: usize> Default for AndroidInstalledAscii<CAPACITY> {
    fn default() -> Self {
        Self::empty()
    }
}

pub type AndroidInstalledPackage = AndroidInstalledAscii<ANDROID_INSTALLED_PACKAGE_CAPACITY>;
pub type AndroidInstalledActivity = AndroidInstalledAscii<ANDROID_INSTALLED_ACTIVITY_CAPACITY>;
pub type AndroidInstalledTitle = AndroidInstalledAscii<ANDROID_INSTALLED_TITLE_CAPACITY>;
pub type AndroidInstalledText = AndroidInstalledAscii<ANDROID_INSTALLED_TEXT_CAPACITY>;

#[cfg(feature = "androidbox-interactive0")]
pub const ANDROID_INSTALLED_VIEW_LABEL_CAPACITY: usize = 96;
#[cfg(feature = "androidbox-interactive0")]
pub const ANDROID_INSTALLED_VIEW_BUTTON_CAPACITY: usize = 64;
#[cfg(feature = "androidbox-interactive0")]
pub type AndroidInstalledViewLabel = AndroidInstalledAscii<ANDROID_INSTALLED_VIEW_LABEL_CAPACITY>;
#[cfg(feature = "androidbox-interactive0")]
pub type AndroidInstalledViewButton = AndroidInstalledAscii<ANDROID_INSTALLED_VIEW_BUTTON_CAPACITY>;

/// One allocation-free Resources view snapshot admitted by InteractiveActivity-1.
///
/// IDs and revision are nonzero, the TextView and Button IDs are distinct, and
/// text is bounded printable ASCII. A callback may be staged as unregistered,
/// but such a state is not active and never exposes a touch target.
#[cfg(feature = "androidbox-interactive0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledActivityViewState {
    label_view_id: u32,
    label_text: AndroidInstalledViewLabel,
    button_view_id: u32,
    button_text: AndroidInstalledViewButton,
    callback_registered: bool,
    revision: u64,
}

#[cfg(feature = "androidbox-interactive0")]
impl AndroidInstalledActivityViewState {
    pub const fn empty() -> Self {
        Self {
            label_view_id: 0,
            label_text: AndroidInstalledViewLabel::empty(),
            button_view_id: 0,
            button_text: AndroidInstalledViewButton::empty(),
            callback_registered: false,
            revision: 0,
        }
    }

    pub fn try_new(
        label_view_id: u32,
        label_text: &str,
        button_view_id: u32,
        button_text: &str,
        callback_registered: bool,
        revision: u64,
    ) -> Option<Self> {
        let label_text = AndroidInstalledViewLabel::from_ascii(label_text)?;
        let button_text = AndroidInstalledViewButton::from_ascii(button_text)?;
        if label_view_id == 0
            || button_view_id == 0
            || label_view_id == button_view_id
            || button_text.is_empty()
            || revision == 0
        {
            return None;
        }
        Some(Self {
            label_view_id,
            label_text,
            button_view_id,
            button_text,
            callback_registered,
            revision,
        })
    }

    pub const fn label_view_id(self) -> u32 {
        self.label_view_id
    }

    pub fn label_text(&self) -> &str {
        self.label_text.as_str()
    }

    pub const fn button_view_id(self) -> u32 {
        self.button_view_id
    }

    pub fn button_text(&self) -> &str {
        self.button_text.as_str()
    }

    pub const fn callback_registered(self) -> bool {
        self.callback_registered
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn is_active(self) -> bool {
        self.revision != 0 && self.callback_registered
    }
}

#[cfg(feature = "androidbox-interactive0")]
impl Default for AndroidInstalledActivityViewState {
    fn default() -> Self {
        Self::empty()
    }
}

/// Stack-bounded retained form of one decoded APK launcher icon.
///
/// The public ABI transports all 256 canonical ARGB pixels. The UI admits at
/// most 16 distinct colors and packs two palette indices per byte so every
/// process model remains safely below its fixed EL0 stack budget.
#[cfg(feature = "androidbox-icon-resources5")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledIcon {
    pub resource_id: u32,
    pub png_crc32: u32,
    palette: [u32; ANDROID_INSTALLED_ICON_PALETTE_CAPACITY],
    palette_len: u8,
    indices: [u8; ANDROID_INSTALLED_ICON_PACKED_BYTES],
}

#[cfg(feature = "androidbox-icon-resources5")]
impl AndroidInstalledIcon {
    pub fn try_new(
        resource_id: u32,
        png_crc32: u32,
        pixels: [u32; ANDROID_INSTALLED_ICON_PIXEL_COUNT],
    ) -> Option<Self> {
        if resource_id == 0 || png_crc32 == 0 || !pixels.iter().any(|pixel| pixel >> 24 != 0) {
            return None;
        }
        let mut palette = [0; ANDROID_INSTALLED_ICON_PALETTE_CAPACITY];
        let mut palette_len = 0usize;
        let mut indices = [0; ANDROID_INSTALLED_ICON_PACKED_BYTES];
        for (index, pixel) in pixels.iter().copied().enumerate() {
            if pixel >> 24 == 0 && pixel & 0x00ff_ffff != 0 {
                return None;
            }
            let palette_index = if let Some(existing) = palette[..palette_len]
                .iter()
                .position(|value| *value == pixel)
            {
                existing
            } else {
                if palette_len == palette.len() {
                    return None;
                }
                palette[palette_len] = pixel;
                palette_len += 1;
                palette_len - 1
            };
            let packed = &mut indices[index / 2];
            if index.is_multiple_of(2) {
                *packed = (palette_index as u8) << 4;
            } else {
                *packed |= palette_index as u8;
            }
        }
        Some(Self {
            resource_id,
            png_crc32,
            palette,
            palette_len: palette_len as u8,
            indices,
        })
    }

    pub fn pixel(self, index: usize) -> Option<u32> {
        if index >= ANDROID_INSTALLED_ICON_PIXEL_COUNT {
            return None;
        }
        let packed = self.indices[index / 2];
        let palette_index = if index.is_multiple_of(2) {
            packed >> 4
        } else {
            packed & 0x0f
        };
        (usize::from(palette_index) < usize::from(self.palette_len))
            .then_some(self.palette[usize::from(palette_index)])
    }
}

/// Complete package-catalog snapshot supplied by the Android package runtime.
///
/// `installed` is the sole installation authority consumed by the renderer.
/// Opening a page or pressing a launcher icon never changes this value. The
/// generation is also embedded in installed-app touch targets so a catalog
/// replacement between press and release fails closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledAppStatus {
    pub installed: bool,
    pub generation: u64,
    pub version_code: u32,
    pub apk_length: u32,
    pub package: AndroidInstalledPackage,
    pub activity: AndroidInstalledActivity,
    pub title: AndroidInstalledTitle,
    pub text: AndroidInstalledText,
    pub signer_digest_sha256: [u8; 32],
    pub apk_digest_sha256: [u8; 32],
    #[cfg(feature = "androidbox-icon-resources5")]
    pub icon: Option<AndroidInstalledIcon>,
}

impl AndroidInstalledAppStatus {
    pub const fn empty() -> Self {
        Self {
            installed: false,
            generation: 0,
            version_code: 0,
            apk_length: 0,
            package: AndroidInstalledPackage::empty(),
            activity: AndroidInstalledActivity::empty(),
            title: AndroidInstalledTitle::empty(),
            text: AndroidInstalledText::empty(),
            signer_digest_sha256: [0; 32],
            apk_digest_sha256: [0; 32],
            #[cfg(feature = "androidbox-icon-resources5")]
            icon: None,
        }
    }

    /// Constructs one internally consistent installed-package snapshot.
    ///
    /// Cryptographic verification and package-store recovery remain runtime
    /// responsibilities. This constructor only enforces the UI's fixed-size,
    /// printable-ASCII and non-empty installed identity invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        generation: u64,
        version_code: u32,
        apk_length: u32,
        package: &str,
        activity: &str,
        title: &str,
        text: &str,
        signer_digest_sha256: [u8; 32],
        apk_digest_sha256: [u8; 32],
    ) -> Option<Self> {
        let package = AndroidInstalledPackage::from_ascii(package)?;
        let activity = AndroidInstalledActivity::from_ascii(activity)?;
        let title = AndroidInstalledTitle::from_ascii(title)?;
        let text = AndroidInstalledText::from_ascii(text)?;
        if generation == 0
            || apk_length == 0
            || package.is_empty()
            || activity.is_empty()
            || title.is_empty()
        {
            return None;
        }
        Some(Self {
            installed: true,
            generation,
            version_code,
            apk_length,
            package,
            activity,
            title,
            text,
            signer_digest_sha256,
            apk_digest_sha256,
            #[cfg(feature = "androidbox-icon-resources5")]
            icon: None,
        })
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    pub fn with_icon(mut self, icon: Option<AndroidInstalledIcon>) -> Option<Self> {
        if !self.installed {
            return None;
        }
        self.icon = icon;
        Some(self)
    }
}

impl Default for AndroidInstalledAppStatus {
    fn default() -> Self {
        Self::empty()
    }
}

/// Runtime-owned outcome for one installed-APK relaunch request.
///
/// Every non-idle variant carries only a compact request token. Publisher
/// title and TextView bytes remain in the separate package catalog snapshot,
/// so starting or completing a request never duplicates those large bounded
/// strings inside `MobileModel`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidInstalledLaunchStatus {
    Idle,
    Pending {
        request_sequence: u64,
        generation: u64,
        apk_sha256: [u8; 32],
    },
    Succeeded {
        request_sequence: u64,
        generation: u64,
        apk_sha256: [u8; 32],
    },
    Failed {
        request_sequence: u64,
        generation: u64,
        apk_sha256: [u8; 32],
        failure: AndroidInstalledLaunchFailure,
    },
}

impl AndroidInstalledLaunchStatus {
    pub const fn idle() -> Self {
        Self::Idle
    }

    fn matches_catalog(&self, catalog: &AndroidInstalledAppStatus) -> bool {
        let (generation, apk_sha256) = match self {
            Self::Idle => return false,
            Self::Pending {
                generation,
                apk_sha256,
                ..
            }
            | Self::Succeeded {
                generation,
                apk_sha256,
                ..
            }
            | Self::Failed {
                generation,
                apk_sha256,
                ..
            } => (*generation, apk_sha256),
        };
        catalog.installed
            && generation == catalog.generation
            && apk_sha256 == &catalog.apk_digest_sha256
    }
}

impl Default for AndroidInstalledLaunchStatus {
    fn default() -> Self {
        Self::idle()
    }
}

/// Launcher-owned binding between one verified package and the boot-local
/// compatible-Activity identity retained by SurfaceServer.
///
/// This stores no title, TextView bytes, Activity pixels, process handle, or
/// background-execution state. Foreground/Recent is derived exclusively from
/// the latest accepted System UI snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledActivitySession {
    identity: UiCompatibleActivityIdentity,
    apk_sha256: [u8; 32],
}

impl AndroidInstalledActivitySession {
    pub const fn identity(self) -> UiCompatibleActivityIdentity {
        self.identity
    }

    pub const fn apk_sha256(self) -> [u8; 32] {
        self.apk_sha256
    }

    fn matches_catalog(&self, catalog: &AndroidInstalledAppStatus) -> bool {
        catalog.installed
            && self.identity.package_generation() == catalog.generation
            && self.apk_sha256 == catalog.apk_digest_sha256
    }
}

/// Bounded failure class supplied by the installed-APK relaunch owner.
///
/// These values do not model Java exceptions and make no ART or general
/// Android compatibility claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AndroidInstalledLaunchFailure {
    Stale,
    Verification,
    Unsupported,
    Unavailable,
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidInstalledUninstallFailure {
    Stale,
    Busy,
    Storage,
    Verification,
}

/// Settings-owned presentation state for one exact installed-package removal.
///
/// Confirmation is local UI state. `Pending`, `Removed`, and `Failed` are set
/// only by the runtime after invoking ABI 52; the renderer never infers a
/// durable result from a button press.
#[cfg(feature = "androidbox-runtime-uninstall1")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AndroidInstalledUninstallStatus {
    #[default]
    Idle,
    Confirming {
        generation: u64,
    },
    Pending {
        request_sequence: u64,
        generation: u64,
    },
    Removed {
        request_sequence: u64,
        removal_generation: u64,
    },
    Failed {
        request_sequence: u64,
        generation: u64,
        failure: AndroidInstalledUninstallFailure,
    },
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
impl AndroidInstalledUninstallStatus {
    pub const fn blocks_apps_page(self) -> bool {
        matches!(
            self,
            Self::Confirming { .. } | Self::Pending { .. } | Self::Failed { .. }
        )
    }

    pub const fn generation(self) -> Option<u64> {
        match self {
            Self::Confirming { generation }
            | Self::Pending { generation, .. }
            | Self::Failed { generation, .. } => Some(generation),
            Self::Idle | Self::Removed { .. } => None,
        }
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidInstallCandidateAction {
    Install,
    Update,
    Reinstall,
}

/// Settings-facing, authority-free mirror of one fully admitted APK source.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstallCandidateStatus {
    pub present: bool,
    pub candidate_id: u64,
    pub action: AndroidInstallCandidateAction,
    pub expected_generation: u64,
    pub expected_version_code: u32,
    pub version_code: u32,
    pub apk_length: u32,
    pub package: AndroidInstalledPackage,
    pub activity: AndroidInstalledActivity,
    pub title: AndroidInstalledTitle,
    pub text: AndroidInstalledText,
    pub signer_digest_sha256: [u8; 32],
    pub apk_digest_sha256: [u8; 32],
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidInstallCandidateStatus {
    pub const fn empty() -> Self {
        Self {
            present: false,
            candidate_id: 0,
            action: AndroidInstallCandidateAction::Install,
            expected_generation: 0,
            expected_version_code: 0,
            version_code: 0,
            apk_length: 0,
            package: AndroidInstalledPackage::empty(),
            activity: AndroidInstalledActivity::empty(),
            title: AndroidInstalledTitle::empty(),
            text: AndroidInstalledText::empty(),
            signer_digest_sha256: [0; 32],
            apk_digest_sha256: [0; 32],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        candidate_id: u64,
        action: AndroidInstallCandidateAction,
        expected_generation: u64,
        expected_version_code: u32,
        version_code: u32,
        apk_length: u32,
        package: &str,
        activity: &str,
        title: &str,
        text: &str,
        signer_digest_sha256: [u8; 32],
        apk_digest_sha256: [u8; 32],
    ) -> Option<Self> {
        let package = AndroidInstalledPackage::from_ascii(package)?;
        let activity = AndroidInstalledActivity::from_ascii(activity)?;
        let title = AndroidInstalledTitle::from_ascii(title)?;
        let text = AndroidInstalledText::from_ascii(text)?;
        let state_valid = match action {
            AndroidInstallCandidateAction::Install => {
                expected_generation == 0 && expected_version_code == 0
            }
            AndroidInstallCandidateAction::Update => {
                expected_generation != 0
                    && expected_version_code != 0
                    && version_code > expected_version_code
            }
            AndroidInstallCandidateAction::Reinstall => {
                expected_generation != 0
                    && expected_version_code != 0
                    && version_code >= expected_version_code
            }
        };
        if candidate_id == 0
            || !state_valid
            || version_code == 0
            || apk_length == 0
            || package.is_empty()
            || activity.is_empty()
            || title.is_empty()
            || signer_digest_sha256.iter().all(|byte| *byte == 0)
            || apk_digest_sha256.iter().all(|byte| *byte == 0)
        {
            return None;
        }
        Some(Self {
            present: true,
            candidate_id,
            action,
            expected_generation,
            expected_version_code,
            version_code,
            apk_length,
            package,
            activity,
            title,
            text,
            signer_digest_sha256,
            apk_digest_sha256,
        })
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
impl Default for AndroidInstallCandidateStatus {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidInstallFailure {
    Stale,
    Busy,
    Storage,
    Verification,
    Unsupported,
}

/// Settings-owned two-step state for one exact install/update candidate.
#[cfg(feature = "androidbox-runtime-install2")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AndroidInstallStatus {
    #[default]
    Idle,
    Confirming {
        candidate_id: u64,
    },
    Pending {
        request_sequence: u64,
        candidate_id: u64,
    },
    Installed {
        request_sequence: u64,
        candidate_id: u64,
        generation: u64,
    },
    Failed {
        request_sequence: u64,
        candidate_id: u64,
        failure: AndroidInstallFailure,
    },
}

#[cfg(feature = "androidbox-runtime-install2")]
impl AndroidInstallStatus {
    pub const fn blocks_apps_page(self) -> bool {
        matches!(
            self,
            Self::Confirming { .. } | Self::Pending { .. } | Self::Failed { .. }
        )
    }

    pub const fn candidate_id(self) -> Option<u64> {
        match self {
            Self::Confirming { candidate_id }
            | Self::Pending { candidate_id, .. }
            | Self::Installed { candidate_id, .. }
            | Self::Failed { candidate_id, .. } => Some(candidate_id),
            Self::Idle => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageView {
    #[default]
    Inbox,
    PreviewGuide,
    OfflineStatus,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CalculatorOperation {
    #[default]
    None,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalculatorKey {
    Clear,
    Sign,
    Percent,
    Operation(CalculatorOperation),
    Digit(u8),
    Decimal,
    Equals,
    Backspace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobilePressedTarget {
    SystemHome,
    OverviewRecent,
    OverviewBackground,
    Phone,
    Messages,
    Calculator,
    Settings,
    Back,
    Display,
    Accessibility,
    Theme,
    Accent,
    SoftwareDimming,
    LargeText,
    HighContrast,
    BootNotification,
    Apps,
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    AppsUninstall,
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    AppsUninstallCancel,
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    AppsUninstallConfirm,
    #[cfg(feature = "androidbox-runtime-install2")]
    AppsInstall,
    #[cfg(feature = "androidbox-runtime-install2")]
    AppsInstallCancel,
    #[cfg(feature = "androidbox-runtime-install2")]
    AppsInstallConfirm,
    #[cfg(feature = "androidbox-multipackage4")]
    AppsInstalledAndroid(u8),
    About,
    DrawerPhone,
    DrawerMessages,
    DrawerCalculator,
    DrawerSettings,
    DrawerAndroidBoxDemo,
    DrawerInstalledAndroid(u64),
    DrawerHandle,
    DrawerHome,
    AndroidBoxExecute,
    #[cfg(feature = "androidbox-interactive0")]
    InstalledAndroidButton(u32),
    ShadeClose,
    PhoneKey(u8),
    PhoneBackspace,
    CalculatorKey(u8),
    MessageGuide,
    MessageOffline,
}

/// Exact redraw scope for one transition between two mobile render models.
///
/// The planner deliberately recognizes only transient pressed-state changes.
/// Every semantic, layout, overlay, clock, appearance, or runtime-content
/// change remains a full-frame update until it has its own independently
/// verified damage contract.
#[cfg(feature = "mobile-ui-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileDamagePlan {
    Unchanged,
    Regions(DamageRegions),
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MobileModel {
    pub page: MobilePage,
    /// Session-wide civil-time snapshot. Unavailable is rendered honestly;
    /// the model never falls back to a build-time or demonstration date.
    pub time: MobileTimeSnapshot,
    pub dark_theme: bool,
    pub alternate_accent: bool,
    /// Session-local final-surface dimming shared through SurfaceServer.
    ///
    /// This changes rendered RGB values only and must not be interpreted as
    /// physical backlight, panel-brightness, power, or persistence state.
    pub software_dimming: UiSoftwareDimming,
    /// Session-wide two-stop semantic type scale shared through SurfaceServer.
    pub large_text: bool,
    /// Session-wide stronger text/outline contrast shared through SurfaceServer.
    pub high_contrast: bool,
    /// SurfaceServer-session mirror of one fixed boot-local System UI notice.
    ///
    /// This is not a general notification, application-posting, push, sound,
    /// vibration, delivery-time, or persistence capability.
    pub boot_notification_visible: bool,
    /// Client-local disclosure state for the fixed notice body.
    pub boot_notification_expanded: bool,
    /// Transient, quantized horizontal card displacement while a contact is
    /// captured. Visibility remains server-owned and commits separately.
    pub boot_notification_offset_px: i16,
    /// SurfaceServer-owned system-UI mode mirrored into this client.
    ///
    /// Overview is a bounded identity-card surface for the last genuinely
    /// focused Shell app or freshly verified compatible Activity. It is not a
    /// task manager or background-execution claim.
    pub system_ui_mode: UiSystemUiMode,
    /// Capacity-one recent identity mirrored from SurfaceServer.
    pub system_ui_recent: Option<UiRecentIdentity>,
    /// Transient upward finger-follow displacement of the Overview card.
    ///
    /// The SurfaceServer-owned recent identity is unchanged until a separate
    /// authenticated `DismissRecent` request commits. This value grants no
    /// lifecycle, process, package, or storage authority.
    pub overview_recent_offset_px: u16,
    /// Authoritative system-navigation contact state. Raw samples beginning in
    /// the bottom system region are consumed by SurfaceServer, not by apps.
    pub system_nav_pressed: bool,
    /// Quantized upward travel supplied by SurfaceServer for deterministic
    /// Overview finger-follow rendering.
    pub system_nav_reveal_px: u16,
    /// Last contiguous SurfaceServer system-UI revision accepted by runtime.
    pub system_ui_revision: u64,
    pub shade_open: bool,
    /// Transient visible panel extent in physical scanout pixels.
    ///
    /// Zero means that no drag override is active, so `shade_open` selects the
    /// stable fully-open or fully-closed state. Non-zero values are bounded by
    /// `SHADE_REVEAL_MAX` when applied through `MobileModel::apply`.
    pub shade_reveal_px: u16,
    pub drawer_open: bool,
    /// Transient All apps extent in physical scanout pixels.
    ///
    /// Zero leaves `drawer_open` authoritative. Non-zero values are bounded
    /// and quantized when applied, keeping finger-follow frame production
    /// deterministic without claiming a timer-driven animation.
    pub drawer_reveal_px: u16,
    /// Quantized transient left-edge Back feedback in physical scanout pixels.
    ///
    /// This is contact-follow feedback only. A semantic Back action is
    /// committed separately on release after the fixed travel threshold.
    pub back_reveal_px: u16,
    /// Physical contact origin used to keep the Back cue under the finger.
    pub back_origin_y: u16,
    /// Quantized upward displacement of the local visual lock surface.
    ///
    /// This is a navigation preview only and does not claim authentication,
    /// credential verification, secure storage, or a hardware-backed lock.
    pub unlock_reveal_px: u16,
    /// Deterministic vertical offset of the active full-page surface.
    ///
    /// A non-zero value renders the real Home below the moving page. The
    /// runtime owns the fixed sequence and returns this field to zero for the
    /// exact stable frame. It does not encode wall-clock time, FPS, or vsync.
    pub page_transition_offset_px: u16,
    /// Bounded physical-pixel scroll offset for the Settings content viewport.
    ///
    /// The app header and both trusted chrome regions remain fixed. The value
    /// is session-local UI state and is clamped/quantized before rendering.
    pub settings_scroll_offset_px: u16,
    /// Bounded physical-pixel scroll offset for the Apps content viewport.
    ///
    /// Package metadata remains read-only; scrolling grants no package,
    /// storage, installation, or Android runtime authority.
    pub apps_scroll_offset_px: u16,
    /// Session-local dialer input. It is never sent to a telephony service.
    pub phone_digits: [u8; PHONE_DIGIT_CAPACITY],
    pub phone_digit_count: u8,
    /// Bounded, session-local integer calculator state. It has no service,
    /// storage, network, or product-ABI dependency.
    pub calculator_value: i64,
    pub calculator_accumulator: i64,
    pub calculator_pending: CalculatorOperation,
    pub calculator_entering: bool,
    pub calculator_decimal_entered: bool,
    pub calculator_fraction_digits: u8,
    pub calculator_negative_zero: bool,
    pub calculator_error: bool,
    /// Local read-only Messages navigation state.
    pub message_view: MessageView,
    /// Runtime-supplied verification result for the restricted DEX-0 demo.
    ///
    /// The renderer never derives this value from a button press.
    pub androidbox_verified: bool,
    /// Runtime-supplied value returned by the latest bounded entry point.
    pub androidbox_boot_value: i32,
    /// Runtime-supplied number of interpreter steps completed.
    pub androidbox_step_count: u32,
    /// Runtime-supplied number of successfully executed tap entry points.
    pub androidbox_tap_count: u32,
    /// Runtime-supplied coarse failure class.
    pub androidbox_error: AndroidBoxError,
    /// Runtime-supplied proof that the APK manifest passed the restricted
    /// Activity-stage parser. A page open never changes this value.
    pub androidbox_manifest_verified: bool,
    /// Runtime-supplied launcher component identity from the verified
    /// MAIN/LAUNCHER manifest entry.
    pub androidbox_launcher_activity: AndroidBoxActivityIdentity,
    /// Runtime-supplied lifecycle milestone for the bounded Activity shim.
    pub androidbox_on_create_completed: bool,
    /// Runtime-supplied TextView content installed by the completed Activity.
    pub androidbox_text_view_content: AndroidBoxTextViewContent,
    /// Runtime-supplied number of Activity interpreter instructions.
    pub androidbox_activity_instruction_count: u32,
    /// Runtime-supplied Resources-1 parser and resolver facts.
    pub androidbox_resources: AndroidBoxResourceStatus,
    /// Runtime-supplied coarse Activity-stage failure.
    pub androidbox_activity_error: AndroidBoxActivityError,
    /// Runtime-supplied package-catalog snapshot.
    ///
    /// The renderer presents an installed identity only when this explicit
    /// snapshot says `installed`; no button, page, or AndroidBox demo result
    /// is treated as installation evidence.
    pub android_installed_app: AndroidInstalledAppStatus,
    /// ABI 55's packed installed-app directory in stable package-volume order.
    #[cfg(feature = "androidbox-multipackage4")]
    pub android_installed_apps: [AndroidInstalledAppStatus; ANDROID_INSTALLED_DIRECTORY_CAPACITY],
    #[cfg(feature = "androidbox-multipackage4")]
    pub android_installed_app_count: u8,
    #[cfg(feature = "androidbox-multipackage4")]
    pub android_installed_directory_revision: u64,
    /// Runtime-uninstall confirmation and terminal status for Settings/Apps.
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    pub android_installed_uninstall: AndroidInstalledUninstallStatus,
    /// Boot-local install/update candidate and its Settings confirmation.
    #[cfg(feature = "androidbox-runtime-install2")]
    pub android_install_candidate: AndroidInstallCandidateStatus,
    #[cfg(feature = "androidbox-runtime-install2")]
    pub android_install: AndroidInstallStatus,
    /// Runtime-owned proof for the most recent installed-APK relaunch.
    ///
    /// This token is deliberately separate from the package catalog. Only a
    /// matching `Succeeded` token authorizes publisher TextView rendering.
    pub android_installed_launch: AndroidInstalledLaunchStatus,
    /// Compact Launcher-owned binding for one compatible Activity session.
    ///
    /// SurfaceServer remains authoritative for whether the identity is
    /// foreground or merely recent.
    pub android_installed_session: Option<AndroidInstalledActivitySession>,
    /// Bounded publisher view state for the current freshly verified Activity.
    #[cfg(feature = "androidbox-interactive0")]
    pub android_installed_activity_view: AndroidInstalledActivityViewState,
    /// Complete bounded View tree published through ABI 49 Scene-RPC-2.
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub android_installed_activity_scene: AndroidInstalledActivitySceneState,
    /// The currently armed interactive surface, used only for transient
    /// contact feedback. Semantic actions still occur on a matching release.
    pub pressed_target: Option<MobilePressedTarget>,
}

impl Default for MobileModel {
    fn default() -> Self {
        Self {
            page: MobilePage::Home,
            time: MobileTimeSnapshot::unavailable(),
            dark_theme: true,
            alternate_accent: false,
            software_dimming: UiSoftwareDimming::Off,
            large_text: false,
            high_contrast: false,
            boot_notification_visible: true,
            boot_notification_expanded: false,
            boot_notification_offset_px: 0,
            system_ui_mode: UiSystemUiMode::Home,
            system_ui_recent: None,
            overview_recent_offset_px: 0,
            system_nav_pressed: false,
            system_nav_reveal_px: 0,
            system_ui_revision: 1,
            shade_open: false,
            shade_reveal_px: 0,
            drawer_open: false,
            drawer_reveal_px: 0,
            back_reveal_px: 0,
            back_origin_y: 0,
            unlock_reveal_px: 0,
            page_transition_offset_px: 0,
            settings_scroll_offset_px: 0,
            apps_scroll_offset_px: 0,
            phone_digits: [0; PHONE_DIGIT_CAPACITY],
            phone_digit_count: 0,
            calculator_value: 0,
            calculator_accumulator: 0,
            calculator_pending: CalculatorOperation::None,
            calculator_entering: false,
            calculator_decimal_entered: false,
            calculator_fraction_digits: 0,
            calculator_negative_zero: false,
            calculator_error: false,
            message_view: MessageView::Inbox,
            androidbox_verified: false,
            androidbox_boot_value: 0,
            androidbox_step_count: 0,
            androidbox_tap_count: 0,
            androidbox_error: AndroidBoxError::None,
            androidbox_manifest_verified: false,
            androidbox_launcher_activity: AndroidBoxActivityIdentity::empty(),
            androidbox_on_create_completed: false,
            androidbox_text_view_content: AndroidBoxTextViewContent::empty(),
            androidbox_activity_instruction_count: 0,
            androidbox_resources: AndroidBoxResourceStatus::empty(),
            androidbox_activity_error: AndroidBoxActivityError::None,
            android_installed_app: AndroidInstalledAppStatus::empty(),
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_apps: [AndroidInstalledAppStatus::empty();
                ANDROID_INSTALLED_DIRECTORY_CAPACITY],
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_app_count: 0,
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_directory_revision: 0,
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            android_installed_uninstall: AndroidInstalledUninstallStatus::Idle,
            #[cfg(feature = "androidbox-runtime-install2")]
            android_install_candidate: AndroidInstallCandidateStatus::empty(),
            #[cfg(feature = "androidbox-runtime-install2")]
            android_install: AndroidInstallStatus::Idle,
            android_installed_launch: AndroidInstalledLaunchStatus::Idle,
            android_installed_session: None,
            #[cfg(feature = "androidbox-interactive0")]
            android_installed_activity_view: AndroidInstalledActivityViewState::empty(),
            #[cfg(feature = "androidbox-scene-rpc2")]
            android_installed_activity_scene: AndroidInstalledActivitySceneState::empty(),
            pressed_target: None,
        }
    }
}

impl MobileModel {
    pub const fn for_page(page: MobilePage) -> Self {
        Self {
            page,
            time: MobileTimeSnapshot::unavailable(),
            dark_theme: true,
            alternate_accent: false,
            software_dimming: UiSoftwareDimming::Off,
            large_text: false,
            high_contrast: false,
            boot_notification_visible: true,
            boot_notification_expanded: false,
            boot_notification_offset_px: 0,
            system_ui_mode: if matches!(page, MobilePage::Lock) {
                UiSystemUiMode::Locked
            } else if matches!(
                page,
                MobilePage::Home | MobilePage::Calculator | MobilePage::AndroidDemo
            ) {
                UiSystemUiMode::Home
            } else {
                UiSystemUiMode::Foreground
            },
            system_ui_recent: None,
            overview_recent_offset_px: 0,
            system_nav_pressed: false,
            system_nav_reveal_px: 0,
            system_ui_revision: 1,
            shade_open: false,
            shade_reveal_px: 0,
            drawer_open: false,
            drawer_reveal_px: 0,
            back_reveal_px: 0,
            back_origin_y: 0,
            unlock_reveal_px: 0,
            page_transition_offset_px: 0,
            settings_scroll_offset_px: 0,
            apps_scroll_offset_px: 0,
            phone_digits: [0; PHONE_DIGIT_CAPACITY],
            phone_digit_count: 0,
            calculator_value: 0,
            calculator_accumulator: 0,
            calculator_pending: CalculatorOperation::None,
            calculator_entering: false,
            calculator_decimal_entered: false,
            calculator_fraction_digits: 0,
            calculator_negative_zero: false,
            calculator_error: false,
            message_view: MessageView::Inbox,
            androidbox_verified: false,
            androidbox_boot_value: 0,
            androidbox_step_count: 0,
            androidbox_tap_count: 0,
            androidbox_error: AndroidBoxError::None,
            androidbox_manifest_verified: false,
            androidbox_launcher_activity: AndroidBoxActivityIdentity::empty(),
            androidbox_on_create_completed: false,
            androidbox_text_view_content: AndroidBoxTextViewContent::empty(),
            androidbox_activity_instruction_count: 0,
            androidbox_resources: AndroidBoxResourceStatus::empty(),
            androidbox_activity_error: AndroidBoxActivityError::None,
            android_installed_app: AndroidInstalledAppStatus::empty(),
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_apps: [AndroidInstalledAppStatus::empty();
                ANDROID_INSTALLED_DIRECTORY_CAPACITY],
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_app_count: 0,
            #[cfg(feature = "androidbox-multipackage4")]
            android_installed_directory_revision: 0,
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            android_installed_uninstall: AndroidInstalledUninstallStatus::Idle,
            #[cfg(feature = "androidbox-runtime-install2")]
            android_install_candidate: AndroidInstallCandidateStatus::empty(),
            #[cfg(feature = "androidbox-runtime-install2")]
            android_install: AndroidInstallStatus::Idle,
            android_installed_launch: AndroidInstalledLaunchStatus::Idle,
            android_installed_session: None,
            #[cfg(feature = "androidbox-interactive0")]
            android_installed_activity_view: AndroidInstalledActivityViewState::empty(),
            #[cfg(feature = "androidbox-scene-rpc2")]
            android_installed_activity_scene: AndroidInstalledActivitySceneState::empty(),
            pressed_target: None,
        }
    }

    pub const fn accent(&self) -> u32 {
        mobile_theme_tokens(self.dark_theme, self.alternate_accent, self.high_contrast).accent
    }

    pub const fn on_accent(&self) -> u32 {
        mobile_theme_tokens(self.dark_theme, self.alternate_accent, self.high_contrast).on_accent
    }

    const fn theme_tokens(&self) -> MobileThemeTokens {
        mobile_theme_tokens(self.dark_theme, self.alternate_accent, self.high_contrast)
    }

    const fn surface_color(&self, raised: bool) -> u32 {
        let theme = self.theme_tokens();
        if raised {
            theme.surface_raised
        } else {
            theme.surface
        }
    }

    const fn outline_color(&self) -> u32 {
        self.theme_tokens().outline
    }

    const fn text_primary(&self) -> u32 {
        self.theme_tokens().text_primary
    }

    const fn text_secondary(&self) -> u32 {
        self.theme_tokens().text_secondary
    }

    const fn text_tertiary(&self) -> u32 {
        self.theme_tokens().text_tertiary
    }

    pub const fn locked() -> Self {
        Self::for_page(MobilePage::Lock)
    }

    pub fn is_pressed(&self, target: MobilePressedTarget) -> bool {
        self.pressed_target == Some(target)
    }

    /// Replaces the complete AndroidBox result with runtime-verified data.
    ///
    /// This is intentionally a pure data mirror: it does not execute DEX,
    /// infer verification, increment the tap count, navigate, or alter
    /// transient input state. The runtime owner must call it after a real
    /// bounded execution attempt.
    pub fn apply_androidbox_result(
        &mut self,
        verified: bool,
        boot_value: i32,
        step_count: u32,
        tap_count: u32,
        error: AndroidBoxError,
    ) -> bool {
        let changed = self.androidbox_verified != verified
            || self.androidbox_boot_value != boot_value
            || self.androidbox_step_count != step_count
            || self.androidbox_tap_count != tap_count
            || self.androidbox_error != error;
        self.androidbox_verified = verified;
        self.androidbox_boot_value = boot_value;
        self.androidbox_step_count = step_count;
        self.androidbox_tap_count = tap_count;
        self.androidbox_error = error;
        changed
    }

    /// Replaces the complete AndroidBox Activity result with runtime data.
    ///
    /// The UI does not parse a manifest, select a launcher component, execute
    /// `onCreate`, parse a resource table or binary XML layout, construct a
    /// TextView, resolve a string reference, count instructions, or infer
    /// success. The runtime owner calls this only after a bounded Activity
    /// attempt. The values are mirrored exactly; render-time success
    /// additionally requires every positive milestone and no error, so
    /// contradictory input cannot be presented as a completed launch.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_androidbox_activity_result(
        &mut self,
        manifest_verified: bool,
        launcher_activity: AndroidBoxActivityIdentity,
        on_create_completed: bool,
        text_view_content: AndroidBoxTextViewContent,
        instruction_count: u32,
        resources: AndroidBoxResourceStatus,
        error: AndroidBoxActivityError,
    ) -> bool {
        let changed = self.androidbox_manifest_verified != manifest_verified
            || self.androidbox_launcher_activity != launcher_activity
            || self.androidbox_on_create_completed != on_create_completed
            || self.androidbox_text_view_content != text_view_content
            || self.androidbox_activity_instruction_count != instruction_count
            || self.androidbox_resources != resources
            || self.androidbox_activity_error != error;
        self.androidbox_manifest_verified = manifest_verified;
        self.androidbox_launcher_activity = launcher_activity;
        self.androidbox_on_create_completed = on_create_completed;
        self.androidbox_text_view_content = text_view_content;
        self.androidbox_activity_instruction_count = instruction_count;
        self.androidbox_resources = resources;
        self.androidbox_activity_error = error;
        changed
    }

    /// Replaces the complete installed-app catalog mirror with runtime data.
    ///
    /// This method performs no verification, persistence, installation,
    /// launch, navigation, or input mutation. In particular, it never derives
    /// installation state from AndroidBox execution results.
    #[cfg(feature = "androidbox-multipackage4")]
    pub fn apply_android_installed_directory_status(
        &mut self,
        apps: [AndroidInstalledAppStatus; ANDROID_INSTALLED_DIRECTORY_CAPACITY],
        count: u8,
        revision: u64,
        selected_index: u8,
    ) -> bool {
        let count = usize::from(count);
        if count > ANDROID_INSTALLED_DIRECTORY_CAPACITY
            || (count == 0 && (revision != 0 || selected_index != 0))
            || (count != 0 && (revision == 0 || usize::from(selected_index) >= count))
            || apps[..count].iter().any(|app| !app.installed)
            || apps[count..].iter().any(|app| app.installed)
        {
            return false;
        }
        for first in 0..count {
            for second in first + 1..count {
                if apps[first].package == apps[second].package {
                    return false;
                }
            }
        }
        let selected = if count == 0 {
            AndroidInstalledAppStatus::empty()
        } else {
            apps[usize::from(selected_index)]
        };
        let directory_changed = self.android_installed_apps != apps
            || usize::from(self.android_installed_app_count) != count
            || self.android_installed_directory_revision != revision;
        self.android_installed_apps = apps;
        self.android_installed_app_count = count as u8;
        self.android_installed_directory_revision = revision;
        self.apply_android_installed_app_status(selected) || directory_changed
    }

    #[cfg(feature = "androidbox-multipackage4")]
    pub fn select_android_installed_app(&mut self, index: u8) -> bool {
        let index = usize::from(index);
        if index >= usize::from(self.android_installed_app_count) {
            return false;
        }
        self.apply_android_installed_app_status(self.android_installed_apps[index])
    }

    pub fn apply_android_installed_app_status(
        &mut self,
        status: AndroidInstalledAppStatus,
    ) -> bool {
        let catalog_changed = self.android_installed_app != status;
        self.android_installed_app = status;
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        let uninstall_invalidated = match self.android_installed_uninstall {
            AndroidInstalledUninstallStatus::Idle => false,
            AndroidInstalledUninstallStatus::Pending { .. }
                if !self.android_installed_app.installed =>
            {
                false
            }
            AndroidInstalledUninstallStatus::Removed { .. }
                if !self.android_installed_app.installed =>
            {
                false
            }
            state
                if state.generation().is_some_and(|generation| {
                    self.android_installed_app.installed
                        && generation == self.android_installed_app.generation
                }) =>
            {
                false
            }
            _ => {
                self.android_installed_uninstall = AndroidInstalledUninstallStatus::Idle;
                true
            }
        };
        #[cfg(not(feature = "androidbox-runtime-uninstall1"))]
        let uninstall_invalidated = false;
        let launch_invalidated = !matches!(
            self.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        ) && !self
            .android_installed_launch
            .matches_catalog(&self.android_installed_app);
        if launch_invalidated {
            self.android_installed_launch = AndroidInstalledLaunchStatus::Idle;
        }
        let session_invalidated = self
            .android_installed_session
            .is_some_and(|session| !session.matches_catalog(&self.android_installed_app));
        if session_invalidated {
            self.android_installed_session = None;
            self.android_installed_launch = AndroidInstalledLaunchStatus::Idle;
        }
        #[cfg(feature = "androidbox-interactive0")]
        let view_invalidated = if catalog_changed || launch_invalidated || session_invalidated {
            self.clear_android_installed_activity_view()
        } else {
            false
        };
        #[cfg(not(feature = "androidbox-interactive0"))]
        let view_invalidated = false;
        catalog_changed
            || uninstall_invalidated
            || launch_invalidated
            || session_invalidated
            || view_invalidated
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    pub fn apply_android_install_candidate_status(
        &mut self,
        status: AndroidInstallCandidateStatus,
    ) -> bool {
        let candidate_changed = self.android_install_candidate != status;
        self.android_install_candidate = status;
        let state_invalidated = match self.android_install {
            AndroidInstallStatus::Idle => false,
            AndroidInstallStatus::Pending { .. } if !status.present => false,
            AndroidInstallStatus::Installed { .. } if !status.present => false,
            state
                if state.candidate_id().is_some_and(|candidate_id| {
                    status.present && candidate_id == status.candidate_id
                }) =>
            {
                false
            }
            _ => {
                self.android_install = AndroidInstallStatus::Idle;
                true
            }
        };
        candidate_changed || state_invalidated
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    pub fn begin_android_install(&mut self, candidate_id: u64) -> bool {
        if self.page != MobilePage::Apps
            || !self.android_install_candidate.present
            || candidate_id == 0
            || candidate_id != self.android_install_candidate.candidate_id
            || !matches!(
                self.android_install,
                AndroidInstallStatus::Idle | AndroidInstallStatus::Installed { .. }
            )
            || !matches!(
                self.android_installed_uninstall,
                AndroidInstalledUninstallStatus::Idle
                    | AndroidInstalledUninstallStatus::Removed { .. }
            )
        {
            return false;
        }
        self.android_install = AndroidInstallStatus::Confirming { candidate_id };
        self.pressed_target = None;
        true
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    pub fn cancel_android_install(&mut self) -> bool {
        if matches!(
            self.android_install,
            AndroidInstallStatus::Confirming { .. }
                | AndroidInstallStatus::Failed { .. }
                | AndroidInstallStatus::Installed { .. }
        ) {
            self.android_install = AndroidInstallStatus::Idle;
            self.pressed_target = None;
            true
        } else {
            false
        }
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    pub fn mark_android_install_pending(
        &mut self,
        request_sequence: u64,
        candidate_id: u64,
    ) -> bool {
        if request_sequence == 0
            || !self.android_install_candidate.present
            || self.android_install_candidate.candidate_id != candidate_id
            || self.android_install != (AndroidInstallStatus::Confirming { candidate_id })
        {
            return false;
        }
        self.android_install = AndroidInstallStatus::Pending {
            request_sequence,
            candidate_id,
        };
        self.pressed_target = None;
        true
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    pub fn finish_android_install(
        &mut self,
        request_sequence: u64,
        expected: AndroidInstallCandidateStatus,
        result: Result<u64, AndroidInstallFailure>,
    ) -> bool {
        if request_sequence == 0
            || !expected.present
            || self.android_install
                != (AndroidInstallStatus::Pending {
                    request_sequence,
                    candidate_id: expected.candidate_id,
                })
        {
            return false;
        }
        self.android_install = match result {
            Ok(generation)
                if !self.android_install_candidate.present
                    && self.android_installed_app.installed
                    && self.android_installed_app.generation == generation
                    && self.android_installed_app.version_code == expected.version_code
                    && self.android_installed_app.apk_length == expected.apk_length
                    && self.android_installed_app.package == expected.package
                    && self.android_installed_app.activity == expected.activity
                    && self.android_installed_app.signer_digest_sha256
                        == expected.signer_digest_sha256
                    && self.android_installed_app.apk_digest_sha256
                        == expected.apk_digest_sha256 =>
            {
                AndroidInstallStatus::Installed {
                    request_sequence,
                    candidate_id: expected.candidate_id,
                    generation,
                }
            }
            Ok(_) => return false,
            Err(failure) => AndroidInstallStatus::Failed {
                request_sequence,
                candidate_id: expected.candidate_id,
                failure,
            },
        };
        self.pressed_target = None;
        true
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    pub fn begin_android_installed_uninstall(&mut self, generation: u64) -> bool {
        if self.page != MobilePage::Apps
            || !self.android_installed_app.installed
            || !apps_uninstall_is_available(*self)
            || generation == 0
            || generation != self.android_installed_app.generation
            || !matches!(
                self.android_installed_uninstall,
                AndroidInstalledUninstallStatus::Idle
                    | AndroidInstalledUninstallStatus::Removed { .. }
            )
        {
            return false;
        }
        self.android_installed_uninstall =
            AndroidInstalledUninstallStatus::Confirming { generation };
        self.pressed_target = None;
        true
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    pub fn cancel_android_installed_uninstall(&mut self) -> bool {
        if matches!(
            self.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Confirming { .. }
                | AndroidInstalledUninstallStatus::Failed { .. }
                | AndroidInstalledUninstallStatus::Removed { .. }
        ) {
            self.android_installed_uninstall = AndroidInstalledUninstallStatus::Idle;
            self.pressed_target = None;
            true
        } else {
            false
        }
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    pub fn mark_android_installed_uninstall_pending(
        &mut self,
        request_sequence: u64,
        generation: u64,
    ) -> bool {
        if request_sequence == 0
            || !self.android_installed_app.installed
            || generation != self.android_installed_app.generation
            || self.android_installed_uninstall
                != (AndroidInstalledUninstallStatus::Confirming { generation })
        {
            return false;
        }
        self.android_installed_uninstall = AndroidInstalledUninstallStatus::Pending {
            request_sequence,
            generation,
        };
        self.pressed_target = None;
        true
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    pub fn finish_android_installed_uninstall(
        &mut self,
        request_sequence: u64,
        generation: u64,
        result: Result<u64, AndroidInstalledUninstallFailure>,
    ) -> bool {
        if request_sequence == 0
            || generation == 0
            || self.android_installed_uninstall
                != (AndroidInstalledUninstallStatus::Pending {
                    request_sequence,
                    generation,
                })
        {
            return false;
        }
        self.android_installed_uninstall = match result {
            Ok(removal_generation)
                if !self.android_installed_app.installed
                    && generation.checked_add(1) == Some(removal_generation) =>
            {
                AndroidInstalledUninstallStatus::Removed {
                    request_sequence,
                    removal_generation,
                }
            }
            Ok(_) => return false,
            Err(failure) => AndroidInstalledUninstallStatus::Failed {
                request_sequence,
                generation,
                failure,
            },
        };
        self.pressed_target = None;
        true
    }

    /// Begins one runtime-owned installed-APK relaunch request.
    ///
    /// The request enters `Pending` only when it is non-zero and names the
    /// currently installed catalog generation. The APK digest is copied into
    /// the compact request token; title and TextView content are not copied.
    /// This method does not navigate or claim that any application is running.
    pub fn begin_android_installed_launch(
        &mut self,
        request_sequence: u64,
        generation: u64,
    ) -> bool {
        if request_sequence == 0
            || !self.android_installed_app.installed
            || generation != self.android_installed_app.generation
        {
            return false;
        }
        let next = AndroidInstalledLaunchStatus::Pending {
            request_sequence,
            generation,
            apk_sha256: self.android_installed_app.apk_digest_sha256,
        };
        let changed = self.android_installed_launch != next;
        self.android_installed_launch = next;
        #[cfg(feature = "androidbox-interactive0")]
        let view_cleared = self.clear_android_installed_activity_view();
        #[cfg(not(feature = "androidbox-interactive0"))]
        let view_cleared = false;
        changed || view_cleared
    }

    /// Publishes a successful relaunch proof for the current pending request.
    ///
    /// A late, replayed, wrong-generation, or wrong-digest result is ignored.
    /// Rendering still rechecks this token against the live catalog, so even a
    /// directly inconsistent model snapshot fails closed.
    pub fn succeed_android_installed_launch(
        &mut self,
        request_sequence: u64,
        generation: u64,
        apk_sha256: [u8; 32],
    ) -> bool {
        let AndroidInstalledLaunchStatus::Pending {
            request_sequence: pending_sequence,
            generation: pending_generation,
            apk_sha256: pending_apk_sha256,
        } = &self.android_installed_launch
        else {
            return false;
        };
        if request_sequence != *pending_sequence
            || generation != *pending_generation
            || apk_sha256 != *pending_apk_sha256
            || !self.android_installed_app.installed
            || generation != self.android_installed_app.generation
            || apk_sha256 != self.android_installed_app.apk_digest_sha256
        {
            return false;
        }
        self.android_installed_launch = AndroidInstalledLaunchStatus::Succeeded {
            request_sequence,
            generation,
            apk_sha256,
        };
        true
    }

    /// Publishes one bounded failure for the current pending request.
    ///
    /// Late or mismatched completions cannot replace a newer request.
    pub fn fail_android_installed_launch(
        &mut self,
        request_sequence: u64,
        generation: u64,
        failure: AndroidInstalledLaunchFailure,
    ) -> bool {
        let AndroidInstalledLaunchStatus::Pending {
            request_sequence: pending_sequence,
            generation: pending_generation,
            apk_sha256,
        } = &self.android_installed_launch
        else {
            return false;
        };
        if request_sequence != *pending_sequence || generation != *pending_generation {
            return false;
        }
        self.android_installed_launch = AndroidInstalledLaunchStatus::Failed {
            request_sequence,
            generation,
            apk_sha256: *apk_sha256,
            failure,
        };
        #[cfg(feature = "androidbox-interactive0")]
        let _ = self.clear_android_installed_activity_view();
        true
    }

    /// Returns true only for a successful request token bound to the current
    /// package generation and APK digest.
    pub fn installed_android_launch_content_ready(&self) -> bool {
        matches!(
            self.android_installed_launch,
            AndroidInstalledLaunchStatus::Succeeded { .. }
        ) && self
            .android_installed_launch
            .matches_catalog(&self.android_installed_app)
    }

    /// Stages or refreshes one compatible Activity session after a successful
    /// fresh package relaunch.
    ///
    /// This does not claim foreground. Publisher content remains hidden until
    /// a matching SurfaceServer `Foreground` snapshot is accepted.
    pub fn bind_android_installed_activity_session(
        &mut self,
        identity: UiCompatibleActivityIdentity,
    ) -> bool {
        if identity.package_generation() != self.android_installed_app.generation
            || !self.installed_android_launch_content_ready()
        {
            return false;
        }
        let next = AndroidInstalledActivitySession {
            identity,
            apk_sha256: self.android_installed_app.apk_digest_sha256,
        };
        self.android_installed_session = Some(next);
        true
    }

    pub fn compatible_android_recent_ready(&self) -> bool {
        let Some(session) = self.android_installed_session else {
            return false;
        };
        session.matches_catalog(&self.android_installed_app)
            && self.system_ui_recent
                == Some(UiRecentIdentity::CompatibleAndroid(session.identity()))
    }

    /// Publisher Activity content is visible only for a fresh proof, matching
    /// catalog/session identity, and an accepted SurfaceServer foreground.
    pub fn installed_android_foreground_content_ready(&self) -> bool {
        self.system_ui_mode == UiSystemUiMode::Foreground
            && self.compatible_android_recent_ready()
            && self.installed_android_launch_content_ready()
    }

    /// Publishes one strictly newer bounded publisher view snapshot.
    ///
    /// The current compatible Activity must already have a fresh foreground
    /// proof. Invalid ASCII, zero/equal IDs, zero/non-increasing revision, or
    /// an empty Button label fail closed without changing the model.
    #[cfg(feature = "androidbox-interactive0")]
    pub fn set_android_installed_activity_view(
        &mut self,
        label_view_id: u32,
        label_text: &str,
        button_view_id: u32,
        button_text: &str,
        callback_registered: bool,
        revision: u64,
    ) -> bool {
        if !self.installed_android_foreground_content_ready() {
            return false;
        }
        let Some(next) = AndroidInstalledActivityViewState::try_new(
            label_view_id,
            label_text,
            button_view_id,
            button_text,
            callback_registered,
            revision,
        ) else {
            return false;
        };
        let current_revision = self.android_installed_activity_view.revision();
        if current_revision != 0 && revision <= current_revision {
            return false;
        }
        let changed = self.android_installed_activity_view != next
            || matches!(
                self.pressed_target,
                Some(MobilePressedTarget::InstalledAndroidButton(_))
            );
        self.android_installed_activity_view = next;
        if matches!(
            self.pressed_target,
            Some(MobilePressedTarget::InstalledAndroidButton(_))
        ) {
            self.pressed_target = None;
        }
        changed
    }

    /// Publishes one complete, strictly newer bounded Scene-RPC-2 tree.
    ///
    /// Tree shape, parent ordering, IDs, callback cardinality and printable
    /// publisher text are all validated by the allocation-free scene value
    /// before any byte becomes renderable.
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub fn set_android_installed_activity_scene(
        &mut self,
        nodes: &[AndroidInstalledActivitySceneNode],
        revision: u64,
    ) -> bool {
        if !self.installed_android_foreground_content_ready() {
            return false;
        }
        let Some(next) = AndroidInstalledActivitySceneState::try_new(nodes, revision) else {
            return false;
        };
        let current_revision = self.android_installed_activity_scene.revision();
        if current_revision != 0 && revision <= current_revision {
            return false;
        }
        let changed = self.android_installed_activity_scene != next
            || matches!(
                self.pressed_target,
                Some(MobilePressedTarget::InstalledAndroidButton(_))
            );
        self.android_installed_activity_scene = next;
        if matches!(
            self.pressed_target,
            Some(MobilePressedTarget::InstalledAndroidButton(_))
        ) {
            self.pressed_target = None;
        }
        changed
    }

    /// Applies one strictly newer callback mutation to an existing TextView.
    #[cfg(feature = "androidbox-scene-rpc2")]
    pub fn update_android_installed_activity_scene_text(
        &mut self,
        view_id: u32,
        text: &str,
        revision: u64,
    ) -> bool {
        if !self.installed_android_foreground_content_ready()
            || !self.android_installed_activity_scene.is_active()
        {
            return false;
        }
        let mut next = self.android_installed_activity_scene;
        if !next.update_text(view_id, text, revision) {
            return false;
        }
        self.android_installed_activity_scene = next;
        if matches!(
            self.pressed_target,
            Some(MobilePressedTarget::InstalledAndroidButton(_))
        ) {
            self.pressed_target = None;
        }
        true
    }

    /// Clears all publisher View bytes and any matching transient Button press.
    #[cfg(feature = "androidbox-interactive0")]
    pub fn clear_android_installed_activity_view(&mut self) -> bool {
        let changed = self.android_installed_activity_view
            != AndroidInstalledActivityViewState::empty()
            || {
                #[cfg(feature = "androidbox-scene-rpc2")]
                {
                    self.android_installed_activity_scene
                        != AndroidInstalledActivitySceneState::empty()
                }
                #[cfg(not(feature = "androidbox-scene-rpc2"))]
                {
                    false
                }
            }
            || matches!(
                self.pressed_target,
                Some(MobilePressedTarget::InstalledAndroidButton(_))
            );
        self.android_installed_activity_view = AndroidInstalledActivityViewState::empty();
        #[cfg(feature = "androidbox-scene-rpc2")]
        {
            self.android_installed_activity_scene = AndroidInstalledActivitySceneState::empty();
        }
        if matches!(
            self.pressed_target,
            Some(MobilePressedTarget::InstalledAndroidButton(_))
        ) {
            self.pressed_target = None;
        }
        changed
    }

    #[cfg(feature = "androidbox-interactive0")]
    pub fn installed_android_interactive_content_ready(&self) -> bool {
        self.installed_android_foreground_content_ready()
            && (self.android_installed_activity_view.is_active() || {
                #[cfg(feature = "androidbox-scene-rpc2")]
                {
                    self.android_installed_activity_scene.is_active()
                }
                #[cfg(not(feature = "androidbox-scene-rpc2"))]
                {
                    false
                }
            })
    }

    pub const fn overview_open(&self) -> bool {
        matches!(self.system_ui_mode, UiSystemUiMode::Overview)
    }

    /// Applies one already-validated SurfaceServer system-UI snapshot.
    ///
    /// The runtime accepts revisions through `UiServerEventTracker` before
    /// calling this method. This function only mirrors that authority into the
    /// render model and clears client-local overlays when a stable system mode
    /// takes ownership.
    pub fn apply_system_ui_state(
        &mut self,
        mode: UiSystemUiMode,
        recent_app: Option<ShellAppId>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    ) -> bool {
        self.apply_system_ui_state_recent(
            mode,
            recent_app.map(UiRecentIdentity::Shell),
            nav_pressed,
            nav_reveal_px,
            revision,
        )
    }

    pub fn apply_system_ui_state_recent(
        &mut self,
        mode: UiSystemUiMode,
        recent: Option<UiRecentIdentity>,
        nav_pressed: bool,
        nav_reveal_px: u16,
        revision: u64,
    ) -> bool {
        let previous_mode = self.system_ui_mode;
        let previous_nav_reveal_px = self.system_nav_reveal_px;
        let nav_released = self.system_nav_pressed && !nav_pressed;
        let mut changed = self.system_ui_mode != mode
            || self.system_ui_recent != recent
            || self.overview_recent_offset_px != 0
            || self.system_nav_pressed != nav_pressed
            || self.system_nav_reveal_px != nav_reveal_px
            || self.system_ui_revision != revision;

        self.system_ui_mode = mode;
        self.system_ui_recent = recent;
        self.overview_recent_offset_px = 0;
        self.system_nav_pressed = nav_pressed;
        self.system_nav_reveal_px = nav_reveal_px.min(OVERVIEW_HOME_COMMIT_PX);
        self.system_ui_revision = revision;

        if self.android_installed_session.is_some_and(|session| {
            !session.matches_catalog(&self.android_installed_app)
                || recent != Some(UiRecentIdentity::CompatibleAndroid(session.identity()))
        }) {
            self.android_installed_session = None;
            self.android_installed_launch = AndroidInstalledLaunchStatus::Idle;
            #[cfg(feature = "androidbox-interactive0")]
            {
                let _ = self.clear_android_installed_activity_view();
            }
            changed = true;
        }

        match mode {
            UiSystemUiMode::Locked => {
                if self.page != MobilePage::Lock {
                    self.page = MobilePage::Lock;
                    changed = true;
                }
                // A bottom Home tap while locked may close an already-open
                // shade, but can never become unlock authority.
                if nav_released {
                    changed |= self.shade_open
                        || self.shade_reveal_px != 0
                        || self.drawer_open
                        || self.drawer_reveal_px != 0;
                    self.shade_open = false;
                    self.shade_reveal_px = 0;
                    self.drawer_open = false;
                    self.drawer_reveal_px = 0;
                }
            }
            UiSystemUiMode::Home => {
                // Calculator and AndroidBox are Launcher-local surfaces and
                // intentionally have no ShellAppId/recent identity. Preserve
                // them while a sub-threshold Overview drag is in flight or
                // cancels.
                let settles_on_home = !nav_pressed
                    && (previous_mode != UiSystemUiMode::Home
                        || !nav_released
                        || previous_nav_reveal_px == 0
                        || previous_nav_reveal_px >= OVERVIEW_HOME_COMMIT_PX);
                if settles_on_home {
                    if self.page != MobilePage::Home {
                        self.page = MobilePage::Home;
                        changed = true;
                    }
                    changed |= self.shade_open
                        || self.shade_reveal_px != 0
                        || self.drawer_open
                        || self.drawer_reveal_px != 0;
                    self.shade_open = false;
                    self.shade_reveal_px = 0;
                    self.drawer_open = false;
                    self.drawer_reveal_px = 0;
                }
            }
            UiSystemUiMode::Foreground => {
                if previous_mode != UiSystemUiMode::Foreground && !nav_pressed {
                    changed |= self.shade_open
                        || self.shade_reveal_px != 0
                        || self.drawer_open
                        || self.drawer_reveal_px != 0;
                    self.shade_open = false;
                    self.shade_reveal_px = 0;
                    self.drawer_open = false;
                    self.drawer_reveal_px = 0;
                }
            }
            UiSystemUiMode::Overview => {
                changed |= self.shade_open
                    || self.shade_reveal_px != 0
                    || self.drawer_open
                    || self.drawer_reveal_px != 0;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
            }
        }
        if previous_mode != mode || nav_released {
            changed |= self.pressed_target.is_some()
                || self.back_reveal_px != 0
                || self.back_origin_y != 0;
            self.pressed_target = None;
            self.back_reveal_px = 0;
            self.back_origin_y = 0;
        }
        changed
    }

    pub const fn effective_overview_reveal_px(&self) -> u16 {
        if self.overview_open() {
            OVERVIEW_RENDER_MAX_PX
        } else if !matches!(self.system_ui_mode, UiSystemUiMode::Locked) && self.system_nav_pressed
        {
            if self.system_nav_reveal_px > OVERVIEW_RENDER_MAX_PX {
                OVERVIEW_RENDER_MAX_PX
            } else {
                self.system_nav_reveal_px
            }
        } else {
            0
        }
    }

    pub const fn effective_overview_recent_offset_px(&self) -> u16 {
        if !matches!(self.system_ui_mode, UiSystemUiMode::Overview)
            || self.system_ui_recent.is_none()
        {
            return 0;
        }
        let bounded = if self.overview_recent_offset_px > OVERVIEW_RECENT_MAX_OFFSET_PX {
            OVERVIEW_RECENT_MAX_OFFSET_PX
        } else {
            self.overview_recent_offset_px
        };
        bounded / OVERVIEW_RECENT_RENDER_QUANTUM_PX * OVERVIEW_RECENT_RENDER_QUANTUM_PX
    }

    pub fn phone_number(&self) -> &str {
        let count = usize::from(self.phone_digit_count).min(PHONE_DIGIT_CAPACITY);
        core::str::from_utf8(&self.phone_digits[..count])
            .expect("the dialer stores only validated ASCII keys")
    }

    fn clear_calculator(&mut self) -> bool {
        let changed = self.calculator_value != 0
            || self.calculator_accumulator != 0
            || self.calculator_pending != CalculatorOperation::None
            || self.calculator_entering
            || self.calculator_decimal_entered
            || self.calculator_fraction_digits != 0
            || self.calculator_negative_zero
            || self.calculator_error;
        self.calculator_value = 0;
        self.calculator_accumulator = 0;
        self.calculator_pending = CalculatorOperation::None;
        self.calculator_entering = false;
        self.calculator_decimal_entered = false;
        self.calculator_fraction_digits = 0;
        self.calculator_negative_zero = false;
        self.calculator_error = false;
        changed
    }

    fn fail_calculator(&mut self) {
        self.calculator_value = 0;
        self.calculator_accumulator = 0;
        self.calculator_pending = CalculatorOperation::None;
        self.calculator_entering = false;
        self.calculator_decimal_entered = false;
        self.calculator_fraction_digits = 0;
        self.calculator_negative_zero = false;
        self.calculator_error = true;
    }

    fn begin_calculator_entry(&mut self) {
        if self.calculator_entering {
            return;
        }
        self.calculator_value = 0;
        self.calculator_entering = true;
        self.calculator_decimal_entered = false;
        self.calculator_fraction_digits = 0;
        self.calculator_negative_zero = false;
        if self.calculator_pending == CalculatorOperation::None {
            self.calculator_accumulator = 0;
        }
    }

    fn apply_calculator_key(&mut self, index: u8) -> bool {
        let Some(key) = calculator_key(index) else {
            return false;
        };
        if key == CalculatorKey::Clear {
            return self.clear_calculator();
        }
        if self.calculator_error {
            return false;
        }

        match key {
            CalculatorKey::Clear => unreachable!("clear is handled before calculator dispatch"),
            CalculatorKey::Digit(digit) => {
                self.begin_calculator_entry();
                if self.calculator_decimal_entered {
                    if self.calculator_fraction_digits >= CALCULATOR_FRACTION_DIGITS {
                        return false;
                    }
                    let next_digits = self.calculator_fraction_digits + 1;
                    let unit = calculator_fraction_unit(next_digits);
                    let signed_digit = if self.calculator_value < 0
                        || self.calculator_value == 0 && self.calculator_negative_zero
                    {
                        -i64::from(digit)
                    } else {
                        i64::from(digit)
                    };
                    let Some(next) = self
                        .calculator_value
                        .checked_add(signed_digit.saturating_mul(unit))
                    else {
                        self.fail_calculator();
                        return true;
                    };
                    if !calculator_value_in_range(next) {
                        self.fail_calculator();
                        return true;
                    }
                    self.calculator_value = next;
                    self.calculator_fraction_digits = next_digits;
                    true
                } else {
                    let sign = if self.calculator_value < 0
                        || self.calculator_value == 0 && self.calculator_negative_zero
                    {
                        -1
                    } else {
                        1
                    };
                    let next = i128::from(self.calculator_value) * 10
                        + i128::from(sign) * i128::from(digit) * i128::from(CALCULATOR_SCALE);
                    let Some(next) = checked_calculator_value(next) else {
                        self.fail_calculator();
                        return true;
                    };
                    self.calculator_value = next;
                    self.calculator_negative_zero = false;
                    true
                }
            }
            CalculatorKey::Decimal => {
                self.begin_calculator_entry();
                if self.calculator_decimal_entered {
                    false
                } else {
                    self.calculator_decimal_entered = true;
                    self.calculator_fraction_digits = 0;
                    true
                }
            }
            CalculatorKey::Sign => {
                if !self.calculator_entering && self.calculator_pending != CalculatorOperation::None
                {
                    self.begin_calculator_entry();
                }
                if self.calculator_value == 0 {
                    self.calculator_entering = true;
                    self.calculator_negative_zero = !self.calculator_negative_zero;
                } else {
                    self.calculator_value = -self.calculator_value;
                    self.calculator_negative_zero = false;
                }
                true
            }
            CalculatorKey::Percent => {
                if !self.calculator_entering && self.calculator_pending != CalculatorOperation::None
                {
                    self.begin_calculator_entry();
                }
                let before = self.calculator_value;
                self.calculator_value /= 100;
                self.calculator_entering = true;
                self.calculator_decimal_entered = false;
                self.calculator_fraction_digits = 0;
                self.calculator_negative_zero = false;
                before != self.calculator_value
            }
            CalculatorKey::Backspace => {
                if !self.calculator_entering {
                    return false;
                }
                if self.calculator_decimal_entered {
                    if self.calculator_fraction_digits == 0 {
                        self.calculator_decimal_entered = false;
                        return true;
                    }
                    let next_digits = self.calculator_fraction_digits - 1;
                    let unit = calculator_fraction_unit(next_digits);
                    let was_negative = self.calculator_value < 0;
                    self.calculator_value = self.calculator_value / unit * unit;
                    self.calculator_fraction_digits = next_digits;
                    self.calculator_negative_zero = was_negative && self.calculator_value == 0;
                    true
                } else {
                    let was_negative = self.calculator_value < 0 || self.calculator_negative_zero;
                    let whole = self.calculator_value / CALCULATOR_SCALE;
                    self.calculator_value = whole / 10 * CALCULATOR_SCALE;
                    self.calculator_negative_zero = was_negative && self.calculator_value == 0;
                    true
                }
            }
            CalculatorKey::Operation(operation) => {
                if self.calculator_pending != CalculatorOperation::None && self.calculator_entering
                {
                    let Some(result) = evaluate_calculator(
                        self.calculator_accumulator,
                        self.calculator_value,
                        self.calculator_pending,
                    ) else {
                        self.fail_calculator();
                        return true;
                    };
                    self.calculator_value = result;
                    self.calculator_accumulator = result;
                } else {
                    self.calculator_accumulator = self.calculator_value;
                }
                let changed = self.calculator_pending != operation
                    || self.calculator_entering
                    || self.calculator_decimal_entered
                    || self.calculator_fraction_digits != 0
                    || self.calculator_negative_zero;
                self.calculator_pending = operation;
                self.calculator_entering = false;
                self.calculator_decimal_entered = false;
                self.calculator_fraction_digits = 0;
                self.calculator_negative_zero = false;
                changed
            }
            CalculatorKey::Equals => {
                if self.calculator_pending == CalculatorOperation::None || !self.calculator_entering
                {
                    return false;
                }
                let Some(result) = evaluate_calculator(
                    self.calculator_accumulator,
                    self.calculator_value,
                    self.calculator_pending,
                ) else {
                    self.fail_calculator();
                    return true;
                };
                self.calculator_value = result;
                self.calculator_accumulator = result;
                self.calculator_pending = CalculatorOperation::None;
                self.calculator_entering = false;
                self.calculator_decimal_entered = false;
                self.calculator_fraction_digits = 0;
                self.calculator_negative_zero = false;
                true
            }
        }
    }

    pub const fn has_in_app_back(&self) -> bool {
        matches!(
            self.page,
            MobilePage::Apps | MobilePage::About | MobilePage::Display | MobilePage::Accessibility
        ) || matches!(self.page, MobilePage::Messages)
            && !matches!(self.message_view, MessageView::Inbox)
    }

    /// Returns the visible quick-settings extent in physical scanout pixels.
    pub const fn effective_shade_reveal_px(&self) -> u16 {
        if self.shade_reveal_px != 0 {
            if self.shade_reveal_px > SHADE_REVEAL_MAX {
                SHADE_REVEAL_MAX
            } else {
                self.shade_reveal_px
            }
        } else if self.shade_open {
            SHADE_REVEAL_MAX
        } else {
            0
        }
    }

    pub const fn effective_drawer_reveal_px(&self) -> u16 {
        if self.drawer_reveal_px != 0 {
            if self.drawer_reveal_px > DRAWER_REVEAL_MAX {
                DRAWER_REVEAL_MAX
            } else {
                self.drawer_reveal_px
            }
        } else if self.drawer_open {
            DRAWER_REVEAL_MAX
        } else {
            0
        }
    }

    pub const fn effective_back_reveal_px(&self) -> u16 {
        if self.back_reveal_px > BACK_GESTURE_REVEAL_MAX {
            BACK_GESTURE_REVEAL_MAX
        } else {
            self.back_reveal_px
        }
    }

    pub const fn effective_unlock_reveal_px(&self) -> u16 {
        if self.unlock_reveal_px > UNLOCK_REVEAL_MAX {
            UNLOCK_REVEAL_MAX
        } else {
            self.unlock_reveal_px
        }
    }

    pub const fn effective_page_transition_offset_px(&self) -> u16 {
        if self.page_transition_offset_px > PAGE_TRANSITION_MAX_OFFSET {
            PAGE_TRANSITION_MAX_OFFSET
        } else {
            self.page_transition_offset_px
        }
    }

    /// Returns the canonical scroll position for the currently visible page.
    pub const fn effective_page_scroll_offset_px(&self) -> u16 {
        match self.page {
            MobilePage::Settings => {
                if self.settings_scroll_offset_px > SETTINGS_SCROLL_MAX_PX {
                    SETTINGS_SCROLL_MAX_PX
                } else {
                    self.settings_scroll_offset_px / PAGE_SCROLL_QUANTUM_PX * PAGE_SCROLL_QUANTUM_PX
                }
            }
            MobilePage::Apps => {
                if self.apps_scroll_offset_px > APPS_SCROLL_MAX_PX {
                    APPS_SCROLL_MAX_PX
                } else {
                    self.apps_scroll_offset_px / PAGE_SCROLL_QUANTUM_PX * PAGE_SCROLL_QUANTUM_PX
                }
            }
            MobilePage::Lock
            | MobilePage::Home
            | MobilePage::Phone
            | MobilePage::Messages
            | MobilePage::Calculator
            | MobilePage::AndroidDemo
            | MobilePage::About
            | MobilePage::Display
            | MobilePage::Accessibility => 0,
        }
    }

    pub fn apply(&mut self, action: MobileAction) -> bool {
        let clears_back_feedback = !matches!(action, MobileAction::SetBackReveal { .. });
        let back_feedback_changed =
            clears_back_feedback && (self.back_reveal_px != 0 || self.back_origin_y != 0);
        if clears_back_feedback {
            self.back_reveal_px = 0;
            self.back_origin_y = 0;
        }
        let clears_unlock_feedback = !matches!(action, MobileAction::SetUnlockReveal(_));
        let unlock_feedback_changed = clears_unlock_feedback && self.unlock_reveal_px != 0;
        if clears_unlock_feedback {
            self.unlock_reveal_px = 0;
        }
        let clears_page_transition = !matches!(action, MobileAction::SetPageTransitionOffset(_));
        let page_transition_changed = clears_page_transition && self.page_transition_offset_px != 0;
        if clears_page_transition {
            self.page_transition_offset_px = 0;
        }
        let clears_boot_notification_offset =
            !matches!(action, MobileAction::SetBootNotificationOffset(_));
        let boot_notification_offset_changed =
            clears_boot_notification_offset && self.boot_notification_offset_px != 0;
        if clears_boot_notification_offset {
            self.boot_notification_offset_px = 0;
        }
        let clears_overview_recent_offset =
            !matches!(action, MobileAction::SetOverviewRecentOffset(_));
        let overview_recent_offset_changed =
            clears_overview_recent_offset && self.overview_recent_offset_px != 0;
        if clears_overview_recent_offset {
            self.overview_recent_offset_px = 0;
        }
        let changed = match action {
            MobileAction::Unlock
                if self.page == MobilePage::Lock
                    && !self.shade_open
                    && self.shade_reveal_px == 0 =>
            {
                self.page = MobilePage::Home;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                true
            }
            MobileAction::Home if self.page == MobilePage::Lock => {
                let changed = self.shade_open
                    || self.shade_reveal_px != 0
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.pressed_target.is_some();
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::Home => {
                let changed = self.page != MobilePage::Home
                    || self.shade_open
                    || self.shade_reveal_px != 0
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.pressed_target.is_some();
                self.page = MobilePage::Home;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::OpenShade => {
                let changed = !self.shade_open
                    || self.shade_reveal_px != 0
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.pressed_target.is_some();
                self.shade_open = true;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::CloseShade => {
                let changed =
                    self.shade_open || self.shade_reveal_px != 0 || self.pressed_target.is_some();
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::OpenDrawer if self.page == MobilePage::Home => {
                let changed = !self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.shade_open
                    || self.shade_reveal_px != 0
                    || self.pressed_target.is_some();
                self.drawer_open = true;
                self.drawer_reveal_px = 0;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::CloseDrawer => {
                let changed =
                    self.drawer_open || self.drawer_reveal_px != 0 || self.pressed_target.is_some();
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::Back if self.shade_open || self.shade_reveal_px != 0 => {
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.pressed_target = None;
                true
            }
            MobileAction::Back if self.drawer_open || self.drawer_reveal_px != 0 => {
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                true
            }
            MobileAction::Back if self.page == MobilePage::Lock => {
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            MobileAction::Back
                if self.page == MobilePage::Messages && self.message_view != MessageView::Inbox =>
            {
                self.message_view = MessageView::Inbox;
                self.pressed_target = None;
                true
            }
            MobileAction::Open(page)
                if self.page != MobilePage::Lock
                    && page != MobilePage::Lock
                    && self.page != page =>
            {
                self.page = page;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                true
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            MobileAction::Back
                if self.page == MobilePage::Apps
                    && matches!(
                        self.android_install,
                        AndroidInstallStatus::Confirming { .. }
                            | AndroidInstallStatus::Failed { .. }
                            | AndroidInstallStatus::Installed { .. }
                    ) =>
            {
                self.cancel_android_install()
            }
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            MobileAction::Back
                if self.page == MobilePage::Apps
                    && matches!(
                        self.android_installed_uninstall,
                        AndroidInstalledUninstallStatus::Confirming { .. }
                            | AndroidInstalledUninstallStatus::Failed { .. }
                            | AndroidInstalledUninstallStatus::Removed { .. }
                    ) =>
            {
                self.cancel_android_installed_uninstall()
            }
            MobileAction::Back if self.page == MobilePage::Accessibility => {
                self.page = MobilePage::Display;
                self.pressed_target = None;
                true
            }
            MobileAction::Back
                if matches!(
                    self.page,
                    MobilePage::Apps | MobilePage::About | MobilePage::Display
                ) =>
            {
                self.page = MobilePage::Settings;
                self.pressed_target = None;
                true
            }
            MobileAction::Back if self.page != MobilePage::Home => {
                self.page = MobilePage::Home;
                self.pressed_target = None;
                true
            }
            MobileAction::ToggleTheme => {
                self.dark_theme = !self.dark_theme;
                self.pressed_target = None;
                true
            }
            MobileAction::ToggleAccent => {
                self.alternate_accent = !self.alternate_accent;
                self.pressed_target = None;
                true
            }
            MobileAction::SetSoftwareDimming(level) => {
                let changed = self.software_dimming != level || self.pressed_target.is_some();
                self.software_dimming = level;
                self.pressed_target = None;
                changed
            }
            MobileAction::ToggleLargeText => {
                self.large_text = !self.large_text;
                self.pressed_target = None;
                true
            }
            MobileAction::ToggleHighContrast => {
                self.high_contrast = !self.high_contrast;
                self.pressed_target = None;
                true
            }
            MobileAction::ActivateBootNotification
                if self.boot_notification_visible
                    && self.shade_open
                    && self.page != MobilePage::Lock =>
            {
                let changed = if self.page == MobilePage::Settings {
                    self.page = MobilePage::About;
                    self.shade_open = false;
                    self.shade_reveal_px = 0;
                    self.boot_notification_expanded = false;
                    true
                } else {
                    self.boot_notification_expanded = !self.boot_notification_expanded;
                    true
                };
                self.pressed_target = None;
                changed
            }
            MobileAction::DismissBootNotification => {
                let changed = self.boot_notification_visible
                    || self.boot_notification_expanded
                    || self.pressed_target.is_some();
                self.boot_notification_visible = false;
                self.boot_notification_expanded = false;
                self.pressed_target = None;
                changed
            }
            MobileAction::PhoneKey(index) if self.page == MobilePage::Phone => {
                let before_pressed = self.pressed_target.is_some();
                let Some(key) = phone_key_byte(index) else {
                    self.pressed_target = None;
                    return before_pressed || back_feedback_changed;
                };
                let count = usize::from(self.phone_digit_count);
                let appended = count < PHONE_DIGIT_CAPACITY;
                if appended {
                    self.phone_digits[count] = key;
                    self.phone_digit_count += 1;
                }
                self.pressed_target = None;
                appended || before_pressed
            }
            MobileAction::PhoneBackspace if self.page == MobilePage::Phone => {
                let before_pressed = self.pressed_target.is_some();
                let removed = self.phone_digit_count != 0;
                if removed {
                    self.phone_digit_count -= 1;
                    self.phone_digits[usize::from(self.phone_digit_count)] = 0;
                }
                self.pressed_target = None;
                removed || before_pressed
            }
            MobileAction::CalculatorKey(index) if self.page == MobilePage::Calculator => {
                let before_pressed = self.pressed_target.is_some();
                let changed = self.apply_calculator_key(index);
                self.pressed_target = None;
                changed || before_pressed
            }
            MobileAction::OpenMessage(view)
                if self.page == MobilePage::Messages && view != MessageView::Inbox =>
            {
                let changed = self.message_view != view || self.pressed_target.is_some();
                self.message_view = view;
                self.pressed_target = None;
                changed
            }
            MobileAction::ExecuteAndroidBoxDex => {
                // Execution belongs to the runtime owner. Applying the UI
                // action only releases visual contact capture and never
                // fabricates verification, values, steps, taps, or errors.
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            MobileAction::LaunchInstalledAndroid(_) => {
                // Durable readback, verification, lifecycle execution, launch
                // status publication, and navigation all belong to the
                // runtime owner. Applying this request only releases visual
                // contact capture.
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            #[cfg(feature = "androidbox-multipackage4")]
            MobileAction::SelectInstalledAndroid(index)
                if self.page == MobilePage::Apps && apps_package_selection_is_available(*self) =>
            {
                let selected = self.select_android_installed_app(index);
                let changed = selected || self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            #[cfg(feature = "androidbox-multipackage4")]
            MobileAction::SelectInstalledAndroid(_) => {
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            MobileAction::BeginInstalledAndroidUninstall(generation) => {
                self.begin_android_installed_uninstall(generation)
            }
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            MobileAction::ConfirmInstalledAndroidUninstall(_) => {
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            #[cfg(feature = "androidbox-runtime-uninstall1")]
            MobileAction::CancelInstalledAndroidUninstall => {
                self.cancel_android_installed_uninstall()
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            MobileAction::BeginAndroidInstall(candidate_id) => {
                self.begin_android_install(candidate_id)
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            MobileAction::ConfirmAndroidInstall(_) => {
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            #[cfg(feature = "androidbox-runtime-install2")]
            MobileAction::CancelAndroidInstall => self.cancel_android_install(),
            #[cfg(feature = "androidbox-interactive0")]
            MobileAction::ActivateInstalledAndroidButton(_) => {
                // Callback dispatch and any resulting View revision belong to
                // the admitted runtime owner. The UI model only releases the
                // matching transient press.
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
            MobileAction::SetShadeReveal(reveal_px) => {
                let reveal_px = reveal_px.min(SHADE_REVEAL_MAX);
                let changed = self.shade_reveal_px != reveal_px
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.pressed_target.is_some();
                self.shade_reveal_px = reveal_px;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetDrawerReveal(reveal_px) if self.page == MobilePage::Home => {
                let quantized = quantize_drawer_reveal(reveal_px);
                let changed = self.drawer_reveal_px != quantized
                    || self.shade_open
                    || self.shade_reveal_px != 0
                    || self.pressed_target.is_some();
                self.drawer_reveal_px = quantized;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetPageScroll { page, offset_px } if self.page == page => {
                let maximum = page_scroll_max_px(page);
                if maximum == 0 {
                    let changed = self.pressed_target.is_some();
                    self.pressed_target = None;
                    changed
                } else {
                    let offset_px =
                        offset_px.min(maximum) / PAGE_SCROLL_QUANTUM_PX * PAGE_SCROLL_QUANTUM_PX;
                    let changed = match page {
                        MobilePage::Settings => {
                            let changed = self.settings_scroll_offset_px != offset_px;
                            self.settings_scroll_offset_px = offset_px;
                            changed
                        }
                        MobilePage::Apps => {
                            let changed = self.apps_scroll_offset_px != offset_px;
                            self.apps_scroll_offset_px = offset_px;
                            changed
                        }
                        MobilePage::Lock
                        | MobilePage::Home
                        | MobilePage::Phone
                        | MobilePage::Messages
                        | MobilePage::Calculator
                        | MobilePage::AndroidDemo
                        | MobilePage::About
                        | MobilePage::Display
                        | MobilePage::Accessibility => false,
                    } || self.pressed_target.is_some();
                    self.pressed_target = None;
                    changed
                }
            }
            MobileAction::SetBackReveal {
                reveal_px,
                origin_y,
            } => {
                let reveal_px = quantize_back_reveal(reveal_px);
                let origin_y = if reveal_px == 0 {
                    0
                } else {
                    origin_y.clamp(BACK_GESTURE_START_MIN_Y, SYSTEM_NAV_TOP_PX - 1)
                };
                let changed = self.back_reveal_px != reveal_px
                    || self.back_origin_y != origin_y
                    || self.pressed_target.is_some();
                self.back_reveal_px = reveal_px;
                self.back_origin_y = origin_y;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetUnlockReveal(reveal_px)
                if self.page == MobilePage::Lock
                    && !self.shade_open
                    && self.shade_reveal_px == 0 =>
            {
                let quantized = quantize_unlock_reveal(reveal_px);
                let changed = self.unlock_reveal_px != quantized
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.pressed_target.is_some();
                self.unlock_reveal_px = quantized;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetBootNotificationOffset(offset_px)
                if self.boot_notification_visible
                    && (self.shade_open
                        || (self.page == MobilePage::Lock
                            && self.shade_reveal_px == 0
                            && self.unlock_reveal_px == 0)) =>
            {
                let offset_px = quantize_boot_notification_offset(offset_px);
                let changed =
                    self.boot_notification_offset_px != offset_px || self.pressed_target.is_some();
                self.boot_notification_offset_px = offset_px;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetOverviewRecentOffset(offset_px)
                if self.overview_open() && self.system_ui_recent.is_some() =>
            {
                let offset_px = quantize_overview_recent_offset(offset_px);
                let changed =
                    self.overview_recent_offset_px != offset_px || self.pressed_target.is_some();
                self.overview_recent_offset_px = offset_px;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetPageTransitionOffset(offset_px)
                if !matches!(self.page, MobilePage::Lock | MobilePage::Home) =>
            {
                let offset_px = offset_px.min(PAGE_TRANSITION_MAX_OFFSET);
                let changed = self.page_transition_offset_px != offset_px
                    || self.shade_open
                    || self.shade_reveal_px != 0
                    || self.drawer_open
                    || self.drawer_reveal_px != 0
                    || self.back_reveal_px != 0
                    || self.back_origin_y != 0
                    || self.unlock_reveal_px != 0
                    || self.pressed_target.is_some();
                self.page_transition_offset_px = offset_px;
                self.shade_open = false;
                self.shade_reveal_px = 0;
                self.drawer_open = false;
                self.drawer_reveal_px = 0;
                self.back_reveal_px = 0;
                self.back_origin_y = 0;
                self.unlock_reveal_px = 0;
                self.pressed_target = None;
                changed
            }
            MobileAction::SetPressed(pressed_target) => {
                let changed = self.pressed_target != pressed_target;
                self.pressed_target = pressed_target;
                changed
            }
            MobileAction::Open(_)
            | MobileAction::Unlock
            | MobileAction::ActivateRecentApp
            | MobileAction::DismissRecentApp
            | MobileAction::CloseOverview
            | MobileAction::OpenDrawer
            | MobileAction::SetDrawerReveal(_)
            | MobileAction::SetUnlockReveal(_)
            | MobileAction::SetBootNotificationOffset(_)
            | MobileAction::SetOverviewRecentOffset(_)
            | MobileAction::SetPageTransitionOffset(_)
            | MobileAction::SetPageScroll { .. }
            | MobileAction::PhoneKey(_)
            | MobileAction::PhoneBackspace
            | MobileAction::CalculatorKey(_)
            | MobileAction::OpenMessage(_)
            | MobileAction::ActivateBootNotification
            | MobileAction::Back => {
                let changed = self.pressed_target.is_some();
                self.pressed_target = None;
                changed
            }
        };
        changed
            || back_feedback_changed
            || unlock_feedback_changed
            || page_transition_changed
            || boot_notification_offset_changed
            || overview_recent_offset_changed
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileAction {
    Open(MobilePage),
    Unlock,
    Home,
    Back,
    /// Request activation of the single SurfaceServer-authenticated recent
    /// ShellAppId from stable Overview.
    ActivateRecentApp,
    /// Request removal of the exact SurfaceServer-authenticated recent card.
    /// This does not itself mutate the server-owned record or kill a process.
    DismissRecentApp,
    /// Request a stable Overview-to-Home transition.
    CloseOverview,
    ToggleTheme,
    ToggleAccent,
    /// Set one of five bounded final-surface software dimming levels.
    SetSoftwareDimming(UiSoftwareDimming),
    /// Toggle the bounded two-stop semantic font scale.
    ToggleLargeText,
    /// Toggle the stronger semantic text/outline contrast palette.
    ToggleHighContrast,
    /// Activate the fixed boot-local notice. Settings opens its existing
    /// About page; other unlocked pages disclose bounded inline detail.
    ActivateBootNotification,
    /// Apply the authoritative session dismissal broadcast.
    DismissBootNotification,
    PhoneKey(u8),
    PhoneBackspace,
    CalculatorKey(u8),
    OpenMessage(MessageView),
    /// Ask the runtime owner to execute the bounded AndroidBox entry point.
    ///
    /// The UI model does not execute code or update result fields when this
    /// action is applied.
    ExecuteAndroidBoxDex,
    /// Ask the runtime owner to re-read, re-verify, and relaunch the installed
    /// APK generation selected in All apps.
    ///
    /// Applying this action to `MobileModel` never navigates and never
    /// fabricates a pending or successful result.
    LaunchInstalledAndroid(u64),
    /// Select one already-installed package for Settings details.
    ///
    /// This is a local, zero-authority directory selection. It neither
    /// launches nor mutates the package and is rejected while an install or
    /// uninstall transaction blocks the Apps page.
    #[cfg(feature = "androidbox-multipackage4")]
    SelectInstalledAndroid(u8),
    /// First, non-authoritative Settings confirmation step.
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    BeginInstalledAndroidUninstall(u64),
    /// Ask the runtime owner to submit the already-confirmed exact generation.
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    ConfirmInstalledAndroidUninstall(u64),
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    CancelInstalledAndroidUninstall,
    /// First, local-only confirmation step for an admitted ABI 53 candidate.
    #[cfg(feature = "androidbox-runtime-install2")]
    BeginAndroidInstall(u64),
    /// Ask the runtime owner to submit the already-confirmed exact candidate.
    #[cfg(feature = "androidbox-runtime-install2")]
    ConfirmAndroidInstall(u64),
    #[cfg(feature = "androidbox-runtime-install2")]
    CancelAndroidInstall,
    /// Ask the runtime owner to invoke the callback registered for one
    /// generation-bound installed-Activity Button.
    #[cfg(feature = "androidbox-interactive0")]
    ActivateInstalledAndroidButton(u32),
    OpenShade,
    CloseShade,
    OpenDrawer,
    CloseDrawer,
    /// Set a transient visible panel extent in physical scanout pixels.
    SetShadeReveal(u16),
    /// Set the transient All apps extent in physical scanout pixels.
    SetDrawerReveal(u16),
    /// Set the transient upward displacement of the visual lock surface.
    SetUnlockReveal(u16),
    /// Set signed, quantized horizontal feedback without dismissing.
    SetBootNotificationOffset(i16),
    /// Set bounded upward Overview-card feedback without removing the record.
    SetOverviewRecentOffset(u16),
    /// Set the deterministic full-page software-transition offset.
    SetPageTransitionOffset(u16),
    /// Set one bounded Settings/Apps content scroll position.
    ///
    /// `page` binds a gesture to the page on which it began, so a delayed
    /// action cannot scroll a newly focused page. Offsets are physical pixels
    /// and settle to `PAGE_SCROLL_QUANTUM_PX`.
    SetPageScroll {
        page: MobilePage,
        offset_px: u16,
    },
    /// Set quantized left-edge Back feedback without committing navigation.
    SetBackReveal {
        reveal_px: u16,
        origin_y: u16,
    },
    /// Set or clear the transient contact-highlight target.
    SetPressed(Option<MobilePressedTarget>),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TouchController {
    pressed: bool,
    armed: Option<MobilePressedTarget>,
    start_x: u16,
    start_y: u16,
    shade_dragged: bool,
    restore_drawer_after_shade: bool,
    drawer_dragged: bool,
    page_scroll_candidate: bool,
    page_scroll_dragged: bool,
    page_scroll_page: Option<MobilePage>,
    page_scroll_start_offset_px: u16,
    unlock_candidate: bool,
    unlock_dragged: bool,
    unlock_rejected: bool,
    edge_back_candidate: bool,
    edge_back_dragged: bool,
    edge_back_rejected: bool,
    software_dimming_drag: bool,
    software_dimming_level: UiSoftwareDimming,
    boot_notification_candidate: bool,
    boot_notification_dragged: bool,
    boot_notification_rejected: bool,
    overview_recent_candidate: bool,
    overview_recent_dragged: bool,
    overview_recent_rejected: bool,
    cancelled_until_release: bool,
}

impl TouchController {
    pub const fn new() -> Self {
        Self {
            pressed: false,
            armed: None,
            start_x: 0,
            start_y: 0,
            shade_dragged: false,
            restore_drawer_after_shade: false,
            drawer_dragged: false,
            page_scroll_candidate: false,
            page_scroll_dragged: false,
            page_scroll_page: None,
            page_scroll_start_offset_px: 0,
            unlock_candidate: false,
            unlock_dragged: false,
            unlock_rejected: false,
            edge_back_candidate: false,
            edge_back_dragged: false,
            edge_back_rejected: false,
            software_dimming_drag: false,
            software_dimming_level: UiSoftwareDimming::Off,
            boot_notification_candidate: false,
            boot_notification_dragged: false,
            boot_notification_rejected: false,
            overview_recent_candidate: false,
            overview_recent_dragged: false,
            overview_recent_rejected: false,
            cancelled_until_release: false,
        }
    }

    pub fn observe(
        &mut self,
        model: MobileModel,
        x: u16,
        y: u16,
        pressed: bool,
    ) -> Option<MobileAction> {
        if self.cancelled_until_release {
            if !pressed {
                *self = Self::new();
            }
            return None;
        }
        match (self.pressed, pressed) {
            (false, true) => {
                self.pressed = true;
                self.start_x = x;
                self.start_y = y;
                self.shade_dragged = false;
                self.restore_drawer_after_shade = false;
                self.drawer_dragged = false;
                self.page_scroll_candidate = false;
                self.page_scroll_dragged = false;
                self.page_scroll_page = None;
                self.page_scroll_start_offset_px = 0;
                let hit = hit_test(model, x, y);
                self.software_dimming_drag = hit == Some(MobilePressedTarget::SoftwareDimming);
                self.software_dimming_level = model.software_dimming;
                self.boot_notification_candidate =
                    hit == Some(MobilePressedTarget::BootNotification);
                self.boot_notification_dragged = false;
                self.boot_notification_rejected = false;
                self.overview_recent_candidate = model.overview_open()
                    && model.system_ui_recent.is_some()
                    && OVERVIEW_RECENT_TARGET.contains(x, y);
                self.overview_recent_dragged = false;
                self.overview_recent_rejected = false;
                self.unlock_candidate = point_inside_visible_display(x, y)
                    && model.page == MobilePage::Lock
                    && !model.shade_open
                    && model.shade_reveal_px == 0
                    && model.unlock_reveal_px == 0
                    && (UNLOCK_GESTURE_START_MIN_Y..SYSTEM_NAV_TOP_PX).contains(&y)
                    && hit.is_none();
                self.unlock_dragged = false;
                self.unlock_rejected = false;
                self.edge_back_candidate = point_inside_visible_display(x, y)
                    && edge_back_available(model)
                    && x < BACK_GESTURE_START_MAX_X
                    && (BACK_GESTURE_START_MIN_Y..SYSTEM_NAV_TOP_PX).contains(&y)
                    && hit.is_none();
                self.edge_back_dragged = false;
                self.edge_back_rejected = false;
                self.page_scroll_candidate = point_inside_visible_display(x, y)
                    && page_scrollable(model.page)
                    && !model.shade_open
                    && model.shade_reveal_px == 0
                    && !model.drawer_open
                    && model.drawer_reveal_px == 0
                    && !model.overview_open()
                    && model.effective_page_transition_offset_px() == 0
                    && x >= BACK_GESTURE_START_MAX_X
                    && (PAGE_SCROLL_VIEWPORT_TOP_PX..PAGE_SCROLL_VIEWPORT_BOTTOM_PX).contains(&y)
                    && hit != Some(MobilePressedTarget::SoftwareDimming)
                    && !self.edge_back_candidate;
                if self.page_scroll_candidate {
                    self.page_scroll_page = Some(model.page);
                    self.page_scroll_start_offset_px = model.effective_page_scroll_offset_px();
                }
                self.armed = if self.unlock_candidate || self.edge_back_candidate {
                    None
                } else {
                    hit
                };
                self.armed
                    .map(|target| MobileAction::SetPressed(Some(target)))
            }
            (true, false) => {
                self.pressed = false;
                let horizontal_travel = x.abs_diff(self.start_x);
                let vertical_travel = y.abs_diff(self.start_y);
                let gesture_is_vertical = vertical_travel >= horizontal_travel.saturating_mul(2);
                let opened_shade = point_inside_visible_display(self.start_x, self.start_y)
                    && !model.shade_open
                    && self.start_y < SHADE_GESTURE_START_MAX_Y
                    && y >= self.start_y.saturating_add(SHADE_GESTURE_MIN_TRAVEL)
                    && gesture_is_vertical;
                let closed_shade = point_inside_visible_display(self.start_x, self.start_y)
                    && model.shade_open
                    && y.saturating_add(SHADE_GESTURE_MIN_TRAVEL) <= self.start_y
                    && gesture_is_vertical;
                let opened_drawer = point_inside_visible_display(self.start_x, self.start_y)
                    && model.page == MobilePage::Home
                    && !model.shade_open
                    && !model.drawer_open
                    && drawer_gesture_can_start(self.start_y)
                    && y.saturating_add(DRAWER_GESTURE_MIN_TRAVEL) <= self.start_y
                    && gesture_is_vertical;
                let closed_drawer = point_inside_visible_display(self.start_x, self.start_y)
                    && model.drawer_open
                    && y >= self.start_y.saturating_add(DRAWER_GESTURE_MIN_TRAVEL)
                    && gesture_is_vertical;
                let armed = self.armed.take();
                let shade_dragged = self.shade_dragged;
                let restore_drawer_after_shade = self.restore_drawer_after_shade;
                let drawer_dragged = self.drawer_dragged;
                let page_scroll_dragged = self.page_scroll_dragged;
                let page_scroll_page = self.page_scroll_page;
                let unlock_candidate = self.unlock_candidate;
                let unlock_dragged = self.unlock_dragged;
                let unlock_rejected = self.unlock_rejected;
                let edge_back_candidate = self.edge_back_candidate;
                let edge_back_dragged = self.edge_back_dragged;
                let edge_back_rejected = self.edge_back_rejected;
                let software_dimming_drag = self.software_dimming_drag;
                let boot_notification_dragged = self.boot_notification_dragged;
                let overview_recent_dragged = self.overview_recent_dragged;
                let overview_recent_rejected = self.overview_recent_rejected;
                self.shade_dragged = false;
                self.restore_drawer_after_shade = false;
                self.drawer_dragged = false;
                self.page_scroll_candidate = false;
                self.page_scroll_dragged = false;
                self.page_scroll_page = None;
                self.page_scroll_start_offset_px = 0;
                self.unlock_candidate = false;
                self.unlock_dragged = false;
                self.unlock_rejected = false;
                self.edge_back_candidate = false;
                self.edge_back_dragged = false;
                self.edge_back_rejected = false;
                self.software_dimming_drag = false;
                self.boot_notification_candidate = false;
                self.boot_notification_dragged = false;
                self.boot_notification_rejected = false;
                self.overview_recent_candidate = false;
                self.overview_recent_dragged = false;
                self.overview_recent_rejected = false;
                if software_dimming_drag {
                    if software_dimming_slider_y_contains(model, y) {
                        let level = software_dimming_for_x(x);
                        if level != self.software_dimming_level {
                            self.software_dimming_level = level;
                            return Some(MobileAction::SetSoftwareDimming(level));
                        }
                    }
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                if boot_notification_dragged {
                    let signed_travel = signed_horizontal_travel(self.start_x, x);
                    let release_is_horizontal =
                        horizontal_travel >= vertical_travel.saturating_mul(2);
                    if point_inside_visible_display(x, y)
                        && release_is_horizontal
                        && horizontal_travel >= BOOT_NOTIFICATION_DISMISS_THRESHOLD_PX
                    {
                        return Some(MobileAction::DismissBootNotification);
                    }
                    return (model.boot_notification_offset_px != 0)
                        .then_some(MobileAction::SetBootNotificationOffset(0))
                        .or_else(|| {
                            (signed_travel != 0 && model.pressed_target.is_some())
                                .then_some(MobileAction::SetPressed(None))
                        });
                }
                if overview_recent_dragged {
                    let upward_travel = self.start_y.saturating_sub(y);
                    let release_is_upward = point_inside_visible_display(x, y)
                        && y < self.start_y
                        && vertical_travel >= horizontal_travel.saturating_mul(2);
                    if release_is_upward && upward_travel >= OVERVIEW_RECENT_DISMISS_THRESHOLD_PX {
                        return Some(MobileAction::DismissRecentApp);
                    }
                    return (model.overview_recent_offset_px != 0)
                        .then_some(MobileAction::SetOverviewRecentOffset(0));
                }
                if overview_recent_rejected {
                    return (model.overview_recent_offset_px != 0)
                        .then_some(MobileAction::SetOverviewRecentOffset(0))
                        .or_else(|| {
                            model
                                .pressed_target
                                .is_some()
                                .then_some(MobileAction::SetPressed(None))
                        });
                }
                if unlock_dragged {
                    let upward_travel = self.start_y.saturating_sub(y);
                    let release_is_valid = point_inside_visible_display(x, y)
                        && y < SYSTEM_NAV_TOP_PX
                        && horizontal_travel <= UNLOCK_GESTURE_MAX_HORIZONTAL_DRIFT_PX;
                    if release_is_valid && upward_travel >= UNLOCK_GESTURE_COMMIT_PX {
                        return Some(MobileAction::Unlock);
                    }
                    return (model.unlock_reveal_px != 0)
                        .then_some(MobileAction::SetUnlockReveal(0));
                }
                if unlock_candidate || unlock_rejected {
                    return if model.unlock_reveal_px != 0 {
                        Some(MobileAction::SetUnlockReveal(0))
                    } else {
                        model
                            .pressed_target
                            .is_some()
                            .then_some(MobileAction::SetPressed(None))
                    };
                }
                if edge_back_dragged {
                    let rightward_travel = x.saturating_sub(self.start_x);
                    let release_is_valid = point_inside_visible_display(x, y)
                        && y < SYSTEM_NAV_TOP_PX
                        && vertical_travel <= BACK_GESTURE_MAX_VERTICAL_DRIFT_PX;
                    if release_is_valid && rightward_travel >= BACK_GESTURE_COMMIT_PX {
                        return Some(MobileAction::Back);
                    }
                    return (model.back_reveal_px != 0).then_some(MobileAction::SetBackReveal {
                        reveal_px: 0,
                        origin_y: 0,
                    });
                }
                if edge_back_candidate || edge_back_rejected {
                    return if model.back_reveal_px != 0 {
                        Some(MobileAction::SetBackReveal {
                            reveal_px: 0,
                            origin_y: 0,
                        })
                    } else {
                        model
                            .pressed_target
                            .is_some()
                            .then_some(MobileAction::SetPressed(None))
                    };
                }
                if page_scroll_dragged {
                    // Drag updates are already clamped and quantized. Release
                    // deterministically settles at the last accepted offset
                    // and cannot fall through into the originally armed tap.
                    if page_scroll_page == Some(model.page) {
                        return None;
                    }
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                if opened_shade {
                    return Some(MobileAction::OpenShade);
                }
                if closed_shade {
                    return Some(MobileAction::CloseShade);
                }
                if opened_drawer {
                    return Some(MobileAction::OpenDrawer);
                }
                if closed_drawer {
                    return Some(MobileAction::CloseDrawer);
                }
                if shade_dragged {
                    if restore_drawer_after_shade {
                        return Some(MobileAction::OpenDrawer);
                    }
                    return (model.shade_reveal_px != 0).then_some(if model.shade_open {
                        MobileAction::OpenShade
                    } else {
                        MobileAction::CloseShade
                    });
                }
                if drawer_dragged {
                    return (model.drawer_reveal_px != 0).then_some(if model.drawer_open {
                        MobileAction::OpenDrawer
                    } else {
                        MobileAction::CloseDrawer
                    });
                }
                if armed.is_none() || armed != hit_test(model, x, y) {
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                match armed {
                    Some(MobilePressedTarget::Phone) => Some(MobileAction::Open(MobilePage::Phone)),
                    Some(MobilePressedTarget::OverviewRecent) => {
                        Some(MobileAction::ActivateRecentApp)
                    }
                    Some(MobilePressedTarget::OverviewBackground) => {
                        Some(MobileAction::CloseOverview)
                    }
                    Some(MobilePressedTarget::Messages) => {
                        Some(MobileAction::Open(MobilePage::Messages))
                    }
                    Some(MobilePressedTarget::Calculator) => {
                        Some(MobileAction::Open(MobilePage::Calculator))
                    }
                    Some(MobilePressedTarget::Settings) => {
                        Some(MobileAction::Open(MobilePage::Settings))
                    }
                    Some(MobilePressedTarget::Back) => Some(MobileAction::Back),
                    Some(MobilePressedTarget::SystemHome) => Some(MobileAction::Home),
                    Some(MobilePressedTarget::Display) => {
                        Some(MobileAction::Open(MobilePage::Display))
                    }
                    Some(MobilePressedTarget::Theme) => Some(MobileAction::ToggleTheme),
                    Some(MobilePressedTarget::Accent) => Some(MobileAction::ToggleAccent),
                    Some(MobilePressedTarget::SoftwareDimming) => None,
                    Some(MobilePressedTarget::Accessibility) => {
                        Some(MobileAction::Open(MobilePage::Accessibility))
                    }
                    Some(MobilePressedTarget::LargeText) => Some(MobileAction::ToggleLargeText),
                    Some(MobilePressedTarget::HighContrast) => {
                        Some(MobileAction::ToggleHighContrast)
                    }
                    Some(MobilePressedTarget::BootNotification) => {
                        if model.page == MobilePage::Lock {
                            Some(MobileAction::SetPressed(None))
                        } else {
                            Some(MobileAction::ActivateBootNotification)
                        }
                    }
                    Some(MobilePressedTarget::Apps) => Some(MobileAction::Open(MobilePage::Apps)),
                    #[cfg(feature = "androidbox-multipackage4")]
                    Some(MobilePressedTarget::AppsInstalledAndroid(index)) => {
                        Some(MobileAction::SelectInstalledAndroid(index))
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    Some(MobilePressedTarget::AppsUninstall) => {
                        Some(MobileAction::BeginInstalledAndroidUninstall(
                            model.android_installed_app.generation,
                        ))
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    Some(MobilePressedTarget::AppsUninstallCancel) => {
                        Some(MobileAction::CancelInstalledAndroidUninstall)
                    }
                    #[cfg(feature = "androidbox-runtime-uninstall1")]
                    Some(MobilePressedTarget::AppsUninstallConfirm) => model
                        .android_installed_uninstall
                        .generation()
                        .map(MobileAction::ConfirmInstalledAndroidUninstall),
                    #[cfg(feature = "androidbox-runtime-install2")]
                    Some(MobilePressedTarget::AppsInstall) => {
                        Some(MobileAction::BeginAndroidInstall(
                            model.android_install_candidate.candidate_id,
                        ))
                    }
                    #[cfg(feature = "androidbox-runtime-install2")]
                    Some(MobilePressedTarget::AppsInstallCancel) => {
                        Some(MobileAction::CancelAndroidInstall)
                    }
                    #[cfg(feature = "androidbox-runtime-install2")]
                    Some(MobilePressedTarget::AppsInstallConfirm) => model
                        .android_install
                        .candidate_id()
                        .map(MobileAction::ConfirmAndroidInstall),
                    Some(MobilePressedTarget::About) => Some(MobileAction::Open(MobilePage::About)),
                    Some(MobilePressedTarget::DrawerPhone) => {
                        Some(MobileAction::Open(MobilePage::Phone))
                    }
                    Some(MobilePressedTarget::DrawerMessages) => {
                        Some(MobileAction::Open(MobilePage::Messages))
                    }
                    Some(MobilePressedTarget::DrawerCalculator) => {
                        Some(MobileAction::Open(MobilePage::Calculator))
                    }
                    Some(MobilePressedTarget::DrawerSettings) => {
                        Some(MobileAction::Open(MobilePage::Settings))
                    }
                    Some(MobilePressedTarget::DrawerAndroidBoxDemo) => {
                        Some(MobileAction::Open(MobilePage::AndroidDemo))
                    }
                    Some(MobilePressedTarget::DrawerInstalledAndroid(generation)) => {
                        Some(MobileAction::LaunchInstalledAndroid(generation))
                    }
                    Some(MobilePressedTarget::DrawerHandle | MobilePressedTarget::DrawerHome) => {
                        Some(MobileAction::CloseDrawer)
                    }
                    Some(MobilePressedTarget::AndroidBoxExecute) => {
                        Some(MobileAction::ExecuteAndroidBoxDex)
                    }
                    #[cfg(feature = "androidbox-interactive0")]
                    Some(MobilePressedTarget::InstalledAndroidButton(view_id)) => {
                        Some(MobileAction::ActivateInstalledAndroidButton(view_id))
                    }
                    Some(MobilePressedTarget::ShadeClose) => Some(MobileAction::CloseShade),
                    Some(MobilePressedTarget::PhoneKey(index)) => {
                        Some(MobileAction::PhoneKey(index))
                    }
                    Some(MobilePressedTarget::PhoneBackspace) => Some(MobileAction::PhoneBackspace),
                    Some(MobilePressedTarget::CalculatorKey(index)) => {
                        Some(MobileAction::CalculatorKey(index))
                    }
                    Some(MobilePressedTarget::MessageGuide) => {
                        Some(MobileAction::OpenMessage(MessageView::PreviewGuide))
                    }
                    Some(MobilePressedTarget::MessageOffline) => {
                        Some(MobileAction::OpenMessage(MessageView::OfflineStatus))
                    }
                    None => None,
                }
            }
            (true, true) => {
                let horizontal_travel = x.abs_diff(self.start_x);
                let vertical_travel = y.abs_diff(self.start_y);
                let rightward_travel = x.saturating_sub(self.start_x);
                let upward_travel = self.start_y.saturating_sub(y);
                if self.boot_notification_dragged {
                    if !point_inside_visible_display(x, y)
                        || horizontal_travel < vertical_travel.saturating_mul(2)
                    {
                        self.boot_notification_dragged = false;
                        self.boot_notification_rejected = true;
                        self.armed = None;
                        return (model.boot_notification_offset_px != 0)
                            .then_some(MobileAction::SetBootNotificationOffset(0));
                    }
                    let offset_px = quantize_boot_notification_offset(signed_horizontal_travel(
                        self.start_x,
                        x,
                    ));
                    if offset_px != model.boot_notification_offset_px {
                        return Some(MobileAction::SetBootNotificationOffset(offset_px));
                    }
                    return None;
                }
                if self.boot_notification_candidate {
                    if horizontal_travel.max(vertical_travel)
                        < BOOT_NOTIFICATION_SWIPE_ACTIVATION_PX
                    {
                        return None;
                    }
                    if horizontal_travel >= vertical_travel.saturating_mul(2) {
                        self.boot_notification_candidate = false;
                        self.boot_notification_dragged = true;
                        self.boot_notification_rejected = false;
                        self.shade_dragged = false;
                        self.restore_drawer_after_shade = false;
                        self.drawer_dragged = false;
                        self.armed = None;
                        return Some(MobileAction::SetBootNotificationOffset(
                            signed_horizontal_travel(self.start_x, x),
                        ));
                    }
                    self.boot_notification_candidate = false;
                    self.armed = None;
                    if vertical_travel < horizontal_travel.saturating_mul(2) {
                        self.boot_notification_rejected = true;
                        return model
                            .pressed_target
                            .is_some()
                            .then_some(MobileAction::SetPressed(None));
                    }
                    // A vertical-dominant contact is intentionally yielded
                    // to the settled-shade close gesture below.
                }
                if self.boot_notification_rejected {
                    return None;
                }
                if self.overview_recent_dragged {
                    if !point_inside_visible_display(x, y)
                        || y >= self.start_y
                        || upward_travel < horizontal_travel.saturating_mul(2)
                    {
                        self.overview_recent_dragged = false;
                        self.overview_recent_rejected = true;
                        self.armed = None;
                        return (model.overview_recent_offset_px != 0)
                            .then_some(MobileAction::SetOverviewRecentOffset(0));
                    }
                    let offset_px = quantize_overview_recent_offset(upward_travel);
                    if offset_px != model.overview_recent_offset_px {
                        return Some(MobileAction::SetOverviewRecentOffset(offset_px));
                    }
                    return None;
                }
                if self.overview_recent_candidate {
                    if horizontal_travel.max(vertical_travel) < OVERVIEW_RECENT_SWIPE_ACTIVATION_PX
                    {
                        return None;
                    }
                    if y < self.start_y && upward_travel >= horizontal_travel.saturating_mul(2) {
                        self.overview_recent_candidate = false;
                        self.overview_recent_dragged = true;
                        self.overview_recent_rejected = false;
                        self.shade_dragged = false;
                        self.restore_drawer_after_shade = false;
                        self.drawer_dragged = false;
                        self.edge_back_candidate = false;
                        self.edge_back_dragged = false;
                        self.edge_back_rejected = false;
                        self.armed = None;
                        return Some(MobileAction::SetOverviewRecentOffset(upward_travel));
                    }
                    self.overview_recent_candidate = false;
                    self.overview_recent_rejected = true;
                    self.armed = None;
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                if self.overview_recent_rejected {
                    return None;
                }
                if self.software_dimming_drag {
                    if !software_dimming_slider_y_contains(model, y) {
                        self.software_dimming_drag = false;
                        self.armed = None;
                        return model
                            .pressed_target
                            .is_some()
                            .then_some(MobileAction::SetPressed(None));
                    }
                    let level = software_dimming_for_x(x);
                    if level != self.software_dimming_level {
                        self.software_dimming_level = level;
                        return Some(MobileAction::SetSoftwareDimming(level));
                    }
                    return None;
                }
                if self.unlock_dragged {
                    if !point_inside_visible_display(x, y)
                        || y >= SYSTEM_NAV_TOP_PX
                        || horizontal_travel > UNLOCK_GESTURE_MAX_HORIZONTAL_DRIFT_PX
                    {
                        self.unlock_dragged = false;
                        self.unlock_rejected = true;
                        return (model.unlock_reveal_px != 0)
                            .then_some(MobileAction::SetUnlockReveal(0));
                    }
                    let reveal_px = quantize_unlock_reveal(upward_travel.min(UNLOCK_REVEAL_MAX));
                    if reveal_px != model.unlock_reveal_px {
                        return Some(MobileAction::SetUnlockReveal(reveal_px));
                    }
                    return None;
                }
                if self.unlock_candidate {
                    if horizontal_travel.max(vertical_travel) < UNLOCK_GESTURE_ACTIVATION_PX {
                        return None;
                    }
                    if point_inside_visible_display(x, y)
                        && y < self.start_y
                        && upward_travel >= horizontal_travel.saturating_mul(2)
                    {
                        self.unlock_candidate = false;
                        self.unlock_dragged = true;
                        self.unlock_rejected = false;
                        self.shade_dragged = false;
                        self.restore_drawer_after_shade = false;
                        self.drawer_dragged = false;
                        self.edge_back_candidate = false;
                        self.edge_back_dragged = false;
                        self.edge_back_rejected = false;
                        self.armed = None;
                        return Some(MobileAction::SetUnlockReveal(upward_travel));
                    }
                    self.unlock_candidate = false;
                    self.unlock_rejected = true;
                    self.armed = None;
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                if self.unlock_rejected {
                    return None;
                }
                if self.edge_back_dragged {
                    if !point_inside_visible_display(x, y)
                        || y >= SYSTEM_NAV_TOP_PX
                        || vertical_travel > BACK_GESTURE_MAX_VERTICAL_DRIFT_PX
                    {
                        self.edge_back_dragged = false;
                        self.edge_back_rejected = true;
                        return (model.back_reveal_px != 0).then_some(
                            MobileAction::SetBackReveal {
                                reveal_px: 0,
                                origin_y: 0,
                            },
                        );
                    }
                    let reveal_px =
                        quantize_back_reveal(rightward_travel.min(BACK_GESTURE_REVEAL_MAX));
                    if reveal_px != model.back_reveal_px {
                        return Some(MobileAction::SetBackReveal {
                            reveal_px,
                            origin_y: self.start_y,
                        });
                    }
                    return None;
                }
                if self.edge_back_candidate {
                    if horizontal_travel.max(vertical_travel) < BACK_GESTURE_ACTIVATION_PX {
                        return None;
                    }
                    if x > self.start_x && rightward_travel >= vertical_travel.saturating_mul(2) {
                        self.edge_back_candidate = false;
                        self.edge_back_dragged = true;
                        self.edge_back_rejected = false;
                        self.shade_dragged = false;
                        self.restore_drawer_after_shade = false;
                        self.drawer_dragged = false;
                        self.armed = None;
                        return Some(MobileAction::SetBackReveal {
                            reveal_px: rightward_travel.min(BACK_GESTURE_REVEAL_MAX),
                            origin_y: self.start_y,
                        });
                    }
                    self.edge_back_candidate = false;
                    self.edge_back_rejected = true;
                    self.armed = None;
                    return model
                        .pressed_target
                        .is_some()
                        .then_some(MobileAction::SetPressed(None));
                }
                if self.edge_back_rejected {
                    return None;
                }
                if self.page_scroll_dragged {
                    let Some(page) = self.page_scroll_page else {
                        self.page_scroll_dragged = false;
                        return None;
                    };
                    if page != model.page
                        || !point_inside_visible_display(x, y)
                        || y >= SYSTEM_NAV_TOP_PX
                    {
                        self.page_scroll_dragged = false;
                        self.page_scroll_page = None;
                        return model
                            .pressed_target
                            .is_some()
                            .then_some(MobileAction::SetPressed(None));
                    }
                    let offset_px = page_scroll_offset_for_drag(
                        self.page_scroll_start_offset_px,
                        self.start_y,
                        y,
                        page_scroll_max_px(page),
                    );
                    if offset_px != model.effective_page_scroll_offset_px() {
                        return Some(MobileAction::SetPageScroll { page, offset_px });
                    }
                    return None;
                }
                if self.page_scroll_candidate
                    && horizontal_travel.max(vertical_travel) >= PAGE_SCROLL_ACTIVATION_PX
                {
                    if vertical_travel >= horizontal_travel.saturating_mul(2)
                        && point_inside_visible_display(x, y)
                        && y < SYSTEM_NAV_TOP_PX
                    {
                        let Some(page) = self.page_scroll_page else {
                            self.page_scroll_candidate = false;
                            return None;
                        };
                        self.page_scroll_candidate = false;
                        self.page_scroll_dragged = true;
                        self.shade_dragged = false;
                        self.restore_drawer_after_shade = false;
                        self.drawer_dragged = false;
                        self.armed = None;
                        let offset_px = page_scroll_offset_for_drag(
                            self.page_scroll_start_offset_px,
                            self.start_y,
                            y,
                            page_scroll_max_px(page),
                        );
                        if offset_px != model.effective_page_scroll_offset_px()
                            || model.pressed_target.is_some()
                        {
                            return Some(MobileAction::SetPageScroll { page, offset_px });
                        }
                        return None;
                    }
                    // A non-vertical displacement is not a scroll. Existing
                    // target capture/cancellation remains authoritative.
                    self.page_scroll_candidate = false;
                    self.page_scroll_page = None;
                }
                let gesture_is_vertical = vertical_travel >= horizontal_travel.saturating_mul(2);
                let reveal_px = if point_inside_visible_display(self.start_x, self.start_y)
                    && !model.shade_open
                    && self.start_y < SHADE_GESTURE_START_MAX_Y
                    && y > self.start_y
                    && gesture_is_vertical
                {
                    Some(y.saturating_sub(self.start_y).min(SHADE_REVEAL_MAX))
                } else if point_inside_visible_display(self.start_x, self.start_y)
                    && model.shade_open
                    && y < self.start_y
                    && gesture_is_vertical
                {
                    // Keep one pixel visible until release so zero can remain
                    // the unambiguous "no transient override" sentinel.
                    Some(
                        SHADE_REVEAL_MAX
                            .saturating_sub(self.start_y.saturating_sub(y))
                            .max(1),
                    )
                } else {
                    None
                };
                if let Some(reveal_px) = reveal_px {
                    self.shade_dragged = true;
                    self.restore_drawer_after_shade |= model.drawer_open;
                    self.drawer_dragged = false;
                    self.armed = None;
                    if reveal_px != model.shade_reveal_px {
                        return Some(MobileAction::SetShadeReveal(reveal_px));
                    }
                    return None;
                }
                if self.shade_dragged {
                    if model.shade_reveal_px != 0 {
                        return Some(MobileAction::SetShadeReveal(0));
                    }
                    return None;
                }

                let drawer_reveal_px = if point_inside_visible_display(self.start_x, self.start_y)
                    && model.page == MobilePage::Home
                    && !model.shade_open
                    && !model.drawer_open
                    && drawer_gesture_can_start(self.start_y)
                    && y < self.start_y
                    && gesture_is_vertical
                {
                    Some(quantize_drawer_reveal(
                        self.start_y.saturating_sub(y).min(DRAWER_REVEAL_MAX),
                    ))
                } else if point_inside_visible_display(self.start_x, self.start_y)
                    && model.drawer_open
                    && y > self.start_y
                    && gesture_is_vertical
                {
                    Some(quantize_drawer_reveal(
                        DRAWER_REVEAL_MAX
                            .saturating_sub(y.saturating_sub(self.start_y))
                            .max(1),
                    ))
                } else {
                    None
                };
                if let Some(reveal_px) = drawer_reveal_px {
                    self.drawer_dragged = true;
                    self.armed = None;
                    if reveal_px != model.drawer_reveal_px {
                        return Some(MobileAction::SetDrawerReveal(reveal_px));
                    }
                    return None;
                }
                if self.drawer_dragged {
                    if model.drawer_reveal_px != 0 {
                        return Some(MobileAction::SetDrawerReveal(0));
                    }
                    return None;
                }
                if self.armed != hit_test(model, x, y) {
                    self.armed = None;
                }
                if model.pressed_target != self.armed {
                    return Some(MobileAction::SetPressed(self.armed));
                }
                None
            }
            (false, false) => {
                self.armed = None;
                self.shade_dragged = false;
                self.restore_drawer_after_shade = false;
                self.drawer_dragged = false;
                self.page_scroll_candidate = false;
                self.page_scroll_dragged = false;
                self.page_scroll_page = None;
                self.page_scroll_start_offset_px = 0;
                self.unlock_candidate = false;
                self.unlock_dragged = false;
                self.unlock_rejected = false;
                self.edge_back_candidate = false;
                self.edge_back_dragged = false;
                self.edge_back_rejected = false;
                self.software_dimming_drag = false;
                self.boot_notification_candidate = false;
                self.boot_notification_dragged = false;
                self.boot_notification_rejected = false;
                self.overview_recent_candidate = false;
                self.overview_recent_dragged = false;
                self.overview_recent_rejected = false;
                model
                    .pressed_target
                    .is_some()
                    .then_some(MobileAction::SetPressed(None))
            }
        }
    }

    /// Cancels the captured contact without ever committing its semantic
    /// action. The returned visual restoration action must be applied once.
    pub fn cancel_contact(&mut self, model: MobileModel) -> Option<MobileAction> {
        let was_pressed = self.pressed;
        let action = if model.unlock_reveal_px != 0 {
            Some(MobileAction::SetUnlockReveal(0))
        } else if model.back_reveal_px != 0 {
            Some(MobileAction::SetBackReveal {
                reveal_px: 0,
                origin_y: 0,
            })
        } else if model.boot_notification_offset_px != 0 {
            Some(MobileAction::SetBootNotificationOffset(0))
        } else if model.overview_recent_offset_px != 0 {
            Some(MobileAction::SetOverviewRecentOffset(0))
        } else if self.restore_drawer_after_shade {
            Some(MobileAction::OpenDrawer)
        } else if model.shade_reveal_px != 0 {
            Some(MobileAction::SetShadeReveal(0))
        } else if model.drawer_reveal_px != 0 {
            Some(MobileAction::SetDrawerReveal(0))
        } else {
            model
                .pressed_target
                .is_some()
                .then_some(MobileAction::SetPressed(None))
        };
        *self = Self::new();
        if was_pressed {
            self.pressed = true;
            self.cancelled_until_release = true;
        }
        action
    }
}

const fn drawer_gesture_can_start(y: u16) -> bool {
    y >= DRAWER_GESTURE_START_MIN_Y && y < DRAWER_GESTURE_START_MAX_Y
}

const fn page_scroll_max_px(page: MobilePage) -> u16 {
    match page {
        MobilePage::Settings => SETTINGS_SCROLL_MAX_PX,
        MobilePage::Apps => APPS_SCROLL_MAX_PX,
        MobilePage::Lock
        | MobilePage::Home
        | MobilePage::Phone
        | MobilePage::Messages
        | MobilePage::Calculator
        | MobilePage::AndroidDemo
        | MobilePage::About
        | MobilePage::Display
        | MobilePage::Accessibility => 0,
    }
}

const fn page_scrollable(page: MobilePage) -> bool {
    page_scroll_max_px(page) != 0
}

const fn quantize_page_scroll_offset(offset_px: u16, maximum_px: u16) -> u16 {
    let bounded = if offset_px > maximum_px {
        maximum_px
    } else {
        offset_px
    };
    bounded / PAGE_SCROLL_QUANTUM_PX * PAGE_SCROLL_QUANTUM_PX
}

fn page_scroll_offset_for_drag(start_offset_px: u16, start_y: u16, y: u16, maximum_px: u16) -> u16 {
    let offset = i32::from(start_offset_px) + i32::from(start_y) - i32::from(y);
    let bounded = offset.clamp(0, i32::from(maximum_px)) as u16;
    quantize_page_scroll_offset(bounded, maximum_px)
}

fn page_scroll_content_y(model: MobileModel, y: u16) -> Option<u16> {
    if !(PAGE_SCROLL_VIEWPORT_TOP_PX..PAGE_SCROLL_VIEWPORT_BOTTOM_PX).contains(&y) {
        return None;
    }
    y.checked_add(model.effective_page_scroll_offset_px())
}

fn software_dimming_slider_y_contains(model: MobileModel, y: u16) -> bool {
    if model.shade_open || model.shade_reveal_px != 0 {
        (688..808).contains(&y)
    } else {
        model.page == MobilePage::Display
            && page_scroll_content_y(model, y).is_some_and(|content_y| {
                (DISPLAY_DIMMING_TARGET.y..DISPLAY_DIMMING_TARGET.y + DISPLAY_DIMMING_TARGET.height)
                    .contains(&content_y)
            })
    }
}

fn software_dimming_for_x(x: u16) -> UiSoftwareDimming {
    let clamped = x.clamp(DIMMING_TRACK_START_X_PX, DIMMING_TRACK_END_X_PX);
    // Five evenly spaced stops span 292 physical pixels, or 73 pixels per
    // interval. The left edge is the darkest software level; the right edge
    // is byte-for-byte identical to the undimmed surface.
    let intensity_stop = ((clamped - DIMMING_TRACK_START_X_PX + 36) / 73).min(4);
    match intensity_stop {
        0 => UiSoftwareDimming::Maximum,
        1 => UiSoftwareDimming::Strong,
        2 => UiSoftwareDimming::Medium,
        3 => UiSoftwareDimming::Light,
        _ => UiSoftwareDimming::Off,
    }
}

const fn software_dimming_alpha(level: UiSoftwareDimming) -> u32 {
    match level {
        UiSoftwareDimming::Off => 0,
        UiSoftwareDimming::Light => 51,
        UiSoftwareDimming::Medium => 102,
        UiSoftwareDimming::Strong => 153,
        UiSoftwareDimming::Maximum => 204,
    }
}

const fn software_dimming_percentage(level: UiSoftwareDimming) -> &'static str {
    match level {
        UiSoftwareDimming::Off => "100%",
        UiSoftwareDimming::Light => "80%",
        UiSoftwareDimming::Medium => "60%",
        UiSoftwareDimming::Strong => "40%",
        UiSoftwareDimming::Maximum => "20%",
    }
}

const fn software_dimming_thumb_x(level: UiSoftwareDimming) -> i32 {
    match level {
        UiSoftwareDimming::Maximum => 178,
        UiSoftwareDimming::Strong => 214,
        UiSoftwareDimming::Medium => 251,
        UiSoftwareDimming::Light => 287,
        UiSoftwareDimming::Off => 324,
    }
}

const fn phone_key_byte(index: u8) -> Option<u8> {
    match index {
        0 => Some(b'1'),
        1 => Some(b'2'),
        2 => Some(b'3'),
        3 => Some(b'4'),
        4 => Some(b'5'),
        5 => Some(b'6'),
        6 => Some(b'7'),
        7 => Some(b'8'),
        8 => Some(b'9'),
        9 => Some(b'*'),
        10 => Some(b'0'),
        11 => Some(b'#'),
        _ => None,
    }
}

const fn calculator_key(index: u8) -> Option<CalculatorKey> {
    match index {
        0 => Some(CalculatorKey::Clear),
        1 => Some(CalculatorKey::Sign),
        2 => Some(CalculatorKey::Percent),
        3 => Some(CalculatorKey::Operation(CalculatorOperation::Divide)),
        4 => Some(CalculatorKey::Digit(7)),
        5 => Some(CalculatorKey::Digit(8)),
        6 => Some(CalculatorKey::Digit(9)),
        7 => Some(CalculatorKey::Operation(CalculatorOperation::Multiply)),
        8 => Some(CalculatorKey::Digit(4)),
        9 => Some(CalculatorKey::Digit(5)),
        10 => Some(CalculatorKey::Digit(6)),
        11 => Some(CalculatorKey::Operation(CalculatorOperation::Subtract)),
        12 => Some(CalculatorKey::Digit(1)),
        13 => Some(CalculatorKey::Digit(2)),
        14 => Some(CalculatorKey::Digit(3)),
        15 => Some(CalculatorKey::Operation(CalculatorOperation::Add)),
        16 => Some(CalculatorKey::Digit(0)),
        17 => Some(CalculatorKey::Decimal),
        18 => Some(CalculatorKey::Equals),
        19 => Some(CalculatorKey::Backspace),
        _ => None,
    }
}

const fn calculator_fraction_unit(digits: u8) -> i64 {
    let mut unit = CALCULATOR_SCALE;
    let mut remaining = digits;
    while remaining != 0 {
        unit /= 10;
        remaining -= 1;
    }
    unit
}

const fn calculator_value_in_range(value: i64) -> bool {
    value >= -CALCULATOR_VALUE_LIMIT && value <= CALCULATOR_VALUE_LIMIT
}

fn checked_calculator_value(value: i128) -> Option<i64> {
    if value < -i128::from(CALCULATOR_VALUE_LIMIT) || value > i128::from(CALCULATOR_VALUE_LIMIT) {
        None
    } else {
        i64::try_from(value).ok()
    }
}

fn evaluate_calculator(lhs: i64, rhs: i64, operation: CalculatorOperation) -> Option<i64> {
    let lhs = i128::from(lhs);
    let rhs = i128::from(rhs);
    let value = match operation {
        CalculatorOperation::None => return None,
        CalculatorOperation::Add => lhs.checked_add(rhs)?,
        CalculatorOperation::Subtract => lhs.checked_sub(rhs)?,
        CalculatorOperation::Multiply => lhs.checked_mul(rhs)? / i128::from(CALCULATOR_SCALE),
        CalculatorOperation::Divide if rhs != 0 => {
            lhs.checked_mul(i128::from(CALCULATOR_SCALE))? / rhs
        }
        CalculatorOperation::Divide => return None,
    };
    checked_calculator_value(value)
}

const fn quantize_drawer_reveal(reveal_px: u16) -> u16 {
    let reveal_px = if reveal_px > DRAWER_REVEAL_MAX {
        DRAWER_REVEAL_MAX
    } else {
        reveal_px
    };
    if reveal_px == 0 || reveal_px == DRAWER_REVEAL_MAX {
        return reveal_px;
    }
    let rounded = reveal_px.saturating_add(DRAWER_RENDER_QUANTUM - 1) / DRAWER_RENDER_QUANTUM
        * DRAWER_RENDER_QUANTUM;
    if rounded > DRAWER_REVEAL_MAX {
        DRAWER_REVEAL_MAX
    } else {
        rounded
    }
}

const fn quantize_unlock_reveal(reveal_px: u16) -> u16 {
    let reveal_px = if reveal_px > UNLOCK_REVEAL_MAX {
        UNLOCK_REVEAL_MAX
    } else {
        reveal_px
    };
    if reveal_px == 0 || reveal_px == UNLOCK_REVEAL_MAX {
        return reveal_px;
    }
    let rounded = reveal_px.saturating_add(UNLOCK_RENDER_QUANTUM - 1) / UNLOCK_RENDER_QUANTUM
        * UNLOCK_RENDER_QUANTUM;
    if rounded > UNLOCK_REVEAL_MAX {
        UNLOCK_REVEAL_MAX
    } else {
        rounded
    }
}

const fn quantize_back_reveal(reveal_px: u16) -> u16 {
    let reveal_px = if reveal_px > BACK_GESTURE_REVEAL_MAX {
        BACK_GESTURE_REVEAL_MAX
    } else {
        reveal_px
    };
    if reveal_px == 0 || reveal_px == BACK_GESTURE_REVEAL_MAX {
        return reveal_px;
    }
    let rounded = reveal_px.saturating_add(BACK_GESTURE_RENDER_QUANTUM - 1)
        / BACK_GESTURE_RENDER_QUANTUM
        * BACK_GESTURE_RENDER_QUANTUM;
    if rounded > BACK_GESTURE_REVEAL_MAX {
        BACK_GESTURE_REVEAL_MAX
    } else {
        rounded
    }
}

const fn signed_horizontal_travel(start_x: u16, x: u16) -> i16 {
    let delta = x as i32 - start_x as i32;
    let bounded = if delta > BOOT_NOTIFICATION_MAX_OFFSET_PX as i32 {
        BOOT_NOTIFICATION_MAX_OFFSET_PX as i32
    } else if delta < -(BOOT_NOTIFICATION_MAX_OFFSET_PX as i32) {
        -(BOOT_NOTIFICATION_MAX_OFFSET_PX as i32)
    } else {
        delta
    };
    bounded as i16
}

const fn quantize_boot_notification_offset(offset_px: i16) -> i16 {
    let raw = offset_px as i32;
    let sign = if raw < 0 { -1_i32 } else { 1_i32 };
    let magnitude = if raw < 0 { -raw } else { raw };
    let bounded = if magnitude > BOOT_NOTIFICATION_MAX_OFFSET_PX as i32 {
        BOOT_NOTIFICATION_MAX_OFFSET_PX as i32
    } else {
        magnitude
    };
    let quantum = BOOT_NOTIFICATION_RENDER_QUANTUM_PX as i32;
    (sign * (bounded / quantum * quantum)) as i16
}

const fn quantize_overview_recent_offset(offset_px: u16) -> u16 {
    let bounded = if offset_px > OVERVIEW_RECENT_MAX_OFFSET_PX {
        OVERVIEW_RECENT_MAX_OFFSET_PX
    } else {
        offset_px
    };
    bounded / OVERVIEW_RECENT_RENDER_QUANTUM_PX * OVERVIEW_RECENT_RENDER_QUANTUM_PX
}

fn edge_back_available(model: MobileModel) -> bool {
    !matches!(model.page, MobilePage::Home | MobilePage::Lock)
        || model.overview_open()
        || model.shade_open
        || model.shade_reveal_px != 0
        || model.drawer_open
        || model.drawer_reveal_px != 0
}

const fn drawer_android_pressed_target(model: MobileModel) -> MobilePressedTarget {
    #[cfg(feature = "androidbox-multipackage4")]
    if model.android_installed_app_count != 0 {
        return MobilePressedTarget::DrawerInstalledAndroid(1);
    }
    if model.android_installed_app.installed {
        MobilePressedTarget::DrawerInstalledAndroid(model.android_installed_app.generation)
    } else {
        MobilePressedTarget::DrawerAndroidBoxDemo
    }
}

pub const fn point_inside_visible_display(x: u16, y: u16) -> bool {
    if x >= WIDTH as u16 || y >= HEIGHT as u16 {
        return false;
    }
    let radius = SCREEN_CORNER_RADIUS_PX;
    let right_center_x = WIDTH as u16 - 1 - radius;
    let bottom_center_y = HEIGHT as u16 - 1 - radius;
    let dx = if x < radius {
        radius - x
    } else {
        x.saturating_sub(right_center_x)
    };
    let dy = if y < radius {
        radius - y
    } else {
        y.saturating_sub(bottom_center_y)
    };
    let dx = dx as u32;
    let dy = dy as u32;
    let radius = radius as u32;
    dx * dx + dy * dy <= radius * radius
}

#[cfg(feature = "androidbox-scene-rpc2")]
#[cfg(not(feature = "androidbox-layout-row14"))]
fn installed_android_scene_node_rect(
    scene: AndroidInstalledActivitySceneState,
    index: usize,
) -> Option<DRect> {
    let nodes = scene.nodes();
    let node = *nodes.get(index)?;
    if node.kind() == AndroidSceneViewKind::LinearLayout {
        return None;
    }
    let leaf_count = nodes
        .iter()
        .filter(|node| node.kind() != AndroidSceneViewKind::LinearLayout)
        .count();
    if leaf_count == 0 {
        return None;
    }
    let ordinal = nodes[..index]
        .iter()
        .filter(|node| node.kind() != AndroidSceneViewKind::LinearLayout)
        .count();
    let mut depth = 0usize;
    let mut parent = node.parent();
    while let Some(parent_index) = parent {
        depth = depth.checked_add(1)?;
        if depth > 3 {
            return None;
        }
        parent = nodes.get(usize::from(parent_index))?.parent();
    }
    let pitch = 208_i32 / i32::try_from(leaf_count).ok()?;
    let y = 216_i32 + i32::try_from(ordinal).ok()? * pitch;
    let indent = i32::try_from(depth.saturating_sub(1).min(2)).ok()? * 8;
    Some(DRect::new(
        34 + indent,
        y,
        292 - indent,
        (pitch - 4).max(20),
    ))
}

#[cfg(feature = "androidbox-layout-row14")]
fn installed_android_scene_vertical_units(
    nodes: &[AndroidInstalledActivitySceneNode],
    index: usize,
    depth: u8,
) -> Option<i32> {
    if depth > 3 {
        return None;
    }
    let node = *nodes.get(index)?;
    if node.kind() != AndroidSceneViewKind::LinearLayout {
        return Some(1);
    }
    let mut units = 0_i32;
    for (child_index, child) in nodes.iter().copied().enumerate() {
        if child.parent() == u8::try_from(index).ok() {
            units = units.checked_add(installed_android_scene_vertical_units(
                nodes,
                child_index,
                depth + 1,
            )?)?;
        }
    }
    Some(units)
}

#[cfg(feature = "androidbox-layout-size18")]
fn installed_android_scene_outer_exact_width(
    node: AndroidInstalledActivitySceneNode,
) -> Option<i32> {
    (node.width() == AndroidSceneLayoutSize::Exact).then(|| {
        i32::from(node.exact_width_dp())
            .checked_add(i32::from(node.layout_margin_left_dp()))?
            .checked_add(i32::from(node.layout_margin_right_dp()))
    })?
}

#[cfg(feature = "androidbox-layout-size18")]
fn installed_android_scene_outer_exact_height(
    node: AndroidInstalledActivitySceneNode,
) -> Option<i32> {
    (node.height() == AndroidSceneLayoutSize::Exact).then(|| {
        i32::from(node.exact_height_dp())
            .checked_add(i32::from(node.layout_margin_top_dp()))?
            .checked_add(i32::from(node.layout_margin_bottom_dp()))
    })?
}

#[cfg(all(
    feature = "androidbox-layout-spacing16",
    not(feature = "androidbox-layout-directional17")
))]
fn installed_android_scene_inset(area: DRect, inset_dp: u8) -> Option<DRect> {
    installed_android_scene_inset_directional(area, inset_dp, inset_dp, inset_dp, inset_dp)
}

#[cfg(feature = "androidbox-layout-spacing16")]
fn installed_android_scene_inset_directional(
    area: DRect,
    left_dp: u8,
    top_dp: u8,
    right_dp: u8,
    bottom_dp: u8,
) -> Option<DRect> {
    let left = i32::from(left_dp);
    let top = i32::from(top_dp);
    let right = i32::from(right_dp);
    let bottom = i32::from(bottom_dp);
    let width = area.width.checked_sub(left.checked_add(right)?)?;
    let height = area.height.checked_sub(top.checked_add(bottom)?)?;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some(DRect::new(
        area.x.checked_add(left)?,
        area.y.checked_add(top)?,
        width,
        height,
    ))
}

#[cfg(feature = "androidbox-layout-row14")]
fn installed_android_scene_node_area(
    scene: AndroidInstalledActivitySceneState,
    index: usize,
    depth: u8,
) -> Option<DRect> {
    if depth > 3 {
        return None;
    }
    let nodes = scene.nodes();
    let node = *nodes.get(index)?;
    if index == 0 {
        return (node.kind() == AndroidSceneViewKind::LinearLayout
            && node.orientation() == AndroidSceneOrientation::Vertical)
            .then_some(DRect::new(34, 216, 292, 208));
    }

    let parent_index = usize::from(node.parent()?);
    let parent = *nodes.get(parent_index)?;
    let parent_area = installed_android_scene_node_area(scene, parent_index, depth + 1)?;
    #[cfg(feature = "androidbox-layout-directional17")]
    let parent_area = installed_android_scene_inset_directional(
        parent_area,
        parent.padding_left_dp(),
        parent.padding_top_dp(),
        parent.padding_right_dp(),
        parent.padding_bottom_dp(),
    )?;
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    let parent_area = installed_android_scene_inset(parent_area, parent.padding_dp())?;
    let parent_id = u8::try_from(parent_index).ok()?;

    let area = match parent.orientation() {
        AndroidSceneOrientation::Vertical => {
            #[cfg(feature = "androidbox-layout-size18")]
            {
                let target_exact_height = installed_android_scene_outer_exact_height(node);
                let mut preceding_fixed_height = 0_i32;
                let mut total_fixed_height = 0_i32;
                let mut preceding_flexible_units = 0_i32;
                let mut total_flexible_units = 0_i32;
                let mut target_seen = false;
                let mut target_flexible_units = 0_i32;
                for (child_index, child) in nodes.iter().copied().enumerate() {
                    if child.parent() != Some(parent_id) {
                        continue;
                    }
                    if let Some(exact_height) = installed_android_scene_outer_exact_height(child) {
                        if !target_seen && child_index != index {
                            preceding_fixed_height =
                                preceding_fixed_height.checked_add(exact_height)?;
                        }
                        total_fixed_height = total_fixed_height.checked_add(exact_height)?;
                    } else {
                        let units = installed_android_scene_vertical_units(nodes, child_index, 0)?;
                        if units == 0 {
                            if child_index == index {
                                return None;
                            }
                            continue;
                        }
                        if child_index == index {
                            target_flexible_units = units;
                        } else if !target_seen {
                            preceding_flexible_units =
                                preceding_flexible_units.checked_add(units)?;
                        }
                        total_flexible_units = total_flexible_units.checked_add(units)?;
                    }
                    if child_index == index {
                        target_seen = true;
                    }
                }
                if !target_seen {
                    return None;
                }
                let flexible_height = parent_area
                    .height
                    .checked_sub(total_fixed_height)
                    .filter(|height| *height >= 0)?;
                let flexible_start = if total_flexible_units == 0 {
                    0
                } else {
                    flexible_height
                        .checked_mul(preceding_flexible_units)?
                        .checked_div(total_flexible_units)?
                };
                let height = if let Some(exact_height) = target_exact_height {
                    exact_height
                } else {
                    if target_flexible_units == 0 || total_flexible_units == 0 {
                        return None;
                    }
                    let flexible_end = flexible_height
                        .checked_mul(preceding_flexible_units.checked_add(target_flexible_units)?)?
                        .checked_div(total_flexible_units)?;
                    flexible_end
                        .checked_sub(flexible_start)?
                        .checked_sub(4)?
                        .max(20)
                };
                let width =
                    installed_android_scene_outer_exact_width(node).unwrap_or(parent_area.width);
                if width <= 0 || width > parent_area.width {
                    return None;
                }
                Some(DRect::new(
                    parent_area.x,
                    parent_area
                        .y
                        .checked_add(preceding_fixed_height)?
                        .checked_add(flexible_start)?,
                    width,
                    height,
                ))
            }
            #[cfg(not(feature = "androidbox-layout-size18"))]
            {
                let node_units = installed_android_scene_vertical_units(nodes, index, 0)?;
                if node_units == 0 {
                    return None;
                }
                let mut preceding_units = 0_i32;
                let mut total_units = 0_i32;
                for (child_index, child) in nodes.iter().copied().enumerate() {
                    if child.parent() != Some(parent_id) {
                        continue;
                    }
                    let units = installed_android_scene_vertical_units(nodes, child_index, 0)?;
                    if child_index < index {
                        preceding_units = preceding_units.checked_add(units)?;
                    }
                    total_units = total_units.checked_add(units)?;
                }
                if total_units == 0 {
                    return None;
                }
                let start = parent_area
                    .height
                    .checked_mul(preceding_units)?
                    .checked_div(total_units)?;
                let end = parent_area
                    .height
                    .checked_mul(preceding_units.checked_add(node_units)?)?
                    .checked_div(total_units)?;
                Some(DRect::new(
                    parent_area.x,
                    parent_area.y.checked_add(start)?,
                    parent_area.width,
                    end.checked_sub(start)?.checked_sub(4)?.max(20),
                ))
            }
        }
        AndroidSceneOrientation::Horizontal => {
            #[cfg(feature = "androidbox-layout-mixed19")]
            {
                let mut preceding_fixed_width = 0_i32;
                let mut total_fixed_width = 0_i32;
                let mut preceding_weight = 0_i32;
                let mut total_weight = 0_i32;
                let mut preceding_weighted_margin = 0_i32;
                let mut total_weighted_margin = 0_i32;
                let mut target_fixed_width = None;
                let mut target_weight = 0_i32;
                let mut target_weighted_margin = 0_i32;
                let mut target_seen = false;

                for (child_index, child) in nodes.iter().copied().enumerate() {
                    if child.parent() != Some(parent_id)
                        || installed_android_scene_vertical_units(nodes, child_index, 0)? == 0
                    {
                        continue;
                    }
                    let horizontal_margin = i32::from(child.layout_margin_left_dp())
                        .checked_add(i32::from(child.layout_margin_right_dp()))?;
                    if child.width() == AndroidSceneLayoutSize::Exact && child.layout_weight() == 0
                    {
                        let fixed_width = installed_android_scene_outer_exact_width(child)?;
                        if child_index == index {
                            target_seen = true;
                            target_fixed_width = Some(fixed_width);
                        } else if !target_seen {
                            preceding_fixed_width =
                                preceding_fixed_width.checked_add(fixed_width)?;
                        }
                        total_fixed_width = total_fixed_width.checked_add(fixed_width)?;
                    } else if child.width() == AndroidSceneLayoutSize::Zero
                        && child.layout_weight() != 0
                    {
                        let weight = i32::from(child.layout_weight());
                        if child_index == index {
                            target_seen = true;
                            target_weight = weight;
                            target_weighted_margin = horizontal_margin;
                        } else if !target_seen {
                            preceding_weight = preceding_weight.checked_add(weight)?;
                            preceding_weighted_margin =
                                preceding_weighted_margin.checked_add(horizontal_margin)?;
                        }
                        total_weight = total_weight.checked_add(weight)?;
                        total_weighted_margin =
                            total_weighted_margin.checked_add(horizontal_margin)?;
                    } else {
                        return None;
                    }
                }
                if !target_seen {
                    return None;
                }
                let distributable_width = parent_area
                    .width
                    .checked_sub(total_fixed_width)?
                    .checked_sub(total_weighted_margin)
                    .filter(|width| *width >= 0)?;
                let weighted_start = if total_weight == 0 {
                    0
                } else {
                    distributable_width
                        .checked_mul(preceding_weight)?
                        .checked_div(total_weight)?
                };
                let start = preceding_fixed_width
                    .checked_add(preceding_weighted_margin)?
                    .checked_add(weighted_start)?;
                let width = if let Some(fixed_width) = target_fixed_width {
                    fixed_width
                } else {
                    if target_weight == 0 || total_weight == 0 {
                        return None;
                    }
                    let weighted_end = distributable_width
                        .checked_mul(preceding_weight.checked_add(target_weight)?)?
                        .checked_div(total_weight)?;
                    weighted_end
                        .checked_sub(weighted_start)?
                        .checked_add(target_weighted_margin)?
                };
                Some(DRect::new(
                    parent_area.x.checked_add(start)?,
                    parent_area.y,
                    width,
                    parent_area.height,
                ))
            }
            #[cfg(all(
                feature = "androidbox-layout-weight15",
                not(feature = "androidbox-layout-mixed19")
            ))]
            {
                let mut preceding_weight = 0_i32;
                let mut total_weight = 0_i32;
                let mut target_weight = 0_i32;
                let mut target_seen = false;
                #[cfg(feature = "androidbox-layout-spacing16")]
                let mut preceding_horizontal_margin = 0_i32;
                #[cfg(feature = "androidbox-layout-spacing16")]
                let mut total_horizontal_margin = 0_i32;
                #[cfg(feature = "androidbox-layout-spacing16")]
                let mut target_horizontal_margin = 0_i32;
                for (child_index, child) in nodes.iter().copied().enumerate() {
                    if child.parent() != Some(parent_id)
                        || installed_android_scene_vertical_units(nodes, child_index, 0)? == 0
                    {
                        continue;
                    }
                    let weight = i32::from(child.layout_weight());
                    if weight == 0 || child.width() != AndroidSceneLayoutSize::Zero {
                        return None;
                    }
                    if child_index == index {
                        target_seen = true;
                        target_weight = weight;
                    } else if !target_seen {
                        preceding_weight = preceding_weight.checked_add(weight)?;
                    }
                    total_weight = total_weight.checked_add(weight)?;
                    #[cfg(feature = "androidbox-layout-spacing16")]
                    {
                        let horizontal_margin = i32::from(child.layout_margin_left_dp())
                            .checked_add(i32::from(child.layout_margin_right_dp()))?;
                        if child_index == index {
                            target_horizontal_margin = horizontal_margin;
                        } else if !target_seen {
                            preceding_horizontal_margin =
                                preceding_horizontal_margin.checked_add(horizontal_margin)?;
                        }
                        total_horizontal_margin =
                            total_horizontal_margin.checked_add(horizontal_margin)?;
                    }
                }
                if !target_seen || target_weight == 0 || total_weight == 0 {
                    return None;
                }
                #[cfg(feature = "androidbox-layout-spacing16")]
                let distributable_width = parent_area
                    .width
                    .checked_sub(total_horizontal_margin)?
                    .max(0);
                #[cfg(not(feature = "androidbox-layout-spacing16"))]
                let distributable_width = parent_area.width;
                let start = distributable_width
                    .checked_mul(preceding_weight)?
                    .checked_div(total_weight)?;
                let end = distributable_width
                    .checked_mul(preceding_weight.checked_add(target_weight)?)?
                    .checked_div(total_weight)?;
                #[cfg(feature = "androidbox-layout-spacing16")]
                let width = end
                    .checked_sub(start)?
                    .checked_add(target_horizontal_margin)?;
                #[cfg(not(feature = "androidbox-layout-spacing16"))]
                let width = end.checked_sub(start)?.checked_sub(4)?.max(20);
                Some(DRect::new(
                    parent_area.x.checked_add(start)?.checked_add({
                        #[cfg(feature = "androidbox-layout-spacing16")]
                        {
                            preceding_horizontal_margin
                        }
                        #[cfg(not(feature = "androidbox-layout-spacing16"))]
                        {
                            0
                        }
                    })?,
                    parent_area.y,
                    width,
                    parent_area.height,
                ))
            }
            #[cfg(not(feature = "androidbox-layout-weight15"))]
            {
                let mut ordinal = 0_i32;
                let mut count = 0_i32;
                let mut target_seen = false;
                for (child_index, child) in nodes.iter().copied().enumerate() {
                    if child.parent() != Some(parent_id)
                        || installed_android_scene_vertical_units(nodes, child_index, 0)? == 0
                    {
                        continue;
                    }
                    if child_index == index {
                        ordinal = count;
                        target_seen = true;
                    }
                    count = count.checked_add(1)?;
                }
                if !target_seen || count == 0 {
                    return None;
                }
                let start = parent_area.width.checked_mul(ordinal)?.checked_div(count)?;
                let end = parent_area
                    .width
                    .checked_mul(ordinal.checked_add(1)?)?
                    .checked_div(count)?;
                Some(DRect::new(
                    parent_area.x.checked_add(start)?,
                    parent_area.y,
                    end.checked_sub(start)?.checked_sub(4)?.max(20),
                    parent_area.height,
                ))
            }
        }
        AndroidSceneOrientation::None => None,
    };
    #[cfg(feature = "androidbox-layout-size18")]
    let area = {
        let area = area?;
        if parent.orientation() == AndroidSceneOrientation::Horizontal
            && node.height() == AndroidSceneLayoutSize::Exact
        {
            let exact_height = installed_android_scene_outer_exact_height(node)?;
            if exact_height > parent_area.height {
                return None;
            }
            Some(DRect::new(area.x, area.y, area.width, exact_height))
        } else {
            Some(area)
        }
    };
    #[cfg(feature = "androidbox-layout-directional17")]
    {
        installed_android_scene_inset_directional(
            area?,
            node.layout_margin_left_dp(),
            node.layout_margin_top_dp(),
            node.layout_margin_right_dp(),
            node.layout_margin_bottom_dp(),
        )
    }
    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    {
        installed_android_scene_inset(area?, node.layout_margin_dp())
    }
    #[cfg(not(feature = "androidbox-layout-spacing16"))]
    {
        area
    }
}

#[cfg(feature = "androidbox-layout-row14")]
fn installed_android_scene_node_rect(
    scene: AndroidInstalledActivitySceneState,
    index: usize,
) -> Option<DRect> {
    let node = *scene.nodes().get(index)?;
    (node.kind() != AndroidSceneViewKind::LinearLayout)
        .then(|| installed_android_scene_node_area(scene, index, 0))
        .flatten()
}

#[cfg(feature = "androidbox-layout-row14")]
fn installed_android_scene_node_text_role(
    scene: AndroidInstalledActivitySceneState,
    index: usize,
) -> MobileTextRole {
    let Some(node) = scene.nodes().get(index).copied() else {
        return MobileTextRole::Body;
    };
    let is_horizontal_button = node.kind() == AndroidSceneViewKind::Button
        && node
            .parent()
            .and_then(|parent| scene.nodes().get(usize::from(parent)))
            .is_some_and(|parent| parent.orientation() == AndroidSceneOrientation::Horizontal);
    if is_horizontal_button {
        MobileTextRole::Label
    } else {
        MobileTextRole::Body
    }
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn installed_android_scene_button_target(model: MobileModel, button_id: u32) -> Option<ShellRect> {
    let scene = model.android_installed_activity_scene;
    if !scene.is_active() || !scene.is_callback_button(button_id) {
        return None;
    }
    let (index, _) = scene.find_by_id(button_id)?;
    let rect = installed_android_scene_node_rect(scene, usize::from(index))?;
    Some(ShellRect::new(
        u16::try_from(rect.x.checked_mul(SCALE)?).ok()?,
        u16::try_from(rect.y.checked_mul(SCALE)?).ok()?,
        u16::try_from(rect.width.checked_mul(SCALE)?).ok()?,
        u16::try_from(rect.height.checked_mul(SCALE)?).ok()?,
    ))
}

#[cfg(feature = "mobile-ui-runtime")]
fn clipped_damage_rect(
    x: i32,
    y: i32,
    width: u16,
    height: u16,
    clip: ShellRect,
) -> Option<DamageRect> {
    let right = x.checked_add(i32::from(width))?;
    let bottom = y.checked_add(i32::from(height))?;
    let clip_right = clip.x.checked_add(clip.width)?;
    let clip_bottom = clip.y.checked_add(clip.height)?;
    let left = x.max(i32::from(clip.x));
    let top = y.max(i32::from(clip.y));
    let right = right.min(i32::from(clip_right));
    let bottom = bottom.min(i32::from(clip_bottom));
    if left >= right || top >= bottom {
        return None;
    }
    Some(DamageRect {
        x: u16::try_from(left).ok()?,
        y: u16::try_from(top).ok()?,
        width: u16::try_from(right - left).ok()?,
        height: u16::try_from(bottom - top).ok()?,
    })
}

#[cfg(feature = "mobile-ui-runtime")]
fn shell_target_damage(target: ShellRect) -> Option<DamageRect> {
    clipped_damage_rect(
        i32::from(target.x),
        i32::from(target.y),
        target.width,
        target.height,
        ShellRect::new(0, 0, WIDTH as u16, HEIGHT as u16),
    )
}

#[cfg(feature = "mobile-ui-runtime")]
fn scrolling_page_target_damage(model: MobileModel, target: ShellRect) -> Option<DamageRect> {
    clipped_damage_rect(
        i32::from(target.x),
        i32::from(target.y) - i32::from(model.effective_page_scroll_offset_px()),
        target.width,
        target.height,
        ShellRect::new(
            0,
            PAGE_SCROLL_VIEWPORT_TOP_PX,
            WIDTH as u16,
            PAGE_SCROLL_VIEWPORT_BOTTOM_PX - PAGE_SCROLL_VIEWPORT_TOP_PX,
        ),
    )
}

#[cfg(feature = "mobile-ui-runtime")]
fn stable_page_is(model: MobileModel, page: MobilePage) -> bool {
    model.page == page
        && model.effective_page_transition_offset_px() == 0
        && model.effective_overview_reveal_px() == 0
        && model.effective_drawer_reveal_px() == 0
        && model.effective_shade_reveal_px() == 0
        && model.effective_back_reveal_px() == 0
        && model.effective_unlock_reveal_px() == 0
}

#[cfg(feature = "mobile-ui-runtime")]
fn stable_shade_is_open(model: MobileModel) -> bool {
    model.shade_open
        && model.shade_reveal_px == 0
        && model.effective_shade_reveal_px() == SHADE_REVEAL_MAX
}

#[cfg(feature = "mobile-ui-runtime")]
fn stable_drawer_is_open(model: MobileModel) -> bool {
    model.page == MobilePage::Home
        && model.drawer_open
        && model.drawer_reveal_px == 0
        && model.effective_drawer_reveal_px() == DRAWER_REVEAL_MAX
        && model.effective_shade_reveal_px() == 0
        && model.effective_overview_reveal_px() == 0
}

#[cfg(feature = "mobile-ui-runtime")]
fn pressed_target_damage(model: MobileModel, target: MobilePressedTarget) -> Option<DamageRect> {
    let fixed = |target| shell_target_damage(target);
    match target {
        MobilePressedTarget::SystemHome => fixed(SYSTEM_HOME_TARGET),
        MobilePressedTarget::OverviewRecent
            if model.overview_open()
                && model.effective_shade_reveal_px() == 0
                && model.effective_drawer_reveal_px() == 0 =>
        {
            fixed(OVERVIEW_RECENT_TARGET)
        }
        // OverviewBackground intentionally has no visual pressed feedback.
        // A valid, in-bounds no-op rectangle preserves the single-rectangle
        // wire contract without repainting an unrelated component.
        MobilePressedTarget::OverviewBackground => Some(DamageRect {
            x: WIDTH as u16 / 2,
            y: HEIGHT as u16 / 2,
            width: 1,
            height: 1,
        }),
        MobilePressedTarget::Phone if stable_page_is(model, MobilePage::Home) => {
            fixed(HOME_PHONE_TARGET)
        }
        MobilePressedTarget::Messages if stable_page_is(model, MobilePage::Home) => {
            fixed(HOME_MESSAGES_TARGET)
        }
        MobilePressedTarget::Calculator if stable_page_is(model, MobilePage::Home) => {
            fixed(HOME_CALCULATOR_TARGET)
        }
        MobilePressedTarget::Settings if stable_page_is(model, MobilePage::Home) => {
            fixed(HOME_SETTINGS_TARGET)
        }
        MobilePressedTarget::Back
            if matches!(
                model.page,
                MobilePage::Phone
                    | MobilePage::Messages
                    | MobilePage::Calculator
                    | MobilePage::AndroidDemo
                    | MobilePage::Settings
                    | MobilePage::Apps
                    | MobilePage::About
                    | MobilePage::Display
                    | MobilePage::Accessibility
            ) && stable_page_is(model, model.page) =>
        {
            fixed(APP_BACK_TARGET)
        }
        MobilePressedTarget::Display if stable_page_is(model, MobilePage::Settings) => {
            scrolling_page_target_damage(model, SETTINGS_DISPLAY_TARGET)
        }
        MobilePressedTarget::Theme if stable_shade_is_open(model) => fixed(QUICK_THEME_TARGET),
        MobilePressedTarget::Theme if stable_page_is(model, MobilePage::Display) => {
            fixed(DISPLAY_THEME_TARGET)
        }
        MobilePressedTarget::Accent if stable_shade_is_open(model) => fixed(QUICK_ACCENT_TARGET),
        MobilePressedTarget::Accent if stable_page_is(model, MobilePage::Display) => {
            fixed(DISPLAY_ACCENT_TARGET)
        }
        MobilePressedTarget::SoftwareDimming if stable_shade_is_open(model) => {
            fixed(QUICK_DIMMING_TARGET)
        }
        MobilePressedTarget::SoftwareDimming if stable_page_is(model, MobilePage::Display) => {
            fixed(DISPLAY_DIMMING_TARGET)
        }
        MobilePressedTarget::Accessibility if stable_page_is(model, MobilePage::Display) => {
            fixed(DISPLAY_ACCESSIBILITY_TARGET)
        }
        MobilePressedTarget::LargeText if stable_page_is(model, MobilePage::Accessibility) => {
            fixed(ACCESSIBILITY_LARGE_TEXT_TARGET)
        }
        MobilePressedTarget::HighContrast if stable_page_is(model, MobilePage::Accessibility) => {
            fixed(ACCESSIBILITY_HIGH_CONTRAST_TARGET)
        }
        MobilePressedTarget::BootNotification
            if model.boot_notification_visible
                && (stable_shade_is_open(model) || stable_page_is(model, MobilePage::Lock)) =>
        {
            // The renderer translates on the 2x design grid, so odd manual
            // offsets (normal input is quantized) intentionally round toward
            // zero in exactly the same way here.
            let visual_offset = i32::from(model.boot_notification_offset_px) / SCALE * SCALE;
            let target = if stable_shade_is_open(model) {
                QUICK_BOOT_NOTIFICATION_TARGET
            } else {
                LOCK_BOOT_NOTIFICATION_TARGET
            };
            clipped_damage_rect(
                i32::from(target.x) + visual_offset,
                i32::from(target.y),
                target.width,
                target.height,
                ShellRect::new(0, 0, WIDTH as u16, HEIGHT as u16),
            )
        }
        MobilePressedTarget::Apps if stable_page_is(model, MobilePage::Settings) => {
            scrolling_page_target_damage(model, SETTINGS_APPS_TARGET)
        }
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        MobilePressedTarget::AppsUninstall if stable_page_is(model, MobilePage::Apps) => {
            scrolling_page_target_damage(model, APPS_UNINSTALL_TARGET)
        }
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        MobilePressedTarget::AppsUninstallCancel if stable_page_is(model, MobilePage::Apps) => {
            fixed(APPS_UNINSTALL_CANCEL_TARGET)
        }
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        MobilePressedTarget::AppsUninstallConfirm if stable_page_is(model, MobilePage::Apps) => {
            fixed(APPS_UNINSTALL_CONFIRM_TARGET)
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        MobilePressedTarget::AppsInstall if stable_page_is(model, MobilePage::Apps) => {
            scrolling_page_target_damage(
                model,
                if model.android_installed_app.installed {
                    APPS_UPDATE_TARGET
                } else {
                    APPS_INSTALL_TARGET
                },
            )
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        MobilePressedTarget::AppsInstallCancel if stable_page_is(model, MobilePage::Apps) => {
            fixed(APPS_INSTALL_CANCEL_TARGET)
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        MobilePressedTarget::AppsInstallConfirm if stable_page_is(model, MobilePage::Apps) => {
            fixed(APPS_INSTALL_CONFIRM_TARGET)
        }
        #[cfg(feature = "androidbox-multipackage4")]
        MobilePressedTarget::AppsInstalledAndroid(index)
            if stable_page_is(model, MobilePage::Apps) =>
        {
            let target = match index {
                0 => APPS_INSTALLED_FIRST_TARGET,
                1 => APPS_INSTALLED_SECOND_TARGET,
                _ => return None,
            };
            scrolling_page_target_damage(model, target)
        }
        MobilePressedTarget::About if stable_page_is(model, MobilePage::Settings) => {
            scrolling_page_target_damage(model, SETTINGS_ABOUT_TARGET)
        }
        MobilePressedTarget::DrawerPhone if stable_drawer_is_open(model) => {
            fixed(DRAWER_PHONE_TARGET)
        }
        MobilePressedTarget::DrawerMessages if stable_drawer_is_open(model) => {
            fixed(DRAWER_MESSAGES_TARGET)
        }
        MobilePressedTarget::DrawerCalculator if stable_drawer_is_open(model) => {
            fixed(DRAWER_CALCULATOR_TARGET)
        }
        MobilePressedTarget::DrawerSettings if stable_drawer_is_open(model) => {
            fixed(DRAWER_SETTINGS_TARGET)
        }
        MobilePressedTarget::DrawerAndroidBoxDemo
            if stable_drawer_is_open(model) && drawer_android_pressed_target(model) == target =>
        {
            fixed(DRAWER_ANDROIDBOX_TARGET)
        }
        MobilePressedTarget::DrawerInstalledAndroid(_)
            if stable_drawer_is_open(model) && drawer_android_pressed_target(model) == target =>
        {
            fixed(DRAWER_ANDROIDBOX_TARGET)
        }
        #[cfg(feature = "androidbox-multipackage4")]
        MobilePressedTarget::DrawerInstalledAndroid(2) if stable_drawer_is_open(model) => {
            fixed(DRAWER_ANDROIDBOX_SECOND_TARGET)
        }
        MobilePressedTarget::DrawerHandle if stable_drawer_is_open(model) => {
            fixed(DRAWER_HANDLE_TARGET)
        }
        MobilePressedTarget::DrawerHome if stable_drawer_is_open(model) => {
            fixed(DRAWER_HOME_TARGET)
        }
        MobilePressedTarget::AndroidBoxExecute
            if stable_page_is(model, MobilePage::AndroidDemo) =>
        {
            fixed(ANDROIDBOX_EXECUTE_TARGET)
        }
        #[cfg(feature = "androidbox-interactive0")]
        MobilePressedTarget::InstalledAndroidButton(button_id)
            if stable_page_is(model, MobilePage::AndroidDemo) =>
        {
            #[cfg(feature = "androidbox-scene-rpc2")]
            if let Some(target) = installed_android_scene_button_target(model, button_id) {
                return fixed(target);
            }
            if model.installed_android_foreground_content_ready()
                && model.android_installed_activity_view.is_active()
                && model.android_installed_activity_view.button_view_id() == button_id
            {
                fixed(INSTALLED_ANDROID_BUTTON_TARGET)
            } else {
                None
            }
        }
        MobilePressedTarget::ShadeClose if stable_shade_is_open(model) => fixed(QUICK_CLOSE_TARGET),
        MobilePressedTarget::PhoneKey(index) if stable_page_is(model, MobilePage::Phone) => {
            PHONE_KEY_TARGETS
                .get(usize::from(index))
                .copied()
                .and_then(fixed)
        }
        MobilePressedTarget::PhoneBackspace if stable_page_is(model, MobilePage::Phone) => {
            fixed(PHONE_BACKSPACE_TARGET)
        }
        MobilePressedTarget::CalculatorKey(index)
            if stable_page_is(model, MobilePage::Calculator) =>
        {
            CALCULATOR_KEY_TARGETS
                .get(usize::from(index))
                .copied()
                .and_then(fixed)
        }
        MobilePressedTarget::MessageGuide if stable_page_is(model, MobilePage::Messages) => {
            fixed(MESSAGE_GUIDE_TARGET)
        }
        MobilePressedTarget::MessageOffline if stable_page_is(model, MobilePage::Messages) => {
            fixed(MESSAGE_OFFLINE_TARGET)
        }
        _ => None,
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn plan_damage_regions(candidates: &[Option<DamageRect>]) -> MobileDamagePlan {
    let mut rects = [DamageRect::EMPTY; 2];
    let mut count = 0_usize;
    for candidate in candidates.iter().flatten().copied() {
        let mut merged = false;
        let mut index = 0_usize;
        while index < count {
            if rects[index].intersects(candidate) {
                rects[index] = rects[index].union(candidate);
                merged = true;
                break;
            }
            index += 1;
        }
        if !merged {
            if count == rects.len() {
                return MobileDamagePlan::Full;
            }
            rects[count] = candidate;
            count += 1;
        }
        if count == 2 && rects[0].intersects(rects[1]) {
            rects[0] = rects[0].union(rects[1]);
            rects[1] = DamageRect::EMPTY;
            count = 1;
        }
    }
    if count == 0 {
        return MobileDamagePlan::Unchanged;
    }
    DamageRegions::try_new(&rects[..count])
        .map(MobileDamagePlan::Regions)
        .unwrap_or(MobileDamagePlan::Full)
}

#[cfg(feature = "mobile-ui-runtime")]
fn clock_damage_plan(previous: MobileModel, current: MobileModel) -> Option<MobileDamagePlan> {
    let mut previous_without_time = previous;
    previous_without_time.time = current.time;
    if previous_without_time != current {
        return None;
    }
    if !stable_page_is(current, current.page) && !stable_shade_is_open(current) {
        return Some(MobileDamagePlan::Full);
    }

    let time_changed = previous.time.time_text().as_str() != current.time.time_text().as_str();
    let short_date_changed =
        previous.time.short_date_text().as_str() != current.time.short_date_text().as_str();
    let long_date_changed =
        previous.time.long_date_text().as_str() != current.time.long_date_text().as_str();
    if !time_changed && !short_date_changed && !long_date_changed {
        return Some(MobileDamagePlan::Unchanged);
    }
    let status = time_changed.then_some(STATUS_TIME_DAMAGE);
    if stable_shade_is_open(current) {
        let shade = (time_changed || short_date_changed).then_some(SHADE_TIME_DATE_DAMAGE);
        return Some(plan_damage_regions(&[status, shade]));
    }
    match current.page {
        MobilePage::Home => {
            let content = match (time_changed, long_date_changed) {
                (true, true) => Some(HOME_TIME_DAMAGE.union(HOME_DATE_DAMAGE)),
                (true, false) => Some(HOME_TIME_DAMAGE),
                (false, true) => Some(HOME_DATE_DAMAGE),
                (false, false) => None,
            };
            Some(plan_damage_regions(&[status, content]))
        }
        MobilePage::Lock => {
            let lock = (time_changed || long_date_changed).then_some(LOCK_TIME_DATE_DAMAGE);
            Some(plan_damage_regions(&[status, lock]))
        }
        _ => Some(plan_damage_regions(&[status])),
    }
}

#[cfg(feature = "mobile-ui-runtime")]
fn phone_damage_plan(previous: MobileModel, current: MobileModel) -> Option<MobileDamagePlan> {
    if !stable_page_is(previous, MobilePage::Phone) || !stable_page_is(current, MobilePage::Phone) {
        return None;
    }
    let digits_changed = previous.phone_digits != current.phone_digits
        || previous.phone_digit_count != current.phone_digit_count;
    let mut normalized = previous;
    normalized.phone_digits = current.phone_digits;
    normalized.phone_digit_count = current.phone_digit_count;
    normalized.pressed_target = current.pressed_target;
    if normalized != current
        || (!digits_changed && previous.pressed_target == current.pressed_target)
    {
        return None;
    }
    let base = MobileModel {
        pressed_target: None,
        ..current
    };
    let previous_press = match previous.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    let current_press = match current.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    Some(plan_damage_regions(&[
        previous_press,
        current_press,
        digits_changed.then_some(PHONE_NUMBER_DAMAGE),
    ]))
}

#[cfg(feature = "mobile-ui-runtime")]
fn calculator_damage_plan(previous: MobileModel, current: MobileModel) -> Option<MobileDamagePlan> {
    if !stable_page_is(previous, MobilePage::Calculator)
        || !stable_page_is(current, MobilePage::Calculator)
    {
        return None;
    }
    let semantic_changed = previous.calculator_value != current.calculator_value
        || previous.calculator_accumulator != current.calculator_accumulator
        || previous.calculator_pending != current.calculator_pending
        || previous.calculator_entering != current.calculator_entering
        || previous.calculator_decimal_entered != current.calculator_decimal_entered
        || previous.calculator_fraction_digits != current.calculator_fraction_digits
        || previous.calculator_negative_zero != current.calculator_negative_zero
        || previous.calculator_error != current.calculator_error;
    let mut normalized = previous;
    normalized.calculator_value = current.calculator_value;
    normalized.calculator_accumulator = current.calculator_accumulator;
    normalized.calculator_pending = current.calculator_pending;
    normalized.calculator_entering = current.calculator_entering;
    normalized.calculator_decimal_entered = current.calculator_decimal_entered;
    normalized.calculator_fraction_digits = current.calculator_fraction_digits;
    normalized.calculator_negative_zero = current.calculator_negative_zero;
    normalized.calculator_error = current.calculator_error;
    normalized.pressed_target = current.pressed_target;
    if normalized != current
        || (!semantic_changed && previous.pressed_target == current.pressed_target)
    {
        return None;
    }
    let base = MobileModel {
        pressed_target: None,
        ..current
    };
    let previous_press = match previous.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    let current_press = match current.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    Some(plan_damage_regions(&[
        previous_press,
        current_press,
        semantic_changed.then_some(CALCULATOR_DISPLAY_DAMAGE),
    ]))
}

#[cfg(feature = "mobile-ui-runtime")]
fn boot_notification_visual_damage(model: MobileModel, on_shade: bool) -> DamageRect {
    let base = if on_shade {
        if model.boot_notification_visible {
            SHADE_BOOT_NOTIFICATION_DAMAGE
        } else {
            SHADE_EMPTY_NOTIFICATION_DAMAGE
        }
    } else {
        LOCK_BOOT_NOTIFICATION_DAMAGE
    };
    if !model.boot_notification_visible || model.boot_notification_offset_px == 0 {
        return base;
    }
    // The renderer moves on the 2x design grid. Include both the fixed reveal
    // underlay and the clipped translated card/shadow so partial repainting
    // removes every old pixel without expanding to the full scanout.
    let visual_offset = i32::from(model.boot_notification_offset_px) / SCALE * SCALE;
    let shifted = clipped_damage_rect(
        i32::from(base.x) + visual_offset,
        i32::from(base.y),
        base.width,
        base.height,
        ShellRect::new(0, 0, WIDTH as u16, HEIGHT as u16),
    );
    shifted.map_or(base, |shifted| base.union(shifted))
}

#[cfg(feature = "mobile-ui-runtime")]
fn boot_notification_damage_plan(
    previous: MobileModel,
    current: MobileModel,
) -> Option<MobileDamagePlan> {
    let semantic_changed = previous.boot_notification_visible != current.boot_notification_visible
        || previous.boot_notification_expanded != current.boot_notification_expanded
        || previous.boot_notification_offset_px != current.boot_notification_offset_px;
    if !semantic_changed {
        return None;
    }
    let mut normalized = previous;
    normalized.boot_notification_visible = current.boot_notification_visible;
    normalized.boot_notification_expanded = current.boot_notification_expanded;
    normalized.boot_notification_offset_px = current.boot_notification_offset_px;
    normalized.pressed_target = current.pressed_target;
    if normalized != current {
        return None;
    }

    let previous_on_shade = stable_shade_is_open(previous);
    let current_on_shade = stable_shade_is_open(current);
    let previous_on_lock = stable_page_is(previous, MobilePage::Lock);
    let current_on_lock = stable_page_is(current, MobilePage::Lock);
    if previous_on_shade != current_on_shade || previous_on_lock != current_on_lock {
        return None;
    }
    if previous_on_shade {
        return Some(plan_damage_regions(&[
            Some(boot_notification_visual_damage(previous, true)),
            Some(boot_notification_visual_damage(current, true)),
        ]));
    }
    if previous_on_lock {
        return Some(plan_damage_regions(&[
            Some(boot_notification_visual_damage(previous, false)),
            Some(boot_notification_visual_damage(current, false)),
        ]));
    }
    // The canonical state still changes while ordinary Home/App pages render
    // no notification pixels. Avoid manufacturing a visual transaction.
    Some(MobileDamagePlan::Unchanged)
}

#[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-interactive0"))]
fn installed_activity_damage_plan(
    previous: MobileModel,
    current: MobileModel,
) -> Option<MobileDamagePlan> {
    if !stable_page_is(previous, MobilePage::AndroidDemo)
        || !stable_page_is(current, MobilePage::AndroidDemo)
        || !previous.installed_android_foreground_content_ready()
        || !current.installed_android_foreground_content_ready()
        || !previous.android_installed_activity_view.is_active()
        || !current.android_installed_activity_view.is_active()
    {
        return None;
    }
    #[cfg(feature = "androidbox-scene-rpc2")]
    if previous.android_installed_activity_scene.is_active()
        || current.android_installed_activity_scene.is_active()
    {
        return None;
    }

    let view_changed =
        previous.android_installed_activity_view != current.android_installed_activity_view;
    let mut normalized = previous;
    normalized.android_installed_activity_view = current.android_installed_activity_view;
    normalized.pressed_target = current.pressed_target;
    if normalized != current || (!view_changed && previous.pressed_target == current.pressed_target)
    {
        return None;
    }

    let base = MobileModel {
        pressed_target: None,
        ..current
    };
    let previous_press = match previous.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    let current_press = match current.pressed_target {
        Some(target) => Some(pressed_target_damage(base, target)?),
        None => None,
    };
    Some(plan_damage_regions(&[
        previous_press,
        current_press,
        view_changed.then_some(INSTALLED_ANDROID_LABEL_DAMAGE),
        view_changed.then_some(INSTALLED_ANDROID_BUTTON_DAMAGE),
    ]))
}

/// Plans the smallest verified set of at most two physical regions for one
/// mobile model transition. Unsupported combinations fail safely to `Full`.
#[cfg(feature = "mobile-ui-runtime")]
pub fn damage_plan(previous: Option<MobileModel>, current: MobileModel) -> MobileDamagePlan {
    let Some(previous) = previous else {
        return MobileDamagePlan::Full;
    };
    if previous == current {
        return MobileDamagePlan::Unchanged;
    }
    if let Some(plan) = clock_damage_plan(previous, current) {
        return plan;
    }
    if let Some(plan) = phone_damage_plan(previous, current) {
        return plan;
    }
    if let Some(plan) = calculator_damage_plan(previous, current) {
        return plan;
    }
    if let Some(plan) = boot_notification_damage_plan(previous, current) {
        return plan;
    }
    #[cfg(feature = "androidbox-scene-rpc2")]
    if let Some(plan) = raster::installed_scene_damage_plan(previous, current) {
        return plan;
    }
    #[cfg(feature = "androidbox-interactive0")]
    if let Some(plan) = installed_activity_damage_plan(previous, current) {
        return plan;
    }
    if previous.pressed_target == current.pressed_target {
        return MobileDamagePlan::Full;
    }

    let mut previous_without_press = previous;
    let mut current_without_press = current;
    previous_without_press.pressed_target = None;
    current_without_press.pressed_target = None;
    if previous_without_press != current_without_press {
        return MobileDamagePlan::Full;
    }

    let previous_damage = previous
        .pressed_target
        .and_then(|target| pressed_target_damage(current_without_press, target));
    let current_damage = current
        .pressed_target
        .and_then(|target| pressed_target_damage(current_without_press, target));
    match (
        previous.pressed_target,
        previous_damage,
        current.pressed_target,
        current_damage,
    ) {
        (Some(_), None, _, _) | (_, _, Some(_), None) => MobileDamagePlan::Full,
        (Some(_), Some(previous), Some(_), Some(current)) => {
            plan_damage_regions(&[Some(previous), Some(current)])
        }
        (Some(_), Some(damage), None, None) | (None, None, Some(_), Some(damage)) => {
            plan_damage_regions(&[Some(damage)])
        }
        (None, None, None, None) => MobileDamagePlan::Unchanged,
        _ => MobileDamagePlan::Full,
    }
}

fn hit_test(model: MobileModel, x: u16, y: u16) -> Option<MobilePressedTarget> {
    if !point_inside_visible_display(x, y) {
        return None;
    }
    if model.effective_page_transition_offset_px() != 0 {
        return None;
    }
    if SYSTEM_HOME_TARGET.contains(x, y) {
        return Some(MobilePressedTarget::SystemHome);
    }
    if model.shade_open || model.shade_reveal_px != 0 {
        if !model.shade_open {
            return None;
        }
        if QUICK_CLOSE_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::ShadeClose);
        }
        if QUICK_THEME_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::Theme);
        }
        if QUICK_ACCENT_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::Accent);
        }
        if QUICK_DIMMING_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::SoftwareDimming);
        }
        if model.boot_notification_visible && QUICK_BOOT_NOTIFICATION_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::BootNotification);
        }
        return None;
    }
    if model.drawer_open || model.drawer_reveal_px != 0 {
        if !model.drawer_open {
            return None;
        }
        if DRAWER_HANDLE_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerHandle);
        }
        if DRAWER_PHONE_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerPhone);
        }
        if DRAWER_MESSAGES_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerMessages);
        }
        if DRAWER_CALCULATOR_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerCalculator);
        }
        if DRAWER_SETTINGS_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerSettings);
        }
        if DRAWER_ANDROIDBOX_TARGET.contains(x, y) {
            return Some(drawer_android_pressed_target(model));
        }
        #[cfg(feature = "androidbox-multipackage4")]
        if DRAWER_ANDROIDBOX_SECOND_TARGET.contains(x, y) && model.android_installed_app_count > 1 {
            return Some(MobilePressedTarget::DrawerInstalledAndroid(2));
        }
        if DRAWER_HOME_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::DrawerHome);
        }
        return None;
    }
    if model.overview_open() {
        let recent_is_activatable = match model.system_ui_recent {
            Some(UiRecentIdentity::Shell(_)) => true,
            Some(UiRecentIdentity::CompatibleAndroid(_)) => model.compatible_android_recent_ready(),
            None => false,
        };
        if recent_is_activatable && OVERVIEW_RECENT_TARGET.contains(x, y) {
            return Some(MobilePressedTarget::OverviewRecent);
        }
        // Preserve both system-edge gestures before arming the Overview
        // background: the left strip belongs to Back, while the top strip is
        // reserved for the shade. In particular, a top-edge down must not
        // schedule a pointless Overview-background pressed frame before the
        // first downward sample arrives; doing so can put the real shade
        // gesture behind an unrelated client present. A short tap elsewhere
        // on the system sheet returns Home; the empty identity card itself
        // remains inert.
        if x >= BACK_GESTURE_START_MAX_X
            && y >= SHADE_GESTURE_START_MAX_Y
            && !OVERVIEW_RECENT_TARGET.contains(x, y)
        {
            return Some(MobilePressedTarget::OverviewBackground);
        }
        return None;
    }
    if model.page == MobilePage::Lock
        && model.effective_unlock_reveal_px() == 0
        && model.boot_notification_visible
        && LOCK_BOOT_NOTIFICATION_TARGET.contains(x, y)
    {
        return Some(MobilePressedTarget::BootNotification);
    }
    #[cfg(feature = "androidbox-runtime-install2")]
    if model.page == MobilePage::Apps {
        match model.android_install {
            AndroidInstallStatus::Confirming { .. } => {
                if APPS_INSTALL_CANCEL_TARGET.contains(x, y) {
                    return Some(MobilePressedTarget::AppsInstallCancel);
                }
                if APPS_INSTALL_CONFIRM_TARGET.contains(x, y) {
                    return Some(MobilePressedTarget::AppsInstallConfirm);
                }
                return None;
            }
            AndroidInstallStatus::Pending { .. } => return None,
            AndroidInstallStatus::Failed { .. } => {
                return APPS_INSTALL_CANCEL_TARGET
                    .contains(x, y)
                    .then_some(MobilePressedTarget::AppsInstallCancel);
            }
            AndroidInstallStatus::Idle | AndroidInstallStatus::Installed { .. } => {}
        }
    }
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    if model.page == MobilePage::Apps {
        match model.android_installed_uninstall {
            AndroidInstalledUninstallStatus::Confirming { .. } => {
                if APPS_UNINSTALL_CANCEL_TARGET.contains(x, y) {
                    return Some(MobilePressedTarget::AppsUninstallCancel);
                }
                if APPS_UNINSTALL_CONFIRM_TARGET.contains(x, y) {
                    return Some(MobilePressedTarget::AppsUninstallConfirm);
                }
                return None;
            }
            AndroidInstalledUninstallStatus::Pending { .. } => return None,
            AndroidInstalledUninstallStatus::Failed { .. } => {
                return APPS_UNINSTALL_CANCEL_TARGET
                    .contains(x, y)
                    .then_some(MobilePressedTarget::AppsUninstallCancel);
            }
            AndroidInstalledUninstallStatus::Idle
            | AndroidInstalledUninstallStatus::Removed { .. } => {}
        }
    }
    let page_content_y = page_scroll_content_y(model, y).unwrap_or(y);
    match model.page {
        MobilePage::Home if HOME_PHONE_TARGET.contains(x, y) => Some(MobilePressedTarget::Phone),
        MobilePage::Home if HOME_MESSAGES_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Messages)
        }
        MobilePage::Home if HOME_CALCULATOR_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Calculator)
        }
        MobilePage::Home if HOME_SETTINGS_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Settings)
        }
        MobilePage::Phone if APP_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::Phone if PHONE_BACKSPACE_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::PhoneBackspace)
        }
        MobilePage::Phone => PHONE_KEY_TARGETS
            .iter()
            .position(|target| target.contains(x, y))
            .and_then(|index| u8::try_from(index).ok())
            .map(MobilePressedTarget::PhoneKey),
        MobilePage::Calculator if APP_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::Calculator => CALCULATOR_KEY_TARGETS
            .iter()
            .position(|target| target.contains(x, y))
            .and_then(|index| u8::try_from(index).ok())
            .map(MobilePressedTarget::CalculatorKey),
        MobilePage::AndroidDemo if APP_BACK_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Back)
        }
        #[cfg(feature = "androidbox-scene-rpc2")]
        MobilePage::AndroidDemo
            if model.installed_android_foreground_content_ready()
                && model.android_installed_activity_scene.is_active() =>
        {
            model
                .android_installed_activity_scene
                .callback_button_ids()
                .find(|button_id| {
                    installed_android_scene_button_target(model, *button_id)
                        .is_some_and(|target| target.contains(x, y))
                })
                .map(MobilePressedTarget::InstalledAndroidButton)
        }
        #[cfg(feature = "androidbox-interactive0")]
        MobilePage::AndroidDemo
            if model.installed_android_foreground_content_ready()
                && model.android_installed_activity_view.is_active()
                && INSTALLED_ANDROID_BUTTON_TARGET.contains(x, y) =>
        {
            Some(MobilePressedTarget::InstalledAndroidButton(
                model.android_installed_activity_view.button_view_id(),
            ))
        }
        MobilePage::AndroidDemo
            if !model.android_installed_app.installed
                && ANDROIDBOX_EXECUTE_TARGET.contains(x, y) =>
        {
            Some(MobilePressedTarget::AndroidBoxExecute)
        }
        MobilePage::Messages if APP_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::Messages
            if model.message_view == MessageView::Inbox && MESSAGE_GUIDE_TARGET.contains(x, y) =>
        {
            Some(MobilePressedTarget::MessageGuide)
        }
        MobilePage::Messages
            if model.message_view == MessageView::Inbox
                && MESSAGE_OFFLINE_TARGET.contains(x, y) =>
        {
            Some(MobilePressedTarget::MessageOffline)
        }
        MobilePage::Settings if APP_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::Settings if SETTINGS_DISPLAY_TARGET.contains(x, page_content_y) => {
            Some(MobilePressedTarget::Display)
        }
        MobilePage::Settings if SETTINGS_APPS_TARGET.contains(x, page_content_y) => {
            Some(MobilePressedTarget::Apps)
        }
        MobilePage::Settings if SETTINGS_ABOUT_TARGET.contains(x, page_content_y) => {
            Some(MobilePressedTarget::About)
        }
        MobilePage::Display if APP_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::Display if DISPLAY_THEME_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Theme)
        }
        MobilePage::Display if DISPLAY_ACCENT_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Accent)
        }
        MobilePage::Display if DISPLAY_DIMMING_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::SoftwareDimming)
        }
        MobilePage::Display if DISPLAY_ACCESSIBILITY_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Accessibility)
        }
        MobilePage::Accessibility if APP_BACK_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::Back)
        }
        MobilePage::Accessibility if ACCESSIBILITY_LARGE_TEXT_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::LargeText)
        }
        MobilePage::Accessibility if ACCESSIBILITY_HIGH_CONTRAST_TARGET.contains(x, y) => {
            Some(MobilePressedTarget::HighContrast)
        }
        #[cfg(feature = "androidbox-multipackage4")]
        MobilePage::Apps
            if apps_package_selection_is_available(model)
                && APPS_INSTALLED_FIRST_TARGET.contains(x, page_content_y) =>
        {
            Some(MobilePressedTarget::AppsInstalledAndroid(0))
        }
        #[cfg(feature = "androidbox-multipackage4")]
        MobilePage::Apps
            if apps_package_selection_is_available(model)
                && model.android_installed_app_count > 1
                && APPS_INSTALLED_SECOND_TARGET.contains(x, page_content_y) =>
        {
            Some(MobilePressedTarget::AppsInstalledAndroid(1))
        }
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        MobilePage::Apps
            if model.android_installed_app.installed
                && apps_uninstall_is_available(model)
                && matches!(
                    model.android_installed_uninstall,
                    AndroidInstalledUninstallStatus::Idle
                )
                && APPS_UNINSTALL_TARGET.contains(x, page_content_y) =>
        {
            Some(MobilePressedTarget::AppsUninstall)
        }
        #[cfg(feature = "androidbox-runtime-install2")]
        MobilePage::Apps
            if model.android_install_candidate.present
                && matches!(model.android_install, AndroidInstallStatus::Idle)
                && matches!(
                    model.android_installed_uninstall,
                    AndroidInstalledUninstallStatus::Idle
                        | AndroidInstalledUninstallStatus::Removed { .. }
                )
                && if model.android_installed_app.installed {
                    APPS_UPDATE_TARGET.contains(x, page_content_y)
                } else {
                    APPS_INSTALL_TARGET.contains(x, page_content_y)
                } =>
        {
            Some(MobilePressedTarget::AppsInstall)
        }
        MobilePage::Apps if APPS_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        MobilePage::About if ABOUT_BACK_TARGET.contains(x, y) => Some(MobilePressedTarget::Back),
        _ => None,
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn apps_uninstall_is_available(model: MobileModel) -> bool {
    #[cfg(feature = "androidbox-runtime-install2")]
    {
        !model.android_install_candidate.present
    }
    #[cfg(not(feature = "androidbox-runtime-install2"))]
    {
        let _ = model;
        true
    }
}

#[cfg(feature = "androidbox-multipackage4")]
fn apps_package_selection_is_available(model: MobileModel) -> bool {
    model.android_installed_app_count > 1
        && !model.android_install_candidate.present
        && matches!(
            model.android_install,
            AndroidInstallStatus::Idle | AndroidInstallStatus::Installed { .. }
        )
        && matches!(
            model.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Idle | AndroidInstalledUninstallStatus::Removed { .. }
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    WrongPixelCount,
    InvalidRowRange,
    InvalidDamageRect,
}

/// Minimal SurfaceServer-owned state needed to rasterize trusted mobile chrome.
///
/// Application page, package, overlay, and pressed-target state are
/// deliberately absent: the trusted top and bottom regions cannot vary with
/// client-owned content.
#[cfg(feature = "mobile-system-chrome0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MobileSystemChromeState {
    time: MobileTimeSnapshot,
    dark_theme: bool,
    alternate_accent: bool,
    software_dimming: UiSoftwareDimming,
    large_text: bool,
    high_contrast: bool,
    nav_pressed: bool,
}

#[cfg(feature = "mobile-system-chrome0")]
impl MobileSystemChromeState {
    pub const fn new(
        time: MobileTimeSnapshot,
        dark_theme: bool,
        alternate_accent: bool,
        software_dimming: UiSoftwareDimming,
        large_text: bool,
        high_contrast: bool,
        nav_pressed: bool,
    ) -> Self {
        Self {
            time,
            dark_theme,
            alternate_accent,
            software_dimming,
            large_text,
            high_contrast,
            nav_pressed,
        }
    }

    pub const fn time(self) -> MobileTimeSnapshot {
        self.time
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

    pub const fn large_text(self) -> bool {
        self.large_text
    }

    pub const fn high_contrast(self) -> bool {
        self.high_contrast
    }

    pub const fn nav_pressed(self) -> bool {
        self.nav_pressed
    }
}

/// Semantic type roles for the phone UI's offline raster font.
///
/// Sizes are expressed in physical output pixels. The atlas contains only a
/// bounded printable-ASCII subset; each unsupported scalar maps to one
/// question-mark glyph and does not imply Unicode shaping support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileTextRole {
    Label,
    Caption,
    Body,
    Title,
    Headline,
    Display,
}

impl MobileTextRole {
    pub const fn line_height_px(self) -> u8 {
        match self {
            Self::Label => mobile_font_data::LABEL_LINE_HEIGHT,
            Self::Caption => mobile_font_data::CAPTION_LINE_HEIGHT,
            Self::Body => mobile_font_data::BODY_LINE_HEIGHT,
            Self::Title => mobile_font_data::TITLE_LINE_HEIGHT,
            Self::Headline => mobile_font_data::HEADLINE_LINE_HEIGHT,
            Self::Display => mobile_font_data::DISPLAY_LINE_HEIGHT,
        }
    }
}

pub fn render(pixels: &mut [u32], model: MobileModel) -> Result<(), RenderError> {
    if pixels.len() != PIXEL_COUNT {
        return Err(RenderError::WrongPixelCount);
    }
    render_rows(pixels, 0, model)
}

/// Rebuilds exactly one nonempty physical damage rectangle in an existing
/// complete mobile backing. Every primitive is clipped horizontally and the
/// row slice clips vertically, so pixels outside `damage` remain byte-exact.
#[cfg(feature = "mobile-ui-runtime")]
pub fn render_damage(
    pixels: &mut [u32],
    model: MobileModel,
    damage: DamageRect,
) -> Result<(), RenderError> {
    if pixels.len() != PIXEL_COUNT {
        return Err(RenderError::WrongPixelCount);
    }
    let right = damage
        .x
        .checked_add(damage.width)
        .ok_or(RenderError::InvalidDamageRect)?;
    let bottom = damage
        .y
        .checked_add(damage.height)
        .ok_or(RenderError::InvalidDamageRect)?;
    if damage.width == 0
        || damage.height == 0
        || usize::from(right) > WIDTH
        || usize::from(bottom) > HEIGHT
    {
        return Err(RenderError::InvalidDamageRect);
    }
    let first_row = usize::from(damage.y);
    let last_row = usize::from(bottom);
    let mut canvas = Canvas {
        pixels: &mut pixels[first_row * WIDTH..last_row * WIDTH],
        first_row,
        first_column: 0,
        row_stride: WIDTH,
        row_count: usize::from(damage.height),
        alternate_accent: model.alternate_accent,
        large_text: model.large_text,
        high_contrast: model.high_contrast,
        design_offset_y_px: 0,
        clip_left_x_px: i32::from(damage.x),
        clip_right_x_px: i32::from(right),
        clip_top_y_px: i32::from(damage.y),
        clip_bottom_y_px: i32::from(bottom),
    };
    render_content_layers(&mut canvas, model);
    render_system_chrome(&mut canvas, model);
    render_screen_corner_mask(&mut canvas);
    apply_software_dimming(&mut canvas, model.software_dimming);
    Ok(())
}

/// Rebuilds one canonical protocol-v6 region set in an existing complete
/// backing. Regions are disjoint, so each changed pixel is rastered once and
/// every byte outside the set remains untouched.
#[cfg(feature = "mobile-ui-runtime")]
pub fn render_damage_regions(
    pixels: &mut [u32],
    model: MobileModel,
    damage: DamageRegions,
) -> Result<(), RenderError> {
    for rect in damage.rects() {
        render_damage(pixels, model, *rect)?;
    }
    Ok(())
}

/// Renders one or more complete physical scanlines beginning at `first_row`.
///
/// The supplied slice must contain an integer number of 720-pixel rows. This
/// lets the runtime choose a bounded scanline batch without allocating a full
/// userspace framebuffer.
pub fn render_rows(
    pixels: &mut [u32],
    first_row: usize,
    model: MobileModel,
) -> Result<(), RenderError> {
    let row_count = validated_row_count(pixels, first_row)?;
    let mut canvas = Canvas {
        pixels,
        first_row,
        first_column: 0,
        row_stride: WIDTH,
        row_count,
        alternate_accent: model.alternate_accent,
        large_text: model.large_text,
        high_contrast: model.high_contrast,
        design_offset_y_px: 0,
        clip_left_x_px: 0,
        clip_right_x_px: WIDTH as i32,
        clip_top_y_px: 0,
        clip_bottom_y_px: HEIGHT as i32,
    };
    render_content_layers(&mut canvas, model);
    render_system_chrome(&mut canvas, model);
    render_screen_corner_mask(&mut canvas);
    apply_software_dimming(&mut canvas, model.software_dimming);
    Ok(())
}

/// Renders client-owned rows in the canonical physical content viewport.
///
/// `first_row` is a physical scanout row, while `pixels` is a tightly packed
/// batch of 720-pixel content rows. Status/navigation chrome and the rounded
/// display mask are intentionally omitted.
#[cfg(feature = "mobile-system-chrome0")]
pub fn render_content_rows(
    pixels: &mut [u32],
    first_row: usize,
    model: MobileModel,
) -> Result<(), RenderError> {
    let row_count = validated_row_count(pixels, first_row)?;
    let last_row = first_row + row_count;
    if first_row < usize::from(MOBILE_CONTENT_VIEWPORT_Y)
        || last_row > usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM)
    {
        return Err(RenderError::InvalidRowRange);
    }
    let mut canvas = Canvas {
        pixels,
        first_row,
        first_column: 0,
        row_stride: WIDTH,
        row_count,
        alternate_accent: model.alternate_accent,
        large_text: model.large_text,
        high_contrast: model.high_contrast,
        design_offset_y_px: 0,
        clip_left_x_px: 0,
        clip_right_x_px: WIDTH as i32,
        clip_top_y_px: i32::from(MOBILE_CONTENT_VIEWPORT_Y),
        clip_bottom_y_px: i32::from(MOBILE_CONTENT_VIEWPORT_BOTTOM),
    };
    render_content_layers(&mut canvas, model);
    apply_software_dimming(&mut canvas, model.software_dimming);
    Ok(())
}

/// Renders one contiguous batch wholly inside trusted top or bottom chrome.
///
/// The two disjoint physical regions are `[0, 64)` and `[1512, 1600)`.
/// SurfaceServer calls this function once per region (or in smaller chunks);
/// requests touching client content fail before writing any pixel.
#[cfg(feature = "mobile-system-chrome0")]
pub fn render_system_chrome_rows(
    pixels: &mut [u32],
    first_row: usize,
    state: MobileSystemChromeState,
) -> Result<(), RenderError> {
    let row_count = validated_row_count(pixels, first_row)?;
    let last_row = first_row + row_count;
    let in_top = last_row <= usize::from(MOBILE_CONTENT_VIEWPORT_Y);
    let in_bottom = first_row >= usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM);
    if !in_top && !in_bottom {
        return Err(RenderError::InvalidRowRange);
    }
    let mut canvas = Canvas {
        pixels,
        first_row,
        first_column: 0,
        row_stride: WIDTH,
        row_count,
        alternate_accent: state.alternate_accent,
        large_text: state.large_text,
        high_contrast: state.high_contrast,
        design_offset_y_px: 0,
        clip_left_x_px: 0,
        clip_right_x_px: WIDTH as i32,
        clip_top_y_px: 0,
        clip_bottom_y_px: HEIGHT as i32,
    };
    canvas.wallpaper(state.dark_theme, state.alternate_accent);
    render_system_chrome_state(
        &mut canvas,
        state.time,
        state.dark_theme,
        state.alternate_accent,
        state.nav_pressed,
    );
    render_screen_corner_mask(&mut canvas);
    apply_software_dimming(&mut canvas, state.software_dimming);
    Ok(())
}

fn validated_row_count(pixels: &[u32], first_row: usize) -> Result<usize, RenderError> {
    if pixels.is_empty() || !pixels.len().is_multiple_of(WIDTH) {
        return Err(RenderError::WrongPixelCount);
    }
    let row_count = pixels.len() / WIDTH;
    if first_row >= HEIGHT || first_row.saturating_add(row_count) > HEIGHT {
        return Err(RenderError::InvalidRowRange);
    }
    Ok(row_count)
}

fn render_content_layers(canvas: &mut Canvas<'_>, model: MobileModel) {
    canvas.wallpaper(model.dark_theme, model.alternate_accent);
    let page_transition_offset_px = model.effective_page_transition_offset_px();
    if page_transition_offset_px != 0 && !matches!(model.page, MobilePage::Lock | MobilePage::Home)
    {
        render_page_transition(canvas, model, page_transition_offset_px);
    } else {
        render_page(canvas, model);
    }
    let overview_reveal_px = model.effective_overview_reveal_px();
    if overview_reveal_px != 0 {
        render_overview(canvas, model, overview_reveal_px);
    }
    let drawer_reveal_px = model.effective_drawer_reveal_px();
    if drawer_reveal_px != 0 {
        render_app_drawer(canvas, model, drawer_reveal_px);
    }
    let shade_reveal_px = model.effective_shade_reveal_px();
    if shade_reveal_px != 0 {
        render_quick_settings(canvas, model, shade_reveal_px);
    }
    render_back_gesture(canvas, model);
}

fn apply_software_dimming(canvas: &mut Canvas<'_>, level: UiSoftwareDimming) {
    let dimming_alpha = software_dimming_alpha(level);
    if dimming_alpha != 0 {
        // Apply the session setting to the final composed software surface so
        // every page, overlay, status element, and navigation element changes
        // consistently. This is intentionally not a hardware-backlight path.
        canvas.tint(COLOR_BLACK, dimming_alpha);
    }
}

#[inline(never)]
fn render_page(canvas: &mut Canvas<'_>, model: MobileModel) {
    match model.page {
        MobilePage::Lock => {
            // Keep a real launcher surface underneath the moving lock sheet so
            // the upward gesture reveals its actual destination rather than a
            // synthetic transition frame. Input remains fail-closed until the
            // separate Unlock action commits on release.
            render_home(canvas, model);
            render_lock_screen(canvas, model);
        }
        MobilePage::Home => render_home(canvas, model),
        MobilePage::Phone => render_phone(canvas, model),
        MobilePage::Messages => render_messages(canvas, model),
        MobilePage::Calculator => render_calculator(canvas, model),
        MobilePage::AndroidDemo => render_android_demo(canvas, &model),
        MobilePage::Settings => render_settings(canvas, &model),
        MobilePage::Apps => render_apps(canvas, model),
        MobilePage::About => render_about(canvas, model),
        MobilePage::Display => render_display(canvas, &model),
        MobilePage::Accessibility => render_accessibility(canvas, &model),
    }
}

#[inline(never)]
fn render_page_transition(canvas: &mut Canvas<'_>, model: MobileModel, offset_px: u16) {
    // The outgoing surface is the real Home renderer, not a bitmap stand-in.
    // Keeping both sides code-rendered also makes every transition frame
    // deterministic in the bounded row renderer.
    let mut home = model;
    home.page = MobilePage::Home;
    home.shade_open = false;
    home.shade_reveal_px = 0;
    home.drawer_open = false;
    home.drawer_reveal_px = 0;
    home.back_reveal_px = 0;
    home.back_origin_y = 0;
    home.unlock_reveal_px = 0;
    home.page_transition_offset_px = 0;
    home.pressed_target = None;
    render_home(canvas, home);

    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.design_offset_y_px = previous_offset_y_px + i32::from(offset_px);
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(SYSTEM_NAV_TOP_PX));
    canvas.wallpaper_layer(model.dark_theme, model.alternate_accent);
    canvas.fill_rect(
        DRect::new(0, 0, i32::from(DESIGN_WIDTH), 1),
        model.outline_color(),
    );
    render_page(canvas, model);
    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl DRect {
    const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    const fn screen_inset(gutter: i32, y: i32, height: i32) -> Self {
        Self::new(
            gutter,
            y,
            DESIGN_WIDTH as i32 - gutter.saturating_mul(2),
            height,
        )
    }
}

fn font_role_assets(
    role: MobileTextRole,
) -> (&'static [mobile_font_data::RasterGlyph], &'static [u8; 95]) {
    match role {
        MobileTextRole::Label => (
            &mobile_font_data::LABEL_GLYPHS,
            &mobile_font_data::LABEL_MAP,
        ),
        MobileTextRole::Caption => (
            &mobile_font_data::CAPTION_GLYPHS,
            &mobile_font_data::CAPTION_MAP,
        ),
        MobileTextRole::Body => (&mobile_font_data::BODY_GLYPHS, &mobile_font_data::BODY_MAP),
        MobileTextRole::Title => (
            &mobile_font_data::TITLE_GLYPHS,
            &mobile_font_data::TITLE_MAP,
        ),
        MobileTextRole::Headline => (
            &mobile_font_data::HEADLINE_GLYPHS,
            &mobile_font_data::HEADLINE_MAP,
        ),
        MobileTextRole::Display => (
            &mobile_font_data::DISPLAY_GLYPHS,
            &mobile_font_data::DISPLAY_MAP,
        ),
    }
}

fn printable_ascii_index(character: char) -> Option<usize> {
    let scalar = u32::from(character);
    if !(0x20..=0x7e).contains(&scalar) {
        return None;
    }
    usize::try_from(scalar - 0x20).ok()
}

pub fn mobile_text_has_glyph(role: MobileTextRole, character: char) -> bool {
    let Some(ascii_index) = printable_ascii_index(character) else {
        return false;
    };
    let (_, map) = font_role_assets(role);
    map[ascii_index] != u8::MAX
}

fn raster_glyph(role: MobileTextRole, character: char) -> mobile_font_data::RasterGlyph {
    let (glyphs, map) = font_role_assets(role);
    let question_index = usize::from(b'?' - b' ');
    let ascii_index = printable_ascii_index(character).unwrap_or(question_index);
    let mapped = map[ascii_index];
    let glyph_index = if mapped == u8::MAX {
        map[question_index]
    } else {
        mapped
    };
    glyphs[usize::from(glyph_index)]
}

pub fn measure_mobile_text_px(role: MobileTextRole, text: &str) -> u16 {
    text.chars().fold(0_u16, |width, character| {
        width.saturating_add(u16::from(raster_glyph(role, character).advance))
    })
}

/// Promotes the compact semantic roles by one bounded step. Large headlines
/// and the lock/home clock keep their established geometry, while labels,
/// captions, and body copy gain 20--25% physical height without moving any
/// hit target or introducing a second font atlas.
const fn accessible_text_role(role: MobileTextRole, large_text: bool) -> MobileTextRole {
    if !large_text {
        return role;
    }
    match role {
        MobileTextRole::Label | MobileTextRole::Caption => MobileTextRole::Body,
        MobileTextRole::Body => MobileTextRole::Title,
        MobileTextRole::Title | MobileTextRole::Headline | MobileTextRole::Display => role,
    }
}

struct Canvas<'a> {
    pixels: &'a mut [u32],
    first_row: usize,
    first_column: usize,
    row_stride: usize,
    row_count: usize,
    alternate_accent: bool,
    large_text: bool,
    high_contrast: bool,
    design_offset_y_px: i32,
    clip_left_x_px: i32,
    clip_right_x_px: i32,
    clip_top_y_px: i32,
    clip_bottom_y_px: i32,
}

impl Canvas<'_> {
    fn index(&self, x: usize, y: usize) -> usize {
        (y - self.first_row) * self.row_stride + x - self.first_column
    }

    fn last_row(&self) -> usize {
        self.first_row + self.row_count
    }

    fn wallpaper(&mut self, dark: bool, alternate_accent: bool) {
        let theme = mobile_theme_tokens(dark, alternate_accent, self.high_contrast);
        for local_y in 0..self.row_count {
            let global_y = self.first_row + local_y;
            let color = mix(
                theme.background_top,
                theme.background_bottom,
                global_y as u32,
                (HEIGHT - 1) as u32,
            );
            let start = self.index(self.clip_left_x_px as usize, global_y);
            let end = self.index(self.clip_right_x_px as usize, global_y);
            self.pixels[start..end].fill(color);
        }
        self.radial_glow(
            302,
            92,
            250,
            theme.wallpaper_glow_primary,
            if dark { 74 } else { 54 },
        );
        self.radial_glow(
            58,
            542,
            220,
            theme.wallpaper_glow_secondary,
            if dark { 46 } else { 42 },
        );
    }

    /// Draws an opaque copy of the standard wallpaper at the current vertical
    /// design offset. This is the moving page surface used by deterministic
    /// transitions; the stable zero-offset path continues to use `wallpaper`
    /// directly and therefore remains pixel-identical.
    #[inline(never)]
    fn wallpaper_layer(&mut self, dark: bool, alternate_accent: bool) {
        let theme = mobile_theme_tokens(dark, alternate_accent, self.high_contrast);
        let layer_top = self.design_offset_y_px.max(0);
        let start_y = layer_top.max(self.first_row as i32).max(self.clip_top_y_px);
        let end_y = self
            .clip_bottom_y_px
            .min(self.last_row() as i32)
            .min(HEIGHT as i32);
        for global_y in start_y..end_y {
            let source_y = (global_y - self.design_offset_y_px).clamp(0, HEIGHT as i32 - 1) as u32;
            let color = mix(
                theme.background_top,
                theme.background_bottom,
                source_y,
                (HEIGHT - 1) as u32,
            );
            let start = self.index(self.clip_left_x_px as usize, global_y as usize);
            let end = self.index(self.clip_right_x_px as usize, global_y as usize);
            self.pixels[start..end].fill(color);
        }
        // Keep the moving layer opaque and bounded at `layer_top`. The stable
        // destination restores the full wallpaper glows at offset zero;
        // omitting them here avoids a large radial primitive bleeding above
        // the moving page edge into stationary system chrome.
    }

    fn tint(&mut self, color: u32, alpha: u32) {
        for local_y in 0..self.row_count {
            let global_y = self.first_row + local_y;
            let start = self.index(self.clip_left_x_px as usize, global_y);
            let end = self.index(self.clip_right_x_px as usize, global_y);
            for pixel in &mut self.pixels[start..end] {
                *pixel = blend(*pixel, color, alpha);
            }
        }
    }

    fn radial_glow(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32, opacity: u32) {
        let center_x = center_x * SCALE;
        let center_y = center_y * SCALE + self.design_offset_y_px;
        let radius = radius * SCALE;
        let radius_squared = radius * radius;
        let start_y = (center_y - radius)
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0);
        let end_y = (center_y + radius + 1)
            .min(self.last_row() as i32)
            .min(HEIGHT as i32)
            .min(self.clip_bottom_y_px);
        for y in start_y..end_y {
            for x in self.clip_left_x_px..self.clip_right_x_px {
                let dx = x - center_x;
                let dy = y - center_y;
                let distance = dx * dx + dy * dy;
                if distance >= radius_squared {
                    continue;
                }
                let strength =
                    ((radius_squared - distance) as u32 * opacity) / radius_squared as u32;
                let index = self.index(x as usize, y as usize);
                self.pixels[index] = blend(self.pixels[index], color, strength.min(opacity));
            }
        }
    }

    fn pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0
            || x < self.clip_left_x_px
            || y < self.first_row as i32
            || x >= WIDTH as i32
            || x >= self.clip_right_x_px
            || y >= self.last_row() as i32
            || y >= HEIGHT as i32
            || y < self.clip_top_y_px
            || y >= self.clip_bottom_y_px
        {
            return;
        }
        let index = self.index(x as usize, y as usize);
        self.pixels[index] = color;
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u32) {
        if alpha == 0
            || x < 0
            || x < self.clip_left_x_px
            || y < self.first_row as i32
            || x >= WIDTH as i32
            || x >= self.clip_right_x_px
            || y >= self.last_row() as i32
            || y >= HEIGHT as i32
            || y < self.clip_top_y_px
            || y >= self.clip_bottom_y_px
        {
            return;
        }
        let index = self.index(x as usize, y as usize);
        self.pixels[index] = blend(self.pixels[index], color, alpha.min(255));
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    fn blend_fill_rect(&mut self, rect: DRect, color: u32, alpha: u32) {
        if alpha == 0 {
            return;
        }
        let left = (rect.x * SCALE)
            .max(0)
            .max(self.clip_left_x_px)
            .min(WIDTH as i32);
        let right = ((rect.x + rect.width) * SCALE)
            .max(0)
            .min(self.clip_right_x_px)
            .min(WIDTH as i32);
        let top = (rect.y * SCALE + self.design_offset_y_px)
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0);
        let bottom = ((rect.y + rect.height) * SCALE + self.design_offset_y_px)
            .min(self.last_row() as i32)
            .min(self.clip_bottom_y_px)
            .min(HEIGHT as i32);
        for y in top..bottom {
            for x in left..right {
                self.blend_pixel(x, y, color, alpha);
            }
        }
    }

    fn fill_rect(&mut self, rect: DRect, color: u32) {
        let left = (rect.x * SCALE)
            .max(0)
            .max(self.clip_left_x_px)
            .min(WIDTH as i32);
        let right = ((rect.x + rect.width) * SCALE)
            .max(0)
            .min(self.clip_right_x_px)
            .min(WIDTH as i32);
        let top = (rect.y * SCALE + self.design_offset_y_px)
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0)
            .min(HEIGHT as i32);
        let bottom = ((rect.y + rect.height) * SCALE + self.design_offset_y_px)
            .min(self.last_row() as i32)
            .max(0)
            .min(HEIGHT as i32)
            .min(self.clip_bottom_y_px);
        if left >= right || top >= bottom {
            return;
        }
        for y in top..bottom {
            let start = self.index(left as usize, y as usize);
            let end = self.index(right as usize, y as usize);
            self.pixels[start..end].fill(color);
        }
    }

    fn blend_rect(&mut self, rect: DRect, color: u32, alpha: u32) {
        if alpha == 0 {
            return;
        }
        let left = (rect.x * SCALE)
            .max(0)
            .max(self.clip_left_x_px)
            .min(WIDTH as i32);
        let right = ((rect.x + rect.width) * SCALE)
            .max(0)
            .min(self.clip_right_x_px)
            .min(WIDTH as i32);
        let top = (rect.y * SCALE + self.design_offset_y_px)
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0)
            .min(HEIGHT as i32);
        let bottom = ((rect.y + rect.height) * SCALE + self.design_offset_y_px)
            .min(self.last_row() as i32)
            .max(0)
            .min(HEIGHT as i32)
            .min(self.clip_bottom_y_px);
        if left >= right || top >= bottom {
            return;
        }
        let alpha = alpha.min(255);
        for y in top..bottom {
            for x in left..right {
                let index = self.index(x as usize, y as usize);
                self.pixels[index] = blend(self.pixels[index], color, alpha);
            }
        }
    }

    fn rounded_rect(&mut self, rect: DRect, radius: i32, color: u32) {
        let left = rect.x * SCALE;
        let top = rect.y * SCALE + self.design_offset_y_px;
        let right = (rect.x + rect.width) * SCALE;
        let bottom = (rect.y + rect.height) * SCALE + self.design_offset_y_px;
        let radius = (radius * SCALE)
            .min((right - left) / 2)
            .min((bottom - top) / 2)
            .max(0);
        let radius_squared = radius * radius;
        let start_y = top
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0);
        let end_y = bottom
            .min(self.last_row() as i32)
            .min(HEIGHT as i32)
            .min(self.clip_bottom_y_px);
        let start_x = left.max(0);
        let end_x = right.min(WIDTH as i32);
        for y in start_y..end_y {
            for x in start_x..end_x {
                let dx = if x < left + radius {
                    left + radius - x - 1
                } else if x >= right - radius {
                    x - (right - radius)
                } else {
                    0
                };
                let dy = if y < top + radius {
                    top + radius - y - 1
                } else if y >= bottom - radius {
                    y - (bottom - radius)
                } else {
                    0
                };
                if dx * dx + dy * dy <= radius_squared {
                    self.pixel(x, y, color);
                }
            }
        }
    }

    fn rounded_border(
        &mut self,
        rect: DRect,
        radius: i32,
        border: i32,
        border_color: u32,
        fill: u32,
    ) {
        self.rounded_rect(rect, radius, border_color);
        if rect.width <= border * 2 || rect.height <= border * 2 {
            return;
        }
        self.rounded_rect(
            DRect::new(
                rect.x + border,
                rect.y + border,
                rect.width - border * 2,
                rect.height - border * 2,
            ),
            radius.saturating_sub(border),
            fill,
        );
    }

    fn card(&mut self, rect: DRect, radius: i32, dark: bool, raised: bool) {
        let theme = mobile_theme_tokens(dark, self.alternate_accent, self.high_contrast);
        let fill = if raised {
            theme.surface_raised
        } else {
            theme.surface
        };
        self.elevated_surface(rect, radius, fill, dark, false);
    }

    /// Draws a tactile surface whose shadow and pressed translation remain
    /// strictly inside `rect`. Damage planners may therefore continue using
    /// the control's public hit rectangle without hidden elevation pixels
    /// leaking into a neighbouring region.
    fn elevated_surface(&mut self, rect: DRect, radius: i32, fill: u32, dark: bool, pressed: bool) {
        let theme = mobile_theme_tokens(dark, self.alternate_accent, self.high_contrast);
        let elevation = MobileShapeTokens::ELEVATION_LOW_PX.min(rect.height.saturating_sub(1));
        let face_offset = if pressed {
            elevation.saturating_sub(1)
        } else {
            0
        };
        let face_height = rect.height.saturating_sub(elevation);
        if face_height <= 0 {
            self.rounded_rect(rect, radius, fill);
            return;
        }
        self.rounded_rect(
            DRect::new(
                rect.x,
                rect.y + elevation,
                rect.width,
                rect.height - elevation,
            ),
            radius.saturating_sub(1),
            theme.shadow,
        );
        self.rounded_border(
            DRect::new(rect.x, rect.y + face_offset, rect.width, face_height),
            radius,
            MobileShapeTokens::OUTLINE_PX,
            theme.outline,
            fill,
        );
    }

    fn circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32) {
        self.circle_physical(
            center_x * SCALE,
            center_y * SCALE + self.design_offset_y_px,
            radius * SCALE,
            color,
        );
    }

    fn ring(&mut self, center_x: i32, center_y: i32, radius: i32, width: i32, color: u32) {
        self.aa_disc(center_x, center_y, radius, Some(width), color);
    }

    fn aa_circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32) {
        self.aa_disc(center_x, center_y, radius, None, color);
    }

    fn aa_crescent(&mut self, center_x: i32, center_y: i32, size: i32, color: u32) {
        const SAMPLE_OFFSETS: [i64; 4] = [1, 3, 5, 7];
        let outer_center_x = i64::from(center_x * SCALE) * 8;
        let outer_center_y = i64::from(center_y * SCALE + self.design_offset_y_px) * 8;
        let outer_radius_design = (size * 9 / 24).max(2);
        let inner_radius_design = (size * 8 / 24).max(1);
        let inner_center_x = outer_center_x + i64::from(icon_offset(4, size) * SCALE) * 8;
        let inner_center_y = outer_center_y + i64::from(icon_offset(-3, size) * SCALE) * 8;
        let outer_radius = i64::from(outer_radius_design * SCALE) * 8;
        let inner_radius = i64::from(inner_radius_design * SCALE) * 8;
        let outer_squared = outer_radius * outer_radius;
        let inner_squared = inner_radius * inner_radius;
        let physical_center_x = outer_center_x / 8;
        let physical_center_y = outer_center_y / 8;
        let physical_radius = (outer_radius + 7) / 8;
        let start_y = (physical_center_y - physical_radius - 1)
            .max(self.first_row as i64)
            .max(i64::from(self.clip_top_y_px))
            .max(0);
        let end_y = (physical_center_y + physical_radius + 2)
            .min(self.last_row() as i64)
            .min(HEIGHT as i64)
            .min(i64::from(self.clip_bottom_y_px));
        let start_x = (physical_center_x - physical_radius - 1).max(0);
        let end_x = (physical_center_x + physical_radius + 2).min(WIDTH as i64);
        for y in start_y..end_y {
            for x in start_x..end_x {
                let mut covered = 0_u32;
                for sample_y in SAMPLE_OFFSETS {
                    for sample_x in SAMPLE_OFFSETS {
                        let sample_x = x * 8 + sample_x;
                        let sample_y = y * 8 + sample_y;
                        let outer_dx = sample_x - outer_center_x;
                        let outer_dy = sample_y - outer_center_y;
                        let inner_dx = sample_x - inner_center_x;
                        let inner_dy = sample_y - inner_center_y;
                        if outer_dx * outer_dx + outer_dy * outer_dy <= outer_squared
                            && inner_dx * inner_dx + inner_dy * inner_dy > inner_squared
                        {
                            covered += 1;
                        }
                    }
                }
                if covered != 0 {
                    self.blend_pixel(
                        i32::try_from(x).expect("icon x fits"),
                        i32::try_from(y).expect("icon y fits"),
                        color,
                        (covered * 255 + 8) / 16,
                    );
                }
            }
        }
    }

    /// Blends a 4x4 supersampled disc or ring. Canonical icons use this path
    /// so their coverage is independent of the row-batch partition while
    /// retaining grayscale edge pixels.
    fn aa_disc(
        &mut self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        ring_width: Option<i32>,
        color: u32,
    ) {
        const SAMPLE_OFFSETS: [i64; 4] = [1, 3, 5, 7];
        let center_x = i64::from(center_x * SCALE) * 8;
        let center_y = i64::from(center_y * SCALE + self.design_offset_y_px) * 8;
        let outer_radius = i64::from((radius * SCALE).max(1)) * 8;
        let inner_radius =
            ring_width.map(|width| (outer_radius - i64::from((width * SCALE).max(1)) * 8).max(0));
        let outer_squared = outer_radius * outer_radius;
        let inner_squared = inner_radius.map(|inner| inner * inner);
        let physical_center_x = center_x / 8;
        let physical_center_y = center_y / 8;
        let physical_radius = (outer_radius + 7) / 8;
        let start_y = (physical_center_y - physical_radius - 1)
            .max(self.first_row as i64)
            .max(i64::from(self.clip_top_y_px))
            .max(0);
        let end_y = (physical_center_y + physical_radius + 2)
            .min(self.last_row() as i64)
            .min(HEIGHT as i64)
            .min(i64::from(self.clip_bottom_y_px));
        let start_x = (physical_center_x - physical_radius - 1).max(0);
        let end_x = (physical_center_x + physical_radius + 2).min(WIDTH as i64);
        for y in start_y..end_y {
            for x in start_x..end_x {
                let mut covered = 0_u32;
                for sample_y in SAMPLE_OFFSETS {
                    for sample_x in SAMPLE_OFFSETS {
                        let dx = x * 8 + sample_x - center_x;
                        let dy = y * 8 + sample_y - center_y;
                        let distance = dx * dx + dy * dy;
                        if distance <= outer_squared
                            && inner_squared.is_none_or(|inner| distance >= inner)
                        {
                            covered += 1;
                        }
                    }
                }
                if covered != 0 {
                    self.blend_pixel(
                        i32::try_from(x).expect("icon x fits"),
                        i32::try_from(y).expect("icon y fits"),
                        color,
                        (covered * 255 + 8) / 16,
                    );
                }
            }
        }
    }

    fn circle_physical(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32) {
        let radius_squared = radius * radius;
        let start_y = (center_y - radius)
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0);
        let end_y = (center_y + radius + 1)
            .min(self.last_row() as i32)
            .min(HEIGHT as i32);
        for y in start_y..end_y {
            for x in (center_x - radius).max(0)..=(center_x + radius).min(WIDTH as i32 - 1) {
                let dx = x - center_x;
                let dy = y - center_y;
                if dx * dx + dy * dy <= radius_squared {
                    self.pixel(x, y, color);
                }
            }
        }
    }

    fn round_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, width: i32, color: u32) {
        const SAMPLE_OFFSETS: [i64; 4] = [1, 3, 5, 7];
        let width = width.max(1);
        let x0 = i64::from(x0 * SCALE) * 8;
        let y0 = i64::from(y0 * SCALE + self.design_offset_y_px) * 8;
        let x1 = i64::from(x1 * SCALE) * 8;
        let y1 = i64::from(y1 * SCALE + self.design_offset_y_px) * 8;
        let radius = i64::from(width * SCALE) * 4;
        let radius_squared = radius * radius;
        let physical_margin = (radius + 7) / 8 + 1;
        let start_x = (x0.min(x1) / 8 - physical_margin).max(0);
        let end_x = (x0.max(x1) / 8 + physical_margin + 1).min(WIDTH as i64);
        let start_y = (y0.min(y1) / 8 - physical_margin)
            .max(self.first_row as i64)
            .max(i64::from(self.clip_top_y_px))
            .max(0);
        let end_y = (y0.max(y1) / 8 + physical_margin + 1)
            .min(self.last_row() as i64)
            .min(HEIGHT as i64)
            .min(i64::from(self.clip_bottom_y_px));
        let vx = x1 - x0;
        let vy = y1 - y0;
        let length_squared = vx * vx + vy * vy;
        for y in start_y..end_y {
            for x in start_x..end_x {
                let mut covered = 0_u32;
                for sample_y in SAMPLE_OFFSETS {
                    for sample_x in SAMPLE_OFFSETS {
                        let sample_x = x * 8 + sample_x;
                        let sample_y = y * 8 + sample_y;
                        let wx = sample_x - x0;
                        let wy = sample_y - y0;
                        let projection = wx * vx + wy * vy;
                        let inside = if projection <= 0 || length_squared == 0 {
                            wx * wx + wy * wy <= radius_squared
                        } else if projection >= length_squared {
                            let dx = sample_x - x1;
                            let dy = sample_y - y1;
                            dx * dx + dy * dy <= radius_squared
                        } else {
                            let cross = wx * vy - wy * vx;
                            cross * cross <= radius_squared * length_squared
                        };
                        if inside {
                            covered += 1;
                        }
                    }
                }
                if covered != 0 {
                    self.blend_pixel(
                        i32::try_from(x).expect("icon x fits"),
                        i32::try_from(y).expect("icon y fits"),
                        color,
                        (covered * 255 + 8) / 16,
                    );
                }
            }
        }
    }

    fn text_role(&mut self, x: i32, y: i32, text: &str, role: MobileTextRole, color: u32) {
        let role = accessible_text_role(role, self.large_text);
        self.text_physical(
            x * SCALE,
            y * SCALE + self.design_offset_y_px,
            text,
            role,
            color,
        );
    }

    fn text_center(&mut self, center_x: i32, y: i32, text: &str, role: MobileTextRole, color: u32) {
        let role = accessible_text_role(role, self.large_text);
        let width = i32::from(measure_mobile_text_px(role, text));
        self.text_physical(
            center_x * SCALE - width / 2,
            y * SCALE + self.design_offset_y_px,
            text,
            role,
            color,
        );
    }

    fn text_center_fitted(
        &mut self,
        center_x: i32,
        y: i32,
        text: &str,
        role: MobileTextRole,
        maximum_width_px: u16,
        color: u32,
    ) {
        let resolved_role = accessible_text_role(role, self.large_text);
        let (prefix, truncated) =
            ellipsized_mobile_text_prefix(text, role, maximum_width_px, self.large_text);
        let prefix_width = measure_mobile_text_px(resolved_role, prefix);
        let ellipsis_width = if truncated {
            measure_mobile_text_px(resolved_role, "...")
        } else {
            0
        };
        let width = i32::from(prefix_width.saturating_add(ellipsis_width));
        let start_x = center_x * SCALE - width / 2;
        let origin_y = y * SCALE + self.design_offset_y_px;
        self.text_physical(start_x, origin_y, prefix, resolved_role, color);
        if truncated {
            self.text_physical(
                start_x + i32::from(prefix_width),
                origin_y,
                "...",
                resolved_role,
                color,
            );
        }
    }

    fn text_end(&mut self, end_x: i32, y: i32, text: &str, role: MobileTextRole, color: u32) {
        let role = accessible_text_role(role, self.large_text);
        let width = i32::from(measure_mobile_text_px(role, text));
        self.text_physical(
            end_x * SCALE - width,
            y * SCALE + self.design_offset_y_px,
            text,
            role,
            color,
        );
    }

    fn text_physical(
        &mut self,
        mut cursor_x: i32,
        origin_y: i32,
        text: &str,
        role: MobileTextRole,
        color: u32,
    ) {
        for character in text.chars() {
            let glyph = raster_glyph(role, character);
            self.raster_glyph(cursor_x, origin_y, glyph, color);
            cursor_x = cursor_x.saturating_add(i32::from(glyph.advance));
        }
    }

    fn raster_glyph(
        &mut self,
        cursor_x: i32,
        origin_y: i32,
        glyph: mobile_font_data::RasterGlyph,
        color: u32,
    ) {
        if glyph.width == 0 || glyph.height == 0 {
            return;
        }
        let glyph_width = i32::from(glyph.width);
        let glyph_height = i32::from(glyph.height);
        let destination_left = cursor_x + i32::from(glyph.x_offset);
        let destination_top = origin_y + i32::from(glyph.y_offset);
        let start_x = destination_left.max(0);
        let end_x = (destination_left + glyph_width).min(WIDTH as i32);
        let start_y = destination_top
            .max(self.first_row as i32)
            .max(self.clip_top_y_px)
            .max(0);
        let end_y = (destination_top + glyph_height)
            .min(self.last_row() as i32)
            .min(HEIGHT as i32)
            .min(self.clip_bottom_y_px);
        if start_x >= end_x || start_y >= end_y {
            return;
        }
        let data_offset =
            usize::try_from(glyph.data_offset).expect("font atlas offset fits in usize");
        let width = usize::from(glyph.width);
        for y in start_y..end_y {
            let glyph_y =
                usize::try_from(y - destination_top).expect("clipped glyph row is non-negative");
            for x in start_x..end_x {
                let glyph_x = usize::try_from(x - destination_left)
                    .expect("clipped glyph column is non-negative");
                let pixel_index = glyph_y * width + glyph_x;
                let packed = FONT_ALPHA4[data_offset + pixel_index / 2];
                let coverage = if pixel_index.is_multiple_of(2) {
                    packed >> 4
                } else {
                    packed & 0x0f
                };
                self.blend_pixel(x, y, color, u32::from(coverage) * 17);
            }
        }
    }

    fn toggle(&mut self, x: i32, y: i32, enabled: bool, accent: u32) {
        let color = if enabled { accent } else { COLOR_TOGGLE_OFF };
        self.rounded_rect(DRect::new(x, y, 52, 30), 15, color);
        let knob_x = if enabled { x + 37 } else { x + 15 };
        self.circle(knob_x, y + 15, 11, COLOR_WHITE);
    }
}

fn unavailable_clock_geometry(role: MobileTextRole) -> (i32, i32, i32, i32, i32) {
    match role {
        MobileTextRole::Label | MobileTextRole::Caption => (6, 3, 1, 6, 3),
        MobileTextRole::Body | MobileTextRole::Title | MobileTextRole::Headline => {
            (10, 4, 2, 12, 5)
        }
        MobileTextRole::Display => (18, 5, 2, 25, 8),
    }
}

fn unavailable_clock_width(role: MobileTextRole) -> i32 {
    let (dash_width, gap, colon_radius, _, _) = unavailable_clock_geometry(role);
    4 * dash_width + 4 * gap + 2 * colon_radius
}

fn draw_unavailable_clock(
    canvas: &mut Canvas<'_>,
    x: i32,
    y: i32,
    role: MobileTextRole,
    color: u32,
) {
    let role = accessible_text_role(role, canvas.large_text);
    let (dash_width, gap, colon_radius, center_y, colon_offset_y) =
        unavailable_clock_geometry(role);
    let dash_height = colon_radius.max(1);
    let first = x;
    let second = first + dash_width + gap;
    let colon_x = second + dash_width + gap + colon_radius;
    let third = colon_x + colon_radius + gap;
    let fourth = third + dash_width + gap;
    let dash_y = y + center_y - dash_height / 2;
    for dash_x in [first, second, third, fourth] {
        canvas.rounded_rect(
            DRect::new(dash_x, dash_y, dash_width, dash_height),
            dash_height / 2,
            color,
        );
    }
    canvas.circle(colon_x, y + center_y - colon_offset_y, colon_radius, color);
    canvas.circle(colon_x, y + center_y + colon_offset_y, colon_radius, color);
}

fn draw_time_role(
    canvas: &mut Canvas<'_>,
    x: i32,
    y: i32,
    snapshot: MobileTimeSnapshot,
    role: MobileTextRole,
    color: u32,
) {
    if snapshot.is_available() {
        let time = snapshot.time_text();
        canvas.text_role(x, y, time.as_str(), role, color);
    } else {
        draw_unavailable_clock(canvas, x, y, role, color);
    }
}

fn draw_time_center(
    canvas: &mut Canvas<'_>,
    center_x: i32,
    y: i32,
    snapshot: MobileTimeSnapshot,
    role: MobileTextRole,
    color: u32,
) {
    if snapshot.is_available() {
        let time = snapshot.time_text();
        canvas.text_center(center_x, y, time.as_str(), role, color);
    } else {
        let effective_role = accessible_text_role(role, canvas.large_text);
        draw_unavailable_clock(
            canvas,
            center_x - unavailable_clock_width(effective_role) / 2,
            y,
            role,
            color,
        );
    }
}

fn render_system_chrome(canvas: &mut Canvas<'_>, model: MobileModel) {
    render_system_chrome_state(
        canvas,
        model.time,
        model.dark_theme,
        model.alternate_accent,
        model.system_nav_pressed
            || model.is_pressed(MobilePressedTarget::DrawerHome)
            || model.is_pressed(MobilePressedTarget::SystemHome),
    );
}

fn render_system_chrome_state(
    canvas: &mut Canvas<'_>,
    time: MobileTimeSnapshot,
    dark_theme: bool,
    alternate_accent: bool,
    nav_pressed: bool,
) {
    let theme = mobile_theme_tokens(dark_theme, alternate_accent, canvas.high_contrast);
    let text = theme.text_primary;
    // A single status layer shared by launcher and apps.
    draw_time_role(canvas, 20, 14, time, MobileTextRole::Label, text);
    // This small preview safe-area outline keeps content clear of a common
    // centered camera position. It is not a claim that the emulator has a
    // camera, sensor, display cutout, or physical panel.
    canvas.circle(180, 18, 5, COLOR_BLACK);
    // The preview is intentionally isolated, so never imply a live radio or
    // Wi-Fi connection in the status area.
    draw_icon(canvas, 303, 19, IconSize::Status, text, Icon::Airplane);
    draw_icon(canvas, 338, 19, IconSize::List, text, Icon::BatteryUnknown);

    // Gesture navigation is rendered once, in the system layer.
    let nav_back = if dark_theme {
        blend(theme.background_bottom, COLOR_BLACK, 68)
    } else {
        blend(theme.background_bottom, COLOR_WHITE, 148)
    };
    canvas.fill_rect(DRect::new(0, 774, 360, 26), nav_back);
    if nav_pressed {
        canvas.rounded_rect(
            DRect::new(122, 776, 116, 22),
            11,
            blend(
                theme.surface_raised,
                if alternate_accent {
                    COLOR_BLUE_ALT
                } else {
                    COLOR_BLUE
                },
                if dark_theme { 112 } else { 58 },
            ),
        );
    }
    canvas.rounded_rect(
        DRect::new(130, 787, 100, MobileSpacingTokens::XS),
        MobileSpacingTokens::XXS,
        text,
    );
}

fn render_back_gesture(canvas: &mut Canvas<'_>, model: MobileModel) {
    let reveal_px = model.effective_back_reveal_px();
    if reveal_px == 0 {
        return;
    }
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(SYSTEM_NAV_TOP_PX));

    let reveal = i32::from(reveal_px) / SCALE;
    let center_y = (i32::from(model.back_origin_y) / SCALE).clamp(88, 744);
    let width = 32 + reveal * 20 / (i32::from(BACK_GESTURE_REVEAL_MAX) / SCALE);
    let center_x = (width - 18).clamp(18, 34);
    let surface = blend(model.surface_color(true), model.accent(), 92);
    canvas.rounded_rect(
        DRect::new(-10, center_y - 24, width, 48),
        MobileShapeTokens::RADIUS_LARGE,
        model.theme_tokens().shadow,
    );
    canvas.rounded_rect(DRect::new(-12, center_y - 25, width, 48), 24, surface);
    draw_icon(
        canvas,
        center_x,
        center_y - 1,
        IconSize::List,
        model.text_primary(),
        Icon::ChevronLeft,
    );
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

fn render_screen_corner_mask(canvas: &mut Canvas<'_>) {
    let radius = i32::from(SCREEN_CORNER_RADIUS_PX);
    let radius_squared = radius * radius;
    let right_center_x = WIDTH as i32 - 1 - radius;
    let bottom_center_y = HEIGHT as i32 - 1 - radius;
    let start_y = canvas.first_row as i32;
    let end_y = canvas.last_row() as i32;

    for y in start_y..end_y {
        let corner_dy = if y <= radius {
            Some(y - radius)
        } else if y >= bottom_center_y {
            Some(y - bottom_center_y)
        } else {
            None
        };
        let Some(dy) = corner_dy else {
            continue;
        };
        for x in 0..=radius {
            let dx = x - radius;
            if dx * dx + dy * dy <= radius_squared {
                continue;
            }
            canvas.pixel(x, y, COLOR_BLACK);
            canvas.pixel(right_center_x + radius - x, y, COLOR_BLACK);
        }
    }
}

#[inline(never)]
fn render_home(canvas: &mut Canvas<'_>, model: MobileModel) {
    let text = model.text_primary();
    let muted = model.text_secondary();
    let date = model.time.long_date_text();

    draw_time_role(canvas, 22, 68, model.time, MobileTextRole::Display, text);

    // Keep the launcher quiet and wallpaper-led. Runtime diagnostics belong
    // in About; the Home surface must not advertise inert controls.
    canvas.elevated_surface(
        DRect::screen_inset(MobileSpacingTokens::PAGE_GUTTER, 164, 80),
        MobileShapeTokens::RADIUS_LARGE,
        model.surface_color(true),
        model.dark_theme,
        false,
    );
    let calendar_surface = if model.time.is_available() {
        model.accent()
    } else {
        model.surface_color(false)
    };
    let calendar_ink = if model.time.is_available() {
        model.on_accent()
    } else {
        muted
    };
    canvas.circle(52, 204, 22, calendar_surface);
    draw_icon(
        canvas,
        52,
        204,
        IconSize::List,
        calendar_ink,
        Icon::Calendar,
    );
    canvas.text_role(84, 174, "Today", MobileTextRole::Headline, text);
    canvas.text_role(84, 210, date.as_str(), MobileTextRole::Caption, muted);

    // A single page indicator communicates launcher position without looking
    // like another tappable control.
    canvas.circle(180, 646, 3, muted);

    // Bottom dock and its hit targets use the same geometry as the shell.
    canvas.elevated_surface(
        DRect::screen_inset(14, 676, 84),
        MobileShapeTokens::RADIUS_LARGE,
        model.surface_color(true),
        model.dark_theme,
        false,
    );
    for (target, x) in [
        (MobilePressedTarget::Phone, 52),
        (MobilePressedTarget::Messages, 138),
        (MobilePressedTarget::Calculator, 224),
        (MobilePressedTarget::Settings, 310),
    ] {
        if model.is_pressed(target) {
            canvas.circle(x, 706, 31, pressed_surface_color(model));
        }
    }
    draw_app_icon(canvas, 52, 706, COLOR_PHONE, Icon::Phone);
    draw_app_icon(canvas, 138, 706, COLOR_MESSAGES, Icon::Messages);
    draw_app_icon(canvas, 224, 706, COLOR_CALCULATOR, Icon::Calculator);
    draw_app_icon(canvas, 310, 706, COLOR_SETTINGS, Icon::Settings);
    canvas.text_center(52, 737, "Phone", MobileTextRole::Label, muted);
    canvas.text_center(138, 737, "Messages", MobileTextRole::Label, muted);
    canvas.text_center(224, 737, "Calculator", MobileTextRole::Label, muted);
    canvas.text_center(310, 737, "Settings", MobileTextRole::Label, muted);
}

#[inline(never)]
fn render_lock_screen(canvas: &mut Canvas<'_>, model: MobileModel) {
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let date = model.time.long_date_text();
    let theme = model.theme_tokens();
    let sheet = theme.background_top;
    let reveal_px = model.effective_unlock_reveal_px();
    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.design_offset_y_px = previous_offset_y_px - i32::from(reveal_px);
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(SYSTEM_NAV_TOP_PX));

    // The oversized rounded sheet fully covers the stable display. Its bottom
    // edge becomes visible only while the contact-follow unlock preview moves.
    canvas.rounded_rect(DRect::new(-24, -20, 408, 850), 40, theme.shadow);
    canvas.rounded_rect(DRect::new(-24, -24, 408, 850), 40, sheet);
    canvas.radial_glow(
        304,
        if model.dark_theme { 92 } else { 82 },
        if model.dark_theme { 248 } else { 238 },
        theme.wallpaper_glow_primary,
        if model.dark_theme { 78 } else { 58 },
    );
    canvas.radial_glow(
        if model.dark_theme { 42 } else { 38 },
        if model.dark_theme { 510 } else { 500 },
        if model.dark_theme { 220 } else { 210 },
        theme.wallpaper_glow_secondary,
        if model.dark_theme { 42 } else { 38 },
    );

    canvas.circle(180, 82, 22, model.surface_color(true));
    draw_icon(canvas, 180, 82, IconSize::List, text, Icon::Lock);
    draw_time_center(canvas, 180, 126, model.time, MobileTextRole::Display, text);
    canvas.text_center(180, 194, date.as_str(), MobileTextRole::Body, muted);

    let notification_offset = if model.boot_notification_visible {
        i32::from(model.boot_notification_offset_px) / SCALE
    } else {
        0
    };
    if model.boot_notification_visible && notification_offset != 0 {
        let underlay = blend(model.surface_color(true), model.accent(), 72);
        canvas.rounded_rect(
            DRect::screen_inset(MobileSpacingTokens::CONTENT_GUTTER, 326, 116),
            26,
            underlay,
        );
        canvas.text_center(
            if notification_offset > 0 { 58 } else { 302 },
            382,
            "Dismiss",
            MobileTextRole::Label,
            COLOR_WHITE,
        );
    }
    canvas.card(
        DRect::new(24 + notification_offset, 326, 312, 116),
        26,
        model.dark_theme,
        true,
    );
    if model.boot_notification_visible {
        if model.is_pressed(MobilePressedTarget::BootNotification) {
            canvas.rounded_rect(
                DRect::new(28 + notification_offset, 330, 304, 108),
                24,
                pressed_surface_color(model),
            );
        }
        canvas.circle(58 + notification_offset, 384, 22, model.accent());
        draw_icon(
            canvas,
            58 + notification_offset,
            384,
            IconSize::List,
            model.on_accent(),
            Icon::Info,
        );
        canvas.text_role(
            92 + notification_offset,
            338,
            "System UI",
            MobileTextRole::Label,
            weak,
        );
        canvas.text_role(
            92 + notification_offset,
            362,
            "Local preview",
            MobileTextRole::Body,
            text,
        );
        canvas.text_role(
            92 + notification_offset,
            397,
            "Unlock to review limits",
            MobileTextRole::Caption,
            muted,
        );
    } else {
        canvas.circle(58, 384, 22, model.surface_color(false));
        draw_icon(canvas, 58, 384, IconSize::List, muted, Icon::Bell);
        canvas.text_role(92, 350, "No notifications", MobileTextRole::Body, text);
        canvas.text_role(
            92,
            387,
            "You're all caught up",
            MobileTextRole::Caption,
            weak,
        );
    }

    draw_icon(canvas, 180, 680, IconSize::List, text, Icon::ChevronUp);
    canvas.text_center(180, 706, "Swipe up to continue", MobileTextRole::Body, text);
    canvas.text_center(180, 740, "Local preview", MobileTextRole::Label, weak);

    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

#[inline(never)]
fn render_overview(canvas: &mut Canvas<'_>, model: MobileModel, reveal_px: u16) {
    let reveal_px = reveal_px.min(OVERVIEW_RENDER_MAX_PX) / OVERVIEW_RENDER_QUANTUM_PX
        * OVERVIEW_RENDER_QUANTUM_PX;
    if reveal_px == 0 || model.system_ui_mode == UiSystemUiMode::Locked {
        return;
    }

    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let theme = model.theme_tokens();
    let neutral = theme.background_top;
    let panel = theme.panel;
    let sheet_top_px = SYSTEM_NAV_TOP_PX.saturating_sub(
        ((u32::from(SYSTEM_NAV_TOP_PX - 240) * u32::from(reveal_px))
            / u32::from(OVERVIEW_RENDER_MAX_PX)) as u16,
    );
    let content_offset_y_px = i32::from(sheet_top_px) - 240;
    let corner_radius_px =
        (64_u32 * u32::from(reveal_px) / u32::from(OVERVIEW_RENDER_MAX_PX)) as i32;
    let scrim_alpha = 255_u32 * u32::from(reveal_px) / u32::from(OVERVIEW_RENDER_MAX_PX);

    // At full reveal this opaque neutral layer makes the stable Overview
    // byte-identical whether the focus handoff originated in Launcher or App.
    // It contains no copied App pixels or synthetic thumbnail.
    canvas.blend_rect(DRect::new(0, 32, 360, 742), neutral, scrim_alpha);

    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.design_offset_y_px = previous_offset_y_px + content_offset_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(SYSTEM_NAV_TOP_PX));

    canvas.rounded_rect(
        DRect::new(0, 124, 360, 654),
        (corner_radius_px / SCALE).max(0),
        theme.shadow,
    );
    canvas.rounded_rect(
        DRect::new(0, 120, 360, 654),
        (corner_radius_px / SCALE).max(0),
        panel,
    );
    canvas.rounded_rect(DRect::new(150, 132, 60, 4), 2, muted);
    canvas.text_role(24, 148, "Overview", MobileTextRole::Title, text);
    canvas.text_role(24, 176, "This session", MobileTextRole::Label, weak);

    let overview_sheet_offset_y_px = canvas.design_offset_y_px;
    canvas.design_offset_y_px =
        overview_sheet_offset_y_px - i32::from(model.effective_overview_recent_offset_px());
    canvas.card(DRect::new(24, 212, 312, 380), 28, model.dark_theme, true);
    if model.is_pressed(MobilePressedTarget::OverviewRecent) {
        canvas.rounded_rect(
            DRect::new(28, 216, 304, 372),
            24,
            pressed_surface_color(model),
        );
    }

    if let Some(recent) = model.system_ui_recent {
        let (
            app_color,
            app_icon,
            use_installed_icon,
            app_name,
            recent_label,
            open_label,
            boundary_label,
        ) = match recent {
            UiRecentIdentity::Shell(app) => {
                let (color, icon, name) = match app {
                    ShellAppId::Phone => (COLOR_PHONE, Icon::Phone, "Phone"),
                    ShellAppId::Messages => (COLOR_MESSAGES, Icon::Messages, "Messages"),
                    ShellAppId::Settings => (COLOR_SETTINGS, Icon::Settings, "Settings"),
                };
                (
                    color,
                    icon,
                    false,
                    name,
                    "Recent app",
                    "Opened this session",
                    "No preview is stored",
                )
            }
            UiRecentIdentity::CompatibleAndroid(identity)
                if model.compatible_android_recent_ready()
                    && identity.package_generation() == model.android_installed_app.generation =>
            {
                (
                    COLOR_ANDROIDBOX,
                    Icon::DroidCode,
                    true,
                    fitted_mobile_text_for_scale(
                        model.android_installed_app.title.as_str(),
                        MobileTextRole::Title,
                        520,
                        model.large_text,
                    ),
                    "Recent compatible app",
                    "Open re-verifies the APK",
                    "No Activity pixels are stored",
                )
            }
            UiRecentIdentity::CompatibleAndroid(_) => (
                COLOR_SETTINGS,
                Icon::Info,
                false,
                "Recent app unavailable",
                "Package identity changed",
                "Open again from All apps",
                "No cached content is shown",
            ),
        };
        if use_installed_icon {
            // Reuse only the package-catalog icon already admitted for the
            // exact compatible session. This is launcher metadata, never an
            // Activity surface, screenshot, thumbnail, or background task.
            draw_installed_app_icon(canvas, 180, 308, model.android_installed_app);
        } else {
            canvas.rounded_rect(DRect::new(148, 276, 64, 64), 18, app_color);
            draw_icon(
                canvas,
                180,
                308,
                IconSize::App,
                accessible_surface_ink(app_color),
                app_icon,
            );
        }
        canvas.text_center(180, 366, app_name, MobileTextRole::Title, text);
        canvas.text_center(180, 400, recent_label, MobileTextRole::Caption, muted);
        if !matches!(
            recent,
            UiRecentIdentity::CompatibleAndroid(_)
                if !model.compatible_android_recent_ready()
        ) {
            canvas.rounded_rect(DRect::new(108, 448, 144, 48), 24, model.accent());
            canvas.text_center(180, 461, "Open", MobileTextRole::Body, model.on_accent());
        }
        canvas.text_center(180, 520, open_label, MobileTextRole::Caption, muted);
        canvas.text_center(180, 555, boundary_label, MobileTextRole::Label, weak);
        canvas.rounded_rect(DRect::new(162, 575, 36, 3), 2, weak);
    } else {
        canvas.circle(180, 312, 32, model.surface_color(false));
        draw_icon(canvas, 180, 312, IconSize::App, muted, Icon::Info);
        canvas.text_center(180, 366, "No recent app", MobileTextRole::Title, text);
        canvas.text_center(
            180,
            405,
            "Open Phone, Messages, or Settings",
            MobileTextRole::Caption,
            muted,
        );
        canvas.text_center(180, 435, "This session only", MobileTextRole::Label, weak);
    }
    canvas.design_offset_y_px = overview_sheet_offset_y_px;
    canvas.text_center(
        180,
        630,
        if model.system_ui_recent.is_some() {
            "Swipe card up to remove"
        } else {
            "No background tasks"
        },
        MobileTextRole::Caption,
        muted,
    );
    canvas.text_center(180, 655, "Swipe up for Home", MobileTextRole::Label, weak);

    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

fn render_app_drawer(canvas: &mut Canvas<'_>, model: MobileModel, reveal_px: u16) {
    if model.page != MobilePage::Home {
        return;
    }
    let text = model.text_primary();
    let muted = model.text_secondary();
    let theme = model.theme_tokens();
    let panel = theme.panel;
    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.design_offset_y_px =
        previous_offset_y_px + i32::from(DRAWER_REVEAL_MAX.saturating_sub(reveal_px));
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(SYSTEM_NAV_TOP_PX));

    canvas.rounded_rect(DRect::new(0, 68, 360, 760), 30, COLOR_SHADOW);
    canvas.rounded_rect(
        DRect::new(0, 64, 360, 756),
        MobileShapeTokens::RADIUS_PANEL,
        panel,
    );
    if model.is_pressed(MobilePressedTarget::DrawerHandle) {
        canvas.rounded_rect(
            DRect::new(120, 66, 120, 50),
            24,
            pressed_surface_color(model),
        );
    }
    canvas.rounded_rect(DRect::new(142, 78, 76, 4), 2, muted);
    canvas.text_role(24, 90, "All apps", MobileTextRole::Title, text);

    for (target, x, color, icon, label) in [
        (
            MobilePressedTarget::DrawerPhone,
            52,
            COLOR_PHONE,
            Icon::Phone,
            "Phone",
        ),
        (
            MobilePressedTarget::DrawerMessages,
            138,
            COLOR_MESSAGES,
            Icon::Messages,
            "Messages",
        ),
        (
            MobilePressedTarget::DrawerCalculator,
            224,
            COLOR_CALCULATOR,
            Icon::Calculator,
            "Calculator",
        ),
        (
            MobilePressedTarget::DrawerSettings,
            310,
            COLOR_SETTINGS,
            Icon::Settings,
            "Settings",
        ),
    ] {
        if model.is_pressed(target) {
            canvas.circle(x, 205, 31, pressed_surface_color(model));
        }
        draw_app_icon(canvas, x, 205, color, icon);
        canvas.text_center(x, 242, label, MobileTextRole::Label, muted);
    }

    let android_target = drawer_android_pressed_target(model);
    if model.is_pressed(android_target) {
        canvas.circle(52, 335, 31, pressed_surface_color(model));
    }
    if model.android_installed_app.installed {
        #[cfg(feature = "androidbox-multipackage4")]
        let installed = model.android_installed_apps[0];
        #[cfg(not(feature = "androidbox-multipackage4"))]
        let installed = model.android_installed_app;
        draw_installed_app_icon(canvas, 52, 335, installed);
        let (title_first, title_second) =
            split_installed_app_label(installed.title.as_str(), model.large_text);
        canvas.text_center(52, 369, title_first, MobileTextRole::Label, text);
        if !title_second.is_empty() {
            canvas.text_center(52, 390, title_second, MobileTextRole::Label, muted);
        }
    } else {
        draw_app_icon(canvas, 52, 335, COLOR_ANDROIDBOX, Icon::DroidCode);
        canvas.text_center(52, 369, "AndroidBox", MobileTextRole::Label, muted);
        canvas.text_center(
            52,
            390,
            "Demo",
            MobileTextRole::Label,
            model.text_tertiary(),
        );
    }

    #[cfg(feature = "androidbox-multipackage4")]
    if model.android_installed_app_count > 1 {
        let installed = model.android_installed_apps[1];
        let target = MobilePressedTarget::DrawerInstalledAndroid(2);
        if model.is_pressed(target) {
            canvas.circle(138, 335, 31, pressed_surface_color(model));
        }
        draw_installed_app_icon(canvas, 138, 335, installed);
        let (title_first, title_second) =
            split_installed_app_label(installed.title.as_str(), model.large_text);
        canvas.text_center(138, 369, title_first, MobileTextRole::Label, text);
        if !title_second.is_empty() {
            canvas.text_center(138, 390, title_second, MobileTextRole::Label, muted);
        }
    }

    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

fn fitted_mobile_text_for_scale(
    value: &str,
    role: MobileTextRole,
    maximum_width_px: u16,
    large_text: bool,
) -> &str {
    let role = accessible_text_role(role, large_text);
    let mut length = value.len();
    while length != 0
        && (!value.is_char_boundary(length)
            || measure_mobile_text_px(role, &value[..length]) > maximum_width_px)
    {
        length -= 1;
    }
    &value[..length]
}

fn ellipsized_mobile_text_prefix(
    value: &str,
    role: MobileTextRole,
    maximum_width_px: u16,
    large_text: bool,
) -> (&str, bool) {
    let role = accessible_text_role(role, large_text);
    if measure_mobile_text_px(role, value) <= maximum_width_px {
        return (value, false);
    }
    let ellipsis_width = measure_mobile_text_px(role, "...");
    (
        fitted_mobile_text_for_scale(
            value,
            role,
            maximum_width_px.saturating_sub(ellipsis_width),
            false,
        ),
        true,
    )
}

#[cfg(test)]
fn fitted_mobile_text(value: &str, role: MobileTextRole, maximum_width_px: u16) -> &str {
    fitted_mobile_text_for_scale(value, role, maximum_width_px, false)
}

fn split_installed_app_label(value: &str, large_text: bool) -> (&str, &str) {
    let Some(first_space) = value.as_bytes().iter().position(|byte| *byte == b' ') else {
        return (
            fitted_mobile_text_for_scale(value, MobileTextRole::Label, 160, large_text),
            "",
        );
    };
    if first_space == 0 {
        return (
            fitted_mobile_text_for_scale(value, MobileTextRole::Label, 160, large_text),
            "",
        );
    }
    let mut second_start = first_space + 1;
    while second_start < value.len() && value.as_bytes()[second_start] == b' ' {
        second_start += 1;
    }
    (
        fitted_mobile_text_for_scale(
            &value[..first_space],
            MobileTextRole::Label,
            160,
            large_text,
        ),
        fitted_mobile_text_for_scale(
            &value[second_start..],
            MobileTextRole::Label,
            160,
            large_text,
        ),
    )
}

fn render_app_header(canvas: &mut Canvas<'_>, model: &MobileModel, title: &str) {
    let text = model.text_primary();
    if model.is_pressed(MobilePressedTarget::Back) {
        canvas.circle(
            28,
            60,
            22,
            blend(
                model.surface_color(true),
                model.accent(),
                if model.dark_theme { 112 } else { 58 },
            ),
        );
    }
    draw_icon(canvas, 28, 60, IconSize::List, text, Icon::ChevronLeft);
    canvas.text_role(52, 48, title, MobileTextRole::Title, text);
}

#[inline(never)]
fn render_phone(canvas: &mut Canvas<'_>, model: MobileModel) {
    render_app_header(canvas, &model, "Phone");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();

    if model.phone_digit_count == 0 {
        canvas.text_center(180, 143, "Enter a number", MobileTextRole::Body, muted);
    } else {
        canvas.text_center(180, 138, model.phone_number(), MobileTextRole::Title, text);
    }
    canvas.fill_rect(DRect::new(32, 187, 296, 1), model.outline_color());

    let digits = [
        ("1", ""),
        ("2", "ABC"),
        ("3", "DEF"),
        ("4", "GHI"),
        ("5", "JKL"),
        ("6", "MNO"),
        ("7", "PQRS"),
        ("8", "TUV"),
        ("9", "WXYZ"),
        ("*", ""),
        ("0", "+"),
        ("#", ""),
    ];
    for (index, (digit, letters)) in digits.into_iter().enumerate() {
        let column = index % 3;
        let row = index / 3;
        let x = 82 + column as i32 * 98;
        let y = 254 + row as i32 * 104;
        let key_target = MobilePressedTarget::PhoneKey(index as u8);
        let surface = if model.is_pressed(key_target) {
            pressed_surface_color(model)
        } else {
            model.surface_color(true)
        };
        canvas.circle(x, y, 35, surface);
        match index {
            9 => {
                // The large Headline atlas intentionally omits punctuation.
                // Draw the dialer asterisk on the same optical grid instead
                // of showing the font's question-mark fallback.
                canvas.round_line(x, y - 13, x, y + 13, 2, text);
                canvas.round_line(x - 11, y - 7, x + 11, y + 7, 2, text);
                canvas.round_line(x - 11, y + 7, x + 11, y - 7, 2, text);
            }
            11 => {
                // Match the asterisk's stroke weight and key-center bounds.
                canvas.round_line(x - 7, y - 14, x - 9, y + 14, 2, text);
                canvas.round_line(x + 9, y - 14, x + 7, y + 14, 2, text);
                canvas.round_line(x - 15, y - 6, x + 15, y - 6, 2, text);
                canvas.round_line(x - 15, y + 6, x + 15, y + 6, 2, text);
            }
            _ => canvas.text_center(x, y - 18, digit, MobileTextRole::Headline, text),
        }
        if !letters.is_empty() {
            canvas.text_center(x, y + 15, letters, MobileTextRole::Label, muted);
        }
    }
    canvas.circle(180, 688, 34, model.surface_color(false));
    draw_icon(canvas, 180, 688, IconSize::App, weak, Icon::Phone);
    canvas.circle(
        258,
        688,
        28,
        if model.is_pressed(MobilePressedTarget::PhoneBackspace) {
            pressed_surface_color(model)
        } else {
            model.surface_color(false)
        },
    );
    draw_icon(canvas, 258, 688, IconSize::List, muted, Icon::Backspace);
    canvas.text_center(
        180,
        735,
        "Calling unavailable in preview",
        MobileTextRole::Caption,
        weak,
    );
}

fn calculator_value_text(model: MobileModel, buffer: &mut [u8; 32]) -> &str {
    if model.calculator_error {
        buffer[..5].copy_from_slice(b"Error");
        return core::str::from_utf8(&buffer[..5]).expect("calculator error text is ASCII");
    }
    fixed_calculator_text(
        model.calculator_value,
        model.calculator_entering && model.calculator_decimal_entered,
        model.calculator_fraction_digits,
        model.calculator_negative_zero,
        buffer,
    )
}

fn fixed_calculator_text(
    value: i64,
    preserve_decimal: bool,
    entered_fraction_digits: u8,
    negative_zero: bool,
    buffer: &mut [u8; 32],
) -> &str {
    let negative = value < 0 || value == 0 && negative_zero;
    let absolute = value.unsigned_abs();
    let scale = CALCULATOR_SCALE as u64;
    let integer = absolute / scale;
    let fraction = absolute % scale;
    let mut length = 0;
    if negative {
        buffer[length] = b'-';
        length += 1;
    }

    let mut reverse = [0u8; 20];
    let mut reverse_length = 0;
    let mut remaining = integer;
    loop {
        reverse[reverse_length] = b'0' + u8::try_from(remaining % 10).unwrap_or(0);
        reverse_length += 1;
        remaining /= 10;
        if remaining == 0 {
            break;
        }
    }
    for index in (0..reverse_length).rev() {
        buffer[length] = reverse[index];
        length += 1;
    }

    let fraction_digits = if preserve_decimal {
        entered_fraction_digits.min(CALCULATOR_FRACTION_DIGITS)
    } else {
        let mut digits = CALCULATOR_FRACTION_DIGITS;
        let mut trimmed = fraction;
        while digits != 0 && trimmed.is_multiple_of(10) {
            trimmed /= 10;
            digits -= 1;
        }
        digits
    };
    if preserve_decimal || fraction_digits != 0 {
        buffer[length] = b'.';
        length += 1;
        let mut divisor = CALCULATOR_SCALE as u64 / 10;
        for _ in 0..fraction_digits {
            buffer[length] = b'0' + u8::try_from((fraction / divisor) % 10).unwrap_or(0);
            length += 1;
            divisor /= 10;
        }
    }
    core::str::from_utf8(&buffer[..length]).expect("calculator formatter emits only ASCII")
}

const fn calculator_operation_text(operation: CalculatorOperation) -> &'static str {
    match operation {
        CalculatorOperation::None => "",
        CalculatorOperation::Add => "+",
        CalculatorOperation::Subtract => "-",
        CalculatorOperation::Multiply => "x",
        CalculatorOperation::Divide => "/",
    }
}

#[inline(never)]
fn render_calculator(canvas: &mut Canvas<'_>, model: MobileModel) {
    render_app_header(canvas, &model, "Calculator");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let mut display_buffer = [0u8; 32];
    let display = calculator_value_text(model, &mut display_buffer);

    canvas.rounded_rect(DRect::new(16, 96, 328, 132), 24, model.surface_color(true));
    if model.calculator_pending != CalculatorOperation::None && !model.calculator_error {
        let mut accumulator_buffer = [0u8; 32];
        let accumulator = fixed_calculator_text(
            model.calculator_accumulator,
            false,
            0,
            false,
            &mut accumulator_buffer,
        );
        canvas.text_end(294, 112, accumulator, MobileTextRole::Caption, muted);
        canvas.text_center(
            316,
            112,
            calculator_operation_text(model.calculator_pending),
            MobileTextRole::Caption,
            muted,
        );
    }
    let display_role = if model.calculator_error {
        MobileTextRole::Title
    } else {
        match display.len() {
            0..=6 => MobileTextRole::Display,
            7..=10 => MobileTextRole::Headline,
            11..=15 => MobileTextRole::Title,
            _ => MobileTextRole::Body,
        }
    };
    canvas.text_end(326, 137, display, display_role, text);
    if model.calculator_error {
        canvas.text_end(
            326,
            184,
            "Clear to continue",
            MobileTextRole::Caption,
            COLOR_AMBER,
        );
    }
    let backspace_pressed = model.is_pressed(MobilePressedTarget::CalculatorKey(19));
    canvas.elevated_surface(
        DRect::new(260, 190, 76, 38),
        18,
        if backspace_pressed {
            pressed_surface_color(model)
        } else {
            model.surface_color(false)
        },
        model.dark_theme,
        backspace_pressed,
    );
    draw_icon(
        canvas,
        300,
        209,
        IconSize::List,
        if model.calculator_error { weak } else { muted },
        Icon::Backspace,
    );

    for index in 0u8..19 {
        let (rect, center_x, center_y) = if index < 16 {
            let column = i32::from(index % 4);
            let row = i32::from(index / 4);
            (
                DRect::new(21 + column * 81, 244 + row * 100, 66, 76),
                54 + column * 81,
                282 + row * 100,
            )
        } else if index == 16 {
            (DRect::new(21, 644, 147, 76), 95, 682)
        } else {
            let column = i32::from(index - 15);
            (
                DRect::new(21 + column * 81, 644, 66, 76),
                54 + column * 81,
                682,
            )
        };
        let operator = matches!(index, 3 | 7 | 11 | 15 | 18);
        let utility = index < 3;
        let surface = if model.is_pressed(MobilePressedTarget::CalculatorKey(index)) {
            pressed_surface_color(model)
        } else if operator {
            model.accent()
        } else if utility {
            model.surface_color(false)
        } else {
            model.surface_color(true)
        };
        canvas.elevated_surface(
            rect,
            MobileShapeTokens::RADIUS_LARGE,
            surface,
            model.dark_theme,
            model.is_pressed(MobilePressedTarget::CalculatorKey(index)),
        );
        let foreground = if operator { model.on_accent() } else { text };
        match index {
            0 => canvas.text_center(
                center_x,
                center_y - 12,
                "C",
                MobileTextRole::Title,
                foreground,
            ),
            1 => canvas.text_center(
                center_x,
                center_y - 10,
                "+/-",
                MobileTextRole::Body,
                foreground,
            ),
            2 => canvas.text_center(
                center_x,
                center_y - 10,
                "%",
                MobileTextRole::Body,
                foreground,
            ),
            3 => canvas.text_center(
                center_x,
                center_y - 12,
                "/",
                MobileTextRole::Title,
                foreground,
            ),
            4 => canvas.text_center(
                center_x,
                center_y - 14,
                "7",
                MobileTextRole::Headline,
                foreground,
            ),
            5 => canvas.text_center(
                center_x,
                center_y - 14,
                "8",
                MobileTextRole::Headline,
                foreground,
            ),
            6 => canvas.text_center(
                center_x,
                center_y - 14,
                "9",
                MobileTextRole::Headline,
                foreground,
            ),
            7 => canvas.text_center(
                center_x,
                center_y - 12,
                "x",
                MobileTextRole::Title,
                foreground,
            ),
            8 => canvas.text_center(
                center_x,
                center_y - 14,
                "4",
                MobileTextRole::Headline,
                foreground,
            ),
            9 => canvas.text_center(
                center_x,
                center_y - 14,
                "5",
                MobileTextRole::Headline,
                foreground,
            ),
            10 => canvas.text_center(
                center_x,
                center_y - 14,
                "6",
                MobileTextRole::Headline,
                foreground,
            ),
            11 => canvas.text_center(
                center_x,
                center_y - 12,
                "-",
                MobileTextRole::Title,
                foreground,
            ),
            12 => canvas.text_center(
                center_x,
                center_y - 14,
                "1",
                MobileTextRole::Headline,
                foreground,
            ),
            13 => canvas.text_center(
                center_x,
                center_y - 14,
                "2",
                MobileTextRole::Headline,
                foreground,
            ),
            14 => canvas.text_center(
                center_x,
                center_y - 14,
                "3",
                MobileTextRole::Headline,
                foreground,
            ),
            15 => canvas.text_center(
                center_x,
                center_y - 12,
                "+",
                MobileTextRole::Title,
                foreground,
            ),
            16 => canvas.text_center(
                center_x,
                center_y - 14,
                "0",
                MobileTextRole::Headline,
                foreground,
            ),
            17 => canvas.text_center(
                center_x,
                center_y - 13,
                ".",
                MobileTextRole::Headline,
                foreground,
            ),
            18 => canvas.text_center(
                center_x,
                center_y - 12,
                "=",
                MobileTextRole::Title,
                foreground,
            ),
            _ => {}
        }
    }
}

const fn androidbox_error_text(error: AndroidBoxError) -> &'static str {
    match error {
        AndroidBoxError::None => "No runtime error",
        AndroidBoxError::InvalidDex => "Invalid DEX",
        AndroidBoxError::VerificationFailed => "Verification failed",
        AndroidBoxError::UnsupportedOpcode => "Unsupported opcode",
        AndroidBoxError::StepLimit => "Step limit reached",
        AndroidBoxError::RuntimeTrap => "Runtime trap",
    }
}

const fn androidbox_activity_error_text(error: AndroidBoxActivityError) -> &'static str {
    match error {
        AndroidBoxActivityError::None => "No Activity error",
        AndroidBoxActivityError::ManifestRejected => "Manifest rejected",
        AndroidBoxActivityError::LauncherActivityMissing => "Launcher activity missing",
        AndroidBoxActivityError::ActivityVerificationFailed => "Activity verification failed",
        AndroidBoxActivityError::UnsupportedFrameworkCall => "Unsupported framework call",
        AndroidBoxActivityError::StepLimit => "Activity step limit",
        AndroidBoxActivityError::RuntimeTrap => "Activity runtime trap",
    }
}

fn androidbox_launcher_manifest_verified(model: &MobileModel) -> bool {
    model.androidbox_manifest_verified
        && model.androidbox_launcher_activity.as_str() == "org.bndroid.demo.MainActivity"
}

const ANDROIDBOX_RESOURCE_LAYOUT_ID: u32 = 0x7f02_0000;
const ANDROIDBOX_RESOURCE_STRING_ID: u32 = 0x7f03_0000;

fn androidbox_resources_complete(resources: AndroidBoxResourceStatus) -> bool {
    resources.resource_table_parsed
        && resources.layout_entry_resolved
        && resources.binary_xml_parsed
        && resources.text_view_verified
        && resources.string_reference_resolved
        && resources.layout_resource_id == ANDROIDBOX_RESOURCE_LAYOUT_ID
        && resources.string_resource_id == ANDROIDBOX_RESOURCE_STRING_ID
}

fn androidbox_on_create_complete(model: &MobileModel) -> bool {
    model.androidbox_on_create_completed
        && model.androidbox_activity_error == AndroidBoxActivityError::None
}

fn androidbox_activity_complete(model: &MobileModel) -> bool {
    model.androidbox_verified
        && model.androidbox_error == AndroidBoxError::None
        && androidbox_launcher_manifest_verified(model)
        && model.androidbox_on_create_completed
        && model.androidbox_text_view_content.as_str() == "AndroidBox resource-backed view"
        && model.androidbox_activity_instruction_count > 0
        && androidbox_resources_complete(model.androidbox_resources)
        && model.androidbox_activity_error == AndroidBoxActivityError::None
}

const fn androidbox_fact_text(verified: bool, success: &'static str) -> &'static str {
    if verified { success } else { "Pending" }
}

const fn androidbox_resource_id_status(resolved: bool, actual: u32, expected: u32) -> &'static str {
    if !resolved {
        "Pending"
    } else if actual == expected {
        "Resolved"
    } else {
        "Mismatch"
    }
}

const fn androidbox_consistent_fact_text(
    reported: bool,
    consistent: bool,
    success: &'static str,
) -> &'static str {
    if !reported {
        "Pending"
    } else if consistent {
        success
    } else {
        "Mismatch"
    }
}

fn installed_android_launch_copy(
    status: &AndroidInstalledLaunchStatus,
) -> (&'static str, &'static str) {
    match status {
        AndroidInstalledLaunchStatus::Idle => {
            ("Launch not requested", "Choose this app again in All apps")
        }
        AndroidInstalledLaunchStatus::Pending { .. } => (
            "APK verification pending",
            "No Activity content is shown yet",
        ),
        AndroidInstalledLaunchStatus::Succeeded { .. } => {
            ("Launch proof is stale", "Choose this app again in All apps")
        }
        AndroidInstalledLaunchStatus::Failed { failure, .. } => match failure {
            AndroidInstalledLaunchFailure::Stale => (
                "Launch request is stale",
                "Choose this app again in All apps",
            ),
            AndroidInstalledLaunchFailure::Verification => {
                ("APK verification failed", "No Activity content is shown")
            }
            AndroidInstalledLaunchFailure::Unsupported => {
                ("App profile unsupported", "No Activity content is shown")
            }
            AndroidInstalledLaunchFailure::Unavailable => (
                "Installed APK unavailable",
                "Choose this app again in All apps",
            ),
        },
    }
}

/// Renders only the bounded launch result published after the package runtime
/// read the committed APK back from disk. This path never executes the legacy
/// embedded demo and exposes no DEX control surface.
fn render_installed_android_legacy_content(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    installed: &AndroidInstalledAppStatus,
    text: u32,
) {
    canvas.elevated_surface(
        DRect::new(34, 224, 292, 112),
        MobileShapeTokens::RADIUS_CONTROL,
        model.surface_color(true),
        model.dark_theme,
        false,
    );
    canvas.text_center_fitted(
        180,
        267,
        installed.text.as_str(),
        MobileTextRole::Body,
        520,
        text,
    );
}

#[cfg(feature = "androidbox-interactive0")]
fn render_installed_android_interactive_content(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
) {
    let view = model.android_installed_activity_view;
    canvas.elevated_surface(
        DRect::new(34, 224, 292, 88),
        MobileShapeTokens::RADIUS_CONTROL,
        model.surface_color(true),
        model.dark_theme,
        false,
    );
    canvas.text_center_fitted(180, 255, view.label_text(), MobileTextRole::Body, 520, text);

    let button_target = MobilePressedTarget::InstalledAndroidButton(view.button_view_id());
    let button_background = if model.is_pressed(button_target) {
        blend(model.accent(), COLOR_BLACK, 82)
    } else {
        model.accent()
    };
    canvas.elevated_surface(
        DRect::new(34, 332, 292, 64),
        MobileShapeTokens::RADIUS_LARGE,
        button_background,
        model.dark_theme,
        model.is_pressed(button_target),
    );
    canvas.text_center_fitted(
        180,
        351,
        view.button_text(),
        MobileTextRole::Body,
        500,
        model.on_accent(),
    );
}

#[cfg(feature = "androidbox-scene-rpc2")]
fn render_installed_android_scene_content(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    let scene = model.android_installed_activity_scene;

    for (index, node) in scene.nodes().iter().copied().enumerate() {
        // LinearLayout contributes hierarchy and indentation to leaf geometry,
        // but is deliberately not presented as an invented visible surface.
        let Some(rect) = installed_android_scene_node_rect(scene, index) else {
            continue;
        };
        let radius = (rect.height / 3).clamp(10, 20);
        #[cfg(feature = "androidbox-layout-row14")]
        let text_role = installed_android_scene_node_text_role(scene, index);
        #[cfg(not(feature = "androidbox-layout-row14"))]
        let text_role = MobileTextRole::Body;
        let text_y = rect.y
            + (rect.height - (i32::from(text_role.line_height_px()) + SCALE - 1) / SCALE).max(0)
                / 2;
        let maximum_text_width_px =
            u16::try_from((rect.width - 20).max(1) * SCALE).unwrap_or(u16::MAX);

        match node.kind() {
            AndroidSceneViewKind::LinearLayout => {}
            AndroidSceneViewKind::TextView => {
                canvas.elevated_surface(
                    rect,
                    radius,
                    model.surface_color(true),
                    model.dark_theme,
                    false,
                );
                canvas.text_center_fitted(
                    rect.x + rect.width / 2,
                    text_y,
                    node.text(),
                    text_role,
                    maximum_text_width_px,
                    text,
                );
            }
            AndroidSceneViewKind::Button => {
                let target = MobilePressedTarget::InstalledAndroidButton(node.id());
                let callback_enabled = node.callback_registered();
                let background = if callback_enabled && model.is_pressed(target) {
                    blend(model.accent(), COLOR_BLACK, 82)
                } else if callback_enabled {
                    model.accent()
                } else {
                    blend(
                        model.surface_color(true),
                        muted,
                        if model.dark_theme { 54 } else { 30 },
                    )
                };
                canvas.elevated_surface(
                    rect,
                    radius,
                    background,
                    model.dark_theme,
                    callback_enabled && model.is_pressed(target),
                );
                canvas.text_center_fitted(
                    rect.x + rect.width / 2,
                    text_y,
                    node.text(),
                    text_role,
                    maximum_text_width_px,
                    if callback_enabled {
                        model.on_accent()
                    } else {
                        muted
                    },
                );
            }
        }
    }
}

#[inline(never)]
fn render_installed_android_app(canvas: &mut Canvas<'_>, model: &MobileModel) {
    let installed = &model.android_installed_app;
    let content_ready = model.installed_android_foreground_content_ready();
    let text = model.text_primary();
    let muted = model.text_secondary();
    render_app_header(
        canvas,
        model,
        fitted_mobile_text_for_scale(
            installed.title.as_str(),
            MobileTextRole::Title,
            420,
            model.large_text,
        ),
    );
    draw_installed_app_icon(canvas, 320, 60, *installed);

    // A foreground Activity presents only its publisher-owned content beneath
    // the normal app bar. Package identity, verification evidence, RPC
    // revisions and compatibility limits remain available in Settings/Apps;
    // they are not repeated as developer chrome over the application.
    if content_ready {
        #[cfg(feature = "androidbox-scene-rpc2")]
        if model.android_installed_activity_scene.is_active() {
            render_installed_android_scene_content(canvas, model, text, muted);
        } else if model.android_installed_activity_view.is_active() {
            render_installed_android_interactive_content(canvas, model, text);
        } else {
            render_installed_android_legacy_content(canvas, model, installed, text);
        }
        #[cfg(all(
            feature = "androidbox-interactive0",
            not(feature = "androidbox-scene-rpc2")
        ))]
        if model.android_installed_activity_view.is_active() {
            render_installed_android_interactive_content(canvas, model, text);
        } else {
            render_installed_android_legacy_content(canvas, model, installed, text);
        }
        #[cfg(not(feature = "androidbox-interactive0"))]
        render_installed_android_legacy_content(canvas, model, installed, text);
    } else {
        let (status, detail) = installed_android_launch_copy(&model.android_installed_launch);
        canvas.card(DRect::new(24, 218, 312, 174), 24, model.dark_theme, true);
        draw_setting_icon(canvas, 180, 264, COLOR_AMBER, Icon::Info);
        canvas.text_center(180, 310, status, MobileTextRole::Caption, text);
        canvas.text_center(180, 346, detail, MobileTextRole::Label, muted);
    }
}

#[inline(never)]
fn render_android_demo(canvas: &mut Canvas<'_>, model: &MobileModel) {
    if model.android_installed_app.installed {
        render_installed_android_app(canvas, model);
        return;
    }
    render_app_header(canvas, model, "AndroidBox Demo");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let dex_failed = model.androidbox_error != AndroidBoxError::None;
    let activity_failed = model.androidbox_activity_error != AndroidBoxActivityError::None;
    let manifest_verified = androidbox_launcher_manifest_verified(model);
    let activity_complete = androidbox_activity_complete(model);
    let on_create_complete = androidbox_on_create_complete(model);

    canvas.card(DRect::new(18, 88, 324, 76), 24, model.dark_theme, true);
    draw_app_icon(canvas, 52, 126, COLOR_ANDROIDBOX, Icon::DroidCode);
    canvas.text_role(
        86,
        98,
        "Resource-backed Activity",
        MobileTextRole::Title,
        text,
    );
    canvas.text_role(
        86,
        128,
        "AndroidBox Activity-1",
        MobileTextRole::Caption,
        muted,
    );
    canvas.text_role(86, 146, "Not Android ART", MobileTextRole::Label, weak);

    canvas.text_role(
        24,
        172,
        "Compatibility boundary",
        MobileTextRole::Label,
        muted,
    );
    canvas.card(DRect::new(18, 188, 324, 54), 20, model.dark_theme, false);
    draw_setting_icon(canvas, 50, 215, COLOR_AMBER, Icon::Info);
    canvas.text_role(
        78,
        196,
        "No install / Binder / JNI",
        MobileTextRole::Body,
        text,
    );
    canvas.text_role(
        78,
        222,
        "No general Android compatibility",
        MobileTextRole::Caption,
        weak,
    );

    canvas.text_role(24, 248, "Activity lifecycle", MobileTextRole::Label, muted);
    canvas.card(DRect::new(18, 266, 324, 136), 22, model.dark_theme, false);
    let activity_color = if activity_failed {
        COLOR_AMBER
    } else if activity_complete {
        COLOR_ANDROIDBOX
    } else {
        COLOR_SETTINGS
    };
    draw_setting_icon(canvas, 52, 298, activity_color, Icon::DroidCode);
    canvas.text_role(
        82,
        276,
        if activity_failed {
            "Activity launch failed"
        } else if activity_complete {
            "MainActivity"
        } else if manifest_verified {
            "Launcher activity verified"
        } else {
            "Waiting for Activity runtime"
        },
        MobileTextRole::Body,
        text,
    );
    canvas.text_role(
        82,
        306,
        if activity_failed {
            androidbox_activity_error_text(model.androidbox_activity_error)
        } else if model.androidbox_launcher_activity.is_empty() {
            "Launcher identity not reported"
        } else {
            model.androidbox_launcher_activity.as_str()
        },
        MobileTextRole::Caption,
        if activity_failed { COLOR_AMBER } else { muted },
    );
    canvas.fill_rect(DRect::new(36, 328, 288, 1), model.outline_color());
    canvas.text_role(
        36,
        338,
        if manifest_verified {
            "Manifest verified"
        } else {
            "Manifest pending"
        },
        MobileTextRole::Label,
        if manifest_verified {
            activity_color
        } else {
            muted
        },
    );
    canvas.text_end(
        324,
        338,
        if on_create_complete {
            "onCreate complete"
        } else {
            "onCreate pending"
        },
        MobileTextRole::Label,
        if on_create_complete {
            activity_color
        } else {
            muted
        },
    );
    canvas.text_role(
        36,
        357,
        "Activity instructions",
        MobileTextRole::Label,
        muted,
    );
    let mut activity_instructions = MobileAsciiText::<10>::new();
    activity_instructions.push_u32(model.androidbox_activity_instruction_count);
    canvas.text_end(
        316,
        357,
        activity_instructions.as_str(),
        MobileTextRole::Label,
        text,
    );
    canvas.rounded_rect(
        DRect::new(34, 378, 292, 20),
        10,
        if model.dark_theme {
            COLOR_CARD_RAISED_DARK
        } else {
            COLOR_CARD_RAISED_LIGHT
        },
    );
    let text_view = if activity_complete {
        model.androidbox_text_view_content.as_str()
    } else if activity_failed {
        "No content installed"
    } else {
        "Waiting for resource view"
    };
    canvas.text_role(
        46,
        381,
        text_view,
        MobileTextRole::Caption,
        if activity_complete { text } else { weak },
    );

    let resources = model.androidbox_resources;
    let resources_complete = androidbox_resources_complete(resources);
    let view_text_matches =
        model.androidbox_text_view_content.as_str() == "AndroidBox resource-backed view";
    let resources_display_complete = resources_complete && view_text_matches;
    let resource_color = if resources_display_complete {
        COLOR_ANDROIDBOX
    } else if resources != AndroidBoxResourceStatus::empty() {
        COLOR_AMBER
    } else {
        COLOR_SETTINGS
    };
    let resource_fact_color = |verified: bool| if verified { resource_color } else { muted };
    let mut layout_label = MobileAsciiText::<17>::new();
    layout_label.push_bytes(b"layout ");
    layout_label.push_hex_u32(resources.layout_resource_id);
    let mut string_label = MobileAsciiText::<18>::new();
    string_label.push_bytes(b"@string ");
    string_label.push_hex_u32(resources.string_resource_id);

    canvas.text_role(24, 408, "Compiled resources", MobileTextRole::Label, muted);
    canvas.card(DRect::new(18, 426, 324, 126), 22, model.dark_theme, false);
    for (label, status, y, verified) in [
        (
            "resources.arsc",
            androidbox_fact_text(resources.resource_table_parsed, "Parsed"),
            435,
            resources.resource_table_parsed,
        ),
        (
            layout_label.as_str(),
            androidbox_resource_id_status(
                resources.layout_entry_resolved,
                resources.layout_resource_id,
                ANDROIDBOX_RESOURCE_LAYOUT_ID,
            ),
            457,
            resources.layout_entry_resolved
                && resources.layout_resource_id == ANDROIDBOX_RESOURCE_LAYOUT_ID,
        ),
        (
            "Binary XML",
            androidbox_fact_text(resources.binary_xml_parsed, "Parsed"),
            479,
            resources.binary_xml_parsed,
        ),
        (
            "TextView",
            androidbox_consistent_fact_text(
                resources.text_view_verified,
                view_text_matches,
                "Verified",
            ),
            501,
            resources.text_view_verified && view_text_matches,
        ),
        (
            string_label.as_str(),
            androidbox_consistent_fact_text(
                resources.string_reference_resolved,
                resources.string_resource_id == ANDROIDBOX_RESOURCE_STRING_ID && view_text_matches,
                "Resolved",
            ),
            523,
            resources.string_reference_resolved
                && resources.string_resource_id == ANDROIDBOX_RESOURCE_STRING_ID
                && view_text_matches,
        ),
    ] {
        canvas.circle(27, y + 5, 3, resource_fact_color(verified));
        canvas.text_role(36, y, label, MobileTextRole::Label, text);
        canvas.text_end(
            316,
            y,
            status,
            MobileTextRole::Label,
            resource_fact_color(verified),
        );
    }

    let mut result_value = MobileAsciiText::<12>::new();
    result_value.push_i32(model.androidbox_boot_value);
    let mut step_count = MobileAsciiText::<10>::new();
    step_count.push_u32(model.androidbox_step_count);
    let mut tap_count = MobileAsciiText::<10>::new();
    tap_count.push_u32(model.androidbox_tap_count);
    canvas.text_role(24, 556, "Restricted DEX-0", MobileTextRole::Label, muted);
    canvas.card(DRect::new(18, 574, 324, 36), 18, model.dark_theme, false);
    canvas.text_role(
        34,
        578,
        if dex_failed {
            androidbox_error_text(model.androidbox_error)
        } else if model.androidbox_verified {
            "Verified DEX execution"
        } else {
            "DEX execution pending"
        },
        MobileTextRole::Caption,
        if dex_failed { COLOR_AMBER } else { text },
    );
    for (label, value, x, value_x) in [
        ("Result", result_value.as_str(), 34, 160),
        ("Steps", step_count.as_str(), 174, 234),
        ("Taps", tap_count.as_str(), 248, 326),
    ] {
        canvas.text_role(x, 596, label, MobileTextRole::Label, muted);
        canvas.text_end(value_x, 596, value, MobileTextRole::Label, text);
    }

    canvas.rounded_rect(
        DRect::new(24, 620, 312, 72),
        24,
        if model.is_pressed(MobilePressedTarget::AndroidBoxExecute) {
            pressed_surface_color_ref(model)
        } else {
            COLOR_ANDROIDBOX
        },
    );
    canvas.text_center(
        180,
        642,
        if model.androidbox_verified && !dex_failed {
            "Run onTap"
        } else {
            "Execute DEX-0"
        },
        MobileTextRole::Body,
        COLOR_WHITE,
    );
    canvas.text_center(
        180,
        710,
        "The runtime owns every result",
        MobileTextRole::Caption,
        weak,
    );
    canvas.text_center(
        180,
        736,
        "Restricted shim / no general compatibility",
        MobileTextRole::Label,
        weak,
    );
}

#[inline(never)]
fn render_messages(canvas: &mut Canvas<'_>, model: MobileModel) {
    let title = match model.message_view {
        MessageView::Inbox => "Messages",
        MessageView::PreviewGuide => "Using Messages",
        MessageView::OfflineStatus => "Connection status",
    };
    render_app_header(canvas, &model, title);
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();

    match model.message_view {
        MessageView::Inbox => {
            // Keep the conversation surface genuinely empty. Preview help is
            // separated into a settings-style information group below it, so
            // local documentation cannot be mistaken for real messages.
            canvas.circle(180, 157, 32, model.surface_color(true));
            draw_icon(canvas, 180, 157, IconSize::App, muted, Icon::Messages);
            canvas.text_center(180, 207, "No conversations", MobileTextRole::Body, text);
            canvas.text_center(
                180,
                241,
                "Messaging is unavailable in this preview",
                MobileTextRole::Caption,
                muted,
            );

            canvas.text_role(24, 310, "Preview information", MobileTextRole::Label, muted);
            canvas.card(DRect::new(18, 334, 324, 152), 22, model.dark_theme, false);
            for (target, y, color, icon, name, preview) in [
                (
                    MobilePressedTarget::MessageGuide,
                    334,
                    model.accent(),
                    Icon::Info,
                    "Using Messages",
                    "Local navigation guide",
                ),
                (
                    MobilePressedTarget::MessageOffline,
                    410,
                    COLOR_AMBER,
                    Icon::Airplane,
                    "Connection status",
                    "Calls and messages unavailable",
                ),
            ] {
                if model.is_pressed(target) {
                    canvas.rounded_rect(
                        DRect::new(20, y + 2, 320, 72),
                        20,
                        pressed_surface_color(model),
                    );
                }
                draw_setting_icon(canvas, 52, y + 38, color, icon);
                canvas.text_role(82, y + 15, name, MobileTextRole::Body, text);
                canvas.text_role(82, y + 45, preview, MobileTextRole::Caption, muted);
                draw_icon(
                    canvas,
                    318,
                    y + 38,
                    IconSize::Status,
                    weak,
                    Icon::ChevronRight,
                );
            }
            canvas.fill_rect(DRect::new(78, 409, 248, 1), model.outline_color());
        }
        MessageView::PreviewGuide => {
            canvas.text_role(24, 105, "Local preview", MobileTextRole::Label, muted);
            canvas.card(DRect::new(18, 150, 304, 132), 20, model.dark_theme, false);
            canvas.text_role(
                36,
                172,
                "Local, read-only preview",
                MobileTextRole::Body,
                text,
            );
            canvas.text_role(
                36,
                207,
                "Swipe from the left edge",
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_role(
                36,
                235,
                "or use Home to navigate.",
                MobileTextRole::Caption,
                muted,
            );
            canvas.rounded_rect(DRect::new(78, 314, 264, 108), 20, model.accent());
            canvas.text_role(
                96,
                337,
                "No messages are sent",
                MobileTextRole::Body,
                model.on_accent(),
            );
            canvas.text_role(
                96,
                372,
                "or received.",
                MobileTextRole::Caption,
                model.on_accent(),
            );
        }
        MessageView::OfflineStatus => {
            canvas.text_role(24, 105, "Local preview", MobileTextRole::Label, muted);
            canvas.card(DRect::new(18, 150, 324, 154), 20, model.dark_theme, false);
            draw_setting_icon(canvas, 58, 194, COLOR_AMBER, Icon::Airplane);
            canvas.text_role(90, 170, "Network is disabled", MobileTextRole::Body, text);
            canvas.text_role(
                90,
                207,
                "No network service is connected.",
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_role(
                90,
                239,
                "Calls and messages cannot be sent.",
                MobileTextRole::Caption,
                muted,
            );
        }
    }
}

#[inline(never)]
fn render_settings_identity_card(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 94, 72),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        true,
    );
    canvas.circle(54, 130, 24, model.accent());
    canvas.text_center(54, 119, "BN", MobileTextRole::Label, model.on_accent());
    canvas.text_role(91, 108, "Bndroid OS", MobileTextRole::Title, text);
    canvas.text_role(91, 139, "Local preview", MobileTextRole::Caption, muted);
}

#[inline(never)]
fn render_settings_display_entry(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 181, "Personalization", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 204, 66),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    if model.is_pressed(MobilePressedTarget::Display) {
        canvas.rounded_rect(
            DRect::new(20, 208, 320, 58),
            20,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 237, model.accent(), Icon::Sun);
    canvas.text_role(82, 217, "Display & appearance", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        247,
        "Theme, color and software dimming",
        MobileTextRole::Caption,
        muted,
    );
    draw_icon(canvas, 318, 237, IconSize::Status, weak, Icon::ChevronRight);
}

#[inline(never)]
fn render_settings_connections(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 293, "Connections", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 316, 142),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    draw_setting_icon(canvas, 50, 351, weak, Icon::Wifi);
    canvas.text_role(82, 335, "Network & internet", MobileTextRole::Body, muted);
    canvas.text_role(
        82,
        365,
        "Unavailable in QEMU",
        MobileTextRole::Caption,
        weak,
    );
    canvas.text_end(316, 344, "Off", MobileTextRole::Label, weak);
    canvas.fill_rect(DRect::new(78, 387, 248, 1), model.outline_color());
    draw_setting_icon(canvas, 50, 422, weak, Icon::Airplane);
    canvas.text_role(82, 406, "Connected devices", MobileTextRole::Body, muted);
    canvas.text_role(
        82,
        436,
        "No device transport",
        MobileTextRole::Caption,
        weak,
    );
    canvas.text_end(316, 415, "Off", MobileTextRole::Label, weak);
}

#[inline(never)]
fn render_settings_phone_status(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 481, "This phone", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 504, 112),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    draw_setting_icon(canvas, 50, 539, weak, Icon::Bell);
    canvas.text_role(82, 523, "Sound & vibration", MobileTextRole::Body, muted);
    canvas.text_role(82, 553, "Not implemented", MobileTextRole::Caption, weak);
    canvas.fill_rect(DRect::new(78, 575, 248, 1), model.outline_color());
    draw_setting_icon(canvas, 50, 590, weak, Icon::Info);
    canvas.text_role(82, 578, "Hardware status", MobileTextRole::Body, muted);
    canvas.text_end(316, 581, "QEMU", MobileTextRole::Label, weak);
}

#[inline(never)]
fn render_settings_system_rows(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 629, "System", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 642, 132),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    if model.is_pressed(MobilePressedTarget::Apps) {
        canvas.rounded_rect(
            DRect::new(24, 646, 320, 58),
            20,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 675, COLOR_ANDROIDBOX, Icon::DroidCode);
    canvas.text_role(82, 651, "Apps", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        681,
        {
            #[cfg(feature = "androidbox-multipackage4")]
            {
                match model.android_installed_app_count {
                    0 => "No installed apps",
                    1 => "1 installed",
                    _ => "2 installed",
                }
            }
            #[cfg(not(feature = "androidbox-multipackage4"))]
            {
                if model.android_installed_app.installed {
                    "1 installed"
                } else {
                    "No installed apps"
                }
            }
        },
        MobileTextRole::Caption,
        muted,
    );
    draw_icon(canvas, 318, 675, IconSize::Status, weak, Icon::ChevronRight);
    canvas.fill_rect(DRect::new(78, 708, 248, 1), model.outline_color());
    if model.is_pressed(MobilePressedTarget::About) {
        canvas.rounded_rect(
            DRect::new(24, 712, 320, 58),
            20,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 741, model.accent(), Icon::Info);
    canvas.text_role(82, 717, "About phone", MobileTextRole::Body, text);
    canvas.text_role(82, 747, "Bndroid OS", MobileTextRole::Caption, muted);
    draw_icon(canvas, 318, 741, IconSize::Status, weak, Icon::ChevronRight);
}

#[inline(never)]
fn render_settings_local_status(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
    weak: u32,
) {
    // This static, explicitly local-only status card occupies the exact
    // below-fold extent that backs the bounded scroll range. It advertises no
    // unavailable service and introduces no inert control.
    canvas.card(DRect::new(16, 786, 328, 100), 20, model.dark_theme, false);
    draw_setting_icon(canvas, 50, 826, model.accent(), Icon::Info);
    canvas.text_role(82, 802, "System status", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        832,
        "Local preview only",
        MobileTextRole::Caption,
        muted,
    );
    canvas.fill_rect(DRect::new(36, 858, 288, 1), model.outline_color());
    canvas.text_role(
        36,
        866,
        "Network services remain unavailable",
        MobileTextRole::Label,
        weak,
    );
}

#[inline(never)]
fn render_settings(canvas: &mut Canvas<'_>, model: &MobileModel) {
    render_app_header(canvas, model, "Settings");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let scroll_offset_px = model.effective_page_scroll_offset_px();
    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_top_y_px = canvas.clip_top_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.clip_top_y_px =
        previous_clip_top_y_px.max(previous_offset_y_px + i32::from(PAGE_SCROLL_VIEWPORT_TOP_PX));
    canvas.clip_bottom_y_px =
        previous_clip_bottom_y_px.min(i32::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX));
    canvas.design_offset_y_px = previous_offset_y_px - i32::from(scroll_offset_px);

    render_settings_identity_card(canvas, model, text, muted);
    render_settings_display_entry(canvas, model, text, muted, weak);
    render_settings_connections(canvas, model, muted, weak);
    render_settings_phone_status(canvas, model, muted, weak);
    render_settings_system_rows(canvas, model, text, muted, weak);
    render_settings_local_status(canvas, model, text, muted, weak);

    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_top_y_px = previous_clip_top_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
    render_page_scroll_indicator(
        canvas,
        scroll_offset_px,
        SETTINGS_SCROLL_MAX_PX,
        model.dark_theme,
    );
}

#[inline(never)]
fn render_display_identity_card(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 94, 72),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        true,
    );
    draw_setting_icon(canvas, 54, 130, model.accent(), Icon::Sun);
    canvas.text_role(91, 108, "Screen & appearance", MobileTextRole::Title, text);
    canvas.text_role(
        91,
        139,
        "720 x 1600 / 2x UI scale",
        MobileTextRole::Caption,
        muted,
    );
}

#[inline(never)]
fn render_display_appearance_shell(canvas: &mut Canvas<'_>, model: &MobileModel, muted: u32) {
    canvas.text_role(24, 181, "Appearance", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 204, 226),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
}

#[inline(never)]
fn render_display_theme_row(canvas: &mut Canvas<'_>, model: &MobileModel, text: u32, muted: u32) {
    if model.is_pressed(MobilePressedTarget::Theme) {
        canvas.rounded_rect(
            DRect::new(20, 208, 320, 58),
            20,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 237, model.accent(), Icon::Moon);
    canvas.text_role(82, 221, "Dark mode", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        251,
        if model.dark_theme { "On" } else { "Off" },
        MobileTextRole::Caption,
        muted,
    );
    canvas.toggle(274, 222, model.dark_theme, model.accent());
    canvas.fill_rect(DRect::new(78, 269, 248, 1), model.outline_color());
}

#[inline(never)]
fn render_display_accent_row(canvas: &mut Canvas<'_>, model: &MobileModel, text: u32, muted: u32) {
    if model.is_pressed(MobilePressedTarget::Accent) {
        canvas.rounded_rect(
            DRect::new(20, 274, 320, 58),
            20,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 303, model.accent(), Icon::Palette);
    canvas.text_role(82, 287, "Accent color", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        317,
        if model.alternate_accent {
            "Violet"
        } else {
            "Ocean blue"
        },
        MobileTextRole::Caption,
        muted,
    );
    canvas.circle(302, 303, 12, model.accent());
    canvas.circle(302, 303, 5, model.on_accent());
    canvas.fill_rect(DRect::new(78, 335, 248, 1), model.outline_color());
}

#[inline(never)]
fn render_display_dimming_row(canvas: &mut Canvas<'_>, model: &MobileModel, text: u32, muted: u32) {
    if model.is_pressed(MobilePressedTarget::SoftwareDimming) {
        canvas.rounded_rect(
            DRect::new(168, 372, 168, 56),
            18,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 378, model.accent(), Icon::Sun);
    canvas.text_role(82, 346, "Software dimming", MobileTextRole::Body, text);
    canvas.text_role(82, 376, "Session only", MobileTextRole::Caption, muted);
    canvas.text_end(
        318,
        346,
        software_dimming_percentage(model.software_dimming),
        MobileTextRole::Label,
        text,
    );
    draw_software_dimming_slider(canvas, model, 407);
}

#[inline(never)]
fn render_display_screen_facts(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.text_role(24, 451, "Screen", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 474, 132),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    canvas.text_role(36, 493, "Resolution", MobileTextRole::Body, text);
    canvas.text_end(324, 502, "720 x 1600", MobileTextRole::Label, muted);
    canvas.fill_rect(DRect::new(36, 540, 288, 1), model.outline_color());
    canvas.text_role(36, 559, "Interface scale", MobileTextRole::Body, text);
    canvas.text_end(324, 568, "360 x 800 dp", MobileTextRole::Label, muted);
}

#[inline(never)]
fn render_display_accessibility_entry(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 629, "Accessibility", MobileTextRole::Label, muted);
    canvas.card(DRect::new(16, 652, 328, 96), 20, model.dark_theme, false);
    if model.is_pressed(MobilePressedTarget::Accessibility) {
        canvas.rounded_rect(
            DRect::new(20, 656, 320, 88),
            MobileShapeTokens::RADIUS_CONTROL,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 692, model.accent(), Icon::Accessibility);
    canvas.text_role(82, 670, "Text & contrast", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        704,
        match (model.large_text, model.high_contrast) {
            (false, false) => "Standard",
            (true, false) => "Larger text",
            (false, true) => "High contrast",
            (true, true) => "Larger text / High contrast",
        },
        MobileTextRole::Caption,
        weak,
    );
    draw_icon(canvas, 318, 692, IconSize::Status, weak, Icon::ChevronRight);
}

#[inline(never)]
fn render_display(canvas: &mut Canvas<'_>, model: &MobileModel) {
    render_app_header(canvas, model, "Display");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();

    render_display_identity_card(canvas, model, text, muted);
    render_display_appearance_shell(canvas, model, muted);
    render_display_theme_row(canvas, model, text, muted);
    render_display_accent_row(canvas, model, text, muted);
    render_display_dimming_row(canvas, model, text, muted);
    render_display_screen_facts(canvas, model, text, muted);
    render_display_accessibility_entry(canvas, model, text, muted, weak);
}

#[inline(never)]
fn render_accessibility_identity_card(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 94, 72),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        true,
    );
    draw_setting_icon(canvas, 54, 130, model.accent(), Icon::Accessibility);
    canvas.text_role(91, 108, "Visual accessibility", MobileTextRole::Title, text);
    canvas.text_role(
        91,
        139,
        "Session-wide and reversible",
        MobileTextRole::Caption,
        muted,
    );
}

#[inline(never)]
fn render_accessibility_preferences(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.text_role(24, 181, "Reading", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 204, 132),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    if model.is_pressed(MobilePressedTarget::LargeText) {
        canvas.rounded_rect(
            DRect::new(20, 208, 320, 58),
            MobileShapeTokens::RADIUS_CONTROL,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 237, model.accent(), Icon::TextSize);
    canvas.text_role(82, 221, "Larger text", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        251,
        if model.large_text { "On" } else { "Off" },
        MobileTextRole::Caption,
        muted,
    );
    canvas.toggle(274, 222, model.large_text, model.accent());
    canvas.fill_rect(DRect::new(78, 269, 248, 1), model.outline_color());

    if model.is_pressed(MobilePressedTarget::HighContrast) {
        canvas.rounded_rect(
            DRect::new(20, 274, 320, 58),
            MobileShapeTokens::RADIUS_CONTROL,
            pressed_surface_color_ref(model),
        );
    }
    draw_setting_icon(canvas, 50, 303, model.accent(), Icon::Contrast);
    canvas.text_role(82, 287, "High contrast", MobileTextRole::Body, text);
    canvas.text_role(
        82,
        317,
        if model.high_contrast { "On" } else { "Off" },
        MobileTextRole::Caption,
        muted,
    );
    canvas.toggle(274, 288, model.high_contrast, model.accent());
}

#[inline(never)]
fn render_accessibility_preview(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.text_role(24, 359, "Preview", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 382, 154),
        MobileShapeTokens::RADIUS_LARGE,
        model.dark_theme,
        true,
    );
    canvas.rounded_rect(DRect::new(34, 404, 64, 64), 22, model.accent());
    canvas.text_center(66, 413, "Aa", MobileTextRole::Title, model.on_accent());
    canvas.text_role(116, 404, "Readable by design", MobileTextRole::Body, text);
    canvas.text_role(
        116,
        440,
        if model.large_text {
            "Larger semantic type"
        } else {
            "Standard semantic type"
        },
        MobileTextRole::Caption,
        muted,
    );
    canvas.fill_rect(DRect::new(34, 484, 292, 1), model.outline_color());
    canvas.text_role(
        34,
        502,
        "The same preference reaches Shell and apps.",
        MobileTextRole::Label,
        muted,
    );
}

#[inline(never)]
fn render_accessibility_boundary(
    canvas: &mut Canvas<'_>,
    model: &MobileModel,
    text: u32,
    muted: u32,
    weak: u32,
) {
    canvas.text_role(24, 565, "Scope", MobileTextRole::Label, muted);
    canvas.card(
        DRect::screen_inset(MobileSpacingTokens::LG, 588, 128),
        MobileShapeTokens::RADIUS_CONTROL,
        model.dark_theme,
        false,
    );
    draw_setting_icon(canvas, 50, 626, model.accent(), Icon::Info);
    canvas.text_role(
        82,
        604,
        "Software UI preference",
        MobileTextRole::Body,
        text,
    );
    canvas.text_role(
        82,
        638,
        "No device or service authority",
        MobileTextRole::Caption,
        weak,
    );
    canvas.fill_rect(DRect::new(36, 668, 288, 1), model.outline_color());
    canvas.text_role(
        36,
        682,
        "Screen reader semantics are still pending",
        MobileTextRole::Label,
        weak,
    );
}

#[inline(never)]
fn render_accessibility(canvas: &mut Canvas<'_>, model: &MobileModel) {
    render_app_header(canvas, model, "Accessibility");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    render_accessibility_identity_card(canvas, model, text, muted);
    render_accessibility_preferences(canvas, model, text, muted);
    render_accessibility_preview(canvas, model, text, muted);
    render_accessibility_boundary(canvas, model, text, muted, weak);
}

fn render_single_installed_apps_summary(
    canvas: &mut Canvas<'_>,
    model: MobileModel,
    installed: AndroidInstalledAppStatus,
    text: u32,
    muted: u32,
) {
    canvas.card(DRect::new(18, 116, 324, 104), 24, model.dark_theme, true);
    draw_installed_app_icon(canvas, 55, 162, installed);
    canvas.text_role(
        94,
        126,
        fitted_mobile_text_for_scale(
            installed.title.as_str(),
            MobileTextRole::Body,
            470,
            model.large_text,
        ),
        MobileTextRole::Body,
        text,
    );
    canvas.text_role(
        94,
        158,
        fitted_mobile_text_for_scale(
            installed.package.as_str(),
            MobileTextRole::Caption,
            470,
            model.large_text,
        ),
        MobileTextRole::Caption,
        muted,
    );
    canvas.circle(100, 194, 4, installed_app_fallback_color(installed));
    canvas.text_role(
        112,
        185,
        "Installed",
        MobileTextRole::Label,
        installed_app_fallback_color(installed),
    );
    #[cfg(feature = "androidbox-runtime-install2")]
    if model.android_install_candidate.present {
        canvas.card(
            DRect::new(240, 168, 92, 44),
            18,
            model.dark_theme,
            model.is_pressed(MobilePressedTarget::AppsInstall),
        );
        canvas.text_center(
            286,
            179,
            match model.android_install_candidate.action {
                AndroidInstallCandidateAction::Install => "Install",
                AndroidInstallCandidateAction::Update => "Update",
                AndroidInstallCandidateAction::Reinstall => "Reinstall",
            },
            MobileTextRole::Label,
            model.accent(),
        );
    }
}

#[cfg(feature = "androidbox-multipackage4")]
fn render_multi_installed_apps_summary(
    canvas: &mut Canvas<'_>,
    model: MobileModel,
    text: u32,
    muted: u32,
) {
    canvas.card(DRect::new(18, 116, 324, 104), 24, model.dark_theme, true);
    let selected_index = model.android_installed_apps
        [..usize::from(model.android_installed_app_count)]
        .iter()
        .position(|app| *app == model.android_installed_app)
        .unwrap_or(0);

    for index in 0..2 {
        let app = model.android_installed_apps[index];
        let left = 18 + index as i32 * 162;
        let target = MobilePressedTarget::AppsInstalledAndroid(index as u8);
        if selected_index == index || model.is_pressed(target) {
            canvas.rounded_rect(
                DRect::new(left + 3, 119, 156, 98),
                21,
                if model.is_pressed(target) {
                    pressed_surface_color(model)
                } else {
                    blend(
                        model.surface_color(true),
                        installed_app_fallback_color(app),
                        if model.dark_theme { 58 } else { 30 },
                    )
                },
            );
        }
        draw_installed_app_icon(canvas, left + 36, 154, app);
        let (title_first, title_second) =
            split_installed_app_label(app.title.as_str(), model.large_text);
        canvas.text_role(
            left + 70,
            126,
            fitted_mobile_text_for_scale(title_first, MobileTextRole::Label, 166, model.large_text),
            MobileTextRole::Label,
            text,
        );
        if !title_second.is_empty() {
            canvas.text_role(
                left + 70,
                149,
                fitted_mobile_text_for_scale(
                    title_second,
                    MobileTextRole::Label,
                    166,
                    model.large_text,
                ),
                MobileTextRole::Label,
                muted,
            );
        }
        canvas.circle(
            left + 74,
            194,
            3,
            if selected_index == index {
                installed_app_fallback_color(app)
            } else {
                muted
            },
        );
        canvas.text_role(
            left + 83,
            185,
            if selected_index == index {
                "Selected"
            } else {
                "Installed"
            },
            MobileTextRole::Label,
            if selected_index == index {
                installed_app_fallback_color(app)
            } else {
                muted
            },
        );
    }
    canvas.fill_rect(DRect::new(180, 126, 1, 84), model.outline_color());
}

#[inline(never)]
fn render_apps(canvas: &mut Canvas<'_>, model: MobileModel) {
    render_app_header(canvas, &model, "Apps");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let installed = model.android_installed_app;
    let scroll_offset_px = model.effective_page_scroll_offset_px();
    let previous_offset_y_px = canvas.design_offset_y_px;
    let previous_clip_top_y_px = canvas.clip_top_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.clip_top_y_px =
        previous_clip_top_y_px.max(previous_offset_y_px + i32::from(PAGE_SCROLL_VIEWPORT_TOP_PX));
    canvas.clip_bottom_y_px =
        previous_clip_bottom_y_px.min(i32::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX));
    canvas.design_offset_y_px = previous_offset_y_px - i32::from(scroll_offset_px);

    canvas.text_role(24, 96, "Installed apps", MobileTextRole::Label, muted);
    if !installed.installed {
        #[cfg(feature = "androidbox-runtime-install2")]
        let candidate = model.android_install_candidate;
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        let removed = matches!(
            model.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Removed { .. }
        );
        #[cfg(not(feature = "androidbox-runtime-uninstall1"))]
        let removed = false;
        canvas.card(DRect::new(18, 120, 324, 246), 24, model.dark_theme, true);
        #[cfg(feature = "androidbox-runtime-install2")]
        let candidate_present = candidate.present;
        #[cfg(not(feature = "androidbox-runtime-install2"))]
        let candidate_present = false;
        canvas.circle(180, 178, 34, model.surface_color(false));
        draw_icon(
            canvas,
            180,
            178,
            IconSize::App,
            if candidate_present {
                COLOR_ANDROIDBOX
            } else {
                muted
            },
            Icon::DroidCode,
        );
        canvas.text_center(
            180,
            226,
            if candidate_present {
                #[cfg(feature = "androidbox-runtime-install2")]
                {
                    match candidate.action {
                        AndroidInstallCandidateAction::Install => "Ready to install",
                        AndroidInstallCandidateAction::Update => "Update available",
                        AndroidInstallCandidateAction::Reinstall => "Ready to reinstall",
                    }
                }
                #[cfg(not(feature = "androidbox-runtime-install2"))]
                {
                    "Ready"
                }
            } else if removed {
                "App uninstalled"
            } else {
                "No installed Android apps"
            },
            MobileTextRole::Title,
            text,
        );
        canvas.text_center(
            180,
            258,
            if candidate_present {
                #[cfg(feature = "androidbox-runtime-install2")]
                {
                    fitted_mobile_text_for_scale(
                        candidate.title.as_str(),
                        MobileTextRole::Caption,
                        520,
                        model.large_text,
                    )
                }
                #[cfg(not(feature = "androidbox-runtime-install2"))]
                {
                    "Local APK"
                }
            } else {
                "Local sideload only"
            },
            MobileTextRole::Caption,
            muted,
        );
        if candidate_present {
            #[cfg(feature = "androidbox-runtime-install2")]
            {
                canvas.card(
                    DRect::new(36, 286, 288, 56),
                    18,
                    model.dark_theme,
                    model.is_pressed(MobilePressedTarget::AppsInstall),
                );
                canvas.text_center(
                    180,
                    300,
                    match candidate.action {
                        AndroidInstallCandidateAction::Install => "Install",
                        AndroidInstallCandidateAction::Update => "Update",
                        AndroidInstallCandidateAction::Reinstall => "Reinstall",
                    },
                    MobileTextRole::Body,
                    model.accent(),
                );
            }
        } else {
            canvas.text_center(
                180,
                304,
                "Verified packages will appear here",
                MobileTextRole::Label,
                weak,
            );
        }

        canvas.text_role(24, 402, "Admission profile", MobileTextRole::Label, muted);
        canvas.card(DRect::new(18, 426, 324, 122), 22, model.dark_theme, false);
        draw_setting_icon(canvas, 52, 466, COLOR_ANDROIDBOX, Icon::DroidCode);
        canvas.text_role(82, 443, "APK v2 required", MobileTextRole::Body, text);
        canvas.text_role(
            82,
            475,
            "Resources-1 profile",
            MobileTextRole::Caption,
            muted,
        );
        canvas.fill_rect(DRect::new(36, 510, 288, 1), model.outline_color());
        canvas.text_role(36, 520, "No app is installed", MobileTextRole::Label, weak);
    } else {
        #[cfg(feature = "androidbox-multipackage4")]
        if apps_package_selection_is_available(model) {
            render_multi_installed_apps_summary(canvas, model, text, muted);
        } else {
            render_single_installed_apps_summary(canvas, model, installed, text, muted);
        }
        #[cfg(not(feature = "androidbox-multipackage4"))]
        render_single_installed_apps_summary(canvas, model, installed, text, muted);

        let mut version = MobileAsciiText::<10>::new();
        version.push_u32(installed.version_code);
        let mut apk_size = MobileAsciiText::<20>::new();
        apk_size.push_u32(installed.apk_length);
        apk_size.push_bytes(b" bytes");
        let mut generation = MobileAsciiText::<20>::new();
        generation.push_u64(installed.generation);
        let mut apk_digest = MobileAsciiText::<8>::new();
        apk_digest.push_hex_prefix(&installed.apk_digest_sha256, 4);
        let mut signer_digest = MobileAsciiText::<8>::new();
        signer_digest.push_hex_prefix(&installed.signer_digest_sha256, 4);

        canvas.text_role(24, 244, "Package details", MobileTextRole::Label, muted);
        canvas.card(DRect::new(18, 266, 324, 236), 22, model.dark_theme, false);
        for (index, (label, value, value_color)) in [
            ("Version", version.as_str(), text),
            ("APK size", apk_size.as_str(), text),
            ("Generation", generation.as_str(), text),
            ("Signature", "APK v2 verified", COLOR_ANDROIDBOX),
            ("Profile", "Resources-1", COLOR_ANDROIDBOX),
            ("APK digest", apk_digest.as_str(), text),
            ("Signer digest", signer_digest.as_str(), text),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 280 + index as i32 * 31;
            canvas.text_role(38, y, label, MobileTextRole::Label, muted);
            canvas.text_end(320, y, value, MobileTextRole::Caption, value_color);
            if index != 6 {
                canvas.fill_rect(DRect::new(38, y + 23, 284, 1), model.outline_color());
            }
        }

        canvas.text_role(24, 524, "Launch", MobileTextRole::Label, muted);
        canvas.card(DRect::new(18, 546, 324, 122), 22, model.dark_theme, false);
        canvas.text_role(36, 562, "Available in All apps", MobileTextRole::Body, text);
        canvas.fill_rect(DRect::new(36, 593, 288, 1), model.outline_color());
        canvas.text_role(
            36,
            608,
            fitted_mobile_text_for_scale(
                installed.activity.as_str(),
                MobileTextRole::Caption,
                520,
                model.large_text,
            ),
            MobileTextRole::Caption,
            muted,
        );
        canvas.text_center(180, 710, "Local sideload only", MobileTextRole::Label, weak);

        #[cfg(feature = "androidbox-runtime-uninstall1")]
        {
            let uninstall_available = apps_uninstall_is_available(model);
            canvas.text_role(24, 690, "Storage", MobileTextRole::Label, muted);
            canvas.card(
                DRect::new(18, 712, 324, 74),
                22,
                model.dark_theme,
                uninstall_available && model.is_pressed(MobilePressedTarget::AppsUninstall),
            );
            draw_setting_icon(
                canvas,
                52,
                749,
                if uninstall_available {
                    COLOR_AMBER
                } else {
                    weak
                },
                Icon::Backspace,
            );
            canvas.text_role(
                82,
                724,
                "Uninstall app",
                MobileTextRole::Body,
                if uninstall_available {
                    COLOR_AMBER
                } else {
                    weak
                },
            );
            canvas.text_role(
                82,
                755,
                if uninstall_available {
                    "Remove launcher entry and package"
                } else {
                    "Resolve the pending update first"
                },
                MobileTextRole::Caption,
                muted,
            );
        }
    }

    #[cfg(not(feature = "androidbox-runtime-uninstall1"))]
    let compatibility_y = 774;
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    let compatibility_y = 818;
    canvas.card(
        DRect::new(18, compatibility_y, 324, 92),
        22,
        model.dark_theme,
        false,
    );
    draw_setting_icon(
        canvas,
        52,
        compatibility_y + 44,
        COLOR_ANDROIDBOX,
        Icon::DroidCode,
    );
    canvas.text_role(
        82,
        compatibility_y + 18,
        "Compatibility boundary",
        MobileTextRole::Body,
        text,
    );
    canvas.text_role(
        82,
        compatibility_y + 50,
        "Resources-1 packages only",
        MobileTextRole::Caption,
        muted,
    );
    canvas.text_role(
        36,
        compatibility_y + 76,
        "General Android APIs are unavailable",
        MobileTextRole::Label,
        weak,
    );

    canvas.design_offset_y_px = previous_offset_y_px;
    canvas.clip_top_y_px = previous_clip_top_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
    render_page_scroll_indicator(
        canvas,
        scroll_offset_px,
        APPS_SCROLL_MAX_PX,
        model.dark_theme,
    );
    #[cfg(feature = "androidbox-runtime-uninstall1")]
    render_apps_uninstall_dialog(canvas, model);
    #[cfg(feature = "androidbox-runtime-install2")]
    render_apps_install_dialog(canvas, model);
}

#[cfg(feature = "androidbox-runtime-install2")]
fn render_apps_install_dialog(canvas: &mut Canvas<'_>, model: MobileModel) {
    let state = model.android_install;
    if !state.blocks_apps_page() {
        return;
    }
    let text = model.text_primary();
    let muted = model.text_secondary();
    let candidate = model.android_install_candidate;
    canvas.fill_rect(DRect::new(0, 88, 360, 686), 0x0010_1524);
    canvas.card(DRect::new(20, 218, 320, 374), 28, model.dark_theme, true);
    canvas.circle(180, 286, 34, model.surface_color(false));
    draw_icon(
        canvas,
        180,
        286,
        IconSize::App,
        COLOR_ANDROIDBOX,
        Icon::DroidCode,
    );

    match state {
        AndroidInstallStatus::Confirming { .. } => {
            let (heading, confirm) = match candidate.action {
                AndroidInstallCandidateAction::Install => ("Install this app?", "Install"),
                AndroidInstallCandidateAction::Update => ("Update this app?", "Update"),
                AndroidInstallCandidateAction::Reinstall => ("Reinstall this app?", "Reinstall"),
            };
            canvas.text_center(180, 342, heading, MobileTextRole::Title, text);
            canvas.text_center(
                180,
                386,
                fitted_mobile_text_for_scale(
                    candidate.title.as_str(),
                    MobileTextRole::Body,
                    520,
                    model.large_text,
                ),
                MobileTextRole::Body,
                text,
            );
            canvas.text_center(
                180,
                422,
                fitted_mobile_text_for_scale(
                    candidate.package.as_str(),
                    MobileTextRole::Caption,
                    520,
                    model.large_text,
                ),
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_center(
                180,
                454,
                "Signature and package identity verified",
                MobileTextRole::Caption,
                muted,
            );
            render_dialog_button(
                canvas,
                DRect::new(40, 508, 125, 56),
                "Cancel",
                model.is_pressed(MobilePressedTarget::AppsInstallCancel),
                model.dark_theme,
                false,
            );
            render_dialog_button(
                canvas,
                DRect::new(195, 508, 125, 56),
                confirm,
                model.is_pressed(MobilePressedTarget::AppsInstallConfirm),
                model.dark_theme,
                false,
            );
        }
        AndroidInstallStatus::Pending { .. } => {
            canvas.text_center(
                180,
                352,
                match candidate.action {
                    AndroidInstallCandidateAction::Install => "Installing...",
                    AndroidInstallCandidateAction::Update => "Updating...",
                    AndroidInstallCandidateAction::Reinstall => "Reinstalling...",
                },
                MobileTextRole::Title,
                text,
            );
            canvas.text_center(
                180,
                402,
                "Verifying and committing package",
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_center(
                180,
                438,
                "Please keep the system running",
                MobileTextRole::Caption,
                muted,
            );
        }
        AndroidInstallStatus::Failed { failure, .. } => {
            canvas.text_center(180, 342, "Could not install", MobileTextRole::Title, text);
            let detail = match failure {
                AndroidInstallFailure::Stale => "The package candidate changed",
                AndroidInstallFailure::Busy => "Package runtime is busy",
                AndroidInstallFailure::Storage => "Storage needs a reboot",
                AndroidInstallFailure::Verification => "Package verification failed",
                AndroidInstallFailure::Unsupported => "This APK profile is unsupported",
            };
            canvas.text_center(180, 400, detail, MobileTextRole::Caption, COLOR_AMBER);
            render_dialog_button(
                canvas,
                DRect::new(40, 508, 125, 56),
                "Done",
                model.is_pressed(MobilePressedTarget::AppsInstallCancel),
                model.dark_theme,
                false,
            );
        }
        AndroidInstallStatus::Idle | AndroidInstallStatus::Installed { .. } => {}
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn render_apps_uninstall_dialog(canvas: &mut Canvas<'_>, model: MobileModel) {
    let state = model.android_installed_uninstall;
    if !state.blocks_apps_page() {
        return;
    }
    let text = model.text_primary();
    let muted = model.text_secondary();
    canvas.fill_rect(DRect::new(0, 88, 360, 686), 0x0010_1524);
    canvas.card(DRect::new(20, 218, 320, 374), 28, model.dark_theme, true);
    canvas.circle(180, 286, 34, model.surface_color(false));
    draw_icon(
        canvas,
        180,
        286,
        IconSize::App,
        COLOR_AMBER,
        Icon::DroidCode,
    );

    match state {
        AndroidInstalledUninstallStatus::Confirming { .. } => {
            canvas.text_center(180, 342, "Uninstall this app?", MobileTextRole::Title, text);
            canvas.text_center(
                180,
                388,
                fitted_mobile_text_for_scale(
                    model.android_installed_app.title.as_str(),
                    MobileTextRole::Body,
                    520,
                    model.large_text,
                ),
                MobileTextRole::Body,
                text,
            );
            canvas.text_center(
                180,
                424,
                "The launcher entry will be removed.",
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_center(
                180,
                454,
                "No managed app data exists yet.",
                MobileTextRole::Caption,
                muted,
            );
            render_dialog_button(
                canvas,
                DRect::new(40, 508, 125, 56),
                "Cancel",
                model.is_pressed(MobilePressedTarget::AppsUninstallCancel),
                model.dark_theme,
                false,
            );
            render_dialog_button(
                canvas,
                DRect::new(195, 508, 125, 56),
                "Uninstall",
                model.is_pressed(MobilePressedTarget::AppsUninstallConfirm),
                model.dark_theme,
                true,
            );
        }
        AndroidInstalledUninstallStatus::Pending { .. } => {
            canvas.text_center(180, 352, "Uninstalling...", MobileTextRole::Title, text);
            canvas.text_center(
                180,
                402,
                "Writing a durable package tombstone",
                MobileTextRole::Caption,
                muted,
            );
            canvas.text_center(
                180,
                438,
                "Please keep the system running",
                MobileTextRole::Caption,
                muted,
            );
        }
        AndroidInstalledUninstallStatus::Failed { failure, .. } => {
            canvas.text_center(180, 342, "Could not uninstall", MobileTextRole::Title, text);
            let detail = match failure {
                AndroidInstalledUninstallFailure::Stale => "The installed package changed",
                AndroidInstalledUninstallFailure::Busy => "Package runtime is busy",
                AndroidInstalledUninstallFailure::Storage => "Storage needs a reboot",
                AndroidInstalledUninstallFailure::Verification => "Package state is inconsistent",
            };
            canvas.text_center(180, 400, detail, MobileTextRole::Caption, COLOR_AMBER);
            render_dialog_button(
                canvas,
                DRect::new(40, 508, 125, 56),
                "Done",
                model.is_pressed(MobilePressedTarget::AppsUninstallCancel),
                model.dark_theme,
                false,
            );
        }
        AndroidInstalledUninstallStatus::Idle | AndroidInstalledUninstallStatus::Removed { .. } => {
        }
    }
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
fn render_dialog_button(
    canvas: &mut Canvas<'_>,
    rect: DRect,
    label: &str,
    pressed: bool,
    dark_theme: bool,
    destructive: bool,
) {
    canvas.card(rect, 18, dark_theme, pressed);
    let color = if destructive {
        COLOR_AMBER
    } else {
        text_color(dark_theme)
    };
    canvas.text_center(
        rect.x + rect.width / 2,
        rect.y + 18,
        label,
        MobileTextRole::Body,
        color,
    );
}

fn render_page_scroll_indicator(
    canvas: &mut Canvas<'_>,
    offset_px: u16,
    maximum_px: u16,
    dark_theme: bool,
) {
    if offset_px == 0 || maximum_px == 0 {
        return;
    }
    const INDICATOR_TOP_DESIGN_Y: i32 = 94;
    const INDICATOR_HEIGHT_DESIGN_PX: i32 = 32;
    const INDICATOR_TRAVEL_DESIGN_PX: i32 = 648;

    let previous_clip_top_y_px = canvas.clip_top_y_px;
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    canvas.clip_top_y_px = previous_clip_top_y_px
        .max(canvas.design_offset_y_px + i32::from(PAGE_SCROLL_VIEWPORT_TOP_PX));
    canvas.clip_bottom_y_px =
        previous_clip_bottom_y_px.min(i32::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX));
    let indicator_y = INDICATOR_TOP_DESIGN_Y
        + i32::from(offset_px) * INDICATOR_TRAVEL_DESIGN_PX / i32::from(maximum_px);
    canvas.rounded_rect(
        DRect::new(352, indicator_y, 3, INDICATOR_HEIGHT_DESIGN_PX),
        2,
        if dark_theme { 0x0095_a0b1 } else { 0x0075_8192 },
    );
    canvas.clip_top_y_px = previous_clip_top_y_px;
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

fn draw_software_dimming_slider(canvas: &mut Canvas<'_>, model: &MobileModel, center_y: i32) {
    let thumb_x = software_dimming_thumb_x(model.software_dimming);
    canvas.rounded_rect(
        DRect::new(178, center_y - 4, 146, 8),
        4,
        model.outline_color(),
    );
    if thumb_x > 178 {
        canvas.rounded_rect(
            DRect::new(178, center_y - 4, thumb_x - 178, 8),
            4,
            model.accent(),
        );
    }
    for stop_x in [178, 214, 251, 287, 324] {
        canvas.circle(
            stop_x,
            center_y,
            2,
            if stop_x <= thumb_x {
                model.accent()
            } else {
                model.text_secondary()
            },
        );
    }
    canvas.circle(thumb_x, center_y, 12, COLOR_WHITE);
    canvas.circle(thumb_x, center_y, 8, model.accent());
}

#[inline(never)]
fn render_about(canvas: &mut Canvas<'_>, model: MobileModel) {
    render_app_header(canvas, &model, "About phone");
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();

    canvas.circle(180, 151, 40, model.accent());
    canvas.circle(180, 151, 31, blend(model.accent(), COLOR_WHITE, 32));
    canvas.text_center(180, 139, "BN", MobileTextRole::Title, model.on_accent());
    canvas.text_center(180, 208, "Bndroid OS", MobileTextRole::Title, text);
    canvas.text_center(180, 242, "Preview build", MobileTextRole::Caption, muted);

    canvas.text_role(24, 282, "Preview environment", MobileTextRole::Label, muted);
    canvas.card(DRect::new(18, 304, 324, 236), 22, model.dark_theme, false);
    let information = [
        ("Display", "720 x 1600"),
        ("Profile", "360 grid @ 2x"),
        ("Runtime", "AArch64"),
        ("Settings", "UI session"),
        ("Network", "Disabled"),
    ];
    for (index, (label, value)) in information.into_iter().enumerate() {
        let y = 332 + index as i32 * 42;
        canvas.text_role(38, y, label, MobileTextRole::Label, muted);
        canvas.text_end(316, y, value, MobileTextRole::Caption, text);
        if index + 1 != information.len() {
            canvas.fill_rect(DRect::new(38, y + 25, 284, 1), model.outline_color());
        }
    }

    canvas.card(DRect::new(18, 568, 324, 116), 22, model.dark_theme, true);
    canvas.circle(50, 626, 18, COLOR_AMBER);
    draw_icon(
        canvas,
        50,
        626,
        IconSize::List,
        COLOR_TEXT_LIGHT,
        Icon::Airplane,
    );
    canvas.text_role(80, 585, "Preview boundary", MobileTextRole::Label, weak);
    canvas.text_role(80, 610, "Local QEMU only", MobileTextRole::Body, text);
    canvas.text_role(
        80,
        641,
        "No hardware or network",
        MobileTextRole::Caption,
        muted,
    );
    canvas.text_role(
        80,
        662,
        "Not a physical-device claim",
        MobileTextRole::Label,
        weak,
    );
}

fn render_quick_settings(canvas: &mut Canvas<'_>, model: MobileModel, shade_reveal_px: u16) {
    let text = model.text_primary();
    let muted = model.text_secondary();
    let weak = model.text_tertiary();
    let date = model.time.short_date_text();
    let theme = model.theme_tokens();
    let panel = theme.panel;

    let full_tint = if model.dark_theme { 112 } else { 54 };
    let tint = full_tint * u32::from(shade_reveal_px) / u32::from(SHADE_REVEAL_MAX);
    canvas.tint(COLOR_BLACK, tint);
    let previous_clip_bottom_y_px = canvas.clip_bottom_y_px;
    if shade_reveal_px < SHADE_REVEAL_MAX {
        // A phone shade expands downward from the status edge: keep its top
        // controls anchored and reveal progressively more content instead of
        // sliding the bottom-most notification into view first. Give the
        // moving edge its own rounded cap and shadow so partially revealed
        // cards do not end at a flat framebuffer cut.
        let reveal_bottom = (i32::from(shade_reveal_px) + SCALE - 1) / SCALE;
        canvas.rounded_rect(
            DRect::new(-24, -24, 408, reveal_bottom + 28),
            38,
            theme.shadow,
        );
        canvas.clip_bottom_y_px = previous_clip_bottom_y_px.min(i32::from(shade_reveal_px));
        canvas.rounded_rect(DRect::new(-24, -28, 408, reveal_bottom + 28), 38, panel);
    } else {
        canvas.rounded_rect(DRect::new(-24, -24, 408, 824), 38, theme.shadow);
        canvas.rounded_rect(DRect::new(-24, -28, 408, 828), 38, panel);
    }
    canvas.radial_glow(
        330,
        34,
        150,
        theme.wallpaper_glow_primary,
        if model.dark_theme { 42 } else { 28 },
    );
    canvas.radial_glow(
        12,
        404,
        170,
        theme.wallpaper_glow_secondary,
        if model.dark_theme { 28 } else { 18 },
    );

    draw_time_role(canvas, 20, 56, model.time, MobileTextRole::Headline, text);
    canvas.text_role(22, 96, date.as_str(), MobileTextRole::Label, muted);
    canvas.circle(
        324,
        80,
        20,
        if model.is_pressed(MobilePressedTarget::ShadeClose) {
            pressed_surface_color(model)
        } else {
            model.surface_color(true)
        },
    );
    draw_icon(canvas, 324, 80, IconSize::List, text, Icon::ChevronDown);

    canvas.text_role(20, 124, "Quick settings", MobileTextRole::Label, muted);
    canvas.card(DRect::new(16, 148, 156, 84), 20, model.dark_theme, true);
    if model.is_pressed(MobilePressedTarget::Theme) {
        canvas.rounded_rect(
            DRect::new(20, 152, 148, 76),
            18,
            pressed_surface_color(model),
        );
    }
    draw_setting_icon(canvas, 50, 181, model.accent(), Icon::Moon);
    canvas.text_role(78, 163, "Theme", MobileTextRole::Body, text);
    canvas.text_role(
        78,
        194,
        if model.dark_theme { "Dark" } else { "Light" },
        MobileTextRole::Caption,
        muted,
    );
    canvas.circle(146, 214, 6, model.accent());
    canvas.circle(146, 214, 2, model.on_accent());

    canvas.card(DRect::new(188, 148, 156, 84), 20, model.dark_theme, true);
    if model.is_pressed(MobilePressedTarget::Accent) {
        canvas.rounded_rect(
            DRect::new(192, 152, 148, 76),
            18,
            pressed_surface_color(model),
        );
    }
    draw_setting_icon(canvas, 222, 181, model.accent(), Icon::Palette);
    canvas.text_role(250, 163, "Accent", MobileTextRole::Body, text);
    canvas.text_role(
        250,
        194,
        if model.alternate_accent {
            "Violet"
        } else {
            "Ocean"
        },
        MobileTextRole::Caption,
        muted,
    );
    canvas.circle(318, 214, 6, model.accent());
    canvas.circle(318, 214, 2, model.on_accent());

    canvas.card(DRect::new(16, 244, 156, 84), 20, model.dark_theme, false);
    draw_setting_icon(canvas, 50, 277, COLOR_SETTINGS, Icon::Wifi);
    canvas.text_role(78, 259, "Wi-Fi", MobileTextRole::Body, text);
    canvas.text_role(78, 290, "Unavailable", MobileTextRole::Caption, muted);
    canvas.rounded_rect(DRect::new(124, 306, 32, 14), 7, COLOR_TOGGLE_OFF);
    canvas.text_center(140, 307, "Off", MobileTextRole::Label, COLOR_WHITE);

    canvas.card(DRect::new(188, 244, 156, 84), 20, model.dark_theme, false);
    draw_setting_icon(canvas, 222, 277, COLOR_AMBER, Icon::Airplane);
    canvas.text_role(250, 259, "Network", MobileTextRole::Body, text);
    canvas.text_role(250, 290, "Disabled", MobileTextRole::Caption, muted);

    canvas.card(DRect::new(16, 344, 328, 60), 20, model.dark_theme, true);
    if model.is_pressed(MobilePressedTarget::SoftwareDimming) {
        canvas.rounded_rect(
            DRect::new(168, 344, 168, 60),
            18,
            pressed_surface_color(model),
        );
    }
    draw_setting_icon(canvas, 50, 374, model.accent(), Icon::Sun);
    canvas.text_role(78, 352, "Display", MobileTextRole::Body, text);
    canvas.text_role(
        78,
        382,
        software_dimming_percentage(model.software_dimming),
        MobileTextRole::Caption,
        muted,
    );
    draw_software_dimming_slider(canvas, &model, 374);

    canvas.text_role(20, 428, "Notifications", MobileTextRole::Label, muted);
    if model.boot_notification_visible {
        let offset = i32::from(model.boot_notification_offset_px) / SCALE;
        if offset != 0 {
            let underlay = blend(model.surface_color(true), model.accent(), 72);
            canvas.rounded_rect(DRect::new(16, 452, 328, 116), 22, underlay);
            canvas.text_center(
                if offset > 0 { 52 } else { 308 },
                506,
                "Dismiss",
                MobileTextRole::Label,
                COLOR_WHITE,
            );
        }
        canvas.card(
            DRect::new(16 + offset, 452, 328, 116),
            22,
            model.dark_theme,
            true,
        );
        if model.is_pressed(MobilePressedTarget::BootNotification) {
            canvas.rounded_rect(
                DRect::new(20 + offset, 456, 320, 108),
                20,
                pressed_surface_color(model),
            );
        }
        canvas.circle(52 + offset, 510, 22, model.accent());
        draw_icon(
            canvas,
            52 + offset,
            510,
            IconSize::List,
            model.on_accent(),
            Icon::Info,
        );
        canvas.text_role(82 + offset, 464, "System UI", MobileTextRole::Label, weak);
        canvas.text_role(
            82 + offset,
            487,
            "Local preview",
            MobileTextRole::Body,
            text,
        );
        canvas.text_role(
            82 + offset,
            517,
            if model.boot_notification_expanded {
                "No hardware or network"
            } else {
                "Tap to review system limits"
            },
            MobileTextRole::Caption,
            muted,
        );
        draw_icon(
            canvas,
            320 + offset,
            510,
            IconSize::Status,
            muted,
            if model.boot_notification_expanded {
                Icon::ChevronUp
            } else {
                Icon::ChevronRight
            },
        );
    } else {
        canvas.card(DRect::new(16, 452, 328, 168), 22, model.dark_theme, false);
        canvas.circle(180, 502, 28, model.surface_color(true));
        draw_icon(canvas, 180, 502, IconSize::App, muted, Icon::Bell);
        canvas.text_center(180, 544, "No notifications", MobileTextRole::Body, text);
        canvas.text_center(
            180,
            576,
            "Nothing is waiting",
            MobileTextRole::Caption,
            weak,
        );
    }
    canvas.clip_bottom_y_px = previous_clip_bottom_y_px;
}

/// Code-drawn glyphs share one 24-unit optical grid. Containers are drawn by
/// their callers, so every glyph is a single foreground color and remains
/// independent of wallpaper, theme, and accent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Icon {
    Phone,
    Messages,
    Calculator,
    Settings,
    DroidCode,
    Moon,
    Palette,
    Sun,
    Accessibility,
    TextSize,
    Contrast,
    Wifi,
    Airplane,
    Info,
    Backspace,
    Calendar,
    Bell,
    Lock,
    BatteryUnknown,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IconSize {
    Status,
    List,
    App,
}

impl IconSize {
    const fn design_px(self) -> i32 {
        match self {
            Self::Status => 16,
            Self::List => 20,
            Self::App => 26,
        }
    }

    const fn stroke_px(self) -> i32 {
        match self {
            Self::Status => 1,
            Self::List | Self::App => 2,
        }
    }
}

const fn icon_offset(value: i32, size: i32) -> i32 {
    value * size / 24
}

fn draw_icon(canvas: &mut Canvas<'_>, x: i32, y: i32, size: IconSize, color: u32, icon: Icon) {
    let size_px = size.design_px();
    let stroke = size.stroke_px();
    let ox = |value| x + icon_offset(value, size_px);
    let oy = |value| y + icon_offset(value, size_px);

    match icon {
        Icon::Phone => {
            canvas.round_line(ox(-8), oy(-9), ox(-4), oy(3), stroke, color);
            canvas.round_line(ox(-4), oy(3), ox(8), oy(9), stroke, color);
            canvas.aa_circle(ox(-8), oy(-9), stroke + 1, color);
            canvas.aa_circle(ox(8), oy(9), stroke + 1, color);
        }
        Icon::Messages => {
            for (x0, y0, x1, y1) in [
                (-8, -7, 8, -7),
                (8, -7, 8, 5),
                (8, 5, -2, 5),
                (-2, 5, -7, 9),
                (-7, 9, -6, 5),
                (-6, 5, -8, 3),
                (-8, 3, -8, -7),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            for dot_x in [-4, 0, 4] {
                canvas.aa_circle(ox(dot_x), oy(-1), stroke, color);
            }
        }
        Icon::Calculator => {
            for (x0, y0, x1, y1) in [
                (-9, -10, 9, -10),
                (9, -10, 9, 10),
                (9, 10, -9, 10),
                (-9, 10, -9, -10),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            canvas.round_line(ox(-6), oy(-5), ox(6), oy(-5), stroke, color);
            for (dot_x, dot_y) in [(-5, 0), (1, 0), (-5, 5), (1, 5), (6, 0), (6, 5)] {
                canvas.aa_circle(ox(dot_x), oy(dot_y), stroke, color);
            }
        }
        Icon::Settings => {
            canvas.ring(x, y, icon_offset(8, size_px), stroke, color);
            canvas.ring(x, y, icon_offset(3, size_px).max(2), stroke, color);
            for (dx, dy) in [
                (0, -10),
                (7, -7),
                (10, 0),
                (7, 7),
                (0, 10),
                (-7, 7),
                (-10, 0),
                (-7, -7),
            ] {
                canvas.round_line(
                    ox(dx * 3 / 4),
                    oy(dy * 3 / 4),
                    ox(dx),
                    oy(dy),
                    stroke,
                    color,
                );
            }
        }
        Icon::DroidCode => {
            canvas.ring(x, oy(1), icon_offset(9, size_px), stroke, color);
            canvas.round_line(ox(-6), oy(-7), ox(-9), oy(-11), stroke, color);
            canvas.round_line(ox(6), oy(-7), ox(9), oy(-11), stroke, color);
            canvas.aa_circle(ox(-4), oy(-2), stroke, color);
            canvas.aa_circle(ox(4), oy(-2), stroke, color);
            canvas.round_line(ox(-5), oy(4), ox(5), oy(4), stroke, color);
        }
        Icon::Moon => {
            canvas.aa_crescent(x, y, size_px, color);
        }
        Icon::Palette => {
            canvas.ring(x, y, icon_offset(9, size_px), stroke, color);
            for (dot_x, dot_y) in [(-4, -3), (1, -5), (5, -1), (-2, 4)] {
                canvas.aa_circle(ox(dot_x), oy(dot_y), stroke, color);
            }
        }
        Icon::Sun => {
            canvas.ring(x, y, icon_offset(5, size_px), stroke, color);
            for (x0, y0, x1, y1) in [
                (0, -11, 0, -8),
                (8, -8, 6, -6),
                (11, 0, 8, 0),
                (8, 8, 6, 6),
                (0, 11, 0, 8),
                (-8, 8, -6, 6),
                (-11, 0, -8, 0),
                (-8, -8, -6, -6),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
        }
        Icon::Accessibility => {
            canvas.aa_circle(x, oy(-8), stroke + 1, color);
            canvas.round_line(x, oy(-4), x, oy(4), stroke, color);
            canvas.round_line(ox(-9), oy(-2), ox(9), oy(-2), stroke, color);
            canvas.round_line(x, oy(4), ox(-7), oy(10), stroke, color);
            canvas.round_line(x, oy(4), ox(7), oy(10), stroke, color);
        }
        Icon::TextSize => {
            canvas.round_line(ox(-10), oy(8), ox(-4), oy(-9), stroke, color);
            canvas.round_line(ox(-4), oy(-9), ox(2), oy(8), stroke, color);
            canvas.round_line(ox(-8), oy(2), ox(0), oy(2), stroke, color);
            canvas.round_line(ox(4), oy(8), ox(7), oy(-2), stroke, color);
            canvas.round_line(ox(7), oy(-2), ox(10), oy(8), stroke, color);
            canvas.round_line(ox(5), oy(4), ox(9), oy(4), stroke, color);
        }
        Icon::Contrast => {
            canvas.ring(x, y, icon_offset(10, size_px), stroke, color);
            canvas.round_line(x, oy(-9), x, oy(9), stroke, color);
            for dot_y in [-5, 0, 5] {
                canvas.aa_circle(ox(-4), oy(dot_y), stroke + 1, color);
            }
        }
        Icon::Wifi => {
            for (x0, y0, x1, y1) in [
                (-10, -3, -5, -7),
                (-5, -7, 0, -9),
                (0, -9, 5, -7),
                (5, -7, 10, -3),
                (-6, 2, -3, -1),
                (-3, -1, 0, -2),
                (0, -2, 3, -1),
                (3, -1, 6, 2),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            canvas.aa_circle(x, oy(7), stroke + 1, color);
        }
        Icon::Airplane => {
            for (x0, y0, x1, y1) in [
                (-10, 3, 10, -7),
                (10, -7, 3, 9),
                (2, -1, -7, -7),
                (-2, 2, -5, 9),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
        }
        Icon::Info => {
            canvas.ring(x, y, icon_offset(10, size_px), stroke, color);
            canvas.aa_circle(x, oy(-5), stroke, color);
            canvas.round_line(x, oy(-1), x, oy(6), stroke, color);
        }
        Icon::Backspace => {
            for (x0, y0, x1, y1) in [
                (-10, 0, -5, -7),
                (-5, -7, 9, -7),
                (9, -7, 9, 7),
                (9, 7, -5, 7),
                (-5, 7, -10, 0),
                (-1, -3, 5, 3),
                (5, -3, -1, 3),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
        }
        Icon::Calendar => {
            for (x0, y0, x1, y1) in [
                (-9, -7, 9, -7),
                (9, -7, 9, 9),
                (9, 9, -9, 9),
                (-9, 9, -9, -7),
                (-9, -2, 9, -2),
                (-5, -10, -5, -5),
                (5, -10, 5, -5),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            for (dot_x, dot_y) in [(-5, 2), (0, 2), (5, 2), (-5, 6), (0, 6)] {
                canvas.aa_circle(ox(dot_x), oy(dot_y), stroke, color);
            }
        }
        Icon::Bell => {
            for (x0, y0, x1, y1) in [
                (-8, 5, -5, 2),
                (-5, 2, -5, -3),
                (-5, -3, -2, -8),
                (-2, -8, 2, -8),
                (2, -8, 5, -3),
                (5, -3, 5, 2),
                (5, 2, 8, 5),
                (8, 5, -8, 5),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            canvas.round_line(ox(-2), oy(9), ox(2), oy(9), stroke, color);
        }
        Icon::Lock => {
            canvas.round_line(ox(-6), oy(-1), ox(-6), oy(-5), stroke, color);
            canvas.round_line(ox(-6), oy(-5), ox(-3), oy(-9), stroke, color);
            canvas.round_line(ox(-3), oy(-9), ox(3), oy(-9), stroke, color);
            canvas.round_line(ox(3), oy(-9), ox(6), oy(-5), stroke, color);
            canvas.round_line(ox(6), oy(-5), ox(6), oy(-1), stroke, color);
            for (x0, y0, x1, y1) in [
                (-8, -1, 8, -1),
                (8, -1, 8, 9),
                (8, 9, -8, 9),
                (-8, 9, -8, -1),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
            canvas.aa_circle(x, oy(4), stroke, color);
        }
        Icon::BatteryUnknown => {
            for (x0, y0, x1, y1) in [
                (-10, -6, 8, -6),
                (8, -6, 8, 6),
                (8, 6, -10, 6),
                (-10, 6, -10, -6),
                (10, -2, 10, 2),
                (-4, 0, 3, 0),
            ] {
                canvas.round_line(ox(x0), oy(y0), ox(x1), oy(y1), stroke, color);
            }
        }
        Icon::ChevronLeft => {
            canvas.round_line(ox(4), oy(-8), ox(-4), y, stroke, color);
            canvas.round_line(ox(-4), y, ox(4), oy(8), stroke, color);
        }
        Icon::ChevronRight => {
            canvas.round_line(ox(-4), oy(-8), ox(4), y, stroke, color);
            canvas.round_line(ox(4), y, ox(-4), oy(8), stroke, color);
        }
        Icon::ChevronUp => {
            canvas.round_line(ox(-8), oy(4), x, oy(-4), stroke, color);
            canvas.round_line(x, oy(-4), ox(8), oy(4), stroke, color);
        }
        Icon::ChevronDown => {
            canvas.round_line(ox(-8), oy(-4), x, oy(4), stroke, color);
            canvas.round_line(x, oy(4), ox(8), oy(-4), stroke, color);
        }
    }
}

fn draw_app_icon(canvas: &mut Canvas<'_>, x: i32, y: i32, color: u32, icon: Icon) {
    canvas.rounded_rect(
        DRect::new(x - 25, y - 25, 50, 50),
        MobileShapeTokens::RADIUS_APP_ICON,
        color,
    );
    let foreground = accessible_surface_ink(color);
    draw_icon(canvas, x, y, IconSize::App, foreground, icon);
}

/// Draws an honest launcher fallback when the admitted package snapshot has
/// no decoded Android icon resource. The monogram and color are derived only
/// from immutable package metadata, so two packages remain visually distinct
/// without pretending that Bndroid decoded an icon it does not possess.
fn installed_app_fallback_color(installed: AndroidInstalledAppStatus) -> u32 {
    const FALLBACK_COLORS: [u32; 6] = [
        0x000b_57d0,
        0x008e_24aa,
        0x0000_6c4c,
        0x00b3_261e,
        0x007a_4f01,
        0x0040_51b5,
    ];
    let palette_index = usize::from(
        installed.apk_digest_sha256[0]
            ^ installed.apk_digest_sha256[3]
            ^ installed.apk_digest_sha256[15],
    ) % FALLBACK_COLORS.len();
    FALLBACK_COLORS[palette_index]
}

fn draw_installed_app_icon(
    canvas: &mut Canvas<'_>,
    x: i32,
    y: i32,
    installed: AndroidInstalledAppStatus,
) {
    #[cfg(feature = "androidbox-icon-resources5")]
    if let Some(icon) = installed.icon {
        for source_y in 0..ANDROID_INSTALLED_ICON_HEIGHT {
            for source_x in 0..ANDROID_INSTALLED_ICON_WIDTH {
                let pixel = icon
                    .pixel(source_y * ANDROID_INSTALLED_ICON_WIDTH + source_x)
                    .expect("admitted installed icon has canonical palette indices");
                let alpha = pixel >> 24;
                if alpha == 0 {
                    continue;
                }
                canvas.blend_fill_rect(
                    DRect::new(
                        x - 24 + source_x as i32 * 3,
                        y - 24 + source_y as i32 * 3,
                        3,
                        3,
                    ),
                    pixel & 0x00ff_ffff,
                    alpha,
                );
            }
        }
        return;
    }

    let letter = installed
        .title
        .as_str()
        .bytes()
        .find(u8::is_ascii_alphanumeric)
        .unwrap_or(b'?')
        .to_ascii_uppercase();
    let monogram = [letter];
    let monogram = core::str::from_utf8(&monogram)
        .expect("installed-title admission and fallback preserve ASCII");

    canvas.rounded_rect(
        DRect::new(x - 25, y - 25, 50, 50),
        MobileShapeTokens::RADIUS_APP_ICON,
        installed_app_fallback_color(installed),
    );
    canvas.text_center(
        x,
        y - 10,
        monogram,
        MobileTextRole::Title,
        accessible_surface_ink(installed_app_fallback_color(installed)),
    );
}

fn draw_setting_icon(canvas: &mut Canvas<'_>, x: i32, y: i32, color: u32, icon: Icon) {
    canvas.rounded_rect(
        DRect::new(x - 18, y - 18, 36, 36),
        MobileShapeTokens::RADIUS_SETTING_ICON,
        color,
    );
    let foreground = accessible_surface_ink(color);
    draw_icon(canvas, x, y, IconSize::List, foreground, icon);
}

#[cfg(feature = "androidbox-runtime-uninstall1")]
const fn text_color(dark: bool) -> u32 {
    mobile_theme_tokens(dark, false, false).text_primary
}

fn pressed_surface_color(model: MobileModel) -> u32 {
    pressed_surface_color_ref(&model)
}

fn pressed_surface_color_ref(model: &MobileModel) -> u32 {
    model.theme_tokens().accent_container
}

const fn component(color: u32, shift: u32) -> u32 {
    (color >> shift) & 0xff
}

const fn mix(first: u32, second: u32, numerator: u32, denominator: u32) -> u32 {
    let inverse = denominator - numerator;
    let red = (component(first, 16) * inverse + component(second, 16) * numerator) / denominator;
    let green = (component(first, 8) * inverse + component(second, 8) * numerator) / denominator;
    let blue = (component(first, 0) * inverse + component(second, 0) * numerator) / denominator;
    (red << 16) | (green << 8) | blue
}

const fn blend(base: u32, overlay: u32, alpha: u32) -> u32 {
    let inverse = 255 - alpha;
    let red = (component(base, 16) * inverse + component(overlay, 16) * alpha) / 255;
    let green = (component(base, 8) * inverse + component(overlay, 8) * alpha) / 255;
    let blue = (component(base, 0) * inverse + component(overlay, 0) * alpha) / 255;
    (red << 16) | (green << 8) | blue
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "androidbox-scene-rpc2")]
    use crate::android_scene::{AndroidSceneLayoutSize, AndroidSceneOrientation};
    extern crate std;
    use std::vec;
    use std::vec::Vec;

    fn render_model(model: MobileModel) -> Vec<u32> {
        let mut pixels = vec![0_u32; PIXEL_COUNT];
        render(&mut pixels, model).unwrap();
        pixels
    }

    #[cfg(feature = "androidbox-multipackage4")]
    fn assert_planned_damage_is_pixel_exact(
        previous: MobileModel,
        current: MobileModel,
        expected: DamageRect,
    ) {
        let expected_regions = DamageRegions::single(expected).unwrap();
        assert_eq!(
            damage_plan(Some(previous), current),
            MobileDamagePlan::Regions(expected_regions)
        );
        let previous_pixels = render_model(previous);
        let current_pixels = render_model(current);
        assert_ne!(previous_pixels, current_pixels);
        let mut damaged = previous_pixels;
        render_damage(&mut damaged, current, expected).unwrap();
        if let Some((index, (actual, wanted))) = damaged
            .iter()
            .zip(current_pixels.iter())
            .enumerate()
            .find(|(_, (actual, wanted))| actual != wanted)
        {
            panic!(
                "damage {expected:?} missed pixel {}/{}, actual={actual:#010x}, wanted={wanted:#010x}",
                index % WIDTH,
                index / WIDTH,
            );
        }
    }

    const fn verified_androidbox_resources() -> AndroidBoxResourceStatus {
        AndroidBoxResourceStatus {
            resource_table_parsed: true,
            layout_entry_resolved: true,
            binary_xml_parsed: true,
            text_view_verified: true,
            string_reference_resolved: true,
            layout_resource_id: ANDROIDBOX_RESOURCE_LAYOUT_ID,
            string_resource_id: ANDROIDBOX_RESOURCE_STRING_ID,
        }
    }

    fn installed_android_app(generation: u64) -> AndroidInstalledAppStatus {
        installed_android_app_with_text(generation, "AndroidBox resource-backed view")
    }

    fn installed_android_app_with_text(generation: u64, text: &str) -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            generation,
            7,
            12_566,
            "org.bndroid.demo",
            "org.bndroid.demo.MainActivity",
            "AndroidBox",
            text,
            [0x5a; 32],
            [0xa5; 32],
        )
        .unwrap()
    }

    fn compatible_overview_model(catalog: AndroidInstalledAppStatus) -> MobileModel {
        let request_sequence = 41;
        let identity =
            UiCompatibleActivityIdentity::new(request_sequence, catalog.generation).unwrap();
        let mut model = MobileModel {
            android_installed_app: catalog,
            ..MobileModel::for_page(MobilePage::Home)
        };
        assert!(model.begin_android_installed_launch(request_sequence, catalog.generation));
        assert!(model.succeed_android_installed_launch(
            request_sequence,
            catalog.generation,
            catalog.apk_digest_sha256,
        ));
        assert!(model.bind_android_installed_activity_session(identity));
        assert!(model.apply_system_ui_state_recent(
            UiSystemUiMode::Overview,
            Some(UiRecentIdentity::CompatibleAndroid(identity)),
            false,
            0,
            2,
        ));
        assert!(model.compatible_android_recent_ready());
        model
    }

    #[cfg(feature = "androidbox-multipackage4")]
    fn installed_android_app_named(
        generation: u64,
        package: &str,
        activity: &str,
        title: &str,
        apk_digest: u8,
    ) -> AndroidInstalledAppStatus {
        AndroidInstalledAppStatus::try_new(
            generation,
            generation as u32,
            12_000 + generation as u32,
            package,
            activity,
            title,
            "Android package content",
            [0x5a; 32],
            [apk_digest; 32],
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    fn android_install_candidate(
        action: AndroidInstallCandidateAction,
        expected_generation: u64,
        expected_version_code: u32,
        version_code: u32,
        apk_digest_sha256: [u8; 32],
    ) -> AndroidInstallCandidateStatus {
        AndroidInstallCandidateStatus::try_new(
            0x53,
            action,
            expected_generation,
            expected_version_code,
            version_code,
            12_566,
            "org.bndroid.demo",
            "org.bndroid.demo.MainActivity",
            "AndroidBox",
            "AndroidBox resource-backed view",
            [0x5a; 32],
            apk_digest_sha256,
        )
        .unwrap()
    }

    #[cfg(feature = "androidbox-interactive0")]
    fn installed_android_foreground_model() -> MobileModel {
        let catalog = installed_android_app_with_text(7, "Legacy publisher text");
        let identity = UiCompatibleActivityIdentity::new(61, 7).unwrap();
        let mut model = MobileModel {
            android_installed_app: catalog,
            ..MobileModel::for_page(MobilePage::AndroidDemo)
        };
        assert!(model.begin_android_installed_launch(61, 7));
        assert!(model.succeed_android_installed_launch(61, 7, [0xa5; 32]));
        assert!(model.bind_android_installed_activity_session(identity));
        assert!(model.apply_system_ui_state_recent(
            UiSystemUiMode::Foreground,
            Some(UiRecentIdentity::CompatibleAndroid(identity)),
            false,
            0,
            2,
        ));
        assert!(model.installed_android_foreground_content_ready());
        model
    }

    #[cfg(feature = "androidbox-interactive0")]
    fn interactive_installed_model(label: &str, revision: u64) -> MobileModel {
        let mut model = installed_android_foreground_model();
        assert!(model.set_android_installed_activity_view(
            0x7f01_0001,
            label,
            0x7f01_0002,
            "Tap me",
            true,
            revision,
        ));
        assert!(model.installed_android_interactive_content_ready());
        model
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn installed_android_scene_nodes() -> [AndroidInstalledActivitySceneNode; 8] {
        [
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::LinearLayout,
                None,
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneOrientation::Vertical,
                "",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(0),
                0x7f01_0001,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Account profile",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::Button,
                Some(0),
                0x7f01_0002,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Verify profile",
                true,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::LinearLayout,
                Some(0),
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::Vertical,
                "",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(3),
                0x7f01_0003,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Profile status: pending",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::Button,
                Some(3),
                0x7f01_0004,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Details unavailable",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(0),
                0x7f01_0005,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Local scene",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(3),
                0x7f01_0006,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "No network access",
                false,
            )
            .unwrap(),
        ]
    }

    #[cfg(feature = "androidbox-layout-row14")]
    fn installed_android_row_scene_nodes() -> [AndroidInstalledActivitySceneNode; 6] {
        [
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::LinearLayout,
                None,
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneOrientation::Vertical,
                "",
                false,
            )
            .unwrap(),
            {
                #[cfg(feature = "androidbox-layout-size18")]
                {
                    AndroidInstalledActivitySceneNode::try_new_sized(
                        AndroidSceneViewKind::TextView,
                        Some(0),
                        0x7f02_0003,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Android components",
                        false,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        240,
                        0,
                    )
                    .unwrap()
                }
                #[cfg(not(feature = "androidbox-layout-size18"))]
                {
                    AndroidInstalledActivitySceneNode::try_new(
                        AndroidSceneViewKind::TextView,
                        Some(0),
                        0x7f02_0003,
                        AndroidSceneLayoutSize::MatchParent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Android components",
                        false,
                    )
                    .unwrap()
                }
            },
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(0),
                0x7f02_0002,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Catalog ready",
                false,
            )
            .unwrap(),
            {
                #[cfg(feature = "androidbox-layout-size18")]
                {
                    AndroidInstalledActivitySceneNode::try_new_sized(
                        AndroidSceneViewKind::LinearLayout,
                        Some(0),
                        0,
                        AndroidSceneLayoutSize::MatchParent,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneOrientation::Horizontal,
                        "",
                        false,
                        0,
                        0,
                        0,
                        0,
                        0,
                        6,
                        4,
                        2,
                        8,
                        0,
                        120,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-directional17",
                    not(feature = "androidbox-layout-size18")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_directional(
                        AndroidSceneViewKind::LinearLayout,
                        Some(0),
                        0,
                        AndroidSceneLayoutSize::MatchParent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::Horizontal,
                        "",
                        false,
                        0,
                        0,
                        0,
                        0,
                        0,
                        6,
                        4,
                        2,
                        8,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-spacing16",
                    not(feature = "androidbox-layout-directional17")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_spaced(
                        AndroidSceneViewKind::LinearLayout,
                        Some(0),
                        0,
                        AndroidSceneLayoutSize::MatchParent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::Horizontal,
                        "",
                        false,
                        0,
                        0,
                        4,
                    )
                    .unwrap()
                }
                #[cfg(not(feature = "androidbox-layout-spacing16"))]
                {
                    AndroidInstalledActivitySceneNode::try_new(
                        AndroidSceneViewKind::LinearLayout,
                        Some(0),
                        0,
                        AndroidSceneLayoutSize::MatchParent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::Horizontal,
                        "",
                        false,
                    )
                    .unwrap()
                }
            },
            {
                #[cfg(feature = "androidbox-layout-mixed19")]
                {
                    AndroidInstalledActivitySceneNode::try_new_sized(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                        0,
                        2,
                        1,
                        4,
                        3,
                        0,
                        0,
                        0,
                        0,
                        132,
                        64,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-size18",
                    not(feature = "androidbox-layout-mixed19")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_sized(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                        1,
                        2,
                        1,
                        4,
                        3,
                        0,
                        0,
                        0,
                        0,
                        0,
                        64,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-directional17",
                    not(feature = "androidbox-layout-size18")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_directional(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                        1,
                        2,
                        1,
                        4,
                        3,
                        0,
                        0,
                        0,
                        0,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-spacing16",
                    not(feature = "androidbox-layout-directional17")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_spaced(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                        1,
                        2,
                        0,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-weight15",
                    not(feature = "androidbox-layout-spacing16")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_weighted(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                        1,
                    )
                    .unwrap()
                }
                #[cfg(not(feature = "androidbox-layout-weight15"))]
                {
                    AndroidInstalledActivitySceneNode::try_new(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0000,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show components",
                        true,
                    )
                    .unwrap()
                }
            },
            {
                #[cfg(feature = "androidbox-layout-size18")]
                {
                    AndroidInstalledActivitySceneNode::try_new_sized(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0001,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::Exact,
                        AndroidSceneOrientation::None,
                        "Show permissions",
                        true,
                        1,
                        6,
                        5,
                        2,
                        1,
                        0,
                        0,
                        0,
                        0,
                        0,
                        56,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-directional17",
                    not(feature = "androidbox-layout-size18")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_directional(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0001,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show permissions",
                        true,
                        1,
                        6,
                        5,
                        2,
                        1,
                        0,
                        0,
                        0,
                        0,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-spacing16",
                    not(feature = "androidbox-layout-directional17")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_spaced(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0001,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show permissions",
                        true,
                        1,
                        2,
                        0,
                    )
                    .unwrap()
                }
                #[cfg(all(
                    feature = "androidbox-layout-weight15",
                    not(feature = "androidbox-layout-spacing16")
                ))]
                {
                    AndroidInstalledActivitySceneNode::try_new_weighted(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0001,
                        AndroidSceneLayoutSize::Zero,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show permissions",
                        true,
                        1,
                    )
                    .unwrap()
                }
                #[cfg(not(feature = "androidbox-layout-weight15"))]
                {
                    AndroidInstalledActivitySceneNode::try_new(
                        AndroidSceneViewKind::Button,
                        Some(3),
                        0x7f02_0001,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneLayoutSize::WrapContent,
                        AndroidSceneOrientation::None,
                        "Show permissions",
                        true,
                    )
                    .unwrap()
                }
            },
        ]
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    fn scene_installed_model(node_count: usize, revision: u64, legacy_view: bool) -> MobileModel {
        assert!((3..=8).contains(&node_count));
        let mut model = installed_android_foreground_model();
        if legacy_view {
            assert!(model.set_android_installed_activity_view(
                0x7f02_0001,
                "Legacy fallback",
                0x7f02_0002,
                "Legacy tap",
                true,
                1,
            ));
        }
        let nodes = installed_android_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes[..node_count], revision));
        assert!(model.android_installed_activity_scene.is_active());
        model
    }

    fn render_text_sample(role: MobileTextRole, text: &str) -> Vec<u32> {
        let row_count = usize::from(role.line_height_px()) + 16;
        let mut pixels = vec![COLOR_BLACK; WIDTH * row_count];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            first_row: 0,
            first_column: 0,
            row_stride: WIDTH,
            row_count,
            alternate_accent: false,
            large_text: false,
            high_contrast: false,
            design_offset_y_px: 0,
            clip_left_x_px: 0,
            clip_right_x_px: WIDTH as i32,
            clip_top_y_px: 0,
            clip_bottom_y_px: row_count as i32,
        };
        canvas.text_role(8, 4, text, role, COLOR_WHITE);
        pixels
    }

    fn render_icon_sample(icon: Icon, size: IconSize, color: u32) -> Vec<u32> {
        let row_count = 96;
        let mut pixels = vec![COLOR_BLACK; WIDTH * row_count];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            first_row: 0,
            first_column: 0,
            row_stride: WIDTH,
            row_count,
            alternate_accent: false,
            large_text: false,
            high_contrast: false,
            design_offset_y_px: 0,
            clip_left_x_px: 0,
            clip_right_x_px: WIDTH as i32,
            clip_top_y_px: 0,
            clip_bottom_y_px: row_count as i32,
        };
        draw_icon(&mut canvas, 40, 24, size, color, icon);
        pixels
    }

    fn digest(pixels: &[u32]) -> u64 {
        let mut digest = 0xcbf2_9ce4_8422_2325_u64;
        for pixel in pixels {
            for byte in pixel.to_le_bytes() {
                digest ^= u64::from(byte);
                digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        digest
    }

    fn pixel_in_rect(x: usize, y: usize, rect: (usize, usize, usize, usize)) -> bool {
        x >= rect.0 && x < rect.2 && y >= rect.1 && y < rect.3
    }

    fn assert_pixel_differences_are_bounded(
        before: &[u32],
        after: &[u32],
        allowed: &[(usize, usize, usize, usize)],
    ) {
        let mut changed = 0;
        for (index, (before, after)) in before.iter().zip(after).enumerate() {
            if before == after {
                continue;
            }
            changed += 1;
            let x = index % WIDTH;
            let y = index / WIDTH;
            assert!(
                allowed.iter().any(|rect| pixel_in_rect(x, y, *rect)),
                "time snapshot changed unexpected pixel ({x}, {y})"
            );
        }
        assert!(changed > 0, "time snapshot did not change any pixels");
    }

    #[test]
    fn phone_geometry_is_exact_twenty_by_nine() {
        assert_eq!((WIDTH, HEIGHT), (720, 1_600));
        assert_eq!((DESIGN_WIDTH, DESIGN_HEIGHT), (360, 800));
        assert_eq!(PIXEL_COUNT, 1_152_000);
        assert_eq!(WIDTH * 20, HEIGHT * 9);
        assert_eq!(SCREEN_CORNER_RADIUS_PX, 64);
    }

    #[test]
    fn semantic_theme_tokens_are_complete_and_preserve_visual_hierarchy() {
        let luma = |color: u32| {
            component(color, 16) * 2_126 + component(color, 8) * 7_152 + component(color, 0) * 722
        };
        let dark = mobile_theme_tokens(true, false, false);
        let light = mobile_theme_tokens(false, false, false);
        let alternate_dark = mobile_theme_tokens(true, true, false);
        let alternate_light = mobile_theme_tokens(false, true, false);
        let dark_model = MobileModel::default();
        let light_model = MobileModel {
            dark_theme: false,
            ..MobileModel::default()
        };

        assert_eq!(dark.text_primary, COLOR_TEXT_DARK);
        assert_eq!(dark.text_secondary, COLOR_TEXT_MUTED_DARK);
        assert_eq!(dark.text_tertiary, COLOR_TEXT_WEAK_DARK);
        assert_eq!(dark.surface, dark_model.surface_color(false));
        assert_eq!(dark.surface_raised, dark_model.surface_color(true));
        assert_eq!(dark.outline, dark_model.outline_color());
        assert!(luma(dark.background_top) < luma(dark.surface));
        assert!(luma(dark.surface) < luma(dark.surface_raised));
        assert!(luma(dark.surface_raised) < luma(dark.text_secondary));
        assert!(luma(dark.text_secondary) < luma(dark.text_primary));

        assert_eq!(light.text_primary, COLOR_TEXT_LIGHT);
        assert_eq!(light.text_secondary, COLOR_TEXT_MUTED_LIGHT);
        assert_eq!(light.text_tertiary, COLOR_TEXT_WEAK_LIGHT);
        assert_eq!(light.surface, light_model.surface_color(false));
        assert_eq!(light.surface_raised, light_model.surface_color(true));
        assert_eq!(light.outline, light_model.outline_color());
        assert!(luma(light.text_primary) < luma(light.text_secondary));
        assert!(luma(light.text_secondary) < luma(light.outline));
        assert!(luma(light.outline) < luma(light.surface_raised));
        assert!(luma(light.surface_raised) < luma(light.background_bottom));
        assert!(luma(light.background_bottom) < luma(light.surface));

        for (base, alternate) in [(dark, alternate_dark), (light, alternate_light)] {
            assert_ne!(base.accent, alternate.accent);
            assert_ne!(base.background_top, alternate.background_top);
            assert_ne!(base.background_bottom, alternate.background_bottom);
            assert_ne!(base.surface, alternate.surface);
            assert_ne!(base.surface_raised, alternate.surface_raised);
            assert_ne!(base.panel, alternate.panel);
            assert_ne!(base.outline, alternate.outline);
            assert_ne!(base.accent_container, alternate.accent_container);
            assert_ne!(
                base.wallpaper_glow_primary,
                alternate.wallpaper_glow_primary
            );
            assert_eq!(base.on_accent, COLOR_TEXT_LIGHT);
            assert_eq!(alternate.on_accent, COLOR_TEXT_LIGHT);
        }
    }

    #[test]
    fn user_accent_recolors_the_complete_phone_scheme_without_moving_targets() {
        let base = MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_318_060),
            ..MobileModel::for_page(MobilePage::Home)
        };
        let alternate = MobileModel {
            alternate_accent: true,
            ..base
        };
        let base_pixels = render_model(base);
        let alternate_pixels = render_model(alternate);

        assert_ne!(digest(&base_pixels), digest(&alternate_pixels));
        assert_eq!(alternate_pixels, render_model(alternate));
        let changed = base_pixels
            .iter()
            .zip(&alternate_pixels)
            .filter(|(left, right)| left != right)
            .count();
        assert!(changed > PIXEL_COUNT / 2, "only {changed} pixels recolored");

        // Wallpaper, raised Home surface, and trusted bottom chrome all take
        // part in the same scheme. The physical hit targets remain constants.
        for (x, y) in [(360, 1_000), (600, 380), (360, 1_550)] {
            assert_ne!(
                base_pixels[y * WIDTH + x],
                alternate_pixels[y * WIDTH + x],
                "scheme did not reach ({x}, {y})"
            );
        }
        assert_eq!(base_pixels[0], COLOR_BLACK);
        assert_eq!(alternate_pixels[0], COLOR_BLACK);
        for (x, y) in [(615, 1_435), (105, 1_435), (360, 1_300)] {
            assert_eq!(hit_test(base, x, y), hit_test(alternate, x, y));
        }

        #[cfg(feature = "mobile-ui-runtime")]
        assert_eq!(damage_plan(Some(base), alternate), MobileDamagePlan::Full);
    }

    #[test]
    fn elevated_surface_never_writes_outside_its_authoritative_control_rect() {
        const ROWS: usize = 200;
        const SENTINEL: u32 = 0x0012_3456;
        let rect = DRect::new(20, 20, 80, 48);
        let physical = (40usize, 40usize, 200usize, 136usize);

        let render_surface = |pressed| {
            let mut pixels = vec![SENTINEL; WIDTH * ROWS];
            let mut canvas = Canvas {
                pixels: &mut pixels,
                first_row: 0,
                first_column: 0,
                row_stride: WIDTH,
                row_count: ROWS,
                alternate_accent: false,
                large_text: false,
                high_contrast: false,
                design_offset_y_px: 0,
                clip_left_x_px: 0,
                clip_right_x_px: WIDTH as i32,
                clip_top_y_px: 0,
                clip_bottom_y_px: ROWS as i32,
            };
            canvas.elevated_surface(
                rect,
                MobileShapeTokens::RADIUS_CONTROL,
                COLOR_BLUE,
                true,
                pressed,
            );
            pixels
        };

        let stable = render_surface(false);
        let pressed = render_surface(true);
        let mut stable_writes = 0;
        let mut pressed_differences = 0;
        for (index, (stable_pixel, pressed_pixel)) in stable.iter().zip(&pressed).enumerate() {
            let x = index % WIDTH;
            let y = index / WIDTH;
            let inside = pixel_in_rect(x, y, physical);
            if *stable_pixel != SENTINEL {
                stable_writes += 1;
                assert!(inside, "stable elevation escaped at {x}/{y}");
            }
            if stable_pixel != pressed_pixel {
                pressed_differences += 1;
                assert!(inside, "pressed elevation escaped at {x}/{y}");
            }
        }
        assert!(stable_writes > 10_000);
        assert!(pressed_differences > 1_000);
    }

    #[test]
    fn unix_seconds_convert_to_valid_gregorian_snapshots() {
        for (unix_seconds, expected) in [
            (0, (1970, 1, 1, 4, 0, 0)),
            (951_782_400, (2000, 2, 29, 2, 0, 0)),
            (1_709_164_800, (2024, 2, 29, 4, 0, 0)),
            (1_785_318_060, (2026, 7, 29, 3, 9, 41)),
            (4_107_542_400, (2100, 3, 1, 1, 0, 0)),
            (MAX_SUPPORTED_UNIX_SECONDS, (9999, 12, 31, 5, 23, 59)),
        ] {
            let snapshot = MobileTimeSnapshot::from_unix_seconds(unix_seconds);
            assert!(snapshot.is_available());
            assert_eq!(
                (
                    snapshot.year,
                    snapshot.month,
                    snapshot.day,
                    snapshot.weekday,
                    snapshot.hour,
                    snapshot.minute,
                ),
                expected
            );
        }

        let unavailable =
            MobileTimeSnapshot::from_unix_seconds(MAX_SUPPORTED_UNIX_SECONDS.saturating_add(1));
        assert_eq!(unavailable, MobileTimeSnapshot::unavailable());
    }

    #[test]
    fn mobile_time_formats_minute_day_leap_and_year_boundaries() {
        let canonical = MobileTimeSnapshot::from_unix_seconds(1_785_318_060);
        assert_eq!(canonical.time_text().as_str(), "09:41");
        assert_eq!(canonical.short_date_text().as_str(), "Wed, Jul 29");
        assert_eq!(canonical.long_date_text().as_str(), "Wednesday, July 29");

        assert_eq!(
            MobileTimeSnapshot::from_unix_seconds(1_785_318_119)
                .time_text()
                .as_str(),
            "09:41"
        );
        assert_eq!(
            MobileTimeSnapshot::from_unix_seconds(1_785_318_120)
                .time_text()
                .as_str(),
            "09:42"
        );

        let before_midnight = MobileTimeSnapshot::from_unix_seconds(1_785_369_599);
        let after_midnight = MobileTimeSnapshot::from_unix_seconds(1_785_369_600);
        assert_eq!(before_midnight.time_text().as_str(), "23:59");
        assert_eq!(
            before_midnight.long_date_text().as_str(),
            "Wednesday, July 29"
        );
        assert_eq!(after_midnight.time_text().as_str(), "00:00");
        assert_eq!(
            after_midnight.long_date_text().as_str(),
            "Thursday, July 30"
        );

        let leap_day = MobileTimeSnapshot::from_unix_seconds(1_709_164_800);
        assert_eq!(leap_day.long_date_text().as_str(), "Thursday, February 29");
        let year_end = MobileTimeSnapshot::from_unix_seconds(4_102_444_799);
        assert_eq!(year_end.long_date_text().as_str(), "Thursday, December 31");
    }

    #[test]
    fn unavailable_time_never_falls_back_to_a_demonstration_date() {
        let unavailable = MobileTimeSnapshot::unavailable();
        assert!(!unavailable.is_available());
        assert_eq!(unavailable.time_text().as_str(), "--:--");
        assert_eq!(unavailable.short_date_text().as_str(), "Time unavailable");
        assert_eq!(unavailable.long_date_text().as_str(), "Time unavailable");

        let pixels = render_model(MobileModel::default());
        let canonical = render_model(MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_318_060),
            ..MobileModel::default()
        });
        assert_ne!(pixels, canonical);
    }

    #[test]
    fn time_changes_are_confined_to_declared_phone_ui_regions() {
        const STATUS_TIME: (usize, usize, usize, usize) = (32, 12, 232, 64);
        const HOME_TIME: (usize, usize, usize, usize) = (32, 120, 332, 272);
        const HOME_TODAY: (usize, usize, usize, usize) = (36, 328, 684, 488);
        const LOCK_TIME_DATE: (usize, usize, usize, usize) = (96, 232, 624, 444);
        const QUICK_TIME_DATE: (usize, usize, usize, usize) = (32, 96, 360, 224);

        let first = MobileTimeSnapshot::from_unix_seconds(1_785_318_060);
        let next_minute = MobileTimeSnapshot::from_unix_seconds(1_785_318_120);
        let next_day = MobileTimeSnapshot::from_unix_seconds(1_785_369_600);

        for (page, allowed) in [
            (MobilePage::Settings, &[STATUS_TIME][..]),
            (MobilePage::Display, &[STATUS_TIME][..]),
            (MobilePage::Accessibility, &[STATUS_TIME][..]),
            (MobilePage::Home, &[STATUS_TIME, HOME_TIME][..]),
            (MobilePage::Lock, &[STATUS_TIME, LOCK_TIME_DATE][..]),
        ] {
            let before = render_model(MobileModel {
                time: first,
                ..MobileModel::for_page(page)
            });
            let after = render_model(MobileModel {
                time: next_minute,
                ..MobileModel::for_page(page)
            });
            assert_pixel_differences_are_bounded(&before, &after, allowed);
        }

        let quick_before = render_model(MobileModel {
            time: first,
            shade_open: true,
            ..MobileModel::for_page(MobilePage::Settings)
        });
        let quick_after = render_model(MobileModel {
            time: next_minute,
            shade_open: true,
            ..MobileModel::for_page(MobilePage::Settings)
        });
        assert_pixel_differences_are_bounded(
            &quick_before,
            &quick_after,
            &[STATUS_TIME, QUICK_TIME_DATE],
        );

        let home_before = render_model(MobileModel {
            time: first,
            ..MobileModel::for_page(MobilePage::Home)
        });
        let home_next_day = render_model(MobileModel {
            time: next_day,
            ..MobileModel::for_page(MobilePage::Home)
        });
        assert_pixel_differences_are_bounded(
            &home_before,
            &home_next_day,
            &[STATUS_TIME, HOME_TIME, HOME_TODAY],
        );
    }

    #[test]
    fn centered_preview_safe_area_is_small_and_has_no_old_pill() {
        let pixels = render_model(MobileModel {
            dark_theme: false,
            ..MobileModel::for_page(MobilePage::Settings)
        });
        assert_eq!(pixels[36 * WIDTH + 360], COLOR_BLACK);
        for (x, y) in [(300, 36), (336, 36), (384, 36), (420, 36)] {
            assert_ne!(
                pixels[y * WIDTH + x],
                COLOR_BLACK,
                "old status pill remains at ({x}, {y})"
            );
        }
    }

    #[test]
    fn raster_font_is_bounded_indexed_and_antialiased() {
        assert_eq!(FONT_ALPHA4.len(), mobile_font_data::FONT_ALPHA4_BYTES);
        assert!(FONT_ALPHA4.len() <= 128 * 1024);
        let mut maximum_end = 0;
        for role in [
            MobileTextRole::Label,
            MobileTextRole::Caption,
            MobileTextRole::Body,
            MobileTextRole::Title,
            MobileTextRole::Headline,
            MobileTextRole::Display,
        ] {
            let (glyphs, map) = font_role_assets(role);
            assert_ne!(map[usize::from(b'?' - b' ')], u8::MAX);
            for mapped in map {
                assert!(*mapped == u8::MAX || usize::from(*mapped) < glyphs.len());
            }
            for glyph in glyphs {
                let offset = usize::try_from(glyph.data_offset).unwrap();
                let pixels = usize::from(glyph.width) * usize::from(glyph.height);
                let end = offset + pixels.div_ceil(2);
                assert!(end <= FONT_ALPHA4.len());
                maximum_end = maximum_end.max(end);
            }
        }
        assert_eq!(maximum_end, FONT_ALPHA4.len());
        assert!(FONT_ALPHA4.iter().any(|packed| {
            let high = packed >> 4;
            let low = packed & 0x0f;
            (1..15).contains(&high) || (1..15).contains(&low)
        }));
    }

    #[test]
    fn raster_font_preserves_case_proportional_width_and_single_fallback() {
        assert!(
            measure_mobile_text_px(MobileTextRole::Title, "iii")
                < measure_mobile_text_px(MobileTextRole::Title, "WWW")
        );
        assert_ne!(
            render_text_sample(MobileTextRole::Title, "Settings"),
            render_text_sample(MobileTextRole::Title, "SETTINGS")
        );
        assert_eq!(
            render_text_sample(MobileTextRole::Body, "é"),
            render_text_sample(MobileTextRole::Body, "?")
        );
        for role in [
            MobileTextRole::Label,
            MobileTextRole::Caption,
            MobileTextRole::Body,
            MobileTextRole::Title,
            MobileTextRole::Headline,
            MobileTextRole::Display,
        ] {
            let sample = render_text_sample(role, "Ag09?");
            assert!(
                sample
                    .iter()
                    .any(|pixel| *pixel != COLOR_BLACK && *pixel != COLOR_WHITE)
            );
        }
    }

    #[test]
    fn canonical_icons_are_bounded_unique_and_antialiased() {
        let icons = [
            Icon::Phone,
            Icon::Messages,
            Icon::Calculator,
            Icon::Settings,
            Icon::DroidCode,
            Icon::Moon,
            Icon::Palette,
            Icon::Sun,
            Icon::Accessibility,
            Icon::TextSize,
            Icon::Contrast,
            Icon::Wifi,
            Icon::Airplane,
            Icon::Info,
            Icon::Backspace,
            Icon::Calendar,
            Icon::Bell,
            Icon::Lock,
            Icon::BatteryUnknown,
            Icon::ChevronLeft,
            Icon::ChevronRight,
            Icon::ChevronUp,
            Icon::ChevronDown,
        ];
        let mut list_digests = [0_u64; 23];
        for size in [IconSize::Status, IconSize::List, IconSize::App] {
            for (index, icon) in icons.into_iter().enumerate() {
                let pixels = render_icon_sample(icon, size, COLOR_WHITE);
                let mut min_x = WIDTH;
                let mut max_x = 0;
                let mut min_y = 96;
                let mut max_y = 0;
                let mut changed = 0;
                for (pixel_index, pixel) in pixels.iter().enumerate() {
                    if *pixel == COLOR_BLACK {
                        continue;
                    }
                    changed += 1;
                    let x = pixel_index % WIDTH;
                    let y = pixel_index / WIDTH;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
                assert!(changed >= 12, "{icon:?}/{size:?}");
                let maximum_extent = usize::try_from(size.design_px() * SCALE + 8).unwrap();
                assert!(max_x - min_x < maximum_extent, "{icon:?}/{size:?}");
                assert!(max_y - min_y < maximum_extent, "{icon:?}/{size:?}");
                assert!(
                    pixels
                        .iter()
                        .any(|pixel| *pixel != COLOR_BLACK && *pixel != COLOR_WHITE),
                    "{icon:?}/{size:?} has no grayscale edge coverage"
                );
                if size == IconSize::List {
                    list_digests[index] = digest(&pixels);
                }
            }
        }
        for left in 0..list_digests.len() {
            for right in (left + 1)..list_digests.len() {
                assert_ne!(list_digests[left], list_digests[right]);
            }
        }
    }

    #[test]
    fn semantic_surface_inks_meet_accessibility_contrast_gate() {
        fn linear_channel(value: u32) -> f64 {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        fn luminance(color: u32) -> f64 {
            0.2126 * linear_channel(component(color, 16))
                + 0.7152 * linear_channel(component(color, 8))
                + 0.0722 * linear_channel(component(color, 0))
        }

        let contrast = |left: u32, right: u32| {
            let lighter = luminance(left).max(luminance(right));
            let darker = luminance(left).min(luminance(right));
            (lighter + 0.05) / (darker + 0.05)
        };

        for surface in [
            COLOR_BLUE,
            COLOR_BLUE_ALT,
            COLOR_PHONE,
            COLOR_MESSAGES,
            COLOR_CALCULATOR,
            COLOR_SETTINGS,
            COLOR_ANDROIDBOX,
            COLOR_AMBER,
        ] {
            let ink = accessible_surface_ink(surface);
            assert!(
                contrast(surface, ink) >= 4.5,
                "surface {surface:#08x} / ink {ink:#08x} failed text contrast"
            );
        }

        for (scheme, high_contrast) in [
            (mobile_theme_tokens(true, false, false), false),
            (mobile_theme_tokens(true, true, false), false),
            (mobile_theme_tokens(false, false, false), false),
            (mobile_theme_tokens(false, true, false), false),
            (mobile_theme_tokens(true, false, true), true),
            (mobile_theme_tokens(true, true, true), true),
            (mobile_theme_tokens(false, false, true), true),
            (mobile_theme_tokens(false, true, true), true),
        ] {
            assert!(contrast(scheme.accent, scheme.on_accent) >= 4.5);
            assert!(contrast(scheme.surface, scheme.text_primary) >= 7.0);
            assert!(contrast(scheme.surface_raised, scheme.text_primary) >= 7.0);
            assert!(contrast(scheme.panel, scheme.text_primary) >= 7.0);
            assert!(contrast(scheme.surface, scheme.text_secondary) >= 4.5);
            assert!(contrast(scheme.surface_raised, scheme.text_secondary) >= 4.5);
            if high_contrast {
                assert_eq!(scheme.text_secondary, scheme.text_primary);
                assert!(contrast(scheme.surface, scheme.outline) >= 3.0);
                assert!(contrast(scheme.surface_raised, scheme.outline) >= 3.0);
            }
        }
    }

    #[test]
    fn semantic_accessibility_preferences_are_bounded_and_change_real_pixels() {
        assert_eq!(
            accessible_text_role(MobileTextRole::Label, true),
            MobileTextRole::Body
        );
        assert_eq!(
            accessible_text_role(MobileTextRole::Caption, true),
            MobileTextRole::Body
        );
        assert_eq!(
            accessible_text_role(MobileTextRole::Body, true),
            MobileTextRole::Title
        );
        for role in [
            MobileTextRole::Title,
            MobileTextRole::Headline,
            MobileTextRole::Display,
        ] {
            assert_eq!(accessible_text_role(role, true), role);
            assert_eq!(accessible_text_role(role, false), role);
        }

        let sample = "WWWWWWWWWWWWWWWWWWWWWWWW";
        let standard_fit = fitted_mobile_text_for_scale(sample, MobileTextRole::Label, 160, false);
        let large_fit = fitted_mobile_text_for_scale(sample, MobileTextRole::Label, 160, true);
        assert!(large_fit.len() < standard_fit.len());
        assert!(
            measure_mobile_text_px(accessible_text_role(MobileTextRole::Label, true), large_fit,)
                <= 160
        );
        let (ellipsis_prefix, truncated) =
            ellipsized_mobile_text_prefix(sample, MobileTextRole::Label, 160, true);
        assert!(truncated);
        assert!(ellipsis_prefix.len() < large_fit.len());
        assert!(
            measure_mobile_text_px(MobileTextRole::Body, ellipsis_prefix)
                + measure_mobile_text_px(MobileTextRole::Body, "...")
                <= 160
        );
        assert_eq!(
            ellipsized_mobile_text_prefix("Readable", MobileTextRole::Label, 160, true),
            ("Readable", false)
        );

        let base = MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_318_060),
            ..MobileModel::for_page(MobilePage::Accessibility)
        };
        let standard = render_model(base);
        let large_model = MobileModel {
            large_text: true,
            ..base
        };
        let large = render_model(large_model);
        let high_model = MobileModel {
            high_contrast: true,
            ..base
        };
        let high = render_model(high_model);
        let combined_model = MobileModel {
            large_text: true,
            high_contrast: true,
            ..base
        };
        let combined = render_model(combined_model);

        for (name, frame) in [("large", &large), ("high", &high), ("combined", &combined)] {
            let changed = standard
                .iter()
                .zip(frame.iter())
                .filter(|(before, after)| before != after)
                .count();
            assert!(changed > 1_000, "{name} changed only {changed} pixels");
            for (x, y) in [
                (0, 0),
                (WIDTH - 1, 0),
                (0, HEIGHT - 1),
                (WIDTH - 1, HEIGHT - 1),
            ] {
                assert_eq!(frame[y * WIDTH + x], COLOR_BLACK, "{name} {x}/{y}");
            }
        }
        assert_ne!(large, high);
        assert_ne!(large, combined);
        assert_ne!(high, combined);

        #[cfg(feature = "mobile-system-chrome0")]
        for model in [large_model, high_model, combined_model] {
            assert_eq!(
                render_split_frame(model, split_chrome_state(model)),
                render_model(model),
                "split ownership changed accessibility pixels"
            );
        }

        #[cfg(feature = "androidbox-interactive0")]
        {
            let activity = interactive_installed_model(
                "Publisher supplied Android Activity text that must remain bounded",
                1,
            );
            let activity_large = MobileModel {
                large_text: true,
                ..activity
            };
            assert_ne!(render_model(activity), render_model(activity_large));
            for (x, y) in [
                (
                    INSTALLED_ANDROID_BUTTON_TARGET.x,
                    INSTALLED_ANDROID_BUTTON_TARGET.y,
                ),
                (
                    INSTALLED_ANDROID_BUTTON_TARGET.x + INSTALLED_ANDROID_BUTTON_TARGET.width - 1,
                    INSTALLED_ANDROID_BUTTON_TARGET.y + INSTALLED_ANDROID_BUTTON_TARGET.height - 1,
                ),
                (360, SYSTEM_NAV_TOP_PX),
            ] {
                assert_eq!(hit_test(activity, x, y), hit_test(activity_large, x, y));
            }
        }
    }

    #[test]
    fn visible_ui_copy_has_explicit_glyph_coverage() {
        for (role, copy) in [
            (MobileTextRole::Display, "00:00 09:41 23:59"),
            (MobileTextRole::Headline, "Today 1234567890 09:41"),
            (
                MobileTextRole::Title,
                "Phone Messages Calculator Settings Display Accessibility Apps AndroidBox Demo Restricted DEX Resource-backed Activity About phone Bndroid OS All apps Using Messages Connection status Error Overview No recent app No installed Android apps Screen & appearance Visual accessibility",
            ),
            (
                MobileTextRole::Body,
                "Sunday Monday Tuesday Wednesday Thursday Friday Saturday January February March April May June July August September October November December Time unavailable Swipe up to continue Enter a number Dark mode Accent color Wi-Fi Network access About phone Theme Accent Network No notifications Local QEMU only Local preview No conversations Using Messages Connection status Local, read-only preview No messages are sent Network is disabled Open Not Android ART Execution failed Verified execution Not executed Execute DEX-0 Run onTap MainActivity Activity launch failed Launcher activity verified Waiting for Activity runtime No install / Binder / JNI APK v2 required System status Compatibility boundary Account profile Verify profile Profile status: pending Details unavailable Local scene No network access Display & appearance Network & internet Connected devices Sound & vibration Hardware status Resolution Interface scale Software surface only Text & contrast Larger text High contrast Readable by design Software UI preference",
            ),
            (
                MobileTextRole::Caption,
                "Sun Mon Tue Wed Thu Fri Sat Sunday Monday Tuesday Wednesday Thursday Friday Saturday Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec January February March April May June July August September October November December Time unavailable You're all caught up Offline Local preview On Off Ocean blue Not available Disabled in preview Disabled Preview build Nothing is waiting Calling unavailable in preview Messaging is unavailable in this preview Local navigation guide Calls and messages unavailable Swipe from the left edge or use Home to navigate or received No network service is connected Calls and messages cannot be sent Clear to continue 720 x 1600 Unlock to review limits Tap to review system limits No hardware or network Recent app Opened this session Open Phone, Messages, or Settings No background tasks Launcher-local DEX-0 No install / Binder / JNI Invalid DEX Verification failed Unsupported opcode Step limit reached Runtime trap No runtime error Result supplied by runtime Waiting for runtime The runtime owns every result AndroidBox Activity-1 No general Android compatibility Manifest rejected Launcher activity missing Activity verification failed Unsupported framework call Activity step limit Activity runtime trap No Activity error org.bndroid.demo.MainActivity Launcher identity not reported No content installed Waiting for resource view AndroidBox resource-backed view Verified DEX execution DEX execution pending Local sideload only Resources-1 profile 1 installed APK v2 verified Resources-1 bytes Launch not requested Choose this app again in All apps APK verification pending No Activity content is shown yet Launch proof is stale Launch request is stale APK verification failed No Activity content is shown App profile unsupported Installed APK unavailable Fresh durable APK readback Hidden until fresh proof All apps remains available Local preview only Resources-1 packages only Theme, color and software dimming Unavailable in QEMU No device transport Not implemented 720 x 1600 / 2x UI scale No panel backlight or HDR control Standard Larger text High contrast Larger text / High contrast Session-wide and reversible Standard semantic type Larger semantic type No device or service authority",
            ),
            (
                MobileTextRole::Label,
                "Sun Mon Tue Wed Thu Fri Sat Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec Time unavailable Appearance Personalization Connections Device This phone System Screen Capability boundary Accessibility Reading Preview Scope The same preference reaches Shell and apps. Screen reader semantics are still pending Apps Installed apps Admission profile Package details Launch activity Preview information Local preview Preview environment Preview boundary System UI Dismiss This session No preview is stored This session only Swipe up for Home Bounded interpreter demo Compatibility boundary No framework compatibility claim Execution status Boot value Result Steps Taps AndroidBox Demo Installed Not Android ART Activity lifecycle Manifest verified Manifest pending onCreate complete onCreate pending TextView Activity instructions Compiled resources resources.arsc layout 0x7f020000 Binary XML @string 0x7f030000 Parsed Pending Resolved Verified Mismatch Restricted DEX-0 Restricted shim / no general compatibility Verified packages will appear here No app is installed Local sideload only Version APK size Generation Signature Profile APK digest Signer digest Network services remain unavailable General Android APIs are unavailable QEMU 360 x 800 dp",
            ),
        ] {
            for character in copy.chars() {
                assert!(
                    mobile_text_has_glyph(role, character),
                    "{role:?} misses {character:?}"
                );
            }
        }
        assert!(!mobile_text_has_glyph(MobileTextRole::Body, 'é'));
    }

    #[test]
    fn rounded_screen_mask_preserves_the_safe_area_on_light_ui() {
        let pixels = render_model(MobileModel {
            dark_theme: false,
            ..MobileModel::default()
        });
        for (x, y) in [
            (0, 0),
            (WIDTH - 1, 0),
            (0, HEIGHT - 1),
            (WIDTH - 1, HEIGHT - 1),
        ] {
            assert_eq!(pixels[y * WIDTH + x], COLOR_BLACK);
        }
        let radius = usize::from(SCREEN_CORNER_RADIUS_PX);
        for (x, y) in [
            (radius, 0),
            (0, radius),
            (WIDTH - 1 - radius, 0),
            (WIDTH - 1, radius),
            (radius, HEIGHT - 1),
            (0, HEIGHT - 1 - radius),
            (WIDTH - 1 - radius, HEIGHT - 1),
            (WIDTH - 1, HEIGHT - 1 - radius),
        ] {
            assert_ne!(pixels[y * WIDTH + x], COLOR_BLACK);
        }
    }

    #[test]
    fn every_page_is_canonical_nonflat_and_distinct() {
        let mut seen = [0_u64; 11];
        for (index, page) in [
            MobilePage::Lock,
            MobilePage::Home,
            MobilePage::Phone,
            MobilePage::Messages,
            MobilePage::Calculator,
            MobilePage::AndroidDemo,
            MobilePage::Settings,
            MobilePage::Apps,
            MobilePage::About,
            MobilePage::Display,
            MobilePage::Accessibility,
        ]
        .into_iter()
        .enumerate()
        {
            let pixels = render_model(MobileModel::for_page(page));
            assert!(pixels.iter().all(|pixel| pixel & 0xff00_0000 == 0));
            assert!(pixels.iter().any(|pixel| *pixel != pixels[0]));
            seen[index] = digest(&pixels);
        }
        for left in 0..seen.len() {
            for right in (left + 1)..seen.len() {
                assert_ne!(seen[left], seen[right]);
            }
        }
    }

    #[test]
    fn row_renderer_matches_complete_frame() {
        for model in [
            MobileModel::locked(),
            MobileModel {
                unlock_reveal_px: 240,
                ..MobileModel::locked()
            },
            MobileModel::for_page(MobilePage::Settings),
            MobileModel::for_page(MobilePage::Display),
            MobileModel::for_page(MobilePage::Accessibility),
            MobileModel {
                large_text: true,
                high_contrast: true,
                ..MobileModel::for_page(MobilePage::Accessibility)
            },
            MobileModel {
                settings_scroll_offset_px: SETTINGS_SCROLL_MAX_PX,
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel::for_page(MobilePage::Apps),
            MobileModel {
                apps_scroll_offset_px: APPS_SCROLL_MAX_PX,
                ..MobileModel::for_page(MobilePage::Apps)
            },
            MobileModel {
                android_installed_app: installed_android_app(7),
                ..MobileModel::for_page(MobilePage::Apps)
            },
            MobileModel {
                shade_reveal_px: 320,
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel {
                shade_open: true,
                shade_reveal_px: 1_000,
                ..MobileModel::for_page(MobilePage::Home)
            },
            MobileModel {
                pressed_target: Some(MobilePressedTarget::About),
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel {
                dark_theme: false,
                alternate_accent: true,
                shade_open: true,
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel {
                software_dimming: UiSoftwareDimming::Strong,
                shade_open: true,
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel {
                drawer_reveal_px: 320,
                ..MobileModel::for_page(MobilePage::Home)
            },
            MobileModel {
                drawer_open: true,
                ..MobileModel::for_page(MobilePage::Home)
            },
            MobileModel {
                back_reveal_px: 96,
                back_origin_y: 800,
                ..MobileModel::for_page(MobilePage::Settings)
            },
            MobileModel {
                phone_digits: *b"1234567890\0\0\0\0\0\0",
                phone_digit_count: 10,
                ..MobileModel::for_page(MobilePage::Phone)
            },
            MobileModel {
                message_view: MessageView::PreviewGuide,
                ..MobileModel::for_page(MobilePage::Messages)
            },
            MobileModel::for_page(MobilePage::Calculator),
            MobileModel {
                calculator_value: 19 * CALCULATOR_SCALE,
                ..MobileModel::for_page(MobilePage::Calculator)
            },
            MobileModel {
                calculator_error: true,
                ..MobileModel::for_page(MobilePage::Calculator)
            },
            MobileModel {
                androidbox_verified: true,
                androidbox_boot_value: -42,
                androidbox_step_count: 17,
                androidbox_tap_count: 3,
                androidbox_manifest_verified: true,
                androidbox_launcher_activity: AndroidBoxActivityIdentity::from_ascii(
                    "org.bndroid.demo.MainActivity",
                )
                .unwrap(),
                androidbox_on_create_completed: true,
                androidbox_text_view_content: AndroidBoxTextViewContent::from_ascii(
                    "AndroidBox resource-backed view",
                )
                .unwrap(),
                androidbox_activity_instruction_count: 11,
                androidbox_resources: verified_androidbox_resources(),
                ..MobileModel::for_page(MobilePage::AndroidDemo)
            },
        ] {
            let complete = render_model(model);
            let mut rows = vec![0_u32; PIXEL_COUNT];
            let mut first_row = 0;
            let mut final_batch_rows = 0;
            while first_row < HEIGHT {
                let batch_rows = (HEIGHT - first_row).min(22);
                render_rows(
                    &mut rows[first_row * WIDTH..(first_row + batch_rows) * WIDTH],
                    first_row,
                    model,
                )
                .unwrap();
                final_batch_rows = batch_rows;
                first_row += batch_rows;
            }
            assert_eq!(final_batch_rows, 16);
            assert_eq!(digest(&complete), digest(&rows));
            assert_eq!(complete, rows);
        }
    }

    #[cfg(feature = "mobile-ui-runtime")]
    fn copy_packed_region(frame: &mut [u32], region: DamageRect, packed: &[u32]) {
        for (row, pixels) in packed.chunks_exact(usize::from(region.width)).enumerate() {
            let start = (usize::from(region.y) + row) * WIDTH + usize::from(region.x);
            frame[start..start + pixels.len()].copy_from_slice(pixels);
        }
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn packed_regions_equal_full_render_across_layers_and_appearance() {
        let models = [
            MobileModel::for_page(MobilePage::Lock),
            MobileModel::for_page(MobilePage::Home),
            MobileModel::for_page(MobilePage::Settings),
            MobileModel {
                shade_open: true,
                ..MobileModel::default()
            },
            MobileModel {
                drawer_open: true,
                ..MobileModel::default()
            },
            MobileModel {
                page_transition_offset_px: 160,
                ..MobileModel::for_page(MobilePage::Phone)
            },
        ];
        for base in models {
            for dark in [true, false] {
                let model = MobileModel {
                    dark_theme: dark,
                    alternate_accent: true,
                    large_text: true,
                    high_contrast: true,
                    software_dimming: UiSoftwareDimming::Strong,
                    ..base
                };
                let complete = render_model(model);
                for region in [
                    DamageRect {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    },
                    DamageRect {
                        x: 17,
                        y: 23,
                        width: 301,
                        height: 127,
                    },
                    DamageRect {
                        x: 63,
                        y: 433,
                        width: 591,
                        height: 301,
                    },
                    DamageRect {
                        x: 4,
                        y: 1203,
                        width: 713,
                        height: 293,
                    },
                    DamageRect {
                        x: 620,
                        y: 1512,
                        width: 100,
                        height: 88,
                    },
                ] {
                    let size = usize::from(region.width) * usize::from(region.height);
                    let mut guarded = vec![0xdead_beef; size + 2];
                    render_region(&mut guarded[1..size + 1], region, model).unwrap();
                    assert_eq!(guarded[0], 0xdead_beef);
                    assert_eq!(guarded[size + 1], 0xdead_beef);
                    for (row, packed) in guarded[1..size + 1]
                        .chunks_exact(usize::from(region.width))
                        .enumerate()
                    {
                        let start = (usize::from(region.y) + row) * WIDTH + usize::from(region.x);
                        assert_eq!(
                            packed,
                            &complete[start..start + packed.len()],
                            "{region:?} row {row}"
                        );
                    }
                }
            }
        }
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn copied_raster_cache_repairs_cancelled_backing_and_resets_focus_baseline() {
        let base = MobileModel::for_page(MobilePage::Phone);
        let mut cache = MobileRasterCache::new();
        assert_eq!(cache.raster_plan(base), MobileDamagePlan::Full);
        cache.record_written(base);
        assert_eq!(cache.present_plan(base, 1), MobileDamagePlan::Full);
        cache.record_presented(base, 1);

        let mut cancelled = base;
        assert!(cancelled.apply(MobileAction::PhoneKey(7)));
        cache.record_written(cancelled);
        let next = MobileModel {
            pressed_target: Some(MobilePressedTarget::PhoneKey(1)),
            ..base
        };
        let mut backing = render_model(cancelled);
        let MobileDamagePlan::Regions(repair) = cache.raster_plan(next) else {
            panic!("missing backing repair")
        };
        render_damage_regions(&mut backing, next, repair).unwrap();
        assert_eq!(
            backing,
            render_model(next),
            "cancelled digit survived in backing"
        );
        let MobileDamagePlan::Regions(present) = cache.present_plan(next, 1) else {
            panic!("missing displayed delta")
        };
        assert!(repair.pixel_count() > present.pixel_count());
        assert_eq!(cache.present_plan(next, 2), MobileDamagePlan::Full);
        assert_eq!(cache.present_plan(next, 0), MobileDamagePlan::Full);
        cache.record_written(next);
        cache.record_presented(next, 2);
        assert_eq!(cache.raster_plan(next), MobileDamagePlan::Unchanged);
        assert_eq!(cache.present_plan(next, 2), MobileDamagePlan::Unchanged);
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn packed_layer_regions_reject_cross_owner_and_invalid_geometry_without_writing() {
        let model = MobileModel::default();
        let chrome = MobileSystemChromeState::new(
            model.time,
            true,
            false,
            UiSoftwareDimming::Off,
            false,
            false,
            false,
        );
        let mut pixels = [0xdead_beef; 32];
        for region in [
            DamageRect {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            DamageRect {
                x: u16::MAX,
                y: 64,
                width: 32,
                height: 1,
            },
            DamageRect {
                x: 0,
                y: 1599,
                width: 16,
                height: 2,
            },
        ] {
            assert!(render_content_region(&mut pixels, region, model).is_err());
            assert!(render_system_chrome_region(&mut pixels, region, chrome).is_err());
            assert!(render_region(&mut pixels, region, model).is_err());
            assert_eq!(pixels, [0xdead_beef; 32]);
        }
        for region in [
            DamageRect {
                x: 0,
                y: 63,
                width: 16,
                height: 2,
            },
            DamageRect {
                x: 0,
                y: 1511,
                width: 16,
                height: 2,
            },
        ] {
            assert_eq!(
                render_content_region(&mut pixels, region, model),
                Err(RenderError::InvalidRowRange)
            );
            assert_eq!(
                render_system_chrome_region(&mut pixels, region, chrome),
                Err(RenderError::InvalidRowRange)
            );
            assert_eq!(pixels, [0xdead_beef; 32]);
        }
        let viewport = DamageRect {
            x: 0,
            y: 64,
            width: 720,
            height: 1448,
        };
        let only_status =
            MobileDamagePlan::Regions(DamageRegions::single(STATUS_TIME_DAMAGE).unwrap());
        assert_eq!(
            clip_damage_plan(only_status, viewport),
            Ok(MobileDamagePlan::Unchanged)
        );
        let crossing = MobileDamagePlan::Regions(
            DamageRegions::single(DamageRect {
                x: 20,
                y: 60,
                width: 200,
                height: 12,
            })
            .unwrap(),
        );
        assert_eq!(
            clip_damage_plan(crossing, viewport),
            Ok(MobileDamagePlan::Regions(
                DamageRegions::single(DamageRect {
                    x: 20,
                    y: 64,
                    width: 200,
                    height: 8
                })
                .unwrap()
            ))
        );
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn cached_chrome_minute_and_navigation_regions_equal_full_layer_render() {
        let base = MobileSystemChromeState::new(
            MobileTimeSnapshot::from_unix_seconds(1_785_318_114),
            true,
            false,
            UiSoftwareDimming::Strong,
            true,
            true,
            false,
        );
        assert_eq!(
            system_chrome_damage_plan(None, base),
            MobileDamagePlan::Full
        );
        assert_eq!(
            system_chrome_damage_plan(Some(base), base),
            MobileDamagePlan::Unchanged
        );
        let same_minute = MobileSystemChromeState {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_318_115),
            ..base
        };
        assert_eq!(
            system_chrome_damage_plan(Some(base), same_minute),
            MobileDamagePlan::Unchanged
        );
        for next in [
            MobileSystemChromeState {
                time: MobileTimeSnapshot::from_unix_seconds(1_785_318_121),
                ..base
            },
            MobileSystemChromeState {
                nav_pressed: true,
                ..base
            },
            MobileSystemChromeState {
                nav_pressed: true,
                time: MobileTimeSnapshot::from_unix_seconds(1_785_318_121),
                ..base
            },
            MobileSystemChromeState {
                dark_theme: false,
                alternate_accent: true,
                ..base
            },
        ] {
            let plan = system_chrome_damage_plan(Some(base), next);
            let mut updated = vec![0xdead_beef; PIXEL_COUNT];
            let mut expected = updated.clone();
            for viewport in [
                DamageRect {
                    x: 0,
                    y: 0,
                    width: 720,
                    height: 64,
                },
                DamageRect {
                    x: 0,
                    y: 1512,
                    width: 720,
                    height: 88,
                },
            ] {
                let start = usize::from(viewport.y) * WIDTH;
                let end = start + usize::from(viewport.height) * WIDTH;
                render_system_chrome_rows(&mut updated[start..end], usize::from(viewport.y), base)
                    .unwrap();
                render_system_chrome_rows(&mut expected[start..end], usize::from(viewport.y), next)
                    .unwrap();
                if let MobileDamagePlan::Regions(regions) =
                    clip_damage_plan(plan, viewport).unwrap()
                {
                    for region in regions.rects() {
                        let mut packed =
                            vec![0; usize::from(region.width) * usize::from(region.height)];
                        render_system_chrome_region(&mut packed, *region, next).unwrap();
                        copy_packed_region(&mut updated, *region, &packed);
                    }
                }
            }
            assert_eq!(updated, expected);
        }
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    fn copied_scene_callback_regions_reconstruct_real_layout_and_leave_gaps_untouched() {
        let base = scene_installed_model(8, 7, true);
        let scene = base.android_installed_activity_scene;
        let label = scene
            .nodes()
            .iter()
            .find(|node| node.kind() == AndroidSceneViewKind::TextView)
            .unwrap()
            .id();
        let button = scene.callback_button_id().unwrap();
        let pressed = MobileModel {
            pressed_target: Some(MobilePressedTarget::InstalledAndroidButton(button)),
            ..base
        };
        let mut next = pressed;
        assert!(next.update_android_installed_activity_scene_text(
            label,
            "Updated by the Android callback",
            8
        ));
        for model in [
            next,
            MobileModel {
                large_text: true,
                high_contrast: true,
                ..next
            },
        ] {
            let before = MobileModel {
                large_text: model.large_text,
                high_contrast: model.high_contrast,
                ..pressed
            };
            let MobileDamagePlan::Regions(regions) = damage_plan(Some(before), model) else {
                panic!("scene callback still redraws Full")
            };
            assert_eq!(regions.count(), 2);
            let old_frame = render_model(before);
            let mut updated = old_frame.clone();
            for rect in regions.rects() {
                let mut packed = vec![0; usize::from(rect.width) * usize::from(rect.height)];
                render_content_region(&mut packed, *rect, model).unwrap();
                copy_packed_region(&mut updated, *rect, &packed);
            }
            assert_eq!(updated, render_model(model));
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    if !regions.rects().iter().any(|r| {
                        ShellRect::new(r.x, r.y, r.width, r.height).contains(x as u16, y as u16)
                    }) {
                        assert_eq!(updated[y * WIDTH + x], old_frame[y * WIDTH + x]);
                    }
                }
            }
        }
        let mut changed_tree = scene_installed_model(7, 9, true);
        changed_tree.pressed_target = None;
        assert_eq!(
            damage_plan(Some(base), changed_tree),
            MobileDamagePlan::Full
        );
        let mut many = base;
        for (offset, id) in scene
            .nodes()
            .iter()
            .filter(|node| node.kind() == AndroidSceneViewKind::TextView)
            .map(|node| node.id())
            .take(3)
            .enumerate()
        {
            assert!(many.update_android_installed_activity_scene_text(
                id,
                "Multiple text updates",
                8 + offset as u64
            ));
        }
        assert_eq!(damage_plan(Some(base), many), MobileDamagePlan::Full);
    }

    #[cfg(feature = "androidbox-layout-size18")]
    #[test]
    fn scene_text_overflow_requires_full_repaint() {
        let mut nodes = installed_android_scene_nodes();
        nodes[1] = AndroidInstalledActivitySceneNode::try_new_sized(
            AndroidSceneViewKind::TextView,
            Some(0),
            nodes[1].id(),
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::Exact,
            AndroidSceneOrientation::None,
            "Tiny",
            false,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            1,
        )
        .unwrap();
        let mut before = installed_android_foreground_model();
        assert!(before.set_android_installed_activity_scene(&nodes, 7));
        let mut after = before;
        assert!(after.update_android_installed_activity_scene_text(
            nodes[1].id(),
            "Changed text",
            8
        ));
        assert_eq!(damage_plan(Some(before), after), MobileDamagePlan::Full);
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn damage_renderer_is_pixel_exact_inside_and_never_touches_outside() {
        let base = MobileModel::for_page(MobilePage::Home);
        let target = MobileModel {
            pressed_target: Some(MobilePressedTarget::Settings),
            ..base
        };
        let base_pixels = render_model(base);
        let target_pixels = render_model(target);

        let interaction_damage = DamageRect {
            x: HOME_SETTINGS_TARGET.x,
            y: HOME_SETTINGS_TARGET.y,
            width: HOME_SETTINGS_TARGET.width,
            height: HOME_SETTINGS_TARGET.height,
        };
        assert_eq!(
            damage_plan(Some(base), target),
            MobileDamagePlan::Regions(DamageRegions::single(interaction_damage).unwrap())
        );
        let mut interaction = base_pixels.clone();
        render_damage(&mut interaction, target, interaction_damage).unwrap();
        assert_eq!(interaction, target_pixels);
        assert_eq!(
            interaction_damage.width as usize * interaction_damage.height as usize,
            32_300
        );

        let narrow = DamageRect {
            x: 117,
            y: 203,
            width: 211,
            height: 307,
        };
        let mut clipped = base_pixels.clone();
        render_damage(&mut clipped, target, narrow).unwrap();
        let right = usize::from(narrow.x + narrow.width);
        let bottom = usize::from(narrow.y + narrow.height);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = y * WIDTH + x;
                if (usize::from(narrow.x)..right).contains(&x)
                    && (usize::from(narrow.y)..bottom).contains(&y)
                {
                    assert_eq!(clipped[index], target_pixels[index], "inside {x}/{y}");
                } else {
                    assert_eq!(clipped[index], base_pixels[index], "outside {x}/{y}");
                }
            }
        }

        let mut one_pixel = base_pixels.clone();
        render_damage(
            &mut one_pixel,
            base,
            DamageRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        )
        .unwrap();
        assert_eq!(one_pixel, base_pixels);
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn damage_planner_is_exact_for_components_unions_scrolling_and_safe_fallbacks() {
        fn assert_exact(previous: MobileModel, current: MobileModel, expected: &[DamageRect]) {
            let expected = DamageRegions::try_new(expected).unwrap();
            assert_eq!(
                damage_plan(Some(previous), current),
                MobileDamagePlan::Regions(expected)
            );
            let mut damaged = render_model(previous);
            render_damage_regions(&mut damaged, current, expected).unwrap();
            assert_eq!(damaged, render_model(current));
        }

        let home = MobileModel::for_page(MobilePage::Home);
        assert_eq!(damage_plan(None, home), MobileDamagePlan::Full);
        assert_eq!(damage_plan(Some(home), home), MobileDamagePlan::Unchanged);

        let settings_pressed = MobileModel {
            pressed_target: Some(MobilePressedTarget::Settings),
            ..home
        };
        assert_exact(
            home,
            settings_pressed,
            &[DamageRect {
                x: 530,
                y: 1_340,
                width: 170,
                height: 190,
            }],
        );

        let phone_pressed = MobileModel {
            pressed_target: Some(MobilePressedTarget::Phone),
            ..home
        };
        assert_exact(
            phone_pressed,
            settings_pressed,
            &[
                DamageRect {
                    x: 20,
                    y: 1_340,
                    width: 170,
                    height: 190,
                },
                DamageRect {
                    x: 530,
                    y: 1_340,
                    width: 170,
                    height: 190,
                },
            ],
        );

        let scrolled_settings = MobileModel {
            settings_scroll_offset_px: 128,
            ..MobileModel::for_page(MobilePage::Settings)
        };
        assert_exact(
            scrolled_settings,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::About),
                ..scrolled_settings
            },
            &[DamageRect {
                x: 48,
                y: 1_288,
                width: 640,
                height: 132,
            }],
        );

        let shifted_notification = MobileModel {
            shade_open: true,
            boot_notification_offset_px: 160,
            software_dimming: UiSoftwareDimming::Maximum,
            ..MobileModel::default()
        };
        assert_exact(
            shifted_notification,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::BootNotification),
                ..shifted_notification
            },
            &[DamageRect {
                x: 192,
                y: 904,
                width: 528,
                height: 232,
            }],
        );

        let overview = MobileModel {
            system_ui_mode: UiSystemUiMode::Overview,
            ..MobileModel::default()
        };
        assert_exact(
            overview,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::OverviewBackground),
                ..overview
            },
            &[DamageRect {
                x: 360,
                y: 800,
                width: 1,
                height: 1,
            }],
        );

        assert_eq!(
            damage_plan(
                Some(home),
                MobileModel {
                    dark_theme: false,
                    ..home
                }
            ),
            MobileDamagePlan::Full
        );
        assert_eq!(
            damage_plan(
                Some(MobileModel::for_page(MobilePage::Phone)),
                MobileModel {
                    pressed_target: Some(MobilePressedTarget::PhoneKey(u8::MAX)),
                    ..MobileModel::for_page(MobilePage::Phone)
                }
            ),
            MobileDamagePlan::Full
        );
        assert_eq!(
            damage_plan(
                Some(MobileModel {
                    drawer_reveal_px: DRAWER_RENDER_QUANTUM,
                    ..home
                }),
                MobileModel {
                    drawer_reveal_px: DRAWER_RENDER_QUANTUM,
                    pressed_target: Some(MobilePressedTarget::Phone),
                    ..home
                }
            ),
            MobileDamagePlan::Full
        );
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn semantic_damage_regions_cover_clock_phone_and_calculator_without_gap_writes() {
        fn assert_exact(previous: MobileModel, current: MobileModel, expected: &[DamageRect]) {
            let expected = DamageRegions::try_new(expected).unwrap();
            assert_eq!(
                damage_plan(Some(previous), current),
                MobileDamagePlan::Regions(expected)
            );

            let current_pixels = render_model(current);
            let mut from_previous = render_model(previous);
            render_damage_regions(&mut from_previous, current, expected).unwrap();
            assert_eq!(from_previous, current_pixels);

            // A sentinel backing proves that neither the bounding box between
            // distant regions nor any other undeclared pixel is rewritten.
            let sentinel = 0x00de_adbe;
            let mut isolated = vec![sentinel; PIXEL_COUNT];
            render_damage_regions(&mut isolated, current, expected).unwrap();
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let index = y * WIDTH + x;
                    let inside = expected.rects().iter().any(|rect| {
                        (usize::from(rect.x)..usize::from(rect.x + rect.width)).contains(&x)
                            && (usize::from(rect.y)..usize::from(rect.y + rect.height)).contains(&y)
                    });
                    assert_eq!(
                        isolated[index],
                        if inside {
                            current_pixels[index]
                        } else {
                            sentinel
                        },
                        "semantic damage pixel {x}/{y}"
                    );
                }
            }
        }

        let minute_41 = MobileTimeSnapshot::from_unix_seconds(1_785_318_060);
        let minute_42 = MobileTimeSnapshot::from_unix_seconds(1_785_318_120);
        let home_41 = MobileModel {
            time: minute_41,
            ..MobileModel::for_page(MobilePage::Home)
        };
        let home_42 = MobileModel {
            time: minute_42,
            ..home_41
        };
        assert_exact(home_41, home_42, &[STATUS_TIME_DAMAGE, HOME_TIME_DAMAGE]);

        let before_midnight = MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_369_599),
            ..MobileModel::for_page(MobilePage::Home)
        };
        let after_midnight = MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_369_600),
            ..before_midnight
        };
        assert_exact(
            before_midnight,
            after_midnight,
            &[STATUS_TIME_DAMAGE, HOME_TIME_DAMAGE.union(HOME_DATE_DAMAGE)],
        );

        let phone_before = MobileModel {
            pressed_target: Some(MobilePressedTarget::PhoneKey(0)),
            ..MobileModel::for_page(MobilePage::Phone)
        };
        let mut phone_after = phone_before;
        assert!(phone_after.apply(MobileAction::PhoneKey(0)));
        assert_exact(
            phone_before,
            phone_after,
            &[
                PHONE_NUMBER_DAMAGE,
                shell_target_damage(PHONE_KEY_TARGETS[0]).unwrap(),
            ],
        );
        let calculator_before = MobileModel {
            pressed_target: Some(MobilePressedTarget::CalculatorKey(4)),
            ..MobileModel::for_page(MobilePage::Calculator)
        };
        let mut calculator_after = calculator_before;
        assert!(calculator_after.apply(MobileAction::CalculatorKey(4)));
        assert_exact(
            calculator_before,
            calculator_after,
            &[
                CALCULATOR_DISPLAY_DAMAGE,
                shell_target_damage(CALCULATOR_KEY_TARGETS[4]).unwrap(),
            ],
        );

        let settings_41 = MobileModel {
            time: minute_41,
            ..MobileModel::for_page(MobilePage::Settings)
        };
        let settings_same_minute = MobileModel {
            time: MobileTimeSnapshot::from_unix_seconds(1_785_318_119),
            ..settings_41
        };
        assert_eq!(
            damage_plan(Some(settings_41), settings_same_minute),
            MobileDamagePlan::Unchanged
        );
        assert_exact(
            settings_same_minute,
            MobileModel {
                time: minute_42,
                ..settings_same_minute
            },
            &[STATUS_TIME_DAMAGE],
        );
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn damage_renderer_rejects_invalid_geometry_before_writing() {
        let model = MobileModel::default();
        let mut pixels = vec![0x0012_3456; PIXEL_COUNT];
        let original = pixels.clone();
        for damage in [
            DamageRect {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            DamageRect {
                x: WIDTH as u16,
                y: 0,
                width: 1,
                height: 1,
            },
            DamageRect {
                x: 0,
                y: HEIGHT as u16,
                width: 1,
                height: 1,
            },
        ] {
            assert_eq!(
                render_damage(&mut pixels, model, damage),
                Err(RenderError::InvalidDamageRect)
            );
            assert_eq!(pixels, original);
        }
        assert_eq!(
            render_damage(&mut pixels[..PIXEL_COUNT - 1], model, DamageRect::FULL),
            Err(RenderError::WrongPixelCount)
        );
        assert_eq!(pixels, original);
    }

    #[test]
    fn lock_actions_and_hidden_surfaces_are_fail_closed_until_explicit_unlock() {
        let mut model = MobileModel::locked();
        let stable = render_model(model);

        for action in [
            MobileAction::Open(MobilePage::Home),
            MobileAction::Open(MobilePage::Phone),
            MobileAction::Open(MobilePage::Messages),
            MobileAction::Open(MobilePage::Calculator),
            MobileAction::Open(MobilePage::AndroidDemo),
            MobileAction::Open(MobilePage::Settings),
            MobileAction::Open(MobilePage::Apps),
            MobileAction::Open(MobilePage::About),
            MobileAction::Open(MobilePage::Display),
            MobileAction::OpenDrawer,
            MobileAction::SetDrawerReveal(DRAWER_GESTURE_MIN_TRAVEL),
            MobileAction::Back,
            MobileAction::Home,
        ] {
            assert!(!model.apply(action), "{action:?}");
            assert_eq!(model.page, MobilePage::Lock, "{action:?}");
            assert_eq!(render_model(model), stable, "{action:?}");
        }

        for (x, y) in [(100, 1_412), (280, 1_412), (448, 1_412), (620, 1_412)] {
            let mut touch = TouchController::new();
            assert_eq!(touch.observe(model, x, y, true), None);
            assert_eq!(touch.observe(model, x, y, false), None);
            assert_eq!(model.page, MobilePage::Lock);
        }

        let mut edge = TouchController::new();
        assert_eq!(edge.observe(model, 32, 800, true), None);
        assert_eq!(edge.observe(model, 176, 800, true), None);
        assert_eq!(edge.observe(model, 176, 800, false), None);
        assert_eq!(render_model(model), stable);

        assert!(model.apply(MobileAction::Unlock));
        assert_eq!(model.page, MobilePage::Home);
        assert_ne!(render_model(model), stable);
    }

    #[test]
    fn lock_system_home_never_unlocks_and_closes_shade_to_exact_lock() {
        let x = SYSTEM_HOME_TARGET.x + SYSTEM_HOME_TARGET.width / 2;
        let y = SYSTEM_HOME_TARGET.y + SYSTEM_HOME_TARGET.height / 2;
        let mut model = MobileModel::locked();
        let stable = render_model(model);

        let mut touch = TouchController::new();
        let pressed = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::SystemHome))
        );
        assert!(model.apply(pressed));
        assert_ne!(render_model(model), stable);
        assert_eq!(touch.observe(model, x, y, false), Some(MobileAction::Home));
        assert!(model.apply(MobileAction::Home));
        assert_eq!(model.page, MobilePage::Lock);
        assert_eq!(render_model(model), stable);

        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.shade_open);
        assert_ne!(render_model(model), stable);
        let mut touch = TouchController::new();
        let pressed = touch.observe(model, x, y, true).unwrap();
        assert!(model.apply(pressed));
        assert_eq!(touch.observe(model, x, y, false), Some(MobileAction::Home));
        assert!(model.apply(MobileAction::Home));
        assert_eq!(model.page, MobilePage::Lock);
        assert!(!model.shade_open);
        assert_eq!(model.shade_reveal_px, 0);
        assert_eq!(render_model(model), stable);
    }

    #[test]
    fn unlock_threshold_is_exact_and_short_drag_restores_lock_exactly() {
        let mut model = MobileModel::locked();
        let stable = render_model(model);
        let mut short = TouchController::new();
        assert_eq!(short.observe(model, 360, 1_200, true), None);
        let reveal = short.observe(model, 360, 961, true).unwrap();
        assert_eq!(reveal, MobileAction::SetUnlockReveal(239));
        assert!(model.apply(reveal));
        assert_eq!(model.unlock_reveal_px, 240);
        assert_ne!(render_model(model), stable);
        assert_eq!(
            short.observe(model, 360, 961, false),
            Some(MobileAction::SetUnlockReveal(0))
        );
        assert!(model.apply(MobileAction::SetUnlockReveal(0)));
        assert_eq!(model.page, MobilePage::Lock);
        assert_eq!(render_model(model), stable);

        let mut exact = TouchController::new();
        assert_eq!(exact.observe(model, 360, 1_200, true), None);
        let reveal = exact.observe(model, 360, 960, true).unwrap();
        assert_eq!(reveal, MobileAction::SetUnlockReveal(240));
        assert!(model.apply(reveal));
        assert_eq!(
            exact.observe(model, 360, 960, false),
            Some(MobileAction::Unlock)
        );
        assert!(model.apply(MobileAction::Unlock));
        assert_eq!(model.page, MobilePage::Home);
    }

    #[test]
    fn unlock_reveal_is_sixteen_pixel_quantized_and_same_bucket_is_stable() {
        let mut model = MobileModel::locked();
        assert!(model.apply(MobileAction::SetUnlockReveal(1)));
        assert_eq!(model.unlock_reveal_px, UNLOCK_RENDER_QUANTUM);
        assert!(!model.apply(MobileAction::SetUnlockReveal(15)));
        assert!(!model.apply(MobileAction::SetUnlockReveal(16)));
        assert!(model.apply(MobileAction::SetUnlockReveal(17)));
        assert_eq!(model.unlock_reveal_px, 32);
        assert!(!model.apply(MobileAction::SetUnlockReveal(31)));
        assert!(!model.apply(MobileAction::SetUnlockReveal(32)));
        assert!(model.apply(MobileAction::SetUnlockReveal(u16::MAX)));
        assert_eq!(model.unlock_reveal_px, UNLOCK_REVEAL_MAX);
        assert_eq!(model.effective_unlock_reveal_px(), UNLOCK_REVEAL_MAX);
        assert!(!model.apply(MobileAction::SetUnlockReveal(u16::MAX)));
    }

    #[test]
    fn unlock_gesture_rejects_horizontal_outside_and_navigation_area_paths() {
        let stable_model = MobileModel::locked();
        let stable = render_model(stable_model);

        let mut horizontal = TouchController::new();
        assert_eq!(horizontal.observe(stable_model, 360, 1_200, true), None);
        assert_eq!(horizontal.observe(stable_model, 384, 1_200, true), None);
        assert_eq!(horizontal.observe(stable_model, 360, 900, true), None);
        assert_eq!(horizontal.observe(stable_model, 360, 900, false), None);

        let mut outside_model = stable_model;
        let mut outside = TouchController::new();
        assert_eq!(outside.observe(outside_model, 360, 1_200, true), None);
        let reveal = outside.observe(outside_model, 360, 1_176, true).unwrap();
        assert!(outside_model.apply(reveal));
        assert_eq!(
            outside.observe(outside_model, 720, 900, false),
            Some(MobileAction::SetUnlockReveal(0))
        );
        assert!(outside_model.apply(MobileAction::SetUnlockReveal(0)));
        assert_eq!(render_model(outside_model), stable);

        let mut drift_model = stable_model;
        let mut drift = TouchController::new();
        assert_eq!(drift.observe(drift_model, 360, 1_200, true), None);
        let reveal = drift.observe(drift_model, 360, 1_176, true).unwrap();
        assert!(drift_model.apply(reveal));
        assert_eq!(
            drift.observe(drift_model, 553, 900, true),
            Some(MobileAction::SetUnlockReveal(0))
        );
        assert!(drift_model.apply(MobileAction::SetUnlockReveal(0)));
        assert_eq!(drift.observe(drift_model, 553, 900, false), None);
        assert_eq!(render_model(drift_model), stable);

        let mut nav_model = stable_model;
        let mut nav = TouchController::new();
        assert_eq!(nav.observe(nav_model, 360, 1_400, true), None);
        let reveal = nav.observe(nav_model, 360, 1_376, true).unwrap();
        assert!(nav_model.apply(reveal));
        assert_eq!(
            nav.observe(nav_model, 360, SYSTEM_NAV_TOP_PX, false),
            Some(MobileAction::SetUnlockReveal(0))
        );
        assert!(nav_model.apply(MobileAction::SetUnlockReveal(0)));
        assert_eq!(nav_model.page, MobilePage::Lock);
        assert_eq!(render_model(nav_model), stable);

        let mut starts_in_nav = TouchController::new();
        assert_eq!(
            starts_in_nav.observe(stable_model, 100, SYSTEM_NAV_TOP_PX, true),
            None
        );
        assert_eq!(starts_in_nav.observe(stable_model, 100, 1_200, true), None);
        assert_eq!(starts_in_nav.observe(stable_model, 100, 1_200, false), None);
    }

    #[test]
    fn unlock_explicit_cancel_restores_and_latches_until_release() {
        let mut model = MobileModel::locked();
        let stable = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 1_200, true), None);
        let reveal = touch.observe(model, 360, 1_168, true).unwrap();
        assert!(model.apply(reveal));
        assert_ne!(render_model(model), stable);

        let clear = touch.cancel_contact(model).unwrap();
        assert_eq!(clear, MobileAction::SetUnlockReveal(0));
        assert!(model.apply(clear));
        assert_eq!(model.page, MobilePage::Lock);
        assert_eq!(render_model(model), stable);

        assert_eq!(touch.observe(model, 360, 900, true), None);
        assert_eq!(touch.observe(model, 360, 900, false), None);
        assert_eq!(touch.observe(model, 360, 1_200, true), None);
        assert_eq!(
            touch.observe(model, 360, 1_176, true),
            Some(MobileAction::SetUnlockReveal(24))
        );
    }

    #[test]
    fn settings_display_apps_and_about_navigation_preserves_real_controls() {
        let mut model = MobileModel::for_page(MobilePage::Settings);
        assert!(model.apply(MobileAction::Open(MobilePage::Display)));
        assert_eq!(model.page, MobilePage::Display);
        let dark = render_model(model);
        assert!(model.apply(MobileAction::ToggleTheme));
        let light = render_model(model);
        assert_ne!(digest(&dark), digest(&light));
        assert!(model.apply(MobileAction::ToggleAccent));
        let alternate = render_model(model);
        assert_ne!(digest(&light), digest(&alternate));
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
        assert!(model.apply(MobileAction::Open(MobilePage::Apps)));
        assert_eq!(model.page, MobilePage::Apps);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
        assert!(model.apply(MobileAction::Open(MobilePage::About)));
        assert_eq!(model.page, MobilePage::About);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
    }

    #[test]
    fn installed_app_status_is_bounded_ascii_runtime_data_only() {
        assert_eq!(
            AndroidInstalledAppStatus::default(),
            AndroidInstalledAppStatus::empty()
        );
        let installed = installed_android_app(7);
        assert!(installed.installed);
        assert_eq!(installed.generation, 7);
        assert_eq!(installed.version_code, 7);
        assert_eq!(installed.apk_length, 12_566);
        assert_eq!(installed.package.as_str(), "org.bndroid.demo");
        assert_eq!(installed.activity.as_str(), "org.bndroid.demo.MainActivity");
        assert_eq!(installed.title.as_str(), "AndroidBox");
        assert_eq!(installed.text.as_str(), "AndroidBox resource-backed view");
        assert_eq!(installed.signer_digest_sha256, [0x5a; 32]);
        assert_eq!(installed.apk_digest_sha256, [0xa5; 32]);

        assert!(
            AndroidInstalledAppStatus::try_new(
                0,
                7,
                12_566,
                "org.bndroid.demo",
                "org.bndroid.demo.MainActivity",
                "AndroidBox",
                "text",
                [1; 32],
                [2; 32],
            )
            .is_none()
        );
        assert!(
            AndroidInstalledAppStatus::try_new(
                1,
                7,
                0,
                "org.bndroid.demo",
                "org.bndroid.demo.MainActivity",
                "AndroidBox",
                "text",
                [1; 32],
                [2; 32],
            )
            .is_none()
        );
        assert!(
            AndroidInstalledAppStatus::try_new(
                1,
                7,
                1,
                "org.bndroid.demo\n",
                "org.bndroid.demo.MainActivity",
                "AndroidBox",
                "text",
                [1; 32],
                [2; 32],
            )
            .is_none()
        );
        assert!(
            AndroidInstalledAppStatus::try_new(
                1,
                7,
                1,
                "org.bndroid.demo",
                "org.bndroid.demo.MainActivity",
                "AndroidBox \u{00e9}",
                "text",
                [1; 32],
                [2; 32],
            )
            .is_none()
        );
        let maximum_package = "A".repeat(ANDROID_INSTALLED_PACKAGE_CAPACITY);
        let too_long_package = "A".repeat(ANDROID_INSTALLED_PACKAGE_CAPACITY + 1);
        assert!(AndroidInstalledPackage::from_ascii(&maximum_package).is_some());
        assert!(AndroidInstalledPackage::from_ascii(&too_long_package).is_none());
        let maximum_activity = "A".repeat(ANDROID_INSTALLED_ACTIVITY_CAPACITY);
        let too_long_activity = "A".repeat(ANDROID_INSTALLED_ACTIVITY_CAPACITY + 1);
        assert!(AndroidInstalledActivity::from_ascii(&maximum_activity).is_some());
        assert!(AndroidInstalledActivity::from_ascii(&too_long_activity).is_none());
        let maximum_title = "A".repeat(ANDROID_INSTALLED_TITLE_CAPACITY);
        let too_long_title = "A".repeat(ANDROID_INSTALLED_TITLE_CAPACITY + 1);
        assert!(AndroidInstalledTitle::from_ascii(&maximum_title).is_some());
        assert!(AndroidInstalledTitle::from_ascii(&too_long_title).is_none());
        let maximum_text = "A".repeat(ANDROID_INSTALLED_TEXT_CAPACITY);
        let too_long_text = "A".repeat(ANDROID_INSTALLED_TEXT_CAPACITY + 1);
        assert!(AndroidInstalledText::from_ascii(&maximum_text).is_some());
        assert!(AndroidInstalledText::from_ascii(&too_long_text).is_none());

        let mut model = MobileModel::default();
        assert!(model.apply_android_installed_app_status(installed));
        assert!(!model.apply_android_installed_app_status(installed));
        let mirrored = model.android_installed_app;
        assert!(model.apply(MobileAction::Open(MobilePage::AndroidDemo)));
        assert!(!model.apply(MobileAction::ExecuteAndroidBoxDex));
        assert_eq!(model.android_installed_app, mirrored);

        let mut demo_result = MobileModel::default();
        assert!(demo_result.apply_androidbox_result(true, -42, 17, 1, AndroidBoxError::None));
        assert_eq!(
            demo_result.android_installed_app,
            AndroidInstalledAppStatus::empty()
        );
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn installed_directory_accepts_two_unique_packed_apps_and_switches_selection() {
        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        let mut model = MobileModel::default();
        assert!(model.apply_android_installed_directory_status([envelope, catalog], 2, 7, 1,));
        assert_eq!(model.android_installed_apps, [envelope, catalog]);
        assert_eq!(model.android_installed_app_count, 2);
        assert_eq!(model.android_installed_directory_revision, 7);
        assert_eq!(model.android_installed_app, catalog);
        assert!(model.select_android_installed_app(0));
        assert_eq!(model.android_installed_app, envelope);
        assert!(!model.select_android_installed_app(2));
        assert_eq!(model.android_installed_app, envelope);
        assert!(!model.apply_android_installed_directory_status([envelope, catalog], 2, 7, 0,));
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn installed_directory_rejects_duplicates_holes_and_noncanonical_counts_transactionally() {
        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        let mut model = MobileModel::default();
        assert!(model.apply_android_installed_directory_status([envelope, catalog], 2, 3, 0,));
        let stable = model;
        for (apps, count, revision, selected) in [
            ([envelope, envelope], 2, 4, 0),
            ([AndroidInstalledAppStatus::empty(), catalog], 2, 4, 0),
            ([envelope, catalog], 1, 4, 0),
            ([envelope, catalog], 3, 4, 0),
            ([envelope, catalog], 2, 0, 0),
            ([envelope, catalog], 2, 4, 2),
        ] {
            assert!(
                !model.apply_android_installed_directory_status(apps, count, revision, selected,)
            );
            assert_eq!(model, stable);
        }
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn two_installed_drawer_cells_emit_stable_one_based_selectors() {
        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        let mut model = MobileModel {
            drawer_open: true,
            ..MobileModel::default()
        };
        assert!(model.apply_android_installed_directory_status([envelope, catalog], 2, 3, 0,));
        let one_app_frame = {
            let mut one = model;
            assert!(one.apply_android_installed_directory_status(
                [envelope, AndroidInstalledAppStatus::empty()],
                1,
                2,
                0,
            ));
            render_model(one)
        };
        assert_ne!(one_app_frame, render_model(model));

        for (target, selector) in [
            (DRAWER_ANDROIDBOX_TARGET, 1),
            (DRAWER_ANDROIDBOX_SECOND_TARGET, 2),
        ] {
            let x = target.x + target.width / 2;
            let y = target.y + target.height / 2;
            let mut touch = TouchController::new();
            assert_eq!(
                touch.observe(model, x, y, true),
                Some(MobileAction::SetPressed(Some(
                    MobilePressedTarget::DrawerInstalledAndroid(selector),
                )))
            );
            assert_eq!(
                touch.observe(model, x, y, false),
                Some(MobileAction::LaunchInstalledAndroid(selector))
            );
        }
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn apps_page_selects_two_installed_packages_without_mutating_the_directory() {
        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        let mut model = MobileModel::for_page(MobilePage::Apps);
        assert!(model.apply_android_installed_directory_status([envelope, catalog], 2, 9, 0,));
        assert!(apps_package_selection_is_available(model));
        let initial = render_model(model);
        let directory = model.android_installed_apps;
        let revision = model.android_installed_directory_revision;

        let x = APPS_INSTALLED_SECOND_TARGET.x + APPS_INSTALLED_SECOND_TARGET.width / 2;
        let y = APPS_INSTALLED_SECOND_TARGET.y + APPS_INSTALLED_SECOND_TARGET.height / 2;
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::AppsInstalledAndroid(1)))
        );
        assert!(model.apply(down));
        assert_ne!(render_model(model), initial);
        let select = touch.observe(model, x, y, false).unwrap();
        assert_eq!(select, MobileAction::SelectInstalledAndroid(1));
        assert!(model.apply(select));
        assert_eq!(model.android_installed_app, catalog);
        assert_eq!(model.android_installed_apps, directory);
        assert_eq!(model.android_installed_directory_revision, revision);
        assert_ne!(render_model(model), initial);

        let stable = model;
        assert!(!model.apply(MobileAction::SelectInstalledAndroid(2)));
        assert_eq!(model, stable);

        model.android_install_candidate = android_install_candidate(
            AndroidInstallCandidateAction::Update,
            1,
            1,
            2,
            catalog.apk_digest_sha256,
        );
        assert!(!apps_package_selection_is_available(model));
        assert!(!model.apply(MobileAction::SelectInstalledAndroid(0)));
        assert_eq!(model.android_installed_app, catalog);
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn advanced_interactive_targets_have_pixel_exact_component_damage() {
        const fn fixed(target: ShellRect) -> DamageRect {
            DamageRect {
                x: target.x,
                y: target.y,
                width: target.width,
                height: target.height,
            }
        }

        let mut uninstall = MobileModel {
            apps_scroll_offset_px: APPS_SCROLL_MAX_PX,
            android_installed_app: installed_android_app(7),
            ..MobileModel::for_page(MobilePage::Apps)
        };
        assert_planned_damage_is_pixel_exact(
            uninstall,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::AppsUninstall),
                ..uninstall
            },
            DamageRect {
                y: APPS_UNINSTALL_TARGET.y - APPS_SCROLL_MAX_PX,
                ..fixed(APPS_UNINSTALL_TARGET)
            },
        );
        assert!(uninstall.begin_android_installed_uninstall(7));
        for (target, rect) in [
            (
                MobilePressedTarget::AppsUninstallCancel,
                APPS_UNINSTALL_CANCEL_TARGET,
            ),
            (
                MobilePressedTarget::AppsUninstallConfirm,
                APPS_UNINSTALL_CONFIRM_TARGET,
            ),
        ] {
            assert_planned_damage_is_pixel_exact(
                uninstall,
                MobileModel {
                    pressed_target: Some(target),
                    ..uninstall
                },
                fixed(rect),
            );
        }

        let install_candidate =
            android_install_candidate(AndroidInstallCandidateAction::Install, 0, 0, 7, [0xa5; 32]);
        let mut install = MobileModel::for_page(MobilePage::Apps);
        assert!(install.apply_android_install_candidate_status(install_candidate));
        assert_planned_damage_is_pixel_exact(
            install,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::AppsInstall),
                ..install
            },
            fixed(APPS_INSTALL_TARGET),
        );
        assert!(install.begin_android_install(install_candidate.candidate_id));
        for (target, rect) in [
            (
                MobilePressedTarget::AppsInstallCancel,
                APPS_INSTALL_CANCEL_TARGET,
            ),
            (
                MobilePressedTarget::AppsInstallConfirm,
                APPS_INSTALL_CONFIRM_TARGET,
            ),
        ] {
            assert_planned_damage_is_pixel_exact(
                install,
                MobileModel {
                    pressed_target: Some(target),
                    ..install
                },
                fixed(rect),
            );
        }

        let update_candidate =
            android_install_candidate(AndroidInstallCandidateAction::Update, 7, 7, 8, [0xb6; 32]);
        let mut clipped_update = MobileModel {
            apps_scroll_offset_px: 192,
            android_installed_app: installed_android_app(7),
            ..MobileModel::for_page(MobilePage::Apps)
        };
        assert!(clipped_update.apply_android_install_candidate_status(update_candidate));
        assert_planned_damage_is_pixel_exact(
            clipped_update,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::AppsInstall),
                ..clipped_update
            },
            DamageRect {
                x: APPS_UPDATE_TARGET.x,
                y: PAGE_SCROLL_VIEWPORT_TOP_PX,
                width: APPS_UPDATE_TARGET.width,
                height: APPS_UPDATE_TARGET.y + APPS_UPDATE_TARGET.height
                    - 192
                    - PAGE_SCROLL_VIEWPORT_TOP_PX,
            },
        );

        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        let mut apps = MobileModel::for_page(MobilePage::Apps);
        assert!(apps.apply_android_installed_directory_status([envelope, catalog], 2, 9, 0));
        for (index, rect) in [APPS_INSTALLED_FIRST_TARGET, APPS_INSTALLED_SECOND_TARGET]
            .into_iter()
            .enumerate()
        {
            assert_planned_damage_is_pixel_exact(
                apps,
                MobileModel {
                    pressed_target: Some(MobilePressedTarget::AppsInstalledAndroid(index as u8)),
                    ..apps
                },
                fixed(rect),
            );
        }

        let mut drawer = MobileModel {
            drawer_open: true,
            ..MobileModel::default()
        };
        assert!(drawer.apply_android_installed_directory_status([envelope, catalog], 2, 9, 0));
        for (selector, rect) in [
            (1, DRAWER_ANDROIDBOX_TARGET),
            (2, DRAWER_ANDROIDBOX_SECOND_TARGET),
        ] {
            assert_planned_damage_is_pixel_exact(
                drawer,
                MobileModel {
                    pressed_target: Some(MobilePressedTarget::DrawerInstalledAndroid(selector)),
                    ..drawer
                },
                fixed(rect),
            );
        }

        let legacy = interactive_installed_model("Ready", 1);
        let legacy_button_id = legacy.android_installed_activity_view.button_view_id();
        assert_planned_damage_is_pixel_exact(
            legacy,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::InstalledAndroidButton(
                    legacy_button_id,
                )),
                ..legacy
            },
            fixed(INSTALLED_ANDROID_BUTTON_TARGET),
        );

        let scene = scene_installed_model(8, 7, true);
        let scene_button_id = scene
            .android_installed_activity_scene
            .callback_button_id()
            .unwrap();
        let scene_target = installed_android_scene_button_target(scene, scene_button_id).unwrap();
        assert_planned_damage_is_pixel_exact(
            scene,
            MobileModel {
                pressed_target: Some(MobilePressedTarget::InstalledAndroidButton(scene_button_id)),
                ..scene
            },
            fixed(scene_target),
        );
        assert_eq!(
            damage_plan(
                Some(scene),
                MobileModel {
                    pressed_target: Some(MobilePressedTarget::InstalledAndroidButton(u32::MAX)),
                    ..scene
                }
            ),
            MobileDamagePlan::Full
        );
    }

    #[cfg(feature = "androidbox-multipackage4")]
    #[test]
    fn installed_fallback_icons_are_stable_distinct_and_package_derived() {
        let envelope = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        );
        let catalog = installed_android_app_named(
            1,
            "org.bndroid.catalog",
            "org.bndroid.catalog.MainActivity",
            "Component catalog",
            0x22,
        );
        assert_eq!(
            installed_app_fallback_color(envelope),
            installed_app_fallback_color(envelope)
        );
        assert_ne!(
            installed_app_fallback_color(envelope),
            installed_app_fallback_color(catalog)
        );

        let mut model = MobileModel {
            drawer_open: true,
            ..MobileModel::default()
        };
        assert!(model.apply_android_installed_directory_status([envelope, catalog], 2, 3, 0,));
        let pixels = render_model(model);
        let icon_y = 335 * SCALE;
        let envelope_x = (52 - 17) * SCALE;
        let catalog_x = (138 - 17) * SCALE;
        assert_eq!(
            pixels[icon_y as usize * WIDTH + envelope_x as usize],
            installed_app_fallback_color(envelope)
        );
        assert_eq!(
            pixels[icon_y as usize * WIDTH + catalog_x as usize],
            installed_app_fallback_color(catalog)
        );
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    #[test]
    fn installed_real_icon_palette_and_drawer_pixels_are_exact() {
        let mut pixels = [0; ANDROID_INSTALLED_ICON_PIXEL_COUNT];
        pixels.fill(0xff0b_57d0);
        pixels[1] = 0xffff_ffff;
        pixels[2] = 0;
        let icon = AndroidInstalledIcon::try_new(0x7f01_0000, 0xf63d_0b72, pixels).unwrap();
        assert_eq!(icon.pixel(0), Some(0xff0b_57d0));
        assert_eq!(icon.pixel(1), Some(0xffff_ffff));
        assert_eq!(icon.pixel(2), Some(0));
        assert_eq!(icon.pixel(ANDROID_INSTALLED_ICON_PIXEL_COUNT), None);

        let installed = installed_android_app_named(
            1,
            "org.bndroid.envelope",
            "org.bndroid.envelope.MainActivity",
            "Envelope demo",
            0x11,
        )
        .with_icon(Some(icon))
        .unwrap();
        let mut model = MobileModel {
            drawer_open: true,
            ..MobileModel::default()
        };
        assert!(model.apply_android_installed_directory_status(
            [installed, AndroidInstalledAppStatus::empty()],
            1,
            1,
            0,
        ));
        let rendered = render_model(model);
        let top = (335 - 24) * SCALE;
        let left = (52 - 24) * SCALE;
        assert_eq!(rendered[top as usize * WIDTH + left as usize], 0x000b_57d0);
        assert_eq!(
            rendered[top as usize * WIDTH + (left + 3 * SCALE) as usize],
            0x00ff_ffff
        );
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    #[test]
    fn installed_real_icon_rejects_noncanonical_or_unbounded_palettes() {
        let mut transparent_rgb = [0xff0b_57d0; ANDROID_INSTALLED_ICON_PIXEL_COUNT];
        transparent_rgb[0] = 1;
        assert!(AndroidInstalledIcon::try_new(0x7f01_0000, 1, transparent_rgb).is_none());

        let mut too_many_colors = [0; ANDROID_INSTALLED_ICON_PIXEL_COUNT];
        for (index, pixel) in too_many_colors.iter_mut().take(17).enumerate() {
            *pixel = 0xff00_0000 | index as u32;
        }
        assert!(AndroidInstalledIcon::try_new(0x7f01_0000, 1, too_many_colors).is_none());
        assert!(
            AndroidInstalledIcon::try_new(0, 1, [0xff0b_57d0; ANDROID_INSTALLED_ICON_PIXEL_COUNT])
                .is_none()
        );
        assert!(
            AndroidInstalledIcon::try_new(
                0x7f01_0000,
                0,
                [0xff0b_57d0; ANDROID_INSTALLED_ICON_PIXEL_COUNT]
            )
            .is_none()
        );
    }

    #[cfg(feature = "androidbox-runtime-uninstall1")]
    #[test]
    fn settings_runtime_uninstall_requires_two_taps_and_runtime_terminal_evidence() {
        let mut model = MobileModel {
            apps_scroll_offset_px: APPS_SCROLL_MAX_PX,
            android_installed_app: installed_android_app(7),
            ..MobileModel::for_page(MobilePage::Apps)
        };
        let x = APPS_UNINSTALL_TARGET.x + APPS_UNINSTALL_TARGET.width / 2;
        let y = APPS_UNINSTALL_TARGET.y + APPS_UNINSTALL_TARGET.height / 2
            - model.effective_page_scroll_offset_px();
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::AppsUninstall))
        );
        assert!(model.apply(down));
        let begin = touch.observe(model, x, y, false).unwrap();
        assert_eq!(begin, MobileAction::BeginInstalledAndroidUninstall(7));
        assert!(model.apply(begin));
        assert_eq!(
            model.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Confirming { generation: 7 }
        );
        assert_eq!(hit_test(model, x, y), None);
        assert!(!model.mark_android_installed_uninstall_pending(1, 8));

        let confirm_x = APPS_UNINSTALL_CONFIRM_TARGET.x + APPS_UNINSTALL_CONFIRM_TARGET.width / 2;
        let confirm_y = APPS_UNINSTALL_CONFIRM_TARGET.y + APPS_UNINSTALL_CONFIRM_TARGET.height / 2;
        let mut confirm_touch = TouchController::new();
        let down = confirm_touch
            .observe(model, confirm_x, confirm_y, true)
            .unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::AppsUninstallConfirm))
        );
        assert!(model.apply(down));
        let confirm = confirm_touch
            .observe(model, confirm_x, confirm_y, false)
            .unwrap();
        assert_eq!(confirm, MobileAction::ConfirmInstalledAndroidUninstall(7));
        assert!(model.apply(confirm));
        assert!(model.mark_android_installed_uninstall_pending(1, 7));
        assert_eq!(hit_test(model, confirm_x, confirm_y), None);
        let pending = render_model(model);

        assert!(model.apply_android_installed_app_status(AndroidInstalledAppStatus::empty()));
        assert_eq!(
            model.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Pending {
                request_sequence: 1,
                generation: 7
            }
        );
        assert!(model.finish_android_installed_uninstall(1, 7, Ok(8)));
        assert_eq!(
            model.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Removed {
                request_sequence: 1,
                removal_generation: 8
            }
        );
        let removed = render_model(model);
        assert_ne!(digest(&pending), digest(&removed));
        assert!(!model.apply(MobileAction::BeginInstalledAndroidUninstall(7)));

        let mut failed = MobileModel {
            android_installed_app: installed_android_app(9),
            ..MobileModel::for_page(MobilePage::Apps)
        };
        assert!(failed.begin_android_installed_uninstall(9));
        assert!(failed.mark_android_installed_uninstall_pending(1, 9));
        assert!(failed.finish_android_installed_uninstall(
            1,
            9,
            Err(AndroidInstalledUninstallFailure::Storage),
        ));
        assert!(matches!(
            failed.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Failed {
                failure: AndroidInstalledUninstallFailure::Storage,
                ..
            }
        ));
        assert!(failed.cancel_android_installed_uninstall());
        assert_eq!(
            failed.android_installed_uninstall,
            AndroidInstalledUninstallStatus::Idle
        );
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    #[test]
    fn settings_runtime_install_requires_two_taps_and_exact_terminal_catalog() {
        let candidate =
            android_install_candidate(AndroidInstallCandidateAction::Install, 0, 0, 7, [0xa5; 32]);
        let mut model = MobileModel::for_page(MobilePage::Apps);
        assert!(model.apply_android_install_candidate_status(candidate));
        let ready = render_model(model);

        let x = APPS_INSTALL_TARGET.x + APPS_INSTALL_TARGET.width / 2;
        let y = APPS_INSTALL_TARGET.y + APPS_INSTALL_TARGET.height / 2;
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::AppsInstall))
        );
        assert!(model.apply(down));
        let begin = touch.observe(model, x, y, false).unwrap();
        assert_eq!(begin, MobileAction::BeginAndroidInstall(0x53));
        assert!(model.apply(begin));
        assert_eq!(
            model.android_install,
            AndroidInstallStatus::Confirming { candidate_id: 0x53 }
        );
        assert_eq!(
            model.android_installed_app,
            AndroidInstalledAppStatus::empty()
        );
        assert_eq!(model.android_install_candidate, candidate);
        let confirming = render_model(model);
        assert_ne!(digest(&ready), digest(&confirming));

        let confirm_x = APPS_INSTALL_CONFIRM_TARGET.x + APPS_INSTALL_CONFIRM_TARGET.width / 2;
        let confirm_y = APPS_INSTALL_CONFIRM_TARGET.y + APPS_INSTALL_CONFIRM_TARGET.height / 2;
        let mut confirm_touch = TouchController::new();
        let down = confirm_touch
            .observe(model, confirm_x, confirm_y, true)
            .unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::AppsInstallConfirm))
        );
        assert!(model.apply(down));
        let confirm = confirm_touch
            .observe(model, confirm_x, confirm_y, false)
            .unwrap();
        assert_eq!(confirm, MobileAction::ConfirmAndroidInstall(0x53));
        assert!(model.apply(confirm));
        assert!(model.mark_android_install_pending(1, 0x53));
        assert!(!model.finish_android_install(2, candidate, Ok(1)));
        assert!(model.apply_android_installed_app_status(installed_android_app(1)));
        assert!(
            model.apply_android_install_candidate_status(AndroidInstallCandidateStatus::empty())
        );
        assert!(model.finish_android_install(1, candidate, Ok(1)));
        assert_eq!(
            model.android_install,
            AndroidInstallStatus::Installed {
                request_sequence: 1,
                candidate_id: 0x53,
                generation: 1,
            }
        );
        assert_ne!(digest(&confirming), digest(&render_model(model)));
    }

    #[cfg(feature = "androidbox-runtime-install2")]
    #[test]
    fn pending_update_blocks_uninstall_and_stale_install_fails_closed() {
        let update =
            android_install_candidate(AndroidInstallCandidateAction::Update, 7, 7, 8, [0xb6; 32]);
        let mut model = MobileModel {
            apps_scroll_offset_px: APPS_SCROLL_MAX_PX,
            android_installed_app: installed_android_app(7),
            ..MobileModel::for_page(MobilePage::Apps)
        };
        assert!(model.apply_android_install_candidate_status(update));
        assert!(!model.begin_android_installed_uninstall(7));
        let uninstall_x = APPS_UNINSTALL_TARGET.x + APPS_UNINSTALL_TARGET.width / 2;
        let uninstall_y = APPS_UNINSTALL_TARGET.y + APPS_UNINSTALL_TARGET.height / 2
            - model.effective_page_scroll_offset_px();
        assert_eq!(hit_test(model, uninstall_x, uninstall_y), None);

        assert!(model.begin_android_install(update.candidate_id));
        assert!(model.mark_android_install_pending(9, update.candidate_id));
        assert!(model.finish_android_install(9, update, Err(AndroidInstallFailure::Stale)));
        assert_eq!(
            model.android_install,
            AndroidInstallStatus::Failed {
                request_sequence: 9,
                candidate_id: update.candidate_id,
                failure: AndroidInstallFailure::Stale,
            }
        );
        assert!(model.cancel_android_install());
        assert_eq!(model.android_install, AndroidInstallStatus::Idle);
    }

    #[test]
    fn installed_launch_proof_is_compact_separate_and_catalog_bound() {
        assert!(
            core::mem::size_of::<AndroidInstalledLaunchStatus>() <= 64,
            "launch state must not duplicate bounded publisher strings"
        );
        let mut model = MobileModel::for_page(MobilePage::Home);
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );
        assert!(!model.begin_android_installed_launch(1, 7));
        assert!(model.apply_android_installed_app_status(installed_android_app(7)));
        assert!(!model.begin_android_installed_launch(0, 7));
        assert!(!model.begin_android_installed_launch(1, 8));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );

        assert!(model.begin_android_installed_launch(11, 7));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Pending {
                request_sequence: 11,
                generation: 7,
                apk_sha256: [0xa5; 32],
            }
        );
        assert!(!model.installed_android_launch_content_ready());
        assert!(!model.succeed_android_installed_launch(10, 7, [0xa5; 32]));
        assert!(!model.succeed_android_installed_launch(11, 8, [0xa5; 32]));
        assert!(!model.succeed_android_installed_launch(11, 7, [0xb6; 32]));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Pending {
                request_sequence: 11,
                generation: 7,
                apk_sha256: [0xa5; 32],
            }
        );

        assert!(model.succeed_android_installed_launch(11, 7, [0xa5; 32]));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Succeeded {
                request_sequence: 11,
                generation: 7,
                apk_sha256: [0xa5; 32],
            }
        );
        assert!(model.installed_android_launch_content_ready());
        assert!(!model.succeed_android_installed_launch(11, 7, [0xa5; 32]));

        assert!(model.apply_android_installed_app_status(installed_android_app(8)));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );
        assert!(!model.installed_android_launch_content_ready());
        assert!(model.begin_android_installed_launch(12, 8));
        let mut replaced_digest = installed_android_app(8);
        replaced_digest.apk_digest_sha256 = [0xc7; 32];
        assert!(model.apply_android_installed_app_status(replaced_digest));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );
        assert!(model.begin_android_installed_launch(13, 8));
        assert!(model.apply_android_installed_app_status(AndroidInstalledAppStatus::empty()));
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );
    }

    #[test]
    fn installed_launch_failure_accepts_only_the_current_pending_request() {
        for (sequence, failure) in [
            (21, AndroidInstalledLaunchFailure::Stale),
            (22, AndroidInstalledLaunchFailure::Verification),
            (23, AndroidInstalledLaunchFailure::Unsupported),
            (24, AndroidInstalledLaunchFailure::Unavailable),
        ] {
            let mut model = MobileModel {
                android_installed_app: installed_android_app(7),
                ..MobileModel::for_page(MobilePage::AndroidDemo)
            };
            assert!(model.begin_android_installed_launch(sequence, 7));
            assert!(!model.fail_android_installed_launch(sequence + 1, 7, failure));
            assert!(!model.fail_android_installed_launch(sequence, 8, failure));
            assert!(model.fail_android_installed_launch(sequence, 7, failure));
            assert_eq!(
                model.android_installed_launch,
                AndroidInstalledLaunchStatus::Failed {
                    request_sequence: sequence,
                    generation: 7,
                    apk_sha256: [0xa5; 32],
                    failure,
                }
            );
            assert!(!model.installed_android_launch_content_ready());
            assert!(!model.fail_android_installed_launch(sequence, 7, failure));
        }
        assert_eq!(
            installed_android_launch_copy(&AndroidInstalledLaunchStatus::Failed {
                request_sequence: 1,
                generation: 7,
                apk_sha256: [0xa5; 32],
                failure: AndroidInstalledLaunchFailure::Stale,
            })
            .0,
            "Launch request is stale"
        );
        assert_eq!(
            installed_android_launch_copy(&AndroidInstalledLaunchStatus::Failed {
                request_sequence: 2,
                generation: 7,
                apk_sha256: [0xa5; 32],
                failure: AndroidInstalledLaunchFailure::Verification,
            })
            .0,
            "APK verification failed"
        );
        assert_eq!(
            installed_android_launch_copy(&AndroidInstalledLaunchStatus::Failed {
                request_sequence: 3,
                generation: 7,
                apk_sha256: [0xa5; 32],
                failure: AndroidInstalledLaunchFailure::Unsupported,
            })
            .0,
            "App profile unsupported"
        );
        assert_eq!(
            installed_android_launch_copy(&AndroidInstalledLaunchStatus::Failed {
                request_sequence: 4,
                generation: 7,
                apk_sha256: [0xa5; 32],
                failure: AndroidInstalledLaunchFailure::Unavailable,
            })
            .0,
            "Installed APK unavailable"
        );
    }

    #[test]
    fn installed_activity_content_requires_a_matching_success_token() {
        let first_catalog = installed_android_app_with_text(7, "FIRST OLD TEXT");
        let second_catalog = installed_android_app_with_text(7, "SECOND OLD TEXT");

        let mut pending_first = MobileModel {
            android_installed_app: first_catalog,
            ..MobileModel::for_page(MobilePage::AndroidDemo)
        };
        assert!(pending_first.begin_android_installed_launch(31, 7));
        let mut pending_second = pending_first;
        assert!(pending_second.apply_android_installed_app_status(second_catalog));
        assert_eq!(render_model(pending_first), render_model(pending_second));

        let mut failed_first = pending_first;
        assert!(failed_first.fail_android_installed_launch(
            31,
            7,
            AndroidInstalledLaunchFailure::Verification,
        ));
        let mut failed_second = failed_first;
        assert!(failed_second.apply_android_installed_app_status(second_catalog));
        assert_eq!(render_model(failed_first), render_model(failed_second));

        let mut succeeded_first = pending_first;
        assert!(succeeded_first.succeed_android_installed_launch(31, 7, [0xa5; 32]));
        let first_identity = UiCompatibleActivityIdentity::new(31, 7).unwrap();
        assert!(succeeded_first.bind_android_installed_activity_session(first_identity));
        assert!(succeeded_first.apply_system_ui_state_recent(
            UiSystemUiMode::Foreground,
            Some(UiRecentIdentity::CompatibleAndroid(first_identity)),
            false,
            0,
            2,
        ));
        let mut succeeded_second = MobileModel {
            android_installed_app: second_catalog,
            ..MobileModel::for_page(MobilePage::AndroidDemo)
        };
        assert!(succeeded_second.begin_android_installed_launch(32, 7));
        assert!(succeeded_second.succeed_android_installed_launch(32, 7, [0xa5; 32]));
        let second_identity = UiCompatibleActivityIdentity::new(32, 7).unwrap();
        assert!(succeeded_second.bind_android_installed_activity_session(second_identity));
        assert!(succeeded_second.apply_system_ui_state_recent(
            UiSystemUiMode::Foreground,
            Some(UiRecentIdentity::CompatibleAndroid(second_identity)),
            false,
            0,
            2,
        ));
        assert!(succeeded_first.installed_android_launch_content_ready());
        assert!(succeeded_second.installed_android_launch_content_ready());
        assert!(succeeded_first.installed_android_foreground_content_ready());
        assert!(succeeded_second.installed_android_foreground_content_ready());
        assert_ne!(
            render_model(succeeded_first),
            render_model(succeeded_second)
        );

        let mut inconsistent_first = succeeded_first;
        inconsistent_first.android_installed_app.apk_digest_sha256 = [0xd8; 32];
        let mut inconsistent_second = inconsistent_first;
        inconsistent_second.android_installed_app.text =
            AndroidInstalledText::from_ascii("HIDDEN MISMATCH").unwrap();
        assert!(!inconsistent_first.installed_android_launch_content_ready());
        assert!(!inconsistent_second.installed_android_launch_content_ready());
        assert_eq!(
            render_model(inconsistent_first),
            render_model(inconsistent_second)
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn installed_activity_view_state_is_bounded_unique_revisioned_and_clearable() {
        assert!(
            AndroidInstalledActivityViewState::try_new(0, "Label", 2, "Tap", true, 1).is_none()
        );
        assert!(
            AndroidInstalledActivityViewState::try_new(1, "Label", 0, "Tap", true, 1).is_none()
        );
        assert!(
            AndroidInstalledActivityViewState::try_new(1, "Label", 1, "Tap", true, 1).is_none()
        );
        assert!(AndroidInstalledActivityViewState::try_new(1, "Label", 2, "", true, 1).is_none());
        assert!(
            AndroidInstalledActivityViewState::try_new(1, "Label", 2, "Tap", true, 0).is_none()
        );
        assert!(
            AndroidInstalledActivityViewState::try_new(1, "Lébel", 2, "Tap", true, 1).is_none()
        );
        let oversized_label = "A".repeat(ANDROID_INSTALLED_VIEW_LABEL_CAPACITY + 1);
        assert!(
            AndroidInstalledActivityViewState::try_new(1, &oversized_label, 2, "Tap", true, 1)
                .is_none()
        );

        let mut unavailable = MobileModel::for_page(MobilePage::AndroidDemo);
        assert!(!unavailable.set_android_installed_activity_view(1, "Label", 2, "Tap", true, 1));
        assert_eq!(
            unavailable.android_installed_activity_view,
            AndroidInstalledActivityViewState::empty()
        );

        let mut model = installed_android_foreground_model();
        assert!(model.set_android_installed_activity_view(1, "Initial", 2, "Tap", false, 1));
        assert!(!model.installed_android_interactive_content_ready());
        let staged = model.android_installed_activity_view;
        assert!(!model.set_android_installed_activity_view(
            1,
            "Revision replay",
            2,
            "Tap",
            true,
            1
        ));
        assert_eq!(model.android_installed_activity_view, staged);
        assert!(model.set_android_installed_activity_view(1, "Callback ready", 2, "Tap", true, 2));
        assert!(model.installed_android_interactive_content_ready());
        assert_eq!(model.android_installed_activity_view.revision(), 2);
        assert_eq!(model.android_installed_activity_view.label_view_id(), 1);
        assert_eq!(model.android_installed_activity_view.button_view_id(), 2);
        assert_eq!(
            model.android_installed_activity_view.label_text(),
            "Callback ready"
        );
        assert_eq!(model.android_installed_activity_view.button_text(), "Tap");
        assert!(model.android_installed_activity_view.callback_registered());
        assert!(model.clear_android_installed_activity_view());
        assert_eq!(
            model.android_installed_activity_view,
            AndroidInstalledActivityViewState::empty()
        );
        assert!(!model.clear_android_installed_activity_view());
    }

    #[cfg(all(feature = "mobile-ui-runtime", feature = "androidbox-interactive0"))]
    #[test]
    fn installed_activity_callback_uses_two_exact_disjoint_damage_regions() {
        let previous = interactive_installed_model("Ready for callback", 1);
        let mut current = previous;
        assert!(current.set_android_installed_activity_view(
            0x7f01_0001,
            "Button callback executed",
            0x7f01_0002,
            "Tap me",
            true,
            2,
        ));
        let expected = DamageRegions::try_new(&[
            INSTALLED_ANDROID_BUTTON_DAMAGE,
            INSTALLED_ANDROID_LABEL_DAMAGE,
        ])
        .unwrap();
        assert_eq!(
            damage_plan(Some(previous), current),
            MobileDamagePlan::Regions(expected)
        );

        let current_pixels = render_model(current);
        let mut damaged = render_model(previous);
        render_damage_regions(&mut damaged, current, expected).unwrap();
        assert_eq!(damaged, current_pixels);

        let sentinel = 0x00de_adbe;
        let mut isolated = vec![sentinel; PIXEL_COUNT];
        render_damage_regions(&mut isolated, current, expected).unwrap();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let inside = expected.rects().iter().any(|rect| {
                    (usize::from(rect.x)..usize::from(rect.x + rect.width)).contains(&x)
                        && (usize::from(rect.y)..usize::from(rect.y + rect.height)).contains(&y)
                });
                assert_eq!(
                    isolated[y * WIDTH + x],
                    if inside {
                        current_pixels[y * WIDTH + x]
                    } else {
                        sentinel
                    },
                    "installed Activity damage pixel {x}/{y}"
                );
            }
        }
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn installed_activity_button_hit_release_and_press_feedback_are_exact() {
        let mut model = interactive_installed_model("Ready", 1);
        let target = INSTALLED_ANDROID_BUTTON_TARGET;
        let view_id = model.android_installed_activity_view.button_view_id();
        let pressed_target = MobilePressedTarget::InstalledAndroidButton(view_id);
        assert_eq!(hit_test(model, target.x, target.y), Some(pressed_target));
        assert_eq!(
            hit_test(
                model,
                target.x + target.width - 1,
                target.y + target.height - 1,
            ),
            Some(pressed_target)
        );
        assert_eq!(hit_test(model, target.x + target.width, target.y), None);
        assert_eq!(hit_test(model, target.x, target.y + target.height), None);

        let stable = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(
            touch.observe(model, target.x + 8, target.y + 8, true),
            Some(MobileAction::SetPressed(Some(pressed_target)))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(pressed_target))));
        let pressed = render_model(model);
        assert_ne!(digest(&stable), digest(&pressed));
        assert_pixel_differences_are_bounded(
            &stable,
            &pressed,
            &[(
                usize::from(target.x),
                usize::from(target.y),
                usize::from(target.x + target.width),
                usize::from(target.y + target.height),
            )],
        );
        assert_eq!(
            touch.observe(
                model,
                target.x + target.width - 8,
                target.y + target.height - 8,
                false,
            ),
            Some(MobileAction::ActivateInstalledAndroidButton(view_id))
        );
        assert!(model.apply(MobileAction::ActivateInstalledAndroidButton(view_id)));
        assert_eq!(render_model(model), stable);

        let mut cancelled_model = interactive_installed_model("Ready", 1);
        let mut cancelled = TouchController::new();
        assert!(matches!(
            cancelled.observe(
                cancelled_model,
                target.x + target.width / 2,
                target.y + target.height / 2,
                true
            ),
            Some(MobileAction::SetPressed(Some(_)))
        ));
        assert!(cancelled_model.apply(MobileAction::SetPressed(Some(pressed_target))));
        assert_eq!(
            cancelled.observe(
                cancelled_model,
                target.x + target.width,
                target.y + target.height / 2,
                false,
            ),
            Some(MobileAction::SetPressed(None))
        );

        let mut inactive = installed_android_foreground_model();
        assert!(inactive.set_android_installed_activity_view(1, "Ready", 2, "Tap", false, 1));
        assert_eq!(
            hit_test(
                inactive,
                target.x + target.width / 2,
                target.y + target.height / 2,
            ),
            None
        );
        let mut background = interactive_installed_model("Ready", 1);
        let recent = background.system_ui_recent;
        assert!(background.apply_system_ui_state_recent(UiSystemUiMode::Home, recent, false, 0, 3));
        assert_eq!(
            hit_test(
                background,
                target.x + target.width / 2,
                target.y + target.height / 2,
            ),
            None
        );
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    fn installed_scene_button_uses_dynamic_geometry_and_has_priority_over_legacy_view() {
        let mut model = scene_installed_model(8, 7, true);
        let scene = model.android_installed_activity_scene;
        let callback_id = scene.callback_button_id().unwrap();
        let target = installed_android_scene_button_target(model, callback_id).unwrap();
        let pressed_target = MobilePressedTarget::InstalledAndroidButton(callback_id);
        assert_ne!(target, INSTALLED_ANDROID_BUTTON_TARGET);
        assert_eq!(hit_test(model, target.x, target.y), Some(pressed_target));
        assert_eq!(
            hit_test(
                model,
                target.x + target.width - 1,
                target.y + target.height - 1,
            ),
            Some(pressed_target)
        );
        assert_eq!(hit_test(model, target.x + target.width, target.y), None);
        assert_eq!(hit_test(model, target.x, target.y + target.height), None);

        let legacy_center_x =
            INSTALLED_ANDROID_BUTTON_TARGET.x + INSTALLED_ANDROID_BUTTON_TARGET.width / 2;
        let legacy_center_y =
            INSTALLED_ANDROID_BUTTON_TARGET.y + INSTALLED_ANDROID_BUTTON_TARGET.height / 2;
        assert!(!target.contains(legacy_center_x, legacy_center_y));
        assert_eq!(hit_test(model, legacy_center_x, legacy_center_y), None);

        let inert_index = scene.find_by_id(0x7f01_0004).unwrap().0;
        let inert_rect =
            installed_android_scene_node_rect(scene, usize::from(inert_index)).unwrap();
        let inert_x = u16::try_from((inert_rect.x + inert_rect.width / 2) * SCALE).unwrap();
        let inert_y = u16::try_from((inert_rect.y + inert_rect.height / 2) * SCALE).unwrap();
        assert_eq!(hit_test(model, inert_x, inert_y), None);

        let stable = render_model(model);
        let mut touch = TouchController::new();
        let center_x = target.x + target.width / 2;
        let center_y = target.y + target.height / 2;
        assert_eq!(
            touch.observe(model, center_x, center_y, true),
            Some(MobileAction::SetPressed(Some(pressed_target)))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(pressed_target))));
        let pressed = render_model(model);
        assert_pixel_differences_are_bounded(
            &stable,
            &pressed,
            &[(
                usize::from(target.x),
                usize::from(target.y),
                usize::from(target.x + target.width),
                usize::from(target.y + target.height),
            )],
        );
        assert_eq!(
            touch.observe(model, center_x, center_y, false),
            Some(MobileAction::ActivateInstalledAndroidButton(callback_id))
        );
        assert!(model.apply(MobileAction::ActivateInstalledAndroidButton(callback_id)));
        assert_eq!(render_model(model), stable);
    }

    #[cfg(feature = "androidbox-multiaction3")]
    #[test]
    fn installed_scene_routes_two_callback_buttons_to_distinct_targets() {
        let mut nodes = installed_android_scene_nodes();
        nodes[5] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::Button,
            Some(3),
            0x7f01_0004,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Reject profile",
            true,
        )
        .unwrap();
        let mut model = installed_android_foreground_model();
        assert!(model.set_android_installed_activity_scene(&nodes[..6], 1));
        let scene = model.android_installed_activity_scene;
        assert_eq!(scene.callback_button_count(), 2);

        let first = installed_android_scene_button_target(model, 0x7f01_0002).unwrap();
        let second = installed_android_scene_button_target(model, 0x7f01_0004).unwrap();
        assert_ne!(first, second);
        for (id, target) in [(0x7f01_0002, first), (0x7f01_0004, second)] {
            let center_x = target.x + target.width / 2;
            let center_y = target.y + target.height / 2;
            assert_eq!(
                hit_test(model, center_x, center_y),
                Some(MobilePressedTarget::InstalledAndroidButton(id))
            );
            let mut touch = TouchController::new();
            assert_eq!(
                touch.observe(model, center_x, center_y, true),
                Some(MobileAction::SetPressed(Some(
                    MobilePressedTarget::InstalledAndroidButton(id)
                )))
            );
            assert!(model.apply(MobileAction::SetPressed(Some(
                MobilePressedTarget::InstalledAndroidButton(id)
            ))));
            assert_eq!(
                touch.observe(model, center_x, center_y, false),
                Some(MobileAction::ActivateInstalledAndroidButton(id))
            );
            assert!(model.apply(MobileAction::SetPressed(None)));
        }
    }

    #[cfg(all(
        feature = "androidbox-layout-row14",
        not(feature = "androidbox-layout-spacing16")
    ))]
    #[test]
    fn installed_scene_horizontal_row_shares_y_and_splits_exact_click_geometry() {
        let mut model = installed_android_foreground_model();
        let nodes = installed_android_row_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;

        let title = installed_android_scene_node_rect(scene, 1).unwrap();
        let status = installed_android_scene_node_rect(scene, 2).unwrap();
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!(
            (title.x, title.y, title.width, title.height),
            (34, 216, 292, 48)
        );
        assert_eq!(
            (status.x, status.y, status.width, status.height),
            (34, 268, 292, 48)
        );
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (34, 320, 142, 100)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (180, 320, 142, 100)
        );
        for (index, rect) in [(4, first), (5, second)] {
            let role = installed_android_scene_node_text_role(scene, index);
            let text = scene.nodes()[index].text();
            let available_width_px = u16::try_from((rect.width - 20) * SCALE).unwrap();
            assert_eq!(role, MobileTextRole::Label);
            assert_eq!(
                fitted_mobile_text(text, role, available_width_px),
                text,
                "horizontal button text must not be clipped"
            );
            assert!(measure_mobile_text_px(role, text) <= available_width_px);
        }

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (68, 640, 284, 200)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (360, 640, 284, 200)
        );
        for (id, target) in [(0x7f02_0000, first_target), (0x7f02_0001, second_target)] {
            assert_eq!(
                hit_test(
                    model,
                    target.x + target.width / 2,
                    target.y + target.height / 2,
                ),
                Some(MobilePressedTarget::InstalledAndroidButton(id))
            );
        }
        assert_eq!(hit_test(model, 356, 740), None);
        assert_ne!(digest(&render_model(model)), 0);
    }

    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-spacing16")
    ))]
    #[test]
    fn installed_scene_uses_apk_weights_for_unequal_geometry_and_hit_testing() {
        let mut nodes = installed_android_row_scene_nodes();
        nodes[4] = AndroidInstalledActivitySceneNode::try_new_weighted(
            AndroidSceneViewKind::Button,
            Some(3),
            0x7f02_0000,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Components",
            true,
            1,
        )
        .unwrap();
        nodes[5] = AndroidInstalledActivitySceneNode::try_new_weighted(
            AndroidSceneViewKind::Button,
            Some(3),
            0x7f02_0001,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Permissions",
            true,
            2,
        )
        .unwrap();

        let mut model = installed_android_foreground_model();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (34, 320, 93, 100)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (131, 320, 191, 100)
        );

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (68, 640, 186, 200)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (262, 640, 382, 200)
        );
        assert_eq!(hit_test(model, 258, 740), None);
        assert_eq!(
            hit_test(model, 161, 740),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0000))
        );
        assert_eq!(
            hit_test(model, 453, 740),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0001))
        );
    }

    #[cfg(all(
        feature = "androidbox-layout-spacing16",
        not(feature = "androidbox-layout-directional17")
    ))]
    #[test]
    fn installed_scene_applies_apk_padding_and_margins_to_raster_and_hits() {
        let mut model = installed_android_foreground_model();
        let nodes = installed_android_row_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;

        let row = installed_android_scene_node_area(scene, 3, 0).unwrap();
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!((row.x, row.y, row.width, row.height), (34, 320, 292, 100));
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (40, 326, 138, 88)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (182, 326, 138, 88)
        );
        assert_eq!(scene.nodes()[3].padding_dp(), 4);
        assert_eq!(scene.nodes()[4].layout_margin_dp(), 2);
        assert_eq!(scene.nodes()[5].layout_margin_dp(), 2);

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (80, 652, 276, 176)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (364, 652, 276, 176)
        );
        assert_eq!(hit_test(model, 360, 740), None);
        assert_eq!(
            hit_test(model, 218, 740),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0000))
        );
        assert_eq!(
            hit_test(model, 502, 740),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0001))
        );
    }

    #[cfg(all(
        feature = "androidbox-layout-directional17",
        not(feature = "androidbox-layout-size18")
    ))]
    #[test]
    fn installed_scene_applies_each_apk_spacing_edge_to_raster_and_hits() {
        let mut model = installed_android_foreground_model();
        let nodes = installed_android_row_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;

        let row = installed_android_scene_node_area(scene, 3, 0).unwrap();
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!((row.x, row.y, row.width, row.height), (34, 320, 292, 100));
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (42, 325, 135, 84)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (187, 329, 135, 82)
        );
        assert_eq!(
            (
                scene.nodes()[3].padding_left_dp(),
                scene.nodes()[3].padding_top_dp(),
                scene.nodes()[3].padding_right_dp(),
                scene.nodes()[3].padding_bottom_dp(),
            ),
            (6, 4, 2, 8)
        );
        assert_eq!(
            (
                scene.nodes()[4].layout_margin_left_dp(),
                scene.nodes()[4].layout_margin_top_dp(),
                scene.nodes()[4].layout_margin_right_dp(),
                scene.nodes()[4].layout_margin_bottom_dp(),
            ),
            (2, 1, 4, 3)
        );
        assert_eq!(
            (
                scene.nodes()[5].layout_margin_left_dp(),
                scene.nodes()[5].layout_margin_top_dp(),
                scene.nodes()[5].layout_margin_right_dp(),
                scene.nodes()[5].layout_margin_bottom_dp(),
            ),
            (6, 5, 2, 1)
        );

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (84, 650, 270, 168)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (374, 658, 270, 164)
        );
        assert_eq!(hit_test(model, 360, 740), None);
        assert_eq!(
            hit_test(model, 219, 734),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0000))
        );
        assert_eq!(
            hit_test(model, 509, 740),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0001))
        );
    }

    #[cfg(all(
        feature = "androidbox-layout-size18",
        not(feature = "androidbox-layout-mixed19")
    ))]
    #[test]
    fn installed_scene_applies_exact_dp_widths_and_heights_to_raster_and_hits() {
        let mut model = installed_android_foreground_model();
        let nodes = installed_android_row_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;

        let title = installed_android_scene_node_rect(scene, 1).unwrap();
        let status = installed_android_scene_node_rect(scene, 2).unwrap();
        let row = installed_android_scene_node_area(scene, 3, 0).unwrap();
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!(
            (title.x, title.y, title.width, title.height),
            (34, 216, 240, 40)
        );
        assert_eq!(
            (status.x, status.y, status.width, status.height),
            (34, 260, 292, 40)
        );
        assert_eq!((row.x, row.y, row.width, row.height), (34, 304, 292, 120));
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (42, 309, 135, 64)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (187, 313, 135, 56)
        );
        assert_eq!(
            (
                scene.nodes()[1].exact_width_dp(),
                scene.nodes()[3].exact_height_dp(),
                scene.nodes()[4].exact_height_dp(),
                scene.nodes()[5].exact_height_dp(),
            ),
            (240, 120, 64, 56)
        );

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (84, 618, 270, 128)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (374, 626, 270, 112)
        );
        assert_eq!(hit_test(model, 360, 682), None);
        assert_eq!(
            hit_test(model, 219, 682),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0000))
        );
        assert_eq!(
            hit_test(model, 509, 682),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0001))
        );
    }

    #[cfg(feature = "androidbox-layout-mixed19")]
    #[test]
    fn installed_scene_mixes_fixed_and_weighted_widths_using_remaining_space() {
        let mut model = installed_android_foreground_model();
        let nodes = installed_android_row_scene_nodes();
        assert!(model.set_android_installed_activity_scene(&nodes, 1));
        let scene = model.android_installed_activity_scene;

        let row = installed_android_scene_node_area(scene, 3, 0).unwrap();
        let first = installed_android_scene_node_rect(scene, 4).unwrap();
        let second = installed_android_scene_node_rect(scene, 5).unwrap();
        assert_eq!((row.x, row.y, row.width, row.height), (34, 304, 292, 120));
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (42, 309, 132, 64)
        );
        assert_eq!(
            (second.x, second.y, second.width, second.height),
            (184, 313, 138, 56)
        );
        for (index, rect) in [(4, first), (5, second)] {
            let role = installed_android_scene_node_text_role(scene, index);
            let text = scene.nodes()[index].text();
            let available_width_px = u16::try_from((rect.width - 20) * SCALE).unwrap();
            assert_eq!(role, MobileTextRole::Label);
            assert_eq!(fitted_mobile_text(text, role, available_width_px), text);
            assert!(measure_mobile_text_px(role, text) <= available_width_px);
        }
        assert_eq!(
            (
                scene.nodes()[4].width(),
                scene.nodes()[4].layout_weight(),
                scene.nodes()[4].exact_width_dp(),
                scene.nodes()[5].width(),
                scene.nodes()[5].layout_weight(),
                scene.nodes()[5].exact_width_dp(),
            ),
            (
                AndroidSceneLayoutSize::Exact,
                0,
                132,
                AndroidSceneLayoutSize::Zero,
                1,
                0,
            )
        );

        let first_target = installed_android_scene_button_target(model, 0x7f02_0000).unwrap();
        let second_target = installed_android_scene_button_target(model, 0x7f02_0001).unwrap();
        assert_eq!(
            (
                first_target.x,
                first_target.y,
                first_target.width,
                first_target.height,
            ),
            (84, 618, 264, 128)
        );
        assert_eq!(
            (
                second_target.x,
                second_target.y,
                second_target.width,
                second_target.height,
            ),
            (368, 626, 276, 112)
        );
        assert_eq!(hit_test(model, 358, 682), None);
        assert_eq!(
            hit_test(model, 216, 682),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0000))
        );
        assert_eq!(
            hit_test(model, 506, 682),
            Some(MobilePressedTarget::InstalledAndroidButton(0x7f02_0001))
        );
    }

    #[cfg(feature = "androidbox-scene-rpc2")]
    #[test]
    fn installed_scene_renderer_handles_every_admitted_prefix_and_skips_layout_surfaces() {
        let mut previous_frame: Option<(usize, Vec<u32>)> = None;
        for node_count in 3..=8 {
            let model = scene_installed_model(node_count, 1, false);
            let scene = model.android_installed_activity_scene;
            assert_eq!(usize::from(scene.len()), node_count);
            #[cfg(not(feature = "androidbox-layout-row14"))]
            let mut previous_leaf_bottom = None;
            #[cfg(feature = "androidbox-layout-row14")]
            let mut previous_leaf_rects: Vec<DRect> = Vec::new();
            for (index, node) in scene.nodes().iter().copied().enumerate() {
                let rect = installed_android_scene_node_rect(scene, index);
                if node.kind() == AndroidSceneViewKind::LinearLayout {
                    assert!(rect.is_none(), "layout {index} must remain nonvisual");
                    continue;
                }
                let rect = rect.unwrap();
                assert!(rect.x >= 34);
                assert!(rect.y >= 216);
                assert!(rect.x + rect.width <= 326);
                assert!(rect.y + rect.height <= 424);
                #[cfg(not(feature = "androidbox-layout-row14"))]
                if let Some(previous_bottom) = previous_leaf_bottom {
                    assert!(previous_bottom <= rect.y);
                }
                #[cfg(feature = "androidbox-layout-row14")]
                {
                    for previous in &previous_leaf_rects {
                        assert!(
                            rect.x >= previous.x + previous.width
                                || previous.x >= rect.x + rect.width
                                || rect.y >= previous.y + previous.height
                                || previous.y >= rect.y + rect.height
                        );
                    }
                    previous_leaf_rects.push(rect);
                }
                #[cfg(not(feature = "androidbox-layout-row14"))]
                {
                    previous_leaf_bottom = Some(rect.y + rect.height);
                }
            }

            let frame = render_model(model);
            if let Some((previous_count, previous)) = previous_frame.as_ref() {
                if node_count == 4 {
                    // The fourth node is a nested LinearLayout. It changes
                    // hierarchy only and therefore contributes no pixels.
                    assert_eq!(&frame, previous);
                } else {
                    assert_ne!(
                        digest(&frame),
                        digest(previous),
                        "{previous_count}/{node_count}"
                    );
                }
            }
            previous_frame = Some((node_count, frame));
        }

        let initial = scene_installed_model(8, 1, false);
        let mut revision_only = initial;
        let nodes = installed_android_scene_nodes();
        assert!(revision_only.set_android_installed_activity_scene(&nodes, 2));
        let initial_frame = render_model(initial);
        let revision_frame = render_model(revision_only);
        assert_eq!(
            initial_frame, revision_frame,
            "an internal Scene-RPC revision must not become publisher-visible pixels"
        );

        let mut updated = revision_only;
        let text_index = updated
            .android_installed_activity_scene
            .find_by_id(0x7f01_0001)
            .unwrap()
            .0;
        let text_rect = installed_android_scene_node_rect(
            updated.android_installed_activity_scene,
            usize::from(text_index),
        )
        .unwrap();
        assert!(updated.update_android_installed_activity_scene_text(
            0x7f01_0001,
            "Account verified",
            3,
        ));
        let updated_frame = render_model(updated);
        assert_pixel_differences_are_bounded(
            &revision_frame,
            &updated_frame,
            &[(
                usize::try_from(text_rect.x * SCALE).unwrap(),
                usize::try_from(text_rect.y * SCALE).unwrap(),
                usize::try_from((text_rect.x + text_rect.width) * SCALE).unwrap(),
                usize::try_from((text_rect.y + text_rect.height) * SCALE).unwrap(),
            )],
        );

        let chrome = split_chrome_state(initial);
        let initial_split = render_split_frame(initial, chrome);
        let updated_split = render_split_frame(updated, chrome);
        let content_start = usize::from(MOBILE_CONTENT_TOP) * WIDTH;
        let content_end = usize::from(MOBILE_CONTENT_BOTTOM) * WIDTH;
        assert_eq!(
            &initial_split[..content_start],
            &updated_split[..content_start]
        );
        assert_eq!(&initial_split[content_end..], &updated_split[content_end..]);
        assert_ne!(
            &initial_split[content_start..content_end],
            &updated_split[content_start..content_end]
        );

        let complete = render_model(updated);
        let mut rows = vec![0_u32; PIXEL_COUNT];
        let mut first_row = 0;
        while first_row < HEIGHT {
            let row_count = (HEIGHT - first_row).min(17);
            let start = first_row * WIDTH;
            let end = start + row_count * WIDTH;
            render_rows(&mut rows[start..end], first_row, updated).unwrap();
            first_row += row_count;
        }
        assert_eq!(rows, complete);
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn installed_activity_keeps_package_diagnostics_in_settings_only() {
        let activity = interactive_installed_model("Ready", 1);
        let original = activity.android_installed_app;
        let mut changed = activity;
        changed.android_installed_app.package =
            AndroidInstalledPackage::from_ascii("org.example.private").unwrap();
        changed.android_installed_app.activity =
            AndroidInstalledActivity::from_ascii("org.example.private.HiddenActivity").unwrap();
        changed.android_installed_app.text =
            AndroidInstalledText::from_ascii("Different legacy payload").unwrap();
        changed.android_installed_app.version_code = 88;
        changed.android_installed_app.apk_length = 99_999;
        changed.android_installed_app.signer_digest_sha256 = [0x3c; 32];

        assert_eq!(
            render_model(activity),
            render_model(changed),
            "package diagnostics must not leak over publisher Activity content"
        );

        let original_settings = MobileModel {
            android_installed_app: original,
            ..MobileModel::for_page(MobilePage::Apps)
        };
        let changed_settings = MobileModel {
            android_installed_app: changed.android_installed_app,
            ..MobileModel::for_page(MobilePage::Apps)
        };
        assert_ne!(
            render_model(original_settings),
            render_model(changed_settings),
            "Settings/Apps must retain the moved package diagnostics"
        );
    }

    #[cfg(feature = "androidbox-interactive0")]
    #[test]
    fn installed_activity_view_revision_changes_only_content_and_not_trusted_chrome() {
        let initial = interactive_installed_model("Ready", 1);
        let mut clicked = initial;
        assert!(clicked.set_android_installed_activity_view(
            0x7f01_0001,
            "Tapped 1",
            0x7f01_0002,
            "Tap me",
            true,
            2,
        ));

        let content_start = usize::from(MOBILE_CONTENT_TOP) * WIDTH;
        let content_end = usize::from(MOBILE_CONTENT_BOTTOM) * WIDTH;
        let mut initial_content = vec![0_u32; content_end - content_start];
        let mut clicked_content = vec![0_u32; content_end - content_start];
        render_content_rows(
            &mut initial_content,
            usize::from(MOBILE_CONTENT_TOP),
            initial,
        )
        .unwrap();
        render_content_rows(
            &mut clicked_content,
            usize::from(MOBILE_CONTENT_TOP),
            clicked,
        )
        .unwrap();
        assert_ne!(digest(&initial_content), digest(&clicked_content));

        let chrome = split_chrome_state(initial);
        let initial_frame = render_split_frame(initial, chrome);
        let clicked_frame = render_split_frame(clicked, chrome);
        assert_eq!(
            &initial_frame[..content_start],
            &clicked_frame[..content_start]
        );
        assert_eq!(&initial_frame[content_end..], &clicked_frame[content_end..]);
        assert!(
            initial_frame[content_start..content_end]
                .iter()
                .zip(clicked_frame[content_start..content_end].iter())
                .any(|(left, right)| left != right)
        );
        assert!(
            initial_frame
                .iter()
                .zip(clicked_frame.iter())
                .enumerate()
                .filter(|(_, (left, right))| left != right)
                .all(|(index, _)| (content_start..content_end).contains(&index))
        );
    }

    #[test]
    fn compatible_activity_home_overview_relaunch_and_finish_are_fail_closed() {
        let catalog = installed_android_app_with_text(7, "VISIBLE FOREGROUND TEXT");
        let identity = UiCompatibleActivityIdentity::new(41, 7).unwrap();
        let recent = Some(UiRecentIdentity::CompatibleAndroid(identity));
        let mut model = MobileModel {
            android_installed_app: catalog,
            ..MobileModel::for_page(MobilePage::Home)
        };

        assert!(model.begin_android_installed_launch(41, 7));
        assert!(model.succeed_android_installed_launch(41, 7, [0xa5; 32]));
        assert!(model.bind_android_installed_activity_session(identity));
        assert!(!model.installed_android_foreground_content_ready());

        assert!(model.apply_system_ui_state_recent(
            UiSystemUiMode::Foreground,
            recent,
            false,
            0,
            2,
        ));
        assert!(model.apply(MobileAction::Open(MobilePage::AndroidDemo)));
        assert!(model.installed_android_foreground_content_ready());
        let foreground = render_model(model);

        assert!(model.apply_system_ui_state_recent(UiSystemUiMode::Home, recent, false, 0, 3,));
        assert_eq!(model.page, MobilePage::Home);
        assert!(model.compatible_android_recent_ready());
        assert!(!model.installed_android_foreground_content_ready());
        assert_ne!(render_model(model), foreground);

        assert!(model.apply_system_ui_state_recent(UiSystemUiMode::Overview, recent, false, 0, 4,));
        let overview = render_model(model);
        let mut changed_activity_only = model;
        changed_activity_only.android_installed_app.text =
            AndroidInstalledText::from_ascii("DIFFERENT HIDDEN TEXT").unwrap();
        assert_eq!(
            render_model(changed_activity_only),
            overview,
            "Overview must not read publisher Activity content"
        );

        let target = OVERVIEW_RECENT_TARGET;
        let x = target.x + target.width / 2;
        let y = target.y + target.height / 2;
        let mut touch = TouchController::new();
        assert_eq!(
            touch.observe(model, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::OverviewRecent
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::OverviewRecent
        ))));
        let pressed = touch
            .observe(model, x, y, false)
            .expect("matching compatible recent must activate");
        assert_eq!(pressed, MobileAction::ActivateRecentApp);
        assert!(model.apply(MobileAction::SetPressed(None)));

        assert!(model.begin_android_installed_launch(42, 7));
        assert!(!model.installed_android_foreground_content_ready());
        assert_eq!(
            render_model(model),
            overview,
            "pending relaunch must not reveal cached Activity pixels"
        );
        assert!(model.fail_android_installed_launch(
            42,
            7,
            AndroidInstalledLaunchFailure::Verification,
        ));
        assert_eq!(
            render_model(model),
            overview,
            "failed relaunch must not reveal cached Activity pixels"
        );

        assert!(model.apply_system_ui_state_recent(UiSystemUiMode::Overview, None, false, 0, 5,));
        assert_eq!(model.android_installed_session, None);
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );
        assert!(!model.compatible_android_recent_ready());
        let mut empty_touch = TouchController::new();
        assert_eq!(empty_touch.observe(model, x, y, true), None);
        assert_eq!(empty_touch.observe(model, x, y, false), None);
    }

    #[test]
    fn compatible_overview_uses_package_derived_icon_fallback_not_generic_android_badge() {
        let catalog = installed_android_app(7);
        let expected = installed_app_fallback_color(catalog);
        let model = compatible_overview_model(catalog);
        let rendered = render_model(model);
        let sample_x = (180 - 17) * SCALE;
        let sample_y = 308 * SCALE;
        assert_eq!(
            rendered[sample_y as usize * WIDTH + sample_x as usize],
            expected,
        );
        assert_ne!(expected, COLOR_ANDROIDBOX);

        let mut changed_activity_text = model;
        changed_activity_text.android_installed_app.text =
            AndroidInstalledText::from_ascii("HIDDEN ACTIVITY PIXELS").unwrap();
        assert_eq!(
            render_model(changed_activity_text),
            rendered,
            "Overview may use package metadata but must never read Activity content"
        );
    }

    #[cfg(feature = "androidbox-icon-resources5")]
    #[test]
    fn compatible_overview_uses_exact_catalog_icon_only_for_matching_session() {
        let mut first_pixels = [0xff8e_24aa; ANDROID_INSTALLED_ICON_PIXEL_COUNT];
        first_pixels[1] = 0xffff_ffff;
        first_pixels[2] = 0;
        let first_icon =
            AndroidInstalledIcon::try_new(0x7f01_0000, 0x1357_2468, first_pixels).unwrap();
        let catalog = installed_android_app(7)
            .with_icon(Some(first_icon))
            .unwrap();
        let model = compatible_overview_model(catalog);
        let rendered = render_model(model);
        let icon_left = (180 - 24) * SCALE;
        let icon_top = (308 - 24) * SCALE;
        assert_eq!(
            rendered[icon_top as usize * WIDTH + icon_left as usize],
            0x008e_24aa,
        );

        let mut second_pixels = [0xff0b_57d0; ANDROID_INSTALLED_ICON_PIXEL_COUNT];
        second_pixels[1] = 0xffff_ffff;
        second_pixels[2] = 0;
        let second_icon =
            AndroidInstalledIcon::try_new(0x7f01_0000, 0x2468_1357, second_pixels).unwrap();
        let mut changed_icon = model;
        changed_icon.android_installed_app.icon = Some(second_icon);
        assert!(changed_icon.compatible_android_recent_ready());
        let changed = render_model(changed_icon);
        assert_pixel_differences_are_bounded(&rendered, &changed, &[(312, 568, 408, 664)]);
        assert_eq!(
            changed[icon_top as usize * WIDTH + icon_left as usize],
            0x000b_57d0,
        );

        let stale_identity = UiCompatibleActivityIdentity::new(42, 8).unwrap();
        let mut stale = model;
        assert!(stale.apply_system_ui_state_recent(
            UiSystemUiMode::Overview,
            Some(UiRecentIdentity::CompatibleAndroid(stale_identity)),
            false,
            0,
            3,
        ));
        assert!(!stale.compatible_android_recent_ready());
        assert_ne!(
            render_model(stale)[icon_top as usize * WIDTH + icon_left as usize],
            0x008e_24aa,
            "a stale recent identity must render the unavailable fallback"
        );
    }

    #[test]
    fn androidbox_render_path_keeps_the_large_model_snapshot_borrowed() {
        type BorrowedRenderer =
            for<'canvas, 'pixels, 'model> fn(&'canvas mut Canvas<'pixels>, &'model MobileModel);
        type BorrowedFact = for<'model> fn(&'model MobileModel) -> bool;

        let _: BorrowedRenderer = render_android_demo;
        let _: BorrowedRenderer = render_installed_android_app;
        let _: BorrowedFact = androidbox_launcher_manifest_verified;
        let _: BorrowedFact = androidbox_on_create_complete;
        let _: BorrowedFact = androidbox_activity_complete;
    }

    #[test]
    fn settings_display_apps_and_about_rows_are_exact_safe_and_navigate() {
        assert_eq!(
            SETTINGS_DISPLAY_TARGET.y + SETTINGS_DISPLAY_TARGET.height,
            540
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                SETTINGS_DISPLAY_TARGET.x,
                SETTINGS_DISPLAY_TARGET.y,
            ),
            Some(MobilePressedTarget::Display)
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                SETTINGS_DISPLAY_TARGET.x + SETTINGS_DISPLAY_TARGET.width - 1,
                SETTINGS_DISPLAY_TARGET.y + SETTINGS_DISPLAY_TARGET.height - 1,
            ),
            Some(MobilePressedTarget::Display)
        );
        assert_eq!(
            SETTINGS_APPS_TARGET.y + SETTINGS_APPS_TARGET.height,
            SETTINGS_ABOUT_TARGET.y
        );
        assert_eq!(
            SETTINGS_ABOUT_TARGET.y + SETTINGS_ABOUT_TARGET.height,
            SYSTEM_NAV_TOP_PX
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                SETTINGS_APPS_TARGET.x,
                SETTINGS_APPS_TARGET.y,
            ),
            Some(MobilePressedTarget::Apps)
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                SETTINGS_APPS_TARGET.x + SETTINGS_APPS_TARGET.width - 1,
                SETTINGS_APPS_TARGET.y + SETTINGS_APPS_TARGET.height - 1,
            ),
            Some(MobilePressedTarget::Apps)
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                SETTINGS_ABOUT_TARGET.x,
                SETTINGS_ABOUT_TARGET.y,
            ),
            Some(MobilePressedTarget::About)
        );
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                360,
                SYSTEM_NAV_TOP_PX,
            ),
            Some(MobilePressedTarget::SystemHome)
        );

        let x = SETTINGS_APPS_TARGET.x + SETTINGS_APPS_TARGET.width / 2;
        let y = SETTINGS_APPS_TARGET.y + SETTINGS_APPS_TARGET.height / 2;
        let mut model = MobileModel::for_page(MobilePage::Settings);
        let mut display_touch = TouchController::new();
        let display_x = SETTINGS_DISPLAY_TARGET.x + SETTINGS_DISPLAY_TARGET.width / 2;
        let display_y = SETTINGS_DISPLAY_TARGET.y + SETTINGS_DISPLAY_TARGET.height / 2;
        let down = display_touch
            .observe(model, display_x, display_y, true)
            .unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::Display))
        );
        assert!(model.apply(down));
        assert_eq!(
            display_touch.observe(model, display_x, display_y, false),
            Some(MobileAction::Open(MobilePage::Display))
        );
        assert!(model.apply(MobileAction::Open(MobilePage::Display)));
        assert!(model.has_in_app_back());
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);

        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::Apps))
        );
        assert!(model.apply(down));
        assert_eq!(
            touch.observe(model, x, y, false),
            Some(MobileAction::Open(MobilePage::Apps))
        );
        assert!(model.apply(MobileAction::Open(MobilePage::Apps)));
        assert_eq!(model.page, MobilePage::Apps);
        assert!(model.has_in_app_back());
        assert_eq!(
            hit_test(
                model,
                APPS_BACK_TARGET.x + APPS_BACK_TARGET.width / 2,
                APPS_BACK_TARGET.y + APPS_BACK_TARGET.height / 2,
            ),
            Some(MobilePressedTarget::Back)
        );
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);

        let mut apps = MobileModel::for_page(MobilePage::Apps);
        let mut edge = TouchController::new();
        assert_eq!(edge.observe(apps, 32, 800, true), None);
        let reveal = edge.observe(apps, 176, 800, true).unwrap();
        assert!(apps.apply(reveal));
        assert_eq!(
            edge.observe(apps, 176, 800, false),
            Some(MobileAction::Back)
        );
        assert!(apps.apply(MobileAction::Back));
        assert_eq!(apps.page, MobilePage::Settings);
    }

    #[test]
    fn display_controls_are_page_scoped_half_open_and_return_to_settings() {
        assert_eq!(
            DISPLAY_THEME_TARGET.y + DISPLAY_THEME_TARGET.height,
            DISPLAY_ACCENT_TARGET.y
        );
        let display = MobileModel::for_page(MobilePage::Display);
        for (target, pressed) in [
            (DISPLAY_THEME_TARGET, MobilePressedTarget::Theme),
            (DISPLAY_ACCENT_TARGET, MobilePressedTarget::Accent),
            (DISPLAY_DIMMING_TARGET, MobilePressedTarget::SoftwareDimming),
            (
                DISPLAY_ACCESSIBILITY_TARGET,
                MobilePressedTarget::Accessibility,
            ),
        ] {
            assert_eq!(hit_test(display, target.x, target.y), Some(pressed));
            assert_eq!(
                hit_test(
                    display,
                    target.x + target.width - 1,
                    target.y + target.height - 1,
                ),
                Some(pressed)
            );
            assert_ne!(
                hit_test(display, target.x + target.width, target.y),
                Some(pressed)
            );
        }
        assert_eq!(
            hit_test(
                MobileModel::for_page(MobilePage::Settings),
                DISPLAY_THEME_TARGET.x,
                DISPLAY_THEME_TARGET.y,
            ),
            Some(MobilePressedTarget::Display)
        );
        assert_eq!(page_scroll_max_px(MobilePage::Display), 0);

        let mut model = display;
        assert!(model.has_in_app_back());
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn accessibility_controls_are_exact_shared_reversible_and_geometry_neutral() {
        assert_eq!(
            DISPLAY_DIMMING_TARGET.y + DISPLAY_DIMMING_TARGET.height,
            856
        );
        assert_eq!(DISPLAY_ACCESSIBILITY_TARGET.y, 1_304);
        assert_eq!(
            DISPLAY_ACCESSIBILITY_TARGET.y + DISPLAY_ACCESSIBILITY_TARGET.height,
            1_496
        );
        assert_eq!(SYSTEM_NAV_TOP_PX, 1_548);
        assert_eq!(
            ACCESSIBILITY_LARGE_TEXT_TARGET.y + ACCESSIBILITY_LARGE_TEXT_TARGET.height,
            ACCESSIBILITY_HIGH_CONTRAST_TARGET.y
        );
        assert_eq!(
            ACCESSIBILITY_HIGH_CONTRAST_TARGET.y + ACCESSIBILITY_HIGH_CONTRAST_TARGET.height,
            672
        );

        let display = MobileModel::for_page(MobilePage::Display);
        let entry_x = DISPLAY_ACCESSIBILITY_TARGET.x + DISPLAY_ACCESSIBILITY_TARGET.width / 2;
        let entry_y = DISPLAY_ACCESSIBILITY_TARGET.y + DISPLAY_ACCESSIBILITY_TARGET.height / 2;
        let mut entry_touch = TouchController::new();
        assert_eq!(
            entry_touch.observe(display, entry_x, entry_y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::Accessibility
            )))
        );
        let mut model = display;
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::Accessibility,
        ))));
        assert_eq!(
            entry_touch.observe(model, entry_x, entry_y, false),
            Some(MobileAction::Open(MobilePage::Accessibility))
        );
        assert!(model.apply(MobileAction::Open(MobilePage::Accessibility)));
        assert_eq!(model.page, MobilePage::Accessibility);
        assert!(model.has_in_app_back());
        assert_eq!(page_scroll_max_px(MobilePage::Accessibility), 0);

        for (target, pressed) in [
            (
                ACCESSIBILITY_LARGE_TEXT_TARGET,
                MobilePressedTarget::LargeText,
            ),
            (
                ACCESSIBILITY_HIGH_CONTRAST_TARGET,
                MobilePressedTarget::HighContrast,
            ),
        ] {
            assert_eq!(hit_test(model, target.x, target.y), Some(pressed));
            assert_eq!(
                hit_test(
                    model,
                    target.x + target.width - 1,
                    target.y + target.height - 1,
                ),
                Some(pressed)
            );
            assert_ne!(
                hit_test(model, target.x + target.width, target.y),
                Some(pressed)
            );
        }

        let standard = model;
        let large_x = ACCESSIBILITY_LARGE_TEXT_TARGET.x + ACCESSIBILITY_LARGE_TEXT_TARGET.width / 2;
        let large_y =
            ACCESSIBILITY_LARGE_TEXT_TARGET.y + ACCESSIBILITY_LARGE_TEXT_TARGET.height / 2;
        let mut large_touch = TouchController::new();
        let down = large_touch.observe(model, large_x, large_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::LargeText))
        );
        assert!(model.apply(down));
        assert_eq!(
            large_touch.observe(model, large_x, large_y, false),
            Some(MobileAction::ToggleLargeText)
        );
        assert!(model.apply(MobileAction::ToggleLargeText));
        assert!(model.large_text);
        assert_eq!(damage_plan(Some(standard), model), MobileDamagePlan::Full);

        let large = model;
        let contrast_x =
            ACCESSIBILITY_HIGH_CONTRAST_TARGET.x + ACCESSIBILITY_HIGH_CONTRAST_TARGET.width / 2;
        let contrast_y =
            ACCESSIBILITY_HIGH_CONTRAST_TARGET.y + ACCESSIBILITY_HIGH_CONTRAST_TARGET.height / 2;
        let mut contrast_touch = TouchController::new();
        let down = contrast_touch
            .observe(model, contrast_x, contrast_y, true)
            .unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::HighContrast))
        );
        assert!(model.apply(down));
        assert_eq!(
            contrast_touch.observe(model, contrast_x, contrast_y, false),
            Some(MobileAction::ToggleHighContrast)
        );
        assert!(model.apply(MobileAction::ToggleHighContrast));
        assert!(model.high_contrast);
        assert_eq!(damage_plan(Some(large), model), MobileDamagePlan::Full);

        for (x, y) in [
            (
                ACCESSIBILITY_LARGE_TEXT_TARGET.x,
                ACCESSIBILITY_LARGE_TEXT_TARGET.y,
            ),
            (large_x, large_y),
            (
                ACCESSIBILITY_HIGH_CONTRAST_TARGET.x,
                ACCESSIBILITY_HIGH_CONTRAST_TARGET.y,
            ),
            (contrast_x, contrast_y),
            (360, SYSTEM_NAV_TOP_PX),
        ] {
            assert_eq!(
                hit_test(standard, x, y),
                hit_test(model, x, y),
                "appearance changed authority at {x}/{y}"
            );
        }

        assert!(model.apply(MobileAction::ToggleHighContrast));
        assert!(model.apply(MobileAction::ToggleLargeText));
        assert!(!model.large_text);
        assert!(!model.high_contrast);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Display);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
    }

    #[test]
    fn settings_and_apps_scroll_geometry_and_state_are_exact_and_bounded() {
        assert_eq!(
            APP_BACK_TARGET.y + APP_BACK_TARGET.height,
            PAGE_SCROLL_VIEWPORT_TOP_PX
        );
        assert_eq!(PAGE_SCROLL_VIEWPORT_BOTTOM_PX, SYSTEM_NAV_TOP_PX);
        assert_eq!(SETTINGS_SCROLL_MAX_PX % PAGE_SCROLL_QUANTUM_PX, 0);
        assert_eq!(APPS_SCROLL_MAX_PX % PAGE_SCROLL_QUANTUM_PX, 0);
        // `card` adds a four-design-pixel shadow. At maximum scroll each
        // below-fold extent therefore ends exactly at the viewport boundary.
        assert_eq!(
            (786 + 4 + 100) * SCALE - i32::from(SETTINGS_SCROLL_MAX_PX),
            i32::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX)
        );
        #[cfg(not(feature = "androidbox-runtime-uninstall1"))]
        let apps_last_card_y = 774;
        #[cfg(feature = "androidbox-runtime-uninstall1")]
        let apps_last_card_y = 818;
        assert_eq!(
            (apps_last_card_y + 4 + 92) * SCALE - i32::from(APPS_SCROLL_MAX_PX),
            i32::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX)
        );

        let mut model = MobileModel::for_page(MobilePage::Settings);
        assert_eq!(model.effective_page_scroll_offset_px(), 0);
        assert!(model.apply(MobileAction::SetPageScroll {
            page: MobilePage::Settings,
            offset_px: 31,
        }));
        assert_eq!(model.settings_scroll_offset_px, 24);
        assert_eq!(model.effective_page_scroll_offset_px(), 24);
        assert!(!model.apply(MobileAction::SetPageScroll {
            page: MobilePage::Apps,
            offset_px: APPS_SCROLL_MAX_PX,
        }));
        assert_eq!(model.apps_scroll_offset_px, 0);
        assert!(model.apply(MobileAction::SetPageScroll {
            page: MobilePage::Settings,
            offset_px: u16::MAX,
        }));
        assert_eq!(model.settings_scroll_offset_px, SETTINGS_SCROLL_MAX_PX);
        assert!(!model.apply(MobileAction::SetPageScroll {
            page: MobilePage::Settings,
            offset_px: SETTINGS_SCROLL_MAX_PX + PAGE_SCROLL_QUANTUM_PX,
        }));

        assert!(model.apply(MobileAction::Open(MobilePage::Apps)));
        assert_eq!(model.effective_page_scroll_offset_px(), 0);
        assert!(model.apply(MobileAction::SetPageScroll {
            page: MobilePage::Apps,
            offset_px: u16::MAX,
        }));
        assert_eq!(model.apps_scroll_offset_px, APPS_SCROLL_MAX_PX);
        assert_eq!(model.effective_page_scroll_offset_px(), APPS_SCROLL_MAX_PX);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Settings);
        assert_eq!(
            model.effective_page_scroll_offset_px(),
            SETTINGS_SCROLL_MAX_PX
        );
        assert_eq!(model.apps_scroll_offset_px, APPS_SCROLL_MAX_PX);

        assert_eq!(
            page_scroll_offset_for_drag(0, 1_000, 1_032, SETTINGS_SCROLL_MAX_PX),
            0
        );
        assert_eq!(
            page_scroll_offset_for_drag(SETTINGS_SCROLL_MAX_PX, 1_000, 968, SETTINGS_SCROLL_MAX_PX,),
            SETTINGS_SCROLL_MAX_PX
        );
    }

    #[test]
    fn settings_and_apps_scroll_follows_the_finger_without_committing_a_tap() {
        let mut blank_model = MobileModel::for_page(MobilePage::Settings);
        let mut blank_drag = TouchController::new();
        assert_eq!(blank_drag.observe(blank_model, 360, 1_000, true), None);
        assert_eq!(blank_drag.observe(blank_model, 360, 985, true), None);
        let scroll = blank_drag.observe(blank_model, 360, 968, true).unwrap();
        assert_eq!(
            scroll,
            MobileAction::SetPageScroll {
                page: MobilePage::Settings,
                offset_px: 32,
            }
        );
        assert!(blank_model.apply(scroll));
        assert_eq!(blank_model.settings_scroll_offset_px, 32);
        assert_eq!(blank_drag.observe(blank_model, 360, 968, false), None);

        let apps_x = SETTINGS_APPS_TARGET.x + SETTINGS_APPS_TARGET.width / 2;
        let apps_y = SETTINGS_APPS_TARGET.y + SETTINGS_APPS_TARGET.height / 2;
        let mut tap_model = MobileModel::for_page(MobilePage::Settings);
        let mut tap = TouchController::new();
        let down = tap.observe(tap_model, apps_x, apps_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::Apps))
        );
        assert!(tap_model.apply(down));
        assert_eq!(
            tap.observe(tap_model, apps_x, apps_y, false),
            Some(MobileAction::Open(MobilePage::Apps))
        );

        let mut drag_model = MobileModel::for_page(MobilePage::Settings);
        let mut row_drag = TouchController::new();
        let down = row_drag.observe(drag_model, apps_x, apps_y, true).unwrap();
        assert!(drag_model.apply(down));
        let scroll = row_drag
            .observe(drag_model, apps_x, apps_y - 32, true)
            .unwrap();
        assert_eq!(
            scroll,
            MobileAction::SetPageScroll {
                page: MobilePage::Settings,
                offset_px: 32,
            }
        );
        assert!(drag_model.apply(scroll));
        assert_eq!(drag_model.pressed_target, None);
        assert_eq!(
            row_drag.observe(drag_model, apps_x, apps_y - 32, false),
            None
        );
        assert_eq!(drag_model.page, MobilePage::Settings);

        let mut scrolled_model = MobileModel {
            settings_scroll_offset_px: 128,
            ..MobileModel::for_page(MobilePage::Settings)
        };
        let translated_y = apps_y - scrolled_model.effective_page_scroll_offset_px();
        assert_eq!(
            hit_test(scrolled_model, apps_x, translated_y),
            Some(MobilePressedTarget::Apps)
        );
        let mut translated_tap = TouchController::new();
        let down = translated_tap
            .observe(scrolled_model, apps_x, translated_y, true)
            .unwrap();
        assert!(scrolled_model.apply(down));
        assert_eq!(
            translated_tap.observe(scrolled_model, apps_x, translated_y, false),
            Some(MobileAction::Open(MobilePage::Apps))
        );
    }

    #[test]
    fn page_scroll_never_steals_back_shade_dimming_or_system_home() {
        let settings = MobileModel::for_page(MobilePage::Settings);

        let mut edge = TouchController::new();
        assert_eq!(edge.observe(settings, 32, 800, true), None);
        assert!(matches!(
            edge.observe(settings, 176, 800, true),
            Some(MobileAction::SetBackReveal { .. })
        ));

        let mut shade = TouchController::new();
        assert_eq!(shade.observe(settings, 360, 80, true), None);
        assert_eq!(
            shade.observe(settings, 360, 320, true),
            Some(MobileAction::SetShadeReveal(240))
        );

        let home_x = SYSTEM_HOME_TARGET.x + SYSTEM_HOME_TARGET.width / 2;
        let home_y = SYSTEM_HOME_TARGET.y + SYSTEM_HOME_TARGET.height / 2;
        let mut home_model = settings;
        let mut home = TouchController::new();
        let down = home.observe(home_model, home_x, home_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::SystemHome))
        );
        assert!(home_model.apply(down));
        assert_eq!(
            home.observe(home_model, home_x, home_y, false),
            Some(MobileAction::Home)
        );

        let mut dimming_model = MobileModel::for_page(MobilePage::Display);
        let slider_y = DISPLAY_DIMMING_TARGET.y + DISPLAY_DIMMING_TARGET.height / 2;
        let mut dimming = TouchController::new();
        let down = dimming.observe(dimming_model, 502, slider_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::SoftwareDimming))
        );
        assert!(dimming_model.apply(down));
        assert_eq!(
            dimming.observe(dimming_model, 575, slider_y, true),
            Some(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Light))
        );
        assert_eq!(dimming_model.page, MobilePage::Display);
    }

    #[test]
    fn scrolled_pages_clip_to_the_fixed_header_and_navigation_boundaries() {
        for (stable_model, scrolled_model) in [
            (
                MobileModel::for_page(MobilePage::Settings),
                MobileModel {
                    settings_scroll_offset_px: SETTINGS_SCROLL_MAX_PX,
                    ..MobileModel::for_page(MobilePage::Settings)
                },
            ),
            (
                MobileModel::for_page(MobilePage::Apps),
                MobileModel {
                    apps_scroll_offset_px: APPS_SCROLL_MAX_PX,
                    ..MobileModel::for_page(MobilePage::Apps)
                },
            ),
        ] {
            let stable = render_model(stable_model);
            let scrolled = render_model(scrolled_model);
            let viewport_top = usize::from(PAGE_SCROLL_VIEWPORT_TOP_PX) * WIDTH;
            let viewport_bottom = usize::from(PAGE_SCROLL_VIEWPORT_BOTTOM_PX) * WIDTH;
            assert_eq!(&stable[..viewport_top], &scrolled[..viewport_top]);
            assert_eq!(&stable[viewport_bottom..], &scrolled[viewport_bottom..]);
            let changed_pixels = stable[viewport_top..viewport_bottom]
                .iter()
                .zip(scrolled[viewport_top..viewport_bottom].iter())
                .filter(|(left, right)| left != right)
                .count();
            assert!(changed_pixels > 10_000, "{changed_pixels}");

            let mut partitioned = vec![0_u32; PIXEL_COUNT];
            let mut first_row = 0;
            while first_row < HEIGHT {
                let row_count = (HEIGHT - first_row).min(19);
                let start = first_row * WIDTH;
                let end = start + row_count * WIDTH;
                render_rows(&mut partitioned[start..end], first_row, scrolled_model).unwrap();
                first_row += row_count;
            }
            assert_eq!(partitioned, scrolled);
        }

        // Sub-quantum persisted values are canonical zero and retain the
        // original stable first frame exactly.
        assert_eq!(
            render_model(MobileModel {
                settings_scroll_offset_px: PAGE_SCROLL_QUANTUM_PX - 1,
                ..MobileModel::for_page(MobilePage::Settings)
            }),
            render_model(MobileModel::for_page(MobilePage::Settings))
        );
        assert_eq!(
            render_model(MobileModel {
                apps_scroll_offset_px: PAGE_SCROLL_QUANTUM_PX - 1,
                ..MobileModel::for_page(MobilePage::Apps)
            }),
            render_model(MobileModel::for_page(MobilePage::Apps))
        );
    }

    #[test]
    fn apps_page_and_drawer_never_infer_installation_from_demo_results() {
        let empty = MobileModel::for_page(MobilePage::Apps);
        let empty_pixels = render_model(empty);
        let installed = MobileModel {
            android_installed_app: installed_android_app(7),
            ..empty
        };
        let installed_pixels = render_model(installed);
        assert_ne!(empty_pixels, installed_pixels);

        let demo_only = MobileModel {
            androidbox_verified: true,
            androidbox_manifest_verified: true,
            androidbox_on_create_completed: true,
            androidbox_resources: verified_androidbox_resources(),
            ..empty
        };
        assert_eq!(
            demo_only.android_installed_app,
            AndroidInstalledAppStatus::empty()
        );
        assert_eq!(
            hit_test(
                MobileModel {
                    drawer_open: true,
                    ..demo_only
                },
                DRAWER_ANDROIDBOX_TARGET.x + DRAWER_ANDROIDBOX_TARGET.width / 2,
                DRAWER_ANDROIDBOX_TARGET.y + DRAWER_ANDROIDBOX_TARGET.height / 2,
            ),
            Some(MobilePressedTarget::DrawerAndroidBoxDemo)
        );
        assert_eq!(
            hit_test(
                MobileModel {
                    drawer_open: true,
                    ..installed
                },
                DRAWER_ANDROIDBOX_TARGET.x + DRAWER_ANDROIDBOX_TARGET.width / 2,
                DRAWER_ANDROIDBOX_TARGET.y + DRAWER_ANDROIDBOX_TARGET.height / 2,
            ),
            Some(MobilePressedTarget::DrawerInstalledAndroid(7))
        );
        assert!(
            measure_mobile_text_px(
                MobileTextRole::Label,
                fitted_mobile_text(
                    installed.android_installed_app.title.as_str(),
                    MobileTextRole::Label,
                    136,
                ),
            ) <= 136
        );
        let long_title = AndroidInstalledTitle::from_ascii("WWWWWWWWWWWWWWWWWWWWWWWW").unwrap();
        let fitted = fitted_mobile_text(long_title.as_str(), MobileTextRole::Label, 136);
        assert!(fitted.len() < long_title.as_str().len());
        assert!(measure_mobile_text_px(MobileTextRole::Label, fitted) <= 136);
        assert_eq!(
            split_installed_app_label("Mac-built Android app", false),
            ("Mac-built", "Android app")
        );
        assert_eq!(split_installed_app_label("OneWord", false), ("OneWord", ""));
    }

    #[test]
    fn installed_drawer_press_is_generation_bound_and_stale_release_fails_closed() {
        let target = DRAWER_ANDROIDBOX_TARGET;
        let x = target.x + target.width / 2;
        let y = target.y + target.height / 2;
        let mut model = MobileModel {
            drawer_open: true,
            android_installed_app: installed_android_app(7),
            ..MobileModel::default()
        };
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::DrawerInstalledAndroid(7)))
        );
        assert!(model.apply(down));
        assert!(model.apply_android_installed_app_status(installed_android_app(8)));
        assert_eq!(
            touch.observe(model, x, y, false),
            Some(MobileAction::SetPressed(None))
        );
        assert!(model.apply(MobileAction::SetPressed(None)));
        assert_eq!(model.page, MobilePage::Home);
        assert!(model.drawer_open);

        let mut fresh = TouchController::new();
        let down = fresh.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::DrawerInstalledAndroid(8)))
        );
        assert!(model.apply(down));
        let launch = fresh.observe(model, x, y, false).unwrap();
        assert_eq!(launch, MobileAction::LaunchInstalledAndroid(8));
        assert!(model.apply(launch));
        assert_eq!(model.page, MobilePage::Home);
        assert!(model.drawer_open);
        assert_eq!(
            model.android_installed_launch,
            AndroidInstalledLaunchStatus::Idle
        );

        let mut uninstall = TouchController::new();
        model.pressed_target = None;
        let down = uninstall.observe(model, x, y, true).unwrap();
        assert!(model.apply(down));
        assert!(model.apply_android_installed_app_status(AndroidInstalledAppStatus::empty()));
        assert_eq!(
            uninstall.observe(model, x, y, false),
            Some(MobileAction::SetPressed(None))
        );
    }

    #[test]
    fn apps_info_card_is_inert_and_preserves_the_complete_edge_back_strip() {
        let model = MobileModel {
            page: MobilePage::Apps,
            android_installed_app: installed_android_app(7),
            ..MobileModel::default()
        };
        for x in [35, 36, 47, 48, 683, 684] {
            assert_eq!(hit_test(model, x, 1_200), None, "x={x}");
        }

        let mut edge_model = model;
        let mut edge = TouchController::new();
        assert_eq!(edge.observe(edge_model, 47, 1_200, true), None);
        let reveal = edge.observe(edge_model, 191, 1_200, true).unwrap();
        assert!(edge_model.apply(reveal));
        assert_eq!(
            edge.observe(edge_model, 191, 1_200, false),
            Some(MobileAction::Back)
        );

        let mut outside = TouchController::new();
        assert_eq!(outside.observe(model, 48, 1_200, true), None);
        assert_eq!(outside.observe(model, 176, 1_200, true), None);
        assert_eq!(outside.observe(model, 176, 1_200, false), None);
    }

    #[test]
    fn software_dimming_is_an_exact_final_surface_transform() {
        let full = render_model(MobileModel::default());
        assert_eq!(
            render_model(MobileModel {
                software_dimming: UiSoftwareDimming::Off,
                ..MobileModel::default()
            }),
            full
        );

        for level in [
            UiSoftwareDimming::Light,
            UiSoftwareDimming::Medium,
            UiSoftwareDimming::Strong,
            UiSoftwareDimming::Maximum,
        ] {
            let alpha = software_dimming_alpha(level);
            let dimmed = render_model(MobileModel {
                software_dimming: level,
                ..MobileModel::default()
            });
            let mut changed = 0;
            for (before, after) in full.iter().zip(dimmed.iter()) {
                let expected = blend(*before, COLOR_BLACK, alpha);
                assert_eq!(*after, expected, "{level:?}");
                assert_eq!(*after >> 24, 0);
                changed += usize::from(before != after);
            }
            assert!(changed > 900_000, "{level:?} changed only {changed} pixels");
        }
    }

    #[test]
    fn software_dimming_slider_is_five_stop_drag_and_tap_input() {
        assert_eq!(
            software_dimming_for_x(DIMMING_TRACK_START_X_PX),
            UiSoftwareDimming::Maximum
        );
        assert_eq!(software_dimming_for_x(429), UiSoftwareDimming::Strong);
        assert_eq!(software_dimming_for_x(502), UiSoftwareDimming::Medium);
        assert_eq!(software_dimming_for_x(575), UiSoftwareDimming::Light);
        assert_eq!(
            software_dimming_for_x(DIMMING_TRACK_END_X_PX),
            UiSoftwareDimming::Off
        );
        assert_eq!(software_dimming_for_x(0), UiSoftwareDimming::Maximum);
        assert_eq!(software_dimming_for_x(u16::MAX), UiSoftwareDimming::Off);

        let mut settings = MobileModel::for_page(MobilePage::Display);
        let mut tap = TouchController::new();
        assert_eq!(
            tap.observe(settings, DIMMING_TRACK_START_X_PX, 800, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::SoftwareDimming
            )))
        );
        assert!(settings.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::SoftwareDimming
        ))));
        assert_eq!(
            tap.observe(settings, DIMMING_TRACK_START_X_PX, 800, false),
            Some(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Maximum))
        );
        assert!(settings.apply(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Maximum)));

        let mut drag = TouchController::new();
        assert_eq!(
            drag.observe(settings, DIMMING_TRACK_START_X_PX, 800, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::SoftwareDimming
            )))
        );
        assert!(settings.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::SoftwareDimming
        ))));
        assert_eq!(
            drag.observe(settings, 502, 800, true),
            Some(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Medium))
        );
        assert!(settings.apply(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Medium)));
        assert_eq!(drag.observe(settings, 503, 800, true), None);
        assert_eq!(
            drag.observe(settings, 575, 800, true),
            Some(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Light))
        );
        assert!(settings.apply(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Light)));
        assert_eq!(drag.observe(settings, 575, 800, false), None);

        let mut shade = MobileModel {
            shade_open: true,
            ..MobileModel::default()
        };
        let mut shade_touch = TouchController::new();
        assert_eq!(
            shade_touch.observe(shade, 502, 748, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::SoftwareDimming
            )))
        );
        assert!(shade.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::SoftwareDimming
        ))));
        assert_eq!(
            shade_touch.observe(shade, 502, 748, false),
            Some(MobileAction::SetSoftwareDimming(UiSoftwareDimming::Medium))
        );
    }

    #[test]
    fn touch_requires_matching_press_and_release_target() {
        let mut touch = TouchController::new();
        let mut home = MobileModel::for_page(MobilePage::Home);
        let mut settings = MobileModel::for_page(MobilePage::Settings);
        let stable_home = render_model(home);
        let pressed = touch.observe(home, 560, 1_412, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::Settings))
        );
        assert!(home.apply(pressed));
        assert_ne!(render_model(home), stable_home);
        assert_eq!(
            touch.observe(home, 560, 1_412, false),
            Some(MobileAction::Open(MobilePage::Settings))
        );
        assert!(home.apply(MobileAction::Open(MobilePage::Settings)));
        assert_eq!(home.pressed_target, None);

        let pressed = touch.observe(settings, 600, 474, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::Display))
        );
        assert!(settings.apply(pressed));
        assert_eq!(
            touch.observe(settings, 40, 800, false),
            Some(MobileAction::SetPressed(None))
        );
        assert!(settings.apply(MobileAction::SetPressed(None)));
        let pressed = touch.observe(settings, 600, 474, true).unwrap();
        assert!(settings.apply(pressed));
        assert_eq!(
            touch.observe(settings, 600, 474, false),
            Some(MobileAction::Open(MobilePage::Display))
        );
        assert!(settings.apply(MobileAction::Open(MobilePage::Display)));
        assert_eq!(settings.pressed_target, None);

        let mut display_touch = TouchController::new();
        let pressed = display_touch.observe(settings, 600, 474, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::Theme))
        );
        assert!(settings.apply(pressed));
        assert_eq!(
            display_touch.observe(settings, 600, 474, false),
            Some(MobileAction::ToggleTheme)
        );
        assert!(settings.apply(MobileAction::ToggleTheme));
        assert_eq!(settings.pressed_target, None);
    }

    #[test]
    fn dragging_out_cancels_press_feedback_until_release() {
        let mut model = MobileModel::for_page(MobilePage::Settings);
        let stable = render_model(model);
        let mut touch = TouchController::new();
        let down = touch.observe(model, 600, 474, true).unwrap();
        assert!(model.apply(down));
        assert_ne!(render_model(model), stable);
        assert_eq!(
            touch.observe(model, 40, 800, true),
            Some(MobileAction::SetPressed(None))
        );
        assert!(model.apply(MobileAction::SetPressed(None)));
        assert_eq!(render_model(model), stable);
        assert_eq!(touch.observe(model, 600, 474, true), None);
        assert_eq!(touch.observe(model, 600, 474, false), None);
    }

    #[test]
    fn phone_keypad_is_bounded_session_local_and_has_no_fake_call_target() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        let stable = render_model(model);
        let target = PHONE_KEY_TARGETS[0];
        let x = target.x + target.width / 2;
        let y = target.y + target.height / 2;
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::PhoneKey(0)))
        );
        assert!(model.apply(down));
        assert_ne!(render_model(model), stable);
        assert_eq!(
            touch.observe(model, x, y, false),
            Some(MobileAction::PhoneKey(0))
        );
        assert!(model.apply(MobileAction::PhoneKey(0)));
        assert_eq!(model.phone_number(), "1");
        assert_eq!(model.pressed_target, None);

        for index in 0..(PHONE_DIGIT_CAPACITY - 1) {
            assert!(model.apply(MobileAction::PhoneKey(
                u8::try_from(index % PHONE_KEY_TARGETS.len()).unwrap()
            )));
        }
        assert_eq!(usize::from(model.phone_digit_count), PHONE_DIGIT_CAPACITY);
        let full = model.phone_digits;
        assert!(!model.apply(MobileAction::PhoneKey(1)));
        assert_eq!(model.phone_digits, full);
        assert!(model.apply(MobileAction::PhoneBackspace));
        assert_eq!(
            usize::from(model.phone_digit_count),
            PHONE_DIGIT_CAPACITY - 1
        );
        assert!(model.apply(MobileAction::Open(MobilePage::Messages)));
        assert!(model.apply(MobileAction::Open(MobilePage::Phone)));
        assert_eq!(
            usize::from(model.phone_digit_count),
            PHONE_DIGIT_CAPACITY - 1
        );

        let mut disabled_call = TouchController::new();
        assert_eq!(disabled_call.observe(model, 360, 1_376, true), None);
        assert_eq!(disabled_call.observe(model, 360, 1_376, false), None);
    }

    #[test]
    fn phone_punctuation_keys_are_drawn_without_headline_font_fallback() {
        assert!(!mobile_text_has_glyph(MobileTextRole::Headline, '*'));
        assert!(!mobile_text_has_glyph(MobileTextRole::Headline, '#'));
        let pixels = render_model(MobileModel::for_page(MobilePage::Phone));
        let text = COLOR_TEXT_DARK;
        let pixel = |x: usize, y: usize| pixels[y * WIDTH + x];
        // Exact centers of the custom vertical asterisk stroke and the two
        // horizontal hash strokes, expressed in output pixels.
        assert_eq!(pixel(164, 1_132), text);
        assert_eq!(pixel(556, 1_120), text);
        assert_eq!(pixel(556, 1_144), text);
    }

    #[test]
    fn calculator_is_bounded_functional_and_session_local() {
        let mut model = MobileModel::for_page(MobilePage::Calculator);
        let initial = render_model(model);

        // 12 + 7 = 19.
        for key in [12, 13, 15, 4, 18] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 19 * CALCULATOR_SCALE);
        assert_eq!(model.calculator_pending, CalculatorOperation::None);
        assert!(!model.calculator_entering);
        let nineteen = render_model(model);
        assert_ne!(nineteen, initial);

        // A local Home/return round-trip preserves the bounded session state.
        assert!(model.apply(MobileAction::Home));
        assert_eq!(model.page, MobilePage::Home);
        assert!(model.apply(MobileAction::Open(MobilePage::Calculator)));
        assert_eq!(model.calculator_value, 19 * CALCULATOR_SCALE);
        assert_eq!(render_model(model), nineteen);

        // 9 x 8 = 72.
        for key in [0, 6, 7, 5, 18] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 72 * CALCULATOR_SCALE);

        // Immediate-operation chaining: 1 + 2 + 3 = 6.
        for key in [0, 12, 15, 13, 15, 14, 18] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 6 * CALCULATOR_SCALE);

        // Fixed-point decimal entry and addition: 1.25 + 0.75 = 2.
        for key in [0, 12, 17, 13, 9, 15, 16, 17, 4, 9, 18] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 2 * CALCULATOR_SCALE);

        // Sign, percent, and decimal-aware backspace remain exact.
        for key in [0, 9, 1, 2] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, -50_000);
        for key in [0, 12, 13, 14, 19] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 12 * CALCULATOR_SCALE);
        for key in [0, 12, 17, 13, 9, 19] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert_eq!(model.calculator_value, 1_200_000);
        assert_eq!(model.calculator_fraction_digits, 1);

        // Divide by zero and numeric overflow fail closed until C.
        for key in [0, 6, 3, 16, 18] {
            assert!(model.apply(MobileAction::CalculatorKey(key)));
        }
        assert!(model.calculator_error);
        let error = render_model(model);
        assert_ne!(error, initial);
        assert!(!model.apply(MobileAction::CalculatorKey(12)));
        assert_eq!(render_model(model), error);
        assert!(model.apply(MobileAction::CalculatorKey(0)));
        assert!(!model.calculator_error);
        assert_eq!(model.calculator_value, 0);

        model.calculator_value = CALCULATOR_VALUE_LIMIT;
        model.calculator_entering = true;
        assert!(model.apply(MobileAction::CalculatorKey(6)));
        assert!(model.calculator_error);
        assert!(model.apply(MobileAction::CalculatorKey(0)));
        assert_eq!(render_model(model), initial);
    }

    #[test]
    fn calculator_touch_feedback_cancels_and_routes_without_app_focus() {
        let mut home = MobileModel::default();
        let mut home_touch = TouchController::new();
        let x = HOME_CALCULATOR_TARGET.x + HOME_CALCULATOR_TARGET.width / 2;
        let y = HOME_CALCULATOR_TARGET.y + HOME_CALCULATOR_TARGET.height / 2;
        assert_eq!(
            home_touch.observe(home, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::Calculator
            )))
        );
        assert!(home.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::Calculator
        ))));
        assert_eq!(
            home_touch.observe(home, x, y, false),
            Some(MobileAction::Open(MobilePage::Calculator))
        );
        assert!(home.apply(MobileAction::Open(MobilePage::Calculator)));
        assert_eq!(home.page, MobilePage::Calculator);

        let target = CALCULATOR_KEY_TARGETS[4];
        let key_x = target.x + target.width / 2;
        let key_y = target.y + target.height / 2;
        let stable = render_model(home);
        let mut cancelled = TouchController::new();
        let down = cancelled.observe(home, key_x, key_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::CalculatorKey(4)))
        );
        assert!(home.apply(down));
        assert_ne!(render_model(home), stable);
        assert_eq!(
            cancelled.observe(home, 360, 1_520, true),
            Some(MobileAction::SetPressed(None))
        );
        assert!(home.apply(MobileAction::SetPressed(None)));
        assert_eq!(cancelled.observe(home, key_x, key_y, true), None);
        assert_eq!(cancelled.observe(home, key_x, key_y, false), None);
        assert_eq!(home.calculator_value, 0);
        assert_eq!(render_model(home), stable);

        let mut committed = TouchController::new();
        let down = committed.observe(home, key_x, key_y, true).unwrap();
        assert!(home.apply(down));
        assert_eq!(
            committed.observe(home, key_x, key_y, false),
            Some(MobileAction::CalculatorKey(4))
        );
        assert!(home.apply(MobileAction::CalculatorKey(4)));
        assert_eq!(home.calculator_value, 7 * CALCULATOR_SCALE);
        assert!(home.apply(MobileAction::Back));
        assert_eq!(home.page, MobilePage::Home);
    }

    #[test]
    fn messages_local_information_navigates_before_leaving_the_app() {
        let mut model = MobileModel::for_page(MobilePage::Messages);
        let inbox = render_model(model);
        let mut touch = TouchController::new();
        let x = MESSAGE_GUIDE_TARGET.x + MESSAGE_GUIDE_TARGET.width / 2;
        let y = MESSAGE_GUIDE_TARGET.y + MESSAGE_GUIDE_TARGET.height / 2;
        assert_eq!(
            touch.observe(model, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::MessageGuide
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::MessageGuide
        ))));
        assert_eq!(
            touch.observe(model, x, y, false),
            Some(MobileAction::OpenMessage(MessageView::PreviewGuide))
        );
        assert!(model.apply(MobileAction::OpenMessage(MessageView::PreviewGuide)));
        assert!(model.has_in_app_back());
        assert_ne!(render_model(model), inbox);
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Messages);
        assert_eq!(model.message_view, MessageView::Inbox);
        assert_eq!(render_model(model), inbox);
        assert!(!model.has_in_app_back());
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Home);
    }

    #[test]
    fn edge_back_is_quantized_thresholded_and_cancel_restores_exactly() {
        let mut model = MobileModel::for_page(MobilePage::Settings);
        let stable = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 32, 800, true), None);
        let reveal = touch.observe(model, 56, 800, true).unwrap();
        assert_eq!(
            reveal,
            MobileAction::SetBackReveal {
                reveal_px: 24,
                origin_y: 800
            }
        );
        assert!(model.apply(reveal));
        let transient = render_model(model);
        assert_ne!(transient, stable);
        for (index, (before, after)) in stable.iter().zip(transient.iter()).enumerate() {
            if before == after {
                continue;
            }
            assert!(index % WIDTH < 104);
            assert!(index / WIDTH < usize::from(SYSTEM_NAV_TOP_PX));
        }
        assert_eq!(
            touch.observe(model, 175, 800, false),
            Some(MobileAction::SetBackReveal {
                reveal_px: 0,
                origin_y: 0
            })
        );
        assert!(model.apply(MobileAction::SetBackReveal {
            reveal_px: 0,
            origin_y: 0
        }));
        assert_eq!(render_model(model), stable);

        let mut committed = TouchController::new();
        assert_eq!(committed.observe(model, 32, 800, true), None);
        let reveal = committed.observe(model, 56, 800, true).unwrap();
        assert!(model.apply(reveal));
        assert_eq!(
            committed.observe(model, 176, 800, false),
            Some(MobileAction::Back)
        );
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Home);
    }

    #[test]
    fn edge_back_boundaries_axis_lock_and_explicit_cancel_are_fail_closed() {
        let settings = MobileModel::for_page(MobilePage::Settings);
        for y in [176, 1_547] {
            let mut inside = TouchController::new();
            assert_eq!(inside.observe(settings, 47, y, true), None);
            assert!(matches!(
                inside.observe(settings, 71, y, true),
                Some(MobileAction::SetBackReveal {
                    reveal_px: 24,
                    origin_y
                }) if origin_y == y
            ));
        }
        let mut outside_x = TouchController::new();
        assert_eq!(outside_x.observe(settings, 48, 800, true), None);
        assert_eq!(outside_x.observe(settings, 192, 800, true), None);
        assert_eq!(outside_x.observe(settings, 192, 800, false), None);

        let mut header_button = TouchController::new();
        assert_eq!(
            header_button.observe(settings, 32, 128, true),
            Some(MobileAction::SetPressed(Some(MobilePressedTarget::Back)))
        );
        assert_eq!(
            header_button.observe(settings, 32, 128, false),
            Some(MobileAction::Back)
        );
        let mut settings_row = TouchController::new();
        assert_eq!(
            settings_row.observe(settings, 32, 474, true),
            Some(MobileAction::SetPressed(Some(MobilePressedTarget::Display)))
        );
        let mut bottom_nav = TouchController::new();
        assert_eq!(bottom_nav.observe(settings, 32, 1_548, true), None);
        assert_eq!(bottom_nav.observe(settings, 200, 1_548, false), None);

        let mut rejected = TouchController::new();
        assert_eq!(rejected.observe(settings, 32, 800, true), None);
        assert_eq!(rejected.observe(settings, 56, 824, true), None);
        assert_eq!(rejected.observe(settings, 200, 800, true), None);
        assert_eq!(rejected.observe(settings, 200, 800, false), None);

        let mut model = settings;
        let stable = render_model(model);
        let mut cancelled = TouchController::new();
        assert_eq!(cancelled.observe(model, 32, 800, true), None);
        let reveal = cancelled.observe(model, 64, 810, true).unwrap();
        assert!(model.apply(reveal));
        assert_ne!(render_model(model), stable);
        let later = cancelled.observe(model, 80, 920, true).unwrap();
        assert!(matches!(later, MobileAction::SetBackReveal { .. }));
        assert!(model.apply(later));
        let clear = cancelled.cancel_contact(model).unwrap();
        assert_eq!(
            clear,
            MobileAction::SetBackReveal {
                reveal_px: 0,
                origin_y: 0
            }
        );
        assert!(model.apply(clear));
        assert_eq!(render_model(model), stable);
        assert_eq!(cancelled.observe(model, 600, 474, true), None);
        assert_eq!(cancelled.observe(model, 600, 474, false), None);
        assert_eq!(
            cancelled.observe(model, 600, 474, true),
            Some(MobileAction::SetPressed(Some(MobilePressedTarget::Display)))
        );

        let mut invalid_release_model = settings;
        let mut invalid_release = TouchController::new();
        assert_eq!(
            invalid_release.observe(invalid_release_model, 32, 800, true),
            None
        );
        let reveal = invalid_release
            .observe(invalid_release_model, 56, 800, true)
            .unwrap();
        assert!(invalid_release_model.apply(reveal));
        assert_eq!(
            invalid_release.observe(invalid_release_model, 176, 1_599, false),
            Some(MobileAction::SetBackReveal {
                reveal_px: 0,
                origin_y: 0
            })
        );
        assert!(invalid_release_model.apply(MobileAction::SetBackReveal {
            reveal_px: 0,
            origin_y: 0
        }));
        assert_eq!(invalid_release_model.page, MobilePage::Settings);

        let mut invalid_move_model = settings;
        let mut invalid_move = TouchController::new();
        assert_eq!(
            invalid_move.observe(invalid_move_model, 32, 800, true),
            None
        );
        let reveal = invalid_move
            .observe(invalid_move_model, 56, 800, true)
            .unwrap();
        assert!(invalid_move_model.apply(reveal));
        assert_eq!(
            invalid_move.observe(invalid_move_model, 100, 993, true),
            Some(MobileAction::SetBackReveal {
                reveal_px: 0,
                origin_y: 0
            })
        );
        assert!(invalid_move_model.apply(MobileAction::SetBackReveal {
            reveal_px: 0,
            origin_y: 0
        }));
        assert_eq!(
            invalid_move.observe(invalid_move_model, 176, 800, false),
            None
        );
        assert_eq!(invalid_move_model.page, MobilePage::Settings);

        let home = MobileModel::default();
        let mut stable_home_edge = TouchController::new();
        assert_eq!(stable_home_edge.observe(home, 32, 800, true), None);
        assert_eq!(stable_home_edge.observe(home, 200, 800, false), None);

        let mut shade_model = MobileModel::default();
        let shade_closed = render_model(shade_model);
        let mut shade = TouchController::new();
        assert_eq!(shade.observe(shade_model, 360, 0, true), None);
        let reveal = shade.observe(shade_model, 360, 100, true).unwrap();
        assert!(shade_model.apply(reveal));
        let clear = shade.cancel_contact(shade_model).unwrap();
        assert_eq!(clear, MobileAction::SetShadeReveal(0));
        assert!(shade_model.apply(clear));
        assert_eq!(render_model(shade_model), shade_closed);
    }

    #[test]
    fn system_home_has_feedback_drag_out_cancel_and_closes_settled_shade() {
        let x = SYSTEM_HOME_TARGET.x + SYSTEM_HOME_TARGET.width / 2;
        let y = SYSTEM_HOME_TARGET.y + SYSTEM_HOME_TARGET.height / 2;
        let mut model = MobileModel::for_page(MobilePage::Phone);
        let stable = render_model(model);
        let mut touch = TouchController::new();
        let down = touch.observe(model, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::SystemHome))
        );
        assert!(model.apply(down));
        assert_ne!(render_model(model), stable);
        assert_eq!(
            touch.observe(model, 120, y, true),
            Some(MobileAction::SetPressed(None))
        );
        assert!(model.apply(MobileAction::SetPressed(None)));
        assert_eq!(render_model(model), stable);
        assert_eq!(touch.observe(model, x, y, true), None);
        assert_eq!(touch.observe(model, x, y, false), None);
        assert_eq!(model.page, MobilePage::Phone);

        let mut committed = TouchController::new();
        let down = committed.observe(model, x, y, true).unwrap();
        assert!(model.apply(down));
        assert_eq!(
            committed.observe(model, x, y, false),
            Some(MobileAction::Home)
        );
        assert!(model.apply(MobileAction::Home));
        assert_eq!(model.page, MobilePage::Home);
        assert_eq!(model.pressed_target, None);

        let mut shade = MobileModel::default();
        assert!(shade.apply(MobileAction::OpenShade));
        let mut shade_touch = TouchController::new();
        let down = shade_touch.observe(shade, x, y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::SystemHome))
        );
        assert!(shade.apply(down));
        assert_eq!(
            shade_touch.observe(shade, x, y, false),
            Some(MobileAction::Home)
        );
        assert!(shade.apply(MobileAction::Home));
        assert!(!shade.shade_open);
        assert_eq!(shade.page, MobilePage::Home);
    }

    #[test]
    #[cfg(feature = "mobile-ui-runtime")]
    fn every_interactive_target_has_a_distinct_pressed_frame() {
        fn assert_pressed_frame(
            model: MobileModel,
            target: MobilePressedTarget,
            target_rect: ShellRect,
        ) {
            let stable = render_model(model);
            let mut pressed = model;
            assert!(pressed.apply(MobileAction::SetPressed(Some(target))));
            let pressed_pixels = render_model(pressed);
            let expected_damage = DamageRect {
                x: target_rect.x,
                y: target_rect.y,
                width: target_rect.width,
                height: target_rect.height,
            };
            assert_eq!(
                damage_plan(Some(model), pressed),
                MobileDamagePlan::Regions(DamageRegions::single(expected_damage).unwrap()),
                "press {target:?}"
            );
            let mut damaged = stable.clone();
            render_damage(&mut damaged, pressed, expected_damage).unwrap();
            assert_eq!(damaged, pressed_pixels, "damage press {target:?}");
            let mut changed = 0;
            for (index, (before, after)) in stable.iter().zip(pressed_pixels.iter()).enumerate() {
                if before == after {
                    continue;
                }
                changed += 1;
                let x = u16::try_from(index % WIDTH).unwrap();
                let y = u16::try_from(index / WIDTH).unwrap();
                assert!(
                    target_rect.contains(x, y),
                    "{target:?} changed a pixel outside {target_rect:?} at {x}/{y}"
                );
            }
            let minimum_changed =
                (usize::from(target_rect.width) * usize::from(target_rect.height) / 100).max(256);
            assert!(
                changed >= minimum_changed,
                "{target:?} changed only {changed} pixels"
            );
            assert!(pressed.apply(MobileAction::SetPressed(None)));
            assert_eq!(
                damage_plan(
                    Some(MobileModel {
                        pressed_target: Some(target),
                        ..model
                    }),
                    pressed
                ),
                MobileDamagePlan::Regions(DamageRegions::single(expected_damage).unwrap()),
                "release {target:?}"
            );
            assert_eq!(render_model(pressed), stable, "{target:?}");
        }

        for (model, target, target_rect) in [
            (
                MobileModel {
                    system_ui_mode: UiSystemUiMode::Overview,
                    system_ui_recent: Some(UiRecentIdentity::Shell(ShellAppId::Phone)),
                    ..MobileModel::for_page(MobilePage::Phone)
                },
                MobilePressedTarget::OverviewRecent,
                OVERVIEW_RECENT_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Phone),
                MobilePressedTarget::SystemHome,
                SYSTEM_HOME_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Home),
                MobilePressedTarget::Phone,
                HOME_PHONE_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Home),
                MobilePressedTarget::Messages,
                HOME_MESSAGES_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Home),
                MobilePressedTarget::Calculator,
                HOME_CALCULATOR_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Home),
                MobilePressedTarget::Settings,
                HOME_SETTINGS_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Settings),
                MobilePressedTarget::Back,
                APP_BACK_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Settings),
                MobilePressedTarget::Display,
                SETTINGS_DISPLAY_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Display),
                MobilePressedTarget::Theme,
                DISPLAY_THEME_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Display),
                MobilePressedTarget::Accent,
                DISPLAY_ACCENT_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Display),
                MobilePressedTarget::SoftwareDimming,
                DISPLAY_DIMMING_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Display),
                MobilePressedTarget::Accessibility,
                DISPLAY_ACCESSIBILITY_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Accessibility),
                MobilePressedTarget::LargeText,
                ACCESSIBILITY_LARGE_TEXT_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Accessibility),
                MobilePressedTarget::HighContrast,
                ACCESSIBILITY_HIGH_CONTRAST_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Settings),
                MobilePressedTarget::Apps,
                SETTINGS_APPS_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Settings),
                MobilePressedTarget::About,
                SETTINGS_ABOUT_TARGET,
            ),
            (
                MobileModel {
                    shade_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::Theme,
                QUICK_THEME_TARGET,
            ),
            (
                MobileModel {
                    shade_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::Accent,
                QUICK_ACCENT_TARGET,
            ),
            (
                MobileModel {
                    shade_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::SoftwareDimming,
                QUICK_DIMMING_TARGET,
            ),
            (
                MobileModel {
                    shade_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::BootNotification,
                QUICK_BOOT_NOTIFICATION_TARGET,
            ),
            (
                MobileModel::locked(),
                MobilePressedTarget::BootNotification,
                LOCK_BOOT_NOTIFICATION_TARGET,
            ),
            (
                MobileModel {
                    shade_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::ShadeClose,
                QUICK_CLOSE_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerPhone,
                DRAWER_PHONE_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerMessages,
                DRAWER_MESSAGES_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerCalculator,
                DRAWER_CALCULATOR_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerSettings,
                DRAWER_SETTINGS_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerAndroidBoxDemo,
                DRAWER_ANDROIDBOX_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    android_installed_app: installed_android_app(7),
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerInstalledAndroid(7),
                DRAWER_ANDROIDBOX_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerHandle,
                DRAWER_HANDLE_TARGET,
            ),
            (
                MobileModel {
                    drawer_open: true,
                    ..MobileModel::default()
                },
                MobilePressedTarget::DrawerHome,
                DRAWER_HOME_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::AndroidDemo),
                MobilePressedTarget::AndroidBoxExecute,
                ANDROIDBOX_EXECUTE_TARGET,
            ),
        ] {
            assert_pressed_frame(model, target, target_rect);
        }
        for (index, target_rect) in PHONE_KEY_TARGETS.into_iter().enumerate() {
            assert_pressed_frame(
                MobileModel::for_page(MobilePage::Phone),
                MobilePressedTarget::PhoneKey(u8::try_from(index).unwrap()),
                target_rect,
            );
        }
        for (index, target_rect) in CALCULATOR_KEY_TARGETS.into_iter().enumerate() {
            assert_pressed_frame(
                MobileModel::for_page(MobilePage::Calculator),
                MobilePressedTarget::CalculatorKey(u8::try_from(index).unwrap()),
                target_rect,
            );
        }
        for (model, target, target_rect) in [
            (
                MobileModel::for_page(MobilePage::Phone),
                MobilePressedTarget::PhoneBackspace,
                PHONE_BACKSPACE_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Messages),
                MobilePressedTarget::MessageGuide,
                MESSAGE_GUIDE_TARGET,
            ),
            (
                MobileModel::for_page(MobilePage::Messages),
                MobilePressedTarget::MessageOffline,
                MESSAGE_OFFLINE_TARGET,
            ),
        ] {
            assert_pressed_frame(model, target, target_rect);
        }
    }

    #[test]
    fn every_touch_target_is_phone_sized_and_centered_inside_the_display() {
        for target in [
            OVERVIEW_RECENT_TARGET,
            HOME_PHONE_TARGET,
            HOME_MESSAGES_TARGET,
            HOME_CALCULATOR_TARGET,
            HOME_SETTINGS_TARGET,
            APP_BACK_TARGET,
            SETTINGS_DISPLAY_TARGET,
            DISPLAY_THEME_TARGET,
            DISPLAY_ACCENT_TARGET,
            DISPLAY_DIMMING_TARGET,
            DISPLAY_ACCESSIBILITY_TARGET,
            ACCESSIBILITY_LARGE_TEXT_TARGET,
            ACCESSIBILITY_HIGH_CONTRAST_TARGET,
            SETTINGS_APPS_TARGET,
            SETTINGS_ABOUT_TARGET,
            QUICK_THEME_TARGET,
            QUICK_ACCENT_TARGET,
            QUICK_DIMMING_TARGET,
            QUICK_CLOSE_TARGET,
            DRAWER_PHONE_TARGET,
            DRAWER_MESSAGES_TARGET,
            DRAWER_CALCULATOR_TARGET,
            DRAWER_SETTINGS_TARGET,
            DRAWER_ANDROIDBOX_TARGET,
            DRAWER_HANDLE_TARGET,
            DRAWER_HOME_TARGET,
            ANDROIDBOX_EXECUTE_TARGET,
            PHONE_BACKSPACE_TARGET,
            MESSAGE_GUIDE_TARGET,
            MESSAGE_OFFLINE_TARGET,
        ] {
            assert!(target.width >= 88);
            assert!(target.height >= 88);
            assert!(usize::from(target.x) + usize::from(target.width) <= WIDTH);
            assert!(usize::from(target.y) + usize::from(target.height) <= HEIGHT);
            let center_x = target.x + target.width / 2;
            let center_y = target.y + target.height / 2;
            assert!(point_inside_visible_display(center_x, center_y));
        }
        for target in PHONE_KEY_TARGETS {
            assert!(target.width >= 88);
            assert!(target.height >= 88);
            assert!(usize::from(target.x) + usize::from(target.width) <= WIDTH);
            assert!(usize::from(target.y) + usize::from(target.height) <= HEIGHT);
            assert!(point_inside_visible_display(
                target.x + target.width / 2,
                target.y + target.height / 2
            ));
        }
        for target in CALCULATOR_KEY_TARGETS {
            assert!(target.width >= 88);
            assert!(target.height >= 88);
            assert!(usize::from(target.x) + usize::from(target.width) <= WIDTH);
            assert!(usize::from(target.y) + usize::from(target.height) <= HEIGHT);
            assert!(point_inside_visible_display(
                target.x + target.width / 2,
                target.y + target.height / 2
            ));
        }
    }

    #[test]
    fn launcher_drawer_and_calculator_targets_do_not_overlap() {
        fn overlaps(left: ShellRect, right: ShellRect) -> bool {
            let left_right = left.x + left.width;
            let right_right = right.x + right.width;
            let left_bottom = left.y + left.height;
            let right_bottom = right.y + right.height;
            left.x < right_right
                && right.x < left_right
                && left.y < right_bottom
                && right.y < left_bottom
        }

        let home_targets = [
            HOME_PHONE_TARGET,
            HOME_MESSAGES_TARGET,
            HOME_CALCULATOR_TARGET,
            HOME_SETTINGS_TARGET,
        ];
        for left in 0..home_targets.len() {
            for right in (left + 1)..home_targets.len() {
                assert!(!overlaps(home_targets[left], home_targets[right]));
            }
        }
        let drawer_targets = [
            DRAWER_PHONE_TARGET,
            DRAWER_MESSAGES_TARGET,
            DRAWER_CALCULATOR_TARGET,
            DRAWER_SETTINGS_TARGET,
            DRAWER_ANDROIDBOX_TARGET,
        ];
        for left in 0..drawer_targets.len() {
            for right in (left + 1)..drawer_targets.len() {
                assert!(!overlaps(drawer_targets[left], drawer_targets[right]));
            }
        }
        for (left, left_target) in CALCULATOR_KEY_TARGETS.iter().enumerate() {
            for (right, right_target) in CALCULATOR_KEY_TARGETS.iter().enumerate().skip(left + 1) {
                assert!(
                    !overlaps(*left_target, *right_target),
                    "calculator targets {left} and {right} overlap"
                );
            }
        }
    }

    #[test]
    fn masked_screen_corners_cannot_start_actions_or_shade_drags() {
        let model = MobileModel::default();
        for (x, y) in [
            (0, 0),
            (WIDTH as u16 - 1, 0),
            (0, HEIGHT as u16 - 1),
            (WIDTH as u16 - 1, HEIGHT as u16 - 1),
        ] {
            assert!(!point_inside_visible_display(x, y));
            let mut touch = TouchController::new();
            assert_eq!(touch.observe(model, x, y, true), None);
            assert_eq!(touch.observe(model, 360, 400, true), None);
            assert_eq!(touch.observe(model, 360, 400, false), None);
        }
        assert!(point_inside_visible_display(360, 0));
        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 0, true), None);
        assert_eq!(
            touch.observe(model, 360, 400, false),
            Some(MobileAction::OpenShade)
        );
    }

    #[test]
    fn drawer_gesture_is_quantized_finger_follow_and_settles_on_release() {
        let mut touch = TouchController::new();
        let mut model = MobileModel::default();
        assert_eq!(touch.observe(model, 360, 1_000, true), None);
        assert_eq!(
            touch.observe(model, 360, 760, true),
            Some(MobileAction::SetDrawerReveal(240))
        );
        assert!(model.apply(MobileAction::SetDrawerReveal(240)));
        assert_eq!(model.drawer_reveal_px, 240);
        assert!(!model.drawer_open);
        assert_eq!(
            touch.observe(model, 360, 760, false),
            Some(MobileAction::OpenDrawer)
        );
        assert!(model.apply(MobileAction::OpenDrawer));
        assert!(model.drawer_open);
        assert_eq!(model.drawer_reveal_px, 0);
        assert_eq!(model.effective_drawer_reveal_px(), DRAWER_REVEAL_MAX);

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 800, true), None);
        assert_eq!(
            touch.observe(model, 360, 1_040, true),
            Some(MobileAction::SetDrawerReveal(1_184))
        );
        assert!(model.apply(MobileAction::SetDrawerReveal(1_184)));
        assert_eq!(
            touch.observe(model, 360, 1_040, false),
            Some(MobileAction::CloseDrawer)
        );
        assert!(model.apply(MobileAction::CloseDrawer));
        assert!(!model.drawer_open);
        assert_eq!(model.effective_drawer_reveal_px(), 0);
    }

    #[test]
    fn short_drawer_drag_restores_home_exactly_and_diagonal_drag_is_rejected() {
        let mut model = MobileModel::default();
        let stable = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 1_000, true), None);
        let reveal = touch.observe(model, 360, 881, true).unwrap();
        assert_eq!(reveal, MobileAction::SetDrawerReveal(128));
        assert!(model.apply(reveal));
        assert_ne!(render_model(model), stable);
        assert_eq!(
            touch.observe(model, 360, 881, false),
            Some(MobileAction::CloseDrawer)
        );
        assert!(model.apply(MobileAction::CloseDrawer));
        assert_eq!(render_model(model), stable);

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 1_000, true), None);
        assert_eq!(touch.observe(model, 700, 700, true), None);
        assert_eq!(touch.observe(model, 700, 700, false), None);
        assert_eq!(render_model(model), stable);
    }

    #[test]
    fn drawer_reveal_has_an_exact_moving_boundary_and_reverse_rows_match() {
        let closed = render_model(MobileModel::default());
        let partial_model = MobileModel {
            drawer_reveal_px: 320,
            ..MobileModel::default()
        };
        let partial = render_model(partial_model);
        let boundary = usize::from(SYSTEM_NAV_TOP_PX - 320);
        assert_eq!(&partial[..boundary * WIDTH], &closed[..boundary * WIDTH]);
        assert!(
            partial[boundary * WIDTH..usize::from(SYSTEM_NAV_TOP_PX) * WIDTH]
                .iter()
                .zip(closed[boundary * WIDTH..usize::from(SYSTEM_NAV_TOP_PX) * WIDTH].iter())
                .any(|(drawer, home)| drawer != home)
        );
        assert_eq!(
            &partial[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..],
            &closed[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..]
        );

        let complete = render_model(partial_model);
        let mut reverse_rows = vec![0_u32; PIXEL_COUNT];
        let starts: Vec<_> = (0..HEIGHT).step_by(22).collect();
        for first_row in starts.into_iter().rev() {
            let row_count = (HEIGHT - first_row).min(22);
            render_rows(
                &mut reverse_rows[first_row * WIDTH..(first_row + row_count) * WIDTH],
                first_row,
                partial_model,
            )
            .unwrap();
        }
        assert_eq!(reverse_rows, complete);
    }

    #[test]
    fn drawer_targets_route_real_apps_and_remain_mutually_exclusive_with_shade() {
        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::OpenDrawer));
        let mut touch = TouchController::new();
        let pressed = touch.observe(model, 140, 440, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::DrawerPhone))
        );
        assert!(model.apply(pressed));
        assert_eq!(
            touch.observe(model, 140, 440, false),
            Some(MobileAction::Open(MobilePage::Phone))
        );

        let mut calculator_drawer = MobileModel::default();
        assert!(calculator_drawer.apply(MobileAction::OpenDrawer));
        let target = DRAWER_CALCULATOR_TARGET;
        let x = target.x + target.width / 2;
        let y = target.y + target.height / 2;
        let mut calculator_touch = TouchController::new();
        assert_eq!(
            calculator_touch.observe(calculator_drawer, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::DrawerCalculator
            )))
        );
        assert_eq!(
            calculator_touch.observe(calculator_drawer, x, y, false),
            Some(MobileAction::Open(MobilePage::Calculator))
        );

        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::OpenDrawer));
        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.shade_open);
        assert!(!model.drawer_open);
        assert_eq!(model.effective_drawer_reveal_px(), 0);
        assert!(model.apply(MobileAction::OpenDrawer));
        assert!(model.drawer_open);
        assert!(!model.shade_open);
        assert_eq!(model.effective_shade_reveal_px(), 0);
    }

    #[test]
    fn androidbox_is_a_half_open_second_drawer_row_target_and_opens_locally() {
        let target = DRAWER_ANDROIDBOX_TARGET;
        let center_x = target.x + target.width / 2;
        let center_y = target.y + target.height / 2;

        let partial = MobileModel {
            drawer_reveal_px: DRAWER_GESTURE_MIN_TRAVEL,
            ..MobileModel::default()
        };
        assert_eq!(hit_test(partial, center_x, center_y), None);

        let mut drawer = MobileModel {
            drawer_open: true,
            ..MobileModel::default()
        };
        assert_eq!(
            hit_test(drawer, target.x, target.y),
            Some(MobilePressedTarget::DrawerAndroidBoxDemo)
        );
        assert_eq!(
            hit_test(
                drawer,
                target.x + target.width - 1,
                target.y + target.height - 1,
            ),
            Some(MobilePressedTarget::DrawerAndroidBoxDemo)
        );
        assert_eq!(hit_test(drawer, target.x + target.width, center_y), None);
        assert_eq!(hit_test(drawer, center_x, target.y + target.height), None);

        let mut touch = TouchController::new();
        let down = touch.observe(drawer, center_x, center_y, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::DrawerAndroidBoxDemo))
        );
        assert!(drawer.apply(down));
        assert_eq!(
            touch.observe(drawer, center_x, center_y, false),
            Some(MobileAction::Open(MobilePage::AndroidDemo))
        );
        assert!(drawer.apply(MobileAction::Open(MobilePage::AndroidDemo)));
        assert_eq!(drawer.page, MobilePage::AndroidDemo);
        assert!(!drawer.drawer_open);
        assert_eq!(drawer.system_ui_mode, UiSystemUiMode::Home);
        assert!(!drawer.androidbox_manifest_verified);
        assert!(drawer.androidbox_launcher_activity.is_empty());
        assert!(!drawer.androidbox_on_create_completed);
        assert!(drawer.androidbox_text_view_content.is_empty());
        assert_eq!(drawer.androidbox_activity_instruction_count, 0);
        assert_eq!(
            drawer.androidbox_resources,
            AndroidBoxResourceStatus::empty()
        );
        assert_eq!(
            drawer.androidbox_activity_error,
            AndroidBoxActivityError::None
        );
    }

    #[test]
    fn androidbox_button_captures_release_cancels_on_leave_and_never_fakes_results() {
        let mut model = MobileModel::for_page(MobilePage::AndroidDemo);
        assert!(model.apply_androidbox_result(true, -42, 17, 3, AndroidBoxError::None));
        assert!(model.apply_androidbox_activity_result(
            true,
            AndroidBoxActivityIdentity::from_ascii("org.bndroid.demo.MainActivity").unwrap(),
            true,
            AndroidBoxTextViewContent::from_ascii("AndroidBox resource-backed view").unwrap(),
            11,
            verified_androidbox_resources(),
            AndroidBoxActivityError::None,
        ));
        let result = (
            model.androidbox_verified,
            model.androidbox_boot_value,
            model.androidbox_step_count,
            model.androidbox_tap_count,
            model.androidbox_error,
            model.androidbox_manifest_verified,
            model.androidbox_launcher_activity,
            model.androidbox_on_create_completed,
            model.androidbox_text_view_content,
            model.androidbox_activity_instruction_count,
            model.androidbox_resources,
            model.androidbox_activity_error,
        );
        let x = ANDROIDBOX_EXECUTE_TARGET.x + ANDROIDBOX_EXECUTE_TARGET.width / 2;
        let y = ANDROIDBOX_EXECUTE_TARGET.y + ANDROIDBOX_EXECUTE_TARGET.height / 2;

        let mut committed = TouchController::new();
        assert_eq!(
            committed.observe(model, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::AndroidBoxExecute
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::AndroidBoxExecute
        ))));
        assert_eq!(
            committed.observe(model, x, y, false),
            Some(MobileAction::ExecuteAndroidBoxDex)
        );
        assert!(model.apply(MobileAction::ExecuteAndroidBoxDex));
        assert_eq!(
            (
                model.androidbox_verified,
                model.androidbox_boot_value,
                model.androidbox_step_count,
                model.androidbox_tap_count,
                model.androidbox_error,
                model.androidbox_manifest_verified,
                model.androidbox_launcher_activity,
                model.androidbox_on_create_completed,
                model.androidbox_text_view_content,
                model.androidbox_activity_instruction_count,
                model.androidbox_resources,
                model.androidbox_activity_error,
            ),
            result
        );

        let mut cancelled = TouchController::new();
        assert_eq!(
            cancelled.observe(model, x, y, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::AndroidBoxExecute
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::AndroidBoxExecute
        ))));
        assert_eq!(
            cancelled.observe(model, 700, y, true),
            Some(MobileAction::SetPressed(None))
        );
        assert!(model.apply(MobileAction::SetPressed(None)));
        assert_eq!(cancelled.observe(model, x, y, true), None);
        assert_eq!(cancelled.observe(model, x, y, false), None);
        assert_eq!(
            (
                model.androidbox_verified,
                model.androidbox_boot_value,
                model.androidbox_step_count,
                model.androidbox_tap_count,
                model.androidbox_error,
                model.androidbox_manifest_verified,
                model.androidbox_launcher_activity,
                model.androidbox_on_create_completed,
                model.androidbox_text_view_content,
                model.androidbox_activity_instruction_count,
                model.androidbox_resources,
                model.androidbox_activity_error,
            ),
            result
        );
    }

    #[test]
    fn androidbox_keeps_launcher_navigation_back_and_shade_semantics() {
        let mut model = MobileModel::for_page(MobilePage::AndroidDemo);
        assert_eq!(model.system_ui_mode, UiSystemUiMode::Home);

        let mut shade = TouchController::new();
        assert_eq!(shade.observe(model, 360, 72, true), None);
        assert_eq!(
            shade.observe(model, 360, 400, false),
            Some(MobileAction::OpenShade)
        );
        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.shade_open);
        assert_eq!(model.page, MobilePage::AndroidDemo);
        assert!(model.apply(MobileAction::Back));
        assert!(!model.shade_open);
        assert_eq!(model.page, MobilePage::AndroidDemo);

        let mut edge_back = TouchController::new();
        assert_eq!(edge_back.observe(model, 32, 800, true), None);
        let reveal = edge_back.observe(model, 56, 800, true).unwrap();
        assert!(model.apply(reveal));
        assert_eq!(
            edge_back.observe(model, 176, 800, false),
            Some(MobileAction::Back)
        );
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, MobilePage::Home);
    }

    #[test]
    fn androidbox_activity_text_is_fixed_capacity_exact_and_renderable() {
        let identity =
            AndroidBoxActivityIdentity::from_ascii("org.bndroid.demo.MainActivity").unwrap();
        let content =
            AndroidBoxTextViewContent::from_ascii("AndroidBox resource-backed view").unwrap();
        assert_eq!(identity.as_str(), "org.bndroid.demo.MainActivity");
        assert_eq!(content.as_str(), "AndroidBox resource-backed view");
        assert!(!identity.is_empty());
        assert!(AndroidBoxActivityIdentity::empty().is_empty());
        assert_eq!(
            AndroidBoxResourceStatus::default(),
            AndroidBoxResourceStatus::empty()
        );
        assert!(androidbox_resources_complete(
            verified_androidbox_resources()
        ));

        let identity_too_wide = "W".repeat(ANDROIDBOX_ACTIVITY_IDENTITY_CAPACITY);
        assert_eq!(
            AndroidBoxActivityIdentity::from_ascii(&identity_too_wide),
            None
        );
        let identity_overflow = "A".repeat(ANDROIDBOX_ACTIVITY_IDENTITY_CAPACITY + 1);
        assert_eq!(
            AndroidBoxActivityIdentity::from_ascii(&identity_overflow),
            None
        );
        let text_view_too_wide = "W".repeat(ANDROIDBOX_TEXT_VIEW_FIRST_LINE_BYTES);
        assert_eq!(
            AndroidBoxTextViewContent::from_ascii(&text_view_too_wide),
            None
        );
        assert_eq!(
            AndroidBoxTextViewContent::from_ascii("line one\nline two"),
            None
        );
        assert_eq!(
            AndroidBoxTextViewContent::from_ascii("AndroidBox \u{00e9}"),
            None
        );
    }

    #[test]
    fn androidbox_resource_copy_fits_the_phone_grid_without_row_overlap() {
        let mut layout = MobileAsciiText::<17>::new();
        layout.push_bytes(b"layout ");
        layout.push_hex_u32(ANDROIDBOX_RESOURCE_LAYOUT_ID);
        let mut string = MobileAsciiText::<18>::new();
        string.push_bytes(b"@string ");
        string.push_hex_u32(ANDROIDBOX_RESOURCE_STRING_ID);
        assert_eq!(layout.as_str(), "layout 0x7f020000");
        assert_eq!(string.as_str(), "@string 0x7f030000");

        for (left, right) in [
            ("resources.arsc", "Parsed"),
            (layout.as_str(), "Resolved"),
            ("Binary XML", "Parsed"),
            ("TextView", "Verified"),
            (string.as_str(), "Mismatch"),
        ] {
            let left_end =
                36 * SCALE + i32::from(measure_mobile_text_px(MobileTextRole::Label, left));
            let right_start =
                316 * SCALE - i32::from(measure_mobile_text_px(MobileTextRole::Label, right));
            assert!(left_end + 16 <= right_start, "{left:?} overlaps {right:?}");
        }

        let line_height = i32::from(MobileTextRole::Label.line_height_px());
        for rows in [435, 457, 479, 501, 523].windows(2) {
            assert!(rows[0] * SCALE + line_height < rows[1] * SCALE);
        }
        assert!(523 * SCALE + line_height <= 552 * SCALE);
        assert!(
            86 * SCALE
                + i32::from(measure_mobile_text_px(
                    MobileTextRole::Title,
                    "Resource-backed Activity"
                ))
                <= 342 * SCALE
        );
        assert!(
            46 * SCALE
                + i32::from(measure_mobile_text_px(
                    MobileTextRole::Caption,
                    "AndroidBox resource-backed view"
                ))
                <= 326 * SCALE
        );
        assert!(692 * SCALE < i32::from(SYSTEM_NAV_TOP_PX));
    }

    #[test]
    fn androidbox_activity_result_is_runtime_only_and_changes_only_its_cards() {
        const ACTIVITY_AND_RESOURCE_CARDS: (usize, usize, usize, usize) = (36, 532, 684, 1_104);
        const ACTIVITY_CARD: (usize, usize, usize, usize) = (36, 532, 684, 804);

        let identity =
            AndroidBoxActivityIdentity::from_ascii("org.bndroid.demo.MainActivity").unwrap();
        let content =
            AndroidBoxTextViewContent::from_ascii("AndroidBox resource-backed view").unwrap();
        let resources = verified_androidbox_resources();
        let mut idle_model = MobileModel::for_page(MobilePage::AndroidDemo);
        assert!(idle_model.apply_androidbox_result(true, 20_260_729, 2, 0, AndroidBoxError::None,));
        let idle = render_model(idle_model);

        let mut success_model = idle_model;
        assert!(success_model.apply_androidbox_activity_result(
            true,
            identity,
            true,
            content,
            7,
            resources,
            AndroidBoxActivityError::None,
        ));
        assert!(!success_model.apply_androidbox_activity_result(
            true,
            identity,
            true,
            content,
            7,
            resources,
            AndroidBoxActivityError::None,
        ));
        assert!(androidbox_activity_complete(&success_model));
        assert_eq!(
            (
                success_model.androidbox_manifest_verified,
                success_model.androidbox_launcher_activity.as_str(),
                success_model.androidbox_on_create_completed,
                success_model.androidbox_text_view_content.as_str(),
                success_model.androidbox_activity_instruction_count,
                success_model.androidbox_resources,
                success_model.androidbox_activity_error,
            ),
            (
                true,
                "org.bndroid.demo.MainActivity",
                true,
                "AndroidBox resource-backed view",
                7,
                resources,
                AndroidBoxActivityError::None,
            )
        );
        let success = render_model(success_model);
        assert_pixel_differences_are_bounded(&idle, &success, &[ACTIVITY_AND_RESOURCE_CARDS]);

        let mut alternate_content_model = success_model;
        assert!(alternate_content_model.apply_androidbox_activity_result(
            true,
            identity,
            true,
            AndroidBoxTextViewContent::from_ascii("AndroidBox real TextView").unwrap(),
            7,
            resources,
            AndroidBoxActivityError::None,
        ));
        assert!(!androidbox_activity_complete(&alternate_content_model));
        let alternate_content = render_model(alternate_content_model);
        assert_pixel_differences_are_bounded(
            &success,
            &alternate_content,
            &[ACTIVITY_AND_RESOURCE_CARDS],
        );

        let mut contradictory_model = idle_model;
        assert!(contradictory_model.apply_androidbox_activity_result(
            true,
            AndroidBoxActivityIdentity::from_ascii("X").unwrap(),
            true,
            content,
            7,
            resources,
            AndroidBoxActivityError::None,
        ));
        assert!(!androidbox_launcher_manifest_verified(&contradictory_model));
        assert!(!androidbox_activity_complete(&contradictory_model));
        let contradictory = render_model(contradictory_model);
        assert_pixel_differences_are_bounded(&idle, &contradictory, &[ACTIVITY_AND_RESOURCE_CARDS]);

        let partial_resources = AndroidBoxResourceStatus {
            resource_table_parsed: true,
            ..AndroidBoxResourceStatus::empty()
        };
        let mut partial_model = idle_model;
        assert!(partial_model.apply_androidbox_activity_result(
            true,
            identity,
            true,
            content,
            7,
            partial_resources,
            AndroidBoxActivityError::None,
        ));
        assert!(
            androidbox_on_create_complete(&partial_model),
            "resource pending must not rewrite a completed onCreate"
        );
        assert!(!androidbox_activity_complete(&partial_model));
        let partial = render_model(partial_model);
        assert_pixel_differences_are_bounded(&idle, &partial, &[ACTIVITY_AND_RESOURCE_CARDS]);

        let mut on_create_pending_model = partial_model;
        assert!(on_create_pending_model.apply_androidbox_activity_result(
            true,
            identity,
            false,
            content,
            7,
            partial_resources,
            AndroidBoxActivityError::None,
        ));
        assert!(!androidbox_on_create_complete(&on_create_pending_model));
        let on_create_pending = render_model(on_create_pending_model);
        assert_pixel_differences_are_bounded(&partial, &on_create_pending, &[ACTIVITY_CARD]);

        let mut failed_model = idle_model;
        assert!(failed_model.apply_androidbox_activity_result(
            false,
            AndroidBoxActivityIdentity::empty(),
            false,
            AndroidBoxTextViewContent::empty(),
            3,
            AndroidBoxResourceStatus::empty(),
            AndroidBoxActivityError::ManifestRejected,
        ));
        let failed = render_model(failed_model);
        assert_pixel_differences_are_bounded(&idle, &failed, &[ACTIVITY_AND_RESOURCE_CARDS]);
        assert_ne!(success, failed);

        for frame in [
            &success,
            &alternate_content,
            &contradictory,
            &partial,
            &on_create_pending,
            &failed,
        ] {
            assert_eq!(&frame[..532 * WIDTH], &idle[..532 * WIDTH]);
            assert_eq!(
                &frame[1_104 * WIDTH..],
                &idle[1_104 * WIDTH..],
                "Activity/resource reports must not change DEX controls or system chrome"
            );
        }
    }

    #[test]
    fn androidbox_activity_success_requires_every_resource_fact_and_identity() {
        let identity =
            AndroidBoxActivityIdentity::from_ascii("org.bndroid.demo.MainActivity").unwrap();
        let content =
            AndroidBoxTextViewContent::from_ascii("AndroidBox resource-backed view").unwrap();
        let verified = verified_androidbox_resources();
        let incomplete_resources = [
            AndroidBoxResourceStatus {
                resource_table_parsed: false,
                ..verified
            },
            AndroidBoxResourceStatus {
                layout_entry_resolved: false,
                ..verified
            },
            AndroidBoxResourceStatus {
                binary_xml_parsed: false,
                ..verified
            },
            AndroidBoxResourceStatus {
                text_view_verified: false,
                ..verified
            },
            AndroidBoxResourceStatus {
                string_reference_resolved: false,
                ..verified
            },
            AndroidBoxResourceStatus {
                layout_resource_id: ANDROIDBOX_RESOURCE_LAYOUT_ID ^ 1,
                ..verified
            },
            AndroidBoxResourceStatus {
                string_resource_id: ANDROIDBOX_RESOURCE_STRING_ID ^ 1,
                ..verified
            },
        ];

        for resources in incomplete_resources {
            assert!(!androidbox_resources_complete(resources));
            let mut model = MobileModel::for_page(MobilePage::AndroidDemo);
            assert!(model.apply_androidbox_result(true, 1, 2, 0, AndroidBoxError::None));
            assert!(model.apply_androidbox_activity_result(
                true,
                identity,
                true,
                content,
                4,
                resources,
                AndroidBoxActivityError::None,
            ));
            assert!(androidbox_on_create_complete(&model));
            assert!(!androidbox_activity_complete(&model), "{resources:?}");
        }

        let mut model = MobileModel::for_page(MobilePage::AndroidDemo);
        assert!(model.apply_androidbox_result(true, 1, 2, 0, AndroidBoxError::None));
        assert!(model.apply_androidbox_activity_result(
            true,
            identity,
            true,
            content,
            4,
            verified,
            AndroidBoxActivityError::None,
        ));
        assert!(androidbox_activity_complete(&model));

        let mut missing_dex = model;
        assert!(missing_dex.apply_androidbox_result(false, 1, 2, 0, AndroidBoxError::None));
        assert!(!androidbox_activity_complete(&missing_dex));

        let mut dex_error = model;
        assert!(dex_error.apply_androidbox_result(
            true,
            1,
            2,
            0,
            AndroidBoxError::VerificationFailed,
        ));
        assert!(!androidbox_activity_complete(&dex_error));

        for (manifest, launcher, on_create, view, instructions, error) in [
            (
                false,
                identity,
                true,
                content,
                4,
                AndroidBoxActivityError::None,
            ),
            (
                true,
                AndroidBoxActivityIdentity::from_ascii("X").unwrap(),
                true,
                content,
                4,
                AndroidBoxActivityError::None,
            ),
            (
                true,
                identity,
                false,
                content,
                4,
                AndroidBoxActivityError::None,
            ),
            (
                true,
                identity,
                true,
                AndroidBoxTextViewContent::from_ascii("AndroidBox real TextView").unwrap(),
                4,
                AndroidBoxActivityError::None,
            ),
            (
                true,
                identity,
                true,
                content,
                0,
                AndroidBoxActivityError::None,
            ),
            (
                true,
                identity,
                true,
                content,
                4,
                AndroidBoxActivityError::UnsupportedFrameworkCall,
            ),
        ] {
            let mut contradictory = model;
            assert!(contradictory.apply_androidbox_activity_result(
                manifest,
                launcher,
                on_create,
                view,
                instructions,
                verified,
                error,
            ));
            assert!(!androidbox_activity_complete(&contradictory));
        }
    }

    #[test]
    fn androidbox_runtime_results_change_only_declared_content_and_preserve_chrome() {
        const RESULT_AND_BUTTON: (usize, usize, usize, usize) = (36, 852, 684, 1_384);
        const TAP_VALUE: (usize, usize, usize, usize) = (620, 1_140, 652, 1_216);

        let idle_model = MobileModel::for_page(MobilePage::AndroidDemo);
        let idle = render_model(idle_model);

        let mut success_model = idle_model;
        assert!(success_model.apply_androidbox_result(true, -42, 17, 1, AndroidBoxError::None));
        assert!(!success_model.apply_androidbox_result(true, -42, 17, 1, AndroidBoxError::None));
        let success = render_model(success_model);
        assert_pixel_differences_are_bounded(&idle, &success, &[RESULT_AND_BUTTON]);

        let mut failed_model = idle_model;
        assert!(failed_model.apply_androidbox_result(false, 0, 3, 0, AndroidBoxError::InvalidDex));
        let failed = render_model(failed_model);
        assert_pixel_differences_are_bounded(&idle, &failed, &[RESULT_AND_BUTTON]);
        assert_ne!(success, failed);

        let mut tapped_model = success_model;
        assert!(tapped_model.apply_androidbox_result(true, -42, 17, 2, AndroidBoxError::None));
        let tapped = render_model(tapped_model);
        assert_pixel_differences_are_bounded(&success, &tapped, &[TAP_VALUE]);

        for frame in [&success, &failed, &tapped] {
            assert_eq!(&frame[..64 * WIDTH], &idle[..64 * WIDTH]);
            assert_eq!(
                &frame[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..],
                &idle[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..]
            );
        }
    }

    #[test]
    fn top_edge_swipe_opens_shade_and_upward_swipe_closes_it() {
        let mut touch = TouchController::new();
        let mut model = MobileModel::default();
        assert_eq!(touch.observe(model, 360, 72, true), None);
        assert_eq!(
            touch.observe(model, 366, 420, false),
            Some(MobileAction::OpenShade)
        );
        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.shade_open);

        let down = touch.observe(model, 360, 1_120, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::BootNotification))
        );
        assert!(model.apply(down));
        assert_eq!(
            touch.observe(model, 350, 720, false),
            Some(MobileAction::CloseShade)
        );
        assert!(model.apply(MobileAction::CloseShade));
        assert!(!model.shade_open);
    }

    #[test]
    fn overview_reserves_top_edge_for_the_same_finger_follow_shade() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        let overview = render_model(model);

        assert_eq!(hit_test(model, 360, 0), None);
        assert_eq!(hit_test(model, 360, SHADE_GESTURE_START_MAX_Y - 1), None);
        assert_eq!(
            hit_test(model, 360, SHADE_GESTURE_START_MAX_Y),
            Some(MobilePressedTarget::OverviewBackground)
        );

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 80, true), None);
        let reveal = touch.observe(model, 360, 700, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(620));
        assert!(model.apply(reveal));
        assert_eq!(model.effective_shade_reveal_px(), 620);
        assert_ne!(render_model(model), overview);

        assert_eq!(
            touch.observe(model, 360, 700, false),
            Some(MobileAction::OpenShade)
        );
        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.overview_open());
        assert_eq!(model.effective_shade_reveal_px(), SHADE_REVEAL_MAX);

        // The settled opaque system sheet must not leak any Overview pixels.
        let mut home = MobileModel::default();
        assert!(home.apply(MobileAction::OpenShade));
        assert_eq!(render_model(model), render_model(home));
    }

    #[test]
    fn shade_gesture_rejects_short_diagonal_and_non_edge_drags() {
        let model = MobileModel::default();
        for ((start_x, start_y), (end_x, end_y)) in [
            ((360, 72), (360, 250)),
            ((360, 72), (700, 420)),
            ((360, 180), (360, 520)),
        ] {
            let mut touch = TouchController::new();
            assert_eq!(touch.observe(model, start_x, start_y, true), None);
            assert_eq!(touch.observe(model, end_x, end_y, false), None);
        }
    }

    #[test]
    fn shade_gesture_thresholds_are_exact_and_settle_only_on_release() {
        let model = MobileModel::default();
        let mut exact = TouchController::new();
        assert_eq!(exact.observe(model, 300, 119, true), None);
        assert_eq!(
            exact.observe(model, 420, 359, true),
            Some(MobileAction::SetShadeReveal(240))
        );
        assert_eq!(
            exact.observe(model, 420, 359, false),
            Some(MobileAction::OpenShade)
        );

        let mut outside_edge = TouchController::new();
        assert_eq!(outside_edge.observe(model, 360, 120, true), None);
        assert_eq!(outside_edge.observe(model, 360, 360, true), None);
        assert_eq!(outside_edge.observe(model, 360, 360, false), None);

        let mut short = TouchController::new();
        assert_eq!(short.observe(model, 360, 119, true), None);
        assert_eq!(
            short.observe(model, 360, 358, true),
            Some(MobileAction::SetShadeReveal(239))
        );
        assert_eq!(short.observe(model, 360, 358, false), None);
    }

    #[test]
    fn shade_drag_renders_intermediate_frames_and_short_release_restores() {
        let mut model = MobileModel::default();
        let closed = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 72, true), None);
        let reveal = touch.observe(model, 362, 192, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(120));
        assert!(model.apply(reveal));
        assert!(!model.shade_open);
        assert_eq!(model.effective_shade_reveal_px(), 120);
        let opening = render_model(model);
        assert_ne!(opening, closed);

        assert_eq!(
            touch.observe(model, 362, 192, false),
            Some(MobileAction::CloseShade)
        );
        assert!(model.apply(MobileAction::CloseShade));
        assert_eq!(model.effective_shade_reveal_px(), 0);
        assert_eq!(render_model(model), closed);

        assert!(model.apply(MobileAction::OpenShade));
        let open = render_model(model);
        let mut touch = TouchController::new();
        let down = touch.observe(model, 360, 1_120, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::BootNotification))
        );
        assert!(model.apply(down));
        let reveal = touch.observe(model, 358, 1_000, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(SHADE_REVEAL_MAX - 120));
        assert!(model.apply(reveal));
        assert!(model.shade_open);
        let closing = render_model(model);
        assert_ne!(closing, open);
        assert_ne!(closing, closed);

        assert_eq!(
            touch.observe(model, 358, 1_000, false),
            Some(MobileAction::OpenShade)
        );
        assert!(model.apply(MobileAction::OpenShade));
        assert_eq!(model.effective_shade_reveal_px(), SHADE_REVEAL_MAX);
        assert_eq!(render_model(model), open);
    }

    #[test]
    fn short_shade_drag_over_open_drawer_restores_exact_drawer_frame() {
        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::OpenDrawer));
        let settled_drawer = render_model(model);

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 72, true), None);
        let reveal = touch.observe(model, 362, 192, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(120));
        assert!(model.apply(reveal));
        assert!(!model.shade_open);
        assert_eq!(model.effective_shade_reveal_px(), 120);
        assert!(!model.drawer_open);
        assert_eq!(model.effective_drawer_reveal_px(), 0);

        let transient = render_model(model);
        let shade_over_home = render_model(MobileModel {
            shade_reveal_px: 120,
            ..MobileModel::default()
        });
        assert_eq!(transient, shade_over_home);
        assert_ne!(transient, settled_drawer);

        assert_eq!(
            touch.observe(model, 362, 192, false),
            Some(MobileAction::OpenDrawer)
        );
        assert!(model.apply(MobileAction::OpenDrawer));
        assert!(model.drawer_open);
        assert_eq!(model.drawer_reveal_px, 0);
        assert!(!model.shade_open);
        assert_eq!(model.shade_reveal_px, 0);
        assert_eq!(render_model(model), settled_drawer);
    }

    #[test]
    fn shade_opened_from_drawer_stays_mutually_exclusive_and_matches_exact_frame() {
        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::OpenDrawer));

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 72, true), None);
        let reveal = touch.observe(model, 360, 312, true).unwrap();
        assert_eq!(
            reveal,
            MobileAction::SetShadeReveal(SHADE_GESTURE_MIN_TRAVEL)
        );
        assert!(model.apply(reveal));
        assert!(!model.drawer_open);
        assert_eq!(model.effective_drawer_reveal_px(), 0);
        assert!(!model.shade_open);
        assert_eq!(model.effective_shade_reveal_px(), SHADE_GESTURE_MIN_TRAVEL);

        assert_eq!(
            touch.observe(model, 360, 312, false),
            Some(MobileAction::OpenShade)
        );
        assert!(model.apply(MobileAction::OpenShade));
        assert!(model.shade_open);
        assert_eq!(model.shade_reveal_px, 0);
        assert!(!model.drawer_open);
        assert_eq!(model.drawer_reveal_px, 0);

        let mut expected = MobileModel::default();
        assert!(expected.apply(MobileAction::OpenShade));
        assert_eq!(model, expected);
        assert_eq!(render_model(model), render_model(expected));
    }

    #[test]
    fn shade_drag_reveal_is_bounded_and_diagonal_cancellation_is_stable() {
        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::SetShadeReveal(u16::MAX)));
        assert_eq!(model.shade_reveal_px, SHADE_REVEAL_MAX);
        assert_eq!(model.effective_shade_reveal_px(), SHADE_REVEAL_MAX);
        assert!(!model.apply(MobileAction::SetShadeReveal(u16::MAX)));
        assert!(model.apply(MobileAction::CloseShade));

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 72, true), None);
        let reveal = touch.observe(model, 360, 300, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(228));
        assert!(model.apply(reveal));
        assert_eq!(
            touch.observe(model, 700, 300, true),
            Some(MobileAction::SetShadeReveal(0))
        );
        assert!(model.apply(MobileAction::SetShadeReveal(0)));
        assert_eq!(model.effective_shade_reveal_px(), 0);
        assert_eq!(touch.observe(model, 700, 300, false), None);

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 200, 1, true), None);
        assert_eq!(
            touch.observe(model, 200, u16::MAX, true),
            Some(MobileAction::SetShadeReveal(SHADE_REVEAL_MAX))
        );
    }

    #[test]
    fn open_shade_intercepts_apps_and_exposes_only_quick_controls() {
        let mut model = MobileModel::default();
        assert!(model.apply(MobileAction::OpenShade));
        let mut touch = TouchController::new();

        assert_eq!(touch.observe(model, 160, 1_412, true), None);
        assert_eq!(touch.observe(model, 160, 1_412, false), None);

        let pressed = touch.observe(model, 180, 380, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::Theme))
        );
        assert!(model.apply(pressed));
        assert_eq!(
            touch.observe(model, 180, 380, false),
            Some(MobileAction::ToggleTheme)
        );
        assert!(model.apply(MobileAction::ToggleTheme));
        let pressed = touch.observe(model, 520, 380, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::Accent))
        );
        assert!(model.apply(pressed));
        assert_eq!(
            touch.observe(model, 520, 380, false),
            Some(MobileAction::ToggleAccent)
        );
        assert!(model.apply(MobileAction::ToggleAccent));
        let pressed = touch.observe(model, 648, 160, true).unwrap();
        assert_eq!(
            pressed,
            MobileAction::SetPressed(Some(MobilePressedTarget::ShadeClose))
        );
        assert!(model.apply(pressed));
        assert_eq!(
            touch.observe(model, 648, 160, false),
            Some(MobileAction::CloseShade)
        );
        assert!(model.apply(MobileAction::CloseShade));
        assert_eq!(model.pressed_target, None);
    }

    #[test]
    fn boot_notification_visibility_changes_only_its_lock_and_shade_regions() {
        let lock_visible = render_model(MobileModel::locked());
        let lock_dismissed = render_model(MobileModel {
            boot_notification_visible: false,
            ..MobileModel::locked()
        });
        assert_pixel_differences_are_bounded(
            &lock_visible,
            &lock_dismissed,
            &[(48, 652, 672, 892)],
        );

        let shade_visible = render_model(MobileModel {
            shade_open: true,
            ..MobileModel::for_page(MobilePage::Settings)
        });
        let shade_dismissed = render_model(MobileModel {
            shade_open: true,
            boot_notification_visible: false,
            ..MobileModel::for_page(MobilePage::Settings)
        });
        assert_pixel_differences_are_bounded(
            &shade_visible,
            &shade_dismissed,
            &[(32, 904, 688, 1_248)],
        );
        assert_eq!(
            &shade_visible[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..],
            &shade_dismissed[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..]
        );
    }

    #[test]
    fn boot_notification_hit_bounds_lock_guard_and_settings_about_are_exact() {
        let shade = MobileModel {
            shade_open: true,
            ..MobileModel::for_page(MobilePage::Settings)
        };
        for (x, y) in [(32, 904), (687, 1_135), (360, 1_020)] {
            assert_eq!(
                hit_test(shade, x, y),
                Some(MobilePressedTarget::BootNotification)
            );
        }
        for (x, y) in [(31, 904), (688, 1_135), (360, 903), (360, 1_136)] {
            assert_ne!(
                hit_test(shade, x, y),
                Some(MobilePressedTarget::BootNotification)
            );
        }
        let stable_lock = MobileModel::locked();
        for (x, y) in [(48, 652), (671, 883), (360, 770)] {
            assert_eq!(
                hit_test(stable_lock, x, y),
                Some(MobilePressedTarget::BootNotification)
            );
        }
        for (x, y) in [(47, 652), (672, 883), (360, 651), (360, 884)] {
            assert_ne!(
                hit_test(stable_lock, x, y),
                Some(MobilePressedTarget::BootNotification)
            );
        }

        let mut stable_lock_tap = stable_lock;
        let mut stable_lock_touch = TouchController::new();
        let down = stable_lock_touch
            .observe(stable_lock_tap, 360, 770, true)
            .unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::BootNotification))
        );
        assert!(stable_lock_tap.apply(down));
        assert_eq!(
            stable_lock_touch.observe(stable_lock_tap, 360, 770, false),
            Some(MobileAction::SetPressed(None))
        );
        assert!(stable_lock_tap.apply(MobileAction::SetPressed(None)));
        assert_eq!(stable_lock_tap.page, MobilePage::Lock);
        assert!(stable_lock_tap.boot_notification_visible);

        let mut settings = shade;
        let mut settings_touch = TouchController::new();
        let down = settings_touch.observe(settings, 360, 1_020, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::BootNotification))
        );
        assert!(settings.apply(down));
        assert_eq!(
            settings_touch.observe(settings, 360, 1_020, false),
            Some(MobileAction::ActivateBootNotification)
        );
        assert!(settings.apply(MobileAction::ActivateBootNotification));
        assert_eq!(settings.page, MobilePage::About);
        assert!(!settings.shade_open);

        let mut lock = MobileModel {
            shade_open: true,
            ..MobileModel::locked()
        };
        let mut lock_touch = TouchController::new();
        let down = lock_touch.observe(lock, 360, 1_020, true).unwrap();
        assert!(lock.apply(down));
        assert_eq!(
            lock_touch.observe(lock, 360, 1_020, false),
            Some(MobileAction::SetPressed(None))
        );
        assert!(lock.apply(MobileAction::SetPressed(None)));
        assert_eq!(lock.page, MobilePage::Lock);
        assert!(lock.shade_open);
    }

    #[test]
    fn boot_notification_swipe_thresholds_are_symmetric_and_restore_exactly() {
        let base = MobileModel {
            shade_open: true,
            ..MobileModel::default()
        };
        let stable = render_model(base);

        let mut short_model = base;
        let mut short = TouchController::new();
        let down = short.observe(short_model, 360, 1_020, true).unwrap();
        assert!(short_model.apply(down));
        let drag = short.observe(short_model, 519, 1_020, true).unwrap();
        assert_eq!(drag, MobileAction::SetBootNotificationOffset(159));
        assert!(short_model.apply(drag));
        assert_eq!(short_model.boot_notification_offset_px, 152);
        let transient = render_model(short_model);
        assert_ne!(transient, stable);
        assert_pixel_differences_are_bounded(&stable, &transient, &[(32, 904, 720, 1_144)]);
        assert_eq!(
            short.observe(short_model, 519, 1_020, false),
            Some(MobileAction::SetBootNotificationOffset(0))
        );
        assert!(short_model.apply(MobileAction::SetBootNotificationOffset(0)));
        assert_eq!(render_model(short_model), stable);

        for end_x in [200, 520] {
            let mut model = base;
            let mut touch = TouchController::new();
            let down = touch.observe(model, 360, 1_020, true).unwrap();
            assert!(model.apply(down));
            let drag = touch.observe(model, end_x, 1_020, true).unwrap();
            assert!(matches!(
                drag,
                MobileAction::SetBootNotificationOffset(offset)
                    if offset.unsigned_abs() == 160
            ));
            assert!(model.apply(drag));
            assert_eq!(
                touch.observe(model, end_x, 1_020, false),
                Some(MobileAction::DismissBootNotification)
            );
            assert!(model.apply(MobileAction::DismissBootNotification));
            assert!(!model.boot_notification_visible);
            assert_eq!(model.boot_notification_offset_px, 0);
        }
    }

    #[test]
    #[cfg(feature = "mobile-ui-runtime")]
    fn lock_notification_reuses_swipe_thresholds_without_unlock_authority() {
        let base = MobileModel::locked();
        let stable = render_model(base);

        let mut short_model = base;
        let mut short = TouchController::new();
        let down = short.observe(short_model, 360, 770, true).unwrap();
        assert_eq!(
            down,
            MobileAction::SetPressed(Some(MobilePressedTarget::BootNotification))
        );
        assert!(short_model.apply(down));
        let drag = short.observe(short_model, 519, 770, true).unwrap();
        assert_eq!(drag, MobileAction::SetBootNotificationOffset(159));
        assert!(short_model.apply(drag));
        assert_eq!(short_model.boot_notification_offset_px, 152);
        assert_eq!(short_model.page, MobilePage::Lock);
        assert_eq!(short_model.effective_unlock_reveal_px(), 0);
        let transient = render_model(short_model);
        assert_ne!(transient, stable);
        assert_pixel_differences_are_bounded(&stable, &transient, &[(48, 652, 720, 892)]);
        let short_damage = DamageRect {
            x: 48,
            y: 652,
            width: 672,
            height: 240,
        };
        assert_eq!(
            damage_plan(Some(base), short_model),
            MobileDamagePlan::Regions(DamageRegions::single(short_damage).unwrap())
        );
        let mut damaged = stable.clone();
        render_damage(&mut damaged, short_model, short_damage).unwrap();
        assert_eq!(damaged, transient);
        assert_eq!(
            short.observe(short_model, 519, 770, false),
            Some(MobileAction::SetBootNotificationOffset(0))
        );
        let held_short = short_model;
        assert!(short_model.apply(MobileAction::SetBootNotificationOffset(0)));
        assert_eq!(
            damage_plan(Some(held_short), short_model),
            MobileDamagePlan::Regions(DamageRegions::single(short_damage).unwrap())
        );
        assert_eq!(render_model(short_model), stable);

        let mut dismissed = base;
        let mut swipe = TouchController::new();
        let down = swipe.observe(dismissed, 360, 770, true).unwrap();
        assert!(dismissed.apply(down));
        let drag = swipe.observe(dismissed, 200, 770, true).unwrap();
        assert_eq!(drag, MobileAction::SetBootNotificationOffset(-160));
        assert!(dismissed.apply(drag));
        assert_eq!(
            swipe.observe(dismissed, 200, 770, false),
            Some(MobileAction::DismissBootNotification)
        );
        let held_dismiss = dismissed;
        assert!(dismissed.apply(MobileAction::DismissBootNotification));
        assert!(!dismissed.boot_notification_visible);
        assert_eq!(dismissed.page, MobilePage::Lock);
        assert_eq!(dismissed.effective_unlock_reveal_px(), 0);
        let dismiss_damage = DamageRect {
            x: 0,
            y: 652,
            width: 672,
            height: 240,
        };
        assert_eq!(
            damage_plan(Some(held_dismiss), dismissed),
            MobileDamagePlan::Regions(DamageRegions::single(dismiss_damage).unwrap())
        );
        let mut damage_dismissed = render_model(held_dismiss);
        render_damage(&mut damage_dismissed, dismissed, dismiss_damage).unwrap();
        assert_eq!(damage_dismissed, render_model(dismissed));

        let mut vertical_model = base;
        let mut vertical = TouchController::new();
        let down = vertical.observe(vertical_model, 360, 770, true).unwrap();
        assert!(vertical_model.apply(down));
        assert_eq!(
            vertical.observe(vertical_model, 360, 520, true),
            Some(MobileAction::SetPressed(None))
        );
        assert!(vertical_model.apply(MobileAction::SetPressed(None)));
        assert_eq!(vertical.observe(vertical_model, 360, 520, false), None);
        assert_eq!(vertical_model.page, MobilePage::Lock);
        assert_eq!(vertical_model.effective_unlock_reveal_px(), 0);
        assert!(vertical_model.boot_notification_visible);
    }

    #[test]
    fn boot_notification_vertical_drag_yields_to_shade_and_cancel_restores() {
        let mut model = MobileModel {
            shade_open: true,
            ..MobileModel::default()
        };
        let stable = render_model(model);
        let mut vertical = TouchController::new();
        let down = vertical.observe(model, 360, 1_020, true).unwrap();
        assert!(model.apply(down));
        let reveal = vertical.observe(model, 360, 780, true).unwrap();
        assert_eq!(reveal, MobileAction::SetShadeReveal(SHADE_REVEAL_MAX - 240));
        assert!(model.apply(reveal));
        assert_eq!(
            vertical.observe(model, 360, 780, false),
            Some(MobileAction::CloseShade)
        );

        let mut model = MobileModel {
            shade_open: true,
            ..MobileModel::default()
        };
        let mut cancelled = TouchController::new();
        let down = cancelled.observe(model, 360, 1_020, true).unwrap();
        assert!(model.apply(down));
        let drag = cancelled.observe(model, 440, 1_020, true).unwrap();
        assert!(model.apply(drag));
        let clear = cancelled.cancel_contact(model).unwrap();
        assert_eq!(clear, MobileAction::SetBootNotificationOffset(0));
        assert!(model.apply(clear));
        assert_eq!(render_model(model), stable);
        assert_eq!(cancelled.observe(model, 520, 1_020, true), None);
        assert_eq!(cancelled.observe(model, 520, 1_020, false), None);
    }

    #[test]
    #[cfg(feature = "mobile-ui-runtime")]
    fn boot_notification_offset_is_row_partition_independent_and_dismissal_stays_local() {
        let shade_stable = MobileModel {
            shade_open: true,
            ..MobileModel::default()
        };
        let offset = MobileModel {
            shade_open: true,
            boot_notification_offset_px: -80,
            ..MobileModel::default()
        };
        let expected = render_model(offset);
        let offset_damage = DamageRect {
            x: 0,
            y: 904,
            width: 688,
            height: 240,
        };
        assert_eq!(
            damage_plan(Some(shade_stable), offset),
            MobileDamagePlan::Regions(DamageRegions::single(offset_damage).unwrap())
        );
        let mut damaged = render_model(shade_stable);
        render_damage(&mut damaged, offset, offset_damage).unwrap();
        assert_eq!(damaged, expected);
        let mut rows = vec![0_u32; PIXEL_COUNT];
        let starts: Vec<_> = (0..HEIGHT).step_by(17).collect();
        for first_row in starts.into_iter().rev() {
            let row_count = (HEIGHT - first_row).min(17);
            render_rows(
                &mut rows[first_row * WIDTH..(first_row + row_count) * WIDTH],
                first_row,
                offset,
            )
            .unwrap();
        }
        assert_eq!(rows, expected);

        let mut dismissed = offset;
        assert!(dismissed.apply(MobileAction::DismissBootNotification));
        let dismiss_damage = DamageRect {
            x: 0,
            y: 904,
            width: 688,
            height: 344,
        };
        assert_eq!(
            damage_plan(Some(offset), dismissed),
            MobileDamagePlan::Regions(DamageRegions::single(dismiss_damage).unwrap())
        );
        let mut damage_dismissed = expected.clone();
        render_damage(&mut damage_dismissed, dismissed, dismiss_damage).unwrap();
        assert_eq!(damage_dismissed, render_model(dismissed));
        assert!(dismissed.apply(MobileAction::ToggleTheme));
        assert!(dismissed.apply(MobileAction::ToggleAccent));
        assert!(!dismissed.boot_notification_visible);
        assert_eq!(dismissed.boot_notification_offset_px, 0);
        dismissed.page = MobilePage::Phone;
        dismissed.shade_open = false;
        let phone = render_model(dismissed);
        dismissed.boot_notification_visible = true;
        assert_eq!(phone, render_model(dismissed));
    }

    #[test]
    fn shade_render_and_controls_produce_distinct_frames() {
        let mut model = MobileModel::default();
        let home = render_model(model);
        assert!(model.apply(MobileAction::OpenShade));
        let shade_dark = render_model(model);
        assert_ne!(digest(&home), digest(&shade_dark));

        assert!(model.apply(MobileAction::ToggleTheme));
        assert!(model.shade_open);
        let shade_light = render_model(model);
        assert_ne!(digest(&shade_dark), digest(&shade_light));

        assert!(model.apply(MobileAction::ToggleAccent));
        assert!(model.shade_open);
        let shade_alternate = render_model(model);
        assert_ne!(digest(&shade_light), digest(&shade_alternate));

        let page = model.page;
        assert!(model.apply(MobileAction::Back));
        assert_eq!(model.page, page);
        assert!(!model.shade_open);
        let themed_home = render_model(model);
        assert_ne!(digest(&home), digest(&themed_home));
        assert_ne!(digest(&shade_alternate), digest(&themed_home));
    }

    #[test]
    fn page_transition_timeline_is_versioned_bounded_and_monotonic() {
        assert_eq!(PAGE_TRANSITION_VERSION, 1);
        assert_eq!(
            PAGE_TRANSITION_ENTER_OFFSETS.len(),
            PAGE_TRANSITION_EXIT_OFFSETS.len()
        );
        assert!(PAGE_TRANSITION_ENTER_OFFSETS.len() >= 4);
        assert!(
            PAGE_TRANSITION_ENTER_OFFSETS
                .windows(2)
                .all(|pair| pair[0] > pair[1])
        );
        assert!(
            PAGE_TRANSITION_EXIT_OFFSETS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert_eq!(
            PAGE_TRANSITION_ENTER_OFFSETS,
            [
                PAGE_TRANSITION_EXIT_OFFSETS[3],
                PAGE_TRANSITION_EXIT_OFFSETS[2],
                PAGE_TRANSITION_EXIT_OFFSETS[1],
                PAGE_TRANSITION_EXIT_OFFSETS[0],
            ]
        );
        assert!(
            PAGE_TRANSITION_ENTER_OFFSETS
                .iter()
                .all(|offset| *offset > 0 && *offset < PAGE_TRANSITION_MAX_OFFSET)
        );
    }

    #[test]
    fn page_transition_state_is_bounded_exclusive_and_fail_closed() {
        let mut model = MobileModel::for_page(MobilePage::Settings);
        model.shade_open = true;
        model.drawer_open = true;
        model.back_reveal_px = BACK_GESTURE_REVEAL_MAX;
        model.back_origin_y = 800;
        model.unlock_reveal_px = UNLOCK_REVEAL_MAX;
        model.pressed_target = Some(MobilePressedTarget::SystemHome);

        assert!(model.apply(MobileAction::SetPageTransitionOffset(u16::MAX)));
        assert_eq!(model.page_transition_offset_px, PAGE_TRANSITION_MAX_OFFSET);
        assert_eq!(
            model.effective_page_transition_offset_px(),
            PAGE_TRANSITION_MAX_OFFSET
        );
        assert!(!model.shade_open);
        assert!(!model.drawer_open);
        assert_eq!(model.back_reveal_px, 0);
        assert_eq!(model.back_origin_y, 0);
        assert_eq!(model.unlock_reveal_px, 0);
        assert_eq!(model.pressed_target, None);
        assert_eq!(hit_test(model, 360, 1_570), None);
        assert_eq!(hit_test(model, 600, 474), None);

        assert!(model.apply(MobileAction::SetPageTransitionOffset(0)));
        assert_eq!(model.effective_page_transition_offset_px(), 0);
        assert_eq!(
            hit_test(model, 360, 1_570),
            Some(MobilePressedTarget::SystemHome)
        );

        let mut home = MobileModel::default();
        assert!(!home.apply(MobileAction::SetPageTransitionOffset(128)));
        assert_eq!(home.page_transition_offset_px, 0);
        let mut lock = MobileModel::locked();
        assert!(!lock.apply(MobileAction::SetPageTransitionOffset(128)));
        assert_eq!(lock.page_transition_offset_px, 0);
    }

    #[test]
    fn page_transition_frames_are_distinct_and_converge_exactly() {
        for page in [
            MobilePage::Phone,
            MobilePage::Messages,
            MobilePage::Calculator,
            MobilePage::AndroidDemo,
            MobilePage::Settings,
            MobilePage::Apps,
            MobilePage::Display,
            MobilePage::Accessibility,
        ] {
            let stable_model = MobileModel::for_page(page);
            let stable = render_model(stable_model);
            let mut model = stable_model;
            let mut prior_digest = None;
            for offset in PAGE_TRANSITION_ENTER_OFFSETS {
                assert!(model.apply(MobileAction::SetPageTransitionOffset(offset)));
                let frame = render_model(model);
                let frame_digest = digest(&frame);
                assert_ne!(frame_digest, digest(&stable), "{page:?}/{offset}");
                if let Some(prior_digest) = prior_digest {
                    assert_ne!(frame_digest, prior_digest, "{page:?}/{offset}");
                }
                prior_digest = Some(frame_digest);

                // The transition layer begins below the status band and is
                // clipped above gesture navigation; both system regions stay
                // bit-identical to the stable target.
                assert_eq!(&frame[..64 * WIDTH], &stable[..64 * WIDTH]);
                assert_eq!(
                    &frame[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..],
                    &stable[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..]
                );
            }
            assert!(model.apply(MobileAction::SetPageTransitionOffset(0)));
            assert_eq!(render_model(model), stable, "{page:?}");
        }
    }

    #[test]
    fn transition_row_renderer_matches_complete_frame() {
        let mut model = MobileModel::for_page(MobilePage::Settings);
        assert!(model.apply(MobileAction::SetPageTransitionOffset(
            PAGE_TRANSITION_ENTER_OFFSETS[1],
        )));
        let complete = render_model(model);
        let mut rows = vec![0_u32; PIXEL_COUNT];
        let mut first_row = 0;
        while first_row < HEIGHT {
            let row_count = (HEIGHT - first_row).min(23);
            let start = first_row * WIDTH;
            let end = start + row_count * WIDTH;
            render_rows(&mut rows[start..end], first_row, model).unwrap();
            first_row += row_count;
        }
        assert_eq!(rows, complete);
    }

    #[test]
    fn system_ui_modes_start_from_the_surface_that_owns_them() {
        assert_eq!(MobileModel::default().system_ui_mode, UiSystemUiMode::Home);
        assert_eq!(MobileModel::locked().system_ui_mode, UiSystemUiMode::Locked);
        for page in [
            MobilePage::Home,
            MobilePage::Calculator,
            MobilePage::AndroidDemo,
        ] {
            assert_eq!(
                MobileModel::for_page(page).system_ui_mode,
                UiSystemUiMode::Home,
                "{page:?} is Launcher-owned"
            );
        }
        for page in [
            MobilePage::Phone,
            MobilePage::Messages,
            MobilePage::Settings,
            MobilePage::Apps,
            MobilePage::About,
            MobilePage::Display,
            MobilePage::Accessibility,
        ] {
            assert_eq!(
                MobileModel::for_page(page).system_ui_mode,
                UiSystemUiMode::Foreground,
                "{page:?} is an App foreground surface"
            );
        }
    }

    #[test]
    fn system_ui_snapshot_application_clears_local_overlays_without_granting_authority() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        model.shade_open = true;
        model.shade_reveal_px = 320;
        model.drawer_open = true;
        model.drawer_reveal_px = 416;
        model.pressed_target = Some(MobilePressedTarget::Back);

        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        assert!(model.overview_open());
        assert_eq!(model.page, MobilePage::Phone);
        assert_eq!(
            model.system_ui_recent,
            Some(UiRecentIdentity::Shell(ShellAppId::Phone))
        );
        assert_eq!(model.effective_overview_reveal_px(), OVERVIEW_RENDER_MAX_PX);
        assert!(!model.shade_open);
        assert!(!model.drawer_open);
        assert_eq!(model.pressed_target, None);

        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Foreground,
            Some(ShellAppId::Phone),
            true,
            u16::MAX,
            3,
        ));
        assert_eq!(model.system_nav_reveal_px, OVERVIEW_HOME_COMMIT_PX);
        assert_eq!(model.effective_overview_reveal_px(), OVERVIEW_RENDER_MAX_PX);
        assert!(!model.overview_open());

        assert!(model.apply_system_ui_state(UiSystemUiMode::Home, None, false, 0, 4));
        assert_eq!(model.page, MobilePage::Home);
        assert_eq!(model.system_ui_recent, None);
        assert_eq!(model.system_ui_revision, 4);

        assert!(model.apply_system_ui_state(UiSystemUiMode::Locked, None, false, 0, 5));
        assert_eq!(model.page, MobilePage::Lock);
        assert_eq!(model.system_ui_mode, UiSystemUiMode::Locked);
    }

    #[test]
    fn overview_hit_testing_is_half_open_and_empty_card_is_inert() {
        let mut recent = MobileModel::for_page(MobilePage::Phone);
        assert!(recent.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        assert_eq!(
            hit_test(recent, OVERVIEW_RECENT_TARGET.x, OVERVIEW_RECENT_TARGET.y),
            Some(MobilePressedTarget::OverviewRecent)
        );
        assert_eq!(
            hit_test(
                recent,
                OVERVIEW_RECENT_TARGET.x + OVERVIEW_RECENT_TARGET.width - 1,
                OVERVIEW_RECENT_TARGET.y + OVERVIEW_RECENT_TARGET.height - 1,
            ),
            Some(MobilePressedTarget::OverviewRecent)
        );
        assert_eq!(
            hit_test(
                recent,
                OVERVIEW_RECENT_TARGET.x + OVERVIEW_RECENT_TARGET.width,
                OVERVIEW_RECENT_TARGET.y,
            ),
            Some(MobilePressedTarget::OverviewBackground)
        );
        assert_eq!(
            hit_test(
                recent,
                OVERVIEW_RECENT_TARGET.x,
                OVERVIEW_RECENT_TARGET.y + OVERVIEW_RECENT_TARGET.height,
            ),
            Some(MobilePressedTarget::OverviewBackground),
        );

        let mut empty = recent;
        assert!(empty.apply_system_ui_state(UiSystemUiMode::Overview, None, false, 0, 3));
        assert_eq!(
            hit_test(empty, 360, 800),
            None,
            "the empty identity card never invents an app action"
        );
        assert_eq!(
            hit_test(empty, 680, 800),
            Some(MobilePressedTarget::OverviewBackground)
        );
    }

    #[test]
    fn overview_card_release_after_leaving_the_entire_card_cancels() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        let stable = render_model(model);
        let mut touch = TouchController::new();
        assert_eq!(
            touch.observe(model, 360, 800, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::OverviewRecent
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::OverviewRecent,
        ))));
        assert_eq!(
            touch.observe(model, 680, 800, false),
            Some(MobileAction::SetPressed(None)),
            "releasing outside the half-open card cannot activate its recent app"
        );
        assert!(model.apply(MobileAction::SetPressed(None)));
        assert_eq!(render_model(model), stable);
    }

    #[test]
    fn overview_recent_swipe_follows_finger_rebounds_and_commits_only_after_threshold() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        let stable = render_model(model);

        let mut touch = TouchController::new();
        assert_eq!(
            touch.observe(model, 360, 900, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::OverviewRecent
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::OverviewRecent,
        ))));
        assert_eq!(
            touch.observe(model, 360, 860, true),
            Some(MobileAction::SetOverviewRecentOffset(40))
        );
        assert!(model.apply(MobileAction::SetOverviewRecentOffset(40)));
        assert_eq!(model.effective_overview_recent_offset_px(), 40);
        assert_eq!(model.pressed_target, None);
        assert_ne!(render_model(model), stable);

        assert_eq!(
            touch.observe(
                model,
                360,
                900 - OVERVIEW_RECENT_DISMISS_THRESHOLD_PX + 8,
                false,
            ),
            Some(MobileAction::SetOverviewRecentOffset(0)),
            "a sub-threshold release must rebound instead of removing the card"
        );
        assert!(model.apply(MobileAction::SetOverviewRecentOffset(0)));
        assert_eq!(render_model(model), stable);

        let mut touch = TouchController::new();
        assert!(touch.observe(model, 360, 900, true).is_some());
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::OverviewRecent,
        ))));
        let committed_y = 900 - OVERVIEW_RECENT_DISMISS_THRESHOLD_PX - 16;
        assert_eq!(
            touch.observe(model, 360, committed_y, true),
            Some(MobileAction::SetOverviewRecentOffset(
                OVERVIEW_RECENT_DISMISS_THRESHOLD_PX + 16
            ))
        );
        assert!(model.apply(MobileAction::SetOverviewRecentOffset(
            OVERVIEW_RECENT_DISMISS_THRESHOLD_PX + 16,
        )));
        assert_eq!(
            touch.observe(model, 360, committed_y, false),
            Some(MobileAction::DismissRecentApp)
        );
        assert!(model.apply(MobileAction::DismissRecentApp));
        assert_eq!(
            model.system_ui_recent,
            Some(UiRecentIdentity::Shell(ShellAppId::Phone)),
            "the UI action alone must not mutate SurfaceServer-owned identity"
        );
        assert!(model.overview_open());
        assert_eq!(model.overview_recent_offset_px, 0);

        assert!(model.apply_system_ui_state(UiSystemUiMode::Overview, None, false, 0, 3));
        assert_eq!(model.system_ui_recent, None);
        assert_eq!(model.effective_overview_recent_offset_px(), 0);
    }

    #[test]
    fn overview_swipe_can_remove_stale_compatible_identity_but_never_activates_it() {
        let identity = UiCompatibleActivityIdentity::new(17, 5).unwrap();
        let recent = UiRecentIdentity::CompatibleAndroid(identity);
        let mut model = MobileModel::for_page(MobilePage::AndroidDemo);
        assert!(model.apply_system_ui_state_recent(
            UiSystemUiMode::Overview,
            Some(recent),
            false,
            0,
            2,
        ));
        assert_eq!(hit_test(model, 360, 900), None);

        let mut touch = TouchController::new();
        assert_eq!(touch.observe(model, 360, 900, true), None);
        assert_eq!(
            touch.observe(model, 360, 640, true),
            Some(MobileAction::SetOverviewRecentOffset(260))
        );
        assert!(model.apply(MobileAction::SetOverviewRecentOffset(260)));
        assert_eq!(
            touch.observe(model, 360, 640, false),
            Some(MobileAction::DismissRecentApp)
        );
    }

    #[test]
    fn overview_card_offset_is_bounded_and_never_changes_system_chrome() {
        let mut model = MobileModel::for_page(MobilePage::Messages);
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Messages),
            false,
            0,
            2,
        ));
        let stable = render_model(model);
        assert!(model.apply(MobileAction::SetOverviewRecentOffset(u16::MAX)));
        assert_eq!(
            model.effective_overview_recent_offset_px(),
            OVERVIEW_RECENT_MAX_OFFSET_PX
        );
        let shifted = render_model(model);
        assert_pixel_differences_are_bounded(&stable, &shifted, &[(48, 104, 672, 1_184)]);
        assert_eq!(&stable[..64 * WIDTH], &shifted[..64 * WIDTH]);
        assert_eq!(
            &stable[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..],
            &shifted[usize::from(SYSTEM_NAV_TOP_PX) * WIDTH..]
        );
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Messages),
            false,
            0,
            3,
        ));
        assert_eq!(model.overview_recent_offset_px, 0);
    }

    #[test]
    fn overview_background_requests_close_without_mutating_server_owned_mode_locally() {
        let mut model = MobileModel::default();
        assert!(model.apply_system_ui_state(UiSystemUiMode::Overview, None, false, 0, 2));
        let mut touch = TouchController::new();
        assert_eq!(
            touch.observe(model, 680, 1_300, true),
            Some(MobileAction::SetPressed(Some(
                MobilePressedTarget::OverviewBackground
            )))
        );
        assert!(model.apply(MobileAction::SetPressed(Some(
            MobilePressedTarget::OverviewBackground,
        ))));
        assert_eq!(
            touch.observe(model, 680, 1_300, false),
            Some(MobileAction::CloseOverview)
        );
        assert!(model.apply(MobileAction::CloseOverview));
        assert!(model.overview_open(), "only SurfaceServer may settle Home");
        assert_eq!(model.pressed_target, None);
    }

    #[test]
    fn overview_identity_card_contains_all_recent_app_variation() {
        let mut phone = MobileModel::for_page(MobilePage::Phone);
        assert!(phone.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        let mut messages = phone;
        assert!(messages.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Messages),
            false,
            0,
            3,
        ));
        assert_pixel_differences_are_bounded(
            &render_model(phone),
            &render_model(messages),
            &[(48, 424, 672, 1_184)],
        );

        let original = render_model(phone);
        let mut altered_phone = phone;
        altered_phone.phone_digits = [9; PHONE_DIGIT_CAPACITY];
        altered_phone.phone_digit_count = PHONE_DIGIT_CAPACITY as u8;
        assert_eq!(
            render_model(altered_phone),
            original,
            "Overview must not copy or inspect the Phone surface"
        );
    }

    #[test]
    fn overview_empty_state_changes_only_the_identity_card() {
        let mut identity = MobileModel::for_page(MobilePage::Phone);
        assert!(identity.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            2,
        ));
        let mut empty = identity;
        assert!(empty.apply_system_ui_state(UiSystemUiMode::Overview, None, false, 0, 3));
        assert_pixel_differences_are_bounded(
            &render_model(identity),
            &render_model(empty),
            &[(48, 424, 672, 1_184), (0, 1_220, 720, 1_300)],
        );
    }

    #[test]
    fn overview_half_reveal_has_exact_sheet_top_and_chunk_independent_rows() {
        let mut model = MobileModel::for_page(MobilePage::Phone);
        assert!(model.apply_system_ui_state(
            UiSystemUiMode::Foreground,
            Some(ShellAppId::Phone),
            true,
            160,
            2,
        ));
        assert_eq!(model.effective_overview_reveal_px(), 160);
        let sheet_top = usize::from(SYSTEM_NAV_TOP_PX)
            - (usize::from(SYSTEM_NAV_TOP_PX - 240) * 160 / usize::from(OVERVIEW_RENDER_MAX_PX));
        assert_eq!(sheet_top, 894);
        let complete = render_model(model);
        assert_eq!(
            complete[sheet_top * WIDTH + WIDTH / 2],
            model.theme_tokens().panel
        );

        let mut rows = vec![0_u32; PIXEL_COUNT];
        for first_row in (0..HEIGHT).step_by(29).rev() {
            let row_count = (HEIGHT - first_row).min(29);
            render_rows(
                &mut rows[first_row * WIDTH..(first_row + row_count) * WIDTH],
                first_row,
                model,
            )
            .unwrap();
        }
        assert_eq!(rows, complete);
    }

    #[test]
    fn overview_preserves_status_and_rounded_display_mask_across_reveal_states() {
        let base = MobileModel::for_page(MobilePage::Phone);
        let base_pixels = render_model(base);
        let mut transient = base;
        assert!(transient.apply_system_ui_state(
            UiSystemUiMode::Foreground,
            Some(ShellAppId::Phone),
            true,
            160,
            2,
        ));
        let mut stable = transient;
        assert!(stable.apply_system_ui_state(
            UiSystemUiMode::Overview,
            Some(ShellAppId::Phone),
            false,
            0,
            3,
        ));

        for frame in [render_model(transient), render_model(stable)] {
            assert_eq!(&frame[..64 * WIDTH], &base_pixels[..64 * WIDTH]);
            for (x, y) in [
                (0, 0),
                (WIDTH - 1, 0),
                (0, HEIGHT - 1),
                (WIDTH - 1, HEIGHT - 1),
            ] {
                assert_eq!(frame[y * WIDTH + x], COLOR_BLACK, "{x}/{y}");
            }
        }
    }

    #[cfg(feature = "mobile-system-chrome0")]
    fn split_chrome_state(model: MobileModel) -> MobileSystemChromeState {
        MobileSystemChromeState::new(
            model.time,
            model.dark_theme,
            model.alternate_accent,
            model.software_dimming,
            model.large_text,
            model.high_contrast,
            model.system_nav_pressed,
        )
    }

    #[cfg(feature = "mobile-system-chrome0")]
    fn render_split_frame(model: MobileModel, chrome: MobileSystemChromeState) -> Vec<u32> {
        let mut pixels = vec![0x00de_adbe; PIXEL_COUNT];
        render_system_chrome_rows(
            &mut pixels[..usize::from(MOBILE_CONTENT_VIEWPORT_Y) * WIDTH],
            0,
            chrome,
        )
        .unwrap();
        render_content_rows(
            &mut pixels[usize::from(MOBILE_CONTENT_VIEWPORT_Y) * WIDTH
                ..usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) * WIDTH],
            usize::from(MOBILE_CONTENT_VIEWPORT_Y),
            model,
        )
        .unwrap();
        render_system_chrome_rows(
            &mut pixels[usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) * WIDTH..],
            usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM),
            chrome,
        )
        .unwrap();
        pixels
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn content_surface_constants_and_split_rows_are_canonical() {
        assert_eq!(MOBILE_CONTENT_VIEWPORT_X, 0);
        assert_eq!(MOBILE_CONTENT_VIEWPORT_Y, 64);
        assert_eq!(MOBILE_CONTENT_VIEWPORT_WIDTH, WIDTH as u16);
        assert_eq!(MOBILE_CONTENT_VIEWPORT_HEIGHT, 1_448);
        assert_eq!(MOBILE_CONTENT_VIEWPORT_BOTTOM, 1_512);

        let mut model = MobileModel::for_page(MobilePage::Settings);
        model.time = MobileTimeSnapshot::from_unix_seconds(1_712_345_600);
        let legacy = render_model(model);
        let split = render_split_frame(model, split_chrome_state(model));
        assert_eq!(
            &split[usize::from(MOBILE_CONTENT_VIEWPORT_Y) * WIDTH
                ..usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) * WIDTH],
            &legacy[usize::from(MOBILE_CONTENT_VIEWPORT_Y) * WIDTH
                ..usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) * WIDTH],
        );
        assert!(split.iter().all(|pixel| *pixel != 0x00de_adbe));
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn trusted_chrome_and_client_content_have_disjoint_difference_bounds() {
        let mut phone = MobileModel::for_page(MobilePage::Phone);
        phone.time = MobileTimeSnapshot::from_unix_seconds(1_712_345_600);
        let mut settings = phone;
        settings.page = MobilePage::Settings;
        let chrome = split_chrome_state(phone);
        let phone_frame = render_split_frame(phone, chrome);
        let settings_frame = render_split_frame(settings, chrome);

        let differing_content: Vec<usize> = phone_frame
            .iter()
            .zip(settings_frame.iter())
            .enumerate()
            .filter_map(|(index, (left, right))| (left != right).then_some(index))
            .collect();
        assert!(!differing_content.is_empty());
        assert!(differing_content.iter().all(|index| {
            let row = index / WIDTH;
            (usize::from(MOBILE_CONTENT_VIEWPORT_Y)..usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM))
                .contains(&row)
        }));

        let pressed_chrome = MobileSystemChromeState::new(
            phone.time,
            phone.dark_theme,
            phone.alternate_accent,
            phone.software_dimming,
            phone.large_text,
            phone.high_contrast,
            true,
        );
        let pressed_frame = render_split_frame(phone, pressed_chrome);
        let differing_chrome: Vec<usize> = phone_frame
            .iter()
            .zip(pressed_frame.iter())
            .enumerate()
            .filter_map(|(index, (left, right))| (left != right).then_some(index))
            .collect();
        assert!(!differing_chrome.is_empty());
        assert!(
            differing_chrome
                .iter()
                .all(|index| index / WIDTH >= usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM))
        );
    }

    #[cfg(feature = "mobile-system-chrome0")]
    #[test]
    fn split_renderers_reject_cross_owner_rows_before_writing() {
        let model = MobileModel::default();
        let chrome = split_chrome_state(model);
        for (first_row, renderer_is_content) in [
            (usize::from(MOBILE_CONTENT_VIEWPORT_Y) - 1, true),
            (usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM), true),
            (usize::from(MOBILE_CONTENT_VIEWPORT_Y), false),
            (usize::from(MOBILE_CONTENT_VIEWPORT_BOTTOM) - 1, false),
        ] {
            let mut row = [0x0012_3456; WIDTH];
            let result = if renderer_is_content {
                render_content_rows(&mut row, first_row, model)
            } else {
                render_system_chrome_rows(&mut row, first_row, chrome)
            };
            assert_eq!(result, Err(RenderError::InvalidRowRange));
            assert!(row.iter().all(|pixel| *pixel == 0x0012_3456));
        }
    }

    #[test]
    fn invalid_ranges_are_rejected_without_writing() {
        let mut short = [0x00ab_cdef; 8];
        assert_eq!(
            render(&mut short, MobileModel::default()),
            Err(RenderError::WrongPixelCount)
        );
        assert_eq!(short, [0x00ab_cdef; 8]);

        let mut row = [0x0012_3456; WIDTH];
        assert_eq!(
            render_rows(&mut row, HEIGHT, MobileModel::default()),
            Err(RenderError::InvalidRowRange)
        );
        assert!(row.iter().all(|pixel| *pixel == 0x0012_3456));
    }
}
