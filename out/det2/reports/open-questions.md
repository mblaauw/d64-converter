# Open Questions

Each question lists the specific evidence needed to close it.

| # | Question | Evidence needed |
| ---: | --- | --- |
| 1 | Which file is the boot entry point? | Directory `file_index` of the autostarted file; first executed PC after autostart; `CpuHistory` PC list. |
| 2 | Does the game use a cruncher, fastloader, or custom loader? | Bytes executed before the first screen change; `$D018`/`$D011` deltas between boot and `t0`; IRQ vector writes. |
| 3 | Which joystick port and input patterns are active? | Frame-numbered input log; `$DC00`/`$DC01` reads at `t0`; response of game state to probe inputs. |
| 4 | Which memory ranges become stable after decrunching? | RAM diff between two runs at the same frame; per-frame write-watchpoint log. |

No frames captured in this session; none of the above can be answered from this run alone.
