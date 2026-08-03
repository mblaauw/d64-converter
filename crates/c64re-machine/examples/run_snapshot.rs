//! Run the embedded core against a saved RAM snapshot + PC.
//! Usage: run_snapshot <ram.bin> <pc-hex> [cycles]
use c64re_machine::{Backend, C64Machine, RomImages};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let ram_path = args
        .next()
        .unwrap_or_else(|| "out/ikplus_live/game.ram".into());
    let pc: u16 = args
        .next()
        .map(|s| u16::from_str_radix(&s, 16).unwrap())
        .unwrap_or(0xf1b5);
    let cycles: u64 = args
        .next()
        .map(|s| s.parse().unwrap())
        .unwrap_or(10_000_000);

    let rom_dir = c64re_machine::discover_vice_rom_dir().ok_or("ROMs not found")?;
    let roms = RomImages::load_from_vice_share(&rom_dir)?;
    let ram = std::fs::read(&ram_path)?;
    let mut machine = C64Machine::from_snapshot(ram, &roms, pc);
    println!("running {cycles} cycles from pc=${pc:04x}...");
    machine.run_cycles(cycles);
    let provenance = machine.provenance().clone();
    let counts = provenance.counts();
    println!("executed bytes: {}", counts.executed);
    println!("read bytes:     {}", counts.cpu_read);
    println!("written bytes:  {}", counts.cpu_written);
    println!("write-then-exec:{}", counts.write_then_execute);
    let ranges = c64re_disasm::executed_ranges(&provenance);
    println!("executed ranges: {}", ranges.len());
    for range in ranges.iter().take(30) {
        println!(
            "  ${:04x}-${:04x} ({} bytes)",
            range.start,
            range.end_inclusive,
            range.len()
        );
    }
    let ram = machine.ram_snapshot().to_vec();
    let lines = c64re_disasm::disassemble_executed(&ram, &provenance, 200);
    println!("\n=== disassembly ({} instructions) ===", lines.len());
    for line in lines.iter().take(60) {
        println!("{}", line.render());
    }
    Ok(())
}
