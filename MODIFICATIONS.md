# Third-party modification record

Notice recorded **2026-10-20** for the current source snapshot. The changes below
were present by this date. This is the date of this consolidated notice, not an
invented date for each historical edit. There is no local Git history to establish
earlier per-edit dates. These files are modified versions, not upstream originals.
This record accompanies distribution under GPLv3 section 5(a).

## Vendored DOOM engine

Baseline: DoomGeneric `e257a09527062bfb986b6e5e0780839b6cbd5b71`.
The [per-file record](docs/engine-provenance.json) records the exact bytes and
whether they match that baseline. Underlying copyright/grants are retained.

| File | Current downstream change |
| --- | --- |
| `vendor/doomgeneric/d_main.c` | Host shutdown callback, per-frame text reset and full gameplay viewport. |
| `vendor/doomgeneric/hu_lib.c` | Export engine HUD/message text to the host. |
| `vendor/doomgeneric/i_system.c` | Route fatal errors and shutdown to the C-only host boundary. |
| `vendor/doomgeneric/m_menu.c` | Export menu text for the MCDU host. |
| `vendor/doomgeneric/r_defs.h` | Sprite ownership metadata for host rendering. |
| `vendor/doomgeneric/r_main.c` | Frame semantic-mask lifecycle. |
| `vendor/doomgeneric/r_things.c` | Export clipped visible sprite ownership/enemy semantics. |
| `vendor/doomgeneric/mcdu_text.h` | New host ABI include, GPL-3.0-only; not an upstream file. |

The host bridge/build configuration is supplied in `src/doom/native/doom_bridge.c`,
`src/doom/native/doom_bridge.h` and `build.rs`. These are downstream host code. The selected
engine build uses a 320 × 200 framebuffer without sound, networking or DEHACKED.
Engine rules are otherwise retained. Do not infer any new upstream permission
from this modification record.

## Aircraft pages and fonts

Baseline: FlyByWire aircraft `76db0aef132fd9698fd4013e2d7239c517dd284e`.
The four SFD/TTF files are unmodified. `assets/aircraft-font/bitmap.json` is a
derived 23 × 29 monochrome raster with generated square placeholders; generator:
`scripts/build-aircraft-font.py`. `src/mcdu/display.rs` creates bold D/O/M glyph
variants at runtime. `src/mcdu/layout.rs` adapts labels, colors and placement
to a standalone Rust state machine and adds the red DOOM entry. No simulator
flight-management logic or navigation database is included.

Preserve [font attribution and upstream license](assets/aircraft-font/NOTICE.md),
sources, generator and [asset hashes](docs/asset-provenance.json).

## Protocol map and references

The BSD-3-Clause captured map in `assets/font-map.json` is preserved as JSON.
The host patches glyph payloads and researched dimension/position/brightness
fields at runtime; it does not distribute modified firmware. The GPLv3 WINCTRL
X-Plane reference informed serialization, descriptor-address interpolation,
LCD clear and lamp commands. Its source is not linked into this program.
The serializer is supplied under GPL-3.0-only; research attribution is preserved
in [THIRD_PARTY.md](THIRD_PARTY.md).

## Images and data

No IWAD is included. Freedoom framebuffer captures, glyph/contrast/ASCII
transformations, comparison layouts and aircraft captures are identified
in [image notices](docs/images/LICENSE.md) and [generation metadata](docs/images/gallery.json).
Underlying Freedoom artwork is not relabeled as the host author's exclusive work.

## Future changes

Migration update **2026-10-26**: host Rust modules and the C bridge were moved
into domain directories. ABI data and rendering were separated, shared glyph
sampling and capture helpers were extracted, and emulator validation and PNG
finalization were strengthened. The explicit C source list preserves the MVP
build inputs. Vendored engine files retain the exact bytes recorded in
`docs/engine-provenance.json`; this migration does not add further engine changes.

Migration update **2026-10-30**: MCDU/DOOM service orchestration and navigation
were moved into `src/service/`. Pause handling is shared, input ownership is
released on shutdown and startup errors, and closing the command channel stops
the service. Blocks and its long-slash launch gesture are excluded. These are
host-only changes; vendored engine and original font files remain unchanged.

Keep this record with the changed files, update its notice date and descriptions
when those modifications change, and regenerate the relevant provenance records.
Do not edit upstream license texts to make a grant appear broader.
