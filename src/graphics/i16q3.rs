use crate::geometry::{new_extent, new_point, new_rect};

new_point!(Point16(x, y, I16Q3, I16Q3(0)));
new_extent!(Extent16(I16Q3, I16Q3(0)));
new_rect!(Rect16(I16Q3, Point16, Extent16));

/// Fixed-point rational number with 3 fractional bits.
///
/// This allows us to correctly implement `Eq`, `Ord`, and `Hash` traits since
/// we don't have to deal with infinity and NaN values.
#[derive(Clone, Copy, Default, Hash)]
pub struct I16Q3(i16);

impl I16Q3 {
    const Q: i16 = 3;
    const K: i32 = (1 << (Self::Q - 1));

    pub const MIN: Self = I16Q3(i16::MIN);
    pub const MAX: Self = I16Q3(i16::MAX);

    fn saturate(value: i32) -> Self {
        Self(value.clamp(i16::MIN.into(), i16::MAX.into()) as i16)
    }

    fn saturate64(value: i64) -> Self {
        Self(value.clamp(i16::MIN.into(), i16::MAX.into()) as i16)
    }
}

impl From<f32> for I16Q3 {
    fn from(value: f32) -> Self {
        if value.is_finite() {
            let int = value * (1 << I16Q3::Q) as f32;
            Self::saturate(int.round() as i32)
        } else {
            I16Q3(0)
        }
    }
}

impl From<f64> for I16Q3 {
    fn from(value: f64) -> Self {
        if value.is_finite() {
            let int = value * (1 << I16Q3::Q) as f64;
            Self::saturate64(int.round() as i64)
        } else {
            I16Q3(0)
        }
    }
}

impl From<I16Q3> for f32 {
    fn from(value: I16Q3) -> Self {
        value.0 as f32 / (1 << I16Q3::Q) as f32
    }
}

impl From<I16Q3> for f64 {
    fn from(value: I16Q3) -> Self {
        value.0 as f64 / (1 << I16Q3::Q) as f64
    }
}

impl From<u8> for I16Q3 {
    fn from(value: u8) -> Self {
        Self((value as i16) << Self::Q)
    }
}

impl From<i8> for I16Q3 {
    fn from(value: i8) -> Self {
        Self((value as i16) << Self::Q)
    }
}

impl From<u16> for I16Q3 {
    fn from(value: u16) -> Self {
        Self::saturate((value as i32) << Self::Q)
    }
}

impl From<i16> for I16Q3 {
    fn from(value: i16) -> Self {
        Self::saturate((value as i32) << Self::Q)
    }
}

impl From<u32> for I16Q3 {
    fn from(value: u32) -> Self {
        Self::saturate64((value as i64) << Self::Q)
    }
}

impl From<i32> for I16Q3 {
    fn from(value: i32) -> Self {
        Self::saturate64((value as i64) << Self::Q)
    }
}

impl std::fmt::Debug for I16Q3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0 as f64 / (1 << Self::Q) as f64)
    }
}

impl<T: Into<I16Q3>> std::ops::Add<T> for I16Q3 {
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self(self.0 + rhs.into().0)
    }
}

impl<T: Into<I16Q3>> std::ops::AddAssign<T> for I16Q3 {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into().0;
    }
}

impl<T: Into<I16Q3>> std::ops::Sub<T> for I16Q3 {
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self(self.0 - rhs.into().0)
    }
}

impl<T: Into<I16Q3>> std::ops::SubAssign<T> for I16Q3 {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs.into().0;
    }
}

impl<T: Into<I16Q3>> std::ops::Mul<T> for I16Q3 {
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        let temp = (self.0 as i32) * (rhs.into().0 as i32);
        let temp = temp + Self::K;

        Self::saturate(temp >> Self::Q)
    }
}

impl<T: Into<I16Q3>> std::ops::MulAssign<T> for I16Q3 {
    fn mul_assign(&mut self, rhs: T) {
        *self = *self * rhs.into();
    }
}

impl<T: Into<I16Q3>> std::ops::Div<T> for I16Q3 {
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        let mut temp = (self.0 as i32) << Self::Q;
        let rhs = rhs.into().0 as i32;

        if (temp >= 0 && rhs >= 0) || (temp < 0 && rhs < 0) {
            temp += rhs / 2;
        } else {
            temp -= rhs / 2;
        }

        temp /= rhs;

        Self::saturate(temp)
    }
}

