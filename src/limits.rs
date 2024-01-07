use crate::{math::Size, Window};

/// Maximum number of windows that can be open at once.
pub const MAX_WINDOWS: usize = 32;

pub const MAX_TITLE_LENGTH: usize = 255;

pub const MAX_WINDOW_DIMENSIONS: Size<Window> = Size::new(u16::MAX as _, u16::MAX as _);

/// The maximum number of windows that can be open in the lifetime of the program.
pub const MAX_LIFETIME_WINDOWS: usize = u32::MAX as _;

#[cfg(all(target_os = "windows", not(target_has_atomic = "8")))]
const _: () = compile_error!("8-byte atomics are required for Windows");

#[cfg(all(target_os = "windows", not(target_has_atomic = "32")))]
const _: () = compile_error!("32-bit atomics are required for Windows");
