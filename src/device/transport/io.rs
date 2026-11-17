// SPDX-License-Identifier: GPL-3.0-only
//! Bounded input polling and padded HID writes, testable without hardware.
use super::{ConnectionError, UsbStats};
use crate::input::Inputs;
use anyhow::{Context, Result, anyhow, bail};
use hidapi::HidDevice;
use std::{
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

pub(super) trait ReportDevice {
    fn write(&self, report: &[u8]) -> Result<usize>;
    fn read(&self, report: &mut [u8]) -> Result<usize>;
}
impl ReportDevice for HidDevice {
    fn write(&self, report: &[u8]) -> Result<usize> {
        Ok(HidDevice::write(self, report)?)
    }
    fn read(&self, report: &mut [u8]) -> Result<usize> {
        Ok(self.read_timeout(report, 0)?)
    }
}

pub(super) fn validate_report(packet: &[u8]) -> Result<()> {
    if packet.is_empty() || packet.len() > 64 {
        bail!("invalid HID report length: {}", packet.len());
    }
    Ok(())
}

pub(super) fn write_report(device: &impl ReportDevice, packet: &[u8]) -> Result<()> {
    validate_report(packet)?;
    let mut report = [0; 64];
    report[..packet.len()].copy_from_slice(packet);
    let written = device
        .write(&report)
        .context("write MCDU report")
        .map_err(ConnectionError::from)?;
    if written != report.len() {
        return Err(ConnectionError::from(anyhow!(
            "short HID write: {written}/{} bytes",
            report.len()
        ))
        .into());
    }
    Ok(())
}

pub(super) fn drain_responses(
    device: &impl ReportDevice,
    responses: &mut Vec<Vec<u8>>,
) -> Result<()> {
    read_reports(device, |report| {
        if responses.len() < 4096 {
            responses.push(report.to_vec());
        }
    })
    .context("read diagnostic response")
}

pub(super) fn send(
    device: &impl ReportDevice,
    packet: &[u8],
    input: &Inputs,
    stats: &UsbStats,
    pacing: Duration,
    next: &mut Instant,
) -> Result<()> {
    validate_report(packet)?;
    let now = Instant::now();
    if *next > now {
        thread::sleep(*next - now);
    }
    write_report(device, packet)?;
    *next = Instant::now() + pacing;
    stats.packets.fetch_add(1, Ordering::Relaxed);
    poll(device, input)
}
pub(super) fn poll(device: &impl ReportDevice, input: &Inputs) -> Result<()> {
    read_reports(device, |report| input.update_panel(report)).context("read MCDU input")
}

fn read_reports(device: &impl ReportDevice, mut consume: impl FnMut(&[u8])) -> Result<()> {
    let mut report = [0; 64];
    // Bound draining so a noisy device cannot starve display output.
    for _ in 0..32 {
        let count = device.read(&mut report).map_err(ConnectionError::from)?;
        if count == 0 {
            break;
        }
        consume(&report[..count]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque};

    #[derive(Default)]
    struct Device {
        writes: RefCell<Vec<Vec<u8>>>,
        reads: RefCell<VecDeque<Vec<u8>>>,
        short_write: bool,
        disconnected: bool,
    }
    impl ReportDevice for Device {
        fn write(&self, report: &[u8]) -> Result<usize> {
            if self.disconnected {
                bail!("disconnected");
            }
            self.writes.borrow_mut().push(report.to_vec());
            Ok(if self.short_write { 63 } else { report.len() })
        }
        fn read(&self, report: &mut [u8]) -> Result<usize> {
            if self.disconnected {
                bail!("disconnected");
            }
            let Some(value) = self.reads.borrow_mut().pop_front() else {
                return Ok(0);
            };
            report[..value.len()].copy_from_slice(&value);
            Ok(value.len())
        }
    }

    #[test]
    fn sending_pads_reports_counts_success_and_updates_input() {
        let device = Device::default();
        let mut up = vec![0; 14];
        up[0] = 1;
        up[4] = 0x20;
        device.reads.borrow_mut().push_back(up);
        let input = Inputs::default();
        let stats = UsbStats::default();
        send(
            &device,
            &[0xf0, 1, 0, 0],
            &input,
            &stats,
            Duration::ZERO,
            &mut Instant::now(),
        )
        .expect("send");
        let reports = device.writes.borrow();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].len(), 64);
        assert_eq!(&reports[0][..4], &[0xf0, 1, 0, 0]);
        assert!(reports[0][4..].iter().all(|&b| b == 0));
        assert_eq!(stats.snapshot().usb_reports, 1);
        assert_eq!(input.state(), 1);
    }

    #[test]
    fn short_and_failed_writes_do_not_count_as_sent_reports() {
        for device in [
            Device {
                short_write: true,
                ..Device::default()
            },
            Device {
                disconnected: true,
                ..Device::default()
            },
        ] {
            let stats = UsbStats::default();
            let error = send(
                &device,
                &[1],
                &Inputs::default(),
                &stats,
                Duration::ZERO,
                &mut Instant::now(),
            )
            .expect_err("failed send");
            assert!(error.is::<ConnectionError>());
            assert_eq!(stats.snapshot().usb_reports, 0);
        }
    }

    #[test]
    fn invalid_lengths_are_rejected_before_writing_or_opening_hardware() {
        let device = Device::default();
        for packet in [vec![], vec![0; 65]] {
            let error = write_report(&device, &packet).expect_err("invalid report");
            assert!(!error.is::<ConnectionError>());
            let error = crate::device::transport::write_once(None, &[packet], Duration::ZERO)
                .err()
                .expect("invalid report");
            assert!(error.to_string().contains("invalid HID report length"));
        }
        assert!(device.writes.borrow().is_empty());
    }

    #[test]
    fn lost_input_connection_is_reported_through_poll_context() {
        let device = Device {
            disconnected: true,
            ..Device::default()
        };
        let error = poll(&device, &Inputs::default()).expect_err("disconnected device");
        assert!(error.is::<ConnectionError>());
        assert_eq!(error.to_string(), "read MCDU input");
    }

    #[test]
    fn polling_is_bounded_and_diagnostic_storage_is_capped() {
        let device = Device::default();
        device
            .reads
            .borrow_mut()
            .extend((0..40).map(|_| vec![1; 14]));
        poll(&device, &Inputs::default()).expect("bounded poll");
        assert_eq!(device.reads.borrow().len(), 8);
        let mut responses = vec![vec![]; 4095];
        drain_responses(&device, &mut responses).expect("drain");
        assert_eq!(responses.len(), 4096);
        assert!(device.reads.borrow().is_empty());
    }
}
