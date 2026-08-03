pub mod extract;

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub use c64re_capture::{CHARSET_BYTES, SCREEN_BYTES, SPRITE_BYTES};
use c64re_vic::DisplayMode;

pub const SPRITE_WIDTH: usize = 24;
pub const SPRITE_HEIGHT: usize = 21;
pub const SCREEN_WIDTH_CHARS: usize = 40;
pub const SCREEN_HEIGHT_CHARS: usize = 25;

pub const C64_PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0xff, 0xff, 0xff],
    [0x88, 0x00, 0x00],
    [0xaa, 0xff, 0xee],
    [0xcc, 0x44, 0xcc],
    [0x00, 0xcc, 0x55],
    [0x00, 0x00, 0xaa],
    [0xee, 0xee, 0x77],
    [0xdd, 0x88, 0x55],
    [0x66, 0x44, 0x00],
    [0xff, 0x77, 0x77],
    [0x33, 0x33, 0x33],
    [0x77, 0x77, 0x77],
    [0xaa, 0xff, 0x66],
    [0x00, 0x88, 0xff],
    [0xbb, 0xbb, 0xbb],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonochromeSprite {
    pub pixels: [[bool; SPRITE_WIDTH]; SPRITE_HEIGHT],
}

impl MonochromeSprite {
    pub fn from_c64_block(block: &[u8]) -> Option<Self> {
        if block.len() < SPRITE_BYTES {
            return None;
        }

        let mut pixels = [[false; SPRITE_WIDTH]; SPRITE_HEIGHT];
        for y in 0..SPRITE_HEIGHT {
            for byte_x in 0..3 {
                let byte = block[y * 3 + byte_x];
                for bit in 0..8 {
                    pixels[y][byte_x * 8 + bit] = byte & (0x80 >> bit) != 0;
                }
            }
        }

        Some(Self { pixels })
    }

    pub fn to_ascii_art(&self) -> String {
        let mut out = String::new();
        for row in &self.pixels {
            for &pixel in row {
                out.push(if pixel { '#' } else { '.' });
            }
            out.push('\n');
        }
        out
    }
}

