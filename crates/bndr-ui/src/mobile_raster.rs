//! Packed region rendering and retained-buffer bookkeeping for copied mobile
//! surfaces. Uses the same Canvas, layers and damage planner as mapped frames.

use super::*;

/// A rendering cache, not a source of focus or application authority.
/// A cancelled submission still leaves new pixels in the producer's backing;
/// only an accepted submission changes the displayed baseline.
pub struct MobileRasterCache {
    written: Option<MobileModel>,
    presented: Option<MobileModel>,
    presented_focus: u32,
}

impl MobileRasterCache {
    pub const fn new() -> Self {
        Self {
            written: None,
            presented: None,
            presented_focus: 0,
        }
    }

    pub fn raster_plan(&self, model: MobileModel) -> MobileDamagePlan {
        damage_plan(self.written, model)
    }

    pub fn present_plan(&self, model: MobileModel, focus: u32) -> MobileDamagePlan {
        let baseline = self
            .presented
            .filter(|_| focus != 0 && self.presented_focus == focus);
        damage_plan(baseline, model)
    }

    pub fn record_written(&mut self, model: MobileModel) {
        self.written = Some(model);
    }

    pub fn record_presented(&mut self, model: MobileModel, focus: u32) {
        self.presented = (focus != 0).then_some(model);
        self.presented_focus = focus;
    }
}

impl Default for MobileRasterCache {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_region(region: DamageRect) -> Result<(), RenderError> {
    DamageRegions::single(region)
        .map(|_| ())
        .map_err(|_| RenderError::InvalidDamageRect)
}

/// Intersects a surface plan with one producer-owned viewport. No intersection
/// means no write; a Full plan means the complete viewport, never its neighbour.
pub fn clip_damage_plan(
    plan: MobileDamagePlan,
    viewport: DamageRect,
) -> Result<MobileDamagePlan, RenderError> {
    validate_region(viewport)?;
    let regions = match plan {
        MobileDamagePlan::Full => {
            return Ok(MobileDamagePlan::Regions(
                DamageRegions::single(viewport).map_err(|_| RenderError::InvalidDamageRect)?,
            ));
        }
        MobileDamagePlan::Unchanged => return Ok(MobileDamagePlan::Unchanged),
        MobileDamagePlan::Regions(regions) => regions,
    };
    let mut clipped = [DamageRect::EMPTY; 2];
    let mut count = 0;
    for region in regions.rects() {
        let left = region.x.max(viewport.x);
        let top = region.y.max(viewport.y);
        let right = (region.x + region.width).min(viewport.x + viewport.width);
        let bottom = (region.y + region.height).min(viewport.y + viewport.height);
        if left < right && top < bottom {
            clipped[count] = DamageRect {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            };
            count += 1;
        }
    }
    if count == 0 {
        return Ok(MobileDamagePlan::Unchanged);
    }
    // The shared constructor restores ordering if clipping moved two regions
    // onto the same row; it also rechecks their non-overlap and bounds.
    Ok(MobileDamagePlan::Regions(
        DamageRegions::try_new(&clipped[..count]).map_err(|_| RenderError::InvalidDamageRect)?,
    ))
}

fn region_canvas(
    pixels: &mut [u32],
    region: DamageRect,
    alternate_accent: bool,
    large_text: bool,
    high_contrast: bool,
) -> Result<Canvas<'_>, RenderError> {
    validate_region(region)?;
    if pixels.len() != usize::from(region.width) * usize::from(region.height) {
        return Err(RenderError::WrongPixelCount);
    }
    Ok(Canvas {
        pixels,
        first_row: usize::from(region.y),
        first_column: usize::from(region.x),
        row_stride: usize::from(region.width),
        row_count: usize::from(region.height),
        alternate_accent,
        large_text,
        high_contrast,
        design_offset_y_px: 0,
        clip_left_x_px: i32::from(region.x),
        clip_right_x_px: i32::from(region.x + region.width),
        clip_top_y_px: i32::from(region.y),
        clip_bottom_y_px: i32::from(region.y + region.height),
    })
}

/// Renders only `region` into tightly packed rows (stride = region.width).
/// Coordinates remain physical scanout coordinates for every shared primitive.
pub fn render_region(
    pixels: &mut [u32],
    region: DamageRect,
    model: MobileModel,
) -> Result<(), RenderError> {
    let mut canvas = region_canvas(
        pixels,
        region,
        model.alternate_accent,
        model.large_text,
        model.high_contrast,
    )?;
    render_content_layers(&mut canvas, model);
    render_system_chrome(&mut canvas, model);
    render_screen_corner_mask(&mut canvas);
    apply_software_dimming(&mut canvas, model.software_dimming);
    Ok(())
}

