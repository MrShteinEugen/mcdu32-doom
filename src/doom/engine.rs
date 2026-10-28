// SPDX-License-Identifier: GPL-3.0-only
use crate::{doom::hud::HudState, input::Inputs};
use anyhow::{Context, Result, ensure};
use std::{
    cell::RefCell,
    collections::VecDeque,
    ffi::{CString, c_char, c_int},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const DOOM_W: usize = 320;
pub const DOOM_H: usize = 200;
static ENGINE_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct Frame {
    pub pixels: Vec<u32>,
    /// Visible sprite ownership: 0 scenery, 1 live monster, 2 player weapon.
    pub semantics: Vec<u8>,
    pub hud: HudState,
    pub text: crate::doom::text::TextFrame,
    pub viewport: [usize; 2],
    pub generation: u64,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            pixels: vec![0; DOOM_W * DOOM_H],
            semantics: vec![0; DOOM_W * DOOM_H],
            hud: HudState::default(),
            text: crate::doom::text::TextFrame::default(),
            viewport: [0; 2],
            generation: 0,
        }
    }
}

impl Frame {
    /// Copy a newly published frame while reusing the framebuffer allocations.
    pub fn update_from(&mut self, latest: &Self) -> bool {
        if self.generation == latest.generation {
            return false;
        }
        self.pixels.clone_from(&latest.pixels);
        self.semantics.clone_from(&latest.semantics);
        self.hud = latest.hud;
        self.text = latest.text;
        self.viewport = latest.viewport;
        self.generation = latest.generation;
        true
    }
}

/// Ensure startup errors also stop already-created engine/USB workers.
pub fn stop_on_drop(stop: &Arc<AtomicBool>) -> impl Drop + use<> {
    struct StopOnDrop(Arc<AtomicBool>);
    impl Drop for StopOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    StopOnDrop(Arc::clone(stop))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_abi_sizes_alignment_and_offsets_match_the_bridge_header() {
        use crate::doom::text::{TextFrame, TextLine};
        use std::mem::{align_of, offset_of, size_of};
        assert_eq!(size_of::<HudState>(), 68);
        assert_eq!(align_of::<HudState>(), 4);
        assert_eq!(offset_of!(HudState, ammo_counts), 36);
        assert_eq!(offset_of!(HudState, ammo_max), 52);
        assert_eq!(size_of::<TextLine>(), 140);
        assert_eq!(offset_of!(TextLine, bytes), 12);
        assert_eq!(size_of::<TextFrame>(), 4488);
        assert_eq!(align_of::<TextFrame>(), 4);
        assert_eq!(offset_of!(TextFrame, lines), 8);
    }
    #[test]
    fn frame_updates_preserve_buffers_and_copy_ui_metadata() {
        let mut latest = Frame {
            generation: 1,
            viewport: [320, 168],
            ..Frame::default()
        };
        latest.pixels[0] = 0xffffff;
        latest.semantics[0] = 1;
        latest.hud.ammo = 42;
        latest.text.count = 1;
        let mut frame = Frame::default();
        let pixels = frame.pixels.as_ptr();
        assert!(frame.update_from(&latest));
        assert_eq!(frame.pixels.as_ptr(), pixels);
        assert_eq!(frame.pixels[0], 0xffffff);
        assert_eq!(frame.semantics[0], 1);
        assert_eq!(frame.hud.ammo, 42);
        assert_eq!(frame.text.count, 1);
        assert_eq!(frame.viewport, [320, 168]);
        assert!(!frame.update_from(&latest));
    }
    #[test]
    fn startup_guard_requests_shutdown_when_scope_exits() {
        let stop = Arc::new(AtomicBool::new(false));
        {
            let _guard = stop_on_drop(&stop);
        }
        assert!(stop.load(Ordering::Relaxed));
    }
}

struct ContextState {
    started: Instant,
    frame: Arc<Mutex<Frame>>,
    input: Arc<Inputs>,
    prior: [u64; 4],
    events: VecDeque<(bool, u8)>,
}
thread_local! {static CONTEXT:RefCell<Option<ContextState>>=const {RefCell::new(None)};}

