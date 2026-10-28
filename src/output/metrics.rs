// SPDX-License-Identifier: GPL-3.0-only
//! Serializable snapshots shared by physical and virtual runs.
use crate::{
    display::{Geometry, HUD_GLYPHS, atlas::Atlas, layout::panel_rows},
    doom::engine::Frame,
};
use serde::Serialize;
use std::time::Duration;

#[derive(Default)]
pub struct RenderTimings {
    frames: u64,
    total_us: u128,
    max_us: u128,
}
impl RenderTimings {
    pub fn record(&mut self, elapsed: Duration) {
        let micros = elapsed.as_micros();
        self.frames += 1;
        self.total_us += micros;
        self.max_us = self.max_us.max(micros);
    }
    pub fn frames(&self) -> u64 {
        self.frames
    }
    pub fn snapshot(&self) -> RenderMetrics {
        RenderMetrics {
            converted_frames: self.frames,
            render_mean_ms: if self.frames == 0 {
                0.0
            } else {
                self.total_us as f64 / self.frames as f64 / 1000.0
            },
            render_max_ms: self.max_us as f64 / 1000.0,
        }
    }
}

#[derive(Serialize)]
pub struct RenderMetrics {
    pub converted_frames: u64,
    pub render_mean_ms: f64,
    pub render_max_ms: f64,
}

#[derive(Serialize)]
pub struct DisplayMetrics {
    columns: u16,
    rows: u16,
    cell_width: u8,
    cell_height: u8,
    display_width: usize,
    display_height: usize,
}
impl From<Geometry> for DisplayMetrics {
    fn from(geometry: Geometry) -> Self {
        Self {
            columns: geometry.columns,
            rows: geometry.rows,
            cell_width: geometry.glyph_width,
            cell_height: geometry.glyph_height,
            display_width: geometry.width(),
            display_height: geometry.height(),
        }
    }
}

#[derive(Serialize)]
pub struct GameMetrics {
    engine_frames: u64,
    health: i32,
    ammo: i32,
    armor: i32,
    active: bool,
    viewport: [usize; 2],
    hud_rows: usize,
    ui_glyphs: usize,
    scene_shapes: usize,
}
impl GameMetrics {
    pub fn new(frame: &Frame, atlas: &Atlas, columns: usize) -> Self {
        Self {
            engine_frames: frame.generation,
            health: frame.hud.health,
            ammo: frame.hud.ammo,
            armor: frame.hud.armor,
            active: frame.hud.active != 0,
            viewport: frame.viewport,
            hud_rows: panel_rows(columns),
            ui_glyphs: HUD_GLYPHS,
            scene_shapes: atlas.scene.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timings_handle_empty_runs_and_report_mean_and_maximum() {
        let mut timings = RenderTimings::default();
        assert_eq!(timings.snapshot().render_mean_ms, 0.0);
        timings.record(Duration::from_micros(1000));
        timings.record(Duration::from_micros(2000));
        let snapshot = timings.snapshot();
        assert_eq!(snapshot.converted_frames, 2);
        assert_eq!(snapshot.render_mean_ms, 1.5);
        assert_eq!(snapshot.render_max_ms, 2.0);
    }
}
