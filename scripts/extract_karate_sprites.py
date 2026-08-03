#!/usr/bin/env python3
"""Extract karate sprites from Karate_Rip.d64: decode + render PNGs."""

import os
import struct
import zlib

# C64 palette (standard VICE-ish values)
PAL = {
    0: (0, 0, 0),
    1: (255, 255, 255),
    2: (136, 0, 0),
    3: (170, 255, 238),
    4: (204, 68, 204),
    5: (0, 204, 85),
    6: (0, 0, 170),
    7: (238, 238, 119),
    8: (221, 136, 85),
    9: (102, 68, 0),
    10: (255, 119, 119),
    11: (51, 51, 51),
    12: (119, 119, 119),
    13: (170, 255, 102),
    14: (0, 136, 255),
    15: (187, 187, 187),
}


def png_chunk(tag, data):
    block = tag + data
    return struct.pack(">I", len(data)) + block + struct.pack(">I", zlib.crc32(block))


def write_png(path, w, h, rgba):
    raw = b"".join(b"\x00" + bytes(rgba[y * w * 4 : (y + 1) * w * 4]) for y in range(h))
    png = b"\x89PNG\r\n\x1a\n"
    png += png_chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += png_chunk(b"IDAT", zlib.compress(raw))
    png += png_chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def sprite_to_rgba(spr, mc0, mc1, color, scale):
    w, h = 24, 21
    px = []
    for row in range(h):
        for byte in spr[row * 3 : row * 3 + 3]:
            for sh in (6, 4, 2, 0):
                v = (byte >> sh) & 3
                if v == 0:
                    px.append((0, 0, 0, 0))
                elif v == 1:
                    px.append(PAL[mc0] + (255,))
                elif v == 2:
                    px.append(PAL[mc1] + (255,))
                else:
                    px.append(PAL[color] + (255,))
    rgba = [c for p in px for c in p]
    out_w, out_h = w * scale, h * scale
    out = bytearray()
    for y in range(out_h):
        for x in range(out_w):
            i = ((y // scale) * w + (x // scale)) * 4
            out += bytes(rgba[i : i + 4])
    return out_w, out_h, bytes(out)


def render_sprite(spr, path, mc0, mc1, color, scale=10):
    w, h, rgba = sprite_to_rgba(spr, mc0, mc1, color, scale)
    write_png(path, w, h, rgba)


def main():
    out = "out/karate_rip/extracted"
    os.makedirs(out, exist_ok=True)
    ram = open("out/karate_rip2/snapshots/vice-capture.ram", "rb").read()
    prg = open("out/karate_rip/disk/files/karate._rip.prg", "rb").read()
    payload = prg[2:]

    # colors observed at capture time (run 1): mc0=10 (light red), mc1=0 (black)
    mc0 = 10
    mc1 = 0
    color = 2
    print(f"multicolor: mc0={mc0} mc1={mc1} sprite-color={color} (from capture)")

    # 1) decoded sprites from the running program (ground truth)
    decoded_slots = {
        "punch": 0x2040,
        "block": 0x20C0,
        "karateka": 0x2100,
    }
    for name, addr in decoded_slots.items():
        spr = ram[addr : addr + 63]
        path = os.path.join(out, f"decoded-{name}.png")
        render_sprite(spr, path, mc0, mc1, color)
        print(f"wrote {path}")

    # 2) raw sprites from the PRG source bank (canonical data)
    #    $100D: raw punch sprite (matches decoded $2040 minus 2 zero rows)
    raw_candidates = {
        "raw-punch": 0x100D,
        "raw-karateka": 0x108F,
    }
    for name, addr in raw_candidates.items():
        off = addr - 0x0801
        spr = payload[off : off + 63]
        if len(spr) < 63:
            continue
        path = os.path.join(out, f"{name}.bin")
        with open(path, "wb") as f:
            f.write(spr)
        print(f"wrote {path} ({len(spr)} bytes)")

    # 3) write the raw source bank as one file for completeness
    bank = payload[0x1000 - 0x0801 : 0x1466 - 0x0801]
    with open(os.path.join(out, "source-bank-1000-1466.bin"), "wb") as f:
        f.write(bank)
    print(f"wrote source-bank-1000-1466.bin ({len(bank)} bytes)")


if __name__ == "__main__":
    main()
