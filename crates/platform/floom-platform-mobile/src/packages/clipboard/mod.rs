// SPDX-License-Identifier: MIT
//! Clipboard — read and write the system clipboard.
//!
//! Unlike the rest of the device-query packages, this one is not optional
//! polish: Floom's own [`floom_platform::Clipboard`] trait is implemented per
//! backend, and on Android there is **no NDK clipboard API at all** — the
//! clipboard is reachable only through `android.content.ClipboardManager`.
//! So the Android arm here is real JNI, and it is what
//! `floom-platform-android` calls to satisfy that trait.
//!
//! Both functions are best-effort in the same way the OS is: Android 10+
//! restricts *reading* the clipboard to the focused app, and an empty clipboard
//! is a normal state rather than an error. Hence `Ok(None)` rather than `Err`
//! for "there is no text to read", and `Err` reserved for a broken bridge.
//!
//! # Ported from
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! [`floom_platform::Clipboard`]: https://docs.rs/floom-platform

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// The clipboard's current text.
///
/// `Ok(None)` means "the clipboard holds no text" (empty, or a non-text clip
/// such as an image or a URI) — not a failure. `Err` means the bridge itself
/// could not be used.
pub fn get_text() -> Result<Option<String>, String> {
    #[cfg(target_os = "android")]
    {
        android::get_text()
    }
    #[cfg(target_os = "ios")]
    {
        ios::get_text()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Err("clipboard is only available on iOS and Android".into())
    }
}

/// Replace the clipboard's text.
pub fn set_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::set_text(text)
    }
    #[cfg(target_os = "ios")]
    {
        ios::set_text(text)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = text;
        Err("clipboard is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_clipboard_refused() {
        // On a non-mobile host the capability is absent, and it must say so
        // rather than pretend the clipboard is empty: `Ok(None)` from
        // `get_text` would be indistinguishable from a real empty clipboard,
        // and a caller could not tell "unsupported" from "nothing copied".
        assert!(get_text().is_err());
        assert!(set_text("hello").is_err());
    }
}
