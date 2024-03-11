//! 2D geometry types with type-safe coordinate spaces.

use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Point<T: Num> {
    pub x: T,
    pub y: T,
}

impl<T: Num> Point<T> {
    pub const ZERO: Self = Self {
        x: T::ZERO,
        y: T::ZERO,
    };

    pub const ONE: Self = Self {
        x: T::ONE,
        y: T::ONE,
    };

    pub const MIN: Self = Self {
        x: T::MIN,
        y: T::MIN,
    };

    pub const MAX: Self = Self {
        x: T::MAX,
        y: T::MAX,
    };

    pub fn new(x: impl Into<T>, y: impl Into<T>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn cast<U: Num + From<T>>(self) -> Point<U> {
        Point {
            x: U::from(self.x),
            y: U::from(self.y),
        }
    }

    pub fn scale_to<U: Num>(self, factor: Scale<T, U>) -> Point<U>
    where
        T: ScaleTo<T, U>,
    {
        Point::new(self.x.scale(factor), self.y.scale(factor))
    }
}

impl<T: Num, I: Into<T>> From<(I, I)> for Point<T> {
    fn from((x, y): (I, I)) -> Self {
        Self::new(x.into(), y.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Extent<T: Num> {
    pub width: T,
    pub height: T,
}

impl<T: Num> Extent<T> {
    pub const ZERO: Self = Self {
        width: T::ZERO,
        height: T::ZERO,
    };

    pub const ONE: Self = Self {
        width: T::ONE,
        height: T::ONE,
    };

    pub const MIN: Self = Self {
        width: T::MIN,
        height: T::MIN,
    };

    pub const MAX: Self = Self {
        width: T::MAX,
        height: T::MAX,
    };

    pub fn new(width: impl Into<T>, height: impl Into<T>) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
        }
    }

    pub fn cast<U: Num + From<T>>(self) -> Extent<U> {
        Extent {
            width: U::from(self.width),
            height: U::from(self.height),
        }
    }

    pub fn scale_to<U: Num>(self, factor: Scale<T, U>) -> Extent<U>
    where
        T: ScaleTo<T, U>,
    {
        Extent::new(self.width.scale(factor), self.height.scale(factor))
    }

    pub fn min(&self, other: &Self) -> Self
    where
        T: Ord,
    {
        Self {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }

    pub fn max(&self, other: &Self) -> Self
    where
        T: Ord,
    {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }
}

impl<T: Num, I: Into<T>> From<(I, I)> for Extent<T> {
    fn from((width, height): (I, I)) -> Self {
        Self::new(width.into(), height.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Rect<T: Num> {
    pub origin: Point<T>,
    pub extent: Extent<T>,
}

impl<T: Num> Rect<T> {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        extent: Extent::ZERO,
    };

    pub const ONE: Self = Self {
        origin: Point::ZERO,
        extent: Extent::ONE,
    };

    pub const MIN: Self = Self {
        origin: Point::MIN,
        extent: Extent::MIN,
    };

    pub const MAX: Self = Self {
        origin: Point::MAX,
        extent: Extent::MAX,
    };

    pub fn new(origin: impl Into<Point<T>>, extent: impl Into<Extent<T>>) -> Self {
        Self {
            origin: origin.into(),
            extent: extent.into(),
        }
    }

    pub fn from_extent(extent: impl Into<Extent<T>>) -> Self {
        Self {
            origin: Point::ZERO,
            extent: extent.into(),
        }
    }

    pub fn cast<U: Num + From<T>>(self) -> Rect<U> {
        Rect {
            origin: self.origin.cast(),
            extent: self.extent.cast(),
        }
    }

    pub fn scale_to<U: Num>(self, factor: Scale<T, U>) -> Rect<U>
    where
        T: ScaleTo<T, U>,
    {
        Rect::new(self.origin.scale_to(factor), self.extent.scale_to(factor))
    }

    pub fn to_xywh(&self) -> [T; 4] {
        let origin = self.origin;
        let extent = self.extent;
        [origin.x, origin.y, extent.width, extent.height]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct Aabb<T: Num> {
    pub min: Point<T>,
    pub max: Point<T>,
}

impl<T: Num> Aabb<T> {
    pub const MIN: Self = Self {
        min: Point::MIN,
        max: Point::MIN,
    };

    pub const MAX: Self = Self {
        min: Point::MAX,
        max: Point::MAX,
    };

    pub fn new(min: Point<T>, max: Point<T>) -> Self {
        Self { min, max }
    }

    pub fn cast<U: Num + From<T>>(self) -> Aabb<U> {
        Aabb {
            min: self.min.cast(),
            max: self.max.cast(),
        }
    }

    pub fn scale_to<U: Num>(self, factor: Scale<T, U>) -> Aabb<U>
    where
        T: ScaleTo<T, U>,
    {
        Aabb::new(self.min.scale_to(factor), self.max.scale_to(factor))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Scale<T: Num, U: Num> {
    pub factor: f32,
    _phantom: std::marker::PhantomData<(T, U)>,
}

impl<T: Num, U: Num> Scale<T, U> {
    pub fn new(factor: f32) -> Self {
        Self {
            factor,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub trait Num: Copy + Default + Add + Sub + Mul + Div + PartialOrd + PartialEq {
    const ZERO: Self;
    const ONE: Self;

    const MIN: Self;
    const MAX: Self;
}

pub trait ScaleTo<T: Num, U: Num> {
    fn scale(&self, factor: Scale<T, U>) -> U;
}

macro_rules! impl_num {
    ($(#[$meta:meta])* $name:ident($int_ty:ty)) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(pub $int_ty);

        impl Add for $name {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                Self(self.0 + other.0)
            }
        }

        impl Sub for $name {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                Self(self.0 - other.0)
            }
        }

        impl Mul for $name {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                Self(self.0 * other.0)
            }
        }

        impl Div for $name {
            type Output = Self;

            fn div(self, other: Self) -> Self {
                Self(self.0 / other.0)
            }
        }

        impl From<$int_ty> for $name {
            fn from(value: $int_ty) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $int_ty {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<$int_ty> for $name {
            fn eq(&self, other: &$int_ty) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for $int_ty {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }

        impl Num for $name {
            const ZERO: Self = Self(<$int_ty>::ZERO);
            const ONE: Self = Self(<$int_ty>::ONE);

            const MIN: Self = Self(<$int_ty>::MIN);
            const MAX: Self = Self(<$int_ty>::MAX);
        }
    };
}

impl_num!(Pixel(f32));

impl_num!(
    #[derive(Eq, Hash)]
    Texel(i16)
);

pub use wixel::Wixel;
mod wixel {
    use super::*;

    impl_num!(Wixel(i16));

    impl Default for Extent<Wixel> {
        /// Special case for the default extent of a window.
        ///
        /// Defaults to [`WINDOW_EXTENT`](crate::limits::WINDOW_EXTENT)'s minimum extent.
        fn default() -> Self {
            crate::limits::WINDOW_EXTENT.min()
        }
    }

    impl ScaleTo<Wixel, Pixel> for Wixel {
        fn scale(&self, factor: Scale<Wixel, Pixel>) -> Pixel {
            Pixel(self.0 as f32 * factor.factor)
        }
    }

    impl From<Wixel> for i32 {
        fn from(value: Wixel) -> Self {
            value.0 as i32
        }
    }

    impl From<Wixel> for f32 {
        fn from(value: Wixel) -> Self {
            value.0 as f32
        }
    }

    impl TryFrom<Wixel> for u32 {
        type Error = std::num::TryFromIntError;

        fn try_from(value: Wixel) -> Result<Self, Self::Error> {
            value.0.try_into()
        }
    }

    impl TryFrom<i32> for Wixel {
        type Error = std::num::TryFromIntError;

        fn try_from(value: i32) -> Result<Self, Self::Error> {
            Ok(Wixel(i16::try_from(value)?))
        }
    }

    impl TryFrom<Extent<i32>> for Extent<Wixel> {
        type Error = std::num::TryFromIntError;

        fn try_from(value: Extent<i32>) -> Result<Self, Self::Error> {
            Ok(Extent {
                width: Wixel(i16::try_from(value.width)?),
                height: Wixel(i16::try_from(value.height)?),
            })
        }
    }
}

macro_rules! impl_prim {
    ($($t:ty),+) => {
        $(
            impl Num for $t {
                const ZERO: Self = 0;
                const ONE: Self = 1;

                const MIN: Self = Self::MIN;
                const MAX: Self = Self::MAX;
            }
        )+
    };
}

impl_prim!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl Num for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    const MIN: Self = Self::MIN;
    const MAX: Self = Self::MAX;
}

impl Num for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    const MIN: Self = Self::MIN;
    const MAX: Self = Self::MAX;
}