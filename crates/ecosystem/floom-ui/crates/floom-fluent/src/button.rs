use floom_design::{
    composite_over, ControlSize, InteractionState, Props, StyleSetBuilder, StyleSetExt, Variant,
};
use floom_style::{StyleProps, StyleSet};
use floom_types::Color;

use crate::tokens::FluentTheme;

/// Fluent's `subtleHover` layer: 3.74% of the content colour.
const HOVER_LAYER: f32 = 0.0374;
/// Fluent's `subtlePressed` layer: 6.98%.
const PRESSED_LAYER: f32 = 0.0698;
/// A selected control gets a deeper layer than a pressed one, because Fluent has
/// no separate selected appearance for a button.
const SELECTED_LAYER: f32 = 0.15;
/// The focus ring's stroke width, in logical px.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// A control's box metrics at one size step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonMetrics {
    pub height: f32,
    pub padding: f32,
    pub font_size: f32,
}

pub fn button_metrics(theme: &FluentTheme, size: ControlSize) -> ButtonMetrics {
    let t = &theme.theme;
    ButtonMetrics {
        height: size.pick(24.0, 32.0, 40.0),
        padding: size.pick(t.spacing.sm, t.spacing.md, t.spacing.lg),
        font_size: size.pick(
            t.typography.size_sm,
            t.typography.size_md,
            t.typography.size_lg,
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisabledStyle {
    /// The accent is replaced by a neutral layer.
    Neutralised,
    Outline,
    Bare,
}

#[derive(Debug, Clone, Copy)]
struct Rest {
    bg: Color,
    fg: Color,
    border: (f32, Color),
    elevation: f32,
    disabled: DisabledStyle,
}

fn rest(theme: &FluentTheme, variant: Variant) -> Rest {
    let c = &theme.colors;
    match variant {
        Variant::Filled => Rest {
            bg: c.accent,
            fg: c.on_accent,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Neutralised,
        },
        Variant::Tonal => Rest {
            bg: c.layer,
            fg: c.text,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Neutralised,
        },
        Variant::Outlined => Rest {
            bg: Color::TRANSPARENT,
            fg: c.text,
            border: (1.0, c.stroke_default),
            elevation: 0.0,
            disabled: DisabledStyle::Outline,
        },
        Variant::Text => Rest {
            bg: Color::TRANSPARENT,
            fg: c.text,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Bare,
        },
        Variant::Elevated => Rest {
            bg: c.accent,
            fg: c.on_accent,
            border: (0.0, Color::TRANSPARENT),
            elevation: theme.theme.elevation.level1,
            disabled: DisabledStyle::Neutralised,
        },
    }
}

/// Every state of one variant.
pub fn button_styles(theme: &FluentTheme, variant: Variant, size: ControlSize) -> StyleSet {
    let c = &theme.colors;
    let rest = rest(theme, variant);
    let m = button_metrics(theme, size);
    let radius = size.pick(
        theme.theme.radius.sm,
        theme.theme.radius.md,
        theme.theme.radius.md,
    );

    let frame = || {
        Props::new()
            .font_size(m.font_size)
            .padding(m.padding)
            .radius(radius)
            .fg(rest.fg)
            .border(rest.border.0, rest.border.1)
            .elevation(rest.elevation)
    };
    // One rule gives the right direction in both schemes: a layer mixed from the
    // *text* colour darkens a light surface and lightens a dark one, so Fluent's
    // light-theme hover darkens the accent while its dark-theme hover lifts it.
    let layered = |amount: f32| {
        frame()
            .bg_if_opaque(composite_over(rest.bg, c.text, amount))
            .build()
    };

    let resting = layered(0.0);
    // A pointing device exists, so hover must be visible.
    let hovered = layered(HOVER_LAYER);
    let pressed = layered(PRESSED_LAYER);
    let selected = {
        let mut p = layered(SELECTED_LAYER);
        if variant.has_elevation_at_rest() {
            p.elevation = Some(theme.theme.elevation.level2);
        }
        p
    };
    let focused = {
        let mut p = layered(PRESSED_LAYER);
        p.border_width = Some(FOCUS_RING_WIDTH);
        p.border_color = Some(c.focus_ring);
        p
    };
    let disabled = match rest.disabled {
        DisabledStyle::Neutralised => frame()
            .bg(c.layer)
            .fg(c.text_disabled)
            .elevation(0.0)
            .build(),
        DisabledStyle::Outline => frame().fg(c.text_disabled).build(),
        DisabledStyle::Bare => frame()
            .fg(c.text_disabled)
            .border(0.0, Color::TRANSPARENT)
            .build(),
    };

    StyleSetBuilder::new()
        .state(InteractionState::Rest, resting)
        .state(InteractionState::Hovered, hovered)
        .state(InteractionState::Pressed, pressed)
        .state(InteractionState::Focused, focused)
        .state(InteractionState::Disabled, disabled)
        .state(InteractionState::Selected, selected)
        .build()
}

/// One state of one variant.
pub fn button_style(
    theme: &FluentTheme,
    variant: Variant,
    size: ControlSize,
    state: InteractionState,
) -> StyleProps {
    button_styles(theme, variant, size).resolve_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> FluentTheme {
        FluentTheme::light()
    }

    #[test]
    fn hover_and_focus_both_change_something() {
        // The mirror of Cupertino's rule, and the reason both crates assert it:
        // a design system that ignores the pointing device it is running on is
        // as wrong as one that assumes a touch screen.
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            assert_ne!(
                button_style(&t, variant, ControlSize::Md, InteractionState::Hovered),
                rest,
                "{variant:?} ignores hover"
            );
            assert_ne!(
                button_style(&t, variant, ControlSize::Md, InteractionState::Focused),
                rest,
                "{variant:?} ignores focus"
            );
        }
    }

    #[test]
    fn only_the_outlined_variant_carries_a_resting_border() {
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            assert_eq!(
                rest.border_width.unwrap_or(0.0) > 0.0,
                variant.has_border_at_rest(),
                "{variant:?}"
            );
        }
    }

    #[test]
    fn only_the_elevated_variant_is_raised_at_rest() {
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            assert_eq!(
                rest.elevation.unwrap_or(0.0) > 0.0,
                variant.has_elevation_at_rest(),
                "{variant:?}"
            );
        }
    }

    #[test]
    fn every_state_of_every_variant_resolves_differently() {
        // Fluent, unlike Cupertino, distinguishes all six: there is no pair of
        // states a user could not tell apart.
        let t = theme();
        for variant in Variant::ALL {
            for size in ControlSize::ALL {
                let mut seen: Vec<(InteractionState, StyleProps)> = Vec::new();
                for state in InteractionState::ALL {
                    let props = button_style(&t, variant, size, state);
                    if let Some((other, _)) = seen.iter().find(|(_, p)| *p == props) {
                        panic!("{variant:?}/{size:?}: {state:?} resolves like {other:?}");
                    }
                    seen.push((state, props));
                }
            }
        }
    }

    #[test]
    fn the_focus_ring_clears_the_boundary_threshold_over_its_resolved_fill() {
        // The fill can be translucent (a subtle or outlined control), so the
        // measurement has to composite it over the page first: comparing a ring
        // against a 7%-alpha colour would report a contrast the user never sees.
        let t = theme();
        for variant in Variant::ALL {
            let f = button_style(&t, variant, ControlSize::Md, InteractionState::Focused);
            let ring = f.border_color.expect("a focus ring has a colour");
            assert_eq!(f.border_width, Some(FOCUS_RING_WIDTH), "{variant:?}");
            let behind = match f.background {
                Some(bg) if bg.a == 255 => bg,
                Some(bg) => composite_over(t.colors.card_background, bg, 1.0),
                None => t.colors.card_background,
            };
            let ratio = floom_design::contrast_ratio(ring, behind);
            assert!(
                ratio >= floom_design::WCAG_AA_LARGE,
                "{variant:?}: ring {ring:?} on {behind:?} is {ratio:.2}:1"
            );
        }
    }

    #[test]
    fn a_subtle_control_hovers_with_a_nearly_clear_tint() {
        // Fluent's subtle hover is 3.74% — a tint the user barely sees. Turning
        // it into a solid fill is the classic way to lose the Fluent look.
        let t = theme();
        let hover = button_style(
            &t,
            Variant::Text,
            ControlSize::Md,
            InteractionState::Hovered,
        );
        let bg = hover.background.expect("hover adds a layer");
        assert!(bg.a > 0 && bg.a < 20, "subtle hover alpha is {}", bg.a);
        assert_eq!(
            (bg.r, bg.g, bg.b),
            (t.colors.text.r, t.colors.text.g, t.colors.text.b),
            "the layer keeps the text colour's hue"
        );
    }

    #[test]
    fn the_accent_darkens_on_hover_in_the_light_scheme_and_lifts_in_the_dark_one() {
        // One rule, two directions — the claim the `text` state layer makes.
        let light = FluentTheme::light();
        let dark = FluentTheme::dark();
        let l_rest = button_style(
            &light,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Rest,
        );
        let l_hover = button_style(
            &light,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Hovered,
        );
        let d_rest = button_style(
            &dark,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Rest,
        );
        let d_hover = button_style(
            &dark,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Hovered,
        );

        let lum = |c: Option<Color>| floom_design::relative_luminance(c.unwrap());
        assert!(
            lum(l_hover.background) < lum(l_rest.background),
            "light hover should darken the accent"
        );
        assert!(
            lum(d_hover.background) > lum(d_rest.background),
            "dark hover should lift the accent"
        );
    }

    #[test]
    fn a_disabled_control_swaps_in_the_disabled_text_colour() {
        let t = theme();
        for variant in Variant::ALL {
            let d = button_style(&t, variant, ControlSize::Md, InteractionState::Disabled);
            assert_eq!(d.foreground, Some(t.colors.text_disabled), "{variant:?}");
            assert_eq!(d.elevation.unwrap_or(0.0), 0.0, "{variant:?}");
        }
        // A neutralised control loses the accent entirely.
        let d = button_style(
            &t,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Disabled,
        );
        assert_eq!(d.background, Some(t.colors.layer));
        assert_ne!(d.background, Some(t.colors.accent));
    }

    #[test]
    fn the_size_ladder_is_the_fluent_one() {
        let t = theme();
        assert_eq!(button_metrics(&t, ControlSize::Sm).height, 24.0);
        assert_eq!(button_metrics(&t, ControlSize::Md).height, 32.0);
        assert_eq!(button_metrics(&t, ControlSize::Lg).height, 40.0);
        // Small controls use the small radius; the other two share Fluent's 4.
        assert_eq!(
            button_style(
                &t,
                Variant::Outlined,
                ControlSize::Sm,
                InteractionState::Rest
            )
            .radius,
            Some(t.theme.radius.sm)
        );
        assert_eq!(
            button_style(
                &t,
                Variant::Outlined,
                ControlSize::Lg,
                InteractionState::Rest
            )
            .radius,
            Some(t.theme.radius.md)
        );
    }
}
