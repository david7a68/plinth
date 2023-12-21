use crate::math::Rect;

use super::{
    backend::{GraphicsCommandList, Image, ResourceState},
    Color, RoundRect,
};

pub(crate) struct DrawData {
    pub rects: Vec<RoundRect<()>>,
    pub command_list: GraphicsCommandList,
}

impl DrawData {
    pub fn new(command_list: GraphicsCommandList) -> Self {
        Self {
            rects: Vec::new(),
            command_list,
        }
    }

    pub fn reset(&mut self) {
        self.command_list.reset();
        self.rects.clear();
    }

    /// Closese the command list and copies the data to the GPU for rendering.
    pub fn finish(&mut self) {
        self.command_list.finish();
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
