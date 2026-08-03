# Observed Assets

Assets extracted from `snapshots/vice-capture.ram` using sampled VIC-II state as the source of truth.

- Manifest: `assets/manifest.json`
- Screen blocks: 5
- Charset blocks: 5
- Displayed sprite blocks: 39

## Screens

| Address | Sample | Raw | Preview | Note |
| ---: | ---: | --- | --- | --- |
| $0000 | 0 | `assets/screens/screen-0000.bin` | `assets/screens/screen-0000.png` | rendered with charset $0000 |
| $0400 | 10 | `assets/screens/screen-0400.bin` | `assets/screens/screen-0400.png` | rendered with charset $1000 |
| $3c00 | 633 | `assets/screens/screen-3c00.bin` | `assets/screens/screen-3c00.png` | rendered with charset $3800 |
| $5c00 | 674 | `assets/screens/screen-5c00.bin` | `assets/screens/screen-5c00.png` | rendered with charset $5000 |
| $2400 | 687 | `assets/screens/screen-2400.bin` | `assets/screens/screen-2400.png` | rendered with charset $1000 |

## Charsets

| Address | Sample | Raw | Preview | Note |
| ---: | ---: | --- | --- | --- |
| $0000 | 0 | `assets/charsets/charset-0000.bin` | `assets/charsets/charset-0000.png` | - |
| $1000 | 10 | `assets/charsets/charset-1000.bin` | `assets/charsets/charset-1000.png` | - |
| $3800 | 633 | `assets/charsets/charset-3800.bin` | `assets/charsets/charset-3800.png` | - |
| $5000 | 674 | `assets/charsets/charset-5000.bin` | `assets/charsets/charset-5000.png` | - |
| $0800 | 688 | `assets/charsets/charset-0800.bin` | `assets/charsets/charset-0800.png` | - |

## Sprites

| Address | Sample | Raw | Preview | Note |
| ---: | ---: | --- | --- | --- |
| $3f00 | 635 | `assets/sprites/sprite-3f00-s0.bin` | `assets/sprites/sprite-3f00-s0.png` | - |
| $1c40 | 635 | `assets/sprites/sprite-1c40-s1.bin` | `assets/sprites/sprite-1c40-s1.png` | - |
| $0980 | 635 | `assets/sprites/sprite-0980-s2.bin` | `assets/sprites/sprite-0980-s2.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $3d80 | 635 | `assets/sprites/sprite-3d80-s3.bin` | `assets/sprites/sprite-3d80-s3.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $1200 | 635 | `assets/sprites/sprite-1200-s4.bin` | `assets/sprites/sprite-1200-s4.png` | - |
| $3c40 | 635 | `assets/sprites/sprite-3c40-s5.bin` | `assets/sprites/sprite-3c40-s5.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $1840 | 635 | `assets/sprites/sprite-1840-s6.bin` | `assets/sprites/sprite-1840-s6.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $2680 | 635 | `assets/sprites/sprite-2680-s7.bin` | `assets/sprites/sprite-2680-s7.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $5400 | 674 | `assets/sprites/sprite-5400-s0.bin` | `assets/sprites/sprite-5400-s0.png` | - |
| $6ec0 | 674 | `assets/sprites/sprite-6ec0-s1.bin` | `assets/sprites/sprite-6ec0-s1.png` | - |
| $6ac0 | 674 | `assets/sprites/sprite-6ac0-s2.bin` | `assets/sprites/sprite-6ac0-s2.png` | - |
| $6900 | 674 | `assets/sprites/sprite-6900-s3.bin` | `assets/sprites/sprite-6900-s3.png` | - |
| $75c0 | 674 | `assets/sprites/sprite-75c0-s4.bin` | `assets/sprites/sprite-75c0-s4.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $7180 | 674 | `assets/sprites/sprite-7180-s6.bin` | `assets/sprites/sprite-7180-s6.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $6a80 | 674 | `assets/sprites/sprite-6a80-s7.bin` | `assets/sprites/sprite-6a80-s7.png` | - |
| $4800 | 677 | `assets/sprites/sprite-4800-s0.bin` | `assets/sprites/sprite-4800-s0.png` | - |
| $4240 | 677 | `assets/sprites/sprite-4240-s1.bin` | `assets/sprites/sprite-4240-s1.png` | - |
| $44c0 | 677 | `assets/sprites/sprite-44c0-s2.bin` | `assets/sprites/sprite-44c0-s2.png` | - |
| $4800 | 677 | `assets/sprites/sprite-4800-s3.bin` | `assets/sprites/sprite-4800-s3.png` | - |
| $4380 | 677 | `assets/sprites/sprite-4380-s4.bin` | `assets/sprites/sprite-4380-s4.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $4500 | 677 | `assets/sprites/sprite-4500-s6.bin` | `assets/sprites/sprite-4500-s6.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $4800 | 677 | `assets/sprites/sprite-4800-s7.bin` | `assets/sprites/sprite-4800-s7.png` | - |
| $4140 | 682 | `assets/sprites/sprite-4140-s0.bin` | `assets/sprites/sprite-4140-s0.png` | - |
| $4480 | 682 | `assets/sprites/sprite-4480-s1.bin` | `assets/sprites/sprite-4480-s1.png` | - |
| $4140 | 682 | `assets/sprites/sprite-4140-s2.bin` | `assets/sprites/sprite-4140-s2.png` | - |
| $4380 | 682 | `assets/sprites/sprite-4380-s3.bin` | `assets/sprites/sprite-4380-s3.png` | - |
| $4500 | 682 | `assets/sprites/sprite-4500-s4.bin` | `assets/sprites/sprite-4500-s4.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $4340 | 682 | `assets/sprites/sprite-4340-s6.bin` | `assets/sprites/sprite-4340-s6.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $43c0 | 682 | `assets/sprites/sprite-43c0-s7.bin` | `assets/sprites/sprite-43c0-s7.png` | - |
| $0000 | 687 | `assets/sprites/sprite-0000-s0.bin` | `assets/sprites/sprite-0000-s0.png` | - |
| $0000 | 687 | `assets/sprites/sprite-0000-s2.bin` | `assets/sprites/sprite-0000-s2.png` | - |
| $0000 | 687 | `assets/sprites/sprite-0000-s5.bin` | `assets/sprites/sprite-0000-s5.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $0000 | 687 | `assets/sprites/sprite-0000-s7.bin` | `assets/sprites/sprite-0000-s7.png` | - |
| $2700 | 688 | `assets/sprites/sprite-2700-s0.bin` | `assets/sprites/sprite-2700-s0.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $27c0 | 688 | `assets/sprites/sprite-27c0-s1.bin` | `assets/sprites/sprite-27c0-s1.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $2880 | 688 | `assets/sprites/sprite-2880-s2.bin` | `assets/sprites/sprite-2880-s2.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $2940 | 688 | `assets/sprites/sprite-2940-s3.bin` | `assets/sprites/sprite-2940-s3.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $2880 | 688 | `assets/sprites/sprite-2880-s4.bin` | `assets/sprites/sprite-2880-s4.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |
| $2940 | 688 | `assets/sprites/sprite-2940-s5.bin` | `assets/sprites/sprite-2940-s5.png` | sprite was displayed in multicolor mode; preview is monochrome fallback |

