//! Asset extraction and preview rendering from captured samples.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use c64re_capture::HardwareSample;
use c64re_vic::DisplayMode;

use crate::{
    render_charset_grid_rgba, render_hires_bitmap_rgba, render_multicolor_bitmap_rgba,
    render_multicolor_text_rgba, render_sprite_multicolor_rgba, render_sprite_rgba,
    render_text_screen_rgba, write_png_rgba, SCREEN_HEIGHT_CHARS, SCREEN_WIDTH_CHARS,
    SPRITE_HEIGHT, SPRITE_WIDTH,
};

#[derive(Debug, Clone)]
pub struct AssetRecord {
    pub kind: &'static str,
    pub address: u16,
    pub sample_index: usize,
    pub path: String,
    pub preview_path: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssetExtractionSummary {
    pub manifest_path: String,
    pub screen_count: usize,
    pub charset_count: usize,
    pub sprite_count: usize,
    pub screens: Vec<AssetRecord>,
    pub charsets: Vec<AssetRecord>,
    pub sprites: Vec<AssetRecord>,
}

/// Extract assets from captured samples: dedupe sprites by content hash and
/// render previews in the correct display mode. The carved bytes were read at
/// observation time; this only dedupes and renders them.
pub fn extract_observed_assets(
    assets_dir: &Path,
    capture: &c64re_capture::ViceCapture,
) -> Result<AssetExtractionSummary, Box<dyn std::error::Error>> {
    let screen_dir = assets_dir.join("screens");
    let charset_dir = assets_dir.join("charsets");
    let sprite_dir = assets_dir.join("sprites");
    fs::create_dir_all(&screen_dir)?;
    fs::create_dir_all(&charset_dir)?;
    fs::create_dir_all(&sprite_dir)?;

    let mut seen_screens = std::collections::BTreeSet::new();
    let mut seen_charsets = std::collections::BTreeSet::new();
    let mut sprite_keys: BTreeMap<(u16, u64), usize> = BTreeMap::new();
    let mut screens = Vec::new();
    let mut charsets = Vec::new();
    let mut sprites: Vec<SpriteAssetRecord> = Vec::new();

    for sample in &capture.samples {
        let screen_address = sample.vic.screen_base();
        if seen_screens.insert(screen_address) {
            if let Some(screen) = sample.carved.screen.as_deref() {
                let base = format!("screen-{}", hex_name(screen_address));
                let raw_path = screen_dir.join(format!("{base}.bin"));
                fs::write(&raw_path, screen)?;
                let mut preview_path = None;
                let note = render_screen_preview(
                    sample,
                    screen,
                    &screen_dir,
                    &base,
                    assets_dir,
                    &mut preview_path,
                )?;
                screens.push(AssetRecord {
                    kind: "screen",
                    address: screen_address,
                    sample_index: sample.index,
                    path: relative_asset_path(&raw_path, assets_dir),
                    preview_path,
                    note,
                });
            }
        }

        let charset_address = sample.vic.charset_base();
        let charset_key = if sample.carved.charset_is_rom {
            u32::MAX
        } else {
            u32::from(charset_address)
        };
        if seen_charsets.insert(charset_key) {
            if let Some(charset) = sample.carved.charset.as_deref() {
                let base = format!("charset-{}", hex_name(charset_address));
                let raw_path = charset_dir.join(format!("{base}.bin"));
                fs::write(&raw_path, charset)?;
                let mut preview_path = None;
                if let Some(rgba) = render_charset_grid_rgba(charset) {
                    let path = charset_dir.join(format!("{base}.png"));
                    write_png_rgba(&path, 128, 128, &rgba)?;
                    preview_path = Some(relative_asset_path(&path, assets_dir));
                }
                let note = sample
                    .carved
                    .charset_is_rom
                    .then(|| "character ROM (VIC bank 0/2 charset base)".to_string());
                charsets.push(AssetRecord {
                    kind: "charset",
                    address: charset_address,
                    sample_index: sample.index,
                    path: relative_asset_path(&raw_path, assets_dir),
                    preview_path,
                    note,
                });
            }
        }

        for sprite_index in 0..8 {
            if !sample.vic.sprite_enabled(sprite_index) {
                continue;
            }
            let Some(sprite) = sample.carved.sprites[sprite_index].as_deref() else {
                continue;
            };
            let pointer = sample.sprite_pointers[sprite_index];
            let sprite_address = sample
                .vic
                .vic_bank_base()
                .wrapping_add(u16::from(pointer) * crate::SPRITE_BYTES as u16);
            let hash = content_hash(sprite);
            let key = (sprite_address, hash);
            if let Some(&existing) = sprite_keys.get(&key) {
                sprites[existing].frames.push(sample.frame);
                sprites[existing].slots.push(sprite_index);
                continue;
            }
            let index = sprites.len();
            sprite_keys.insert(key, index);
            let base = format!("sprite-{}-{:x}", hex_name(sprite_address), hash);
            let raw_path = sprite_dir.join(format!("{base}.bin"));
            fs::write(&raw_path, sprite)?;
            let mut preview_path = None;
            render_sprite_preview(
                sample,
                sprite_index,
                sprite,
                &sprite_dir,
                &base,
                assets_dir,
                &mut preview_path,
            )?;
            let note = if sample.vic.sprite_multicolor(sprite_index) {
                Some("multicolor sprite".to_string())
            } else {
                None
            };
            sprites.push(SpriteAssetRecord {
                record: AssetRecord {
                    kind: "sprite",
                    address: sprite_address,
                    sample_index: sample.index,
                    path: relative_asset_path(&raw_path, assets_dir),
                    preview_path,
                    note,
                },
                frames: vec![sample.frame],
                slots: vec![sprite_index],
            });
        }
    }

    let summary = AssetExtractionSummary {
        manifest_path: "assets/manifest.json".to_string(),
        screen_count: screens.len(),
        charset_count: charsets.len(),
        sprite_count: sprites.len(),
        screens,
        charsets,
        sprites: sprites.into_iter().map(|s| s.record).collect(),
    };
    fs::write(
        assets_dir.join("manifest.json"),
        asset_manifest_json(&summary),
    )?;
    Ok(summary)
}

#[derive(Debug, Clone)]
struct SpriteAssetRecord {
    record: AssetRecord,
    frames: Vec<u64>,
    slots: Vec<usize>,
}

/// Render the preview for a screen according to its actual display mode.
fn render_screen_preview(
    sample: &HardwareSample,
    screen: &[u8],
    screen_dir: &Path,
    base: &str,
    assets_dir: &Path,
    preview_path: &mut Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let rgba = match sample.display_mode {
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
    };
    if let Some(rgba) = rgba {
        let path = screen_dir.join(format!("{base}.png"));
        write_png_rgba(
            &path,
            (SCREEN_WIDTH_CHARS * 8) as u32,
            (SCREEN_HEIGHT_CHARS * 8) as u32,
            &rgba,
        )?;
        *preview_path = Some(relative_asset_path(&path, assets_dir));
    }
    Ok(Some(format!(
        "rendered in {} mode",
        sample.display_mode.as_str()
    )))
}

/// Render a sprite preview honoring multicolor mode.
fn render_sprite_preview(
    sample: &HardwareSample,
    sprite_index: usize,
    sprite: &[u8],
    sprite_dir: &Path,
    base: &str,
    assets_dir: &Path,
    preview_path: &mut Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rgba = if sample.vic.sprite_multicolor(sprite_index) {
        render_sprite_multicolor_rgba(
            sprite,
            sample.vic.sprite_colors_d027_d02e[sprite_index],
            sample.vic.multicolor_0_d025,
            sample.vic.multicolor_1_d026,
        )
    } else {
        render_sprite_rgba(sprite, sample.vic.sprite_colors_d027_d02e[sprite_index])
    };
    if let Some(rgba) = &rgba {
        let path = sprite_dir.join(format!("{base}.png"));
        write_png_rgba(&path, SPRITE_WIDTH as u32, SPRITE_HEIGHT as u32, rgba)?;
        *preview_path = Some(relative_asset_path(&path, assets_dir));
    }
    Ok(())
}

/// Simple 64-bit content hash for dedupe.
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

pub fn relative_asset_path(path: &Path, assets_dir: &Path) -> String {
    let relative = path.strip_prefix(assets_dir).unwrap_or(path);
    format!("assets/{}", relative.display())
}

pub fn asset_manifest_json(summary: &AssetExtractionSummary) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"manifest_path\": \"{}\",\n",
        summary.manifest_path
    ));
    out.push_str("  \"screens\": ");
    write_asset_array_json(&mut out, &summary.screens, 2);
    out.push_str(",\n  \"charsets\": ");
    write_asset_array_json(&mut out, &summary.charsets, 2);
    out.push_str(",\n  \"sprites\": ");
    write_asset_array_json(&mut out, &summary.sprites, 2);
    out.push_str("\n}\n");
    out
}

