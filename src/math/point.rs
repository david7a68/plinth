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

    pub fn retype<U2>(self) -> Point<U2> {
        Point::new(self.x, self.y)
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

impl<U, U2> std::ops::Div<Scale<U, U2>> for Point<U2> {
    type Output = Point<U>;

    fn div(self, rhs: Scale<U, U2>) -> Self::Output {
        Point::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<U> std::ops::DivAssign<Scale<U, U>> for Point<U> {
    fn div_assign(&mut self, rhs: Scale<U, U>) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl<U> Clone for Point<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Point<U> {}

impl<U> Default for Point<U> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

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

macro_rules! from_tuple {
    ($($kind:ty),+) => {
        $(
            impl<U> From<($kind, $kind)> for Point<U> {
                fn from((x, y): ($kind, $kind)) -> Self {
                    Self::new(x as f64, y as f64)
                }
            }
        )+
    };
}

from_tuple!(u8, u16, u32, i8, i16, i32, f32, f64);

impl<U> From<Vec2<U>> for Point<U> {
    fn from(vec: Vec2<U>) -> Self {
        Self::new(vec.x, vec.y)
    }
}

impl<U> PartialEq for Point<U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let point = Point::<()>::new(1.0, 2.0);
        assert_eq!(point.x, 1.0);
        assert_eq!(point.y, 2.0);
    }

    #[test]
    fn diff() {
        let a = Point::<()>::new(1.0, 2.0);
        let b = Point::<()>::new(3.0, 4.0);

        assert_eq!(b - a, Translate::<(), ()>::new(2.0, 2.0));
    }

    #[test]
    fn transforms() {
        struct A;
        struct B;

        let point = Point::<A>::new(1.0, 2.0);

        let translate = Translate::<A, B>::new(3.0, 4.0);
        assert_eq!(point + translate, Point::<B>::new(4.0, 6.0));
        assert_eq!(point + -translate, Point::<B>::new(-2.0, -2.0));
        assert_eq!(point + translate - translate, point);

        let scale = Scale::<A, B>::new(5.0, 6.0);
        assert_eq!(point * scale, Point::<B>::new(5.0, 12.0));
        assert_eq!(point * scale / scale, point);

        let mut point = point;
        point += Translate::new(7.0, 8.0);
        assert_eq!(point, Point::<A>::new(8.0, 10.0));

        point -= Translate::new(7.0, 8.0);
        assert_eq!(point, Point::<A>::new(1.0, 2.0));

        point *= Scale::new(9.0, 10.0);
        assert_eq!(point, Point::<A>::new(9.0, 20.0));

        point /= Scale::new(9.0, 10.0);
        assert_eq!(point, Point::<A>::new(1.0, 2.0));
    }
}
