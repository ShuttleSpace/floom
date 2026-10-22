//! Android battery state via `BatteryManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::BatteryState;

pub fn battery_state() -> BatteryState {
    // TODO: register a `BatteryManager` receiver and read
    // `EXTRA_LEVEL` / `EXTRA_SCALE` / `EXTRA_PLUGGED` / `EXTRA_STATUS` from the
    // sticky `ACTION_BATTERY_CHANGED` intent via the JNI bridge.
    BatteryState::default()
}

pub fn is_low_power_mode() -> bool {
    // TODO: read `Settings.Global` `BATTERY_SAVER_MODE` (API 21+).
    false
}
