// SPDX-License-Identifier: GPL-3.0-only
//! PC controls and virtual HID reports. Physical A-E service keys do not
//! override the computer keyboard's WASD/QE bindings.
use minifb::{Key, Window};

pub fn keyboard_state(window: &Window) -> u32 {
    if !window.is_active() {
        return 0;
    }
    let bindings: [&[Key]; 22] = [
        &[Key::Up, Key::W],
        &[Key::Down, Key::S],
        &[Key::Left, Key::A],
        &[Key::Right, Key::D],
        &[Key::LeftCtrl, Key::RightCtrl],
        &[Key::Space],
        &[Key::LeftShift, Key::RightShift],
        &[Key::Q],
        &[Key::E],
        &[Key::Enter],
        &[Key::Escape],
        &[Key::Tab],
        &[Key::Key1],
        &[Key::Key2],
        &[Key::Key3],
        &[Key::Key4],
        &[Key::Key5],
        &[Key::Key6],
        &[Key::Key7],
        &[Key::Key8],
        &[Key::Key9],
        &[Key::Y],
    ];
    bindings.iter().enumerate().fold(0, |state, (index, keys)| {
        state
            | if keys.iter().any(|&key| window.is_key_down(key)) {
                1 << index
            } else {
                0
            }
    })
}

pub fn keyboard_report(window: &Window) -> [u8; 14] {
    report(|key| window.is_active() && window.is_key_down(key))
}

fn report(mut pressed: impl FnMut(Key) -> bool) -> [u8; 14] {
    let mut report = [0_u8; 14];
    report[0] = 1;
    let bindings: [(&[Key], usize, u8); 15] = [
        (&[Key::Up, Key::W], 4, 0x20),
        (&[Key::Down, Key::S], 4, 0x80),
        (&[Key::Left, Key::A], 4, 0x10),
        (&[Key::Right, Key::D], 4, 0x40),
        (&[Key::LeftCtrl, Key::RightCtrl], 1, 0x40),
        (&[Key::Space], 1, 1),
        (&[Key::LeftShift, Key::RightShift], 1, 2),
        (&[Key::Q], 1, 4),
        (&[Key::E], 2, 1),
        (&[Key::Enter], 2, 0x10),
        (&[Key::Escape], 4, 1),
        (&[Key::Tab], 3, 8),
        (&[Key::P], 2, 0x20),
        (&[Key::Backspace], 10, 2),
        (&[Key::Slash], 9, 64),
    ];
    for (keys, byte, mask) in bindings {
        if keys.iter().any(|&key| pressed(key)) {
            report[byte] |= mask;
        }
    }
    for (index, key) in [
        Key::Key1,
        Key::Key2,
        Key::Key3,
        Key::Key4,
        Key::Key5,
        Key::Key6,
        Key::Key7,
        Key::Key8,
        Key::Key9,
        Key::Key0,
    ]
    .into_iter()
    .enumerate()
    {
        if pressed(key) {
            let (byte, mask) = if index < 8 {
                (5, 1 << index)
            } else if index == 8 {
                (6, 1)
            } else {
                (6, 4)
            };
            report[byte] |= mask;
        }
    }
    for (index, key) in [
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
    ]
    .into_iter()
    .enumerate()
    .skip(5)
    {
        if pressed(key) {
            let bit = index + 4;
            report[6 + bit / 8] |= 1 << (bit % 8);
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{decode_mcdu, decode_raw};
    #[test]
    fn pc_movement_keys_do_not_trigger_physical_text_area_actions() {
        let report = report(|key| matches!(key, Key::A | Key::E));
        assert_eq!(decode_mcdu(&report), Some((1 << 2) | (1 << 8)));
        assert_eq!(decode_raw(&report), [0; 4]);
    }
}
