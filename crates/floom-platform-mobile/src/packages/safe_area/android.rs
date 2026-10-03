//! Android safe-area insets.
//!
//! TODO(floom): read the real insets from `WindowInsets` (system gesture
//! insets + display cutout) via the JNI bridge owned by the actual
//! `floom-platform-android` window backend, which holds the `Activity` /
//! `Window` context. Until then we return zero insets.

use floom_types::Insets;

pub fn safe_area_insets() -> Insets {
    Insets::ZERO
}
