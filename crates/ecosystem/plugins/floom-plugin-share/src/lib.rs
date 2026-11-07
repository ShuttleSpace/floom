//! Share sheet — hand text or files to other apps.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The arms present
//! `NSSharingServicePicker` (macOS), `UIActivityViewController` (iOS) and an
//! `Intent.ACTION_SEND` / `ACTION_SEND_MULTIPLE` chooser (Android). Windows
//! and Linux refuse and name the door: WinRT's `DataTransferManager` needs the
//! app's `HWND`, and Linux has no cross-desktop share API at all.
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

/// Present the system share sheet for a block of text.
pub fn share_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::share_text(text)
    }
    #[cfg(target_os = "ios")]
    {
        ios::share_text(text)
    }
    #[cfg(target_os = "macos")]
    {
        macos::share_text(text)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::share_text(text)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        let _ = text;
        Err(unsupported())
    }
}

/// Present the system share sheet for one or more local file paths.
pub fn share_files(paths: &[&str]) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::share_files(paths)
    }
    #[cfg(target_os = "ios")]
    {
        ios::share_files(paths)
    }
    #[cfg(target_os = "macos")]
    {
        macos::share_files(paths)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::share_files(paths)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        let _ = paths;
        Err(unsupported())
    }
}

/// The desktop half that is not written yet, named precisely: an arm that
/// says "unsupported" without saying what would make it unsupported is a dead end.
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos"
)))]
fn unsupported() -> String {
    if cfg!(target_os = "windows") {
        "share: Windows shares through WinRT's `DataTransferManager`, shown with \
         `IDataTransferManagerInterop::ShowShareUIForWindow` — that needs the app's \
         HWND and the WinRT `windows` crate, and this arm is not written"
            .into()
    } else {
        "share: Linux has no cross-desktop share API (xdg-desktop-portal has no \
         share interface); copy to the clipboard, or hand a URL to the open-url plugin"
            .into()
    }
}

#[cfg(all(
    test,
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_names_the_door() {
        let err = share_text("hello").unwrap_err();
        assert!(err.contains("share:"), "{err}");
        assert!(share_files(&["/tmp/a.txt"]).is_err());
    }
}
