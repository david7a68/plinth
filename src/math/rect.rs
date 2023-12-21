use super::{point::Point, scale::Scale, size::Size, translate::Translate};

pub struct Rect<U> {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    _unit: std::marker::PhantomData<U>,
}

impl<U> Rect<U> {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
            _unit: std::marker::PhantomData,
        }
    }

    pub fn from_points(top_left: Point<U>, bottom_right: Point<U>) -> Self {
        Self::new(
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
        )
    }

    pub fn from_zero(size: Size<U>) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }

    pub fn from_origin(point: Point<U>, size: Size<U>) -> Self {
        Self::new(point.x, point.y, size.width, size.height)
    }

    pub fn from_center(center: Point<U>, size: Size<U>) -> Self {
        Self::new(
            center.x - size.width / 2.0,
            center.y - size.height / 2.0,
            size.width,
            size.height,
        )
    }

    /// Reinterprets the `Rect` as its own coordinate space.
    ///
    /// Also returns the translation back to the original coordinate space.
    pub fn reinterpret_coordinate_space<U2>(&self) -> (Rect<U2>, Translate<U2, U>) {
        (
            Rect::new(0.0, 0.0, self.width, self.height),
            Translate::new(self.x, self.y),
        )
    }

    pub fn retype<U2>(&self) -> Rect<U2> {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    pub fn center(&self) -> Point<U> {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let left = self.left().max(other.left());
        let right = self.right().min(other.right());
        let top = self.top().max(other.top());
        let bottom = self.bottom().min(other.bottom());

        if left > right || top > bottom {
            None
        } else {
            Some(Self::new(left, top, right - left, bottom - top))
        }
    }

    pub fn top(&self) -> f64 {
        self.y
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn top_left(&self) -> Point<U> {
        Point::new(self.x, self.y)
    }

    pub fn top_right(&self) -> Point<U> {
        Point::new(self.x + self.width, self.y)
    }

    pub fn bottom_left(&self) -> Point<U> {
        Point::new(self.x, self.y + self.height)
    }

    pub fn bottom_right(&self) -> Point<U> {
        Point::new(self.x + self.width, self.y + self.height)
    }

    pub fn size(&self) -> Size<U> {
        Size::new(self.width, self.height)
    }
}

impl<U, U2> std::ops::Add<Translate<U, U2>> for Rect<U> {
    type Output = Rect<U2>;

    fn add(self, rhs: Translate<U, U2>) -> Self::Output {
        Rect::new(self.x + rhs.x, self.y + rhs.y, self.width, self.height)
    }
}

impl<U, T: Into<Translate<U, U>>> std::ops::AddAssign<T> for Rect<U> {
    fn add_assign(&mut self, rhs: T) {
        let rhs = rhs.into();
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<U, U2> std::ops::Sub<Translate<U, U2>> for Rect<U2> {
    type Output = Rect<U>;

    fn sub(self, rhs: Translate<U, U2>) -> Self::Output {
        Rect::new(self.x - rhs.x, self.y - rhs.y, self.width, self.height)
    }
}

impl<U, T: Into<Translate<U, U>>> std::ops::SubAssign<T> for Rect<U> {
    fn sub_assign(&mut self, rhs: T) {
        let rhs = rhs.into();
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<U, U2> std::ops::Mul<Scale<U, U2>> for Rect<U> {
    type Output = Rect<U2>;

    fn mul(self, rhs: Scale<U, U2>) -> Self::Output {
        Rect::new(
            self.x * rhs.x,
            self.y * rhs.y,
            self.width * rhs.x,
            self.height * rhs.y,
        )
    }
}

impl<U> std::ops::MulAssign<Scale<U, U>> for Rect<U> {
    fn mul_assign(&mut self, rhs: Scale<U, U>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.width *= rhs.x;
        self.height *= rhs.y;
    }
}

impl<U, U2> std::ops::Div<Scale<U, U2>> for Rect<U2> {
    type Output = Rect<U>;

    fn div(self, rhs: Scale<U, U2>) -> Self::Output {
        Rect::new(
            self.x / rhs.x,
            self.y / rhs.y,
            self.width / rhs.x,
            self.height / rhs.y,
        )
    }
}

impl<U> std::ops::DivAssign<Scale<U, U>> for Rect<U> {
    fn div_assign(&mut self, rhs: Scale<U, U>) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.width /= rhs.x;
        self.height /= rhs.y;
    }
}

impl<U> Clone for Rect<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Rect<U> {}

impl<U> std::fmt::Debug for Rect<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rect")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<U> std::fmt::Display for Rect<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<U> From<Size<U>> for Rect<U> {
    fn from(size: Size<U>) -> Self {
        Self::from_zero(size)
    }
}

impl<U> PartialEq for Rect<U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y && self.width == rhs.width && self.height == rhs.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let rect = Rect::<()>::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(rect.x, 1.0);
        assert_eq!(rect.y, 2.0);
        assert_eq!(rect.width, 3.0);
        assert_eq!(rect.height, 4.0);

        assert_eq!(rect.top(), 2.0);
        assert_eq!(rect.bottom(), 6.0);
        assert_eq!(rect.left(), 1.0);
        assert_eq!(rect.right(), 4.0);

        assert_eq!(rect.top_left(), Point::new(1.0, 2.0));
        assert_eq!(rect.top_right(), Point::new(4.0, 2.0));
        assert_eq!(rect.bottom_left(), Point::new(1.0, 6.0));
        assert_eq!(rect.bottom_right(), Point::new(4.0, 6.0));

        assert_eq!(rect.size(), Size::new(3.0, 4.0));

        assert_eq!(
            rect,
            Rect::from_points(Point::new(1.0, 2.0), Point::new(4.0, 6.0))
        );

        assert_eq!(
            rect,
            Rect::from_origin(Point::new(1.0, 2.0), Size::new(3.0, 4.0))
        );

        assert_eq!(
            rect,
            Rect::from_center(Point::new(2.5, 4.0), Size::new(3.0, 4.0))
        );
    }

    #[test]
    fn new_zero() {
        let a = Rect::<()>::from_zero(Size::new(3.0, 4.0));
        assert_eq!(a, Rect::new(0.0, 0.0, 3.0, 4.0));
        assert_eq!(a, Rect::from(Size::new(3.0, 4.0)));
    }

    #[test]
    fn transforms() {
        struct A;
        struct B;
        struct C;

        let rect = Rect::<A>::new(1.0, 2.0, 3.0, 4.0);
        let scale = Scale::<A, B>::new(2.0, 3.0);
        let translate = Translate::<B, C>::new(4.0, 5.0);

        assert_eq!(rect * scale / scale, rect);
        assert_eq!(rect * scale * scale.inverse(), rect);
        assert_eq!(rect * scale + translate, Rect::new(6.0, 11.0, 6.0, 12.0));
        assert_eq!((rect * scale + translate - translate) / scale, rect);

        let mut rect = rect;
        let scale = Scale::<A, A>::new(2.0, 3.0);
        let translate = Translate::<A, A>::new(4.0, 5.0);

        rect *= scale;
        assert_eq!(rect, Rect::new(2.0, 6.0, 6.0, 12.0));

        rect /= scale;
        assert_eq!(rect, Rect::new(1.0, 2.0, 3.0, 4.0));

        rect += translate;
        assert_eq!(rect, Rect::new(5.0, 7.0, 3.0, 4.0));

        rect -= translate;
        assert_eq!(rect, Rect::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn properties() {
        struct A;
        struct B;

        let rect = Rect::<A>::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(rect.center(), Point::new(2.5, 4.0));

        let (r, t) = rect.reinterpret_coordinate_space::<B>();
        assert_eq!(r, Rect::from(Size::new(3.0, 4.0)));
        assert_eq!(r + t, rect);
    }

    #[test]
    fn intersection() {}
}
