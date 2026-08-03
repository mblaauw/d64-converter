# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 60
- First screen base: $0400
- First charset base: $0800
- First sprite pointer table: $07f8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 1 | 20 | $ff48 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 2 | 30 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 3 | 40 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 4 | 50 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 5 | 60 | $2f52 | multicolor-text | $19 | $0400 | $2000 | 6 | $07 | 23 |
| 6 | 70 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 7 | 80 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 8 | 90 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 19 |
| 9 | 100 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 10 | 110 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 20 |
| 11 | 120 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 12 | 130 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 13 | 140 | $ff48 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 14 | 150 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 21 |
| 15 | 160 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 16 | 170 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 17 | 180 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 18 | 190 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 19 | 200 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 20 | 210 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 21 |
| 21 | 220 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 22 | 230 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 23 | 240 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 24 | 250 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 25 | 260 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 26 | 270 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 27 | 280 | $ff48 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 28 | 290 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 29 | 300 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 30 | 310 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 31 | 320 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 32 | 330 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 33 | 340 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 21 |
| 34 | 350 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 18 |
| 35 | 360 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 36 | 370 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 37 | 380 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 38 | 390 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 39 | 400 | $ff48 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 40 | 410 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 41 | 420 | $ff48 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 42 | 430 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 43 | 440 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 44 | 450 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 45 | 460 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 46 | 470 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 47 | 480 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 21 |
| 48 | 490 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 49 | 500 | $2f55 | multicolor-text | $19 | $0400 | $2000 | 6 | $07 | 23 |
| 50 | 510 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 51 | 520 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 52 | 530 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 53 | 540 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 22 |
| 54 | 550 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 55 | 560 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 56 | 570 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 23 |
| 57 | 580 | $2e1b | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 58 | 590 | $2e1e | standard-text | $13 | $0400 | $0800 | 6 | $00 | 24 |
| 59 | 600 | $2e20 | standard-text | $13 | $0400 | $0800 | 6 | $00 | 21 |
