use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub const SPRITE_WIDTH: usize = 24;
pub const SPRITE_HEIGHT: usize = 21;
pub const SPRITE_BYTES: usize = 64;
pub const CHARSET_BYTES: usize = 2048;
pub const SCREEN_BYTES: usize = 1000;
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
}
