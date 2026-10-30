# SPDX-License-Identifier: GPL-3.0-only
"""Rebuild the checked-in monochrome banks from the licensed FBW font sources.

Requires Pillow. Runtime Rust code does not depend on Python or system fonts.
"""
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
CHARS = ' !"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~°☐←↑→↓Δ⬡◀▶█▲▼■□'
glyphs = []
for name in ('HoneywellMCDU', 'HoneywellMCDUSmall'):
    font = ImageFont.truetype(str(ROOT / 'assets/aircraft-font' / (name + '.ttf')), 29)
    for ch in CHARS:
        image = Image.new('L', (23, 29))
        draw = ImageDraw.Draw(image)
        if ch in '☐□■█':
            draw.rectangle((2, 5, 19, 24), outline=255, width=2, fill=255 if ch in '■█' else None)
        else:
            draw.text((1, 26), ch, font=font, fill=255, anchor='ls', stroke_width=0)
        bitmap = [0] * 87
        for y in range(29):
            for x in range(23):
                if image.getpixel((x, y)) >= 128:
                    bitmap[y * 3 + x // 8] |= 0x80 >> (x % 8)
        glyphs.append(bitmap)
(ROOT / 'assets/aircraft-font/bitmap.json').write_text(json.dumps(glyphs, separators=(',', ':')) + '\n', encoding='utf-8')
