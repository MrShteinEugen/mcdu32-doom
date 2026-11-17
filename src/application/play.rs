// SPDX-License-Identifier: GPL-3.0-only
//! Physical MCDU output and the matching raster preview.
use super::args::PlayArgs;
use super::play_metrics::PlayMetrics;
use crate::{
    desktop::keyboard::keyboard_state,
    device::transport::{self, UsbStats},
    display::{
        Cell,
        atlas::Atlas,
        render::{Renderer, Style, rasterize_grid},
    },
    doom::engine::{self, Frame},
    input::Inputs,
    output::{
        capture::Capture,
        metrics::{GameMetrics, RenderTimings},
    },
    runtime::latest::Latest,
};
use anyhow::{Result, bail};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub(super) fn run(options: PlayArgs) -> Result<()> {
    let profile = options.profile();
    let wad = crate::doom::wad::Wad::open(&options.common.wad)?;
    let atlas = Atlas::from_wad(&wad);
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_error = engine::stop_on_drop(&stop);
    let shutdown = Arc::clone(&stop);
    ctrlc::set_handler(move || shutdown.store(true, Ordering::Relaxed))?;
    let input = Arc::new(Inputs::default());
    let frame = Arc::new(Mutex::new(Frame::default()));
    let mailbox = Arc::new(Latest::<Vec<Cell>>::default());
    let geometry = profile.geometry();
    let (cols, rows, cell_w, cell_h) = (
        geometry.columns as usize,
        geometry.rows as usize,
        geometry.glyph_width as usize,
        geometry.glyph_height as usize,
    );
    let stats = Arc::new(UsbStats::default());
    let (view_width, view_height) = if options.original {
        (engine::DOOM_W, engine::DOOM_H)
    } else {
        (cols * cell_w, rows * cell_h)
    };
    let mut window = if options.common.headless {
        None
    } else {
        let mut window = Window::new(
            if options.original {
                "DOOM — original graphics"
            } else if options.common.style == Style::Contrast {
                "MCDU DOOM — contrast / F6 color"
            } else {
                "MCDU DOOM — color / F6 contrast"
            },
            view_width,
            view_height,
            WindowOptions {
                resize: true,
                scale: if options.original {
                    minifb::Scale::X2
                } else {
                    minifb::Scale::X1
                },
                ..WindowOptions::default()
            },
        )?;
        window.set_target_fps(120);
        Some(window)
    };
    let mut renderer = Renderer::for_cells(&atlas, cols, rows, cell_w, cell_h)?;
    renderer.set_style(options.common.style);
    let usb = if options.mcdu {
        Some(transport::spawn(
            atlas.clone(),
            options.device_path,
            Arc::clone(&mailbox),
            Arc::clone(&input),
            Arc::clone(&stop),
            Arc::clone(&stats),
            transport::DisplayOptions {
                pacing: Duration::from_micros(options.timing.packet_us),
                geometry,
                legacy_font: options.legacy_font,
                keep_grid: options.keep_grid,
            },
        )?)
    } else {
        None
    };
    let game = match engine::spawn(
        &options.common.wad,
        !options.menu,
        Arc::clone(&frame),
        Arc::clone(&input),
        Arc::clone(&stop),
    ) {
        Ok(game) => game,
        Err(error) => {
            stop.store(true, Ordering::Relaxed);
            if let Some(usb) = usb {
                let _ = usb.join();
            }
            return Err(error);
        }
    };
    let mut source = Frame::default();
    let mut screen = vec![Cell::default(); cols * rows];
    let mut pixels = vec![0; view_width * view_height];
    let started = Instant::now();
    if options.benchmark {
        input.keyboard.store(1 << 3, Ordering::Relaxed);
    }
    let mut next_frame = started;
    let mut timings = RenderTimings::default();
    let interval = Duration::from_secs_f64(1.0 / f64::from(options.timing.fps));
    eprintln!(
        "Controls: arrows/WASD, Ctrl fire, Space use, Shift run, Q/E strafe, 1–7 weapons, F6 contrast/color. Close window or Ctrl+C to stop."
    );
    let outcome = (|| -> Result<()> {
        while !stop.load(Ordering::Relaxed) {
            let limit = if options.benchmark && options.common.seconds == 0 {
                10
            } else {
                options.common.seconds
            };
            if limit > 0 && started.elapsed() >= Duration::from_secs(limit) {
                break;
            }
            if let Some(window) = &mut window {
                window.update();
                if !window.is_open() {
                    break;
                }
                if !options.original && window.is_key_pressed(Key::F6, KeyRepeat::No) {
                    renderer.set_style(if renderer.style() == Style::Contrast {
                        Style::Color
                    } else {
                        Style::Contrast
                    });
                    window.set_title(&format!(
                        "MCDU DOOM — {:?} / F6 toggles contrast",
                        renderer.style()
                    ));
                }
                input
                    .keyboard
                    .store(keyboard_state(window), Ordering::Relaxed);
            }
            if Instant::now() >= next_frame {
                let fresh = {
                    let shared = frame.lock().expect("engine frame mutex poisoned");
                    source.update_from(&shared)
                };
                if fresh {
                    let start = Instant::now();
                    if options.original {
                        pixels.copy_from_slice(&source.pixels);
                    } else {
                        screen = renderer.convert_frame(&source, true)?;
                        rasterize_grid(&atlas, &screen, cols, rows, cell_w, cell_h, &mut pixels);
                    }
                    timings.record(start.elapsed());
                    if options.mcdu {
                        mailbox.publish(screen.clone());
                    }
                    if let Some(window) = &mut window {
                        window.update_with_buffer(&pixels, view_width, view_height)?;
                    }
                }
                // No catch-up burst: old presentation deadlines are dropped.
                next_frame += interval;
                if next_frame <= Instant::now() {
                    next_frame = Instant::now() + interval;
                }
            }
            if window.is_none() {
                thread::sleep(Duration::from_millis(1));
            }
        }
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    input.keyboard.store(0, Ordering::Relaxed);
    let game_result = game
        .join()
        .map_err(|_| anyhow::anyhow!("DOOM engine thread panicked"));
    let usb_result = if let Some(usb) = usb {
        usb.join()
            .map_err(|_| anyhow::anyhow!("USB thread panicked"))?
    } else {
        Ok(())
    };
    game_result??;
    usb_result?;
    outcome?;
    if timings.frames() == 0 {
        bail!("engine produced no frames");
    }
    let metrics = PlayMetrics {
        style: renderer.style(),
        display: profile,
        geometry: geometry.into(),
        packet_us: options.timing.packet_us,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        render: timings.snapshot(),
        usb: stats.snapshot(),
        game: GameMetrics::new(&source, &atlas, cols),
    };
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    if let Some(directory) = &options.common.capture {
        let capture = Capture::new(directory)?;
        if options.mcdu {
            let last = stats
                .last_screen
                .lock()
                .expect("USB frame snapshot poisoned");
            if !last.is_empty() {
                rasterize_grid(&atlas, &last, cols, rows, cell_w, cell_h, &mut pixels);
            }
        }
        capture.image(
            if options.original {
                "original.png"
            } else {
                "mcdu.png"
            },
            &pixels,
            view_width,
            view_height,
        )?;
        capture.source(&source)?;
        if !options.original {
            capture.comparisons(&atlas, &source, geometry, renderer.style())?;
        }
        capture.json("metrics.json", &metrics)?;
    }
    Ok(())
}
