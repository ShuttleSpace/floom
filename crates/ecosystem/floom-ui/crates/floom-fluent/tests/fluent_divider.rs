#![cfg(feature = "divider")]
//! Fluent divider: the rule is flush with the top of its box.

use floom_signal::SignalUpdate;
use floom_types::Rect;
use floom_widgets::{Divider, RuleAlignment};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DividerTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::divider::DividerOrientation;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::divider::FluentDivider;

const TALL: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 100.0,
};

fn fluent() -> FluentDivider {
    FluentDivider::new().with_theme(MultiTheme::new_for(DesignSystem::Fluent))
}

fn tokens() -> DividerTokens {
    match MultiTheme::new_for(DesignSystem::Fluent).component(ComponentId::Divider) {
        ComponentTokens::Divider(t) => t,
        _ => unreachable!(),
    }
}

#[test]
fn a_horizontal_rule_sits_on_the_top_edge() {
    let rule = fluent().divider().resolve_bounds(TALL);
    assert_eq!(rule.y0, TALL.y0, "v1 wrote no vertical placement, so: top");
    assert_eq!(rule.y1, TALL.y0 + 1.0);
}

#[test]
fn the_engine_default_would_have_centred_it() {
    let centred = Divider::new().resolve_bounds(TALL);
    assert_eq!(
        centred.y0, 49.5,
        "the engine centres a 1-thick rule in a 100-tall box; this kit does not want that"
    );
    assert_ne!(
        fluent().divider().rule_alignment(),
        RuleAlignment::Center,
        "and it asked for Start explicitly rather than inheriting the default"
    );
}

#[test]
fn a_vertical_rule_sits_on_the_left_edge() {
    let rule = fluent()
        .orientation(DividerOrientation::Vertical)
        .divider()
        .resolve_bounds(TALL);
    assert_eq!(rule.x0, TALL.x0);
    assert_eq!(rule.x1, TALL.x0 + 1.0);
}

#[test]
fn the_rule_spans_its_axis_fully() {
    let horizontal = fluent().divider().resolve_bounds(TALL);
    assert_eq!((horizontal.x0, horizontal.x1), (TALL.x0, TALL.x1));

    let vertical = fluent()
        .orientation(DividerOrientation::Vertical)
        .divider()
        .resolve_bounds(TALL);
    assert_eq!((vertical.y0, vertical.y1), (TALL.y0, TALL.y1));
}

#[test]
fn the_inset_shrinks_both_ends_and_leaves_the_start_edge_alone() {
    let plain = fluent().divider().resolve_bounds(TALL);
    let inset = fluent().inset(16.0).divider().resolve_bounds(TALL);
    assert_eq!(
        inset.y0, plain.y0,
        "the start edge is not affected by the inset"
    );
    assert_eq!(inset.x0, TALL.x0 + 16.0);
    assert_eq!(inset.x1, TALL.x1 - 16.0);
}

#[test]
fn the_thickness_is_the_tokens() {
    assert_eq!(fluent().resolved_style().thickness, tokens().thickness);
    assert_eq!(tokens().thickness, 1.0);
}

#[test]
fn the_colour_is_outline_variant() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let resolved = fluent().with_theme(theme.clone()).resolved_style();
    assert_eq!(resolved.color, theme.colors().outline_variant);
}

#[test]
fn the_rule_is_painted_in_the_resolved_colour() {
    let resolved = fluent().resolved_style();
    let widget = fluent().divider();

    let id = floom_core::ElementId::new(1, 1);
    let painted = floom_widgets::testkit::paint_snapshot(&widget, id, TALL);

    assert_eq!(
        painted.split_whitespace().last(),
        Some(
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                resolved.color.r, resolved.color.g, resolved.color.b, resolved.color.a
            )
            .as_str()
        ),
        "painted: {painted}"
    );
}

#[test]
fn the_props_are_kept() {
    let built = fluent()
        .orientation(DividerOrientation::Vertical)
        .inset(12.0);
    assert_eq!(built.props().orientation, DividerOrientation::Vertical);
    assert_eq!(built.props().inset, 12.0);
}

#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(fluent().resolved_style().design_system, "fluent");
}

#[test]
fn the_default_is_a_flush_horizontal_rule() {
    let rule = fluent().divider().resolve_bounds(TALL);
    assert_eq!((rule.y0, rule.height()), (TALL.y0, 1.0));
    assert_eq!(rule.width(), TALL.width(), "and it spans the width");
}

#[test]
fn the_theme_is_a_snapshot_taken_at_construction() {
    let built = fluent().with_theme(MultiTheme::new_for(DesignSystem::Fluent));
    let before = built.resolved_style().color;

    floom_core_theme::global_theme::design_system_signal().set(DesignSystem::Material);

    let after = built.resolved_style().color;
    assert_eq!(
        before, after,
        "a snapshot cannot follow a theme switch; if this now differs, this crate \
         has been switched to a live signal and the divergence is gone"
    );
}
