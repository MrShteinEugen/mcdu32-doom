// SPDX-License-Identifier: GPL-3.0-only
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub const DOOM_KEYS: [u8; 22] = [
    0xad, 0xaf, 0xac, 0xae, 0xa3, 0xa2, 0xb6, 0xa0, 0xa1, 13, 27, 9, b'1', b'2', b'3', b'4', b'5',
    b'6', b'7', b'8', b'9', b'y',
];

#[derive(Default)]
pub struct Inputs {
    pub navigation_owned: AtomicBool,
    panel: Mutex<[u8; 14]>,
    /// Commands are consumed only on the C engine thread (1=new game, 2=home).
    pub host_action: AtomicU32,
    pub suspended: AtomicBool,
    pub keyboard: AtomicU32,
    pub mcdu: AtomicU32,
    pub raw: [AtomicU64; 4],
}

impl Inputs {
    /// Keep physical keys distinct from their context-specific DOOM bindings.
    pub fn update_panel(&self, report: &[u8]) {
        if let Some(state) = decode_mcdu(report) {
            let mut panel = self.panel.lock().expect("panel report mutex");
            panel.fill(0);
            let length = report.len().min(panel.len());
            panel[..length].copy_from_slice(&report[..length]);
            if self.navigation_owned.load(Ordering::Relaxed) {
                let mut game = *panel;
                // The desktop router owns page keys before any engine callback
                // can turn them into pause, weapon or Escape actions.
                game[2] &= 0x1f;
                game[3] = 0;
                game[4] &= 0xf0;
                self.mcdu
                    .store(decode_mcdu(&game).unwrap_or(0), Ordering::Relaxed);
                self.update_raw(decode_raw(&game));
            } else {
                self.mcdu.store(state, Ordering::Relaxed);
                self.update_raw(decode_raw(report));
            }
        }
    }
    pub fn panel_snapshot(&self) -> [u8; 14] {
        *self.panel.lock().expect("panel report mutex")
    }
    pub fn release_panel(&self) {
        let mut released = [0; 14];
        released[0] = 1;
        self.update_panel(&released);
    }
    pub fn state(&self) -> u32 {
        self.keyboard.load(Ordering::Relaxed) | self.mcdu.load(Ordering::Relaxed)
    }

    /// Merge all input sources by DOOM key code, so duplicate bindings do not
    /// release a key while another source still holds it.
    pub fn key_bitmap(&self) -> [u64; 4] {
        if self.suspended.load(Ordering::Relaxed) {
            return [0; 4];
        }
        let mut keys = std::array::from_fn(|i| self.raw[i].load(Ordering::Relaxed));
        let state = self.state();
        for (index, &code) in DOOM_KEYS.iter().enumerate() {
            if state & (1 << index) != 0 {
                keys[code as usize / 64] |= 1 << (code % 64);
            }
        }
        keys
    }

    pub fn update_raw(&self, keys: [u64; 4]) {
        for (output, value) in self.raw.iter().zip(keys) {
            output.store(value, Ordering::Relaxed);
        }
    }
}

/// MCDU text keys, with A-E reserved for actions. +/- deliberately sends no
/// minus key: DOOM interprets it as shrinking the viewport.
pub fn decode_raw(report: &[u8]) -> [u64; 4] {
    let mut keys = [0; 4];
    if report.len() < 11 || report[0] != 1 {
        return keys;
    }
    let mut set = |code: u8| {
        keys[code as usize / 64] |= 1 << (code % 64);
    };
    for index in 0..26 {
        let bit = 4 + index;
        if report[6 + bit / 8] & (1 << (bit % 8)) != 0 {
            set(match index {
                0 => 0xa2, // A: use
                1 => 0xb6, // B: run
                2 => 9,    // C: automap
                3 => 13,   // D: confirm
                4 => 27,   // E: escape/menu
                _ => b'a' + index as u8,
            });
        }
    }
    for (byte, mask, code) in [
        (6, 2, b'.'),
        (6, 4, b'0'),
        (9, 64, b'/'),
        (9, 128, 0xa2),
        (10, 2, 127),
        (2, 0x20, 0xff),
        (2, 0x40, b'3'),
        (2, 0x80, b'4'),
        (3, 1, b'5'),
        (3, 0x10, 9),
        (3, 0x20, b'6'),
        (3, 0x40, b'7'),
        (3, 0x80, 0xa2),
        (4, 4, 13),
        (10, 1, 0xa3), // OVERFLY: fire
    ] {
        if report[byte] & mask != 0 {
            set(code);
        }
    }
    keys
}

