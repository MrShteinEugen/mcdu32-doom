// SPDX-License-Identifier: GPL-3.0-only
use crate::display::{
    CELL_H, CELL_W, COLS, Cell, PALETTE, ROWS, Screen,
    atlas::{Atlas, SAMPLE},
    color::rgb,
    hud::Hud,
};
use crate::doom::hud::HudState;
use anyhow::{Result, ensure};
mod contrast;
use contrast::{contrast_scene, enhance_sprites};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
    #[default]
    Color,
    Contrast,
}

struct Candidate {
    id: u8,
    mask: [f32; SAMPLE * SAMPLE],
    energy: f32,
    roughness: f32,
}

pub struct Renderer {
    candidates: Vec<Candidate>,
    previous: Vec<Cell>,
    hud: Hud,
    cols: usize,
    rows: usize,
    enhanced: Vec<u32>,
    toned: Vec<u32>,
    style: Style,
}

impl Renderer {
    /// Compose scene, status bar and the engine's semantic UI text.
    pub fn convert_frame(
        &mut self,
        frame: &crate::doom::engine::Frame,
        enhanced: bool,
    ) -> Result<Vec<Cell>> {
        let mut cells = self.convert_semantic(
            &frame.pixels,
            if enhanced { &frame.semantics } else { &[] },
            crate::doom::engine::DOOM_W,
            crate::doom::engine::DOOM_H,
            frame.hud,
        )?;
        crate::display::text::apply(
            &frame.text,
            &mut cells,
            self.cols,
            self.rows,
            frame.hud.active != 0,
        );
        Ok(cells)
    }
    pub fn new(atlas: &Atlas) -> Self {
        Self::for_grid(atlas, COLS, ROWS).expect("valid default grid")
    }

    pub fn for_grid(atlas: &Atlas, cols: usize, rows: usize) -> Result<Self> {
        Self::for_cells(atlas, cols, rows, CELL_W, CELL_H)
    }

    pub fn for_cells(
        atlas: &Atlas,
        cols: usize,
        rows: usize,
        cell_w: usize,
        cell_h: usize,
    ) -> Result<Self> {
        ensure!(
            (24..=96).contains(&cols) && (4..=64).contains(&rows),
            "grid must be 24..96 columns and 4..64 rows"
        );
        ensure!(
            rows > crate::display::layout::panel_rows(cols),
            "grid needs at least one gameplay row above the status bar"
        );
        ensure!(
            cell_w > 0 && cell_h > 0,
            "glyph cell dimensions must be positive"
        );
        Ok(Self {
            candidates: atlas
                .scene
                .iter()
                .map(|&id| {
                    let glyph = &atlas.glyphs[id as usize];
                    let mask = if cell_w == CELL_W && cell_h == CELL_H {
                        glyph.features()
                    } else {
                        glyph.resized_features(cell_w, cell_h)
                    };
                    let energy = mask.iter().map(|x| x * x).sum();
                    let mut transitions = 0;
                    let mut pairs = 0;
                    for y in 0..cell_h {
                        for x in 0..cell_w {
                            let pixel =
                                |px: usize, py: usize| glyph.resized_pixel(px, py, cell_w, cell_h);
                            if x + 1 < cell_w {
                                transitions += usize::from(pixel(x, y) != pixel(x + 1, y));
                                pairs += 1;
                            }
                            if y + 1 < cell_h {
                                transitions += usize::from(pixel(x, y) != pixel(x, y + 1));
                                pairs += 1;
                            }
                        }
                    }
                    Candidate {
                        id,
                        mask,
                        energy,
                        roughness: transitions as f32 / pairs.max(1) as f32,
                    }
                })
                .collect(),
            previous: vec![
                Cell {
                    glyph: crate::display::atlas::BLANK,
                    color: 0,
                };
                cols * rows
            ],
            hud: Hud::default(),
            cols,
            rows,
            enhanced: Vec::new(),
            toned: Vec::new(),
            style: Style::Color,
        })
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn set_style(&mut self, style: Style) {
        if self.style != style {
            self.style = style;
            self.previous.fill(Cell {
                glyph: crate::display::atlas::BLANK,
                color: 0,
            });
        }
    }

    /// Convert a complete DOOM RGB frame; active gameplay excludes its old status bar.
    pub fn convert(
        &mut self,
        pixels: &[u32],
        width: usize,
        height: usize,
        hud: HudState,
    ) -> Result<Screen> {
        ensure!(
            self.cols == COLS && self.rows == ROWS,
            "use convert_grid for nonstandard grids"
        );
        self.convert_grid(pixels, width, height, hud)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("screen size mismatch"))
    }

