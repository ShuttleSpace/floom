//! iOS system chrome styling.
//!
//! TODO(floem): apply via `UIApplication.statusBarStyle` and
//! `UINavigationBar.standardAppearance` / `UIViewController`
//! `setNeedsStatusBarAppearanceUpdate` in the `floem-platform-ios` backend.

use super::SystemChromeStyle;

pub fn set_system_chrome_style(_style: SystemChromeStyle) -> Result<(), String> {
    Err("iOS system chrome styling not yet implemented".into())
}
