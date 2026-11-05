// SPDX-License-Identifier: MIT
//! Patches: the named operations a reload applies, and their transactional
//! application.

use floom_core::LayoutRequest;
use floom_runtime_core::{DirtyFlags, DirtySet};
use floom_style::StyleProps;
use floom_types::ElementId;
use floom_dsl::ui_ir::{EventName, NodeKey, NodeKind, Prop};

use crate::live::{LiveTree, element_kind};

/// One named change to the live tree.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchOp {
    /// Mount `kind` at child `index` of `parent`, with these properties.
    /// `parent` is `None` for a new root (the old root was removed by an
    /// earlier op — renaming the root key is exactly that).
    Insert {
        key: NodeKey,
        parent: Option<NodeKey>,
        index: usize,
        kind: NodeKind,
        props: Vec<Prop>,
    },
    /// Unmount `key` and its whole subtree. Their ids are freed.
    Remove { key: NodeKey },
    /// Re-order `key` under `parent` so it sits at child `index`.
    Move { key: NodeKey, index: usize },
    /// Replace the element's NORMAL-state style with these properties.
    SetStyle { key: NodeKey, props: StyleProps },
    /// Drop the element's recorded layout request.
    ClearLayout { key: NodeKey },
    /// Replace the element's layout request.
    SetLayout { key: NodeKey, layout: LayoutRequest },
    /// Wire an event to a handler name.
    AddEvent {
        key: NodeKey,
        event: EventName,
        handler: String,
    },
    /// Unwire an event.
    RemoveEvent { key: NodeKey, event: EventName },
}

/// An ordered list of operations. Diff order matters: removals before
/// inserts, so re-parenting under a brand-new container works.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiPatch {
    pub ops: Vec<PatchOp>,
}

impl UiPatch {
    pub fn new(ops: Vec<PatchOp>) -> Self {
        Self { ops }
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// What a successful apply did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    /// Operations applied, in order.
    pub applied: usize,
    /// Elements mounted (`Insert` ops).
    pub mounted: Vec<NodeKey>,
    /// Elements unmounted (`Remove` ops), including their subtrees.
    pub unmounted: Vec<NodeKey>,
}

/// Why an apply failed. The tree is exactly as it was before the patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolledBack {
    /// The zero-based index of the operation that failed.
    pub failed_at: usize,
    /// What went wrong, naming keys rather than ids.
    pub reason: String,
}

impl LiveTree {
    /// Apply a patch transactionally: all operations, or none.
    ///
    /// The dirty set receives the invalidations a host needs — structure
    /// changes mark the parent's children, style changes mark style+paint,
    /// layout changes propagate up the ancestor chain.
    pub fn apply(
        &mut self,
        patch: &UiPatch,
        dirty: &mut DirtySet,
    ) -> Result<ApplyReport, RolledBack> {
        let checkpoint = self.checkpoint();
        let mut report = ApplyReport {
            applied: 0,
            mounted: Vec::new(),
            unmounted: Vec::new(),
        };
        for (index, op) in patch.ops.iter().enumerate() {
            if let Err(reason) = self.apply_one(op, dirty, &mut report) {
                self.restore(checkpoint);
                return Err(RolledBack {
                    failed_at: index,
                    reason,
                });
            }
            report.applied += 1;
        }
        Ok(report)
    }

