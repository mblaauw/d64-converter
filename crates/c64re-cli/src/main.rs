use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use c64re_assets::{
    render_charset_grid_rgba, render_hires_bitmap_rgba, render_multicolor_bitmap_rgba,
    render_multicolor_text_rgba, render_sprite_multicolor_rgba, render_sprite_rgba,
    render_text_screen_rgba, write_png_rgba, CHARSET_BYTES, SCREEN_BYTES, SCREEN_HEIGHT_CHARS,
    SCREEN_WIDTH_CHARS, SPRITE_BYTES, SPRITE_HEIGHT, SPRITE_WIDTH,
};
use c64re_d64::D64Image;
use c64re_report::{
    blueprint_markdown, directory_json, disk_info_json, json_escape, open_questions_markdown,
};
use c64re_trace::AnalysisSession;
use c64re_vic::{DisplayMode, VicState};
use c64re_vice_bmp::{Memspace, ViceMonitor};

/// VICE memory bank id for the "ram" bank (from BANKS_AVAILABLE).
const RAM_BANK_ID: u16 = 1;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "disk" => {
            let path = required_arg(args.next(), "missing .d64 path")?;
            print_disk(&path)?;
        }
        "analyze" => {
            let path = required_arg(args.next(), "missing .d64 path")?;
            let mut options = AnalyzeOptions::default();
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--out" => {
                        options.out =
                            PathBuf::from(required_arg(args.next(), "missing --out value")?)
                    }
                    "--vice" => options.vice = true,
                    "--seconds" => {
                        let value = required_arg(args.next(), "missing --seconds value")?;
                        options.seconds = value.parse()?;
                    }
                    "--vice-addr" => {
                        options.vice_addr = required_arg(args.next(), "missing --vice-addr value")?
                    }
                    "--sample-hz" => {
                        let value = required_arg(args.next(), "missing --sample-hz value")?;
                        options.sample_hz = value.parse()?;
                    }
                    "--autoplay" => options.autoplay = true,
                    "--autostart-file" => {
                        options.autostart_file =
                            Some(required_arg(args.next(), "missing --autostart-file value")?)
                    }
                    "--sid-seconds" => {
                        let value = required_arg(args.next(), "missing --sid-seconds value")?;
                        options.sid_seconds = value.parse()?;
                    }
                    "--cmdline-autostart" => options.cmdline_autostart = true,
                    unknown => return Err(format!("unknown argument: {unknown}").into()),
                }
            }
            analyze(&path, &options)?;
        }
        "vice-smoke" => {
            let addr = args.next().unwrap_or_else(|| "127.0.0.1:6502".to_string());
            vice_smoke(&addr)?;
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command: {other}").into()),
    }

    Ok(())
}

fn print_usage() {
    println!("c64re - C64 reverse-engineering lab");
    println!();
    println!("Usage:");
    println!("  c64re disk <game.d64>");
    println!("  c64re analyze <game.d64> --out <dir> [--vice] [--seconds 5] [--sample-hz 10] [--autoplay] [--autostart-file NAME] [--sid-seconds 3]");
    println!("  c64re vice-smoke [host:port]");
}

struct AnalyzeOptions {
    out: PathBuf,
    vice: bool,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    autostart_file: Option<String>,
    sid_seconds: u64,
    cmdline_autostart: bool,
    vice_addr: String,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            out: PathBuf::from("out/analysis"),
            vice: false,
            seconds: 5,
            sample_hz: 10,
            autoplay: false,
            autostart_file: None,
            sid_seconds: 0,
            cmdline_autostart: false,
            vice_addr: "127.0.0.1:6502".to_string(),
        }
    }
}

fn vice_smoke(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = ViceMonitor::connect(addr)?;
    monitor.ping()?;
    let registers = monitor.registers()?;
    let reset_vector = monitor.read_memory(0xfffc, 0xfffd)?;
    println!("connected to VICE binary monitor at {addr}");
    println!("pc=${:04x}", registers.pc);
    println!(
        "reset_vector=${:02x}{:02x}",
        reset_vector.get(1).copied().unwrap_or_default(),
        reset_vector.first().copied().unwrap_or_default()
    );
    Ok(())
}

fn print_disk(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image = D64Image::open(path)?;
    if let Ok(info) = image.disk_info() {
        println!("disk: {} / {} / {}", info.name, info.id, info.dos_type);
    }
    for entry in image.directory()? {
        println!(
            "{:>4} {:<3} {:>2}/{:<2} {}",
            entry.blocks,
            entry.file_type.as_str(),
            entry.first_track,
            entry.first_sector,
            entry.name
        );
    }
    Ok(())
}

