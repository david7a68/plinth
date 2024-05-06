pub(crate) mod event_loop;
mod input;
pub(crate) mod limits;
mod power;
pub(crate) mod time;
mod window;

pub use event_loop::InputEvent;
pub use input::*;
pub use power::*;
pub use window::*;

use crate::geometry::{new_extent, new_point, new_rect};

use self::limits::{SYS_WINDOW_COORD_MAX, SYS_WINDOW_COORD_MIN};

#[cfg(target_os = "windows")]
#[path = "win32/mod.rs"]
mod platform_impl;

new_point! {
    #[derive(Eq)]
    WindowPoint(x, y, i16, 0),
    { limit: SYS_WINDOW_COORD_MIN, SYS_WINDOW_COORD_MAX, "Window point out of limits" },
}

new_extent! {
    #[derive(Eq)]
    WindowExtent(i16, 0),
    { limit: SYS_WINDOW_COORD_MIN, SYS_WINDOW_COORD_MAX, "Window extent out of limits" },
}

new_rect! {
    WindowRect(i16, WindowPoint, WindowExtent),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DpiScale {
    pub factor: f32,
}

impl DpiScale {
    pub const IDENTITY: Self = Self { factor: 1.0 };

    pub fn new(factor: f32) -> Self {
        Self { factor }
    }
}
