use crate::{Canvas, Image, Panel, Text};

#[derive(Clone, Copy)]
pub struct SceneNodeId {
    index: u32,
    generation: u32,
}

pub enum SceneNode {
    Canvas(Canvas),
    Image(Image),
    Text(Text),
    Panel(Panel),
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

pub struct Scene {}

impl Scene {
    pub fn new() -> Self {
        todo!()
    }

    pub fn root(&self) -> Option<&SceneNode> {
        todo!()
    }

    pub fn root_mut(&mut self) -> Option<&mut SceneNode> {
        todo!()
    }

    pub fn root_id(&self) -> Option<SceneNodeId> {
        todo!()
    }

    pub fn set_root(&mut self, node: impl Into<SceneNode>) -> (SceneNodeId, Option<SceneNode>) {
        todo!()
    }

    pub fn get<T: TryFrom<SceneNode>>(&self, node: SceneNodeId) -> Option<&T> {
        todo!()
    }

    pub fn get_node(&self, node: SceneNodeId) -> Option<&SceneNode> {
        todo!()
    }

    pub fn get_mut<T: TryFrom<SceneNode>>(&mut self, node: SceneNodeId) -> Option<&mut T> {
        todo!()
    }

    pub fn get_node_mut(&mut self, node: SceneNodeId) -> Option<&mut SceneNode> {
        todo!()
    }

    pub fn add_child(&mut self, parent: SceneNodeId, node: impl Into<SceneNode>) -> SceneNodeId {
        todo!()
    }

    pub fn remove_node(&mut self, node: SceneNodeId) -> Option<SceneNode> {
        todo!()
    }

    pub fn walk(&self, visitor: &mut impl SceneVisitor) {
        todo!()
    }

    pub fn walk_mut(&mut self, visitor: &mut impl SceneVisitorMut) {
        todo!()
    }
}
