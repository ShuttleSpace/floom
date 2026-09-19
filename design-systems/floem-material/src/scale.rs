//! Material Design 3 scale tokens carried by `floem-material`.
//!
//! [`MaterialTheme`] (in `tokens.rs`) already owns the colour *roles* and the
//! baseline/derived schemes. This module adds the remaining MD3 token
//! dimensions that the upstream [`gpui-mobile`] project ships as a standalone
//! `theme.rs` but which `floem-material` had not yet expressed:
//!
//! * [`TypeScale`] — the display/headline/title/body/label size ladder,
//! * [`ShapeScale`] — the corner-radius ladder,
//! * [`ElevationLevel`] — the 0–5 elevation steps,
//! * [`MotionDuration`] — the standard motion durations,
//! * [`StateLayer`] — the interactive-state overlay opacities.
//!
//! These are pure values (logical pixels / milliseconds / opacities) that any
//! Material widget can read. They deliberately do *not* replace the role-based
//! colour system in `tokens.rs`; they are the other half of the token set.
//!
//! [`gpui-mobile`]: https://github.com/itsbalamurali/gpui-mobile

use floem_types::Color;

/// Convert a `0xRRGGBB` integer to a [`Color`].
///
/// MD3 palette constants are conventionally written as hex; this is the
/// canonical adapter into Floem's `u8` colour type. Alpha defaults to 255.
///
/// ```rust
/// use floem_material::hex_color;
/// use floem_types::Color;
/// assert_eq!(hex_color(0x6750A4), Color::rgb(0x67, 0x50, 0xA4));
/// ```
pub const fn hex_color(hex: u32) -> Color {
    Color::rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

/// Material Design 3 type scale sizes in logical pixels.
///
/// Follows the MD3 type scale specification:
/// <https://m3.material.io/styles/typography/type-scale-tokens>
pub struct TypeScale;

impl TypeScale {
    // ── Display ──────────────────────────────────────────────────────
    /// Display Large — 57px
    pub const DISPLAY_LARGE: f32 = 57.0;
    /// Display Medium — 45px
    pub const DISPLAY_MEDIUM: f32 = 45.0;
    /// Display Small — 36px
    pub const DISPLAY_SMALL: f32 = 36.0;

    // ── Headline ─────────────────────────────────────────────────────
    /// Headline Large — 32px
    pub const HEADLINE_LARGE: f32 = 32.0;
    /// Headline Medium — 28px
    pub const HEADLINE_MEDIUM: f32 = 28.0;
    /// Headline Small — 24px
    pub const HEADLINE_SMALL: f32 = 24.0;

    // ── Title ────────────────────────────────────────────────────────
    /// Title Large — 22px
    pub const TITLE_LARGE: f32 = 22.0;
    /// Title Medium — 16px
    pub const TITLE_MEDIUM: f32 = 16.0;
    /// Title Small — 14px
    pub const TITLE_SMALL: f32 = 14.0;

    // ── Body ─────────────────────────────────────────────────────────
    /// Body Large — 16px
    pub const BODY_LARGE: f32 = 16.0;
    /// Body Medium — 14px
    pub const BODY_MEDIUM: f32 = 14.0;
    /// Body Small — 12px
    pub const BODY_SMALL: f32 = 12.0;

    // ── Label ────────────────────────────────────────────────────────
    /// Label Large — 14px
    pub const LABEL_LARGE: f32 = 14.0;
    /// Label Medium — 12px
    pub const LABEL_MEDIUM: f32 = 12.0;
    /// Label Small — 11px
    pub const LABEL_SMALL: f32 = 11.0;
}

/// Material Design 3 shape (corner radius) tokens in logical pixels.
///
/// <https://m3.material.io/styles/shape/shape-scale-tokens>
pub struct ShapeScale;

impl ShapeScale {
    /// None — 0px (sharp corners)
    pub const NONE: f32 = 0.0;
    /// Extra Small — 4px
    pub const EXTRA_SMALL: f32 = 4.0;
    /// Small — 8px
    pub const SMALL: f32 = 8.0;
    /// Medium — 12px
    pub const MEDIUM: f32 = 12.0;
    /// Large — 16px
    pub const LARGE: f32 = 16.0;
    /// Extra Large — 28px
    pub const EXTRA_LARGE: f32 = 28.0;
    /// Full — very large value to create a pill/circle shape.
    pub const FULL: f32 = 1000.0;
}

/// Material Design 3 elevation levels.
///
/// In MD3, elevation is primarily expressed through tonal colour shifts (in
/// dark mode) or subtle shadow (in light mode), rather than heavy drop shadows.
pub struct ElevationLevel;

impl ElevationLevel {
    /// Level 0 — no elevation (flat).
    pub const LEVEL0: u8 = 0;
    /// Level 1 — low elevation (e.g. cards, text fields).
    pub const LEVEL1: u8 = 1;
    /// Level 2 — medium elevation (e.g. elevated buttons, menus).
    pub const LEVEL2: u8 = 2;
    /// Level 3 — high elevation (e.g. navigation bars, FABs).
    pub const LEVEL3: u8 = 3;
    /// Level 4 — higher elevation (e.g. app bars when scrolled).
    pub const LEVEL4: u8 = 4;
    /// Level 5 — highest elevation (e.g. dialogs).
    pub const LEVEL5: u8 = 5;
}

/// Material Design 3 motion duration tokens in milliseconds.
///
/// <https://m3.material.io/styles/motion/easing-and-duration/tokens-specs>
pub struct MotionDuration;

impl MotionDuration {
    /// Short 1 — 50ms (micro interactions)
    pub const SHORT1: u64 = 50;
    /// Short 2 — 100ms (small transitions)
    pub const SHORT2: u64 = 100;
    /// Short 3 — 150ms (selection changes)
    pub const SHORT3: u64 = 150;
    /// Short 4 — 200ms (icon changes)
    pub const SHORT4: u64 = 200;
    /// Medium 1 — 250ms (expansion)
    pub const MEDIUM1: u64 = 250;
    /// Medium 2 — 300ms (page transitions)
    pub const MEDIUM2: u64 = 300;
    /// Medium 3 — 350ms (larger transitions)
    pub const MEDIUM3: u64 = 350;
    /// Medium 4 — 400ms (complex transitions)
    pub const MEDIUM4: u64 = 400;
    /// Long 1 — 450ms (enter / appear)
    pub const LONG1: u64 = 450;
    /// Long 2 — 500ms (detailed transitions)
    pub const LONG2: u64 = 500;
    /// Long 3 — 550ms (sheet transitions)
    pub const LONG3: u64 = 550;
    /// Long 4 — 600ms (full-screen transitions)
    pub const LONG4: u64 = 600;
    /// Extra Long 1 — 700ms
    pub const EXTRA_LONG1: u64 = 700;
    /// Extra Long 2 — 800ms
    pub const EXTRA_LONG2: u64 = 800;
    /// Extra Long 3 — 900ms
    pub const EXTRA_LONG3: u64 = 900;
    /// Extra Long 4 — 1000ms
    pub const EXTRA_LONG4: u64 = 1000;
}

/// Interactive state layers as defined by MD3.
///
/// Each state has a prescribed opacity that should be applied as an overlay on
/// top of a component's base colour to communicate an interactive state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateLayer {
    /// Pointer hover state.
    Hover,
    /// Keyboard/accessibility focus state.
    Focus,
    /// Active press / touch-down state.
    Pressed,
    /// Drag state.
    Dragged,
}

