use crate::{
    math::Rect,
    platform::gfx::{DrawCommand, DrawList},
};

use super::{Color, RoundRect};

pub struct Canvas<'a, U> {
    bounds: Rect<U>,
    data: &'a mut DrawList,

    n_rects: u32,
}

impl<'a, U> Canvas<'a, U> {
    pub(crate) fn new(data: &'a mut DrawList, bounds: Rect<U>) -> Self {
        data.rects.clear();
        data.commands.clear();
        data.commands.push(DrawCommand::Begin);

        Self {
            bounds,
            data,
            n_rects: 0,
        }
    }

    pub(crate) fn finish(self) -> &'a mut DrawList {
        if self.n_rects < self.data.rects.len() as u32 {
            self.data.commands.push(DrawCommand::DrawRects {
                first: self.n_rects,
                count: self.data.rects.len() as u32 - self.n_rects,
            });
        }

        self.data.commands.push(DrawCommand::End);
        self.data
    }

    pub fn rect(&self) -> &Rect<U> {
        &self.bounds
    }

    pub fn clear(&mut self, color: Color) {
        self.data.commands.push(DrawCommand::Clear(color));
    }

    pub fn draw_rect(&mut self, rect: impl Into<RoundRect<U>>) {
        self.data.rects.push(rect.into().retype());
    }
}
