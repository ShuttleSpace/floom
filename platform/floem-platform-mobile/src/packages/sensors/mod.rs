//! Motion and environment sensors — availability queries.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `CMMotionManager` (iOS) and `SensorManager` (Android). Streaming readings
//! is a subscription concern; this package only exposes which sensors exist
//! and a single synchronous snapshot read where the OS provides one.
//!
//! `available_sensors` returns the full set of kinds the package knows how to
//! surface; `is_sensor_available` answers the same question for one kind.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// A physical sensor the package can surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorKind {
    /// Linear acceleration excluding gravity.
    Accelerometer,
    /// Rotation rate around three axes.
    Gyroscope,
    /// Ambient magnetic field.
    Magnetometer,
    /// Ambient air pressure.
    Barometer,
    /// Near-object proximity (e.g. face during a call).
    Proximity,
    /// Ambient light level.
    AmbientLight,
}

impl SensorKind {
    /// Every sensor kind this package knows how to query.
    pub fn all() -> &'static [SensorKind] {
        &[
            SensorKind::Accelerometer,
            SensorKind::Gyroscope,
            SensorKind::Magnetometer,
            SensorKind::Barometer,
            SensorKind::Proximity,
            SensorKind::AmbientLight,
        ]
    }
}

/// A single three-axis reading plus the time it was taken.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SensorReading {
    /// X-axis value (sensor-specific units).
    pub x: f64,
    /// Y-axis value.
    pub y: f64,
    /// Z-axis value.
    pub z: f64,
    /// Seconds since the Unix epoch when the sample was taken.
    pub timestamp: i64,
}

/// The set of sensors present on the device.
pub fn available_sensors() -> Vec<SensorKind> {
    #[cfg(target_os = "android")]
    {
        android::available_sensors()
    }
    #[cfg(target_os = "ios")]
    {
        ios::available_sensors()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Vec::new()
    }
}

/// Whether a specific sensor is present.
pub fn is_sensor_available(kind: SensorKind) -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_sensor_available(kind)
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_sensor_available(kind)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = kind;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reports_no_sensors() {
        assert!(available_sensors().is_empty());
        assert!(!is_sensor_available(SensorKind::Accelerometer));
    }

    #[test]
    fn all_enumerates_every_kind() {
        assert_eq!(SensorKind::all().len(), 6);
    }
}
