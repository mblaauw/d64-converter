use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use c64re_assets::{
    render_charset_grid_rgba, render_sprite_rgba, render_text_screen_rgba, write_png_rgba,
    CHARSET_BYTES, SCREEN_BYTES, SCREEN_HEIGHT_CHARS, SCREEN_WIDTH_CHARS, SPRITE_BYTES,
    SPRITE_HEIGHT, SPRITE_WIDTH,
};
use c64re_d64::D64Image;
use c64re_report::{blueprint_markdown, directory_json, disk_info_json, json_escape};
use c64re_trace::AnalysisSession;
use c64re_vic::VicState;
use c64re_vice_bmp::ViceMonitor;

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
    println!("  c64re analyze <game.d64> --out <dir> [--vice] [--seconds 5] [--sample-hz 10] [--autoplay] [--autostart-file NAME]");
    println!("  c64re vice-smoke [host:port]");
}

struct AnalyzeOptions {
    out: PathBuf,
    vice: bool,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    autostart_file: Option<String>,
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
        let boot_path = match &options.autostart_file {
            Some(wanted) => {
                let resolved = resolve_boot_path(out, &directory, wanted)?;
                session.notes.push(format!(
                    "Autostarted disk file '{wanted}' instead of the whole disk."
                ));
                resolved
            }
            None => path.to_string(),
        };
        let capture = capture_with_vice(
            &boot_path,
            &snapshots,
            options.seconds,
            options.sample_hz,
            options.autoplay,
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
}

#[derive(Debug, Clone)]
struct HardwareSample {
    index: usize,
    elapsed_ms: u128,
    pc: u16,
    vic: VicState,
    sid_registers: [u8; 25],
    sprite_pointers: [u8; 8],
}

#[derive(Debug, Clone)]
struct InputStep {
    start_ms: u128,
    end_ms: u128,
    port: u16,
    value: u16,
    label: &'static str,
}

#[derive(Debug, Clone)]
struct InputEvent {
    elapsed_ms: u128,
    port: u16,
    value: u16,
    label: String,
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
            "    \"input_event_count\": {}\n",
            capture.input_events.len()
        ));
    } else {
        out.push_str("    \"status\": \"not_run\",\n");
        out.push_str(
            "    \"reason\": \"run analyze with --vice to capture live emulator state\"\n",
        );
    }
    out.push_str("  },\n");
}

fn resolve_boot_path(
    out: &Path,
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
    let entry = matches[0];
    let safe_name = safe_filename(&entry.name);
    let relative_path = format!(
        "disk/files/{safe_name}.{}",
        entry.file_type.as_str().to_ascii_lowercase()
    );
    let path = out.join(&relative_path);
    if !path.exists() {
        return Err(format!("extracted file not found at {}", path.display()).into());
    }
    Ok(path.to_string_lossy().to_string())
}

