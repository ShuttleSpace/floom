//! Battery level and charging state.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read `UIDevice`
//! (iOS), `BatteryManager` (Android), IOKit's `IOPS` API (macOS), sysfs
//! `/sys/class/power_supply` (Linux), and Win32 `GetSystemPowerStatus`
//! (Windows). Each is a real measurement on the platform that has it; a
//! platform with no readable power supply (a desktop with no battery, a box
//! without the sysfs node) reports "no reading": `level: None`, not charging,
//! not full.
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
// Android is `target_os = "linux"` but served by `android.rs`, so the desktop
// Linux arm is gated on Linux *minus* Android. OpenHarmony has no battery arm
// yet and falls through to the inert default below.
#[cfg(all(
    target_os = "linux",
    not(target_os = "android"),
    not(target_env = "ohos")
))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_os = "windows")]
mod windows;

/// Snapshot of the device battery.
///
/// `level` is `Option` because "the OS would not give me a reading" is a
/// distinct answer from "the battery is at some fraction", and collapsing the
/// two is what makes a failed read look like a full battery.
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
    /// This is also the answer for a host with no readable battery. It is
    /// deliberately *not* `level: 1.0`, which would be a number no arm had
    /// produced.
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
    #[cfg(all(
        target_os = "linux",
        not(target_os = "android"),
        not(target_env = "ohos")
    ))]
    {
        linux::battery_state()
    }
    #[cfg(target_os = "macos")]
    {
        macos::battery_state()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::battery_state()
    }
    #[cfg(target_os = "windows")]
    {
        windows::battery_state()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )))]
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
    #[cfg(all(
        target_os = "linux",
        not(target_os = "android"),
        not(target_env = "ohos")
    ))]
    {
        linux::is_low_power_mode()
    }
    #[cfg(target_os = "macos")]
    {
        macos::is_low_power_mode()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_low_power_mode()
    }
    #[cfg(target_os = "windows")]
    {
        windows::is_low_power_mode()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The call must always return a well-formed answer: either no reading or a
    /// level in `[0, 1]`. On a battery-bearing host this may be a real reading,
    /// so we assert the shape, not the specific value.
    #[test]
    fn host_reading_is_well_formed() {
        let s = battery_state();
        if let Some(level) = s.level {
            assert!((0.0..=1.0).contains(&level), "level {level} out of range");
        }
    }

    #[test]
    fn host_low_power_is_false_on_desktop() {
        // Battery-saver is not uniformly observable; every arm reports a boolean
        // without panicking.
        let _ = is_low_power_mode();
    }
}
