use std::cell::RefCell;

use crate::math::Rect;

use super::{
    backend::{Buffer, Device, GraphicsCommandList, Image, ResourceState},
    Color, RoundRect,
};

pub(crate) struct DrawData {
    pub rects: Vec<RoundRect<()>>,
    pub buffer: RefCell<Buffer>,
    pub command_list: GraphicsCommandList,
}

impl DrawData {
    pub fn new(buffer: Buffer, command_list: GraphicsCommandList) -> Self {
        Self {
            rects: vec![],
            buffer: RefCell::new(buffer),
            command_list,
        }
    }

    pub fn reset(&mut self) {
        self.command_list.reset();
        self.rects.clear();
    }

    /// Makes the draw data ready for rendering.
    pub(crate) fn finish(&mut self) {
        self.command_list.finish();
    }

    #[tracing::instrument(skip(self, device))]
    pub(super) fn sync_to_gpu(&self, device: &Device) {
        let mut buffer = self.buffer.borrow_mut();

        let rect_size = std::mem::size_of_val(self.rects.as_slice());
        let buffer_size = rect_size;

        if buffer_size > buffer.size() as usize {
            device.resize_memory(&mut buffer, rect_size as u64);
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.rects.as_ptr() as *const u8,
                buffer.as_mut_slice().as_mut_ptr(),
                rect_size,
            );
        }
    }
}

pub struct Canvas<'a, U> {
    bounds: Rect<U>,
    target: &'a Image,
    data: &'a mut DrawData,
}

impl<'a, U> Canvas<'a, U> {
    pub(crate) fn new(data: &'a mut DrawData, bounds: Rect<U>, target: &'a Image) -> Self {
        data.command_list.image_barrier(
            target,
            ResourceState::Present,
            ResourceState::RenderTarget,
        );
        data.command_list.set_render_target(target);

        Self {
            bounds,
            target,
            data,
        }
    }

    pub(crate) fn finish(self) -> &'a mut DrawData {
        // self.data.command_list.draw_instanced(...);

        self.data.command_list.image_barrier(
            self.target,
            ResourceState::RenderTarget,
            ResourceState::Present,
        );

        self.data
    }

    pub fn rect(&self) -> &Rect<U> {
        &self.bounds
    }

    pub fn clear(&mut self, color: Color) {
        self.data
            .command_list
            .clear([color.r, color.g, color.b, color.a]);
    }

    pub fn draw_rect(&mut self, rect: impl Into<RoundRect<U>>) {
        self.data.rects.push(rect.into().retype());
    }
}