fn analyze(path: &str, options: &AnalyzeOptions) -> Result<(), Box<dyn std::error::Error>> {
    let out = &options.out;
    let image = D64Image::open(path)?;
    let disk_info = image.disk_info()?;
    let directory = image.directory()?;
    let disk = out.join("disk");
    let assets = out.join("assets");
    let reports = out.join("reports");
    let traces = out.join("traces");
    let extracted = disk.join("files");
    let snapshots = out.join("snapshots");
    fs::create_dir_all(&disk)?;
    fs::create_dir_all(&assets)?;
    fs::create_dir_all(&reports)?;
    fs::create_dir_all(&traces)?;
    fs::create_dir_all(&extracted)?;
    fs::create_dir_all(&snapshots)?;

    // VICE's drive emulation can write back to the disk image (hi-score
    // savers, BAM updates). Autostart a private copy so the source file is
    // never modified; the copy is kept for reproducibility.
    let working_disk = out.join("disk").join("working.d64");
    fs::copy(path, &working_disk)?;

    let mut extracted_files = Vec::new();
    let mut static_ram = vec![0_u8; 65_536];
    for entry in &directory {
        let bytes = image.read_file(entry)?;
        let safe_name = safe_filename(&entry.name);
        let relative_path = format!(
            "disk/files/{safe_name}.{}",
            entry.file_type.as_str().to_ascii_lowercase()
        );
        fs::write(out.join(&relative_path), &bytes)?;
        let metadata = ExtractedFileMetadata::from_bytes(entry, relative_path, &bytes);
        if entry.file_type == c64re_d64::FileType::Prg {
            load_prg_into_ram(&mut static_ram, &bytes);
        }
        extracted_files.push(metadata);
    }

    let mut session = AnalysisSession::new(path);
    session.notes.push(format!(
        "Extracted {} directory entries into `{}`.",
        directory.len(),
        extracted.display()
    ));
    fs::write(disk.join("info.json"), disk_info_json(&disk_info))?;
    fs::write(disk.join("directory.json"), directory_json(&directory))?;
    fs::write(snapshots.join("static-load.ram"), &static_ram)?;
    let vice_capture = if options.vice {
        let autostart_name = match &options.autostart_file {
            Some(wanted) => {
                let resolved = resolve_file_name(&directory, wanted)?;
                session.notes.push(format!(
                    "Autostarted disk file '{wanted}' from the attached image."
                ));
                Some(resolved)
            }
            None => None,
        };
        let capture = capture_with_vice(
            working_disk.to_str().ok_or("invalid working disk path")?,
            autostart_name.as_deref(),
            &assets,
            &snapshots,
            options.seconds,
            options.sample_hz,
            options.autoplay,
            options.sid_seconds,
            options.cmdline_autostart,
            &options.vice_addr,
        )?;
        fs::write(
            reports.join("ram-diff.md"),
            ram_diff_markdown(&static_ram, &capture.ram),
        )?;
        fs::write(
            traces.join("hardware-samples.json"),
            hardware_samples_json(&capture.samples),
        )?;
        fs::write(
            reports.join("hardware-samples.md"),
            hardware_samples_markdown(&capture.samples),
        )?;
        if !capture.input_events.is_empty() {
            fs::write(
                traces.join("input-events.json"),
                input_events_json(&capture.input_events),
            )?;
            fs::write(
                reports.join("input-events.md"),
                input_events_markdown(&capture.input_events),
            )?;
        }
        if !capture.sid_writes.is_empty() {
            fs::write(
                traces.join("sid-writes.json"),
                sid_writes_json(&capture.sid_writes),
            )?;
            fs::write(
                reports.join("sid-writes.md"),
                sid_writes_markdown(&capture.sid_writes),
            )?;
        }
        let asset_summary = extract_observed_assets(&assets, &capture)?;
        fs::write(
            reports.join("assets.md"),
            asset_summary_markdown(&asset_summary),
        )?;
        session.notes.push(format!(
            "Captured live VICE RAM after {} seconds into `{}`.",
            options.seconds, capture.ram_snapshot_path
        ));
        session.notes.push(format!(
            "Collected {} hardware samples into `{}`.",
            capture.samples.len(),
            capture
                .hardware_samples_path
                .as_deref()
                .unwrap_or("traces/hardware-samples.json")
        ));
        session.notes.push(format!(
            "Extracted {} screen blocks, {} charsets, and {} displayed sprite blocks into `assets/`.",
            asset_summary.screen_count, asset_summary.charset_count, asset_summary.sprite_count
        ));
        match capture.game_start_frame {
            Some(frame) => session.notes.push(format!(
                "Game start (t0) detected at frame {frame}: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase."
            )),
            None => session.notes.push(
                "No game start (t0) detected: capture stayed in the KERNAL/loader phase for the whole run."
                    .to_string(),
            ),
        }
        if !capture.input_events.is_empty() {
            session.notes.push(format!(
                "Applied {} joystick input events from the default autoplay script.",
                capture.input_events.len()
            ));
        }
        Some(capture)
    } else {
        None
    };
    fs::write(
        out.join("session.json"),
        session_json(path, &extracted_files, vice_capture.as_ref()),
    )?;
    fs::write(
        reports.join("memory-map.md"),
        memory_map_markdown(&extracted_files),
    )?;
    fs::write(
        reports.join("blueprint.md"),
        blueprint_markdown(&session, Some(&disk_info), &directory),
    )?;
    fs::write(
        reports.join("open-questions.md"),
        open_questions_markdown(&session),
    )?;

    println!("wrote {}", reports.join("blueprint.md").display());
    Ok(())
}

