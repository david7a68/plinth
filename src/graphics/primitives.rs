use crate::math::Rect;

use super::Color;

pub struct RoundRect<U> {
    pub rect: Rect<f32, U>,
    pub color: Color,
}

impl<U> RoundRect<U> {
    #[must_use]
    pub fn builder(rect: Rect<f32, U>) -> RoundRectBuilder<U> {
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
    rect: Rect<f32, U>,
    color: Color,
}

impl<U> RoundRectBuilder<U> {
    #[must_use]
    pub fn new(rect: Rect<f32, U>) -> Self {
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
