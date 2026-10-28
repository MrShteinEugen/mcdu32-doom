// SPDX-License-Identifier: GPL-3.0-only
//! Virtual HID display timing and an interactive, device-free MCDU front panel.
use super::panel::{LCD_H, LCD_W, PANEL_H, PANEL_W, Panel};
use crate::{
    device::virtual_display::VirtualDisplay,
    display::{
        atlas::Atlas,
        render::{Renderer, Style, rasterize_grid},
    },
    doom::{
        engine::{self, Frame},
        wad::Wad,
    },
    input::Inputs,
    output::{
        capture::Capture,
        metrics::{DisplayMetrics, GameMetrics, RenderMetrics, RenderTimings},
    },
};
use anyhow::{Result, ensure};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(clap::Args)]
pub struct Options {
    /// Glyph rendering style.
    #[arg(long, value_enum, default_value = "color")]
    pub style: Style,
    /// Show the original frame beside the glyph display.
    #[arg(long)]
    pub comparison: bool,
    #[arg(long, default_value = crate::config::DEFAULT_WAD)]
    pub wad: PathBuf,
    #[arg(long = "columns", default_value_t = 53)]
    pub cols: usize,
    #[arg(long, default_value_t = 31)]
    pub rows: usize,
    #[arg(long, default_value_t = crate::config::DEFAULT_FPS, value_parser = clap::value_parser!(u32).range(1..))]
    pub fps: u32,
    #[arg(long, default_value_t = crate::config::DEFAULT_PACKET_US)]
    pub packet_us: u64,
    /// Run without opening windows.
    #[arg(long)]
    pub headless: bool,
    /// Stop after this many seconds; zero runs until closed.
    #[arg(long, default_value_t = 0)]
    pub seconds: u64,
    /// Write PNG captures and metrics to this directory after stopping.
    #[arg(long)]
    pub capture: Option<PathBuf>,
    /// Exercise movement and firing through the emulated panel input.
    #[arg(long)]
    pub benchmark: bool,
}

#[derive(serde::Serialize)]
struct EmulatorMetrics {
    style: Style,
    mode: &'static str,
    usb_access: bool,
    #[serde(flatten)]
    geometry: DisplayMetrics,
    fps_limit: u32,
    packet_us: u64,
    elapsed_seconds: f64,
    #[serde(flatten)]
    game: GameMetrics,
    #[serde(flatten)]
    render: RenderMetrics,
    presented_frames: u64,
    dropped_waiting_frames: u64,
    simulated_reports: u64,
    last_transfer_ms: f64,
}

