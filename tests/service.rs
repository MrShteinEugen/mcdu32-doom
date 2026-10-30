// SPDX-License-Identifier: GPL-3.0-only
//! Exercise a real C engine with tray commands, without touching USB or the desktop.
use mcdu_doom::{
    display::Profile,
    service::{self, Command, Options, OutputMode, Status},
};
use std::{
    path::Path,
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread,
    time::{Duration, Instant},
};

fn wait_for(status: &Mutex<Status>, predicate: impl Fn(&Status) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if predicate(&status.lock().expect("status")) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "service did not reach expected state"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn real_game_switches_modes_returns_home_and_stops_without_usb() {
    let wad = Path::new("data/freedoom-0.13.0/freedoom1.wad");
    assert!(wad.is_file(), "install Freedoom before running this test");
    let directory = std::path::PathBuf::from("target/service-tests")
        .join(format!("mcdu-service-test-{}", std::process::id()));
    let status = Arc::new(Mutex::new(Status::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let _guard = mcdu_doom::doom::engine::stop_on_drop(&stop);
    let (sender, receiver) = service::channel();
    let worker = service::spawn(
        Options {
            wad: wad.into(),
            profile: Profile::Full,
            mode: OutputMode::Glyphs,
            packet_us: 3000,
            no_usb: true,
            autostart: false,
            seconds: 15,
            capture: Some(directory.clone()),
            directory: directory.clone(),
        },
        receiver,
        Arc::clone(&status),
        Arc::clone(&stop),
    )
    .expect("start service");
    wait_for(&status, |s| s.connected && !s.playing);
    sender.send(Command::Start).expect("start command");
    wait_for(&status, |s| s.playing);
    thread::sleep(Duration::from_millis(700));
    sender
        .send(Command::Mode(OutputMode::Contrast))
        .expect("contrast");
    wait_for(&status, |s| s.mode == OutputMode::Contrast);
    thread::sleep(Duration::from_millis(500));
    sender
        .send(Command::Mode(OutputMode::Ascii))
        .expect("ASCII");
    wait_for(&status, |s| s.mode == OutputMode::Ascii);
    thread::sleep(Duration::from_millis(500));
    sender.send(Command::Home).expect("home");
    wait_for(&status, |s| !s.playing);
    sender.send(Command::Start).expect("restart");
    wait_for(&status, |s| s.playing);
    thread::sleep(Duration::from_millis(200));
    sender.send(Command::Home).expect("home again");
    wait_for(&status, |s| !s.playing);
    sender.send(Command::Quit).expect("quit");
    worker
        .join()
        .expect("worker")
        .expect("clean service shutdown");
    let metrics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("metrics.json")).expect("metrics"))
            .expect("JSON");
    assert_eq!(metrics["no_usb"], true);
    assert_eq!(metrics["playing"], false);
    assert_eq!(metrics["mode"], "ascii");
    for mode in 0..3 {
        assert!(
            metrics["mode_frames"][mode].as_u64().expect("frames") > 0,
            "mode {mode} was not rendered"
        );
    }
    assert_eq!(service::saved_mode(&directory), OutputMode::Ascii);
}
