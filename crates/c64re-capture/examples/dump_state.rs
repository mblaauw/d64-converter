//! Dump the running VICE's RAM + a savestate so the pipeline can anchor to
//! gameplay (post-intro) instead of the crack intro.
use c64re_capture::connect;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out_prefix = args.next().unwrap_or_else(|| "out/live".into());
    let do_savestate = args.next().map(|s| s == "savestate").unwrap_or(false);

    let mut monitor = connect("127.0.0.1:6502")?;
    monitor.set_read_timeout(Duration::from_secs(10))?;
    let pc = monitor
        .registers_raw()?
        .iter()
        .find(|r| r.id == 3)
        .map(|r| r.value)
        .unwrap_or(0);
    println!("live pc=${pc:04x}");
    let ram = monitor.read_memory_in(c64re_vice_bmp::Memspace::Main, 0x0000, 0xffff, false, 1)?;
    std::fs::write(format!("{out_prefix}.ram"), &ram)?;
    println!("wrote {out_prefix}.ram ({} bytes)", ram.len());
    if do_savestate {
        monitor.dump(&format!("{out_prefix}.vsf"), false, false)?;
        println!("wrote {out_prefix}.vsf");
    }
    let _ = monitor.continue_run();
    Ok(())
}