    pub fn convert_grid(
        &mut self,
        pixels: &[u32],
        width: usize,
        height: usize,
        hud: HudState,
    ) -> Result<Vec<Cell>> {
        self.convert_semantic(pixels, &[], width, height, hud)
    }

    /// Give visible enemies stronger contrast and fitting priority without new glyphs.
    /// Empty ownership disables enhancement and reproduces the original conversion.
    pub fn convert_semantic(
        &mut self,
        pixels: &[u32],
        ownership: &[u8],
        width: usize,
        height: usize,
        hud: HudState,
    ) -> Result<Vec<Cell>> {
        ensure!(
            width > 0 && height > 0 && width.checked_mul(height) == Some(pixels.len()),
            "invalid framebuffer dimensions"
        );
        ensure!(
            ownership.is_empty() || ownership.len() == pixels.len(),
            "invalid sprite mask dimensions"
        );
        let enhanced = !ownership.is_empty() && hud.active != 0;
        if enhanced {
            enhance_sprites(pixels, ownership, width, &mut self.enhanced);
        }
        let pixels = if enhanced { &self.enhanced[..] } else { pixels };
        if self.style == Style::Contrast {
            contrast_scene(
                pixels,
                if enhanced { ownership } else { &[] },
                width,
                height,
                &mut self.toned,
            );
        }
        let pixels = if self.style == Style::Contrast {
            &self.toned[..]
        } else {
            pixels
        };
        let rows = crate::display::layout::scene_rows(self.cols, self.rows, hud.active != 0);
        let source_h = if hud.active != 0 {
            height * 168 / 200
        } else {
            height
        };
        let mut screen = self.previous.clone();
        for cy in 0..rows {
            for cx in 0..self.cols {
                let mut has_enemy = false;
                let mut samples = [[0.0_f32; 3]; SAMPLE * SAMPLE];
                let mut total_energy = 0.0;
                let mut weights = [1.0_f32; SAMPLE * SAMPLE];
                for sy in 0..SAMPLE {
                    for sx in 0..SAMPLE {
                        let x = ((cx * SAMPLE + sx) * 2 + 1) * width / (self.cols * SAMPLE * 2);
                        let y = ((cy * SAMPLE + sy) * 2 + 1) * source_h / (rows * SAMPLE * 2);
                        let color = pixels[y.min(height - 1) * width + x.min(width - 1)];
                        let rgb = rgb(color);
                        let weight = if enhanced
                            && ownership[y.min(height - 1) * width + x.min(width - 1)] == 1
                        {
                            has_enemy = true;
                            4.0
                        } else {
                            1.0
                        };
                        weights[sy * SAMPLE + sx] = weight;
                        total_energy += weight * rgb.iter().map(|x| x * x).sum::<f32>();
                        samples[sy * SAMPLE + sx] = rgb;
                    }
                }
                let previous = self.previous[cy * self.cols + cx];
                let mut best = Cell::default();
                let mut best_error = f32::INFINITY;
                let mut previous_error = f32::INFINITY;
                for candidate in &self.candidates {
                    // Avoid fine halftone patterns that turn large flat surfaces
                    // into visual noise on the compact physical raster.
                    if self.style == Style::Contrast && candidate.roughness > 0.35 {
                        continue;
                    }
                    let mut sum = [0.0_f32; 3];
                    let mut energy = 0.0;
                    for ((sample, mask), weight) in samples.iter().zip(candidate.mask).zip(weights)
                    {
                        energy += weight * mask * mask;
                        for channel in 0..3 {
                            sum[channel] += weight * sample[channel] * mask;
                        }
                    }
                    for (color, &packed) in PALETTE.iter().enumerate() {
                        if self.style == Style::Contrast
                            && ![0, 2, 9].contains(&color)
                            && !(has_enemy && [1, 7].contains(&color))
                        {
                            continue;
                        }
                        let rgb = rgb(packed);
                        let error = total_energy
                            + (if enhanced { energy } else { candidate.energy })
                                * rgb.iter().map(|x| x * x).sum::<f32>()
                            - 2.0 * (sum[0] * rgb[0] + sum[1] * rgb[1] + sum[2] * rgb[2]);
                        if candidate.id == previous.glyph && color == previous.color as usize {
                            previous_error = error;
                        }
                        if error < best_error {
                            best_error = error;
                            best = Cell {
                                glyph: candidate.id,
                                color: color as u8,
                            };
                        }
                    }
                }
                // Keep the existing tile unless the improvement is perceptible.
                if previous_error <= best_error + total_energy * 0.015 {
                    best = previous;
                }
                screen[cy * self.cols + cx] = best;
            }
        }
        if hud.active != 0 {
            self.hud.apply_grid(hud, &mut screen, self.cols, self.rows);
        }
        self.previous.clone_from(&screen);
        Ok(screen)
    }
}

