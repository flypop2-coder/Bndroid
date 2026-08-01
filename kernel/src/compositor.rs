//! Bounded software composition for the fixed M27 ramfb scanout.

use crate::framebuffer::{HEIGHT, PIXEL_COUNT, WIDTH, pixel_digest};

pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 22;
pub const COLOR_CURSOR: u32 = 0x00f8_fafc;
pub const COLOR_CURSOR_PRESSED: u32 = 0x00f9_7316;
pub const COLOR_CURSOR_OUTLINE: u32 = 0x0011_1827;
pub const COLOR_CURSOR_SHADOW: u32 = 0x0000_0000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains(self, x: usize, y: usize) -> bool {
        let right = self.x.saturating_add(self.width);
        let bottom = self.y.saturating_add(self.height);
        x >= self.x && x < right && y >= self.y && y < bottom
    }

    pub fn intersects(self, other: Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        let self_right = self.x.saturating_add(self.width);
        let self_bottom = self.y.saturating_add(self.height);
        let other_right = other.x.saturating_add(other.width);
        let other_bottom = other.y.saturating_add(other.height);
        self.x < other_right
            && other.x < self_right
            && self.y < other_bottom
            && other.y < self_bottom
    }

    pub fn clipped(self) -> Self {
        let x = self.x.min(WIDTH);
        let y = self.y.min(HEIGHT);
        let right = self.x.saturating_add(self.width).min(WIDTH);
        let bottom = self.y.saturating_add(self.height).min(HEIGHT);
        Self::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y))
    }

    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other.clipped();
        }
        if other.is_empty() {
            return self.clipped();
        }
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
        Self::new(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )
        .clipped()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CursorState {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub pressed: bool,
}

impl CursorState {
    pub const fn hidden() -> Self {
        Self {
            x: 0,
            y: 0,
            visible: false,
            pressed: false,
        }
    }

    pub const fn visible(x: usize, y: usize, pressed: bool) -> Self {
        Self {
            x,
            y,
            visible: true,
            pressed,
        }
    }

    pub fn bounds(self) -> Rect {
        if !self.visible {
            Rect::default()
        } else {
            Rect::new(self.x, self.y, CURSOR_WIDTH, CURSOR_HEIGHT).clipped()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositorError {
    WrongScenePixelCount,
    WrongScanoutPixelCount,
    EmptyDamage,
    DamageOutOfBounds,
}

impl CompositorError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongScenePixelCount => "compositor scene surface has the wrong pixel count",
            Self::WrongScanoutPixelCount => "compositor scanout surface has the wrong pixel count",
            Self::EmptyDamage => "compositor scene damage is empty",
            Self::DamageOutOfBounds => "compositor scene damage is outside the scanout",
        }
    }
}

/// A non-empty scene rectangle proven to be fully contained by the scanout.
///
/// Construction is the only fallible part of publishing an already-rastered
/// scene. Keeping the fields private prevents an untrusted damage rectangle
/// from reaching the infallible fixed-array commit path through clipping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedSceneDamage(Rect);

