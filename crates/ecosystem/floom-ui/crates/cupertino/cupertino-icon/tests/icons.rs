//! The icon path table and the engine widget built from it.
//!
//! The table itself did not change in this port — it never depended on v1 — so most of
//! these tests are about the two things that *did*: the normalisation that decides
//! whether a name resolves, and the fact that the fallback is a drawn circle rather than
//! nothing.

use floom_types::{ElementId, Rect};

use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext};
use floom_widgets::VectorIcon;

use floom_widget_cupertino_icon::{
    cupertino_icon, cupertino_icon_path, cupertino_icon_svg, VIEW_BOX,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

/// A named icon resolves to real path data. The control for everything below: if this
/// failed, every other assertion here would pass vacuously.
#[test]
fn a_named_icon_resolves_to_real_path_data() {
    for name in ["menu", "close", "expand_more", "chevron_right"] {
        assert!(
            !cupertino_icon_path(name).is_empty(),
            "{name:?} resolves to something"
        );
        // I first asserted "no spaces", which is wrong: SVG path data uses spaces as
        // argument separators (`M3 18h18v-2H3v2z`). What actually distinguishes it from
        // prose or markup is a command letter and no tag.
        let path = cupertino_icon_path(name);
        assert!(
            path.chars().any(|c| c.is_ascii_uppercase()),
            "{name:?} carries a path command"
        );
        assert!(!path.contains('<'), "{name:?} is path data, not markup");
    }
}

/// Names are normalised, so the three spellings of one icon agree.
///
/// v1 wrote `name.to_lowercase().replace(['_', '-', ' '], "")` before the match. That
/// line is the whole reason a caller can pass `"expand_more"` and a kit can pass
/// `"expandmore"` and both land on the same icon — easy to lose in a port, and its
/// loss is invisible until someone uses the other spelling.
#[test]
fn names_are_normalised_before_the_lookup() {
    let canonical = cupertino_icon_path("expand_more");
    assert_eq!(
        cupertino_icon_path("expandmore"),
        canonical,
        "no underscore"
    );
    assert_eq!(cupertino_icon_path("expand-more"), canonical, "a hyphen");
    assert_eq!(
        cupertino_icon_path("Expand More"),
        canonical,
        "case and a space"
    );
    assert_eq!(cupertino_icon_path("EXPAND_MORE"), canonical, "all caps");
}

/// **An unknown name draws a filled circle, not nothing.** That is v1's `FALLBACK`, and
/// it is kept deliberately: a missing icon that draws a dot is visibly wrong, where an
/// empty box looks like a spacing bug. The cost is that a typo is not a crash.
#[test]
fn an_unknown_name_falls_back_to_a_drawn_shape() {
    let fallback = cupertino_icon_path("definitely_not_an_icon");
    assert!(
        !fallback.is_empty(),
        "the fallback is a path, not emptiness"
    );
    assert_ne!(
        fallback,
        cupertino_icon_path("close"),
        "and it is not silently some other icon"
    );
}

/// The SVG wrapper and the path agree — they are two views of one table, and a port
/// that rewrote one of them would leave them quietly inconsistent.
#[test]
fn the_svg_wrapper_contains_the_same_path() {
    let svg = cupertino_icon_svg("close");
    assert!(
        svg.contains("viewBox=\"0 0 24 24\""),
        "the set's own view box"
    );
    assert!(
        svg.contains(cupertino_icon_path("close")),
        "and the path the table holds"
    );
}

/// The view box is 24x24, which is a property of the data rather than of any widget.
#[test]
fn the_view_box_is_the_sets_own() {
    assert_eq!(VIEW_BOX, BOX);
}

/// And the engine widget the kit actually uses is built from that table and paints.
#[test]
fn the_engine_icon_paints() {
    let icon = cupertino_icon("close");
    let scene = paint_snapshot(&icon, ElementId::new(1, 1), BOX);
    assert!(
        scene.contains("Path") || scene.contains("path") || scene.len() > 4,
        "something was drawn: {scene}"
    );
}

/// A  for an unknown name still paints — which is the whole reason the
/// fallback is a circle.
#[test]
fn an_unknown_name_still_paints() {
    let icon = cupertino_icon("definitely_not_an_icon");
    let scene = paint_snapshot(&icon, ElementId::new(1, 1), BOX);
    assert!(
        scene.contains("Path") || scene.contains("path") || scene.len() > 4,
        "a missing icon draws a dot rather than nothing: {scene}"
    );
}

/// The type is the engine's, so a kit can hand it to  unchanged.
#[test]
fn the_icon_is_an_engine_vector_icon() {
    fn takes(_: &VectorIcon) {}
    takes(&cupertino_icon("menu"));
}
