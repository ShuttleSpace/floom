//! Desktop connectivity, measured through `std` alone.
//!
//! [`super::is_connected`] answers "does the machine have a usable network
//! path" by `connect`ing a UDP socket to an address nobody ever sends to:
//! connect on UDP consults the routing table and transmits nothing, so what
//! comes back is what the kernel itself resolved — a measurement the machine
//! made, not a poll of some API. On a phone the same question is answered by
//! the platform (`ConnectivityManager`, `SCNetworkReachability`); on a desktop
//! `std` *is* the platform.
//!
//! [`super::connection_type`] separates Wi-Fi from wired only where the
//! kernel names it (Linux sysfs); where no std source distinguishes them
//! (macOS, Windows) it reports `Other` — a connected-but-unclassified path —
//! rather than guessing an interface kind no source named.
//!
//! The answer is "a route exists", not "the internet is reachable": a captive
//! portal also has this crate's is_connected true, just as `ConnectivityManager`
//! marks validated internet only after its own probe. Sharing that
//! nuance in the docs keeps the desktop and phone answers comparable
//! rather than quietly different.

use std::net::UdpSocket;

use super::ConnectionType;

/// Route-probe targets: public DNS addresses chosen for their routing, not
/// their reachability (packets are never sent, so a filtered target behaves
/// identically to a reachable one). IPv4 first, IPv6 for machines whose
/// default route is IPv6-only.
const PROBE_TARGETS: [&str; 2] = ["8.8.8.8:80", "[2001:4860:4860::8888]:80"];

/// Whether the kernel has a usable default route. See the module docs for
/// what that does and does not mean.
pub fn is_connected() -> bool {
    for target in PROBE_TARGETS {
        let bind_to = if target.starts_with('[') {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        // bind, connect, read — the first step that fails says this target
        // resolved to nothing, and the next gets its turn.
        if UdpSocket::bind(bind_to)
            .and_then(|socket| {
                socket
                    .connect(target)
                    .map(|_| socket)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            })
            .and_then(|socket| socket.local_addr())
            .is_ok()
        {
            return true;
        }
    }
    false
}

pub fn is_metered() -> bool {
    // Desktop connections are treated as unmetered. Windows carries a
    // "metered" flag per connection and Linux names its cost class in
    // NetworkManager, but neither has a `std` door; reporting `false` here is
    // the desktop default posture, and a desktop caller downloading
    // gigabytes is trusted to be on a line that can take it.
    false
}

/// The connection kind, classified where the OS names it.
pub fn connection_type() -> ConnectionType {
    if !is_connected() {
        return ConnectionType::None;
    }
    classify()
}

/// Classify the default route's interface. Only Linux names the interface
/// kind through `std`-readable files; macOS and Windows need their own APIs
/// (route sockets, `WlanQueryInterface`), so they answer with the honest
/// "connected, yet unclassified" value instead of an interface no source named.
fn classify() -> ConnectionType {
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    {
        let iface = match default_linux_interface() {
            Some(name) => name,
            None => return ConnectionType::Other,
        };
        // wlan* interfaces carry a `wireless/` directory; wired ones do not.
        let is_wifi = std::path::Path::new("/sys/class/net")
            .join(&iface)
            .join("wireless")
            .is_dir();
        if is_wifi {
            ConnectionType::Wifi
        } else {
            ConnectionType::Ethernet
        }
    }
    #[cfg(not(all(target_os = "linux", not(target_env = "ohos"))))]
    {
        ConnectionType::Other
    }
}

/// The interface the default route uses, read from `/proc/net/route`: the
/// kube rows whose flags carry `RTF_GATEWAY` (`0x2`). A desktop whose routes
/// come from somewhere else (an unusual container) hangs up with `Other`.
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
fn default_linux_interface() -> Option<String> {
    let content = std::fs::read_to_string("/proc/net/route").ok()?;
    let mut lines = content.lines();
    // The header row is `Iface Destination Gateway Flags ...` and names no
    // interface, so it is skipped, not parsed.
    lines.next();
    for line in lines {
        let mut fields = line.split('\t');
        let iface = fields.next()?;
        let _destination = fields.next()?;
        let _gateway = fields.next()?;
        let flags = fields.next()?;
        if let Ok(flags) = u16::from_str_radix(flags, 16) {
            if flags & 0x2 != 0 {
                return Some(iface.to_string());
            }
        }
    }
    None
}

/// The full status on a desktop: the three primitives composed.
pub fn connectivity_status() -> super::ConnectivityStatus {
    let is_connected = is_connected();
    super::ConnectivityStatus {
        connection_type: connection_type(),
        is_connected,
        is_metered: is_metered(),
    }
}
