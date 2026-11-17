// SPDX-License-Identifier: GPL-3.0-only
#![cfg_attr(windows, windows_subsystem = "windows")]
//! Desktop entry point without a console window.
fn main() {
    mcdu_doom::application::entry();
}
