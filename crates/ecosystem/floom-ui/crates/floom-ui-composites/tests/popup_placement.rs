//! The popup's placement: the one decision a `Popup` makes that is not plumbing.
//!
//! Everything else about a popup — asking the host, learning its handle, being
//! dismissed from outside, falling back to an in-tree layer — is `OverlaySurface`
//! and is covered end-to-end against the real host in `floem-app/tests/overlay.rs`.
//! What is new here is *where the panel goes*, and these tests are about that and
//! nothing else.
//!
//! They run against `RecordingUpdateContext`'s overlay door, which did not exist
//! before this change. A composite that mounts its panel from `update` is exactly
//! what a popup does, and with the door defaulting to `false` a test could not tell a
//! deliberate in-tree fallback from a missing implementation.

use floom_core::{ChildKey, LayoutRequest, Widget, WidgetAny};
use floom_types::{OverlayKind, Rect};

use floom_ui_composites::Popup;
use floom_ui_composites::{popup_placement_rect, PopupPlacement};
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};
use floom_widgets::Text;

/// A control in the middle of nowhere, so a test that "looks right" has to mean it.
const ANCHOR: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 140.0,
};

fn below(gap: f64, height: f64) -> PopupPlacement {
    PopupPlacement::Below { gap, height }
}

// --- the geometry, on its own ---------------------------------------------

/// The control group for this whole file: `Fill` asks for the anchor's own box.
/// Every popup that existed before placement was added asked for exactly this, so
/// this assertion is what proves the new field changed nothing by default.
#[test]
fn fill_asks_for_the_controls_own_box() {
    assert_eq!(
        popup_placement_rect(PopupPlacement::Fill, ANCHOR),
        ANCHOR,
        "unchanged behaviour: the panel covers the control"
    );
}

/// And the default really is `Fill`, so a `Popup` nobody configured has not changed.
#[test]
fn the_default_placement_is_fill() {
    assert_eq!(PopupPlacement::default(), PopupPlacement::Fill);
}

/// A dropdown hangs off the bottom edge, not over it.
#[test]
fn below_hangs_off_the_bottom_edge() {
    let rect = popup_placement_rect(below(4.0, 100.0), ANCHOR);
    assert_eq!(rect.y0, 144.0, "the anchor's bottom (140) plus the gap (4)");
    assert_eq!(rect.y1, 244.0, "and the panel is as tall as it asked to be");
    assert_eq!(rect.height(), 100.0, "which is a real bound, not a hint");
}

/// The panel stays aligned with the field it belongs to.
#[test]
fn below_keeps_the_controls_width_and_horizontal_position() {
    let rect = popup_placement_rect(below(4.0, 100.0), ANCHOR);
    assert_eq!(
        rect.x0, ANCHOR.x0,
        "the panel starts where the field starts"
    );
    assert_eq!(rect.x1, ANCHOR.x1, "and is exactly as wide as it");
}

/// The opposite edge, for a control near the bottom of the window.
#[test]
fn above_hangs_off_the_top_edge() {
    let rect = popup_placement_rect(
        PopupPlacement::Above {
            gap: 4.0,
            height: 100.0,
        },
        ANCHOR,
    );
    assert_eq!(rect.y1, 96.0, "the anchor's top (100) less the gap (4)");
    assert_eq!(rect.y0, -4.0, "and the panel grows upwards from there");
    assert_eq!(rect.height(), 100.0);
}

/// A panel must not overlap the control it is attached to. With no gap it *touches*,
/// which is the floor; with a gap there is daylight.
#[test]
fn a_panel_never_overlaps_its_control() {
    for gap in [0.0, 1.0, 4.0, 40.0] {
        let below = popup_placement_rect(below(gap, 100.0), ANCHOR);
        assert!(
            below.y0 >= ANCHOR.y1,
            "below with gap {gap} starts at {} but the control ends at {}",
            below.y0,
            ANCHOR.y1
        );
        let above = popup_placement_rect(PopupPlacement::Above { gap, height: 100.0 }, ANCHOR);
        assert!(
            above.y1 <= ANCHOR.y0,
            "above with gap {gap} ends at {} but the control starts at {}",
            above.y1,
            ANCHOR.y0
        );
    }
}

