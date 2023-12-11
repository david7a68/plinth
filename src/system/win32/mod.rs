mod application;
mod event_thread;
mod window;
mod window_thread;

use std::time::Duration;

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

pub fn present_time_now() -> Duration {
    let mut time = 0;
    unsafe { QueryPerformanceCounter(&mut time) }.unwrap();

    let micros = (time * 1_000_000) / *QPF_FREQUENCY;
    Duration::from_micros(micros as u64)
}

pub fn present_time_from_ticks(ticks: u64) -> Duration {
    let micros = (ticks * 1_000_000) / *QPF_FREQUENCY as u64;
    Duration::from_micros(micros as u64)
}
