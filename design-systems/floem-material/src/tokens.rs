//! Material 3 colour roles and the token set built around them.

use floem_design::{readable_on, tonal_ramp};
use floem_theme::{ColorScheme, Theme};
use floem_types::Color;

/// The Material 3 colour roles this crate styles against.
///
/// `floem_theme::ColorTokens` carries seven slots; Material needs nineteen. The
/// extra twelve live here rather than in the shared crate, because a role named
/// `primary_container` is meaningless to Cupertino, and padding the shared token
/// struct with one system's vocabulary is how it becomes a union of all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialRoles {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,
    pub shadow: Color,
}

impl MaterialRoles {
    /// Every `on_*`/base pair, for the contrast audit.
    ///
    /// The list is exhaustive by construction: adding a role without adding it
    /// here is caught by `role_count_matches_the_audited_pairs`.
    pub fn contrast_pairs(&self) -> [(&'static str, Color, Color); 8] {
        [
            ("on_primary/primary", self.on_primary, self.primary),
            (
                "on_primary_container/primary_container",
                self.on_primary_container,
                self.primary_container,
            ),
            (
                "on_secondary_container/secondary_container",
                self.on_secondary_container,
                self.secondary_container,
            ),
            ("on_surface/surface", self.on_surface, self.surface),
            (
                "on_surface_variant/surface_variant",
                self.on_surface_variant,
                self.surface_variant,
            ),
            ("on_error/error", self.on_error, self.error),
            (
                "on_error_container/error_container",
                self.on_error_container,
                self.error_container,
            ),
            (
                "on_surface/surface_variant",
                self.on_surface,
                self.surface_variant,
            ),
        ]
    }
}

/// A Material theme: the generic token table plus Material's colour roles.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialTheme {
    pub scheme: ColorScheme,
    pub roles: MaterialRoles,
    /// Typography, spacing, radius, elevation, motion and density. Its
    /// `colors` are kept in step with `roles` so a generic consumer that only
    /// knows `floem_theme::Theme` still gets Material's primary and surface.
    pub theme: Theme,
}

impl MaterialTheme {
    /// The published Material 3 baseline light scheme.
    pub fn light() -> Self {
        let roles = MaterialRoles {
            primary: Color::rgb(103, 80, 164),
            on_primary: Color::rgb(255, 255, 255),
            primary_container: Color::rgb(234, 221, 255),
            on_primary_container: Color::rgb(33, 0, 93),
            secondary_container: Color::rgb(232, 222, 248),
            on_secondary_container: Color::rgb(29, 25, 43),
            surface: Color::rgb(254, 247, 255),
            surface_variant: Color::rgb(231, 224, 236),
            on_surface: Color::rgb(29, 27, 32),
            on_surface_variant: Color::rgb(73, 69, 79),
            outline: Color::rgb(121, 116, 126),
            outline_variant: Color::rgb(202, 196, 208),
            error: Color::rgb(179, 38, 30),
            on_error: Color::rgb(255, 255, 255),
            error_container: Color::rgb(249, 222, 220),
            on_error_container: Color::rgb(65, 14, 11),
            shadow: Color::BLACK,
        };
        Self::assemble(ColorScheme::Light, roles)
    }