pub fn write_png_rgba(
    path: impl AsRef<Path>,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

pub fn render_sprite_rgba(block: &[u8], color: u8) -> Option<Vec<u8>> {
    let sprite = MonochromeSprite::from_c64_block(block)?;
    let fg = C64_PALETTE[usize::from(color & 0x0f)];
    let mut rgba = vec![0_u8; SPRITE_WIDTH * SPRITE_HEIGHT * 4];
    for y in 0..SPRITE_HEIGHT {
        for x in 0..SPRITE_WIDTH {
            let offset = (y * SPRITE_WIDTH + x) * 4;
            if sprite.pixels[y][x] {
                rgba[offset] = fg[0];
                rgba[offset + 1] = fg[1];
                rgba[offset + 2] = fg[2];
                rgba[offset + 3] = 0xff;
            }
        }
    }
    Some(rgba)
}

/// Render a multicolor sprite: 12 wide-pixels per row (24 screen pixels),
/// using the sprite color plus the shared multicolor pair ($D025/$D026).
/// Pixel pairs: 00 = transparent, 01 = sprite color, 10 = mc0, 11 = mc1.
pub fn render_sprite_multicolor_rgba(
    block: &[u8],
    sprite_color: u8,
    mc0: u8,
    mc1: u8,
) -> Option<Vec<u8>> {
    if block.len() < SPRITE_BYTES {
        return None;
    }
    let palette = C64_PALETTE;
    let colors = [
        [0_u8; 3],
        palette[usize::from(sprite_color & 0x0f)],
        palette[usize::from(mc0 & 0x0f)],
        palette[usize::from(mc1 & 0x0f)],
    ];
    let mut rgba = vec![0_u8; SPRITE_WIDTH * SPRITE_HEIGHT * 4];
    for y in 0..SPRITE_HEIGHT {
        for byte_x in 0..3 {
            let byte = block[y * 3 + byte_x];
            for pair in 0..4 {
                let code = (byte >> (6 - pair * 2)) & 0x03;
                let color = colors[usize::from(code)];
                let x0 = byte_x * 8 + pair * 2;
                for dx in 0..2 {
                    let offset = (y * SPRITE_WIDTH + x0 + dx) * 4;
                    rgba[offset] = color[0];
                    rgba[offset + 1] = color[1];
                    rgba[offset + 2] = color[2];
                    rgba[offset + 3] = 0xff;
                }
            }
        }
    }
    Some(rgba)
}

/// Render a multicolor text screen using the screen matrix, charset, and
/// color RAM. Each character: even bits are the 4-color MC pattern,
/// odd bits select between background color and color-RAM color.
pub fn render_multicolor_text_rgba(
    screen: &[u8],
    charset: &[u8],
    color_ram: &[u8],
    background_color: u8,
    mc0: u8,
    mc1: u8,
) -> Option<Vec<u8>> {
    if screen.len() < SCREEN_BYTES || charset.len() < CHARSET_BYTES {
        return None;
    }
    let color_ram = if color_ram.len() < SCREEN_BYTES {
        &[0_u8; SCREEN_BYTES][..]
    } else {
        color_ram
    };
    let palette = C64_PALETTE;
    let width = SCREEN_WIDTH_CHARS * 8;
    let height = SCREEN_HEIGHT_CHARS * 8;
    let mut rgba = vec![0_u8; width * height * 4];

    for cy in 0..SCREEN_HEIGHT_CHARS {
        for cx in 0..SCREEN_WIDTH_CHARS {
            let ch = usize::from(screen[cy * SCREEN_WIDTH_CHARS + cx]);
            let ram_color = palette[usize::from(color_ram[cy * SCREEN_WIDTH_CHARS + cx] & 0x0f)];
            let bg = C64_PALETTE[usize::from(background_color & 0x0f)];
            let mc_colors = [
                bg,
                ram_color,
                palette[usize::from(mc0 & 0x0f)],
                palette[usize::from(mc1 & 0x0f)],
            ];
            for row in 0..8 {
                let bits = charset[ch * 8 + row];
                for pair in 0..4 {
                    // Multicolor text: pixel pairs from the left, each 2 bits.
                    // 00 = background, 01 = color RAM, 10 = mc0, 11 = mc1.
                    let hi = (bits >> (7 - pair * 2)) & 0x01;
                    let lo = (bits >> (6 - pair * 2)) & 0x01;
                    let code = usize::from((hi << 1) | lo);
                    let color = mc_colors[code];
                    let x = cx * 8 + pair * 2;
                    let y = cy * 8 + row;
                    for dx in 0..2 {
                        let offset = (y * width + x + dx) * 4;
                        rgba[offset] = color[0];
                        rgba[offset + 1] = color[1];
                        rgba[offset + 2] = color[2];
                        rgba[offset + 3] = 0xff;
                    }
                }
            }
        }
    }
    Some(rgba)
}

/// Render a hires bitmap screen: 8000 bytes, 8 bytes per 8-pixel cell,
/// color RAM selects the foreground color per cell.
pub fn render_hires_bitmap_rgba(
    bitmap: &[u8],
    color_ram: &[u8],
    background_color: u8,
) -> Option<Vec<u8>> {
    if bitmap.len() < 8000 {
        return None;
    }
    let color_ram = if color_ram.len() < SCREEN_BYTES {
        &[0_u8; SCREEN_BYTES][..]
    } else {
        color_ram
    };
    let palette = C64_PALETTE;
    let width = SCREEN_WIDTH_CHARS * 8;
    let height = SCREEN_HEIGHT_CHARS * 8;
    let mut rgba = vec![0_u8; width * height * 4];

    for cy in 0..SCREEN_HEIGHT_CHARS {
        for cx in 0..SCREEN_WIDTH_CHARS {
            let cell = cy * SCREEN_WIDTH_CHARS + cx;
            let fg = palette[usize::from(color_ram[cell] & 0x0f)];
            let bg = palette[usize::from(background_color & 0x0f)];
            for row in 0..8 {
                let bits = bitmap[cell * 8 + row];
                for bit in 0..8 {
                    let pixel_on = bits & (0x80 >> bit) != 0;
                    let color = if pixel_on { fg } else { bg };
                    let x = cx * 8 + bit;
                    let y = cy * 8 + row;
                    let offset = (y * width + x) * 4;
                    rgba[offset] = color[0];
                    rgba[offset + 1] = color[1];
                    rgba[offset + 2] = color[2];
                    rgba[offset + 3] = 0xff;
                }
            }
        }
    }
    Some(rgba)
}

/// Render a multicolor bitmap screen: pixel pairs per cell use background,
/// color-RAM, $D022, and $D023.
pub fn render_multicolor_bitmap_rgba(
    bitmap: &[u8],
    color_ram: &[u8],
    background_color: u8,
    bg1: u8,
    bg2: u8,
) -> Option<Vec<u8>> {
    if bitmap.len() < 8000 {
        return None;
    }
    let color_ram = if color_ram.len() < SCREEN_BYTES {
        &[0_u8; SCREEN_BYTES][..]
    } else {
        color_ram
    };
    let palette = C64_PALETTE;
    let width = SCREEN_WIDTH_CHARS * 8;
    let height = SCREEN_HEIGHT_CHARS * 8;
    let mut rgba = vec![0_u8; width * height * 4];

    for cy in 0..SCREEN_HEIGHT_CHARS {
        for cx in 0..SCREEN_WIDTH_CHARS {
            let cell = cy * SCREEN_WIDTH_CHARS + cx;
            let colors = [
                palette[usize::from(background_color & 0x0f)],
                palette[usize::from(color_ram[cell] & 0x0f)],
                palette[usize::from(bg1 & 0x0f)],
                palette[usize::from(bg2 & 0x0f)],
            ];
            for row in 0..8 {
                let bits = bitmap[cell * 8 + row];
                for pair in 0..4 {
                    let hi = (bits >> (7 - pair * 2)) & 0x01;
                    let lo = (bits >> (6 - pair * 2)) & 0x01;
                    let code = usize::from((hi << 1) | lo);
                    let color = colors[code];
                    let x = cx * 8 + pair * 2;
                    let y = cy * 8 + row;
                    for dx in 0..2 {
                        let offset = (y * width + x + dx) * 4;
                        rgba[offset] = color[0];
                        rgba[offset + 1] = color[1];
                        rgba[offset + 2] = color[2];
                        rgba[offset + 3] = 0xff;
                    }
                }
            }
        }
    }
    Some(rgba)
}

pub fn render_charset_grid_rgba(charset: &[u8]) -> Option<Vec<u8>> {
    if charset.len() < CHARSET_BYTES {
        return None;
    }

    let width = 16 * 8;
    let height = 16 * 8;
    let mut rgba = vec![0_u8; width * height * 4];
    for ch in 0..256 {
        let cell_x = (ch % 16) * 8;
        let cell_y = (ch / 16) * 8;
        for row in 0..8 {
            let bits = charset[ch * 8 + row];
            for bit in 0..8 {
                let pixel_on = bits & (0x80 >> bit) != 0;
                let x = cell_x + bit;
                let y = cell_y + row;
                let offset = (y * width + x) * 4;
                let color = if pixel_on {
                    C64_PALETTE[1]
                } else {
                    C64_PALETTE[0]
                };
                rgba[offset] = color[0];
                rgba[offset + 1] = color[1];
                rgba[offset + 2] = color[2];
                rgba[offset + 3] = 0xff;
            }
        }
    }
    Some(rgba)
}

pub fn render_text_screen_rgba(
    screen: &[u8],
    charset: &[u8],
    background_color: u8,
    foreground_color: u8,
) -> Option<Vec<u8>> {
    if screen.len() < SCREEN_BYTES || charset.len() < CHARSET_BYTES {
        return None;
    }

    let width = SCREEN_WIDTH_CHARS * 8;
    let height = SCREEN_HEIGHT_CHARS * 8;
    let bg = C64_PALETTE[usize::from(background_color & 0x0f)];
    let fg = C64_PALETTE[usize::from(foreground_color & 0x0f)];
    let mut rgba = vec![0_u8; width * height * 4];

    for cy in 0..SCREEN_HEIGHT_CHARS {
        for cx in 0..SCREEN_WIDTH_CHARS {
            let ch = usize::from(screen[cy * SCREEN_WIDTH_CHARS + cx]);
            for row in 0..8 {
                let bits = charset[ch * 8 + row];
                for bit in 0..8 {
                    let pixel_on = bits & (0x80 >> bit) != 0;
                    let x = cx * 8 + bit;
                    let y = cy * 8 + row;
                    let offset = (y * width + x) * 4;
                    let color = if pixel_on { fg } else { bg };
                    rgba[offset] = color[0];
                    rgba[offset + 1] = color[1];
                    rgba[offset + 2] = color[2];
                    rgba[offset + 3] = 0xff;
                }
            }
        }
    }
    Some(rgba)
}

/// Compose a full 320x200 frame: render the screen in its actual display
/// mode, then blit the enabled sprites at their hardware positions.
///
/// `sprite_x`/`sprite_y` are the VIC sprite coordinates (0-511 / 0-255);
/// Y-expanded sprites are drawn doubled vertically. Sprites use
/// transparent-background RGBA so they composite over the screen.
pub fn compose_frame_rgba(sample: &c64re_capture::HardwareSample, screen_rgba: &mut [u8]) {
    let width = SCREEN_WIDTH_CHARS * 8;
    for sprite_index in 0..8 {
        if !sample.vic.sprite_enabled(sprite_index) {
            continue;
        }
        let Some(block) = sample.carved.sprites[sprite_index].as_deref() else {
            continue;
        };
        let (sprite_rgba, sprite_w) = if sample.vic.sprite_multicolor(sprite_index) {
            let rgba = render_sprite_multicolor_rgba(
                block,
                sample.vic.sprite_colors_d027_d02e[sprite_index],
                sample.vic.multicolor_0_d025,
                sample.vic.multicolor_1_d026,
            )
            .unwrap_or_default();
            (rgba, SPRITE_WIDTH)
        } else {
            let rgba = render_sprite_rgba(block, sample.vic.sprite_colors_d027_d02e[sprite_index])
                .unwrap_or_default();
            (rgba, SPRITE_WIDTH)
        };
        let x_expand = sample.vic.sprite_x_expanded(sprite_index);
        let y_expand = sample.vic.sprite_y_expanded(sprite_index);
        let x = usize::from(sample.vic.sprite_x[sprite_index]);
        let y = usize::from(sample.vic.sprite_y[sprite_index]);
        for sy in 0..SPRITE_HEIGHT {
            let ty = y.wrapping_add(sy);
            if ty >= 200 {
                continue;
            }
            for sx in 0..sprite_w {
                let tx = x.wrapping_add(sx);
                if tx >= width {
                    continue;
                }
                let offset = (sy * sprite_w + sx) * 4;
                let alpha = sprite_rgba[offset + 3];
                if alpha == 0 {
                    continue;
                }
                let dst = (ty * width + tx) * 4;
                screen_rgba[dst..dst + 4].copy_from_slice(&sprite_rgba[offset..offset + 4]);
                if x_expand && tx + 1 < width {
                    let dst2 = ((ty * width) + tx + 1) * 4;
                    screen_rgba[dst2..dst2 + 4].copy_from_slice(&sprite_rgba[offset..offset + 4]);
                }
                if y_expand && ty + 1 < 200 {
                    let dst3 = ((ty + 1) * width + tx) * 4;
                    screen_rgba[dst3..dst3 + 4].copy_from_slice(&sprite_rgba[offset..offset + 4]);
                }
            }
        }
    }
}

/// Render the full frame for a captured sample: display-mode-aware screen
/// plus sprites. Returns 320x200 RGBA.
pub fn render_frame_rgba(sample: &c64re_capture::HardwareSample) -> Option<Vec<u8>> {
    let width = SCREEN_WIDTH_CHARS * 8;
    let height = SCREEN_HEIGHT_CHARS * 8;
    let mut rgba = vec![0_u8; width * height * 4];
    let screen = sample.carved.screen.as_deref()?;
    let filled = match sample.display_mode {
        DisplayMode::StandardText | DisplayMode::ExtendedBackground => {
            sample.carved.charset.as_deref().and_then(|charset| {
                render_text_screen_rgba(screen, charset, sample.vic.background_color_d021, 1)
            })
        }
        DisplayMode::MulticolorText => sample.carved.charset.as_deref().and_then(|charset| {
            render_multicolor_text_rgba(
                screen,
                charset,
                &sample.color_ram,
                sample.vic.background_color_d021,
                sample.vic.multicolor_0_d025,
                sample.vic.multicolor_1_d026,
            )
        }),
        DisplayMode::HiresBitmap => sample.carved.bitmap.as_deref().and_then(|bitmap| {
            render_hires_bitmap_rgba(bitmap, &sample.color_ram, sample.vic.background_color_d021)
        }),
        DisplayMode::MulticolorBitmap => sample.carved.bitmap.as_deref().and_then(|bitmap| {
            render_multicolor_bitmap_rgba(
                bitmap,
                &sample.color_ram,
                sample.vic.background_color_d021,
                sample.vic.background_1_d022,
                sample.vic.background_2_d023,
            )
        }),
    }?;
    rgba.copy_from_slice(&filled);
    compose_frame_rgba(sample, &mut rgba);
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_first_sprite_pixel() {
        let mut block = [0_u8; SPRITE_BYTES];
        block[0] = 0x80;
        let sprite = MonochromeSprite::from_c64_block(&block).unwrap();
        assert!(sprite.pixels[0][0]);
        assert!(!sprite.pixels[0][1]);
    }

    #[test]
    fn renders_charset_grid_size() {
        let charset = [0_u8; CHARSET_BYTES];
        let rgba = render_charset_grid_rgba(&charset).unwrap();
        assert_eq!(rgba.len(), 128 * 128 * 4);
    }

    #[test]
    fn renders_multicolor_sprite_size() {
        let block = [0_u8; SPRITE_BYTES];
        let rgba = render_sprite_multicolor_rgba(&block, 1, 5, 6).unwrap();
        assert_eq!(rgba.len(), SPRITE_WIDTH * SPRITE_HEIGHT * 4);
    }

    #[test]
    fn renders_multicolor_text_size() {
        let screen = [0_u8; SCREEN_BYTES];
        let charset = [0_u8; CHARSET_BYTES];
        let color_ram = [0_u8; SCREEN_BYTES];
        let rgba = render_multicolor_text_rgba(&screen, &charset, &color_ram, 0, 4, 14).unwrap();
        assert_eq!(rgba.len(), 320 * 200 * 4);
    }

    #[test]
    fn renders_bitmap_size() {
        let bitmap = [0_u8; 8000];
        let color_ram = [0_u8; SCREEN_BYTES];
        let hires = render_hires_bitmap_rgba(&bitmap, &color_ram, 0).unwrap();
        assert_eq!(hires.len(), 320 * 200 * 4);
        let mc = render_multicolor_bitmap_rgba(&bitmap, &color_ram, 0, 1, 2).unwrap();
        assert_eq!(mc.len(), 320 * 200 * 4);
    }
}
