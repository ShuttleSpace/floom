// SPDX-License-Identifier: MIT
//! Semantics node schema and the retained semantics tree.
//!
//! The tree is keyed by [`ElementId`] so it can be invalidated directly from the
//! runtime's `SEMANTICS` dirty flag without walking the element tree.

use std::collections::{HashMap, HashSet};

use bitflags::bitflags;
use floom_types::{ElementId, Rect};

/// Accessible role of a semantics node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticsRole {
    /// No semantics; node is skipped by assistive technology.
    None,
    Button,
    CheckBox,
    Radio,
    Switch,
    Slider,
    TextField,
    TextView,
    Image,
    ImageButton,
    Link,
    Heading,
    StaticText,
    List,
    ListItem,
    Group,
    Scrollable,
    ProgressBar,
    Tab,
    TabBar,
    Dialog,
    MenuItem,
    /// Framework or application-specific role.
    Custom(u32),
}

bitflags! {
    /// State flags reported to assistive technology.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SemanticsState: u32 {
        const SELECTED   = 1 << 0;
        const CHECKED    = 1 << 1;
        const DISABLED   = 1 << 2;
        const HIDDEN     = 1 << 3;
        const FOCUSED    = 1 << 4;
        const EXPANDED   = 1 << 5;
        const COLLAPSED  = 1 << 6;
        const ENABLED    = 1 << 7;
        const FOCUSABLE  = 1 << 8;
        const SCROLLABLE = 1 << 9;
        const LIVE       = 1 << 10;
    }
}

/// Action an assistive technology may request on a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticsAction {
    Activate,
    SetValue(String),
    Increase,
    Decrease,
    ScrollIntoView,
    Show,
    Hide,
    Custom(u32),
}

/// Description of one accessible element.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticsNode {
    pub role: SemanticsRole,
    /// Where the element is, in the window's logical points.
    ///
    /// Without this an adapter knows *what* is on screen and not *where*, which
    /// is the shape of a node a screen reader can name but not touch: the iOS
    /// gate recorded exactly that — labels present, every frame zero.
    pub bounds: Rect,
    pub label: String,
    pub value: String,
    pub hint: String,
    pub state: SemanticsState,
    pub actions: Vec<SemanticsAction>,
    pub children: Vec<ElementId>,
}

impl SemanticsNode {
    pub fn new(role: SemanticsRole) -> Self {
        Self {
            role,
            bounds: Rect::default(),
            label: String::new(),
            value: String::new(),
            hint: String::new(),
            state: SemanticsState::empty(),
            actions: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn with_state(mut self, state: SemanticsState) -> Self {
        self.state = state;
        self
    }

    pub fn with_action(mut self, action: SemanticsAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_child(mut self, child: ElementId) -> Self {
        self.children.push(child);
        self
    }

    /// A node whose role is [`SemanticsRole::None`] and which carries no
    /// information is not worth publishing to an adapter.
    pub fn is_empty(&self) -> bool {
        matches!(self.role, SemanticsRole::None)
            && self.label.is_empty()
            && self.value.is_empty()
            && self.hint.is_empty()
            && self.state.is_empty()
            && self.actions.is_empty()
            && self.children.is_empty()
    }
}

impl Default for SemanticsNode {
    fn default() -> Self {
        Self::new(SemanticsRole::None)
    }
}

/// Changes produced since the previous [`SemanticsTree::collect_updates`].
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SemanticsDiff {
    /// Nodes that did not exist at the previous drain.
    pub added: Vec<(ElementId, SemanticsNode)>,
    /// Nodes that existed before but changed semantics.
    pub updated: Vec<(ElementId, SemanticsNode)>,
    /// Elements whose node was removed.
    pub removed: Vec<ElementId>,
}

impl SemanticsDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }

    /// Every element touched by this diff, in added → updated → removed order.
    pub fn changed_ids(&self) -> Vec<ElementId> {
        let mut ids =
            Vec::with_capacity(self.added.len() + self.updated.len() + self.removed.len());
        ids.extend(self.added.iter().map(|(id, _)| *id));
        ids.extend(self.updated.iter().map(|(id, _)| *id));
        ids.extend(self.removed.iter().copied());
        ids
    }
}

/// Retained semantics tree keyed by [`ElementId`].
///
/// Nodes are upserted wholesale; the tree tracks which elements changed since
/// the last drain so adapters receive the smallest possible diff.
#[derive(Debug, Default)]
pub struct SemanticsTree {
    nodes: HashMap<ElementId, SemanticsNode>,
    dirty: HashMap<ElementId, bool>,
    /// Elements inserted since the last drain (differentiates added vs updated).
    inserted: HashSet<ElementId>,
    /// Elements removed since the last drain.
    removed: Vec<ElementId>,
    /// Optional root element, used when publishing a whole-tree snapshot.
    root: Option<ElementId>,
}

impl SemanticsTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the tree in place for a wholesale rebuild, retaining the node
    /// map's capacity. A host that re-collects after any change must not pay
    /// a fresh allocation per element for storage it already owned — the
    /// Phase 28.1 allocation audit measured a full rebuild allocating per
    /// node purely because the previous tree (and its spare capacity) was
    /// dropped on the floor. Diff bookkeeping resets with the content, so a
    /// rebuild reports the same full diff dropping and recreating the tree
    /// always did.
    pub fn begin_rebuild(&mut self) {
        self.nodes.clear();
        self.dirty.clear();
        self.inserted.clear();
        self.removed.clear();
        self.root = None;
    }

