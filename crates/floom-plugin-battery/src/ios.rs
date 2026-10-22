//! iOS battery state via `UIDevice`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::BatteryState;

pub fn battery_state() -> BatteryState {
    // TODO: set `UIDevice.currentDevice.batteryMonitoringEnabled = true` and
    // read `batteryLevel` (0..1) and `batteryState` (.charging / .full).
    BatteryState::default()
}

pub fn is_low_power_mode() -> bool {
    // TODO: read `ProcessInfo.processInfo.isLowPowerModeEnabled`
    // (NSProcessInfo, iOS 9+).
    false
}
