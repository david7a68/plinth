use std::time::Instant;

pub struct PresentTiming {
    pub next_frame: Instant,
    pub last_frame: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationFrequency {
    /// The minimum rate at which the window would like to receive repaint events.
    pub min_fps: Option<f32>,
    /// The maximum rate at which the window would like to receive repaint events.
    pub max_fps: Option<f32>,
    /// The optimal rate at which the window would like to receive repaint events.
    pub optimal_fps: f32,
}
