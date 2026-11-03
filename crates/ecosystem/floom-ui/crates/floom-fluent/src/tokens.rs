
use floom_theme::{ColorScheme, Theme};
use floom_types::Color;

/// The Fluent colour roles this crate styles against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluentColors {
    /// The brand accent every primary control is drawn in.
    pub accent: Color,
    /// The content colour on top of `accent`.
    ///
    /// White in the light scheme, black in the dark one. That is Fluent's own
    /// choice, not a contrast calculation: the dark accent is a *light* blue, so
    /// black content clears AA on it where white only just clears the 3:1
    /// boundary.
    pub on_accent: Color,
    /// The default control surface.
    pub layer: Color,
    /// The hover surface, one step off `layer`.
    pub layer_alt: Color,
    /// The surface a raised element sits on.
    pub card_background: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    /// The 1px boundary on a standard control.
    ///
    /// Deliberately subtle — Fluent's default stroke is a hairline separation,
    /// not an accessible boundary, because the control's own fill already
    /// conveys its extent. The audit holds it to a *visibility* floor rather
    /// than to WCAG's 3:1 for essential boundaries.
    pub stroke_default: Color,
    /// The focus ring, drawn in two strokes by Fluent; this is the inner one.
    pub focus_ring: Color,
}

impl FluentColors {
    /// Every pair that must be legible, with the threshold it has to clear.
    pub fn contrast_pairs(&self) -> [(&'static str, Color, Color, f64); 5] {
        [
            (
                "on_accent/accent",
                self.on_accent,
                self.accent,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "text/card_background",
                self.text,
                self.card_background,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "text/layer",
                self.text,
                self.layer,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "text_secondary/layer",
                self.text_secondary,
                self.layer,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "focus_ring/card_background",
                self.focus_ring,
                self.card_background,
                floom_design::WCAG_AA_LARGE,
            ),
        ]
    }

    /// The stroke's own floor: it must be *perceptible* against the card, with
    /// no claim to be an accessible boundary.
    pub const STROKE_VISIBILITY_FLOOR: f64 = 1.2;
}

/// A Fluent theme: the generic token table plus Fluent's colour roles.
#[derive(Debug, Clone, PartialEq)]
pub struct FluentTheme {
    pub scheme: ColorScheme,
    pub colors: FluentColors,
    pub theme: Theme,
}

impl FluentTheme {
    pub fn light() -> Self {
        let colors = FluentColors {
            accent: Color::rgb(0, 120, 212),
            on_accent: Color::rgb(255, 255, 255),
            layer: Color::rgb(243, 243, 243),
            layer_alt: Color::rgb(235, 235, 235),
            card_background: Color::rgb(255, 255, 255),
            text: Color::rgb(36, 36, 36),
            text_secondary: Color::rgb(97, 97, 97),
            text_disabled: Color::rgb(189, 189, 189),
            stroke_default: Color::rgb(209, 209, 209),
            focus_ring: Color::rgb(0, 0, 0),
        };
        Self::assemble(ColorScheme::Light, colors)
    }

