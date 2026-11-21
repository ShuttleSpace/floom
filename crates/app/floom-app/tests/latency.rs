// SPDX-License-Identifier: MIT
//! P10/P11 — the first frame and the input latency, measured in **paints**.
//!
//! Competitors publish milliseconds (5 ms to first frame, 0.12 ms tap to
//! commit). A headless runner cannot hold those numbers: the clock is noisy and
//! the monitor is not part of the measurement. What is machine-independent is
//! how many frames the answer costs, so that is what is asserted here and the
//! wall clock is printed for the record.
//!
//! The picture, in this framework, is the scene: the paint ops the host hands
//! to whatever presenter is behind it. A tap that reaches the scene has reached
//! the pixels of every backend, and asking for one more frame to see it would
//! be a frame of latency nobody needs to pay.

use std::time::Instant;

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_testkit::TestApp;
use floom_types::{Brush, Color, Length, Size};
use floom_widgets::{Container, Switch};

/// The two rounded rects a switch paints, as `(track fill, knob x0)`.
///
/// Reading them out of the scene rather than out of the widget is the point:
/// the widget's `is_on` has been true before and after a repaint that did not
/// happen, and this is the assertion that catches that.
fn switch_paint(scene: &[SceneNode]) -> (Color, f64) {
    let mut rects = scene.iter().filter_map(|node| match node {
        SceneNode::Rect { bounds, fill, .. } => Some((*bounds, fill.clone())),
        _ => None,
    });
    let (_, track) = rects.next().expect("the switch paints a track");
    let (knob, _) = rects.next().expect("the switch paints a knob");
    let solid = |brush: Brush| match brush {
        Brush::Solid(color) => color,
        other => panic!("a switch paints flat colours, not {other:?}"),
    };
    (solid(track), knob.x0)
}

/// A tap must reach the scene, and it must not cost an extra frame to do it.
#[test]
fn a_tap_reaches_the_picture_without_an_extra_frame() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), Switch::new("Wi-Fi"));
    app.settle();

    let off = switch_paint(&app.scene_nodes());
    assert_eq!(
        off,
        (Color::rgb(0xCB, 0xD5, 0xE0), 0.0),
        "settled, it is off"
    );

    let started = Instant::now();
    app.tap("Wi-Fi");
    let elapsed = started.elapsed();

    let on = switch_paint(&app.scene_nodes());
    assert_eq!(
        on,
        (Color::rgb(0x38, 0xA1, 0x69), 20.0),
        "the tap moved the knob and greened the track"
    );
    assert!(
        !app.host().needs_frame(),
        "the tap served its own frame: nothing is left owing"
    );
    assert!(!app.damaged(), "and it left no damage behind");

    println!("tap to painted scene: {elapsed:?}");
}

/// The first frame of a large tree paints it; the second frame, with nothing
/// changed, paints the same picture again and *updates* nothing.
///
/// The second half matters because the host repaints the whole scene every
/// frame it draws (that is what makes retained-mode bugs impossible), so
/// "settled" cannot mean "painted nothing". It means "updated nothing, damaged
/// nothing" — the frame is cheap, not absent.
#[test]
fn the_first_frame_paints_a_large_tree_and_the_next_updates_nothing() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(320.0, 2_000.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    for _ in 0..2_000 {
        host.mount(
            root,
            LayoutStyle {
                height: Some(Length::Px(1.0)),
                ..LayoutStyle::default()
            },
            Container::new(),
        );
    }

    let started = Instant::now();
    let first = host.frame();
    let elapsed = started.elapsed();

    assert!(
        first.painted >= 2_000,
        "the first frame painted the tree, not just laid it out: {}",
        first.painted
    );
    assert!(!host.needs_frame(), "and left nothing owing");

    let second = host.frame();
    assert_eq!(second.updated, 0, "nothing changed, so nothing updated");
    assert!(
        host.scene().damage().is_empty(),
        "and the second frame damaged nothing"
    );

    println!("first frame of 2,000 rows: {elapsed:?}");
}
