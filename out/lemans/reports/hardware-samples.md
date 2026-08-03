# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 75
- First screen base: $0400
- First charset base: $3800
- First sprite pointer table: $07f8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 1 | 20 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 2 | 30 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 3 | 40 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 4 | 50 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 5 | 60 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 6 | 70 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 7 | 80 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 8 | 90 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 9 | 100 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 10 | 110 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 11 | 120 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 12 | 130 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 13 | 140 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 14 | 150 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 15 | 160 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 16 | 170 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 17 | 180 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 18 | 190 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 19 | 200 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 20 | 210 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 21 | 220 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 22 | 230 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 23 | 240 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 24 | 250 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 25 | 260 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 26 | 270 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 27 | 280 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 28 | 290 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 29 | 300 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 30 | 310 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 31 | 320 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 32 | 330 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 33 | 340 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 34 | 350 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 35 | 360 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 36 | 370 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 37 | 380 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 38 | 390 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 39 | 400 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 40 | 410 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 41 | 420 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 42 | 430 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 43 | 440 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 44 | 450 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 45 | 460 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 46 | 470 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 47 | 480 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 48 | 490 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 49 | 500 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 50 | 510 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 51 | 520 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 52 | 530 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 53 | 540 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 54 | 550 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 55 | 560 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 56 | 570 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 57 | 580 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 58 | 590 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 59 | 600 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 60 | 610 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 61 | 620 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 62 | 630 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 63 | 640 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 64 | 650 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 65 | 660 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 66 | 670 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 67 | 680 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 68 | 690 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 69 | 700 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 70 | 710 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 71 | 720 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 72 | 730 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 73 | 740 | $e461 | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
| 74 | 750 | $e45f | multicolor-text | $1f | $0400 | $3800 | 0 | $00 | 0 |
