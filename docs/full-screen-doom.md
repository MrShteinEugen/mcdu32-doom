# Full-screen game output

The `full` profile uses 53 × 31 cells and 11 × 14 glyphs, for a 583 × 434 raster.
`dense` uses 48 × 28 cells (528 × 392); `standard` uses 24 × 14 cells and 23 × 29
glyphs (552 × 406). All profiles share the glyph-bank and screen encoders.

```powershell
cargo run --locked -- play --display full --headless --seconds 3 --capture output/full
cargo run --locked -- play --display full --mcdu
```

The first command captures the output without USB. The second sends it to the
panel. `--packet-us` sets the packet delay; `--fps` sets the frame preparation
rate. The LCD refresh rate must be measured on the device.

Use the [grid probe](grid-probe.md) to check firmware and layout compatibility.
See [display capabilities](../specs/MCDU-display-capabilities.md) for the font
format and packet counts.
