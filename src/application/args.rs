// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    config::{DEFAULT_FPS, DEFAULT_PACKET_US, DEFAULT_WAD},
    display::Profile,
    display::render::Style,
};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "DOOM raster output for WINCTRL MCDU-32")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start hidden in the Windows tray and serve the physical MCDU.
    #[command(alias = "tray")]
    Serve(ServeArgs),
    /// Export or send a static coordinate table and experimental compact font.
    Probe(crate::diagnostics::probe::Options),
    /// Show original graphics and glyph output from one game in two windows.
    Compare(CommonArgs),
    /// Simulate a complete MCDU front panel without USB access.
    Emulate(EmulateArgs),
    /// Read-only enumeration of connected MCDU HID collections.
    Devices,
    /// Export the glyph dictionary and font transaction without USB writes.
    Atlas {
        #[arg(long, default_value = "output")]
        out: PathBuf,
    },
    /// Run actual DOOM gameplay, optionally sending frames to the MCDU.
    Play(PlayArgs),
}

#[derive(Args)]
pub struct ServeArgs {
    #[arg(long, default_value = DEFAULT_WAD)]
    pub wad: PathBuf,
    #[arg(long, value_enum)]
    pub mode: Option<crate::service::OutputMode>,
    #[arg(long, value_enum, default_value = "full")]
    pub display: Profile,
    #[arg(long, default_value_t = DEFAULT_PACKET_US, value_parser=clap::value_parser!(u64).range(500..=20000))]
    pub packet_us: u64,
    /// Run the background session without a tray (diagnostics).
    #[arg(long)]
    pub no_tray: bool,
    /// Do not enumerate or write USB (offline verification).
    #[arg(long)]
    pub no_usb: bool,
    #[arg(long)]
    pub autostart: bool,
    #[arg(long, default_value_t = 0)]
    pub seconds: u64,
    #[arg(long)]
    pub capture: Option<PathBuf>,
}
impl Default for ServeArgs {
    fn default() -> Self {
        Self {
            wad: DEFAULT_WAD.into(),
            mode: None,
            display: Profile::Full,
            packet_us: DEFAULT_PACKET_US,
            no_tray: false,
            no_usb: false,
            autostart: false,
            seconds: 0,
            capture: None,
        }
    }
}

#[derive(Args)]
pub struct CommonArgs {
    /// Mostly neutral scenery with colored enemies and HUD.
    #[arg(long, value_enum, default_value = "contrast")]
    pub style: Style,
    #[arg(long, default_value = DEFAULT_WAD)]
    pub wad: PathBuf,
    /// Run without a preview window; physical MCDU controls remain active.
    #[arg(long)]
    pub headless: bool,
    /// Stop automatically after this many seconds (0 means no limit).
    #[arg(long, default_value_t = 0)]
    pub seconds: u64,
    /// Save reconstructed and original frames, comparisons and metrics.
    #[arg(long)]
    pub capture: Option<PathBuf>,
}

#[derive(Args)]
pub struct TimingArgs {
    #[arg(long, default_value_t = DEFAULT_FPS, value_parser = clap::value_parser!(u32).range(1..=60))]
    pub fps: u32,
    /// Minimum screen-packet delay; font/grid setup always uses at least 10 ms.
    #[arg(long, default_value_t = DEFAULT_PACKET_US, value_parser = clap::value_parser!(u64).range(500..=20000))]
    pub packet_us: u64,
}
impl Default for TimingArgs {
    fn default() -> Self {
        Self {
            fps: DEFAULT_FPS,
            packet_us: DEFAULT_PACKET_US,
        }
    }
}

#[derive(Args)]
pub struct EmulateArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(flatten)]
    pub timing: TimingArgs,
    #[arg(long, default_value_t = 53, value_parser = clap::value_parser!(u16).range(24..=96))]
    pub columns: u16,
    #[arg(long, default_value_t = 31, value_parser = clap::value_parser!(u16).range(4..=64))]
    pub rows: u16,
    /// Rotate and fire through simulated MCDU reports for a timing/input test.
    #[arg(long, requires = "headless")]
    pub benchmark: bool,
}

#[derive(Args)]
pub struct PlayArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(flatten)]
    pub timing: TimingArgs,
    /// Upload the custom font and send game frames to the selected MCDU.
    #[arg(long)]
    pub mcdu: bool,
    /// Confirmed full (53x31), dense (48x28), or standard (24x14) profile.
    #[arg(long, value_enum, default_value = "full", conflicts_with = "dense")]
    pub display: Profile,
    /// Compatibility shortcut for --display dense.
    #[arg(long, conflicts_with = "original")]
    pub dense: bool,
    /// Upload the original 23x29 glyphs even with the dense logical grid.
    #[arg(long, requires_all = ["mcdu", "dense"])]
    pub legacy_font: bool,
    /// Keep the grid configured externally, including during font upload.
    #[arg(long, requires = "mcdu")]
    pub keep_grid: bool,
    /// Display the original RGB game framebuffer without glyph conversion.
    #[arg(long, conflicts_with = "mcdu")]
    pub original: bool,
    /// Exact HID path from devices; required when selection is ambiguous.
    #[arg(long, requires = "mcdu")]
    pub device_path: Option<String>,
    /// Start in the original title/menu instead of the first map.
    #[arg(long)]
    pub menu: bool,
    /// Rotate continuously to measure output on a changing scene.
    #[arg(long, requires = "headless")]
    pub benchmark: bool,
}
impl PlayArgs {
    pub fn profile(&self) -> Profile {
        if self.dense {
            Profile::Dense
        } else {
            self.display
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn commands_have_consistent_flags_and_preserve_hardware_defaults() {
        Cli::command().debug_assert();
        let Cli {
            command: Some(Command::Play(options)),
        } = Cli::parse_from(["mcdu-doom", "play"])
        else {
            panic!("play");
        };
        assert_eq!(options.timing.packet_us, DEFAULT_PACKET_US);
        assert_eq!(options.profile(), Profile::Full);
        assert_eq!(options.common.style, Style::Contrast);
        assert!(!options.mcdu);
    }
    #[test]
    fn dense_compatibility_and_original_conflicts_remain_enforced() {
        let Cli {
            command: Some(Command::Play(options)),
        } = Cli::parse_from(["mcdu-doom", "play", "--dense"])
        else {
            panic!("play");
        };
        assert_eq!(options.profile(), Profile::Dense);
        assert!(Cli::try_parse_from(["mcdu-doom", "play", "--original", "--mcdu"]).is_err());
        assert!(Cli::try_parse_from(["mcdu-doom", "play", "--mcdu", "--legacy-font"]).is_err());
    }
}
