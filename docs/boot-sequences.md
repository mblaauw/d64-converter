# Game Boot Sequences

How each tested game reaches actual gameplay, for the capture pipeline.

## International Karate Plus (`International Karate Plus.d64`)

**Disk**: C64HQ crack release (disk title `WWW.C64HQ.COM`). Has a crack
intro with a trainer menu — this is NOT the game.

**Files** (autostart = first PRG with BASIC header):

| File | Load | BASIC | Role |
| --- | ---: | --- | --- |
| `IK. .HI 100..REM` | $0801 | SYS 2059 | The game (48 KB, decruncher) |
| `IK. INTROS .REM` | $0801 | SYS 2059 | Intro file |
| `IK. - TOP 30.REM` | $FDB3 | — | High-score table |
| `I.KARATE 2...FLT` | $0800 | — | Fastloader variant |
| `INTER.KARATE II.` | $0801 | SYS 2064 | Variant |

**Key sequence to reach gameplay** (SPACE/ESC on the intro, ESC after the
loader, Y to flush the highscore):

1. Title screen appears (boxer art, PC in the $2e00-$2e30 wait loop,
   d018=$13). The wait loop is `LDA $DC01 / CMP #$EF / BNE` — it polls
   **$DC01 for fire** (joystick port 1).
2. **SPACE**, then **ESC** (sequential, not together) — skips the crack
   intro. PC leaves $2e00 for $0340+ (game loader, d018=$95).
3. Wait for the load.
4. **ESC** — instruction screen.
5. Wait for the game.
6. **Y** — flush the highscore table.
7. Gameplay starts (game code at $ee00+, d018=$17/$bb).

**Critical quirk**: the intro wait loop reads $DC01 directly (not via
KERNAL), so the binary monitor's `keyboard_feed` (which feeds the KERNAL
keyboard buffer) does NOT work for it. The monitor's `joyport_set` only
routes with the io device attached, and the io device pollutes $DC01 reads
(bits 5-7 masked by the keyboard matrix — $DC01 reads 0x0f, never the $EF
the game expects). The only reliable paths:

- **Manual play** with the window focused (VICE's keyset maps SPACE to
  fire on the joystick port — what a human does naturally).
- **osascript keystrokes** to the focused VICE window (same keyset path).
- **Capture the live state**: play past the intro once, then dump
  `snapshots/vice-capture.ram` (e.g. via the `dump_state` example) and
  run the pipeline's `--embedded` core against it.

**Boot timing** (observed, stepped capture): load $ee00 (d018=$15) until
~frame 7000, decrunch ~7100-7400, intro d018=$75 at ~7500, title loop
d018=$13 from ~7700. t0 fires at the intro ($75) before the title, so the
boot script must hold the t0 heuristic (post-grace) until the keys have
advanced the game.

## Ghostbusters (`Ghostbusters.d64`)

No intro keys needed. Autostart runs to gameplay directly.

## Le Mans (`Le Mans.d64`)

Tiny BASIC+ML game. No intro keys needed; waits for a keyboard key during
gameplay (KERNAL CHRIN — `keyboard_feed` works here).

## Heart of Africa (`Heart of Africa.d64`)

Multi-file (EA + LOADER). No intro keys needed; capture is non-deterministic
(timing-dependent poll loop).
