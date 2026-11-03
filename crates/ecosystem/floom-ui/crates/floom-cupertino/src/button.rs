
use floom_design::{
    ControlSize, InteractionState, Props, StyleSetBuilder, StyleSetExt, Variant, composite_over,
};
use floom_style::{StyleProps, StyleSet};
use floom_types::Color;

use crate::tokens::CupertinoTheme;

/// Apple's minimum comfortable touch target, in points.
pub const HIG_MIN_TOUCH_TARGET: f32 = 44.0;

/// How far a pressed control dims.
const PRESSED_OPACITY: f32 = 0.6;
/// How far a disabled control dims.
const DISABLED_OPACITY: f32 = 0.35;
/// Strength of the selected-state tint layer.
const SELECTED_LAYER: f32 = 0.15;

/// A control's box metrics at one size step.
///
/// `height` is the *visual* height. It is deliberately allowed to be smaller
/// than [`HIG_MIN_TOUCH_TARGET`]: iOS separates what the user sees from what the
/// finger can hit, and [`touch_target`] is the second number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonMetrics {
    pub height: f32,
    pub padding: f32,
    pub font_size: f32,
}

pub fn button_metrics(theme: &CupertinoTheme, size: ControlSize) -> ButtonMetrics {
    let t = &theme.theme;
    ButtonMetrics {
        height: size.pick(28.0, 34.0, 44.0),
        padding: size.pick(t.spacing.sm, t.spacing.md, t.spacing.lg),
        font_size: size.pick(
            t.typography.size_sm,
            t.typography.size_md,
            t.typography.size_lg,
        ),
    }
}

/// The hit area for a control of this size — never below the HIG minimum.
pub fn touch_target(theme: &CupertinoTheme, size: ControlSize) -> f32 {
    button_metrics(theme, size).height.max(HIG_MIN_TOUCH_TARGET)
}

#[derive(Debug, Clone, Copy)]
struct Rest {
    bg: Color,
    fg: Color,
    border: (f32, Color),
    radius: f32,
}

fn rest(theme: &CupertinoTheme, variant: Variant) -> Rest {
    let c = &theme.colors;
    let r = &theme.theme.radius;
    match variant {
        // A filled control is a pill; Apple's bordered and plain styles are not.
        Variant::Filled => Rest {
            bg: c.tint,
            fg: c.on_tint,
            border: (0.0, Color::TRANSPARENT),
            radius: r.full,
        },
        Variant::Tonal => Rest {
            bg: c.fill,
            fg: c.label,
            border: (0.0, Color::TRANSPARENT),
            radius: r.lg,
        },
        Variant::Outlined => Rest {
            bg: Color::TRANSPARENT,
            fg: c.tint,
            border: (1.0, c.tint),
            radius: r.md,
        },
        Variant::Text => Rest {
            bg: Color::TRANSPARENT,
            fg: c.tint,
            border: (0.0, Color::TRANSPARENT),
            radius: r.md,
        },
        // iOS has no raised button. The nearest neighbour is a control sitting
        // on a card: a grouped surface with no shadow at all.
        Variant::Elevated => Rest {
            bg: c.secondary_grouped_background,
            fg: c.tint,
            border: (0.0, Color::TRANSPARENT),
            radius: r.md,
        },
    }
}

