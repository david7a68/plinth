macro_rules! new_point_options {
    ($name:ident($x:ident, $y:ident)) => {};
    ($name:ident($x:ident, $y:ident), {limit: $min:expr, $max:expr, $msg:expr}) => {
        impl $crate::core::limit::Limit for $name {
            const ASSERT_MESSAGE: &'static str = $msg;

            #[inline]
            fn min() -> Self {
                Self { $x: $min, $y: $min }
            }

            #[inline]
            fn max() -> Self {
                Self { $x: $max, $y: $max }
            }

            #[inline]
            fn clamp(&mut self) {
                self.$x = self.$x.clamp(Self::min().$x, Self::max().$x);
                self.$y = self.$y.clamp(Self::min().$y, Self::max().$y);
            }

            #[inline]
            fn limit_check(&self) -> bool {
                let $x = $min <= self.$x && self.$x <= $max;
                let $y = $min <= self.$y && self.$y <= $max;
                $x && $y
            }
        }
    };
    ($name:ident($x:ident, $y:ident), {limit: $min:expr, $max:expr, $msg:expr}, $($ops:tt),*) => {
        $crate::geometry::new_point_options!($name($x, $y), { limit: $min, $max, $msg });
        $crate::geometry::new_point_options!($name($x, $y), $($ops),*);
    };
    ($name:ident($x:ident, $y:ident), Eq) => {
        impl Eq for $name {}
    };
    ($name:ident($x:ident, $y:ident), Eq, $($ops:tt),*) => {
        $crate::geometry::new_point_options($name($x, $y), Eq);
        $crate::geometry::new_point_options($name, $($ops),*);
    };
}

macro_rules! new_point {
    ($(#[$meta:meta])* $name:ident($x:ident, $y:ident), $element_ty:ty, $zero:expr) => {
        $(#[$meta:meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name {
            pub $x: $element_ty,
            pub $y: $element_ty,
        }

        impl $name {
            pub const ORIGIN: Self = Self { $x: $zero, $y: $zero };

            pub fn new($x: $element_ty, $y: $element_ty) -> Self {
                Self { $x, $y }
            }
        }

        impl From<($element_ty, $element_ty)> for $name {
            fn from(($x, $y): ($element_ty, $element_ty)) -> Self {
                Self { $x, $y }
            }
        }
    };
    ($(#[$meta:meta])* $name:ident($x:ident, $y:ident), $element_ty:ty, $zero:expr, $($ops:tt),*) => {
        new_point! {
            $(#[$meta])*
            $name($x, $y), $element_ty, $zero
        }
        $crate::geometry::new_point_options!($name($x, $y), $($ops),*);
    };
}

macro_rules! new_extent_options {
    ($name:ty) => {};
    ($name:ty, {limit: $min:expr, $max:expr, $msg:expr}) => {
        impl $crate::core::limit::Limit for $name {
            const ASSERT_MESSAGE: &'static str = $msg;

            #[inline]
            fn min() -> Self {
                Self { width: $min, height: $min }
            }

            #[inline]
            fn max() -> Self {
                Self { width: $max, height: $max }
            }

            #[inline]
            fn clamp(&mut self) {
                self.width = self.width.clamp(Self::min().width, Self::max().width);
                self.height = self.height.clamp(Self::min().height, Self::max().height);
            }

            #[inline]
            fn limit_check(&self) -> bool {
                let width = $min <= self.width && self.width <= $max;
                let height = $min <= self.height && self.height <= $max;
                width && height
            }
        }
    };
    ($name:ty, {limit: $min:expr, $max:expr, $msg:expr}, $($ops:tt),*) => {
        $crate::geometry::new_extent_options!($name, { limit: $min, $max, $msg });
        $crate::geometry::new_extent_options!($name, $($ops),*);
    };
    ($name:ty, Eq) => {
        impl Eq for $name {}
    };
    ($name:ty, Eq, $($ops:tt),*) => {
        $crate::geometry::new_extent_options($name, Eq);
        $crate::geometry::new_extent_options($name, $($ops),*);
    };
}

macro_rules! new_extent {
    ($(#[$meta:meta])* $name:ident, $element_ty:ty, $zero:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name {
            pub width: $element_ty,
            pub height: $element_ty,
        }

        impl $name {
            pub const ZERO: Self = Self { width: $zero, height: $zero };

            pub fn new(width: $element_ty, height: $element_ty) -> Self {
                Self { width, height }
            }
        }

        impl From<($element_ty, $element_ty)> for $name {
            fn from((width, height): ($element_ty, $element_ty)) -> Self {
                Self { width, height }
            }
        }
    };
    ($(#[$meta:meta])* $name:ident, $element_ty:ty, $zero:expr, $($ops:tt),*) => {
        $crate::geometry::new_extent! {
            $(#[$meta])*
            $name, $element_ty, $zero
        }
        $crate::geometry::new_extent_options!($name, $($ops),*);
    };
}

macro_rules! new_rect {
    ($(#[$meta:meta])* $name:ident, $element_ty:ty, $point_ty:ty, $extent_ty:ty) => {
        $(#[$meta:meta])*
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name {
            pub origin: $point_ty,
            pub extent: $extent_ty,
        }

        impl $name {
            pub const ZERO: Self = Self { origin: <$point_ty>::ORIGIN, extent: <$extent_ty>::ZERO };

            pub fn new(origin: $point_ty, extent: $extent_ty) -> Self {
                Self { origin, extent }
            }
        }

        impl From<($element_ty, $element_ty, $element_ty, $element_ty)> for $name {
            fn from((x, y, w, h): ($element_ty, $element_ty, $element_ty, $element_ty)) -> Self {
                Self { origin: <$point_ty>::new(x, y), extent: <$extent_ty>::new(w, h) }
            }
        }
    };
}

pub(crate) use new_extent;
pub(crate) use new_extent_options;
pub(crate) use new_point;
pub(crate) use new_point_options;
pub(crate) use new_rect;

new_point!(Point(x, y), f32, 0.0);
new_extent!(Extent, f32, 0.0);
new_rect!(Rect, f32, Point, Extent);

impl Rect {
    pub fn to_xywh(&self) -> [f32; 4] {
        [
            self.origin.x,
            self.origin.y,
            self.extent.width,
            self.extent.height,
        ]
    }
}