impl ValidatedSceneDamage {
    pub const fn rect(self) -> Rect {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompositionEvidence {
    pub dirty: Rect,
    pub restored_pixels: usize,
    pub blended_pixels: usize,
    pub scanout_digest: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SceneUpdateEvidence {
    pub damage: Rect,
    pub composition: Rect,
    pub written_pixels: usize,
    pub restored_pixels: usize,
    pub blended_pixels: usize,
    pub scene_digest: u64,
    pub scanout_digest: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScenePublishEvidence {
    pub damage: Rect,
    pub composition: Rect,
    pub restored_pixels: usize,
    pub blended_pixels: usize,
    pub scene_digest: u64,
    pub scanout_digest: u64,
}

/// Read-only proof that scanout is exactly the opaque scene plus one current
/// cursor layer.  This is used when a degraded Surface session is rebound:
/// the frozen scanout may legitimately differ from the scene while the
/// physical cursor remains visible, but no unrelated pixel may differ.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanoutCompositionEvidence {
    pub scene_digest: u64,
    pub scanout_digest: u64,
    pub cursor_pixels: usize,
    pub exact: bool,
}

pub fn compose_full(
    scene: &[u32],
    scanout: &mut [u32],
    cursor: CursorState,
) -> Result<CompositionEvidence, CompositorError> {
    validate_surfaces(scene, scanout)?;
    scanout.copy_from_slice(scene);
    let blended_pixels = draw_cursor(scanout, cursor);
    Ok(CompositionEvidence {
        dirty: Rect::new(0, 0, WIDTH, HEIGHT),
        restored_pixels: PIXEL_COUNT,
        blended_pixels,
        scanout_digest: pixel_digest(scanout),
    })
}

pub fn validate_scanout_composition(
    scene: &[u32],
    scanout: &[u32],
    cursor: CursorState,
) -> Result<ScanoutCompositionEvidence, CompositorError> {
    validate_surfaces(scene, scanout)?;
    let bounds = cursor.bounds();
    let mut cursor_pixels = 0_usize;
    let mut exact = true;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let index = y * WIDTH + x;
            let mut expected = scene[index];
            if cursor.visible && bounds.contains(x, y) {
                let local_x = x - cursor.x;
                let local_y = y - cursor.y;
                if let Some((color, alpha)) = cursor_pixel(local_x, local_y, cursor.pressed) {
                    expected = blend_xrgb8888(expected, color, alpha);
                    cursor_pixels = cursor_pixels
                        .checked_add(1)
                        .ok_or(CompositorError::DamageOutOfBounds)?;
                }
            }
            exact &= scanout[index] == expected;
        }
    }
    Ok(ScanoutCompositionEvidence {
        scene_digest: pixel_digest(scene),
        scanout_digest: pixel_digest(scanout),
        cursor_pixels,
        exact,
    })
}

pub fn update_cursor(
    scene: &[u32],
    scanout: &mut [u32],
    previous: CursorState,
    next: CursorState,
) -> Result<CompositionEvidence, CompositorError> {
    validate_surfaces(scene, scanout)?;
    let dirty = previous.bounds().union(next.bounds());
    let restored_pixels = restore_scene_rect(scene, scanout, dirty);
    let blended_pixels = draw_cursor(scanout, next);
    Ok(CompositionEvidence {
        dirty,
        restored_pixels,
        blended_pixels,
        scanout_digest: pixel_digest(scanout),
    })
}

/// Commits one opaque scene damage rectangle and preserves the cursor layer.
///
/// Pixels outside `damage` are never rewritten. If the damage intersects the
/// cursor, its complete clipped bounds are restored from the updated scene and
/// alpha-composed exactly once; otherwise the existing cursor pixels remain
/// untouched.
pub fn update_scene<F>(
    scene: &mut [u32],
    scanout: &mut [u32],
    cursor: CursorState,
    damage: Rect,
    mut pixel: F,
) -> Result<SceneUpdateEvidence, CompositorError>
where
    F: FnMut(usize, usize) -> u32,
{
    validate_surfaces(scene, scanout)?;
    let damage = damage.clipped();
    let validated = validate_scene_damage(damage)?;
    let scene: &mut [u32; PIXEL_COUNT] = scene
        .try_into()
        .map_err(|_| CompositorError::WrongScenePixelCount)?;
    let scanout: &mut [u32; PIXEL_COUNT] = scanout
        .try_into()
        .map_err(|_| CompositorError::WrongScanoutPixelCount)?;
    for y in damage.y..damage.y + damage.height {
        for x in damage.x..damage.x + damage.width {
            scene[y * WIDTH + x] = pixel(x, y) & 0x00ff_ffff;
        }
    }
    let published = publish_scene_damage(scene, scanout, cursor, validated);
    Ok(SceneUpdateEvidence {
        damage,
        composition: published.composition,
        written_pixels: damage.width * damage.height,
        restored_pixels: published.restored_pixels,
        blended_pixels: published.blended_pixels,
        scene_digest: published.scene_digest,
        scanout_digest: published.scanout_digest,
    })
}

/// Strictly validates a scene damage rectangle without clipping it.
///
/// Callers handling untrusted coordinates must run this before mutating the
/// scene. A successful token can then be passed to [`publish_scene_damage`],
/// whose fixed-array signature and private token make publication infallible.
pub fn validate_scene_damage(damage: Rect) -> Result<ValidatedSceneDamage, CompositorError> {
    if damage.is_empty() {
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
    if right > WIDTH || bottom > HEIGHT {
        return Err(CompositorError::DamageOutOfBounds);
    }
    Ok(ValidatedSceneDamage(damage))
}

/// Publishes damage from an already-updated opaque scene and preserves the
/// cursor layer.
///
/// All fallible geometry and storage validation is represented by the input
/// types. If damage intersects the cursor, the complete union is restored and
/// the cursor is alpha-composed exactly once. Otherwise existing cursor bytes
/// are left untouched.
pub fn publish_scene_damage(
    scene: &[u32; PIXEL_COUNT],
    scanout: &mut [u32; PIXEL_COUNT],
    cursor: CursorState,
    damage: ValidatedSceneDamage,
) -> ScenePublishEvidence {
    let damage = damage.rect();
    let cursor_bounds = cursor.bounds();
    let cursor_affected = damage.intersects(cursor_bounds);
    let composition = if cursor_affected {
        damage.union(cursor_bounds)
    } else {
        damage
    };
    let restored_pixels = restore_scene_rect(scene, scanout, composition);
    let blended_pixels = if cursor_affected {
        draw_cursor(scanout, cursor)
    } else {
        0
    };
    ScenePublishEvidence {
        damage,
        composition,
        restored_pixels,
        blended_pixels,
        scene_digest: pixel_digest(scene),
        scanout_digest: pixel_digest(scanout),
    }
}

fn validate_surfaces(scene: &[u32], scanout: &[u32]) -> Result<(), CompositorError> {
    if scene.len() != PIXEL_COUNT {
        return Err(CompositorError::WrongScenePixelCount);
    }
    if scanout.len() != PIXEL_COUNT {
        return Err(CompositorError::WrongScanoutPixelCount);
    }
    Ok(())
}

fn restore_scene_rect(scene: &[u32], scanout: &mut [u32], rect: Rect) -> usize {
    let rect = rect.clipped();
    for y in rect.y..rect.y + rect.height {
        let start = y * WIDTH + rect.x;
        let end = start + rect.width;
        scanout[start..end].copy_from_slice(&scene[start..end]);
    }
    rect.width * rect.height
}

fn draw_cursor(scanout: &mut [u32], cursor: CursorState) -> usize {
    if !cursor.visible {
        return 0;
    }
    let bounds = cursor.bounds();
    let mut blended = 0;
    for y in bounds.y..bounds.y + bounds.height {
        for x in bounds.x..bounds.x + bounds.width {
            let local_x = x - cursor.x;
            let local_y = y - cursor.y;
            let Some((color, alpha)) = cursor_pixel(local_x, local_y, cursor.pressed) else {
                continue;
            };
            let index = y * WIDTH + x;
            scanout[index] = blend_xrgb8888(scanout[index], color, alpha);
            blended += 1;
        }
    }
    blended
}

fn cursor_pixel(x: usize, y: usize, pressed: bool) -> Option<(u32, u8)> {
    let foreground = arrow_class(x, y);
    if let Some(outline) = foreground {
        let color = if outline {
            COLOR_CURSOR_OUTLINE
        } else if pressed {
            COLOR_CURSOR_PRESSED
        } else {
            COLOR_CURSOR
        };
        return Some((color, 255));
    }
    if x >= 2 && y >= 2 && arrow_class(x - 2, y - 2).is_some() {
        return Some((COLOR_CURSOR_SHADOW, 96));
    }
    None
}

fn arrow_class(x: usize, y: usize) -> Option<bool> {
    if y < 14 && x <= y / 2 {
        let outline = x == 0 || x == y / 2 || y == 13;
        return Some(outline);
    }
    if (10..20).contains(&y) && (4..8).contains(&x) {
        let outline = x == 4 || x == 7 || y == 10 || y == 19;
        return Some(outline);
    }
    None
}

pub const fn blend_xrgb8888(background: u32, foreground: u32, alpha: u8) -> u32 {
    if alpha == 0 {
        return background & 0x00ff_ffff;
    }
    if alpha == 255 {
        return foreground & 0x00ff_ffff;
    }
    let inverse = 255_u32 - alpha as u32;
    let alpha = alpha as u32;
    let blue = blend_channel(background, foreground, 0, alpha, inverse);
    let green = blend_channel(background, foreground, 8, alpha, inverse);
    let red = blend_channel(background, foreground, 16, alpha, inverse);
    blue | (green << 8) | (red << 16)
}

const fn blend_channel(
    background: u32,
    foreground: u32,
    shift: u32,
    alpha: u32,
    inverse: u32,
) -> u32 {
    let background = (background >> shift) & 0xff;
    let foreground = (foreground >> shift) & 0xff;
    (foreground * alpha + background * inverse + 127) / 255
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::render_boot_splash;
    use std::vec;

    #[test]
    fn rectangles_clip_and_union_without_overflow() {
        assert_eq!(
            Rect::new(WIDTH - 10, HEIGHT - 10, 20, 20).clipped(),
            Rect::new(WIDTH - 10, HEIGHT - 10, 10, 10)
        );
        assert_eq!(
            Rect::new(10, 20, 5, 6).union(Rect::new(30, 40, 7, 8)),
            Rect::new(10, 20, 27, 28)
        );
        assert_eq!(
            Rect::new(usize::MAX, usize::MAX, 8, 8).clipped(),
            Rect::new(WIDTH, HEIGHT, 0, 0)
        );
        assert!(Rect::new(10, 20, 5, 6).contains(10, 20));
        assert!(Rect::new(10, 20, 5, 6).contains(14, 25));
        assert!(!Rect::new(10, 20, 5, 6).contains(15, 25));
        assert!(!Rect::new(usize::MAX, usize::MAX, 8, 8).contains(0, 0));
        assert!(Rect::new(10, 20, 5, 6).intersects(Rect::new(14, 25, 7, 8)));
        assert!(!Rect::new(10, 20, 5, 6).intersects(Rect::new(15, 25, 7, 8)));
        assert!(!Rect::new(10, 20, 5, 0).intersects(Rect::new(10, 20, 5, 6)));
    }

    #[test]
    fn hidden_full_composition_preserves_the_scene_exactly() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let mut scanout = vec![0_u32; PIXEL_COUNT];
        let evidence =
            compose_full(&scene, &mut scanout, CursorState::hidden()).expect("scanout geometry");
        assert_eq!(scanout, scene);
        assert_eq!(evidence.restored_pixels, PIXEL_COUNT);
        assert_eq!(evidence.blended_pixels, 0);
        assert_eq!(evidence.scanout_digest, pixel_digest(&scene));
    }

    #[test]
    fn frozen_scanout_validation_proves_the_exact_cursor_layer() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");

        let hidden = validate_scanout_composition(&scene, &scene, CursorState::hidden())
            .expect("hidden composition geometry");
        assert!(hidden.exact);
        assert_eq!(hidden.cursor_pixels, 0);
        assert_eq!(hidden.scene_digest, hidden.scanout_digest);

        let cursor = CursorState::visible(136, 184, false);
        let mut scanout = vec![0_u32; PIXEL_COUNT];
        let composed = compose_full(&scene, &mut scanout, cursor).expect("visible composition");
        let visible = validate_scanout_composition(&scene, &scanout, cursor)
            .expect("visible composition geometry");
        assert!(visible.exact);
        assert!(visible.cursor_pixels > 0);
        assert_eq!(visible.scanout_digest, composed.scanout_digest);
        assert_ne!(visible.scene_digest, visible.scanout_digest);

        scanout[0] ^= 1;
        let corrupt = validate_scanout_composition(&scene, &scanout, cursor)
            .expect("corrupt composition geometry");
        assert!(!corrupt.exact);
    }

