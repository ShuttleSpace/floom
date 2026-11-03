//! iOS safe-area insets.
//!
//! TODO(floom): read the real insets from the `UIWindow` / view's
//! `safeAreaInsets` via objc2, in the actual `floom-platform-ios` window
//! backend which owns the `UIWindow` / `UIView` context. Until then we return
//! zero insets.

use floom_types::Insets;

pub fn safe_area_insets() -> Insets {
    Insets::ZERO
}
