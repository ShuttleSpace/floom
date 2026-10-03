//! Color manipulation helpers shared by every design-system component.
//!
//! These keep visual logic token-driven and avoid hard-coded rgba literals
//! in component code (per the project's token-first rule).

use crate::tokens::color::Color;

pub const WHITE: Color = Color::rgb(255, 255, 255);
pub const BLACK: Color = Color::rgb(0, 0, 0);

/// Return `color` with the given alpha (0..=255).
pub fn with_alpha(color: Color, alpha: u8) -> Color {
    Color { a: alpha, ..color }
}

/// Linear interpolation between two colors. `t` is clamped to `0..=1`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
        a: lerp(a.a, b.a),
    }
}

/// Lighten toward white by `amt` (0..=1).
pub fn lighten(color: Color, amt: f32) -> Color {
    mix(color, WHITE, amt)
}

/// Darken toward black by `amt` (0..=1).
pub fn darken(color: Color, amt: f32) -> Color {
    mix(color, BLACK, amt)
}

/// Perceptual luminance in `0..=1` (sRGB approximation).
pub fn luminance(color: Color) -> f32 {
    let to_lin = |c: u8| {
        let s = c as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * to_lin(color.r) + 0.7152 * to_lin(color.g) + 0.0722 * to_lin(color.b)
}

/// Pick a readable foreground (black or white) for a given background.
pub fn contrast(color: Color) -> Color {
    if luminance(color) > 0.5 {
        BLACK
    } else {
        WHITE
    }
}
