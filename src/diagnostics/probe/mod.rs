// SPDX-License-Identifier: GPL-3.0-only
//! Static display probes. Offline generation never opens HID.
pub use crate::device::protocol::{Layout, layout_packets};
pub use crate::display::Geometry as Grid;
use crate::{
    device::{
        font::{self, Packing},
        protocol, transport,
    },
    output::capture::save_png,
};
mod patterns;
use anyhow::{Result, ensure};
use clap::{Args, ValueEnum};
use patterns::{diagnostic_atlas, expected, fill, table};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Copy, Debug, Default, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GridOrder {
    Before,
    #[default]
    After,
    Both,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pattern {
    #[default]
    Coordinates,
    Solid,
    Checkerboard,
}

#[derive(Args, Debug)]
pub struct Options {
    #[arg(long, default_value_t = 48, value_parser=clap::value_parser!(u16).range(16..=96))]
    pub columns: u16,
    #[arg(long, default_value_t = 28, value_parser=clap::value_parser!(u16).range(4..=64))]
    pub rows: u16,
    #[arg(long, default_value_t = 11, value_parser=clap::value_parser!(u8).range(5..=23))]
    pub glyph_width: u8,
    #[arg(long, default_value_t = 14, value_parser=clap::value_parser!(u8).range(7..=29))]
    pub glyph_height: u8,
    #[arg(long, value_enum, default_value = "tight")]
    pub packing: Packing,
    #[arg(long, value_enum, default_value = "coordinates")]
    pub pattern: Pattern,
    /// Foreground palette index for solid/checkerboard patterns (2 = white).
    #[arg(long, default_value_t = 2, value_parser=clap::value_parser!(u8).range(1..=10))]
    pub color: u8,
    #[arg(long, value_enum, default_value = "captured")]
    pub layout: Layout,
    #[arg(long, value_enum, default_value = "after")]
    pub grid_order: GridOrder,
    /// Use the original captured font transaction; requires 23x29 glyphs.
    #[arg(long)]
    pub baseline: bool,
    /// Test only the grid, keeping a previously loaded diagnostic font.
    #[arg(long, conflicts_with = "baseline")]
    pub skip_font: bool,
    /// Send once to the physical panel. Otherwise only export files.
    #[arg(long)]
    pub mcdu: bool,
    #[arg(long, requires = "mcdu")]
    pub device_path: Option<String>,
    #[arg(long, default_value_t = 10000, value_parser=clap::value_parser!(u64).range(500..=20000))]
    pub packet_us: u64,
    #[arg(long, default_value = "output/grid-probe")]
    pub out: PathBuf,
}

pub fn run(options: Options) -> Result<()> {
    let grid = Grid::new(
        options.columns,
        options.rows,
        options.glyph_width,
        options.glyph_height,
    )?;
    ensure!(
        !options.baseline || (grid.glyph_width == 23 && grid.glyph_height == 29),
        "--baseline requires --glyph-width 23 --glyph-height 29"
    );
    ensure!(
        !options.skip_font || options.pattern == Pattern::Coordinates,
        "solid/checkerboard must upload their full-cell glyph; omit --skip-font"
    );
    let atlas = diagnostic_atlas();
    let (screen, text) = match options.pattern {
        Pattern::Coordinates => table(&grid),
        pattern => fill(&grid, pattern, options.color),
    };
    let mut font = if options.skip_font {
        font::Upload {
            packets: Vec::new(),
            banks: Vec::new(),
        }
    } else if options.baseline {
        let mut packets = protocol::font_packets(&atlas, 200)?;
        protocol::preserve_grid(&mut packets);
        font::Upload {
            packets,
            banks: Vec::new(),
        }
    } else {
        font::rebuild(
            &atlas,
            grid.glyph_width as usize,
            grid.glyph_height as usize,
            options.packing,
            200,
        )?
    };
    let layout = layout_packets(&atlas, grid, options.layout)?;
    let display = protocol::encode_cells(&atlas, &screen)?;
    let mut reports = Vec::new();
    if matches!(options.grid_order, GridOrder::Before | GridOrder::Both) {
        reports.extend(layout.clone());
    }
    reports.append(&mut font.packets);
    if matches!(options.grid_order, GridOrder::After | GridOrder::Both) {
        reports.extend(layout);
    }
    reports.extend(display.clone());
    std::fs::create_dir_all(&options.out)?;
    // Persist the exact replayable reports, not just a reconstructed screen.
    std::fs::write(
        options.out.join("reports.json"),
        serde_json::to_vec(&reports)?,
    )?;
    let binary: Vec<_> = reports
        .iter()
        .flat_map(|p| {
            let mut padded = [0; 64];
            padded[..p.len()].copy_from_slice(p);
            padded
        })
        .collect();
    std::fs::write(options.out.join("reports.bin"), binary)?;
    std::fs::write(options.out.join("table.txt"), &text)?;
    let pixels = expected(&atlas, &screen, grid);
    save_png(
        &options.out.join("expected.png"),
        &pixels,
        grid.columns as usize * grid.glyph_width as usize,
        grid.rows as usize * grid.glyph_height as usize,
    )?;
    let mut manifest = serde_json::json!({
        "grid": grid, "packing": options.packing, "layout": options.layout,
        "pattern": options.pattern, "color": options.color,
        "grid_order": options.grid_order, "baseline": options.baseline, "skip_font": options.skip_font, "banks": font.banks,
        "reports": reports.len(), "screen_reports": display.len(), "packet_us": options.packet_us,
        "last_cell": {"row": grid.rows - 1, "column": grid.columns - 1},
        "hardware_requested": options.mcdu, "usb_written": false, "usb_transfer_complete": false,
        "hardware_visual_confirmed": false,
        "address_method": "piecewise interpolation of captured WRITE descriptor addresses; unverified on compact fonts",
        "note": "PNG is the requested output; USB success does not confirm firmware acceptance"
    });
    let manifest_path = options.out.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    println!(
        "Probe: {}x{} cells, {}x{} glyphs; {} reports. Files: {}",
        grid.columns,
        grid.rows,
        grid.glyph_width,
        grid.glyph_height,
        reports.len(),
        options.out.display()
    );
    if options.mcdu {
        match transport::write_once(
            options.device_path.as_deref(),
            &reports,
            Duration::from_micros(options.packet_us),
        ) {
            Ok(transmission) => {
                manifest["usb_written"] = true.into();
                manifest["usb_transfer_complete"] = true.into();
                manifest["transmission"] = serde_json::to_value(transmission)?;
                println!(
                    "USB transfer complete. Inspect the {:?} pattern through row {} and column {}; visual acceptance is still unknown.",
                    options.pattern,
                    grid.rows - 1,
                    grid.columns - 1
                );
            }
            Err(error) => {
                manifest["usb_written"] = serde_json::Value::Null;
                manifest["error"] = format!("{error:#}").into();
                std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
                return Err(error);
            }
        }
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    } else {
        println!("Offline export; no HID device opened.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labelled_last_cell_and_grid_geometry_survive_packet_boundaries() {
        let grid = Grid::new(48, 28, 11, 14).expect("grid");
        let atlas = diagnostic_atlas();
        let (screen, text) = table(&grid);
        assert_eq!(screen.len(), 1344);
        assert!(
            text.lines()
                .last()
                .expect("last row")
                .contains("END R27 C47")
        );
        assert_eq!(atlas.address(screen[1343].glyph).0, '#');
        let packets = protocol::encode_cells(&atlas, &screen).expect("screen");
        assert_eq!(packets.len(), 64);
        let wire: Vec<_> = packets
            .iter()
            .flat_map(|p| p[1..].iter().copied())
            .collect();
        assert_eq!(&wire[4030..4032], &[2, b'#']); // high attribute byte + last character
        for mode in [Layout::Captured, Layout::Standalone, Layout::Begin] {
            let packets = layout_packets(&atlas, grid, mode).expect("layout");
            let wire: Vec<_> = packets
                .iter()
                .filter(|p| p[1] == 0)
                .flat_map(|p| p[4..4 + p[3] as usize].iter().copied())
                .collect();
            let start = wire
                .windows(8)
                .position(|p| p == [0x32, 0xbb, 0, 0, 0x18, 1, 0, 0])
                .expect("grid command");
            assert_eq!(&wire[start + 17..start + 25], &[64, 0, 44, 0, 28, 0, 48, 0]);
        }
    }
    #[test]
    fn oversized_geometry_is_rejected_before_hid() {
        assert!(Grid::new(48, 28, 23, 29).is_err());
        assert!(Grid::new(24, 14, 23, 29).is_ok());
    }

    #[test]
    fn solid_pattern_has_no_gaps_in_either_font_bank() {
        let grid = Grid::new(48, 28, 11, 14).expect("grid");
        let atlas = diagnostic_atlas();
        let (screen, _) = fill(&grid, Pattern::Solid, 2);
        let pixels = expected(&atlas, &screen, grid);
        assert_eq!(pixels.len(), 528 * 392);
        assert!(pixels.iter().all(|&p| p == 0xffffff));
    }
}
