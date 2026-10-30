// SPDX-License-Identifier: GPL-3.0-only
//! Editable standalone page state; visual templates live in `layout`.
use super::keyboard::Key;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub enum Page {
    #[default]
    Menu,
    Status,
    InitA,
    InitB,
    Prog,
    Perf(u8),
    Data,
    FlightPlan,
    Dir,
    RadNav,
    Fuel,
    Secondary,
    Atc,
    Airport,
    Atsu,
    Aids,
    Cfds,
    NavBackup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    StartDoom,
}

#[derive(Clone, Debug)]
pub struct Flight {
    pub route: String,
    pub number: String,
    pub cost_index: String,
    pub cruise: String,
    pub zfw: String,
    pub block: String,
    pub runway: String,
    pub v1: String,
    pub vr: String,
    pub v2: String,
    pub flex: String,
    pub qnh: String,
    pub temperature: String,
    pub wind: String,
    pub vor1: String,
    pub vor2: String,
    pub ils: String,
}
impl Default for Flight {
    fn default() -> Self {
        Self {
            route: "EGLL/EDDF".into(),
            number: "AFL320".into(),
            cost_index: "25".into(),
            cruise: "FL350".into(),
            zfw: "60.0".into(),
            block: "6.0".into(),
            runway: "27R".into(),
            v1: "140".into(),
            vr: "144".into(),
            v2: "149".into(),
            flex: "45".into(),
            qnh: "1013".into(),
            temperature: "15".into(),
            wind: "000/00".into(),
            vor1: "113.60".into(),
            vor2: "114.20".into(),
            ils: "110.30".into(),
        }
    }
}
#[derive(Default)]
pub struct Avionics {
    pub page: Page,
    pub flight: Flight,
    pub scratch: String,
    pub message: Option<String>,
    pub(super) deleting: bool,
    offset: usize,
}
#[derive(Clone, Copy)]
enum Field {
    Route,
    Number,
    Cost,
    Cruise,
    Zfw,
    Block,
    Runway,
    V1,
    Vr,
    V2,
    Flex,
    Qnh,
    Temp,
    Wind,
    Vor1,
    Vor2,
    Ils,
}
impl Avionics {
    pub fn select(&mut self, page: Page) {
        self.page = page;
        self.offset = 0;
        self.message = None;
    }
    pub fn clear_scratch(&mut self) {
        self.scratch.clear();
        self.message = None;
        self.deleting = false;
    }
    pub fn handle(&mut self, key: Key) -> Action {
        if let Some(page) = page_key(key) {
            self.select(page);
            return Action::None;
        }
        match key {
            Key::Character(ch) => {
                self.message = None;
                if self.deleting {
                    self.clear_scratch();
                }
                if self.scratch.len() < 22 {
                    self.scratch.push(ch.to_ascii_uppercase());
                }
            }
            Key::Sign => {
                if self.scratch.starts_with('-') {
                    self.scratch.remove(0);
                } else if self.scratch.len() < 22 {
                    self.scratch.insert(0, '-');
                }
            }
            Key::Clear => {
                if self.message.take().is_none() && self.scratch.pop().is_none() {
                    self.deleting = !self.deleting;
                }
            }
            Key::Left | Key::Right => self.cycle(key == Key::Right),
            Key::Up => self.offset = self.offset.saturating_sub(1),
            Key::Down => self.offset = (self.offset + 1).min(2),
            Key::Lsk { right, row } => return self.lsk(right, row),
            _ => {}
        }
        Action::None
    }
    fn cycle(&mut self, forward: bool) {
        self.page = match self.page {
            Page::InitA | Page::InitB => {
                if self.page == Page::InitA {
                    Page::InitB
                } else {
                    Page::InitA
                }
            }
            Page::Perf(phase) => Page::Perf(if forward {
                (phase + 1) % 6
            } else {
                (phase + 5) % 6
            }),
            other => other,
        };
    }
    fn lsk(&mut self, right: bool, row: usize) -> Action {
        let field = match (self.page, right, row) {
            (Page::Menu, false, 0) => {
                self.select(Page::Status);
                None
            }
            (Page::Menu, false, 1) => {
                self.select(Page::Atsu);
                None
            }
            (Page::Menu, false, 2) => {
                self.select(Page::Aids);
                None
            }
            (Page::Menu, false, 3) => {
                self.select(Page::Cfds);
                None
            }
            (Page::Menu, false, 4) => {
                self.clear_scratch();
                return Action::StartDoom;
            }
            (Page::Menu, true, 0) => {
                self.select(Page::NavBackup);
                None
            }
            (Page::Menu, true, 5) if self.scratch == "DOOM" => {
                self.clear_scratch();
                return Action::StartDoom;
            }
            (Page::Data, false, 3) => {
                self.select(Page::Status);
                None
            }
            (Page::Data, false, 4) => {
                self.select(Page::Airport);
                None
            }
            (Page::Atsu, false, 0) => {
                self.select(Page::Atc);
                None
            }
            (Page::InitA, true, 0) => Some(Field::Route),
            (Page::InitA, false, 2) => Some(Field::Number),
            (Page::InitA, false, 4) => Some(Field::Cost),
            (Page::InitA, false, 5) | (Page::Prog, false, 0) => Some(Field::Cruise),
            (Page::InitB, true, 0) => Some(Field::Zfw),
            (Page::InitB, true, 1) => Some(Field::Block),
            (Page::Perf(0), false, 0) => Some(Field::V1),
            (Page::Perf(0), false, 1) => Some(Field::Vr),
            (Page::Perf(0), false, 2) => Some(Field::V2),
            (Page::Perf(0), true, 0) => Some(Field::Runway),
            (Page::Perf(0), true, 3) => Some(Field::Flex),
            (Page::Perf(4), false, 0) => Some(Field::Qnh),
            (Page::Perf(4), false, 1) => Some(Field::Temp),
            (Page::Perf(4), false, 2) => Some(Field::Wind),
            (Page::RadNav, false, 0) => Some(Field::Vor1),
            (Page::RadNav, true, 0) => Some(Field::Vor2),
            (Page::RadNav, false, 2) => Some(Field::Ils),
            (Page::Dir, false, 0) => Some(Field::Route),
            (Page::Perf(_), true, 5) => {
                self.cycle(true);
                None
            }
            (Page::Perf(_), false, 5) => {
                self.cycle(false);
                None
            }
            (Page::Atc | Page::Airport | Page::NavBackup, false, 5) => {
                self.select(Page::Menu);
                None
            }
            _ => {
                self.message = Some("NOT ALLOWED".into());
                None
            }
        };
        if let Some(field) = field {
            self.edit(field);
        }
        Action::None
    }
    fn edit(&mut self, field: Field) {
        let target = match field {
            Field::Route => &mut self.flight.route,
            Field::Number => &mut self.flight.number,
            Field::Cost => &mut self.flight.cost_index,
            Field::Cruise => &mut self.flight.cruise,
            Field::Zfw => &mut self.flight.zfw,
            Field::Block => &mut self.flight.block,
            Field::Runway => &mut self.flight.runway,
            Field::V1 => &mut self.flight.v1,
            Field::Vr => &mut self.flight.vr,
            Field::V2 => &mut self.flight.v2,
            Field::Flex => &mut self.flight.flex,
            Field::Qnh => &mut self.flight.qnh,
            Field::Temp => &mut self.flight.temperature,
            Field::Wind => &mut self.flight.wind,
            Field::Vor1 => &mut self.flight.vor1,
            Field::Vor2 => &mut self.flight.vor2,
            Field::Ils => &mut self.flight.ils,
        };
        if self.scratch.is_empty() && !self.deleting {
            self.scratch = target.clone();
            return;
        }
        if self.deleting {
            target.clear();
            self.clear_scratch();
            return;
        }
        let numeric = |min: f64, max: f64| {
            self.scratch
                .parse::<f64>()
                .ok()
                .is_some_and(|v| v.is_finite() && (min..=max).contains(&v))
        };
        let valid = match field {
            Field::Route => {
                self.scratch.len() == 9
                    && self.scratch.as_bytes()[4] == b'/'
                    && self
                        .scratch
                        .chars()
                        .filter(|c| *c != '/')
                        .all(|c| c.is_ascii_alphabetic())
            }
            Field::Number => {
                self.scratch.len() <= 8 && self.scratch.chars().all(|c| c.is_ascii_alphanumeric())
            }
            Field::Cruise => self
                .scratch
                .strip_prefix("FL")
                .unwrap_or(&self.scratch)
                .parse::<u16>()
                .ok()
                .is_some_and(|v| (100..=450).contains(&v)),
            Field::Cost => self.scratch.parse::<u16>().ok().is_some_and(|v| v <= 999),
            Field::Zfw => numeric(30.0, 80.0),
            Field::Block => numeric(0.0, 30.0),
            Field::V1 | Field::Vr | Field::V2 => self
                .scratch
                .parse::<u16>()
                .ok()
                .is_some_and(|v| (80..=220).contains(&v)),
            Field::Flex => numeric(0.0, 70.0),
            Field::Qnh => numeric(870.0, 1085.0),
            Field::Temp => numeric(-60.0, 60.0),
            Field::Vor1 | Field::Vor2 => numeric(108.0, 117.95),
            Field::Ils => numeric(108.1, 111.95),
            Field::Runway => {
                let digits = self.scratch.trim_end_matches(['L', 'C', 'R']);
                digits.len() == 2
                    && digits
                        .parse::<u8>()
                        .ok()
                        .is_some_and(|v| (1..=36).contains(&v))
            }
            Field::Wind => self
                .scratch
                .split_once('/')
                .is_some_and(|(direction, speed)| {
                    direction.parse::<u16>().ok().is_some_and(|v| v <= 360)
                        && speed.parse::<u16>().ok().is_some_and(|v| v <= 200)
                }),
        };
        if valid {
            *target = if matches!(field, Field::Cruise) && !self.scratch.starts_with("FL") {
                format!("FL{}", self.scratch)
            } else {
                self.scratch.clone()
            };
            self.clear_scratch();
        } else {
            self.message = Some("FORMAT ERROR".into());
        }
    }
}

