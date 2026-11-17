// SPDX-License-Identifier: GPL-3.0-only
use std::{path::Path, process::Command};

#[test]
fn executable_help_version_and_argument_errors_need_no_wad_or_usb() {
    let binary = env!("CARGO_BIN_EXE_mcdu-doom");
    for subcommand in [
        None,
        Some("serve"),
        Some("play"),
        Some("emulate"),
        Some("compare"),
        Some("devices"),
        Some("atlas"),
        Some("probe"),
    ] {
        let mut command = Command::new(binary);
        if let Some(subcommand) = subcommand {
            command.arg(subcommand);
        }
        let output = command.arg("--help").output().expect("help");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .expect("version");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
    for args in [
        vec!["blocks"],
        vec!["serve", "--mode", "blocks"],
        vec!["play", "--original", "--mcdu"],
        vec!["serve", "--packet-us", "0"],
    ] {
        let output = Command::new(binary)
            .args(args)
            .output()
            .expect("invalid command");
        assert_eq!(output.status.code(), Some(2));
    }
    let output = Command::new(binary)
        .args([
            "serve",
            "--no-tray",
            "--no-usb",
            "--wad",
            "target/cli-tests/missing.wad",
        ])
        .output()
        .expect("missing WAD");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("WAD"));
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn console_service_stops_and_captures_aircraft_page_without_usb() {
    assert!(Path::new("data/freedoom-0.13.0/freedoom1.wad").is_file());
    let directory = std::path::PathBuf::from("target/cli-tests")
        .join(format!("console-service-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args([
            "serve",
            "--no-tray",
            "--no-usb",
            "--seconds",
            "2",
            "--capture",
        ])
        .arg(&directory)
        .output()
        .expect("console service");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metrics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("metrics.json")).expect("metrics"))
            .expect("JSON");
    assert_eq!(metrics["application"], "avionics");
    assert_eq!(metrics["no_usb"], true);
    assert_eq!(metrics["playing"], false);
    assert_eq!(metrics["usb"]["usb_reports"], 0);
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn service_reports_capture_failure_when_usb_is_disabled() {
    assert!(Path::new("data/freedoom-0.13.0/freedoom1.wad").is_file());
    let directory = std::path::PathBuf::from("target/cli-tests")
        .join(format!("invalid-capture-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let capture = directory.join("existing-file");
    std::fs::write(&capture, b"this is a file").expect("capture fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args([
            "serve",
            "--no-tray",
            "--no-usb",
            "--seconds",
            "1",
            "--capture",
        ])
        .arg(&capture)
        .output()
        .expect("console service");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("create capture directory"), "{stderr}");
}

