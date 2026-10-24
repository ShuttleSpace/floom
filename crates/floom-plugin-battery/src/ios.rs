//! iOS battery state via `UIDevice` and `NSProcessInfo`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Why monitoring has to be switched on
//!
//! `UIDevice.batteryLevel` returns `-1.0` until `batteryMonitoringEnabled` is
//! `true`; the flag costs nothing to set and the system does not otherwise
//! maintain the value. It is set on every call rather than once behind a
//! `OnceLock` because `UIDevice` is a singleton and the setter is idempotent —
//! a "did we already enable it" flag would be a second source of truth for
//! state the OS already holds.
//!
//! Enabling monitoring also has a documented side effect: it turns on the
//! battery-level notifications the system would otherwise skip. That is
//! acceptable for a plugin whose whole purpose is reading the battery, and it
//! is why the crate's callers opt in by depending on it.

use super::BatteryState;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// `UIDeviceBatteryState` values, from `UIDevice.h`.
///
/// The third value, `Unknown` (`0`), is not named here because nothing matches
/// on it: a reading that has not arrived yet also has `batteryLevel == -1.0`,
/// which is the earlier and stronger check.
const BATTERY_STATE_CHARGING: i64 = 2;
const BATTERY_STATE_FULL: i64 = 3;

pub fn battery_state() -> BatteryState {
    unsafe {
        let device: *mut AnyObject = msg_send![class!(UIDevice), currentDevice];
        if device.is_null() {
            return BatteryState::default();
        }

        // `batteryMonitoringEnabled` is a `BOOL` property, so it is written with
        // `setBatteryMonitoringEnabled:` rather than assigned through a field.
        let _: () = msg_send![device, setBatteryMonitoringEnabled: true];

        let level: f32 = msg_send![device, batteryLevel];
        let state: i64 = msg_send![device, batteryState];

        // `-1.0` is UIKit's "no reading", not a charge level; reporting it as a
        // fraction would put the UI below empty and is the one value that must
        // not be passed through.
        if level < 0.0 {
            return BatteryState::default();
        }

        BatteryState {
            level: level.clamp(0.0, 1.0),
            is_charging: state == BATTERY_STATE_CHARGING,
            // `Full` is UIKit's "plugged in and topped up"; `Unknown` is not
            // full, and neither is `Unplugged`.
            is_full: state == BATTERY_STATE_FULL,
        }
    }
}

pub fn is_low_power_mode() -> bool {
    unsafe {
        let info: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
        if info.is_null() {
            return false;
        }
        let enabled: bool = msg_send![info, isLowPowerModeEnabled];
        enabled
    }
}
