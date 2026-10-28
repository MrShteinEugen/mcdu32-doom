// SPDX-License-Identifier: GPL-3.0-only
//! Code-drawn MCDU front panel. Its mouse hit boxes and legends share one table.
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window};

pub const PANEL_W: usize = 760;
pub const PANEL_H: usize = 1000;
pub const LCD_X: usize = 88;
pub const LCD_Y: usize = 48;
pub const LCD_W: usize = crate::display::LCD_WIDTH;
pub const LCD_H: usize = crate::display::LCD_HEIGHT;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && y >= self.y as f32
            && x < (self.x + self.w) as f32
            && y < (self.y + self.h) as f32
    }
}

pub struct Button {
    pub label: String,
    pub rect: Rect,
    pub byte: usize,
    pub mask: u8,
}

pub struct Panel {
    pub buttons: Vec<Button>,
    latched: Vec<bool>,
    active: Vec<bool>,
    left_previous: bool,
    right_previous: bool,
    capture: Option<usize>,
    brightness: f32,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub fn new() -> Self {
        let mut buttons = Vec::new();
        for row in 0..6 {
            let (byte, mask) = if row < 2 {
                (1, 0x40 << row)
            } else {
                (2, 1 << (row - 2))
            };
            buttons.push(Button {
                label: format!("R{}", row + 1),
                rect: Rect {
                    x: 668,
                    y: 82 + row * 62,
                    w: 64,
                    h: 30,
                },
                byte,
                mask,
            });
            buttons.push(Button {
                label: format!("L{}", row + 1),
                rect: Rect {
                    x: 28,
                    y: 82 + row * 62,
                    w: 64,
                    h: 30,
                },
                byte: 1,
                mask: 1 << row,
            });
        }
        let functions = [
            ("DIR", 2, 0x10),
            ("PROG", 2, 0x20),
            ("PERF", 2, 0x40),
            ("INIT", 2, 0x80),
            ("DATA", 3, 1),
            ("BRT", 3, 4),
            ("F-PLN", 3, 8),
            ("RAD\nNAV", 3, 0x10),
            ("FUEL\nPRED", 3, 0x20),
            ("SEC\nF-PLN", 3, 0x40),
            ("ATC\nCOMM", 3, 0x80),
            ("DIM", 4, 2),
            ("MCDU\nMENU", 4, 1),
            ("AIR\nPORT", 4, 4),
            ("", 3, 2),
            ("", 4, 8),
        ];
        for (index, (label, byte, mask)) in functions.into_iter().enumerate() {
            buttons.push(Button {
                label: label.into(),
                rect: Rect {
                    x: 40 + index % 6 * 114,
                    y: 500 + index / 6 * 48,
                    w: 104,
                    h: 40,
                },
                byte,
                mask,
            });
        }
        for (index, (label, mask)) in [("<", 0x10), ("^", 0x20), ("v", 0x80), (">", 0x40)]
            .into_iter()
            .enumerate()
        {
            buttons.push(Button {
                label: label.into(),
                rect: Rect {
                    x: 40 + index % 2 * 76,
                    y: 654 + index / 2 * 46,
                    w: 64,
                    h: 36,
                },
                byte: 4,
                mask,
            });
        }
        for index in 0..30 {
            let (label, byte, mask) = if index < 26 {
                let bit = index + 4;
                ((b'A' + index as u8) as char, 6 + bit / 8, 1 << (bit % 8))
            } else {
                match index {
                    26 => ('/', 9, 64),
                    27 => (' ', 9, 128),
                    28 => ('!', 10, 1),
                    _ => ('#', 10, 2),
                }
            };
            let label = match label {
                ' ' => "SP".into(),
                '!' => "OVFY".into(),
                '#' => "CLR".into(),
                c => c.to_string(),
            };
            buttons.push(Button {
                label,
                rect: Rect {
                    x: 300 + index % 5 * 84,
                    y: 670 + index / 5 * 44,
                    w: 72,
                    h: 36,
                },
                byte,
                mask,
            });
        }
        for (index, label) in ["1", "2", "3", "4", "5", "6", "7", "8", "9", ".", "0", "+/-"]
            .into_iter()
            .enumerate()
        {
            let (byte, mask) = if index < 8 {
                (5, 1 << index)
            } else {
                (6, 1 << (index - 8))
            };
            buttons.push(Button {
                label: label.into(),
                rect: Rect {
                    x: 40 + index % 3 * 76,
                    y: 758 + index / 3 * 44,
                    w: 64,
                    h: 36,
                },
                byte,
                mask,
            });
        }
        let count = buttons.len();
        Self {
            buttons,
            latched: vec![false; count],
            active: vec![false; count],
            left_previous: false,
            right_previous: false,
            capture: None,
            brightness: 1.0,
        }
    }

