use crate::geometry::Rect;

use super::{color::Color, image::Image};

pub struct RoundRect {
    pub rect: Rect,
    pub color: Color,
    pub image: Image,
}

impl RoundRect {
    #[must_use]
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            color: Color::WHITE,
            image: Image::default(),
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn with_image(mut self, image: Image) -> Self {
        self.image = image;
        self
    }
}
