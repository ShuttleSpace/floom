// SPDX-License-Identifier: MIT
//! Color value type (sRGB, non-premultiplied).

/// 8-bit sRGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_constants() {
        assert_eq!(Color::default(), Color::TRANSPARENT);
        assert_eq!(Color::BLACK.a, 255);
        assert_eq!(Color::rgb(1, 2, 3), Color::rgba(1, 2, 3, 255));
    }
}
