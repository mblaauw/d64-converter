//! A real 6502 instruction decoder with the full NMOS opcode table,
//! addressing modes, and operand formatting. Used to disassemble executed
//! code seeded by CpuHistory PCs / provenance coverage.

use c64re_provenance::ProvenanceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRange {
    pub start: u16,
    pub end_inclusive: u16,
}

impl AddressRange {
    pub fn len(self) -> usize {
        usize::from(self.end_inclusive - self.start) + 1
    }

    pub fn is_empty(self) -> bool {
        self.end_inclusive < self.start
    }
}

pub fn executed_ranges(provenance: &ProvenanceMap) -> Vec<AddressRange> {
    let mut ranges = Vec::new();
    let mut current_start: Option<u16> = None;

    for address in 0_u16..=u16::MAX {
        let executed = provenance.get(address).executed;
        match (current_start, executed) {
            (None, true) => current_start = Some(address),
            (Some(start), false) => {
                ranges.push(AddressRange {
                    start,
                    end_inclusive: address.wrapping_sub(1),
                });
                current_start = None;
            }
            _ => {}
        }
    }

    if let Some(start) = current_start {
        ranges.push(AddressRange {
            start,
            end_inclusive: u16::MAX,
        });
    }

    ranges
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
}

impl AddressingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implied => "implied",
            Self::Accumulator => "accumulator",
            Self::Immediate => "immediate",
            Self::ZeroPage => "zeropage",
            Self::ZeroPageX => "zeropage,x",
            Self::ZeroPageY => "zeropage,y",
            Self::Absolute => "absolute",
            Self::AbsoluteX => "absolute,x",
            Self::AbsoluteY => "absolute,y",
            Self::Indirect => "indirect",
            Self::IndirectX => "(indirect,x)",
            Self::IndirectY => "(indirect),y",
            Self::Relative => "relative",
        }
    }
}

/// Decoded instruction metadata: mnemonic, mode, and length in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpInfo {
    pub mnemonic: &'static str,
    pub mode: AddressingMode,
    pub length: u8,
}

/// A fully formatted disassembly line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisasmLine {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: &'static str,
    pub mode: AddressingMode,
    /// Raw operand bytes (excluding opcode).
    pub operand: Vec<u8>,
    /// Human-readable operand text ("$C000", "#$0A", "($10),Y" ...).
    pub operand_text: String,
}

impl std::fmt::Display for DisasmLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl DisasmLine {
    pub fn render(&self) -> String {
        let bytes: Vec<String> = self.bytes.iter().map(|b| format!("{b:02x}")).collect();
        let padded = if bytes.len() < 3 {
            let mut v = bytes.clone();
            while v.len() < 3 {
                v.push("  ".to_string());
            }
            v
        } else {
            bytes
        };
        format!(
            "${:04x}: {:}  {} {}",
            self.address,
            padded.join(" "),
            self.mnemonic,
            self.operand_text
        )
    }
}

/// Decode one opcode into its mnemonic, mode, and length.
/// Unknown/illegal opcodes map to "???" with a best-effort length of 1.
pub fn decode(opcode: u8) -> OpInfo {
    let (mnemonic, mode, length) = OPCODE_TABLE[usize::from(opcode)];
    OpInfo {
        mnemonic,
        mode,
        length,
    }
}

/// Disassemble `count` instructions from RAM at `address`, honoring
/// instruction lengths (linear sweep from the seed).
pub fn disassemble(ram: &[u8], mut address: u16, count: usize) -> Vec<DisasmLine> {
    let mut lines = Vec::new();
    for _ in 0..count {
        let Some(&opcode) = ram.get(usize::from(address)) else {
            break;
        };
        let info = decode(opcode);
        let mut operand = Vec::new();
        for offset in 1..info.length {
            let byte = ram
                .get(usize::from(address.wrapping_add(u16::from(offset))))
                .copied()
                .unwrap_or(0);
            operand.push(byte);
        }
        let operand_text = format_operand(info.mode, &operand, address);
        let mut bytes = vec![opcode];
        bytes.extend_from_slice(&operand);
        lines.push(DisasmLine {
            address,
            bytes,
            mnemonic: info.mnemonic,
            mode: info.mode,
            operand,
            operand_text,
        });
        address = address.wrapping_add(u16::from(info.length));
    }
    lines
}

