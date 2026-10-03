//! Core accessibility tree builder.
//!
//! Translates Floom's live view tree, per-view declared attributes, layout
//! geometry and focus state into a neutral, diffable
//! [`AccessTreeUpdate`]. Platform backends consume that update (see
//! [`crate::accessibility::AccessibilityBackend`]) to drive the native screen
//! reader. The design mirrors the embedder subsystem (`crates/floom/src/embedder/`): a
//! data-oriented protocol plus a core adapter, independent of any windowing
//! library.
//!
//! This module deliberately knows **nothing** about winit, macOS/iOS
//! `NSAccessibility`, AT-SPI2 or UIA. It only folds view/layout/focus state
//! into the protocol defined in the sibling modules.
//!
//! ## Traversal model
//!
//! The accessibility tree is keyed by [`ViewId`] (each view = exactly one
//! node). We walk `ViewStorage::children` starting from the window root,
//! reading the declared [`crate::accessibility::AccessibilityAttrs`] off each
//! view's [`crate::view::ViewState`] and its layout box
//! (`LayoutChanged::box_window`) for geometry. A view marked
//! `AccessibilityAttrs::hidden` is dropped and its children are re-parented to
//! the nearest exposed ancestor (mirroring ARIA `role="presentation"`).

use rustc_hash::FxHashSet;
use slotmap::Key;

use crate::{
    ViewId,
    accessibility::{AccessNode, AccessNodeId, AccessState, AccessTreeUpdate},
    view::ViewStorage,
};

/// Build a neutral [`AccessTreeUpdate`] for the given window root.
///
/// `storage` is the live [`ViewStorage`]; `root_view_id` the window's root
/// view; `focused_view` the currently focused view (or `None`). The returned
/// update carries every exposed node (as an upsert), the tree root, and the
/// focused node id when `focused_view` is `Some`.
///
/// Node ids are the view's `KeyData` packed as `u64`, so they are stable
/// across updates for the same logical view.
pub(crate) fn build_access_tree_update(
    storage: &ViewStorage,
    root_view_id: ViewId,
    focused_view: Option<ViewId>,
) -> AccessTreeUpdate {
    let mut nodes = Vec::new();
    let mut visited: FxHashSet<ViewId> = FxHashSet::default();
    // The root has no parent; its own id is pushed here but discarded.
    let mut root_children = Vec::new();
    let root = collect(
        storage,
        root_view_id,
        &mut root_children,
        &mut nodes,
        focused_view,
        &mut visited,
    );

    let mut update = AccessTreeUpdate::new();
    if let Some(root) = root {
        update = update.with_root(root);
    }
    update = update.with_nodes(nodes);
    if let Some(fv) = focused_view {
        update = update.with_focus(node_id(fv));
    }
    update
}

/// Recursive subtree walk.
///
/// Emits the node for `view_id` (unless it is hidden) into `nodes` and pushes
/// its [`AccessNodeId`] onto `parent_children` — the child list of the nearest
/// exposed ancestor. A hidden (decorative / presentational) view is not
/// emitted; instead its children are re-parented by pushing their ids directly
/// onto `parent_children` (mirroring ARIA `role="presentation"` / `none`).
fn collect(
    storage: &ViewStorage,
    view_id: ViewId,
    parent_children: &mut Vec<AccessNodeId>,
    nodes: &mut Vec<AccessNode>,
    focused_view: Option<ViewId>,
    visited: &mut FxHashSet<ViewId>,
) -> Option<AccessNodeId> {
    // Cycle guard: a view must never be visited twice.
    if !visited.insert(view_id) {
        return None;
    }

    let state_rc = storage.states.get(view_id)?;
    let state = state_rc.borrow();
    let attrs = state.accessibility.as_ref();

    let child_ids: Vec<ViewId> = storage
        .children
        .get(view_id)
        .cloned()
        .unwrap_or_default();

    // Decorative / presentational view: drop it and re-parent its children to
    // the nearest exposed ancestor (`parent_children`).
    if attrs.map(|a| a.hidden).unwrap_or(false) {
        for child in child_ids {
            collect(storage, child, parent_children, nodes, focused_view, visited);
        }
        return None;
    }

    let access_id = node_id(view_id);
    let role = attrs.map(|a| a.role).unwrap_or_default();
    let mut node = AccessNode::new(access_id, role);

    if let Some(a) = attrs {
        if let Some(label) = &a.label {
            node = node.with_label(label.clone());
        }
        if let Some(value) = &a.value {
            node = node.with_value(value.clone());
        }
        if let Some(description) = &a.description {
            node = node.with_description(description.clone());
        }
        if !a.actions.is_empty() {
            node = node.with_actions(a.actions.iter().cloned());
        }
    }

    let mut states = AccessState::empty();
    if Some(view_id) == focused_view {
        states |= AccessState::FOCUSED;
    }
    if states != AccessState::empty() {
        node = node.with_states(states);
    }

    if let Some(layout) = &state.layout {
        node = node.with_bounds(layout.box_window());
    }

    // Recurse first so child AccessNodeIds are known; a hidden child returns
    // `None` and is not pushed here, but its re-parented descendants were
    // already pushed into `my_children` by the recursive call below.
    let mut my_children = Vec::with_capacity(child_ids.len());
    for child in child_ids {
        collect(storage, child, &mut my_children, nodes, focused_view, visited);
    }

    node = node.with_children(my_children);
    nodes.push(node);
    parent_children.push(access_id);
    Some(access_id)
}

