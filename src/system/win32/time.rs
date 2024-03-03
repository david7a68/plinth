use std::sync::OnceLock;

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::system::time::NANOSECONDS_PER_SECOND;

fn query_freq() -> i64 {
    static FREQUENCY: OnceLock<i64> = OnceLock::new();
    *FREQUENCY.get_or_init(|| {
        let mut freq = 0;
        unsafe { QueryPerformanceFrequency(&mut freq) }.unwrap();
        freq
    })
}

pub fn now_nanoseconds() -> i64 {
    let mut ticks = 0;
    unsafe { QueryPerformanceCounter(&mut ticks) }.unwrap();
    qpc_to_nanoseconds(ticks)
}

pub fn qpc_to_nanoseconds(ticks: i64) -> i64 {
    mul_div_i64(ticks, NANOSECONDS_PER_SECOND, query_freq())
}

/// Scale without overflow as long as the result and n * d do not overflow.
fn mul_div_i64(value: i64, numerator: i64, denominator: i64) -> i64 {
    let q = value / denominator;
    let r = value % denominator;

    q * numerator + r * numerator / denominator
}
