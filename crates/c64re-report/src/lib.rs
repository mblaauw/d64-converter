//! Markdown and JSON report writers.
//!
//! JSON output is produced with serde so the schema is round-trippable.

use c64re_capture::HardwareSample;
pub use c64re_capture::{InputEvent, SidWrite, ViceCapture};
use c64re_d64::{DirectoryEntry, DiskInfo};
use c64re_trace::AnalysisSession;
use serde::Serialize;

pub fn blueprint_markdown(
    session: &AnalysisSession,
    disk_info: Option<&DiskInfo>,
    directory: &[DirectoryEntry],
) -> String {
    let counts = session.provenance.counts();
    let has_provenance = counts.executed > 0
        || counts.cpu_read > 0
        || counts.cpu_written > 0
        || counts.vic_fetched > 0
        || counts.sid_written > 0
        || counts.write_then_execute > 0;
    let mut out = String::new();
    out.push_str("# C64 Reverse-Engineering Blueprint\n\n");
    out.push_str("## Source\n\n");
    out.push_str(&format!("- Disk image: `{}`\n", session.source_path));
    if let Some(info) = disk_info {
        out.push_str(&format!("- Disk title: `{}`\n", info.name));
        out.push_str(&format!("- Disk ID: `{}`\n", info.id));
        out.push_str(&format!("- DOS type: `{}`\n", info.dos_type));
    }
    out.push_str(&format!("- Directory entries: {}\n", directory.len()));
    if !session.frames.is_empty() {
        out.push_str(&format!("- Captured frames: {}\n", session.frames.len()));
    } else {
        out.push_str("- Captured frames: none (frame-stepped capture not yet run)\n");
    }
    out.push('\n');

    out.push_str("## Disk Directory\n\n");
    if directory.is_empty() {
        out.push_str("No directory entries found.\n\n");
    } else {
        out.push_str("| Blocks | Type | First T/S | Name |\n");
        out.push_str("| ---: | --- | --- | --- |\n");
        for entry in directory {
            out.push_str(&format!(
                "| {} | {} | {}/{} | `{}` |\n",
                entry.blocks,
                entry.file_type.as_str(),
                entry.first_track,
                entry.first_sector,
                entry.name
            ));
        }
        out.push('\n');
    }

    if has_provenance {
        out.push_str("## Provenance Summary\n\n");
        out.push_str(&format!("- Executed bytes: {}\n", counts.executed));
        out.push_str(&format!("- CPU-read bytes: {}\n", counts.cpu_read));
        out.push_str(&format!("- CPU-written bytes: {}\n", counts.cpu_written));
        out.push_str(&format!("- VIC-fetched bytes: {}\n", counts.vic_fetched));
        out.push_str(&format!("- SID-written bytes: {}\n", counts.sid_written));
        out.push_str(&format!(
            "- Write-then-execute bytes: {}\n\n",
            counts.write_then_execute
        ));
    } else {
        out.push_str("## Provenance\n\n");
        out.push_str(
            "- Not collected yet: provenance requires an instrumented capture (see T13 work items).\n\n",
        );
    }

    out.push_str("## Current Findings\n\n");
    if session.notes.is_empty() {
        out.push_str("- Disk parsing completed. Emulator instrumentation has not run yet.\n");
    } else {
        for note in &session.notes {
            out.push_str(&format!("- {note}\n"));
        }
    }

    out.push_str(
        "\n## Open Questions\n\nSee `open-questions.md` for the open questions and the specific evidence needed to close each one.\n\n",
    );

    out
}

pub fn open_questions_markdown(session: &AnalysisSession) -> String {
    let mut out = String::new();
    out.push_str("# Open Questions\n\n");
    out.push_str("Each question lists the specific evidence needed to close it.\n\n");
    out.push_str("| # | Question | Evidence needed |\n");
    out.push_str("| ---: | --- | --- |\n");
    out.push_str("| 1 | Which file is the boot entry point? | Directory `file_index` of the autostarted file; first executed PC after autostart; `CpuHistory` PC list. |\n");
    out.push_str("| 2 | Does the game use a cruncher, fastloader, or custom loader? | Bytes executed before the first screen change; `$D018`/`$D011` deltas between boot and `t0`; IRQ vector writes. |\n");
    out.push_str("| 3 | Which joystick port and input patterns are active? | Frame-numbered input log; `$DC00`/`$DC01` reads at `t0`; response of game state to probe inputs. |\n");
    out.push_str("| 4 | Which memory ranges become stable after decrunching? | RAM diff between two runs at the same frame; per-frame write-watchpoint log. |\n");
    if session.frames.is_empty() {
        out.push_str("\nNo frames captured in this session; none of the above can be answered from this run alone.\n");
    } else {
        out.push_str(&format!(
            "\nSession captured {} frame(s); evidence above should be collected in a dedicated probe run.\n",
            session.frames.len()
        ));
    }
    out
}

pub fn directory_json(directory: &[DirectoryEntry]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (index, entry) in directory.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\n");
        out.push_str(&format!(
            "    \"name\": \"{}\",\n",
            json_escape(&entry.name)
        ));
        out.push_str(&format!(
            "    \"file_type\": \"{}\",\n",
            entry.file_type.as_str()
        ));
        out.push_str(&format!("    \"closed\": {},\n", entry.closed));
        out.push_str(&format!("    \"locked\": {},\n", entry.locked));
        out.push_str(&format!("    \"first_track\": {},\n", entry.first_track));
        out.push_str(&format!("    \"first_sector\": {},\n", entry.first_sector));
        out.push_str(&format!("    \"blocks\": {}\n", entry.blocks));
        out.push_str("  }");
    }
    out.push_str("\n]\n");
    out
}

