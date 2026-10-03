//! Retained scene graph (OS / GPU independent).

mod damage;
mod node;
mod resources;

pub use damage::DamageRegion;
pub use floom_types::{Color, Point, Rect, ResourceId, SceneNodeId, Size, Transform};
pub use node::SceneNode;
pub use resources::{Resource, ResourceKind, ResourceRegistry};

use floom_types::SceneNodeId as Id;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Free,
    Live { generation: u32 },
}

/// Retained scene with transactional updates and damage tracking.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    slots: Vec<SlotState>,
    nodes: Vec<Option<SceneNode>>,
    /// Freed indices with the generation to use on next alloc.
    free: VecDeque<(u32, u32)>,
    updating: bool,
    damage: DamageRegion,
    tx_damage: DamageRegion,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn damage(&self) -> &DamageRegion {
        &self.damage
    }

    pub fn get(&self, id: Id) -> Option<&SceneNode> {
        if !self.contains(id) {
            return None;
        }
        self.nodes.get(id.index() as usize)?.as_ref()
    }

    /// Iterate live `(SceneNodeId, &SceneNode)` pairs in index order.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &SceneNode)> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let SlotState::Live { generation } = slot else {
                return None;
            };
            let id = Id::new(index as u32, *generation);
            let node = self.nodes.get(index)?.as_ref()?;
            Some((id, node))
        })
    }

    pub fn contains(&self, id: Id) -> bool {
        if id.is_invalid() {
            return false;
        }
        match self.slots.get(id.index() as usize) {
            Some(SlotState::Live { generation }) => id.matches_generation(*generation),
            _ => false,
        }
    }

    pub fn begin_update(&mut self) {
        assert!(!self.updating, "nested begin_update");
        self.updating = true;
        self.tx_damage.clear();
    }

    pub fn upsert(&mut self, id: Id, node: SceneNode) {
        assert!(self.updating, "upsert outside begin_update/finish_update");
        assert!(self.contains(id), "upsert of unknown or stale SceneNodeId");

        let index = id.index() as usize;
        let old = self.nodes[index].take();
        if let Some(old_node) = &old {
            // Repainting the identical payload is not damage. This is what
            // makes per-frame damage meaningful for a retained host that
            // re-paints every widget: only bounds/payload *changes* dirty the
            // region.
            if *old_node == node {
                self.nodes[index] = Some(node);
                return;
            }
            match old_node.bounds_hint() {
                Some(b) => self.tx_damage.union_rect(b),
                // Groups (None hint) force a full frame.
                None => self.tx_damage.mark_full_frame(),
            }
        }
        self.damage_node(&node);
        self.nodes[index] = Some(node);
    }

    pub fn remove(&mut self, id: Id) {
        assert!(self.updating, "remove outside begin_update/finish_update");
        if !self.contains(id) {
            return;
        }
        let index = id.index() as usize;
        if let Some(old) = self.nodes[index].take() {
            self.damage_node(&old);
        }
        let SlotState::Live { generation } = self.slots[index] else {
            return;
        };
        let next_generation = generation.wrapping_add(1).max(1);
        self.slots[index] = SlotState::Free;
        self.free.push_back((id.index(), next_generation));
    }

    pub fn finish_update(&mut self) {
        assert!(self.updating, "finish_update without begin_update");
        self.updating = false;
        self.damage.union(&self.tx_damage);
        self.tx_damage.clear();
    }

    pub fn clear_damage(&mut self) {
        self.damage.clear();
    }

    /// Insert a new node and return its stable id (must be inside a transaction).
    pub fn insert(&mut self, node: SceneNode) -> Id {
        let id = self.alloc_id();
        // alloc creates Live empty slot; upsert requires contains — OK
        self.upsert(id, node);
        id
    }

    /// Allocate an id without inserting a node (for external upsert).
    pub fn alloc_id(&mut self) -> Id {
        if let Some((index, generation)) = self.free.pop_front() {
            self.slots[index as usize] = SlotState::Live { generation };
            if self.nodes.len() <= index as usize {
                self.nodes.resize_with(index as usize + 1, || None);
            }
            debug_assert!(self.nodes[index as usize].is_none());
            return Id::new(index, generation);
        }
        let index = self.slots.len() as u32;
        self.slots.push(SlotState::Live { generation: 1 });
        self.nodes.push(None);
        Id::new(index, 1)
    }

    fn damage_node(&mut self, node: &SceneNode) {
        if let Some(b) = node.bounds_hint() {
            self.tx_damage.union_rect(b);
        } else {
            self.tx_damage.mark_full_frame();
        }
    }
}
