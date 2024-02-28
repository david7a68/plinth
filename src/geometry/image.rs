#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    pub const ONE: Self = Self {
        x: 1,
        y: 1,
        width: 1,
        height: 1,
    };

    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> u16 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u16 {
        self.y + self.height
    }

    pub fn to_xywh(&self) -> [u16; 4] {
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
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}
