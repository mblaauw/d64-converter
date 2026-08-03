use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use c64re_capture::{capture_with_vice, resolve_file_name};
use c64re_d64::{load_prg_into_ram, safe_filename, D64Image, ExtractedFileMetadata};
use c64re_report::{
    blueprint_markdown, directory_json, disk_info_json, hardware_samples_json,
    hardware_samples_markdown, input_events_json, input_events_markdown, memory_map_markdown,
    open_questions_markdown, ram_diff_markdown, session_json, sid_writes_json, sid_writes_markdown,
    SessionFileInput,
};

#[derive(Parser)]
#[command(name = "c64re", version, about = "C64 reverse-engineering lab")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the disk directory.
    Disk {
        /// Path to the .d64 image.
        path: String,
    },
    /// Parse the disk; optionally capture live emulator state with --vice.
    Analyze(AnalyzeArgs),
    /// Check the VICE binary monitor connection.
    ViceSmoke {
        /// host:port of the VICE binary monitor.
        #[arg(default_value = "127.0.0.1:6502")]
        addr: String,
    },
}

#[derive(Args)]
struct AnalyzeArgs {
    /// Path to the .d64 image.
    path: String,
    /// Output directory.
    #[arg(long, default_value = "out/analysis")]
    out: PathBuf,
    /// Run the VICE capture pipeline.
    #[arg(long)]
    vice: bool,
    /// Capture duration in seconds of game time.
    #[arg(long, default_value_t = 5)]
    seconds: u64,
    /// Hardware samples per second of game time.
    #[arg(long, default_value_t = 10)]
    sample_hz: u64,
    /// Apply the default autoplay input script.
    #[arg(long)]
    autoplay: bool,
    /// Select a disk file by name (substring match).
    #[arg(long)]
    autostart_file: Option<String>,
    /// Harvest SID write activity for N seconds.
    #[arg(long, default_value_t = 0)]
    sid_seconds: u64,
    /// Autostart via VICE command line (for fastloader games).
    #[arg(long)]
    cmdline_autostart: bool,
    /// VICE binary monitor address.
    #[arg(long, default_value = "127.0.0.1:6502")]
    vice_addr: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Disk { path } => print_disk(&path)?,
        Command::Analyze(args) => analyze(&args)?,
        Command::ViceSmoke { addr } => vice_smoke(&addr)?,
    }
    Ok(())
}

fn vice_smoke(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = c64re_vice_bmp::ViceMonitor::connect(addr)?;
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

fn analyze(args: &AnalyzeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out = &args.out;
    let image = D64Image::open(&args.path)?;
    let disk_info = image.disk_info()?;
    let directory = image.directory()?;
    let disk = out.join("disk");
    let assets = out.join("assets");
    let reports = out.join("reports");
    let traces = out.join("traces");
    let extracted = disk.join("files");
    let snapshots = out.join("snapshots");
    for dir in [&disk, &assets, &reports, &traces, &extracted, &snapshots] {
        fs::create_dir_all(dir)?;
    }

    // VICE's drive emulation can write back to the disk image (hi-score
    // savers, BAM updates). Autostart a private copy so the source file is
    // never modified.
    let working_disk = disk.join("working.d64");
    fs::copy(&args.path, &working_disk)?;

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

    let mut session = c64re_trace::AnalysisSession::new(&args.path);
    session.notes.push(format!(
        "Extracted {} directory entries into `{}`.",
        directory.len(),
        extracted.display()
    ));
    fs::write(disk.join("info.json"), disk_info_json(&disk_info))?;
    fs::write(disk.join("directory.json"), directory_json(&directory))?;
    fs::write(snapshots.join("static-load.ram"), &static_ram)?;

    let vice_capture = if args.vice {
        let autostart_name = match &args.autostart_file {
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
            args.seconds,
            args.sample_hz,
            args.autoplay,
            args.sid_seconds,
            args.cmdline_autostart,
            &args.vice_addr,
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
        let asset_summary = c64re_assets::extract::extract_observed_assets(&assets, &capture)?;
        fs::write(
            reports.join("assets.md"),
            c64re_assets::extract::asset_summary_markdown(&asset_summary),
        )?;
        session.notes.push(format!(
            "Captured live VICE RAM after {} seconds into `{}`.",
            args.seconds, capture.ram_snapshot_path
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
            asset_summary.screen_count,
            asset_summary.charset_count,
            asset_summary.sprite_count
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

    let file_inputs: Vec<SessionFileInput> = extracted_files
        .iter()
        .map(|f| SessionFileInput {
            name: f.name.clone(),
            file_type: f.file_type.clone(),
            path: f.path.clone(),
            bytes: f.bytes,
            load_address: f.load_address,
            end_address_exclusive: f.end_address_exclusive,
            checksum16: f.checksum16,
            basic_sys: f.basic_sys,
        })
        .collect();
    fs::write(
        out.join("session.json"),
        session_json(&args.path, &file_inputs, vice_capture.as_ref()),
    )?;
    fs::write(
        reports.join("memory-map.md"),
        memory_map_markdown(&file_inputs),
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
