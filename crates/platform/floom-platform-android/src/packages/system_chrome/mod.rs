// SPDX-License-Identifier: MIT
//! Android system-bar styling.
//!
//! The style descriptor is `floom_platform::mobile::SystemChromeStyle`; only
//! the Android application arm lives here.

#[cfg(target_os = "android")]
mod android;

pub use floom_platform::mobile::{BarIconBrightness, SystemChromeStyle};

/// Apply the given system chrome style.
pub fn set_system_chrome_style(style: SystemChromeStyle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::set_system_chrome_style(style)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = style;
        Err("system chrome styling is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_types::Color;

    #[test]
    fn host_refuses_style() {
        let style = SystemChromeStyle {
            status_bar_color: Some(Color::BLACK),
            ..Default::default()
        };
        assert!(set_system_chrome_style(style).is_err());
    }
}
