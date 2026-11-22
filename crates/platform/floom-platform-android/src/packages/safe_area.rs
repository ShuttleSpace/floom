// SPDX-License-Identifier: MIT
//! Android safe-area insets — the OS-reserved region of the window.
//!
//! The pure `Insets` shape is `floom_types::Insets`; only the Android read
//! lives here. Host builds return zero insets.

#[cfg(target_os = "android")]
mod android;

use floom_types::Insets;

/// Current safe-area insets in logical pixels.
pub fn safe_area_insets() -> Insets {
    #[cfg(target_os = "android")]
    {
        android::safe_area_insets()
    }
    #[cfg(not(target_os = "android"))]
    {
        Insets::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_returns_zero_insets() {
        assert_eq!(safe_area_insets(), Insets::ZERO);
    }
}
