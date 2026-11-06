use floom_theme::{ColorScheme, Theme};
use floom_types::Color;

/// The HarmonyOS colour roles this crate styles against.
///
/// The four semantic roles (`brand`, `warning`, `error`, `success`) are
/// documented separately from the text/surface roles because HarmonyOS treats
/// them as a fixed set that does not change between light and dark — only their
/// content colours do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonyColors {
    pub brand: Color,
    pub on_brand: Color,
    pub warning: Color,
    pub error: Color,
    pub success: Color,
    /// The page background.
    pub background: Color,
    /// A card or container sitting on the background.
    pub card: Color,
    /// A neutral fill for secondary controls.
    pub fill: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    /// A hairline separator, not an accessible boundary.
    pub divider: Color,
}

impl HarmonyColors {
    /// Every pair that must be legible, with the threshold it has to clear.
    ///
    /// The brand pair targets the 3:1 boundary rather than 4.5:1: the published
    /// brand blue with white content measures about 3.9:1, which is the value
    /// the guidance specifies.
    pub fn contrast_pairs(&self) -> [(&'static str, Color, Color, f64); 4] {
        [
            (
                "on_brand/brand",
                self.on_brand,
                self.brand,
                floom_design::WCAG_AA_LARGE,
            ),
            (
                "text/card",
                self.text,
                self.card,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "text_secondary/card",
                self.text_secondary,
                self.card,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "text/background",
                self.text,
                self.background,
                floom_design::WCAG_AA_NORMAL,
            ),
        ]
    }
}

/// A HarmonyOS theme: the generic token table plus Harmony's colour roles.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonyTheme {
    pub scheme: ColorScheme,
    pub colors: HarmonyColors,
    pub theme: Theme,
}

impl HarmonyTheme {
    pub fn light() -> Self {
        let colors = HarmonyColors {
            brand: Color::rgb(0, 125, 255),
            on_brand: Color::rgb(255, 255, 255),
            warning: Color::rgb(255, 117, 0),
            error: Color::rgb(232, 64, 38),
            success: Color::rgb(100, 187, 92),
            background: Color::rgb(241, 243, 245),
            card: Color::rgb(255, 255, 255),
            fill: Color::rgb(0, 0, 0),
            text: Color::rgb(24, 36, 49),
            text_secondary: Color::rgb(102, 102, 102),
            text_disabled: Color::rgb(178, 178, 178),
            divider: Color::rgb(229, 229, 229),
        };
        Self::assemble(ColorScheme::Light, colors)
    }

