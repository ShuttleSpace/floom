//! Generational element arena.

use floom_core::{Element, ElementKind};
use floom_types::ElementId;

use crate::id::IdAllocator;

/// Slot-map style storage for [`Element`] values keyed by [`ElementId`].
#[derive(Debug, Default)]
pub struct ElementArena {
    ids: IdAllocator,
    /// Indexed by `ElementId::index()`. Vacant when the id slot is free.
    elements: Vec<Option<Element>>,
}

impl ElementArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Insert `element` metadata, assigning a fresh id (ignores `element.id`).
    pub fn insert(&mut self, mut element: Element) -> ElementId {
        let id = self.ids.alloc();
        element.id = id;
        let index = id.index() as usize;
        if index >= self.elements.len() {
            self.elements.resize_with(index + 1, || None);
        }
        debug_assert!(self.elements[index].is_none());
        self.elements[index] = Some(element);
        id
    }

    /// Convenience: insert a new primitive element with no parent.
    pub fn insert_primitive(&mut self) -> ElementId {
        self.insert(Element::new(ElementId::INVALID, ElementKind::Primitive))
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        if !self.ids.contains(id) {
            return None;
        }
        self.elements.get(id.index() as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        if !self.ids.contains(id) {
            return None;
        }
        self.elements.get_mut(id.index() as usize)?.as_mut()
    }

    pub fn remove(&mut self, id: ElementId) -> Option<Element> {
        if !self.ids.contains(id) {
            return None;
        }
        let index = id.index() as usize;
        let element = self.elements.get_mut(index)?.take();
        assert!(self.ids.free(id));
        element
    }

    /// Put an element back at its own id (checkpoint/restore).
    ///
    /// [`insert`](Self::insert) mints a fresh id on purpose; a rollback must
    /// not, or every retained handle in the restored tree would dangle. The
    /// id allocator refuses the restore unless the slot is free and expects
    /// exactly this generation, so a stale id cannot sneak back in.
    pub fn restore(&mut self, element: Element) -> bool {
        if !self.ids.restore(element.id) {
            return false;
        }
        let index = element.id.index() as usize;
        if index >= self.elements.len() {
            self.elements.resize_with(index + 1, || None);
        }
        self.elements[index] = Some(element);
        true
    }
}

#[cfg(test)]
#[path = "arena_tests.rs"]
mod arena_tests;
