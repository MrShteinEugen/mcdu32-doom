// SPDX-License-Identifier: GPL-3.0-only
use crate::display::atlas::{BLANK, PORTRAIT, text_id};
use crate::display::{COLS, Cell, Screen};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudState {
    pub active: i32,
    pub health: i32,
    pub armor: i32,
    pub ammo: i32,
    pub weapon: i32,
    pub owned: i32,
    pub keys: i32,
    pub skulls: i32,
    pub damage: i32,
    pub ammo_counts: [i32; 4],
    pub ammo_max: [i32; 4],
}

#[derive(Default)]
pub struct Hud {
    previous: Option<HudState>,
    cached: Vec<Cell>,
    pub rebuilds: u64,
    cached_cols: usize,
}

pub fn panel_rows(cols: usize) -> usize {
    if cols >= 40 { 5 } else { 3 }
}

impl Hud {
    pub fn apply(&mut self, state: HudState, screen: &mut Screen) {
        self.apply_grid(state, screen, COLS, crate::display::ROWS);
    }

    /// DOOM's group order on compact grids, legacy three-row layout at 24 columns.
    pub fn apply_grid(&mut self, state: HudState, screen: &mut [Cell], cols: usize, rows: usize) {
        assert!(cols >= COLS && rows >= panel_rows(cols));
        assert_eq!(screen.len(), cols * rows);
        // Only the visible portrait state matters, not every damage countdown tick.
        let visible = HudState {
            damage: i32::from(state.damage > 0),
            ..state
        };
        if self.previous != Some(visible) || self.cached_cols != cols {
            self.previous = Some(visible);
            self.cached_cols = cols;
            self.rebuilds += 1;
            self.cached = if cols >= 40 {
                classic_panel(state, cols)
            } else {
                legacy_panel(state)
            };
        }
        let panel = &mut screen[(rows - panel_rows(cols)) * cols..];
        panel.fill(Cell {
            glyph: BLANK,
            color: 0,
        });
        if cols >= 40 {
            panel.copy_from_slice(&self.cached);
        } else {
            let offset = (cols - COLS) / 2;
            for row in 0..3 {
                panel[row * cols + offset..row * cols + offset + COLS]
                    .copy_from_slice(&self.cached[row * COLS..(row + 1) * COLS]);
            }
        }
    }
}

fn legacy_panel(state: HudState) -> Vec<Cell> {
    let mut panel = [Cell {
        glyph: BLANK,
        color: 0,
    }; COLS * 3];
    label(&mut panel, 0, 0, &[18, 19], 2); // HP
    number(
        &mut panel,
        1,
        0,
        state.health,
        3,
        if state.health <= 25 { 6 } else { 2 },
    );
    put(&mut panel, 1, 3, 25, 2);
    label(&mut panel, 0, 5, &[20, 21], 2); // AR
    number(&mut panel, 1, 5, state.armor, 3, 4);
    put(&mut panel, 1, 8, 25, 4);
    label(&mut panel, 0, 10, &[20, 22], 2); // AM
    number(&mut panel, 1, 10, state.ammo, 3, 7);
    label(&mut panel, 0, 14, &[23], 2); // W
    number(&mut panel, 1, 14, weapon_slot(state.weapon), 1, 2);
    label(&mut panel, 0, 16, &[24], 2); // K
    for key in 0..3 {
        if state.keys & (1 << key) != 0 {
            put(
                &mut panel,
                1,
                16 + key,
                if state.skulls & (1 << key) != 0 {
                    29
                } else {
                    28
                },
                [3, 7, 6][key],
            );
        }
    }
    // Highlight owned weapon slots using the same numeric glyphs.
    for slot in 1..=7 {
        let owned = match slot {
            1 => state.owned & ((1 << 0) | (1 << 7)),
            3 => state.owned & ((1 << 2) | (1 << 8)),
            _ => state.owned & (1 << (slot - 1)),
        };
        if owned != 0 {
            put(
                &mut panel,
                2,
                slot as usize - 1,
                (slot - 1 + 1) as u8,
                if weapon_slot(state.weapon) == slot {
                    7
                } else {
                    9
                },
            );
        }
    }
    for part in 0..4 {
        put(
            &mut panel,
            part / 2,
            21 + part % 2,
            (10 + usize::from(state.damage > 0) * 4 + part) as u8,
            2,
        );
    }
    panel.to_vec()
}

