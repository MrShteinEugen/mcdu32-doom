// SPDX-License-Identifier: GPL-3.0-only
//! Context routing and physical key edges for MCDU and DOOM.
use crate::mcdu::{Action, Avionics, keyboard::Key, page_key};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum View {
    #[default]
    Home,
    Doom,
    Avionics,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    StartDoom,
    PauseDoom,
    Home,
}
pub struct Navigation {
    pub view: View,
    pub aviation: Avionics,
    pub generation: u64,
    previous: Vec<Key>,
    began: Instant,
    repeat: Option<(Key, Duration, Duration)>,
}
impl Default for Navigation {
    fn default() -> Self {
        Self {
            view: View::Home,
            aviation: Avionics::default(),
            generation: 0,
            previous: Vec::new(),
            began: Instant::now(),
            repeat: None,
        }
    }
}
impl Navigation {
    pub fn time(&self) -> Duration {
        self.began.elapsed()
    }
    pub fn select(&mut self, view: View) {
        self.view = view;
        self.generation += 1;
    }
    pub fn aircraft(&mut self) {
        self.aviation.select(crate::mcdu::Page::Menu);
        self.select(View::Avionics);
    }
    pub fn render(&self, cols: usize, rows: usize) -> Vec<crate::display::Cell> {
        match self.view {
            View::Avionics => self
                .aviation
                .render(crate::mcdu::display::COLUMNS, crate::mcdu::display::ROWS),
            _ => super::screens::home(cols, rows),
        }
    }
    pub fn font_profile(&self) -> crate::device::transport::FontProfile {
        if self.view == View::Avionics {
            crate::device::transport::FontProfile::Aircraft
        } else {
            crate::device::transport::FontProfile::Game
        }
    }
    pub fn mcdu_fm(&self) -> bool {
        self.view == View::Avionics && self.aviation.page == crate::mcdu::Page::Menu
    }
    pub fn step(&mut self, held: Vec<Key>, now: Duration) -> Effect {
        let pressed: Vec<_> = held
            .iter()
            .filter(|key| !self.previous.contains(key))
            .copied()
            .collect();
        self.previous = held.clone();
        if pressed.contains(&Key::Menu) {
            self.aircraft();
            return Effect::PauseDoom;
        }
        for &key in &pressed {
            if key != Key::Dir || self.view == View::Avionics {
                if let Some(page) = page_key(key) {
                    self.aviation.select(page);
                    self.select(View::Avionics);
                    return Effect::PauseDoom;
                }
            }
        }
        match self.view {
            View::Home => {
                if pressed.iter().any(|key| {
                    matches!(
                        key,
                        Key::Dir
                            | Key::Character('D' | 'A')
                            | Key::Lsk {
                                right: false,
                                row: 0
                            }
                    )
                }) {
                    self.select(View::Doom);
                    return Effect::StartDoom;
                }
            }
            View::Doom => {
                if pressed.iter().any(|key| {
                    matches!(
                        key,
                        Key::Character('E')
                            | Key::Lsk {
                                right: false,
                                row: 5
                            }
                    )
                }) {
                    self.select(View::Home);
                    return Effect::Home;
                }
            }
            View::Avionics => {
                for key in pressed.iter().copied() {
                    if self.aviation.handle(key) == Action::StartDoom {
                        self.select(View::Doom);
                        return Effect::StartDoom;
                    }
                    self.generation += 1;
                    if matches!(key, Key::Character(_) | Key::Clear) {
                        self.repeat = Some((key, now, now + Duration::from_millis(450)));
                    }
                }
                if let Some((key, began, next)) = self.repeat {
                    if !held.contains(&key) {
                        self.repeat = None;
                    } else if now >= next {
                        if key == Key::Clear && now.saturating_sub(began) >= Duration::from_secs(1)
                        {
                            self.aviation.clear_scratch();
                        } else {
                            self.aviation.handle(key);
                        }
                        self.generation += 1;
                        self.repeat = Some((key, began, now + Duration::from_millis(120)));
                    }
                }
            }
        }
        Effect::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcdu_menu_and_functions_route_while_letters_stay_text() {
        let mut nav = Navigation::default();
        nav.select(View::Doom);
        assert_eq!(nav.step(vec![Key::Menu], Duration::ZERO), Effect::PauseDoom);
        assert_eq!(nav.view, View::Avionics);
        nav.step(vec![Key::Init], Duration::from_millis(1));
        nav.step(vec![Key::Character('A')], Duration::from_millis(2));
        assert_eq!(nav.aviation.scratch, "A");
    }
    #[test]
    fn fifth_left_key_launches_doom_and_fm_tracks_menu_only() {
        let mut nav = Navigation::default();
        assert!(!nav.mcdu_fm());
        nav.aircraft();
        assert!(nav.mcdu_fm());
        nav.step(vec![Key::Init], Duration::ZERO);
        assert!(!nav.mcdu_fm());
        nav.step(vec![Key::Menu], Duration::from_millis(1));
        assert!(nav.mcdu_fm());
        assert_eq!(
            nav.step(
                vec![Key::Lsk {
                    right: false,
                    row: 4
                }],
                Duration::from_millis(2)
            ),
            Effect::StartDoom
        );
        assert_eq!(nav.view, View::Doom);
        assert!(!nav.mcdu_fm());
        nav.step(vec![Key::Menu], Duration::from_millis(3));
        assert!(nav.mcdu_fm());
    }
    #[test]
    fn hidden_doom_command_changes_font_family_and_keeps_menu_unmodified() {
        use crate::device::transport::FontProfile;
        let mut nav = Navigation::default();
        nav.aircraft();
        assert_eq!(nav.font_profile(), FontProfile::Aircraft);
        assert_eq!(nav.render(53, 31).len(), 24 * 14);
        for (i, ch) in "DOOM".chars().enumerate() {
            nav.step(vec![], Duration::from_millis(i as u64 * 2));
            nav.step(
                vec![Key::Character(ch)],
                Duration::from_millis(i as u64 * 2 + 1),
            );
        }
        assert_eq!(
            nav.step(
                vec![Key::Lsk {
                    right: true,
                    row: 5
                }],
                Duration::from_secs(1)
            ),
            Effect::StartDoom
        );
        assert_eq!(nav.font_profile(), FontProfile::Game);
        assert_eq!(nav.render(53, 31).len(), 53 * 31);
        assert_eq!(
            nav.step(vec![Key::Menu], Duration::from_secs(2)),
            Effect::PauseDoom
        );
        assert_eq!(nav.font_profile(), FontProfile::Aircraft);
    }
    #[test]
    fn slash_is_ordinary_input_and_cannot_switch_game_mode() {
        let mut nav = Navigation::default();
        nav.select(View::Doom);
        for now in [0, 10, 11, 60] {
            assert_eq!(
                nav.step(vec![Key::Character('/')], Duration::from_secs(now)),
                Effect::None
            );
            assert_eq!(nav.view, View::Doom);
        }
        nav.aircraft();
        nav.step(vec![], Duration::from_secs(61));
        nav.step(vec![Key::Character('/')], Duration::from_secs(62));
        assert_eq!(nav.aviation.scratch, "/");
    }
}
