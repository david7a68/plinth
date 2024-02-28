use crate::{frame::FramesPerSecond, system::time::Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// The target refresh rate, if a frame rate has been set.
    pub target_frame_rate: Option<FramesPerSecond>,

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
