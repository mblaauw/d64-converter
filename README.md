# d64-converter

An experiment in C64 game archaeology: analyze a `.d64`, run it under an instrumented emulator, collect evidence-backed traces/assets, and use that data to build a hybrid Rust clone that can be rewritten routine by routine.

This is not a magic source converter. The intended workflow is:

```text
.d64
  -> disk parse
  -> instrumented emulation
  -> unpacked RAM snapshot
  -> provenance map
  -> exact VIC/SID capture
  -> probe experiments
  -> evidence-backed disassembly
  -> blueprint/report
  -> hybrid clone shell
```

## Current State

The initial workspace contains:

- `c64re-d64`: real `.d64` directory parsing and file-chain extraction.
- `c64re-provenance`: byte-level provenance flags.
- `c64re-vic` / `c64re-sid`: hardware-state data models.
- `c64re-trace`: analysis session and frame trace models.
- `c64re-report`: Markdown report generation.
- `c64re-cli`: first CLI commands.
- `c64re-vice-bmp`, `c64re-probes`, `c64re-disasm`, `c64re-assets`, `c64re-hybrid`: compile-ready scaffolding for the next milestones.

## Usage

```bash
cargo run -p c64re-cli -- disk path/to/game.d64
cargo run -p c64re-cli -- analyze path/to/game.d64 --out out/game
```

`analyze` currently performs the disk parse and writes a first `reports/blueprint.md`. VICE execution, provenance collection, and exact VIC/SID asset extraction are the next implementation steps.
