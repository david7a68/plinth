mod canvas;
mod color;
mod image;
mod pixel;
mod tree;

pub use canvas::Canvas;
pub use color::{Color, ColorSpace, Srgb};
pub use image::Image;
pub use pixel::Pixel;
pub use tree::{VisualId, VisualTree};

pub enum Visual {
    Canvas(Canvas),
    Image(Image),
}

pub trait FromVisual: Sized {
    fn from_node(node: Visual) -> Option<Self>;

    fn from_ref(node: &Visual) -> Option<&Self>;

    fn from_mut(node: &mut Visual) -> Option<&mut Self>;
}
