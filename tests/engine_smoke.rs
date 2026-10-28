// SPDX-License-Identifier: GPL-3.0-only
//! Headless engine lifecycle; opt in after installing the pinned Freedoom data.
use mcdu_doom::{
    config::DEFAULT_WAD,
    doom::engine::{self, DOOM_H, DOOM_W, Frame},
    input::Inputs,
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

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn publishes_game_frames_and_shuts_down_without_usb() {
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_error = engine::stop_on_drop(&stop);
    let frame = Arc::new(Mutex::new(Frame::default()));
    let input = Arc::new(Inputs::default());
    let worker = engine::spawn(
        Path::new(DEFAULT_WAD),
        true,
        Arc::clone(&frame),
        Arc::clone(&input),
        Arc::clone(&stop),
    )
    .expect("start engine");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut playable = None;
    while Instant::now() < deadline && !worker.is_finished() {
        let current = frame.lock().expect("frame").clone();
        if current.generation >= 3 && current.hud.active != 0 {
            playable = Some(current);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    stop.store(true, Ordering::Relaxed);
    let shutdown_deadline = Instant::now() + Duration::from_secs(5);
    while !worker.is_finished() && Instant::now() < shutdown_deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        worker.is_finished(),
        "engine did not stop before the deadline"
    );
    worker
        .join()
        .expect("engine thread")
        .expect("clean C shutdown");
    let playable = playable.expect("engine did not publish gameplay frames");
    assert_eq!(playable.pixels.len(), DOOM_W * DOOM_H);
    assert_eq!(playable.semantics.len(), DOOM_W * DOOM_H);
    assert_eq!(playable.viewport, [320, 168]);
    assert!(playable.pixels.iter().any(|&pixel| pixel != 0));
    assert!(playable.semantics.iter().all(|&kind| kind <= 2));
    assert!(engine::spawn(Path::new(DEFAULT_WAD), true, frame, input, stop).is_err());
}
