use crate::geometry::{Pixel, Rect};

use super::Color;

pub struct RoundRect {
    pub rect: Rect<Pixel>,
    pub color: Color,
}

impl RoundRect {
    #[must_use]
    pub fn new(rect: Rect<Pixel>) -> Self {
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
