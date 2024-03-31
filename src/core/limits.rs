use crate::geometry::{Texel, Wixel};

pub struct Limit<T>(pub T);

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

    pub fn test<E>(&self, value: &str, error: E) -> Result<(), E> {
        if value.len() <= LIMIT {
            Ok(())
        } else {
            Err(error)
        }
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

            pub const fn min(&self) -> crate::geometry::Extent<$num> {
                crate::geometry::Extent {
                    width: $new(MIN_X),
                    height: $new(MIN_Y),
                }
            }

            pub const fn max(&self) -> crate::geometry::Extent<$num> {
                crate::geometry::Extent {
                    width: $new(MAX_X),
                    height: $new(MAX_Y),
                }
            }

            pub fn test(&self, value: impl TryInto<crate::geometry::Extent<$num>>) -> bool {
                let Ok(value): Result<crate::geometry::Extent<$num>, _> = value.try_into() else {
                    return false;
                };

                value.width.0 >= MIN_X
                    && value.height.0 >= MIN_Y
                    && value.width.0 <= MAX_X
                    && value.height.0 <= MAX_Y
            }

            pub fn check(&self, value: impl TryInto<crate::geometry::Extent<$num>>) {
                let Ok(value): Result<crate::geometry::Extent<$num>, _> = value.try_into() else {
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

            pub fn check_debug(&self, value: impl TryInto<crate::geometry::Extent<$num>>) {
                #[cfg(debug_assertions)]
                self.check(value);
            }
        }
    };
}

extent_min_max!(WixelExtent, Wixel, i16, Wixel);
extent_min_max!(TexelExtentRange, Texel, i16, Texel);