use crate::BdfGlyph;
use embedded_graphics::{
    prelude::*,
    primitives::Rectangle,
    text::{
        renderer::{TextMetrics, TextRenderer},
        Baseline,
    },
};

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
impl SerializedBdfFont<'_> {
    /// Returns the global font ascent
    pub const fn ascent(self) -> u16 {
        u16::from_be_bytes([self.data[0], self.data[1]])
    }
    /// Returns the global font descent
    pub const fn descent(self) -> u16 {
        u16::from_be_bytes([self.data[2], self.data[3]])
    }
    /// Returns index of the replacement character
    pub const fn replacement(self) -> u32 {
        u32::from_be_bytes([self.data[4], self.data[5], self.data[6], self.data[7]])
    }
    /// Returns the length of the glyph table
    pub const fn character_count(self) -> u32 {
        u32::from_be_bytes([self.data[8], self.data[9], self.data[10], self.data[11]])
    }
    /// Data is indexed from the start of the data block, not the start of the font
    ///
    /// Example: `glyph.draw(position, self.color, &self.font.data[self.font.data_offset()..], target)?;`
    pub const fn data_offset(self) -> usize {
        // Header + Character Table
        (12 + (self.character_count() * 17)) as usize
    }

    /// Returns a BdfGlyph in the glyph table
    pub const fn character_table(self, idx: u32) -> BdfGlyph {
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

        BdfGlyph {
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
            start_index: data_index as usize,
        }
    }
}
impl<'a> SerializedBdfFont<'a> {
    /// Searches for the glyph
    ///
    /// TODO: Make the search faster by not constructing a full glyph, and instead checking the corresponding character field in the entry
    pub fn get_glyph(self, c: char) -> BdfGlyph {
        for i in 0..self.character_count() {
            let tested_character = self.character_table(i);
            if self.character_table(i).character == c {
                return tested_character;
            }
        }

        self.character_table(self.replacement())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Serialized BDF Text Renderer with Color
pub struct SerializedBdfTextStyle<'a, C> {
    font: &'a SerializedBdfFont<'a>,
    color: C,
}

impl<'a, C: PixelColor> SerializedBdfTextStyle<'a, C> {
    /// Creates a new character style.
    pub fn new(font: &'a SerializedBdfFont<'a>, color: C) -> Self {
        Self { font, color }
    }

    fn baseline_offset(&self, baseline: Baseline) -> i32 {
        match baseline {
            Baseline::Top => self.font.ascent().saturating_sub(1) as i32,
            Baseline::Bottom => -(self.font.descent() as i32),
            Baseline::Middle => (self.font.ascent() as i32 - self.font.descent() as i32) / 2,
            Baseline::Alphabetic => 0,
        }
    }
}

impl<C: PixelColor> TextRenderer for SerializedBdfTextStyle<'_, C> {
    type Color = C;

    fn draw_string<D>(
        &self,
        text: &str,
        position: Point,
        baseline: Baseline,
        target: &mut D,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut position = position + Point::new(0, self.baseline_offset(baseline));

        for c in text.chars() {
            let glyph = self.font.get_glyph(c);

            glyph.draw(
                position,
                self.color,
                &self.font.data[self.font.data_offset()..],
                target,
            )?;

            position.x += glyph.device_width as i32;
        }

        Ok(position)
    }

    fn draw_whitespace<D>(
        &self,
        width: u32,
        position: Point,
        baseline: Baseline,
        _target: &mut D,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let position = position + Point::new(0, self.baseline_offset(baseline));

        Ok(position + Size::new(width, 0))
    }

    fn measure_string(&self, text: &str, position: Point, baseline: Baseline) -> TextMetrics {
        let position = position + Point::new(0, self.baseline_offset(baseline));

        let dx = text
            .chars()
            .map(|c| self.font.get_glyph(c).device_width)
            .sum();

        // TODO: calculate correct bounding box
        let bounding_box = Rectangle::new(
            position - Size::new(0, self.font.ascent().saturating_sub(1) as u32),
            Size::new(dx, self.line_height()),
        );

        TextMetrics {
            bounding_box,
            next_position: position + Size::new(dx, 0),
        }
    }

    fn line_height(&self) -> u32 {
        // TODO: add separate line height field?
        (self.font.ascent() + self.font.descent()) as u32
    }
}
