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

impl<U> std::ops::Mul<f64> for Size<U> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.width * rhs, self.height * rhs)
    }
}

impl<U> std::ops::MulAssign<f64> for Size<U> {
    fn mul_assign(&mut self, rhs: f64) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl<U> std::ops::Div<f64> for Size<U> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.width / rhs, self.height / rhs)
    }
}

impl<U> std::ops::DivAssign<f64> for Size<U> {
    fn div_assign(&mut self, rhs: f64) {
        self.width /= rhs;
        self.height /= rhs;
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

impl<U, U2> std::ops::Div<Scale<U, U2>> for Size<U2> {
    type Output = Size<U>;

    fn div(self, rhs: Scale<U, U2>) -> Self::Output {
        Size::new(self.width / rhs.x, self.height / rhs.y)
    }
}

impl<U> std::ops::DivAssign<Scale<U, U>> for Size<U> {
    fn div_assign(&mut self, rhs: Scale<U, U>) {
        self.width /= rhs.x;
        self.height /= rhs.y;
    }
}

impl<U> Clone for Size<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Size<U> {}

impl<U> Default for Size<U> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

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

impl<U> PartialEq for Size<U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.width == rhs.width && self.height == rhs.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let size = Size::<()>::new(1.0, 2.0);
        assert_eq!(size.width, 1.0);
        assert_eq!(size.height, 2.0);

        assert_eq!(size, (1.0, 2.0).into());
        assert_eq!(size, Vec2::<()>::new(1.0, 2.0).into());
    }

    #[test]
    fn scale() {
        struct A;
        struct B;

        let size = Size::<A>::new(1.0, 2.0);
        assert_eq!(size * 2.0, Size::<A>::new(2.0, 4.0));
        assert_eq!(size / 2.0, Size::<A>::new(0.5, 1.0));

        let scale = Scale::<A, B>::from(2.0);
        assert_eq!(size * scale, Size::<B>::new(2.0, 4.0));
        assert_eq!(size * scale / scale, size);

        let mut size = size;
        size *= 2.0;
        assert_eq!(size, Size::<A>::new(2.0, 4.0));
        size /= 2.0;
        assert_eq!(size, Size::<A>::new(1.0, 2.0));
    }
}