fn capture_with_vice(
    disk_path: &str,
    snapshots: &Path,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    addr: &str,
) -> Result<ViceCapture, Box<dyn std::error::Error>> {
    let mut child = launch_vice(disk_path, addr)?;
    let result =
        capture_with_running_vice(&mut child, snapshots, seconds, sample_hz, autoplay, addr);

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

fn launch_vice(boot_path: &str, addr: &str) -> Result<Child, Box<dyn std::error::Error>> {
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
            "-controlport2device",
            "1",
            "-binarymonitor",
            "-binarymonitoraddress",
            &monitor_addr,
            boot_path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

fn capture_with_running_vice(
    child: &mut Child,
    snapshots: &Path,
    seconds: u64,
    sample_hz: u64,
    autoplay: bool,
    addr: &str,
) -> Result<ViceCapture, Box<dyn std::error::Error>> {
    let mut monitor = connect_with_retry(addr, Duration::from_secs(10))?;
    monitor.ping()?;
    let input_script = autoplay.then(|| default_autoplay_script(seconds));
    let mut current_joy2 = None;
    let mut input_events = Vec::new();
    if autoplay {
        apply_joyport(
            &mut monitor,
            0,
            2,
            0,
            "neutral",
            &mut current_joy2,
            &mut input_events,
        )?;
    }
    monitor.continue_run()?;

    let start = Instant::now();
    let run_for = Duration::from_secs(seconds);
    let sample_interval = sample_interval(sample_hz);
    let mut samples = Vec::new();
    while start.elapsed() < run_for {
        if let Some(status) = child.try_wait()? {
            return Err(format!("VICE exited early with status {status}").into());
        }
        thread::sleep(sample_interval.min(run_for.saturating_sub(start.elapsed())));
        if start.elapsed() <= run_for {
            let elapsed_ms = start.elapsed().as_millis();
            if let Some(script) = &input_script {
                let (port, value, label) = desired_joy_value(script, elapsed_ms);
                apply_joyport(
                    &mut monitor,
                    elapsed_ms,
                    port,
                    value,
                    label,
                    &mut current_joy2,
                    &mut input_events,
                )?;
            }
            samples.push(read_hardware_sample(
                &mut monitor,
                samples.len(),
                elapsed_ms,
            )?);
            monitor.continue_run()?;
        }
    }

    let registers = monitor.registers()?;
    let reset_vector = monitor.read_memory(0xfffc, 0xfffd)?;
    let ram = monitor.read_memory(0x0000, 0xffff)?;
    if ram.len() != 65_536 {
        return Err(format!("expected 65536 RAM bytes from VICE, got {}", ram.len()).into());
    }

    let snapshot_path = snapshots.join("vice-capture.ram");
    fs::write(&snapshot_path, &ram)?;
    let reset_vector = u16::from_le_bytes([
        reset_vector.first().copied().unwrap_or_default(),
        reset_vector.get(1).copied().unwrap_or_default(),
    ]);

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
    })
}

fn sample_interval(sample_hz: u64) -> Duration {
    if sample_hz == 0 {
        Duration::from_secs(1)
    } else {
        Duration::from_micros(1_000_000 / sample_hz)
    }
}

fn read_hardware_sample(
    monitor: &mut ViceMonitor,
    index: usize,
    elapsed_ms: u128,
) -> Result<HardwareSample, Box<dyn std::error::Error>> {
    let registers = monitor.registers()?;
    let vic_registers = monitor.read_memory(0xd000, 0xd02e)?;
    let bank_select = monitor
        .read_memory(0xdd00, 0xdd00)?
        .first()
        .copied()
        .unwrap_or_default();
    let sid_registers = fixed_25(&monitor.read_memory(0xd400, 0xd418)?);
    let vic = parse_vic_state(&vic_registers, bank_select);
    let sprite_pointers = fixed_8(&monitor.read_memory(
        vic.sprite_pointer_table(),
        vic.sprite_pointer_table().wrapping_add(7),
    )?);

    Ok(HardwareSample {
        index,
        elapsed_ms,
        pc: registers.pc,
        vic,
        sid_registers,
        sprite_pointers,
    })
}

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

    VicState {
        bank_select_dd00,
        memory_setup_d018: registers.get(0x18).copied().unwrap_or_default(),
        sprite_enable_d015: registers.get(0x15).copied().unwrap_or_default(),
        sprite_multicolor_d01c: registers.get(0x1c).copied().unwrap_or_default(),
        sprite_extra_x_d010: extra_x,
        background_color_d021: registers.get(0x21).copied().unwrap_or_default() & 0x0f,
        multicolor_0_d025: registers.get(0x25).copied().unwrap_or_default() & 0x0f,
        multicolor_1_d026: registers.get(0x26).copied().unwrap_or_default() & 0x0f,
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

fn default_autoplay_script(seconds: u64) -> Vec<InputStep> {
    let mut steps = Vec::new();
    let total_ms = u128::from(seconds) * 1000;
    let pattern = [
        ("fire", 0x10_u16, 500_u128),
        ("neutral", 0x00_u16, 500),
        ("right", 0x08_u16, 800),
        ("fire", 0x10_u16, 300),
        ("neutral", 0x00_u16, 400),
        ("left", 0x04_u16, 800),
        ("up", 0x01_u16, 500),
        ("down", 0x02_u16, 500),
        ("neutral", 0x00_u16, 700),
    ];

    let mut cursor = 1500_u128;
    while cursor < total_ms {
        for &(label, value, duration) in &pattern {
            if cursor >= total_ms {
                break;
            }
            let end = (cursor + duration).min(total_ms);
            steps.push(InputStep {
                start_ms: cursor,
                end_ms: end,
                port: 2,
                value,
                label,
            });
            cursor = end;
        }
    }

    steps
}

fn desired_joy_value(script: &[InputStep], elapsed_ms: u128) -> (u16, u16, &'static str) {
    script
        .iter()
        .find(|step| elapsed_ms >= step.start_ms && elapsed_ms < step.end_ms)
        .map(|step| (step.port, step.value, step.label))
        .unwrap_or((2, 0, "neutral"))
}

fn apply_joyport(
    monitor: &mut ViceMonitor,
    elapsed_ms: u128,
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
        elapsed_ms,
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
        out.push_str(&format!("    \"elapsed_ms\": {},\n", sample.elapsed_ms));
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
            "      \"screen_base\": {},\n",
            sample.vic.screen_base()
        ));
        out.push_str(&format!(
            "      \"charset_base\": {},\n",
            sample.vic.charset_base()
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
            "      \"background_color_d021\": {},\n",
            sample.vic.background_color_d021
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
            "    \"sid_registers_d400_d418\": {}\n",
            json_u8_array(&sample.sid_registers)
        ));
        out.push_str("  }");
    }
    out.push_str("\n]\n");
    out
}