fn write_asset_array_json(out: &mut String, records: &[AssetRecord], indent: usize) {
    let pad = " ".repeat(indent);
    let item_pad = " ".repeat(indent + 2);
    out.push_str("[\n");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!("{item_pad}{{\n"));
        out.push_str(&format!("{item_pad}  \"kind\": \"{}\",\n", record.kind));
        out.push_str(&format!("{item_pad}  \"address\": {},\n", record.address));
        out.push_str(&format!(
            "{item_pad}  \"address_hex\": \"{}\",\n",
            hex16(record.address)
        ));
        out.push_str(&format!(
            "{item_pad}  \"sample_index\": {},\n",
            record.sample_index
        ));
        out.push_str(&format!(
            "{item_pad}  \"path\": \"{}\",\n",
            json_escape(&record.path)
        ));
        match &record.preview_path {
            Some(path) => out.push_str(&format!(
                "{item_pad}  \"preview_path\": \"{}\",\n",
                json_escape(path)
            )),
            None => out.push_str(&format!("{item_pad}  \"preview_path\": null,\n")),
        }
        match &record.note {
            Some(note) => out.push_str(&format!(
                "{item_pad}  \"note\": \"{}\"\n",
                json_escape(note)
            )),
            None => out.push_str(&format!("{item_pad}  \"note\": null\n")),
        }
        out.push_str(&format!("{item_pad}}}"));
    }
    out.push_str(&format!("\n{pad}]"));
}

