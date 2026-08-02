#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeInput {
    Idle { frames: u32 },
    HoldRight { frames: u32 },
    HoldLeft { frames: u32 },
    HoldUp { frames: u32 },
    HoldDown { frames: u32 },
    Fire { frames: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub name: String,
    pub setup_frames: u32,
    pub input: ProbeInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFinding {
    pub address: u16,
    pub role: String,
    pub confidence: u8,
    pub evidence: Vec<String>,
    pub written_by: Vec<u16>,
}

impl ProbeFinding {
    pub fn new(address: u16, role: impl Into<String>) -> Self {
        Self {
            address,
            role: role.into(),
            confidence: 0,
            evidence: Vec::new(),
            written_by: Vec::new(),
        }
    }
}

pub fn default_probe_library() -> Vec<ProbeDefinition> {
    vec![
        ProbeDefinition {
            name: "idle".to_string(),
            setup_frames: 30,
            input: ProbeInput::Idle { frames: 60 },
        },
        ProbeDefinition {
            name: "hold-right".to_string(),
            setup_frames: 30,
            input: ProbeInput::HoldRight { frames: 60 },
        },
        ProbeDefinition {
            name: "fire".to_string(),
            setup_frames: 30,
            input: ProbeInput::Fire { frames: 20 },
        },
    ]
}
