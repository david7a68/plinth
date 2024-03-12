#![allow(clippy::module_name_repetitions)]

pub mod geometry;
pub mod graphics;
pub mod limits;
pub mod resource;
mod string;
pub mod system;
pub mod time;

mod application;
mod arena;

pub use application::{AppContext, Application, Config, EventHandler};
pub use string::HashedStr;
