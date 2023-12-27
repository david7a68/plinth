// pub(crate) mod backend;
mod canvas;
mod color;
mod frame_statistics;
mod primitives;

pub use self::canvas::*;
pub use self::color::*;
pub use self::frame_statistics::*;
pub use self::primitives::*;

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::HighPerformance,
            debug_mode: cfg!(debug_assertions),
        }
    }
}
