//! Deterministic launcher state, hit testing, and tap gesture routing.

use crate::{
    compositor::Rect,
    framebuffer::{
        COLOR_CARD_CYAN, COLOR_CARD_GREEN, COLOR_CARD_PURPLE, COLOR_PHONE_HEADER,
        COLOR_PHONE_SCREEN, HEIGHT, WIDTH, boot_splash_pixel,
    },
};

pub const PHONE_SCREEN: Rect = Rect::new(56, 64, 208, 368);
pub const PHONE_TARGET: Rect = Rect::new(72, 132, 176, 72);
pub const MESSAGES_TARGET: Rect = Rect::new(72, 220, 176, 72);
pub const SETTINGS_TARGET: Rect = Rect::new(72, 308, 176, 72);
pub const HOME_TARGET: Rect = Rect::new(128, 448, 64, 24);

pub const COLOR_APP_PANEL: u32 = 0x000f_172a;
pub const COLOR_APP_ROW: u32 = 0x00e2_e8f0;
pub const COLOR_APP_ROW_MUTED: u32 = 0x0064_748b;
pub const COLOR_APP_TOGGLE: u32 = 0x0038_bdf8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppId {
    Phone,
    Messages,
    Settings,
}

impl AppId {
    pub const fn raw(self) -> u8 {
        match self {
            Self::Phone => 1,
            Self::Messages => 2,
            Self::Settings => 3,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Messages => "messages",
            Self::Settings => "settings",
        }
    }