/// Format an operand according to its addressing mode. `address` is the
/// instruction address (used for relative branches and JMP indirect notes).
fn format_operand(mode: AddressingMode, operand: &[u8], address: u16) -> String {
    match mode {
        AddressingMode::Implied => String::new(),
        AddressingMode::Accumulator => "A".to_string(),
        AddressingMode::Immediate => {
            let value = operand.first().copied().unwrap_or(0);
            format!("#${value:02x}")
        }
        AddressingMode::ZeroPage => {
            let value = operand.first().copied().unwrap_or(0);
            format!("${value:02x}")
        }
        AddressingMode::ZeroPageX => {
            let value = operand.first().copied().unwrap_or(0);
            format!("${value:02x},X")
        }
        AddressingMode::ZeroPageY => {
            let value = operand.first().copied().unwrap_or(0);
            format!("${value:02x},Y")
        }
        AddressingMode::Absolute => {
            let value = le16(operand);
            format!("${value:04x}")
        }
        AddressingMode::AbsoluteX => {
            let value = le16(operand);
            format!("${value:04x},X")
        }
        AddressingMode::AbsoluteY => {
            let value = le16(operand);
            format!("${value:04x},Y")
        }
        AddressingMode::Indirect => {
            let value = le16(operand);
            format!("(${value:04x})")
        }
        AddressingMode::IndirectX => {
            let value = operand.first().copied().unwrap_or(0);
            format!("(${value:02x},X)")
        }
        AddressingMode::IndirectY => {
            let value = operand.first().copied().unwrap_or(0);
            format!("(${value:02x}),Y")
        }
        AddressingMode::Relative => {
            let offset = operand.first().copied().unwrap_or(0);
            let delta = i8::from_le_bytes([offset]) as i16;
            let target = address.wrapping_add(2).wrapping_add(delta as u16);
            format!("${target:04x}")
        }
    }
}

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
    ])
}

