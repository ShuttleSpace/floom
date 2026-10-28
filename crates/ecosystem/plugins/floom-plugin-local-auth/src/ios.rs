//! iOS local authentication via `LAContext`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Both questions are synchronous here
//!
//! `canEvaluatePolicy:error:` answers synchronously whether the device can
//! authenticate with a given policy, and `biometryType` (iOS 11+) names the
//! modality. So [`available_biometric`] returns the real kind — Face ID, Touch
//! ID or optic ID — rather than a guess, and a device with no enrolled biometric
//! reports `None` because the policy check genuinely failed.
//!
//! # Why the prompt cannot report its result
//!
//! `evaluatePolicy:localizedReason:reply:` answers through a block on an
//! arbitrary queue. Waiting for it with a semaphore — the trick
//! `floom-plugin-notifications` uses for the *settings* read — would deadlock
//! here, because the prompt's UI is drawn on the main thread and blocks until
//! the user taps. So [`authenticate`] refuses with that reason instead of
//! launching a prompt whose answer nobody would see.
//!
//! This is a real limitation of a synchronous API, not an unwired arm: the
//! difference is that `available_biometric` above *is* wired and returns
//! measured values, so a caller can tell the plugin works and this one call
//! cannot.

use super::BiometricKind;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

// `LAContext` is reached through `objc_getClass`, which only finds it if
// LocalAuthentication is loaded.
#[link(name = "LocalAuthentication", kind = "framework")]
unsafe extern "C" {}

/// `LABiometryType`.
const LABIOMETRY_TYPE_NONE: i64 = 0;
const LABIOMETRY_TYPE_TOUCH_ID: i64 = 1;
const LABIOMETRY_TYPE_FACE_ID: i64 = 2;
const LABIOMETRY_TYPE_OPTIC_ID: i64 = 4;

/// `LAPolicy.deviceOwnerAuthenticationWithBiometrics`.
const LAPOLICY_BIOMETRICS: i64 = 1;

pub fn available_biometric() -> BiometricKind {
    unsafe {
        let context: *mut AnyObject = msg_send![class!(LAContext), new];
        if context.is_null() {
            return BiometricKind::None;
        }

        // `canEvaluatePolicy:error:` takes an `NSError**`; passing null is
        // documented as "do not care about the reason", which is right here
        // because the only reason that matters is "cannot".
        let can_evaluate: bool = msg_send![
            context,
            canEvaluatePolicy: LAPOLICY_BIOMETRICS,
            error: std::ptr::null_mut::<*mut AnyObject>()
        ];
        if !can_evaluate {
            // No enrolled biometric, or the device has none: the policy check
            // already said so, so `biometryType` is not consulted.
            return BiometricKind::None;
        }

        // `biometryType` is only populated *after* a policy check on iOS 11+,
        // which is exactly the order used above.
        let biometry: i64 = msg_send![context, biometryType];
        match biometry {
            LABIOMETRY_TYPE_TOUCH_ID => BiometricKind::Fingerprint,
            // Face ID and optic ID are both "the device looks at you"; the
            // crate has no separate optic kind, and folding it into `Face` is
            // the closest true statement — it is not a fingerprint.
            LABIOMETRY_TYPE_FACE_ID | LABIOMETRY_TYPE_OPTIC_ID => BiometricKind::Face,
            LABIOMETRY_TYPE_NONE => BiometricKind::None,
            // A modality newer than this match: report `None` rather than
            // mislabel it as one of the three the enum knows.
            _ => BiometricKind::None,
        }
    }
}

pub fn authenticate(reason: &str) -> Result<(), String> {
    if reason.is_empty() {
        return Err("local-auth: the prompt reason is empty".into());
    }
    // The reason is named so a caller can see it was received; the refusal is
    // about the result, not the input.
    Err(format!(
        "local-auth: iOS answers evaluatePolicy through a reply block, and the \
         prompt it shows is drawn on the main thread — waiting for the block \
         from here would deadlock. Reason received: {reason:?}. Call \
         LAContext.evaluatePolicy from the app's UIKit layer. \
         available_biometric() does work and reports the real modality."
    ))
}
