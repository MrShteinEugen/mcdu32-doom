// SPDX-License-Identifier: GPL-3.0-only
use crate::display::{CELL_H, CELL_W, GLYPHS, HUD_GLYPHS};
use crate::doom::wad::Patch;

pub const CHARSET: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~°☐←↑→↓Δ⬡◀▶█▲▼■□";
pub const BLANK: u8 = 30;
pub const SOLID: u8 = 31;
pub const SAMPLE: usize = 8;
pub const PORTRAIT: usize = 64;
const EXTRA_LETTERS: &str = "BCDEFGIJLONQSTUVXYZ";
const EXTRA_PUNCTUATION: &str = ".:!?><'()+,_\"";

/// Human-readable text uses dedicated font slots, independent of wire addresses.
pub fn text_id(character: char) -> u8 {
    let character = character.to_ascii_uppercase();
    match character {
        '0'..='9' => character as u8 - b'0',
        'H' => 18,
        'P' => 19,
        'A' => 20,
        'R' => 21,
        'M' => 22,
        'W' => 23,
        'K' => 24,
        '%' => 25,
        '/' => 26,
        '-' => 27,
        ' ' => BLANK,
        _ => EXTRA_LETTERS
            .chars()
            .position(|c| c == character)
            .map(|i| 32 + i as u8)
            .or_else(|| {
                EXTRA_PUNCTUATION
                    .chars()
                    .position(|c| c == character)
                    .map(|i| 51 + i as u8)
            })
            .unwrap_or(54), // ?
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    /// Row-major, most significant bit first, padded to three bytes per row.
    pub bitmap: [u8; CELL_H * 3],
}

/// Shared sampling coordinates for glyph matching, rasterization and font uploads.
pub(crate) fn source_pixel(x: usize, y: usize, width: usize, height: usize) -> (usize, usize) {
    assert!(x < width && y < height, "pixel outside resized glyph");
    (
        ((2 * x + 1) * CELL_W / (2 * width)).min(CELL_W - 1),
        ((2 * y + 1) * CELL_H / (2 * height)).min(CELL_H - 1),
    )
}

impl Glyph {
    pub fn from_fn(mut pixel: impl FnMut(usize, usize) -> bool) -> Self {
        let mut glyph = Self {
            bitmap: [0; CELL_H * 3],
        };
        for y in 0..CELL_H {
            for x in 0..CELL_W {
                if pixel(x, y) {
                    glyph.bitmap[y * 3 + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        glyph
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        self.bitmap[y * 3 + x / 8] & (0x80 >> (x % 8)) != 0
    }

    /// Sample the center of a target pixel using the font upload's nearest-neighbor rule.
    pub fn resized_pixel(&self, x: usize, y: usize, width: usize, height: usize) -> bool {
        let (source_x, source_y) = source_pixel(x, y, width, height);
        self.pixel(source_x, source_y)
    }

    pub fn features(&self) -> [f32; SAMPLE * SAMPLE] {
        let mut features = [0.0; SAMPLE * SAMPLE];
        for sy in 0..SAMPLE {
            for sx in 0..SAMPLE {
                let x0 = sx * CELL_W / SAMPLE;
                let x1 = (sx + 1) * CELL_W / SAMPLE;
                let y0 = sy * CELL_H / SAMPLE;
                let y1 = (sy + 1) * CELL_H / SAMPLE;
                let mut on = 0;
                for y in y0..y1 {
                    for x in x0..x1 {
                        on += usize::from(self.pixel(x, y));
                    }
                }
                features[sy * SAMPLE + sx] = on as f32 / ((x1 - x0) * (y1 - y0)) as f32;
            }
        }
        features
    }

    /// Area-average the actual compact raster, even below the feature-grid size.
    pub fn resized_features(&self, width: usize, height: usize) -> [f32; SAMPLE * SAMPLE] {
        assert!(width > 0 && height > 0);
        let mut features = [0.0; SAMPLE * SAMPLE];
        for sy in 0..SAMPLE {
            for sx in 0..SAMPLE {
                let x0 = sx as f32 * width as f32 / SAMPLE as f32;
                let x1 = (sx + 1) as f32 * width as f32 / SAMPLE as f32;
                let y0 = sy as f32 * height as f32 / SAMPLE as f32;
                let y1 = (sy + 1) as f32 * height as f32 / SAMPLE as f32;
                let mut ink = 0.0;
                for y in y0.floor() as usize..(y1.ceil() as usize).min(height) {
                    for x in x0.floor() as usize..(x1.ceil() as usize).min(width) {
                        if self.resized_pixel(x, y, width, height) {
                            ink += (((x + 1) as f32).min(x1) - x0.max(x as f32))
                                * (((y + 1) as f32).min(y1) - y0.max(y as f32));
                        }
                    }
                }
                features[sy * SAMPLE + sx] = ink / ((x1 - x0) * (y1 - y0));
            }
        }
        features
    }
}

#[derive(Clone)]
pub struct Atlas {
    pub glyphs: Vec<Glyph>,
    pub scene: Vec<u8>,
    characters: Vec<char>,
}

impl Atlas {
    /// Load WAD portraits once, keeping a generated fallback for unusual IWADs.
    pub fn from_wad(wad: &crate::doom::wad::Wad) -> Self {
        let mut atlas = Self::universal();
        match (wad.portrait("STFST00"), wad.portrait("STFOUCH0")) {
            (Ok(normal), Ok(hurt)) => atlas.set_portraits(&normal, &hurt),
            _ => eprintln!("WAD portraits unavailable; using the built-in face."),
        }
        atlas
    }
    pub fn universal() -> Self {
        let digits = [
            [14, 17, 19, 21, 25, 17, 14],
            [4, 12, 4, 4, 4, 4, 14],
            [14, 17, 1, 2, 4, 8, 31],
            [30, 1, 1, 14, 1, 1, 30],
            [2, 6, 10, 18, 31, 2, 2],
            [31, 16, 16, 30, 1, 1, 30],
            [14, 16, 16, 30, 17, 17, 14],
            [31, 1, 2, 4, 8, 8, 8],
            [14, 17, 17, 14, 17, 17, 14],
            [14, 17, 17, 15, 1, 1, 14],
        ];
        let mut glyphs: Vec<Glyph> = digits.into_iter().map(text_glyph).collect();
        for state in 0..2 {
            for quadrant in 0..4 {
                glyphs.push(Glyph::from_fn(|x, y| {
                    let xx = (x + (quadrant % 2) * CELL_W) as i32 - 23;
                    let yy = (y + (quadrant / 2) * CELL_H) as i32;
                    let outline = (xx * xx * 3 + (yy - 29) * (yy - 29)) < 1200;
                    let hair = yy < 15 && yy > 5;
                    let eyes = (20..24).contains(&yy) && (5..12).contains(&xx.abs());
                    let mouth = (40..43 + state * 3).contains(&yy) && xx.abs() < 8;
                    outline && (hair || eyes || mouth || xx.abs() > 17)
                }));
            }
        }
        // H, P, A, R, M, W, K at slots 18..24.
        for pattern in [
            [17, 17, 17, 31, 17, 17, 17],
            [30, 17, 17, 30, 16, 16, 16],
            [14, 17, 17, 31, 17, 17, 17],
            [30, 17, 17, 30, 20, 18, 17],
            [17, 27, 21, 21, 17, 17, 17],
            [17, 17, 17, 21, 21, 21, 10],
            [17, 18, 20, 24, 20, 18, 17],
            [25, 25, 2, 4, 8, 19, 19],
            [1, 2, 2, 4, 8, 8, 16],
            [0, 0, 0, 31, 0, 0, 0],
        ] {
            glyphs.push(text_glyph(pattern));
        }
        glyphs.push(Glyph::from_fn(|x, y| {
            (5..18).contains(&x)
                && (8..21).contains(&y)
                && (x == 5 || x == 17 || y == 8 || y == 20 || (x > 9 && y == 14))
        }));
        glyphs.push(Glyph::from_fn(|x, y| {
            let xx = x as i32 - 11;
            let yy = y as i32 - 12;
            (xx * xx + yy * yy < 65 && !(yy.abs() < 2 && (3..6).contains(&xx.abs())))
                || ((18..22).contains(&y) && (7..16).contains(&x) && x % 3 != 0)
        }));
        glyphs.push(Glyph::from_fn(|_, _| false));
        glyphs.push(Glyph::from_fn(|_, _| true));
        assert_eq!(glyphs.len(), 32);
        for rows in [
            [30, 17, 17, 30, 17, 17, 30],
            [14, 17, 16, 16, 16, 17, 14],
            [30, 17, 17, 17, 17, 17, 30],
            [31, 16, 16, 30, 16, 16, 31],
            [31, 16, 16, 30, 16, 16, 16],
            [14, 17, 16, 23, 17, 17, 15],
            [14, 4, 4, 4, 4, 4, 14],
            [7, 2, 2, 2, 18, 18, 12],
            [16, 16, 16, 16, 16, 16, 31],
            [14, 17, 17, 17, 17, 17, 14],
            [17, 25, 21, 19, 17, 17, 17],
            [14, 17, 17, 17, 21, 18, 13],
            [15, 16, 16, 14, 1, 1, 30],
            [31, 4, 4, 4, 4, 4, 4],
            [17, 17, 17, 17, 17, 17, 14],
            [17, 17, 17, 17, 17, 10, 4],
            [17, 17, 10, 4, 10, 17, 17],
            [17, 17, 10, 4, 4, 4, 4],
            [31, 1, 2, 4, 8, 16, 31],
            [0, 0, 0, 0, 0, 12, 12],
            [0, 12, 12, 0, 12, 12, 0],
            [4, 4, 4, 4, 4, 0, 4],
            [14, 17, 1, 2, 4, 0, 4],
            [16, 8, 4, 2, 4, 8, 16],
            [1, 2, 4, 8, 4, 2, 1],
            [4, 4, 2, 0, 0, 0, 0],
            [2, 4, 8, 8, 8, 4, 2],
            [8, 4, 2, 2, 2, 4, 8],
            [0, 4, 4, 31, 4, 4, 0],
            [0, 0, 0, 0, 4, 4, 8],
            [0, 0, 0, 0, 0, 0, 31],
            [10, 10, 5, 0, 0, 0, 0],
        ] {
            glyphs.push(text_glyph(rows));
        }
        assert_eq!(glyphs.len(), PORTRAIT);
        // Full-screen HUD portrait: 4x4 tiles. The original 2x2 states remain
        // available to the standard 24-column layout.
        for part in 0..16 {
            let glyph = Glyph::from_fn(|x, y| {
                let sx = (x + part % 4 * CELL_W) / 2;
                let sy = (y + part / 4 * CELL_H) / 2;
                glyphs[10 + sy / CELL_H * 2 + sx / CELL_W].pixel(sx % CELL_W, sy % CELL_H)
            });
            glyphs.push(glyph);
        }
        assert_eq!(glyphs.len(), HUD_GLYPHS);
        let mut push = |glyph: Glyph| {
            if glyphs.len() < GLYPHS && !glyphs.iter().any(|old| old == &glyph) {
                glyphs.push(glyph);
            }
        };
        for mask in 1..63 {
            push(Glyph::from_fn(|x, y| {
                mask & (1 << ((y * 3 / CELL_H) * 2 + x * 2 / CELL_W)) != 0
            }));
        }
        for offset in [3, 6, 9, 12, 15, 18] {
            for invert in [false, true] {
                push(Glyph::from_fn(|x, _| (x < offset) ^ invert));
                push(Glyph::from_fn(|_, y| {
                    (y < offset * CELL_H / CELL_W) ^ invert
                }));
            }
        }
        for slope in [-2.0_f32, -0.5, 0.5, 2.0] {
            for offset in [-9.0, -3.0, 3.0, 9.0] {
                for invert in [false, true] {
                    push(Glyph::from_fn(|x, y| {
                        ((y as f32 - 14.0) > slope * (x as f32 - 11.0) + offset) ^ invert
                    }));
                }
            }
        }
        for x0 in [7, 15] {
            for y0 in [7, 14, 21] {
                for direction in 0..4 {
                    push(Glyph::from_fn(|x, y| {
                        ((x < x0) ^ (direction & 1 != 0)) && ((y < y0) ^ (direction & 2 != 0))
                    }));
                }
            }
        }
        for offset in [2, 6, 10, 14, 18] {
            for thickness in [1, 3] {
                push(Glyph::from_fn(|x, _| {
                    (offset..offset + thickness).contains(&x)
                }));
                push(Glyph::from_fn(|_, y| {
                    (offset..offset + thickness).contains(&y)
                }));
            }
        }
        const BAYER: [usize; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
        for density in 1..16 {
            push(Glyph::from_fn(|x, y| BAYER[(y % 4) * 4 + x % 4] < density));
        }
        for radius in [3, 5, 7, 9, 11] {
            for stretch in [1, 2] {
                push(Glyph::from_fn(|x, y| {
                    let xx = x as i32 - 11;
                    let yy = y as i32 - 14;
                    xx * xx + yy * yy / stretch < radius * radius
                }));
            }
        }
        // Deduplication frees slots. Fill them with additional useful edge positions.
        for offset in -28..29 {
            for slope in [-1.5_f32, -1.0, -0.25, 0.25, 1.0, 1.5] {
                push(Glyph::from_fn(|x, y| {
                    y as f32 - 14.0 > slope * (x as f32 - 11.0) + offset as f32
                }));
            }
        }
        assert_eq!(
            glyphs.len(),
            GLYPHS,
            "universal dictionary must fill the agreed budget"
        );
        let mut scene = vec![BLANK, SOLID];
        scene.extend((HUD_GLYPHS..GLYPHS).map(|id| id as u8));
        Self {
            glyphs,
            scene,
            characters: CHARSET.chars().collect(),
        }
    }

    /// Replace the eight portrait tiles once at startup with the selected WAD's face.
    pub fn set_portraits(&mut self, normal: &Patch, hurt: &Patch) {
        for part in 0..16 {
            self.glyphs[PORTRAIT + part] = Glyph::from_fn(|x, y| {
                let px = (x + part % 4 * CELL_W) * normal.width / (4 * CELL_W);
                let py = (y + part / 4 * CELL_H) * normal.height / (4 * CELL_H);
                normal.luma[py * normal.width + px] >= 96
            });
        }
        for (state, patch) in [normal, hurt].iter().enumerate() {
            for quadrant in 0..4 {
                self.glyphs[10 + state * 4 + quadrant] = Glyph::from_fn(|x, y| {
                    let px = (x + (quadrant % 2) * CELL_W) * patch.width / (2 * CELL_W);
                    let py = (y + (quadrant / 2) * CELL_H) * patch.height / (2 * CELL_H);
                    patch.luma[py * patch.width + px] >= 96
                });
            }
        }
    }

    pub fn address(&self, glyph: u8) -> (char, bool) {
        let id = usize::from(glyph);
        (self.characters[id % 110], id >= 110)
    }
}

/// Expand a 5x7 character into the shared, centered 23x29 font cell.
pub(crate) fn text_glyph(rows: [u8; 7]) -> Glyph {
    Glyph::from_fn(|x, y| {
        if !(4..19).contains(&x) || !(4..25).contains(&y) {
            return false;
        }
        rows[(y - 4) / 3] & (1 << (4 - (x - 4) / 3)) != 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_features_are_finite_and_preserve_solid_ink() {
        let atlas = Atlas::universal();
        for (width, height) in [(11, 14), (5, 6)] {
            assert!(
                atlas.glyphs[BLANK as usize]
                    .resized_features(width, height)
                    .iter()
                    .all(|&x| x == 0.0)
            );
            assert!(
                atlas.glyphs[SOLID as usize]
                    .resized_features(width, height)
                    .iter()
                    .all(|&x| (x - 1.0).abs() < 0.00001)
            );
            for glyph in &atlas.glyphs {
                assert!(
                    glyph
                        .resized_features(width, height)
                        .iter()
                        .all(|x| x.is_finite() && (0.0..=1.00001).contains(x))
                );
            }
        }
    }
    #[test]
    fn budget_addresses_and_unique_scene() {
        let atlas = Atlas::universal();
        assert_eq!(CHARSET.chars().count(), 110);
        assert_eq!(atlas.glyphs.len(), 220);
        assert_eq!(atlas.scene.len(), 142);
        for (i, &a) in atlas.scene.iter().enumerate() {
            for &b in &atlas.scene[..i] {
                assert_ne!(atlas.glyphs[a as usize], atlas.glyphs[b as usize]);
            }
        }
        assert_eq!(atlas.address(0), (' ', false));
        assert_eq!(atlas.address(110), (' ', true));
        for glyph in &atlas.glyphs {
            for row in glyph.bitmap.chunks_exact(3) {
                assert_eq!(row[2] & 1, 0);
            }
        }
    }
}
