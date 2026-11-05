// SPDX-License-Identifier: MIT
//! Per-element style sets keyed by [`StyleState`], plus resolution.

use std::collections::HashMap;

use crate::props::{StyleProps, StyleState};

/// Styles for one element across interaction states.
#[derive(Debug, Clone, Default)]
pub struct StyleSet {
    by_state: HashMap<StyleState, StyleProps>,
}

impl StyleSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, state: StyleState, props: StyleProps) {
        self.by_state.insert(state, props);
    }

    pub fn get(&self, state: StyleState) -> Option<&StyleProps> {
        self.by_state.get(&state)
    }

    /// Precedence (last wins): NORMAL < SELECTED < FOCUSED < HOVER < PRESSED < DISABLED.
    pub fn resolve(&self, active: StyleState) -> StyleProps {
        const ORDER: &[StyleState] = &[
            StyleState::SELECTED,
            StyleState::FOCUSED,
            StyleState::HOVER,
            StyleState::PRESSED,
            StyleState::DISABLED,
        ];
        let mut out = StyleProps::default();
        if let Some(base) = self.by_state.get(&StyleState::NORMAL) {
            out.merge_over(base);
        }
        for &flag in ORDER {
            if active.contains(flag)
                && let Some(p) = self.by_state.get(&flag)
            {
                out.merge_over(p);
            }
        }
        out
    }
}

/// Map of element keys → style sets (for invalidation benches / tests).
#[derive(Debug, Default)]
pub struct StyleMap {
    pub entries: HashMap<u64, StyleSet>,
    pub invalidate_count: u64,
}

impl StyleMap {
    pub fn insert(&mut self, id: u64, set: StyleSet) {
        self.entries.insert(id, set);
        self.invalidate_count += 1;
    }

    pub fn resolve(&self, id: u64, state: StyleState) -> StyleProps {
        self.entries
            .get(&id)
            .map(|s| s.resolve(state))
            .unwrap_or_default()
    }

    pub fn invalidate(&mut self, id: u64) {
        let _ = id;
        self.invalidate_count += 1;
    }
}
