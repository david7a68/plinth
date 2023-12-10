use std::time::{Duration, Instant};

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
    pub prev_present_time: Instant,

    /// The estimated time that the next present will occur.
    pub next_estimated_present: Instant,
}

impl FrameStatistics {}

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
