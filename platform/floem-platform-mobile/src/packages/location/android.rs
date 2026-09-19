//! Android location via `FusedLocationProviderClient`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{Location, LocationAuthorization};

pub fn authorization_status() -> LocationAuthorization {
    // TODO: read `ContextCompat.checkSelfPermission` for
    // `ACCESS_FINE_LOCATION` / `ACCESS_COARSE_LOCATION` and map to the enum.
    LocationAuthorization::NotDetermined
}

pub fn is_location_enabled() -> bool {
    // TODO: on API 28+ use `LocationManager.isLocationEnabled()`; on older
    // APIs read the `LOCATION_PROVIDERS_ALLOWED` system setting.
    false
}

pub fn request_authorization() -> Result<(), String> {
    // TODO: launch the runtime-permission flow from the activity. This cannot
    // be satisfied synchronously; the real API returns via a callback.
    Err("location authorization requires an Android activity prompt".into())
}

pub fn current_location() -> Result<Location, String> {
    // TODO: `FusedLocationProviderClient.getLastLocation()` (returns via
    // Task<Location>); on first run fall back to `requestLocationUpdates`.
    Err("location is not available on the host".into())
}
