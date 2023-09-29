use super::Vec2;

pub struct Scale<Src, Dst> {
    pub x: f64,
    pub y: f64,
    _unit: std::marker::PhantomData<(Src, Dst)>,
}

impl<Src, Dst> Scale<Src, Dst> {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }

    pub fn inverse(self) -> Scale<Dst, Src> {
        Scale::new(1.0 / self.x, 1.0 / self.y)
    }
}

impl<Src, Dst, Dst2> std::ops::Mul<Scale<Dst, Dst2>> for Scale<Src, Dst> {
    type Output = Scale<Src, Dst2>;

    fn mul(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Scale::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<U> std::ops::MulAssign<Scale<U, U>> for Scale<U, U> {
    fn mul_assign(&mut self, rhs: Scale<U, U>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<Src, Dst, Dst2> std::ops::Div<Scale<Dst, Dst2>> for Scale<Src, Dst> {
    type Output = Scale<Src, Dst2>;

    fn div(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Scale::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<U> std::ops::DivAssign<Scale<U, U>> for Scale<U, U> {
    fn div_assign(&mut self, rhs: Scale<U, U>) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl<Src, Dst> Clone for Scale<Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Src, Dst> Copy for Scale<Src, Dst> {}

impl<Src, Dst> std::fmt::Debug for Scale<Src, Dst> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scale")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("from", &std::any::type_name::<Src>())
            .field("to", &std::any::type_name::<Dst>())
            .finish()
    }
}

impl<Src, Dst> std::fmt::Display for Scale<Src, Dst> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<Src, Dst> From<f64> for Scale<Src, Dst> {
    fn from(value: f64) -> Self {
        Self::new(value, value)
    }
}

impl<Src, Dst> From<(f64, f64)> for Scale<Src, Dst> {
    fn from(value: (f64, f64)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl<Src, Dst> From<Vec2<Src>> for Scale<Src, Dst> {
    fn from(value: Vec2<Src>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<Src, Dst> PartialEq for Scale<Src, Dst> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}
