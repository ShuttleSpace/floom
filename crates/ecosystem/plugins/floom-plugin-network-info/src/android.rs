//! Android Wi-Fi identity via `NetworkInfoPlugin`.
//!
//! The capability is Java: `WifiManager` is an Android API, and
//! `NetworkInfoPlugin` owns it. This file only carries the ask across JNI.

use super::WifiInfo;
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::{JObjectArray, JValue};

/// `NetworkInfoPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.network_info.NetworkInfoPlugin";

/// `WifiManager.UNKNOWN_SSID` — "able to talk Wi-Fi, not willing to name it".
const UNKNOWN_SSID: &str = "<unknown ssid>";

pub fn wifi_info() -> Result<WifiInfo, String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let array = env
            .call_static_method(
                &class,
                jni::jni_str!("wifiInfo"),
                jni::jni_sig!("(Landroid/content/Context;)[Ljava/lang/String;"),
                &[JValue::Object(&context)],
            )
            .e()?
            .l()
            .e()?;
        let array: JObjectArray = env.cast_local::<JObjectArray>(array).e()?;

        let ssid = string_at(env, &array, 0)
            .filter(|ssid| ssid != UNKNOWN_SSID)
            .map(|ssid| ssid.trim_matches('"').to_string())
            .and_then(|ssid| if ssid.is_empty() { None } else { Some(ssid) });
        let bssid = string_at(env, &array, 1);

        Ok(WifiInfo { ssid, bssid })
    })
}

/// Element `index` of a `String[]`, or `None` when the entry is null or empty.
fn string_at(env: &mut jni::Env<'_>, array: &JObjectArray<'_>, index: usize) -> Option<String> {
    let value = env.get_object_array_element(array, index).ok()?;
    if value.is_null() {
        return None;
    }
    let string = env.as_cast::<jni::objects::JString<'_>>(&value).ok()?;
    let s = string.try_to_string(env).ok()?;
    if s.is_empty() { None } else { Some(s) }
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