/// Decode report 0x01 only; font acknowledgements are not keyboard reports.
pub fn decode_mcdu(report: &[u8]) -> Option<u32> {
    if report.len() < 11 || report[0] != 1 {
        return None;
    }
    let mappings = [
        (4, 0x20, 0),
        (4, 0x80, 1),
        (4, 0x10, 2),
        (4, 0x40, 3),
        (1, 0x40, 4), // R1: fire
        (1, 0x01, 5), // L1: use
        (1, 0x02, 6), // L2: run
        (1, 0x04, 7),
        (2, 0x01, 8), // L3/R3: strafe
        (1, 0x08, 1),
        (2, 0x02, 0), // L4/R4: back/forward
        (1, 0x10, 2),
        (2, 0x04, 3), // L5/R5: turn
        (1, 0x20, 10),
        (2, 0x08, 4), // L6/R6: menu/fire
        (2, 0x10, 9),
        (4, 0x01, 10),
        (3, 0x08, 11), // DIR/MENU/F-PLN
        (5, 0x01, 12),
        (5, 0x02, 13),
        (5, 0x04, 14),
        (5, 0x08, 15),
        (5, 0x10, 16),
        (5, 0x20, 17),
        (5, 0x40, 18),
        (5, 0x80, 19),
        (6, 0x01, 20),
        (9, 0x10, 21), // 9 / Y
    ];
    Some(mappings.iter().fold(0, |state, &(byte, mask, key)| {
        state
            | if report[byte] & mask != 0 {
                1 << key
            } else {
                0
            }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_page_keys_do_not_reach_doom_but_remain_in_physical_snapshot() {
        let input = Inputs::default();
        input.navigation_owned.store(true, Ordering::Relaxed);
        let mut report = [0; 14];
        report[0] = 1;
        report[4] = 1;
        report[2] = 0x80;
        input.update_panel(&report);
        assert_eq!(input.key_bitmap(), [0; 4]);
        assert_eq!(input.panel_snapshot(), report);
        report[1] = 1;
        input.update_panel(&report);
        assert_ne!(input.key_bitmap()[0xa2 / 64] & (1 << (0xa2 % 64)), 0);
        input.release_panel();
        assert_eq!(input.key_bitmap(), [0; 4]);
    }
    #[test]
    fn text_area_actions_do_not_leak_letters_or_resize_keys() {
        for (byte, mask, code) in [
            (6, 0x10, 0xa2),
            (6, 0x20, 0xb6),
            (6, 0x40, 9),
            (6, 0x80, 13),
            (7, 1, 27),
            (10, 1, 0xa3),
        ] {
            let mut report = [0; 14];
            report[0] = 1;
            report[byte] = mask;
            let mut expected = [0_u64; 4];
            expected[code / 64] = 1 << (code % 64);
            assert_eq!(decode_raw(&report), expected);
        }
        let mut report = [0; 14];
        report[0] = 1;
        report[6] = 8;
        assert_eq!(decode_raw(&report), [0; 4]);
        assert_eq!(decode_mcdu(&report), Some(0));
    }
    #[test]
    fn releasing_one_source_keeps_the_other_binding_held() {
        let input = Inputs::default();
        input.keyboard.store(1 << 5, Ordering::Relaxed);
        let mut report = [0; 14];
        report[0] = 1;
        report[9] = 128;
        input.update_raw(decode_raw(&report));
        input.keyboard.store(0, Ordering::Relaxed);
        assert_ne!(input.key_bitmap()[0xa2 / 64] & (1_u64 << (0xa2 % 64)), 0);
        input.update_raw([0; 4]);
        assert_eq!(input.key_bitmap(), [0; 4]);
    }
    #[test]
    fn simultaneous_and_release() {
        let mut report = [0; 14];
        report[0] = 1;
        report[4] = 0x20;
        report[1] = 0x40;
        assert_eq!(decode_mcdu(&report), Some(1 | (1 << 4)));
        report[1] = 0;
        report[4] = 0;
        assert_eq!(decode_mcdu(&report), Some(0));
        report[0] = 0xf0;
        assert_eq!(decode_mcdu(&report), None);
    }
}
