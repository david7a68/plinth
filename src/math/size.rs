use super::{scale::Scale, Vec2};

pub struct Size<U> {
    pub width: f64,
    pub height: f64,
    _unit: std::marker::PhantomData<U>,
}

impl<U> Size<U> {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            _unit: std::marker::PhantomData,
        }
    }
}

impl<U, U2> std::ops::Mul<Scale<U, U2>> for Size<U> {
    type Output = Size<U2>;

    fn mul(self, rhs: Scale<U, U2>) -> Self::Output {
        Size::new(self.width * rhs.x, self.height * rhs.y)
    }
}

impl<U> std::ops::MulAssign<Scale<U, U>> for Size<U> {
    fn mul_assign(&mut self, rhs: Scale<U, U>) {
        self.width *= rhs.x;
        self.height *= rhs.y;
    }
}

impl<U> Clone for Size<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Size<U> {}

impl<U> std::fmt::Debug for Size<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Size")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<U> std::fmt::Display for Size<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<U> From<(f64, f64)> for Size<U> {
    fn from((width, height): (f64, f64)) -> Self {
        Self::new(width, height)
    }
}

impl<U> From<Vec2<U>> for Size<U> {
    fn from(vec: Vec2<U>) -> Self {
        Self::new(vec.x, vec.y)
    }
}

impl PartialEq for Size<()> {
    fn eq(&self, rhs: &Self) -> bool {
        self.width == rhs.width && self.height == rhs.height
    }
}
