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

## What works today

- **`c64re-d64`**: `.d64` parsing (35/40-track, with or without error-byte block),
  directory + file-chain extraction, and a `D64Builder` for synthetic test
  images (no copyrighted fixtures in the repo).
- **`c64re-vice-bmp`**: a hand-rolled VICE binary-monitor client — event pump
  (checkpoint/stop events are queued, not dropped), Dump/Undump savestates,
  DisplayGet screenshots, bank/register discovery, CPU history, non-stopping
  watchpoints, raster conditions, resource get/set. Protocol-tested.
- **`c64re-cli analyze --vice`**: deterministic, frame-stepped capture:
  1. Autostart the disk through the monitor (working copy, so the source
     image is never modified).
  2. Detect game start (t0) — screen base leaves `$0400`, PC leaves ROM.
  3. Settle 900 frames, then Dump/Undump a savestate as the deterministic
     anchor (VICE's drive I/O under warp is not cycle-deterministic).
  4. Frame-step by VIC raster wrap; input scripted by frame number.
  5. Carve screen/charset/sprite/bitmap bytes **at observation time**, dedupe
     sprites by content hash, render in the correct display mode
     (text / multicolor / ECM / hires / multicolor-bitmap) with color RAM.
  6. Read RAM from the true `ram` bank; source chargen from ROM.
  7. Optionally harvest SID write activity (`--sid-seconds`) from
     non-stopping write watchpoints — SID registers are write-only, so
     register-read dumps are open-bus garbage.
- Reports: `blueprint.md`, `open-questions.md` (with the evidence needed to
  close each), `hardware-samples.md` (one row per sample with display mode),
  `assets.md`, `sid-writes.md`, `ram-diff.md`, `session.json`.
- `--probe`: runs controlled-input experiments (hold-left/right/up/down,
  fire) against the t0 savestate and diffs each run's RAM against an idle
  baseline → `probe-findings.json/md` (input-sensitive memory ranges).
- `--provenance`: replays the savestate with the autoplay script and harvests
  executed PCs from `CpuHistory` into an approximate coverage map →
  `provenance.json/md` (executed ranges).
- `--disasm`: coverage-seeded linear disassembly of executed code (real 6502
  decoder) → `disassembly.json/md`.

Caveat: the deterministic anchor is the savestate taken after the settle
period. If a game crashes or idles in a tight loop by then (some titles do —
reset vector `$0000`, BRK loops), the replay, probes, and provenance honestly
report that state: zero probe findings and a 1-2 byte coverage map are valid
signals, not bugs.

Verified on Ghostbusters and International Karate Plus: two consecutive runs
produce byte-identical `hardware-samples.json` and `input-events.json`.

## Embedded core & hybrid shell

- `c64re-cpu`: a real NMOS 6502 core (official + common undocumented
  opcodes, cycle timing, page-cross penalties).
- `c64re-machine`: C64 memory map with `$01` banking and a
  provenance-recording bus; `--embedded` runs the core against the captured
  RAM snapshot and reports true per-byte executed/read/written/
  write-then-execute ranges (self-modifying code detection).
- `c64re-hybrid`: windowed playback of captured frames (macroquad), with
  modern keyboard mapped to C64 joystick bits:
  `cargo run -p c64re-hybrid --bin replay -- out/game`
  (WASD/arrows = joystick, SPACE = fire, TAB = autoplay, R = restart).

The `Backend` trait (machine crate) lets callers use either the embedded
core or VICE interchangeably; `ViceBackend` (capture crate) is the
compatibility fallback.

Other data-model crates: `c64re-provenance`, `c64re-trace` (byte-level
provenance and frame traces), `c64re-probes` (probe experiments),
`c64re-disasm` (real NMOS 6502 decoder; `--disasm` does a coverage-seeded
linear sweep of executed code).

## Usage

```bash
cargo run -p c64re-cli -- disk path/to/game.d64
cargo run -p c64re-cli -- analyze path/to/game.d64 --out out/game \
  --vice --seconds 10 --sample-hz 5 --autoplay
```

`analyze` parses the disk, extracts files, and (with `--vice`) runs the
capture pipeline above. Requires a VICE `x64sc` on `PATH`.

## License / content policy

The repo ships no game images or extracted game content. `out/` is
git-ignored; analyses are private-use only. Test fixtures are synthetic
(`D64Builder`).
