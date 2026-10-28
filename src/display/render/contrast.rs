// SPDX-License-Identifier: GPL-3.0-only
//! Pixel transforms applied before glyph matching.
use super::rgb;

/// Neutral scenery with expanded tonal range and a modest edge boost. Warm
/// monster accents are already supplied by the semantic sprite pass; the HUD
/// is composed afterwards and keeps its colors.
pub(super) fn contrast_scene(
    pixels: &[u32],
    mask: &[u8],
    width: usize,
    height: usize,
    output: &mut Vec<u32>,
) {
    output.resize(pixels.len(), 0);
    let luma = |i: usize| {
        let c = rgb(pixels[i]);
        c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114
    };
    for (i, pixel) in output.iter_mut().enumerate() {
        if mask.get(i) == Some(&1) {
            *pixel = pixels[i];
            continue;
        }
        let x = i % width;
        let y = i / width;
        let center = luma(i);
        let blur = (center * 4.0
            + luma(y * width + x.saturating_sub(1))
            + luma(y * width + (x + 1).min(width - 1))
            + luma(y.saturating_sub(1) * width + x)
            + luma((y + 1).min(height - 1) * width + x))
            / 8.0;
        let sharpened = center + (center - blur) * 0.8;
        let level = ((sharpened - 8.0) * 255.0 / 112.0)
            .clamp(0.0, 255.0)
            .round() as u32;
        *pixel = level * 0x010101;
    }
}

/// Preserve scenery exactly; emphasize only visible sprite pixels. The contour
/// lies inside the ownership mask, so it cannot expand through an occluding wall.
pub(super) fn enhance_sprites(pixels: &[u32], mask: &[u8], width: usize, output: &mut Vec<u32>) {
    output.resize(pixels.len(), 0);
    output.copy_from_slice(pixels);
    for (i, &class) in mask.iter().enumerate() {
        if class == 0 {
            continue;
        }
        let channels = rgb(pixels[i]);
        let luma = channels[0] * 0.299 + channels[1] * 0.587 + channels[2] * 0.114;
        if class == 1 {
            let edge = i % width == 0
                || i % width == width - 1
                || i < width
                || i + width >= mask.len()
                || [i - 1, i + 1, i - width, i + width]
                    .iter()
                    .any(|&n| mask[n] != class);
            output[i] = if edge {
                0xffff60
            } else {
                let brightness = (150.0 + luma * 0.40).min(250.0) as u32;
                // Warm body tones with luminance variation for face/limb detail.
                (brightness << 16) | ((brightness * 165 / 255) << 8) | (brightness / 8)
            };
        } else if class == 2 {
            let brightness = (55.0 + luma * 1.25).min(255.0) as u32;
            output[i] = brightness * 0x010101;
        }
    }
}
