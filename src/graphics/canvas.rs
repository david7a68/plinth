use crate::{math::Rect, LogicalPixel};

use super::{Color, RoundRect};

pub trait Canvas {
    #[must_use]
    fn region(&self) -> Rect<u16, LogicalPixel>;

    fn clear(&mut self, color: Color);

    fn draw_rect(&mut self, rect: RoundRect);
}
