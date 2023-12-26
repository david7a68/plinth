use std::{
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInterval {
    pub num_frames: i64,
    pub time: Duration,
}

impl FrameInterval {
    pub fn as_frames_per_second(&self) -> FramesPerSecond {
        FramesPerSecond(1.0 / self.time.0)
    }

    pub fn as_seconds_per_frame(&self) -> SecondsPerFrame {
        SecondsPerFrame(self.time)
    }
}

pub struct FrameTime {
    pub index: u64,
    pub time: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramesPerSecond(pub f64);

impl FramesPerSecond {
    pub const ZERO: Self = Self(0.0);

    pub fn round(self) -> Self {
        Self(self.0.round())
    }

    pub fn frame_time(self) -> SecondsPerFrame {
        SecondsPerFrame(Duration(1.0) / self.0)
    }

    pub fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }
}

impl PartialOrd for FramesPerSecond {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&rhs.0)
    }
}

impl PartialEq<f64> for FramesPerSecond {
    fn eq(&self, rhs: &f64) -> bool {
        self.0 == *rhs
    }
}

impl PartialOrd<f64> for FramesPerSecond {
    fn partial_cmp(&self, rhs: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(rhs)
    }
}

impl Add<f64> for FramesPerSecond {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Div for FramesPerSecond {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Div<f64> for FramesPerSecond {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SecondsPerFrame(pub Duration);

impl SecondsPerFrame {
    pub const ZERO: Self = Self(Duration::ZERO);

    pub fn as_frames_per_second(&self) -> FramesPerSecond {
        FramesPerSecond(1.0 / self.0 .0)
    }
}

impl From<FramesPerSecond> for SecondsPerFrame {
    fn from(fps: FramesPerSecond) -> Self {
        Self(Duration(1.0) / fps.0)
    }
}

impl From<SecondsPerFrame> for Duration {
    fn from(spf: SecondsPerFrame) -> Self {
        spf.0
    }
}

impl From<Duration> for SecondsPerFrame {
    fn from(d: Duration) -> Self {
        Self(d)
    }
}

impl Add<Instant> for SecondsPerFrame {
    type Output = Instant;

    fn add(self, rhs: Instant) -> Self::Output {
        rhs + self.0
    }
}

impl Add<SecondsPerFrame> for Instant {
    type Output = Self;

    fn add(self, rhs: SecondsPerFrame) -> Self::Output {
        self + rhs.0
    }
}

impl Add for SecondsPerFrame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<f64> for SecondsPerFrame {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<SecondsPerFrame> for f64 {
    type Output = Duration;

    fn mul(self, rhs: SecondsPerFrame) -> Self::Output {
        self * rhs.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Instant(pub f64);

impl Instant {
    pub const ZERO: Self = Self(0.0);

    pub fn now() -> Self {
        Self(crate::system::present_time_now())
    }

    pub fn elapsed(&self) -> Duration {
        Duration(crate::system::present_time_now() - self.0)
    }

    pub fn from_ticks(ticks: u64) -> Self {
        Self(crate::system::present_time_from_ticks(ticks))
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

impl PartialOrd for Instant {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&rhs.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Duration(pub f64);

impl Duration {
    pub const ZERO: Self = Self(0.0);

    /// The number of frames needed to cover the given duration.
    pub fn frames_for(self, duration: Duration) -> FrameInterval {
        let num_frames = (duration.0 / self.0).ceil() as i64;
        let time = self.0 * num_frames as f64;

        FrameInterval {
            num_frames,
            time: Self(time),
        }
    }
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
