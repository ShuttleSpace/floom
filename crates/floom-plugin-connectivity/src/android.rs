//! Android connectivity via `ConnectivityManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{ConnectionType, ConnectivityStatus};

pub fn connection_type() -> ConnectionType {
    // TODO: query `ConnectivityManager.getActiveNetworkInfo()` /
    // `NetworkCapabilities` (API 29+) and map `TYPE_WIFI` / `TYPE_MOBILE` /
    // `TRANSPORT_*` to `ConnectionType` via the JNI bridge.
    ConnectionType::None
}

pub fn connectivity_status() -> ConnectivityStatus {
    // TODO: combine `connection_type()` with `NetworkCapabilities` metered /
    // internet flags.
    ConnectivityStatus::default()
}

pub fn is_connected() -> bool {
    // TODO: check `ConnectivityManager.getActiveNetwork()` is non-null and has
    // `NET_CAPABILITY_INTERNET`.
    false
}

pub fn is_metered() -> bool {
    // TODO: read `NetworkCapabilities.NET_CAPABILITY_NOT_METERED`.
    false
}
