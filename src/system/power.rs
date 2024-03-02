/// Platform-global power preference.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPreference {
    /// Balance power consumption with user experience. Don't worry about saving
    /// power, but don't waste it either.
    Balanced,
    /// Reduce power consumption without compromising user experience.
    LowPower,
    /// Reduce power consumption as much as possible even if it means
    /// compromising user experience.
    VeryLowPower,
    /// Performance without compromise.
    MaxPerformance,
}

/// The computer's power source.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    /// The computer is plugged into an external power source.
    ///
    /// The platform has no visibility into the power source's capacity.
    External,
    /// The computer is running on battery power.
    Battery,
    /// The computer is running on an uninterruptible power supply (UPS).
    Backup,
}

/// The state of the computer's monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorState {
    /// One or more monitors are on.
    On,
    /// All monitors are off.
    Off,
    /// The monitors are dimmed due to inactivity.
    Dimmed,
}
