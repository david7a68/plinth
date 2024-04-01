//! Static limits and constraints.

use crate::core::limits::{StrLen, Usize, WixelExtent, MAX};

/// Maximum number of windows that can be open at once.
pub const SYS_MAX_WINDOWS: Usize<8, MAX> = Usize::new("Too many windows open");

/// The maximum number of UTF-8 bytes that can be used to represent a window title.
pub const SYS_TITLE_LENGTH: StrLen<256> = StrLen::new("Window title too long");

/// The maximum and minimum size of a window (inclusive).
pub const SYS_WINDOW_EXTENT: WixelExtent<100, 100, { i16::MAX }, { i16::MAX }> =
    WixelExtent::new("Window extent out of range.");

/// The maximum number of UTF-8 bytes that can be used to represent a path to a
/// resource.
pub const RES_PATH_LENGTH: StrLen<1024> = StrLen::new("Resource path too long");

pub use crate::graphics::limits::*;
