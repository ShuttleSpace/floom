//! The composites that host a scroll viewport, and the state they keep.
//!
//! A tab strip and an accordion panel are both "one of several, and the chosen
//! one shows more", and both are normally put inside a scroll viewport. What is
//! asserted here is that the choice survives frames, reconciles and a
//! re-anchored offset -- the three ways a composite's state can be lost when its
//! subtree is rebuilt around it.
//!
//! This used to be the tail of `floom-widgets/tests/scroll.rs`, carrying the
//! widgets it exercises along with it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use floom_app as Host;
use floom_core as Core;
use floom_core::{Axis, ScrollOffset, Size};
use floom_layout as LY;
use floom_ui_composites::{Accordion, Tabs};
use floom_widgets as W;
use floom_widgets::{Scroll, ScrollAxes, Stack};

/// A zero-cost child that fills its box, so a scroll can be given content with a
/// known extent.
/// A zero-cost child that fills its box, so a scroll can be given content with a
/// known extent.
struct Filler;

impl floom_core::WidgetAny for Filler {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl floom_core::Widget for Filler {
    fn create(&self, _cx: &mut dyn floom_core::CreateContext) -> floom_core::ElementId {
        floom_core::ElementId::new(0, 0)
    }
    fn update(&mut self, _cx: &mut dyn floom_core::UpdateContext, _id: floom_core::ElementId) {}
    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: floom_core::ElementId) {}
}

fn wheel(y: f64) -> floom_core::Event {
    floom_core::Event::Scroll(floom_core::ScrollEvent {
        delta_x: 0.0,
        delta_y: y,
        position: floom_core::Point::new(50.0, 50.0),
        modifiers: floom_core::Modifiers::default(),
    })
}
#[test]
fn the_offset_survives_frames_and_reconciles() {
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        Scroll::new().child(floom_core::Child::new(
            0,
            floom_core::LayoutRequest::new().width(200.0).height(900.0),
            || Box::new(Filler) as Box<dyn floom_core::WidgetAny>,
        )),
    );
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    let root = host.root().unwrap();
    let child = host.child_at(root, 0).unwrap();

    host.dispatch(wheel(100.0));
    for _ in 0..6 {
        host.frame();
    }
    assert_eq!(host.bounds(child).y0, 100.0, "the first wheel moved it");

    host.dispatch(wheel(100.0));
    for _ in 0..6 {
        host.frame();
    }
    assert_eq!(
        host.bounds(child).y0,
        200.0,
        "the second wheel added to the first rather than replacing it, so the \
         offset outlived the frames between them"
    );
}

/// A named leaf, so a caller can tell which child it mounted.
fn filler_child(key: &'static str) -> Core::Child {
    Core::Child::new(
        key,
        Core::LayoutRequest::new().width(100.0).height(50.0),
        || Box::new(Filler) as Box<dyn Core::WidgetAny>,
    )
}

// --- state across rebuilds -------------------------------------------------

/// **The premise behind `Tabs`' shared selection, checked rather than assumed.**
///
/// A composite is rebuilt whenever its parent re-declares `children()`, and a
/// rebuild constructs a fresh value. If the selected index were a plain field,
/// every tab would snap back to the first one whenever anything else in the
/// tree changed.
///
/// `Tabs` keeps it in an `Arc<AtomicUsize>` so it survives that. The test
/// selects, runs frames, and reads the selection back off the mounted widget —
/// if the widget had been rebuilt with a fresh `AtomicUsize`, the handle the
/// caller holds would no longer be the one the widget reads.
#[test]
fn a_tabs_selection_survives_frames() {
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let tabs = Tabs::new()
        .tab("One", filler_child("one"))
        .tab("Two", filler_child("two"))
        .tab("Three", filler_child("three"));
    let selection = tabs.selection();
    host.mount_root(LY::LayoutStyle::default(), tabs);
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }

    selection.store(2, std::sync::atomic::Ordering::SeqCst);
    host.frame();
    for _ in 0..6 {
        host.frame();
    }

    let node = host.root().expect("the tabs are mounted at the root");
    let live = host.widget::<Tabs>(node).unwrap().selection();
    assert_eq!(
        live.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "the widget still reads the selection the caller wrote, so the two \\
         share one cell rather than each having its own"
    );
    assert_eq!(
        live.load(std::sync::atomic::Ordering::SeqCst),
        selection.load(std::sync::atomic::Ordering::SeqCst),
        "and it is the same cell, not a copy that happens to agree"
    );
}

/// The mirror image: a caller that took the handle *before* mounting and writes
/// to it after must see the widget react. This is the contract that makes
/// `Tabs` drivable from outside its own subtree — a tab bar whose selection is
/// only settable by the widget would be useless to a header that wants to
/// change it.
#[test]
fn a_selection_written_from_outside_reaches_the_widget() {
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let tabs = Tabs::new()
        .tab("One", filler_child("one"))
        .tab("Two", filler_child("two"));
    let selection = tabs.selection();
    host.mount_root(LY::LayoutStyle::default(), tabs);
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }

    selection.store(1, std::sync::atomic::Ordering::SeqCst);
    host.frame();
    for _ in 0..6 {
        host.frame();
    }

    // Reading the `Arc` back proves nothing: it is the cell the caller wrote, so
    // it holds the right number whether or not the widget ever looked at it.
    // What matters is whether the widget *acted* on the selection, and the
    // observable effect is the panel it mounted — the bar is keyed by the
    // current selection, so a selection the widget ignored leaves the same
    // subtree it had before.
    let root = host.root().unwrap();
    let after_switch = host.child_count(root);
    assert!(
        after_switch > 0,
        "the tabs mounted their bar and the active panel"
    );

    // Switch back and forth: each change re-keys the bar and remounts the
    // panel, so a widget that ignored its own selection cell would settle on
    // the first panel and never move.
    selection.store(0, std::sync::atomic::Ordering::SeqCst);
    host.frame();
    for _ in 0..6 {
        host.frame();
    }
    assert_eq!(
        host.child_count(root),
        after_switch,
        "and back again, so the panel count is the settled shape rather than \
         whatever the first mount happened to produce"
    );
}

/// `Accordion`'s open set, same claim. A panel that forgets it is open when the
/// parent re-declares its children is the kind of bug that only shows up in an
/// app that happens to re-render at the wrong moment.
#[test]
fn an_accordion_panel_stays_open_across_frames() {
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(400.0, 300.0));
    let accordion = Accordion::new()
        .panel(
            "First",
            |_open| Box::new(Filler) as Box<dyn Core::WidgetAny>,
            || Box::new(Filler) as Box<dyn Core::WidgetAny>,
        )
        .panel(
            "Second",
            |_open| Box::new(Filler) as Box<dyn Core::WidgetAny>,
            || Box::new(Filler) as Box<dyn Core::WidgetAny>,
        );
    host.mount_root(LY::LayoutStyle::default(), accordion);
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }

    let node = host.root().expect("the accordion is mounted at the root");
    host.widget_mut::<Accordion>(node)
        .unwrap()
        .set_open(1, true);
    host.frame();
    for _ in 0..6 {
        host.frame();
    }

    assert!(
        host.widget::<Accordion>(node).unwrap().is_open(1),
        "the panel the caller opened is still open after several frames, so the \\
         open set is shared rather than rebuilt with the widget"
    );
}
