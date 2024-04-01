use core::panic;

use crate::{
    geometry::{Pixel, Rect, UV},
    graphics::{backend::texture_atlas::CachedTextureId, color::Color, primitives::RoundRect},
    limits::GFX_DRAW_PRIM_COUNT,
};

use super::backend::texture_atlas::TextureCache;

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
    pub fn new(rect: &RoundRect, uvwh: &Rect<UV>, texture_id: u32, sampler: Sampler) -> Self {
        Self {
            xywh: rect.rect.to_xywh().map(|x| x.0),
            uvwh: uvwh.to_xywh().map(|x| x.0),
            color: rect.color.to_array_f32(),
            texture_id,
            sampler,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Begin(Rect<Pixel>),
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
    pub(super) areas: Vec<Rect<Pixel>>,
    pub(super) colors: Vec<Color>,
    pub(super) commands: Vec<(DrawCommand, u32)>,
}

impl DrawList {
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

pub(super) struct DrawIter<'a> {
    areas: &'a [Rect<Pixel>],
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
    textures: &'a TextureCache,
    draw_list: &'a mut DrawList,
    region: Rect<Pixel>,
    rect_batch_start: usize,
    rect_batch_count: usize,
    state: DrawCommand,
}

impl<'a> Canvas<'a> {
    pub(super) fn new(
        textures: &'a TextureCache,
        draw_list: &'a mut DrawList,
        region: Rect<Pixel>,
    ) -> Self {
        draw_list.clear();
        draw_list.areas.push(region);
        draw_list.commands.push((DrawCommand::Begin, 0));

        Self {
            textures,
            draw_list,
            region,
            rect_batch_start: 0,
            rect_batch_count: 0,
            state: DrawCommand::Begin,
        }
    }

    pub fn region(&self) -> Rect<Pixel> {
        self.region
    }

    pub fn clear(&mut self, color: Color) {
        match self.state {
            DrawCommand::Begin => {}
            DrawCommand::Clear => {} // todo: might be an error? maybe surface in log. -dz (2024-03-24)
            DrawCommand::Rects => self.submit_batch(),
            DrawCommand::Close => panic!("Canvas state Close -> Clear is a bug."),
        }

        GFX_DRAW_PRIM_COUNT.check(&self.draw_list.colors.len());

        self.draw_list
            .commands
            .push((DrawCommand::Clear, self.draw_list.colors.len() as u32));

        self.draw_list.colors.push(color);
        self.state = DrawCommand::Clear;
    }

    pub fn draw_rect(&mut self, rect: RoundRect) {
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
            &rect,
            &uvwh,
            texture_id.index(),
            Sampler::default(),
        ));

        GFX_DRAW_PRIM_COUNT.check(&self.rect_batch_count);

        self.rect_batch_count += 1;

        self.state = DrawCommand::Rects;
    }

    pub fn finish(&mut self) {
        match self.state {
            DrawCommand::Begin => {}
            DrawCommand::Clear => {}
            DrawCommand::Rects => self.submit_batch(),
            DrawCommand::Close => return,
        }

        self.draw_list.commands.push((DrawCommand::Close, 0));
        self.state = DrawCommand::Close;
    }

    pub fn skip_draw_and_finish(&mut self) {
        self.draw_list.clear();
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
