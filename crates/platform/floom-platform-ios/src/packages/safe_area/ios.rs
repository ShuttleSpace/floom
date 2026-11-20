// SPDX-License-Identifier: MIT
//! iOS safe-area insets, supplied by the host through a C ABI seam.
//!
//! UIKit only publishes `UIView.safeAreaInsets` on the host's main thread with
//! a live `UIWindow`; Rust owns no `UIWindow` (it receives only a drawable), so
//! the host reads the insets after each layout and pushes them through
//! [`crate::floom_ios_set_safe_area_insets`]. Until the host does, the insets
//! are zero — the same as before the seam existed, but now observably so.

use std::sync::RwLock;

use floom_types::Insets;

static SAFE_AREA: RwLock<Insets> = RwLock::new(Insets::ZERO);

/// Host entry point: publish the current safe-area insets in logical points.
pub fn set_safe_area_insets(insets: Insets) {
    if let Ok(mut guard) = SAFE_AREA.write() {
        *guard = insets;
    }
}

pub fn safe_area_insets() -> Insets {
    SAFE_AREA.read().map(|g| *g).unwrap_or(Insets::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_push_is_read_back() {
        let insets = Insets {
            top: 44.0,
            right: 8.0,
            bottom: 34.0,
            left: 8.0,
        };
        set_safe_area_insets(insets);
        assert_eq!(safe_area_insets(), insets);
        set_safe_area_insets(Insets::ZERO);
    }
}