/// Map a [`ViewId`] to a stable [`AccessNodeId`].
#[inline]
fn node_id(id: ViewId) -> AccessNodeId {
    AccessNodeId(id.data().as_ffi())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use peniko::kurbo::{Point, Rect};

    use crate::{
        BoxTree, ViewId,
        accessibility::{AccessibilityAttrs, AccessRole, AccessState, AccessTree},
        context::LayoutChanged,
        view::state::ViewState,
        view::ViewStorage,
    };
    use super::*;

    /// Build a `ViewStorage` with a small tree:
    ///
    /// ```text
    /// root(Window, "Main", 0..100)        <- focused child = button
    ///   ├─ button(Button, "OK", 10..50)   <- FOCUSED
    ///   └─ hidden(role ignored)
    ///        └─ text(Text, "Hi", 10..60)  <- re-parented to root
    /// ```
    ///
    /// Returns `(storage, root, button, hidden, text)`.
    fn fixture() -> (ViewStorage, ViewId, ViewId, ViewId, ViewId) {
        let mut storage = ViewStorage::new();
        let mut box_tree = BoxTree::default();

        let root = new_state(&mut storage, &mut box_tree);
        let button = new_state(&mut storage, &mut box_tree);
        let hidden = new_state(&mut storage, &mut box_tree);
        let text = new_state(&mut storage, &mut box_tree);

        set_attrs(
            &storage,
            root,
            AccessibilityAttrs {
                role: AccessRole::Window,
                label: Some("Main".into()),
                ..Default::default()
            },
        );
        set_bounds(&storage, root, Rect::new(0.0, 0.0, 100.0, 100.0));

        set_attrs(
            &storage,
            button,
            AccessibilityAttrs {
                role: AccessRole::Button,
                label: Some("OK".into()),
                actions: vec![crate::accessibility::AccessAction::Default],
                ..Default::default()
            },
        );
        set_bounds(&storage, button, Rect::new(10.0, 10.0, 50.0, 30.0));

        set_attrs(
            &storage,
            hidden,
            AccessibilityAttrs {
                role: AccessRole::Group,
                hidden: true,
                ..Default::default()
            },
        );

        set_attrs(
            &storage,
            text,
            AccessibilityAttrs {
                role: AccessRole::Text,
                label: Some("Hi".into()),
                ..Default::default()
            },
        );
        set_bounds(&storage, text, Rect::new(10.0, 50.0, 50.0, 20.0));

        storage.children.insert(root, vec![button, hidden]);
        storage.parent.insert(button, Some(root));
        storage.children.insert(hidden, vec![text]);
        storage.parent.insert(text, Some(hidden));
        storage.parent.insert(hidden, Some(root));

        (storage, root, button, hidden, text)
    }

    fn new_state(storage: &mut ViewStorage, box_tree: &mut BoxTree) -> ViewId {
        let id = storage.view_ids.insert(());
        storage.root.insert(id, id);
        // Borrow the storage's own taffy tree so we don't need to name the
        // internal `LayoutNodeCx` type in the test.
        let mut taffy = storage.taffy.borrow_mut();
        let state = ViewState::new(id, &mut *taffy, box_tree);
        storage.states.insert(id, Rc::new(RefCell::new(state)));
        id
    }

    fn set_attrs(storage: &ViewStorage, id: ViewId, attrs: AccessibilityAttrs) {
        storage.states.get(id).unwrap().borrow_mut().accessibility = Some(attrs);
    }

    fn set_bounds(storage: &ViewStorage, id: ViewId, rect: Rect) {
        storage.states.get(id).unwrap().borrow_mut().layout = Some(LayoutChanged {
            new_box: rect,
            new_content_box: rect,
            new_window_origin: Point::new(rect.x0, rect.y0),
        });
    }

    #[test]
    fn builds_window_tree_with_roles_labels_bounds() {
        let (storage, root, button, _hidden, text) = fixture();
        let update = build_access_tree_update(&storage, root, Some(button));

        assert!(!update.is_empty());
        assert_eq!(update.root, Some(node_id(root)));

        let tree = apply(&update);

        let root_node = tree.get(node_id(root)).expect("root present");
        assert_eq!(root_node.role, AccessRole::Window);
        assert_eq!(root_node.label.as_deref(), Some("Main"));
        assert_eq!(root_node.bounds, Some(Rect::new(0.0, 0.0, 100.0, 100.0)));
        // Hidden container must NOT appear; its child re-parented to root.
        assert_eq!(root_node.children, vec![node_id(button), node_id(text)]);

        let btn = tree.get(node_id(button)).expect("button present");
        assert_eq!(btn.role, AccessRole::Button);
        assert_eq!(btn.label.as_deref(), Some("OK"));
        assert_eq!(btn.bounds, Some(Rect::new(10.0, 10.0, 50.0, 30.0)));
        assert!(btn.states.contains(AccessState::FOCUSED));
        assert!(btn.supports(crate::accessibility::AccessAction::Default));
        assert!(btn.children.is_empty());

        let txt = tree.get(node_id(text)).expect("text re-parented to root");
        assert_eq!(txt.role, AccessRole::Text);
        assert_eq!(txt.label.as_deref(), Some("Hi"));
    }

    #[test]
    fn focus_and_hidden_reparenting() {
        let (storage, root, button, hidden, text) = fixture();
        let update = build_access_tree_update(&storage, root, Some(button));

        // Focus is reported on the button, not the hidden container or text.
        assert_eq!(update.focus, Some(node_id(button)));

        let tree = apply(&update);
        assert!(tree.get(node_id(hidden)).is_none(), "hidden node omitted");
        assert_eq!(tree.focused().map(|n| n.id), Some(node_id(button)));
    }

    fn apply(update: &AccessTreeUpdate) -> AccessTree {
        let mut tree = AccessTree::new();
        tree.apply(update.clone());
        tree
    }
}