    pub fn dark() -> Self {
        let colors = HarmonyColors {
            brand: Color::rgb(0, 125, 255),
            on_brand: Color::rgb(255, 255, 255),
            warning: Color::rgb(255, 117, 0),
            error: Color::rgb(232, 64, 38),
            success: Color::rgb(100, 187, 92),
            background: Color::rgb(0, 0, 0),
            card: Color::rgb(32, 34, 36),
            fill: Color::rgb(255, 255, 255),
            text: Color::rgb(230, 230, 230),
            text_secondary: Color::rgb(163, 163, 163),
            text_disabled: Color::rgb(77, 77, 77),
            divider: Color::rgb(42, 42, 42),
        };
        Self::assemble(ColorScheme::Dark, colors)
    }

    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::light(),
            ColorScheme::Dark => Self::dark(),
        }
    }

    /// The neutral fill for a secondary control, as an opaque colour.
    ///
    /// `fill` is a *base* colour (black in light, white in dark) that HarmonyOS
    /// applies at low opacity. Materialising it here keeps the state-layer maths
    /// in one place instead of at each call site.
    pub fn neutral_fill(&self) -> Color {
        floom_design::composite_over(self.colors.card, self.colors.fill, 0.05)
    }

    fn assemble(scheme: ColorScheme, colors: HarmonyColors) -> Self {
        let mut theme = match scheme {
            ColorScheme::Light => Theme::light(),
            ColorScheme::Dark => Theme::dark(),
        };
        theme.typography.font_family = "sans-serif".into();
        // HarmonyOS body / subtitle / title steps.
        theme.typography.size_sm = 12.0;
        theme.typography.size_md = 16.0;
        theme.typography.size_lg = 20.0;
        theme.typography.line_height = 1.5;
        // The 8vp scale, with the 4vp half-step.
        theme.spacing.xs = 4.0;
        theme.spacing.sm = 8.0;
        theme.spacing.md = 16.0;
        theme.spacing.lg = 24.0;
        theme.spacing.xl = 32.0;
        // Card and container corners. Control corners are *derived* per size by
        // `button_metrics`; these are the container radii.
        theme.radius.sm = 8.0;
        theme.radius.md = 12.0;
        theme.radius.lg = 16.0;
        theme.elevation.level0 = 0.0;
        theme.elevation.level1 = 2.0;
        theme.elevation.level2 = 4.0;
        theme.elevation.level3 = 8.0;
        theme.motion.duration_fast_ms = 150;
        theme.motion.duration_normal_ms = 250;
        theme.motion.duration_slow_ms = 400;
        theme.colors.background = colors.background;
        theme.colors.surface = colors.card;
        theme.colors.primary = colors.brand;
        theme.colors.on_primary = colors.on_brand;
        theme.colors.danger = colors.error;
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
    fn the_semantic_roles_do_not_change_between_schemes() {
        // HarmonyOS keeps its brand and status colours fixed and adapts the
        // surrounding surfaces instead. If these start differing, the crate has
        // drifted into "light/dark variants of everything", which is a different
        // design decision than the one it documents.
        let l = HarmonyTheme::light().colors;
        let d = HarmonyTheme::dark().colors;
        assert_eq!(l.brand, d.brand);
        assert_eq!(l.warning, d.warning);
        assert_eq!(l.error, d.error);
        assert_eq!(l.success, d.success);
    }

    #[test]
    fn the_surfaces_do_change_between_schemes() {
        let l = HarmonyTheme::light().colors;
        let d = HarmonyTheme::dark().colors;
        assert_ne!(l.background, d.background);
        assert_ne!(l.card, d.card);
        assert_ne!(l.text, d.text);
        assert_ne!(l.divider, d.divider);
        // The neutral fill's *base* also inverts, so one opacity rule works in
        // both schemes.
        assert_ne!(l.fill, d.fill);
    }

    #[test]
    fn the_neutral_fill_is_opaque_and_close_to_the_card() {
        // It is a 5% tint of the scheme's fill base over the card, so it must be
        // barely distinguishable from the card — a solid grey here means the
        // compositing was skipped.
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let t = HarmonyTheme::for_scheme(scheme);
            let fill = t.neutral_fill();
            assert_eq!(fill.a, 255, "a state layer result is opaque over a card");
            let delta = (fill.r as i32 - t.colors.card.r as i32).abs()
                + (fill.g as i32 - t.colors.card.g as i32).abs()
                + (fill.b as i32 - t.colors.card.b as i32).abs();
            assert!(
                delta > 0 && delta < 60,
                "{scheme:?} neutral fill is {delta} away from the card"
            );
        }
    }

    #[test]
    fn the_spacing_scale_sits_on_the_eight_vp_grid() {
        let t = HarmonyTheme::light();
        for step in [
            t.theme.spacing.xs,
            t.theme.spacing.sm,
            t.theme.spacing.md,
            t.theme.spacing.lg,
            t.theme.spacing.xl,
        ] {
            assert_eq!(step % 4.0, 0.0, "{step} is off the 8vp grid");
        }
        assert_eq!(t.theme.spacing.md, 16.0);
    }

    #[test]
    fn depth_is_available_unlike_cupertino() {
        // HarmonyOS does raise popups and cards, so the shadow ladder is real
        // here — the opposite of the iOS crate's rule.
        let t = HarmonyTheme::light();
        assert!(t.theme.elevation.level1 > 0.0);
        assert!(t.theme.elevation.level1 < t.theme.elevation.level3);
    }
}
