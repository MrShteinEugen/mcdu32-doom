// SPDX-License-Identifier: GPL-3.0-only
//! One worker owns the device and completes each frame before switching fonts.
use super::{
    Configuration, Source, UsbStats,
    hid::select_device,
    io::{poll, send},
};
use crate::{config::MIN_SETUP_PACKET_US, device::protocol::encode_cells, input::Inputs};
use anyhow::{Context, Result, bail};
use hidapi::HidApi;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) fn spawn_worker(
    configurations: Vec<Configuration>,
    requested_path: Option<String>,
    frames: Source,
    input: Arc<Inputs>,
    stop: Arc<AtomicBool>,
    stats: Arc<UsbStats>,
    pacing: Duration,
) -> Result<JoinHandle<Result<()>>> {
    let api = HidApi::new()?;
    let info = select_device(&api, requested_path.as_deref())?;
    let device = api.open_path(info.path())?;
    device.set_blocking_mode(false)?;
    thread::Builder::new()
        .name("mcdu-usb".into())
        .spawn(move || {
            let run = || -> Result<()> {
                let mut next_packet = Instant::now();
                if matches!(&frames, Source::Dynamic(_)) {
                    send(
                        &device,
                        &crate::device::protocol::annunciator_brightness(),
                        &input,
                        &stats,
                        pacing,
                        &mut next_packet,
                    )?;
                }
                let mut active = None;
                let mut previous = None;
                let mut indicator = None;
                while !stop.load(Ordering::Relaxed) {
                    poll(&device, &input)?;
                    let Some(frame) = frames.wait() else {
                        continue;
                    };
                    if previous.as_ref() == Some(&frame) {
                        continue;
                    }
                    let config = configurations
                        .get(frame.profile.index())
                        .context("display font family not configured")?;
                    if frame.cells.len() != config.geometry.cells() {
                        bail!(
                            "frame has {} cells, expected {}",
                            frame.cells.len(),
                            config.geometry.cells()
                        );
                    }
                    // Turn FM off before a long font transition away from MENU.
                    if !frame.mcdu_fm && indicator != Some(false) {
                        send(
                            &device,
                            &crate::device::protocol::mcdu_fm_led(false),
                            &input,
                            &stats,
                            pacing,
                            &mut next_packet,
                        )?;
                        stats.mcdu_fm.store(false, Ordering::Relaxed);
                        indicator = Some(false);
                    }
                    if active != Some(frame.profile) {
                        // Clear the physical LCD, not just the outgoing text rectangle.
                        // Also required at startup: a previous client may have drawn
                        // a wider grid whose margins our 24x14 frame cannot overwrite.
                        for packet in crate::device::protocol::black_background_packets() {
                            send(&device, &packet, &input, &stats, pacing, &mut next_packet)?;
                        }
                        let setup_pacing = pacing.max(Duration::from_micros(MIN_SETUP_PACKET_US));
                        for (index, packet) in config.setup.iter().enumerate() {
                            if stop.load(Ordering::Relaxed) {
                                return Ok(());
                            }
                            send(
                                &device,
                                packet,
                                &input,
                                &stats,
                                setup_pacing,
                                &mut next_packet,
                            )
                            .with_context(|| {
                                format!("{:?} display setup packet {index}", frame.profile)
                            })?;
                        }
                        active = Some(frame.profile);
                        stats.font_uploads.fetch_add(1, Ordering::Relaxed);
                        stats
                            .display_profile
                            .store(frame.profile.index() as u64 + 1, Ordering::Relaxed);
                    }
                    let start = Instant::now();
                    for (index, packet) in encode_cells(&config.atlas, &frame.cells)?
                        .iter()
                        .enumerate()
                    {
                        send(&device, packet, &input, &stats, pacing, &mut next_packet)
                            .with_context(|| {
                                format!("screen packet {index}, {} cells", frame.cells.len())
                            })?;
                    }
                    // MENU's indicator comes on once its complete page is visible.
                    if frame.mcdu_fm && indicator != Some(true) {
                        send(
                            &device,
                            &crate::device::protocol::mcdu_fm_led(true),
                            &input,
                            &stats,
                            pacing,
                            &mut next_packet,
                        )?;
                        stats.mcdu_fm.store(true, Ordering::Relaxed);
                        indicator = Some(true);
                    }
                    stats
                        .last_frame_us
                        .store(start.elapsed().as_micros() as u64, Ordering::Relaxed);
                    stats.frames.fetch_add(1, Ordering::Relaxed);
                    stats
                        .last_screen
                        .lock()
                        .expect("USB frame snapshot poisoned")
                        .clone_from(&frame.cells);
                    previous = Some(frame);
                }
                Ok(())
            };
            let result = run();
            // Release the annunciator on normal exit and after a session error.
            // A disconnected device may reject this best-effort cleanup.
            let mut next = Instant::now() + pacing;
            let cleanup = send(
                &device,
                &crate::device::protocol::mcdu_fm_led(false),
                &input,
                &stats,
                pacing,
                &mut next,
            );
            if cleanup.is_ok() {
                stats.mcdu_fm.store(false, Ordering::Relaxed);
            }
            let result = result.and(cleanup);
            input.release_panel();
            if result.is_err() {
                stop.store(true, Ordering::Relaxed);
            }
            result
        })
        .context("start USB worker")
}
