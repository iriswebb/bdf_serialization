use crate::{DisplayBdfGlyph, ProportionalFont, ProportionalTextStyle};
use embedded_graphics::{prelude::*, primitives::Rectangle};

/// * Header (12 Bytes):
/// -    Ascent  (pixels, u16 BE)
/// -    Descent (pixels, u16 BE)
/// -    Replacement Character (index into character table, u32 BE)
/// -    Character Table Length (entries, u32 BE)
///
/// * Glyph Table (17 Bytes Per Entry):
/// -    corresponding codepoint (u32 BE)
/// -    top_left.x (i16 BE)
/// -    top_left.y (i16 BE)
/// -    size.width (u16 BE)
/// -    size.height (u16 BE)
/// -    device_width (pixels, u8)
/// -    data index  (bytes from start of data, u32 BE)
///
/// Font bitmap data is stored afterwards
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SerializedBdfFont<'a> {
    /// The raw u8 data of the serialized font
    pub data: &'a [u8],
}
impl<'a> SerializedBdfFont<'a> {
    /// Returns the length of the glyph table
    pub const fn character_count(self) -> u32 {
        u32::from_be_bytes([self.data[8], self.data[9], self.data[10], self.data[11]])
    }

    /// Returns the offset of the data block
    fn data_index(self) -> usize {
        12 + (self.character_count() * 17) as usize
    }

    /// Returns a BdfGlyph in the glyph table
    pub fn character_table(self, idx: u32) -> DisplayBdfGlyph<'a> {
        let offset = 12 + (idx * 17) as usize;
        let corresponding_character = char::from_u32(u32::from_be_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]));
        let top_left_x = i16::from_be_bytes([self.data[offset + 4], self.data[offset + 5]]);
        let top_left_y = i16::from_be_bytes([self.data[offset + 6], self.data[offset + 7]]);
        let width = u16::from_be_bytes([self.data[offset + 8], self.data[offset + 9]]);
        let height = u16::from_be_bytes([self.data[offset + 10], self.data[offset + 11]]);
        let kerning = self.data[offset + 12];
        let data_index = u32::from_be_bytes([
            self.data[offset + 13],
            self.data[offset + 14],
            self.data[offset + 15],
            self.data[offset + 16],
        ]);

        DisplayBdfGlyph {
            character: corresponding_character.unwrap(),
            bounding_box: Rectangle {
                top_left: Point {
                    x: top_left_x as i32,
                    y: top_left_y as i32,
                },
                size: Size {
                    width: width as u32,
                    height: height as u32,
                },
            },
            device_width: kerning as u32,
            bitmap_data: &self.data[(self.data_index() + data_index as usize)..],
        }
    }
}
impl<'a> ProportionalFont<'a> for SerializedBdfFont<'a> {
    fn metrics(&self) -> crate::proportional::Metrics {
        crate::proportional::Metrics {
            ascent: u16::from_be_bytes([self.data[0], self.data[1]]) as u32,
            descent: u16::from_be_bytes([self.data[2], self.data[3]]) as u32,
            line_height: (u16::from_be_bytes([self.data[0], self.data[1]])
                + u16::from_be_bytes([self.data[2], self.data[3]])) as u32,
        }
    }

    fn replacement_glyph(&self) -> DisplayBdfGlyph<'_> {
        let rpos = u32::from_be_bytes([self.data[4], self.data[5], self.data[6], self.data[7]]);
        self.character_table(rpos)
    }

    fn lookup(&self, c: char) -> Option<DisplayBdfGlyph<'_>> {
        for i in 0..self.character_count() {
            let tested_character = self.character_table(i);
            if self.character_table(i).character == c {
                return Some(tested_character);
            }
        }

        None
    }
}

/// Stylized serialized BDF text
pub type SerializedBdfTextStyle<'a, C> = ProportionalTextStyle<'a, SerializedBdfFont<'a>, C>;
