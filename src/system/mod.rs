pub(crate) mod event_loop;
pub mod input;
pub mod power;
pub(crate) mod time;
pub mod window;

#[cfg(target_os = "windows")]
#[path = "win32/mod.rs"]
mod platform_impl;
