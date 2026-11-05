// SPDX-License-Identifier: MIT
//! iOS system chrome styling.
//!
//! TODO(floom): apply via `UIApplication.statusBarStyle` and
//! `UINavigationBar.standardAppearance` / `UIViewController`
//! `setNeedsStatusBarAppearanceUpdate` in the `floom-platform-ios` backend.

use super::SystemChromeStyle;

pub fn set_system_chrome_style(_style: SystemChromeStyle) -> Result<(), String> {
    Err("iOS system chrome styling not yet implemented".into())
}
