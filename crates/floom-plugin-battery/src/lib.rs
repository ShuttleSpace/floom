//! Battery level and charging state.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read `UIDevice`
//! (iOS) and `BatteryManager` (Android). The host arm reports a full,
//! non-charging cell so callers gating UI on "low battery" don't misbehave.
//!
//! A **capability plugin**: one capability, one crate, opt-in by
//! dependency. The SDK does not carry it — an app that wants this
//! capability adds this crate, and an app that does not, does not link
//! it.
//!
//! Where an arm is not wired yet, the call reports that plainly rather
//! than returning a value that looks measured.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// Snapshot of the device battery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryState {
    /// Charge level in the range `0.0..=1.0`.
    pub level: f32,
    /// Whether the device is currently drawing power.
    pub is_charging: bool,
    /// Whether the battery is at (or treated as) full.
    pub is_full: bool,
}

impl Default for BatteryState {
    fn default() -> Self {
        BatteryState {
            level: 1.0,
            is_charging: false,
            is_full: false,
        }
    }
}

/// Snapshot the current battery state.
pub fn battery_state() -> BatteryState {
    #[cfg(target_os = "android")]
    {
        android::battery_state()
    }
    #[cfg(target_os = "ios")]
    {
        ios::battery_state()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        BatteryState::default()
    }
}

/// Whether the OS is in low-power / battery-saver mode.
pub fn is_low_power_mode() -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_low_power_mode()
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_low_power_mode()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reports_full_non_charging() {
        let s = battery_state();
        assert_eq!(s.level, 1.0);
        assert!(!s.is_charging);
    }

    #[test]
    fn host_low_power_is_false() {
        assert!(!is_low_power_mode());
    }
}
