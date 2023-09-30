use super::{FromVisual, Visual};

pub struct Panel {}

impl Panel {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Panel> for Visual {
    fn from(panel: Panel) -> Self {
        Self::Panel(panel)
    }
}

impl FromVisual for Panel {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }
}
