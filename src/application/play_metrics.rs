// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    device::transport::UsbMetrics,
    display::Profile,
    display::render::Style,
    output::metrics::{DisplayMetrics, GameMetrics, RenderMetrics},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct PlayMetrics {
    pub style: Style,
    pub display: Profile,
    #[serde(flatten)]
    pub geometry: DisplayMetrics,
    pub packet_us: u64,
    pub elapsed_seconds: f64,
    #[serde(flatten)]
    pub render: RenderMetrics,
    #[serde(flatten)]
    pub usb: UsbMetrics,
    #[serde(flatten)]
    pub game: GameMetrics,
}
