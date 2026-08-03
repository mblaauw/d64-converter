# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/Karate_Rip.d64`
- Disk title: `KARATE. RIP`
- Disk ID: `10`
- DOS type: ``
- Directory entries: 1
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 13 | PRG | 17/0 | `KARATE. RIP` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 1 directory entries into `out/karate_rip2/disk/files`.
- Captured live VICE RAM after 60 seconds into `snapshots/vice-capture.ram`.
- Collected 600 hardware samples into `traces/hardware-samples.json`.
- Extracted 1 screen blocks, 1 charsets, and 8 displayed sprite blocks into `assets/`.
- Game start (t0) detected at frame 712: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase.
- Execution coverage: 73 executed bytes.
- Disassembled 73 executed instructions into `reports/disassembly.md`.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