#[derive(Debug, Clone)]
struct ViceCapture {
    address: String,
    seconds: u64,
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    status: u8,
    reset_vector: u16,
    ram_snapshot_path: String,
    ram_bytes: usize,
    ram: Vec<u8>,
    hardware_samples_path: Option<String>,
    samples: Vec<HardwareSample>,
    input_events_path: Option<String>,
    input_events: Vec<InputEvent>,
    game_start_frame: Option<u64>,
    sid_writes_path: Option<String>,
    sid_writes: Vec<SidWrite>,
}

#[derive(Debug, Clone)]
struct HardwareSample {
    index: usize,
    frame: u64,
    pc: u16,
    vic: VicState,
    sid_registers: [u8; 25],
    sprite_pointers: [u8; 8],
    color_ram: Vec<u8>,
    display_mode: DisplayMode,
    carved: CarvedSample,
}

/// Bytes carved from the emulator at observation time (while paused).
#[derive(Debug, Clone, Default)]
struct CarvedSample {
    screen: Option<Vec<u8>>,
    charset: Option<Vec<u8>>,
    charset_is_rom: bool,
    bitmap: Option<Vec<u8>>,
    sprites: [Option<Vec<u8>>; 8],
}

impl CarvedSample {
    /// Write the carved bytes as raw files next to the sample index.
    fn write_raw(&self, dir: &Path, index: usize) -> std::io::Result<()> {
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

#[derive(Debug, Clone)]
struct InputStep {
    start_frame: u64,
    end_frame: u64,
    port: u16,
    value: u16,
    label: &'static str,
}

#[derive(Debug, Clone)]
struct InputEvent {
    frame: u64,
    port: u16,
    value: u16,
    label: String,
}

/// Per-register SID write counts, harvested from non-stopping watchpoints.
#[derive(Debug, Clone)]
struct SidWrite {
    address: u16,
    write_count: u32,
}

struct ExtractedFileMetadata {
    name: String,
    file_type: String,
    path: String,
    bytes: usize,
    load_address: Option<u16>,
    end_address_exclusive: Option<u16>,
    checksum16: u16,
    basic_sys: Option<u16>,
}

impl ExtractedFileMetadata {
    fn from_bytes(entry: &c64re_d64::DirectoryEntry, path: String, bytes: &[u8]) -> Self {
        let load_address = (entry.file_type == c64re_d64::FileType::Prg && bytes.len() >= 2)
            .then(|| u16::from_le_bytes([bytes[0], bytes[1]]));
        let payload_len = bytes.len().saturating_sub(2);
        let end_address_exclusive =
            load_address.map(|address| address.wrapping_add(payload_len as u16));
        let checksum16 = bytes.iter().fold(0_u16, |checksum, &byte| {
            checksum.wrapping_add(u16::from(byte))
        });
        let basic_sys = load_address.and_then(|address| detect_basic_sys(address, bytes));

        Self {
            name: entry.name.clone(),
            file_type: entry.file_type.as_str().to_string(),
            path,
            bytes: bytes.len(),
            load_address,
            end_address_exclusive,
            checksum16,
            basic_sys,
        }
    }
}

fn session_json(
    source_path: &str,
    files: &[ExtractedFileMetadata],
    vice_capture: Option<&ViceCapture>,
) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(source_path)
    ));
    out.push_str("  \"analysis_version\": 1,\n");
    write_emulator_json(&mut out, vice_capture);
    out.push_str("  \"extracted_files\": [\n");
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&file.name)
        ));
        out.push_str(&format!("      \"file_type\": \"{}\",\n", file.file_type));
        out.push_str(&format!(
            "      \"path\": \"{}\",\n",
            json_escape(&file.path)
        ));
        out.push_str(&format!("      \"bytes\": {},\n", file.bytes));
        match file.load_address {
            Some(address) => out.push_str(&format!("      \"load_address\": {},\n", address)),
            None => out.push_str("      \"load_address\": null,\n"),
        }
        match file.end_address_exclusive {
            Some(address) => {
                out.push_str(&format!("      \"end_address_exclusive\": {},\n", address))
            }
            None => out.push_str("      \"end_address_exclusive\": null,\n"),
        }
        out.push_str(&format!("      \"checksum16\": {},\n", file.checksum16));
        match file.basic_sys {
            Some(address) => out.push_str(&format!("      \"basic_sys\": {}\n", address)),
            None => out.push_str("      \"basic_sys\": null\n"),
        }
        out.push_str("    }");
    }
    out.push_str("\n  ]\n");
    out.push_str("}\n");
    out
}

