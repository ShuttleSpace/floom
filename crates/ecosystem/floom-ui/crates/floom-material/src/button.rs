//! Material 3 button variants across the six interaction states.

use floom_design::{
    ControlSize, InteractionState, Props, StyleSetBuilder, Variant, composite_over,
    most_contrasting, with_alpha,
};
use floom_style::{StyleProps, StyleSet};
use floom_types::Color;

use crate::tokens::MaterialTheme;

/// Material's hover state layer: the content colour at 8%.
const HOVER_LAYER: f32 = 0.08;
/// Pressed and focused layers: 12%.
const PRESSED_LAYER: f32 = 0.12;
/// Selected stands in for Material's *dragged* layer, which is 16%.
const SELECTED_LAYER: f32 = 0.16;
/// A disabled filled container is `onSurface` at 12% — 31/255, rounded.
const DISABLED_CONTAINER_ALPHA: u8 = 31;
/// Disabled content is `onSurface` at 38% — 97/255, rounded.
const DISABLED_CONTENT_ALPHA: u8 = 97;
/// Width of the focus ring, in logical px.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// A button's box metrics at one size step.
///
/// Material's own steps are 32/40/48 dp; they are spelled out rather than
/// derived from a shared multiplier because a design system that invents its
/// own ladder is no longer that design system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonMetrics {
    pub height: f32,
    pub padding: f32,
    pub font_size: f32,
}

/// The metrics of a Material button at `size`.
pub fn button_metrics(theme: &MaterialTheme, size: ControlSize) -> ButtonMetrics {
    let t = &theme.theme;
    ButtonMetrics {
        height: size.pick(32.0, 40.0, 48.0),
        padding: size.pick(t.spacing.sm, t.spacing.md, t.spacing.lg),
        font_size: size.pick(
            t.typography.size_sm,
            t.typography.size_md,
            t.typography.size_lg,
        ),
    }
}

/// What a variant does when disabled.
///
/// The three cases are genuinely different in Material: a filled button keeps a
/// grey container, an outlined one keeps a grey outline but no fill, and a text
/// button has nothing to grey out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisabledStyle {
    /// A 12%-`onSurface` container.
    Container,
    /// A 12%-`onSurface` outline, transparent inside.
    Outline,
    /// Nothing at all.
    Bare,
}

/// The variant's identity at rest, from which every state is derived.
#[derive(Debug, Clone, Copy)]
struct Rest {
    bg: Color,
    fg: Color,
    border: (f32, Color),
    elevation: f32,
    /// The colour the interaction layers are mixed *from*: Material mixes the
    /// content colour over the container, not a generic white.
    layer: Color,
    /// The variant's *preferred* focus-ring colour. It is a preference, not the
    /// answer: [`focused`] measures it against the fill it will sit on and falls
    /// back when the preferred colour would be invisible.
    ring: Color,
    disabled: DisabledStyle,
}

fn rest(theme: &MaterialTheme, variant: Variant) -> Rest {
    let r = &theme.roles;
    match variant {
        Variant::Filled => Rest {
            bg: r.primary,
            fg: r.on_primary,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            layer: r.on_primary,
            ring: r.on_primary,
            disabled: DisabledStyle::Container,
        },
        Variant::Tonal => Rest {
            bg: r.secondary_container,
            fg: r.on_secondary_container,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            layer: r.on_secondary_container,
            ring: r.on_secondary_container,
            disabled: DisabledStyle::Container,
        },
        Variant::Outlined => Rest {
            bg: Color::TRANSPARENT,
            fg: r.primary,
            border: (1.0, r.outline),
            elevation: 0.0,
            layer: r.primary,
            ring: r.primary,
            disabled: DisabledStyle::Outline,
        },
        Variant::Text => Rest {
            bg: Color::TRANSPARENT,
            fg: r.primary,
            border: (0.0, Color::TRANSPARENT),
            elevation: 0.0,
            layer: r.primary,
            ring: r.primary,
            disabled: DisabledStyle::Bare,
        },
        Variant::Elevated => Rest {
            bg: r.surface,
            fg: r.primary,
            border: (0.0, Color::TRANSPARENT),
            // Material elevates a resting elevated button to its level 1.
            elevation: theme.theme.elevation.level1,
            layer: r.primary,
            ring: r.primary,
            disabled: DisabledStyle::Container,
        },
    }
}

