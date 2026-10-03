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

    /// Put a previously-live id back without bumping its generation.
    ///
    /// This is the checkpoint/restore path: after a rollback the allocator
    /// must be able to resurrect the *same* handle, or every retained id in
    /// the restored tree would dangle. It is only allowed when the slot is
    /// free and its recorded next generation equals `id`'s — i.e. this is
    /// exactly the handle this allocator would mint next for the index, so a
    /// stale id can never be resurrected through it.
    pub fn restore(&mut self, id: ElementId) -> bool {
        if id.is_invalid() {
            return false;
        }
        let index = id.index();
        // The slot must be free and expecting exactly this generation.
        let Some(IdSlotState::Free {
            next_free,
            next_generation,
        }) = self.slots.get(index as usize).copied()
        else {
            return false; // live slot, out of range, or invalid id
        };
        if next_generation == 0 || id.generation() == 0 || id.generation() >= next_generation {
            // Restoring the generation the allocator would hand out *next*
            // (or one already superseded) would alias a live-or-future handle.
            // Only a strictly earlier generation is safe, and only under the
            // rollback discipline: the generation in between was allocated
            // inside a failed patch that released no handles.
            return false;
        }
        // Unlink the index from the free list. The list is singly linked, so
        // this is a walk; restore is a rare, user-visible operation and the
        // list is short (one link per free slot).
        let mut prev: Option<u32> = None;
        let mut current = self.free_head;
        let mut found = false;
        while let Some(c) = current {
            if c == index {
                found = true;
                break;
            }
            prev = Some(c);
            current = match self.slots[c as usize] {
                IdSlotState::Free { next_free, .. } => next_free,
                IdSlotState::Occupied { .. } => unreachable!("free list only walks free slots"),
            };
        }
        if !found {
            return false;
        }
        match prev {
            None => self.free_head = next_free,
            Some(p) => {
                if let IdSlotState::Free {
                    next_generation: prev_gen,
                    ..
                } = self.slots[p as usize]
                {
                    self.slots[p as usize] = IdSlotState::Free {
                        next_free,
                        next_generation: prev_gen,
                    };
                }
            }
        }
        self.slots[index as usize] = IdSlotState::Occupied {
            generation: id.generation(),
        };
        self.live_count += 1;
        true
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
