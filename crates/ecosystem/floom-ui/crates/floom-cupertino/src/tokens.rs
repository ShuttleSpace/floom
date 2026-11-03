
use floom_theme::{ColorScheme, Theme};
use floom_types::Color;

/// The Cupertino colour roles this crate styles against.
///
/// Several are translucent by design — Apple's `secondaryLabel` and
/// `separatorColor` are defined as a base colour *with an alpha*, which is why
/// [`floom_types::Color`]'s four channels are enough and no separate "opacity"
/// role is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CupertinoColors {
    /// `systemBlue`: the tint every interactive control is drawn in.
    pub tint: Color,
    /// The content colour on top of `tint`.
    pub on_tint: Color,
    /// A muted fill for secondary buttons and grouped rows.
    pub fill: Color,
    pub label: Color,
    pub secondary_label: Color,
    pub tertiary_label: Color,
    /// The page background behind grouped content.
    pub grouped_background: Color,
    /// The card colour *on* the grouped background.
    pub secondary_grouped_background: Color,
    pub separator: Color,
    /// `systemRed`: destructive actions.
    pub destructive: Color,
}

impl CupertinoColors {
    /// Every pair that must be legible, with the threshold it has to clear.
    ///
    /// The threshold is part of the data because the pairs are not
    /// interchangeable: Apple's own `systemBlue` on white measures about
    /// 4.0:1, which is *below* WCAG AA for body text and above the 3:1
    /// boundary. Reproducing the platform's published value is the point of
    /// this crate, so the audit records the boundary that pairing does meet
    /// instead of quietly darkening Apple's blue until 4.5 fits.
    pub fn contrast_pairs(&self) -> [(&'static str, Color, Color, f64); 4] {
        [
            (
                "on_tint/tint",
                self.on_tint,
                self.tint,
                floom_design::WCAG_AA_LARGE,
            ),
            (
                "label/grouped_background",
                self.label,
                self.grouped_background,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "label/secondary_grouped_background",
                self.label,
                self.secondary_grouped_background,
                floom_design::WCAG_AA_NORMAL,
            ),
            (
                "tint/secondary_grouped_background",
                self.tint,
                self.secondary_grouped_background,
                floom_design::WCAG_AA_LARGE,
            ),
        ]
    }
}

/// A Cupertino theme: the generic token table plus Apple's colour roles.
#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTheme {
    pub scheme: ColorScheme,
    pub colors: CupertinoColors,
    pub theme: Theme,
}

impl CupertinoTheme {
    pub fn light() -> Self {
        let colors = CupertinoColors {
            tint: Color::rgb(0, 122, 255),
            on_tint: Color::rgb(255, 255, 255),
            fill: Color::rgba(120, 120, 128, 51),
            label: Color::rgb(0, 0, 0),
            secondary_label: Color::rgba(60, 60, 67, 153),
            tertiary_label: Color::rgba(60, 60, 67, 76),
            grouped_background: Color::rgb(242, 242, 247),
            secondary_grouped_background: Color::rgb(255, 255, 255),
            separator: Color::rgba(60, 60, 67, 73),
            destructive: Color::rgb(255, 59, 48),
        };
        Self::assemble(ColorScheme::Light, colors)
    }

