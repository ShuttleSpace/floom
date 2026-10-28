//! iOS connectivity via `SystemConfiguration`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Why reachability and not `NWPathMonitor`
//!
//! `NWPathMonitor` (Network.framework) is the modern API and is the right choice
//! for *observing* changes, but it reports through a callback on a dispatch
//! queue: there is no synchronous "what is the path right now" call. This crate
//! exposes snapshot functions only, so it uses `SCNetworkReachability`, whose
//! `SCNetworkReachabilityGetFlags` is exactly a synchronous query.
//!
//! # What reachability can and cannot say
//!
//! Reachability answers "can a packet plausibly leave this device toward this
//! host", not "is there internet". A Wi-Fi network behind a captive portal is
//! reachable. This matches what the crate documents: `is_connected` is the OS's
//! view of a usable path, not a guarantee the path reaches anything.
//!
//! The interface *kind* comes from the reachability flags: `WWAN` is present
//! only when the path goes over cellular, so its absence with a reachable path
//! means Wi-Fi or wired.

use super::{ConnectionType, ConnectivityStatus};
use std::net::Ipv4Addr;
use std::os::raw::{c_char, c_int, c_void};

/// `SCNetworkReachabilityFlags`.
type ReachabilityFlags = u32;

const FLAG_REACHABLE: ReachabilityFlags = 1 << 1;
const FLAG_CONNECTION_REQUIRED: ReachabilityFlags = 1 << 2;
const FLAG_CONNECTION_ON_TRAFFIC: ReachabilityFlags = 1 << 3;
const FLAG_IS_WWAN: ReachabilityFlags = 1 << 17;

#[link(name = "SystemConfiguration", kind = "framework")]
unsafe extern "C" {
    fn SCNetworkReachabilityCreateWithName(
        allocator: *const c_void,
        nodename: *const c_char,
    ) -> *mut c_void;
    fn SCNetworkReachabilityGetFlags(target: *mut c_void, flags: *mut ReachabilityFlags) -> c_int;
    fn CFRelease(cf: *const c_void);
}

pub fn connection_type() -> ConnectionType {
    match reachability() {
        Some(flags) if is_reachable(flags) => {
            if flags & FLAG_IS_WWAN != 0 {
                ConnectionType::Mobile
            } else {
                ConnectionType::Wifi
            }
        }
        _ => ConnectionType::None,
    }
}

pub fn connectivity_status() -> ConnectivityStatus {
    match reachability() {
        Some(flags) if is_reachable(flags) => {
            let is_wwan = flags & FLAG_IS_WWAN != 0;
            ConnectivityStatus {
                connection_type: if is_wwan {
                    ConnectionType::Mobile
                } else {
                    ConnectionType::Wifi
                },
                is_connected: true,
                // Cellular is metered by definition; Wi-Fi is not, and iOS
                // exposes no "this Wi-Fi is capped" signal to a third-party
                // app. This is the same rule the crate documents for the host
                // arm, applied to the one platform where the interface kind is
                // known.
                is_metered: is_wwan,
            }
        }
        _ => ConnectivityStatus::default(),
    }
}

pub fn is_connected() -> bool {
    reachability().is_some_and(is_reachable)
}

pub fn is_metered() -> bool {
    // A path is metered exactly when it is cellular. An unreachable device is
    // not metered — there is no connection to charge for.
    reachability().is_some_and(|flags| is_reachable(flags) && flags & FLAG_IS_WWAN != 0)
}

/// Whether the flags describe a usable path.
///
/// `kSCNetworkFlagsReachable` alone is not enough: a path that requires a
/// connection (`kSCNetworkFlagsConnectionRequired`) still has to be established
/// first, and the documented way to ask "would a connection attempt succeed"
/// is to also require `ConnectionOnTraffic` or `ConnectionOnDemand`. Only the
/// traffic case can be answered from the flags without starting the connection,
/// so a path that needs dialling reads as not connected — which is the honest
/// answer for a snapshot call.
fn is_reachable(flags: ReachabilityFlags) -> bool {
    if flags & FLAG_REACHABLE == 0 {
        return false;
    }
    if flags & FLAG_CONNECTION_REQUIRED == 0 {
        return true;
    }
    flags & FLAG_CONNECTION_ON_TRAFFIC != 0
}

/// Query reachability for a fixed, always-routable host.
///
/// `0.0.0.0` is the documented zero-address probe: it asks about the device's
/// own routing table rather than about a particular server, so it does not
/// depend on any third-party host being up, and it needs no DNS.
fn reachability() -> Option<ReachabilityFlags> {
    // The address is built in Rust and passed as a C string, so there is no
    // hard-coded byte array to get out of sync with its own length.
    let nodename = std::ffi::CString::new(Ipv4Addr::UNSPECIFIED.to_string()).ok()?;
    unsafe {
        let target = SCNetworkReachabilityCreateWithName(std::ptr::null(), nodename.as_ptr());
        if target.is_null() {
            return None;
        }
        let mut flags: ReachabilityFlags = 0;
        let ok = SCNetworkReachabilityGetFlags(target, &mut flags);
        // The target is a CoreFoundation object created here and owned here, so
        // it is released before returning either way.
        CFRelease(target);
        if ok == 0 { None } else { Some(flags) }
    }
}
