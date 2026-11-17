# Static grid probe

`probe` writes a packet stream, a text grid, a PNG of the expected display and
`manifest.json`. It opens the device only when hardware output is requested.

```powershell
.\probe-grid.ps1 -ProbeProfile Compact
cargo run --locked -- probe --columns 53 --rows 31 --out output/probe-full
```

| Script profile | Grid | Glyph |
| --- | --- | --- |
| Baseline | 24 × 14 | 23 × 29, original transaction |
| Compact | 24 × 14 | 11 × 14 |
| Wide | 48 × 14 | 11 × 14 |
| Tall | 24 × 28 | 11 × 14 |
| Dense, DenseTight, DenseCaptured | 48 × 28 | 11 × 14 |

`DenseTight` selects `tight` packing; `DenseCaptured` selects `captured` packing.
The script accepts layouts `captured`, `standalone` and `begin`; grid orders
`before`, `after` and `both`; and patterns `coordinates`, `solid` and
`checkerboard`. Use `-KeepFont` to retain the current font. The CLI rejects
`--skip-font` with solid or checkerboard patterns, which require a full-cell
glyph. It also rejects grids that exceed the 584 × 440 display model.

```powershell
.\probe-grid.ps1 -ProbeProfile Dense -Hardware
```

`-DevicePath` requires `-Hardware` and takes the exact path reported by `devices`.
Stop any service or other application using the panel before running a probe.

| File | Contents |
| --- | --- |
| `reports.json` | Original HID reports |
| `reports.bin` | Reports padded to 64 bytes |
| `expected.png` | Expected display image |
| `manifest.json` | Probe configuration and transfer status |

`usb_transfer_complete` confirms delivery of the reports. Inspect the last row
and column, glyph dimensions and return to normal display mode on the panel to
verify the layout. See [display capabilities](../specs/MCDU-display-capabilities.md).
