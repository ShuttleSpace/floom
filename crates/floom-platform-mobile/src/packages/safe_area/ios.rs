//! iOS safe-area insets.
//!
//! TODO(floem): read the real insets from the `UIWindow` / view's
//! `safeAreaInsets` via objc2, in the actual `floem-platform-ios` window
//! backend which owns the `UIWindow` / `UIView` context. Until then we return
//! zero insets.

use floem_types::Insets;

pub fn safe_area_insets() -> Insets {
    Insets::ZERO
}
