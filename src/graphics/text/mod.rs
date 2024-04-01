#![allow(unused)]

use crate::geometry::{Extent, Pixel, Texel};

use super::{gl::TextureId, Color, DrawList};

pub enum Error {
    GlyphCacheFull,
}

pub struct TextEngine {}

impl TextEngine {
    pub fn new(glyph_cache_texture: TextureId, glyph_cache_texture_size: Extent<Texel>) -> Self {
        Self {}
    }

    pub fn get_font(&self, name: &str) -> Option<Font> {
        None
    }

    pub fn layout_text(&self, layout: LayoutOptions) -> TextLayout {
        TextLayout {}
    }

    pub fn draw(&self, layout: &TextLayout, draw_list: &mut DrawList) -> Result<(), Error> {
        // also need position, and a way to rasterize new glyphs

        todo!()
    }

    pub fn glyph_cache_compact(&self) {
        todo!()
    }

    pub fn glyph_cache_add_texture(&self, texture: TextureId, size: Extent<Texel>) {
        todo!()
    }
}

enum Weight {
    Light,
    Normal,
    Bold,
}

enum Style {
    Normal,
    Italic,
    Oblique,
}

enum WrapMode {
    None,
    Word,
    Character,
}

pub struct Font {}

impl Font {}

pub struct TextLayout {}

impl TextLayout {}

struct RichText {
    block: BlockOptions,
    spans: Vec<SpanOptions>, // should be small vec
}

impl RichText {
    pub fn layout_options(&self) -> LayoutOptions {
        LayoutOptions {
            text: "",
            block: &self.block,
            spans: &self.spans,
        }
    }
}

struct SimpleText {
    block: BlockOptions,
    span: SpanOptions,
}

impl SimpleText {
    pub fn layout_options(&self) -> LayoutOptions {
        LayoutOptions {
            text: "",
            block: &self.block,
            spans: std::slice::from_ref(&self.span),
        }
    }
}

pub struct LayoutOptions<'a> {
    text: &'a str,
    block: &'a BlockOptions,
    spans: &'a [SpanOptions],
}

pub struct BlockOptions {
    wrap: WrapMode,
    size: Extent<Pixel>,
    line_spacing: f32,
}

pub struct SpanOptions {
    start: u16,
    bytes: u16,
    font: Font,
    style: Style,
    weight: Weight,
    size: f32,
    color: Color,
}

pub struct GlyphCacheUpdates<'a> {
    buffer: &'a [u8],
    updates: &'a [GlyphCacheUpdate],
}

pub struct GlyphCacheUpdate {
    texture_id: TextureId,
    byte_offset: u32,
    size: Extent<Texel>,
}
