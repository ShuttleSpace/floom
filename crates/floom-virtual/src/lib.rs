//! Retained virtual scrolling viewport for Floem 2.0.
//!
//! `VirtualViewport` turns an unbounded item list into a small, stable set of
//! materialized elements. It computes the visible item range from the scroll
//! offset and viewport size, then reports which stable-keyed elements must be
//! built (`enter`) and which can be recycled (`exit`). Callers keep a bounded
//! pool of elements (and native views) so memory stays flat regardless of list
//! length.
//!
//! This crate depends only on `floem-types`; it has no knowledge of the element
//! tree, renderer, or platform — virtualization is a pure geometry + key-reuse
//! primitive.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use floem_types::ElementId;

/// A visible item range, expressed as `[start, end)` over item indices.
pub type VisibleRange = std::ops::Range<usize>;

/// Per-item main-axis extents. Either a single uniform size or a per-index list.
#[derive(Debug, Clone)]
pub struct ItemSizes {
    kind: Kind,
    count: usize,
}

#[derive(Debug, Clone)]
enum Kind {
    Uniform(f32),
    List { sizes: Vec<f32>, prefix: Vec<f32> },
}

impl ItemSizes {
    /// Every item is `size` main-axis units; `count` items total.
    pub fn uniform(size: f32, count: usize) -> Self {
        Self {
            kind: Kind::Uniform(size.max(0.0)),
            count,
        }
    }

    /// Per-index extents. A prefix-sum table is built for `O(log n)` range queries.
    pub fn from_extents(extents: impl Into<Vec<f32>>) -> Self {
        let sizes = extents.into();
        let count = sizes.len();
        let mut prefix = Vec::with_capacity(count + 1);
        prefix.push(0.0);
        let mut acc = 0.0;
        for &s in &sizes {
            acc += s.max(0.0);
            prefix.push(acc);
        }
        Self {
            kind: Kind::List { sizes, prefix },
            count,
        }
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.count
    }

    /// True when there are no items.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Main-axis extent of item `i`.
    pub fn extent_at(&self, i: usize) -> f32 {
        match &self.kind {
            Kind::Uniform(size) => *size,
            Kind::List { sizes, .. } => sizes.get(i).copied().unwrap_or(0.0),
        }
    }

    /// Start offset (main-axis) of item `i`.
    pub fn offset_of(&self, i: usize) -> f32 {
        match &self.kind {
            Kind::Uniform(size) => i as f32 * *size,
            Kind::List { prefix, .. } => prefix.get(i).copied().unwrap_or(0.0),
        }
    }

    /// Total main-axis extent of all items.
    pub fn total_extent(&self) -> f32 {
        match &self.kind {
            Kind::Uniform(size) => self.count as f32 * *size,
            Kind::List { prefix, .. } => prefix.last().copied().unwrap_or(0.0),
        }
    }

    /// First item index whose end offset is strictly greater than `offset`
    /// (i.e. the first potentially visible item), or `count` if none.
    fn first_visible(&self, offset: f32) -> usize {
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.offset_of(mid + 1) > offset {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo
    }

    /// First item index whose start offset is `>= bottom` (exclusive end of the
    /// visible range), or `count` if none.
    fn first_past(&self, bottom: f32) -> usize {
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.offset_of(mid) >= bottom {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo
    }

    /// Visible range for `[scroll, scroll + viewport]`, applying `overscan`.
    pub fn compute_visible_range(
        &self,
        scroll: f32,
        viewport: f32,
        overscan: &Overscan,
    ) -> VisibleRange {
        if self.count == 0 || viewport <= 0.0 {
            return 0..0;
        }
        let (top, bottom) = match overscan {
            Overscan::Items(_) => (scroll.max(0.0), scroll + viewport),
            Overscan::Pixels(px) => ((scroll - px).max(0.0), scroll + viewport + px),
        };
        if bottom <= top {
            return 0..0;
        }
        let mut start = self.first_visible(top);
        let mut end = self.first_past(bottom);
        if start >= end {
            return 0..0;
        }
        if let Overscan::Items(n) = overscan {
            start = start.saturating_sub(*n);
            end = (end + *n).min(self.count);
        }
        start..end
    }
}

/// Overscan policy applied around the strictly visible window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overscan {
    /// Extend the range by `n` items on each side.
    Items(usize),
    /// Extend the scroll window by `px` on each side before the range query.
    Pixels(f32),
}

impl Default for Overscan {
    fn default() -> Self {
        Overscan::Items(0)
    }
}

/// A stable-key → element handle map. Tracks which elements are currently
/// materialized and yields the set that fell outside a newly visible key set.
#[derive(Debug, Clone)]
pub struct KeyMap<K, E> {
    entries: HashMap<K, E>,
}

impl<K: Clone + Eq + Hash, E: Copy> Default for KeyMap<K, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Eq + Hash, E: Copy> KeyMap<K, E> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn get(&self, key: &K) -> Option<E> {
        self.entries.get(key).copied()
    }

