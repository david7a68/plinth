use std::path::Path;

use crate::{
    math::{Pixels, Size},
    visuals::{FromVisual, Visual},
};

pub struct Image {}

impl Image {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ()> {
        todo!()
    }

    pub fn size(&self) -> Size<Pixels> {
        todo!()
    }
}

impl From<Image> for Visual {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl FromVisual for Image {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }
}
