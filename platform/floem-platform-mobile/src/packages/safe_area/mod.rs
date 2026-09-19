//! Safe-area insets — keep content clear of system UI.
//!
//! Mobile screens reserve regions for the OS: the status bar, camera notch,
//! home indicator, and navigation bar. Apps must avoid drawing under them.
//! This package exposes those insets as a [`floem_types::Insets`] and a
//! `#[cfg]`-dispatched [`safe_area_insets`] that returns the live values on
//! iOS/Android and [`Insets::ZERO`] on the host.
//!
//! The native arms live in `android.rs` / `ios.rs`; the real values come from
//! the platform window backend (`floem-platform-{android,ios}`), which owns
//! the `UIWindow` / `ANativeWindow` context needed to read them.
//!
//! # Ported from
//!
//! Adapted from `gpui-mobile` (`ios/window.rs::safe_area_insets`), used under
//! its Apache-2.0 option.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use floem_types::Insets;

/// Current safe-area insets in logical pixels: `(top, right, bottom, left)` of
/// the region the OS reserves for system chrome. Content laid out inside these
/// insets is never covered.
pub fn safe_area_insets() -> Insets {
    #[cfg(target_os = "android")]
    {
        android::safe_area_insets()
    }
    #[cfg(target_os = "ios")]
    {
        ios::safe_area_insets()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Insets::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_returns_zero_insets() {
        // On a non-mobile host build the safe area is unknown → zero insets
        // (content is free to use the full screen).
        let insets = safe_area_insets();
        assert_eq!(insets, Insets::ZERO);
    }
}
