//! The running application's own identity.
//!
//! `AppInfo` answers "which app am I, and which version of it is installed" —
//! the display name, the bundle identifier / application id, the version and
//! the build number. An app uses it for an about screen, for a user-agent
//! string, or to gate a migration on "the data was written by 1.3".
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # Why every field is an `Option`
//!
//! The platform does not always have an answer, and the caller has to be able
//! to tell that apart from an empty one. macOS reads the bundle it is running
//! inside; a bare executable is not in a bundle and has no `Info.plist`, so
//! every field is `None` there. Linux has no application-package concept at
//! all, and the Windows version resource is a deliberate follow-up rather than
//! something this crate pretends to read.
//!
//! A `String::new()` default would be the mistake the battery plugin documents:
//! an empty string and a real-but-empty value are indistinguishable, so a
//! caller formatting `"app {}"` would print `"app "` and believe it.
//!
//! # Why the host arm is not a stub on every desktop
//!
//! On macOS the *host* is the real source: `NSBundle.mainBundle` is exactly
//! what `package_info_plus` reads on that platform. So this plugin's Apple arm
//! is compiled by an ordinary `cargo test` on a Mac, unlike the nine
//! mobile-only plugins whose native arms no host build ever reaches. That is
//! why the arm lives in `apple.rs` rather than being split into `ios.rs` and
//! `macos.rs`: the code is the same, and keeping it in one file keeps it in the
//! one place a host test covers.

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;
#[cfg(target_env = "ohos")]
mod ohos;

/// The identity of the running application.
///
/// Each field is `None` when this platform (or this build) has nothing to
/// report, never `Some("")`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppInfo {
    /// The user-visible name: `CFBundleDisplayName` on Apple platforms, the
    /// launcher label on Android.
    pub name: Option<String>,
    /// The stable identifier: `CFBundleIdentifier` (`com.example.app`) or the
    /// Android application id.
    pub package_name: Option<String>,
    /// The marketing version: `CFBundleShortVersionString` / `versionName`.
    pub version: Option<String>,
    /// The build number: `CFBundleVersion` / `versionCode`.
    ///
    /// Kept as a string because Android's is an integer and Apple's is a
    /// dotted string (`"42.1"`); comparing them as numbers would be wrong on at
    /// least one of the two.
    pub build_number: Option<String>,
}

/// Read the running application's identity.
///
/// Never fails: "this platform has no answer" is `AppInfo::default()`, which is
/// what the type is for. A caller that needs to know whether it got anything
/// can test the fields.
pub fn app_info() -> AppInfo {
    #[cfg(target_os = "android")]
    {
        android::app_info()
    }
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        apple::app_info()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::app_info()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        AppInfo::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_measured_is_all_none() {
        // The default is the honest "no answer" value. If it ever grew a
        // plausible-looking string, a caller could not tell a desktop build
        // from an app that genuinely reported nothing.
        assert_eq!(
            AppInfo::default(),
            AppInfo {
                name: None,
                package_name: None,
                version: None,
                build_number: None,
            }
        );
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[test]
    fn a_reported_field_is_never_the_empty_string() {
        // `cargo test`'s binary is not inside an `.app`, so on this machine the
        // expected answer is all-`None`. The invariant that must hold on a
        // bundled app too is the one the type promises: a field is either
        // absent or has something in it.
        let info = app_info();
        for (field, value) in [
            ("name", &info.name),
            ("package_name", &info.package_name),
            ("version", &info.version),
            ("build_number", &info.build_number),
        ] {
            if let Some(value) = value {
                assert!(
                    !value.is_empty(),
                    "{field} was reported as an empty string, which is a value nobody measured"
                );
            }
        }
    }
}
