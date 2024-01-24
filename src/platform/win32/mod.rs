//! Win32 implementation.S
//!
//! Notes:
//!
//! - Event loop
//!   - Windows assigns one message loop to each thread.
//!   - Two windows on the same event loop cannot be repainted at the same time
//!     during Windows' snap-resize. One window gets replaced with a grey pane.
//!   - During a resize or move operation, DefWindowProc enters a modal loop
//!     from which there is no escape or interruption until the user releases
//!     the window chrome.
//!     - This is a problem for any running animations, since they won't get
//!       timers when that happens.
//!     - The solution is to use two threads per window, one that can preserve a
//!       frame clock, and one that pumps events.
//!   - The application terminates once all windows are closed.
//! - VSync
//!   - Compositor VSync is tied to the primary output, whatever that happens to
//!     be.
//!   - In windowed mode, the compositor adds one frame of latency as it
//!   performs composition only once per vblank.
//!   - It is faster and more reliable to call `WaitForVBlank` than to call
//!     `WaitForCompositorClock`, since the latter varies by a little bit most
//!     of the time but will sometimes cause skipped frames for applications
//!     with light workloads.
//!   - Waiting for Vblank on every UI thread increases power usage if multiple
//!     windows are running at different rates and distributes timing code
//!     accross threads. Prefer a dedicated VSync thread.
//! - Rendering
//!   - The goal is to draw the UI in one shot across each device.
//!   - Coordinating rendering between threads to minimize queue submissions is
//!     more effort than it's worth and would introduce an additional frame of
//!     latency.
//!     - Assuming a single submit per frame, you would need an obnoxious number
//!       of winodws to impair performance (supposition).
//!
//! Design:
//!
//! - Main Thread: operates the vsync clock (instead of being left idle). UI
//!   threads send requests to be notified at particular vblanks (or 0 for the
//!   next vblank). UI threads are also notified when the main display device
//!   changes.
//! - Window Threads:
//!   - Event Loop: Calls `GetMessage` in a loop, using a channel to send events
//!     to the UI thread.
//!   - UI/Render: Receives events from the Event Loop and the VSync clock.
//!     Performs rendering on VSync events and submits work to the graphics
//!     queue directly.

mod application;
mod vsync;
mod window;

pub use application::{AppContextImpl, ApplicationImpl};
pub(crate) use vsync::VSyncRequest;
pub(crate) use window::Win32WindowEventInterposer;
pub use window::WindowImpl;

use lazy_static::lazy_static;
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

lazy_static! {
    static ref QPF_FREQUENCY: i64 = {
        let mut freq = 0;
        unsafe { QueryPerformanceFrequency(&mut freq) }.unwrap();
        freq
    };
}

pub fn present_time_now() -> f64 {
    let mut time = 0;
    unsafe { QueryPerformanceCounter(&mut time) }.unwrap();
    time as f64 / *QPF_FREQUENCY as f64
}

pub fn present_time_from_ticks(ticks: u64) -> f64 {
    ticks as f64 / *QPF_FREQUENCY as f64
}
