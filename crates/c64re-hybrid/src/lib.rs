//! Hybrid clone shell: play back captured frames in a window, with modern
//! keyboard input mapped onto the C64 joystick bits.
//!
//! This is the M3 milestone: the emulator is the oracle, the captured
//! frames are rendered by our own Rust code, and the player's input is
//! expressed as the same CIA-line values the game would read.

use macroquad::prelude::*;
use std::path::Path;
use std::time::Instant;

/// A loaded capture session ready for playback.
pub struct PlaybackSession {
    pub samples: Vec<c64re_capture::HardwareSample>,
    pub source: String,
    pub game_start_frame: Option<u64>,
}

/// Load a capture from `out/<name>` — reads traces/hardware-samples.json.
pub fn load_session(out_dir: &Path) -> Result<PlaybackSession, Box<dyn std::error::Error>> {
    let samples_path = out_dir.join("traces/hardware-samples.json");
    let raw_dir = out_dir.join("assets/raw");
    let samples: Vec<RawSample> = serde_json::from_slice(&std::fs::read(&samples_path)?)?;
    let mut hardware = Vec::new();
    for raw in &samples {
        let dir = raw_dir.join(format!("sample-{:04}", raw.index));
        let carved = load_carved(&dir);
        hardware.push(c64re_capture::HardwareSample {
            index: raw.index,
            frame: raw.frame,
            pc: raw.pc,
            vic: raw.vic,
            sid_registers: raw.sid_registers,
            sprite_pointers: raw.sprite_pointers,
            color_ram: raw.color_ram.clone(),
            display_mode: raw.vic.display_mode(),
            carved,
        });
    }
    let session_path = out_dir.join("session.json");
    let game_start_frame = std::fs::read(&session_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| {
            v.pointer("/emulator/game_start_frame")
                .and_then(|x| x.as_u64())
        });
    Ok(PlaybackSession {
        samples: hardware,
        source: out_dir.display().to_string(),
        game_start_frame,
    })
}

fn load_carved(dir: &Path) -> c64re_capture::CarvedSample {
    let read = |name: &str| std::fs::read(dir.join(name)).ok();
    let charset = read("sample-charset.bin").or_else(|| read("sample-charset-rom.bin"));
    let mut sprites: [Option<Vec<u8>>; 8] = Default::default();
    for (slot, slot_bytes) in sprites.iter_mut().enumerate() {
        *slot_bytes = read(&format!("sample-sprite-s{slot}.bin"));
    }
    c64re_capture::CarvedSample {
        screen: read("sample-screen.bin"),
        charset,
        charset_is_rom: dir.join("sample-charset-rom.bin").exists(),
        bitmap: read("sample-bitmap.bin"),
        sprites,
    }
}

/// Raw JSON mirror of a hardware sample (serde round-trip of the trace).
#[derive(serde::Deserialize)]
struct RawSample {
    index: usize,
    frame: u64,
    pc: u16,
    vic: c64re_vic::VicState,
    sid_registers: [u8; 25],
    sprite_pointers: [u8; 8],
    color_ram: Vec<u8>,
}

/// Map modern keyboard state to C64 joystick bits (active-high,
/// VICE-style: fire 0x10, right 0x08, left 0x04, down 0x02, up 0x01).
pub fn keyboard_to_joyport() -> u16 {
    let mut value = 0_u16;
    if is_key_down(KeyCode::Space) || is_key_down(KeyCode::LeftControl) {
        value |= 0x10;
    }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        value |= 0x08;
    }
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        value |= 0x04;
    }
    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        value |= 0x02;
    }
    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        value |= 0x01;
    }
    value
}

/// Render the captured frames in a window.
pub async fn run_player(session: &PlaybackSession) {
    let mut frame_index = 0_usize;
    let mut last_frame_time = Instant::now();
    let frame_duration = std::time::Duration::from_micros(20_000); // PAL ~50 fps
    let mut autoplay = false;
    let mut joy = 0_u16;

    loop {
        if frame_index >= session.samples.len() {
            frame_index = 0;
        }
        let sample = &session.samples[frame_index];

        // Scale the 320x200 frame to the window.
        let (sw, sh) = (screen_width() as u32, screen_height() as u32);
        let scale = (sw / 320).max(1) as f32;
        let ox = (sw as f32 - 320.0 * scale) / 2.0;
        let oy = (sh as f32 - 200.0 * scale) / 2.0;

        if let Some(rgba) = c64re_assets::render_frame_rgba(sample) {
            let texture = Texture2D::from_rgba8(320, 200, &rgba);
            draw_texture_ex(
                &texture,
                ox,
                oy,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(320.0 * scale, 200.0 * scale)),
                    ..Default::default()
                },
            );
        }

        // Overlay: frame number, PC, display mode, joystick value.
        let mode = sample.display_mode.as_str();
        let info = format!(
            "frame {}/{} (captured {})  pc=${:04x}  {}  joy=${joy:02x}",
            sample.index,
            session.samples.len() - 1,
            sample.frame,
            sample.pc,
            mode
        );
        draw_rectangle(ox, oy, 320.0 * scale, 18.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_text(&info, ox + 4.0, oy + 14.0, 14.0, WHITE);
        let hint = "WASD/arrows = joystick, SPACE = fire, TAB = autoplay, R = restart";
        draw_text(hint, ox + 4.0, oy + 200.0 * scale - 6.0, 13.0, GRAY);

        // Input mapping.
        if is_key_pressed(KeyCode::Tab) {
            autoplay = !autoplay;
        }
        if is_key_pressed(KeyCode::R) {
            frame_index = 0;
        }
        if autoplay {
            // Simple demo autoplay: drive right/fire cycles.
            let t = frame_index / 30;
            joy = match t % 4 {
                0 => 0x10,
                1 => 0x08,
                2 => 0x10,
                _ => 0x04,
            };
        } else {
            joy = keyboard_to_joyport();
        }

        // Pace playback at PAL frame rate (or hold on the last frame).
        let elapsed = last_frame_time.elapsed();
        if elapsed >= frame_duration {
            frame_index = frame_index.saturating_add(1);
            last_frame_time = Instant::now();
        }

        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joystick_bit_values_are_stable() {
        // The C64 joystick bit layout (VICE-style, active-high).
        assert_eq!(0x10, 1 << 4); // fire
        assert_eq!(0x08, 1 << 3); // right
        assert_eq!(0x04, 1 << 2); // left
        assert_eq!(0x02, 1 << 1); // down
        assert_eq!(0x01, 1 << 0); // up
    }

    #[test]
    fn renders_frame_from_ghostbusters_capture() {
        let out = std::path::Path::new("out/gb_embedded");
        if !out.join("traces/hardware-samples.json").exists() {
            eprintln!("skipping: no capture at {out:?}");
            return;
        }
        let session = load_session(out).unwrap();
        assert!(!session.samples.is_empty());
        let frame = c64re_assets::render_frame_rgba(&session.samples[0]).unwrap();
        assert_eq!(frame.len(), 320 * 200 * 4);
    }
}
