use crate::{math::Rect, platform::gfx::DrawList};

use super::{Color, RoundRect};

pub struct Canvas<'a, U> {
    bounds: Rect<U>,
    data: &'a mut DrawList,
}

impl<'a, U> Canvas<'a, U> {
    #[must_use]
    pub(crate) fn new(data: &'a mut DrawList, bounds: Rect<U>) -> Self {
        data.reset();
        data.begin(bounds);

        Self { bounds, data }
    }

    #[must_use]
    pub(crate) fn finish(self) -> &'a mut DrawList {
        self.data.end();
        self.data
    }

    #[must_use]
    pub fn rect(&self) -> &Rect<U> {
        &self.bounds
    }

    pub fn clear(&mut self, color: Color) {
        self.data.clear(color);
    }

    pub fn draw_rect(&mut self, rect: impl Into<RoundRect<U>>) {
        self.data.draw_rect(rect);
    }
}
