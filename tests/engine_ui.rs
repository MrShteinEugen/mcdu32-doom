// SPDX-License-Identifier: GPL-3.0-only
use mcdu_doom::output::capture::save_png;
use mcdu_doom::{
    display::atlas::Atlas,
    display::render::{Renderer, Style, rasterize_grid},
    doom::engine::{self, Frame},
    doom::wad::Wad,
    input::{Inputs, decode_mcdu, decode_raw},
};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

fn wait_frame(shared: &Mutex<Frame>, condition: impl Fn(&Frame) -> bool) -> Frame {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let frame = shared.lock().expect("frame");
        if condition(&frame) {
            return frame.clone();
        }
        drop(frame);
        assert!(
            Instant::now() < deadline,
            "engine UI did not reach expected state"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
fn report(input: &Inputs, byte: usize, mask: u8) {
    let mut report = [0; 14];
    report[0] = 1;
    report[byte] = mask;
    input
        .mcdu
        .store(decode_mcdu(&report).expect("report"), Ordering::Relaxed);
    input.update_raw(decode_raw(&report));
}
fn capture(name: &str, frame: &Frame) {
    let Some(directory) = std::env::var_os("MCDU_UI_CAPTURE_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory).expect("capture directory");
    let wad = Wad::open(Path::new("data/freedoom-0.13.0/freedoom1.wad")).expect("WAD");
    let mut atlas = Atlas::universal();
    atlas.set_portraits(
        &wad.portrait("STFST00").expect("normal face"),
        &wad.portrait("STFOUCH0").expect("hurt face"),
    );
    let mut renderer = Renderer::for_cells(&atlas, 53, 31, 11, 14).expect("renderer");
    renderer.set_style(Style::Contrast);
    let cells = renderer.convert_frame(frame, true).expect("render UI");
    let mut pixels = vec![0; 583 * 434];
    rasterize_grid(&atlas, &cells, 53, 31, 11, 14, &mut pixels);
    save_png(&directory.join(format!("{name}.png")), &pixels, 583, 434).expect("PNG");
    std::fs::write(
        directory.join(format!("{name}.txt")),
        frame
            .text
            .lines()
            .iter()
            .map(|line| line.value())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .expect("text");
}

#[test]
#[ignore = "requires Freedoom 0.13.0; synthetic panel reports only, no USB"]
fn physical_text_area_reports_fire_open_readable_menu_and_cannot_shrink_view() {
    let wad = Path::new("data/freedoom-0.13.0/freedoom1.wad");
    assert!(wad.is_file(), "install pinned Freedoom first");
    let shared = Arc::new(Mutex::new(Frame::default()));
    let inputs = Arc::new(Inputs::default());
    let stop = Arc::new(AtomicBool::new(false));
    let _guard = engine::stop_on_drop(&stop);
    let game = engine::spawn(
        wad,
        true,
        Arc::clone(&shared),
        Arc::clone(&inputs),
        Arc::clone(&stop),
    )
    .expect("engine");
    let initial = wait_frame(&shared, |f| {
        f.hud.active != 0 && f.viewport == [320, 168] && f.hud.ammo == 50 && f.generation > 35
    });
    capture("gameplay", &initial);
    for _ in 0..5 {
        report(&inputs, 6, 8); // +/-
        thread::sleep(Duration::from_millis(60));
        report(&inputs, 6, 0);
        thread::sleep(Duration::from_millis(60));
    }
    assert_eq!(shared.lock().expect("frame").viewport, [320, 168]);
    report(&inputs, 10, 1); // OVERFLY
    wait_frame(&shared, |f| f.hud.ammo < initial.hud.ammo);
    report(&inputs, 10, 0);
    thread::sleep(Duration::from_millis(80));
    report(&inputs, 7, 1); // E -> Escape
    let menu = wait_frame(&shared, |f| {
        f.text.menu == 1
            && f.text
                .lines()
                .iter()
                .any(|l| l.value().contains("NEW GAME"))
    });
    report(&inputs, 7, 0);
    assert!(
        menu.text
            .lines()
            .iter()
            .any(|l| l.value().starts_with("> "))
    );
    capture("menu", &menu);
    thread::sleep(Duration::from_millis(80));
    report(&inputs, 6, 0x80); // D -> Enter: New Game
    wait_frame(&shared, |f| {
        f.text.lines().iter().any(|l| l.value() == "CHOOSE EPISODE")
    });
    report(&inputs, 6, 0);
    thread::sleep(Duration::from_millis(80));
    report(&inputs, 7, 1); // Escape closes the menu
    wait_frame(&shared, |f| f.text.menu == 0);
    report(&inputs, 7, 0);
    // IDDQD does not use the reserved A-E keys at this layer. The message is
    // exported from HUlib with the same lifetime as the original HUD message.
    for key in b"iddqd" {
        let mut bitmap = [0; 4];
        bitmap[*key as usize / 64] = 1 << (*key % 64);
        inputs.update_raw(bitmap);
        thread::sleep(Duration::from_millis(70));
        inputs.update_raw([0; 4]);
        thread::sleep(Duration::from_millis(70));
    }
    let message = wait_frame(&shared, |f| {
        f.text
            .lines()
            .iter()
            .any(|l| l.value().to_ascii_uppercase().contains("MODE"))
    });
    capture("message", &message);
    stop.store(true, Ordering::Relaxed);
    game.join().expect("engine thread").expect("shutdown");
}
