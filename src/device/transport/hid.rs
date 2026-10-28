// SPDX-License-Identifier: GPL-3.0-only
//! Device discovery and finite diagnostic transactions.
use super::{
    io::{drain_responses, validate_report, write_report},
    is_mcdu,
};
use anyhow::{Context, Result, bail};
use hidapi::{DeviceInfo, HidApi};
use serde::Serialize;
use std::{
    ffi::CString,
    thread,
    time::{Duration, Instant},
};

pub fn list() -> Result<()> {
    let api = HidApi::new()?;
    let mut count = 0;
    for info in api.device_list().filter(|d| is_mcdu(d)) {
        count += 1;
        println!(
            "{}: {:04x}:{:04x} firmware={:04x} usage={:04x}:{:04x} interface={}\n  {}",
            info.product_string().unwrap_or("MCDU"),
            info.vendor_id(),
            info.product_id(),
            info.release_number(),
            info.usage_page(),
            info.usage(),
            info.interface_number(),
            info.path().to_string_lossy()
        );
    }
    if count == 0 {
        println!("No connected MCDU-32 found.");
    }
    Ok(())
}

#[derive(Serialize)]
pub struct Transmission {
    pub product: String,
    pub firmware_release: u16,
    pub reports_written: usize,
    pub elapsed_ms: u128,
    /// Raw responses may include keyboard reports; they do not prove layout acceptance.
    pub responses: Vec<Vec<u8>>,
}

/// Send one finite diagnostic transaction and close the device, leaving its screen visible.
pub fn write_once(
    path: Option<&str>,
    reports: &[Vec<u8>],
    pacing: Duration,
) -> Result<Transmission> {
    for packet in reports {
        validate_report(packet)?;
    }
    let api = HidApi::new()?;
    let info = select_device(&api, path)?;
    let mut result = Transmission {
        product: info.product_string().unwrap_or("MCDU").to_owned(),
        firmware_release: info.release_number(),
        reports_written: 0,
        elapsed_ms: 0,
        responses: Vec::new(),
    };
    let device = api
        .open_path(info.path())
        .context("open MCDU for diagnostic display")?;
    device.set_blocking_mode(false)?;
    let started = Instant::now();
    for (index, packet) in reports.iter().enumerate() {
        write_report(&device, packet).with_context(|| {
            format!(
                "diagnostic report {index}; {} earlier reports written",
                result.reports_written
            )
        })?;
        result.reports_written += 1;
        thread::sleep(pacing);
        drain_responses(&device, &mut result.responses)?;
    }
    thread::sleep(Duration::from_millis(100));
    drain_responses(&device, &mut result.responses)?;
    result.elapsed_ms = started.elapsed().as_millis();
    Ok(result)
}

pub(super) fn select_device<'a>(api: &'a HidApi, path: Option<&str>) -> Result<&'a DeviceInfo> {
    if let Some(path) = path {
        let path = CString::new(path)?;
        return api
            .device_list()
            .find(|d| d.path() == path.as_c_str() && is_mcdu(d))
            .context("selected path is not a connected MCDU-32");
    }
    let devices: Vec<_> = api.device_list().filter(|d| is_mcdu(d)).collect();
    match devices.as_slice() {
        [] => bail!("MCDU not found; connect the panel and run `mcdu-doom devices`"),
        [only] => Ok(*only),
        _ => bail!(
            "multiple MCDU HID collections/devices found; select an exact --device-path from `devices`"
        ),
    }
}
