use std::time::Duration;

use crate::system;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStatistics {
    /// The time that the last present occurred.
    pub prev_present_time: PresentInstant,

    /// The estimated time that the next present will occur.
    pub next_present_time: PresentInstant,
}

/// The time that a present has or will occur.
///
/// This is used because there is not always a way to construct `Instant`s from
/// compositor timestamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentInstant(Duration);

impl PresentInstant {
    pub const ZERO: Self = Self(Duration::ZERO);

    pub fn now() -> Self {
        Self(system::present_time_now())
    }

    pub fn elapsed(&self) -> PresentDuration {
        PresentDuration(Self::now().0 - self.0)
    }

    pub fn from_ticks(ticks: u64) -> Self {
        Self(system::present_time_from_ticks(ticks))
    }
}

impl std::ops::Add<Duration> for PresentInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub for PresentInstant {
    type Output = PresentDuration;

    fn sub(self, rhs: Self) -> Self::Output {
        PresentDuration(self.0 - rhs.0)
    }
}

impl PartialOrd for PresentInstant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentDuration(Duration);

impl PresentDuration {
    pub const ZERO: Self = Self(Duration::ZERO);

    pub fn from_secs_f32(secs: f32) -> Self {
        Self(Duration::from_secs_f32(secs))
    }
}

// impl std::ops::Div<PresentDuration> for PresentDuration {
//     type Output = f64;

//     fn div(self, rhs: PresentDuration) -> Self::Output {
//         self.0.as_secs_f64() / rhs.0.as_secs_f64()
//     }
// }

impl std::ops::Add<PresentDuration> for PresentInstant {
    type Output = Self;

    fn add(self, rhs: PresentDuration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Add<PresentInstant> for PresentDuration {
    type Output = PresentInstant;

    fn add(self, rhs: PresentInstant) -> Self::Output {
        PresentInstant(self.0 + rhs.0)
    }
}

impl std::ops::Mul<u32> for PresentDuration {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl std::ops::Mul<PresentDuration> for u32 {
    type Output = PresentDuration;

    fn mul(self, rhs: PresentDuration) -> Self::Output {
        PresentDuration(rhs.0 * self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentStatistics {
    pub monitor_rate: f32,

    pub prev_present_time: PresentInstant,

    pub next_estimated_present_time: PresentInstant,
}

impl PresentStatistics {
    pub fn frame_budget(&self) -> PresentDuration {
        self.next_estimated_present_time - self.prev_present_time
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefreshRate {
    /// The slowest acceptable refresh rate.
    ///
    /// Set this to 0 to disable the lower bound.
    pub min_fps: f32,
    /// The highest acceptable refresh rate.
    ///
    /// Set this to `f32::INFINITY` to disable the upper bound.
    pub max_fps: f32,
    /// The optimal refresh rate.
    ///
    /// Set this to 0 to disable animation.
    pub optimal_fps: f32,
}
