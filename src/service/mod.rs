// SPDX-License-Identifier: GPL-3.0-only
//! Background MCDU session. UI commands never access the C engine or HID handle.
use crate::{
    device::transport::ConnectionError,
    display::{Profile, ascii, atlas::Atlas},
    doom::engine::{self, Frame},
    input::Inputs,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};
pub mod navigation;
use navigation::View;
mod controller;
mod lifecycle;
mod screens;
mod session;
use controller::Controller;
pub use screens::home;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    Glyphs,
    #[default]
    Contrast,
    Ascii,
}
impl OutputMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Glyphs => "Глифы",
            Self::Contrast => "Контраст",
            Self::Ascii => "DOOM ASCII",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Start,
    Home,
    Avionics,
    Mode(OutputMode),
    Quit,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub application: View,
    pub connected: bool,
    pub playing: bool,
    pub mode: OutputMode,
    pub detail: String,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            application: View::Home,
            connected: false,
            playing: false,
            mode: OutputMode::Contrast,
            detail: "Подключение MCDU…".into(),
        }
    }
}

pub struct Options {
    pub wad: PathBuf,
    pub profile: Profile,
    pub mode: OutputMode,
    pub packet_us: u64,
    pub no_usb: bool,
    pub autostart: bool,
    pub seconds: u64,
    pub capture: Option<PathBuf>,
    pub directory: PathBuf,
}

pub fn log(directory: &std::path::Path, message: &str) {
    if std::fs::create_dir_all(directory).is_ok() {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("service.log"))
        {
            let _ = writeln!(file, "{message}");
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Preferences {
    mode: OutputMode,
}
pub fn saved_mode(directory: &std::path::Path) -> OutputMode {
    std::fs::read(directory.join("preferences.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Preferences>(&bytes).ok())
        .map_or(OutputMode::default(), |p| p.mode)
}

pub fn spawn(
    options: Options,
    commands: Receiver<Command>,
    status: Arc<Mutex<Status>>,
    stop: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<Result<()>>> {
    spawn_with_inputs(options, commands, status, stop, Arc::new(Inputs::default()))
}

/// Injectable panel input supports deterministic service testing without USB.
pub fn spawn_with_inputs(
    options: Options,
    commands: Receiver<Command>,
    status: Arc<Mutex<Status>>,
    stop: Arc<AtomicBool>,
    input: Arc<Inputs>,
) -> Result<thread::JoinHandle<Result<()>>> {
    // Validate user data before creating background workers or touching USB.
    let wad = crate::doom::wad::Wad::open(&options.wad)?;
    let mut atlas = Atlas::from_wad(&wad);
    ascii::install(&mut atlas);
    thread::Builder::new()
        .name("mcdu-service".into())
        .spawn(move || {
            let _stop_guard = engine::stop_on_drop(&stop);
            let _input_session = lifecycle::InputSession::acquire(input.clone());
            input.host_action.store(2, Ordering::Release);
            let shared = Arc::new(Mutex::new(Frame::default()));
            let game = engine::spawn(
                &options.wad,
                true,
                Arc::clone(&shared),
                Arc::clone(&input),
                Arc::clone(&stop),
            )?;
            let mut controller = Controller {
                mode: options.mode,
                ..Controller::default()
            };
            controller.navigation.aircraft();
            let started = Instant::now();
            let result = (|| -> Result<()> {
                while !stop.load(Ordering::Relaxed) {
                    if options.seconds > 0
                        && started.elapsed() >= Duration::from_secs(options.seconds)
                    {
                        break;
                    }
                    let session = session::Session {
                        options: &options,
                        commands: &commands,
                        status: &status,
                        stop: &stop,
                        input: &input,
                        shared: &shared,
                        atlas: &atlas,
                        started,
                    };
                    match session.run(&mut controller) {
                        Ok(()) => break,
                        Err(error) if error.is::<ConnectionError>() => {
                            log(&options.directory, &format!("MCDU session: {error:#}"));
                            controller.command(Command::Avionics, &input, &stop);
                            input.release_panel();
                            *status.lock().expect("status mutex") = Status {
                                application: controller.navigation.view,
                                connected: false,
                                playing: false,
                                mode: controller.mode,
                                detail: format!("MCDU недоступна: {error}"),
                            };
                            // Keep the tray responsive while waiting for a reconnect.
                            let retry = Instant::now() + Duration::from_secs(2);
                            while !stop.load(Ordering::Relaxed) && Instant::now() < retry {
                                for command in pending_commands(&commands) {
                                    if let Command::Mode(mode) = command {
                                        controller.mode = mode;
                                    } else if matches!(command, Command::Quit) {
                                        stop.store(true, Ordering::Relaxed);
                                    }
                                }
                                if options.seconds > 0
                                    && started.elapsed() >= Duration::from_secs(options.seconds)
                                {
                                    return Ok(());
                                }
                                thread::sleep(Duration::from_millis(20));
                            }
                        }
                        Err(error) => return Err(error),
                    }
                }
                Ok(())
            })();
            stop.store(true, Ordering::Relaxed);
            input.suspended.store(true, Ordering::Relaxed);
            let game_result = game
                .join()
                .map_err(|_| anyhow::anyhow!("engine thread panicked"))
                .and_then(|result| result);
            let result = result.and(game_result);
            let mut state = status.lock().expect("status mutex");
            state.connected = false;
            state.playing = false;
            state.detail = match &result {
                Ok(()) => "Сервис остановлен".into(),
                Err(error) => format!("Сервис остановлен: {error:#}"),
            };
            result
        })
        .context("start background MCDU service")
}

pub fn channel() -> (Sender<Command>, Receiver<Command>) {
    std::sync::mpsc::channel()
}

fn pending_commands(commands: &Receiver<Command>) -> Vec<Command> {
    let mut result = Vec::new();
    loop {
        match commands.try_recv() {
            Ok(command) => result.push(command),
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                result.push(Command::Quit);
                break;
            }
        }
    }
    result
}
