use crate::math::Rect;

use super::{Color, DefaultColorSpace, GraphicsCommandList};

pub struct Canvas<'a, U> {
    rect: Rect<U>,
    geometry: &'a mut GeometryBuffer,
    command_list: &'a mut GraphicsCommandList,
}

impl<'a, U> Canvas<'a, U> {
    pub fn new(
        rect: Rect<U>,
        geometry: &'a mut GeometryBuffer,
        command_list: &'a mut GraphicsCommandList,
    ) -> Self {
        Self {
            rect,
            geometry,
            command_list,
        }
    }

    pub fn rect(&self) -> &Rect<U> {
        &self.rect
    }

    pub fn clear(&mut self, color: Color<DefaultColorSpace>) {
        self.command_list
            .clear([color.r, color.g, color.b, color.a]);
    }

    pub fn draw_rect(&mut self, rect: impl Into<Rect<U>>, color: Color<DefaultColorSpace>) {
        todo!()
    }
}

pub struct GeometryBuffer {
    // todo: this will be something else soon
    vertices: Vec<f32>,
    indices: Vec<u32>,
}

impl GeometryBuffer {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}
