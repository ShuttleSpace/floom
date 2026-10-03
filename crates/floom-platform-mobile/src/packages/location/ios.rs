//! iOS location via `CLLocationManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{Location, LocationAuthorization};

pub fn authorization_status() -> LocationAuthorization {
    // TODO: read `CLLocationManager.authorizationStatus()` and map
    // `authorizedAlways` / `authorizedWhenInUse` / `denied` / `restricted`.
    LocationAuthorization::NotDetermined
}

pub fn is_location_enabled() -> bool {
    // TODO: `CLLocationManager.locationServicesEnabled()`.
    false
}

pub fn request_authorization() -> Result<(), String> {
    // TODO: `requestWhenInUseAuthorization()` / `requestAlwaysAuthorization()`
    // (async, via the delegate).
    Err("location authorization requires an iOS prompt".into())
}

pub fn current_location() -> Result<Location, String> {
    // TODO: `CLLocationManager.location` (last known) after authorization.
    Err("location is not available on the host".into())
}
