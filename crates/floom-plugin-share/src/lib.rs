//! Share sheet — hand text or files to other apps.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms present
//! `UIActivityViewController` (iOS) and an `Intent.ACTION_SEND` /
//! `ACTION_SEND_MULTIPLE` chooser (Android). The host arm refuses.
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
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = text;
        Err("share is only available on iOS and Android".into())
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
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = paths;
        Err("share is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_share_refused() {
        assert!(share_text("hello").is_err());
        assert!(share_files(&["/tmp/a.txt"]).is_err());
        assert!(share_files(&[]).is_err());
    }
}
