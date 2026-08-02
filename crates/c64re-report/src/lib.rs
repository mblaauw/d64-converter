use c64re_d64::{DirectoryEntry, DiskInfo};
use c64re_trace::AnalysisSession;

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
