//! Deterministic XRGB8888 framebuffer geometry and boot-splash renderer.

#[cfg(not(feature = "mobile-ui-runtime"))]
pub const WIDTH: usize = 320;
#[cfg(feature = "mobile-ui-runtime")]
pub const WIDTH: usize = 720;
#[cfg(not(feature = "mobile-ui-runtime"))]
pub const HEIGHT: usize = 480;
#[cfg(feature = "mobile-ui-runtime")]
pub const HEIGHT: usize = 1_600;
pub const BYTES_PER_PIXEL: usize = 4;
pub const STRIDE: usize = WIDTH * BYTES_PER_PIXEL;
pub const PIXEL_COUNT: usize = WIDTH * HEIGHT;
pub const BYTE_COUNT: usize = PIXEL_COUNT * BYTES_PER_PIXEL;

pub const COLOR_BACKGROUND: u32 = 0x0010_162d;
pub const COLOR_STATUS: u32 = 0x002e_66f5;
pub const COLOR_PHONE_BORDER: u32 = 0x00f8_fafc;
pub const COLOR_PHONE_SCREEN: u32 = 0x001e_293b;
pub const COLOR_PHONE_HEADER: u32 = 0x001d_4ed8;
pub const COLOR_CARD_CYAN: u32 = 0x0006_b6d4;
pub const COLOR_CARD_PURPLE: u32 = 0x008b_5cf6;
pub const COLOR_CARD_GREEN: u32 = 0x0022_c55e;
pub const COLOR_NAVIGATION: u32 = 0x001f_2937;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    WrongPixelCount,
}

impl RenderError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongPixelCount => "framebuffer pixel slice has the wrong length",
        }
    }
}

pub fn render_boot_splash(pixels: &mut [u32]) -> Result<u64, RenderError> {
    if pixels.len() != PIXEL_COUNT {
        return Err(RenderError::WrongPixelCount);
    }
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            pixels[y * WIDTH + x] = boot_splash_pixel(x, y);
        }
    }
    Ok(pixel_digest(pixels))
}

#[cfg(not(feature = "mobile-ui-runtime"))]
pub const fn boot_splash_pixel(x: usize, y: usize) -> u32 {
    if x >= WIDTH || y >= HEIGHT {
        return 0;
    }
    let mut color = COLOR_BACKGROUND;
    if y < 28 {
        color = COLOR_STATUS;
    }
    if x >= 48 && x < 272 && y >= 56 && y < 440 {
        color = COLOR_PHONE_BORDER;
    }
    if x >= 56 && x < 264 && y >= 64 && y < 432 {
        color = COLOR_PHONE_SCREEN;
    }
    if x >= 56 && x < 264 && y >= 64 && y < 112 {
        color = COLOR_PHONE_HEADER;
    }
    if x >= 72 && x < 248 && y >= 132 && y < 204 {
        color = COLOR_CARD_CYAN;
    }
    if x >= 72 && x < 248 && y >= 220 && y < 292 {
        color = COLOR_CARD_PURPLE;
    }
    if x >= 72 && x < 248 && y >= 308 && y < 380 {
        color = COLOR_CARD_GREEN;
    }
    if y >= 448 {
        color = COLOR_NAVIGATION;
    }
    if x >= 128 && x < 192 && y >= 458 && y < 464 {
        color = COLOR_PHONE_BORDER;
    }
    color
}

#[cfg(feature = "mobile-ui-runtime")]
pub const fn boot_splash_pixel(x: usize, y: usize) -> u32 {
    if x >= WIDTH || y >= HEIGHT {
        return 0;
    }
    if x >= 330 && x < 390 && y >= 702 && y < 762 {
        return COLOR_PHONE_BORDER;
    }
    if x >= 288 && x < 432 && y >= 660 && y < 804 {
        return COLOR_PHONE_HEADER;
    }
    if x >= 264 && x < 456 && y >= 842 && y < 854 {
        return COLOR_CARD_CYAN;
    }
    if y < 72 {
        return 0x0008_0d1b;
    }
    COLOR_BACKGROUND
}

pub fn pixel_digest(pixels: &[u32]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for pixel in pixels {
        for byte in pixel.to_le_bytes() {
            digest ^= u64::from(byte);
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn geometry_is_exact_and_page_bounded() {
        #[cfg(not(feature = "mobile-ui-runtime"))]
        {
            assert_eq!(STRIDE, 1280);
            assert_eq!(PIXEL_COUNT, 153_600);
            assert_eq!(BYTE_COUNT, 614_400);
        }
        #[cfg(feature = "mobile-ui-runtime")]
        {
            assert_eq!(STRIDE, 2_880);
            assert_eq!(PIXEL_COUNT, 1_152_000);
            assert_eq!(BYTE_COUNT, 4_608_000);
        }
        assert_eq!(BYTE_COUNT % 4096, 0);
    }

    #[cfg(not(feature = "mobile-ui-runtime"))]
    #[test]
    fn splash_layers_and_bounds_are_deterministic() {
        assert_eq!(boot_splash_pixel(0, 0), COLOR_STATUS);
        assert_eq!(boot_splash_pixel(0, 100), COLOR_BACKGROUND);
        assert_eq!(boot_splash_pixel(50, 100), COLOR_PHONE_BORDER);
        assert_eq!(boot_splash_pixel(60, 100), COLOR_PHONE_HEADER);
        assert_eq!(boot_splash_pixel(100, 160), COLOR_CARD_CYAN);
        assert_eq!(boot_splash_pixel(100, 230), COLOR_CARD_PURPLE);
        assert_eq!(boot_splash_pixel(100, 320), COLOR_CARD_GREEN);
        assert_eq!(boot_splash_pixel(160, 455), COLOR_NAVIGATION);
        assert_eq!(boot_splash_pixel(160, 460), COLOR_PHONE_BORDER);
        assert_eq!(boot_splash_pixel(WIDTH, 0), 0);
        assert_eq!(boot_splash_pixel(0, HEIGHT), 0);
    }

    #[cfg(feature = "mobile-ui-runtime")]
    #[test]
    fn mobile_splash_covers_the_full_twenty_by_nine_scanout() {
        assert_eq!((WIDTH, HEIGHT), (720, 1_600));
        assert_eq!(boot_splash_pixel(0, 0), 0x0008_0d1b);
        assert_eq!(boot_splash_pixel(0, 100), COLOR_BACKGROUND);
        assert_eq!(boot_splash_pixel(300, 700), COLOR_PHONE_HEADER);
        assert_eq!(boot_splash_pixel(350, 720), COLOR_PHONE_BORDER);
        assert_eq!(boot_splash_pixel(300, 846), COLOR_CARD_CYAN);
        assert_eq!(boot_splash_pixel(WIDTH, 0), 0);
        assert_eq!(boot_splash_pixel(0, HEIGHT), 0);
    }

    #[test]
    fn complete_splash_has_a_stable_byte_digest() {
        let mut pixels = vec![0_u32; PIXEL_COUNT];
        let digest = render_boot_splash(&mut pixels).expect("exact framebuffer");
        assert_eq!(digest, pixel_digest(&pixels));
        #[cfg(not(feature = "mobile-ui-runtime"))]
        assert_eq!(digest, 0x6ef9_c2b7_d15f_de25);
        assert_eq!(
            render_boot_splash(&mut pixels[..PIXEL_COUNT - 1]),
            Err(RenderError::WrongPixelCount)
        );
    }
}
