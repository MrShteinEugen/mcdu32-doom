// SPDX-License-Identifier: GPL-3.0-only
//! A separate test process keeps the single-instance C engine isolated.
use mcdu_doom::{
    doom::engine::{self, Frame},
    input::Inputs,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[test]
fn invalid_wad_returns_a_rust_error_instead_of_exiting_the_process() {
    let directory =
        std::path::PathBuf::from("target/engine-errors").join(std::process::id().to_string());
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let path = directory.join("invalid.wad");
    std::fs::write(&path, b"invalid WAD data").expect("invalid WAD fixture");
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_error = engine::stop_on_drop(&stop);
    let worker = engine::spawn(
        &path,
        true,
        Arc::new(Mutex::new(Frame::default())),
        Arc::new(Inputs::default()),
        Arc::clone(&stop),
    )
    .expect("worker starts; the C engine validates the WAD");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !worker.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        worker.is_finished(),
        "engine did not return after a fatal WAD error"
    );
    assert!(
        worker
            .join()
            .expect("engine thread must not panic")
            .is_err()
    );
    assert!(stop.load(Ordering::Relaxed));
}
