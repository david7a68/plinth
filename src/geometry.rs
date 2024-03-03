//! 2D geometry types [`Point`], [`Extent`], [`Rect`], and [`Scale`] and
//! type-safe coordinate spaces like [`Pixel`] and [`Wixel`].

use std::fmt::Debug;

/// A conversion factor from physical pixels to logical pixels.
pub type DpiScale = Scale<Wixel, Pixel>;

/// A scale factor for converting between two units.
#[derive(Clone, Copy, Debug)]
pub struct Scale<From: ScaleTo<To>, To> {
    pub factor: f32,
    _phantom: std::marker::PhantomData<(From, To)>,
}

impl<From: ScaleTo<To>, To> Scale<From, To> {
    #[must_use]
    pub fn new(factor: f32) -> Self {
        Self {
            factor,
            _phantom: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn scale(self, value: From) -> To {
        value.scale_to(self.factor)
    }
}

impl<From: ScaleTo<To>, To> Default for Scale<From, To> {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// A 2D extent.
#[derive(Clone, Copy, Debug, Default)]
pub struct Extent<T: Unit> {
    pub width: T,
    pub height: T,
}

impl<T: Unit> Extent<T> {
    pub const ZERO: Self = Self {
        width: T::ZERO,
        height: T::ZERO,
    };

    pub const MIN: Self = Self {
        width: T::MIN,
        height: T::MIN,
    };

    pub const MAX: Self = Self {
        width: T::MAX,
        height: T::MAX,
    };

    #[must_use]
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn cast<T2: Unit + From<T>>(self) -> Extent<T2> {
        Extent {
            width: T2::from(self.width),
            height: T2::from(self.height),
        }
    }

    #[must_use]
    pub fn scale_to<To: Unit>(self, scale: Scale<T, To>) -> Extent<To>
    where
        T: ScaleTo<To>,
    {
        Extent {
            width: scale.scale(self.width),
            height: scale.scale(self.height),
        }
    }
}

impl<T: Unit, T1: Into<T>, T2: Into<T>> From<(T1, T2)> for Extent<T> {
    fn from((width, height): (T1, T2)) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
        }
    }
}

/// A 2D point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Point<T: Unit> {
    pub x: T,
    pub y: T,
}

impl<T: Unit> Point<T> {
    pub const ZERO: Self = Self {
        x: T::ZERO,
        y: T::ZERO,
    };

    pub const MIN: Self = Self {
        x: T::MIN,
        y: T::MIN,
    };

    pub const MAX: Self = Self {
        x: T::MAX,
        y: T::MAX,
    };
}

impl<T: Unit, T1: Into<T>, T2: Into<T>> From<(T1, T2)> for Point<T> {
    fn from((x, y): (T1, T2)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

/// A 2D rectangle.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect<T: Unit> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: Unit> Rect<T> {
    pub const ZERO: Self = Self {
        x: T::ZERO,
        y: T::ZERO,
        width: T::ZERO,
        height: T::ZERO,
    };

    pub const MIN: Self = Self {
        x: T::MIN,
        y: T::MIN,
        width: T::MIN,
        height: T::MIN,
    };

    pub const MAX: Self = Self {
        x: T::MAX,
        y: T::MAX,
        width: T::MAX,
        height: T::MAX,
    };

    pub fn new(origin: impl Into<Point<T>>, extent: impl Into<Extent<T>>) -> Self {
        let origin = origin.into();
        let extent = extent.into();

        Self {
            x: origin.x,
            y: origin.y,
            width: extent.width,
            height: extent.height,
        }
    }
}

impl<T: Unit> From<Extent<T>> for Rect<T> {
    fn from(extent: Extent<T>) -> Self {
        Self {
            x: T::ZERO,
            y: T::ZERO,
            width: extent.width,
            height: extent.height,
        }
    }
}

impl Unit for i16 {
    const ZERO: Self = 0;
    const MIN: Self = i16::MIN;
    const MAX: Self = i16::MAX;
}

impl Unit for i32 {
    const ZERO: Self = 0;
    const MIN: Self = i32::MIN;
    const MAX: Self = i32::MAX;
}

impl Unit for f32 {
    const ZERO: Self = 0.0;
    const MIN: Self = f32::MIN;
    const MAX: Self = f32::MAX;
}

macro_rules! impl_unit {
    ($(#[$($meta:meta)*])* $ty:ident ($int_ty: ty, $zero:expr), ($($int:ty),*), ($($try_int:ty),*)) => {
        $(#[$($meta)*])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $ty(pub $int_ty);

        impl Unit for $ty {
            const ZERO: Self = Self($zero);
            const MIN: Self = Self(<$int_ty>::MIN);
            const MAX: Self = Self(<$int_ty>::MAX);
        }

        impl From<$int_ty> for $ty {
            fn from(value: $int_ty) -> Self {
                Self(value)
            }
        }

        impl From<$ty> for $int_ty {
            fn from(value: $ty) -> Self {
                value.0
            }
        }

        $(
            impl From<$ty> for $int {
                fn from(value: $ty) -> Self {
                    Self::from(value.0)
                }
            }
        )*

        $(
            impl TryFrom<$ty> for $try_int {
                type Error = <Self as TryFrom<$int_ty>>::Error;

                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    value.0.try_into()
                }
            }
        )*
    };
}

impl_unit!(
    /// Scale-agnostic coordinates.
    Pixel(f32, 0.0),
    (),
    ()
);

impl_unit!(
    /// Texture coordinates.
    #[derive(Eq, Ord)]
    Texel(i16, 0),
    (i32, f32),
    ()
);

impl_unit!(
    /// Window coordinates.
    #[derive(Eq, Ord)]
    Wixel(i16, 0),
    (i32, f32),
    (u32)
);

impl ScaleTo<Pixel> for Wixel {
    fn scale_to(self, factor: f32) -> Pixel {
        Pixel(f32::from(self.0) * factor)
    }
}

/// A marker trait for units that can be used in this module.
pub trait Unit: Copy + Debug + Default {
    const ZERO: Self;
    const MIN: Self;
    const MAX: Self;
}

/// A trait for scaling from one unit to another.
pub trait ScaleTo<T> {
    fn scale_to(self, factor: f32) -> T;
}
