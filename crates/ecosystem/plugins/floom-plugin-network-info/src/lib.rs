//! The current Wi-Fi network's name, and this device's address on it.
//!
//! This is `network_info_plus`'s territory, and the catalog's A7
//! (`docs/plugins.md`) — with one design correction the Flutter
//! package invites: it conflates **two questions with different privacy
//! profiles** under one "network info" roof.
//!
//! * [`local_ip`] — "what is *my* address". Not private in any meaningful
//!   sense (every peer you talk to learns it), answerable everywhere, and
//!   the same question on a phone and on a desktop.
//! * [`wifi_info`] — "what is the *network's* name". Genuinely private —
//!   SSIDs are location-identifying, and both phone platforms gate them
//!   behind permissions or entitlements for exactly that reason.
//!
//! Splitting them is why this crate has two functions instead of one
//! struct: a caller who only wants to print "connected from 192.168.1.4"
//! should not have to route through the capability that drags in the
//! location-permission story.
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # What `wifi_info` answers, per platform
//!
//! | Platform | `Ok(..)` with `None` fields | `Err` |
//! |---|---|---|
//! | Android | the SSID came back `<unknown ssid>` — location permission not granted (or location, or Wi-Fi, is off); the platform is *able* to talk Wi-Fi but not willing to name it | no `WifiManager` service, or the bridge is down |
//! | iOS / macOS | no Wi-Fi interface to ask about | `CNCopyCurrentNetworkInfo` returned nothing — on iOS 13+ that is the missing `com.apple.developer.network-information` entitlement as often as it is "not associated" (see the arm's docs: the API conflates the two) |
//! | Linux | — | NetworkManager over D-Bus is the only door, and this arm does not start a system-service conversation |
//! | Windows | — | `WlanQueryInterface` (wlanapi) is the door; deliberately not in v1 — the crate doc records it as a named follow-up rather than shipping an arm this build cannot run |
//!
//! # What this crate deliberately does not do
//!
//! No subnet mask, gateway, or broadcast address: `network_info_plus` has
//! them, but every platform answers them through a different mechanism
//! (ioctl in Java, routing sockets on Apple, MIB queries on Windows), and
//! v1 answers the two questions callers actually ask. No DNS, no
//! reachability — that is `floom-plugin-connectivity`'s question, and it is
//! already answered there.

use std::net::{IpAddr, UdpSocket};

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;

/// The identity of the current Wi-Fi network.
///
/// Each field is `None` when the platform did not name it; the crate doc's
/// table says what `None` means on each platform, because the reasons
/// differ and the caller deserves to know which wall they hit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WifiInfo {
    /// The network's SSID. On Android the platform wraps known SSIDs in
    /// literal double quotes; this crate strips them, so what arrives is
    /// the name as the user would read it.
    pub ssid: Option<String>,
    /// The access point's MAC address, `aa:bb:cc:dd:ee:ff`.
    pub bssid: Option<String>,
}

