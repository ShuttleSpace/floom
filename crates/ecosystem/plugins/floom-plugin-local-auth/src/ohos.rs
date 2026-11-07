//! OpenHarmony local authentication is not wired to this crate yet.

use super::BiometricKind;

pub fn available_biometric() -> BiometricKind {
    BiometricKind::None
}

pub fn authenticate(_reason: &str) -> Result<(), String> {
    Err(
        "local authentication: OpenHarmony User Authentication Kit is not wired to this crate yet"
            .into(),
    )
}
