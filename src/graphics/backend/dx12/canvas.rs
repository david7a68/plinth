use crate::{
    geometry::image,
    graphics::{Color, RoundRect},
    limits::enforce_draw_list_max_commands_u32,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RRect {
    pub xywh: [f32; 4],
    pub color: [f32; 4],
}

impl From<RoundRect> for RRect {
    #[inline]
    fn from(value: RoundRect) -> Self {
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
    Clear,
    DrawRects,
}

#[repr(align(16))]
pub struct DrawList {
    pub(super) rects: Vec<RRect>,
    pub(super) areas: Vec<image::Rect>,
    pub(super) clears: Vec<Color>,
    pub(super) commands: Vec<(DrawCommand, u32)>,

    n_rects: u32,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            areas: Vec::new(),
            clears: Vec::new(),
            commands: Vec::new(),
            n_rects: 0,
        }
    }

    fn flush_command(&mut self, command: DrawCommand) {
        if command != DrawCommand::DrawRects && self.rects.len() > self.n_rects as usize {
            let end = enforce_draw_list_max_commands_u32(self.rects.len());
            self.commands.push((DrawCommand::DrawRects, end));
            self.n_rects = end;
        }
    }

    pub fn finish(&mut self) {
        self.flush_command(DrawCommand::End);
        self.commands.push((DrawCommand::End, 0));
    }
}

pub struct Canvas<'a> {
    draw_list: &'a mut DrawList,
    region: image::Rect,
}

impl<'a> Canvas<'a> {
    pub fn new(draw_list: &'a mut DrawList, region: image::Rect) -> Self {
        draw_list.rects.clear();
        draw_list.areas.clear();
        draw_list.clears.clear();
        draw_list.commands.clear();
        draw_list.areas.push(region);
        draw_list.commands.push((DrawCommand::Begin, 0));
        draw_list.n_rects = 0;

        Self { draw_list, region }
    }

    pub fn finish(self) -> &'a mut DrawList {
        self.draw_list.flush_command(DrawCommand::End);
        self.draw_list.commands.push((DrawCommand::End, 0));

        self.draw_list
    }

    pub fn region(&self) -> image::Rect {
        self.region
    }

    pub fn clear(&mut self, color: Color) {
        self.draw_list.flush_command(DrawCommand::DrawRects);
        self.draw_list.commands.push((
            DrawCommand::Clear,
            enforce_draw_list_max_commands_u32(self.draw_list.clears.len()),
        ));
        self.draw_list.clears.push(color);
    }

    pub fn draw_rect(&mut self, rect: RoundRect) {
        self.draw_list.flush_command(DrawCommand::DrawRects);
        self.draw_list.rects.push(rect.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::pixel;

    #[test]
    fn draw_list() {
        let mut list = DrawList::new();
        let mut canvas = Canvas::new(&mut list, image::Rect::new(0, 0, 100, 100));

        canvas.clear(Color::WHITE);
        canvas.draw_rect(RoundRect {
            rect: pixel::Rect::ZERO,
            color: Color::BLACK,
        });
        canvas.draw_rect(RoundRect {
            rect: pixel::Rect::ZERO,
            color: Color::BLACK,
        });

        let draws = canvas.finish();

        assert_eq!(draws.commands.len(), 4);
        assert_eq!(draws.rects.len(), 2);
        assert_eq!(draws.areas.len(), 1);
        assert_eq!(draws.clears.len(), 1);

        assert_eq!(draws.commands[0], (DrawCommand::Begin, 0));
        assert_eq!(draws.commands[1], (DrawCommand::Clear, 0));
        assert_eq!(draws.commands[2], (DrawCommand::DrawRects, 2));
        assert_eq!(draws.commands[3], (DrawCommand::End, 0));
    }
}
