// SPDX-License-Identifier: GPL-3.0-only
//! Color ASCII using the shared font and the panel's existing foreground palette.
use crate::display::{
    Cell, PALETTE,
    atlas::{Atlas, text_glyph},
    color::rgb,
    hud::Hud,
    layout::scene_rows,
};

use crate::doom::engine::{DOOM_W, Frame};

pub const RAMP: &str = " .:-=+*#%@";

pub fn glyph(character: char) -> u8 {
    // Bank 1 preserves the text/HUD slots in bank 0 and uses single-byte ASCII.
    110 + (character as u8 - b' ')
}

/// Reserve ten existing slots, keeping both graphical modes and ASCII in one upload.
pub fn install(atlas: &mut Atlas) {
    let patterns = [
        [0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 4, 4],
        [0, 4, 4, 0, 4, 4, 0],
        [0, 0, 0, 31, 0, 0, 0],
        [0, 0, 31, 0, 31, 0, 0],
        [0, 4, 4, 31, 4, 4, 0],
        [0, 21, 14, 31, 14, 21, 0],
        [10, 10, 31, 10, 31, 10, 10],
        [25, 25, 2, 4, 8, 19, 19],
        [14, 17, 23, 21, 23, 16, 14],
    ];
    for (ch, pattern) in RAMP.chars().zip(patterns) {
        let id = glyph(ch);
        atlas.scene.retain(|&candidate| candidate != id);
        atlas.glyphs[usize::from(id)] = text_glyph(pattern);
    }
}

pub struct AsciiRenderer {
    ramp: Vec<(u8, f32)>,
    cols: usize,
    rows: usize,
    hud: Hud,
    colors: Vec<(u8, [f32; 3])>,
}
impl AsciiRenderer {
    pub fn new(atlas: &Atlas, cols: usize, rows: usize, width: usize, height: usize) -> Self {
        let mut ramp: Vec<_> = RAMP
            .chars()
            .map(|ch| {
                let id = glyph(ch);
                let density = atlas.glyphs[usize::from(id)]
                    .resized_features(width, height)
                    .iter()
                    .sum::<f32>()
                    / 64.0;
                (id, density)
            })
            .collect();
        ramp.sort_by(|a, b| a.1.total_cmp(&b.1));
        Self {
            ramp,
            cols,
            rows,
            hud: Hud::default(),
            // Match hue independently of glyph density. Dark factory gray/olive
            // entries would dim sparse characters; use the seven bright colors.
            colors: (1..=7)
                .map(|index| {
                    let rgb = rgb(PALETTE[index]);
                    let length = rgb.iter().map(|c| c * c).sum::<f32>().sqrt();
                    (index as u8, rgb.map(|c| c / length))
                })
                .collect(),
        }
    }

    fn foreground(&self, rgb: [u32; 3]) -> u8 {
        let max = *rgb.iter().max().expect("RGB");
        let min = *rgb.iter().min().expect("RGB");
        if max == 0 || (max - min) as f32 <= max as f32 * 0.12 {
            return 2;
        }
        let mut best = (2, f32::NEG_INFINITY);
        for &(color, direction) in &self.colors {
            let similarity = rgb[0] as f32 * direction[0]
                + rgb[1] as f32 * direction[1]
                + rgb[2] as f32 * direction[2];
            if similarity > best.1 {
                best = (color, similarity);
            }
        }
        best.0
    }

