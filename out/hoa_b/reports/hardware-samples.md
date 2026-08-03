# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 100
- First screen base: $e400
- First charset base: $e000
- First sprite pointer table: $e7f8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10 | $f543 | multicolor-text | $99 | $e400 | $e000 | 0 | $0e | 0 |
| 1 | 20 | $f540 | multicolor-text | $99 | $e400 | $e000 | 0 | $0c | 0 |
| 2 | 30 | $f543 | multicolor-text | $99 | $e400 | $e000 | 0 | $0a | 0 |
| 3 | 40 | $f543 | multicolor-text | $99 | $e400 | $e000 | 0 | $08 | 0 |
| 4 | 50 | $f540 | multicolor-text | $99 | $e400 | $e000 | 0 | $06 | 0 |
| 5 | 60 | $f508 | multicolor-text | $99 | $e400 | $e000 | 0 | $0b | 0 |
| 6 | 70 | $f508 | multicolor-text | $99 | $e400 | $e000 | 0 | $09 | 0 |
| 7 | 80 | $f502 | multicolor-text | $99 | $e400 | $e000 | 0 | $0e | 0 |
| 8 | 90 | $f543 | multicolor-text | $99 | $e400 | $e000 | 0 | $02 | 0 |
| 9 | 100 | $f540 | multicolor-text | $99 | $e400 | $e000 | 0 | $0e | 0 |
| 10 | 110 | $f540 | multicolor-text | $99 | $e400 | $e000 | 0 | $0c | 0 |
| 11 | 120 | $f540 | multicolor-text | $99 | $e400 | $e000 | 0 | $0a | 0 |
| 12 | 130 | $f543 | multicolor-text | $99 | $e400 | $e000 | 0 | $08 | 0 |
| 13 | 140 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 12 |
| 14 | 150 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 15 |
| 15 | 160 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 15 |
| 16 | 170 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 17 |
| 17 | 180 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 18 | 190 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 19 | 200 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 20 | 210 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 21 | 220 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 22 | 230 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 23 | 240 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 24 | 250 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 25 | 260 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $01 | 18 |
| 26 | 270 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 18 |
| 27 | 280 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 28 | 290 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 29 | 300 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 21 |
| 30 | 310 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 31 | 320 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 32 | 330 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 33 | 340 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 34 | 350 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 35 | 360 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 36 | 370 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 37 | 380 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 38 | 390 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $02 | 22 |
| 39 | 400 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 40 | 410 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 41 | 420 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 42 | 430 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 43 | 440 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 44 | 450 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 45 | 460 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 20 |
| 46 | 470 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 47 | 480 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 48 | 490 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 49 | 500 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 50 | 510 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 51 | 520 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 0 | $03 | 22 |
| 52 | 530 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 53 | 540 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 54 | 550 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 55 | 560 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 56 | 570 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 57 | 580 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 58 | 590 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 1 | $04 | 22 |
| 59 | 600 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 60 | 610 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 61 | 620 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 21 |
| 62 | 630 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 63 | 640 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 64 | 650 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 65 | 660 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 66 | 670 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 67 | 680 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 68 | 690 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 69 | 700 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 70 | 710 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 71 | 720 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $04 | 22 |
| 72 | 730 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 73 | 740 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 74 | 750 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 75 | 760 | $0d50 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 76 | 770 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 77 | 780 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 19 |
| 78 | 790 | $0d4d | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |
| 79 | 800 | $0d52 | multicolor-bitmap | $2d | $8800 | $b000 | 4 | $05 | 22 |

Only the first 80 samples are shown. 20 samples omitted.
