// SPDX-License-Identifier: GPL-3.0-only
//! Capture actual engine frames and render every comparison from the same snapshot.
//! No windows or USB devices are opened. Run through scripts/capture-readme.ps1.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use mcdu_doom::{
    device::protocol,
    display::{
        Cell, Geometry, Profile,
        ascii::{self, AsciiRenderer},
        atlas::Atlas,
        render::{Renderer, Style, rasterize_grid},
    },
    doom::{
        engine::{self, Frame},
        wad::Wad,
    },
    input::Inputs,
    mcdu::{self, Avionics, Page},
    output::capture::save_png,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "docs/images")]
    out: PathBuf,
}

struct Picture {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
}
fn raster(atlas: &Atlas, cells: &[Cell], grid: Geometry) -> Picture {
    let mut pixels = vec![0; grid.width() * grid.height()];
    rasterize_grid(
        atlas,
        cells,
        grid.columns as usize,
        grid.rows as usize,
        grid.glyph_width as usize,
        grid.glyph_height as usize,
        &mut pixels,
    );
    Picture {
        pixels,
        width: grid.width(),
        height: grid.height(),
    }
}
fn save(out: &Path, name: &str, image: &Picture) -> Result<()> {
    save_png(
        &out.join(format!("{name}.png")),
        &image.pixels,
        image.width,
        image.height,
    )
}
fn comparison(out: &Path, name: &str, frame: &Frame, atlas: &Atlas) -> Result<serde_json::Value> {
    let grid = Profile::Full.geometry();
    let source = Picture {
        pixels: frame.pixels.clone(),
        width: 320,
        height: 200,
    };
    save(out, &format!("{name}-source"), &source)?;
    let mut pictures = vec![source];
    let mut modes = Vec::new();
    for (label, style) in [("glyphs", Style::Color), ("contrast", Style::Contrast)] {
        let mut renderer = Renderer::for_cells(atlas, 53, 31, 11, 14)?;
        renderer.set_style(style);
        let began = Instant::now();
        let cells = renderer.convert_frame(frame, true)?;
        let elapsed = began.elapsed().as_secs_f64() * 1000.0;
        let image = raster(atlas, &cells, grid);
        save(out, &format!("{name}-{label}"), &image)?;
        modes.push(serde_json::json!({"mode":label,"reports":protocol::encode_cells(atlas,&cells)?.len(),"render_ms":elapsed}));
        pictures.push(image);
    }
    let mut ascii = AsciiRenderer::new(atlas, 53, 31, 11, 14);
    let began = Instant::now();
    let cells = ascii.convert(frame);
    let elapsed = began.elapsed().as_secs_f64() * 1000.0;
    let image = raster(atlas, &cells, grid);
    save(out, &format!("{name}-ascii"), &image)?;
    modes.push(serde_json::json!({"mode":"color-ascii","reports":protocol::encode_cells(atlas,&cells)?.len(),"render_ms":elapsed}));
    pictures.push(image);
    // Source is enlarged exactly 2x without changing its 320:200 pixel ratio.
    // Other tiles retain their native 583x434 raster. Padding is presentation only.
    let text_atlas = mcdu::display::atlas();
    let (tile_w, tile_h, gap, margin) = (640, 478, 16, 20);
    let (width, height) = (tile_w * 2 + gap + margin * 2, tile_h * 2 + gap + margin * 2);
    let mut sheet = Picture {
        pixels: vec![0x15191f; width * height],
        width,
        height,
    };
    for (i, (image, label)) in pictures
        .iter()
        .zip([
            "SOURCE - FREEDOOM",
            "GLYPHS - COLOR",
            "GLYPHS - CONTRAST",
            "COLOR ASCII",
        ])
        .enumerate()
    {
        let left = margin + (i % 2) * (tile_w + gap);
        let top = margin + (i / 2) * (tile_h + gap);
        let mut text = mcdu::display::Text::default();
        text.put(0, 0, label, 2, false);
        let title = raster(&text_atlas, &text.cells, mcdu::display::geometry());
        for y in 0..29 {
            for x in 0..title.width {
                sheet.pixels[(top + y) * width + left + 10 + x] = title.pixels[y * title.width + x];
            }
        }
        let scale = if i == 0 { 2 } else { 1 };
        let x0 = left + (tile_w - image.width * scale) / 2;
        let y0 = top + 40 + (434 - image.height * scale) / 2;
        for y in 0..image.height * scale {
            for x in 0..image.width * scale {
                sheet.pixels[(y0 + y) * width + x0 + x] =
                    image.pixels[(y / scale) * image.width + x / scale];
            }
        }
    }
    save(out, &format!("{name}-comparison"), &sheet)?;
    let fingerprint = frame
        .pixels
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, pixel| {
            pixel
                .to_le_bytes()
                .into_iter()
                .fold(hash, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
        });
    Ok(
        serde_json::json!({"scene":name,"source_generation":frame.generation,"source_fnv1a64":format!("{fingerprint:016x}"),"source_pixels":[320,200],"display_pixels":[583,434],"visible_enemy_pixels":frame.semantics.iter().filter(|&&v|v==1).count(),"modes":modes}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;
    let path = Path::new("data/freedoom-0.13.0/freedoom1.wad");
    let wad = Wad::open(path).context("install pinned Freedoom with scripts/setup-freedoom.ps1")?;
    let mut atlas = Atlas::from_wad(&wad);
    ascii::install(&mut atlas);
    let shared = Arc::new(Mutex::new(Frame::default()));
    let input = Arc::new(Inputs::default());
    let stop = Arc::new(AtomicBool::new(false));
    let _guard = engine::stop_on_drop(&stop);
    let worker = engine::spawn(
        path,
        true,
        Arc::clone(&shared),
        Arc::clone(&input),
        Arc::clone(&stop),
    )?;
    let result = (|| -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let first = loop {
            let frame = shared.lock().expect("frame").clone();
            if frame.hud.active != 0 && frame.generation > 35 {
                break frame;
            }
            ensure!(
                Instant::now() < deadline,
                "engine did not publish a gameplay frame"
            );
            thread::sleep(Duration::from_millis(10));
        };
        let mut best = first.clone();
        let mut score = usize::MAX;
        for (keys, millis) in [
            (1 << 0, 7000),
            (1 << 2, 800),
            (1 << 0, 4000),
            (1 << 3, 2000),
        ] {
            input.keyboard.store(keys, Ordering::Relaxed);
            let until = Instant::now() + Duration::from_millis(millis);
            while Instant::now() < until {
                // Re-press USE while walking, rather than holding it outside
                // a door's activation range for the whole movement interval.
                let use_key = if keys & 1 != 0
                    && Instant::now()
                        .duration_since(until - Duration::from_millis(millis))
                        .as_millis()
                        / 250
                        % 2
                        == 0
                {
                    1 << 5
                } else {
                    0
                };
                input.keyboard.store(keys | use_key, Ordering::Relaxed);
                let frame = shared.lock().expect("frame").clone();
                let enemies = frame.semantics.iter().filter(|&&v| v == 1).count();
                let next_score = enemies.abs_diff(5000);
                if frame.hud.health > 0 && enemies >= 150 && next_score < score {
                    best = frame;
                    score = next_score;
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
        input.keyboard.store(0, Ordering::Relaxed);
        let has_encounter = score != usize::MAX;
        ensure!(
            has_encounter,
            "no visible-enemy snapshot found; retry the capture instead of publishing a mislabeled encounter"
        );
        let scenes = vec![
            comparison(&args.out, "foyer", &first, &atlas)?,
            comparison(&args.out, "encounter", &best, &atlas)?,
        ];
        let text_atlas = mcdu::display::atlas();
        let mut pages = Avionics::default();
        for (name, page) in [
            ("mcdu-menu", Page::Menu),
            ("mcdu-init", Page::InitA),
            ("mcdu-perf", Page::Perf(0)),
            ("mcdu-prog", Page::Prog),
            ("mcdu-radio", Page::RadNav),
        ] {
            pages.select(page);
            save(
                &args.out,
                name,
                &raster(
                    &text_atlas,
                    &pages.render(24, 14),
                    mcdu::display::geometry(),
                ),
            )?;
        }
        std::fs::write(
            args.out.join("gallery.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"game_data":"Freedoom Phase 1 0.13.0","scenes":scenes,"usb_opened":false,"images":"software captures; not photographs of physical LCD","comparison":"each four-way comparison uses one immutable engine Frame"}),
            )?,
        )?;
        println!("Gallery written to {}", args.out.display());
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    let engine_result = worker
        .join()
        .map_err(|_| anyhow::anyhow!("engine thread panicked"))?;
    result?;
    engine_result
}