/// Every state of one variant.
pub fn button_styles(theme: &CupertinoTheme, variant: Variant, size: ControlSize) -> StyleSet {
    let rest = rest(theme, variant);
    let m = button_metrics(theme, size);

    let frame = || {
        Props::new()
            .font_size(m.font_size)
            .padding(m.padding)
            .radius(rest.radius)
            .bg_if_opaque(rest.bg)
            .fg(rest.fg)
            .border(rest.border.0, rest.border.1)
    };

    let resting = frame().build();
    // Touch: there is no pointing device, so a hover must not change anything.
    let hovered = resting.clone();
    // Keyboard focus is expressed by selection on iOS, not by a ring.
    let focused = resting.clone();
    let pressed = frame().opacity(PRESSED_OPACITY).build();
    let disabled = frame().opacity(DISABLED_OPACITY).build();
    let selected = frame()
        // The layer is mixed from the variant's *content* colour, not its tint.
        // A tint-over-tint layer is a no-op on a filled control — which is
        // exactly how selection first came out invisible here.
        .bg_if_opaque(composite_over(rest.bg, rest.fg, SELECTED_LAYER))
        .build();

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
    theme: &CupertinoTheme,
    variant: Variant,
    size: ControlSize,
    state: InteractionState,
) -> StyleProps {
    button_styles(theme, variant, size).resolve_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> CupertinoTheme {
        CupertinoTheme::light()
    }

    #[test]
    fn hover_and_focus_change_nothing() {
        // The defining iOS rule. On a touch platform a hover treatment is a bug,
        // and a focus ring on a button is not how iOS shows keyboard focus.
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            assert_eq!(
                button_style(&t, variant, ControlSize::Md, InteractionState::Hovered),
                rest,
                "{variant:?} reacts to hover"
            );
            assert_eq!(
                button_style(&t, variant, ControlSize::Md, InteractionState::Focused),
                rest,
                "{variant:?} draws a focus ring"
            );
        }
    }

    #[test]
    fn the_states_that_differ_from_rest_are_exactly_pressed_disabled_selected() {
        // The complement of the test above, and the one that catches an
        // over-eager hand: a state quietly added to the "same as rest" group
        // would make a control stop responding.
        let t = theme();
        let mut same = Vec::new();
        for state in InteractionState::ALL {
            let p = button_style(&t, Variant::Filled, ControlSize::Md, state);
            if p == button_style(&t, Variant::Filled, ControlSize::Md, InteractionState::Rest) {
                same.push(state);
            }
        }
        assert_eq!(
            same,
            vec![
                InteractionState::Rest,
                InteractionState::Hovered,
                InteractionState::Focused
            ]
        );
    }

    #[test]
    fn no_variant_casts_a_shadow() {
        let t = theme();
        for variant in Variant::ALL {
            for state in InteractionState::ALL {
                let p = button_style(&t, variant, ControlSize::Md, state);
                assert_eq!(
                    p.elevation.unwrap_or(0.0),
                    0.0,
                    "{variant:?}/{state:?} casts a shadow"
                );
            }
        }
    }

    #[test]
    fn only_the_outlined_variant_carries_a_border() {
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
    fn press_and_disable_dim_by_different_amounts() {
        // Two states that both mean "not the normal one" must not converge, or
        // a disabled control looks like a held-down one.
        let t = theme();
        let pressed = button_style(
            &t,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Pressed,
        );
        let disabled = button_style(
            &t,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Disabled,
        );
        assert_eq!(pressed.opacity, Some(PRESSED_OPACITY));
        assert_eq!(disabled.opacity, Some(DISABLED_OPACITY));
        assert!(DISABLED_OPACITY < PRESSED_OPACITY);
    }

    #[test]
    fn selection_tints_without_moving_the_geometry() {
        let t = theme();
        let rest = button_style(&t, Variant::Tonal, ControlSize::Md, InteractionState::Rest);
        let selected = button_style(
            &t,
            Variant::Tonal,
            ControlSize::Md,
            InteractionState::Selected,
        );
        assert_ne!(
            selected.background, rest.background,
            "selection is invisible"
        );
        assert_eq!(selected.radius, rest.radius);
        assert_eq!(selected.border_width, rest.border_width);
    }

    #[test]
    fn the_touch_target_clears_the_high_minimum_at_every_size() {
        let t = theme();
        for size in ControlSize::ALL {
            assert!(
                touch_target(&t, size) >= HIG_MIN_TOUCH_TARGET,
                "{size:?} is not hittable"
            );
        }
        // And the minimum is doing real work: the small and medium *visual*
        // heights are below it, so a control drawn at its visual height alone
        // would be too small to tap.
        assert!(button_metrics(&t, ControlSize::Sm).height < HIG_MIN_TOUCH_TARGET);
        assert!(button_metrics(&t, ControlSize::Md).height < HIG_MIN_TOUCH_TARGET);
        assert_eq!(
            button_metrics(&t, ControlSize::Lg).height,
            HIG_MIN_TOUCH_TARGET
        );
    }

    #[test]
    fn a_filled_control_is_a_pill_and_a_bordered_one_is_not() {
        let t = theme();
        let filled = button_style(&t, Variant::Filled, ControlSize::Md, InteractionState::Rest);
        let outlined = button_style(
            &t,
            Variant::Outlined,
            ControlSize::Md,
            InteractionState::Rest,
        );
        assert_eq!(filled.radius, Some(t.theme.radius.full));
        assert_eq!(outlined.radius, Some(t.theme.radius.md));
        assert_ne!(filled.radius, outlined.radius);
    }

    #[test]
    fn the_dark_scheme_flips_the_label_and_keeps_the_control_white_on_blue() {
        let light = CupertinoTheme::light();
        let dark = CupertinoTheme::dark();
        let l = button_style(
            &light,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Rest,
        );
        let d = button_style(
            &dark,
            Variant::Filled,
            ControlSize::Md,
            InteractionState::Rest,
        );
        assert_ne!(l.background, d.background);
        assert_eq!(
            l.foreground, d.foreground,
            "on-tint is white on Apple's blue in both schemes"
        );
        let label_light = button_style(
            &light,
            Variant::Text,
            ControlSize::Md,
            InteractionState::Rest,
        );
        let label_dark = button_style(
            &dark,
            Variant::Text,
            ControlSize::Md,
            InteractionState::Rest,
        );
        assert_ne!(label_light.foreground, label_dark.foreground);
    }
}
