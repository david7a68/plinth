use crate::{
    color::{Color, Srgb},
    math::{Rect, Size},
};

use super::{FromVisual, Pixel, Visual};

/// A drawable area.
pub struct Canvas {}

impl Canvas {
    pub fn new(rect: impl Into<Rect<Pixel>>) -> Self {
        todo!()
    }

    pub fn size(&self) -> Size<Pixel> {
        todo!()
    }

    pub fn rect(&self) -> Rect<Pixel> {
        todo!()
    }

    pub fn set_rect(&mut self, rect: impl Into<Rect<Pixel>>) {
        todo!()
    }

    pub fn drawable_area(&self) -> Rect<Canvas> {
        self.rect().reinterpret_coordinate_space().0
    }

    pub fn clear(&mut self, color: Color<Srgb>) {
        todo!()
    }

    pub fn draw_rect(&mut self, rect: impl Into<Rect<Canvas>>, color: Color<Srgb>) {
        todo!()
    }
}

impl From<Canvas> for Visual {
    fn from(canvas: Canvas) -> Self {
        Self::Canvas(canvas)
    }
}

impl FromVisual for Canvas {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }
}
