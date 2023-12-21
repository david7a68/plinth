use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub num_frames: i64,
    pub time: Duration,
}

impl Interval {
    pub fn as_frames_per_second(&self) -> FramesPerSecond {
        FramesPerSecond(self.num_frames as f64 / self.time.as_secs_f64())
    }

    pub fn as_seconds_per_frame(&self) -> SecondsPerFrame {
        SecondsPerFrame(self.time.as_secs_f64() / self.num_frames as f64)
    }
}

macro_rules! binops {
    ($unit:ty) => {
        impl std::ops::Add for $unit {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $unit {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::Div for $unit {
            type Output = f64;

            fn div(self, rhs: Self) -> Self::Output {
                self.0 as f64 / rhs.0 as f64
            }
        }

        impl std::ops::Div<f64> for $unit {
            type Output = Self;

            fn div(self, rhs: f64) -> Self::Output {
                Self(self.0 / rhs)
            }
        }

        impl std::ops::Mul for $unit {
            type Output = Self;

            fn mul(self, rhs: Self) -> Self::Output {
                Self(self.0 * rhs.0)
            }
        }

        impl std::ops::Mul<f64> for $unit {
            type Output = Self;

            fn mul(self, rhs: f64) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl std::cmp::PartialOrd for $unit {
            fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&rhs.0)
            }
        }

        impl std::cmp::PartialEq<f64> for $unit {
            fn eq(&self, rhs: &f64) -> bool {
                self.0 == *rhs
            }
        }

        impl std::cmp::PartialOrd<f64> for $unit {
            fn partial_cmp(&self, rhs: &f64) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(rhs)
            }
        }

        impl $unit {
            pub fn min(self, rhs: Self) -> Self {
                Self(self.0.min(rhs.0))
            }

            pub fn max(self, rhs: Self) -> Self {
                Self(self.0.max(rhs.0))
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramesPerSecond(pub f64);

binops!(FramesPerSecond);

impl FramesPerSecond {
    pub const ZERO: Self = Self(0.0);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SecondsPerFrame(pub f64);

binops!(SecondsPerFrame);

impl SecondsPerFrame {
    pub const ZERO: Self = Self(0.0);

    /// Returns the smallest number of frames that would encompass the duration.
    pub fn interval_over(self, time: Duration) -> Interval {
        let num_frames = (time.as_secs_f64() / self.0).ceil() as i64;
        let time = Duration::from_secs_f64(num_frames as f64 * self.0);
        Interval { num_frames, time }
    }
}

impl From<FramesPerSecond> for SecondsPerFrame {
    fn from(fps: FramesPerSecond) -> Self {
        Self(1.0 / fps.0)
    }
}
