//! Android Wi-Fi identity via `WifiManager`.
//!
//! The Java sequence:
//!
//! ```java
//! WifiManager wm = (WifiManager) activity.getSystemService("wifi");
//! WifiInfo info = wm.getConnectionInfo();
//! info.getSSID();    // "\"MyNet\"" when known, "<unknown ssid>" when withheld
//! info.getBSSID();   // "aa:bb:cc:dd:ee:ff" or null
//! ```
//!
//! # Why `<unknown ssid>` is a `None`, not an error
//!
//! Since API 27 the platform requires `ACCESS_FINE_LOCATION` — granted, not
//! merely declared, and with location services on — before it will name a
//! network, because an SSID is a location. Without all three it returns the
//! sentinel `<unknown ssid>` rather than throwing, and that state is the
//! platform *answering*: "I am not willing to say". A `None` field plus this
//! doc is the honest mapping; an `Err` would force every caller to treat
//! "the app has not asked the user for location" as a failure.
//!
//! `getConnectionInfo` itself is deprecated from API 31, but it still
//! answers, and the runtime's behaviour — not the header's annotation — is
//! what this arm reports.
//!
//! # The quotes
//!
//! A *known* SSID arrives wrapped in literal double quotes (a WifiInfo
//! serialization quirk older than most apps); this arm strips them, so the
//! caller reads the name as the user does.

use super::WifiInfo;
use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::JObject;
use jni::strings::JNIString;

/// `Context.WIFI_SERVICE`.
const WIFI_SERVICE: &str = "wifi";

/// `WifiManager.UNKNOWN_SSID` — "able to talk Wi-Fi, not willing to name it".
const UNKNOWN_SSID: &str = "<unknown ssid>";

pub fn wifi_info() -> Result<WifiInfo, String> {
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let service: JObject = env.new_string(WIFI_SERVICE).e()?.into();
        let manager = env
            .call_method(
                &activity,
                jni::jni_str!("getSystemService"),
                jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                &[obj(&service)],
            )
            .and_then(|v| v.l())
            .e()?;
        if manager.is_null() {
            return Err(
                "wifi_info: no WifiManager service — this device has no Wi-Fi manager \
                 to ask"
                    .into(),
            );
        }

        let info = env
            .call_method(
                &manager,
                jni::jni_str!("getConnectionInfo"),
                jni::jni_sig!("()Landroid/net/wifi/WifiInfo;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if info.is_null() {
            return Err("wifi_info: getConnectionInfo returned null".into());
        }

        let ssid = string_call(env, &info, "getSSID")
            // The sentinel is the platform's refusal, not a name.
            .filter(|ssid| ssid != UNKNOWN_SSID)
            .map(|ssid| ssid.trim_matches('"').to_string())
            .and_then(|ssid| if ssid.is_empty() { None } else { Some(ssid) });
        let bssid = string_call(env, &info, "getBSSID");

        Ok(WifiInfo { ssid, bssid })
    })
}

/// A no-argument `java.lang.String` method call as a Rust string, or `None`
/// on null — the state "the platform did not name it".
fn string_call(env: &mut jni::Env, object: &JObject<'_>, method: &str) -> Option<String> {
    let value = env
        .call_method(
            object,
            JNIString::from(method),
            jni::jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|v| v.l())
        .ok()?;
    if value.is_null() {
        return None;
    }
    let string = env.as_cast::<jni::objects::JString<'_>>(&value).ok()?;
    let s = string.try_to_string(env).ok()?;
    if s.is_empty() { None } else { Some(s) }
}
