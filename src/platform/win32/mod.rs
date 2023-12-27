mod application;
mod event_thread;
mod swapchain;
mod window;
mod window_thread;

pub use application::{AppContextImpl, ApplicationImpl};
use lazy_static::lazy_static;
pub use window::WindowImpl;
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
