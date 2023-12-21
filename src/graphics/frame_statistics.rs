use std::time::Duration;

use crate::system;
pub(crate) use crate::time::{FramesPerSecond, Interval, SecondsPerFrame};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// The time that the last present occurred.
    pub prev_present: Present,

    /// The estimated time that the next present will occur.
    pub next_present: Present,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Present {
    pub id: u64,
    pub time: PresentInstant,
}

impl std::ops::Sub for Present {
    type Output = Interval;

    fn sub(self, rhs: Self) -> Self::Output {
        Interval {
            num_frames: self.id as i64 - rhs.id as i64, // todo: overflow check
            time: self.time - rhs.time,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentInstant {
    time: Duration,
}

impl PresentInstant {
    pub const ZERO: Self = Self {
        time: Duration::ZERO,
    };

    pub fn now() -> Self {
        Self {
            time: system::present_time_now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        system::present_time_now() - self.time
    }

    pub(crate) fn from_ticks(ticks: u64, frequency: u64) -> Self {
        Self {
            time: system::present_time_from_ticks(ticks, frequency),
        }
    }
}

impl std::ops::Sub for PresentInstant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.time - rhs.time
    }
}

impl std::ops::Add<Duration> for PresentInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            time: self.time + rhs,
        }
    }
}

impl std::cmp::PartialOrd for PresentInstant {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&rhs.time)
    }
}

impl std::ops::Add<SecondsPerFrame> for PresentInstant {
    type Output = Self;

    fn add(self, rhs: SecondsPerFrame) -> Self::Output {
        Self {
            time: self.time + Duration::from_secs_f64(rhs.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PresentStatistics {
    pub monitor_rate: FramesPerSecond,

    pub prev_present_time: PresentInstant,

    pub next_estimated_present_time: PresentInstant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RefreshRate {
    /// The slowest acceptable refresh rate.
    ///
    /// Set this to 0 to disable the lower bound.
    pub min_fps: FramesPerSecond,
    /// The highest acceptable refresh rate.
    ///
    /// Set this to `f32::INFINITY` to disable the upper bound.
    pub max_fps: FramesPerSecond,
    /// The optimal refresh rate.
    ///
    /// Set this to 0 to disable animation.
    pub optimal_fps: FramesPerSecond,
}
