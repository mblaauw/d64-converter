# C64 Reverse-Engineering Blueprint

## Source

- Disk image: `/Users/mich/Downloads/Ghostbusters.d64`
- Disk title: `WWW.C64HQ.COM`
- Disk ID: `00`
- DOS type: `2A`
- Directory entries: 1
- Captured frames: none (frame-stepped capture not yet run)

## Disk Directory

| Blocks | Type | First T/S | Name |
| ---: | --- | --- | --- |
| 128 | PRG | 17/0 | `GHOSTBUSTERS` |

## Provenance

- Not collected yet: provenance requires an instrumented capture (see T13 work items).

## Current Findings

- Extracted 1 directory entries into `out/gb7/disk/files`.
- Captured live VICE RAM after 10 seconds into `snapshots/vice-capture.ram`.
- Collected 50 hardware samples into `traces/hardware-samples.json`.
- Extracted 2 screen blocks, 2 charsets, and 0 displayed sprite blocks into `assets/`.
- No game start (t0) detected: capture stayed in the KERNAL/loader phase for the whole run.
- Applied 18 joystick input events from the default autoplay script.

## Open Questions

See `open-questions.md` for the open questions and the specific evidence needed to close each one.

