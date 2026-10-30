// SPDX-License-Identifier: GPL-3.0-only
//! Offline glyph and font-report export.
use crate::{
    device::protocol,
    display::{CELL_H, CELL_W, HUD_GLYPHS, atlas::Atlas},
    output::capture::save_png,
};
use anyhow::Result;
use std::path::Path;

pub(super) fn export(out: &Path) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let atlas = Atlas::universal();
    let width = 20 * CELL_W;
    let height = 11 * CELL_H;
    let mut pixels = vec![0; width * height];
    for (id, glyph) in atlas.glyphs.iter().enumerate() {
        for y in 0..CELL_H {
            for x in 0..CELL_W {
                pixels[(id / 20 * CELL_H + y) * width + id % 20 * CELL_W + x] = if glyph.pixel(x, y)
                {
                    if id < HUD_GLYPHS { 0xffcc66 } else { 0xffffff }
                } else {
                    0x101014
                };
            }
        }
    }
    save_png(&out.join("atlas.png"), &pixels, width, height)?;
    std::fs::write(
        out.join("font-reports.json"),
        serde_json::to_vec(&protocol::font_packets(&atlas, 200)?)?,
    )?;
    println!(
        "220 glyphs: 80 UI/text/portrait, 140 scene, 2 UI masks reused. Exported to {}",
        out.display()
    );
    Ok(())
}
