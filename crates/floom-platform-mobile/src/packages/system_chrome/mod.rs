//! System chrome styling — status bar and navigation bar appearance.
//!
//! Mobile apps can tint the OS-drawn bars (status bar icons, navigation bar
//! background) to match their theme instead of leaving them default black.
//! This package describes the desired style as a [`SystemChromeStyle`] and a
//! `#[cfg]`-dispatched [`set_system_chrome_style`] that applies it on
//! iOS/Android and refuses on the host.
//!
//! # Ported from
//!
//! Adapted from `gpui-mobile` (`SystemChromeStyle`), used under its Apache-2.0
//! option.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use floem_types::Color;

/// Brightness of the icons drawn inside a system bar (status bar / nav bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarIconBrightness {
    /// Dark icons — use on a light bar background.
    Dark,
    /// Light icons — use on a dark bar background.
    #[default]
    Light,
}

/// Describes the appearance of the OS system bars.
///
/// `None` for a color means "leave that bar at its system default".
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SystemChromeStyle {
    /// Status bar background color (`None` = default).
    pub status_bar_color: Option<Color>,
    /// Navigation bar background color (`None` = default).
    pub navigation_bar_color: Option<Color>,
    /// Icon brightness for the status bar.
    pub status_bar_icon_brightness: BarIconBrightness,
    /// Icon brightness for the navigation bar.
    pub navigation_bar_icon_brightness: BarIconBrightness,
}

/// Apply the given system chrome style.
///
/// Returns `Err` on the host and on platforms/SDK levels that cannot apply it.
pub fn set_system_chrome_style(style: SystemChromeStyle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::set_system_chrome_style(style)
    }
    #[cfg(target_os = "ios")]
    {
        ios::set_system_chrome_style(style)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = style;
        Err("system chrome styling is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_refuses_style() {
        let style = SystemChromeStyle {
            status_bar_color: Some(Color::BLACK),
            ..Default::default()
        };
        assert!(set_system_chrome_style(style).is_err());
    }

    #[test]
    fn default_style_is_inert_data() {
        let style = SystemChromeStyle::default();
        assert_eq!(style.status_bar_color, None);
        assert_eq!(style.navigation_bar_color, None);
        assert_eq!(style.status_bar_icon_brightness, BarIconBrightness::Light);
    }
}
