# Embedded Executed Code

Coverage-seeded linear disassembly of executed code.

- Instructions: 127

```
$08c5: 85 15     STA $15
$08c7: c9 02     CMP #$02
$08c9: b0 03     BCS $08ce
$08cb: a9 00     LDA #$00
$08cd: 60        RTS 
$08ce: a9 00     LDA #$00
$08d0: 85 16     STA $16
$08d2: a5 15     LDA $15
$08d4: 4a        LSR A
$08d5: 38        SEC 
$08d6: 26 16     ROL $16
$08d8: c9 00     CMP #$00
$08da: d0 f8     BNE $08d4
$08dc: 20 aa 08  JSR $08aa
$08df: 25 16     AND $16
$08e1: c5 15     CMP $15
$08e3: b0 f7     BCS $08dc
$08e5: 60        RTS 
$08e6: c0 00     CPY #$00
$08e8: f0 43     BEQ $092d
$08ea: 84 18     STY $18
$08ec: 48        PHA 
$08ed: a9 00     LDA #$00
$0aba: d0 06     BNE $0ac2
$0abc: a5 56     LDA $56
$0ac2: a9 01     LDA #$01
$0ac4: 60        RTS 
$0ac5: a5 56     LDA $56
$4438: a5 16     LDA $16
$443a: dd 67 93  CMP $9367,X
$443d: 90 0b     BCC $444a
$443f: e8        INX 
$4440: e0 04     CPX #$04
$4442: b0 1d     BCS $4461
$4444: 98        TYA 
$4445: 49 01     EOR #$01
$4447: a8        TAY 
$4448: 10 ee     BPL $4438
$444a: a6 15     LDX $15
$444c: c0 00     CPY #$00
$444e: f0 08     BEQ $4458
$4450: ad 1b d0  LDA $d01b
$4453: 1d ef 59  ORA $59ef,X
$4456: d0 06     BNE $445e
$4458: ad 1b d0  LDA $d01b
$445b: 3d f7 59  AND $59f7,X
$445e: 8d 1b d0  STA $d01b
$4461: e6 15     INC $15
$4463: a6 15     LDX $15
$4465: e0 08     CPX #$08
$4467: 90 c5     BCC $442e
$4469: 60        RTS 
$446a: ad 5c 93  LDA $935c
$446d: 29 0f     AND #$0f
$446f: c9 08     CMP #$08
$4471: b0 06     BCS $4479
$4473: 0a        ASL A
$4474: 18        CLC 
$4475: 69 10     ADC #$10
$4477: d0 03     BNE $447c
$445e: 8d 1b d0  STA $d01b
$4461: e6 15     INC $15
$4463: a6 15     LDX $15
$4465: e0 08     CPX #$08
$4467: 90 c5     BCC $442e
$4469: 60        RTS 
$446a: ad 5c 93  LDA $935c
$446d: 29 0f     AND #$0f
$446f: c9 08     CMP #$08
$4471: b0 06     BCS $4479
$4473: 0a        ASL A
$4474: 18        CLC 
$7693: db a9 04  DCP $04a9,Y
$7696: 85 17     STA $17
$7698: a9 0a     LDA #$0a
$769a: 20 c5 08  JSR $08c5
$769d: 85 15     STA $15
$769f: a4 17     LDY $17
$76a1: b9 1f 93  LDA $931f,Y
$76a4: 0a        ASL A
$76a5: 85 16     STA $16
$76a7: 0a        ASL A
$76a8: 0a        ASL A
$76a9: 65 16     ADC $16
$76ab: 65 15     ADC $15
$76ad: aa        TAX 
$76ae: 99 25 93  STA $9325,Y
$76b1: bd 44 95  LDA $9544,X
$76b4: 99 2d 93  STA $932d,Y
$76b7: bd 76 95  LDA $9576,X
$76ba: 99 36 93  STA $9336,Y
$76bd: c6 17     DEC $17
$76bf: 10 d7     BPL $7698
$76c1: a9 02     LDA #$02
$76c3: 85 17     STA $17
$76c5: a9 32     LDA #$32
$76c7: 20 c5 08  JSR $08c5
$76ca: a0 04     LDY #$04
$76cc: d9 25 93  CMP $9325,Y
$76cf: f0 f4     BEQ $76c5
$76d1: 88        DEY 
$76d2: 10 f8     BPL $76cc
$76d4: aa        TAX 
$76d5: a4 17     LDY $17
$76d7: 99 2a 93  STA $932a,Y
$76da: bd 44 95  LDA $9544,X
$76dd: 99 32 93  STA $9332,Y
$76e0: bd 76 95  LDA $9576,X
$76e3: 99 3b 93  STA $933b,Y
$76e6: c6 17     DEC $17
$76e8: 10 db     BPL $76c5
$76ea: a0 04     LDY #$04
$76ec: b9 1f 93  LDA $931f,Y
$76ef: aa        TAX 
$76f0: 98        TYA 
$76f1: 9d 3f 93  STA $933f,X
$76f4: a9 0c     LDA #$0c
$76f6: 99 45 93  STA $9345,Y
$76f9: 88        DEY 
$76fa: 10 f0     BPL $76ec
$76fc: ad 32 93  LDA $9332
$76ff: 85 86     STA $86
$7701: ad 3b 93  LDA $933b
$7704: 85 87     STA $87
$7706: ad 2a 93  LDA $932a
$7709: a0 00     LDY #$00
$770b: a2 0a     LDX #$0a
```