macro_rules! impl_fixed_rhs_binops {
    ($($ty:ty), +) => {
        $(
            impl std::ops::Add<I16Q3> for $ty {
                type Output = I16Q3;

                fn add(self, rhs: I16Q3) -> I16Q3 {
                    I16Q3::from(self) + rhs
                }
            }

            impl std::ops::Sub<I16Q3> for $ty {
                type Output = I16Q3;

                fn sub(self, rhs: I16Q3) -> I16Q3 {
                    I16Q3::from(self) - rhs
                }
            }

            impl std::ops::Mul<I16Q3> for $ty {
                type Output = I16Q3;

                fn mul(self, rhs: I16Q3) -> I16Q3 {
                    I16Q3::from(self) * rhs
                }
            }

            impl std::ops::Div<I16Q3> for $ty {
                type Output = I16Q3;

                fn div(self, rhs: I16Q3) -> I16Q3 {
                    I16Q3::from(self) / rhs
                }
            }

            impl std::ops::Rem<I16Q3> for $ty {
                type Output = I16Q3;

                fn rem(self, rhs: I16Q3) -> I16Q3 {
                    I16Q3::from(self) % rhs
                }
            }

            impl PartialEq<I16Q3> for $ty {
                fn eq(&self, other: &I16Q3) -> bool {
                    I16Q3::from(*self).eq(other)
                }
            }

            impl PartialOrd<I16Q3> for $ty {
                fn partial_cmp(&self, other: &I16Q3) -> Option<std::cmp::Ordering> {
                    I16Q3::from(*self).partial_cmp(other)
                }
            }
        )+
    };
}

impl_fixed_rhs_binops!(i8, i16, u8, u16, f32, f64);

impl<T: Into<I16Q3>> std::ops::DivAssign<T> for I16Q3 {
    fn div_assign(&mut self, rhs: T) {
        *self = *self / rhs.into();
    }
}

impl<T: Into<I16Q3>> std::ops::Rem<T> for I16Q3 {
    type Output = Self;

    fn rem(self, rhs: T) -> Self {
        Self(self.0 % rhs.into().0)
    }
}

impl<T: Into<I16Q3>> std::ops::RemAssign<T> for I16Q3 {
    fn rem_assign(&mut self, rhs: T) {
        self.0 %= rhs.into().0;
    }
}

impl<T: Into<I16Q3> + Copy> PartialEq<T> for I16Q3 {
    fn eq(&self, other: &T) -> bool {
        self.0 == (*other).into().0
    }
}

impl Eq for I16Q3 {}

impl<T: Into<I16Q3> + Copy> PartialOrd<T> for I16Q3 {
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&(*other).into().0)
    }
}

impl Ord for I16Q3 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::I16Q3;

    #[test]
    fn fixed32_arithmetic() {
        assert_eq!(3.5, I16Q3::from(1.0) + I16Q3::from(2.5));
        assert_eq!(-1.5, I16Q3::from(1.0) - I16Q3::from(2.5));
        assert_eq!(2.5, I16Q3::from(1.0) * I16Q3::from(2.5));
        assert_eq!(0.4, I16Q3::from(1.0) / I16Q3::from(2.5));
        assert_eq!(2.0, I16Q3::from(1.0) / I16Q3::from(0.5));

        assert_eq!(3.5, 1.0 + I16Q3::from(2.5));
        assert_eq!(-1.5, 1.0 - I16Q3::from(2.5));
        assert_eq!(2.5, 1.0 * I16Q3::from(2.5));
        assert_eq!(0.4, 1.0 / I16Q3::from(2.5));
        assert_eq!(2.0, 1.0 / I16Q3::from(0.5));

        assert_eq!(3.5, I16Q3::from(1.0) + 2.5);
        assert_eq!(-1.5, I16Q3::from(1.0) - 2.5);
        assert_eq!(2.5, I16Q3::from(1.0) * 2.5);
        assert_eq!(0.4, I16Q3::from(1.0) / 2.5);
        assert_eq!(2.0, I16Q3::from(1.0) / 0.5);
    }

    #[test]
    fn limits() {
        assert_eq!(I16Q3::from(i16::MAX as f32 + f32::EPSILON), I16Q3::MAX);
        assert_eq!(I16Q3::from(i16::MIN as f32 - f32::EPSILON), I16Q3::MIN);
        assert_eq!(I16Q3::from(0.000000001), I16Q3::from(0.0));
    }

    #[test]
    fn rounding() {
        // 3 bits, 8 options, 0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875
        let centers = [0.0f32, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0];
        let variances = [0.0f32, 0.05, 0.1];

        for center in centers {
            for variance in variances {
                {
                    let value = center + variance;
                    let fixed = I16Q3::from(value);
                    assert_eq!((value * 8.0).round() / 8.0, f32::from(fixed));
                }
                {
                    let value = center - variance;
                    let fixed = I16Q3::from(value);
                    assert_eq!((value * 8.0).round() / 8.0, f32::from(fixed));
                }
            }
        }

        // nearest value from 0.2 is 0.25, so 0.25 * 2 = 0.5
        assert_eq!(I16Q3::from(0.2) * I16Q3::from(2), I16Q3::from(0.5));
    }

    #[test]
    #[should_panic]
    fn fixed32_divide_by_zero() {
        let _ = I16Q3::from(1.0) / I16Q3::from(0.0);
    }
}
