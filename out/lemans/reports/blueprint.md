# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/Le Mans.d64`
- Disk title: `C64.COM`
- Disk ID: `00`
- DOS type: `2A`
- Directory entries: 1
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 33 | PRG | 17/0 | `LE MANS` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 1 directory entries into `out/lemans/disk/files`.
- Captured live VICE RAM after 15 seconds into `snapshots/vice-capture.ram`.
- Collected 75 hardware samples into `traces/hardware-samples.json`.
- Extracted 1 screen blocks, 1 charsets, and 0 displayed sprite blocks into `assets/`.
- Game start (t0) detected at frame 1327: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase.
- Applied 27 joystick input events from the default autoplay script.
- Probe experiments found 73 input-sensitive ranges.
- Execution coverage: 103 executed bytes.
- Disassembled 103 executed instructions into `reports/disassembly.md`.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

