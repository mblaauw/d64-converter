//! Run the $0B5A decoder on the embedded core with correct setup.
//! Reader $0050: LDA ($F2),Y; INC $F2; BNE +2; INC $F3; RTS  (Y=0)
//! Dest at ($F4),Y with Y advancing; source at $F2.
use c64re_machine::{Backend, C64Machine, RomImages};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prg = std::fs::read("out/karate_rip/disk/files/karate._rip.prg")?;
    let mut ram = vec![0_u8; 65536];
    let payload = &prg[2..];
    ram[0x0801..0x0801 + payload.len()].copy_from_slice(payload);

    // reader at $0050: LDA ($F2),Y; INC $F2; BNE +2; INC $F3; RTS
    let reader = [0xb1, 0xf2, 0xe6, 0xf2, 0xd0, 0x02, 0xe6, 0xf3, 0x60];
    ram[0x0050..0x0050 + reader.len()].copy_from_slice(&reader);
    ram[0x00f2] = 0x4d; // source low $104D
    ram[0x00f3] = 0x10; // source high
    ram[0x00f4] = 0x00; // dest low $0D00
    ram[0x00f5] = 0x0d; // dest high

    let rom_dir = c64re_machine::discover_vice_rom_dir().ok_or("ROMs not found")?;
    let roms = RomImages::load_from_vice_share(&rom_dir)?;
    let mut machine = C64Machine::from_snapshot(ram, &roms, 0x0b5a);
    machine.run_cycles(500_000);
    let ram = machine.ram_snapshot().to_vec();
    let out = &ram[0x0d00..0x0d00 + 63];
    println!("decoded $0D00: {:02x?}", &out[..30]);
    // compare to expected $20C0 sprite (from the VICE capture)
    let expected =
        &std::fs::read("out/karate_rip/snapshots/vice-capture.ram").unwrap()[0x20c0..0x20c0 + 63];
    println!("expected $20C0: {:02x?}", &expected[..30]);
    let match_count = out
        .iter()
        .zip(expected.iter())
        .filter(|(a, b)| a == b)
        .count();
    println!("match: {match_count}/63");
    Ok(())
}
