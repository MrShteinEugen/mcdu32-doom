// SPDX-License-Identifier: GPL-3.0-only
//! Separate 24 x 14 text font. Bank 5 is large; bank 6 is small.
use crate::display::{
    Cell, Geometry, Profile,
    atlas::{Atlas, CHARSET, Glyph},
};

pub const COLUMNS: usize = 24;
pub const ROWS: usize = 14;
pub fn geometry() -> Geometry {
    Profile::Standard.geometry()
}

pub fn atlas() -> Atlas {
    let bitmaps: Vec<Vec<u8>> =
        serde_json::from_str(include_str!("../../assets/aircraft-font/bitmap.json"))
            .expect("checked-in aircraft font");
    assert_eq!(bitmaps.len(), 220);
    let mut atlas = Atlas::universal();
    atlas.scene.clear();
    atlas.glyphs = bitmaps
        .into_iter()
        .map(|bytes| Glyph {
            bitmap: bytes.try_into().expect("23x29 glyph"),
        })
        .collect();
    // Three otherwise unused lowercase slots hold bold capitals for the game
    // entry. Keep all normal aircraft characters and the small bank intact.
    for ch in ['D', 'O', 'M'] {
        let source = atlas.glyphs[glyph(ch, false) as usize].clone();
        atlas.glyphs[emphasis_glyph(ch) as usize] =
            Glyph::from_fn(|x, y| source.pixel(x, y) || (x > 0 && source.pixel(x - 1, y)));
    }
    atlas
}

pub fn emphasis_glyph(ch: char) -> u8 {
    assert!(matches!(ch, 'D' | 'O' | 'M'));
    glyph(ch.to_ascii_lowercase(), false)
}

pub fn glyph(ch: char, small: bool) -> u8 {
    let ch = if ch == '\u{a0}' { ' ' } else { ch };
    CHARSET
        .chars()
        .position(|c| c == ch)
        .unwrap_or(b'?' as usize - 32) as u8
        + if small { 110 } else { 0 }
}

pub struct Text {
    pub cells: Vec<Cell>,
}
impl Default for Text {
    fn default() -> Self {
        Self {
            cells: vec![
                Cell {
                    glyph: glyph(' ', false),
                    color: 2
                };
                COLUMNS * ROWS
            ],
        }
    }
}
impl Text {
    pub fn doom_entry(&mut self, row: usize) {
        self.put(row, 0, "<DOOM", 6, false);
        for (i, ch) in "DOOM".chars().enumerate() {
            self.cells[row * COLUMNS + i + 1].glyph = emphasis_glyph(ch);
        }
    }
    pub fn put(&mut self, row: usize, col: usize, text: &str, color: u8, small: bool) {
        if text.is_empty() {
            return;
        }
        assert!(row < ROWS && col < COLUMNS);
        for (i, ch) in text.chars().take(COLUMNS - col).enumerate() {
            self.cells[row * COLUMNS + col + i] = Cell {
                glyph: glyph(ch, small),
                color,
            };
        }
    }
    pub fn right(&mut self, row: usize, text: &str, color: u8, small: bool) {
        self.put(
            row,
            COLUMNS.saturating_sub(text.chars().count()),
            text,
            color,
            small,
        );
    }
    pub fn title(&mut self, text: &str) {
        self.put(
            0,
            COLUMNS.saturating_sub(text.chars().count()) / 2,
            text,
            2,
            false,
        );
    }
    pub fn field(&mut self, index: usize, right: bool, label: &str, value: &str, color: u8) {
        assert!(index < 6);
        let row = 2 + index * 2;
        if right {
            self.right(row - 1, label, 2, true);
            self.right(row, value, color, false);
        } else {
            self.put(row - 1, 0, label, 2, true);
            self.put(row, 0, value, color, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aircraft_fonts_preserve_both_banks_except_reserved_bold_entry_slots() {
        let original: Vec<Vec<u8>> =
            serde_json::from_str(include_str!("../../assets/aircraft-font/bitmap.json"))
                .expect("bitmap fixture");
        let atlas = atlas();
        assert_eq!(atlas.glyphs.len(), 220);
        for (index, bytes) in original.iter().enumerate() {
            if ['D', 'O', 'M']
                .iter()
                .any(|&ch| index == usize::from(emphasis_glyph(ch)))
            {
                continue;
            }
            assert_eq!(
                atlas.glyphs[index].bitmap.as_slice(),
                bytes,
                "glyph {index}"
            );
        }
        for ch in ['D', 'O', 'M'] {
            let normal = &atlas.glyphs[usize::from(glyph(ch, false))];
            let bold = &atlas.glyphs[usize::from(emphasis_glyph(ch))];
            assert!(
                bold.bitmap.iter().map(|b| b.count_ones()).sum::<u32>()
                    > normal.bitmap.iter().map(|b| b.count_ones()).sum::<u32>()
            );
        }
    }
}
