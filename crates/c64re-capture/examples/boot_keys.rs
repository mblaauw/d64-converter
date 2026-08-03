//! Boot-key experiment: step frames; when the title screen is detected
//! (crack intro boxer art), feed SPACE then ESC (sequential). Then watch
//! for the next screens: instructions -> ESC, game loaded -> Y.
use c64re_capture::{connect, read_raster_line};
use c64re_vice_bmp::ViceMonitor;
use std::time::Duration;

const SPACE: &[u8] = &[0x20];
const ESC: &[u8] = &[0x1b];
const KEY_Y: &[u8] = &[0x59];

fn step_frames(monitor: &mut ViceMonitor, frames: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut prev = read_raster_line(monitor)?;
    for _ in 0..frames {
        let mut wrapped = false;
        for _ in 0..16 {
            monitor.step_instructions(2500, false)?;
            let raster = read_raster_line(monitor)?;
            wrapped = raster < prev;
            prev = raster;
            if wrapped {
                break;
            }
        }
        if !wrapped {
            return Err("stalled: raster stopped".into());
        }
    }
    Ok(())
}

fn screen_text(monitor: &mut ViceMonitor) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = monitor.read_memory_in(c64re_vice_bmp::Memspace::Main, 0x0400, 0x0800, false, 1)?;
    Ok(bytes
        .iter()
        .map(|&c| {
            if (32..127).contains(&c) {
                c as char
            } else {
                '.'
            }
        })
        .collect())
}

fn dump_screen(txt: &str, label: &str) {
    println!("=== {label}");
    for row in 0..25 {
        let line = &txt[row * 40..(row + 1) * 40];
        if !line.trim_matches('.').is_empty() {
            println!("{line}");
        }
    }
}

fn pc(monitor: &mut ViceMonitor) -> Result<u16, Box<dyn std::error::Error>> {
    let regs = monitor.registers_raw()?;
    Ok(regs
        .iter()
        .find(|r| r.id == 3)
        .map(|r| r.value)
        .unwrap_or(0))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let disk = args
        .next()
        .unwrap_or_else(|| "/Users/mich/Downloads/International Karate Plus.d64".into());
    let max_frames: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(20000);

    let monitor_addr = "ip4://127.0.0.1:6502";
    let mut child = std::process::Command::new("x64sc")
        .args([
            "-default",
            "-warp",
            "-silent",
            "-drive8type",
            "1541",
            "-controlport1device",
            "io",
            "-controlport2device",
            "io",
            "-binarymonitor",
            "-binarymonitoraddress",
            monitor_addr,
            &disk,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut monitor = connect("127.0.0.1:6502")?;
        monitor.set_read_timeout(Duration::from_secs(10))?;
        let mut frame = 0_u64;
        let mut prev_signature = String::new();
        let mut stage: u8 = 0; // 0=boot/load, 1=title, 2=instructions, 3=game
        let mut keys_sent_at: Option<u64> = None;

        while frame < max_frames {
            step_frames(&mut monitor, 50)?;
            frame += 50;
            let txt = screen_text(&mut monitor)?;
            let sig: String = txt
                .lines()
                .map(|l| l.trim_matches('.'))
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("|");
            let changed = sig != prev_signature;
            if changed {
                println!(
                    "[frame {frame}] pc=${:04x} stage={stage} key={:?}",
                    pc(&mut monitor)?,
                    keys_sent_at
                );
                dump_screen(&txt, &format!("frame {frame}"));
                prev_signature = sig.clone();
            }
            let d018 = monitor
                .read_memory(0xd018, 0xd018)?
                .first()
                .copied()
                .unwrap_or(0);
            let art_present = sig.chars().filter(|&c| c != '|' && c != '.').count() > 40;

            match stage {
                0 => {
                    // waiting for the title art (crack intro)
                    if art_present && d018 != 0x13 {
                        stage = 1;
                        println!(
                            "[frame {frame}] TITLE detected (d018=${d018:02x}); sending SPACE"
                        );
                        monitor.keyboard_feed(SPACE)?;
                        keys_sent_at = Some(frame);
                    }
                }
                1 => {
                    // SPACE sent; send ESC a bit later (sequential)
                    if let Some(at) = keys_sent_at {
                        if frame >= at + 30 {
                            println!("[frame {frame}] sending ESC (after SPACE)");
                            monitor.keyboard_feed(ESC)?;
                            stage = 2;
                            keys_sent_at = Some(frame);
                        }
                    }
                }
                2 => {
                    // waiting for the load/instruction screen change; then ESC
                    if let Some(at) = keys_sent_at {
                        if frame >= at + 60 && changed {
                            println!("[frame {frame}] screen changed after intro ESC; sending ESC for instructions");
                            monitor.keyboard_feed(ESC)?;
                            stage = 3;
                            keys_sent_at = Some(frame);
                        }
                    }
                }
                3 => {
                    // waiting for the game; then Y (flush highscore)
                    if let Some(at) = keys_sent_at {
                        if frame >= at + 60 && changed {
                            println!(
                                "[frame {frame}] screen changed again; sending Y (flush highscore)"
                            );
                            monitor.keyboard_feed(KEY_Y)?;
                            stage = 4;
                            keys_sent_at = Some(frame);
                        }
                    }
                }
                _ => {
                    // post-game: keep dumping changes
                }
            }
        }
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
