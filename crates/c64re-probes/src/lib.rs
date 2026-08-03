//! Probe experiments: run a controlled input against a savestate, capture
//! RAM, and compare across probes to find input-sensitive memory.

use std::path::Path;
use std::time::Duration;

use c64re_capture::{apply_joyport, desired_joy_value, read_raster_line, InputStep};
use c64re_vice_bmp::{Memspace, ViceMonitor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeInput {
    Idle { frames: u32 },
    HoldRight { frames: u32 },
    HoldLeft { frames: u32 },
    HoldUp { frames: u32 },
    HoldDown { frames: u32 },
    Fire { frames: u32 },
}

impl ProbeInput {
    /// Convert to a single InputStep spanning the whole probe window.
    fn to_step(&self) -> InputStep {
        let (value, label) = match self {
            Self::Idle { .. } => (0x00, "neutral"),
            Self::HoldRight { .. } => (0x08, "right"),
            Self::HoldLeft { .. } => (0x04, "left"),
            Self::HoldUp { .. } => (0x01, "up"),
            Self::HoldDown { .. } => (0x02, "down"),
            Self::Fire { .. } => (0x10, "fire"),
        };
        InputStep {
            start_frame: 0,
            end_frame: u64::MAX,
            port: 2,
            value,
            label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub name: String,
    pub setup_frames: u32,
    pub input: ProbeInput,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeRun {
    pub name: String,
    pub frames: u64,
    pub ram_path: String,
    pub changed_bytes: usize,
}

/// One probe run: restore the savestate, apply the scripted input for the
/// probe window, and dump RAM to a file.
pub fn run_probe(
    monitor: &mut ViceMonitor,
    savestate: &Path,
    definition: &ProbeDefinition,
    probe_frames: u64,
    ram_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    monitor.undump(savestate.to_str().unwrap_or("t0.vsf"))?;
    monitor.set_read_timeout(Duration::from_secs(10))?;

    // Setup phase: advance without input. Bounded by wall clock so a JAM
    // cannot hang the run.
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    let mut frame = 0_u64;
    let mut prev_raster = read_raster_line(monitor)?;
    while frame < u64::from(definition.setup_frames) {
        if std::time::Instant::now() > deadline {
            return Err("probe setup timed out (emulator jam?)".into());
        }
        advance_one_frame(monitor, &mut prev_raster)?;
        frame += 1;
    }

    // Probe phase: hold the input.
    let step = definition.input.clone().to_step();
    let mut current_joy2 = None;
    let mut input_events = Vec::new();
    apply_joyport(
        monitor,
        frame,
        step.port,
        step.value,
        step.label,
        &mut current_joy2,
        &mut input_events,
    )?;
    while frame < u64::from(definition.setup_frames) + probe_frames {
        if std::time::Instant::now() > deadline {
            return Err("probe timed out (emulator jam?)".into());
        }
        advance_one_frame(monitor, &mut prev_raster)?;
        frame += 1;
        let (port, value, label) = desired_joy_value(std::slice::from_ref(&step), frame);
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

    let ram = monitor.read_memory_in(Memspace::Main, 0x0000, 0xffff, false, 1)?;
    std::fs::write(ram_path, &ram)?;
    Ok(ram)
}

fn advance_one_frame(
    monitor: &mut ViceMonitor,
    prev_raster: &mut u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wrapped = false;
    for _ in 0..16 {
        monitor.step_instructions(2500, false)?;
        let raster = read_raster_line(monitor)?;
        wrapped = raster < *prev_raster;
        *prev_raster = raster;
        if wrapped {
            break;
        }
    }
    if !wrapped {
        return Err(
            "probe stalled: VIC raster stopped advancing (CPU jam or emulator halt)".into(),
        );
    }
    Ok(())
}

/// A finding: a memory range whose content differs between probes, which is
/// therefore input-sensitive (candidates for game state).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeFinding {
    pub start: u16,
    pub end: u16,
    pub role: String,
    pub confidence: u8,
    pub changed_in: Vec<String>,
    pub baseline_first: String,
}

/// Diff a probe RAM against the baseline and produce findings for ranges
/// that differ. Skips pure I/O mirrors ($D000-$DFFF) and the stack.
pub fn diff_against_baseline(baseline: &[u8], probes: &[(String, Vec<u8>)]) -> Vec<ProbeFinding> {
    let mut findings: Vec<ProbeFinding> = Vec::new();
    for (name, ram) in probes {
        let ranges = changed_ranges(baseline, ram);
        for (start, end) in ranges {
            if start >= 0xd000 || (0x0100..=0x01ff).contains(&start) {
                continue;
            }
            let existing = findings
                .iter_mut()
                .find(|f| f.start == start as u16 && f.end == end as u16);
            match existing {
                Some(finding) => finding.changed_in.push(name.clone()),
                None => findings.push(ProbeFinding {
                    start: start as u16,
                    end: end as u16,
                    role: "input-sensitive".to_string(),
                    confidence: 40,
                    changed_in: vec![name.clone()],
                    baseline_first: format!("${:02x}", baseline.get(start).copied().unwrap_or(0)),
                }),
            }
        }
    }
    findings.sort_by_key(|f| f.start);
    findings
}

fn changed_ranges(left: &[u8], right: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut current: Option<(usize, usize)> = None;
    let limit = left.len().min(right.len());
    for index in 0..limit {
        if left[index] != right[index] {
            match &mut current {
                Some((_, end)) => *end = index,
                None => current = Some((index, index)),
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

/// The standard probe library: an idle baseline plus directional/fire probes.
pub fn default_probe_library() -> Vec<ProbeDefinition> {
    vec![
        ProbeDefinition {
            name: "hold-right".to_string(),
            setup_frames: 30,
            input: ProbeInput::HoldRight { frames: 60 },
        },
        ProbeDefinition {
            name: "hold-left".to_string(),
            setup_frames: 30,
            input: ProbeInput::HoldLeft { frames: 60 },
        },
        ProbeDefinition {
            name: "hold-up".to_string(),
            setup_frames: 30,
            input: ProbeInput::HoldUp { frames: 60 },
        },
        ProbeDefinition {
            name: "hold-down".to_string(),
            setup_frames: 30,
            input: ProbeInput::HoldDown { frames: 60 },
        },
        ProbeDefinition {
            name: "fire".to_string(),
            setup_frames: 30,
            input: ProbeInput::Fire { frames: 20 },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs_against_baseline() {
        let baseline = vec![0_u8; 0x500];
        let mut ram = baseline.clone();
        ram[0x0400] = 0x42;
        ram[0x0401] = 0x43;
        let findings = diff_against_baseline(&baseline, &[("probe".to_string(), ram)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].start, 0x0400);
        assert_eq!(findings[0].end, 0x0401);
    }

    #[test]
    fn skips_io_and_stack() {
        let baseline = vec![0_u8; 0xe000];
        let mut ram = baseline.clone();
        ram[0xd020] = 0xff; // I/O
        ram[0x0150] = 0xff; // stack
        ram[0x2000] = 0xff; // real RAM
        let findings = diff_against_baseline(&baseline, &[("p".to_string(), ram)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].start, 0x2000);
    }
}
