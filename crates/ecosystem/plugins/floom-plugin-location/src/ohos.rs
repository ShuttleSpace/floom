//! OpenHarmony location is not wired to this crate yet.

use super::{Location, LocationAuthorization};

pub fn authorization_status() -> LocationAuthorization {
    LocationAuthorization::NotDetermined
}

pub fn is_location_enabled() -> bool {
    false
}

pub fn request_authorization() -> Result<(), String> {
    Err("location: OpenHarmony location (@kit.LocationKit) is not wired to this crate yet".into())
}

pub fn current_location() -> Result<Location, String> {
    Err("location: OpenHarmony location (@kit.LocationKit) is not wired to this crate yet".into())
}
