//! Android local authentication via `LocalAuthPlugin`.
//!
//! The capability is Java: `KeyguardManager` is an Android API, and
//! `LocalAuthPlugin` owns it. This file only carries the ask across JNI.
//!
//! # What this arm actually uses, and why
//!
//! The modern API is `androidx.biometric.BiometricPrompt`, which is an androidx
//! class: using it means adding the `androidx.biometric` dependency and a
//! `FragmentActivity` to host the prompt. The example apps here are
//! `android:hasCode="false"` `NativeActivity` builds with no androidx, so
//! `BiometricPrompt` is not available to this plugin at all — a
//! `ClassNotFoundException` at the first call, not a compile error, because the
//! class is resolved by name at runtime.
//!
//! What *is* in the framework, since API 16, is `KeyguardManager`:
//!
//! * `isDeviceSecure()` — whether a PIN, pattern, password or biometric is
//!   enrolled. That is the platform's answer to "can this device authenticate
//!   the user at all", which is the question `available_biometric` asks.
//! * `createConfirmDeviceCredentialIntent()` — an `Intent` that runs the system
//!   device-credential screen. This is a real authentication prompt, owned by
//!   the system, launched with `startActivityForResult`.
//!
//! # Why `Fingerprint` and not `Face` / `Iris`
//!
//! `KeyguardManager` reports *that* the device is secured, not *which* modality
//! secures it. Android's `PackageManager.FEATURE_FINGERPRINT` is a separate,
//! synchronous query, so the kind is reported from that when present, and
//! otherwise the device is secured by a credential the framework will not name.
//! Guessing `Face` because the phone has a face sensor would be exactly the
//! kind of unverifiable claim this plugin family exists to avoid.
//!
//! # Why `authenticate` reports the result it cannot see
//!
//! `createConfirmDeviceCredentialIntent` is launched for a result, and the
//! result arrives in `onActivityResult` — a callback the NativeActivity glue
//! does not forward to Floom's JNI bridge (the example manifest says so
//! explicitly). So this arm launches the prompt and reports that the *result* is
//! unavailable, rather than returning `Ok` when the intent was merely started.

use super::BiometricKind;
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `LocalAuthPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.local_auth.LocalAuthPlugin";

/// `LocalAuthPlugin.BIOMETRIC_NONE`.
const BIOMETRIC_NONE: i32 = 0;
/// `LocalAuthPlugin.BIOMETRIC_FINGERPRINT`.
const BIOMETRIC_FINGERPRINT: i32 = 1;

pub fn available_biometric() -> BiometricKind {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let kind = env
            .call_static_method(
                &class,
                jni::jni_str!("availableBiometric"),
                jni::jni_sig!("(Landroid/content/Context;)I"),
                &[JValue::Object(&context)],
            )
            .e()?
            .i()
            .e()?;
        Ok(match kind {
            BIOMETRIC_FINGERPRINT => BiometricKind::Fingerprint,
            _ => BiometricKind::None,
        })
    })
    .unwrap_or(BiometricKind::None)
}

pub fn authenticate(reason: &str) -> Result<(), String> {
    if reason.is_empty() {
        return Err("local-auth: the prompt reason is empty".into());
    }
    let reason_for_msg = reason.to_string();
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let reason = env.new_string(reason).e()?;
        let launched = env
            .call_static_method(
                &class,
                jni::jni_str!("authenticate"),
                jni::jni_sig!("(Landroid/content/Context;Ljava/lang/String;)Z"),
                &[JValue::Object(&context), JValue::Object(&reason)],
            )
            .e()?
            .z()
            .e()?;
        if !launched {
            // The platform returns null when no credential is enrolled; there
            // is nothing to prompt for.
            return Err(
                "local-auth: the device has no enrolled credential, so there is \
                 nothing to authenticate against"
                    .into(),
            );
        }
        Err(format!(
            "local-auth: the device-credential prompt was launched for {reason_for_msg:?}, \
             but its result arrives in onActivityResult, which the \
             hasCode=\"false\" NativeActivity does not forward to Floom. \
             Treat this call as \"prompt shown\", not \"authenticated\"."
        ))
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