/// A packed content region must remain wholly inside the client viewport.
#[cfg(feature = "mobile-system-chrome0")]
pub fn render_content_region(
    pixels: &mut [u32],
    region: DamageRect,
    model: MobileModel,
) -> Result<(), RenderError> {
    validate_region(region)?;
    if region.y < MOBILE_CONTENT_VIEWPORT_Y
        || region.y + region.height > MOBILE_CONTENT_VIEWPORT_BOTTOM
    {
        return Err(RenderError::InvalidRowRange);
    }
    let mut canvas = region_canvas(
        pixels,
        region,
        model.alternate_accent,
        model.large_text,
        model.high_contrast,
    )?;
    render_content_layers(&mut canvas, model);
    apply_software_dimming(&mut canvas, model.software_dimming);
    Ok(())
}

/// A packed chrome region cannot span or overlap application content.
#[cfg(feature = "mobile-system-chrome0")]
pub fn render_system_chrome_region(
    pixels: &mut [u32],
    region: DamageRect,
    state: MobileSystemChromeState,
) -> Result<(), RenderError> {
    validate_region(region)?;
    if region.y + region.height > MOBILE_CONTENT_VIEWPORT_Y
        && region.y < MOBILE_CONTENT_VIEWPORT_BOTTOM
    {
        return Err(RenderError::InvalidRowRange);
    }
    let mut canvas = region_canvas(
        pixels,
        region,
        state.alternate_accent,
        state.large_text,
        state.high_contrast,
    )?;
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

#[cfg(feature = "mobile-system-chrome0")]
pub fn system_chrome_damage_plan(
    previous: Option<MobileSystemChromeState>,
    current: MobileSystemChromeState,
) -> MobileDamagePlan {
    let Some(mut previous) = previous else {
        return MobileDamagePlan::Full;
    };
    let time_changed = previous.time.time_text() != current.time.time_text();
    let nav_changed = previous.nav_pressed != current.nav_pressed;
    previous.time = current.time;
    previous.nav_pressed = current.nav_pressed;
    if previous != current {
        return MobileDamagePlan::Full;
    }
    plan_damage_regions(&[
        time_changed.then_some(STATUS_TIME_DAMAGE),
        nav_changed.then_some(DamageRect {
            x: 0,
            y: MOBILE_CONTENT_VIEWPORT_BOTTOM,
            width: WIDTH as u16,
            height: HEIGHT as u16 - MOBILE_CONTENT_VIEWPORT_BOTTOM,
        }),
    ])
}

/// Reuses the actual scene layout for text and pressed-state deltas. Changes
/// to hierarchy, size, identity or callback binding require a complete frame.
#[cfg(feature = "androidbox-scene-rpc2")]
pub(super) fn installed_scene_damage_plan(
    previous: MobileModel,
    current: MobileModel,
) -> Option<MobileDamagePlan> {
    if !stable_page_is(previous, MobilePage::AndroidDemo)
        || !stable_page_is(current, MobilePage::AndroidDemo)
        || !previous.installed_android_foreground_content_ready()
        || !current.installed_android_foreground_content_ready()
    {
        return None;
    }
    let before = previous.android_installed_activity_scene;
    let after = current.android_installed_activity_scene;
    if !before.is_active() || !after.is_active() || before.nodes().len() != after.nodes().len() {
        return None;
    }
    let mut normalized = previous;
    normalized.android_installed_activity_scene = after;
    normalized.pressed_target = current.pressed_target;
    if normalized != current {
        return None;
    }
    let mut candidates = [None; super::super::android_scene::ANDROID_ACTIVITY_SCENE_MAX_NODES + 2];
    let unpressed = MobileModel {
        pressed_target: None,
        ..current
    };
    for (index, target) in [previous.pressed_target, current.pressed_target]
        .into_iter()
        .enumerate()
    {
        if let Some(target) = target {
            candidates[index] = Some(pressed_target_damage(unpressed, target)?);
        }
    }
    for (index, (old, new)) in before.nodes().iter().zip(after.nodes()).enumerate() {
        if !old.same_structure(*new) {
            return None;
        }
        if old.text() == new.text() {
            continue;
        }
        let rect = installed_android_scene_node_rect(after, index)?;
        if installed_android_scene_node_rect(before, index) != Some(rect) {
            return None;
        }
        #[cfg(feature = "androidbox-layout-row14")]
        let role = installed_android_scene_node_text_role(after, index);
        #[cfg(not(feature = "androidbox-layout-row14"))]
        let role = MobileTextRole::Body;
        let text_y =
            (rect.height - (i32::from(role.line_height_px()) + SCALE - 1) / SCALE).max(0) / 2;
        let text_height =
            i32::from(accessible_text_role(role, current.large_text).line_height_px());
        // An admitted tiny exact height can put text below the control. Until
        // that overflow has a separate measured bound, repaint the whole page.
        if text_y * SCALE + text_height > rect.height * SCALE {
            return None;
        }
        candidates[index + 2] = Some(DamageRect {
            x: u16::try_from(rect.x.checked_mul(SCALE)?).ok()?,
            y: u16::try_from(rect.y.checked_mul(SCALE)?).ok()?,
            width: u16::try_from(rect.width.checked_mul(SCALE)?).ok()?,
            height: u16::try_from(rect.height.checked_mul(SCALE)?).ok()?,
        });
    }
    Some(plan_damage_regions(&candidates))
}
