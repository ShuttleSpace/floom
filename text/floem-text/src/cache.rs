//! Layout cache keyed by text + options.

use rustc_hash::FxHashMap;

use crate::layout::{LaidOutText, TextLayoutOptions};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    text: String,
    max_width_bits: u64,
    font_size_bits: u32,
    align: u8,
}

impl CacheKey {
    fn new(text: &str, options: &TextLayoutOptions) -> Self {
        Self {
            text: text.to_string(),
            max_width_bits: options.max_width.to_bits(),
            font_size_bits: options.font_size.to_bits(),
            align: options.align as u8,
        }
    }
}

/// Simple shaping result cache.
#[derive(Debug, Default)]
pub struct LayoutCache {
    map: FxHashMap<CacheKey, LaidOutText>,
    hits: u64,
    misses: u64,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_insert_with(
        &mut self,
        text: &str,
        options: &TextLayoutOptions,
        f: impl FnOnce() -> LaidOutText,
    ) -> &LaidOutText {
        let key = CacheKey::new(text, options);
        if self.map.contains_key(&key) {
            self.hits += 1;
        } else {
            self.misses += 1;
            self.map.insert(key.clone(), f());
        }
        self.map.get(&key).expect("just inserted")
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

    pub fn clear(&mut self) {
        self.map.clear();
        self.hits = 0;
        self.misses = 0;
    }
}