    pub const fn accent(self) -> u32 {
        match self {
            Self::Phone => COLOR_CARD_CYAN,
            Self::Messages => COLOR_CARD_PURPLE,
            Self::Settings => COLOR_CARD_GREEN,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiView {
    #[default]
    Home,
    App(AppId),
}

impl UiView {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::App(app) => app.name(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTarget {
    App(AppId),
    Home,
}

impl UiTarget {
    pub const fn name(self) -> &'static str {
        match self {
            Self::App(app) => app.name(),
            Self::Home => "home",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiCommit {
    pub from: UiView,
    pub to: UiView,
    pub target: UiTarget,
    pub damage: Rect,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiSnapshot {
    pub view: UiView,
    pub armed: Option<UiTarget>,
    pub pointer_pressed: bool,
    pub reports: u64,
    pub taps: u64,
    pub transitions: u64,
    pub surface_generation: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiController {
    view: UiView,
    armed: Option<UiTarget>,
    pointer_pressed: bool,
    reports: u64,
    taps: u64,
    transitions: u64,
    surface_generation: u64,
}

impl UiController {
    pub const fn new() -> Self {
        Self {
            view: UiView::Home,
            armed: None,
            pointer_pressed: false,
            reports: 0,
            taps: 0,
            transitions: 0,
            surface_generation: 0,
        }
    }

    pub fn observe(&mut self, x: usize, y: usize, pressed: bool) -> Option<UiCommit> {
        self.reports = self.reports.saturating_add(1);
        match (self.pointer_pressed, pressed) {
            (false, true) => {
                self.armed = hit_test(self.view, x, y);
                self.pointer_pressed = true;
                None
            }
            (true, false) => {
                self.pointer_pressed = false;
                let armed = self.armed.take();
                let released = hit_test(self.view, x, y);
                let target = armed.filter(|target| Some(*target) == released)?;
                self.taps = self.taps.saturating_add(1);
                let from = self.view;
                let to = match target {
                    UiTarget::App(app) => UiView::App(app),
                    UiTarget::Home => UiView::Home,
                };
                if to == from {
                    return None;
                }
                self.view = to;
                self.transitions = self.transitions.saturating_add(1);
                self.surface_generation = self.surface_generation.saturating_add(1);
                Some(UiCommit {
                    from,
                    to,
                    target,
                    damage: PHONE_SCREEN,
                    generation: self.surface_generation,
                })
            }
            (_, _) => {
                self.pointer_pressed = pressed;
                None
            }
        }
    }

    pub const fn snapshot(&self) -> UiSnapshot {
        UiSnapshot {
            view: self.view,
            armed: self.armed,
            pointer_pressed: self.pointer_pressed,
            reports: self.reports,
            taps: self.taps,
            transitions: self.transitions,
            surface_generation: self.surface_generation,
        }
    }
}

pub fn hit_test(view: UiView, x: usize, y: usize) -> Option<UiTarget> {
    match view {
        UiView::Home if PHONE_TARGET.contains(x, y) => Some(UiTarget::App(AppId::Phone)),
        UiView::Home if MESSAGES_TARGET.contains(x, y) => Some(UiTarget::App(AppId::Messages)),
        UiView::Home if SETTINGS_TARGET.contains(x, y) => Some(UiTarget::App(AppId::Settings)),
        UiView::App(_) if HOME_TARGET.contains(x, y) => Some(UiTarget::Home),
        _ => None,
    }
}

/// Returns one opaque XRGB8888 scene pixel for the committed UI state.
pub fn scene_pixel(view: UiView, x: usize, y: usize) -> u32 {
    if x >= WIDTH || y >= HEIGHT {
        return 0;
    }
    let UiView::App(app) = view else {
        return boot_splash_pixel(x, y);
    };
    if !PHONE_SCREEN.contains(x, y) {
        return boot_splash_pixel(x, y);
    }
    if y < 112 {
        return if Rect::new(72, 82, 48, 12).contains(x, y) {
            COLOR_PHONE_HEADER
        } else {
            app.accent()
        };
    }

    let mut color = COLOR_PHONE_SCREEN;
    if Rect::new(72, 128, 176, 248).contains(x, y) {
        color = COLOR_APP_PANEL;
    }
    if Rect::new(128, 144, 64, 64).contains(x, y) {
        color = app.accent();
    }
    if Rect::new(88, 236, 112, 12).contains(x, y)
        || Rect::new(88, 276, 128, 12).contains(x, y)
        || Rect::new(88, 316, 96, 12).contains(x, y)
    {
        color = COLOR_APP_ROW;
    }
    if Rect::new(88, 252, 144, 6).contains(x, y)
        || Rect::new(88, 292, 112, 6).contains(x, y)
        || Rect::new(88, 332, 136, 6).contains(x, y)
    {
        color = COLOR_APP_ROW_MUTED;
    }
    if Rect::new(212, 312, 24, 20).contains(x, y) {
        color = COLOR_APP_TOGGLE;
    }
    color
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "mobile-ui-runtime"))]
    use crate::framebuffer::{PIXEL_COUNT, pixel_digest};

    #[test]
    fn home_targets_have_exact_half_open_boundaries() {
        assert_eq!(
            hit_test(UiView::Home, 72, 132),
            Some(UiTarget::App(AppId::Phone))
        );
        assert_eq!(
            hit_test(UiView::Home, 247, 379),
            Some(UiTarget::App(AppId::Settings))
        );
        assert_eq!(hit_test(UiView::Home, 248, 379), None);
        assert_eq!(hit_test(UiView::Home, 160, 432), None);
        assert_eq!(
            hit_test(UiView::App(AppId::Settings), 128, 448),
            Some(UiTarget::Home)
        );
    }

    #[test]
    fn press_and_release_on_one_card_commits_one_transition() {
        let mut controller = UiController::new();
        assert_eq!(controller.observe(160, 342, false), None);
        assert_eq!(controller.observe(160, 342, true), None);
        let commit = controller
            .observe(160, 342, false)
            .expect("settings activation");
        assert_eq!(commit.from, UiView::Home);
        assert_eq!(commit.to, UiView::App(AppId::Settings));
        assert_eq!(commit.target, UiTarget::App(AppId::Settings));
        assert_eq!(commit.damage, PHONE_SCREEN);
        assert_eq!(commit.generation, 1);
        assert_eq!(
            controller.snapshot(),
            UiSnapshot {
                view: UiView::App(AppId::Settings),
                armed: None,
                pointer_pressed: false,
                reports: 3,
                taps: 1,
                transitions: 1,
                surface_generation: 1,
            }
        );
    }

    #[test]
    fn release_on_a_different_target_cancels_the_tap() {
        let mut controller = UiController::new();
        controller.observe(160, 160, true);
        assert_eq!(controller.observe(160, 250, false), None);
        assert_eq!(controller.snapshot().view, UiView::Home);
        assert_eq!(controller.snapshot().taps, 0);
    }

    #[test]
    fn app_navigation_returns_to_the_launcher() {
        let mut controller = UiController::new();
        controller.observe(160, 342, true);
        controller.observe(160, 342, false).expect("open app");
        controller.observe(160, 460, true);
        let commit = controller.observe(160, 460, false).expect("go home");
        assert_eq!(commit.from, UiView::App(AppId::Settings));
        assert_eq!(commit.to, UiView::Home);
        assert_eq!(commit.target, UiTarget::Home);
        assert_eq!(commit.generation, 2);
    }

    #[cfg(not(feature = "mobile-ui-runtime"))]
    #[test]
    fn home_surface_is_the_m27_scene_byte_for_byte() {
        let pixels: std::vec::Vec<u32> = (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| scene_pixel(UiView::Home, x, y)))
            .collect();
        assert_eq!(pixels.len(), PIXEL_COUNT);
        assert_eq!(pixel_digest(&pixels), 0x6ef9_c2b7_d15f_de25);
    }

    #[cfg(not(feature = "mobile-ui-runtime"))]
    #[test]
    fn app_surface_changes_only_the_declared_damage() {
        let home: std::vec::Vec<u32> = (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| scene_pixel(UiView::Home, x, y)))
            .collect();
        let settings: std::vec::Vec<u32> = (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| scene_pixel(UiView::App(AppId::Settings), x, y)))
            .collect();
        let mut changed = 0;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = y * WIDTH + x;
                if home[index] != settings[index] {
                    changed += 1;
                    assert!(PHONE_SCREEN.contains(x, y));
                }
            }
        }
        assert!(changed > 50_000);
        assert_ne!(pixel_digest(&home), pixel_digest(&settings));
        assert_eq!(pixel_digest(&settings), 0xf79f_5bb3_5824_52a5);
        assert_eq!(scene_pixel(UiView::App(AppId::Settings), WIDTH, 0), 0);
        assert_eq!(scene_pixel(UiView::App(AppId::Settings), 0, HEIGHT), 0);
    }
}
