// SPDX-License-Identifier: GPL-3.0-only
//! Screen geometry, glyphs and HUD rendering.
pub mod atlas;
pub mod geometry;
pub mod hud;
mod screen;

pub use geometry::{Geometry, LCD_HEIGHT, LCD_WIDTH, Profile};
pub use screen::{
    CELL_H, CELL_W, COLS, Cell, GLYPHS, HEIGHT, HUD_GLYPHS, PALETTE, ROWS, SCENE_ROWS, Screen,
    WIDTH,
};