/// NMOS 6502 opcode table: (mnemonic, addressing mode, length).
/// Illegal opcodes are decoded with the closest-length heuristic so linear
/// sweeps still advance; their names are the common ones where known.
const OPCODE_TABLE: [(&str, AddressingMode, u8); 256] = [
    ("BRK", AddressingMode::Implied, 1),
    ("ORA", AddressingMode::IndirectX, 2),
    ("???", AddressingMode::Implied, 1),
    ("SLO", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::ZeroPage, 2),
    ("ORA", AddressingMode::ZeroPage, 2),
    ("ASL", AddressingMode::ZeroPage, 2),
    ("SLO", AddressingMode::ZeroPage, 2),
    ("PHP", AddressingMode::Implied, 1),
    ("ORA", AddressingMode::Immediate, 2),
    ("ASL", AddressingMode::Accumulator, 1),
    ("ANC", AddressingMode::Immediate, 2),
    ("NOP", AddressingMode::Absolute, 3),
    ("ORA", AddressingMode::Absolute, 3),
    ("ASL", AddressingMode::Absolute, 3),
    ("SLO", AddressingMode::Absolute, 3),
    ("BPL", AddressingMode::Relative, 2),
    ("ORA", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("SLO", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("ORA", AddressingMode::ZeroPageX, 2),
    ("ASL", AddressingMode::ZeroPageX, 2),
    ("SLO", AddressingMode::ZeroPageX, 2),
    ("CLC", AddressingMode::Implied, 1),
    ("ORA", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("SLO", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("ORA", AddressingMode::AbsoluteX, 3),
    ("ASL", AddressingMode::AbsoluteX, 3),
    ("SLO", AddressingMode::AbsoluteX, 3),
    ("JSR", AddressingMode::Absolute, 3),
    ("AND", AddressingMode::IndirectX, 2),
    ("???", AddressingMode::Implied, 1),
    ("RLA", AddressingMode::IndirectX, 2),
    ("BIT", AddressingMode::ZeroPage, 2),
    ("AND", AddressingMode::ZeroPage, 2),
    ("ROL", AddressingMode::ZeroPage, 2),
    ("RLA", AddressingMode::ZeroPage, 2),
    ("PLP", AddressingMode::Implied, 1),
    ("AND", AddressingMode::Immediate, 2),
    ("ROL", AddressingMode::Accumulator, 1),
    ("ANC", AddressingMode::Immediate, 2),
    ("BIT", AddressingMode::Absolute, 3),
    ("AND", AddressingMode::Absolute, 3),
    ("ROL", AddressingMode::Absolute, 3),
    ("RLA", AddressingMode::Absolute, 3),
    ("BMI", AddressingMode::Relative, 2),
    ("AND", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("RLA", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("AND", AddressingMode::ZeroPageX, 2),
    ("ROL", AddressingMode::ZeroPageX, 2),
    ("RLA", AddressingMode::ZeroPageX, 2),
    ("SEC", AddressingMode::Implied, 1),
    ("AND", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("RLA", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("AND", AddressingMode::AbsoluteX, 3),
    ("ROL", AddressingMode::AbsoluteX, 3),
    ("RLA", AddressingMode::AbsoluteX, 3),
    ("RTI", AddressingMode::Implied, 1),
    ("EOR", AddressingMode::IndirectX, 2),
    ("???", AddressingMode::Implied, 1),
    ("SRE", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::ZeroPage, 2),
    ("EOR", AddressingMode::ZeroPage, 2),
    ("LSR", AddressingMode::ZeroPage, 2),
    ("SRE", AddressingMode::ZeroPage, 2),
    ("PHA", AddressingMode::Implied, 1),
    ("EOR", AddressingMode::Immediate, 2),
    ("LSR", AddressingMode::Accumulator, 1),
    ("ALR", AddressingMode::Immediate, 2),
    ("JMP", AddressingMode::Absolute, 3),
    ("EOR", AddressingMode::Absolute, 3),
    ("LSR", AddressingMode::Absolute, 3),
    ("SRE", AddressingMode::Absolute, 3),
    ("BVC", AddressingMode::Relative, 2),
    ("EOR", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("SRE", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("EOR", AddressingMode::ZeroPageX, 2),
    ("LSR", AddressingMode::ZeroPageX, 2),
    ("SRE", AddressingMode::ZeroPageX, 2),
    ("CLI", AddressingMode::Implied, 1),
    ("EOR", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("SRE", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("EOR", AddressingMode::AbsoluteX, 3),
    ("LSR", AddressingMode::AbsoluteX, 3),
    ("SRE", AddressingMode::AbsoluteX, 3),
    ("RTS", AddressingMode::Implied, 1),
    ("ADC", AddressingMode::IndirectX, 2),
    ("???", AddressingMode::Implied, 1),
    ("RRA", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::ZeroPage, 2),
    ("ADC", AddressingMode::ZeroPage, 2),
    ("ROR", AddressingMode::ZeroPage, 2),
    ("RRA", AddressingMode::ZeroPage, 2),
    ("PLA", AddressingMode::Implied, 1),
    ("ADC", AddressingMode::Immediate, 2),
    ("ROR", AddressingMode::Accumulator, 1),
    ("ARR", AddressingMode::Immediate, 2),
    ("JMP", AddressingMode::Indirect, 3),
    ("ADC", AddressingMode::Absolute, 3),
    ("ROR", AddressingMode::Absolute, 3),
    ("RRA", AddressingMode::Absolute, 3),
    ("BVS", AddressingMode::Relative, 2),
    ("ADC", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("RRA", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("ADC", AddressingMode::ZeroPageX, 2),
    ("ROR", AddressingMode::ZeroPageX, 2),
    ("RRA", AddressingMode::ZeroPageX, 2),
    ("SEI", AddressingMode::Implied, 1),
    ("ADC", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("RRA", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("ADC", AddressingMode::AbsoluteX, 3),
    ("ROR", AddressingMode::AbsoluteX, 3),
    ("RRA", AddressingMode::AbsoluteX, 3),
    ("NOP", AddressingMode::Immediate, 2),
    ("STA", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::Immediate, 2),
    ("SAX", AddressingMode::IndirectX, 2),
    ("STY", AddressingMode::ZeroPage, 2),
    ("STA", AddressingMode::ZeroPage, 2),
    ("STX", AddressingMode::ZeroPage, 2),
    ("SAX", AddressingMode::ZeroPage, 2),
    ("DEY", AddressingMode::Implied, 1),
    ("NOP", AddressingMode::Immediate, 2),
    ("TXA", AddressingMode::Implied, 1),
    ("ANE", AddressingMode::Immediate, 2),
    ("STY", AddressingMode::Absolute, 3),
    ("STA", AddressingMode::Absolute, 3),
    ("STX", AddressingMode::Absolute, 3),
    ("SAX", AddressingMode::Absolute, 3),
    ("BCC", AddressingMode::Relative, 2),
    ("STA", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("SHA", AddressingMode::IndirectY, 2),
    ("STY", AddressingMode::ZeroPageX, 2),
    ("STA", AddressingMode::ZeroPageX, 2),
    ("STX", AddressingMode::ZeroPageY, 2),
    ("SAX", AddressingMode::ZeroPageY, 2),
    ("TYA", AddressingMode::Implied, 1),
    ("STA", AddressingMode::AbsoluteY, 3),
    ("TXS", AddressingMode::Implied, 1),
    ("SHS", AddressingMode::AbsoluteY, 3),
    ("SHY", AddressingMode::AbsoluteX, 3),
    ("STA", AddressingMode::AbsoluteX, 3),
    ("SHX", AddressingMode::AbsoluteY, 3),
    ("SHA", AddressingMode::AbsoluteY, 3),
    ("LDY", AddressingMode::Immediate, 2),
    ("LDA", AddressingMode::IndirectX, 2),
    ("LDX", AddressingMode::Immediate, 2),
    ("LAX", AddressingMode::IndirectX, 2),
    ("LDY", AddressingMode::ZeroPage, 2),
    ("LDA", AddressingMode::ZeroPage, 2),
    ("LDX", AddressingMode::ZeroPage, 2),
    ("LAX", AddressingMode::ZeroPage, 2),
    ("TAY", AddressingMode::Implied, 1),
    ("LDA", AddressingMode::Immediate, 2),
    ("TAX", AddressingMode::Implied, 1),
    ("LAX", AddressingMode::Immediate, 2),
    ("LDY", AddressingMode::Absolute, 3),
    ("LDA", AddressingMode::Absolute, 3),
    ("LDX", AddressingMode::Absolute, 3),
    ("LAX", AddressingMode::Absolute, 3),
    ("BCS", AddressingMode::Relative, 2),
    ("LDA", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("LAX", AddressingMode::IndirectY, 2),
    ("LDY", AddressingMode::ZeroPageX, 2),
    ("LDA", AddressingMode::ZeroPageX, 2),
    ("LDX", AddressingMode::ZeroPageY, 2),
    ("LAX", AddressingMode::ZeroPageY, 2),
    ("CLV", AddressingMode::Implied, 1),
    ("LDA", AddressingMode::AbsoluteY, 3),
    ("TSX", AddressingMode::Implied, 1),
    ("LAS", AddressingMode::AbsoluteY, 3),
    ("LDY", AddressingMode::AbsoluteX, 3),
    ("LDA", AddressingMode::AbsoluteX, 3),
    ("LDX", AddressingMode::AbsoluteY, 3),
    ("LAX", AddressingMode::AbsoluteY, 3),
    ("CPY", AddressingMode::Immediate, 2),
    ("CMP", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::Immediate, 2),
    ("DCP", AddressingMode::IndirectX, 2),
    ("CPY", AddressingMode::ZeroPage, 2),
    ("CMP", AddressingMode::ZeroPage, 2),
    ("DEC", AddressingMode::ZeroPage, 2),
    ("DCP", AddressingMode::ZeroPage, 2),
    ("INY", AddressingMode::Implied, 1),
    ("CMP", AddressingMode::Immediate, 2),
    ("DEX", AddressingMode::Implied, 1),
    ("AXS", AddressingMode::Immediate, 2),
    ("CPY", AddressingMode::Absolute, 3),
    ("CMP", AddressingMode::Absolute, 3),
    ("DEC", AddressingMode::Absolute, 3),
    ("DCP", AddressingMode::Absolute, 3),
    ("BNE", AddressingMode::Relative, 2),
    ("CMP", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("DCP", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("CMP", AddressingMode::ZeroPageX, 2),
    ("DEC", AddressingMode::ZeroPageX, 2),
    ("DCP", AddressingMode::ZeroPageX, 2),
    ("CLD", AddressingMode::Implied, 1),
    ("CMP", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("DCP", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("CMP", AddressingMode::AbsoluteX, 3),
    ("DEC", AddressingMode::AbsoluteX, 3),
    ("DCP", AddressingMode::AbsoluteX, 3),
    ("CPX", AddressingMode::Immediate, 2),
    ("SBC", AddressingMode::IndirectX, 2),
    ("NOP", AddressingMode::Immediate, 2),
    ("ISB", AddressingMode::IndirectX, 2),
    ("CPX", AddressingMode::ZeroPage, 2),
    ("SBC", AddressingMode::ZeroPage, 2),
    ("INC", AddressingMode::ZeroPage, 2),
    ("ISB", AddressingMode::ZeroPage, 2),
    ("INX", AddressingMode::Implied, 1),
    ("SBC", AddressingMode::Immediate, 2),
    ("NOP", AddressingMode::Implied, 1),
    ("SBC", AddressingMode::Immediate, 2),
    ("CPX", AddressingMode::Absolute, 3),
    ("SBC", AddressingMode::Absolute, 3),
    ("INC", AddressingMode::Absolute, 3),
    ("ISB", AddressingMode::Absolute, 3),
    ("BEQ", AddressingMode::Relative, 2),
    ("SBC", AddressingMode::IndirectY, 2),
    ("???", AddressingMode::Implied, 1),
    ("ISB", AddressingMode::IndirectY, 2),
    ("NOP", AddressingMode::ZeroPageX, 2),
    ("SBC", AddressingMode::ZeroPageX, 2),
    ("INC", AddressingMode::ZeroPageX, 2),
    ("ISB", AddressingMode::ZeroPageX, 2),
    ("SED", AddressingMode::Implied, 1),
    ("SBC", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::Implied, 1),
    ("ISB", AddressingMode::AbsoluteY, 3),
    ("NOP", AddressingMode::AbsoluteX, 3),
    ("SBC", AddressingMode::AbsoluteX, 3),
    ("INC", AddressingMode::AbsoluteX, 3),
    ("ISB", AddressingMode::AbsoluteX, 3),
];

/// Disassemble the executed ranges of a provenance map (from RAM). This is
/// the "coverage-seeded" disassembly: only code that actually ran is shown.
pub fn disassemble_executed(
    ram: &[u8],
    provenance: &ProvenanceMap,
    max_lines: usize,
) -> Vec<DisasmLine> {
    let ranges = executed_ranges(provenance);
    let mut lines = Vec::new();
    for range in ranges {
        if lines.len() >= max_lines {
            break;
        }
        let count = (range.len()).min(max_lines - lines.len());
        let mut sweep = disassemble(ram, range.start, count);
        lines.append(&mut sweep);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_opcodes() {
        assert_eq!(decode(0xa9).mnemonic, "LDA");
        assert_eq!(decode(0xa9).mode, AddressingMode::Immediate);
        assert_eq!(decode(0xa9).length, 2);
        assert_eq!(decode(0x4c).mnemonic, "JMP");
        assert_eq!(decode(0x4c).mode, AddressingMode::Absolute);
        assert_eq!(decode(0x4c).length, 3);
        assert_eq!(decode(0x6c).mode, AddressingMode::Indirect);
        assert_eq!(decode(0xd0).mode, AddressingMode::Relative);
        assert_eq!(decode(0x60).mnemonic, "RTS");
    }

    #[test]
    fn formats_operands() {
        let mut ram = vec![0_u8; 65_536];
        // LDA #$0A
        ram[0x0801] = 0xa9;
        ram[0x0802] = 0x0a;
        let line = disassemble(&ram, 0x0801, 1);
        assert_eq!(line[0].operand_text, "#$0a");
        // LDA $C000,X
        ram[0x0801] = 0xbd;
        ram[0x0802] = 0x00;
        ram[0x0803] = 0xc0;
        let line = disassemble(&ram, 0x0801, 1);
        assert_eq!(line[0].operand_text, "$c000,X");
        // JMP ($FFFC)
        ram[0x0801] = 0x6c;
        ram[0x0802] = 0xfc;
        ram[0x0803] = 0xff;
        let line = disassemble(&ram, 0x0801, 1);
        assert_eq!(line[0].operand_text, "($fffc)");
        // BNE with relative target
        ram[0x0801] = 0xd0;
        ram[0x0802] = 0x05;
        let line = disassemble(&ram, 0x0801, 1);
        assert_eq!(line[0].operand_text, "$0808");
    }

    #[test]
    fn linear_sweep_advances_by_length() {
        let mut ram = vec![0_u8; 65_536];
        // LDA #$0A (2) + RTS (1)
        ram[0x0800] = 0xa9;
        ram[0x0801] = 0x0a;
        ram[0x0802] = 0x60;
        let lines = disassemble(&ram, 0x0800, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].address, 0x0800);
        assert_eq!(lines[1].address, 0x0802);
        assert_eq!(lines[1].mnemonic, "RTS");
    }

    #[test]
    fn executed_range_is_empty_fixed() {
        let range = AddressRange {
            start: 10,
            end_inclusive: 5,
        };
        assert!(range.is_empty());
    }
}
