// SPDX-License-Identifier: GPL-3.0-only
//! Native desktop notifications; service commands are independent of Win32.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use crate::service::{Command, OutputMode};
#[cfg(windows)]
pub use windows::{Instance, run};

pub fn report_error(error: &anyhow::Error) {
    #[cfg(windows)]
    {
        let title: Vec<u16> = "MCDU DOOM".encode_utf16().chain(Some(0)).collect();
        let message: Vec<u16> = format!("Не удалось запустить MCDU DOOM:\n{error:#}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: UTF-16 buffers are terminated and remain alive for the modal
        // call; the error dialog has no parent window and retains no pointers.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
    #[cfg(not(windows))]
    eprintln!("{error:#}");
}

#[cfg(windows)]
pub(super) fn menu_command(id: i32) -> Option<Command> {
    match id {
        1 => Some(Command::Start),
        2 => Some(Command::Home),
        3 => Some(Command::Quit),
        4 => Some(Command::Avionics),
        10 => Some(Command::Mode(OutputMode::Glyphs)),
        11 => Some(Command::Mode(OutputMode::Contrast)),
        12 => Some(Command::Mode(OutputMode::Ascii)),
        _ => None,
    }
}