    #[test]
    fn cursor_move_restores_old_pixels_and_changes_only_the_dirty_union() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let mut scanout = scene.clone();
        let first = CursorState::visible(80, 140, false);
        update_cursor(&scene, &mut scanout, CursorState::hidden(), first).expect("first cursor");
        let before = scanout.clone();
        let second = CursorState::visible(180, 260, true);
        let evidence = update_cursor(&scene, &mut scanout, first, second).expect("moved cursor");
        assert_eq!(evidence.dirty, Rect::new(80, 140, 112, 142));
        assert_eq!(evidence.restored_pixels, 112 * 142);
        assert!(evidence.blended_pixels > 0);
        assert_ne!(scanout, before);
        assert_eq!(scanout[140 * WIDTH + 80], scene[140 * WIDTH + 80]);
        assert_eq!(scanout[0], scene[0]);
    }

    #[test]
    fn cursor_clips_at_the_scanout_edge_and_pressed_color_is_distinct() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let mut edge_scanout = scene.clone();
        let cursor = CursorState::visible(WIDTH - 3, HEIGHT - 4, false);
        let released_evidence =
            update_cursor(&scene, &mut edge_scanout, CursorState::hidden(), cursor)
                .expect("edge cursor");
        assert_eq!(
            released_evidence.dirty,
            Rect::new(WIDTH - 3, HEIGHT - 4, 3, 4)
        );

        let mut released = scene.clone();
        let mut pressed = scene.clone();
        let cursor = CursorState::visible(100, 200, false);
        let released_evidence = update_cursor(&scene, &mut released, CursorState::hidden(), cursor)
            .expect("released cursor");
        let pressed_evidence = update_cursor(
            &scene,
            &mut pressed,
            CursorState::hidden(),
            CursorState {
                pressed: true,
                ..cursor
            },
        )
        .expect("pressed cursor");
        assert_ne!(
            released_evidence.scanout_digest,
            pressed_evidence.scanout_digest
        );
    }

    #[test]
    fn alpha_blending_is_channel_exact_and_clears_the_x_byte() {
        assert_eq!(blend_xrgb8888(0xff00_0000, 0x00ff_ffff, 0), 0);
        assert_eq!(blend_xrgb8888(0, 0xffff_ffff, 255), 0x00ff_ffff);
        assert_eq!(blend_xrgb8888(0x0010_2030, 0x00f0_e0d0, 128), 0x0080_8080);
    }

    #[test]
    fn surface_lengths_are_rejected_before_any_copy() {
        let scene = vec![0_u32; PIXEL_COUNT];
        let mut scanout = vec![0_u32; PIXEL_COUNT];
        assert_eq!(
            compose_full(
                &scene[..PIXEL_COUNT - 1],
                &mut scanout,
                CursorState::hidden()
            ),
            Err(CompositorError::WrongScenePixelCount)
        );
        assert_eq!(
            compose_full(
                &scene,
                &mut scanout[..PIXEL_COUNT - 1],
                CursorState::hidden()
            ),
            Err(CompositorError::WrongScanoutPixelCount)
        );
    }

    #[test]
    fn scene_damage_disjoint_from_cursor_preserves_the_cursor_bytes() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let cursor = CursorState::visible(12, 342, false);
        let mut scanout = scene.clone();
        update_cursor(&scene, &mut scanout, CursorState::hidden(), cursor).expect("cursor");
        let cursor_before = scanout[342 * WIDTH + 12];
        let evidence = update_scene(
            &mut scene,
            &mut scanout,
            cursor,
            Rect::new(100, 100, 8, 4),
            |_, _| 0xff12_3456,
        )
        .expect("scene update");
        assert_eq!(evidence.damage, Rect::new(100, 100, 8, 4));
        assert_eq!(evidence.composition, evidence.damage);
        assert_eq!(evidence.written_pixels, 32);
        assert_eq!(evidence.restored_pixels, 32);
        assert_eq!(evidence.blended_pixels, 0);
        assert_eq!(scene[100 * WIDTH + 100], 0x0012_3456);
        assert_eq!(scanout[342 * WIDTH + 12], cursor_before);
    }

    #[test]
    fn scene_damage_under_cursor_recomposes_it_exactly_once() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let cursor = CursorState::visible(100, 200, false);
        let mut scanout = scene.clone();
        update_cursor(&scene, &mut scanout, CursorState::hidden(), cursor).expect("cursor");
        let evidence = update_scene(
            &mut scene,
            &mut scanout,
            cursor,
            Rect::new(104, 206, 2, 2),
            |_, _| COLOR_CURSOR_PRESSED,
        )
        .expect("overlapping scene update");
        assert_eq!(evidence.damage, Rect::new(104, 206, 2, 2));
        assert_eq!(evidence.composition, cursor.bounds());
        assert_eq!(evidence.restored_pixels, CURSOR_WIDTH * CURSOR_HEIGHT);
        assert!(evidence.blended_pixels > 0);

        let mut expected = vec![0_u32; PIXEL_COUNT];
        compose_full(&scene, &mut expected, cursor).expect("reference composition");
        assert_eq!(scanout, expected);
        assert_eq!(evidence.scanout_digest, pixel_digest(&expected));
    }

    #[test]
    fn empty_scene_damage_is_rejected_without_mutation() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let mut scanout = scene.clone();
        let before = scene.clone();
        assert_eq!(
            update_scene(
                &mut scene,
                &mut scanout,
                CursorState::hidden(),
                Rect::new(WIDTH, HEIGHT, 1, 1),
                |_, _| 0,
            ),
            Err(CompositorError::EmptyDamage)
        );
        assert_eq!(scene, before);
        assert_eq!(scanout, before);
    }

    #[test]
    fn strict_publish_validation_rejects_empty_clipped_and_overflowing_damage() {
        assert_eq!(
            validate_scene_damage(Rect::new(0, 0, 0, 1)),
            Err(CompositorError::EmptyDamage)
        );
        assert_eq!(
            validate_scene_damage(Rect::new(WIDTH, HEIGHT, 1, 1)),
            Err(CompositorError::DamageOutOfBounds)
        );
        assert_eq!(
            validate_scene_damage(Rect::new(usize::MAX, 0, 2, 1)),
            Err(CompositorError::DamageOutOfBounds)
        );
    }

    #[test]
    fn publishing_prerastered_disjoint_damage_preserves_every_cursor_byte() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let cursor = CursorState::visible(12, 342, false);
        let mut scanout = scene.clone();
        update_cursor(&scene, &mut scanout, CursorState::hidden(), cursor).expect("cursor");
        let cursor_before: std::vec::Vec<u32> = (cursor.bounds().y
            ..cursor.bounds().y + cursor.bounds().height)
            .flat_map(|y| {
                scanout[y * WIDTH + cursor.bounds().x
                    ..y * WIDTH + cursor.bounds().x + cursor.bounds().width]
                    .iter()
                    .copied()
            })
            .collect();
        let damage = Rect::new(100, 100, 8, 4);
        for y in damage.y..damage.y + damage.height {
            scene[y * WIDTH + damage.x..y * WIDTH + damage.x + damage.width].fill(0x0012_3456);
        }
        let scene: &[u32; PIXEL_COUNT] = scene.as_slice().try_into().unwrap();
        let scanout: &mut [u32; PIXEL_COUNT] = scanout.as_mut_slice().try_into().unwrap();
        let evidence = publish_scene_damage(
            scene,
            scanout,
            cursor,
            validate_scene_damage(damage).unwrap(),
        );
        let cursor_after: std::vec::Vec<u32> = (cursor.bounds().y
            ..cursor.bounds().y + cursor.bounds().height)
            .flat_map(|y| {
                scanout[y * WIDTH + cursor.bounds().x
                    ..y * WIDTH + cursor.bounds().x + cursor.bounds().width]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(cursor_after, cursor_before);
        assert_eq!(evidence.damage, damage);
        assert_eq!(evidence.composition, damage);
        assert_eq!(evidence.restored_pixels, 32);
        assert_eq!(evidence.blended_pixels, 0);
    }

    #[test]
    fn publishing_prerastered_cursor_damage_matches_one_fresh_composition() {
        let mut scene = vec![0_u32; PIXEL_COUNT];
        render_boot_splash(&mut scene).expect("scene geometry");
        let cursor = CursorState::visible(100, 200, true);
        let mut scanout = scene.clone();
        update_cursor(&scene, &mut scanout, CursorState::hidden(), cursor).expect("cursor");
        let damage = Rect::new(104, 206, 2, 2);
        for y in damage.y..damage.y + damage.height {
            scene[y * WIDTH + damage.x..y * WIDTH + damage.x + damage.width].fill(0x0012_3456);
        }
        let mut expected = vec![0_u32; PIXEL_COUNT];
        compose_full(&scene, &mut expected, cursor).expect("reference composition");
        let scene: &[u32; PIXEL_COUNT] = scene.as_slice().try_into().unwrap();
        let scanout: &mut [u32; PIXEL_COUNT] = scanout.as_mut_slice().try_into().unwrap();
        let evidence = publish_scene_damage(
            scene,
            scanout,
            cursor,
            validate_scene_damage(damage).unwrap(),
        );
        assert_eq!(&scanout[..], &expected);
        assert_eq!(evidence.composition, cursor.bounds());
        assert_eq!(evidence.restored_pixels, CURSOR_WIDTH * CURSOR_HEIGHT);
        assert!(evidence.blended_pixels > 0);
        assert_eq!(evidence.scanout_digest, pixel_digest(&expected));
    }
}
