//! Disassemble a RAM region from a file using the project's decoder.
use c64re_disasm::disassemble;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "/tmp/karate_rip_full.ram".into());
    let start = u16::from_str_radix(&args.next().unwrap_or_else(|| "0c00".into()), 16).unwrap();
    let count: usize = args.next().map(|s| s.parse().unwrap()).unwrap_or(100);
    let ram = std::fs::read(&path).unwrap();
    let lines = disassemble(&ram, start, count);
    for line in lines.iter() {
        println!("{}", line.render());
    }
}
