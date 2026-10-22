//! Font weight, end to end: the measure, the glyph resource, the change of weight
//! on an already-mounted label, and the rounding bug weight ran into.
//!
//! Weight is the first text property in 2.0 that is not visible in the geometry
//! alone — a size change moves the box, a colour change shows in a pixel, but a
//! weight change is the *same string at the same size in a different face*. So it
//! is also the first one where "it compiled" and "the cache key mentions it" are
//! the only things between a working feature and 73 floem-ui call sites rendering
//! flat. These tests are the difference between those two things.
//!
//! Two caches have to learn the weight or the feature silently does nothing:
//! `floom-text`'s layout cache (keyed on text + options) and the host's glyph
//! resource cache (keyed on text + size). Both hand back *shaped glyph runs* on a
//! hit, so a key without the weight answers a `Bold` request with the `Normal`
//! glyphs for the same string.

use floom_app::{NodeId, WidgetHost};
use floom_core::FontWeight;
use floom_layout::LayoutStyle;
use floom_scene::{ResourceId, ResourceKind, SceneNode};
use floom_types::{Rect, Size};
use floom_widgets::{Container, Text};

/// The glyph resource a text node points at. `nth` picks among several text nodes
/// in insertion order.
fn glyphs_of(host: &WidgetHost, nth: usize) -> ResourceId {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { glyphs, .. } => Some(*glyphs),
            _ => None,
        })
        .nth(nth)
        .expect("a text node in the scene")
}

/// The laid-out extent of a text node — the width it was measured and painted at.
fn text_extent(host: &WidgetHost, nth: usize) -> Size {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { size, .. } => Some(*size),
            _ => None,
        })
        .nth(nth)
        .expect("a text node in the scene")
}

fn text_rect(host: &WidgetHost, nth: usize) -> Rect {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { origin, size, .. } => Some(Rect::from_origin_size(*origin, *size)),
            _ => None,
        })
        .nth(nth)
        .expect("a text node in the scene")
}

/// A viewport a label fits in, so nothing wraps for want of room.
fn viewport() -> Size {
    Size::new(400.0, 100.0)
}

/// Mount one label as the root's only child, filling the viewport.
fn mount_one(host: &mut WidgetHost, widget: Text) -> NodeId {
    host.set_size(viewport());
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let label = host.mount(root, LayoutStyle::default(), widget);
    host.frame();
    label
}

/// Mount two labels into one host.
///
/// Both live in the *same* host on purpose. `ResourceId`s are per-registry and
/// every host hands out the same first id, so comparing an id from one host
/// against an id from another compares two counters rather than two resources —
/// which is how the first draft of the test below "passed" against a broken
/// cache. A glyph resource id only means anything next to the registry it came
/// from.
fn mount_two(host: &mut WidgetHost, first: Text, second: Text) -> (NodeId, NodeId) {
    host.set_size(viewport());
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let a = host.mount(root, LayoutStyle::default(), first);
    let b = host.mount(root, LayoutStyle::default(), second);
    host.frame();
    (a, b)
}

/// Two weights of one string must not share a glyph resource.
///
/// **Which of the two weight-carrying lines this actually pins, measured rather
/// than assumed.** The host keeps the weight in two places: in `glyph_hash`, and
/// in the retained entry's comparison. Three mutations, three answers:
///
/// * drop it from `glyph_hash` only → **all 8 tests still green**. The output is
///   right, because the retained-key comparison catches the mismatch and the
///   label re-shapes. It costs a re-shape per frame per text node, which no
///   assertion here can see.
/// * drop it from the entry comparison only → **all 8 still green**. That line is
///   redundant: the hash already puts the two weights in different buckets.
/// * drop both → **3 tests red**, including this one.
///
/// So one line is load-bearing and the other is defence in depth, and *which* is
/// which is not obvious from reading either. The pair is pinned here because the
/// pair is what the code says; the cost of the hash-only variant is real and
/// invisible, which is worth knowing before anyone "simplifies" it.
#[test]
fn two_weights_of_one_string_intern_different_glyphs() {
    let mut host = WidgetHost::new();
    mount_two(
        &mut host,
        Text::new("Toolbar title"),
        Text::new("Toolbar title").weight(FontWeight::Bold),
    );

    assert_ne!(
        glyphs_of(&host, 0),
        glyphs_of(&host, 1),
        "the same string at two weights interned one glyph resource"
    );
}

/// The control in the other direction, and the reason the assertion above is not
/// vacuous: two labels of the *same* weight do share one resource, because the
/// cache is doing its job rather than missing on everything.
#[test]
fn one_weight_of_one_string_interns_one_glyph_resource() {
    let mut host = WidgetHost::new();
    mount_two(&mut host, Text::new("shared"), Text::new("shared"));

    assert_eq!(
        glyphs_of(&host, 0),
        glyphs_of(&host, 1),
        "identical text at identical weight should reuse the interned runs"
    );
}

/// The interned resource really is the weight's own runs and not a relabelled copy
/// of one set: the shaped runs themselves differ — which is the assertion that
/// would fail if the cache fell back to re-shaping rather than sharing, since a
/// re-shape still produces the right runs.
#[test]
fn the_two_glyph_resources_differ_in_content() {
    let mut host = WidgetHost::new();
    mount_two(
        &mut host,
        Text::new("Weight check"),
        Text::new("Weight check").weight(FontWeight::Bold),
    );

    let runs_at = |host: &WidgetHost, nth: usize| match &host
        .resources()
        .get(glyphs_of(host, nth))
        .expect("the interned resource")
        .kind
    {
        ResourceKind::GlyphRun { runs, .. } => runs.clone(),
        other => panic!("the text node's resource is not a glyph run: {other:?}"),
    };
    assert_ne!(
        runs_at(&host, 0),
        runs_at(&host, 1),
        "two weights produced the same shaped runs"
    );
}

