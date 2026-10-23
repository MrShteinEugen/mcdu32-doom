// SPDX-License-Identifier: GPL-3.0-only
use crate::display::{CELL_H, CELL_W, Screen, atlas::Atlas};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid font map: {0}")]
    InvalidMap(String),
    #[error("invalid screen cell {0}")]
    InvalidCell(usize),
}

/// IDs 0xf0 and 0xf2 are report IDs, not an extra USB payload byte.
pub fn screen_packets(atlas: &Atlas, screen: &Screen) -> Result<Vec<Vec<u8>>, ProtocolError> {
    encode_cells(atlas, screen)
}

/// Encode a logical screen for simulation. Nonstandard lengths must not be sent
/// to hardware without separately validating its grid configuration.
pub fn encode_cells(
    atlas: &Atlas,
    screen: &[crate::display::Cell],
) -> Result<Vec<Vec<u8>>, ProtocolError> {
    if screen.is_empty() {
        return Err(ProtocolError::InvalidCell(0));
    }
    let mut data = Vec::with_capacity(1200);
    for (index, cell) in screen.iter().enumerate() {
        if cell.glyph as usize >= atlas.glyphs.len() || cell.color > 10 {
            return Err(ProtocolError::InvalidCell(index));
        }
        let (character, small) = atlas.address(cell.glyph);
        let mut attributes = u16::from(cell.color) * 0x21 + if small { 0x16b } else { 0 };
        if index == 0 {
            attributes += 1;
        }
        if index == screen.len() - 1 {
            attributes += 2;
        }
        data.extend(attributes.to_le_bytes());
        let mut utf8 = [0; 4];
        data.extend(character.encode_utf8(&mut utf8).as_bytes());
    }
    Ok(data
        .chunks(63)
        .map(|chunk| {
            let mut packet = vec![0; 64];
            packet[0] = 0xf2;
            packet[1..1 + chunk.len()].copy_from_slice(chunk);
            packet
        })
        .collect())
}

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

/// Patch the captured, licensed font transaction only at documented bitmap/geometry offsets.
pub fn font_packets(atlas: &Atlas, brightness: u8) -> Result<Vec<Vec<u8>>, ProtocolError> {
    captured_font_packets(atlas, brightness)
}

/// Compact geometry with rebuilt 32-byte records (two bytes per row), visually
/// confirmed with the static 48x28 probe on Captain V1.06.
pub fn font_packets_for_grid(
    atlas: &Atlas,
    brightness: u8,
    dense: bool,
) -> Result<Vec<Vec<u8>>, ProtocolError> {
    if dense {
        return crate::device::font::rebuild(
            atlas,
            11,
            14,
            crate::device::font::Packing::Tight,
            brightness,
        )
        .map(|upload| upload.packets)
        .map_err(|error| invalid(format!("{error:#}")));
    }
    font_packets(atlas, brightness)
}

/// Use the same font packing and captured geometry transaction as the working
/// probe. Remove embedded grid commands before applying the chosen geometry.
pub fn display_setup(
    atlas: &Atlas,
    geometry: crate::display::Geometry,
    legacy_font: bool,
    keep_grid: bool,
) -> Result<Vec<Vec<u8>>, ProtocolError> {
    let mut packets = if geometry.compact() && !legacy_font {
        crate::device::font::rebuild(
            atlas,
            geometry.glyph_width as usize,
            geometry.glyph_height as usize,
            crate::device::font::Packing::Tight,
            200,
        )
        .map_err(|error| invalid(format!("{error:#}")))?
        .packets
    } else {
        font_packets(atlas, 200)?
    };
    preserve_grid(&mut packets);
    if !keep_grid {
        packets.extend(layout_packets(atlas, geometry, Layout::Captured)?);
    }
    Ok(packets)
}

