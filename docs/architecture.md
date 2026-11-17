# Architecture

Both binaries, `src/main.rs` and `src/bin/mcdu-doom-tray.rs`, call
`application::entry`. The application layer parses arguments and selects a mode.
`src/lib.rs` exposes the library modules.

| Module | Responsibility |
| --- | --- |
| `application` | CLI, command dispatch, game loop and atlas export |
| `config` | Shared defaults |
| `desktop` | Emulator, keyboard, front panel and Win32 tray |
| `device` | Protocol, font banks, virtual display and USB HID |
| `diagnostics` | Static grid probe and diagnostic font |
| `display` | Geometry, glyphs, text, HUD and rendering |
| `doom` | IWAD handling, C/Rust interface and engine thread |
| `input` | Key state and MCDU report decoding |
| `mcdu` | Page state, text entry and rendering |
| `output` | PNG export, frame capture and metrics |
| `runtime` | Latest-frame exchange |
| `service` | MCDU/DOOM navigation, commands, USB session and shutdown |

## Service and display

The service switches among `Home`, `Doom` and `Avionics`. It owns the engine
thread and USB session. Only the engine thread calls the C API; the CLI and tray
send commands. `Latest` keeps one frame, so a slow USB connection does not queue
stale frames. `DynamicFonts` supplies the game and aircraft atlases and display
geometry to the transport. The transport does not depend on MCDU page logic.

On shutdown, the service signals its workers, joins them, releases input and
clears its status. Closing the command channel also stops the service. A USB
discovery or I/O error returns the service to its connection wait loop. Errors
in rendering, preferences or captures end the service and reach the CLI or tray.
RAII releases input if startup fails.

## Engine and rendering

The C engine has process-wide state and supports one instance per process.
Integration tests use separate test executables or launch the CLI as a child
process. The game, emulator, probe and capture tools share rendering and output
primitives. Contract tests cover the wire format and fixed frames.

MCDU pages retain local input. They do not connect to a simulator, an AIRAC
database or aircraft flight systems.
