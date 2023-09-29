use super::{Scale, Translate};

pub struct Vec2<U> {
    pub x: f64,
    pub y: f64,
    _unit: std::marker::PhantomData<U>,
}

impl<U> Vec2<U> {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }
}

impl<U> std::ops::Neg for Vec2<U> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl<U> std::ops::Add for Vec2<U> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<U> std::ops::AddAssign for Vec2<U> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<U> std::ops::Sub for Vec2<U> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<U> std::ops::SubAssign for Vec2<U> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<U> std::ops::Rem<Vec2<U>> for Vec2<U> {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.x % rhs.x, self.y % rhs.y)
    }
}

impl<U> std::ops::RemAssign<Vec2<U>> for Vec2<U> {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
    }
}

impl<U, U2> std::ops::Add<Translate<U, U2>> for Vec2<U> {
    type Output = Vec2<U2>;

    fn add(self, rhs: Translate<U, U2>) -> Self::Output {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<U> std::ops::AddAssign<Translate<U, U>> for Vec2<U> {
    fn add_assign(&mut self, rhs: Translate<U, U>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<U, U2> std::ops::Mul<Scale<U, U2>> for Vec2<U> {
    type Output = Vec2<U2>;

    fn mul(self, rhs: Scale<U, U2>) -> Self::Output {
        Vec2::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<U> std::ops::MulAssign<Scale<U, U>> for Vec2<U> {
    fn mul_assign(&mut self, rhs: Scale<U, U>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<U> std::ops::Mul<f64> for Vec2<U> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<U> std::ops::MulAssign<f64> for Vec2<U> {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<U> std::ops::Div<f64> for Vec2<U> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<U> std::ops::DivAssign<f64> for Vec2<U> {
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<U> std::ops::Rem<f64> for Vec2<U> {
    type Output = Self;

    fn rem(self, rhs: f64) -> Self::Output {
        Self::new(self.x % rhs, self.y % rhs)
    }
}

impl<U> std::ops::RemAssign<f64> for Vec2<U> {
    fn rem_assign(&mut self, rhs: f64) {
        self.x %= rhs;
        self.y %= rhs;
    }
}

impl<U> Clone for Vec2<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Vec2<U> {}

impl<U> std::fmt::Debug for Vec2<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vec2")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<U> std::fmt::Display for Vec2<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl PartialEq for Vec2<()> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}

impl<U> From<(f64, f64)> for Vec2<U> {
    fn from((x, y): (f64, f64)) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }
}
