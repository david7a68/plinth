use crate::time::Instant;
pub(crate) use crate::time::{FrameInterval, FramesPerSecond, SecondsPerFrame};

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
    pub time: Instant,
}

impl std::ops::Sub for Present {
    type Output = FrameInterval;

    fn sub(self, rhs: Self) -> Self::Output {
        FrameInterval {
            num_frames: self.id as i64 - rhs.id as i64, // todo: overflow check
            time: (self.time - rhs.time).into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PresentStatistics {
    pub monitor_rate: FramesPerSecond,

    pub prev_present_time: Instant,

    pub next_estimated_present_time: Instant,
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
