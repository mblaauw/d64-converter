# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 50
- First screen base: $0000
- First charset base: $0000
- First sprite pointer table: $03f8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10 | $fd77 | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 1 | 20 | $fd7a | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 2 | 30 | $fd79 | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 3 | 40 | $fd7c | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 4 | 50 | $fd7e | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 5 | 60 | $fd7c | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 6 | 70 | $fd7e | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 7 | 80 | $fd81 | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 8 | 90 | $fd80 | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 9 | 100 | $fd83 | standard-text | $01 | $0000 | $0000 | 0 | $00 | 0 |
| 10 | 110 | $e5cf | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 11 | 120 | $e5cf | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 12 | 130 | $e5d4 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 13 | 140 | $e5d4 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 14 | 150 | $e5d4 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 15 | 160 | $eea9 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 16 | 170 | $ed5a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 17 | 180 | $eeb2 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 18 | 190 | $eeac | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 19 | 200 | $eea9 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 20 | 210 | $ed5d | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 21 | 220 | $eeb1 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 22 | 230 | $eeac | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 23 | 240 | $ed5a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 24 | 250 | $ee6a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 25 | 260 | $ee60 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 26 | 270 | $ee5a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 27 | 280 | $eeaf | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 28 | 290 | $ee6a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 29 | 300 | $eeaf | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 30 | 310 | $ee63 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 31 | 320 | $ee5d | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 32 | 330 | $eeac | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 33 | 340 | $ee62 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 34 | 350 | $eea9 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 35 | 360 | $ee60 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 36 | 370 | $ee67 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 37 | 380 | $eeaf | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 38 | 390 | $ee60 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 39 | 400 | $ee5a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 40 | 410 | $ee1b | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 41 | 420 | $eeb1 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 42 | 430 | $ee67 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 43 | 440 | $ee67 | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 44 | 450 | $eeac | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 45 | 460 | $ee6d | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 46 | 470 | $ee1b | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 47 | 480 | $ee5d | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 48 | 490 | $ee5a | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
| 49 | 500 | $ee1e | standard-text | $15 | $0400 | $1000 | 0 | $06 | 0 |
