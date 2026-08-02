use c64re_d64::{DirectoryEntry, DiskInfo};
use c64re_trace::AnalysisSession;

pub fn blueprint_markdown(
    session: &AnalysisSession,
    disk_info: Option<&DiskInfo>,
    directory: &[DirectoryEntry],
) -> String {
    let counts = session.provenance.counts();
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
    out.push_str(&format!("- Captured frames: {}\n", session.frames.len()));
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

    out.push_str("## Current Findings\n\n");
    if session.notes.is_empty() {
        out.push_str("- Disk parsing completed. Emulator instrumentation has not run yet.\n");
    } else {
        for note in &session.notes {
            out.push_str(&format!("- {note}\n"));
        }
    }

    out.push_str("\n## Open Questions\n\n");
    out.push_str("- Which file is the boot entry point?\n");
    out.push_str("- Does the game use a cruncher, fastloader, or custom loader?\n");
    out.push_str("- Which joystick port and input patterns are active?\n");
    out.push_str("- Which memory ranges become stable after decrunching?\n");

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
