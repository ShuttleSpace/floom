//! Android connectivity via `ConnectivityManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
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
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

pub fn connection_type() -> ConnectionType {
    jni_helpers::with_env(|env| {
        Ok(read_capabilities(env)?
            .map(|c| c.connection_type)
            .unwrap_or(ConnectionType::None))
    })
    .unwrap_or(ConnectionType::None)
}

pub fn connectivity_status() -> ConnectivityStatus {
    jni_helpers::with_env(|env| {
        Ok(read_capabilities(env)?
            .map(Capabilities::to_status)
            .unwrap_or_default())
    })
    .unwrap_or_default()
}

pub fn is_connected() -> bool {
    jni_helpers::with_env(|env| {
        Ok(read_capabilities(env)?
            .map(|c| c.has_internet)
            .unwrap_or(false))
    })
    .unwrap_or(false)
}

pub fn is_metered() -> bool {
    jni_helpers::with_env(|env| {
        Ok(read_capabilities(env)?
            .map(|c| c.is_metered)
            .unwrap_or(false))
    })
    .unwrap_or(false)
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
fn read_capabilities(env: &mut jni::Env) -> Result<Option<Capabilities>, String> {
    let Some(network) = active_network(env)? else {
        return Ok(None);
    };
    let manager = connectivity_manager(env)?;
    let caps = env
        .call_method(
            &manager,
            jni::jni_str!("getNetworkCapabilities"),
            jni::jni_sig!("(Landroid/net/Network;)Landroid/net/NetworkCapabilities;"),
            &[JValue::Object(&network)],
        )
        .e()?
        .l()
        .e()?;
    if caps.is_null() {
        // The network was lost between the two calls; there is nothing to
        // describe.
        return Ok(None);
    }

    let transports = transport_mask(env, &caps)?;
    let has_internet = has_capability(env, &caps, "NET_CAPABILITY_INTERNET")?;
    // `NOT_METERED` is the one capability that is phrased negatively, so its
    // *absence* is what makes a connection metered.
    let not_metered = has_capability(env, &caps, "NET_CAPABILITY_NOT_METERED")?;

    Ok(Some(Capabilities {
        connection_type: from_transports(transports),
        has_internet,
        is_metered: !not_metered,
    }))
}

/// `ConnectivityManager.getActiveNetwork()`.
fn active_network<'local>(env: &mut jni::Env<'local>) -> Result<Option<JObject<'local>>, String> {
    let manager = connectivity_manager(env)?;
    let network = env
        .call_method(
            &manager,
            jni::jni_str!("getActiveNetwork"),
            jni::jni_sig!("()Landroid/net/Network;"),
            &[],
        )
        .e()?
        .l()
        .e()?;
    Ok(if network.is_null() {
        None
    } else {
        Some(network)
    })
}

/// `Context.getSystemService(ConnectivityManager.class)`.
fn connectivity_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/net/ConnectivityManager"))
        .e()?;
    env.call_method(
        &activity,
        jni::jni_str!("getSystemService"),
        jni::jni_sig!("(Ljava/lang/Class;)Ljava/lang/Object;"),
        &[JValue::Object(&JObject::from(class))],
    )
    .e()?
    .l()
    .e()
}

/// The `TRANSPORT_*` bitmask from `NetworkCapabilities.getTransports()`.
///
/// `getTransports()` returns a primitive `int[]`, so it is cast to `JIntArray`
/// (which checks the runtime type) and read with `get_region` rather than
/// element by element.
fn transport_mask(env: &mut jni::Env, caps: &JObject<'_>) -> Result<i32, String> {
    let array = env
        .call_method(
            caps,
            jni::jni_str!("getTransports"),
            jni::jni_sig!("()[I"),
            &[],
        )
        .e()?
        .l()
        .e()?;
    if array.is_null() {
        return Ok(0);
    }
    let array = jni::objects::JIntArray::cast_local(env, array).e()?;
    let len = array.len(env).e()?;
    let mut transports = vec![0i32; len];
    array.get_region(env, 0, &mut transports).e()?;

    // `NetworkCapabilities.TRANSPORT_CELLULAR` = 0, `_WIFI` = 1,
    // `_BLUETOOTH` = 2, `_ETHERNET` = 3.
    Ok(transports.iter().fold(0i32, |mask, t| mask | (1 << t)))
}

/// `NetworkCapabilities.hasCapability(int)`, with the constant fetched by name.
///
/// The constants are `static final int`s on `NetworkCapabilities`, so they are
/// read rather than hard-coded: `NET_CAPABILITY_INTERNET` is `12` today, and a
/// literal here would be a number nobody could check against the platform.
fn has_capability(env: &mut jni::Env, caps: &JObject<'_>, name: &str) -> Result<bool, String> {
    let class = env
        .find_class(jni::jni_str!("android/net/NetworkCapabilities"))
        .e()?;
    let value = env
        .get_static_field(
            &class,
            jni::strings::JNIString::from(name),
            jni::jni_sig!("I"),
        )
        .e()?
        .i()
        .e()?;
    let result = env
        .call_method(
            caps,
            jni::jni_str!("hasCapability"),
            jni::jni_sig!("(I)Z"),
            &[JValue::Int(value)],
        )
        .e()?;
    result.z().e()
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
