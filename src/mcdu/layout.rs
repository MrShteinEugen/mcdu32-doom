// SPDX-License-Identifier: GPL-3.0-only
// Page template portions: Copyright (c) 2021-2023, 2025 FlyByWire Simulations.
//! Airbus page labels/placement adapted from FBW legacy page templates.
//! Reference revision and font attribution: assets/aircraft-font/NOTICE.md.
use super::{
    display::Text,
    pages::{Avionics, Page},
};

fn render(app: &Avionics) -> Vec<crate::display::Cell> {
    let mut p = Text::default();
    let f = &app.flight;
    let value = |s: &str, empty: &str| {
        if s.is_empty() {
            empty.to_owned()
        } else {
            s.to_owned()
        }
    };
    match app.page {
        Page::Menu => {
            p.put(0, 7, "MCDU MENU", 2, false);
            p.put(1, 17, "SELECT", 9, true);
            p.put(2, 0, "<FMGC", 4, false);
            p.right(2, "NAV B/UP>", 9, false);
            p.put(4, 0, "<ATSU", 2, false);
            p.put(6, 0, "<AIDS", 2, false);
            p.put(8, 0, "<CFDS", 2, false);
            p.doom_entry(10);
        }
        Page::Status => {
            p.title("A320-200");
            p.field(0, false, " ENG", "LEAP-1A26", 4);
            p.field(1, false, " ACTIVE NAV DATA BASE", " ------/------", 3);
            p.field(2, false, " SECOND NAV DATA BASE", " ------/------", 9);
            p.field(4, false, "CHG CODE", "[ ]", 9);
            p.field(5, false, "IDLE/PERF", "+0.0/+0.0", 4);
            p.field(5, true, "SOFTWARE  ", "STATUS/XLOAD>", 9);
        }
        Page::InitA => {
            p.put(0, 10, "INIT", 2, false);
            p.field(0, false, " CO RTE", "NONE", 3);
            p.field(
                0,
                true,
                "FROM/TO  ",
                &value(&f.route, "☐☐☐☐/☐☐☐☐"),
                if f.route.is_empty() { 1 } else { 3 },
            );
            p.field(1, false, "ALTN/CO RTE", "NONE", 3);
            p.field(1, true, "INIT ", "REQUEST*", 1);
            p.field(
                2,
                false,
                "FLT NBR",
                &value(&f.number, "☐☐☐☐☐☐☐☐"),
                if f.number.is_empty() { 1 } else { 3 },
            );
            p.right(6, "IRS INIT>", 2, false);
            p.right(8, "WIND/TEMP>", 2, false);
            p.field(
                4,
                false,
                "COST INDEX",
                &value(&f.cost_index, "☐☐☐"),
                if f.cost_index.is_empty() { 1 } else { 3 },
            );
            p.field(4, true, "TROPO", "36090", 3);
            p.field(
                5,
                false,
                "CRZ FL/TEMP",
                &format!("{}/---°", value(&f.cruise, "-----")),
                3,
            );
            p.field(5, true, "GND TEMP", &format!("+{}°", f.temperature), 3);
        }
        Page::InitB => {
            p.title("INIT FUEL PRED");
            p.field(0, false, "TAXI", "0.2", 3);
            p.field(0, true, "ZFW/ZFWCG", &format!("{}/25.0", f.zfw), 3);
            p.field(1, false, "TRIP /TIME", "---.-/----", 2);
            p.field(1, true, "BLOCK", &f.block, 3);
            p.field(2, false, "RTE RSV/%", "---.-/5.0", 3);
            p.field(3, false, "ALTN /TIME", "---.-/----", 2);
            p.field(3, true, "TOW/  LW", "---.-/---.-", 2);
            p.field(4, false, "FINAL/TIME", "---.-/0030", 3);
            p.field(4, true, "TRIP WIND", "HD000", 3);
            p.field(5, false, "MIN DEST FOB", "---.-", 3);
            p.field(5, true, "EXTRA/TIME", "---.-/----", 2);
        }
        Page::Prog => {
            p.put(0, 8, "TO", 4, false);
            p.put(0, 13, &f.number, 2, false);
            p.field(0, false, " CRZ", &f.cruise, 3);
            p.right(1, "OPT    REC MAX", 2, true);
            p.right(2, "-----   -----", 4, false);
            p.put(4, 0, "<REPORT", 2, false);
            p.put(7, 2, "BRG / DIST", 2, true);
            p.put(8, 0, "---°/----.-", 2, false);
            p.put(8, 13, "TO", 2, true);
            p.right(8, "[     ]", 3, false);
            p.field(4, false, " PREDICTIVE", "<GPS", 2);
            p.right(10, "GPS PRIMARY", 2, false);
            p.put(11, 0, "REQUIRED", 2, true);
            p.put(11, 10, "ACCUR", 2, true);
            p.right(11, "ESTIMATED", 2, true);
            p.put(12, 0, "1.0NM", 3, false);
            p.put(12, 10, "HIGH", 4, false);
            p.right(12, "0.05NM", 4, true);
        }
        Page::Perf(0) => {
            p.put(0, 2, "TAKE OFF RWY", 2, false);
            p.put(0, 15, &f.runway, 4, false);
            for (i, label, speed, retract) in [
                (0, " V1   FLP RETR", &f.v1, "F=---"),
                (1, " VR   SLT RETR", &f.vr, "S=---"),
                (2, " V2     CLEAN", &f.v2, "O=---"),
            ] {
                p.put(1 + i * 2, 0, label, 2, true);
                p.put(2 + i * 2, 0, &value(speed, "☐☐☐"), 3, false);
                p.put(2 + i * 2, 5, retract, 4, false);
            }
            p.field(1, true, "TO SHIFT ", "[  ]*", 9);
            p.field(2, true, "FLAPS/THS", "1/UP0.0", 3);
            p.field(3, false, "TRANS ALT", "5000", 3);
            p.field(3, true, "FLEX TO TEMP", &format!("{}°", f.flex), 3);
            p.field(4, false, "THR RED/ACC", "1500/1500", 3);
            p.field(4, true, "ENG OUT ACC", "1500", 3);
            p.field(5, false, " UPLINK", "<TO DATA", 9);
            p.field(5, true, "NEXT", "PHASE>", 2);
        }
        Page::Perf(4) => {
            p.title("APPR");
            p.field(0, false, "QNH", &f.qnh, 3);
            p.field(1, false, "TEMP", &format!("+{}°", f.temperature), 3);
            p.put(4, 8, "O=---", 4, false);
            p.field(1, true, "BARO", "[   ]", 3);
            p.field(2, false, "MAG WIND", &f.wind, 3);
            p.put(6, 10, "S=---", 4, false);
            p.field(2, true, "RADIO", "[   ]", 3);
            p.field(3, false, "TRANS ALT", "5000", 3);
            p.put(8, 10, "F=---", 4, false);
            p.field(4, false, "VAPP   VLS", "---    ---", 3);
            p.field(4, true, "LDG CONF ", "FULL/CONF3*", 3);
            p.field(5, false, "PREV", "<PHASE", 2);
            p.field(5, true, "NEXT", "PHASE>", 2);
        }
        Page::Perf(phase) => {
            p.title(["TAKE OFF", "CLB", "CRZ", "DES", "APPR", "GO AROUND"][usize::from(phase) % 6]);
            if phase == 5 {
                for (row, label, value) in [
                    (1, "FLP RETR", "F=---"),
                    (3, "SLT RETR", "S=---"),
                    (5, "CLEAN", "O=---"),
                ] {
                    p.put(row, 7, label, 2, true);
                    p.put(row + 1, 7, value, 4, false);
                }
                p.field(4, false, "THR RED/ACC", "1500/1500", 3);
                p.field(4, true, "ENG OUT ACC", "1500", 3);
            } else {
                p.field(0, false, "ACT MODE", "ECON", 4);
                p.field(1, false, "CI", &f.cost_index, 3);
                p.field(1, true, "PRED TO", &f.cruise, 3);
                p.field(2, false, "MANAGED", "---/---", 2);
                p.field(2, true, "DIST  TIME", "----  ----", 4);
                p.field(3, false, "PRESEL", "[   ]", 3);
                if phase == 2 {
                    p.field(3, true, "DES CABIN RATE", "-350FT/MN", 3);
                    p.right(10, "STEP ALTS>", 2, false);
                }
            }
            p.field(5, false, "PREV", "<PHASE", 2);
            p.field(5, true, "NEXT", "PHASE>", 2);
        }
        Page::Data => {
            p.title("DATA INDEX");
            p.right(0, "1/2", 2, true);
            p.field(0, false, " POSITION", "<MONITOR", 2);
            p.field(1, false, " IRS", "<MONITOR", 2);
            p.field(2, false, " GPS", "<MONITOR", 2);
            p.put(8, 0, "<A/C STATUS", 2, false);
            p.field(4, false, " CLOSEST", "<AIRPORTS", 2);
            p.field(5, false, " EQUITIME", "<POINT", 9);
            p.field(5, true, "ACARS/PRINT ", "FUNCTION>", 9);
        }
        Page::RadNav => {
            p.title("RADIO NAV");
            p.field(0, false, "VOR1/FREQ", &format!("[ ]/{}", f.vor1), 3);
            p.field(0, true, "FREQ/VOR2", &format!("{}/[ ]", f.vor2), 3);
            p.field(1, false, "CRS", "[   ]", 3);
            p.field(1, true, "CRS", "[   ]", 3);
            p.field(2, false, " LS /FREQ", &format!("[ ]/{}", f.ils), 3);
            p.field(3, false, "CRS", "[   ]", 3);
            p.field(3, true, "SLOPE", "---", 2);
            p.field(4, false, "ADF1/FREQ", "[ ]/---.-", 3);
            p.field(4, true, "FREQ/ADF2", "---.-/[ ]", 3);
            p.put(12, 0, "BFO", 9, false);
            p.right(12, "BFO", 9, false);
        }
        Page::FlightPlan => {
            p.put(0, 0, "FROM", 2, true);
            p.right(0, &f.number, 2, false);
            p.put(1, 7, "SPD/ALT", 2, true);
            p.right(1, "UTC", 2, true);
            p.put(
                2,
                0,
                &format!(
                    "{}{}",
                    f.route.split('/').next().unwrap_or("----"),
                    f.runway
                ),
                4,
                false,
            );
            p.put(2, 11, "---/-----", 4, true);
            p.right(2, "----", 2, true);
            p.put(4, 0, "---F-PLN DISCONTINUITY--", 2, false);
            p.put(6, 0, f.route.split('/').nth(1).unwrap_or("----"), 4, false);
            p.put(8, 0, "------END OF F-PLN------", 2, false);
            p.put(10, 0, "-----NO ALTN F-PLN-----", 2, false);
            p.field(
                5,
                false,
                "DEST",
                f.route.split('/').nth(1).unwrap_or("----"),
                4,
            );
            p.put(11, 7, "DIST  EFOB", 2, true);
            p.right(11, "TIME", 2, true);
            p.right(12, "---- ----.- ----", 2, true);
        }
        Page::Dir => {
            p.title("DIR TO");
            p.field(0, false, " WAYPOINT", "[     ]*", 3);
            p.field(0, true, "DIST  UTC", "---  ----", 4);
            p.put(4, 0, "---F-PLN WAYPOINTS---", 2, true);
            p.put(
                6,
                0,
                &format!("<{}", f.route.split('/').nth(1).unwrap_or("----")),
                3,
                false,
            );
            p.field(5, false, "", "<ERASE", 1);
            p.field(5, true, "", "DIRECT*", 1);
        }
        Page::Fuel => {
            p.title("FUEL PRED");
            p.put(1, 0, "AT", 2, true);
            p.put(1, 12, "UTC", 2, true);
            p.right(1, "EFOB", 2, true);
            p.put(2, 0, f.route.split('/').nth(1).unwrap_or("----"), 4, false);
            p.right(2, "----  ---.-", 4, false);
            p.field(2, false, "RTE RSV/%", "---.-/5.0", 3);
            p.field(2, true, "ZFW/ZFWCG", &format!("{}/25.0", f.zfw), 3);
            p.field(3, false, "ALTN /TIME", "---.-/----", 2);
            p.field(3, true, "FOB", &f.block, 4);
            p.field(4, false, "FINAL/TIME", "---.-/0030", 3);
            p.field(4, true, "GW/ CG", "---.-/--.-", 4);
            p.field(5, false, "MIN DEST FOB", "---.-", 3);
            p.field(5, true, "EXTRA/TIME", "---.-/----", 2);
        }
        Page::Secondary => {
            p.title("SEC INDEX");
            p.put(2, 0, "<COPY ACTIVE", 2, false);
            p.put(4, 0, "<INIT", 2, false);
            p.put(6, 0, "<SEC F-PLN", 2, false);
            p.put(8, 0, "<PERF", 2, false);
        }
        Page::Atsu => {
            p.title("ATSU DATALINK");
            p.put(2, 0, "<ATC MENU", 2, false);
            p.right(4, "AOC MENU>", 2, false);
            p.field(4, true, "DATALINK ", "STATUS>", 2);
            p.right(12, "COMM MENU>", 2, false);
        }
        Page::Aids => {
            p.title("AIDS");
            p.field(0, false, "CALL-UP", "<PARAM", 9);
            p.right(2, "LOAD STATUS>", 9, false);
            p.put(4, 0, "<PROGRAMMING", 9, false);
            p.put(6, 0, "<SAR", 9, false);
            p.field(2, true, "LIST OF", "PREV REP>", 9);
            p.field(3, true, "STORED", "REPORTS>", 9);
            p.field(4, false, "ASSIGNMENT", "<REMOTE PRINT", 9);
            p.field(4, true, "MAN REQST", "REPORTS>", 9);
            p.put(12, 0, "DAR = RUNNING", 4, false);
            p.field(5, true, "POST", "STOP*", 3);
        }
        Page::Cfds => {
            p.title("CFDS");
            p.right(0, "1/2", 2, true);
            for (row, text, color) in [
                (2, "<LAST LEG REPORT", 9),
                (4, "<LAST LEG ECAM REPORT", 9),
                (6, "<PREVIOUS LEGS REPORT", 9),
                (8, "<AVIONICS STATUS", 2),
                (10, "<SYSTEM REPORT / TEST", 2),
            ] {
                p.put(row, 0, text, color, false);
            }
            p.put(11, 10, "POST", 2, true);
            p.put(12, 0, "*SEND", 3, false);
            p.put(12, 9, "FLT REP", 2, true);
            p.right(12, "PRINT*", 9, false);
        }
        Page::Atc => {
            p.title("ATC MENU");
            p.put(2, 0, "<NOTIFICATION", 2, false);
            p.right(2, "CONNECTION>", 2, false);
            p.put(4, 0, "<MESSAGE RECORD", 2, false);
            p.right(4, "ATIS>", 2, false);
            p.put(6, 0, "<REQUEST", 9, false);
            p.right(6, "REPORTS>", 9, false);
            p.put(12, 0, "<RETURN", 2, false);
        }
        Page::Airport => {
            p.title("AIRPORTS");
            p.right(0, "1/2", 2, true);
            p.field(0, false, "AIRPORT", "[    ]", 3);
            p.put(12, 0, "<RETURN", 2, false);
        }
        Page::NavBackup => {
            p.title("NAV B/UP");
            p.field(0, false, "FROM/TO", &f.route, 3);
            p.field(1, false, "POSITION", "----.---/----.---", 4);
            p.put(12, 0, "<RETURN", 2, false);
        }
    }
    let scratch = app.message.as_deref().unwrap_or(if app.deleting {
        "CLR"
    } else if app.page == Page::Menu && app.scratch.is_empty() {
        "SELECT DESIRED SYSTEM"
    } else {
        &app.scratch
    });
    p.put(
        13,
        0,
        scratch,
        if app.message.is_some() { 1 } else { 2 },
        false,
    );
    p.cells
}

impl Avionics {
    pub fn render(&self, cols: usize, rows: usize) -> Vec<crate::display::Cell> {
        assert_eq!(
            (cols, rows),
            (super::display::COLUMNS, super::display::ROWS),
            "aircraft pages require the text display profile"
        );
        render(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_pages_use_classic_grid_and_both_text_banks() {
        let mut app = Avionics::default();
        for page in [
            Page::Menu,
            Page::Status,
            Page::InitA,
            Page::InitB,
            Page::Prog,
            Page::Data,
            Page::FlightPlan,
            Page::RadNav,
            Page::Fuel,
            Page::Secondary,
            Page::Atc,
            Page::Airport,
            Page::Atsu,
            Page::Aids,
            Page::Cfds,
            Page::NavBackup,
            Page::Perf(0),
            Page::Perf(4),
        ] {
            app.select(page);
            {
                let (cols, rows) = (24, 14);
                let cells = app.render(cols, rows);
                assert_eq!(cells.len(), cols * rows);
                assert!(
                    cells
                        .iter()
                        .all(|cell| usize::from(cell.glyph) < crate::display::GLYPHS)
                );
            }
        }
    }
}
