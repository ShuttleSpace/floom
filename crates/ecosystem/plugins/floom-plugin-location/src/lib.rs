//! Geolocation — authorization, availability and a one-shot position read.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `CLLocationManager` (iOS, macOS) and `LocationManager` (Android). On the two
//! Apple platforms `authorization_status` and `is_location_enabled` are real
//! synchronous reads; `current_location` has no synchronous route on either and
//! says so. See `src/ios.rs` / `src/macos.rs` for the per-platform reasons.
//!
//! Note: continuous location streaming is a separate concern (a subscription
//! with callbacks); this package only exposes the synchronous reads plus the
//! permission gates an app must check first. What "synchronous" costs is
//! documented per call: `current_location` on Android is the OS's last-known
//! cached fix, not a fresh one, and on iOS/macOS there is no synchronous route
//! at all.
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
#[cfg(target_os = "macos")]
mod macos;
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
    #[cfg(target_os = "macos")]
    {
        macos::authorization_status()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::authorization_status()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
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
    #[cfg(target_os = "macos")]
    {
        macos::is_location_enabled()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_location_enabled()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
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
    #[cfg(target_os = "macos")]
    {
        macos::request_authorization()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::request_authorization()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        Err("location is only available on iOS, macOS and Android".into())
    }
}

/// Best-effort one-shot position read.
///
/// Android returns the OS's **last-known** fix, which can be hours old — the
/// `timestamp` field is what lets a caller decide whether to trust it. iOS and
/// macOS have no synchronous route to a fix at all (see `src/ios.rs` /
/// `src/macos.rs`), so this returns `Err` there. Both arms return `Err` when not
/// authorized or when no fix is available.
pub fn current_location() -> Result<Location, String> {
    #[cfg(target_os = "android")]
    {
        android::current_location()
    }
    #[cfg(target_os = "ios")]
    {
        ios::current_location()
    }
    #[cfg(target_os = "macos")]
    {
        macos::current_location()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::current_location()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        Err("location is only available on iOS, macOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a desktop Mac the status and the system switch are real reads; on the
    /// remaining inert targets they are the documented defaults. Either way the
    /// value is one the type can express.
    #[test]
    fn the_status_read_is_well_formed() {
        let _ = authorization_status();
        let _ = is_location_enabled();
    }

    /// `current_location` refuses on Apple platforms (no synchronous fix) and on
    /// the inert targets — it never invents a position. `request_authorization`
    /// either reports "prompt shown" or a named refusal, so it is only checked
    /// for not panicking.
    #[test]
    fn the_position_read_is_honest() {
        assert!(current_location().is_err());
        let _ = request_authorization();
    }
}
