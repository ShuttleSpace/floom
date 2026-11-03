//! Android device facts via `DeviceInfoPlugin`.
//!
//! The capability is Java: `Build` fields are public static final fields; the
//! OS version and API level live in the nested `Build.VERSION` class.
//! `DeviceInfoPlugin` owns them. This file only carries the ask across JNI.
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
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JObjectArray;

/// `DeviceInfoPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.device_info.DeviceInfoPlugin";

pub fn device_info() -> DeviceInfo {
    bridge::with_env(|env| {
        let array = array(env)?;
        let (model, manufacturer, fingerprint, os_version, sdk_version, locale) =
            basics(env, &array)?;

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
            sdk_version: Some(sdk_version),
            is_physical_device: is_physical,
            locale,
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
    bridge::with_env(|env| {
        let array = array(env)?;
        Ok(basics(env, &array)?.0)
    })
    .unwrap_or_default()
}

/// `[model, manufacturer, fingerprint, osVersion, sdkInt, locale]`.
fn basics<'local>(
    env: &mut jni::Env<'local>,
    array: &JObjectArray<'local>,
) -> Result<(String, String, String, String, i32, String), String> {
    let model = string_at(array, env, 0).unwrap_or_default();
    let manufacturer = string_at(array, env, 1).unwrap_or_default();
    let fingerprint = string_at(array, env, 2).unwrap_or_default();
    let os_version = string_at(array, env, 3).unwrap_or_default();
    let sdk_version = string_at(array, env, 4)
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();
    let locale = string_at(array, env, 5).unwrap_or_default();
    Ok((
        model,
        manufacturer,
        fingerprint,
        os_version,
        sdk_version,
        locale,
    ))
}

/// Element `index` of a `String[]`, or `None` when it is null or blank.
fn string_at(array: &JObjectArray, env: &mut jni::Env, index: usize) -> Option<String> {
    let value = array.get_element(env, index).ok()?;
    if value.is_null() {
        return None;
    }
    let string = env.as_cast::<jni::objects::JString<'_>>(&value).ok()?;
    let s = string.try_to_string(env).ok()?;
    if s.is_empty() { None } else { Some(s) }
}

/// `DeviceInfoPlugin.basics()` as a `String[]`, or `None`.
fn array<'local>(env: &mut jni::Env<'local>) -> Result<JObjectArray<'local>, String> {
    let class = class(env)?;
    let array = env
        .call_static_method(
            &class,
            jni::jni_str!("basics"),
            jni::jni_sig!("()[Ljava/lang/String;"),
            &[],
        )
        .e()?
        .l()
        .e()?;
    env.cast_local::<JObjectArray>(array).e()
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
