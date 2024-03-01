use std::ops::Mul;

use crate::geometry::{self, pixel};

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
}

impl WindowSize {
    pub const MAX: Self = Self {
        width: i32::MAX,
        height: i32::MAX,
    };
}

impl WindowSize {
    pub fn into_rect(self) -> WindowRect {
        WindowRect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<WindowRect> for geometry::image::Rect {
    fn from(rect: WindowRect) -> Self {
        geometry::image::Rect {
            x: u16::try_from(rect.x).unwrap(),
            y: u16::try_from(rect.y).unwrap(),
            width: u16::try_from(rect.width).unwrap(),
            height: u16::try_from(rect.height).unwrap(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DpiScale {
    pub factor: f32,
}

impl Mul<WindowSize> for DpiScale {
    type Output = pixel::Size;

    fn mul(self, rhs: WindowSize) -> Self::Output {
        pixel::Size {
            width: rhs.width as f32 * self.factor,
            height: rhs.height as f32 * self.factor,
        }
    }
}

impl Mul<WindowPoint> for DpiScale {
    type Output = pixel::Size;

    fn mul(self, rhs: WindowPoint) -> Self::Output {
        pixel::Size {
            width: rhs.x as f32 * self.factor,
            height: rhs.y as f32 * self.factor,
        }
    }
}

impl Mul<WindowRect> for DpiScale {
    type Output = pixel::Rect;

    fn mul(self, rhs: WindowRect) -> Self::Output {
        pixel::Rect {
            x: rhs.x as f32 * self.factor,
            y: rhs.y as f32 * self.factor,
            width: rhs.width as f32 * self.factor,
            height: rhs.height as f32 * self.factor,
        }
    }
}

impl Default for DpiScale {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}
