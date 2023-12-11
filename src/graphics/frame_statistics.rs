use std::time::Duration;

use crate::system;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStatistics {
    /// The time that could have been used for rendering the previous frame.
    pub prev_max_frame_budget: Duration,

    /// The adjusted frame budget for rendering the previous frame.
    ///
    /// A window may reduce the frame budget if its contents can be rendered
    /// quickly in order to reduce input latency. That is to say, if the window
    /// is refreshing at 30 fps on a 60 Hz display (presenting every 2
    /// refreshes) but only needs 1 ms to render, it may choose to allocate one
    /// refresh (16.6 ms) instead of two. This reduces perceived input latency
    /// by 16.6 ms.
    ///
    /// As a consequence, this value may change depending on the display refresh
    /// rate.
    pub prev_adj_frame_budget: Duration,

    /// The time that was spent on repainting the previous frame.
    pub prev_cpu_render_time: Duration,

    /// The time that the GPU spent on rendering the previous frame.
    pub prev_gpu_render_time: Duration,

    /// The total time that was spent rendering the previous frame.
    ///
    /// This may be slightly less than the sum of `cpu_render_time` and
    /// `gpu_render_time` due to overlap.
    pub prev_all_render_time: Duration,

    /// The time that the last present occurred.
    pub prev_present_time: PresentTime,

    /// The estimated time that the next present will occur.
    pub next_present_time: PresentTime,
}

/// The time that a present has or will occur.
///
/// This is used because there is not always a way to construct `Instant`s from
/// compositor timestamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentTime(Duration);

impl PresentTime {
    pub fn now() -> Self {
        Self(system::present_time_now())
    }

    pub fn from_ticks(ticks: u64) -> Self {
        Self(system::present_time_from_ticks(ticks))
    }
}

impl std::ops::Add<Duration> for PresentTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub for PresentTime {
    type Output = PresentDuration;

    fn sub(self, rhs: Self) -> Self::Output {
        PresentDuration(self.0 - rhs.0)
    }
}

impl PartialOrd for PresentTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentDuration(Duration);

impl PresentDuration {
    pub fn round_to_multiple_of(&self, other: Self) -> Self {
        Self(Duration::from_secs_f64(
            (self.0.as_secs_f64() / other.0.as_secs_f64()).round() * other.0.as_secs_f64(),
        ))
    }
}

impl std::ops::Div<PresentDuration> for PresentDuration {
    type Output = f64;

    fn div(self, rhs: PresentDuration) -> Self::Output {
        self.0.as_secs_f64() / rhs.0.as_secs_f64()
    }
}

impl std::ops::Add<PresentDuration> for PresentTime {
    type Output = Self;

    fn add(self, rhs: PresentDuration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Add<PresentTime> for PresentDuration {
    type Output = PresentTime;

    fn add(self, rhs: PresentTime) -> Self::Output {
        PresentTime(self.0 + rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentStatistics {
    pub current_rate: f32,

    pub prev_present_time: PresentTime,

    pub next_estimated_present_time: PresentTime,
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
