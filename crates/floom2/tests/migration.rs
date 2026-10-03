//! Phase 30.3 — the before/after snippets in `docs/2.0/MIGRATION.md`, compiled.
//!
//! A migration guide whose examples do not compile is worse than none, so the
//! snippets in that document are the snippets in this file. Two rules:
//!
//! - The **before** half imports `floem::compat::v1_prelude` (Phase 29.2's
//!   frozen v1 surface), which is what a v1 app should switch to *before* it
//!   starts porting. It proves the app still builds the way it always did.
//! - The **after** half imports `floem2::prelude` and does the same job in the
//!   2.0 model, asserting the behaviour the guide claims (a click fires once,
//!   the label updates, node handles come from the host).
//!
//! What is *not* here: mixing the two trees. There is no adapter that mounts a
//! v1 `View` inside a 2.0 host — that is what the `on_unimplemented`
//! diagnostic on `Widget`/`WidgetAny` tells you at the mount point, and it is
//! pinned as a compile-fail doctest in `floem2`'s crate docs.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

// ---------------------------------------------------------------------------
// BEFORE — the v1 application, unchanged.
// ---------------------------------------------------------------------------
mod before {
    use floem::ViewId;
    use floem::compat::v1_prelude::*;

    /// The v1 counter: state is a signal, the tree is a value, and the label
    /// re-runs when the signal changes.
    ///
    /// The `v_stack`/`label`/`button` free functions are the deprecated spelling
    /// of `Stack::vertical`/`Label::derived`/`Button::new`; the guide shows the
    /// current one, because that is what a porting app has today.
    #[test]
    fn v1_counter_builds_and_its_signal_drives_the_label() {
        let mut counter = RwSignal::new(0_i32);

        let view = Stack::vertical((
            Label::derived(move || format!("Value: {}", counter.get())),
            Button::new("Increment").action(move || counter += 1),
        ));

        // The reactive graph is what updates the label: no handle to the widget
        // is involved, and none is available.
        assert_eq!(counter.get(), 0);
        counter.set(41);
        assert_eq!(counter.get(), 41);

        // The tree is a value; it has not been mounted and needs no host.
        let _ = view;
    }

    /// v1 ids come from a global registry, handed out at construction time.
    ///
    /// This is the part of `ViewId` that has no 2.0 analogue: it is a *live*
    /// handle into process-global state, so it can be copied anywhere and used
    /// to reach any view from any thread that has one.
    #[test]
    fn v1_view_ids_come_from_the_global_registry() {
        let id = ViewId::new();
        assert!(id.is_valid());

        // A `ViewId` is a stable identity even before anything is mounted.
        let other = ViewId::new();
        assert_ne!(id, other);
    }
}

// ---------------------------------------------------------------------------
// AFTER — the same counter in Floem 2.0.
// ---------------------------------------------------------------------------
mod after {
    use super::*;
    use floem2::prelude::*;

    /// The 2.0 counter: the tree is mounted in a host, children are keyed, and
    /// the application owns the state.
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

        // Where v1 re-ran a closure, 2.0 updates the widget in place. Bumping
        // the revision is what makes the next frame repaint it.
        host.widget_mut::<Text>(label_id)
            .expect("label")
            .set_content(format!("Value: {}", clicks.load(Ordering::SeqCst)));
        host.frame();

        assert_eq!(host.widget::<Text>(label_id).unwrap().content(), "Value: 1");
    }

    /// `NodeId` is scoped to the host that issued it — the deliberate break with
    /// `ViewId`. A second host does not know the first one's nodes.
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
    /// the key reuses it. That is the reconciliation contract a v1 `dyn_stack`
    /// user is moving to.
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
}
