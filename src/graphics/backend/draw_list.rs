use core::panic;

use crate::{
    geometry::{Pixel, Rect},
    graphics::{Color, Image, RoundRect},
    limits::enforce_draw_list_max_commands_u32,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RRect {
    pub xywh: [f32; 4],
    pub color: [f32; 4],
}

impl RRect {
    pub fn new(rect: &RoundRect) -> Self {
        Self {
            xywh: rect.rect.to_xywh().map(|x| x.0),
            color: rect.color.to_array_f32(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Begin(Rect<Pixel>),
    Close,
    Clear(Color),
    Rects(Image, u32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DrawCommand {
    Begin,
    Close,
    Clear,
    Rects,
}

#[repr(align(16))]
pub(crate) struct DrawList {
    pub prims: Vec<RRect>,
    pub areas: Vec<Rect<Pixel>>,
    pub colors: Vec<Color>,
    pub images: Vec<Image>,
    pub commands: Vec<(DrawCommand, u32)>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            prims: Vec::new(),
            areas: Vec::new(),
            colors: Vec::new(),
            images: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.prims.clear();
        self.areas.clear();
        self.colors.clear();
        self.images.clear();
        self.commands.clear();
    }

    pub fn iter(&self) -> CommandIterator<'_> {
        CommandIterator {
            areas: &self.areas,
            colors: &self.colors,
            images: &self.images,
            commands: &self.commands,
            index: 0,
            draws: 0,
        }
    }
}

pub struct CommandIterator<'a> {
    areas: &'a [Rect<Pixel>],
    colors: &'a [Color],
    images: &'a [Image],
    commands: &'a [(DrawCommand, u32)],
    index: usize,
    draws: usize,
}

impl Iterator for CommandIterator<'_> {
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
                let image = self.images[self.draws];
                self.draws += 1;

                Command::Rects(image, count)
            }
        };

        self.index += 1;
        Some(cmd)
    }
}

pub struct Canvas<'a> {
    draw_list: &'a mut DrawList,
    region: Rect<Pixel>,
    rect_batch_start: usize,
    rect_batch_count: usize,
    rect_batch_image: Image,
    state: DrawCommand,
}

impl<'a> Canvas<'a> {
    pub fn new(draw_list: &'a mut DrawList, region: Rect<Pixel>) -> Self {
        draw_list.clear();
        draw_list.areas.push(region);
        draw_list.commands.push((DrawCommand::Begin, 0));

        Self {
            draw_list,
            region,
            rect_batch_start: 0,
            rect_batch_count: 0,
            rect_batch_image: Image::default(),
            state: DrawCommand::Begin,
        }
    }

    pub fn region(&self) -> Rect<Pixel> {
        self.region
    }

    pub fn clear(&mut self, color: Color) {
        match self.state {
            DrawCommand::Begin => {}
            DrawCommand::Clear => {
                // todo: might be an error? maybe surface in log. -dz (2024-03-24)
            }
            DrawCommand::Rects => self.submit_batch(),
            DrawCommand::Close => panic!("Canvas state Close -> Clear is a bug."),
        }

        self.draw_list.commands.push((
            DrawCommand::Clear,
            enforce_draw_list_max_commands_u32(self.draw_list.colors.len()),
        ));

        self.draw_list.colors.push(color);
        self.state = DrawCommand::Clear;
    }

    pub fn draw_rect(&mut self, rect: RoundRect) {
        match self.state {
            DrawCommand::Begin => {
                debug_assert_eq!(self.rect_batch_start, 0);
                debug_assert_eq!(self.rect_batch_count, 0);
                debug_assert_eq!(self.rect_batch_image, Image::default());
                debug_assert_eq!(self.draw_list.prims.len(), 0);
            }
            DrawCommand::Clear => {
                debug_assert_eq!(self.rect_batch_count, 0);

                self.rect_batch_start = self.draw_list.prims.len();
            }
            DrawCommand::Rects => {
                debug_assert!(self.rect_batch_count > 0);

                if rect.image != self.rect_batch_image {
                    self.submit_batch();
                    self.rect_batch_image = rect.image;
                }
            }
            DrawCommand::Close => panic!("Canvas state Close -> DrawRect is a bug."),
        }

        self.draw_list.prims.push(RRect::new(&rect));

        self.rect_batch_count += 1;
        self.rect_batch_image = rect.image;

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

        self.draw_list.images.push(self.rect_batch_image);

        self.rect_batch_start = self.draw_list.prims.len();
        self.rect_batch_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        geometry::{Pixel, Rect},
        graphics::Image,
    };

    #[test]
    fn draw_list() {
        let mut list = DrawList::new();
        let mut canvas = Canvas::new(&mut list, Rect::new((0.0, 0.0), (100.0, 100.0)));

        canvas.clear(Color::WHITE);
        canvas.draw_rect(RoundRect {
            rect: Rect::<Pixel>::ZERO,
            color: Color::BLACK,
            image: Image::default(),
        });
        canvas.draw_rect(RoundRect {
            rect: Rect::<Pixel>::ZERO,
            color: Color::BLACK,
            image: Image::default(),
        });

        canvas.finish();

        assert_eq!(list.commands.len(), 4);
        assert_eq!(list.prims.len(), 2);
        assert_eq!(list.areas.len(), 1);
        assert_eq!(list.images.len(), 1);
        assert_eq!(list.colors.len(), 1);

        assert_eq!(list.commands[0], (DrawCommand::Begin, 0));
        assert_eq!(list.commands[1], (DrawCommand::Clear, 0));
        assert_eq!(list.commands[2], (DrawCommand::Rects, 2));
        assert_eq!(list.commands[3], (DrawCommand::Close, 0));

        {
            let mut it = list.iter();

            assert_eq!(
                it.next(),
                Some(Command::Begin(Rect::new((0.0, 0.0), (100.0, 100.0))))
            );
            assert_eq!(it.next(), Some(Command::Clear(Color::WHITE)));
            assert_eq!(it.next(), Some(Command::Rects(Image::default(), 2)));
            assert_eq!(it.next(), Some(Command::Close));
        }
    }
}
