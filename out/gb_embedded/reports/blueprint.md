# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/Ghostbusters.d64`
- Disk title: `GB`
- Disk ID: `00`
- DOS type: `2A`
- Directory entries: 1
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 130 | PRG | 5/0 | `GHOSTBUSTERS` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 1 directory entries into `out/gb_embedded/disk/files`.
- Captured live VICE RAM after 8 seconds into `snapshots/vice-capture.ram`.
- Collected 40 hardware samples into `traces/hardware-samples.json`.
- Extracted 1 screen blocks, 1 charsets, and 4 displayed sprite blocks into `assets/`.
- Game start (t0) detected at frame 4287: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase.
- Applied 14 joystick input events from the default autoplay script.
- Embedded 6502 core: 248 executed bytes across 48 ranges (true per-byte provenance).
- Embedded disassembly: 248 executed instructions.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