pub fn page_key(key: Key) -> Option<Page> {
    Some(match key {
        Key::Menu => Page::Menu,
        Key::Init => Page::InitA,
        Key::Prog => Page::Prog,
        Key::Perf => Page::Perf(0),
        Key::Data => Page::Data,
        Key::FlightPlan => Page::FlightPlan,
        Key::Dir => Page::Dir,
        Key::RadNav => Page::RadNav,
        Key::Fuel => Page::Fuel,
        Key::Secondary => Page::Secondary,
        Key::Atc => Page::Atc,
        Key::Airport => Page::Airport,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scratchpad_updates_route_and_cruise_with_validation_and_navigation() {
        let mut app = Avionics::default();
        app.handle(Key::Init);
        for ch in "KJFK/KBOS".chars() {
            app.handle(Key::Character(ch));
        }
        app.handle(Key::Lsk {
            right: true,
            row: 0,
        });
        assert_eq!(app.flight.route, "KJFK/KBOS");
        assert!(app.scratch.is_empty());
        app.scratch = "FL999".into();
        app.handle(Key::Lsk {
            right: false,
            row: 5,
        });
        assert_eq!(app.flight.cruise, "FL350");
        assert_eq!(app.message.as_deref(), Some("FORMAT ERROR"));
        app.clear_scratch();
        app.scratch = "380".into();
        app.handle(Key::Lsk {
            right: false,
            row: 5,
        });
        assert_eq!(app.flight.cruise, "FL380");
        app.handle(Key::Right);
        assert_eq!(app.page, Page::InitB);
        app.handle(Key::Menu);
        assert_eq!(app.page, Page::Menu);
        app.scratch = "DOOM".into();
        assert_eq!(
            app.handle(Key::Lsk {
                right: true,
                row: 5
            }),
            Action::StartDoom
        );
    }
}
