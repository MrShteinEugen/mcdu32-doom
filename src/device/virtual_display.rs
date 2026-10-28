// SPDX-License-Identifier: GPL-3.0-only
//! Complete-frame transfer model, independent of windows or USB handles.
use crate::{
    device::protocol::encode_cells,
    display::{
        Cell,
        atlas::{Atlas, BLANK},
    },
};
use anyhow::{Context, Result, ensure};
use std::time::Duration;

struct Transfer {
    cells: Vec<Cell>,
    ready: Duration,
    packets: usize,
    duration: Duration,
}

/// A complete frame arrives after its modeled transfer. Only the newest waiting
/// frame is retained; input and the front-panel repaint are never throttled here.
pub struct VirtualDisplay {
    pub visible: Vec<Cell>,
    transfer: Option<Transfer>,
    pending: Option<Vec<Cell>>,
    packet_time: Duration,
    min_frame: Duration,
    pub presented: u64,
    pub dropped: u64,
    pub reports: u64,
    pub last_transfer: Duration,
}

impl VirtualDisplay {
    pub fn new(cells: usize, packet_time: Duration, fps: u32) -> Result<Self> {
        ensure!(
            cells > 0 && fps > 0,
            "invalid virtual display size or frequency"
        );
        Ok(Self {
            visible: vec![
                Cell {
                    glyph: BLANK,
                    color: 0
                };
                cells
            ],
            transfer: None,
            pending: None,
            packet_time,
            min_frame: Duration::from_secs_f64(1.0 / f64::from(fps)),
            presented: 0,
            dropped: 0,
            reports: 0,
            last_transfer: Duration::ZERO,
        })
    }

    pub fn submit(&mut self, cells: Vec<Cell>, now: Duration, atlas: &Atlas) -> Result<()> {
        ensure!(
            cells.len() == self.visible.len(),
            "virtual display frame size mismatch"
        );
        if let Some(transfer) = &self.transfer {
            if self.pending.as_ref() == Some(&cells)
                || (self.pending.is_none() && transfer.cells == cells)
            {
                return Ok(());
            }
            if self.pending.replace(cells).is_some() {
                self.dropped += 1;
            }
        } else if self.visible != cells {
            self.begin(cells, now, atlas)?;
        }
        Ok(())
    }
    fn begin(&mut self, cells: Vec<Cell>, now: Duration, atlas: &Atlas) -> Result<()> {
        let packets = encode_cells(atlas, &cells)?.len();
        let duration = (self.packet_time * packets as u32).max(self.min_frame);
        self.transfer = Some(Transfer {
            cells,
            ready: now + duration,
            packets,
            duration,
        });
        Ok(())
    }
    pub fn advance(&mut self, now: Duration, atlas: &Atlas) -> Result<bool> {
        if self.transfer.as_ref().is_none_or(|t| now < t.ready) {
            return Ok(false);
        }
        let transfer = self.transfer.take().context("missing virtual transfer")?;
        self.visible = transfer.cells;
        self.presented += 1;
        self.reports += transfer.packets as u64;
        self.last_transfer = transfer.duration;
        if let Some(pending) = self.pending.take() {
            if pending != self.visible {
                self.begin(pending, now, atlas)?;
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(glyph: u8) -> Vec<Cell> {
        vec![Cell { glyph, color: 2 }; 48 * 28]
    }
    #[test]
    fn dense_grid_waits_for_all_packets_and_replaces_waiting_frames() {
        let atlas = Atlas::universal();
        let mut display =
            VirtualDisplay::new(48 * 28, Duration::from_millis(3), 5).expect("display");
        display
            .submit(frame(31), Duration::ZERO, &atlas)
            .expect("submit");
        assert!(
            !display
                .advance(Duration::from_millis(199), &atlas)
                .expect("advance")
        );
        display
            .submit(frame(32), Duration::from_millis(100), &atlas)
            .expect("submit");
        display
            .submit(frame(33), Duration::from_millis(150), &atlas)
            .expect("submit");
        assert_eq!(display.dropped, 1);
        assert!(
            display
                .advance(Duration::from_millis(200), &atlas)
                .expect("advance")
        );
        assert_eq!(display.visible[0].glyph, 31);
        assert_eq!(display.reports, 64);
        assert!(
            display
                .advance(Duration::from_millis(400), &atlas)
                .expect("advance")
        );
        assert_eq!(display.visible[0].glyph, 33);
        assert_eq!(display.presented, 2);
    }
    #[test]
    fn unicode_payload_increases_simulated_transfer_time() {
        let atlas = Atlas::universal();
        let mut display =
            VirtualDisplay::new(48 * 28, Duration::from_millis(3), 5).expect("display");
        display
            .submit(frame(109), Duration::ZERO, &atlas)
            .expect("submit");
        assert!(
            !display
                .advance(Duration::from_millis(200), &atlas)
                .expect("advance")
        );
        assert!(
            display
                .advance(Duration::from_millis(321), &atlas)
                .expect("advance")
        );
        assert_eq!(display.reports, 107);
    }
}
