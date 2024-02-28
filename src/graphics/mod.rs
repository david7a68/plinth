mod backend;
mod canvas;
mod color;
mod frame_statistics;
mod primitives;

use crate::system::power::PowerPreference;

pub use self::canvas::*;
pub use self::color::*;
pub use self::frame_statistics::*;
pub use self::primitives::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    #[default]
    Auto,
    #[cfg(target_os = "windows")]
    Dx12,
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
            power_preference: PowerPreference::MaxPerformance,
            debug_mode: cfg!(debug_assertions),
            texture_upload_buffer_size: 655356 * 65536,
        }
    }
}

enum GraphicsImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Graphics),
}

pub struct Graphics {
    device: GraphicsImpl,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        todo!()
    }
}

enum ContextImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Context),
}

pub struct Context {
    context: ContextImpl,
}

impl Context {}
