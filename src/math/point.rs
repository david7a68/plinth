use super::{scale::Scale, translate::Translate, Vec2};

pub struct Point<U> {
    pub x: f64,
    pub y: f64,
    _unit: std::marker::PhantomData<U>,
}

impl<U> Point<U> {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }
}

impl<U, U2> std::ops::Add<Translate<U, U2>> for Point<U> {
    type Output = Point<U2>;

    fn add(self, rhs: Translate<U, U2>) -> Self::Output {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<U> std::ops::AddAssign<Translate<U, U>> for Point<U> {
    fn add_assign(&mut self, rhs: Translate<U, U>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<U, U2> std::ops::Sub<Translate<U, U2>> for Point<U2> {
    type Output = Point<U>;

    fn sub(self, rhs: Translate<U, U2>) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<U> std::ops::SubAssign<Translate<U, U>> for Point<U> {
    fn sub_assign(&mut self, rhs: Translate<U, U>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<U, U2> std::ops::Sub<Point<U2>> for Point<U> {
    type Output = Translate<U, U2>;

    fn sub(self, rhs: Point<U2>) -> Self::Output {
        Translate::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<U, U2> std::ops::Mul<Scale<U, U2>> for Point<U> {
    type Output = Point<U2>;

    fn mul(self, rhs: Scale<U, U2>) -> Self::Output {
        Point::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<U> std::ops::MulAssign<Scale<U, U>> for Point<U> {
    fn mul_assign(&mut self, rhs: Scale<U, U>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<U> Clone for Point<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Point<U> {}

impl<U> std::fmt::Debug for Point<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<U> std::fmt::Display for Point<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<U> From<(f64, f64)> for Point<U> {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl<U> From<Vec2<U>> for Point<U> {
    fn from(vec: Vec2<U>) -> Self {
        Self::new(vec.x, vec.y)
    }
}

impl PartialEq for Point<()> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}
