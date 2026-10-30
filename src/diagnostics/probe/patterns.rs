// SPDX-License-Identifier: GPL-3.0-only
//! Test font and grid patterns used by the static probe.
use super::{Grid, Pattern};
use crate::display::{
    Cell,
    atlas::{Atlas, CHARSET, Glyph},
};
pub(super) fn diagnostic_atlas() -> Atlas {
    let mut atlas = Atlas::universal();
    for (index, ch) in CHARSET.chars().enumerate() {
        let pattern: [u8; 7] = match ch {
            '0' => [14, 17, 19, 21, 25, 17, 14],
            '1' => [4, 12, 4, 4, 4, 4, 14],
            '2' => [14, 17, 1, 2, 4, 8, 31],
            '3' => [30, 1, 1, 14, 1, 1, 30],
            '4' => [2, 6, 10, 18, 31, 2, 2],
            '5' => [31, 16, 16, 30, 1, 1, 30],
            '6' => [14, 16, 16, 30, 17, 17, 14],
            '7' => [31, 1, 2, 4, 8, 8, 8],
            '8' => [14, 17, 17, 14, 17, 17, 14],
            '9' => [14, 17, 17, 15, 1, 1, 14],
            'R' => [30, 17, 17, 30, 20, 18, 17],
            'C' => [14, 17, 16, 16, 16, 17, 14],
            'E' => [31, 16, 16, 30, 16, 16, 31],
            'N' => [17, 25, 25, 21, 19, 19, 17],
            'D' => [30, 17, 17, 17, 17, 17, 30],
            '|' => [4; 7],
            '-' => [0, 0, 0, 31, 0, 0, 0],
            '+' => [0, 4, 4, 31, 4, 4, 0],
            '#' => [10, 10, 31, 10, 31, 10, 10],
            '.' => [0, 0, 0, 0, 0, 12, 12],
            _ => [0; 7],
        };
        let glyph = Glyph::from_fn(|x, y| {
            if ch == '@' {
                return true;
            }
            if !(4..19).contains(&x) || !(4..25).contains(&y) {
                return false;
            }
            pattern[(y - 4) / 3] & (1 << (4 - (x - 4) / 3)) != 0
        });
        atlas.glyphs[index] = glyph.clone();
        atlas.glyphs[index + 110] = glyph;
    }
    atlas
}

pub(super) fn table(grid: &Grid) -> (Vec<Cell>, String) {
    let cols = grid.columns as usize;
    let rows = grid.rows as usize;
    let mut chars = vec!['.'; cols * rows];
    for col in 0..cols {
        chars[col] = char::from(b'0' + (col / 10) as u8);
        chars[cols + col] = char::from(b'0' + (col % 10) as u8);
        chars[(rows - 1) * cols + col] = '-';
    }
    for row in 2..rows - 1 {
        chars[row * cols] = '|';
        chars[(row + 1) * cols - 1] = '|';
        for col in (1..cols - 7).step_by(8) {
            for (offset, ch) in format!("R{row:02}C{col:02}").chars().enumerate() {
                chars[row * cols + col + offset] = ch;
            }
        }
    }
    for (offset, ch) in format!("END R{:02} C{:02}", rows - 1, cols - 1)
        .chars()
        .enumerate()
    {
        chars[(rows - 1) * cols + 1 + offset] = ch;
    }
    chars[(rows - 1) * cols] = '+';
    chars[rows * cols - 1] = '#';
    let text = chars
        .chunks(cols)
        .map(|r| r.iter().collect::<String>() + "\n")
        .collect();
    let screen = chars
        .iter()
        .enumerate()
        .map(|(index, &ch)| Cell {
            glyph: CHARSET
                .chars()
                .position(|c| c == ch)
                .expect("diagnostic ASCII") as u8
                + if (index / cols) % 2 == 1 { 110 } else { 0 },
            color: if index == rows * cols - 1 {
                6
            } else if index / cols == rows - 1 {
                4
            } else if index % cols == cols - 1 {
                7
            } else {
                2
            },
        })
        .collect();
    (screen, text)
}

pub(super) fn fill(grid: &Grid, pattern: Pattern, color: u8) -> (Vec<Cell>, String) {
    let cols = grid.columns as usize;
    let rows = grid.rows as usize;
    let glyph = CHARSET
        .chars()
        .position(|ch| ch == '@')
        .expect("ASCII full-cell slot") as u8;
    let screen = (0..cols * rows)
        .map(|index| Cell {
            glyph: glyph + if (index / cols) % 2 == 1 { 110 } else { 0 },
            color: if pattern == Pattern::Checkerboard && (index / cols + index % cols) % 2 == 1 {
                0
            } else {
                color
            },
        })
        .collect();
    let text = ("@".repeat(cols) + "\n").repeat(rows);
    (screen, text)
}

pub(super) fn expected(atlas: &Atlas, screen: &[Cell], grid: Grid) -> Vec<u32> {
    let mut pixels = vec![0; grid.width() * grid.height()];
    crate::display::render::rasterize_grid(
        atlas,
        screen,
        grid.columns as usize,
        grid.rows as usize,
        grid.glyph_width as usize,
        grid.glyph_height as usize,
        &mut pixels,
    );
    pixels
}
