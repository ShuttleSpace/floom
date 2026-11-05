// SPDX-License-Identifier: MIT
//! Verify the reactive rebuild seam: a signal change drives an effect that asks
//! the host to re-roll the tree, re-running the build closures so fresh signal
//! reads replace the mounted widgets.

use std::sync::Arc;

use floom_app::WidgetHost;
use floom_core::{
    Axis, Event, EventContext, LayoutRequest, Point, Size, Widget, WidgetAny, request_rebuild_root,
};
use floom_core::SemanticsHint;
use floom_layout::LayoutStyle;
use floom_signal::{Effect, SignalGet, SignalUpdate, SyncBaseSignal};
use floom_types::{PointerButton, PointerPhase};
use floom_widgets::{Stack, Text};

/// Reads `Text` content from the first text node in the tree.
fn first_text(host: &WidgetHost) -> String {
    for index in 0..host.node_count() {
        let node = host.node_at(index).expect("node at index");
        if let Some(text) = host.widget::<Text>(node) {
            return text.content().to_string();
        }
    }
    panic!("no text node found");
}

#[test]
fn signal_change_rebuilds_the_tree_and_updates_text() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));

    // The signal lives behind an `Arc`, mirrored by the generated form's
    // `Arc<SyncBaseSignal<_>>`.
    let value = Arc::new(SyncBaseSignal::new(0));

    // An effect that tracks the signal and, on every change after the first run,
    // asks the host to re-roll the whole tree — the closure-capture seam.
    let tracked = value.clone();
    Effect::new({
        let tracked = tracked.clone();
        move |prev: Option<i32>| -> i32 {
            if prev.is_some() {
                request_rebuild_root();
            }
            tracked.get()
        }
    });

    // The root reads the signal through `.get()` inside its child closure.
    let reader = value.clone();
    let root = host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical).push(move || {
            let reader = reader.clone();
            Box::new(Text::new(format!("count: {}", reader.get())))
        }),
    );
    let _ = root;

    // Mount frame: initial render.
    host.frame();
    assert_eq!(first_text(&host), "count: 0");

    // Flip the signal. The effect fires and queues a rebuild; the next frame
    // re-rolls the tree and the text reflects the new value.
    value.set(7);
    host.frame();
    assert_eq!(first_text(&host), "count: 7");

    // A second change keeps working.
    value.update(|v| *v += 3);
    host.frame();
    assert_eq!(first_text(&host), "count: 10");
}

/// A leaf that mirrors a `Slider`'s drag contract in miniature: it captures the
/// pointer on press and holds a `pressed` flag that must survive a reactive
/// rebuild mid-drag. A rebuild that swaps the widget would drop the flag and the
/// drag would stop tracking after the first move.
struct DragProbe {
    pressed: bool,
}

impl Widget for DragProbe {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn floom_core::UpdateContext, _id: floom_core::ElementId) {}

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().width(120.0).height(24.0))
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: floom_core::ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.pressed = true;
                cx.request_pointer_capture(id);
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                self.pressed = false;
                true
            }
            PointerPhase::Move => self.pressed,
            _ => false,
        }
    }

    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: floom_core::ElementId) {}
}

impl WidgetAny for DragProbe {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        None
    }
}

/// A captured widget must keep its state across a reactive rebuild: otherwise a
/// drag (Slider, canvas, …) that changes a signal every move would be torn down
/// by its own rebuild and stop tracking. Regression guard for the Slider-during-
/// drag bug.
#[test]
fn captured_widget_survives_reactive_rebuild() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));

    // A signal whose effect drives a rebuild; flipping it stands in for the
    // signal a drag writes on every move.
    let trigger = Arc::new(SyncBaseSignal::new(0));
    let tracked = trigger.clone();
    Effect::new({
        let tracked = tracked.clone();
        move |prev: Option<i32>| -> i32 {
            if prev.is_some() {
                request_rebuild_root();
            }
            tracked.get()
        }
    });

    let root = host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical).push(|| Box::new(DragProbe { pressed: false })),
    );
    host.frame();

    // Press on the probe to start a "drag" and capture the pointer.
    let probe = host.node_at(1).expect("probe is the second node");
    let bounds = host.bounds(probe);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(Event::Pointer(
        floom_types::PointerEvent::primary_down(centre),
    ));
    assert!(
        host.widget::<DragProbe>(probe).expect("probe").pressed,
        "press sets the captured widget's pressed flag"
    );

    // The drag moves: flip the signal so a rebuild is queued, then run the frame.
    // The probe must NOT be rebuilt, so its pressed flag survives.
    trigger.set(1);
    host.frame();
    assert!(
        host.widget::<DragProbe>(probe).expect("probe").pressed,
        "a captured widget is preserved across a reactive rebuild, so the drag keeps tracking"
    );

    // Releasing clears capture and the flag, even though the tree is rebuilt.
    host.dispatch(Event::Pointer(
        floom_types::PointerEvent::primary_up(centre),
    ));
    assert!(
        !host.widget::<DragProbe>(probe).expect("probe").pressed,
        "release still ends the drag after the rebuild"
    );

    let _ = root;
}
