// SPDX-License-Identifier: GPL-3.0-only
//! Device-free runner while the rest of the MVP application is being migrated.
use clap::Parser;
use mcdu_doom::desktop::emulator;

#[derive(Parser)]
#[command(about = "Run the MCDU emulator without a USB device")]
struct Command {
    #[command(flatten)]
    options: emulator::Options,
}

fn main() -> anyhow::Result<()> {
    emulator::run(Command::parse().options)
}
