# Hardware Samples

Frame-stepped VICE binary-monitor polls of VIC-II, SID, and sprite pointer state. Each sample stops the emulator at the KERNAL IRQ entry (one PAL frame), reads state, then resumes.

- Samples: 40
- First screen base: $0c00
- First charset base: $3000
- First sprite pointer table: $0ff8

| # | frame | PC | mode | D018 | screen | charset | sprites | bg | SID nonzero |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 1 | 20 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 2 | 30 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 3 | 40 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 4 | 50 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 5 | 60 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 6 | 70 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 7 | 80 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 8 | 90 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 9 | 100 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 10 | 110 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 11 | 120 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 12 | 130 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 13 | 140 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 14 | 150 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 15 | 160 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 16 | 170 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 17 | 180 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 18 | 190 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 19 | 200 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 20 | 210 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 21 | 220 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 22 | 230 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 23 | 240 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 24 | 250 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 25 | 260 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 26 | 270 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 27 | 280 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 28 | 290 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 29 | 300 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 30 | 310 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 31 | 320 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 32 | 330 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 33 | 340 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 34 | 350 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 35 | 360 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 36 | 370 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 37 | 380 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 38 | 390 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
| 39 | 400 | $0002 | multicolor-bitmap | $3c | $0c00 | $3000 | 4 | $0c | 25 |
