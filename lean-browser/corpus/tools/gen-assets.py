#!/usr/bin/env python3
"""Generate the corpus's tiny raster assets with the stdlib only.

- img/quadrants.png : 48x32 RGB, four flat colour quadrants (< 200 bytes)
- img/alpha.png     : 32x32 RGBA, a soft disc over transparency

JPEGs need an encoder; see gen-jpeg.mjs (Chromium via Playwright).
"""

import struct
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent / "img"


def png(path, width, height, rows, color_type):
    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(r) for r in rows)
    data = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(data)
    print(f"{path} {len(data)} bytes")


def quadrants():
    w, h = 48, 32
    cols = [(224, 80, 60), (60, 140, 220), (240, 200, 60), (70, 170, 90)]
    rows = []
    for y in range(h):
        row = []
        for x in range(w):
            c = cols[(0 if y < h // 2 else 2) + (0 if x < w // 2 else 1)]
            row += list(c)
        rows.append(row)
    png(HERE / "quadrants.png", w, h, rows, 2)


def alpha():
    w = h = 32
    rows = []
    for y in range(h):
        row = []
        for x in range(w):
            d = ((x - 15.5) ** 2 + (y - 15.5) ** 2) ** 0.5
            a = max(0, min(255, int(255 * (14 - d) / 3))) if d < 14 else 0
            row += [40, 90, 200, a]
        rows.append(row)
    png(HERE / "alpha.png", w, h, rows, 6)


if __name__ == "__main__":
    HERE.mkdir(exist_ok=True)
    quadrants()
    alpha()
