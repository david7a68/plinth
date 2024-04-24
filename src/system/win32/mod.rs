mod event_loop;
mod input;
pub mod time;
mod window;

pub use event_loop::{ActiveEventLoop, EventLoop, EventLoopError};
pub use window::{Waker, Window, WindowError};

mod api {
    pub use crate::system::event_loop::{ActiveEventLoop, EventLoopError};
    pub use crate::system::window::{Waker, Window, WindowAttributes, WindowError};
}
