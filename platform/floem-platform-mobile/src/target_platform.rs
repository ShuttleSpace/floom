//! Runtime platform detection.
//!
//! Provides a [`TargetPlatform`] enum and [`target_platform()`] so application
//! code can branch on the running OS at runtime without sprinkling `#[cfg]`
//! everywhere. Compile-time target selection still uses `#[cfg(target_os =
//! "…")]` as usual.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

/// The platform the application is currently running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    /// Android (NDK / JNI).
    Android,
    /// iOS (UIKit / Metal).
    IOS,
    /// macOS (AppKit / Metal).
    MacOS,
    /// Linux (X11 / Wayland).
    Linux,
    /// Windows (Win32 / DirectX).
    Windows,
    /// Web (wasm32 / WebGPU).
    Web,
}

impl TargetPlatform {
    /// Returns `true` if this is a mobile platform (Android or iOS).
    pub fn is_mobile(self) -> bool {
        matches!(self, Self::Android | Self::IOS)
    }

    /// Returns `true` if this is a desktop platform (macOS, Linux, or Windows).
    pub fn is_desktop(self) -> bool {
        matches!(self, Self::MacOS | Self::Linux | Self::Windows)
    }

    /// Returns `true` if this is Android.
    pub fn is_android(self) -> bool {
        self == Self::Android
    }

    /// Returns `true` if this is iOS.
    pub fn is_ios(self) -> bool {
        self == Self::IOS
    }

    /// Returns `true` if this is macOS.
    pub fn is_macos(self) -> bool {
        self == Self::MacOS
    }

    /// Returns `true` if this is an Apple platform (iOS or macOS).
    pub fn is_apple(self) -> bool {
        matches!(self, Self::IOS | Self::MacOS)
    }

    /// Returns `true` if this is a web (WASM) target.
    pub fn is_web(self) -> bool {
        self == Self::Web
    }
}

impl core::fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::Android => "Android",
            Self::IOS => "iOS",
            Self::MacOS => "macOS",
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::Web => "Web",
        };
        f.write_str(s)
    }
}

/// Returns the [`TargetPlatform`] for the current compilation target.
///
/// Resolved at compile time via `cfg` — no runtime overhead.
pub fn target_platform() -> TargetPlatform {
    #[cfg(target_os = "android")]
    {
        TargetPlatform::Android
    }
    #[cfg(target_os = "ios")]
    {
        TargetPlatform::IOS
    }
    #[cfg(target_os = "macos")]
    {
        TargetPlatform::MacOS
    }
    #[cfg(target_os = "linux")]
    {
        TargetPlatform::Linux
    }
    #[cfg(target_os = "windows")]
    {
        TargetPlatform::Windows
    }
    #[cfg(target_arch = "wasm32")]
    {
        TargetPlatform::Web
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_arch = "wasm32",
    )))]
    {
        TargetPlatform::Linux
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert!(TargetPlatform::Android.is_mobile());
        assert!(TargetPlatform::IOS.is_mobile());
        assert!(TargetPlatform::IOS.is_apple());
        assert!(!TargetPlatform::Android.is_apple());
        assert!(TargetPlatform::MacOS.is_desktop());
        assert!(!TargetPlatform::Web.is_desktop());
    }

    #[test]
    fn display_roundtrip() {
        assert_eq!(TargetPlatform::IOS.to_string(), "iOS");
        assert_eq!(TargetPlatform::Android.to_string(), "Android");
    }
}
