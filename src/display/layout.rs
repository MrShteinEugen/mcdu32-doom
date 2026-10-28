// SPDX-License-Identifier: GPL-3.0-only
//! Shared row allocation for gameplay and its status panel.
pub fn panel_rows(cols: usize) -> usize {
    if cols >= 40 { 5 } else { 3 }
}

/// Rows available for the scene after reserving the active HUD.
pub fn scene_rows(cols: usize, rows: usize, gameplay: bool) -> usize {
    if gameplay {
        rows - panel_rows(cols)
    } else {
        rows
    }
}
