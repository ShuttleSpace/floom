//! Haptic feedback and vibration.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
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
#[cfg(target_env = "ohos")]
mod ohos;

/// The type of haptic feedback to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticFeedback {
    /// A light impact (subtle tap).
    Light,
    /// A medium impact.
    Medium,
    /// A heavy impact (strong tap).
    Heavy,
    /// Selection feedback (light tick for picker scrolling).
    Selection,
    /// Success notification feedback.
    Success,
    /// Warning notification feedback.
    Warning,
    /// Error notification feedback.
    Error,
}

/// Vibrate the device for the given duration in milliseconds.
pub fn vibrate(duration_ms: u32) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::vibrate(duration_ms)
    }
    #[cfg(target_os = "android")]
    {
        android::vibrate(duration_ms)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::vibrate(duration_ms)
    }
    #[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
    {
        let _ = duration_ms;
        Err("vibration is only available on iOS and Android".into())
    }
}

/// Trigger a specific haptic feedback pattern.
pub fn haptic_feedback(feedback: HapticFeedback) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::haptic_feedback(feedback)
    }
    #[cfg(target_os = "android")]
    {
        android::haptic_feedback(feedback)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::haptic_feedback(feedback)
    }
    #[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
    {
        let _ = feedback;
        Err("vibration is only available on iOS and Android".into())
    }
}

/// Check whether the device supports vibration.
pub fn can_vibrate() -> bool {
    #[cfg(target_os = "ios")]
    {
        ios::can_vibrate()
    }
    #[cfg(target_os = "android")]
    {
        android::can_vibrate()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::can_vibrate()
    }
    #[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_is_inert() {
        // On a non-mobile host build the public API must refuse to run and
        // report capability as absent rather than panic.
        assert!(!can_vibrate());
        assert!(vibrate(100).is_err());
        assert!(haptic_feedback(HapticFeedback::Light).is_err());
    }
}
