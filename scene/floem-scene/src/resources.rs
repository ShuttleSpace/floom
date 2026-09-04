//! Shared scene resources with ref-count tracking.

use floem_types::ResourceId;
use std::collections::VecDeque;

/// Kind of retained resource (renderer backends map these to GPU objects later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceKind {
    Image { width: u32, height: u32 },
    GlyphRun { glyph_count: u32 },
    Blob { bytes: usize },
}

/// A registry-owned resource payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub kind: ResourceKind,
}

#[derive(Debug)]
struct Entry {
    resource: Resource,
    refs: u32,
    generation: u32,
}

/// Ref-counted resource table with optional budget eviction hooks.
#[derive(Debug, Default)]
pub struct ResourceRegistry {
    entries: Vec<Option<Entry>>,
    free: VecDeque<(u32, u32)>,
    /// Soft budget on total `Blob` bytes + image pixels (approx).
    budget_units: u64,
    used_units: u64,
    evictions: u64,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            budget_units: u64::MAX,
            ..Self::default()
        }
    }

    pub fn with_budget(mut self, budget_units: u64) -> Self {
        self.budget_units = budget_units;
        self
    }

    pub fn used_units(&self) -> u64 {
        self.used_units
    }

    pub fn eviction_count(&self) -> u64 {
        self.evictions
    }

    pub fn insert(&mut self, resource: Resource) -> ResourceId {
        let cost = resource_cost(&resource);
        self.maybe_evict(cost);
        let generation_and_index = if let Some((index, generation)) = self.free.pop_front() {
            (index, generation)
        } else {
            let index = self.entries.len() as u32;
            self.entries.push(None);
            (index, 1u32)
        };
        let (index, generation) = generation_and_index;
        self.entries[index as usize] = Some(Entry {
            resource,
            refs: 1,
            generation,
        });
        self.used_units = self.used_units.saturating_add(cost);
        ResourceId::new(index, generation)
    }

    pub fn retain(&mut self, id: ResourceId) -> bool {
        let Some(entry) = self.live_mut(id) else {
            return false;
        };
        entry.refs = entry.refs.saturating_add(1);
        true
    }

    pub fn release(&mut self, id: ResourceId) -> bool {
        let index = id.index() as usize;
        let Some(entry) = self.entries.get_mut(index).and_then(|e| e.as_mut()) else {
            return false;
        };
        if !id.matches_generation(entry.generation) {
            return false;
        }
        entry.refs = entry.refs.saturating_sub(1);
        if entry.refs == 0 {
            let cost = resource_cost(&entry.resource);
            let generation = entry.generation;
            self.entries[index] = None;
            self.used_units = self.used_units.saturating_sub(cost);
            let next_generation = generation.wrapping_add(1).max(1);
            self.free.push_back((id.index(), next_generation));
        }
        true
    }

    pub fn get(&self, id: ResourceId) -> Option<&Resource> {
        self.live(id).map(|e| &e.resource)
    }

    pub fn refs(&self, id: ResourceId) -> Option<u32> {
        self.live(id).map(|e| e.refs)
    }

    /// Hook: drop zero-ref entries already released; if still over budget, evict
    /// least-recently-inserted single-ref resources (simple policy).
    pub fn maybe_evict(&mut self, upcoming_cost: u64) {
        while self.used_units.saturating_add(upcoming_cost) > self.budget_units {
            if !self.evict_one() {
                break;
            }
        }
    }

    fn evict_one(&mut self) -> bool {
        // Prefer entries with refs == 1 (only registry hold).
        let candidate = self
            .entries
            .iter()
            .enumerate()
            .find_map(|(i, e)| e.as_ref().filter(|e| e.refs == 1).map(|_| i));
        let Some(index) = candidate else {
            return false;
        };
        let entry = self.entries[index].take().unwrap();
        let cost = resource_cost(&entry.resource);
        self.used_units = self.used_units.saturating_sub(cost);
        let next_generation = entry.generation.wrapping_add(1).max(1);
        self.free.push_back((index as u32, next_generation));
        self.evictions += 1;
        true
    }

    fn live(&self, id: ResourceId) -> Option<&Entry> {
        let entry = self.entries.get(id.index() as usize)?.as_ref()?;
        if id.matches_generation(entry.generation) {
            Some(entry)
        } else {
            None
        }
    }

    fn live_mut(&mut self, id: ResourceId) -> Option<&mut Entry> {
        let entry = self.entries.get_mut(id.index() as usize)?.as_mut()?;
        if id.matches_generation(entry.generation) {
            Some(entry)
        } else {
            None
        }
    }
}

fn resource_cost(resource: &Resource) -> u64 {
    match resource.kind {
        ResourceKind::Image { width, height } => u64::from(width) * u64::from(height),
        ResourceKind::GlyphRun { glyph_count } => u64::from(glyph_count) * 16,
        ResourceKind::Blob { bytes } => bytes as u64,
    }
}
