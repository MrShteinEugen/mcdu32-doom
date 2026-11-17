// SPDX-License-Identifier: GPL-3.0-only
//! Command dispatch; game loops and export code live in separate modules.
mod args;
mod atlas;
mod serve;

pub fn report_background_error(error: &anyhow::Error) {
    crate::service::log(
        std::path::Path::new("output/tray"),
        &format!("Application stopped: {error:#}"),
    );
    crate::desktop::tray::report_error(error);
}
mod play;
mod play_metrics;

use crate::{desktop::emulator, device::transport, display::Profile};
use anyhow::Result;
use args::{Command, CommonArgs, EmulateArgs, TimingArgs};

fn run(command: Option<Command>) -> Result<()> {
    let command = command.unwrap_or_else(|| Command::Serve(args::ServeArgs::default()));
    match command {
        Command::Serve(options) => serve::run(options),
        Command::Probe(options) => crate::diagnostics::probe::run(options),
        Command::Devices => transport::list(),
        Command::Atlas { out } => atlas::export(&out),
        Command::Play(options) => play::run(options),
        Command::Emulate(options) => emulate(options, false),
        Command::Compare(common) => compare(common),
    }
}

fn compare(common: CommonArgs) -> Result<()> {
    let geometry = Profile::Full.geometry();
    emulate(
        EmulateArgs {
            common,
            timing: TimingArgs::default(),
            columns: geometry.columns,
            rows: geometry.rows,
            benchmark: false,
        },
        true,
    )
}

fn emulate(options: EmulateArgs, comparison: bool) -> Result<()> {
    emulator::run(emulator::Options {
        style: options.common.style,
        comparison,
        wad: options.common.wad,
        cols: usize::from(options.columns),
        rows: usize::from(options.rows),
        fps: options.timing.fps,
        packet_us: options.timing.packet_us,
        headless: options.common.headless,
        seconds: options.common.seconds,
        capture: options.common.capture,
        benchmark: options.benchmark,
    })
}

use clap::Parser;

pub fn entry() {
    let command = args::Cli::parse().command;
    let background = command.is_none()
        || matches!(&command,Some(args::Command::Serve(options)) if !options.no_tray);
    if let Err(error) = run(command) {
        if background {
            report_background_error(&error);
        } else {
            eprintln!("Error: {error:#}");
        }
        std::process::exit(1);
    }
}
