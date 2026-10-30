// SPDX-License-Identifier: GPL-3.0-only
use crate::display::{
    Cell,
    atlas::{BLANK, text_id},
};

pub fn home(cols: usize, rows: usize) -> Vec<Cell> {
    let mut screen = vec![
        Cell {
            glyph: BLANK,
            color: 0
        };
        cols * rows
    ];
    let title = "DOOM";
    let action = "> START DOOM";
    for (row, text, color) in [(rows / 2 - 2, title, 2), (rows / 2, action, 7)] {
        let offset = row * cols + cols.saturating_sub(text.len()) / 2;
        for (i, ch) in text.chars().take(cols).enumerate() {
            screen[offset + i] = Cell {
                glyph: text_id(ch),
                color,
            };
        }
    }
    screen
}