    pub fn hit(&self, point: Option<(f32, f32)>) -> Option<usize> {
        point.and_then(|(x, y)| {
            self.buttons
                .iter()
                .position(|button| button.rect.contains(x, y))
        })
    }

    /// Pointer capture keeps a held key pressed until release, even outside its box.
    pub fn pointer(
        &mut self,
        point: Option<(f32, f32)>,
        left: bool,
        right: bool,
        focused: bool,
        reset: bool,
    ) -> [u8; 14] {
        if !focused || reset {
            self.latched.fill(false);
            self.active.fill(false);
            self.capture = None;
        } else {
            let hit = self.hit(point);
            if right && !self.right_previous {
                if let Some(index) = hit {
                    self.latched[index] = !self.latched[index];
                }
            }
            if left && !self.left_previous {
                self.capture = hit;
                if let Some(index) = hit {
                    match self.buttons[index].label.as_str() {
                        "BRT" => self.brightness = (self.brightness + 0.1).min(1.0),
                        "DIM" => self.brightness = (self.brightness - 0.1).max(0.2),
                        _ => {}
                    }
                }
            }
            if !left {
                self.capture = None;
            }
            self.active.clone_from(&self.latched);
            if let Some(index) = self.capture {
                self.active[index] = true;
            }
        }
        self.left_previous = left;
        self.right_previous = right;
        let mut report = [0; 14];
        report[0] = 1;
        for (button, active) in self.buttons.iter().zip(&self.active) {
            if *active {
                report[button.byte] |= button.mask;
            }
        }
        report
    }

    pub fn input(&mut self, window: &Window) -> [u8; 14] {
        let mut report = self.pointer(
            window.get_mouse_pos(MouseMode::Discard),
            window.get_mouse_down(MouseButton::Left),
            window.get_mouse_down(MouseButton::Right),
            window.is_active(),
            window.is_key_pressed(Key::F12, KeyRepeat::No),
        );
        for (byte, key) in report.iter_mut().zip(Self::keyboard_input(window)) {
            *byte |= key;
        }
        self.display_report(&report);
        report
    }

    /// Translate PC controls to virtual HID while keeping physical A-E separate.
    pub fn keyboard_input(window: &Window) -> [u8; 14] {
        crate::desktop::keyboard::keyboard_report(window)
    }

    pub fn display_report(&mut self, report: &[u8; 14]) {
        for (index, button) in self.buttons.iter().enumerate() {
            self.active[index] = report[button.byte] & button.mask != 0;
        }
    }

    pub fn draw(&self, lcd: &[u32], status: &str, output: &mut [u32]) {
        assert_eq!(output.len(), PANEL_W * PANEL_H);
        assert_eq!(lcd.len(), LCD_W * LCD_H);
        output.fill(0x18222b);
        rect(
            output,
            Rect {
                x: 10,
                y: 10,
                w: 740,
                h: 978,
            },
            0x33434f,
        );
        rect(
            output,
            Rect {
                x: 18,
                y: 18,
                w: 724,
                h: 962,
            },
            0x263640,
        );
        text(output, 254, 24, "MCDU DOOM / EMULATOR", 2, 0xc7d9df);
        rect(
            output,
            Rect {
                x: 80,
                y: 40,
                w: 600,
                h: 456,
            },
            0x0a1015,
        );
        for y in 0..LCD_H {
            for x in 0..LCD_W {
                let rgb = lcd[y * LCD_W + x];
                let scale =
                    |shift: u32| (((rgb >> shift) & 255_u32) as f32 * self.brightness) as u32;
                output[(LCD_Y + y) * PANEL_W + LCD_X + x] =
                    (scale(16) << 16) | (scale(8) << 8) | scale(0);
            }
        }
        for (index, button) in self.buttons.iter().enumerate() {
            rect(
                output,
                button.rect,
                if self.active[index] {
                    0xd99d40
                } else {
                    0x0c151c
                },
            );
            rect(
                output,
                Rect {
                    x: button.rect.x + 2,
                    y: button.rect.y + 2,
                    w: button.rect.w - 4,
                    h: button.rect.h - 4,
                },
                if self.active[index] {
                    0x936829
                } else {
                    0x344754
                },
            );
            let line_count = button.label.split('\n').count();
            let scale = if line_count == 1 { 2 } else { 1 };
            for (line, label) in button.label.split('\n').enumerate() {
                let width = label.len() * 6 * scale;
                let x = button.rect.x + (button.rect.w.saturating_sub(width)) / 2;
                let y =
                    button.rect.y + (button.rect.h - line_count * 8 * scale) / 2 + line * 8 * scale;
                text(output, x, y, label, scale, 0xe0eeee);
            }
            if self.latched[index] {
                rect(
                    output,
                    Rect {
                        x: button.rect.x + 4,
                        y: button.rect.y + 4,
                        w: 4,
                        h: 4,
                    },
                    0xffdd65,
                );
            }
        }
        text(output, 40, 480, status, 1, 0x8faab4);
        text(
            output,
            40,
            952,
            "LMB: HOLD  RMB: LATCH  F12: RELEASE ALL",
            2,
            0x95b3bf,
        );
        text(
            output,
            40,
            974,
            "ARROWS MOVE / R1 FIRE / L1 USE / L2 RUN / L3 R3 STRAFE",
            1,
            0x95b3bf,
        );
    }
}