/// Reconstruct exactly the chosen raster tiles, rather than displaying source pixels.
pub fn rasterize(atlas: &Atlas, screen: &Screen, output: &mut [u32]) {
    rasterize_grid(atlas, screen, COLS, ROWS, CELL_W, CELL_H, output);
}

/// Simulated compact glyph cells. This resampling is not proof of hardware support.
pub fn rasterize_grid(
    atlas: &Atlas,
    screen: &[Cell],
    cols: usize,
    rows: usize,
    cell_w: usize,
    cell_h: usize,
    output: &mut [u32],
) {
    assert_eq!(screen.len(), cols * rows);
    assert_eq!(output.len(), cols * cell_w * rows * cell_h);
    let width = cols * cell_w;
    for (index, cell) in screen.iter().enumerate() {
        let glyph = &atlas.glyphs[cell.glyph as usize];
        for y in 0..cell_h {
            for x in 0..cell_w {
                output[(index / cols * cell_h + y) * width + index % cols * cell_w + x] =
                    if glyph.resized_pixel(x, y, cell_w, cell_h) {
                        PALETTE[cell.color as usize]
                    } else {
                        0
                    };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{HEIGHT, WIDTH};
    #[test]
    fn enhancement_preserves_scenery_and_stays_inside_visible_silhouette() {
        let pixels = vec![0x202020; 25];
        let mut mask = vec![0; 25];
        for y in 1..4 {
            for x in 1..4 {
                mask[y * 5 + x] = 1;
            }
        }
        let mut output = Vec::new();
        enhance_sprites(&pixels, &mask, 5, &mut output);
        for (i, &class) in mask.iter().enumerate() {
            if class == 0 {
                assert_eq!(output[i], pixels[i]);
            }
        }
        assert_eq!(output[6], 0xffff60);
        assert!(rgb(output[12])[0] > rgb(pixels[12])[0]);
        assert_ne!(output[12], output[6]);
        // A wall occluding the right side must remain untouched.
        mask[13] = 0;
        enhance_sprites(&pixels, &mask, 5, &mut output);
        assert_eq!(output[13], pixels[13]);
        assert_eq!(output[12], 0xffff60);
    }

    #[test]
    fn enhancement_handles_frame_edges_and_invalid_masks() {
        let mut output = Vec::new();
        enhance_sprites(&[0; 4], &[1; 4], 2, &mut output);
        assert_eq!(output, vec![0xffff60; 4]);
        let atlas = Atlas::universal();
        assert!(
            Renderer::new(&atlas)
                .convert_semantic(&[0; 4], &[1], 2, 2, HudState::default())
                .is_err()
        );
    }

    #[test]
    fn inactive_game_does_not_apply_stale_sprite_mask() {
        let atlas = Atlas::universal();
        let pixels = vec![0; 320 * 200];
        let mask = vec![1; pixels.len()];
        let result = Renderer::new(&atlas)
            .convert_semantic(&pixels, &mask, 320, 200, HudState::default())
            .expect("frame");
        assert!(result.iter().all(|cell| cell.color == 0));
    }
    #[test]
    fn solid_and_black_are_exact() {
        let atlas = Atlas::universal();
        let mut renderer = Renderer::new(&atlas);
        let screen = renderer
            .convert(&vec![0xffffff; 320 * 200], 320, 200, HudState::default())
            .expect("frame");
        assert!(
            screen
                .iter()
                .all(|c| c.glyph == crate::display::atlas::SOLID && c.color == 2)
        );
        let screen = renderer
            .convert(&vec![0; 320 * 200], 320, 200, HudState::default())
            .expect("frame");
        let mut output = vec![0; WIDTH * HEIGHT];
        rasterize(&atlas, &screen, &mut output);
        assert!(output.iter().all(|&c| c == 0));
    }
    #[test]
    fn rejects_malformed_frame() {
        let atlas = Atlas::universal();
        let mut renderer = Renderer::new(&atlas);
        assert!(renderer.convert(&[], 0, 200, HudState::default()).is_err());
    }

    #[test]
    fn contrast_uses_neutral_scenery_preserves_hud_colors_and_filters_halftones() {
        let atlas = Atlas::universal();
        let mut renderer = Renderer::for_cells(&atlas, 53, 31, 11, 14).expect("renderer");
        renderer.set_style(Style::Contrast);
        let frame: Vec<_> = (0..320 * 200)
            .map(|i| {
                let level = 30 + (i % 320) * 130 / 320;
                ((level + 30) as u32) << 16 | ((level + 15) as u32) << 8 | level as u32
            })
            .collect();
        let hud = HudState {
            active: 1,
            health: 100,
            ammo: 30,
            armor: 50,
            ..HudState::default()
        };
        let screen = renderer
            .convert_semantic(&frame, &[], 320, 200, hud)
            .expect("frame");
        assert!(
            screen[..53 * 26]
                .iter()
                .all(|c| [0, 2, 9].contains(&c.color))
        );
        assert!(screen[..53 * 26].iter().all(|c| {
            renderer
                .candidates
                .iter()
                .any(|g| g.id == c.glyph && g.roughness <= 0.35)
        }));
        assert!(screen[53 * 26..].iter().any(|c| c.color == 4));
        assert!(screen[53 * 26..].iter().any(|c| c.color == 7));
        assert!(screen[..53 * 26].iter().any(|c| c.color == 2));
        let mut toned = Vec::new();
        contrast_scene(&[0, 0xffffff, 0x303030, 0x909090], &[], 2, 2, &mut toned);
        assert_eq!(toned[0], 0);
        assert_eq!(toned[1], 0xffffff);
        assert!(toned[3] > toned[2]);
    }

    #[test]
    fn contrast_retains_warm_enemy_accents() {
        let atlas = Atlas::universal();
        let mut renderer = Renderer::for_cells(&atlas, 53, 31, 11, 14).expect("renderer");
        renderer.set_style(Style::Contrast);
        let pixels = vec![0x505050; 320 * 200];
        let mut ownership = vec![0; pixels.len()];
        for y in 45..120 {
            ownership[y * 320 + 130..y * 320 + 190].fill(1);
        }
        let screen = renderer
            .convert_semantic(
                &pixels,
                &ownership,
                320,
                200,
                HudState {
                    active: 1,
                    ..HudState::default()
                },
            )
            .expect("frame");
        assert!(screen[..53 * 26].iter().any(|c| [1, 7].contains(&c.color)));
        assert!(
            screen[..53 * 26]
                .iter()
                .all(|c| [0, 1, 2, 7, 9].contains(&c.color))
        );
    }
}
