use std::ops::{Add, AddAssign, Sub, SubAssign};

use super::platform_impl;

pub(crate) const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;
pub(crate) const NANOSECONDS_PER_SECOND_F64: f64 = 1_000_000_000.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nanoseconds(pub i64);

impl Nanoseconds {
    pub fn now() -> Self {
        Self(platform_impl::time::now_nanoseconds())
    }

    #[cfg(target_os = "windows")]
    pub fn from_qpc_time(ticks: i64) -> Self {
        Self(platform_impl::time::qpc_to_nanoseconds(ticks))
    }
}

impl Add for Nanoseconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Nanoseconds {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Nanoseconds {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Nanoseconds {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl From<Hertz> for Nanoseconds {
    fn from(hertz: Hertz) -> Self {
        hertz.to_period()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Hertz(pub f64);

impl Hertz {
    pub fn from_period(period: Nanoseconds) -> Self {
        let period_s = period.0 / NANOSECONDS_PER_SECOND;
        let period_n = period.0 % NANOSECONDS_PER_SECOND;

        #[allow(clippy::cast_precision_loss)]
        let period_f = period_s as f64 + period_n as f64 / NANOSECONDS_PER_SECOND_F64;

        Self(1.0 / period_f)
    }

    pub fn to_period(self) -> Nanoseconds {
        let period_s = 1.0 / self.0;
        let period_n = period_s * NANOSECONDS_PER_SECOND_F64;

        #[allow(clippy::cast_possible_truncation)]
        Nanoseconds(period_n.floor() as i64)
    }
}
