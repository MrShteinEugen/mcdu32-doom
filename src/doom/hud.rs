// SPDX-License-Identifier: GPL-3.0-only
//! HUD state copied across the C ABI, independent of display layout.
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
