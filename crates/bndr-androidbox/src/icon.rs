use crate::digest::crc32;
use crate::{AndroidBoxEnvelopeScratch, Error};

pub const ANDROID_LAUNCHER_ICON_WIDTH: usize = 16;
pub const ANDROID_LAUNCHER_ICON_HEIGHT: usize = 16;
pub const ANDROID_LAUNCHER_ICON_PIXEL_COUNT: usize =
    ANDROID_LAUNCHER_ICON_WIDTH * ANDROID_LAUNCHER_ICON_HEIGHT;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
#[cfg(feature = "androidbox-density-icons7")]
const MDP_SOURCE_WIDTH: usize = 48;
#[cfg(feature = "androidbox-density-icons7")]
const MDP_SOURCE_HEIGHT: usize = 48;
#[cfg(feature = "androidbox-density-icons7")]
const MDP_DENSITY_DPI: u16 = 160;

/// Exact, owned launcher artwork decoded from one signed APK resource.
///
/// Pixels use canonical `0xAARRGGBB`; fully transparent pixels carry zero RGB.
/// The fixed 16x16 profile keeps parsing, ABI transfer, and rendering bounded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidLauncherIcon {
    resource_id: u32,
    png_crc32: u32,
    pixels: [u32; ANDROID_LAUNCHER_ICON_PIXEL_COUNT],
    #[cfg(feature = "androidbox-density-icons7")]
    source_width: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    source_height: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    source_density_dpi: u16,
    #[cfg(feature = "androidbox-density-icons7")]
    color_quantized: bool,
}

impl AndroidLauncherIcon {
    pub const fn resource_id(self) -> u32 {
        self.resource_id
    }

    pub const fn png_crc32(self) -> u32 {
        self.png_crc32
    }

