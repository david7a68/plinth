use crate::{
    color::{Color, Srgb},
    math::{Pixels, Rect},
};

use super::{FromVisual, Visual};

pub struct Canvas {}

impl Canvas {
    pub fn new() -> Self {
        todo!()
    }

    pub fn clear(&mut self, color: Color<Srgb>) {
        todo!()
    }

    pub fn fill(&mut self, rect: Rect<Pixels>, color: Color<Srgb>) {
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
