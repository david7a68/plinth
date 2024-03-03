use std::ops::{Add, AddAssign, Sub};

use crate::system::time::{Hertz, Nanoseconds};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramesPerSecond(pub(crate) Hertz);

impl FramesPerSecond {
    #[must_use]
    pub const fn new(fps: f64) -> Self {
        Self(Hertz(fps))
    }

    #[must_use]
    pub fn from_period(period: PresentPeriod) -> Self {
        Self(Hertz::from_period(period.0))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresentTime(Nanoseconds);

impl PresentTime {
    #[must_use]
    pub fn now() -> Self {
        Self(Nanoseconds::now())
    }

    #[must_use]
    pub(crate) fn from_qpc_time(ticks: i64) -> Self {
        Self(Nanoseconds::from_qpc_time(ticks))
    }
}

impl Add<PresentPeriod> for PresentTime {
    type Output = Self;

    fn add(self, rhs: PresentPeriod) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<PresentPeriod> for PresentTime {
    fn add_assign(&mut self, rhs: PresentPeriod) {
        self.0 += rhs.0;
    }
}

impl Sub<PresentTime> for PresentTime {
    type Output = PresentPeriod;

    fn sub(self, rhs: PresentTime) -> Self::Output {
        PresentPeriod(self.0 - rhs.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresentPeriod(Nanoseconds);

impl From<FramesPerSecond> for PresentPeriod {
    fn from(fps: FramesPerSecond) -> Self {
        Self(fps.0.into())
    }
}
