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
pub trait ProportionalFont<'a>{
    // returns a struct containing ascent, descent, baseline_offset, and line_height
    fn metrics(&self) -> &Metrics;
    
    // There are two options on how to handle the replacement glyph.
    // Either return `None` if `c` is not found and let the renderer handle the replacement glyph:

    // note the added lifetime, see changed `BdfGlyph` below
    fn lookup(&self, c: char) -> Option<BdfGlyph<'_>>;
    fn replacement_glyph(&self) -> BdfGlyph<'_>;
    
    // Or return the replacement glyph as part of `lookup`. `replacement_glyph` should then be unnecessary.
    fn lookup(&self, c: char) -> BdfGlyph<'_>;
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
