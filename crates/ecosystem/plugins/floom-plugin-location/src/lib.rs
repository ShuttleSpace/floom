//! Geolocation — authorization, availability and a one-shot position read.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `CLLocationManager` (iOS) and `LocationManager` (Android). The host arm
//! refuses every call that needs a real position.
//!
//! Note: continuous location streaming is a separate concern (a subscription
//! with callbacks); this package only exposes the synchronous reads plus the
//! permission gates an app must check first. What "synchronous" costs is
//! documented per call: `current_location` on Android is the OS's last-known
//! cached fix, not a fresh one, and on iOS there is no synchronous route at all.
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

/// The app's location authorization status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocationAuthorization {
    /// The user has not been asked yet.
    #[default]
    NotDetermined,
    /// The app is restricted (e.g. parental controls).
    Restricted,
    /// The user denied access.
    Denied,
    /// Authorized only while the app is in use.
    AuthorizedWhenInUse,
    /// Authorized at all times.
    AuthorizedAlways,
}

/// A single geographic position.
#[derive(Debug, Clone, Default)]
pub struct Location {
    /// Latitude in degrees.
    pub latitude: f64,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Altitude in meters (`0.0` if unavailable).
    pub altitude: f64,
    /// Horizontal accuracy in meters (negative = invalid).
    pub horizontal_accuracy: f64,
    /// Vertical accuracy in meters (negative = invalid).
    pub vertical_accuracy: f64,
    /// Seconds since the Unix epoch when the fix was taken.
    pub timestamp: i64,
}

/// Current location authorization status.
pub fn authorization_status() -> LocationAuthorization {
    #[cfg(target_os = "android")]
    {
        android::authorization_status()
    }
    #[cfg(target_os = "ios")]
    {
        ios::authorization_status()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::authorization_status()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        LocationAuthorization::NotDetermined
    }
}

/// Whether the device has any location provider enabled at the OS level.
pub fn is_location_enabled() -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_location_enabled()
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_location_enabled()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_location_enabled()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        false
    }
}

/// Request location authorization. Returns `Err` if already denied or the
/// platform cannot prompt.
pub fn request_authorization() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::request_authorization()
    }
    #[cfg(target_os = "ios")]
    {
        ios::request_authorization()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::request_authorization()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        Err("location is only available on iOS and Android".into())
    }
}

/// Best-effort one-shot position read.
///
/// Android returns the OS's **last-known** fix, which can be hours old — the
/// `timestamp` field is what lets a caller decide whether to trust it. iOS has
/// no synchronous route to a fix at all (see `src/ios.rs`), so this returns
/// `Err` there. Both arms return `Err` when not authorized or when no fix is
/// available.
pub fn current_location() -> Result<Location, String> {
    #[cfg(target_os = "android")]
    {
        android::current_location()
    }
    #[cfg(target_os = "ios")]
    {
        ios::current_location()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::current_location()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        Err("location is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reports_not_determined_and_disabled() {
        assert_eq!(authorization_status(), LocationAuthorization::NotDetermined);
        assert!(!is_location_enabled());
    }

    #[test]
    fn host_requests_refused() {
        assert!(request_authorization().is_err());
        assert!(current_location().is_err());
    }
}
