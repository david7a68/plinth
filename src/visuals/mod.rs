mod canvas;
mod image;
mod panel;
mod text;
mod tree;

pub use canvas::Canvas;
pub use image::Image;
pub use panel::Panel;
pub use text::Text;
pub use tree::{VisualId, VisualTree};

pub enum Visual {
    Canvas(Canvas),
    Image(Image),
    Text(Text),
    Panel(Panel),
}

pub trait FromVisual: Sized {
    fn from_node(node: Visual) -> Option<Self>;

    fn from_ref(node: &Visual) -> Option<&Self>;

    fn from_mut(node: &mut Visual) -> Option<&mut Self>;
}
