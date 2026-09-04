//! Persistent parent/child element tree over [`ElementArena`].

use floem_types::ElementId;
use smallvec::SmallVec;

use crate::arena::ElementArena;

type ChildList = SmallVec<[ElementId; 4]>;

/// Child lists keyed by element index, kept consistent with [`Element::parent`].
#[derive(Debug, Default)]
pub struct ElementTree {
    children: Vec<ChildList>,
}

impl ElementTree {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_index(&mut self, index: usize) {
        if index >= self.children.len() {
            self.children.resize_with(index + 1, ChildList::new);
        }
    }

    pub fn children(&self, id: ElementId) -> &[ElementId] {
        self.children
            .get(id.index() as usize)
            .map(|c| c.as_slice())
            .unwrap_or(&[])
    }

    pub fn parent(&self, arena: &ElementArena, id: ElementId) -> Option<ElementId> {
        arena.get(id).and_then(|e| e.parent)
    }

    /// Append `child` under `parent`. Detaches from any previous parent first.
    pub fn append_child(
        &mut self,
        arena: &mut ElementArena,
        parent: ElementId,
        child: ElementId,
    ) -> bool {
        self.insert_child(arena, parent, self.children(parent).len(), child)
    }

    /// Insert `child` at `index` under `parent`.
    pub fn insert_child(
        &mut self,
        arena: &mut ElementArena,
        parent: ElementId,
        index: usize,
        child: ElementId,
    ) -> bool {
        if parent == child {
            return false;
        }
        if arena.get(parent).is_none() || arena.get(child).is_none() {
            return false;
        }
        if self.is_ancestor(arena, child, parent) {
            // Would create a cycle.
            return false;
        }

        self.detach(arena, child);

        let parent_index = parent.index() as usize;
        self.ensure_index(parent_index);
        let kids = &mut self.children[parent_index];
        let index = index.min(kids.len());
        kids.insert(index, child);

        if let Some(el) = arena.get_mut(child) {
            el.parent = Some(parent);
        }

        debug_assert!(self.debug_consistent(arena, parent, child));
        true
    }

    /// Detach `id` from its parent (if any). Does not remove from the arena.
    pub fn detach(&mut self, arena: &mut ElementArena, id: ElementId) -> bool {
        let Some(el) = arena.get(id) else {
            return false;
        };
        let Some(parent) = el.parent else {
            return true;
        };

        let pindex = parent.index() as usize;
        if let Some(kids) = self.children.get_mut(pindex) {
            if let Some(pos) = kids.iter().position(|&c| c == id) {
                kids.remove(pos);
            }
        }
        if let Some(el) = arena.get_mut(id) {
            el.parent = None;
        }
        true
    }

    /// Move `id` among siblings under `parent` to `index`.
    pub fn move_child(
        &mut self,
        arena: &mut ElementArena,
        parent: ElementId,
        id: ElementId,
        index: usize,
    ) -> bool {
        if arena.get(parent).is_none() || arena.get(id).is_none() {
            return false;
        }
        if arena.get(id).and_then(|e| e.parent) != Some(parent) {
            // Not currently a child of parent — treat as insert.
            return self.insert_child(arena, parent, index, id);
        }

        let pindex = parent.index() as usize;
        self.ensure_index(pindex);
        let kids = &mut self.children[pindex];
        let Some(from) = kids.iter().position(|&c| c == id) else {
            return false;
        };
        // `index` is the insertion position after removal.
        let child = kids.remove(from);
        let to = index.min(kids.len());
        kids.insert(to, child);
        true
    }

    /// Depth-first preorder walk of the subtree rooted at `root`.
    pub fn walk_preorder(&self, root: ElementId, mut visit: impl FnMut(ElementId)) {
        self.walk_preorder_dyn(root, &mut visit);
    }

    fn walk_preorder_dyn(&self, root: ElementId, visit: &mut dyn FnMut(ElementId)) {
        visit(root);
        for &child in self.children(root) {
            self.walk_preorder_dyn(child, visit);
        }
    }

    fn is_ancestor(&self, arena: &ElementArena, ancestor: ElementId, node: ElementId) -> bool {
        let mut current = Some(node);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.parent(arena, id);
        }
        false
    }

    #[cfg(debug_assertions)]
    fn debug_consistent(&self, arena: &ElementArena, parent: ElementId, child: ElementId) -> bool {
        let parent_ok = arena.get(parent).is_some();
        let child_ok = arena.get(child).map(|e| e.parent == Some(parent)).unwrap_or(false);
        let listed = self.children(parent).contains(&child);
        parent_ok && child_ok && listed
    }

    #[cfg(not(debug_assertions))]
    fn debug_consistent(&self, _arena: &ElementArena, _parent: ElementId, _child: ElementId) -> bool {
        true
    }
}

/// Plan-shaped free functions.
pub fn append_child(
    tree: &mut ElementTree,
    arena: &mut ElementArena,
    parent: ElementId,
    child: ElementId,
) -> bool {
    tree.append_child(arena, parent, child)
}

pub fn insert_child(
    tree: &mut ElementTree,
    arena: &mut ElementArena,
    parent: ElementId,
    index: usize,
    child: ElementId,
) -> bool {
    tree.insert_child(arena, parent, index, child)
}

pub fn detach(tree: &mut ElementTree, arena: &mut ElementArena, id: ElementId) -> bool {
    tree.detach(arena, id)
}

pub fn move_child(
    tree: &mut ElementTree,
    arena: &mut ElementArena,
    parent: ElementId,
    id: ElementId,
    index: usize,
) -> bool {
    tree.move_child(arena, parent, id, index)
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;
