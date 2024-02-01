// #![allow(clippy::cast_possible_truncation, clippy::module_name_repetitions)]

pub mod frame;
pub mod graphics;
pub mod io;
pub mod limits;
pub mod math;
pub mod time;

mod application;
mod platform;
mod window;

pub use application::Application;
pub use window::*;