    /// Upsert a node and mark the element dirty.
    pub fn set_node(&mut self, id: ElementId, node: SemanticsNode) {
        if self.nodes.insert(id, node).is_none() {
            self.inserted.insert(id);
        }
        self.dirty.insert(id, true);
    }

    /// Remove a node; the removal is reported on the next drain.
    pub fn remove(&mut self, id: ElementId) {
        if self.nodes.remove(&id).is_some() {
            self.dirty.insert(id, true);
            self.removed.push(id);
            self.inserted.remove(&id);
        }
        if self.root == Some(id) {
            self.root = None;
        }
    }

    pub fn get(&self, id: ElementId) -> Option<&SemanticsNode> {
        self.nodes.get(&id)
    }

    /// Iterate all live `(ElementId, &SemanticsNode)` pairs. Order is unspecified
    /// (a `HashMap`); callers that need a stable snapshot sort by
    /// `(id.index(), id.generation())` — the id has no `Ord` impl by design.
    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &SemanticsNode)> + '_ {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }

    pub fn contains(&self, id: ElementId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn root(&self) -> Option<ElementId> {
        self.root
    }

    pub fn set_root(&mut self, id: ElementId) {
        self.root = Some(id);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Called by the runtime when an element's `SEMANTICS` dirty flag is set.
    ///
    /// Unknown elements are ignored: an element without a node has nothing to
    /// publish.
    pub fn mark_semantics_dirty(&mut self, id: ElementId) {
        if self.nodes.contains_key(&id) {
            self.dirty.insert(id, true);
        }
    }

    pub fn is_dirty(&self, id: ElementId) -> bool {
        self.dirty.get(&id).copied().unwrap_or(false)
    }

    /// Whether any element has pending semantics changes.
    pub fn has_pending_updates(&self) -> bool {
        self.dirty.values().any(|dirty| *dirty) || !self.removed.is_empty()
    }

    /// Drain pending changes into a diff and clear the dirty state.
    pub fn collect_updates(&mut self) -> SemanticsDiff {
        let mut added = Vec::new();
        let mut updated = Vec::new();
        let mut cleared = Vec::new();

        for (&id, &is_dirty) in self.dirty.iter() {
            if !is_dirty {
                continue;
            }
            cleared.push(id);
            if let Some(node) = self.nodes.get(&id) {
                if self.inserted.contains(&id) {
                    added.push((id, node.clone()));
                } else {
                    updated.push((id, node.clone()));
                }
            }
        }

        let removed = std::mem::take(&mut self.removed);
        let removed_set: HashSet<ElementId> = removed.iter().copied().collect();
        for id in cleared {
            if removed_set.contains(&id) {
                // Node is gone; drop the tracking entry entirely.
                self.dirty.remove(&id);
            } else {
                self.dirty.insert(id, false);
            }
            self.inserted.remove(&id);
        }

        SemanticsDiff {
            added,
            updated,
            removed,
        }
    }
}

/// Builder that updates one element's node inside a [`SemanticsTree`].
///
/// Preserves existing fields when the node already exists, so a widget can
/// update just its label without restating role/actions.
pub struct SemanticsContext<'a> {
    tree: &'a mut SemanticsTree,
    id: ElementId,
    node: SemanticsNode,
}

impl<'a> SemanticsContext<'a> {
    pub fn new(tree: &'a mut SemanticsTree, id: ElementId, role: SemanticsRole) -> Self {
        let node = tree
            .get(id)
            .cloned()
            .unwrap_or_else(|| SemanticsNode::new(role));
        Self { tree, id, node }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.node.label = label.into();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.node.value = value.into();
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.node.hint = hint.into();
        self
    }

    pub fn state(mut self, state: SemanticsState) -> Self {
        self.node.state = state;
        self
    }

    pub fn add_action(mut self, action: SemanticsAction) -> Self {
        self.node.actions.push(action);
        self
    }

    pub fn child(mut self, child: ElementId) -> Self {
        self.node.children.push(child);
        self
    }

    /// Commit the node and mark the element semantics-dirty.
    pub fn commit(self) {
        self.tree.set_node(self.id, self.node);
    }
}