    pub fn dark() -> Self {
        let colors = CupertinoColors {
            tint: Color::rgb(10, 132, 255),
            on_tint: Color::rgb(255, 255, 255),
            fill: Color::rgba(120, 120, 128, 81),
            label: Color::rgb(255, 255, 255),
            secondary_label: Color::rgba(235, 235, 245, 153),
            tertiary_label: Color::rgba(235, 235, 245, 76),
            grouped_background: Color::rgb(0, 0, 0),
            secondary_grouped_background: Color::rgb(28, 28, 30),
            separator: Color::rgba(84, 84, 88, 153),
            destructive: Color::rgb(255, 69, 58),
        };
        Self::assemble(ColorScheme::Dark, colors)
    }

    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::light(),
            ColorScheme::Dark => Self::dark(),
        }
    }

    fn assemble(scheme: ColorScheme, colors: CupertinoColors) -> Self {
        let mut theme = match scheme {
            ColorScheme::Light => Theme::light(),
            ColorScheme::Dark => Theme::dark(),
        };
        // Resolves to San Francisco on Apple platforms through the system font
        // stack; no font file is shipped or named.
        theme.typography.font_family = "system-ui".into();
        // iOS point sizes: footnote, body, title3.
        theme.typography.size_sm = 13.0;
        theme.typography.size_md = 17.0;
        theme.typography.size_lg = 20.0;
        theme.typography.line_height = 1.29;
        // The 8pt grid.
        theme.spacing.xs = 4.0;
        theme.spacing.sm = 8.0;
        theme.spacing.md = 16.0;
        theme.spacing.lg = 24.0;
        theme.spacing.xl = 32.0;
        theme.radius.sm = 6.0;
        theme.radius.md = 10.0;
        theme.radius.lg = 14.0;
        // Cupertino nests corners instead of casting shadows.
        theme.elevation.level0 = 0.0;
        theme.elevation.level1 = 0.0;
        theme.elevation.level2 = 0.0;
        theme.elevation.level3 = 0.0;
        theme.motion.duration_fast_ms = 250;
        theme.motion.duration_normal_ms = 350;
        theme.motion.duration_slow_ms = 500;
        theme.colors.background = colors.grouped_background;
        theme.colors.surface = colors.secondary_grouped_background;
        theme.colors.primary = colors.tint;
        theme.colors.on_primary = colors.on_tint;
        theme.colors.danger = colors.destructive;
        theme.colors.text = colors.label;
        theme.colors.text_muted = colors.secondary_label;
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
    fn cupertino_nests_corners_instead_of_casting_shadows() {
        // The defining departure from Material. If a later edit reintroduces a
        // shadow ladder, this is where it is caught.
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let t = CupertinoTheme::for_scheme(scheme);
            assert_eq!(t.theme.elevation.level0, 0.0);
            assert_eq!(t.theme.elevation.level1, 0.0);
            assert_eq!(t.theme.elevation.level2, 0.0);
            assert_eq!(t.theme.elevation.level3, 0.0);
        }
    }

    #[test]
    fn the_type_scale_is_ios_point_sizes_not_the_shared_default() {
        let t = CupertinoTheme::light();
        assert_eq!(t.theme.typography.size_md, 17.0, "iOS body is 17pt");
        assert_eq!(t.theme.typography.font_family, "system-ui");
        // The shared default is 14; Cupertino must not silently inherit it.
        assert_ne!(
            t.theme.typography.size_md,
            Theme::light().typography.size_md
        );
    }

    #[test]
    fn the_spacing_scale_sits_on_the_eight_point_grid() {
        let t = CupertinoTheme::light();
        for step in [
            t.theme.spacing.xs,
            t.theme.spacing.sm,
            t.theme.spacing.md,
            t.theme.spacing.lg,
            t.theme.spacing.xl,
        ] {
            assert_eq!(step % 4.0, 0.0, "{step} is off the 8pt grid");
        }
    }

    #[test]
    fn the_two_schemes_disagree_on_the_surfaces() {
        let l = CupertinoTheme::light().colors;
        let d = CupertinoTheme::dark().colors;
        assert_ne!(l.grouped_background, d.grouped_background);
        assert_ne!(l.label, d.label);
        assert_ne!(l.tint, d.tint);
        // On-tint stays white in both: that is Apple's own choice, not an
        // oversight.
        assert_eq!(l.on_tint, d.on_tint);
    }

    #[test]
    fn a_translucent_role_keeps_its_alpha() {
        // `secondaryLabel` is defined as a colour *with* alpha; flattening it to
        // opaque would break every grouped-list treatment.
        let t = CupertinoTheme::light();
        assert!(t.colors.secondary_label.a < 255);
        assert!(t.colors.tertiary_label.a < t.colors.secondary_label.a);
        assert!(t.colors.separator.a < 255);
        assert_eq!(t.colors.label.a, 255);
    }
}
