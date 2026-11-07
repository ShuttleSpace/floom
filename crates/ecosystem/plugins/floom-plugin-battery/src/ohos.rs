//! OpenHarmony battery state has no Rust-side read yet.

use super::BatteryState;

pub fn battery_state() -> BatteryState {
    BatteryState::default()
}

pub fn is_low_power_mode() -> bool {
    false
}
