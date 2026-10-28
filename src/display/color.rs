// SPDX-License-Identifier: GPL-3.0-only
//! Color conversions shared by glyph matching and pixel transforms.
pub(super) fn rgb(color: u32) -> [f32; 3] {
    [
        ((color >> 16) & 255) as f32,
        ((color >> 8) & 255) as f32,
        (color & 255) as f32,
    ]
}
