use core::panic;

use crate::{
    core::arena::Arena,
    geometry::Rect,
    graphics::{color::Color, primitives::RoundRect, texture_atlas::CachedTextureId, UvRect},
    system::DpiScale,
};

use super::{text::TextEngine, texture_atlas::TextureCache, FontOptions, TextBox};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextureFilter {
    #[default]
    Point,
    Linear,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sampler {
    pub filter: TextureFilter,
}

#[repr(C)]
#[repr(align(16))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RRect {
    pub xywh: [f32; 4],
    pub uvwh: [f32; 4],
    pub color: [f32; 4],
    pub texture_id: u32,
    pub sampler: Sampler,
}

impl RRect {
    pub fn new(rect: &RoundRect, uvwh: &UvRect, texture_id: u32, sampler: Sampler) -> Self {
        Self {
            xywh: rect.rect.to_xywh(),
            uvwh: uvwh.to_uvwh(),
            color: rect.color.to_array_f32(),
            texture_id,
            sampler,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Begin(Rect),
    Close,
    Clear(Color),
    Rects(u32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DrawCommand {
    Begin,
    Close,
    Clear,
    Rects,
}

#[derive(Debug, Default)]
#[repr(align(16))]
pub struct DrawList {
    pub(super) prims: Vec<RRect>,
    pub(super) areas: Vec<Rect>,
    pub(super) colors: Vec<Color>,
    pub(super) commands: Vec<(DrawCommand, u32)>,
}

impl DrawList {
    #[must_use]
    pub fn new() -> Self {
        Self {
            prims: Vec::new(),
            areas: Vec::new(),
            colors: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.prims.clear();
        self.areas.clear();
        self.colors.clear();
        self.commands.clear();
    }

    pub(super) fn iter(&self) -> DrawIter<'_> {
        DrawIter {
            areas: &self.areas,
            colors: &self.colors,
            commands: &self.commands,
            index: 0,
            draws: 0,
        }
    }
}

impl<'a> IntoIterator for &'a DrawList {
    type Item = Command;
    type IntoIter = DrawIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct DrawIter<'a> {
    areas: &'a [Rect],
    colors: &'a [Color],
    commands: &'a [(DrawCommand, u32)],
    index: usize,
    draws: usize,
}

impl Iterator for DrawIter<'_> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.commands.len() {
            return None;
        }

        let cmd = match self.commands[self.index] {
            (DrawCommand::Begin, area_i) => {
                debug_assert_eq!(area_i, 0);
                Command::Begin(self.areas[0])
            }
            (DrawCommand::Close, _) => Command::Close,
            (DrawCommand::Clear, color_i) => Command::Clear(self.colors[color_i as usize]),
            (DrawCommand::Rects, count) => {
                self.draws += 1;
                Command::Rects(count)
            }
        };

        self.index += 1;
        Some(cmd)
    }
}

pub struct Canvas<'a> {
    arena: &'a mut Arena,
    text_engine: &'a TextEngine,
    textures: &'a TextureCache,
    draw_list: &'a mut DrawList,
    region: Rect,
    rect_batch_start: usize,
    rect_batch_count: usize,
    state: DrawCommand,
    dpi: DpiScale,
}

impl<'a> Canvas<'a> {
    pub(super) fn new(
        textures: &'a TextureCache,
        text_engine: &'a TextEngine,
        arena: &'a mut Arena,
        draw_list: &'a mut DrawList,
        region: Rect,
        dpi: DpiScale,
    ) -> Self {
        draw_list.clear();
        draw_list.areas.push(region);
        draw_list.commands.push((DrawCommand::Begin, 0));

        Self {
            arena,
            text_engine,
            textures,
            draw_list,
            region,
            rect_batch_start: 0,
            rect_batch_count: 0,
            state: DrawCommand::Begin,
            dpi,
        }
    }

    #[must_use]
    pub fn region(&self) -> Rect {
        self.region
    }

    pub fn clear(&mut self, color: Color) {
        match self.state {
            DrawCommand::Begin | DrawCommand::Clear => {}
            DrawCommand::Rects => self.submit_batch(),
            DrawCommand::Close => panic!("Canvas state Close -> Clear is a bug."),
        }

        self.draw_list
            .commands
            .push((DrawCommand::Clear, self.draw_list.colors.len() as u32));

        self.draw_list.colors.push(color);
        self.state = DrawCommand::Clear;
    }

    pub fn draw_rect(&mut self, rect: &RoundRect) {
        match self.state {
            DrawCommand::Begin => {
                debug_assert_eq!(self.rect_batch_start, 0);
                debug_assert_eq!(self.rect_batch_count, 0);
                debug_assert_eq!(self.draw_list.prims.len(), 0);
            }
            DrawCommand::Clear => {
                debug_assert_eq!(self.rect_batch_count, 0);
                self.rect_batch_start = self.draw_list.prims.len();
            }
            DrawCommand::Rects => {} // no-op
            DrawCommand::Close => panic!("Canvas state Close -> DrawRect is a bug."),
        }

        let cache_id = CachedTextureId::new(rect.image.key.index(), rect.image.key.epoch());
        let (texture_id, uvwh) = self.textures.get_uv_rect(cache_id);

        self.draw_list.prims.push(RRect::new(
            rect,
            &uvwh,
            texture_id.index(),
            Sampler::default(),
        ));

        self.rect_batch_count += 1;

        self.state = DrawCommand::Rects;
    }

    pub fn draw_text(&mut self, text: &str, font: FontOptions, rect: TextBox) {
        let layout = self
            .text_engine
            .layout_text(self.arena, text, rect, font, self.dpi);

        layout.draw(self.draw_list);
    }

    pub fn finish(&mut self) {
        match self.state {
            DrawCommand::Begin | DrawCommand::Clear => {}
            DrawCommand::Rects => self.submit_batch(),
            DrawCommand::Close => return,
        }

        self.draw_list.commands.push((DrawCommand::Close, 0));
        self.state = DrawCommand::Close;
    }

    fn submit_batch(&mut self) {
        self.draw_list
            .commands
            .push((DrawCommand::Rects, self.rect_batch_count as u32));

        self.rect_batch_start = self.draw_list.prims.len();
        self.rect_batch_count = 0;
    }
}
