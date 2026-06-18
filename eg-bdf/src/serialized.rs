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
    data: &'a [u8],
}
impl<'a> SerializedBdfFont<'a> {
    // Structure Sizes
    const HEADER_SIZE: usize = 12;
    const CHARACTER_TABLE_ENTRY_SIZE: u32 = 17;

    // Offsets for the header
    const ASCENT_OFFSET: usize = 0;
    const DESCENT_OFFSET: usize = 2;
    const REPLACEMENT_OFFSET: usize = 4;
    const CHAR_TABLE_LEN_OFFSET: usize = 8;

    // Offsets for the character table entries
    const CODEPOINT_OFFSET: usize = 0;
    const TOPLEFTX_OFFSET: usize = 4;
    const TOPLEFTY_OFFSET: usize = 6;
    const SIZEX_OFFSET: usize = 8;
    const SIZEY_OFFSET: usize = 10;
    const KERN_OFFSET: usize = 12;
    const IDX_OFFSET: usize = 13;

    /// Constructs a serialized font without first verifing the data
    pub const fn from_unverified_data(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Verifies data in a way that prevents panics and returns a serialized font if the data is valid
    ///
    /// TODO: Make this const
    pub fn verify_data(data: &'a [u8]) -> Result<Self, &'static str> {
        // No header
        if data.len() < Self::HEADER_SIZE {
            return Err("No header");
        }

        // Character table length invalid
        let c_count = u16::from_be_bytes([
            data[Self::CHAR_TABLE_LEN_OFFSET],
            data[Self::CHAR_TABLE_LEN_OFFSET + 1],
        ]);

        let metadata_size: usize =
            Self::HEADER_SIZE + ((c_count as usize) * (Self::CHARACTER_TABLE_ENTRY_SIZE) as usize);

        // Character table too small
        if data.len() < metadata_size {
            return Err("No metadata");
        }

        // Safe to construct a font and index its metadata
        let font = Self { data };

        // Verify Each Entry
        for i in 0..font.character_count() {
            let offset = Self::HEADER_SIZE + (i * Self::CHARACTER_TABLE_ENTRY_SIZE) as usize;
            // Corresponding Character Invalid
            char::from_u32(font.get_be_u32(offset + Self::CODEPOINT_OFFSET))
                .ok_or("Invalid character")?;

            // Data index not within bitmap data
            let idx = font.get_be_u32(offset + Self::IDX_OFFSET) as usize;

            if idx > data.len() {
                return Err("Invalid bitmap index");
            }
        }

        // Data is okay
        Ok(font)
    }

    fn get_be_i16(&self, idx: usize) -> i16 {
        i16::from_be_bytes(self.data[idx..idx + 2].try_into().unwrap())
    }

    fn get_be_u16(&self, idx: usize) -> u16 {
        u16::from_be_bytes(self.data[idx..idx + 2].try_into().unwrap())
    }

    fn get_be_u32(&self, idx: usize) -> u32 {
        u32::from_be_bytes(self.data[idx..idx + 4].try_into().unwrap())
    }

    /// Returns the length of the glyph table
    pub fn character_count(self) -> u32 {
        self.get_be_u32(Self::CHAR_TABLE_LEN_OFFSET)
    }

    /// Returns the offset of the data block
    fn data_index(self) -> usize {
        Self::HEADER_SIZE + (self.character_count() * Self::CHARACTER_TABLE_ENTRY_SIZE) as usize
    }

    /// Returns a BdfGlyph in the glyph table
    pub fn character_table(self, idx: u32) -> DisplayBdfGlyph<'a> {
        let offset = Self::HEADER_SIZE + (idx * Self::CHARACTER_TABLE_ENTRY_SIZE) as usize;
        let corresponding_character =
            char::from_u32(self.get_be_u32(offset + Self::CODEPOINT_OFFSET));
        let top_left_x = self.get_be_i16(offset + Self::TOPLEFTX_OFFSET);
        let top_left_y = self.get_be_i16(offset + Self::TOPLEFTY_OFFSET);
        let width = self.get_be_u16(offset + Self::SIZEX_OFFSET);
        let height = self.get_be_u16(offset + Self::SIZEY_OFFSET);
        let kerning = self.data[offset + Self::KERN_OFFSET];
        let data_index = self.get_be_u32(offset + Self::IDX_OFFSET);

        DisplayBdfGlyph {
            character: corresponding_character.unwrap(),
            bounding_box: Rectangle {
                top_left: Point {
                    x: i32::from(top_left_x),
                    y: i32::from(top_left_y),
                },
                size: Size {
                    width: u32::from(width),
                    height: u32::from(height),
                },
            },
            device_width: u32::from(kerning),
            bitmap_data: &self.data[(self.data_index() + data_index as usize)..],
        }
    }
}
impl<'a> ProportionalFont<'a> for SerializedBdfFont<'a> {
    fn metrics(&self) -> crate::proportional::Metrics {
        crate::proportional::Metrics {
            ascent: self.get_be_u16(Self::ASCENT_OFFSET) as u32,
            descent: self.get_be_u16(Self::DESCENT_OFFSET) as u32,
            line_height: self.get_be_u16(Self::ASCENT_OFFSET) as u32
                + self.get_be_u16(Self::DESCENT_OFFSET) as u32,
        }
    }

    fn replacement_glyph(&self) -> DisplayBdfGlyph<'_> {
        let rpos = self.get_be_u32(Self::REPLACEMENT_OFFSET);
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
