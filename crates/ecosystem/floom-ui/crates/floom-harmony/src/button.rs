use floom_design::{
    ControlSize, InteractionState, Props, StyleSetBuilder, StyleSetExt, Variant, composite_over,
    most_contrasting,
};
use floom_style::{StyleProps, StyleSet};
use floom_types::Color;

use crate::tokens::HarmonyTheme;

/// HarmonyOS's documented interaction-state opacities.
const HOVER_LAYER: f32 = 0.05;
const PRESSED_LAYER: f32 = 0.10;
const SELECTED_LAYER: f32 = 0.15;
const DISABLED_LAYER: f32 = 0.04;
/// The focus ring's stroke width, in vp.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// A control's box metrics at one size step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonMetrics {
    pub height: f32,
    /// Half the height: a HarmonyOS control is a capsule, so its corner is a
    /// consequence of its height rather than a separate choice.
    pub radius: f32,
    pub padding: f32,
    pub font_size: f32,
}

pub fn button_metrics(theme: &HarmonyTheme, size: ControlSize) -> ButtonMetrics {
    let t = &theme.theme;
    let height = size.pick(32.0, 40.0, 48.0);
    ButtonMetrics {
        height,
        radius: height / 2.0,
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
    /// A 4% neutral container.
    Container,
    /// A hairline divider in place of the brand outline.
    Outline,
    /// Nothing.
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

fn rest(theme: &HarmonyTheme, variant: Variant) -> Rest {
    let c = &theme.colors;
    match variant {
        Variant::Filled => Rest {
            bg: c.brand,
            fg: c.on_brand,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Container,
        },
        Variant::Tonal => Rest {
            bg: theme.neutral_fill(),
            fg: c.text,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Container,
        },
        Variant::Outlined => Rest {
            bg: Color::TRANSPARENT,
            fg: c.brand,
            border: (1.0, c.brand),
            elevation: 0.0,
            disabled: DisabledStyle::Outline,
        },
        Variant::Text => Rest {
            bg: Color::TRANSPARENT,
            fg: c.brand,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            disabled: DisabledStyle::Bare,
        },
        Variant::Elevated => Rest {
            bg: c.card,
            fg: c.brand,
            border: (0.0, Color::TRANSPARENT),
            elevation: theme.theme.elevation.level1,
            disabled: DisabledStyle::Container,
        },
    }
}

/// Every state of one variant.
pub fn button_styles(theme: &HarmonyTheme, variant: Variant, size: ControlSize) -> StyleSet {
    let c = &theme.colors;
    let rest = rest(theme, variant);
    let m = button_metrics(theme, size);

    let frame = || {
        Props::new()
            .font_size(m.font_size)
            .padding(m.padding)
            .radius(m.radius)
            .fg(rest.fg)
            .border(rest.border.0, rest.border.1)
            .elevation(rest.elevation)
    };
    // HarmonyOS overlays the scheme's *fill base* — black on light, white on
    // dark — at a documented opacity, so one set of numbers works in both
    // schemes instead of a light-theme and a dark-theme table.
    let layered = |amount: f32| {
        frame()
            .bg_if_opaque(composite_over(rest.bg, c.fill, amount))
            .build()
    };

    let resting = layered(0.0);
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
        let bg = composite_over(rest.bg, c.fill, PRESSED_LAYER);
        let ring = most_contrasting(
            if bg.a == 255 {
                bg
            } else {
                composite_over(c.card, bg, 1.0)
            },
            &[c.brand, c.text, Color::WHITE, Color::BLACK],
        );
        frame()
            .bg_if_opaque(bg)
            .border(FOCUS_RING_WIDTH, ring)
            .build()
    };
    let disabled = {
        let content = c.text_disabled;
        match rest.disabled {
            DisabledStyle::Container => frame()
                .bg_if_opaque(composite_over(c.card, c.fill, DISABLED_LAYER))
                .fg(content)
                .elevation(0.0)
                .build(),
            DisabledStyle::Outline => frame().fg(content).border(1.0, c.divider).build(),
            DisabledStyle::Bare => frame().fg(content).border(0.0, Color::TRANSPARENT).build(),
        }
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
    theme: &HarmonyTheme,
    variant: Variant,
    size: ControlSize,
    state: InteractionState,
) -> StyleProps {
    button_styles(theme, variant, size).resolve_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> HarmonyTheme {
        HarmonyTheme::light()
    }

    #[test]
    fn every_control_is_a_capsule_at_every_size() {
        // The signature rule: the radius is derived from the height, so it is
        // different at each size step. Asserting a constant radius would let a
        // control with square corners pass at exactly one size.
        let t = theme();
        for variant in Variant::ALL {
            for size in ControlSize::ALL {
                let rest = button_style(&t, variant, size, InteractionState::Rest);
                let expected = button_metrics(&t, size).height / 2.0;
                assert_eq!(
                    rest.radius,
                    Some(expected),
                    "{variant:?}/{size:?} is not a capsule"
                );
            }
        }
        // And the derived radius really does vary with size.
        let r = |size| {
            button_style(&t, Variant::Filled, size, InteractionState::Rest)
                .radius
                .unwrap()
        };
        assert!(r(ControlSize::Sm) < r(ControlSize::Md));
        assert!(r(ControlSize::Md) < r(ControlSize::Lg));
    }

    #[test]
    fn hover_is_a_five_percent_layer_and_press_is_ten() {
        let t = theme();
        let rest = button_style(&t, Variant::Tonal, ControlSize::Md, InteractionState::Rest);
        let hover = button_style(
            &t,
            Variant::Tonal,
            ControlSize::Md,
            InteractionState::Hovered,
        );
        let press = button_style(
            &t,
            Variant::Tonal,
            ControlSize::Md,
            InteractionState::Pressed,
        );
        let expected =
            |amount: f32| composite_over(rest.background.unwrap(), t.colors.fill, amount);
        assert_eq!(hover.background, Some(expected(HOVER_LAYER)));
        assert_eq!(press.background, Some(expected(PRESSED_LAYER)));
        // Pressing is visibly stronger than hovering, in the right direction.
        assert_ne!(hover.background, press.background);
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
    fn the_focus_ring_clears_the_boundary_threshold_over_its_resolved_fill() {
        let t = theme();
        for variant in Variant::ALL {
            let f = button_style(&t, variant, ControlSize::Md, InteractionState::Focused);
            let ring = f.border_color.expect("a focus ring has a colour");
            assert_eq!(f.border_width, Some(FOCUS_RING_WIDTH), "{variant:?}");
            let behind = match f.background {
                Some(bg) if bg.a == 255 => bg,
                Some(bg) => composite_over(t.colors.card, bg, 1.0),
                None => t.colors.card,
            };
            let ratio = floom_design::contrast_ratio(ring, behind);
            assert!(
                ratio >= floom_design::WCAG_AA_LARGE,
                "{variant:?}: ring {ring:?} on {behind:?} is {ratio:.2}:1"
            );
        }
    }

    #[test]
    fn the_disabled_container_is_a_neutral_not_the_brand() {
        let t = theme();
        let d = button_style(
            &t,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Disabled,
        );
        assert_eq!(d.foreground, Some(t.colors.text_disabled));
        assert_ne!(d.background, Some(t.colors.brand), "the brand survived");
        assert_eq!(d.elevation, Some(0.0));
        // The outlined variant keeps a border but swaps it for the hairline.
        let o = button_style(
            &t,
            Variant::Outlined,
            ControlSize::Md,
            InteractionState::Disabled,
        );
        assert_eq!(o.border_color, Some(t.colors.divider));
    }

    #[test]
    fn one_set_of_state_numbers_works_in_both_schemes() {
        // Because the layer is mixed from the scheme's own fill base, the
        // light scheme's hover darkens and the dark scheme's lightens with the
        // same opacity. If a per-scheme table crept in, these would diverge.
        let light = HarmonyTheme::light();
        let dark = HarmonyTheme::dark();
        let lum = |theme: &HarmonyTheme, state: InteractionState| {
            let bg = button_style(theme, Variant::Tonal, ControlSize::Md, state)
                .background
                .unwrap();
            (bg.r as u32 + bg.g as u32 + bg.b as u32) as f32
        };
        assert!(
            lum(&light, InteractionState::Hovered) < lum(&light, InteractionState::Rest),
            "light hover should darken"
        );
        assert!(
            lum(&dark, InteractionState::Hovered) > lum(&dark, InteractionState::Rest),
            "dark hover should lighten"
        );
    }

    #[test]
    fn every_state_of_every_variant_resolves_differently() {
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
}
