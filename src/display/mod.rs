// SPDX-License-Identifier: GPL-3.0-only
//! Screen geometry, glyphs and HUD rendering.
pub mod ascii;
pub mod atlas;
mod color;
pub mod geometry;
pub mod hud;
pub mod layout;
pub mod render;
mod screen;
pub mod text;

pub use geometry::{Geometry, LCD_HEIGHT, LCD_WIDTH, Profile};
pub use screen::{
    CELL_H, CELL_W, COLS, Cell, GLYPHS, HEIGHT, HUD_GLYPHS, PALETTE, ROWS, SCENE_ROWS, Screen,
    WIDTH,
};
