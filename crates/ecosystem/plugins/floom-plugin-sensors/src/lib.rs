//! Motion and environment sensors — availability queries.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `CMMotionManager` (iOS) and `SensorManager` (Android).
//!
//! This package answers **which sensors exist**, and nothing else. Reading a
//! sensor is a subscription concern: Android delivers samples through a
//! `SensorEventListener` and iOS through `startAccelerometerUpdatesToQueue:`,
//! both of which are callbacks this synchronous API has nowhere to put. There is
//! deliberately no `reading()` function rather than one that returns `None` —
//! a `None` from a wired arm and a `None` from an unwired one look identical to
//! the caller, which is the confusion this plugin family exists to remove.
//!
//! `available_sensors` returns the full set of kinds the package knows how to
//! query, filtered to those the device actually has; `is_sensor_available`
//! answers the same question for one kind.
//!
//! A **capability plugin**: one capability, one crate, opt-in by
//! dependency. The SDK does not carry it — an app that wants this
//! capability adds this crate, and an app that does not, does not link
//! it.
//!
//! Where an arm is not wired yet, the call reports that plainly rather
//! than returning a value that looks measured.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_env = "ohos")]
mod ohos;

/// A physical sensor the package can surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorKind {
    /// Acceleration in m/s², including gravity.
    ///
    /// Android `TYPE_ACCELEROMETER`, iOS `CMMotionManager.accelerometerAvailable`
    /// — the raw accelerometer on both, so the reading includes gravity. The
    /// derived "linear acceleration" sensor (Android `TYPE_LINEAR_ACCELERATION`,
    /// iOS `deviceMotion`) subtracts it, but it is a composite that only exists
    /// when the device also has a gyroscope, so it is a different question from
    /// "does this device have an accelerometer".
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
    #[cfg(target_env = "ohos")]
    {
        ohos::available_sensors()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
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
    #[cfg(target_env = "ohos")]
    {
        ohos::is_sensor_available(kind)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
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