fn write_emulator_json(out: &mut String, vice_capture: Option<&ViceCapture>) {
    out.push_str("  \"emulator\": {\n");
    if let Some(capture) = vice_capture {
        out.push_str("    \"status\": \"captured\",\n");
        out.push_str("    \"engine\": \"VICE x64sc binary monitor\",\n");
        out.push_str(&format!(
            "    \"address\": \"{}\",\n",
            json_escape(&capture.address)
        ));
        out.push_str(&format!("    \"seconds\": {},\n", capture.seconds));
        out.push_str("    \"registers\": {\n");
        out.push_str(&format!("      \"pc\": {},\n", capture.pc));
        out.push_str(&format!("      \"a\": {},\n", capture.a));
        out.push_str(&format!("      \"x\": {},\n", capture.x));
        out.push_str(&format!("      \"y\": {},\n", capture.y));
        out.push_str(&format!("      \"sp\": {},\n", capture.sp));
        out.push_str(&format!("      \"status\": {}\n", capture.status));
        out.push_str("    },\n");
        out.push_str(&format!(
            "    \"reset_vector\": {},\n",
            capture.reset_vector
        ));
        out.push_str(&format!(
            "    \"ram_snapshot_path\": \"{}\",\n",
            json_escape(&capture.ram_snapshot_path)
        ));
        out.push_str(&format!("    \"ram_bytes\": {},\n", capture.ram_bytes));
        match &capture.hardware_samples_path {
            Some(path) => out.push_str(&format!(
                "    \"hardware_samples_path\": \"{}\",\n",
                json_escape(path)
            )),
            None => out.push_str("    \"hardware_samples_path\": null,\n"),
        }
        out.push_str(&format!(
            "    \"hardware_sample_count\": {}\n",
            capture.samples.len()
        ));
        out.pop();
        out.push_str(",\n");
        match &capture.input_events_path {
            Some(path) => out.push_str(&format!(
                "    \"input_events_path\": \"{}\",\n",
                json_escape(path)
            )),
            None => out.push_str("    \"input_events_path\": null,\n"),
        }
        out.push_str(&format!(
            "    \"input_event_count\": {},\n",
            capture.input_events.len()
        ));
        match capture.game_start_frame {
            Some(frame) => out.push_str(&format!("    \"game_start_frame\": {frame},\n")),
            None => out.push_str("    \"game_start_frame\": null,\n"),
        }
        match &capture.sid_writes_path {
            Some(path) => out.push_str(&format!(
                "    \"sid_writes_path\": \"{}\",\n",
                json_escape(path)
            )),
            None => out.push_str("    \"sid_writes_path\": null,\n"),
        }
        out.push_str(&format!(
            "    \"sid_write_count\": {}\n",
            capture.sid_writes.len()
        ));
    } else {
        out.push_str("    \"status\": \"not_run\",\n");
        out.push_str(
            "    \"reason\": \"run analyze with --vice to capture live emulator state\"\n",
        );
    }
    out.push_str("  },\n");
}

