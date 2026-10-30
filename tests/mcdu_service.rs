// SPDX-License-Identifier: GPL-3.0-only
//! Real C engine plus synthetic physical panel reports, with no USB access.
use mcdu_doom::{
    display::Profile,
    input::Inputs,
    service::{self, Command, Options, OutputMode, Status, navigation::View},
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
fn wait(status: &Mutex<Status>, view: View, seconds: u64) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        if status.lock().expect("status").application == view {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "expected {view:?}, got {:?}",
            status.lock().expect("status")
        );
        thread::sleep(Duration::from_millis(20));
    }
}
fn press(input: &Inputs, byte: usize, mask: u8) {
    let mut report = [0; 14];
    report[0] = 1;
    report[byte] = mask;
    input.update_panel(&report);
    thread::sleep(Duration::from_millis(50));
    input.release_panel();
    thread::sleep(Duration::from_millis(30));
}
#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn physical_menu_routes_pages_and_disconnected_command_channel_stops_service() {
    let wad = Path::new("data/freedoom-0.13.0/freedoom1.wad");
    assert!(wad.is_file(), "install Freedoom before running this test");
    let directory = std::path::PathBuf::from("target/service-tests")
        .join(format!("mcdu-pages-test-{}", std::process::id()));
    let status = Arc::new(Mutex::new(Status::default()));
    let input = Arc::new(Inputs::default());
    let stop = Arc::new(AtomicBool::new(false));
    let _guard = mcdu_doom::doom::engine::stop_on_drop(&stop);
    let (commands, receiver) = service::channel();
    let worker = service::spawn_with_inputs(
        Options {
            wad: wad.into(),
            profile: Profile::Full,
            mode: OutputMode::Ascii,
            packet_us: 10000,
            no_usb: true,
            autostart: false,
            seconds: 25,
            capture: Some(directory.clone()),
            directory: directory.clone(),
        },
        receiver,
        Arc::clone(&status),
        Arc::clone(&stop),
        Arc::clone(&input),
    )
    .expect("service");
    // Wait for setup, rather than mistaking the initial status for a ready session.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !status.lock().expect("status").connected {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(20));
    }
    press(&input, 1, 0x10); // L5: visible red DOOM item on the startup menu.
    wait(&status, View::Doom, 5);
    press(&input, 4, 1);
    wait(&status, View::Avionics, 5);
    press(&input, 2, 0x80); // INIT, independently of its former weapon binding.
    press(&input, 4, 1);
    commands.send(Command::Home).expect("home");
    wait(&status, View::Home, 5);
    commands.send(Command::Start).expect("start again");
    wait(&status, View::Doom, 5);
    press(&input, 4, 1);
    wait(&status, View::Avionics, 5);
    drop(commands);
    let deadline = Instant::now() + Duration::from_secs(8);
    while !worker.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        worker.is_finished(),
        "service did not stop when command senders were dropped"
    );
    worker.join().expect("worker").expect("shutdown");
    assert!(stop.load(Ordering::Relaxed));
    assert!(!input.navigation_owned.load(Ordering::Relaxed));
    assert!(input.suspended.load(Ordering::Relaxed));
    assert!(!status.lock().expect("status").connected);
    assert!(!status.lock().expect("status").playing);
    let metrics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("metrics.json")).expect("metrics"))
            .expect("JSON");
    assert_eq!(metrics["application"], "avionics");
    assert_eq!(metrics["playing"], false);
}
