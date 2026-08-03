# Input Events

Joystick events applied through VICE `JOYPORT_SET` during capture, scheduled by frame number (PAL, 50 frames/s). Values use VICE's active-high joystick bitmask.

- Events: 14

| frame | Port | Value | Label |
| ---: | ---: | ---: | --- |
| 0 | 1 | $00 | neutral |
| 75 | 1 | $10 | fire |
| 100 | 1 | $00 | neutral |
| 125 | 1 | $08 | right |
| 165 | 1 | $10 | fire |
| 180 | 1 | $00 | neutral |
| 200 | 1 | $04 | left |
| 240 | 1 | $01 | up |
| 265 | 1 | $02 | down |
| 290 | 1 | $00 | neutral |
| 325 | 1 | $10 | fire |
| 350 | 1 | $00 | neutral |
| 375 | 1 | $08 | right |
| 400 | 1 | $00 | neutral |