fn resolve_file_name(
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

#[allow(clippy::too_many_arguments)]
fn capture_with_vice(
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
fn launch_vice(addr: &str) -> Result<Child, Box<dyn std::error::Error>> {
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
                "capture stalled during load: VIC raster stopped advancing (CPU jam or emulator halt)".into(),
            );
        }
        frame += 1;
        if frame.is_multiple_of(500) {
            println!("loading... frame {frame} (t0 not yet detected)");
        }
        // t0 heuristic: the CPU has left ROM (running game code in RAM) and
        // the machine is in a stable state across several frames. The screen
        // base check alone is unreliable: some games keep the video matrix
        // at $0400 while switching charsets. (The IRQ vector is not checked
        // either: many games keep the KERNAL IRQ handler.)
        let d018 = monitor.read_memory(0xd018, 0xd018)?;
        let pc = monitor.registers()?.pc;
        let in_game =
            (0x0200..0xa000).contains(&pc) && d018.first().copied().unwrap_or_default() != 0x15;
        if in_game {
            game_screen_frames += 1;
            if game_screen_frames >= 30 {
                game_start_frame = Some(frame);
                println!(
                    "game start (t0) detected at frame {frame}: pc=${pc:04x} d018=${:02x}",
                    d018.first().copied().unwrap_or_default()
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
                "capture stalled during settle: VIC raster stopped advancing (CPU jam or emulator halt)".into(),
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
fn sample_every_frames(sample_hz: u64) -> u64 {
    if sample_hz == 0 {
        1
    } else {
        50u64.saturating_div(sample_hz).max(1)
    }
}

/// Read the current VIC raster line (LIN register, id 53) via the monitor.
fn read_raster_line(monitor: &mut ViceMonitor) -> Result<u16, Box<dyn std::error::Error>> {
    let registers = monitor.registers_raw()?;
    Ok(registers
        .iter()
        .find(|r| r.id == 53)
        .map(|r| r.value)
        .unwrap_or(0))
}

/// Decode the writing instruction from CPU history: a store to a SID
/// register is `STA/STX/STY $D4xx`, with the value in A/X/Y and the address
/// in the operand.
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
    std::thread::sleep(Duration::from_secs(seconds));
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
fn read_hardware_sample(
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

/// Character ROM is visible at $D000 in the VICE "rom" bank.
const ROM_CHARSET_ROM_ADDRESS: u16 = 0xd000;
/// VICE memory bank id for the "rom" bank (from BANKS_AVAILABLE).
const ROM_BANK_ID: u16 = 2;

fn parse_vic_state(registers: &[u8], bank_select_dd00: u8) -> VicState {
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
fn default_autoplay_script(seconds: u64) -> Vec<InputStep> {
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

fn desired_joy_value(script: &[InputStep], frame: u64) -> (u16, u16, &'static str) {
    script
        .iter()
        .find(|step| frame >= step.start_frame && frame < step.end_frame)
        .map(|step| (step.port, step.value, step.label))
        .unwrap_or((2, 0, "neutral"))
}

fn apply_joyport(
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

fn connect_with_retry(
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

fn memory_map_markdown(files: &[ExtractedFileMetadata]) -> String {
    let mut out = String::new();
    out.push_str("# Static Memory Map\n\n");
    out.push_str("This map is produced by loading extracted PRG files at their embedded load addresses. It is not yet an emulator-unpacked snapshot.\n\n");
    out.push_str("| File | Type | Load | End | Bytes | BASIC SYS |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: | ---: |\n");
    for file in files {
        let load = file
            .load_address
            .map(hex16)
            .unwrap_or_else(|| "-".to_string());
        let end = file
            .end_address_exclusive
            .map(hex16)
            .unwrap_or_else(|| "-".to_string());
        let basic_sys = file.basic_sys.map(hex16).unwrap_or_else(|| "-".to_string());
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            file.name, file.file_type, load, end, file.bytes, basic_sys
        ));
    }
    out
}

fn ram_diff_markdown(static_ram: &[u8], vice_ram: &[u8]) -> String {
    let ranges = changed_ranges(static_ram, vice_ram);
    let changed_bytes: usize = ranges.iter().map(|range| range.end - range.start + 1).sum();

    let mut out = String::new();
    out.push_str("# Static vs VICE RAM Diff\n\n");
    out.push_str("Compares `snapshots/static-load.ram` with `snapshots/vice-capture.ram`.\n\n");
    out.push_str(&format!("- Changed bytes: {}\n", changed_bytes));
    out.push_str(&format!("- Changed ranges: {}\n\n", ranges.len()));
    out.push_str("| Range | Bytes | Static first | VICE first |\n");
    out.push_str("| --- | ---: | ---: | ---: |\n");
    for range in ranges.iter().take(64) {
        out.push_str(&format!(
            "| {}-{} | {} | ${:02x} | ${:02x} |\n",
            hex16(range.start as u16),
            hex16(range.end as u16),
            range.end - range.start + 1,
            static_ram[range.start],
            vice_ram[range.start]
        ));
    }
    if ranges.len() > 64 {
        out.push_str(&format!(
            "\nOnly the first 64 ranges are shown. {} ranges omitted.\n",
            ranges.len() - 64
        ));
    }
    out
}

fn hardware_samples_json(samples: &[HardwareSample]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\n");
        out.push_str(&format!("    \"index\": {},\n", sample.index));
        out.push_str(&format!("    \"frame\": {},\n", sample.frame));
        out.push_str(&format!("    \"pc\": {},\n", sample.pc));
        out.push_str("    \"vic\": {\n");
        out.push_str(&format!(
            "      \"bank_select_dd00\": {},\n",
            sample.vic.bank_select_dd00
        ));
        out.push_str(&format!(
            "      \"memory_setup_d018\": {},\n",
            sample.vic.memory_setup_d018
        ));
        out.push_str(&format!(
            "      \"control_1_d011\": {},\n",
            sample.vic.control_1_d011
        ));
        out.push_str(&format!(
            "      \"control_2_d016\": {},\n",
            sample.vic.control_2_d016
        ));
        out.push_str(&format!(
            "      \"screen_base\": {},\n",
            sample.vic.screen_base()
        ));
        out.push_str(&format!(
            "      \"charset_base\": {},\n",
            sample.vic.charset_base()
        ));
        out.push_str(&format!(
            "      \"bitmap_base\": {},\n",
            sample.vic.bitmap_base()
        ));
        out.push_str(&format!(
            "      \"display_mode\": \"{}\",\n",
            sample.display_mode.as_str()
        ));
        out.push_str(&format!(
            "      \"sprite_pointer_table\": {},\n",
            sample.vic.sprite_pointer_table()
        ));
        out.push_str(&format!(
            "      \"sprite_enable_d015\": {},\n",
            sample.vic.sprite_enable_d015
        ));
        out.push_str(&format!(
            "      \"sprite_multicolor_d01c\": {},\n",
            sample.vic.sprite_multicolor_d01c
        ));
        out.push_str(&format!(
            "      \"sprite_y_expand_d017\": {},\n",
            sample.vic.sprite_y_expand_d017
        ));
        out.push_str(&format!(
            "      \"sprite_x_expand_d01d\": {},\n",
            sample.vic.sprite_x_expand_d01d
        ));
        out.push_str(&format!(
            "      \"sprite_priority_d01b\": {},\n",
            sample.vic.sprite_priority_d01b
        ));
        out.push_str(&format!(
            "      \"background_color_d021\": {},\n",
            sample.vic.background_color_d021
        ));
        out.push_str(&format!(
            "      \"background_1_d022\": {},\n",
            sample.vic.background_1_d022
        ));
        out.push_str(&format!(
            "      \"background_2_d023\": {},\n",
            sample.vic.background_2_d023
        ));
        out.push_str(&format!(
            "      \"background_3_d024\": {},\n",
            sample.vic.background_3_d024
        ));
        out.push_str(&format!(
            "      \"multicolor_0_d025\": {},\n",
            sample.vic.multicolor_0_d025
        ));
        out.push_str(&format!(
            "      \"multicolor_1_d026\": {},\n",
            sample.vic.multicolor_1_d026
        ));
        out.push_str(&format!(
            "      \"sprite_x\": {},\n",
            json_u16_array(&sample.vic.sprite_x)
        ));
        out.push_str(&format!(
            "      \"sprite_y\": {},\n",
            json_u8_array(&sample.vic.sprite_y)
        ));
        out.push_str(&format!(
            "      \"sprite_colors_d027_d02e\": {},\n",
            json_u8_array(&sample.vic.sprite_colors_d027_d02e)
        ));
        out.push_str(&format!(
            "      \"sprite_pointers\": {}\n",
            json_u8_array(&sample.sprite_pointers)
        ));
        out.push_str("    },\n");
        out.push_str(&format!(
            "    \"sid_registers_d400_d418\": {},\n",
            json_u8_array(&sample.sid_registers)
        ));
        out.push_str(&format!(
            "    \"color_ram_d800_dbe7\": [{}]\n",
            sample
                .color_ram
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str("  }");
    }
    out.push_str("\n]\n");
    out
}

fn hardware_samples_markdown(samples: &[HardwareSample]) -> String {
    let mut out = String::new();
    out.push_str("# Hardware Samples\n\n");
    out.push_str("Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.\n\n");
    out.push_str(&format!("- Samples: {}\n", samples.len()));
    if let Some(first) = samples.first() {
        out.push_str(&format!(
            "- First screen base: {}\n",
            hex16(first.vic.screen_base())
        ));
        out.push_str(&format!(
            "- First charset base: {}\n",
            hex16(first.vic.charset_base())
        ));
        out.push_str(&format!(
            "- First sprite pointer table: {}\n",
            hex16(first.vic.sprite_pointer_table())
        ));
    }
    out.push('\n');
    out.push_str(
        "| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |\n",
    );
    out.push_str("| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for sample in samples.iter().take(80) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | ${:02x} | {} | {} | {} | ${:02x} | {} |\n",
            sample.index,
            sample.frame,
            hex16(sample.pc),
            sample.display_mode.as_str(),
            sample.vic.memory_setup_d018,
            hex16(sample.vic.screen_base()),
            hex16(sample.vic.charset_base()),
            sample.vic.sprite_enable_d015.count_ones(),
            sample.vic.background_color_d021,
            sample
                .sid_registers
                .iter()
                .filter(|&&byte| byte != 0)
                .count()
        ));
    }
    if samples.len() > 80 {
        out.push_str(&format!(
            "\nOnly the first 80 samples are shown. {} samples omitted.\n",
            samples.len() - 80
        ));
    }
    out
}

fn input_events_json(events: &[InputEvent]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (index, event) in events.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\n");
        out.push_str(&format!("    \"frame\": {},\n", event.frame));
        out.push_str(&format!("    \"port\": {},\n", event.port));
        out.push_str(&format!("    \"value\": {},\n", event.value));
        out.push_str(&format!(
            "    \"label\": \"{}\"\n",
            json_escape(&event.label)
        ));
        out.push_str("  }");
    }
    out.push_str("\n]\n");
    out
}

fn input_events_markdown(events: &[InputEvent]) -> String {
    let mut out = String::new();
    out.push_str("# Input Events\n\n");
    out.push_str("Joystick events applied through VICE `JOYPORT_SET` during capture, scheduled by frame number (PAL, 50 frames/s). Values use VICE's active-high joystick bitmask.\n\n");
    out.push_str(&format!("- Events: {}\n\n", events.len()));
    out.push_str("| frame | Port | Value | Label |\n");
    out.push_str("| ---: | ---: | ---: | --- |\n");
    for event in events {
        out.push_str(&format!(
            "| {} | {} | ${:02x} | {} |\n",
            event.frame, event.port, event.value, event.label
        ));
    }
    out
}

fn sid_writes_json(writes: &[SidWrite]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (index, write) in writes.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\n");
        out.push_str(&format!("    \"address\": {},\n", write.address));
        out.push_str(&format!(
            "    \"address_hex\": \"{}\",\n",
            hex16(write.address)
        ));
        out.push_str(&format!("    \"write_count\": {}\n", write.write_count));
        out.push_str("  }");
    }
    out.push_str("\n]\n");
    out
}

fn sid_writes_markdown(writes: &[SidWrite]) -> String {
    let mut out = String::new();
    out.push_str("# SID Write Activity\n\n");
    out.push_str("SID registers ($D400-$D418) are write-only; reads return open-bus garbage. Per-register write counts were harvested from non-stopping write watchpoints during a replay.\n\n");
    out.push_str(&format!("- Registers written: {}\n\n", writes.len()));
    out.push_str("| Register | Write count |\n");
    out.push_str("| ---: | ---: |\n");
    for write in writes {
        out.push_str(&format!(
            "| {} | {} |\n",
            hex16(write.address),
            write.write_count
        ));
    }
    out
}

#[derive(Debug, Clone)]
struct AssetExtractionSummary {
    manifest_path: String,
    screen_count: usize,
    charset_count: usize,
    sprite_count: usize,
    screens: Vec<AssetRecord>,
    charsets: Vec<AssetRecord>,
    sprites: Vec<AssetRecord>,
}

#[derive(Debug, Clone)]
struct AssetRecord {
    kind: &'static str,
    address: u16,
    sample_index: usize,
    path: String,
    preview_path: Option<String>,
    note: Option<String>,
}

fn extract_observed_assets(
    assets_dir: &Path,
    capture: &ViceCapture,
) -> Result<AssetExtractionSummary, Box<dyn std::error::Error>> {
    let screen_dir = assets_dir.join("screens");
    let charset_dir = assets_dir.join("charsets");
    let sprite_dir = assets_dir.join("sprites");
    fs::create_dir_all(&screen_dir)?;
    fs::create_dir_all(&charset_dir)?;
    fs::create_dir_all(&sprite_dir)?;

    // Dedupe sprites by (address, content hash) so the same bytes carved from
    // multiple slots/frames collapse into one asset, with frame references.
    let mut seen_screens = BTreeSet::new();
    let mut seen_charsets = BTreeSet::new();
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
            // ROM charsets are shared; one asset for the ROM image.
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
                .wrapping_add(u16::from(pointer) * SPRITE_BYTES as u16);
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
fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone)]
struct SpriteAssetRecord {
    record: AssetRecord,
    frames: Vec<u64>,
    slots: Vec<usize>,
}

fn relative_asset_path(path: &Path, assets_dir: &Path) -> String {
    let relative = path.strip_prefix(assets_dir).unwrap_or(path);
    format!("assets/{}", relative.display())
}

fn asset_manifest_json(summary: &AssetExtractionSummary) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"manifest_path\": \"{}\",\n",
        json_escape(&summary.manifest_path)
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

fn asset_summary_markdown(summary: &AssetExtractionSummary) -> String {
    let mut out = String::new();
    out.push_str("# Observed Assets\n\n");
    out.push_str("Assets extracted from `snapshots/vice-capture.ram` using sampled VIC-II state as the source of truth.\n\n");
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

fn json_u8_array<const N: usize>(values: &[u8; N]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&value.to_string());
    }
    out.push(']');
    out
}

fn json_u16_array<const N: usize>(values: &[u16; N]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&value.to_string());
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_bytes_formats_space_separated() {
        let formatted: Vec<String> = [0x00_u8, 0xea, 0x31]
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect();
        assert_eq!(formatted.join(" "), "00 ea 31");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangedRange {
    start: usize,
    end: usize,
}

fn changed_ranges(left: &[u8], right: &[u8]) -> Vec<ChangedRange> {
    let len = left.len().min(right.len());
    let mut ranges = Vec::new();
    let mut start = None;

    for index in 0..len {
        if left[index] != right[index] {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(range_start) = start.take() {
            ranges.push(ChangedRange {
                start: range_start,
                end: index - 1,
            });
        }
    }

    if let Some(range_start) = start {
        ranges.push(ChangedRange {
            start: range_start,
            end: len - 1,
        });
    }

    ranges
}

fn load_prg_into_ram(ram: &mut [u8], bytes: &[u8]) {
    if bytes.len() < 2 {
        return;
    }
    let load_address = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
    let payload = &bytes[2..];
    let available = ram.len().saturating_sub(load_address);
    let len = payload.len().min(available);
    ram[load_address..load_address + len].copy_from_slice(&payload[..len]);
}

fn detect_basic_sys(load_address: u16, bytes: &[u8]) -> Option<u16> {
    if load_address != 0x0801 || bytes.len() < 6 {
        return None;
    }

    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        let next_line = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        if next_line == 0 {
            return None;
        }
        offset += 4;

        while offset < bytes.len() && bytes[offset] != 0 {
            if bytes[offset] == 0x9e {
                return parse_decimal_after_sys(&bytes[offset + 1..]);
            }
            offset += 1;
        }
        offset += 1;
    }

    None
}

fn parse_decimal_after_sys(bytes: &[u8]) -> Option<u16> {
    let mut value = 0_u16;
    let mut found_digit = false;
    for &byte in bytes {
        if byte == 0 || byte == b':' {
            break;
        }
        if byte.is_ascii_digit() {
            found_digit = true;
            value = value
                .saturating_mul(10)
                .saturating_add(u16::from(byte - b'0'));
        } else if found_digit {
            break;
        }
    }
    found_digit.then_some(value)
}

fn hex16(value: u16) -> String {
    format!("${value:04x}")
}

fn hex_name(value: u16) -> String {
    format!("{value:04x}")
}

fn safe_filename(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}

fn required_arg(
    value: Option<String>,
    message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    value.ok_or_else(|| message.to_string().into())
}
