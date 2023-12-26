use std::cell::RefCell;

use crate::math::Rect;

use super::{
    backend::{Buffer, Device, GraphicsCommandList, Image, ResourceState},
    Color, RoundRect,
};

enum Command {
    Begin,
    End,
    Clear(Color),
    DrawRects { first: u32, count: u32 },
}

/// A list of draw operations and their associated geometry.
///
/// Draw lists are kept on the CPU and must be copied to the GPU for rendering.
/// Once copied, a draw list may be cleared and reused for the next frame.
///
/// The separation between draw lists and draw data is intended to allow for a
/// single draw list to provide data to multiple frames in flight, allowing the
/// CPU to begin recording the next frame while the GPU is still rendering the
/// previous one.
pub struct DrawList {
    rects: Vec<RoundRect<()>>,
    commands: Vec<Command>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            commands: Vec::new(),
        }
    }
}

pub(crate) struct DrawBuffer {
    pub buffer: RefCell<Buffer>,
    pub command_list: GraphicsCommandList,
}

impl DrawBuffer {
    pub fn new(buffer: Buffer, command_list: GraphicsCommandList) -> Self {
        Self {
            buffer: RefCell::new(buffer),
            command_list,
        }
    }

    #[tracing::instrument(skip(self, device, target, data))]
    pub(super) fn copy_to_gpu(&mut self, device: &Device, target: &Image, data: &DrawList) {
        let mut buffer = self.buffer.borrow_mut();

        let rect_size = std::mem::size_of_val(data.rects.as_slice());
        let buffer_size = rect_size;

        if buffer_size > buffer.size() as usize {
            device.resize_memory(&mut buffer, rect_size as u64);
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.rects.as_ptr() as *const u8,
                buffer.as_mut_slice().as_mut_ptr(),
                rect_size,
            );
        }

        for command in &data.commands {
            match command {
                Command::Begin => {
                    self.command_list.reset();

                    self.command_list.image_barrier(
                        target,
                        ResourceState::Present,
                        ResourceState::RenderTarget,
                    );
                    self.command_list.set_render_target(target);
                }
                Command::End => {
                    self.command_list.image_barrier(
                        target,
                        ResourceState::RenderTarget,
                        ResourceState::Present,
                    );
                    self.command_list.finish();
                }
                Command::Clear(color) => {
                    self.command_list.clear(color.to_array_f32());
                }
                Command::DrawRects { first, count } => {
                    // todo
                }
            }
        }
    }
}

pub struct Canvas<'a, U> {
    bounds: Rect<U>,
    data: &'a mut DrawList,

    n_rects: u32,
}

impl<'a, U> Canvas<'a, U> {
    pub(crate) fn new(data: &'a mut DrawList, bounds: Rect<U>) -> Self {
        data.rects.clear();
        data.commands.clear();
        data.commands.push(Command::Begin);

        Self {
            bounds,
            data,
            n_rects: 0,
        }
    }

    pub(crate) fn finish(self) -> &'a mut DrawList {
        if self.n_rects < self.data.rects.len() as u32 {
            self.data.commands.push(Command::DrawRects {
                first: self.n_rects,
                count: self.data.rects.len() as u32 - self.n_rects,
            });
        }

        self.data.commands.push(Command::End);
        self.data
    }

    pub fn rect(&self) -> &Rect<U> {
        &self.bounds
    }

    pub fn clear(&mut self, color: Color) {
        self.data.commands.push(Command::Clear(color));
    }

    pub fn draw_rect(&mut self, rect: impl Into<RoundRect<U>>) {
        self.data.rects.push(rect.into().retype());
    }
}
