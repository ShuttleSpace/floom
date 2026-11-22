// SPDX-License-Identifier: MIT
//! Safe-area insets — host may set them via a C ABI seam; otherwise zero.

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "ios")]
pub use ios::set_safe_area_insets;

use floom_types::Insets;

/// Current safe-area insets in logical pixels.
pub fn safe_area_insets() -> Insets {
    #[cfg(target_os = "ios")]
    {
        ios::safe_area_insets()
    }
    #[cfg(not(target_os = "ios"))]
    {
        Insets::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_returns_zero() {
        assert_eq!(safe_area_insets(), Insets::ZERO);
    }
}
