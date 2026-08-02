use c64re_provenance::ProvenanceMap;
use c64re_sid::SidWrite;
use c64re_vic::VicState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryAccessKind {
    CpuRead,
    CpuWrite,
    Execute,
    VicFetch,
    SidWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEvent {
    pub frame: u64,
    pub cycle: u64,
    pub pc: Option<u16>,
    pub address: u16,
    pub value: Option<u8>,
    pub kind: MemoryAccessKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTrace {
    pub frame: u64,
    pub pc_samples: Vec<u16>,
    pub vic: VicState,
    pub sid_writes: Vec<SidWrite>,
    pub memory_events: Vec<MemoryEvent>,
}

#[derive(Debug, Clone)]
pub struct AnalysisSession {
    pub source_path: String,
    pub frames: Vec<FrameTrace>,
    pub ram_snapshot: Option<Vec<u8>>,
    pub provenance: ProvenanceMap,
    pub notes: Vec<String>,
}

impl AnalysisSession {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            frames: Vec::new(),
            ram_snapshot: None,
            provenance: ProvenanceMap::c64_ram(),
            notes: Vec::new(),
        }
    }
}
