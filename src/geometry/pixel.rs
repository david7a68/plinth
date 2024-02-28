#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        width: 1.0,
        height: 1.0,
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn to_xywh(&self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Into<Box> for Rect {
    fn into(self) -> Box {
        Box {
            top: self.y,
            bottom: self.bottom(),
            left: self.x,
            right: self.right(),
        }
    }
}

pub struct Box {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}
