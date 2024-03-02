use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use super::platform_impl;

pub(crate) const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;

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

impl Mul<f64> for Nanoseconds {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        let rhs = (rhs * NANOSECONDS_PER_SECOND as f64).floor() as i64;
        Self(self.0 * rhs)
    }
}

impl Div<f64> for Nanoseconds {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        let rhs = (rhs * NANOSECONDS_PER_SECOND as f64).floor() as i64;
        Self(self.0 / rhs)
    }
}

impl Div for Nanoseconds {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        let lhs_seconds = self.0 / NANOSECONDS_PER_SECOND;
        let rhs_seconds = rhs.0 / NANOSECONDS_PER_SECOND;
        let div_seconds = lhs_seconds as f64 / rhs_seconds as f64;

        let lhs_nanos = self.0 % NANOSECONDS_PER_SECOND;
        let rhs_nanos = rhs.0 % NANOSECONDS_PER_SECOND;
        let div_nanos = lhs_nanos as f64 / rhs_nanos as f64;

        div_seconds + div_nanos
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
        let period_s = period.0 as f64 / NANOSECONDS_PER_SECOND as f64;
        let hz = 1.0 / period_s;
        Self(hz)
    }

    pub fn to_period(self) -> Nanoseconds {
        let period_s = 1.0 / self.0;
        let period_n = period_s * NANOSECONDS_PER_SECOND as f64;
        Nanoseconds(period_n.floor() as i64)
    }
}
