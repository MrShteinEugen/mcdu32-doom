// SPDX-License-Identifier: GPL-3.0-only
//! Standalone MCDU pages with FlyByWire fonts and conventional Airbus layouts.
pub mod display;
pub mod keyboard;
mod layout;
mod pages;
pub use pages::{Action, Avionics, Flight, Page, page_key};
