use std::ops::{Add, AddAssign, Div, Mul};

use crate::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameId(pub u64);

impl Add<u16> for FrameId {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl AddAssign<u16> for FrameId {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs as u64;
    }
}

impl Add<u32> for FrameId {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl AddAssign<u32> for FrameId {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs as u64;
    }
}

impl Add<u64> for FrameId {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for FrameId {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RefreshRate {
    /// The slowest acceptable refresh rate.
    ///
    /// Set this to 0 to disable the lower bound.
    pub min: FramesPerSecond,
    /// The highest acceptable refresh rate.
    ///
    /// Set this to `f32::INFINITY` to disable the upper bound.
    pub max: FramesPerSecond,
    /// The optimal refresh rate.
    ///
    /// Set this to 0 to disable animation.
    pub now: FramesPerSecond,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RedrawRequest {
    Idle,
    /// Redraw once, as soon as possible.
    Once,
    /// Redraw once, to present on the target frame.
    AtFrame(FrameId),
    /// Redraw continuously at or above the target frame rate.
    AtFrameRate(FramesPerSecond),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramesPerSecond(pub f64);

impl FramesPerSecond {
    pub const ZERO: Self = Self(0.0);

    pub fn from_frame_time(frame_time: Duration) -> Self {
        Self(1.0 / frame_time.0)
    }

    pub fn round(self) -> Self {
        Self(self.0.round())
    }

    pub fn frame_time(self) -> Duration {
        Duration(1.0) / self.0
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
