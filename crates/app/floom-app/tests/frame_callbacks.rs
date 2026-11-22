//! `WidgetHost::on_frame`: the seam event handlers use to reconcile state they
//! cannot reach the host for.

use std::cell::Cell;
use std::rc::Rc;

use floom_app::WidgetHost;
use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_layout::LayoutStyle;
use floom_types::{ElementId, Size};
use floom_widgets::{Axis, Button, Stack, Text};

fn host_with_counter() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 100.0));
    let root = Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("Count: 0")) as Box<dyn WidgetAny>
        }))
        .child(Child::new(ChildKey::index(1), LayoutRequest::new(), || {
            Box::new(Button::new("Increment")) as Box<dyn WidgetAny>
        }));
    host.mount_root(LayoutStyle::default(), root);
    host
}

#[test]
fn a_frame_callback_runs_once_per_frame_and_can_edit_the_tree() {
    let mut host = host_with_counter();
    let calls = Rc::new(Cell::new(0u32));
    let seen = Rc::clone(&calls);
    host.on_frame(move |host| {
        let n = seen.get() + 1;
        seen.set(n);
        if let Some(label) = host.node_at(1)
            && let Some(text) = host.widget_mut::<Text>(label)
        {
            text.set_content(format!("Count: {n}"));
        }
    });

    host.frame();
    assert_eq!(calls.get(), 1, "the callback runs on every frame");
    host.frame();
    assert_eq!(calls.get(), 2);
    let semantics = host.semantics();
    let node = semantics
        .iter()
        .find(|(id, _)| *id == ElementId::new(1, 1))
        .map(|(_, node)| node)
        .expect("the label publishes");
    assert_eq!(node.label, "Count: 2");
}