fn rect(output: &mut [u32], r: Rect, color: u32) {
    for y in r.y..(r.y + r.h).min(PANEL_H) {
        for x in r.x..(r.x + r.w).min(PANEL_W) {
            output[y * PANEL_W + x] = color;
        }
    }
}
fn text(output: &mut [u32], x: usize, y: usize, value: &str, scale: usize, color: u32) {
    for (i, character) in value.chars().enumerate() {
        let rows = font(character);
        for (yy, row) in rows.into_iter().enumerate() {
            for xx in 0..5 {
                if row & (1 << (4 - xx)) != 0 {
                    rect(
                        output,
                        Rect {
                            x: x + (i * 6 + xx) * scale,
                            y: y + yy * scale,
                            w: scale,
                            h: scale,
                        },
                        color,
                    );
                }
            }
        }
    }
}
fn font(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 14],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 25, 21, 19, 19, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        '<' => [2, 4, 8, 16, 8, 4, 2],
        '>' => [8, 4, 2, 1, 2, 4, 8],
        '^' => [4, 10, 17, 4, 4, 4, 4],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point(panel: &Panel, label: &str) -> (f32, f32) {
        let b = panel
            .buttons
            .iter()
            .find(|b| b.label == label)
            .expect("button");
        (
            (b.rect.x + b.rect.w / 2) as f32,
            (b.rect.y + b.rect.h / 2) as f32,
        )
    }
    #[test]
    fn all_keys_have_distinct_report_bits_and_hit_boxes() {
        let panel = Panel::new();
        assert_eq!(panel.buttons.len(), 74);
        for (i, a) in panel.buttons.iter().enumerate() {
            assert_eq!(
                panel.hit(Some(((a.rect.x + 2) as f32, (a.rect.y + 2) as f32))),
                Some(i)
            );
            for b in &panel.buttons[..i] {
                assert_ne!((a.byte, a.mask), (b.byte, b.mask));
            }
        }
    }
    #[test]
    fn simultaneous_latched_movement_fire_and_focus_release() {
        let mut panel = Panel::new();
        let up = point(&panel, "^");
        let fire = point(&panel, "R1");
        panel.pointer(Some(up), false, true, true, false);
        panel.pointer(None, false, false, true, false);
        let report = panel.pointer(Some(fire), true, false, true, false);
        assert_eq!(crate::input::decode_mcdu(&report), Some(1 | (1 << 4)));
        let report = panel.pointer(None, true, false, true, false);
        assert_eq!(crate::input::decode_mcdu(&report), Some(1 | (1 << 4)));
        let report = panel.pointer(None, false, false, false, false);
        assert_eq!(crate::input::decode_mcdu(&report), Some(0));
    }
    #[test]
    fn alpha_and_clear_reach_doom() {
        let mut panel = Panel::new();
        let z = point(&panel, "Z");
        let report = panel.pointer(Some(z), true, false, true, false);
        let keys = crate::input::decode_raw(&report);
        assert_ne!(keys[b'z' as usize / 64] & (1 << (b'z' % 64)), 0);
    }
}
