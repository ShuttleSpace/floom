//! Android connectivity via `ConnectivityPlugin`.
//!
//! The capability is Java: `ConnectivityManager` and `NetworkCapabilities` are
//! Android APIs, and `ConnectivityPlugin` owns them. This file only carries the
//! ask across JNI.
//!
//! # Why `NetworkCapabilities` and not `getActiveNetworkInfo`
//!
//! `getActiveNetworkInfo()` is deprecated (API 29) and reports the *default*
//! network, which on a device with both Wi-Fi and cellular is whichever one the
//! system chose to route through — not necessarily the one the app should
//! describe. `getActiveNetwork()` plus `getNetworkCapabilities()` answers the
//! same question with a richer answer, and the capabilities carry both facts
//! this crate needs: the transports (Wi-Fi / cellular / ethernet / bluetooth)
//! and `NET_CAPABILITY_NOT_METERED`.
//!
//! # What "connected" means here
//!
//! A network with `NET_CAPABILITY_INTERNET` but not `NET_CAPABILITY_VALIDATED`
//! is connected to something that is not the internet — a captive portal, a
//! hotel Wi-Fi login page. This crate reports `is_connected` from
//! `INTERNET` alone and leaves validation to the caller, because "the OS has a
//! usable path" and "the path reaches the open internet" are different
//! questions and only the first is what a capability plugin can answer
//! synchronously.

use super::{ConnectionType, ConnectivityStatus};
use floom_platform_android::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `ConnectivityPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.connectivity.ConnectivityPlugin";

pub fn connection_type() -> ConnectionType {
    read_capabilities()
        .map(|c| c.connection_type)
        .unwrap_or(ConnectionType::None)
}

pub fn connectivity_status() -> ConnectivityStatus {
    read_capabilities()
        .map(Capabilities::to_status)
        .unwrap_or_default()
}

pub fn is_connected() -> bool {
    read_capabilities().map(|c| c.has_internet).unwrap_or(false)
}

pub fn is_metered() -> bool {
    read_capabilities().map(|c| c.is_metered).unwrap_or(false)
}

/// The facts read from one `NetworkCapabilities`.
struct Capabilities {
    connection_type: ConnectionType,
    has_internet: bool,
    is_metered: bool,
}

impl Capabilities {
    fn to_status(self) -> ConnectivityStatus {
        ConnectivityStatus {
            connection_type: self.connection_type,
            is_connected: self.has_internet,
            is_metered: self.is_metered,
        }
    }
}

/// Read the active network's capabilities, or `None` when there is no network.
///
/// One round trip serves all four public functions: each of them asks a
/// different question of the same object, and re-reading the capabilities four
/// times would let a network change between two answers in the same call.
fn read_capabilities() -> Option<Capabilities> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let array = env
            .call_static_method(
                &class,
                jni::jni_str!("capabilities"),
                jni::jni_sig!("(Landroid/content/Context;)[I"),
                &[JValue::Object(&context)],
            )
            .e()?
            .l()
            .e()?;
        if array.is_null() {
            // No active network, or capabilities unavailable. There is nothing
            // to describe.
            return Ok(None);
        }
        let array = env.cast_local::<jni::objects::JIntArray>(array).e()?;
        let mut values = [0i32; 3];
        env.get_int_array_region(&array, 0, &mut values).e()?;
        let (mask, has_internet, is_metered) = (values[0], values[1] != 0, values[2] != 0);
        Ok(Some(Capabilities {
            connection_type: from_transports(mask),
            has_internet,
            is_metered,
        }))
    })
    .ok()
    .flatten()
}

/// Map the transport bitmask onto the crate's enum.
///
/// The order is a priority, not a set: a phone on Wi-Fi that also has a cellular
/// radio active reports both transports, and the one the user would name is
/// Wi-Fi. Ethernet outranks Wi-Fi because a docked device is on the wire.
fn from_transports(mask: i32) -> ConnectionType {
    // `NetworkCapabilities.TRANSPORT_CELLULAR` = 0, `_WIFI` = 1,
    // `_BLUETOOTH` = 2, `_ETHERNET` = 3.
    const CELLULAR: i32 = 1 << 0;
    const WIFI: i32 = 1 << 1;
    const BLUETOOTH: i32 = 1 << 2;
    const ETHERNET: i32 = 1 << 3;

    if mask & ETHERNET != 0 {
        ConnectionType::Ethernet
    } else if mask & WIFI != 0 {
        ConnectionType::Wifi
    } else if mask & CELLULAR != 0 {
        ConnectionType::Mobile
    } else if mask & BLUETOOTH != 0 {
        ConnectionType::Bluetooth
    } else {
        ConnectionType::None
    }
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
