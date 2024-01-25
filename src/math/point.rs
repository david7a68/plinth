pub struct Point<T, U = ()> {
    pub x: T,
    pub y: T,
    _unit: std::marker::PhantomData<U>,
}

impl<T, U> Point<T, U> {
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
    pub fn retype<U2>(self) -> Point<T, U2> {
        Point::new(self.x, self.y)
    }
}

impl<T: Clone, U> Clone for Point<T, U> {
    fn clone(&self) -> Self {
        Self::new(self.x.clone(), self.y.clone())
    }
}

impl<T: Copy, U> Copy for Point<T, U> {}

impl<T: Default, U> Default for Point<T, U> {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl<T: std::fmt::Debug, U> std::fmt::Debug for Point<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<T, U> std::ops::Index<usize> for Point<T, U> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

impl<T, U> std::ops::IndexMut<usize> for Point<T, U> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Index out of bounds"),
        }
    }
}

macro_rules! impl_from {
    ($($t:ty),+) => {
        $(
            impl<U> From<$t> for Point<$t, U> {
                fn from(value: $t) -> Self {
                    Self::new(value, value)
                }
            }

            impl<U> From<($t, $t)> for Point<$t, U> {
                fn from(value: ($t, $t)) -> Self {
                    Self::new(value.0, value.1)
                }
            }
        )+
    };
}

impl_from!(i16, u16, f32);