    pub const fn pixels(&self) -> &[u32; ANDROID_LAUNCHER_ICON_PIXEL_COUNT] {
        &self.pixels
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_width(self) -> u16 {
        self.source_width
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_height(self) -> u16 {
        self.source_height
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn source_density_dpi(self) -> u16 {
        self.source_density_dpi
    }

    #[cfg(feature = "androidbox-density-icons7")]
    pub const fn color_quantized(self) -> bool {
        self.color_quantized
    }
}

pub(crate) fn decode_launcher_icon(
    resource_id: u32,
    expected_png_crc32: u32,
    density_dpi: u16,
    png: &[u8],
    scratch: &mut AndroidBoxEnvelopeScratch,
) -> Result<AndroidLauncherIcon, Error> {
    if resource_id == 0
        || png.len() < PNG_SIGNATURE.len()
        || png.get(..PNG_SIGNATURE.len()) != Some(PNG_SIGNATURE)
        || crc32(png) != expected_png_crc32
    {
        return Err(Error::LauncherIconPng);
    }

    let mut cursor = PNG_SIGNATURE.len();
    let mut saw_ihdr = false;
    let mut saw_idat = false;
    let mut saw_iend = false;
    let mut idat = None;
    let mut source_width = 0usize;
    let mut source_height = 0usize;
    while cursor < png.len() {
        let length =
            usize::try_from(read_be_u32(png, cursor)?).map_err(|_| Error::LauncherIconPng)?;
        let kind_start = cursor.checked_add(4).ok_or(Error::LauncherIconPng)?;
        let data_start = kind_start.checked_add(4).ok_or(Error::LauncherIconPng)?;
        let data_end = data_start
            .checked_add(length)
            .ok_or(Error::LauncherIconPng)?;
        let chunk_end = data_end.checked_add(4).ok_or(Error::LauncherIconPng)?;
        let kind = png
            .get(kind_start..data_start)
            .ok_or(Error::LauncherIconPng)?;
        let data = png
            .get(data_start..data_end)
            .ok_or(Error::LauncherIconPng)?;
        let expected_chunk_crc = read_be_u32(png, data_end)?;
        if crc32(
            png.get(kind_start..data_end)
                .ok_or(Error::LauncherIconPng)?,
        ) != expected_chunk_crc
        {
            return Err(Error::LauncherIconPng);
        }

        match kind {
            b"IHDR" if !saw_ihdr && !saw_idat && length == 13 => {
                source_width =
                    usize::try_from(read_be_u32(data, 0)?).map_err(|_| Error::LauncherIconPng)?;
                source_height =
                    usize::try_from(read_be_u32(data, 4)?).map_err(|_| Error::LauncherIconPng)?;
                let default_source = source_width == ANDROID_LAUNCHER_ICON_WIDTH
                    && source_height == ANDROID_LAUNCHER_ICON_HEIGHT
                    && density_dpi == 0;
                #[cfg(feature = "androidbox-density-icons7")]
                let supported_source = default_source
                    || (source_width == MDP_SOURCE_WIDTH
                        && source_height == MDP_SOURCE_HEIGHT
                        && density_dpi == MDP_DENSITY_DPI);
                #[cfg(not(feature = "androidbox-density-icons7"))]
                let supported_source = default_source;
                if !supported_source || data[8..] != [8, 6, 0, 0, 0] {
                    return Err(Error::LauncherIconPng);
                }
                saw_ihdr = true;
            }
            b"IDAT" if saw_ihdr && !saw_idat && !saw_iend && !data.is_empty() => {
                idat = Some(data);
                saw_idat = true;
            }
            b"IEND" if saw_ihdr && saw_idat && !saw_iend && data.is_empty() => {
                saw_iend = true;
                if chunk_end != png.len() {
                    return Err(Error::LauncherIconPng);
                }
            }
            _ => return Err(Error::LauncherIconPng),
        }
        cursor = chunk_end;
    }
    if !saw_ihdr || !saw_idat || !saw_iend || cursor != png.len() {
        return Err(Error::LauncherIconPng);
    }

    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{
        decompress,
        inflate_flags::{TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF},
    };

    let scanline_bytes = source_width
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(1))
        .ok_or(Error::LauncherIconPng)?;
    let inflated_bytes = source_height
        .checked_mul(scanline_bytes)
        .ok_or(Error::LauncherIconPng)?;
    let (decompressor, output) = scratch.inflate_parts_mut();
    if inflated_bytes > output.len() {
        return Err(Error::LauncherIconPng);
    }
    let compressed = idat.ok_or(Error::LauncherIconPng)?;
    let (status, input_read, output_written) = decompress(
        decompressor,
        compressed,
        output,
        0,
        TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    if status != TINFLStatus::Done
        || input_read != compressed.len()
        || output_written != inflated_bytes
    {
        return Err(Error::LauncherIconPng);
    }

    let decoded = &mut output[..inflated_bytes];
    unfilter_rgba8(decoded, source_width, source_height, scanline_bytes)?;
    let mut pixels = [0u32; ANDROID_LAUNCHER_ICON_PIXEL_COUNT];
    if source_width == ANDROID_LAUNCHER_ICON_WIDTH {
        for y in 0..ANDROID_LAUNCHER_ICON_HEIGHT {
            let row = y * scanline_bytes + 1;
            for x in 0..ANDROID_LAUNCHER_ICON_WIDTH {
                let source = row + x * 4;
                pixels[y * ANDROID_LAUNCHER_ICON_WIDTH + x] = canonical_pixel(
                    decoded[source],
                    decoded[source + 1],
                    decoded[source + 2],
                    decoded[source + 3],
                );
            }
        }
    } else {
        #[cfg(feature = "androidbox-density-icons7")]
        normalize_mdpi_rgba8(decoded, scanline_bytes, &mut pixels)?;
        #[cfg(not(feature = "androidbox-density-icons7"))]
        return Err(Error::LauncherIconPng);
    }
    #[cfg(feature = "androidbox-density-icons7")]
    let color_quantized = quantize_icon_pixels(&mut pixels);
    Ok(AndroidLauncherIcon {
        resource_id,
        png_crc32: expected_png_crc32,
        pixels,
        #[cfg(feature = "androidbox-density-icons7")]
        source_width: u16::try_from(source_width).map_err(|_| Error::LauncherIconPng)?,
        #[cfg(feature = "androidbox-density-icons7")]
        source_height: u16::try_from(source_height).map_err(|_| Error::LauncherIconPng)?,
        #[cfg(feature = "androidbox-density-icons7")]
        source_density_dpi: density_dpi,
        #[cfg(feature = "androidbox-density-icons7")]
        color_quantized,
    })
}

const fn canonical_pixel(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
    if alpha == 0 {
        0
    } else {
        (alpha as u32) << 24 | (red as u32) << 16 | (green as u32) << 8 | blue as u32
    }
}

#[cfg(feature = "androidbox-density-icons7")]
fn normalize_mdpi_rgba8(
    decoded: &[u8],
    scanline_bytes: usize,
    pixels: &mut [u32; ANDROID_LAUNCHER_ICON_PIXEL_COUNT],
) -> Result<(), Error> {
    if decoded.len() != MDP_SOURCE_HEIGHT * scanline_bytes
        || scanline_bytes != 1 + MDP_SOURCE_WIDTH * 4
    {
        return Err(Error::LauncherIconPng);
    }
    for target_y in 0..ANDROID_LAUNCHER_ICON_HEIGHT {
        for target_x in 0..ANDROID_LAUNCHER_ICON_WIDTH {
            let mut alpha_sum = 0u32;
            let mut red_alpha_sum = 0u32;
            let mut green_alpha_sum = 0u32;
            let mut blue_alpha_sum = 0u32;
            for source_y in target_y * 3..target_y * 3 + 3 {
                for source_x in target_x * 3..target_x * 3 + 3 {
                    let source = source_y * scanline_bytes + 1 + source_x * 4;
                    let alpha = u32::from(decoded[source + 3]);
                    alpha_sum += alpha;
                    red_alpha_sum += u32::from(decoded[source]) * alpha;
                    green_alpha_sum += u32::from(decoded[source + 1]) * alpha;
                    blue_alpha_sum += u32::from(decoded[source + 2]) * alpha;
                }
            }
            pixels[target_y * ANDROID_LAUNCHER_ICON_WIDTH + target_x] = if alpha_sum == 0 {
                0
            } else {
                let alpha = ((alpha_sum + 4) / 9) as u8;
                let red = ((red_alpha_sum + alpha_sum / 2) / alpha_sum) as u8;
                let green = ((green_alpha_sum + alpha_sum / 2) / alpha_sum) as u8;
                let blue = ((blue_alpha_sum + alpha_sum / 2) / alpha_sum) as u8;
                canonical_pixel(red, green, blue, alpha)
            };
        }
    }
    Ok(())
}

#[cfg(feature = "androidbox-density-icons7")]
fn quantize_icon_pixels(pixels: &mut [u32; ANDROID_LAUNCHER_ICON_PIXEL_COUNT]) -> bool {
    const PALETTE_CAPACITY: usize = 16;

    let has_transparent = pixels.contains(&0);
    let mut palette = [0; PALETTE_CAPACITY];
    let mut counts = [0u16; PALETTE_CAPACITY];
    let first_color = usize::from(has_transparent);
    let mut palette_len = first_color;
    let mut unique_colors = first_color;

    for index in 0..pixels.len() {
        let pixel = pixels[index];
        if pixel == 0 && has_transparent {
            continue;
        }
        if pixels[..index].contains(&pixel) {
            continue;
        }
        unique_colors += 1;
        let count = pixels.iter().filter(|value| **value == pixel).count() as u16;
        let mut insertion = first_color;
        while insertion < palette_len
            && (counts[insertion] > count
                || (counts[insertion] == count && palette[insertion] < pixel))
        {
            insertion += 1;
        }
        if insertion >= palette.len() {
            continue;
        }
        let end = core::cmp::min(palette_len, palette.len() - 1);
        let mut cursor = end;
        while cursor > insertion {
            palette[cursor] = palette[cursor - 1];
            counts[cursor] = counts[cursor - 1];
            cursor -= 1;
        }
        palette[insertion] = pixel;
        counts[insertion] = count;
        if palette_len < palette.len() {
            palette_len += 1;
        }
    }
    if unique_colors <= PALETTE_CAPACITY {
        return false;
    }

    for pixel in pixels {
        if *pixel == 0 && has_transparent {
            continue;
        }
        let mut best = first_color;
        let mut best_distance = icon_color_distance(*pixel, palette[best]);
        for candidate in first_color + 1..palette_len {
            let distance = icon_color_distance(*pixel, palette[candidate]);
            if distance < best_distance
                || (distance == best_distance && palette[candidate] < palette[best])
            {
                best = candidate;
                best_distance = distance;
            }
        }
        *pixel = palette[best];
    }
    true
}

#[cfg(feature = "androidbox-density-icons7")]
fn icon_color_distance(left: u32, right: u32) -> u64 {
    let left_alpha = (left >> 24) & 0xff;
    let right_alpha = (right >> 24) & 0xff;
    let mut distance = i64::from(left_alpha).abs_diff(i64::from(right_alpha)) * 4;
    for shift in [16, 8, 0] {
        let left_component = ((left >> shift) & 0xff) * left_alpha;
        let right_component = ((right >> shift) & 0xff) * right_alpha;
        distance += i64::from(left_component).abs_diff(i64::from(right_component));
    }
    distance
}

fn unfilter_rgba8(
    bytes: &mut [u8],
    width: usize,
    height: usize,
    scanline_bytes: usize,
) -> Result<(), Error> {
    if width == 0
        || height == 0
        || scanline_bytes != 1 + width * 4
        || bytes.len() != height * scanline_bytes
    {
        return Err(Error::LauncherIconPng);
    }
    for y in 0..height {
        let row_start = y * scanline_bytes;
        let filter = bytes[row_start];
        for x in 0..width * 4 {
            let offset = row_start + 1 + x;
            let left = if x >= 4 { bytes[offset - 4] } else { 0 };
            let up = if y == 0 {
                0
            } else {
                bytes[offset - scanline_bytes]
            };
            let upper_left = if y != 0 && x >= 4 {
                bytes[offset - scanline_bytes - 4]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                4 => paeth(left, up, upper_left),
                _ => return Err(Error::LauncherIconPng),
            };
            bytes[offset] = bytes[offset].wrapping_add(predictor);
        }
    }
    Ok(())
}

fn paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let left = i16::from(left);
    let up = i16::from(up);
    let upper_left = i16::from(upper_left);
    let estimate = left + up - upper_left;
    let left_distance = (estimate - left).abs();
    let up_distance = (estimate - up).abs();
    let upper_left_distance = (estimate - upper_left).abs();
    if left_distance <= up_distance && left_distance <= upper_left_distance {
        left as u8
    } else if up_distance <= upper_left_distance {
        up as u8
    } else {
        upper_left as u8
    }
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let raw: [u8; 4] = bytes
        .get(offset..offset.checked_add(4).ok_or(Error::LauncherIconPng)?)
        .ok_or(Error::LauncherIconPng)?
        .try_into()
        .map_err(|_| Error::LauncherIconPng)?;
    Ok(u32::from_be_bytes(raw))
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "androidbox-density-icons7")]
    use super::quantize_icon_pixels;
    use super::{decode_launcher_icon, read_be_u32};
    use crate::digest::crc32;
    use crate::{AndroidBoxEnvelopeScratch, Error};

