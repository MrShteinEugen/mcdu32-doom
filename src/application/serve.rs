// SPDX-License-Identifier: GPL-3.0-only
//! Thin desktop lifecycle around the independently testable service.
use super::args::ServeArgs;
#[cfg(windows)]
use crate::desktop::tray;
use crate::service::{self, Status};
use anyhow::{Context, Result};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(super) fn run(options: ServeArgs) -> Result<()> {
    #[cfg(windows)]
    let _instance = if !options.no_tray {
        let Some(instance) = tray::Instance::acquire()? else {
            return Ok(());
        };
        Some(instance)
    } else {
        None
    };
    let root = if options.wad.is_file() {
        std::env::current_dir()?
    } else {
        let executable = std::env::current_exe()?;
        executable
            .ancestors()
            .find(|path| path.join(&options.wad).is_file())
            .context("WAD not found; supply --wad")?
            .to_path_buf()
    };
    let wad = if options.wad.is_absolute() {
        options.wad.clone()
    } else {
        root.join(&options.wad)
    };
    let directory = root.join("output/tray");
    let mode = options
        .mode
        .unwrap_or_else(|| service::saved_mode(&directory));
    let status = Arc::new(Mutex::new(Status {
        mode,
        ..Status::default()
    }));
    let stop = Arc::new(AtomicBool::new(false));
    let shutdown = Arc::clone(&stop);
    if options.no_tray {
        ctrlc::set_handler(move || shutdown.store(true, Ordering::Relaxed))?;
    }
    let (sender, receiver) = service::channel();
    let worker = service::spawn(
        service::Options {
            wad,
            profile: options.display,
            mode,
            packet_us: options.packet_us,
            no_usb: options.no_usb,
            autostart: options.autostart,
            seconds: options.seconds,
            capture: options.capture,
            directory: directory.clone(),
        },
        receiver,
        Arc::clone(&status),
        Arc::clone(&stop),
    )?;
    let ui = if options.no_tray {
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    } else {
        #[cfg(windows)]
        {
            tray::run(sender, status, Arc::clone(&stop))
        }
        #[cfg(not(windows))]
        {
            let _ = (sender, status);
            Err(anyhow::anyhow!(
                "Windows tray is supported only on Windows; use --no-tray"
            ))
        }
    };
    stop.store(true, Ordering::Relaxed);
    let result = worker
        .join()
        .map_err(|_| anyhow::anyhow!("background service panicked"))?;
    if let Err(error) = &result {
        service::log(&directory, &format!("Service stopped: {error:#}"));
    }
    ui?;
    result
}
