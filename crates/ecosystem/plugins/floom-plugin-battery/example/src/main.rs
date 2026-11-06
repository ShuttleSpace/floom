// SPDX-License-Identifier: MIT
//! Runs the `battery` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_battery::{battery_state, is_low_power_mode};

fn main() {
    let state = battery_state();
    println!(
        "level={:?} charging={} full={}",
        state.level, state.is_charging, state.is_full
    );
    println!("low power mode={}", is_low_power_mode());
}