    const ENVELOPE_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x10, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0xf3, 0xff, 0x61, 0x00, 0x00, 0x00, 0x36, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60,
        0x80, 0x02, 0xee, 0xf0, 0x0b, 0xff, 0x49, 0xc1, 0x0c, 0xe8, 0x80, 0x6c, 0x03, 0x48, 0xd5,
        0x88, 0x61, 0x10, 0xba, 0x00, 0x21, 0x40, 0x3f, 0x03, 0x28, 0xf6, 0x02, 0x4d, 0xc2, 0x80,
        0xbe, 0x06, 0x0c, 0xce, 0x30, 0xa0, 0x4f, 0x3a, 0xa0, 0x5b, 0x9e, 0xa0, 0x5a, 0x6e, 0x04,
        0x00, 0xf0, 0xa3, 0x6e, 0xe1, 0x63, 0xf0, 0x49, 0xac, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    #[cfg(feature = "androidbox-density-icons7")]
    const ENVELOPE_MDPI_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x30, 0x08, 0x06, 0x00, 0x00, 0x00, 0x57,
        0x02, 0xf9, 0x87, 0x00, 0x00, 0x00, 0x62, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0xed, 0xd9,
        0x41, 0x0e, 0x00, 0x10, 0x0c, 0x04, 0xc0, 0xde, 0x3d, 0xd0, 0xb7, 0xfd, 0x88, 0x17, 0x38,
        0x90, 0x48, 0x94, 0xd9, 0xc4, 0xdd, 0x5c, 0x1a, 0xba, 0x11, 0x93, 0x94, 0xda, 0xfa, 0x4d,
        0x27, 0x56, 0x03, 0x00, 0x00, 0x90, 0x04, 0x70, 0xdb, 0x45, 0x97, 0x61, 0x00, 0xd9, 0x00,
        0xa7, 0x03, 0x00, 0x00, 0x00, 0xb0, 0x07, 0x48, 0x3f, 0x46, 0x01, 0x00, 0x00, 0x3e, 0x7e,
        0x0b, 0x01, 0x00, 0x00, 0x00, 0x18, 0xa3, 0x00, 0x00, 0x00, 0xf6, 0x42, 0xfe, 0xc4, 0x00,
        0x00, 0x00, 0xfa, 0x01, 0x80, 0x87, 0xbb, 0x33, 0x35, 0x2b, 0x00, 0xc0, 0xa3, 0x80, 0x01,
        0x22, 0x0b, 0xe6, 0x0e, 0xf0, 0xcd, 0x35, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn decode(png: &[u8], expected_crc32: u32) -> Result<super::AndroidLauncherIcon, Error> {
        let mut scratch = AndroidBoxEnvelopeScratch::new();
        decode_launcher_icon(0x7f01_0000, expected_crc32, 0, png, &mut scratch)
    }

