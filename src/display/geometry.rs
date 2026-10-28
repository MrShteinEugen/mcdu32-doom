// SPDX-License-Identifier: GPL-3.0-only
//! Display geometries confirmed by coordinate probes on the Captain V1.06.
use anyhow::{Result, ensure};
use clap::ValueEnum;
use serde::Serialize;

pub const LCD_WIDTH: usize = 584;
pub const LCD_HEIGHT: usize = 440;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    #[default]
    Full,
    Dense,
    Standard,
}

impl Profile {
    pub fn geometry(self) -> Geometry {
        let (columns, rows, width, height) = match self {
            Self::Full => (53, 31, 11, 14),
            Self::Dense => (48, 28, 11, 14),
            Self::Standard => (24, 14, 23, 29),
        };
        Geometry::new(columns, rows, width, height).expect("valid built-in display profile")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Geometry {
    pub columns: u16,
    pub rows: u16,
    pub glyph_width: u8,
    pub glyph_height: u8,
    pub left: u16,
    pub top: u16,
}

impl Geometry {
    /// Virtual geometry only: arbitrary grids are not proof of device support.
    pub fn simulated(columns: usize, rows: usize) -> Result<Self> {
        ensure!(
            (24..=96).contains(&columns) && (4..=64).contains(&rows),
            "virtual grid must be 24..96 columns and 4..64 rows"
        );
        ensure!(
            rows > crate::display::layout::panel_rows(columns),
            "virtual grid needs a gameplay row above the status bar"
        );
        let (width, height) = match (columns, rows) {
            (53, 31) | (48, 28) => (11, 14),
            (24, 14) => (23, 29),
            _ => (LCD_WIDTH / columns, LCD_HEIGHT / rows),
        };
        Ok(Self {
            columns: columns as u16,
            rows: rows as u16,
            glyph_width: width as u8,
            glyph_height: height as u8,
            left: 0,
            top: 0,
        })
    }
    pub fn new(columns: u16, rows: u16, width: u8, height: u8) -> Result<Self> {
        ensure!(
            (16..=96).contains(&columns) && (4..=64).contains(&rows),
            "grid outside diagnostic limits"
        );
        ensure!(
            (5..=23).contains(&width) && (7..=29).contains(&height),
            "invalid glyph dimensions"
        );
        let pixel_w = u32::from(columns) * u32::from(width);
        let pixel_h = u32::from(rows) * u32::from(height);
        ensure!(
            pixel_w <= LCD_WIDTH as u32 && pixel_h <= LCD_HEIGHT as u32,
            "grid does not fit the driver's 584x440 display model"
        );
        Ok(Self {
            columns,
            rows,
            glyph_width: width,
            glyph_height: height,
            left: 36 + ((LCD_WIDTH as u32 - pixel_w) / 2) as u16,
            top: 20 + ((LCD_HEIGHT as u32 - pixel_h) / 2) as u16,
        })
    }

    pub fn cells(self) -> usize {
        self.columns as usize * self.rows as usize
    }
    pub fn width(self) -> usize {
        self.columns as usize * self.glyph_width as usize
    }
    pub fn height(self) -> usize {
        self.rows as usize * self.glyph_height as usize
    }
    pub fn compact(self) -> bool {
        self.glyph_width != 23 || self.glyph_height != 29
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simulation_validates_dimensions_before_division_and_matches_known_rasters() {
        for (columns, rows) in [(0, 31), (53, 0), (53, 4), (97, 31)] {
            assert!(Geometry::simulated(columns, rows).is_err());
        }
        for profile in [Profile::Full, Profile::Dense, Profile::Standard] {
            let hardware = profile.geometry();
            let simulation =
                Geometry::simulated(usize::from(hardware.columns), usize::from(hardware.rows))
                    .expect("simulation");
            assert_eq!(simulation.width(), hardware.width());
            assert_eq!(simulation.height(), hardware.height());
            assert_eq!(simulation.cells(), hardware.cells());
        }
    }
}
