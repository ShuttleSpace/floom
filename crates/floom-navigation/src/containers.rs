//! Navigation container state machines: stack, tabs, split.
//!
//! Widget-free by architecture: each container holds only navigation state
//! (what is visible, in what arrangement) and the widget layer renders it.
//! All of them keep *per-slot* state so switching tabs or collapsing a split
//! does not lose the hidden content's position.

/// Which pane is on top in a narrow stack layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackPane {
    /// The first (list/master) pane.
    #[default]
    Primary,
    /// The second (detail) pane.
    Secondary,
}

/// Two-pane adaptive container: side-by-side when wide, a single pane that
/// swaps when narrow.
#[derive(Debug, Clone)]
pub struct StackNav {
    /// `true` renders both panes side-by-side (wide layout).
    pub expanded: bool,
    /// When collapsed, which pane is visible.
    pub pane: StackPane,
}

impl Default for StackNav {
    fn default() -> Self {
        Self {
            expanded: true,
            pane: StackPane::Primary,
        }
    }
}

impl StackNav {
    /// Whether the detail pane is currently visible.
    pub fn shows_secondary(&self) -> bool {
        self.expanded || self.pane == StackPane::Secondary
    }

    /// Show the detail pane (a no-op in the expanded layout).
    pub fn open_secondary(&mut self) {
        self.pane = StackPane::Secondary;
    }

    /// Show the primary pane (a no-op in the expanded layout).
    pub fn open_primary(&mut self) {
        self.pane = StackPane::Primary;
    }

    /// Re-evaluate after the window resized: widening expands, and a pane
    /// becomes visible either way.
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }
}

/// A tab bar where each tab owns independent state (its own navigation stack
/// position, scroll offset, …) held by the app, keyed by tab id.
#[derive(Debug, Clone)]
pub struct TabNav {
    selected: usize,
    count: usize,
}

impl TabNav {
    /// `count` tabs, starting on tab `selected`.
    ///
    /// # Panics
    /// When `count == 0` or `selected >= count`.
    pub fn new(count: usize, selected: usize) -> Self {
        assert!(count > 0, "a tab container needs at least one tab");
        assert!(selected < count, "selected tab out of range");
        Self { selected, count }
    }

    /// The visible tab.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Switch tabs; out-of-range requests are ignored (defensive against a
    /// stale UI emitting a dead tab index after the list shrank).
    pub fn select(&mut self, index: usize) {
        if index < self.count {
            self.selected = index;
        }
    }

    /// How many tabs exist.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Whether this tab is the visible one (widget keys use this).
    pub fn is_selected(&self, index: usize) -> bool {
        index == self.selected
    }
}

/// Leading/trailing pane container with a draggable divider.
#[derive(Debug, Clone)]
pub struct SplitNav {
    /// Fraction (0..=1) of the total width the leading pane occupies.
    pub ratio: f32,
    /// Whether the divider can be moved.
    pub resizable: bool,
    /// Whether the leading pane is shown at all (collapsible sidebar).
    pub leading_visible: bool,
}

impl Default for SplitNav {
    fn default() -> Self {
        Self {
            ratio: 0.3,
            resizable: true,
            leading_visible: true,
        }
    }
}

impl SplitNav {
    /// Clamp-and-set the divider from a pointer position expressed as a
    /// fraction of the container width. Denies anything that would squeeze a
    /// pane below `min` (as a fraction), keeping the divider grabbable.
    pub fn set_ratio(&mut self, ratio: f32, min: f32) {
        let max = 1.0 - min;
        self.ratio = ratio.clamp(min, max);
    }

    /// The effective leading fraction: 0 when the pane is hidden.
    pub fn effective_ratio(&self) -> f32 {
        if self.leading_visible {
            self.ratio
        } else {
            0.0
        }
    }

    /// Toggle the leading pane, restoring its previous ratio on re-open.
    pub fn toggle_leading(&mut self) {
        self.leading_visible = !self.leading_visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_nav_swaps_panes_only_when_collapsed() {
        let mut stack = StackNav::default();
        assert!(stack.expanded);
        assert!(stack.shows_secondary());

        // Collapsed: panes swap on demand.
        stack.set_expanded(false);
        assert!(!stack.shows_secondary());
        stack.open_secondary();
        assert!(stack.shows_secondary());
        stack.open_primary();
        assert!(!stack.shows_secondary());

        // Re-expanding shows both, whichever pane was selected.
        stack.set_expanded(true);
        assert!(stack.shows_secondary());
    }

    #[test]
    fn tab_nav_selects_within_range() {
        let mut tabs = TabNav::new(3, 0);
        assert!(tabs.is_selected(0));

        tabs.select(2);
        assert_eq!(tabs.selected(), 2);

        // Stale indices (e.g. after the tab list shrank) are ignored.
        tabs.select(9);
        assert_eq!(tabs.selected(), 2);

        assert_eq!(tabs.count(), 3);
    }

    #[test]
    #[should_panic(expected = "at least one tab")]
    fn tab_nav_rejects_zero_tabs() {
        let _ = TabNav::new(0, 0);
    }

    #[test]
    fn split_nav_clamps_and_restores() {
        let mut split = SplitNav::default();
        assert!((split.effective_ratio() - 0.3).abs() < f32::EPSILON);

        // The divider never squeezes a pane below the minimum.
        split.set_ratio(0.05, 0.2);
        assert!((split.ratio - 0.2).abs() < f32::EPSILON);
        split.set_ratio(0.98, 0.2);
        assert!((split.ratio - 0.8).abs() < f32::EPSILON);

        // Hiding the leading pane collapses to 0; re-opening restores.
        split.toggle_leading();
        assert_eq!(split.effective_ratio(), 0.0);
        split.toggle_leading();
        assert!((split.effective_ratio() - 0.8).abs() < f32::EPSILON);
    }
}
