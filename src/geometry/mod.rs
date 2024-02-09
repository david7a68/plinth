// use std::{ops::Mul, time::Duration};

// use self::{unit::CoordinateUnit, vector::Vec2f};

mod point;
mod rect;
mod scale;
mod size;
mod vec;

pub use point::Point;
pub use rect::Rect;
pub use scale::Scale;
pub use size::Size;
pub use vec::Vec2;

pub struct Pixel;
