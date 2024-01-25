use super::Vec2;

pub struct Scale<Src, Dst> {
    pub x: f32,
    pub y: f32,
    _unit: std::marker::PhantomData<(Src, Dst)>,
}

impl<Src, Dst> Scale<Src, Dst> {
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn inverse(self) -> Scale<Dst, Src> {
        Scale::new(1.0 / self.x, 1.0 / self.y)
    }
}

impl<Src, Dst> std::ops::Mul<f32> for Scale<Src, Dst> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<Src, Dst> std::ops::MulAssign<f32> for Scale<Src, Dst> {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<Src, Dst> std::ops::Div<f32> for Scale<Src, Dst> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<Src, Dst> std::ops::DivAssign<f32> for Scale<Src, Dst> {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<Src, Dst, Dst2> std::ops::Mul<Scale<Dst, Dst2>> for Scale<Src, Dst> {
    type Output = Scale<Src, Dst2>;

    fn mul(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Scale::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<Src, Dst> std::ops::MulAssign<Scale<Dst, Dst>> for Scale<Src, Dst> {
    fn mul_assign(&mut self, rhs: Scale<Dst, Dst>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<Src, Dst, Dst2> std::ops::Div<Scale<Dst, Dst2>> for Scale<Src, Dst2> {
    type Output = Scale<Src, Dst>;

    fn div(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Scale::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<Src, Dst> std::ops::DivAssign<Scale<Dst, Dst>> for Scale<Src, Dst> {
    fn div_assign(&mut self, rhs: Scale<Dst, Dst>) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

// impl<Src, Cst> std::ops::Mul<Duration> for Scale<Src, Cst> {
//     type Output = Scale<Src, Cst>;

//     fn mul(self, rhs: Duration) -> Self::Output {
//         Self::new(self.x * rhs.as_secs_f64(), self.y * rhs.as_secs_f64())
//     }
// }

// impl<Src, Dst> std::ops::Div<Duration> for Scale<Src, Dst> {
//     type Output = Scale<Src, Dst>;

//     fn div(self, rhs: Duration) -> Self::Output {
//         Self::new(self.x / rhs.as_secs_f64(), self.y / rhs.as_secs_f64())
//     }
// }

impl<Src, Dst> Clone for Scale<Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Src, Dst> Copy for Scale<Src, Dst> {}

impl<Src, Dst> Default for Scale<Src, Dst> {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

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

impl<Src, Dst> From<f32> for Scale<Src, Dst> {
    fn from(value: f32) -> Self {
        Self::new(value, value)
    }
}

impl<Src, Dst> From<(f32, f32)> for Scale<Src, Dst> {
    fn from(value: (f32, f32)) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        struct A;
        struct B;

        let a = Scale::<A, B>::new(1.0, 2.0);
        assert_eq!(a.x, 1.0);
        assert_eq!(a.y, 2.0);

        let b = (1.0, 2.0).into();
        assert_eq!(a, b);

        let c = Vec2::new(1.0, 2.0).into();
        assert_eq!(a, c);

        let c = 1.0.into();
        assert_eq!(Scale::<A, B>::new(1.0, 1.0), c);
    }

    #[test]
    fn ops() {
        struct A;
        struct B;
        struct C;

        let a = Scale::<A, B>::new(1.0, 2.0);
        let b = Scale::<B, C>::new(3.0, 4.0);

        assert_eq!(a * b, Scale::<A, C>::new(3.0, 8.0));
        assert_eq!(a * b / b, a);

        let mut c = a;
        let d = Scale::<B, B>::new(3.0, 4.0);

        c *= d;
        assert_eq!(c, Scale::<A, B>::new(3.0, 8.0));

        c /= d;
        assert_eq!(c, a);
    }

    #[test]
    fn float_ops() {
        struct A;
        struct B;

        let a = Scale::<A, B>::new(1.0, 2.0);
        assert_eq!(a * 2.0, Scale::<A, B>::new(2.0, 4.0));
        assert_eq!(a / 2.0, Scale::<A, B>::new(0.5, 1.0));

        let mut b = a;
        let mut c = a;

        b *= 2.0;
        assert_eq!(b, Scale::<A, B>::new(2.0, 4.0));

        c *= Scale::new(2.0, 2.0);
        assert_eq!(b, c);

        b /= 2.0;
        assert_eq!(b, a);

        c /= Scale::new(2.0, 2.0);
        assert_eq!(b, c);
    }

    #[test]
    fn inverse() {
        let a = Scale::<(), ()>::new(1.0, 2.0);
        assert_eq!(a.inverse(), Scale::<(), ()>::new(1.0, 0.5));
        assert_eq!(a.inverse().inverse(), a);
    }
}