pub fn disk_info_json(info: &DiskInfo) -> String {
    format!(
        "{{\n  \"name\": \"{}\",\n  \"id\": \"{}\",\n  \"dos_type\": \"{}\"\n}}\n",
        json_escape(&info.name),
        json_escape(&info.id),
        json_escape(&info.dos_type)
    )
}

pub fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SessionFile {
    name: String,
    file_type: String,
    path: String,
    bytes: usize,
    load_address: Option<u16>,
    end_address_exclusive: Option<u16>,
    checksum16: u16,
    basic_sys: Option<u16>,
}

#[derive(Serialize)]
struct EmulatorInfo<'a> {
    status: &'static str,
    engine: &'static str,
    address: &'a str,
    seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    registers: Option<Registers>,
    reset_vector: u16,
    ram_snapshot_path: &'a str,
    ram_bytes: usize,
    hardware_samples_path: Option<&'a str>,
    hardware_sample_count: usize,
    input_events_path: Option<&'a str>,
    input_event_count: usize,
    game_start_frame: Option<u64>,
    sid_writes_path: Option<&'a str>,
    sid_write_count: usize,
}

#[derive(Serialize)]
struct Registers {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    status: u8,
}

#[derive(Serialize)]
struct Session<'a> {
    source_path: &'a str,
    analysis_version: u32,
    emulator: EmulatorInfo<'a>,
    extracted_files: &'a [SessionFile],
}

pub fn session_json(
    source_path: &str,
    files: &[SessionFileInput],
    vice_capture: Option<&ViceCapture>,
) -> String {
    let session_files: Vec<SessionFile> = files
        .iter()
        .map(|f| SessionFile {
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
    let emulator = match vice_capture {
        Some(c) => EmulatorInfo {
            status: "captured",
            engine: "VICE x64sc binary monitor",
            address: &c.address,
            seconds: c.seconds,
            registers: Some(Registers {
                pc: c.pc,
                a: c.a,
                x: c.x,
                y: c.y,
                sp: c.sp,
                status: c.status,
            }),
            reset_vector: c.reset_vector,
            ram_snapshot_path: &c.ram_snapshot_path,
            ram_bytes: c.ram_bytes,
            hardware_samples_path: c.hardware_samples_path.as_deref(),
            hardware_sample_count: c.samples.len(),
            input_events_path: c.input_events_path.as_deref(),
            input_event_count: c.input_events.len(),
            game_start_frame: c.game_start_frame,
            sid_writes_path: c.sid_writes_path.as_deref(),
            sid_write_count: c.sid_writes.len(),
        },
        None => EmulatorInfo {
            status: "not_run",
            engine: "VICE x64sc binary monitor",
            address: "",
            seconds: 0,
            registers: None,
            reset_vector: 0,
            ram_snapshot_path: "",
            ram_bytes: 0,
            hardware_samples_path: None,
            hardware_sample_count: 0,
            input_events_path: None,
            input_event_count: 0,
            game_start_frame: None,
            sid_writes_path: None,
            sid_write_count: 0,
        },
    };
    let session = Session {
        source_path,
        analysis_version: 1,
        emulator,
        extracted_files: &session_files,
    };
    serde_json::to_string_pretty(&session).unwrap_or_else(|_| "{}".to_string())
}

/// Input metadata for `session_json` (the CLI's extracted-file summary).
#[derive(Debug, Clone)]
pub struct SessionFileInput {
    pub name: String,
    pub file_type: String,
    pub path: String,
    pub bytes: usize,
    pub load_address: Option<u16>,
    pub end_address_exclusive: Option<u16>,
    pub checksum16: u16,
    pub basic_sys: Option<u16>,
}

// ---------------------------------------------------------------------------
// Memory map / RAM diff
// ---------------------------------------------------------------------------

pub fn memory_map_markdown(files: &[SessionFileInput]) -> String {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangedRange {
    start: usize,
    end: usize,
}

pub fn ram_diff_markdown(static_ram: &[u8], vice_ram: &[u8]) -> String {
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

fn changed_ranges(left: &[u8], right: &[u8]) -> Vec<ChangedRange> {
    let mut ranges = Vec::new();
    let mut current: Option<ChangedRange> = None;
    let limit = left.len().min(right.len());
    for index in 0..limit {
        if left[index] != right[index] {
            match &mut current {
                Some(range) => range.end = index,
                None => {
                    current = Some(ChangedRange {
                        start: index,
                        end: index,
                    })
                }
            }
        } else if let Some(range) = current.take() {
            ranges.push(range);
        }
    }
    if let Some(range) = current.take() {
        ranges.push(range);
    }
    ranges
}

// ---------------------------------------------------------------------------
// Hardware samples
// ---------------------------------------------------------------------------

pub fn hardware_samples_json(samples: &[HardwareSample]) -> String {
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

pub fn hardware_samples_markdown(samples: &[HardwareSample]) -> String {
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

// ---------------------------------------------------------------------------
// Input events
// ---------------------------------------------------------------------------

pub fn input_events_json(events: &[InputEvent]) -> String {
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

pub fn input_events_markdown(events: &[InputEvent]) -> String {
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

// ---------------------------------------------------------------------------
// SID writes
// ---------------------------------------------------------------------------

pub fn sid_writes_json(writes: &[SidWrite]) -> String {
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

pub fn sid_writes_markdown(writes: &[SidWrite]) -> String {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn json_u8_array<const N: usize>(values: &[u8; N]) -> String {
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

pub fn json_u16_array<const N: usize>(values: &[u16; N]) -> String {
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

pub fn hex16(value: u16) -> String {
    format!("${value:04x}")
}

pub fn hex_name(value: u16) -> String {
    format!("{value:04x}")
}
