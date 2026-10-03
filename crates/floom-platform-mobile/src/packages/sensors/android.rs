//! Android sensors via `SensorManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{SensorKind, SensorReading};

pub fn available_sensors() -> Vec<SensorKind> {
    // TODO: `SensorManager.getSensorList(Sensor.TYPE_ALL)` and map the
    // `TYPE_*` constants to `SensorKind` via the JNI bridge.
    Vec::new()
}

pub fn is_sensor_available(kind: SensorKind) -> bool {
    // TODO: `getDefaultSensor(TYPE_*)` non-null check per kind.
    let _ = kind;
    false
}

/// Synchronous snapshot read (for sensors that expose a last-known value).
pub fn reading(_kind: SensorKind) -> Option<SensorReading> {
    // TODO: pull the most recent event from a registered listener.
    None
}