    /// Materialize `key` with `element`. Re-assigning an existing key overwrites it.
    pub fn assign(&mut self, key: K, element: E) {
        self.entries.insert(key, element);
    }

    /// Keep only entries whose key is in `visible`; return the evicted
    /// `(key, element)` pairs so callers can return them to a pool.
    pub fn retain_visible(&mut self, visible: &HashSet<K>) -> Vec<(K, E)> {
        let mut evicted = Vec::new();
        let mut kept = HashMap::with_capacity(visible.len());
        for (k, e) in self.entries.drain() {
            if visible.contains(&k) {
                kept.insert(k, e);
            } else {
                evicted.push((k, e));
            }
        }
        self.entries = kept;
        evicted
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// The change set produced by [`VirtualViewport::update`].
#[derive(Debug, Clone)]
pub struct ViewportDiff<K, E> {
    /// `(index, key)` pairs that became visible and need an element assigned.
    pub enter: Vec<(usize, K)>,
    /// `(key, element)` pairs that left the visible set and can be recycled.
    pub exit: Vec<(K, E)>,
}

/// Retained virtual scrolling viewport.
///
/// `K` is the stable per-item key; `E` is the element handle (defaults to
/// [`ElementId`]). The viewport itself owns no elements — it only computes the
/// visible range and diffs the materialized key set so callers can recycle a
/// bounded pool.
pub struct VirtualViewport<K, E = ElementId> {
    sizes: ItemSizes,
    scroll_offset: f32,
    viewport_extent: f32,
    overscan: Overscan,
    materialized: KeyMap<K, E>,
}

impl<K: Clone + Eq + Hash, E: Copy> VirtualViewport<K, E> {
    pub fn new(sizes: ItemSizes, viewport_extent: f32) -> Self {
        Self {
            sizes,
            scroll_offset: 0.0,
            viewport_extent,
            overscan: Overscan::Items(0),
            materialized: KeyMap::new(),
        }
    }

    pub fn set_sizes(&mut self, sizes: ItemSizes) {
        self.sizes = sizes;
    }

    pub fn sizes(&self) -> &ItemSizes {
        &self.sizes
    }

    pub fn set_viewport_extent(&mut self, extent: f32) {
        self.viewport_extent = extent;
    }

    pub fn viewport_extent(&self) -> f32 {
        self.viewport_extent
    }

    pub fn scroll_to(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn set_overscan(&mut self, overscan: Overscan) {
        self.overscan = overscan;
    }

    pub fn overscan(&self) -> Overscan {
        self.overscan
    }

    /// Current visible item range (after overscan), recomputed from scroll state.
    pub fn visible_range(&self) -> VisibleRange {
        self.sizes
            .compute_visible_range(self.scroll_offset, self.viewport_extent, &self.overscan)
    }

    /// Number of materialized (keyed) elements.
    pub fn materialized_count(&self) -> usize {
        self.materialized.len()
    }

    /// Element currently bound to `key`, if materialized.
    pub fn get(&self, key: &K) -> Option<E> {
        self.materialized.get(key)
    }

    /// Bind `element` to `key` (call after an `enter` is reported).
    pub fn assign(&mut self, key: K, element: E) {
        self.materialized.assign(key, element);
    }

    /// Recompute the visible range and diff the materialized key set.
    ///
    /// For each `enter` pair the caller must `assign` an element (reusing an
    /// `exit` element from the pool when possible); `exit` elements are already
    /// removed from the viewport and can be returned to the pool.
    pub fn update(&mut self, key_at: impl Fn(usize) -> K) -> ViewportDiff<K, E> {
        let range = self.visible_range();
        let cap = range.end.saturating_sub(range.start);
        let mut visible_keys = HashSet::with_capacity(cap);
        let mut enter = Vec::new();
        for i in range.clone() {
            let key = key_at(i);
            if visible_keys.insert(key.clone()) && !self.materialized.contains_key(&key) {
                enter.push((i, key));
            }
        }
        let exit = self.materialized.retain_visible(&visible_keys);
        ViewportDiff { enter, exit }
    }
}