/// Every state of one variant, as a resolvable [`StyleSet`].
pub fn button_styles(theme: &MaterialTheme, variant: Variant, size: ControlSize) -> StyleSet {
    let r = &theme.roles;
    let rest = rest(theme, variant);
    let metrics = button_metrics(theme, size);
    let radius = theme.theme.radius.full;

    let frame = || {
        Props::new()
            .font_size(metrics.font_size)
            .padding(metrics.padding)
            .radius(radius)
    };
    // The rest frame with a state layer mixed over the container.
    let layered = |amount: f32| {
        frame()
            .bg_if_opaque(composite_over(rest.bg, rest.layer, amount))
            .fg(rest.fg)
            .border(rest.border.0, rest.border.1)
            .elevation(rest.elevation)
            .build()
    };

    let hover = layered(HOVER_LAYER);
    let pressed = layered(PRESSED_LAYER);
    let selected = {
        let mut p = layered(SELECTED_LAYER);
        if variant.has_elevation_at_rest() {
            // Dragging lifts a raised button further off the page.
            p.elevation = Some(theme.theme.elevation.level3);
        }
        p
    };
    let focused = {
        // A ring in the variant's own colour is invisible on the outlined and
        // text variants, whose focused fill *is* a primary tint. The preferred
        // colour leads the candidate list and measurement demotes it when it
        // would not be seen.
        let bg = composite_over(rest.bg, rest.layer, PRESSED_LAYER);
        let ring = most_contrasting(bg, &[rest.ring, r.on_surface, Color::WHITE, Color::BLACK]);
        frame()
            .bg_if_opaque(bg)
            .fg(rest.fg)
            .border(FOCUS_RING_WIDTH, ring)
            .elevation(rest.elevation)
            .build()
    };
    let disabled = {
        let content = with_alpha(r.on_surface, DISABLED_CONTENT_ALPHA);
        let container = with_alpha(r.on_surface, DISABLED_CONTAINER_ALPHA);
        match rest.disabled {
            DisabledStyle::Container => frame()
                .bg(container)
                .fg(content)
                .border(0.0, Color::TRANSPARENT)
                .elevation(0.0)
                .build(),
            DisabledStyle::Outline => frame().fg(content).border(1.0, container).build(),
            DisabledStyle::Bare => frame().fg(content).border(0.0, Color::TRANSPARENT).build(),
        }
    };

    StyleSetBuilder::new()
        .state(InteractionState::Rest, layered(0.0))
        .state(InteractionState::Hovered, hover)
        .state(InteractionState::Pressed, pressed)
        .state(InteractionState::Focused, focused)
        .state(InteractionState::Disabled, disabled)
        .state(InteractionState::Selected, selected)
        .build()
}

