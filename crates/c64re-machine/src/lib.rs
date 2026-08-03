//! A minimal C64 machine: memory map with ROM banking, a
//! provenance-recording bus, and helpers to load the system ROMs.
//!
//! This is the embedded-core milestone: it runs the real 6502 core against
//! a C64 memory map and records per-byte provenance (executed / read /
//! written / write-then-execute) natively — no emulator needed.

use c64re_cpu::{Bus, Cpu6502};
use c64re_provenance::ProvenanceMap;
use std::fs;
use std::path::Path;

/// Size of the full C64 address space.
pub const MEM_SIZE: usize = 0x10000;
/// BASIC ROM base ($A000), 8K.
pub const BASIC_BASE: usize = 0xa000;
/// KERNAL ROM base ($E000), 8K.
pub const KERNAL_BASE: usize = 0xe000;
/// Character ROM base ($D000), 4K.
pub const CHARGEN_BASE: usize = 0xd000;

/// System ROM images, in the order VICE ships them.
pub struct RomImages {
    pub basic: Vec<u8>,
    pub kernal: Vec<u8>,
    pub chargen: Vec<u8>,
}

impl RomImages {
    /// Load the standard PAL ROMs from a VICE share directory.
    pub fn load_from_vice_share(dir: &Path) -> std::io::Result<Self> {
        let basic = fs::read(dir.join("basic-901226-01.bin"))?;
        let kernal = fs::read(dir.join("kernal-901227-03.bin"))?;
        let chargen = fs::read(dir.join("chargen-901225-01.bin"))?;
        Ok(Self {
            basic,
            kernal,
            chargen,
        })
    }
}

/// CPU view of the C64 memory map with banking via $01.
///
/// Bank select (CPU port $01):
/// - bit 0: 0 = BASIC ROM at $A000, 1 = RAM
/// - bit 1: 0 = RAM at $D000-$DFFF, 1 = I/O
/// - bit 2: 0 = RAM at $E000, 1 = KERNAL ROM
///
/// The bus records every access into a `ProvenanceMap`; the `fetch` method
/// (opcode reads) marks bytes as executed, data reads as `cpu_read`, writes
/// as `cpu_written`.
pub struct C64Bus {
    ram: Box<[u8; MEM_SIZE]>,
    basic_rom: Box<[u8; 8192]>,
    kernal_rom: Box<[u8; 8192]>,
    chargen_rom: Box<[u8; 4096]>,
    pub provenance: ProvenanceMap,
    /// CPU port register ($01).
    pub port01: u8,
    /// CIA I/O stub: reads return 0xff, writes are absorbed.
    pub io: Box<[u8; 4096]>,
    pub cycles: u64,
}

impl C64Bus {
    /// Build the machine from a raw 64K RAM image (the VICE ram-bank
    /// snapshot) plus system ROMs.
    pub fn new(ram: [u8; MEM_SIZE], roms: &RomImages) -> Self {
        let mut basic = [0_u8; 8192];
        let mut kernal = [0_u8; 8192];
        let mut chargen = [0_u8; 4096];
        basic.copy_from_slice(&roms.basic[..roms.basic.len().min(8192)]);
        kernal.copy_from_slice(&roms.kernal[..roms.kernal.len().min(8192)]);
        chargen.copy_from_slice(&roms.chargen[..roms.chargen.len().min(4096)]);
        Self {
            ram: Box::new(ram),
            basic_rom: Box::new(basic),
            kernal_rom: Box::new(kernal),
            chargen_rom: Box::new(chargen),
            provenance: ProvenanceMap::c64_ram(),
            port01: 0x37,
            io: Box::new([0xff; 4096]),
            cycles: 0,
        }
    }

    pub fn ram(&self) -> &[u8; MEM_SIZE] {
        &self.ram
    }

    fn map_read(&self, addr: u16) -> u8 {
        let address = usize::from(addr);
        match address {
            0x0000..=0x9fff => self.ram[address],
            0xa000..=0xbfff => {
                if self.port01 & 0x01 == 0 {
                    self.basic_rom[address - BASIC_BASE]
                } else {
                    self.ram[address]
                }
            }
            0xc000..=0xcfff => self.ram[address],
            0xd000..=0xdfff => {
                if self.port01 & 0x02 == 0 {
                    self.ram[address]
                } else {
                    // I/O region: return the chargen when $D018 selects it is
                    // handled by the VIC, not the CPU; CPU reads of chargen
                    // are visible when the VICII/chargen banking is active.
                    // We expose chargen at $D000-$DFFF and stub the rest.
                    self.chargen_rom[address - CHARGEN_BASE]
                }
            }
            0xe000..=0xffff => {
                if self.port01 & 0x04 == 0 {
                    self.ram[address]
                } else {
                    self.kernal_rom[address - KERNAL_BASE]
                }
            }
            _ => 0,
        }
    }

