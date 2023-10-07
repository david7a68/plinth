use std::time::Duration;

use super::{Scale, Size, Vec2};

pub struct Translate<Src, Dst> {
    pub x: f64,
    pub y: f64,
    _unit: std::marker::PhantomData<(Src, Dst)>,
}

impl<Src, Dst> Translate<Src, Dst> {
    pub fn new(x: f64, y: f64) -> Self {
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

impl<Src, Dst> std::ops::Add for Translate<Src, Dst> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<Src, Dst> std::ops::AddAssign for Translate<Src, Dst> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<Src, Dst> std::ops::Sub for Translate<Src, Dst> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<Src, Dst> std::ops::SubAssign for Translate<Src, Dst> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<Src, Dst> std::ops::Mul<f64> for Translate<Src, Dst> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<Src, Dst> std::ops::MulAssign<f64> for Translate<Src, Dst> {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<Src, Dst> std::ops::Div<f64> for Translate<Src, Dst> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<Src, Dst> std::ops::DivAssign<f64> for Translate<Src, Dst> {
    fn div_assign(&mut self, rhs: f64) {
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

impl<Src, Dst> std::ops::Div<Size<Dst>> for Translate<Src, Dst> {
    type Output = Scale<Src, Dst>;

    fn div(self, rhs: Size<Dst>) -> Self::Output {
        Scale::new(self.x / rhs.width, self.y / rhs.height)
    }
}

impl<Src, Cst> std::ops::Mul<Duration> for Translate<Src, Cst> {
    type Output = Translate<Src, Cst>;

    fn mul(self, rhs: Duration) -> Self::Output {
        Self::new(self.x * rhs.as_secs_f64(), self.y * rhs.as_secs_f64())
    }
}

impl<Src, Dst> std::ops::Div<Duration> for Translate<Src, Dst> {
    type Output = Translate<Src, Dst>;

    fn div(self, rhs: Duration) -> Self::Output {
        Self::new(self.x / rhs.as_secs_f64(), self.y / rhs.as_secs_f64())
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

impl<Src, Dst> From<(f64, f64)> for Translate<Src, Dst> {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl<Src, Dst> From<Vec2<Src>> for Translate<Src, Dst> {
    fn from(vec: Vec2<Src>) -> Self {
        Self::new(vec.x, vec.y)
    }
}
