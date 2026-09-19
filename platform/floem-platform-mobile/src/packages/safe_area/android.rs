//! Android safe-area insets.
//!
//! TODO(floem): read the real insets from `WindowInsets` (system gesture
//! insets + display cutout) via the JNI bridge owned by the actual
//! `floem-platform-android` window backend, which holds the `Activity` /
//! `Window` context. Until then we return zero insets.

use floem_types::Insets;

pub fn safe_area_insets() -> Insets {
    Insets::ZERO
}
