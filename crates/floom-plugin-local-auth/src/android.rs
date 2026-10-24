//! Android local authentication via `KeyguardManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
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
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

pub fn available_biometric() -> BiometricKind {
    jni_helpers::with_env(|env| {
        let keyguard = keyguard_manager(env)?;
        // `isDeviceSecure()` is API 23+; on 16–22 the deprecated
        // `isKeyguardSecure()` answers the same question. Both are tried in
        // order so the plugin does not report "no biometric" on a secured
        // older device.
        let secure = match env
            .call_method(
                &keyguard,
                jni::jni_str!("isDeviceSecure"),
                jni::jni_sig!("()Z"),
                &[],
            )
            .and_then(|v| v.z())
        {
            Ok(secure) => secure,
            Err(_) => {
                env.exception_clear();
                env.call_method(
                    &keyguard,
                    jni::jni_str!("isKeyguardSecure"),
                    jni::jni_sig!("()Z"),
                    &[],
                )
                .and_then(|v| v.z())
                .unwrap_or(false)
            }
        };
        if !secure {
            return Ok(BiometricKind::None);
        }
        // The device is secured. If it has a fingerprint sensor, that is the
        // modality the system will offer; otherwise the credential is a PIN or
        // pattern, which has no `BiometricKind` and is reported as `None` — the
        // enum names biometrics, not "a lock screen exists".
        Ok(if has_fingerprint(env)? {
            BiometricKind::Fingerprint
        } else {
            BiometricKind::None
        })
    })
    .unwrap_or(BiometricKind::None)
}

pub fn authenticate(reason: &str) -> Result<(), String> {
    if reason.is_empty() {
        return Err("local-auth: the prompt reason is empty".into());
    }
    jni_helpers::with_env(|env| {
        let keyguard = keyguard_manager(env)?;
        let activity = jni_helpers::activity(env)?;

        let intent = env
            .call_method(
                &keyguard,
                jni::jni_str!("createConfirmDeviceCredentialIntent"),
                jni::jni_sig!(
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/Intent;"
                ),
                &[
                    JValue::Object(&JObject::null()),
                    JValue::Object(&JObject::null()),
                ],
            )
            .e()?
            .l()
            .e()?;
        if intent.is_null() {
            // The platform returns null when no credential is enrolled; there
            // is nothing to prompt for.
            return Err(
                "local-auth: the device has no enrolled credential, so there is \
                 nothing to authenticate against"
                    .into(),
            );
        }

        // `startActivityForResult` needs a request code; the result is what the
        // NativeActivity glue does not forward, which is why this function
        // cannot report success.
        env.call_method(
            &activity,
            jni::jni_str!("startActivityForResult"),
            jni::jni_sig!("(Landroid/content/Intent;I)V"),
            &[JValue::Object(&intent), JValue::Int(0)],
        )
        .e()?;

        Err(format!(
            "local-auth: the device-credential prompt was launched for {reason:?}, \
             but its result arrives in onActivityResult, which the \
             hasCode=\"false\" NativeActivity does not forward to Floom. \
             Treat this call as \"prompt shown\", not \"authenticated\"."
        ))
    })
}

/// `Context.getSystemService(KeyguardManager.class)`.
fn keyguard_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/app/KeyguardManager"))
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

/// `PackageManager.hasSystemFeature(FEATURE_FINGERPRINT)`.
fn has_fingerprint(env: &mut jni::Env) -> Result<bool, String> {
    let activity = jni_helpers::activity(env)?;
    let manager = env
        .call_method(
            &activity,
            jni::jni_str!("getPackageManager"),
            jni::jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        )
        .e()?
        .l()
        .e()?;
    // The class is resolved into a local first: `get_static_field` already
    // borrows `env` mutably, so looking it up inline is a second borrow.
    let package_manager = env
        .find_class(jni::jni_str!("android/content/pm/PackageManager"))
        .e()?;
    let feature = env
        .get_static_field(
            &package_manager,
            jni::jni_str!("FEATURE_FINGERPRINT"),
            jni::jni_sig!("Ljava/lang/String;"),
        )
        .e()?
        .l()
        .e()?;
    env.call_method(
        &manager,
        jni::jni_str!("hasSystemFeature"),
        jni::jni_sig!("(Ljava/lang/String;)Z"),
        &[JValue::Object(&feature)],
    )
    .e()?
    .z()
    .e()
}
