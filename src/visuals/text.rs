use super::{FromVisual, Visual};

pub struct Text {}

impl Text {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Text> for Visual {
    fn from(text: Text) -> Self {
        Self::Text(text)
    }
}

impl FromVisual for Text {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }
}
