//! Android location via `LocationPlugin`.
//!
//! The capability is Java: `LocationManager` is an Android API, and
//! `LocationPlugin` owns it. This file only carries the ask across JNI.
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
use floom_platform_android::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `LocationPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.location.LocationPlugin";

/// `LocationPlugin.AUTH_*` discriminants.
const AUTH_NOT_DETERMINED: i32 = 0;
const AUTH_WHEN_IN_USE: i32 = 1;
const AUTH_ALWAYS: i32 = 2;
const AUTH_DENIED: i32 = 3;
const AUTH_RESTRICTED: i32 = 4;

pub fn authorization_status() -> LocationAuthorization {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let code = env
            .call_static_method(
                &class,
                jni::jni_str!("authorizationStatus"),
                jni::jni_sig!("(Landroid/content/Context;)I"),
                &[JValue::Object(&context)],
            )
            .e()?
            .i()
            .e()?;
        Ok(match code {
            AUTH_WHEN_IN_USE => LocationAuthorization::AuthorizedWhenInUse,
            AUTH_ALWAYS => LocationAuthorization::AuthorizedAlways,
            AUTH_DENIED => LocationAuthorization::Denied,
            AUTH_RESTRICTED => LocationAuthorization::Restricted,
            _ => LocationAuthorization::NotDetermined,
        })
    })
    .unwrap_or(LocationAuthorization::NotDetermined)
}

pub fn is_location_enabled() -> bool {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let enabled = env
            .call_static_method(
                &class,
                jni::jni_str!("isLocationEnabled"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        Ok(enabled)
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
        LocationAuthorization::NotDetermined => bridge::with_env(|env| {
            let class = class(env)?;
            let context = bridge::activity(env)?;
            let requested = env
                .call_static_method(
                    &class,
                    jni::jni_str!("requestPermissions"),
                    jni::jni_sig!("(Landroid/content/Context;)Z"),
                    &[JValue::Object(&context)],
                )
                .e()?
                .z()
                .e()?;
            if requested {
                Ok(())
            } else {
                Err("location: requestPermissions did not run (no Activity context)".into())
            }
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
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let array = env
            .call_static_method(
                &class,
                jni::jni_str!("currentLocation"),
                jni::jni_sig!("(Landroid/content/Context;)[D"),
                &[JValue::Object(&context)],
            )
            .e()?
            .l()
            .e()?;
        if array.is_null() {
            return Err(
                "location: no last-known fix is cached; the OS has not produced one \
                 since boot, so there is nothing to report"
                    .into(),
            );
        }
        let array = env.cast_local::<jni::objects::JDoubleArray>(array).e()?;
        let mut values = [0.0f64; 5];
        env.get_double_array_region(&array, 0, &mut values).e()?;
        let (latitude, longitude, altitude, accuracy) =
            (values[0], values[1], values[2], values[3]);
        let time_ms = values[4];

        Ok(Location {
            latitude,
            longitude,
            altitude,
            horizontal_accuracy: accuracy as f64,
            // `Location` carries no vertical accuracy; `getVerticalAccuracyMeters`
            // is API 26+ and returns `0` when unknown, which the crate documents
            // as "unavailable" through a negative value.
            vertical_accuracy: -1.0,
            timestamp: (time_ms / 1000.0) as i64,
        })
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
