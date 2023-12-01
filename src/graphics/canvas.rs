use crate::math::Rect;

use super::{Color, DefaultColorSpace};

pub struct Canvas {}

impl Canvas {
    pub fn rect(&self) -> Rect<Self> {
        todo!()
    }

    pub fn clear(&mut self, color: Color<DefaultColorSpace>) {
        todo!()
    }

    pub fn draw_rect(&mut self, rect: impl Into<Rect<Self>>, color: Color<DefaultColorSpace>) {
        todo!()
    }
}