    fn map_write(&mut self, addr: u16, value: u8) {
        let address = usize::from(addr);
        match address {
            0x0000..=0x9fff => self.ram[address] = value,
            0xa000..=0xbfff => {
                if self.port01 & 0x01 != 0 {
                    self.ram[address] = value;
                }
                // Writes to BASIC ROM are ignored (or go to RAM if RAM
                // is banked in, handled above).
            }
            0xc000..=0xcfff => self.ram[address] = value,
            0xd000..=0xdfff => {
                if self.port01 & 0x02 == 0 {
                    self.ram[address] = value;
                } else {
                    // I/O writes: capture $01 (CPU port) and absorb the rest.
                    if address == 0x0001 || address == 0xd001 {
                        // $01 is at 0x0001, not $D001; handled below.
                    }
                    if address == 0x0001 {
                        self.port01 = value;
                    }
                    self.io[address - 0xd000] = value;
                }
            }
            0xe000..=0xffff if self.port01 & 0x04 == 0 => {
                self.ram[address] = value;
            }
            _ => {}
        }
        if address == 0x0001 {
            self.port01 = value;
        }
    }
}

impl Bus for C64Bus {
    fn fetch(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        let value = self.map_read(addr);
        self.provenance.get_mut(addr).mark_executed();
        value
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        let value = self.map_read(addr);
        self.provenance.get_mut(addr).mark_cpu_read();
        value
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.cycles += 1;
        self.map_write(addr, value);
        self.provenance.get_mut(addr).mark_cpu_written();
    }

    fn tick(&mut self, cycles: u8) {
        self.cycles += u64::from(cycles);
    }
}

/// Execution backend: the embedded machine or an external emulator (VICE).
///
/// The embedded `C64Machine` records true per-byte provenance on every
/// access; a VICE adapter (in `c64re-capture`) implements the same trait so
/// callers can run against either without knowing which is underneath.
pub trait Backend {
    fn pc(&mut self) -> u16;
    fn set_pc(&mut self, pc: u16);
    fn read_mem(&mut self, addr: u16) -> u8;
    fn write_mem(&mut self, addr: u16, value: u8);
    /// Run the CPU for `cycles` (cycle budget).
    fn run_cycles(&mut self, cycles: u64);
    fn provenance(&self) -> &ProvenanceMap;
}

impl Backend for C64Bus {
    fn pc(&mut self) -> u16 {
        // The bus alone has no CPU; callers keep a Cpu6502 alongside.
        // This implementation is provided for the C64Machine wrapper below.
        unreachable!("C64Bus has no PC; use C64Machine")
    }
    fn set_pc(&mut self, _pc: u16) {
        unreachable!("C64Bus has no PC; use C64Machine")
    }
    fn read_mem(&mut self, addr: u16) -> u8 {
        self.map_read(addr)
    }
    fn write_mem(&mut self, addr: u16, value: u8) {
        self.map_write(addr, value);
        self.provenance.get_mut(addr).mark_cpu_written();
    }
    fn run_cycles(&mut self, _cycles: u64) {
        unreachable!("C64Bus has no CPU; use C64Machine")
    }
    fn provenance(&self) -> &ProvenanceMap {
        &self.provenance
    }
}

/// A CPU + bus pair presenting the full `Backend` interface.
pub struct C64Machine {
    pub cpu: Cpu6502<C64Bus>,
}

impl C64Machine {
    /// Build from a raw 64K RAM image, system ROMs, and a starting PC.
    pub fn new(ram: [u8; MEM_SIZE], roms: &RomImages, pc: u16) -> Self {
        let bus = C64Bus::new(ram, roms);
        Self {
            cpu: Cpu6502::new(pc, 0, 0, 0, 0xfd, 0x00, bus),
        }
    }

    /// Load a VICE ram-bank snapshot (Vec of 65536 bytes).
    pub fn from_snapshot(ram: Vec<u8>, roms: &RomImages, pc: u16) -> Self {
        let mut image = [0_u8; MEM_SIZE];
        image.copy_from_slice(&ram[..ram.len().min(MEM_SIZE)]);
        Self::new(image, roms, pc)
    }

