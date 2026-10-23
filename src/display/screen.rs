// SPDX-License-Identifier: GPL-3.0-only
//! Screen cells, legacy font dimensions and the captured palette.
/// Captured legacy grid; runtime modes use `super::Profile` instead.
pub const COLS: usize = 24;
pub const ROWS: usize = 14;
pub const SCENE_ROWS: usize = 11;
pub const CELL_W: usize = 23;
pub const CELL_H: usize = 29;
pub const WIDTH: usize = COLS * CELL_W;
pub const HEIGHT: usize = ROWS * CELL_H;
pub const GLYPHS: usize = 220;
pub const HUD_GLYPHS: usize = 80;

/// RGB values from the captured factory palette. The first entry is black.
pub const PALETTE: [u32; 11] = [
    0x000000, 0xffa500, 0xffffff, 0x00ffff, 0x00ff00, 0xff00ff, 0xff0000, 0xffff00, 0x615c42,
    0x777777, 0x79735e,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub glyph: u8,
    pub color: u8,
}

pub type Screen = [Cell; COLS * ROWS];
