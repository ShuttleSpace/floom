#![cfg(feature = "icon_button")]
//! The ported Fluent icon button.

use floom_core::Widget;
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::icon_button::IconButtonVariant as PropsVariant;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;
use floom_widgets::{Pressable, Ripple, Surface};

use floom_fluent::widgets::icon_button::FluentIconButton;

fn fluent() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn button() -> FluentIconButton {
    let _ = fluent();
    FluentIconButton::new("close")
}

#[test]
fn the_tokens_reach_the_engine_style() {
    let theme = fluent();
    let style = button().resolved_style();

    let floom_core_theme::component_tokens::ComponentTokens::IconButton(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::IconButton)
    else {
        unreachable!("ComponentId::IconButton resolves to IconButtonTokens")
    };

    assert_eq!(style.size, tokens.size, "the button's edge length");
    assert_eq!(style.icon_size, tokens.icon_size, "and the glyph's");
    assert_eq!(style.radius, tokens.radius, "and the corner radius");
    assert_eq!(
        style.border_width, 1.0,
        "v1 wrote `.border(1.0)` unconditionally"
    );
}

#[test]
fn an_explicit_size_replaces_the_token_and_leaves_the_glyph() {
    let token_size = button().resolved_style().size;
    let resized = button().size(72.0).resolved_style();

    assert_eq!(resized.size, 72.0, "the box takes the explicit size");
    assert_eq!(
        resized.icon_size,
        button().resolved_style().icon_size,
        "the glyph keeps its own token — v1 scaled one and not the other"
    );
    assert_ne!(resized.size, token_size, "and the two really do differ");
}

/// That is correction 9 biting a control group written to catch a different bug. A
#[test]
fn the_crate_really_reads_fluent_tokens() {
    let style = button().resolved_style();
    let theme = fluent();
    assert_eq!(
        style.foreground,
        to_paint(theme.colors().on_surface_variant),
        "the icon's foreground is Fluent's `on_surface_variant`"
    );
    assert_eq!(
        style.contained.background,
        to_paint(theme.colors().primary),
        "and the contained fill is Fluent's primary — Material's would still \
         satisfy every other assertion here"
    );
}

#[test]
fn the_variant_table_matches_v1s_three_pairs() {
    let theme = fluent();
    let style = button().resolved_style();

    let transparent = style.text.background;
    assert_eq!(
        (style.text.background, style.text.border),
        (transparent, transparent),
        "text: v1's `Color::rgba(0, 0, 0, 0)` for both"
    );
    assert_eq!(
        (style.contained.background, style.contained.border),
        (
            to_paint(theme.colors().primary),
            to_paint(theme.colors().primary)
        ),
        "contained: filled *and* outlined in the primary"
    );
    assert_eq!(
        (style.outlined.background, style.outlined.border),
        (transparent, to_paint(theme.colors().outline)),
        "outlined: no fill, outlined in the outline colour"
    );
}

#[test]
fn the_ripple_ink_follows_v1s_rule() {
    let theme = fluent();
    let style = button().resolved_style();

    assert_eq!(
        style.contained.ripple_ink,
        to_paint(with_alpha(theme.colors().on_primary, 24)),
        "contained: `on_primary` at 24"
    );
    for (name, paint) in [("text", &style.text), ("outlined", &style.outlined)] {
        assert_eq!(
            paint.ripple_ink,
            to_paint(with_alpha(theme.colors().on_surface, 24)),
            "{name}: `on_surface` at 24"
        );
    }
    assert_ne!(
        style.contained.ripple_ink, style.text.ripple_ink,
        "CONTROL: the contained ink really is a different colour, so the rule is \\
         doing something rather than reading one arm twice"
    );
}

#[test]
fn the_disabled_colours_are_two_alphas_of_on_surface() {
    let theme = fluent();
    let style = button().resolved_style();
    let on_surface = theme.colors().on_surface;

    assert_eq!(
        style.disabled_foreground,
        to_paint(with_alpha(on_surface, 38))
    );
    assert_eq!(style.disabled_border, to_paint(with_alpha(on_surface, 38)));
    assert_eq!(
        style.disabled_background,
        to_paint(with_alpha(on_surface, 12)),
        "the fill is fainter than the outline, which is why they are two fields"
    );
    assert_eq!(
        style.foreground,
        to_paint(theme.colors().on_surface_variant)
    );

    assert_ne!(
        style.disabled_background, style.disabled_border,
        "CONTROL: 12 and 38 are different, so the pair is not one value twice"
    );
}

