#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineHook {
    pub entry: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwinDiff {
    pub address: u16,
    pub original: u8,
    pub replacement: u8,
}

pub trait C64Core {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn step_frame(&mut self);
}

pub trait NativeRoutine<C: C64Core> {
    fn hook(&self) -> RoutineHook;
    fn run(&mut self, core: &mut C);
}

pub fn diff_ram(original: &[u8], replacement: &[u8]) -> Vec<TwinDiff> {
    original
        .iter()
        .zip(replacement.iter())
        .enumerate()
        .filter_map(|(address, (&left, &right))| {
            (left != right).then_some(TwinDiff {
                address: address as u16,
                original: left,
                replacement: right,
            })
        })
        .collect()
}
