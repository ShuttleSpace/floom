//! Generational `ElementId` allocator with stale-ID rejection.

use floem_types::ElementId;

#[derive(Debug, Clone, Copy)]
enum IdSlotState {
    /// Unused slot. `next_generation` is used on the next alloc of this index.
    Free {
        next_free: Option<u32>,
        next_generation: u32,
    },
    Occupied {
        generation: u32,
    },
}

/// Metadata for one index slot in the allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdSlot {
    pub generation: u32,
    pub occupied: bool,
}

/// Allocates and frees [`ElementId`] values with generation bumping.
#[derive(Debug)]
pub struct IdAllocator {
    slots: Vec<IdSlotState>,
    free_head: Option<u32>,
    live_count: usize,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdAllocator {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
            live_count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.live_count
    }

    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Allocate a fresh live [`ElementId`].
    ///
    /// Generation starts at `1` so `(index=0, generation=0)` remains the invalid sentinel.
    pub fn alloc(&mut self) -> ElementId {
        if let Some(index) = self.free_head.take() {
            let slot = &mut self.slots[index as usize];
            let IdSlotState::Free {
                next_free,
                next_generation,
            } = *slot
            else {
                panic!("IdAllocator free list corruption");
            };
            self.free_head = next_free;
            let generation = next_generation.max(1);
            *slot = IdSlotState::Occupied { generation };
            self.live_count += 1;
            return ElementId::new(index, generation);
        }

        let index = self.slots.len() as u32;
        self.slots.push(IdSlotState::Occupied { generation: 1 });
        self.live_count += 1;
        ElementId::new(index, 1)
    }

    /// Free a live id. Returns `false` if the id was already stale/invalid.
    pub fn free(&mut self, id: ElementId) -> bool {
        if !self.contains(id) {
            return false;
        }
        let index = id.index() as usize;
        let IdSlotState::Occupied { generation } = self.slots[index] else {
            return false;
        };
        let next_generation = generation.wrapping_add(1).max(1);
        self.slots[index] = IdSlotState::Free {
            next_free: self.free_head,
            next_generation,
        };
        self.free_head = Some(id.index());
        self.live_count -= 1;
        true
    }

    /// Returns whether `id` refers to a currently live allocation.
    pub fn contains(&self, id: ElementId) -> bool {
        if id.is_invalid() {
            return false;
        }
        match self.slots.get(id.index() as usize) {
            Some(IdSlotState::Occupied { generation }) => id.matches_generation(*generation),
            _ => false,
        }
    }

    pub fn slot(&self, index: u32) -> Option<IdSlot> {
        match self.slots.get(index as usize)? {
            IdSlotState::Free {
                next_generation, ..
            } => Some(IdSlot {
                generation: *next_generation,
                occupied: false,
            }),
            IdSlotState::Occupied { generation } => Some(IdSlot {
                generation: *generation,
                occupied: true,
            }),
        }
    }
}

#[cfg(test)]
#[path = "id_tests.rs"]
mod id_tests;
