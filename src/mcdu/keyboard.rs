// SPDX-License-Identifier: GPL-3.0-only
//! Physical panel keys; aircraft text entry never passes through game bindings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Menu,
    Dir,
    Prog,
    Perf,
    Init,
    Data,
    FlightPlan,
    RadNav,
    Fuel,
    Secondary,
    Atc,
    Airport,
    Left,
    Right,
    Up,
    Down,
    Lsk { right: bool, row: usize },
    Character(char),
    Clear,
    Sign,
    Overfly,
}

pub fn held(report: &[u8; 14]) -> Vec<Key> {
    if report[0] != 1 {
        return Vec::new();
    }
    let mut keys = Vec::new();
    for (byte, mask, key) in [
        (4, 1, Key::Menu),
        (2, 0x10, Key::Dir),
        (2, 0x20, Key::Prog),
        (2, 0x40, Key::Perf),
        (2, 0x80, Key::Init),
        (3, 1, Key::Data),
        (3, 8, Key::FlightPlan),
        (3, 0x10, Key::RadNav),
        (3, 0x20, Key::Fuel),
        (3, 0x40, Key::Secondary),
        (3, 0x80, Key::Atc),
        (4, 4, Key::Airport),
        (4, 0x10, Key::Left),
        (4, 0x40, Key::Right),
        (4, 0x20, Key::Up),
        (4, 0x80, Key::Down),
        (10, 2, Key::Clear),
        (10, 1, Key::Overfly),
        (6, 8, Key::Sign),
        (9, 64, Key::Character('/')),
        (9, 128, Key::Character(' ')),
    ] {
        if report[byte] & mask != 0 {
            keys.push(key);
        }
    }
    for row in 0..6 {
        if report[1] & (1 << row) != 0 {
            keys.push(Key::Lsk { right: false, row });
        }
        let (byte, mask) = if row < 2 {
            (1, 0x40 << row)
        } else {
            (2, 1 << (row - 2))
        };
        if report[byte] & mask != 0 {
            keys.push(Key::Lsk { right: true, row });
        }
    }
    for index in 0..26 {
        let bit = index + 4;
        if report[6 + bit / 8] & (1 << (bit % 8)) != 0 {
            keys.push(Key::Character((b'A' + index as u8) as char));
        }
    }
    for (byte, mask, ch) in [
        (5, 1, '1'),
        (5, 2, '2'),
        (5, 4, '3'),
        (5, 8, '4'),
        (5, 16, '5'),
        (5, 32, '6'),
        (5, 64, '7'),
        (5, 128, '8'),
        (6, 1, '9'),
        (6, 2, '.'),
        (6, 4, '0'),
    ] {
        if report[byte] & mask != 0 {
            keys.push(Key::Character(ch));
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_functions_text_and_lsk_are_independent() {
        let mut report = [0; 14];
        report[0] = 1;
        report[4] = 1;
        report[6] = 0x10;
        report[2] = 0x80;
        report[1] = 0x40;
        let keys = held(&report);
        assert!(keys.contains(&Key::Menu));
        assert!(keys.contains(&Key::Init));
        assert!(keys.contains(&Key::Character('A')));
        assert!(keys.contains(&Key::Lsk {
            right: true,
            row: 0
        }));
        report[0] = 0xf0;
        assert!(held(&report).is_empty());
    }
}
