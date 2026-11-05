// SPDX-License-Identifier: MIT
//! Layout cache keyed by text + options, bounded by an entry cap.

use rustc_hash::FxHashMap;

use crate::layout::{LaidOutText, TextLayoutOptions};

/// Default number of shaped results a [`LayoutCache`] retains.
///
/// Sized for a whole screenful of labels with headroom: an entry is one
/// [`LaidOutText`] (a `Vec<GlyphRun>`), which runs a few hundred bytes for a
/// short label up to a few KB for a wrapped paragraph — so 1024 entries is
/// roughly 1 MB of retained shaping work, in the same ballpark as the
/// renderer's 4 MiB glyph raster budget. Enough for a complex screen's labels
/// to survive a reflow, without letting an app that shapes unbounded strings
/// (a chat log, a virtualised list) grow without limit.
pub const DEFAULT_LAYOUT_CACHE_CAPACITY: usize = 1024;

/// The identity a shaping result was produced for, retained *inside* the entry
/// so a lookup can verify without allocating: the map is keyed by a u64 hash
/// and the hit path must not mint an owned `String` per text node per frame
/// just to find out the text is unchanged (the Phase 28.1 allocation audit
/// measured exactly that — one key allocation per painted text node per
/// frame). A hash collision with different content degrades to a re-shape,
/// never to a wrong result.
///
/// `weight` is in this list because of what it costs to leave out: a hit returns
/// glyph runs, so a cache that keyed on (text, size, align) alone would answer a
/// `Medium` request with the `Normal` runs for the same string. Nothing about the
/// result would look wrong — it is the same text at the same size, just not the
/// weight asked for, and the only symptom is that 50 `font_weight` call sites in
/// floem-ui render flat.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheKey {
    text: String,
    max_width_bits: u64,
    font_size_bits: u32,
    align: u8,
    weight: u16,
    /// The typeface request, because it changes the glyph ids and the advances
    /// outright: a hit for a `Monospace` request must never answer with the
    /// proportional runs for the same string.
    family: floom_types::FontFamily,
}

impl CacheKey {
    fn new(text: &str, options: &TextLayoutOptions) -> Self {
        Self {
            text: text.to_string(),
            max_width_bits: options.max_width.to_bits(),
            font_size_bits: options.font_size.to_bits(),
            align: options.align as u8,
            weight: options.weight.to_value(),
            family: options.family.clone(),
        }
    }

    fn matches(&self, text: &str, options: &TextLayoutOptions) -> bool {
        self.text == text
            && self.max_width_bits == options.max_width.to_bits()
            && self.font_size_bits == options.font_size.to_bits()
            && self.align == options.align as u8
            && self.weight == options.weight.to_value()
            && self.family == options.family
    }
}

/// Hashes (text, layout options) into the map key. Every field of `CacheKey`
/// participates, so two keys hash equal only when their content does.
fn cache_key_hash(text: &str, options: &TextLayoutOptions) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    options.max_width.to_bits().hash(&mut hasher);
    options.font_size.to_bits().hash(&mut hasher);
    (options.align as u8).hash(&mut hasher);
    options.weight.to_value().hash(&mut hasher);
    options.family.hash(&mut hasher);
    hasher.finish()
}

/// A cached shaping result stamped with the tick it was last used at, so
/// eviction can drop the entry that has gone unused longest.
#[derive(Debug)]
struct CacheEntry {
    key: CacheKey,
    value: LaidOutText,
    last_used: u64,
}

/// Bounded shaping-result cache that evicts least-recently-used entries.
///
/// Callers reuse the same `GlyphRun`s from a hit, which also keeps their font
/// data stable — the renderer's glyph raster cache keys off that data.
#[derive(Debug)]
pub struct LayoutCache {
    map: FxHashMap<u64, CacheEntry>,
    /// Monotonic access counter. Comparing `last_used` is what makes eviction
    /// LRU rather than first-in-first-out, so a label painted every frame is
    /// never dropped in favour of one-off text.
    clock: u64,
    /// Maximum entries retained, clamped to at least 1: the cache hands out
    /// references into itself, so it must hold the entry it just produced.
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutCache {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_LAYOUT_CACHE_CAPACITY)
    }

    /// Builds a cache retaining at most `capacity` entries.
    ///
    /// `0` is clamped to `1`: a capacity below one cannot be honoured because
    /// the cache returns a reference into itself.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: FxHashMap::default(),
            clock: 0,
            capacity: capacity.max(1),
            hits: 0,
            misses: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Resizes the cache, evicting down immediately if it is now over the cap.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        // `>` here (not `>=`): shrinking should leave exactly the new cap,
        // whereas `make_room` leaves one slot free for an insert.
        while self.map.len() > self.capacity {
            if !self.evict_oldest() {
                break;
            }
        }
    }

    pub fn get_or_insert_with(
        &mut self,
        text: &str,
        options: &TextLayoutOptions,
        f: impl FnOnce() -> LaidOutText,
    ) -> &LaidOutText {
        // The hit path is allocation-free: verify the retained key against the
        // borrowed text instead of building an owned key to look up with.
        let hash = cache_key_hash(text, options);
        let hit = self
            .map
            .get(&hash)
            .is_some_and(|entry| entry.key.matches(text, options));
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
            // Make room before inserting so `len()` never exceeds the cap.
            self.make_room();
            let entry = CacheEntry {
                key: CacheKey::new(text, options),
                value: f(),
                last_used: self.next_tick(),
            };
            self.map.insert(hash, entry);
        }
        // Bump the LRU order for both paths, then lend the value out. Direct
        // field access (not `next_tick`) so the map borrow stays disjoint.
        let entry = self.map.get_mut(&hash).expect("hit or just inserted");
        self.clock = self.clock.wrapping_add(1);
        entry.last_used = self.clock;
        &entry.value
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }

    pub fn misses(&self) -> u64 {
        self.misses
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Drops every entry and resets the hit/miss counters, keeping the cap.
    pub fn clear(&mut self) {
        self.map.clear();
        self.hits = 0;
        self.misses = 0;
    }

    fn next_tick(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        self.clock
    }

    /// Evicts the least-recently-used entry, returning false if already empty.
    fn evict_oldest(&mut self) -> bool {
        let oldest = self
            .map
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(hash, _)| *hash);
        match oldest {
            Some(hash) => {
                self.map.remove(&hash);
                true
            }
            None => false,
        }
    }

    /// Evicts until at least one slot is free, so an insert cannot overshoot
    /// the cap.
    ///
    /// The scan is O(n) but only runs once the cache is full, and `n` is the
    /// cap — a few thousand u64 compares against shaping work that is orders of
    /// magnitude more expensive.
    fn make_room(&mut self) {
        while self.map.len() >= self.capacity {
            if !self.evict_oldest() {
                break;
            }
        }
    }
}
