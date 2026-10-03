//! iOS sensors via `CMMotionManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{SensorKind, SensorReading};

pub fn available_sensors() -> Vec<SensorKind> {
    // TODO: probe `CMMotionManager` `accelerometerAvailable`,
    // `gyroAvailable`, `magnetometerAvailable`, etc.
    Vec::new()
}

pub fn is_sensor_available(kind: SensorKind) -> bool {
    // TODO: map each `SensorKind` to the corresponding
    // `CMMotionManager.*Available` boolean.
    let _ = kind;
    false
}

/// Synchronous snapshot read of the most recent device-motion value.
pub fn reading(_kind: SensorKind) -> Option<SensorReading> {
    // TODO: `CMMotionManager.deviceMotion` last sample.
    None
}
