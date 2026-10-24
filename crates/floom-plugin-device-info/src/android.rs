//! Android device facts via `android.os.Build`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! Every field is read through the JNI bridge in `floom_platform_mobile::jni`.
//! `Build` holds the hardware facts as `public static final` fields; the OS
//! version and API level live in the nested `Build.VERSION` class.
//!
//! # Why the reads are defensive
//!
//! A `Build` field can be missing for reasons that have nothing to do with the
//! device — an OEM that stripped it, a ProGuard rule that renamed it — so each
//! read falls back to an empty string and the *absence* shows up as an empty
//! field rather than as a failed call. A device fact that cannot be read is an
//! empty string, not an error: the caller asked "what is this device", and
//! "I don't know" is a legitimate answer that a `Result` would force the
//! caller to treat as a failure.
//!
//! The one field worth being exactly right about is `is_physical_device`,
//! because an app that gates behaviour on it (analytics, licensing) would
//! otherwise be told the truth about the wrong machine.

use super::DeviceInfo;
use floom_platform_mobile::jni::{self as jni_helpers};
use jni::objects::{JClass, JObject, JString};
use jni::strings::JNIString;

pub fn device_info() -> DeviceInfo {
    jni_helpers::with_env(|env| {
        let model = build_string(env, "MODEL", "");
        let manufacturer = build_string(env, "MANUFACTURER", "");
        let fingerprint = build_string(env, "FINGERPRINT", "");
        let os_version = build_string(env, "RELEASE", "VERSION");
        let sdk_int = build_int(env, "SDK_INT", "VERSION");

        // The emulator's fingerprint starts with `generic` (or carries
        // `vbox`/`generic_sdk`); a real device's does not. This is the cheapest
        // reliable "not real silicon" signal Android exposes without a
        // permission.
        let is_physical = !fingerprint.is_empty()
            && !fingerprint.starts_with("generic")
            && !fingerprint.contains("vbox")
            && !fingerprint.contains("generic_sdk")
            && fingerprint != "unknown";

        Ok(DeviceInfo {
            model,
            manufacturer,
            os_name: "Android".to_string(),
            os_version,
            sdk_version: sdk_int,
            is_physical_device: is_physical,
            locale: locale(env),
        })
    })
    .unwrap_or_default()
}

pub fn device_name() -> String {
    // Android has no single public "device name" the way iOS's `UIDevice.name`
    // is one. The user-visible name lives in `Settings.Global.DEVICE_NAME`
    // (API 25+) or `Settings.Secure.bluetooth_name`, neither readable without a
    // permission. `Build.MODEL` is the honest public answer, and it is what the
    // user sees in most places anyway.
    jni_helpers::with_env(|env| Ok(build_string(env, "MODEL", ""))).unwrap_or_default()
}

/// `Build$<class>.<field>` as a Rust string, or empty when it cannot be read.
///
/// An empty `class` means `Build` itself, where the hardware facts live;
/// `"VERSION"` means the nested class, where the OS ones do.
///
/// The field name is a runtime `JNIString`, which is what lets one helper
/// serve every field. The signature is a `jni_sig!` literal because that macro
/// takes only literals — and there are only two signatures in this module, so
/// the asymmetry costs nothing.
fn build_string(env: &mut jni::Env, field: &str, class: &str) -> String {
    let Some(cls) = build_class(env, class) else {
        return String::new();
    };
    let Ok(value) = env.get_static_field(
        &cls,
        JNIString::from(field),
        jni::jni_sig!("Ljava/lang/String;"),
    ) else {
        return String::new();
    };
    let Ok(obj) = value.l() else {
        return String::new();
    };
    object_to_string(env, &obj)
}

/// `Build$<class>.<field>` as an `i32`.
fn build_int(env: &mut jni::Env, field: &str, class: &str) -> Option<i32> {
    let cls = build_class(env, class)?;
    let value = env
        .get_static_field(&cls, JNIString::from(field), jni::jni_sig!("I"))
        .ok()?;
    value.i().ok()
}

/// The `Build` class, or one of its nested classes.
///
/// The nested classes' JNI names use a `$`: `Build$VERSION`. An empty `class`
/// is `Build` itself.
fn build_class<'local>(env: &mut jni::Env<'local>, class: &str) -> Option<JClass<'local>> {
    // `find_class` takes an `AsRef<JNIStr>`, and a `JNIString` is the runtime
    // constructor for one — `jni_str!` only accepts a literal, so a nested class
    // name assembled with `format!` would be looked up as the literal text
    // `"{name}"` and never resolve.
    let name = if class.is_empty() {
        "android/os/Build".to_string()
    } else {
        format!("android/os/Build${class}")
    };
    env.find_class(JNIString::from(name)).ok()
}

/// A `java.lang.String` object as a Rust `String`.
///
/// Empty when the object is null or is not a string — which is what a
/// defensively-read `Build` field can hand over. `as_cast` is the safe route:
/// it borrows the object, checks at runtime that it really is a
/// `java.lang.String`, and hands back a wrapper that derefs to `JString`. No
/// cast of the raw handle is involved, and the original reference stays valid.
fn object_to_string(env: &mut jni::Env, obj: &JObject<'_>) -> String {
    if obj.is_null() {
        return String::new();
    }
    let Ok(string) = env.as_cast::<JString<'_>>(obj) else {
        return String::new();
    };
    // `try_to_string` is the non-deprecated route: it borrows the JString and
    // releases the characters itself, where `Env::get_string` leaks a guard the
    // caller has to remember to drop.
    string.try_to_string(env).unwrap_or_default()
}

/// The device's locale as a BCP-47 tag, via `Locale.getDefault()`.
fn locale(env: &mut jni::Env) -> String {
    let Ok(cls) = env.find_class(jni::jni_str!("java/util/Locale")) else {
        return String::new();
    };
    let Ok(default) = env
        .call_static_method(
            &cls,
            jni::jni_str!("getDefault"),
            jni::jni_sig!("()Ljava/util/Locale;"),
            &[],
        )
        .and_then(|v| v.l())
    else {
        return String::new();
    };
    let Ok(tag) = env
        .call_method(
            &default,
            jni::jni_str!("toLanguageTag"),
            jni::jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|v| v.l())
    else {
        return String::new();
    };
    // `toLanguageTag()` is the BCP-47 form (`en-US`); `toString()` gives
    // `en_US`, which is not what the field documents.
    object_to_string(env, &tag)
}
