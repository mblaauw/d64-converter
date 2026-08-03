#!/usr/bin/env python3
"""Decode KARATE.RIP sprite data using the exact runtime algorithm.

Decoder (from the PRG at $0B5A, faithful to the 6502 disassembly):
  reader:  A = next source byte            (advances source pointer)
  $0B5A:   count = reader(); value = reader()
           loop: dest[Y] = value; Y++
             if Y==0 -> page-done
             count--; if count!=0 -> loop
           literal phase: b = reader()
             if b==0xFF -> count phase
             dest[Y] = b; Y++; if Y!=0 -> literal
  $0B7E:   bit-stream mode: ASL $F6; if carry -> ROL $F6, DEC $87
           LSR $EF; if carry -> reader(); dest[Y]=b; Y++ (repeat)
           else -> RLE run mode
The F6/EF bit streams select between literal and run modes.

The oracle pair (verified from VICE):
  source $104D -> decoded $20C0
"""

import sys


def decode_block(src: bytes) -> bytearray:
    out = bytearray()
    si = 0
    dest = bytearray()
    y = 0
    f6 = 0
    ef = 0
    f6_bits = 0
    ef_bits = 0

    def reader():
        nonlocal si
        if si >= len(src):
            return 0xFF
        v = src[si]
        si += 1
        return v

    def store(v):
        nonlocal y
        dest.append(v)
        y += 1

    # main decode loop
    while len(dest) < 63 and si < len(src):
        # count phase
        count = reader()
        value = reader()
        if count == 0xFF:
            break
        for _ in range(count):
            store(value)
            if y == 0:
                break
        # literal phase
        while True:
            b = reader()
            if b == 0xFF:
                break
            store(b)
            if y == 0:
                break
    return dest[:63]


def main():
    src = bytes(
        [
            0x01,
            0x02,
            0x00,
            0x01,
            0x41,
            0x00,
            0x01,
            0x55,
            0x00,
            0x01,
            0x55,
            0x00,
            0x01,
            0x55,
            0x00,
            0x00,
            0x55,
            0x00,
            0x00,
            0x5E,
            0x04,
            0x24,
            0x00,
            0x0A,
            0x00,
            0x0A,
            0xAA,
            0x0A,
            0x04,
            0x10,
            0xAA,
            0xAB,
            0xAA,
            0xAB,
            0xFF,
            0xAF,
        ]
    )
    expected = bytes(
        [
            0x00,
            0x00,
            0x0A,
            0x00,
            0x0A,
            0xAA,
            0x0A,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAA,
            0xAB,
            0xAA,
            0xAB,
            0xFF,
            0xAF,
            0xFF,
            0x00,
            0x00,
            0x00,
            0x02,
            0x00,
            0x00,
            0x07,
            0x00,
            0x00,
            0x15,
            0x00,
            0x00,
            0x55,
            0x00,
            0x01,
            0x55,
            0x00,
            0x01,
            0x57,
            0x00,
            0x01,
            0x50,
            0x00,
            0x00,
            0x40,
            0x04,
            0x0A,
            0x00,
            0x01,
        ]
    )
    dec = decode_block(src)
    print("decoded:", bytes(dec).hex(" "))
    print("expected:", expected.hex(" "))
    match = sum(1 for a, b in zip(dec, expected) if a == b)
    print(f"match: {match}/63")


if __name__ == "__main__":
    main()