    #[test]
    fn exact_png_decodes_to_canonical_bounded_pixels() {
        let png_crc32 = crc32(ENVELOPE_PNG);
        let icon = decode(ENVELOPE_PNG, png_crc32).unwrap();
        assert_eq!(icon.resource_id(), 0x7f01_0000);
        assert_eq!(icon.png_crc32(), 0xf63d_0b72);
        assert_eq!(icon.png_crc32(), png_crc32);
        assert!(icon.pixels().contains(&0xff0b_57d0));
        assert!(icon.pixels().contains(&0xffff_ffff));
        assert!(icon.pixels().contains(&0));
        assert!(
            icon.pixels()
                .iter()
                .all(|pixel| pixel >> 24 != 0 || *pixel == 0)
        );
    }

    #[cfg(feature = "androidbox-density-icons7")]
    #[test]
    fn exact_mdpi_png_normalizes_to_the_same_retained_artwork() {
        let mut default_scratch = AndroidBoxEnvelopeScratch::new();
        let default = decode_launcher_icon(
            0x7f01_0000,
            crc32(ENVELOPE_PNG),
            0,
            ENVELOPE_PNG,
            &mut default_scratch,
        )
        .unwrap();
        let mut mdpi_scratch = AndroidBoxEnvelopeScratch::new();
        let mdpi = decode_launcher_icon(
            0x7f01_0000,
            crc32(ENVELOPE_MDPI_PNG),
            160,
            ENVELOPE_MDPI_PNG,
            &mut mdpi_scratch,
        )
        .unwrap();
        assert_eq!(mdpi.source_width(), 48);
        assert_eq!(mdpi.source_height(), 48);
        assert_eq!(mdpi.source_density_dpi(), 160);
        assert!(!mdpi.color_quantized());
        assert_eq!(mdpi.pixels(), default.pixels());
        assert_eq!(mdpi.png_crc32(), 0xfc4d_4dc6);
    }

