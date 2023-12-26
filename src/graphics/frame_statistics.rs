pub(crate) use crate::time::FramesPerSecond;
use crate::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    pub frame_rate: FramesPerSecond,

    /// The time that the last present occurred.
    pub prev_present_time: Instant,

    /// The estimated time that the next present will occur.
    pub next_present_time: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PresentStatistics {
    pub monitor_rate: FramesPerSecond,

    pub prev_present_time: Instant,
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