    /// Area averaging visits source pixels once, with no universal-glyph search.
    pub fn convert(&mut self, frame: &Frame) -> Vec<Cell> {
        let active = frame.hud.active != 0;
        let scene_rows = scene_rows(self.cols, self.rows, active);
        let source_h = if active { 168 } else { 200 };
        let max_density = self.ramp.last().map_or(1.0, |item| item.1);
        let mut cells = vec![
            Cell {
                glyph: glyph(' '),
                color: 2
            };
            self.cols * self.rows
        ];
        for cy in 0..scene_rows {
            for cx in 0..self.cols {
                let (x0, x1) = (
                    cx * DOOM_W / self.cols,
                    ((cx + 1) * DOOM_W / self.cols).max(cx * DOOM_W / self.cols + 1),
                );
                let (y0, y1) = (
                    cy * source_h / scene_rows,
                    ((cy + 1) * source_h / scene_rows).max(cy * source_h / scene_rows + 1),
                );
                let mut brightness = 0.0;
                let mut enemy = 0;
                let mut rgb = [0u32; 3];
                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = y * DOOM_W + x;
                        let p = frame.pixels[i];
                        rgb[0] += (p >> 16) & 255;
                        rgb[1] += (p >> 8) & 255;
                        rgb[2] += p & 255;
                        let live = frame.semantics[i] == 1;
                        enemy += usize::from(live);
                        let luma = (((p >> 16) & 255) * 77
                            + ((p >> 8) & 255) * 150
                            + (p & 255) * 29) as f32
                            / 256.0;
                        brightness += if live {
                            (luma * 1.2 + 90.0).min(255.0)
                        } else {
                            luma
                        };
                    }
                }
                let count = (x1 - x0) * (y1 - y0);
                let target = (brightness / count as f32 / 255.0).sqrt() * max_density;
                let id = self
                    .ramp
                    .iter()
                    .min_by(|a, b| (a.1 - target).abs().total_cmp(&(b.1 - target).abs()))
                    .map_or(glyph(' '), |v| v.0);
                cells[cy * self.cols + cx] = Cell {
                    glyph: id,
                    color: match self.foreground(rgb) {
                        2 if enemy * 4 >= count => 7,
                        color => color,
                    },
                };
            }
        }
        if active {
            self.hud
                .apply_grid(frame.hud, &mut cells, self.cols, self.rows);
        }
        crate::display::text::apply(&frame.text, &mut cells, self.cols, self.rows, active);
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ascii_preserves_colored_regions_and_neutral_surfaces_without_extra_reports() {
        let mut atlas = Atlas::universal();
        install(&mut atlas);
        let mut renderer = AsciiRenderer::new(&atlas, 8, 4, 11, 14);
        let mut frame = Frame::default();
        let stripes = [
            0xff0000, 0x00ff00, 0x0000ff, 0xffff00, 0xffa500, 0xff00ff, 0x808080, 0xffffff,
        ];
        for (i, pixel) in frame.pixels.iter_mut().enumerate() {
            *pixel = stripes[(i % 320) / 40];
        }
        let cells = renderer.convert(&frame);
        for row in cells.chunks(8) {
            assert_eq!(
                row.iter().map(|c| c.color).collect::<Vec<_>>(),
                [6, 4, 3, 7, 1, 5, 2, 2]
            );
            assert!(row.iter().all(|c| c.glyph != glyph(' ')));
        }
        let reports = crate::device::protocol::encode_cells(&atlas, &cells).unwrap();
        let monochrome: Vec<_> = cells
            .iter()
            .map(|c| Cell {
                glyph: c.glyph,
                color: 2,
            })
            .collect();
        assert_eq!(
            reports.len(),
            crate::device::protocol::encode_cells(&atlas, &monochrome)
                .unwrap()
                .len()
        );
    }
    #[test]
    fn neutral_enemies_remain_visible_and_colored_sprites_keep_their_hue() {
        let mut atlas = Atlas::universal();
        install(&mut atlas);
        let mut renderer = AsciiRenderer::new(&atlas, 8, 4, 11, 14);
        let mut frame = Frame {
            pixels: vec![0x303030; 320 * 200],
            ..Frame::default()
        };
        for i in 0..frame.pixels.len() {
            if i % 320 < 80 {
                frame.semantics[i] = 1;
            }
            if i % 320 < 40 {
                frame.pixels[i] = 0x700000;
            }
        }
        let cells = renderer.convert(&frame);
        for row in cells.chunks(8) {
            assert_eq!(row[0].color, 6);
            assert_eq!(row[1].color, 7);
            assert_eq!(row[2].color, 2);
        }
    }
    #[test]
    fn ascii_slots_keep_hud_and_wire_codes_and_reduce_packet_size() {
        let mut atlas = Atlas::universal();
        let hud = atlas.glyphs[..crate::display::HUD_GLYPHS].to_vec();
        install(&mut atlas);
        assert_eq!(hud, atlas.glyphs[..crate::display::HUD_GLYPHS]);
        for ch in RAMP.chars() {
            assert_eq!(atlas.address(glyph(ch)), (ch, true));
            assert!(!atlas.scene.contains(&glyph(ch)));
        }
        let mut renderer = AsciiRenderer::new(&atlas, 53, 31, 11, 14);
        let dark = renderer.convert(&Frame::default());
        assert!(dark.iter().all(|c| c.glyph == glyph(' ')));
        assert_eq!(
            crate::device::protocol::encode_cells(&atlas, &dark)
                .expect("packets")
                .len(),
            79
        );
        let bright = renderer.convert(&Frame {
            pixels: vec![0xffffff; 320 * 200],
            ..Frame::default()
        });
        assert!(bright.iter().all(|c| c.glyph != glyph(' ')));
    }
}
