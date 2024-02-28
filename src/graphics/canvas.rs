use crate::geometry::image;

use super::{Color, RoundRect};

pub trait Canvas {
    #[must_use]
    fn region(&self) -> image::Rect;

    fn clear(&mut self, color: Color);

    fn draw_rect(&mut self, rect: RoundRect);
}
