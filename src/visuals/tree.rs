use crate::{
    math::Point,
    visuals::{FromVisual, Image, Visual},
};

use super::{Canvas, Pixel};

#[derive(Clone, Copy)]
pub struct VisualId {
    index: u32,
    generation: u32,
}

pub trait SceneVisitor {
    fn visit_canvas(&mut self, canvas: &Canvas);
    fn visit_image(&mut self, image: &Image);
}

pub trait SceneVisitorMut {
    fn visit_canvas_mut(&mut self, canvas: &mut Canvas);
    fn visit_image_mut(&mut self, image: &mut Image);
}

/// A tree of visual objects.
///
/// The visual tree separates a window into several parts which may be updated
/// at different frequencies in order to improve performance. For example, a
/// video player may update the playing video more frequently than the UI. This
/// is especially important for conserving power and bandwidth, such as when
/// running on underpowered hardware or over a remove desktop session.
///
/// The visual tree defines a plane on which all objects are placed. However, it
/// does not constrain them to the viewable area (i.e. within the window). It is
/// up to you to make sure to update the sizes and positions of objects as
/// necessary when the window is resized.
pub struct VisualTree {}

impl VisualTree {
    pub fn new() -> Self {
        todo!()
    }

    pub fn root<T: FromVisual>(&self) -> Option<&T> {
        todo!()
    }

    pub fn root_mut<T: FromVisual>(&mut self) -> Option<&mut T> {
        todo!()
    }

    pub fn root_id(&self) -> Option<VisualId> {
        todo!()
    }

    pub fn set_root(&mut self, node: impl Into<Visual>) -> (VisualId, Option<Visual>) {
        todo!()
    }

    pub fn set_root_id(&mut self, node: VisualId) -> Option<VisualId> {
        todo!()
    }

    pub fn get<T: FromVisual>(&self, node: VisualId) -> Option<&T> {
        todo!()
    }

    pub fn get_node(&self, node: VisualId) -> Option<&Visual> {
        todo!()
    }

    pub fn get_mut<T: FromVisual>(&mut self, node: VisualId) -> Option<&mut T> {
        todo!()
    }

    pub fn get_node_mut(&mut self, node: VisualId) -> Option<&mut Visual> {
        todo!()
    }

    pub fn add_child(&mut self, parent: VisualId, node: impl Into<Visual>) -> VisualId {
        todo!()
    }

    pub fn remove_node(&mut self, node: VisualId) -> Option<Visual> {
        todo!()
    }

    pub fn hit_test(&self, point: Point<Pixel>) -> Option<(VisualId, &Visual)> {
        todo!()
    }

    pub fn hit_test_mut(&mut self, point: Point<Pixel>) -> Option<(VisualId, &mut Visual)> {
        todo!()
    }

    pub fn walk(&self, visitor: &mut impl SceneVisitor) {
        todo!()
    }

    pub fn walk_mut(&mut self, visitor: &mut impl SceneVisitorMut) {
        todo!()
    }
}
