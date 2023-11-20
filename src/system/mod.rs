#[cfg(any(target_os = "windows", doc))]
mod win32;

#[cfg(any(target_os = "windows", doc))]
pub use win32::*;
