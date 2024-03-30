//! 2D geometry types with type-safe coordinate spaces.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

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

impl<T: Num> Div for Extent<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self {
            width: self.width / other.width,
            height: self.height / other.height,
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

impl<T: Num, I: Into<T>> From<(I, I, I, I)> for Rect<T> {
    fn from((x, y, w, h): (I, I, I, I)) -> Self {
        Self::new((x, y), (w, h))
    }
}

impl<T: Num, I: Into<T>> From<((I, I), (I, I))> for Rect<T> {
    fn from(((x, y), (w, h)): ((I, I), (I, I))) -> Self {
        Self::new((x, y), (w, h))
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
    ($(#[$meta:meta])* $name:ident($int_ty:ty): From($($from_ty:ty),*), Into($($into_ty:ty),*) ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(pub $int_ty);

        impl Add for $name {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                Self(self.0 + other.0)
            }
        }

        impl Add<$int_ty> for $name {
            type Output = Self;

            fn add(self, other: $int_ty) -> Self {
                Self(self.0 + other)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, other: Self) {
                self.0 += other.0;
            }
        }

        impl AddAssign<$int_ty> for $name {
            fn add_assign(&mut self, other: $int_ty) {
                self.0 += other;
            }
        }

        impl Sub for $name {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                Self(self.0 - other.0)
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, other: Self) {
                self.0 -= other.0;
            }
        }

        impl SubAssign<$int_ty> for $name {
            fn sub_assign(&mut self, other: $int_ty) {
                self.0 -= other;
            }
        }

        impl Mul for $name {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                Self(self.0 * other.0)
            }
        }

        impl MulAssign for $name {
            fn mul_assign(&mut self, other: Self) {
                self.0 *= other.0;
            }
        }

        impl MulAssign<$int_ty> for $name {
            fn mul_assign(&mut self, other: $int_ty) {
                self.0 *= other;
            }
        }

        impl Div for $name {
            type Output = Self;

            fn div(self, other: Self) -> Self {
                Self(self.0 / other.0)
            }
        }

        impl DivAssign for $name {
            fn div_assign(&mut self, other: Self) {
                self.0 /= other.0;
            }
        }

        impl DivAssign<$int_ty> for $name {
            fn div_assign(&mut self, other: $int_ty) {
                self.0 /= other;
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

        $(
            impl From<$from_ty> for $name {
                fn from(value: $from_ty) -> Self {
                    Self(value as $int_ty)
                }
            }
        )*

        $(
            impl From<$name> for $into_ty {
                fn from(value: $name) -> Self {
                    <$into_ty>::from(value.0)
                }
            }
        )*

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

impl_num!(Pixel(f32): From(), Into());

impl_num!(
    #[derive(Eq, Hash, Ord)]
    Texel(i16):
    From(),
    Into()
);

impl From<Extent<Texel>> for Extent<f32> {
    fn from(value: Extent<Texel>) -> Self {
        Extent::new(value.width.0 as f32, value.height.0 as f32)
    }
}

impl From<Texel> for Extent<f32> {
    fn from(value: Texel) -> Self {
        Extent::new(value.0 as f32, value.0 as f32)
    }
}

impl TryFrom<Texel> for u64 {
    type Error = std::num::TryFromIntError;

    fn try_from(value: Texel) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl_num!(
    UV(f32):
    From(),
    Into()
);

impl ScaleTo<Texel, UV> for Texel {
    fn scale(&self, factor: Scale<Texel, UV>) -> UV {
        UV(self.0 as f32 * factor.factor)
    }
}

pub use wixel::Wixel;
mod wixel {
    use super::*;

    impl_num!(
        #[derive(Eq, Hash, Ord)]
        Wixel(i16):
        From(),
        Into(i32, f32)
    );

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