impl StateLayer {
    /// Returns the opacity for a state layer overlay.
    ///
    /// | State    | Opacity |
    /// |----------|---------|
    /// | Hover    | 0.08    |
    /// | Focus    | 0.12    |
    /// | Pressed  | 0.12    |
    /// | Dragged  | 0.16    |
    pub const fn opacity(self) -> f32 {
        match self {
            StateLayer::Hover => 0.08,
            StateLayer::Focus => 0.12,
            StateLayer::Pressed => 0.12,
            StateLayer::Dragged => 0.16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_splits_channels() {
        assert_eq!(hex_color(0x6750A4), Color::rgb(0x67, 0x50, 0xA4));
        assert_eq!(hex_color(0x000000), Color::rgb(0, 0, 0));
        assert_eq!(hex_color(0xFFFFFF), Color::rgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn state_layer_opacities() {
        assert_eq!(StateLayer::Hover.opacity(), 0.08);
        assert_eq!(StateLayer::Focus.opacity(), 0.12);
        assert_eq!(StateLayer::Pressed.opacity(), 0.12);
        assert_eq!(StateLayer::Dragged.opacity(), 0.16);
    }

    #[test]
    fn type_scale_monotonic_within_groups() {
        assert!(TypeScale::DISPLAY_SMALL < TypeScale::DISPLAY_MEDIUM);
        assert!(TypeScale::BODY_SMALL < TypeScale::BODY_LARGE);
    }
}
