use crate::{
    graphics::{Color, RoundRect},
    math::Rect,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubmitId(pub u64);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RRect {
    xywh: [f32; 4],
    color: [f32; 4],
}

impl<U> From<RoundRect<U>> for RRect {
    #[inline]
    fn from(value: RoundRect<U>) -> Self {
        Self {
            xywh: value.rect.to_xywh(),
            color: value.color.to_array_f32(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DrawCommand {
    Begin,
    End,
    Clip,
    Clear,
    DrawRects,
}

#[repr(align(16))]
pub struct DrawList {
    pub(super) rects: Vec<RRect>,
    pub(super) areas: Vec<Rect<()>>,
    pub(super) clears: Vec<Color>,
    pub(super) commands: Vec<(DrawCommand, u32)>,

    n_rects: u32,

    is_ended: bool,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            areas: Vec::new(),
            clears: Vec::new(),
            commands: Vec::new(),
            n_rects: 0,
            is_ended: false,
        }
    }

    pub fn reset(&mut self) {
        self.rects.clear();
        self.areas.clear();
        self.clears.clear();
        self.commands.clear();
        self.n_rects = 0;

        self.is_ended = false;
    }

    pub fn begin<U>(&mut self, clip: Rect<U>) {
        assert!(!self.is_ended);

        self.commands.push((DrawCommand::Begin, 0));
        self.commands.push((DrawCommand::Clip, 0));
        self.areas.push(clip.retype());
    }

    pub fn end(&mut self) {
        assert!(!self.is_ended);

        self.flush_command(DrawCommand::End);

        self.commands.push((DrawCommand::End, 0));
        self.is_ended = true;
    }

    pub fn clear(&mut self, color: Color) {
        assert!(!self.is_ended);

        self.flush_command(DrawCommand::DrawRects);

        self.commands
            .push((DrawCommand::Clear, self.clears.len() as u32));
        self.clears.push(color);
    }

    pub fn draw_rect<U>(&mut self, rect: impl Into<RoundRect<U>>) {
        assert!(!self.is_ended);

        self.flush_command(DrawCommand::DrawRects);

        self.rects.push(rect.into().into());
    }

    fn flush_command(&mut self, command: DrawCommand) {
        if command != DrawCommand::DrawRects && self.n_rects < self.rects.len() as u32 {
            let end = self.rects.len() as u32;
            self.commands.push((DrawCommand::DrawRects, end));
            self.n_rects = end;
        }
    }
}

pub trait Frame {}

pub trait Image {}

pub trait RenderTarget: Image {}

pub trait Context {
    type Frame: Frame;
    type Image: Image;

    fn create_frame(&self) -> Self::Frame;

    fn draw(
        &mut self,
        content: &DrawList,
        frame: &mut Self::Frame,
        image: impl Into<Self::Image>,
    ) -> SubmitId;

    fn wait(&self, submit_id: SubmitId);

    fn wait_for_idle(&self);
}

pub trait Device {
    type Context: Context;

    fn create_context(&self) -> Self::Context;

    fn wait(&self, submit_id: SubmitId);

    fn wait_for_idle(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_list() {
        let mut list = DrawList::new();

        list.begin(Rect::<()>::new(0.0, 0.0, 100.0, 100.0));
        list.clear(Color::WHITE);
        list.draw_rect(RoundRect::<()> {
            rect: Rect::ZERO,
            color: Color::BLACK,
        });
        list.draw_rect(RoundRect::<()> {
            rect: Rect::ZERO,
            color: Color::BLACK,
        });
        list.end();

        assert_eq!(list.commands.len(), 5);
        assert_eq!(list.rects.len(), 2);
        assert_eq!(list.areas.len(), 1);
        assert_eq!(list.clears.len(), 1);

        assert_eq!(list.commands[0], (DrawCommand::Begin, 0));
        assert_eq!(list.commands[1], (DrawCommand::Clip, 0));
        assert_eq!(list.commands[2], (DrawCommand::Clear, 0));
        assert_eq!(list.commands[3], (DrawCommand::DrawRects, 2));
        assert_eq!(list.commands[4], (DrawCommand::End, 0));
    }
}
