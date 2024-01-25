use crate::time::Duration;

use super::{Scale, Vec2};

pub struct Translate<Src, Dst> {
    pub x: f32,
    pub y: f32,
    _unit: std::marker::PhantomData<(Src, Dst)>,
}

impl<Src, Dst> Translate<Src, Dst> {
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }
}

impl<Src, Dst> std::ops::Neg for Translate<Src, Dst> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl<Src, Dst, Dst2> std::ops::Add<Translate<Dst, Dst2>> for Translate<Src, Dst> {
    type Output = Translate<Src, Dst2>;

    fn add(self, rhs: Translate<Dst, Dst2>) -> Self::Output {
        Translate::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<Src, Dst> std::ops::AddAssign for Translate<Src, Dst> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<Src, Dst, Dst2> std::ops::Sub<Translate<Dst, Dst2>> for Translate<Src, Dst2> {
    type Output = Translate<Src, Dst>;

    fn sub(self, rhs: Translate<Dst, Dst2>) -> Self::Output {
        Translate::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<Src, Dst> std::ops::SubAssign for Translate<Src, Dst> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<Src, Dst> std::ops::Mul<f32> for Translate<Src, Dst> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<Src, Dst> std::ops::MulAssign<f32> for Translate<Src, Dst> {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<Src, Dst> std::ops::Div<f32> for Translate<Src, Dst> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<Src, Dst> std::ops::DivAssign<f32> for Translate<Src, Dst> {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<Src, Dst, Dst2> std::ops::Mul<Scale<Dst, Dst2>> for Translate<Src, Dst> {
    type Output = Translate<Src, Dst2>;

    fn mul(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Translate::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<Src, Dst, Dst2> std::ops::Div<Scale<Dst, Dst2>> for Translate<Src, Dst2> {
    type Output = Translate<Src, Dst>;

    fn div(self, rhs: Scale<Dst, Dst2>) -> Self::Output {
        Translate::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<Src, Dst> std::ops::Mul<Duration> for Translate<Src, Dst> {
    type Output = Translate<Src, Dst>;

    fn mul(self, rhs: Duration) -> Self::Output {
        Self::new(
            (f64::from(self.x) * rhs.0) as f32,
            (f64::from(self.y) * rhs.0) as f32,
        )
    }
}

impl<Src, Dst> std::ops::MulAssign<Duration> for Translate<Src, Dst> {
    fn mul_assign(&mut self, rhs: Duration) {
        let seconds = rhs.0;
        self.x = (f64::from(self.x) * seconds) as f32;
        self.y = (f64::from(self.y) * seconds) as f32;
    }
}

impl<Src, Dst> std::ops::Div<Duration> for Translate<Src, Dst> {
    type Output = Translate<Src, Dst>;

    fn div(self, rhs: Duration) -> Self::Output {
        Self::new(
            (f64::from(self.x) / rhs.0) as f32,
            (f64::from(self.y) / rhs.0) as f32,
        )
    }
}

impl<Src, Dst> std::ops::DivAssign<Duration> for Translate<Src, Dst> {
    fn div_assign(&mut self, rhs: Duration) {
        let seconds = rhs.0;
        self.x = (f64::from(self.x) / seconds) as f32;
        self.y = (f64::from(self.y) / seconds) as f32;
    }
}

impl<Src, Dst> Clone for Translate<Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Src, Dst> Copy for Translate<Src, Dst> {}

impl<Src, Dst> std::fmt::Debug for Translate<Src, Dst> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Translate")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("from", &std::any::type_name::<Src>())
            .field("to", &std::any::type_name::<Dst>())
            .finish()
    }
}

impl<Src, Dst> std::fmt::Display for Translate<Src, Dst> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<Src, Dst> PartialEq for Translate<Src, Dst> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}

impl<Src, Dst> From<(f32, f32)> for Translate<Src, Dst> {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl<Src, Dst> From<Vec2<Src>> for Translate<Src, Dst> {
    fn from(vec: Vec2<Src>) -> Self {
        Self::new(vec.x, vec.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let a = Translate::<(), ()>::new(1.0, 2.0);
        assert_eq!(a.x, 1.0);
        assert_eq!(a.y, 2.0);

        let b = (1.0, 2.0).into();
        assert_eq!(a, b);

        let c = Vec2::new(1.0, 2.0).into();
        assert_eq!(a, c);
    }

    #[test]
    fn ops() {
        struct A;
        struct B;
        struct C;

        let a = Translate::<A, B>::new(1.0, 2.0);
        let b = Translate::<B, C>::new(3.0, 4.0);

        assert_eq!(-a, Translate::<A, B>::new(-1.0, -2.0));
        assert_eq!(a + b, Translate::<A, C>::new(4.0, 6.0));
        assert_eq!(a - a, Translate::<A, A>::new(0.0, 0.0));

        let mut c = a;
        c += a;
        assert_eq!(c, Translate::<A, B>::new(2.0, 4.0));
        c -= a;
        assert_eq!(c, a);
    }

    #[test]
    fn float_ops() {
        let a = Translate::<(), ()>::new(1.0, 2.0);

        assert_eq!(a * 2.0, Translate::<(), ()>::new(2.0, 4.0));
        assert_eq!(a / 2.0, Translate::<(), ()>::new(0.5, 1.0));
    }

    #[test]
    fn scale_ops() {
        struct A;
        struct B;
        struct C;

        let a = Translate::<A, B>::new(1.0, 2.0);
        let b = Scale::<B, C>::new(3.0, 4.0);

        assert_eq!(a * b, Translate::<A, C>::new(3.0, 8.0));
        assert_eq!(a * b / b, a);
    }

    #[test]
    fn time_ops() {
        struct A;
        struct B;

        let a = Translate::<A, B>::new(1.0, 2.0);
        let time = Duration(2.0);

        assert_eq!(a * time, Translate::<A, B>::new(2.0, 4.0));
        assert_eq!(a * time / time, a);

        let mut b = a;
        b *= time;
        assert_eq!(b, Translate::<A, B>::new(2.0, 4.0));
        b /= time;
        assert_eq!(b, a);
    }
}
