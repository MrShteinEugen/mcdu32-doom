// SPDX-License-Identifier: GPL-3.0-only
//! Font-bank reserialization from the licensed capture map.
//! Tight 11x14 records are verified on Captain V1.06; other packings are probes.
use super::captured_font;
use crate::display::{CELL_H, CELL_W, atlas::Atlas};
use anyhow::{Context, Result, bail, ensure};
use clap::ValueEnum;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Packing {
    /// Three bytes per bitmap row, matching the known height-resize implementation.
    Padded,
    /// Minimum whole-byte bitmap row; verified for the compact 11x14 font.
    #[default]
    Tight,
    /// Original 91-byte records, for comparison with the previous dense experiment.
    Captured,
}

#[derive(Debug)]
struct Bank {
    geometry: Vec<u8>,
    write: Vec<u8>,
    commit: Vec<u8>,
    addresses: Vec<u32>,
    data: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct BankInfo {
    pub id: u32,
    pub width: usize,
    pub height: usize,
    pub row_bytes: usize,
    pub record_bytes: usize,
    pub declared_records: usize,
    pub bitmap_buffer_bytes: usize,
    pub geometry_size: u32,
    pub write_chunks: usize,
}

pub struct Upload {
    pub packets: Vec<Vec<u8>>,
    pub banks: Vec<BankInfo>,
}

/// Resample both banks, regenerate record lengths, chunk offsets and packet sizes.
/// The captured suffix restores palette and brightness. It contains no grid command.
pub fn rebuild(
    atlas: &Atlas,
    width: usize,
    height: usize,
    packing: Packing,
    brightness: u8,
) -> Result<Upload> {
    ensure!(
        (5..=CELL_W).contains(&width),
        "font width must be 5..={CELL_W}"
    );
    ensure!(
        (7..=CELL_H).contains(&height),
        "font height must be 7..={CELL_H}"
    );
    let original = captured_font::packets(atlas, brightness)?;
    let resets: Vec<_> = original
        .iter()
        .enumerate()
        .filter(|(_, p)| p[..2] == [0xf0, 0] && p[3] == 2)
        .map(|(i, _)| i)
        .collect();
    ensure!(
        resets.len() == 2,
        "capture must contain two bank boundaries"
    );
    let banks = [
        parse_bank(&original[..resets[0]])?,
        parse_bank(&original[resets[0] + 1..resets[1]])?,
    ];
    let mut suffix = original[resets[1] + 1..].to_vec();
    captured_font::preserve_grid(&mut suffix);
    let row_bytes = match packing {
        Packing::Tight => width.div_ceil(8),
        Packing::Padded | Packing::Captured => 3,
    };
    let storage_height = if matches!(packing, Packing::Captured) {
        CELL_H
    } else {
        height
    };
    let record_bytes = 4 + row_bytes * storage_height;
    let mut packets = Vec::new();
    let mut infos = Vec::new();
    let mut sequence = 0_u8;
    for bank in banks {
        let old_stride = u32_at(&bank.geometry, 25)? as usize;
        let count = u32_at(&bank.geometry, 29)? as usize;
        ensure!(
            old_stride == 91 && bank.data.len() == count * old_stride,
            "captured bank buffer disagrees with its record count"
        );
        let mut buffer = vec![0; count * record_bytes];
        for (src, dst) in bank
            .data
            .chunks_exact(old_stride)
            .zip(buffer.chunks_exact_mut(record_bytes))
        {
            dst[..4].copy_from_slice(&src[..4]);
            for y in 0..height {
                for x in 0..width {
                    let (sx, sy) = crate::display::atlas::source_pixel(x, y, width, height);
                    if src[4 + sy * 3 + sx / 8] & (0x80 >> (sx % 8)) != 0 {
                        dst[4 + y * row_bytes + x / 8] |= 0x80 >> (x % 8);
                    }
                }
            }
        }
        let mut geometry = bank.geometry.clone();
        let size_extra = u32_at(&geometry, 37)?
            .checked_sub((count * old_stride) as u32)
            .context("geometry size smaller than its bitmap buffer")?;
        geometry[21..23].copy_from_slice(&(width as u16).to_le_bytes());
        geometry[23..25].copy_from_slice(&(height as u16).to_le_bytes());
        put_u32(&mut geometry, 25, record_bytes as u32);
        put_u32(&mut geometry, 37, buffer.len() as u32 + size_extra);
        packets.push(wrap(&geometry, &mut sequence));
        for (chunk, data) in buffer.chunks(512).enumerate() {
            let offset = chunk * 512;
            // Same piecewise descriptor-address interpolation as the researched
            // WINCTRL resizer. These opaque addresses remain a hardware hypothesis.
            let address = descriptor_address(&bank.addresses, offset / record_bytes, old_stride);
            let mut write = bank.write.clone();
            put_u32(&mut write, 8, address);
            put_u32(&mut write, 13, data.len() as u32 + 12);
            put_u32(&mut write, 17, u32_at(&geometry, 17)?);
            put_u32(&mut write, 21, offset as u32);
            put_u32(&mut write, 25, data.len() as u32);
            let mut commit = bank.commit.clone();
            put_u32(&mut commit, 8, address);
            let unit: Vec<_> = write
                .into_iter()
                .chain(data.iter().copied())
                .chain(commit)
                .collect();
            // Avoid the capture's special TYPE=0x02 bank-boundary report.
            // Rebalance one byte when an otherwise valid stream would end in
            // two bytes; this leaves the logical command stream unchanged.
            let first = if unit.len() % 60 == 2 { 59 } else { 60 };
            let split = first.min(unit.len());
            packets.push(wrap(&unit[..split], &mut sequence));
            for part in unit[split..].chunks(60) {
                packets.push(wrap(part, &mut sequence));
            }
            for _ in 0..3 {
                packets.push(vec![0xf0, 1, sequence, 0]);
                sequence = sequence.wrapping_add(1);
            }
        }
        infos.push(BankInfo {
            id: u32_at(&geometry, 17)?,
            width,
            height,
            row_bytes,
            record_bytes,
            declared_records: count,
            bitmap_buffer_bytes: buffer.len(),
            geometry_size: u32_at(&geometry, 37)?,
            write_chunks: buffer.len().div_ceil(512),
        });
    }
    // Do not mix baseline bank-reset packets with the resize transaction.
    packets.extend(suffix);
    Ok(Upload {
        packets,
        banks: infos,
    })
}

fn parse_bank(packets: &[Vec<u8>]) -> Result<Bank> {
    let raw: Vec<_> = packets
        .iter()
        .filter(|p| p[..2] == [0xf0, 0])
        .flat_map(|p| p[4..4 + p[3] as usize].iter().copied())
        .collect();
    ensure!(
        raw.len() >= 42 && raw[4..8] == [6, 1, 0, 0],
        "missing font geometry"
    );
    let mut bank = Bank {
        geometry: raw[..42].to_vec(),
        write: Vec::new(),
        commit: Vec::new(),
        addresses: Vec::new(),
        data: Vec::new(),
    };
    let mut cursor = 42;
    while cursor < raw.len() {
        let head = raw
            .get(cursor..cursor + 29)
            .context("truncated WRITE header")?;
        ensure!(
            head[4..8] == [7, 1, 0, 0],
            "unexpected captured font command"
        );
        let offset = u32_at(head, 21)? as usize;
        let length = u32_at(head, 25)? as usize;
        ensure!(
            u32_at(head, 17)? == u32_at(&bank.geometry, 17)?,
            "WRITE belongs to the wrong font bank"
        );
        ensure!(
            offset == bank.data.len() && length <= 512,
            "invalid captured font chunk"
        );
        ensure!(
            u32_at(head, 13)? as usize == length + 12,
            "WRITE size mismatch"
        );
        bank.addresses.push(u32_at(head, 8)?);
        if bank.write.is_empty() {
            bank.write = head.to_vec();
        }
        cursor += 29;
        bank.data.extend_from_slice(
            raw.get(cursor..cursor + length)
                .context("truncated bitmap chunk")?,
        );
        cursor += length;
        // The baseline capture's last two zero bytes are in its reset report.
        // Only use the complete first COMMIT as the rebuilding template.
        let available = (raw.len() - cursor).min(17);
        let commit = &raw[cursor..cursor + available];
        ensure!(
            commit.len() >= 15 && commit[4..8] == [5, 1, 0, 0],
            "missing captured COMMIT"
        );
        if bank.commit.is_empty() {
            ensure!(commit.len() == 17, "first COMMIT must be complete");
            bank.commit = commit.to_vec();
        }
        cursor += available;
    }
    Ok(bank)
}

fn descriptor_address(addresses: &[u32], glyph: usize, stride: usize) -> u32 {
    for (index, pair) in addresses.windows(2).enumerate() {
        let first = index * 512 / stride;
        let next = (index + 1) * 512 / stride;
        if (first..next).contains(&glyph) {
            return pair[0] + (pair[1] - pair[0]) * (glyph - first) as u32 / (next - first) as u32;
        }
    }
    addresses.last().copied().unwrap_or_default()
}

fn wrap(payload: &[u8], sequence: &mut u8) -> Vec<u8> {
    let mut packet = vec![0; 64];
    packet[..4].copy_from_slice(&[0xf0, 0, *sequence, payload.len() as u8]);
    packet[4..4 + payload.len()].copy_from_slice(payload);
    *sequence = sequence.wrapping_add(1);
    packet
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let Some(word) = bytes.get(offset..offset + 4) else {
        bail!("truncated font field at {offset}")
    };
    Ok(u32::from_le_bytes(word.try_into()?))
}
fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembled_stream_has_consistent_banks_and_compact_bitmaps() {
        for (packing, stride) in [
            (Packing::Tight, 32),
            (Packing::Padded, 46),
            (Packing::Captured, 91),
        ] {
            let upload = rebuild(&Atlas::universal(), 11, 14, packing, 200).expect("rebuild");
            assert!(
                upload
                    .packets
                    .iter()
                    .all(|p| !p.is_empty() && p.len() <= 64)
            );
            assert!(
                !upload
                    .packets
                    .iter()
                    .any(|p| p[..2] == [0xf0, 0] && p[3] == 2)
            );
            let second = upload
                .packets
                .iter()
                .position(|p| p.len() == 64 && p[3] == 42 && p[21] == 6)
                .expect("second bank");
            // Decode the logical wire after reassembling packet/chunk boundaries.
            let bank = parse_bank(&upload.packets[..second]).expect("decode generated stream");
            assert_eq!(u32_at(&bank.geometry, 25).expect("stride"), stride);
            assert_eq!(bank.data.len(), 116 * stride as usize);
            assert!(bank.data.chunks_exact(stride as usize).any(|record| {
                u32_at(record, 0).expect("code")
                    == crate::display::atlas::CHARSET
                        .chars()
                        .nth(31)
                        .expect("solid code") as u32
                    && record[4..4 + 14 * upload.banks[0].row_bytes]
                        .chunks(upload.banks[0].row_bytes)
                        .all(|row| row[0] == 255 && row[1] == 224)
            }));
            assert_eq!(
                upload.banks.iter().map(|b| b.id).collect::<Vec<_>>(),
                [5, 6]
            );
            let mut start = 0;
            for info in &upload.banks {
                let count = 1
                    + (0..info.bitmap_buffer_bytes)
                        .step_by(512)
                        .map(|off| {
                            (29 + (info.bitmap_buffer_bytes - off).min(512) + 17).div_ceil(60) + 3
                        })
                        .sum::<usize>();
                let bank =
                    parse_bank(&upload.packets[start..start + count]).expect("complete bank");
                assert_eq!(u32_at(&bank.geometry, 17).expect("bank id"), info.id);
                assert_eq!(bank.data.len(), info.declared_records * info.record_bytes);
                let codes: Vec<_> = bank
                    .data
                    .chunks_exact(info.record_bytes)
                    .map(|r| u32_at(r, 0).expect("record code"))
                    .filter(|&code| code != 0)
                    .collect();
                assert_eq!(codes.len(), 110);
                assert_eq!(
                    codes,
                    crate::display::atlas::CHARSET
                        .chars()
                        .map(|c| c as u32)
                        .collect::<Vec<_>>()
                );
                start += count;
            }
        }
    }
}
