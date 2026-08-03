//! A cycle-counted NMOS 6502 CPU core.
//!
//! The core is bus-agnostic: memory access goes through the [`Bus`] trait,
//! which also receives a cycle tick. This lets an embedding emulator record
//! per-byte provenance, timing, or anything else without touching the CPU
//! logic.
//!
//! Instruction timing follows the official NMOS 6502 datasheet: branch
//! taken adds a cycle, page-crossing adds a cycle, RMW ops are 2 cycles
//! longer than the plain read. The common undocumented opcodes (SLO, RLA,
//! SRE, RRA, DCP, ISB, LAX, SAX) are implemented with their accepted
//! semantics; the rest of the illegal space decodes as `NOP`/`JAM` style
//! no-ops with nominal timing.

/// Memory interface: the CPU reads/writes through here.
pub trait Bus {
    /// Fetch an opcode byte (counts as an execution).
    fn fetch(&mut self, addr: u16) -> u8;
    /// Read a data byte.
    fn read(&mut self, addr: u16) -> u8;
    /// Write a data byte.
    fn write(&mut self, addr: u16, value: u8);
    /// Advance the bus clock by `cycles` (for page-cross and RMW timing).
    fn tick(&mut self, cycles: u8);
}

/// Processor status flags.
pub mod flags {
    pub const CARRY: u8 = 0x01;
    pub const ZERO: u8 = 0x02;
    pub const IRQ_DISABLE: u8 = 0x04;
    pub const DECIMAL: u8 = 0x08;
    pub const BRK: u8 = 0x10;
    pub const UNUSED: u8 = 0x20;
    pub const OVERFLOW: u8 = 0x40;
    pub const NEGATIVE: u8 = 0x80;
}

/// The 6502 CPU.
#[derive(Debug, Clone)]
pub struct Cpu6502<B: Bus> {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub p: u8,
    pub bus: B,
}

/// An operand: an effective address, or an immediate value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operand {
    Addr(u16),
    Value(u8),
}

/// Addressing modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Imp,
    Acc,
    Imm,
    Zp,
    Zpx,
    Zpy,
    Abs,
    Absx,
    Absy,
    Ind,
    Indx,
    Indy,
    Rel,
}

/// Operations (official + common undocumented).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    // Undocumented (with accepted semantics)
    SlO,
    Rla,
    Sre,
    Rra,
    Dcp,
    Isb,
    Lax,
    Sax,
    /// JAM: halt the CPU (treated as a no-op loop for now).
    Jam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decoded {
    op: Op,
    mode: Mode,
    cycles: u8,
}

