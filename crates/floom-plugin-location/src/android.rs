//! Android location via `LocationManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Why `LocationManager` and not `FusedLocationProviderClient`
//!
//! Fused location is part of Google Play Services, which is a large
//! `com.google.android.gms` dependency that the example apps here do not carry
//! (they are `android:hasCode="false"` `NativeActivity` builds). `LocationManager`
//! is the framework's own provider and answers the same three questions this
//! crate asks, so no dependency is added.
//!
//! # What each call can honestly answer
//!
//! * `authorization_status` — a real permission check via
//!   `Context.checkSelfPermission`, mapped onto the crate's enum. Android has no
//!   "when in use" vs "always" split in the permission string for *foreground*
//!   access, so a granted `ACCESS_FINE_LOCATION` is reported as
//!   `AuthorizedWhenInUse` and a granted `ACCESS_BACKGROUND_LOCATION` as
//!   `AuthorizedAlways`.
//! * `is_location_enabled` — `LocationManager.isLocationEnabled()` (API 28+),
//!   falling back to `isProviderEnabled(GPS_PROVIDER)` on older devices. This is
//!   the system-wide location switch, not the app's permission.
//! * `current_location` — `getLastKnownLocation` is a synchronous read of the
//!   last fix the OS cached. It is *not* a fresh fix, and that distinction is
//!   documented on the returned value: a cached fix can be hours old, so the
//!   timestamp is what lets a caller decide.
//! * `request_authorization` — issues the real `requestPermissions` call. `Ok`
//!   means "the OS has been asked", not "the user agreed": the result arrives in
//!   `onRequestPermissionsResult`, a callback this crate does not take, so the
//!   caller polls `authorization_status()`. It returns `Err` when the status is
//!   already `Denied`, because the OS will not prompt twice — asking again would
//!   report `Ok` for a prompt that never appears.

use super::{Location, LocationAuthorization};
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

/// `Manifest.permission.ACCESS_FINE_LOCATION`.
const ACCESS_FINE_LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";
/// `Manifest.permission.ACCESS_BACKGROUND_LOCATION`.
const ACCESS_BACKGROUND_LOCATION: &str = "android.permission.ACCESS_BACKGROUND_LOCATION";

/// `PackageManager.PERMISSION_GRANTED`.
const PERMISSION_GRANTED: i32 = 0;

pub fn authorization_status() -> LocationAuthorization {
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let fine = check_permission(env, &activity, ACCESS_FINE_LOCATION)?;
        if fine != PERMISSION_GRANTED {
            // Denied or never asked: Android does not distinguish "denied" from
            // "not yet asked" through this API, and `shouldShowRequestPermissionRationale`
            // is the only signal — it returns false for both. Reporting
            // `NotDetermined` keeps the caller's next step (ask) correct either
            // way.
            return Ok(LocationAuthorization::NotDetermined);
        }
        let background = check_permission(env, &activity, ACCESS_BACKGROUND_LOCATION)?;
        Ok(if background == PERMISSION_GRANTED {
            LocationAuthorization::AuthorizedAlways
        } else {
            LocationAuthorization::AuthorizedWhenInUse
        })
    })
    .unwrap_or(LocationAuthorization::NotDetermined)
}

pub fn is_location_enabled() -> bool {
    jni_helpers::with_env(|env| {
        let manager = location_manager(env)?;
        // `isLocationEnabled()` is API 28+. On older devices the closest
        // equivalent is asking whether the GPS provider is enabled, which is
        // what the system switch gates.
        match env
            .call_method(
                &manager,
                jni::jni_str!("isLocationEnabled"),
                jni::jni_sig!("()Z"),
                &[],
            )
            .and_then(|v| v.z())
        {
            Ok(enabled) => Ok(enabled),
            Err(_) => {
                env.exception_clear();
                let provider = env.new_string("gps").e()?;
                env.call_method(
                    &manager,
                    jni::jni_str!("isProviderEnabled"),
                    jni::jni_sig!("(Ljava/lang/String;)Z"),
                    &[JValue::Object(&provider)],
                )
                .e()?
                .z()
                .e()
            }
        }
    })
    .unwrap_or(false)
}

