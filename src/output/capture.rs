// SPDX-License-Identifier: GPL-3.0-only
//! Files written by both the physical player and the device-free emulator.
use crate::{
    display::{
        Geometry,
        atlas::Atlas,
        render::{Renderer, Style, rasterize_grid},
    },
    doom::engine::{DOOM_H, DOOM_W, Frame},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

pub struct Capture {
    directory: PathBuf,
}
impl Capture {
    pub fn new(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("create capture directory {}", directory.display()))?;
        Ok(Self {
            directory: directory.to_owned(),
        })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn image(&self, name: &str, pixels: &[u32], width: usize, height: usize) -> Result<()> {
        save_png(&self.directory.join(name), pixels, width, height)
    }
    pub fn json(&self, name: &str, value: &impl Serialize) -> Result<()> {
        let path = self.directory.join(name);
        std::fs::write(&path, serde_json::to_vec_pretty(value)?)
            .with_context(|| format!("write {}", path.display()))
    }
    pub fn source(&self, frame: &Frame) -> Result<()> {
        self.image("source.png", &frame.pixels, DOOM_W, DOOM_H)?;
        self.json(
            "text.json",
            &frame
                .text
                .lines()
                .iter()
                .map(|line| line.value())
                .collect::<Vec<_>>(),
        )
    }

    /// Compare styles and sprite enhancement using the identical source frame.
    pub fn comparisons(
        &self,
        atlas: &Atlas,
        frame: &Frame,
        geometry: Geometry,
        style: Style,
    ) -> Result<()> {
        save_style_comparison(atlas, frame, geometry, self.directory())?;
        for (name, enhanced) in [
            ("glyphs-enhanced.png", true),
            ("glyphs-baseline.png", false),
        ] {
            let pixels = render_pixels(atlas, frame, geometry, style, enhanced)?;
            self.image(name, &pixels, geometry.width(), geometry.height())?;
        }
        let mask: Vec<_> = frame
            .semantics
            .iter()
            .map(|&class| match class {
                1 => 0xffff00,
                2 => 0x00ffff,
                _ => 0,
            })
            .collect();
        self.image("sprite-mask.png", &mask, DOOM_W, DOOM_H)
    }
}

/// Render both styles from exactly the same source frame for visual comparison.
pub fn save_style_comparison(
    atlas: &Atlas,
    frame: &Frame,
    geometry: Geometry,
    out: &Path,
) -> Result<()> {
    let width = geometry.width();
    let height = geometry.height();
    let mut comparison = vec![0x303030; (width * 2 + 11) * height];
    for (side, style) in [Style::Color, Style::Contrast].into_iter().enumerate() {
        let image = render_pixels(atlas, frame, geometry, style, true)?;
        save_png(
            &out.join(if side == 0 {
                "color.png"
            } else {
                "contrast.png"
            }),
            &image,
            width,
            height,
        )?;
        for y in 0..height {
            let start = y * (width * 2 + 11) + side * (width + 11);
            comparison[start..start + width].copy_from_slice(&image[y * width..(y + 1) * width]);
        }
    }
    save_png(
        &out.join("style-comparison.png"),
        &comparison,
        width * 2 + 11,
        height,
    )
}

fn render_pixels(
    atlas: &Atlas,
    frame: &Frame,
    geometry: Geometry,
    style: Style,
    enhanced: bool,
) -> Result<Vec<u32>> {
    let mut renderer = Renderer::for_cells(
        atlas,
        usize::from(geometry.columns),
        usize::from(geometry.rows),
        usize::from(geometry.glyph_width),
        usize::from(geometry.glyph_height),
    )?;
    renderer.set_style(style);
    let cells = renderer.convert_frame(frame, enhanced)?;
    let mut pixels = vec![0; geometry.width() * geometry.height()];
    rasterize_grid(
        atlas,
        &cells,
        usize::from(geometry.columns),
        usize::from(geometry.rows),
        usize::from(geometry.glyph_width),
        usize::from(geometry.glyph_height),
        &mut pixels,
    );
    Ok(pixels)
}

pub fn save_png(path: &Path, pixels: &[u32], width: usize, height: usize) -> Result<()> {
    ensure!(
        width > 0 && height > 0 && width.checked_mul(height) == Some(pixels.len()),
        "invalid PNG dimensions"
    );
    let width = u32::try_from(width).context("PNG width exceeds u32")?;
    let height = u32::try_from(height).context("PNG height exceeds u32")?;
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let bytes: Vec<_> = pixels
        .iter()
        .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8])
        .collect();
    writer.write_image_data(&bytes)?;
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn png_round_trip_preserves_dimensions_and_rgb_channels() {
        let directory = PathBuf::from("target/capture-tests").join(std::process::id().to_string());
        std::fs::create_dir_all(&directory).expect("capture directory");
        let path = directory.join("rgb.png");
        save_png(&path, &[0xff0000, 0x00ff00, 0x0000ff], 3, 1).expect("save PNG");
        let decoder = png::Decoder::new(BufReader::new(File::open(path).expect("open PNG")));
        let mut reader = decoder.read_info().expect("PNG header");
        let mut pixels = vec![0; reader.output_buffer_size().expect("buffer size")];
        let frame = reader.next_frame(&mut pixels).expect("decode PNG pixels");
        assert_eq!((frame.width, frame.height), (3, 1));
        assert_eq!(
            &pixels[..frame.buffer_size()],
            &[255, 0, 0, 0, 255, 0, 0, 0, 255]
        );
        reader.finish().expect("complete PNG");
    }

    #[test]
    fn invalid_dimensions_are_rejected_before_creating_a_file() {
        let path = Path::new("target/invalid-capture-dimensions.png");
        for (pixels, width, height) in [(&[][..], 0, 0), (&[0][..], 2, 1)] {
            let error = save_png(path, pixels, width, height).expect_err("invalid dimensions");
            assert!(error.to_string().contains("invalid PNG dimensions"));
        }
        assert!(!path.exists());
    }
}
