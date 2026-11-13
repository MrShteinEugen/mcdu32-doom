# MCDU-32 display capabilities

The application renders a grid of colored characters using two downloadable
font banks. `display::Profile` defines the grid; `device::font` and
`device::protocol` encode the font and screen packets.

| Profile | Grid | Glyph | Raster |
| --- | --- | --- | --- |
| `full` | 53 × 31 | 11 × 14 | 583 × 434 |
| `dense` | 48 × 28 | 11 × 14 | 528 × 392 |
| `standard` | 24 × 14 | 23 × 29 | 552 × 406 |

All profiles fit the 584 × 440 display model. MCDU pages use a separate font
atlas and retain the 24 × 14 grid. See [game output](../docs/full-screen-doom.md).

## Font banks

The application atlas holds 220 glyphs: 110 addressable symbols in each of two
banks. Glyph indices are stored as `u8`. This is an application limit; the
firmware's memory capacity is not established by the atlas size.

`assets/font-map.json` describes the captured transaction. Banks 5 and 6 each
declare 116 records, of which the map addresses 110 symbols. An original record
uses 91 bytes: a four-byte code and 29 three-byte rows. A packed 11 × 14 record
uses 32 bytes: the code and 14 two-byte rows. The generator sets the geometry,
record size, bank size and transfer blocks together.

The renderer uses text, lines, diagonals and segment masks. The 2 × 3 segment
set contains 64 masks. All six segments in a cell share one color.

## Screen traffic

An HID report carries 63 payload bytes after its report ID. A screen cell uses
two attribute bytes plus a UTF-8 character code.

| Grid | Cells | Reports with single-byte character codes |
| --- | ---: | ---: |
| 24 × 14 | 336 | 16 |
| 48 × 28 | 1344 | 64 |
| 53 × 31 | 1643 | 79 |

Multibyte character codes increase traffic. `--packet-us` sets the packet delay;
USB operations and firmware processing add to the transfer time.

## Hardware verification

Use a coordinate grid to check the last row and column, then test solid fill
and font switching. Record the visual result for the specific panel and
firmware version. HID delivery alone cannot confirm that the firmware accepted
the layout. See the [grid probe](../docs/grid-probe.md).

The application loads fonts through HID. It does not modify firmware or expose
a direct framebuffer interface.
