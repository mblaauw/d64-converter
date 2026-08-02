#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ByteProvenance {
    pub executed: bool,
    pub cpu_read: bool,
    pub cpu_written: bool,
    pub vic_fetched: bool,
    pub sid_written: bool,
    pub write_then_execute: bool,
}

impl ByteProvenance {
    pub fn mark_executed(&mut self) {
        if self.cpu_written {
            self.write_then_execute = true;
        }
        self.executed = true;
    }

    pub fn mark_cpu_read(&mut self) {
        self.cpu_read = true;
    }

    pub fn mark_cpu_written(&mut self) {
        self.cpu_written = true;
    }

    pub fn mark_vic_fetched(&mut self) {
        self.vic_fetched = true;
    }

    pub fn mark_sid_written(&mut self) {
        self.sid_written = true;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceMap {
    bytes: Vec<ByteProvenance>,
}

impl ProvenanceMap {
    pub fn new(size: usize) -> Self {
        Self {
            bytes: vec![ByteProvenance::default(); size],
        }
    }

    pub fn c64_ram() -> Self {
        Self::new(65_536)
    }

    pub fn get(&self, address: u16) -> ByteProvenance {
        self.bytes[usize::from(address)]
    }

    pub fn get_mut(&mut self, address: u16) -> &mut ByteProvenance {
        &mut self.bytes[usize::from(address)]
    }

    pub fn counts(&self) -> ProvenanceCounts {
        self.bytes
            .iter()
            .fold(ProvenanceCounts::default(), |mut counts, byte| {
                counts.executed += usize::from(byte.executed);
                counts.cpu_read += usize::from(byte.cpu_read);
                counts.cpu_written += usize::from(byte.cpu_written);
                counts.vic_fetched += usize::from(byte.vic_fetched);
                counts.sid_written += usize::from(byte.sid_written);
                counts.write_then_execute += usize::from(byte.write_then_execute);
                counts
            })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProvenanceCounts {
    pub executed: usize,
    pub cpu_read: usize,
    pub cpu_written: usize,
    pub vic_fetched: usize,
    pub sid_written: usize,
    pub write_then_execute: usize,
}
