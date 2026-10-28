//! Android app identity via `PackageManager`.
//!
//! The Java sequence is small:
//!
//! ```java
//! String pkg    = activity.getPackageName();
//! PackageInfo p = activity.getPackageManager().getPackageInfo(pkg, 0);
//! CharSequence label = activity.getApplicationInfo().loadLabel(pm);
//! p.versionName; p.versionCode;
//! ```
//!
//! Every read degrades to `None` on its own: a ROM can omit `versionName`
//! (unsigned builds), and one missing field must not blank the others, so no
//! read is allowed to fail the whole call. `versionCode` is the deprecated
//! `int` field rather than API 28's `longVersionCode` — it is present on every
//! API level, and the string the type carries is the same either way.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::AppInfo;
use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::{JObject, JString};
use jni::strings::JNIString;

pub fn app_info() -> AppInfo {
    jni_helpers::with_env(read_info).unwrap_or_default()
}

fn read_info(env: &mut jni::Env) -> Result<AppInfo, String> {
    let activity = jni_helpers::activity(env)?;

    let package_name = {
        let value = env
            .call_method(
                &activity,
                jni::jni_str!("getPackageName"),
                jni::jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        object_to_string(env, &value)
    };

    let pm = env
        .call_method(
            &activity,
            jni::jni_str!("getPackageManager"),
            jni::jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    if pm.is_null() {
        // The package name is still an answer the caller can use, even when
        // the manager that would have filled in the rest is not there.
        return Ok(AppInfo {
            package_name: non_empty(package_name),
            ..Default::default()
        });
    }

    let info = {
        let name_arg: JObject = env.new_string(&package_name).e()?.into();
        let value = env
            .call_method(
                &pm,
                jni::jni_str!("getPackageInfo"),
                jni::jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;"),
                &[obj(&name_arg), jni::objects::JValue::Int(0)],
            )
            .and_then(|v| v.l())
            .e()?;
        value
    };
    if info.is_null() {
        return Ok(AppInfo {
            package_name: non_empty(package_name),
            ..Default::default()
        });
    }

    // `PackageInfo.versionName` is a public field; `null` on unsigned builds.
    let version = field_string(env, &info, "versionName");
    // `versionCode`, the deprecated-but-universal `int` field.
    let build_number = env
        .get_field(&info, JNIString::from("versionCode"), jni::jni_sig!("I"))
        .ok()
        .and_then(|v| v.i().ok())
        .map(|code| code.to_string());

    let name = app_label(env, &activity, &pm);

    Ok(AppInfo {
        name,
        package_name: non_empty(package_name),
        version,
        build_number,
    })
}

/// The launcher label: `ApplicationInfo.loadLabel(PackageManager)`.
///
/// This is the one read that can cost real time — it resolves through the
/// app's resources — but `package_info` is a call an app makes once, at about
/// screen time, so the cost is not worth a cache and the complexity a cache
/// would add.
fn app_label(env: &mut jni::Env, activity: &JObject<'_>, pm: &JObject<'_>) -> Option<String> {
    let app_info_obj = env
        .call_method(
            activity,
            jni::jni_str!("getApplicationInfo"),
            jni::jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )
        .and_then(|v| v.l())
        .ok()?;
    if app_info_obj.is_null() {
        return None;
    }
    let label = env
        .call_method(
            &app_info_obj,
            jni::jni_str!("loadLabel"),
            jni::jni_sig!("(Landroid/content/pm/PackageManager;)Ljava/lang/CharSequence;"),
            &[obj(pm)],
        )
        .and_then(|v| v.l())
        .ok()?;
    if label.is_null() {
        return None;
    }
    non_empty(object_to_string(env, &label))
}

/// A `String` public field of an object, or `None` when null.
fn field_string(env: &mut jni::Env, owner: &JObject<'_>, field: &str) -> Option<String> {
    let value = env
        .get_field(
            owner,
            JNIString::from(field),
            jni::jni_sig!("Ljava/lang/String;"),
        )
        .ok()?;
    let obj = value.l().ok()?;
    if obj.is_null() {
        return None;
    }
    non_empty(object_to_string(env, &obj))
}

/// A `java.lang.String` object as a Rust `String`.
///
/// Empty on null or on a non-string, which is what a defensively-read field
/// can hand over. `as_cast` is the safe route: it borrows the object, checks
/// at runtime that it really is a `java.lang.String`, and hands back a wrapper
/// that derefs to `JString`.
fn object_to_string(env: &mut jni::Env, obj: &JObject<'_>) -> String {
    if obj.is_null() {
        return String::new();
    }
    let Ok(string) = env.as_cast::<JString<'_>>(obj) else {
        return String::new();
    };
    string.try_to_string(env).unwrap_or_default()
}

/// The type promises "never `Some("")`": an empty string is a value nobody
/// measured, and the Apple arm makes the same promise.
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}
