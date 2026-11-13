# Running MCDU DOOM

Run these commands from the repository root in PowerShell 7. Building from
source requires Rust 1.85 or later, MSVC C++ Build Tools and the Windows SDK.
The Windows x64 package requires the Microsoft Visual C++ v14 x64 runtime
(`VCRUNTIME140.dll`). See packages.

```powershell
.\scripts\setup-freedoom.ps1
cargo build --release --locked --bins
.\target\release\mcdu-doom.exe --help
```

The default IWAD is `data/freedoom-0.13.0/freedoom1.wad`. Supply a different
local file with `--wad` in the CLI or `-Wad` in the launcher.

## Windows tray and physical panel

```powershell
.\run.ps1
.\run.ps1 -Style ascii
```

The launcher builds and starts `mcdu-doom-tray.exe`. The service opens the panel,
uploads fonts and displays the MCDU menu. Press L5 or select the tray command to
start DOOM. Press MENU to return to the MCDU pages. The tray also selects a
rendering mode, displays license information and stops the service.

Only one tray instance runs the service. Run `mcdu-doom-tray.exe` directly to use
an existing build, or use `mcdu-doom.exe` for CLI commands. Mode settings, status
and logs are stored in `output/tray/`. Check firmware and layout compatibility
with the [grid probe](grid-probe.md).

## Run without USB

```powershell
.\run.ps1 -NoTray -NoUsb -Seconds 3 -Capture 'output/service check'
cargo run --locked -- emulate --headless --benchmark --seconds 6 --capture output/emulator
cargo run --locked -- play --headless --seconds 3 --capture output/game
cargo run --locked -- compare --seconds 10
```

`serve --no-tray --no-usb` runs the service without opening a window or accessing
USB. `emulate` provides a virtual front panel. `compare` shows the original frame
alongside the glyph output; `play` runs the game without the virtual panel.
Use `--headless` to disable game windows, `--seconds` to set a time limit, and
`--autostart` to start the map in service mode. Close the window or press Ctrl+C
to stop a foreground run.

To send game frames to the physical display from the CLI:

```powershell
cargo run --locked -- play --mcdu --display full
```

## Game keyboard controls

| Action in `play` | Keys |
| --- | --- |
| Move forward / backward | Up / Down or W / S |
| Turn | Left / Right or A / D |
| Strafe | Q / E |
| Fire | Ctrl |
| Use / open door | Space |
| Run | Shift |
| Select weapon | 1–7 |
| Confirm / menu | Enter / Esc |
| Automap | Tab |
| Switch between color and contrast glyphs | F6 |

The emulator also accepts input through its on-screen panel buttons.

## Diagnostics

```powershell
cargo run --locked -- devices
cargo run --locked -- atlas --out output/atlas
.\probe-grid.ps1 -ProbeProfile Compact
```

`devices` lists HID collections. `atlas` and `probe` export files without opening
a device by default. Use `-Hardware` in the probe script or `--mcdu` in the CLI to
send the grid to the panel.

## Launcher options and completion

`run.ps1` and `probe-grid.ps1` accept `-Configuration debug` and `-SkipBuild`.
They resolve paths from their own directory and restore the working directory
on exit. `-NoTray` and `-NoUsb` apply to the service; `-PreviewOnly`, `-Compare`
and `-Headless` select game commands.

To enable PowerShell completion in the current session:

```powershell
. .\scripts\completions.ps1
```

Completion reads the command and option names from the executable's help output.
Use `--help` on any command to list its options.
