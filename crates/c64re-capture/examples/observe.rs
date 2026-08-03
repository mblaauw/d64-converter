//! Observer: poll the running VICE, logging screen changes, d018, and PC.
//! IMPORTANT: the binary monitor traps the emulator on every command, so we
//! send a continue (Exit) after each poll — otherwise VICE stays frozen in
//! the monitor and manual play is impossible.
use c64re_capture::connect;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let poll_ms: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(250);
    let max_secs: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(600);

    let mut monitor = connect("127.0.0.1:6502")?;
    monitor.set_read_timeout(Duration::from_secs(5))?;

    let poll = |m: &mut c64re_vice_bmp::ViceMonitor| -> (u16, u8, String) {
        let result = (|| -> Result<(u16, u8, String), Box<dyn std::error::Error>> {
            let pc = m
                .registers_raw()?
                .iter()
                .find(|r| r.id == 3)
                .map(|r| r.value)
                .unwrap_or(0);
            let d018 = m.read_memory(0xd018, 0xd018)?.first().copied().unwrap_or(0);
            let txt = m
                .read_memory_in(c64re_vice_bmp::Memspace::Main, 0x0400, 0x0800, false, 1)?
                .iter()
                .map(|&c| {
                    if (32..127).contains(&c) {
                        c as char
                    } else {
                        '.'
                    }
                })
                .collect::<String>();
            Ok((pc, d018, txt))
        })();
        // Resume the emulator after every command burst.
        let _ = m.continue_run();
        match result {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[observer] poll failed: {e}");
                (0, 0, String::new())
            }
        }
    };

    let signature = |txt: &str| -> String {
        txt.lines()
            .map(|l| l.trim_matches('.'))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("|")
    };

    let start = std::time::Instant::now();
    let mut prev_sig = String::new();
    let mut last_report = Duration::ZERO;
    loop {
        let (p, d, txt) = poll(&mut monitor);
        let sig = signature(&txt);
        let elapsed = start.elapsed();
        if sig != prev_sig {
            println!(
                "[t={:6.1}s] pc=${p:04x} d018=${d:02x} changed:",
                elapsed.as_secs_f32()
            );
            for row in 0..25 {
                let line = &txt[row * 40..(row + 1) * 40];
                if !line.trim_matches('.').is_empty() {
                    println!("{line}");
                }
            }
            prev_sig = sig;
            last_report = elapsed;
        } else if elapsed - last_report > Duration::from_secs(15) {
            println!(
                "[t={:6.1}s] (no change) pc=${p:04x} d018=${d:02x}",
                elapsed.as_secs_f32()
            );
            last_report = elapsed;
        }
        if elapsed > Duration::from_secs(max_secs) {
            break;
        }
        std::thread::sleep(Duration::from_millis(poll_ms));
    }
    Ok(())
}