fn hardware_samples_markdown(samples: &[HardwareSample]) -> String {
    let mut out = String::new();
    out.push_str("# Hardware Samples\n\n");
    out.push_str("Periodic VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample briefly stops the emulator, reads state, then resumes execution.\n\n");
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
    out.push_str("| # | ms | PC | D018 | screen | charset | sprites enabled | sprite pointers | bg | SID nonzero |\n");
    out.push_str("| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |\n");
    for sample in samples.iter().take(80) {
        out.push_str(&format!(
            "| {} | {} | {} | ${:02x} | {} | {} | {} | `{}` | ${:02x} | {} |\n",
            sample.index,
            sample.elapsed_ms,
            hex16(sample.pc),
            sample.vic.memory_setup_d018,
            hex16(sample.vic.screen_base()),
            hex16(sample.vic.charset_base()),
            sample.vic.sprite_enable_d015.count_ones(),
            hex_bytes(&sample.sprite_pointers),
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
        out.push_str(&format!("    \"elapsed_ms\": {},\n", event.elapsed_ms));
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
    out.push_str("Joystick events applied through VICE `JOYPORT_SET` during capture. Values use VICE's active-high joystick bitmask.\n\n");
    out.push_str(&format!("- Events: {}\n\n", events.len()));
    out.push_str("| ms | Port | Value | Label |\n");
    out.push_str("| ---: | ---: | ---: | --- |\n");
    for event in events {
        out.push_str(&format!(
            "| {} | {} | ${:02x} | {} |\n",
            event.elapsed_ms, event.port, event.value, event.label
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

    let mut seen_screens = BTreeSet::new();
    let mut seen_charsets = BTreeSet::new();
    let mut seen_sprites = BTreeSet::new();
    let mut screens = Vec::new();
    let mut charsets = Vec::new();
    let mut sprites = Vec::new();

    for sample in &capture.samples {
        let screen_address = sample.vic.screen_base();
        if seen_screens.insert(screen_address) {
            if let Some(screen) = ram_slice(&capture.ram, screen_address, SCREEN_BYTES) {
                let base = format!("screen-{}", hex_name(screen_address));
                let raw_path = screen_dir.join(format!("{base}.bin"));
                fs::write(&raw_path, screen)?;
                let mut preview_path = None;
                let note = if let Some(charset) =
                    ram_slice(&capture.ram, sample.vic.charset_base(), CHARSET_BYTES)
                {
                    if let Some(rgba) = render_text_screen_rgba(
                        screen,
                        charset,
                        sample.vic.background_color_d021,
                        1,
                    ) {
                        let path = screen_dir.join(format!("{base}.png"));
                        write_png_rgba(
                            &path,
                            (SCREEN_WIDTH_CHARS * 8) as u32,
                            (SCREEN_HEIGHT_CHARS * 8) as u32,
                            &rgba,
                        )?;
                        preview_path = Some(relative_asset_path(&path, assets_dir));
                    }
                    Some(format!(
                        "rendered with charset {}",
                        hex16(sample.vic.charset_base())
                    ))
                } else {
                    Some(format!(
                        "charset {} unavailable for preview",
                        hex16(sample.vic.charset_base())
                    ))
                };
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
        if seen_charsets.insert(charset_address) {
            if let Some(charset) = ram_slice(&capture.ram, charset_address, CHARSET_BYTES) {
                let base = format!("charset-{}", hex_name(charset_address));
                let raw_path = charset_dir.join(format!("{base}.bin"));
                fs::write(&raw_path, charset)?;
                let mut preview_path = None;
                if let Some(rgba) = render_charset_grid_rgba(charset) {
                    let path = charset_dir.join(format!("{base}.png"));
                    write_png_rgba(&path, 128, 128, &rgba)?;
                    preview_path = Some(relative_asset_path(&path, assets_dir));
                }
                charsets.push(AssetRecord {
                    kind: "charset",
                    address: charset_address,
                    sample_index: sample.index,
                    path: relative_asset_path(&raw_path, assets_dir),
                    preview_path,
                    note: None,
                });
            }
        }

        for sprite_index in 0..8 {
            if !sample.vic.sprite_enabled(sprite_index) {
                continue;
            }
            let pointer = sample.sprite_pointers[sprite_index];
            let sprite_address = sample
                .vic
                .vic_bank_base()
                .wrapping_add(u16::from(pointer) * SPRITE_BYTES as u16);
            if !seen_sprites.insert((sprite_address, sprite_index)) {
                continue;
            }
            if let Some(sprite) = ram_slice(&capture.ram, sprite_address, SPRITE_BYTES) {
                let base = format!("sprite-{}-s{}", hex_name(sprite_address), sprite_index);
                let raw_path = sprite_dir.join(format!("{base}.bin"));
                fs::write(&raw_path, sprite)?;
                let mut preview_path = None;
                if let Some(rgba) =
                    render_sprite_rgba(sprite, sample.vic.sprite_colors_d027_d02e[sprite_index])
                {
                    let path = sprite_dir.join(format!("{base}.png"));
                    write_png_rgba(&path, SPRITE_WIDTH as u32, SPRITE_HEIGHT as u32, &rgba)?;
                    preview_path = Some(relative_asset_path(&path, assets_dir));
                }
                let note = if sample.vic.sprite_multicolor_d01c & (1 << sprite_index) != 0 {
                    Some(
                        "sprite was displayed in multicolor mode; preview is monochrome fallback"
                            .to_string(),
                    )
                } else {
                    None
                };
                sprites.push(AssetRecord {
                    kind: "sprite",
                    address: sprite_address,
                    sample_index: sample.index,
                    path: relative_asset_path(&raw_path, assets_dir),
                    preview_path,
                    note,
                });
            }
        }
    }

    let summary = AssetExtractionSummary {
        manifest_path: "assets/manifest.json".to_string(),
        screen_count: screens.len(),
        charset_count: charsets.len(),
        sprite_count: sprites.len(),
        screens,
        charsets,
        sprites,
    };
    fs::write(
        assets_dir.join("manifest.json"),
        asset_manifest_json(&summary),
    )?;
    Ok(summary)
}

fn ram_slice(ram: &[u8], address: u16, len: usize) -> Option<&[u8]> {
    let start = usize::from(address);
    let end = start.checked_add(len)?;
    ram.get(start..end)
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

fn hex_bytes<const N: usize>(values: &[u8; N]) -> String {
    let mut out = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{value:02x}"));
    }
    out
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