/// Opcode table: (operation, mode, base cycles). Illegal opcodes use the
/// commonly documented timings (2 for implicit/zero-page-class, 4 for
/// absolute-class NOPs etc.).
static TABLE: std::sync::LazyLock<[Decoded; 256]> = std::sync::LazyLock::new(|| {
    use Mode::*;
    use Op::*;
    let mut t = [Decoded {
        op: Jam,
        mode: Imp,
        cycles: 2,
    }; 256];
    // Official opcodes.
    let mut set = |opcode: usize, op: Op, mode: Mode, cycles: u8| {
        t[opcode] = Decoded { op, mode, cycles };
    };
    set(0x00, Brk, Imp, 7);
    set(0x01, Ora, Indx, 6);
    set(0x05, Ora, Zp, 3);
    set(0x06, Asl, Zp, 5);
    set(0x08, Php, Imp, 3);
    set(0x09, Ora, Imm, 2);
    set(0x0a, Asl, Acc, 2);
    set(0x0d, Ora, Abs, 4);
    set(0x0e, Asl, Abs, 6);
    set(0x10, Bpl, Rel, 2);
    set(0x11, Ora, Indy, 5);
    set(0x15, Ora, Zpx, 4);
    set(0x16, Asl, Zpx, 6);
    set(0x18, Clc, Imp, 2);
    set(0x19, Ora, Absy, 4);
    set(0x1d, Ora, Absx, 4);
    set(0x1e, Asl, Absx, 7);
    set(0x20, Jsr, Abs, 6);
    set(0x21, And, Indx, 6);
    set(0x24, Bit, Zp, 3);
    set(0x25, And, Zp, 3);
    set(0x26, Rol, Zp, 5);
    set(0x28, Plp, Imp, 4);
    set(0x29, And, Imm, 2);
    set(0x2a, Rol, Acc, 2);
    set(0x2c, Bit, Abs, 4);
    set(0x2d, And, Abs, 4);
    set(0x2e, Rol, Abs, 6);
    set(0x30, Bmi, Rel, 2);
    set(0x31, And, Indy, 5);
    set(0x35, And, Zpx, 4);
    set(0x36, Rol, Zpx, 6);
    set(0x38, Sec, Imp, 2);
    set(0x39, And, Absy, 4);
    set(0x3d, And, Absx, 4);
    set(0x3e, Rol, Absx, 7);
    set(0x40, Rti, Imp, 6);
    set(0x41, Eor, Indx, 6);
    set(0x45, Eor, Zp, 3);
    set(0x46, Lsr, Zp, 5);
    set(0x48, Pha, Imp, 3);
    set(0x49, Eor, Imm, 2);
    set(0x4a, Lsr, Acc, 2);
    set(0x4c, Jmp, Abs, 3);
    set(0x4d, Eor, Abs, 4);
    set(0x4e, Lsr, Abs, 6);
    set(0x50, Bvc, Rel, 2);
    set(0x51, Eor, Indy, 5);
    set(0x55, Eor, Zpx, 4);
    set(0x56, Lsr, Zpx, 6);
    set(0x58, Cli, Imp, 2);
    set(0x59, Eor, Absy, 4);
    set(0x5d, Eor, Absx, 4);
    set(0x5e, Lsr, Absx, 7);
    set(0x60, Rts, Imp, 6);
    set(0x61, Adc, Indx, 6);
    set(0x65, Adc, Zp, 3);
    set(0x66, Ror, Zp, 5);
    set(0x68, Pla, Imp, 4);
    set(0x69, Adc, Imm, 2);
    set(0x6a, Ror, Acc, 2);
    set(0x6c, Jmp, Ind, 5);
    set(0x6d, Adc, Abs, 4);
    set(0x6e, Ror, Abs, 6);
    set(0x70, Bvs, Rel, 2);
    set(0x71, Adc, Indy, 5);
    set(0x75, Adc, Zpx, 4);
    set(0x76, Ror, Zpx, 6);
    set(0x78, Sei, Imp, 2);
    set(0x79, Adc, Absy, 4);
    set(0x7d, Adc, Absx, 4);
    set(0x7e, Ror, Absx, 7);
    set(0x81, Sta, Indx, 6);
    set(0x84, Sty, Zp, 3);
    set(0x85, Sta, Zp, 3);
    set(0x86, Stx, Zp, 3);
    set(0x88, Dey, Imp, 2);
    set(0x8a, Txa, Imp, 2);
    set(0x8c, Sty, Abs, 4);
    set(0x8d, Sta, Abs, 4);
    set(0x8e, Stx, Abs, 4);
    set(0x90, Bcc, Rel, 2);
    set(0x91, Sta, Indy, 6);
    set(0x94, Sty, Zpx, 4);
    set(0x95, Sta, Zpx, 4);
    set(0x96, Stx, Zpy, 4);
    set(0x98, Tya, Imp, 2);
    set(0x99, Sta, Absy, 5);
    set(0x9a, Txs, Imp, 2);
    set(0x9d, Sta, Absx, 5);
    set(0xa0, Ldy, Imm, 2);
    set(0xa1, Lda, Indx, 6);
    set(0xa2, Ldx, Imm, 2);
    set(0xa4, Ldy, Zp, 3);
    set(0xa5, Lda, Zp, 3);
    set(0xa6, Ldx, Zp, 3);
    set(0xa8, Tay, Imp, 2);
    set(0xa9, Lda, Imm, 2);
    set(0xaa, Tax, Imp, 2);
    set(0xac, Ldy, Abs, 4);
    set(0xad, Lda, Abs, 4);
    set(0xae, Ldx, Abs, 4);
    set(0xb0, Bcs, Rel, 2);
    set(0xb1, Lda, Indy, 5);
    set(0xb4, Ldy, Zpx, 4);
    set(0xb5, Lda, Zpx, 4);
    set(0xb6, Ldx, Zpy, 4);
    set(0xb8, Clv, Imp, 2);
    set(0xb9, Lda, Absy, 4);
    set(0xba, Tsx, Imp, 2);
    set(0xbc, Ldy, Absx, 4);
    set(0xbd, Lda, Absx, 4);
    set(0xbe, Ldx, Absy, 4);
    set(0xc0, Cpy, Imm, 2);
    set(0xc1, Cmp, Indx, 6);
    set(0xc4, Cpy, Zp, 3);
    set(0xc5, Cmp, Zp, 3);
    set(0xc6, Dec, Zp, 5);
    set(0xc8, Iny, Imp, 2);
    set(0xc9, Cmp, Imm, 2);
    set(0xca, Dex, Imp, 2);
    set(0xcc, Cpy, Abs, 4);
    set(0xcd, Cmp, Abs, 4);
    set(0xce, Dec, Abs, 6);
    set(0xd0, Bne, Rel, 2);
    set(0xd1, Cmp, Indy, 5);
    set(0xd5, Cmp, Zpx, 4);
    set(0xd6, Dec, Zpx, 6);
    set(0xd8, Cld, Imp, 2);
    set(0xd9, Cmp, Absy, 4);
    set(0xdd, Cmp, Absx, 4);
    set(0xde, Dec, Absx, 7);
    set(0xe0, Cpx, Imm, 2);
    set(0xe1, Sbc, Indx, 6);
    set(0xe4, Cpx, Zp, 3);
    set(0xe5, Sbc, Zp, 3);
    set(0xe6, Inc, Zp, 5);
    set(0xe8, Inx, Imp, 2);
    set(0xe9, Sbc, Imm, 2);
    set(0xea, Nop, Imp, 2);
    set(0xec, Cpx, Abs, 4);
    set(0xed, Sbc, Abs, 4);
    set(0xee, Inc, Abs, 6);
    set(0xf0, Beq, Rel, 2);
    set(0xf1, Sbc, Indy, 5);
    set(0xf5, Sbc, Zpx, 4);
    set(0xf6, Inc, Zpx, 6);
    set(0xf8, Sed, Imp, 2);
    set(0xf9, Sbc, Absy, 4);
    set(0xfd, Sbc, Absx, 4);
    set(0xfe, Inc, Absx, 7);

    // Common undocumented opcodes.
    // SLO = ASL + ORA
    for &(oc, mode) in &[
        (0x07, Zp),
        (0x17, Zpx),
        (0x0f, Abs),
        (0x1f, Absx),
        (0x1b, Absy),
        (0x03, Indx),
        (0x13, Indy),
    ] {
        set(
            oc,
            SlO,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // RLA = ROL + AND
    for &(oc, mode) in &[
        (0x27, Zp),
        (0x37, Zpx),
        (0x2f, Abs),
        (0x3f, Absx),
        (0x3b, Absy),
        (0x23, Indx),
        (0x33, Indy),
    ] {
        set(
            oc,
            Rla,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // SRE = LSR + EOR
    for &(oc, mode) in &[
        (0x47, Zp),
        (0x57, Zpx),
        (0x4f, Abs),
        (0x5f, Absx),
        (0x5b, Absy),
        (0x43, Indx),
        (0x53, Indy),
    ] {
        set(
            oc,
            Sre,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // RRA = ROR + ADC
    for &(oc, mode) in &[
        (0x67, Zp),
        (0x77, Zpx),
        (0x6f, Abs),
        (0x7f, Absx),
        (0x7b, Absy),
        (0x63, Indx),
        (0x73, Indy),
    ] {
        set(
            oc,
            Rra,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // DCP = DEC + CMP
    for &(oc, mode) in &[
        (0xc7, Zp),
        (0xd7, Zpx),
        (0xcf, Abs),
        (0xdf, Absx),
        (0xdb, Absy),
        (0xc3, Indx),
        (0xd3, Indy),
    ] {
        set(
            oc,
            Dcp,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // ISB = INC + SBC
    for &(oc, mode) in &[
        (0xe7, Zp),
        (0xf7, Zpx),
        (0xef, Abs),
        (0xff, Absx),
        (0xfb, Absy),
        (0xe3, Indx),
        (0xf3, Indy),
    ] {
        set(
            oc,
            Isb,
            mode,
            match mode {
                Zp => 5,
                Zpx => 6,
                Abs => 6,
                Absx => 7,
                Absy => 7,
                Indx => 8,
                Indy => 8,
                _ => 2,
            },
        );
    }
    // LAX = LDA + LDX
    for &(oc, mode) in &[
        (0xa7, Zp),
        (0xb7, Zpy),
        (0xaf, Abs),
        (0xbf, Absy),
        (0xa3, Indx),
        (0xb3, Indy),
    ] {
        set(
            oc,
            Lax,
            mode,
            match mode {
                Zp => 3,
                Zpy => 4,
                Abs => 4,
                Absy => 4,
                Indx => 6,
                Indy => 5,
                _ => 2,
            },
        );
    }
    // SAX = STA + STX
    for &(oc, mode) in &[(0x87, Zp), (0x97, Zpy), (0x8f, Abs), (0x83, Indx)] {
        set(
            oc,
            Sax,
            mode,
            match mode {
                Zp => 3,
                Zpy => 4,
                Abs => 4,
                Indx => 6,
                _ => 2,
            },
        );
    }
    // NOP variants: zp/zpx/abs/absx/immediate.
    for &(oc, mode) in &[
        (0x04, Zp),
        (0x44, Zp),
        (0x64, Zp),
        (0x14, Zpx),
        (0x34, Zpx),
        (0x54, Zpx),
        (0x74, Zpx),
        (0xd4, Zpx),
        (0xf4, Zpx),
        (0x0c, Abs),
        (0x1c, Absx),
        (0x3c, Absx),
        (0x5c, Absx),
        (0x7c, Absx),
        (0xdc, Absx),
        (0xfc, Absx),
        (0x80, Imm),
        (0x82, Imm),
        (0x89, Imm),
        (0xc2, Imm),
        (0xe2, Imm),
    ] {
        set(
            oc,
            Nop,
            mode,
            match mode {
                Zp => 3,
                Zpx => 4,
                Abs => 4,
                Absx => 4,
                Imm => 2,
                _ => 2,
            },
        );
    }
    // JAM opcodes: halt.
    for &oc in &[
        0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xb2, 0xd2, 0xf2,
    ] {
        set(oc, Jam, Imp, 2);
    }
    t
});

impl<B: Bus> Cpu6502<B> {
    pub fn new(pc: u16, a: u8, x: u8, y: u8, sp: u8, p: u8, bus: B) -> Self {
        Self {
            pc,
            a,
            x,
            y,
            sp,
            p,
            bus,
        }
    }

    /// Execute one instruction; returns the number of cycles used.
    pub fn step(&mut self) -> u8 {
        let opcode = self.bus.fetch(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let decoded = TABLE[usize::from(opcode)];
        let mut cycles = decoded.cycles;
        let mode = decoded.mode;

        match decoded.op {
            Op::Brk => {
                self.push16(self.pc.wrapping_add(1));
                self.push(self.p | flags::BRK | flags::UNUSED);
                self.p |= flags::IRQ_DISABLE;
                let vector = self.read16(0xfffe);
                self.pc = vector;
            }
            Op::Jsr => {
                let target = self.fetch16();
                self.push16(self.pc.wrapping_sub(1));
                self.pc = target;
            }
            Op::Rts => {
                let ret = self.pop16();
                self.pc = ret.wrapping_add(1);
            }
            Op::Rti => {
                self.p = self.pop();
                self.pc = self.pop16();
            }
            Op::Jmp => {
                let target = self.fetch16();
                if mode == Mode::Ind {
                    // JMP ($xxFF) page-wrap bug.
                    let addr = target;
                    let lo = self.bus.read(addr);
                    let hi_addr = (addr & 0xff00) | ((addr + 1) & 0x00ff);
                    let hi = self.bus.read(hi_addr);
                    self.pc = u16::from_le_bytes([lo, hi]);
                } else {
                    self.pc = target;
                }
            }
            _ => {
                let operand = self.resolve(mode, &mut cycles);
                self.execute(decoded.op, mode, operand, &mut cycles);
            }
        }
        cycles
    }

    fn resolve(&mut self, mode: Mode, cycles: &mut u8) -> Operand {
        match mode {
            Mode::Imp | Mode::Acc => Operand::Value(0),
            Mode::Ind => Operand::Value(0), // handled in step()
            Mode::Imm => Operand::Value(self.fetch8()),
            Mode::Zp => Operand::Addr(u16::from(self.fetch8())),
            Mode::Zpx => {
                let base = u16::from(self.fetch8());
                Operand::Addr((base.wrapping_add(u16::from(self.x))) & 0x00ff)
            }
            Mode::Zpy => {
                let base = u16::from(self.fetch8());
                Operand::Addr((base.wrapping_add(u16::from(self.y))) & 0x00ff)
            }
            Mode::Abs => Operand::Addr(self.fetch16()),
            Mode::Absx => {
                let base = self.fetch16();
                let addr = base.wrapping_add(u16::from(self.x));
                if (base & 0xff00) != (addr & 0xff00) {
                    *cycles += 1;
                }
                Operand::Addr(addr)
            }
            Mode::Absy => {
                let base = self.fetch16();
                let addr = base.wrapping_add(u16::from(self.y));
                if (base & 0xff00) != (addr & 0xff00) {
                    *cycles += 1;
                }
                Operand::Addr(addr)
            }
            Mode::Indx => {
                let zp = (u16::from(self.fetch8()) + u16::from(self.x)) & 0x00ff;
                let lo = self.bus.read(zp);
                let hi = self.bus.read(zp.wrapping_add(1) & 0x00ff);
                Operand::Addr(u16::from_le_bytes([lo, hi]))
            }
            Mode::Indy => {
                let zp = u16::from(self.fetch8());
                let lo = self.bus.read(zp);
                let hi = self.bus.read(zp.wrapping_add(1) & 0x00ff);
                let base = u16::from_le_bytes([lo, hi]);
                let addr = base.wrapping_add(u16::from(self.y));
                if (base & 0xff00) != (addr & 0xff00) {
                    *cycles += 1;
                }
                Operand::Addr(addr)
            }
            Mode::Rel => {
                let offset = self.fetch8();
                Operand::Addr(self.pc.wrapping_add(i8::from_le_bytes([offset]) as u16))
            }
        }
    }

    fn execute(&mut self, op: Op, mode: Mode, operand: Operand, cycles: &mut u8) {
        // Immediate-mode ops consume the value directly; memory modes read
        // through the bus.
        let addr = match operand {
            Operand::Addr(a) => a,
            Operand::Value(_) => 0,
        };
        let read_operand = |cpu: &mut Self| -> u8 {
            match operand {
                Operand::Value(v) => v,
                Operand::Addr(a) => cpu.bus.read(a),
            }
        };
        match op {
            Op::Brk | Op::Jmp | Op::Jsr | Op::Rti | Op::Rts => unreachable!("handled in step"),

            Op::Lda => {
                let v = read_operand(self);
                self.a = v;
                self.set_zn(v);
            }
            Op::Ldx => {
                let v = read_operand(self);
                self.x = v;
                self.set_zn(v);
            }
            Op::Ldy => {
                let v = read_operand(self);
                self.y = v;
                self.set_zn(v);
            }
            Op::Sta => self.bus.write(addr, self.a),
            Op::Stx => self.bus.write(addr, self.x),
            Op::Sty => self.bus.write(addr, self.y),
            Op::Tax => self.x = self.a,
            Op::Tay => self.y = self.a,
            Op::Txa => self.a = self.x,
            Op::Tya => self.a = self.y,
            Op::Tsx => self.x = self.sp,
            Op::Txs => self.sp = self.x,
            Op::Inx => self.x = self.x.wrapping_add(1),
            Op::Iny => self.y = self.y.wrapping_add(1),
            Op::Dex => self.x = self.x.wrapping_sub(1),
            Op::Dey => self.y = self.y.wrapping_sub(1),
            Op::Pha => self.push(self.a),
            Op::Php => self.push(self.p | flags::BRK | flags::UNUSED),
            Op::Pla => {
                self.a = self.pop();
                self.set_zn(self.a);
            }
            Op::Plp => self.p = self.pop(),
            Op::Clc => self.p &= !flags::CARRY,
            Op::Sec => self.p |= flags::CARRY,
            Op::Cli => self.p &= !flags::IRQ_DISABLE,
            Op::Sei => self.p |= flags::IRQ_DISABLE,
            Op::Clv => self.p &= !flags::OVERFLOW,
            Op::Cld => self.p &= !flags::DECIMAL,
            Op::Sed => self.p |= flags::DECIMAL,
            Op::Nop => {}
            Op::Jam => {
                // Halt: loop forever on the same opcode by not advancing PC.
                self.pc = self.pc.wrapping_sub(1);
            }
            Op::Bpl | Op::Bmi | Op::Bvc | Op::Bvs | Op::Bcc | Op::Bcs | Op::Bne | Op::Beq => {
                let taken = match op {
                    Op::Bpl => self.p & flags::NEGATIVE == 0,
                    Op::Bmi => self.p & flags::NEGATIVE != 0,
                    Op::Bvc => self.p & flags::OVERFLOW == 0,
                    Op::Bvs => self.p & flags::OVERFLOW != 0,
                    Op::Bcc => self.p & flags::CARRY == 0,
                    Op::Bcs => self.p & flags::CARRY != 0,
                    Op::Bne => self.p & flags::ZERO == 0,
                    Op::Beq => self.p & flags::ZERO != 0,
                    _ => unreachable!(),
                };
                if taken {
                    *cycles += 1;
                    let target = addr;
                    if (self.pc & 0xff00) != (target & 0xff00) {
                        *cycles += 1;
                    }
                    self.pc = target;
                }
            }
            Op::And => {
                let v = read_operand(self);
                self.a &= v;
                self.set_zn(self.a);
            }
            Op::Ora => {
                let v = read_operand(self);
                self.a |= v;
                self.set_zn(self.a);
            }
            Op::Eor => {
                let v = read_operand(self);
                self.a ^= v;
                self.set_zn(self.a);
            }
            Op::Adc => {
                let v = read_operand(self);
                self.adc(v);
            }
            Op::Sbc => {
                let v = read_operand(self);
                self.sbc(v);
            }
            Op::Cmp => {
                let v = self.read(addr);
                self.compare(self.a, v);
            }
            Op::Cpx => {
                let v = self.read(addr);
                self.compare(self.x, v);
            }
            Op::Cpy => {
                let v = self.read(addr);
                self.compare(self.y, v);
            }
            Op::Bit => {
                let value = read_operand(self);
                self.p = (self.p & !(flags::NEGATIVE | flags::OVERFLOW | flags::ZERO))
                    | (value & (flags::NEGATIVE | flags::OVERFLOW));
                self.set_z(self.a & value != 0);
            }
            Op::Asl => {
                let value = self.rmw(addr, mode, cycles);
                let shifted = value << 1;
                self.set_c(value & 0x80 != 0);
                self.rmw_write(addr, mode, shifted);
                self.set_zn(shifted);
            }
            Op::Lsr => {
                let value = self.rmw(addr, mode, cycles);
                let shifted = value >> 1;
                self.set_c(value & 0x01 != 0);
                self.rmw_write(addr, mode, shifted);
                self.set_zn(shifted);
            }
            Op::Rol => {
                let value = self.rmw(addr, mode, cycles);
                let carry = u8::from(self.p & flags::CARRY != 0);
                let shifted = (value << 1) | carry;
                self.set_c(value & 0x80 != 0);
                self.rmw_write(addr, mode, shifted);
                self.set_zn(shifted);
            }
            Op::Ror => {
                let value = self.rmw(addr, mode, cycles);
                let carry = u8::from(self.p & flags::CARRY != 0) << 7;
                let shifted = (value >> 1) | carry;
                self.set_c(value & 0x01 != 0);
                self.rmw_write(addr, mode, shifted);
                self.set_zn(shifted);
            }
            Op::Inc => {
                let value = self.rmw(addr, mode, cycles);
                let incremented = value.wrapping_add(1);
                self.rmw_write(addr, mode, incremented);
                self.set_zn(incremented);
            }
            Op::Dec => {
                let value = self.rmw(addr, mode, cycles);
                let decremented = value.wrapping_sub(1);
                self.rmw_write(addr, mode, decremented);
                self.set_zn(decremented);
            }
            // Undocumented combined ops.
            Op::SlO => {
                let value = self.rmw(addr, mode, cycles);
                let shifted = value << 1;
                self.set_c(value & 0x80 != 0);
                self.rmw_write(addr, mode, shifted);
                self.a |= shifted;
                self.set_zn(self.a);
            }
            Op::Rla => {
                let value = self.rmw(addr, mode, cycles);
                let carry = u8::from(self.p & flags::CARRY != 0);
                let shifted = (value << 1) | carry;
                self.set_c(value & 0x80 != 0);
                self.rmw_write(addr, mode, shifted);
                self.a &= shifted;
                self.set_zn(self.a);
            }
            Op::Sre => {
                let value = self.rmw(addr, mode, cycles);
                let shifted = value >> 1;
                self.set_c(value & 0x01 != 0);
                self.rmw_write(addr, mode, shifted);
                self.a ^= shifted;
                self.set_zn(self.a);
            }
            Op::Rra => {
                let value = self.rmw(addr, mode, cycles);
                let carry = u8::from(self.p & flags::CARRY != 0) << 7;
                let shifted = (value >> 1) | carry;
                self.set_c(value & 0x01 != 0);
                self.rmw_write(addr, mode, shifted);
                self.adc(shifted);
            }
            Op::Dcp => {
                let value = self.rmw(addr, mode, cycles);
                let decremented = value.wrapping_sub(1);
                self.rmw_write(addr, mode, decremented);
                self.compare(self.a, decremented);
            }
            Op::Isb => {
                let value = self.rmw(addr, mode, cycles);
                let incremented = value.wrapping_add(1);
                self.rmw_write(addr, mode, incremented);
                self.sbc(incremented);
            }
            Op::Lax => {
                let value = read_operand(self);
                self.a = value;
                self.x = value;
                self.set_zn(value);
            }
            Op::Sax => {
                self.bus.write(addr, self.a & self.x);
            }
        }
    }

    fn adc(&mut self, value: u8) {
        let carry = u16::from(self.p & flags::CARRY != 0);
        let sum = u16::from(self.a) + u16::from(value) + carry;
        let result = sum as u8;
        self.set_c(sum > 0xff);
        self.set_v((self.a ^ result) & (value ^ result) & 0x80 != 0);
        self.a = result;
        self.set_zn(result);
    }

    fn sbc(&mut self, value: u8) {
        let carry = u16::from(self.p & flags::CARRY != 0);
        let diff = u16::from(self.a)
            .wrapping_add(u16::from(!value))
            .wrapping_add(carry);
        let result = diff as u8;
        self.set_c(diff > 0xff);
        self.set_v((self.a ^ result) & (!value ^ result) & 0x80 != 0);
        self.a = result;
        self.set_zn(result);
    }

    fn compare(&mut self, register: u8, value: u8) {
        let diff = register.wrapping_sub(value);
        self.set_c(register >= value);
        self.set_zn(diff);
    }

    fn set_zn(&mut self, value: u8) {
        self.p = (self.p & !(flags::ZERO | flags::NEGATIVE))
            | (u8::from(value == 0) << 1)
            | (value & flags::NEGATIVE);
    }

    fn set_z(&mut self, zero: bool) {
        self.p = (self.p & !flags::ZERO) | (u8::from(zero) << 1);
    }

    fn set_c(&mut self, carry: bool) {
        self.p = (self.p & !flags::CARRY) | u8::from(carry);
    }

    fn set_v(&mut self, overflow: bool) {
        self.p = (self.p & !flags::OVERFLOW) | (u8::from(overflow) << 6);
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    fn fetch8(&mut self) -> u8 {
        let value = self.bus.fetch(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch8();
        let hi = self.fetch8();
        u16::from_le_bytes([lo, hi])
    }

    fn read16(&mut self, addr: u16) -> u16 {
        let lo = self.bus.read(addr);
        let hi = self.bus.read(addr.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    fn push(&mut self, value: u8) {
        self.bus.write(0x0100_u16 | u16::from(self.sp), value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.read(0x0100_u16 | u16::from(self.sp))
    }

    fn push16(&mut self, value: u16) {
        self.push((value >> 8) as u8);
        self.push(value as u8);
    }

    fn pop16(&mut self) -> u16 {
        let lo = self.pop();
        let hi = self.pop();
        u16::from_le_bytes([lo, hi])
    }

    /// Read for read-modify-write ops. The base cycle count in the table
    /// already includes the extra RMW memory cycles (e.g. INC zp = 5).
    fn rmw(&mut self, addr: u16, mode: Mode, cycles: &mut u8) -> u8 {
        let _ = cycles;
        if mode == Mode::Acc {
            return self.a;
        }
        self.bus.read(addr)
    }

    fn rmw_write(&mut self, addr: u16, mode: Mode, value: u8) {
        if mode == Mode::Acc {
            self.a = value;
        } else {
            self.bus.write(addr, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial flat-RAM bus for tests.
    struct RamBus {
        ram: Vec<u8>,
        pub cycles: u64,
    }

    impl RamBus {
        fn new(ram: Vec<u8>) -> Self {
            Self { ram, cycles: 0 }
        }
    }

    impl Bus for RamBus {
        fn fetch(&mut self, addr: u16) -> u8 {
            self.cycles += 1;
            self.ram[usize::from(addr)]
        }
        fn read(&mut self, addr: u16) -> u8 {
            self.cycles += 1;
            self.ram[usize::from(addr)]
        }
        fn write(&mut self, addr: u16, value: u8) {
            self.cycles += 1;
            self.ram[usize::from(addr)] = value;
        }
        fn tick(&mut self, cycles: u8) {
            self.cycles += u64::from(cycles);
        }
    }

    fn cpu_with(program: &[u8], pc: u16) -> Cpu6502<RamBus> {
        let mut ram = vec![0_u8; 0x10000];
        let start = usize::from(pc);
        ram[start..start + program.len()].copy_from_slice(program);
        Cpu6502::new(pc, 0, 0, 0, 0xfd, 0x00, RamBus::new(ram))
    }

    #[test]
    fn lda_immediate_sets_a_and_z() {
        // LDA #$00
        let mut cpu = cpu_with(&[0xa9, 0x00], 0x0800);
        let cycles = cpu.step();
        assert_eq!(cpu.a, 0x00);
        assert_ne!(cpu.p & flags::ZERO, 0);
        assert_eq!(cycles, 2);
        assert_eq!(cpu.pc, 0x0802);
    }

    #[test]
    fn lda_immediate_negative_sets_n() {
        let mut cpu = cpu_with(&[0xa9, 0x80], 0x0800);
        cpu.step();
        assert_eq!(cpu.a, 0x80);
        assert_ne!(cpu.p & flags::NEGATIVE, 0);
    }

    #[test]
    fn adc_with_carry_and_overflow() {
        // ADC #$01 with A=0xFF: sets carry, clears zero, no overflow.
        let mut cpu = cpu_with(&[0x69, 0x01], 0x0800);
        cpu.a = 0xff;
        cpu.step();
        assert_eq!(cpu.a, 0x00);
        assert_ne!(cpu.p & flags::CARRY, 0);
        assert_ne!(cpu.p & flags::ZERO, 0);
        assert_eq!(cpu.p & flags::OVERFLOW, 0);
    }

    #[test]
    fn adc_overflow_positive() {
        // 0x7F + 0x01 = 0x80 -> overflow.
        let mut cpu = cpu_with(&[0x69, 0x01], 0x0800);
        cpu.a = 0x7f;
        cpu.step();
        assert_eq!(cpu.a, 0x80);
        assert_ne!(cpu.p & flags::OVERFLOW, 0);
        assert_ne!(cpu.p & flags::NEGATIVE, 0);
    }

    #[test]
    fn branch_taken_and_untaken_cycles() {
        // BNE +$05 with Z=1: not taken, 2 cycles.
        let mut cpu = cpu_with(&[0xd0, 0x05], 0x0800);
        cpu.p |= flags::ZERO;
        let cycles = cpu.step();
        assert_eq!(cycles, 2);
        assert_eq!(cpu.pc, 0x0802);

        // BNE +$05 with Z=0: taken, 3 cycles.
        let mut cpu = cpu_with(&[0xd0, 0x05], 0x0800);
        cpu.p &= !flags::ZERO;
        let cycles = cpu.step();
        assert_eq!(cycles, 3);
        assert_eq!(cpu.pc, 0x0807);
    }

    #[test]
    fn jsr_rts_round_trip() {
        // JSR $0900 ; RTS
        let mut cpu = cpu_with(&[0x20, 0x00, 0x09, 0x60], 0x0800);
        cpu.step(); // JSR
        assert_eq!(cpu.pc, 0x0900);
        cpu.step(); // whatever at $0900 (0x00 = BRK placeholder) -> skip
                    // Load RTS at $0900 instead by patching:
        let mut cpu = cpu_with(&[0x20, 0x00, 0x09, 0x60], 0x0800);
        cpu.bus.ram[0x0900] = 0x60;
        cpu.step();
        assert_eq!(cpu.pc, 0x0900);
        cpu.step(); // RTS
        assert_eq!(cpu.pc, 0x0803);
    }

    #[test]
    fn sta_absolute_writes_ram() {
        // STA $C000
        let mut cpu = cpu_with(&[0x8d, 0x00, 0xc0], 0x0800);
        cpu.a = 0x42;
        cpu.step();
        assert_eq!(cpu.bus.ram[0xc000], 0x42);
    }

    #[test]
    fn inc_rmw_reads_then_writes() {
        // INC $10 (zp), starting at $10 = 0x05 -> 0x06
        let mut cpu = cpu_with(&[0xe6, 0x10], 0x0800);
        cpu.bus.ram[0x10] = 0x05;
        let cycles = cpu.step();
        assert_eq!(cpu.bus.ram[0x10], 0x06);
        assert_eq!(cycles, 5);
    }

    #[test]
    fn jmp_indirect_page_wrap() {
        // JMP ($10FF): vectors at $10FF and $1000 (page wrap bug)
        let mut cpu = cpu_with(&[0x6c, 0xff, 0x10], 0x0800);
        cpu.bus.ram[0x10ff] = 0x34;
        cpu.bus.ram[0x1000] = 0x12;
        cpu.step();
        assert_eq!(cpu.pc, 0x1234);
    }

    #[test]
    fn undocumented_slo_combines_asl_and_ora() {
        // SLO zp: mem<<1, A |= mem
        let mut cpu = cpu_with(&[0x07, 0x10], 0x0800);
        cpu.bus.ram[0x10] = 0x80;
        cpu.a = 0x01;
        cpu.step();
        assert_eq!(cpu.bus.ram[0x10], 0x00);
        assert_eq!(cpu.a, 0x01);
        assert_ne!(cpu.p & flags::CARRY, 0);
    }

    #[test]
    fn counts_cycles_across_instructions() {
        // LDA #$01 (2) + STA $C000 (4)
        let mut cpu = cpu_with(&[0xa9, 0x01, 0x8d, 0x00, 0xc0], 0x0800);
        let first = cpu.step();
        let second = cpu.step();
        assert_eq!(first, 2);
        assert_eq!(second, 4);
        assert_eq!(cpu.bus.cycles, 6);
    }
}
