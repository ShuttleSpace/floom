//! Device identity and environment facts (model, OS, locale, …).
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `UIDevice` (iOS) and `Build` / `SystemProperty` (Android). The host arm
//! returns an empty [`DeviceInfo`] so the crate stays a no-op on desktop/web.
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

/// Static-ish facts about the device the app is running on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceInfo {
    /// Marketing/model name, e.g. `"iPhone15,2"` or `"Pixel 7"`.
    pub model: String,
    /// Hardware manufacturer, e.g. `"Apple"` or `"Google"`.
    pub manufacturer: String,
    /// OS name, e.g. `"iOS"` or `"Android"`.
    pub os_name: String,
    /// OS version string, e.g. `"17.4"` or `"14"`.
    pub os_version: String,
    /// SDK / API level where meaningful (`None` on iOS).
    pub sdk_version: Option<i32>,
    /// `true` when running on real silicon, `false` on a simulator/emulator.
    pub is_physical_device: bool,
    /// BCP-47 locale, e.g. `"en-US"`.
    pub locale: String,
}

/// Snapshot the current device facts.
pub fn device_info() -> DeviceInfo {
    #[cfg(target_os = "android")]
    {
        android::device_info()
    }
    #[cfg(target_os = "ios")]
    {
        ios::device_info()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        DeviceInfo::default()
    }
}

/// A short, human-readable device name (e.g. the user's phone name).
pub fn device_name() -> String {
    #[cfg(target_os = "android")]
    {
        android::device_name()
    }
    #[cfg(target_os = "ios")]
    {
        ios::device_name()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_returns_default_info() {
        let info = device_info();
        assert_eq!(info, DeviceInfo::default());
        assert!(info.model.is_empty());
    }

    #[test]
    fn host_returns_unknown_name() {
        assert_eq!(device_name(), "unknown");
    }
}
