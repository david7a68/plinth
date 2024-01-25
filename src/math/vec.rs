pub struct Vec2<T, U = ()> {
    pub x: T,
    pub y: T,
    _unit: std::marker::PhantomData<U>,
}

macro_rules! impl_binop {
    ($ty:ty, $($trait:ident, $fn:ident, $op:tt),*) => {
        $(
            impl<U, R: Into<Vec2<$ty, U>>> std::ops::$trait<R> for Vec2<$ty, U>
            {
                type Output = Self;

                #[inline]
                fn $fn(self, rhs: R) -> Self::Output {
                    let rhs = rhs.into();
                    Self::new(self.x $op rhs.x, self.y $op rhs.y)
                }
            }
        )*
    };
}

macro_rules! impl_op_assign {
    ($ty:ty, $($trait:ident, $fn:ident, $op:tt),*) => {
        $(
            impl<U, R: Into<Vec2<$ty, U>>> std::ops::$trait<R> for Vec2<$ty, U>
            {
                #[inline]
                fn $fn(&mut self, rhs: R) {
                    let rhs = rhs.into();
                    self.x $op rhs.x;
                    self.y $op rhs.y;
                }
            }
        )*
    };
}

macro_rules! impl_vec2_ops {
    ($($ty:ty),*) => {
        $(
            impl<U> From<$ty> for Vec2<$ty, U> {
                #[inline]
                fn from(x: $ty) -> Self {
                    Self::new(x, x)
                }
            }

            impl<U> From<($ty, $ty)> for Vec2<$ty, U> {
                #[inline]
                fn from((x, y): ($ty, $ty)) -> Self {
                    Self::new(x, y)
                }
            }

            impl<U> From<Vec2<$ty, U>> for ($ty, $ty) {
                #[inline]
                fn from(v: Vec2<$ty, U>) -> Self {
                    (v.x, v.y)
                }
            }

            impl<U> PartialEq for Vec2<$ty, U> {
                #[inline]
                fn eq(&self, rhs: &Self) -> bool {
                    self.x == rhs.x && self.y == rhs.y
                }
            }

            impl_binop!($ty, Add, add, +);
            impl_binop!($ty, Sub, sub, -);
            impl_binop!($ty, Mul, mul, *);
            impl_binop!($ty, Div, div, /);
            impl_binop!($ty, Rem, rem, %);

            impl_op_assign!($ty, AddAssign, add_assign, +=);
            impl_op_assign!($ty, SubAssign, sub_assign, -=);
            impl_op_assign!($ty, MulAssign, mul_assign, *=);
            impl_op_assign!($ty, DivAssign, div_assign, /=);
            impl_op_assign!($ty, RemAssign, rem_assign, %=);
        )*
    };
}

impl_vec2_ops!(u16, i16, f32);

macro_rules! impl_f32_from_promote {
    ($($other:ty),*) => {
        $(
            impl<U> From<Vec2<$other, U>> for Vec2<f32, U> {
                #[inline]
                fn from(v: Vec2<$other, U>) -> Self {
                    Self::new(f32::from(v.x), f32::from(v.y))
                }
            }

            impl<U> From<($other, $other)> for Vec2<f32, U> {
                #[inline]
                fn from((x, y): ($other, $other)) -> Self {
                    Self::new(f32::from(x), f32::from(y))
                }
            }
        )*
    };
}

impl_f32_from_promote!(u16, i16);

impl<T, U> Vec2<T, U> {
    #[inline]
    #[must_use]
    pub fn new(x: T, y: T) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }

    #[inline]
    #[must_use]
    pub fn splat(x: T) -> Self
    where
        T: Copy,
    {
        Self::new(x, x)
    }

    #[inline]
    #[must_use]
    pub fn retype<U2>(self) -> Vec2<T, U2> {
        Vec2::new(self.x, self.y)
    }
}

impl<T: Clone, U> Clone for Vec2<T, U> {
    fn clone(&self) -> Self {
        Self::new(self.x.clone(), self.y.clone())
    }
}

impl<T: Copy, U> Copy for Vec2<T, U> {}

impl<T: Default, U> Default for Vec2<T, U> {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl<T: std::fmt::Debug, U> std::fmt::Debug for Vec2<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vec2")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<T: std::fmt::Display + std::fmt::Debug, U> std::fmt::Display for Vec2<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<T, U> std::ops::Index<usize> for Vec2<T, U> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl<T, U> std::ops::IndexMut<usize> for Vec2<T, U> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16_ops() {
        let a = Vec2::<u16>::new(2, 6);

        // implicit conversion
        assert_eq!(a * 2, (2, 12).into());
        assert_eq!(a / 2, (1, 3).into());
        assert_eq!(a % 3, (2, 0).into());

        // explicit conversion
        assert_eq!(a * Vec2::new(2, 2), (4, 12).into());
        assert_eq!(a / Vec2::new(2, 2), (1, 3).into());
        assert_eq!(a % Vec2::splat(3), (2, 0).into());

        let mut b = a;

        b *= 2;
        assert_eq!(b, (4, 12).into());
        b /= 2;
        assert_eq!(b, (2, 6).into());
        b %= 3;
        assert_eq!(b, (2, 0).into());
    }

    #[test]
    fn i16_ops() {
        let a = Vec2::<i16>::new(2, 6);

        // implicit conversion
        assert_eq!(a * 2, (2, 12).into());
        assert_eq!(a / 2, (1, 3).into());
        assert_eq!(a % 3, (2, 0).into());

        // explicit conversion
        assert_eq!(a * Vec2::new(2, 2), (4, 12).into());
        assert_eq!(a / Vec2::new(2, 2), (1, 3).into());
        assert_eq!(a % Vec2::splat(3), (2, 0).into());

        let mut b = a;

        b *= 2;
        assert_eq!(b, (4, 12).into());
        b /= 2;
        assert_eq!(b, (2, 6).into());
        b %= 3;
        assert_eq!(b, (2, 0).into());
    }

    #[test]
    fn f32_ops() {
        let a = Vec2::<f32>::new(1.0, 2.0);

        // implicit conversion
        assert_eq!(a * 2.0, (2.0, 4.0).into());
        assert_eq!(a / 2.0, (0.5, 1.0).into());
        assert_eq!(a % 2.0, (1.0, 0.0).into());

        // explicit conversion
        assert_eq!(a * Vec2::new(2.0, 2.0), (2.0, 4.0).into());
        assert_eq!(a / Vec2::new(2.0, 2.0), (0.5, 1.0).into());
        assert_eq!(a % Vec2::new(2.0, 2.0), (1.0, 0.0).into());

        let mut b = a;

        b *= 2.0;
        assert_eq!(b, (2.0, 4.0).into());
        b /= 2.0;
        assert_eq!(b, (1.0, 2.0).into());
        b %= 2.0;
        assert_eq!(b, (1.0, 0.0).into());
    }
}
