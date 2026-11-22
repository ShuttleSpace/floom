// SPDX-License-Identifier: MIT
//! Focus has to be visible, not only tracked.
//!
//! `docs/gates/android.md` recorded that focus moving between two buttons could
//! not be observed on a device: the widget knew it was focused, the semantics
//! tree said so, and nothing on screen changed. The host paints the ring, so
//! the check belongs here rather than in any one design system.

use floom_accessibility::SemanticsRole;
use floom_app::{FocusRing, WidgetHost};
use floom_core::{Axis, Child, ChildKey, LayoutRequest, Length};
use floom_layout::LayoutStyle;
use floom_types::{Color, Size};
use floom_widgets::compose::Stack;
use floom_widgets::controls::Button;

const EDGES: usize = 4;

fn mounted() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 400.0));
    host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(400.0)),
            ..LayoutStyle::default()
        },
        Stack::new(Axis::Vertical).child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Button::new("Save")),
        )),
    );
    host.frame();
    host
}

fn rings_in(host: &WidgetHost, color: floom_types::Color) -> Vec<floom_types::Rect> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            floom_scene::SceneNode::Rect { bounds, fill, .. }
                if *fill == floom_types::Brush::Solid(color) =>
            {
                Some(*bounds)
            }
            _ => None,
        })
        .collect()
}

fn rings(host: &WidgetHost) -> Vec<floom_types::Rect> {
    rings_in(host, FocusRing::DEFAULT.color)
}

fn save(host: &WidgetHost) -> floom_types::ElementId {
    host.semantics()
        .iter()
        .find(|(_, node)| node.role == SemanticsRole::Button)
        .map(|(id, _)| id)
        .expect("the mounted tree holds one button")
}

#[test]
fn focusing_an_element_paints_a_ring_on_its_edges() {
    let mut host = mounted();
    assert!(rings(&host).is_empty(), "nothing is focused yet");

    host.focus_next();
    let id = host.focused().expect("the button takes focus");
    let bounds = host.bounds(host.node_at(id.index() as usize).unwrap());
    host.frame();

    let ring = rings(&host);
    assert_eq!(ring.len(), EDGES, "one rect per edge: {ring:?}");
    let width = FocusRing::DEFAULT.width;
    // The four edges tile the box: same outer extent, and together they cover
    // the border band and nothing in the middle.
    let outer = ring
        .iter()
        .fold(floom_types::Rect::ZERO, |acc, r| floom_types::Rect {
            x0: acc.x0.min(r.x0),
            y0: acc.y0.min(r.y0),
            x1: acc.x1.max(r.x1),
            y1: acc.y1.max(r.y1),
        });
    assert_eq!(outer, bounds, "the ring hugs the element it marks");
    // Each edge spans the box on one axis and is `width` thick on the other.
    assert!(ring.iter().all(|r| {
        let spans_x =
            (r.width() - bounds.width()).abs() < 1e-9 && (r.height() - width).abs() < 1e-9;
        let spans_y =
            (r.height() - bounds.height()).abs() < 1e-9 && (r.width() - width).abs() < 1e-9;
        spans_x || spans_y
    }));
}

#[test]
fn blurring_the_element_retires_the_ring() {
    let mut host = mounted();
    host.focus_next();
    host.frame();
    assert_eq!(rings(&host).len(), EDGES);

    // Pointer focus: the ring is a keyboard affordance, so this must clear it.
    let id = host.focused().expect("still focused");
    host.set_focus(id);
    host.frame();
    assert!(rings(&host).is_empty(), "a blurred element leaves no ring");
}

#[test]
fn a_settled_ring_damages_nothing_and_paints_four_rects() {
    let mut host = mounted();
    host.focus_next();
    host.frame();

    let stats = host.frame();
    assert_eq!(stats.updated, 0, "the second frame changes no payload");
    assert!(host.scene().damage().is_empty(), "and damages no region");
}

#[test]
fn turning_the_ring_off_paints_nothing_and_a_custom_colour_is_used() {
    let mut host = mounted();
    host.focus_next();

    host.set_focus_ring(None);
    host.frame();
    assert!(rings(&host).is_empty());

    let accent = Color::rgb(0x38, 0xA1, 0x69);
    host.set_focus_ring(Some(FocusRing {
        color: accent,
        width: 3.0,
    }));
    host.frame();
    assert_eq!(rings_in(&host, accent).len(), EDGES);
    assert!(rings(&host).is_empty(), "and the default colour is gone");
}
