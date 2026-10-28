// SPDX-License-Identifier: GPL-3.0-only
//! Decode the licensed capture map without depending on rebuilt font transactions.
use super::error::{ProtocolError, invalid};
use crate::display::{CELL_H, CELL_W, atlas::Atlas};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FontMap {
    glyph_width: usize,
    glyph_height: usize,
    packets: Vec<String>,
    x_offset_offset: usize,
    y_offset_offset: usize,
    glyph_width_offsets: Vec<usize>,
    glyph_height_offsets: Vec<usize>,
    display_brightness_offset: usize,
    large_glyph_offsets: Vec<GlyphOffsets>,
    small_glyph_offsets: Vec<GlyphOffsets>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GlyphOffsets {
    character: char,
    glyph_map: Vec<i32>,
}

pub(super) fn packets(atlas: &Atlas, brightness: u8) -> Result<Vec<Vec<u8>>, ProtocolError> {
    let map: FontMap = serde_json::from_str(include_str!("../../assets/font-map.json"))
        .map_err(|e| invalid(e.to_string()))?;
    if map.glyph_width != CELL_W || map.glyph_height != CELL_H {
        return Err(invalid("font dimensions mismatch"));
    }
    let mut blob = Vec::new();
    let mut lengths = Vec::new();
    for encoded in &map.packets {
        let hex = encoded.replace("{CP}", "32bb");
        if hex.len() % 2 != 0 {
            return Err(invalid("odd hex length"));
        }
        let clean: String = hex
            .chars()
            .map(|c| if "_HLWXY".contains(c) { '0' } else { c })
            .collect();
        for pair in clean.as_bytes().chunks_exact(2) {
            let word = std::str::from_utf8(pair).map_err(|_| invalid("non-ASCII packet"))?;
            blob.push(u8::from_str_radix(word, 16).map_err(|_| invalid("bad packet hex"))?);
        }
        lengths.push(clean.len() / 2);
    }
    for (id, glyph) in atlas.glyphs.iter().enumerate() {
        let (character, small) = atlas.address(id as u8);
        let offsets = if small {
            &map.small_glyph_offsets
        } else {
            &map.large_glyph_offsets
        };
        let offset = offsets
            .iter()
            .find(|entry| entry.character == character)
            .ok_or_else(|| invalid(format!("unmapped glyph {character}")))?;
        let expanded = expand_offsets(&offset.glyph_map)?;
        if expanded.len() != glyph.bitmap.len() {
            return Err(invalid(format!("glyph {character} byte count mismatch")));
        }
        for (&offset, &value) in expanded.iter().zip(&glyph.bitmap) {
            set(&mut blob, offset, value)?;
        }
    }
    // Same origin as the source driver's centered 23x29 MCDU layout.
    set(&mut blob, map.x_offset_offset, 0x34)?;
    set(&mut blob, map.y_offset_offset, 0x25)?;
    set(&mut blob, map.display_brightness_offset, brightness)?;
    for &offset in &map.glyph_width_offsets {
        set(&mut blob, offset, CELL_W as u8)?;
    }
    for &offset in &map.glyph_height_offsets {
        set(&mut blob, offset, CELL_H as u8)?;
    }
    let mut packets = Vec::with_capacity(lengths.len());
    let mut start = 0;
    for length in lengths {
        packets.push(blob[start..start + length].to_vec());
        start += length;
    }
    Ok(packets)
}

/// Remove only the captured grid-position/dimension transaction. Font uploads
/// contain this command too, so omitting a later grid command alone is insufficient.
pub fn preserve_grid(packets: &mut Vec<Vec<u8>>) {
    packets.retain(|packet| {
        !(packet.len() >= 46
            && packet[0] == 0xf0
            && packet[1] == 0
            && packet[3] == 42
            && packet[4..10] == [0x32, 0xbb, 0, 0, 0x18, 1])
    });
}

fn set(blob: &mut [u8], offset: usize, value: u8) -> Result<(), ProtocolError> {
    *blob
        .get_mut(offset)
        .ok_or_else(|| invalid("offset outside packet blob"))? = value;
    Ok(())
}
fn expand_offsets(map: &[i32]) -> Result<Vec<usize>, ProtocolError> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < map.len() {
        if map[index] == -1 {
            if index + 2 >= map.len() || map[index + 1] < 0 || map[index + 2] < map[index + 1] {
                return Err(invalid("bad offset range"));
            }
            result.extend((map[index + 1]..=map[index + 2]).map(|n| n as usize));
            index += 3;
        } else {
            if map[index] < 0 {
                return Err(invalid("negative offset"));
            }
            result.push(map[index] as usize);
            index += 1;
        }
    }
    Ok(result)
}
