# Screenshot gallery

The game images use **Freedoom Phase 1 0.13.0**, rendered by the vendored engine.
They do not contain commercial DOOM game data. The comparison panels are
software captures, not physical-panel photographs or LCD-FPS measurements.

## One frame, four renderers

Spawn room: original engine frame, color glyphs, contrast glyphs, color ASCII

Enemy encounter: the same four renderers, including the source damage flash

Each sheet uses one immutable `Frame` for all four tiles. The source damage/pickup
palette effects are retained; the contrast renderer deliberately neutralizes scenery
and emphasizes visible enemies. The original 320 × 200 source is enlarged exactly
2× using nearest-neighbor sampling in the sheet, without aspect-ratio correction.
The other tiles retain their native 583 × 434 raster. Captions and padding are
presentation only. Individual PNGs contain neither the captions nor the padding.

| Scene           | Engine framebuffer | Color glyphs | Contrast glyphs | Color ASCII |
|-----------------|--------------------|--------------|-----------------|-------------|
| Spawn room      | 320 × 200          | 583 × 434    | 583 × 434       | 583 × 434   |
| Enemy encounter | 320 × 200          | 583 × 434    | 583 × 434       | 583 × 434   |

Source frame generations, fingerprints, visible-enemy pixel counts, actual packet
counts and one-off render timings are in [gallery.json](images/gallery.json).
Those single-sample timings are not a benchmark or a hardware presentation rate.

## Aircraft pages

| MCDU MENU, red DOOM entry at L5 | INIT |
|---------------------------------|------|
| MCDU MENU                       | INIT |

| TAKE OFF | PROG |
|----------|------|
| PERF     | PROG |

RADIO NAV, full raster.
Aircraft screenshots use the actual 24 × 14 font/grid and local page state.
Fonts and template references are credited in [the font notice](../assets/aircraft-font/NOTICE.md).

## Recreate the images

From the repository root on Windows:

```powershell
.\scripts\setup-freedoom.ps1
.\scripts\capture-readme.ps1
```

The script verifies the pinned Phase 1 IWAD SHA-256, runs the Rust
`readme_gallery` example and writes PNG hashes
to [image-hashes.json](images/image-hashes.json). It opens no window or USB device
and does not modify the tray service's files. The engine is live, so the exact
frame chosen and timing samples can vary between runs; comparisons within a run
always use the same source snapshot. The scripted player walks, turns and pulses
USE to open doors; no game assets or framebuffer pixels are painted in afterwards.

If no enemy is found, capture fails before replacing the published gallery;
retry it instead of presenting an empty view as an enemy example. The checked-in
gallery contains an actual encounter. Preserve [licenses and credits](images/LICENSE.md)
when reusing any screenshot.