pub fn asset_summary_markdown(summary: &AssetExtractionSummary) -> String {
    let mut out = String::new();
    out.push_str("# Observed Assets\n\n");
    out.push_str("Assets carved at observation time from sampled VIC-II state.\n\n");
    out.push_str(&format!("- Manifest: `{}`\n", summary.manifest_path));
    out.push_str(&format!("- Screen blocks: {}\n", summary.screen_count));
    out.push_str(&format!("- Charset blocks: {}\n", summary.charset_count));
    out.push_str(&format!(
        "- Displayed sprite blocks: {}\n\n",
        summary.sprite_count
    ));
    write_asset_table_markdown(&mut out, "Screens", &summary.screens);
    write_asset_table_markdown(&mut out, "Charsets", &summary.charsets);
    write_asset_table_markdown(&mut out, "Sprites", &summary.sprites);
    out
}

fn write_asset_table_markdown(out: &mut String, title: &str, records: &[AssetRecord]) {
    out.push_str(&format!("## {title}\n\n"));
    if records.is_empty() {
        out.push_str("No observed assets of this type.\n\n");
        return;
    }
    out.push_str("| Address | Sample | Raw | Preview | Note |\n");
    out.push_str("| ---: | ---: | --- | --- | --- |\n");
    for record in records {
        out.push_str(&format!(
            "| {} | {} | `{}` | {} | {} |\n",
            hex16(record.address),
            record.sample_index,
            record.path,
            record
                .preview_path
                .as_ref()
                .map(|path| format!("`{path}`"))
                .unwrap_or_else(|| "-".to_string()),
            record.note.as_deref().unwrap_or("-")
        ));
    }
    out.push('\n');
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

pub fn hex16(value: u16) -> String {
    format!("${value:04x}")
}

pub fn hex_name(value: u16) -> String {
    format!("{value:04x}")
}