pub fn request_authorization() -> Result<(), String> {
    match authorization_status() {
        LocationAuthorization::AuthorizedAlways | LocationAuthorization::AuthorizedWhenInUse => {
            Ok(())
        }
        LocationAuthorization::Restricted => Err(
            "location: the app is restricted (parental controls or device policy), so \
             the OS will not show a permission prompt"
                .into(),
        ),
        LocationAuthorization::Denied => Err(
            "location: permission was denied; Android will not prompt a second time. \
             The user has to re-grant it from Settings → Apps → Permissions."
                .into(),
        ),
        LocationAuthorization::NotDetermined => jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let permission = env.new_string(ACCESS_FINE_LOCATION).e()?;
            let permission_array = env
                .new_object_array(1, jni::jni_str!("java/lang/String"), &permission)
                .e()?;
            // The request code is arbitrary: the result is delivered to
            // `onRequestPermissionsResult`, which this plugin does not
            // implement, so the value is never read back. The prompt still
            // appears, which is the whole job of this call.
            env.call_method(
                &activity,
                jni::jni_str!("requestPermissions"),
                jni::jni_sig!("([Ljava/lang/String;I)V"),
                &[
                    JValue::Object(&JObject::from(permission_array)),
                    JValue::Int(0),
                ],
            )
            .e()?;
            Ok(())
        }),
    }
}

pub fn current_location() -> Result<Location, String> {
    if authorization_status() == LocationAuthorization::NotDetermined {
        // Asking the provider without permission throws a SecurityException
        // inside the JVM, which would surface as an opaque JNI error.
        return Err("location: no location permission is granted; check \
             authorization_status() first"
            .into());
    }
    jni_helpers::with_env(|env| {
        let manager = location_manager(env)?;
        // `GPS_PROVIDER` first, then `NETWORK_PROVIDER`: a GPS fix is more
        // accurate, and the network provider is the fallback for an indoor
        // device. Both are last-known reads, so neither blocks.
        let mut fix = last_known(env, &manager, "gps")?;
        if fix.is_none() {
            fix = last_known(env, &manager, "network")?;
        }
        let location = fix.ok_or_else(|| {
            "location: no last-known fix is cached; the OS has not produced one \
             since boot, so there is nothing to report"
                .to_string()
        })?;

        let latitude = call_double(env, &location, "getLatitude")?;
        let longitude = call_double(env, &location, "getLongitude")?;
        let altitude = call_double(env, &location, "getAltitude")?;
        let accuracy = call_float(env, &location, "getAccuracy")?;
        // `Location.getTime()` is Unix milliseconds; the crate's field is
        // seconds, so this is the one conversion the struct requires.
        let time_ms = env
            .call_method(
                &location,
                jni::jni_str!("getTime"),
                jni::jni_sig!("()J"),
                &[],
            )
            .e()?
            .j()
            .e()?;

        Ok(Location {
            latitude,
            longitude,
            altitude,
            horizontal_accuracy: accuracy as f64,
            // `Location` carries no vertical accuracy; `getVerticalAccuracyMeters`
            // is API 26+ and returns `0` when unknown, which the crate documents
            // as "unavailable" through a negative value.
            vertical_accuracy: -1.0,
            timestamp: time_ms / 1000,
        })
    })
}

/// `LocationManager.getLastKnownLocation(provider)`.
fn last_known<'local>(
    env: &mut jni::Env<'local>,
    manager: &JObject<'_>,
    provider: &str,
) -> Result<Option<JObject<'local>>, String> {
    let name = env.new_string(provider).e()?;
    let location = env
        .call_method(
            manager,
            jni::jni_str!("getLastKnownLocation"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/location/Location;"),
            &[JValue::Object(&name)],
        )
        .e()?
        .l()
        .e()?;
    // A provider that exists but has no fix returns null, which is the
    // documented "nothing cached" answer.
    Ok(if location.is_null() {
        None
    } else {
        Some(location)
    })
}

/// `Context.checkSelfPermission(permission)`.
fn check_permission(
    env: &mut jni::Env,
    activity: &JObject<'_>,
    permission: &str,
) -> Result<i32, String> {
    let name = env.new_string(permission).e()?;
    Ok(env
        .call_method(
            activity,
            jni::jni_str!("checkSelfPermission"),
            jni::jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&name)],
        )
        .e()?
        .i()
        .e()?)
}

/// `Context.getSystemService(LocationManager.class)`.
fn location_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/location/LocationManager"))
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

/// `Location.getLatitude()` / `getLongitude()` / `getAltitude()`.
fn call_double(env: &mut jni::Env, location: &JObject<'_>, method: &str) -> Result<f64, String> {
    let name = jni::strings::JNIString::from(method);
    Ok(env
        .call_method(location, name, jni::jni_sig!("()D"), &[])
        .e()?
        .d()
        .e()?)
}

/// `Location.getAccuracy()`.
fn call_float(env: &mut jni::Env, location: &JObject<'_>, method: &str) -> Result<f32, String> {
    let name = jni::strings::JNIString::from(method);
    Ok(env
        .call_method(location, name, jni::jni_sig!("()F"), &[])
        .e()?
        .f()
        .e()?)
}
