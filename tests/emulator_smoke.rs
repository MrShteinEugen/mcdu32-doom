// SPDX-License-Identifier: GPL-3.0-only
//! End-to-end device-free run, isolated from other single-instance engine tests.
use mcdu_doom::{
    config::{DEFAULT_FPS, DEFAULT_PACKET_US, DEFAULT_WAD},
    desktop::emulator::{self, Options},
    display::render::Style,
};
use std::{fs::File, io::BufReader, path::PathBuf};

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn headless_emulator_presents_frames_fires_and_writes_valid_captures() {
    let directory = PathBuf::from("target/emulator-smoke").join(std::process::id().to_string());
    emulator::run(Options {
        style: Style::Contrast,
        comparison: false,
        wad: DEFAULT_WAD.into(),
        cols: 48,
        rows: 28,
        fps: DEFAULT_FPS,
        packet_us: DEFAULT_PACKET_US,
        headless: true,
        seconds: 6,
        capture: Some(directory.clone()),
        benchmark: true,
    })
    .expect("headless emulator run");
    let metrics: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory.join("metrics.json")).expect("metrics file"),
    )
    .expect("metrics JSON");
    assert_eq!(metrics["usb_access"], false);
    assert_eq!(metrics["columns"], 48);
    assert_eq!(metrics["rows"], 28);
    assert_eq!(metrics["style"], "contrast");
    assert!(
        metrics["presented_frames"]
            .as_u64()
            .expect("presented frames")
            > 0
    );
    assert!(
        metrics["simulated_reports"]
            .as_u64()
            .expect("simulated reports")
            > 0
    );
    assert!(
        metrics["ammo"].as_i64().expect("ammo") < 50,
        "panel report did not fire"
    );
    let text: Vec<String> =
        serde_json::from_slice(&std::fs::read(directory.join("text.json")).expect("text capture"))
            .expect("text JSON");
    assert!(text.len() <= 32);
    for (name, dimensions) in [
        ("emulator.png", (760, 1000)),
        ("mcdu.png", (528, 392)),
        ("source.png", (320, 200)),
        ("color.png", (528, 392)),
        ("contrast.png", (528, 392)),
        ("style-comparison.png", (1067, 392)),
        ("glyphs-enhanced.png", (528, 392)),
        ("glyphs-baseline.png", (528, 392)),
        ("sprite-mask.png", (320, 200)),
    ] {
        let path = directory.join(name);
        let decoder = png::Decoder::new(BufReader::new(File::open(&path).expect("PNG file")));
        let mut reader = decoder.read_info().expect("PNG header");
        let mut buffer = vec![0; reader.output_buffer_size().expect("PNG buffer size")];
        let frame = reader.next_frame(&mut buffer).expect("PNG image data");
        assert_eq!(
            (frame.width, frame.height),
            dimensions,
            "{}",
            path.display()
        );
        reader.finish().expect("PNG end chunk");
    }
}
