use crate::geometry::pixel::Rect;

use super::Color;

pub struct RoundRect {
    pub rect: Rect,
    pub color: Color,
}

impl RoundRect {
    #[must_use]
    pub fn builder(rect: Rect) -> RoundRectBuilder {
        RoundRectBuilder::new(rect)
    }
}

impl From<RoundRectBuilder> for RoundRect {
    fn from(value: RoundRectBuilder) -> Self {
        value.build()
    }
}

pub struct RoundRectBuilder {
    rect: Rect,
    color: Color,
}

impl RoundRectBuilder {
    #[must_use]
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            color: Color::BLACK,
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn build(self) -> RoundRect {
        RoundRect {
            rect: self.rect,
            color: self.color,
        }
    }
}
