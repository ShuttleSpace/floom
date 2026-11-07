//! Network reachability and connection type.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `SystemConfiguration` (iOS) and `ConnectivityManager` (Android). The host
//! arm reports "no connection".
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

/// The kind of network the device is using, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionType {
    /// No active network path.
    #[default]
    None,
    /// Wi-Fi.
    Wifi,
    /// Cellular (mobile data).
    Mobile,
    /// Wired Ethernet.
    Ethernet,
    /// Bluetooth tethering / PAN.
    Bluetooth,
    /// Any other path.
    Other,
}

/// A richer view of connectivity than a single enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConnectivityStatus {
    /// The active connection kind.
    pub connection_type: ConnectionType,
    /// Whether any network path is currently usable.
    pub is_connected: bool,
    /// Whether the active connection is metered (e.g. cellular, capped).
    pub is_metered: bool,
}

/// The active connection kind.
pub fn connection_type() -> ConnectionType {
    #[cfg(target_os = "android")]
    {
        android::connection_type()
    }
    #[cfg(target_os = "ios")]
    {
        ios::connection_type()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::connection_type()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        ConnectionType::None
    }
}

/// The full connectivity status.
pub fn connectivity_status() -> ConnectivityStatus {
    #[cfg(target_os = "android")]
    {
        android::connectivity_status()
    }
    #[cfg(target_os = "ios")]
    {
        ios::connectivity_status()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::connectivity_status()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        ConnectivityStatus::default()
    }
}

/// Whether any network path is currently usable.
pub fn is_connected() -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_connected()
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_connected()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_connected()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        false
    }
}

/// Whether the active connection is metered (e.g. cellular, capped).
pub fn is_metered() -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_metered()
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_metered()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_metered()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reports_no_connection() {
        assert_eq!(connection_type(), ConnectionType::None);
        assert!(!is_connected());
        assert!(!is_metered());
        assert_eq!(connectivity_status(), ConnectivityStatus::default());
    }
}
