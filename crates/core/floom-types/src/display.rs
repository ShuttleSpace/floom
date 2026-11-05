// SPDX-License-Identifier: MIT
//! Display / scale metadata.

use crate::geometry::Size;

/// Logical-to-physical scale factor.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ScaleFactor(pub f64);

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

impl ScaleFactor {
    #[inline]
    pub const fn new(v: f64) -> Self {
        Self(v)
    }

    #[inline]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Per-display information available to layout/paint without OS handles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayInfo {
    pub logical_size: Size,
    pub scale_factor: ScaleFactor,
}

impl Default for DisplayInfo {
    fn default() -> Self {
        Self {
            logical_size: Size::ZERO,
            scale_factor: ScaleFactor::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_default_is_one() {
        assert_eq!(ScaleFactor::default().get(), 1.0);
    }
}
