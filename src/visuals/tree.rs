use crate::{
    math::{Pixels, Point, Rect},
    visuals::{FromVisual, Image, Visual},
};

use super::{Canvas, Panel, Text};

#[derive(Clone, Copy)]
pub struct VisualId {
    index: u32,
    generation: u32,
}

pub trait SceneVisitor {
    fn visit_canvas(&mut self, canvas: &Canvas);
    fn visit_image(&mut self, image: &Image);
    fn visit_text(&mut self, text: &Text);
    fn visit_panel(&mut self, panel: &Panel);
}

pub trait SceneVisitorMut {
    fn visit_canvas_mut(&mut self, canvas: &mut Canvas);
    fn visit_image_mut(&mut self, image: &mut Image);
    fn visit_text_mut(&mut self, text: &mut Text);
    fn visit_panel_mut(&mut self, panel: &mut Panel);
}

pub struct VisualTree {}

impl VisualTree {
    pub fn new() -> Self {
        todo!()
    }

    pub fn root(&self) -> Option<&Visual> {
        todo!()
    }

    pub fn root_mut(&mut self) -> Option<&mut Visual> {
        todo!()
    }

    pub fn root_id(&self) -> Option<VisualId> {
        todo!()
    }

    pub fn view_rect(&self, node: VisualId) -> Option<Rect<Pixels>> {
        todo!()
    }

    pub fn set_view_rect(&mut self, node: VisualId, rect: Rect<Pixels>) {
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

    pub fn hit_test(&self, point: Point<Pixels>) -> Option<(VisualId, &Visual)> {
        todo!()
    }

    pub fn hit_test_mut(&mut self, point: Point<Pixels>) -> Option<(VisualId, &mut Visual)> {
        todo!()
    }

    pub fn walk(&self, visitor: &mut impl SceneVisitor) {
        todo!()
    }

    pub fn walk_mut(&mut self, visitor: &mut impl SceneVisitorMut) {
        todo!()
    }
}
