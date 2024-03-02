use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::system::time::NANOSECONDS_PER_SECOND;

lazy_static::lazy_static! {
    static ref QPF_FREQUENCY: i64 = {
        let mut freq = 0;
        unsafe { QueryPerformanceFrequency(&mut freq) }.unwrap();
        freq
    };
}

pub fn now_nanoseconds() -> i64 {
    let mut ticks = 0;
    unsafe { QueryPerformanceCounter(&mut ticks) }.unwrap();
    qpc_to_nanoseconds(ticks)
}

pub fn qpc_to_nanoseconds(ticks: i64) -> i64 {
    mul_div_i64(ticks, NANOSECONDS_PER_SECOND, *QPF_FREQUENCY)
}

/// Scale without overflow as long as the result and n * d do not overflow.
fn mul_div_i64(v: i64, n: i64, d: i64) -> i64 {
    let q = v / d;
    let r = v % d;

    q * n + r * n / d
}
