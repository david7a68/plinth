mod frame;
pub mod geometry;
mod graphics;
pub mod limits;
mod system;

mod application;

pub use application::{AppContext, Application, EventHandler};
pub use graphics::{Canvas, Color, FrameInfo, GraphicsConfig, RoundRect};
pub use system::window::{Window, WindowAttributes, WindowError, WindowWaker};

pub mod input {
    pub use crate::system::input::*;
}

pub mod time {
    pub use crate::frame::{FramesPerSecond, SecondsPerFrame};
    pub use crate::system::time::*;
}

pub mod power {
    pub use crate::system::power::*;
}
