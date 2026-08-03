# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/International Karate Plus.d64`
- Disk title: `WWW.C64HQ.COM`
- Disk ID: `00`
- DOS type: `2A`
- Directory entries: 8
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 0 | DEL | 18/0 | `----------------` |
| 193 | PRG | 9/0 | `IK. .HI 100..REM` |
| 81 | PRG | 29/0 | `IK. INTROS  .REM` |
| 1 | PRG | 33/4 | `IK. - TOP 30.REM` |
| 0 | DEL | 18/0 | `----------------` |
| 169 | PRG | 17/0 | `I.KARATE 2...FLT` |
| 181 | PRG | 19/0 | `INTER.KARATE II.` |
| 0 | DEL | 18/0 | `----------------` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 8 directory entries into `out/ikplus5/disk/files`.
- Captured live VICE RAM after 40 seconds into `snapshots/vice-capture.ram`.
- Collected 200 hardware samples into `traces/hardware-samples.json`.
- Extracted 1 screen blocks, 1 charsets, and 0 displayed sprite blocks into `assets/`.
- Game start (t0) detected at frame 20: IRQ vector left the KERNAL default and the screen base left $0400. Earlier frames are the loader phase.
- Applied 72 joystick input events from the default autoplay script.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

