use core::panic;

use crate::{geometry::Point, graphics::color::Color};

use super::{
    text::{LayoutId, TextLayout},
    TextureRect,
};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextureFilter {
    #[default]
    Point = 0,
    Linear = 1,
}

impl TextureFilter {
    pub const fn from_bits(bits: u32) -> Self {
        match bits {
            0 => Self::Point,
            1 => Self::Linear,
            _ => panic!("Invalid texture filter bits"),
        }
    }

    pub const fn into_bits(self) -> u32 {
        self as u32
    }
}

#[bitfield_struct::bitfield(u32)]
#[derive(PartialEq)]
pub struct PrimitiveFlags {
    #[bits(1)]
    pub filter: TextureFilter,
    #[bits(31)]
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Primitive {
    pub xywh: [f32; 4],
    pub uvwh: [f32; 4],
    pub color: [f32; 4],
    pub texture_id: u32,
    pub flags: PrimitiveFlags,
    pub empty: u64,
}

impl Primitive {
    const DEFAULT: Self = Self {
        xywh: [0.0; 4],
        uvwh: [0.0; 4],
        color: [0.0; 4],
        texture_id: 0,
        flags: PrimitiveFlags::new(),
        empty: 0,
    };
}

pub enum Command<'a> {
    Begin {
        view: TextureRect,
        clip: TextureRect,
    },
    Close,
    Clear {
        color: Color,
    },
    Rects {
        rects: &'a [Primitive],
    },
    Chars {
        layout: LayoutId,
        glyphs: &'a [Primitive],
    },
}

pub enum CommandMut<'a> {
    Begin {
        view: TextureRect,
        clip: TextureRect,
    },
    Close,
    Clear {
        color: Color,
    },
    Rects {
        rects: &'a mut [Primitive],
    },
    Chars {
        layout: LayoutId,
        glyphs: &'a mut [Primitive],
    },
}

#[derive(Clone)]
enum Command_ {
    Begin {
        view: TextureRect,
        clip: TextureRect,
    },
    Close,
    Clear(Color),
    Rects {
        first: u32,
        count: u32,
    },
    Chars {
        first: u32,
        count: u32,
        layout: LayoutId,
    },
}

pub struct DrawList {
    prims: Vec<Primitive>,
    commands: Vec<Command_>,
    prim_start: u32,
    prim_count: u32,
    closed: bool,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            prims: Vec::new(),
            commands: Vec::new(),
            prim_start: 0,
            prim_count: 0,
            closed: false,
        }
    }

    pub fn iter(&self) -> CommandIter {
        CommandIter {
            commands: &self.commands,
            prims: &self.prims,
            count: 0,
        }
    }

    pub fn iter_mut(&mut self) -> CommandIterMut {
        CommandIterMut {
            commands: &self.commands,
            prims: &mut self.prims,
            count: 0,
        }
    }

    pub fn prims(&self) -> &[Primitive] {
        &self.prims
    }

    pub fn reset(&mut self) {
        self.prims.clear();
        self.commands.clear();
        self.prim_start = 0;
        self.prim_count = 0;
        self.closed = false;
    }

    pub fn begin(&mut self, view: TextureRect, clip: TextureRect) {
        self.commands.push(Command_::Begin { view, clip });
    }

    pub fn clear(&mut self, color: Color) {
        assert!(!self.closed, "DrawList is closed");
        self.flush_prims();
        self.commands.push(Command_::Clear(color));
    }

    pub fn close(&mut self) {
        if !self.closed {
            self.flush_prims();
            self.commands.push(Command_::Close);
            self.closed = true;
        }
    }

    pub fn draw_prim(&mut self, prim: &Primitive) {
        assert!(!self.closed, "DrawList is closed");
        self.prims.push(prim.clone());
        self.prim_count += 1;
    }

    /// Adds a command to draw the characters of a text layout.
    ///
    /// Reserves space for the characters in the prims array but does not fill
    /// them with data. A second pass is required to fill the prims array with
    /// the actual glyphs to draw.
    pub fn draw_chars(&mut self, layout: &TextLayout, at: Point) {
        assert!(!self.closed, "DrawList is closed");
        self.flush_prims();

        let count = layout.glyph_count();

        self.commands.push(Command_::Chars {
            first: self.prim_start,
            count,
            layout: layout.id(),
        });

        self.prims.extend(
            std::iter::once(Primitive {
                xywh: [at.x, at.y, 0.0, 0.0],
                ..Primitive::DEFAULT
            })
            .chain(std::iter::repeat(Primitive::DEFAULT))
            .take(count as usize),
        );

        self.prim_start += count;
    }

    fn flush_prims(&mut self) {
        if self.prim_count > 0 {
            let command = Command_::Rects {
                first: self.prim_start,
                count: self.prim_count,
            };

            self.commands.push(command);
            self.prim_start += self.prim_count;
            self.prim_count = 0;
        }

        debug_assert_eq!(self.prim_count, 0);
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandIter<'a> {
    commands: &'a [Command_],
    prims: &'a [Primitive],
    count: usize,
}

impl<'a> Iterator for CommandIter<'a> {
    type Item = Command<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == self.commands.len() {
            None
        } else {
            let cmd = self.commands[self.count].clone();
            self.count += 1;

            let r = match cmd {
                Command_::Begin { view, clip } => Command::Begin { view, clip },
                Command_::Close => Command::Close,
                Command_::Clear(color) => Command::Clear { color },
                Command_::Rects { first, count } => Command::Rects {
                    rects: &self.prims[first as usize..(first + count) as usize],
                },
                Command_::Chars {
                    first,
                    count,
                    layout,
                } => Command::Chars {
                    layout,
                    glyphs: &self.prims[first as usize..(first + count) as usize],
                },
            };

            Some(r)
        }
    }
}

pub struct CommandIterMut<'a> {
    commands: &'a [Command_],
    prims: &'a mut [Primitive],
    count: usize,
}

impl<'a> Iterator for CommandIterMut<'a> {
    type Item = CommandMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == self.commands.len() {
            None
        } else {
            let cmd = self.commands[self.count].clone();
            self.count += 1;

            let r = match cmd {
                Command_::Begin { view, clip } => CommandMut::Begin { view, clip },
                Command_::Close => CommandMut::Close,
                Command_::Clear(color) => CommandMut::Clear { color },
                Command_::Rects { first: _, count } => {
                    let slice = std::mem::take(&mut self.prims);
                    let (rects, tail) = slice.split_at_mut(count as usize);
                    self.prims = tail;

                    CommandMut::Rects { rects }
                }
                Command_::Chars {
                    first: _,
                    count,
                    layout,
                } => {
                    let slice = std::mem::take(&mut self.prims);
                    let (glyphs, tail) = slice.split_at_mut(count as usize);
                    self.prims = tail;

                    CommandMut::Chars { layout, glyphs }
                }
            };

            Some(r)
        }
    }
}
