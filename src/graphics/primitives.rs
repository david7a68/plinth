use crate::math::Rect;

use super::Color;

pub struct RoundRect<U> {
    pub rect: Rect<U>,
    pub color: Color,
}

impl<U> RoundRect<U> {
    #[must_use]
    pub fn builder(rect: Rect<U>) -> RoundRectBuilder<U> {
        RoundRectBuilder::new(rect)
    }

    #[must_use]
    pub fn retype<U2>(self) -> RoundRect<U2> {
        RoundRect {
            rect: self.rect.retype(),
            color: self.color,
        }
    }
}

impl<U> From<RoundRectBuilder<U>> for RoundRect<U> {
    fn from(value: RoundRectBuilder<U>) -> Self {
        value.build()
    }
}

pub struct RoundRectBuilder<U> {
    rect: Rect<U>,
    color: Color,
}

impl<U> RoundRectBuilder<U> {
    #[must_use]
    pub fn new(rect: Rect<U>) -> Self {
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
    pub fn build(self) -> RoundRect<U> {
        RoundRect {
            rect: self.rect,
            color: self.color,
        }
    }
}