/// No HID enumeration or USB writes occur anywhere in this entry point.
pub fn run(options: Options) -> Result<()> {
    let geometry = crate::display::Geometry::simulated(options.cols, options.rows)?;
    ensure!(options.fps > 0, "emulator FPS must be greater than zero");
    let wad = Wad::open(&options.wad)?;
    let atlas = Atlas::from_wad(&wad);
    let (cell_w, cell_h) = (
        usize::from(geometry.glyph_width),
        usize::from(geometry.glyph_height),
    );
    let mut renderer = Renderer::for_cells(&atlas, options.cols, options.rows, cell_w, cell_h)?;
    renderer.set_style(options.style);
    let width = options.cols * cell_w;
    let height = options.rows * cell_h;
    let mut sender = VirtualDisplay::new(
        options.cols * options.rows,
        Duration::from_micros(options.packet_us),
        options.fps,
    )?;
    let mut panel = Panel::new();
    let mut window = if options.headless {
        None
    } else {
        let mut window = Window::new(
            &format!(
                "MCDU DOOM — {}x{} / max {} FPS / no USB",
                options.cols, options.rows, options.fps
            ),
            if options.comparison { LCD_W } else { PANEL_W },
            if options.comparison { LCD_H } else { PANEL_H },
            WindowOptions::default(),
        )?;
        window.set_target_fps(60);
        if options.comparison {
            window.set_position(710, 100);
        }
        Some(window)
    };
    let mut original_window = if options.comparison && !options.headless {
        let mut window = Window::new(
            "DOOM — original graphics / same game",
            engine::DOOM_W,
            engine::DOOM_H,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )?;
        window.set_target_fps(0);
        window.set_position(40, 100);
        Some(window)
    } else {
        None
    };
    let stop = Arc::new(AtomicBool::new(false));
    let on_ctrl_c = Arc::clone(&stop);
    ctrlc::set_handler(move || on_ctrl_c.store(true, Ordering::Relaxed))?;
    let _stop_on_error = engine::stop_on_drop(&stop);
    let inputs = Arc::new(Inputs::default());
    let shared = Arc::new(Mutex::new(Frame::default()));
    let game = engine::spawn(
        &options.wad,
        true,
        Arc::clone(&shared),
        Arc::clone(&inputs),
        Arc::clone(&stop),
    )?;
    let mut source = Frame::default();
    let mut pixels = vec![0; width * height];
    let mut lcd = vec![0; LCD_W * LCD_H];
    let mut surface = vec![0; PANEL_W * PANEL_H];
    let started = Instant::now();
    let mut next_prepare = Instant::now();
    let mut timings = RenderTimings::default();
    let mut prepared_generation = 0;
    let mut enhanced = true;
    let prepare_interval = prepare_interval(options.fps);
    if options.comparison {
        println!(
            "One game, two views: original RGB and {}x{} glyphs, <= {} FPS. F5 toggles sprite emphasis.",
            options.cols, options.rows, options.fps
        );
    } else {
        println!(
            "Device-free MCDU emulator: {}x{} cells; {}x{} pixels per glyph; presentation <= {} FPS. LMB holds, RMB latches, F12 releases all.",
            options.cols, options.rows, cell_w, cell_h, options.fps
        );
    }
    let outcome = (|| -> Result<()> {
        while !stop.load(Ordering::Relaxed) {
            let now = started.elapsed();
            let limit = if options.benchmark && options.seconds == 0 {
                10
            } else {
                options.seconds
            };
            if limit > 0 && now >= Duration::from_secs(limit) {
                break;
            }
            let mut report = [0_u8; 14];
            report[0] = 1;
            if let Some(window) = &window {
                if !window.is_open() {
                    break;
                }
                report = if options.comparison {
                    Panel::keyboard_input(window)
                } else {
                    panel.input(window)
                };
                handle_display_keys(window, &mut enhanced, &mut renderer);
            }
            if let Some(window) = &original_window {
                handle_display_keys(window, &mut enhanced, &mut renderer);
                if !window.is_open() {
                    break;
                }
                for (byte, key) in report.iter_mut().zip(Panel::keyboard_input(window)) {
                    *byte |= key;
                }
            }
            if options.benchmark && source.hud.active != 0 {
                // Exercise the same report decoder as a held MCDU right arrow + R1.
                report[4] |= 0x40;
                report[1] |= 0x40;
                panel.display_report(&report);
            }
            inputs.update_panel(&report);
            {
                let frame = shared.lock().expect("engine frame mutex poisoned");
                source.update_from(&frame);
            }
            if Instant::now() >= next_prepare {
                if source.generation != prepared_generation {
                    let start = Instant::now();
                    let cells = renderer.convert_frame(&source, enhanced)?;
                    timings.record(start.elapsed());
                    sender.submit(cells, started.elapsed(), &atlas)?;
                    prepared_generation = source.generation;
                }
                next_prepare = Instant::now() + prepare_interval;
            }
            if sender.advance(started.elapsed(), &atlas)? {
                rasterize_grid(
                    &atlas,
                    &sender.visible,
                    options.cols,
                    options.rows,
                    cell_w,
                    cell_h,
                    &mut pixels,
                );
                lcd.fill(0);
                let x_offset = (LCD_W - width) / 2;
                let y_offset = (LCD_H - height) / 2;
                for y in 0..height {
                    lcd[(y + y_offset) * LCD_W + x_offset
                        ..(y + y_offset) * LCD_W + x_offset + width]
                        .copy_from_slice(&pixels[y * width..(y + 1) * width]);
                }
            }
            let status = format!(
                "SIM ONLY  {}X{}  {} FRAMES  LAST {:.0} MS  MAX {} FPS",
                options.cols,
                options.rows,
                sender.presented,
                sender.last_transfer.as_secs_f64() * 1000.0,
                options.fps
            );
            if !options.comparison {
                panel.draw(&lcd, &status, &mut surface);
            }
            if let Some(window) = &mut original_window {
                window.update_with_buffer(&source.pixels, engine::DOOM_W, engine::DOOM_H)?;
            }
            if let Some(window) = &mut window {
                if options.comparison {
                    window.set_title(if enhanced {
                        if renderer.style() == Style::Contrast {
                            "Glyph graphics — contrast / F6 color / F5 enemies"
                        } else {
                            "Glyph graphics — color / F6 contrast / F5 enemies"
                        }
                    } else {
                        "Glyph graphics — no sprite emphasis / F5 enemies / F6 contrast"
                    });
                    window.update_with_buffer(&lcd, LCD_W, LCD_H)?;
                } else {
                    window.update_with_buffer(&surface, PANEL_W, PANEL_H)?;
                }
            } else {
                thread::sleep(Duration::from_millis(2));
            }
        }
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    inputs.release_panel();
    game.join()
        .map_err(|_| anyhow::anyhow!("engine thread panicked"))??;
    outcome?;
    let metrics = EmulatorMetrics {
        style: renderer.style(),
        mode: if options.comparison {
            "comparison"
        } else {
            "emulator"
        },
        usb_access: false,
        geometry: geometry.into(),
        fps_limit: options.fps,
        packet_us: options.packet_us,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        game: GameMetrics::new(&source, &atlas, options.cols),
        render: timings.snapshot(),
        presented_frames: sender.presented,
        dropped_waiting_frames: sender.dropped,
        simulated_reports: sender.reports,
        last_transfer_ms: sender.last_transfer.as_secs_f64() * 1000.0,
    };
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    if let Some(out) = options.capture {
        let capture = Capture::new(&out)?;
        if !options.comparison {
            capture.image("emulator.png", &surface, PANEL_W, PANEL_H)?;
        }
        capture.image("mcdu.png", &pixels, width, height)?;
        capture.source(&source)?;
        capture.comparisons(&atlas, &source, geometry, renderer.style())?;
        capture.json("metrics.json", &metrics)?;
    }
    Ok(())
}

fn prepare_interval(fps: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(fps.saturating_mul(2).clamp(5, 30)))
}

fn handle_display_keys(window: &Window, enhanced: &mut bool, renderer: &mut Renderer) {
    if window.is_key_pressed(Key::F5, KeyRepeat::No) {
        *enhanced = !*enhanced;
    }
    if window.is_key_pressed(Key::F6, KeyRepeat::No) {
        renderer.set_style(if renderer.style() == Style::Contrast {
            Style::Color
        } else {
            Style::Contrast
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        options: Options,
    }

    #[test]
    fn invalid_grid_and_frequency_are_rejected_before_loading_game_data() {
        for args in [
            vec!["emulator", "--columns", "0"],
            vec!["emulator", "--rows", "4"],
        ] {
            let mut options = Command::try_parse_from(args).expect("options").options;
            options.wad = "missing-game-data.wad".into();
            assert!(
                run(options)
                    .expect_err("invalid geometry")
                    .to_string()
                    .contains("grid")
            );
        }
        let mut options = Command::try_parse_from(["emulator"])
            .expect("options")
            .options;
        options.fps = 0;
        options.wad = "missing-game-data.wad".into();
        assert!(
            run(options)
                .expect_err("invalid FPS")
                .to_string()
                .contains("FPS")
        );
        assert!(Command::try_parse_from(["emulator", "--fps", "0"]).is_err());
    }

    #[test]
    fn high_frame_rate_keeps_the_preparation_limit_without_overflow() {
        assert_eq!(
            prepare_interval(u32::MAX),
            Duration::from_secs_f64(1.0 / 30.0)
        );
        assert_eq!(prepare_interval(1), Duration::from_millis(200));
    }
}
