mod application;
mod vsync;
mod window;

pub use application::{AppContextImpl, ApplicationImpl};
pub(crate) use vsync::VSyncRequest;
pub(crate) use window::Interposer;
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
    (time / *QPF_FREQUENCY) as f64
}

pub fn present_time_from_ticks(ticks: i64) -> f64 {
    (ticks / *QPF_FREQUENCY) as f64
}