    #[cfg(feature = "androidbox-density-icons7")]
    #[test]
    fn color_quantization_is_deterministic_bounded_and_preserves_transparency() {
        let mut first = [0u32; super::ANDROID_LAUNCHER_ICON_PIXEL_COUNT];
        for (index, pixel) in first.iter_mut().enumerate().skip(1) {
            let value = index as u32;
            *pixel = 0xff00_0000
                | ((value.wrapping_mul(73) & 0xff) << 16)
                | ((value.wrapping_mul(151) & 0xff) << 8)
                | (value.wrapping_mul(199) & 0xff);
        }
        let mut second = first;
        assert!(quantize_icon_pixels(&mut first));
        assert!(quantize_icon_pixels(&mut second));
        assert_eq!(first, second);
        assert_eq!(first[0], 0);
        let mut palette = [0u32; 16];
        let mut palette_len = 0usize;
        for pixel in first {
            if !palette[..palette_len].contains(&pixel) {
                palette[palette_len] = pixel;
                palette_len += 1;
            }
        }
        assert_eq!(palette_len, 16);
    }

    #[test]
    fn png_identity_chunk_integrity_and_terminal_shape_fail_closed() {
        assert_eq!(
            decode(ENVELOPE_PNG, crc32(ENVELOPE_PNG) ^ 1),
            Err(Error::LauncherIconPng)
        );

        let mut corrupt_chunk = ENVELOPE_PNG.to_vec();
        corrupt_chunk[48] ^= 0x01;
        let corrupt_crc32 = crc32(&corrupt_chunk);
        assert_eq!(
            decode(&corrupt_chunk, corrupt_crc32),
            Err(Error::LauncherIconPng)
        );

        let mut trailing = ENVELOPE_PNG.to_vec();
        trailing.push(0);
        let trailing_crc32 = crc32(&trailing);
        assert_eq!(
            decode(&trailing, trailing_crc32),
            Err(Error::LauncherIconPng)
        );
    }

    #[test]
    fn png_dimensions_and_resource_identity_fail_closed() {
        let mut wrong_width = ENVELOPE_PNG.to_vec();
        wrong_width[19] = 0x0f;
        let ihdr_crc = crc32(&wrong_width[12..29]);
        wrong_width[29..33].copy_from_slice(&ihdr_crc.to_be_bytes());
        let whole_crc = crc32(&wrong_width);
        assert_eq!(decode(&wrong_width, whole_crc), Err(Error::LauncherIconPng));

        let mut scratch = AndroidBoxEnvelopeScratch::new();
        assert_eq!(
            decode_launcher_icon(0, crc32(ENVELOPE_PNG), 0, ENVELOPE_PNG, &mut scratch),
            Err(Error::LauncherIconPng)
        );
        assert_eq!(
            read_be_u32(ENVELOPE_PNG, ENVELOPE_PNG.len()),
            Err(Error::LauncherIconPng)
        );
    }
}
