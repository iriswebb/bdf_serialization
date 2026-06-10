use crate::{BdfFont, BdfGlyph};
use embedded_graphics::{
    prelude::*,
    primitives::Rectangle,
    text::{
        renderer::{CharacterStyle, TextMetrics, TextRenderer},
        Baseline,
    },
};

/// A proportional font
pub trait ProportionalFont<'a>: Clone {
    /// Returns the global font ascent
    fn ascent(&self) -> u16;

    /// Returns the global font descent
    fn descent(&self) -> u16;

    /// Returns index of the replacement character
    fn replacement(&self) -> u32;

    /// Data is indexed from the start of the data block, not the start of the font
    ///
    /// Example: `glyph.draw(position, self.color, &self.font.data[self.font.data_offset()..], target)?;`
    fn data_offset(&self) -> usize;

    /// Returns a slice of the binary bitmap data
    fn data(&self) -> &'a [u8];

    /// Finds the BDF glyph corresponding to a character
    fn lookup(&self, c: char) -> BdfGlyph;

    /// Returns the baseline offset
    fn baseline_offset(&self, baseline: Baseline) -> i32 {
        match baseline {
            Baseline::Top => self.ascent().saturating_sub(1) as i32,
            Baseline::Bottom => -(self.descent() as i32),
            Baseline::Middle => (self.ascent() as i32 - self.descent() as i32) / 2,
            Baseline::Alphabetic => 0,
        }
    }

    /// Returns the default line height
    fn line_height(&self) -> u32 {
        (self.ascent() + self.descent()) as u32
    }
}

impl<'a> ProportionalFont<'a> for BdfFont<'a> {
    fn ascent(&self) -> u16 {
        self.ascent as u16
    }

    fn descent(&self) -> u16 {
        self.descent as u16
    }

    fn replacement(&self) -> u32 {
        self.replacement_character as u32
    }

    fn data_offset(&self) -> usize {
        0
    }

    fn data(&self) -> &'a [u8] {
        self.data
    }

    fn lookup(&self, c: char) -> BdfGlyph {
        *self
            .glyphs
            .iter()
            .find(|g| g.character == c)
            // TODO: don't panic if replacement_character is invalid
            .unwrap_or_else(|| &self.glyphs[self.replacement_character])
    }
}

/// A generalized text style for proportional fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProportionalTextStyle<'a, F: ProportionalFont<'a>, C: PixelColor> {
    font: &'a F,
    color: C,
}

impl<'a, F: ProportionalFont<'a>, C: PixelColor> ProportionalTextStyle<'a, F, C> {
    /// Creates a new text style
    pub fn new(font: &'a F, color: C) -> Self {
        Self { font, color }
    }
}

impl<'a, C: PixelColor, F: ProportionalFont<'a>> CharacterStyle
    for ProportionalTextStyle<'a, F, C>
{
    type Color = C;

    fn set_text_color(&mut self, text_color: Option<C>) {
        // TODO: support transparent text
        if let Some(color) = text_color {
            self.color = color;
        }
    }

    // TODO: implement additional methods
}

impl<'a, C: PixelColor, F: ProportionalFont<'a>> TextRenderer for ProportionalTextStyle<'a, F, C> {
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
        let mut position = position + Point::new(0, self.font.baseline_offset(baseline));

        for c in text.chars() {
            let glyph = self.font.lookup(c);

            glyph.draw(
                position,
                self.color,
                &self.font.data()[self.font.data_offset()..],
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
        let position = position + Point::new(0, self.font.baseline_offset(baseline));

        Ok(position + Size::new(width, 0))
    }

    fn measure_string(&self, text: &str, position: Point, baseline: Baseline) -> TextMetrics {
        let position = position + Point::new(0, self.font.baseline_offset(baseline));

        let dx = text.chars().map(|c| self.font.lookup(c).device_width).sum();

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
        self.font.line_height()
    }
}