fn classic_panel(state: HudState, cols: usize) -> Vec<Cell> {
    let mut panel = vec![
        Cell {
            glyph: BLANK,
            color: 0
        };
        cols * 5
    ];
    let mut text = |row: usize, col: usize, value: &str, color: u8| {
        for (offset, character) in value.chars().take(cols - col).enumerate() {
            panel[row * cols + col + offset] = Cell {
                glyph: text_id(character),
                color,
            };
        }
    };
    let health = cols * 44 / 320;
    let arms = cols * 104 / 320;
    let face = cols / 2 - 2;
    let armor = cols * 184 / 320;
    let keys = cols * 239 / 320;
    let ammunition = cols - 8;
    text(0, 0, &"-".repeat(cols), 9);
    text(
        2,
        0,
        &if state.ammo < 0 {
            "---".into()
        } else {
            format!("{:3}", state.ammo.min(999))
        },
        6,
    );
    text(4, 0, "AMMO", 2);
    text(
        2,
        health,
        &format!("{:3}%", state.health.clamp(0, 999)),
        if state.health <= 25 { 6 } else { 2 },
    );
    text(4, health, "HEALTH", 2);
    text(4, arms, "ARMS", 2);
    for slot in 2..=7 {
        let owned = match slot {
            3 => state.owned & ((1 << 2) | (1 << 8)),
            _ => state.owned & (1 << (slot - 1)),
        };
        text(
            1 + (slot - 2) as usize / 3,
            arms + (slot - 2) as usize % 3,
            &slot.to_string(),
            if weapon_slot(state.weapon) == slot {
                7
            } else if owned != 0 {
                2
            } else {
                9
            },
        );
    }
    text(2, armor, &format!("{:3}%", state.armor.clamp(0, 999)), 4);
    text(4, armor, "ARMOR", 2);
    // Same order as DOOM's status bar, independent of the selected weapon.
    for (row, index, initial) in [(1, 0, 'B'), (2, 1, 'S'), (3, 3, 'R'), (4, 2, 'C')] {
        text(
            row,
            ammunition,
            &format!(
                "{}{:3}/{:3}",
                initial,
                state.ammo_counts[index].clamp(0, 999),
                state.ammo_max[index].clamp(0, 999)
            ),
            7,
        );
    }
    for key in 0..3 {
        if state.keys & (1 << key) != 0 {
            panel[(key + 1) * cols + keys] = Cell {
                glyph: if state.skulls & (1 << key) != 0 {
                    29
                } else {
                    28
                },
                color: [3, 7, 6][key],
            };
        }
    }
    for part in 0..16 {
        panel[(1 + part / 4) * cols + face + part % 4] = Cell {
            glyph: (PORTRAIT + part) as u8,
            color: if state.damage > 0 { 6 } else { 2 },
        };
    }
    panel
}

pub fn weapon_slot(weapon: i32) -> i32 {
    match weapon {
        7 => 1,
        8 => 3,
        _ => weapon + 1,
    }
}
fn put(panel: &mut [Cell], row: usize, col: usize, glyph: u8, color: u8) {
    panel[row * COLS + col] = Cell { glyph, color };
}
fn label(panel: &mut [Cell], row: usize, col: usize, letters: &[u8], color: u8) {
    for (i, &glyph) in letters.iter().enumerate() {
        put(panel, row, col + i, glyph, color);
    }
}
fn number(panel: &mut [Cell], row: usize, col: usize, value: i32, width: usize, color: u8) {
    if value < 0 {
        for i in 0..width {
            put(panel, row, col + i, 27, color);
        }
        return;
    }
    let mut value = value.min(10_i32.pow(width as u32) - 1);
    for i in (0..width).rev() {
        put(panel, row, col + i, (value % 10) as u8, color);
        value /= 10;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_countdown_does_not_rebuild_panel() {
        let mut hud = Hud::default();
        let mut screen = [Cell::default(); 336];
        hud.apply(
            HudState {
                health: 100,
                damage: 20,
                ..HudState::default()
            },
            &mut screen,
        );
        hud.apply(
            HudState {
                health: 100,
                damage: 19,
                ..HudState::default()
            },
            &mut screen,
        );
        assert_eq!(hud.rebuilds, 1);
        hud.apply(
            HudState {
                health: 99,
                damage: 19,
                ..HudState::default()
            },
            &mut screen,
        );
        assert_eq!(hud.rebuilds, 2);
        assert!(screen[264..].iter().all(|cell| cell.glyph < 32));
    }

    #[test]
    fn full_width_hud_keeps_scene_intact_and_uses_both_edges() {
        let mut hud = Hud::default();
        let scene = Cell {
            glyph: 200,
            color: 3,
        };
        let mut screen = vec![scene; 53 * 31];
        hud.apply_grid(
            HudState {
                health: 100,
                armor: 50,
                ammo: 35,
                owned: 7,
                ..HudState::default()
            },
            &mut screen,
            53,
            31,
        );
        assert!(screen[..53 * 26].iter().all(|&cell| cell == scene));
        assert_eq!(screen[53 * 30].glyph, text_id('A')); // AMMO at left
        assert_eq!(screen[53 * 27 + 24].glyph, PORTRAIT as u8);
        assert_eq!(screen[53 * 30 + 27].glyph, (PORTRAIT + 15) as u8);
        assert_eq!(screen[53 * 27 + 45].glyph, text_id('B'));
        assert_eq!(screen[53 * 30 + 45].glyph, text_id('C'));
    }
}
