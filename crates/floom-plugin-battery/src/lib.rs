//! Battery level and charging state.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read `UIDevice`
//! (iOS) and `BatteryManager` (Android). The host arm reports "no reading":
//! `level: None`, not charging, not full.
//!
//! A **capability plugin**: one capability, one crate, opt-in by
//! dependency. The SDK does not carry it — an app that wants this
//! capability adds this crate, and an app that does not, does not link
//! it.
//!
//! Where an arm cannot measure, the call reports that plainly rather
//! than returning a value that looks measured. That is why
//! [`BatteryState::level`] is an `Option`: a read that failed and a battery at
//! some fraction are different answers, and a caller gating UI on "low battery"
//! needs to be able to tell them apart.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// Snapshot of the device battery.
///
/// `level` is `Option` because "the OS would not give me a reading" is a
/// distinct answer from "the battery is at some fraction", and collapsing the
/// two is what makes a failed read look like a full battery. On iOS the
/// platform says so itself: `UIDevice.batteryLevel` returns `-1.0` when
/// monitoring is not enabled yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryState {
    /// Charge level in the range `0.0..=1.0`, or `None` when the platform gave
    /// no reading.
    pub level: Option<f32>,
    /// Whether the device is currently drawing power.
    pub is_charging: bool,
    /// Whether the battery is at (or treated as) full.
    pub is_full: bool,
}

impl Default for BatteryState {
    /// Nothing measured: no level, not charging, not full.
    ///
    /// This is also the host answer. It is deliberately *not* `level: 1.0`,
    /// which would be a number no arm had produced.
    fn default() -> Self {
        BatteryState {
            level: None,
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
    fn host_reports_no_reading() {
        let s = battery_state();
        // A host has no battery. The old default claimed `level: 1.0`, which is
        // a number no arm produced and which a caller cannot tell apart from a
        // genuinely full phone.
        assert_eq!(s.level, None);
        assert!(!s.is_charging);
        assert!(!s.is_full);
    }

    #[test]
    fn host_low_power_is_false() {
        assert!(!is_low_power_mode());
    }
}
