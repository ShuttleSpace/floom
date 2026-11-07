//! OpenHarmony sensors are not wired to this crate yet.

use super::SensorKind;

pub fn available_sensors() -> Vec<SensorKind> {
    Vec::new()
}

pub fn is_sensor_available(_kind: SensorKind) -> bool {
    false
}
