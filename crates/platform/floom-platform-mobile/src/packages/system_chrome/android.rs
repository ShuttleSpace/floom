// SPDX-License-Identifier: MIT
//! Android system chrome styling.
//!
//! TODO(floom): apply via `WindowInsetsController` / `WindowCompat` in the
//! `floom-platform-android` backend (set status/navigation bar colors and the
//! `AppearanceLightStatusBars` / `AppearanceLightNavigationBars` flags).

use super::SystemChromeStyle;

pub fn set_system_chrome_style(_style: SystemChromeStyle) -> Result<(), String> {
    Err("Android system chrome styling not yet implemented".into())
}
