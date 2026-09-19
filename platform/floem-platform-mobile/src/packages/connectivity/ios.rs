//! iOS connectivity via `SystemConfiguration`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::{ConnectionType, ConnectivityStatus};

pub fn connection_type() -> ConnectionType {
    // TODO: use `SCNetworkReachability` to determine Wi-Fi vs cellular vs none.
    ConnectionType::None
}

pub fn connectivity_status() -> ConnectivityStatus {
    // TODO: derive metered flag from the active interface.
    ConnectivityStatus::default()
}

pub fn is_connected() -> bool {
    // TODO: `SCNetworkReachability` with the connection-required flag cleared.
    false
}

pub fn is_metered() -> bool {
    // iOS does not expose a metered concept; Wi-Fi is unmetered, cellular is.
    false
}
