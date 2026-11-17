// SPDX-License-Identifier: GPL-3.0-only
//! USB display frames and font configuration; hardware I/O lives in submodules.
use crate::{
    device::protocol::display_setup,
    display::{Cell, Geometry, atlas::Atlas},
    input::Inputs,
    runtime::latest::Latest,
};
use anyhow::Result;
use hidapi::DeviceInfo;
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
mod hid;
mod io;
mod session;
pub use hid::{Transmission, list, write_once};
use session::spawn_worker;

/// The service may reopen the device after discovery or HID I/O fails.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub(crate) struct ConnectionError(#[from] anyhow::Error);

pub const VENDOR: u16 = 0x4098;
pub const PRODUCTS: [u16; 3] = [0xbb36, 0xbb3e, 0xbb3a];

pub fn is_mcdu(info: &DeviceInfo) -> bool {
    info.vendor_id() == VENDOR && PRODUCTS.contains(&info.product_id())
}

#[derive(Default)]
pub struct UsbStats {
    pub frames: AtomicU64,
    pub packets: AtomicU64,
    pub last_frame_us: AtomicU64,
    pub last_screen: Mutex<Vec<Cell>>,
    pub font_uploads: AtomicU64,
    pub display_profile: AtomicU64,
    pub mcdu_fm: AtomicBool,
}

#[derive(Serialize)]
pub struct UsbMetrics {
    pub usb_frames: u64,
    pub usb_reports: u64,
    pub usb_last_frame_ms: f64,
    pub font_uploads: u64,
    pub display_profile: &'static str,
    pub mcdu_fm: bool,
}
impl UsbStats {
    pub fn snapshot(&self) -> UsbMetrics {
        UsbMetrics {
            usb_frames: self.frames.load(Ordering::Relaxed),
            usb_reports: self.packets.load(Ordering::Relaxed),
            usb_last_frame_ms: self.last_frame_us.load(Ordering::Relaxed) as f64 / 1000.0,
            font_uploads: self.font_uploads.load(Ordering::Relaxed),
            display_profile: match self.display_profile.load(Ordering::Relaxed) {
                1 => "game",
                2 => "aircraft",
                _ => "pending",
            },
            mcdu_fm: self.mcdu_fm.load(Ordering::Relaxed),
        }
    }
}

pub struct DisplayOptions {
    pub pacing: Duration,
    pub geometry: Geometry,
    pub legacy_font: bool,
    pub keep_grid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontProfile {
    Game,
    Aircraft,
}
impl FontProfile {
    fn index(self) -> usize {
        match self {
            Self::Game => 0,
            Self::Aircraft => 1,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayFrame {
    pub profile: FontProfile,
    pub mcdu_fm: bool,
    pub cells: Vec<Cell>,
}

struct Configuration {
    atlas: Atlas,
    geometry: Geometry,
    setup: Vec<Vec<u8>>,
}
enum Source {
    Static(Arc<Latest<Vec<Cell>>>),
    Dynamic(Arc<Latest<DisplayFrame>>),
}
impl Source {
    fn wait(&self) -> Option<DisplayFrame> {
        match self {
            Self::Static(source) => {
                source
                    .wait(Duration::from_millis(2))
                    .map(|cells| DisplayFrame {
                        profile: FontProfile::Game,
                        mcdu_fm: false,
                        cells,
                    })
            }
            Self::Dynamic(source) => source.wait(Duration::from_millis(2)),
        }
    }
}

/// The application supplies its fonts; transport does not construct MCDU pages.
pub struct DynamicFonts {
    pub game: Atlas,
    pub aircraft: Atlas,
    pub aircraft_geometry: Geometry,
}

impl Configuration {
    fn managed(atlas: Atlas, geometry: Geometry) -> Result<Self> {
        use crate::device::{
            font::{self, Packing},
            protocol,
        };
        let mut setup = font::rebuild(
            &atlas,
            usize::from(geometry.glyph_width),
            usize::from(geometry.glyph_height),
            Packing::Tight,
            200,
        )?
        .packets;
        protocol::preserve_grid(&mut setup);
        setup.extend(protocol::black_background_packets());
        setup.extend(protocol::layout_packets(
            &atlas,
            geometry,
            protocol::Layout::Captured,
        )?);
        Ok(Self {
            atlas,
            geometry,
            setup,
        })
    }
}

pub fn spawn(
    atlas: Atlas,
    requested_path: Option<String>,
    frames: Arc<Latest<Vec<Cell>>>,
    input: Arc<Inputs>,
    stop: Arc<AtomicBool>,
    stats: Arc<UsbStats>,
    options: DisplayOptions,
) -> Result<JoinHandle<Result<()>>> {
    let config = Configuration {
        setup: display_setup(
            &atlas,
            options.geometry,
            options.legacy_font,
            options.keep_grid,
        )?,
        atlas,
        geometry: options.geometry,
    };
    spawn_worker(
        vec![config],
        requested_path,
        Source::Static(frames),
        input,
        stop,
        stats,
        options.pacing,
    )
}

/// Each frame carries its font family, so stale game cells cannot be interpreted
/// using text fonts. Complete the current transaction before changing either bank.
pub fn spawn_dynamic(
    fonts: DynamicFonts,
    frames: Arc<Latest<DisplayFrame>>,
    input: Arc<Inputs>,
    stop: Arc<AtomicBool>,
    stats: Arc<UsbStats>,
    options: DisplayOptions,
) -> Result<JoinHandle<Result<()>>> {
    anyhow::ensure!(
        !options.legacy_font && !options.keep_grid,
        "dynamic fonts require managed font banks and geometry"
    );
    let game = Configuration::managed(fonts.game, options.geometry)?;
    let text = Configuration::managed(fonts.aircraft, fonts.aircraft_geometry)?;
    spawn_worker(
        vec![game, text],
        None,
        Source::Dynamic(frames),
        input,
        stop,
        stats,
        options.pacing,
    )
}
