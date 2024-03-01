use crate::geometry::pixel::Rect;

use super::Color;

pub struct RoundRect {
    pub rect: Rect,
    pub color: Color,
}

impl RoundRect {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            color: Color::BLACK,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}