    fn apply_one(
        &mut self,
        op: &PatchOp,
        dirty: &mut DirtySet,
        report: &mut ApplyReport,
    ) -> Result<(), String> {
        match op {
            PatchOp::Insert {
                key,
                parent,
                index,
                kind,
                props,
            } => {
                if self.id_of(key).is_some() {
                    return Err(format!("`{}` is already mounted", key.as_str()));
                }
                let parent_id = match parent {
                    Some(parent) => {
                        let Some(parent_id) = self.id_of(parent) else {
                            return Err(format!("parent `{}` is not mounted", parent.as_str()));
                        };
                        Some(parent_id)
                    }
                    None => None,
                };
                let id = self.arena.insert(floom_core::Element::new(
                    ElementId::INVALID,
                    element_kind(*kind),
                ));
                let linked = match parent_id {
                    Some(parent_id) => {
                        self.tree
                            .insert_child(&mut self.arena, parent_id, *index, id)
                    }
                    None => true, // a new root: parentless by definition
                };
                if !linked {
                    // The arena slot is occupied but the tree rejected the
                    // link; free it before rolling back.
                    let _ = self.arena.remove(id);
                    return Err(format!(
                        "cannot insert `{}` under `{}` at {index}",
                        key.as_str(),
                        parent.as_ref().map(|p| p.as_str()).unwrap_or("(root)")
                    ));
                }
                self.register(key.clone(), *kind, id);
                self.set_props(id, *kind, props);
                if let Some(parent_id) = parent_id {
                    dirty.invalidate(&self.arena, parent_id, DirtyFlags::CHILDREN);
                }
                dirty.invalidate(&self.arena, id, DirtyFlags::PAINT);
                report.mounted.push(key.clone());
                Ok(())
            }
            PatchOp::Remove { key } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                let parent_id = self.arena.get(id).and_then(|e| e.parent);
                // Collect the subtree while it is still linked.
                let mut subtree = Vec::new();
                self.tree.walk_preorder(id, |n| subtree.push(n));
                // Detach the root from its parent, then drop every child link
                // in the subtree: a removed element's index can be reused, and
                // its child list must not survive into the next occupant.
                self.tree.detach(&mut self.arena, id);
                for node in &subtree {
                    self.tree.clear_children(&mut self.arena, *node);
                }
                for node in subtree.iter().rev() {
                    self.unregister(*node);
                    assert!(
                        self.arena.remove(*node).is_some(),
                        "a walked node is live in the arena"
                    );
                }
                if let Some(parent_id) = parent_id {
                    dirty.invalidate(&self.arena, parent_id, DirtyFlags::CHILDREN);
                }
                report.unmounted.push(key.clone());
                Ok(())
            }
            PatchOp::Move { key, index } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                let Some(parent_id) = self.arena.get(id).and_then(|e| e.parent) else {
                    return Err(format!("`{}` has no parent to move within", key.as_str()));
                };
                if !self.tree.move_child(&mut self.arena, parent_id, id, *index) {
                    return Err(format!("cannot move `{}` to index {index}", key.as_str()));
                }
                dirty.invalidate(&self.arena, parent_id, DirtyFlags::CHILDREN);
                Ok(())
            }
            PatchOp::SetStyle { key, props } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                self.set_style(id, props.clone());
                dirty.invalidate(&self.arena, id, DirtyFlags::STYLE | DirtyFlags::PAINT);
                Ok(())
            }
            PatchOp::SetLayout { key, layout } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                self.layouts_mut().insert(id.raw(), *layout);
                dirty.propagate_layout(&self.arena, id);
                Ok(())
            }
            PatchOp::ClearLayout { key } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                self.layouts_mut().remove(&id.raw());
                dirty.propagate_layout(&self.arena, id);
                Ok(())
            }
            PatchOp::AddEvent {
                key,
                event,
                handler,
            } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                let events = self.events_mut().entry(id.raw()).or_default();
                if events.iter().any(|b| b.event == *event) {
                    return Err(format!("`{}` already handles {event:?}", key.as_str()));
                }
                events.push(floom_dsl::ui_ir::EventBinding::new(*event, handler.clone()));
                dirty.invalidate(&self.arena, id, DirtyFlags::STATE);
                Ok(())
            }
            PatchOp::RemoveEvent { key, event } => {
                let Some(id) = self.id_of(key) else {
                    return Err(format!("`{}` is not mounted", key.as_str()));
                };
                let events = self.events_mut().get_mut(&id.raw());
                let Some(events) = events else {
                    return Err(format!("`{}` has no events to remove", key.as_str()));
                };
                let before = events.len();
                events.retain(|b| b.event != *event);
                if events.len() == before {
                    return Err(format!("`{}` does not handle {event:?}", key.as_str()));
                }
                dirty.invalidate(&self.arena, id, DirtyFlags::STATE);
                Ok(())
            }
        }
    }
}
