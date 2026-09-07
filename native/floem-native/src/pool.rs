//! Native view slot pooling.
//!
//! A [`NativeSlot`] wraps a stable [`NativeViewId`] minted by the runtime. The
//! pool keeps a free list of recycled slots so that, as virtualized rows scroll
//! out of view, their native views can be reused for newly visible rows instead
//! of being destroyed and recreated.

use std::collections::HashMap;
use std::hash::Hash;

use floem_types::NativeViewId;

/// An opaque, reusable native view slot.
///
/// `id` is the stable [`NativeViewId`] the platform backend binds a concrete
/// native handle to; `generation` lets the platform detect a slot that was
/// recycled and must be re-bound (e.g. a fresh `UIView`/`Surface`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeSlot {
    pub id: NativeViewId,
    pub generation: u32,
}

/// Pools native views keyed by a stable data key `K`.
///
/// `acquire` returns the live slot for a key, reusing a freed slot when
/// available (otherwise minting a new one). `release` returns a slot to the free
/// list when its item scrolls out of view.
pub struct NativeViewPool<K> {
    live: HashMap<K, NativeSlot>,
    free: Vec<NativeSlot>,
    next_index: u32,
}

impl<K: Clone + Eq + Hash> Default for NativeViewPool<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Eq + Hash> NativeViewPool<K> {
    pub fn new() -> Self {
        Self {
            live: HashMap::new(),
            free: Vec::new(),
            next_index: 0,
        }
    }

    /// Live slot for `key`. Mints a fresh slot (reusing a freed one if present)
    /// when the key is not already live.
    pub fn acquire(&mut self, key: K) -> NativeSlot {
        if let Some(&slot) = self.live.get(&key) {
            return slot;
        }
        let slot = self.free.pop().unwrap_or_else(|| {
            let idx = self.next_index;
            self.next_index += 1;
            NativeSlot {
                id: NativeViewId::new(idx, 1),
                generation: 1,
            }
        });
        self.live.insert(key, slot);
        slot
    }

    /// Release the slot bound to `key` back to the free list (e.g. its item
    /// scrolled out). Returns the released slot, or `None` if the key was not live.
    pub fn release(&mut self, key: &K) -> Option<NativeSlot> {
        if let Some(slot) = self.live.remove(key) {
            self.free.push(slot);
            Some(slot)
        } else {
            None
        }
    }

    /// Bump the generation of a live slot, signalling the platform to re-bind it.
    pub fn bump_generation(&mut self, key: &K) -> Option<NativeSlot> {
        let slot = self.live.get_mut(key)?;
        slot.generation = slot.generation.wrapping_add(1);
        if slot.generation == 0 {
            slot.generation = 1;
        }
        Some(*slot)
    }

    /// Currently live slot for `key`, if any.
    pub fn get(&self, key: &K) -> Option<NativeSlot> {
        self.live.get(key).copied()
    }

    /// Number of live (bound) slots.
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// Number of slots parked in the free list, available for reuse.
    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    pub fn clear(&mut self) {
        self.live.clear();
        self.free.clear();
        self.next_index = 0;
    }
}
