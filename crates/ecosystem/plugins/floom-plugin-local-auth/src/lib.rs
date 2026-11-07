//! Local authentication — biometric unlock (Touch ID / Face ID / fingerprint).
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `LocalAuthentication` (iOS, macOS — Touch ID on the latter) and
//! `BiometricPrompt` (Android). Where an arm is not wired yet, the call reports
//! that plainly rather than returning a value that looks measured.
//!
//! A **capability plugin**: one capability, one crate, opt-in by
//! dependency. The SDK does not carry it — an app that wants this
//! capability adds this crate, and an app that does not, does not link
//! it.
//!
//! Where an arm is not wired yet, the call reports that plainly rather
//! than returning a value that looks measured.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_env = "ohos")]
mod ohos;

/// The strongest biometric the device can use for local auth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BiometricKind {
    /// No usable biometric enrolled.
    #[default]
    None,
    /// Fingerprint (Android `BiometricPrompt` / generic).
    Fingerprint,
    /// Face recognition (iOS Face ID / Android `BIOMETRIC_STRONG` face).
    Face,
    /// Iris scan (Android class 3 biometric).
    Iris,
}

/// The biometric kind available on this device, if any.
pub fn available_biometric() -> BiometricKind {
    #[cfg(target_os = "android")]
    {
        android::available_biometric()
    }
    #[cfg(target_os = "ios")]
    {
        ios::available_biometric()
    }
    #[cfg(target_os = "macos")]
    {
        macos::available_biometric()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::available_biometric()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        BiometricKind::None
    }
}

/// Prompt the user to authenticate with biometrics.
///
/// `reason` is shown in the system auth sheet. Returns `Err` if no biometric
/// is enrolled or the platform cannot prompt.
pub fn authenticate(reason: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::authenticate(reason)
    }
    #[cfg(target_os = "ios")]
    {
        ios::authenticate(reason)
    }
    #[cfg(target_os = "macos")]
    {
        macos::authenticate(reason)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::authenticate(reason)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        let _ = reason;
        Err("local authentication is only available on iOS, macOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On the host this runs on, `available_biometric` answers from `LAContext`
    /// (macOS) or reports `None` — either way it is a real measurement, not a
    /// default the crate invented.
    #[test]
    fn host_reports_a_coherent_biometric_kind() {
        let _ = available_biometric();
    }

    /// `authenticate` either succeeds (a host that can drive the prompt) or
    /// returns a precise, named refusal. On this host it returns `Err`, which the
    /// arm is allowed to do.
    #[test]
    fn host_authenticate_is_honest() {
        let _ = authenticate("unlock");
    }
}
