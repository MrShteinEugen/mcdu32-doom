// SPDX-License-Identifier: GPL-3.0-only
//! Arrange engine text on the glyph grid without changing the engine data.
use crate::display::{
    Cell,
    atlas::{BLANK, text_id},
    layout::scene_rows,
};
use crate::doom::text::TextFrame;

pub fn apply(text: &TextFrame, screen: &mut [Cell], cols: usize, rows: usize, gameplay: bool) {
    let blank = Cell {
        glyph: BLANK,
        color: 0,
    };
    let scene_rows = scene_rows(cols, rows, gameplay);
    if text.menu != 0 {
        screen.fill(blank);
    }
    for line in text.lines() {
        let value = line.value();
        let characters: Vec<_> = value.chars().collect();
        let area_rows = if text.menu != 0 { rows } else { scene_rows };
        let source_height = if gameplay && text.menu == 0 { 168 } else { 200 };
        let row = (line.y.max(0) as usize * area_rows / source_height).min(area_rows - 1);
        let col = if line.x < 0 {
            cols.saturating_sub(characters.len()) / 2
        } else {
            (line.x as usize * cols / 320).min(cols - 1)
        };
        if text.menu == 0 {
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
        apply(&text, &mut cells, 53, 31, true);
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
        apply(&text, &mut cells, 53, 31, true);
        assert_eq!(cells[0].glyph, text_id('N'));
        assert!(cells[53 * 26..].iter().all(|c| c.glyph == 200));
        text.count = 999;
        assert_eq!(text.lines().len(), 32);
    }
}
