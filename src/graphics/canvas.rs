use crate::{core::arena::Arena, geometry::Point, system::DpiScale};

use super::{Color, DrawList, FontOptions, RenderTarget, RoundRect, TextBox, TextLayout};

pub struct Canvas<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) scale: DpiScale,
    pub(crate) target: &'a RenderTarget,
    pub(crate) draw_list: &'a mut DrawList,
}

impl<'a> Canvas<'a> {
    pub(crate) fn begin(&mut self) {
        self.draw_list.reset();
        self.draw_list
            .begin(self.target.extent().into(), self.target.extent().into());
    }

    pub fn clear(&mut self, color: Color) {
        self.draw_list.clear(color);
    }

    pub fn draw_rect(&mut self, rect: &RoundRect) {
        self.draw_list.draw_prim(&rect.data);
    }

    pub fn draw_text(&mut self, text: &str, font: &FontOptions, area: &TextBox, at: Point) {
        /*
        let layout_id = self.layout_cache.get_or_create(font, area);
        */

        // self.draw_list.draw_chars(&layout, at);
    }

    pub fn draw_text_layout(&mut self, text: &TextLayout, at: Point) {
        self.draw_list.draw_chars(text, at);
    }

    pub fn finish(&mut self) {
        self.draw_list.close();
    }
}
