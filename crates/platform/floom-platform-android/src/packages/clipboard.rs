// SPDX-License-Identifier: MIT
//! Clipboard — read and write the system clipboard.
//!
//! Android has no NDK clipboard API; the Android arm reaches
//! `android.content.ClipboardManager` through the JNI bridge in `crate::jni`.
//! Host builds refuse the operation rather than pretend an empty clipboard.

#[cfg(target_os = "android")]
mod android;

/// The clipboard's current text. `Ok(None)` means "no text to read".
pub fn get_text() -> Result<Option<String>, String> {
    #[cfg(target_os = "android")]
    {
        android::get_text()
    }
    #[cfg(not(target_os = "android"))]
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
    #[cfg(not(target_os = "android"))]
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
        assert!(get_text().is_err());
        assert!(set_text("hello").is_err());
    }
}
