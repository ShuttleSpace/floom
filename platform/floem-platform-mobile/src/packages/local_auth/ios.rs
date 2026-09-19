//! iOS local auth via `LocalAuthentication`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::BiometricKind;

pub fn available_biometric() -> BiometricKind {
    // TODO: `LAContext.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics)`
    // and inspect `biometryType` (.faceID -> Face, .touchID -> Fingerprint).
    BiometricKind::None
}

pub fn authenticate(_reason: &str) -> Result<(), String> {
    // TODO: `LAContext.evaluatePolicy(...)` async via the reply block.
    Err("local authentication requires an iOS prompt".into())
}