#[test]
fn every_props_variant_maps_to_an_engine_variant() {
    for (props, expected) in [
        (
            PropsVariant::Text,
            floom_ui_composites::IconButtonVariant::Text,
        ),
        (
            PropsVariant::Contained,
            floom_ui_composites::IconButtonVariant::Contained,
        ),
        (
            PropsVariant::Outlined,
            floom_ui_composites::IconButtonVariant::Outlined,
        ),
    ] {
        assert_eq!(
            button().variant(props).engine_variant(),
            expected,
            "{props:?} crosses the boundary unchanged"
        );
    }
}

#[test]
fn the_kit_hands_the_engine_a_pressable_box() {
    let view = button().view();
    let children = view.children();
    assert_eq!(children.len(), 1);

    let ripple = children[0].build();
    assert!(
        ripple.as_any().is::<Ripple>(),
        "the ripple is still outermost after the kit's translation"
    );
    let pressable = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let pressable = pressable.children()[0].build();
    assert!(
        pressable.as_any().is::<Pressable>(),
        "the pressable is still the click target"
    );
    let surface = pressable
        .as_any()
        .downcast_ref::<Pressable>()
        .unwrap()
        .children()[0]
        .build();
    assert!(
        surface.as_any().is::<Surface>(),
        "and the surface is still the box"
    );
}

#[test]
fn the_glyph_takes_the_buttons_colour_and_not_the_icons_own() {
    let style = button().resolved_style();
    let icon_own = to_paint(button().icon_style().color);

    assert_ne!(
        style.foreground, icon_own,
        "this kit's icon foreground and the button's enabled glyph are different \
         colours, so the override is observable while enabled"
    );
    assert_eq!(
        button().view().icon().fill(),
        style.foreground,
        "the mounted icon carries the button's colour, not its own"
    );
}

#[test]
fn a_disabled_buttons_glyph_is_the_disabled_foreground() {
    let style = button().disabled(true).resolved_style();
    let view = button().disabled(true).view();
    assert_eq!(view.icon().fill(), style.disabled_foreground);
    assert_ne!(
        view.icon().fill(),
        button().view().icon().fill(),
        "CONTROL: and it really is a different colour from the enabled one"
    );
}

#[test]
fn the_glyph_is_sized_from_the_icon_token() {
    let style = button().resolved_style();
    let view = button().size(72.0).view();
    assert_eq!(view.icon().edge(), style.icon_size);
}

#[test]
fn two_views_agree() {
    let a = button().view();
    let b = button().view();
    assert_eq!(a.resolved_style(), b.resolved_style());
    assert_eq!(a.icon().fill(), b.icon().fill());
    assert_eq!(a.icon().edge(), b.icon().edge());
}

#[test]
fn the_props_report_the_click_callback() {
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let button = {
        let hits = hits.clone();
        button().on_click(move || {
            hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
    };

    assert!(
        button.props().on_click.is_some(),
        "the v1-shaped copy is set, so `props()` reports what v1 reported"
    );

    let view = button.view();
    let mut ripple = view.children()[0].build();
    let mut inner = {
        let ripple = ripple.as_any_mut().downcast_mut::<Ripple>().unwrap();
        ripple.children()[0].build()
    };
    let pressable = inner
        .as_any_mut()
        .downcast_mut::<Pressable>()
        .expect("the pressable");

    use floom_types::{Point, PointerEvent, Rect};
    let bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 40.0,
        y1: 40.0,
    };
    let id = pressable.create(&mut floom_widgets::testkit::AllocatingCreateContext::new());
    let at = Point::new(20.0, 20.0);
    let mut down = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_down(at).into(),
    );
    pressable.event(&mut down, id);
    let mut up = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_up(at).into(),
    );
    pressable.event(&mut up, id);

    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "CONTROL: the engine's copy fired"
    );
}

#[test]
fn a_button_with_no_callback_is_fine() {
    assert!(button().props().on_click.is_none());
    let view = button().view();
    assert_eq!(view.children().len(), 1, "and it still assembles");
}

fn to_paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