    pub fn into_bus(self) -> C64Bus {
        self.cpu.bus
    }

    /// Current 64K RAM image (the bus's current memory map).
    pub fn ram_snapshot(&self) -> &[u8; MEM_SIZE] {
        self.cpu.bus.ram()
    }
}

impl Backend for C64Machine {
    fn pc(&mut self) -> u16 {
        self.cpu.pc
    }
    fn set_pc(&mut self, pc: u16) {
        self.cpu.pc = pc;
    }
    fn read_mem(&mut self, addr: u16) -> u8 {
        self.cpu.bus.read(addr)
    }
    fn write_mem(&mut self, addr: u16, value: u8) {
        self.cpu.bus.write(addr, value);
    }
    fn run_cycles(&mut self, cycles: u64) {
        let mut budget = cycles;
        // JAM halts the CPU; without it, step until the budget is consumed.
        while budget > 0 {
            let before = self.cpu.bus.cycles;
            let used = u64::from(self.cpu.step());
            budget = budget.saturating_sub(used);
            if self.cpu.bus.cycles == before {
                break; // JAM loop: PC is not advancing
            }
        }
    }
    fn provenance(&self) -> &ProvenanceMap {
        &self.cpu.bus.provenance
    }
}

/// Auto-discover the VICE share directory (homebrew default on macOS).
pub fn discover_vice_rom_dir() -> Option<std::path::PathBuf> {
    for candidate in [
        "/opt/homebrew/share/vice/C64",
        "/usr/local/share/vice/C64",
        "/usr/share/vice/C64",
    ] {
        let path = std::path::Path::new(candidate);
        if path.join("basic-901226-01.bin").exists() {
            return Some(path.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use c64re_cpu::Cpu6502;

    fn roms() -> RomImages {
        RomImages {
            basic: vec![0xea; 8192],
            kernal: vec![0xea; 8192],
            chargen: vec![0xea; 4096],
        }
    }

    #[test]
    fn bank_select_switches_basic_rom_and_ram() {
        let mut ram = [0_u8; MEM_SIZE];
        ram[0xa000] = 0xaa; // RAM byte at BASIC base
        let mut bus = C64Bus::new(ram, &roms());
        bus.basic_rom[0] = 0xbb;
        // port01 bit0 = 0 -> BASIC ROM visible
        bus.port01 = 0x36;
        assert_eq!(bus.map_read(0xa000), 0xbb);
        // port01 bit0 = 1 -> RAM visible
        bus.port01 = 0x37;
        assert_eq!(bus.map_read(0xa000), 0xaa);
    }

    #[test]
    fn provenance_tracks_fetch_read_write() {
        let ram = [0_u8; MEM_SIZE];
        let mut bus = C64Bus::new(ram, &roms());
        // Program: LDA #$42 (2 bytes) then STA $C000 (3 bytes)
        bus.ram[0x0800] = 0xa9;
        bus.ram[0x0801] = 0x42;
        bus.ram[0x0802] = 0x8d;
        bus.ram[0x0803] = 0x00;
        bus.ram[0x0804] = 0xc0;
        let mut cpu = Cpu6502::new(0x0800, 0, 0, 0, 0xfd, 0, bus);
        cpu.step();
        cpu.step();
        let bus = cpu.bus;
        assert!(bus.provenance.get(0x0800).executed);
        assert!(bus.provenance.get(0x0801).executed);
        assert!(bus.provenance.get(0x0802).executed);
        assert!(bus.provenance.get(0xc000).cpu_written);
        assert_eq!(bus.ram[0xc000], 0x42);
    }

    #[test]
    fn write_then_execute_is_marked() {
        let mut ram = [0_u8; MEM_SIZE];
        ram[0x0800] = 0x8d; // STA $0801 (self-modifying: writes to the next byte)
        ram[0x0801] = 0x01;
        ram[0x0802] = 0x08;
        ram[0x0803] = 0xea; // NOP
        ram[0x0804] = 0xea;
        ram[0x0805] = 0xea;
        let bus = C64Bus::new(ram, &roms());
        let mut cpu = Cpu6502::new(0x0800, 0xea, 0, 0, 0xfd, 0, bus);
        cpu.step(); // STA $0801 writes 0xea to $0801
        cpu.pc = 0x0801;
        cpu.step(); // execute whatever is now at $0801
        assert!(cpu.bus.provenance.get(0x0801).cpu_written);
        assert!(cpu.bus.provenance.get(0x0801).executed);
        assert!(cpu.bus.provenance.get(0x0801).write_then_execute);
    }
}
