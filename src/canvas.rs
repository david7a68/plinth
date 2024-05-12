use crate::{
    core::arena::Arena,
    geometry::Rect,
    graphics::{Color, DrawList, Graphics, RenderTarget, RoundRect},
    system::Dpi,
    text::{GlyphCache, Text, TextDirection, TextEngine, TextStyle},
};

pub struct Canvas<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) scale: Dpi,
    pub(crate) target: &'a RenderTarget,
    pub(crate) graphics: &'a Graphics,
    pub(crate) draw_list: &'a mut DrawList,
    pub(crate) text_engine: &'a TextEngine,
    pub(crate) glyph_cache: &'a mut GlyphCache,
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

    pub fn draw_text(&mut self, text: &str, style: &TextStyle, rect: &Rect) {
        let text = Text::from_str(self.arena, text, "en-us", TextDirection::LeftToRight);

        let layout = self
            .text_engine
            .compute_layout(&text, rect.extent, self.scale, style);

        self.text_engine.draw_layout(
            self.arena,
            self.graphics,
            &layout,
            rect.origin,
            Color::BLACK,
            self.glyph_cache,
            self.draw_list,
        );
    }

    pub fn finish(&mut self) {
        self.draw_list.close();
    }
}
