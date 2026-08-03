# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/Heart of Africa.d64`
- Disk title: `C64.COM`
- Disk ID: `00`
- DOS type: `2A`
- Directory entries: 4
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 0 | DEL | 18/0 | `----------------` |
| 2111 | PRG | 17/0 | `EA` |
| 2111 | PRG | 17/1 | `LOADER` |
| 0 | DEL | 18/0 | `----------------` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 4 directory entries into `out/hoa_b/disk/files`.
- Captured live VICE RAM after 20 seconds into `snapshots/vice-capture.ram`.
- Collected 100 hardware samples into `traces/hardware-samples.json`.
- Extracted 2 screen blocks, 2 charsets, and 10 displayed sprite blocks into `assets/`.
- Game start (t0) detected at frame 1149: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase.
- Applied 36 joystick input events from the default autoplay script.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