/// This device's address on the network its default route uses.
///
/// One implementation serves every platform, because `std` *is* the platform
/// abstraction here: a UDP socket is `connect`ed to an address nobody ever
/// sends to — connect on UDP consults the routing table and sends nothing —
/// and the socket's local address is the interface the route chose. On a
/// phone on Wi-Fi that is the Wi-Fi address; on a wired desktop it is the
/// wired one; the function's name says "local", not "wi-fi", because that
/// is the honest scope.
///
/// The probe targets are public DNS addresses chosen for their routing, not
/// their reachability: packets are never sent, so a filtered or unreachable
/// target behaves identically to a reachable one. IPv4 is tried first and
/// IPv6 on its failure, for machines whose default route is IPv6-only.
pub fn local_ip() -> Option<IpAddr> {
    for target in ["8.8.8.8:80", "[2001:4860:4860::8888]:80"] {
        let bind_to = if target.starts_with('[') {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        // A let-chain, in the order the steps must happen: bind, connect,
        // read. The first step that fails means this target says nothing,
        // and the next target gets its turn.
        if let Ok(socket) = UdpSocket::bind(bind_to)
            && socket.connect(target).is_ok()
            && let Ok(addr) = socket.local_addr()
        {
            return Some(addr.ip());
        }
    }
    None
}

/// The current Wi-Fi network's SSID and BSSID, where the platform is
/// willing to name them.
///
/// Read the crate doc's table before acting on `None` fields or on `Err`:
/// the interesting states ("not associated", "not permitted", "no API")
/// map differently per platform, and each arm documents its own mapping.
pub fn wifi_info() -> Result<WifiInfo, String> {
    #[cfg(target_os = "android")]
    {
        android::wifi_info()
    }
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        apple::wifi_info()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
    {
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        let reason = "wifi_info: Linux names Wi-Fi networks through NetworkManager \
                      over D-Bus, and this arm does not start a system-service conversation. \
                      A NetworkManager-backed arm would be the follow-up";
        #[cfg(target_env = "ohos")]
        let reason = "wifi_info: OpenHarmony exposes Wi-Fi state through ArkTS \
                      (@kit.ConnectivityKit), which this crate has not wired yet";
        #[cfg(not(target_os = "linux"))]
        let reason = "wifi_info: Windows names Wi-Fi networks through \
                      WlanQueryInterface (wlanapi) — a client handle, interface \
                      enumeration, and an attributes walk that this v1 deliberately \
                      leaves as a named follow-up rather than shipping an arm this \
                      build cannot run";
        Err(reason.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The host this test runs on fetched crates to build it, so it has a
    /// default route, so it has a local address on that route. This is the
    /// desktop-real doctrine again: `std` is the platform here, and the
    /// answer is a measurement the machine itself made.
    #[test]
    fn a_host_with_a_route_has_a_local_address() {
        if cfg!(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )) {
            let ip = local_ip();
            assert!(ip.is_some(), "no local address on a machine with a route");
            // A connected UDP socket's local address is a real interface
            // address: never the unspecified address a fabricated default
            // would land on.
            let ip = ip.unwrap();
            assert!(
                !(ip.is_unspecified()),
                "the local address must name an interface, not {ip}"
            );
        }
    }

    /// Whatever the Apple arm answers, it answers *precisely*: either named
    /// fields that are non-empty and quote-free, or an `Err` that names the
    /// API and therefore the way out. Which branch a given machine takes is
    /// itself the finding — see the arm's docs for what was observed where.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn wifi_info_names_fields_or_names_its_reason() {
        match wifi_info() {
            Ok(info) => {
                if let Some(ssid) = &info.ssid {
                    assert!(!ssid.is_empty(), "an SSID nobody measured");
                    assert!(!ssid.starts_with('"'), "unstripped quotes: {ssid:?}");
                }
                if let Some(bssid) = &info.bssid {
                    assert!(!bssid.is_empty(), "a BSSID nobody measured");
                }
            }
            Err(reason) => {
                assert!(
                    reason.contains("CNCopy"),
                    "the refusal must name the CaptiveNetwork API it could not get \
                     past -- `CNCopySupportedInterfaces` when the interface list is \
                     unreadable (a CI runner with no Wi-Fi), `CNCopyCurrentNetworkInfo` \
                     when the SSID itself is withheld: {reason}"
                );
            }
        }
    }

    /// The desktop refusals name their doors, so the next person finds them.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
    #[test]
    fn the_desktop_refusal_names_the_door() {
        let err = wifi_info().unwrap_err();
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        assert!(err.contains("NetworkManager"), "{err}");
        #[cfg(target_env = "ohos")]
        assert!(err.contains("ConnectivityKit"), "{err}");
        #[cfg(target_os = "windows")]
        assert!(err.contains("WlanQueryInterface"), "{err}");
    }
}