/// One state of one variant.
pub fn button_style(
    theme: &MaterialTheme,
    variant: Variant,
    size: ControlSize,
    state: InteractionState,
) -> StyleProps {
    use floom_design::StyleSetExt;
    button_styles(theme, variant, size).resolve_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> MaterialTheme {
        MaterialTheme::light()
    }

    #[test]
    fn metrics_follow_materials_own_ladder() {
        let t = theme();
        assert_eq!(button_metrics(&t, ControlSize::Sm).height, 32.0);
        assert_eq!(button_metrics(&t, ControlSize::Md).height, 40.0);
        assert_eq!(button_metrics(&t, ControlSize::Lg).height, 48.0);
    }

    #[test]
    fn only_the_outlined_variant_carries_a_resting_border() {
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            let has = rest.border_width.unwrap_or(0.0) > 0.0;
            assert_eq!(
                has,
                variant.has_border_at_rest(),
                "{variant:?} border = {has}"
            );
        }
    }

    #[test]
    fn only_the_elevated_variant_carries_a_resting_shadow() {
        let t = theme();
        for variant in Variant::ALL {
            let rest = button_style(&t, variant, ControlSize::Md, InteractionState::Rest);
            let has = rest.elevation.unwrap_or(0.0) > 0.0;
            assert_eq!(
                has,
                variant.has_elevation_at_rest(),
                "{variant:?} elevation = {has}"
            );
        }
    }

    #[test]
    fn a_filled_button_paints_its_content_in_on_primary() {
        let t = theme();
        let rest = button_style(&t, Variant::Filled, ControlSize::Md, InteractionState::Rest);
        assert_eq!(rest.background, Some(t.roles.primary));
        assert_eq!(rest.foreground, Some(t.roles.on_primary));
    }

    #[test]
    fn disabled_uses_the_documented_alphas_and_drops_the_accent() {
        let t = theme();
        for variant in Variant::ALL {
            let d = button_style(&t, variant, ControlSize::Md, InteractionState::Disabled);
            assert_eq!(
                d.foreground,
                Some(with_alpha(t.roles.on_surface, DISABLED_CONTENT_ALPHA)),
                "{variant:?}"
            );
            assert_eq!(
                d.border_width.unwrap_or(0.0) > 0.0,
                variant == Variant::Outlined,
                "{variant:?} changed its border rule when disabled"
            );
            match variant {
                Variant::Outlined | Variant::Text => {}
                _ => assert_eq!(
                    d.background,
                    Some(with_alpha(t.roles.on_surface, DISABLED_CONTAINER_ALPHA)),
                    "{variant:?} keeps a greyed container"
                ),
            }
            assert_eq!(d.elevation, Some(0.0), "{variant:?} still casts a shadow");
        }
    }

    #[test]
    fn the_focus_ring_clears_the_ui_component_threshold_on_its_own_fill() {
        // A ring drawn in the same colour as the container is not a ring, and
        // this is invisible in a screenshot of the focused state alone. The
        // outlined and text variants are the ones that break: their focused
        // fill is a primary tint, so a primary ring vanishes on it.
        let t = theme();
        for variant in Variant::ALL {
            for size in ControlSize::ALL {
                let f = button_style(&t, variant, size, InteractionState::Focused);
                assert_eq!(f.border_width, Some(FOCUS_RING_WIDTH), "{variant:?}");
                let ring = f.border_color.expect("a focus ring has a colour");
                let behind = f.background.unwrap_or(t.roles.surface);
                let behind = if behind.a == 0 {
                    t.roles.surface
                } else {
                    behind
                };
                let ratio = floom_design::contrast_ratio(ring, behind);
                assert!(
                    ratio >= floom_design::WCAG_AA_LARGE,
                    "{variant:?}/{size:?}: ring {ring:?} on {behind:?} is {ratio:.2}:1"
                );
            }
        }
    }

    #[test]
    fn every_state_of_every_variant_resolves_differently() {
        // The claim the gallery leans on. If two states collapse, a swatch grid
        // shows a duplicate and nothing else notices.
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
    fn a_hover_layer_on_a_bare_container_is_a_tint_not_a_block() {
        // The compressed form of a real bug: `lerp` from a transparent
        // container pulls the RGB toward black, so the "tint" came out dark and
        // muddy instead of being the accent at 8%.
        let t = theme();
        let hover = button_style(
            &t,
            Variant::Text,
            ControlSize::Md,
            InteractionState::Hovered,
        );
        let bg = hover.background.expect("hover adds a layer");
        assert!(bg.a > 0, "the layer must be visible");
        assert!(
            bg.a < 64,
            "an 8% layer over nothing is still nearly clear: {}",
            bg.a
        );
        assert_eq!(
            (bg.r, bg.g, bg.b),
            (t.roles.primary.r, t.roles.primary.g, t.roles.primary.b),
            "the layer kept the accent's hue"
        );
    }

    #[test]
    fn size_changes_the_box_not_the_colour() {
        let t = theme();
        let sm = button_style(&t, Variant::Filled, ControlSize::Sm, InteractionState::Rest);
        let lg = button_style(&t, Variant::Filled, ControlSize::Lg, InteractionState::Rest);
        assert!(lg.padding.unwrap() > sm.padding.unwrap());
        assert!(lg.font_size.unwrap() > sm.font_size.unwrap());
        assert_eq!(sm.background, lg.background);
    }

    #[test]
    fn the_dark_scheme_swaps_the_fill_and_the_content() {
        let light = MaterialTheme::light();
        let dark = MaterialTheme::dark();
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
        assert_eq!(l.background, Some(light.roles.primary));
        assert_eq!(d.background, Some(dark.roles.primary));
        assert_ne!(l.background, d.background);
        // Dark mode inverts the relationship: a light fill with dark content.
        assert_eq!(d.foreground, Some(dark.roles.on_primary));
        assert!(
            floom_design::relative_luminance(dark.roles.primary)
                > floom_design::relative_luminance(dark.roles.on_primary),
            "a dark-scheme primary should be the lighter of the pair"
        );
    }
}
