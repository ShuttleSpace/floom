//! Device identity and environment facts (model, OS, locale, …).
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `UIDevice` (iOS) and `Build` / `SystemProperty` (Android); the desktop arms
//! read what `std` already exposes (`/etc/os-release`, `SystemVersion.plist`,
//! the locale and host-name environment) — see `desktop.rs`. Fields with no
//! `std` door (`model`, `manufacturer` on a desktop) stay empty rather than
//! being invented.
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
// Desktop arms: macOS, Windows and Linux (Linux minus OHOS — per the OHOS
// contract in `docs/platforms.md`, OHOS is `target_os = "linux"`) share one `std`-only
// implementation. Web (`wasm32`) keeps the inert fallback because a browser tab
// has no host identity to report.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
))]
mod desktop;

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
    #[cfg(target_env = "ohos")]
    {
        ohos::device_info()
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::device_info()
    }
    #[cfg(target_arch = "wasm32")]
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
    #[cfg(target_env = "ohos")]
    {
        ohos::device_name()
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
    ))]
    {
        desktop::device_name()
    }
    #[cfg(target_arch = "wasm32")]
    {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a host the desktop arm reports what `std` can read: the OS name is
    /// always known, the device is real silicon, and every field is well-formed
    /// (never the empty string masquerading as a measured value). Fields with no
    /// `std` source stay empty by design.
    #[test]
    fn desktop_reports_what_std_can_see() {
        let info = device_info();
        assert!(!info.os_name.is_empty(), "the OS name is always known");
        assert!(info.is_physical_device, "a desktop runs on real silicon");
        // `model` and `manufacturer` have no `std` door; they stay empty rather
        // than being invented.
        assert!(info.model.is_empty());
        assert!(info.manufacturer.is_empty());
    }

    #[test]
    fn host_name_is_well_formed() {
        let name = device_name();
        assert!(!name.is_empty(), "a host always has a name");
    }
}