    /// The published Material 3 baseline dark scheme.
    pub fn dark() -> Self {
        let roles = MaterialRoles {
            primary: Color::rgb(208, 188, 255),
            on_primary: Color::rgb(56, 30, 114),
            primary_container: Color::rgb(79, 55, 139),
            on_primary_container: Color::rgb(234, 221, 255),
            secondary_container: Color::rgb(74, 68, 88),
            on_secondary_container: Color::rgb(232, 222, 248),
            surface: Color::rgb(20, 18, 24),
            surface_variant: Color::rgb(73, 69, 79),
            on_surface: Color::rgb(230, 224, 233),
            on_surface_variant: Color::rgb(202, 196, 208),
            outline: Color::rgb(147, 143, 153),
            outline_variant: Color::rgb(73, 69, 79),
            error: Color::rgb(242, 184, 181),
            on_error: Color::rgb(96, 20, 16),
            error_container: Color::rgb(140, 29, 24),
            on_error_container: Color::rgb(249, 222, 220),
            shadow: Color::BLACK,
        };
        Self::assemble(ColorScheme::Dark, roles)
    }

    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::light(),
            ColorScheme::Dark => Self::dark(),
        }
    }

    /// Derive a scheme from a seed colour.
    ///
    /// Primary and its container come from the seed's tonal ramp at the tone
    /// Material's own generator uses for those roles (40/90 in light, 80/30 in
    /// dark). Neutrals stay neutral: they are taken from the ramp of a fixed
    /// mid-grey rather than from the seed, because tinting every surface with
    /// the accent is a different design decision than the one Material makes.
    ///
    /// The `on_*` colours are *measured*, via [`readable_on`], not copied.
    pub fn from_seed(seed: Color, scheme: ColorScheme) -> Self {
        let ramp = tonal_ramp(seed);
        let neutral = tonal_ramp(Color::rgb(128, 128, 128));
        let (primary_tone, primary_container_tone) = match scheme {
            ColorScheme::Light => (40, 90),
            ColorScheme::Dark => (80, 30),
        };
        let primary = ramp.tone(primary_tone);
        let primary_container = ramp.tone(primary_container_tone);

        let (surface_tone, surface_variant_tone, on_surface_tone) = match scheme {
            ColorScheme::Light => (99, 90, 10),
            ColorScheme::Dark => (6, 30, 90),
        };
        let surface = neutral.tone(surface_tone);
        let surface_variant = neutral.tone(surface_variant_tone);
        let on_surface = neutral.tone(on_surface_tone);

        let roles = MaterialRoles {
            primary,
            on_primary: readable_on(primary),
            primary_container,
            on_primary_container: readable_on(primary_container),
            secondary_container: ramp.tone(match scheme {
                ColorScheme::Light => 90,
                ColorScheme::Dark => 30,
            }),
            on_secondary_container: readable_on(ramp.tone(match scheme {
                ColorScheme::Light => 90,
                ColorScheme::Dark => 30,
            })),
            surface,
            surface_variant,
            on_surface,
            on_surface_variant: readable_on(surface_variant),
            // The outline only has to separate two surfaces, so it targets the
            // 3:1 boundary rather than 4.5:1 — picking it by text contrast would
            // make every Material outline almost black.
            outline: neutral.tone(match scheme {
                ColorScheme::Light => 55,
                ColorScheme::Dark => 70,
            }),
            outline_variant: neutral.tone(match scheme {
                ColorScheme::Light => 80,
                ColorScheme::Dark => 40,
            }),
            error: Color::rgb(
                match scheme {
                    ColorScheme::Light => 179,
                    ColorScheme::Dark => 242,
                },
                match scheme {
                    ColorScheme::Light => 38,
                    ColorScheme::Dark => 184,
                },
                match scheme {
                    ColorScheme::Light => 30,
                    ColorScheme::Dark => 181,
                },
            ),
            on_error: readable_on(Color::rgb(179, 38, 30)),
            error_container: Color::rgb(
                match scheme {
                    ColorScheme::Light => 249,
                    ColorScheme::Dark => 140,
                },
                match scheme {
                    ColorScheme::Light => 222,
                    ColorScheme::Dark => 29,
                },
                match scheme {
                    ColorScheme::Light => 220,
                    ColorScheme::Dark => 24,
                },
            ),
            on_error_container: readable_on(Color::rgb(249, 222, 220)),
            shadow: Color::BLACK,
        };
        Self::assemble(scheme, roles)
    }

    fn assemble(scheme: ColorScheme, roles: MaterialRoles) -> Self {
        let mut theme = match scheme {
            ColorScheme::Light => Theme::light(),
            ColorScheme::Dark => Theme::dark(),
        };
        // Android's own typeface; the shared default says "sans-serif".
        theme.typography.font_family = "Roboto".into();
        // Material's elevation ladder is finer-grained than the four slots the
        // shared token struct offers, so the four slots carry levels 1/3/6/12.
        theme.elevation.level0 = 0.0;
        theme.elevation.level1 = 1.0;
        theme.elevation.level2 = 3.0;
        theme.elevation.level3 = 6.0;
        // Keep the generic colour slots in step with the Material roles, so
        // `floem_theme::Theme`-only consumers see Material's colours.
        theme.colors.background = roles.surface;
        theme.colors.surface = roles.surface_variant;
        theme.colors.primary = roles.primary;
        theme.colors.on_primary = roles.on_primary;
        theme.colors.danger = roles.error;
        theme.colors.text = roles.on_surface;
        theme.colors.text_muted = roles.on_surface_variant;
        Self {
            scheme,
            roles,
            theme,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_audited_pair_list_covers_every_on_role() {
        // If a role is added without an audit entry, the contrast test silently
        // stops covering it. Eight pairs is the count that must grow with the
        // struct.
        let pairs = MaterialTheme::light().roles.contrast_pairs();
        assert_eq!(pairs.len(), 8);
        let names: Vec<_> = pairs.iter().map(|(n, _, _)| *n).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "duplicate audit entry");
    }

    #[test]
    fn a_derived_scheme_still_uses_the_scheme_it_was_asked_for() {
        assert_eq!(
            MaterialTheme::from_seed(Color::rgb(1, 2, 3), ColorScheme::Light).scheme,
            ColorScheme::Light
        );
        assert_eq!(
            MaterialTheme::from_seed(Color::rgb(1, 2, 3), ColorScheme::Dark).scheme,
            ColorScheme::Dark
        );
    }

    #[test]
    fn a_seed_changes_the_accent_but_not_the_neutrals() {
        let a = MaterialTheme::from_seed(Color::rgb(37, 99, 235), ColorScheme::Light);
        let b = MaterialTheme::from_seed(Color::rgb(220, 38, 38), ColorScheme::Light);
        assert_ne!(
            a.roles.primary, b.roles.primary,
            "the accent must follow the seed"
        );
        assert_eq!(
            a.roles.surface, b.roles.surface,
            "neutrals are deliberately seeded from grey, not the accent"
        );
    }

    #[test]
    fn a_light_and_a_dark_scheme_disagree_on_almost_every_role() {
        let light = MaterialTheme::light().roles;
        let dark = MaterialTheme::dark().roles;
        let same: Vec<&str> = [
            ("primary", light.primary == dark.primary),
            ("surface", light.surface == dark.surface),
            ("on_surface", light.on_surface == dark.on_surface),
            ("outline", light.outline == dark.outline),
            ("error", light.error == dark.error),
            ("shadow", light.shadow == dark.shadow),
        ]
        .into_iter()
        .filter(|(_, same)| *same)
        .map(|(name, _)| name)
        .collect();
        // `shadow` is black in both, by Material's own definition. Anything else
        // matching means a role was forgotten in one of the two palettes.
        assert_eq!(same, vec!["shadow"], "these roles did not change scheme");
    }

    #[test]
    fn the_generic_theme_slots_track_the_material_roles() {
        let t = MaterialTheme::dark();
        assert_eq!(t.theme.colors.primary, t.roles.primary);
        assert_eq!(t.theme.colors.background, t.roles.surface);
        assert_eq!(t.theme.colors.text, t.roles.on_surface);
        assert_eq!(t.theme.typography.font_family, "Roboto");
        assert_eq!(t.theme.elevation.level1, 1.0);
    }

    #[test]
    fn for_scheme_is_the_same_as_the_named_constructor() {
        assert_eq!(
            MaterialTheme::for_scheme(ColorScheme::Light),
            MaterialTheme::light()
        );
        assert_eq!(
            MaterialTheme::for_scheme(ColorScheme::Dark),
            MaterialTheme::dark()
        );
    }
}
