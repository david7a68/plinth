use super::point::Point;

pub struct Rect<T, U = ()> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
    _unit: std::marker::PhantomData<U>,
}

impl<T, U> Rect<T, U> {
    #[must_use]
    pub const fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
            _unit: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn from_points(top_left: Point<T, U>, bottom_right: Point<T, U>) -> Self
    where
        T: std::ops::Sub<Output = T> + Copy,
    {
        Self::new(
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
            top_left.x,
            top_left.y,
        )
    }
}

macro_rules! impl_rect {
    ($($type:ty),+) => {
        $(
            impl<U> Rect<$type, U> {
                #[must_use]
                pub fn top(&self) -> $type {
                    self.y
                }

                #[must_use]
                pub fn left(&self) -> $type {
                    self.x
                }

                #[must_use]
                pub fn top_left(&self) -> Point<$type, U> {
                    Point::new(self.x, self.y)
                }

                #[must_use]
                pub fn right(&self) -> $type {
                    self.x + self.width
                }

                #[must_use]
                pub fn top_right(&self) -> Point<$type, U> {
                    Point::new(self.x + self.width, self.y)
                }

                #[must_use]
                pub fn bottom(&self) -> $type {
                    self.y + self.height
                }

                #[must_use]
                pub fn bottom_left(&self) -> Point<$type, U> {
                    Point::new(self.x, self.y + self.height)
                }

                #[must_use]
                pub fn bottom_right(&self) -> Point<$type, U> {
                    Point::new(self.x + self.width, self.y + self.height)
                }

                #[inline]
                #[must_use]
                pub fn to_xywh(&self) -> [$type; 4] {
                    [self.x, self.y, self.width, self.height]
                }

                #[must_use]
                pub fn size(&self) -> crate::math::Size<$type, U> {
                    crate::math::Size::new(self.width, self.height)
                }

                #[must_use]
                pub fn retype<U2>(&self) -> Rect<$type, U2> {
                    Rect::new(self.x, self.y, self.width, self.height)
                }
            }

            impl<U> From<(Point<$type, U>, Point<$type, U>)> for Rect<$type, U> {
                fn from((top_left, bottom_right): (Point<$type, U>, Point<$type, U>)) -> Self {
                    Self::from_points(top_left, bottom_right)
                }
            }

            impl<U> From<Rect<$type, U>> for (Point<$type, U>, Point<$type, U>) {
                fn from(rect: Rect<$type, U>) -> Self {
                    (rect.top_left(), rect.bottom_right())
                }
            }
        )+
    }
}

macro_rules! impl_zero {
    ($($type:ty, $zero:expr),*) => {
        $(
            impl<U> Rect<$type, U> {
                pub const ZERO: Self = Self::new($zero, $zero, $zero, $zero);
            }
        )*
    };
}

impl_zero!(i16, 0, u16, 0, f32, 0.0);
impl_rect!(i16, u16, f32);

impl<T: Default, U> Default for Rect<T, U> {
    fn default() -> Self {
        Self::new(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }
}

impl<T: Clone, U> Clone for Rect<T, U> {
    fn clone(&self) -> Self {
        Self::new(
            self.x.clone(),
            self.y.clone(),
            self.width.clone(),
            self.height.clone(),
        )
    }
}

impl<T: Copy, U> Copy for Rect<T, U> {}

impl<T: std::fmt::Debug, U> std::fmt::Debug for Rect<T, U> {
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

impl<T: std::fmt::Debug, U> std::fmt::Display for Rect<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<T: PartialEq, U> PartialEq for Rect<T, U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y && self.width == rhs.width && self.height == rhs.height
    }
}
