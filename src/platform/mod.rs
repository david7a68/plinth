#[cfg(any(target_os = "windows", doc))]
mod win32;

#[cfg(any(target_os = "windows", doc))]
mod dx12;

#[cfg(any(target_os = "windows", doc))]
pub use win32::*;

pub(crate) mod gfx;
