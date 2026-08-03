//! Frame-stepped VICE capture: deterministic hardware sampling, input
//! scripting, savestate replay, and SID write harvesting.
//!
//! The capture model is documented in the workspace README:
//! autostart -> game-start detection (t0) -> settle -> Dump/Undump
//! savestate -> frame-stepped replay.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Bytes in one sprite block (24x21, 3 bytes per row).
pub const SPRITE_BYTES: usize = 64;
/// Bytes in one 2K character set.
pub const CHARSET_BYTES: usize = 2048;
/// Bytes in the 1000-byte video matrix.
pub const SCREEN_BYTES: usize = 1000;
use c64re_provenance::ProvenanceMap;
use c64re_vic::{DisplayMode, VicState};
use c64re_vice_bmp::{Memspace, ViceMonitor};

/// VICE memory bank id for the "ram" bank (from BANKS_AVAILABLE).
const RAM_BANK_ID: u16 = 1;
/// VICE memory bank id for the "rom" bank (from BANKS_AVAILABLE).
const ROM_BANK_ID: u16 = 2;
/// Character ROM is visible at $D000 in the VICE "rom" bank.
const ROM_CHARSET_ROM_ADDRESS: u16 = 0xd000;

/// Result of one instrumented VICE run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ViceCapture {
    pub address: String,
    pub seconds: u64,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
    pub reset_vector: u16,
    pub ram_snapshot_path: String,
    pub ram_bytes: usize,
    pub ram: Vec<u8>,
    pub hardware_samples_path: Option<String>,
    pub samples: Vec<HardwareSample>,
    pub input_events_path: Option<String>,
    pub input_events: Vec<InputEvent>,
    pub game_start_frame: Option<u64>,
    pub sid_writes_path: Option<String>,
    pub sid_writes: Vec<SidWrite>,
}

/// One hardware observation at a frame boundary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HardwareSample {
    pub index: usize,
    pub frame: u64,
    pub pc: u16,
    pub vic: VicState,
    pub sid_registers: [u8; 25],
    pub sprite_pointers: [u8; 8],
    pub color_ram: Vec<u8>,
    pub display_mode: DisplayMode,
    pub carved: CarvedSample,
}

/// Bytes carved from the emulator at observation time (while paused).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CarvedSample {
    pub screen: Option<Vec<u8>>,
    pub charset: Option<Vec<u8>>,
    pub charset_is_rom: bool,
    pub bitmap: Option<Vec<u8>>,
    pub sprites: [Option<Vec<u8>>; 8],
}

impl CarvedSample {
    /// Write the carved bytes as raw files next to the sample index.
    pub fn write_raw(&self, dir: &Path, index: usize) -> std::io::Result<()> {
        let prefix = format!("sample-{index:04}");
        if let Some(bytes) = &self.screen {
            fs::write(dir.join(format!("{prefix}-screen.bin")), bytes)?;
        }
        if let Some(bytes) = &self.charset {
            let name = if self.charset_is_rom {
                format!("{prefix}-charset-rom.bin")
            } else {
                format!("{prefix}-charset.bin")
            };
            fs::write(dir.join(name), bytes)?;
        }
        if let Some(bytes) = &self.bitmap {
            fs::write(dir.join(format!("{prefix}-bitmap.bin")), bytes)?;
        }
        for (slot, bytes) in self.sprites.iter().enumerate() {
            if let Some(bytes) = bytes {
                fs::write(dir.join(format!("{prefix}-sprite-s{slot}.bin")), bytes)?;
            }
        }
        Ok(())
    }
}

/// One step of the autoplay input script, scheduled by frame number.
#[derive(Debug, Clone)]
pub struct InputStep {
    pub start_frame: u64,
    pub end_frame: u64,
    pub port: u16,
    pub value: u16,
    pub label: &'static str,
}

/// A joystick event actually applied to VICE.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InputEvent {
    pub frame: u64,
    pub port: u16,
    pub value: u16,
    pub label: String,
}

/// Per-register SID write counts, harvested from non-stopping watchpoints.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SidWrite {
    pub address: u16,
    pub write_count: u32,
}

