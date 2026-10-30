// SPDX-License-Identifier: GPL-3.0-only
use super::home;
use super::{
    Command, Options, OutputMode, Preferences, Status, controller::Controller, pending_commands,
};
use crate::{
    device::transport::{self, DisplayFrame, FontProfile, UsbStats},
    display::{
        ascii::AsciiRenderer,
        atlas::Atlas,
        render::{Renderer, Style},
    },
    doom::engine::{self, Frame},
    input::Inputs,
    runtime::latest::Latest,
};
use anyhow::Result;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    thread,
    time::{Duration, Instant},
};

pub(super) struct Session<'a> {
    pub(super) options: &'a Options,
    pub(super) commands: &'a Receiver<Command>,
    pub(super) status: &'a Mutex<Status>,
    pub(super) stop: &'a Arc<AtomicBool>,
    pub(super) input: &'a Arc<Inputs>,
    pub(super) shared: &'a Mutex<Frame>,
    pub(super) atlas: &'a Atlas,
    pub(super) started: Instant,
}

impl Session<'_> {
    pub(super) fn run(self, controller: &mut Controller) -> Result<()> {
        let Self {
            options,
            commands,
            status,
            stop,
            input,
            shared,
            atlas,
            started,
        } = self;
        let geometry = options.profile.geometry();
        let cols = geometry.columns as usize;
        let rows = geometry.rows as usize;
        let mut renderer = Renderer::for_cells(
            atlas,
            cols,
            rows,
            geometry.glyph_width as usize,
            geometry.glyph_height as usize,
        )?;
        let mut ascii = AsciiRenderer::new(
            atlas,
            cols,
            rows,
            geometry.glyph_width as usize,
            geometry.glyph_height as usize,
        );
        let mailbox = Arc::new(Latest::<DisplayFrame>::default());
        let stats = Arc::new(UsbStats::default());
        let usb_stop = Arc::new(AtomicBool::new(false));
        let _usb_guard = engine::stop_on_drop(&usb_stop);
        let usb = if options.no_usb {
            None
        } else {
            Some(transport::spawn_dynamic(
                transport::DynamicFonts {
                    game: atlas.clone(),
                    aircraft: crate::mcdu::display::atlas(),
                    aircraft_geometry: crate::mcdu::display::geometry(),
                },
                Arc::clone(&mailbox),
                Arc::clone(input),
                Arc::clone(&usb_stop),
                Arc::clone(&stats),
                transport::DisplayOptions {
                    pacing: Duration::from_micros(options.packet_us),
                    geometry,
                    legacy_font: false,
                    keep_grid: false,
                },
            )?)
        };
        let mut source = Frame::default();
        if options.autostart {
            controller.command(Command::Start, input, stop);
        }
        let mut screen = controller.navigation.render(cols, rows);
        mailbox.publish(DisplayFrame {
            profile: controller.navigation.font_profile(),
            mcdu_fm: controller.navigation.mcdu_fm(),
            cells: screen.clone(),
        });
        *status.lock().expect("status mutex") = Status {
            application: controller.navigation.view,
            connected: true,
            playing: controller.playing,
            mode: controller.mode,
            detail: "MCDU подключена".into(),
        };
        let mut next = Instant::now();
        let mut rendered = 0u64;
        let mut mode_frames = [0u64; 3];
        let mut render_us = 0u128;
        let mut snapshot_at = Instant::now();
        let outcome = (|| -> Result<()> {
            while !stop.load(Ordering::Relaxed) && !usb_stop.load(Ordering::Relaxed) {
                if options.seconds > 0 && started.elapsed() >= Duration::from_secs(options.seconds)
                {
                    break;
                }
                let before = (
                    controller.playing,
                    controller.mode,
                    controller.navigation.generation,
                );
                for command in pending_commands(commands) {
                    controller.command(command, input, stop);
                }
                controller.buttons(input, stop);
                if before.1 != controller.mode {
                    std::fs::create_dir_all(&options.directory)?;
                    std::fs::write(
                        options.directory.join("preferences.json"),
                        serde_json::to_vec_pretty(&Preferences {
                            mode: controller.mode,
                        })?,
                    )?;
                }
                let changed = before
                    != (
                        controller.playing,
                        controller.mode,
                        controller.navigation.generation,
                    );
                if changed {
                    next = Instant::now();
                }
                if changed && !controller.playing {
                    screen = controller.navigation.render(cols, rows);
                    mailbox.publish(DisplayFrame {
                        profile: controller.navigation.font_profile(),
                        mcdu_fm: controller.navigation.mcdu_fm(),
                        cells: screen.clone(),
                    });
                }
                if Instant::now() >= next && controller.playing {
                    let fresh = source.update_from(&shared.lock().expect("frame mutex"));
                    if fresh || changed {
                        // Any original DOOM menu returns to the single-item host menu.
                        if source.text.menu != 0 {
                            controller.command(Command::Home, input, stop);
                            screen = home(cols, rows);
                        } else {
                            let began = Instant::now();
                            screen = match controller.mode {
                                OutputMode::Ascii => ascii.convert(&source),
                                mode => {
                                    renderer.set_style(if mode == OutputMode::Contrast {
                                        Style::Contrast
                                    } else {
                                        Style::Color
                                    });
                                    renderer.convert_frame(&source, true)?
                                }
                            };
                            rendered += 1;
                            mode_frames[controller.mode as usize] += 1;
                            render_us += began.elapsed().as_micros();
                        }
                        mailbox.publish(DisplayFrame {
                            profile: controller.navigation.font_profile(),
                            mcdu_fm: controller.navigation.mcdu_fm(),
                            cells: screen.clone(),
                        });
                    }
                    next = Instant::now()
                        + Duration::from_secs_f64(if controller.mode == OutputMode::Ascii {
                            1.0 / 30.0
                        } else {
                            1.0 / 5.0
                        });
                }
                {
                    let mut info = status.lock().expect("status mutex");
                    info.playing = controller.playing;
                    info.mode = controller.mode;
                    info.application = controller.navigation.view;
                }
                if Instant::now() >= snapshot_at || changed {
                    let info = status.lock().expect("status mutex");
                    let snapshot = serde_json::json!({
                        "application": info.application,
                        "connected": info.connected,
                        "playing": info.playing,
                        "mode": info.mode,
                        "detail": info.detail,
                        "usb": stats.snapshot(),
                        "mode_frames": mode_frames,
                    });
                    if std::fs::create_dir_all(&options.directory).is_ok() {
                        let _ = std::fs::write(
                            options.directory.join("status.json"),
                            serde_json::to_vec_pretty(&snapshot)?,
                        );
                    }
                    snapshot_at = Instant::now() + Duration::from_secs(1);
                }
                thread::sleep(Duration::from_millis(2));
            }
            Ok(())
        })();
        usb_stop.store(true, Ordering::Relaxed);
        let usb_result = if let Some(usb) = usb {
            usb.join()
                .map_err(|_| anyhow::anyhow!("USB worker panicked"))?
        } else {
            Ok(())
        };
        if let Some(directory) = &options.capture {
            let aircraft_atlas;
            let (atlas, geometry) = if controller.navigation.font_profile() == FontProfile::Aircraft
            {
                aircraft_atlas = crate::mcdu::display::atlas();
                (&aircraft_atlas, crate::mcdu::display::geometry())
            } else {
                (atlas, geometry)
            };
            let cols = geometry.columns as usize;
            let rows = geometry.rows as usize;
            let capture = crate::output::capture::Capture::new(directory)?;
            let mut pixels = vec![0; geometry.width() * geometry.height()];
            crate::display::render::rasterize_grid(
                atlas,
                &screen,
                cols,
                rows,
                geometry.glyph_width as usize,
                geometry.glyph_height as usize,
                &mut pixels,
            );
            capture.image("mcdu.png", &pixels, geometry.width(), geometry.height())?;
            capture.source(&source)?;
            capture.json(
                "metrics.json",
                &serde_json::json!({
                    "application": controller.navigation.view,
                    "mode": controller.mode,
                    "playing": controller.playing,
                    "no_usb": options.no_usb,
                    "rendered_frames": rendered,
                    "mode_frames": mode_frames,
                    "render_mean_ms": if rendered > 0 {
                        render_us as f64 / rendered as f64 / 1000.0
                    } else { 0.0 },
                    "screen_reports": crate::device::protocol::encode_cells(atlas, &screen)?.len(),
                    "usb": stats.snapshot(),
                }),
            )?;
        }
        outcome?;
        usb_result
    }
}
