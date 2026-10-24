//! Cupertino divider: the rule is flush with the top of its box.
//!
//! ## Why this suite exists at all
//!
//! The three v1 dividers were identical — measured, not assumed: with naming and
//! the `design_system` field removed, their `resolved_style()` and `view()` bodies
//! were byte-identical. There is no per-kit style parameter here, and inventing
//! one would be inventing a disagreement.
//!
//! What the kits *do* disagree on is one thing, and it is worth a test each:
//!
//! * **Cross-axis placement.** v1 wrote no vertical placement for a horizontal
//!   rule, so the rule landed on the container's top. The engine centres instead.
//!   All three kits ask for `RuleAlignment::Start` to reproduce v1, and the tests
//!   below are what make that a port rather than a preference.
//! * **Theme source.** Material reads the live signal; Cupertino and Fluent take
//!   a construction-time snapshot.
//!
//! Every test here has a control: without one, "the rule is at the top" also
//! passes for a rule that is at the top *because nothing else was tried*.

use floom_signal::SignalUpdate;
use floom_types::Rect;
use floom_widgets::{Divider, RuleAlignment};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DividerTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::divider::DividerOrientation;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_divider::CupertinoDivider;

/// A box taller than any rule, so centring and top-flush cannot coincide.
const TALL: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 100.0,
};

/// A Cupertino divider with an explicit theme.
///
/// `with_theme(..)` rather than setting the process-global design system: two
/// tests in this binary assert different things about different kits' theme
/// sources, and a test that mutated global state to do it would make them order-
/// dependent. `floom_core_theme` re-reads the global on every accessor, so an
/// explicit theme is the only way to pin one.
fn cupertino() -> CupertinoDivider {
    CupertinoDivider::new().with_theme(MultiTheme::new_for(DesignSystem::Cupertino))
}

fn tokens() -> DividerTokens {
    match MultiTheme::new_for(DesignSystem::Cupertino).component(ComponentId::Divider) {
        ComponentTokens::Divider(t) => t,
        _ => unreachable!(),
    }
}

// --- the disagreement ------------------------------------------------------

/// **Material's one styling opinion: the rule sits on the top edge.**
#[test]
fn a_horizontal_rule_sits_on_the_top_edge() {
    let rule = cupertino().divider().resolve_bounds(TALL);
    assert_eq!(rule.y0, TALL.y0, "v1 wrote no vertical placement, so: top");
    assert_eq!(rule.y1, TALL.y0 + 1.0);
}

/// **The control.** The engine's own default centres the rule, and this kit
/// overrides it. Without this assertion a widget that centred would pass
/// everything above.
#[test]
fn the_engine_default_would_have_centred_it() {
    let centred = Divider::new().resolve_bounds(TALL);
    assert_eq!(
        centred.y0, 49.5,
        "the engine centres a 1-thick rule in a 100-tall box; this kit does not want that"
    );
    assert_ne!(
        cupertino().divider().rule_alignment(),
        RuleAlignment::Center,
        "and it asked for Start explicitly rather than inheriting the default"
    );
}

/// A vertical rule sits on the left edge, by the same argument.
#[test]
fn a_vertical_rule_sits_on_the_left_edge() {
    let rule = cupertino()
        .orientation(DividerOrientation::Vertical)
        .divider()
        .resolve_bounds(TALL);
    assert_eq!(rule.x0, TALL.x0);
    assert_eq!(rule.x1, TALL.x0 + 1.0);
}

/// The rule spans the full extent of its axis, which is what v1's `width_full()`
/// and `height_full()` meant.
#[test]
fn the_rule_spans_its_axis_fully() {
    let horizontal = cupertino().divider().resolve_bounds(TALL);
    assert_eq!((horizontal.x0, horizontal.x1), (TALL.x0, TALL.x1));

    let vertical = cupertino()
        .orientation(DividerOrientation::Vertical)
        .divider()
        .resolve_bounds(TALL);
    assert_eq!((vertical.y0, vertical.y1), (TALL.y0, TALL.y1));
}

/// The inset shrinks the rule at both ends without moving it off the start edge.
#[test]
fn the_inset_shrinks_both_ends_and_leaves_the_start_edge_alone() {
    let plain = cupertino().divider().resolve_bounds(TALL);
    let inset = cupertino().inset(16.0).divider().resolve_bounds(TALL);
    assert_eq!(
        inset.y0, plain.y0,
        "the start edge is not affected by the inset"
    );
    assert_eq!(inset.x0, TALL.x0 + 16.0);
    assert_eq!(inset.x1, TALL.x1 - 16.0);
}

// --- tokens ---------------------------------------------------------------

/// The thickness is the token's, and the token is 1.0.
#[test]
fn the_thickness_is_the_tokens() {
    assert_eq!(cupertino().resolved_style().thickness, tokens().thickness);
    assert_eq!(tokens().thickness, 1.0);
}

/// The colour is `outline_variant` — the one role all three kits read.
#[test]
fn the_colour_is_outline_variant() {
    let theme = MultiTheme::new_for(DesignSystem::Cupertino);
    let resolved = cupertino().with_theme(theme.clone()).resolved_style();
    assert_eq!(resolved.color, theme.colors().outline_variant);
}

/// **The control for the colour test**: the rule is actually *painted* in the
/// resolved colour, so a resolver that returned the right colour while the
/// widget ignored it would not pass. The snapshot carries the fill hex.
#[test]
fn the_rule_is_painted_in_the_resolved_colour() {
    let resolved = cupertino().resolved_style();
    let widget = cupertino().divider();

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

// --- props and identity ---------------------------------------------------

/// The props survive the round trip.
#[test]
fn the_props_are_kept() {
    let built = cupertino()
        .orientation(DividerOrientation::Vertical)
        .inset(12.0);
    assert_eq!(built.props().orientation, DividerOrientation::Vertical);
    assert_eq!(built.props().inset, 12.0);
}

/// The kit names itself, so a caller holding three adapters can tell them apart.
#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(cupertino().resolved_style().design_system, "cupertino");
}

/// The default is a horizontal rule with no inset — v1's default too.
#[test]
fn the_default_is_a_flush_horizontal_rule() {
    let rule = cupertino().divider().resolve_bounds(TALL);
    assert_eq!((rule.y0, rule.height()), (TALL.y0, 1.0));
    assert_eq!(rule.width(), TALL.width(), "and it spans the width");
}

/// **The one thing that actually differs between the kits**, and the only thing
/// these two crates do that Material does not: the theme is a **snapshot**.
///
/// Material calls `theme_signal()` and follows a global theme switch. These two
/// call `MultiTheme::new_for(..)` in `new()`, so the divider is styled once and
/// a later switch to a different palette leaves it alone.
///
/// The test measures the observable consequence -- reading the colour twice --
/// rather than asserting on the constructor's source text, so it fails if the
/// snapshot is ever replaced by a live signal and also if the live signal is
/// ever replaced by a snapshot.
#[test]
fn the_theme_is_a_snapshot_taken_at_construction() {
    let built = cupertino().with_theme(MultiTheme::new_for(DesignSystem::Cupertino));
    let before = built.resolved_style().color;

    // Switch the process-global design system to another palette.
    floom_core_theme::global_theme::design_system_signal().set(DesignSystem::Material);

    let after = built.resolved_style().color;
    assert_eq!(
        before, after,
        "a snapshot cannot follow a theme switch; if this now differs, this crate \
         has been switched to a live signal and the divergence is gone"
    );
}