/// Run the full capture pipeline: launch VICE, autostart, find game start,
/// settle, savestate-replay, sample hardware, harvest SID writes.
#[allow(clippy::too_many_arguments)]
pub fn capture_with_vice(
    disk_path: &str,
    autostart_name: Option<&str>,
    assets: &Path,
    snapshots: &Path,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    sid_seconds: u64,
    cmdline_autostart: bool,
    addr: &str,
) -> Result<ViceCapture, Box<dyn std::error::Error>> {
    let mut child = if cmdline_autostart {
        launch_vice_with_disk(disk_path, addr)?
    } else {
        launch_vice(addr)?
    };
    let result = capture_with_running_vice(
        &mut child,
        assets,
        snapshots,
        seconds,
        sample_hz,
        autoplay,
        disk_path,
        autostart_name,
        sid_seconds,
        cmdline_autostart,
        addr,
    );

    if result.is_ok() {
        if let Ok(mut monitor) = ViceMonitor::connect(addr) {
            let _ = monitor.quit();
        }
        let _ = child.wait();
    } else {
        let _ = child.kill();
        let _ = child.wait();
    }

    result
}

/// Launch VICE bare (no disk) with the binary monitor. The machine is then
/// power-cycled and the game autostarted through the monitor, giving a
/// canonical, deterministic start point. No warp: VICE's drive emulation is
/// only cycle-deterministic without warp, which the savestate replay relies
/// on.
pub fn launch_vice(addr: &str) -> Result<Child, Box<dyn std::error::Error>> {
    let monitor_addr = if addr.contains("://") {
        addr.to_string()
    } else {
        format!("ip4://{addr}")
    };

    Ok(Command::new("x64sc")
        .args([
            "-default",
            "-silent",
            "-drive8type",
            "1541",
            "-controlport2device",
            "1",
            "-binarymonitor",
            "-binarymonitoraddress",
            &monitor_addr,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

/// Launch VICE with the disk on the command line: VICE's own autostart loads
/// and runs the game, which some fastloaders (e.g. `.FLT`) require. Used with
/// `--cmdline-autostart`.
fn launch_vice_with_disk(disk_path: &str, addr: &str) -> Result<Child, Box<dyn std::error::Error>> {
    let monitor_addr = if addr.contains("://") {
        addr.to_string()
    } else {
        format!("ip4://{addr}")
    };

    Ok(Command::new("x64sc")
        .args([
            "-default",
            "-warp",
            "-silent",
            "-drive8type",
            "1541",
            "-controlport2device",
            "1",
            "-binarymonitor",
            "-binarymonitoraddress",
            &monitor_addr,
            disk_path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

#[allow(clippy::too_many_arguments)]
fn capture_with_running_vice(
    child: &mut Child,
    assets: &Path,
    snapshots: &Path,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    disk_path: &str,
    autostart_name: Option<&str>,
    sid_seconds: u64,
    cmdline_autostart: bool,
    addr: &str,
) -> Result<ViceCapture, Box<dyn std::error::Error>> {
    let mut monitor = connect_with_retry(addr, Duration::from_secs(10))?;
    monitor.ping()?;
    monitor.set_read_timeout(Duration::from_secs(10))?;

    if cmdline_autostart {
        // VICE already autostarted the disk on the command line and is
        // running the game (this is the path for fastloader games). Skip the
        // power-cycle + monitor autostart.
        println!(
            "connected to cmdline-autostarted VICE at pc=${:04x}",
            monitor.registers()?.pc
        );
    } else {
        // Canonical start: power-cycle the bare machine, then autostart the
        // disk through the monitor. The load phase (drive I/O) is not
        // cycle-deterministic, so we step loosely until the game enters its
        // own video mode (t0), then dump a savestate and replay it for a
        // deterministic capture.
        monitor.power_cycle()?;
        monitor.drain_events();
        let autostart_target = match autostart_name {
            Some(name) => format!("{disk_path}:{name}"),
            None => disk_path.to_string(),
        };
        monitor.autostart(&autostart_target, true, 0)?;
        let sync = monitor.wait_for_stop()?;
        println!(
            "autostart sync at pc=${:04x} ({sync:?})",
            monitor.registers()?.pc
        );
    }
    let mut frame = 0_u64;

    let mut game_start_frame: Option<u64> = None;
    let mut game_screen_frames = 0_u64;
    let mut prev_raster = read_raster_line(&mut monitor)?;
    while game_start_frame.is_none() {
        if let Some(status) = child.try_wait()? {
            return Err(format!("VICE exited early with status {status}").into());
        }
        // Raster increases monotonically within a PAL frame (0..311), so any
        // decrease marks a frame boundary. 2500 instructions ≈ half a frame.
        // If the CPU jams (illegal opcode), the raster stalls: bail out.
        let mut wrapped = false;
        for _ in 0..16 {
            monitor.step_instructions(2500, false)?;
            let raster = read_raster_line(&mut monitor)?;
            wrapped = raster < prev_raster;
            prev_raster = raster;
            if wrapped {
                break;
            }
        }
        if !wrapped {
            return Err(
                "capture stalled during load: VIC raster stopped advancing (CPU jam or emulator halt)"
                    .into(),
            );
        }
        frame += 1;
        if frame.is_multiple_of(500) {
            println!("loading... frame {frame} (t0 not yet detected)");
        }
        // t0 heuristic: the game has taken over the VIC, stable across
        // several frames. Signal: the charset base is no longer the KERNAL
        // default ($1000, d018 bits 1-3 = 2) or the boot state ($0000, = 0),
        // OR the screen base left the KERNAL default ($0400). This accepts
        // custom-charset games (Le Mans d018=$1F), bitmap modes, and games
        // that keep the video matrix at $0400, while rejecting boot and the
        // KERNAL ready screen. The PC is deliberately not required to be in
        // RAM: games idling in a KERNAL call (e.g. CHRIN waiting for a key)
        // park in ROM while their video mode is already active.
        let d018 = monitor.read_memory(0xd018, 0xd018)?;
        let pc = monitor.registers()?.pc;
        let value = d018.first().copied().unwrap_or_default();
        let charset_bits = (value >> 1) & 0x07;
        let screen_bits = (value >> 4) & 0x0f;
        // Boot and KERNAL states: charset $0000/$1000, screen $0000/$0400.
        let custom_charset = !matches!(charset_bits, 0 | 2);
        let moved_screen = !matches!(screen_bits, 0 | 1);
        let in_game = custom_charset || moved_screen;
        if in_game {
            game_screen_frames += 1;
            if game_screen_frames >= 30 {
                game_start_frame = Some(frame);
                println!(
                    "game start (t0) detected at frame {frame}: pc=${pc:04x} d018=${value:02x}"
                );
            }
        } else {
            game_screen_frames = 0;
        }
    }

    // Deterministic capture: step a settle period past t0 (so the game
    // finishes any startup disk I/O and the drive goes idle), then dump the
    // machine state and replay it for the actual capture. Frame stepping is
    // deterministic without warp and with the drive idle; the drive's I/O
    // phases are inherently non-deterministic, so the load and settle are
    // captured loosely and the dump is the deterministic anchor.
    const SETTLE_FRAMES: u64 = 900; // 18 s of game time
    println!("settling for {SETTLE_FRAMES} frames after t0...");
    let settle_target = frame + SETTLE_FRAMES;
    while frame < settle_target {
        let mut wrapped = false;
        for _ in 0..16 {
            monitor.step_instructions(2500, false)?;
            let raster = read_raster_line(&mut monitor)?;
            wrapped = raster < prev_raster;
            prev_raster = raster;
            if wrapped {
                break;
            }
        }
        if !wrapped {
            return Err(
                "capture stalled during settle: VIC raster stopped advancing (CPU jam or emulator halt)"
                    .into(),
            );
        }
        frame += 1;
        if frame.is_multiple_of(300) {
            println!("settling... frame {frame}/{settle_target}");
        }
    }
    let t0_frame = game_start_frame.unwrap_or(frame);
    let savestate = snapshots.join("t0.vsf");
    monitor.dump(savestate.to_str().unwrap_or("t0.vsf"), false, false)?;
    monitor.undump(savestate.to_str().unwrap_or("t0.vsf"))?;
    frame = 0;
    println!(
        "savestate at frame {t0_frame} (+{SETTLE_FRAMES} settle); capture replay starts at frame 0"
    );

    let input_script = autoplay.then(|| default_autoplay_script(seconds));
    let mut current_joy2 = None;
    let mut input_events = Vec::new();
    if autoplay {
        apply_joyport(
            &mut monitor,
            frame,
            2,
            0,
            "neutral",
            &mut current_joy2,
            &mut input_events,
        )?;
    }

    let total_frames = seconds * 50; // PAL
    let sample_every = sample_every_frames(sample_hz);
    let mut samples = Vec::new();
    prev_raster = read_raster_line(&mut monitor)?;
    while frame < total_frames {
        if let Some(status) = child.try_wait()? {
            return Err(format!("VICE exited early with status {status}").into());
        }
        // Advance in instruction chunks until the raster line wraps.
        let mut wrapped = false;
        for _ in 0..16 {
            monitor.step_instructions(2500, false)?;
            let raster = read_raster_line(&mut monitor)?;
            wrapped = raster < prev_raster;
            prev_raster = raster;
            if wrapped {
                break;
            }
        }
        if !wrapped {
            return Err(
                "capture stalled: VIC raster stopped advancing (CPU jam or emulator halt)".into(),
            );
        }
        frame += 1;
        if frame.is_multiple_of(500) {
            println!("captured frame {frame}/{}", total_frames);
        }

        if let Some(script) = &input_script {
            let (port, value, label) = desired_joy_value(script, frame);
            apply_joyport(
                &mut monitor,
                frame,
                port,
                value,
                label,
                &mut current_joy2,
                &mut input_events,
            )?;
        }
        if frame.is_multiple_of(sample_every) {
            let sample =
                read_hardware_sample(&mut monitor, samples.len(), frame, &assets.join("raw"))?;
            samples.push(sample);
        }
    }

    let registers = monitor.registers()?;
    let reset_vector = monitor.read_memory(0xfffc, 0xfffd)?;
    // True RAM view (bank "ram"): the CPU view returns KERNAL/BASIC ROM and
    // I/O where banked in, losing RAM under those areas.
    let ram = monitor.read_memory_in(Memspace::Main, 0x0000, 0xffff, false, RAM_BANK_ID)?;
    if ram.len() != 65_536 {
        return Err(format!("expected 65536 RAM bytes from VICE, got {}", ram.len()).into());
    }

    let snapshot_path = snapshots.join("vice-capture.ram");
    fs::write(&snapshot_path, &ram)?;
    let reset_vector = u16::from_le_bytes([
        reset_vector.first().copied().unwrap_or_default(),
        reset_vector.get(1).copied().unwrap_or_default(),
    ]);

    // SID write harvest: SID registers are write-only; reads return open-bus
    // garbage. Set a write watchpoint, free-run the replay, and decode the
    // writing instruction from CPU history to get (frame, address, value).
    let sid_writes = harvest_sid_writes(&mut monitor, &savestate, sid_seconds)?;

    Ok(ViceCapture {
        address: addr.to_string(),
        seconds,
        pc: registers.pc,
        a: registers.a,
        x: registers.x,
        y: registers.y,
        sp: registers.sp,
        status: registers.status,
        reset_vector,
        ram_snapshot_path: "snapshots/vice-capture.ram".to_string(),
        ram_bytes: ram.len(),
        ram,
        hardware_samples_path: Some("traces/hardware-samples.json".to_string()),
        samples,
        input_events_path: (!input_events.is_empty())
            .then(|| "traces/input-events.json".to_string()),
        input_events,
        game_start_frame,
        sid_writes_path: (!sid_writes.is_empty()).then(|| "traces/sid-writes.json".to_string()),
        sid_writes,
    })
}

/// Frames between hardware samples (PAL runs 50 IRQ frames per second).
pub fn sample_every_frames(sample_hz: u64) -> u64 {
    if sample_hz == 0 {
        1
    } else {
        50u64.saturating_div(sample_hz).max(1)
    }
}

/// Read the current VIC raster line (LIN register, id 53) via the monitor.
pub fn read_raster_line(monitor: &mut ViceMonitor) -> Result<u16, Box<dyn std::error::Error>> {
    let registers = monitor.registers_raw()?;
    Ok(registers
        .iter()
        .find(|r| r.id == 53)
        .map(|r| r.value)
        .unwrap_or(0))
}

/// Harvest SID register write activity via non-stopping write watchpoints.
/// SID registers ($D400-$D418) are write-only: reads return open-bus garbage,
/// so per-register write counts are the only reliable evidence. One
/// non-stopping watchpoint per register, free-run for `seconds`, then read
/// the hit counts.
fn harvest_sid_writes(
    monitor: &mut ViceMonitor,
    savestate: &Path,
    seconds: u64,
) -> Result<Vec<SidWrite>, Box<dyn std::error::Error>> {
    if seconds == 0 {
        return Ok(Vec::new());
    }
    monitor.undump(savestate.to_str().unwrap_or("t0.vsf"))?;
    let mut watchpoints = Vec::new();
    for register in 0xd400..=0xd418 {
        let cp = monitor.watchpoint_nostop(register, register, c64re_vice_bmp::WatchMode::Write)?;
        watchpoints.push((register, cp));
    }
    println!("harvesting SID write activity for {seconds}s (25 non-stopping watchpoints)...");
    monitor.continue_run()?;
    thread::sleep(Duration::from_secs(seconds));
    let mut writes = Vec::new();
    for (register, cp) in watchpoints {
        if let Ok(info) = monitor.checkpoint_get(cp) {
            if info.hit_count > 0 {
                writes.push(SidWrite {
                    address: register,
                    write_count: info.hit_count,
                });
            }
        }
        let _ = monitor.checkpoint_delete(cp);
    }
    writes.sort_by_key(|w| w.address);
    println!(
        "harvested SID write activity: {} registers touched",
        writes.len()
    );
    Ok(writes)
}

/// Sample the hardware state at a frame boundary, carving the displayed
/// bytes at observation time (while paused) into `carve_dir`.
pub fn read_hardware_sample(
    monitor: &mut ViceMonitor,
    index: usize,
    frame: u64,
    carve_dir: &Path,
) -> Result<HardwareSample, Box<dyn std::error::Error>> {
    let registers = monitor.registers()?;
    let vic_registers = monitor.read_memory(0xd000, 0xd02e)?;
    let bank_select = monitor
        .read_memory(0xdd00, 0xdd00)?
        .first()
        .copied()
        .unwrap_or_default();
    let sid_registers = fixed_25(&monitor.read_memory(0xd400, 0xd418)?);
    let vic = parse_vic_state(&vic_registers, bank_select & 0x03);
    let sprite_pointers = fixed_8(&monitor.read_memory(
        vic.sprite_pointer_table(),
        vic.sprite_pointer_table().wrapping_add(7),
    )?);
    // Color RAM $D800-$DBE7 (1000 bytes) accompanies the video matrix.
    let color_ram = monitor.read_memory(0xd800, 0xdbe7)?;

    // Carve the displayed bytes at observation time: read them while the
    // emulator is paused, never from a later snapshot.
    let mut carved = CarvedSample::default();
    if let Some(screen) = read_bank_bytes(monitor, vic.screen_base(), SCREEN_BYTES)? {
        carved.screen = Some(screen);
    }
    let charset_source = if vic.is_rom_charset() {
        // Character ROM lives in the VICE "rom" bank at $D000.
        read_rom_bank_bytes(monitor, ROM_CHARSET_ROM_ADDRESS, CHARSET_BYTES)?
    } else {
        read_bank_bytes(monitor, vic.charset_base(), CHARSET_BYTES)?
    };
    if let Some(charset) = charset_source {
        carved.charset = Some(charset);
        carved.charset_is_rom = vic.is_rom_charset();
    }
    if vic.uses_bitmap() {
        carved.bitmap = read_bank_bytes(monitor, vic.bitmap_base(), 8000)?;
    }
    for (sprite_index, &pointer) in sprite_pointers.iter().enumerate() {
        if !vic.sprite_enabled(sprite_index) {
            continue;
        }
        let sprite_address = vic
            .vic_bank_base()
            .wrapping_add(u16::from(pointer) * SPRITE_BYTES as u16);
        if let Some(bytes) = read_bank_bytes(monitor, sprite_address, SPRITE_BYTES)? {
            carved.sprites[sprite_index] = Some(bytes);
        }
    }
    fs::create_dir_all(carve_dir)?;
    carved.write_raw(carve_dir, index)?;

    Ok(HardwareSample {
        index,
        frame,
        pc: registers.pc,
        vic,
        sid_registers,
        sprite_pointers,
        color_ram,
        display_mode: vic.display_mode(),
        carved,
    })
}

/// Read bytes from the machine's current bank view (RAM where present).
fn read_bank_bytes(
    monitor: &mut ViceMonitor,
    address: u16,
    len: usize,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if len == 0 {
        return Ok(None);
    }
    let end = address.checked_add(len as u16 - 1);
    let Some(end) = end else {
        return Ok(None);
    };
    Ok(Some(monitor.read_memory(address, end)?))
}

/// Read bytes from the VICE "rom" bank (id 2), used for character ROM.
fn read_rom_bank_bytes(
    monitor: &mut ViceMonitor,
    address: u16,
    len: usize,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if len == 0 {
        return Ok(None);
    }
    let end = address.checked_add(len as u16 - 1);
    let Some(end) = end else {
        return Ok(None);
    };
    Ok(Some(monitor.read_memory_in(
        Memspace::Main,
        address,
        end,
        false,
        ROM_BANK_ID,
    )?))
}

/// Decode VIC-II registers ($D000-$D02E) into a `VicState`.
pub fn parse_vic_state(registers: &[u8], bank_select_dd00: u8) -> VicState {
    let mut sprite_x = [0_u16; 8];
    let mut sprite_y = [0_u8; 8];
    let extra_x = registers.get(0x10).copied().unwrap_or_default();
    for index in 0..8 {
        let x_low = registers.get(index * 2).copied().unwrap_or_default();
        let x_high = u16::from((extra_x >> index) & 0x01) << 8;
        sprite_x[index] = x_high | u16::from(x_low);
        sprite_y[index] = registers.get(index * 2 + 1).copied().unwrap_or_default();
    }

    let mut sprite_colors = [0_u8; 8];
    for (index, color) in sprite_colors.iter_mut().enumerate() {
        *color = registers.get(0x27 + index).copied().unwrap_or_default() & 0x0f;
    }

    let reg = |index: usize| registers.get(index).copied().unwrap_or_default();
    VicState {
        bank_select_dd00,
        memory_setup_d018: reg(0x18),
        control_1_d011: reg(0x11),
        control_2_d016: reg(0x16),
        sprite_enable_d015: reg(0x15),
        sprite_multicolor_d01c: reg(0x1c),
        sprite_y_expand_d017: reg(0x17),
        sprite_x_expand_d01d: reg(0x1d),
        sprite_priority_d01b: reg(0x1b),
        sprite_extra_x_d010: extra_x,
        background_color_d021: reg(0x21) & 0x0f,
        background_1_d022: reg(0x22) & 0x0f,
        background_2_d023: reg(0x23) & 0x0f,
        background_3_d024: reg(0x24) & 0x0f,
        multicolor_0_d025: reg(0x25) & 0x0f,
        multicolor_1_d026: reg(0x26) & 0x0f,
        sprite_colors_d027_d02e: sprite_colors,
        sprite_x,
        sprite_y,
    }
}

fn fixed_8(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0_u8; 8];
    let len = bytes.len().min(out.len());
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn fixed_25(bytes: &[u8]) -> [u8; 25] {
    let mut out = [0_u8; 25];
    let len = bytes.len().min(out.len());
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// Default autoplay script, scheduled in frames at PAL rate (50 frames/s).
pub fn default_autoplay_script(seconds: u64) -> Vec<InputStep> {
    let mut steps = Vec::new();
    let total_frames = seconds * 50;
    let pattern = [
        ("fire", 0x10_u16, 25_u64),
        ("neutral", 0x00_u16, 25),
        ("right", 0x08_u16, 40),
        ("fire", 0x10_u16, 15),
        ("neutral", 0x00_u16, 20),
        ("left", 0x04_u16, 40),
        ("up", 0x01_u16, 25),
        ("down", 0x02_u16, 25),
        ("neutral", 0x00_u16, 35),
    ];

    let mut cursor = 75_u64;
    while cursor < total_frames {
        for &(label, value, duration) in &pattern {
            if cursor >= total_frames {
                break;
            }
            let end = (cursor + duration).min(total_frames);
            steps.push(InputStep {
                start_frame: cursor,
                end_frame: end,
                port: 2,
                value,
                label,
            });
            cursor = end;
        }
    }

    steps
}

pub fn desired_joy_value(script: &[InputStep], frame: u64) -> (u16, u16, &'static str) {
    script
        .iter()
        .find(|step| frame >= step.start_frame && frame < step.end_frame)
        .map(|step| (step.port, step.value, step.label))
        .unwrap_or((2, 0, "neutral"))
}

pub fn apply_joyport(
    monitor: &mut ViceMonitor,
    frame: u64,
    port: u16,
    value: u16,
    label: &str,
    current_joy2: &mut Option<u16>,
    input_events: &mut Vec<InputEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    if *current_joy2 == Some(value) {
        return Ok(());
    }

    let applied_port = match monitor.joyport_set(port, value) {
        Ok(()) => port,
        Err(err) if port == 2 => {
            let _ = err;
            monitor.joyport_set(1, value)?;
            1
        }
        Err(err) => return Err(Box::new(err)),
    };
    *current_joy2 = Some(value);
    input_events.push(InputEvent {
        frame,
        port: applied_port,
        value,
        label: label.to_string(),
    });
    Ok(())
}

/// Connect to VICE with a 10-second retry window.
pub fn connect(addr: &str) -> Result<ViceMonitor, Box<dyn std::error::Error>> {
    connect_with_retry(addr, Duration::from_secs(10))
}

pub fn connect_with_retry(
    addr: &str,
    timeout: Duration,
) -> Result<ViceMonitor, Box<dyn std::error::Error>> {
    let start = Instant::now();
    loop {
        match ViceMonitor::connect(addr) {
            Ok(monitor) => return Ok(monitor),
            Err(err) if start.elapsed() < timeout => {
                let _ = err;
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(Box::new(err)),
        }
    }
}

/// Resolve a file name on the disk by case-insensitive substring match.
pub fn resolve_file_name(
    directory: &[c64re_d64::DirectoryEntry],
    wanted: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let wanted_lower = wanted.to_ascii_lowercase();
    let matches: Vec<&c64re_d64::DirectoryEntry> = directory
        .iter()
        .filter(|entry| entry.name.to_ascii_lowercase().contains(&wanted_lower))
        .collect();
    if matches.is_empty() {
        return Err(
            format!("--autostart-file '{wanted}' did not match any file on the disk").into(),
        );
    }
    if matches.len() > 1 {
        let names: Vec<&str> = matches.iter().map(|entry| entry.name.as_str()).collect();
        return Err(format!(
            "--autostart-file '{wanted}' is ambiguous (matches: {})",
            names.join(", ")
        )
        .into());
    }
    Ok(matches[0].name.clone())
}

/// Convenience re-export so callers can build output paths.
pub fn output_paths(out: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    (
        out.join("assets"),
        out.join("reports"),
        out.join("traces"),
        out.join("snapshots"),
    )
}

/// Replay the savestate for `frames` and harvest executed PC addresses from
/// CpuHistory into a provenance map. This is an approximate coverage map:
/// sampling every N frames misses short-lived code, but the main loops and
/// handlers show up. When `autoplay` is set, the default input script runs
/// during the replay so the capture includes input-driven code.
pub fn collect_provenance(
    monitor: &mut ViceMonitor,
    savestate: &Path,
    frames: u64,
    sample_every: u64,
    autoplay: bool,
) -> Result<ProvenanceMap, Box<dyn std::error::Error>> {
    monitor.undump(savestate.to_str().unwrap_or("t0.vsf"))?;
    let mut provenance = ProvenanceMap::c64_ram();
    let input_script = autoplay.then(|| default_autoplay_script(frames / 50));
    let mut current_joy2 = None;
    let mut input_events = Vec::new();
    let mut prev_raster = read_raster_line(monitor)?;
    let mut frame = 0_u64;
    while frame < frames {
        let mut wrapped = false;
        for _ in 0..16 {
            monitor.step_instructions(2500, false)?;
            let raster = read_raster_line(monitor)?;
            wrapped = raster < prev_raster;
            prev_raster = raster;
            if wrapped {
                break;
            }
        }
        if !wrapped {
            return Err(
                "provenance replay stalled: VIC raster stopped advancing (CPU jam or emulator halt)"
                    .into(),
            );
        }
        frame += 1;
        if let Some(script) = &input_script {
            let (port, value, label) = desired_joy_value(script, frame);
            apply_joyport(
                monitor,
                frame,
                port,
                value,
                label,
                &mut current_joy2,
                &mut input_events,
            )?;
        }
        if frame.is_multiple_of(sample_every) {
            // CpuHistory returns the most recent instructions; mark those PCs
            // as executed. We request a small window to bound the cost.
            if let Ok(history) = monitor.cpu_history(64) {
                for entry in &history {
                    provenance.get_mut(entry.pc).mark_executed();
                }
            }
        }
    }
    Ok(provenance)
}