#[test]
fn static_probe_exports_the_requested_wire_stream_without_hardware() {
    let directory = std::path::PathBuf::from("target/cli-tests")
        .join(format!("mcdu-probe-test-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args([
            "probe",
            "--packing",
            "tight",
            "--layout",
            "begin",
            "--grid-order",
            "both",
            "--out",
        ])
        .arg(&directory)
        .output()
        .expect("run static probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).expect("manifest"))
            .expect("JSON");
    assert_eq!(manifest["usb_written"], false);
    assert_eq!(manifest["hardware_requested"], false);
    assert_eq!(manifest["hardware_visual_confirmed"], false);
    assert_eq!(manifest["banks"][0]["record_bytes"], 32);
    assert_eq!(manifest["banks"][1]["id"], 6);
    let wire = std::fs::read(directory.join("reports.bin")).expect("binary reports");
    assert_eq!(
        wire.len(),
        manifest["reports"].as_u64().expect("count") as usize * 64
    );
    let screen: Vec<_> = wire
        .chunks_exact(64)
        .filter(|p| p[0] == 0xf2)
        .flat_map(|p| p[1..].iter().copied())
        .collect();
    assert_eq!(screen.len(), 1344 * 3);
    assert_eq!(&screen[screen.len() - 3..], &[0x33, 2, b'#']);
    assert!(
        std::fs::read(directory.join("expected.png"))
            .expect("PNG")
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}

#[test]
fn probe_rejects_oversized_hardware_grid_before_device_enumeration() {
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args([
            "probe",
            "--mcdu",
            "--glyph-width",
            "23",
            "--glyph-height",
            "29",
        ])
        .output()
        .expect("run invalid probe");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not fit"));
}

#[test]
fn grid_only_probe_does_not_upload_font_banks() {
    let directory = std::path::PathBuf::from("target/cli-tests")
        .join(format!("mcdu-grid-only-test-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args(["probe", "--skip-font", "--out"])
        .arg(&directory)
        .output()
        .expect("run grid-only probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wire = std::fs::read(directory.join("reports.bin")).expect("reports");
    assert_eq!(wire.len(), (4 + 64) * 64);
    assert!(
        !wire
            .chunks_exact(64)
            .any(|p| p[0] == 0xf0 && p[8..12] == [6, 1, 0, 0])
    );
}

#[test]
fn invalid_fps_is_rejected_before_usb_access() {
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args(["play", "--mcdu", "--fps", "0"])
        .output()
        .expect("run CLI");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid value"));
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn real_freedoom_frame_and_clean_shutdown_when_data_is_available() {
    assert!(
        Path::new("data/freedoom-0.13.0/freedoom1.wad").is_file(),
        "install Freedoom first"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args(["play", "--headless", "--seconds", "1"])
        .output()
        .expect("run engine");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"active\": true"),
        "game did not reach its first map"
    );
    assert!(
        stdout.contains("\"converted_frames\""),
        "host did not finish after C shutdown"
    );
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn full_screen_gameplay_captures_the_confirmed_surface_without_usb() {
    assert!(
        Path::new("data/freedoom-0.13.0/freedoom1.wad").is_file(),
        "install Freedoom first"
    );
    let directory = std::path::PathBuf::from("target/cli-tests")
        .join(format!("mcdu-full-game-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        .args([
            "play",
            "--display",
            "full",
            "--headless",
            "--seconds",
            "2",
            "--capture",
        ])
        .arg(&directory)
        .output()
        .expect("run full-screen game");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metrics: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("metrics.json")).expect("metrics"))
            .expect("JSON");
    assert_eq!(metrics["columns"], 53);
    assert_eq!(metrics["rows"], 31);
    assert_eq!(metrics["display_width"], 583);
    assert_eq!(metrics["display_height"], 434);
    assert_eq!(metrics["usb_frames"], 0);
    assert_eq!(metrics["active"], true);
    let image = std::fs::read(directory.join("mcdu.png")).expect("PNG");
    assert_eq!(
        u32::from_be_bytes(image[16..20].try_into().expect("width")),
        583
    );
    assert_eq!(
        u32::from_be_bytes(image[20..24].try_into().expect("height")),
        434
    );
}

#[test]
#[ignore = "requires Freedoom 0.13.0; run scripts/setup-freedoom.ps1 first"]
fn dense_emulator_presents_frames_and_fires_via_mcdu_report() {
    assert!(
        Path::new("data/freedoom-0.13.0/freedoom1.wad").is_file(),
        "install Freedoom first"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_mcdu-doom"))
        // Debug builds spend substantially longer matching 1,344 glyph cells.
        .args([
            "emulate",
            "--columns",
            "48",
            "--rows",
            "28",
            "--headless",
            "--benchmark",
            "--seconds",
            "6",
        ])
        .output()
        .expect("run emulator");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let start = stdout.find("{\n").expect("emulator metrics");
    let metrics: serde_json::Value = serde_json::from_str(&stdout[start..]).expect("metrics JSON");
    assert_eq!(metrics["usb_access"], false);
    assert_eq!(metrics["columns"], 48);
    assert_eq!(metrics["rows"], 28);
    assert!(metrics["presented_frames"].as_u64().expect("presented") > 0);
    assert!(
        metrics["ammo"].as_i64().expect("ammo") < 50,
        "R1 report did not fire the weapon"
    );
}