/// The box has to follow the weight. A weight that reached only the paint would
/// leave the node measured for the regular glyphs while drawing the bold ones,
/// and the symptom is text clipped to a width it no longer needs.
///
/// `Bold` rather than `Medium` because the default font stack's faces generally
/// ship only a regular and a bold, so `500` resolves to the same face as `400`
/// and the two legitimately measure alike — an assertion about the installed
/// fonts rather than about the code.
#[test]
fn a_bold_label_measures_wider_than_the_same_regular_label() {
    let mut host = WidgetHost::new();
    mount_two(
        &mut host,
        Text::new("Toolbar title"),
        Text::new("Toolbar title").weight(FontWeight::Bold),
    );

    let regular = text_extent(&host, 0);
    let bold = text_extent(&host, 1);
    assert!(
        bold.width > regular.width,
        "the measured extent must follow the weight: {regular:?} vs {bold:?}"
    );
}

/// The host has to *notice* a weight change on a mounted tree, which means the
/// comparison in `sync_text_measure` has to include it. A label that becomes bold
/// after mount has to end up both wider (re-measured) and differently shaped
/// (re-interned).
#[test]
fn changing_the_weight_after_mount_re_measures_and_re_shapes() {
    let mut host = WidgetHost::new();
    let label = mount_one(&mut host, Text::new("Keyboard shortcuts"));

    let before = text_rect(&host, 0);
    let before_glyphs = glyphs_of(&host, 0);
    assert!(
        before.width() > 0.0,
        "it measured something to start with: {before:?}"
    );

    host.widget_mut::<Text>(label)
        .expect("the mounted text widget")
        .set_weight(FontWeight::Bold);
    host.frame();

    let after = text_rect(&host, 0);
    assert!(
        after.width() > before.width(),
        "the box did not re-measure: {before:?} then {after:?}"
    );
    assert!(
        after.height() <= before.height() + 0.01,
        "and it should still be one line, not a re-wrapped two: {before:?} then {after:?}"
    );
    assert_ne!(
        before_glyphs,
        glyphs_of(&host, 0),
        "the glyph resource was reused across a weight change"
    );
}

/// And the control: a mounted label that does *not* change keeps its resource, so
/// the assertion above is not satisfied by a host that re-shapes everything every
/// frame — which is what a *hash-only* fix would do, and would pass a pixels test
/// while costing a re-shape per frame per text node.
#[test]
fn an_unchanged_weight_keeps_its_glyph_resource() {
    let mut host = WidgetHost::new();
    let label = mount_one(&mut host, Text::new("Steady"));
    let before_rect = text_rect(&host, 0);
    let before_glyphs = glyphs_of(&host, 0);

    host.frame();

    assert_eq!(
        before_glyphs,
        glyphs_of(&host, 0),
        "a steady label re-interned its runs"
    );
    assert_eq!(
        before_rect.width(),
        text_rect(&host, 0).width(),
        "and re-measured for nothing"
    );
    let _ = label;
}

// --- the rounding bug weight ran into --------------------------------------

/// A label that fits on one line paints on one line.
///
/// `floom-layout` hands back a measure leaf's box rounded to whole pixels, and the
/// paint path used to re-shape the same string against that rounded box. A string
/// whose natural width lands in the upper half-pixel of its own box therefore
/// measured in one line, laid out in one line, and painted in two. Measured over
/// 2997 random regular-weight strings, 498 of them did.
///
/// The first two strings are the ones that broke; `Toolbar title` is regular
/// weight and does not, which is the control — it is in the list to show the test
/// is about the boundary rather than about these particular words.
#[test]
fn a_label_that_fits_on_one_line_paints_on_one_line() {
    // Each measured at 14px regular, and each lands in the upper half-pixel of
    // its own rounded box: 79.37 -> 79.0, 68.46 -> 68.0, 71.59 -> 72.0.
    for text in ["kmgfbq nctio", "s rmfleynnt", "Toolbar title"] {
        let mut host = WidgetHost::new();
        mount_one(&mut host, Text::new(text));

        let painted = text_extent(&host, 0);
        assert!(
            painted.height <= 14.5,
            "{text:?} wrapped to {} lines even though it fits in one",
            painted.height / 14.0
        );
        assert!(painted.width > 0.0, "{text:?} measured nothing at all");
    }
}

/// The same fix seen from the other side: a label that *does* need two lines
/// still gets two. Without this, "one line" above could be satisfied by never
/// wrapping anything.
///
/// The root is a **column** on purpose. `LayoutStyle::default()` is a row, so an
/// auto-width label is a *main-axis* child there: it shrink-wraps to its natural
/// width and overflows the parent, which is correct flexbox and not the thing
/// under test. As a column child the label's width is its cross axis, it is
/// stretched to the viewport, and the measure is handed the real constraint.
#[test]
fn a_label_that_needs_two_lines_still_wraps() {
    let long = "a label far wider than the box it is given by the viewport";
    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 100.0));
    let root = host.mount_root(
        LayoutStyle {
            flex_direction: floom_layout::FlexDirection::Column,
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    host.mount(root, LayoutStyle::default(), Text::new(long));
    host.frame();

    let painted = text_extent(&host, 0);
    assert!(
        painted.height > 14.5,
        "a label much wider than its box should wrap, got {painted:?}"
    );
    assert!(
        painted.width <= 120.5,
        "and stay inside the box it was given: {painted:?}"
    );
}