unsafe extern "C" {
    fn MCDU_Create(argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn MCDU_Tick() -> c_int;
    fn MCDU_Shutdown() -> c_int;
    fn MCDU_GetHud(hud: *mut HudState);
    fn MCDU_GetMask() -> *const u8;
    fn MCDU_GetText(text: *mut crate::doom::text::TextFrame);
    fn MCDU_HostAction(action: c_int);
    static DG_ScreenBuffer: *const u32;
    static viewwidth: c_int;
    static viewheight: c_int;
}

/// The original C engine is single-instance. All its calls stay on one dedicated thread.
pub fn spawn(
    wad: &Path,
    warp: bool,
    frame: Arc<Mutex<Frame>>,
    input: Arc<Inputs>,
    stop: Arc<AtomicBool>,
) -> Result<JoinHandle<Result<()>>> {
    ensure!(wad.is_file(), "WAD does not exist: {}", wad.display());
    let path = wad.canonicalize().context("resolve WAD path")?;
    // The C runtime uses narrow paths. Use a normal path rather than Win32's extended prefix.
    let path = path
        .to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .to_owned();
    let mut strings = vec![
        CString::new("mcdu-doom")?,
        CString::new("-iwad")?,
        CString::new(path)?,
        CString::new("-nosound")?,
    ];
    if warp {
        strings.push(CString::new("-warp")?);
        strings.push(CString::new("1")?);
        strings.push(CString::new("1")?);
    }
    strings.push(CString::new("-nogui")?);
    ensure!(
        ENGINE_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok(),
        "the original DOOM engine supports one instance per process"
    );
    thread::Builder::new()
        .name("doom-engine".into())
        .spawn(move || {
            CONTEXT.with(|state| {
                *state.borrow_mut() = Some(ContextState {
                    started: Instant::now(),
                    frame,
                    input,
                    prior: [0; 4],
                    events: VecDeque::with_capacity(64),
                })
            });
            let mut argv: Vec<_> = strings.iter_mut().map(|s| s.as_ptr().cast_mut()).collect();
            let argc = argv.len() as c_int;
            argv.push(std::ptr::null_mut());
            // SAFETY: CString storage and the null-terminated argv array remain alive for
            // the entire loop; this is the sole thread accessing the C engine's globals.
            let mut status = unsafe { MCDU_Create(argc, argv.as_mut_ptr()) };
            while !stop.load(Ordering::Relaxed) {
                if status != 0 {
                    break;
                }
                let action = CONTEXT.with(|state| {
                    state
                        .borrow()
                        .as_ref()
                        .map_or(0, |c| c.input.host_action.swap(0, Ordering::AcqRel))
                });
                if action != 0 {
                    // SAFETY: host commands are handled on the sole initialized engine
                    // thread, before entering the next tick; no C data is shared.
                    unsafe {
                        MCDU_HostAction(action as c_int);
                    }
                }
                // SAFETY: initialized above, and no other thread enters the engine.
                status = unsafe { MCDU_Tick() };
            }
            if status == 0 {
                // SAFETY: shutdown runs on the same initialized engine thread.
                status = unsafe { MCDU_Shutdown() };
            }
            stop.store(true, Ordering::Relaxed);
            CONTEXT.with(|state| *state.borrow_mut() = None);
            ensure!(
                status == 1,
                "DOOM engine stopped with an error; see engine diagnostics above"
            );
            Ok(())
        })
        .context("start DOOM engine")
}

// Export names match DoomGeneric's C declarations; Rust names follow snake_case.
#[unsafe(export_name = "DG_Init")]
extern "C" fn dg_init() {}

#[unsafe(export_name = "DG_DrawFrame")]
extern "C" fn dg_draw_frame() {
    CONTEXT.with(|state| {
        if let Some(context) = state.borrow().as_ref() {
            // SAFETY: the C engine allocated a 320x200 u32 buffer before callbacks;
            // it is read only on the same thread and not retained beyond this call.
            let pixels = unsafe {
                if DG_ScreenBuffer.is_null() {
                    return;
                }
                std::slice::from_raw_parts(DG_ScreenBuffer, DOOM_W * DOOM_H)
            };
            let mut hud = HudState::default();
            // SAFETY: HudState is repr(C), matching McduHud's scalars and arrays.
            unsafe {
                MCDU_GetHud(&mut hud);
            }
            let mut frame = context.frame.lock().expect("engine frame mutex poisoned");
            frame.pixels.copy_from_slice(pixels);
            if hud.active != 0 {
                // SAFETY: the C mask is a static 320x200 byte array, updated only
                // on this engine thread; copying completes before the next tick.
                let mask = unsafe { std::slice::from_raw_parts(MCDU_GetMask(), DOOM_W * DOOM_H) };
                frame.semantics.copy_from_slice(mask);
            } else {
                frame.semantics.fill(0);
            }
            frame.hud = hud;
            // SAFETY: TextFrame is repr(C) with the same bounded arrays as McduText;
            // the engine thread owns the source and the frame is locked for writing.
            unsafe {
                MCDU_GetText(&mut frame.text);
                frame.viewport = [viewwidth.max(0) as usize, viewheight.max(0) as usize];
            }
            frame.generation += 1;
        }
    });
}

#[unsafe(export_name = "DG_GetTicksMs")]
extern "C" fn dg_get_ticks_ms() -> u32 {
    CONTEXT.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(0, |c| c.started.elapsed().as_millis() as u32)
    })
}

#[unsafe(export_name = "DG_SleepMs")]
extern "C" fn dg_sleep_ms(ms: u32) {
    thread::sleep(Duration::from_millis(u64::from(ms)));
}

#[unsafe(export_name = "DG_SetWindowTitle")]
extern "C" fn dg_set_window_title(_title: *const c_char) {}

#[unsafe(export_name = "DG_GetKey")]
extern "C" fn dg_get_key(pressed: *mut c_int, key: *mut u8) -> c_int {
    if pressed.is_null() || key.is_null() {
        return 0;
    }
    CONTEXT.with(|state| {
        let mut state = state.borrow_mut();
        let Some(context) = state.as_mut() else {
            return 0;
        };
        let current = context.input.key_bitmap();
        for (bank, &held) in current.iter().enumerate() {
            let mut changed = held ^ context.prior[bank];
            while changed != 0 {
                let bit = changed.trailing_zeros();
                context
                    .events
                    .push_back((held & (1_u64 << bit) != 0, (bank * 64 + bit as usize) as u8));
                changed &= changed - 1;
            }
        }
        context.prior = current;
        if let Some((down, code)) = context.events.pop_front() {
            // SAFETY: the C caller passes writable int and unsigned-char pointers.
            // They are checked non-null and are written only during this callback.
            unsafe {
                *pressed = c_int::from(down);
                *key = code;
            }
            1
        } else {
            0
        }
    })
}
