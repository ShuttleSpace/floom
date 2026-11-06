// SPDX-License-Identifier: MIT
//! Old document → new document, as a patch of named operations.
//!
//! Matching is by [`NodeKey`] and only by [`NodeKey`]. That single decision is
//! what makes state survive a reload: a node whose key still exists keeps its
//! element, so everything keyed by the element — styles, layout, wiring, and
//! whatever state a layer stashed on it — carries over. Matching by position
//! would be cheaper to write and would silently reset the state of every node
//! after the first insertion.

use std::collections::HashMap;

use floom_dsl::ui_ir::{NodeKey, UiDocument};

use crate::live::seat_props;
use crate::patch::{PatchOp, UiPatch};

/// An element's state in one document, for diffing.
struct NodeState {
    /// The key of the parent node, and this node's index in its default slot.
    parent: Option<NodeKey>,
    index: usize,
    /// The default-slot children, in order — the material for move detection.
    children: Vec<NodeKey>,
}

/// Collect parent/index/children from the slot structure.
fn structure(document: &UiDocument) -> HashMap<NodeKey, NodeState> {
    let mut out: HashMap<NodeKey, NodeState> = document
        .nodes
        .iter()
        .map(|node| {
            (
                node.key.clone(),
                NodeState {
                    parent: None,
                    index: 0,
                    children: Vec::new(),
                },
            )
        })
        .collect();
    for node in &document.nodes {
        if let Some(state) = out.get_mut(&node.key)
            && let Some(slot) = node.slots.first()
        {
            state.children = slot.children.clone();
        }
        for slot in &node.slots {
            for (index, child) in slot.children.iter().enumerate() {
                if let Some(state) = out.get_mut(child) {
                    state.parent = Some(node.key.clone());
                    state.index = index;
                }
            }
        }
    }
    out
}

/// Compute the patch that turns `old` into `new`.
///
/// The result applies cleanly to a tree mounted from `old`. Removals come
/// before inserts so that re-parenting under a brand-new container works.
pub fn diff(old: &UiDocument, new: &UiDocument) -> UiPatch {
    let before = structure(old);
    let after = structure(new);

    fn node_of(document: &UiDocument) -> HashMap<&NodeKey, &floom_dsl::ui_ir::UiNode> {
        document.nodes.iter().map(|n| (&n.key, n)).collect()
    }
    let old_nodes = node_of(old);
    let new_nodes = node_of(new);

    let mut ops: Vec<PatchOp> = Vec::new();

    // Only the *roots* of removed subtrees are named: removing a parent takes
    // its children with it, and naming every descendant would make the report
    // claim work the applier does not do.
    let removed: Vec<&NodeKey> = before
        .keys()
        .filter(|key| !after.contains_key(*key))
        .collect();
    for key in &removed {
        let parent_removed = before[*key]
            .parent
            .as_ref()
            .map(|p| !after.contains_key(p))
            .unwrap_or(false);
        if !parent_removed {
            ops.push(PatchOp::Remove {
                key: (*key).clone(),
            });
        }
    }

    // Same parent in both documents but a different position. Applied in
    // ascending target order, sequential moves land in the target arrangement.
    let mut matched: Vec<&NodeKey> = before
        .keys()
        .filter(|key| after.contains_key(*key))
        .collect();
    matched.sort();
    for key in &matched {
        let (old_state, new_state) = (&before[*key], &after[*key]);
        if old_state.parent == new_state.parent && old_state.index != new_state.index {
            ops.push(PatchOp::Move {
                key: (*key).clone(),
                index: new_state.index,
            });
        }
    }

    for key in &matched {
        let old_node = old_nodes[*key];
        let new_node = new_nodes[*key];

        let (old_style, old_layout) = seat_props(old_node.kind, &old_node.props);
        let (new_style, new_layout) = seat_props(new_node.kind, &new_node.props);
        if old_style != new_style {
            ops.push(PatchOp::SetStyle {
                key: (*key).clone(),
                props: new_style,
            });
        }
        if old_layout != new_layout {
            if new_layout == floom_core::LayoutRequest::new() {
                ops.push(PatchOp::ClearLayout {
                    key: (*key).clone(),
                });
            } else {
                ops.push(PatchOp::SetLayout {
                    key: (*key).clone(),
                    layout: new_layout,
                });
            }
        }

        for binding in &new_node.events {
            if !old_node.events.iter().any(|b| b.event == binding.event) {
                ops.push(PatchOp::AddEvent {
                    key: (*key).clone(),
                    event: binding.event,
                    handler: binding.handler.clone(),
                });
            }
        }
        for binding in &old_node.events {
            if !new_node.events.iter().any(|b| b.event == binding.event) {
                ops.push(PatchOp::RemoveEvent {
                    key: (*key).clone(),
                    event: binding.event,
                });
            }
        }
    }

    // Parents before children: a node whose parent is itself new cannot mount
    // until the parent exists.
    let mut inserted: Vec<&NodeKey> = after
        .keys()
        .filter(|key| !before.contains_key(*key))
        .collect();
    inserted.sort();
    let mut order: Vec<&NodeKey> = Vec::with_capacity(inserted.len());
    let mut pending = inserted;
    while !pending.is_empty() {
        let mut progressed = false;
        let mut next_round = Vec::new();
        for key in pending {
            let parent_mounted = after[key]
                .parent
                .as_ref()
                .map(|p| before.contains_key(p) || order.contains(&p))
                .unwrap_or(true);
            if parent_mounted {
                order.push(key);
                progressed = true;
            } else {
                next_round.push(key);
            }
        }
        pending = next_round;
        if !progressed {
            // A cycle the IR validator would refuse; never loop forever here.
            order.append(&mut pending);
        }
    }
    for key in &order {
        let node = new_nodes[*key];
        let state = &after[*key];
        ops.push(PatchOp::Insert {
            key: (*key).clone(),
            // `None` mounts a new root — renaming the root key removes the old
            // root and mounts this one parentless.
            parent: state.parent.clone(),
            index: state.index,
            kind: node.kind,
            props: node.props.clone(),
        });
    }

    UiPatch { ops }
}
