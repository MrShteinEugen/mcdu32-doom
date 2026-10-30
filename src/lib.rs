// SPDX-License-Identifier: GPL-3.0-only
//! Raster glyph rendering and the WINCTRL MCDU-32 wire protocol.
//! All rendering is usable without opening a USB device.
pub mod config;
pub mod desktop;
pub mod device;
pub mod display;
pub mod doom;
pub mod input;
pub mod mcdu;
pub mod output;
pub mod runtime;
