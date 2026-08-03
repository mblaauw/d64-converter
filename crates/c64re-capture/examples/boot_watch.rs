//! Boot-sequence watcher: step frames from a cmdline-autostarted disk and
//! dump the screen at intervals so the boot flow (crack intro -> load ->
//! instructions -> game) can be mapped. Then a key script can be calibrated.
use c64re_capture::{connect, read_raster_line};
use c64re_vice_bmp::ViceMonitor;
use std::time::Duration;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let disk = args
        .next()
        .unwrap_or_else(|| "/Users/mich/Downloads/International Karate Plus.d64".into());
    let total_frames: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(4000);
    let chunk: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(120);

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
        while frame < total_frames {
            step_frames(&mut monitor, chunk)?;
            frame += chunk;
            let txt = screen_text(&mut monitor)?;
            // signature: only the non-dot rows, compacted
            let sig: String = txt
                .lines()
                .map(|l| l.trim_matches('.'))
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("|");
            if sig != prev_signature {
                dump_screen(&txt, &format!("frame {frame}"));
                prev_signature = sig;
            }
        }
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
