//! Network reachability and connection type.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `SystemConfiguration` (iOS) and `ConnectivityManager` (Android); the
//! desktop arms measure through `std` (see `desktop.rs`). The Android/iOS arms
//! report validated internet; the desktop arms report a usable route, which is
//! the same question a phone answers through its platform API — see the desktop
//! module for how the two stay comparable.
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
// Desktop arms: macOS, Windows and Linux (Linux minus OHOS — see the OHOS
// contract in `docs/platforms.md`, where OHOS is `target_os = "linux"`) share one
// `std`-only implementation. Web (`wasm32`) has no `UdpSocket`, so it keeps the
// inert fallback below rather than reaching for a browser API.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
))]
mod desktop;

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
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::connection_type()
    }
    #[cfg(target_arch = "wasm32")]
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
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::connectivity_status()
    }
    #[cfg(target_arch = "wasm32")]
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
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::is_connected()
    }
    #[cfg(target_arch = "wasm32")]
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
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::is_metered()
    }
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_answer_is_self_consistent() {
        // Whatever the host's actual route state is, the crate's own story must
        // cohere: a reported connection carries a non-None type, and the status
        // mirrors the individual calls. A bare `cargo test` build runs on a host
        // with a route, but the invariant holds whether or not one exists.
        let connected = is_connected();
        let status = connectivity_status();
        assert_eq!(status.is_connected, connected);
        if connected {
            assert_ne!(connection_type(), ConnectionType::None);
            assert!(!status.is_metered);
        } else {
            assert_eq!(connection_type(), ConnectionType::None);
            assert_eq!(status, ConnectivityStatus::default());
        }
    }
}
