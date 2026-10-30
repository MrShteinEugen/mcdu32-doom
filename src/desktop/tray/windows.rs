// SPDX-License-Identifier: GPL-3.0-only
//! Native notification-area icon. Every Win32 handle is owned by the UI thread.
use super::menu_command;
use crate::service::{Command, OutputMode, Status};
use anyhow::{Context, Result, ensure};
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    System::{Console::FreeConsole, LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{Shell::*, WindowsAndMessaging::*},
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
const CALLBACK: u32 = WM_APP + 1;

pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire() -> Result<Option<Self>> {
        // SAFETY: the UTF-16 name lives for the synchronous call; the returned
        // mutex handle is uniquely owned and closed by Drop.
        unsafe {
            let handle = CreateMutexW(null(), 0, wide("Local\\McduDoom.Tray").as_ptr());
            ensure!(
                !handle.is_null(),
                "create application mutex: {}",
                std::io::Error::last_os_error()
            );
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return Ok(None);
            }
            Ok(Some(Self(handle)))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: this object exclusively owns the valid mutex handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

struct State {
    commands: Sender<Command>,
    status: Arc<Mutex<Status>>,
    restart: u32,
    restore_tip: std::cell::Cell<bool>,
    icon: HICON,
}
struct Tray {
    window: HWND,
    icon: NOTIFYICONDATAW,
    state: Box<State>,
    owns_icon: bool,
}
impl Tray {
    fn new(commands: Sender<Command>, status: Arc<Mutex<Status>>) -> Result<Self> {
        // SAFETY: class and title are nul-terminated UTF-16 buffers; the hidden
        // window and callback run solely on this thread. Box<State> has a stable
        // address until Drop clears GWLP_USERDATA and destroys the window.
        unsafe {
            let module = GetModuleHandleW(null());
            let class = wide("McduDoomTrayWindow");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: module,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            ensure!(
                RegisterClassW(&wc) != 0,
                "register tray window: {}",
                std::io::Error::last_os_error()
            );
            let window = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("MCDU DOOM").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                module,
                null(),
            );
            ensure!(
                !window.is_null(),
                "create tray window: {}",
                std::io::Error::last_os_error()
            );
            let mut tray = Self {
                window,
                icon: std::mem::zeroed(),
                state: Box::new(State {
                    commands,
                    status,
                    restart: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                    restore_tip: std::cell::Cell::new(false),
                    icon: null_mut(),
                }),
                owns_icon: false,
            };
            SetWindowLongPtrW(
                window,
                GWLP_USERDATA,
                (&mut *tray.state as *mut State) as isize,
            );
            tray.icon.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            tray.icon.hWnd = window;
            tray.icon.uID = 1;
            tray.icon.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            tray.icon.uCallbackMessage = CALLBACK;
            let mut pixels = vec![0u8; 32 * 32 * 4];
            let pattern = [30u8, 17, 17, 17, 17, 17, 30];
            for y in 0..32 {
                for x in 0..32 {
                    let letter = (8..23).contains(&x)
                        && (5..26).contains(&y)
                        && pattern[(y - 5) / 3] & (1 << (4 - (x - 8) / 3)) != 0;
                    pixels[(y * 32 + x) * 4..(y * 32 + x + 1) * 4].copy_from_slice(if letter {
                        &[40, 180, 255, 255]
                    } else {
                        &[28, 20, 12, 255]
                    });
                }
            }
            let mask = [0u8; 128];
            tray.icon.hIcon = CreateIcon(module, 32, 32, 1, 32, mask.as_ptr(), pixels.as_ptr());
            tray.owns_icon = !tray.icon.hIcon.is_null();
            if !tray.owns_icon {
                tray.icon.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
            }
            tray.state.icon = tray.icon.hIcon;
            tray.tooltip();
            ensure!(
                Shell_NotifyIconW(NIM_ADD, &tray.icon) != 0,
                "create notification icon: {}",
                std::io::Error::last_os_error()
            );
            Ok(tray)
        }
    }
    fn tooltip(&mut self) -> bool {
        let state = self.state.status.lock().expect("status mutex");
        let text = format!(
            "MCDU DOOM · {} · {}",
            state.mode.label(),
            if state.connected {
                if state.playing {
                    "Игра"
                } else {
                    "Меню"
                }
            } else {
                "Ожидание MCDU"
            }
        );
        let mut tip = [0u16; 128];
        for (dest, value) in tip[..127].iter_mut().zip(text.encode_utf16()) {
            *dest = value;
        }
        let changed = self.icon.szTip != tip;
        self.icon.szTip = tip;
        changed
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: remove the owned icon, detach the stable State pointer before it
        // is freed, then destroy our window. Shared system icon is not destroyed.
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.icon);
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
            DestroyWindow(self.window);
            if self.owns_icon {
                DestroyIcon(self.icon.hIcon);
            }
        }
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: only this module installs GWLP_USERDATA. It remains a live State
    // on this UI thread until Tray::drop detaches it before destroying the window.
    unsafe {
        let state = GetWindowLongPtrW(window, GWLP_USERDATA) as *const State;
        if !state.is_null() {
            let state = &*state;
            if message == WM_COMMAND {
                if wparam & 0xffff == 5 {
                    let message = wide(
                        "MCDU DOOM: GPL-3.0-only.\nNo warranty. No aircraft/vendor endorsement.\nDOOM engine: original GPL-2.0-or-later notices retained.\nFont transaction map: Andrew Whewell, BSD-3-Clause.\nSee LICENSE, NOTICE.md and THIRD_PARTY.md in the source distribution.\nExact corresponding source must accompany a binary release as required by GPL.",
                    );
                    MessageBoxW(
                        window,
                        message.as_ptr(),
                        wide("Licenses and source").as_ptr(),
                        MB_OK | MB_ICONINFORMATION,
                    );
                    return 0;
                }
                if let Some(command) = menu_command((wparam & 0xffff) as i32) {
                    let _ = state.commands.send(command);
                }
                return 0;
            }
            if message == CALLBACK
                && [WM_RBUTTONUP, WM_LBUTTONUP, WM_CONTEXTMENU].contains(&(lparam as u32))
            {
                popup(window, state);
                return 0;
            }
            if message == state.restart && state.restart != 0 {
                // The outer loop restores icon fields on its next iteration.
                let mut icon: NOTIFYICONDATAW = std::mem::zeroed();
                icon.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                icon.hWnd = window;
                icon.uID = 1;
                icon.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
                icon.uCallbackMessage = CALLBACK;
                icon.hIcon = state.icon;
                Shell_NotifyIconW(NIM_ADD, &icon);
                state.restore_tip.set(true);
                return 0;
            }
            if message == WM_CLOSE || message == WM_ENDSESSION && wparam != 0 {
                let _ = state.commands.send(Command::Quit);
                return 0;
            }
            if message == WM_QUERYENDSESSION {
                return 1;
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}

fn popup(window: HWND, state: &State) {
    let info = state.status.lock().expect("status mutex").clone();
    // SAFETY: menu is created, used synchronously by this window's thread and
    // destroyed before returning. Win32 copies the temporary UTF-16 labels.
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let add = |flags, id, text: &str| AppendMenuW(menu, flags, id, wide(text).as_ptr());
        add(
            MF_STRING | MF_GRAYED,
            0,
            if info.connected {
                "MCDU подключена"
            } else {
                "Ожидание подключения MCDU"
            },
        );
        add(MF_SEPARATOR, 0, "");
        add(MF_STRING, 4, "Авиационное меню");
        add(
            MF_STRING | if info.connected { 0 } else { MF_GRAYED },
            1,
            "Начать DOOM",
        );
        add(
            MF_STRING | if info.connected { 0 } else { MF_GRAYED },
            2,
            "Игровое меню",
        );
        add(MF_SEPARATOR, 0, "");
        for (id, mode, text) in [
            (10, OutputMode::Glyphs, "Графика: глифы"),
            (11, OutputMode::Contrast, "Графика: контраст"),
            (12, OutputMode::Ascii, "Графика: ASCII"),
        ] {
            add(
                MF_STRING | if info.mode == mode { MF_CHECKED } else { 0 },
                id,
                text,
            );
        }
        add(MF_SEPARATOR, 0, "");
        add(MF_STRING, 5, "Лицензии и исходный код");
        add(MF_STRING, 3, "Выход");
        let mut point: POINT = std::mem::zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(window);
        let choice = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            window,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(window, WM_NULL, 0, 0);
        if choice != 0 {
            PostMessageW(window, WM_COMMAND, choice as usize, 0);
        }
    }
}

pub fn run(
    commands: Sender<Command>,
    status: Arc<Mutex<Status>>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let mut tray = Tray::new(commands, status).context("initialize Windows tray")?;
    // SAFETY: detach only this desktop-mode process from its inherited console;
    // CLI diagnostic subcommands keep their console and captured output.
    unsafe {
        FreeConsole();
    }
    while !stop.load(Ordering::Relaxed) {
        // SAFETY: MSG storage is initialized, PeekMessage writes within it, and
        // dispatches only on the thread that created our hidden window.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if tray.tooltip() || tray.state.restore_tip.replace(false) {
                Shell_NotifyIconW(NIM_MODIFY, &tray.icon);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}
