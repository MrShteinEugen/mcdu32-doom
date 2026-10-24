// SPDX-License-Identifier: GPL-3.0-only
//! Text copied from DOOM's UI, rendered with readable font slots rather than
//! matched against scene tiles. Coordinates use DOOM's 320x200 canvas.
use crate::display::{
    Cell,
    atlas::{BLANK, text_id},
    hud::panel_rows,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextLine {
    pub x: i32,
    pub y: i32,
    pub color: i32,
    pub bytes: [u8; 128],
}
impl Default for TextLine {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            color: 2,
            bytes: [0; 128],
        }
    }
}
impl TextLine {
    pub fn value(&self) -> String {
        let end = self
            .bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.bytes.len());
        String::from_utf8_lossy(&self.bytes[..end]).into_owned()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TextFrame {
    pub count: i32,
    pub menu: i32,
    pub lines: [TextLine; 32],
}
impl TextFrame {
    pub fn lines(&self) -> &[TextLine] {
        &self.lines[..self.count.clamp(0, 32) as usize]
    }

    pub fn apply(&self, screen: &mut [Cell], cols: usize, rows: usize, gameplay: bool) {
        let blank = Cell {
            glyph: BLANK,
            color: 0,
        };
        let scene_rows = if gameplay {
            rows - panel_rows(cols)
        } else {
            rows
        };
        if self.menu != 0 {
            screen.fill(blank);
        }
        for line in self.lines() {
            let value = line.value();
            let characters: Vec<_> = value.chars().collect();
            let area_rows = if self.menu != 0 { rows } else { scene_rows };
            let source_height = if gameplay && self.menu == 0 { 168 } else { 200 };
            let row = (line.y.max(0) as usize * area_rows / source_height).min(area_rows - 1);
            let col = if line.x < 0 {
                cols.saturating_sub(characters.len()) / 2
            } else {
                (line.x as usize * cols / 320).min(cols - 1)
            };
            if self.menu == 0 {
                let end = (row + 2).min(scene_rows);
                screen[row * cols..end * cols].fill(blank);
            }
            for (index, &character) in characters.iter().enumerate() {
                let target_row = row + (col + index) / cols;
                if target_row >= area_rows {
                    break;
                }
                screen[target_row * cols + (col + index) % cols] = Cell {
                    glyph: text_id(character),
                    color: line.color.clamp(0, 10) as u8,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_and_messages_use_text_slots_with_bounds_and_uppercase() {
        let mut text = TextFrame {
            count: 1,
            menu: 1,
            ..TextFrame::default()
        };
        text.lines[0].x = -1;
        text.lines[0].y = 40;
        text.lines[0].bytes[..8].copy_from_slice(b"New game");
        let mut cells = vec![
            Cell {
                glyph: 200,
                color: 6
            };
            53 * 31
        ];
        text.apply(&mut cells, 53, 31, true);
        assert_eq!(cells[6 * 53 + 22].glyph, text_id('N'));
        assert!(
            cells
                .iter()
                .all(|c| usize::from(c.glyph) < crate::display::HUD_GLYPHS)
        );
        text.menu = 0;
        text.lines[0].x = 0;
        text.lines[0].y = 0;
        cells.fill(Cell {
            glyph: 200,
            color: 6,
        });
        text.apply(&mut cells, 53, 31, true);
        assert_eq!(cells[0].glyph, text_id('N'));
        assert!(cells[53 * 26..].iter().all(|c| c.glyph == 200));
        text.count = 999;
        assert_eq!(text.lines().len(), 32);
    }
}