fn captured_font_packets(atlas: &Atlas, brightness: u8) -> Result<Vec<Vec<u8>>, ProtocolError> {
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

/// Text-grid block (0x0118) followed by COMMIT, following the source driver's
/// setScreenPosition transaction. Row/column expansion still needs device validation.
pub fn grid_packet(dense: bool) -> [u8; 64] {
    let mut packet = [0; 64];
    packet[..4].copy_from_slice(&[0xf0, 0, 0, 0x2a]);
    packet[4..8].copy_from_slice(&[0x32, 0xbb, 0, 0]);
    packet[8..12].copy_from_slice(&[0x18, 1, 0, 0]);
    packet[17] = 8;
    packet[21] = if dense { 64 } else { 0x34 };
    packet[23] = if dense { 44 } else { 0x25 };
    packet[25] = if dense { 28 } else { 14 };
    packet[27] = if dense { 48 } else { 24 };
    packet[29..33].copy_from_slice(&[0x32, 0xbb, 0, 0]);
    packet[33..37].copy_from_slice(&[5, 1, 0, 0]);
    packet[41] = 1;
    packet
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

fn invalid(message: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidMap(message.into())
}

/// Paint the panel's entire LCD black, including pixels outside the text grid.
/// Observed 0x0104 BLACK command from the GPLv3 WINCTRL X-Plane reference,
/// ProductFMC::showBackground (see THIRD_PARTY.md). Does not change firmware.
pub fn black_background_packets() -> Vec<Vec<u8>> {
    let mut packet = vec![0; 64];
    packet[..22].copy_from_slice(&[
        0xf0, 0x00, 0x03, 0x12, 0x32, 0xbb, 0, 0, 0x04, 0x01, 0, 0, 0xfd, 0x24, 0x07, 0, 0, 1, 0,
        0, 0, 0x0e,
    ]);
    let mut packets = vec![packet];
    packets.extend((0..3).map(|_| vec![0xf0, 1, 0, 0]));
    packets
}

/// Set only the FM annunciator (LED 9), preserving backlight and other lamps.
/// WINCTRL 0x0349 LED command, as documented by the GPLv3 protocol reference.
pub fn mcdu_fm_led(enabled: bool) -> [u8; 64] {
    let mut report = [0; 64];
    report[..14].copy_from_slice(&[
        0x02,
        0x32,
        0xbb,
        0,
        0,
        0x03,
        0x49,
        9,
        u8::from(enabled),
        0,
        0,
        0,
        0,
        0,
    ]);
    report
}

/// Enable annunciator brightness independently of keyboard/display backlights.
/// Binary lamp commands alone stay invisible if a previous client left it at 0.
pub fn annunciator_brightness() -> [u8; 64] {
    let mut report = mcdu_fm_led(false);
    report[7] = 2;
    report[8] = 255;
    report
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::Cell;
    #[test]
    fn fm_annunciator_command_sets_only_led_nine() {
        let off = mcdu_fm_led(false);
        let on = mcdu_fm_led(true);
        assert_eq!(&on[..8], &[2, 0x32, 0xbb, 0, 0, 3, 0x49, 9]);
        assert_eq!(on[8], 1);
        assert_eq!(off[8], 0);
        assert!(
            on.iter()
                .zip(off)
                .enumerate()
                .all(|(i, (a, b))| i == 8 || *a == b)
        );
        assert!(on[9..].iter().all(|b| *b == 0));
    }
    #[test]
    fn full_lcd_erase_is_independent_of_text_grid_and_font() {
        let packets = black_background_packets();
        assert_eq!(packets[0].len(), 64);
        assert_eq!(packets[0][3], 18);
        assert_eq!(&packets[0][4..12], &[0x32, 0xbb, 0, 0, 4, 1, 0, 0]);
        assert_eq!(&packets[0][16..22], &[0, 1, 0, 0, 0, 14]);
        assert!(packets[0][22..].iter().all(|b| *b == 0));
        assert!(packets[1..].iter().all(|p| p == &[0xf0, 1, 0, 0]));
    }
    #[test]
    fn preserving_user_grid_removes_only_captured_layout_transaction() {
        let original = font_packets(&Atlas::universal(), 200).expect("font");
        let mut preserved = original.clone();
        preserve_grid(&mut preserved);
        assert_eq!(preserved.len() + 1, original.len());
        let filtered: Vec<_> = original
            .into_iter()
            .filter(|p| p[4..10] != [0x32, 0xbb, 0, 0, 0x18, 1])
            .collect();
        assert_eq!(preserved, filtered);
        assert!(preserved.iter().all(|p| !p.is_empty() && p.len() <= 64));
    }
    #[test]
    fn dense_gameplay_uses_rebuilt_compact_records() {
        let packets = font_packets_for_grid(&Atlas::universal(), 200, true).expect("compact");
        assert_eq!(&packets[0][25..29], &[11, 0, 14, 0]);
        assert_eq!(&packets[0][29..33], &32_u32.to_le_bytes());
        assert!(
            packets.len()
                < font_packets(&Atlas::universal(), 200)
                    .expect("baseline")
                    .len()
        );
    }

    #[test]
    fn full_display_setup_matches_confirmed_font_and_grid_transaction() {
        let packets = display_setup(
            &Atlas::universal(),
            crate::display::Profile::Full.geometry(),
            false,
            false,
        )
        .expect("setup");
        assert_eq!(&packets[0][25..33], &[11, 0, 14, 0, 32, 0, 0, 0]);
        let grid: Vec<_> = packets
            .iter()
            .filter(|p| p.len() >= 46 && p[8..12] == [0x18, 1, 0, 0])
            .collect();
        assert_eq!(grid.len(), 1);
        assert_eq!(&grid[0][21..29], &[36, 0, 23, 0, 31, 0, 53, 0]);
        assert_ne!(&grid[0][12..16], &[0; 4]);
        assert_eq!(&packets[packets.len() - 1][..2], &[0xf0, 1]);
    }

    #[test]
    fn grid_transaction_encodes_dimensions_and_commit() {
        let dense = grid_packet(true);
        assert_eq!(&dense[..4], &[0xf0, 0, 0, 42]);
        assert_eq!(&dense[25..29], &[28, 0, 48, 0]);
        assert_eq!(&dense[33..37], &[5, 1, 0, 0]);
        assert_eq!(dense[41], 1);
        assert_eq!(&grid_packet(false)[25..29], &[14, 0, 24, 0]);
    }
    #[test]
    fn all_220_glyphs_map_and_upload() {
        let atlas = Atlas::universal();
        let packets = font_packets(&atlas, 180).expect("captured map");
        assert!(packets.len() > 300);
        assert!(packets.iter().all(|p| !p.is_empty() && p.len() <= 64));
    }
    #[test]
    fn ascii_frame_boundaries_and_unicode() {
        let atlas = Atlas::universal();
        let mut screen = [Cell {
            glyph: 30,
            color: 2,
        }; 336];
        let packets = screen_packets(&atlas, &screen).expect("screen");
        assert_eq!(packets.len(), 16);
        assert_eq!(&packets[0][1..3], &0x43_u16.to_le_bytes());
        let stream: Vec<_> = packets
            .iter()
            .flat_map(|p| p[1..].iter().copied())
            .collect();
        assert_eq!(&stream[1005..1007], &0x44_u16.to_le_bytes());
        screen[0].glyph = 109;
        assert_eq!(screen_packets(&atlas, &screen).expect("unicode").len(), 17);
    }
    #[test]
    fn invalid_cell_returns_error() {
        let atlas = Atlas::universal();
        let screen = [Cell {
            glyph: 255,
            color: 0,
        }; 336];
        assert!(screen_packets(&atlas, &screen).is_err());
    }
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    /// Patch the captured 0x0118 transaction, retaining its opaque address.
    #[default]
    Captured,
    /// The standalone zero-address form used by the X-Plane driver.
    Standalone,
    /// Prefix 0x0118 with the observed 0x011e initialization command.
    Begin,
}

pub fn layout_packets(
    atlas: &Atlas,
    grid: crate::display::Geometry,
    mode: Layout,
) -> Result<Vec<Vec<u8>>, ProtocolError> {
    let capture = font_packets(atlas, 200)?;
    let mut packet = capture
        .iter()
        .find(|p| p.len() >= 46 && p[..2] == [0xf0, 0] && p[3] == 42 && p[8..12] == [0x18, 1, 0, 0])
        .ok_or_else(|| invalid("missing captured grid transaction"))?
        .clone();
    packet[21..23].copy_from_slice(&grid.left.to_le_bytes());
    packet[23..25].copy_from_slice(&grid.top.to_le_bytes());
    packet[25..27].copy_from_slice(&grid.rows.to_le_bytes());
    packet[27..29].copy_from_slice(&grid.columns.to_le_bytes());
    if matches!(mode, Layout::Standalone) {
        packet[12..16].fill(0);
        packet[37..41].fill(0);
    }
    let mut result = if matches!(mode, Layout::Begin) {
        let mut begin = packet[4..25].to_vec();
        begin[4] = 0x1e;
        begin[13..17].fill(0);
        begin.truncate(21);
        begin.extend_from_slice(&packet[4..46]);
        begin
            .chunks(60)
            .enumerate()
            .map(|(i, data)| {
                let mut p = vec![0; 64];
                p[..4].copy_from_slice(&[0xf0, 0, i as u8, data.len() as u8]);
                p[4..4 + data.len()].copy_from_slice(data);
                p
            })
            .collect()
    } else {
        vec![packet]
    };
    result.extend((0..3).map(|_| vec![0xf0, 1, 0, 0]));
    Ok(result)
}
