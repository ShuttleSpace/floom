// SPDX-License-Identifier: MIT
//! Dirty flags and invalidation helpers.

use bitflags::bitflags;
use floom_types::ElementId;

use crate::arena::ElementArena;

bitflags! {
    /// Fine-grained invalidation bits for a single element.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct DirtyFlags: u32 {
        const STATE      = 1 << 0;
        const STYLE      = 1 << 1;
        const LAYOUT     = 1 << 2;
        const PAINT      = 1 << 3;
        const HIT_TEST   = 1 << 4;
        const SEMANTICS  = 1 << 5;
        const CHILDREN   = 1 << 6;
        const TRANSFORM  = 1 << 7;
    }
}

/// Side table of dirty bits keyed by element index (generation checked via arena).
#[derive(Debug, Default)]
pub struct DirtySet {
    flags: Vec<DirtyFlags>,
    /// Elements that have become dirty since the last [`Self::drain_pending`].
    pending: Vec<ElementId>,
}

impl DirtySet {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_index(&mut self, index: usize) {
        if index >= self.flags.len() {
            self.flags.resize(index + 1, DirtyFlags::empty());
        }
    }

    pub fn get(&self, id: ElementId) -> DirtyFlags {
        self.flags
            .get(id.index() as usize)
            .copied()
            .unwrap_or_else(DirtyFlags::empty)
    }

    pub fn clear(&mut self, id: ElementId) {
        if let Some(slot) = self.flags.get_mut(id.index() as usize) {
            *slot = DirtyFlags::empty();
        }
    }

    /// Mark `id` dirty. Returns `false` if the id is not live in `arena`.
    pub fn invalidate(&mut self, arena: &ElementArena, id: ElementId, flags: DirtyFlags) -> bool {
        if arena.get(id).is_none() {
            return false;
        }
        self.ensure_index(id.index() as usize);
        let was_clean = self.flags[id.index() as usize].is_empty();
        self.flags[id.index() as usize].insert(flags);
        if was_clean {
            self.pending.push(id);
        }
        true
    }

    /// Take the list of newly dirtied element ids (does not clear flags).
    pub fn drain_pending(&mut self) -> Vec<ElementId> {
        std::mem::take(&mut self.pending)
    }

    /// Mark layout dirty on `id` and walk parents via `Element::parent`.
    pub fn propagate_layout(&mut self, arena: &ElementArena, id: ElementId) -> bool {
        if !self.invalidate(arena, id, DirtyFlags::LAYOUT) {
            return false;
        }
        let mut current = arena.get(id).and_then(|e| e.parent);
        while let Some(parent) = current {
            if !self.invalidate(arena, parent, DirtyFlags::LAYOUT) {
                break;
            }
            current = arena.get(parent).and_then(|e| e.parent);
        }
        true
    }
}

pub fn invalidate(
    set: &mut DirtySet,
    arena: &ElementArena,
    id: ElementId,
    flags: DirtyFlags,
) -> bool {
    set.invalidate(arena, id, flags)
}

pub fn propagate_layout(set: &mut DirtySet, arena: &ElementArena, id: ElementId) -> bool {
    set.propagate_layout(arena, id)
}

#[cfg(test)]
#[path = "dirty_tests.rs"]
mod dirty_tests;
