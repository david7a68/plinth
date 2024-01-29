pub(crate) mod backend;
mod canvas;
mod color;
mod frame_statistics;
mod image;
mod primitives;

pub use self::canvas::*;
pub use self::color::*;
pub use self::frame_statistics::*;
pub use self::image::*;
pub use self::primitives::*;

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
    /// The size of the texture upload buffer in bytes.
    pub texture_upload_buffer_size: isize,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::HighPerformance,
            debug_mode: cfg!(debug_assertions),
            texture_upload_buffer_size: 655356 * 65536,
        }
    }
}
