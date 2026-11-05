// SPDX-License-Identifier: MIT
//! Host mount / click / key behaviour, exercised as integration tests.
//!
//! These cover the parts of the widget-host contract that are easy to get
//! wrong: state lives in the app, not a global reactive graph; a `NodeId` is
//! scoped to the host that issued it; children are addressed by a stable key.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use floom::prelude::*;

/// The counter: the tree is mounted in a host, children are keyed, and the
/// application owns the state.
#[test]
fn the_same_counter_mounts_clicks_and_updates() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(320.0, 120.0));

    // State is an ordinary value the app owns — no global reactive graph.
    let clicks = Arc::new(AtomicI32::new(0));

    let counter = clicks.clone();
    let root = Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("Value: 0"))
        }))
        .child(Child::new(
            ChildKey::index(1),
            LayoutRequest::new(),
            move || {
                let counter = counter.clone();
                Box::new(Button::new("Increment").on_click(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                }))
            },
        ));

    // `mount_root` hands back a `NodeId`: a handle from *this* host, not
    // from a registry. It is meaningless to any other host.
    let root_id = host.mount_root(LayoutStyle::default(), root);
    host.frame();
    assert!(host.is_mounted(root_id));

    let mut label_id = None;
    let mut button_id = None;
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if host.widget::<Text>(node).is_some() {
            label_id = Some(node);
        }
        if host.widget::<Button>(node).is_some() {
            button_id = Some(node);
        }
    }
    let label_id = label_id.expect("mounted label");
    let button_id = button_id.expect("mounted button");

    // The click: press and release over the button's centre.
    let bounds = host.bounds(button_id);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(PointerEvent::primary_down(centre).into());
    host.dispatch(PointerEvent::primary_up(centre).into());
    host.frame();
    assert_eq!(clicks.load(Ordering::SeqCst), 1, "one click, one callback");

    // Where a closure used to re-run, the widget updates in place. Bumping
    // the revision is what makes the next frame repaint it.
    host.widget_mut::<Text>(label_id)
        .expect("label")
        .set_content(format!("Value: {}", clicks.load(Ordering::SeqCst)));
    host.frame();

    assert_eq!(host.widget::<Text>(label_id).unwrap().content(), "Value: 1");
}

/// `NodeId` is scoped to the host that issued it. A second host does not know
/// the first one's nodes.
#[test]
fn node_ids_are_scoped_to_their_host() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(100.0, 50.0));
    let root = host.mount_root(LayoutStyle::default(), Text::new("one"));
    host.frame();

    let mut other = WidgetHost::new();
    other.set_size(Size::new(100.0, 50.0));
    other.frame();

    assert!(host.is_mounted(root), "the issuing host knows the node");
    assert!(
        !other.is_mounted(root),
        "a NodeId from one host is not a node of another"
    );
}

/// Children are keyed: replacing a child's key replaces the widget, keeping
/// the key reuses it.
#[test]
fn children_are_addressed_by_stable_key() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 100.0));

    let root = Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("first"))
        }))
        .child(Child::new(ChildKey::index(1), LayoutRequest::new(), || {
            Box::new(Text::new("second"))
        }));
    let root_id = host.mount_root(LayoutStyle::default(), root);
    host.frame();

    let texts: Vec<String> = (0..host.node_count())
        .filter_map(|i| host.node_at(i))
        .filter_map(|n| host.widget::<Text>(n))
        .map(|t| t.content().to_string())
        .collect();
    assert_eq!(texts, vec!["first".to_string(), "second".to_string()]);

    // Re-keying child 1 makes it a different child: the old widget is gone.
    host.widget_mut::<Stack>(root_id).unwrap();
    host.frame();
    assert_eq!(host.node_count(), 3, "root plus two keyed children");
}
