//! macOS local authentication via `LAContext`.
//!
//! Touch ID on macOS is the same `LocalAuthentication` framework as Face ID on
//! iOS, reached through the same `LAContext` API. Adapted from the iOS arm
//! (`ios.rs`), which already documents the two questions this crate asks.
//!
//! [`available_biometric`] calls `canEvaluatePolicy:` synchronously and reads
//! `biometryType`, so it returns the *real* modality — `Fingerprint` when Touch
//! ID is enrolled, `None` otherwise. A bare `cargo test` on a Mac with no Touch
//! ID enrolled sees `None`, which is a measurement, not a stub.
//!
//! [`authenticate`] cannot run the prompt from here: `evaluatePolicy:` answers
//! through a reply block on the main thread, and waiting for it from a background
//! Rust call would deadlock against the prompt's own UI. So it refuses with that
//! reason, exactly as the iOS arm does. This is a real limitation of a
//! synchronous API, not an unwired arm — `available_biometric` above is wired
//! and reports the true modality.

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

        // `biometryType` is only populated *after* a policy check on macOS 11+,
        // which is exactly the order used above. macOS ships Touch ID, so the
        // populated value is `Touch ID`; the other arms are kept so a modality
        // newer than this match is reported as `None` rather than mislabeled.
        let biometry: i64 = msg_send![context, biometryType];
        match biometry {
            LABIOMETRY_TYPE_TOUCH_ID => BiometricKind::Fingerprint,
            LABIOMETRY_TYPE_FACE_ID | LABIOMETRY_TYPE_OPTIC_ID => BiometricKind::Face,
            LABIOMETRY_TYPE_NONE => BiometricKind::None,
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
        "local-auth: macOS answers evaluatePolicy through a reply block, and the \
         prompt it shows is drawn on the main thread — waiting for the block \
         from here would deadlock. Reason received: {reason:?}. Call \
         LAContext.evaluatePolicy from the app's AppKit layer. \
         available_biometric() does work and reports the real modality."
    ))
}
