//! Step frames and log (pc, d018) every N frames during the load phase —
//! to find a reliable boot-script gate signal for IK+.
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
            return Err("stalled".into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_frames: u64 = 9000;
    let _log_every: u64 = 100;

    let monitor_addr = "ip4://127.0.0.1:6503";
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
            "/Users/mich/Downloads/International Karate Plus.d64",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut monitor = connect("127.0.0.1:6503")?;
        monitor.set_read_timeout(Duration::from_secs(10))?;
        let mut frame = 0_u64;
        while frame < max_frames {
            step_frames(&mut monitor, 100)?;
            frame += 100;
            let regs = monitor.registers_raw()?;
            let pc = regs
                .iter()
                .find(|r| r.id == 3)
                .map(|r| r.value)
                .unwrap_or(0);
            let d018 = monitor
                .read_memory(0xd018, 0xd018)?
                .first()
                .copied()
                .unwrap_or(0);
            println!("frame {frame}: pc=${pc:04x} d018=${d018:02x}");
        }
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
