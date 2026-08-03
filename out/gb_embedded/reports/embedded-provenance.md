# Execution Provenance

Per-byte provenance: executed, read, written, and write-then-execute (self-modifying) ranges.

- Executed bytes: 248
- CPU-read bytes: 28
- CPU-written bytes: 338
- Write-then-execute bytes: 2
- Executed ranges: 11
- Read ranges: 14
- Written ranges: 21
- Write-then-execute ranges: 1

| Range | Bytes | Kind |
| --- | ---: | --- |
| $0000-$0002 | 3 | executed |
| $e518-$e559 | 66 | executed |
| $e5a0-$e5b3 | 20 | executed |
| $ee8e-$ee96 | 9 | executed |
| $fd15-$fd2f | 27 | executed |
| $fda3-$fdeb | 73 | executed |
| $fdf3-$fdf8 | 6 | executed |
| $fe66-$fe6e | 9 | executed |
| $ff48-$ff57 | 16 | executed |
| $ff6e-$ff7f | 18 | executed |
| $ffff-$ffff | 1 | executed |
| $0000-$0001 | 2 | written |
| $0099-$009a | 2 | written |
| $00c3-$00c4 | 2 | written |
| $00cc-$00cd | 2 | written |
| $00cf-$00cf | 1 | written |
| $00d9-$01ff | 295 | written |
| $0286-$0286 | 1 | written |
| $0289-$0289 | 1 | written |
| $028b-$028c | 2 | written |
| $028f-$0291 | 3 | written |
| $0333-$0333 | 1 | written |
| $8ab2-$8ab2 | 1 | written |
| $d026-$d02e | 9 | written |
| $d418-$d418 | 1 | written |
| $dc00-$dc00 | 1 | written |
| $dc02-$dc05 | 4 | written |
| $dc0d-$dc0f | 3 | written |
| $dd00-$dd00 | 1 | written |
| $dd02-$dd03 | 2 | written |
| $dd0d-$dd0f | 3 | written |
| $fd4f-$fd4f | 1 | written |
| $0000-$0001 | 2 | write-then-execute |
