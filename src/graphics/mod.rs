mod backend;

pub use self::backend::*;

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct Config {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::HighPerformance,
            debug_mode: cfg!(debug_assertions),
        }
    }
}
