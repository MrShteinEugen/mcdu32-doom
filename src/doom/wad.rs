// SPDX-License-Identifier: GPL-3.0-only
use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

pub struct Wad {
    bytes: Vec<u8>,
    entries: Vec<(String, usize, usize)>,
}
pub struct Patch {
    pub width: usize,
    pub height: usize,
    pub luma: Vec<u8>,
}

impl Wad {
    pub fn open(path: &Path) -> Result<Self> {
        Self::parse(fs::read(path).with_context(|| format!("read WAD {}", path.display()))?)
    }
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < 12 || !matches!(&bytes[..4], b"IWAD" | b"PWAD") {
            bail!("invalid WAD header");
        }
        let count = read32(&bytes, 4)? as usize;
        let directory = read32(&bytes, 8)? as usize;
        let end = directory
            .checked_add(count.checked_mul(16).context("WAD directory overflow")?)
            .context("WAD directory overflow")?;
        if end > bytes.len() {
            bail!("WAD directory is truncated");
        }
        let mut entries = Vec::with_capacity(count);
        for record in bytes[directory..end].chunks_exact(16) {
            let start = read32(record, 0)? as usize;
            let size = read32(record, 4)? as usize;
            if start.checked_add(size).is_none_or(|end| end > bytes.len()) {
                bail!("WAD lump outside file");
            }
            let name = String::from_utf8_lossy(&record[8..16])
                .trim_end_matches('\0')
                .to_ascii_uppercase();
            entries.push((name, start, size));
        }
        Ok(Self { bytes, entries })
    }
    pub fn lump(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.0 == name)
            .map(|(_, start, size)| &self.bytes[*start..start + size])
    }
    pub fn portrait(&self, name: &str) -> Result<Patch> {
        let data = self
            .lump(name)
            .with_context(|| format!("WAD has no {name}"))?;
        let palette = self.lump("PLAYPAL").context("WAD has no PLAYPAL")?;
        if data.len() < 8 || palette.len() < 768 {
            bail!("truncated portrait or palette");
        }
        let width = u16::from_le_bytes([data[0], data[1]]) as usize;
        let height = u16::from_le_bytes([data[2], data[3]]) as usize;
        if width == 0 || height == 0 || width > 1024 || height > 1024 || 8 + width * 4 > data.len()
        {
            bail!("invalid patch dimensions");
        }
        let mut luma = vec![0; width * height];
        for x in 0..width {
            let mut offset = read32(data, 8 + x * 4)? as usize;
            loop {
                let top = *data.get(offset).context("truncated patch column")?;
                if top == 255 {
                    break;
                }
                let length = *data.get(offset + 1).context("truncated patch post")? as usize;
                let pixels = data
                    .get(offset + 3..offset + 3 + length)
                    .context("truncated patch pixels")?;
                for (y, &index) in pixels.iter().enumerate() {
                    if top as usize + y < height {
                        let p = &palette[index as usize * 3..index as usize * 3 + 3];
                        luma[(top as usize + y) * width + x] =
                            ((u32::from(p[0]) * 77 + u32::from(p[1]) * 150 + u32::from(p[2]) * 29)
                                >> 8) as u8;
                    }
                }
                offset += length + 4;
            }
        }
        Ok(Patch {
            width,
            height,
            luma,
        })
    }
}
fn read32(bytes: &[u8], offset: usize) -> Result<u32> {
    let data = bytes
        .get(offset..offset + 4)
        .context("truncated WAD integer")?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_out_of_bounds_directory() {
        let mut bytes = b"IWAD".to_vec();
        bytes.extend(1_u32.to_le_bytes());
        bytes.extend(1000_u32.to_le_bytes());
        assert!(Wad::parse(bytes).is_err());
    }
}
