use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Sub},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Instant(pub f64);

impl Instant {
    pub const ZERO: Self = Self(0.0);

    pub fn now() -> Self {
        Self(crate::platform::present_time_now())
    }

    pub fn elapsed(&self) -> Duration {
        Duration(crate::platform::present_time_now() - self.0)
    }

    pub fn from_ticks(ticks: u64) -> Self {
        Self(crate::platform::present_time_from_ticks(ticks))
    }

    pub fn max(&self, rhs: &Self) -> Self {
        Self(self.0.max(rhs.0))
    }

    pub fn saturating_sub(&self, rhs: &Self) -> Duration {
        Duration((self.0 - rhs.0).max(0.0))
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0;
    }
}

impl PartialOrd for Instant {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&rhs.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Duration(pub f64);

impl Duration {
    pub const ZERO: Self = Self(0.0);
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Div for Duration {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Div<f64> for Duration {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Div<Duration> for f64 {
    type Output = f64;

    fn div(self, rhs: Duration) -> Self::Output {
        self / rhs.0
    }
}

impl Mul<f64> for Duration {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Duration> for f64 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        Duration(self * rhs.0)
    }
}

impl PartialOrd for Duration {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&rhs.0)
    }
}

impl PartialEq<f64> for Duration {
    fn eq(&self, rhs: &f64) -> bool {
        self.0 == *rhs
    }
}

impl PartialEq<Duration> for f64 {
    fn eq(&self, rhs: &Duration) -> bool {
        *self == rhs.0
    }
}

impl PartialOrd<f64> for Duration {
    fn partial_cmp(&self, rhs: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(rhs)
    }
}

impl Sum for Duration {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |a, b| a + b)
    }
}

impl<'a> Sum<&'a Duration> for Duration {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |a, b| a + *b)
    }
}
