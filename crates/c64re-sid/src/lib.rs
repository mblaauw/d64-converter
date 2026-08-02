#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidWrite {
    pub frame: u64,
    pub cycle: u64,
    pub address: u16,
    pub value: u8,
}

impl SidWrite {
    pub fn new(frame: u64, cycle: u64, address: u16, value: u8) -> Self {
        Self {
            frame,
            cycle,
            address,
            value,
        }
    }

    pub fn register_index(self) -> Option<u8> {
        (0xd400..=0xd418)
            .contains(&self.address)
            .then_some((self.address - 0xd400) as u8)
    }
}
