// use std::{ops::Mul, time::Duration};

// use self::{unit::CoordinateUnit, vector::Vec2f};

mod geometry;
mod unit;

pub use geometry::*;
pub use unit::*;

pub struct PixelsPerSecond;

impl CoordinateUnit for PixelsPerSecond {}

impl From<PixelsPerSecond> for Pixel {
    fn from(_: PixelsPerSecond) -> Self {
        Pixel
    }
}
