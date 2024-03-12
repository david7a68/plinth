//! Static limits and constraints.

use crate::geometry::{Extent, Texel, Wixel};

/// Maximum number of windows that can be open at once.
pub const MAX_WINDOWS: Usize<8> = Usize::new(
    |Limit(limit), value| *value < limit,
    "Too many windows open",
);

/// The maximum number of UTF-8 bytes that can be used to represent a window title.
pub const MAX_WINDOW_TITLE_LENGTH: StrLen<256> = StrLen::new("Window title too long");

/// The maximum and minimum size of a window (inclusive).
pub const WINDOW_EXTENT: WixelExtent<100, 100, { i16::MAX }, { i16::MAX }> =
    WixelExtent::new("Window extent out of range.");

pub const MAX_ITEMS_PER_DRAW_LIST: Usize<{ u32::MAX as _ }> = Usize::new(
    |Limit(limit), value| *value < limit,
    "Too many items in draw list",
);

/// The maximum number of UTF-8 bytes that can be used to represent a path to a
/// resource.
pub const MAX_RESOURCE_PATH_LENGTH: StrLen<1024> = StrLen::new("Resource path too long");

pub const IMAGE_EXTENT: TexelExtent<1, 1, 4096, 4096> =
    TexelExtent::new("Image extent out of range.");

pub const MAX_IMAGE_COUNT: Usize<1023> = // 1023 since 0 is reserved for "no image"
    Usize::new(|Limit(limit), value| *value < limit, "Too many images");

/// Enforces the maximum number of items of each kind in a draw list and returns
/// the value as a `u32`.
///
/// ## Panics
///
/// Panics if `value` is greater than [`MAX_ITEMS_PER_DRAW_LIST`].
#[must_use]
pub(crate) fn enforce_draw_list_max_commands_u32(value: usize) -> u32 {
    MAX_ITEMS_PER_DRAW_LIST.check(&value);
    u32::try_from(value).unwrap()
}

pub struct Limit<T>(T);

pub struct Usize<const LIMIT: usize> {
    check: fn(Limit<usize>, &usize) -> bool,
    error: &'static str,
}

impl<const LIMIT: usize> Usize<LIMIT> {
    pub const fn new(check: fn(Limit<usize>, &usize) -> bool, error: &'static str) -> Self {
        Self { check, error }
    }

    pub const fn get(&self) -> usize {
        LIMIT
    }

    pub fn check(&self, value: &usize) {
        assert!((self.check)(Limit(LIMIT), value), "{}", self.error);
    }

    pub fn check_debug(&self, value: &usize) {
        #[cfg(debug_assertions)]
        self.check(value);
    }
}

pub struct StrLen<const LIMIT: usize> {
    error: &'static str,
}

impl<const LIMIT: usize> StrLen<LIMIT> {
    pub const fn new(error: &'static str) -> Self {
        Self { error }
    }

    pub const fn get(&self) -> usize {
        LIMIT
    }

    pub fn test<E>(&self, value: &str, error: E) -> Result<(), E> {
        if value.len() <= LIMIT {
            Ok(())
        } else {
            Err(error)
        }
    }

    pub const fn check(&self, value: &str) {
        assert!(value.len() <= LIMIT, "{}", self.error);
    }

    pub fn clamp<'a>(&self, value: &'a str) -> &'a str {
        &value[..LIMIT]
    }

    pub fn check_debug(&self, value: &str) {
        #[cfg(debug_assertions)]
        self.check(value);
    }
}

impl<const LIMIT: usize> std::fmt::Display for StrLen<LIMIT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LIMIT)
    }
}

macro_rules! extent_min_max {
    ($name:ident, $num:ty, $int:ty, $new:tt) => {
        pub struct $name<const MIN_X: $int, const MIN_Y: $int, const MAX_X: $int, const MAX_Y: $int>
        {
            error: &'static str,
        }

        impl<const MIN_X: $int, const MIN_Y: $int, const MAX_X: $int, const MAX_Y: $int>
            $name<MIN_X, MIN_Y, MAX_X, MAX_Y>
        {
            pub const fn new(error: &'static str) -> Self {
                Self { error }
            }

            pub const fn min(&self) -> Extent<$num> {
                Extent {
                    width: $new(MIN_X),
                    height: $new(MIN_Y),
                }
            }

            pub const fn max(&self) -> Extent<$num> {
                Extent {
                    width: $new(MAX_X),
                    height: $new(MAX_Y),
                }
            }

            pub fn check(&self, value: impl TryInto<Extent<$num>>) {
                let Ok(value): Result<Extent<$num>, _> = value.try_into() else {
                    panic!("{}", self.error)
                };

                assert!(
                    value.width.0 >= MIN_X,
                    "width({}) >= MIN_X({}) is false: {}",
                    value.width.0,
                    MIN_X,
                    self.error
                );

                assert!(
                    value.height.0 >= MIN_Y,
                    "height({}) >= MIN_Y({}) is false: {}",
                    value.height.0,
                    MIN_Y,
                    self.error
                );

                assert!(value.width.0 <= MAX_X, "{}", self.error);

                assert!(value.height.0 <= MAX_Y, "{}", self.error);
            }

            pub fn check_debug(&self, value: impl TryInto<Extent<$num>>) {
                #[cfg(debug_assertions)]
                self.check(value);
            }
        }
    };
}

extent_min_max!(WixelExtent, Wixel, i16, Wixel);
extent_min_max!(TexelExtent, Texel, i16, Texel);
