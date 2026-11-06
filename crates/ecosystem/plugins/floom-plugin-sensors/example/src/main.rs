// SPDX-License-Identifier: MIT
//! Runs the `sensors` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_sensors::{SensorKind, available_sensors, is_sensor_available};

fn main() {
    println!("all: {:?}", SensorKind::all());
    println!("available: {:?}", available_sensors());
    for kind in SensorKind::all() {
        println!("{kind:?}: {}", is_sensor_available(*kind));
    }
}