    pub fn dark() -> Self {
        let colors = FluentColors {
            accent: Color::rgb(40, 153, 245),
            on_accent: Color::rgb(0, 0, 0),
            layer: Color::rgb(32, 32, 32),
            layer_alt: Color::rgb(43, 43, 43),
            card_background: Color::rgb(28, 28, 28),
            text: Color::rgb(255, 255, 255),
            text_secondary: Color::rgb(214, 214, 214),
            text_disabled: Color::rgb(110, 110, 110),
            stroke_default: Color::rgb(58, 58, 58),
            focus_ring: Color::rgb(255, 255, 255),
        };
        Self::assemble(ColorScheme::Dark, colors)
    }

    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::light(),
            ColorScheme::Dark => Self::dark(),
        }
    }

    fn assemble(scheme: ColorScheme, colors: FluentColors) -> Self {
        let mut theme = match scheme {
            ColorScheme::Light => Theme::light(),
            ColorScheme::Dark => Theme::dark(),
        };
        theme.typography.font_family = "Segoe UI".into();
        // Fluent's caption / body / title steps.
        theme.typography.size_sm = 12.0;
        theme.typography.size_md = 14.0;
        theme.typography.size_lg = 20.0;
        theme.typography.line_height = 1.43;
        theme.spacing.xs = 4.0;
        theme.spacing.sm = 8.0;
        theme.spacing.md = 12.0;
        theme.spacing.lg = 16.0;
        theme.spacing.xl = 24.0;
        theme.radius.sm = 2.0;
        theme.radius.md = 4.0;
        theme.radius.lg = 8.0;
        // Fluent's shadow ladder, at the four steps the shared token struct has.
        theme.elevation.level0 = 0.0;
        theme.elevation.level1 = 2.0;
        theme.elevation.level2 = 4.0;
        theme.elevation.level3 = 8.0;
        theme.motion.duration_fast_ms = 100;
        theme.motion.duration_normal_ms = 200;
        theme.motion.duration_slow_ms = 300;
        theme.colors.background = colors.card_background;
        theme.colors.surface = colors.layer;
        theme.colors.primary = colors.accent;
        theme.colors.on_primary = colors.on_accent;
        theme.colors.danger = Color::rgb(196, 43, 28);
        theme.colors.text = colors.text;
        theme.colors.text_muted = colors.text_secondary;
        Self {
            scheme,
            colors,
            theme,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_neutral_ramp_is_three_distinct_opaque_layers() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let c = FluentTheme::for_scheme(scheme).colors;
            assert_eq!(c.layer.a, 255, "a Fluent layer is opaque");
            assert_eq!(c.layer_alt.a, 255);
            assert_eq!(c.card_background.a, 255);
            assert_ne!(c.layer, c.layer_alt, "hover must be a separate layer");
            assert_ne!(c.layer, c.card_background);
        }
    }

    #[test]
    fn the_stroke_is_a_hairline_not_an_accessible_boundary() {
        // Pin the documented property rather than a number: the stroke is
        // visible but does not clear the 3:1 that WCAG asks of an *essential*
        // boundary. If someone "fixes" it to pass 3:1, the Fluent look is gone
        // and this test should be the thing that raises the question.
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let c = FluentTheme::for_scheme(scheme).colors;
            let ratio = floom_design::contrast_ratio(c.stroke_default, c.card_background);
            assert!(
                ratio >= FluentColors::STROKE_VISIBILITY_FLOOR,
                "{scheme:?} stroke is invisible at {ratio:.2}:1"
            );
            assert!(
                ratio < floom_design::WCAG_AA_LARGE,
                "{scheme:?} stroke now clears 3:1 — revisit the doc comment"
            );
        }
    }

    #[test]
    fn the_focus_ring_inverts_with_the_scheme() {
        // A ring has to contrast with what it surrounds; Fluent solves that by
        // flipping it black/white rather than by tinting it.
        let light = FluentTheme::light().colors;
        let dark = FluentTheme::dark().colors;
        assert!(floom_design::relative_luminance(light.focus_ring) < 0.1);
        assert!(floom_design::relative_luminance(dark.focus_ring) > 0.9);
    }

    #[test]
    fn the_accent_flips_its_content_colour_between_schemes() {
        // The dark accent is a light blue, so the content flips to black. This
        // is why `on_accent` is a token and not a `readable_on` call: the
        // light scheme's *white* is Fluent's published value and only just
        // clears AA.
        assert_eq!(
            FluentTheme::light().colors.on_accent,
            Color::rgb(255, 255, 255)
        );
        assert_eq!(FluentTheme::dark().colors.on_accent, Color::BLACK);
    }

    #[test]
    fn the_control_ladder_is_the_fluent_one() {
        let t = FluentTheme::light();
        assert_eq!(t.theme.typography.size_md, 14.0);
        assert_eq!(t.theme.radius.md, 4.0);
        assert_eq!(t.theme.elevation.level1, 2.0);
        assert_eq!(t.theme.typography.font_family, "Segoe UI");
    }
}
