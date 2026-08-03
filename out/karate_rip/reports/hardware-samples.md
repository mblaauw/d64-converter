# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 200
- First screen base: $0400
- First charset base: $1000
- First sprite pointer table: $07f8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 5 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 1 | 10 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 2 | 15 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 3 | 20 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 4 | 25 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 5 | 30 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 6 | 35 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 7 | 40 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 8 | 45 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 9 | 50 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 10 | 55 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 11 | 60 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 12 | 65 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 13 | 70 | $eb74 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 14 | 75 | $eab7 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 15 | 80 | $ead5 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 16 | 85 | $eab7 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 17 | 90 | $eacf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 18 | 95 | $eac9 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 19 | 100 | $eab6 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 20 | 105 | $eab1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 21 | 110 | $eacb | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 22 | 115 | $eac9 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 23 | 120 | $eab6 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 24 | 125 | $ea6b | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 25 | 130 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 26 | 135 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 27 | 140 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 28 | 145 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 29 | 150 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 30 | 155 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 31 | 160 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 32 | 165 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 33 | 170 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 34 | 175 | $eab6 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 35 | 180 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 36 | 185 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 37 | 190 | $eacd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 38 | 195 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 39 | 200 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 40 | 205 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 41 | 210 | $eab1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 42 | 215 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 43 | 220 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 44 | 225 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 45 | 230 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 46 | 235 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 47 | 240 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 48 | 245 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 49 | 250 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 50 | 255 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 51 | 260 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 52 | 265 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 53 | 270 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 54 | 275 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 55 | 280 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 56 | 285 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 57 | 290 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 58 | 295 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 59 | 300 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 60 | 305 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 61 | 310 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 62 | 315 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 63 | 320 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 64 | 325 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 65 | 330 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 66 | 335 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 67 | 340 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 68 | 345 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 69 | 350 | $e5cf | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 70 | 355 | $e5d4 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 71 | 360 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 72 | 365 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 73 | 370 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 74 | 375 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 75 | 380 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 76 | 385 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 77 | 390 | $e5d1 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 78 | 395 | $eb26 | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |
| 79 | 400 | $e5cd | standard-text | $15 | $0400 | $1000 | 8 | $06 | 16 |

Only the first 80 samples are shown. 120 samples omitted.
