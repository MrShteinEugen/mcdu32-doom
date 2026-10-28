// SPDX-License-Identifier: GPL-3.0-only
//! Golden contracts captured before the publication refactor. These protect the
//! font transaction and cell selection that were checked on the physical panel.
use mcdu_doom::doom::hud::HudState;
use mcdu_doom::{
    device::protocol::{display_setup, encode_cells},
    display::{
        Profile,
        atlas::Atlas,
        render::{Renderer, Style},
    },
    doom::engine::Frame,
};

fn fingerprint(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn scene() -> Frame {
    let mut frame = Frame::default();
    for (i, pixel) in frame.pixels.iter_mut().enumerate() {
        let x = i % 320;
        let y = i / 320;
        *pixel = (((x * 171 / 320) as u32) << 16)
            | (((y * 155 / 200) as u32) << 8)
            | ((x + y) % 137) as u32;
        if (110..155).contains(&x) && (30..110).contains(&y) {
            frame.semantics[i] = 1;
        }
        if (140..185).contains(&x) && (120..168).contains(&y) {
            frame.semantics[i] = 2;
        }
    }
    frame.hud = HudState {
        active: 1,
        health: 86,
        armor: 35,
        ammo: 42,
        weapon: 1,
        owned: 7,
        keys: 5,
        damage: 0,
        ammo_counts: [42, 12, 80, 3],
        ammo_max: [200, 50, 300, 50],
        ..HudState::default()
    };
    frame.text.count = 1;
    let message = b"PICKED UP A MEDIKIT.";
    frame.text.lines[0].bytes[..message.len()].copy_from_slice(message);
    frame
}

#[test]
fn confirmed_font_and_rendering_contract() {
    let atlas = Atlas::universal();
    let frame = scene();
    for (profile, expected_font, expected_screens) in [
        (
            Profile::Full,
            0x226ee2c73ae7cee4,
            [0x730699a5b460d9d5, 0x50e581545a2a296d],
        ),
        (
            Profile::Dense,
            0x96b509669d2a6e73,
            [0xca23a30b120ac74f, 0x16b6bd3986dcfd43],
        ),
        (
            Profile::Standard,
            0x7ea9045a9df36db2,
            [0x70103f153c7c9d5b, 0x8721f1f2a12dcee9],
        ),
    ] {
        let geometry = profile.geometry();
        let font = display_setup(&atlas, geometry, false, false).expect("font setup");
        assert_eq!(
            fingerprint(font.into_iter().flatten()),
            expected_font,
            "{profile:?} font"
        );
        for (style, expected) in [Style::Color, Style::Contrast]
            .into_iter()
            .zip(expected_screens)
        {
            let mut renderer = Renderer::for_cells(
                &atlas,
                geometry.columns as usize,
                geometry.rows as usize,
                geometry.glyph_width as usize,
                geometry.glyph_height as usize,
            )
            .expect("renderer");
            renderer.set_style(style);
            let cells = renderer.convert_frame(&frame, true).expect("frame");
            assert_eq!(
                fingerprint(
                    encode_cells(&atlas, &cells)
                        .expect("screen")
                        .into_iter()
                        .flatten()
                ),
                expected,
                "{profile:?} {style:?}"
            );
        }
    }
}
