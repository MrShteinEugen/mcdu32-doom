// SPDX-License-Identifier: GPL-3.0-only
//! Bounded text copied from DOOM's UI over the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextLine {
    pub x: i32,
    pub y: i32,
    pub color: i32,
    pub bytes: [u8; 128],
}
impl Default for TextLine {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            color: 2,
            bytes: [0; 128],
        }
    }
}
impl TextLine {
    pub fn value(&self) -> String {
        let end = self
            .bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.bytes.len());
        String::from_utf8_lossy(&self.bytes[..end]).into_owned()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TextFrame {
    pub count: i32,
    pub menu: i32,
    pub lines: [TextLine; 32],
}
impl TextFrame {
    pub fn lines(&self) -> &[TextLine] {
        &self.lines[..self.count.clamp(0, 32) as usize]
    }
}