/// The gap is *added* to whichever edge, so a caller who passes the wrong sign
/// overlaps the control rather than pushing the panel off-screen — which is the
/// failure worth catching, because it is the one that hides the field.
#[test]
fn the_gap_moves_the_panel_away_rather_than_under_it() {
    let negative = popup_placement_rect(below(-8.0, 100.0), ANCHOR);
    assert_eq!(
        negative.y0, 132.0,
        "a negative gap pulls the panel up, *into* the control — which is what a \
         wrong-signed token does, and is why it is pinned here rather than clamped"
    );
    assert!(
        negative.y0 < ANCHOR.y1,
        "the panel now overlaps the control: the bug this makes visible"
    );
}

/// Placement is orthogonal to size: a very wide, very short anchor still produces a
/// panel of exactly the control's width. (A control group against an implementation
/// that measured the anchor's *height* instead of using its bottom edge.)
#[test]
fn placement_uses_the_edges_and_not_the_size() {
    let tall = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 80.0,
        y1: 600.0,
    };
    let rect = popup_placement_rect(below(0.0, 50.0), tall);
    assert_eq!(rect.y0, 600.0, "the bottom edge, not the top and not zero");
    assert_eq!(
        rect.height(),
        50.0,
        "and the requested height, not the anchor's"
    );
}

// --- through the widget, to the host ---------------------------------------

/// A popup that opens asks the host for the placed rect.
///
/// This is the test that earns the placement field: the pure function above could
/// be right while the widget ignored it, and the widget is what a combobox uses.
#[test]
fn an_opening_popup_asks_for_the_placed_rect() {
    let mut popup = Popup::new().placement(below(4.0, 100.0)).content_with(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("row")) as Box<dyn WidgetAny>,
    );
    popup.set_present(true);

    let id = popup.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    popup.update(&mut cx, id);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one overlay asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open {
            kind,
            modal,
            bounds,
            ..
        } => {
            assert_eq!(*kind, OverlayKind::Popup, "a popup, not a dialog");
            assert!(
                !modal,
                "and never modal — a dropdown is dismissed, not blocking"
            );
            assert_eq!(
                *bounds,
                popup_placement_rect(below(4.0, 100.0), ANCHOR),
                "and the rect is the placed one, not the control's own box"
            );
        }
        _ => panic!("expected an open overlay, got a close"),
    }
}

/// The control group for the test above: the same widget with no placement asks for
/// the control's own box. Without this, "the rect is the placed one" could be passing
/// because `Fill` and `Below` happen to agree on this anchor.
#[test]
fn an_opening_popup_with_no_placement_asks_for_its_own_box() {
    let mut popup = Popup::new().content_with(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("row")) as Box<dyn WidgetAny>
    });
    popup.set_present(true);

    let id = popup.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    popup.update(&mut cx, id);

    match cx.opened_overlays()[0] {
        floom_widgets::testkit::OverlayCall::Open { bounds, .. } => assert_eq!(
            *bounds, ANCHOR,
            "an unconfigured popup still covers its control"
        ),
        _ => panic!("expected an open overlay, got a close"),
    }
}

/// The placement is readable back, so a kit that composes one can assert on it the
/// way it asserts on every other token it resolved.
#[test]
fn a_popup_reports_the_placement_it_was_given() {
    let popup = Popup::new().placement(below(4.0, 100.0));
    assert_eq!(popup.current_placement(), below(4.0, 100.0));
    assert_eq!(Popup::new().current_placement(), PopupPlacement::Fill);
}

/// A popup that was never asked to present asks the host for nothing.
#[test]
fn a_closed_popup_asks_for_nothing() {
    let mut popup = Popup::new().placement(below(4.0, 100.0)).content_with(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("row")) as Box<dyn WidgetAny>,
    );
    let id = popup.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    popup.update(&mut cx, id);
    assert!(
        !cx.opened_any_overlay(),
        "no overlay for a popup nobody opened"
    );
}
