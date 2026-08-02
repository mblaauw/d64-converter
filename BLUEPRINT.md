# c64re Blueprint

## Goal

Build a Rust-native reverse-engineering pipeline for C64 `.d64` games that produces structured evidence: disk contents, unpacked memory snapshots, byte provenance, hardware-derived assets, probe findings, disassembly annotations, and a continuously working hybrid clone.

## Architecture

```text
d64 -> disk parser -> extracted files + metadata
    -> VICE binary monitor / embedded core
        -> unpacked RAM snapshot
        -> byte provenance map
        -> per-frame VIC/SID logs
        -> scripted differential probes
    -> coverage-aware disassembly
    -> exact asset rendering
    -> generated reports
    -> hybrid Rust shell
    -> routine-by-routine native replacement
```

## Milestones

1. Instrumented analysis: boot, run, snapshot RAM, and record frame/hardware state.
2. Exact asset capture: render sprites, charsets, screens, and audio streams from observed VIC/SID state.
3. Evidence-backed disassembly: disassemble the unpacked snapshot using executed-byte provenance and coverage.
4. Probe loop: run controlled inputs, diff RAM, set watchpoints, and emit symbol findings.
5. Hybrid shell: embed or drive the original C64 code from a Rust game loop.
6. Native replacement: hook 6502 routine entry points and twin-diff original vs Rust behavior.

## Design Rule

The emulator is the oracle. Static analysis, asset extraction, mechanics inference, and clone validation consume emulator evidence rather than guessing from disk bytes alone.
