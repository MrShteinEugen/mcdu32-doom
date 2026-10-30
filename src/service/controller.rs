// SPDX-License-Identifier: GPL-3.0-only
use super::{
    Command, OutputMode,
    navigation::{Effect, Navigation, View},
};
use crate::input::Inputs;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub(super) struct Controller {
    pub(super) navigation: Navigation,
    pub(super) playing: bool,
    pub(super) mode: OutputMode,
    pub(super) held: u8,
}
impl Controller {
    pub(super) fn command(&mut self, command: Command, input: &Inputs, stop: &AtomicBool) {
        match command {
            Command::Start => {
                self.navigation.select(View::Doom);
                self.playing = true;
                input.host_action.store(1, Ordering::Release);
                input.suspended.store(false, Ordering::Relaxed);
            }
            Command::Home => {
                self.navigation.select(View::Home);
                self.pause(input);
            }
            Command::Mode(mode) => self.mode = mode,
            Command::Avionics => {
                self.navigation.aircraft();
                self.pause(input);
            }
            Command::Quit => stop.store(true, Ordering::Relaxed),
        }
    }
    fn pause(&mut self, input: &Inputs) {
        self.playing = false;
        input.suspended.store(true, Ordering::Relaxed);
        input.host_action.store(2, Ordering::Release);
    }
    pub(super) fn buttons(&mut self, input: &Inputs, stop: &AtomicBool) {
        let panel = input.panel_snapshot();
        if panel[0] == 1 {
            let effect = self
                .navigation
                .step(crate::mcdu::keyboard::held(&panel), self.navigation.time());
            match effect {
                Effect::StartDoom => self.command(Command::Start, input, stop),
                Effect::Home => self.command(Command::Home, input, stop),
                Effect::PauseDoom => self.pause(input),
                Effect::None => {}
            }
            return;
        }
        let raw = |code: u8| {
            input.raw[code as usize / 64].load(Ordering::Relaxed) & (1_u64 << (code % 64)) != 0
        };
        let state = input.state();
        let held = u8::from(state & ((1 << 5) | (1 << 9)) != 0 || raw(13) || raw(0xa2))
            | (u8::from(state & (1 << 10) != 0 || raw(27)) << 1);
        let down = held & !self.held;
        self.held = held;
        if self.playing && down & 2 != 0 {
            self.command(Command::Home, input, stop);
        } else if !self.playing && down & 1 != 0 {
            self.command(Command::Start, input, stop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_suspends_game_input_and_held_start_does_not_restart() {
        let input = Inputs::default();
        let stop = AtomicBool::new(false);
        let mut state = Controller::default();
        input.mcdu.store(1 << 5, Ordering::Relaxed);
        state.buttons(&input, &stop);
        assert!(state.playing);
        assert_eq!(input.host_action.load(Ordering::Relaxed), 1);
        input.mcdu.store((1 << 5) | (1 << 10), Ordering::Relaxed);
        state.buttons(&input, &stop);
        assert!(!state.playing);
        assert_eq!(input.key_bitmap(), [0; 4]);
        input.mcdu.store(1 << 5, Ordering::Relaxed);
        state.buttons(&input, &stop);
        assert!(!state.playing);
        input.mcdu.store(0, Ordering::Relaxed);
        state.buttons(&input, &stop);
        input.mcdu.store(1 << 5, Ordering::Relaxed);
        state.buttons(&input, &stop);
        assert!(state.playing);
    }
}
