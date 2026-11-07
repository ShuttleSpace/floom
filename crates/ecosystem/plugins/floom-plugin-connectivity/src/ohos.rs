//! OpenHarmony connectivity is not wired to this crate yet.

use super::{ConnectionType, ConnectivityStatus};

pub fn connection_type() -> ConnectionType {
    ConnectionType::None
}

pub fn connectivity_status() -> ConnectivityStatus {
    ConnectivityStatus::default()
}

pub fn is_connected() -> bool {
    false
}

pub fn is_metered() -> bool {
    false
}
