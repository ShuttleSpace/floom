//! Android local auth via `BiometricPrompt`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::BiometricKind;

pub fn available_biometric() -> BiometricKind {
    // TODO: `BiometricManager.canAuthenticate()` and map
    // `BIOMETRIC_STRONG` / `BIOMETRIC_WEAK` to `Face` / `Fingerprint`.
    BiometricKind::None
}

pub fn authenticate(_reason: &str) -> Result<(), String> {
    // TODO: build a `BiometricPrompt` from the activity and surface the
    // result through a callback. Not satisfiable synchronously here.
    Err("local authentication requires an Android activity prompt".into())
}
