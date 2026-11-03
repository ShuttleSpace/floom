//! The icon path table and the engine widget built from it.

use floom_types::{ElementId, Rect};

use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext};
use floom_widgets::VectorIcon;

use floom_widget_fluent_icon::{fluent_icon, fluent_icon_path, fluent_icon_svg, VIEW_BOX};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

#[test]
fn a_named_icon_resolves_to_real_path_data() {
    for name in ["menu", "close", "expand_more", "chevron_right"] {
        assert!(
            !fluent_icon_path(name).is_empty(),
            "{name:?} resolves to something"
        );
        let path = fluent_icon_path(name);
        assert!(
            path.chars().any(|c| c.is_ascii_uppercase()),
            "{name:?} carries a path command"
        );
        assert!(!path.contains('<'), "{name:?} is path data, not markup");
    }
}

#[test]
fn names_are_normalised_before_the_lookup() {
    let canonical = fluent_icon_path("expand_more");
    assert_eq!(fluent_icon_path("expandmore"), canonical, "no underscore");
    assert_eq!(fluent_icon_path("expand-more"), canonical, "a hyphen");
    assert_eq!(
        fluent_icon_path("Expand More"),
        canonical,
        "case and a space"
    );
    assert_eq!(fluent_icon_path("EXPAND_MORE"), canonical, "all caps");
}

/// empty box looks like a spacing bug. The cost is that a typo is not a crash.
#[test]
fn an_unknown_name_falls_back_to_a_drawn_shape() {
    let fallback = fluent_icon_path("definitely_not_an_icon");
    assert!(
        !fallback.is_empty(),
        "the fallback is a path, not emptiness"
    );
    assert_ne!(
        fallback,
        fluent_icon_path("close"),
        "and it is not silently some other icon"
    );
}

#[test]
fn the_svg_wrapper_contains_the_same_path() {
    let svg = fluent_icon_svg("close");
    assert!(
        svg.contains("viewBox=\"0 0 24 24\""),
        "the set's own view box"
    );
    assert!(
        svg.contains(fluent_icon_path("close")),
        "and the path the table holds"
    );
}

#[test]
fn the_view_box_is_the_sets_own() {
    assert_eq!(VIEW_BOX, BOX);
}

#[test]
fn the_engine_icon_paints() {
    let icon = fluent_icon("close");
    let scene = paint_snapshot(&icon, ElementId::new(1, 1), BOX);
    assert!(
        scene.contains("Path") || scene.contains("path") || scene.len() > 4,
        "something was drawn: {scene}"
    );
}

#[test]
fn an_unknown_name_still_paints() {
    let icon = fluent_icon("definitely_not_an_icon");
    let scene = paint_snapshot(&icon, ElementId::new(1, 1), BOX);
    assert!(
        scene.contains("Path") || scene.contains("path") || scene.len() > 4,
        "a missing icon draws a dot rather than nothing: {scene}"
    );
}

#[test]
fn the_icon_is_an_engine_vector_icon() {
    fn takes(_: &VectorIcon) {}
    takes(&fluent_icon("menu"));
}
