#![allow(clippy::module_name_repetitions)]

pub mod geometry;
pub mod graphics;
pub mod limits;
mod static_str;
pub mod system;
pub mod time;

mod application;

pub use application::{AppContext, Application, Config, EventHandler};
pub use static_str::StaticStr;
